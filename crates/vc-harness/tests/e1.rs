//! E1: from the power stream alone, the observer recovers Calib's hidden
//! compliance (0.70) within +/-0.08 and median latency (3.0 s) within +/-0.5 s,
//! over 200 scored cues, for each of 20 seeds.
use vc_efficacy::Verdict;
use vc_efficacy::observer::SMOOTHING_LAG_MS;
use vc_harness::{RunOpts, run_sim};

const CALIB: u8 = 4;
const CUES: u32 = 200;

fn median(mut v: Vec<i32>) -> f64 {
    v.sort_unstable();
    let n = v.len();
    f64::from(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    })
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "slow in debug: cargo test -p vc-harness --release --test e1"
)]
fn e1_the_observer_recovers_calib() {
    for seed in 0..20u64 {
        let (mut complied, mut scored, mut latencies, mut session) = (0u32, 0u32, Vec::new(), 0u64);
        while scored < CUES {
            let run = run_sim(RunOpts {
                scenario: "creep",
                persona: CALIB,
                seed: Some(seed * 1_000 + session),
                ..RunOpts::default()
            })
            .expect("the ride runs");
            for o in &run.outcomes {
                if scored == CUES {
                    break;
                }
                match o.verdict {
                    Verdict::Complied => {
                        complied += 1;
                        scored += 1;
                        if let Some(l) = o.l_obs_ms {
                            latencies.push(l - SMOOTHING_LAG_MS);
                        }
                    }
                    Verdict::Partial | Verdict::Ignored | Verdict::Reversed => scored += 1,
                    Verdict::Confounded | Verdict::Invalid => {}
                }
            }
            session += 1;
        }
        let rate = f64::from(complied) / f64::from(scored);
        let lat_s = median(latencies) / 1000.0;
        println!(
            "seed {seed:2}: compliance {rate:.3} · median latency {lat_s:.2} s · {session} sessions"
        );
        assert!(
            (rate - 0.70).abs() <= 0.08,
            "seed {seed}: compliance {rate:.3}"
        );
        assert!(
            (lat_s - 3.0).abs() <= 0.5,
            "seed {seed}: latency {lat_s:.2} s"
        );
    }
}
