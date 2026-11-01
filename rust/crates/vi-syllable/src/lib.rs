//! vi-syllable: mô hình âm tiết tiếng Việt (thuần, không I/O).
//!
//! Port nguyên văn từ `vi_syllable/__init__.py` (bản Python v4, là oracle).
//! Âm tiết = âm đầu (onset) + cụm nguyên âm (âm đệm + âm chính + bán âm cuối) + âm cuối (coda) + thanh.
//!
//! Quy ước:
//! * Mọi chỉ số (index, độ dài) tính theo `char` (Unicode scalar), KHÔNG theo byte.
//! * Các hàm phân tích (`parse`, `check`, `tone_index`) nhận chuỗi **thường, không thanh, NFC**.
//! * Thanh: 0 ngang, 1 sắc, 2 huyền, 3 hỏi, 4 ngã, 5 nặng.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::iter;
use spin::Once;

use unicode_normalization::UnicodeNormalization;

// ---------------------------------------------------------------- kiểu dữ liệu

/// Trạng thái chính tả của 1 âm tiết.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// Âm tiết hoàn chỉnh hợp lệ.
    Valid,
    /// Chưa hoàn chỉnh nhưng có thể thành hợp lệ bằng cách gõ thêm chữ và/hoặc thêm dấu phụ.
    Partial,
    /// Không thể là âm tiết tiếng Việt.
    Invalid,
}

/// Kiểu đặt thanh cho `oa`, `oe`, `uy` không có âm cuối.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ToneStyle {
    /// Kiểu cũ: hòa, thúy.
    #[default]
    Old,
    /// Kiểu mới: hoà, thuý.
    New,
}

/// Dấu phụ. Ký hiệu trong bản Python: `^` mũ, `(` trăng, `+` móc, `-` gạch (đ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    Circumflex,
    Breve,
    Horn,
    Stroke,
}

impl Mark {
    /// Ký tự tổ hợp Unicode (Stroke không có, vì `đ` là ký tự riêng).
    pub fn combining(self) -> Option<char> {
        match self {
            Mark::Circumflex => Some('\u{0302}'),
            Mark::Breve => Some('\u{0306}'),
            Mark::Horn => Some('\u{031B}'),
            Mark::Stroke => None,
        }
    }

    fn from_combining(c: char) -> Option<Mark> {
        match c {
            '\u{0302}' => Some(Mark::Circumflex),
            '\u{0306}' => Some(Mark::Breve),
            '\u{031B}' => Some(Mark::Horn),
            _ => None,
        }
    }
}

fn tone_combining(tone: u8) -> Option<char> {
    match tone {
        1 => Some('\u{0301}'),
        2 => Some('\u{0300}'),
        3 => Some('\u{0309}'),
        4 => Some('\u{0303}'),
        5 => Some('\u{0323}'),
        _ => None,
    }
}

fn combining_tone(c: char) -> Option<u8> {
    match c {
        '\u{0301}' => Some(1),
        '\u{0300}' => Some(2),
        '\u{0309}' => Some(3),
        '\u{0303}' => Some(4),
        '\u{0323}' => Some(5),
        _ => None,
    }
}

/// Kết quả tách âm tiết.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Syllable {
    pub onset: String,
    pub cluster: String,
    pub coda: String,
    /// Vị trí (theo char) bắt đầu cụm nguyên âm.
    pub cluster_start: usize,
}

impl Syllable {
    /// Vị trí (theo char) ngay sau cụm nguyên âm.
    pub fn cluster_end(&self) -> usize {
        self.cluster_start + self.cluster.chars().count()
    }
}

/// Ký tự đã tách: chữ cơ sở (thường), dấu phụ, thanh, viết hoa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decomposed {
    pub base: char,
    pub mark: Option<Mark>,
    pub tone: u8,
    pub upper: bool,
}

// ---------------------------------------------------------------- bảng dữ liệu

pub const VOWELS: &str = "aăâeêioôơuưy";
const MARKED_VOWELS: &str = "ăâêôơư";
const FRONT_VOWELS: &str = "eêiy";

/// Âm đầu, dài trước để so khớp dài nhất. "gi" và "qu" xử lý riêng trong `parse`.
const ONSETS: &[&str] = &[
    "ngh", "ng", "nh", "gh", "kh", "ph", "th", "tr", "ch", "b", "c", "d", "đ", "g", "h", "k", "l", "m", "n",
    "p", "r", "s", "t", "v", "x",
];

