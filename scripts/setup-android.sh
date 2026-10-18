#!/usr/bin/env bash
# One command for the iQOO rig: toolchain -> native library -> APK -> install on the phone.
#
#   scripts/setup-android.sh             build everything locally, install on the phone
#   scripts/setup-android.sh --from-ci   install the APK CI built (needs `gh auth login`)
#   scripts/setup-android.sh --apk FILE  install an APK you downloaded yourself
#   scripts/setup-android.sh --no-device stop after the build (no phone needed)
#
# macOS (Apple silicon) with Homebrew. Every install command is printed before it runs, every
# step is skipped when it is already done (safe to re-run), and any failure stops the script
# with the command that failed. It installs only: temurin@17, android-commandlinetools, the SDK
# packages below, cargo-ndk and the Rust Android target. No Android Studio.
set -euo pipefail
trap 'echo >&2; echo "FAILED (line $LINENO): $BASH_COMMAND" >&2; echo "Nothing after this step ran. Fix it and re-run the script." >&2' ERR

cd "$(dirname "$0")/.."

NDK_VERSION="30.0.16248370"
SDK_PACKAGES=("platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;$NDK_VERSION")
PKG="dev.velocoach.rig"
REPO="Zakir032002/velocoach"
APK_LOCAL="apps/android/app/build/outputs/apk/debug/app-debug.apk"

MODE="build"
APK=""
DEVICE=1
while [[ $# -gt 0 ]]; do
    case "$1" in
        --from-ci) MODE="ci" ;;
        --apk) MODE="file"; APK="${2:?--apk needs a path}"; shift ;;
        --no-device) DEVICE=0 ;;
        -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
        *) echo "unknown option: $1 (try --help)" >&2; exit 2 ;;
    esac
    shift
done

say() { printf '\n== %s\n' "$*"; }
run() { printf '+ %s\n' "$*"; "$@"; }
die() { echo "ERROR: $*" >&2; exit 1; }

[[ "$(uname)" == Darwin ]] || die "this script targets macOS (the MacBook). On Linux, see docs/PHONE_CHECKLIST.md."
command -v brew >/dev/null || die "Homebrew is required (https://brew.sh). It is not installed by this script."

# --- Java 17 -----------------------------------------------------------------------------
say "Java 17 (Temurin)"
if ! /usr/libexec/java_home -v 17 >/dev/null 2>&1; then
    run brew install --cask temurin@17
fi
JAVA_HOME="$(/usr/libexec/java_home -v 17)"
export JAVA_HOME
echo "JAVA_HOME=$JAVA_HOME"

# --- Android command-line tools and SDK packages ------------------------------------------
say "Android command-line tools"
if ! command -v sdkmanager >/dev/null; then
    run brew install --cask android-commandlinetools
fi
ANDROID_HOME="${ANDROID_HOME:-$(brew --prefix)/share/android-commandlinetools}"
export ANDROID_HOME ANDROID_SDK_ROOT="$ANDROID_HOME"
echo "ANDROID_HOME=$ANDROID_HOME"

say "SDK licences"
if [[ ! -f "$ANDROID_HOME/licenses/android-sdk-license" ]]; then
    printf '+ yes | sdkmanager --sdk_root=%s --licenses\n' "$ANDROID_HOME"
    yes | sdkmanager --sdk_root="$ANDROID_HOME" --licenses >/dev/null || true
    [[ -f "$ANDROID_HOME/licenses/android-sdk-license" ]] || die "SDK licences were not accepted"
fi

pkg_dir() { echo "$ANDROID_HOME/${1//;//}"; }
if [[ "$MODE" == build ]]; then WANT=("${SDK_PACKAGES[@]}"); else WANT=("platform-tools"); fi
for p in "${WANT[@]}"; do
    if [[ -d "$(pkg_dir "$p")" ]]; then
        echo "ok: $p"
    else
        run sdkmanager --sdk_root="$ANDROID_HOME" --install "$p"
    fi
done
ADB="$ANDROID_HOME/platform-tools/adb"
[[ -x "$ADB" ]] || die "adb not found at $ADB"

