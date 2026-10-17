# Efficacy: how velocoach-rs scores its own cues

Observer version `vc-efficacy 0.3.0` (frozen for the pilot; every change is in
[EFFICACY_CHANGELOG.md](EFFICACY_CHANGELOG.md)). Every number below is reproducible with the
commands at the end. Back to the [README](../README.md).

> **Status of the instrument's own validation (read this first).** All results here are
> **measured in simulation** (simulated riders, no real rider or sensor data). E2, E3, E4, E6
> and E8 meet their criteria. **E1 does not meet its criterion.** Observer 0.2.0, as first
> specified, had 0/20 seeds within tolerance. Observer 0.3.0, changed for stated reasons
> **after** that failure ([changelog](EFFICACY_CHANGELOG.md)), gives
> **15/20 seeds within tolerance (seeds 4,5,8,12,17 fail: 0.705/2.42 s, 0.720/2.42 s,
> 0.595/2.67 s, 0.615/3.17 s, 0.660/2.42 s)** and, on held-out seeds 1000–1019,
> **17/20 seeds within tolerance (seeds 1003,1005,1013 fail: 0.595/2.42 s, 0.555/2.92 s,
> 0.590/3.17 s)**, against an amended criterion of ≥ 18/20. Compliance rates from this
> observer carry that caveat. Human pilot: planned, pre-registered, not yet run.

## What is measured

For each scored cue: did the rider's power move in the asked direction by at least half
the asked amount, and hold it, within 15 s of the end of the phrase, compared with where
power was already heading? The answer is one of: complied, partial, ignored, reversed,
confounded, invalid.

Scored: P2 cues (all arms, including shams), the P3 cardiac-drift cue, and the surge's
"go now". Not scored: safety, sensor dropout, countdowns (they are not requests to
change power).

## The math

- P3(k): the mean of three consecutive 1-s bins of power (a 40-bin ring).
- Anchor: the last complete second before the phrase's onset, so a reaction in the middle
  of the phrase cannot move the baseline.
- Pre-trend: ordinary least squares over P3 for the 10 s up to the anchor (at least 8
  points), clamped to +/-2 W/s.
- Counterfactual: P̂(k) = P3(anchor) + slope x (k - anchor).
- Effect: E(k) = d x (P3(k) - P̂(k)), where d is the asked direction (+1/-1).
- Hold: H(k) = the minimum of E over 3 consecutive bins.
- Window: from the onset to the end of the phrase + 15 s. The asked amount is
  max(|Δ*|, 10 W).
- The rule, from `crates/vc-efficacy/src/observer.rs` (`Window::step`):
  - complied as soon as some H(k) >= 0.5 x Δ (`COMPLY_FRAC`);
  - reversed as soon as the *largest* E over the same three bins is <= -0.2 x Δ
    (`REVERSE_FRAC`): a reversal must hold for 3 s, exactly as compliance must (0.3.0;
    0.2.0 used one bin), whichever of the two comes first;
  - at the end of the window: partial if the largest H seen reached 0.2 x Δ
    (`PARTIAL_FRAC`), otherwise ignored.
- L_obs = the end of the second in which the hold first reached the complied threshold,
  minus the end of the phrase (it can be negative). It includes a smoothing lag that is
  **derived, not fitted** (`observer.rs`):

      SMOOTHING_LAG_MS = (N - 1)/2 · Ts + BIN_STAMP_DELAY_MS
                       = (12 - 1)/2 · 250 ms + 500 ms = 1375 + 500 = 1875 ms

  where N = `P3_WINDOW_S` x `POWER_RATE_HZ` = 3 s x 4 Hz samples, Ts = 1/`POWER_RATE_HZ`.
  The first term is the group delay of an N-sample trailing mean (its step response crosses
  50 % there); the second is half a 1-s bin, because L_obs is stamped at the end of the bin.
  0.2.0 used 2000 ms; an experiment fitted to E1 had suggested 1500 ms, which was not used.
- Human mode: L_onset = the first slider move of >= 5 % in the asked direction minus the
  onset; a move within 0.35 s is flagged coincidental.

## Confounders: a reason to say "don't know"

| Flag | Condition |
|---|---|
| quality | power not Fresh, or any missing bin, anywhere in the window |
| grade | the road grade changed by more than 3 % (Mac harness only: phones have no grade) |
| coasting | power under 20 W |
| cadence_zero | cadence 0 rpm while power is reported |
| p1_fired | a P1 cue was decided inside the window |
| overlap | another cue's audio inside the window (blocked by the Policy Engine; unit-tested) |
| audio_interrupted | the phrase did not finish |

