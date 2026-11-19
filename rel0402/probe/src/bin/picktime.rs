use flowmaid::model::Document;
use flowmaid::{journey, mindmap, parser, pie, sankey, seq, timeline};
use std::time::Instant;
fn grid(w: f64, h: f64, f: &dyn Fn(f64, f64) -> bool) -> (usize, f64) {
    let (mut n, mut hits) = (0usize, 0usize);
    let t = Instant::now();
    let mut y = 0.0;
    while y <= h { let mut x = 0.0; while x <= w { if f(x, y) { hits += 1; } n += 1; x += 4.0; } y += 4.0; }
    let us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
    let _ = hits;
    (n, us)
}
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let (n, us) = match parser::parse_document(&src).unwrap() {
            Document::Mindmap(d) => { let s = mindmap::scene(&d); grid(s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Document::Sequence(d) => { let s = seq::scene(&d); grid(s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Document::Pie(d) => { let s = pie::scene(&d); grid(s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Document::Journey(d) => { let s = journey::scene(&d); grid(s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Document::Timeline(d) => { let s = timeline::scene(&d); grid(s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Document::Sankey(d) => { let s = sankey::scene(&d); grid(s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            _ => continue,
        };
        println!("{name}\t{n} picks\t{us:.2} us/pick");
    }
}
