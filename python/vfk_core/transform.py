"""Hàm trung tâm: chuỗi phím thô -> chữ hiển thị. Thuần, không I/O, không trạng thái toàn cục.

Nguyên tắc: luôn tính lại toàn bộ từ chuỗi phím thô (raw). Độ dài raw bị chặn (MAX_RAW),
nên chi phí mỗi phím là O(MAX_RAW^2) với hằng số nhỏ; Rust có thể dùng mảng cố định trên stack.
"""
from __future__ import annotations

from dataclasses import dataclass, field

import vi_syllable as vs
from vi_syllable import Status, ToneStyle

from .rules import QUICK_TELEX, InputMethodSpec

MAX_RAW = 24   # vượt quá thì coi là chuỗi thô (URL, mã, token...), không biến đổi

# Âm đầu cho phép "uơ" mở (uơ, thuơ, huơ, khuơ, quơ...); các âm đầu khác phải thành "ươ".
_OPEN_UO_ONSETS = {"", "c", "h", "k", "kh", "qu", "th"}


def _vni_literal(raw_s: str, im: InputMethodSpec) -> str | None:
    """VNI: chuỗi số trông như số thật (2024, 10) hoặc cặp phím số lặp (a11)
    phải giữ nguyên, không bị biến thành dấu. Trả về chuỗi thay thế hoặc None."""
    if not im.digit_keys or not any(c.isdigit() for c in raw_s):
        return None
    # Cặp số giống nhau là escape: a11 -> a1, viet655 -> viet65.
    out, i, changed = [], 0, False
    while i < len(raw_s):
        if (raw_s[i].isdigit() and i + 1 < len(raw_s) and raw_s[i + 1] == raw_s[i]):
            out.append(raw_s[i])
            i += 2
            changed = True
        else:
            out.append(raw_s[i])
            i += 1
    if changed:
        return "".join(out)
    # Chuỗi số ở cuối có từ 2 phím thanh trở lên là số thật (nam2024, ba10);
    # còn 65/64 là tổ hợp dấu + thanh nên vẫn chuyển (viet65, nguye64n).
    tones = 0
    for ch in reversed(raw_s):
        if not ch.isdigit():
            break
        if "0" <= ch <= "5":
            tones += 1
    return raw_s if tones >= 2 else None


@dataclass(frozen=True)
class TransformOptions:
    tone_style: ToneStyle = ToneStyle.OLD
    spell_check: bool = True       # chặn biến đổi làm sai chính tả + khôi phục phím thô khi từ sai
    w_as_u: bool = True            # Telex: "w" đứng riêng -> "ư"
    bracket_keys: bool = False     # Telex: "[" -> "ơ", "]" -> "ư"
    quick_telex: bool = False      # cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th (đầu từ)
    allow_coda_k: bool = False     # cho phép âm cuối k (tên riêng: Đắk Lắk), chỉ với sắc/nặng


@dataclass
class Letter:
    base: str                     # a-z thường
    upper: bool
    mark: str | None = None       # ^ ( + -
    from_standalone: bool = False # sinh ra từ phím "w" đứng riêng / phím [ ]
    auto_horn: bool = False       # móc trên u được thêm tự động bởi luật cặp ươ


@dataclass
class WordState:
    letters: list[Letter] = field(default_factory=list)
    tone: int = 0
    undone: bool = False          # người dùng đã hoàn tác ít nhất 1 lần -> không tự khôi phục
    undo_key: str | None = None   # phím gây hoàn tác gần nhất (để phân biệt ass với coffee)
    latched: bool = False         # hoàn tác thanh (ass) -> phần còn lại của từ gõ thẳng
    locked: set = field(default_factory=set)  # phím dấu phụ đã hoàn tác (xooong) -> phím đó gõ thẳng
    transformed: bool = False     # có ít nhất 1 biến đổi tiếng Việt
    keep: bool = False            # giữ nguyên dù sai chính tả (đ ở giữa từ: vcđ, nđm)
    keep_pos: int = -1            # vị trí đ được bảo vệ


