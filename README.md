# velocoach-rs

[![CI](https://github.com/Zakir032002/velocoach/actions/workflows/ci.yml/badge.svg)](https://github.com/Zakir032002/velocoach/actions/workflows/ci.yml)

**Closed-loop voice coach in Rust that measures whether its own cues work — demo runs
hardware-in-the-loop over real BLE between two phones with SIMULATED sensors (no real
rider/sensor data).**

An iPhone 12 runs the coach (CoreBluetooth central). An iQOO Android phone runs a simulated
rider, power meter, heart-rate strap and cadence sensor as a BLE peripheral. Every cue gets a
verdict (complied, ignored, reversed, confounded …), computed from the power stream alone.
The verdicts are scored against the simulator's ground truth.

- **Status, honestly:** the software criteria below are measured in simulation and run in
  CI. The phone latency runs are **pending**. The observer's ground-truth test **E1 does not
  meet its criterion**: 15/20 seeds within tolerance (development) and 17/20 (held-out),
  against ≥ 18/20.
- **Human pilot: planned, pre-registered, not yet run** ([STUDY.md](STUDY.md)).
- **Earbud audio-output latency: not measured.**

[Status](#status) · [3-minute video](docs/VIDEO_SCRIPT.md) (script; link after recording) ·
[Phone checklist](docs/PHONE_CHECKLIST.md) · [EFFICACY.md](docs/EFFICACY.md) ·
[LATENCY.md](docs/LATENCY.md) · [STUDY.md](STUDY.md) · [THIRD_PARTY.md](THIRD_PARTY.md) ·
[design PDFs](docs/design/)

## Status

One row per success criterion from the plan ([docs/design/](docs/design/)). There are three
statuses:

- *measured-on-hardware*: from a phone run;
- *measured-in-simulation*: software, simulated sensors, run in CI unless noted;
- *pending*: not yet measured.

Every row names the test or CI step that produces it.

| # | Criterion | Status | Result | Named test / CI step | Evidence |
|---|---|---|---|---|---|
| S1 | BLE notify → core decision (T1→T2) p99 < 1 ms, iPhone 12 | pending | — (histogram exact to 1/16: `percentiles_are_within_one_sixteenth`) | phone run, `dense` 15 min | [LATENCY.md](docs/LATENCY.md#s1--ble-notify-to-core-decision-t1---t2) |
| S2 | sensor send → audio scheduled (T0→T3) p99 < 150 ms, with the clock-sync bound | pending | — (bound logic: `the_error_never_exceeds_the_bound`, `a_stamp_finds_its_packet_and_completes_the_cue`) | phone run, `dense` 15 min | [LATENCY.md](docs/LATENCY.md#s2--sensor-send-to-audio-scheduled-t0---t3) |
| S3 | to the ear (T0→T4) reported | pending | **not measured** (generic earbuds) | loopback screen (optional) | [LATENCY.md](docs/LATENCY.md#s3--to-the-ear-t0---t4-not-measured) |
| S4 | ≥ 10 min fuzzing per decoder | measured-in-simulation (local run) | see [ENGINEERING_NOTES.md](docs/ENGINEERING_NOTES.md#fuzzing) | `scripts/fuzz-all.sh 600`; CI "Fuzz smoke tests"; `decoders_never_panic` | [ENGINEERING_NOTES.md](docs/ENGINEERING_NOTES.md#fuzzing) |
| S5 | golden logs byte-identical; device capture replays | measured-in-simulation; device replay pending | 5/5 golden rides | CI "Check golden logs"; `a_captured_ride_replays_identically` | [golden/](golden/) |
| S6 | 60-min soak ×2, no unrecovered failure | pending | — | `soak` scenario + `vc-cli soak-check` | [PHONE_CHECKLIST.md](docs/PHONE_CHECKLIST.md#4-latency-and-soak-runs-the-s-rows-of-the-status-table) |
| S7 | phrase lint 100 % | measured-in-simulation | every phrase passes | `lint_every_phrase_passes_its_profile` | `crates/vc-core/src/phrase.rs` |
| S8 | `no_std` core | measured-in-simulation | 5 crates build for `thumbv7em-none-eabihf` | CI "Build no_std crates for Cortex-M4F" | [below](#crates) |
| S9 | policy and safety properties | measured-in-simulation | P1 never muted; Group never speaks P2/P3; safety within one tick | `p1_is_never_muted`, `group_never_speaks_p2_or_p3`, `safety_fires_within_one_tick_after_120_s` | `crates/vc-core/tests/policy_props.rs` |
| S10 | zero allocations per ingest/tick, ledger included | measured-in-simulation | 0 | `ten_thousand_ingests_and_ticks_after_warm_up_allocate_nothing` | `crates/vc-core/tests/alloc.rs` |
| E1 | observer recovers a known rider (Calib 0.70 / 3.0 s) | measured-in-simulation | **criterion not met:** 15/20 seeds within tolerance (seeds 4,5,8,12,17 fail); held-out 17/20 (seeds 1003,1005,1013 fail) | `e1_development_seeds`, `e1_held_out_seeds` (regression guards at the measured level) | [EFFICACY.md](docs/EFFICACY.md#e1-the-observer-recovers-a-known-rider--criterion-not-met), [changelog](docs/EFFICACY_CHANGELOG.md) |
| E2 | every confounder → Confounded, through the Engine | measured-in-simulation | 6/6, control not confounded | `e2_every_confounder_reaches_the_verdict`, `the_control_ride_is_ignored_not_confounded` | [EFFICACY.md](docs/EFFICACY.md#e2-every-confounder-is-caught-through-the-full-engine) |
| E3 | the tuner cannot reach safety | measured-in-simulation | holds | `e3_tuning_never_reaches_safety` | [EFFICACY.md](docs/EFFICACY.md#e3e4-the-tuners-reach) |
| E4 | tuned parameters stay inside Aki's ranges | measured-in-simulation | holds | `e4_tuned_parameters_stay_inside_akis_ranges` | [EFFICACY.md](docs/EFFICACY.md#e3e4-the-tuners-reach) |
| E5 | observer update: 0 allocations | measured-in-simulation | 0 | `ten_cues_over_ten_minutes_allocate_nothing` | `crates/vc-efficacy/tests/alloc.rs` |
| E6 | same seed → byte-identical ledger, two OSes | measured-in-simulation | Linux = macOS = golden | CI "E6 byte-identical ledger"; `same_seed_and_cues_give_identical_rides_and_records` | [golden/creep_study_11.jsonl](golden/creep_study_11.jsonl) |
| E7 | human-mode demo on video | pending | — | scene D6 | [DEMOS.md](docs/DEMOS.md) |
| E8 | null persona: 95 % CI covers 0 in 92–98 % | measured-in-simulation | 190/200 = 95.0 % | `e8_the_null_persona_covers_zero` | [EFFICACY.md](docs/EFFICACY.md#e8-the-statistics-with-a-null-rider) |
| M1 | pre-registered before the first counted session; hash on every record | measured-in-simulation (CI) | STUDY.md at `92aa968`, app hash matches | CI "M1 protocol hash" (`scripts/check-protocol-hash.sh`) | [STUDY.md](STUDY.md) |
| M2 | arm frequencies in bounds; allocation replays from the seed | measured-in-simulation (simulated study run); pilot pending | PASS on 158 simulated decision points | `vc-report --audit`; `p1_is_never_randomised_and_every_p2_has_a_draw` | [docs/sim-study/report.html](docs/sim-study/report.html) |
| M3 | 0 P1 records with an arm | measured-in-simulation; pilot pending | 0 | audit M3; `p1_is_never_randomised_and_every_p2_has_a_draw` | same |
| M4 | one `policy_hash` across Study sessions (tuner frozen) | measured-in-simulation; pilot pending | 1 | audit M4; `study_and_replay_freeze_the_tuner` | same |
| M5 | ≥ 90 % valid; p95 sync bound ≤ 15 ms | measured-in-simulation (bound 0 µs offline); pilot pending | 100 % | audit M5 | same |
| M6 | published figures regenerate from committed ledgers | measured-in-simulation (simulated study run); pilot pending | byte-identical | CI "Simulated study run regenerates", "M6" | [data/sim-study/](data/sim-study/) |
| M7 | mandatory labels present | measured-in-simulation (CI) | present | CI "M7 mandatory labels present" | [STUDY.md](STUDY.md) |
| M8 | written consent before a participant's first session | pending | no participants yet | — | [STUDY.md § Consent](STUDY.md#consent-read-and-signed-before-a-participants-first-session) |

The simulated study run puts the pre-registered protocol through the full pipeline in
software, with personas instead of people. It tests the pipeline. It is not data about people
and not evidence of efficacy.

## How it works

A sans-IO Rust core: it receives bytes and times, and returns at most one decision per call.
The phones own the radio, the threads and the audio.

<a id="crates"></a>

| Crate | Role | `no_std` |
|---|---|---|
| vc-gatt | GATT decoders (HRM, CPM, CSC) + the rig protocol | yes |
| vc-physio | W′bal (exact + GoldenCheetah-parity), quality, cadence | yes |
| vc-efficacy | the observer, the ledger, the bounded tuner | yes |
| vc-core | the Engine: directives, Policy Engine, phrases, Study mode (does not link `alloc`) | yes |
| vc-sim | the simulated rider and its personas | yes |
| vc-ffi | UniFFI bindings for Swift and Kotlin | no (std) |
| vc-harness | the headless closed loop (Engine + simulator) | no (std) |
| vc-report | JSONL ledger, statistics, the HTML report card | no (std) |
| vc-cli | headless rides, golden logs, replay | no (std) |

"`no_std`" means exactly this: these five crates build for `thumbv7em-none-eabihf` in CI
(step "Build no_std crates for Cortex-M4F"). Their own unit tests build with std.

**Directives → Policy Engine → cue.** Eight directives propose cues: W′bal low, soft-pedal,
grey-zone drift, cadence, cardiac drift, safety, sensor dropout and surge. The Policy Engine
then applies the profile's rules:

- priority;
- a cooldown counted from the end of the last clip;
- a P1 cue may interrupt anything;
- nothing else is said during an observation window.

The profiles (Solo / Rolling / Group) and the parameter names respond to Aki Kakko's
published Policy Engine design ([credits](#credits)).

**The observer.** After a cue, it compares the rider's power with where the power was already
heading (a 10-s pre-trend). The cue counts as complied if power moved by at least half the
asked amount and held for 3 s, within 15 s. Anything else that could explain the change makes
the verdict "confounded" rather than a guess: a grade change, a dropout, a safety cue,
coasting. Shams (a cue decided but not played) are scored the same way, which measures the
observer's false-positive rate. The smoothing lag is derived, not fitted:
(N − 1)/2 · Ts + half a bin = 1.875 s
([EFFICACY.md](docs/EFFICACY.md#the-math)).

**The tuner.** In Coach mode, at step boundaries, it may move delivery parameters within
fixed ranges (cooldown 15–45 s, countdown lead 5–8 s). It may also switch to a phrasing that
clearly works better. Its type cannot hold anything safety-critical. In Study mode it is
frozen.

**Study mode.** Seeded PCG32 allocation: sham 0.2 / direct 0.4 / autonomy 0.4. P1 cues are
never randomised, and the tuner is frozen. Every record carries the propensity, the draw
index and the protocol hash.

**Provenance.** Every ledger's `session_start` carries a data label (for example
`simulated persona=calib seed=11 link=offline`). Every report page carries the banner
"SIMULATED DATA — persona …, seeds …, observer_version …".

## Run it

    cargo test --workspace
    cargo run -p vc-cli -- golden --check                       # S5: five golden rides
    cargo test --release -p vc-harness --test e1 -- --nocapture # E1: prints N/20 and the failing seeds
    cargo test --release -p vc-report --test e8 -- --nocapture  # E8
    cargo run -p vc-cli -- sim --scenario climb_surge           # one ride, headless
    scripts/sim-study.sh                                        # the protocol, in software

Phones: [docs/PHONE_CHECKLIST.md](docs/PHONE_CHECKLIST.md). `scripts/check-machine.sh`, then
`scripts/setup-ios.sh` and `scripts/setup-android.sh`: each prints every install command
before running it, is safe to re-run, and stops loudly on failure. No Android Studio is
needed. CI builds the rig APK (artifact `rig-apk`) and compiles the iPhone app on every push.
If the two phones won't link, the iPhone app has an in-process fallback (L1) that runs the
simulator on the iPhone itself. It is labelled "no radio" everywhere.

## Limits

- Simulated sensors only. One iPhone, one Android phone, generic earbuds.
- E1 does not meet its criterion. The observer underestimates compliance slightly (mean
  about 0.67 against 0.70), so compliance figures carry that caveat.
- Compliance is not benefit. A slider is not a pedal.
- Personas are assumptions, not data.
- No human sessions have been run.

## What I'd do next

1. Run the pre-registered pilot, and report it whatever it shows.
2. Real riders on real trainers, with the same ledger schema (nothing in the observer
   assumes a simulator).
3. A phone-side grade source (barometer + GPS), so the grade confounder works outdoors.
4. Per-rider ledgers that persist across rides.
5. FIT file replay through the same Engine.

## Credits

velocoach-rs responds to, and operationalises, ideas from Aki Kakko's posts on the Alphanome
Knowledge Hub. It is not endorsed by Aki Kakko or Domesly. Dates are `article:published_time`
in UTC:

- [The Adaptive Architecture: Solving Audio Fatigue and Context Shift in Real-Time Outdoor Cycling Coaching](https://www.alphanome.ai/post/the-adaptive-architecture-solving-audio-fatigue-and-context-shift-in-real-time-outdoor-cycling-coac) — 2026-10-05: the Policy Engine, profiles, earcon-first alerts, the "> 3 seconds" hold.
- [The Static FIT-File Paradox: Closing the Loop Between Static Prescriptions and Real-Time Adaptive Execution](https://www.alphanome.ai/post/the-static-fit-file-paradox-closing-the-loop-between-static-prescriptions-and-real-time-adaptive-ex) — 2026-10-04
- [Algorithmic Calibration of Stimulus Efficiency in Endurance Systems](https://www.alphanome.ai/post/algorithmic-calibration-of-stimulus-efficiency-in-endurance-systems) — 2026-10-03
- [Bridging The Gap Between Static Workout Plans and Live Sensory Stream](https://www.alphanome.ai/post/bridging-the-gap-between-static-workout-plans-and-live-sensory-stream) — 2026-09-28
- [The Psychology of the Soft-Pedal: Overcoming the Cognitive Pause in Endurance Performance](https://www.alphanome.ai/post/the-psychology-of-the-soft-pedal-overcoming-the-cognitive-pause-in-endurance-performance) — 2026-09-02
- [Next-Generation Cycling Technology: A 2026 Industry Analysis](https://www.alphanome.ai/post/next-generation-cycling-technology-a-2026-industry-analysis) — 2026-08-12
- [Investor Analysis: The Active Coaching Category Paradigm](https://www.alphanome.ai/post/investor-analysis-the-active-coaching-category-paradigm) — 2026-08-06
- [Closed-Loop Cybernetic Intelligence in Cycling: Real-Time Active Coaching via Multi-Sensor Fusion and Edge-AI Intervention Architecture](https://www.alphanome.ai/post/closed-loop-cybernetic-intelligence-in-cycling-real-time-active-coaching-via-multi-sensor-fusion-an) — 2026-08-02
- [The Neuro-Psychology of Algorithmic Guidance](https://www.alphanome.ai/post/the-neuro-psychology-of-algorithmic-guidance-attentional-ergonomics-agency-and-stress-dynamics-in): psychological reactance, the reason for the autonomy-preserving arm.

What this project adds is measuring each cue's effect against ground truth. W′bal follows
Skiba's differential model as implemented in GoldenCheetah (independent port, cross-checked:
[WPRIME_GC.md](docs/WPRIME_GC.md)). The voice is Piper `en_US-ljspeech-medium`, trained on
the public-domain LJ Speech dataset ([assets/cues/VOICE_LICENSE.md](assets/cues/VOICE_LICENSE.md)).

Author: Zakir Shaik. Open-source contributor to RisingWave, Apache Kvrocks and Apache Fury
(since 2024).

## License

MIT OR Apache-2.0, at your option: [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE).

## Appendix: the VC-SIM rig protocol

Every message starts with a 1-byte version (`0x01`) and a 1-byte opcode. This table is
checked against the encoders by `the_readme_protocol_table_matches_the_encoders`
(`crates/vc-gatt/tests/readme_protocol.rs`): the code is the source of truth.

### Telemetry (VCSIM_TELEM, iQOO → iPhone)

| Message | Opcode | Size | Description |
| :--- | :--- | :--- | :--- |
| `STAMP` | `0x01` | 15 B | T0 of the sensor notification just sent (iQOO clock) |
| `PONG` | `0x02` | 20 B | clock-sync reply: PING received / PONG sent |
| `STATUS` | `0x03` | 12 B | simulator state and active fault bitmask |
| `RIDER_INPUT` | `0x04` | 14 B | human slider effort, timestamped on the iQOO clock |

### Control (VCSIM_CTRL, iPhone → iQOO)

| Message | Opcode | Size | Description |
| :--- | :--- | :--- | :--- |
| `PING` | `0x10` | 12 B | clock-sync request |
| `LOAD` | `0x20` | 9 B | scenario, persona (0–4), seed, human mode |
| `START` | `0x21` | 2 B | start the ride |
| `STOP` | `0x22` | 2 B | stop the ride |
| `FAULT` | `0x30` | 10 B | inject a fault (dropout, garbage, disconnect, coast, HR redline, W′ mismatch) |
| `CLEAR` | `0x31` | 3 B | clear faults (`0xFF` clears all) |
| `RATE` | `0x40` | 4 B | sensor rate |
| `CUE_PLAYED` | `0x50` | 15 B | a cue's ask, arm and audio timing |
