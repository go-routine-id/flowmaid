//! dump <file> [ids...]: scene edges (from, to, waypoints) and clusters, blocks on unless MODE=off.
use flowmaid::model::Document;
fn main() {
    let on = std::env::var("MODE").map(|m| m != "off").unwrap_or(true);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let src = std::fs::read_to_string(&args[0]).unwrap();
    let ids: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
    let sc = flowmaid::quality::with_blocks(on, || {
        let doc = flowmaid::parser::parse_document(&src).unwrap();
        let g = match doc { Document::Flowchart(g) => g, Document::State(g) => g, _ => panic!() };
        flowmaid::scene::scene(&g)
    });
    let want = |s: &str| ids.is_empty() || ids.contains(&s);
    for c in &sc.clusters { if want(&c.id) { println!("C {} x={:.1} y={:.1} w={:.1} h={:.1}", c.id, c.x, c.y, c.w, c.h); } }
    for n in &sc.nodes { if want(&n.id) { println!("N {} x={:.1} y={:.1} w={:.1} h={:.1}", n.id, n.x, n.y, n.w, n.h); } }
    for (i, e) in sc.edges.iter().enumerate() {
        if want(&e.from) || want(&e.to) {
            let w: Vec<String> = e.waypoints.iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect();
            println!("E{i} {}->{} bez={:?} wp={}", e.from, e.to, e.bezier.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>(), w.join(" "));
        }
    }
}