@dataclass(frozen=True)
class TransformResult:
    text: str
    restored: bool                # trả về phím thô do sai chính tả
    status: Status                # trạng thái chính tả của chữ hiển thị
    # Khi restored vì gõ đôi phím thanh (FF/SS...), đây là dạng đã bỏ phím
    # escape (ViFFa -> ViFa). Rỗng nếu không phải trường hợp này.
    rendered: str = ""


# ---------------------------------------------------------------- hiển thị
def _display(letters: list[Letter]) -> list[tuple[str, str | None, bool]]:
    """Chuẩn hoá hiển thị dấu móc:
    - ư + o -> ươ  (gõ 'uwo' vẫn ra 'ươ')
    - ươ đứng cuối từ -> uơ  (thuở, huơ: 'ươ' không bao giờ đứng cuối âm tiết)
    """
    out = [(l.base, l.mark, l.upper) for l in letters]
    n = len(out)
    for i in range(n - 1):
        if out[i][:2] == ("u", "+") and out[i + 1][:2] == ("o", None):
            out[i + 1] = ("o", "+", out[i + 1][2])
    if (n >= 2 and out[-2][:2] == ("u", "+") and out[-1][:2] == ("o", "+")
            and letters[-2].auto_horn):
        out[-2] = ("u", None, out[-2][2])
    return out


def _toneless_lower(letters: list[Letter]) -> str:
    return "".join(vs.compose_char(b, m) for b, m, _ in _display(letters))


def _render(st: WordState, style: ToneStyle) -> str:
    disp = _display(st.letters)
    chars = [vs.compose_char(b, m, 0, up) for b, m, up in disp]
    if st.tone:
        idx = vs.tone_index("".join(c.lower() for c in chars), style)
        if idx is None:                                    # 'qu' + thanh, chưa có nguyên âm chính
            idx = next((i for i in range(len(disp) - 1, -1, -1) if disp[i][0] in vs.VOWELS), None)
        if idx is not None:
            b, m, up = disp[idx]
            chars[idx] = vs.compose_char(b, m, st.tone, up)
    return "".join(chars)


def _status(letters: list[Letter], tone: int, opt: TransformOptions) -> Status:
    return vs.check(_toneless_lower(letters), tone, opt.allow_coda_k)


def _cluster_span(letters: list[Letter]) -> tuple[int, int]:
    syl = vs.parse(_toneless_lower(letters))
    return syl.cluster_start, syl.cluster_start + len(syl.cluster)


# ---------------------------------------------------------------- 1 phím
def _candidates(st: WordState, rules) -> list[tuple[tuple[int, str], ...]]:
    """Vị trí có thể nhận dấu, theo ưu tiên: cặp ươ trước, rồi từ phải sang trái."""
    start, end = _cluster_span(st.letters)
    L = st.letters
    pairs, singles = [], []
    for r in rules:
        if r.mark == "-":
            # đ có thể đứng sau phụ âm (vcdd -> vcđ, nddm -> nđm) hoặc đầu từ.
            # Nếu từ đã có nguyên âm và d ở đầu, ưu tiên d đầu (duowngd -> đường).
            if L and L[0].base == "d" and end > start:
                singles.append(((0, "-"),))
            elif L and L[-1].base == "d":
                singles.append(((len(L) - 1, "-"),))
        elif r.pair:
            for i in range(start, end - 1):
                if L[i].base == "u" and L[i + 1].base == "o":
                    pairs.append(((i, "+"), (i + 1, "+")))
        else:
            for i in range(start, end):
                if L[i].base in r.targets:
                    singles.append(((i, r.mark),))
    singles.sort(key=lambda c: -c[0][0])
    seen, out = set(), []
    for c in pairs + singles:
        if c not in seen:
            seen.add(c)
            out.append(c)
    return out


