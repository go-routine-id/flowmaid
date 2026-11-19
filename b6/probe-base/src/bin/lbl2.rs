use flowmaid::{parser::parse_document, Document};
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&f).unwrap();
    let g = match parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => return };
    for nd in &g.nodes { let w = flowmaid::layout::text_width(&nd.label); if !nd.label.contains('\n') && w > 200.0 { println!("{} {:.0} {}", nd.id, w, nd.label); } }
}
