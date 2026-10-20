# Engineering notes

## Architecture: sans-IO, one rule
Logic flows downward; only vc-ffi knows phones exist. vc-gatt, vc-physio and
vc-efficacy depend on no workspace crate; vc-core on those three; vc-sim on
vc-core; vc-ffi on vc-core and vc-sim; vc-report and vc-cli are std. The
phones own IO and threads; the core gets bytes and times and returns at most
one decision per call, which is why a ride replays identically on a Mac.

## The zero-allocation boundary
What runs per packet (decode, quality, W'bal, directives, policy, observer,
ledger push) is no_std with fixed-size arrays: tested with a counting
allocator (S10, E5). Serialisation happens in std code on a 1 Hz drain of a
256-record ring (decision 4). Cost: fixed capacities everywhere (64 steps,
256 records, 16 latencies) and a ring that can drop (it never has: dropped_before = 0).

## Determinism
One PRNG (PCG32 in vc-core::study, reference vectors in tests), libm for every
transcendental, no clock reads in the core, no hash-map iteration in output.
Evidence: golden logs (S5), byte-identical ledgers on Linux and macOS (E6),
phone captures replaying identically (gate G-11, the soak).

## UniFFI
0.32.2, proc macros, library mode. One Mutex per object (never contended:
one serial queue per phone). Errors in ingest become counters, never exceptions.
Stubs with final signatures froze FFI v0 on Day 8. Finding: bytes cross as `Data`
in Swift and `ByteArray` in Kotlin (a length-prefixed RustBuffer: one copy of 4-20 bytes,
not zero-copy as the plan hoped; measured inside T1->T2).

## Android rig (Vivo / OriginOS)
connectedDevice foreground service, permissions before startForeground;
services added one at a time (onServiceAdded), advertising after the last;
one notification in flight with a 500 ms watchdog; STAMP after each sensor
notification. Vivo: battery __, autostart __, recents lock __; screen off ->
__ (Day 11). 30-min and 60-min screen-on runs: __.

## iOS coach
One serial queue (ble.q) for CoreBluetooth, the Engine and audio scheduling;
T1 on the callback's first line; reconnect = connect again (no timeout);
write-without-response held until the peripheral is ready; a stalled tick
timer is backed up by packets.

## AVAudio semantics
Session .playback / .voicePrompt / .duckOthers, active all ride (no activation
delay before a safety cue; music stays ducked). Cues scheduled at an explicit
hostTime; onset and end computed as T3 (+ dur) + d_route; .dataPlayedBack
cross-checked, outputLatency not subtracted (decision 6); every clip carries a
generation so interrupted or reset clips are reported once, as failed.

