"""Sinh các cách gõ hợp lệ khác nhau của cùng 1 từ (gõ dấu tự do)."""
import vi_syllable as vs


def structure(word, im):
    """[(phím chữ, phím dấu phụ | None)], phím thanh | None. None nếu không đảo được."""
    units, tone = [], 0
    for ch in word:
        b, m, t, up = vs.decompose_char(ch)
        if not ("a" <= b <= "z"):
            return None
        tone = t or tone
        mk = None
        if m:
            mk = im.mark_key(b, m)
            if mk is None:
                return None
            if up and mk.isalpha():
                mk = mk.upper()
        units.append((b.upper() if up else b, mk))
    return units, (im.tone_key(tone) if tone else None)


def variants(word, im):
    st = structure(word, im)
    if st is None:
        return
    units, tk = st
    letters = [u for u, _ in units]
    first_vowel = next((i for i, l in enumerate(letters) if l.lower() in "aeiouy"), None)
    marks = [m for _, m in units if m]
    # với cặp ươ chỉ cần 1 phím móc khi dồn cuối
    if im.name != "vni" and "w" in [m.lower() for m in marks if m]:
        pass
    dedup_marks = []
    for m in marks:
        if not (dedup_marks and m.lower() in "w7" and dedup_marks[-1].lower() == m.lower()):
            dedup_marks.append(m)
    # 1) gõ dấu phụ ngay sau chữ, thanh ở mọi vị trí sau nguyên âm đầu
    inline = []
    for u, m in units:
        inline.append(u)
        if m:
            inline.append(m)
    positions = range(len(inline) + 1)
    if tk and first_vowel is not None:
        fv = inline.index(letters[first_vowel])
        for p in positions:
            if p > fv:
                yield "".join(inline[:p] + [tk] + inline[p:])
    else:
        yield "".join(inline)
    # 2) dồn dấu phụ về cuối (rồi thanh), và thanh trước dấu phụ
    tail = dedup_marks + ([tk] if tk else [])
    yield "".join(letters + tail)
    if tk:
        yield "".join(letters + [tk] + dedup_marks)
