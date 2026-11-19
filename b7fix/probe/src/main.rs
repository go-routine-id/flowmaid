use flowmaid::model::Document;
use flowmaid::scene::{self, SceneEdge, Scene};
type P = (f64, f64);
fn show(tag: &str, sc: &Scene, node: &str) {
    for (i, e) in sc.edges.iter().enumerate() {
        if e.from != node && e.to != node && node != "*" { continue; }
        let (a, b) = match (e.waypoints.first(), e.waypoints.last()) {
            (Some(&a), Some(&b)) if e.waypoints.len() >= 2 => (a, b),
            _ => (e.bezier[0], e.bezier[3]),
        };
        let lab = e.label.as_ref().map(|l| format!("{} @({:.1},{:.1}) w{:.1}", l.0, (l.1).0, (l.1).1, l.2)).unwrap_or_default();
        println!("{tag} #{i} {}->{} start ({:.1},{:.1}) end ({:.1},{:.1}) wp{} bez {:?} [{lab}]", e.from, e.to, a.0, a.1, b.0, b.1, e.waypoints.len(),
          e.bezier.iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect::<Vec<_>>());
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&args[1]).unwrap();
    let node = args.get(2).cloned().unwrap_or("*".into());
    let drag = args.get(3).cloned();
    let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => panic!() };
    let base = scene::scene(&g);
    let at: Vec<P> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
    show("scene", &base, &node);
    for c in &base.clusters { println!("cluster {} x{:.1} y{:.1} w{:.1} h{:.1} bottom {:.1}", c.id, c.x, c.y, c.w, c.h, c.y + c.h); }
    for n in &base.nodes { println!("node {} ({:.1},{:.1}) {:.1}x{:.1}", n.id, n.x, n.y, n.w, n.h); }
    let r = scene::route(&g, &at);
    show("route", &r, &node);
    if let Some(d) = drag {
        let v = g.node_index(&d).unwrap();
        let mut c = at.clone(); c[v].0 += 1.0;
        let p = scene::route_partial(&g, &c, &base, &at);
        show("drag ", &p, &node);
    }
}
