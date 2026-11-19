// Verifier probe (b12a3-route fix round 1). Usage: probe <off|on> files...
// off: hashes of scene / route / route_partial / route-of-drag SVGs (compare pre vs head).
// on : the same hashes plus contract checks in block mode.
use flowmaid::model::{Document, Graph};
use flowmaid::scene::{self, Scene};

fn fnv(h: &mut u64, s: &str) {
    for b in s.bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}
fn h1(s: &str) -> u64 {
    let mut h = 0xcbf29ce484222325;
    fnv(&mut h, s);
    h
}

fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}

fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let l2 = abx * abx + aby * aby;
    let t = if l2 <= 1e-12 { 0.0 } else { (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / l2).clamp(0.0, 1.0) };
    (a.0 + t * abx - p.0).hypot(a.1 + t * aby - p.1)
}

fn poly(e: &flowmaid::scene::SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 {
        e.waypoints.clone()
    } else {
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
}

#[derive(Default)]
struct Tally {
    frames: usize,
    parallel: usize,
    finite: usize,
    detached: usize,
    sharp: usize,
    pick: usize,
    nondet: usize,
    member_out: usize,
    canvas: usize,
    through: usize,
    cold_warm: usize,
    route_vs_partial: usize,
    chained: usize,
    first: Vec<String>,
}

impl Tally {
    fn note(&mut self, s: String) {
        if self.first.len() < std::env::var("NOTES").ok().and_then(|s| s.parse().ok()).unwrap_or(30) {
            self.first.push(s);
        }
    }
}

fn check(g: &Graph, sc: &Scene, what: &str, t: &mut Tally, moved: Option<usize>) {
    t.frames += 1;
    if sc.edges.len() < g.edges.len() {
        t.parallel += 1;
        t.note(format!("{what}: {} scene edges < {} graph edges", sc.edges.len(), g.edges.len()));
    }
    for (i, e) in g.edges.iter().enumerate() {
        if let Some(se) = sc.edges.get(i) {
            if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
                t.parallel += 1;
                t.note(format!("{what}: edge {i} is {}->{} in scene", se.from, se.to));
            }
        }
    }
    for (i, nd) in sc.nodes.iter().enumerate() {
        if nd.id != g.nodes[i].id {
            t.parallel += 1;
        }
    }
    for e in &sc.edges {
        if !e.bezier.iter().chain(&e.waypoints).all(|p| p.0.is_finite() && p.1.is_finite()) {
            t.finite += 1;
            t.note(format!("{what}: {}->{} not finite", e.from, e.to));
        }
    }
    for (i, e) in g.edges.iter().enumerate() {
        if e.from == e.to {
            continue;
        }
        let Some(se) = sc.edges.get(i) else { continue };
        let pl = poly(se);
        if pl.len() < 2 {
            continue;
        }
        for (v, p) in [(e.from, pl[0]), (e.to, pl[pl.len() - 1])] {
            let nd = &sc.nodes[v];
            let (x0, y0, x1, y1) = (nd.x - nd.w / 2.0 - 4.0, nd.y - nd.h / 2.0 - 4.0, nd.x + nd.w / 2.0 + 4.0, nd.y + nd.h / 2.0 + 4.0);
            if !(p.0 >= x0 && p.0 <= x1 && p.1 >= y0 && p.1 <= y1) {
                t.detached += 1;
                t.note(format!(
                    "{what}: {}->{} end {:?} off node {} box ({:.1},{:.1} {:.1}x{:.1}){}",
                    se.from, se.to, (p.0.round(), p.1.round()), nd.id, nd.x, nd.y, nd.w, nd.h,
                    if moved == Some(v) { " [moved]" } else { "" }
                ));
            }
        }
    }
    for se in &sc.edges {
        let w = &se.waypoints;
        if w.len() < 3 {
            continue;
        }
        for k in 1..w.len() - 1 {
            let a = (w[k].0 - w[k - 1].0, w[k].1 - w[k - 1].1);
            let b = (w[k + 1].0 - w[k].0, w[k + 1].1 - w[k].1);
            let (la, lb) = (a.0.hypot(a.1), b.0.hypot(b.1));
            if la < 4.0 || lb < 4.0 {
                continue;
            }
            let cos = (a.0 * b.0 + a.1 * b.1) / (la * lb);
            if cos < 0.6 && la.max(lb) > 14.5 {
                t.sharp += 1;
                t.note(format!("{what}: {}->{} sharp turn at {:?} (cos {:.2})", se.from, se.to, (w[k].0.round(), w[k].1.round()), cos));
                break;
            }
        }
    }
    for (i, se) in sc.edges.iter().enumerate().take(g.edges.len()) {
        let pl = poly(se);
        if pl.len() < 2 {
            continue;
        }
        let (k, len) = pl.windows(2).enumerate().map(|(k, w)| (k, (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))).fold((0, 0.0), |b, x| if x.1 > b.1 { x } else { b });
        if len < 20.0 {
            continue;
        }
        let p = ((pl[k].0 + pl[k + 1].0) / 2.0, (pl[k].1 + pl[k + 1].1) / 2.0);
        if sc.nodes.iter().any(|n| (p.0 - n.x).abs() <= n.w / 2.0 + 2.0 && (p.1 - n.y).abs() <= n.h / 2.0 + 2.0) {
            continue;
        }
        match sc.edge_at(p.0, p.1, 1.5) {
            Some(j) if j == i => {}
            Some(j) => {
                let other = poly(&sc.edges[j]);
                let d = other.windows(2).map(|w| seg_dist(p, w[0], w[1])).fold(f64::INFINITY, f64::min);
                if d > 3.5 {
                    t.pick += 1;
                    t.note(format!("{what}: pick at {:?} on {}->{} got {}->{} ({d:.1} px away)", p, se.from, se.to, sc.edges[j].from, sc.edges[j].to));
                }
            }
            None => {
                t.pick += 1;
                t.note(format!("{what}: pick at {:?} on {}->{} got nothing", p, se.from, se.to));
            }
        }
    }
    for s in &g.subgraphs {
        let Some(c) = sc.clusters.iter().find(|c| c.id == s.id) else { continue };
        for &v in &s.nodes {
            let nd = &sc.nodes[v];
            if nd.x - nd.w / 2.0 < c.x - 1.0 || nd.x + nd.w / 2.0 > c.x + c.w + 1.0 || nd.y - nd.h / 2.0 < c.y - 1.0 || nd.y + nd.h / 2.0 > c.y + c.h + 1.0 {
                t.member_out += 1;
                t.note(format!("{what}: {} outside frame {}", nd.id, s.id));
            }
        }
    }
    for (i, e) in g.edges.iter().enumerate() {
        let Some(se) = sc.edges.get(i) else { continue };
        let pl = poly(se);
        for (u, nd) in sc.nodes.iter().enumerate() {
            if u == e.from || u == e.to {
                continue;
            }
            let r = (nd.x - nd.w / 2.0 + 3.0, nd.y - nd.h / 2.0 + 3.0, nd.x + nd.w / 2.0 - 3.0, nd.y + nd.h / 2.0 - 3.0);
            if r.2 <= r.0 || r.3 <= r.1 {
                continue;
            }
            let hit = pl.windows(2).any(|w| {
                (0..=8).any(|k| {
                    let t = k as f64 / 8.0;
                    let p = (w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t);
                    p.0 > r.0 && p.0 < r.2 && p.1 > r.1 && p.1 < r.3
                })
            });
            if hit {
                t.through += 1;
                t.note(format!("{what}: {}->{} through node {}", se.from, se.to, nd.id));
            }
        }
    }
    for se in &sc.edges {
        for p in poly(se) {
            if p.0 < sc.origin.0 - 0.5 || p.1 < sc.origin.1 - 0.5 || p.0 > sc.origin.0 + sc.width + 0.5 || p.1 > sc.origin.1 + sc.height + 0.5 {
                t.canvas += 1;
                t.note(format!("{what}: {}->{} point {:?} off canvas", se.from, se.to, p));
                break;
            }
        }
    }
}

