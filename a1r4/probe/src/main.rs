use flowmaid::class::{card_spots, rel_head, route, scene, to_svg, ClassScene};
use flowmaid::model::Document;
use flowmaid::scene::SceneEdge;
use std::time::Instant;

fn poly(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 { return e.waypoints.clone(); }
    let [p0, p1, p2, p3] = e.bezier;
    (0..=32).map(|i| { let t = i as f64 / 32.0; let u = 1.0 - t;
        (u*u*u*p0.0 + 3.0*u*u*t*p1.0 + 3.0*u*t*t*p2.0 + t*t*t*p3.0,
         u*u*u*p0.1 + 3.0*u*u*t*p1.1 + 3.0*u*t*t*p2.1 + t*t*t*p3.1) }).collect()
}
fn ends(e: &SceneEdge) -> [(f64, f64); 2] { let p = poly(e); [p[0], p[p.len() - 1]] }
fn d(a: (f64, f64), b: (f64, f64)) -> f64 { (a.0 - b.0).hypot(a.1 - b.1) }

fn check(tag: &str, cs: &ClassScene, names: &[String]) {
    let sc = &cs.scene;
    let nrel = cs.rels.len();
    // cards: each card should be nearer its own end than any other edge's end
    let spots = card_spots(cs);
    for (j, s) in spots.iter().enumerate() {
        let my = ends(&sc.edges[j]);
        for k in 0..2 {
            if let Some(p) = s[k] {
                let own = d(p, my[k]);
                let mut best = (own, j, k);
                for (i, e) in sc.edges.iter().enumerate().take(nrel) {
                    let en = ends(e);
                    for kk in 0..2 {
                        if i == j && kk == k { continue; }
                        let dd = d(p, en[kk]);
                        if dd + 6.0 < best.0 { best = (dd, i, kk); }
                    }
                }
                if best.1 != j || best.2 != k {
                    println!("  {tag}: card of rel {j} ({}->{}) end {k} at ({:.0},{:.0}) is {:.0}px from own end but {:.0}px from rel {} end {}", sc.edges[j].from, sc.edges[j].to, p.0, p.1, own, best.0, best.1, best.2);
                }
                if own > 40.0 { println!("  {tag}: card of rel {j} end {k} is {:.0}px from its end", own); }
            }
        }
    }
    // glyph tips coinciding
    let heads: Vec<_> = (0..nrel).map(|j| rel_head(cs, j)).collect();
    let _ = heads;
    let mut tips = vec![];
    for j in 0..nrel { let e = ends(&sc.edges[j]); tips.push((j, e[1])); }
    for a in 0..tips.len() { for b in a+1..tips.len() {
        if d(tips[a].1, tips[b].1) < 6.0 { println!("  {tag}: glyph tips of rel {} and {} within {:.1}px at ({:.1},{:.1})", tips[a].0, tips[b].0, d(tips[a].1, tips[b].1), tips[a].1 .0, tips[a].1 .1); }
    }}
    // pick consistency
    for (j, e) in sc.edges.iter().enumerate() {
        let pts = poly(e);
        let mid = pts[pts.len() / 2];
        if sc.node_at(mid.0, mid.1).is_none() {
            match sc.edge_at(mid.0, mid.1, 3.0) {
                Some(i) if i == j => {}
                Some(i) => { let op = poly(&sc.edges[i]); let dd = op.iter().map(|p| d(*p, mid)).fold(f64::INFINITY, f64::min); if dd > 2.0 { println!("  {tag}: pick at mid of edge {j} returned {i} (d={dd:.1})") } }
                None => println!("  {tag}: pick at mid of edge {j} returned None"),
            }
        }
    }
    for (j, e) in sc.edges.iter().enumerate().take(nrel) {
        if e.waypoints.len() < 3 { continue; }
        let mut c: Vec<(f64,f64)> = Vec::new();
        for &p in &e.waypoints {
            if let Some(q) = c.last() { if (q.0-p.0).abs() < 0.5 && (q.1-p.1).abs() < 0.5 { continue; } }
            if c.len() >= 2 { let u = c[c.len()-2]; let v = c[c.len()-1];
                let inl = ((u.0-v.0).abs() < 0.5 && (v.0-p.0).abs() < 0.5) || ((u.1-v.1).abs() < 0.5 && (v.1-p.1).abs() < 0.5);
                if inl { c.pop(); } }
            c.push(p);
        }
        let ortho = c.windows(2).all(|w| (w[0].0-w[1].0).abs() < 0.5 || (w[0].1-w[1].1).abs() < 0.5);
        if !ortho || c.len() < 3 { continue; }
        let n = c.len();
        let last = d(c[n-1], c[n-2]);
        let first = d(c[0], c[1]);
        if last < 20.0 { println!("  {tag}: edge {j} final leg {last:.1}px"); }
        if first < 20.0 { println!("  {tag}: edge {j} first leg {first:.1}px"); }
    }
    println!("{tag}: nodes={} edges={} rels={} notes={} w={:.0} h={:.0}", sc.nodes.len(), sc.edges.len(), nrel, cs.notes.len(), sc.width, sc.height);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::Path::new(&args[0]);
    std::fs::create_dir_all(out).unwrap();
    for f in &args[1..] {
        let src = std::fs::read_to_string(f).unwrap();
        let name = std::path::Path::new(f).file_stem().unwrap().to_str().unwrap().to_string();
        let d = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Class(d)) => d,
            Ok(_) => { println!("{name}: not class"); continue; }
            Err(e) => { println!("{name}: parse error {e:?}"); continue; }
        };
        let names: Vec<String> = d.classes.iter().map(|c| c.name.clone()).collect();
        let n = d.classes.len();
        let cs = scene(&d);
        check(&format!("{name}/scene"), &cs, &names);
        let s_a = to_svg(&cs); let s_b = to_svg(&scene(&d));
        if s_a != s_b { println!("{name}: NONDETERMINISTIC scene"); }
        let all: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|nd| (nd.x, nd.y)).collect();
        let centers: Vec<(f64, f64)> = all.iter().take(n).cloned().collect();
        let r0 = route(&d, &all);
        check(&format!("{name}/route-id"), &r0, &names);
        if to_svg(&r0) != to_svg(&route(&d, &all)) { println!("{name}: NONDETERMINISTIC route"); }
        if n == 0 { continue; }
        let iters = 40;
        let t = Instant::now();
        let mut acc = 0usize;
        let mut worst = 0.0f64;
        for k in 0..iters {
            let mut c = centers.clone();
            c[0].0 += k as f64 * 3.0; c[0].1 += k as f64 * 1.0;
            let t1 = Instant::now();
            let r = route(&d, &c);
            worst = worst.max(t1.elapsed().as_secs_f64() * 1000.0);
            acc += to_svg(&r).len();
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0 / iters as f64;
        println!("{name}: drag route+to_svg avg {ms:.3} ms, worst route {worst:.3} ms (acc {acc})");
        if n >= 2 {
            let mut c = centers.clone(); c.swap(0, 1);
            let r = route(&d, &c); check(&format!("{name}/swap01"), &r, &names);
            std::fs::write(out.join(format!("{name}.swap01.svg")), to_svg(&r)).unwrap();
            let mut c = centers.clone(); c[n / 2] = (-300.0, -200.0);
            let r = route(&d, &c); check(&format!("{name}/negative"), &r, &names);
            std::fs::write(out.join(format!("{name}.negative.svg")), to_svg(&r)).unwrap();
            let maxy = centers.iter().map(|c| c.1).fold(0.0, f64::max);
            let c: Vec<_> = centers.iter().map(|&(x, y)| (x, maxy - y + 40.0)).collect();
            let r = route(&d, &c); check(&format!("{name}/flipped"), &r, &names);
            std::fs::write(out.join(format!("{name}.flipped.svg")), to_svg(&r)).unwrap();
        }
    }
}
