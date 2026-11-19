// Canvas check for ER / class route_partial frames over the whole drawing
// (boxes, every edge waypoint and bezier end, label chips, class
// cardinalities): chains of four frames dragging boxes past each side in
// turn, from the laid-out scene and from a route base. Each frame must
// (a) hold the drawing on its canvas and inside its SVG viewBox,
// (b) never shrink from its base's canvas, and (c) keep positions.
// usage: canvas3 <file.mmd>...
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::scene::Scene;
use flowmaid::{class, er};

fn view_box(svg: &str) -> (f64, f64, f64, f64) {
    let at = svg.find("viewBox=\"").unwrap() + 9;
    let v: Vec<f64> = svg[at..].split('"').next().unwrap().split(' ').map(|t| t.parse().unwrap()).collect();
    (v[0], v[1], v[2], v[3])
}

fn drawing(sc: &Scene, extra: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut b = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut add = |x: f64, y: f64| {
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    };
    for n in &sc.nodes {
        add(n.x - n.w / 2.0, n.y - n.h / 2.0);
        add(n.x + n.w / 2.0, n.y + n.h / 2.0);
    }
    for e in &sc.edges {
        for p in &e.waypoints {
            add(p.0, p.1);
        }
        add(e.bezier[0].0, e.bezier[0].1);
        add(e.bezier[3].0, e.bezier[3].1);
        if let Some((_, m, w)) = &e.label {
            add(m.0 - w / 2.0, m.1 - 10.0);
            add(m.0 + w / 2.0, m.1 + 10.0);
        }
    }
    for p in extra {
        add(p.0, p.1);
    }
    b
}

struct Tally {
    frames: usize,
    bad: usize,
}

fn check(t: &mut Tally, what: &str, base: &Scene, fr: &Scene, svg: &str, extra: &[(f64, f64)], at: &[(f64, f64)]) -> bool {
    t.frames += 1;
    let (ox, oy) = fr.origin;
    let (fx, fy) = (ox + fr.width, oy + fr.height);
    let (bx, by) = (base.origin.0 + base.width, base.origin.1 + base.height);
    let d = drawing(fr, &[]);
    let dc = drawing(fr, extra);
    if dc != d && (dc.0 < ox - 1e-6 || dc.1 < oy - 1e-6 || dc.2 > fx + 1e-6 || dc.3 > fy + 1e-6) {
        println!("CARD-OFF {what}");
    }
    let (vx, vy, vw, vh) = view_box(svg);
    let mut why = Vec::new();
    if d.0 < ox - 1e-6 || d.1 < oy - 1e-6 || d.2 > fx + 1e-6 || d.3 > fy + 1e-6 {
        why.push(format!("drawing {d:?} off canvas ({ox},{oy})-({fx},{fy})"));
    }
    if d.0 < vx - 0.5 || d.1 < vy - 0.5 || d.2 > vx + vw + 0.5 || d.3 > vy + vh + 0.5 {
        why.push(format!("drawing {d:?} off viewBox {vx} {vy} {vw} {vh}"));
    }
    if fx < bx - 1e-9 || fy < by - 1e-9 || ox > base.origin.0 + 1e-9 || oy > base.origin.1 + 1e-9 {
        why.push(format!("canvas shrank: base ({:?}) far ({bx},{by}) frame ({ox},{oy}) far ({fx},{fy})", base.origin));
    }
    for (i, c) in at.iter().enumerate() {
        let n = &fr.nodes[i];
        if n.x != c.0 || n.y != c.1 {
            why.push(format!("node {i} moved: ({},{}) vs ({},{})", n.x, n.y, c.0, c.1));
            break;
        }
    }
    if !why.is_empty() {
        t.bad += 1;
        println!("BAD {what}: {}", why.join("; "));
        return false;
    }
    true
}

