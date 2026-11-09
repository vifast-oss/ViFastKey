# ViFastKey web

Website giới thiệu + playground + API Python cho ViFastKey.

- `app.py` — FastAPI: phục vụ landing page, API chuyển chữ bằng engine Python,
  chuyển bảng mã, thống kê ẩn danh, góp ý và `/admin`.
- `static/` — bản sao `web/` lúc build (landing page, WASM, brand, font).
- `downloads/` — mirror asset release (`downloads/<version>/...`), được mount vào
  `/app/downloads:ro`; nội dung binary bị gitignore.
- `Dockerfile` / `docker-compose.yml` — image Python 3.12, chạy uvicorn cổng 8000.
- `stage.sh` — tạo build context phẳng (vì Dockerfile cần `python/` + `static/`).
- `sync-downloads.sh` — copy asset từ `dist/` và sinh `sha256sums.txt`.

## API công khai

| Endpoint | Việc |
|---|---|
| `POST /api/type` | Chuỗi phím thô → chữ Việt bằng engine Python |
| `POST /api/charset` | Chuyển giữa 16 bảng mã |
| `GET /api/meta` | Phiên bản, kiểu gõ, bảng mã, giới hạn |
| `GET /api/stats` | Thống kê tổng hợp, không có nội dung người dùng |
| `POST /api/download` | Đếm lượt bấm nút tải theo nền tảng |
| `POST /api/visit` | Đếm lượt xem ẩn danh |
| `POST /api/feedback` | Góp ý/báo lỗi |
| `GET /healthz` | Healthcheck cho Docker/Traefik |

Ví dụ:

```bash
curl -s https://key.vifast.org/api/type \
  -H 'content-type: application/json' \
  -d '{"text":"Tieengs Vieejt laf ngoon ngwx","method":"telex","tone":"new"}'
# {"output":"Tiếng Việt là ngon ngữ", ...}
```

Quy ước ký tự đặc biệt giống CLI: `<` Backspace, `~` Escape, `|` Enter.
Tối đa 500 ký tự mỗi lượt; rate limit 60 lượt/phút/IP.

## Chạy local

```bash
python3 -m venv /private/tmp/vifastkey-venv
/private/tmp/vifastkey-venv/bin/pip install -r deploy/vifastkey_web/requirements.txt
PYTHONPATH=python STATIC_DIR=web \
  /private/tmp/vifastkey-venv/bin/uvicorn app:app \
  --app-dir deploy/vifastkey_web --reload --port 8091
# http://127.0.0.1:8091
```

`/admin` khoá khi `ADMIN_PASSWORD` để trống. Đặt `ADMIN_PASSWORD` rồi mở `/login`
để đăng nhập bằng **form mật khẩu** (cookie phiên ký HMAC, hạn 12 giờ). Trang quản
trị có thêm bảng **thống kê truy cập** do `admin_portal.py` dựng.

## Deploy

### Cách nhanh

```bash
./deploy/vifastkey_web/deploy-jp.sh
```

Script tự build UEFI, sync downloads, stage, rsync sang `root@jp:/opt/vifastkey`,
`docker compose up -d --build`, rồi kiểm tra healthcheck + file UEFI + SHA.
Chi tiết/rollback: [`AGENTS.md`](../../AGENTS.md).

### Cách đang dùng: Docker trực tiếp trên host `jp`

Container chạy trên **host `jp`** (không còn qua LXD), join network ngoài
`coolify`, Traefik của Coolify cấp TLS cho `https://key.vifast.org/` theo label
trong `docker-compose.yml`. Thư mục `/opt/vifastkey` giữ `data/` (SQLite) và
`downloads/` giữa các lần deploy.

```bash
# 0) Đóng gói UEFI + đồng bộ mirror tải xuống (asset binary bị gitignore)
./uefi/package.sh
./deploy/vifastkey_web/sync-downloads.sh 0.1.2

# 1) Trên máy local: tạo build context
./deploy/vifastkey_web/stage.sh /tmp/vifastkey-stage

# 2) Đưa lên jp (giữ data/downloads/.env)
rsync -a --delete --exclude data --exclude downloads --exclude .env \
  /tmp/vifastkey-stage/ root@jp:/opt/vifastkey/

# 3) Build và chạy
ssh root@jp 'cd /opt/vifastkey && docker compose up -d --build'
```

Kiểm tra:

```bash
ssh root@jp 'docker compose -f /opt/vifastkey/docker-compose.yml ps'
curl -sI https://key.vifast.org/healthz
curl -s https://key.vifast.org/api/meta | head -c 300
```

Traefik của Coolify trên `jp` tự phát hiện label Traefik của container trên network
`coolify` và cấp chứng chỉ Let's Encrypt cho `key.vifast.org`.

### Nếu triển khai trực tiếp trên Coolify của `jp`

Dùng `docker-compose.yml` này với network `coolify` của host đó, hoặc tạo ứng dụng
Dockerfile trong Coolify với:

- Base directory: `/`
- Dockerfile: `/deploy/vifastkey_web/Dockerfile`
- Port: `8000`
- Domain: `key.vifast.org`

Lưu ý: trên `jp`, subdomain `hoctuthien.com` đang được TCP passthrough sang LXD,
nên nếu muốn phục vụ từ Coolify của `jp` phải sửa router passthrough trước.

## Dữ liệu & quyền riêng tư

- SQLite tại `data/stats.sqlite3`, bind mount `./data:/app/data`.
- Chỉ lưu hash visitor/IP (salt ngẫu nhiên), số ký tự, kiểu gõ, thời gian xử lý.
- Không lưu nội dung gõ thử. Nội dung góp ý nằm ở `/admin`; mặc định khoá,
  đặt `ADMIN_PASSWORD` để bật form đăng nhập ở `/login`.
- Lượt xem trang cho bảng thống kê nằm ở bảng `hits` cùng file SQLite; không lưu
  IP thô, chỉ hash có muối đổi mỗi ngày.
- Lượt bấm tải chỉ lưu `platform`, `asset`, `source`, thời gian và hash visitor;
  không lưu IP thô hay nội dung trang.
- `/admin/feedback.csv` xuất CSV; `/admin.json` trả JSON thống kê.
- Volume phải ghi được bởi UID `10001` (user `app` trong container).
