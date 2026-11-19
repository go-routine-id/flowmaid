//! route_partial timing over quality::drag_frames: per frame fastest of
//! 3 runs; prints worst, p95, median and total per file.
use flowmaid::model::Document;
use flowmaid::quality::{drag_frames, fastest_ms};
use flowmaid::scene;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => continue,
        };
        let st = scene::scene(&g);
        let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut ms: Vec<f64> = Vec::new();
        for (_v, c) in drag_frames(&st) {
            ms.push(fastest_ms(3, || scene::route_partial(&g, &c, &st, &base)));
        }
        let tot: f64 = ms.iter().sum();
        ms.sort_by(|a, b| a.total_cmp(b));
        let n = ms.len();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name}\tframes={n}\tworst={:.2}\tp95={:.2}\tmedian={:.2}\ttotal={:.1}", ms[n - 1], ms[(n * 95 / 100).min(n - 1)], ms[n / 2], tot);
    }
}
