# GoldenCheetah W′bal cross-check

The fact-check ledger listed GoldenCheetah's differential W′bal model as verified
only from secondary sources ("WPrime.cpp not opened"). This note records opening it
and what `vc-physio`'s `WbalGc` was compared against.

## The source

- File: `src/Metrics/WPrime.cpp` in <https://github.com/GoldenCheetah/GoldenCheetah>,
  `master` as fetched on 2026-10-14 (raw file, 49,533 bytes).
- SHA-256 of the fetched file: `f0713447cf8ff712f49b902f4b96c24ef72594136c586c6b00499669287b1c7c`.
- Commit hash of the file: **not recorded**. The GitHub API was not reachable from
  the session that fetched it. Fill it in from a clone with
  `git log -1 --format=%H -- src/Metrics/WPrime.cpp`.

## The rule, from the code (WPrime::setRide, lines 262-289)

```
// differential equation Froncioni / Clarke
double W = WPRIME;
for (int t=0; t<=last; t++) {
    int smoothedValue = smoothed.valueY(t);
    if (smoothedValue < CP) W = W + (CP-smoothedValue)*(WPRIME-W)/WPRIME;
    else                    W = W + (CP-smoothedValue);
    ...
    values[t] = W;
}
```

| Question | Answer from the code |
|---|---|
| Update below CP | `W += (CP - P) * (W' - W) / W'` per sample |
| Update at or above CP | `W += (CP - P)` (i.e. `W -= P - CP`) per sample |
| Time step | one step per sample of a 1 s series (gaps in the recording are zero-filled upstream, line 152) |
| Starting value | `W = WPRIME` |
| Capped at W′? | no explicit cap; the below-CP branch cannot exceed W′ from a start at W′ |
| Which power is applied | the sample at second `t`, for `t = 0 .. last` inclusive (the first sample is applied too) |
| Power type | `int smoothedValue = smoothed.valueY(t)`: the spline value, truncated toward zero to an integer |

This matches `gc_step_1s` in `crates/vc-physio/src/wbal.rs`, which is also what
`WbalGc` steps.

Two findings worth keeping:

1. **The differential form is not GoldenCheetah's default.** The branch is chosen by
   `appsettings->value(NULL, GC_WBALFORM, "int")` (line 99, and `GC_WBALFORM` in
   `src/Core/Settings.h`): the default is `"int"`, the Skiba integral model, with a
   TAU estimated from the ride. The differential model is the alternative, selected in
   GoldenCheetah's preferences. `WbalGc` matches the alternative.
2. **GoldenCheetah truncates power to an integer watt** before applying the update. On
   integer-watt data (a real ride export) that changes nothing; on the simulator's
   fractional watts it shifts the balance by up to ~180 J over a climb (see below).
   `WbalGc` does not truncate.

## The cross-check

`vc-cli power-trace` turns a simulated ride into a 1 Hz series (mean of each second's
four physics steps); `vc-cli wbal` runs `WbalGc` and `WbalExact` over it;
`tools/gc_wbal.py` is an independent Python port written from the rule above, not from
the Rust.

```
for s in climb_surge soak; do
  cargo run -q -p vc-cli -- power-trace --scenario $s > build/gc/$s-power.csv
  cargo run -q -p vc-cli -- wbal build/gc/$s-power.csv > build/gc/$s-ours.csv
  python3 tools/gc_wbal.py build/gc/$s-power.csv build/gc/$s-ours.csv
done
```

| Scenario | Seconds | port vs WbalGc (max) | port with `int()` truncation vs WbalGc | WbalExact vs WbalGc |
|---|---|---|---|---|
| climb_surge | 1,140 | 0.000000 J | 183.9 J | 31.0 J |
| soak | 3,600 | 0.000000 J | 152.5 J | 28.3 J |

Reading it:

- **port vs WbalGc, 0 J:** `WbalGc` computes GoldenCheetah's differential update, sample
  for sample, to floating-point precision. No alignment shift was needed: the simulator's
  first sample is under CP, so applying sample 0 (GoldenCheetah) or starting at sample 1
  (`WbalGc`'s clock) gives the same series.
- **`int()` column:** the cost of GoldenCheetah's integer power on non-integer input.
  It is a property of the input data, not a difference in the model.
- **WbalExact vs WbalGc, ~30 J:** the exact solution of the same differential equation
  against GoldenCheetah's one-second Euler step: the cost of GoldenCheetah's
  discretisation (Day 4 measured 38.9 J over a different hour; same order, ~0.15-0.2 %
  of W′).

## Not done

- Loading the same series into the GoldenCheetah desktop app (differential model, CP
  250 W, W′ 20 kJ) and screenshotting its W′bal chart beside the port's curve. That is
  a manual step; one picture of two independent tools agreeing is worth keeping for the
  README.
