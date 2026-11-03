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

#define VFK_PLAN_PASS 0u
#define VFK_PLAN_PREEDIT 1u
#define VFK_PLAN_COMMIT 2u
#define VFK_PLAN_REPLACE 3u
#define VFK_PLAN_FAIL_SAFE 4u

#define VFK_FALLBACK_NONE 0u
#define VFK_FALLBACK_STALE_SURROUNDING 1u
#define VFK_FALLBACK_ALWAYS 2u

typedef struct VfkPlan {
    uint32_t kind;
    uint32_t delete_utf16;
    uint32_t delete_chars;
    uint32_t cursor_utf16;
    uint32_t wait_ms;
    bool pass_key;
    const char *text; /* thuộc session, hợp lệ tới lần gọi kế tiếp */
} VfkPlan;

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

/* Chốt composition trước reset/focus-out. Trả COMMIT nếu đang PREEDIT. */
VfkAction vfk_flush(VfkSession *s);

/* Chuỗi engine tin là đang hiển thị; dùng để xác minh surrounding text.
 * Chuỗi thuộc session, hợp lệ tới lần gọi kế tiếp. */
const char *vfk_display_text(VfkSession *s);

/* Đổi offset char (codepoint) sang UTF-16 code unit; UINT32_MAX nếu sai. */
uint32_t vfk_char_to_utf16_offset(const char *text, uint32_t chars);

/* Đổi offset char (codepoint) / UTF-16 code unit sang byte UTF-8; UINT32_MAX nếu sai. */
uint32_t vfk_char_to_byte_offset(const char *text, uint32_t chars);
uint32_t vfk_utf16_to_byte_offset(const char *text, uint32_t units);

/* Có phải ký tự thuộc từ (chữ/số/underscore/combining mark) không. */
bool vfk_is_word_codepoint(uint32_t codepoint);

/* Lập kế hoạch thi hành Action bằng logic adapter dùng chung.
 * - before_display: vfk_display_text() trước khi xử lý phím (nullable).
 * - surrounding_text: snapshot đầy đủ (nullable), offset UTF-16.
 * - allow_unverified_replace: chỉ dùng cho port tự quản lý toàn bộ text
 *   (Windows hook), nơi không có surrounding text.
 * Trả false nếu tham số sai; khi true, out nhận VfkPlan. */
bool vfk_plan_action(VfkSession *s, const VfkAction *action, uint32_t field_kind,
                     const char *before_display, const char *surrounding_text,
                     uint32_t cursor_utf16, uint32_t anchor_utf16,
                     bool allow_unverified_replace, VfkPlan *out);

/* Như vfk_plan_action nhưng có fallback khi Replace fail-safe vì surrounding cũ.
 * fallback: VFK_FALLBACK_NONE | VFK_FALLBACK_STALE_SURROUNDING | VFK_FALLBACK_ALWAYS. */
bool vfk_plan_action_with_fallback(VfkSession *s, const VfkAction *action, uint32_t field_kind,
                                   const char *before_display, const char *surrounding_text,
                                   uint32_t cursor_utf16, uint32_t anchor_utf16,
                                   bool allow_unverified_replace, uint32_t fallback,
                                   VfkPlan *out);

/* Đổi chế độ xuất chữ: VFK_MODE_PREEDIT hoặc VFK_MODE_REPLACE.
 * Có thể trả COMMIT/PASS để adapter chốt composition cũ trước khi đổi. */
VfkAction vfk_set_output_mode(VfkSession *s, uint32_t mode);

/* Gợi ý timing/ACK từ AppProfile hiện tại. */
uint32_t vfk_profile_backspace_delay_ms(VfkSession *s);
uint32_t vfk_profile_delay_per_char_ms(VfkSession *s);
bool vfk_profile_wait_ack(VfkSession *s);
/* 0 = không đặt, 1 = PREEDIT, 2 = REPLACE. */
uint32_t vfk_profile_output_mode(VfkSession *s);

/* Cấu hình / từ điển. Chuỗi *_json thuộc về session, hợp lệ tới lần gọi *_json kế tiếp. */
bool vfk_set_config_json(VfkSession *s, const char *json);
/* Như vfk_set_config_json nhưng trả Action cần áp trước khi dùng session tiếp.
 * Trả false nếu JSON sai; khi đó out_action không bị thay đổi. */
bool vfk_apply_config_json(VfkSession *s, const char *json, VfkAction *out_action);
const char *vfk_config_json(VfkSession *s);
uint64_t vfk_config_version(VfkSession *s);
const char *vfk_app_profile_json(VfkSession *s);
void vfk_load_dictionary(VfkSession *s, const char *english /* nullable */, const char *vietnamese /* nullable */);
/* Thêm từ (mỗi từ cách nhau khoảng trắng/xuống dòng) vào dictionary của session. */
void vfk_add_dictionary_words(VfkSession *s, const char *words, bool vietnamese);

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
