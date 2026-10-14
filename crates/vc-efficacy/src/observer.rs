//! The cue-efficacy observer (plan §3.7). For one cue: anchor just before the
//! audio's onset, fit the pre-trend, project where power was heading, and score
//! movement beyond that inside the window. No clock, no randomness, no
//! allocation: everything arrives as an argument.
use crate::p3::{NS_PER_S, P3Ring};
use crate::{Confound, CueEvent, CueOutcome, Verdict};

pub const PRE_TREND_BINS: u64 = 10;
pub const MIN_TREND_POINTS: usize = 8;
pub const SLOPE_CLAMP_W_PER_S: f64 = 2.0;
pub const MIN_DELTA_W: f64 = 10.0;
/// The effect must persist over this many 1-s bins (3 s).
pub const HOLD_BINS: u64 = 3;
pub const WINDOW_AFTER_END_NS: u64 = 15 * NS_PER_S;
pub const COMPLY_FRAC: f64 = 0.5;
pub const PARTIAL_FRAC: f64 = 0.2;
pub const REVERSE_FRAC: f64 = 0.2;
pub const GRADE_CHANGE_PCT: f64 = 3.0;
pub const MIN_POWER_W: f64 = 20.0;
pub const COINCIDENTAL_NS: u64 = 350_000_000;
/// A step change is detected 1.5-2.5 s after it happens (3-s P3 over 1-s bins),
/// 2.0 s on average. E1 compares L_obs - SMOOTHING_LAG_MS with the truth.
pub const SMOOTHING_LAG_MS: i32 = 2_000;

/// What the Engine knows at each update, besides the power ring.
#[derive(Debug, Clone, Copy)]
pub struct ObsContext<'a> {
    pub ring: &'a P3Ring,
    pub power_fresh: bool,
    pub power_w: f64,
    pub cadence_rpm: Option<f64>,
    /// From the simulator on the Mac; from GPS or a barometer later. None on the phones today.
    pub grade_pct: Option<f64>,
    /// A P1 cue was decided since the last update.
    pub p1_fired: bool,
    /// Another cue's audio is playing inside this window.
    pub overlap: bool,
    pub audio_interrupted: bool,
    /// Human mode: the first slider move of at least 5 % in the asked direction.
    pub rider_move_ns: Option<u64>,
}

