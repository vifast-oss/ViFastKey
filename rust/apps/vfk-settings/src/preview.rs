//! Gõ thử bằng engine thật, dùng cú pháp raw key giống CLI:
//! `<` Backspace, `~` Escape, `|` Enter.

use vfk_engine::testing::Harness;
use vfk_engine::{Action, Config, KeyEvent, OutputMode};

#[derive(Clone, Debug, Default)]
pub struct PreviewResult {
    pub output: String,
    pub actions: Vec<String>,
    pub error: Option<String>,
}

pub fn run(config: Config, mode: OutputMode, app: &str, input: &str) -> PreviewResult {
    let mut harness = Harness::new(mode, config, app, "");
    let mut actions = Vec::new();
    let mut buffer = String::new();

    for ch in input.chars() {
        match ch {
            '<' => {
                flush(&mut harness, &mut buffer, &mut actions);
                press(&mut harness, "Backspace", &mut actions);
            }
            '~' => {
                flush(&mut harness, &mut buffer, &mut actions);
                press(&mut harness, "Escape", &mut actions);
            }
            '|' => {
                flush(&mut harness, &mut buffer, &mut actions);
                press(&mut harness, "Enter", &mut actions);
            }
            _ => buffer.push(ch),
        }
    }
    flush(&mut harness, &mut buffer, &mut actions);

    PreviewResult { output: harness.text(), actions, error: None }
}

fn flush(harness: &mut Harness, buffer: &mut String, actions: &mut Vec<String>) {
    for ch in buffer.chars() {
        let ev = KeyEvent::ch(ch);
        let action = harness.session.process_key(&ev);
        harness.field.apply(&action, Some(&ev));
        push_action(actions, ch, &action);
    }
    buffer.clear();
}

fn press(harness: &mut Harness, name: &str, actions: &mut Vec<String>) {
    let ev = KeyEvent::parse(name);
    let action = harness.session.process_key(&ev);
    harness.field.apply(&action, Some(&ev));
    push_action(actions, name, &action);
}

fn push_action(actions: &mut Vec<String>, key: impl std::fmt::Display, action: &Action) {
    if actions.len() < 40 {
        actions.push(format!("{key} -> {}", action_name(action)));
    }
}

fn action_name(action: &Action) -> String {
    match action {
        Action::PassThrough => "pass".to_string(),
        Action::Preedit { text, .. } => format!("preedit {text:?}"),
        Action::Commit { text, pass_key } => {
            format!("commit {text:?}{}", if *pass_key { " + key" } else { "" })
        }
        Action::Replace { delete_before, insert, pass_key } => {
            format!("replace -{delete_before} +{insert:?}{}", if *pass_key { " + key" } else { "" })
        }
    }
}
