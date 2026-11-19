use flowmaid::{class, parser, Document};
use std::time::Instant;
fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let Document::Class(d) = parser::parse_document(&src).unwrap() else { panic!() };
        let t = Instant::now();
        let sc = class::scene(&d);
        let scene_ms = t.elapsed().as_secs_f64() * 1e3;
        let mut pos: Vec<(f64, f64)> = sc.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let (mut wr, mut ws) = (0.0f64, 0.0f64);
        let (mut sr, mut ss) = (0.0f64, 0.0f64);
        for k in 0..40 {
            pos[k % d.classes.len()].0 += 13.0;
            let t = Instant::now();
            let r = class::route(&d, &pos);
            let a = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let _ = class::to_svg(&r);
            let b = t.elapsed().as_secs_f64() * 1e3;
            wr = wr.max(a); ws = ws.max(b); sr += a; ss += b;
        }
        println!("{path}: scene {scene_ms:.2} ms, route worst {wr:.2} avg {:.2}, svg worst {ws:.2} avg {:.2}", sr/40.0, ss/40.0);
    }
}
