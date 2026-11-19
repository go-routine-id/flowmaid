// fuzz <dir...>: random drags (seeded) through route / route_partial / route_sized; finite, index-parallel, deterministic, to_svg ok
use flowmaid::model::Document;
use flowmaid::scene;
fn main() {
    let mut files: Vec<String> = Vec::new();
    for a in std::env::args().skip(1) {
        let mut v: Vec<String> = std::fs::read_dir(&a).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().map_or(false, |e| e == "mmd")).map(|p| p.to_string_lossy().to_string()).collect();
        v.sort();
        files.extend(v);
    }
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let (mut n, mut bad, mut hops) = (0, 0, 0);
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => continue,
        };
        if g.nodes.is_empty() || g.edges.len() > 120 {
            continue;
        }
        let st = scene::scene(&g);
        let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        for round in 0..6 {
            let mut c = base.clone();
            let moves = 1 + (rnd() * 3.0) as usize;
            for _ in 0..moves {
                let v = (rnd() * c.len() as f64) as usize % c.len();
                let amp = [1.0, 12.0, 60.0, 250.0, 0.0, 600.0][round];
                c[v].0 += (rnd() - 0.5) * 2.0 * amp;
                c[v].1 += (rnd() - 0.5) * 2.0 * amp;
            }
            if round == 4 {
                // coincident centres
                let v = (rnd() * c.len() as f64) as usize % c.len();
                let w = (rnd() * c.len() as f64) as usize % c.len();
                c[v] = c[w];
            }
            let sizes: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.w * (0.5 + rnd() * 2.0), n.h * (0.5 + rnd() * 2.0))).collect();
            for (name, sc) in [("route", scene::route(&g, &c)), ("partial", scene::route_partial(&g, &c, &st, &base)), ("sized", scene::route_sized(&g, &c, &sizes))] {
                n += 1;
                let ok = sc.edges.len() >= g.edges.len()
                    && sc.nodes.len() == g.nodes.len()
                    && sc.edges.iter().all(|e| e.bezier.iter().chain(e.waypoints.iter()).all(|p| p.0.is_finite() && p.1.is_finite()))
                    && sc.width.is_finite()
                    && sc.height.is_finite()
                    && g.edges.iter().enumerate().all(|(i, ge)| sc.edges[i].from == g.nodes[ge.from].id && sc.edges[i].to == g.nodes[ge.to].id);
                let svg = scene::to_svg(&sc);
                hops += svg.matches(" A ").count() / 2;
                let det = match name {
                    "route" => svg == scene::to_svg(&scene::route(&g, &c)),
                    "partial" => svg == scene::to_svg(&scene::route_partial(&g, &c, &st, &base)),
                    _ => svg == scene::to_svg(&scene::route_sized(&g, &c, &sizes)),
                };
                // a pick anywhere must not panic
                let _ = sc.hit_test(c[0].0 + 3.0, c[0].1 - 40.0, 4.0);
                let _ = sc.hits_in_rect(0.0, 0.0, sc.width, sc.height);
                if !ok || !det || svg.contains("NaN") || svg.contains("inf") {
                    bad += 1;
                    println!("BAD {f} {name} round {round}: ok={ok} det={det}");
                }
            }
        }
    }
    println!("fuzz: {n} scenes, {bad} bad, {hops} hops painted");
}
