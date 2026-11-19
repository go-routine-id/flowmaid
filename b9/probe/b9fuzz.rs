// TEMPORARY fuzz for B9 (not committed).
use flowmaid::model::Document;
fn finite(s: &flowmaid::scene::Scene) -> bool {
    s.nodes.iter().all(|n| n.x.is_finite() && n.y.is_finite())
        && s.edges.iter().all(|e| e.bezier.iter().chain(&e.waypoints).all(|p| p.0.is_finite() && p.1.is_finite()))
        && s.clusters.iter().all(|c| c.x.is_finite() && c.w.is_finite())
}
fn main() {
    let dir = std::env::args().nth(1).unwrap();
    let mut worst = (0.0f64, String::new());
    let mut n = 0;
    for f in std::fs::read_dir(&dir).unwrap() {
        let p = f.unwrap().path();
        let src = std::fs::read_to_string(&p).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g) | Document::State(g)) => g,
            _ => continue,
        };
        n += 1;
        let s = flowmaid::scene::scene(&g);
        assert!(finite(&s), "{p:?} static");
        assert_eq!(s.nodes.len(), g.nodes.len());
        let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = flowmaid::scene::route(&g, &at);
        assert!(finite(&r), "{p:?} routed");
        assert_eq!(r.edges.len(), s.edges.len(), "{p:?} edge count");
        let zero = flowmaid::scene::route_sized(&g, &at, &vec![(0.0, 0.0); at.len()]);
        assert!(finite(&zero), "{p:?} zero");
        for v in 0..g.nodes.len() {
            for (dx, dy) in [(37.0, 0.0), (0.0, -55.0), (120.0, 90.0)] {
                let mut m = at.clone();
                m[v].0 += dx;
                m[v].1 += dy;
                let t = std::time::Instant::now();
                let q = flowmaid::scene::route_partial(&g, &m, &s, &at);
                let ms = t.elapsed().as_secs_f64() * 1e3;
                if ms > worst.0 { worst = (ms, format!("{p:?} v{v}")); }
                assert!(finite(&q), "{p:?} partial {v}");
                assert_eq!(q.edges.len(), s.edges.len(), "{p:?} partial edges");
                assert_eq!(q.clusters.len(), s.clusters.len(), "{p:?} partial clusters");
            }
        }
    }
    println!("{n} graphs ok; worst route_partial {:.2} ms at {}", worst.0, worst.1);
}
