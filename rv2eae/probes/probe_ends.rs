use std::io::BufRead;
fn off(sc: &flowmaid::scene::Scene) -> Vec<(String, String, f64)> {
    let d = |p: (f64,f64), q: (f64,f64)| ((p.0-q.0).powi(2)+(p.1-q.1).powi(2)).sqrt();
    sc.edges.iter().filter(|e| !e.waypoints.is_empty()).filter_map(|e| {
        let a = e.waypoints[0]; let b = *e.waypoints.last().unwrap();
        let m = d(a, e.bezier[0]).max(d(b, e.bezier[3]));
        (m > 0.5).then(|| (e.from.clone(), e.to.clone(), m))
    }).collect()
}
fn main() {
    let (mut files, mut edges, mut bad_s, mut bad_r) = (0, 0, 0, 0);
    for line in std::io::stdin().lock().lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let Ok(doc) = flowmaid::parser::parse_document(&src) else { continue };
        let g = match doc {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => continue,
        };
        let r = std::panic::catch_unwind(|| {
            let sc = flowmaid::scene::scene(&g);
            let pos: Vec<(f64,f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rs = flowmaid::scene::route(&g, &pos);
            (sc.edges.len(), off(&sc), off(&rs))
        });
        let Ok((n, s, r)) = r else { println!("PANIC {path}"); continue };
        files += 1; edges += n; bad_s += s.len(); bad_r += r.len();
        for (f, t, m) in s.iter().take(3) { println!("SCENE {path} {f}->{t} {m:.1}"); }
        for (f, t, m) in r.iter().take(3) { println!("ROUTE {path} {f}->{t} {m:.1}"); }
    }
    println!("files={files} edges={edges} scene_off={bad_s} route_off={bad_r}");
}
