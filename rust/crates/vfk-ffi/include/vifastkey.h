/* ViFastKey C ABI. Viết tay, khớp crates/vfk-ffi/src/lib.rs (có thể sinh lại bằng cbindgen).
 *
 * Quy ước:
 *  - Chuỗi vào/ra: UTF-8, kết thúc NUL.
 *  - delete_utf16 / cursor_utf16: đơn vị UTF-16 code unit.
 *  - VfkAction.text thuộc về session, hợp lệ tới lần gọi kế tiếp trên CÙNG session.
 *  - Một session chỉ dùng từ 1 luồng tại 1 thời điểm.
 *
 * Cách áp VfkAction (adapter):
 *  PASS    : trả phím cho OS / app.
 *  PREEDIT : đặt preedit = text, con trỏ tại cursor_utf16.
 *  REPLACE : xoá delete_utf16 code unit trước con trỏ, chèn text; nếu pass_key thì gửi tiếp phím gốc.
 *  COMMIT  : commit text, xoá preedit; nếu pass_key thì gửi tiếp phím gốc.
 */
#ifndef VIFASTKEY_H
#define VIFASTKEY_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define VFK_PASS 0u
#define VFK_PREEDIT 1u
#define VFK_REPLACE 2u
#define VFK_COMMIT 3u

#define VFK_MODE_PREEDIT 0u
#define VFK_MODE_REPLACE 1u

#define VFK_FIELD_TEXT 0u
#define VFK_FIELD_PASSWORD 1u
#define VFK_FIELD_NUMBER 2u
#define VFK_FIELD_EMAIL 3u
#define VFK_FIELD_URL 4u
#define VFK_FIELD_TERMINAL 5u

typedef struct VfkSession VfkSession;

typedef struct VfkAction {
    uint32_t kind;
    uint32_t delete_utf16;
    uint32_t cursor_utf16;
    bool pass_key;
    const char *text; /* NULL nếu PASS */
} VfkAction;

/* Vòng đời. config_json có thể NULL hoặc JSON một phần. Trả NULL nếu JSON sai. */
VfkSession *vfk_session_new(const char *config_json, uint32_t mode);
void vfk_session_free(VfkSession *s);

/* Phím: key là 1 ký tự UTF-8 hoặc tên ("Backspace", "Enter", "Escape", "Tab", "Left", "Right",
 * "Up", "Down", "Home", "End", "Delete", "PageUp", "PageDown", "Shift", "Control", "Alt", "Meta",
 * "Super", "CapsLock", "Option", "Command", "Fn"). */
VfkAction vfk_process_key(VfkSession *s, const char *key, bool ctrl, bool alt, bool meta);
VfkAction vfk_process_char(VfkSession *s, uint32_t codepoint, bool ctrl, bool alt, bool meta);

/* Ngữ cảnh */
VfkAction vfk_focus_changed(VfkSession *s, const char *app_id /* nullable */);
void vfk_set_surrounding_text(VfkSession *s, const char *before_cursor /* nullable */);
VfkAction vfk_cursor_moved(VfkSession *s);
VfkAction vfk_set_field_kind(VfkSession *s, uint32_t kind);
VfkAction vfk_toggle_vietnamese(VfkSession *s);
bool vfk_is_enabled(VfkSession *s);
void vfk_reset(VfkSession *s);

/* Cấu hình / từ điển. Chuỗi *_json thuộc về session, hợp lệ tới lần gọi *_json kế tiếp. */
bool vfk_set_config_json(VfkSession *s, const char *json);
const char *vfk_config_json(VfkSession *s);
uint64_t vfk_config_version(VfkSession *s);
const char *vfk_app_profile_json(VfkSession *s);
void vfk_load_dictionary(VfkSession *s, const char *english /* nullable */, const char *vietnamese /* nullable */);

/* Bảng mã: mỗi dòng "key\tTên hiển thị". */
const char *vfk_charset_list(void);

/* Mã hoá/giải mã Unicode <-> bảng mã. Chuỗi trả về thuộc session, hợp lệ tới lần gọi
 * charset kế tiếp. Với bảng một byte, dùng vfk_charset_encode_bytes để lấy bytes thô. */
const char *vfk_charset_encode(VfkSession *s, const char *charset, const char *text);
const char *vfk_charset_decode(VfkSession *s, const char *charset, const char *text);
const uint8_t *vfk_charset_encode_bytes(VfkSession *s, const char *charset, const char *text,
                                        size_t *out_len);
const char *vfk_charset_decode_bytes(VfkSession *s, const char *charset,
                                     const uint8_t *data, size_t len);

const char *vfk_version(void);

#ifdef __cplusplus
}
#endif

#endif /* VIFASTKEY_H */
