// rpworst <off|on> files...: the fixture_drag_perf_guard loop (route_partial worst frame, ms).
use flowmaid::model::{Document, Graph};
use flowmaid::quality::{drag_frames, fastest_ms, DRAG_RUNS};
fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let on = a[1] == "on";
    let retime: usize = std::env::var("RETIME").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    for f in &a[2..] {
        let Some(g) = graph_of(&std::fs::read_to_string(f).unwrap()) else { continue };
        let (worst, node, over6, frames) = flowmaid::quality::with_blocks(on, || {
            let st = flowmaid::scene::scene(&g);
            let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            let (mut worst, mut node, mut over6, mut frames) = (0.0f64, 0usize, 0usize, 0usize);
            for (v, c) in drag_frames(&st) {
                frames += 1;
                let t = |n: usize| fastest_ms(n, || flowmaid::scene::route_partial(&g, &c, &st, &base));
                let mut ms = t(DRAG_RUNS);
                if ms >= 6.0 {
                    ms = ms.min(t(retime));
                }
                if ms >= 6.0 {
                    over6 += 1;
                }
                if ms > worst {
                    worst = ms;
                    node = v;
                }
            }
            (worst, node, over6, frames)
        });
        println!("{}\troute_partial worst {:.2} ms (node {})\t{} of {} frames >= 6 ms", std::path::Path::new(f).file_stem().unwrap().to_string_lossy(), worst, node, over6, frames);
    }
}
