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

fn make_app(mode: OutputMode, app: &str) -> Harness {
    Harness::new(mode, cfg(), app, "")
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
fn default_prefers_valid_vietnamese() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("mix max six tax too this ");
        assert_eq!(h.text(), "mĩ mã sĩ tã tô thí ", "{m:?}");
    }
}

#[test]
fn english_words_use_telex_escape_by_default() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("mixx thiss seee dataa lasst ");
        assert_eq!(h.text(), "mix this see data last ", "{m:?}");
    }
}

#[test]
fn double_tone_key_uses_standard_escape() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("server boss pass miss ass ");
        assert_eq!(h.text(), "server bos pas mis as ");
        let mut h = make(m);
        h.type_str("bosss passs misss asss ");
        assert_eq!(h.text(), "boss pass miss ass ");
    }
}

#[test]
fn uppercase_double_f_escape_brand() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("ViFFastTts ");
        assert_eq!(h.text(), "ViFastTts ");
    }
}

#[test]
fn lowercase_double_f_uses_standard_escape() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("coffee ");
        assert_eq!(h.text(), "cofee ");
        let mut h = make(m);
        h.type_str("cofffee ");
        assert_eq!(h.text(), "coffee ");
    }
}

#[test]
fn double_s_escape_english_word() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("vissa ");
        assert_eq!(h.text(), "visa ");
    }
}

#[test]
fn double_r_escape_english_word() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("serrver ");
        assert_eq!(h.text(), "server ");
    }
}

#[test]
fn escape_follows_second_key_case() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("ViFfa ");
        assert_eq!(h.text(), "Vifa ");
    }
}

#[test]
fn escape_f_accepts_lower_then_upper() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("VifFa ");
        assert_eq!(h.text(), "ViFa ");
    }
}

#[test]
fn backspace_after_escape_restores_tone() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["phar", "r", "r", BS, BS, "r", " "]);
        assert_eq!(h.text(), "phả ");
    }
}

#[test]
fn backspace_after_duplicate_letters_restores_word() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("nhuwnggg");
        assert_eq!(h.text(), "nhuwnggg");
        type_mixed(&mut h, &[BS]);
        assert_eq!(h.text(), "nhuwngg");
        type_mixed(&mut h, &[BS]);
        assert_eq!(h.text(), "nhưng");
    }
}

#[test]
fn backspace_after_space_adopted_word_does_not_duplicate() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["ViFastKey", " ", BS, BS, "y", BS]);
        assert_eq!(h.text(), "ViFastKe", "{m:?}");
        h.type_str("y ");
        assert_eq!(h.text(), "ViFastKey ", "{m:?}");
    }
}

#[test]
fn backspace_after_invalid_suffix_restores_word() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("nhuwnggg");
        type_mixed(&mut h, &[BS, BS]);
        assert_eq!(h.text(), "nhưng");
        // Gõ thêm ký tự làm từ sai chính tả: hiện raw, nhưng xoá đi phải quay lại tiếng Việt.
        h.type_str("l");
        assert_eq!(h.text(), "nhuwngl");
        type_mixed(&mut h, &[BS]);
        assert_eq!(h.text(), "nhưng");

        // Từ Việt đã đúng + ký tự thừa: xoá ký tự thừa phải quay lại từ Việt.
        h.type_str(" too");
        assert_eq!(h.text(), "nhưng tô");
        h.type_str("l");
        assert_eq!(h.text(), "nhưng tool");
        type_mixed(&mut h, &[BS]);
        assert_eq!(h.text(), "nhưng tô");
    }
}

#[test]
fn backspace_restores_valid_syllable_not_in_dictionary() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("thoangr");
        assert_eq!(h.text(), "thoảng", "{m:?}");
        h.type_str("e");
        assert_eq!(h.text(), "thoangre", "{m:?}");
        type_mixed(&mut h, &[BS]);
        assert_eq!(h.text(), "thoảng", "{m:?}");
    }
}

