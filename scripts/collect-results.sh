#!/usr/bin/env bash
# After a phone ride: pull every file the iPhone app wrote, check it, and build the report.
#
#   scripts/collect-results.sh                 copy from the connected iPhone (devicectl)
#   scripts/collect-results.sh --from DIR      use files you copied yourself (Finder > iPhone >
#                                              Files > VeloCoach > captures, or AirDrop)
#
# Output: results/<UTC stamp>/ with the raw files, the rig's log (if the iQOO is on USB), the
# replay check, the soak check, the audit, report.html and SUMMARY.md. Everything there is
# SIMULATED sensor data over real BLE (hardware-in-the-loop), not rider data.
set -euo pipefail
trap 'echo >&2; echo "FAILED (line $LINENO): $BASH_COMMAND" >&2' ERR

cd "$(dirname "$0")/.."

BUNDLE_ID="dev.zakir.velocoach.coach"
FROM=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --from) FROM="${2:?--from needs a directory}"; shift ;;
        -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
        *) echo "unknown option: $1 (try --help)" >&2; exit 2 ;;
    esac
    shift
done

run() { printf '+ %s\n' "$*"; "$@"; }
die() { echo "ERROR: $*" >&2; exit 1; }

OUT="results/$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$OUT/raw"

if [[ -n "$FROM" ]]; then
    [[ -d "$FROM" ]] || die "no such directory: $FROM"
    run cp -R "$FROM"/. "$OUT/raw/"
else
    [[ "$(uname)" == Darwin ]] || die "copying from the iPhone needs macOS; use --from DIR"
    JSON="$OUT/devices.json"
    run xcrun devicectl list devices --json-output "$JSON" >/dev/null
    UDID="$(python3 - "$JSON" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
ids = [x["hardwareProperties"]["udid"] for x in d["result"]["devices"]
       if x.get("hardwareProperties", {}).get("platform") == "iOS"
       and x.get("connectionProperties", {}).get("pairingState") == "paired"]
print(ids[0] if len(ids) == 1 else "")
EOF
)"
    [[ -n "$UDID" ]] || die "expected exactly one paired iPhone (or use --from DIR)"
    run xcrun devicectl device copy from --device "$UDID" \
        --domain-type appDataContainer --domain-identifier "$BUNDLE_ID" \
        --source Documents/captures --destination "$OUT/raw"
fi

# The rig's log, when the iQOO is still on USB.
ADB="${ANDROID_HOME:-$(brew --prefix 2>/dev/null)/share/android-commandlinetools}/platform-tools/adb"
if [[ -x "$ADB" ]] && "$ADB" get-state >/dev/null 2>&1; then
    "$ADB" logcat -d -s vc-rig > "$OUT/rig-logcat.txt" || true
    cp build/machine/android-device.txt "$OUT/" 2>/dev/null || true
fi
cp build/machine/mac.txt "$OUT/" 2>/dev/null || true

shopt -s nullglob
CAPTURES=("$OUT"/raw/*.vcraw)
LEDGERS=("$OUT"/raw/*.jsonl)
SOAKS=("$OUT"/raw/*.csv)
(( ${#CAPTURES[@]} + ${#LEDGERS[@]} > 0 )) || die "no .vcraw or .jsonl files in $OUT/raw"

run cargo build -q --release -p vc-cli -p vc-report
CLI=target/release/vc-cli
REPORT=target/release/vc-report

{
    echo "# Ride results $(basename "$OUT")"
    echo
    echo "**SIMULATED sensors** over real BLE between two phones (hardware-in-the-loop), or"
    echo "in-process on the iPhone (L1) where the ledger's data label says link=in-process."
    echo "No real rider or sensor data. Earbud output latency: not measured."
    echo
    echo "Commit: $(git rev-parse --short HEAD) · observer: $(grep -o 'vc-efficacy [0-9.]*' crates/vc-efficacy/src/lib.rs | head -1)"
    echo
    echo "## Replay (S5 on device: the captured bytes give the same decisions)"
    echo
    for f in "${CAPTURES[@]}"; do
        if r="$("$CLI" replay "$f" 2>&1)"; then echo "- PASS $(basename "$f"): $r"; else echo "- **FAIL** $(basename "$f"): $r"; fi
    done
    if (( ${#SOAKS[@]} > 0 )); then
        echo
        echo "## Soak (S6)"
        echo
        for f in "${SOAKS[@]}"; do
            echo '```'
            "$CLI" soak-check "$f" 2>&1 || true
            echo '```'
        done
    fi
    if (( ${#LEDGERS[@]} > 0 )); then
        echo
        echo "## Ledger audit and report"
        echo
        echo '```'
        "$REPORT" "${LEDGERS[@]}" -o "$OUT/report.html" --title "Phone ride (SIMULATED sensors)" --audit 2>&1 || true
        echo '```'
        echo
        echo "Latency figures (T1->T2, T0->T3 with the clock-sync bound) are on the app's latency"
        echo "panel and in the ledger's t0_ns..t3_ns and clock_sync_error_bound_us fields."
        echo "Report: [report.html](report.html)"
    fi
} > "$OUT/SUMMARY.md"

cat "$OUT/SUMMARY.md"
echo
echo "OK: $OUT (copy the numbers into docs/LATENCY.md and the README status table)."
