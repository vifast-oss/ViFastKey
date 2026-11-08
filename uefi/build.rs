//! Rasterize subset ASCII + tiếng Việt của Be Vietnam Pro thành font atlas
//! 8-bit alpha. App UEFI chỉ cần ô cố định nên một bảng sorted là đủ.

use std::env;
use std::fs;
use std::path::PathBuf;

use fontdue::{Font, FontSettings};
use image::imageops::FilterType;
use qrcodegen::{QrCode, QrCodeEcc};

const CELL_W: usize = 14;
const CELL_H: usize = 26;
const FONT_PX: f32 = 16.0;
const BASELINE: i32 = 18;
const HEADER: usize = 8;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let ttf_path = manifest.join("../resources/fonts/BeVietnamPro-Regular.ttf");
    println!("cargo:rerun-if-changed={}", ttf_path.display());

    let ttf = fs::read(&ttf_path).unwrap_or_else(|e| panic!("read {}: {e}", ttf_path.display()));
    let font = Font::from_bytes(ttf, FontSettings::default()).expect("parse Be Vietnam Pro");

    for ch in ['ế', 'ộ', 'ữ', 'đ', 'Đ', 'ư', 'ơ', 'ă', 'â', 'ê', 'ô'] {
        assert!(font.has_glyph(ch), "font thiếu glyph {ch:?}");
    }

    let mut chars: Vec<char> = (0x20u32..=0x7E).filter_map(char::from_u32).collect();
    for cp in 0x00A0u32..=0x1EF9 {
        if let Some(ch) = char::from_u32(cp) {
            if font.has_glyph(ch) {
                chars.push(ch);
            }
        }
    }
    for ch in [
        '…', '–', '—', '‘', '’', '“', '”', '•', '←', '→', '↑', '↓', '✓', '✗',
    ] {
        if font.has_glyph(ch) {
            chars.push(ch);
        }
    }
    chars.sort_unstable();
    chars.dedup();

    let count: u16 = chars.len().try_into().expect("quá nhiều glyph");
    let mut out = Vec::with_capacity(HEADER + chars.len() * (4 + CELL_W * CELL_H));
    out.extend_from_slice(b"VFKF");
    out.push(CELL_W as u8);
    out.push(CELL_H as u8);
    out.extend_from_slice(&count.to_le_bytes());

    for ch in chars {
        let (metrics, bitmap) = font.rasterize(ch, FONT_PX);
        let mut cell = [0u8; CELL_W * CELL_H];
        let dest_x = (CELL_W as i32 - metrics.width as i32) / 2;
        let dest_y = BASELINE - (metrics.height as i32 + metrics.ymin);

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let x = dest_x + col as i32;
                let y = dest_y + row as i32;
                if (0..CELL_W as i32).contains(&x) && (0..CELL_H as i32).contains(&y) {
                    let alpha = bitmap[row * metrics.width + col];
                    if alpha != 0 {
                        cell[y as usize * CELL_W + x as usize] = alpha;
                    }
                }
            }
        }

        out.extend_from_slice(&(ch as u32).to_le_bytes());
        out.extend_from_slice(&cell);
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let out_path = out_dir.join("vietnamese_font.bin");
    fs::write(&out_path, &out).expect("write font atlas");
    println!(
        "cargo:warning=font atlas: {} glyphs, {} bytes",
        count,
        out.len()
    );

    // Logo 96x96 RGBA: đủ nét khi vẽ 36px ở header và 64px trong help.
    let logo_path = manifest.join("../resources/brand/vifastkey-icon-192.png");
    println!("cargo:rerun-if-changed={}", logo_path.display());
    let logo = image::open(&logo_path)
        .expect("read ViFastKey logo")
        .to_rgba8();
    let logo = image::imageops::resize(&logo, 96, 96, FilterType::Lanczos3);
    let mut logo_out = Vec::with_capacity(8 + 96 * 96 * 4);
    logo_out.extend_from_slice(b"VFKL");
    logo_out.extend_from_slice(&96u16.to_le_bytes());
    logo_out.extend_from_slice(&96u16.to_le_bytes());
    logo_out.extend_from_slice(logo.as_raw());
    fs::write(out_dir.join("logo.bin"), &logo_out).expect("write logo");
    println!("cargo:warning=logo: {} bytes", logo_out.len());

    // QR trang chủ; có quiet zone 4 module để quét dễ hơn.
    let qr = QrCode::encode_text("https://key.vifast.org/", QrCodeEcc::Medium)
        .expect("encode homepage QR");
    let quiet = 4usize;
    let qr_size = qr.size() as usize + quiet * 2;
    let mut qr_out = Vec::with_capacity(6 + qr_size * qr_size);
    qr_out.extend_from_slice(b"VFKQ");
    qr_out.extend_from_slice(&(qr_size as u16).to_le_bytes());
    for y in 0..qr_size {
        for x in 0..qr_size {
            let mx = x as i32 - quiet as i32;
            let my = y as i32 - quiet as i32;
            let on =
                mx >= 0 && my >= 0 && mx < qr.size() && my < qr.size() && qr.get_module(mx, my);
            qr_out.push(on as u8);
        }
    }
    fs::write(out_dir.join("homepage_qr.bin"), &qr_out).expect("write QR");
    println!("cargo:warning=homepage QR: {} bytes", qr_out.len());
}
