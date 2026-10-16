use anyhow::{Context, Result, anyhow};
use vc_core::Mode;
use vc_gatt::{Ctrl, MAX_PAYLOAD, encode_ctrl};
use vc_harness::{RunOpts, SimRun};
use vc_sim::Outcome;

/// Run the Engine and Simulator together using virtual time (see vc-harness).
fn run_sim(name: &str, persona: u8, seed: Option<u64>, mode: Mode) -> Result<SimRun> {
    vc_harness::run_sim(RunOpts {
        scenario: name,
        persona,
        seed,
        mode,
        mode_seed: seed.unwrap_or(0),
        ctrl_at: &[],
        human_permille: None,
    })
    .map_err(|e| anyhow!(e))
}

/// Write golden/<name>.log, or with --check compare and fail on any difference.
const GOLDEN: [&str; 5] = [
    "climb_surge",
    "recovery_drift",
    "safety_hr",
    "dropout",
    "grade_confound",
];

/// Generate golden logs from real simulator scenarios, or compare with --check.
fn golden(check: bool) -> Result<()> {
    std::fs::create_dir_all("golden")?;
    let mut failed = 0;

    for name in GOLDEN {
        // Use the scenario's default persona and seed.
        let run = run_sim(name, 0, None, Mode::Coach)?;

        let mut text = String::from(vc_core::golden::HEADER);
        for decision in &run.decisions {
            vc_core::golden::write_line(&mut text, decision)
                .map_err(|_| anyhow!("format error"))?;
        }

        let path = format!("golden/{name}.log");

        if !check {
            std::fs::write(&path, &text)?;
            println!("wrote {path}: {} decisions", run.decisions.len());
            continue;
        }

        let want = std::fs::read_to_string(&path)
            .with_context(|| format!("{path} is missing: run `vc-cli golden`"))?;

        if want == text {
            println!("ok                    {path}");
        } else {
            failed += 1;
            let line = want
                .lines()
                .zip(text.lines())
                .position(|(a, b)| a != b)
                .map_or(0, |i| i + 1);

            println!("DIFF                          {path} (first difference at line {line})");
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

    let flag = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    };

    match args.get(1).map(String::as_str) {
        Some("sim") => {
            let scenario = flag("--scenario").unwrap_or("climb_surge");

            let persona = match flag("--persona") {
                Some(name) => persona_code(name)?,
                None => 0,
            };

            let seed = flag("--seed").map(str::parse).transpose()?;

            let mode = match flag("--mode") {
                None | Some("coach") => Mode::Coach,
                Some("study") => Mode::Study,
                Some(other) => {
                    return Err(anyhow!("mode must be coach or study, not '{other}'"));
                }
            };

            sim_cmd(
                scenario,
                persona,
                seed,
                mode,
                flag("--jsonl"),
                flag("--rider").unwrap_or("R01"),
            )
        }
        Some("golden") => golden(args.iter().any(|arg| arg == "--check")),
        Some("hex") => hex_cmd(&args[2..]),
        Some("phrases") => phrases_cmd(),
        Some("replay") => replay_cmd(args.get(2).context("usage: vc-cli replay <file.vcraw>")?),
        Some("soak-check") => {
            soak_check(args.get(2).context("usage: vc-cli soak-check <soak.csv>")?)
        }
        Some("power-trace") => power_trace(flag("--scenario").unwrap_or("climb_surge")),
        Some("find-interrupt") => find_interrupt(flag("--scenario").unwrap_or("recovery_drift")),
        Some("wbal") => wbal_cmd(
            args.get(2).context("usage: vc-cli wbal <power.csv>")?,
            250.0,
            20_000.0,
        ),
        _ => {
            eprintln!(
                "Usage:\n  vc-cli sim [--scenario NAME] [--persona NAME] [--seed N] \
                 [--mode coach|study] [--jsonl FILE [--rider ID]]\n  vc-cli golden [--check]\n  \
                 vc-cli hex <load|start|stop|ping|...>\n  vc-cli phrases\n  \
                 vc-cli replay <file.vcraw>\n  vc-cli soak-check <soak.csv>\n  \
                 vc-cli power-trace [--scenario NAME]\n  vc-cli find-interrupt [--scenario NAME]\n  \
                 vc-cli wbal <power.csv>"
            );
            std::process::exit(1);
        }
    }
}

/// Print CTRL messages as hex for nRF Connect's write dialog.
fn hex_cmd(args: &[String]) -> Result<()> {
    let num = |i: usize, default: u32| -> Result<u32> {
        args.get(i)
            .map_or(Ok(default), |s| s.parse::<u32>().context("numbers only"))
    };
    let msg = match args.first().map(String::as_str) {
        Some("load") => Ctrl::Load {
            scenario: num(1, 1)? as u8,
            persona: num(2, 0)? as u8,
            seed: num(3, 42)?,
            human: num(4, 0)? == 1,
        },
        Some("start") => Ctrl::Start,
        Some("stop") => Ctrl::Stop,
        Some("ping") => Ctrl::Ping {
            ping_id: num(1, 1)? as u16,
            t_client_ns: 0,
        },
        _ => anyhow::bail!(
            "usage: vc-cli hex load [scenario persona seed human] | start | stop | ping [id]"
        ),
    };
    let mut b = [0u8; MAX_PAYLOAD];
    let n = encode_ctrl(&msg, &mut b).map_err(|e| anyhow!("{e:?}"))?;
    println!(
        "{}",
        b[..n]
            .iter()
            .map(|x| format!("{x:02X}"))
            .collect::<String>()
    );
    Ok(())
}

/// The phrase bank as TSV: key, text, default earcon, earcon-if-enabled.
fn phrases_cmd() -> Result<()> {
    use vc_core::phrase::{PHRASES, earcon_for};
    for p in PHRASES {
        let default = earcon_for(p.directive, 0).map_or("-", |e| e.key());
        let enabled = earcon_for(p.directive, u8::MAX).map_or("-", |e| e.key());
        println!("{}\t{}\t{}\t{}", p.key, p.text, default, enabled);
    }
    // The volume-calibration clip: not a cue, never linted, never sent to the Engine.
    println!("CAL\tAdjust the volume now, this is a test\t-\t-");
    Ok(())
}

fn replay_cmd(path: &str) -> Result<()> {
    let data = std::fs::read(path).with_context(|| format!("reading {path}"))?;
    let r = vc_core::capture::replay(&data).map_err(|e| anyhow!(e))?;
    println!(
        "phone: {} decisions · replay: {} decisions",
        r.phone.len(),
        r.replayed.len()
    );
    if r.phone == r.replayed {
        println!("identical");
        return Ok(());
    }
    let i = r
        .phone
        .iter()
        .zip(&r.replayed)
        .position(|(a, b)| a != b)
        .unwrap_or(r.phone.len().min(r.replayed.len()));
    println!("first difference at decision {i}:");
    println!("  phone:  {:?}", r.phone.get(i));
    println!("  replay: {:?}", r.replayed.get(i));
    anyhow::bail!("replay differs");
}

/// After run_sim: drain the Engine's ledger ring through the same writer the phone uses.
fn write_jsonl(run: &mut vc_harness::SimRun, path: &str, rider: &str, data: String) -> Result<()> {
    let mut writer = vc_report::Writer::new(rider, None);
    writer.data = Some(data);
    let mut buf = [vc_core::LedgerRecord::SessionEnd {
        t_ns: 0,
        cues: 0,
        dropped: 0,
    }; 64];
    let mut text = String::new();
    loop {
        let n = run.engine.ledger_pop(&mut buf);
        if n == 0 {
            break;
        }
        for r in &buf[..n] {
            text.push_str(&vc_report::to_json(&writer.line(r)));
            text.push('\n');
        }
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// S6 from the soak log.
fn soak_check(path: &str) -> Result<()> {
    const GRACE_S: f64 = 20.0;
    let sc = vc_sim::scenario::by_name("soak").context("no soak scenario")?;
    // (start, end incl. grace, name) for every scripted fault
    let windows: Vec<(f64, f64, String)> = sc
        .faults
        .iter()
        .map(|f| {
            let start = f64::from(f.at_s);
            let dur = if f.dur_ms == 0 {
                f64::from(f.param) / 4.0
            } else {
                f64::from(f.dur_ms) / 1000.0
            }; // garbage: 4 per s
            (
                start,
                start + dur + GRACE_S,
                format!("{:?} at {}:00", f.kind, f.at_s / 60),
            )
        })
        .collect();
    let rows: Vec<Vec<String>> = std::fs::read_to_string(path)?
        .lines()
        .skip(1)
        .map(|l| l.split(',').map(str::to_string).collect())
        .filter(|r: &Vec<String>| r.len() >= 16)
        .collect();
    anyhow::ensure!(!rows.is_empty(), "no rows");
    let t = |r: &Vec<String>| r[0].parse::<f64>().unwrap_or(0.0);
    let healthy =
        |r: &Vec<String>| r[2] == "ready" && r[3] == "Fresh" && r[4] == "Fresh" && r[5] == "Fresh";

    let mut unexplained = Vec::new();
    let mut last_t = 0.0;
    let mut longest_gap = 0.0f64;
    for r in &rows {
        if r[1].is_empty() {
            longest_gap = longest_gap.max(t(r) - last_t);
            last_t = t(r);
        }
        if t(r) > 60.0 && !healthy(r) && !windows.iter().any(|(a, b, _)| (*a..=*b).contains(&t(r)))
        {
            unexplained.push(format!(
                "{:7.1} s  link {} · HR {} · power {} · cadence {}",
                t(r),
                r[2],
                r[3],
                r[4],
                r[5]
            ));
        }
    }
    for (a, b, name) in &windows {
        let back = rows.iter().find(|r| {
            // Only within this fault's own window: a later fault's recovery is not this one's.
            t(r) > *a
                && t(r) <= *b
                && healthy(r)
                && rows.iter().any(|x| t(x) > *a && t(x) < t(r) && !healthy(x))
        });
        match back {
            Some(r) => println!("{name:<28} recovered by {:.0} s after the fault", t(r) - a),
            None if rows.iter().filter(|x| t(x) > *a && t(x) < *b).all(healthy) => {
                println!("{name:<28} no visible effect")
            }
            None => unexplained.push(format!("{name}: never recovered")),
        }
    }
    let last = rows.last().context("empty")?;
    let (stamps, lost) = (
        last[8].parse::<f64>().unwrap_or(0.0),
        last[9].parse::<f64>().unwrap_or(0.0),
    );
    println!(
        "rows {} · longest gap {longest_gap:.0} s · reconnects {} · decode errors {} · STAMP loss {:.2} %",
        rows.len(),
        last[10],
        last[6],
        100.0 * lost / (stamps + lost).max(1.0)
    );
    for u in &unexplained {
        println!("UNEXPLAINED {u}");
    }
    anyhow::ensure!(
        longest_gap <= 20.0,
        "the log stopped for {longest_gap:.0} s: the app stalled or died"
    );
    anyhow::ensure!(healthy(last), "the soak did not end healthy");
    anyhow::ensure!(
        unexplained.is_empty(),
        "{} unexplained failure(s)",
        unexplained.len()
    );
    println!("S6: PASS");
    Ok(())
}

fn power_trace(scenario: &str) -> Result<()> {
    let sc =
        vc_sim::scenario::by_name(scenario).with_context(|| format!("no scenario '{scenario}'"))?;
    let mut sim = vc_sim::Simulator::new(sc.id, 0, None, false).map_err(|e| anyhow!("{e}"))?;
    sim.start(0);
    let mut out = heapless::Vec::<_, { vc_sim::sim::MAX_OUT }>::new();
    println!("t_s,power_w");
    let (mut sum, mut k) = (0.0, 0u64);
    while k * 250_000_000 < sc.total_ns() {
        k += 1;
        sim.step_into(k * 250_000_000, &mut out); // exactly one 250 ms physics step
        sum += sim.truth().map_or(0.0, |t| t.power_w);
        if k % 4 == 0 {
            println!("{},{:.3}", k / 4 - 1, sum / 4.0); // second s covers (s, s+1]
            sum = 0.0;
        }
    }
    Ok(())
}

fn wbal_cmd(path: &str, cp: f64, wprime: f64) -> Result<()> {
    use vc_physio::{WbalExact, WbalGc, WbalParams, WprimeBalance};
    let par = WbalParams::new(cp, wprime).context("CP and W′ must be positive")?;
    let (mut gc, mut exact) = (WbalGc::new(par), WbalExact::new(par));
    println!("t_s,power_w,wbal_gc_j,wbal_exact_j");
    for line in std::fs::read_to_string(path)?.lines().skip(1) {
        let mut f = line.split(',');
        let (Some(t), Some(p)) = (f.next(), f.next()) else {
            continue;
        };
        let (t, p): (u64, f64) = (t.parse()?, p.parse()?);
        let ns = t * 1_000_000_000;
        let (g, e) = (gc.update(ns, p), exact.update(ns, p));
        println!("{t},{p:.3},{g:.6},{e:.6}");
    }
    Ok(())
}

fn find_interrupt(scenario: &str) -> Result<()> {
    let fault = |at: u64| {
        let mut b = [0u8; vc_gatt::MAX_PAYLOAD];
        let n = vc_gatt::encode_ctrl(
            &vc_gatt::Ctrl::Fault {
                kind: vc_gatt::FaultKind::HrRedline,
                channel: None,
                dur_ms: 300_000,
                param: 0,
            },
            &mut b,
        )
        .map_err(|e| anyhow!("{e:?}"))?;
        Ok::<_, anyhow::Error>(vec![(at * 1_000_000_000, b[..n].to_vec())])
    };
    let mut hits = Vec::new();
    for at in 60..900u64 {
        let ctrl = fault(at)?;
        let run = vc_harness::run_sim(vc_harness::RunOpts {
            scenario,
            ctrl_at: &ctrl,
            ..Default::default()
        })
        .map_err(|e| anyhow!(e))?;
        if let Some(d) = run
            .decisions
            .iter()
            .find(|d| d.directive == vc_core::Directive::SafetyHr && d.interrupt)
        {
            hits.push((at, d.decided_ns as f64 / 1e9));
        }
    }
    for (at, t) in &hits {
        println!("fault at {at:>4} s -> safety interrupts at {t:.1} s");
    }
    Ok(())
}

fn persona_code(name: &str) -> Result<u8> {
    vc_sim::Persona::ALL
        .iter()
        .find(|p| p.name() == name)
        .map(|p| p.code())
        .with_context(|| format!("persona must be dana, sam, ned or calib, not '{name}'"))
}

fn sim_cmd(
    scenario: &str,
    persona: u8,
    seed: Option<u64>,
    mode: Mode,
    jsonl: Option<&str>,
    rider: &str,
) -> Result<()> {
    let mut run = run_sim(scenario, persona, seed, mode)?;

    if let Some(path) = jsonl {
        let persona_name = vc_sim::Persona::ALL
            .iter()
            .find(|p| p.code() == persona)
            .or_else(|| {
                vc_sim::scenario::by_name(scenario)
                    .map(|s| &s.persona)
                    .filter(|_| persona == 0)
            })
            .map_or("unknown", |p| p.name());
        let data = format!(
            "simulated persona={persona_name} seed={} link=offline",
            seed.map_or_else(|| "none".to_string(), |s| s.to_string())
        );
        write_jsonl(&mut run, path, rider, data)?;
    }

    print!("{}", vc_core::golden::HEADER);

    for decision in &run.decisions {
        let mut line = String::new();

        vc_core::golden::write_line(&mut line, decision).map_err(|_| anyhow!("format"))?;

        let truth = run
            .truth
            .iter()
            .find(|record| record.cue_id == decision.id)
            .map_or(String::new(), |record| {
                format!(
                    " | rider: {:?}{}",
                    record.outcome,
                    record
                        .latency_ms
                        .map_or(String::new(), |latency| format!(" after {latency} ms"))
                )
            });

        println!("{}{truth}", line.trim_end());
    }

    // Every observer outcome, so the tuner and the grade confounder are visible in a terminal.
    for o in &run.outcomes {
        let d = run.decisions.iter().find(|d| d.id == o.cue_id);
        println!(
            "cue {:>3} {:<20} {:<9} -> {:<10} L_obs {:>6} {}",
            o.cue_id,
            d.map_or("?", |d| d.directive.name()),
            d.map_or("?", |d| d.arm.name()),
            o.verdict.name(),
            o.l_obs_ms
                .map_or("-".into(), |l| format!("{:.1} s", f64::from(l) / 1000.0)),
            vc_efficacy::Confound::ALL
                .iter()
                .filter(|(c, _)| o.confound.contains(*c))
                .map(|(_, n)| format!("? {n}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    let spontaneous = run
        .truth
        .iter()
        .filter(|record| record.outcome == Outcome::Spontaneous)
        .count();

    println!(
        "\n{} decisions; {spontaneous} spontaneous corrections; \
         decode errors {}; implausible {}",
        run.decisions.len(),
        run.snapshot.decode_errors,
        run.snapshot.implausible,
    );

    Ok(())
}
