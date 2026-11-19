use flowmaid::model::{ClassDiagram, Document, ErDiagram, Graph, Mindmap};
use flowmaid::{class, er, mindmap, parser, scene};
use std::panic::{catch_unwind, AssertUnwindSafe};
fn pos(base: &[(f64, f64)], k: usize, mode: u8) -> Vec<(f64, f64)> {
    (0..k)
        .map(|i| match mode {
            0 => base.get(i).copied().unwrap_or((20.0 * i as f64, 15.0 * i as f64)),
            1 => (0.0, 0.0),
            2 => if i == 0 { (f64::NAN, 3.0) } else { base.get(i).copied().unwrap_or((1.0, 1.0)) },
            3 => (-1e6 + i as f64, 1e6),
            4 => (f64::INFINITY, 0.0),
            5 => (1e300 * (i as f64 + 1.0), -1e300),
            6 => (f64::MAX, f64::MIN),
            _ => (1e15 * i as f64, 1e15),
        })
        .collect()
}
fn chk(tag: &str, f: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(f)).is_err() {
        println!("PANIC {tag}");
    }
}
fn main() {
    if std::env::var("QUIET").is_ok() { std::panic::set_hook(Box::new(|_| {})); }
    let mut docs: Vec<(String, Document)> = Vec::new();
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        if let Ok(d) = parser::parse_document(&src) {
            docs.push((f, d));
        }
    }
    docs.push(("empty-flow".into(), Document::Flowchart(Graph::default())));
    docs.push(("empty-class".into(), Document::Class(ClassDiagram::default())));
    docs.push(("empty-er".into(), Document::Er(ErDiagram::default())));
    docs.push(("empty-mind".into(), Document::Mindmap(Mindmap::default())));
    for (f, d) in &docs {
        match d {
            Document::Flowchart(g) | Document::State(g) => {
                let sc = scene::scene(g);
                let c: Vec<_> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                let n = g.nodes.len();
                for mode in 0..8u8 {
                    let p = pos(&c, n, mode);
                    chk(&format!("{f} flow mode{mode}"), || {
                        let s = scene::route(g, &p);
                        let _ = scene::to_svg(&s);
                    });
                }
            }
            Document::Class(d) => {
                let cs = class::scene(d);
                let c: Vec<_> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                for k in [d.classes.len(), d.classes.len() + d.notes.len()] {
                    for mode in 0..8u8 {
                        let p = pos(&c, k, mode);
                        chk(&format!("{f} class k{k} mode{mode}"), || {
                            let _ = class::to_svg(&class::route(d, &p));
                        });
                    }
                }
            }
            Document::Er(d) => {
                let es = er::scene(d);
                let c: Vec<_> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                for mode in 0..8u8 {
                    let p = pos(&c, d.entities.len(), mode);
                    chk(&format!("{f} er mode{mode}"), || {
                        let _ = er::to_svg(&er::route(d, &p));
                    });
                }
            }
            Document::Mindmap(d) => {
                let ms = mindmap::scene(d);
                let c: Vec<_> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                for mode in 0..8u8 {
                    let p = pos(&c, d.nodes.len(), mode);
                    chk(&format!("{f} mind mode{mode}"), || {
                        let _ = mindmap::to_svg(&mindmap::route(d, &p));
                    });
                }
            }
            _ => {}
        }
    }
    println!("done {}", docs.len());
}
