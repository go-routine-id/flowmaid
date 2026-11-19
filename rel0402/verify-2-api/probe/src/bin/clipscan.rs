use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{class, er};
fn covers(sc: &flowmaid::scene::Scene) -> bool {
    let (ox, oy) = sc.origin;
    sc.nodes.iter().all(|n| n.x + n.w / 2.0 <= ox + sc.width + 1e-6 && n.y + n.h / 2.0 <= oy + sc.height + 1e-6)
}
fn main() {
    let (mut tot, mut bad_p, mut bad_r) = (0, 0, 0);
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let Ok(doc) = parse_document(&src) else { continue };
        match doc {
            Document::Er(d) if d.entities.len() >= 2 => {
                let laid = er::scene(&d);
                let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut p1 = at.clone();
                p1[0] = (-400.0, -300.0);
                let b = er::route(&d, &p1);
                let mut p2 = p1.clone();
                let k = at.len() - 1;
                p2[k] = (b.scene.origin.0 + b.scene.width + 20.0, b.scene.origin.1 + b.scene.height + 20.0);
                let f2 = er::route_partial(&d, &p2, &b, &p1);
                tot += 1;
                if !covers(&f2.scene) { bad_p += 1; }
                if !covers(&er::route(&d, &p2).scene) { bad_r += 1; }
            }
            Document::Class(d) if d.classes.len() >= 2 => {
                let laid = class::scene(&d);
                let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut p1 = at.clone();
                p1[0] = (-400.0, -300.0);
                let b = class::route(&d, &p1);
                let mut p2 = p1.clone();
                let k = d.classes.len() - 1;
                p2[k] = (b.scene.origin.0 + b.scene.width + 20.0, b.scene.origin.1 + b.scene.height + 20.0);
                let f2 = class::route_partial(&d, &p2, &b, &p1);
                tot += 1;
                if !covers(&f2.scene) { bad_p += 1; }
                if !covers(&class::route(&d, &p2).scene) { bad_r += 1; }
            }
            _ => {}
        }
    }
    println!("diagrams {tot}: route_partial frame does not hold its boxes in {bad_p}; route does not in {bad_r}");
}
