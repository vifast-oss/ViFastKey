# Kiến trúc ViFastKey

Tài liệu này mô tả ranh giới tái sử dụng giữa engine, adapter-core và các port.
Mục tiêu: **một quyết định chỉ nằm ở một tầng**; adapter chỉ dịch API nền tảng.

## 1. Các tầng

```
┌─────────────────────────────────────────────────────────────┐
│ Platform adapter                                            │
│ IBus · Fcitx5 · Wayland · macOS IMK · iOS · Android · Windows│
│ - nhận key/context                                          │
│ - gọi engine                                                │
│ - thi hành Plan bằng API của nền tảng                       │
└───────────────────────────┬─────────────────────────────────┘
                            │ Action / Plan
┌───────────────────────────▼─────────────────────────────────┐
│ Adapter-core (Rust + Python)                                │
│ - field policy (password/URL/number/email/terminal)         │
│ - surrounding guard (selection, mid-word, stale snapshot)   │
│ - action planning + fallback Replace                        │
│ - UTF-8/UTF-16/codepoint helpers + timing                   │
└───────────────────────────┬─────────────────────────────────┘
                            │ KeyEvent
┌───────────────────────────▼─────────────────────────────────┐
│ Engine (Python oracle + Rust port)                          │
│ - Telex/Simple Telex/VNI/VIQR                               │
│ - tone placement, spelling, macro, soft reset               │
│ - Action: Pass/Preedit/Commit/Replace                       │
└─────────────────────────────────────────────────────────────┘
```

### Engine

- Python: `python/vfk_engine/`
- Rust: `rust/crates/vfk-engine/`

Hai bản là **port có chủ đích**, không phải adapter. Chúng không được gọi API OS.
`conformance/core.jsonl` và `conformance/engine.jsonl` khóa hành vi Python ↔ Rust.

### Adapter-core

- Python: `python/vfk_adapter_core.py`
- Rust: `rust/crates/vfk-adapter-core/`
- C ABI: `vfk_plan_action_with_fallback` trong `rust/crates/vfk-ffi/`

Đây là nơi duy nhất quyết định:

- field nào bị tắt (password/email/number);
- `Replace` có an toàn không (selection, caret giữa từ, snapshot lệch);
- có thử lại `Replace` khi surrounding cũ hay không;
- xoá bao nhiêu codepoint/UTF-16 và chờ bao lâu.

Adapter không được tự gọi `deleteSurroundingText`/`replaceText` khi chưa có plan
`Replace`. Nếu planner trả `fail_safe`, adapter reset engine và để phím gốc đi tiếp.

### Platform adapter

Adapter chỉ giữ phần **không thể dùng chung**:

- map event/API của OS sang `KeyEvent`;
- lấy `before_display` và surrounding snapshot;
- thi hành `Plan` bằng preedit/commit/delete/forward;
- quản lý lifecycle: focus, cursor, field, secure input, timer/queue.

Ví dụ:

- `linux/fcitx5/src/vifastkey_state.cpp`
- `rust/apps/vfk-wayland/src/controller.rs`
- `macos/Sources/ActionApplier.swift`
- `linux/ibus/ibus_engine.py`

## 2. Nguồn dữ liệu dùng chung

### App profile

`python/vfk_engine/app_profiles.json` là nguồn duy nhất cho alias và preset:

- Python đọc lúc import;
- Rust `include_str!` lúc compile;
- `conformance/app_profiles_behavior.jsonl` khóa hành vi terminal/soft-reset.

Không hard-code lại danh sách `xterm`/`ptyxis`/Electron trong adapter hoặc trong
tài liệu cài đặt.

### Conformance

- `conformance/*.jsonl`: engine Python ↔ Rust.
- `adapter-conformance/replace.jsonl`: plan `Replace` dùng chung, gồm cả fallback.
- `conformance/app_profiles_behavior.jsonl`: app profile + terminal soft-reset.

Khi thêm behavior mới, thêm case vào conformance trước, rồi sửa planner/engine.

## 3. Quy ước chống lặp logic

1. **Quyết định** thuộc adapter-core; **thi hành** thuộc adapter.
2. Không viết lại retry `Replace` trong từng adapter — dùng
   `vfk_plan_action_with_fallback`/`plan_action_with_fallback`.
3. Không hard-code app profile trong adapter — dùng engine profile.
4. Không tự đổi UTF-16/UTF-8/codepoint — dùng helper adapter-core/C ABI.
5. Python/Rust engine được phép lặp thuật toán vì là port; mọi thay đổi phải có
   conformance tương ứng.
6. Test theo nền tảng vẫn cần cho API/lifecycle, nhưng policy phải có test chung.

## 4. Thêm một port mới

1. Map field/context sang `FieldKind`.
2. Lấy surrounding snapshot đầy đủ (cursor/anchor UTF-16).
3. Gọi engine để lấy `Action`.
4. Gọi `vfk_plan_action_with_fallback` (hoặc bản Python tương đương).
5. Thi hành `Pass/Preedit/Commit/Replace/FailSafe`.
6. Thêm test adapter cho lifecycle; không copy logic guard/retry.
7. Chạy `adapter-conformance` và `conformance/app_profiles_behavior.jsonl`.
