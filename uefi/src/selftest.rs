//! Kiểm tra headless cho editor/engine. Chỉ compile với feature `selftest`;
//! kết quả ghi ra debug console của QEMU (port 0xE9).

use alloc::format;
use alloc::string::String;

use crate::debugcon;
use crate::editor::Editor;

fn current_line(editor: &Editor) -> String {
    let (line, _) = editor.cursor();
    let mut out: String = editor.lines()[line].iter().collect();
    out.push_str(editor.active_text());
    out
}

fn type_text(editor: &mut Editor, text: &str) {
    for ch in text.chars() {
        editor.on_char(ch);
    }
}

fn check(name: &str, actual: &str, expected: &str, failures: &mut usize) {
    if actual == expected {
        debugcon::line(&format!("PASS {name}"));
    } else {
        *failures += 1;
        debugcon::line(&format!(
            "FAIL {name}: got {actual:?}, expected {expected:?}"
        ));
    }
}

pub fn run() -> bool {
    let mut failures = 0usize;

    // Telex composition and commit.
    let mut ed = Editor::new(80, 20);
    type_text(&mut ed, "tieengs");
    check("telex-active", &current_line(&ed), "tiếng", &mut failures);
    ed.on_char(' ');
    check("telex-commit", &current_line(&ed), "tiếng ", &mut failures);
    check("telex-raw-empty", ed.raw(), "", &mut failures);

    // Capitalized multi-word input.
    let mut ed = Editor::new(80, 20);
    type_text(&mut ed, "Tieengs Vieejt");
    ed.on_char(' ');
    check(
        "telex-words",
        &current_line(&ed),
        "Tiếng Việt ",
        &mut failures,
    );

    // English escape: spelling stays literal after commit.
    let mut ed = Editor::new(80, 20);
    type_text(&mut ed, "text");
    check("english-active", &current_line(&ed), "text", &mut failures);
    ed.on_char(' ');
    check("english-commit", &current_line(&ed), "text ", &mut failures);

    // Common Telex cases.
    let mut ed = Editor::new(80, 20);
    type_text(&mut ed, "thuowng");
    check("telex-uow", &current_line(&ed), "thương", &mut failures);

    let mut ed = Editor::new(80, 20);
    type_text(&mut ed, "dd");
    check("telex-d", &current_line(&ed), "đ", &mut failures);

    // Backspace removes the last raw key, not the last display scalar.
    let mut ed = Editor::new(80, 20);
    type_text(&mut ed, "tieengs");
    ed.on_backspace();
    check("backspace-1", &current_line(&ed), "tiêng", &mut failures);
    ed.on_backspace();
    check("backspace-2", &current_line(&ed), "tiên", &mut failures);

    // VNI.
    let mut ed = Editor::new(80, 20);
    ed.on_toggle_method();
    type_text(&mut ed, "Vie65t");
    check("vni-active", &current_line(&ed), "Việt", &mut failures);
    ed.on_char(' ');
    check("vni-commit", &current_line(&ed), "Việt ", &mut failures);

    // English mode inserts literal text.
    let mut ed = Editor::new(80, 20);
    ed.on_toggle_language();
    type_text(&mut ed, "abc");
    check("english-mode", &current_line(&ed), "abc", &mut failures);

    // Enter creates a new line.
    let mut ed = Editor::new(80, 20);
    type_text(&mut ed, "abc");
    ed.on_enter();
    type_text(&mut ed, "xyz");
    check("line-0", &current_line(&ed), "xyz", &mut failures);
    check(
        "line-count",
        &format!("{}", ed.lines().len()),
        "2",
        &mut failures,
    );

    failures == 0
}
