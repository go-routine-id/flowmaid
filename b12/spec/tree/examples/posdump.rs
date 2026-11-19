//! Dump each node's centre and size from the static scene: `id x y w h` per line.
use flowmaid::Document;
fn main() {
    let path = std::env::args().nth(1).expect("file");
    let src = std::fs::read_to_string(&path).expect("read");
    let g = match flowmaid::parser::parse_document(&src).expect("parses") {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!("not a flowchart"),
    };
    let sc = flowmaid::scene::scene(&g);
    for n in &sc.nodes {
        println!("{}\t{}\t{}\t{}\t{}", n.id, n.x, n.y, n.w, n.h);
    }
}
