# DEVLOG

## Day 1 — 2026-10-04

- **Done:** nRF Connect HR server on iQOO; BleProbe/Scanner app receives 0x2A37; workspace + CI pushed
- **Measured:** packet interval ~1000 ms; first bytes seen: 16 9E 00 04
- **Versions:** iOS 18, Xcode 16, macOS (Apple Silicon arm64)
- **Decision:** Background Modes on free team = signed (added to Info.plist directly)
- **Broke:** None
- **Tomorrow:** vc-gatt HRM + CSC

## Day 2 — 2026-10-07

- **Done:** cursor, HRM, CSC, Sensor Location, 10k-case proptests
- **Measured:** test count 18 (5 roundtrip proptests + 13 unit/vector tests), proptest time 0.44 s
- **Decision:** RR capped at 9 (default MTU), oldest kept; explicit units in fields (time_1024, rr_1024)
- **Broke:** u32_le endianness bug in cursor.rs fixed (switched from big-endian to little-endian)
- **Tomorrow:** CPM, VC-SIM codecs, fuzz targets

## Day 3 — 2026-10-10

- **Done:** CPM (all 13 flag bits), CP/CSC Feature, VC-SIM TELEM/CTRL, proptests, 5 fuzz targets, fuzz CI
- **Measured:** fuzz execs in 10 min: ctrl 471,537,101 runs; crashes 0; 23 tests passing
- **Decision:** CPM extreme fields kept as raw bytes (types unverified, sim never sends them)
- **Decision:** VC-SIM is strict (exact length, versioned) so accepted bytes re-encode identically
- **Broke:** None
- **Tomorrow:** vc-physio (W′ balance, quality state machine, cadence deriver)

## Day 4 — 2026-10-12

- Done: W′ balance exact + GoldenCheetah parity + Waterworth, quality state machine, cadence deriver
- Measured: exact vs GC worst gap over 1 h = 38.917 J
- Decision: readings cover the interval just ended; gaps > 10 s are zero-filled; negative or non-finite power counts as 0 W
- Decision: first revolution after > 2.5 s quiet is a baseline
- Decision: f64 retained; software cost on a single-precision-FPU MCU acknowledged
- Broke: none
- Tomorrow: vc-core no_std engine, eight directives, workout progression, S10 zero allocations

## Day 5 — 2026-10-12 (v-final plan)

