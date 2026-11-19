// The route_partial worst frame per fixture, as router::tests::fixture_drag_perf_guard times it
// (fastest of DRAG_RUNS, re-timed with RETIME_RUNS over 8 ms), without stopping at the first miss.
use flowmaid::model::Document;
use flowmaid::quality::{drag_frames, fastest_ms, DRAG_FRAME_MS, DRAG_RUNS};
use flowmaid::scene::{route_partial, scene};
fn main() {
    let retime: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    for name in ["quality/dense-lr", "quality/dense-td", "quality/bench-75", "blocks/blocks-overview"] {
        let src = std::fs::read_to_string(format!("/home/user/flowmaid/tests/fixtures/{name}.mmd")).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => unreachable!(),
        };
        let st = scene(&g);
        let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut worst = (0.0f64, 0usize);
        let mut over = 0;
        for (v, c) in drag_frames(&st) {
            let mut ms = fastest_ms(DRAG_RUNS, || route_partial(&g, &c, &st, &base));
            if ms >= DRAG_FRAME_MS { ms = ms.min(fastest_ms(retime, || route_partial(&g, &c, &st, &base))); }
            if ms > 6.0 { over += 1; }
            if ms > worst.0 { worst = (ms, v); }
        }
        println!("{name}: route_partial worst {:.2} ms (node {}), {over} frames over 6 ms", worst.0, worst.1);
    }
}
