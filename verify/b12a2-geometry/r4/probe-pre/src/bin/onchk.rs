//! Contract probe with the block layout switched ON (HEAD only).
//! Per file: block_mode, whether the drawing differs from the switch-off
//! one, and every contract violation found.
use flowmaid::model::{Document, Graph};
use flowmaid::quality::with_blocks;
use flowmaid::scene::{self, Scene};

fn sig(sc: &Scene) -> String {
    let mut s = String::new();
    for n in &sc.nodes {
        s += &format!("{}:{:.3},{:.3},{:.3},{:.3};", n.id, n.x, n.y, n.w, n.h);
    }
    for e in &sc.edges {
        s += &format!("{}>{}:{:?}|{:?};", e.from, e.to, e.bezier, e.waypoints);
    }
    for c in &sc.clusters {
        s += &format!("{}:{:.3},{:.3},{:.3},{:.3};", c.id, c.x, c.y, c.w, c.h);
    }
    s += &format!("{:.3}x{:.3}", sc.width, sc.height);
    s
}

fn finite(sc: &Scene) -> bool {
    sc.width.is_finite()
        && sc.height.is_finite()
        && sc.nodes.iter().all(|n| n.x.is_finite() && n.y.is_finite() && n.w.is_finite() && n.h.is_finite())
        && sc.clusters.iter().all(|c| c.x.is_finite() && c.y.is_finite() && c.w.is_finite() && c.h.is_finite())
        && sc.edges.iter().all(|e| {
            e.bezier.iter().all(|p| p.0.is_finite() && p.1.is_finite())
                && e.waypoints.iter().all(|p| p.0.is_finite() && p.1.is_finite())
        })
}

/// Contract checks on one scene; returns the violations.
fn check(g: &Graph, sc: &Scene, what: &str, out: &mut Vec<String>) {
    if !finite(sc) {
        out.push(format!("{what}: non-finite geometry"));
    }
    if sc.nodes.len() != g.nodes.len() {
        out.push(format!("{what}: {} scene nodes for {} graph nodes", sc.nodes.len(), g.nodes.len()));
        return;
    }
    if sc.edges.len() < g.edges.len() {
        out.push(format!("{what}: {} scene edges for {} graph edges", sc.edges.len(), g.edges.len()));
        return;
    }
    for (i, e) in g.edges.iter().enumerate() {
        let se = &sc.edges[i];
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
            out.push(format!("{what}: edge {i} not index-parallel ({}>{} vs {}>{})", se.from, se.to, g.nodes[e.from].id, g.nodes[e.to].id));
        }
    }
    // Node centres inside the canvas; clusters hold their members.
    for n in &sc.nodes {
        if n.x - n.w / 2.0 < -0.5 || n.y - n.h / 2.0 < -0.5 || n.x + n.w / 2.0 > sc.width + 0.5 || n.y + n.h / 2.0 > sc.height + 0.5 {
            out.push(format!("{what}: node {} outside the canvas", n.id));
        }
    }
    // Waypoints: dense where orthogonal — a long diagonal segment between
    // two waypoints means a host's spline will not reproduce a rounded
    // orthogonal path; and every waypoint inside the canvas.
    for (i, e) in sc.edges.iter().enumerate() {
        for w in e.waypoints.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            if dx > 30.0 && dy > 30.0 {
                out.push(format!("{what}: edge {i} ({}>{}) has a long diagonal waypoint step {:.0}x{:.0}", e.from, e.to, dx, dy));
                break;
            }
        }
        for p in e.waypoints.iter().chain(e.bezier.iter()) {
            if p.0 < -1.0 || p.1 < -1.0 || p.0 > sc.width + 1.0 || p.1 > sc.height + 1.0 {
                out.push(format!("{what}: edge {i} ({}>{}) leaves the canvas at {:.0},{:.0} (canvas {:.0}x{:.0})", e.from, e.to, p.0, p.1, sc.width, sc.height));
                break;
            }
        }
        // Pick == render, coarse: the middle of every waypoint step (or of
        // the cubic) is picked as an edge unless a node lies there.
        let mids: Vec<(f64, f64)> = if e.waypoints.len() >= 2 {
            e.waypoints.windows(2).map(|w| ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0)).collect()
        } else {
            let b = e.bezier;
            let t = 0.5f64;
            let u = 1.0 - t;
            vec![(
                u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0,
                u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1,
            )]
        };
        let len: f64 = e.waypoints.windows(2).map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt()).sum();
        if e.waypoints.len() >= 2 && len < 0.5 {
            continue; // a zero-length stand-in is not drawn
        }
        let mut miss = 0;
        for &(x, y) in &mids {
            if sc.node_at(x, y).is_some() {
                continue;
            }
            if sc.edge_at(x, y, 4.0).is_none() {
                miss += 1;
            }
        }
        if miss * 4 > mids.len().max(1) {
            out.push(format!("{what}: edge {i} ({}>{}) not picked on {miss} of {} waypoint steps", e.from, e.to, mids.len()));
        }
    }
}

