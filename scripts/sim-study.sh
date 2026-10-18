#!/usr/bin/env bash
# The pre-registered study protocol (STUDY.md) run end to end in SOFTWARE, with simulated
# personas standing in for people. This is a dry run of the pipeline (allocation, frozen tuner,
# ledger, report), not human data and not evidence of efficacy. Human pilot: planned,
# pre-registered, not yet run.
#
# Riders SIM1-SIM3 = personas dana, sam, ned; 3 sessions each on creep_pilot; session seed =
# 1000 x rider number + session number, as STUDY.md specifies for people.
# Output: data/sim-study/*.jsonl and docs/sim-study/report.html (CI regenerates and compares).
set -euo pipefail
cd "$(dirname "$0")/.."

out=data/sim-study
rm -rf "$out"
mkdir -p "$out" docs/sim-study
cargo build -q -p vc-cli -p vc-report
personas=(dana sam ned)
for r in 1 2 3; do
    for s in 1 2 3; do
        seed=$((1000 * r + s))
        target/debug/vc-cli sim --scenario creep_pilot --mode study --persona "${personas[r - 1]}" \
            --seed "$seed" --rider "SIM$r" --jsonl "$out/SIM${r}_s${s}.jsonl" >/dev/null
    done
done
target/debug/vc-report "$out"/*.jsonl -o docs/sim-study/report.html \
    --title "Simulated study run (software only; not human data)" --audit
