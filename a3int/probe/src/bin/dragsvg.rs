use flowmaid::model::Document;
use flowmaid::scene::{route_partial, scene, to_svg};
// usage: dragsvg <file.mmd> <node> <dx> <dy> <out.svg>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::State(g) => g, _ => panic!() };
    let sc = scene(&g);
    let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let ci = g.node_index(&a[2]).unwrap();
    let mut m = base.clone();
    m[ci].0 += a[3].parse::<f64>().unwrap();
    m[ci].1 += a[4].parse::<f64>().unwrap();
    let rp = route_partial(&g, &m, &sc, &base);
    std::fs::write(&a[5], to_svg(&rp)).unwrap();
}
