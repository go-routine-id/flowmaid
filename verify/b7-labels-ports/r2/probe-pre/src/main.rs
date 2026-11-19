use flowmaid::model::{Document, Graph};
use flowmaid::scene;
fn graph_of(src: &str) -> Graph {
    match flowmaid::parser::parse_document(src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => panic!() }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let drag = &a[2]; let from = &a[3]; let to = &a[4];
    let dx: f64 = a.get(5).map(|s| s.parse().unwrap()).unwrap_or(1.0);
    let g = graph_of(&src);
    let sc = scene::scene(&g);
    let at: Vec<(f64,f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let v = g.nodes.iter().position(|n| &n.id == drag).unwrap();
    let mut c = at.clone(); c[v].0 += dx;
    let p = scene::route_partial(&g, &c, &sc, &at);
    let r = scene::route(&g, &c);
    let r0 = scene::route(&g, &at);
    for i in 0..g.edges.len() {
        let e = &sc.edges[i];
        if &e.from != from || &e.to != to { continue; }
        let show = |nm: &str, s: &scene::Scene| { let e = &s.edges[i];
            let w: Vec<String> = e.waypoints.iter().map(|p| format!("({:.0},{:.0})", p.0, p.1)).collect();
            println!("  {nm}: bez {:?} n_wp {} wp_first/last {:?} {:?} label {:?}", e.bezier.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>(), e.waypoints.len(), w.first(), w.last(), e.label.as_ref().map(|l| (l.1 .0.round(), l.1 .1.round())));
        };
        println!("edge {i} {}->{}", e.from, e.to);
        show("scene", &sc); show("route(at)", &r0); show("partial", &p); show("route(c)", &r);
    }
}
