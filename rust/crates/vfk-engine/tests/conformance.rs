//! Chạy `conformance/engine.jsonl`: 3.000 kịch bản, so TỪNG Action + văn bản cuối.
//! Mỗi dòng: {"mode","config","prefix","keys":[..],"actions":[{"t":..}],"text"}

use std::path::PathBuf;

use serde_json::{json, Value};
use vfk_engine::testing::Harness;
use vfk_engine::*;

fn action_json(a: &Action) -> Value {
    match a {
        Action::PassThrough => json!({"t": "pass"}),
        Action::Preedit { text, cursor } => json!({"t": "preedit", "text": text, "cursor": cursor}),
        Action::Commit { text, pass_key } => json!({"t": "commit", "text": text, "pass_key": pass_key}),
        Action::Replace { delete_before, insert, pass_key } => {
            json!({"t": "replace", "del": delete_before, "ins": insert, "pass_key": pass_key})
        }
    }
}

#[test]
fn engine_conformance() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/engine.jsonl");
    let text = std::fs::read_to_string(&path).expect("thiếu conformance/engine.jsonl");
    let mut n = 0usize;
    let mut fails: Vec<String> = Vec::new();
    for (lineno, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
        let c: Value = serde_json::from_str(line).unwrap();
        let config: Config = serde_json::from_value(c["config"].clone()).expect("config JSON");
        let mode: OutputMode = serde_json::from_value(c["mode"].clone()).unwrap();
        let prefix = c["prefix"].as_str().unwrap();
        let mut h = Harness::new(mode, config, "app", prefix);

        let mut acts = Vec::new();
        let mut invariant_broken = None;
        for k in c["keys"].as_array().unwrap() {
            let k = k.as_str().unwrap();
            let a = h.field.key(&mut h.session, k);
            acts.push(action_json(&a));
            // bất biến: màn hình khớp session với MỌI cấu hình
            let f = h.field.0.borrow();
            let shown = f.text();
            if invariant_broken.is_none() && !shown.ends_with(&h.session.display()) {
                invariant_broken =
                    Some(format!("sau phím {k:?}: màn hình {shown:?}, session {:?}", h.session.display()));
            }
        }
        n += 1;
        let exp_acts = c["actions"].as_array().unwrap();
        if let Some(msg) = invariant_broken {
            fails.push(format!("dòng {}: bất biến hỏng {msg}", lineno + 1));
        } else if &acts != exp_acts || h.text() != c["text"].as_str().unwrap() {
            let first = acts.iter().zip(exp_acts.iter()).position(|(a, b)| a != b);
            fails.push(format!(
                "dòng {}: keys={} ; lệch ở phím #{first:?}: rust={:?} python={:?} ; text rust={:?} python={:?}",
                lineno + 1,
                c["keys"],
                first.map(|i| &acts[i]),
                first.map(|i| &exp_acts[i]),
                h.text(),
                c["text"]
            ));
        }
    }
    assert!(n >= 1000, "quá ít kịch bản: {n}");
    let shown: Vec<&String> = fails.iter().take(15).collect();
    assert!(fails.is_empty(), "{}/{} kịch bản sai, 15 đầu:\n{:#?}", fails.len(), n, shown);
}
