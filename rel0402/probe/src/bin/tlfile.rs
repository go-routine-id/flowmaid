use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::class;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let Ok(Document::Class(d)) = parse_document(&src) else { panic!() };
        let laid = class::scene(&d);
        let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        for who in 0..at.len() {
            for to in [(-150.0, -90.0), (-150.0, 60.0), (80.0, -90.0), (-60.0, -60.0)] {
                let mut p = at.clone();
                p[who] = to;
                let fr = class::route_partial(&d, &p, &laid, &at);
                let (ox, oy) = fr.scene.origin;
                for (j, e) in fr.scene.edges.iter().enumerate() {
                    if let Some(q) = e.waypoints.iter().find(|q| q.0 < ox - 1e-6 || q.1 < oy - 1e-6) {
                        println!("{f} box {who} ({}) to {to:?}: edge {j} {q:?} origin {:?}", fr.scene.nodes[who].id, fr.scene.origin);
                    }
                }
            }
        }
    }
}
