use flowmaid::model::{Document, EdgeKind};
use flowmaid::scene::scene;
fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    // elbow legs (scene coords) given as p0x p0y cornerx cornery p3x p3y
    let a: Vec<f64> = std::env::args().skip(2).map(|s| s.parse().unwrap()).collect();
    let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::State(g) | Document::Flowchart(g) => g, _ => panic!() };
    let sc = scene(&g);
    for (i, n) in g.nodes.iter().enumerate() { println!("node {i} {} {:?} at ({:.1},{:.1}) {:.1}x{:.1}", n.id, n.shape, sc.nodes[i].x, sc.nodes[i].y, sc.nodes[i].w, sc.nodes[i].h); }
    let side = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let cross = |p: (f64, f64), q: (f64, f64), u: (f64, f64), v: (f64, f64)| side(p, q, u) * side(p, q, v) < 0.0 && side(u, v, p) * side(u, v, q) < 0.0;
    for (ei, e) in g.edges.iter().enumerate() {
        let u = (sc.nodes[e.from].x, sc.nodes[e.from].y); let v = (sc.nodes[e.to].x, sc.nodes[e.to].y);
        let mut hit = String::new();
        if a.len() == 6 && e.from != e.to && !matches!(e.kind, EdgeKind::Invisible) {
            let legs = [((a[0], a[1]), (a[2], a[3])), ((a[2], a[3]), (a[4], a[5]))];
            for (k, (p, q)) in legs.iter().enumerate() { if cross(*p, *q, u, v) { hit += &format!(" CROSSES leg{k}"); } }
        }
        println!("edge {ei} {} -> {} kind {:?}{hit}", g.nodes[e.from].id, g.nodes[e.to].id, e.kind);
    }
    for s in &g.subgraphs { println!("subgraph {} nodes {:?}", s.id, s.nodes); }
}
