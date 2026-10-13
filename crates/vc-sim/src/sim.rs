//! A deterministic virtual rider that emits sensor bytes and reacts to cues.
use heapless::Vec;
use vc_core::WorkoutPlan;
use vc_core::study::Pcg32;
use vc_gatt::{Channel, Ctrl, FaultKind, decode_ctrl};

use crate::SimError;
use crate::faults::{CoastMode, Faults, Injected};
use crate::packets::{Payload, ch_index, garbage, sensor_packet};
use crate::persona::{Persona, Reaction, spontaneous_per_step};
use crate::rider::{Effort, Rider, RiderFaults, STEP_NS, Truth};
use crate::scenario::{Scenario, by_id};

const MS: u64 = 1_000_000;
const S: u64 = 1_000_000_000;

pub const MAX_RECORDS: usize = 512;
pub const MAX_OUT: usize = 32;
const MAX_STEPS_PER_CALL: u32 = 2_400;
const MIN_DEVIATION_W: f64 = 5.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimPacket {
    pub channel: Channel,
    /// Per-characteristic sequence number.
    pub char_seq: u16,
    pub due_ns: u64,
    pub bytes: Payload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtrlOutcome {
    Ack,
    Pong { ping_id: u16, t_rx_ns: u64 },
    Disconnect { dur_ms: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Complied,
    Ignored,
    NotApplicable,
    Spontaneous,
}

/// Ground truth about the rider's actual response. Never sent over BLE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResponseRecord {
    pub cue_id: u32,
    pub kind: u8,
    pub arm: u8,
    pub rx_ns: u64,
    pub audio_end_ns: u64,
    pub outcome: Outcome,
    pub latency_ms: Option<u32>,
    pub applied_ns: Option<u64>,
    pub offset_w: f64,
}

#[derive(Debug, Clone, Copy)]
struct Pending {
    at_ns: u64,
    offset_w: f64,
    record: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Simulator {
    sc: &'static Scenario,
    /// None in human mode: a person, not a model, changes effort.
    persona: Option<Persona>,
    seed: u64,
    human: bool,
    effort_permille: i16,
    run: Option<Run>,
}

#[derive(Debug, Clone)]
struct Run {
    start_ns: u64,
    rider: Rider,
    rng: Pcg32,
    faults: Faults,
    next_fault: usize,
    steps_done: u64,
    coach_index: Option<usize>,
    creep_w: f64,
    response_w: f64,
    pending: Option<Pending>,
    records: Vec<ResponseRecord, MAX_RECORDS>,
    seq: [u16; 3],
    garbage_k: u32,
    cpm_every: u64,
    pending_disconnect: Option<u32>,
}

impl Simulator {
    /// `persona_code == 0` selects the scenario default.
    /// `seed == None` selects the scenario's seed.
    pub fn new(
        scenario_id: u8,
        persona_code: u8,
        seed: Option<u64>,
        human: bool,
    ) -> Result<Self, SimError> {
        let sc = by_id(scenario_id).ok_or(SimError::UnknownScenario(scenario_id))?;

        let persona = match (human, persona_code) {
            (true, _) => None,
            (false, 0) => Some(sc.persona),
            (false, code) => Some(Persona::from_code(code).ok_or(SimError::UnknownPersona(code))?),
        };

        Ok(Self {
            sc,
            persona,
            seed: seed.unwrap_or(sc.seed),
            human,
            effort_permille: 600,
            run: None,
        })
    }

    pub fn scenario(&self) -> &'static Scenario {
        self.sc
    }

    pub fn name(&self) -> &'static str {
        self.sc.name
    }

    pub fn plan(&self) -> WorkoutPlan {
        self.sc.plan()
    }

    pub fn persona(&self) -> Option<Persona> {
        self.persona
    }

    pub fn is_running(&self) -> bool {
        self.run.is_some()
    }

    pub fn truth(&self) -> Option<Truth> {
        self.run.as_ref().map(|r| r.rider.truth())
    }

    pub fn responses(&self) -> &[ResponseRecord] {
        match &self.run {
            Some(run) => &run.records,
            None => &[],
        }
    }

    /// Human-mode effort, as thousandths of CP, clamped to 0–2000.
    pub fn set_human_effort(&mut self, permille: i16) {
        self.effort_permille = permille.clamp(0, 2_000);
    }

    pub fn start(&mut self, now_ns: u64) {
        self.run = Some(Run {
            start_ns: now_ns,
            rider: Rider::new(self.sc.rider),
            rng: Pcg32::new(self.seed, 0),
            faults: Faults::default(),
            next_fault: 0,
            steps_done: 0,
            coach_index: None,
            creep_w: 0.0,
            response_w: 0.0,
            pending: None,
            records: Vec::new(),
            seq: [0; 3],
            garbage_k: 0,
            cpm_every: 1,
            pending_disconnect: None,
        });
    }

    pub fn stop(&mut self) {
        self.run = None;
    }

    pub fn take_disconnect(&mut self) -> Option<u32> {
        self.run.as_mut()?.pending_disconnect.take()
    }

    /// Clear `out` and fill it with packets due by `now_ns`.
    pub fn step_into(&mut self, now_ns: u64, out: &mut Vec<SimPacket, MAX_OUT>) {
        out.clear();

        let human_w = self
            .human
            .then(|| f64::from(self.effort_permille) * self.sc.rider.cp_w / 1000.0);
        let (sc, modelled) = (self.sc, self.persona.is_some());

        let Some(run) = self.run.as_mut() else {
            return;
        };

        let elapsed = now_ns.saturating_sub(run.start_ns);
        let mut count = 0;

        while (run.steps_done + 1) * STEP_NS <= elapsed && count < MAX_STEPS_PER_CALL {
            let k = run.steps_done + 1;
            run.step(sc, k * STEP_NS, human_w, modelled);
            run.emit(k, out);
            run.steps_done = k;
            count += 1;
        }
    }

    pub fn apply_ctrl(&mut self, now_ns: u64, bytes: &[u8]) -> Result<CtrlOutcome, SimError> {
        match decode_ctrl(bytes)? {
            Ctrl::Ping { ping_id, .. } => Ok(CtrlOutcome::Pong {
                ping_id,
                t_rx_ns: now_ns,
            }),

            Ctrl::Load {
                scenario,
                persona,
                seed,
                human,
            } => {
                let effort = self.effort_permille;
                *self = Self::new(scenario, persona, Some(u64::from(seed)), human)?;
                self.effort_permille = effort;
                Ok(CtrlOutcome::Ack)
            }

            Ctrl::Start => {
                self.start(now_ns);
                Ok(CtrlOutcome::Ack)
            }

            Ctrl::Stop => {
                self.stop();
                Ok(CtrlOutcome::Ack)
            }

            Ctrl::Fault {
                kind,
                channel,
                dur_ms,
                param,
            } => {
                let run = self.run.as_mut().ok_or(SimError::NotStarted)?;
                let at = now_ns.saturating_sub(run.start_ns);

                Ok(match run.inject(kind, channel, at, dur_ms, param) {
                    Some(ms) => CtrlOutcome::Disconnect { dur_ms: ms },
                    None => CtrlOutcome::Ack,
                })
            }

            Ctrl::Clear { kind } => {
                if let Some(run) = self.run.as_mut() {
                    run.faults.clear(kind);
                }
                Ok(CtrlOutcome::Ack)
            }

            Ctrl::Rate { channel, hz_x10 } => {
                if hz_x10 == 0 {
                    return Err(SimError::BadRate);
                }

                let run = self.run.as_mut().ok_or(SimError::NotStarted)?;

                if channel.is_none_or(|ch| ch == Channel::Cpm) {
                    run.cpm_every = match hz_x10 {
                        40.. => 1,
                        20..=39 => 2,
                        _ => 4,
                    };
                }

                Ok(CtrlOutcome::Ack)
            }

            Ctrl::CuePlayed {
                cue_id,
                kind,
                dir,
                delta_w,
                arm,
                starts_in_ms,
                dur_ms,
            } => {
                let (seed, persona) = (self.seed, self.persona);
                let run = self.run.as_mut().ok_or(SimError::NotStarted)?;

                let rx = now_ns.saturating_sub(run.start_ns);
                let audio_end = rx + (u64::from(starts_in_ms) + u64::from(dur_ms)) * MS;

                let reaction = persona.map_or(Reaction::NotApplicable, |p| {
                    p.react(seed, cue_id, arm, dir, delta_w, run.rider.wprime_fraction())
                });

                run.on_cue(cue_id, kind, arm, rx, audio_end, reaction);
                Ok(CtrlOutcome::Ack)
            }
        }
    }
}

