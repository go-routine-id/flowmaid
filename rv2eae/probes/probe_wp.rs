fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => continue,
        };
        let sc = flowmaid::scene::scene(&g);
        let wp = sc.edges.iter().filter(|e| !e.waypoints.is_empty()).count();
        let bad_ends = sc.edges.iter().filter(|e| !e.waypoints.is_empty()).filter(|e| {
            let a = e.waypoints[0]; let b = *e.waypoints.last().unwrap();
            let d = |p: (f64,f64), q: (f64,f64)| ((p.0-q.0).powi(2)+(p.1-q.1).powi(2)).sqrt();
            d(a, e.bezier[0]) > 0.5 || d(b, e.bezier[3]) > 0.5
        }).count();
        println!("{path}: edges={} with_waypoints={} bezier_ends_off={} h={:.0} w={:.0}", sc.edges.len(), wp, bad_ends, sc.height, sc.width);
    }
}
