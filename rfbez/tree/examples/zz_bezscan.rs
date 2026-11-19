// For each file: scene() and route(); for edges with waypoints, no curve, not self-loop,
// report ends > 3px from path ends and whether bezier end lies on the end node's outline.
fn d(a: (f64, f64), b: (f64, f64)) -> f64 { ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() }
fn on_outline(sc: &flowmaid::scene::Scene, id: &str, p: (f64, f64)) -> String {
    if let Some(i) = sc.nodes.iter().position(|n| n.id == id) {
        let n = &sc.nodes[i];
        let (cx, cy) = (n.x, n.y);
        let v = (p.0 - cx, p.1 - cy);
        let l = (v.0 * v.0 + v.1 * v.1).sqrt().max(1e-9);
        let u = (v.0 / l, v.1 / l);
        let pin = (p.0 - u.0 * 1.0, p.1 - u.1 * 1.0);
        let pout = (p.0 + u.0 * 1.0, p.1 + u.1 * 1.0);
        let inside = |q: (f64, f64)| sc.node_at(q.0, q.1) == Some(i);
        // bbox check also
        let inbox = |q: (f64, f64)| (q.0 - cx).abs() <= n.w / 2.0 + 0.01 && (q.1 - cy).abs() <= n.h / 2.0 + 0.01;
        format!("node:in={} out={} inbox_in={} inbox_out={}", inside(pin), inside(pout), inbox(pin), inbox(pout))
    } else if let Some(c) = sc.clusters.iter().find(|c| c.id == id) {
        let on = (p.0 - c.x).abs() < 1.0 || (p.0 - c.x - c.w).abs() < 1.0 || (p.1 - c.y).abs() < 1.0 || (p.1 - c.y - c.h).abs() < 1.0;
        format!("cluster:on_border={}", on)
    } else { "?".into() }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    for path in &args[1..] {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(flowmaid::model::Document::Flowchart(g)) => g,
            Ok(flowmaid::model::Document::State(g)) => g,
            _ => continue,
        };
        if g.edges.len() > 400 { continue; }
        let s = flowmaid::scene::scene(&g);
        let pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = flowmaid::scene::route(&g, &pos);
        for (mode, sc) in [("scene", &s), ("route", &r)] {
            for e in &sc.edges {
                if e.waypoints.len() < 2 || e.from == e.to { continue; }
                if format!("{:?}", e.meta.curve) != "None" { continue; }
                let w0 = e.waypoints[0];
                let wn = *e.waypoints.last().unwrap();
                let d0 = d(w0, e.bezier[0]);
                let d1 = d(wn, e.bezier[3]);
                if d0 > 3.0 || d1 > 3.0 {
                    let o0 = if d0 > 3.0 { on_outline(sc, &e.from, e.bezier[0]) } else { "-".into() };
                    let o1 = if d1 > 3.0 { on_outline(sc, &e.to, e.bezier[3]) } else { "-".into() };
                    println!("{}\t{}\t{}->{}\td0={:.1}\td1={:.1}\t{}\t{}\tkind={:?}", path, mode, e.from, e.to, d0, d1, o0, o1, e.kind);
                }
            }
        }
    }
}