def _literal(k: str) -> Letter:
    return Letter(k.lower(), k != k.lower())


def _step(st: WordState, k: str, im: InputMethodSpec, opt: TransformOptions) -> None:
    L = st.letters
    lk = k.lower()
    if st.latched or lk in st.locked:
        L.append(_literal(k))
        return
    start, end = _cluster_span(L)
    has_vowel = end > start or (len(L) == 2 and L[0].base == "q" and L[1].base == "u")

    # 0) Gõ nhanh phụ âm đầu
    if (opt.quick_telex and len(L) == 1 and not st.transformed and L[0].base == lk
            and lk in QUICK_TELEX and L[0].mark is None):
        up = L[0].upper
        L[:] = [Letter(c, up and (i == 0 or k.isupper())) for i, c in enumerate(QUICK_TELEX[lk])]
        st.transformed = True
        return

    # 1) Thanh
    if lk in im.tones and has_vowel:
        t = im.tones[lk]
        if st.tone == t:                                   # ass -> as
            st.tone, st.undone, st.latched = 0, True, True
            st.undo_key = lk
            L.append(_literal(k))
            return
        lowered = _toneless_lower(L)
        if opt.spell_check and (not vs.tone_allowed(lowered, t, opt.allow_coda_k)
                                or vs.check(lowered, t, opt.allow_coda_k) is Status.INVALID):
            L.append(_literal(k))                          # batf -> batf (t chỉ đi với sắc/nặng)
            return
        st.tone, st.transformed = t, True
        return
    if lk == im.remove_tone and has_vowel and (st.tone or st.transformed):
        if st.tone:
            st.tone = 0
        else:                                              # không có thanh: z/0 xoá dấu phụ
            for l in L:
                if l.mark in ("^", "(", "+"):
                    l.mark = None
        return

    # 2) Dấu phụ
    if lk in im.marks:
        cands = _candidates(st, im.marks[lk])
        if cands:
            applied = [c for c in cands if all(L[i].mark == m for i, m in c)]
            pending = [c for c in cands if c not in applied]
            for c in pending:
                old = [(i, L[i].mark, L[i].auto_horn) for i, _ in c]
                protected = False
                for i, m in c:
                    if len(c) == 2 and L[i].base == "u" and L[i].mark != m:
                        onset = vs.parse(_toneless_lower(L)).onset
                        L[i].auto_horn = onset in _OPEN_UO_ONSETS
                    L[i].mark = m
                    if m == "-" and not st.keep:
                        # đ đứng sau phụ âm/đầu từ là viết tắt hợp lệ (vcđ, nđm, đc);
                        # còn "ađ" trong add/address thì vẫn phải khôi phục phím thô.
                        if i == 0:
                            protected = all(l.base not in vs.VOWELS for l in L[i + 1:])
                        elif L[i - 1].base not in vs.VOWELS:
                            protected = True
                        if protected:
                            st.keep, st.keep_pos = True, i
                if protected:                              # viết tắt chứa đ: giữ dù sai chính tả
                    st.transformed = True
                    return
                if not opt.spell_check or _status(L, st.tone, opt) is not Status.INVALID:
                    st.transformed = True
                    return
                for i, m, a in old:
                    L[i].mark, L[i].auto_horn = m, a
            if applied:                                    # aaa -> aa, ww -> w, ddd -> dd
                c = applied[0]
                iu = c[0][0]
                if (len(c) == 2 and L[iu].auto_horn and c[1][0] == len(L) - 1):
                    L[iu].auto_horn = False                # 'thuow' (thuơ) + w -> 'thươ' (kiểu UniKey: thuowwng)
                    return
                st.undone = True
                st.undo_key = lk
                st.locked.add(lk)
                i0 = c[0][0]
                if len(c) == 1 and L[i0].from_standalone:  # ư sinh từ "w" -> trả lại "w"
                    L[i0] = Letter(lk, L[i0].upper)
                    if i0 != len(L) - 1:                   # 'windows' giữ cả 2 chữ w
                        L.append(_literal(k))
                    return
                for i, _ in c:
                    L[i].mark = None
                L.append(_literal(k))
                return
            if pending and not opt.spell_check:
                for i, m in pending[0]:
                    L[i].mark = m
                st.transformed = True
                return
            # mọi ứng viên đều sai chính tả -> gõ thẳng phím (vd 'hoaa', 'book')

    # 3) Phím chèn chữ có dấu
    if opt.bracket_keys and k in im.insert_keys:
        base, mark = im.insert_keys[k]
        L.append(Letter(base, False, mark, True))
        st.transformed = True
        return
    if opt.w_as_u and lk in im.standalone:
        base, mark = im.standalone[lk]
        cand = L + [Letter(base, k != lk, mark, True)]
        if not opt.spell_check or _status(cand, st.tone, opt) is not Status.INVALID:
            L.append(cand[-1])
            st.transformed = True
            return

    L.append(_literal(k))