impl Run {
    fn step(&mut self, sc: &Scenario, t: u64, human_w: Option<f64>, modelled: bool) {
        let step_start = t - STEP_NS;
        self.inject_scheduled(sc, step_start);
        let at = sc.at(step_start);

        // A new coach-visible step resets its rider-specific deviation.
        let coach = at.map(|a| a.coach_index);
        if coach != self.coach_index {
            self.coach_index = coach;
            self.creep_w = 0.0;
            self.response_w = 0.0;
            self.pending = None;
        }

        // Apply any response whose scheduled time has arrived.
        if let Some(pending) = self.pending
            && pending.at_ns <= t
        {
            self.response_w += pending.offset_w;
            self.pending = None;

            if let Some(record) = pending.record.and_then(|i| self.records.get_mut(i)) {
                record.applied_ns = Some(t);
            }
        }

        // Draw on every step, even when the rider is not eligible to correct.
        let spontaneous_draw = self.rng.next_f64();

        let (offset, creep_rate, grade, target, rpm) = match at {
            Some(a) => (
                a.sim.offset_w,
                a.sim.creep_w_per_s,
                a.sim.grade_pct,
                a.sim.step.target_w.unwrap_or(150.0),
                a.sim.step.cadence_rpm.unwrap_or(90.0),
            ),
            None => (0.0, 0.0, 0.0, 0.0, 0.0),
        };

        let deviation = offset + self.creep_w + self.response_w;

        if modelled
            && spontaneous_draw < spontaneous_per_step()
            && libm::fabs(deviation) >= MIN_DEVIATION_W
        {
            self.response_w -= deviation;

            let _ = self.records.push(ResponseRecord {
                cue_id: 0,
                kind: 0,
                arm: 0,
                rx_ns: t,
                audio_end_ns: t,
                outcome: Outcome::Spontaneous,
                latency_ms: None,
                applied_ns: Some(t),
                offset_w: -deviation,
            });
        }

        self.creep_w += creep_rate * 0.25;

        let base = human_w.unwrap_or(target);
        let effort = Effort {
            target_w: (base + offset + self.creep_w + self.response_w).max(0.0),
            target_rpm: rpm,
            grade_pct: grade,
        };

        let faults = RiderFaults {
            coasting: self.faults.coast_at(t).is_some(),
            hr_redline: self.faults.redline_at(t),
        };

        self.rider.step(t, effort, faults, &mut self.rng);
    }

