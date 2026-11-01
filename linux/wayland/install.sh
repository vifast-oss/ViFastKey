#!/usr/bin/env bash
# Cài adapter Wayland standalone vào ~/.local và systemd user service.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin/vifastkey-wayland"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

if ! command -v cargo >/dev/null 2>&1; then
    echo "Cần cargo (rustup) để build." >&2
    exit 1
fi
if ! pkg-config --exists xkbcommon 2>/dev/null; then
    echo "Cần libxkbcommon-dev (Ubuntu: sudo apt install libxkbcommon-dev)." >&2
    exit 1
fi

echo "== build release"
cargo build --manifest-path "$ROOT/rust/Cargo.toml" --release -p vifastkey-wayland

echo "== cài binary: $BIN"
install -Dm755 "$ROOT/rust/target/release/vifastkey-wayland" "$BIN"

echo "== cài systemd user service"
mkdir -p "$UNIT_DIR"
install -Dm644 "$ROOT/linux/wayland/vifastkey-wayland.service" \
    "$UNIT_DIR/vifastkey-wayland.service"
# Thay %h bằng HOME thật để service dùng đúng binary vừa cài.
sed -i "s|%h/.local|$PREFIX|g" "$UNIT_DIR/vifastkey-wayland.service"

systemctl --user daemon-reload || true

cat <<EOF

Đã cài. Kiểm tra compositor:
  $BIN --probe

Chạy thử (foreground):
  $BIN --debug

Bật service:
  systemctl --user enable --now vifastkey-wayland.service
  journalctl --user -u vifastkey-wayland -f

Lưu ý: cần compositor hỗ trợ zwp_input_method_v2 + zwp_virtual_keyboard_v1
(Sway/wlroots; Hyprland/niri nếu --probe có protocol). GNOME/KDE dùng
IBus/Fcitx5, không dùng adapter này.
EOF
