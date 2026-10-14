# Efficacy: how velocoach-rs scores its own cues

Observer version `vc-efficacy 0.2.0`. Every number below is reproducible with the
commands at the end. Back to the [README](../README.md).

> **Status of the instrument's own validation (read this first).** E2, E3, E4, E6 and
> E8 pass. **E1 does not pass yet**: with the observer exactly as specified, none of
> the 20 seeds recovers Calib's hidden compliance within tolerance (the table below).
> The cause is understood (see "E1 is open"), nothing here claims otherwise, and no
> efficacy conclusion should be drawn from the observer's compliance rates until E1 is
> resolved.

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
  - reversed as soon as some H(k) <= -0.2 x Δ (`REVERSE_FRAC`), whichever of the two
    comes first;
  - at the end of the window: partial if the largest H seen reached 0.2 x Δ
    (`PARTIAL_FRAC`), otherwise ignored.
- L_obs = the end of the second in which the hold first reached the complied threshold,
  minus the end of the phrase (it can be negative). It includes a smoothing lag of about
  2.0 s (the 3-s mean), documented as `SMOOTHING_LAG_MS = 2000`.
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

## E1: the observer recovers a known rider — **OPEN**

The test (`cargo test --release -p vc-harness --test e1`): 20 seeds x 200 scored cues on
the `creep` scenario with Calib; pass when each seed's compliance is within 0.70 +/- 0.08
and its median latency (L_obs minus the smoothing lag) within 3.0 +/- 0.5 s.

Output with the observer as specified (every seed is outside the compliance band):

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

Compliance 0.275 to 0.380 against a target of 0.70 +/- 0.08 (0 of 20 seeds in band).
Latency 2.05 to 3.92 s (within 3.0 +/- 0.5 s for 16 of 20 seeds).

**Why it is open.** Diagnosed by instrumenting runs. The experiments below were run on
scratch copies and reverted; none of them is in the committed code.

1. **What E1 counts.** The count includes the P3 cardiac-drift cue, which carries no
   phrasing arm, so the persona never reacts to it (`Reaction::NotApplicable`): it is
   scored `ignored` or `reversed` by construction. Counting only cues with a modelled
   arm (Direct or Autonomy) raised the mean compliance from 0.34 to 0.55.
2. **Reversal is decided on one bin, compliance on three.** H(k) is the *minimum* of E
   over three bins. "Complied" needs H >= 0.5 x Δ, so all three bins must be high; but
   "reversed" needs only H <= -0.2 x Δ, so one bin is enough. With the 10 W floor that is
   -2 W, against about 1.5 W of power noise in a 3-s mean, and 2,294 of 4,000 scored cues
   came out `reversed`. Requiring all three bins to be <= -0.2 x Δ (the sustained
   mirror-image of the compliance rule) with change 1 gave per-seed compliance of 0.61 to
   0.76 (mean about 0.67).
3. **The lag constant.** With changes 1 and 2 the median latency (L_obs minus
   `SMOOTHING_LAG_MS`) sits at about 2.55 s against 3.0 s: the measured smoothing lag is
   about 1.5 s, not 2.0 s. Setting `SMOOTHING_LAG_MS = 1500` centres it (2.80 to 3.55 s).

With all three, **18 of 20 seeds pass**; the other two (seeds 8 and 12) are at 0.610 and
0.615 compliance, just under the 0.62 lower bound. Change 1 is a harness choice (what E1
counts); change 2 alters the observer's specified verdict rule; change 3 alters a documented
constant. Changes 2 and 3 move the instrument, so they need a decision, a version bump
(`OBSERVER_VERSION`) and the regenerated golden ledger before the pre-registration commit.
Options and costs are in `docs/PLAN_VS_REPO.md`. Until then the README and the release notes
list E1 as open, and the pilot's compliance proportions are reported with that caveat.

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
{"record_type":"cue","schema":"velocoach.irp.v1","observer_version":"vc-efficacy 0.2.0","session_id":"d10166d104b14907","cue_id":1,"rider_id":"R01","mode":"study","tuner_frozen":true,"policy_hash":"ae2873eccb00a23f","protocol_hash":null,"profile":"solo","directive":"GREY_ZONE_DRIFT","priority":"P2","aki_class":"P3","arm":"direct","is_sham":false,"propensity":0.399993896484375,"p_arm_given_delivered":0.5,"seed":11,"draw_index":0,"t_decision_ns":214500000000,"t_cue_onset_ns":214550000000,"t_audio_end_ns":216950000000,"onset_source":"scheduled","clock_sync_error_bound_us":0,"pre_trend_slope_w_per_s":1.3386363636363636,"slope_clamped":false,"dir":-1,"delta_star_w":13,"verdict":"complied","latency_ms":7050,"l_onset_ms":null,"l_end_ms":null,"coincidental":false,"confound_flags":[],"effect_max_w":6.96969696969694,"dropped_before":0,"t0_ns":null,"t1_ns":214500000000,"t2_ns":214500000000,"t3_ns":214550000000}
```

## Reproduce

    cargo test -p vc-efficacy
    cargo test -p vc-core --test e2 --test e3
    cargo test --release -p vc-harness --test e1 -- --nocapture    # fails today: E1 is open
    cargo test --release -p vc-report --test e8 -- --nocapture

## Limitations

- **E1 is open** (above).
- Compliance is not benefit.
- The pre-trend assumes the next 15 s continue the last 10 s.
- A 10 W ask is close to the noise of a 3-s power mean.
- A feasibility pilot with a slider proxy, not cycling; personas are assumptions; R01 is
  the author and not blind to the arms.
