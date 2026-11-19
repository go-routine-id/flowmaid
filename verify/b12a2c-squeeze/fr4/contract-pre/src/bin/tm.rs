use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;
fn best<T>(n: usize, mut f: impl FnMut() -> T) -> f64 {
    let mut b = f64::INFINITY;
    for _ in 0..n { let t = Instant::now(); let r = f(); b = b.min(t.elapsed().as_secs_f64() * 1e3); std::hint::black_box(r); }
    b
}
fn main() {
    let blocks = std::env::var("BLOCKS").map(|v| v == "1").unwrap_or(false);
    let reps: usize = std::env::var("REPS").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        flowmaid::quality::with_blocks(blocks, || {
            let st = best(reps, || scene::scene(&g));
            let sc = scene::scene(&g);
            let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = best(reps.min(10), || scene::route(&g, &c));
            let r = scene::route(&g, &c);
            let n = g.nodes.len();
            let mut worst = 0.0f64;
            for &k in &[0usize, n / 2, n.saturating_sub(1)] {
                for d in [(1.0, 0.0), (60.0, 25.0), (-150.0, 80.0)] {
                    let mut c2 = c.clone(); c2[k].0 += d.0; c2[k].1 += d.1;
                    let t = best(reps.min(10), || scene::route_partial(&g, &c2, &r, &c));
                    worst = worst.max(t);
                }
            }
            println!("{name}\t{st:.3}\t{rt:.3}\t{worst:.3}");
        });
    }
}
