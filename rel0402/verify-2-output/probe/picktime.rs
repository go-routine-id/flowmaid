//! Per-pick cost: for each file, a 80x80 grid; per point the min over 7 reps (noise-free), then mean and max over points.
use flowmaid::model::Document;
use flowmaid::{journey, mindmap, parser::parse_document, pie, sankey, seq, timeline};
use std::time::Instant;
fn run(name: &str, kind: &str, w: f64, h: f64, f: &dyn Fn(f64, f64) -> bool) {
    let mut mins = Vec::new();
    for a in 0..80 { for b in 0..80 {
        let (x, y) = (w * (a as f64 + 0.41) / 80.0, h * (b as f64 + 0.59) / 80.0);
        let mut best = u128::MAX;
        for _ in 0..7 { let t = Instant::now(); std::hint::black_box(f(x, y)); best = best.min(t.elapsed().as_nanos()); }
        mins.push(best as f64 / 1e3);
    }}
    let mean = mins.iter().sum::<f64>() / mins.len() as f64;
    let max = mins.iter().cloned().fold(0.0, f64::max);
    println!("{kind:9} mean {mean:8.3} us  worst point {max:9.3} us  {name}");
}
fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let name = path.rsplit('/').take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/");
        match parse_document(&src) {
            Ok(Document::Mindmap(d)) => { let s = mindmap::scene(&d); run(&name, "mindmap", s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Ok(Document::Sequence(d)) => { let s = seq::scene(&d); run(&name, "seq", s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Ok(Document::Pie(d)) => { let s = pie::scene(&d); run(&name, "pie", s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Ok(Document::Journey(d)) => { let s = journey::scene(&d); run(&name, "journey", s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Ok(Document::Timeline(d)) => { let s = timeline::scene(&d); run(&name, "timeline", s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            Ok(Document::Sankey(d)) => { let s = sankey::scene(&d); run(&name, "sankey", s.width, s.height, &|x, y| s.hit_test(x, y, 4.0).is_some()) }
            _ => {}
        }
    }
}
