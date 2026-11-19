// Verifier scratch probe (round 2): public API only, builds against pre and now.
//   vp nodes <file>                 node rects + clusters in SVG coordinates (static scene)
//   vp svg <mode> <file> [node dx dy]  write SVG of mode scene|route|partial (optionally one node moved)
//   vp check <files...>             interactive contract per file (static, route, drags, partial)
//   vp sig <files...>               hashes: svg static, svg routed, to_mermaid, geometry
//   vp rp <files...>                route_partial timing (worst frame per file)
use flowmaid::model::{Document, EdgeKind, Graph};
use flowmaid::scene::{self, Hit, Scene, SceneEdge};
use std::collections::BTreeMap;

type Pt = (f64, f64);

fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
fn seg_dist(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1e-12 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}
fn poly_dist(p: Pt, l: &[Pt]) -> f64 {
    if l.len() == 1 {
        return dist(p, l[0]);
    }
    l.windows(2).map(|w| seg_dist(p, w[0], w[1])).fold(f64::INFINITY, f64::min)
}
fn cubic(b: &[Pt; 4], t: f64) -> Pt {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    (w[0] * b[0].0 + w[1] * b[1].0 + w[2] * b[2].0 + w[3] * b[3].0, w[0] * b[0].1 + w[1] * b[1].1 + w[2] * b[2].1 + w[3] * b[3].1)
}
fn scene_line(e: &SceneEdge) -> Vec<Pt> {
    if e.waypoints.len() >= 2 {
        e.waypoints.clone()
    } else {
        (0..=48).map(|k| cubic(&e.bezier, k as f64 / 48.0)).collect()
    }
}
fn drawn(e: &SceneEdge) -> bool {
    !matches!(e.kind, EdgeKind::Invisible) || e.meta.style.stroke_width.map_or(false, |w| w > 0.0)
}

#[derive(Clone, Debug)]
struct Hop {
    y: f64,
    lo: f64,
    hi: f64,
    r: f64,
    top: f64,
    bad: Option<String>,
}
struct Painted {
    edge: usize,
    width: f64,
    pts: Vec<Pt>,   // samples, 0.5 px
    hopf: Vec<bool>, // sample on a hop (arcs + flat top)
    hops: Vec<Hop>,
    holes: Vec<(Pt, f64)>,
    dashed: bool,
}

/// Samples of a path d (absolute M L Q C A) and its hops. step in px.
fn sample_d(d: &str, step: f64) -> (Vec<Pt>, Vec<bool>, Vec<Hop>) {
    let toks: Vec<&str> = d.split([' ', ',']).filter(|t| !t.is_empty()).collect();
    let num = |i: usize| toks[i].parse::<f64>().unwrap();
    let (mut out, mut flag, mut hops) = (Vec::<Pt>::new(), Vec::<bool>::new(), Vec::<Hop>::new());
    let mut cur = (0.0, 0.0);
    let mut i = 0;
    let mut open: Option<(Pt, f64, Pt)> = None; // foot, r, crown-start
    let mut carry: Option<String> = None;
    while i < toks.len() {
        match toks[i] {
            "M" => {
                cur = (num(i + 1), num(i + 2));
                out.push(cur);
                flag.push(false);
                i += 3;
            }
            "L" => {
                let p = (num(i + 1), num(i + 2));
                let n = (dist(cur, p) / step).ceil().max(1.0) as usize;
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    out.push((cur.0 + (p.0 - cur.0) * t, cur.1 + (p.1 - cur.1) * t));
                    flag.push(open.is_some());
                }
                cur = p;
                i += 3;
            }
            "Q" => {
                let (c, p) = ((num(i + 1), num(i + 2)), (num(i + 3), num(i + 4)));
                let n = ((dist(cur, c) + dist(c, p)) / step).ceil().max(8.0) as usize;
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    let u = 1.0 - t;
                    out.push((u * u * cur.0 + 2.0 * u * t * c.0 + t * t * p.0, u * u * cur.1 + 2.0 * u * t * c.1 + t * t * p.1));
                    flag.push(open.is_some());
                }
                cur = p;
                i += 5;
            }
            "C" => {
                let b = [cur, (num(i + 1), num(i + 2)), (num(i + 3), num(i + 4)), (num(i + 5), num(i + 6))];
                let len = dist(b[0], b[1]) + dist(b[1], b[2]) + dist(b[2], b[3]);
                let n = (len / step).ceil().max(8.0) as usize;
                for k in 1..=n {
                    out.push(cubic(&b, k as f64 / n as f64));
                    flag.push(open.is_some());
                }
                cur = b[3];
                i += 7;
            }
            "A" => {
                let (r, sweep) = (num(i + 1), num(i + 5) == 1.0);
                let end = (num(i + 6), num(i + 7));
                let (hx, hy) = ((cur.0 - end.0) / 2.0, (cur.1 - end.1) / 2.0);
                let rr = r.max(hx.hypot(hy));
                let k = ((rr * rr - hx * hx - hy * hy).max(0.0) / (hx * hx + hy * hy)).sqrt();
                let k = if sweep { k } else { -k };
                let c = (k * hy + (cur.0 + end.0) / 2.0, -k * hx + (cur.1 + end.1) / 2.0);
                let a0 = (cur.1 - c.1).atan2(cur.0 - c.0);
                let mut da = (end.1 - c.1).atan2(end.0 - c.0) - a0;
                if sweep && da < 0.0 {
                    da += std::f64::consts::TAU;
                } else if !sweep && da > 0.0 {
                    da -= std::f64::consts::TAU;
                }
                for j in 1..=24 {
                    let a = a0 + da * j as f64 / 24.0;
                    out.push((c.0 + rr * a.cos(), c.1 + rr * a.sin()));
                    flag.push(true);
                }
                match open.take() {
                    None => {
                        // rising quarter: from foot (on the line) to crown start
                        let mut bad = None;
                        if (num(i + 1) - num(i + 2)).abs() > 1e-9 || num(i + 3) != 0.0 || num(i + 4) != 0.0 {
                            bad = Some("arc flags".to_string());
                        }
                        if ((cur.0 - end.0).abs() - r).abs() > 0.16 || ((cur.1 - end.1) - r).abs() > 0.16 {
                            bad = Some(format!("rising arc not a quarter turn up: {:?}->{:?} r {r} dev={}", cur, end, (((cur.0 - end.0).abs() - r).abs()).max(((cur.1 - end.1) - r).abs())));
                        }
                        open = Some((cur, r, end));
                        carry = bad;
                    }
                    Some((foot, r0, crown)) => {
                        let mut bad = carry.take();
                        if (r - r0).abs() > 1e-9 {
                            bad = Some("radii differ".to_string());
                        }
                        if ((cur.0 - end.0).abs() - r).abs() > 0.16 || ((end.1 - cur.1) - r).abs() > 0.16 {
                            bad = Some(format!("falling arc not a quarter turn down: {:?}->{:?} r {r} dev={}", cur, end, (((cur.0 - end.0).abs() - r).abs()).max(((end.1 - cur.1) - r).abs())));
                        }
                        if (cur.1 - crown.1).abs() > 0.05 {
                            bad = Some("top not flat".to_string());
                        }
                        if (end.1 - foot.1).abs() > 1.05 {
                            bad = Some(format!("feet on different lines {} {}", foot.1, end.1));
                        }
                        hops.push(Hop { y: foot.1.max(end.1), lo: foot.0.min(end.0), hi: foot.0.max(end.0), r, top: crown.1, bad });
                    }
                }
                cur = end;
                i += 8;
            }
            other => panic!("unknown path token {other} in {d}"),
        }
    }
    if open.is_some() {
        hops.push(Hop { y: 0.0, lo: 0.0, hi: 0.0, r: 0.0, top: 0.0, bad: Some("hop never comes down".into()) });
    }
    (out, flag, hops)
}

