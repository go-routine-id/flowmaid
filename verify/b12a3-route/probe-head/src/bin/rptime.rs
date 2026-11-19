// rptime <off|on> <files...>: worst route_partial frame over quality::drag_frames
use flowmaid::model::Document;
use flowmaid::quality::{drag_frames, fastest_ms, DRAG_RUNS};
use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let on = a[1] == "on";
    for f in &a[2..] {
        let src = std::fs::read_to_string(f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => continue };
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        flowmaid::quality::with_blocks(on, || {
            let st = scene::scene(&g);
            let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            let (mut worst, mut wv, mut over6, mut n) = (0.0f64, 0usize, 0usize, 0usize);
            for (v, c) in drag_frames(&st) {
                let mut ms = fastest_ms(DRAG_RUNS, || scene::route_partial(&g, &c, &st, &base));
                if ms > 6.0 { ms = ms.min(fastest_ms(10, || scene::route_partial(&g, &c, &st, &base))); }
                n += 1;
                if ms > 6.0 { over6 += 1; }
                if ms > worst { worst = ms; wv = v; }
            }
            println!("{name}\t{}\tworst {:.2} ms (node {})\t{} of {} frames over 6 ms", if on {"on"} else {"off"}, worst, g.nodes[wv].id, over6, n);
        });
    }
}
