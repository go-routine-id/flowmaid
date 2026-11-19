//! For class diagrams: kept relations (no end on a moved box) whose
//! cardinality spot (class::card_spots, what the painter draws) differs
//! between the base and the route_partial frame.
use flowmaid::model::Document;
use flowmaid::{class, parser};

fn main() {
    let list = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let (mut files, mut frames, mut kept, mut movedc, mut maxd) = (0, 0, 0, 0, 0.0f64);
    let mut shown = 0;
    let mut files_with = 0;
    for path in list.lines().filter(|l| !l.is_empty()) {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let Ok(Document::Class(d)) = parser::parse_document(&src) else { continue };
        files += 1;
        let base = class::scene(&d);
        let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let nc = d.classes.len();
        let bc = class::card_spots(&base);
        let mut any = false;
        let step = (nc / 6).max(1);
        for i in (0..nc).step_by(step) {
            let mut to = c.clone();
            to[i].0 += 37.0;
            to[i].1 += 23.0;
            let f = class::route_partial(&d, &to, &base, &c);
            frames += 1;
            let fc = class::card_spots(&f);
            let idx = |id: &str| f.scene.nodes.iter().position(|n| n.id == id).unwrap();
            for (j, e) in base.scene.edges.iter().enumerate().take(d.relations.len()) {
                let (a, z) = (idx(&e.from), idx(&e.to));
                let mv = |k: usize| (f.scene.nodes[k].x, f.scene.nodes[k].y) != (base.scene.nodes[k].x, base.scene.nodes[k].y);
                if mv(a) || mv(z) {
                    continue;
                }
                kept += 1;
                if fc[j] != bc[j] {
                    movedc += 1;
                    any = true;
                    let mut dd = 0.0f64;
                    for s in 0..2 {
                        if let (Some(p), Some(q)) = (fc[j][s], bc[j][s]) {
                            dd = dd.max((p.0 - q.0).hypot(p.1 - q.1));
                        }
                    }
                    maxd = maxd.max(dd);
                    if shown < 8 {
                        shown += 1;
                        println!(
                            "{path} drag box {i}: kept rel {j} ({} -> {}) card spots base {:?} frame {:?} (moved {dd:.1}px); edge identical: {}",
                            e.from, e.to, bc[j], fc[j],
                            format!("{:?}", f.scene.edges[j]) == format!("{:?}", e)
                        );
                    }
                }
            }
        }
        if any {
            files_with += 1;
        }
    }
    println!("class files {files}, frames {frames}, kept relations {kept}, kept with a moved cardinality {movedc}, files with any {files_with}, max shift {maxd:.1}px");
}
