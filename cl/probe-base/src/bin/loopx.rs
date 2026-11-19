//! Self-loop defects in class diagrams: a loop (or its label chip) meeting another box,
//! another class's loop, or another loop's label.
use flowmaid::model::Document;
use flowmaid::scene::SceneEdge;

type Rect = (f64, f64, f64, f64);

fn poly(e: &SceneEdge) -> Vec<(f64, f64)> {
    if !e.waypoints.is_empty() {
        return e.waypoints.clone();
    }
    let b = e.bezier;
    (0..=32)
        .map(|k| {
            let t = k as f64 / 32.0;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            (
                w[0] * b[0].0 + w[1] * b[1].0 + w[2] * b[2].0 + w[3] * b[3].0,
                w[0] * b[0].1 + w[1] * b[1].1 + w[2] * b[2].1 + w[3] * b[3].1,
            )
        })
        .collect()
}
fn bbox(p: &[(f64, f64)]) -> Rect {
    p.iter().fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |b, q| {
        (b.0.min(q.0), b.1.min(q.1), b.2.max(q.0), b.3.max(q.1))
    })
}
fn meet(a: Rect, b: Rect) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}
fn seg_hits(r: Rect, a: (f64, f64), b: (f64, f64)) -> bool {
    // sample the segment
    (0..=8).any(|k| {
        let t = k as f64 / 8.0;
        let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        r.0 < p.0 && p.0 < r.2 && r.1 < p.1 && p.1 < r.3
    })
}
fn cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let s = |o: (f64, f64), p: (f64, f64), q: (f64, f64)| (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0);
    s(c, d, a) * s(c, d, b) < -1e-9 && s(a, b, c) * s(a, b, d) < -1e-9
}

fn main() {
    let mut total = 0;
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let Ok(Document::Class(d)) = flowmaid::parser::parse_document(&src) else { continue };
        let cs = flowmaid::class::scene(&d);
        let sc = &cs.scene;
        let chip = |e: &SceneEdge| e.label.as_ref().map(|(_, m, w)| (m.0 - w / 2.0, m.1 - 11.0, m.0 + w / 2.0, m.1 + 11.0));
        let mut bad: Vec<String> = Vec::new();
        let loops: Vec<&SceneEdge> = sc.edges.iter().filter(|e| e.from == e.to).collect();
        for (i, e) in loops.iter().enumerate() {
            let line = poly(e);
            let c = chip(e);
            for n in sc.nodes.iter().filter(|n| n.id != e.from) {
                let r = (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0);
                if line.windows(2).any(|s| seg_hits(r, s[0], s[1])) {
                    bad.push(format!("loop {}#{i} into box {}", e.from, n.id));
                }
                if c.is_some_and(|c| meet(c, r)) {
                    bad.push(format!("label {}#{i} over box {}", e.from, n.id));
                }
            }
            for (k, o) in loops.iter().enumerate() {
                if k == i {
                    continue;
                }
                let other = poly(o);
                if c.is_some_and(|c| other.windows(2).any(|s| seg_hits(c, s[0], s[1]))) {
                    bad.push(format!("label {}#{i} over loop {}#{k}", e.from, o.from));
                }
                if k > i {
                    if let (Some(a), Some(b)) = (c, chip(o)) {
                        if meet(a, b) {
                            bad.push(format!("labels {}#{i} / {}#{k} overlap", e.from, o.from));
                        }
                    }
                    if o.from != e.from && meet(bbox(&line), bbox(&other)) {
                        bad.push(format!("loop {}#{i} region meets loop {}#{k}", e.from, o.from));
                    }
                    if line.windows(2).any(|p| other.windows(2).any(|q| cross(p[0], p[1], q[0], q[1]))) {
                        bad.push(format!("loop {}#{i} crosses loop {}#{k}", e.from, o.from));
                    }
                }
            }
            // other (non-loop) edges through the chip or crossing the loop
            for o in sc.edges.iter().filter(|o| o.from != o.to) {
                let other = poly(o);
                if c.is_some_and(|c| other.windows(2).any(|s| seg_hits(c, s[0], s[1]))) {
                    bad.push(format!("label {}#{i} over edge {}>{}", e.from, o.from, o.to));
                }
                if line.windows(2).any(|p| other.windows(2).any(|q| cross(p[0], p[1], q[0], q[1]))) {
                    bad.push(format!("loop {}#{i} crosses edge {}>{}", e.from, o.from, o.to));
                }
            }
        }
        total += bad.len();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name}\tbad={}\t{}x{}\t{}", bad.len(), sc.width.round(), sc.height.round(), bad.join("; "));
    }
    println!("TOTAL bad={total}");
}
