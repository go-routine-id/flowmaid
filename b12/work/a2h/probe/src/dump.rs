//! Block mode: dump frames, nodes and edges (static scene, or routed with ROUTED=1).
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    let mut a = std::env::args().skip(1);
    let f = a.next().unwrap();
    let ids: Vec<String> = a.collect();
    let src = std::fs::read_to_string(&f).unwrap();
    let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => return };
    let on = std::env::var("OFF").is_err();
    let mut sc = with_blocks(on, || scene::scene(&g));
    if std::env::var("ROUTED").is_ok() {
        let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        sc = with_blocks(on, || scene::route(&g, &c));
    }
    println!("canvas {:.1} x {:.1}", sc.width, sc.height);
    let all = ids.is_empty();
    for c in &sc.clusters { if all || ids.contains(&c.id) { println!("frame {} '{}' ({:.1},{:.1},{:.1},{:.1}) bottom={:.1} right={:.1}", c.id, c.title, c.x, c.y, c.w, c.h, c.y + c.h, c.x + c.w); } }
    for n in &sc.nodes { if all || ids.contains(&n.id) { println!("node {} ({:.1},{:.1}) {:.1}x{:.1}", n.id, n.x, n.y, n.w, n.h); } }
    for e in &sc.edges { if all || ids.contains(&e.from) || ids.contains(&e.to) {
        let w: Vec<String> = e.waypoints.iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect();
        println!("edge {}->{} bez=({:.1},{:.1})..({:.1},{:.1}) lbl={:?} wps={}", e.from, e.to, e.bezier[0].0, e.bezier[0].1, e.bezier[3].0, e.bezier[3].1, e.label.as_ref().map(|l| (l.0.clone(), (l.1 .0 * 10.0).round() / 10.0, (l.1 .1 * 10.0).round() / 10.0, (l.2*10.0).round()/10.0)), w.join(" ")); } }
}
