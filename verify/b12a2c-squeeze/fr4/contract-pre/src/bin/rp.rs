use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;
fn ms<T>(f: impl FnOnce() -> T) -> (T, f64) { let t = Instant::now(); let r = f(); (r, t.elapsed().as_secs_f64() * 1e3) }
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        flowmaid::quality::with_blocks(true, || {
            let mut sc_t = f64::INFINITY; let mut sc = None;
            for _ in 0..5 { let (s, t) = ms(|| scene::scene(&g)); sc_t = sc_t.min(t); sc = Some(s); }
            let sc = sc.unwrap();
            let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let (r, rt) = ms(|| scene::route(&g, &c));
            let n = g.nodes.len();
            let mut worst = 0.0f64; let mut wk = String::new(); let mut tot = 0.0; let mut cnt = 0;
            for k in 0..n { for d in [(1.0, 0.0), (60.0, 25.0), (-150.0, 80.0)] {
                let mut c2 = c.clone(); c2[k].0 += d.0; c2[k].1 += d.1;
                let mut best = f64::INFINITY;
                for _ in 0..3 { let (_p, t) = ms(|| scene::route_partial(&g, &c2, &r, &c)); best = best.min(t); }
                tot += best; cnt += 1;
                if best > worst { worst = best; wk = format!("node {k} {d:?}"); }
            } }
            println!("{name}: scene {sc_t:.2} route {rt:.2} route_partial worst {worst:.2} ({wk}) mean {:.2}", tot / cnt as f64);
        });
    }
}
