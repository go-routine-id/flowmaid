//! Interactive-contract probe: scene / route / route_partial / picking /
//! determinism / to_mermaid round trip, with the block layout switched on
//! and off, over the given inputs drawn in four directions.
//! Usage: contract <sigdir> <files...>   (writes <sigdir>/<name>-<dir>-<on|off>.sig)
use flowmaid::model::{Document, Graph};
use flowmaid::quality::with_blocks;
use flowmaid::scene::{self, Scene};
use std::time::Instant;

fn sig(sc: &Scene) -> String {
    let mut s = String::new();
    for n in &sc.nodes {
        s += &format!("N {}:{:.3},{:.3},{:.3},{:.3}\n", n.id, n.x, n.y, n.w, n.h);
    }
    for e in &sc.edges {
        s += &format!("E {}>{}:{:?}|{:?}\n", e.from, e.to, e.bezier, e.waypoints);
    }
    for c in &sc.clusters {
        s += &format!("C {}:{:.3},{:.3},{:.3},{:.3}\n", c.id, c.x, c.y, c.w, c.h);
    }
    s += &format!("{:.3}x{:.3}\n", sc.width, sc.height);
    s
}

fn finite(sc: &Scene) -> bool {
    sc.width.is_finite()
        && sc.height.is_finite()
        && sc.nodes.iter().all(|n| n.x.is_finite() && n.y.is_finite())
        && sc.edges.iter().all(|e| {
            e.bezier.iter().all(|p| p.0.is_finite() && p.1.is_finite())
                && e.waypoints.iter().all(|p| p.0.is_finite() && p.1.is_finite())
        })
}

fn poly(e: &flowmaid::scene::SceneEdge) -> Vec<(f64, f64)> {
    if !e.waypoints.is_empty() {
        return e.waypoints.clone();
    }
    let b = e.bezier;
    (0..=24)
        .map(|k| {
            let t = k as f64 / 24.0;
            let u = 1.0 - t;
            (
                u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0,
                u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1,
            )
        })
        .collect()
}

fn dist_seg(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    let q = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt()
}

fn dist_poly(p: (f64, f64), pts: &[(f64, f64)]) -> f64 {
    pts.windows(2).map(|w| dist_seg(p, w[0], w[1])).fold(f64::INFINITY, f64::min)
}

