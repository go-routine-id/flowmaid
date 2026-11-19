use flowmaid::model::Document;
use flowmaid::scene::{route, scene};
fn main() {
    for src in ["stateDiagram-v2\n[*] --> A\nA --> B\nB --> C\nC --> [*]", "flowchart TD\nA-->B\nA-->C\nB-->D\nC-->D\nD-->|back|A"] {
        let g = match flowmaid::parser::parse_document(src).unwrap() { Document::State(g) | Document::Flowchart(g) => g, _ => panic!() };
        let sc = scene(&g);
        let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route(&g, &base);
        for (a, b) in sc.edges.iter().zip(&rt.edges) {
            println!("{}->{} same={} {:?} vs {:?}", a.from, a.to, a.bezier == b.bezier && a.waypoints == b.waypoints, a.bezier[0], b.bezier[0]);
        }
    }
}
