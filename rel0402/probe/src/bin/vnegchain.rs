//! A frame built on a base whose canvas reaches up/left (negative origin, #76),
//! then a drag to the far side: does the canvas (and to_svg's viewBox) hold the drawing?
use flowmaid::model::Document;
use flowmaid::{class, er, parser::parse_document};
fn vb(svg: &str) -> String { let v = &svg[svg.find("viewBox=\"").unwrap() + 9..]; v[..v.find('"').unwrap()].to_string() }
fn report(tag: &str, sc: &flowmaid::scene::Scene, svg: &str) {
    let (mut mx, mut my) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for n in &sc.nodes { mx = mx.max(n.x + n.w / 2.0); my = my.max(n.y + n.h / 2.0); }
    for e in &sc.edges { for p in e.waypoints.iter().chain(e.bezier.iter()) { mx = mx.max(p.0); my = my.max(p.1); } }
    let (fx, fy) = (sc.origin.0 + sc.width, sc.origin.1 + sc.height);
    println!("{tag:14} origin=({:.1},{:.1}) far=({fx:.1},{fy:.1}) drawing reaches ({mx:.1},{my:.1}) {} viewBox={}", sc.origin.0, sc.origin.1,
        if mx > fx + 1e-6 || my > fy + 1e-6 { "CLIPPED" } else { "ok" }, vb(svg));
}
fn main() {
    let er_src = "erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER ||--|{ LINE : contains\n  LINE }o--|| PRODUCT : of\n  PRODUCT }|..|{ CATEGORY : in\n";
    let Ok(Document::Er(d)) = parse_document(er_src) else { panic!() };
    let base = er::scene(&d);
    let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut to1 = at.clone();
    to1[0] = (-150.0, -90.0);
    let f1 = er::route_partial(&d, &to1, &base, &at);
    report("er f1", &f1.scene, &er::to_svg(&f1));
    let mut to2 = to1.clone();
    to2[4] = (to2[4].0 + 300.0, to2[4].1 + 200.0);
    let f2 = er::route_partial(&d, &to2, &f1, &to1);
    report("er f2 partial", &f2.scene, &er::to_svg(&f2));
    let r2 = er::route(&d, &to2);
    report("er f2 route", &r2.scene, &er::to_svg(&r2));

    let cl_src = "classDiagram\n  A <|-- B\n  B *-- C\n  C --> D : uses\n  D o-- E\n";
    let Ok(Document::Class(c)) = parse_document(cl_src) else { panic!() };
    let base = class::scene(&c);
    let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut to1 = at.clone();
    to1[0] = (-150.0, -90.0);
    let f1 = class::route_partial(&c, &to1, &base, &at);
    report("class f1", &f1.scene, &class::to_svg(&f1));
    let mut to2 = to1.clone();
    to2[4] = (to2[4].0 + 300.0, to2[4].1 + 200.0);
    let f2 = class::route_partial(&c, &to2, &f1, &to1);
    report("class f2 part", &f2.scene, &class::to_svg(&f2));
    let r2 = class::route(&c, &to2);
    report("class f2 route", &r2.scene, &class::to_svg(&r2));
}