fn main() {
    let on_flag = std::env::var("MODE").map(|m| m != "off").unwrap_or(true);
    let files: Vec<String> = std::env::args().skip(1).collect();
    let (mut nblock, mut nbad, mut ndiff_outside) = (0, 0, 0);
    for f in &files {
        let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => continue,
        };
        let r = std::panic::catch_unwind(|| {
            let mut bad: Vec<String> = Vec::new();
            let off = with_blocks(false, || scene::scene(&g));
            let off_c: Vec<(f64, f64)> = off.nodes.iter().map(|n| (n.x, n.y)).collect();
            let off_r = with_blocks(false, || scene::route(&g, &off_c));
            let rep = with_blocks(on_flag, || flowmaid::quality::measure_source(&src, false));
            let block_mode = rep.as_ref().map(|r| r.block_mode).unwrap_or(false);
            let on = with_blocks(on_flag, || scene::scene(&g));
            let on2 = with_blocks(on_flag, || scene::scene(&g));
            if sig(&on) != sig(&on2) || scene::to_svg(&on) != scene::to_svg(&on2) {
                bad.push("scene not deterministic".into());
            }
            let same = scene::to_svg(&on) == scene::to_svg(&off) && sig(&on) == sig(&off);
            check(&g, &on, "scene", &mut bad);
            let c: Vec<(f64, f64)> = on.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = with_blocks(on_flag, || scene::route(&g, &c));
            let rt2 = with_blocks(on_flag, || scene::route(&g, &c));
            if sig(&rt) != sig(&rt2) {
                bad.push("route not deterministic".into());
            }
            check(&g, &rt, "route", &mut bad);
            // Outside block mode the switch changes nothing.
            if !block_mode {
                if !same {
                    bad.push("NOT block mode but scene differs with the switch on".into());
                }
                let rt_off_at_on = with_blocks(false, || scene::route(&g, &c));
                if sig(&rt) != sig(&rt_off_at_on) || sig(&off_r) != sig(&with_blocks(on_flag, || scene::route(&g, &off_c))) {
                    bad.push("NOT block mode but route differs with the switch on".into());
                }
            }
            // Drags in block mode: every node (capped) moved, partial and full.
            let n = c.len();
            let stride = if g.edges.len() > 150 { n.max(1) } else { (n / 10).max(1) };
            for v in (0..n).step_by(stride) {
                for &(dx, dy) in &[(7.0, 5.0), (-120.0, 60.0), (1.0, 0.0)] {
                    let mut pos = c.clone();
                    pos[v].0 += dx;
                    pos[v].1 += dy;
                    let p = with_blocks(on_flag, || scene::route_partial(&g, &pos, &on, &c));
                    let p2 = with_blocks(on_flag, || scene::route_partial(&g, &pos, &on, &c));
                    if sig(&p) != sig(&p2) {
                        bad.push(format!("route_partial not deterministic (node {v})"));
                    }
                    let mut pb = Vec::new();
                    check(&g, &p, "partial", &mut pb);
                    pb.retain(|m| !m.contains("outside the canvas") && !m.contains("leaves the canvas"));
                    if let Some(m) = pb.first() {
                        bad.push(format!("drag node {v} by {dx},{dy}: {m}"));
                    }
                    let r = with_blocks(on_flag, || scene::route(&g, &pos));
                    if !finite(&r) {
                        bad.push(format!("route non-finite after drag of node {v}"));
                    }
                }
            }
            // to_mermaid does not depend on the switch.
            let m_on = with_blocks(on_flag, || flowmaid::to_mermaid(&g));
            let m_off = with_blocks(false, || flowmaid::to_mermaid(&g));
            if m_on != m_off {
                bad.push("to_mermaid differs with the switch".into());
            }
            // Round trip: parse(to_mermaid(g)) draws the same block scene.
            if let Ok(Document::Flowchart(g2)) = flowmaid::parser::parse_document(&m_on) {
                let again = with_blocks(on_flag, || scene::scene(&g2));
                if matches!(flowmaid::parser::parse_document(&src), Ok(Document::Flowchart(_))) && again.nodes.len() == on.nodes.len() {
                    let moved = again.nodes.iter().zip(&on.nodes).filter(|(a, b)| (a.x - b.x).abs() > 0.5 || (a.y - b.y).abs() > 0.5).count();
                    if moved > 0 {
                        bad.push(format!("round trip through to_mermaid moves {moved} nodes"));
                    }
                }
            }
            (block_mode, same, bad)
        });
        match r {
            Ok((bm, same, bad)) => {
                nblock += usize::from(bm);
                if !bm && !same {
                    ndiff_outside += 1;
                }
                if !bad.is_empty() {
                    nbad += 1;
                }
                println!("{name}\tblock_mode={}\tsame_as_off={}\tviolations={}", u8::from(bm), u8::from(same), bad.len());
                for b in bad.iter().take(6) {
                    println!("    {b}");
                }
            }
            Err(_) => {
                nbad += 1;
                println!("{name}\tPANIC");
            }
        }
    }
    println!("TOTAL files={} block_mode={nblock} with_violations={nbad} differs_outside_block_mode={ndiff_outside}", files.len());
}
