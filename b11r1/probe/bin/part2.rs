use std::time::Instant;
// part2 <file>: the slowest route_partial frames (best of 15 runs each): each node dragged 1, 8 and 40 px
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
        let mut all: Vec<(f64, String)> = Vec::new();
        for v in 0..at.len() {
            for d in [(1.0, 0.0), (0.0, 8.0), (40.0, 25.0)] {
                let mut c = at.clone();
                c[v] = (c[v].0 + d.0, c[v].1 + d.1);
                let mut best = f64::INFINITY;
                for _ in 0..15 {
                    let t = Instant::now();
                    std::hint::black_box(flowmaid::scene::route_partial(&g, &c, &base, &at));
                    best = best.min(t.elapsed().as_secs_f64() * 1e3);
                }
                all.push((best, format!("{} {:?}", g.nodes[v].id, d)));
            }
        }
        all.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mean = all.iter().map(|a| a.0).sum::<f64>() / all.len() as f64;
        println!("{:<14} mean {mean:.2} ms; slowest: {}", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), all.iter().take(5).map(|a| format!("{:.2} [{}]", a.0, a.1)).collect::<Vec<_>>().join("  "));
    }
}
