use flowmaid::model::{Document, Graph};
use flowmaid::scene::{self, Hit, Scene};
use std::time::Instant;

type Pt = (f64, f64);

fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}

fn cubic(p: [Pt; 4], t: f64) -> Pt {
    let u = 1.0 - t;
    let a = u * u * u;
    let b = 3.0 * u * u * t;
    let c = 3.0 * u * t * t;
    let d = t * t * t;
    (
        a * p[0].0 + b * p[1].0 + c * p[2].0 + d * p[3].0,
        a * p[0].1 + b * p[1].1 + c * p[2].1 + d * p[3].1,
    )
}

/// Points sampled along an SVG path `d` (M/L/C/Q/Z; A arcs skipped).
fn sample_d(d: &str) -> Vec<Pt> {
    let toks: Vec<String> = d
        .replace(',', " ")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut cur = (0.0, 0.0);
    let mut cmd = String::new();
    let num = |i: &mut usize| -> f64 {
        let v = toks[*i].parse::<f64>().unwrap_or(f64::NAN);
        *i += 1;
        v
    };
    while i < toks.len() {
        if toks[i].chars().next().unwrap().is_ascii_alphabetic() {
            cmd = toks[i].clone();
            i += 1;
        }
        match cmd.as_str() {
            "M" => {
                cur = (num(&mut i), num(&mut i));
                out.push(cur);
                cmd = "L".into();
            }
            "L" => {
                let p = (num(&mut i), num(&mut i));
                for k in 1..=10 {
                    let t = k as f64 / 10.0;
                    out.push((cur.0 + (p.0 - cur.0) * t, cur.1 + (p.1 - cur.1) * t));
                }
                cur = p;
            }
            "C" => {
                let p1 = (num(&mut i), num(&mut i));
                let p2 = (num(&mut i), num(&mut i));
                let p3 = (num(&mut i), num(&mut i));
                for k in 1..=20 {
                    out.push(cubic([cur, p1, p2, p3], k as f64 / 20.0));
                }
                cur = p3;
            }
            "Q" => {
                let p1 = (num(&mut i), num(&mut i));
                let p2 = (num(&mut i), num(&mut i));
                for k in 1..=20 {
                    let t = k as f64 / 20.0;
                    let u = 1.0 - t;
                    out.push((
                        u * u * cur.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
                        u * u * cur.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
                    ));
                }
                cur = p2;
            }
            "A" => {
                for _ in 0..5 {
                    num(&mut i);
                }
                cur = (num(&mut i), num(&mut i));
                out.push(cur);
            }
            "Z" | "z" => {}
            other => panic!("unknown cmd {other} in {d}"),
        }
    }
    out
}

