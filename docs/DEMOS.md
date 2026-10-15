# Demo scenes and the video

How each scene in the 3-minute video is produced, so it can be rehearsed until it
behaves the same way twice, and the shot list and voiceover that go with it. Back to the
[README](../README.md).

Every scene is a seeded run in **Coach mode** (not Study), so badges show. None of these
runs counts for anything. The study's code is the code that ran the study: only a scenario,
two printers and a button were added for the demos (they sit outside the decision path).

## Scenes

| Scene | Where | Set-up | The moment | What the screen shows |
|---|---|---|---|---|
| D1 | iPhone + rig | `climb_surge`, Dana, Coach | the first W′-low earcon, then the phrase | a ✓ badge with L_obs |
| D2 | Mac terminal | `creep_long`, Sam, Coach | the first `tuner_change` line | the rule (T3), Direct -> Autonomy, n, evidence |
| D3 | iPhone + rig | `recovery_drift`, Dana, Coach, HR fault at the rehearsed time | the safety cue | a P2 phrase cut off by the safety cue; then the E3 test output |
| D4 | Mac terminal | `grade_demo`, Coach | the cue across the grade change | `? grade` beside that cue |
| D5 | iPhone + rig | `dropout`, Dana, Coach | the scheduled power dropout | quality Stale -> Lost, "Check power meter connection.", Recovering -> Fresh |
| D6 | iPhone + rig, you on the slider | `creep_pilot`, human, slider at 70 %, Coach | the first grey-zone cue | ease the slider; badge ✓ __ s = L_onset |
| D7 | Mac browser | `docs/study/results.html` | — | arms, sham false-positive rate, CIs, "feasibility pilot" |

### Headless rehearsal (done; the Mac-side scenes are deterministic)

These were produced with `vc-cli` and are exact; the phone scenes still need two runs on
the phones (below).

