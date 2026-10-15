# velocoach-rs: the whole project, how it links, and what is left

One document that explains every part of the repository, how the parts connect, and a
frank list of what must still be done before the project can be called finished. Companion
documents: [PLAN_VS_REPO.md](PLAN_VS_REPO.md) (plan against repo, criterion by criterion),
[COMPLETION_GUIDE.md](COMPLETION_GUIDE.md) (the step-by-step finish),
[RUNBOOK.md](RUNBOOK.md) (running study sessions).

Contents: 1 What it is · 2 The system on one page · 3 The Rust core, crate by crate · 4 The
wire protocol · 5 The life of a packet · 6 The life of a cue · 7 The two phone apps · 8
Determinism, capture, replay, golden logs · 9 The study (how the pilot is wired into the
code) · 10 Tools, scripts and CI · 11 Document map · 12 **Gaps: what is left before "done"**

---

## 1. What it is

velocoach-rs is a **closed-loop voice coach for cycling written in Rust**, built to answer one
question that most coaching apps never ask of themselves: *did the cue actually work?*

- A coach (the **iPhone**) listens to heart rate, power and cadence over Bluetooth, decides
  when to speak ("ease off", "go now", "check your power meter"), and plays the cue in the
  rider's earbud.
- Because there are no real sensors in this project, a second phone (the **iQOO Android
  phone**) is a Bluetooth sensor rig that *simulates a rider*: it advertises a real heart-rate
  strap, power meter and cadence sensor and drives them from a virtual rider model.
- After every cue, an **observer** looks only at the power stream that follows and gives a
  verdict: complied, partial, ignored, reversed, confounded (something else could explain
  it) or invalid. Verdicts go to a **ledger**, from which statistics, a report and a bounded
  **tuner** are derived.
- A small **pre-registered pilot** uses a human on a slider (instead of a bike) and
  micro-randomisation (some cues are shams that play nothing) to measure whether cues and
  phrasing change behaviour, and to measure the observer's own false-positive rate.

The central engineering idea is **sans-IO**: all decision logic is a synchronous Rust library
that is handed bytes and timestamps and returns at most one decision per call. It does no
I/O, reads no clock, allocates nothing per packet. The phones own the radio, the threads and
the audio. That is why the same code runs on a Mac, an iPhone and an Android phone, and why
any ride can be replayed byte for byte.

## 2. The system on one page

```
                    (real Bluetooth LE, GATT)
  iQOO (Android)  ───────────────────────────────►  iPhone 12
  "the rig"             HRM  2A37   notify            "the coach"
  Kotlin app            CPM  2A63   notify            Swift app
  ┌─────────────┐       CSC  2A5B   notify            ┌──────────────────────────┐
  │ GattRig     │       TELEM 6C9F0002 notify         │ BLECentral  (ble.q)      │
  │ NotifyQueue │ ◄───  CTRL  6C9F0003 write          │ T1 stamped on 1st line   │
  │ SimDriver   │                                     │ CoachModel  (ble.q)      │
  │  ▼          │   rider model + scenarios           │  ▼ bytes+time            │
  │ vc-ffi(sim) │   (Rust, same crates)               │ vc-ffi ► vc-core Engine  │
  │  Simulator  │                                     │  ▼ CueDecision           │
  └─────────────┘                                     │ AudioCuePlayer           │
   slider (human mode)  ──RIDER_INPUT on TELEM──►     │  ▼ earbud                │
                                                      │ ledger JSONL + capture   │
                                                      └──────────────────────────┘
                                                                  │ AirDrop / share sheet
                                                                  ▼
  Mac:  vc-cli (sim, replay, golden, soak-check, ...)   vc-report (stats, audit, HTML)
```

Four properties tie it together:

1. **One core.** `vc-core` (plus `vc-gatt`, `vc-physio`, `vc-efficacy`) is the only place
   decisions are made. The iPhone links it through UniFFI; the Mac tools link it directly;
   the rig links `vc-sim` through the same FFI crate.
2. **One clock discipline.** Every time handed to the core is an explicit `u64` of
   nanoseconds. The iPhone uses `mach_continuous_time`; the rig uses `elapsedRealtimeNanos`;
   Cristian-style sync maps one onto the other with a measured error bound.
