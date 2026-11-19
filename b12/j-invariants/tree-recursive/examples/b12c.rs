use flowmaid::model::{Document, End};
use flowmaid::scene;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g) | Document::State(g)) => g,
            _ => continue,
        };
        let s = scene::scene(&g);
        let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = scene::route(&g, &at);
        println!("{f}: nodes {} edges g={} sub={} scene={} route={} clusters scene={} route={}", g.nodes.len(), g.edges.len(), g.sub_edges.len(), s.edges.len(), r.edges.len(), s.clusters.len(), r.clusters.len());
        for (i, e) in g.edges.iter().enumerate() {
            let se = &s.edges[i];
            if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
                println!("  edge {i}: graph {}->{} scene {}->{}", g.nodes[e.from].id, g.nodes[e.to].id, se.from, se.to);
            }
        }
        let name = |e: &End| match e { End::Node(v) => g.nodes[*v].id.clone(), End::Sub(s) => g.subgraphs[*s].id.clone() };
        for (j, se) in g.sub_edges.iter().enumerate() {
            let a = s.edges.get(g.edges.len() + j).map(|e| format!("{}->{}", e.from, e.to));
            let b = r.edges.get(g.edges.len() + j).map(|e| format!("{}->{}", e.from, e.to));
            let w = format!("{}->{}", name(&se.from), name(&se.to));
            if a.as_deref() != Some(&w) || b.as_deref() != Some(&w) {
                println!("  sub {j}: graph {w} scene {a:?} route {b:?}");
            }
        }
        for (a, b) in s.clusters.iter().zip(&r.clusters) {
            if a.id != b.id || a.depth != b.depth { println!("  cluster order: scene {} d{} route {} d{}", a.id, a.depth, b.id, b.depth); }
        }
        for w in s.clusters.windows(2) { if w[0].depth > w[1].depth { println!("  scene clusters not outermost first: {} d{} before {} d{}", w[0].id, w[0].depth, w[1].id, w[1].depth); } }
        for e in &s.edges { if e.bezier.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) || e.waypoints.iter().any(|p| !p.0.is_finite()||!p.1.is_finite()) { println!("  non-finite edge {}->{}", e.from, e.to); } }
    }
}
