#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
(cd "$ROOT/rust" && cargo build --release -p vfk-ffi --target aarch64-apple-ios-sim)
(cd "$HERE" && xcodegen generate)
xcodebuild -project "$HERE/ViFastKey.xcodeproj" -scheme ViFastKeyHost \
  -sdk iphonesimulator -configuration Debug \
  -derivedDataPath "$HERE/build" ARCHS=arm64 ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO build
echo "App: $HERE/build/Build/Products/Debug-iphonesimulator/ViFastKeyHost.app"
