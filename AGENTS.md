# ViFastKey — hướng dẫn cho agent

> Bộ gõ tiếng Việt: Python là oracle, Rust là bản port; adapter cho desktop/mobile/web/UEFI.
> Đọc file này trước khi sửa code hoặc deploy.

## 1. Quy ước chung

- Python (oracle): `python/` — sửa hành vi ở đây trước, sinh conformance.
- Rust (port): `rust/` — phải qua 100% `conformance/*.jsonl`.
- UEFI demo: `uefi/` — workspace riêng, dùng lại `vfk-core`; không fork engine.
- Web + deploy: `web/` (static) và `deploy/vifastkey_web/` (FastAPI).
- Ảnh/promo: `docs/promo/`.
- Tài khoản GitHub của chủ dự án đang bị khoá; khi mở khoá sẽ public theo MIT hoặc Apache-2.0.
  Không tự tạo release GitHub mới nếu chưa được yêu cầu.

## 2. Kiểm tra trước khi báo xong

```bash
# Python
cd python && python3 -m pytest -q && python3 -m ruff check .

# Rust
cd rust && cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# UEFI demo
make uefi-test          # self-test 15 case + UI smoke QEMU
```

- Không commit binary trong `dist/`, `deploy/vifastkey_web/downloads/`, `uefi/build/`, `uefi/target/`.
- Giữ tên asset UEFI là `vifastkey-uefi-x86_64.zip`; version nằm ở thư mục
  `/downloads/<version>/`, không thêm hậu tố version vào tên file.

## 3. Deploy web lên `jp`

Web production chạy **trực tiếp bằng Docker trên host `jp`** (không còn qua LXD):
`root@jp` → `/opt/vifastkey` → Docker Compose → Traefik của Coolify → `https://key.vifast.org/`.
Container join network ngoài tên `coolify`; domain/TLS khai báo bằng label trong
`deploy/vifastkey_web/docker-compose.yml`.

### Cách nhanh: dùng script

```bash
./deploy/vifastkey_web/deploy-jp.sh
```

Script sẽ:

1. build UEFI + đóng gói `dist/vifastkey-uefi-x86_64.zip`;
2. sync asset vào `deploy/vifastkey_web/downloads/0.1.2/` và sinh `sha256sums.txt`;
3. stage `web/` + `python/` + `downloads/` vào `/tmp/vifastkey-stage`;
4. backup `/opt/vifastkey` trên jp (bỏ `data/`, `downloads/`);
5. rsync stage → `root@jp:/opt/vifastkey/` (giữ `data/`, `downloads/`, `.env`);
6. `docker compose up -d --build`;
7. kiểm tra `/healthz`, trang chủ có UEFI, file UEFI tải được và SHA khớp local.

Có thể override:

```bash
JP_HOST=root@jp VERSION=0.1.2 ./deploy/vifastkey_web/deploy-jp.sh
```

### Cách thủ công (khi script lỗi)

```bash
# 1) Build + stage
./uefi/package.sh
./deploy/vifastkey_web/sync-downloads.sh 0.1.2
./deploy/vifastkey_web/stage.sh /tmp/vifastkey-stage

# 2) Đưa sang jp, giữ data/.env
rsync -a --delete --exclude data --exclude downloads --exclude .env \
  /tmp/vifastkey-stage/ root@jp:/opt/vifastkey/

# 3) Build + restart
ssh root@jp 'cd /opt/vifastkey && docker compose up -d --build'
```

### Kiểm tra bắt buộc sau deploy

```bash
curl -fsS https://key.vifast.org/healthz
curl -fsS https://key.vifast.org/api/meta
curl -fsS https://key.vifast.org/ | grep -F 'vifastkey-uefi-x86_64.zip'
curl -fsS -o /dev/null -w '%{http_code} %{size_download}\n' \
  https://key.vifast.org/downloads/0.1.2/vifastkey-uefi-x86_64.zip
ssh root@jp 'cd /opt/vifastkey && docker compose logs --tail=80'
```

Không được chỉ nói “đã deploy” nếu chưa kiểm tra các URL trên. Nếu lỗi, xem log
container và khôi phục bản backup `deploy-jp.sh` đã tạo (nếu có).

### Lưu ý an toàn

- **Không** `--delete` khi chưa loại trừ `data/`, `downloads/`, `.env`.
- Giữ nguyên `/opt/vifastkey/data` (SQLite thống kê) và `.env`.
- `downloads` mount read-only; asset phải có trong stage.
- UID ghi `data/` là `10001` (user `app` trong container).
- Asset trong `downloads/` phải đọc được bởi UID `10001`; script sync/deploy đã
  `chmod 644` — nếu thêm file thủ công, kiểm tra lại quyền.
- Deploy có downtime ngắn trong lúc `docker compose up -d --build`.
- Container cũ trong LXD `hoctuthien-vps` chỉ còn là bản rollback; đừng deploy vào đó.

## 4. Sau khi đổi nội dung web

- Nếu sửa `web/index.html`/`style.css`/`server-demo.js`, bump query `?v=...`
  trong `web/index.html` để tránh cache.
- Nếu thêm asset, cập nhật `DOWNLOAD_PLATFORMS`/`DOWNLOAD_LABELS` trong
  `deploy/vifastkey_web/app.py` và chạy `sync-downloads.sh`.
- Nếu có thay đổi UEFI, cập nhật `uefi/docs/screenshot-*.png` và kiểm tra QR.
