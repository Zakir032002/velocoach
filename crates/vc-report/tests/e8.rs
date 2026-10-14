//! E8: with Ned (no true difference between arms), the 95 % bootstrap CI of the
//! pooled A-vs-B difference covers 0 in 92-98 % of 200 replicated pilots.
use vc_core::study::{Pcg32, StudyAllocator};
use vc_efficacy::Arm;
use vc_report::stats::{BOOTSTRAP_B, Group, Obs, bootstrap_diff};

const REPLICATES: u64 = 200;
const PEOPLE: u32 = 3;
const POINTS: u32 = 50;

fn replicate(r: u64) -> Vec<Obs> {
    let mut obs = Vec::new();
    for person in 0..PEOPLE {
        let seed = r * 100 + u64::from(person);
        let mut alloc = StudyAllocator::new(seed);
        let mut ned = Pcg32::new(seed, 7);
        for _ in 0..POINTS {
            let group = match alloc.draw().arm {
                Arm::Direct => Group::A,
                Arm::Autonomy => Group::B,
                _ => continue, // shams are not in the A-vs-B comparison
            };
            obs.push(Obs {
                person,
                group,
                complied: ned.next_f64() < 0.5,
            });
        }
    }
    obs
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "slow in debug: cargo test -p vc-report --release --test e8"
)]
fn e8_the_null_persona_covers_zero() {
    let covered = (0..REPLICATES)
        .filter(|&r| {
            bootstrap_diff(&replicate(r), 1_000 + r, BOOTSTRAP_B)
                .is_some_and(|ci| ci.lo <= 0.0 && 0.0 <= ci.hi)
        })
        .count();
    let rate = covered as f64 / REPLICATES as f64;
    println!("E8 coverage: {covered}/{REPLICATES} = {rate:.3}");
    assert!((0.92..=0.98).contains(&rate), "coverage {rate:.3}");
}
