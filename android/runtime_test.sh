#!/usr/bin/env bash
# Runtime smoke test trên emulator/máy thật đã bật USB debugging.
# Yêu cầu: APK debug đã build và adb có trong PATH/ANDROID_HOME.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
: "${ANDROID_HOME:=/opt/homebrew/share/android-commandlinetools}"
ADB="${ADB:-$ANDROID_HOME/platform-tools/adb}"
APK="${APK:-$HERE/app/build/outputs/apk/debug/app-debug.apk}"

[ -x "$ADB" ] || { echo "Không tìm thấy adb: $ADB" >&2; exit 1; }
[ -f "$APK" ] || { echo "Không tìm thấy APK: $APK" >&2; exit 1; }

"$ADB" wait-for-device
"$ADB" install -r "$APK" >/dev/null
# Khởi động lại activity trước, rồi mới chọn IME: force-stop cùng package sẽ
# kill luôn IME và Android có thể reset về bàn phím mặc định.
"$ADB" shell am force-stop app.vifastkey
"$ADB" shell am start -W -n app.vifastkey/.TestActivity >/dev/null
sleep 2
"$ADB" shell ime enable app.vifastkey/.ViFastKeyService >/dev/null
"$ADB" shell ime set app.vifastkey/.ViFastKeyService >/dev/null
# Chạm vào ô nhập để chắc chắn soft keyboard/IME đang hiển thị.
"$ADB" shell input tap 540 300
sleep 3

# Xoá nội dung cũ. 20 phím đủ cho smoke test.
for _ in $(seq 1 20); do "$ADB" shell input keyevent 67 >/dev/null; done
# t i e e n g s
"$ADB" shell input keyevent 48 37 33 33 42 35 47
sleep 2

"$ADB" shell uiautomator dump /sdcard/vifastkey-window.xml >/dev/null
TEXT="$("$ADB" shell cat /sdcard/vifastkey-window.xml \
    | tr '>' '\n' \
    | grep -o 'text="[^"]*" resource-id="" class="android.widget.EditText"' \
    | head -1 \
    | sed -E 's/^text="([^"]*)".*/\1/')"

echo "Android IME output: ${TEXT:-<empty>}"
[ "$TEXT" = "tiếng" ] || { echo "FAIL: expected tiếng" >&2; exit 1; }
echo "Android runtime test passed"