# --- Build (or fetch) the APK -------------------------------------------------------------
case "$MODE" in
build)
    say "Rust toolchain for Android"
    command -v rustup >/dev/null || die "rustup is required (https://rustup.rs); it is not installed by this script"
    if ! rustup target list --installed | grep -qx aarch64-linux-android; then
        run rustup target add aarch64-linux-android
    fi
    if ! cargo ndk --version >/dev/null 2>&1; then
        run cargo install cargo-ndk --locked
    fi

    say "Native library + Kotlin bindings"
    export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
    [[ -d "$ANDROID_NDK_HOME" ]] || die "NDK missing at $ANDROID_NDK_HOME"
    run bash scripts/build-android.sh

    say "APK (Gradle wrapper; downloads Gradle and the Android plugin on first run)"
    printf 'sdk.dir=%s\n' "$ANDROID_HOME" > apps/android/local.properties
    (cd apps/android && run ./gradlew --no-daemon :app:assembleDebug)
    APK="$APK_LOCAL"
    ;;
ci)
    say "APK from CI (artifact rig-apk)"
    command -v gh >/dev/null || die "--from-ci needs the GitHub CLI (brew install gh; gh auth login), or download rig-apk from the Actions page and use --apk FILE"
    BRANCH="$(git rev-parse --abbrev-ref HEAD)"
    RUN_ID="$(gh run list --repo "$REPO" --workflow ci.yml --branch "$BRANCH" --status success --limit 1 --json databaseId -q '.[0].databaseId')"
    [[ -n "$RUN_ID" ]] || die "no successful CI run on $BRANCH"
    rm -rf build/ci-apk
    run gh run download "$RUN_ID" --repo "$REPO" --name rig-apk --dir build/ci-apk
    APK="build/ci-apk/app-debug.apk"
    ;;
file) ;;
esac
[[ -f "$APK" ]] || die "APK not found: $APK"
echo "APK: $APK"

if [[ "$DEVICE" == 0 ]]; then
    echo; echo "OK: built $APK (no device step)."; exit 0
fi

# --- The phone ----------------------------------------------------------------------------
say "Phone (USB debugging)"
run "$ADB" start-server
DEVICES="$("$ADB" devices | awk 'NR>1 && $2=="device" {print $1}')"
UNAUTH="$("$ADB" devices | awk 'NR>1 && $2=="unauthorized" {print $1}')"
[[ -z "$UNAUTH" ]] || die "phone $UNAUTH has not authorised this Mac: unlock it and tap 'Allow USB debugging', then re-run"
COUNT="$(printf '%s' "$DEVICES" | grep -c . || true)"
[[ "$COUNT" == 1 ]] || die "expected exactly one phone in 'adb devices', found $COUNT. On the iQOO: Settings > About phone > Software version (tap 7x) > Developer options > USB debugging ON (and 'Install via USB' ON). Use a data cable."
SERIAL="$DEVICES"
A=("$ADB" -s "$SERIAL")

SDK_INT="$("${A[@]}" shell getprop ro.build.version.sdk | tr -d '\r')"
(( SDK_INT >= 31 )) || die "the rig needs Android 12 (API 31) or newer; this phone reports API $SDK_INT"

say "Device facts (saved to build/machine/android-device.txt)"
mkdir -p build/machine
{
    echo "model: $("${A[@]}" shell getprop ro.product.model | tr -d '\r')"
    echo "manufacturer: $("${A[@]}" shell getprop ro.product.manufacturer | tr -d '\r')"
    echo "android: $("${A[@]}" shell getprop ro.build.version.release | tr -d '\r') (API $SDK_INT)"
    echo "os build: $("${A[@]}" shell getprop ro.vivo.os.build.display.id | tr -d '\r')"
    echo "fingerprint: $("${A[@]}" shell getprop ro.build.fingerprint | tr -d '\r')"
} | tee build/machine/android-device.txt

say "Install"
run "${A[@]}" install -r -g "$APK"

say "Permissions and battery"
for perm in BLUETOOTH_ADVERTISE BLUETOOTH_CONNECT; do
    run "${A[@]}" shell pm grant "$PKG" "android.permission.$perm"
done
if (( SDK_INT >= 33 )); then
    run "${A[@]}" shell pm grant "$PKG" android.permission.POST_NOTIFICATIONS
fi
run "${A[@]}" shell dumpsys deviceidle whitelist "+$PKG"
run "${A[@]}" shell cmd appops set "$PKG" RUN_ANY_IN_BACKGROUND allow

say "Launch"
run "${A[@]}" shell am start -n "$PKG/.MainActivity"

cat <<EOF

OK: VeloCoach Rig is installed and open on $SERIAL.
On the phone, once: Settings > Battery > (app) VeloCoach Rig > allow background activity /
"No restrictions", and turn Bluetooth on. OriginOS can still stop background apps; keep the rig
app on screen during a ride. Next: scripts/setup-ios.sh, then docs/PHONE_CHECKLIST.md.
EOF
