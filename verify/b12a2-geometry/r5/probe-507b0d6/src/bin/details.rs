use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&f).unwrap();
    let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => return };
    let sc = with_blocks(true, || scene::scene(&g));
    for c in &sc.clusters { println!("cluster {} x={:.1} y={:.1} w={:.1} h={:.1} title={:?}", c.id, c.x, c.y, c.w, c.h, c.title); }
    for e in &sc.edges { if e.waypoints.len() > 1 { println!("edge {}>{} wp={:?}", e.from, e.to, e.waypoints.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>()); } else { println!("edge {}>{} bez={:?}", e.from, e.to, e.bezier); } }
    let m = with_blocks(true, || flowmaid::quality::measure(&g, &sc));
    for d in &m.details { println!("detail {d}"); }
}
