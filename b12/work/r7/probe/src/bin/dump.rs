//! dump <file.mmd> [route]: nodes, frames and edge corners of the scene
use flowmaid::Document;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&args[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!("not a flowchart"),
    };
    let mut sc = flowmaid::scene::scene(&g);
    if args.get(2).map(String::as_str) == Some("route") {
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        sc = flowmaid::scene::route(&g, &at);
    }
    for n in &sc.nodes {
        println!("node {:12} x {:7.1} y {:7.1} w {:6.1} h {:6.1}  [{:.1}..{:.1}] x [{:.1}..{:.1}]", n.id, n.x, n.y, n.w, n.h, n.x - n.w / 2.0, n.x + n.w / 2.0, n.y - n.h / 2.0, n.y + n.h / 2.0);
    }
    for c in &sc.clusters {
        println!("frame {:12} x {:7.1}..{:7.1} y {:7.1}..{:7.1}", c.id, c.x, c.x + c.w, c.y, c.y + c.h);
    }
    for e in &sc.edges {
        let pts: Vec<(f64, f64)> = if e.waypoints.is_empty() { e.bezier.to_vec() } else { e.waypoints.clone() };
        // corners only: drop points collinear with their neighbours
        let mut out: Vec<(f64, f64)> = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if i == 0 || i + 1 == pts.len() { out.push(p); continue; }
            let (a, b) = (pts[i - 1], pts[i + 1]);
            let cross = (p.0 - a.0) * (b.1 - p.1) - (p.1 - a.1) * (b.0 - p.0);
            if cross.abs() > 1e-6 { out.push(p); }
        }
        let s: Vec<String> = out.iter().map(|p| format!("({:.0},{:.0})", p.0, p.1)).collect();
        println!("edge {}->{} {} {}", e.from, e.to, if e.waypoints.is_empty() { "bez" } else { "wp" }, s.join(" "));
    }
}
