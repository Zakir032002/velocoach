# What is built, and how to show each part

**The whole plan is implemented** except its cut list (end of section 2). That is about 17,000 lines of Rust plus
about 2,200 lines of Swift (iPhone) and Kotlin (iQOO), all tested in CI. Back to the
[README](../README.md).

This page lists **every feature**, says where it lives in the code, and says how to make it
appear on screen. Section 3 has one demo scene per feature.

---

## 1. The one idea

The project is a voice coach that does not just talk. After **every** cue it checks whether
the rider actually did what it said, and it gives the cue a verdict:

| Verdict | Meaning |
|---|---|
| ✓ complied | power moved at least half the asked amount and held for 3 s |
| – partial | moved, but not enough |
| – ignored | nothing changed |
| ↺ reversed | the rider did the opposite |
| ? confounded | something else could explain the change (a hill, a dropout, a safety cue, coasting) so it refuses to guess |
| invalid | not enough clean data |

The iPhone shows ✓ for complied, ↺ reversed, ? confounded, and – plus the word for the rest.
No public product the plan found scores every cue like this (plan §1). Everything else in the project
exists to make that verdict trustworthy, fast and measurable.

---

## 2. Every feature, where it is, how to see it

### The engine (Rust core, runs on the phone)

| Feature | Where in the code | How to see it |
|---|---|---|
| Decodes real Bluetooth sensor bytes (heart rate, power, cadence) | `crates/vc-gatt` | iPhone ride screen: the line `quality  HR … · power … · cadence …` |
| W′ balance (anaerobic reserve), exact + GoldenCheetah-parity | `crates/vc-physio` | the W′-low cue in `climb_surge` ("Ease off now, your reserve is at twenty") |
| Signal quality: Fresh → Stale → Lost | `vc-physio` | `dropout` scenario: the quality line changes, then "Check power meter connection." |
| 8 directives with Aki's names: SURGE_PENDING, SOFT_PEDAL_WARNING, GREY_ZONE_DRIFT, WPRIME_LOW, CADENCE_LOW, CARDIAC_DRIFT, SAFETY_HR, SENSOR_DROPOUT | `crates/vc-core/src/directive.rs` | different scenarios trigger different ones (table in section 3) |
| Policy Engine with Aki's Solo / Rolling / Group profiles, priorities, cooldowns, P1 interrupts | `vc-core/src/policy.rs` | `safety_hr` / `recovery_drift`: the safety cue cuts off a phrase. Group muting is shown by the property test `group_never_speaks_p2_or_p3` |
| Earcon first, then speech (Piper voice, public domain) | `assets/cues/`, `AudioCuePlayer.swift` | every cue: a tone, then the phrase |
| Phrase lint (verb first, ≤1 number, 8–12 syllables) | `vc-core/src/lint.rs` | `cargo test -p vc-core lint` |
| Safety gate: HR > 98 % HRmax for > 120 s, untouchable by adaptive code | `vc-core/src/safety.rs` + property tests | `safety_hr` scenario, or the "HR redline at" box in the app |
| Zero memory allocation per packet, `no_std` | the whole core | `cargo test -p vc-core --test alloc`, and the CI step building for a microcontroller |

### The observer (the verdicts)

| Feature | Where | How to see it |
|---|---|---|
| Scores every cue against where power was already heading (pre-trend) | `crates/vc-efficacy/src/observer.rs` | the ✓/–/↺/? badge after each cue on the iPhone |
| Confounders (grade, dropout, coasting, cadence 0, audio interrupted, P1 inside window, overlap) | same | `grade_demo`: the cue at 7:21 says **? grade** |
| Shams: a cue decided but **not** played, scored anyway | `vc-core/src/study.rs` | Study mode; the report card's "sham (false-positive rate)" row |
| Latency per cue, with the derived 1.875 s smoothing lag | `observer.rs` | the badge's seconds number |
| Ground-truth test against a known simulated rider (E1) | `crates/vc-harness/tests/e1.rs` | `cargo test --release -p vc-harness --test e1 -- --nocapture` prints the table: 15/20 and 17/20 |

### The tuner (the coach adapts)

| Feature | Where | How to see it |
|---|---|---|
| Moves Aki's parameters (cooldown 15–45 s, countdown 5–8 s) only inside his ranges | `vc-efficacy/src/tuner.rs` | property test `e4_tuned_parameters_stay_inside_akis_ranges` |
| Switches phrasing (Direct → Autonomy) when one clearly works better, and logs why | same | `creep_long` with persona Sam: a `tuner_change` line at 42:00 (terminal command in section 3) |
| Can never reach safety | type system + `e3_tuning_never_reaches_safety` | the test |

### Modes and switches (what each one is for)

