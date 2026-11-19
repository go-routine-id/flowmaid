use std::time::Instant;
fn main() {
    let reps: usize = std::env::var("REPS").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(flowmaid::model::Document::Flowchart(g)) | Ok(flowmaid::model::Document::State(g)) => g,
            _ => continue,
        };
        // adaptive reps: fewer for slow files
        let t0 = Instant::now();
        let _ = flowmaid::scene::scene(&g);
        let one = t0.elapsed().as_secs_f64();
        let n = if one > 0.005 { 20 } else { reps };
        let mut best = f64::INFINITY;
        for _ in 0..n {
            let t = Instant::now();
            let s = flowmaid::scene::scene(&g);
            std::hint::black_box(&s);
            best = best.min(t.elapsed().as_secs_f64());
        }
        println!("{}\t{:.4}", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), best * 1e3);
    }
}