#[test]
fn spell_restore_for_invalid_english() {
    for m in MODES {
        let mut h = make(m); // không có dictionary auto-restore
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
    assert_eq!(
        h.session.focus_changed(Some("chrome")),
        Action::Commit { text: "việt".into(), pass_key: false }
    );
}

#[test]
fn excluded_app() {
    let mut h =
        Harness::new(OutputMode::Replace, Config { excluded_apps: vec!["vim".into()], ..cfg() }, "vim", "");
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
    let mut h = Harness::new(
        OutputMode::Replace,
        Config { unicode_form: UnicodeForm::Nfd, ..cfg() },
        "notepad",
        "Việt",
    );
    h.type_str("s");
    assert_eq!(h.text(), "Việts");
}

#[test]
fn edit_previous_word_nfd_screen() {
    let nfd: String = "tiêng".nfd().collect();
    let mut h = Harness::new(
        OutputMode::Replace,
        Config { unicode_form: UnicodeForm::Nfd, ..cfg() },
        "notepad",
        &nfd,
    );
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
fn soft_reset_disabled_in_linux_terminal_profile() {
    for m in MODES {
        let mut h = make_app(m, "xterm");
        type_mixed(&mut h, &["hoa ", BS, "f"]);
        assert_eq!(h.text(), "hoaf", "{m:?}");
    }
}

#[test]
fn backspace_right_after_space_restores_previous_word() {
    for m in MODES {
        let mut h = make(m);
        type_mixed(&mut h, &["nawm ", BS]);
        assert_eq!(h.text(), "năm", "{m:?}");
        type_mixed(&mut h, &["s"]);
        assert_eq!(h.text(), "nắm", "{m:?}");
    }
}

#[test]
fn backspace_through_garbage_restores_previous_word() {
    for m in MODES {
        let mut h = make(m);
        // 3 Backspace xoá hết "asdf" (a+s hiển thị "á"), Backspace thứ 4 xoá
        // dấu cách và khôi phục "năm".
        type_mixed(&mut h, &["nawm ", "asdf", BS, BS, BS]);
        assert_eq!(h.text(), "năm ", "{m:?}");
        type_mixed(&mut h, &[BS]);
        assert_eq!(h.text(), "năm", "{m:?}");
        type_mixed(&mut h, &["s"]);
        assert_eq!(h.text(), "nắm", "{m:?}");
    }
}

#[test]
fn soft_reset_not_after_macro() {
    let mut h =
        make_with(OutputMode::Replace, Config { edit_previous_word: false, macro_undo: false, ..cfg() });
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
fn smart_caps_vietnamese_shape_pairs() {
    for m in MODES {
        let mut h = make_with(m, Config { smart_caps: true, ..cfg() });
        h.type_str("AW AWSC AA OO OW UW JSON API ");
        assert_eq!(h.text(), "Ă ẮC Â Ô Ơ Ư JSON API ", "{m:?}");
    }
}

#[test]
fn url_like_text_keeps_raw() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("vietnam.vn foo.bar nguyen.van@example.com src/main.rs ");
        assert_eq!(h.text(), "vietnam.vn foo.bar nguyen.van@example.com src/main.rs ", "{m:?}");
    }
}

#[test]
fn url_field_allows_search_and_is_url_safe() {
    for m in MODES {
        let mut h = make_with(m, Config { smart_caps: true, ..cfg() });
        h.session.set_field_kind(FieldKind::Url);
        h.type_str("w3schools.com vietnam.vn tieengs ");
        assert_eq!(h.text(), "w3schools.com vietnam.vn tiếng ", "{m:?}");
    }
}

#[test]
fn uppercase_ww() {
    for m in MODES {
        let mut h = make_with(m, Config { w_as_u: true, ..cfg() });
        h.type_str("WW ");
        assert_eq!(h.text(), "W ");
    }
}

#[test]
fn config_app_profiles_keep_presets() {
    let config: Config = serde_json::from_str(
        r#"{"app_profiles":{"kitty":{"output_mode":"replace","backspace_delay_ms":9}}}"#,
    )
    .unwrap();
    assert!(config.app_profiles.contains_key("chrome"), "preset chrome phải còn");
    assert!(config.app_profiles.contains_key("firefox"), "preset firefox phải còn");
    let kitty = config.app_profiles.get("kitty").unwrap();
    assert_eq!(kitty.output_mode, Some(OutputMode::Replace));
    assert_eq!(kitty.backspace_delay_ms, 9);
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

#[test]
fn macro_undo_restores_trigger_and_separator() {
    let mut h = make_with(
        OutputMode::Replace,
        Config { macros: HashMap::from([("ko".into(), "không".into())]), macro_undo: true, ..cfg() },
    );
    h.type_str("ko ");
    assert_eq!(h.text(), "không ");
    h.field.key(&mut h.session, BS);
    assert_eq!(h.text(), "ko ");
    h.field.key(&mut h.session, BS);
    assert_eq!(h.text(), "ko");
}

#[test]
fn auto_capitalize_handles_ellipsis_quotes_and_newline() {
    let mut h = make_with(OutputMode::Replace, Config { auto_capitalize: true, ..cfg() });
    h.type_str("xin chaof… ddi. ”ddaay? ");
    assert_eq!(h.text(), "xin chào… Đi. ”Đây? ");
    h.type_str("\noong");
    assert!(h.text().ends_with("\nÔng"));
}

#[test]
fn viqr_engine_word_boundary() {
    let mut h = make_with(OutputMode::Replace, Config { input_method: "viqr".into(), ..cfg() });
    h.type_str("tie^ng' ");
    assert_eq!(h.text(), "tiếng ");
}

#[test]
fn preedit_edits_previous_word() {
    for mode in MODES {
        let mut h = Harness::new(mode, cfg(), "notepad", "tiêng");
        h.type_str("s");
        assert_eq!(h.text(), "tiếng", "{mode:?}");
        h.type_str(" ");
        assert_eq!(h.text(), "tiếng ", "{mode:?}");
    }
}

#[test]
fn phrase_macro_across_spaces() {
    let mut h = make_with(
        OutputMode::Replace,
        Config {
            macros: HashMap::from([("k biet".into(), "không biết".into()), ("biet".into(), "biết".into())]),
            edit_previous_word: false,
            ..cfg()
        },
    );
    h.type_str("toi k biet ");
    assert_eq!(h.text(), "toi không biết ");
}

#[test]
fn macro_in_off_mode() {
    let mut h = make_with(
        OutputMode::Replace,
        Config {
            macros: HashMap::from([("ko".into(), "không".into())]),
            macro_in_off_mode: true,
            default_vietnamese: false,
            edit_previous_word: false,
            ..cfg()
        },
    );
    h.type_str("ko ");
    assert_eq!(h.text(), "không ");
}

#[test]
fn user_dictionary_api() {
    let mut d = vfk_engine::testing::WordListDictionary::from_text("", "");
    d.add_words(["Lước", "LƯỚT"], true);
    d.add_words(["Customword"], false);
    assert!(d.is_vietnamese("lước") && d.is_vietnamese("lướt"));
    assert!(d.is_english("customword"));
}

#[test]
fn app_id_normalization_and_aliases() {
    assert_eq!(normalize_app_id(Some("/usr/bin/kitty")), "kitty");
    assert_eq!(normalize_app_id(Some("C:\\Program Files\\Google\\Chrome.exe")), "chrome");
    assert_eq!(normalize_app_id(Some("com.google.Chrome")), "chrome");
    assert_eq!(normalize_app_id(Some("com.microsoft.VSCode")), "code");
    let c = Config::default();
    assert!(resolve_profile(&c, Some("com.google.Chrome")).wait_ack);
    assert_eq!(resolve_profile(&c, Some("com.microsoft.VSCode")).backspace_delay_ms, 3);
}

#[test]
fn app_profile_overrides_method_tone_and_spell() {
    let mut profiles = default_app_profiles();
    profiles.insert(
        "notepad".into(),
        AppProfile {
            input_method: Some("vni".into()),
            tone_style: Some(ToneStyle::New),
            spell_check: Some(false),
            ..Default::default()
        },
    );
    let mut h = Harness::new(OutputMode::Replace, Config { app_profiles: profiles, ..cfg() }, "notepad", "");
    h.type_str("hoa2 ");
    assert_eq!(h.text(), "hoà ");
    h.session.focus_changed(Some("other"));
    h.type_str("tieengs ");
    assert_eq!(h.text(), "hoà tiếng ");
}

#[test]
fn config_json_patch_merges_and_rejects_bad_values() {
    let c = Config {
        tone_style: ToneStyle::Old,
        macros: HashMap::from([("ko".into(), "không".into())]),
        ..cfg()
    };
    let p = c.apply_json_patch(r#"{"tone_style":"new"}"#).expect("patch hợp lệ");
    assert_eq!(p.tone_style, ToneStyle::New);
    assert_eq!(p.macros.get("ko").map(String::as_str), Some("không"));
    assert!(c.apply_json_patch(r#"{"tone_style":"không có"}"#).is_none());
}

// ---------------------------------------------------------------- tuỳ chọn mới

#[test]
fn proper_name_coda_k() {
    for m in MODES {
        let mut h = make_with(m, Config { allow_coda_k: true, ..cfg() });
        h.type_str("Ddawsk Lawsk ");
        assert_eq!(h.text(), "Đắk Lắk ", "{m:?}");
    }
}

#[test]
fn proper_name_coda_k_off_restores_raw() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("Ddawsk Lawsk ");
        assert_eq!(h.text(), "Ddawsk Lawsk ", "{m:?}");
    }
}

#[test]
fn macro_digits() {
    let mut m = macros();
    m.insert("mp3".into(), "máy nghe nhạc".into());
    m.insert("k8s".into(), "kubernetes".into());
    for mode in MODES {
        let mut h = make_with(mode, Config { macro_digits: true, macros: m.clone(), ..cfg() });
        h.type_str("mp3 k8s MP3 123 ");
        assert_eq!(h.text(), "máy nghe nhạc kubernetes MÁY NGHE NHẠC 123 ", "{mode:?}");
    }
}

#[test]
fn macro_digits_off_does_not_expand() {
    let mut m = macros();
    m.insert("mp3".into(), "máy nghe nhạc".into());
    for mode in MODES {
        let mut h = make_with(mode, Config { macros: m.clone(), ..cfg() });
        h.type_str("mp3 ");
        assert_eq!(h.text(), "mp3 ", "{mode:?}");
    }
}

#[test]
fn macro_digits_do_not_start_a_word() {
    let mut m = macros();
    m.insert("123".into(), "một hai ba".into());
    for mode in MODES {
        let mut h = make_with(mode, Config { macro_digits: true, macros: m.clone(), ..cfg() });
        h.type_str("123 ");
        assert_eq!(h.text(), "123 ", "{mode:?}");
    }
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
fn default_w_is_literal() {
    for m in MODES {
        let mut h = make(m);
        h.type_str("web ww ");
        assert_eq!(h.text(), "web ww ");
    }
}

#[test]
fn w_as_u_escape_continues_as_literal() {
    for m in MODES {
        let mut h = make_with(m, Config { w_as_u: true, ..cfg() });
        h.type_str("ww wwe wweb ");
        assert_eq!(h.text(), "w we web ");
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
            let o = vfk_core::TransformOptions {
                w_as_u: h.session.config().w_as_u,
                ..vfk_core::TransformOptions::default()
            };
            let expected = vfk_core::transform_str(&word, &vfk_core::TELEX, &o, false).text;
            assert_eq!(h.text(), if h.session.is_literal() { word.clone() } else { expected });
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