# ---------------------------------------------------------------- API
def _clone(st: WordState) -> WordState:
    return WordState(letters=[Letter(l.base, l.upper, l.mark, l.from_standalone, l.auto_horn)
                              for l in st.letters],
                     tone=st.tone, undone=st.undone, undo_key=st.undo_key,
                     latched=st.latched, transformed=st.transformed, locked=set(st.locked),
                     keep=st.keep, keep_pos=st.keep_pos)


def _result(st: WordState, raw_s: str, opt: TransformOptions, final: bool) -> TransformResult:
    text = _render(st, opt.tone_style)
    status = _status(st.letters, st.tone, opt)
    if opt.spell_check and st.transformed:
        start, end = _cluster_span(st.letters)
        # Hoàn tác đúng chỗ (aaa, ass, DDD, aaan) thì tôn trọng ý người gõ; còn hoàn tác
        # nằm giữa từ tiếng Anh (coffee, banana, cheese, daddy) thì khôi phục phím thô.
        low = raw_s.lower()
        deliberate_undo = bool(st.undone and st.undo_key
                               and (low.endswith(st.undo_key * 2)
                                    or st.undo_key * 3 in low))
        # Từ chỉ có nguyên âm (không có âm đầu) như aw/aws/uow được giữ dù mới là PARTIAL:
        # "ă", "ắ", "uơ" là thứ người dùng muốn thấy, không phải từ tiếng Anh dở dang.
        toneless = _toneless_lower(st.letters)
        # "dươ", "tườ", "lượ" là các cụm ươ/uơ đang gõ dở nhưng người dùng muốn thấy;
        # không khôi phục phím thô chỉ vì còn thiếu âm cuối.
        open_uo = start > 0 and toneless.endswith(("ươ", "uơ"))
        bad_partial = final and status is Status.PARTIAL and end > start and start > 0 and not open_uo
        # đ viết tắt chỉ được bảo vệ khi phía sau nó không có nguyên âm (đc, nđm, vcđ);
        # "đăk" vẫn phải khôi phục phím thô khi chưa bật allow_coda_k.
        keep_ok = st.keep and not any(l.base in vs.VOWELS for l in st.letters[st.keep_pos + 1:])
        bad = (not keep_ok and not deliberate_undo
               and (status is Status.INVALID or bad_partial))
        if bad:
            # Gõ đôi phím thanh (FF/SS...) là escape để lấy chữ thường. Với
            # từ viết hoa kiểu tên riêng (ViFFastTts), trả dạng đã bỏ phím
            # escape để tiếp tục gõ; từ thường/coffee vẫn giữ raw như cũ.
            escaped = ""
            if (st.undone and st.latched and st.undo_key == "f"
                    and not raw_s.isupper()):
                positions = [i for i, ch in enumerate(raw_s) if ch.lower() == "f"]
                # Chỉ escape FF viết hoa liền nhau trong từ mixed-case kiểu
                # ViFastTts. coffee/COFFEE và các phím thanh khác giữ raw.
                if (len(positions) >= 2
                        and positions[-1] == positions[-2] + 1
                        and all(raw_s[i].isupper() for i in positions[-2:])):
                    escaped = text
            return TransformResult(raw_s, True, status, escaped)
    return TransformResult(text, False, status)


