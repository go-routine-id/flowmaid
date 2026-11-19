use flowmaid::model::Document;
use flowmaid::{parser, scene};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let spread: f64 = a[2].parse().unwrap();
    let Ok(Document::Flowchart(g) | Document::State(g)) = parser::parse_document(&src) else { return };
    let c: Vec<(f64, f64)> = (0..g.nodes.len()).map(|i| (spread * i as f64, spread * (i % 3) as f64)).collect();
    let t = std::time::Instant::now();
    let s = scene::route(&g, &c);
    println!("ok spread={spread} edges={} {:?}", s.edges.len(), t.elapsed());
}
