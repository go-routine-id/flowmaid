use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::class;
fn ext(cs: &class::ClassScene, tag: &str) {
    let sc = &cs.scene;
    let (ox, oy) = sc.origin;
    println!("{tag}: origin ({ox:.2},{oy:.2}) size {:.2} x {:.2} -> right {:.2} bottom {:.2}", sc.width, sc.height, ox + sc.width, oy + sc.height);
    for n in &sc.nodes {
        let (l, t, r, b) = (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0);
        if l < ox - 1e-6 || t < oy - 1e-6 || r > ox + sc.width + 1e-6 || b > oy + sc.height + 1e-6 {
            println!("   box {} off: l {l:.2} t {t:.2} r {r:.2} b {b:.2}", n.id);
        }
    }
}
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let i: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let Ok(Document::Class(d)) = parse_document(&std::fs::read_to_string(&f).unwrap()) else { panic!() };
    let n = d.classes.len();
    let laid = class::scene(&d);
    let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    ext(&laid, "laid");
    let routed = class::route(&d, &at);
    ext(&routed, "route(at)");
    let mut to = at.clone();
    to[i].0 += 37.0;
    to[i].1 -= 23.0;
    ext(&class::route(&d, &to), "route(to)");
    let f1 = class::route_partial(&d, &to, &routed, &at);
    ext(&f1, "partial(to, base=routed)");
    let f2 = class::route_partial(&d, &to, &laid, &at);
    ext(&f2, "partial(to, base=laid)");
    let routed_c = class::route(&d, &at[..n]);
    ext(&routed_c, "route(at classes only)");
    let svg = class::to_svg(&f1);
    let vb = &svg[svg.find("viewBox=\"").unwrap() + 9..];
    println!("partial svg viewBox {}", &vb[..vb.find('"').unwrap()]);
    let svg = class::to_svg(&class::route(&d, &to));
    let vb = &svg[svg.find("viewBox=\"").unwrap() + 9..];
    println!("route svg viewBox {}", &vb[..vb.find('"').unwrap()]);
}
