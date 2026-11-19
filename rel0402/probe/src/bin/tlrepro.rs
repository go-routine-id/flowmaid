// Small hub class diagrams: does a frame (box 0 dragged past the top
// left) or route run a line past its canvas's top-left?
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::scene::Scene;
use flowmaid::class;
fn past(sc: &Scene) -> Option<String> {
    let (ox, oy) = sc.origin;
    for (j, e) in sc.edges.iter().enumerate() {
        for p in &e.waypoints {
            if p.0 < ox - 1e-6 || p.1 < oy - 1e-6 {
                return Some(format!("edge {j} {p:?} origin {:?}", sc.origin));
            }
        }
    }
    None
}
fn main() {
    let mut srcs: Vec<String> = Vec::new();
    for k in [4usize, 6, 8, 12] {
        let mut s = String::from("classDiagram\n  direction TB\n  class Hub {\n    +id int\n    +name String\n    +save() bool\n  }\n");
        for i in 0..k {
            if i % 2 == 0 {
                s.push_str(&format!("  class S{i} {{\n    +run()\n    -state int\n  }}\n"));
            }
        }
        for i in 0..k {
            s.push_str(&format!("  S{i} ..> Hub : uses{i}\n"));
        }
        srcs.push(s);
    }
    for s in &srcs {
        let Ok(Document::Class(d)) = parse_document(s) else { panic!() };
        let laid = class::scene(&d);
        let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        for who in 0..at.len() {
            for to in [(-150.0, -90.0), (-150.0, 60.0), (80.0, -90.0)] {
                let mut p = at.clone();
                p[who] = to;
                let f = class::route_partial(&d, &p, &laid, &at);
                let r = class::route(&d, &p);
                if let Some(w) = past(&f.scene) {
                    println!("n={} box {who} to {to:?}: frame {w}; route {:?}", at.len(), past(&r.scene));
                }
            }
        }
    }
    for k in 2..14 {
        for dir in ["TB", "LR"] {
            let mut s = format!("classDiagram\n  direction {dir}\n");
            for i in 0..k {
                s.push_str(&format!("  S{i} ..> Hub : uses{i}\n"));
            }
            let Ok(Document::Class(d)) = parse_document(&s) else { panic!() };
            let laid = class::scene(&d);
            let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            for who in 0..at.len() {
                for to in [(-150.0, -90.0), (-150.0, 60.0), (80.0, -90.0)] {
                    let mut p = at.clone();
                    p[who] = to;
                    let f = class::route_partial(&d, &p, &laid, &at);
                    let r = class::route(&d, &p);
                    if let Some(w) = past(&f.scene) {
                        println!("k={k} {dir} box {who} to {to:?}: frame {w}; route {:?}", past(&r.scene));
                    }
                }
            }
        }
    }
}
