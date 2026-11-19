use flowmaid::model::{ClassDiagram, Document, ErDiagram, Graph, Mindmap};
use flowmaid::{class, er, mindmap, parser, scene};
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn pad(c: &[(f64, f64)], k: usize) -> Vec<(f64, f64)> {
    (0..k)
        .map(|i| c.get(i).copied().unwrap_or((40.0 * i as f64, 30.0 * i as f64)))
        .collect()
}

struct Tally {
    files: usize,
    eq: usize,
    none_ok: usize,
    fail: Vec<String>,
}

fn guarded<T>(f: impl FnOnce() -> T) -> Result<T, ()> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| ())
}

/// Every sweep for one diagram, `accepted` the counts route takes.
fn sweep<S: std::fmt::Debug>(
    t: &mut Tally,
    what: &str,
    c: &[(f64, f64)],
    accepted: &[usize],
    maxk: usize,
    route: &dyn Fn(&[(f64, f64)]) -> S,
    try_route: &dyn Fn(&[(f64, f64)]) -> Option<S>,
    svg: &dyn Fn(&S) -> String,
) {
    for k in 0..=maxk {
        let p = pad(c, k);
        match guarded(|| try_route(&p)) {
            Err(()) => t.fail.push(format!("{what}: try_route PANIC at count {k}")),
            Ok(None) => {
                if accepted.contains(&k) {
                    t.fail.push(format!("{what}: None at accepted count {k}"));
                } else {
                    t.none_ok += 1;
                }
            }
            Ok(Some(s)) => {
                if !accepted.contains(&k) {
                    t.fail.push(format!("{what}: Some at rejected count {k}"));
                    continue;
                }
                match guarded(|| route(&p)) {
                    Err(()) => t.fail.push(format!("{what}: route panicked at accepted {k}")),
                    Ok(r) => {
                        if format!("{r:?}") == format!("{s:?}") && svg(&r) == svg(&s) {
                            t.eq += 1;
                        } else {
                            t.fail.push(format!("{what}: try_route != route at {k}"));
                        }
                    }
                }
            }
        }
    }
    // Rejected counts: route must panic for class/ER/flowchart (or give
    // an empty mindmap) -- not checked here; only no-panic of try_route.
    // Non-finite and out-of-reach coordinates at an accepted count.
    if let Some(&k) = accepted.iter().find(|&&k| k > 0) {
        let base = pad(c, k);
        for (bad, label) in [
            (f64::NAN, "nan"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
            (1.0e6 + 1.0, "1e6+1"),
            (-1.0e6 - 1.0, "-1e6-1"),
            (f64::MAX, "max"),
            (f64::MIN_POSITIVE, "minpos"),
            (-0.0, "-0"),
            (1.0e6, "1e6"),
            (-1.0e6, "-1e6"),
        ] {
            for idx in [0, k - 1] {
                for axis in 0..2 {
                    let mut p = base.clone();
                    if axis == 0 {
                        p[idx].0 = bad
                    } else {
                        p[idx].1 = bad
                    }
                    let want_some = bad.is_finite() && bad.abs() <= 1.0e6;
                    match guarded(|| try_route(&p)) {
                        Err(()) => t.fail.push(format!("{what}: PANIC {label} idx {idx} axis {axis}")),
                        Ok(None) if want_some => {
                            t.fail.push(format!("{what}: None for in-reach {label}"))
                        }
                        Ok(Some(_)) if !want_some => {
                            t.fail.push(format!("{what}: Some for {label}"))
                        }
                        Ok(_) => t.none_ok += 1,
                    }
                }
            }
        }
        // every centre in one corner of the reach
        for corner in [(1.0e6, 1.0e6), (-1.0e6, -1.0e6), (0.0, 0.0)] {
            let p = vec![corner; k];
            if guarded(|| try_route(&p)).is_err() {
                t.fail.push(format!("{what}: PANIC all at {corner:?}"));
            }
        }
    }
}

fn check_graph(t: &mut Tally, what: &str, g: &Graph) {
    let sc = scene::scene(g);
    let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = g.nodes.len();
    sweep(
        t,
        what,
        &c,
        &[n],
        n + 5,
        &|p| scene::route(g, p),
        &|p| scene::try_route(g, p),
        &|s| scene::to_svg(s),
    );
}

fn check_class(t: &mut Tally, what: &str, d: &ClassDiagram) {
    let cs = class::scene(d);
    let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = d.classes.len();
    let m = d.notes.len();
    sweep(
        t,
        what,
        &c,
        &[n, n + m],
        n + m + 5,
        &|p| class::route(d, p),
        &|p| class::try_route(d, p),
        &|s| class::to_svg(s),
    );
}

fn check_er(t: &mut Tally, what: &str, d: &ErDiagram) {
    let es = er::scene(d);
    let c: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = d.entities.len();
    sweep(
        t,
        what,
        &c,
        &[n],
        n + 5,
        &|p| er::route(d, p),
        &|p| er::try_route(d, p),
        &|s| er::to_svg(s),
    );
}

fn check_mind(t: &mut Tally, what: &str, d: &Mindmap) {
    let ms = mindmap::scene(d);
    let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
    let n = d.nodes.len();
    sweep(
        t,
        what,
        &c,
        &[n],
        n + 5,
        &|p| mindmap::route(d, p),
        &|p| mindmap::try_route(d, p),
        &|s| mindmap::to_svg(s),
    );
}

pub fn run(list: &str) {
    let mut t = Tally {
        files: 0,
        eq: 0,
        none_ok: 0,
        fail: Vec::new(),
    };
    // empty diagrams
    check_graph(&mut t, "<Graph::default>", &Graph::default());
    check_class(&mut t, "<ClassDiagram::default>", &ClassDiagram::default());
    check_er(&mut t, "<ErDiagram::default>", &ErDiagram::default());
    check_mind(&mut t, "<Mindmap::default>", &Mindmap::default());
    // mindmap: empty map + no positions gives Some(empty), mismatch None
    let e = Mindmap::default();
    if mindmap::try_route(&e, &[]).is_none() {
        t.fail.push("empty mindmap, no positions: None".into());
    }
    if mindmap::try_route(&e, &[(1.0, 1.0)]).is_some() {
        t.fail.push("empty mindmap, one position: Some".into());
    }
    for (src, what) in [
        ("flowchart TD", "<flowchart TD>"),
        ("graph LR\n", "<graph LR>"),
        ("stateDiagram-v2\n", "<state empty>"),
        ("classDiagram\n", "<class empty>"),
        ("erDiagram\n", "<er empty>"),
        ("mindmap\n", "<mindmap empty>"),
        ("classDiagram\nnote \"free\"", "<class free note only>"),
        ("classDiagram\nclass A\nnote for A \"n\"\nnote \"m\"", "<class 1+2 notes>"),
    ] {
        match parser::parse_document(src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => check_graph(&mut t, what, &g),
            Ok(Document::Class(d)) => check_class(&mut t, what, &d),
            Ok(Document::Er(d)) => check_er(&mut t, what, &d),
            Ok(Document::Mindmap(d)) => check_mind(&mut t, what, &d),
            Ok(_) => println!("{what}: other doc"),
            Err(e) => println!("{what}: parse error {e}"),
        }
    }
    let mut kinds = std::collections::BTreeMap::<&str, usize>::new();
    for f in fs::read_to_string(list).unwrap().lines() {
        let Ok(src) = fs::read_to_string(f) else {
            continue;
        };
        let Ok(doc) = parser::parse_document(&src) else {
            continue;
        };
        let k = match &doc {
            Document::Flowchart(g) => {
                check_graph(&mut t, f, g);
                "flowchart"
            }
            Document::State(g) => {
                check_graph(&mut t, f, g);
                "state"
            }
            Document::Class(d) => {
                check_class(&mut t, f, d);
                "class"
            }
            Document::Er(d) => {
                check_er(&mut t, f, d);
                "er"
            }
            Document::Mindmap(d) => {
                check_mind(&mut t, f, d);
                "mindmap"
            }
            _ => continue,
        };
        *kinds.entry(k).or_default() += 1;
        t.files += 1;
    }
    println!(
        "files {} kinds {:?} equal {} none/ok {} failures {}",
        t.files,
        kinds,
        t.eq,
        t.none_ok,
        t.fail.len()
    );
    for f in &t.fail {
        println!("FAIL {f}");
    }
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

pub fn fuzz(list: &str, seed: u64, iters: usize) {
    let mut r = Rng(seed.max(1));
    let mut panics = 0;
    let mut runs = 0;
    let mut slow = Vec::new();
    for f in fs::read_to_string(list).unwrap().lines() {
        let Ok(src) = fs::read_to_string(f) else {
            continue;
        };
        let Ok(doc) = parser::parse_document(&src) else {
            continue;
        };
        for it in 0..iters {
            let n = match &doc {
                Document::Flowchart(g) | Document::State(g) => g.nodes.len(),
                Document::Class(d) => d.classes.len() + if it % 2 == 0 { d.notes.len() } else { 0 },
                Document::Er(d) => d.entities.len(),
                Document::Mindmap(d) => d.nodes.len(),
                _ => break,
            };
            // scale: small (jitter), medium, the whole reach
            let span = [300.0, 5000.0, 1.0e6][it % 3];
            let p: Vec<(f64, f64)> = (0..n)
                .map(|_| ((r.next() * 2.0 - 1.0) * span, (r.next() * 2.0 - 1.0) * span))
                .collect();
            let t0 = std::time::Instant::now();
            let ok = guarded(|| match &doc {
                Document::Flowchart(g) | Document::State(g) => {
                    let s = scene::try_route(g, &p).unwrap();
                    scene::to_svg(&s).len()
                }
                Document::Class(d) => class::to_svg(&class::try_route(d, &p).unwrap()).len(),
                Document::Er(d) => er::to_svg(&er::try_route(d, &p).unwrap()).len(),
                Document::Mindmap(d) => mindmap::to_svg(&mindmap::try_route(d, &p).unwrap()).len(),
                _ => 0,
            });
            runs += 1;
            let ms = t0.elapsed().as_millis();
            if ms > 3000 {
                slow.push(format!("{f} it {it} span {span}: {ms} ms"));
            }
            if ok.is_err() {
                panics += 1;
                println!("PANIC {f} it {it} span {span}");
            }
        }
    }
    println!("fuzz runs {runs} panics {panics} slow {}", slow.len());
    for s in slow {
        println!("SLOW {s}");
    }
}
