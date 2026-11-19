use flowmaid::{mindmap, model::Document, parser::parse_document};
fn main() {
    let Ok(Document::Mindmap(d)) = parse_document("mindmap\n  root\n    a\n    b\n      c") else { return };
    let base = mindmap::scene(&d);
    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
    println!("base canvas {}x{}", base.width, base.height);
    let mut mv = at.clone();
    mv[1] = (-300.0, -200.0);
    let f = mindmap::route_partial(&d, &mv, &base, &at);
    let r = mindmap::route(&d, &mv);
    println!("frame canvas {}x{}  node1 box x={} y={} (route canvas {}x{})", f.width, f.height, f.nodes[1].x, f.nodes[1].y, r.width, r.height);
    let svg = mindmap::to_svg(&f);
    let vb = &svg[svg.find("viewBox").unwrap()..][..40];
    println!("svg {vb}");
    let mut mv2 = at.clone();
    mv2[1].0 += 2000.0;
    let f2 = mindmap::route_partial(&d, &mv2, &base, &at);
    let f3 = mindmap::route_partial(&d, &at, &f2, &mv2);
    println!("drag right: {}x{}; back from that frame: {}x{} (base {}x{})", f2.width, f2.height, f3.width, f3.height, base.width, base.height);
}
