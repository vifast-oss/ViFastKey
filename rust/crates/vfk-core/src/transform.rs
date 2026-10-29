//! Hàm trung tâm: chuỗi phím thô -> chữ hiển thị. Port nguyên văn `vfk_core/transform.py`.
//!
//! Thuần, không I/O, không trạng thái toàn cục. Mọi chỉ số tính theo `char`.
//! Mỗi `Letter` ứng đúng 1 ký tự hiển thị, nên chỉ số trong `letters` == chỉ số char của chuỗi hiển thị.

use vi_syllable::{self as vs, lower, upper, Mark, Status, ToneStyle};

use crate::rules::{quick_telex, InputMethodSpec, MarkRule};

/// Vượt quá thì coi là chuỗi thô (URL, mã, token...), không biến đổi.
pub const MAX_RAW: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformOptions {
    pub tone_style: ToneStyle,
    /// Chặn biến đổi làm sai chính tả + khôi phục phím thô khi từ sai.
    pub spell_check: bool,
    /// Telex: "w" đứng riêng -> "ư".
    pub w_as_u: bool,
    /// Telex: "[" -> "ơ", "]" -> "ư".
    pub bracket_keys: bool,
    /// cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th (đầu từ).
    pub quick_telex: bool,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            tone_style: ToneStyle::Old,
            spell_check: true,
            w_as_u: true,
            bracket_keys: false,
            quick_telex: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Letter {
    /// Chữ cơ sở thường (a-z, hoặc ký tự khác nếu gõ thẳng).
    pub base: char,
    pub upper: bool,
    pub mark: Option<Mark>,
    /// Sinh ra từ phím "w" đứng riêng / phím [ ].
    pub from_standalone: bool,
    /// Móc trên u được thêm tự động bởi luật cặp ươ.
    pub auto_horn: bool,
}

impl Letter {
    fn new(base: char, upper: bool) -> Self {
        Letter { base, upper, mark: None, from_standalone: false, auto_horn: false }
    }

    fn literal(k: char) -> Self {
        let lk = lower(k);
        Letter::new(lk, k != lk)
    }
}

#[derive(Debug, Clone, Default)]
pub struct WordState {
    pub letters: Vec<Letter>,
    pub tone: u8,
    /// Người dùng đã hoàn tác ít nhất 1 lần -> không tự khôi phục.
    pub undone: bool,
    /// Hoàn tác thanh (ass) -> phần còn lại của từ gõ thẳng.
    pub latched: bool,
    /// Phím dấu phụ đã hoàn tác (xooong) -> phím đó gõ thẳng.
    pub locked: Vec<char>,
    /// Có ít nhất 1 biến đổi tiếng Việt.
    pub transformed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformResult {
    pub text: String,
    /// Trả về phím thô do sai chính tả.
    pub restored: bool,
    /// Trạng thái chính tả của chữ hiển thị.
    pub status: Status,
}

type Disp = (char, Option<Mark>, bool);

// ---------------------------------------------------------------- hiển thị

/// Chuẩn hoá hiển thị dấu móc:
/// * ư + o -> ươ (gõ "uwo" vẫn ra "ươ")
/// * ươ đứng cuối từ, móc ở u do tự động -> uơ (thuở, huơ)
fn display(letters: &[Letter]) -> Vec<Disp> {
    let mut out: Vec<Disp> = letters.iter().map(|l| (l.base, l.mark, l.upper)).collect();
    let n = out.len();
    for i in 0..n.saturating_sub(1) {
        if out[i].0 == 'u' && out[i].1 == Some(Mark::Horn) && out[i + 1].0 == 'o' && out[i + 1].1.is_none() {
            out[i + 1] = ('o', Some(Mark::Horn), out[i + 1].2);
        }
    }
    if n >= 2
        && out[n - 2].0 == 'u'
        && out[n - 2].1 == Some(Mark::Horn)
        && out[n - 1].0 == 'o'
        && out[n - 1].1 == Some(Mark::Horn)
        && letters[n - 2].auto_horn
    {
        out[n - 2] = ('u', None, out[n - 2].2);
    }
    out
}

fn toneless_lower(letters: &[Letter]) -> String {
    display(letters).into_iter().map(|(b, m, _)| vs::compose_char(b, m, 0, false)).collect()
}

fn render(st: &WordState, style: ToneStyle) -> String {
    let disp = display(&st.letters);
    let mut chars: Vec<char> = disp.iter().map(|&(b, m, up)| vs::compose_char(b, m, 0, up)).collect();
    if st.tone != 0 {
        let lowered: String = disp.iter().map(|&(b, m, _)| vs::compose_char(b, m, 0, false)).collect();
        let idx = vs::tone_index(&lowered, style)
            // 'qu' + thanh, chưa có nguyên âm chính
            .or_else(|| (0..disp.len()).rev().find(|&i| vs::is_vowel(disp[i].0)));
        if let Some(i) = idx {
            let (b, m, up) = disp[i];
            chars[i] = vs::compose_char(b, m, st.tone, up);
        }
    }
    chars.into_iter().collect()
}

fn status(letters: &[Letter], tone: u8) -> Status {
    vs::check(&toneless_lower(letters), tone)
}

fn cluster_span(letters: &[Letter]) -> (usize, usize) {
    let syl = vs::parse(&toneless_lower(letters));
    (syl.cluster_start, syl.cluster_end())
}

// ---------------------------------------------------------------- 1 phím

/// Một ứng viên = 1 vị trí (dấu thường) hoặc 2 vị trí (cặp ươ).
type Cand = Vec<(usize, Mark)>;

/// Vị trí có thể nhận dấu, theo ưu tiên: cặp ươ trước, rồi từ phải sang trái.
fn candidates(st: &WordState, rules: &[MarkRule]) -> Vec<Cand> {
    let (start, end) = cluster_span(&st.letters);
    let l = &st.letters;
    let mut pairs: Vec<Cand> = Vec::new();
    let mut singles: Vec<Cand> = Vec::new();
    for r in rules {
        if r.mark == Mark::Stroke {
            if !l.is_empty() && l[0].base == 'd' && end > start {
                // đ cần có nguyên âm: 'dd', 'did', 'duowngd'
                singles.push(vec![(0, Mark::Stroke)]);
            } else if l.len() == 1 && l[0].base == 'd' {
                // 'dd' ở đầu từ
                singles.push(vec![(0, Mark::Stroke)]);
            }
        } else if r.pair {
            for i in start..end.saturating_sub(1) {
                if l[i].base == 'u' && l[i + 1].base == 'o' {
                    pairs.push(vec![(i, Mark::Horn), (i + 1, Mark::Horn)]);
                }
            }
        } else {
            for i in start..end {
                if r.targets.contains(l[i].base) {
                    singles.push(vec![(i, r.mark)]);
                }
            }
        }
    }
    // sort ổn định, giảm dần theo vị trí đầu
    singles.sort_by_key(|c| std::cmp::Reverse(c[0].0));
    let mut out: Vec<Cand> = Vec::new();
    for c in pairs.into_iter().chain(singles) {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

fn is_applied(l: &[Letter], c: &[(usize, Mark)]) -> bool {
    c.iter().all(|&(i, m)| l[i].mark == Some(m))
}

fn step(st: &mut WordState, k: char, im: &InputMethodSpec, opt: &TransformOptions) {
    let lk = lower(k);
    if st.latched || st.locked.contains(&lk) {
        st.letters.push(Letter::literal(k));
        return;
    }
    let (start, end) = cluster_span(&st.letters);
    let has_vowel = end > start
        || (st.letters.len() == 2 && st.letters[0].base == 'q' && st.letters[1].base == 'u');

    // 0) Gõ nhanh phụ âm đầu
    if opt.quick_telex
        && st.letters.len() == 1
        && !st.transformed
        && st.letters[0].base == lk
        && st.letters[0].mark.is_none()
    {
        if let Some(q) = quick_telex(lk) {
            let up = st.letters[0].upper;
            let k_upper = k.is_uppercase();
            st.letters = q
                .chars()
                .enumerate()
                .map(|(i, c)| Letter::new(c, up && (i == 0 || k_upper)))
                .collect();
            st.transformed = true;
            return;
        }
    }

    // 1) Thanh
    if let Some(t) = im.tone_of(lk) {
        if has_vowel {
            if st.tone == t {
                // ass -> as
                st.tone = 0;
                st.undone = true;
                st.latched = true;
                st.letters.push(Letter::literal(k));
                return;
            }
            let lowered = toneless_lower(&st.letters);
            if opt.spell_check
                && (!vs::tone_allowed(&lowered, t) || vs::check(&lowered, t) == Status::Invalid)
            {
                // batf -> batf (t chỉ đi với sắc/nặng)
                st.letters.push(Letter::literal(k));
                return;
            }
            st.tone = t;
            st.transformed = true;
            return;
        }
    }
    if lk == im.remove_tone && has_vowel && (st.tone != 0 || st.transformed) {
        if st.tone != 0 {
            st.tone = 0;
        } else {
            // không có thanh: z/0 xoá dấu phụ
            for l in st.letters.iter_mut() {
                if matches!(l.mark, Some(Mark::Circumflex | Mark::Breve | Mark::Horn)) {
                    l.mark = None;
                }
            }
        }
        return;
    }

    // 2) Dấu phụ
    if let Some(rules) = im.marks_for(lk) {
        let cands = candidates(st, rules);
        if !cands.is_empty() {
            let applied: Vec<Cand> =
                cands.iter().filter(|c| is_applied(st.letters.as_slice(), c.as_slice())).cloned().collect();
            let pending: Vec<Cand> = cands.iter().filter(|c| !applied.contains(*c)).cloned().collect();

            for c in &pending {
                let old: Vec<(usize, Option<Mark>, bool)> =
                    c.iter().map(|&(i, _)| (i, st.letters[i].mark, st.letters[i].auto_horn)).collect();
                for &(i, m) in c {
                    let l = &mut st.letters[i];
                    if c.len() == 2 && l.base == 'u' && l.mark != Some(m) {
                        l.auto_horn = true; // 'thuow' -> móc u tự động (có thể bỏ: thuở)
                    }
                    l.mark = Some(m);
                }
                if !opt.spell_check || status(&st.letters, st.tone) != Status::Invalid {
                    st.transformed = true;
                    return;
                }
                for (i, m, a) in old {
                    st.letters[i].mark = m;
                    st.letters[i].auto_horn = a;
                }
            }

            if let Some(c) = applied.first() {
                // aaa -> aa, ww -> w, ddd -> dd
                let iu = c[0].0;
                if c.len() == 2 && st.letters[iu].auto_horn && c[1].0 == st.letters.len() - 1 {
                    // 'thuow' (thuơ) + w -> 'thươ' (kiểu UniKey: thuowwng)
                    st.letters[iu].auto_horn = false;
                    return;
                }
                st.undone = true;
                if !st.locked.contains(&lk) {
                    st.locked.push(lk);
                }
                let i0 = c[0].0;
                if c.len() == 1 && st.letters[i0].from_standalone {
                    // ư sinh từ "w" -> trả lại "w"
                    let up = st.letters[i0].upper;
                    st.letters[i0] = Letter::new(lk, up);
                    if i0 != st.letters.len() - 1 {
                        // 'windows' giữ cả 2 chữ w
                        st.letters.push(Letter::literal(k));
                    }
                    return;
                }
                for &(i, _) in c {
                    st.letters[i].mark = None;
                }
                st.letters.push(Letter::literal(k));
                return;
            }

            if !pending.is_empty() && !opt.spell_check {
                for &(i, m) in &pending[0] {
                    st.letters[i].mark = Some(m);
                }
                st.transformed = true;
                return;
            }
            // mọi ứng viên đều sai chính tả -> gõ thẳng phím (vd 'hoaa', 'book')
        }
    }

    // 3) Phím chèn chữ có dấu
    if opt.bracket_keys {
        if let Some((base, mark)) = im.insert_for(k) {
            st.letters.push(Letter {
                base,
                upper: false,
                mark: Some(mark),
                from_standalone: true,
                auto_horn: false,
            });
            st.transformed = true;
            return;
        }
    }
    if opt.w_as_u {
        if let Some((base, mark)) = im.standalone_for(lk) {
            let letter =
                Letter { base, upper: k != lk, mark: Some(mark), from_standalone: true, auto_horn: false };
            let mut cand = st.letters.clone();
            cand.push(letter);
            if !opt.spell_check || status(&cand, st.tone) != Status::Invalid {
                st.letters.push(letter);
                st.transformed = true;
                return;
            }
        }
    }

    st.letters.push(Letter::literal(k));
}

// ---------------------------------------------------------------- API

fn result(st: &WordState, raw: &[char], opt: &TransformOptions, is_final: bool) -> TransformResult {
    let text = render(st, opt.tone_style);
    let stt = status(&st.letters, st.tone);
    if opt.spell_check && st.transformed {
        let (start, end) = cluster_span(&st.letters);
        // đã hoàn tác (ass -> as, boss -> bos) thì tôn trọng ý người gõ; từ tiếng Anh như 'boss'
        // được khôi phục ở engine nhờ port Dictionary.
        let bad = !st.undone
            && (stt == Status::Invalid || (is_final && stt == Status::Partial && end > start));
        if bad {
            return TransformResult { text: raw.iter().collect(), restored: true, status: stt };
        }
    }
    TransformResult { text, restored: false, status: stt }
}

/// Hàm thuần: chuỗi phím thô -> chữ.
///
/// `is_final = true` khi kết thúc từ (dấu cách, Enter...): khôi phục cả từ chưa trọn vẹn có
/// nguyên âm như "lă" (law), "tiê" (tiee).
pub fn transform(raw: &[char], im: &InputMethodSpec, opt: &TransformOptions, is_final: bool) -> TransformResult {
    if raw.len() > MAX_RAW {
        return TransformResult { text: raw.iter().collect(), restored: false, status: Status::Invalid };
    }
    let mut st = WordState::default();
    for &k in raw {
        step(&mut st, k, im, opt);
    }
    result(&st, raw, opt, is_final)
}

/// Tiện ích: `transform` nhận `&str`.
pub fn transform_str(raw: &str, im: &InputMethodSpec, opt: &TransformOptions, is_final: bool) -> TransformResult {
    let cs: Vec<char> = raw.chars().collect();
    transform(&cs, im, opt, is_final)
}

/// Bộ biến đổi tăng dần: `push()` chỉ chạy 1 bước thay vì chạy lại từ đầu.
/// Kết quả LUÔN bằng `transform(raw)` (có test so khớp).
#[derive(Debug, Clone)]
pub struct Composer {
    im: &'static InputMethodSpec,
    opt: TransformOptions,
    raw: Vec<char>,
    states: Vec<WordState>,
}

impl Composer {
    pub fn new(im: &'static InputMethodSpec, opt: TransformOptions) -> Self {
        Composer { im, opt, raw: Vec::with_capacity(MAX_RAW + 1), states: vec![WordState::default()] }
    }

    pub fn with_raw(im: &'static InputMethodSpec, opt: TransformOptions, raw: &[char]) -> Self {
        let mut c = Composer::new(im, opt);
        for &k in raw {
            c.push(k);
        }
        c
    }

    pub fn push(&mut self, k: char) {
        self.raw.push(k);
        if self.raw.len() > MAX_RAW {
            return;
        }
        let mut st = self.states.last().cloned().unwrap_or_default();
        step(&mut st, k, self.im, &self.opt);
        self.states.push(st);
    }

    /// Bỏ phím thô cuối (khác Backspace của người dùng: engine xoá theo ký tự hiển thị).
    pub fn pop(&mut self) {
        if !self.raw.is_empty() {
            if self.raw.len() <= MAX_RAW {
                self.states.pop();
            }
            self.raw.pop();
        }
    }

    pub fn raw(&self) -> &[char] {
        &self.raw
    }

    pub fn result(&self, is_final: bool) -> TransformResult {
        if self.raw.len() > MAX_RAW {
            return TransformResult { text: self.raw.iter().collect(), restored: false, status: Status::Invalid };
        }
        let st = self.states.last().expect("states luôn có ít nhất 1 phần tử");
        result(st, &self.raw, &self.opt, is_final)
    }
}

/// Ngược lại: "tiếng" -> "tieengs". `None` nếu không đảo được (ký tự lạ, thiếu phím).
pub fn to_raw(word: &str, im: &InputMethodSpec) -> Option<String> {
    let mut keys = String::new();
    let mut tone = 0u8;
    let mut prev: Option<(char, Option<Mark>)> = None;
    for ch in word.chars() {
        let d = vs::decompose_char(ch);
        let b = d.base;
        if !b.is_ascii_lowercase() {
            return None;
        }
        if d.tone != 0 {
            tone = d.tone;
        }
        let key_b = if d.upper { upper(b) } else { b };
        keys.push(key_b);
        if d.mark.is_none() && prev == Some((b, None)) && im.mark_key(b, Mark::Circumflex) == Some(b) {
            keys.push(key_b); // xoong -> xooong (hoàn tác phím mũ)
        }
        prev = Some((b, d.mark));
        if let Some(m) = d.mark {
            let mk = im.mark_key(b, m)?;
            keys.push(if d.upper && mk.is_alphabetic() { upper(mk) } else { mk });
        }
    }
    if tone != 0 {
        keys.push(im.tone_key(tone)?);
    }
    Some(keys)
}

/// Tìm chuỗi phím thô tái tạo đúng `word` (dùng cho Backspace và sửa từ cũ).
pub fn rebuild(word: &str, im: &InputMethodSpec, opt: &TransformOptions) -> Option<Vec<char>> {
    let raw: Vec<char> = to_raw(word, im)?.chars().collect();
    if transform(&raw, im, opt, false).text == word {
        Some(raw)
    } else {
        None
    }
}
