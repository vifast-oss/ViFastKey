//! Gợi ý từ theo tiền tố, local. Không dùng model/AI; lexicon sinh từ âm tiết
//! tiếng Việt và danh sách tiếng Anh public-domain (xem THIRD_PARTY_NOTICES).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

const VIETNAMESE: &str = include_str!("../data/vietnamese_syllables.txt");
const ENGLISH: &str = include_str!("../data/english_suggest.txt");
const VIETNAMESE_COMMON: &str = include_str!("../data/vietnamese_common.txt");
const ENGLISH_COMMON: &str = include_str!("../data/english_common.txt");

/// Bỏ dấu phụ/thanh, giữ đ/Đ thành d/D.
pub fn remove_diacritics(text: &str) -> String {
    text.chars()
        .flat_map(|c| c.to_string().nfd().collect::<Vec<_>>())
        .filter(|c| unicode_normalization::char::canonical_combining_class(*c) == 0)
        .map(|c| match c {
            'đ' => 'd',
            'Đ' => 'D',
            c => c,
        })
        .collect::<String>()
        .nfc()
        .collect()
}

fn vietnamese_common_folded() -> &'static HashSet<String> {
    static COMMON: OnceLock<HashSet<String>> = OnceLock::new();
    COMMON.get_or_init(|| {
        VIETNAMESE_COMMON.split_whitespace().map(|word| remove_diacritics(word).to_lowercase()).collect()
    })
}

fn match_case(original: &str, candidate: &str) -> String {
    let all_upper = original.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase());
    if all_upper {
        return candidate.to_uppercase();
    }
    let mut chars = candidate.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        first.to_uppercase().chain(chars).collect()
    } else {
        candidate.to_string()
    }
}

/// Khoảng cách sửa lỗi (thay/thêm/xoá/hoán vị cạnh nhau), cắt sớm nếu vượt `max`.
fn osa_distance(a: &str, b: &str, max: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let mut prev_prev = vec![0usize; b.len() + 1];
    let mut prev = (0..=b.len()).collect::<Vec<_>>();
    let mut row = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let mut value = (prev[j + 1] + 1).min(row[j] + 1).min(prev[j] + cost);
            if i > 0 && j > 0 && ca == b[j - 1] && a[i - 1] == cb {
                value = value.min(prev_prev[j - 1] + 1);
            }
            row[j + 1] = value;
        }
        std::mem::swap(&mut prev_prev, &mut prev);
        std::mem::swap(&mut prev, &mut row);
    }
    let distance = prev[b.len()];
    (distance <= max).then_some(distance)
}

pub struct SuggestionEngine {
    words: HashSet<String>,
    folded: HashMap<String, Vec<String>>,
    folded_keys: Vec<String>,
    priority: HashSet<String>,
    priority_folded: Vec<(String, String)>,
    limit: usize,
}

