#[path = "../../psrc/common.rs"]
mod common;
use flowmaid::model::{ClassDiagram, Document, ErDiagram, Graph, Mindmap};
use flowmaid::{class, er, mindmap, parser, scene};
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn dbg<T: std::fmt::Debug>(t: &T) -> String {
    format!("{t:?}")
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// (laid centres, accepted counts) and a closure kind
fn laid(d: &Document) -> Option<(Vec<(f64, f64)>, Vec<usize>)> {
    Some(match d {
        Document::Flowchart(g) | Document::State(g) => {
            (scene::scene(g).nodes.iter().map(|n| (n.x, n.y)).collect(), vec![g.nodes.len()])
        }
        Document::Class(c) => {
            let n = c.classes.len();
            let mut v = vec![n, n + c.notes.len()];
            v.dedup();
            (class::scene(c).scene.nodes.iter().map(|k| (k.x, k.y)).collect(), v)
        }
        Document::Er(e) => (er::scene(e).scene.nodes.iter().map(|k| (k.x, k.y)).collect(), vec![e.entities.len()]),
        Document::Mindmap(m) => (mindmap::scene(m).nodes.iter().map(|k| (k.cx(), k.cy())).collect(), vec![m.nodes.len()]),
        _ => return None,
    })
}

/// try_route result: None, or Some(equal to route?)
fn tr(d: &Document, p: &[(f64, f64)]) -> Option<bool> {
    match d {
        Document::Flowchart(g) | Document::State(g) => scene::try_route(g, p).map(|s| dbg(&s) == dbg(&scene::route(g, p))),
        Document::Class(c) => class::try_route(c, p).map(|s| dbg(&s) == dbg(&class::route(c, p))),
        Document::Er(e) => er::try_route(e, p).map(|s| dbg(&s) == dbg(&er::route(e, p))),
        Document::Mindmap(m) => mindmap::try_route(m, p).map(|s| dbg(&s) == dbg(&mindmap::route(m, p))),
        _ => None,
    }
}
/// try_route only (no comparison), returns is_some
fn tr_only(d: &Document, p: &[(f64, f64)]) -> bool {
    match d {
        Document::Flowchart(g) | Document::State(g) => scene::try_route(g, p).map(|s| scene::to_svg(&s).len()).is_some(),
        Document::Class(c) => class::try_route(c, p).map(|s| class::to_svg(&s).len()).is_some(),
        Document::Er(e) => er::try_route(e, p).map(|s| er::to_svg(&s).len()).is_some(),
        Document::Mindmap(m) => mindmap::try_route(m, p).map(|s| mindmap::to_svg(&s).len()).is_some(),
        _ => false,
    }
}

fn positions(l: &[(f64, f64)], k: usize) -> Vec<(f64, f64)> {
    (0..k).map(|i| l.get(i).copied().unwrap_or((50.0 + 25.0 * i as f64, 30.0 + 15.0 * i as f64))).collect()
}

fn fuzz(name: &str, d: &Document, stats: &mut [usize; 6]) {
    let Some((l, counts)) = laid(d) else { return };
    let n = l.len();
    let mut fail = |what: String, stats: &mut [usize; 6]| {
        stats[5] += 1;
        println!("FAIL {name}: {what}");
    };
    // counts 0..n+6
    for k in 0..n + 7 {
        let p = positions(&l, k);
        match catch_unwind(AssertUnwindSafe(|| tr(d, &p))) {
            Err(_) => fail(format!("panic at count {k}"), stats),
            Ok(r) => {
                stats[0] += 1;
                if counts.contains(&k) {
                    if r != Some(true) {
                        fail(format!("count {k} accepted but got {r:?}"), stats);
                    }
                } else if r.is_some() {
                    fail(format!("count {k} not accepted but got {r:?}"), stats);
                }
            }
        }
    }
    let mut rng = Rng(0x9e3779b97f4a7c15 ^ (n as u64 + 1) * 0x1234567);
    for &k in &counts {
        if k == 0 {
            continue;
        }
        // non-finite
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for idx in [0, k / 2, k - 1] {
                for axis in 0..2 {
                    let mut p = positions(&l, k);
                    if axis == 0 { p[idx].0 = bad } else { p[idx].1 = bad }
                    match catch_unwind(AssertUnwindSafe(|| tr(d, &p))) {
                        Err(_) => fail(format!("panic nonfinite {bad} idx {idx} axis {axis}"), stats),
                        Ok(r) => {
                            stats[1] += 1;
                            if r.is_some() {
                                fail(format!("nonfinite gave scene"), stats);
                            }
                        }
                    }
                }
            }
        }
        // random/odd positions within reach
        let mut sets: Vec<Vec<(f64, f64)>> = vec![
            vec![(0.0, 0.0); k],
            vec![(-1e6, -1e6); k],
            vec![(1e6, 1e6); k],
            vec![(-0.0, 5e-324); k],
        ];
        for s in 0..6 {
            let scale = [1.0, 50.0, 1000.0, 1e4, 1e5, 1e6][s];
            sets.push((0..k).map(|_| ((rng.next() * 2.0 - 1.0) * scale, (rng.next() * 2.0 - 1.0) * scale)).collect());
        }
        // pairs coincide
        sets.push((0..k).map(|i| if i % 2 == 0 { (100.0, 100.0) } else { (100.0, 100.0 + 1e-9) }).collect());
        let mut p = positions(&l, k);
        p.reverse();
        sets.push(p);
        for p in &sets {
            match catch_unwind(AssertUnwindSafe(|| (tr(d, p), tr_only(d, p)))) {
                Err(_) => fail(format!("panic within reach {:?}", &p[..p.len().min(3)]), stats),
                Ok((r, _)) => {
                    stats[2] += 1;
                    if r != Some(true) {
                        fail(format!("within reach gave {r:?}"), stats);
                    }
                }
            }
        }
        // beyond reach
        for past in [1e6 + 1e-6, 1.000001e6, -2e6, 1e15, f64::MAX, -1e300] {
            let mut p = positions(&l, k);
            p[k - 1].0 = past;
            match catch_unwind(AssertUnwindSafe(|| tr(d, &p))) {
                Err(_) => fail(format!("panic past reach {past}"), stats),
                Ok(r) => {
                    stats[3] += 1;
                    if past.abs() > 1e6 && r.is_some() {
                        fail(format!("past reach {past} gave scene"), stats);
                    }
                }
            }
        }
    }
    stats[4] += 1;
}