fn check(g: &Graph, sc: &Scene, what: &str, out: &mut Vec<String>) {
    if !finite(sc) {
        out.push(format!("{what}: non-finite"));
    }
    if sc.nodes.len() != g.nodes.len() || sc.edges.len() < g.edges.len() {
        out.push(format!("{what}: counts {} {} vs {} {}", sc.nodes.len(), sc.edges.len(), g.nodes.len(), g.edges.len()));
        return;
    }
    for (i, e) in g.edges.iter().enumerate() {
        let se = &sc.edges[i];
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
            out.push(format!("{what}: edge {i} not index-parallel"));
        }
    }
    // Long diagonal stretches in waypoints (a host's spline would cut
    // corners): any segment longer than 16 px that is neither horizontal
    // nor vertical.
    for (i, e) in sc.edges.iter().enumerate() {
        for w in e.waypoints.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            if dx > 0.75 && dy > 0.75 && (dx * dx + dy * dy).sqrt() > 16.0 {
                out.push(format!("{what}: edge {i} {}>{} diagonal waypoint segment {:?}->{:?}", e.from, e.to, w[0], w[1]));
                break;
            }
        }
    }
    // Picking: the middle of each edge's longest stretch picks that edge
    // or another whose path passes there.
    for (i, e) in sc.edges.iter().enumerate().take(g.edges.len()) {
        let p = poly(e);
        if p.len() < 2 {
            continue;
        }
        let w = p
            .windows(2)
            .max_by(|a, b| {
                let la = (a[1].0 - a[0].0).hypot(a[1].1 - a[0].1);
                let lb = (b[1].0 - b[0].0).hypot(b[1].1 - b[0].1);
                la.total_cmp(&lb)
            })
            .unwrap();
        let m = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
        match sc.edge_at(m.0, m.1, 2.0) {
            Some(j) if j == i => {}
            Some(j) => {
                if dist_poly(m, &poly(&sc.edges[j])) > 4.0 {
                    out.push(format!("{what}: edge {i} mid {m:?} picks {j} which is not there"));
                }
            }
            None => {
                // A node or label may cover it; hit_test tells.
                if sc.node_at(m.0, m.1).is_none() {
                    out.push(format!("{what}: edge {i} {}>{} mid {m:?} picks nothing", e.from, e.to));
                }
            }
        }
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let sigdir = a.next().unwrap();
    std::fs::create_dir_all(&sigdir).unwrap();
    let mut worst_partial = 0.0f64;
    let mut total = 0usize;
    let mut bad = 0usize;
    for f in a {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src0 = std::fs::read_to_string(&f).unwrap();
        for dir in ["TD", "BT", "LR", "RL"] {
            let src = flip(&src0, dir);
            let g = match flowmaid::parser::parse_document(&src) {
                Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
                _ => continue,
            };
            let flow = matches!(flowmaid::parser::parse_document(&src), Ok(Document::Flowchart(_)));
            for on in [false, true] {
                let what = format!("{name}-{dir}-{}", if on { "on" } else { "off" });
                let mut out = Vec::new();
                let sc = with_blocks(on, || scene::scene(&g));
                let sc2 = with_blocks(on, || scene::scene(&g));
                if sig(&sc) != sig(&sc2) {
                    out.push(format!("{what}: scene not deterministic"));
                }
                if scene::to_svg(&sc) != scene::to_svg(&sc2) {
                    out.push(format!("{what}: svg not deterministic"));
                }
                check(&g, &sc, &format!("{what} scene"), &mut out);
                let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                // route() at the scene's own centres
                let r0 = with_blocks(on, || scene::route(&g, &base));
                check(&g, &r0, &format!("{what} route0"), &mut out);
                let mut s = sig(&sc);
                s += "--route0\n";
                s += &sig(&r0);
                // drags of three nodes
                for (k, d) in [(0usize, (37.0, 23.0)), (g.nodes.len() / 2, (-41.0, 15.0)), (g.nodes.len().saturating_sub(1), (12.0, -60.0))] {
                    if k >= g.nodes.len() {
                        continue;
                    }
                    let mut c = base.clone();
                    c[k].0 += d.0;
                    c[k].1 += d.1;
                    let r = with_blocks(on, || scene::route(&g, &c));
                    let r2 = with_blocks(on, || scene::route(&g, &c));
                    if sig(&r) != sig(&r2) {
                        out.push(format!("{what}: route not deterministic"));
                    }
                    check(&g, &r, &format!("{what} route k{k}"), &mut out);
                    let t = Instant::now();
                    let p = with_blocks(on, || scene::route_partial(&g, &c, &sc, &base));
                    let ms = t.elapsed().as_secs_f64() * 1e3;
                    worst_partial = worst_partial.max(ms);
                    let p2 = with_blocks(on, || scene::route_partial(&g, &c, &sc, &base));
                    if sig(&p) != sig(&p2) {
                        out.push(format!("{what}: route_partial not deterministic"));
                    }
                    check(&g, &p, &format!("{what} partial k{k}"), &mut out);
                    s += &format!("--route k{k}\n{}--partial k{k}\n{}", sig(&r), sig(&p));
                }
                std::fs::write(format!("{sigdir}/{what}.sig"), s).unwrap();
                if flow && !on {
                    let m = flowmaid::to_mermaid(&g);
                    match flowmaid::parser::parse(&m) {
                        Ok(g2) => {
                            if flowmaid::to_mermaid(&g2) != m {
                                out.push(format!("{what}: to_mermaid round trip not stable"));
                            }
                        }
                        Err(e) => out.push(format!("{what}: to_mermaid output does not parse: {e}")),
                    }
                }
                total += 1;
                if !out.is_empty() {
                    bad += 1;
                }
                for l in out {
                    println!("{l}");
                }
            }
        }
    }
    println!("checked {total} (file, dir, mode); with findings {bad}; worst route_partial {worst_partial:.2} ms");
}

fn flip(src: &str, dir: &str) -> String {
    let mut out = String::new();
    let mut done = false;
    for line in src.lines() {
        let t = line.trim_start();
        if !done && (t.starts_with("flowchart ") || t.starts_with("graph ")) {
            let kw = if t.starts_with("graph") { "graph" } else { "flowchart" };
            out += &format!("{kw} {dir}\n");
            done = true;
            continue;
        }
        if !done && t.starts_with("direction ") {
            // top-level direction of a state diagram
            let d = if dir == "TD" { "TB" } else { dir };
            out += &format!("direction {d}\n");
            done = true;
            continue;
        }
        out += line;
        out += "\n";
    }
    out
}
