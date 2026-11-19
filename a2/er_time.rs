use flowmaid::model::Document;
use std::time::Instant;
fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let Document::Er(d) = flowmaid::parser::parse_document(&src).unwrap() else { panic!() };
    let t = Instant::now();
    let n = 200;
    let mut s = flowmaid::er::scene(&d);
    for _ in 0..n { s = flowmaid::er::scene(&d); }
    let scene_ms = t.elapsed().as_secs_f64() * 1e3 / n as f64;
    let mut pos: Vec<(f64, f64)> = s.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let t = Instant::now();
    let mut worst = 0.0f64;
    for k in 0..n {
        pos[0].0 += 3.0; pos[0].1 += if k % 2 == 0 { 2.0 } else { -1.0 };
        let t1 = Instant::now();
        let r = flowmaid::er::route(&d, &pos);
        std::hint::black_box(&r);
        worst = worst.max(t1.elapsed().as_secs_f64() * 1e3);
    }
    let route_ms = t.elapsed().as_secs_f64() * 1e3 / n as f64;
    println!("scene {scene_ms:.3} ms  route mean {route_ms:.3} ms  worst {worst:.3} ms");
}
