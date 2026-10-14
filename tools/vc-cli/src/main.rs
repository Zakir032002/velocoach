use anyhow::{Context, Result, anyhow};
use vc_core::engine::AudioFailure;
use vc_core::{Arm, CueDecision, Engine, EngineConfig, Mode, Snapshot, Source};
use vc_gatt::{Channel, Ctrl, MAX_PAYLOAD, encode_ctrl};
use vc_sim::sim::MAX_OUT;
use vc_sim::{Outcome, ResponseRecord, SimPacket, Simulator, scenario};

const MS: u64 = 1_000_000;
const S: u64 = 1_000_000_000;
const ASSUMED_CLIP_MS: u16 = 2_400;

pub struct SimRun {
    pub decisions: Vec<CueDecision>,
    pub truth: Vec<ResponseRecord>,
    pub snapshot: Snapshot,
}

struct Harness {
    sim: Simulator,
    engine: Engine,
    playing: Vec<(u64, u64, u32)>, // (audio end time, cue ID)
    decisions: Vec<CueDecision>,
}
impl Harness {
    fn finish_due(&mut self, t: u64) {
        let engine = &mut self.engine;

        self.playing.retain(|&(onset, end, id)| {
            let done = end <= t;
            if done {
                engine.on_cue_played(id, onset, end, end);
            }
            !done
        });
    }

    fn play(&mut self, t: u64, d: CueDecision) -> Result<()> {
        self.decisions.push(d);

        // Sham decisions have no audio and are not sent to the rider.
        if d.arm == Arm::Sham {
            return Ok(());
        }

        // A P1 interrupt terminates whatever clip is currently playing.
        if d.interrupt {
            for (_, _, id) in self.playing.drain(..) {
                self.engine.on_cue_failed(id, AudioFailure::Interrupted);
            }
        }

        let onset = t + u64::from(d.lead_ms) * MS;
        let end = onset + u64::from(ASSUMED_CLIP_MS) * MS;
        self.playing.push((onset, end, d.id));

        let delta_w = d
            .delta_star_w
            .round()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;

        let msg = Ctrl::CuePlayed {
            cue_id: d.id,
            kind: d.directive.code(),
            dir: d.dir,
            delta_w,
            arm: arm_code(d.arm),
            starts_in_ms: d.lead_ms,
            dur_ms: ASSUMED_CLIP_MS,
        };

        let mut bytes = [0u8; MAX_PAYLOAD];
        let n = encode_ctrl(&msg, &mut bytes).map_err(|e| anyhow!("{e:?}"))?;

        self.sim
            .apply_ctrl(t, &bytes[..n])
            .map_err(|e| anyhow!("{e}"))?;

        Ok(())
    }
}

/// Run the Engine and Simulator together using virtual time.
fn run_sim(name: &str, persona: u8, seed: Option<u64>, mode: Mode) -> Result<SimRun> {
    let sc = scenario::by_name(name).with_context(|| format!("no scenario named '{name}'"))?;

    let sim = Simulator::new(sc.id, persona, seed, false).map_err(|e| anyhow!("{e}"))?;

    let cfg = EngineConfig {
        profile: sc.profile,
        ..EngineConfig::default()
    };

    let mut engine = Engine::new(cfg, sim.plan()).map_err(|e| anyhow!("{e}"))?;

    if mode == Mode::Study {
        engine
            .set_mode(mode, seed.unwrap_or(0))
            .map_err(|e| anyhow!("{e}"))?;
    }
    let mut h = Harness {
        sim,
        engine,
        playing: Vec::new(),
        decisions: Vec::new(),
    };

    h.sim.start(0);
    h.engine.start(0);

    let mut out = heapless::Vec::<SimPacket, MAX_OUT>::new();
    let end = sc.total_ns() + 5 * S;
    let mut t = 0u64;

    while t <= end {
        // 1. Complete clips whose simulated audio has ended.
        h.finish_due(t);

        // 2. Generate sensor packets and ingest them into the Engine.
        h.sim.step_into(t, &mut out);

        for packet in out.iter() {
            let source = match packet.channel {
                Channel::Hrm => Source::Hr,
                Channel::Cpm => Source::Power,
                Channel::Csc => Source::Csc,
            };

            if let Ok(Some(decision)) = h.engine.ingest(t, source, &packet.bytes) {
                h.play(t, decision)?;
            }
        }

        // 3. Run the policy engine every 250 ms.
        if t.is_multiple_of(250 * MS)
            && let Some(decision) = h.engine.tick(t)
        {
            h.play(t, decision)?;
        }

        // Advance virtual time by 50 ms. Never sleep.
        t += 50 * MS;
    }

    Ok(SimRun {
        truth: h.sim.responses().to_vec(),
        snapshot: h.engine.snapshot(),
        decisions: h.decisions,
    })
}

fn arm_code(arm: Arm) -> u8 {
    match arm {
        Arm::None => 0,
        Arm::Direct => 1,
        Arm::Autonomy => 2,
        Arm::Sham => 3,
    }
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

    match args.get(1).map(String::as_str) {
        Some("sim") => {
            let flag = |name: &str| {
                args.iter()
                    .position(|arg| arg == name)
                    .and_then(|i| args.get(i + 1))
                    .map(String::as_str)
            };

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

            sim_cmd(scenario, persona, seed, mode)
        }
        Some("golden") => golden(args.iter().any(|arg| arg == "--check")),
        Some("hex") => hex_cmd(&args[2..]),
        Some("phrases") => phrases_cmd(),
        _ => {
            eprintln!(
                "Usage:\n  vc-cli sim [--scenario NAME] \
                 [--persona NAME] [--seed N]\n  vc-cli golden [--check]"
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

fn persona_code(name: &str) -> Result<u8> {
    vc_sim::Persona::ALL
        .iter()
        .find(|p| p.name() == name)
        .map(|p| p.code())
        .with_context(|| format!("persona must be dana, sam, ned or calib, not '{name}'"))
}

fn sim_cmd(scenario: &str, persona: u8, seed: Option<u64>, mode: Mode) -> Result<()> {
    let run = run_sim(scenario, persona, seed, mode)?;

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
