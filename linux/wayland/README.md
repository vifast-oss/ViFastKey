# ViFastKey Wayland (standalone)

Adapter Wayland **không cần Fcitx5/IBus**. Nó nói trực tiếp với compositor qua:

- `zwp_input_method_v2` — nhận surrounding text, preedit, commit, delete.
- `zwp_virtual_keyboard_v1` — chuyển tiếp phím không do IME xử lý.
- `zwp_input_method_keyboard_grab_v2` — nhận key event từ compositor.

Logic tiếng Việt nằm trong `vfk-engine`; quyết định preedit/commit/replace dùng
`vfk-adapter-core` (giống Fcitx5/IBus/IMK).

## Yêu cầu

- Linux + Wayland.
- Compositor hỗ trợ `zwp_input_method_v2` **và** `zwp_virtual_keyboard_v1`:
  - ✅ Sway (đã test), các compositor wlroots khác.
  - ◐ Hyprland/niri: chạy `--probe` trước; protocol phải được expose.
  - ❌ GNOME Mutter, KDE KWin — dùng IBus/Fcitx5 thay thế.
- `libxkbcommon` (runtime + dev khi build).
- Build: Rust stable (MSRV workspace).

Kiểm tra nhanh:

```bash
vifastkey-wayland --probe
# zwp_input_method_manager_v2: true
# zwp_virtual_keyboard_manager_v1: true
# wl_seat: true
```

Nếu `probe` báo thiếu, compositor không hỗ trợ; đừng cố chạy.

## Build

```bash
cd rust
cargo build --release -p vifastkey-wayland
./target/release/vifastkey-wayland --probe
```

Ubuntu/Debian:

```bash
sudo apt install libxkbcommon-dev
```

## Cài + chạy

```bash
./linux/wayland/install.sh
systemctl --user enable --now vifastkey-wayland.service
journalctl --user -u vifastkey-wayland -f
```

Hoặc chạy trực tiếp:

```bash
~/.local/bin/vifastkey-wayland --debug
```

Cấu hình dùng chung `~/.config/vifastkey/config.json`
(hoặc `--config PATH`). File thiếu → dùng default. Adapter tự reload khi mtime
đổi và không có composition.

## Hành vi

- **Preedit mặc định**: gõ `tieengs` → preedit `tiếng`; space/Enter commit.
- **Surrounding text**: sửa từ trước con trỏ bằng `delete_surrounding_text`
  khi app cung cấp snapshot; snapshot lệch → fail-safe, không xoá mù.
- **Field policy**: `content_type.purpose` = password/pin → pass-through;
  URL/email/number/terminal có policy riêng.
- **Pass-through**: phím không do engine xử lý (mũi tên, Ctrl shortcut, ...)
  được gửi qua virtual keyboard; keycode/Modifiers giữ nguyên.
- **Alt+,**: bật/tắt tiếng Việt (giống Fcitx5/IBus).
- **Toggle/release**: release của phím đã bị IME tiêu thụ không bị forward;
  release của phím pass-through được forward.

## Giới hạn

- `zwp_input_method_v2` **không cung cấp app_id**, nên adapter standalone chưa
  áp dụng `AppProfile` theo app. Dùng cấu hình global; nếu cần per-app, dùng
  Fcitx5/IBus.
- Không có popup surface (candidate window/emoji picker).
- Không hỗ trợ app không dùng `text-input-v3` (một số app XWayland/legacy).
- Không can thiệp password field nếu compositor không gửi `content_type`.
- Nếu compositor thiếu `virtual-keyboard-v1`, phím pass-through sẽ không tới
  app; IME vẫn commit chữ nhưng Backspace/mũi tên có thể bị nuốt.

## Test

Unit test (mọi OS):

```bash
cargo test -p vifastkey-wayland
```

E2E trên Linux (Sway headless + GTK4 + wtype):

```bash
./linux/wayland/runtime_test.sh
```

Script tự tạo `XDG_RUNTIME_DIR`, chạy Sway headless, mở GTK Entry, gõ bằng
`wtype`, và kiểm tra:

| Case | Kỳ vọng |
|---|---|
| `tieengs ` | `tiếng ` |
| `Vieejt Nam ` | `Việt Nam ` |
| `text` + Esc + space | `text ` |
| Alt+, rồi `as ` | `as ` |
| Alt+, lần nữa rồi `as ` | `as á ` |

Cần gói: `sway wtype python3-gi gir1.2-gtk-4.0`.

## Debug

```bash
RUST_LOG=debug vifastkey-wayland
```

- `đã nạp keymap` — compositor gửi keymap; nếu lặp nhiều lần, bình thường khi
  nhiều client/keyboard khác nhau kết nối.
- `key code=...` — key đã được xkbcommon dịch.
- `input-method-v2 unavailable` — compositor/seat không hỗ trợ, hoặc đã có IME
  khác đang giữ seat.
- `compositor không có zwp_virtual_keyboard_manager_v1` — pass-through sẽ không
  hoạt động.

## Kiến trúc

```text
linux.rs (Wayland protocol)
  ├─ zwp_input_method_v2 events ──► Controller
  ├─ keyboard grab events ───────► xkbcommon ──► KeyInput
  └─ virtual keyboard ◄──────────── ForwardKey
controller.rs (test được trên macOS)
  ├─ ImeSession (vfk-engine)
  ├─ vfk-adapter-core::plan_action
  └─ Backend trait (preedit/commit/replace/forward)
```
