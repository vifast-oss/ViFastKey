# Cài đặt ViFastKey trên mọi nền tảng

Ưu tiên **Windows** và **macOS**. Linux, Android, iOS, extension, **UEFI demo**,
web/WASM và build từ source ở các mục sau.

> Trang tải: <https://key.vifast.org/#tai-ve>
> Mirror tải trực tiếp khi GitHub Release tạm gián đoạn.
> Các gói desktop chưa ký Authenticode/Developer ID; SmartScreen/Gatekeeper có thể cảnh báo.

## Windows 10/11 — Hook-only (khuyến nghị)

Từ bản này Windows dùng **keyboard hook**, không cài TSF, không cần ký.

1. Tải `vifastkey-windows-hook-0.1.2.zip`.
2. Chuột phải → **Extract All** (đừng chạy trực tiếp trong ZIP).
3. Mở PowerShell tại thư mục đã giải nén:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
```

Không cần Administrator. Installer sẽ:

- chép `vifastkey-settings.exe` vào `%LOCALAPPDATA%\ViFastKey`;
- thêm autostart `ViFastKey` (Settings chạy `--hidden` vào tray);
- chạy hook nền bằng chính binary đó (`--hook`);
- gỡ TSF/ bản hook cũ nếu có.

4. Trong Windows, chọn input source **English (US)**:

```text
Settings → Time & language → Language & region
→ Add a language → English (United States)
→ Win+Space chọn ENG
```

5. Mở **ViFastKey Settings** từ Start Menu hoặc `Ctrl+Shift+,`.

Nếu SmartScreen cảnh báo: **More info → Run anyway** (bản chưa ký).

Sau khi nâng cấp từ bản TSF cũ, hãy đăng xuất/đăng nhập một lần để xoá
ViFastKey TSF khỏi language list; installer đã cố gắng unregister nhưng DLL có thể
đang được app cũ giữ.

### Trạng thái tray

| Icon | Ý nghĩa |
|---|---|
| Logo ViFastKey | Hook đang gõ tiếng Việt |
| `E` | Hook đang English |
| Logo mờ/xám | Đang nhường Microsoft Vietnamese/Japanese hoặc Game Mode |
| Toast | Hiện khi đổi Việt/Anh hoặc khi nhường bộ gõ Windows |

- `Alt+,`: đổi Việt ↔ Anh khi Windows đang English US.
- **Click trái tray**: đổi Việt ↔ Anh; **double-click** mở Settings; chuột phải mở menu.
- `Ctrl+Shift+,`: mở Settings.
- Nút X: ẩn xuống tray. Thoát hẳn: menu tray → **Thoát**.

### Input source và tránh gõ đôi

Hook chỉ xử lý khi foreground dùng **English US (HKL 0x0409)**:

| Windows input | ViFastKey |
|---|---|
| English US | xử lý Việt/Anh; Alt+, hoạt động |
| Vietnamese | pass-through, nhường Microsoft Vietnamese |
| Japanese | pass-through cả English/Japanese mode |
| Ngôn ngữ khác | pass-through |

Muốn dùng ViFastKey: chọn **ENG**. Muốn dùng bộ gõ Windows: chọn **VIE/JP**.
`0x0409` là language ID; nếu bạn dùng Dvorak/US International, hãy chuyển về
US QWERTY để engine nhận phím đúng.

### Game

- Thêm game vào **Ứng dụng → loại trừ**, hoặc chuyển English trước khi chơi.
- Hook có thể bị anti-cheat chặn; game fullscreen exclusive thường không có IME.
- Không nên dùng hook trong game có anti-cheat nghiêm ngặt.

### Gỡ

```powershell
.\uninstall.ps1
```

Giữ `config.json`, macro và `settings-ui.json`. Xoá sạch thêm
`%APPDATA%\ViFastKey`.

## macOS 12+ — IMK

1. Tải `vifastkey-macos-imk-universal.zip` và giải nén.
2. Chạy installer trong thư mục vừa giải nén:

```bash
./install.sh
```

Installer copy `ViFastKey.app` vào `~/Library/Input Methods/` và đăng ký input source.

3. **Đăng xuất rồi đăng nhập** — macOS chỉ làm mới danh sách input source khi
   session được tạo lại.
4. **System Settings → Keyboard → Text Input → Edit → + → Vietnamese → ViFastKey**.
   macOS sẽ hỏi *"Allow ViFastKey to enable ViFastKey?"* → bấm **Allow** (một lần).
   Đây là hộp thoại bảo mật của macOS cho mọi bộ gõ bên thứ ba.
5. Gõ thử `Tieengs Vieejt` → `Tiếng Việt`.

Kiểm tra trạng thái thật (phải ra `enabled=true`):

```bash
"$HOME/Library/Input Methods/ViFastKey.app/Contents/MacOS/ViFastKeyIM" --status
```

Nếu Gatekeeper chặn:

```bash
xattr -dr com.apple.quarantine "$HOME/Library/Input Methods/ViFastKey.app"
```

**Không cài lại nhiều lần.** Bản phát hành chưa ký Developer ID/notarize; mỗi lần
build/cài lại, chữ ký ad-hoc đổi nên macOS coi là app mới và hỏi lại quyền. Nếu
hộp thoại Allow lặp lại: xoá input source, chạy lại installer **một lần**, rồi
đăng xuất/đăng nhập.

Adapter dùng IMK preedit, menu Telex/Simple Telex/VNI, macro và app profile.

## Linux — chọn một adapter

> **Chỉ cài MỘT trong ba:** IBus, Fcitx5 **hoặc** Wayland.
> Cài nhiều hơn sẽ tranh nhau xử lý phím, gõ đôi hoặc mất chữ.

- **GNOME / IBus** → cài gói IBus.
- **KDE / Fcitx5** → cài gói Fcitx5.
- **Sway / Hyprland / niri** (hỗ trợ `zwp_input_method_v2`) → cài Wayland standalone.

Chỉ chọn một dòng theo môi trường đang dùng; không cài cả ba.

## Linux — Wayland standalone (Sway / Hyprland / niri)

Gói `vifastkey-wayland-linux-x86_64.tar.gz` dùng `zwp_input_method_v2` +
`zwp_virtual_keyboard_v1`, không cần Fcitx5/IBus. GNOME/KDE dùng Fcitx5 hoặc
IBus bên dưới.

```bash
tar -xzf vifastkey-wayland-linux-x86_64.tar.gz
cd vifastkey-wayland
./install.sh
systemctl --user enable --now vifastkey-wayland.service
```

Kiểm tra compositor:

```bash
vifastkey-wayland --probe
```

Nếu thiếu `zwp_input_method_manager_v2`, compositor không hỗ trợ. Chi tiết:
`linux/wayland/README.md`.

## Linux — IBus

```bash
tar -xzf vifastkey-ibus-linux.tar.gz
cd vifastkey-ibus
./install.sh
```

Sau đó:

1. Logout/login để GNOME/IBus nhận engine mới và environment.
2. **Settings → Keyboard → Input Sources → + → Vietnamese → ViFastKey**.
3. Nếu terminal/GTK app vẫn dùng fcitx cũ, kiểm tra:

```bash
echo "$GTK_IM_MODULE $QT_IM_MODULE $XMODIFIERS"
ibus restart
```

Cần `ibus`, `python3-gi`, `gir1.2-ibus-1.0` (Ubuntu/Debian). Chi tiết:
`linux/ibus/README.md`.

## Linux — Fcitx5

```bash
tar -xzf vifastkey-fcitx5-linux-x86_64.tar.gz
cd vifastkey-fcitx5
./install.sh
```

Thêm **ViFastKey** trong Fcitx5 Configuration → Input Method; logout/login nếu
cần. Cần `fcitx5`, `fcitx5-frontend-gtk3/gtk4`, `fcitx5-frontend-qt5/6`.
Chi tiết: `linux/fcitx5/README.md`.

## UEFI x86_64 — bản demo đầu tiên

Đây là **ứng dụng UEFI tự chứa**, chạy trước khi OS boot: editor nhỏ, Telex/VNI,
GOP + font tiếng Việt nhúng. Không phải IME toàn hệ thống; không gõ được vào
UEFI Setup, UEFI Shell hay app UEFI khác.

1. Tải `vifastkey-uefi-x86_64.zip`.
2. Giải nén vào thư mục gốc của USB **FAT32**; giữ nguyên `EFI/BOOT`.
   Kết quả đúng: `\EFI\BOOT\BOOTX64.EFI`.
3. Tắt **Secure Boot** trong firmware, hoặc ký `BOOTX64.EFI` bằng key riêng và
   enroll cert. Bật USB boot, tắt Fast Boot/CSM nếu USB không hiện.
4. Mở boot menu (F12/F11/F8/Esc tùy máy), chọn USB. App tự mở editor.
5. Gõ thử:

```text
Tieengs Vieejt   ->   Tiếng Việt
thuowng           ->   thương
dd                ->   đ
```

Phím tắt: `F1` trợ giúp + QR trang chủ (`F1` lần nữa hoặc `Esc` để đóng),
`F2` Việt/Anh, `F3` Telex/VNI,
`F10` thoát về boot menu, `Backspace` xoá phím thô cuối, `Enter` xuống dòng,
mũi tên/Home/End di chuyển. Header có logo và địa chỉ
`key.vifast.org`; mở `F1` để quét QR bằng điện thoại.

Yêu cầu: máy **x86_64 UEFI** (không hỗ trợ firmware IA32), USB FAT32, bàn phím
layout US, Secure Boot tắt hoặc tự ký. Không cần OS/Python/Node.

Giới hạn bản demo: chỉ editor + Telex/VNI + English escape + font nhúng; chưa
có macro, gợi ý từ, charset legacy, file cấu hình, replace mode. Chưa ký
Secure Boot; chưa test rộng trên firmware thật.

Xử lý sự cố:

- USB không hiện trong boot menu: kiểm tra FAT32, đúng đường dẫn
  `\EFI\BOOT\BOOTX64.EFI`, tắt Fast Boot/CSM, thử cổng USB 2.0.
- Secure Boot violation: tắt Secure Boot hoặc ký file.
- Màn hình đen/không có GOP: firmware không cung cấp graphics output; thử máy
  khác hoặc QEMU/OVMF.
- `F10` quay về boot menu là đúng thiết kế; rút USB rồi khởi động lại máy.

Build từ source:

```bash
cd uefi
./tools/build.sh          # build/esp/EFI/BOOT/BOOTX64.EFI
./tools/test.sh           # self-test engine trong QEMU
./package.sh              # dist/vifastkey-uefi-x86_64.zip
```

## Android

Tải `vifastkey-android-debug.apk`, bật **Install unknown apps**, cài rồi mở.
Trong Android Settings → System → Languages & input → On-screen keyboard →
**ViFastKey** → bật. Đây là IME Android đầy đủ; bản debug chưa ký Play Store.

## iOS

`vifastkey-ios-simulator.zip` chỉ chạy trên **iOS Simulator** (chưa ký Apple
Developer). Cài Xcode, mở Simulator, kéo app vào; hoặc build từ `ios/`.
Thiết bị thật cần ký Apple Developer + provision profile.

## Chrome/Edge extension

```bash
# tải vifastkey-chrome-extension.zip rồi giải nén
# chrome://extensions → Developer mode → Load unpacked → chọn thư mục
```

Extension dùng WASM; không cần server. Chi tiết: `chrome-extension/README.md`.

## Web/WASM

- Bản online: <https://key.vifast.org/>
- Playground chạy WASM local trong browser.
- API Python: `/api/type`, `/api/charset`; xem `deploy/vifastkey_web/README.md`.
- Gói `vifastkey-wasm.zip` để nhúng hoặc phục vụ static.

## Build từ source

```bash
# Python oracle
cd python
pip install -e ".[dev]"
python -m pytest -q

