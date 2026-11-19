use flowmaid::model::Document;
fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let Document::Er(d) = flowmaid::parser::parse_document(&src).unwrap() else { panic!() };
    let es = flowmaid::er::scene(&d);
    for n in &es.scene.nodes { println!("node {} c=({:.1},{:.1}) {}x{}", n.id, n.x, n.y, n.w, n.h); }
    for e in &es.scene.edges {
        println!("edge {}->{} bez={:?}", e.from, e.to, e.bezier.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>());
        if !e.waypoints.is_empty() { let w = &e.waypoints; println!("   wps n={} first={:?} last3={:?}", w.len(), &w[..3.min(w.len())], &w[w.len().saturating_sub(4)..]); }
    }
}
