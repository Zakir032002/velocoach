# What is not done, and exactly how to do it

This document is measured against **the main plan** ("velocoach-rs — Final Plan, Days 5–20",
v-final, Oct 11 2026): its three layers, its day-by-day schedule, its success criteria
(S1–S10, E1–E8, M1–M8), its gates (G-11, G-14), its fallback rules (K1–K4, L1, L2), its cut
list and never-cut list, and its risk register. Every work package below says which part of
the plan it satisfies, what is missing today, the precise steps to finish it, how you know it
is finished, and what to do if it fails.

Companion documents: `PROJECT_GUIDE.md` (how everything works and links),
`PLAN_VS_REPO.md` (criterion table), `COMPLETION_GUIDE.md` (shorter checklist version),
`RUNBOOK.md` (session procedure), `DEMOS.md` (scenes and script).

---

## 0. Where the project stands against the plan

The plan has three layers; each starts only after the one below passes its gate.

| Plan layer | Plan's confidence | State in the repository |
|---|---|---|
| 1. Real-time core end to end over real BLE on two phones (v0.1.0, Day 11) | ~70 % | **Code complete, never run on hardware.** Rig and coach apps exist as source files; neither has been compiled; gate G-11 has not been attempted |
| 2. Headless observer, ledger, tuner, report (v0.2.0, Day 14) | ~85 % | **Code complete and green** (196 tests, golden logs, E6, E8, thumbv7em) **except E1, which fails.** v0.2.0 not tagged |
| 3. Pre-registered slider pilot (Days 15–18) + docs + video + ship (Days 19–20) | ~50–60 % | **Tooling and documents complete; zero data collected.** Everything that needs a person, a clock or a camera is open |

What "complete" means in this document: the work package is finished only when its
acceptance test is true and its evidence is written down (DEVLOG, LATENCY.md, STUDY.md).
The plan's rule is "no claim without its number, no number without its source".

### Critical path

```
WP1 E1 ─┐
        ├─► WP3 pre-registration ─► WP7 dry run ─► WP8 soak+sessions ─► WP9 analysis ─► WP11 ship
WP2 apps build ─► WP4 gate G-11 ─► WP6 latency (S2/S3) ─┘                         ▲
                       WP5 gate G-14 (needs WP1) ───────────────────────────────┘
WP10 docs/demos/recording runs in parallel with WP8–WP9 (soak hours) and finishes before WP11
```

Items WP1 and WP2 need no people and no hardware beyond your two phones; do them first.

### Honest estimate of the remaining effort

| Package | Hands-on time | Calendar constraint |
|---|---|---|
| WP1 E1 | 2–4 h | none |
| WP2 apps build | 3–6 h | Mac + Xcode, Android Studio |
| WP3 pre-registration | 0.5 h | after WP1 |
| WP4 gate G-11 | 3–4 h | iQOO + iPhone, earbuds |
| WP5 gate G-14 | 0.5 h | after WP1 |
| WP6 latency (S2, S3) | 3 h | camera, earbuds |
| WP7 dry run | 2 h | re-sign the app first |
| WP8 soak + sessions | 1 h soak (unattended) + 20 min × 9 sessions + set-up/export ≈ 5–6 h | friends' availability |
| WP9 analysis | 2–3 h | after the last session |
| WP10 docs + rehearsal + recording | 8–10 h | quiet room |
| WP11 ship | 6–7 h incl. 1 h soak | CI, upload processing |
| WP12 cut-list items | optional | only if time remains |

---

## WP1. Resolve E1 (the observer must recover a known rider)

