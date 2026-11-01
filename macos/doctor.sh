#!/usr/bin/env bash
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$HOME/Library/Input Methods/ViFastKey.app"
echo "== ViFastKey macOS doctor =="
echo "arch: $(uname -m)  macos: $(sw_vers -productVersion)"
if [ -d "$APP" ]; then echo "app: OK ($APP)"; else echo "app: MISSING — chạy macos/install.sh"; fi
if [ -x "$APP/Contents/MacOS/ViFastKeyIM" ]; then
  echo "binary: OK"
  file "$APP/Contents/MacOS/ViFastKeyIM"
else
  echo "binary: MISSING"
fi
if pgrep -x ViFastKeyIM >/dev/null; then echo "process: running"; else echo "process: not running (mở app hoặc đăng nhập lại)"; fi
echo "config: $HOME/Library/Application Support/ViFastKey/config.json"
[ -f "$HOME/Library/Application Support/ViFastKey/config.json" ] && echo "config: OK" || echo "config: chưa có (tự tạo khi chạy lần đầu)"
echo "input sources:"
defaults read com.apple.HIToolbox AppleEnabledInputSources 2>/dev/null | grep -A3 -B1 -i vifastkey || echo "  chưa thấy ViFastKey trong AppleEnabledInputSources"
defaults read com.apple.HIToolbox AppleSelectedInputSources 2>/dev/null | grep -A3 -B1 -i vifastkey || echo "  chưa chọn ViFastKey làm input source"
entries="$(defaults read com.apple.HIToolbox AppleEnabledInputSources 2>/dev/null | grep -c 'app.vifastkey.inputmethod' || true)"
if [ "${entries:-0}" -gt 1 ]; then
  echo "  CẢNH BÁO: $entries entry ViFastKey (nên là 1) — xoá bớt trong Input Sources"
fi

echo "signature:"
codesign -dv "$APP" 2>&1 | grep -E '^(Identifier|Signature|CDHash|TeamIdentifier)=' | sed 's/^/  /' || true
codesign -dr - "$APP" 2>&1 | sed 's/^/  /' || true

echo "copies:"
for p in "$ROOT/macos/build/ViFastKey.app" "$HOME/Downloads/ViFastKey.app" \
         "/Applications/ViFastKey.app" "/Library/Input Methods/ViFastKey.app"; do
  [ -d "$p" ] && echo "  $p"
done
[ -d "$ROOT/macos/build/ViFastKey.app" ] && echo "  (bản trong repo chỉ để build, đừng mở/đăng ký nó)"

echo "crash reports:"
ls -t "$HOME/Library/Logs/DiagnosticReports"/ViFastKeyIM* 2>/dev/null | head -3 || echo "  không có"
echo
echo "Nếu chưa gõ được: đăng xuất/đăng nhập, kiểm tra Input Sources, xem Console/log:"
echo "  log stream --predicate 'process == \"ViFastKeyIM\"' --level debug"
echo "Self-test headless:"
(cd "$ROOT/rust" && cargo build --release -p vfk-ffi >/dev/null) && swiftc -O -import-objc-header "$ROOT/macos/Sources/Bridging.h" -framework Foundation "$ROOT/rust/target/release/libvifastkey.a" "$ROOT/macos/Tests/main.swift" "$ROOT/macos/Sources/VfkBridge.swift" "$ROOT/macos/Sources/ActionApplier.swift" "$ROOT/macos/Sources/KeyMapper.swift" -o /private/tmp/vifastkey-selftest && /private/tmp/vifastkey-selftest
