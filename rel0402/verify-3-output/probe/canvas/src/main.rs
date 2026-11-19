// For ER and class route_partial frames (several drags, chained): does
// the canvas hold every box, every waypoint/bezier end, every label
// centre and every class cardinality spot? Compare: route() for the same
// positions. Prints counts of points off the canvas (> 1 px out).
use flowmaid::model::Document;
use flowmaid::{class, er, parser, scene::Scene};

fn off(sc: &Scene, pts: &[(f64, f64)]) -> usize {
    let (ox, oy) = sc.origin;
    let detail = std::env::var("DETAIL").is_ok();
    let mut n = 0;
    for (i, p) in pts.iter().enumerate() {
        if p.0 < ox - 1.0 || p.1 < oy - 1.0 || p.0 > ox + sc.width + 1.0 || p.1 > oy + sc.height + 1.0 {
            n += 1;
            if detail {
                let nn = sc.nodes.len() * 2;
                let ncard: usize = std::env::var("NCARD").ok().and_then(|v| v.parse().ok()).unwrap_or(0); let _ = ncard; let what = if i < nn { format!("node{}", i / 2) } else if i + CARDS.with(|c| c.get()) >= pts.len() { "card".to_string() } else { format!("line_or_label{}", i - nn) };
                eprintln!("  off {what} {:?} canvas o={:?} w={:.2} h={:.2}", p, sc.origin, sc.width, sc.height);
            }
        }
    }
    n
}
fn pts(sc: &Scene, extra: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    for n in &sc.nodes {
        v.push((n.x - n.w / 2.0, n.y - n.h / 2.0));
        v.push((n.x + n.w / 2.0, n.y + n.h / 2.0));
    }
    for e in &sc.edges {
        v.extend(e.waypoints.iter().copied());
        if e.waypoints.is_empty() {
            v.push(e.bezier[0]);
            v.push(e.bezier[3]);
        }
        if let Some((_, c, w)) = &e.label {
            v.push((c.0 - w / 2.0, c.1));
            v.push((c.0 + w / 2.0, c.1));
        }
    }
    v.extend_from_slice(extra);
    v
}
thread_local! { static CARDS: std::cell::Cell<usize> = std::cell::Cell::new(0); }
const DRAGS: [(f64, f64); 5] = [(37.0, 23.0), (-400.0, -300.0), (-900.0, 40.0), (900.0, 700.0), (60.0, -800.0)];
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let Ok(src) = std::fs::read_to_string(&f) else { return };
    let Ok(doc) = parser::parse_document(&src) else { return };
    let (mut fr_off, mut rt_off, mut frames) = (0usize, 0usize, 0usize);
    let ty;
    match &doc {
        Document::Er(d) => {
            ty = "er";
            let base = er::scene(d);
            let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            if c.is_empty() { return; }
            for k in [0, c.len() / 2, c.len() - 1] {
                let mut prev = (base.clone(), c.clone());
                for dd in DRAGS {
                    let mut m = prev.1.clone();
                    m[k] = (m[k].0 + dd.0, m[k].1 + dd.1);
                    let fr = er::route_partial(d, &m, &prev.0, &prev.1);
                    let rt = er::route(d, &m);
                    CARDS.with(|c| c.set(0));
                    fr_off += off(&fr.scene, &pts(&fr.scene, &[]));
                    if std::env::var("DETAIL").is_ok() { eprintln!(" route:"); }
                    rt_off += off(&rt.scene, &pts(&rt.scene, &[]));
                    frames += 1;
                    prev = (fr, m);
                }
            }
        }
        Document::Class(d) => {
            ty = "class";
            let base = class::scene(d);
            let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            if d.classes.is_empty() { return; }
            for k in [0, d.classes.len() / 2, d.classes.len() - 1] {
                let mut prev = (base.clone(), c.clone());
                for dd in DRAGS {
                    let mut m = prev.1.clone();
                    m[k] = (m[k].0 + dd.0, m[k].1 + dd.1);
                    let fr = class::route_partial(d, &m, &prev.0, &prev.1);
                    let rt = class::route(d, &m);
                    if std::env::var("DETAIL").is_ok() { eprintln!("class k={k} dd={dd:?}"); }
                    let cf: Vec<(f64, f64)> = class::card_spots(&fr).iter().flatten().flatten().copied().collect();
                    let cr: Vec<(f64, f64)> = class::card_spots(&rt).iter().flatten().flatten().copied().collect();
                    if std::env::var("DETAIL").is_ok() { eprintln!(" frame: nodes={} edges={} cards={}", fr.scene.nodes.len(), fr.scene.edges.len(), cf.len()); for (j,e) in fr.scene.edges.iter().enumerate() { eprintln!("   e{j} {}->{} wp={:?} bz={:?} lbl={:?}", e.from, e.to, e.waypoints.len(), [e.bezier[0], e.bezier[3]], e.label.as_ref().map(|l| (l.1, l.2))); } eprintln!("   cards {:?}", cf); }
                    CARDS.with(|c| c.set(cf.len()));
                    fr_off += off(&fr.scene, &pts(&fr.scene, &cf));
                    CARDS.with(|c| c.set(cr.len()));
                    if std::env::var("DETAIL").is_ok() { eprintln!(" route:"); }
                    rt_off += off(&rt.scene, &pts(&rt.scene, &cr));
                    frames += 1;
                    prev = (fr, m);
                }
            }
        }
        _ => return,
    }
    println!("{ty}\t{frames}\t{fr_off}\t{rt_off}\t{f}");
}
