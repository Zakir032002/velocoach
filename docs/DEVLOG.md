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
- Measured: tests 46; proptest cases 512 + 512 + 24; golden logs surge_wprime 3 / redline_rolling 2 / dropout 1 decisions
- Decision: safety-critical behaviour is code in decide(), not configuration, so no tuner can reach it
- Decision: cooldown and observer window run from the clip's real end; the window only binds once cooldown nears 15 s
- Decision: "Soft-pedal" hyphenated as Aki's verb; four surge recordings, one per lead (5-8 s)
- Broke: none
- Tomorrow: vc-gatt RIDER_INPUT + CUE_PLAYED, vc-sim scenarios and personas, vc-cli sim, golden logs from scenarios, 10-min fuzz

