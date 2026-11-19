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


fn painted(svg: &str) -> Vec<(String, Vec<(f64, f64)>)> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(i) = rest.find("<path d=\"") {
        rest = &rest[i + 9..];
        let Some(j) = rest.find('"') else { break };
        let d = &rest[..j];
        let tail = &rest[j..];
        let end = tail.find('>').unwrap_or(0);
        let after = &tail[end + 1..];
        if !after.starts_with("<title>") { continue; }
        let t_end = after.find("</title>").unwrap_or(7);
        let title = after[7..t_end].to_string();
        let toks: Vec<&str> = d.split(|c: char| c == ' ' || c == ',').filter(|t| !t.is_empty()).collect();
        let mut pts: Vec<(f64, f64)> = Vec::new();
        let mut k = 0; let mut cmd = "M";
        let mut cur = (0.0, 0.0);
        while k < toks.len() {
            let t = toks[k];
            if t.chars().all(|c| c.is_ascii_alphabetic()) { cmd = t; k += 1; if cmd == "Z" { continue; } }
            let num = |k: &mut usize| -> f64 { let v = toks.get(*k).and_then(|x| x.parse().ok()).unwrap_or(0.0); *k += 1; v };
            match cmd {
                "M" | "L" => { let p = (num(&mut k), num(&mut k)); pts.push(p); cur = p; }
                "C" => { let a = (num(&mut k), num(&mut k)); let b = (num(&mut k), num(&mut k)); let c = (num(&mut k), num(&mut k));
                    for s in 1..=12 { let t = s as f64 / 12.0; let u = 1.0 - t;
                        pts.push((u*u*u*cur.0 + 3.0*u*u*t*a.0 + 3.0*u*t*t*b.0 + t*t*t*c.0, u*u*u*cur.1 + 3.0*u*u*t*a.1 + 3.0*u*t*t*b.1 + t*t*t*c.1)); }
                    cur = c; }
                "Q" => { let a = (num(&mut k), num(&mut k)); let c = (num(&mut k), num(&mut k));
                    for s in 1..=8 { let t = s as f64 / 8.0; let u = 1.0 - t; pts.push((u*u*cur.0 + 2.0*u*t*a.0 + t*t*c.0, u*u*cur.1 + 2.0*u*t*a.1 + t*t*c.1)); }
                    cur = c; }
                "A" => { for _ in 0..5 { num(&mut k); } let c = (num(&mut k), num(&mut k)); pts.push(c); cur = c; }
                _ => { k += 1; }
            }
        }
        out.push((title, pts));
    }
    out
}

fn pick_painted(name: &str, sc: &Scene, tag: &str, out: &mut String) -> usize {
    let svg = scene::to_svg(sc);
    let paths = painted(&svg);
    // routed scenes are painted shifted (view_shift): undo the most common start offset
    let mut votes: std::collections::BTreeMap<(i64, i64), usize> = std::collections::BTreeMap::new();
    {
        let mut used0 = vec![false; paths.len()];
        for e in sc.edges.iter() {
            let key = format!("{} \u{2192} {}", e.from, e.to);
            if let Some(pi) = (0..paths.len()).find(|&j| !used0[j] && (paths[j].0 == key || paths[j].0.starts_with(&format!("{key}:")))) {
                used0[pi] = true;
                if let Some(&q) = paths[pi].1.first() {
                    *votes.entry((((q.0 - e.bezier[0].0) * 10.0).round() as i64, ((q.1 - e.bezier[0].1) * 10.0).round() as i64)).or_default() += 1;
                }
            }
        }
    }
    let shift = votes.iter().max_by_key(|(_, &c)| c).map(|(&(x, y), _)| (x as f64 / 10.0, y as f64 / 10.0)).unwrap_or((0.0, 0.0));
    let paths: Vec<(String, Vec<(f64, f64)>)> = paths.into_iter().map(|(t, q)| (t, q.into_iter().map(|p| (p.0 - shift.0, p.1 - shift.1)).collect())).collect();
    let mut used = vec![false; paths.len()];
    let mut miss = 0;
    for (i, e) in sc.edges.iter().enumerate() {
        let key = format!("{} \u{2192} {}", e.from, e.to);
        let Some(pi) = (0..paths.len()).find(|&j| !used[j] && (paths[j].0 == key || paths[j].0.starts_with(&format!("{key}:")))) else { continue };
        used[pi] = true;
        let pts = &paths[pi].1;
        if pts.len() < 2 { continue; }
        for f in [0.3, 0.5, 0.7] {
            let p = point_at(pts, f);
            if sc.node_at(p.0, p.1).is_some() { continue; }
            match sc.edge_at(p.0, p.1, 3.0) {
                Some(k) if k == i => {}
                other => {
                    // another painted edge through the same point is fine
                    let ok = other.map_or(false, |k| {
                        let ok_key = format!("{} \u{2192} {}", sc.edges[k].from, sc.edges[k].to);
                        paths.iter().any(|(t, q)| (t == &ok_key || t.starts_with(&format!("{ok_key}:"))) && q.windows(2).any(|w| seg_dist(p, w[0], w[1]) < 2.0))
                    });
                    if !ok { miss += 1; out.push_str(&format!("  {tag} painted pick miss edge {i} ({} -> {}) at {:.1},{:.1}: {:?}\n", e.from, e.to, p.0, p.1, other)); }
                }
            }
        }
    }
    let _ = name;
    miss
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
        let hr = sc.hits_in_rect(p.0 - 1.0, p.1 - 1.0, p.0 + 1.0, p.1 + 1.0);
        if !hr.iter().any(|h| matches!(h, Hit::Edge(_))) {
            out.push_str(&format!("  marquee at edge {i} midpoint found no edge\n"));
            probs.push(format!("{name} blocks={blocks}: marquee missed edge {i}"));
        }
        match sc.hit_test(p.0, p.1, 4.0) {
            Some(Hit::Edge(_)) | Some(Hit::Cluster(_)) | Some(Hit::Node(_)) | None => {}
        }
    }
    out.push_str(&format!("  edge picks: {tot} tried, {miss} missed\n"));
    let pm0 = pick_painted(name, &sc, "scene", out);
    out.push_str(&format!("  painted picks missed (scene): {pm0}\n"));
    if pm0 > 0 { probs.push(format!("{name} blocks={blocks}: {pm0} painted picks missed")); }
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
    let rpm = pick_painted(name, &r, "route", out);
    if rpm > 0 { probs.push(format!("{name} blocks={blocks}: route {rpm} painted picks missed")); }
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
            let dpm = pick_painted(name, &p, "drag", out);
            if dpm > 0 { probs.push(format!("{name} blocks={blocks}: drag {k} {d:?}: {dpm} painted picks missed")); }
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
