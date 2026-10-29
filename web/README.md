# ViFastKey Web

Demo gõ thử chạy hoàn toàn trong trình duyệt: Rust engine → WASM, ô nhập mô phỏng
chế độ `REPLACE` (áp từng `Action` lên văn bản, giống adapter UniKey/OpenKey).

## Chạy

```bash
./web/build.sh
python3 -m http.server 8080 -d web
# mở http://127.0.0.1:8080/
```

`build.sh` cần Rust + target `wasm32-unknown-unknown` + `wasm-bindgen`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
```

## Cấu trúc

| File | Vai trò |
|---|---|
| `index.html` | landing page + playground |
| `style.css` | theme theo `design/tokens.json`, font Be Vietnam Pro |
| `app.js` | REPLACE adapter cho trình duyệt, xử lý caret/selection/paste |
| `pkg/` | glue `wasm-bindgen` + `vfk_wasm_bg.wasm` (sinh bởi `build.sh`) |
| `brand/`, `fonts/` | bản copy từ `resources/` để deploy tĩnh |

## Hành vi đã mô phỏng

- Telex / Simple Telex / VNI, dấu cũ/mới.
- `replace`, `commit`, `pass`; `pass_key` cho dấu cách/Enter.
- Backspace xoá đúng ký tự hiển thị; Delete, Esc, chuyển caret bằng chuột/phím.
- Surrounding text để sửa từ vừa gõ (`tiêng` + `s` → `tiếng`).
- Khôi phục tiếng Anh, macro (nếu cấu hình), Việt/Anh theo app.
- Tùy chọn chính tả, `w` → `ư`, `[` `]`, Quick Telex, viết hoa đầu câu, smart caps.

## Giới hạn

- Demo dùng chế độ `REPLACE`; chưa có preedit/TSF/IMK thật.
- Bàn phím ảo trên mobile có thể không phát `keydown` cho từng phím; nên gõ bằng
  bàn phím vật lý để thấy engine xử lý.
- Chưa có app desktop/extension; xem `../README.md` và GitHub Releases.

## Deploy tĩnh

Thư mục `web/` sau khi build là site tĩnh. Có thể deploy nguyên thư mục lên GitHub
Pages/Cloudflare Pages/Netlify. Không cần server API.
