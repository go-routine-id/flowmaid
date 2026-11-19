// Interactive-contract probe (verifier scratch; public API only, builds against pre and now).
// contract <mode> <files...>
//   mode = check : per file, static + drags; prints FAIL lines and a summary row
//   mode = fans  : per file fan crossing / uneven-port counts (static | routed)
//   mode = dump  : node rects + hub sides of one file
use flowmaid::model::{Document, Graph, Shape};
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
    (
        w[0] * b[0].0 + w[1] * b[1].0 + w[2] * b[2].0 + w[3] * b[3].0,
        w[0] * b[0].1 + w[1] * b[1].1 + w[2] * b[2].1 + w[3] * b[3].1,
    )
}

/// The scene's own polyline of an edge: waypoints, or the bezier sampled.
fn scene_line(e: &SceneEdge) -> Vec<Pt> {
    if e.waypoints.len() >= 2 {
        e.waypoints.clone()
    } else {
        (0..=32).map(|k| cubic(&e.bezier, k as f64 / 32.0)).collect()
    }
}

fn ends(e: &SceneEdge) -> (Pt, Pt) {
    if e.waypoints.len() >= 2 {
        (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
    } else {
        (e.bezier[0], e.bezier[3])
    }
}

/// One hop found in a painted path: line y, crown x range, radius, arc samples.
#[derive(Clone, Debug)]
struct Arc {
    from: Pt,
    to: Pt,
    r: f64,
    pts: Vec<Pt>,
}

/// Samples along an SVG path `d` (absolute M/L/Q/C/A), step px on lines; and its arcs.
fn sample_d(d: &str, step: f64) -> (Vec<Pt>, Vec<Arc>) {
    let (p, a, _) = sample_d_flag(d, step);
    (p, a)
}

/// [`sample_d`] plus, per sample, whether it lies on a hop (its arcs or the flat top between them).
fn sample_d_flag(d: &str, step: f64) -> (Vec<Pt>, Vec<Arc>, Vec<bool>) {
    let toks: Vec<&str> = d.split([' ', ',']).filter(|t| !t.is_empty()).collect();
    let num = |i: usize| toks[i].parse::<f64>().unwrap();
    let (mut out, mut arcs, mut cur, mut i) = (Vec::new(), Vec::new(), (0.0, 0.0), 0);
    let mut flag: Vec<bool> = Vec::new();
    let mut in_hop = false;
    while i < toks.len() {
        let before = out.len();
        let was_in = in_hop;
        match toks[i] {
            "M" => {
                cur = (num(i + 1), num(i + 2));
                out.push(cur);
                i += 3;
            }
            "L" => {
                let p = (num(i + 1), num(i + 2));
                let n = (dist(cur, p) / step).ceil().max(1.0) as usize;
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    out.push((cur.0 + (p.0 - cur.0) * t, cur.1 + (p.1 - cur.1) * t));
                }
                cur = p;
                i += 3;
            }
            "Q" => {
                let (c, p) = ((num(i + 1), num(i + 2)), (num(i + 3), num(i + 4)));
                for k in 1..=24 {
                    let t = k as f64 / 24.0;
                    let u = 1.0 - t;
                    out.push((
                        u * u * cur.0 + 2.0 * u * t * c.0 + t * t * p.0,
                        u * u * cur.1 + 2.0 * u * t * c.1 + t * t * p.1,
                    ));
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
                let pts: Vec<Pt> = (1..=12)
                    .map(|j| {
                        let a = a0 + da * j as f64 / 12.0;
                        (c.0 + rr * a.cos(), c.1 + rr * a.sin())
                    })
                    .collect();
                out.extend(pts.iter().copied());
                arcs.push(Arc { from: cur, to: end, r, pts });
                cur = end;
                i += 8;
                in_hop = !in_hop;
            }
            other => panic!("unknown path token {other} in {d}"),
        }
        let f = was_in || in_hop;
        flag.extend(std::iter::repeat(f).take(out.len() - before));
    }
    (out, arcs, flag)
}

/// The edge paths of a flowchart SVG in order: `<path d="..." ...><title>`.
fn svg_edge_paths(svg: &str) -> Vec<(String, f64)> {
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
        let sw = attrs
            .split(" stroke-width=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(2.0);
        out.push((d.to_string(), sw));
    }
    out
}

struct Painted {
    edge: usize,
    hop: Vec<bool>,
    pts: Vec<Pt>,
    arcs: Vec<Arc>,
    width: f64,
}

/// Painted edge strokes in scene coordinates, matched to scene edges.
fn painted(sc: &Scene, svg: &str, fails: &mut Vec<String>, tag: &str) -> Vec<Painted> {
    let paths = svg_edge_paths(svg);
    if paths.is_empty() {
        return Vec::new();
    }
    let sampled: Vec<(Vec<Pt>, Vec<Arc>, f64, Vec<bool>)> = paths
        .iter()
        .map(|(d, w)| {
            let (p, a, f) = sample_d_flag(d, 2.0);
            (p, a, *w, f)
        })
        .collect();
    // The shift: most common (path start - scene start) over in-order pairs, to 0.1 px.
    let drawn0: Vec<usize> = (0..sc.edges.len())
        .filter(|&i| {
            let e = &sc.edges[i];
            !matches!(e.kind, flowmaid::model::EdgeKind::Invisible) || e.meta.style.stroke_width.map_or(false, |w| w > 0.0)
        })
        .collect();
    let mut votes: BTreeMap<(i64, i64), usize> = BTreeMap::new();
    for (k, (p, ..)) in sampled.iter().enumerate() {
        let Some(&i) = drawn0.get(k) else { break };
        let (s, z) = ends(&sc.edges[i]);
        for (a, b) in [(p[0], s), (p[p.len() - 1], z)] {
            let key = (((a.0 - b.0) * 10.0).round() as i64, ((a.1 - b.1) * 10.0).round() as i64);
            *votes.entry(key).or_default() += 1;
        }
    }
    // Among the best-voted shifts, the one that lays the most painted samples on their own scene lines.
    let mut cands: Vec<((i64, i64), usize)> = votes.iter().map(|(k, v)| (*k, *v)).collect();
    cands.sort_by(|a, b| b.1.cmp(&a.1));
    cands.truncate(8);
    cands.push(((0, 0), 0));
    let fit = |t: (i64, i64)| -> usize {
        let (tx, ty) = (t.0 as f64 / 10.0, t.1 as f64 / 10.0);
        let mut n = 0;
        for (k, (p, ..)) in sampled.iter().enumerate() {
            let Some(&i) = drawn0.get(k) else { break };
            let l = scene_line(&sc.edges[i]);
            n += p.iter().step_by(4).filter(|q| poly_dist((q.0 - tx, q.1 - ty), &l) < 0.6).count();
        }
        n
    };
    let (tx, ty) = cands.iter().map(|c| c.0).max_by_key(|&t| fit(t)).unwrap();
    let (tx, ty) = (tx as f64 / 10.0, ty as f64 / 10.0);
    let mut out = Vec::new();
    // Drawn edges in order (as to_svg emits them): everything but `~~~` without a stroke width.
    let drawn: Vec<usize> = (0..sc.edges.len())
        .filter(|&i| {
            let e = &sc.edges[i];
            !matches!(e.kind, flowmaid::model::EdgeKind::Invisible) || e.meta.style.stroke_width.map_or(false, |w| w > 0.0)
        })
        .collect();
    if drawn.len() != sampled.len() {
        fails.push(format!("{tag}: {} svg edge paths for {} drawn scene edges", sampled.len(), drawn.len()));
        return out;
    }
    for (k, &i) in drawn.iter().enumerate() {
        let (p, a, w, fl) = &sampled[k];
        out.push(Painted {
            edge: i,
            hop: fl.clone(),
            pts: p.iter().map(|q| (q.0 - tx, q.1 - ty)).collect(),
            arcs: a
                .iter()
                .map(|x| Arc {
                    from: (x.from.0 - tx, x.from.1 - ty),
                    to: (x.to.0 - tx, x.to.1 - ty),
                    r: x.r,
                    pts: x.pts.iter().map(|q| (q.0 - tx, q.1 - ty)).collect(),
                })
                .collect(),
            width: *w,
        });
    }
    out
}

fn node_rect(n: &scene::SceneNode) -> (f64, f64, f64, f64) {
    (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0)
}

#[derive(Default, Clone, Debug)]
struct Stats {
    edges: usize,
    hops: usize,
    samples: usize,
    pick_miss: usize,
    hop_pick_miss: usize,
    hop_under_wrong: usize,
    sharp: usize,
    off_path: usize,
    hop_node: usize,
    hop_label: usize,
    hop_frame: usize,
    hop_edge: usize,
    hop_down: usize,
    hop_head: usize,
    masks: usize,
    hop_paint_unpicked: usize,
    hop_tight_node: usize,
}

/// All the per-scene checks. `g` gives the index-parallel contract.
fn check_scene(g: &Graph, sc: &Scene, tag: &str, fails: &mut Vec<String>, st: &mut Stats) {
    // 1. index-parallel nodes / edges, finite numbers.
    if sc.nodes.len() != g.nodes.len() {
        fails.push(format!("{tag}: {} scene nodes for {} graph nodes", sc.nodes.len(), g.nodes.len()));
        return;
    }
    for (i, n) in sc.nodes.iter().enumerate() {
        if n.id != g.nodes[i].id {
            fails.push(format!("{tag}: node {i} id {} != {}", n.id, g.nodes[i].id));
        }
        if ![n.x, n.y, n.w, n.h].iter().all(|v| v.is_finite()) {
            fails.push(format!("{tag}: node {i} not finite"));
        }
    }
    if sc.edges.len() < g.edges.len() {
        fails.push(format!("{tag}: {} scene edges < {} graph edges", sc.edges.len(), g.edges.len()));
        return;
    }
    for (i, ge) in g.edges.iter().enumerate() {
        let e = &sc.edges[i];
        if e.from != g.nodes[ge.from].id || e.to != g.nodes[ge.to].id {
            fails.push(format!("{tag}: edge {i} {}->{} is not graph edge {}->{}", e.from, e.to, g.nodes[ge.from].id, g.nodes[ge.to].id));
        }
        if e.kind != ge.kind {
            fails.push(format!("{tag}: edge {i} kind differs"));
        }
    }
    for (i, e) in sc.edges.iter().enumerate() {
        if !e.bezier.iter().chain(e.waypoints.iter()).all(|p| p.0.is_finite() && p.1.is_finite()) {
            fails.push(format!("{tag}: edge {i} not finite"));
        }
        // 2. corners delivered rounded: a waypoint where the path turns
        // sharply has both neighbours within CORNER_R (router::smooth).
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
                st.sharp += 1;
                if st.sharp <= 3 {
                    fails.push(format!("{tag}: edge {i} {}->{} sharp corner at ({:.1},{:.1}) legs {:.1}/{:.1}", e.from, e.to, c.0, c.1, lu, lv));
                }
            }
        }
        // ends of waypoints agree with from/to nodes' boxes (within 2 px of the box)
    }
    st.edges += sc.edges.len();
    // 3. painted strokes vs scene geometry and picking.
    let svg = scene::to_svg(sc);
    st.masks += svg.matches("<mask").count();
    let pa = painted(sc, &svg, fails, tag);
    let lines: Vec<Vec<Pt>> = sc.edges.iter().map(scene_line).collect();
    let by_edge: BTreeMap<usize, &Painted> = pa.iter().map(|p| (p.edge, p)).collect();
    for p in &pa {
        let e = &sc.edges[p.edge];
        let arc_pts: Vec<Pt> = p.arcs.iter().flat_map(|a| a.pts.iter().copied()).collect();
        for (k, &q) in p.pts.iter().enumerate() {
            if k % 2 == 1 {
                continue;
            }
            st.samples += 1;
            // painted ink lies on the scene path (rounded corners cut <= ~4.2 px; hops <= r)
            let on_arc = p.hop[k] || arc_pts.iter().any(|&a| dist(a, q) < 1e-6);
            let d = poly_dist(q, &lines[p.edge]);
            let lim = if on_arc { 9.5 } else { 4.5 };
            if e.waypoints.len() >= 2 && d > lim {
                st.off_path += 1;
                if st.off_path <= 3 {
                    fails.push(format!("{tag}: edge {} {}->{} painted ({:.1},{:.1}) is {:.1} px off its scene path", p.edge, e.from, e.to, q.0, q.1, d));
                }
            }
            // pick == render: the painted point picks this edge, or one painted within reach.
            match sc.edge_at(q.0, q.1, 0.75) {
                Some(j) if j == p.edge => {}
                Some(j) => {
                    let near = by_edge.get(&j).map_or(f64::INFINITY, |o| poly_dist(q, &o.pts)).min(poly_dist(q, &lines[j]) - 3.0);
                    if near > 0.75 + o_half(by_edge.get(&j)) + 0.6 {
                        st.pick_miss += 1;
                        if st.pick_miss <= 3 {
                            fails.push(format!("{tag}: pick at painted ({:.1},{:.1}) of edge {} gives edge {j}, painted {:.1} px away", q.0, q.1, p.edge, near));
                        }
                    }
                }
                None => {
                    // under a node? (a node covers it: hit_test would give the node)
                    if sc.node_at(q.0, q.1).is_none() {
                        st.pick_miss += 1;
                        if p.hop[k] {
                            st.hop_paint_unpicked += 1;
                        }
                        if st.pick_miss <= 3 {
                            fails.push(format!("{tag}: painted ({:.1},{:.1}) of edge {} {}->{} picks nothing", q.0, q.1, p.edge, e.from, e.to));
                        }
                    }
                }
            }
        }
        // 4. hops: arcs come in pairs (up, [flat], down).
        let mut k = 0;
        while k + 1 < p.arcs.len() {
            let (up, down) = (&p.arcs[k], &p.arcs[k + 1]);
            k += 2;
            st.hops += 1;
            let y = up.from.1;
            let r = up.r;
            let (xa, xb) = (up.from.0.min(down.to.0), up.from.0.max(down.to.0));
            let top = y - r;
            let y = y.max(down.to.1);
            let top = up.to.1;
            if (down.to.1 - up.from.1).abs() > 1.01 || up.pts.iter().chain(&down.pts).any(|q| q.1 > y + 0.3) || (y - r - top).abs() > 1.01 || (down.from.1 - top).abs() > 0.05 {
                st.hop_down += 1;
                fails.push(format!("{tag}: edge {} hop at ({:.1},{:.1}) is not an upward hop: {:?} {:?}", p.edge, xa, y, up, down));
                continue;
            }
            // pick on the arc picks the hopping edge (or the plumb line it crosses at the crown)
            for a in [up, down] {
                let q = a.pts[5]; // mid-arc, 45 degrees up
                match sc.edge_at(q.0, q.1, 0.3) {
                    Some(j) if j == p.edge => {}
                    other => {
                        st.hop_pick_miss += 1;
                        if st.hop_pick_miss <= 3 {
                            fails.push(format!("{tag}: mid-arc ({:.1},{:.1}) of edge {}'s hop picks {:?}", q.0, q.1, p.edge, other));
                        }
                    }
                }
            }
            // the marquee over the crown holds the hopping edge
            let cx = up.to.0;
            let hits = sc.hits_in_rect(cx - 1.0, top - 0.5, cx + 1.0, top + 0.5);
            if !hits.contains(&Hit::Edge(p.edge)) {
                st.hop_pick_miss += 1;
                if st.hop_pick_miss <= 3 {
                    fails.push(format!("{tag}: marquee on the crown ({:.1},{:.1}) of edge {} misses it: {:?}", cx, top, p.edge, hits));
                }
            }
            // on the line under the hop (no ink of this edge): never this edge at tol 0
            let under = (up.from.0 + (up.to.0 - up.from.0) * 0.5, y);
            if sc.edge_at(under.0, under.1, 0.0) == Some(p.edge) {
                st.hop_under_wrong += 1;
                if st.hop_under_wrong <= 3 {
                    fails.push(format!("{tag}: under the hop ({:.1},{:.1}) still picks edge {}", under.0, under.1, p.edge));
                }
            }
            // collisions of the hop's ink: nodes, labels, frame borders, other strokes.
            let half = p.width / 2.0;
            let bb = (xa, top - half, xb, y);
            for n in &sc.nodes {
                let nr = node_rect(n);
                if bb.0 < nr.2 + 3.0 && bb.2 > nr.0 - 3.0 && bb.1 < nr.3 + 3.0 && bb.3 > nr.1 - 3.0 && (y > nr.3 || y < nr.1) {
                    st.hop_tight_node += 1;
                    if st.hop_tight_node <= 4 {
                        fails.push(format!("{tag}: (tight) edge {} {}->{} hop at ({:.1},{:.1}) r {:.1} within 3 px of node {} [{:.1},{:.1},{:.1},{:.1}]", p.edge, e.from, e.to, cx, y, r, n.id, nr.0, nr.1, nr.2, nr.3));
                    }
                }
                if bb.0 < nr.2 + 1.0 && bb.2 > nr.0 - 1.0 && bb.1 < nr.3 + 1.0 && bb.3 > nr.1 - 1.0 {
                    // the level leg itself already inside? only count when the line y is outside
                    if y > nr.3 || y < nr.1 {
                        st.hop_node += 1;
                        fails.push(format!("{tag}: edge {} {}->{} hop at ({:.1},{:.1}) r {:.1} pokes into node {}", p.edge, e.from, e.to, cx, y, r, n.id));
                    }
                }
            }
            for (j, o) in sc.edges.iter().enumerate() {
                if let Some((text, c, w)) = &o.label {
                    let lines_n = 1 + text.matches("<br").count() + text.matches('\n').count();
                    let h = 11.0 + 8.5 * (lines_n - 1) as f64;
                    let lr = (c.0 - w / 2.0, c.1 - h, c.0 + w / 2.0, c.1 + h);
                    if bb.0 < lr.2 && bb.2 > lr.0 && bb.1 < lr.3 && bb.3 > lr.1 {
                        st.hop_label += 1;
                        if st.hop_label <= 6 {
                            fails.push(format!("{tag}: edge {} {}->{} hop at ({:.1},{:.1}) under/into label of edge {j} '{}'", p.edge, e.from, e.to, cx, y, text));
                        }
                    }
                }
            }
            for c in &sc.clusters {
                // a level border line between the hop's line and its crown
                for by in [c.y, c.y + c.h] {
                    if by < y - 0.5 && by > top - half - 1.0 && xb > c.x && xa < c.x + c.w {
                        st.hop_frame += 1;
                        fails.push(format!("{tag}: edge {} {}->{} hop at ({:.1},{:.1}) r {:.1} reaches frame {} border y={:.1}", p.edge, e.from, e.to, cx, y, r, c.id, by));
                    }
                }
            }
            for o in &pa {
                if o.edge == p.edge {
                    continue;
                }
                // other ink within 2 px of the arc that is not a plumb stroke crossing under the hop
                let mut worst = f64::INFINITY;
                for w in o.pts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    let plumb_under = (a.0 - b.0).abs() <= 0.35 * (a.1 - b.1).abs() && a.0.min(b.0) > xa + r - 1.5 && a.0.max(b.0) < xb - r + 1.5;
                    if plumb_under {
                        continue;
                    }
                    for q in up.pts.iter().chain(&down.pts) {
                        // only the raised part of the arc (the feet are on the line itself)
                        if q.1 < y - 1.5 {
                            worst = worst.min(seg_dist(*q, a, b));
                        }
                    }
                }
                if worst < half + o.width / 2.0 + 1.0 {
                    st.hop_edge += 1;
                    if st.hop_edge <= 6 {
                        let oe = &sc.edges[o.edge];
                        fails.push(format!("{tag}: edge {} {}->{} hop at ({:.1},{:.1}) r {:.1} comes {:.1} px from edge {} {}->{}", p.edge, e.from, e.to, cx, y, r, worst, o.edge, oe.from, oe.to));
                    }
                }
            }
            // arrowheads of other edges: an end within the hop's box
            for (j, o) in sc.edges.iter().enumerate() {
                if j == p.edge {
                    continue;
                }
                let (s, z) = ends(o);
                for q in [z] {
                    let ink = up.pts.iter().chain(&down.pts).map(|a| dist(*a, q)).fold(f64::INFINITY, f64::min);
                    if ink < 9.0 {
                        st.hop_head += 1;
                        if st.hop_head <= 6 {
                            fails.push(format!("{tag}: edge {} hop at ({:.1},{:.1}) sits by an end of edge {j} {}->{} at ({:.1},{:.1})", p.edge, cx, y, o.from, o.to, q.0, q.1));
                        }
                    }
                }
            }
        }
    }
}

