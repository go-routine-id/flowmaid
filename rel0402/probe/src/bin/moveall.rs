// Every box of each class file moved by (-3, +260) from the laid-out
// scene: does the route_partial frame (and route) hold every line
// waypoint on its canvas?
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::scene::Scene;
use flowmaid::class;
fn off(sc: &Scene) -> usize {
    let (ox, oy) = sc.origin;
    let (fx, fy) = (ox + sc.width, oy + sc.height);
    sc.edges.iter().flat_map(|e| e.waypoints.iter()).filter(|p| p.0 < ox - 1e-6 || p.1 < oy - 1e-6 || p.0 > fx + 1e-6 || p.1 > fy + 1e-6).count()
}
fn main() {
    let (mut frames, mut fbad, mut rbad) = (0, 0, 0);
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let Ok(Document::Class(d)) = parse_document(&src) else { continue };
        let laid = class::scene(&d);
        let at: Vec<_> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        for i in 0..d.classes.len().min(12) {
            let mut to = at.clone();
            to[i] = (to[i].0 - 3.0, to[i].1 + 260.0);
            let fr = class::route_partial(&d, &to, &laid, &at);
            let r = class::route(&d, &to);
            frames += 1;
            let (a, b) = (off(&fr.scene), off(&r.scene));
            if a > 0 { fbad += 1; println!("FRAME {f} box {i}: {a} waypoints off"); }
            if b > 0 { rbad += 1; println!("ROUTE {f} box {i}: {b} waypoints off (origin {:?})", r.scene.origin); }
        }
    }
    println!("frames {frames}: route_partial with lines off its canvas {fbad}, route {rbad}");
}
