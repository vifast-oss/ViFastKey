//! Bảng mã tiếng Việt: Unicode, NFD, TCVN3, VNI-Windows, VISCII, VPS, BKHCM,
//! Vietware, CP1258, VIQR, NCR và C string.
//!
//! Dữ liệu bảng lấy từ `bamboo-core` (MIT) — xem `data/charsets.json` và
//! `THIRD_PARTY_NOTICES.md`. Không chép engine GPL.
//!
//! `encode()` trả về chuỗi Unicode của các codepoint/byte legacy (giống bamboo-core).
//! Với bảng một byte, `encode_bytes()` đổi thẳng sang `Vec<u8>`.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

const DATA: &str = include_str!("../data/charsets.json");

pub fn charset_names() -> impl Iterator<Item = &'static str> {
    CHARSETS.iter().copied()
}

pub const CHARSETS: &[&str] = &[
    "unicode",
    "unicode_nfd",
    "tcvn3",
    "vni_windows",
    "cp1258",
    "viqr",
    "viscii",
    "vps",
    "bkhcm1",
    "bkhcm2",
    "vietware_x",
    "vietware_full",
    "ncr_decimal",
    "ncr_hex",
    "cstring_hex",
    "cstring_decimal",
];

const SINGLE_BYTE: &[&str] =
    &["tcvn3", "vni_windows", "viscii", "vps", "bkhcm1", "bkhcm2", "vietware_x", "vietware_full"];

struct Table {
    display: String,
    map: HashMap<char, String>,
}

struct DecodeTable {
    chars: HashMap<String, char>,
    starts: HashSet<char>,
    max_len: usize,
}

struct Tables {
    tables: HashMap<String, Table>,
    decode: HashMap<String, DecodeTable>,
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let value: serde_json::Value = serde_json::from_str(DATA).expect("charsets.json hỏng");
        let mut tables = HashMap::new();
        let mut decode = HashMap::new();
        if let Some(obj) = value["charsets"].as_object() {
            for (key, item) in obj {
                let display = item["name"].as_str().unwrap_or(key).to_string();
                let mut map = HashMap::new();
                if let Some(m) = item["map"].as_object() {
                    for (ch, code) in m {
                        if let (Some(c), Some(s)) = (ch.chars().next(), code.as_str()) {
                            map.insert(c, s.to_string());
                        }
                    }
                }
                let mut groups: HashMap<String, Vec<char>> = HashMap::new();
                for (ch, code) in &map {
                    groups.entry(code.clone()).or_default().push(*ch);
                }
                let mut chars = HashMap::new();
                for (code, mut letters) in groups {
                    if letters.len() == 1 {
                        chars.insert(code, letters[0]);
                    } else if letters.len() == 2
                        && letters[0].to_lowercase().to_string() == letters[1].to_lowercase().to_string()
                    {
                        letters.sort_by_key(|c| !c.is_lowercase());
                        chars.insert(code, letters[0]);
                    }
                }
                if key == "viscii" {
                    for (code, ch) in [
                        ('\u{2}', 'Ẳ'),
                        ('\u{5}', 'Ẵ'),
                        ('\u{6}', 'Ẫ'),
                        ('\u{14}', 'Ỷ'),
                        ('\u{19}', 'Ỹ'),
                        ('\u{1e}', 'Ỵ'),
                        ('\u{a0}', 'Õ'),
                    ] {
                        chars.insert(code.to_string(), ch);
                    }
                }
                let starts = chars.keys().filter_map(|k| k.chars().next()).collect();
                let max_len = chars.keys().map(|k| k.chars().count()).max().unwrap_or(0);
                decode.insert(key.clone(), DecodeTable { chars, starts, max_len });
                tables.insert(key.clone(), Table { display, map });
            }
        }
        Tables { tables, decode }
    })
}

pub fn display_name(charset: &str) -> Option<&str> {
    match normalize(charset) {
        "unicode" => Some("Unicode (NFC)"),
        "unicode_nfd" => Some("Unicode tổ hợp (NFD)"),
        key => tables().tables.get(key).map(|t| t.display.as_str()),
    }
}

