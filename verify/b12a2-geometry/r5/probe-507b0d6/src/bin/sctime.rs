//! Default build: min-of-N wall time (ms) of scene() and of route() at the
//! scene's centres, per file. Usage: sctime <n> <files...>
use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;
fn main() {
    let mut a = std::env::args().skip(1);
    let n: usize = a.next().unwrap().parse().unwrap();
    for f in a {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => continue,
        };
        let (mut bs, mut br) = (f64::MAX, f64::MAX);
        let sc = scene::scene(&g);
        let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for _ in 0..n {
            let t = Instant::now();
            let s = scene::scene(&g);
            std::hint::black_box(&s);
            bs = bs.min(t.elapsed().as_secs_f64() * 1e3);
            let t = Instant::now();
            let r = scene::route(&g, &c);
            std::hint::black_box(&r);
            br = br.min(t.elapsed().as_secs_f64() * 1e3);
        }
        println!("{}\tscene_min={:.3}\troute_min={:.3}", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), bs, br);
    }
}
