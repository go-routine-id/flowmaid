//! Block mode: scene() frames vs route() frames at the scene's centres, split along / across the flow.
//! Per file: along_max (TD/BT: y, h; LR/RL: x, w), across_max (the other two), the frame of each, and title cuts.
use flowmaid::model::{Direction, Document};
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        if g.subgraphs.is_empty() { continue; }
        let sc = with_blocks(true, || scene::scene(&g));
        let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = with_blocks(true, || scene::route(&g, &c));
        let vert = matches!(g.direction, Direction::TD | Direction::BT);
        let (mut al, mut ac) = ((0.0f64, String::new()), (0.0f64, String::new()));
        for a in &sc.clusters {
            if let Some(b) = rt.clusters.iter().find(|b| b.id == a.id) {
                let (dx, dy, dw, dh) = ((a.x - b.x).abs(), (a.y - b.y).abs(), (a.w - b.w).abs(), (a.h - b.h).abs());
                let (along, across) = if vert { (dy.max((a.y + a.h - b.y - b.h).abs()), dx.max((a.x + a.w - b.x - b.w).abs())) } else { (dx.max((a.x + a.w - b.x - b.w).abs()), dy.max((a.y + a.h - b.y - b.h).abs())) };
                let _ = (dw, dh);
                if along > al.0 { al = (along, format!("{} s=({:.1},{:.1},{:.1},{:.1}) r=({:.1},{:.1},{:.1},{:.1})", a.id, a.x, a.y, a.w, a.h, b.x, b.y, b.w, b.h)); }
                if across > ac.0 { ac = (across, a.id.clone()); }
            }
        }
        let cut_s = scene::to_svg(&sc).matches('…').count(); let cut_r = scene::to_svg(&rt).matches('…').count();
        println!("{name}\talong={:.1}\tacross={:.1}\tcut={}/{}\t{}\t{}", al.0, ac.0, cut_s, cut_r, al.1, ac.1);
    }
}
