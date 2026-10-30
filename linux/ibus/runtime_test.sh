#!/usr/bin/env bash
# Runtime smoke test IBus + GTK trên X11/Xvfb.
#
# Yêu cầu:
#   - đã chạy ./linux/ibus/install.sh (component nằm trong /usr/share/ibus/component)
#   - ibus, ibus-gtk3, python3-gi, Xvfb, openbox, xdotool, zenity, dbus-x11
#
# Script tự tạo XDG_CONFIG_HOME/XDG_CACHE_HOME tạm nên không sửa cấu hình IBus
# của bạn.
set -euo pipefail

if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then
    exec dbus-run-session -- "$0" "$@"
fi

COMP="${IBUS_COMPONENT_DIR:-/usr/share/ibus/component}/vifastkey.xml"
if [ ! -f "$COMP" ]; then
    echo "FAIL: chưa thấy $COMP; chạy ./linux/ibus/install.sh trước." >&2
    exit 2
fi

TMP="$(mktemp -d)"
XVFB_PID=""
OPENBOX_PID=""
IBUS_PID=""
ZENITY_PID=""

cleanup() {
    for pid in "$ZENITY_PID" "$IBUS_PID" "$OPENBOX_PID" "$XVFB_PID"; do
        [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
    done
    rm -rf "$TMP"
}
trap cleanup EXIT

export DISPLAY=":99"
export XDG_CONFIG_HOME="$TMP/config"
export XDG_CACHE_HOME="$TMP/cache"
export XDG_RUNTIME_DIR="$TMP/runtime"
export GTK_IM_MODULE=ibus
export QT_IM_MODULE=ibus
export XMODIFIERS=@im=ibus
mkdir -p "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

Xvfb :99 -screen 0 1280x800x24 >"$TMP/xvfb.log" 2>&1 &
XVFB_PID=$!
openbox >"$TMP/openbox.log" 2>&1 &
OPENBOX_PID=$!
sleep 1
ibus-daemon -drx >"$TMP/ibus.log" 2>&1 &
IBUS_PID=$!
sleep 4

zenity --entry --title=ViFastKeyTest --text="Type:" >"$TMP/out" 2>"$TMP/zenity.log" &
ZENITY_PID=$!
sleep 2
WID="$(xdotool search --name ViFastKeyTest | head -1)"
if [ -z "$WID" ]; then
    echo "FAIL: không tìm thấy cửa sổ zenity" >&2
    cat "$TMP/zenity.log" >&2 || true
    exit 1
fi
xdotool windowactivate --sync "$WID"
sleep 1
if ! ibus engine vifastkey 2>"$TMP/engine.err"; then
    echo "FAIL: không chọn được engine vifastkey" >&2
    cat "$TMP/engine.err" >&2 || true
    exit 1
fi
sleep 2

xdotool type --delay 60 "tieengs"
xdotool key Return
sleep 2

OUT="$(cat "$TMP/out" 2>/dev/null || true)"
echo "IBus output: ${OUT:-<empty>}"
if [ "$OUT" != "tiếng" ]; then
    echo "FAIL: expected tiếng" >&2
    echo "--- ibus log ---" >&2
    tail -50 "$TMP/ibus.log" >&2 || true
    exit 1
fi
echo "IBus runtime test passed"
