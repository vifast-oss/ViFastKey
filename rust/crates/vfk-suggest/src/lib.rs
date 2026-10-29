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

pub struct SuggestionEngine {
    words: HashSet<String>,
    folded: HashMap<String, Vec<String>>,
    priority: HashSet<String>,
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
            priority: HashSet::new(),
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
    }

    pub fn suggest(&self, prefix: &str, limit: usize) -> Vec<String> {
        let folded = remove_diacritics(prefix.trim()).to_lowercase();
        if folded.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut keys: Vec<&String> = self.folded.keys().filter(|k| k.starts_with(&folded)).collect();
        keys.sort();
        for key in keys {
            if let Some(words) = self.folded.get(key) {
                out.extend(words.iter().cloned());
            }
        }
        out.sort_by_key(|w| (!self.priority.contains(w), w.chars().count(), w.clone()));
        out.dedup();
        out.truncate(if limit == 0 { self.limit } else { limit });
        out
    }

    pub fn len(&self) -> usize {
        self.words.len()
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
}
