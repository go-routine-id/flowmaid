//! Block mode: per file, the largest difference (px) ALONG THE FLOW between
//! scene()'s frames and route()'s frames at the scene's centres (TD/BT: top
//! and bottom; LR/RL: left and right), the largest across-flow difference,
//! whether route() cuts a title scene() draws whole, and whether the routed
//! frame is narrower than the scene's across the flow.
use flowmaid::model::{Direction, Document};
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        let bm = with_blocks(true, || flowmaid::quality::measure_source(&src, false)).map(|r| r.block_mode).unwrap_or(false);
        let sc = with_blocks(true, || scene::scene(&g));
        let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = with_blocks(true, || scene::route(&g, &c));
        let vertical = matches!(g.direction, Direction::TD | Direction::BT);
        let (mut along, mut across) = (0.0f64, 0.0f64);
        let mut who = String::new();
        for a in &sc.clusters {
            if let Some(b) = rt.clusters.iter().find(|b| b.id == a.id) {
                let (al, ac) = if vertical {
                    ((a.y - b.y).abs().max((a.y + a.h - b.y - b.h).abs()), (a.x - b.x).abs().max((a.x + a.w - b.x - b.w).abs()))
                } else {
                    ((a.x - b.x).abs().max((a.x + a.w - b.x - b.w).abs()), (a.y - b.y).abs().max((a.y + a.h - b.y - b.h).abs()))
                };
                if al > along { along = al; who = a.id.clone(); }
                across = across.max(ac);
            }
        }
        let qs = with_blocks(true, || flowmaid::quality::measure(&g, &sc));
        let qr = with_blocks(true, || flowmaid::quality::measure(&g, &rt));
        println!("{name}\tbm={}\talong={along:.1}\t{who}\tacross={across:.1}\ttcut_s={}\ttcut_r={}", u8::from(bm), qs.title_cut, qr.title_cut);
    }
}
