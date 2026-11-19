use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{class, er};
fn report(tag: &str, sc: &flowmaid::scene::Scene, svg: &str) {
    let (ox, oy) = sc.origin;
    let vb = &svg[svg.find("viewBox=\"").unwrap() + 9..];
    let vb = &vb[..vb.find('"').unwrap()];
    let maxr = sc.nodes.iter().map(|n| n.x + n.w / 2.0).fold(f64::MIN, f64::max);
    let maxb = sc.nodes.iter().map(|n| n.y + n.h / 2.0).fold(f64::MIN, f64::max);
    println!("{tag}: origin ({ox:.1},{oy:.1}) right {:.1} bottom {:.1} | boxes reach right {maxr:.1} bottom {maxb:.1} | viewBox {vb}", ox + sc.width, oy + sc.height);
}
fn main() {
    let src = "erDiagram\n  A ||--o{ B : x\n  B ||--|{ C : y\n  C }o--|| D : z\n";
    let Ok(Document::Er(d)) = parse_document(src) else { panic!() };
    let laid = er::scene(&d);
    let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    // frame 1: A dragged past the top-left
    let mut p1 = at.clone();
    p1[0] = (-300.0, -200.0);
    let f1 = er::route_partial(&d, &p1, &laid, &at);
    report("er frame1 (A to -300,-200)", &f1.scene, &er::to_svg(&f1));
    // frame 2 (chained): D dragged to the right/bottom
    let mut p2 = p1.clone();
    let right = f1.scene.origin.0 + f1.scene.width;
    let bottom = f1.scene.origin.1 + f1.scene.height;
    p2[3] = (right - 10.0, bottom - 10.0);
    let f2 = er::route_partial(&d, &p2, &f1, &p1);
    report("er frame2 partial (D to far corner)", &f2.scene, &er::to_svg(&f2));
    let r2 = er::route(&d, &p2);
    report("er route same positions        ", &r2.scene, &er::to_svg(&r2));
    // base = route() with negative origin
    let rb = er::route(&d, &p1);
    let f3 = er::route_partial(&d, &p2, &rb, &p1);
    report("er partial from route base     ", &f3.scene, &er::to_svg(&f3));

    let csrc = "classDiagram\n  A --> B\n  B --> C\n  C --> D\n";
    let Ok(Document::Class(cd)) = parse_document(csrc) else { panic!() };
    let laid = class::scene(&cd);
    let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut p1 = at.clone();
    p1[0] = (-300.0, -200.0);
    let f1 = class::route_partial(&cd, &p1, &laid, &at);
    report("class frame1", &f1.scene, &class::to_svg(&f1));
    let mut p2 = p1.clone();
    let right = f1.scene.origin.0 + f1.scene.width;
    let bottom = f1.scene.origin.1 + f1.scene.height;
    p2[3] = (right - 10.0, bottom - 10.0);
    let f2 = class::route_partial(&cd, &p2, &f1, &p1);
    report("class frame2 partial", &f2.scene, &class::to_svg(&f2));
    let r2 = class::route(&cd, &p2);
    report("class route same pos", &r2.scene, &class::to_svg(&r2));
}
