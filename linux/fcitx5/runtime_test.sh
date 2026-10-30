#!/usr/bin/env bash
# Runtime smoke test Fcitx5 + GTK trên X11/Xvfb.
#
# Yêu cầu:
#   - addon đã build và cài (./build.sh; sudo cmake --install build --prefix /usr)
#   - fcitx5, fcitx5-frontend-gtk3, Xvfb, openbox, xdotool, zenity, dbus-x11
#
# Script tự tạo XDG_CONFIG_HOME tạm nên không sửa config Fcitx5 của bạn.
set -euo pipefail

if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then
    exec dbus-run-session -- "$0" "$@"
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
XVFB_PID=""
OPENBOX_PID=""
FCITX_PID=""
ZENITY_PID=""

cleanup() {
    for pid in "$ZENITY_PID" "$FCITX_PID" "$OPENBOX_PID" "$XVFB_PID"; do
        [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
    done
    rm -rf "$TMP"
}
trap cleanup EXIT

export DISPLAY=":99"
export XDG_CONFIG_HOME="$TMP/config"
export XDG_RUNTIME_DIR="$TMP/runtime"
export GTK_IM_MODULE=fcitx
export QT_IM_MODULE=fcitx
export XMODIFIERS=@im=fcitx
mkdir -p "$XDG_CONFIG_HOME/fcitx5" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

cat > "$XDG_CONFIG_HOME/fcitx5/profile" <<'EOF'
[Groups/0]
Name=Default
Default Layout=us
DefaultIM=vifastkey

[Groups/0/Items/0]
Name=keyboard-us
Layout=

[Groups/0/Items/1]
Name=vifastkey
Layout=

[GroupOrder]
0=Default
EOF

Xvfb :99 -screen 0 1280x800x24 >"$TMP/xvfb.log" 2>&1 &
XVFB_PID=$!
openbox >"$TMP/openbox.log" 2>&1 &
OPENBOX_PID=$!
sleep 1
fcitx5 -d >"$TMP/fcitx5.log" 2>&1 &
FCITX_PID=$!
sleep 3

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
fcitx5-remote -o >/dev/null 2>&1 || true
fcitx5-remote -s vifastkey >/dev/null 2>&1 || true
sleep 1

xdotool type --delay 60 "tieengs"
xdotool key Return
sleep 2

OUT="$(cat "$TMP/out" 2>/dev/null || true)"
echo "Fcitx5 output: ${OUT:-<empty>}"
if [ "$OUT" != "tiếng" ]; then
    echo "FAIL: expected tiếng" >&2
    echo "--- fcitx5 log ---" >&2
    tail -50 "$TMP/fcitx5.log" >&2 || true
    exit 1
fi
echo "Fcitx5 runtime test passed"
