//! Interactive-contract probe (verifier, fix round of b12a2b-base).
//! Usage: contract <sigdir> <files...>
//! For each file x {TD,BT,LR,RL} x {blocks off, on}: scene twice (determinism),
//! index-parallel edges, route() at the scene centres and after three drags,
//! route_partial() for the same drags (determinism, timing), picking along the
//! drawn waypoints, to_mermaid round trip, compact fold determinism. Writes one
//! signature file per case so two builds can be diffed.
use flowmaid::model::{Document, Graph};
use flowmaid::quality::with_blocks;
use flowmaid::scene::{self, Scene, SceneEdge};
use std::time::Instant;

fn sig(sc: &Scene) -> String {
    let mut s = String::new();
    for n in &sc.nodes {
        s += &format!("N {}:{:?},{:?},{:?},{:?}\n", n.id, n.x, n.y, n.w, n.h);
    }
    for e in &sc.edges {
        s += &format!("E {}>{}:{:?}|{:?}\n", e.from, e.to, e.bezier, e.waypoints);
    }
    for c in &sc.clusters {
        s += &format!("C {}:{:?},{:?},{:?},{:?}\n", c.id, c.x, c.y, c.w, c.h);
    }
    s += &format!("{:?}x{:?}\n", sc.width, sc.height);
    s
}

fn poly(e: &SceneEdge) -> Vec<(f64, f64)> {
    if !e.waypoints.is_empty() {
        return e.waypoints.clone();
    }
    let b = e.bezier;
    (0..=32)
        .map(|k| {
            let t = k as f64 / 32.0;
            let u = 1.0 - t;
            (
                u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0,
                u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1,
            )
        })
        .collect()
}

fn dseg(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

fn dpoly(p: (f64, f64), pts: &[(f64, f64)]) -> f64 {
    pts.windows(2).map(|w| dseg(p, w[0], w[1])).fold(f64::INFINITY, f64::min)
}

fn check(g: &Graph, sc: &Scene, what: &str, out: &mut Vec<String>) {
    let fin = sc.width.is_finite()
        && sc.edges.iter().all(|e| {
            e.bezier.iter().chain(e.waypoints.iter()).all(|p| p.0.is_finite() && p.1.is_finite())
        });
    if !fin {
        out.push(format!("{what}: non-finite"));
    }
    if sc.nodes.len() != g.nodes.len() || sc.edges.len() < g.edges.len() {
        out.push(format!("{what}: counts"));
        return;
    }
    for (i, e) in g.edges.iter().enumerate() {
        let se = &sc.edges[i];
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
            out.push(format!("{what}: edge {i} not index-parallel"));
        }
    }
    for (i, e) in sc.edges.iter().enumerate() {
        // dense sampling: a host B-spline through the waypoints must not cut a
        // corner: no waypoint segment longer than 16 px that is diagonal
        for w in e.waypoints.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            if dx > 0.75 && dy > 0.75 && dx.hypot(dy) > 16.0 {
                out.push(format!("{what}: edge {i} {}>{} diagonal waypoint segment", e.from, e.to));
                break;
            }
        }
    }
    // picking along the drawn path: midpoints of every axis-aligned waypoint
    // segment longer than 8 px pick this edge or one drawn within 3 px there
    for (i, e) in sc.edges.iter().enumerate().take(g.edges.len()) {
        let p = poly(e);
        for w in p.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            if dx.hypot(dy) < 8.0 || (dx > 0.5 && dy > 0.5) {
                continue;
            }
            let m = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
            match sc.edge_at(m.0, m.1, 3.0) {
                Some(j) if j == i => {}
                Some(j) if j < sc.edges.len() && dpoly(m, &poly(&sc.edges[j])) <= 3.5 => {}
                Some(j) => out.push(format!("{what}: pick {i} {}>{} at {m:?} gives {j}", e.from, e.to)),
                None => {
                    if sc.node_at(m.0, m.1).is_none() && sc.hit_test(m.0, m.1, 3.0).is_none() {
                        out.push(format!("{what}: pick {i} {}>{} at {m:?} gives nothing", e.from, e.to));
                    }
                }
            }
        }
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let sigdir = a.next().unwrap();
    std::fs::create_dir_all(&sigdir).unwrap();
    let (mut worst_on, mut worst_off) = (0.0f64, 0.0f64);
    let (mut total, mut bad) = (0usize, 0usize);
    for f in a {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src0 = std::fs::read_to_string(&f).unwrap();
        for dir in ["TD", "BT", "LR", "RL"] {
            let src = flip(&src0, dir);
            let doc = flowmaid::parser::parse_document(&src);
            let flow = matches!(doc, Ok(Document::Flowchart(_)));
            let g = match doc {
                Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
                _ => continue,
            };
            for on in [false, true] {
                let what = format!("{name}-{dir}-{}", if on { "on" } else { "off" });
                let mut out = Vec::new();
                let sc = with_blocks(on, || scene::scene(&g));
                let sc2 = with_blocks(on, || scene::scene(&g));
                if sig(&sc) != sig(&sc2) || scene::to_svg(&sc) != scene::to_svg(&sc2) {
                    out.push(format!("{what}: scene not deterministic"));
                }
                check(&g, &sc, &format!("{what} scene"), &mut out);
                let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                let r0 = with_blocks(on, || scene::route(&g, &base));
                check(&g, &r0, &format!("{what} route0"), &mut out);
                let mut s = format!("{}--svg\n{}\n--route0\n{}", sig(&sc), scene::to_svg(&sc), sig(&r0));
                let n = g.nodes.len();
                for (k, d) in [(0usize, (37.0, 23.0)), (n / 2, (-41.0, 15.0)), (n.saturating_sub(1), (12.0, -60.0))] {
                    if k >= n {
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
                    if on { worst_on = worst_on.max(ms) } else { worst_off = worst_off.max(ms) }
                    let p2 = with_blocks(on, || scene::route_partial(&g, &c, &sc, &base));
                    if sig(&p) != sig(&p2) {
                        out.push(format!("{what}: route_partial not deterministic"));
                    }
                    check(&g, &p, &format!("{what} partial k{k}"), &mut out);
                    s += &format!("--route k{k}\n{}--partial k{k}\n{}", sig(&r), sig(&p));
                }
                let fo = flowmaid::fold::CompactOptions::for_extent(600.0);
                let c1 = with_blocks(on, || flowmaid::fold::render_compact(&g, &fo));
                let c2 = with_blocks(on, || flowmaid::fold::render_compact(&g, &fo));
                if c1 != c2 {
                    out.push(format!("{what}: compact not deterministic"));
                }
                s += &format!("--compact\n{c1}\n");
                std::fs::write(format!("{sigdir}/{what}.sig"), s).unwrap();
                if flow && !on {
                    let m = flowmaid::to_mermaid(&g);
                    match flowmaid::parser::parse(&m) {
                        Ok(g2) => {
                            if flowmaid::to_mermaid(&g2) != m {
                                out.push(format!("{what}: to_mermaid round trip not stable"));
                            }
                        }
                        Err(e) => out.push(format!("{what}: to_mermaid does not parse: {e}")),
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
    println!("checked {total}; with findings {bad}; worst route_partial off {worst_off:.2} ms, on {worst_on:.2} ms");
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
