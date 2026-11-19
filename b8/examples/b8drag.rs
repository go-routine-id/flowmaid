//! Scratch (not committed): robust drag benchmark — per frame min over
//! R repeats, then worst and mean frame.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let reps: usize = std::env::var("REPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    for path in &args {
        let src = std::fs::read_to_string(path).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => continue,
        };
        let sc = flowmaid::scene::scene(&g);
        let frames = flowmaid::quality::drag_frames(&sc);
        let mut best = vec![f64::MAX; frames.len()];
        for _ in 0..reps {
            for (k, (_, c)) in frames.iter().enumerate() {
                let t = std::time::Instant::now();
                std::hint::black_box(flowmaid::scene::route(&g, c));
                best[k] = best[k].min(t.elapsed().as_secs_f64() * 1e3);
            }
        }
        let worst = best.iter().cloned().fold(0.0, f64::max);
        if std::env::var_os("SHOWWORST").is_some() {
            let mut idx: Vec<usize> = (0..best.len()).collect();
            idx.sort_by(|&a, &b| best[b].total_cmp(&best[a]));
            for &k in idx.iter().take(5) {
                let (v, c) = &frames[k];
                println!(
                    "  frame {k} node {} at ({:.0},{:.0}) {:.2} ms",
                    g.nodes[*v].id, c[*v].0, c[*v].1, best[k]
                );
            }
        }
        let mean = best.iter().sum::<f64>() / best.len() as f64;
        let centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut rbest = f64::MAX;
        for _ in 0..10 {
            let t = std::time::Instant::now();
            std::hint::black_box(flowmaid::scene::route(&g, &centers));
            rbest = rbest.min(t.elapsed().as_secs_f64() * 1e3);
        }
        let name = std::path::Path::new(path)
            .file_stem()
            .unwrap()
            .to_string_lossy();
        println!(
            "{name}\troute {rbest:.2}\tdrag_worst {worst:.2}\tdrag_mean {mean:.2}\tframes {}",
            frames.len()
        );
    }
}
