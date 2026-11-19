// coldwarm <files...>: block mode; route() at the scene's centres warm (after scene() on the thread) and cold
// (on a fresh thread, no memo); route_partial and route() of a 1 px drag; fastest of N runs, ms.
use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;
fn fastest<T>(n: usize, mut f: impl FnMut() -> T) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..n { let t = Instant::now(); std::hint::black_box(f()); best = best.min(t.elapsed().as_secs_f64() * 1e3); }
    best
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    for f in &a[1..] {
        let src = std::fs::read_to_string(f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => continue };
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let (scene_ms, warm, drag_route, drag_partial, at) = flowmaid::quality::with_blocks(true, || {
            let s = fastest(5, || scene::scene(&g));
            let sc = scene::scene(&g);
            let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let warm = fastest(10, || scene::route(&g, &at));
            let mut c = at.clone(); c[0].0 += 1.0;
            let dr = fastest(5, || scene::route(&g, &c));
            let dp = fastest(5, || scene::route_partial(&g, &c, &sc, &at));
            (s, warm, dr, dp, at)
        });
        // Cold: each run on a fresh thread (no memo), the scene not laid out there before.
        let mut cold = f64::INFINITY;
        for _ in 0..5 {
            let g2 = g.clone(); let at2 = at.clone();
            let ms = std::thread::spawn(move || flowmaid::quality::with_blocks(true, || { let t = Instant::now(); std::hint::black_box(scene::route(&g2, &at2)); t.elapsed().as_secs_f64() * 1e3 })).join().unwrap();
            cold = cold.min(ms);
        }
        println!("{name}\tscene {scene_ms:.2}\troute warm {warm:.3}\troute cold {cold:.2}\t1px drag: route {drag_route:.2} route_partial {drag_partial:.2}");
    }
}