struct SvgEdge {
    d: String,
    width: f64,
    mask: Option<String>,
    dashed: bool,
}
fn svg_edges(svg: &str) -> (Vec<SvgEdge>, BTreeMap<String, Vec<(Pt, f64)>>) {
    let mut masks: BTreeMap<String, Vec<(Pt, f64)>> = BTreeMap::new();
    for chunk in svg.split("<mask id=\"").skip(1) {
        let id = chunk.split('"').next().unwrap().to_string();
        let body = chunk.split("</mask>").next().unwrap();
        let mut v = Vec::new();
        for c in body.split("<circle cx=\"").skip(1) {
            let cx: f64 = c.split('"').next().unwrap().parse().unwrap();
            let cy: f64 = c.split("cy=\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap();
            let r: f64 = c.split("r=\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap();
            v.push(((cx, cy), r));
        }
        masks.insert(id, v);
    }
    let mut out = Vec::new();
    for chunk in svg.split("<path d=\"").skip(1) {
        let Some(q) = chunk.find('"') else { continue };
        let d = &chunk[..q];
        let rest = &chunk[q..];
        let Some(gt) = rest.find('>') else { continue };
        if !rest[gt..].starts_with("><title>") {
            continue;
        }
        let attrs = &rest[..gt];
        let width = attrs.split(" stroke-width=\"").nth(1).and_then(|s| s.split('"').next()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(2.0);
        let mask = attrs.split("mask=\"url(#").nth(1).and_then(|s| s.split(')').next()).map(|s| s.to_string());
        out.push(SvgEdge { d: d.to_string(), width, mask, dashed: attrs.contains("stroke-dasharray") });
    }
    (out, masks)
}

/// Painted strokes in scene coordinates + the svg shift.
fn painted(sc: &Scene, svg: &str) -> Result<(Vec<Painted>, Pt), String> {
    let (se, masks) = svg_edges(svg);
    let dr: Vec<usize> = (0..sc.edges.len()).filter(|&i| drawn(&sc.edges[i])).collect();
    if dr.len() != se.len() {
        return Err(format!("{} svg edge paths for {} drawn scene edges", se.len(), dr.len()));
    }
    let sampled: Vec<(Vec<Pt>, Vec<bool>, Vec<Hop>)> = se.iter().map(|e| sample_d(&e.d, 0.5)).collect();
    // shift: vote on (path start - scene start), check by fit
    let mut votes: BTreeMap<(i64, i64), usize> = BTreeMap::new();
    for (k, &i) in dr.iter().enumerate() {
        let l = scene_line(&sc.edges[i]);
        let p = &sampled[k].0;
        for (a, b) in [(p[0], l[0]), (p[p.len() - 1], l[l.len() - 1])] {
            *votes.entry((((a.0 - b.0) * 10.0).round() as i64, ((a.1 - b.1) * 10.0).round() as i64)).or_default() += 1;
        }
    }
    let mut c: Vec<_> = votes.into_iter().collect();
    c.sort_by(|a, b| b.1.cmp(&a.1));
    c.truncate(6);
    let fit = |t: (i64, i64)| -> usize {
        let (tx, ty) = (t.0 as f64 / 10.0, t.1 as f64 / 10.0);
        let mut n = 0;
        for (k, &i) in dr.iter().enumerate() {
            let l = scene_line(&sc.edges[i]);
            n += sampled[k].0.iter().step_by(16).filter(|q| poly_dist((q.0 - tx, q.1 - ty), &l) < 0.6).count();
        }
        n
    };
    let best = c.iter().map(|x| x.0).max_by_key(|&t| fit(t)).unwrap_or((0, 0));
    let (tx, ty) = (best.0 as f64 / 10.0, best.1 as f64 / 10.0);
    let mut out = Vec::new();
    for (k, &i) in dr.iter().enumerate() {
        let (p, f, h) = &sampled[k];
        out.push(Painted {
            edge: i,
            width: se[k].width,
            pts: p.iter().map(|q| (q.0 - tx, q.1 - ty)).collect(),
            hopf: f.clone(),
            hops: h.iter().map(|h| Hop { y: h.y - ty, lo: h.lo - tx, hi: h.hi - tx, top: h.top - ty, ..h.clone() }).collect(),
            holes: se[k].mask.as_ref().and_then(|m| masks.get(m)).map(|v| v.iter().map(|&(c, r)| ((c.0 - tx, c.1 - ty), r)).collect()).unwrap_or_default(),
            dashed: se[k].dashed,
        });
    }
    Ok((out, (tx, ty)))
}

#[derive(Default, Debug, Clone)]
struct St {
    scenes: usize,
    edges: usize,
    hops: usize,
    holes: usize,
    probes_a: usize,
    probes_b: usize,
    a_none: usize,       // painted point picks nothing
    a_other: usize,      // painted point picks an edge not painted there
    b_phantom: usize,    // pick answers an edge with no ink (nor removed ink) there
    b_missed: usize,     // ink there, pick answers nothing
    hop_shape: usize,
    hop_dev35: usize,
    hop_dev60: usize,
    hop_dev_max: f64,
    hop_twin: usize,
    hop_foot_x: usize,
    hop_no_crossing: usize,
    hop_node: usize,
    hop_label: usize,
    hop_frame: usize,
    hop_ink: usize,
    hop_head: usize,
    hop_gap_near: usize, // mask hole of the same edge within 12 px of its own hop
    sharp: usize,
    off_path: f64,       // worst painted (non-hop) distance from the scene's own line
    geom_arc: usize,
    idx: usize,
    nondet: usize,
    dash_hops: usize,
    cross_total: usize,
    cross_hop: usize,
    cross_gap: usize,
    cross_plain: usize,
}

fn node_rect(n: &scene::SceneNode) -> (f64, f64, f64, f64) {
    (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0)
}

fn check_scene(g: &Graph, sc: &Scene, tag: &str, fails: &mut Vec<String>, st: &mut St, grid: bool) {
    st.scenes += 1;
    macro_rules! fail {
        ($c:expr, $m:expr) => {{
            $c += 1;
            if $c <= 3 {
                let m: String = $m;
                fails.push(format!("{tag}: {m}"));
            }
        }};
    }
    // index-parallel
    if sc.nodes.len() != g.nodes.len() || sc.edges.len() < g.edges.len() {
        fail!(st.idx, format!("sizes: {} nodes/{} edges for graph {}/{}", sc.nodes.len(), sc.edges.len(), g.nodes.len(), g.edges.len()));
        return;
    }
    for (i, n) in sc.nodes.iter().enumerate() {
        if n.id != g.nodes[i].id || ![n.x, n.y, n.w, n.h].iter().all(|v| v.is_finite()) {
            fail!(st.idx, format!("node {i} mismatch / not finite"));
        }
    }
    for (i, ge) in g.edges.iter().enumerate() {
        let e = &sc.edges[i];
        if e.from != g.nodes[ge.from].id || e.to != g.nodes[ge.to].id || e.kind != ge.kind {
            fail!(st.idx, format!("edge {i} {}->{} is not graph edge {}->{}", e.from, e.to, g.nodes[ge.from].id, g.nodes[ge.to].id));
        }
    }
    st.edges += sc.edges.len();
    for (i, e) in sc.edges.iter().enumerate() {
        if !e.bezier.iter().chain(e.waypoints.iter()).all(|p| p.0.is_finite() && p.1.is_finite()) {
            fail!(st.idx, format!("edge {i} not finite"));
        }
        let w = &e.waypoints;
        for j in 1..w.len().saturating_sub(1) {
            let (a, c, b) = (w[j - 1], w[j], w[j + 1]);
            let (u, v) = ((c.0 - a.0, c.1 - a.1), (b.0 - c.0, b.1 - c.1));
            let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
            if lu < 1e-6 || lv < 1e-6 {
                continue;
            }
            let cos = (u.0 * v.0 + u.1 * v.1) / (lu * lv);
            if cos < 0.5 && (lu > 14.05 || lv > 14.05) {
                fail!(st.sharp, format!("edge {i} {}->{} corner at ({:.1},{:.1}) not flanked: legs {:.1}/{:.1}", e.from, e.to, c.0, c.1, lu, lv));
            }
        }
    }
    let svg = scene::to_svg(sc);
    if svg != scene::to_svg(sc) {
        fail!(st.nondet, "to_svg differs between two calls".into());
    }
    let (pa, _) = match painted(sc, &svg) {
        Ok(x) => x,
        Err(m) => {
            fail!(st.idx, m);
            return;
        }
    };
    let lines: Vec<Vec<Pt>> = sc.edges.iter().map(scene_line).collect();
    let by_edge: BTreeMap<usize, &Painted> = pa.iter().map(|p| (p.edge, p)).collect();
    // ink of edge j near p: painted polyline distance; "removed ink": scene line within a mask hole or near an end (head trim)
    let ink_dist = |j: usize, p: Pt| -> f64 { by_edge.get(&j).map_or(f64::INFINITY, |o| poly_dist(p, &o.pts)) };
    let removed_ok = |j: usize, p: Pt, slack: f64| -> bool {
        let Some(o) = by_edge.get(&j) else { return false };
        let l = &lines[j];
        if poly_dist(p, l) > o.width / 2.0 + slack + 4.5 {
            return false;
        }
        // near an end: trimmed under a head (11 px head + grown heads up to ~20) or an end squared by the painter
        let near_end = dist(p, l[0]) < 26.0 || dist(p, l[l.len() - 1]) < 26.0;
        let in_hole = o.holes.iter().any(|&(c, r)| dist(p, c) <= r + o.width / 2.0 + slack + 0.2);
        near_end || in_hole
    };
    // (a) every painted sample picks its edge or one inked there
    for p in &pa {
        let e = &sc.edges[p.edge];
        st.hops += p.hops.len();
        st.holes += p.holes.len();
        if p.dashed && !p.hops.is_empty() {
            st.dash_hops += p.hops.len();
        }
        for (k, &q) in p.pts.iter().enumerate() {
            // off-path: non-hop ink within the corner-cut of the scene's own polyline
            if !p.hopf[k] && e.waypoints.len() >= 2 {
                let d = poly_dist(q, &lines[p.edge]);
                if d > st.off_path {
                    st.off_path = d;
                }
            }
            if k % 3 != 0 {
                continue;
            }
            if p.holes.iter().any(|&(c, r)| dist(q, c) <= r + 0.3) {
                continue; // ink removed by the casing: either answer is fine
            }
            st.probes_a += 1;
            match sc.edge_at(q.0, q.1, 0.3) {
                Some(j) if j == p.edge => {}
                Some(j) => {
                    let half = by_edge.get(&j).map_or(1.0, |o| o.width / 2.0);
                    if ink_dist(j, q) > half + 0.3 + 0.2 && !removed_ok(j, q, 0.5) {
                        fail!(st.a_other, format!("painted ({:.1},{:.1}) of edge {} {}->{} picks edge {j} {}->{} whose ink is {:.2} px away", q.0, q.1, p.edge, e.from, e.to, sc.edges[j].from, sc.edges[j].to, ink_dist(j, q)));
                    }
                }
                None => {
                    fail!(st.a_none, format!("painted ({:.1},{:.1}) of edge {} {}->{} (hop part: {}) picks nothing", q.0, q.1, p.edge, e.from, e.to, p.hopf[k]));
                }
            }
        }
    }
    // (b) grid round every hop (and its line under it): a pick answers ink
    let probe_b = |x: f64, y: f64, st: &mut St, fails: &mut Vec<String>| {
        st.probes_b += 1;
        let q = (x, y);
        match sc.edge_at(x, y, 0.0) {
            Some(j) => {
                let half = by_edge.get(&j).map_or(1.0, |o| o.width / 2.0);
                if ink_dist(j, q) > half + 0.2 && !removed_ok(j, q, 0.2) {
                    st.b_phantom += 1;
                    if st.b_phantom <= 4 {
                        fails.push(format!("{tag}: edge_at({:.2},{:.2},0) = edge {j} {}->{} but its ink is {:.2} px away (half width {half})", x, y, sc.edges[j].from, sc.edges[j].to, ink_dist(j, q)));
                    }
                }
            }
            None => {
                // any ink strictly covering the point?
                for o in &pa {
                    let d = poly_dist(q, &o.pts);
                    if d < o.width / 2.0 - 0.2 && !o.holes.iter().any(|&(c, r)| dist(q, c) <= r + 0.3) {
                        st.b_missed += 1;
                        if st.b_missed <= 4 {
                            fails.push(format!("{tag}: edge_at({:.2},{:.2},0) = None but edge {} {}->{} inks it ({:.2} px from its centre line)", x, y, o.edge, sc.edges[o.edge].from, sc.edges[o.edge].to, d));
                        }
                        break;
                    }
                }
            }
        }
    };
    for p in &pa {
        for h in &p.hops {
            if h.bad.is_some() {
                continue;
            }
            if grid {
                let mut y = h.top - 3.0;
                while y <= h.y + 3.0 {
                    let mut x = h.lo - 4.0;
                    while x <= h.hi + 4.0 {
                        probe_b(x, y, st, fails);
                        x += 0.7;
                    }
                    y += 0.7;
                }
            } else {
                // a cross through the hop: its line, its crown height
                let mut x = h.lo - 3.0;
                while x <= h.hi + 3.0 {
                    probe_b(x, h.y, st, fails);
                    probe_b(x, h.top, st, fails);
                    probe_b(x, (h.y + h.top) / 2.0, st, fails);
                    x += 0.9;
                }
            }
        }
    }
    // (c) hop sanity
    let label_boxes: Vec<(usize, (f64, f64, f64, f64))> = sc
        .edges
        .iter()
        .enumerate()
        .filter_map(|(j, o)| {
            o.label.as_ref().map(|(text, c, w)| {
                let n = 1 + text.matches("<br").count() + text.matches('\n').count();
                let h = 11.0 + 8.5 * (n - 1) as f64;
                (j, (c.0 - w / 2.0, c.1 - h, c.0 + w / 2.0, c.1 + h))
            })
        })
        .collect();
    for p in &pa {
        let e = &sc.edges[p.edge];
        for h in &p.hops {
            if let Some(b) = &h.bad {
                let dev = b.split("dev=").nth(1).and_then(|x| x.parse::<f64>().ok()).unwrap_or(9.0);
                if dev > st.hop_dev_max { st.hop_dev_max = dev; }
                if dev > 0.35 { st.hop_dev35 += 1; }
                if dev > 0.6 { st.hop_dev60 += 1; }
                if dev > 0.35 || std::env::var("ALLSHAPE").is_ok() {
                    fail!(st.hop_shape, format!("edge {} {}->{} hop at ({:.1}..{:.1},{:.1}): {b}", p.edge, e.from, e.to, h.lo, h.hi, h.y));
                } else { st.hop_shape += 1; }
                if dev > 1.2 { continue; }
            }
            if (h.y - h.top - h.r).abs() > 1.2 || h.top > h.y {
                fail!(st.hop_shape, format!("edge {} hop at y {:.1} top {:.1} r {:.1}: not r high / not upward", p.edge, h.y, h.top, h.r));
            }
            let half = p.width / 2.0;
            let bb = (h.lo, h.top - half, h.hi, h.y);
            // a crossing under it: some other edge's ink passes through the hop's line between its feet, steeply
            let mut under = 0;
            for o in &pa {
                if o.edge == p.edge {
                    continue;
                }
                let mut hit = false;
                for w in o.pts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if (a.1 - h.y) * (b.1 - h.y) <= 0.0 && a.0.min(b.0) >= h.lo + h.r - 1.5 && a.0.max(b.0) <= h.hi - h.r + 1.5 && (a.1 - b.1).abs() > 1e-9 {
                        hit = true;
                        break;
                    }
                }
                if hit {
                    under += 1;
                }
                // other ink near the raised part of the hop (not a plumb line under it)
                let mut worst = f64::INFINITY;
                for w in o.pts.windows(2).step_by(2) {
                    let (a, b) = (w[0], w[1]);
                    if a.0.max(b.0) < bb.0 - 6.0 || a.0.min(b.0) > bb.2 + 6.0 || a.1.max(b.1) < bb.1 - 6.0 || a.1.min(b.1) > bb.3 + 6.0 {
                        continue;
                    }
                    let plumb_under = (a.0 - b.0).abs() <= 0.35 * (a.1 - b.1).abs() && a.0.min(b.0) > h.lo + h.r - 1.5 && a.0.max(b.0) < h.hi - h.r + 1.5;
                    if plumb_under {
                        continue;
                    }
                    // raised ink of this hop
                    for (k, q) in p.pts.iter().enumerate() {
                        if p.hopf[k] && q.0 >= h.lo - 0.1 && q.0 <= h.hi + 0.1 && q.1 < h.y - 1.5 && q.1 >= h.top - 0.1 {
                            worst = worst.min(seg_dist(*q, a, b));
                        }
                    }
                }
                if worst < half + o.width / 2.0 + 1.0 && o.hops.iter().any(|g| (g.y - h.y).abs() < 0.3 && (g.lo - h.lo).abs() < 0.3 && (g.hi - h.hi).abs() < 0.3) {
                    st.hop_twin += 1;
                } else if worst < half + o.width / 2.0 + 1.0 {
                    let oe = &sc.edges[o.edge];
                    fail!(st.hop_ink, format!("edge {} {}->{} hop at ({:.1}..{:.1},{:.1}) comes {:.1} px from ink of edge {} {}->{}", p.edge, e.from, e.to, h.lo, h.hi, h.y, worst, o.edge, oe.from, oe.to));
                }
            }
            // a foreign stroke crossing the hop's line right at a foot (not under its crown), uncut
            for o in &pa {
                if o.edge == p.edge {
                    continue;
                }
                let mut at: Option<f64> = None;
                for w in o.pts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if (a.1 - h.y) * (b.1 - h.y) <= 0.0 && (a.1 - b.1).abs() > 1e-9 {
                        let x = a.0 + (b.0 - a.0) * (h.y - a.1) / (b.1 - a.1);
                        let foot = (x >= h.lo - 2.5 && x <= h.lo + h.r - 1.5) || (x <= h.hi + 2.5 && x >= h.hi - h.r + 1.5);
                        if foot {
                            at = Some(x);
                            break;
                        }
                    }
                }
                if let Some(x) = at {
                    let cut = p.holes.iter().chain(o.holes.iter()).any(|&(c, r)| dist(c, (x, h.y)) < r + 1.0);
                    let twin = o.hops.iter().any(|g| (g.y - h.y).abs() < 0.3 && (g.lo - h.lo).abs() < 0.3 && (g.hi - h.hi).abs() < 0.3);
                    if !cut && !twin {
                        let oe = &sc.edges[o.edge];
                        fail!(st.hop_foot_x, format!("edge {} {}->{} hop ({:.1}..{:.1},{:.1}) has its foot on the uncut crossing with edge {} {}->{} at x={:.1}", p.edge, e.from, e.to, h.lo, h.hi, h.y, o.edge, oe.from, oe.to, x));
                    }
                }
            }
            if under == 0 {
                fail!(st.hop_no_crossing, format!("edge {} {}->{} hop at ({:.1}..{:.1},{:.1}) steps over nothing", p.edge, e.from, e.to, h.lo, h.hi, h.y));
            }
            for n in &sc.nodes {
                let nr = node_rect(n);
                if bb.0 < nr.2 + 1.0 && bb.2 > nr.0 - 1.0 && bb.1 < nr.3 + 1.0 && bb.3 > nr.1 - 1.0 && (h.y > nr.3 || h.y < nr.1) {
                    fail!(st.hop_node, format!("edge {} {}->{} hop at ({:.1}..{:.1},{:.1}) pokes into node {}", p.edge, e.from, e.to, h.lo, h.hi, h.y, n.id));
                }
            }
            for (j, lr) in &label_boxes {
                if bb.0 < lr.2 && bb.2 > lr.0 && bb.1 < lr.3 && bb.3 > lr.1 {
                    fail!(st.hop_label, format!("edge {} {}->{} hop at ({:.1}..{:.1},{:.1}) into label of edge {j}", p.edge, e.from, e.to, h.lo, h.hi, h.y));
                }
            }
            for c in &sc.clusters {
                for by in [c.y, c.y + c.h] {
                    if by < h.y - 0.5 && by > h.top - half - 1.0 && h.hi > c.x && h.lo < c.x + c.w {
                        fail!(st.hop_frame, format!("edge {} {}->{} hop at ({:.1}..{:.1},{:.1}) reaches frame {} border y={:.1}", p.edge, e.from, e.to, h.lo, h.hi, h.y, c.id, by));
                    }
                }
                for bx in [c.x, c.x + c.w] {
                    if bx > h.lo - 1.0 && bx < h.hi + 1.0 && c.y < h.y && c.y + c.h > h.top && !(bx > h.lo + h.r - 1.5 && bx < h.hi - h.r + 1.5) {
                        fail!(st.hop_frame, format!("edge {} {}->{} hop foot at ({:.1}..{:.1},{:.1}) on frame {} side x={:.1}", p.edge, e.from, e.to, h.lo, h.hi, h.y, c.id, bx));
                    }
                }
            }
            for (j, o) in sc.edges.iter().enumerate() {
                if j == p.edge || !drawn(o) {
                    continue;
                }
                let l = &lines[j];
                let z = l[l.len() - 1];
                if z.0 > bb.0 - 8.0 && z.0 < bb.2 + 8.0 && z.1 > bb.1 - 8.0 && z.1 < bb.3 + 8.0 {
                    fail!(st.hop_head, format!("edge {} hop at ({:.1}..{:.1},{:.1}) sits by the head of edge {j} {}->{} at ({:.1},{:.1})", p.edge, h.lo, h.hi, h.y, o.from, o.to, z.0, z.1));
                }
            }
            for &(c, _) in &p.holes {
                if (c.1 - h.y).abs() < 1.5 && c.0 > h.lo - 12.0 && c.0 < h.hi + 12.0 {
                    fail!(st.hop_gap_near, format!("edge {} {}->{} is cut at ({:.1},{:.1}) within 12 px of its own hop ({:.1}..{:.1})", p.edge, e.from, e.to, c.0, c.1, h.lo, h.hi));
                }
            }
            // the scene's own geometry has no bump there
            let crown = ((h.lo + h.hi) / 2.0, h.top);
            if (poly_dist(crown, &lines[p.edge]) - h.r).abs() > 0.6 {
                fail!(st.geom_arc, format!("edge {} scene path is {:.2} px from its hop's crown (r {})", p.edge, poly_dist(crown, &lines[p.edge]), h.r));
            }
        }
    }
    // crossing census among painted strokes (axis legs only is too narrow: all proper crossings of scene lines)
    let mut total = 0;
    let mut hop = 0;
    let mut gap = 0;
    for a in 0..lines.len() {
        if !drawn(&sc.edges[a]) {
            continue;
        }
        for b in a + 1..lines.len() {
            if !drawn(&sc.edges[b]) {
                continue;
            }
            for xp in crossings(&lines[a], &lines[b]) {
                total += 1;
                let hopped = [a, b].iter().any(|j| by_edge.get(j).map_or(false, |o| o.hops.iter().any(|h| (xp.1 - h.y).abs() < 1.5 && xp.0 > h.lo + 1.0 && xp.0 < h.hi - 1.0)));
                let gapped = [a, b].iter().any(|j| by_edge.get(j).map_or(false, |o| o.holes.iter().any(|&(c, r)| dist(c, xp) < r + 2.5)));
                if hopped {
                    hop += 1;
                } else if gapped {
                    gap += 1;
                }
            }
        }
    }
    st.cross_total += total;
    st.cross_hop += hop;
    st.cross_gap += gap;
    st.cross_plain += total - hop - gap;
}

fn crossings(a: &[Pt], b: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    let bb = |l: &[Pt]| l.iter().fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |m, p| (m.0.min(p.0), m.1.min(p.1), m.2.max(p.0), m.3.max(p.1)));
    let (ba, bbb) = (bb(a), bb(b));
    if ba.0 > bbb.2 || bbb.0 > ba.2 || ba.1 > bbb.3 || bbb.1 > ba.3 {
        return out;
    }
    for s in a.windows(2) {
        for t in b.windows(2) {
            let (p, r) = (s[0], (s[1].0 - s[0].0, s[1].1 - s[0].1));
            let (q, u) = (t[0], (t[1].0 - t[0].0, t[1].1 - t[0].1));
            let den = r.0 * u.1 - r.1 * u.0;
            if den.abs() < 1e-9 {
                continue;
            }
            let tt = ((q.0 - p.0) * u.1 - (q.1 - p.1) * u.0) / den;
            let uu = ((q.0 - p.0) * r.1 - (q.1 - p.1) * r.0) / den;
            if tt > 1e-6 && tt < 1.0 - 1e-6 && uu > 1e-6 && uu < 1.0 - 1e-6 {
                let x = (p.0 + tt * r.0, p.1 + tt * r.1);
                // not at shared ends
                if dist(x, a[0]) < 3.0 || dist(x, a[a.len() - 1]) < 3.0 || dist(x, b[0]) < 3.0 || dist(x, b[b.len() - 1]) < 3.0 {
                    continue;
                }
                if out.iter().all(|&o| dist(o, x) > 6.0) {
                    out.push(x);
                }
            }
        }
    }
    out
}