/// The `d` of each edge `<path>` (those with a `<title>` child that names
/// an edge: 'A → B'), in paint order.
fn edge_paths(svg: &str) -> Vec<String> {
    let mut v = Vec::new();
    for line in svg.lines() {
        if line.starts_with("<path d=\"") && line.contains("fill=\"none\"") && line.contains("<title>")
        {
            let d = &line[9..];
            let end = d.find('"').unwrap();
            v.push(d[..end].to_string());
        }
    }
    v
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn check(name: &str, src: &str, report: &mut Vec<String>) {
    let Some(g) = graph_of(src) else {
        report.push(format!("{name}: not a flowchart/state"));
        return;
    };
    let t0 = Instant::now();
    let sc = scene::scene(&g);
    let scene_ms = t0.elapsed().as_secs_f64() * 1e3;
    // Determinism
    let sc2 = scene::scene(&g);
    let svg = scene::to_svg(&sc);
    assert_eq!(svg, scene::to_svg(&sc2), "{name}: scene svg nondeterministic");
    // index parallel
    let mut bad = 0;
    for (i, e) in g.edges.iter().enumerate() {
        let se = &sc.edges[i];
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
            bad += 1;
        }
    }
    assert!(sc.edges.len() >= g.edges.len());
    if bad > 0 {
        report.push(format!("{name}: {bad} edges not index-parallel"));
    }
    // pick == render: sample each painted edge path; scene coords = svg - (tx,ty)
    // Find translation from the first node's svg title group: use width/height match.
    let pick_fail = pick_check(&sc, &svg);
    // Drag: move each node in turn by (+37, +23), route + route_partial
    let base_c: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let base_r = scene::route(&g, &base_c);
    let r_again = scene::route(&g, &base_c);
    assert_eq!(scene::to_svg(&base_r), scene::to_svg(&r_again), "{name}: route nondeterministic");
    let mut worst_route: f64 = 0.0;
    let mut worst_partial: f64 = 0.0;
    let mut route_pick_fail = 0;
    let mut sparse_corner = 0;
    let mut nonpar = 0;
    let mut cyl_off = 0;
    let n = g.nodes.len();
    for k in 0..n.min(12) {
        let mut c = base_c.clone();
        c[k] = (c[k].0 + 37.0, c[k].1 + 23.0);
        let t = Instant::now();
        let r = scene::route(&g, &c);
        worst_route = worst_route.max(t.elapsed().as_secs_f64() * 1e3);
        let mut best = f64::INFINITY;
        let mut p = base_r.clone();
        for _ in 0..3 {
            let t = Instant::now();
            p = scene::route_partial(&g, &c, &base_r, &base_c);
            best = best.min(t.elapsed().as_secs_f64() * 1e3);
        }
        worst_partial = worst_partial.max(best);
        let p2 = scene::route_partial(&g, &c, &base_r, &base_c);
        assert_eq!(scene::to_svg(&p), scene::to_svg(&p2), "{name}: partial nondeterministic");
        for s in [&r, &p] {
            for (i, e) in g.edges.iter().enumerate() {
                if s.edges[i].from != g.nodes[e.from].id || s.edges[i].to != g.nodes[e.to].id {
                    nonpar += 1;
                }
            }
            // waypoints: at every turn > 35 deg, the neighbouring segments are short (rounded)
            for e in &s.edges {
                let w = &e.waypoints;
                if w.len() < 3 {
                    continue;
                }
                for j in 1..w.len() - 1 {
                    let a = (w[j].0 - w[j - 1].0, w[j].1 - w[j - 1].1);
                    let b = (w[j + 1].0 - w[j].0, w[j + 1].1 - w[j].1);
                    let la = a.0.hypot(a.1);
                    let lb = b.0.hypot(b.1);
                    if la < 1e-6 || lb < 1e-6 {
                        continue;
                    }
                    let cos = (a.0 * b.0 + a.1 * b.1) / (la * lb);
                    if cos < (75f64).to_radians().cos() && la > 30.0 && lb > 30.0 {
                        sparse_corner += 1;
                    }
                }
            }
            // cylinder ends on the drum outline (top/bottom caps)
            for e in &s.edges {
                for (id, at_start) in [(&e.from, true), (&e.to, false)] {
                    let Some(nd) = s.nodes.iter().find(|n| &n.id == id) else { continue };
                    if nd.shape != flowmaid::model::Shape::Cylinder {
                        continue;
                    }
                    let pt = if e.waypoints.is_empty() {
                        if at_start { e.bezier[0] } else { e.bezier[3] }
                    } else if at_start {
                        e.waypoints[0]
                    } else {
                        e.waypoints[e.waypoints.len() - 1]
                    };
                    let rx = nd.w / 2.0;
                    let ry = (rx / (2.5 + nd.w / 50.0)).min(10.0).min(nd.h / 4.0);
                    let dx = pt.0 - nd.x;
                    if dx.abs() >= rx - 0.5 {
                        continue;
                    }
                    let top = nd.y - nd.h / 2.0;
                    let bot = nd.y + nd.h / 2.0;
                    let k = dx / rx;
                    let ins = ry * (1.0 - (1.0 - k * k).max(0.0).sqrt());
                    // an end on a cap: its y must be top+ins (or bot-ins)
                    if (pt.1 - top).abs() < ry + 0.6 && (pt.1 - (top + ins)).abs() > 0.6 {
                        cyl_off += 1;
                    }
                    if (pt.1 - bot).abs() < ry + 0.6 && (pt.1 - (bot - ins)).abs() > 0.6 {
                        cyl_off += 1;
                    }
                }
            }
        }
        route_pick_fail += pick_check(&r, &scene::to_svg(&r));
        route_pick_fail += pick_check(&p, &scene::to_svg(&p));
    }
    // end gaps (B0 metric) static, routed, every drag frame
    let mut gaps = (0usize, 0.0f64);
    let mut acc = |s: &Scene, w: &str| {
        let m = flowmaid::quality::measure(&g, s);
        gaps.0 += m.end_gaps;
        gaps.1 = gaps.1.max(m.end_gap_max);
        if m.end_gaps > 0 && std::env::var("DBG").is_ok() {
            for d in m.details.iter().filter(|d| d.contains("end_gap")) { eprintln!("{name} [{w}]: {d}"); }
        }
    };
    acc(&sc, "static");
    acc(&base_r, "routed");
    let size: Vec<Pt> = base_r.nodes.iter().map(|n| (n.w, n.h)).collect();
    let mut nframes = 0;
    let mut worst_frame: f64 = 0.0;
    for (v, centres) in flowmaid::quality::drag_frames(&base_r) {
        let overlaps = (0..centres.len()).any(|u| {
            u != v
                && (centres[u].0 - centres[v].0).abs() < (size[u].0 + size[v].0) / 2.0
                && (centres[u].1 - centres[v].1).abs() < (size[u].1 + size[v].1) / 2.0
        });
        if overlaps { continue; }
        let t = Instant::now();
        let p = scene::route_partial(&g, &centres, &base_r, &base_c);
        worst_frame = worst_frame.max(t.elapsed().as_secs_f64() * 1e3);
        acc(&p, &format!("drag v{v} {:?}", centres[v]));
        nframes += 1;
    }
    // corner picking: an empty box corner of a slanted/round shape is not that node
    let mut corner_bad = 0;
    for (i, n) in sc.nodes.iter().enumerate() {
        use flowmaid::model::Shape::*;
        let want_empty = match n.shape { Hexagon | Cylinder | Stadium | Rounded | Diamond | Circle => 4, Parallelogram | ParallelogramAlt => 2, _ => 0 };
        if want_empty == 0 { continue; }
        let (hw, hh) = (n.w / 2.0 - 0.6, n.h / 2.0 - 0.6);
        let mut empty = 0;
        for c in [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)] {
            if sc.node_at(n.x + c.0, n.y + c.1) != Some(i) { empty += 1; }
        }
        if empty != want_empty { corner_bad += 1; if std::env::var("DBG").is_ok() { eprintln!("{name}: corner {} {:?} empty {empty}", n.id, n.shape); } }
    }
    report.push(format!("{name}: end_gaps={} end_gap_max={:.3} frames={nframes} partial_frame_worst={worst_frame:.2} corner_bad={corner_bad}", gaps.0, gaps.1));
    if let Ok(dir) = std::env::var("DUMP") {
        std::fs::write(format!("{dir}/{name}.mmd.out"), flowmaid::to_mermaid(&g)).unwrap();
        let labels: Vec<String> = sc.nodes.iter().map(|n| format!("{} at {:.1},{:.1} {:.1}x{:.1} {:?}", n.id, n.x, n.y, n.w, n.h, n.label)).chain(sc.clusters.iter().map(|c| format!("FRAME {} {:?} at {:.1},{:.1} {:.1}x{:.1}", c.id, c.title, c.x, c.y, c.w, c.h))).collect();
        std::fs::write(format!("{dir}/{name}.labels"), labels.join("\n")).unwrap();
        std::fs::write(format!("{dir}/{name}.static.svg"), &svg).unwrap();
        std::fs::write(format!("{dir}/{name}.routed.svg"), scene::to_svg(&base_r)).unwrap();
    }
    // to_mermaid round trip
    let m = flowmaid::to_mermaid(&g);
    let rt = match graph_of(&m) {
        Some(g2) => flowmaid::to_mermaid(&g2) == m,
        None => false,
    };
    report.push(format!(
        "{name}: edges={} scene_ms={scene_ms:.2} route_worst={worst_route:.2} partial_worst={worst_partial:.2} pick_fail(static)={pick_fail} pick_fail(drag)={route_pick_fail} nonparallel={nonpar} sparse_corners={sparse_corner} cyl_end_off={cyl_off} roundtrip={rt}",
        g.edges.len()
    ));
}