const CODAS: &[&str] = &["c", "ch", "m", "n", "ng", "nh", "p", "t"];
const STOP_CODAS: &[&str] = &["c", "ch", "p", "t"];
/// Âm cuối k chỉ dùng cho tên riêng/địa danh (Đắk Lắk); bật bằng `allow_coda_k`.
const CODAS_WITH_K: &[&str] = &["c", "ch", "m", "n", "ng", "nh", "p", "t", "k"];
const STOP_CODAS_WITH_K: &[&str] = &["c", "ch", "p", "t", "k"];
const CH_NH_CLUSTERS: &[&str] = &["a", "ê", "i", "oa", "uê", "uy"];

const OPEN_CLUSTERS: &[&str] = &[
    "a", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y", "ai", "ao", "au", "ay", "âu", "ây", "eo", "êu", "ia",
    "iu", "oa", "oe", "oi", "ôi", "ơi", "ua", "uê", "ui", "uy", "ưa", "ưi", "ưu", "uơ", "iêu", "yêu", "oai",
    "oay", "oao", "oeo", "uây", "uôi", "ươi", "ươu", "uya", "uyu",
];

const CLOSED_CLUSTERS: &[&str] = &[
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "oa", "oă", "oe", "uâ", "uê", "uy", "iê", "yê",
    "uô", "ươ", "uyê", "oo",
];

#[inline]
pub fn is_vowel(c: char) -> bool {
    VOWELS.contains(c)
}

#[inline]
fn is_letter(c: char) -> bool {
    c.is_ascii_lowercase() || c == 'đ' || is_vowel(c)
}

/// Chữ thường 1 ký tự (giữ nguyên nếu lowercase sinh nhiều ký tự).
#[inline]
pub fn lower(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

/// Chữ hoa 1 ký tự (giữ nguyên nếu uppercase sinh nhiều ký tự).
#[inline]
pub fn upper(c: char) -> char {
    let mut it = c.to_uppercase();
    match (it.next(), it.next()) {
        (Some(u), None) => u,
        _ => c,
    }
}

struct Tables {
    onset_prefixes: BTreeSet<String>,
    open: BTreeSet<String>,
    closed: BTreeSet<String>,
    ch_nh: BTreeSet<String>,
    /// Cụm nguyên âm (kể cả gõ dở, chưa đủ dấu) có thể hoàn thiện.
    relaxed_any: BTreeSet<String>,
    /// coda (có thể gõ dở) -> các cụm nguyên âm (đã bỏ bớt dấu) đi được với nó.
    relaxed_for_coda: BTreeMap<String, BTreeSet<String>>,
    /// Như trên nhưng có thêm âm cuối k (tuỳ chọn tên riêng).
    relaxed_for_coda_k: BTreeMap<String, BTreeSet<String>>,
}

fn prefixes(s: &str) -> Vec<String> {
    let cs: Vec<char> = s.chars().collect();
    (1..=cs.len()).map(|i| cs[..i].iter().collect()).collect()
}

fn demark(c: char) -> char {
    match c {
        'ă' | 'â' => 'a',
        'ê' => 'e',
        'ô' | 'ơ' => 'o',
        'ư' => 'u',
        _ => c,
    }
}

/// Mọi biến thể bỏ bớt dấu phụ: "ươ" -> {ươ, uơ, ưo, uo}.
fn relax(s: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = iter::once(String::new()).collect();
    for ch in s.chars() {
        let opts = [ch, demark(ch)];
        let mut next = BTreeSet::new();
        for p in &out {
            for o in opts {
                let mut q = p.clone();
                q.push(o);
                next.insert(q);
            }
        }
        out = next;
    }
    out
}

fn tables() -> &'static Tables {
    static T: Once<Tables> = Once::new();
    T.call_once(|| {
        let mut onset_prefixes = BTreeSet::new();
        for o in ONSETS.iter().copied().chain(["gi", "qu"]) {
            onset_prefixes.extend(prefixes(o));
        }
        let open: BTreeSet<String> = OPEN_CLUSTERS.iter().map(|s| s.to_string()).collect();
        let closed: BTreeSet<String> = CLOSED_CLUSTERS.iter().map(|s| s.to_string()).collect();
        let ch_nh: BTreeSet<String> = CH_NH_CLUSTERS.iter().map(|s| s.to_string()).collect();

        let mut relaxed_any = BTreeSet::new();
        for c in open.iter().chain(closed.iter()) {
            for p in prefixes(c) {
                relaxed_any.extend(relax(&p));
            }
        }

        let mut relaxed_for_coda: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for coda in CODAS {
            for cl in CLOSED_CLUSTERS {
                if (*coda == "ch" || *coda == "nh") && !ch_nh.contains(*cl) {
                    continue;
                }
                for p in prefixes(coda) {
                    relaxed_for_coda.entry(p).or_default().extend(relax(cl));
                }
            }
        }
        let mut relaxed_for_coda_k = relaxed_for_coda.clone();
        for cl in CLOSED_CLUSTERS {
            relaxed_for_coda_k.entry("k".to_string()).or_default().extend(relax(cl));
        }

        Tables { onset_prefixes, open, closed, ch_nh, relaxed_any, relaxed_for_coda, relaxed_for_coda_k }
    })
}

