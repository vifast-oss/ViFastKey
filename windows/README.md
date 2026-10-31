# Windows components

- `tsf/` — text service TSF preedit (C++ COM + C ABI), mặc định.
- `installer/` — cài/gỡ per-user, đăng ký TSF, copy hook/settings.
- `replace` (Rust) — hook Replace, nằm tại
  [`../rust/apps/vfk-windows-replace`](../rust/apps/vfk-windows-replace),
  binary phát hành `vifastkey-replace.exe`.

Crate Rust không đặt trực tiếp trong thư mục này vì workspace root là `rust/` và
Cargo yêu cầu member phải nằm dưới workspace root. Tách workspace riêng sẽ làm
`cargo test/clippy --workspace`, CI và release phức tạp hơn mà không đổi binary.

Xem:

- [`tsf/README.md`](tsf/README.md)
- [`../rust/apps/vfk-windows-replace/README.md`](../rust/apps/vfk-windows-replace/README.md)
- [`../packaging/windows/README.md`](../packaging/windows/README.md)
