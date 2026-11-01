# Downloads mirror

Thư mục này chỉ chứa file release khi deploy; nội dung file binary bị `.gitignore`
bỏ qua. Cấu trúc:

```
downloads/
└── 0.1.0/
    ├── vifastkey-uefi-x86_64.zip
    ├── vifastkey-windows-hook-0.1.0.zip
    └── ...
```

Tạo/cập nhật mirror từ `dist/`:

```bash
./uefi/package.sh
./deploy/vifastkey_web/sync-downloads.sh 0.1.0
```

Script sẽ copy asset và sinh `sha256sums.txt`. Sau đó chạy `stage.sh` để đưa
`downloads/` vào build context; Docker compose mount thư mục này vào
`/app/downloads:ro`.
