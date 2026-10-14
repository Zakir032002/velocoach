
#!/usr/bin/env bash
set -uo pipefail

cd "$(dirname "$0")/.."

SECS="${1:-600}"
LOG_DIR="build/fuzz-logs"
mkdir -p "$LOG_DIR"

failed=0

run() {
    local dir="$1"
    local target="$2"
    local log="$LOG_DIR/$target.log"

    echo "=== Fuzzing $target for up to ${SECS}s ==="

    if (
        cd "$dir" &&
        cargo +nightly fuzz run "$target" -- \
            "-max_total_time=$SECS" \
            -rss_limit_mb=1024 \
            -print_final_stats=1
    ) >"$log" 2>&1; then
        if grep -q "CRASH" "$log"; then
            echo "$target: FAILED — crash marker found; inspect $log"
            failed=1
        else
            echo "$target: OK"
        fi
    else
        echo "$target: FAILED — inspect $log"
        failed=1
    fi
}

for target in hrm cpm csc telem ctrl; do
    run crates/vc-gatt "$target"
done

run crates/vc-sim vcsim

echo
echo "=== Fuzz execution counts ==="
grep -H "stat::number_of_executed_units" "$LOG_DIR"/*.log || true

exit "$failed"