// ---------------------------------------------------------------- ký tự

/// Ghép chữ cơ sở + dấu phụ + thanh thành 1 ký tự NFC.
pub fn compose_char(base: char, mark: Option<Mark>, tone: u8, is_upper: bool) -> char {
    let mut buf: [char; 3] = ['\0'; 3];
    let mut n = 0;
    match mark {
        Some(Mark::Stroke) => {
            buf[n] = if base == 'd' { 'đ' } else { base };
            n += 1;
        }
        Some(m) => {
            buf[n] = base;
            n += 1;
            if let Some(c) = m.combining() {
                buf[n] = c;
                n += 1;
            }
        }
        None => {
            buf[n] = base;
            n += 1;
        }
    }
    if let Some(t) = tone_combining(tone) {
        buf[n] = t;
        n += 1;
    }
    let mut it = buf[..n].iter().copied().nfc();
    let ch = it.next().unwrap_or(base);
    debug_assert!(it.next().is_none(), "compose_char không ra 1 ký tự: {:?}", &buf[..n]);
    if is_upper {
        upper(ch)
    } else {
        ch
    }
}

/// Tách 1 ký tự thành (chữ cơ sở thường, dấu phụ, thanh, viết hoa?).
pub fn decompose_char(ch: char) -> Decomposed {
    let lo = lower(ch);
    let is_upper = ch != lo;
    if lo == 'đ' {
        return Decomposed { base: 'd', mark: Some(Mark::Stroke), tone: 0, upper: is_upper };
    }
    let mut it = iter::once(lo).nfd();
    let base = it.next().unwrap_or(lo);
    let mut mark = None;
    let mut tone = 0;
    for c in it {
        if let Some(m) = Mark::from_combining(c) {
            mark = Some(m);
        } else if let Some(t) = combining_tone(c) {
            tone = t;
        }
    }
    Decomposed { base, mark, tone, upper: is_upper }
}

/// Bỏ thanh khỏi chuỗi, trả về (chuỗi không thanh, thanh).
pub fn strip_tone(text: &str) -> (String, u8) {
    let mut out = String::with_capacity(text.len());
    let mut tone = 0;
    for ch in text.chars() {
        let d = decompose_char(ch);
        if d.tone != 0 {
            tone = d.tone;
        }
        out.push(compose_char(d.base, d.mark, 0, d.upper));
    }
    (out, tone)
}

// ---------------------------------------------------------------- phân tích

/// Tách chuỗi thường, không thanh thành âm đầu / cụm nguyên âm / phần còn lại.
pub fn parse(s: &str) -> Syllable {
    let cs: Vec<char> = s.chars().collect();
    let onset: &str = if s.starts_with("gi") && cs.len() > 2 && is_vowel(cs[2]) {
        "gi"
    } else if s.starts_with("qu") {
        "qu"
    } else {
        ONSETS.iter().copied().find(|o| s.starts_with(o)).unwrap_or("")
    };
    let i = onset.chars().count();
    let mut j = i;
    while j < cs.len() && is_vowel(cs[j]) {
        j += 1;
    }
    Syllable {
        onset: onset.to_string(),
        cluster: cs[i..j].iter().collect(),
        coda: cs[j..].iter().collect(),
        cluster_start: i,
    }
}

