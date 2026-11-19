//! Scratch (not committed): time route() on a file.
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let n: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let src = std::fs::read_to_string(&path).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
        _ => return,
    };
    let sc = flowmaid::scene::scene(&g);
    let centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut best = f64::MAX;
    for _ in 0..n {
        let t = std::time::Instant::now();
        let r = flowmaid::scene::route(&g, &centers);
        std::hint::black_box(r);
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    println!("route best {:.2} ms", best);
}
