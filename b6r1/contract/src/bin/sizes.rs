fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&path).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        flowmaid::model::Document::Flowchart(g) | flowmaid::model::Document::State(g) => g,
        _ => panic!(),
    };
    let sc = flowmaid::scene::scene(&g);
    for n in &sc.nodes {
        println!("{:>4} {:?} {:.1}x{:.1} aspect {:.2} {:?}", n.id, n.shape, n.w, n.h, n.w / n.h.max(1e-9), n.label);
    }
}