## Personas are assumptions

| Persona | Complies | Latency median | Notes |
|---|---|---|---|
| Dana | 90 % | 1.8 s | the cooperative rider |
| Sam | 30 % direct / 80 % autonomy | 2.5 s | ignores cues below 10 % W' |
| Ned | 50 % either arm | 2.5 s | the null rider (E8) |
| Calib | 70 % | 3.0 s | the known rider (E1) |

Latency is log-normal around the median (sigma 0.35-0.45), clamped to 0.6-12 s. All
respond 5 % of the time (per 15 s window) without a cue. These are design choices for
validating the instrument, not claims about real riders (`crates/vc-sim/src/persona.rs`).

## E1: the observer recovers a known rider — criterion **not met**

The test (`cargo test --release -p vc-harness --test e1 -- --nocapture`): per seed, 200 scored
cues on the `creep` scenario with Calib (hidden compliance 0.70, median latency 3.0 s).

Criterion: ~~"200 seeded cues, Calib 70 %/3.0 s → estimate within ±0.08 / ±0.5 s over 20
seeds"~~ → **"≥ 18/20 seeds within ±0.08 compliance and ±0.5 s median latency"** (amended
2026-10-16, after the initial failure on 20/20 seeds; see the
[changelog](EFFICACY_CHANGELOG.md)). Median latency is L_obs minus `SMOOTHING_LAG_MS`.

Which cues count is defined from constants, not tuned on seeds: only Direct and Autonomy
phrasings (the persona model answers nothing else), and only cues where audio end +
`MIN_LATENCY_S` (0.6 s) + `SMOOTHING_LAG_MS` + the 3-s hold fits before the window closes and
before the next workout step.

### Observer 0.3.0, development seeds 0–19

**15/20 seeds within tolerance (seeds 4,5,8,12,17 fail: 0.705/2.42 s, 0.720/2.42 s,
0.595/2.67 s, 0.615/3.17 s, 0.660/2.42 s).**

| seed | compliance | median latency | sessions | skipped: no arm | skipped: infeasible | within tolerance |
|---|---|---|---|---|---|---|
| 0 | 0.675 | 2.67 s | 38 | 272 | 3 | yes |
| 1 | 0.625 | 2.67 s | 40 | 298 | 3 | yes |
| 2 | 0.725 | 2.67 s | 39 | 285 | 4 | yes |
| 3 | 0.625 | 2.92 s | 40 | 293 | 3 | yes |
| 4 | 0.705 | 2.42 s | 38 | 281 | 3 | **no** |
| 5 | 0.720 | 2.42 s | 40 | 290 | 7 | **no** |
| 6 | 0.720 | 2.67 s | 37 | 269 | 2 | yes |
| 7 | 0.620 | 2.67 s | 40 | 285 | 6 | yes |
| 8 | 0.595 | 2.67 s | 37 | 260 | 6 | **no** |
| 9 | 0.695 | 2.67 s | 38 | 279 | 0 | yes |
| 10 | 0.700 | 2.80 s | 39 | 282 | 5 | yes |
| 11 | 0.740 | 2.67 s | 39 | 286 | 6 | yes |
| 12 | 0.615 | 3.17 s | 40 | 295 | 0 | **no** |
| 13 | 0.670 | 2.92 s | 40 | 288 | 3 | yes |
| 14 | 0.720 | 2.92 s | 39 | 276 | 3 | yes |
| 15 | 0.645 | 2.92 s | 38 | 275 | 6 | yes |
| 16 | 0.655 | 2.67 s | 36 | 260 | 1 | yes |
| 17 | 0.660 | 2.42 s | 39 | 285 | 5 | **no** |
| 18 | 0.645 | 2.67 s | 39 | 274 | 0 | yes |
| 19 | 0.695 | 3.17 s | 38 | 277 | 2 | yes |

### Observer 0.3.0, held-out seeds 1000–1019 (never run while the observer was changed)

**17/20 seeds within tolerance (seeds 1003,1005,1013 fail: 0.595/2.42 s, 0.555/2.92 s,
0.590/3.17 s).**

