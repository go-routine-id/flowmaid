// route_partial vs route() over the quality harness's drag frames (same frames, same timing method).
use flowmaid::{parser, quality, scene};
fn main() {
    let dir = std::env::args().nth(1).expect("dir");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path())
        .filter(|p| p.extension().map_or(false, |e| e == "mmd")).collect();
    files.sort();
    println!("file\tframes\tpartial_worst\tpartial_mean\troute_worst\troute_mean");
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match parser::parse(&src) { Ok(g) => g, Err(_) => continue };
        let st = scene::scene(&g);
        let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let frames = quality::drag_frames(&st);
        let (mut pw, mut ps, mut rw, mut rs) = (0f64, 0f64, 0f64, 0f64);
        for (_, c) in &frames {
            let p = quality::fastest_ms(quality::DRAG_RUNS, || scene::route_partial(&g, c, &st, &base));
            let r = quality::fastest_ms(quality::DRAG_RUNS, || scene::route(&g, c));
            pw = pw.max(p); ps += p; rw = rw.max(r); rs += r;
        }
        let n = frames.len().max(1) as f64;
        println!("{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}", f.file_stem().unwrap().to_string_lossy(), frames.len(), pw, ps / n, rw, rs / n);
    }
}