fn onset_ok(onset: &str, first: char) -> bool {
    let front = FRONT_VOWELS.contains(first);
    if matches!(onset, "k" | "gh" | "ngh") && !front {
        return false;
    }
    if matches!(onset, "c" | "ng" | "q") && front {
        return false;
    }
    if onset == "g" && (first == 'e' || first == 'ê') {
        return false;
    }
    if onset == "qu" && (first == 'u' || first == 'ư') {
        return false;
    }
    true
}

/// Kiểm tra chính tả 1 âm tiết (chuỗi thường, không thanh).
///
/// * `Valid`: âm tiết hoàn chỉnh.
/// * `Partial`: còn hoàn thiện được bằng cách gõ thêm chữ và/hoặc dấu phụ
///   (cho phép gõ dấu tự do: "chuyenj" + "e" -> "chuyện").
/// * `Invalid`: không thể thành âm tiết tiếng Việt.
pub fn check(s: &str, tone: u8) -> Status {
    check_with(s, tone, false)
}

/// Như [`check`] nhưng có tuỳ chọn cho âm cuối `k` (tên riêng: Đắk Lắk).
pub fn check_with(s: &str, tone: u8, allow_coda_k: bool) -> Status {
    if s.is_empty() {
        return Status::Partial;
    }
    if s.chars().any(|c| !is_letter(c)) {
        return Status::Invalid;
    }
    let t = tables();
    let syl = parse(s);
    if syl.cluster.is_empty() {
        if !syl.coda.is_empty() {
            return if t.onset_prefixes.contains(s) { Status::Partial } else { Status::Invalid };
        }
        return Status::Partial;
    }
    if syl.coda.chars().any(is_vowel) {
        return Status::Invalid;
    }
    let mut cluster = syl.cluster.clone();
    if syl.onset == "qu" && cluster.starts_with('y') {
        cluster.insert(0, 'u'); // quýt, quỳnh, quyết: kiểm tra như vần uy / uyê
    }
    let first = syl.cluster.chars().next().unwrap_or('a');
    if !onset_ok(&syl.onset, first) {
        return Status::Invalid;
    }
    let stop_bad = matches!(tone, 2..=4);
    let codas = if allow_coda_k { CODAS_WITH_K } else { CODAS };
    let stop_codas = if allow_coda_k { STOP_CODAS_WITH_K } else { STOP_CODAS };
    let relaxed_for_coda = if allow_coda_k { &t.relaxed_for_coda_k } else { &t.relaxed_for_coda };
    let coda = syl.coda.as_str();

    // --- hợp lệ hoàn chỉnh
    if coda.is_empty() {
        if t.open.contains(&cluster) {
            return Status::Valid;
        }
    } else if codas.contains(&coda)
        && t.closed.contains(&cluster)
        && !((coda == "ch" || coda == "nh") && !t.ch_nh.contains(&cluster))
    {
        if stop_codas.contains(&coda) && stop_bad {
            return Status::Invalid;
        }
        return Status::Valid;
    }

    // --- còn có thể hoàn thiện
    if coda.is_empty() {
        return if t.relaxed_any.contains(&cluster) { Status::Partial } else { Status::Invalid };
    }
    if let Some(set) = relaxed_for_coda.get(coda) {
        if set.contains(&cluster) {
            if stop_bad && stop_codas.contains(&coda) {
                return Status::Invalid;
            }
            return Status::Partial;
        }
    }
    Status::Invalid
}

/// Như [`check_with`] nhưng dùng khi kết thúc từ.
///
/// Âm tiết có âm cuối tắc (c, ch, p, t, và k khi bật tên riêng) phải mang thanh
/// sắc/nặng mới là từ hoàn chỉnh. Nhờ vậy `dât`/`bôt` bị coi là chưa hợp lệ ở cuối từ,
/// còn `lát`/`mút` vẫn hợp lệ.
pub fn check_final(s: &str, tone: u8, allow_coda_k: bool) -> Status {
    let st = check_with(s, tone, allow_coda_k);
    if st != Status::Valid {
        return st;
    }
    let syl = parse(s);
    let stop_codas = if allow_coda_k { STOP_CODAS_WITH_K } else { STOP_CODAS };
    if stop_codas.contains(&syl.coda.as_str()) && !matches!(tone, 1 | 5) {
        Status::Invalid
    } else {
        st
    }
}

