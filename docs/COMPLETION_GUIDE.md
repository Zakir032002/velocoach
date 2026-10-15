# Completion guide

How to take the repository from "all code and documents applied" to a shipped
v1.0.0 with a pilot, a video and a message. Read [PLAN_VS_REPO.md](PLAN_VS_REPO.md)
first for what exists and what does not. Everything below is human work: phones,
people, a clock, a camera.

Conventions: `$` lines are commands; "done when" lines are the acceptance test for a
step; every `__` in a document has a row in Appendix A saying where its number comes
from. Do the phases in order. Phase 2 must come before any dated commit; Phase 6 onward
is frozen for decision-affecting code.

## Phase 0: integrate and build the apps (about half a day)

1. Review and merge the branch.
   ```
   $ git fetch origin && git checkout claude/velocoach-days-9-14
   $ cargo fmt --all -- --check
   $ cargo clippy --workspace --all-targets --all-features -- -D warnings
   $ cargo test --workspace --all-features            # 196 pass, 2 ignored (release-only)
   $ cargo run -q -p vc-cli -- golden --check
   ```
   Open a pull request into `main` yourself when you are happy (none was opened).
2. Rebuild both native layers (the FFI grew: clock, link stats, rider input, report card).
   ```
   $ scripts/build-ios.sh        # on the Mac: XCFramework + Swift bindings
   $ scripts/build-android.sh    # needs ANDROID_NDK_HOME and cargo-ndk
   ```
3. iPhone project (Xcode). The repo has no `.xcodeproj` for VeloCoach; the one you made on
   Day 10 must pick up two **new files**: add `apps/ios/VeloCoach/LoopbackView.swift` and
   `StudyConfig.swift` to the target. Minimum deployment target **iOS 17** (the `VcCore` package declares `.iOS(.v17)`; `ShareLink` needs 16).
   Build. Likely first-build errors, in order of probability:
   - a binding name differs slightly (`.t0t3`, `.t1t2`, `.hrRedline`, `lOnsetMs`,
     `negativeT0t1` were checked against generated bindings; anything else is new);
   - `String(format:)` argument types in `CoachModel.latencyLines` / `soakRow`
     (`%u` wants `UInt32`, `%ld` wants `Int`, `%llu` wants `UInt64`);
   - `audio.play` now returns a tuple `(durMs:, t3Ns:)?`.
4. Android project (Android Studio). Sync, build, run on the iQOO. New code: STAMP after
   every sensor notification (`NotifyQueue`), binder-time PING (`GattRig`, `SimDriver`),
   the "I am the rider" switch and slider (`MainActivity`, `SimDriver`, `RigService.driver`).
   The generated Kotlin package must be `dev.velocoach.ffi` (set in `crates/vc-ffi/uniffi.toml`).
5. Smoke test on the phones (10 minutes): rig on, iPhone connects, `climb_surge`, Dana,
   Start. You should see the latency panel fill in (`T0->T1`, `clock θ ... ±...`) and, after
   a cue, a badge. **Done when:** `T0>T1` stays 0 and a `stamps` count climbs.

## Phase 1: resolve E1 (before the pre-registration)

E1 is on the plan's never-cut list and fails today. Decide now: after the first counted
session the observer is frozen. Options and evidence: PLAN_VS_REPO.md section 4 and
EFFICACY.md. The recommended fix (Option A) is three small changes:

1. `crates/vc-harness/tests/e1.rs`: count only cues whose arm the persona models.
   ```rust
   let modelled = run.decisions.iter().find(|d| d.id == o.cue_id).is_some_and(|d| {
       matches!(d.arm, vc_efficacy::Arm::Direct | vc_efficacy::Arm::Autonomy)
   });
   if !modelled { continue; }     // right after the `if scored == CUES { break; }`
   ```
2. `crates/vc-efficacy/src/observer.rs`: reversal must be sustained over the same three
   bins that complied needs.
   ```rust
   /// The largest effect over bins k .. k+2. At or below -x for all three bins = sustained.
   fn hold_max(&self, ring: &P3Ring, k: u64) -> Option<f64> {
       let mut h = f64::NEG_INFINITY;
       for j in k..k + HOLD_BINS { h = h.max(self.effect(ring, j)?); }
       Some(h)
   }
   // in step(), replace the reversed test with:
   if self.hold_max(ctx.ring, k).is_some_and(|m| m <= -REVERSE_FRAC * self.delta) {
       return Some((Verdict::Reversed, Some(k)));
   }
   ```
