## velocoach-rs v1.0.0

A closed-loop voice coach in Rust that measures whether its own cues work.

- Gates passed: G-11 (v0.1.0, two phones, replay), G-14 (v0.2.0, headless
  closed loop), and this release.
- Latency: T1->T2 p99 __ us; T0->T3 p99 __ ms +/- __ ms; earbud route __ ms (loopback).
- Robustness: two 60-minute soaks with scripted faults, __; __ M fuzz executions,
  __ crashes.
- Instrument: E1 (observer recovers a known rider) **open: see docs/EFFICACY.md**, E2
  (every confounder), E3/E4 (the tuner cannot reach safety; bounded), E6 (byte-identical
  ledgers across operating systems), E8 (statistics calibrated with a null rider, 190/200).
- Feasibility pilot: n = __ decision points from __ people, slider proxy, not
  cycling. See STUDY.md.
- Known limits: simulated sensors; one device pair; observer latency includes
  ~2 s smoothing; personas are assumptions.
