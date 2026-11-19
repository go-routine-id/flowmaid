//! Over a file list: frame 1 drags node 0 to (-150,-90); frame 2 (built on frame 1) drags the last
//! node by (+600,+400). Is frame 2's canvas holding its nodes? Compare with route().
use flowmaid::model::Document;
use flowmaid::{class, er, parser::parse_document};
use std::io::BufRead;
fn clipped(sc: &flowmaid::scene::Scene) -> bool {
    let (fx, fy) = (sc.origin.0 + sc.width, sc.origin.1 + sc.height);
    sc.nodes.iter().any(|n| n.x + n.w / 2.0 > fx + 1e-6 || n.y + n.h / 2.0 > fy + 1e-6)
}
fn main() {
    let list = std::env::args().nth(1).unwrap();
    let (mut n, mut cp, mut cr) = (0, 0, 0);
    for line in std::io::BufReader::new(std::fs::File::open(list).unwrap()).lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        match parse_document(&src) {
            Ok(Document::Er(d)) if d.entities.len() >= 2 => {
                let base = er::scene(&d);
                let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut t1 = at.clone(); t1[0] = (-150.0, -90.0);
                let f1 = er::route_partial(&d, &t1, &base, &at);
                let mut t2 = t1.clone(); let k = t2.len() - 1; t2[k] = (t2[k].0 + 600.0, t2[k].1 + 400.0);
                let f2 = er::route_partial(&d, &t2, &f1, &t1);
                n += 1; if clipped(&f2.scene) { cp += 1; } if clipped(&er::route(&d, &t2).scene) { cr += 1; }
            }
            Ok(Document::Class(d)) if d.classes.len() >= 2 => {
                let base = class::scene(&d);
                let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut t1 = at.clone(); t1[0] = (-150.0, -90.0);
                let f1 = class::route_partial(&d, &t1, &base, &at);
                let mut t2 = t1.clone(); let k = d.classes.len() - 1; t2[k] = (t2[k].0 + 600.0, t2[k].1 + 400.0);
                let f2 = class::route_partial(&d, &t2, &f1, &t1);
                n += 1; if clipped(&f2.scene) { cp += 1; println!("CLIP {path}"); } if clipped(&class::route(&d, &t2).scene) { cr += 1; }
            }
            _ => {}
        }
    }
    println!("diagrams={n} route_partial_clipped={cp} route_clipped={cr}");
}
