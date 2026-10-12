use anyhow::{Context, Result, anyhow};
use vc_core::golden;
use vc_core::testkit::Ride;
use vc_core::{EngineConfig, Profile, Step, StepKind, WorkoutPlan};

fn ride(profile: Profile, steps: &[Step]) -> Result<Ride> {
    let cfg = EngineConfig {
        profile,
        ..EngineConfig::default()
    };
    let plan = WorkoutPlan::from_steps(steps).map_err(|_| anyhow!("plan too long"))?;
    Ride::new(cfg, plan).map_err(|e| anyhow!("{e}"))
}

/// Day 6 stub: three scripted rides. Day 7 replaces them with vc-sim scenarios.
fn scripted() -> Result<Vec<(&'static str, Ride)>> {
    use StepKind::*;
    // 1. A warm-up, a countdown, then a 400 W interval that drains W′.
    let mut surge = ride(
        Profile::Solo,
        &[
            Step::new(Warmup, 60, Some(150.0)),
            Step::new(Interval, 180, Some(400.0)),
            Step::new(Recovery, 120, Some(120.0)),
        ],
    )?;
    surge.run(60.0);
    surge.stream.power_w = Some(400.0);
    surge.run(180.0);
    surge.stream.power_w = Some(120.0);
    surge.run(120.0);

    // 2. Heart rate above 98 % of max, Rolling profile.
    let mut redline = ride(Profile::Rolling, &[Step::new(Steady, 600, Some(200.0))])?;
    redline.stream.power_w = Some(200.0);
    redline.stream.hr_bpm = Some(188);
    redline.run(150.0);

    // 3. The power meter goes quiet for 20 s and comes back.
    let mut dropout = ride(Profile::Solo, &[Step::new(Steady, 600, Some(200.0))])?;
    dropout.stream.power_w = Some(200.0);
    dropout.run(30.0);
    dropout.stream.power_w = None;
    dropout.run(20.0);
    dropout.stream.power_w = Some(200.0);
    dropout.run(30.0);

    Ok(vec![
        ("surge_wprime", surge),
        ("redline_rolling", redline),
        ("dropout", dropout),
    ])
}

/// Write golden/<name>.log, or with --check compare and fail on any difference.
fn golden(check: bool) -> Result<()> {
    std::fs::create_dir_all("golden")?;
    let mut failed = 0;
    for (name, ride) in scripted()? {
        let mut text = String::from(golden::HEADER);
        for (_, d) in &ride.log {
            golden::write_line(&mut text, d).map_err(|_| anyhow!("format error"))?;
        }
        let path = format!("golden/{name}.log");
        if !check {
            std::fs::write(&path, &text)?;
            println!("wrote {path}: {} decisions", ride.log.len());
            continue;
        }
        let want = std::fs::read_to_string(&path).with_context(|| {
            format!("{path} is missing: run `vc-cli golden` once and review it")
        })?;
        if want == text {
            println!("ok {path}");
        } else {
            failed += 1;
            let line = want
                .lines()
                .zip(text.lines())
                .position(|(a, b)| a != b)
                .map_or(0, |i| i + 1);
            println!("DIFF {path} (first difference at line {line})");
        }
    }
    anyhow::ensure!(
        failed == 0,
        "{failed} golden log(s) differ: review, then rerun without --check to accept"
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("golden") => golden(args.iter().any(|a| a == "--check")),
        _ => {
            eprintln!("Usage: vc-cli golden [--check]");
            std::process::exit(1);
        }
    }
}
