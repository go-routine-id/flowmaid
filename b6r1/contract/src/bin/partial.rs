use flowmaid::model::{Document, Graph};
use flowmaid::scene;
use std::time::Instant;
fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn main() {
    for f in std::env::args().skip(1) {
        let g = graph_of(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let sc = scene::scene(&g);
        let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = scene::route(&g, &c);
        let frames = flowmaid::quality::drag_frames(&r);
        let mut worst: f64 = 0.0;
        let mut sum = 0.0;
        let mut over6 = 0;
        let mut rworst: f64 = 0.0;
        let mut rsum = 0.0;
        for (_, cs) in &frames {
            let mut best = f64::INFINITY;
            for _ in 0..3 {
                let t = Instant::now();
                let p = scene::route_partial(&g, cs, &r, &c);
                best = best.min(t.elapsed().as_secs_f64() * 1e3);
                std::hint::black_box(p);
            }
            worst = worst.max(best);
            sum += best;
            if best > 6.0 { over6 += 1; }
            let mut rb = f64::INFINITY;
            for _ in 0..2 {
                let t = Instant::now();
                let p = scene::route(&g, cs);
                rb = rb.min(t.elapsed().as_secs_f64() * 1e3);
                std::hint::black_box(p);
            }
            rworst = rworst.max(rb);
            rsum += rb;
        }
        let n = frames.len() as f64;
        println!("{}: frames={} partial worst={worst:.2} mean={:.2} over6={over6} | route worst={rworst:.2} mean={:.2}", f, frames.len(), sum / n, rsum / n);
    }
}
