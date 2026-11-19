fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let src = std::fs::read_to_string(&args[0]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
        _ => return,
    };
    let sc = flowmaid::scene::scene(&g);
    let pos: Vec<(f64,f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let rs = flowmaid::scene::route(&g, &pos);
    for (name, s) in [("scene", &sc), ("route", &rs)] {
        for e in &s.edges {
            if e.waypoints.is_empty() { continue; }
            let d = |p: (f64,f64), q: (f64,f64)| ((p.0-q.0).powi(2)+(p.1-q.1).powi(2)).sqrt();
            let a = e.waypoints[0]; let b = *e.waypoints.last().unwrap();
            let m = d(a, e.bezier[0]).max(d(b, e.bezier[3]));
            if m > 0.5 {
                println!("{name} {}->{} curve={:?} kind={:?} off={m:.1}\n  bezier={:?}\n  wps({})={:?}", e.from, e.to, e.meta.curve, e.kind, e.bezier, e.waypoints.len(), e.waypoints);
            }
        }
    }
    for n in &sc.nodes { if args.len() > 1 { println!("node {} ({:.1},{:.1}) {:.1}x{:.1} {:?}", n.id, n.x, n.y, n.w, n.h, n.shape); } }
}
