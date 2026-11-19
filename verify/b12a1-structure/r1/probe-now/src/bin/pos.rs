//! pos <file>: node positions and layer order with blocks off and on.
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => continue,
        };
        for on in [false, true] {
            let sc = with_blocks(on, || flowmaid::scene::scene(&g));
            let horizontal = matches!(g.direction, flowmaid::model::Direction::LR | flowmaid::model::Direction::RL);
            // rows along the flow, members by position across it
            let mut v: Vec<(i64, i64, String)> = sc.nodes.iter().map(|n| {
                let (a, b) = if horizontal { (n.x, n.y) } else { (n.y, n.x) };
                (a.round() as i64, b.round() as i64, n.id.clone())
            }).collect();
            v.sort();
            let mut rows: Vec<(i64, Vec<String>)> = Vec::new();
            for (a, _, id) in v {
                match rows.last_mut() {
                    Some((ra, ids)) if (a - *ra).abs() <= 6 => ids.push(id),
                    _ => rows.push((a, vec![id])),
                }
            }
            let rep = with_blocks(on, || flowmaid::quality::measure_source(&src, false)).unwrap();
            println!("{} blocks={} canvas {:.0}x{:.0} x_drawn={} rows: {}", std::path::Path::new(&f).file_name().unwrap().to_string_lossy(), u8::from(on), sc.width, sc.height, rep.statik.x_drawn,
                rows.iter().map(|r| r.1.join(" ")).collect::<Vec<_>>().join(" / "));
        }
    }
}
