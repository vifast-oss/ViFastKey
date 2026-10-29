//! vfk-charset: chuyển Unicode <-> bảng mã tiếng Việt.
//!
//! vfk-charset --list
//! vfk-charset --to tcvn3 --text "Tiếng Việt" > vi.tcvn
//! vfk-charset --from tcvn3 --file vi.tcvn --to unicode
//! vfk-charset --detect --file vi.tcvn

use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};

use vfk_core::{charset, CHARSETS};

struct Args {
    to: Option<String>,
    from: Option<String>,
    texts: Vec<String>,
    file: Option<String>,
    output: Option<String>,
    detect: bool,
    list: bool,
    clipboard: bool,
    case: Option<String>,
    remove_diacritics: bool,
}

fn usage() -> &'static str {
    "vfk-charset — chuyển bảng mã tiếng Việt\n\
     \n\
     --list                         liệt kê bảng mã\n\
     --to NAME --text \"...\"        Unicode -> bảng mã\n\
     --from NAME --file FILE        bảng mã -> Unicode (mặc định --to unicode)\n\
     --detect --file FILE           đoán bảng mã\n\
     --output FILE                  ghi ra file\n\
     \n\
     Bảng một byte: TCVN3, VNI Windows, VISCII, VPS, BKHCM 1/2, Vietware X/Full.\n\
     Bảng văn bản: Unicode/NFD, VIQR, CP1258, NCR, C string."
}

fn parse() -> Args {
    let mut a = Args {
        to: None,
        from: None,
        texts: Vec::new(),
        file: None,
        output: None,
        detect: false,
        list: false,
        clipboard: false,
        case: None,
        remove_diacritics: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--to" | "-t" => a.to = it.next(),
            "--from" | "-f" => a.from = it.next(),
            "--text" => a.texts.push(it.next().unwrap_or_default()),
            "--file" => a.file = it.next(),
            "--output" | "-o" => a.output = it.next(),
            "--detect" => a.detect = true,
            "--list" => a.list = true,
            "--clipboard" => a.clipboard = true,
            "--case" => a.case = it.next(),
            "--remove-diacritics" => a.remove_diacritics = true,
            "--version" | "-V" => {
                println!("vfk-charset {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => {
                eprintln!("Tham số không rõ: {other}\n\n{}", usage());
                std::process::exit(2);
            }
        }
    }
    a
}

fn main() {
    let args = parse();
    if args.list {
        for key in CHARSETS {
            println!("{key:18} {}", charset::display_name(key).unwrap_or(key));
        }
        return;
    }
    let from_key = args.from.as_deref().map(charset::normalize);
    let to_key = args.to.as_deref().map(charset::normalize);
    let from_single = from_key.is_some_and(charset::is_single_byte);
    let to_single = to_key.is_some_and(charset::is_single_byte);

    let raw: Vec<u8> = if args.clipboard {
        let out = Command::new("pbpaste").output().unwrap_or_else(|e| {
            eprintln!("Không đọc được clipboard: {e}");
            std::process::exit(1);
        });
        out.stdout
    } else if let Some(path) = &args.file {
        fs::read(path).unwrap_or_else(|e| {
            eprintln!("Không đọc được {path}: {e}");
            std::process::exit(1);
        })
    } else {
        args.texts.join("\n").into_bytes()
    };

    if args.detect {
        let text: String = raw.iter().map(|&b| b as char).collect();
        for key in charset::detect(&text) {
            println!("{key}");
        }
        return;
    }

    let text = if let Some(key) = from_key {
        if from_single {
            charset::decode_bytes(&raw, key)
        } else {
            let s = String::from_utf8_lossy(&raw);
            charset::decode(&s, key)
        }
    } else {
        String::from_utf8_lossy(&raw).into_owned()
    };

    let mut text = text;
    if args.remove_diacritics {
        text = charset::remove_diacritics(&text);
    }
    if let Some(mode) = &args.case {
        text = charset::change_case(&text, mode).unwrap_or_else(|| {
            eprintln!("--case phải là upper/lower/title");
            std::process::exit(2);
        });
    }

    let payload: Vec<u8> = match to_key {
        Some(key) if to_single => charset::encode_bytes(&text, key).unwrap_or_else(|| {
            eprintln!("Không mã hoá được bằng {key}");
            std::process::exit(1);
        }),
        Some(key) => charset::encode(&text, key).into_bytes(),
        None => text.into_bytes(),
    };

    if args.clipboard {
        let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn().unwrap_or_else(|e| {
            eprintln!("Không ghi được clipboard: {e}");
            std::process::exit(1);
        });
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&payload);
        }
        let _ = child.wait();
    } else if let Some(path) = args.output {
        if let Err(e) = fs::write(&path, &payload) {
            eprintln!("Không ghi được {path}: {e}");
            std::process::exit(1);
        }
    } else {
        let mut out = io::stdout();
        if let Err(e) = out.write_all(&payload) {
            eprintln!("Lỗi ghi stdout: {e}");
            std::process::exit(1);
        }
    }
}
