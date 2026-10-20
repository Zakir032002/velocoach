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

- Done: vc-core no_std (config with the published parameter names + profile table, 3-s smoothing, safety gate, workout + auto-pause, eight directives, testkit), table tests, engine tests, alloc test, thumbv7em CI
- Measured: test count 29; directive timings as in the tables (surge 55/60 s, W′ 106.75 s, safety 122 s)
- Decision: auto-pause uses "power under 20 W, cranks stopped" because there is no wheel speed
- Decision: CADENCE_LOW is not a power ask (dir 0), so the observer will not score it on power
- Decision: the Tcore > 39.5 C safety rule is documented, not implemented (no sensor)
- Broke: none
- Tomorrow: Policy Engine, phrase bank, lint, S9 proptests, golden format

## Day 6 — 2026-10-12 (v-final plan)

- Done: Policy Engine (staleness, P1 first and interrupting, profile mute, cooldown from clip end, observer window, Coach round-robin), phrase bank (16) + earcons, lint strict/terse, S9 properties, golden format, vc-cli golden stub with 3 rides
- Measured: tests 45; proptest cases 512 + 512 + 24; golden logs surge_wprime 3 / redline_rolling 2 / dropout 1 decisions
- Decision: safety-critical behaviour is code in decide(), not configuration, so no tuner can reach it
- Decision: cooldown and observer window run from the clip's real end; the window only binds once cooldown nears 15 s
- Decision: "Soft-pedal" hyphenated as the published verb; four surge recordings, one per lead (5-8 s)
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

## Days 8–20

Day 8 onward is recorded in the git history (one commit per piece of work) and in the
documents it produced: the observer and E1 in [EFFICACY.md](EFFICACY.md) and
[EFFICACY_CHANGELOG.md](EFFICACY_CHANGELOG.md), fuzzing and design decisions in
[ENGINEERING_NOTES.md](ENGINEERING_NOTES.md), phone measurements in [LATENCY.md](LATENCY.md),
and the current state in the README's Status table.