/// Whether `sc`'s drawing lies on its own canvas, and what reaches past it.
fn own(sc: &Scene) -> String {
    let (ox, oy) = sc.origin;
    let (fx, fy) = (ox + sc.width, oy + sc.height);
    let mut out = Vec::new();
    for (j, e) in sc.edges.iter().enumerate() {
        for p in &e.waypoints {
            if p.0 < ox - 1e-6 || p.1 < oy - 1e-6 || p.0 > fx + 1e-6 || p.1 > fy + 1e-6 {
                out.push(format!("edge {j} waypoint {p:?}"));
                break;
            }
        }
        if let Some((_, m, w)) = &e.label {
            if m.0 - w / 2.0 < ox - 1e-6 || m.1 - 10.0 < oy - 1e-6 || m.0 + w / 2.0 > fx + 1e-6 || m.1 + 10.0 > fy + 1e-6 {
                out.push(format!("edge {j} label at {m:?}"));
            }
        }
    }
    for n in &sc.nodes {
        if n.x - n.w / 2.0 < ox - 1e-6 || n.y - n.h / 2.0 < oy - 1e-6 || n.x + n.w / 2.0 > fx + 1e-6 || n.y + n.h / 2.0 > fy + 1e-6 {
            out.push(format!("node {}", n.id));
        }
    }
    if out.is_empty() { "ok".into() } else { out.join(", ") }
}

fn cards(cs: &class::ClassScene) -> Vec<(f64, f64)> {
    class::card_spots(cs).into_iter().flatten().flatten().collect()
}

/// The positions of the frames of one drag chain: box 0 past the top
/// left, the last box past the bottom right, the middle box past the
/// bottom left, box 0 past the top right.
fn chain(at: &[(f64, f64)], k: usize, canvas: (f64, f64, f64, f64)) -> Vec<Vec<(f64, f64)>> {
    let (ox, oy, w, h) = canvas;
    let mut out = Vec::new();
    let mut p = at.to_vec();
    p[0] = (ox - 150.0, oy - 90.0);
    out.push(p.clone());
    p[k - 1] = (ox + w + 600.0, oy + h + 400.0);
    out.push(p.clone());
    p[k / 2] = (ox - 220.0, oy + h + 700.0);
    out.push(p.clone());
    p[0] = (ox + w + 900.0, oy - 300.0);
    out.push(p.clone());
    out
}

fn main() {
    let mut t = Tally { frames: 0, bad: 0 };
    let mut files = 0;
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let Ok(doc) = parse_document(&src) else { continue };
        match doc {
            Document::Er(d) if d.entities.len() >= 2 => {
                files += 1;
                let laid = er::scene(&d);
                let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let s = &laid.scene;
                let steps = chain(&at, at.len(), (s.origin.0, s.origin.1, s.width, s.height));
                for start in ["laid", "route"] {
                    let (mut base, mut base_at) = if start == "laid" {
                        (laid.clone(), at.clone())
                    } else {
                        (er::route(&d, &steps[0]), steps[0].clone())
                    };
                    for (i, p) in steps.iter().enumerate() {
                        if start == "route" && i == 0 {
                            continue;
                        }
                        let fr = er::route_partial(&d, p, &base, &base_at);
                        check(&mut t, &format!("{f} er {start} step {i}"), &base.scene, &fr.scene, &er::to_svg(&fr), &[], p);
                        base = fr;
                        base_at = p.clone();
                    }
                }
            }
            Document::Class(d) if d.classes.len() >= 2 => {
                files += 1;
                let laid = class::scene(&d);
                let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let s = &laid.scene;
                let k = d.classes.len();
                let mut sets = vec![("all", at.clone())];
                if at.len() > k {
                    sets.push(("classes", at[..k].to_vec()));
                }
                for (which, at) in sets {
                    let steps = chain(&at, k, (s.origin.0, s.origin.1, s.width, s.height));
                    for start in ["laid", "route"] {
                        let (mut base, mut base_at) = if start == "laid" {
                            (class::route(&d, &at), at.clone())
                        } else {
                            (class::route(&d, &steps[0]), steps[0].clone())
                        };
                        if start == "laid" && which == "all" {
                            base = laid.clone();
                        }
                        for (i, p) in steps.iter().enumerate() {
                            if start == "route" && i == 0 {
                                continue;
                            }
                            let fr = class::route_partial(&d, p, &base, &base_at);
                            let n = p.len();
                            let ok = check(
                                &mut t,
                                &format!("{f} class {which} {start} step {i}"),
                                &base.scene,
                                &fr.scene,
                                &class::to_svg(&fr),
                                &cards(&fr),
                                &p[..n.min(k)],
                            );
                            if !ok {
                                println!("  frame: {}", own(&fr.scene));
                                println!("  route at the same positions: {}", own(&class::route(&d, p).scene));
                            }
                            base = fr;
                            base_at = p.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    println!("files {files} frames {} bad {}", t.frames, t.bad);
}
