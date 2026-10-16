# Latency

**Status: pending.** No hardware run has been recorded yet; every `__` below is filled from
`scripts/collect-results.sh` after the phone runs ([PHONE_CHECKLIST.md](PHONE_CHECKLIST.md)).
Sensors are simulated in every run. **Earbud audio-output latency is not measured** (generic
earbuds); no figure for it is claimed anywhere in this repository. Back to the
[README](../README.md).

All times are on the iPhone's clock (mach_continuous_time) except T0, which is
the iQOO's clock mapped onto the iPhone's by clock sync. Every number that
involves T0 carries the sync bound.

## Definitions
| Mark | Meaning |
|---|---|
| T0 | the iQOO hands a sensor notification to its Bluetooth stack (STAMP) |
| T1 | first line of the iPhone's didUpdateValueFor |
| T2 | Engine.ingest (or tick) returned a decision |
| T3 | the cue's scheduled render start (AVAudioTime hostTime = now + lead_ms) |
| T4 = t_on | the cue reaches the ear: T3 + d_route |
| t_end | T3 + clip length + d_route |
| L_obs | observer latency from t_end (includes the derived 1.875 s smoothing lag: (12 - 1)/2 x 250 ms + 500 ms) |
| L_onset, L_end | human mode: first 5 % slider move minus t_on / t_end |

## Clock sync
Cristian's method on PING/PONG at 1 Hz (8-ping burst on every connection);
the minimum-RTT sample of the last 16; error bound +/- rtt_min / 2.
Measured: rtt_min __ us, bound +/- __ us; 95th percentile of the per-cue bound
over the dry run: __ us (M5 requires <= 15 ms). T0 later than T1: __ times.

## S1 — BLE notify to core decision (T1 -> T2)
iPhone 12, 15 min, capture off: p50 __ us, p90 __, p99 __, max __ (n = __).
With capture on: p99 __ us. Target: p99 < 1 ms.

## S2 — sensor send to audio scheduled (T0 -> T3)
Scenario `dense`, 15 min, earbuds. Packet-triggered cues only (n = __):
p50 __ ms, p90 __, p99 __, max __, each +/- __ ms (sync bound).
With n ~ 60, p99 is close to the maximum.
Per packet, T0 -> T1 (n = __): p50 __ ms, p99 __ ms: most of S2 is the radio.
T1 -> T3 includes the deliberate 50 ms scheduling lead: p50 __ ms.
Tick-released cues (countdowns, grace windows) have no T0: T1 -> T3 p50 __ ms.
Target: p99 < 150 ms with the bound shown.

## S3 — to the ear (T0 -> T4): not measured
Earbud output latency is treated as unmeasured. The loopback method below exists; if it is
run, its result goes here with its method. Until then: not measured.

| Route | outputLatency (T4_est - T3) | loopback d_route (T4_meas - T3) |
|---|---|---|
| earbuds (__ model) | __ ms | median __ ms, IQR __-__ ms (20 reps) |
| built-in speaker | __ ms | reference |
T0 -> T4 on earbuds: about __ ms at the median.

## d_route method
The iPhone schedules a 5 ms 1 kHz click at a known T3 and shows one white frame
aimed at the same host time; the iQOO films both at __ fps with an earbud held
to its microphone. d_route(earbuds) = median(flash->click, earbuds) -
median(flash->click, speaker) + outputLatency(speaker). Resolution ~ one
frame (__ ms). Script: tools/loopback.py.

## Per-cue chain
Every cue record in the dry run's ledger carries t0_ns to t3_ns. Three
representative cues (fast, median, slow), each with its sync bound:
- fast: cue __ (__): T0 -> T1 __ ms, T1 -> T2 __ us, T2 -> T3 __ ms; total __ ms +/- __ us
- median: cue __ (__): T0 -> T1 __ ms, T1 -> T2 __ us, T2 -> T3 __ ms; total __ ms +/- __ us
- slow: cue __ (__): T0 -> T1 __ ms, T1 -> T2 __ us, T2 -> T3 __ ms; total __ ms +/- __ us

## .dataPlayedBack agreement
Callback time minus computed end (T3 + dur + d_route), last 20 cues:
speaker __ to __ ms; earbuds __ to __ ms; cues beyond +/- 50 ms flagged: __.
outputLatency is not subtracted from .dataPlayedBack (decision 6).

## Limits
- One iPhone 12, one iQOO, one pair of earbuds, one room.
- p99 over ~60 cues is close to the maximum; per-packet numbers carry the variance.
- The bound assumes nothing about link symmetry; the true error is usually smaller.
- d_route is a median over 20 repetitions; Bluetooth routes can drift between sessions.
- Simulated sensors: the iQOO sends at exact rates; real meters jitter more.
