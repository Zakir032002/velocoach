# Audit of the project against the main plan, and everything still to do (version 2)

> **Update 2026-10-16.** The decisions asked for here were answered (ANSWERS v1: 1A 2A* 3A 4C
> 5A 6B) and applied: observer 0.3.0 ([EFFICACY_CHANGELOG.md](EFFICACY_CHANGELOG.md)), the
> Android Gradle and XcodeGen projects (both built in CI), the setup scripts, the
> pre-registration, the cue bank and the README status table. The README's
> [Status](../README.md#status) table is the current state. What remains is the phone work
> in [PHONE_CHECKLIST.md](PHONE_CHECKLIST.md). This document is kept as the record of the audit.


This document replaces the first `REMAINING_WORK.md`. It has three parts:

1. **Audit.** The whole repository checked against the main plan ("velocoach-rs — Final Plan,
   Days 5–20", v-final, Oct 11 2026): what was verified and how, what was wrong and has now
   been fixed, what is still open, and what could not be checked from this environment.
2. **Review of version 1.** What was wrong or missing in the earlier remaining-work document,
   so you can see what changed and why.
3. **What is not done, and how to do it.** Thirteen work packages (WP1–WP13) in dependency
   order, each with the plan reference, the gap, exact steps, an acceptance test and the plan's
   fallback.

Companion documents: `PROJECT_GUIDE.md` (how everything works and links), `PLAN_VS_REPO.md`
(criterion table), `COMPLETION_GUIDE.md` (short checklist), `RUNBOOK.md` (session procedure),
`DEMOS.md` (scenes and script), `EFFICACY.md` (observer, E1 evidence).

---

# Part 1. Audit

## 1.1 How the project was checked

| Plan section | What was checked | How | Result |
|---|---|---|---|
| §3.2 crate map and dependency rules | `vc-efficacy` depends on no workspace crate; `serde_json` only in `vc-report`; no randomness or clock reads in the `no_std` crates; no `std::time` | read every `Cargo.toml`; grep for `std::time`, `SystemTime`, `Instant` | **correct** |
| §3.3 data model and ledger schema | every field of the plan's table exists in `CueLine` (plus the optional `t0..t3`) | read `schema.rs`, compared field by field; example line from the golden ledger | **correct** (additions are optional fields) |
| §3.4 APIs | every listed Engine / FFI / report / CLI function exists | grep; generated Swift and Kotlin bindings read | **correct**, except `vc-cli ble` (not built, Lite fallback L2) |
| §3.5 opcode table | opcodes and sizes | a scratch test encoded every message and printed the length and opcode | **differs from the plan in 5 places**, all allowed by the plan's "keep the code's numbers" (table in WP13) |
| §3.6 Policy Engine and profiles | profile table values; decide() order (P1 first, Group mutes P2/P3, cooldown, one open window); phrase bank; earcon table | read `config.rs`, `policy.rs`, `phrase.rs`, `tools/make_earcons.py` | **correct** ("Soft-pedal" hyphenated, documented) |
| §3.6 directive rules | safety 98 %/120 s; cardiac drift 20 s; cadence ≥10 rpm/8 s; W′ 20 %; HR freeze 30 s; dropout cue at 5 s; Stale 3 s / Lost 10 s | read the defaults in `config.rs`, `quality.rs` | **correct** |
| §3.7 observer | anchor, 10-s OLS, ±2 W/s clamp, window, hold = min of 3 bins, complied/reversed/partial rules, confounders, coincidental flag | read `observer.rs`; unit tests; E2 | **matches the plan exactly — including a flaw in the plan itself** (E1, below) |
| §3.7 ledger and tuner | Beta counts, 16-entry ring, rules T1–T4/T2-P3/T3, n ≥ 8, bounds, one change per parameter per boundary, frozen outside Coach | read `ledger.rs`, `tuner.rs`; E3/E4 | **correct** |
| §3.8 latency | T0..T3 definitions, Cristian min-RTT of 16, ± rtt_min/2, onset sources, tick-released cues have no T0 | read `clock.rs`, `chain.rs`, app code | **correct after two fixes** (below) |
| §3.9 study mode | draw from PCG32(seed, draw index): u < 0.2 sham, then 0.5/0.5; P1 never drawn; sham consumes cooldown, plays nothing, sends nothing, gets a virtual onset | read `study.rs`, `policy.rs`; M3/M4 tests | **correct** |
| §4 criteria S1–S10, E1–E8, M1–M8 | each test run by name (output below) | `cargo test` per criterion | **all green except E1**; S1, S2, S3, S6, E7 need hardware; M-criteria are tool-ready |
| §5 Days 5–20 | each day's deliverables exist | file inventory + the day documents | **all code and document deliverables exist**; field deliverables open |
| §7 study protocol | allocation, outcomes, analysis, language, consent text in `STUDY.md` | read; ran the analysis pipeline on 9 synthetic sessions | **pipeline works end to end** (129 ledger lines, audit M2–M5 PASS); one protocol number was unachievable (1.4) |
| §8–§10 demos, video, message | scenes, shot list, script, message template | produced `DEMOS.md`; ran D2/D3/D4/D5 headlessly | **correct**; D4 needed a new scenario |
| §12 fact-check ledger | each claim | opened `WPrime.cpp`; others need the web | GoldenCheetah claim **corrected** (default is the integral form); rest "to verify" |

Whole-repository evidence, re-run at the end of this pass:

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features` | **201 passed, 0 failed** (E1 and E8 are release-only and run separately) |
| `vc-cli golden --check` | 5 of 5 identical |
| E6 (`creep`, Study, seed 11, run twice, compared with `golden/creep_study_11.jsonl`) | byte-identical |
| `cargo build` of the five `no_std` crates for `thumbv7em-none-eabihf` (S8) | builds |
| E8 (release) | 190 / 200 = 95.0 % |
| **E1 (release)** | **fails, 0 / 20 seeds** |
| Swift and Kotlin sources | **parse without syntax errors** (tree-sitter); they have not been type-checked or compiled |
| `ci.yml` | valid YAML; every local link in the documents resolves |

## 1.2 Verified correct (no action)

- The sans-IO architecture holds: the dependency rule, `no_std`/no-alloc hot path (S10, E5 allocation
  tests), determinism (E6, goldens, capture replay test), randomness only in `vc-core::study` and
  the seeded bootstrap.
- The Policy Engine, profiles, phrase lint (S7), safety properties (S9), the tuner bounds (E3/E4),
  the confounder table (E2: 6/6 plus the control), the Study freeze and "P1 never carries an arm" (M3/M4).
- The wire protocol round-trips (38 `vc-gatt` tests) and all six fuzz targets exist.
- The ledger schema, the report pipeline and the audit, including on a rehearsal of the whole pilot:
  nine synthetic sessions (R01–R03 × 3, `creep`, Study, seeds 1001–3003) → 129 lines, 44 drawn decision
  points, M2 frequencies PASS, M2 replay 0 mismatches, M3 0, M4 one `policy_hash`, M5 100 % valid.
- W′bal against GoldenCheetah: an independent Python port written from `WPrime.cpp` matches
  `WbalGc` to 0.000000 J on `climb_surge` and `soak`.

## 1.3 Defects found in this pass and fixed (commits `65d3103`, `b9d1e36`)

| # | Defect | Effect if left | Fix |
|---|---|---|---|
| F1 | **STAMP counters did not restart on a new ride.** The iPhone creates a new Engine (packet counters at 0) for every ride; the rig reset its counters only on a new Bluetooth connection. | From the second ride on a connection, no STAMP matched its packet: T0 missing everywhere, `stamps_lost` climbing, S2 unmeasurable. The study (R00, then nine sessions) and the soak all reuse one connection. | `NotifyQueue.resetCounters()` is also called on LOAD (`SimDriver`) |
| F2 | **The slider's position was not the simulated effort.** The slider showed one value while the Simulator ran at its own default (60 %) until the first move; a LOAD replaced the Simulator and reset it again; and the coach got no baseline RIDER_INPUT until the first move. | wrong power at the start of every session; `L_onset` empty for the first cue | `SimDriver` remembers the slider, re-applies it after each LOAD, and sends a baseline RIDER_INPUT 2 s after START; the screen sends the position when the switch goes on; default 70 % |
| F3 | **Cues with no T0 recorded a sync bound of 0.** Tick-released cues have no triggering packet, so `clock_sync_error_bound_us` was 0 for them. | the M5 audit's 95th-percentile bound understated the real bound | such cues now record the clock's current bound (0 only when there is genuinely no clock, as in headless runs; goldens unchanged) |
| F4 | **The pilot's scenario could not produce the pre-registered number of decision points** (details 1.4, item 1). | n ≈ 40 instead of 100–180; about 8 shams | new scenario 11 `creep_pilot`; `STUDY.md`, `RUNBOOK.md`, `DEMOS.md` use it; regression tests in `crates/vc-harness/tests/pilot_yield.rs`; `RunOpts.human_permille` added |
| F5 | `ENGINEERING_NOTES` carried the plan's unverified "zero-copy" assumption | wrong statement in the shipped notes | recorded: bytes cross as `Data` / `ByteArray` (one copy of 4–20 bytes) |

## 1.4 Findings still open (they need your decision or your hardware)

1. **Decision-point yield (design risk for the whole pilot).** `STUDY.md` expected "15–20 decision points
   per session". Measured headlessly, the original `creep` scenario gives 4–6 per session (four creep blocks;
   the grey-zone directive re-arms only at a step boundary): about 40 for nine sessions, about 8 shams,
   so the cue-versus-sham comparison would rest on ~8 observations. `creep_pilot` gives 17–20. **In human
   mode the yield also depends on where the rider holds the slider**, because the creep is added to the
   slider's power and the grey-zone floor is 190 W: a human at 60 % (150 W) plus 32 W of creep reaches
   only 182 W and gets **no cues at all**; at 70 % or more, 17–20. That is why the pilot now starts the
   slider at 70 %. Two risks remain and only the dry run (WP7) can settle them: a rider who pre-empts the
   drift generates fewer cues, and a rider whose slider sits well above 70 % starts every cycle inside the
   zone. If the dry run yields fewer than about 12 decision points, adjust the cycle (not the briefing
   card) and re-run the dry run **before** committing `STUDY.md`.
2. **E1 fails, and the cause is partly in the plan itself.** §3.7 defines the hold as `H(t) = min E over
   [t, t+3 s]`, complied as `H ≥ 0.5Δ*` (all three bins high) but reversed as `H ≤ −0.2Δ*` (any one bin low).
   The repository implements this exactly. With a 10 W floor, −2 W is inside the noise of a 3-s mean, so
   reversal fires far too easily (2,294 of 4,000 scored cues). Two further causes: the E1 test counts the
   cardiac-drift cue that no persona can react to, and the 2.0 s smoothing lag is really about 1.5 s. Three
   small changes bring 18 of 20 seeds into band (WP1). Until you decide, every "reversed" rate and every
   compliance proportion is uncalibrated.
3. **S4 evidence is thin.** The plan wants ≥ 10 minutes of fuzzing per decoder. DEVLOG records one 10-minute
   run (ctrl, 471.5 M executions, Day 3) and an aggregate of 35.1 M executions (Day 7); there is no per-decoder
   log for all six targets. `cargo-fuzz` and a nightly toolchain were not available here. Run
   `scripts/fuzz-all.sh 600` and keep `build/fuzz-logs/`.
4. **Wire-format differences from the plan's §3.5 table** (WP13). Harmless, but the plan says to update the
   table; the real table is in `ENGINEERING_NOTES.md` terms below.
5. **The plan's fact-check ledger is wrong on one line**: GoldenCheetah's default W′bal form is the Skiba
   integral; the differential model the plan names is the alternative setting.
6. **Plan M7 vs the Day 18 CI step** list different mandatory labels ("n-small" vs "personas are assumptions").
7. **Observer behaviour on creep** still deserves one look after E1 (WP1): on `creep_long`, cardiac-drift cues
   (which no persona answers) often read *reversed*, because the creeping power moves opposite to the ask.
   Confirm that the pre-trend absorbs the creep for the P2 cues the pilot actually uses.

## 1.5 What could not be verified here

- Anything that needs a radio, a speaker, a camera, a person or a clock: S1, S2, S3, S6, E7, M8, the entire pilot.
- Whether the Swift and Kotlin sources compile. They parse; every FFI name they call was checked against
  freshly generated bindings; type errors are possible.
- The facts that need the web: Aki's posts, the two reaction-time papers, the Piper voice's licence, the
  GoldenCheetah file's commit hash (GitHub's API and Hugging Face were unreachable).
- Fuzzing (no `cargo-fuzz`/nightly here) and the criterion benchmark numbers.

---

# Part 2. Review of version 1 of this document

| # | What version 1 said | Problem | Now |
|---|---|---|---|
| R1 | The pilot yields 15–20 decision points on `creep` | false: 4–6 | finding 1.4.1, F4, WP3, WP7 |
| R2 | Slider at 60 % for the dry run and sessions | at 60 % the pilot scenario gives no cues | 70 % everywhere; test added |
| R3 | WP13 table: "PONG/STATUS/FAULT sizes differ; RATE `hz_x10`" | PONG matches the plan (the plan's byte column excludes the 2-byte header); CLEAR carries a kind byte; RATE's opcode is 0x40, not 0x32 | corrected table in WP13 |
| R4 | E1 fix: change the constant and rerun | `SMOOTHING_LAG_MS` also feeds the tuner (T1 uses it; the ledger's "complied under 1 s / 2 s" counts subtract it), so the demo D2 timing, `tuner_props` and the golden ledger can move | WP1 now lists the knock-on checks |
| R5 | E1's reversed rule presented only as "the plan's rule" | it should say the plan's own definition is asymmetric | 1.4.2 and WP1 |
| R6 | WP8 sessions assumed the rig and iPhone counters stay aligned | they did not (F1) | fixed; WP6 now checks it |
| R7 | WP11: `git push origin main v1.0.0` | the work lives on a feature branch: merge first | WP11 step 5 |
| R8 | iOS deployment target stated as 16 in the companion guide | the package declares iOS 17 | corrected in `COMPLETION_GUIDE.md` |
| R9 | No mention that ENGINEERING_NOTES' "zero-copy" was unverified, nor that S4 evidence is thin | gaps | 1.3 F5, 1.4.3 |
| R10 | Treated dense-scenario S2 cues as all packet-triggered | a time-based P1 cue gets a T0 only if it happens to be decided inside an `ingest` call (the packet is then recorded as its trigger) | WP6c explains and asks for the n |
| R11 | WP2 asked to commit Xcode/Gradle projects but did not say the sources were never compiled | made the risk look smaller | stated in 1.5 and WP2 |

---

# Part 3. What is not done, and how to do it

## 0. Where the project stands against the plan

| Plan layer | Plan's confidence | State |
|---|---|---|
| 1. Real-time core, end to end over real BLE on two phones (v0.1.0) | ~70 % | **Code complete, never run on hardware; apps never compiled.** Gate G-11 not attempted |
| 2. Headless observer, ledger, tuner, report (v0.2.0) | ~85 % | **Complete and green except E1.** Not tagged |
| 3. Pre-registered slider pilot, docs, video, ship | ~50–60 % | **Tooling and documents complete; no data.** All field work open |

### Critical path

```
WP1 E1 ─┐
        ├─► WP3 pre-registration ─► WP7 dry run ─► WP8 soak+sessions ─► WP9 analysis ─► WP11 ship
WP2 apps build ─► WP4 gate G-11 ─► WP6 latency (S2/S3) ─┘                         ▲
                       WP5 gate G-14 (needs WP1) ───────────────────────────────┘
WP10 docs/demos/recording runs in parallel with WP8–WP9 and finishes before WP11
```

### Hands-on time

| Package | Time | Constraint |
|---|---|---|
| WP1 E1 | 2–4 h | none |
| WP2 apps build | 3–6 h | Mac + Xcode, Android Studio |
| WP3 pre-registration | 0.5 h | after WP1 |
| WP4 gate G-11 | 3–4 h | both phones, earbuds |
| WP5 gate G-14 | 0.5 h | after WP1 |
| WP6 latency | 3 h | camera, earbuds |
| WP7 dry run | 2 h | re-sign first |
| WP8 soak + sessions | 1 h unattended + 9 × (20 min + set-up/export) ≈ 6 h | friends |
| WP9 analysis | 2–3 h | after the last session |
| WP10 docs, rehearsal, recording | 8–10 h | quiet room |
| WP11 ship | 6–7 h incl. a 1 h soak | CI, upload |
| WP12 cut-list items | optional | |

---

## WP1. Resolve E1 (the observer must recover a known rider)

**Plan reference:** Day 13 H5; **E1**; the never-cut list ("Observer + ground-truth test"); gate G-14.

**Gap:** E1 fails on all 20 seeds (compliance 0.28–0.38 against 0.70 ± 0.08). The v0.2.0 tag would be false and
every compliance proportion in the pilot uncalibrated.

**Why (all three shown by experiment; reverted; details in `EFFICACY.md`):**
1. E1 counts the cardiac-drift (P3) cue, which carries no phrasing arm; no persona reacts to it.
2. Reversal needs one low bin; compliance needs three high bins (the plan's own `H = min` definition). 2 W of
   reversal is inside the noise.
3. `SMOOTHING_LAG_MS` is 2000 but measures about 1500, so latency reads 0.45 s low.

**Decide first (write it in DEVLOG):**

| Option | Do | Expected E1 |
|---|---|---|
| A (recommended) | all three changes | about 18/20 seeds |
| B | changes 1 and 2 | about 15/20 (latency off) |
| C | nothing | 0/20; no document may call the observer validated; v0.2.0/v1.0.0 must say "E1 open" |

**Steps for option A**

1. `crates/vc-harness/tests/e1.rs`: after `if scored == CUES { break; }` add
   ```rust
   let modelled = run.decisions.iter().find(|d| d.id == o.cue_id).is_some_and(|d| {
       matches!(d.arm, vc_efficacy::Arm::Direct | vc_efficacy::Arm::Autonomy)
   });
   if !modelled { continue; }
   ```
2. `crates/vc-efficacy/src/observer.rs`: add
   ```rust
   /// Largest effect over bins k .. k+2: <= -x here means all three bins are reversed.
   fn hold_max(&self, ring: &P3Ring, k: u64) -> Option<f64> {
       let mut h = f64::NEG_INFINITY;
       for j in k..k + HOLD_BINS { h = h.max(self.effect(ring, j)?); }
       Some(h)
   }
   ```
   and in `Window::step` replace `if h <= -REVERSE_FRAC * self.delta {` with
   `if self.hold_max(ctx.ring, k).is_some_and(|m| m <= -REVERSE_FRAC * self.delta) {`.
3. `SMOOTHING_LAG_MS` → `1_500` (fix its doc comment). **It is also used by the tuner and the ledger**
   (`tuner.rs` T1 median; `ledger.rs` lag-corrected "complied under 1 s / 2 s"). So after the change:
   `cargo test --workspace` (tuner and ledger tests), then re-run D2 (`vc-cli sim --scenario creep_long
   --jsonl /tmp/d2.jsonl; grep tuner_change /tmp/d2.jsonl`) and update `docs/DEMOS.md` with the new
   `tuner_change` time and numbers. Update the "2.0 s" statements in `LATENCY.md`, `EFFICACY.md`, the README
   limits, `DEMOS.md` (0:35 voiceover) and `ENGINEERING_NOTES.md`.
4. Bump `OBSERVER_VERSION` (`crates/vc-efficacy/src/lib.rs`) to `"vc-efficacy 0.3.0"`.
5. Regenerate and inspect the committed artefacts:
   ```
   cargo run -q -p vc-cli -- sim --scenario creep --mode study --seed 11 --jsonl golden/creep_study_11.jsonl
   cargo run -q -p vc-report -- golden/creep_study_11.jsonl -o docs/examples/creep_study_11.html
   cargo run -q -p vc-cli -- golden --check          # the five text goldens should not change
   ```
6. Everything:
   ```
   cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   cargo test --release -p vc-harness --test e1 -- --nocapture
   cargo test --release -p vc-report --test e8 -- --nocapture       # stay within 92–98 %
   ```
7. The stragglers (seeds 8 and 12 sat 0.01 under the 0.62 floor): either document a ±0.10 tolerance as a deviation,
   or print per-verdict counts for seed 12 and look for a fourth cause. Understand the residual before tuning anything.
8. Paste the 20-row table into `docs/EFFICACY.md` (replace the OPEN section), update README / RELEASE_NOTES /
   PLAN_VS_REPO, and remove `continue-on-error: true` from the E1 step in `.github/workflows/ci.yml`.
9. Look at P2 verdicts on `creep_pilot` and `creep_long` (finding 1.4.7): an ignored cue should read *ignored*, not
   *reversed*. If not, the 2 W/s slope clamp or the 10-s fit window is mishandling the creep.

**Acceptance:** E1 passes (or option C is written into every place that mentions the observer's validity); E2, E3, E4,
E8 pass; CI green; DEVLOG has the decision and numbers.

---

## WP2. Make both apps build from the repository

**Plan reference:** Days 8–10 (FFI shells, rig, coach), CI "ios"/"android".

**Gap:** no Xcode project for `VeloCoach`, no Gradle project for the rig, the generated cue bank is not committed,
and none of the Day 15–17 app code has ever been compiled (it parses; see 1.5).

### 2a. iPhone (Xcode, on the Mac)

1. `scripts/build-ios.sh` → `apps/ios/VcCore/VcFFI.xcframework` and the Swift bindings (package targets **iOS 17**).
2. Xcode → New iOS App `VeloCoach` (SwiftUI, Swift), saved under `apps/ios/`. Delete the template's `ContentView.swift`
   and app file (the `@main` is in `RideView.swift`).
3. Add to the target: `AudioCuePlayer.swift`, `BLECentral.swift`, `Clock.swift`, `CoachModel.swift`, `LoopbackView.swift`,
   `RideView.swift`, `StudyConfig.swift`, `Uuids.swift`.
4. Add the local package `apps/ios/VcCore`; link `VcCore`.
5. Info: point *Info.plist File* at `VeloCoach/Info.plist` (and turn off *Generate Info.plist File*), or copy its keys
   (Bluetooth usage string, `UIFileSharingEnabled`, `LSSupportsOpeningDocumentsInPlace`, background modes
   `bluetooth-central` and `audio`). Capabilities → Background Modes. A free team may refuse background modes: record it
   in DEVLOG and keep the screen on during runs.
6. Signing: personal team, unique bundle id; deployment iOS 17.
7. Cue bank: install Piper and a voice (`voices/en_US-lessac-medium.onnx` + `.json` by default), run
   `scripts/render-phrases.sh`, add `apps/ios/VeloCoach/Cues` as a **folder reference** (blue folder). **Check the voice's
   licence before committing the audio** (WP10).
8. Run on the iPhone. Expect `vc: N cue buffers loaded`. Likely compile errors: a binding label (names were checked, but
   confirm), a `String(format:)` argument type (`%u`↔`UInt32`, `%ld`↔`Int`, `%llu`↔`UInt64`), the tuple returned by `audio.play`.
9. Commit the `.xcodeproj` (not `xcuserdata`) and add a CI step
   `xcodebuild -project apps/ios/VeloCoach.xcodeproj -scheme VeloCoach -destination 'generic/platform=iOS Simulator' build`.

### 2b. Rig (Android Studio)

1. `ANDROID_NDK_HOME` (r27c in CI), `cargo install cargo-ndk`, `scripts/build-android.sh` → `jniLibs/arm64-v8a/libvc_ffi.so`
   and the Kotlin bindings (package `dev.velocoach.ffi`, set by `crates/vc-ffi/uniffi.toml`; built with `--features sim`).
2. Add Gradle files (versions to taste):
   `apps/android/settings.gradle.kts`
   ```kotlin
   pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
   dependencyResolutionManagement { repositories { google(); mavenCentral() } }
   rootProject.name = "velocoach-rig"
   include(":app")
   ```
   `apps/android/build.gradle.kts`
   ```kotlin
   plugins {
       id("com.android.application") version "8.5.2" apply false
       id("org.jetbrains.kotlin.android") version "1.9.24" apply false
   }
   ```
   `apps/android/app/build.gradle.kts`
   ```kotlin
   plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
   android {
       namespace = "dev.velocoach.rig"
       compileSdk = 34
       defaultConfig { applicationId = "dev.velocoach.rig"; minSdk = 31; targetSdk = 34; versionCode = 1; versionName = "1.0" }
       compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
       kotlinOptions { jvmTarget = "17" }
   }
   dependencies { implementation("net.java.dev.jna:jna:5.14.0@aar") }
   ```
   (`minSdk 31`: `BLUETOOTH_ADVERTISE` is Android 12; the manifest already carries the Android 14
   `FOREGROUND_SERVICE_CONNECTED_DEVICE` permission and service type.)
3. Install on the iQOO; grant Bluetooth and notification permissions. Expect `services 4/4 · advertising true`.
4. nRF Connect (Day 9 H7): four services, right feature values; write LOAD + START by hand → 60/240/60 notifications
   per minute, 0 dropped.
5. Commit the Gradle files; add `./gradlew :app:assembleDebug` to the android CI job.
6. **Vivo (plan risk: kill medium, impact high):** Battery → this app → *No restrictions*; i Manager → Autostart → allow;
   lock the app in Recents; rig in front, screen on. Write the menu names in DEVLOG and `ENGINEERING_NOTES.md`.

### 2c. Smoke test (10 min)

Rig in front → `link: ready` → `climb_surge`, Dana → Start. Pass: the latency panel fills; `T0>T1` stays 0; `stamps`
climbs; **start a second ride without reconnecting and confirm `stamps` and T0 still match** (fix F1); a cue with an earcon
first; a badge; the report card at Stop; Export offers capture + ledger.

**Acceptance:** both apps install from a clean checkout following only these steps; CI compiles both.

---

## WP3. Freeze the pre-registration (M1)

**Plan reference:** Day 15 H5; **M1**; never-cut "consent and pre-registration".

**Gap:** the date in `STUDY.md`, the two deviation dates, and `StudyConfig.protocolHash` (`"PENDING"`).

**Steps (after WP1 — the observer is part of what you pre-register)**

1. Read `STUDY.md` as a stranger would. **It now names the `creep_pilot` scenario with the slider starting at 70 %** (see
   1.4.1); keep that, or revert to `creep` and rewrite the "expected decision points" line honestly (4–6 per session, 12–18 per
   person, 36–54 total). Do not commit text you know the code cannot meet.
2. If the dry run (WP7) will change the scenario, do the dry run **before** this commit. The documents order it the other way
   (commit on Day 15, dry-run Day 16) and handle a change as a Deviation; either is honest, but a deviation is noisier.
3. Replace `Date: [YYYY-MM-DD]` and both `2026-10-__` dates.
4. `git add STUDY.md && git commit -m "docs: STUDY.md pre-registration"`.
5. `git log -1 --format=%h -- STUDY.md` → `StudyConfig.protocolHash`; commit that **separately**.
6. `scripts/check-protocol-hash.sh` → `ok`; run before every build and session.
7. Any later STUDY.md change repeats 4–6; sessions before and after are reported separately.

**Acceptance:** the script prints `ok`; the app shows the same hash; the first Study ledger line carries it.

---

## WP4. Gate G-11: the two-phone run, S1, replay, Vivo, v0.1.0

**Plan reference:** Day 11; **G-11**; **S1**; K1/L1.

**Gate:** ≥ 15 min without an unrecovered disconnect · earcon-first cues · identical replay · CI green · T1→T2 measured.

1. Both phones on chargers, Vivo settings done, earbuds paired, volume calibration played once.
2. 20-minute run: `climb_surge`, Dana, Coach (Study off). Provoke one recovery: rig Bluetooth off/on; the iPhone must
   reconnect by itself in about 20 s.
3. **S1:** read the T1→T2 `p99` (µs) before stopping; target < 1 ms. Note capture-on and, if you want it, a build with capture
   off. DEVLOG Day 11 and `LATENCY.md` S1.
4. Export → AirDrop:
   ```
   mkdir -p runs/g11 && mv ~/Downloads/ride-*.vcraw ~/Downloads/ledger-*.jsonl runs/g11/
   cargo run -q -p vc-cli -- replay runs/g11/ride-*.vcraw       # must print: identical
   ```
   `different` means a hidden input in the Engine: fix the Engine, never the capture.
5. Vivo: 30 minutes, unplugged, screen on; service alive, CPM sent ≈ 6000; then 2 minutes screen-off for the record.
6. `git tag -a v0.1.0 -m "G-11: iQOO -> BLE -> Rust on iPhone 12 -> spoken cue. Replay identical. T1->T2 p99 __ us."`

**Fallbacks:** **K1** the gate fails → Day 12 mornings are fixes; still failing at end of Day 13 → **L1** (in-process simulator
on the iPhone, latency labelled "in-process"). **L2** (Mac central + `afplay`) needs `vc-cli ble` (WP12.1).

---

## WP5. Gate G-14: tag v0.2.0

**Plan reference:** Day 14 H7; E1–E6, E8, S4–S10; K2.

1. After WP1: workspace tests, release E1/E8, `golden --check`, CI green on both OSes.
2. E5: `cargo bench -p vc-efficacy -- observer` (note µs per update); allocation tests as in 1.1.
3. **S4:** run `scripts/fuzz-all.sh 600` (nightly + cargo-fuzz) and keep `build/fuzz-logs/`; the recorded evidence is thin (1.4.3).
4. `git tag -a v0.2.0 -m "G-14: headless closed loop. E1 20/20 seeds, E8 coverage 190/200, observer __ ns, 0 allocs."` —
   write the real E1 count. **K2:** gate not green → drop friend #3, feedback buttons; pilot is you alone.

---

## WP6. Day 15: clock sync, d_route loopback, S2

**Plan reference:** Day 15 H1–H4; **S2**, **S3**; risks "route delay jittery", "`.dataPlayedBack` early".

### 6a. Clock sync (15 min)
Start any ride. Healthy: `rtt_min` a few ms; bound ± rtt_min/2 well under 15 ms (M5 uses the 95th percentile); `T0>T1` = 0;
`lost` small. Start a **second** ride without reconnecting and confirm it still matches (F1). If `T0>T1` climbs: confirm `b` is
stamped in the binder callback, `reset_link` runs on connect, and ping when the queue is quiet.

### 6b. d_route by loopback (about 1 h)
Equipment: iQOO as camera on a stand, earbuds, tape.
1. iPhone: *Loopback*; full brightness; auto-brightness off; Do Not Disturb.
2. iQOO camera: highest frame rate that still records normal-speed sound (test a 2 s clip first). 120 fps ≈ 8 ms.
3. **Earbuds run:** one earbud taped to the iQOO's bottom microphone; record; *Start (20 clicks)*; wait for `20 / 20`.
   **Speaker run:** earbuds disconnected, iPhone speaker facing the same mic position.
4. `python3 tools/loopback.py runs/loopback/earbuds.mp4` and `.../speaker.mp4` (needs `ffmpeg`, `ffprobe`, `numpy`). Expect 20
   pairs each. Swing > ±50 ms between repetitions → say so; report median ± IQR.
5. `d_route(earbuds) = median(earbuds) − median(speaker) + outputLatency(speaker)`; `outputLatency` is logged
   (`vc: route [...] outputLatency N ms`). iOS's estimate for the earbuds is `T4_est`; your d_route is `T4_meas`.
6. Enter the median in *Route delay (ms)*; onsets are then `loopback`.
7. `.dataPlayedBack` check: *skew flags* counts callbacks more than ±50 ms from T3 + dur + d_route. Speaker route should be 0.
   Do not subtract `outputLatency` from `.dataPlayedBack` (plan decision 6).
8. Fill `LATENCY.md` S3, the method (fps, resolution) and the agreement section.

### 6c. S2 (15+ minutes)
`dense`, Dana, Coach, earbuds, route delay entered, chargers. Note: a time-based cue (countdown, "Go now") gets a T0 only if
it is decided inside an `ingest` call, where the triggering packet is recorded; cues decided on a tick have T1→T3 only. Read
`T0→T3 p99 ± bound`, `T0→T1`, `T1→T3`, `stamps/lost` **and the n of packet-triggered cues** (the plan assumed about 60).
Export to `runs/s2-15min/`. With ~60 cues p99 is nearly the maximum: say so in `LATENCY.md` and give the per-packet T0→T1
distribution beside it. Target p99 < 150 ms with the bound shown; if missed, report where the time went (radio vs the deliberate
50 ms lead) — the plan reports, it does not hide.

---

## WP7. Day 16: re-sign, badges, human mode, dry run, LATENCY.md

**Plan reference:** Day 16; **K3**; risk "free profile expires mid-study"; M5 groundwork.

1. Re-sign when fewer than 4 days remain: plug in, Product → Run; note the expiry (Settings → General → VPN & Device
   Management); run `check-protocol-hash.sh` first.
2. Badges test (`climb_surge`, Dana, Coach): a badge per scored cue; ✓ for the W′ cue; no `? grade` on flat sections; report
   card matches the badges.
3. Human-mode check (10 min): rig *I am the rider*, **slider at 70 %**; iPhone `creep_pilot`, Human rider on. Power ≈ 175 W
   in the warm-up (0.7 × 250) and rises ≈ 0.8 W/s in each 40 s block; move the slider down 10 % and power falls ≈ 25 W
   within a fraction of a second; when a grey-zone cue plays, ease off and read ✓ with your `L_onset`.
4. **Dry run, rider R00**, exactly like a counted session (`RUNBOOK.md`): hash check; rig on, slider 70 %; iPhone Study on,
   **R00, session 1 (seed 1)**, `creep_pilot`, Human rider on; read `policy_hash` off the screen and write it down; Start; phone
   face down; 20 minutes; Stop; Export:
   ```
   mkdir -p runs/study/R00 && mv ~/Downloads/ledger-*.jsonl ~/Downloads/ride-*.vcraw runs/study/R00/
   cargo run -q -p vc-cli -- replay runs/study/R00/ride-*.vcraw
   cargo run -q -p vc-report -- runs/study/R00/*.jsonl -o runs/study/R00/report.html --audit
   ```
   Pass: replay `identical`; M3 0; M4 one `policy_hash`; M5 ≥ 90 % valid and p95 bound ≤ 15 ms; every Complied cue has an
   `l_onset_ms`; the iQOO showed nothing but the slider; comfortable volume, every phrase understood first time; and **the yield:
   count decision points** (`grep -c '"arm":"\(sham\|direct\|autonomy\)"' runs/study/R00/*.jsonl`, or read the report). The plan expects 15–20. Below about
   12 means the creep cycle does not fit this rider: adjust the scenario (not the briefing), dry-run again, and only then commit
   `STUDY.md` (WP3). Also look at how many cues the human pre-empted.
5. Protocol changes found now go in `STUDY.md` Deviations (or, if not yet committed, simply into the text), recommit, update
   `protocolHash`, dry-run again. Interface fixes need no deviation.
6. Fill `LATENCY.md` completely, including three per-cue chains (fast, median, slow) from the dry run's `t0_ns…t3_ns`.
7. **K3:** human mode not working by tonight → pilot in L1 (Simulator in-process on the iPhone with an on-screen slider); a
   dated deviation; latency labelled "in-process".
8. DEVLOG Day 16, including "K3: human mode working: yes/no".

---

## WP8. Day 17: soak 1 (S6), then the counted sessions

**Plan reference:** Day 17; **S6**; M2–M5 per session; **M8**.

### 8a. Soak 1 (60 min, unattended)
1. Chargers on; rig in front, screen on; earbuds; route delay set.
2. iPhone: `soak`, Dana, Coach, Start. The app sends four RATE changes (1,500 / 1,560 / 2,700 / 2,760 s); the scenario has five
   faults (power dropout 600 s, garbage 1,200 s, disconnect 1,800 s, heart-rate dropout 2,400 s, coasting 3,000 s). Touch nothing.
   If background modes were granted, lock the iPhone from 30:00 and log it; otherwise leave the screen on and log that.
3. Fill the `ENGINEERING_NOTES.md` blanks while it runs.
4. Afterwards:
   ```
   mkdir -p runs/soak-1 && mv ~/Downloads/{soak,ride,ledger}-*.{csv,vcraw,jsonl} runs/soak-1/ 2>/dev/null
   cargo run -q -p vc-cli -- soak-check runs/soak-1/soak-*.csv      # S6: PASS, every fault recovered
   cargo run -q -p vc-cli -- replay runs/soak-1/ride-*.vcraw         # identical
   ```
   Note the rig's `dropped` counter and both batteries. STAMP loss < 0.5 % is the earlier target.
5. On failure open the capture first, the soak CSV second. A fix is allowed only if decisions stay untouched; rerun. Counted
   sessions wait for a passing soak.

### 8b. Counted sessions
Follow `RUNBOOK.md` verbatim. Per session: `check-protocol-hash.sh` ok → Study on, correct rider/session (the seed on screen is
1000 × rider + session) → `policy_hash` equals SESSIONS.md → rig on, **slider 70 %** → Human rider on, `creep_pilot` → Start, phone
face down, researcher silent → Stop → Export → `runs/study/R0x/` → replay `identical` → `vc-report --audit` (M3–M5) → SESSIONS.md
entry. ≥ 20 minutes between one person's sessions. R01 S1–S3 (1001–1003), R02 (2001–2003) and R03 (3001–3003) with consent first.

### 8c. Consent (M8)
Adult only; two printed copies; explain the three things (a test of software, stop any time, deletion on request); the publication
line is optional, default no; both sign and date both copies; `CONSENT.md` holds pseudonym, date, publication yes/no, sessions —
never a name. Briefing card word for word; answer in-session questions only with "I'll tell you everything after your last session".

### 8d. End of every study day
```
cd runs/study && shasum -a 256 R0*/* > SHA256SUMS
zip -er ~/study-backup.zip . -x '*.DS_Store'            # keep a second copy elsewhere
shasum -a 256 -c SHA256SUMS
cargo run -q -p vc-report -- runs/study/R0*/*.jsonl --exclude-rider R00 --audit --audit-only
```
M4 must show one `policy_hash`; two means stop and find out why.

**Freeze (plan rule):** from the first counted session to the last, no change that affects decisions, verdicts or the ledger.

---

## WP9. Day 18: remaining sessions, K4, analysis, Results

**Plan reference:** Day 18; **K4**; M2–M7.

1. Re-check the hash, build and signing expiry. Sessions: R02 S2–S3; R03 consent + S1–S3. Debrief each participant after their last
   session, write down what they noticed; a noticed pattern is a limitation, not a reason to drop data.
2. **K4** (table in `RUNBOOK.md`): friend unavailable → two-person study and say so; aborted session → keep, no repeat; withdrawal
   → delete files and backups that day and re-run; everything fails → single-person feasibility run. Never top up with your own sessions.
3. Final backup; nothing is added afterwards.
4. Audit and analysis:
   ```
   cd runs/study && shasum -a 256 -c SHA256SUMS && cd -
   for f in runs/study/R0*/ride-*.vcraw; do cargo run -q -p vc-cli -- replay "$f"; done
   cargo run -q -p vc-report -- runs/study/R0*/*.jsonl --exclude-rider R00 -o runs/study/results.html --audit | tee runs/study/audit.txt
   ```
   All five audit lines PASS before reading any result; otherwise report affected sessions with and without them.
5. Write `STUDY.md` Results (the template is in place): n and people, exclusions, data quality, both primary outcomes with Wilson 95 %
   intervals and the pooled within-person difference with its bootstrap interval, secondary outcomes, per-person table, deviations,
   limitations, consent count, the command and the commit. Mandatory wording; no p-values.
6. **M6:** ledger JSONL (never captures) of R01 and each consenting friend → `data/study/`; `vc-report data/study/*.jsonl -o
   docs/study/results.html`; commit both. A declined publication → say the published report is a subset.
7. **M7:** the CI grep checks "feasibility pilot", "slider proxy", "personas are assumptions"; the plan lists "n-small". Make the
   documents contain all four phrases, or add `"n-small"` to the loop.

---

## WP10. Day 19: re-check, fill the documents, rehearse, record

**Plan reference:** Day 19; §8; §12; risk "a new post introduces per-cue compliance".

1. Re-read Aki's Knowledge Hub and domesly.com (wording only). A new post on per-cue compliance changes the pitch to "an implementation
   of an idea you've written about, with an instrument to measure it"; cite it by title and date. The trial email's deliverable format wins.
2. `docs/FACT_CHECK.md` to its final state: verify at source Aki's parameter names/ranges and earcon table, Żak et al. 2023 and
   Buchholtz & Burgess 2020; add the GoldenCheetah commit hash (`git log -1 --format=%H -- src/Metrics/WPrime.cpp` in a clone).
3. `THIRD_PARTY.md`: read the Piper voice's model card; run `cargo deny check licenses` (or `cargo license`).
4. Fill every `__` (`COMPLETION_GUIDE.md` Appendix A maps each to its source). README top screen must fit a laptop screen.
5. Rehearse D1–D7 twice (record in `DEMOS.md`). Headless numbers already there: D2, D3 (fault 847), D4 (`grade_demo`), D5; D6 is now
   `creep_pilot` with the slider at 70 % (first cue about 100 s in). D1's badge shows L_obs, which includes the smoothing lag — say so.
6. Record pictures and voice separately; play D1/D3/D6 through the iPhone speaker with the caption "A rider would hear this in open-ear
   headphones."

---

## WP11. Day 20: edit, final soak, CI, fuzz smoke, hygiene, tag, upload, message

**Plan reference:** Day 20; S6 (second run), S4; video spec; message ≤ 150 words.

1. Edit to ≤ 3:00 (voiceover first); captions for every number and the three mandatory phrases during 2:05–2:30; the speaker caption on
   D1/D3/D6; own SRT; export 1080p H.264; upload **unlisted** as soon as it exports; automatic captions off.
2. **Final soak** on the shipping build (`runs/soak-2/`), same commands as WP8a; both soaks side by side in `ENGINEERING_NOTES.md`. A failure is
   documented honestly, not rerun until green.
3. CI green on every job; `scripts/fuzz-all.sh 20`; counts into `RELEASE_NOTES.md`; a crash blocks the tag.
4. Hygiene: `git status --short`; `git ls-files | grep -E '^runs/|\.vcraw$|CONSENT|SESSIONS|\.mp4$'` must print nothing; `git ls-files data/study`;
   `grep -h -o '"rider_id":"[^"]*"' data/study/*.jsonl | sort -u`; `git log --all --format='%an <%ae>' | sort -u`; `git grep -i '<friend name>'`
   locally for each friend, never written into a file.
5. **Merge the feature branch into `main` first** (the work lives on `claude/velocoach-days-9-14`), then:
   ```
   git tag -a v1.0.0 -m "velocoach-rs v1.0.0: closed-loop voice coach with per-cue efficacy scoring. S1 T1->T2 p99 __ us; S2 T0->T3 p99 __ ms +/- __ ms; S6 soak x2; E1-E8 green; pilot n = __ (feasibility)."
   git push origin main v1.0.0
   gh release create v1.0.0 --title "velocoach-rs v1.0.0" --notes-file docs/RELEASE_NOTES.md docs/study/results.html
   ```
   Only when E1, CI, both soaks and the hygiene check are honest. Make the repository public; open every README link in a private window.
6. The message (≤ 150 words; numbers from the README; the caveat in the same breath as the pilot; no location; Aki's name right; nothing that
   grades his work). The plan's §9 and the Day 19–20 document have two versions; the trial email's channel and format win. Do not call the
   observer validated unless E1 passes.
7. Buffer hour: blockers to sending first; then a fuzz crash or soak failure; then documentation fixes. Never a feature or a last look at
   the study numbers.
8. DEVLOG Day 20, including three things you would do differently.

---

## WP12. Cut-list items and smaller code gaps (only after the above)

| # | Item | How |
|---|---|---|
| 12.1 | `vc-cli ble` (Mac central; plan §3.1 and Lite **L2**) — **not built** | subcommand in `tools/vc-cli` with `btleplug` + `tokio`: scan for 0x1818, subscribe to 2A37/2A63/2A5B and TELEM, feed `Engine::ingest(t_ns, source, bytes)`, tick every 250 ms, play phrases with `afplay` from `build/phrases`, write the same ledger; label latency "Mac-path". Needed only if the iPhone path fails (K1) |
| 12.2 | FIT replay | FIT parser crate → byte frames or direct inputs → `vc-harness`. Cut #2 |
| 12.3 | Intervals.icu fetch | std crate calling the API for planned workouts → `WorkoutPlan`. Cut #5 |
| 12.4 | Post-ride feedback buttons | "Too chatty" → cooldown +10 s, "Missed target changes" → lead 5→8 s, as `tuner_change` records (Coach only). Cut #6 |
| 12.5 | Decoupling cue | a P3 directive in `directive.rs`, phrase, lint, catalogue, tests. Cut #8 |
| 12.6 | Waterworth W′bal | exists (`WbalWaterworth`); expose a switch if wanted |
| 12.7 | Group profile | exists; add a picker to demo muting |
| 12.8 | App tests | Kotlin: `NotifyQueue` (one in flight; counters reset on connect **and LOAD**; STAMP follows a sensor item). Swift: `badgeText`, `StudyConfig.seed`, the PENDING guard |
| 12.9 | Tuner visible on the phone | one outcomes-list line when the ledger drain returns a `tuner_change` |
| 12.10 | Real rider settings | CP, W′, HR max, profile as settings instead of the hard-coded 250 W / 20 kJ / 190 bpm / Solo |
| 12.11 | Fuzz and bench evidence | `scripts/fuzz-all.sh 600`; `cargo bench -p vc-efficacy`; paste the numbers (1.4.3) |

---

## WP13. Reconciling the repository with the plan's specifics

Record these once in `ENGINEERING_NOTES.md` so a reader of the plan is not surprised.

**Wire format — measured from the code (message length includes the 2-byte `ver | op` header):**

| Message | Op | Real length | Plan (§3.5 payload B) | Difference |
|---|---|---|---|---|
| STAMP | 0x01 | 15 B: `seq u16, channel u8, char_seq u16, t0 u64` | 15 payload (`seq u32 …` = 17 B on the wire) | `seq` is 16-bit |
| PONG | 0x02 | 20 B | 18 payload | none (the plan's column excludes the header) |
| STATUS | 0x03 | 12 B: `seq u16, scenario, state, faults u16, q_drops u16, notify_errs u16` | 6 | richer in the code |
| RIDER_INPUT | 0x04 | 14 B | 12 | none (fixed by the plan) |
| PING | 0x10 | 12 B | 10 | none |
| LOAD | 0x20 | 9 B | 7 | none |
| START / STOP | 0x21 / 0x22 | 2 B | 0 | none |
| FAULT | 0x30 | 10 B: `kind, channel, dur_ms u32, param i16` | 4 (`dur_ms u16`) | 32-bit duration and a parameter |
| CLEAR | 0x31 | 3 B (a `kind` byte, 0xFF clears all) | 0 | one byte |
| RATE | **0x40** | 4 B: `channel, hz_x10` | **0x32**, `hz_x4`, 2 | opcode and unit |
| CUE_PLAYED | 0x50 | 15 B | 13 | none (fixed by the plan) |

**Other differences**

| Plan | Repository | Action |
|---|---|---|
| §3.4 `Engine::new(cfg)` | `Engine::new(cfg, plan)` plus `Engine::forScenario(cfg, scenarioId)` on the phones | note |
| §3.4 `vc-cli … --mode replay`, `raw.bin` | `vc-cli replay file.vcraw` | note |
| §3.3 `onset_source` loopback / output_latency / virtual | emits `scheduled`, `loopback`, `virtual` (`output_latency` reserved) | note |
| §3.7 hold rule | implemented as specified (the asymmetry is the plan's) | WP1 |
| §7 `creep` scenario, 15–20 decision points | `creep` yields 4–6; `creep_pilot` is the pilot's scenario | WP3; note the deviation from the plan's wording |
| §12 "GoldenCheetah differential default" | integral is the default | corrected in `FACT_CHECK.md`, `WPRIME_GC.md` |
| §8 D4 "? grade" | needs `grade_demo` (scenario 10) | done |
| M7 labels | CI greps a different third phrase | WP9.7 |
| Day 8 H5 "confirm zero-copy bytes" | not zero-copy: `Data` / `ByteArray`, one copy | done (F5) |
| VcCore targets iOS 17; the plan's risk register says iOS 27 on the phone | | build and test on the installed OS; note API differences |

---

## Decision rules to keep nearby (from the plan)

- **K1** G-11 fails → Day 12 mornings are fixes; still failing at end of Day 13 → **L1**.
- **K2** G-14 not green → drop friend #3, feedback buttons, GoldenCheetah cross-check; pilot is you alone.
- **K3** human mode not working by end of Day 16 → pilot in L1, deviation recorded.
- **K4** no sessions after Day 18.
- **L2** Mac central + `afplay` (needs WP12.1).
- Cut in order: IPS/SNIPS, FIT replay, Waterworth, Group, Intervals.icu, feedback buttons, benches, decoupling, friends' sessions (keep
  yours), GoldenCheetah cross-check, phone badge (keep the HTML report), iPhone report-card screen.
- **Never cut:** safety gate, S9, E3, golden logs (S5), latency with its sync bound (S2/S3), fuzzing (S4), **E1**, E2, the Study freeze and
  P1-never-randomised tests (M3/M4), ledger schema, consent and pre-registration (M1/M8).

## Final definition of done

Tick every line, with evidence in DEVLOG or the named document:

- [ ] WP1 E1 passes (or option C stated everywhere) · [ ] WP2 both apps build from the repo, CI compiles them
- [ ] WP3 `check-protocol-hash.sh` prints `ok`, `STUDY.md` and the scenario agree · [ ] WP4 G-11 + v0.1.0 · [ ] WP5 G-14 + v0.2.0 (+ S4 logs)
- [ ] WP6 S2 and S3 with bounds; second ride on one connection matches · [ ] WP7 dry run passes the audit **and yields ≥ ~12 decision points**
- [ ] WP8 soak 1 PASS; sessions with consent (M8) · [ ] WP9 audit PASS, Results written, M6/M7 CI green
- [ ] WP10 fact-check final, licences verified, no stray `__`, scenes rehearsed twice, shots recorded
- [ ] WP11 video ≤ 3:00 unlisted, soak 2 PASS, CI green, fuzz clean, hygiene clean, branch merged, v1.0.0 released, message sent
