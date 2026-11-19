use flowmaid::model::Document;
fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::State(g) => g, _ => panic!() };
    let sc = flowmaid::scene::scene(&g);
    for (i, e) in sc.edges.iter().enumerate() {
        println!("e{i} {}->{} label {:?} bez {:?} wp {}", e.from, e.to, e.label.as_ref().map(|l| (&l.0, (l.1.0*10.0).round()/10.0, (l.1.1*10.0).round()/10.0)), e.bezier.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>(), e.waypoints.len());
    }
    for n in &sc.nodes { println!("n {} ({:.1},{:.1}) {:.1}x{:.1}", n.id, n.x, n.y, n.w, n.h); }
}
