use std::time::Instant;
fn fastest<F: FnMut()>(n: usize, mut f: F) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..n { let t = Instant::now(); f(); best = best.min(t.elapsed().as_secs_f64() * 1e3); }
    best
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let runs: usize = args[0].parse().unwrap();
    for path in &args[1..] {
        let src = std::fs::read_to_string(path).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => continue,
        };
        let sc = flowmaid::scene::scene(&g);
        let frames = flowmaid::quality::drag_frames(&sc);
        let mut worst = 0.0f64; let mut sum = 0.0; let mut wi = 0;
        for (k, (_, c)) in frames.iter().enumerate() {
            let ms = fastest(runs, || { std::hint::black_box(flowmaid::scene::route(&g, c)); });
            if ms > worst { worst = ms; wi = k; }
            sum += ms;
        }
        // route_partial: base = route at static centres, drag the first frame's node
        let centres: Vec<(f64,f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let base = flowmaid::scene::route(&g, &centres);
        let mut pw = 0.0f64;
        for (_, c) in frames.iter().take(40) {
            let ms = fastest(runs, || { std::hint::black_box(flowmaid::scene::route_partial(&g, c, &base, &centres)); });
            pw = pw.max(ms);
        }
        let ss = fastest(runs.min(10), || { std::hint::black_box(flowmaid::scene::scene(&g)); });
        println!("{}\tframes={}\tworst={:.3} (frame {})\tmean={:.3}\tpartial_worst={:.3}\tscene={:.3}", path.rsplit('/').next().unwrap(), frames.len(), worst, wi, sum / frames.len() as f64, pw, ss);
    }
}
