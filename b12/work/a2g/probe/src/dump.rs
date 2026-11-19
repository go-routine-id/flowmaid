//! Block mode: dump frames and the edges touching given ids (static scene).
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    let mut a = std::env::args().skip(1);
    let f = a.next().unwrap();
    let ids: Vec<String> = a.collect();
    let src = std::fs::read_to_string(&f).unwrap();
    let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => return };
    let sc = with_blocks(true, || scene::scene(&g));
    for c in &sc.clusters { if ids.is_empty() || ids.contains(&c.id) { println!("frame {} ({:.1},{:.1},{:.1},{:.1})", c.id, c.x, c.y, c.w, c.h); } }
    for n in &sc.nodes { if ids.contains(&n.id) { println!("node {} ({:.1},{:.1}) {:.1}x{:.1}", n.id, n.x, n.y, n.w, n.h); } }
    for e in &sc.edges { if ids.contains(&e.from) || ids.contains(&e.to) {
        let w: Vec<String> = e.waypoints.iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect();
        println!("edge {}->{} bez=({:.1},{:.1})..({:.1},{:.1}) wps={}", e.from, e.to, e.bezier[0].0, e.bezier[0].1, e.bezier[3].0, e.bezier[3].1, w.join(" ")); } }
}
