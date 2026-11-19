use flowmaid::parser::parse_document;
use flowmaid::Document;
use flowmaid::scene::{route, scene, Scene, SceneEdge};
type P = (f64, f64);
fn cubic(b: &[P; 4], t: f64) -> P {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    (b.iter().zip(w).map(|(p, k)| p.0 * k).sum(), b.iter().zip(w).map(|(p, k)| p.1 * k).sum())
}
fn poly(e: &SceneEdge) -> Vec<P> {
    if e.waypoints.len() >= 2 { e.waypoints.clone() } else { (0..=16).map(|k| cubic(&e.bezier, k as f64 / 16.0)).collect() }
}
fn corners(e: &SceneEdge) -> Vec<P> {
    let w = poly(e);
    let mut out: Vec<P> = vec![];
    for p in w {
        if let Some(&q) = out.last() { if (p.0-q.0).abs()<1e-6 && (p.1-q.1).abs()<1e-6 { continue; } }
        if out.len() >= 2 {
            let (a, b) = (out[out.len()-2], out[out.len()-1]);
            let sx = (a.0-b.0).abs()<1e-6 && (b.0-p.0).abs()<1e-6;
            let sy = (a.1-b.1).abs()<1e-6 && (b.1-p.1).abs()<1e-6;
            if sx || sy { out.pop(); }
        }
        out.push(p);
    }
    // drop rounding points: keep only points where both neighbors are axis-adjacent and turn
    out
}
fn cross(a: P, b: P, c: P, d: P) -> bool {
    let s = |p: P, q: P, r: P| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2, d3, d4) = (s(c, d, a), s(c, d, b), s(a, b, c), s(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}
fn report(sc: &Scene, what: &str, verbose: bool) -> (usize, usize) {
    let polys: Vec<Vec<P>> = sc.edges.iter().map(poly).collect();
    let mut tot = 0; let mut dbl = 0;
    for i in 0..polys.len() { for j in i+1..polys.len() {
        let mut n = 0;
        for a in polys[i].windows(2) { for b in polys[j].windows(2) { if cross(a[0],a[1],b[0],b[1]) { n += 1; } } }
        if n > 0 { tot += n; if n >= 2 { dbl += 1; }
          if verbose { println!("  {what} X {}->{} x {}->{} : {n} [{} {}]", sc.edges[i].from, sc.edges[i].to, sc.edges[j].from, sc.edges[j].to, corners(&sc.edges[i]).len(), corners(&sc.edges[j]).len()); } }
    }}
    (tot, dbl)
}
fn side_of(sc: &Scene, id: &str, p: P) -> &'static str {
    let n = match sc.nodes.iter().find(|n| n.id == id) { Some(n) => n, None => return "?" };
    let (dl, dr, dt, db) = ((p.0 - (n.x - n.w / 2.0)).abs(), (p.0 - (n.x + n.w / 2.0)).abs(), (p.1 - (n.y - n.h / 2.0)).abs(), (p.1 - (n.y + n.h / 2.0)).abs());
    let m = dl.min(dr).min(dt).min(db);
    if m == dl { "L" } else if m == dr { "R" } else if m == dt { "T" } else { "B" }
}
fn note_stats(sc: &Scene, out: &mut std::collections::BTreeMap<String, usize>) {
    let polys: Vec<Vec<P>> = sc.edges.iter().map(poly).collect();
    for (i, l) in sc.edges.iter().enumerate() {
        let (v, vp) = if l.to.starts_with("__note") { (&l.from, polys[i][0]) } else if l.from.starts_with("__note") { (&l.to, *polys[i].last().unwrap()) } else { continue };
        let ls = side_of(sc, v, vp);
        for (j, e) in sc.edges.iter().enumerate() {
            if j == i || e.from.starts_with("__note") || e.to.starts_with("__note") { continue; }
            let ep = if &e.from == v { polys[j][0] } else if &e.to == v { *polys[j].last().unwrap() } else { continue };
            let es = side_of(sc, v, ep);
            let mut n = 0;
            for a in polys[i].windows(2) { for b in polys[j].windows(2) { if cross(a[0],a[1],b[0],b[1]) { n += 1; } } }
            let k = format!("{} {} x{}", if ls == es { "same" } else { "diff" }, corners(e).len().min(6), n.min(2));
            *out.entry(k).or_default() += 1;
        }
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let verbose = std::env::var("V").is_ok();
    let dump = std::env::var("D").is_ok();
    let mut sums = [0usize; 4];
    let mut sums2 = [0f64; 2];
    let mut nstat = std::collections::BTreeMap::new();
    for f in &args[1..] {
        let src = std::fs::read_to_string(f).unwrap();
        let g = match parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        let sc = scene(&g);
        let centers: Vec<P> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rs = route(&g, &centers);
        if dump {
            for n in &sc.nodes { println!("N {} {:.1} {:.1}", n.id, n.x, n.y); }
            for (k, e) in sc.edges.iter().enumerate() {
                let c: Vec<String> = corners(e).iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect();
                println!("S{k} {}->{}: {}", e.from, e.to, c.join(" "));
            }
            for (k, e) in rs.edges.iter().enumerate() {
                let c: Vec<String> = corners(e).iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect();
                println!("R{k} {}->{}: {}", e.from, e.to, c.join(" "));
            }
        }
        let (ms, mr) = (flowmaid::quality::measure(&g, &sc), flowmaid::quality::measure(&g, &rs));
        let xd = |m: &flowmaid::quality::Metrics| m.fields().into_iter().chain(m.late_fields()).chain(m.tail_fields()).find(|f| f.0 == "x_drawn").map(|f| f.1).unwrap_or(-1.0);
        let (xs, xr) = (xd(&ms), xd(&mr));
        sums2[0] += xs; sums2[1] += xr;
        note_stats(&sc, &mut nstat);
        let (a, b) = report(&sc, "S", verbose);
        let (c, d) = report(&rs, "R", verbose);
        sums[0]+=a; sums[1]+=b; sums[2]+=c; sums[3]+=d;
        if args.len() <= 30 || b > 0 || d > 0 || std::env::var("ALL").is_ok() { println!("{}\tS {a} dbl {b} xd {xs}\tR {c} dbl {d} xd {xr}", f); }
    }
    if std::env::var("NS").is_ok() { for (k, v) in &nstat { println!("NS {k} {v}"); } }
    println!("TOTAL\tS {} dbl {} xd {}\tR {} dbl {} xd {}", sums[0], sums[1], sums2[0], sums[2], sums[3], sums2[1]);
}
