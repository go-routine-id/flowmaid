use std::time::Instant;
// worst and mean route_partial frame (ms): each node dragged 1, 8 and 40 px
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::model::Document::Flowchart(g) | flowmaid::model::Document::State(g) => g,
            _ => continue,
        };
        let s = flowmaid::scene::scene(&g);
        let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let base = flowmaid::scene::route(&g, &at);
        let (mut worst, mut sum, mut n) = (0.0f64, 0.0, 0);
        for v in 0..at.len() {
            for d in [(1.0, 0.0), (0.0, 8.0), (40.0, 25.0)] {
                let mut c = at.clone();
                c[v] = (c[v].0 + d.0, c[v].1 + d.1);
                let mut best = f64::INFINITY;
                for _ in 0..3 {
                    let t = Instant::now();
                    std::hint::black_box(flowmaid::scene::route_partial(&g, &c, &base, &at));
                    best = best.min(t.elapsed().as_secs_f64() * 1e3);
                }
                worst = worst.max(best); sum += best; n += 1;
            }
        }
        println!("{:<16} partial worst={worst:6.2}ms mean={:6.2}ms frames={n}", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), sum / n as f64);
    }
}
