//! Block mode: dump nodes, frames and edges (static and routed at the scene's centres).
//! usage: dump <file.mmd> [s|r] [ids...]   (no ids: every node and frame, edges touching nothing)
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    let mut a = std::env::args().skip(1);
    let f = a.next().unwrap();
    let mode = a.next().unwrap_or_else(|| "s".into());
    let ids: Vec<String> = a.collect();
    let src = std::fs::read_to_string(&f).unwrap();
    let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => return };
    let st = with_blocks(true, || scene::scene(&g));
    let sc = if mode == "r" {
        let at: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        with_blocks(true, || scene::route(&g, &at))
    } else { st };
    let want = |id: &str| ids.is_empty() || ids.iter().any(|x| x == id);
    for c in &sc.clusters { if want(&c.id) { println!("frame {} x {:.1}..{:.1} y {:.1}..{:.1}", c.id, c.x, c.x + c.w, c.y, c.y + c.h); } }
    for n in &sc.nodes { if want(&n.id) { println!("node {} c ({:.1},{:.1}) x {:.1}..{:.1} y {:.1}..{:.1}", n.id, n.x, n.y, n.x - n.w / 2.0, n.x + n.w / 2.0, n.y - n.h / 2.0, n.y + n.h / 2.0); } }
    for e in &sc.edges { if !ids.is_empty() && (want(&e.from) || want(&e.to)) {
        let w: Vec<String> = e.waypoints.iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect();
        println!("edge {}->{} bez ({:.1},{:.1})..({:.1},{:.1}) wps {}", e.from, e.to, e.bezier[0].0, e.bezier[0].1, e.bezier[3].0, e.bezier[3].1, w.join(" ")); } }
}