3. **One ledger.** Every scored cue becomes one JSON line (`velocoach.irp.v1`); everything
   downstream (statistics, HTML, audit, off-policy estimates) reads that file only.
4. **One seed.** The rider's behaviour, the study's allocation of shams and arms, and the
   scenarios are all driven by PCG32 streams from a seed, so a run is reproducible.

## 3. The Rust core, crate by crate

Dependency direction is strictly downward; only `vc-ffi` knows phones exist.

```
vc-gatt   vc-physio   vc-efficacy      (no workspace dependencies, all no_std)
      \       |       /
          vc-core                       (no_std, no alloc; the Engine)
             |
          vc-sim                        (no_std; the virtual rider and its packets)
             |
  vc-ffi (UniFFI; "sim" feature adds the Simulator for the rig)     vc-harness (std; Engine+sim loop)
                                                       vc-report (std)   vc-cli (std)
```

### 3.1 `vc-gatt`: bytes in, typed values out

Decoders and encoders for the Bluetooth SIG characteristics the coach consumes, plus the
project's private rig protocol.

- `hrm.rs` (Heart Rate Measurement 0x2A37, 8/16-bit values, RR intervals), `cpm.rs` (Cycling
  Power Measurement 0x2A63, all 13 flag bits, crank data), `csc.rs` (Speed/Cadence 0x2A5B),
  `feature.rs`, `location.rs`, `cursor.rs` (a bounds-checked reader), `error.rs`.
- `vcsim.rs`: the TELEM and CTRL messages of section 4. Strict by design: exact length,
  versioned, so anything accepted re-encodes to identical bytes.
- Proven by: property round-trip tests, vector tests, and six fuzz targets (`hrm`, `cpm`,
  `csc`, `telem`, `ctrl`, `vcsim`). Decoders never panic on any input; malformed bytes become
  counters in the Engine, not exceptions.

### 3.2 `vc-physio`: what the body is doing

- `wbal.rs`: W′ balance (the energy above critical power the rider has left). Three models:
  `WbalExact` (the exact solution of the differential equation), `WbalGc` (GoldenCheetah's
  one-second Euler step, verified against `WPrime.cpp`, see `WPRIME_GC.md`) and
  `WbalWaterworth`. Gaps longer than 10 s are zero-filled.
- `quality.rs`: a per-channel state machine (Fresh → Stale → Lost → Recovering → Fresh) that
  everything else consults. The observer refuses to score a window with non-Fresh power.
- `cadence.rs`: derives rpm from crank counters (handles wraparound and a power meter without
  crank data).

### 3.3 `vc-efficacy`: did the cue work?

- `p3.rs`: a 40-bin ring of one-second power bins; P3(k) is the mean of three consecutive bins.
- `observer.rs`: for one cue, anchor on the last full second before the audio's onset, fit a
  10-second least-squares pre-trend (clamped to ±2 W/s), project it forward as the
  counterfactual, and score `E(k) = direction × (P3(k) − projection)`. Hold `H(k)` is the
  minimum of E over three bins. Complied at `H ≥ 0.5Δ`; reversed at `H ≤ −0.2Δ`; otherwise
  partial (`max H ≥ 0.2Δ`) or ignored at the end of the window (phrase end + 15 s). Confounders
  (quality, grade change > 3 %, coasting < 20 W, cadence zero, a P1 cue inside the window,
  overlapping audio, interrupted audio) turn a verdict into *confounded*. `L_obs` includes
  about two seconds of 3-s smoothing lag. Human mode adds `L_onset`/`L_end` from the slider.
- `ledger.rs`: per (directive, arm, profile) Beta-style counts and a 16-entry latency ring.
- `tuner.rs`: the bounded tuner (rules T1–T4). Its type holds exactly four things: the
  cooldown (15–45 s), the surge countdown lead (5–8 s), which P3 cues get an earcon, and which
  P2 directives default to autonomy phrasing. Safety is not in its type, so it cannot be reached
  by construction (E3). Frozen in Study and Replay modes (E4).
- No clock, no randomness, no allocation: proven by counting-allocator tests (E5, S10).

### 3.4 `vc-core`: the Engine

