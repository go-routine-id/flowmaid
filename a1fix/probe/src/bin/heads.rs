use flowmaid::class::{card_spots, rel_head, route, scene, to_svg};
use flowmaid::model::Document;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let Ok(Document::Class(d)) = flowmaid::parser::parse_document(&src) else { continue };
        let cs = scene(&d);
        let svg = to_svg(&cs);
        let mut miss = 0;
        for j in 0..cs.rels.len() {
            let h = rel_head(&cs, j);
            if !h.polygon.is_empty() {
                let pts = h.polygon.iter().map(|(x, y)| format!("{:.1},{:.1}", x, y)).collect::<Vec<_>>().join(" ");
                if !svg.contains(&pts) { miss += 1; println!("  {f}: head {j} polygon not in svg"); }
            }
            for s in &h.segments {
                let p = format!("M {:.1} {:.1} L {:.1} {:.1}", s[0].0, s[0].1, s[1].0, s[1].1);
                if !svg.contains(&p) { miss += 1; println!("  {f}: head {j} seg not in svg"); }
            }
        }
        let spots = card_spots(&cs);
        let mut cmiss = 0;
        for (j, sp) in spots.iter().enumerate() {
            for (k, s) in sp.iter().enumerate() {
                if let Some((x, y)) = s {
                    let p = format!("x=\"{:.1}\" y=\"{:.1}\"", x, y);
                    if !svg.contains(&p) { cmiss += 1; println!("  {f}: card {j}/{k} at {x:.1},{y:.1} not in svg (clamped?)"); }
                }
            }
        }
        // full node list route == scene?
        let all: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&d, &all);
        let same_nodes = r.scene.nodes.iter().zip(&cs.scene.nodes).all(|(a, b)| (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9);
        println!("{f}: head_miss={miss} card_miss={cmiss} route(all) keeps nodes={same_nodes} notes={}", cs.notes.len());
    }
}