| seed | compliance | median latency | sessions | skipped: no arm | skipped: infeasible | within tolerance |
|---|---|---|---|---|---|---|
| 1000 | 0.700 | 2.67 s | 38 | 270 | 3 | yes |
| 1001 | 0.700 | 2.92 s | 38 | 269 | 5 | yes |
| 1002 | 0.625 | 3.17 s | 38 | 266 | 3 | yes |
| 1003 | 0.595 | 2.42 s | 40 | 292 | 1 | **no** |
| 1004 | 0.685 | 3.42 s | 38 | 267 | 4 | yes |
| 1005 | 0.555 | 2.92 s | 40 | 293 | 3 | **no** |
| 1006 | 0.705 | 2.92 s | 39 | 289 | 3 | yes |
| 1007 | 0.670 | 2.92 s | 36 | 255 | 2 | yes |
| 1008 | 0.680 | 3.17 s | 37 | 267 | 6 | yes |
| 1009 | 0.690 | 2.67 s | 39 | 295 | 9 | yes |
| 1010 | 0.625 | 2.92 s | 39 | 277 | 5 | yes |
| 1011 | 0.740 | 2.92 s | 37 | 263 | 11 | yes |
| 1012 | 0.655 | 2.92 s | 38 | 276 | 4 | yes |
| 1013 | 0.590 | 3.17 s | 40 | 286 | 4 | **no** |
| 1014 | 0.700 | 3.17 s | 40 | 284 | 4 | yes |
| 1015 | 0.645 | 2.92 s | 39 | 285 | 3 | yes |
| 1016 | 0.660 | 3.17 s | 39 | 286 | 7 | yes |
| 1017 | 0.645 | 2.67 s | 38 | 264 | 3 | yes |
| 1018 | 0.625 | 3.17 s | 37 | 271 | 5 | yes |
| 1019 | 0.715 | 2.92 s | 39 | 286 | 1 | yes |

The failing seeds stay in this table and in the test output. Compliance is estimated slightly
low on average (development mean about 0.67 against 0.70); three development failures miss
only on latency at 2.42 s against a lower bound of 2.50 s (the median falls on a 0.25-s grid
set by the 4 Hz sampling). That bias is reported, not corrected: choosing constants to remove
it would be fitting. The two tests guard the measured 15/20 and 17/20 against regressions;
they are not the criterion.

### Observer 0.2.0 (as first specified): 0/20 seeds within tolerance

Kept for the record. Compliance 0.275 to 0.380 on every seed against 0.70 +/- 0.08, because
(1) E1 counted cues the persona cannot answer, (2) "reversed" was decided on one bin while
"complied" needed three, and (3) the lag constant was 2000 ms instead of the derived 1875 ms.
The before/after rules and reasons are in [EFFICACY_CHANGELOG.md](EFFICACY_CHANGELOG.md).

| seed | compliance | median latency | sessions |
|---|---|---|---|
| 0 | 0.365 | 3.05 s | 16 |
| 1 | 0.355 | 2.55 s | 16 |
| 2 | 0.340 | 2.55 s | 16 |
| 3 | 0.350 | 3.42 s | 16 |
| 4 | 0.335 | 2.55 s | 16 |
| 5 | 0.350 | 2.67 s | 16 |
| 6 | 0.355 | 2.55 s | 16 |
| 7 | 0.365 | 2.55 s | 16 |
| 8 | 0.380 | 3.92 s | 16 |
| 9 | 0.335 | 2.55 s | 16 |
| 10 | 0.360 | 3.05 s | 17 |
| 11 | 0.365 | 3.05 s | 16 |
| 12 | 0.275 | 3.55 s | 17 |
| 13 | 0.350 | 2.67 s | 16 |
| 14 | 0.365 | 2.80 s | 16 |
| 15 | 0.330 | 3.17 s | 16 |
| 16 | 0.340 | 2.05 s | 16 |
| 17 | 0.325 | 2.30 s | 16 |
| 18 | 0.295 | 3.05 s | 17 |
| 19 | 0.350 | 2.80 s | 16 |

## E2: every confounder is caught through the full Engine

`cargo test -p vc-core --test e2`: a grey-zone cue on a Rolling ride (200 W in tempo, 10 s
of history before the cue); a disturbance is injected after 19 s and removed 5 s later.

| Confounder | How it was injected | Verdict |
|---|---|---|
| quality | power stream dropped for 5 s | confounded |
| coasting | power 5 W for 5 s | confounded |
| cadence_zero | cadence 0 rpm with power reported | confounded |
| grade | grade set to -6 % | confounded |
| audio_interrupted | the cue's audio reported as interrupted | confounded |
| p1_fired | a step-up whose countdown (P1) falls inside the window | confounded |
| *(control)* | nothing injected | ignored (not confounded) |

