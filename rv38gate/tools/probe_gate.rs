fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => panic!("not a flowchart/state"),
        };
        let sc = flowmaid::scene::scene(&g);
        let axes = |w: &[(f64, f64)]| {
            w.len() >= 3
                && w.windows(2).all(|q| (q[0].0 - q[1].0).abs() < 1e-6 || (q[0].1 - q[1].1).abs() < 1e-6)
        };
        let sq = sc.edges.iter().filter(|e| axes(&e.waypoints)).count();
        let band = |p: &str| {
            sc.nodes.iter().filter(|n| n.id.starts_with(p) && n.id[p.len()..].chars().all(|c| c.is_ascii_digit()))
                .map(|n| (n.y - n.h / 2.0, n.y + n.h / 2.0))
                .fold((f64::MAX, f64::MIN), |a, b| (a.0.min(b.0), a.1.max(b.1)))
        };
        let gap = band("t").0 - band("s").1;
        println!(
            "{}: g.edges={} sub_edges={} scene.edges={} on_axes={} st_row_gap={:.1} h={:.0}",
            path.rsplit('/').next().unwrap(), g.edges.len(), g.sub_edges.len(), sc.edges.len(), sq, gap, sc.height
        );
    }
}