## The observer
Anchor = the last complete second before onset (a mid-phrase reaction cannot
move the baseline); L_obs includes a derived 1.875 s smoothing lag ((N - 1)/2 · Ts + half a bin); any hole
in the data is a confounder; shams scored like cues; the surge's "go now" is
the only P1 cue observed (the tuner's T1 needs it).

**Open item (E1).** The observer as specified does not recover the known rider in E1
(compliance 0.28-0.38 against 0.70): `reversed` is decided on one hold bin while
`complied` needs three, the cardiac-drift cue is counted though no persona reacts to it,
and the measured smoothing lag is nearer 1.5 s than 2.0 s. See docs/EFFICACY.md.

## The tuner
Its type holds four delivery parameters and nothing else; safety is out of
reach by construction (E3) and the ranges are fixed (E4); frozen in Study.

## Fuzzing
S4, measured 2026-10-16 with `scripts/fuzz-all.sh 600` (cargo-fuzz, libFuzzer, rustc 1.101.0-nightly,
Linux x86_64, commit `5e82b08`; vcsim re-run separately after the script was edited mid-run).
Each target ran for 601 s, `-rss_limit_mb=1024`. **0 crashes, 0 timeouts, 0 OOMs.**

| Target | Decoder | Executions in 601 s |
|---|---|---|
| hrm | Heart Rate Measurement | 231.6 M |
| cpm | Cycling Power Measurement | 162.1 M |
| csc | Cycling Speed and Cadence | 299.7 M |
| telem | rig telemetry (STAMP, PONG, STATUS, RIDER_INPUT) | 305.3 M |
| ctrl | rig control (PING … CUE_PLAYED) | 296.0 M |
| vcsim | the simulator driven by arbitrary CTRL sequences | 0.49 M (each input steps a whole simulator) |

CI runs the same targets for 60 s each on every push ("Fuzz smoke tests"). Earlier records:
ctrl 471.5 M executions in 10 minutes (Day 3), 35.1 M across targets (Day 7).

## Decisions and deviations
| Decision | Why | Cost |
|---|---|---|
| LOAD carries persona + human (9 B) | the rig must know who rides | one Day 3 layout changed |
| PCG32 moved to Day 7 | one PRNG for the simulator and the study | — |
| Rider stepped at 4 Hz | responses visible within 250 ms (E1 +/-0.5 s) | 4x physiology steps |
| Auto-pause without speed | no wheel sensor | power < 20 W + cranks stopped instead |
| Tcore > 39.5 C not implemented | no sensor | documented only |
| CADENCE_LOW is not a power ask | its effect is cadence | not scored by the observer |
| "Soft-pedal" hyphenated | verb-first lint | — |
| Tick-released cues have no T0 | no triggering packet | reported as T1->T3 |
| The rig's rider switch wins over LOAD | the person holding the slider decides | — |
| E2 rides in Rolling | 10 s of history before the cue | — |
| Reversed on one hold bin (as specified) | the spec's `H = min` is reused for both verdicts | E1 open: see docs/EFFICACY.md |

## Findings from the build days (Days 4-7)

### Day 4 — W′ Balance & Microcontroller Trade-offs

#### 1. GoldenCheetah Parity Measurement
- **Test**: `exact_and_gc_agree_within_half_a_percent_over_an_hour` (20 min @ 200 W, 5 × [3 min @ 330 W + 3 min @ 150 W], 10 min @ 150 W at 1 Hz).
- **Measured Worst Difference**: **38.917 J** (~0.19% of $W'$ capacity of 20,000 J).
- **Acceptance Threshold**: Below 100 J (0.5% of $W'$).
- **Analysis**: The differential exponential model (`WbalExact`) and GoldenCheetah's 1-second Euler stepping model (`WbalGc`) agree within 38.9 J over a full 1-hour ride. The slight gap arises because Euler steps recover ~0.2% more per 3-minute rest interval, with the difference decaying during subsequent work intervals.

#### 2. Cortex-M4F Single-Precision FPU & `f64` Trade-off
- **Observation**: The Cortex-M4F microcontroller target (`thumbv7em-none-eabihf`) features a hardware single-precision (32-bit) floating point unit (FPU) only. Double-precision (`f64`) operations run entirely in software emulation via compiler intrinsics.
- **Architectural Decision**: `f64` is retained in `vc-physio` because:
  1. The primary execution target is modern mobile hardware (iOS and Android devices), where 64-bit hardware FPUs are standard and instantaneous.
  2. Compiling for `thumbv7em-none-eabihf` verifies strict `#![no_std]` compliance and architectural portability.
  3. If targeted to resource-constrained microcontrollers (e.g. nRF52-class smart displays or sensor pods), the mathematical models can be made generic over `f32`, or `f64` can be retained at the acceptable cost of tens of microseconds per periodic update.

### Day 5 — Core Engine, Safety & Workout State Machine

#### 1. Auto-Pause Speed Proxy Substitution
- **Specification Rule**: the auto-pause rule specifies cadence = 0 RPM and speed < 2 km/h for > 3 s.
- **Hardware Constraint & Substitution**: There is no dedicated wheel-speed sensor in this rig. Missing CPM packets alone cannot indicate a rider has stopped (that could be a sensor dropout).
- **Implementation**: The Engine substitutes "power Fresh and under 20 W, cranks stopped" for > 3 s (`auto_pause_ns`). While auto-paused, the workout active clock halts and only P1 directives (safety, dropout) remain active.

#### 2. Core Temperature Safety Rule
- **Observation**: the original safety rule includes a hard cutoff for core body temperature $T_{\text{core}} > 39.5\ ^\circ\text{C}$.
- **Decision**: Documented but not implemented in `RedlineGate` because standard cycling BLE sensors (HRM, CPM) do not measure or transmit core body temperature telemetry.

### Day 6 — Policy Engine, Phrasing & Golden Logs

#### 1. Safety-Critical Logic in Code, Not Configuration
- **Design Principle**: "P1 is never muted" and "P1 may interrupt P2/P3 clips" are hardcoded branches in `Policy::decide`, not configurable struct fields. No profile or tuner can override life-safety alerts.
- **Tunable vs Invariable**: Only global cooldown (clamped to 15–45 s), surge lead (5–8 s), optional P3 earcons, and default P2 phrasing arm are exposed to the tuner.

#### 2. Clip End Alignment: Cooldown & Observer Window
- **Audio Timing**: Both cooldown and observer window measure from the true end of playback. Until `on_audio_finished` fires, the Policy projects completion via `est_phrase_ns` (2.5 s).
- **Window Interlock**: At the default 20 s cooldown and 15 s window, cooldown outlasts the window. If the tuner lowers cooldown to 15 s, the window guarantees a P2 cue is never graded while another cue plays.

#### 3. Phrasing Conventions & Lint Rules
- **Verb-First & Hyphenation**: "Soft-pedal" is explicitly hyphenated to serve as an imperative verb compound, satisfying the strict lint requirement for a verb as the initial token.
- **Surge Countdown Variants**: Distinct recordings exist for leads 5, 6, 7, and 8 seconds (`SRG5`–`SRG8`) so the spoken number matches `promptSurgeCountdownSec`.
- **W′ Re-arm Interlock**: In continuous hard efforts, the 20 s cooldown blocks a second $W'$ cue before hysteresis re-arms.

### Day 7 — Controllable Virtual Rider, Scenarios & Fuzzing

#### 1. Protocol Evolution & Payload Compactness
- **LOAD v2**: Refactored to 9 bytes containing `scenario (u8)`, `persona (u8)`, `seed (u32)`, and `human (bool)`. The decoder enforces `persona <= PERSONA_MAX (4)` and strict boolean mapping for `human`.
- **Rider Input & Cue Played**: `RIDER_INPUT` (14 B) timestamps slider efforts on the mobile clock, and `CUE_PLAYED` (15 B) feeds cue audio playback schedules to the virtual rider.

#### 2. Reproducibility & Independent PRNG Streams
- **Seeded Randomness**: Random number generation is consolidated entirely in `vc_core::study::Pcg32`.
- **Stream Independence**: Rider physiological noise and cue reaction drawings use separate streams (`CUE_STREAM_BASE | cue_id`), guaranteeing that injecting faults or reacting to cues never shifts the rider's physiological noise sequence.

#### 3. 4 Hz Virtual Rider & Scenario Harness
- **Temporal Resolution**: The virtual rider steps every 250 ms (4 Hz) to allow simulated compliance within $\pm 0.5$ s of audio completion.
- **Headless Clock**: The `vc-cli` test harness drives the entire closed loop on a virtual clock without radio or sleeping, running a 20-minute workout simulation in tens of milliseconds.
