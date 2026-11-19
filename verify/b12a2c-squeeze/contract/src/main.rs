use flowmaid::model::Document;
use flowmaid::scene::{self, Hit, Scene};
use std::time::Instant;

fn centres(sc: &Scene) -> Vec<(f64, f64)> {
    sc.nodes.iter().map(|n| (n.x, n.y)).collect()
}

fn poly_len(p: &[(f64, f64)]) -> f64 {
    p.windows(2).map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt()).sum()
}

fn point_at(p: &[(f64, f64)], f: f64) -> (f64, f64) {
    let total = poly_len(p);
    let mut want = total * f;
    for w in p.windows(2) {
        let l = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        if l >= want && l > 0.0 {
            let t = want / l;
            return (w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t);
        }
        want -= l;
    }
    *p.last().unwrap()
}

fn bez_at(b: &[(f64, f64); 4], t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let x = u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0;
    let y = u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1;
    (x, y)
}

/// Sharpest single-point turn (degrees) among waypoints, ignoring the ends.
fn sharp_turns(p: &[(f64, f64)]) -> usize {
    let mut n = 0;
    for w in p.windows(3) {
        let a = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let b = (w[2].0 - w[1].0, w[2].1 - w[1].1);
        let la = (a.0 * a.0 + a.1 * a.1).sqrt();
        let lb = (b.0 * b.0 + b.1 * b.1).sqrt();
        if la < 1e-6 || lb < 1e-6 {
            continue;
        }
        let c = ((a.0 * b.0 + a.1 * b.1) / (la * lb)).clamp(-1.0, 1.0);
        let ang = c.acos().to_degrees();
        // a corner turned in one point, with long legs on both sides: not rounded
        if ang > 60.0 && la > 12.0 && lb > 12.0 {
            n += 1;
        }
    }
    n
}