/// Âm cuối tắc (c, ch, p, t) chỉ đi với thanh sắc / nặng.
pub fn tone_allowed(s: &str, tone: u8) -> bool {
    tone_allowed_with(s, tone, false)
}

/// Như [`tone_allowed`] nhưng tính cả âm cuối `k` khi bật tên riêng.
pub fn tone_allowed_with(s: &str, tone: u8, allow_coda_k: bool) -> bool {
    let syl = parse(s);
    let stop_codas = if allow_coda_k { STOP_CODAS_WITH_K } else { STOP_CODAS };
    !(stop_codas.contains(&syl.coda.as_str()) && matches!(tone, 2..=4))
}

/// Vị trí (theo char) đặt thanh trong chuỗi thường, không thanh.
pub fn tone_index(s: &str, style: ToneStyle) -> Option<usize> {
    let syl = parse(s);
    let c: Vec<char> = syl.cluster.chars().collect();
    let st = syl.cluster_start;
    if c.is_empty() {
        return None;
    }
    if let Some(p) = c.iter().position(|&x| x == 'ơ') {
        return Some(st + p); // ươ -> ơ, uơ -> ơ
    }
    if let Some(p) = c.iter().position(|&x| MARKED_VOWELS.contains(x)) {
        return Some(st + p); // ưa, uyê, uâ ...
    }
    if !syl.coda.is_empty() {
        return Some(st + c.len() - 1); // toán, huỳnh
    }
    if c.len() == 1 {
        return Some(st);
    }
    if c.len() == 3 {
        return Some(st + 1); // oái, khuỷu, ngoèo
    }
    if matches!(syl.cluster.as_str(), "oa" | "oe" | "uy") {
        return Some(st + if style == ToneStyle::New { 1 } else { 0 }); // hòa / hoà
    }
    Some(st) // mùa, kìa, hai
}

/// Đặt thanh lên một từ đã có dấu phụ (giữ hoa/thường).
pub fn place_tone(word: &str, tone: u8, style: ToneStyle) -> String {
    let (base, _) = strip_tone(word);
    if tone == 0 {
        return base;
    }
    let idx = match tone_index(&base.to_lowercase(), style) {
        Some(i) => i,
        None => return base,
    };
    base.chars()
        .enumerate()
        .map(|(i, ch)| {
            if i == idx {
                let d = decompose_char(ch);
                compose_char(d.base, d.mark, tone, d.upper)
            } else {
                ch
            }
        })
        .collect()
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn relax_uo() {
        let r = relax("ươ");
        for v in ["ươ", "uơ", "ưo", "uo"] {
            assert!(r.contains(v), "{v}");
        }
        assert_eq!(r.len(), 4);
    }

    #[test]
    fn compose_basic() {
        assert_eq!(compose_char('e', Some(Mark::Circumflex), 1, false), 'ế');
        assert_eq!(compose_char('d', Some(Mark::Stroke), 0, true), 'Đ');
        assert_eq!(compose_char('u', Some(Mark::Horn), 2, true), 'Ừ');
    }

    #[test]
    fn check_final_stop_coda_requires_tone() {
        assert_eq!(check_final("dat", 0, false), Status::Invalid);
        assert_eq!(check_final("dat", 1, false), Status::Valid);
        assert_eq!(check_final("mô", 0, false), Status::Valid);
        assert_eq!(check_final("lat", 1, false), Status::Valid);
        assert_eq!(check_final("đăk", 1, true), Status::Valid);
        assert_eq!(check_final("đăk", 0, true), Status::Invalid);
    }

    #[test]
    fn coda_k_proper_names() {
        assert_eq!(check("đăk", 1), Status::Invalid);
        assert_eq!(check_with("đăk", 1, true), Status::Valid);
        assert_eq!(check_with("lăk", 5, true), Status::Valid);
        assert_eq!(check_with("đăk", 2, true), Status::Invalid);
        assert!(tone_allowed_with("đăk", 1, true));
        assert!(!tone_allowed_with("đăk", 3, true));
    }
}