impl<'a> ObsContext<'a> {
    /// Everything normal: Fresh power at `power_w`, pedalling, nothing else happening.
    pub fn quiet(ring: &'a P3Ring, power_w: f64) -> Self {
        Self {
            ring,
            power_fresh: true,
            power_w,
            cadence_rpm: Some(90.0),
            grade_pct: None,
            p1_fired: false,
            overlap: false,
            audio_interrupted: false,
            rider_move_ns: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Window {
    ev: CueEvent,
    dir: f64,
    delta: f64,
    anchor: u64,
    p_anchor: f64,
    slope: f64,
    clamped: bool,
    next_k: u64,
    last_k: u64,
    end_bin: u64,
    grade0: Option<f64>,
    confound: Confound,
    max_hold: f64,
    rider_move_ns: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Observer {
    open: Option<Window>,
}

fn fabs(x: f64) -> f64 {
    if x < 0.0 { -x } else { x }
}

/// Signed milliseconds a - b.
fn ms(a: u64, b: u64) -> i32 {
    ((a as i64 - b as i64) / 1_000_000).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// OLS slope (W/s) of P3 over the 11 bins ending at `anchor`; None with < 8 points.
fn pre_trend(ring: &P3Ring, anchor: u64) -> Option<f64> {
    let first = anchor.checked_sub(PRE_TREND_BINS)?;
    let (mut n, mut sx, mut sy, mut sxx, mut sxy) = (0usize, 0.0, 0.0, 0.0, 0.0);
    for k in first..=anchor {
        if let Some(y) = ring.p3(k) {
            let x = (k - first) as f64;
            n += 1;
            sx += x;
            sy += y;
            sxx += x * x;
            sxy += x * y;
        }
    }
    if n < MIN_TREND_POINTS {
        return None;
    }
    let nf = n as f64;
    let den = nf * sxx - sx * sx;
    (den > 0.0).then(|| (nf * sxy - sx * sy) / den)
}

fn invalid(ev: &CueEvent) -> CueOutcome {
    CueOutcome {
        cue_id: ev.cue_id,
        verdict: Verdict::Invalid,
        confound: Confound::default(),
        coincidental: false,
        l_obs_ms: None,
        l_onset_ms: None,
        l_end_ms: None,
        pre_slope_w_per_s: 0.0,
        slope_clamped: false,
        effect_max_w: 0.0,
    }
}

impl Window {
    fn effect(&self, ring: &P3Ring, k: u64) -> Option<f64> {
        let projected = self.p_anchor + self.slope * (k - self.anchor) as f64;
        Some(self.dir * (ring.p3(k)? - projected))
    }

    /// H(k): the smallest effect over bins k .. k+2. None if any P3 is missing.
    fn hold(&self, ring: &P3Ring, k: u64) -> Option<f64> {
        let mut h = f64::INFINITY;
        for j in k..k + HOLD_BINS {
            h = h.min(self.effect(ring, j)?);
        }
        Some(h)
    }

    fn absorb(&mut self, ctx: &ObsContext<'_>) {
        let mut flag = |cond: bool, c: Confound| {
            if cond {
                self.confound.insert(c);
            }
        };
        flag(!ctx.power_fresh, Confound::QUALITY);
        flag(ctx.p1_fired, Confound::P1_FIRED);
        flag(
            matches!((self.grade0, ctx.grade_pct), (Some(g0), Some(g)) if fabs(g - g0) > GRADE_CHANGE_PCT),
            Confound::GRADE,
        );
        flag(
            ctx.cadence_rpm.is_some_and(|c| c <= 0.0),
            Confound::CADENCE_ZERO,
        );
        flag(ctx.power_w < MIN_POWER_W, Confound::COASTING);
        flag(ctx.overlap, Confound::OVERLAP);
        flag(ctx.audio_interrupted, Confound::AUDIO_INTERRUPTED);
        if self.rider_move_ns.is_none() {
            self.rider_move_ns = ctx.rider_move_ns;
        }
    }

    /// Score every second whose hold is complete. Some((verdict, bin)) when decided.
    fn step(&mut self, t_ns: u64, ctx: &ObsContext<'_>) -> Option<(Verdict, Option<u64>)> {
        if t_ns >= self.ev.t_onset_ns {
            self.absorb(ctx);
        }
        let complete = ctx.ring.newest_second()?.checked_sub(1)?; // the newest bin is still filling
        while self.next_k <= self.last_k && self.next_k + HOLD_BINS - 1 <= complete {
            let k = self.next_k;
            self.next_k += 1;
            let Some(h) = self.hold(ctx.ring, k) else {
                self.confound.insert(Confound::QUALITY); // a hole in the power data
                continue;
            };
            self.max_hold = self.max_hold.max(h);
            if h >= COMPLY_FRAC * self.delta {
                return Some((Verdict::Complied, Some(k)));
            }
            if h <= -REVERSE_FRAC * self.delta {
                return Some((Verdict::Reversed, Some(k)));
            }
        }
        if complete >= self.end_bin {
            let v = if self.max_hold >= PARTIAL_FRAC * self.delta {
                Verdict::Partial
            } else {
                Verdict::Ignored
            };
            return Some((v, None));
        }
        None
    }
}

impl Observer {
    pub const fn new() -> Self {
        Self { open: None }
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn open_cue(&self) -> Option<u32> {
        self.open.map(|w| w.ev.cue_id)
    }

    /// Start watching a cue once its onset and end are known (the audio callback,
    /// or the virtual timing of a sham). Returns an outcome at once if the cue
    /// cannot be scored. A second cue while a window is open is Invalid, and the
    /// open window is marked OVERLAP.
    pub fn open(
        &mut self,
        ev: &CueEvent,
        ring: &P3Ring,
        grade_pct: Option<f64>,
    ) -> Option<CueOutcome> {
        if let Some(w) = self.open.as_mut() {
            w.confound.insert(Confound::OVERLAP);
            return Some(invalid(ev));
        }
        let scoreable = ev.priority != 1
            && (ev.dir == 1 || ev.dir == -1)
            && ev.t_onset_ns > 0
            && ev.t_end_ns >= ev.t_onset_ns;
        if !scoreable {
            return Some(invalid(ev));
        }
        // The last complete second before the onset: a mid-phrase reaction cannot move the anchor.
        let Some(anchor) = (ev.t_onset_ns / NS_PER_S).checked_sub(1) else {
            return Some(invalid(ev));
        };
        let (Some(p_anchor), Some(raw)) = (ring.p3(anchor), pre_trend(ring, anchor)) else {
            return Some(invalid(ev));
        };
        let slope = raw.clamp(-SLOPE_CLAMP_W_PER_S, SLOPE_CLAMP_W_PER_S);
        let end_bin = (ev.t_end_ns + WINDOW_AFTER_END_NS) / NS_PER_S;
        self.open = Some(Window {
            ev: *ev,
            dir: f64::from(ev.dir),
            delta: fabs(ev.delta_star_w).max(MIN_DELTA_W),
            anchor,
            p_anchor,
            slope,
            clamped: slope != raw,
            next_k: anchor + 1,
            last_k: (end_bin + 1).saturating_sub(HOLD_BINS),
            end_bin,
            grade0: grade_pct,
            confound: Confound::default(),
            max_hold: f64::NEG_INFINITY,
            rider_move_ns: None,
        });
        None
    }

    /// Call after every power sample (and on ticks). Returns the outcome once decided.
    pub fn update(&mut self, t_ns: u64, ctx: &ObsContext<'_>) -> Option<CueOutcome> {
        let (verdict, bin) = self.open.as_mut()?.step(t_ns, ctx)?;
        self.finish(verdict, bin)
    }

    /// The ride stopped with a window open: it cannot be scored.
    pub fn abandon(&mut self) -> Option<CueOutcome> {
        self.open.take().map(|w| invalid(&w.ev))
    }

    fn finish(&mut self, verdict: Verdict, bin: Option<u64>) -> Option<CueOutcome> {
        let w = self.open.take()?;
        let (onset, end) = (w.ev.t_onset_ns, w.ev.t_end_ns);
        let l_obs_ms = bin.map(|k| ms((k + 1) * NS_PER_S, end)); // evidence exists at the end of bin k
        let (l_onset_ms, l_end_ms, coincidental) = match w.rider_move_ns {
            Some(m) => (
                Some(ms(m, onset)),
                Some(ms(m, end)),
                m < onset + COINCIDENTAL_NS,
            ),
            None => (None, None, false),
        };
        Some(CueOutcome {
            cue_id: w.ev.cue_id,
            verdict: if w.confound.is_empty() {
                verdict
            } else {
                Verdict::Confounded
            },
            confound: w.confound,
            coincidental,
            l_obs_ms,
            l_onset_ms,
            l_end_ms,
            pre_slope_w_per_s: w.slope,
            slope_clamped: w.clamped,
            effect_max_w: if w.max_hold.is_finite() {
                w.max_hold
            } else {
                0.0
            },
        })
    }
}
