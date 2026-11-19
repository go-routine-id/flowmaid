//! Scratch: static vs routed back-edge distance.
use crate::model::{Document, Graph};
use crate::scene::{edge_polyline, route, scene};
type Pt = (f64, f64);
fn load(path: &str) -> Option<Graph> {
    let src = std::fs::read_to_string(path).ok()?;
    match crate::parser::parse_document(&src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn dist_poly(p: Pt, poly: &[Pt]) -> f64 {
    let mut best = f64::INFINITY;
    for w in poly.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (vx, vy) = (b.0 - a.0, b.1 - a.1);
        let l2 = vx * vx + vy * vy;
        let t = if l2 > 0.0 { (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        best = best.min((p.0 - a.0 - t * vx).hypot(p.1 - a.1 - t * vy));
    }
    best
}
#[test]
fn vside() {
    let list = std::env::var("VB3_LIST").unwrap();
    let outp = std::env::var("VB3_OUT").unwrap();
    let mut out = String::new();
    let (mut n, mut far) = (0, 0);
    for path in std::fs::read_to_string(&list).unwrap().lines() {
        let Some(g) = load(path.trim()) else { continue };
        let name = std::path::Path::new(path.trim()).file_stem().unwrap().to_string_lossy().to_string();
        let st = scene(&g);
        let c: Vec<Pt> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route(&g, &c);
        for (i, e) in g.edges.iter().enumerate() {
            if e.from == e.to || st.edges[i].waypoints.len() < 3 { continue; }
            // only lanes: static waypoints ortho with >= 3 pts
            let a = edge_polyline(&st.edges[i]);
            let b = edge_polyline(&rt.edges[i]);
            let d = a.iter().map(|&p| dist_poly(p, &b)).fold(0.0f64, f64::max)
                .max(b.iter().map(|&p| dist_poly(p, &a)).fold(0.0f64, f64::max));
            n += 1;
            if d > 80.0 {
                far += 1;
                out.push_str(&format!("{name} {}->{} {d:.0}\n", g.nodes[e.from].id, g.nodes[e.to].id));
            }
        }
    }
    out.push_str(&format!("TOTAL edges={n} far={far}\n"));
    std::fs::write(outp, out).unwrap();
}