| Switch | What it does | When to use it |
|---|---|---|
| **Coach mode** (default) | the tuner is on; the coach adapts | normal demo rides |
| **Study mode** | the tuner is frozen; each cue is randomised Direct 40 % / Autonomy 40 % / Sham 20 %; P1 never randomised | the pre-registered pilot; also shows shams |
| **Replay** | re-runs a recorded ride byte for byte, from the `.vcraw` capture | `vc-cli replay` (done by `collect-results.sh`) |
| **Persona** picker (dana, sam, ned, calib) | which simulated rider the iQOO plays | Dana = good rider; Sam = ignores orders, follows suggestions; Ned = random (statistics check); Calib = the known rider for E1 |
| **Scenario** picker (11 scenarios) | which workout the rider does | section 3 |
| **Human rider** | a person is the rider: a slider on the iQOO replaces the persona | scene D6 |
| **In-process simulator (L1)** | runs everything on the iPhone with no iQOO and no radio | backup, if Bluetooth won't link |
| **HR redline at __ s** | forces a heart-rate redline at that second | scene D3 (safety) |
| **Route delay (ms)** | earbud delay from the loopback test | leave at 0 (earbud latency is not measured) |

### Measurement and evidence

| Feature | Where | How to see it |
|---|---|---|
| Clock sync between the phones (PING/PONG, error bound) | `vc-core/src/clock.rs` | the iPhone latency panel: `clock θ … ±… µs` |
| Latency chain T0→T1→T2→T3 per cue | `vc-core/src/chain.rs` | the latency panel lines `T1→T2 …`, `T0→T3 …` |
| Ledger: every cue as one JSON line (`velocoach.irp.v1`) | `crates/vc-report/src/schema.rs` | the `ledger-*.jsonl` files from `collect-results.sh` |
| HTML report card: arms, Wilson CIs, bootstrap, SNIPS what-if, timeline, SIMULATED banner | `crates/vc-report` | iPhone **Report card**, or `docs/sim-study/report.html` |
| Statistics checked with a null rider (E8) | `vc-report/tests/e8.rs` | 190/200 = 95 % coverage |
| Fuzzing (6 targets × 10 min, 0 crashes), golden rides, byte-identical ledgers on Linux and macOS | `scripts/fuzz-all.sh`, CI | the CI badge and the README status table |
| Pre-registered study + consent | `STUDY.md` | protocol hash `92aa968` checked in CI |

### What is NOT built (on purpose; it is the plan's cut list)

FIT-file replay, Intervals.icu live fetch, post-ride feedback buttons, the decoupling cue, and
the Mac-as-central fallback (L2). The core-temperature half of the safety rule is not built
either: there is no temperature sensor. The README's "What I'd do next" lists these honestly.

---

## 3. The demo scenes: one feature each

Each scene shows a **different** part of the project. Phone scenes: settings on the iPhone,
then **Start**. Times are ride time (the step counter). "Mac" scenes are one Terminal
command each (`cd ~/velocoach` first). They need no phones and give the same result every
time.

| # | Shows | Where | Setup | The moment |
|---|---|---|---|---|
| **D1** | a cue and its verdict | iPhone | `climb_surge`, persona default | **8:50**: tone + "Ease off now, your reserve is at twenty" → **✓ complied ~3 s** |
| **D2** | the tuner learning | Mac | `cargo run -q -p vc-cli -- sim --scenario creep_long --jsonl /tmp/d2.jsonl > /dev/null && grep tuner_change /tmp/d2.jsonl` | one line: rule T3 switches GREY_ZONE_DRIFT from Direct to Autonomy, n = 16, at 2,520 s |
| **D3** | safety beats everything | iPhone | `recovery_drift`, type **847** in "HR redline at" | **~18:03** (1083 s): the safety cue cuts off the phrase that is playing. Speed up the wait in the edit |
| **D3b** | …and it's proven, not hoped | Mac | `cargo test -q -p vc-core --test policy_props --test e3` | tests pass: P1 never muted; tuning never reaches safety |
| **D4** | it refuses to guess | Mac | `cargo run -q -p vc-cli -- sim --scenario grade_demo \| grep grade` | `GREY_ZONE_DRIFT … -> confounded … ? grade` |
| **D5** | sensor failure handling | iPhone | `dropout` | **10:08**: quality goes Stale → Lost, "Check power meter connection.", then back to Fresh |
| **D6** | a human in the loop | iPhone + iQOO | iQOO: **I am the rider** (slider at 70 %); iPhone: **Human rider** ON, `creep_pilot` | first cue within about 2 minutes: you ease the slider and the badge shows your own reaction time |
| **D7** | the evidence | iPhone / Mac | **Report card** after any ride; or `open docs/sim-study/report.html` | arms, sham false-positive rate, intervals, the red SIMULATED DATA banner |
| **D8** | honesty | browser | the README **Status** table | E1 15/20 not met, latency measured or pending, earbud not measured |

`dense` (16 min) is the latency ride: it gives the S1/S2 numbers, not a showcase scene.

### If you only have time for a few
1. **D1** (iPhone, ~10 min of riding, mostly sped up)
2. **D2 + D3b + D4** (Mac terminal, 2 min in total, no phones)
3. **D7** (the report card after D1)
4. **D8** (README table)
5. If there's time: **D3** on the phone (18 min) or **D6** (you on the slider, 3 min)

Those five already show the coach, the verdicts, the tuner, safety, confounders, the evidence
and the honesty.
