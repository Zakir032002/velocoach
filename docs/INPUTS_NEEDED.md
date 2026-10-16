# What I need from you before I finish everything

Goal: when I'm done, the only things left for you are hardware steps: plug in the iPhone and
the iQOO, run one script per phone, press Start, and record. To get there I need answers to the
questions below.

**How to reply:** each question has lettered options and my recommendation. A message like
`1A 2A 3A 4: Mac M2, macOS 26.1, Xcode 27.0 …` is enough. If a question doesn't matter to you,
skip it and I'll use the recommendation.

---

## Part A. Decisions that change what I build (most important)

**1. Scope for the submission.** You said you need the demo submitted fast.
- **A (recommended):** demo first. Ship the two-phone demo, the headless layer, the docs, the
  video script and the report from simulated data now. Mark the human pilot as "planned,
  pre-registered, not yet run". This is honest under the plan's K2 rule, and you can run the pilot later.
- B: the full plan, including the 9-session pilot with friends before submitting (2+ more days of sessions).
- C: pilot with you alone (3 sessions, about 1.5 h). The plan's K2/K4 fallback: a single-person feasibility run.

**2. E1 (the observer's self-test, currently failing on all 20 seeds).**
- **A (recommended):** apply the three fixes. Count only cues the rider can react to; a "reversed"
  verdict must hold for all 3 seconds; the measured smoothing lag is 1.5 s. Measured result: 18/20 seeds.
  I then re-run everything and update the docs.
- B: leave the observer as the plan wrote it and say "E1 open" everywhere, including the video.

**3. Pilot scenario** (matters only if 1B or 1C).
- **A (recommended):** `creep_pilot` with the slider starting at 70 %. Gives 17–20 decision points per session.
- B: the original `creep` (about 5 per session). STUDY.md would be rewritten to say so.

**4. Text-to-speech voice for the spoken cues.** The phrases have to be rendered to audio once,
on your Mac. Hugging Face, where Piper voices live, is blocked from my environment, so I can't render them here.
- **A (recommended):** Piper with the voice `en_US-lessac-medium`. The script downloads it, you run one
  command. I'll note the licence as "verify" unless you confirm it.
- B: macOS's built-in `say` voice. No download and zero setup, but the licence for a public demo is unclear.
- C: you name another Piper voice.

**5. May I merge to `main` and tag?** Work is on the branch `claude/velocoach-days-9-14`.
- **A (recommended):** I open a pull request; you click merge; you create the tags after your phone runs.
- B: I push straight to `main` (only if you explicitly say so).

**6. The Mac fallback (`vc-cli ble`, the plan's L2).** It lets the Mac replace the iPhone if the
iPhone build fails.
- **A (recommended):** build it. It's insurance for the demo deadline (about half a day of my work).
- B: skip it.

---

## Part B. Facts about your machines (so the build scripts just work)

7. **Mac:** chip (M1/M2/…), macOS version, Xcode version. Is Homebrew installed? Is Android Studio installed on the Mac?
8. **Apple signing:** the existing BleProbe project uses team `42H9VCQ78N` and bundle id `dev.zakir.velocoach`.
   - Shall I use team `42H9VCQ78N` and bundle id `dev.zakir.velocoach.coach` for the coach app? (yes/no)
   - Is it a free personal team? (Free profiles expire after 7 days and may refuse background modes.)
9. **iPhone:** model (plan says iPhone 12) and iOS version (BleProbe targets iOS 27.0; the new app will match).
10. **iQOO:** exact model, Android version, OriginOS version. Is USB debugging already on?
11. **Android tooling on the Mac:** installed or not:
    - the Android SDK
    - the NDK (version?)
    - Java 17
    - `cargo-ndk`

    If not, my setup script will install them via Homebrew/sdkmanager. Tell me if that's not OK.
12. **Rust on the Mac:** is `rustup` installed? (The scripts add the iOS/Android targets.)
13. **Earbuds:** model, for LATENCY.md. Optional.

---

## Part C. Facts for the documents and the message (I won't invent these)

14. **Aki's posts.** The URLs, or exact titles and dates, of the posts the code cites: "The Adaptive
    Architecture" (Oct 5), "Bridging…" (Sep 28), "The Psychology of the Soft-Pedal" (Sep 2), the
    Closed-Loop post. If you paste URLs I'll try to verify them at source; otherwise they stay "to verify".
15. **The trial instructions.** The exact deliverable the trial asked for, the deadline, and the channel
    (email / form / LinkedIn). The plan says the trial's own instructions override everything.
16. **Author line and links:** "Zakir Shaik" (as in the plan) OK? Your public GitHub repo URL. Will the
    repo be public at submission?
17. **Your open-source line for the message** (plan §9: "contributor to RisingWave, Apache KVRocks and
    Apache Fury"). Still accurate?
18. **Participants (only if 1B/1C):** how many friends are actually available, and on which days.
    Give no names, just counts.

---

## What I will build myself once you answer (no further input needed)

| # | Work | Result for you |
|---|---|---|
| 1 | **Android Gradle project** (settings, app module, JNA, signing for debug), plus a CI build job | `scripts/setup-android.sh` builds the Rust library and installs the rig app with one command (USB) |
| 2 | **iOS project** as an XcodeGen spec (`project.yml`) with your team/bundle id, Info.plist, background modes, `Cues` folder, package link; plus a CI compile job | `scripts/setup-ios.sh` builds the XCFramework, renders the phrases, generates the project and opens it. You press Run |
| 3 | **E1 fix** (per your answer to 2), golden ledger regenerated, docs updated, CI gating restored | E1 green in CI |
| 4 | **Compile fixes** for the Swift/Kotlin code, found by CI on GitHub's macOS and Linux runners | apps that build first time on your machine |
| 5 | **App unit tests** (Kotlin NotifyQueue counters; Swift badge text, study seed, hash guard) | in CI |
| 6 | **Fuzz evidence (S4) and E5 benchmark from CI.** A manual "long fuzz" workflow: 10 min per decoder, logs uploaded as artifacts | numbers for README/ENGINEERING_NOTES without your Mac |
| 7 | **`vc-cli ble` Mac fallback** (if 6A) | insurance for the demo |
| 8 | **Results collector:** `scripts/collect-results.sh` reads the files you AirDrop from the phones (captures, ledgers, soak CSV, loopback videos) and fills the `__` blanks in README/LATENCY/STUDY/ENGINEERING_NOTES automatically | after a phone run you drop files in a folder and run one command |
| 9 | **Demo-from-simulation report:** `docs/study/results.html`, with figures regenerated by CI (M6), built from a clearly labelled simulated dataset if the pilot is postponed (1A) | the D7 scene exists without participants |
| 10 | **Docs finalised:** every blank I can fill from code or CI is filled. What remains needs your phones and is listed on one page (`docs/PHONE_CHECKLIST.md`) | one checklist |
| 11 | **Video kit:** shot list, voiceover text with numbers filled, SRT subtitles generated from the script | you record and edit |

After that, **your part** will be:
1. Run `scripts/setup-ios.sh` and `scripts/setup-android.sh` with the phones plugged in.
2. Follow `docs/PHONE_CHECKLIST.md`: the two-phone run, the loopback clip, the soak, and the pilot if chosen.
3. AirDrop the files and run `scripts/collect-results.sh`.
4. Record the video, upload it, send the message.

Things I can't do for you: anything that touches the phones, signing in with your Apple ID, recording
video, and sending the message.
