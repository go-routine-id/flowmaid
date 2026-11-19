//! Write the block-mode (switch forced on) SVG of each input, static and
//! routed at the scene's centres, into <outdir>.
//! Usage: onsvg <outdir> <files...>
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    let mut a = std::env::args().skip(1);
    let out = a.next().unwrap();
    std::fs::create_dir_all(&out).unwrap();
    for f in a {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        let svg = with_blocks(true, || flowmaid::render_svg(&src));
        match svg {
            Ok(s) => std::fs::write(format!("{out}/{name}.svg"), s).unwrap(),
            Err(e) => eprintln!("{name}: {e}"),
        }
        if let Ok(Document::Flowchart(g)) | Ok(Document::State(g)) = flowmaid::parser::parse_document(&src) {
            let sc = with_blocks(true, || scene::scene(&g));
            let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = with_blocks(true, || scene::route(&g, &c));
            std::fs::write(format!("{out}/{name}.routed.svg"), scene::to_svg(&rt)).unwrap();
        }
    }
}