**Plan reference:** Day 13 H5; success criterion **E1**; the never-cut list ("Observer +
ground-truth test"); Day 14 gate G-14 ("E1–E6, E8 and S4–S10 green"); README row.

**What is missing:** E1 fails on all 20 seeds (compliance 0.28–0.38 against 0.70 ± 0.08).
Until it passes, "it measures whether its cues work" is unproven, the v0.2.0 tag would be
false, and every compliance number in the pilot is uncalibrated.

**Why it fails (three causes, all shown by experiment; details in `EFFICACY.md`):**
1. The E1 test counts the cardiac-drift (P3) cue, which carries no phrasing arm; the persona
   never reacts to it, so it is "ignored/reversed" by construction.
2. The observer decides *reversed* from a single hold bin (`H = min` of three bins is ≤ −0.2Δ
   when any one bin is low) but *complied* only when all three bins are high. With a 10 W
   floor, −2 W is inside the power noise.
3. `SMOOTHING_LAG_MS` is 2000 but the measured lag is about 1500, so latency reads 0.45 s low.

**Decision first (write it in DEVLOG):** the plan's §3.7 spells the reversed rule as "first t_r
with H ≤ −0.2Δ\* (whichever threshold is hit first wins)". Changing it is a change to the
plan's own specification, so decide deliberately:

| Option | Do | Expected E1 |
|---|---|---|
| A (recommended) | all three changes below | about 18/20 seeds |
| B | changes 1 and 2 only | about 15/20 (latency off by ~0.45 s) |
| C | change nothing; E1 stays red | 0/20; then no document may call the observer validated, v0.2.0/v1.0.0 must say "E1 open" |

**Steps for option A**

1. `crates/vc-harness/tests/e1.rs`: inside `for o in &run.outcomes`, after the
   `if scored == CUES { break; }`, add
   ```rust
   let modelled = run.decisions.iter().find(|d| d.id == o.cue_id).is_some_and(|d| {
       matches!(d.arm, vc_efficacy::Arm::Direct | vc_efficacy::Arm::Autonomy)
   });
   if !modelled { continue; }
   ```
2. `crates/vc-efficacy/src/observer.rs`: add a method next to `hold`
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
3. Change `SMOOTHING_LAG_MS` to `1_500` and fix its doc comment ("measured about 1.5 s").
   Update the same fact wherever it is written: `docs/LATENCY.md` (definition of L_obs),
   `docs/EFFICACY.md` (math section), README "Limits", `docs/DEMOS.md` voiceover (0:35),
   `docs/ENGINEERING_NOTES.md` (observer paragraph).
4. Bump `OBSERVER_VERSION` in `crates/vc-efficacy/src/lib.rs` to `"vc-efficacy 0.3.0"`.
5. Regenerate the committed artefacts and look at the diff by eye (verdicts may change; nothing
   structural should):
   ```
   cargo run -q -p vc-cli -- sim --scenario creep --mode study --seed 11 --jsonl golden/creep_study_11.jsonl
   cargo run -q -p vc-report -- golden/creep_study_11.jsonl -o docs/examples/creep_study_11.html
   cargo run -q -p vc-cli -- golden --check          # the five text goldens should be unaffected
   ```
6. Run everything:
   ```
   cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   cargo test --release -p vc-harness --test e1 -- --nocapture
   cargo test --release -p vc-report --test e8 -- --nocapture      # must stay within 92–98 %
   ```
7. Look at the stragglers. In the experiment seeds 8 and 12 sat at 0.610 and 0.615, 0.01
   below the band. Either (a) accept a documented tolerance (+/- 0.10) as a deviation, or (b)
   look for a fourth cause (print per-verdict counts for seed 12: how many `Partial`,
   `Reversed`, `Confounded`; spontaneous 5 % responses are included in the persona and should be
   small). Do not tune constants until the number passes; understand the residual first.
8. Paste the printed 20-row table into `docs/EFFICACY.md` (replace the "OPEN" section), update the
   README row, `docs/RELEASE_NOTES.md`, and `docs/PLAN_VS_REPO.md`; delete `continue-on-error: true`
   from the E1 step of `.github/workflows/ci.yml`.

**Acceptance:** `cargo test --release -p vc-harness --test e1` passes; E2, E3, E4, E8 still pass;
CI green; DEVLOG records the decision and the numbers.

**Also check while you are there (plan: "a pre-trend counterfactual handles creep"):** on a
`creep_long` ride, look at the ledger's P2 cues only (`grep GREY_ZONE_DRIFT` in the JSONL). A cue the
rider ignores should read *ignored*, not *reversed*, because the creep is in the pre-trend. If it
reads reversed, the 2 W/s slope clamp or the 10-s fit window is mis-handling the creep; fix that
before the pilot, since `creep` is the pilot's scenario.

**If you choose C:** edit `README.md`, `EFFICACY.md`, the Results limitations and the video
voiceover so none says "validated"; state "E1 open" next to every compliance figure.

---

## WP2. Make both apps build from the repository

**Plan reference:** Days 9–10 (rig and coach apps), Day 8 (UniFFI shells), CI "Android/iOS"
jobs. **Prerequisite for everything on hardware.**

**What is missing:**
- No Xcode project for `apps/ios/VeloCoach` (only source files and `Info.plist`). The Day 10 project
  you made locally is not in the repository.
- No Gradle project for `apps/android` (sources and manifest only).
- The spoken-cue bank `apps/ios/VeloCoach/Cues/` is generated, not committed.
- The Day 15–17 additions to both apps have never been compiled.

### 2a. iPhone app (Xcode, on the Mac)

1. Native layer: `scripts/build-ios.sh` (needs `rustup`, Xcode command-line tools). It writes
   `apps/ios/VcCore/VcFFI.xcframework` and `Sources/VcCore/vc_ffi.swift`. The package targets **iOS 17**.
2. Xcode → File → New → Project → iOS App, product name `VeloCoach`, interface SwiftUI, language Swift,
   save inside `apps/ios/` so the folder is `apps/ios/VeloCoach/` (choose "Replace" for the existing
   folder only if Xcode offers; otherwise create elsewhere and move the `.xcodeproj` in).
3. Delete the template's `ContentView.swift` and `VeloCoachApp.swift` (the `@main` is in `RideView.swift`).
4. Add to the target (Add Files, "Create folder references" NOT needed for Swift files):
   `AudioCuePlayer.swift`, `BLECentral.swift`, `Clock.swift`, `CoachModel.swift`, `LoopbackView.swift`,
   `RideView.swift`, `StudyConfig.swift`, `Uuids.swift`.
5. Add the local package: File → Add Package Dependencies → Add Local… → `apps/ios/VcCore`; link
   `VcCore` to the target.
6. Info: use the repository's `Info.plist` (Target → Build Settings → *Info.plist File* =
   `VeloCoach/Info.plist`, and turn off *Generate Info.plist File*), or copy its keys: Bluetooth usage
   string, `UIFileSharingEnabled`, `LSSupportsOpeningDocumentsInPlace`, background modes
   `bluetooth-central` and `audio`. Capabilities tab: Background Modes → *Uses Bluetooth LE accessories*
   and *Audio, AirPlay, and Picture in Picture* (a free team may refuse background modes; if so, record it
   in DEVLOG and keep the screen on during runs, see WP8).
7. Signing: your personal team, a unique bundle id (for example `dev.velocoach.coach`). Deployment
   target iOS 17.
8. Cue bank: install Piper and a voice (the script defaults to `voices/en_US-lessac-medium.onnx` + `.json`),
   then `scripts/render-phrases.sh`. This writes `apps/ios/VeloCoach/Cues/` (CAF files + `manifest.json`).
   Add `Cues` to the target as a **folder reference** (blue folder), so it lands in the bundle as `Cues`.
   **Before publishing, check the voice's licence** (WP10). Do not commit the audio until you have.
9. Build to the iPhone (Product → Run). Expected log on launch: `vc: N cue buffers loaded`.
   Likely compile errors, in order: a generated-binding label differs; a `String(format:)` argument type
   (`%u` ↔ `UInt32`, `%ld` ↔ `Int`, `%llu` ↔ `UInt64`); `audio.play` now returns a tuple.
