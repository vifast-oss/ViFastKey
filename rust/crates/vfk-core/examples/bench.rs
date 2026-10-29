//! cargo run -p vfk-core --release --example bench
use std::time::Instant;

use vfk_core::*;

fn main() {
    let words = ["nguwowif", "truwowngf", "khuyeens", "dduwowngf", "Vieejt", "thuowng", "quyeets"];
    let rounds = 20_000;
    let o = TransformOptions::default();
    let keys: usize = words.iter().map(|w| w.len()).sum::<usize>() * rounds;

    let t = Instant::now();
    let mut sink = 0usize;
    for _ in 0..rounds {
        for w in words {
            let cs: Vec<char> = w.chars().collect();
            for i in 1..=cs.len() {
                sink += transform(&cs[..i], &TELEX, &o, false).text.len();
            }
        }
    }
    let a = t.elapsed().as_nanos() as f64 / keys as f64 / 1000.0;

    let t = Instant::now();
    for _ in 0..rounds {
        for w in words {
            let mut c = Composer::new(&TELEX, o);
            for k in w.chars() {
                c.push(k);
                sink += c.result(false).text.len();
            }
        }
    }
    let b = t.elapsed().as_nanos() as f64 / keys as f64 / 1000.0;
    println!("transform: {a:.2} µs/phím   Composer: {b:.2} µs/phím   (sink {sink})");
}
