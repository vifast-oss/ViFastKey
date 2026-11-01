# Linux adapters

- `ibus/` — ✅ engine IBus Python (core test được, cần Linux + PyGObject/IBus để chạy thật).
- `fcitx5/` — ✅ addon C++ dùng C ABI Rust; build bằng CMake, CI Ubuntu.
- `wayland/` — ✅ adapter Wayland standalone (`zwp_input_method_v2` +
  `zwp_virtual_keyboard_v1`), không cần Fcitx5/IBus; test Sway headless + GTK4.
- `vifastkey-settings.desktop` — UI cấu hình egui; tray dùng StatusNotifierItem
  (`ksni`), không cần GTK/AppIndicator trong binary. GNOME cần extension
  **AppIndicator and KStatusNotifierItem Support** để thấy icon tray.

Xem `ibus/README.md`, `fcitx5/README.md` và `docs/PLATFORMS.md`.