fn origin_check(list: &str) {
    // per routed scene: name variant origin(x,y) width height; flowchart: drawn node check
    for f in fs::read_to_string(list).unwrap().lines().filter(|l| !l.is_empty()) {
        let Ok(src) = fs::read_to_string(f) else { continue };
        let n = common::name_of(f);
        let Ok(doc) = parser::parse_document(&src) else { continue };
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut rows = Vec::new();
            match &doc {
                Document::Flowchart(g) | Document::State(g) => {
                    if g.nodes.is_empty() { return rows; }
                    let laid = scene::scene(g);
                    rows.push(format!("{n}\tscene\t{:?}\t{}", laid.origin, common::view_box(&scene::to_svg(&laid))));
                    let c: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.x, k.y)).collect();
                    let half: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
                    let mut all = vec![("route0".to_string(), scene::route(g, &c), c.clone())];
                    for (lbl, d) in common::drags(&c, &half) {
                        all.push((format!("route-{lbl}"), scene::route(g, &d), d.clone()));
                        let p = scene::route_partial(g, &d, &laid, &c);
                        if lbl == "over" || lbl == "far" {
                            let p2 = scene::route_partial(g, &c, &p, &d);
                            all.push((format!("chain-{lbl}"), p2, c.clone()));
                        }
                        all.push((format!("part-{lbl}"), p, d.clone()));
                    }
                    for (v, sc, d) in all {
                        let moved = sc.nodes.iter().zip(&d).filter(|(k, p)| (k.x, k.y) != **p).count();
                        let svg = scene::to_svg(&sc);
                        let vb = common::view_box(&svg);
                        // drawn node check via title + next line
                        let lines: Vec<&str> = svg.lines().collect();
                        let (mut ok, mut off, mut miss) = (0, 0, 0);
                        for (i, k) in sc.nodes.iter().enumerate() {
                            if k.label.is_empty() || k.label.contains('\n') || sc.nodes.iter().filter(|o| o.label == k.label).count() != 1 { continue; }
                            let esc = k.label.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;");
                            let t = format!("<g><title>{esc}</title>");
                            let Some(at) = lines.iter().position(|l| *l == t) else { continue };
                            let Some(cen) = lines.get(at + 1).and_then(|l| centre(l)) else { continue };
                            if (cen.0 - k.x).abs() <= 0.1 && (cen.1 - k.y).abs() <= 0.1 {
                                ok += 1;
                                if sc.node_at(cen.0, cen.1) != Some(i) { miss += 1; }
                            } else { off += 1; }
                        }
                        rows.push(format!("{n}\t{v}\t{:?}\t{vb}\tw={:.3} h={:.3}\tmoved={moved}\tdrawn_at_scene={ok} off={off} pickmiss={miss}", sc.origin, sc.width, sc.height));
                    }
                }
                Document::Class(d) => {
                    let laid = class::scene(d);
                    let c: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.x, k.y)).collect();
                    if c.is_empty() { return rows; }
                    let half: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
                    rows.push(format!("{n}\tscene\t{:?}\t{}", laid.scene.origin, common::view_box(&class::to_svg(&laid))));
                    let mut all = vec![("route0".to_string(), c.clone())];
                    for (lbl, dd) in common::drags(&c, &half) { all.push((format!("route-{lbl}"), dd)); }
                    for (v, dd) in all {
                        let cs = class::route(d, &dd);
                        let moved = cs.scene.nodes.iter().zip(&dd).filter(|(k, p)| (k.x, k.y) != **p).count();
                        rows.push(format!("{n}\t{v}\t{:?}\t{}\tw={:.3} h={:.3}\tmoved={moved}", cs.scene.origin, common::view_box(&class::to_svg(&cs)), cs.scene.width, cs.scene.height));
                    }
                }
                Document::Er(d) => {
                    let laid = er::scene(d);
                    let c: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.x, k.y)).collect();
                    if c.is_empty() { return rows; }
                    let half: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
                    rows.push(format!("{n}\tscene\t{:?}\t{}", laid.scene.origin, common::view_box(&er::to_svg(&laid))));
                    let mut all = vec![("route0".to_string(), c.clone())];
                    for (lbl, dd) in common::drags(&c, &half) { all.push((format!("route-{lbl}"), dd)); }
                    for (v, dd) in all {
                        let es = er::route(d, &dd);
                        let moved = es.scene.nodes.iter().zip(&dd).filter(|(k, p)| (k.x, k.y) != **p).count();
                        rows.push(format!("{n}\t{v}\t{:?}\t{}\tw={:.3} h={:.3}\tmoved={moved}", es.scene.origin, common::view_box(&er::to_svg(&es)), es.scene.width, es.scene.height));
                    }
                }
                _ => {}
            }
            rows
        }));
        match r {
            Ok(rows) => for r in rows { println!("{r}") },
            Err(_) => println!("{n}\tPANIC"),
        }
    }
}

