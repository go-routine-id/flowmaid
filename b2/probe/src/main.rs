use flowmaid::model::Document;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => continue,
        };
        let s = flowmaid::scene::scene(&g);
        let svg = flowmaid::render::render(&g);
        let vb = svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap().to_string();
        println!("{f}: scene {:.1}x{:.1} svg viewBox {vb}", s.width, s.height);
        let pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = flowmaid::scene::route(&g, &pos);
        println!("   route {:.1}x{:.1}", r.width, r.height);
        for c in &s.clusters {
            println!("   cluster {} x={:.1} y={:.1} w={:.1} h={:.1}", c.id, c.x, c.y, c.w, c.h);
        }
        if std::env::var("NODES").is_ok() {
            for n in &s.nodes { println!("   node {} ({:.1},{:.1}) {:.1}x{:.1}", n.id, n.x, n.y, n.w, n.h); }
            for e in &s.edges { println!("   edge {}->{} bez {:?} wps {}", e.from, e.to, e.bezier.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>(), e.waypoints.len()); }
        }
    }
}
