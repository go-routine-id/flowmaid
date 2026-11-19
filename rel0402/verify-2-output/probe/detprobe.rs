//! Cross-process determinism of the new functions: hashes of route_partial frames and pick grids.
use flowmaid::model::Document;
use flowmaid::{class, er, journey, mindmap, parser::parse_document, pie, sankey, seq, timeline};
use std::io::BufRead;
fn h(s: &str) -> u64 { let mut x: u64 = 0xcbf29ce484222325; for b in s.bytes() { x ^= b as u64; x = x.wrapping_mul(0x100000001b3); } x }
fn grid<T: std::fmt::Debug>(w: f64, ht: f64, f: impl Fn(f64, f64, f64) -> T) -> u64 {
    let mut s = String::new();
    for a in 0..50 { for b in 0..50 { let r = f(w * (a as f64 + 0.3) / 50.0, ht * (b as f64 + 0.7) / 50.0, 3.0); s.push_str(&format!("{r:?};")); } }
    h(&s)
}
fn main() {
    let list = std::env::args().nth(1).unwrap();
    for line in std::io::BufReader::new(std::fs::File::open(list).unwrap()).lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let out = match parse_document(&src) {
            Ok(Document::Er(d)) if !d.entities.is_empty() => {
                let b = er::scene(&d); let at: Vec<(f64, f64)> = b.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut t = at.clone(); let k = t.len() / 2; t[k].0 += 55.0; t[k].1 -= 31.0;
                let f = er::route_partial(&d, &t, &b, &at);
                format!("{:x} {:x}", h(&format!("{f:?}")), h(&er::to_svg(&f)))
            }
            Ok(Document::Class(d)) if !d.classes.is_empty() => {
                let b = class::scene(&d); let at: Vec<(f64, f64)> = b.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut t = at.clone(); let k = d.classes.len() / 2; t[k].0 += 55.0; t[k].1 -= 31.0;
                let f = class::route_partial(&d, &t, &b, &at);
                format!("{:x} {:x} {:x}", h(&format!("{f:?}")), h(&class::to_svg(&f)), h(&format!("{:?}", class::card_spots(&f))))
            }
            Ok(Document::Mindmap(d)) if !d.nodes.is_empty() => {
                let b = mindmap::scene(&d); let at: Vec<(f64, f64)> = b.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                let mut t = at.clone(); let k = t.len() / 2; t[k].0 += 55.0;
                let f = mindmap::route_partial(&d, &t, &b, &at);
                format!("{:x} {:x}", h(&format!("{f:?}")), grid(b.width, b.height, |x, y, tol| b.hit_test(x, y, tol)))
            }
            Ok(Document::Sequence(d)) => { let s = seq::scene(&d); format!("{:x}", grid(s.width, s.height, |x, y, t| s.hit_test(x, y, t))) }
            Ok(Document::Pie(d)) => { let s = pie::scene(&d); format!("{:x}", grid(s.width, s.height, |x, y, t| s.hit_test(x, y, t))) }
            Ok(Document::Journey(d)) => { let s = journey::scene(&d); format!("{:x}", grid(s.width, s.height, |x, y, t| s.hit_test(x, y, t))) }
            Ok(Document::Timeline(d)) => { let s = timeline::scene(&d); format!("{:x}", grid(s.width, s.height, |x, y, t| s.hit_test(x, y, t))) }
            Ok(Document::Sankey(d)) => { let s = sankey::scene(&d); format!("{:x}", grid(s.width, s.height, |x, y, t| s.hit_test(x, y, t))) }
            _ => continue,
        };
        println!("{path}\t{out}");
    }
}
