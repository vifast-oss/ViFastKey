//! Golden test dùng chung file TSV với bản Python: testdata/golden/*.tsv
//! Định dạng: `tuỳ_chọn<TAB>phím<TAB>kết_quả` ; tuỳ chọn: telex|vni|simple_telex,old,new,final,quick,bracket,nospell,coda_k

use std::path::PathBuf;

use vfk_core::*;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_spec(spec: &str) -> (&'static InputMethodSpec, TransformOptions, bool) {
    let parts: Vec<&str> = spec.split(',').collect();
    let im = InputMethodSpec::by_name(parts[0]).unwrap_or_else(|| panic!("kiểu gõ lạ: {}", parts[0]));
    let o = TransformOptions {
        tone_style: if parts.contains(&"new") { ToneStyle::New } else { ToneStyle::Old },
        spell_check: !parts.contains(&"nospell"),
        w_as_u: true,
        bracket_keys: parts.contains(&"bracket"),
        quick_telex: parts.contains(&"quick"),
        allow_coda_k: parts.contains(&"coda_k"),
    };
    (im, o, parts.contains(&"final"))
}

#[test]
fn golden_tsv() {
    let dir = root().join("testdata/golden");
    let mut n = 0;
    let mut fails = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("thiếu testdata/golden") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("tsv") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for line in text.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            let (spec, inp, exp) = (cols[0], cols[1], cols[2]);
            let (im, o, fin) = parse_spec(spec);
            let got = transform_str(inp, im, &o, fin).text;
            n += 1;
            if got != exp {
                fails.push(format!("{}: [{spec}] {inp} -> {got} (cần {exp})", path.display()));
            }
        }
    }
    assert!(n > 100, "đọc được quá ít ca: {n}");
    assert!(fails.is_empty(), "{} ca sai:\n{}", fails.len(), fails.join("\n"));
}

#[test]
fn max_raw_is_literal() {
    let s: String = "a".repeat(MAX_RAW + 1);
    assert_eq!(transform_str(&s, &TELEX, &TransformOptions::default(), false).text, s);
}

#[test]
fn to_raw_roundtrip() {
    for w in ["tiếng", "người", "Việt", "đường", "hòa", "quyết", "giữ", "ĐI", "khuỷu", "xoong"]
    {
        for im in [&TELEX, &VNI] {
            let raw = to_raw(w, im).unwrap_or_else(|| panic!("to_raw({w})"));
            assert_eq!(transform_str(&raw, im, &TransformOptions::default(), false).text, w, "{w} qua {raw}");
        }
    }
}

#[test]
fn coda_k_rebuild() {
    let o = TransformOptions { allow_coda_k: true, ..TransformOptions::default() };
    for im in [&TELEX, &VNI] {
        assert!(rebuild("đắk", im, &o).is_some(), "{}", im.name);
    }
}

#[test]
fn english_restore_at_word_end() {
    for w in ["text", "case", "windows", "class", "law", "debug", "json"] {
        assert_eq!(transform_str(w, &TELEX, &TransformOptions::default(), true).text, w);
    }
}

/// Bộ sinh số ngẫu nhiên xác định (không cần crate rand).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn diff_rebuilds_exactly_and_no_panic() {
    let alpha: Vec<char> = "aeiouydwsfrxjzqnghtcmpbklAEOUWDS".chars().collect();
    let mut rng = Lcg(1);
    let o = TransformOptions::default();
    for _ in 0..4000 {
        let len = 1 + rng.below(10);
        let raw: Vec<char> = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
        let mut prev = String::new();
        for i in 1..=raw.len() {
            assert!(!transform(&raw[..i], &TELEX, &o, true).text.is_empty());
            let cur = transform(&raw[..i], &TELEX, &o, false).text;
            let e = diff(&prev, &cur);
            let keep: String = prev.chars().take(prev.chars().count() - e.delete_before).collect();
            assert_eq!(keep + &e.insert, cur);
            prev = cur;
        }
    }
}

#[test]
fn output_case_follows_input() {
    let alpha: Vec<char> = "tieengsnguowif".chars().collect();
    let mut rng = Lcg(2);
    let o = TransformOptions::default();
    for _ in 0..2000 {
        let len = 1 + rng.below(8);
        let raw: String = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
        let up = transform_str(&raw.to_uppercase(), &TELEX, &o, false).text;
        let lo = transform_str(&raw, &TELEX, &o, false).text;
        assert_eq!(up, lo.to_uppercase(), "{raw}");
    }
}

#[test]
fn composer_equals_transform() {
    let mut rng = Lcg(5);
    for (im, alpha) in [
        (&TELEX, "aeiouydwsfrxjzqnghtcAW"),
        (&VNI, "aeiouydnghtc0123456789"),
        (&SIMPLE_TELEX, "aeiouydwsfrxjzqnghtcAW"),
        (&VIQR, "aeiouyd^('+~?.0AEOU"),
    ] {
        let alpha: Vec<char> = alpha.chars().collect();
        for _ in 0..1500 {
            let o = TransformOptions {
                tone_style: if rng.below(2) == 0 { ToneStyle::Old } else { ToneStyle::New },
                spell_check: rng.below(10) != 0,
                ..TransformOptions::default()
            };
            let mut comp = Composer::new(im, o);
            let mut keys: Vec<char> = Vec::new();
            for _ in 0..(1 + rng.below(30)) {
                if !keys.is_empty() && rng.below(100) < 15 {
                    keys.pop();
                    comp.pop();
                } else {
                    let k = alpha[rng.below(alpha.len())];
                    keys.push(k);
                    comp.push(k);
                }
                for fin in [false, true] {
                    assert_eq!(comp.result(fin), transform(&keys, im, &o, fin), "{keys:?}");
                }
            }
        }
    }
}