fn o_half(p: Option<&&Painted>) -> f64 {
    p.map_or(1.0, |p| p.width / 2.0)
}

/// Fan metrics from public geometry: (fan crossings, uneven fanned sides, fanned sides, max port gap spread).
fn fans(sc: &Scene) -> (usize, usize, usize, Vec<String>) {
    // (node index, side) -> [(edge, port along side)]
    let mut m: BTreeMap<(usize, u8), Vec<(usize, f64)>> = BTreeMap::new();
    let idx: BTreeMap<&str, usize> = sc.nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
    for (i, e) in sc.edges.iter().enumerate() {
        if e.from == e.to {
            continue;
        }
        let (s, z) = ends(e);
        for (id, p) in [(&e.from, s), (&e.to, z)] {
            let Some(&v) = idx.get(id.as_str()) else { continue };
            let r = node_rect(&sc.nodes[v]);
            let d = [(p.1 - r.1).abs(), (p.0 - r.2).abs(), (p.1 - r.3).abs(), (p.0 - r.0).abs()];
            let side = (0..4).min_by(|&a, &b| d[a].total_cmp(&d[b])).unwrap();
            if d[side] > 1.5 {
                continue; // not on the box's side (round shapes)
            }
            let along = if side % 2 == 0 { p.0 } else { p.1 };
            m.entry((v, side as u8)).or_default().push((i, along));
        }
    }
    let (mut x, mut uneven, mut sides) = (0, 0, 0);
    let mut det = Vec::new();
    for ((v, side), mut ends_) in m {
        if ends_.len() < 3 {
            continue;
        }
        if matches!(sc.nodes[v].shape, Shape::Diamond | Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd | Shape::ForkBar) {
            continue;
        }
        sides += 1;
        ends_.sort_by(|a, b| a.1.total_cmp(&b.1));
        let gaps: Vec<f64> = ends_.windows(2).map(|w| w[1].1 - w[0].1).collect();
        let (lo, hi) = gaps.iter().fold((f64::INFINITY, 0.0f64), |(lo, hi), &g| (lo.min(g), hi.max(g)));
        if hi - lo > 2.0 {
            uneven += 1;
            det.push(format!("uneven {} side {} n={} pitch {:.1}..{:.1}", sc.nodes[v].id, side, ends_.len(), lo, hi));
        }
        let mut fx = 0;
        for a in 0..ends_.len() {
            for b in a + 1..ends_.len() {
                let (la, lb) = (scene_line(&sc.edges[ends_[a].0]), scene_line(&sc.edges[ends_[b].0]));
                if ends_[a].0 == ends_[b].0 {
                    continue;
                }
                for u in la.windows(2) {
                    for w in lb.windows(2) {
                        if cross(u[0], u[1], w[0], w[1]) {
                            fx += 1;
                        }
                    }
                }
            }
        }
        if fx > 0 {
            det.push(format!("fan_x {} side {} n={} crossings {}", sc.nodes[v].id, side, ends_.len(), fx));
        }
        x += fx;
    }
    (x, uneven, sides, det)
}

