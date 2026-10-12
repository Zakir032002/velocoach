//! Scripted rides (feature `testkit`, std): constant streams of real GATT
//! bytes, fed exactly as the phones will. Power at 4 Hz, HR at 1 Hz,
//! a tick every 250 ms.

use vc_gatt::{Contact, Cpm, CrankData, Hrm, MAX_PAYLOAD, encode_cpm, encode_hrm};

use crate::{CueDecision, Directive, Engine, EngineConfig, EngineError, Source, WorkoutPlan};

const S: u64 = 1_000_000_000;
const STEP_NS: u64 = 250_000_000;

/// What the sensors report until changed. None = that sensor is silent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stream {
    pub power_w: Option<f64>,
    pub hr_bpm: Option<u16>,
    pub cadence_rpm: f64,
}

impl Default for Stream {
    fn default() -> Self {
        Self {
            power_w: Some(150.0),
            hr_bpm: Some(120),
            cadence_rpm: 90.0,
        }
    }
}

pub fn cpm_bytes(watts: f64, crank: Option<CrankData>) -> Vec<u8> {
    let mut b = [0u8; MAX_PAYLOAD];
    let m = Cpm {
        power_w: libm::round(watts) as i16,
        crank,
        ..Cpm::default()
    };

    let n = encode_cpm(&m, &mut b).unwrap_or(0);
    b[..n].to_vec()
}

pub fn hrm_bytes(bpm: u16) -> Vec<u8> {
    let mut b = [0u8; MAX_PAYLOAD];
    let m = Hrm {
        bpm,
        contact: Contact::Detected,
        energy_kj: None,
        rr_1024: Default::default(),
    };

    let n = encode_hrm(&m, &mut b).unwrap_or(0);
    b[..n].to_vec()
}

pub struct Ride {
    pub engine: Engine,
    pub stream: Stream,
    /// (seconds, decision) for everything the Engine returned.
    pub log: Vec<(f64, CueDecision)>,
    t_ns: u64,
    revs_f: f64,
    revs: u16,
    event_1024: u16,
}

impl Ride {
    pub fn new(cfg: EngineConfig, plan: WorkoutPlan) -> Result<Self, EngineError> {
        let mut engine = Engine::new(cfg, plan)?;
        engine.start(0);

        Ok(Self {
            engine,
            stream: Stream::default(),
            log: Vec::new(),
            t_ns: 0,
            revs_f: 0.0,
            revs: 0,
            event_1024: 0,
        })
    }

    pub fn now_s(&self) -> f64 {
        self.t_ns as f64 / 1e9
    }

    /// Ride on with the current stream for `secs`.
    pub fn run(&mut self, secs: f64) {
        let steps = libm::round(secs * 4.0) as u64;

        for _ in 0..steps {
            let t = self.t_ns;

            if t > 0 {
                self.advance_crank(t);
            }

            if let Some(w) = self.stream.power_w {
                let crank = CrankData {
                    revs: self.revs,
                    time_1024: self.event_1024,
                };
                let bytes = cpm_bytes(w, Some(crank));
                self.feed(t, Source::Power, &bytes);
            }

            if t.is_multiple_of(S)
                && let Some(bpm) = self.stream.hr_bpm
            {
                self.feed(t, Source::Hr, &hrm_bytes(bpm));
            }

            if let Some(d) = self.engine.tick(t) {
                self.log.push((t as f64 / 1e9, d));
            }

            self.t_ns += STEP_NS;
        }
    }

    /// Times of every decision for one directive.
    pub fn times(&self, d: Directive) -> Vec<f64> {
        self.log
            .iter()
            .filter(|(_, c)| c.directive == d)
            .map(|(t, _)| *t)
            .collect()
    }

    fn feed(&mut self, t: u64, source: Source, bytes: &[u8]) {
        if let Ok(Some(d)) = self.engine.ingest(t, source, bytes) {
            self.log.push((t as f64 / 1e9, d));
        }
    }

    /// Integrate cadence over the 250 ms ending at `t`, as a real crank
    /// sensor would.
    fn advance_crank(&mut self, t: u64) {
        let rpm = self.stream.cadence_rpm;

        if rpm <= 0.0 {
            return; // Coasting: both counters repeat.
        }

        let (dt, t_end) = (0.25, t as f64 / 1e9);
        let rev_per_s = rpm / 60.0;
        let (before, after) = (self.revs_f, self.revs_f + rev_per_s * dt);

        if libm::floor(after) > libm::floor(before) {
            let t_last = t_end - dt + (libm::floor(after) - before) / rev_per_s;

            self.revs = self
                .revs
                .wrapping_add((libm::floor(after) - libm::floor(before)) as u16);

            self.event_1024 = (libm::round(t_last * 1024.0) as u64 % 65_536) as u16;
        }

        self.revs_f = after;
    }
}
