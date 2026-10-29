use std::io::Read;

use vfk_suggest::{default_engine, SuggestionEngine};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut prefix = None;
    let mut limit = 8usize;
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--limit" => limit = args.next().and_then(|v| v.parse().ok()).unwrap_or(8),
            "--user" => {
                if let Some(path) = args.next() {
                    files.push(path);
                }
            }
            "--help" | "-h" => {
                println!("vfk-suggest PREFIX [--limit N] [--user FILE]");
                return;
            }
            _ if prefix.is_none() => prefix = Some(arg),
            _ => {
                eprintln!("Tham số không rõ: {arg}");
                std::process::exit(2);
            }
        }
    }
    let Some(prefix) = prefix else {
        eprintln!("Cần PREFIX");
        std::process::exit(2);
    };
    let mut engine = SuggestionEngine::new(std::iter::empty::<&str>(), limit);
    if files.is_empty() {
        for word in default_engine().suggest(&prefix, limit) {
            println!("{word}");
        }
        return;
    }
    for path in files {
        if let Ok(mut f) = std::fs::File::open(&path) {
            let mut text = String::new();
            if f.read_to_string(&mut text).is_ok() {
                engine.load_text(&text);
            }
        }
    }
    // Nạp thêm lexicon mặc định (nếu có) bằng cách lấy gợi ý rồi bổ sung.
    engine.add_words(default_engine().suggest(&prefix, 10_000));
    for word in engine.suggest(&prefix, limit) {
        println!("{word}");
    }
}