3. `SMOOTHING_LAG_MS`: `2_000` -> `1_500` (and the comment above it, `docs/LATENCY.md`'s
   "~2.0 s", the README limit, EFFICACY.md's "about 2.0 s", and the 0:35 voiceover line
   "about two seconds of smoothing").
4. Bump `OBSERVER_VERSION` in `crates/vc-efficacy/src/lib.rs` (`"vc-efficacy 0.3.0"`), then:
   ```
   $ cargo run -q -p vc-cli -- sim --scenario creep --mode study --seed 11 --jsonl golden/creep_study_11.jsonl
   $ cargo run -q -p vc-report -- golden/creep_study_11.jsonl -o docs/examples/creep_study_11.html
   $ cargo test --workspace --all-features
   $ cargo test --release -p vc-harness --test e1 -- --nocapture
   $ cargo test --release -p vc-report --test e8 -- --nocapture
   ```
   Review the golden ledger diff by eye (verdicts change, nothing else should).
5. Re-run E2/E3/E8; paste the new E1 table into `docs/EFFICACY.md` (replace the "OPEN"
   section), change the README row and the RELEASE_NOTES line, and remove
   `continue-on-error: true` from the E1 step in `.github/workflows/ci.yml`.

Experiments gave 18/20 seeds for this; the two stragglers are at 0.610 and 0.615 against a
0.62 floor. If they stay out, either widen the tolerance in the documents (a decision: write
it down) or find the next cause. **Done when:** E1 passes, or the decision to proceed
without it is recorded in STUDY.md Deviations and the Results limitations.

## Phase 2: freeze the pre-registration (M1)

1. Edit `STUDY.md` once: replace `Date: [YYYY-MM-DD]`, and the two Deviation dates
   (`2026-10-__`). Leave the protocol section alone afterwards.
2. ```
   $ git add STUDY.md && git commit -m "docs: STUDY.md pre-registration"
   $ git log -1 --format=%h -- STUDY.md          # e.g. 3f9c2a1
   ```
