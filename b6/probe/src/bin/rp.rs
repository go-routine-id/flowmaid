use flowmaid::{parser::parse_document, Document, scene, quality};
use std::time::Instant;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => continue };
        let st = scene::scene(&g);
        let at: Vec<(f64,f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = scene::route(&g, &at);
        let mut worst = 0.0f64; let mut tot = 0.0; let mut n = 0;
        for (_, c) in quality::drag_frames(&rt).into_iter().step_by(3) {
            let mut best = f64::INFINITY;
            for _ in 0..3 { let t = Instant::now(); let _ = scene::route_partial(&g, &c, &rt, &at); best = best.min(t.elapsed().as_secs_f64()*1e3); }
            worst = worst.max(best); tot += best; n += 1;
        }
        println!("{} route_partial worst {:.2} ms mean {:.2}", f.rsplit('/').next().unwrap(), worst, tot / n as f64);
    }
}