/// Number of sampled stroke points (from the SVG) where edge_at does not
/// pick an edge whose painted stroke is under that point.
fn pick_check(sc: &Scene, svg: &str) -> usize {
    // translation: compare scene node centre vs SVG; svg vb = scene width/height?
    let (tx, ty) = translation(sc, svg);
    let drawn: Vec<usize> = sc
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| svg.contains(&format!("<title>")) && e.from != "" && true)
        .map(|(i, _)| i)
        .collect();
    let paths = edge_paths(svg);
    let mut fail = 0;
    // map paths to edges by title "from → to" order — use edge_at directly:
    // each sampled point must be picked by SOME edge at tol 0.5 (the painted
    // stroke is under it), and the hit must be an Edge (or a Node on top).
    let _ = drawn;
    for d in &paths {
        let pts = sample_d(d);
        for (j, p) in pts.iter().enumerate() {
            if j == 0 || j + 1 == pts.len() {
                continue;
            }
            let q = (p.0 - tx, p.1 - ty);
            if sc.node_at(q.0, q.1).is_some() {
                continue;
            }
            if sc.edge_at(q.0, q.1, 0.5).is_none() {
                if std::env::var("DBG").is_ok() && fail < 3 { eprintln!("unpicked ({:.1},{:.1}) t=({tx:.1},{ty:.1}) d={}", q.0, q.1, &d[..d.len().min(80)]); }
                fail += 1;
            }
        }
    }
    // Node centres pick their node
    for (i, n) in sc.nodes.iter().enumerate() {
        match sc.hit_test(n.x, n.y, 2.0) {
            Some(Hit::Node(k)) if k == i => {}
            Some(Hit::Node(_)) => {}
            other => { if std::env::var("DBG").is_ok() { eprintln!("node {} ({:?}) centre ({:.1},{:.1}) hit {:?}", n.id, n.shape, n.x, n.y, other); } fail += 1000 },
        }
    }
    let _ = dist((0.0, 0.0), (0.0, 0.0));
    fail
}

