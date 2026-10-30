# ViFastKey Web

Landing page + playground + API Python cho ViFastKey.

## Chạy

### Chỉ xem trang tĩnh

```bash
./web/build.sh
python3 -m http.server 8080 -d web
# mở http://127.0.0.1:8080/
```

`build.sh` cần Rust + target `wasm32-unknown-unknown` + `wasm-bindgen`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
```

### Đầy đủ API Python

```bash
python3 -m venv /private/tmp/vifastkey-venv
/private/tmp/vifastkey-venv/bin/pip install -r deploy/vifastkey_web/requirements.txt
PYTHONPATH=python STATIC_DIR=web \
  /private/tmp/vifastkey-venv/bin/uvicorn app:app \
  --app-dir deploy/vifastkey_web --reload --port 8091
# http://127.0.0.1:8091
```

API dùng engine Python trong `python/` (oracle của bản Rust), chỉ thư viện chuẩn
cộng FastAPI/uvicorn. `/admin` mở công khai nếu chưa đặt `ADMIN_PASSWORD`.

## Cấu trúc

| File | Vai trò |
|---|---|
| `index.html` | landing page + playground + server demo |
| `style.css` | theme theo `design/tokens.json`, font Be Vietnam Pro |
| `app.js` | adapter REPLACE + PREEDIT, macro, chuyển bảng mã, caret/selection/paste |
| `server-demo.js` | gọi `/api/type`, `/api/stats`, `/api/visit`, gửi góp ý |
| `pkg/` | glue `wasm-bindgen` + `vfk_wasm_bg.wasm` (sinh bởi `build.sh`) |
| `brand/`, `fonts/` | bản copy từ `resources/` để deploy tĩnh |

Phần FastAPI/Docker nằm ở [`../deploy/vifastkey_web/`](../deploy/vifastkey_web/README.md).

## Hành vi đã mô phỏng

- Telex / Simple Telex / VNI / VIQR, dấu cũ/mới.
- `replace`, `preedit`, `commit`, `pass`; `pass_key` cho dấu cách/Enter.
- Backspace xoá đúng ký tự hiển thị; Delete, Esc, chuyển caret bằng chuột/phím.
- Surrounding text để sửa từ vừa gõ (`tiêng` + `s` → `tiếng`).
- Khôi phục tiếng Anh, macro (kể cả macro có số `mp3`, `k8s`), Việt/Anh theo app.
- Tùy chọn chính tả, `w` → `ư`, `[` `]`, Quick Telex, viết hoa đầu câu, smart caps,
  âm cuối `k` (`Đắk Lắk`), NFC/NFD.
- Ô macro trong “Tùy chọn nâng cao” lưu cấu hình vào `localStorage`.
- Công cụ chuyển bảng mã ngay trong playground (Unicode, TCVN3, VNI-Windows, VISCII,
  VIQR, CP1258, NCR, C string...), hiển thị cả hex cho bảng một byte.
- API Python: chuyển chuỗi phím thô, chuyển bảng mã, thống kê ẩn danh, góp ý.

## Giới hạn

- Playground là mô phỏng preedit trong `<textarea>`, không phải TSF/IMK/Fcitx5 thật.
- Bàn phím ảo Android/Gboard được xử lý qua `beforeinput` + `compositionend`
  (không phụ thuộc `keydown`); nếu bàn phím khác không phát sự kiện chuẩn, thử
  gõ bằng bàn phím vật lý.
- API Python giới hạn 500 ký tự/lượt, 60 lượt/phút/IP.

## Deploy

- Bản đang chạy: <https://vifastkey.hoctuthien.com/>
- Hướng dẫn Docker/Coolify/LXD: [`../deploy/vifastkey_web/README.md`](../deploy/vifastkey_web/README.md).
- Trang tĩnh vẫn có thể deploy riêng lên GitHub Pages/Cloudflare Pages/Netlify
  (không có API Python, playground WASM vẫn chạy).
