//! route() per drag frame on one flowchart (as examples/quality --drag): sum of fastest-of-3 per frame.
use flowmaid::model::Document;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let reps: usize = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(5);
    let src = std::fs::read_to_string(&path).unwrap();
    let Ok(Document::Flowchart(g)) = flowmaid::parser::parse_document(&src) else { panic!() };
    let sc = flowmaid::scene::scene(&g);
    let frames = flowmaid::quality::drag_frames(&sc);
    for _ in 0..reps {
        let mut sum = 0.0;
        for (_, c) in &frames {
            sum += flowmaid::quality::fastest_ms(3, || flowmaid::scene::route(&g, c));
        }
        print!("{:.2} ", sum / frames.len() as f64);
    }
    println!();
}
