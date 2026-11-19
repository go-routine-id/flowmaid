use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser};
use std::fmt::Write as _;
use std::time::Instant;

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
fn grid(w: f64, h: f64, n: usize) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    for i in 0..=n {
        for j in 0..=n {
            v.push((-5.0 + (w + 10.0) * i as f64 / n as f64, -5.0 + (h + 10.0) * j as f64 / n as f64));
        }
    }
    v
}
const TOLS: [f64; 5] = [-1.0, 0.0, 2.0, 6.0, 40.0];

/// Picks over a grid at several tolerances, as one string; plus NaN probes.
fn pick_dump<H: std::fmt::Debug>(w: f64, h: f64, f: impl Fn(f64, f64, f64) -> Option<H>) -> String {
    let mut s = String::new();
    for (x, y) in grid(w, h, 60) {
        for t in TOLS {
            let _ = write!(s, "{:?}", f(x, y, t));
        }
    }
    let _ = write!(s, "{:?}{:?}{:?}", f(f64::NAN, 1.0, 1.0), f(1.0, f64::INFINITY, 1.0), f(1.0, 1.0, f64::NAN));
    s
}
/// Pick timing: (mean us, max us) over the grid at tol 4.
fn pick_time<H>(w: f64, h: f64, f: impl Fn(f64, f64, f64) -> Option<H>) -> (f64, f64, usize) {
    // per point: the min of 7 timed calls (robust to preemption); then
    // the mean and max of those over the grid.
    let pts = grid(w, h, 80);
    let mut tot = 0.0;
    let mut mx: f64 = 0.0;
    let mut hits = 0;
    for &(x, y) in &pts {
        let mut best = f64::INFINITY;
        for _ in 0..7 {
            let t = Instant::now();
            let r = std::hint::black_box(f(x, y, 4.0));
            best = best.min(t.elapsed().as_secs_f64() * 1e6);
            if r.is_some() && best.is_finite() { }
        }
        if f(x, y, 4.0).is_some() { hits += 1; }
        tot += best;
        mx = mx.max(best);
    }
    (tot / pts.len() as f64, mx, hits)
}

fn moved(c: &[(f64, f64)], k: usize, d: (f64, f64)) -> Vec<(f64, f64)> {
    let mut m = c.to_vec();
    m[k] = (c[k].0 + d.0, c[k].1 + d.1);
    m
}
const DRAGS: [(f64, f64); 4] = [(37.0, 23.0), (-400.0, -300.0), (900.0, 700.0), (-0.04, 0.03)];