fn translation(sc: &Scene, svg: &str) -> Pt {
    for n in &sc.nodes {
        if n.shape == flowmaid::model::Shape::Rect {
            let key = format!("width=\"{:.1}\" height=\"{:.1}\" rx=\"3.0\"", n.w, n.h);
            if let Some(pos) = svg.find(&key) {
                let pre = &svg[..pos];
                let rs = pre.rfind("<rect x=\"").unwrap();
                let s = &svg[rs + 9..];
                let x: f64 = s[..s.find('"').unwrap()].parse().unwrap();
                let s2 = &s[s.find("y=\"").unwrap() + 3..];
                let y: f64 = s2[..s2.find('"').unwrap()].parse().unwrap();
                return (x - (n.x - n.w / 2.0), y - (n.y - n.h / 2.0));
            }
        }
    }
    let mut texts: Vec<Pt> = Vec::new();
    for part in svg.split("<text x=\"").skip(1) {
        let x: f64 = match part[..part.find('"').unwrap()].parse() { Ok(v) => v, Err(_) => continue };
        let r = &part[part.find("y=\"").unwrap() + 3..];
        let y: f64 = match r[..r.find('"').unwrap()].parse() { Ok(v) => v, Err(_) => continue };
        texts.push((x, y));
    }
    let mut counts: std::collections::BTreeMap<(i64, i64), usize> = Default::default();
    for n in &sc.nodes {
        for t in &texts {
            let k = (((t.0 - n.x) * 10.0).round() as i64, ((t.1 - n.y) * 10.0).round() as i64);
            *counts.entry(k).or_default() += 1;
        }
    }
    let best = counts.iter().max_by_key(|(_, c)| **c).map(|(k, _)| *k).unwrap_or((0, 0));
    (best.0 as f64 / 10.0, best.1 as f64 / 10.0)
}


