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
- Tomorrow: decoupling + vc-sim HR kinetics

