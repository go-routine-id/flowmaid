use std::io::BufRead;
fn axis(w: &[(f64,f64)]) -> bool {
    w.windows(2).all(|p| (p[0].0 - p[1].0).abs() < 0.01 || (p[0].1 - p[1].1).abs() < 0.01)
}
fn main() {
    for line in std::io::stdin().lock().lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let Ok(doc) = flowmaid::parser::parse_document(&src) else { continue };
        let g = match doc {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => continue,
        };
        let Ok((sc0, rs0)) = std::panic::catch_unwind(|| { let sc = flowmaid::scene::scene(&g); let pos: Vec<(f64,f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect(); let rs = flowmaid::scene::route(&g, &pos); (sc, rs) }) else { continue };
        for (mode, sc) in [("scene", &sc0), ("route", &rs0)] {
        let n = sc.edges.len();
        if n > 400 { continue; }
        let d = |p: (f64,f64), q: (f64,f64)| ((p.0-q.0).powi(2)+(p.1-q.1).powi(2)).sqrt();
        for e in &sc.edges {
            if e.waypoints.is_empty() || e.meta.curve.is_some() || matches!(e.kind, flowmaid::model::EdgeKind::Invisible) || e.from == e.to { continue; }
            let a = e.waypoints[0]; let b = *e.waypoints.last().unwrap();
            let m = d(a, e.bezier[0]).max(d(b, e.bezier[3]));
            if m > 3.0 {
                println!("OFF {mode} n={n} axis={} wps={} off={m:.1} {path} {}->{}", axis(&e.waypoints), e.waypoints.len(), e.from, e.to);
            }
        }
        }
    }
}
