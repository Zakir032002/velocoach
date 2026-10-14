//! E5: opening, updating and closing windows never touches the heap.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use vc_efficacy::observer::{ObsContext, Observer};
use vc_efficacy::{Arm, CueEvent, P3Ring};

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let _ = ARMED.try_with(|a| {
            if a.get() {
                let _ = COUNT.try_with(|c| c.set(c.get() + 1));
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

#[test]
fn ten_cues_over_ten_minutes_allocate_nothing() {
    const MS: u64 = 1_000_000;
    let (mut ring, mut obs) = (P3Ring::new(), Observer::new());
    ARMED.with(|a| a.set(true));
    for t in (0..600_000u64).step_by(250) {
        // a slow sawtooth so verdicts vary
        ring.push(t * MS, 200.0 + ((t / 1_000) % 60) as f64);
        let cue_start = t % 60_000 == 30_000;
        if cue_start && !obs.is_open() {
            let ev = CueEvent {
                cue_id: (t / 60_000) as u32,
                directive: 1,
                priority: 2,
                aki_class: 2,
                arm: Arm::Autonomy,
                propensity_q16: u16::MAX,
                is_sham: false,
                dir: -1,
                delta_star_w: 20.0,
                profile: 0,
                t_decision_ns: (t - 2_400) * MS,
                t_onset_ns: (t - 2_400) * MS,
                t_end_ns: t * MS,
            };
            let _ = obs.open(&ev, &ring, Some(0.0));
        }
        let _ = obs.update(t * MS, &ObsContext::quiet(&ring, 200.0));
    }
    ARMED.with(|a| a.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
}
