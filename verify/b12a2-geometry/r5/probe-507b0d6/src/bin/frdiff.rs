//! Block mode: largest difference (px) between scene()'s frames and route()'s frames at the scene's centres,
//! per file: max over clusters of |x|,|y|,|w|,|h| differences; and whether a routed title is cut.
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        let sc = with_blocks(true, || scene::scene(&g));
        let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = with_blocks(true, || scene::route(&g, &c));
        let mut worst = 0.0f64; let mut who = String::new();
        for a in &sc.clusters {
            if let Some(b) = rt.clusters.iter().find(|b| b.id == a.id) {
                let d = [(a.x - b.x).abs(), (a.y - b.y).abs(), (a.w - b.w).abs(), (a.h - b.h).abs()];
                let m = d.iter().cloned().fold(0.0, f64::max);
                if m > worst { worst = m; who = format!("{} dx={:.1} dy={:.1} dw={:.1} dh={:.1}", a.id, d[0], d[1], d[2], d[3]); }
            }
        }
        let svg_s = scene::to_svg(&sc); let svg_r = scene::to_svg(&rt);
        let cut_s = svg_s.matches('…').count(); let cut_r = svg_r.matches('…').count();
        println!("{name}\tfr_max={worst:.1}\t{who}\tcut_scene={cut_s}\tcut_route={cut_r}");
    }
}
