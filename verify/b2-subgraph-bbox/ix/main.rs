use flowmaid::model::{End, Graph};
use flowmaid::scene::{route, route_partial, scene, to_svg, Scene};

type P = (f64, f64);

fn parse_paths(svg: &str) -> Vec<(String, Vec<P>, f64)> {
    // (title, dense polyline, stroke width) for each edge path (title contains →)
    let mut out = Vec::new();
    for chunk in svg.split("<path ").skip(1) {
        let end = match chunk.find("</path>") { Some(e) => e, None => continue };
        let el = &format!(" {}", &chunk[..end]);
        let title = match el.split("<title>").nth(1) { Some(t) => t.split("</title>").next().unwrap().to_string(), None => continue };
        if !title.contains('→') { continue; }
        let d = el.split(" d=\"").nth(1).unwrap().split('"').next().unwrap();
        let sw: f64 = el.split(" stroke-width=\"").nth(1).map(|s| s.split('"').next().unwrap().parse().unwrap_or(2.0)).unwrap_or(2.0);
        let toks: Vec<&str> = d.split(|c: char| c == ' ' || c == ',').filter(|t| !t.is_empty()).collect();
        let mut pts: Vec<P> = Vec::new();
        let mut i = 0; let mut op = 'M'; let mut at = (0.0, 0.0);
        let num = |s: &str| s.parse::<f64>().ok();
        while i < toks.len() {
            if num(toks[i]).is_none() { op = toks[i].chars().next().unwrap(); i += 1; continue; }
            let take = |k: usize, i: usize| -> Vec<f64> { (0..k).map(|j| toks[i + j].parse::<f64>().unwrap()).collect() };
            match op {
                'M' | 'L' => { let a = take(2, i); i += 2; let p = (a[0], a[1]);
                    if op == 'L' { for s in 1..=20 { let t = s as f64 / 20.0; pts.push((at.0 + (p.0 - at.0) * t, at.1 + (p.1 - at.1) * t)); } } else { pts.push(p); }
                    at = p; }
                'C' => { let a = take(6, i); i += 6; let (c1, c2, p) = ((a[0], a[1]), (a[2], a[3]), (a[4], a[5]));
                    for s in 1..=40 { let t = s as f64 / 40.0; let u = 1.0 - t;
                        pts.push((u*u*u*at.0 + 3.0*u*u*t*c1.0 + 3.0*u*t*t*c2.0 + t*t*t*p.0, u*u*u*at.1 + 3.0*u*u*t*c1.1 + 3.0*u*t*t*c2.1 + t*t*t*p.1)); }
                    at = p; }
                'Q' => { let a = take(4, i); i += 4; let (c1, p) = ((a[0], a[1]), (a[2], a[3]));
                    for s in 1..=30 { let t = s as f64 / 30.0; let u = 1.0 - t;
                        pts.push((u*u*at.0 + 2.0*u*t*c1.0 + t*t*p.0, u*u*at.1 + 2.0*u*t*c1.1 + t*t*p.1)); }
                    at = p; }
                _ => { panic!("unhandled op {op} in {d}"); }
            }
        }
        out.push((title, pts, sw));
    }
    out
}

fn dist_seg(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1); let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}
fn dist_poly(p: P, pl: &[P]) -> f64 { pl.windows(2).map(|w| dist_seg(p, w[0], w[1])).fold(f64::INFINITY, f64::min) }

fn end_id(g: &Graph, e: End) -> String { match e { End::Node(v) => g.nodes[v].id.clone(), End::Sub(s) => g.subgraphs[s].id.clone() } }

