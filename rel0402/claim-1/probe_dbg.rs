use flowmaid::model::Document;
use flowmaid::{class, parser, scene};
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&f).unwrap();
    match parser::parse_document(&src).unwrap() {
        Document::Class(d) => println!("{:#?}", class::scene(&d)),
        Document::Flowchart(g) | Document::State(g) => println!("{:#?}", scene::scene(&g)),
        _ => {}
    }
}
