use flowmaid::{parser::parse, scene::{scene, route}, quality::measure};
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let mode = std::env::args().nth(2).unwrap_or_default();
    let src = std::fs::read_to_string(&f).unwrap();
    let g = parse(&src).unwrap();
    let sc = scene(&g);
    let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let r = route(&g, &at);
    if mode == "route" {
        let k: usize = std::env::args().nth(3).unwrap().parse().unwrap();
        let mut w = 0.0;
        for _ in 0..k { let r = route(&g, &at); w += r.width; }
        println!("{w}");
        return;
    }
    if mode == "flow" {
        let pos = |id: &str| sc.nodes.iter().find(|n| n.id == id).map(|n| (n.x, n.y)).unwrap();
        let fwd = sc.edges.iter().take(g.edges.len()).filter(|e| e.from != e.to && pos(&e.to).0 > pos(&e.from).0 + 1.0).count();
        println!("{fwd}/{} forward (LR), size {:.0}x{:.0}", g.edges.len(), sc.width, sc.height);
        return;
    }
    if mode == "starts" {
        let n0 = &sc.nodes[0];
        println!("n0 {:.1},{:.1} {:.1}x{:.1}", n0.x, n0.y, n0.w, n0.h);
        for e in sc.edges.iter() { if e.from == n0.id { println!("{} start {:?} wps {}", e.to, (e.bezier[0].0.round(), e.bezier[0].1.round()), e.waypoints.len()); } }
        return;
    }
    if mode == "svg" { print!("{}", flowmaid::scene::to_svg(&r)); return; }
    if mode == "edges" {
        for want in std::env::args().skip(3) {
            for e in r.edges.iter().filter(|e| format!("{}->{}", e.from, e.to) == want) {
                println!("{want}: bez {:?}\n  wps {:?}", e.bezier, e.waypoints.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>());
            }
            for n in r.nodes.iter() { if want.contains(&format!(">{}", n.id)) || want.starts_with(&format!("{}-", n.id)) { println!("  node {} at {:.0},{:.0} {:.0}x{:.0}", n.id, n.x, n.y, n.w, n.h); } }
        }
        return;
    }
    for (lab, s) in [("static", &sc), ("routed", &r)] {
        let m = measure(&g, s);
        for d in &m.details { if d.contains("edge_node") || d.contains("chip") { println!("{lab} {d}"); } }
    }
}
