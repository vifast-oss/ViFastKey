//! Timing policy dùng chung.
//!
//! Adapter nào cũng cần một khoảng chờ sau khi xoá surrounding text trước khi
//! commit. Android/macOS/iOS hiện dùng 0 hoặc tự đoán; Fcitx5 dùng AppProfile.
//! Gom về đây để các port cùng một quy tắc.

use vfk_engine::AppProfile;

pub const MIN_REPLACE_WAIT_MS: u32 = 3;
pub const MAX_REPLACE_WAIT_MS: u32 = 60;
/// Khoảng bảo vệ sau commit trước khi nhận key tiếp theo.
pub const POST_COMMIT_DELAY_MS: u32 = 10;

/// Thời gian chờ trước commit sau khi xoá `delete_chars` ký tự.
///
/// `wait_ack` bật: cộng delay cơ sở + delay theo số ký tự, trong khoảng
/// [3, 60] ms. `wait_ack` tắt: 3 ms để vẫn nhường frontend một nhịp.
pub fn replace_wait_ms(profile: &AppProfile, delete_chars: u32) -> u32 {
    if !profile.wait_ack {
        return MIN_REPLACE_WAIT_MS;
    }
    let computed =
        profile.backspace_delay_ms.saturating_add(profile.delay_per_char_ms.saturating_mul(delete_chars));
    computed.clamp(MIN_REPLACE_WAIT_MS, MAX_REPLACE_WAIT_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_fast() {
        let profile = AppProfile::default();
        assert_eq!(replace_wait_ms(&profile, 5), MIN_REPLACE_WAIT_MS);
    }

    #[test]
    fn wait_ack_scales_with_chars() {
        let profile = AppProfile {
            wait_ack: true,
            backspace_delay_ms: 2,
            delay_per_char_ms: 2,
            ..AppProfile::default()
        };
        assert_eq!(replace_wait_ms(&profile, 1), 4);
        assert_eq!(replace_wait_ms(&profile, 5), 12);
        assert_eq!(replace_wait_ms(&profile, 100), MAX_REPLACE_WAIT_MS);
    }
}
