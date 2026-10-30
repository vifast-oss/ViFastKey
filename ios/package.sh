#!/usr/bin/env bash
# Đóng gói app simulator để test/CI. Bản device/TestFlight cần Apple account,
# provisioning profile và DEVELOPMENT_TEAM; script không tự ký thay bạn.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
"$HERE/build.sh"
APP="$HERE/build/Build/Products/Debug-iphonesimulator/ViFastKeyHost.app"
ZIP="$HERE/build/ViFastKey-ios-simulator.zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"
echo "Simulator app: $APP"
echo "Zip: $ZIP"
echo "Device/TestFlight: mở project trong Xcode, chọn team, archive và upload."