fn geom_sig(sc: &Scene) -> String {
    let mut s = String::new();
    for n in &sc.nodes {
        s.push_str(&format!("{}:{:.3},{:.3},{:.3},{:.3};", n.id, n.x, n.y, n.w, n.h));
    }
    for e in &sc.edges {
        s.push_str(&format!("{}>{}:{:?}|{:?}|{:?};", e.from, e.to, e.bezier, e.waypoints, e.label));
    }
    for c in &sc.clusters {
        s.push_str(&format!("{}:{:.3},{:.3},{:.3},{:.3};", c.id, c.x, c.y, c.w, c.h));
    }
    s
}

fn busiest(g: &Graph, k: usize) -> Vec<usize> {
    let mut deg = vec![0usize; g.nodes.len()];
    for e in &g.edges {
        deg[e.from] += 1;
        deg[e.to] += 1;
    }
    let mut v: Vec<usize> = (0..deg.len()).collect();
    v.sort_by_key(|&i| (std::cmp::Reverse(deg[i]), i));
    v.truncate(k);
    v
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args[0].as_str();
    match mode {
        "nodes" => {
            let src = std::fs::read_to_string(&args[1]).unwrap();
            let g = graph_of(&src).unwrap();
            let sc = if args.get(2).map(|s| s.as_str()) == Some("route") {
                let s = scene::scene(&g);
                let at: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
                scene::route(&g, &at)
            } else {
                scene::scene(&g)
            };
            let svg = scene::to_svg(&sc);
            let (_, (tx, ty)) = painted(&sc, &svg).unwrap();
            for n in &sc.nodes {
                println!("node {} {:.0} {:.0} {:.0} {:.0}", n.id, n.x - n.w / 2.0 + tx, n.y - n.h / 2.0 + ty, n.w, n.h);
            }
            for c in &sc.clusters {
                println!("cluster {} {:.0} {:.0} {:.0} {:.0}", c.id, c.x + tx, c.y + ty, c.w, c.h);
            }
        }
        "svg" => {
            let src = std::fs::read_to_string(&args[2]).unwrap();
            let g = graph_of(&src).unwrap();
            let s = scene::scene(&g);
            let mut at: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let base = at.clone();
            if args.len() >= 6 {
                let v = g.nodes.iter().position(|n| n.id == args[3]).expect("node id");
                at[v].0 += args[4].parse::<f64>().unwrap();
                at[v].1 += args[5].parse::<f64>().unwrap();
            }
            let sc = match args[1].as_str() {
                "scene" => s,
                "route" => scene::route(&g, &at),
                "partial" => scene::route_partial(&g, &at, &s, &base),
                _ => panic!("mode"),
            };
            print!("{}", scene::to_svg(&sc));
        }
        "sig" => {
            for f in &args[1..] {
                let src = std::fs::read_to_string(f).unwrap();
                let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
                let Some(g) = graph_of(&src) else {
                    println!("{name}\tnot-a-graph");
                    continue;
                };
                let s = scene::scene(&g);
                let at: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
                let r = scene::route(&g, &at);
                let mm = flowmaid::emit::to_mermaid(&g);
                let rt = graph_of(&mm).map(|g2| flowmaid::emit::to_mermaid(&g2));
                println!(
                    "{name}\tsvg={:016x}\trsvg={:016x}\tgeom={:016x}\trgeom={:016x}\tmm={:016x}\troundtrip={}",
                    hash(&scene::to_svg(&s)),
                    hash(&scene::to_svg(&r)),
                    hash(&geom_sig(&s)),
                    hash(&geom_sig(&r)),
                    hash(&mm),
                    rt.as_deref() == Some(mm.as_str())
                );
            }
        }
        "rp" => {
            for f in &args[1..] {
                let src = std::fs::read_to_string(f).unwrap();
                let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
                let Some(g) = graph_of(&src) else { continue };
                let s = scene::scene(&g);
                let base: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
                let frames = flowmaid::quality::drag_frames(&s);
                let (mut worst, mut sum, mut n, mut wv) = (0.0f64, 0.0, 0, 0);
                let mut all: Vec<f64> = Vec::new();
                for (v, c) in &frames {
                    let mut best = f64::INFINITY;
                    for _ in 0..std::env::var("REPS").ok().and_then(|v| v.parse().ok()).unwrap_or(3usize) {
                        let t = std::time::Instant::now();
                        let out = scene::route_partial(&g, c, &s, &base);
                        std::hint::black_box(&out);
                        best = best.min(t.elapsed().as_secs_f64() * 1e3);
                    }
                    all.push(best);
                    if let Ok(d) = std::env::var("DUMP") {
                        use std::io::Write;
                        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(d).unwrap();
                        writeln!(f, "{name}\t{}\t{}\t{best:.4}", n, g.nodes[*v].id).unwrap();
                    }
                    if best > worst {
                        worst = best;
                        wv = *v;
                    }
                    sum += best;
                    n += 1;
                }
                all.sort_by(|a, b| a.total_cmp(b));
                let q = |f: f64| all[((all.len() as f64 - 1.0) * f) as usize];
                println!("{name}\tframes={n}\tworst_ms={worst:.2}\tp99={:.2}\tp95={:.2}\tover6={}\tmean_ms={:.2}\tworst_node={}", q(0.99), q(0.95), all.iter().filter(|&&x| x > 6.0).count(), sum / n.max(1) as f64, g.nodes[wv].id);
            }
        }
        "swap" => {
            // vp swap <file> <toA> <toB> [route] : swap the ports of the two edges ending at toA / toB on their shared source side
            let src = std::fs::read_to_string(&args[1]).unwrap();
            let g = graph_of(&src).unwrap();
            let s0 = scene::scene(&g);
            let mut sc = if args.get(4).map(|s| s.as_str()) == Some("route") {
                let at: Vec<Pt> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
                scene::route(&g, &at)
            } else {
                s0
            };
            let show = |tag: &str, sc: &Scene| {
                let m = flowmaid::quality::measure(&g, sc);
                println!("{tag}: fan_x={} x_drawn={} edge_edge_x={} defects={} overlap_pairs={} tight_pairs={} tight_len={:.0} band_pairs={} edge_node_pairs={} lbl_edge={} end_kink={} fan_uneven={} wiggles={}", m.fan_x, m.x_drawn, m.edge_edge_x, m.defects(), m.overlap_pairs, m.tight_pairs, m.tight_len, m.band_pairs, m.edge_node_pairs, m.lbl_edge, m.end_kink, m.fan_uneven, m.wiggles);
            };
            show("before", &sc);
            let ia = sc.edges.iter().position(|e| e.to == args[2]).unwrap();
            let ib = sc.edges.iter().position(|e| e.to == args[3]).unwrap();
            let (pa, pb) = (sc.edges[ia].waypoints[0].0, sc.edges[ib].waypoints[0].0);
            for (i, from, to) in [(ia, pa, pb), (ib, pb, pa)] {
                for w in sc.edges[i].waypoints.iter_mut() {
                    if (w.0 - from).abs() < 1e-6 {
                        w.0 = to;
                    }
                }
                let w0 = sc.edges[i].waypoints[0];
                sc.edges[i].bezier[0] = w0;
            }
            show("after swap", &sc);
            if let Some(out) = args.get(5) {
                std::fs::write(out, scene::to_svg(&sc)).unwrap();
            }
        }
        "pk" => {
            for f in &args[1..] {
                let src = std::fs::read_to_string(f).unwrap();
                let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
                let Some(g) = graph_of(&src) else { continue };
                let s = scene::scene(&g);
                let mut pts: Vec<Pt> = Vec::new();
                for e in &s.edges {
                    let l = scene_line(e);
                    for w in l.windows(2) {
                        let n = (dist(w[0], w[1]) / 3.0).ceil().max(1.0) as usize;
                        for k in 0..n {
                            let t = k as f64 / n as f64;
                            pts.push((w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t));
                        }
                    }
                }
                // off-stroke points too: a grid over the canvas
                let (mut x1, mut y1) = (0.0f64, 0.0f64);
                for n in &s.nodes { x1 = x1.max(n.x + n.w); y1 = y1.max(n.y + n.h); }
                let mut grid: Vec<Pt> = Vec::new();
                let mut y = 0.0; while y < y1 { let mut x = 0.0; while x < x1 { grid.push((x, y)); x += 23.0; } y += 19.0; }
                let time = |v: &Vec<Pt>| {
                    let mut best = f64::INFINITY;
                    for _ in 0..7 {
                        let t = std::time::Instant::now();
                        let mut c = 0usize;
                        for p in v { if s.hit_test(p.0, p.1, 3.0).is_some() { c += 1; } }
                        std::hint::black_box(c);
                        best = best.min(t.elapsed().as_secs_f64() * 1e6 / v.len() as f64);
                    }
                    best
                };
                println!("{name}\ton_stroke_us={:.2} ({} pts)\tgrid_us={:.2} ({} pts)", time(&pts), pts.len(), time(&grid), grid.len());
            }
        }
        "check" => {
            let grid = std::env::var("GRID").is_ok();
            let mut tot = St::default();
            for f in &args[1..] {
                let src = std::fs::read_to_string(f).unwrap();
                let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
                let Some(g) = graph_of(&src) else { continue };
                let mut fails = Vec::new();
                let mut st = St::default();
                let s = scene::scene(&g);
                let s2 = scene::scene(&g);
                if geom_sig(&s) != geom_sig(&s2) || scene::to_svg(&s) != scene::to_svg(&s2) {
                    st.nondet += 1;
                    fails.push(format!("{name}: scene() not deterministic"));
                }
                check_scene(&g, &s, &format!("{name} scene"), &mut fails, &mut st, grid);
                let base: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
                let r = scene::route(&g, &base);
                if geom_sig(&r) != geom_sig(&scene::route(&g, &base)) {
                    st.nondet += 1;
                    fails.push(format!("{name}: route() not deterministic"));
                }
                check_scene(&g, &r, &format!("{name} route"), &mut fails, &mut st, grid);
                let moves: [(f64, f64); 4] = [(180.0, 140.0), (-90.0, 60.0), (37.0, -11.0), (-3.0, 1.0)];
                for (bi, &v) in busiest(&g, 3).iter().enumerate() {
                    for (mi, &(dx, dy)) in moves.iter().enumerate() {
                        if (bi + mi) % 2 == 1 && g.nodes.len() > 60 {
                            continue;
                        }
                        let mut c = base.clone();
                        c[v] = (c[v].0 + dx, c[v].1 + dy);
                        let p = scene::route_partial(&g, &c, &s, &base);
                        if geom_sig(&p) != geom_sig(&scene::route_partial(&g, &c, &s, &base)) {
                            st.nondet += 1;
                            fails.push(format!("{name}: route_partial() not deterministic"));
                        }
                        check_scene(&g, &p, &format!("{name} partial {}{:+}{:+}", g.nodes[v].id, dx, dy), &mut fails, &mut st, grid);
                        if mi == 0 {
                            let rr = scene::route(&g, &c);
                            check_scene(&g, &rr, &format!("{name} route {}{:+}{:+}", g.nodes[v].id, dx, dy), &mut fails, &mut st, grid);
                        }
                    }
                }
                // hit_test priorities stay: a node centre gives the node
                for (i, n) in s.nodes.iter().enumerate() {
                    if s.node_at(n.x, n.y) == Some(i) && !matches!(s.hit_test(n.x, n.y, 3.0), Some(Hit::Node(j)) if j == i) {
                        fails.push(format!("{name}: hit_test at node {} centre gives {:?}", n.id, s.hit_test(n.x, n.y, 3.0)));
                    }
                }
                println!("{name}\t{st:?}");
                for m in fails.iter().take(40) {
                    println!("  FAIL {m}");
                }
                macro_rules! add { ($($f:ident),*) => { $( tot.$f += st.$f; )* } }
                add!(hop_foot_x, hop_dev35, hop_dev60, hop_twin, scenes, edges, hops, holes, probes_a, probes_b, a_none, a_other, b_phantom, b_missed, hop_shape, hop_no_crossing, hop_node, hop_label, hop_frame, hop_ink, hop_head, hop_gap_near, sharp, geom_arc, idx, nondet, dash_hops, cross_total, cross_hop, cross_gap, cross_plain);
                tot.off_path = tot.off_path.max(st.off_path); tot.hop_dev_max = tot.hop_dev_max.max(st.hop_dev_max);
            }
            println!("TOTAL\t{tot:?}");
        }
        _ => panic!("mode"),
    }
}
