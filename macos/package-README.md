# ViFastKey cho macOS

Bộ gõ tiếng Việt dạng input method (IMK), chạy trong `~/Library/Input Methods`.

## Cài

1. Giải nén ZIP.
2. Mở Terminal tại thư mục vừa giải nén và chạy:

   ```bash
   ./install.sh
   ```

3. **Đăng xuất rồi đăng nhập** — bắt buộc một lần để macOS làm mới danh sách
   input source.
4. **System Settings → Keyboard → Text Input → Edit → + → Vietnamese → ViFastKey**.
5. macOS hỏi *"Allow ViFastKey to enable ViFastKey?"* → bấm **Allow** (một lần).
   Đây là hộp thoại bảo mật của macOS cho mọi bộ gõ bên thứ ba.
6. Gõ thử `Tieengs Vieejt` → `Tiếng Việt`.

## Kiểm tra

```bash
"$HOME/Library/Input Methods/ViFastKey.app/Contents/MacOS/ViFastKeyIM" --status
```

Phải thấy `enabled=true`. Nếu Gatekeeper chặn:

```bash
xattr -dr com.apple.quarantine "$HOME/Library/Input Methods/ViFastKey.app"
```

## Lưu ý

- Bản phát hành chưa ký Developer ID/notarize.
- **Không cài lại nhiều lần.** Mỗi lần cài lại, chữ ký ad-hoc đổi nên macOS coi
  là app mới và hỏi lại quyền. Nếu hộp thoại Allow lặp lại: xoá input source,
  chạy lại `./install.sh` một lần, rồi đăng xuất/đăng nhập.
- Cấu hình: `~/Library/Application Support/ViFastKey/config.json`.
- Phím tắt: `Alt+,` bật/tắt tiếng Việt trong app.

## Gỡ

```bash
./uninstall.sh
```

Sau đó xoá ViFastKey khỏi System Settings → Keyboard → Text Input và đăng xuất/
đăng nhập. Cấu hình vẫn giữ trong `~/Library/Application Support/ViFastKey/`.
