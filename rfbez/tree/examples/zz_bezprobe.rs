fn d(a: (f64, f64), b: (f64, f64)) -> f64 { ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() }
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&args[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        flowmaid::model::Document::Flowchart(g) => g,
        flowmaid::model::Document::State(g) => g,
        _ => return,
    };
    let s = flowmaid::scene::scene(&g);
    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("scene");
    let sc = if mode == "route" {
        let pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        flowmaid::scene::route(&g, &pos)
    } else { s };
    if std::env::var("NODES").is_ok() { for n in &sc.nodes { println!("node {} ({:.1},{:.1}) {:.1}x{:.1}", n.id, n.x, n.y, n.w, n.h); } }
    for e in &sc.edges {
        if e.waypoints.is_empty() { continue; }
        let w0 = e.waypoints[0];
        let wn = *e.waypoints.last().unwrap();
        let d0 = d(w0, e.bezier[0]);
        let d1 = d(wn, e.bezier[3]);
        if d0 > 0.5 || d1 > 0.5 || args.get(3).is_some() {
            println!("{} -> {}: d0={:.2} d1={:.2} wp={:?} bez={:?} curve={:?}", e.from, e.to, d0, d1, e.waypoints, e.bezier, e.meta.curve);
        }
    }
    let svg = flowmaid::scene::to_svg(&sc);
    if let Some(p) = args.get(4) { std::fs::write(p, svg).unwrap(); }
}
