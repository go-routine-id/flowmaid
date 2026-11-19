//! route_partial vs route on extreme positions: a panic in route_partial where route does not is a finding.
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser::parse_document};
use std::io::BufRead;
use std::panic::{catch_unwind, AssertUnwindSafe};
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let list = std::env::args().nth(1).unwrap();
    let pts = [(1.0e6, 1.0e6), (-1.0e6, -1.0e6), (1.0e6, -1.0e6), (0.0, 0.0), (-0.0, 5.0), (3.0e5, 7.0), (f64::NAN, 1.0), (f64::INFINITY, 0.0)];
    let (mut n, mut bad, mut both) = (0, 0, 0);
    for line in std::io::BufReader::new(std::fs::File::open(list).unwrap()).lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let Ok(doc) = parse_document(&src) else { continue };
        for p in pts {
            let (rp, rr) = match &doc {
                Document::Er(d) if !d.entities.is_empty() => {
                    let b = er::scene(d); let at: Vec<(f64, f64)> = b.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let mut t = at.clone(); t[0] = p;
                    (catch_unwind(AssertUnwindSafe(|| { er::route_partial(d, &t, &b, &at); })).is_ok(), catch_unwind(AssertUnwindSafe(|| { er::route(d, &t); })).is_ok())
                }
                Document::Class(d) if !d.classes.is_empty() => {
                    let b = class::scene(d); let at: Vec<(f64, f64)> = b.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let mut t = at.clone(); let k = t.len() - 1; t[k] = p;
                    (catch_unwind(AssertUnwindSafe(|| { class::route_partial(d, &t, &b, &at); })).is_ok(), catch_unwind(AssertUnwindSafe(|| { class::route(d, &t); })).is_ok())
                }
                Document::Mindmap(d) if !d.nodes.is_empty() => {
                    let b = mindmap::scene(d); let at: Vec<(f64, f64)> = b.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                    let mut t = at.clone(); t[0] = p;
                    (catch_unwind(AssertUnwindSafe(|| { mindmap::route_partial(d, &t, &b, &at); })).is_ok(), catch_unwind(AssertUnwindSafe(|| { mindmap::route(d, &t); })).is_ok())
                }
                _ => continue,
            };
            n += 1;
            if !rp && rr { bad += 1; println!("PARTIAL_ONLY_PANIC {path} {p:?}"); }
            if !rp && !rr { both += 1; }
        }
    }
    println!("cases={n} partial-only panics={bad} both panic={both}");
}
