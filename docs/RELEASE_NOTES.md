# Release notes (drafts)

Tags are made by the author after the phone runs ([PHONE_CHECKLIST.md](PHONE_CHECKLIST.md)),
not before. Every figure is filled from `results/<stamp>/SUMMARY.md`. Back to the
[README](../README.md).

## v0.1.0: two phones, real BLE, simulated sensors (after the first phone run)

- The iPhone 12 coach and the iQOO rig connect over real BLE. Sensors are simulated.
- BLE notify → core decision (T1→T2) p99 __ µs; sensor send → audio scheduled (T0→T3)
  p99 __ ms ± __ ms (clock-sync bound). Earbud audio-output latency: not measured.
- The device capture replays to the same decisions: yes/no.
- In-process fallback (L1) on the iPhone, labelled "no radio".

## v0.2.0: the instrument (after the soak runs)

- Observer `vc-efficacy 0.3.0`, frozen. E1: 15/20 seeds within tolerance (seeds 4,5,8,12,17
  fail) and 17/20 on held-out seeds (1003,1005,1013 fail), against ≥ 18/20. **The criterion
  is not met.** See [EFFICACY_CHANGELOG.md](EFFICACY_CHANGELOG.md).
- E2 6/6, E3/E4 property tests, E6 byte-identical ledgers on Linux and macOS, E8 190/200.
- Two 60-minute soaks with scripted faults: __. Fuzzing: 6 targets × 10 min, 0 crashes (ENGINEERING_NOTES.md).
- The pre-registered study protocol has been run end to end in software
  (`scripts/sim-study.sh`). Human pilot: planned, pre-registered, not yet run.
- Known limits: simulated sensors; one device pair; personas are assumptions; compliance is
  not benefit.
