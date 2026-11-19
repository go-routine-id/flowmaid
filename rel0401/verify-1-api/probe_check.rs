// Head-only checks of the fallible try_route variants.
use flowmaid::model::{ClassDiagram, Document, ErDiagram, Graph, Mindmap};
use flowmaid::{class, er, mindmap, parser, scene};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn pad(base: &[(f64, f64)], k: usize) -> Vec<(f64, f64)> {
    (0..k)
        .map(|i| {
            base.get(i)
                .copied()
                .unwrap_or((37.0 * i as f64 + 5.0, 23.0 * i as f64 - 11.0))
        })
        .collect()
}

struct Tally {
    calls: usize,
    some: usize,
    eq: usize,
    bad: Vec<String>,
}

fn guarded<T>(t: &mut Tally, tag: &str, f: impl FnOnce() -> T) -> Option<T> {
    t.calls += 1;
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => Some(v),
        Err(_) => {
            t.bad.push(format!("PANIC {tag}"));
            None
        }
    }
}

fn nonfinite(base: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let mut v = Vec::new();
    if base.is_empty() {
        return v;
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for which in [0usize, base.len() - 1] {
            for axis in 0..2 {
                let mut p = base.to_vec();
                if axis == 0 {
                    p[which].0 = bad;
                } else {
                    p[which].1 = bad;
                }
                v.push(p);
            }
        }
    }
    v
}

fn flow(t: &mut Tally, tag: &str, g: &Graph, extra: usize) {
    let sc = scene::scene(g);
    let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = g.nodes.len();
    for k in 0..=n + extra {
        let p = pad(&c, k);
        let tag = format!("{tag} flow k={k}/{n}");
        let r = guarded(t, &tag, || scene::try_route(g, &p).map(|s| scene::to_svg(&s)));
        let Some(r) = r else { continue };
        match (k == n, r) {
            (true, Some(svg)) => {
                t.some += 1;
                let want = scene::to_svg(&scene::route(g, &p));
                if want == svg {
                    t.eq += 1
                } else {
                    t.bad.push(format!("NEQ {tag}"))
                }
            }
            (false, None) => {}
            (acc, got) => t.bad.push(format!("WRONG {tag}: accepted={acc} got_some={}", got.is_some())),
        }
    }
    for p in nonfinite(&c) {
        let tag = format!("{tag} flow nonfinite");
        if let Some(Some(_)) = guarded(t, &tag, || scene::try_route(g, &p).map(|_| ())) {
            t.bad.push(format!("SOME {tag}"));
        }
    }
}

fn cls(t: &mut Tally, tag: &str, d: &ClassDiagram, extra: usize) {
    let cs = class::scene(d);
    let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let (n, m) = (d.classes.len(), d.notes.len());
    for k in 0..=n + m + extra {
        let p = pad(&c, k);
        let tag = format!("{tag} class k={k}/{n}+{m}");
        let r = guarded(t, &tag, || class::try_route(d, &p).map(|s| class::to_svg(&s)));
        let Some(r) = r else { continue };
        let acc = k == n || k == n + m;
        match (acc, r) {
            (true, Some(svg)) => {
                t.some += 1;
                let want = class::to_svg(&class::route(d, &p));
                if want == svg {
                    t.eq += 1
                } else {
                    t.bad.push(format!("NEQ {tag}"))
                }
            }
            (false, None) => {}
            (acc, got) => t.bad.push(format!("WRONG {tag}: accepted={acc} got_some={}", got.is_some())),
        }
    }
    for p in nonfinite(&c).into_iter().chain(nonfinite(&c[..n.min(c.len())])) {
        let tag = format!("{tag} class nonfinite");
        if let Some(Some(_)) = guarded(t, &tag, || class::try_route(d, &p).map(|_| ())) {
            t.bad.push(format!("SOME {tag}"));
        }
    }
}

fn erd(t: &mut Tally, tag: &str, d: &ErDiagram, extra: usize) {
    let es = er::scene(d);
    let c: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = d.entities.len();
    for k in 0..=n + extra {
        let p = pad(&c, k);
        let tag = format!("{tag} er k={k}/{n}");
        let r = guarded(t, &tag, || er::try_route(d, &p).map(|s| er::to_svg(&s)));
        let Some(r) = r else { continue };
        match (k == n, r) {
            (true, Some(svg)) => {
                t.some += 1;
                let want = er::to_svg(&er::route(d, &p));
                if want == svg {
                    t.eq += 1
                } else {
                    t.bad.push(format!("NEQ {tag}"))
                }
            }
            (false, None) => {}
            (acc, got) => t.bad.push(format!("WRONG {tag}: accepted={acc} got_some={}", got.is_some())),
        }
    }
    for p in nonfinite(&c) {
        let tag = format!("{tag} er nonfinite");
        if let Some(Some(_)) = guarded(t, &tag, || er::try_route(d, &p).map(|_| ())) {
            t.bad.push(format!("SOME {tag}"));
        }
    }
}

fn mind(t: &mut Tally, tag: &str, d: &Mindmap, extra: usize) {
    let ms = mindmap::scene(d);
    let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
    let n = d.nodes.len();
    for k in 0..=n + extra {
        let p = pad(&c, k);
        let tag = format!("{tag} mind k={k}/{n}");
        let r = guarded(t, &tag, || mindmap::try_route(d, &p).map(|s| mindmap::to_svg(&s)));
        let Some(r) = r else { continue };
        match (k == n, r) {
            (true, Some(svg)) => {
                t.some += 1;
                let want = mindmap::to_svg(&mindmap::route(d, &p));
                if want == svg {
                    t.eq += 1
                } else {
                    t.bad.push(format!("NEQ {tag}"))
                }
            }
            (false, None) => {}
            (acc, got) => t.bad.push(format!("WRONG {tag}: accepted={acc} got_some={}", got.is_some())),
        }
    }
    for p in nonfinite(&c) {
        let tag = format!("{tag} mind nonfinite");
        if let Some(Some(_)) = guarded(t, &tag, || mindmap::try_route(d, &p).map(|_| ())) {
            t.bad.push(format!("SOME {tag}"));
        }
    }
}

