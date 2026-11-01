#!/usr/bin/env bash
# Test thật adapter Wayland: Sway headless + GTK4 text-input + wtype.
#
# Chỉ chạy trên Linux có:
#   sway, wtype, python3-gi, gir1.2-gtk-4.0
#
# Cách dùng:
#   ./linux/wayland/runtime_test.sh
#   VFK_WAYLAND_BIN=./rust/target/release/vifastkey-wayland ./linux/wayland/runtime_test.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${VFK_WAYLAND_BIN:-$ROOT/rust/target/release/vifastkey-wayland}"
if [ ! -x "$BIN" ]; then
    BIN="$ROOT/rust/target/debug/vifastkey-wayland"
fi
if [ ! -x "$BIN" ]; then
    echo "SKIP: chưa build vifastkey-wayland ($BIN)" >&2
    exit 77
fi

for cmd in sway wtype python3; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "SKIP: thiếu $cmd" >&2
        exit 77
    fi
done
if ! python3 -c 'import gi; gi.require_version("Gtk", "4.0")' >/dev/null 2>&1; then
    echo "SKIP: thiếu python3-gi / gir1.2-gtk-4.0" >&2
    exit 77
fi

TMP="$(mktemp -d /tmp/vfk-wayland-test.XXXXXX)"
export XDG_RUNTIME_DIR="$TMP/runtime"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export WLR_BACKENDS=headless
export WLR_RENDERER=pixman

cat > "$TMP/sway.conf" <<'EOF'
output HEADLESS-1 resolution 1280x800
EOF

cat > "$TMP/client.py" <<'PY'
import gi, os, sys
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk, GLib, Gio

out = sys.argv[1]

class App(Gtk.Application):
    def __init__(self):
        super().__init__(
            application_id=f"dev.vifastkey.Test{os.getpid()}",
            flags=Gio.ApplicationFlags.NON_UNIQUE,
        )
        self.entry = None

    def do_activate(self):
        win = Gtk.ApplicationWindow(application=self, title="vfk-test")
        self.entry = Gtk.Entry()
        win.set_child(self.entry)
        win.set_default_size(500, 120)
        win.present()
        self.entry.grab_focus()
        GLib.timeout_add(80, self.dump)
        GLib.timeout_add(30000, self.quit)

    def dump(self):
        with open(out, "w", encoding="utf-8") as f:
            f.write(self.entry.get_text())
        return True

App().run([])
PY

SWAY_PID=""
IME_PID=""
CLIENT_PID=""
cleanup() {
    [ -n "$CLIENT_PID" ] && kill "$CLIENT_PID" 2>/dev/null || true
    [ -n "$IME_PID" ] && kill "$IME_PID" 2>/dev/null || true
    if [ -n "$SWAY_PID" ]; then
        SWAYSOCK="$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* 2>/dev/null | head -1 || true)"
        if [ -n "$SWAYSOCK" ]; then
            SWAYSOCK="$SWAYSOCK" swaymsg exit >/dev/null 2>&1 || true
        fi
        kill "$SWAY_PID" 2>/dev/null || true
        wait "$SWAY_PID" 2>/dev/null || true
    fi
    rm -rf "$TMP"
}
trap cleanup EXIT

sway -c "$TMP/sway.conf" > "$TMP/sway.log" 2>&1 &
SWAY_PID=$!
for _ in $(seq 1 50); do
    [ -S "$XDG_RUNTIME_DIR/wayland-1" ] && break
    sleep 0.1
done
if [ ! -S "$XDG_RUNTIME_DIR/wayland-1" ]; then
    echo "FAIL: Sway không khởi động" >&2
    tail -20 "$TMP/sway.log" >&2
    exit 1
fi
export WAYLAND_DISPLAY=wayland-1
export SWAYSOCK="$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* | head -1)"

"$BIN" --debug > "$TMP/ime.log" 2>&1 &
IME_PID=$!
sleep 1
if ! kill -0 "$IME_PID" 2>/dev/null; then
    echo "FAIL: vifastkey-wayland thoát sớm" >&2
    cat "$TMP/ime.log" >&2
    exit 1
fi

PASS=0
FAIL=0

run_case() {
    local name="$1" expected="$2"
    shift 2
    local out="$TMP/out-$name.txt"
    rm -f "$out"
    python3 "$TMP/client.py" "$out" > "$TMP/client-$name.log" 2>&1 &
    CLIENT_PID=$!
    sleep 1.8
    "$@"
    sleep 0.8
    local got
    got="$(cat "$out" 2>/dev/null || true)"
    if [ "$got" = "$expected" ]; then
        echo "PASS $name: $(printf '%q' "$got")"
        PASS=$((PASS + 1))
    else
        echo "FAIL $name: got $(printf '%q' "$got"), want $(printf '%q' "$expected")" >&2
        FAIL=$((FAIL + 1))
    fi
    kill "$CLIENT_PID" 2>/dev/null || true
    wait "$CLIENT_PID" 2>/dev/null || true
    CLIENT_PID=""
    sleep 0.8
}

type_text() {
    wtype -d 80 "$1"
}
type_key() {
    wtype -k "$1"
}

run_case telex      "tiếng "      type_text "tieengs "
run_case capital    "Việt Nam "   type_text "Vieejt Nam "
run_case escape_raw "text "       bash -c 'wtype -d 80 "text"; sleep 0.3; wtype -k Escape; wtype -d 80 " "'
run_case toggle_off "as "         bash -c 'wtype -M alt -k comma -m alt; sleep 0.3; wtype -d 80 "as "'
run_case toggle_on  "as á "       bash -c 'wtype -M alt -k comma -m alt; sleep 0.3; wtype -d 80 "as "; wtype -M alt -k comma -m alt; sleep 0.3; wtype -d 80 "as "'
run_case replace_prev "Cháo"      bash -c 'wtype -d 80 "Chaof "; sleep 0.4; wtype -k BackSpace; sleep 0.3; wtype -d 80 "s"'
run_case pass_through "ab"        bash -c 'wtype -M alt -k comma -m alt; sleep 0.3; wtype -d 80 "abc"; sleep 0.3; wtype -k BackSpace'

echo "----------------------------------------"
echo "Wayland runtime: $PASS passed, $FAIL failed"
if [ "$FAIL" -ne 0 ]; then
    echo "--- IME log" >&2
    tail -80 "$TMP/ime.log" >&2
    exit 1
fi
