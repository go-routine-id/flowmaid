// scenebench <off|on> reps files...: fastest scene() / route() of each file, ms.
use flowmaid::model::{Document, Graph};
fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let on = a[1] == "on";
    let reps: usize = a[2].parse().unwrap();
    for f in &a[3..] {
        let Some(g) = graph_of(&std::fs::read_to_string(f).unwrap()) else { continue };
        let (s, r) = flowmaid::quality::with_blocks(on, || {
            let mut best_s = f64::INFINITY;
            let mut best_r = f64::INFINITY;
            let mut at = Vec::new();
            for _ in 0..reps {
                let t = std::time::Instant::now();
                let sc = flowmaid::scene::scene(&g);
                best_s = best_s.min(t.elapsed().as_secs_f64() * 1e3);
                at = sc.nodes.iter().map(|n| (n.x, n.y)).collect::<Vec<_>>();
            }
            for _ in 0..reps {
                let t = std::time::Instant::now();
                let _ = flowmaid::scene::route(&g, &at);
                best_r = best_r.min(t.elapsed().as_secs_f64() * 1e3);
            }
            (best_s, best_r)
        });
        println!("{}\tscene {:.3}\troute {:.3}", std::path::Path::new(f).file_stem().unwrap().to_string_lossy(), s, r);
    }
}