| Scene | Command | Result |
|---|---|---|
| D1 (headless twin) | `vc-cli sim --scenario climb_surge` | W′-low cue 3 at 529.5 s (`WPL_D`, direct), observer: complied, L_obs 3.0 s (the persona's true reaction: 1.2 s; the ~2 s difference is the smoothing lag); cue 7 at 869.75 s (autonomy), L_obs 3.8 s. Surge countdown/go-now at 295/300 s and 715/720 s |
| D2 | `vc-cli sim --scenario creep_long --jsonl /tmp/d2.jsonl` then `grep tuner_change /tmp/d2.jsonl` | one `tuner_change`: rule T3, `default_variant:GREY_ZONE_DRIFT` 0 -> 1 (Direct -> Autonomy), n = 16, evidence 0.5, at 2,520 s (42:00). Total ride 3,120 s |
| D3 | `vc-cli find-interrupt --scenario recovery_drift` | HR-redline fault at 846, 847 or 848 s makes the safety cue interrupt a phrase at 1,083.0 s. Use **847 s**, the middle of the run of hits |
| D4 | `vc-cli sim --scenario grade_demo` | the grey-zone cue at 441.0 s is `confounded ... ? grade` (the second -6 % pulse falls inside its window). Note: the original `grade_confound` scenario scores the same cue `complied`, because its grade block ends at 430 s, before the cue's onset, so it cannot show `? grade` |
| D5 | `vc-cli sim --scenario dropout` | the SENSOR_DROPOUT cue at 608.25 s |
| D6 (headless twin) | `vc-cli sim --scenario creep_pilot` | first grey-zone cue at 149.25 s (persona rider, default seed); with a human holding the slider at 70 % (seed 1001) the first cue is at 99 s and the next at 153 s, at 75 % at 78 s. Plan about 100 s of ride before the moment |

## Demo tools

1. `vc-cli sim` prints the outcomes after the decisions (cue, directive, arm, verdict,
   L_obs, confounders), so D2 and D4 are visible in a terminal.
2. Scenario 9, `creep_long` (D2): twelve creep blocks, because the tuner's T3 needs at
   least 8 cues per phrasing.
3. Scenario 10, `grade_demo` (D4): `grade_confound` plus a second grade pulse across the
   cue's window.
4. `vc-cli find-interrupt --scenario recovery_drift` (D3): searches the fault time that
   makes the safety gate trip during a 2.4-second clip. Pick the middle of a run of hits.
5. A fault field on the iPhone (D3): "HR redline at __ s" on the ride screen sends
   `encode_fault(hrRedline, ...)` at that ride second.

## The rehearsal rule

Run every scene twice. It is ready when both runs show the same thing at the same ride
time (+/- a second on the phones). Write each one's numbers here.

| Scene | Scenario / persona / seed | Ride time of the moment | What to do and when | What appears | Run 1 | Run 2 |
|---|---|---|---|---|---|---|
| D1 | climb_surge / Dana / __ | __ s | start; wait for the earcon | ✓ __ s | | |
| D2 | creep_long / Sam / 42 | 2,520 s | (Mac) | tuner_change T3 | exact | exact |
| D3 | recovery_drift / Dana / __ | __ s | HR redline at 847 s | cut-off phrase | | |
| D4 | grade_demo / Dana / 42 | 441 s | (Mac) | `? grade` | exact | exact |
| D5 | dropout / Dana / __ | __ s | start | Stale -> Lost -> Fresh | | |
| D6 | creep_pilot / human, slider 70 % / __ | __ s | ease the slider after the words | ✓ __ s | | |
| D7 | — | — | open results.html | report card | | |

| If a scene... | Then |
|---|---|
| D1's badge reads about 3.8 s, not 1.8 s expected | in Coach mode the badge shows L_obs, which includes the 2 s smoothing lag. Say so in the voiceover ("about two seconds of that is the smoothing"); do not change the badge for the video |
| D2 shows no `tuner_change` | Sam's direct and autonomy counts never both reached 8; read the report card's per-arm counts and lengthen the run (another creep block) rather than lowering the threshold |
| D3 never interrupts on the phone | the phone's cue times drifted from the simulation's; try the neighbouring hits from find-interrupt |
| D4 shows `? grade` on the wrong cue | the window that straddles the grade change is the one that should be confounded; check which cue's window contains it |
| D6's first move is "coincidental" | you moved before the phrase finished; wait for the words, then move |

## Recording

Record pictures and voice separately. Every scene is captured as a screen recording (or a
camera shot for the hardware), with no talking; the voiceover is recorded afterwards on the
Mac, segment by segment, against the script.

| What | Tool | Notes |
|---|---|---|
| iPhone screen | built-in screen recording (Control Centre) | microphone off; Do Not Disturb on; full brightness |
| iQOO screen | built-in screen recorder | only for D6's slider and D5's rig view; no notifications |
| Mac terminal, browser, docs | QuickTime -> New Screen Recording, one window | terminal font 18 pt or larger; no clutter |
| The hardware shot (0:00) | Mac camera via QuickTime -> New Movie Recording, or a borrowed phone | both phones and an earbud on a plain desk, daylight |
| The diagram (0:12) | one Keynote slide | the crate map and the two phones, from the README, large type |
| Voiceover | QuickTime -> New Audio Recording, the best microphone available | a small room with soft furnishings; same distance every take |

Bluetooth audio does not reliably appear in iPhone screen recordings. For the scenes where
the cue must be heard (D1, D3, D6), play it through the iPhone's speaker: disconnect the
earbuds for those takes, and put a caption on screen: **"A rider would hear this in
open-ear headphones."**

## The video, as the plan specifies it

| Time | Shot | Line |
|---|---|---|
| 0:00-0:12 | earbud + both phones | "A voice coach that checks whether its own cues worked." |
| 0:12-0:35 | the §3.1 diagram | sans-IO Rust core, no_std, UniFFI, real BLE |
| 0:35-1:10 | D6 + D1 | badge, L_onset, earcon-first |
| 1:10-1:35 | D3 | "Adaptive code can't reach P1: property-tested." |
| 1:35-2:05 | D2 | Aki's parameter names, bounded ranges |
| 2:05-2:30 | the report card | arms, sham false-positive rate, CIs, "feasibility pilot" |
| 2:30-2:50 | LATENCY.md | T0->T3 p99 +/- bound; T4 estimated vs loopback |
| 2:50-3:00 | the repo | "Built in 20 days; here's what I'd do next." |

### Shot list

| Segment | Shots to capture | Spare |
|---|---|---|
| 0:00-0:12 | both phones and an earbud on the desk; a slow push-in | a second angle |
| 0:12-0:35 | the diagram slide; the five golden logs passing `golden --check`; a replay saying "identical" | the crate list scrolling |
| 0:35-1:10 | D6 (iPhone screen + the iQOO slider, side by side) from 10 s before the cue to the badge; D1 from the earcon to the badge | a second D6 take |
| 1:10-1:35 | D3 on the iPhone: the P2 phrase cut off by the safety cue; the E3 test passing in the terminal | vc-core/src/efficacy tunable type on screen |
| 1:35-2:05 | D2 in the terminal: outcome lines scrolling, the `tuner_change` JSON | the README's tuner paragraph |
| 2:05-2:30 | D7: the report card, scrolled slowly from the header to the per-person table | STUDY.md's Results heading |
| 2:30-2:50 | LATENCY.md: the S2 line and the S3 table | the loopback video frame with the flash |
| 2:50-3:00 | the GitHub repository page, the README top screen | the tag v1.0.0 |

### Voiceover script

About 2.3 words a second leaves room to breathe; 3:00 is roughly 400 words. Fill the
blanks from DEVLOG before recording, and read numbers exactly as they appear on screen.

**0:00 (about 25 words).** This is a voice coach for cycling, written in Rust, that checks
whether its own cues worked. The rider is simulated; the Bluetooth is real.

**0:12 (about 50 words).** The core is sans-IO Rust: no_std, no allocation per packet, the
same code on a Mac and on both phones through UniFFI. The Android phone is a Bluetooth
sensor running a simulated rider; the iPhone is the coach. Any ride can be replayed byte
for byte.

**0:35 (about 80 words).** Here I'm the rider, on a slider. A tone, then the phrase, and I
ease off. The badge is my response time from the start of the phrase: __ seconds. With no
human, the observer works from power alone: it compares power with where it was already
heading, and calls it "complied" only if the change holds. That number includes about two
seconds of smoothing, which is documented and measured.

**1:10 (about 55 words).** Safety comes first. When heart rate stays near maximum, the
safety cue interrupts anything that's playing. The adaptive code can't touch this: the
tuner's type has no field for it, and a property test throws random histories at the whole
engine to check the safety cue still fires on time.

**1:35 (about 65 words).** What the tuner can change is delivery, in Aki Kakko's own
parameter names and ranges: a cooldown of 15 to 45 seconds, a countdown of 5 to 8. This
simulated rider ignores orders but responds to suggestions; after eight cues of each
phrasing, the tuner switches the default, and logs why.

**2:05 (about 60 words).** Then a small pre-registered study: __ people, three sessions
each, a slider, not a bike. A fifth of the cues were shams, decided but never played, so
the observer's own false positives are measured too. It's a feasibility pilot, n = __; the
intervals are wide, and the report says so.

**2:30 (about 40 words).** From the sensor sending a packet to the audio being scheduled:
p99 __ milliseconds, plus or minus __ for clock sync. Earbuds add about __ more, measured
with a camera, not estimated.

**2:50 (about 20 words).** Built in twenty days. Everything is in the repository, including
what I'd do next.

Two things not to say: that the study shows the coach works (it is a feasibility pilot of an
instrument), and anything about Aki's phrases being better or worse than others. While E1
is open (docs/EFFICACY.md), do not say the observer "was validated against a known rider";
the 0:35 segment's "documented and measured" smoothing lag is the documented constant
only until E1 passes.

### Edit rules (Day 20)

1. Lay the voiceover first, all eight segments end to end, with about half a second of air
   between them. Its total length is the video's length. If over 3:00, cut words first.
2. Lay pictures under each segment. Speed up waiting time to 4x or 8x with a small label;
   never speed up the moment itself.
3. Captions on screen for every number, and for the three mandatory phrases during
   2:05-2:30: "feasibility pilot", "slider proxy, not cycling", "n = __ from __ people".
4. The speaker-played caption on D1, D3 and D6.
5. A title card (two seconds at most): "velocoach-rs · Zakir Shaik". An end card with the
   repository URL in large type.
6. Voiceover levelled; cue audio slightly under the voice; no music, or very quiet music
   under 0:00-0:12 only.

An SRT written from the script (one cue per sentence, under two lines of about 42
characters, timed against the final edit), uploaded instead of automatic captions:

```
1
00:00:00,500 --> 00:00:05,000
This is a voice coach for cycling, written in Rust,

2
00:00:05,000 --> 00:00:09,500
that checks whether its own cues worked.

3
00:00:09,500 --> 00:00:12,000
The rider is simulated; the Bluetooth is real.
```

Export 1080p, H.264, at the recording's frame rate. Watch it twice, once with sound and once
muted: 3:00 or less; every number on screen equals the one in the README and the docs; the
three mandatory phrases visible during the study segment; the speaker caption on every
speaker-played cue; no notification, phone number, Wi-Fi name, email or participant name in
any frame and no consent sheet in shot; Aki's name spelled right and nothing that grades his
work; understandable muted and with sound; the last frame shows the repository URL long
enough to read.
