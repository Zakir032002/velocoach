#!/usr/bin/env bash
# vc-ffi for iPhone + simulator -> Swift bindings -> XCFramework.
set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="${PROFILE:-release}"
OUT="apps/ios/VcCore"

rustup target add aarch64-apple-ios aarch64-apple-ios-sim >/dev/null

for target in aarch64-apple-ios aarch64-apple-ios-sim; do
    cargo build -p vc-ffi --profile "$PROFILE" --target "$target"
done

# Host library: UniFFI reads metadata from this build.
cargo build -p vc-ffi --profile "$PROFILE"

rm -rf build/swift build/headers
mkdir -p build/swift build/headers

cargo run -q -p uniffi-bindgen -- generate \
    --library "target/$PROFILE/libvc_ffi.dylib" \
    --language swift \
    --out-dir build/swift

cp build/swift/*.h build/headers/
cp build/swift/vc_ffiFFI.modulemap build/headers/module.modulemap

mkdir -p "$OUT"
rm -rf "$OUT/VcFFI.xcframework"

xcodebuild -create-xcframework \
    -library "target/aarch64-apple-ios/$PROFILE/libvc_ffi.a" \
    -headers build/headers \
    -library "target/aarch64-apple-ios-sim/$PROFILE/libvc_ffi.a" \
    -headers build/headers \
    -output "$OUT/VcFFI.xcframework"

mkdir -p "$OUT/Sources/VcCore"
cp build/swift/vc_ffi.swift "$OUT/Sources/VcCore/vc_ffi.swift"

echo "OK: $OUT/VcFFI.xcframework + Sources/VcCore/vc_ffi.swift"