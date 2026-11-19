use flowmaid::model::Document;
use flowmaid::{class, parser::parse_document};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let i: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let src = std::fs::read_to_string(path).unwrap();
    println!("{src}");
    let Ok(Document::Class(d)) = parse_document(&src) else { panic!() };
    let laid = class::scene(&d);
    let all: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut to = all.clone();
    to[i] = (to[i].0 - 3.0, to[i].1 + 260.0);
    let f = class::route_partial(&d, &to, &laid, &all);
    let r = class::route(&d, &to);
    for (tag, cs) in [("laid", &laid), ("partial", &f), ("route", &r)] {
        println!("== {tag} origin {:?} {}x{}", cs.scene.origin, cs.scene.width, cs.scene.height);
        for n in &cs.scene.nodes { println!("  node {} ({:.1},{:.1}) {}x{}", n.id, n.x, n.y, n.w, n.h); }
        for e in &cs.scene.edges { println!("  edge {}->{} wp {:?} bz {:?}", e.from, e.to, e.waypoints, e.bezier); }
        let svg = class::to_svg(cs);
        let vb = &svg[svg.find("viewBox=\"").unwrap() + 9..];
        println!("  viewBox {}", &vb[..vb.find('"').unwrap()]);
        for l in svg.lines().filter(|l| l.starts_with("<path")) { println!("  {}", &l[..l.len().min(200)]); }
    }
}
