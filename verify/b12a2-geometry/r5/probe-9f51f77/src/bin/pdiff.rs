//! Block mode: a 1 px drag of node 0 through route_partial: largest frame change (px) against scene().
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        let sc = with_blocks(true, || scene::scene(&g));
        let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut worst = (0.0f64, String::new());
        for v in 0..base.len() {
            let mut pos = base.clone(); pos[v].0 += 1.0;
            let p = with_blocks(true, || scene::route_partial(&g, &pos, &sc, &base));
            for a in &sc.clusters { if let Some(b) = p.clusters.iter().find(|b| b.id == a.id) {
                let m = (a.w - b.w).abs().max((a.x - b.x).abs()).max((a.h - b.h).abs());
                if m > worst.0 { worst = (m, format!("{} w {:.1}->{:.1} (drag {})", a.id, a.w, b.w, sc.nodes[v].id)); } } }
        }
        println!("{name}\tpartial_1px_fr={:.1}\t{}", worst.0, worst.1);
    }
}
