// pk <file>: time picks over a grid (static scene)
use std::time::Instant;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::model::Document::Flowchart(g) | flowmaid::model::Document::State(g) => g,
            _ => continue,
        };
        let s = flowmaid::scene::scene(&g);
        let (mut n, mut hits) = (0u32, 0u32);
        let t = Instant::now();
        let step = (s.width.max(s.height) / 150.0).max(7.0);
        let mut y = 0.0;
        while y < s.height { let mut x = 0.0; while x < s.width { n += 1; if s.hit_test(x, y, 4.0).is_some() { hits += 1; } x += step; } y += step; }
        let us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        let t = Instant::now();
        let svg = flowmaid::scene::to_svg(&s);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        println!("{:<16} edges={:<3} picks={n} hits={hits} {us:.2} us/hit_test  to_svg {ms:.2} ms ({} bytes)", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), s.edges.len(), svg.len());
    }
}