fn main() {
    let mode = std::env::args().nth(1).unwrap();
    let f = std::env::args().nth(2).unwrap();
    let Ok(src) = std::fs::read_to_string(&f) else { return };
    let Ok(doc) = parser::parse_document(&src) else { return };
    let mut o = String::new(); // determinism dump
    let mut bad: Vec<String> = Vec::new(); // contract failures
    let mut ty = "";
    let mut timing = None;
    let mut size = 0usize;
    match &doc {
        Document::Er(d) => {
            ty = "er";
            let base = er::scene(d);
            size = base.scene.nodes.len() + base.scene.edges.len();
            let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            if c.is_empty() { return; }
            let k = c.len() / 2;
            let id = base.scene.nodes[k].id.clone();
            let mut prev = (base.clone(), c.clone());
            for (di, dd) in DRAGS.iter().enumerate() {
                let m = moved(&c, k, *dd);
                let fr = er::route_partial(d, &m, &base, &c);
                let fr2 = er::route_partial(d, &m, &base, &c);
                let a = format!("{fr:?}");
                if a != format!("{fr2:?}") { bad.push(format!("er nondet d{di}")); }
                let _ = write!(o, "{}|{}|", fnv(&a), fnv(&er::to_svg(&fr)));
                if Some(&fr.scene) .is_some() {
                    for (j, (e, b)) in fr.scene.edges.iter().zip(&base.scene.edges).enumerate() {
                        if e.from != id && e.to != id && format!("{e:?}") != format!("{b:?}") {
                            bad.push(format!("er kept edge {j} changed d{di}"));
                        }
                    }
                    for (i, n) in fr.scene.nodes.iter().enumerate() {
                        if (n.x, n.y) != m[i] { bad.push(format!("er node {i} not at centre d{di}")); }
                        let (ox, oy) = fr.scene.origin;
                        if n.x - n.w / 2.0 < ox - 1e-9 || n.y - n.h / 2.0 < oy - 1e-9 || n.x + n.w / 2.0 > ox + fr.scene.width + 1e-9 || n.y + n.h / 2.0 > oy + fr.scene.height + 1e-9 {
                            bad.push(format!("er node {i} off canvas d{di}"));
                        }
                    }
                }
                let ch = er::route_partial(d, &m, &prev.0, &prev.1);
                let _ = write!(o, "{}|", fnv(&format!("{ch:?}")));
                prev = (ch, m.clone());
                if er::try_route_partial(d, &m, &base, &c).map(|x| format!("{x:?}")) != Some(a.clone()) { bad.push("er try != partial".into()); }
            }
            // fallback: wrong base counts / wrong base positions
            let m = moved(&c, k, (37.0, 23.0));
            let full = format!("{:?}", er::route(d, &m));
            let bc_bad: Vec<(f64, f64)> = c.iter().map(|p| (p.0 + 1.0, p.1)).collect();
            if format!("{:?}", er::route_partial(d, &m, &base, &bc_bad)) != full { bad.push("er misaligned base != route".into()); }
            if format!("{:?}", er::route_partial(d, &m, &base, &c[..c.len() - 1])) != full { bad.push("er short base != route".into()); }
            if er::try_route_partial(d, &m[..m.len() - 1], &base, &c).is_some() { bad.push("er try short not None".into()); }
            let mut nanm = m.clone(); nanm[0].0 = f64::NAN;
            if er::try_route_partial(d, &nanm, &base, &c).is_some() { bad.push("er try nan not None".into()); }
            // edit: one more entity
            let src2 = format!("{src}\n  ZZZVERIFYNEW {{\n    int id\n  }}\n");
            if let Ok(Document::Er(d2)) = parser::parse_document(&src2) {
                let mut m2 = m.clone(); m2.push((c[0].0 + 13.0, c[0].1 + 900.0));
                if m2.len() == d2.entities.len() && format!("{:?}", er::route_partial(&d2, &m2, &base, &c)) != format!("{:?}", er::route(&d2, &m2)) { bad.push("er edit != route".into()); }
            }
            if mode == "time" {
                let fr = er::route_partial(d, &moved(&c, k, (37.0, 23.0)), &base, &c);
                let t = Instant::now(); let n = 5;
                for _ in 0..n { std::hint::black_box(er::route(d, &moved(&c, k, (37.0, 23.0)))); }
                let full = t.elapsed().as_secs_f64() * 1e3 / n as f64;
                let t = Instant::now();
                for _ in 0..n { std::hint::black_box(er::route_partial(d, &moved(&c, k, (43.0, 26.0)), &base, &c)); }
                let part = t.elapsed().as_secs_f64() * 1e3 / n as f64;
                let t = Instant::now();
                for _ in 0..n { std::hint::black_box(er::route_partial(d, &moved(&c, k, (49.0, 29.0)), &fr, &moved(&c, k, (37.0, 23.0)))); }
                let chain = t.elapsed().as_secs_f64() * 1e3 / n as f64;
                timing = Some(format!("route_ms={full:.3} partial_ms={part:.3} chained_ms={chain:.3}"));
            }
        }
        Document::Class(d) => {
            ty = "class";
            let base = class::scene(d);
            size = base.scene.nodes.len() + base.scene.edges.len();
            let all: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            let nc = d.classes.len();
            if nc == 0 { return; }
            let base_cards = class::card_spots(&base);
            for (tag, c) in [("all", all.clone()), ("cls", all[..nc].to_vec())] {
                let k = nc / 2;
                let id = base.scene.nodes[k].id.clone();
                let mut prev = (base.clone(), c.clone());
                for (di, dd) in DRAGS.iter().enumerate() {
                    let m = moved(&c, k, *dd);
                    let fr = class::route_partial(d, &m, &base, &c);
                    let a = format!("{fr:?}");
                    if a != format!("{:?}", class::route_partial(d, &m, &base, &c)) { bad.push(format!("class nondet {tag} d{di}")); }
                    let _ = write!(o, "{}|{}|{}|", fnv(&a), fnv(&class::to_svg(&fr)), fnv(&format!("{:?}", class::card_spots(&fr))));
                    // kept edges: neither end on k. With "cls", notes are placed by fallback and may move too.
                    if tag == "all" {
                        let cards = class::card_spots(&fr);
                        for (j, (e, b)) in fr.scene.edges.iter().zip(&base.scene.edges).enumerate() {
                            if e.from != id && e.to != id {
                                if format!("{e:?}") != format!("{b:?}") { bad.push(format!("class kept edge {j} changed {tag} d{di}")); }
                                if j < cards.len() && cards[j] != base_cards[j] { bad.push(format!("class kept cards {j} changed {tag} d{di}")); }
                            }
                        }
                    }
                    for (i, n) in fr.scene.nodes.iter().enumerate().take(m.len()) {
                        if (n.x, n.y) != m[i] { bad.push(format!("class node {i} not at centre {tag} d{di}")); }
                    }
                    let (ox, oy) = fr.scene.origin;
                    for (i, n) in fr.scene.nodes.iter().enumerate() {
                        if n.x - n.w / 2.0 < ox - 1e-9 || n.y - n.h / 2.0 < oy - 1e-9 || n.x + n.w / 2.0 > ox + fr.scene.width + 1e-9 || n.y + n.h / 2.0 > oy + fr.scene.height + 1e-9 {
                            bad.push(format!("class node {i} off canvas {tag} d{di}"));
                        }
                    }
                    let ch = class::route_partial(d, &m, &prev.0, &prev.1);
                    let _ = write!(o, "{}|{}|", fnv(&format!("{ch:?}")), fnv(&class::to_svg(&ch)));
                    prev = (ch, m.clone());
                    if class::try_route_partial(d, &m, &base, &c).map(|x| format!("{x:?}")) != Some(a.clone()) { bad.push("class try != partial".into()); }
                }
                let m = moved(&c, k, (37.0, 23.0));
                let full = format!("{:?}", class::route(d, &m));
                let bc_bad: Vec<(f64, f64)> = c.iter().map(|p| (p.0 + 1.0, p.1)).collect();
                if format!("{:?}", class::route_partial(d, &m, &base, &bc_bad)) != full { bad.push(format!("class misaligned base != route {tag}")); }
                if class::try_route_partial(d, &m[..m.len() - 1], &base, &c).is_some() && m.len() - 1 != nc && m.len() - 1 != all.len() { bad.push("class try short not None".into()); }
                let src2 = format!("{src}\n  class ZZZVERIFYNEW\n");
                if let Ok(Document::Class(d2)) = parser::parse_document(&src2) {
                    if d2.classes.len() == nc + 1 {
                        // new class goes after existing classes; notes after classes
                        let mut m2 = m[..nc].to_vec(); m2.push((c[0].0 + 13.0, c[0].1 + 900.0));
                        if tag == "all" { m2.extend_from_slice(&m[nc..]); }
                        if format!("{:?}", class::route_partial(&d2, &m2, &base, &c)) != format!("{:?}", class::route(&d2, &m2)) { bad.push(format!("class edit != route {tag}")); }
                    }
                }
                if mode == "time" && tag == "all" {
                    let n = 5;
                    let t = Instant::now();
                    for _ in 0..n { std::hint::black_box(class::route(d, &moved(&c, k, (37.0, 23.0)))); }
                    let full = t.elapsed().as_secs_f64() * 1e3 / n as f64;
                    let t = Instant::now();
                    for _ in 0..n { std::hint::black_box(class::route_partial(d, &moved(&c, k, (43.0, 26.0)), &base, &c)); }
                    let part = t.elapsed().as_secs_f64() * 1e3 / n as f64;
                    timing = Some(format!("route_ms={full:.3} partial_ms={part:.3}"));
                }
            }
        }
        Document::Mindmap(d) => {
            ty = "mindmap";
            let base = mindmap::scene(d);
            size = base.nodes.len() + base.edges.len();
            let c: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
            if c.is_empty() { return; }
            let k = c.len() / 2;
            let ends = mindmap::link_ends(d);
            if ends.len() != base.edges.len() { bad.push("mm link_ends len".into()); }
            let mut prev = (base.clone(), c.clone());
            for (di, dd) in DRAGS.iter().enumerate() {
                let m = moved(&c, k, *dd);
                let fr = mindmap::route_partial(d, &m, &base, &c);
                let a = format!("{fr:?}");
                let _ = write!(o, "{}|{}|", fnv(&a), fnv(&mindmap::to_svg(&fr)));
                for (j, (e, b)) in fr.edges.iter().zip(&base.edges).enumerate() {
                    let (p, q) = ends[j];
                    if p != k && q != k && format!("{e:?}") != format!("{b:?}") { bad.push(format!("mm kept edge {j} changed d{di}")); }
                }
                for (i, (n, b)) in fr.nodes.iter().zip(&base.nodes).enumerate() {
                    if i != k && format!("{n:?}") != format!("{b:?}") { bad.push(format!("mm kept node {i} changed d{di}")); }
                }
                // equals route up to float rounding: compare SVG
                let full = mindmap::route(d, &m);
                if mindmap::to_svg(&full) != mindmap::to_svg(&fr) { bad.push(format!("mm svg != route svg d{di}")); }
                let ch = mindmap::route_partial(d, &m, &prev.0, &prev.1);
                let _ = write!(o, "{}|", fnv(&format!("{ch:?}")));
                prev = (ch, m.clone());
            }
            let m = moved(&c, k, (37.0, 23.0));
            let src2 = format!("{src}\n");
            let _ = src2;
            if mindmap::try_route_partial(d, &m[..m.len() - 1], &base, &c).is_some() { bad.push("mm try short not None".into()); }
            let ms = base.clone();
            let _ = write!(o, "{}|", fnv(&pick_dump(ms.width, ms.height, |x, y, t| ms.hit_test(x, y, t))));
            if mode == "time" { let t = pick_time(ms.width, ms.height, |x, y, t| ms.hit_test(x, y, t)); timing = Some(format!("pick_mean_us={:.2} pick_max_us={:.1} hits={}", t.0, t.1, t.2)); }
            if mode == "time" {
                let n = 20;
                let t = Instant::now();
                for _ in 0..n { std::hint::black_box(mindmap::route(d, &moved(&c, k, (37.0, 23.0)))); }
                let full = t.elapsed().as_secs_f64() * 1e3 / n as f64;
                let t = Instant::now();
                for _ in 0..n { std::hint::black_box(mindmap::route_partial(d, &moved(&c, k, (43.0, 26.0)), &base, &c)); }
                let part = t.elapsed().as_secs_f64() * 1e3 / n as f64;
                timing = Some(format!("{} route_ms={full:.3} partial_ms={part:.3}", timing.unwrap_or_default()));
            }
        }
        Document::Sequence(d) => {
            ty = "seq";
            let s = flowmaid::seq::scene(d);
            size = s.boxes.len() + s.messages.len() + s.notes.len() + s.frames.len() + s.activations.len();
            let _ = write!(o, "{}|", fnv(&pick_dump(s.width, s.height, |x, y, t| s.hit_test(x, y, t))));
            if mode == "time" { let t = pick_time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)); timing = Some(format!("pick_mean_us={:.2} pick_max_us={:.1} hits={}", t.0, t.1, t.2)); }
        }
        Document::Pie(d) => {
            ty = "pie";
            let s = flowmaid::pie::scene(d);
            size = s.slices.len();
            let _ = write!(o, "{}|", fnv(&pick_dump(s.width, s.height, |x, y, t| s.hit_test(x, y, t))));
            if mode == "time" { let t = pick_time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)); timing = Some(format!("pick_mean_us={:.2} pick_max_us={:.1} hits={}", t.0, t.1, t.2)); }
        }
        Document::Journey(d) => {
            ty = "journey";
            let s = flowmaid::journey::scene(d);
            size = s.tasks.len() + s.sections.len();
            let _ = write!(o, "{}|", fnv(&pick_dump(s.width, s.height, |x, y, t| s.hit_test(x, y, t))));
            if mode == "time" { let t = pick_time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)); timing = Some(format!("pick_mean_us={:.2} pick_max_us={:.1} hits={}", t.0, t.1, t.2)); }
        }
        Document::Timeline(d) => {
            ty = "timeline";
            let s = flowmaid::timeline::scene(d);
            size = s.sections.iter().map(|x| 1 + x.periods.iter().map(|p| 1 + p.events.len()).sum::<usize>()).sum();
            let _ = write!(o, "{}|", fnv(&pick_dump(s.width, s.height, |x, y, t| s.hit_test(x, y, t))));
            if mode == "time" { let t = pick_time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)); timing = Some(format!("pick_mean_us={:.2} pick_max_us={:.1} hits={}", t.0, t.1, t.2)); }
        }
        Document::Sankey(d) => {
            ty = "sankey";
            let s = flowmaid::sankey::scene(d);
            size = s.nodes.len() + s.links.len();
            let _ = write!(o, "{}|", fnv(&pick_dump(s.width, s.height, |x, y, t| s.hit_test(x, y, t))));
            if mode == "time" { let t = pick_time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)); timing = Some(format!("pick_mean_us={:.2} pick_max_us={:.1} hits={}", t.0, t.1, t.2)); }
        }
        _ => {}
    }
    if ty.is_empty() { return; }
    if mode == "time" {
        println!("{ty}\t{size}\t{}\t{f}", timing.unwrap_or_default());
    } else {
        bad.sort(); bad.dedup();
        println!("{ty}\t{size}\t{}\t{}\t{f}", fnv(&o), bad.join(";"));
    }
}
