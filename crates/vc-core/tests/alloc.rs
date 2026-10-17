//! S10: after warm-up, ingest() and tick() never touch the heap.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use vc_core::testkit::{cpm_bytes, hrm_bytes};
use vc_core::{Engine, EngineConfig, LedgerRecord, Source, Step, StepKind, WorkoutPlan};
use vc_gatt::CrankData;

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

fn note() {
    let _ = ARMED.try_with(|armed| {
        if armed.get() {
            let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        }
    });
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note();
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

#[test]
fn ten_thousand_ingests_and_ticks_after_warm_up_allocate_nothing() {
    // Everything that allocates happens before arming.
    let plan = WorkoutPlan::from_steps(&[
        Step::new(StepKind::Warmup, 600, Some(150.0)).with_cadence(90.0),
        Step::new(StepKind::Interval, 240, Some(320.0)),
        Step::new(StepKind::Recovery, 180, Some(120.0)),
    ])
    .unwrap();

    let mut calls: Vec<(u64, Source, Vec<u8>)> = Vec::new();

    for k in 0..12_000u64 {
        let t = k * 250_000_000;
        let crank = CrankData {
            revs: (k / 3) as u16,
            time_1024: ((k / 3) * 683 % 65_536) as u16,
        };
        // After the interval, ride the recovery well over its target so that
        // soft-pedal cues are decided, played and scored while counting.
        let watts = if (2_400..3_360).contains(&k) {
            330.0
        } else if k >= 3_360 {
            170.0
        } else {
            160.0
        };

        calls.push((t, Source::Power, cpm_bytes(watts, Some(crank))));

        if k % 4 == 0 {
            calls.push((t, Source::Hr, hrm_bytes(150 + (k / 400) as u16)));
        }
    }

    let mut drained = [LedgerRecord::SessionEnd {
        t_ns: 0,
        cues: 0,
        dropped: 0,
    }; 64];
    let mut scored_cues = 0usize;

    let mut e = Engine::new(EngineConfig::default(), plan).unwrap();
    e.start(0);

    let mut run = |e: &mut Engine, slice: &[(u64, Source, Vec<u8>)]| {
        for (i, (t, src, bytes)) in slice.iter().enumerate() {
            let mut decided = e.ingest(*t, *src, bytes).ok().flatten();

            if *src == Source::Power {
                decided = decided.or(e.tick(*t));
            }

            // Every decision is "played": 2.4 s of audio after its lead.
            if let Some(d) = decided {
                let onset = *t + u64::from(d.lead_ms) * 1_000_000;
                e.on_cue_played(d.id, onset, onset + 2_400_000_000, onset + 2_400_000_000);
            }

            // About once a simulated second: drain the ledger ring.
            if i % 4_000 == 0 {
                let n = e.ledger_pop(&mut drained);
                scored_cues += drained[..n]
                    .iter()
                    .filter(|r| matches!(r, LedgerRecord::Cue { .. }))
                    .count();
            }
        }
    };

    run(&mut e, &calls[..2_000]); // Warm-up: quality channels become Fresh.

    // Count allocations from this point onward.
    ARMED.with(|a| a.set(true));
    run(&mut e, &calls[2_000..12_000]);
    e.end_session(3_000_000_000_000);
    ARMED.with(|a| a.set(false));

    let n = COUNT.with(Cell::get);
    assert_eq!(n, 0, "{n} heap allocations on the hot path");

    // The armed region must really have scored cues, or the test proves nothing.
    let n = e.ledger_pop(&mut drained);
    scored_cues += drained[..n]
        .iter()
        .filter(|r| matches!(r, LedgerRecord::Cue { .. }))
        .count();
    assert!(
        scored_cues > 0,
        "no cue was observed and recorded while counting"
    );
}