fn check(name: &str, g: &flowmaid::model::Graph, blocks: bool, out: &mut String) -> Vec<String> {
    let mut probs = Vec::new();
    let sc = flowmaid::quality::with_blocks(blocks, || scene::scene(g));
    let sc2 = flowmaid::quality::with_blocks(blocks, || scene::scene(g));
    let svg = scene::to_svg(&sc);
    if svg != scene::to_svg(&sc2) {
        probs.push(format!("{name}: scene not deterministic"));
    }
    out.push_str(&format!("=== {name} blocks={blocks} block_layout? svg {} bytes\n", svg.len()));
    // index-parallel
    if sc.edges.len() < g.edges.len() {
        probs.push(format!("{name}: scene edges {} < graph edges {}", sc.edges.len(), g.edges.len()));
    }
    for (i, e) in g.edges.iter().enumerate() {
        let se = &sc.edges[i];
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
            probs.push(format!("{name}: edge {i} not index-parallel ({} -> {} vs {} -> {})", se.from, se.to, g.nodes[e.from].id, g.nodes[e.to].id));
        }
    }
    // waypoints rounded / dense
    let mut sharp = 0;
    for e in &sc.edges {
        sharp += sharp_turns(&e.waypoints);
    }
    if sharp > 0 {
        out.push_str(&format!("  sharp single-point corners in waypoints: {sharp}\n"));
    }
    // picking: the midpoint of each drawn edge picks it (or an edge passing as close)
    let mut miss = 0;
    let mut tot = 0;
    for (i, e) in sc.edges.iter().enumerate() {
        let p = if e.waypoints.len() >= 2 { point_at(&e.waypoints, 0.37) } else { bez_at(&e.bezier, 0.37) };
        if sc.node_at(p.0, p.1).is_some() {
            continue;
        }
        tot += 1;
        match sc.edge_at(p.0, p.1, 4.0) {
            Some(k) if k == i => {}
            Some(k) => {
                // another edge at the same spot is fine if its route also passes there
                let o = &sc.edges[k];
                let d = if o.waypoints.len() >= 2 {
                    o.waypoints.windows(2).map(|w| seg_dist(p, w[0], w[1])).fold(f64::INFINITY, f64::min)
                } else { 0.0 };
                if d > 6.0 { miss += 1; out.push_str(&format!("  pick of edge {i} at {p:?} gave {k} (d {d:.1})\n")); }
            }
            None => {
                miss += 1;
                out.push_str(&format!("  pick of edge {i} ({} -> {}) at {:.1},{:.1} gave none\n", e.from, e.to, p.0, p.1));
            }
        }
        match sc.hit_test(p.0, p.1, 4.0) {
            Some(Hit::Edge(_)) | Some(Hit::Cluster(_)) | Some(Hit::Node(_)) | None => {}
        }
    }
    out.push_str(&format!("  edge picks: {tot} tried, {miss} missed\n"));
    if miss > 0 { probs.push(format!("{name} blocks={blocks}: {miss} picks missed")); }
    // route at the scene's centres
    let c = centres(&sc);
    let t0 = Instant::now();
    let r = flowmaid::quality::with_blocks(blocks, || scene::route(g, &c));
    let rt = t0.elapsed().as_secs_f64() * 1e3;
    let rs = scene::to_svg(&r);
    let r2 = flowmaid::quality::with_blocks(blocks, || scene::route(g, &c));
    if rs != scene::to_svg(&r2) { probs.push(format!("{name}: route not deterministic")); }
    if r.edges.len() != sc.edges.len() { probs.push(format!("{name}: route edges {} vs scene {}", r.edges.len(), sc.edges.len())); }
    let mut fr = 0.0f64;
    for (a, b) in sc.clusters.iter().zip(&r.clusters) {
        fr = fr.max((a.x - b.x).abs()).max((a.y - b.y).abs()).max((a.w - b.w).abs()).max((a.h - b.h).abs());
    }
    out.push_str(&format!("  route ms {rt:.2}; route==scene svg: {}; frame max diff {fr:.2}\n", rs == svg));
    // drags: move each of 3 nodes by (60,25) and (-150,80)
    let n = g.nodes.len();
    let mut worst = 0.0f64;
    for &k in &[0usize, n / 2, n.saturating_sub(1)] {
        for d in [(60.0, 25.0), (-150.0, 80.0), (1.0, 0.0)] {
            let mut c2 = c.clone();
            c2[k].0 += d.0; c2[k].1 += d.1;
            let t0 = Instant::now();
            let p = flowmaid::quality::with_blocks(blocks, || scene::route_partial(g, &c2, &r, &c));
            let ms = t0.elapsed().as_secs_f64() * 1e3;
            worst = worst.max(ms);
            let p2 = flowmaid::quality::with_blocks(blocks, || scene::route_partial(g, &c2, &r, &c));
            if scene::to_svg(&p) != scene::to_svg(&p2) { probs.push(format!("{name}: route_partial not deterministic")); }
            for (i, e) in g.edges.iter().enumerate() {
                if p.edges[i].from != g.nodes[e.from].id || p.edges[i].to != g.nodes[e.to].id {
                    probs.push(format!("{name}: drag edge {i} not index-parallel"));
                }
            }
            // moved node centre
            if ((p.nodes[k].x - c2[k].0).abs() > 0.01) || ((p.nodes[k].y - c2[k].1).abs() > 0.01) {
                probs.push(format!("{name}: dragged node {k} not at its centre"));
            }
            // pick the dragged node
            if p.node_at(c2[k].0, c2[k].1) != Some(k) {
                // other nodes may overlap after a far drag
                out.push_str(&format!("  node_at after drag of {k} by {d:?}: {:?}\n", p.node_at(c2[k].0, c2[k].1)));
            }
            let mut pm = 0;
            for (i, e) in p.edges.iter().enumerate() {
                let q = if e.waypoints.len() >= 2 { point_at(&e.waypoints, 0.5) } else { bez_at(&e.bezier, 0.5) };
                if p.node_at(q.0, q.1).is_some() { continue; }
                if p.edge_at(q.0, q.1, 4.0).is_none() { pm += 1; let _ = i; }
            }
            if pm > 0 { probs.push(format!("{name}: drag {k} {d:?}: {pm} edge picks missed")); }
        }
    }
    out.push_str(&format!("  worst route_partial ms {worst:.2}\n"));
    // to_mermaid round trip
    let m = flowmaid::to_mermaid(g);
    match flowmaid::parser::parse_document(&m) {
        Ok(Document::Flowchart(g2)) | Ok(Document::State(g2)) => {
            let m2 = flowmaid::to_mermaid(&g2);
            if m2 != m { probs.push(format!("{name}: to_mermaid round-trip changed")); }
            let s2 = flowmaid::quality::with_blocks(blocks, || scene::scene(&g2));
            out.push_str(&format!("  reparsed scene svg equal: {}\n", scene::to_svg(&s2) == svg));
        }
        Ok(_) => probs.push(format!("{name}: to_mermaid reparsed as another kind")),
        Err(e) => probs.push(format!("{name}: to_mermaid reparse error {e:?}")),
    }
    // dump the svgs
    if let Ok(dir) = std::env::var("DUMP") {
        let tag = if blocks { "on" } else { "off" };
        std::fs::write(format!("{dir}/{name}.{tag}.scene.svg"), &svg).unwrap();
        std::fs::write(format!("{dir}/{name}.{tag}.route.svg"), &rs).unwrap();
    }
    probs
}

fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1e-12 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    let q = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt()
}

fn main() {
    let mut out = String::new();
    let mut probs = Vec::new();
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => { out.push_str(&format!("skip {name}\n")); continue; }
        };
        for blocks in [false, true] {
            probs.extend(check(&name, &g, blocks, &mut out));
        }
    }
    print!("{out}");
    println!("PROBLEMS {}", probs.len());
    for p in probs { println!("  {p}"); }
}
