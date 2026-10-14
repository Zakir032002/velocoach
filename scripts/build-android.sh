#!/usr/bin/env bash
# vc-ffi with the simulator for Android arm64-v8a.
set -euo pipefail

cd "$(dirname "$0")/.."

: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to your Android NDK path}"

APP="apps/android/app"
EXT=$([[ "$(uname)" == Darwin ]] && echo dylib || echo so)

rustup target add aarch64-linux-android >/dev/null

cargo ndk -t arm64-v8a --platform 33 \
    -o "$APP/src/main/jniLibs" \
    build -p vc-ffi --features sim --release

# Generate bindings from the host build with the same sim feature.
cargo build -p vc-ffi --features sim --release

cargo run -q -p uniffi-bindgen -- generate \
    --library "target/release/libvc_ffi.$EXT" \
    --language kotlin \
    --out-dir "$APP/src/main/java"

echo "OK: $APP/src/main/jniLibs/arm64-v8a/libvc_ffi.so + Kotlin bindings"