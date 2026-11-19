// Interactive contract over flowchart/state inputs: index-parallel nodes
// and edges, sub-edges after, host centres kept, finite geometry, frames
// round members, determinism, to_mermaid round trip, picking on the drawn
// geometry, waypoint density (a host's B-spline through the waypoints
// stays on the polyline), bars never resized by a drag.
use flowmaid::model::{End, Graph, Shape};
use flowmaid::scene::{self, Hit, Scene, SceneEdge};
use flowmaid::{parser::parse_document, to_mermaid, Document};
use std::time::Instant;

type P = (f64, f64);
fn graph(src: &str) -> Option<Graph> {
    match parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn end_id(g: &Graph, e: End) -> String {
    match e {
        End::Node(v) => g.nodes[v].id.clone(),
        End::Sub(s) => g.subgraphs[s].id.clone(),
    }
}
fn cubic(b: &[P; 4], n: usize) -> Vec<P> {
    (0..=n)
        .map(|k| {
            let t = k as f64 / n as f64;
            let u = 1.0 - t;
            (
                u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0,
                u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1,
            )
        })
        .collect()
}
// d3 curveBasis through the points (what a host's B-spline draws).
fn basis(p: &[P]) -> Vec<P> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let mut q = vec![p[0], p[0]];
    q.extend_from_slice(p);
    q.push(*p.last().unwrap());
    q.push(*p.last().unwrap());
    let mut out = vec![p[0]];
    for w in q.windows(4) {
        for k in 1..=8 {
            let t = k as f64 / 8.0;
            let b0 = (1.0 - t).powi(3) / 6.0;
            let b1 = (3.0 * t.powi(3) - 6.0 * t * t + 4.0) / 6.0;
            let b2 = (-3.0 * t.powi(3) + 3.0 * t * t + 3.0 * t + 1.0) / 6.0;
            let b3 = t.powi(3) / 6.0;
            out.push((
                b0 * w[0].0 + b1 * w[1].0 + b2 * w[2].0 + b3 * w[3].0,
                b0 * w[0].1 + b1 * w[1].1 + b2 * w[2].1 + b3 * w[3].1,
            ));
        }
    }
    out
}
fn seg_dist(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}
fn poly_dist(p: P, line: &[P]) -> f64 {
    line.windows(2).map(|w| seg_dist(p, w[0], w[1])).fold(f64::INFINITY, f64::min)
}
fn drawn(e: &SceneEdge) -> Vec<P> {
    if e.waypoints.len() >= 2 { basis(&e.waypoints) } else { cubic(&e.bezier, 24) }
}
fn is_axis(e: &SceneEdge) -> bool {
    // an orthogonal path: most of its length on the axes
    if e.waypoints.len() < 3 { return false; }
    let (mut ax, mut all) = (0.0, 0.0);
    for w in e.waypoints.windows(2) {
        let l = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        all += l;
        if (w[1].0 - w[0].0).abs() < 0.01 || (w[1].1 - w[0].1).abs() < 0.01 { ax += l; }
    }
    all > 0.0 && ax / all > 0.8
}

struct Stats { bad: usize, spline_dev: f64, picks: usize, pick_miss: usize }

