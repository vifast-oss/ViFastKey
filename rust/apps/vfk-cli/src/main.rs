//! vfk: REPL và công cụ gõ thử ViFastKey. Port của `apps/vfk_cli.py`.
//!
//! cargo run -p vfk-cli -- --demo
//! cargo run -p vfk-cli
//! cargo run -p vfk-cli -- --method vni --tone new --text "Vie65t Nam"
//! Lệnh: :telex :vni :simple_telex :old :new :replace :preedit :toggle :trace
//!       :coda_k :macro_digits :spell :help :q
//! Trong dòng gõ: '<' = Backspace, '~' = Escape, '|' = Enter.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use vfk_engine::testing::Harness;
use vfk_engine::*;
use vi_syllable::ToneStyle;

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

const HELP: &str = r#"ViFastKey CLI — gõ thử chuỗi phím thô.

  vfk                          REPL tương tác
  vfk --demo                   chạy bộ ví dụ
  vfk --text "Tieengs Vieejt"  gõ một dòng rồi in kết quả (lặp lại được)
  --method telex|simple_telex|vni
  --tone old|new
  --mode replace|preedit
  --nfd --no-spell --allow-coda-k --macro-digits --trace
  --macro KHOÁ=GIÁ TRỊ

Trong REPL: :telex :vni :simple_telex :old :new :replace :preedit :toggle
            :trace :coda_k :macro_digits :spell :help :q
Trong dòng gõ: '<' = Backspace, '~' = Escape, '|' = Enter."#;

struct Repl {
    config: Config,
    mode: OutputMode,
    trace: bool,
    h: Harness,
}

impl Repl {
    fn new(config: Config, mode: OutputMode, trace: bool) -> Self {
        let h = Harness::new(mode, config.clone(), "cli", "");
        Repl { config, mode, trace, h }
    }

    fn rebuild(&mut self) {
        self.h = Harness::new(self.mode, self.config.clone(), "cli", "");
    }

    fn status(&self) {
        let ts = if self.config.tone_style == ToneStyle::New { "new" } else { "old" };
        let md = if self.mode == OutputMode::Preedit { "preedit" } else { "replace" };
        let mut flags = Vec::new();
        if self.config.allow_coda_k {
            flags.push("coda_k");
        }
        if self.config.macro_digits {
            flags.push("macro_digits");
        }
        if !self.config.spell_check {
            flags.push("no-spell");
        }
        let suffix = if flags.is_empty() { String::new() } else { format!(" | {}", flags.join(",")) };
        println!("  [{} | {ts} | {md}{suffix}]", self.config.input_method);
    }

    /// Trả false khi thoát.
    fn command(&mut self, cmd: &str) -> bool {
        match cmd {
            ":q" | ":quit" | ":exit" => return false,
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
            ":coda_k" => {
                self.config.allow_coda_k = !self.config.allow_coda_k;
                self.rebuild();
            }
            ":macro_digits" => {
                self.config.macro_digits = !self.config.macro_digits;
                self.rebuild();
            }
            ":spell" => {
                self.config.spell_check = !self.config.spell_check;
                self.rebuild();
            }
            ":help" => println!("{HELP}"),
            _ => println!("  Lệnh không rõ (thử :help)"),
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
                '|' => KeyEvent::new(Key::Enter),
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

struct Args {
    demo: bool,
    texts: Vec<String>,
    method: Option<String>,
    tone: Option<ToneStyle>,
    mode: OutputMode,
    nfd: bool,
    no_spell: bool,
    allow_coda_k: bool,
    macro_digits: bool,
    macros: Vec<(String, String)>,
    trace: bool,
}

fn next_value(it: &mut impl Iterator<Item = String>, flag: &str) -> String {
    match it.next() {
        Some(v) => v,
        None => {
            eprintln!("Thiếu giá trị cho {flag}");
            std::process::exit(2);
        }
    }
}

fn parse_args() -> Args {
    let mut args = Args {
        demo: false,
        texts: Vec::new(),
        method: None,
        tone: None,
        mode: OutputMode::Replace,
        nfd: false,
        no_spell: false,
        allow_coda_k: false,
        macro_digits: false,
        macros: Vec::new(),
        trace: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--demo" => args.demo = true,
            "--text" => args.texts.push(next_value(&mut it, "--text")),
            "--method" => args.method = Some(next_value(&mut it, "--method")),
            "--tone" => {
                let v = next_value(&mut it, "--tone");
                args.tone = match v.as_str() {
                    "old" => Some(ToneStyle::Old),
                    "new" => Some(ToneStyle::New),
                    _ => {
                        eprintln!("--tone phải là old hoặc new");
                        std::process::exit(2);
                    }
                };
            }
            "--mode" => {
                let v = next_value(&mut it, "--mode");
                args.mode = match v.as_str() {
                    "replace" => OutputMode::Replace,
                    "preedit" => OutputMode::Preedit,
                    _ => {
                        eprintln!("--mode phải là replace hoặc preedit");
                        std::process::exit(2);
                    }
                };
            }
            "--nfd" => args.nfd = true,
            "--no-spell" => args.no_spell = true,
            "--allow-coda-k" => args.allow_coda_k = true,
            "--macro-digits" => args.macro_digits = true,
            "--trace" => args.trace = true,
            "--macro" => {
                let v = next_value(&mut it, "--macro");
                match v.split_once('=') {
                    Some((k, value)) if !k.trim().is_empty() => {
                        args.macros.push((k.trim().to_lowercase(), value.to_string()));
                    }
                    _ => {
                        eprintln!("--macro cần dạng KHOÁ=GIÁ TRỊ, nhận: {v:?}");
                        std::process::exit(2);
                    }
                }
            }
            "--version" | "-V" => {
                println!("ViFastKey {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other => {
                eprintln!("Tham số không rõ: {other}\n\n{HELP}");
                std::process::exit(2);
            }
        }
    }
    args
}

fn main() {
    let args = parse_args();
    let mut macros = HashMap::from([
        ("ko".to_string(), "không".to_string()),
        ("vn".to_string(), "Việt Nam".to_string()),
        ("dc".to_string(), "được".to_string()),
    ]);
    for (k, v) in &args.macros {
        macros.insert(k.clone(), v.clone());
    }
    let mut config = Config { macros, ..Config::default() };
    if let Some(m) = args.method {
        config.input_method = m;
    }
    if let Some(t) = args.tone {
        config.tone_style = t;
    }
    config.unicode_form = if args.nfd { UnicodeForm::Nfd } else { UnicodeForm::Nfc };
    config.spell_check = !args.no_spell;
    config.allow_coda_k = args.allow_coda_k;
    config.macro_digits = args.macro_digits;

    let mut r = Repl::new(config, args.mode, args.trace);
    if args.demo {
        for line in DEMO {
            println!("{:<60} -> {:?}", format!("{line:?}"), r.type_line(line));
        }
        r.command(":vni");
        let l = "Vie65t Nam, nguo7i2 d9i";
        println!("{:<60} -> {:?}", format!("{l:?}"), r.type_line(l));
        return;
    }
    if !args.texts.is_empty() {
        for line in &args.texts {
            println!("{}", r.type_line(line));
        }
        return;
    }

    println!("{HELP}");
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
