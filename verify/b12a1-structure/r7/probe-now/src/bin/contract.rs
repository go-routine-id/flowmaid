//! Interactive-contract probe (scratch, outside the repo). MODE=on runs
//! everything inside `quality::with_blocks(true, ..)`.
//! Per input: index-parallel edges, waypoint corner sampling, picking
//! along the drawn waypoints, determinism, to_mermaid round trip, for
//! scene(), route() at the scene's centres, and route_partial() after
//! moving a few nodes.
use flowmaid::model::Document;
use flowmaid::scene::{self, Hit, Scene};

fn sig(sc: &Scene) -> String {
    let mut s = String::new();
    for n in &sc.nodes {
        s += &format!("{}:{:.4},{:.4},{:.4},{:.4};", n.id, n.x, n.y, n.w, n.h);
    }
    for e in &sc.edges {
        s += &format!("{}>{}:{:?}|{:?};", e.from, e.to, e.bezier, e.waypoints);
    }
    for c in &sc.clusters {
        s += &format!("{}:{:.4},{:.4},{:.4},{:.4};", c.id, c.x, c.y, c.w, c.h);
    }
    s
}

struct Stats {
    parallel_bad: usize,
    sharp: usize,
    corners: usize,
    pick_pts: usize,
    pick_none: usize,
    pick_node: usize,
    nonfinite: usize,
}

fn check(g: &flowmaid::model::Graph, sc: &Scene, st: &mut Stats, tag: &str, name: &str) {
    // Index-parallel: the first g.edges.len() scene edges are g.edges.
    if sc.edges.len() < g.edges.len() {
        st.parallel_bad += 1;
        eprintln!("{name} {tag}: fewer scene edges {} < {}", sc.edges.len(), g.edges.len());
    }
    for (i, e) in g.edges.iter().enumerate() {
        let Some(se) = sc.edges.get(i) else { break };
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
            st.parallel_bad += 1;
            eprintln!("{name} {tag}: edge {i} {}>{} vs {}>{}", g.nodes[e.from].id, g.nodes[e.to].id, se.from, se.to);
        }
    }
    for (ei, e) in sc.edges.iter().enumerate() {
        let w = &e.waypoints;
        if w.iter().chain(e.bezier.iter()).any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            st.nonfinite += 1;
        }
        // Sharp corners: a turn of more than 60 deg at a waypoint whose
        // two legs are both longer than 12 px (a corner nobody rounded).
        for k in 1..w.len().saturating_sub(1) {
            let (a, b, c) = (w[k - 1], w[k], w[k + 1]);
            let (ux, uy) = (b.0 - a.0, b.1 - a.1);
            let (vx, vy) = (c.0 - b.0, c.1 - b.1);
            let (lu, lv) = ((ux * ux + uy * uy).sqrt(), (vx * vx + vy * vy).sqrt());
            if lu < 1e-9 || lv < 1e-9 {
                continue;
            }
            let cos = (ux * vx + uy * vy) / (lu * lv);
            if cos < 0.5 {
                st.corners += 1;
                if lu > 12.0 && lv > 12.0 {
                    st.sharp += 1;
                    eprintln!("{name} {tag}: edge {ei} {}>{} sharp corner at ({:.1},{:.1}) legs {:.1}/{:.1}", e.from, e.to, b.0, b.1, lu, lv);
                }
            }
        }
        // Picking along the waypoint polyline (the drawn path within
        // corner rounding), away from the ends and from nodes.
        if w.len() >= 2 {
            let mut acc = 0.0;
            let total: f64 = w.windows(2).map(|p| ((p[1].0 - p[0].0).powi(2) + (p[1].1 - p[0].1).powi(2)).sqrt()).sum();
            for p in w.windows(2) {
                let l = ((p[1].0 - p[0].0).powi(2) + (p[1].1 - p[0].1).powi(2)).sqrt();
                let n = (l / 10.0).floor() as usize;
                for j in 0..n {
                    let t = j as f64 / n.max(1) as f64;
                    let s = acc + t * l;
                    acc += 0.0;
                    if s < 20.0 || s > total - 20.0 {
                        continue;
                    }
                    let x = p[0].0 + t * (p[1].0 - p[0].0);
                    let y = p[0].1 + t * (p[1].1 - p[0].1);
                    if sc.node_at(x, y).is_some() {
                        continue;
                    }
                    st.pick_pts += 1;
                    match sc.edge_at(x, y, 2.0) {
                        Some(_) => {}
                        None => {
                            st.pick_none += 1;
                            if st.pick_none < 6 {
                                eprintln!("{name} {tag}: edge {ei} {}>{} not picked at ({x:.1},{y:.1})", e.from, e.to);
                            }
                        }
                    }
                    if let Some(Hit::Node(_)) = sc.hit_test(x, y, 0.0) {
                        st.pick_node += 1;
                    }
                }
                acc += l;
            }
        }
    }
}

fn main() {
    let on = std::env::var("MODE").map(|m| m == "on").unwrap_or(false);
    let files: Vec<String> = std::env::args().skip(1).collect();
    for f in files {
        let name = std::path::Path::new(&f).file_name().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => continue,
        };
        let run = || {
            let mut st = Stats { parallel_bad: 0, sharp: 0, corners: 0, pick_pts: 0, pick_none: 0, pick_node: 0, nonfinite: 0 };
            let sc = scene::scene(&g);
            let sc2 = scene::scene(&g);
            let det_s = sig(&sc) == sig(&sc2) && scene::to_svg(&sc) == scene::to_svg(&sc2);
            check(&g, &sc, &mut st, "scene", &name);
            let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = scene::route(&g, &base);
            let rt2 = scene::route(&g, &base);
            let det_r = sig(&rt) == sig(&rt2);
            check(&g, &rt, &mut st, "route", &name);
            let n = base.len();
            let mut det_p = true;
            let stride = (n / 5).max(1);
            for v in (0..n).step_by(stride) {
                let mut pos = base.clone();
                pos[v].0 += 37.0;
                pos[v].1 -= 23.0;
                let p = scene::route_partial(&g, &pos, &sc, &base);
                let p2 = scene::route_partial(&g, &pos, &sc, &base);
                det_p &= sig(&p) == sig(&p2);
                check(&g, &p, &mut st, "partial", &name);
                let r = scene::route(&g, &pos);
                check(&g, &r, &mut st, "routemv", &name);
            }
            // to_mermaid round trip: emit, parse, emit again.
            let m1 = flowmaid::to_mermaid(&g);
            let rt_ok = match flowmaid::parser::parse_document(&m1) {
                Ok(Document::Flowchart(g2)) | Ok(Document::State(g2)) => flowmaid::to_mermaid(&g2) == m1,
                _ => false,
            };
            println!(
                "{name}\tedges={}/{}\tparallel_bad={}\tnonfinite={}\tcorners={}\tsharp={}\tpick={}/{}\tnone={}\tdet={}{}{}\tmmd_rt={}",
                sc.edges.len(), g.edges.len(), st.parallel_bad, st.nonfinite, st.corners, st.sharp,
                st.pick_pts - st.pick_none, st.pick_pts, st.pick_none,
                u8::from(det_s), u8::from(det_r), u8::from(det_p), u8::from(rt_ok)
            );
        };
        if on {
            flowmaid::quality::with_blocks(true, run);
        } else {
            run();
        }
    }
}
