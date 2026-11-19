// scratch debug (not committed)
use flowmaid::scene::{route, scene};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(path).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        flowmaid::model::Document::Flowchart(g) => g,
        flowmaid::model::Document::State(g) => g,
        _ => panic!(),
    };
    let s = scene(&g);
    let routed = std::env::args().nth(2).is_some();
    let sc = if routed {
        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        route(&g, &c)
    } else {
        s
    };
    println!("canvas {} x {}", sc.width, sc.height);
    for n in &sc.nodes {
        println!(
            "node {} c=({:.1},{:.1}) {:.1}x{:.1}",
            n.id, n.x, n.y, n.w, n.h
        );
    }
    for e in &sc.edges {
        println!(
            "edge {}->{} bez={:?} wps={:?} label={:?}",
            e.from,
            e.to,
            e.bezier
                .map(|p| ((p.0 * 10.0).round() / 10.0, (p.1 * 10.0).round() / 10.0)),
            e.waypoints
                .iter()
                .map(|p| ((p.0 * 10.0).round() / 10.0, (p.1 * 10.0).round() / 10.0))
                .collect::<Vec<_>>(),
            e.label.as_ref().map(|l| (l.1, l.2))
        );
    }
}
