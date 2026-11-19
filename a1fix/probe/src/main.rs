use flowmaid::class::{route, scene, to_svg, ClassScene};
use flowmaid::model::Document;
use flowmaid::scene::SceneEdge;
use std::time::Instant;

fn poly(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 {
        return e.waypoints.clone();
    }
    let [p0, p1, p2, p3] = e.bezier;
    (0..=32)
        .map(|i| {
            let t = i as f64 / 32.0;
            let u = 1.0 - t;
            (
                u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0,
                u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1,
            )
        })
        .collect()
}
fn seg_hits(r: (f64, f64, f64, f64), a: (f64, f64), b: (f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, a.0 - r.0), (dx, r.2 - a.0), (-dy, a.1 - r.1), (dy, r.3 - a.1)] {
        if p.abs() < 1e-12 {
            if q < 0.0 { return false; }
        } else {
            let t = q / p;
            if p < 0.0 { t0 = t0.max(t) } else { t1 = t1.min(t) }
            if t0 > t1 { return false; }
        }
    }
    true
}
fn check(tag: &str, cs: &ClassScene) {
    let sc = &cs.scene;
    let mut through = 0;
    for (j, e) in sc.edges.iter().enumerate() {
        let pts = poly(e);
        for (k, n) in sc.nodes.iter().enumerate() {
            if n.id == e.from || n.id == e.to { continue; }
            let r = (n.x - n.w / 2.0 + 1.0, n.y - n.h / 2.0 + 1.0, n.x + n.w / 2.0 - 1.0, n.y + n.h / 2.0 - 1.0);
            if pts.windows(2).any(|s| seg_hits(r, s[0], s[1])) {
                through += 1;
                println!("  {tag}: edge {j} {}->{} through node {k} {}", e.from, e.to, n.id);
            }
        }
        // pick at a point on the drawn line
        let mid = pts[pts.len() / 2];
        if sc.node_at(mid.0, mid.1).is_none() {
            match sc.edge_at(mid.0, mid.1, 3.0) {
                Some(i) if i == j => {}
                Some(i) => { let o = &sc.edges[i]; let op = poly(o); let d = op.iter().map(|p| (p.0-mid.0).hypot(p.1-mid.1)).fold(f64::INFINITY, f64::min); if d > 2.0 { println!("  {tag}: pick at mid of edge {j} returned {i} (d={d:.1})") } }
                None => println!("  {tag}: pick at mid of edge {j} returned None"),
            }
        }
    }
    let svg = to_svg(cs);
    let zero = svg.lines().filter(|l| l.starts_with("<polygon")).filter(|l| {
        let p: Vec<&str> = l.split('"').nth(1).unwrap().split(' ').collect();
        p.windows(2).all(|w| w[0] == w[1])
    }).count();
    println!("{tag}: nodes={} edges={} through={} zero_polys={} w={:.0} h={:.0}", sc.nodes.len(), sc.edges.len(), through, zero, sc.width, sc.height);
    #[cfg(feature = "x")]
    {}
    let _ = NEW;
}
#[cfg(not(feature = "base"))]
const NEW: bool = true;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::Path::new(&args[0]);
    for f in &args[1..] {
        let src = std::fs::read_to_string(f).unwrap();
        let name = std::path::Path::new(f).file_stem().unwrap().to_str().unwrap().to_string();
        let d = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Class(d)) => d,
            Ok(_) => { println!("{name}: not class"); continue; }
            Err(e) => { println!("{name}: parse error {e:?}"); continue; }
        };
        let n = d.classes.len();
        let cs = scene(&d);
        check(&format!("{name}/scene"), &cs);
        let centers: Vec<(f64, f64)> = cs.scene.nodes.iter().take(n).map(|nd| (nd.x, nd.y)).collect();
        // identity route
        let r0 = route(&d, &centers);
        check(&format!("{name}/route-id"), &r0);
        let s1 = to_svg(&r0);
        let s2 = to_svg(&route(&d, &centers));
        if s1 != s2 { println!("{name}: NONDETERMINISTIC route"); }
        // timing: drag first class by small steps
        let iters = 50;
        let t = Instant::now();
        let mut acc = 0usize;
        for k in 0..iters {
            let mut c = centers.clone();
            c[0].0 += k as f64 * 3.0;
            c[0].1 += k as f64 * 1.0;
            let r = route(&d, &c);
            acc += to_svg(&r).len();
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0 / iters as f64;
        println!("{name}: drag route+to_svg {ms:.3} ms (acc {acc})");
        // drags: swap the first two classes' positions; move last class on top of the first; move one far negative
        if n >= 2 {
            let mut c = centers.clone();
            c.swap(0, 1);
            let r = route(&d, &c);
            check(&format!("{name}/swap01"), &r);
            std::fs::write(out.join(format!("{name}.swap01.svg")), to_svg(&r)).unwrap();
            let mut c = centers.clone();
            c[n - 1] = (c[0].0 + 10.0, c[0].1 + 5.0);
            let r = route(&d, &c);
            check(&format!("{name}/overlap"), &r);
            let mut c = centers.clone();
            c[n / 2] = (-300.0, -200.0);
            let r = route(&d, &c);
            check(&format!("{name}/negative"), &r);
            std::fs::write(out.join(format!("{name}.negative.svg")), to_svg(&r)).unwrap();
            // invert vertical order of everything (children above parents)
            let maxy = centers.iter().map(|c| c.1).fold(0.0, f64::max);
            let c: Vec<_> = centers.iter().map(|&(x, y)| (x, maxy - y + 40.0)).collect();
            let r = route(&d, &c);
            check(&format!("{name}/flipped"), &r);
            std::fs::write(out.join(format!("{name}.flipped.svg")), to_svg(&r)).unwrap();
        }
    }
}
