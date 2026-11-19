use flowmaid::model::Document;
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&f).unwrap();
    let Ok(Document::Class(d)) = flowmaid::parser::parse_document(&src) else { return };
    let cs = flowmaid::class::scene(&d);
    for n in &cs.scene.nodes {
        println!("node {} x {:.1}..{:.1} y {:.1}..{:.1}", n.id, n.x - n.w / 2.0, n.x + n.w / 2.0, n.y - n.h / 2.0, n.y + n.h / 2.0);
    }
    for e in cs.scene.edges.iter().filter(|e| e.from == e.to) {
        let b = e.bezier;
        let pts: Vec<(f64, f64)> = (0..=64).map(|k| { let t = k as f64 / 64.0; let u = 1.0 - t; let w = [u*u*u, 3.0*u*u*t, 3.0*u*t*t, t*t*t];
            (w[0]*b[0].0+w[1]*b[1].0+w[2]*b[2].0+w[3]*b[3].0, w[0]*b[0].1+w[1]*b[1].1+w[2]*b[2].1+w[3]*b[3].1) }).collect();
        let (x0, x1) = pts.iter().fold((f64::MAX, f64::MIN), |r, p| (r.0.min(p.0), r.1.max(p.0)));
        let (y0, y1) = pts.iter().fold((f64::MAX, f64::MIN), |r, p| (r.0.min(p.1), r.1.max(p.1)));
        println!("loop {} wp={} x {:.1}..{:.1} y {:.1}..{:.1} label {:?}", e.from, e.waypoints.len(), x0, x1, y0, y1,
            e.label.as_ref().map(|(t, m, w)| (t.clone(), (m.0 - w / 2.0, m.0 + w / 2.0), m.1)));
    }
}
