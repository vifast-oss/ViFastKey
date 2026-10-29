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
echo
echo "Nếu chưa gõ được: đăng xuất/đăng nhập, kiểm tra Input Sources, xem Console/log:"
echo "  log stream --predicate 'process == \"ViFastKeyIM\"' --level debug"
echo "Self-test headless:"
(cd "$ROOT/rust" && cargo build --release -p vfk-ffi >/dev/null) && swiftc -O -import-objc-header "$ROOT/macos/Sources/Bridging.h" -framework Foundation "$ROOT/rust/target/release/libvifastkey.a" "$ROOT/macos/Tests/main.swift" "$ROOT/macos/Sources/VfkBridge.swift" "$ROOT/macos/Sources/ActionApplier.swift" "$ROOT/macos/Sources/KeyMapper.swift" -o /private/tmp/vifastkey-selftest && /private/tmp/vifastkey-selftest
