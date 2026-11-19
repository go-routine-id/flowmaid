use std::io::BufRead;
fn main() {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let doc = match flowmaid::parser::parse_document(&src) { Ok(d) => d, Err(_) => continue };
        let g = match doc {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => continue,
        };
        eprintln!("FILE {path}");
        let r = std::panic::catch_unwind(|| flowmaid::scene::scene(&g).edges.len());
        if r.is_err() { eprintln!("PANIC {path}"); }
    }
}
