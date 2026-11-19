use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::scene;
fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let g = match parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => continue };
        flowmaid::quality::with_blocks(true, || {
            let sc = scene::scene(&g);
            let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = scene::route(&g, &c);
            println!("== {path}");
            for a in &sc.clusters {
                if let Some(b) = r.clusters.iter().find(|b| b.id == a.id) {
                    let d = [(b.x - a.x), (b.y - a.y), (b.x + b.w - a.x - a.w), (b.y + b.h - a.y - a.h)];
                    let m = d.iter().fold(0.0f64, |m, v| m.max(v.abs()));
                    if m > 0.5 {
                        println!("  {:<20} scene ({:.1},{:.1},{:.1}x{:.1}) route ({:.1},{:.1},{:.1}x{:.1}) d L{:.1} T{:.1} R{:.1} B{:.1}", a.id, a.x, a.y, a.w, a.h, b.x, b.y, b.w, b.h, d[0], d[1], d[2], d[3]);
                        // which members, and which scene edges run inside the scene frame but outside the member box
                        let (x0, x1) = (a.x, a.x + a.w);
                        let (y0, y1) = (a.y, a.y + a.h);
                        for (i, e) in sc.edges.iter().enumerate() {
                            let inside = e.waypoints.iter().filter(|p| p.0 > x0 && p.0 < x1 && p.1 > y0 && p.1 < y1).count();
                            let rin = r.edges[i].waypoints.iter().filter(|p| p.0 > x0 && p.0 < x1 && p.1 > y0 && p.1 < y1).count();
                            if inside > 0 && (d[0].abs() > 0.5 || d[2].abs() > 0.5) {
                                let xs: Vec<f64> = e.waypoints.iter().filter(|p| p.1 > y0 && p.1 < y1 && p.0 > x0 && p.0 < x1).map(|p| (p.0 * 10.0).round() / 10.0).collect();
                                let mn = xs.iter().cloned().fold(f64::INFINITY, f64::min);
                                let mx = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                                println!("     scene edge {i} {}->{}: {inside} pts inside, x {mn:.1}..{mx:.1}; route pts inside {rin}", e.from, e.to);
                            }
                        }
                        for n in &sc.nodes {
                            if n.x > x0 && n.x < x1 && n.y > y0 && n.y < y1 { println!("     member {} x {:.1}..{:.1}", n.id, n.x - n.w / 2.0, n.x + n.w / 2.0); }
                        }
                    }
                }
            }
        });
    }
}
