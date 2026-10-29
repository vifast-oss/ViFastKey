//! Chạy `conformance/core.jsonl` (sinh bởi bản Python: `python -m tools.export_conformance`).
//! Mỗi dòng: {"im","opts":{tone_style,spell_check,w_as_u,bracket_keys,quick_telex},"final","keys","expected"}

use std::path::PathBuf;

use serde_json::Value;
use vfk_core::*;

#[test]
fn core_conformance() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/core.jsonl");
    let text = std::fs::read_to_string(&path).expect("thiếu conformance/core.jsonl");
    let mut n = 0usize;
    let mut fails: Vec<String> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let c: Value = serde_json::from_str(line).unwrap();
        let im = InputMethodSpec::by_name(c["im"].as_str().unwrap()).unwrap();
        let op = &c["opts"];
        let o = TransformOptions {
            tone_style: if op["tone_style"] == "new" { ToneStyle::New } else { ToneStyle::Old },
            spell_check: op["spell_check"].as_bool().unwrap(),
            w_as_u: op["w_as_u"].as_bool().unwrap(),
            bracket_keys: op["bracket_keys"].as_bool().unwrap(),
            quick_telex: op["quick_telex"].as_bool().unwrap(),
        };
        let fin = c["final"].as_bool().unwrap();
        let keys = c["keys"].as_str().unwrap();
        let exp = c["expected"].as_str().unwrap();
        let got = transform_str(keys, im, &o, fin).text;
        n += 1;
        if got != exp {
            fails.push(format!("{} {:?} final={fin} {keys:?}: {got:?} != {exp:?}", im.name, o));
        }
    }
    assert!(n > 10_000, "quá ít ca: {n}");
    let shown: Vec<&String> = fails.iter().take(30).collect();
    assert!(fails.is_empty(), "{}/{} ca sai, 30 ca đầu:\n{:#?}", fails.len(), n, shown);
}