impl SuggestionEngine {
    pub fn new<I, S>(words: I, limit: usize) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut engine = SuggestionEngine {
            words: HashSet::new(),
            folded: HashMap::new(),
            folded_keys: Vec::new(),
            priority: HashSet::new(),
            priority_folded: Vec::new(),
            limit,
        };
        engine.add_words(words);
        engine
    }

    pub fn add_words<I, S>(&mut self, words: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut added = false;
        for word in words {
            let word = word.as_ref().trim();
            if !word.is_empty() && self.words.insert(word.to_string()) {
                added = true;
            }
        }
        if added {
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.folded.clear();
        for word in &self.words {
            let key = remove_diacritics(word).to_lowercase();
            self.folded.entry(key).or_default().push(word.clone());
        }
        for list in self.folded.values_mut() {
            list.sort();
        }
        self.folded_keys = self.folded.keys().cloned().collect();
        self.folded_keys.sort();
        self.rebuild_priority();
    }

    fn rebuild_priority(&mut self) {
        self.priority_folded.clear();
        for word in &self.priority {
            self.priority_folded.push((remove_diacritics(word).to_lowercase(), word.clone()));
        }
        self.priority_folded.sort();
        self.priority_folded.dedup();
    }

    pub fn load_text(&mut self, text: &str) {
        self.add_words(text.split_whitespace());
    }

    pub fn add_priority<I, S>(&mut self, words: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for word in words {
            let word = word.as_ref().trim();
            if !word.is_empty() {
                self.priority.insert(word.to_string());
                self.add_words([word]);
            }
        }
        self.rebuild_priority();
    }

    /// Gợi ý theo tiền tố; nếu không có kết quả thì sửa lỗi gần đúng
    /// (gõ nhầm phím bên cạnh, thiếu/thừa một ký tự).
    pub fn suggest(&self, prefix: &str, limit: usize) -> Vec<String> {
        let folded = remove_diacritics(prefix.trim()).to_lowercase();
        if folded.is_empty() {
            return Vec::new();
        }
        let limit = if limit == 0 { self.limit } else { limit };
        let mut out = Vec::new();
        let start = self.folded_keys.partition_point(|k| k.as_str() < folded.as_str());
        for key in &self.folded_keys[start..] {
            if !key.starts_with(&folded) {
                break;
            }
            if let Some(words) = self.folded.get(key) {
                out.extend(words.iter().cloned());
            }
        }
        if out.is_empty() {
            return self.fuzzy_suggest(&folded, limit);
        }
        out.sort_by_key(|w| (!self.priority.contains(w), w.chars().count(), w.clone()));
        out.dedup();
        out.truncate(limit);
        out
    }

    fn fuzzy_suggest(&self, folded: &str, limit: usize) -> Vec<String> {
        let len = folded.chars().count();
        if len < 4 {
            return Vec::new();
        }
        let max = if len >= 7 { 2 } else { 1 };
        self.fuzzy_candidates(folded, max).into_iter().map(|c| c.3).take(limit).collect()
    }

    fn fuzzy_candidates(&self, folded: &str, max: usize) -> Vec<(usize, usize, usize, String)> {
        let len = folded.chars().count();
        let mut candidates: Vec<(usize, usize, usize, String)> = Vec::new();
        for (key, word) in &self.priority_folded {
            if key.chars().count().abs_diff(len) > max {
                continue;
            }
            let Some(distance) = osa_distance(folded, key, max) else {
                continue;
            };
            if distance == 0 {
                continue;
            }
            let first_mismatch = usize::from(folded.chars().next() != key.chars().next());
            candidates.push((distance, first_mismatch, word.chars().count(), word.clone()));
        }
        candidates.sort();
        candidates.dedup_by(|a, b| a.3 == b.3);
        if let Some(best) = candidates.first().map(|c| c.0) {
            candidates.retain(|c| c.0 == best);
        }
        candidates
    }

    /// Tự sửa khi từ hiện tại không hợp lệ và chỉ có đúng một ứng viên
    /// tiếng Việt thông dụng gần giống, giữ nguyên chữ hoa đầu từ.
    pub fn autocorrect(&self, word: &str) -> Option<String> {
        let trimmed = word.trim();
        let folded = remove_diacritics(trimmed).to_lowercase();
        let len = folded.chars().count();
        if len < 4 || folded.chars().any(|c| !c.is_alphabetic()) {
            return None;
        }
        if self.folded.contains_key(&folded) {
            return None; // đã là từ hợp lệ
        }
        let max = if len >= 7 { 2 } else { 1 };
        let common = vietnamese_common_folded();
        let mut corrected: Option<String> = None;
        let mut keys: HashSet<String> = HashSet::new();
        for (_, first_mismatch, _, candidate) in self.fuzzy_candidates(&folded, max) {
            if first_mismatch != 0 {
                continue;
            }
            let key = remove_diacritics(&candidate).to_lowercase();
            if !common.contains(&key) {
                continue;
            }
            let prefix = folded.chars().zip(key.chars()).take_while(|(a, b)| a == b).count();
            if prefix < 2 {
                continue;
            }
            keys.insert(key);
            corrected = Some(candidate);
        }
        if keys.len() != 1 {
            return None;
        }
        corrected.map(|word| match_case(trimmed, &word))
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(word)
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

pub fn default_engine() -> &'static SuggestionEngine {
    static ENGINE: OnceLock<SuggestionEngine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut engine = SuggestionEngine::new(std::iter::empty::<&str>(), 8);
        engine.add_priority(VIETNAMESE_COMMON.split_whitespace());
        engine.add_priority(ENGLISH_COMMON.split_whitespace());
        engine.load_text(VIETNAMESE);
        engine.load_text(ENGLISH);
        engine
    })
}

pub fn suggest(prefix: &str, limit: usize) -> Vec<String> {
    default_engine().suggest(prefix, limit)
}

pub fn autocorrect(word: &str) -> Option<String> {
    default_engine().autocorrect(word)
}

pub fn contains(word: &str) -> bool {
    default_engine().contains(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_matches_without_diacritics() {
        let words = default_engine().suggest("nguoi", 20);
        assert!(words.iter().any(|w| w == "người"), "{words:?}");
        assert!(!words.is_empty());
    }

    #[test]
    fn user_words_and_limit() {
        let mut e = SuggestionEngine::new(["tiếng", "tiền", "tie"], 2);
        assert_eq!(e.suggest("tie", 0).len(), 2);
        e.add_words(["tiểu"]);
        assert!(e.suggest("tieu", 10).contains(&"tiểu".to_string()));
        assert!(default_engine().len() > 20_000);
    }

    #[test]
    fn corrects_near_miss_word() {
        let words = default_engine().suggest("tieengx", 6);
        assert!(words.iter().any(|w| w == "tiếng"), "{words:?}");
    }

    #[test]
    fn keeps_prefix_priority_before_corrections() {
        let words = default_engine().suggest("nguoi", 20);
        assert_eq!(words.first().map(String::as_str), Some("người"), "{words:?}");
    }

    #[test]
    fn autocorrects_only_invalid_common_words() {
        assert_eq!(autocorrect("tieengd").as_deref(), Some("tiếng"));
        assert_eq!(autocorrect("Tieengd").as_deref(), Some("Tiếng"));
        assert_eq!(autocorrect("tiếng"), None);
        assert_eq!(autocorrect("hello"), None);
        assert_eq!(autocorrect("tie"), None);
    }
}
