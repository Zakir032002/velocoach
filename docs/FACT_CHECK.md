# Fact-check ledger

EFFICACY.md, STUDY.md and the README rely on a handful of outside facts. One line per
fact; anything still marked secondary stays marked secondary in the documents. "Secondary,
and here is our own measurement" is a stronger sentence than an unverified "verified".
Back to the [README](../README.md).

| Fact | Used in | Status | Checked |
|---|---|---|---|
| Aki's Policy Engine parameter names and ranges | vc-core config, README | **to verify at source** (post: __) | 2026-10-__ |
| Aki's earcon table and phrase rules | phrase bank, lint | **to verify at source** (post: __) | 2026-10-__ |
| GoldenCheetah differential W'bal | vc-physio WbalGc | **verified in WPrime.cpp** (`WPrime::setRide`, lines 262-289; file SHA-256 f0713447...b1c7c; commit hash not recorded): docs/WPRIME_GC.md. GoldenCheetah's *default* form is the Skiba integral (`GC_WBALFORM` default `"int"`); the differential model is the alternative setting | 2026-10-14 |
| GoldenCheetah truncates power to an integer watt before the update | WPRIME_GC.md | verified in WPrime.cpp line 274 (`int smoothedValue = ...`) | 2026-10-14 |
| Zak et al. 2023, Sensors 23(8):3898 (simple visual reaction with a brake-lever response, median 0.348 s riding) | STUDY.md, EFFICACY.md | **to verify at source** | 2026-10-__ |
| Buchholtz & Burgess 2020, S Afr J Sports Med 32(1) (online simple reaction while cycling, medians 0.37 s males, 0.42 s females) | STUDY.md, EFFICACY.md | **to verify at source** | 2026-10-__ |
| AirPods latency around 160 ms (secondary report) | LATENCY.md context | secondary; our own earbuds measured at __ ms by loopback | 2026-10-__ |
| AVAudioPlayerNode completion semantics (`.dataPlayedBack`) | AudioCuePlayer, LATENCY.md | Apple documentation | 2026-10-__ |

Re-check Aki's Knowledge Hub and domesly.com for anything new since Oct 11 (the risk
register's "a new post introduces per-cue compliance"): wording changes only, nothing in the
code or the study changes because of what a blog says.
