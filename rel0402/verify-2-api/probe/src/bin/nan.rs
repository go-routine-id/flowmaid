use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{class, er, mindmap};
use std::panic::{catch_unwind, AssertUnwindSafe};
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let which = std::env::args().nth(1).unwrap();
    let bad: f64 = std::env::args().nth(2).unwrap().parse().unwrap();
    let er_src = "erDiagram\n  A ||--o{ B : x\n  B ||--|{ C : y\n  C }o--|| D : z\n  A ||--|| D : w\n";
    let cl_src = "classDiagram\n  A <|-- B\n  B \"1\" --> \"*\" C : x\n  C --> D\n  A --> D\n  note for B \"n\"\n";
    match which.as_str() {
        "er" => {
            let Ok(Document::Er(d)) = parse_document(er_src) else { panic!() };
            let base = er::scene(&d);
            let at: Vec<_> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            let mut c = at.clone();
            c[1].0 = bad;
            let a = catch_unwind(AssertUnwindSafe(|| er::route(&d, &c).scene.nodes.len()));
            let b = catch_unwind(AssertUnwindSafe(|| er::route_partial(&d, &c, &base, &at).scene.nodes.len()));
            println!("er bad={bad} route ok={} partial ok={}", a.is_ok(), b.is_ok());
        }
        "class" => {
            let Ok(Document::Class(d)) = parse_document(cl_src) else { panic!() };
            let base = class::scene(&d);
            let at: Vec<_> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            let mut c = at.clone();
            c[1].0 = bad;
            let a = catch_unwind(AssertUnwindSafe(|| class::route(&d, &c).scene.nodes.len()));
            let b = catch_unwind(AssertUnwindSafe(|| class::route_partial(&d, &c, &base, &at).scene.nodes.len()));
            println!("class bad={bad} route ok={} partial ok={}", a.is_ok(), b.is_ok());
        }
        _ => {
            let Ok(Document::Mindmap(d)) = parse_document("mindmap\n  r\n    a\n    b\n") else { panic!() };
            let base = mindmap::scene(&d);
            let at: Vec<_> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
            let mut c = at.clone();
            c[1].0 = bad;
            let a = catch_unwind(AssertUnwindSafe(|| mindmap::route(&d, &c).nodes.len()));
            let b = catch_unwind(AssertUnwindSafe(|| mindmap::route_partial(&d, &c, &base, &at).nodes.len()));
            println!("mind bad={bad} route ok={} partial ok={}", a.is_ok(), b.is_ok());
        }
    }
}
