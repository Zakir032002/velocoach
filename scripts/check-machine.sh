#!/usr/bin/env bash
# Read-only: what this Mac has and what the setup scripts would still install. Installs
# nothing. Exit status 1 if something only you can install (Xcode, Homebrew, rustup) is missing.
set -uo pipefail
cd "$(dirname "$0")/.."

missing_manual=0
ok() { printf '  ok      %-22s %s\n' "$1" "$2"; }
todo() { printf '  setup   %-22s %s\n' "$1" "$2"; }
manual() { printf '  MANUAL  %-22s %s\n' "$1" "$2"; missing_manual=1; }

echo "Machine"
if [[ "$(uname)" == Darwin ]]; then
    ok macOS "$(sw_vers -productVersion) on $(uname -m), $(( $(sysctl -n hw.memsize) / 1073741824 )) GB RAM"
else
    manual macOS "this is $(uname -s); the phone builds need the MacBook"
fi
free_gb=$(df -g . 2>/dev/null | awk 'NR==2 {print $4}')
[[ -n "$free_gb" ]] && { (( free_gb >= 15 )) && ok disk "$free_gb GB free" || manual disk "$free_gb GB free; the SDK, NDK and Xcode builds need about 15 GB"; }

echo "Prerequisites (install yourself)"
command -v brew >/dev/null && ok Homebrew "$(brew --version | head -1)" || manual Homebrew "https://brew.sh"
dev="$(xcode-select -p 2>/dev/null || true)"
[[ "$dev" == *Xcode*.app/* ]] && ok Xcode "$(xcodebuild -version 2>/dev/null | head -1)" || manual Xcode "App Store, then sudo xcode-select -s /Applications/Xcode.app"
command -v rustup >/dev/null && ok rustup "$(rustc --version 2>/dev/null)" || manual rustup "https://rustup.rs"
command -v git >/dev/null && ok git "$(git rev-parse --abbrev-ref HEAD 2>/dev/null) @ $(git rev-parse --short HEAD 2>/dev/null)"

echo "Installed by scripts/setup-ios.sh"
command -v xcodegen >/dev/null && ok xcodegen "$(xcodegen --version 2>/dev/null)" || todo xcodegen "brew install xcodegen"
for t in aarch64-apple-ios aarch64-apple-ios-sim; do
    rustup target list --installed 2>/dev/null | grep -qx "$t" && ok "$t" "" || todo "$t" "rustup target add $t"
done
n=$(find assets/cues -name '*.wav' 2>/dev/null | wc -l | tr -d ' ')
(( n > 0 )) && ok "cue bank" "$n WAVs" || manual "cue bank" "git pull (CI renders it)"

echo "Installed by scripts/setup-android.sh"
/usr/libexec/java_home -v 17 >/dev/null 2>&1 && ok "Java 17" "$(/usr/libexec/java_home -v 17)" || todo "Java 17" "brew install --cask temurin@17"
SDK="${ANDROID_HOME:-$(brew --prefix 2>/dev/null)/share/android-commandlinetools}"
command -v sdkmanager >/dev/null && ok sdkmanager "$SDK" || todo sdkmanager "brew install --cask android-commandlinetools"
for p in platform-tools platforms/android-36 build-tools/36.0.0 ndk/30.0.16248370; do
    [[ -d "$SDK/$p" ]] && ok "$p" "" || todo "$p" "sdkmanager --install"
done
rustup target list --installed 2>/dev/null | grep -qx aarch64-linux-android && ok aarch64-linux-android "" || todo aarch64-linux-android "rustup target add aarch64-linux-android"
cargo ndk --version >/dev/null 2>&1 && ok cargo-ndk "$(cargo ndk --version 2>/dev/null | head -1)" || todo cargo-ndk "cargo install cargo-ndk --locked"

echo "Phones (plug them in to check)"
ADB="$SDK/platform-tools/adb"
if [[ -x "$ADB" ]]; then
    d=$("$ADB" devices | awk 'NR>1 && NF {print $1" ("$2")"}' | tr '\n' ' ')
    [[ -n "$d" ]] && ok iQOO "$d" || todo iQOO "not on USB / USB debugging off"
else
    todo iQOO "adb not installed yet"
fi
if command -v xcrun >/dev/null && xcrun devicectl list devices >/dev/null 2>&1; then
    i=$(xcrun devicectl list devices 2>/dev/null | grep -ci iphone || true)
    (( i > 0 )) && ok iPhone "$(xcrun devicectl list devices 2>/dev/null | grep -i iphone | head -1 | tr -s ' ')" || todo iPhone "not connected / not trusted"
fi

echo
if (( missing_manual )); then
    echo "Install the MANUAL items first, then run scripts/setup-ios.sh and scripts/setup-android.sh."
    exit 1
fi
echo "Next: scripts/setup-ios.sh and scripts/setup-android.sh (both safe to re-run)."
