from vfk_core import all_emoji, search_emoji


def test_search_by_vietnamese_keyword():
    result = search_emoji("cười")
    assert result and result[0][0] in {"😀", "😁", "😂", "🤣", "😊"}


def test_search_without_diacritics():
    assert any(char == "🔥" for char, _ in search_emoji("lua"))


def test_all_unique():
    emojis = all_emoji()
    assert len(emojis) >= 50
    assert len({c for c, _ in emojis}) == len(emojis)