`Engine::new(cfg, plan)` then, on the shell's thread: `start(t)`, `ingest(t, source, bytes)`,
`tick(t)`, `on_cue_played/failed`, `on_cue_scheduled`, `snapshot()`, `ledger_pop()`, and the
latency/clock/capture calls. Each `ingest`/`tick` returns at most one `CueDecision`.

What happens inside `ingest`:

1. `engine.rs` dispatches by source: HR/Power/CSC go to the `vc-gatt` decoders; Telem is
   decoded as STAMP / PONG / RIDER_INPUT.
2. `vc-physio` updates W′, quality and cadence; `smooth.rs`, `hysteresis.rs` give the
   directives their smoothed inputs and re-arm behaviour.
3. `directive.rs` evaluates eight directives: `WPRIME_LOW`, `SOFT_PEDAL_WARNING`,
   `GREY_ZONE_DRIFT`, `CADENCE_LOW` (P2, requests to change power or cadence),
   `CARDIAC_DRIFT` (P3), `SAFETY_HR`, `SENSOR_DROPOUT`, `SURGE_PENDING` (P1, never randomised).
   `safety.rs` (`RedlineGate`) is separate code that no tuner can touch.
4. `workout.rs` follows the planned steps (warm-up, interval, recovery…), pauses automatically
   when the rider has stopped, and emits the surge countdown and "go now".
5. `policy.rs` (the **Policy Engine**) chooses among candidate cues using the profile
   (`config.rs`: Solo ±8 % / 5 s / 8 s, Rolling ±15 % / 10 s / 15 s, Group ±20 % / 15 s / 20 s with
   P2/P3 muted): priority, cooldown from the **end of the last clip**, P1 may interrupt a P2/P3
   phrase, a P1 is never muted, and no second P2/P3 cue is allowed inside an observation window.
   In Study mode it asks `study.rs` (`StudyAllocator`, PCG32 per seed and draw index) to allocate
   **sham 0.2 / direct 0.4 / autonomy 0.4** to each eligible P2 decision point, logging the
   propensity; a sham plays nothing but still starts the cooldown.
6. `phrase.rs` + `lint.rs`: the phrase bank (direct and autonomy wordings, earcon first) and the
   lint rules every phrase must satisfy (verb first, at most one number, syllable bounds).
7. `efficacy.rs` (the **Hub**) glues the observer to the Engine: pending cues wait for their
   audio onset/end, the window opens at audio end, late confounders are replayed, the finished
   verdict goes to the ledger, step boundaries call the tuner, and slider moves become
   `rider_move` times.
8. `ledger_ring.rs`: a fixed 256-record ring the shell drains at about 1 Hz (so serialisation
   stays out of the hot path); `capture.rs` records every input (types 1–12) for replay;
   `latency.rs` keeps four log-linear histograms; `clock.rs` + `chain.rs` do clock sync and
   per-cue T0–T3; `golden.rs` renders the golden-log format.

### 3.5 `vc-sim`: the virtual rider

