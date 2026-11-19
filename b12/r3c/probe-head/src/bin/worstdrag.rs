// worstdrag <file>: block mode; route() per quality::drag_frames frame (fastest of 5), top 8 frames.
use flowmaid::model::Document;
use flowmaid::quality::{drag_frames, fastest_ms};
use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => panic!() };
    flowmaid::quality::with_blocks(true, || {
        let st = scene::scene(&g);
        let at: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut v: Vec<(f64, usize, (f64, f64), f64)> = Vec::new();
        for (n, c) in drag_frames(&st) {
            let r = fastest_ms(5, || scene::route(&g, &c));
            let p = fastest_ms(5, || scene::route_partial(&g, &c, &st, &at));
            v.push((r, n, (c[n].0 - at[n].0, c[n].1 - at[n].1), p));
        }
        v.sort_by(|x, y| y.0.total_cmp(&x.0));
        for (r, n, d, p) in v.iter().take(8) { println!("route {r:.2} ms  route_partial {p:.2} ms  node {} by ({:.0},{:.0})", g.nodes[*n].id, d.0, d.1); }
        let mean: f64 = v.iter().map(|x| x.0).sum::<f64>() / v.len() as f64;
        println!("frames {} mean {mean:.2}", v.len());
    });
}