fn centre(line: &str) -> Option<(f64, f64)> {
    let num = |k: &str| -> Option<f64> {
        let pat = format!(" {k}=\"");
        let i = line.find(&pat)? + pat.len();
        line[i..i + line[i..].find('"')?].parse().ok()
    };
    if line.starts_with("<rect ") {
        Some((num("x")? + num("width")? / 2.0, num("y")? + num("height")? / 2.0))
    } else if line.starts_with("<circle ") || line.starts_with("<ellipse ") {
        Some((num("cx")?, num("cy")?))
    } else if line.starts_with("<polygon ") {
        let i = line.find(" points=\"")? + 9;
        let pts: Vec<(f64, f64)> = line[i..i + line[i..].find('"')?].split(' ').filter_map(|p| {
            let (x, y) = p.split_once(',')?;
            Some((x.parse().ok()?, y.parse().ok()?))
        }).collect();
        (pts.len() == 4 && pts[0].0 == pts[2].0 && pts[1].1 == pts[3].1).then(|| (pts[0].0, pts[1].1))
    } else {
        None
    }
}

fn far(kind: &str, v: f64, idx: Option<usize>, all: bool) {
    let src = match kind {
        "flow" => include_str!("../../head/examples/demo.mmd"),
        "sub" => include_str!("../../head/examples/subgraph.mmd"),
        "state" => include_str!("../../head/examples/state.mmd"),
        "class" => include_str!("../../head/examples/class.mmd"),
        "er" => include_str!("../../head/examples/er.mmd"),
        _ => include_str!("../../head/examples/mindmap.mmd"),
    };
    let d = parser::parse_document(src).unwrap();
    let (l, counts) = laid(&d).unwrap();
    let k = counts[0];
    let mut p = positions(&l, k);
    if all { for q in p.iter_mut() { *q = (v, -v); } } else { p[idx.unwrap_or(k - 1)].0 = v; }
    let t = std::time::Instant::now();
    let len = match &d {
        Document::Flowchart(g) | Document::State(g) => scene::to_svg(&scene::route(g, &p)).len(),
        Document::Class(c) => class::to_svg(&class::route(c, &p)).len(),
        Document::Er(e) => er::to_svg(&er::route(e, &p)).len(),
        Document::Mindmap(m) => mindmap::to_svg(&mindmap::route(m, &p)).len(),
        _ => 0,
    };
    println!("{kind} {v:e}: ok svg {len} bytes in {:?}", t.elapsed());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args[1] == "far" {
        // keep the default hook so a panic message shows
        let idx = args.get(4).and_then(|a| a.parse().ok());
        let all = args.get(4).map_or(false, |a| a == "all");
        far(&args[2], args[3].parse().unwrap(), idx, all);
        return;
    }
    std::panic::set_hook(Box::new(|_| {}));
    if common::run_common(&args) {
        return;
    }
    match args[1].as_str() {
        "fuzz" => {
            let mut stats = [0usize; 6];
            let mut docs: Vec<(String, Document)> = Vec::new();
            for f in fs::read_to_string(&args[2]).unwrap().lines().filter(|l| !l.is_empty()) {
                if let Ok(s) = fs::read_to_string(f) {
                    if let Ok(d) = parser::parse_document(&s) {
                        docs.push((common::name_of(f), d));
                    }
                }
            }
            for s in ["flowchart TD", "stateDiagram-v2", "classDiagram", "erDiagram", "classDiagram\nnote \"x\"", "flowchart LR\nA", "classDiagram\nclass A\nnote for A \"n\"\nnote \"m\"", "mindmap\n  r"] {
                docs.push((format!("{s:?}"), parser::parse_document(s).unwrap()));
            }
            docs.push(("Graph::default".into(), Document::Flowchart(Graph::default())));
            docs.push(("State Graph::default".into(), Document::State(Graph::default())));
            docs.push(("ClassDiagram::default".into(), Document::Class(ClassDiagram::default())));
            docs.push(("ErDiagram::default".into(), Document::Er(ErDiagram::default())));
            docs.push(("Mindmap::default".into(), Document::Mindmap(Mindmap::default())));
            for (n, d) in &docs {
                fuzz(n, d, &mut stats);
            }
            println!("docs {} counts {} nonfinite {} within {} past {} fails {}", stats[4], stats[0], stats[1], stats[2], stats[3], stats[5]);
        }
        "origin" => origin_check(&args[2]),
        _ => eprintln!("unknown"),
    }
}
