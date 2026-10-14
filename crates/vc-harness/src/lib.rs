//! Engine + Simulator in one process on a virtual clock: no radio, no sleeping.
//! Shared by vc-cli (sim, golden, jsonl) and the E1/E6/E8 tests.
use vc_core::{CueDecision, Engine, EngineConfig, Mode, Snapshot, Source};
use vc_efficacy::{Arm, CueOutcome, Ledger};
use vc_gatt::{Channel, Ctrl, MAX_PAYLOAD, encode_ctrl};
use vc_sim::sim::MAX_OUT;
use vc_sim::{ResponseRecord, SimPacket, Simulator, scenario};

const MS: u64 = 1_000_000;
const S: u64 = 1_000_000_000;
pub const ASSUMED_CLIP_MS: u16 = 2_400;

#[derive(Debug, Clone, Copy)]
pub struct RunOpts<'a> {
    pub scenario: &'a str,
    pub persona: u8,
    pub seed: Option<u64>,
    pub mode: Mode,
    pub mode_seed: u64,
}

impl Default for RunOpts<'_> {
    fn default() -> Self {
        Self {
            scenario: "climb_surge",
            persona: 0,
            seed: None,
            mode: Mode::Coach,
            mode_seed: 0,
        }
    }
}

pub struct SimRun {
    pub decisions: Vec<CueDecision>,
    /// NEW: every observer outcome, in order.
    pub outcomes: Vec<CueOutcome>,
    pub truth: Vec<ResponseRecord>,
    pub ledger: Ledger,
    pub snapshot: Snapshot,
    pub engine: Engine,
}

struct Harness {
    sim: Simulator,
    engine: Engine,
    playing: Vec<(u64, u64, u32)>, // (onset, end, cue id)
    decisions: Vec<CueDecision>,
    outcomes: Vec<CueOutcome>,
}

impl Harness {
    fn finish_due(&mut self, t: u64) {
        let engine = &mut self.engine;
        self.playing.retain(|&(onset, end, id)| {
            let done = end <= t;
            if done {
                engine.on_cue_played(id, onset, end, end); // CHANGED: both ends of the audio
            }
            !done
        });
    }

    fn play(&mut self, t: u64, d: CueDecision) -> Result<(), String> {
        self.decisions.push(d);
        if d.arm == Arm::Sham {
            return Ok(());
        }
        if d.interrupt {
            for (_, _, id) in self.playing.drain(..) {
                self.engine
                    .on_cue_failed(id, vc_core::engine::AudioFailure::Interrupted);
            }
        }
        let onset = t + u64::from(d.lead_ms) * MS;
        self.playing
            .push((onset, onset + u64::from(ASSUMED_CLIP_MS) * MS, d.id));
        let delta_w = d
            .delta_star_w
            .round()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
        let msg = Ctrl::CuePlayed {
            cue_id: d.id,
            kind: d.directive.code(),
            dir: d.dir,
            delta_w,
            arm: d.arm.code(),
            starts_in_ms: d.lead_ms,
            dur_ms: ASSUMED_CLIP_MS,
        };
        let mut b = [0u8; MAX_PAYLOAD];
        let n = encode_ctrl(&msg, &mut b).map_err(|e| format!("{e:?}"))?;
        self.sim.apply_ctrl(t, &b[..n]).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// NEW: collect each outcome once.
    fn collect_outcome(&mut self) {
        if let Some(o) = self.engine.last_outcome()
            && self
                .outcomes
                .last()
                .is_none_or(|last| last.cue_id != o.cue_id)
        {
            self.outcomes.push(o);
        }
    }
}

pub fn run_sim(o: RunOpts<'_>) -> Result<SimRun, String> {
    let sc = scenario::by_name(o.scenario)
        .ok_or_else(|| format!("no scenario named '{}'", o.scenario))?;
    let sim = Simulator::new(sc.id, o.persona, o.seed, false).map_err(|e| e.to_string())?;
    let cfg = EngineConfig {
        profile: sc.profile,
        ..EngineConfig::default()
    };
    let mut engine = Engine::new(cfg, sim.plan()).map_err(|e| e.to_string())?;
    engine
        .set_mode(o.mode, o.mode_seed)
        .map_err(|e| e.to_string())?;
    let mut h = Harness {
        sim,
        engine,
        playing: Vec::new(),
        decisions: Vec::new(),
        outcomes: Vec::new(),
    };
    h.sim.start(0);
    h.engine.start(0);

    let mut out = heapless::Vec::<SimPacket, MAX_OUT>::new();
    let end = sc.total_ns() + 20 * S; // NEW: 20 s extra so the last window can close
    let mut t = 0u64;
    while t <= end {
        h.finish_due(t);
        h.sim.step_into(t, &mut out);
        if let Some(truth) = h.sim.truth() {
            h.engine.set_grade(Some(truth.grade_pct)); // NEW: the Mac knows the road
        }
        for p in out.iter() {
            let source = match p.channel {
                Channel::Hrm => Source::Hr,
                Channel::Cpm => Source::Power,
                Channel::Csc => Source::Csc,
            };
            if let Ok(Some(d)) = h.engine.ingest(t, source, &p.bytes) {
                h.play(t, d)?;
            }
            h.collect_outcome();
        }
        if t.is_multiple_of(250 * MS)
            && let Some(d) = h.engine.tick(t)
        {
            h.play(t, d)?;
        }
        h.collect_outcome();
        t += 50 * MS;
    }
    Ok(SimRun {
        truth: h.sim.responses().to_vec(),
        ledger: h.engine.ledger().clone(),
        snapshot: h.engine.snapshot(),
        decisions: h.decisions,
        outcomes: h.outcomes,
        engine: h.engine,
    })
}
