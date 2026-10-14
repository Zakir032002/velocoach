# velocoach-rs

**A closed-loop voice coach in Rust that measures whether its own cues work.**

iPhone 12 (CoreBluetooth central, the coach) <-> iQOO (BLE peripheral running a
simulated rider). No real sensors: the rider, the power meter and the heart-rate
strap are simulated over real Bluetooth.

- **A Policy Engine with Aki Kakko's profiles** (Solo / Rolling / Group) and parameter
  names, from his Knowledge Hub; earcon first, then speech.
- **Every cue gets a verdict** (complied, ignored, confounded...) from the power
  stream alone, and the safety gate is out of reach of the adaptive code:
  property-tested ([E3](docs/EFFICACY.md#e3e4-the-tuners-reach)). The observer's own
  validation against a known rider ([E1](docs/EFFICACY.md#e1-the-observer-recovers-a-known-rider--open))
  is **open**.
- **T0->T3 p99 = __ ms (+/- __ ms clock-sync bound)** · T1->T2 p99 = __ us ·
  **0 allocations per packet** ([LATENCY.md](docs/LATENCY.md)).
- **A feasibility pilot** (slider proxy, not cycling; n = __ decision points from
  __ people): [STUDY.md](STUDY.md). Not evidence of efficacy.

[3-minute video](__) · [LATENCY.md](docs/LATENCY.md) · [EFFICACY.md](docs/EFFICACY.md) ·
[STUDY.md](STUDY.md) · [ENGINEERING_NOTES.md](docs/ENGINEERING_NOTES.md) ·
[THIRD_PARTY.md](THIRD_PARTY.md)

## How it works

A sans-IO Rust core: it receives bytes and times, and returns at most one
decision per call. The phones own the radio, the threads and the audio.

    vc-gatt     GATT decoders (HRM, CPM, CSC) + the rig protocol      no_std
    vc-physio   W'bal (exact + GoldenCheetah-parity), quality, cadence no_std
    vc-efficacy the observer, the ledger, the bounded tuner           no_std
    vc-core     the Engine: directives, Policy Engine, phrases, study  no_std
    vc-sim      the simulated rider and its personas                  no_std
    vc-ffi      UniFFI bindings for Swift and Kotlin
    vc-harness  the headless closed loop (Engine + simulator)         std
    vc-report   JSONL ledger, statistics, the HTML report card        std
    vc-cli      headless rides, golden logs, replay                   std

**Directives -> Policy Engine -> cue.** Eight directives (W'bal low, soft-pedal,
grey-zone drift, cadence, cardiac drift, safety, sensor dropout, surge) propose
cues. The Policy Engine applies the profile's rules: priority, cooldown from the
end of the last clip, a P1 that may interrupt anything, and an observation
window during which nothing else is said.

**The observer.** After a cue, it compares the rider's power with where it was
heading before the cue (a 10-second pre-trend) and asks whether it moved by at
least half the requested amount, and held, within 15 seconds. Anything that
could explain the change (a gradient change, a dropout, a safety cue, coasting)
makes the verdict "confounded" instead of a guess. Shams (a cue decided but not
played) are scored the same way, which gives the observer's false-positive rate.

**The tuner.** In Coach mode, at step boundaries, it may move Aki's delivery
parameters inside his ranges (cooldown 15-45 s, countdown lead 5-8 s) and switch
a phrasing that clearly works better. Its type cannot hold anything
safety-critical. In Study mode it is frozen.

## Numbers

| What | Result | Source |
|---|---|---|
| Decision time in Rust (T1->T2), iPhone 12 | p99 __ us | LATENCY.md S1 |
| Sensor send -> audio scheduled (T0->T3) | p99 __ ms +/- __ ms | LATENCY.md S2 |
| Route to the ear, earbuds (loopback) | median __ ms (IQR __-__) | LATENCY.md S3 |
| Allocations per packet / per observer update | 0 / 0 | S10, E5 |
| Observer recovers a known rider (E1, 20 seeds) | **open**: 0 / 20 seeds in band (compliance 0.28-0.38 against 0.70 +/- 0.08) | EFFICACY.md |
| Every confounder caught through the Engine (E2) | 6 / 6, control not confounded | EFFICACY.md |
| Statistics with a null rider (E8) | 95 % CI covers 0 in 190 / 200 | EFFICACY.md |
| Fuzzing, 6 decoders | __ M executions, 0 crashes | ENGINEERING_NOTES.md |
| Soak with scripted faults, 60 min x 2 | __ | ENGINEERING_NOTES.md |
| Same seed, byte-identical ledger | Linux and macOS, in CI | E6 |
| W'bal against GoldenCheetah's differential model | independent port: 0.000000 J difference | docs/WPRIME_GC.md |

## The pilot

Feasibility pilot · slider proxy, not cycling · personas are assumptions.
n = __ decision points from __ people. Cue vs sham: complied __ vs __
(pooled within-person difference __, 95 % CI __ to __). Autonomy vs direct
phrasing: __ (95 % CI __ to __). Not powered to detect differences under
about 0.2. Pre-registered in [STUDY.md](STUDY.md) before the first session;
full results, deviations and limitations there.

## Reproduce

    cargo test --workspace --all-features
    cargo run -p vc-cli -- golden --check                  # five golden rides, byte-identical
    cargo test --release -p vc-harness --test e1 -p vc-report --test e8
    cargo run -p vc-cli -- sim --scenario climb_surge      # a ride, headless
    cargo run -p vc-report -- data/study/*.jsonl -o results.html   # the published figures

(`e1` fails today; see EFFICACY.md.) Phones: `scripts/build-ios.sh` and
`scripts/build-android.sh`, then open `apps/ios` in Xcode and `apps/android` in
Android Studio. The operating procedure for the pilot is [docs/RUNBOOK.md](docs/RUNBOOK.md).

## Limits

- Simulated sensors, one iPhone, one Android phone, one pair of earbuds.
- A slider is not a pedal: the pilot measures response to a cue, not riding.
- Compliance is not benefit.
- The observer's latency includes about 2 s of smoothing lag (documented as
  `SMOOTHING_LAG_MS`).
- The observer's validation against a known rider (E1) is open.

## What I'd do next

1. Real riders on real trainers, with the same ledger schema (nothing in the
   observer assumes a simulator).
2. A grade source on the phone (barometer + GPS), so the grade confounder works
   outdoors as it does on the Mac.
3. Per-rider ledgers that persist across rides, so the tuner learns a person.
4. Off-policy evaluation (IPS/SNIPS, built in vc-report) on a study large
   enough for it to say something.
5. FIT file replay of recorded rides through the same Engine.

## Credits

Policy Engine profiles, parameter names, earcon table and phrase rules: Aki
Kakko's Knowledge Hub (posts: __). W'bal: Skiba's differential model as
implemented in GoldenCheetah (cross-checked in docs/WPRIME_GC.md). Voice: Piper
(__ voice, __ licence). See THIRD_PARTY.md.

## License

MIT OR Apache-2.0, at your option: [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE).

## Appendix: the VC-SIM rig protocol

The VCSIM protocol enables closed-loop simulation over BLE GATT characteristics. All messages start with a 1-byte version (`0x01`) and a 1-byte opcode.

### Telemetry (VCSIM_TELEM)

| Message | Opcode | Size | Description |
| :--- | :--- | :--- | :--- |
| `STAMP` | `0x01` | 15 B | High-resolution hardware time sync |
| `PONG` | `0x02` | 20 B | Latency pong response |
| `STATUS` | `0x03` | 12 B | Simulator state and active fault bitmask |
| `RIDER_INPUT` | `0x04` | 14 B | Human slider effort timestamped on mobile clock |

### Control (VCSIM_CTRL)

| Message | Opcode | Size | Description |
| :--- | :--- | :--- | :--- |
| `PING` | `0x10` | 12 B | Ping request |
| `LOAD` | `0x20` | 9 B | Load scenario, persona (0–4), seed, and human mode |
| `START` | `0x21` | 2 B | Start simulation run |
| `STOP` | `0x22` | 2 B | Stop simulation run |
| `FAULT` | `0x30` | 10 B | Inject fault (dropout, garbage, coast, redline, disconnect, $W'$ mismatch) |
| `CLEAR` | `0x31` | 3 B | Clear faults (`0xFF` clears all) |
| `RATE` | `0x40` | 4 B | Sensor decimation rate setting |
| `CUE_PLAYED` | `0x50` | 15 B | Cue scheduled/played audio event (ask, arm, timing) |