    fn on_cue(
        &mut self,
        cue_id: u32,
        kind: u8,
        arm: u8,
        rx: u64,
        audio_end: u64,
        reaction: Reaction,
    ) {
        let (outcome, latency_s, offset_w) = match reaction {
            Reaction::NotApplicable => (Outcome::NotApplicable, None, 0.0),
            Reaction::Ignore => (Outcome::Ignored, None, 0.0),
            Reaction::Respond {
                latency_s,
                offset_w,
            } => (Outcome::Complied, Some(latency_s), offset_w),
        };

        let record_index = self.records.len();
        let stored = self
            .records
            .push(ResponseRecord {
                cue_id,
                kind,
                arm,
                rx_ns: rx,
                audio_end_ns: audio_end,
                outcome,
                latency_ms: latency_s.map(|s| libm::round(s * 1000.0) as u32),
                applied_ns: None,
                offset_w,
            })
            .is_ok();

        if let Some(latency_s) = latency_s {
            // A newer instruction replaces one not yet acted on.
            self.pending = Some(Pending {
                at_ns: audio_end + libm::round(latency_s * 1e9) as u64,
                offset_w,
                record: stored.then_some(record_index),
            });
        }
    }

    fn emit(&mut self, k: u64, out: &mut Vec<SimPacket, MAX_OUT>) {
        let t = k * STEP_NS;
        let truth = self.rider.truth();

        let silent = self.faults.coast_at(t) == Some(CoastMode::Silent);
        let crank = (!silent).then(|| self.rider.crank());

        let due = [
            k.is_multiple_of(4),
            k.is_multiple_of(self.cpm_every),
            k % 4 == 2,
        ];

        for (ch, is_due) in [Channel::Hrm, Channel::Cpm, Channel::Csc]
            .into_iter()
            .zip(due)
        {
            if !is_due || self.faults.silenced(ch, t) {
                continue;
            }

            let bytes = if self.faults.take_garbage(ch) {
                self.garbage_k = self.garbage_k.wrapping_add(1);
                garbage(self.garbage_k)
            } else {
                let Some(bytes) = sensor_packet(ch, &truth, crank) else {
                    continue;
                };
                bytes
            };

            let index = ch_index(ch);
            if let Some(seq) = self.seq.get_mut(index) {
                *seq = seq.wrapping_add(1);
            }

            let char_seq = self.seq.get(index).copied().unwrap_or(0);

            if out.is_full() {
                out.remove(0);
            }

            let _ = out.push(SimPacket {
                channel: ch,
                char_seq,
                due_ns: t,
                bytes,
            });
        }
    }

    fn inject_scheduled(&mut self, sc: &Scenario, t_ns: u64) {
        while let Some(fault) = sc.faults.get(self.next_fault) {
            let at_ns = u64::from(fault.at_s) * S;
            if at_ns > t_ns {
                break;
            }

            if let Some(ms) =
                self.inject(fault.kind, fault.channel, at_ns, fault.dur_ms, fault.param)
            {
                self.pending_disconnect = Some(ms);
            }

            self.next_fault += 1;
        }
    }

    fn inject(
        &mut self,
        kind: FaultKind,
        channel: Option<Channel>,
        at_ns: u64,
        dur_ms: u32,
        param: i16,
    ) -> Option<u32> {
        match self.faults.inject(kind, channel, at_ns, dur_ms, param) {
            Injected::Done => None,
            Injected::SetTrueWprime(joules) => {
                self.rider.set_true_wprime(joules);
                None
            }
            Injected::Disconnect(ms) => Some(ms),
        }
    }
}
