//! Edge cases of the #111 / #112 contracts.
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser};

fn dbg<T: std::fmt::Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn main() {
    // 1. An edit of `direction` only (same entities/classes, same counts).
    let er_a = "erDiagram\nA ||--o{ B : has\nB ||--|{ C : holds\nC }o--|| D : in\nD ||--o{ A : back\nA ||--|| C : x\n";
    let er_b = format!("erDiagram\ndirection LR\n{}", &er_a["erDiagram\n".len()..]);
    let (Ok(Document::Er(da)), Ok(Document::Er(db))) = (parser::parse_document(er_a), parser::parse_document(&er_b)) else {
        panic!()
    };
    let base = er::scene(&da);
    let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut to = c.clone();
    to[0].0 += 40.0;
    let ra = er::route(&da, &to);
    let rb = er::route(&db, &to);
    let pb = er::route_partial(&db, &to, &base, &c);
    println!(
        "ER direction edit: route depends on direction: {}; route_partial(edited) == route(edited): {}",
        dbg(&ra) != dbg(&rb),
        dbg(&pb) == dbg(&rb)
    );

    let cl_a = "classDiagram\nA <|-- B\nB *-- C\nC --> D : uses\nD o-- A\nA --> C\nB ..> D\n";
    let cl_b = format!("classDiagram\ndirection LR\n{}", &cl_a["classDiagram\n".len()..]);
    let (Ok(Document::Class(da)), Ok(Document::Class(db))) = (parser::parse_document(cl_a), parser::parse_document(&cl_b)) else {
        panic!()
    };
    let base = class::scene(&da);
    let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut to = c.clone();
    to[0].0 += 40.0;
    let ra = class::route(&da, &to);
    let rb = class::route(&db, &to);
    let pb = class::route_partial(&db, &to, &base, &c);
    println!(
        "Class direction edit: route depends on direction: {}; route_partial(edited) == route(edited): {}",
        dbg(&ra) != dbg(&rb),
        dbg(&pb) == dbg(&rb)
    );

    // 2. #76: a drag past the top-left: route_partial's canvas vs route's.
    for src in [er_a, cl_a] {
        match parser::parse_document(src) {
            Ok(Document::Er(d)) => {
                let base = er::scene(&d);
                let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut to = c.clone();
                to[1] = (-200.0, -120.0);
                let p = er::route_partial(&d, &to, &base, &c);
                let r = er::route(&d, &to);
                println!(
                    "ER past top-left: node kept {:?} vs {:?}; origin partial {:?} route {:?}; size partial {}x{} route {}x{}",
                    (p.scene.nodes[1].x, p.scene.nodes[1].y), to[1], p.scene.origin, r.scene.origin,
                    p.scene.width, p.scene.height, r.scene.width, r.scene.height
                );
                let svg = er::to_svg(&p);
                let vb = &svg[svg.find("viewBox=\"").unwrap()..];
                println!("   partial {}", &vb[..vb.find('>').unwrap().min(60)]);
                let svg = er::to_svg(&r);
                let vb = &svg[svg.find("viewBox=\"").unwrap()..];
                println!("   route   {}", &vb[..vb.find('>').unwrap().min(60)]);
            }
            Ok(Document::Class(d)) => {
                let base = class::scene(&d);
                let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut to = c.clone();
                to[1] = (-200.0, -120.0);
                let p = class::route_partial(&d, &to, &base, &c);
                let r = class::route(&d, &to);
                println!(
                    "Class past top-left: node kept {:?} vs {:?}; origin partial {:?} route {:?}; size partial {}x{} route {}x{}",
                    (p.scene.nodes[1].x, p.scene.nodes[1].y), to[1], p.scene.origin, r.scene.origin,
                    p.scene.width, p.scene.height, r.scene.width, r.scene.height
                );
            }
            _ => {}
        }
    }

    // 3. Mindmap route_partial vs route equality (Debug) on the example.
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    if let Ok(Document::Mindmap(d)) = parser::parse_document(&src) {
        let base = mindmap::scene(&d);
        let c: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        let (mut eq, mut n) = (0, 0);
        for i in 0..c.len() {
            let mut to = c.clone();
            to[i].0 -= 41.0;
            let p = mindmap::route_partial(&d, &to, &base, &c);
            let r = mindmap::route(&d, &to);
            n += 1;
            if dbg(&p) == dbg(&r) {
                eq += 1;
            }
        }
        println!("mindmap example: route_partial == route (Debug) in {eq}/{n} frames");
        // Mindmap past the top-left
        let mut to = c.clone();
        to[1] = (-100.0, -50.0);
        let p = mindmap::route_partial(&d, &to, &base, &c);
        let r = mindmap::route(&d, &to);
        println!("mindmap past top-left: partial {}x{} route {}x{} node {:?}", p.width, p.height, r.width, r.height, (p.nodes[1].cx(), p.nodes[1].cy()));
        let picked = p.hit_test(p.nodes[1].cx(), p.nodes[1].cy(), 0.0);
        println!("   pick at the dragged node's centre (off canvas): {picked:?}");
    }
}