- Done: vc-core no_std (config with Aki's names + profile table, 3-s smoothing, safety gate, workout + auto-pause, eight directives, testkit), table tests, engine tests, alloc test, thumbv7em CI
- Measured: test count 29; directive timings as in the tables (surge 55/60 s, W′ 106.75 s, safety 122 s)
- Decision: auto-pause uses "power under 20 W, cranks stopped" because there is no wheel speed
- Decision: CADENCE_LOW is not a power ask (dir 0), so the observer will not score it on power
- Decision: Aki's Tcore > 39.5 C rule is documented, not implemented (no sensor)
- Broke: none
- Tomorrow: Policy Engine, phrase bank, lint, S9 proptests, golden format

## Day 6 — 2026-10-12 (v-final plan)

- Done: Policy Engine (staleness, P1 first and interrupting, profile mute, cooldown from clip end, observer window, Coach round-robin), phrase bank (16) + earcons, lint strict/terse, S9 properties, golden format, vc-cli golden stub with 3 rides
- Measured: tests 45; proptest cases 512 + 512 + 24; golden logs surge_wprime 3 / redline_rolling 2 / dropout 1 decisions
- Decision: safety-critical behaviour is code in decide(), not configuration, so no tuner can reach it
- Decision: cooldown and observer window run from the clip's real end; the window only binds once cooldown nears 15 s
- Decision: "Soft-pedal" hyphenated as Aki's verb; four surge recordings, one per lead (5-8 s)
- Broke: none
- Tomorrow: vc-gatt RIDER_INPUT + CUE_PLAYED, vc-sim scenarios and personas, vc-cli sim, golden logs from scenarios, 10-min fuzz

## Day 7 — 2026-10-13 (v-final plan)

- Done: vc-gatt RIDER_INPUT (14 B), CUE_PLAYED (15 B), LOAD v2 (9 B); Pcg32 in vc-core::study; vc-sim (4 Hz rider, six scenarios, Dana/Sam/Ned/Calib, spontaneous correction, ground truth); vc-cli sim; golden logs x5; vcsim fuzz target
- Measured: headless 20-min ride in 45 ms; personas over 4000 draws: Dana 90%/90% (1.8s), Sam 30%/80% (2.5s), Ned 50%/50% (2.5s), Calib 70%/70% (3.0s); fuzz 35.1M executions, crashes 0
- Decision: LOAD changed to the plan's layout (persona + human); other Day 3 layouts kept, README table updated
- Decision: one PRNG (Pcg32 in vc-core::study), moved forward from Day 8 so the simulator can share it
- Decision: the rider steps at 4 Hz so a response is visible within 250 ms (E1 needs +/-0.5 s)
- Broke: none
- Tomorrow: vc-efficacy types, Study allocator + policy_hash, vc-ffi, build scripts, both shells linking

## Day 9 — 2026-10-__ (v-final plan)
- Done: manifest + runtime permissions before startForeground; RigService (connectedDevice); GattRig (4 services one at a time, adv + scan response); NotifyQueue (one in flight, drop-oldest-same-char, 500 ms watchdog); SimDriver (HandlerThread, 50 ms drift-free, CTRL, PONG); vc-cli hex; encode_pong
- Measured: nRF Connect over 60 s: HRM __ / CPM __ / CSC __, dropped __; reconnect in __ s
- Vivo: battery __, autostart __, recents lock __ (menu names on this build: __)
- Decision: CTRL writes answered before the simulator sees them; nothing on a binder thread waits
- Broke: __
- Tomorrow: the iPhone central, the Engine on ble.q, Piper phrases, the audio player

## Day 10 — 2026-10-__ (v-final plan)
- Done: VeloCoach app (Info.plist, background modes: granted/refused __); Clock (mach_continuous_time); BLECentral on ble.q, T1 first line, auto-reconnect; CoachModel (Engine on ble.q, strict 250 ms tick + packet fallback, snapshot 4 Hz); Piper phrase bank + earcons, __ buffers, longest __ ms; AudioCuePlayer (voicePrompt/duckOthers, hostTime, dataPlayedBack, generations, interruptions); CUE_PLAYED; calibration
- Measured: first surge countdown at __ s, go at __ s, W′ cue at __ s; Dana's power after the cue __ W; callback - computed end: __ ms (earbuds), __ ms (speaker)
- Installed: 2026-10-__ __:__; profile expires 2026-10-__; re-sign Day 16 H1
- Decision: session active all ride (no activation delay before a cue; music stays ducked)
- Decision: onset/end computed from T3 + dur (+ d_route from Day 15); the callback is cross-checked, outputLatency not subtracted
- Broke: __
- Tomorrow: Day 11 gate: 20-min run, T1->T2 histogram, raw capture + replay, Vivo 30-min, tag v0.1.0

## Day 11 — 2026-10-__ (v-final plan) — GATE G-11
- Run 1 (climb_surge, 19 min): faults __; unrecovered __; decisions __; skew __..__ ms
- T1->T2 (15 min, capture off): p50 __ us, p90 __, p99 __, max __ (n=__); with capture on: p99 __
- Replay of run 1: __ decisions, identical: yes/no (if no: cause __, fix __)
- Vivo 30 min, unplugged, screen on: service alive __; CPM sent __ / ~6000; battery __ % -> __ %
- Vivo screen off 2 min (info): power stayed Fresh / went Stale after __ s
- Fixes today: CP Feature crank check; audio route display; rig reopens the GATT server after BT toggles; __
- Gate: 15 min __ · earcons __ · replay __ · CI __ · T1->T2 __  =>  PASS / FAIL
- Tagged v0.1.0 at __
- Tomorrow: vc-efficacy P3Ring + Observer (headless)

### Day 11 run 1 — climb_surge, dana, seed 42, earbuds: __
- 00:00 start; ready after __ s
- 04:55 / 05:00 countdown / go: heard __ / __
- 06:00 rig BT off 10 s: Stale at __, Lost at __, dropout cue at __, ready again at __, Fresh at __
- 08:50 W′ cue at __ (arm __); power after __ W
- 10:00-12:00 locked: __
- 14:00 earbud out/in: __
- totals: decisions __; decode errors __; rig dropped __; CTRL received __; skew range __ to __ ms
- unrecovered failures: __ (gate needs 0 in >= 15 min)

## Day 12 — 2026-10-__ (v-final plan)
- Done: vc-efficacy P3Ring (40 x 1-s bins) and Observer (anchor before onset, OLS pre-trend clamped +/-2 W/s, counterfactual, 3-s hold, verdicts, L_obs, coincidental, 7 confounders); __ synthetic tests + offset property; E5
- Measured: update at a bin boundary __ ns, mid-bin __ ns (M2); allocations 0
- Decision: anchor = the last complete second before onset (mid-phrase reactions cannot move the baseline)
- Decision: L_obs reported as defined (includes ~2.0 s of smoothing lag); SMOOTHING_LAG_MS documented for E1
- Decision: a hole in the power data is a quality confounder, never a verdict
- Broke: __
- Tomorrow: ledger + tuner, E3/E4, Engine integration, E1, E2

## Day 13 — 2026-10-__ (v-final plan)
- Done: vc-efficacy ledger (Beta, 16-entry L_obs ring, lag-corrected fast counts) and tuner (T1/T1', T2, T2-P3, T3, T4, n >= 8, 8 new cues between moves, frozen in Study/Replay); vc-core efficacy hub (observed cues, pending -> open at audio end, late confounders replayed, step-boundary tuning); vc-harness; testkit rides play their cues
- E1 (20 seeds x 200 cues): compliance __..__ (target 0.70 +/- 0.08), latency __..__ s (3.0 +/- 0.5) -> table in EFFICACY.md
- E2: 6 confounders through the Engine, control Ignored; OVERLAP unit-tested only (not producible)
- E3: 32 random ledgers x 9 boundaries: safety untouched, safety cue at 122 s every time
- E4: 512 cases in bounds; Study/Replay frozen
- Decision: the surge's "Go now" is observed (T1 needs it); every other P1 cue is not
- Decision: tuner latency thresholds use L_obs - smoothing lag
- Checkpoint: E1-E4 green -> IPS allowed on Day 14 H6: yes/no
- Broke: __
- Tomorrow: ledger ring, vc-report (schema, JSONL, stats, html, audit), E6, E8, gate G-14

## Day 14 — 2026-10-__ (v-final plan) — GATE G-14
- Done: LedgerRing<256> + ledger_pop + session start/end; vc-report (velocoach.irp.v1 writer/reader, Wilson, person-stratified bootstrap B=10,000 seed 20261013, median/IQR, html report card, audit M2-M5, IPS/SNIPS + conservative gate: built/cut); drain_ledger_jsonl on the phone; vc-cli --jsonl; E6 in CI
- E1 __/20 · E2 6/6 · E3 ok · E4 ok · E5 __ ns, 0 allocs · E6 identical on Linux and macOS · E8 coverage __/200
- S4 ok · S5 ok · S6 scheduled Day 17 · S7 ok · S8 ok · S9 ok · S10 ok (with ledger push)
- Tagged v0.2.0 at __
- Decision: the session id is derived (start time, seed, rider), never from a wall clock
- Decision: E8 tests the statistics with Ned's outcomes drawn directly; E1 tests the observer
- Broke: __
- Tomorrow: clock sync, per-cue T0-T3, d_route loopback, 15-min S2 run, STUDY.md, human mode

## Day 15 — 2026-10-__ (v-final plan)
- Done: clock.rs (Cristian, min-RTT of 16, +/- rtt_min/2; b stamped in the binder callback); PING 8-burst + 1 Hz; STAMPs after every sensor notification; chain.rs (T0-T3 per cue, stamps lost, T0>T1); histograms T0T1/T1T2/T1T3/T0T3 in the Engine; ledger chain fields; scenario 7 dense; loopback screen + tools/loopback.py; route delay setting; STUDY.md committed (hash __); StudyConfig; check-protocol-hash.sh; human mode on the rig (switch, slider, RIDER_INPUT, no cue info)
- Clock: rtt_min __ us, bound +/- __ us, theta __ ms
- d_route: earbuds median __ ms (IQR __-__), speaker __ ms (IQR __-__), speaker outputLatency __ ms -> d_route(earbuds) = __ ms; outputLatency(earbuds) reported __ ms (T4_est vs T4_meas)
- S2 (dense, 15 min, earbuds): T0->T3 p50 __ p90 __ p99 __ max __ ms +/- __ ms (n=__); T0->T1 p99 __ ms (n=__); stamps lost __; T0>T1 __; skew flags __
- Decision: tick-released cues reported as T1->T3 only (no packet, no T0)
- Decision: the rig's "I am the rider" switch wins over LOAD's human byte
- Broke: __
- Tomorrow: re-sign; badges + report card; L_onset; dry run; share sheet; LATENCY.md

## Day 16 — 2026-10-__ (v-final plan)
- Re-signed __:__; new profile expires 2026-10-__ (covers Day 20)
- Done: last_observed + CueOutcomeFfi (directive, arm, L_onset, coincidental); report_card from the ledger; badges, list, card (hidden during Study); rider moves -> L_onset/L_end (5 % rule, 64-input history); human.rs; share-sheet export; --exclude-rider; LATENCY.md
- Dry run R00 (creep, Study, seed 1, human, earbuds): __ decision points (__ sham); verdicts __; L_onset median __ s; replay identical __; audit M2 __ M3 __ M4 __ M5 __ (valid __ %, p95 bound __ us)
- Dry-run findings: __ (interface fixes: __; deviations: none / __)
- K3: human mode working: yes / no -> L1
- Broke: __
- Tomorrow: 60-min soak with a scripted fault plan (S6) + ENGINEERING_NOTES; my sessions R01 S1-S3; friend 1 consent + S1

## Day 17 — 2026-10-__ (v-final plan)
- Soak (60 min, scenario soak, 5 faults + 4 RATE changes): soak-check __ ; recoveries __ s / __ / __ / __ / __ ; replay identical __ ; STAMP loss __ % ; rig dropped __ ; battery iPhone __ -> __ %, iQOO __ -> __ % ; iPhone screen locked from 30:00: yes/no
- ENGINEERING_NOTES.md drafted (__ blanks left)
- Deviations recorded before R01 S1: aborted-session rule; optional publication line on the consent sheet (protocol hash now __)
- Sessions: R01 S1 __ dp, S2 __ dp, S3 __ dp ; R02 S1 __ dp (consent __, publication __)
- Interim audit (no outcomes viewed): M2 __ M3 __ M4 __ M5 __
- Backups: SHA256SUMS + encrypted zip in __ places
- Broke / noted (not fixed): __
- Tomorrow: R02 S2-S3, R03 consent + S1-S3, then the analysis

## Day 18 — 2026-10-__ (v-final plan)
- Sessions: R02 S2 __ dp, S3 __ dp ; R03 consent __ (publication __), S1 __ dp, S2 __ dp, S3 __ dp ; aborted __ ; debriefs done __
- Study total: __ decision points from __ people (sham __ / direct __ / autonomy __) ; excluded __ confounded, __ invalid
- Audit: M2 __ M3 __ M4 __ M5 __ ; every capture identical __ ; checksums OK __
- Primary: cue - sham __ (95 % CI __ to __) ; autonomy - direct __ (95 % CI __ to __)
- STUDY.md Results written ; data/study/ has __ ledgers ; M6 CI step added
- GoldenCheetah: WPrime.cpp @ __ lines __ ; port vs WbalGc max __ J ; WbalExact vs WbalGc max __ J  (or: cut, README drafted)
- Tomorrow: re-check Aki's hub and domesly.com (wording only), README, the four docs, rehearse the demo seeds, record

## Day 19 — 2026-10-__ (v-final plan)
- Hub/site re-check: new posts since Oct 11: __ ; wording changes: __
- Docs: README, EFFICACY, ENGINEERING_NOTES, STUDY (Results), LATENCY, THIRD_PARTY; Piper voice licence: __
- Demo tools: outcomes in vc-cli sim, creep_long (scenario 9), find-interrupt (fault at __ s), encode_fault + fault field
- Rehearsed D1-D7 (DEMOS.md); recorded __ shots + 8 voiceover segments

## Day 20 — 2026-10-__ (v-final plan) — SHIP DAY (template: not yet done)
- Video: __:__ long, uploaded unlisted __:__
- Final soak: S6 __ ; recoveries __ ; replay __
- CI green; fuzz smoke __ executions, 0 crashes; hygiene clean
- Tagged v1.0.0 at __; release published
- Message sent at __:__ via __
- Twenty days: __ commits, __ tests, __ lines of Rust; the three things I'd do differently: __
