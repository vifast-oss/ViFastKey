//! Port của tests/test_engine.py.

use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;
use vfk_engine::testing::Harness;
use vfk_engine::*;
use vi_syllable::ToneStyle;

const MODES: [OutputMode; 2] = [OutputMode::Replace, OutputMode::Preedit];
const BS: &str = "Backspace";

fn macros() -> HashMap<String, String> {
    HashMap::from([("ko".to_string(), "không".to_string()), ("vn".to_string(), "Việt Nam".to_string())])
}

fn cfg() -> Config {
    Config { macros: macros(), ..Config::default() }
}

fn make(mode: OutputMode) -> Harness {
    Harness::new(mode, cfg(), "notepad", "")
}

fn make_with(mode: OutputMode, config: Config) -> Harness {
    Harness::new(mode, config, "notepad", "")
}

fn chars(s: &str) -> Vec<String> {
    s.chars().map(|c| c.to_string()).collect()
}

fn keys(parts: &[&str]) -> Vec<String> {
    // phần tử dài hơn 1 ký tự và là tên phím -> giữ nguyên; còn lại tách từng ký tự
    let named = ["Backspace", "Escape", "Enter", "Left", "Shift", "Option", "Tab"];
    let mut out = Vec::new();
    for p in parts {
        if named.contains(p) {
            out.push(p.to_string());
        } else {
            out.extend(chars(p));
        }
    }
    out
}

fn type_mixed(h: &mut Harness, parts: &[&str]) {
    for k in keys(parts) {
        h.field.key(&mut h.session, &k);
    }
}

// ---------------------------------------------------------------- gõ cơ bản

#[test]
fn sentence() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("Tieengs Vieejt raast hay, nguwowif Vieejt thif thuowng nhau. ");
        assert_eq!(h.text(), "Tiếng Việt rất hay, người Việt thì thương nhau. ", "{m:?}");
    }
}

#[test]
fn vni_sentence() {
    for m in MODES {
        let mut h = make_with(m, Config { input_method: "vni".into(), ..cfg() });
        h.type_str("Vie65t Nam, nguo7i2 d9i ho5c 2 na8m. ");
        assert_eq!(h.text(), "Việt Nam, người đi học 2 năm. ", "{m:?}");
    }
}

#[test]
fn new_tone_style() {
    for m in MODES {
        let mut h = make_with(m, Config { tone_style: ToneStyle::New, ..cfg() });
        h.type_str("hoaf bifnh thuyr ");
        assert_eq!(h.text(), "hoà bình thuỷ ", "{m:?}");
    }
}

// ---------------------------------------------------------------- backspace

#[test]
fn backspace_deletes_visible_char_keeps_marks() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["tieengs", BS]);
        assert_eq!(h.text(), "tiến");
        h.type_str("g ");
        assert_eq!(h.text(), "tiếng ");
    }
}

#[test]
fn backspace_then_change_tone() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["nguwowif", BS]);
        assert_eq!(h.text(), "ngườ");
        h.type_str("ix ");
        assert_eq!(h.text(), "ngưỡi ");
    }
}

#[test]
fn backspace_into_literal_word() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["text", BS]);
        assert_eq!(h.text(), "tex");
        h.type_str("t ");
        assert_eq!(h.text(), "text ");
    }
}

#[test]
fn backspace_nfd() {
    for m in MODES {
        let mut h = make_with(m, Config { unicode_form: UnicodeForm::Nfd, ..cfg() });
        type_mixed(&mut h, &["vieejt", BS]);
        assert_eq!(h.text(), "việ".nfd().collect::<String>());
    }
}

// ---------------------------------------------------------------- kết thúc từ

#[test]
fn macro_and_case() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("ko Ko KO vn ");
        assert_eq!(h.text(), "không Không KHÔNG Việt Nam ");
    }
}

#[test]
fn dictionary_restore() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("data her mix this did ");
        assert_eq!(h.text(), "data her mix thí đi ");
    }
}

#[test]
fn double_key_english_restored_by_dictionary() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("server boss pass miss ass ");
        assert_eq!(h.text(), "server boss pass miss as ");
    }
}

#[test]
fn spell_restore_without_dictionary() {
    for m in MODES {
        let mut h = make_with(m, Config { auto_restore_english: false, ..cfg() });
        h.type_str("text law windows boss ");
        assert_eq!(h.text(), "text law windows bos ");
    }
}

#[test]
fn escape_restores_raw() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["vieejt", "Escape", " "]);
        assert_eq!(h.text(), "vieejt ");
    }
}

#[test]
fn enter_and_punctuation() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["xin chaof", "Enter", "ddi ddaau?"]);
        assert_eq!(h.text(), "xin chào\nđi đâu?");
    }
}

