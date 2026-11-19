//! Default build: worst route_partial frame (ms, best of 5 per node) for a 3 px drag of each node.
use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        let sc = scene::scene(&g);
        let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut worst = 0.0f64; let mut sum = 0.0;
        for v in 0..base.len() {
            let mut pos = base.clone(); pos[v].0 += 3.0;
            let mut best = f64::MAX;
            for _ in 0..5 { let t = Instant::now(); let p = scene::route_partial(&g, &pos, &sc, &base); std::hint::black_box(&p); best = best.min(t.elapsed().as_secs_f64() * 1e3); }
            worst = worst.max(best); sum += best;
        }
        println!("{}\tworst={:.2}\tmean={:.2}", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), worst, sum / base.len().max(1) as f64);
    }
}