# Rust engine + desktop UI
cd ../rust
cargo build --workspace
cargo test --workspace
cargo run -p vifastkey-settings

# Windows hook-only package
cargo build --release -p vifastkey-settings
powershell -ExecutionPolicy Bypass -File ..\packaging\package-windows.ps1

# Linux settings
cargo build --release -p vifastkey-settings
VIFASTKEY_SETTINGS_BIN=target/release/vifastkey-settings \
  ../packaging/package-linux.sh

# macOS IMK
cd ..
./macos/build.sh
./macos/package.sh
```

Windows cần Rust MSVC + Windows SDK; Linux cần X11/Wayland dev libs; macOS cần
Xcode. TSF cũ nằm trong `windows/tsf/` chỉ để tham khảo/legacy, **không được
đóng gói mặc định**.

## Kênh phân phối

| Kênh | Trạng thái |
|---|---|
| Website `/downloads/0.1.2/` | Mirror chính khi GitHub tạm khoá |
| GitHub Release | Có asset UEFI `vifastkey-uefi-x86_64.zip` cùng các nền tảng khác |
| UEFI demo | `vifastkey-uefi-x86_64.zip`, chạy trước OS; chưa ký Secure Boot |
| Winget/Scoop/Homebrew | Chưa có; cần installer/ký |
| Chrome Web Store / Play Store / App Store | Chưa phát hành |

## Giấy phép

Mã nguồn MIT. Xem `LICENSE` và `THIRD_PARTY_NOTICES.md`.
