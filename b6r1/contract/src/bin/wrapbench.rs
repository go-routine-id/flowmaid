use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;
fn best<F: FnMut()>(n: usize, mut f: F) -> f64 {
    let mut b = f64::INFINITY;
    for _ in 0..n { let t = Instant::now(); f(); b = b.min(t.elapsed().as_secs_f64() * 1e3); }
    b
}
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&path).unwrap();
    let Document::Flowchart(g) = flowmaid::parser::parse_document(&src).unwrap() else { panic!() };
    let wrap = best(50, || { for n in &g.nodes { std::hint::black_box(flowmaid::layout::intrinsic_size_wrapped(n, g.wrapping_width(), flowmaid::layout::text_width)); } });
    let sc_ms = best(30, || { std::hint::black_box(scene::scene(&g)); });
    let sc = scene::scene(&g);
    let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let r_ms = best(30, || { std::hint::black_box(scene::route(&g, &c)); });
    let mut g2 = g.clone();
    g2.set_wrapping_width(None);
    let sc2_ms = best(30, || { std::hint::black_box(scene::scene(&g2)); });
    let sc2 = scene::scene(&g2);
    let c2: Vec<(f64, f64)> = sc2.nodes.iter().map(|n| (n.x, n.y)).collect();
    let r2_ms = best(30, || { std::hint::black_box(scene::route(&g2, &c2)); });
    println!("{path}: nodes={} wrap_all_nodes={wrap:.3}ms scene={sc_ms:.3} route={r_ms:.3} | nowrap scene={sc2_ms:.3} route={r2_ms:.3}", g.nodes.len());
}
