"""Bảng emoji nhỏ, tìm theo tên/từ khoá tiếng Việt. Không phụ thuộc mạng."""
from __future__ import annotations

from .charset import remove_diacritics

# (emoji, tên, từ khoá)
_EMOJI = [
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
]


def _fold(s: str) -> str:
    return remove_diacritics(s).lower()


def search(query: str, limit: int = 8) -> list[tuple[str, str]]:
    q = _fold(query.strip())
    if not q:
        return _EMOJI[:limit]
    scored = []
    for char, name, keys in _EMOJI:
        hay = _fold(name + " " + keys)
        if q in hay:
            score = 0 if hay.startswith(q) else 1
            scored.append((score, len(name), char, name))
    scored.sort()
    return [(char, name) for _, _, char, name in scored[:limit]]


def all_emoji() -> list[tuple[str, str]]:
    return [(char, name) for char, name, _ in _EMOJI]


__all__ = ["all_emoji", "search"]