`rider.rs` steps at 4 Hz: power follows the planned target plus noise, a persona reacts to cues
(`persona.rs`: Dana 90 %/1.8 s, Sam 30 % direct / 80 % autonomy / 2.5 s, Ned 50 %/2.5 s as the
null rider, Calib 70 %/3.0 s as the known rider), heart rate follows power with lag, and faults
(`faults.rs`: dropout, garbage, coast, HR redline, disconnect, W′ mismatch) can be scheduled or
injected. `packets.rs` encodes HRM/CPM/CSC notifications; `sim.rs` is the `Simulator` driven by
CTRL messages; `scenario.rs` holds eleven scenarios: `climb_surge`, `recovery_drift`, `safety_hr`,
`dropout`, `grade_confound`, `creep`, `dense`, `soak`, `creep_long`, `grade_demo`, `creep_pilot`
(the pilot's scenario: 18 cycles of 40 s creep and 20 s easy, about 17 decision points per session). Human mode
replaces the persona with a slider effort (fraction of CP).

### 3.6 `vc-ffi`: the only place phones touch Rust

UniFFI 0.32.2 (proc macros, library mode) exposes the Engine (`Engine`, `CueDecision`,
`Snapshot`, `CueOutcomeFfi`, `ClockStatus`, `LinkStats`, `LatencySummary`, `ReportCard`, encoders
such as `encode_load/start/stop/ping/rate/fault/cue_played/rider_input/stamp/pong`) to Swift and
Kotlin. The `sim` feature adds `Simulator` and the rig-side encoders for the Android build only.
One `Mutex` per object (never contended: each phone drives it from one serial queue). A
`uniffi.toml` fixes the Kotlin package (`dev.velocoach.ffi`).

### 3.7 `vc-harness`, `vc-report`, `vc-cli`

- `vc-harness`: runs the real Engine against the real Simulator on a virtual clock (no radio, no
  sleeping): a 20-minute ride in milliseconds. It feeds the Mac the true road grade, applies
  scripted CTRL messages (`ctrl_at`), and returns decisions, outcomes and the ledger. E1 and the
  demos use it.
- `vc-report`: the schema (`velocoach.irp.v1`: session_start, cue, tuner_change, session_end),
  Wilson intervals, a seeded person-stratified bootstrap (B = 10,000, seed 20261013), medians/IQR,
  the HTML report card (including the per-person panel and a "what if" panel), the audit M2–M5,
  and IPS/SNIPS with a conservative adopt/keep gate. Binary flags: `-o`, `--audit`,
  `--audit-only`, `--exclude-rider`.
- `vc-cli`: `sim`, `golden [--check]`, `replay`, `soak-check`, `power-trace`, `wbal`,
  `find-interrupt`, `phrases`, `hex`, and the CTRL helpers.

## 4. The wire protocol

Standard services the coach subscribes to (the rig advertises them as a real device would):
Heart Rate 180D/2A37, Cycling Power 1818/2A63 (+ Feature 2A65), Cycling Speed & Cadence
1816/2A5B. Plus the private **VC-SIM** service `6C9F0001-5E1A-4C2B-9D3E-7A1B2C3D4E5F` with two
characteristics: TELEM `…0002` (rig → coach, notify) and CTRL `…0003` (coach → rig, write).

| Direction | Message | Purpose |
|---|---|---|
| rig → coach (TELEM) | `STAMP` | sent right after every sensor notification: T0 on the rig clock, the channel, and the per-channel packet counter |
| | `PONG` | clock-sync reply carrying b (receive) and c (send) times |
| | `STATUS` | simulator state and fault bitmask |
| | `RIDER_INPUT` | human slider effort, stamped on the rig clock |
| coach → rig (CTRL) | `PING` | clock-sync request with the coach's time a |
| | `LOAD`, `START`, `STOP` | choose scenario, persona, seed, human flag; run |
| | `FAULT`, `CLEAR`, `RATE` | inject faults; change a sensor's rate (soak) |
| | `CUE_PLAYED` | tells the virtual rider what a cue asked and when its audio plays (so personas can react) |

## 5. The life of a packet (latency chain)

```
T0  rig hands the notification to its Bluetooth stack          (rig clock)
    └─ STAMP follows, carrying T0 and the per-channel counter
T1  first line of the iPhone's didUpdateValueFor                (phone clock)
T2  Engine.ingest returned a decision                           (phone clock)
T3  the cue's audio is scheduled to render (hostTime = now + lead)
T4  the cue reaches the ear = T3 + d_route  (measured, not promised)
```

- `clock.rs` runs PING/PONG at 1 Hz (an 8-ping burst on every connection and at Start) and keeps
  the minimum-RTT of the last 16 samples: offset θ and error bound ±rtt_min/2. Only the rig's
  binder-callback receive time is used for `b`.
- `chain.rs` matches each STAMP to its packet by `(channel, counter)`, maps T0 onto the phone's
  clock, and attaches T0–T3 to the cue that packet triggered. A T0 later than T1 is counted as
  "the method's assumption failed" and shown on screen.
- Four histograms (T0→T1, T1→T2, T1→T3, T0→T3) feed the panel and the soak log; the cue's
  chain also goes into the ledger (`t0_ns`…`t3_ns`), with the sync bound beside it.
- Tick-released cues (countdowns, grace windows) have no triggering packet and so no T0; they are
  reported T1→T3 only.
- d_route is measured with the loopback (Section 7.3) and entered as the "route delay" setting;
  onsets then carry `onset_source: "loopback"` instead of `"scheduled"`.

## 6. The life of a cue

1. **Directive** fires from the smoothed sensor state (e.g. W′ low, grey-zone drift).
2. **Policy Engine** accepts or drops it (cooldown, priority, profile, window). In Study mode a
   P2 candidate is allocated sham/direct/autonomy by PCG32(seed, draw index).
3. **CueDecision** returns to the shell: directive, phrase key, earcon key, lead, interrupt flag,
   direction, Δ*, arm, propensity, draw index.
4. **Audio** (iPhone): a sham plays nothing. Otherwise `AudioCuePlayer` schedules the earcon then
   the phrase at an explicit `AVAudioTime(hostTime:)`; the shell reports `on_cue_scheduled` (T2/T3)
   and sends `CUE_PLAYED` so the virtual rider can react.
5. **Audio end** (`.dataPlayedBack`): `on_cue_played` gives onset and end; the Hub opens the
   observation window at the end (for a sham, a virtual onset/end are set from medians).
6. **Observer** scores the window from the power stream and the slider (human mode); confounders
   are collected during the window.
7. **Verdict → ledger** (`ledger.record`) and → the 256-record ring → JSONL on the phone → file.
8. **Badge/report**: the phone shows a badge for six seconds and a report card at Stop (hidden
   during Study sessions so the rider never sees it).
9. **Tuner** (Coach mode only) may adjust delivery at the next step boundary, writing a
   `tuner_change` record with rule, parameter, from/to, n and evidence.
10. **Offline**: `vc-report` turns ledgers into statistics, the HTML report and the audit.

## 7. The two phone apps

### 7.1 Rig: `apps/android/app/src/main/java/dev/velocoach/rig`

- `RigService` is a `connectedDevice` foreground service (permissions requested first) that owns
  the GATT server and the simulator; `RigService.driver` exposes the driver to the screen.
- `GattRig` adds the four services one at a time (waiting for `onServiceAdded`), advertises after
  the last, accepts one central, and stamps the CTRL write's arrival time in the binder callback.
- `NotifyQueue` allows exactly one notification in flight (with a 500 ms watchdog), takes T0
  immediately before each sensor notification and queues the STAMP after it; its counters reset on
  every new connection and on every LOAD (a new ride makes a new Engine on the iPhone).
- `SimDriver` owns the `Simulator` on a `HandlerThread` ("sim", 50 ms steps); applies CTRL;
  replies PONG; forwards slider moves as `RIDER_INPUT`; the "I am the rider" switch wins over
  LOAD's human byte.
- `MainActivity`: status counters and a scenario/persona picker; in human mode it hides **all** cue
  information and shows only a large slider (sent at most every 50 ms plus the final position).
  `RigStatus` holds counters. Keep-screen-on is set; Vivo battery/autostart/lock settings matter
  (RUNBOOK and Day 9 notes).

### 7.2 Coach: `apps/ios/VeloCoach`

- `BLECentral`: one serial queue `ble.q`; `Clock.nowNs()` as the **first line** of the value callback
  (T1); subscribes to the four notifying characteristics; auto-reconnect with no timeout;
  write-without-response held until the peripheral is ready; calls `centralDidConnect` before the
  first notification of a new link.
- `CoachModel`: the Engine lives on `ble.q`; a strict 250 ms tick (also driven by packets if the timer
  stalls); PING burst/1 Hz; T2 after each ingest; `on_cue_scheduled`; badge/outcome polling; ledger
  and capture drained to an `io.q` file writer; Study setup (`setMode`, `setRider`, seed = 1000 ×
  rider + session, `protocolHash`); route delay → `setRouteDelayKnown`; soak CSV (one row per 10 s
  and per event) and scripted RATE messages; HR-redline fault field for demo D3.
- `AudioCuePlayer`: session `.playback`/`.voicePrompt`/`.duckOthers`, kept active; the phrase bank
  (rendered offline with Piper, earcons from `tools/make_earcons.py`) as `AVAudioPCMBuffer`s;
  explicit host-time scheduling; every clip has a generation so interrupted or reset clips are
  reported once as failed; recovers from engine, interruption, route and media-services resets;
  `callbackSkewMs` cross-checks computed end against `.dataPlayedBack`.
- `RideView`: live numbers, latency panel, Study controls (with the policy hash visible before
  Start), human-rider toggle, route-delay field, badge/outcomes (hidden during a Study session),
  report card sheet, loopback sheet, share-sheet export of capture, ledger and soak log.
- `LoopbackView`: schedules a 5 ms click and a one-frame white flash at the same host time so
  the iQOO camera can measure d_route. `StudyConfig`: rider ids, the seed formula, and the
  protocol hash.

### 7.3 Native tools that serve the apps

`scripts/build-ios.sh` (XCFramework + Swift bindings), `scripts/build-android.sh` (NDK `.so` +
Kotlin bindings, with `--features sim`), `scripts/render-phrases.sh` (Piper → WAV → CAF),
`tools/loopback.py` (video → d_route), `tools/gc_wbal.py` (W′bal cross-check),
`scripts/check-protocol-hash.sh` (the app's hash must equal STUDY.md's last commit).

## 8. Determinism, capture, replay, golden logs

- **No hidden inputs.** The core reads no clock and no global state; PCG32 is the only PRNG; `libm`
  provides every transcendental; no hash-map iteration affects output.
- **Capture** (`.vcraw`, written by the phone every ride): a header (config + plan) then every
  input as a typed record (sensor bytes with times, ping/pong, resets, scheduled cues, audio events,
  mode/seed, route-delay flag…). `vc-cli replay file.vcraw` feeds them back into a fresh Engine and
  prints `identical` iff every decision matches. A difference means a hidden input in the Engine.
- **Golden logs** (`golden/*.log`): five scenario rides rendered as text; `vc-cli golden --check`
  compares byte for byte in CI. `golden/creep_study_11.jsonl` is the same for the ledger (E6, on
  Linux and macOS).
- **Fuzzing** protects the decoders; **property tests** protect the policy (S9), the tuner (E3/E4),
  the Study freeze and "P1 never carries an arm" (M3).

## 9. The study: how the pilot is wired into the code

| Pilot element (STUDY.md) | Where it lives |
|---|---|
| Within-person micro-randomisation: sham 0.2 / direct 0.4 / autonomy 0.4 | `study.rs` `StudyAllocator`, PCG32(session seed, draw index); `Hub` stores `draw_index` |
| Propensity logged; allocation replayable | ledger `propensity`, `seed`, `draw_index`; `--audit` M2 re-draws and compares |
| P1 never randomised | `arm = none` for P1; M3 audit + property test |
| Tuner frozen; one policy | `Mode::Study`; `policy_hash` in every record; M4 audit |
| Protocol committed first | `STUDY.md`; `StudyConfig.protocolHash`; `check-protocol-hash.sh`; M1 |
| Human on a slider; rig shows nothing | rig human mode; `RIDER_INPUT`; `rider_move`; `L_onset`, `L_end`, coincidental < 0.35 s |
| Seeds: 1000 × rider + session | `StudyConfig.seed`; the seed is shown on screen and logged |
| Dry run excluded | rider `R00`; `vc-report --exclude-rider R00` |
| Analysis pre-specified | Wilson, person-stratified bootstrap, no p-values: `vc-report` |
| Data quality | `--audit` M2–M5; M6 CI regeneration; M7 label grep; M8 consent (paper) |

## 10. Tools, scripts and CI

`ci.yml` jobs: **test** (ubuntu + macOS: fmt, clippy `-D warnings`, workspace tests, golden check,
E6 ledger compare, M6 regeneration when `data/study/` exists, M7 label grep, `thumbv7em` build of the
five `no_std` crates), **release-tests** (E8 gating; E1 non-gating while open), **fuzz-smoke**
(60 s per target), **ios** (XCFramework + bindings), **android** (NDK library + Kotlin bindings).

## 11. Document map

| File | What it is |
|---|---|
| `README.md` | the one-screen pitch, numbers, reproduce commands |
| `STUDY.md` | the pre-registration, deviations, Results template |
| `docs/PROJECT_GUIDE.md` | this document |
| `docs/PLAN_VS_REPO.md` / `COMPLETION_GUIDE.md` | comparison and the finishing steps |
| `docs/EFFICACY.md` | observer maths, confounders, personas, E1/E2/E8, schema |
| `docs/LATENCY.md` | definitions, clock sync, S1/S2/S3 tables (to be filled) |
| `docs/ENGINEERING_NOTES.md` | architecture decisions and their costs |
| `docs/RUNBOOK.md` | session procedure, consent, briefing, backups |
| `docs/DEMOS.md` | demo scenes, shot list, voiceover |
| `docs/WPRIME_GC.md`, `FACT_CHECK.md`, `THIRD_PARTY.md`, `RELEASE_NOTES.md`, `DEVLOG.md` | cross-check, sources, licences, release text, daily log |

---

## 12. Gaps: what is left before "done"

"Done" has three levels. Level A is what the repository can prove without hardware; B is what
the plan's success criteria need from the phones; C is the study and the shipping steps. Status
as of the last commit on `claude/velocoach-days-9-14`.

### A. Engineering gaps (code and repo)

| # | Gap | Why it matters | To close it |
|---|---|---|---|
| A1 | **E1 fails (0/20 seeds)** | never-cut criterion; "the observer measures efficacy" is unproven until a known rider is recovered | apply the three changes in COMPLETION_GUIDE phase 1 (18/20 seeds in experiment), decide on the last two seeds, bump `OBSERVER_VERSION`, regenerate the golden ledger, make the CI step gating again |
| A2 | **Apps never compiled** | Swift/Kotlin were written to the documents and name-checked against generated bindings only | build both; fix compile errors; run the Phase 0 smoke test |
| A3 | **No buildable app projects in the repo** | there is no `.xcodeproj` for VeloCoach and no Gradle files for the rig; the repo alone cannot produce either app, and CI cannot compile them | commit the Xcode project (or an XcodeGen spec) and the Gradle project (settings, app module, JNA dependency `jna:5.14.0@aar`); add `xcodebuild` and `gradle assembleDebug` jobs |
| A4 | **No automated tests for app code** | the rig queue, STAMP counters, ping logic, badge text and Study guards are only exercised by hand | add Kotlin unit tests (NotifyQueue, counter reset) and Swift tests (badgeText, StudyConfig.seed); optional |
| A5 | **Build-time artefacts not in the repo** | the phrase bank `Cues/` (CAF + manifest) is generated by Piper and not committed; a fresh checkout has no audio | run `scripts/render-phrases.sh` and either commit the (licence-checked) output or document the exact command in the README |
| A6 | **Hard-coded rider and profile on the phone** | `CoachModel.config()` fixes CP 250 W, W′ 20 kJ, HR max 190, profile Solo; no UI for Rolling/Group | acceptable for the pilot (Study requires Solo); for real riders add settings |
| A7 | **Tuner activity is invisible on the phone** | only ledger and report show `tuner_change` | optional UI line; demo D2 uses the Mac |
| A8 | **Observer behaviour in creeping rides needs a look** | in `creep_long`, many cardiac-drift cues come out `reversed` (the creep pushes power up while the cue asks for less); this is partly the persona never reacting, but confirm the pre-trend handles creep before trusting `reversed` rates | check after A1 with a P2-only view of the ledger |
| A9 | **Plan items never built (cut list)** | FIT replay, Intervals.icu fetch, post-ride feedback buttons, decoupling cue, Mac BLE central `vc-cli ble` (Lite L2) | all are cuttable by the plan; listed in "what I'd do next". Build `vc-cli ble` only if the iPhone path fails |
| A10 | **E5 timing and S4 smoke counts not recorded** | the README table has blanks | `cargo bench -p vc-efficacy`, `scripts/fuzz-all.sh 20`, paste numbers |
| A11 | **Dependency licence audit** | THIRD_PARTY claims MIT/Apache compatibility | run `cargo deny check licenses` or `cargo license` |
| A12 | **Pre-registration inputs** | `StudyConfig.protocolHash` = `PENDING`; `STUDY.md` date and two deviation dates blank | COMPLETION_GUIDE phase 2 |

### B. Measurements the success criteria need (phones and clock)

| Criterion | Missing | How |
|---|---|---|
| G-11 / v0.1.0 | 20-minute two-phone run, replay `identical`, Vivo 30-min screen-on | phase 3 |
| S1 | T1→T2 p99 < 1 ms on the iPhone 12 (15 min, capture off, then on) | panel / DEVLOG / LATENCY |
| S2 | T0→T3 p99 < 150 ms with its sync bound (`dense`, 15 min, earbuds) | phase 5 |
| S3 | d_route by loopback (20 + 20 clicks), `outputLatency` per route, T4 estimated vs measured | phase 5 |
| S6 | two 60-minute soaks (Day 17 and the shipping build), `soak-check` PASS, replay identical | phases 7 and 9 |
| S4 | the Day 20 fuzz smoke on the tagged build | `scripts/fuzz-all.sh 20` |
| v0.2.0 | headless gate G-14 tag (needs A1) | phase 4 |

### C. Study and shipping gaps

| Item | State |
|---|---|
| Dry run R00 (20 min, human mode, audit M3–M5) | not done; also validates human mode (K3 fallback L1) |
| Counted sessions R01 ×3, R02 ×3, R03 ×3 (or fewer, per K2/K4) with consent (M8), seeds 1001–3003 | not done |
| Audit M2–M5 passing, Results written, per-person table, deviations, limitations | not done |
| M6: `data/study/*.jsonl` and `docs/study/results.html` committed | not done (CI step waits for them) |
| Hub/site re-check and fact-check ledger ("to verify at source" rows, GoldenCheetah commit hash, Piper voice licence) | not done |
| Every `__` in README, LATENCY, ENGINEERING_NOTES, DEMOS, RELEASE_NOTES | open: ~150 cells, mapped to sources in COMPLETION_GUIDE Appendix A |
| Demo rehearsal (D1–D7, twice each) and recording; video ≤ 3:00 with subtitles, unlisted upload | not done (D2, D3, D4, D5 headless numbers already in DEMOS.md) |
| Hygiene check, tag v1.0.0, release, public repo, message to Aki | not done |

### D. Definition of done (tick when true)

- [ ] E1 passes (or the decision to ship without it is written in STUDY.md Deviations and in every place that mentions the observer's validity)
- [ ] Both apps build from the repository on a clean machine; CI compiles them
- [ ] `cargo fmt`, `clippy -D warnings`, `cargo test --workspace`, golden check, E6, E8 and (release) E1 all green in CI on both operating systems
- [ ] G-11 passed and v0.1.0 tagged; G-14 passed and v0.2.0 tagged
- [ ] S1, S2, S3 measured and written in LATENCY.md; S6 passed twice
- [ ] `STUDY.md` committed, hash in the app, `check-protocol-hash.sh` prints `ok`
- [ ] Dry run clean; counted sessions complete (or the K-rule outcome recorded); consent for every participant; audit M2–M5 PASS
- [ ] Results written with the mandatory language; M6 and M7 CI green; ledgers of consenting participants only in `data/study/`
- [ ] No `__` left outside intentional templates; fact-check ledger has no "to verify" rows; THIRD_PARTY has a verified voice licence
- [ ] Video ≤ 3:00 uploaded unlisted and linked; hygiene check clean
- [ ] v1.0.0 tagged and released; README links work from a private window; message sent

### E. Suggested order of attack

1. **A1** (E1) and **A2/A3** (make the apps build): these decide everything else and need no people.
2. **A12** (pre-registration) immediately after A1, because the observer is frozen from then on.
3. Gate G-11 and S1, then S2/S3, then the dry run (phases 3, 5, 6).
4. Soak 1, then the nine counted sessions, analysis and M6 (phase 7).
5. Documents, rehearsal, recording (phase 8), then the shipping day (phase 9).

If time runs short, cut in the plan's order (IPS, FIT, Waterworth, Group, Intervals.icu, feedback
buttons, benches, decoupling, friends' sessions…), but never the items on its never-cut list:
safety gate, S9, E3, golden logs, latency with its bound, fuzzing, **E1**, E2, the Study freeze,
the ledger schema, consent and pre-registration.
