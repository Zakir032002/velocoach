# Phone checklist

This is everything left to do by hand: set up the two phones, record the demo and collect the
numbers. The scripts do everything else. Each step says what you should see. If you don't see
it, the step's "if not" line says what to do.

All sensor data is **simulated**. The iQOO plays the rider, power meter, heart-rate strap and
cadence sensor over real Bluetooth. Back to the [README](../README.md).

**New to the project?** Read [EXPLAINED.md](EXPLAINED.md) first.

## 0. Once, on the MacBook (about 45 min, mostly downloads)

| # | Do | You should see |
|---|---|---|
| 0.1 | `git clone https://github.com/Zakir032002/velocoach && cd velocoach && git checkout claude/velocoach-days-9-14` (or `main` after the merge) | the repo |
| 0.2 | `scripts/check-machine.sh` | `ok` for Homebrew, Xcode and rustup. Install anything marked `MANUAL` yourself: Xcode from the App Store; [brew.sh](https://brew.sh); [rustup.rs](https://rustup.rs) |
| 0.3 | `scripts/setup-android.sh --no-device` | `OK: built …app-debug.apk`. Every install command is printed before it runs. Re-running is safe. *If Gradle runs out of memory on the 8 GB Mac:* close Xcode and re-run. *If you don't want to build:* skip this; step 1.3 can use CI's APK |
| 0.4 | `scripts/setup-ios.sh` | Xcode opens `VeloCoach.xcodeproj` |
| 0.5 | Xcode > Settings > Accounts > add the Apple ID of team 42H9VCQ78N | the team appears as "Personal Team" |

## 1. iQOO (the rig)

| # | Do | You should see |
|---|---|---|
| 1.1 | Settings > About phone > Software version: tap 7 times. Then Settings > System > Developer options: **USB debugging ON** and **Install via USB ON** (OriginOS asks for a vivo account for the second one) | "You are now a developer" |
| 1.2 | Connect with a data cable and unlock. Tap **Allow** on "Allow USB debugging?" and tick "Always allow" | — |
| 1.3 | `scripts/setup-android.sh`, or `scripts/setup-android.sh --from-ci` (needs `brew install gh && gh auth login`) | `OK: VeloCoach Rig is installed and open`. The device model and Android/OriginOS version are saved to `build/machine/android-device.txt` |
| 1.4 | On the phone: Settings > Battery > Background power consumption management > VeloCoach Rig > **Allow** (the wording differs between OriginOS versions; look for "No restrictions") and turn Bluetooth on | — |
| 1.5 | Open VeloCoach Rig and keep it on screen. The screen stays on by itself | `services 4/4 · advertising true`; after the iPhone connects, `central …` and the subscribed channels |

## 2. iPhone 12 (the coach)

| # | Do | You should see |
|---|---|---|
| 2.1 | Connect, unlock, tap **Trust**. Settings > Privacy & Security > **Developer Mode ON** (the phone restarts) | — |
| 2.2 | In Xcode, choose the iPhone as the run destination and press **Run** (⌘R). Or run `scripts/setup-ios.sh --install` | the app opens on the phone. *If you see "untrusted developer":* Settings > General > VPN & Device Management > your Apple ID > Trust. *If signing rejects the background modes:* `scripts/setup-ios.sh --no-background` and keep the app in the foreground |
| 2.3 | Allow Bluetooth when asked. Put the earbuds in and tap **Volume check** | a spoken test cue |
| 2.4 | Free-team signing expires after 7 days: press Run again to renew it | — |

## 3. A ride (hardware-in-the-loop, the demo)

| # | Do | You should see |
|---|---|---|
| 3.1 | iPhone: Scenario `climb_surge`, Persona `default`, **In-process OFF**, Study mode OFF | `link: ready` once it finds the rig |
| 3.2 | Tap **Start** | the step counter, the quality line, live latency. Spoken cues, each followed by a verdict (✓ complied, – ignored, ? confounded, ↺ reversed) |
| 3.3 | Latency panel: note **T1→T2 p99** (BLE notify → core decision) and **T0→T3 p99 ± bound** | numbers with the clock-sync bound. Earbud output latency is **not measured**; don't quote one |
| 3.4 | After the ride: **Report card**. Then **Export ride**, or leave the files on the phone for step 5 | the report card, with the SIMULATED DATA banner |
| 3.5 | Human mode (E7): on the iQOO, switch on **I am the rider** and move the slider (it starts at 70 %). On the iPhone, switch on **Human rider (the iQOO slider)**, then Start | the verdicts follow your slider |

### Fallback L1: iPhone only, no radio

If the two phones won't link, switch **In-process simulator** ON. The simulator then runs
inside the iPhone through the same Engine path. The latency panel says
`IN-PROCESS (L1): no BLE`; record that as it is and don't present it as a BLE figure.

## 4. Latency and soak runs (the S rows of the status table)

| Run | How | Feeds |
|---|---|---|
| S1/S2: 15 min | scenario `dense`, HIL | T1→T2 p99, T0→T3 p99 ± bound |
| S6: 60 min ×2 | scenario `soak` (writes a soak CSV) | `vc-cli soak-check` in step 5 |
| S3 | Loopback screen, 20 clicks, with the iPhone mic next to an earbud | optional: only if you want a measured route figure. Otherwise the README keeps "not measured" |

## 5. Collect

`scripts/collect-results.sh` (the iPhone on USB), or `scripts/collect-results.sh --from DIR`
with the files you exported. It writes `results/<stamp>/SUMMARY.md`, which has the replay
check, the soak check, the audit and the report. Copy its numbers into `docs/LATENCY.md` and the
README status table, changing each row from *pending* to *measured-on-hardware*. Commit
`results/<stamp>/` (without `raw/*.vcraw` if it is large).

## 6. Demo scenes

Each feature has its own demo scene, with exact settings and ride times, in
[WHAT_WE_BUILT.md § 3](WHAT_WE_BUILT.md#3-the-demo-scenes-one-feature-each). Before making the
repository public, run `scripts/pre-public-check.sh`.

**Not now:** the human pilot. It is planned and pre-registered ([STUDY.md](../STUDY.md)) but has
not been run. A later run on yourself is labelled "n=1 author dry run (engineering feasibility,
not evidence of efficacy)", keeps its ledgers in `data/dry-run/`, and is excluded from any
study numbers (`vc-report --exclude-rider R00`).