fn huge_pos(c: &[(f64, f64)], mode: u8) -> Vec<(f64, f64)> {
    c.iter()
        .enumerate()
        .map(|(i, p)| match mode {
            0 => (1e300 * (i as f64 + 1.0), -1e300),
            1 => (f64::MAX, f64::MIN),
            2 => (1e15 * i as f64, 1e15),
            3 => (0.0, 0.0),
            4 => (p.0 * 1e-300, p.1),
            5 => (-1e9 + i as f64, 1e9),
            6 => if i == 0 { (f64::MAX, p.1) } else { *p },
            7 => (f64::MIN_POSITIVE, -0.0),
            8..=12 => {
                let s = 10f64.powi(mode as i32 - 5);
                (s * i as f64, s * (i % 3) as f64)
            }
            13 => if i == 0 { (-1e6, -1e6) } else { *p },
            14 => if i == 0 { (1e7, 1e7) } else { *p },
            15 => if i == 0 { (-1e9, p.1) } else { *p },
            _ => if i == 0 { (1e12, p.1) } else { *p },
        })
        .collect()
}

pub fn run(list: &str, huge: Option<&str>) {
    std::panic::set_hook(Box::new(|_| {}));
    let mut t = Tally { calls: 0, some: 0, eq: 0, bad: Vec::new() };
    let mut docs: Vec<(String, Document)> = Vec::new();
    for f in std::fs::read_to_string(list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        if let Ok(d) = parser::parse_document(&src) {
            docs.push((f.to_string(), d));
        }
    }
    for (name, src) in [
        ("p-flow", "flowchart TD"),
        ("p-state", "stateDiagram-v2"),
        ("p-class", "classDiagram"),
        ("p-class-note", "classDiagram\nnote \"only a note\""),
        ("p-class-2notes", "classDiagram\nnote \"a\"\nnote \"b\""),
        ("p-class-notefor", "classDiagram\nclass A\nnote for A \"x\"\nnote \"free\""),
        ("p-er", "erDiagram"),
        ("p-mind", "mindmap"),
        ("p-mind1", "mindmap\n  root"),
    ] {
        match parser::parse_document(src) {
            Ok(d) => docs.push((name.into(), d)),
            Err(e) => println!("noparse {name}: {e}"),
        }
    }
    docs.push(("empty-flow".into(), Document::Flowchart(Graph::default())));
    docs.push(("empty-class".into(), Document::Class(ClassDiagram::default())));
    docs.push(("empty-er".into(), Document::Er(ErDiagram::default())));
    docs.push(("empty-mind".into(), Document::Mindmap(Mindmap::default())));
    let mut per = std::collections::BTreeMap::<&str, usize>::new();
    for (f, d) in &docs {
        if let Some(h) = huge {
            // Huge finite coordinates: no panic (route may run).
            let mode: u8 = h.parse().unwrap();
            let tag = format!("{f} huge{mode}");
            match d {
                Document::Flowchart(g) | Document::State(g) => {
                    let c: Vec<_> = scene::scene(g).nodes.iter().map(|n| (n.x, n.y)).collect();
                    let p = huge_pos(&c, mode);
                    guarded(&mut t, &tag, || scene::try_route(g, &p).map(|s| scene::to_svg(&s)));
                }
                Document::Class(d) => {
                    let c: Vec<_> = class::scene(d).scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let p = huge_pos(&c, mode);
                    guarded(&mut t, &tag, || class::try_route(d, &p).map(|s| class::to_svg(&s)));
                    let k = d.classes.len();
                    guarded(&mut t, &tag, || class::try_route(d, &p[..k]).map(|s| class::to_svg(&s)));
                }
                Document::Er(d) => {
                    let c: Vec<_> = er::scene(d).scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let p = huge_pos(&c, mode);
                    guarded(&mut t, &tag, || er::try_route(d, &p).map(|s| er::to_svg(&s)));
                }
                Document::Mindmap(d) => {
                    let c: Vec<_> = mindmap::scene(d).nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                    let p = huge_pos(&c, mode);
                    guarded(&mut t, &tag, || mindmap::try_route(d, &p).map(|s| mindmap::to_svg(&s)));
                }
                _ => {}
            }
            continue;
        }
        match d {
            Document::Flowchart(g) | Document::State(g) => {
                *per.entry("flow/state").or_default() += 1;
                flow(&mut t, f, g, 5)
            }
            Document::Class(d) => {
                *per.entry("class").or_default() += 1;
                cls(&mut t, f, d, 5)
            }
            Document::Er(d) => {
                *per.entry("er").or_default() += 1;
                erd(&mut t, f, d, 5)
            }
            Document::Mindmap(d) => {
                *per.entry("mindmap").or_default() += 1;
                mind(&mut t, f, d, 5)
            }
            _ => {}
        }
    }
    println!(
        "docs={} per={per:?} calls={} accepted={} equal={} bad={}",
        docs.len(),
        t.calls,
        t.some,
        t.eq,
        t.bad.len()
    );
    for b in t.bad.iter().take(60) {
        println!("{b}");
    }
}