#[test]
fn auto_capitalize() {
    for m in MODES {
        let mut h = make_with(m, Config { auto_capitalize: true, ..cfg() });
        type_mixed(&mut h, &["xin chaof. ddi ddaau? vaang", "Enter", "oong ok"]);
        assert_eq!(h.text(), "xin chào. Đi đâu? Vâng\nÔng ok", "{m:?}");
    }
}

#[test]
fn navigation_breaks_word() {
    for m in MODES {
        let mut h = make_with(m, Config { edit_previous_word: false, ..cfg() });
        type_mixed(&mut h, &["tieeng", "Left", "s"]);
        assert_eq!(h.text(), "tiêngs");
    }
}

// ---------------------------------------------------------------- ngữ cảnh

#[test]
fn replace_mode_plain_letters_pass_through() {
    let mut h = make(OutputMode::Replace);
    assert_eq!(h.session.process_key(&KeyEvent::ch('v')), Action::PassThrough);
    assert_eq!(h.session.process_key(&KeyEvent::ch('i')), Action::PassThrough);
}

#[test]
fn preedit_commit_on_focus_change() {
    let mut h = make(OutputMode::Preedit);
    h.type_str("vieejt");
    assert_eq!(h.session.focus_changed(Some("chrome")), Action::Commit { text: "việt".into(), pass_key: false });
}

#[test]
fn excluded_app() {
    let mut h = Harness::new(OutputMode::Replace, Config { excluded_apps: vec!["vim".into()], ..cfg() }, "vim", "");
    h.type_str("vieejt");
    assert_eq!(h.text(), "vieejt");
}

#[test]
fn toggle_remembered_per_app() {
    let mut h = make(OutputMode::Replace);
    h.session.toggle_vietnamese();
    assert_eq!(h.store.load().app_modes.get("notepad"), Some(&false));
    h.session.focus_changed(Some("chrome"));
    assert!(h.session.is_enabled());
    h.session.focus_changed(Some("notepad"));
    assert!(!h.session.is_enabled());
}

#[test]
fn edit_previous_word() {
    let mut h = Harness::new(OutputMode::Replace, cfg(), "notepad", "tôi học tiêng");
    h.type_str("s");
    assert_eq!(h.text(), "tôi học tiếng");
}

#[test]
fn edit_previous_word_invalid_key_keeps_word() {
    let mut h = Harness::new(OutputMode::Replace, cfg(), "notepad", "Việt");
    h.type_str("f");
    assert_eq!(h.text(), "Việtf");
}

#[test]
fn edit_previous_word_ignores_english() {
    let mut h = Harness::new(OutputMode::Replace, cfg(), "notepad", "hello");
    h.type_str("s");
    assert_eq!(h.text(), "hellos");
}

#[test]
fn edit_previous_word_skips_when_normal_form_differs() {
    let mut h = Harness::new(OutputMode::Replace, Config { unicode_form: UnicodeForm::Nfd, ..cfg() }, "notepad", "Việt");
    h.type_str("s");
    assert_eq!(h.text(), "Việts");
}

#[test]
fn edit_previous_word_nfd_screen() {
    let nfd: String = "tiêng".nfd().collect();
    let mut h = Harness::new(OutputMode::Replace, Config { unicode_form: UnicodeForm::Nfd, ..cfg() }, "notepad", &nfd);
    h.type_str("s");
    assert_eq!(h.text(), "tiếng".nfd().collect::<String>());
}

#[test]
fn nfd_output() {
    for m in MODES {
        let mut h = make_with(m, Config { unicode_form: UnicodeForm::Nfd, ..cfg() });
        h.type_str("Vieejt Nam ");
        assert_eq!(h.text(), "Việt Nam ".nfd().collect::<String>());
    }
}

// ---------------------------------------------------------------- edge case từ các dự án khác

#[test]
fn soft_reset_backspace_after_space_then_retone() {
    let mut h = make_with(OutputMode::Replace, Config { edit_previous_word: false, ..cfg() });
    type_mixed(&mut h, &["Chaof ", BS, "f"]);
    assert_eq!(h.text(), "Chaof");
}

#[test]
fn soft_reset_add_tone_after_space() {
    let mut h = make_with(OutputMode::Replace, Config { edit_previous_word: false, ..cfg() });
    type_mixed(&mut h, &["tieeng ", BS, "s "]);
    assert_eq!(h.text(), "tiếng ");
}

#[test]
fn soft_reset_not_after_macro() {
    let mut h = make_with(OutputMode::Replace, Config { edit_previous_word: false, ..cfg() });
    type_mixed(&mut h, &["ko ", BS, "s"]);
    assert_eq!(h.text(), "khôngs");
}