3. Put that hash in `apps/ios/VeloCoach/StudyConfig.swift` (`protocolHash`, currently
   `"PENDING"`; the app refuses counted sessions while it is `PENDING`). Commit that
   *separately* (the hash is of STUDY.md's commit, so this commit must not touch STUDY.md).
4. `$ scripts/check-protocol-hash.sh` must print `ok: <hash>`. Run it before **every** build
   and every session.
5. If you change STUDY.md later (a Deviation), the hash changes: update `StudyConfig`,
   rebuild, re-check.

The documents sequence this as: STUDY.md commit on Day 15, deviations added on Day 17.
This branch already contains the deviation text, so one commit before the dry run is
enough; the deviations' dates are then the date you commit.

## Phase 3: gate G-11 (Day 11 work, on the phones)

Prerequisites: both phones charging; Vivo settings from Day 9 (battery "no restrictions",
autostart on, app locked in recents, rig app in front with screen on).

1. 20-minute run, any scenario with cues (`climb_surge`), earbuds in. Require: 15+ minutes
   with no unrecovered disconnect; earcon before every P1/P2 phrase.
2. S1: read the T1->T2 line on the panel (`capture off` first, then `on`). Target p99 < 1 ms.
   Write it in DEVLOG Day 11 and in `docs/LATENCY.md` S1.
3. Capture and replay. After Stop, "Export ride" (ShareLink) -> AirDrop to the Mac:
   ```
   $ mkdir -p runs/g11 && mv ~/Downloads/ride-*.vcraw ~/Downloads/ledger-*.jsonl runs/g11/
   $ cargo run -q -p vc-cli -- replay runs/g11/ride-*.vcraw       # must print: identical
   ```
   If it does not print `identical`, the Engine has a hidden input: fix the Engine, not the
   capture.
4. Vivo: 30 minutes, unplugged, screen on, service alive, CPM counter near 6000; then 2 minutes
   screen off for the record. DEVLOG Day 11.
5. Tag:
   ```
   $ git tag -a v0.1.0 -m "G-11: iQOO -> BLE -> Rust on iPhone 12 -> spoken cue. Replay identical. T1->T2 p99 __ us."
   ```
   If the gate fails: K1 (Day 12 mornings become a fix block; if still failing after Day 13,
   Lite path L1).

## Phase 4: gate G-14 tag (headless, no phones)

After Phase 1 and a green CI:
```
$ cargo test --release -p vc-harness --test e1 -p vc-report --test e8
$ git tag -a v0.2.0 -m "G-14: headless closed loop. E1 20/20 seeds, E8 coverage 190/200, observer __ ns, 0 allocs."
$ cargo bench -p vc-efficacy -- observer             # note the observer's microseconds (E5)
```
Fill the E1 count honestly (the tag message in the documents says `E1 __/20`).

## Phase 5: Day 15, latency (clock, S2, d_route)

1. Clock: with the app connected, watch `clock θ ... ±...`. Healthy: `rtt_min` a few ms, bound
   under ±15 ms, `T0>T1` = 0. If PONGs wait in the rig queue, ping when the link is quiet.
2. d_route loopback (about an hour). Needs the iQOO as camera, the earbuds, tape.
   - iPhone: Loopback button; full brightness; auto-brightness off.
   - iQOO camera: highest frame rate that still records normal-speed sound (test a 2 s clip).
   - Earbuds run: one earbud taped to the iQOO's bottom microphone; start recording; tap Start;
     wait for `20 / 20`; stop. Speaker run: earbuds disconnected, phone speaker toward the mic.
   - Copy both files to the Mac:
     ```
     $ python3 tools/loopback.py runs/loopback/earbuds.mp4
     $ python3 tools/loopback.py runs/loopback/speaker.mp4
     ```
     (needs `ffmpeg`, `ffprobe`, `numpy`). Expect 20 pairs each. d_route(earbuds) =
     median(earbuds) - median(speaker) + outputLatency(speaker) (the app logs `outputLatency`
     per route; read it from the Xcode console).
   - Enter the result in the app's "Route delay (ms)" field. Onsets become `loopback`.
3. S2: scenario `dense`, Dana, Coach, earbuds, 15+ minutes, both phones charging. Read the panel;
   export capture and ledger into `runs/s2-15min/`. Write T0->T3 p99 **with its bound** in
   LATENCY.md and DEVLOG Day 15. With about 60 cues p99 is nearly the maximum: say so.

## Phase 6: Day 16, dry run and LATENCY.md

1. Re-sign the iPhone app (free profile lasts 7 days; do it when fewer than 4 remain); note
   the expiry in DEVLOG.
2. Dry run R00 exactly like a counted session (Phase 7's runbook): Study on, rider R00,
   session 1 (seed 1), scenario `creep_pilot`, Human rider on, rig "I am the rider", slider at 70 %.
   ```
   $ mkdir -p runs/study/R00 && mv ~/Downloads/ledger-*.jsonl ~/Downloads/ride-*.vcraw runs/study/R00/
   $ cargo run -q -p vc-cli -- replay runs/study/R00/ride-*.vcraw
   $ cargo run -q -p vc-report -- runs/study/R00/*.jsonl -o runs/study/R00/report.html --audit
   ```
   Pass: replay `identical`; M3 0 P1-with-arm; M4 one `policy_hash`; M5 >= 90 % valid and p95
   bound <= 15 ms; every Complied cue in human mode has an `l_onset_ms`; the iQOO showed
   nothing during the ride; 15-20 decision points, about a fifth shams.
3. If human mode does not work by tonight: K3 (pilot in L1: Simulator in-process with an
   on-screen slider; record it as a Deviation).
4. Fill `docs/LATENCY.md` completely (S1, S2, S3, d_route, skew flags, the three per-cue chains).

## Phase 7: Days 17-18, soak, sessions, analysis

Read `docs/RUNBOOK.md` end to end first; print the checklist and the briefing card.

1. **Soak 1 (60 min)**: scenario `soak`, Dana, Coach, capture on, both phones on chargers, rig
   app in front. Do not touch either screen. If background modes were granted, lock the iPhone
   from 30:00 and say so in DEVLOG. Afterwards:
   ```
   $ mkdir -p runs/soak-1 && mv ~/Downloads/{soak,ride,ledger}-*.{csv,vcraw,jsonl} runs/soak-1/ 2>/dev/null
   $ cargo run -q -p vc-cli -- soak-check runs/soak-1/soak-*.csv     # S6: PASS
   $ cargo run -q -p vc-cli -- replay runs/soak-1/ride-*.vcraw       # identical
   ```
   Note battery start/end on both phones and the rig's dropped counter by hand. A fix is
   allowed only if decisions stay untouched; rerun after any fix.
2. **Counted sessions**, exactly as the runbook: R01 S1-S3 (seeds 1001-1003), R02 consent + S1-S3
   (2001-2003), R03 consent + S1-S3 (3001-3003). Per session: check hash, Study on, right
   rider/session (seed shown), policy_hash equals SESSIONS.md, phone face down, export, replay,
   audit, log entry. **Freeze:** no decision-affecting code changes from the first counted session
   to the last. Bugs go in DEVLOG, protocol changes in STUDY.md Deviations first.
3. K4: no sessions after Day 18. Debrief each participant after their last session.
4. Backups every study day:
   ```
   $ cd runs/study && shasum -a 256 R0*/* > SHA256SUMS && zip -er ~/study-backup.zip . -x '*.DS_Store'
   ```
5. Analysis (Day 18 H6): verify checksums, replay every capture, then
   ```
   $ cargo run -q -p vc-report -- runs/study/R0*/*.jsonl --exclude-rider R00 -o runs/study/results.html --audit | tee runs/study/audit.txt
   ```
   All five audit lines must PASS before reading a result. Fill STUDY.md's Results section from
   `results.html` and `audit.txt` (numbers, not adjectives), including the per-person table and
   the commit the analysis ran at.
6. M6: copy R01's and each consenting friend's **ledger JSONL only** into `data/study/`, then
   ```
   $ cargo run -q -p vc-report -- data/study/*.jsonl -o docs/study/results.html
   $ git add data/study docs/study && git commit -m "data: published pseudonymous ledgers and report"
   ```
   CI then compares a regenerated report byte for byte.
7. Soak 2 happens on Day 20 (Phase 9).

## Phase 8: Day 19, documents and recording

1. Re-check Aki's Knowledge Hub and domesly.com (wording only), update `docs/FACT_CHECK.md`:
   verify the two reaction-time papers and the posts that supply the parameter names and the
   earcon table at the source; fill the GoldenCheetah commit hash
   (`git log -1 --format=%H -- src/Metrics/WPrime.cpp` in a clone) in `docs/WPRIME_GC.md`;
   read the Piper voice's `MODEL_CARD` and fill `THIRD_PARTY.md` (if the licence is restricted,
   say so there and in the README, or re-render with a permissive voice).
2. Fill every `__` in README, LATENCY, ENGINEERING_NOTES (Appendix A). The final repository must
   contain none that is not an intentional template cell. `grep -rn '__' README.md docs STUDY.md`.
3. Rehearse D1-D7 twice (docs/DEMOS.md has the exact headless numbers for D2, D3, D4 and D5):
   record the phone-side ride times in the rehearsal table. D3: use fault second 847.
4. Record pictures (no talking) and the voiceover separately; follow DEMOS.md's shot list;
   play D1/D3/D6 cues through the iPhone speaker and caption them.

## Phase 9: Day 20, ship

1. Edit to <= 3:00 (voiceover first). Upload **unlisted** as soon as it exports, with your own
   SRT, then put the URL in the README before tagging.
2. **Soak 2** (60 min) on the build that ships; both soaks side by side in
   ENGINEERING_NOTES. If it fails where soak 1 passed: document it honestly, do not rerun until
   green.
3. CI green on every job; fuzz smoke: `scripts/fuzz-all.sh 20`; execution counts go in
   RELEASE_NOTES. Any crash blocks the tag.
4. Hygiene (once, before making the repository public):
   ```
   $ git status --short
   $ git ls-files | grep -E '^runs/|\.vcraw$|CONSENT|SESSIONS|\.mp4$' && echo "PRIVATE FILES TRACKED"
   $ git ls-files data/study
   $ grep -h -o '"rider_id":"[^"]*"' data/study/*.jsonl | sort -u
   $ git log --all --format='%an <%ae>' | sort -u
   ```
   Also `git grep -i '<each friend's name>'` locally (never write the names into a file).
5. Tag and release (v1.0.0 only if the E1 line is honest and CI is green):
   ```
   $ git tag -a v1.0.0 -m "velocoach-rs v1.0.0: closed-loop voice coach with per-cue efficacy scoring. S1 T1->T2 p99 __ us; S2 T0->T3 p99 __ ms +/- __ ms; S6 soak x2; E1-E8 green; pilot n = __ (feasibility)."
   $ git push origin main v1.0.0
   $ gh release create v1.0.0 --title "velocoach-rs v1.0.0" --notes-file docs/RELEASE_NOTES.md docs/study/results.html
   ```
6. Make the repository public; open the README in a private window and click every link.
7. The message: 150 words or fewer, numbers from the README, the mandatory caveat in the same
   breath as the pilot ("feasibility pilot, slider proxy, not cycling, n = __"), no location.
   Template: the final section of the Day 19-20 document (also in `docs/RELEASE_NOTES.md`
   style). Do not say the observer was validated against a known rider unless E1 passes.
8. Keep the last hour as buffer. Write DEVLOG Day 20, including the three things you would do
   differently.

## Rules that never relax

- The safety gate, S9, E3, golden logs, fuzzing, latency with its bound, E1/E2, the Study
  freeze, the ledger schema, pre-registration and consent are never cut.
- Every claim has a number and every number a source; the mandatory language ("feasibility
  pilot", "slider proxy, not cycling", "n = __ from __ people", "personas are assumptions")
  travels everywhere the pilot is mentioned: video, README, message.
- Never grade Aki's work; credit him by name and post title; say nothing about where anyone
  is based.
- If the clock runs out, cut in the plan's order (IPS first; friends' sessions before your
  own; GoldenCheetah cross-check is already done).

## Appendix A: where every blank comes from

| File | Blanks | Source |
|---|---|---|
| `STUDY.md` (protocol part) | 1 (the date) | you, on commit day |
| `STUDY.md` (Results) | about 65 | `runs/study/results.html`, `audit.txt` (Phase 7.5) |
| `docs/LATENCY.md` | 56 | S1, S2, loopback, dry run, soak 2 (Phases 3, 5, 6, 9) |
| `docs/ENGINEERING_NOTES.md` | 13 | fuzz logs, Vivo findings, the UniFFI byte type (Swift `Data`, Kotlin `ByteArray`), both soaks |
| `README.md` | 27 | LATENCY.md, results.html, fuzz counts, video URL, Hub posts, Piper voice |
| `docs/DEMOS.md` | 20 | phone rehearsal (Phase 8.3) |
| `docs/RUNBOOK.md` | 26 | session-log templates: filled per session in `runs/study/SESSIONS.md`, not here |
| `docs/RELEASE_NOTES.md` | 9 | LATENCY.md, fuzz smoke, results |
| `docs/FACT_CHECK.md` | 9 | Phase 8.1 |
| `docs/DEVLOG.md` | Day 11 onward | the day's numbers (templates for Days 15-20 are in place) |
| `THIRD_PARTY.md` | 1 | the voice's model card |

## Appendix B: if something goes wrong

| Symptom | Likely cause | First move |
|---|---|---|
| `T0>T1` climbs | clock bound wrong, stale sample after reconnect | the PING burst and `reset_link` on connect; see Day 16 table |
| replay prints `different` | hidden input in the Engine | bisect on the capture; fix the Engine |
| `shasum -c` fails | a ledger was edited | restore from the encrypted backup |
| M4: two `policy_hash` values | rebuild with another configuration, or Study off | stop; report the sessions separately |
| soak-check FAIL | a recovery slower than the grace | open the capture first, the soak log second |
| `check-protocol-hash.sh` MISMATCH | STUDY.md committed after the hash was set | update `StudyConfig.protocolHash`, rebuild |
| app says "STUDY.md is not committed yet" | `protocolHash` still `PENDING` | Phase 2 step 3 |
| iPhone app expired | free profile (7 days) | re-sign; never between a person's sessions |