class Composer:
    """Bộ biến đổi tăng dần: push() O(độ dài từ) thay vì chạy lại từ đầu.

    Kết quả LUÔN bằng transform(raw) (có property test). Rust: struct với ArrayVec snapshot.
    """

    def __init__(self, im: InputMethodSpec, opt: TransformOptions | None = None, raw=()):
        self.im, self.opt = im, opt or TransformOptions()
        self.raw: list[str] = []
        self._states: list[WordState] = [WordState()]
        for k in raw:
            self.push(k)

    def push(self, k: str) -> None:
        self.raw.append(k)
        if len(self.raw) > MAX_RAW:
            return
        st = _clone(self._states[-1])
        _step(st, k, self.im, self.opt)
        self._states.append(st)

    def pop(self) -> None:
        """Bỏ phím thô cuối (khác Backspace của người dùng: engine xoá theo ký tự hiển thị)."""
        if self.raw:
            if len(self.raw) <= MAX_RAW:
                self._states.pop()
            self.raw.pop()

    def result(self, final: bool = False) -> TransformResult:
        raw_s = "".join(self.raw)
        if len(self.raw) > MAX_RAW:
            return TransformResult(raw_s, False, Status.INVALID)
        lit = _vni_literal(raw_s, self.im)
        if lit is not None:
            return TransformResult(lit, True, Status.INVALID)
        return _result(self._states[-1], raw_s, self.opt, final)


def transform(raw: str | list[str], im: InputMethodSpec, opt: TransformOptions | None = None,
              final: bool = False) -> TransformResult:
    """Hàm thuần: chuỗi phím thô -> chữ. final=True khi kết thúc từ (dấu cách, Enter...):
    khôi phục cả từ chưa trọn vẹn có nguyên âm như 'lă' (law), 'tiê' (tiee)."""
    opt = opt or TransformOptions()
    raw_s = "".join(raw)
    if len(raw) > MAX_RAW:
        return TransformResult(raw_s, False, Status.INVALID)
    lit = _vni_literal(raw_s, im)
    if lit is not None:
        return TransformResult(lit, True, Status.INVALID)
    st = WordState()
    for k in raw:
        _step(st, k, im, opt)
    return _result(st, raw_s, opt, final)


def to_raw(word: str, im: InputMethodSpec) -> str | None:
    """Ngược lại: 'tiếng' -> 'tieengs'. None nếu không đảo được (ký tự lạ, thiếu phím)."""
    keys, tone, prev = [], 0, None
    for ch in word:
        b, m, t, up = vs.decompose_char(ch)
        if not ("a" <= b <= "z"):
            return None
        tone = t or tone
        keys.append(b.upper() if up else b)
        if m is None and prev == (b, None) and im.mark_key(b, "^") == b:
            keys.append(b.upper() if up else b)     # xoong -> xooong (hoàn tác phím mũ)
        prev = (b, m)
        if m:
            mk = im.mark_key(b, m)
            if mk is None:
                return None
            keys.append(mk.upper() if (up and mk.isalpha()) else mk)
    if tone:
        tk = im.tone_key(tone)
        if tk is None:
            return None
        keys.append(tk)
    return "".join(keys)


def rebuild(word: str, im: InputMethodSpec, opt: TransformOptions | None = None) -> list[str] | None:
    """Tìm chuỗi phím thô tái tạo đúng 'word' (dùng cho Backspace và sửa từ cũ)."""
    raw = to_raw(word, im)
    if raw is not None and transform(raw, im, opt).text == word:
        return list(raw)
    return None
