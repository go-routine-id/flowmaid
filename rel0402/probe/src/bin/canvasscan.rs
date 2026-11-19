// Canvas scan for ER / class route_partial frames: does each frame's
// canvas (origin, width, height) hold every box? Scenarios:
//  A) chained: laid -> f1 (box 0 to (-150,-90)) -> f2 (last box +600,+400),
//     also +300,+200 and -3,+260 and +260,-3;
//  B) class only: base = class::route(d, class centres) (notes placed
//     beside, a free note may lie above: origin < 0 without a drag), each
//     class moved by (+900,+500), (-3,+260), (+37,0) (at most 12 classes).
// usage: canvasscan <file.mmd>...
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::scene::Scene;
use flowmaid::{class, er};

fn covers(sc: &Scene) -> bool {
    let (ox, oy) = sc.origin;
    sc.nodes.iter().all(|n| {
        n.x - n.w / 2.0 >= ox - 1e-6
            && n.y - n.h / 2.0 >= oy - 1e-6
            && n.x + n.w / 2.0 <= ox + sc.width + 1e-6
            && n.y + n.h / 2.0 <= oy + sc.height + 1e-6
    })
}

fn main() {
    let moves = [(600.0, 400.0), (300.0, 200.0), (-3.0, 260.0), (260.0, -3.0)];
    let (mut files, mut frames, mut bad_p, mut bad_r) = (0usize, 0usize, 0usize, 0usize);
    let (mut neg_base, mut b_frames, mut b_bad) = (0usize, 0usize, 0usize);
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let Ok(doc) = parse_document(&src) else { continue };
        match doc {
            Document::Er(d) if d.entities.len() >= 2 => {
                files += 1;
                let laid = er::scene(&d);
                let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut p1 = at.clone();
                p1[0] = (-150.0, -90.0);
                let f1 = er::route_partial(&d, &p1, &laid, &at);
                frames += 1;
                if !covers(&f1.scene) {
                    bad_p += 1;
                    println!("BAD {f} f1");
                }
                let k = at.len() - 1;
                for (dx, dy) in moves {
                    let mut p2 = p1.clone();
                    p2[k].0 += dx;
                    p2[k].1 += dy;
                    let f2 = er::route_partial(&d, &p2, &f1, &p1);
                    frames += 1;
                    if !covers(&f2.scene) {
                        bad_p += 1;
                        println!("BAD {f} chained +({dx},{dy})");
                    }
                    if !covers(&er::route(&d, &p2).scene) {
                        bad_r += 1;
                    }
                }
            }
            Document::Class(d) if d.classes.len() >= 2 => {
                files += 1;
                let laid = class::scene(&d);
                let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut p1 = at.clone();
                p1[0] = (-150.0, -90.0);
                let f1 = class::route_partial(&d, &p1, &laid, &at);
                frames += 1;
                if !covers(&f1.scene) {
                    bad_p += 1;
                    println!("BAD {f} f1");
                }
                let k = d.classes.len() - 1;
                for (dx, dy) in moves {
                    let mut p2 = p1.clone();
                    p2[k].0 += dx;
                    p2[k].1 += dy;
                    let f2 = class::route_partial(&d, &p2, &f1, &p1);
                    frames += 1;
                    if !covers(&f2.scene) {
                        bad_p += 1;
                        println!("BAD {f} chained +({dx},{dy})");
                    }
                    if !covers(&class::route(&d, &p2).scene) {
                        bad_r += 1;
                    }
                }
                let n = d.classes.len();
                let cc = &at[..n];
                let base = class::route(&d, cc);
                if base.scene.origin.0 < 0.0 || base.scene.origin.1 < 0.0 {
                    neg_base += 1;
                }
                for i in 0..n.min(12) {
                    for (dx, dy) in [(900.0, 500.0), (-3.0, 260.0), (37.0, 0.0)] {
                        let mut to = cc.to_vec();
                        to[i].0 += dx;
                        to[i].1 += dy;
                        let fr = class::route_partial(&d, &to, &base, cc);
                        b_frames += 1;
                        if !covers(&fr.scene) {
                            b_bad += 1;
                            println!("BAD {f} on route(classes) box {i} +({dx},{dy})");
                        }
                    }
                }
            }
            _ => {}
        }
    }
    println!(
        "files {files}: chained frames {frames}, not holding their boxes {bad_p} (route at the same positions: {bad_r}); \
         class bases on route(class centres) {neg_base} with origin < 0, frames {b_frames}, not holding {b_bad}"
    );
}
