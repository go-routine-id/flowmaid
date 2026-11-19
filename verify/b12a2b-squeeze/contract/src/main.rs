use flowmaid::model::{Document, Graph};
use flowmaid::scene::{self, Scene};
use std::time::Instant;

fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}

fn centres(sc: &Scene) -> Vec<(f64, f64)> {
    sc.nodes.iter().map(|n| (n.x, n.y)).collect()
}

fn check(name: &str, g: &Graph, sc: &Scene, what: &str) -> usize {
    let mut bad = 0;
    if sc.edges.len() < g.edges.len() {
        println!("{name} {what}: scene edges {} < graph edges {}", sc.edges.len(), g.edges.len());
        return 1;
    }
    for (i, e) in g.edges.iter().enumerate() {
        let se = &sc.edges[i];
        let (f, t) = (&g.nodes[e.from].id, &g.nodes[e.to].id);
        if &se.from != f || &se.to != t {
            // sub-edges may anchor on clusters; only report node mismatch
            println!("{name} {what}: edge {i} {}->{} vs graph {}->{}", se.from, se.to, f, t);
            bad += 1;
        }
        let w = &se.waypoints;
        if w.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) || se.bezier.iter().any(|p| !p.0.is_finite()) {
            println!("{name} {what}: edge {i} non-finite");
            bad += 1;
        }
        // orthogonal path delivered densely: a sharp 90 deg corner between two long legs with no rounding points
        if w.len() >= 3 {
            for k in 1..w.len() - 1 {
                let a = (w[k].0 - w[k - 1].0, w[k].1 - w[k - 1].1);
                let b = (w[k + 1].0 - w[k].0, w[k + 1].1 - w[k].1);
                let la = (a.0 * a.0 + a.1 * a.1).sqrt();
                let lb = (b.0 * b.0 + b.1 * b.1).sqrt();
                if la > 30.0 && lb > 30.0 {
                    let cos = (a.0 * b.0 + a.1 * b.1) / (la * lb);
                    if cos.abs() < 0.2 {
                        println!("{name} {what}: edge {i} sharp corner at {:?} (legs {la:.0}/{lb:.0})", w[k]);
                        bad += 1;
                    }
                }
            }
        }
    }
    // pick == render: a point on the middle of each edge's waypoints/bezier picks some edge (ideally itself)
    let mut miss = 0;
    let mut other = 0;
    for (i, se) in sc.edges.iter().enumerate().take(g.edges.len()) {
        let p = if se.waypoints.len() >= 2 {
            let w = &se.waypoints;
            let m = w.len() / 2;
            ((w[m - 1].0 + w[m].0) / 2.0, (w[m - 1].1 + w[m].1) / 2.0)
        } else {
            let b = se.bezier;
            let t = 0.5f64;
            let u = 1.0 - t;
            (
                u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0,
                u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1,
            )
        };
        match sc.edge_at(p.0, p.1, 4.0) {
            None => {
                miss += 1;
                let near = (0..20).map(|k| sc.edge_at(p.0, p.1, k as f64)).position(|h| h.is_some());
                println!("  miss edge {i} {}->{} at ({:.1},{:.1}) wp {} picked at tol {:?}", se.from, se.to, p.0, p.1, se.waypoints.len(), near);
            }
            Some(j) if j != i => other += 1,
            _ => {}
        }
    }
    if miss > 0 {
        println!("{name} {what}: edge_at missed {miss} of {} midpoints", g.edges.len());
    }
    println!("{name} {what}: edges {} (+{} sub), pick other {other}, miss {miss}, bad {bad}", g.edges.len(), sc.edges.len() - g.edges.len());
    bad + miss
}

fn main() {
    let mut total = 0;
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().to_string();
        let Some(g) = graph_of(&src) else { println!("{name}: not a flowchart/state"); continue };
        let s1 = scene::scene(&g);
        let s2 = scene::scene(&g);
        let svg1 = scene::to_svg(&s1);
        let svg2 = scene::to_svg(&s2);
        if svg1 != svg2 { println!("{name}: scene SVG not deterministic"); total += 1; }
        total += check(&name, &g, &s1, "scene");
        let c0 = centres(&s1);
        let r0 = scene::route(&g, &c0);
        total += check(&name, &g, &r0, "route@scene");
        // move each of up to 3 nodes by 37/23 px and route + route_partial
        for v in [0usize, g.nodes.len() / 2, g.nodes.len().saturating_sub(1)] {
            if v >= g.nodes.len() { continue; }
            let mut c = c0.clone();
            c[v].0 += 37.0;
            c[v].1 += 23.0;
            let t = Instant::now();
            let r = scene::route(&g, &c);
            let rt = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let p = scene::route_partial(&g, &c, &r0, &c0);
            let pt = t.elapsed().as_secs_f64() * 1e3;
            let p2 = scene::route_partial(&g, &c, &r0, &c0);
            if scene::to_svg(&p) != scene::to_svg(&p2) { println!("{name}: route_partial not deterministic"); total += 1; }
            total += check(&name, &g, &r, &format!("route mv{v}"));
            total += check(&name, &g, &p, &format!("partial mv{v}"));
            println!("{name} mv{v}: route {rt:.2} ms, partial {pt:.2} ms");
        }
        // to_mermaid round trip
        let m1 = flowmaid::to_mermaid(&g);
        let g2 = graph_of(&m1).expect("reparse");
        let m2 = flowmaid::to_mermaid(&g2);
        if m1 != m2 { println!("{name}: to_mermaid round trip differs"); total += 1; }
    }
    println!("TOTAL problems {total}");
}
