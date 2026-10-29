//! Emoji nhỏ, tìm theo tên/từ khoá tiếng Việt. Không mạng.

use crate::charset::remove_diacritics;

const EMOJI: &[(&str, &str, &str)] = &[
    ("😀", "mặt cười", "cuoi vui smile happy"),
    ("😁", "cười tươi", "cuoi vui grin"),
    ("😂", "cười ra nước mắt", "cuoi vui joy tears"),
    ("🤣", "cười lăn", "cuoi vui rofl"),
    ("😊", "cười hiền", "cuoi vui blush"),
    ("😍", "mắt trái tim", "yeu thich love heart eyes"),
    ("🥰", "yêu thương", "yeu thuong love"),
    ("😘", "hôn gió", "hon kiss"),
    ("😎", "đeo kính", "cool kinh"),
    ("🤔", "suy nghĩ", "suy nghi think"),
    ("😅", "cười ngượng", "cuoi nguong sweat"),
    ("😢", "khóc", "khoc buon sad"),
    ("😭", "khóc to", "khoc buon sob"),
    ("😡", "giận", "gian tuc angry"),
    ("🥺", "mắt long lanh", "xin please"),
    ("😴", "buồn ngủ", "ngu sleep"),
    ("🤒", "ốm", "om sick"),
    ("🤝", "bắt tay", "bat tay handshake"),
    ("👍", "like", "thich ok good"),
    ("👎", "dislike", "khong thich bad"),
    ("👏", "vỗ tay", "vo tay clap"),
    ("🙏", "cảm ơn", "cam on thanks pray"),
    ("💪", "mạnh mẽ", "manh me strong"),
    ("👌", "ok", "ok tuyet"),
    ("❤️", "trái tim", "yeu love heart"),
    ("💔", "tim vỡ", "buon love broken"),
    ("🔥", "lửa", "lua hot fire"),
    ("⭐", "ngôi sao", "sao star"),
    ("✨", "lấp lánh", "lap lanh sparkle"),
    ("🎉", "tiệc", "tiec party celebrate"),
    ("🎊", "pháo giấy", "phao giay confetti"),
    ("🎁", "quà", "qua gift"),
    ("☕", "cà phê", "ca phe coffee"),
    ("🍜", "mì", "mi noodle"),
    ("🍕", "pizza", "pizza"),
    ("🍺", "bia", "bia beer"),
    ("🚀", "tên lửa", "ten lua rocket"),
    ("💡", "bóng đèn", "bong den idea"),
    ("📌", "ghim", "ghim pin"),
    ("📎", "kẹp giấy", "kep giay clip"),
    ("✅", "xong", "xong ok check"),
    ("❌", "sai", "sai no cross"),
    ("⚠️", "cảnh báo", "canh bao warning"),
    ("❓", "hỏi", "hoi question"),
    ("❗", "chú ý", "chu y exclamation"),
    ("💯", "100", "100 tuyet"),
    ("🐱", "mèo", "meo cat"),
    ("🐶", "chó", "cho dog"),
    ("🌸", "hoa anh đào", "hoa sakura"),
    ("🌹", "hoa hồng", "hoa hong rose"),
    ("🌞", "mặt trời", "mat troi sun"),
    ("🌙", "mặt trăng", "mat trang moon"),
    ("☀️", "nắng", "nang sun"),
    ("🌧️", "mưa", "mua rain"),
    ("🍀", "cỏ bốn lá", "co bon la luck"),
    ("🎵", "nốt nhạc", "not nhac music"),
    ("🎶", "nhạc", "nhac music"),
    ("📱", "điện thoại", "dien thoai phone"),
    ("💻", "laptop", "laptop computer"),
    ("⌨️", "bàn phím", "ban phim keyboard"),
    ("🖱️", "chuột", "chuot mouse"),
    ("📷", "máy ảnh", "may anh camera"),
    ("🎮", "game", "game choi"),
    ("🏠", "nhà", "nha home"),
    ("🚗", "ô tô", "o to car"),
    ("✈️", "máy bay", "may bay plane"),
    ("🏖️", "biển", "bien beach"),
    ("🇻🇳", "Việt Nam", "viet nam vietnam"),
];

pub fn search(query: &str, limit: usize) -> Vec<(&'static str, &'static str)> {
    let q = remove_diacritics(query.trim()).to_lowercase();
    if q.is_empty() {
        return EMOJI.iter().take(limit).map(|(c, n, _)| (*c, *n)).collect();
    }
    let mut scored: Vec<(u8, usize, &'static str, &'static str)> = EMOJI
        .iter()
        .filter_map(|(char, name, keys)| {
            let hay = remove_diacritics(&format!("{name} {keys}")).to_lowercase();
            if hay.contains(&q) {
                let score = if hay.starts_with(&q) { 0 } else { 1 };
                Some((score, name.chars().count(), *char, *name))
            } else {
                None
            }
        })
        .collect();
    scored.sort();
    scored.into_iter().take(limit).map(|(_, _, c, n)| (c, n)).collect()
}

pub fn all_emoji() -> &'static [(&'static str, &'static str, &'static str)] {
    EMOJI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_works() {
        assert!(search("cuoi", 8).iter().any(|(c, _)| *c == "😂"));
        assert!(search("lua", 8).iter().any(|(c, _)| *c == "🔥"));
        assert!(all_emoji().len() >= 50);
    }
}