fn contract(tag: &str, g: &Graph, sc: &Scene, centers: Option<&[P]>, st: &mut Stats) {
    let mut fail = |m: String| {
        println!("  FAIL[{tag}] {m}");
        st.bad += 1;
    };
    if sc.nodes.len() != g.nodes.len() {
        fail(format!("nodes {} vs {}", sc.nodes.len(), g.nodes.len()));
        return;
    }
    for (i, (n, m)) in sc.nodes.iter().zip(&g.nodes).enumerate() {
        if n.id != m.id { fail(format!("node {i} id {} vs {}", n.id, m.id)); }
        if !(n.x.is_finite() && n.y.is_finite() && n.w.is_finite() && n.h.is_finite()) { fail(format!("node {} not finite", n.id)); }
        if let Some(c) = centers {
            if (n.x - c[i].0).abs() > 1e-6 || (n.y - c[i].1).abs() > 1e-6 {
                fail(format!("node {} moved from host centre {:?} -> {:?}", n.id, c[i], (n.x, n.y)));
            }
        }
    }
    if sc.edges.len() < g.edges.len() {
        fail(format!("edges {} < {}", sc.edges.len(), g.edges.len()));
        return;
    }
    for (i, (e, m)) in sc.edges.iter().zip(&g.edges).enumerate() {
        if e.from != g.nodes[m.from].id || e.to != g.nodes[m.to].id {
            fail(format!("edge {i} {}->{} vs {}->{}", e.from, e.to, g.nodes[m.from].id, g.nodes[m.to].id));
        }
    }
    let subs: Vec<(String, String)> = g.sub_edges.iter().map(|e| (end_id(g, e.from), end_id(g, e.to))).collect();
    let drawn_subs: Vec<(String, String)> = sc.edges[g.edges.len()..].iter().map(|e| (e.from.clone(), e.to.clone())).collect();
    if subs != drawn_subs { fail(format!("sub-edges {subs:?} vs {drawn_subs:?}")); }
    for w in sc.clusters.windows(2) {
        if w[0].depth > w[1].depth { fail("clusters not outermost-first".into()); }
    }
    for s in &g.subgraphs {
        match sc.clusters.iter().find(|c| c.id == s.id) {
            None => fail(format!("no frame {}", s.id)),
            Some(c) => for &v in &s.nodes {
                let n = &sc.nodes[v];
                if n.x - n.w / 2.0 < c.x - 0.5 || n.x + n.w / 2.0 > c.x + c.w + 0.5 || n.y - n.h / 2.0 < c.y - 0.5 || n.y + n.h / 2.0 > c.y + c.h + 0.5 {
                    fail(format!("{} outside frame {}", n.id, s.id));
                }
            },
        }
    }
    for (i, e) in sc.edges.iter().enumerate() {
        if !e.bezier.iter().chain(&e.waypoints).all(|p| p.0.is_finite() && p.1.is_finite()) {
            fail(format!("non-finite {}->{}", e.from, e.to));
            continue;
        }
        if let Some((_, c, w)) = &e.label {
            if !(c.0.is_finite() && c.1.is_finite() && w.is_finite()) { fail(format!("label not finite {}->{}", e.from, e.to)); }
        }
        // a host's spline through the waypoints stays on an orthogonal path
        if is_axis(e) {
            let sp = basis(&e.waypoints);
            let dev = sp.iter().map(|&p| poly_dist(p, &e.waypoints)).fold(0.0, f64::max);
            st.spline_dev = st.spline_dev.max(dev);
            let longest = e.waypoints.windows(2).map(|w| ((w[1].0-w[0].0).powi(2)+(w[1].1-w[0].1).powi(2)).sqrt()).fold(0.0, f64::max);
            let _ = longest;
            if dev > 6.0 { fail(format!("host spline leaves the orthogonal path of {}->{} by {dev:.1} px ({} waypoints)", e.from, e.to, e.waypoints.len())); }
        }
        // picking on the drawn geometry (the middle of the path: painter trims under heads)
        if e.meta.style.stroke_width == Some(0.0) { continue; }
        let pts = drawn(e);
        let n = pts.len();
        for k in [n / 4, n / 2, 3 * n / 4] {
            let p = pts[k.min(n - 1)];
            st.picks += 1;
            match sc.edge_at(p.0, p.1, 3.0) {
                Some(j) if j == i => {}
                Some(j) => {
                    // fine when edge j is drawn through the same spot
                    if poly_dist(p, &drawn(&sc.edges[j])) > 4.0 { st.pick_miss += 1; fail(format!("edge {i} picked as {j} at {p:?}")); }
                }
                None => { st.pick_miss += 1; fail(format!("edge {i} {}->{} not picked at {p:?}", e.from, e.to)); }
            }
            // hit_test agrees unless a node covers the spot
            if sc.node_at(p.0, p.1).is_none() {
                match sc.hit_test(p.0, p.1, 3.0) {
                    Some(Hit::Edge(_)) => {}
                    other => { fail(format!("hit_test at {p:?} on edge {i}: {other:?}")); }
                }
            }
        }
    }
    for (i, n) in sc.nodes.iter().enumerate() {
        if n.w > 0.0 && n.h > 0.0 {
            match sc.hit_test(n.x, n.y, 3.0) {
                Some(Hit::Node(_)) => {}
                other => fail(format!("hit_test at centre of node {i} {}: {other:?}", n.id)),
            }
        }
    }
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let mut total_bad = 0;
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let Some(g) = graph(&src) else { continue };
        let name = f.rsplit('/').next().unwrap();
        let mut st = Stats { bad: 0, spline_dev: 0.0, picks: 0, pick_miss: 0 };
        let sc = scene::scene(&g);
        contract("scene", &g, &sc, None, &mut st);
        if scene::to_svg(&sc) != scene::to_svg(&scene::scene(&g)) { println!("  FAIL nondeterministic scene"); st.bad += 1; }
        let back = to_mermaid(&g);
        match graph(&back) {
            Some(g2) => {
                if to_mermaid(&g2) != back { println!("  FAIL to_mermaid not a fixed point"); st.bad += 1; }
                if scene::to_svg(&scene::scene(&g2)) != scene::to_svg(&sc) { println!("  NOTE[{name}] round-trip scene differs"); }
            }
            None => { println!("  FAIL to_mermaid output does not parse"); st.bad += 1; }
        }
        let at: Vec<P> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let sizes: Vec<P> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
        let r = scene::route(&g, &at);
        contract("route", &g, &r, Some(&at), &mut st);
        if scene::to_svg(&r) != scene::to_svg(&scene::route(&g, &at)) { println!("  FAIL nondeterministic route"); st.bad += 1; }
        let bars: Vec<usize> = (0..g.nodes.len()).filter(|&v| g.nodes[v].shape == Shape::ForkBar).collect();
        let mut bar_changes = 0;
        let mut check_bars = |what: &str, d: &Scene, st: &mut Stats| {
            for &b in &bars {
                if (d.nodes[b].w - sc.nodes[b].w).abs() > 0.01 || (d.nodes[b].h - sc.nodes[b].h).abs() > 0.01 {
                    bar_changes += 1;
                    if bar_changes <= 4 {
                        println!("  FAIL[{what}] bar {} {:?} -> {:?}", g.nodes[b].id, (sc.nodes[b].w, sc.nodes[b].h), (d.nodes[b].w, d.nodes[b].h));
                    }
                    st.bad += 1;
                }
            }
        };
        check_bars("route at scene centres", &r, &mut st);
        let (mut worst_partial, mut worst_route) = (0.0f64, 0.0f64);
        let mut frame_mismatch = 0;
        for v in 0..g.nodes.len() {
            for (dx, dy) in [(25.0, 0.0), (0.0, 40.0), (-60.0, 35.0), (140.0, -90.0), (250.0, 0.0), (0.0, 220.0)] {
                let mut c = at.clone();
                c[v].0 += dx;
                c[v].1 += dy;
                let t = Instant::now();
                let p = scene::route_partial(&g, &c, &sc, &at);
                worst_partial = worst_partial.max(t.elapsed().as_secs_f64() * 1e3);
                let t = Instant::now();
                let full = scene::route(&g, &c);
                worst_route = worst_route.max(t.elapsed().as_secs_f64() * 1e3);
                let id = &g.nodes[v].id;
                contract(&format!("partial {id} {dx},{dy}"), &g, &p, Some(&c), &mut st);
                contract(&format!("route {id} {dx},{dy}"), &g, &full, Some(&c), &mut st);
                check_bars(&format!("partial {id} {dx},{dy}"), &p, &mut st);
                check_bars(&format!("route {id} {dx},{dy}"), &full, &mut st);
                let rs = scene::route_sized(&g, &c, &sizes);
                check_bars(&format!("route_sized {id} {dx},{dy}"), &rs, &mut st);
                if scene::to_svg(&p) != scene::to_svg(&scene::route_partial(&g, &c, &sc, &at)) { println!("  FAIL nondeterministic route_partial"); st.bad += 1; }
                for cl in &full.clusters {
                    if let Some(pc) = p.clusters.iter().find(|x| x.id == cl.id) {
                        if (pc.x - cl.x).abs() > 0.5 || (pc.y - cl.y).abs() > 0.5 || (pc.w - cl.w).abs() > 0.5 || (pc.h - cl.h).abs() > 0.5 { frame_mismatch += 1; }
                    }
                }
                // drag on from a full route as base, and back again
                let mut c2 = c.clone();
                c2[v].0 += 15.0;
                c2[v].1 -= 10.0;
                let p2 = scene::route_partial(&g, &c2, &full, &c);
                contract(&format!("partial-on-route {id}"), &g, &p2, Some(&c2), &mut st);
                check_bars(&format!("partial-on-route {id} {dx},{dy}"), &p2, &mut st);
                let back = scene::route_partial(&g, &at, &full, &c);
                contract(&format!("partial-back {id}"), &g, &back, Some(&at), &mut st);
                check_bars(&format!("partial-back {id} {dx},{dy}"), &back, &mut st);
            }
        }
        println!("{name}: n {} e {} sub {} bars {} | partial worst {:.2} ms route worst {:.2} ms | frame mismatch {} | spline dev {:.2} | picks {} miss {} | bar changes {} | bad {}",
            g.nodes.len(), g.edges.len(), g.sub_edges.len(), bars.len(), worst_partial, worst_route, frame_mismatch, st.spline_dev, st.picks, st.pick_miss, bar_changes, st.bad);
        total_bad += st.bad;
    }
    println!("TOTAL BAD {total_bad}");
}
