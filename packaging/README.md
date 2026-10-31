# Packaging

Hướng dẫn và script đóng gói cho người dùng cuối:

| Gói | Nội dung | README |
|---|---|---|
| `vifastkey-windows-tsf-<version>.zip` | TSF x64/x86 + engine + settings + hook + installer | `packaging/windows/README.md` |
| `vifastkey-settings-windows-x64.zip` | Chỉ UI settings (CI) | `packaging/windows/settings-only-README.md` |
| `vifastkey-settings-linux-x86_64-<version>.tar.gz` | UI settings + desktop entry/icon + install script | `packaging/linux/README.md` |

## Windows

```powershell
# Sau khi build:
#   cargo build --release -p vfk-ffi -p vfk-windows-replace -p vifastkey-settings
#   cmake -S windows/tsf -B build/tsf-x64 -A x64 -DVFK_RUST_PROFILE=release
#   cmake --build build/tsf-x64 --config Release
#   (tương tự build/tsf-x86)
.\packaging\package-windows.ps1 -X64Build .\build\tsf-x64 -X86Build .\build\tsf-x86
```

Gói có `README.md` hướng dẫn cài đặt/giới thiệu tính năng, `DEVELOPER.md` cho
build/kỹ thuật, `LICENSE`, `THIRD_PARTY_NOTICES.md`, installer/uninstaller.

## Linux

```bash
cargo build --release -p vifastkey-settings
./packaging/package-linux.sh
```

Gói có `README.md`, `DEVELOPER.md`, `install.sh`, `uninstall.sh`, desktop entry,
icon, `LICENSE`, `THIRD_PARTY_NOTICES.md`.

Gói Linux **chỉ là UI cấu hình**; engine IBus/Fcitx5 cài theo `linux/ibus/` hoặc
`linux/fcitx5/`.
