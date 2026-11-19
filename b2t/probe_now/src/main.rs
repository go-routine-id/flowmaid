use flowmaid::model::Document;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => continue,
        };
        let s = flowmaid::scene::scene(&g);
        let minx = s.nodes.iter().map(|n| n.x - n.w / 2.0).fold(f64::INFINITY, f64::min);
        let miny = s.nodes.iter().map(|n| n.y - n.h / 2.0).fold(f64::INFINITY, f64::min);
        println!("{f}: n0 {:?} minx {:?} miny {:?} w {:?} h {:?}", (s.nodes[0].x, s.nodes[0].y), minx, miny, s.width, s.height);
    }
}
