#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")/.."

SECS="${1:-600}"
mkdir -p build/fuzz-logs

run() {
    local dir="$1"
    local target="$2"
    local log="build/fuzz-logs/$target.log"

    if (
        cd "$dir" &&
        cargo +nightly fuzz run "$target" -- \
            -max_total_time="$SECS" \
            -rss_limit_mb=1024 \
            -print_final_stats=1
    ) > "$log" 2>&1; then
        echo "$target: ok"
    else
        echo "$target: FAILED — inspect $log"
    fi
}

for target in hrm cpm csc telem ctrl; do
    run crates/vc-gatt "$target"
done

run crates/vc-sim vcsim

grep -H "stat::number_of_executed_units" build/fuzz-logs/*.log || true
