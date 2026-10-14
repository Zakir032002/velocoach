#!/usr/bin/env python3
"""An independent port of GoldenCheetah's differential W'bal (src/Metrics/WPrime.cpp,
WPrime::setRide, lines 262-289 of master as fetched 2026-10-14; see docs/WPRIME_GC.md
for the file's SHA-256) to cross-check vc-physio. The rule was written from the C++
before this was run.

usage: gc_wbal.py power.csv ours.csv [--cp 250 --wprime 20000]
"""
import csv
import sys

CP, WPRIME = 250.0, 20_000.0


def gc_differential(powers, truncate=False):
    # WPrime.cpp: `int smoothedValue = smoothed.valueY(t);` then, for t = 0..last,
    #   P < CP:  W = W + (CP - P) * (WPRIME - W) / WPRIME
    #   else:    W = W + (CP - P)
    # starting at W = WPRIME, one step per second, the sample's own power, no cap.
    w, out = WPRIME, []
    for p in powers:
        if truncate:
            p = int(p)  # C++ int conversion truncates toward zero
        if p < CP:
            w = w + (WPRIME - w) * (CP - p) / WPRIME
        else:
            w = w - (p - CP)
        out.append(w)
    return out


def main(power_csv, ours_csv):
    powers = [float(r["power_w"]) for r in csv.DictReader(open(power_csv))]
    ours = list(csv.DictReader(open(ours_csv)))
    gc = gc_differential(powers)
    gc_int = gc_differential(powers, truncate=True)
    n = min(len(gc), len(ours))
    d_gc = [abs(gc[i] - float(ours[i]["wbal_gc_j"])) for i in range(n)]
    d_int = [abs(gc_int[i] - float(ours[i]["wbal_gc_j"])) for i in range(n)]
    d_ex = [abs(float(ours[i]["wbal_exact_j"]) - float(ours[i]["wbal_gc_j"])) for i in range(n)]
    print(f"{n} seconds")
    print(f"port vs WbalGc:     max |diff| {max(d_gc):.6f} J at t = {d_gc.index(max(d_gc))} s")
    print(f"port with GC's int() truncation of power vs WbalGc: max |diff| {max(d_int):.3f} J at t = {d_int.index(max(d_int))} s")
    print(f"WbalExact vs WbalGc: max |diff| {max(d_ex):.3f} J at t = {d_ex.index(max(d_ex))} s")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
