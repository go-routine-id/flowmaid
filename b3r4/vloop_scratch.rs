//! Scratch: back-edges (and any edge) touching a self-loop.
use crate::model::{Document, Graph};
use crate::scene::{edge_painted_polyline, route, scene, Scene};
type Pt = (f64, f64);
fn seg_cross(p: Pt, q: Pt, r: Pt, s: Pt) -> bool {
    let side = |a: Pt, b: Pt, c: Pt| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let (d1, d2) = (side(p, q, r), side(p, q, s));
    let (d3, d4) = (side(r, s, p), side(r, s, q));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}
fn load(path: &str) -> Option<Graph> {
    let src = std::fs::read_to_string(path).ok()?;
    match crate::parser::parse_document(&src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn check(name: &str, mode: &str, g: &Graph, sc: &Scene, out: &mut String) -> (usize, usize) {
    let polys: Vec<Vec<Pt>> = sc.edges.iter().map(edge_painted_polyline).collect();
    let (mut own, mut other) = (0, 0);
    for (li, l) in g.edges.iter().enumerate() {
        if l.from != l.to { continue; }
        let lp = &polys[li];
        for (ei, e) in g.edges.iter().enumerate() {
            if e.from == e.to { continue; }
            let p = &polys[ei];
            let hit = p.windows(2).any(|a| lp.windows(2).any(|b| seg_cross(a[0], a[1], b[0], b[1])));
            if hit {
                let back = sc.edges[ei].waypoints.len() > 2;
                let at_end = e.from == l.from || e.to == l.from;
                if at_end { own += 1 } else { other += 1 }
                out.push_str(&format!("{name} {mode} loop {} crossed by {}->{} lane={back} at_end={at_end}\n", g.nodes[l.from].id, g.nodes[e.from].id, g.nodes[e.to].id));
            }
        }
    }
    (own, other)
}
#[test]
fn vloop() {
    let list = std::env::var("VB3_LIST").unwrap();
    let outp = std::env::var("VB3_OUT").unwrap();
    let mut out = String::new();
    let mut t = [0usize; 4];
    for path in std::fs::read_to_string(&list).unwrap().lines() {
        let path = path.trim();
        let Some(g) = load(path) else { continue };
        if !g.edges.iter().any(|e| e.from == e.to) { continue; }
        let name = std::path::Path::new(path).file_stem().unwrap().to_string_lossy().to_string();
        let st = scene(&g);
        let c: Vec<Pt> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route(&g, &c);
        let a = check(&name, "static", &g, &st, &mut out);
        let b = check(&name, "routed", &g, &rt, &mut out);
        t[0] += a.0; t[1] += a.1; t[2] += b.0; t[3] += b.1;
    }
    out.push_str(&format!("TOTAL static own={} other={} routed own={} other={}\n", t[0], t[1], t[2], t[3]));
    std::fs::write(outp, out).unwrap();
}
