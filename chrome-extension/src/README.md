# ViFastKey for Web (Chrome/Edge MV3)

Tiện ích web, **không phải IME hệ thống**. Dùng WASM engine ViFastKey để:

- Chuột phải vùng chọn → **ViFastKey: chuẩn hoá vùng chọn**.
- Chuột phải vùng chọn → **bỏ dấu**.
- Popup có bàn phím ảo, chuyển Telex/VNI/VIQR và chèn kết quả vào ô đang focus.

## Build

```bash
./chrome-extension/build.sh
# -> chrome-extension/dist
```

Mở `chrome://extensions` → Developer mode → Load unpacked → chọn `dist`.

## Giới hạn

- Chỉ trang web; không address bar, native app, password field, một số editor canvas.
- Context menu dùng `activeTab` + inject khi người dùng bấm, không content script thường trú.
- `contenteditable`/React controlled input có thể cần xử lý thêm; bản MVP dùng `setRangeText`/`execCommand`.
- WASM normalize giữ từ tiếng Anh đã biết trong lexicon 20k; vẫn có thể sai với từ lạ.
- Chưa phát hành Chrome Web Store.
