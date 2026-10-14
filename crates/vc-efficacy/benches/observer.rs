use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use vc_efficacy::observer::{ObsContext, Observer};
use vc_efficacy::{Arm, CueEvent, P3Ring};

const MS: u64 = 1_000_000;

/// A ring with 108 s of history and a window opened at 102 s, updated through 107.75 s.
fn prepared() -> (P3Ring, Observer) {
    let ev = CueEvent {
        cue_id: 1,
        directive: 1,
        priority: 2,
        aki_class: 2,
        arm: Arm::Direct,
        propensity_q16: u16::MAX,
        is_sham: false,
        dir: -1,
        delta_star_w: 25.0,
        profile: 0,
        t_decision_ns: 99_950 * MS,
        t_onset_ns: 100_000 * MS,
        t_end_ns: 102_000 * MS,
    };
    let (mut ring, mut obs) = (P3Ring::new(), Observer::new());
    for t in (0..=107_750u64).step_by(250) {
        ring.push(t * MS, 200.0);
        if t == 102_000 {
            obs.open(&ev, &ring, None);
        }
        if t >= 102_000 {
            obs.update(t * MS, &ObsContext::quiet(&ring, 200.0));
        }
    }
    (ring, obs)
}

fn observer(c: &mut Criterion) {
    let state = prepared();
    c.bench_function("push + update at a bin boundary, window open", |b| {
        b.iter_batched(
            || state.clone(),
            |(mut ring, mut obs)| {
                ring.push(108_000 * MS, 200.0);
                black_box(obs.update(108_000 * MS, &ObsContext::quiet(&ring, 200.0)))
            },
            BatchSize::SmallInput,
        );
    });
    c.bench_function("push + update mid-bin, window open", |b| {
        b.iter_batched(
            || state.clone(),
            |(mut ring, mut obs)| {
                ring.push(107_900 * MS, 200.0);
                black_box(obs.update(107_900 * MS, &ObsContext::quiet(&ring, 200.0)))
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, observer);
criterion_main!(benches);
