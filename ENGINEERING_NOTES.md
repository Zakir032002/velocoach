# Engineering Notes

## Day 4 — W′ Balance & Microcontroller Trade-offs

### 1. GoldenCheetah Parity Measurement
- **Test**: `exact_and_gc_agree_within_half_a_percent_over_an_hour` (20 min @ 200 W, 5 × [3 min @ 330 W + 3 min @ 150 W], 10 min @ 150 W at 1 Hz).
- **Measured Worst Difference**: **38.917 J** (~0.19% of $W'$ capacity of 20,000 J).
- **Acceptance Threshold**: Below 100 J (0.5% of $W'$).
- **Analysis**: The differential exponential model (`WbalExact`) and GoldenCheetah's 1-second Euler stepping model (`WbalGc`) agree within 38.9 J over a full 1-hour ride. The slight gap arises because Euler steps recover ~0.2% more per 3-minute rest interval, with the difference decaying during subsequent work intervals.

### 2. Cortex-M4F Single-Precision FPU & `f64` Trade-off
- **Observation**: The Cortex-M4F microcontroller target (`thumbv7em-none-eabihf`) features a hardware single-precision (32-bit) floating point unit (FPU) only. Double-precision (`f64`) operations run entirely in software emulation via compiler intrinsics.
- **Architectural Decision**: `f64` is retained in `vc-physio` because:
  1. The primary execution target is modern mobile hardware (iOS and Android devices), where 64-bit hardware FPUs are standard and instantaneous.
  2. Compiling for `thumbv7em-none-eabihf` verifies strict `#![no_std]` compliance and architectural portability.
  3. If targeted to resource-constrained microcontrollers (e.g. nRF52-class smart displays or sensor pods), the mathematical models can be made generic over `f32`, or `f64` can be retained at the acceptable cost of tens of microseconds per periodic update.

## Day 5 — Core Engine, Safety & Workout State Machine

### 1. Auto-Pause Speed Proxy Substitution
- **Specification Rule**: Aki's auto-pause rule specifies cadence = 0 RPM and speed < 2 km/h for > 3 s.
- **Hardware Constraint & Substitution**: There is no dedicated wheel-speed sensor in this rig. Missing CPM packets alone cannot indicate a rider has stopped (that could be a sensor dropout).
- **Implementation**: The Engine substitutes "power Fresh and under 20 W, cranks stopped" for > 3 s (`auto_pause_ns`). While auto-paused, the workout active clock halts and only P1 directives (safety, dropout) remain active.

### 2. Core Temperature Safety Rule
- **Observation**: Aki's original safety rule includes a hard cutoff for core body temperature $T_{\text{core}} > 39.5\ ^\circ\text{C}$.
- **Decision**: Documented but not implemented in `RedlineGate` because standard cycling BLE sensors (HRM, CPM) do not measure or transmit core body temperature telemetry.
