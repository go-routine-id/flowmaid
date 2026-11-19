//! Sum and worst of route_partial / route over quality::drag_frames, each frame the fastest of 5.
use flowmaid::quality::{drag_frames, fastest_ms};
use flowmaid::scene::{route, route_partial, scene};
use flowmaid::Document;
fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).expect("read");
        let g = match flowmaid::parser::parse_document(&src).expect("parses") {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => continue,
        };
        let st = scene(&g);
        let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let (mut sp, mut sr, mut wp, mut wr, mut n) = (0.0, 0.0, 0.0f64, 0.0f64, 0);
        for (_, c) in drag_frames(&st) {
            let p = fastest_ms(5, || route_partial(&g, &c, &st, &base));
            let r = fastest_ms(5, || route(&g, &c));
            sp += p; sr += r; wp = wp.max(p); wr = wr.max(r); n += 1;
        }
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name}: {n} frames; route_partial sum {sp:.1} ms worst {wp:.2}; route sum {sr:.1} ms worst {wr:.2}");
    }
}