fn check(name: &str, what: &str, g: &Graph, sc: &Scene, bad: &mut Vec<String>) {
    // index-parallel
    for (i, e) in g.edges.iter().enumerate() {
        let se = &sc.edges[i];
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id { bad.push(format!("{name} {what}: edge {i} not parallel ({}->{} vs {}->{})", se.from, se.to, g.nodes[e.from].id, g.nodes[e.to].id)); }
    }
    let mut k = g.edges.len();
    for se in &g.sub_edges { if k < sc.edges.len() && sc.edges[k].from == end_id(g, se.from) && sc.edges[k].to == end_id(g, se.to) { k += 1; } }
    if k != sc.edges.len() { bad.push(format!("{name} {what}: sub-edges after edges mismatch: matched up to {k} of {}", sc.edges.len())); }
    // waypoint density: max gap between consecutive waypoints
    let mut maxgap: f64 = 0.0; let mut worst = String::new();
    for e in &sc.edges { for w in e.waypoints.windows(2) { let d = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt(); if d > maxgap { maxgap = d; worst = format!("{}->{}", e.from, e.to); } } }
    // pick == render
    let svg = to_svg(sc);
    let vb = format!("viewBox=\"0 0 {:.0} {:.0}\"", sc.width, sc.height);
    let mut paths = parse_paths(&svg);
    let drawn0: Vec<usize> = (0..sc.edges.len()).filter(|&i| !matches!(sc.edges[i].kind, flowmaid::model::EdgeKind::Invisible)).collect();
    let mut aligned = svg.contains(&vb);
    if !aligned && paths.len() == drawn0.len() {
        // estimate the export's top-left shift from path starts
        let mut dxs: Vec<(i64, i64)> = Vec::new();
        for (pi, (_, pl, _)) in paths.iter().enumerate() {
            let e = &sc.edges[drawn0[pi]];
            let s0 = if e.waypoints.is_empty() { e.bezier[0] } else { e.waypoints[0] };
            dxs.push((((pl[0].0 - s0.0) * 10.0).round() as i64, ((pl[0].1 - s0.1) * 10.0).round() as i64));
        }
        let mut best = (0, (0i64, 0i64));
        for c in &dxs { let n = dxs.iter().filter(|x| *x == c).count(); if n > best.0 { best = (n, *c); } }
        let (dx, dy) = (best.1 .0 as f64 / 10.0, best.1 .1 as f64 / 10.0);
        if best.0 * 2 > dxs.len() {
            for (_, pl, _) in paths.iter_mut() { for q in pl.iter_mut() { q.0 -= dx; q.1 -= dy; } }
            aligned = true;
            println!("  [{name} {what}] export shift ({dx},{dy}) from {}/{} starts", best.0, dxs.len());
        }
    }
    let drawn: Vec<usize> = (0..sc.edges.len()).filter(|&i| !matches!(sc.edges[i].kind, flowmaid::model::EdgeKind::Invisible)).collect();
    let mut misses = 0; let mut samples = 0;
    if aligned && paths.len() == drawn.len() {
        for (pi, (title, pl, sw)) in paths.iter().enumerate() {
            let i = drawn[pi];
            let e = &sc.edges[i];
            if !title.starts_with(&format!("{} → {}", e.from, e.to)) { bad.push(format!("{name} {what}: path {pi} title {title} vs edge {i} {}->{}", e.from, e.to)); continue; }
            let n = pl.len(); if n < 4 { continue; }
            let (a, b) = (pl[0], pl[n - 1]);
            for q in pl.iter().step_by(3) {
                let near_end = ((q.0 - a.0).hypot(q.1 - a.1) < 14.0) || ((q.0 - b.0).hypot(q.1 - b.1) < 14.0);
                if near_end { continue; }
                samples += 1;
                match sc.edge_at(q.0, q.1, 0.5) {
                    Some(j) if j == i => {}
                    Some(j) => {
                        // accept if the point is on j's painted stroke too
                        let pj = drawn.iter().position(|&x| x == j).map(|p| &paths[p]);
                        let ok = pj.map_or(false, |(_, plj, swj)| dist_poly(*q, plj) <= swj / 2.0 + 1.0);
                        if !ok { misses += 1; if misses <= 3 { bad.push(format!("{name} {what}: point {:?} on {}->{} picks edge {j} ({}->{})", q, e.from, e.to, sc.edges[j].from, sc.edges[j].to)); } }
                    }
                    None => { let _ = sw; misses += 1; if misses <= 3 { bad.push(format!("{name} {what}: point {:?} on {}->{} picks nothing", q, e.from, e.to)); } }
                }
            }
        }
    } else {
        println!("  [{name} {what}] skip pick check: aligned={aligned} paths={} drawn={}", paths.len(), drawn.len());
    }
    println!("  {name:14} {what:8} edges={:3} maxgap={:6.1} ({worst}) picks={samples} misses={misses} size={:.0}x{:.0}", sc.edges.len(), maxgap, sc.width, sc.height);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outdir = &args[0];
    let mut bad = Vec::new();
    for path in &args[1..] {
        let name = path.rsplit('/').next().unwrap().trim_end_matches(".mmd").to_string();
        let src = std::fs::read_to_string(path).unwrap();
        let doc = flowmaid::parser::parse_document(&src).unwrap();
        let g = match doc { flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g, _ => { continue } };
        let mm = flowmaid::to_mermaid(&g);
        std::fs::write(format!("{outdir}/{name}.to_mermaid.txt"), &mm).unwrap();
        // round trip
        if let Ok(g2) = flowmaid::parser::parse(&mm) {
            let mm2 = flowmaid::to_mermaid(&g2);
            if mm2 != mm { bad.push(format!("{name}: to_mermaid round trip not stable")); }
            let mem = |g: &Graph| -> Vec<(String, Vec<String>)> { let mut v: Vec<_> = g.subgraphs.iter().map(|s| { let mut n: Vec<String> = s.nodes.iter().map(|&i| g.nodes[i].id.clone()).collect(); n.sort(); (s.id.clone(), n) }).collect(); v.sort(); v };
            if mem(&g) != mem(&g2) { bad.push(format!("{name}: membership changes on round trip {:?} vs {:?}", mem(&g), mem(&g2))); }
        } else { println!("  {name}: to_mermaid output does not parse as flowchart (state?)"); }
        let s = scene(&g);
        let s_again = scene(&g);
        if to_svg(&s) != to_svg(&s_again) { bad.push(format!("{name}: static scene not deterministic")); }
        check(&name, "scene", &g, &s, &mut bad);
        let pos: Vec<P> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&g, &pos);
        let r2 = route(&g, &pos);
        if to_svg(&r) != to_svg(&r2) { bad.push(format!("{name}: route not deterministic")); }
        check(&name, "route", &g, &r, &mut bad);
        // drag every node a bit, one at a time, with route_partial + route
        let mut worst_partial: f64 = 0.0;
        for v in 0..g.nodes.len().min(40) {
            let mut moved = pos.clone();
            moved[v] = (pos[v].0 + 23.0, pos[v].1 - 17.0);
            let t = std::time::Instant::now();
            let p = route_partial(&g, &moved, &r, &pos);
            worst_partial = worst_partial.max(t.elapsed().as_secs_f64() * 1e3);
            let p2 = route_partial(&g, &moved, &r, &pos);
            if to_svg(&p) != to_svg(&p2) { bad.push(format!("{name}: route_partial not deterministic (node {v})")); }
            if v % 3 == 0 { check(&name, &format!("part{v}"), &g, &p, &mut bad); }
            let full = route(&g, &moved);
            if v % 5 == 0 { check(&name, &format!("drag{v}"), &g, &full, &mut bad); }
        }
        println!("  {name:14} worst route_partial (single, unwarmed) {:.2} ms", worst_partial);
        std::fs::write(format!("{outdir}/{name}.route.svg"), to_svg(&r)).unwrap();
    }
    println!("PROBLEMS: {}", bad.len());
    for b in &bad { println!("  {b}"); }
}