/// route() on a fresh thread: no memo of earlier layouts.
fn cold_route(g: &Graph, c: &[(f64, f64)], on: bool) -> String {
    let g = g.clone();
    let c = c.to_vec();
    std::thread::spawn(move || flowmaid::quality::with_blocks(on, || scene::to_svg(&scene::route(&g, &c))))
        .join()
        .unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let on = args[1] == "on";
    let checks = on || std::env::var("CHECK").is_ok();
    let max_nodes: usize = std::env::var("MAXN").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
    for f in &args[2..] {
        let src = std::fs::read_to_string(f).unwrap();
        let Some(g) = graph_of(&src) else { continue };
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let mut t = Tally::default();
        let mut h: u64 = 0xcbf29ce484222325;
        let mut h_route: u64 = 0xcbf29ce484222325;
        let clusters = flowmaid::quality::with_blocks(on, || {
            let base = scene::scene(&g);
            if checks {
                check(&g, &base, &format!("{name} scene"), &mut t, None);
            }
            fnv(&mut h, &scene::to_svg(&base));
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r0 = scene::route(&g, &at);
            if checks {
                check(&g, &r0, &format!("{name} route@scene"), &mut t, None);
                if cold_route(&g, &at, on) != scene::to_svg(&r0) {
                    t.cold_warm += 1;
                    t.note(format!("{name}: route@scene cold != warm"));
                }
            }
            fnv(&mut h_route, &scene::to_svg(&r0));
            let n = g.nodes.len();
            let step = (n / max_nodes).max(1);
            for v in (0..n).step_by(step) {
                let mut prev: Option<(Scene, Vec<(f64, f64)>)> = None;
                for (k, d) in [(1.0, 0.0), (0.0, -1.0), (60.0, 25.0), (-150.0, 80.0), (0.0, 220.0)].iter().enumerate() {
                    let mut c = at.clone();
                    c[v] = (c[v].0 + d.0, c[v].1 + d.1);
                    let p = scene::route_partial(&g, &c, &base, &at);
                    let what = format!("{name} drag {} by {:?}", g.nodes[v].id, d);
                    if checks {
                        check(&g, &p, &what, &mut t, Some(v));
                    }
                    let s1 = scene::to_svg(&p);
                    fnv(&mut h, &s1);
                    if k == 2 {
                        let p2 = scene::route_partial(&g, &c, &base, &at);
                        if scene::to_svg(&p2) != s1 {
                            t.nondet += 1;
                            t.note(format!("{what}: route_partial not deterministic"));
                        }
                        let r = scene::route(&g, &c);
                        let rs = scene::to_svg(&r);
                        if checks {
                            check(&g, &r, &format!("{what} [route]"), &mut t, Some(v));
                            if rs != s1 {
                                t.route_vs_partial += 1;
                                t.note(format!("{what}: route() != route_partial() from the scene"));
                            }
                            if cold_route(&g, &c, on) != rs {
                                t.cold_warm += 1;
                                t.note(format!("{what}: route cold != warm"));
                            }
                        }
                        fnv(&mut h_route, &rs);
                    }
                    // A chained drag: the next frame from the previous frame.
                    if checks {
                        if let Some((pb, pc)) = &prev {
                            let q = scene::route_partial(&g, &c, pb, pc);
                            check(&g, &q, &format!("{what} [chained]"), &mut t, Some(v));
                            let mut x = 0;
                            for e in &q.edges {
                                if !e.bezier.iter().chain(&e.waypoints).all(|p| p.0.is_finite() && p.1.is_finite()) {
                                    x += 1;
                                }
                            }
                            t.chained += x;
                        }
                    }
                    prev = Some((p, c));
                }
            }
            let m = flowmaid::to_mermaid(&g);
            let g2 = flowmaid::parser::parse(&m).ok();
            let rt = match &g2 {
                Some(g2) => flowmaid::to_mermaid(g2) == m && scene::to_svg(&scene::scene(g2)) == scene::to_svg(&base),
                None => false,
            };
            if !rt {
                t.note(format!("{name}: to_mermaid round trip differs"));
            }
            let cs = flowmaid::fold::scene_compact(&g, &flowmaid::fold::CompactOptions::for_extent(600.0));
            let _ = cs;
            base.clusters.len()
        });
        println!(
            "{name}\tclusters={clusters}\tframes={}\tparallel={}\tfinite={}\tdetached={}\tsharp={}\tpick={}\tnondet={}\tmember_out={}\tcanvas={}\tthrough={}\tcold_warm={}\troute_vs_partial={}\tchained_nonfinite={}\thash={:016x}\troute_hash={:016x}",
            t.frames, t.parallel, t.finite, t.detached, t.sharp, t.pick, t.nondet, t.member_out, t.canvas, t.through, t.cold_warm, t.route_vs_partial, t.chained, h, h_route
        );
        let _ = h1("");
        for s in &t.first {
            println!("    {s}");
        }
    }
}