10. Commit the project (`.xcodeproj`, not `xcuserdata`) so CI can build it, and add a CI step:
    `xcodebuild -project apps/ios/VeloCoach.xcodeproj -scheme VeloCoach -destination 'generic/platform=iOS Simulator' build`.

### 2b. Rig app (Android Studio)

1. Native layer: set `ANDROID_NDK_HOME` (NDK r27c is what CI uses), `cargo install cargo-ndk`, then
   `scripts/build-android.sh`. It writes `app/src/main/jniLibs/arm64-v8a/libvc_ffi.so` and the Kotlin
   bindings under `app/src/main/java/dev/velocoach/ffi/` (package fixed by `crates/vc-ffi/uniffi.toml`).
2. Add the Gradle project files (none exist). A working skeleton (bump versions to what your Android
   Studio offers):
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
   dependencies {
       implementation("net.java.dev.jna:jna:5.14.0@aar")   // UniFFI's Kotlin bindings load the .so through JNA
   }
   ```
   (`minSdk 31` because `BLUETOOTH_ADVERTISE` appeared in Android 12; the manifest already declares the
   Android 14 `FOREGROUND_SERVICE_CONNECTED_DEVICE` permission and service type.) Add `gradle.properties`
   with `android.useAndroidX=true` if Studio asks.
3. Build and install on the iQOO (`./gradlew :app:installDebug` or Run). Grant Bluetooth and notification
   permissions when asked. Expected on screen: `services 4/4 · advertising true` within a second.
4. Check with nRF Connect (Day 9 H7): four services, right Feature values; write LOAD + START by hand and
   see 60/240/60 notifications per minute with 0 dropped.
5. Commit the Gradle files and add a CI step `./gradlew :app:assembleDebug` to the android job (it can run
   after `scripts/build-android.sh`).
6. **Vivo settings (plan risk "Vivo kills the FGS", likelihood medium, impact high):** Settings →
   Battery → this app → *No restrictions* (wording varies); i Manager → App manager → Autostart → allow;
   Recents → lock the app; keep it in front with the screen on. Write the menu names you used in DEVLOG
   and `ENGINEERING_NOTES.md` ("Vivo: battery __, autostart __, recents lock __").

### 2c. Smoke test (10 minutes)

Rig in front → iPhone shows `link: ready` → pick `climb_surge`, Dana → Start. Pass: the latency panel
fills (`clock θ … ±… µs`, `T0→T1`, `T1→T2`), `T0>T1` stays 0, `stamps` climbs, a cue is heard with an earcon first, a
badge appears after the audio. Stop → report card shows; Export ride offers three files (capture, ledger,
and soak CSV only for the `soak` scenario).

**Acceptance:** both apps install from a clean checkout following only these steps; CI compiles both.
**Fallback:** if the Xcode/Gradle projects cannot be committed, record the exact local setup in
`ENGINEERING_NOTES.md` and keep the smoke test as the gate.

---

## WP3. Freeze the pre-registration (M1)

**Plan reference:** Day 15 H5 ("Commit STUDY.md (§7) = pre-registration, and put protocol_hash in the
app config"); criterion **M1**; the never-cut item "Consent and pre-registration if any human session
happens".

**What is missing:** the date in `STUDY.md`, the two deviation dates, the committed hash in
`StudyConfig.protocolHash` (currently `"PENDING"`).

**Steps** (do this after WP1, because the observer is part of what you are pre-registering)

1. Read `STUDY.md` once as a stranger would: question, design, allocation (sham 0.2 / direct 0.4 / autonomy
   0.4), outcomes, analysis, mandatory language, consent text. The protocol section must not be edited
   after the commit; fix typos now.
2. Replace `Date: [YYYY-MM-DD]` and the two `2026-10-__` dates in *Deviations* with today's date.
3. `git add STUDY.md && git commit -m "docs: STUDY.md pre-registration"`.
4. `git log -1 --format=%h -- STUDY.md` → put that value in `apps/ios/VeloCoach/StudyConfig.swift`
   (`protocolHash`). Commit this **separately** (the hash is of STUDY.md's commit, so this commit must not
   touch STUDY.md).
5. `scripts/check-protocol-hash.sh` → `ok: <hash>`. Run it before every build and every session.
6. If STUDY.md ever changes again (new deviation), repeat steps 3–5 and note the new hash in DEVLOG; sessions
   before and after are then reported separately.

**Acceptance:** the script prints `ok`; the app shows the same hash under Study settings; the first Study
ledger line carries it (`"protocol_hash"`).

---

## WP4. Gate G-11: the two-phone run, S1, replay, Vivo, v0.1.0

**Plan reference:** Day 11 (H1–H6), **G-11**, criterion **S1**, S5 groundwork, K1/L1.

**Gate conditions (plan):** ≥ 15 min without an unrecovered disconnect · earcon-first cues heard ·
identical replay · CI green · T1→T2 measured.

**Steps**

1. Set-up: both phones on chargers, Vivo settings made (WP2b.6), earbuds paired to the iPhone, calibration
   clip played once (the app prompts; side buttons set `.voicePrompt` volume while it speaks).
2. 20-minute run: scenario `climb_surge`, Dana, Coach mode (Study off), Start. Do not touch either phone.
   Provoke recovery once on purpose: switch the rig's Bluetooth off and on (the rig re-advertises and the
   iPhone must reconnect by itself within about 20 s).
3. Read **S1** before you stop: T1→T2 line on the panel (`p99` in µs). Target p99 < 1 ms. Do one run with
   capture on (the app always captures) and note the p99; if you want the capture-off number, build once with
   `engine.setCapture(on: false)`. Write both in DEVLOG Day 11 and in `LATENCY.md` S1 (iPhone 12, n = …).
4. Stop → Export ride → AirDrop to the Mac:
   ```
   mkdir -p runs/g11 && mv ~/Downloads/ride-*.vcraw ~/Downloads/ledger-*.jsonl runs/g11/
   cargo run -q -p vc-cli -- replay runs/g11/ride-*.vcraw       # must print: identical
   ```
   `different` means the Engine has a hidden input (clock read, iteration order); fix the Engine, never
   the capture.
5. Vivo test: 30 minutes, unplugged, screen on, rig in front. Pass: service alive, CPM sent ≈ 6000. Then
   2 minutes screen-off for the record (does power go Stale and when).
6. Tag:
   ```
   git tag -a v0.1.0 -m "G-11: iQOO -> BLE -> Rust on iPhone 12 -> spoken cue. Replay identical. T1->T2 p99 __ us."
   ```
7. DEVLOG Day 11 filled (template exists).

**Fallbacks (plan):** **K1** — if the gate fails, Day 12's mornings become a fix block; if it still fails
at the end of Day 13, switch to **L1**: run the `Simulator` in-process on the iPhone with an on-screen
slider and label every latency "in-process". The observer, ledger and study are unchanged. **L2** (Mac
central + `afplay`) is the second fallback and needs `vc-cli ble` (WP12.1).

**Acceptance:** all five gate conditions true and recorded; tag created.

---

## WP5. Gate G-14: tag v0.2.0

**Plan reference:** Day 14 H7; criteria E1–E6, E8, S4–S10; K2.

**Missing:** the tag, and the evidence lines that make the tag message true.

**Steps**

1. After WP1: `cargo test --workspace --all-features`, `cargo test --release -p vc-harness --test e1 -p vc-report --test e8`,
   `cargo run -q -p vc-cli -- golden --check`, and CI green on both operating systems (E6 is a CI step).
2. E5 (observer speed): `cargo bench -p vc-efficacy -- observer`; note µs per update. Allocation: it is
   asserted by `cargo test -p vc-efficacy --test alloc` and `cargo test -p vc-core --test alloc` (S10).
3. S4: run `scripts/fuzz-all.sh 60` (smoke) and keep the logs; the plan wants ≥ 10 minutes per decoder in
   total, so either point to Day 7's logs (DEVLOG Day 3 lists the execution counts) or run
   `scripts/fuzz-all.sh 600` once on a machine with nightly + cargo-fuzz.
4. Tag with real numbers:
   ```
   git tag -a v0.2.0 -m "G-14: headless closed loop. E1 20/20 seeds, E8 coverage 190/200, observer __ ns, 0 allocs."
   ```
   Do not write "E1 20/20" unless it is true. **K2:** if the gate is not green, the plan drops friend #3, the
   feedback buttons and the GoldenCheetah cross-check (the last is already done) and limits the pilot to you.

**Acceptance:** tag exists; DEVLOG Day 14 line "E1 __/20 · E2 6/6 …" completed.

---

## WP6. Day 15: clock sync check, d_route loopback, S2 run

**Plan reference:** Day 15 H1–H4; criteria **S2**, **S3**; §3.8 latency definitions; risk "Bluetooth route
delay is jittery"; risk "`.dataPlayedBack` fires early".

**Missing:** every measurement. Code, scripts and tables are ready.

### 6a. Clock sync check (15 min)
Start any ride. Healthy: `rtt_min` a few ms; bound `±rtt_min/2` well under 15 ms (M5 needs the **95th
percentile** under ±15 ms); `T0>T1` = 0; `lost` small. If `T0>T1` climbs, θ is wrong: confirm the rig stamps
`b` in the binder callback (it does in `GattRig`), that `reset_link` runs on each connect, and ping when the
queue is quiet. Write the numbers in DEVLOG Day 15.

### 6b. d_route by loopback (about 1 hour)
Equipment: iQOO as camera on a stand, the earbuds, tape.
1. iPhone: *Loopback* button. Full brightness, auto-brightness off, Do Not Disturb.
2. iQOO camera: pick the highest frame rate that still records normal-speed sound. Test with a 2-second clip
   (slow-motion modes often drop audio). 120 fps ≈ 8 ms resolution, 60 fps ≈ 17 ms.
3. **Earbuds run:** earbuds connected to the iPhone; press one earbud to the iQOO's bottom microphone; start
   recording; tap *Start (20 clicks)*; wait for `20 / 20`; stop. **Speaker run:** disconnect the earbuds; iPhone
   speaker facing the same mic position; repeat.
4. Copy both videos to `runs/loopback/` and run
   ```
   python3 tools/loopback.py runs/loopback/earbuds.mp4
   python3 tools/loopback.py runs/loopback/speaker.mp4
   ```
   (needs `ffmpeg`, `ffprobe`, `numpy`). Expect 20 pairs each. If the earbuds' deltas swing by more than
   ±50 ms, say so; the honest report is median ± IQR.
5. `d_route(earbuds) = median(earbuds) − median(speaker) + outputLatency(speaker)`. Read `outputLatency` from
   the Xcode console line `vc: route [...] outputLatency N ms` for each route; its estimate for the earbuds is
   `T4_est`, your d_route is `T4_meas`. The gap between them is one of the project's interesting numbers.
6. Enter the median in the app's *Route delay (ms)* field. From now on onsets are recorded as `loopback`.
7. `.dataPlayedBack` cross-check: after a ride, the panel's *skew flags* counts cues where callback minus
   (T3 + duration + d_route) exceeds ±50 ms. Speaker route should be 0; note the earbuds' count. Do **not**
   subtract `outputLatency` from `.dataPlayedBack` (plan decision 6).
8. Fill `LATENCY.md` S3, the d_route method (fps, resolution), and the agreement section.

### 6c. S2: sensor send to audio scheduled (15+ minutes)
Scenario `dense` (a 15 s interval and a 15 s recovery repeated 30 times: about 60 cues, mostly P1),
Dana, Coach, earbuds on, route delay entered, both phones charging. Read the panel at the end:
`T0→T3 p99 … ± bound`, `T0→T1`, `T1→T3`, `stamps/lost`. Target p99 < 150 ms **with the bound shown**.
Export into `runs/s2-15min/`. With ~60 cues p99 is essentially the maximum — write that sentence in
`LATENCY.md`, and give the per-packet T0→T1 distribution (thousands of samples) beside it.

**Acceptance:** `LATENCY.md` S2/S3 tables contain numbers with their bounds; `T0>T1` explained or zero.
**If S2 > 150 ms:** the plan reports, it does not hide: show where the time went (T0→T1 is the radio; the 50 ms
lead is deliberate). Tuning the connection interval is allowed (it is not a decision-path change).

---

## WP7. Day 16: re-sign, badges, human mode, dry run, LATENCY.md

**Plan reference:** Day 16 H1–H7; **K3** (human mode not working → L1); risk "Free profile expires mid-study";
criterion M5 groundwork.

1. **Re-sign** (plan rule: whenever fewer than 4 days remain): plug the iPhone in, select it, Product → Run;
   check Settings → General → VPN & Device Management for the new expiry; write it in DEVLOG. Run
   `scripts/check-protocol-hash.sh` first.
2. **Badges test:** `climb_surge`, Dana, Coach: a badge after each scored cue; ✓ for the W′ cue when Dana complies;
   no `? grade` on flat sections; the report card at Stop matches the badges you saw.
3. **Human mode check (10 min):** rig: *I am the rider* on, slider 60 %. iPhone: `creep`, Human rider on. Power ≈ 150 W
   (0.6 × 250) in the warm-up and rises slowly; move the slider down 10 % and power falls ≈ 25 W within a fraction of
   a second; when a grey-zone cue plays, ease off and read ✓ with your L_onset.
4. **The uncounted dry run, rider R00** — exactly like a counted session (see `RUNBOOK.md`): hash check, rig
   *I am the rider*, iPhone Study on, **R00, session 1 (seed 1)**, `creep`, Human rider on; read `policy_hash` off the
   screen and write it down; Start; phone face down; 20 minutes; Stop; Export; then
   ```
   mkdir -p runs/study/R00 && mv ~/Downloads/ledger-*.jsonl ~/Downloads/ride-*.vcraw runs/study/R00/
   cargo run -q -p vc-cli -- replay runs/study/R00/ride-*.vcraw
   cargo run -q -p vc-report -- runs/study/R00/*.jsonl -o runs/study/R00/report.html --audit
   ```
   Pass: replay `identical`; M3 0 P1-with-arm; M4 one `policy_hash`; M5 ≥ 90 % not Invalid and p95 sync bound
   ≤ 15 ms; every Complied cue has an `l_onset_ms`; 15–20 decision points with about a fifth shams; the iQOO
   showed nothing but the slider; the volume was comfortable and every phrase understood first time.
5. If the protocol must change because of the dry run (for example the creep is too fast to counteract), change it
   *now*, record it in `STUDY.md` Deviations, recommit, update `protocolHash`, and dry-run again. Interface fixes
   (larger slider, clearer label) need no deviation.
6. Fill `LATENCY.md` completely: S1, S2, S3, clock numbers, three per-cue chains (fast, median, slow) from the dry
   run ledger (`t0_ns…t3_ns`), `.dataPlayedBack` agreement, limits.
7. **K3:** if human mode does not work by tonight, run the pilot in L1 (Simulator in-process on the iPhone with an
   on-screen slider). The observer, allocation and analysis are identical; only the slider moves, and that is a
   dated deviation in STUDY.md, with latency labelled "in-process".
8. DEVLOG Day 16 (template exists), including *K3: human mode working: yes/no*.

---

## WP8. Day 17: soak 1 (S6), then the counted sessions begin

**Plan reference:** Day 17 H1–H7; criteria **S6**, **M2–M5** per session, **M8**; "ENGINEERING_NOTES while it runs".

### 8a. Soak 1 (60 minutes, unattended)
1. Both phones on chargers; rig app in front, screen on; earbuds connected; route delay set.
2. iPhone: scenario `soak`, Dana, Coach, Start. The app sends four RATE changes at 1500/1560/2700/2760 s; the scenario
   has five scripted faults every 10 minutes (power dropout at 600 s, garbage at 1,200 s, disconnect at 1,800 s, heart-rate dropout at 2,400 s, coasting at 3,000 s). Do not touch either screen. If background
   modes were granted, lock the iPhone from 30:00 and log it; if refused, leave the screen on and log that instead.
3. Write `ENGINEERING_NOTES.md` blanks while it runs (fuzz counts from DEVLOG, Vivo findings, UniFFI bytes = Swift
   `Data` / Kotlin `ByteArray`, one copy of 4–20 bytes).
4. After: Export (capture, ledger, soak CSV) →
   ```
   mkdir -p runs/soak-1 && mv ~/Downloads/{soak,ride,ledger}-*.{csv,vcraw,jsonl} runs/soak-1/ 2>/dev/null
   cargo run -q -p vc-cli -- soak-check runs/soak-1/soak-*.csv      # S6: PASS, every fault recovered
   cargo run -q -p vc-cli -- replay runs/soak-1/ride-*.vcraw         # identical, an hour of real radio
   ```
   Note the rig's `dropped` counter and both batteries by hand (start/end) for Day 20's comparison; STAMP loss
   should be < 0.5 % (the earlier design's target).
5. If `soak-check` fails: open the capture first (replay reproduces the Engine's view), the soak CSV second. A fix is
   allowed only if decisions stay untouched (a Swift reconnect bug, a queue size); rerun the soak. The counted sessions
   wait for a passing soak.

### 8b. Counted sessions (R01 S1–S3, then R02 S1…)
Follow `RUNBOOK.md` verbatim (checklist, briefing card, SESSIONS.md entry). Per session:
`check-protocol-hash.sh` ok → Study on, correct rider/session (seed on screen = 1000 × rider + session) →
policy_hash equals SESSIONS.md → rig *I am the rider*, slider 60 % → Human rider on, `creep` → Start, phone face down,
researcher silent → Stop → Export → file into `runs/study/R0x/` → replay `identical` → `vc-report --audit` (M3–M5) →
log. At least 20 minutes between one person's sessions.

**Before R01 S1** the aborted-session rule and the optional publication line are already in `STUDY.md` Deviations
(WP3). Your own sessions: you are not blind to the arms; ride as a real rider would and do not look at results.

### 8c. Friend 1 (R02): consent (M8) and S1
Adult only; give the printed sheet (two copies), explain the three things in your own words (a test of software, stop
any time, deletion on request); the publication line is optional, default no; both sign and date; electronic record
holds only `CONSENT.md` lines (pseudonym, date, publication yes/no, sessions). Read the briefing card word for word;
answer questions during a session only with "I'll tell you everything after your last session" and log the question.

### 8d. End of day
```
cd runs/study && shasum -a 256 R0*/* > SHA256SUMS
zip -er ~/study-backup-day17.zip . -x '*.DS_Store'      # keep a second copy elsewhere
shasum -a 256 -c SHA256SUMS
cargo run -q -p vc-report -- runs/study/R0*/*.jsonl --exclude-rider R00 --audit --audit-only   # no outcome panels
```
M4 must show one `policy_hash`. If it shows two, stop and find out why before the next session.

**Freeze (plan rule):** from the first counted session to the last, no change that affects decisions, verdicts or the
ledger. Interface-only fixes go in DEVLOG; protocol changes in STUDY.md first.

**Acceptance (Day 17):** S6 PASS + replay identical; ENGINEERING_NOTES drafted; R01 S1–S3 and R02 S1 each replayed,
audited and logged; R02 consent on paper and in `CONSENT.md`; backups with checksums; DEVLOG Day 17.

---

## WP9. Day 18: remaining sessions, K4, analysis, Results

**Plan reference:** Day 18 H1–H7; **K4** (no sessions after Day 18); criteria **M2–M6**, **M7**.

1. Re-check the hash, the build and the signing expiry before H1.
2. Sessions: R02 S2, S3 (seeds 2002, 2003); R03 consent + S1–S3 (3001–3003). Debrief each participant after their last
   session (two phrasings, shams, what was measured), ask what they noticed and write it down without judgement; ask R02 not
   to discuss with R03 until evening. A noticed pattern is a limitation, not a reason to drop data.
3. **K4 outcomes** (table in `RUNBOOK.md`): a friend unavailable → two-person study, n shrinks to ≈ 90–120 and the Results
   say so; aborted session → keep data, no repeat; withdrawal → delete files and backups the same day, record "one
   participant withdrew", re-run the analysis; everything fails → single-person feasibility run. Never top up with extra sessions
   of your own.
4. Final backup (checksums + encrypted zip). From now on nothing is added to `runs/study/`.
5. **Audit and analysis:**
   ```
   cd runs/study && shasum -a 256 -c SHA256SUMS && cd -
   for f in runs/study/R0*/ride-*.vcraw; do cargo run -q -p vc-cli -- replay "$f"; done
   cargo run -q -p vc-report -- runs/study/R0*/*.jsonl --exclude-rider R00 -o runs/study/results.html --audit | tee runs/study/audit.txt
   ```
   All five audit lines must PASS before reading any result. If one fails, the Results say so and report the affected
   sessions with and without them (the pre-registration forbids quietly dropping them).
6. **Write the Results** in `STUDY.md` from `results.html` and `audit.txt`: n and people, excluded counts, data-quality lines,
   both primary outcomes with Wilson 95 % intervals and the pooled within-person difference with its bootstrap interval,
   secondary outcomes (L_onset median/IQR per arm, L_end, sham false-positive rate, Partial/Reversed rates, coincidental
   first moves), the per-person table, deviations (as recorded, dated), limitations, consent count. The command and the commit it ran at.
   Mandatory wording: "feasibility pilot", "slider proxy, not cycling", "n = … decision points from … people", "not powered to
   detect differences under ~0.2", "personas are assumptions". No p-values.
7. **M6:** copy the ledger JSONL (never the captures) of R01 and each friend who ticked the publication line into
   `data/study/`; `cargo run -q -p vc-report -- data/study/*.jsonl -o docs/study/results.html`; commit both. The guarded CI step
   then compares a regenerated report byte for byte. If a friend declined publication, say in STUDY.md that the published
   report covers a subset and give the full-study numbers beside it.
8. **M7:** the CI grep checks "feasibility pilot", "slider proxy", "personas are assumptions". The master plan's M7 lists
   "n-small" instead of the third phrase; either add `"n-small"` to the CI loop or leave as is, but make the documents contain
   both so either reading passes.
9. DEVLOG Day 18. (The GoldenCheetah cross-check is already done: `WPRIME_GC.md`.)

---

## WP10. Day 19: re-check, fill the documents, rehearse, record

**Plan reference:** Day 19 H1–H6; §8 demos and video; §12 fact-check ledger; risk "A new post introduces per-cue compliance".

1. **Re-check Aki's Knowledge Hub and domesly.com** (wording only; nothing in code, the study or its results changes). If a
   post since Oct 11 introduces per-cue compliance, the pitch becomes "an implementation of an idea you've written about, with
   an instrument to measure it"; cite it by title and date. Take the trial post/email's deliverable format and deadline as the
   rule over anything here.
2. **Fact-check ledger** (`docs/FACT_CHECK.md`) to its final state with today's date: verify at source Aki's parameter names/ranges
   and earcon table (cite post titles), Żak et al. 2023 (Sensors 23(8):3898) and Buchholtz & Burgess 2020 (S Afr J Sports Med
   32(1)). **Two corrections to the master plan's ledger already found:** GoldenCheetah's *default* W′bal form is the Skiba
   integral, not the differential (the plan's ledger says "differential default"); the differential is the alternative setting
   — the repo documents this in `WPRIME_GC.md`. And the plan said "WPrime.cpp not opened": it has been opened (SHA-256 recorded;
   add the commit hash from a clone: `git log -1 --format=%H -- src/Metrics/WPrime.cpp`).
3. **THIRD_PARTY.md:** read the Piper voice's `MODEL_CARD`; a trial demo for a company is a commercial context, and some voices are
   non-commercial. If restricted, say so in THIRD_PARTY and the README, or re-render with a permissive voice and re-run `render-phrases.sh`.
   Run `cargo deny check licenses` (or `cargo license`) to back the "all MIT/Apache" claim.
4. **Fill every blank** (`grep -rn '__' README.md docs STUDY.md`): `COMPLETION_GUIDE.md` Appendix A maps each to its source. README top
   screen must fit a laptop screen; every number links to where it was measured.
5. **Rehearse D1–D7 twice** (record both runs in the `DEMOS.md` rehearsal table). Headless numbers are already there: D2
   tuner_change at 2,520 s, D3 fault second 847 → interrupt at 1,083 s, D4 `grade_demo` → `? grade`, D5 dropout cue at 608.25 s.
   Phone-side: D1 (`climb_surge`, Dana, Coach; the badge shows L_obs which includes the smoothing lag — say so in the voiceover),
   D3 (HR redline field = 847), D5, D6 (human, `creep`; wait for the words, then move, or the first move is flagged coincidental).
6. **Record** pictures and voice separately (iPhone screen recording with Do Not Disturb; iQOO recorder for D6/D5; terminal 18 pt;
   hardware shot by daylight; voiceover in a soft-furnished room). Bluetooth audio does not appear in screen recordings: play D1,
   D3, D6 through the iPhone speaker and caption "A rider would hear this in open-ear headphones." Copy everything into
   `~/velocoach-video/raw/` named by segment.
7. Commit `DEMOS.md`; DEVLOG Day 19.

---

## WP11. Day 20: edit, final soak, CI, fuzz smoke, hygiene, tag, upload, message

**Plan reference:** Day 20 H1–H7; criteria S6 (second run), S4; the video spec and the ≤ 150-word message.

1. **Edit (≤ 3:00)**: lay the voiceover first (8 segments, half a second of air), then pictures; speed up waiting time 4–8× but never
   the moment itself; captions for every number and for the three mandatory phrases during 2:05–2:30; the speaker caption on
   D1/D3/D6; title card ≤ 2 s; end card with the repository URL. Export 1080p H.264. Write an SRT from the script (one cue per
   sentence, ≤ 2 lines of ~42 characters, timed to the final edit). Watch twice (sound, then muted) against the checklist in `DEMOS.md`.
   Upload **unlisted** as soon as it exports so processing finishes during the soak; upload the SRT and turn automatic captions off.
2. **Final 60-minute soak** on the build that ships (same `soak` scenario): same commands as WP8a with `runs/soak-2/`; put both soaks
   side by side in `ENGINEERING_NOTES.md` (S6, recovery times per fault, replay, STAMP loss, batteries). If it fails where soak 1
   passed, the diff since is small (demo tools): look there first; document honestly ("failed once on the final build: …; cause …"),
   do not rerun until green.
3. **CI:** every job green on both OSes (including the E1/E8 job and the M6/M7 steps). **Fuzz smoke:** `scripts/fuzz-all.sh 20`; put
   execution counts in `RELEASE_NOTES.md`. A crash blocks the tag (add the input to the corpus; fix in a decoder, never a decision).
4. **Hygiene check** (the repository is about to be public and linked to a founder):
   ```
   git status --short
   git ls-files | grep -E '^runs/|\.vcraw$|CONSENT|SESSIONS|\.mp4$' && echo "PRIVATE FILES TRACKED"
   git ls-files data/study
   grep -h -o '"rider_id":"[^"]*"' data/study/*.jsonl | sort -u
   git log --all --format='%an <%ae>' | sort -u
   ```
   plus `git grep -i '<friend name>'` locally for each friend (never write the names into a file). `LICENSE-MIT`/`LICENSE-APACHE` exist
   and `THIRD_PARTY.md` is linked from the README (both done).
5. **Tag and release** (only when E1, CI, soaks and the hygiene check are honest):
   ```
   git tag -a v1.0.0 -m "velocoach-rs v1.0.0: closed-loop voice coach with per-cue efficacy scoring. S1 T1->T2 p99 __ us; S2 T0->T3 p99 __ ms +/- __ ms; S6 soak x2; E1-E8 green; pilot n = __ (feasibility)."
   git push origin main v1.0.0
   gh release create v1.0.0 --title "velocoach-rs v1.0.0" --notes-file docs/RELEASE_NOTES.md docs/study/results.html
   ```
   Make the repository public; open the README in a private window; click every link; the video link must not be `__`.
6. **The message** (≤ 150 words; numbers from the README; the mandatory caveat in the same breath as the pilot; no location; Aki's
   name spelled right; nothing that grades his work). Two versions exist: the master plan's §9 (mentions your open-source
   contributions) and the Day 19–20 document's template. If the trial post named another channel or format, that wins. Do not
   call the observer validated unless E1 passes. Read it aloud once.
7. Buffer hour: broken link, failed tag or private file first; then a fuzz crash or soak failure; then small documentation fixes.
   Never a feature, a refactor, or a last look at the study numbers to "improve" them.
8. DEVLOG Day 20, including the three things you would do differently.

---

## WP12. Cut-list items and smaller code gaps (only after the above)

The plan cuts in this order; none blocks the never-cut list. Each is currently **not built** unless noted.

| # | Item | How, if you want it |
|---|---|---|
| 12.1 | `vc-cli ble` (Mac central via tokio + btleplug) — **Lite fallback L2**; also in the plan's architecture diagram | New subcommand in `tools/vc-cli` using `btleplug` + `tokio`: scan for service 0x1818, subscribe to 2A37/2A63/2A5B and TELEM, feed `vc_core::Engine::ingest(t_ns, source, bytes)`, tick at 250 ms, play phrases with `afplay` (render the WAVs from `build/phrases`), write the same ledger. Label all latency "Mac-path". Needed only if the iPhone path fails (K1) |
| 12.2 | FIT file replay | Add a FIT parser crate (std), turn records into `Source::Power/Hr/Csc` byte frames or direct `Engine` inputs, run through `vc-harness`. Cut #2 |
| 12.3 | Intervals.icu live fetch | std crate calling the Intervals.icu API for planned workouts → `WorkoutPlan`. Cut #5; the plan already moved it out |
| 12.4 | Post-ride feedback buttons ("Too chatty", "Missed target changes") | Two buttons in `RideView` mapping to the tuner's published feedback mappings (+10 s cooldown; lead 5→8 s); record as a `tuner_change` with rule `F1/F2`. Cut #6; only in Coach mode |
| 12.5 | Decoupling cue | A P3 directive on HR/power drift over a steady block; add to `directive.rs`, the phrase bank, lint, catalogue, tests. Cut #8 |
| 12.6 | Waterworth W′bal | **exists** (`WbalWaterworth`); expose a config switch if wanted. Cut #3 |
| 12.7 | Group profile | **exists**; add a picker to the phone if you want to demo muting |
| 12.8 | Criterion benches | `benches/observer.rs` **exists** |
| 12.9 | App-code tests | Kotlin: unit tests for `NotifyQueue` (one in flight, counter reset on `onConnected`, STAMP follows a sensor item); Swift: `badgeText`, `StudyConfig.seed`, the PENDING guard |
| 12.10 | Tuner visible on the phone | one line in the outcomes list when the ledger drain returns a `tuner_change` record |
| 12.11 | Real rider settings | settings for CP, W′, HR max, profile in `CoachModel.config()` instead of hard-coded 250 W/20 kJ/190 bpm/Solo |

---

## WP13. Reconciling the repository with the master plan's specifics

Small differences found when comparing the plan text to the code. None blocks anything; record each once in
`ENGINEERING_NOTES.md` ("decisions and deviations") so a reader of the plan is not surprised.

| Plan says | Repository does | Action |
|---|---|---|
| §3.5 opcode table: STAMP `seq u32`; PONG 18 B; STATUS 6 B; FAULT `kind, char_id, dur_ms u16` (4 B); RATE `hz_x4` | STAMP `seq u16`; PONG/STATUS/FAULT sizes differ (README table lists the real ones); FAULT carries `dur_ms u32` + `param i16`; RATE `hz_x10` | the plan itself says "keep the code's numbers if Day 2–3 assigned different ones"; add the actual table to ENGINEERING_NOTES |
| §3.4 `Engine::new(cfg)` | `Engine::new(cfg, plan)` plus `Engine::forScenario(cfg, scenarioId)` for the phones | note it |
| §3.4 `vc-cli … --mode replay`, `raw.bin` | replay is `vc-cli replay file.vcraw` | note it |
| §3.3 `onset_source` values loopback / output_latency / virtual | code emits `scheduled`, `loopback`, virtual for shams | note it (and LATENCY.md already explains) |
| §12 "GoldenCheetah differential default" | GoldenCheetah default is the Skiba integral | corrected in `FACT_CHECK.md` and `WPRIME_GC.md` |
| §8/Day 19 D4 "grade confounder: ? grade" | needs `grade_demo` (scenario 10) because `grade_confound`'s grade block ends before the cue | already added; mention in DEMOS (done) |
| M7 labels "slider proxy", "n-small", "feasibility pilot" | Day 18 CI step greps "personas are assumptions" instead of "n-small" | WP9.8 |
| VcCore targets iOS 17 | plan says iOS 27 on the iPhone (risk register) | build/test on the installed OS; note any API differences found |

---

## Decision rules to keep nearby (from the plan)

- **K1** gate G-11 fails → Day 12 mornings are fixes; still failing at end of Day 13 → **L1** (in-process simulator on the iPhone).
- **K2** G-14 not green → drop friend #3, feedback buttons, GoldenCheetah cross-check; pilot is you alone.
- **K3** human mode not working by end of Day 16 → pilot in L1, deviation recorded.
- **K4** no sessions after Day 18, absolutely.
- **L2** Mac central + `afplay` (needs WP12.1) if the phone audio path fails.
- Cut in order: IPS/SNIPS, FIT replay, Waterworth, Group, Intervals.icu, feedback buttons, benches, decoupling, friends' sessions
  (keep yours), GoldenCheetah cross-check, phone badge (keep the HTML report), iPhone report-card screen.
- **Never cut:** safety gate, Policy Engine tests (S9), P1 property test (E3), golden logs (S5), latency with its sync bound
  (S2/S3), fuzzing (S4), **observer + ground-truth test (E1)**, confounder tests (E2), study freeze and P1-never-randomised
  tests (M3/M4), ledger schema, consent and pre-registration (M1/M8).

## Final definition of done

Tick every line, with its evidence written in DEVLOG or the named document:

- [ ] WP1 E1 passes (or option C is stated everywhere)  · [ ] WP2 both apps build from the repo, CI compiles them
- [ ] WP3 `check-protocol-hash.sh` prints `ok`  · [ ] WP4 G-11 + v0.1.0  · [ ] WP5 G-14 + v0.2.0
- [ ] WP6 S2 and S3 measured with bounds  · [ ] WP7 dry run passes the audit; LATENCY.md has no blanks
- [ ] WP8 soak 1 PASS; sessions done with consent (M8)  · [ ] WP9 audit PASS, Results written, M6/M7 CI green
- [ ] WP10 fact-check final, licences verified, no stray `__`, scenes rehearsed twice, shots recorded
- [ ] WP11 video ≤ 3:00 uploaded unlisted, soak 2 PASS, CI green, fuzz clean, hygiene clean, v1.0.0 released, message sent
