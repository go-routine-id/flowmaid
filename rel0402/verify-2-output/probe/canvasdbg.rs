use flowmaid::model::Document;
use flowmaid::{class, parser::parse_document};
fn bbox(sc: &flowmaid::scene::Scene) -> (f64, f64, f64, f64) {
    let mut b = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for n in &sc.nodes {
        b.0 = b.0.min(n.x - n.w / 2.0); b.1 = b.1.min(n.y - n.h / 2.0);
        b.2 = b.2.max(n.x + n.w / 2.0); b.3 = b.3.max(n.y + n.h / 2.0);
    }
    b
}
fn show(tag: &str, cs: &class::ClassScene) {
    let sc = &cs.scene;
    let b = bbox(sc);
    let svg = class::to_svg(cs);
    let vb = &svg[svg.find("viewBox=\"").unwrap() + 9..];
    let vb = &vb[..vb.find('"').unwrap()];
    println!("{tag:10} origin={:?} w={:.1} h={:.1} -> canvas x {:.1}..{:.1} y {:.1}..{:.1} | node bbox x {:.1}..{:.1} y {:.1}..{:.1} | viewBox {vb}",
        sc.origin, sc.width, sc.height, sc.origin.0, sc.origin.0 + sc.width, sc.origin.1, sc.origin.1 + sc.height, b.0, b.2, b.1, b.3);
}
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let i: usize = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(0);
    let src = std::fs::read_to_string(path).unwrap();
    let Ok(Document::Class(d)) = parse_document(&src) else { panic!() };
    let laid = class::scene(&d);
    let nc = d.classes.len();
    let all: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let cls = all[..nc].to_vec();
    show("laid", &laid);
    let routed = class::route(&d, &cls);
    show("routed", &routed);
    let rat: Vec<(f64, f64)> = routed.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    for mv in [(900.0, 500.0), (-3.0, 260.0), (0.0, 2000.0), (2000.0, 0.0)] {
        let mut to = cls.clone();
        to[i] = (to[i].0 + mv.0, to[i].1 + mv.1);
        println!("-- move class {i} by {mv:?}");
        show("route", &class::route(&d, &to));
        show("rp(routed)", &class::route_partial(&d, &to, &routed, &rat[..nc]));
        show("rp(rt,all)", &class::route_partial(&d, &to, &routed, &rat));
        let mut toall = rat.clone();
        toall[i] = to[i];
        show("rp(all)", &class::route_partial(&d, &toall, &routed, &rat));
        show("rp(laid)", &class::route_partial(&d, &{ let mut t = all.clone(); t[i] = to[i]; t }, &laid, &all));
    }
}
