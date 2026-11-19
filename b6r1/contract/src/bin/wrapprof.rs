use flowmaid::model::Document;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&path).unwrap();
    let Document::Flowchart(g) = flowmaid::parser::parse_document(&src).unwrap() else { panic!() };
    for _ in 0..20 {
        for n in &g.nodes { std::hint::black_box(flowmaid::layout::intrinsic_size_wrapped(n, g.wrapping_width(), flowmaid::layout::text_width)); }
    }
}