fn ends_of(e: &flowmaid::scene::SceneEdge) -> (Pt, Pt) {
    if e.waypoints.len() >= 2 {
        (e.waypoints[0], *e.waypoints.last().unwrap())
    } else {
        (e.bezier[0], e.bezier[3])
    }
}

/// Lone ends on straight node sides: (count, count off-centre > 5 px, max offset)
fn lone_ends(sc: &Scene) -> (usize, usize, f64, Vec<String>) {
    use flowmaid::model::Shape::*;
    let mut sides: std::collections::BTreeMap<(usize, u8), Vec<(f64, String)>> = Default::default();
    for e in &sc.edges {
        if e.from == e.to { continue; }
        let (a, b) = ends_of(e);
        for (id, p) in [(&e.from, a), (&e.to, b)] {
            let Some(vi) = sc.nodes.iter().position(|n| &n.id == id) else { continue };
            let n = &sc.nodes[vi];
            if !matches!(n.shape, Rect | Rounded | Stadium | Subroutine) { continue; }
            let (l, r, t, bt) = (n.x - n.w / 2.0, n.x + n.w / 2.0, n.y - n.h / 2.0, n.y + n.h / 2.0);
            let tag = format!("{}->{}", e.from, e.to);
            if (p.1 - t).abs() < 1.5 && p.0 > l && p.0 < r { sides.entry((vi, 0)).or_default().push((p.0 - n.x, tag)); }
            else if (p.1 - bt).abs() < 1.5 && p.0 > l && p.0 < r { sides.entry((vi, 1)).or_default().push((p.0 - n.x, tag)); }
            else if (p.0 - l).abs() < 1.5 && p.1 > t && p.1 < bt { sides.entry((vi, 2)).or_default().push((p.1 - n.y, tag)); }
            else if (p.0 - r).abs() < 1.5 && p.1 > t && p.1 < bt { sides.entry((vi, 3)).or_default().push((p.1 - n.y, tag)); }
        }
    }
    let (mut c, mut off, mut mx) = (0, 0, 0.0f64);
    let mut list = Vec::new();
    for ((vi, s), v) in &sides {
        if v.len() != 1 { continue; }
        c += 1;
        let o = v[0].0.abs();
        if o > 5.0 { off += 1; list.push(format!("{}:{} {} off {:.1}", sc.nodes[*vi].id, s, v[0].1, o)); }
        mx = mx.max(o);
    }
    (c, off, mx, list)
}

fn edge_moved(a: &flowmaid::scene::SceneEdge, b: &flowmaid::scene::SceneEdge, tol: f64) -> f64 {
    let mut worst = 0.0f64;
    if a.waypoints.len() != b.waypoints.len() { return 999.0; }
    for (p, q) in a.waypoints.iter().zip(&b.waypoints) { worst = worst.max(dist(*p, *q)); }
    if a.waypoints.is_empty() { for (p, q) in a.bezier.iter().zip(&b.bezier) { worst = worst.max(dist(*p, *q)); } }
    let _ = tol;
    worst
}

/// Labels whose chip comes within `ARROW` px of its own edge's arrival tip.
fn label_head(sc: &Scene) -> (usize, usize, Vec<String>) {
    let mut n = 0; let mut bad = 0; let mut l = Vec::new();
    for e in &sc.edges {
        let Some((_, m, w)) = &e.label else { continue };
        if e.from == e.to { continue; }
        n += 1;
        let tip = ends_of(e).1;
        let (hw, hh) = (w / 2.0, 10.0);
        let dx = ((tip.0 - m.0).abs() - hw).max(0.0);
        let dy = ((tip.1 - m.1).abs() - hh).max(0.0);
        let d = dx.hypot(dy);
        if d < 17.0 { bad += 1; l.push(format!("{}->{} {:.1}", e.from, e.to, d)); }
    }
    (n, bad, l)
}

