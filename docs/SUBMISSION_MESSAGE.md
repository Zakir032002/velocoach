# Submission message (template)

Send it with the repository link and the video. Fill the `[ ]` parts from the README status
table on the day; leave out any number you don't have. Back to the [README](../README.md).

---

Hi Aki,

Here is my trial submission, velocoach-rs: [repository URL] · [3-minute video URL].

It is a closed-loop voice coach in Rust that measures whether its own cues work. The demo runs
hardware-in-the-loop over real BLE between two phones (an iPhone 12 as the coach, an Android
phone as a simulated rider and sensors). All sensor data is simulated; there is no real rider
or sensor data.

It responds to your posts on the Policy Engine, profiles and earcon-first alerts (credited
with links in the README). What it adds is scoring every cue against ground truth: complied,
ignored, reversed or confounded, with shams to count the observer's own false positives.

Where it stands, honestly:
- Software criteria are measured in simulation and run in CI (golden rides, zero
  allocations per packet, safety property tests, byte-identical ledgers on two OSes).
- The observer's ground-truth test does not meet its criterion: 15/20 seeds within tolerance,
  17/20 on held-out seeds, against 18/20. The fixes and their reasons are in
  docs/EFFICACY_CHANGELOG.md.
- Phone latency: [T1→T2 p99 … / T0→T3 p99 … ± … ms, or "pending"]. Earbud latency is not
  measured.
- Human pilot: planned, pre-registered, not yet run.

About me: open-source contributor to RisingWave, Apache Kvrocks and Apache Fury (since 2024).
[Links to merged PRs only.]

Thanks,
Zakir Shaik