pub fn normalize(name: &str) -> &str {
    match name.trim().to_lowercase().as_str() {
        "unicode" | "nfc" | "utf-8" | "utf8" => "unicode",
        "unicode_nfd" | "unicode tổ hợp" | "nfd" | "decomposed" => "unicode_nfd",
        "tcvn3" | "tcvn3 (abc)" | "abc" => "tcvn3",
        "vni" | "vni windows" | "vni_windows" => "vni_windows",
        "cp1258" | "windows 1258 codepage" | "windows-1258" => "cp1258",
        "viqr" => "viqr",
        "viscii" => "viscii",
        "vps" => "vps",
        "bkhcm1" | "bkhcm 1" => "bkhcm1",
        "bkhcm2" | "bkhcm 2" => "bkhcm2",
        "vietware_x" | "vietware x" => "vietware_x",
        "vietware_full" | "vietware full" => "vietware_full",
        "ncr_decimal" | "ncr decimal" | "ncr" => "ncr_decimal",
        "ncr_hex" | "ncr hex" => "ncr_hex",
        "cstring_hex" | "unicode c string hex" => "cstring_hex",
        "cstring_decimal" | "unicode c string decimal" => "cstring_decimal",
        _ => "unicode",
    }
}

pub fn is_single_byte(charset: &str) -> bool {
    SINGLE_BYTE.contains(&normalize(charset))
}

fn is_ascii_punct(ch: char) -> bool {
    ch.is_ascii_punctuation()
}

/// Unicode -> bảng mã đích (chuỗi codepoint/byte legacy).
pub fn encode(text: &str, charset: &str) -> String {
    let key = normalize(charset);
    match key {
        "unicode" => text.nfc().collect(),
        "unicode_nfd" => text.nfd().collect(),
        "viqr" => encode_viqr(text),
        _ => {
            let Some(table) = tables().tables.get(key) else { return text.to_string() };
            text.chars().map(|ch| table.map.get(&ch).cloned().unwrap_or_else(|| ch.to_string())).collect()
        }
    }
}

fn encode_viqr(text: &str) -> String {
    let Some(table) = tables().tables.get("viqr") else { return text.to_string() };
    let codes: HashSet<&str> = table.map.values().map(String::as_str).collect();
    let mut out = String::with_capacity(text.len());
    let mut prev = String::new();
    for ch in text.chars() {
        let found = table.map.get(&ch).map(String::as_str);
        let joined = format!("{prev}{ch}");
        let escaped = ch == '\\' || (is_ascii_punct(ch) && codes.contains(joined.as_str()));
        if found.is_some() || escaped {
            if escaped {
                out.push('\\');
            }
            match found {
                Some(code) => out.push_str(code),
                None => out.push(ch),
            }
        } else {
            out.push(ch);
        }
        if escaped {
            prev.clear();
        } else {
            prev = found.unwrap_or("").to_string();
            if found.is_none() {
                prev.clear();
                prev.push(ch);
            }
        }
    }
    out
}

/// Bảng mã -> Unicode NFC. `bytes` được hiểu là từng byte một.
pub fn decode(text: &str, charset: &str) -> String {
    let key = normalize(charset);
    match key {
        "unicode" | "unicode_nfd" => return text.nfc().collect(),
        _ => {}
    }
    let Some(table) = tables().decode.get(key) else { return text.nfc().collect() };
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if key == "viqr" && chars[i] == '\\' && i + 1 < chars.len() && is_ascii_punct(chars[i + 1]) {
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }
        let ch = chars[i];
        let mut found: Option<(char, usize)> = None;
        if table.starts.contains(&ch) {
            let max = table.max_len.min(chars.len() - i);
            for n in (1..=max).rev() {
                let code: String = chars[i..i + n].iter().collect();
                if let Some(&c) = table.chars.get(&code) {
                    found = Some((c, n));
                    break;
                }
            }
        }
        match found {
            Some((c, n)) if c != ch || n > 1 => {
                out.push(c);
                i += n;
            }
            _ => {
                out.push(ch);
                i += 1;
            }
        }
    }
    out.nfc().collect()
}

pub fn decode_bytes(data: &[u8], charset: &str) -> String {
    let text: String = data.iter().map(|&b| b as char).collect();
    decode(&text, charset)
}

