use flowmaid::model::Document;
use flowmaid::scene::{route, route_partial, scene, Scene};
fn reads_own(sc: &Scene) -> (usize, usize) {
    // (labels whose chip is crossed by a foreign stroke, labels whose own line is not the nearest)
    let polys: Vec<Vec<(f64, f64)>> = sc.edges.iter().map(|e| {
        if e.waypoints.len() >= 2 { e.waypoints.clone() } else {
            let b = e.bezier; (0..=100).map(|i| { let t = i as f64 / 100.0; let u = 1.0 - t;
            (u*u*u*b[0].0 + 3.0*u*u*t*b[1].0 + 3.0*u*t*t*b[2].0 + t*t*t*b[3].0, u*u*u*b[0].1 + 3.0*u*u*t*b[1].1 + 3.0*u*t*t*b[2].1 + t*t*t*b[3].1) }).collect() }
    }).collect();
    let mut crossed = 0; let mut far = 0;
    for (ei, e) in sc.edges.iter().enumerate() {
        let Some((_, c, w)) = &e.label else { continue };
        let r = (c.0 - w / 2.0, c.1 - 10.0, c.0 + w / 2.0, c.1 + 10.0);
        let rd = |p: &Vec<(f64, f64)>| p.iter().map(|q| { let dx = (r.0 - q.0).max(q.0 - r.2).max(0.0); let dy = (r.1 - q.1).max(q.1 - r.3).max(0.0); dx.hypot(dy) }).fold(f64::INFINITY, f64::min);
        let inside = |p: &Vec<(f64, f64)>| p.iter().any(|q| q.0 > r.0 + 0.5 && q.0 < r.2 - 0.5 && q.1 > r.1 + 0.5 && q.1 < r.3 - 0.5);
        let own = rd(&polys[ei]);
        let other = polys.iter().enumerate().filter(|(j, _)| *j != ei).map(|(_, p)| rd(p)).fold(f64::INFINITY, f64::min);
        if polys.iter().enumerate().any(|(j, p)| j != ei && inside(p)) { crossed += 1; }
        if own > other + 0.5 { far += 1; }
    }
    (crossed, far)
}
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::State(g) => g, _ => panic!() };
        let sc = scene(&g);
        let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let (mut n, mut cr, mut fa, mut panics) = (0, 0, 0, 0);
        let s0 = reads_own(&sc);
        for v in 0..g.nodes.len() {
            for dx in (-3..=3).map(|k| k as f64 * 20.0) {
                for dy in (-3..=3).map(|k| k as f64 * 20.0) {
                    let mut m = base.clone(); m[v].0 += dx; m[v].1 += dy;
                    let r = std::panic::catch_unwind(|| (route_partial(&g, &m, &sc, &base), route(&g, &m)));
                    match r {
                        Ok((rp, rt)) => { for (k, s) in [&rp, &rt].into_iter().enumerate() { let (a, b) = reads_own(s); n += 1; cr += (a > 0) as usize; fa += (b > 0) as usize; if std::env::var("SWEEPV").is_ok() { println!("  {} {} {dx} {dy} {k} {a} {b}", g.nodes[v].id, v); } } }
                        Err(_) => panics += 1,
                    }
                }
            }
        }
        println!("{f}: scene crossed {} far {} | drags {n}: crossed {cr} far {fa} panics {panics}", s0.0, s0.1);
    }
}
