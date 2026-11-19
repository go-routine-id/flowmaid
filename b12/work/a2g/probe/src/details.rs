//! Block mode: harness detail lines (static) of each file matching a word.
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    let mut a = std::env::args().skip(1);
    let word = a.next().unwrap();
    for f in a {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        let sc = with_blocks(true, || scene::scene(&g));
        let m = with_blocks(true, || flowmaid::quality::measure(&g, &sc));
        for d in m.details.iter().filter(|d| d.contains(&word)) { println!("{f}: {d}"); }
    }
}