pub fn encode_bytes(text: &str, charset: &str) -> Option<Vec<u8>> {
    if !is_single_byte(charset) {
        return None;
    }
    encode(text, charset).chars().map(|c| u8::try_from(c as u32).ok()).collect()
}

pub fn convert(text: &str, from: &str, to: &str) -> String {
    encode(&decode(text, from), to)
}

const DETECT_PRIORITY: &[&str] = &[
    "tcvn3",
    "vni_windows",
    "viscii",
    "vps",
    "bkhcm1",
    "bkhcm2",
    "vietware_x",
    "vietware_full",
    "cp1258",
    "viqr",
    "ncr_hex",
    "ncr_decimal",
    "cstring_hex",
    "cstring_decimal",
];

pub fn detect(text: &str) -> Vec<&'static str> {
    let mut candidates: Vec<&str> =
        CHARSETS.iter().copied().filter(|c| *c != "unicode" && *c != "unicode_nfd").collect();
    if text.to_lowercase().contains("&#x") {
        candidates.sort_by_key(|c| *c != "ncr_hex");
    } else if text.contains("&#") {
        candidates.sort_by_key(|c| *c != "ncr_decimal");
    } else if text.contains("\\u") || text.contains("\\x") {
        candidates.sort_by_key(|c| *c != "cstring_hex" && *c != "cstring_decimal");
    }
    let mut scored: Vec<(usize, &'static str)> = candidates
        .into_iter()
        .filter_map(|key| {
            let decoded = decode(text, key);
            if decoded == text {
                return None;
            }
            let score = decoded
                .chars()
                .filter(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace() && !c.is_ascii())
                .count();
            Some((score, key))
        })
        .collect();
    scored.sort_by_key(|(score, key)| {
        let prio = DETECT_PRIORITY.iter().position(|p| p == key).unwrap_or(999);
        (std::cmp::Reverse(*score), prio)
    });
    scored.into_iter().map(|(_, key)| key).collect()
}

pub fn change_case(text: &str, mode: &str) -> Option<String> {
    match mode.to_lowercase().as_str() {
        "upper" => Some(text.to_uppercase()),
        "lower" => Some(text.to_lowercase()),
        "title" => Some(
            text.split_inclusive(' ')
                .map(|part| {
                    let mut c = part.chars();
                    match c.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                        None => String::new(),
                    }
                })
                .collect(),
        ),
        _ => None,
    }
}

pub fn remove_diacritics(text: &str) -> String {
    text.chars()
        .flat_map(|ch| ch.to_string().nfd().collect::<Vec<_>>())
        .filter(|ch| unicode_normalization::char::canonical_combining_class(*ch) == 0)
        .map(|ch| match ch {
            'đ' => 'd',
            'Đ' => 'D',
            c => c,
        })
        .collect::<String>()
        .nfc()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viqr_known() {
        assert_eq!(encode("tiếng Việt", "viqr"), "tie^'ng Vie^.t");
        assert_eq!(decode("tie^'ng Vie^.t", "viqr"), "tiếng Việt");
        assert_eq!(encode("mưa.", "viqr"), "mu+a\\.");
        assert_eq!(decode("mu+a\\.", "viqr"), "mưa.");
    }

    #[test]
    fn tcvn3_bytes() {
        assert_eq!(encode_bytes("Tiếng Việt", "tcvn3").unwrap(), b"Ti\xd5ng Vi\xd6t");
        assert_eq!(decode_bytes(b"Ti\xd5ng Vi\xd6t", "tcvn3"), "Tiếng Việt");
    }

    #[test]
    fn all_roundtrip() {
        for key in CHARSETS {
            let encoded = encode("Tiếng Việt: Đắk Lắk, 2024!", key);
            assert_eq!(decode(&encoded, key), "Tiếng Việt: Đắk Lắk, 2024!", "{key}");
        }
    }

    #[test]
    fn ncr_and_diacritics() {
        assert_eq!(encode("Việt", "ncr_decimal"), "Vi&#7879;t");
        assert_eq!(decode("Vi&#x1EC7;t", "ncr_hex"), "Việt");
        assert_eq!(remove_diacritics("Tiếng Việt Đắk Lắk"), "Tieng Viet Dak Lak");
    }
}
