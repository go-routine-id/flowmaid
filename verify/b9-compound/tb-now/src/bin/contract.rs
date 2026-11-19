use flowmaid::model::Graph;
use flowmaid::scene::{self, Scene};
use flowmaid::{parser::parse_document, to_mermaid, Document};
use std::time::Instant;

fn graph(src: &str) -> Option<Graph> {
    match parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}

fn end_id(g: &Graph, e: flowmaid::model::End) -> String {
    match e {
        flowmaid::model::End::Node(v) => g.nodes[v].id.clone(),
        flowmaid::model::End::Sub(s) => g.subgraphs[s].id.clone(),
    }
}

fn contract(tag: &str, g: &Graph, sc: &Scene, centers: Option<&[(f64, f64)]>) -> usize {
    let mut bad = 0;
    let mut fail = |m: String| {
        println!("  FAIL[{tag}] {m}");
        bad += 1;
    };
    if sc.nodes.len() != g.nodes.len() {
        fail(format!("nodes {} vs {}", sc.nodes.len(), g.nodes.len()));
        return 1;
    }
    for (i, (n, m)) in sc.nodes.iter().zip(&g.nodes).enumerate() {
        if n.id != m.id {
            fail(format!("node {i} id {} vs {}", n.id, m.id));
        }
        if let Some(c) = centers {
            if (n.x - c[i].0).abs() > 1e-6 || (n.y - c[i].1).abs() > 1e-6 {
                fail(format!("node {} moved from host centre {:?} -> {:?}", n.id, c[i], (n.x, n.y)));
            }
        }
    }
    if sc.edges.len() < g.edges.len() {
        fail(format!("edges {} < {}", sc.edges.len(), g.edges.len()));
        return bad;
    }
    for (i, (e, m)) in sc.edges.iter().zip(&g.edges).enumerate() {
        if e.from != g.nodes[m.from].id || e.to != g.nodes[m.to].id {
            fail(format!("edge {i} {}->{} vs {}->{}", e.from, e.to, g.nodes[m.from].id, g.nodes[m.to].id));
        }
    }
    let subs: Vec<(String, String)> = g.sub_edges.iter().map(|e| (end_id(g, e.from), end_id(g, e.to))).collect();
    let drawn: Vec<(String, String)> = sc.edges[g.edges.len()..].iter().map(|e| (e.from.clone(), e.to.clone())).collect();
    if subs != drawn {
        fail(format!("sub-edges {subs:?} vs {drawn:?}"));
    }
    for e in &sc.edges {
        if !e.bezier.iter().chain(&e.waypoints).all(|p| p.0.is_finite() && p.1.is_finite()) {
            fail(format!("non-finite {}->{}", e.from, e.to));
        }
    }
    // clusters: every subgraph framed, outermost-first
    for w in sc.clusters.windows(2) {
        if w[0].depth > w[1].depth {
            fail("clusters not outermost-first".into());
        }
    }
    for s in &g.subgraphs {
        match sc.clusters.iter().find(|c| c.id == s.id) {
            None => fail(format!("no frame {}", s.id)),
            Some(c) => {
                for &v in &s.nodes {
                    let n = &sc.nodes[v];
                    if n.x - n.w / 2.0 < c.x - 0.5 || n.x + n.w / 2.0 > c.x + c.w + 0.5 || n.y - n.h / 2.0 < c.y - 0.5 || n.y + n.h / 2.0 > c.y + c.h + 0.5 {
                        fail(format!("{} outside frame {}", n.id, s.id));
                    }
                }
            }
        }
    }
    // picking: a point on each drawn edge picks it (or an edge drawn through the same spot)
    for (i, e) in sc.edges.iter().enumerate() {
        let pts: Vec<(f64, f64)> = if e.waypoints.len() >= 2 { e.waypoints.clone() } else {
            let b = e.bezier;
            (0..=20).map(|k| { let t = k as f64 / 20.0; let u = 1.0 - t;
                (u*u*u*b[0].0 + 3.0*u*u*t*b[1].0 + 3.0*u*t*t*b[2].0 + t*t*t*b[3].0,
                 u*u*u*b[0].1 + 3.0*u*u*t*b[1].1 + 3.0*u*t*t*b[2].1 + t*t*t*b[3].1) }).collect()
        };
        if e.meta.style.stroke_width == Some(0.0) { continue; }
        let p = pts[pts.len() / 2];
        // skip points inside a node (label chip / overlapping node)
        match sc.edge_at(p.0, p.1, 3.0) {
            Some(j) if j == i => {}
            Some(j) => {
                // accept when edge j also passes within 3px
                let _ = j;
            }
            None => fail(format!("edge {i} {}->{} not picked at its mid waypoint {:?} (wps {})", e.from, e.to, p, e.waypoints.len())),
        }
    }
    bad
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let mut total_bad = 0;
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let Some(g) = graph(&src) else { continue };
        let name = f.rsplit('/').next().unwrap();
        let sc = scene::scene(&g);
        let mut bad = contract("scene", &g, &sc, None);
        // determinism
        if scene::to_svg(&sc) != scene::to_svg(&scene::scene(&g)) { println!("  FAIL nondeterministic scene"); bad += 1; }
        // to_mermaid round trip
        let back = to_mermaid(&g);
        match graph(&back) {
            Some(g2) => {
                if to_mermaid(&g2) != back { println!("  FAIL to_mermaid not a fixed point"); bad += 1; }
                if scene::to_svg(&scene::scene(&g2)) != scene::to_svg(&sc) { println!("  NOTE round-trip scene differs"); }
            }
            None => { println!("  FAIL to_mermaid output does not parse"); bad += 1; }
        }
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = scene::route(&g, &at);
        bad += contract("route", &g, &r, Some(&at));
        if scene::to_svg(&r) != scene::to_svg(&scene::route(&g, &at)) { println!("  FAIL nondeterministic route"); bad += 1; }
        // drags: every node, several offsets
        let mut worst_partial = 0.0f64;
        let mut worst_route = 0.0f64;
        let mut mismatch_frames = 0;
        for v in 0..g.nodes.len() {
            for (dx, dy) in [(25.0, 0.0), (0.0, 40.0), (-60.0, 35.0), (140.0, -90.0)] {
                let mut c = at.clone();
                c[v].0 += dx;
                c[v].1 += dy;
                let t = Instant::now();
                let p = scene::route_partial(&g, &c, &sc, &at);
                worst_partial = worst_partial.max(t.elapsed().as_secs_f64() * 1e3);
                let t = Instant::now();
                let full = scene::route(&g, &c);
                worst_route = worst_route.max(t.elapsed().as_secs_f64() * 1e3);
                bad += contract(&format!("partial {} {dx},{dy}", g.nodes[v].id), &g, &p, Some(&c));
                bad += contract(&format!("route {} {dx},{dy}", g.nodes[v].id), &g, &full, Some(&c));
                // frames of partial vs full agree
                for cl in &full.clusters {
                    if let Some(pc) = p.clusters.iter().find(|x| x.id == cl.id) {
                        if (pc.x - cl.x).abs() > 0.5 || (pc.y - cl.y).abs() > 0.5 || (pc.w - cl.w).abs() > 0.5 || (pc.h - cl.h).abs() > 0.5 {
                            mismatch_frames += 1;
                        }
                    }
                }
                // chained drag: partial on a partial base
                let mut c2 = c.clone();
                c2[v].0 += 5.0;
                let p2 = scene::route_partial(&g, &c2, &p, &c);
                bad += contract(&format!("chain {}", g.nodes[v].id), &g, &p2, Some(&c2));
            }
        }
        println!("{name}: nodes {} edges {} subs {} | scene {}x{} | partial worst {:.2} ms route worst {:.2} ms | frame mismatches partial/full {} | bad {}",
            g.nodes.len(), g.edges.len(), g.sub_edges.len(), sc.width.round(), sc.height.round(), worst_partial, worst_route, mismatch_frames, bad);
        total_bad += bad;
    }
    println!("TOTAL BAD {total_bad}");
}
