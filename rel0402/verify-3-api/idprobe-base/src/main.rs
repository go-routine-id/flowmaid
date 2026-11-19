// Identity probe: prints a hash per routed output; built against base and head.
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{class, er, mindmap, scene};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn h(s: &str) -> u64 {
    let mut x = DefaultHasher::new();
    s.hash(&mut x);
    x.finish()
}

fn drags(at: &[(f64, f64)]) -> Vec<(String, Vec<(f64, f64)>)> {
    let n = at.len();
    let mut out = vec![("at".to_string(), at.to_vec())];
    if n == 0 {
        return out;
    }
    for (i, d) in [(0, (37.0, 23.0)), (n - 1, (-3000.0, -2000.0)), (n / 2, (1500.0, 900.0)), (n / 3, (-40.0, -15.0)), (0, (0.0, -600.0))] {
        let mut m = at.to_vec();
        m[i].0 += d.0;
        m[i].1 += d.1;
        out.push((format!("i{i}({},{})", d.0, d.1), m));
    }
    out
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let list = std::env::args().nth(1).unwrap();
    for path in std::fs::read_to_string(&list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let Ok(Ok(doc)) = catch_unwind(|| parse_document(&src)) else { continue };
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut lines = Vec::new();
            match &doc {
                Document::Flowchart(g) | Document::State(g) => {
                    let base = scene::scene(g);
                    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
                    for (t, mv) in drags(&at) {
                        let r = scene::route(g, &mv);
                        lines.push(format!("route {t} {:x} {:x}", h(&format!("{r:?}")), h(&scene::to_svg(&r))));
                        let p = scene::route_partial(g, &mv, &base, &at);
                        lines.push(format!("rp {t} {:x} {:x}", h(&format!("{p:?}")), h(&scene::to_svg(&p))));
                        let mut mv2 = mv.clone();
                        if let Some(q) = mv2.first_mut() {
                            q.0 += 13.0;
                        }
                        let p2 = scene::route_partial(g, &mv2, &p, &mv);
                        lines.push(format!("rp2 {t} {:x} {:x}", h(&format!("{p2:?}")), h(&scene::to_svg(&p2))));
                    }
                }
                Document::Er(d) => {
                    let base = er::scene(d);
                    let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    for (t, mv) in drags(&at) {
                        let r = er::route(d, &mv);
                        lines.push(format!("route {t} {:x} {:x}", h(&format!("{r:?}")), h(&er::to_svg(&r))));
                    }
                }
                Document::Class(d) => {
                    let base = class::scene(d);
                    let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    lines.push(format!("scene {:x} {:?}", h(&format!("{base:?}")), class::card_spots(&base).len()));
                    for (t, mv) in drags(&at) {
                        let r = class::route(d, &mv);
                        lines.push(format!("route {t} {:x} {:x} {:x}", h(&format!("{r:?}")), h(&class::to_svg(&r)), h(&format!("{:?}", class::card_spots(&r)))));
                        let cls = &mv[..d.classes.len().min(mv.len())];
                        let r = class::route(d, cls);
                        lines.push(format!("routec {t} {:x} {:x}", h(&format!("{r:?}")), h(&class::to_svg(&r))));
                    }
                }
                Document::Mindmap(d) => {
                    let base = mindmap::scene(d);
                    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                    for (t, mv) in drags(&at) {
                        let r = mindmap::route(d, &mv);
                        lines.push(format!("route {t} {:x} {:x}", h(&format!("{r:?}")), h(&mindmap::to_svg(&r))));
                    }
                }
                _ => {}
            }
            lines
        }));
        match r {
            Ok(lines) => {
                for l in lines {
                    println!("{path}\t{l}");
                }
            }
            Err(_) => println!("{path}\tPANIC"),
        }
    }
}
