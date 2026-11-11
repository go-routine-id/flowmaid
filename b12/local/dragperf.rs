//! Local (never committed): the loop of `router::tests::fixture_drag_perf_guard`
//! for any `.mmd` files, e.g. the analog. Prints, per file, the worst
//! `route_partial` frame and the worst `route()` frame over
//! `quality::drag_frames`.
//!   cp b12/local/dragperf.rs <repo>/examples/ && cargo run --release --example dragperf -- a.mmd b.mmd
use flowmaid::quality::{drag_frames, fastest_ms, DRAG_FRAME_MS, DRAG_RUNS};
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
        let steady = |ms: f64, time: &dyn Fn(usize) -> f64| if ms < DRAG_FRAME_MS { ms } else { ms.min(time(10)) };
        let (mut full, mut part, mut over) = ((0.0f64, 0), (0.0f64, 0), 0);
        for (v, c) in drag_frames(&st) {
            let time = |n: usize| fastest_ms(n, || route(&g, &c));
            let ms = steady(time(DRAG_RUNS), &time);
            over += usize::from(ms > DRAG_FRAME_MS);
            if ms > full.0 { full = (ms, v); }
            let time = |n: usize| fastest_ms(n, || route_partial(&g, &c, &st, &base));
            let ms = steady(time(DRAG_RUNS), &time);
            if ms > part.0 { part = (ms, v); }
        }
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name}: route_partial worst {:.2} ms (node {}); route() worst {:.2} ms (node {}), {over} frames over {DRAG_FRAME_MS} ms", part.0, part.1, full.0, full.1);
    }
}