All six confounders and the control pass. The overlap flag cannot be produced through
the Engine (the Policy Engine allows no second P2/P3 cue inside a window); it is
unit-tested in `vc-efficacy`.

## E3/E4: the tuner's reach

Its type holds the cooldown (15-45 s), the countdown lead (5-8 s), the P3 earcon set and
the default phrasing, nothing else. E3: across random ledgers through the Engine, the
safety settings are untouched and the safety cue fires at the same moment. E4: the
parameters stay in range for any ledger; Study and Replay change nothing.

## E6: byte-identical ledgers

`vc-cli sim --scenario creep --mode study --seed 11 --jsonl` twice, and against
`golden/creep_study_11.jsonl`, in CI on Linux and macOS.

## E8: the statistics with a null rider

200 replicated three-person pilots with Ned: the 95 % bootstrap interval of the pooled
difference covers 0 in **190 / 200 (95.0 %)** (target 92-98 %). `cargo test --release -p
vc-report --test e8 -- --nocapture`.

## The ledger schema: velocoach.irp.v1

One JSON object per line, tagged by `record_type` (`session_start`, `cue`,
`tuner_change`, `session_end`); `crates/vc-report/src/schema.rs`.

| Field (cue record) | Meaning |
|---|---|
| schema, observer_version, session_id, rider_id | identity of the file and the pseudonymous rider |
| mode, tuner_frozen, policy_hash, protocol_hash, profile | Coach/Study, the policy in force (hash), STUDY.md's commit |
| cue_id, directive, priority, aki_class | which cue |
| arm, is_sham, propensity, p_arm_given_delivered | the allocation and its probability |
| seed, draw_index | the micro-randomisation draw (replayable) |
| t_decision_ns, t_cue_onset_ns, t_audio_end_ns, onset_source | timing; `onset_source` is `scheduled` or `loopback` |
| clock_sync_error_bound_us | the Cristian bound at the cue |
| pre_trend_slope_w_per_s, slope_clamped, dir, delta_star_w | the observer's inputs |
| verdict, latency_ms (L_obs), l_onset_ms, l_end_ms, coincidental | the outcome |
| confound_flags, effect_max_w | why "don't know", and the largest hold seen |
| dropped_before | ledger-ring drops before this record |
| t0_ns .. t3_ns | the latency chain of the triggering packet (phone clock; optional) |

One example line (the first cue of the committed golden ledger, seed 11):

```
{"record_type":"cue","schema":"velocoach.irp.v1","observer_version":"vc-efficacy 0.3.0","session_id":"d10166d104b14907","cue_id":1,"rider_id":"R01","mode":"study","tuner_frozen":true,"policy_hash":"ae2873eccb00a23f","protocol_hash":null,"profile":"solo","directive":"GREY_ZONE_DRIFT","priority":"P2","aki_class":"P3","arm":"direct","is_sham":false,"propensity":0.399993896484375,"p_arm_given_delivered":0.5,"seed":11,"draw_index":0,"t_decision_ns":214500000000,"t_cue_onset_ns":214550000000,"t_audio_end_ns":216950000000,"onset_source":"scheduled","clock_sync_error_bound_us":0,"pre_trend_slope_w_per_s":1.3386363636363636,"slope_clamped":false,"dir":-1,"delta_star_w":13,"verdict":"complied","latency_ms":7050,"l_onset_ms":null,"l_end_ms":null,"coincidental":false,"confound_flags":[],"effect_max_w":6.96969696969694,"dropped_before":0,"t0_ns":null,"t1_ns":214500000000,"t2_ns":214500000000,"t3_ns":214550000000}
```

## Reproduce

    cargo test -p vc-efficacy
    cargo test -p vc-core --test e2 --test e3
    cargo test --release -p vc-harness --test e1 -- --nocapture    # prints N/20 and the failing seeds
    cargo test --release -p vc-report --test e8 -- --nocapture

## Limitations

- **E1 does not meet its criterion** (15/20 and 17/20 against ≥ 18/20; above).
- Compliance is not benefit.
- The pre-trend assumes the next 15 s continue the last 10 s.
- A 10 W ask is close to the noise of a 3-s power mean.
- A feasibility pilot with a slider proxy, not cycling; personas are assumptions; R01 is
  the author and not blind to the arms.
