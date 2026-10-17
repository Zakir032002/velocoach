#!/usr/bin/env bash
# One command for the iPhone coach app: Rust core -> XCFramework -> Xcode project -> phone.
#
#   scripts/setup-ios.sh                   build the core, generate the project, open Xcode
#   scripts/setup-ios.sh --install         ... and build, sign, install and launch on the
#                                          connected iPhone from the command line
#   scripts/setup-ios.sh --no-background   leave out UIBackgroundModes (use if signing rejects
#                                          them; then keep the app in the foreground)
#
# macOS with Xcode (not only the Command Line Tools) and Homebrew. Every install command is
# printed before it runs, done steps are skipped (safe to re-run), and any failure stops the
# script with the command that failed. It installs only: xcodegen and the two Rust iOS targets.
# Signing: team 42H9VCQ78N (free personal team), bundle id dev.zakir.velocoach.coach, no
# entitlement-backed capability.
set -euo pipefail
trap 'echo >&2; echo "FAILED (line $LINENO): $BASH_COMMAND" >&2; echo "Nothing after this step ran. Fix it and re-run the script." >&2' ERR

cd "$(dirname "$0")/.."

BUNDLE_ID="dev.zakir.velocoach.coach"
INSTALL=0
SPEC="project.yml"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --install) INSTALL=1 ;;
        --no-background) SPEC="project.nobg.yml" ;;
        -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
        *) echo "unknown option: $1 (try --help)" >&2; exit 2 ;;
    esac
    shift
done

say() { printf '\n== %s\n' "$*"; }
run() { printf '+ %s\n' "$*"; "$@"; }
die() { echo "ERROR: $*" >&2; exit 1; }

[[ "$(uname)" == Darwin ]] || die "the iPhone app builds on macOS only"
command -v brew >/dev/null || die "Homebrew is required (https://brew.sh). It is not installed by this script."

say "Xcode"
DEV_DIR="$(xcode-select -p 2>/dev/null || true)"
[[ "$DEV_DIR" == *Xcode*.app/* ]] || die "full Xcode is required (App Store), then: sudo xcode-select -s /Applications/Xcode.app ; found '$DEV_DIR'"
run xcodebuild -version

say "XcodeGen"
if ! command -v xcodegen >/dev/null; then
    run brew install xcodegen
fi

say "Rust targets"
command -v rustup >/dev/null || die "rustup is required (https://rustup.rs); it is not installed by this script"
for t in aarch64-apple-ios aarch64-apple-ios-sim; do
    if ! rustup target list --installed | grep -qx "$t"; then
        run rustup target add "$t"
    fi
done

say "Cue bank"
WAVS="$(find assets/cues -name '*.wav' | wc -l | tr -d ' ')"
(( WAVS > 0 )) || die "assets/cues has no WAVs: git pull (CI renders and commits them), or run scripts/render_cues.sh"
echo "ok: $WAVS WAV files in assets/cues"

say "Rust core -> VcFFI.xcframework + Swift bindings"
run bash scripts/build-ios.sh

say "Xcode project ($SPEC)"
if [[ "$SPEC" == project.nobg.yml ]]; then
    grep -v 'UIBackgroundModes' apps/ios/project.yml > apps/ios/project.nobg.yml
fi
(cd apps/ios && run xcodegen generate --spec "$SPEC")

mkdir -p build/machine
{
    echo "macOS: $(sw_vers -productVersion) ($(uname -m))"
    echo "xcode: $(xcodebuild -version | tr '\n' ' ')"
    echo "rustc: $(rustc --version)"
} > build/machine/mac.txt

if [[ "$INSTALL" == 0 ]]; then
    run open apps/ios/VeloCoach.xcodeproj
    cat <<EOF

OK: the project is open in Xcode. To put it on the iPhone (first time only):
  1. Xcode > Settings > Accounts: sign in with the Apple ID of team 42H9VCQ78N.
  2. Plug in the iPhone, unlock it, tap "Trust". Choose it as the run destination (top bar).
  3. Press Run (Cmd-R). The first time, on the iPhone: Settings > General > VPN & Device
     Management > your Apple ID > Trust; and Settings > Privacy & Security > Developer Mode ON
     (the phone restarts).
  4. If signing complains about background modes: re-run with --no-background.
Free-team profiles expire after 7 days: press Run again to renew.
Or run this script with --install to do steps 2-3 from the command line.
EOF
    exit 0
fi

say "iPhone"
JSON="build/machine/devices.json"
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
[[ -n "$UDID" ]] || die "expected exactly one paired iPhone: plug it in, unlock it, tap Trust, enable Developer Mode (Settings > Privacy & Security), then re-run"
echo "iPhone: $UDID"

say "Build and sign (needs the Apple ID signed in to Xcode > Settings > Accounts)"
run xcodebuild -project apps/ios/VeloCoach.xcodeproj -scheme VeloCoach -configuration Debug \
    -destination "id=$UDID" -derivedDataPath build/ios-dd -allowProvisioningUpdates build

say "Install and launch"
run xcrun devicectl device install app --device "$UDID" build/ios-dd/Build/Products/Debug-iphoneos/VeloCoach.app
if ! run xcrun devicectl device process launch --device "$UDID" "$BUNDLE_ID"; then
    echo "Launch was refused: on the iPhone, Settings > General > VPN & Device Management > trust the developer, then open VeloCoach by hand."
fi

echo; echo "OK: VeloCoach is on the iPhone. Next: docs/PHONE_CHECKLIST.md."
