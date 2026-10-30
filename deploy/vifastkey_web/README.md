# ViFastKey web

Website giới thiệu + playground + API Python cho ViFastKey.

- `app.py` — FastAPI: phục vụ landing page, API chuyển chữ bằng engine Python,
  chuyển bảng mã, thống kê ẩn danh, góp ý và `/admin`.
- `static/` — bản sao `web/` lúc build (landing page, WASM, brand, font).
- `Dockerfile` / `docker-compose.yml` — image Python 3.12, chạy uvicorn cổng 8000.
- `stage.sh` — tạo build context phẳng (vì Dockerfile cần `python/` + `static/`).

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
curl -s https://vifastkey.hoctuthien.com/api/type \
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

`/admin` mở công khai khi `ADMIN_PASSWORD` để trống. Đặt `ADMIN_PASSWORD` để bật
Basic Auth (user `admin`).

## Deploy

### Cách đang dùng: LXD `hoctuthien-vps` (Traefik của Coolify)

`hoctuthien.com` và toàn bộ subdomain đang được Traefik trên `jp` passthrough
TLS vào container LXD `hoctuthien-vps`; app `vifasttts` cũng chạy trong LXD này.
Vì vậy ViFastKey được đặt cạnh `vifasttts`:

```bash
# 1) Trên máy local: tạo build context
./deploy/vifastkey_web/stage.sh /private/tmp/vifastkey-stage

# 2) Đưa lên jp rồi vào LXD
rsync -a --delete /private/tmp/vifastkey-stage/ root@jp:/tmp/vifastkey-stage/
ssh root@jp 'lxc exec hoctuthien-vps -- mkdir -p /opt/vifastkey'
ssh root@jp 'tar -C /tmp/vifastkey-stage -czf - . | lxc exec hoctuthien-vps -- tar -xzf - -C /opt/vifastkey'

# 3) Tạo .env + data, build và chạy
ssh root@jp 'lxc exec hoctuthien-vps -- bash -lc "
  cd /opt/vifastkey &&
  install -d -o 10001 -g 10001 data &&
  test -f .env || { printf \"STATS_SALT=%s\\n\" \"\$(openssl rand -hex 24)\" > .env; chmod 600 .env; } &&
  docker compose up -d --build
"'
```

Kiểm tra:

```bash
ssh root@jp 'lxc exec hoctuthien-vps -- docker compose -f /opt/vifastkey/docker-compose.yml ps'
curl -sI https://vifastkey.hoctuthien.com/healthz
curl -s https://vifastkey.hoctuthien.com/api/meta | head -c 300
```

Coolify trên LXD tự phát hiện label Traefik của container trên network `coolify`
và cấp chứng chỉ Let's Encrypt cho `vifastkey.hoctuthien.com`.

### Nếu triển khai trực tiếp trên Coolify của `jp`

Dùng `docker-compose.yml` này với network `coolify` của host đó, hoặc tạo ứng dụng
Dockerfile trong Coolify với:

- Base directory: `/`
- Dockerfile: `/deploy/vifastkey_web/Dockerfile`
- Port: `8000`
- Domain: `vifastkey.hoctuthien.com`

Lưu ý: trên `jp`, subdomain `hoctuthien.com` đang được TCP passthrough sang LXD,
nên nếu muốn phục vụ từ Coolify của `jp` phải sửa router passthrough trước.

## Dữ liệu & quyền riêng tư

- SQLite tại `data/stats.sqlite3`, bind mount `./data:/app/data`.
- Chỉ lưu hash visitor/IP (salt ngẫu nhiên), số ký tự, kiểu gõ, thời gian xử lý.
- Không lưu nội dung gõ thử. Nội dung góp ý nằm ở `/admin`; mặc định mở công khai,
  đặt `ADMIN_PASSWORD` để bật Basic Auth.
- `/admin/feedback.csv` xuất CSV; `/admin.json` trả JSON thống kê.
- Volume phải ghi được bởi UID `10001` (user `app` trong container).
