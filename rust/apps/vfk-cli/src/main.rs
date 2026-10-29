//! vfk: REPL gõ thử ViFastKey. Port của `apps/vfk_cli.py`.
//!
//! cargo run -p vfk-cli -- --demo
//! cargo run -p vfk-cli
//! Lệnh: :telex :vni :simple_telex :old :new :replace :preedit :toggle :trace :q
//! Trong dòng gõ: '<' = Backspace, '~' = Escape.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use vfk_engine::testing::Harness;
use vfk_engine::*;
use vi_syllable::ToneStyle;

struct Repl {
    config: Config,
    mode: OutputMode,
    trace: bool,
    h: Harness,
}

impl Repl {
    fn new() -> Self {
        let macros = HashMap::from([
            ("ko".to_string(), "không".to_string()),
            ("vn".to_string(), "Việt Nam".to_string()),
            ("dc".to_string(), "được".to_string()),
        ]);
        let config = Config { macros, ..Config::default() };
        let mode = OutputMode::Replace;
        let h = Harness::new(mode, config.clone(), "cli", "");
        Repl { config, mode, trace: false, h }
    }

    fn rebuild(&mut self) {
        self.h = Harness::new(self.mode, self.config.clone(), "cli", "");
    }

    fn status(&self) {
        let ts = if self.config.tone_style == ToneStyle::New { "new" } else { "old" };
        let md = if self.mode == OutputMode::Preedit { "preedit" } else { "replace" };
        println!("  [{} | {ts} | {md}]", self.config.input_method);
    }

    /// Trả false khi thoát.
    fn command(&mut self, cmd: &str) -> bool {
        match cmd {
            ":q" => return false,
            ":telex" | ":vni" | ":simple_telex" => {
                self.config.input_method = cmd[1..].to_string();
                self.rebuild();
            }
            ":old" => {
                self.config.tone_style = ToneStyle::Old;
                self.rebuild();
            }
            ":new" => {
                self.config.tone_style = ToneStyle::New;
                self.rebuild();
            }
            ":replace" => {
                self.mode = OutputMode::Replace;
                self.rebuild();
            }
            ":preedit" => {
                self.mode = OutputMode::Preedit;
                self.rebuild();
            }
            ":toggle" => {
                self.h.session.toggle_vietnamese();
                println!("  Tiếng Việt: {}", if self.h.session.is_enabled() { "BẬT" } else { "TẮT" });
            }
            ":trace" => self.trace = !self.trace,
            _ => println!("  Lệnh không rõ"),
        }
        self.status();
        true
    }

    fn type_line(&mut self, line: &str) -> String {
        {
            let mut f = self.h.field.0.borrow_mut();
            f.committed.clear();
            f.preedit.clear();
        }
        self.h.session.reset();
        for ch in line.chars() {
            let ev = match ch {
                '<' => KeyEvent::new(Key::Backspace),
                '~' => KeyEvent::new(Key::Escape),
                c => KeyEvent::ch(c),
            };
            let act = self.h.session.process_key(&ev);
            self.h.field.apply(&act, Some(&ev));
            if self.trace {
                println!("  {:<12} -> {:?}  | {:?}", format!("{:?}", ev.key), act, self.h.text());
            }
        }
        // chốt từ cuối (không gửi Enter xuống ô nhập)
        let act = self.h.session.process_key(&KeyEvent::new(Key::Enter));
        self.h.field.apply(&act, None);
        self.h.text()
    }
}

const DEMO: &[&str] = &[
    "Tieengs Vieejt laf ngoon ngwx raats hay.",
    "Nguwowif Vieejt Nam ddi hojc owr truwowngf ddaij hojc",
    "hoaf thuys khoer quyeets giuwx",
    "text case windows mix six",
    "ko dc vn",
    "tieengs<g",
    "server boss data this did",
    "Thuowr nhor, xooongj, quyts, khuyur tay",
    "thuowng thuowwng huowu ruowuj",
];

fn main() {
    let demo = std::env::args().any(|a| a == "--demo");
    let mut r = Repl::new();
    if demo {
        for line in DEMO {
            println!("{:<60} -> {:?}", format!("{line:?}"), r.type_line(line));
        }
        r.command(":vni");
        let l = "Vie65t Nam, nguo7i2 d9i";
        println!("{:<60} -> {:?}", format!("{l:?}"), r.type_line(l));
        return;
    }
    println!("ViFastKey REPL. Lệnh: :telex :vni :simple_telex :old :new :replace :preedit :toggle :trace :q");
    println!("Trong dòng gõ: '<' = Backspace, '~' = Escape.");
    let stdin = io::stdin();
    loop {
        print!("gõ> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end_matches(&['\n', '\r'][..]);
        if line.starts_with(':') {
            if !r.command(line.trim()) {
                break;
            }
            continue;
        }
        println!("  => {}", r.type_line(line));
    }
}