fn b7(name: &str, g: &Graph, sc: &Scene, report: &mut Vec<String>) {
    let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let r = scene::route(g, &at);
    let (c0, o0, m0, l0) = lone_ends(sc);
    let (c1, o1, m1, l1) = lone_ends(&r);
    let (ln, lb, ll) = label_head(sc);
    let (rn, rb, rl) = label_head(&r);
    // static vs route
    let mut diff = 0; let mut dmax = 0.0f64; let mut dl = Vec::new();
    for i in 0..g.edges.len() { let d = edge_moved(&sc.edges[i], &r.edges[i], 1.0); if d > 1.0 { diff += 1; dmax = dmax.max(d); dl.push(format!("{}->{}:{:.0}", sc.edges[i].from, sc.edges[i].to, d)); } }
    // 1px drags via route_partial from static scene: unrelated edges moving
    let mut jumpy = 0; let mut jmax = 0.0f64; let mut frames = 0; let mut jl = Vec::new();
    let mut lone_drag_off = 0; let JT: f64 = std::env::var("JT").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    for v in 0..g.nodes.len().min(40) {
        let mut c = at.clone();
        c[v].0 += 1.0;
        let p = scene::route_partial(g, &c, sc, &at);
        frames += 1;
        lone_drag_off += lone_ends(&p).1;
        if std::env::var("FRAMEDBG").is_ok() {
            let nm: usize = (0..g.edges.len()).filter(|&i| { let e=&g.edges[i]; e.from!=v && e.to!=v && edge_moved(&sc.edges[i], &p.edges[i], 1.0) > JT }).count();
            let shift = p.nodes.iter().zip(&sc.nodes).map(|(a,b)| dist((a.x,a.y),(b.x,b.y))).fold(0.0f64, f64::max);
            let r2 = scene::route(g, &c);
            let same_as_route = (0..g.edges.len()).filter(|&i| edge_moved(&r2.edges[i], &p.edges[i], 1.0) < 0.01).count();
            eprintln!("{name} drag {} : unrelated moved {nm} / {} ; node shift {shift:.1}; edges equal to route(): {same_as_route}", g.nodes[v].id, g.edges.len());
        }
        for i in 0..g.edges.len() {
            let e = &g.edges[i];
            if e.from == v || e.to == v { continue; }
            let d = edge_moved(&sc.edges[i], &p.edges[i], 1.0);
            if d > JT { jumpy += 1; jmax = jmax.max(d); if jl.len() < 4 { jl.push(format!("drag {} moved {}->{} {:.0}", g.nodes[v].id, sc.edges[i].from, sc.edges[i].to, d)); } }
        }
    }
    report.push(format!("{name}: B7 lone static {c0}/{o0} off max {m0:.1} | route {c1}/{o1} max {m1:.1} | lbl~head static {lb}/{ln} route {rb}/{rn} | scene!=route edges {diff} max {dmax:.0} | 1px-drag unrelated moves {jumpy} in {frames} frames max {jmax:.0} | drag lone off {lone_drag_off}"));
    if std::env::var("B7DBG").is_ok() {
        for x in l0 { report.push(format!("   static lone {x}")); }
        for x in l1 { report.push(format!("   route lone {x}")); }
        for x in ll { report.push(format!("   static lbl-head {x}")); }
        for x in rl { report.push(format!("   route lbl-head {x}")); }
        for x in dl { report.push(format!("   scene!=route {x}")); }
        for x in jl { report.push(format!("   {x}")); }
    }
}

fn main() {
    let mut report = Vec::new();
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        if std::env::var("ONLYB7").is_err() { check(&name, &src, &mut report); }
        if let Some(g) = graph_of(&src) { let sc = scene::scene(&g); b7(&name, &g, &sc, &mut report);
            if std::env::var("QDET").is_ok() { let m = flowmaid::quality::measure(&g, &sc); for d in &m.details { report.push(format!("   Q static {d}")); }
                let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect(); let r = scene::route(&g, &at); let m = flowmaid::quality::measure(&g, &r); for d in &m.details { report.push(format!("   Q routed {d}")); } } }
    }
    for r in report {
        println!("{r}");
    }
}
