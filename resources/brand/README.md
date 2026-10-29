# ViFastKey brand assets

Logo chính thức của dự án. Tên hiển thị luôn viết **ViFastKey** (PascalCase).

## File

| File | Dùng cho |
|---|---|
| `vifastkey-icon.svg` | icon gốc, web/app, nền trong |
| `vifastkey-lockup.svg` | logo ngang gốc, nền trắng |
| `vifastkey-lockup-transparent.svg` | logo ngang nền trong suốt (nguồn dựng PNG) |
| `vifastkey-lockup.png` | logo ngang nền trong suốt |
| `vifastkey-lockup-cream.png` | README, web, tài liệu (nền kem, đọc được ở dark mode) |
| `vifastkey-icon-512.png` | app icon, avatar, manifest |
| `vifastkey-icon-192.png` | manifest / PWA |
| `favicon.ico`, `favicon-16.png`, `favicon-32.png` | favicon |
| `apple-touch-icon.png` | iOS home screen, nền đỏ full-bleed |
| `og-image.png` | Open Graph / Twitter card 1200×630 |
| `site.webmanifest` | manifest PWA |

Web dùng PNG cho lockup để font không bị lệch giữa macOS/Windows/Linux; SVG vẫn là
nguồn thiết kế. Icon dùng PNG cho app/favicon, SVG cho web. `web/build.sh` copy các
file cần thiết sang `web/brand/` và `web/fonts/` trước khi deploy tĩnh.

## Màu

| Vai trò | Màu |
|---|---|
| Đỏ logo (gradient) | `#E5352B` → `#B3140F` |
| Đỏ wordmark / accent | `#D42A22` |
| Vàng sao (gradient) | `#FFE14D` → `#FFB800` |
| Vàng speed line | `#FFC93C` |
| Chữ chính | `#1F2430` |
| Chữ phụ | `#6B7280` |
| Nền kem | `#FFF8EC` |

Token đầy đủ cho UI nằm ở `design/tokens.json`.

## Quy ước dùng

- Kích thước icon tối thiểu: **24 px**. Dưới 32 px dùng icon, không dùng lockup.
- Lockup tối thiểu: **280 px** chiều rộng; giữ khoảng trống quanh logo ít nhất bằng
  chiều cao chữ `V`.
- Không kéo giãn, đổi màu, thêm bóng/viền, hoặc đặt trên nền nhiều chi tiết.
- Nền sáng: dùng bản gốc. Nền tối: đặt logo trên nền kem hoặc panel sáng, không
  đổi màu wordmark.
- Không dùng logo để ngụ ý chứng nhận/thương mại khi chưa có quyền.

## Dựng lại PNG

Yêu cầu macOS (Quick Look `qlmanage`) + Pillow + scipy:

```bash
python3 resources/brand/build_assets.py
```

Logo do chủ dự án cung cấp. Code repo là MIT; nhãn hiệu/logo không tự động thuộc
phạm vi giấy phép MIT.
