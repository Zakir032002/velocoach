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