#[test]
fn modifier_alone_does_not_break_word() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["tieeng", "Shift", "Option", "s "]);
        assert_eq!(h.text(), "tiếng ");
    }
}

#[test]
fn password_field_disabled() {
    for m in MODES {
        let mut h = make(m);
        h.session.set_field_kind(FieldKind::Password);
        h.type_str("aaas");
        assert_eq!(h.text(), "aaas");
    }
}

#[test]
fn cursor_moved_commits_preedit() {
    let mut h = make(OutputMode::Preedit);
    h.type_str("vieejt");
    let a = h.session.cursor_moved();
    h.field.apply(&a, None);
    {
        let f = h.field.0.borrow();
        assert_eq!(f.committed, "việt");
        assert_eq!(f.preedit, "");
    }
    h.type_str("s");
    assert_eq!(h.text(), "việts");
}

#[test]
fn smart_caps() {
    for m in MODES {
        let mut h = make_with(m, Config { smart_caps: true, ..cfg() });
        h.type_str("HTTPS JSON DDI Vieejt ");
        assert_eq!(h.text(), "HTTPS JSON ĐI Việt ");
    }
}

#[test]
fn uppercase_ww() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("WW ");
        assert_eq!(h.text(), "W ");
    }
}

#[test]
fn app_profiles() {
    let c = Config::default();
    assert_eq!(resolve_profile(&c, Some("/usr/bin/kitty")).output_mode, Some(OutputMode::Preedit));
    assert!(resolve_profile(&c, Some("C:\\Program Files\\Google\\Chrome.exe")).wait_ack);
    assert_eq!(resolve_profile(&c, Some("Firefox")).edit_previous_word, Some(false));
    let mut h = Harness::new(OutputMode::Replace, cfg(), "minecraft", "");
    h.type_str("vieejt");
    assert_eq!(h.text(), "vieejt");
}

#[test]
fn firefox_profile_disables_edit_previous_word() {
    let mut h = Harness::new(OutputMode::Replace, cfg(), "firefox", "tiêng");
    h.type_str("s");
    assert_eq!(h.text(), "tiêngs");
}

#[test]
fn config_json_roundtrip() {
    let c = Config { tone_style: ToneStyle::New, macros: macros(), ..Config::default() };
    let s = serde_json::to_string(&c).unwrap();
    let back: Config = serde_json::from_str(&s).unwrap();
    assert_eq!(back, c);
    // JSON thiếu trường -> dùng mặc định
    let partial: Config = serde_json::from_str(r#"{"input_method":"vni","tone_style":"new"}"#).unwrap();
    assert_eq!(partial.input_method, "vni");
    assert!(partial.spell_check);
}

// ---------------------------------------------------------------- property

struct Lcg(u64);
impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) % n as u64) as usize
    }
}

#[test]
fn type_then_backspace_all() {
    let alpha: Vec<char> = "aeiouydwsfrxjngthc".chars().collect();
    let mut rng = Lcg(3);
    for _ in 0..800 {
        let len = 1 + rng.below(8);
        let word: String = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
        for m in MODES {
            let mut h = make_with(m, Config { edit_previous_word: false, ..cfg() });
            h.type_str(&word);
            let o = vfk_core::TransformOptions::default();
            assert_eq!(h.text(), vfk_core::transform_str(&word, &vfk_core::TELEX, &o, false).text);
            let n = h.session.display().chars().count();
            for _ in 0..n {
                h.field.key(&mut h.session, BS);
            }
            assert_eq!(h.text(), "", "{m:?} {word}");
        }
    }
}

#[test]
fn modes_agree_on_random_typing() {
    let ks: Vec<&str> = "aeiouydwsfrxjzngthcAWS ,."
        .split("")
        .filter(|s| !s.is_empty())
        .chain([BS, BS, "Escape", "Enter", "Left"])
        .collect();
    let mut rng = Lcg(7);
    for _ in 0..1500 {
        let len = 1 + rng.below(25);
        let seq: Vec<&str> = (0..len).map(|_| ks[rng.below(ks.len())]).collect();
        let mut texts = Vec::new();
        for m in MODES {
            let c = Config { edit_previous_word: false, restore_after_space: false, ..cfg() };
            let mut h = make_with(m, c);
            for k in &seq {
                h.field.key(&mut h.session, k);
                let f = h.field.0.borrow();
                let shown = if m == OutputMode::Preedit { &f.preedit } else { &f.committed };
                assert!(shown.ends_with(&h.session.display()), "{m:?} {seq:?}");
            }
            h.field.key(&mut h.session, "Enter");
            texts.push(h.text());
        }
        assert_eq!(texts[0], texts[1], "{seq:?}");
    }
}