fn cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let o = |p: Pt, q: Pt, r: Pt| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    d1 * d2 < -1e-6 && d3 * d4 < -1e-6
}

/// Total proper crossings between all edge pairs (scene polylines).
fn crossings(sc: &Scene) -> usize {
    let lines: Vec<Vec<Pt>> = sc.edges.iter().map(scene_line).collect();
    let mut n = 0;
    for a in 0..lines.len() {
        for b in a + 1..lines.len() {
            for u in lines[a].windows(2) {
                for w in lines[b].windows(2) {
                    if cross(u[0], u[1], w[0], w[1]) {
                        n += 1;
                    }
                }
            }
        }
    }
    n
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args[0].as_str();
    let mut files: Vec<String> = Vec::new();
    for a in &args[1..] {
        let p = std::path::Path::new(a);
        if p.is_dir() {
            let mut v: Vec<String> = std::fs::read_dir(p)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().map_or(false, |e| e == "mmd"))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            v.sort();
            files.extend(v);
        } else {
            files.push(a.clone());
        }
    }
    let mut total = Stats::default();
    let mut nfail = 0;
    let (mut tfx, mut tun, mut tsd, mut tx) = ([0usize; 2], [0usize; 2], [0usize; 2], [0usize; 2]);
    for f in &files {
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(f).unwrap();
        let Some(g) = graph_of(&src) else { continue };
        let sc = scene::scene(&g);
        let base: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        match mode {
            "dump" => {
                for n in &sc.nodes {
                    println!("node {} x={:.1} y={:.1} w={:.1} h={:.1} {:?}", n.id, n.x, n.y, n.w, n.h, n.shape);
                }
                for c in &sc.clusters {
                    println!("cluster {} x={:.1} y={:.1} w={:.1} h={:.1}", c.id, c.x, c.y, c.w, c.h);
                }
                let (x, u, s, det) = fans(&sc);
                println!("static fans: x={x} uneven={u} sides={s}");
                for d in det {
                    println!("  {d}");
                }
                let rt = scene::route(&g, &base);
                let (x, u, s, det) = fans(&rt);
                println!("routed fans: x={x} uneven={u} sides={s}");
                for d in det {
                    println!("  {d}");
                }
            }
            "fans" => {
                let rt = scene::route(&g, &base);
                let (a, b) = (fans(&sc), fans(&rt));
                let (xa, xb) = (crossings(&sc), crossings(&rt));
                println!("{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", a.0, b.0, a.1, b.1, a.2, b.2, xa, xb);
                tfx[0] += a.0;
                tfx[1] += b.0;
                tun[0] += a.1;
                tun[1] += b.1;
                tsd[0] += a.2;
                tsd[1] += b.2;
                tx[0] += xa;
                tx[1] += xb;
            }
            "check" => {
                let mut fails = Vec::new();
                let mut st = Stats::default();
                check_scene(&g, &sc, &format!("{name} static"), &mut fails, &mut st);
                // determinism
                let svg = scene::to_svg(&sc);
                if svg != scene::to_svg(&scene::scene(&g)) {
                    fails.push(format!("{name}: scene() not deterministic"));
                }
                // to_mermaid round trip: same drawing
                let mm = flowmaid::to_mermaid(&g);
                let rt_ok = match flowmaid::parser::parse(&mm) {
                    Ok(g2) => scene::to_svg(&scene::scene(&g2)) == svg || !src.trim_start().starts_with("flowchart") && !src.trim_start().starts_with("graph"),
                    Err(_) => false,
                };
                if !rt_ok {
                    fails.push(format!("{name}: to_mermaid round trip draws differently"));
                }
                // route at the static centres
                let rt = scene::route(&g, &base);
                check_scene(&g, &rt, &format!("{name} route"), &mut fails, &mut st);
                if scene::to_svg(&rt) != scene::to_svg(&scene::route(&g, &base)) {
                    fails.push(format!("{name}: route() not deterministic"));
                }
                // drags: the busiest nodes and the first / last, a few offsets each
                let mut deg = vec![0usize; g.nodes.len()];
                for e in &g.edges {
                    deg[e.from] += 1;
                    deg[e.to] += 1;
                }
                let mut order: Vec<usize> = (0..g.nodes.len()).collect();
                order.sort_by(|&a, &b| deg[b].cmp(&deg[a]).then(a.cmp(&b)));
                let mut picks: Vec<usize> = order.iter().copied().take(3).collect();
                for v in [0, g.nodes.len() - 1] {
                    if !picks.contains(&v) {
                        picks.push(v);
                    }
                }
                let mut hsum = 0u64;
                if std::env::var_os("NODRAG").is_some() {
                    picks.clear();
                }
                for &v in &picks {
                    for (k, d) in [(1.0, 0.0), (37.0, 23.0), (-60.0, 15.0), (180.0, 140.0), (-5.0, -90.0)].iter().enumerate() {
                        let mut c = base.clone();
                        c[v].0 += d.0;
                        c[v].1 += d.1;
                        let full = scene::route(&g, &c);
                        check_scene(&g, &full, &format!("{name} route drag {} #{k}", g.nodes[v].id), &mut fails, &mut st);
                        let part = scene::route_partial(&g, &c, &sc, &base);
                        check_scene(&g, &part, &format!("{name} route_partial drag {} #{k}", g.nodes[v].id), &mut fails, &mut st);
                        let ps = scene::to_svg(&part);
                        if ps != scene::to_svg(&scene::route_partial(&g, &c, &sc, &base)) {
                            fails.push(format!("{name}: route_partial not deterministic (drag {} #{k})", g.nodes[v].id));
                        }
                        // the dragged node sits where it was put
                        if dist((part.nodes[v].x, part.nodes[v].y), c[v]) > 1e-6 || dist((full.nodes[v].x, full.nodes[v].y), c[v]) > 1e-6 {
                            fails.push(format!("{name}: dragged node {} not at its centre", g.nodes[v].id));
                        }
                        hsum ^= hash(&ps).rotate_left(k as u32) ^ hash(&scene::to_svg(&full));
                    }
                }
                println!(
                    "{name}\tedges={}\thops={}\tmasks={}\tsamples={}\tpick_miss={}\thop_pick_miss={}\tunder_wrong={}\tsharp={}\toff_path={}\thop_node={}\thop_label={}\thop_frame={}\thop_edge={}\thop_head={}\thop_unpicked={}\ttight_node={}\tmermaid={:016x}\tsvg={:016x}\tdrags={:016x}\tfails={}",
                    g.edges.len(), st.hops, st.masks, st.samples, st.pick_miss, st.hop_pick_miss, st.hop_under_wrong, st.sharp, st.off_path, st.hop_node, st.hop_label, st.hop_frame, st.hop_edge, st.hop_head, st.hop_paint_unpicked, st.hop_tight_node, hash(&mm), hash(&svg), hsum, fails.len()
                );
                for f in fails.iter().take(40) {
                    println!("  FAIL {f}");
                }
                nfail += fails.len();
                total.hops += st.hops;
                total.pick_miss += st.pick_miss;
                total.hop_pick_miss += st.hop_pick_miss;
                total.hop_under_wrong += st.hop_under_wrong;
                total.sharp += st.sharp;
                total.off_path += st.off_path;
                total.hop_node += st.hop_node;
                total.hop_label += st.hop_label;
                total.hop_frame += st.hop_frame;
                total.hop_edge += st.hop_edge;
                total.hop_head += st.hop_head;
                total.samples += st.samples;
                total.hop_paint_unpicked += st.hop_paint_unpicked;
                total.hop_tight_node += st.hop_tight_node;
            }
            _ => panic!("mode"),
        }
    }
    if mode == "check" {
        println!("TOTAL fails={nfail} {total:?}");
    }
    if mode == "fans" {
        println!("TOTAL\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", tfx[0], tfx[1], tun[0], tun[1], tsd[0], tsd[1], tx[0], tx[1]);
    }
}
