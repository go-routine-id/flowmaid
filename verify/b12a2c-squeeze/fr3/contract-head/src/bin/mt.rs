use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        flowmaid::quality::with_blocks(true, || {
            let sc = scene::scene(&g);
            let mut best = f64::INFINITY;
            for _ in 0..5 { let t = Instant::now(); let _m = flowmaid::quality::measure(&g, &sc); best = best.min(t.elapsed().as_secs_f64()*1e3); }
            let mut sb = f64::INFINITY;
            for _ in 0..5 { let t = Instant::now(); let _s = scene::scene(&g); sb = sb.min(t.elapsed().as_secs_f64()*1e3); }
            println!("{name}: edges {} scene {sb:.2} ms measure {best:.2} ms", g.edges.len());
        });
    }
}
