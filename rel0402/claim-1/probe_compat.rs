//! Compat probe: built against 0.40.1 and 0.40.2, prints one line per
//! input file with hashes of static and routed output of APIs that exist
//! in both. Usage: probe_compat <list-file>
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser, scene};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn h<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn moves(at: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let mut out = vec![at.to_vec()];
    if at.is_empty() {
        return out;
    }
    let i = at.len() / 2;
    let mut a = at.to_vec();
    a[i] = (a[i].0 + 37.0, a[i].1 + 23.0);
    out.push(a);
    let mut b = at.to_vec();
    b[0] = (-150.0, -90.0);
    out.push(b);
    let mut c = at.to_vec();
    let last = c.len() - 1;
    c[last] = (c[last].0 + 400.0, c[last].1 + 300.0);
    out.push(c);
    out
}

fn picks(sc: &scene::Scene) -> String {
    let mut s = String::new();
    let steps = 12;
    for i in 0..=steps {
        for j in 0..=steps {
            let x = sc.width * i as f64 / steps as f64;
            let y = sc.height * j as f64 / steps as f64;
            s.push_str(&format!("{:?};", sc.hit_test(x, y, 3.0)));
        }
    }
    for n in &sc.nodes {
        s.push_str(&format!("{:?};", sc.hit_test(n.x, n.y, 0.0)));
    }
    s
}

fn flow(g: &flowmaid::model::Graph) -> String {
    let base = scene::scene(g);
    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut parts = vec![
        format!("s{:x}", h(&scene::to_svg(&base))),
        format!("p{:x}", h(&picks(&base))),
        format!("DBG{:x}", h(&format!("{:?}", base))),
    ];
    if g.nodes.len() > 120 {
        return parts.join(" ");
    }
    for m in moves(&at) {
        let r = scene::route(g, &m);
        parts.push(format!("r{:x}", h(&scene::to_svg(&r))));
        parts.push(format!("rp{:x}", h(&picks(&r))));
        let p = scene::route_partial(g, &m, &base, &at);
        parts.push(format!("q{:x}", h(&scene::to_svg(&p))));
    }
    parts.join(" ")
}

fn main() {
    let list = std::env::args().nth(1).expect("list");
    for f in std::fs::read_to_string(&list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        let out = match parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => flow(&g),
            Ok(Document::Er(d)) => {
                let base = er::scene(&d);
                let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut parts = vec![
                    format!("s{:x}", h(&er::to_svg(&base))),
                    format!("p{:x}", h(&picks(&base.scene))),
                ];
                if d.entities.len() <= 80 {
                    for m in moves(&at) {
                        let r = er::route(&d, &m);
                        parts.push(format!("r{:x}", h(&er::to_svg(&r))));
                        parts.push(format!("rp{:x}", h(&picks(&r.scene))));
                    }
                }
                parts.join(" ")
            }
            Ok(Document::Class(d)) => {
                let base = class::scene(&d);
                let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut parts = vec![
                    format!("s{:x}", h(&class::to_svg(&base))),
                    format!("p{:x}", h(&picks(&base.scene))),
                    format!("c{:x}", h(&format!("{:?}", class::card_spots(&base)))),
                    format!("d{:x}", h(&format!("{:?}", base))),
                ];
                if d.classes.len() <= 60 {
                    for m in moves(&at) {
                        let r = class::route(&d, &m);
                        parts.push(format!("r{:x}", h(&class::to_svg(&r))));
                        parts.push(format!("rc{:x}", h(&format!("{:?}", class::card_spots(&r)))));
                    }
                }
                parts.join(" ")
            }
            Ok(Document::Mindmap(d)) => {
                let base = mindmap::scene(&d);
                let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                let mut parts = vec![format!("s{:x}", h(&mindmap::to_svg(&base)))];
                for m in moves(&at) {
                    let r = mindmap::route(&d, &m);
                    parts.push(format!("r{:x}", h(&mindmap::to_svg(&r))));
                }
                parts.join(" ")
            }
            Ok(_) => "other".into(),
            Err(e) => format!("err {:x}", h(&format!("{e:?}"))),
        };
        println!("{f}\t{out}");
    }
}
