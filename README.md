# velocoach-rs

A zero-cost, sensor-free, real-time cycling voice coach written in Rust.

## Architecture

`velocoach-rs` is structured as a collection of decoupled, deterministic `#![no_std]` crates:

- **`vc-gatt`**: BLE GATT packet decoders/encoders (HRM, CPM, CSC) and VCSIM wire protocol.
- **`vc-physio`**: Physiological estimation ($W'$ balance differential exponential model, cadence tracking, quality states).
- **`vc-core`**: Core engine, state machine, safety supervisor, directives, and policy decision engine.
- **`vc-sim`**: Deterministic virtual rider (4 Hz step), 6 scenarios, 4 personas, fault injection, and BLE packet generator.
- **`vc-cli`**: Command-line simulation runner, virtual clock harness, and golden log verification.

## VCSIM Protocol Specification

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
