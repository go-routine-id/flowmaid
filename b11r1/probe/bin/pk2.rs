// pk2 <file>...: time hit_test at points ON the edges (every ~5 px of each scene path), static scene
use std::time::Instant;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::model::Document::Flowchart(g) | flowmaid::model::Document::State(g) => g,
            _ => continue,
        };
        let s = flowmaid::scene::scene(&g);
        let mut pts: Vec<(f64, f64)> = Vec::new();
        for e in &s.edges {
            let w: Vec<(f64, f64)> = if e.waypoints.len() >= 2 { e.waypoints.clone() } else { vec![e.bezier[0], e.bezier[3]] };
            for q in w.windows(2) {
                let l = (q[1].0 - q[0].0).hypot(q[1].1 - q[0].1);
                let n = (l / 5.0).ceil().max(1.0) as usize;
                for k in 0..n {
                    let t = k as f64 / n as f64;
                    pts.push((q[0].0 + (q[1].0 - q[0].0) * t, q[0].1 + (q[1].1 - q[0].1) * t - 3.0));
                }
            }
        }
        let mut best = f64::INFINITY;
        let mut hits = 0u32;
        for _ in 0..3 {
            hits = 0;
            let t = Instant::now();
            for &(x, y) in &pts { if s.hit_test(x, y, 4.0).is_some() { hits += 1; } }
            best = best.min(t.elapsed().as_secs_f64() * 1e6 / pts.len() as f64);
        }
        let mut svg_ms = f64::INFINITY;
        for _ in 0..5 { let t = Instant::now(); std::hint::black_box(flowmaid::scene::to_svg(&s)); svg_ms = svg_ms.min(t.elapsed().as_secs_f64() * 1e3); }
        println!("{:<16} edges={:<3} picks={} hits={hits} {best:.2} us/hit_test on strokes; to_svg {svg_ms:.2} ms", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), s.edges.len(), pts.len());
    }
}
