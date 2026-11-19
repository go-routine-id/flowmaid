//! Scratch probe (not committed): layering metrics per file.
use flowmaid::model::Direction;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("file\tw\th\taspect\talong\tedges\tframe_slack\tcross\tdefects");
    for path in &args {
        let src = std::fs::read_to_string(path).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() {
            flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
            _ => continue,
        };
        let sc = flowmaid::scene::scene(&g);
        let along = g
            .edges
            .iter()
            .filter(|e| e.from != e.to)
            .filter(|e| {
                let (a, b) = (&sc.nodes[e.from], &sc.nodes[e.to]);
                match g.direction {
                    Direction::LR => b.x > a.x + 1.0,
                    Direction::RL => b.x < a.x - 1.0,
                    Direction::TD => b.y > a.y + 1.0,
                    Direction::BT => b.y < a.y - 1.0,
                }
            })
            .count();
        let ne = g.edges.iter().filter(|e| e.from != e.to).count();
        // Frame slack: frame area / member bbox area (worst), member bbox
        // grown by 14 pad.
        let mut worst: f64 = 1.0;
        for c in &sc.clusters {
            let Some(si) = g.subgraphs.iter().position(|s| s.id == c.id) else {
                continue;
            };
            let mut members: Vec<usize> = Vec::new();
            let mut stack = vec![si];
            while let Some(s) = stack.pop() {
                members.extend(g.subgraphs[s].nodes.iter().copied());
                for (k, t) in g.subgraphs.iter().enumerate() {
                    if t.parent == Some(s) {
                        stack.push(k);
                    }
                }
            }
            if members.is_empty() {
                continue;
            }
            let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for &v in &members {
                let n = &sc.nodes[v];
                x0 = x0.min(n.x - n.w / 2.0);
                x1 = x1.max(n.x + n.w / 2.0);
                y0 = y0.min(n.y - n.h / 2.0);
                y1 = y1.max(n.y + n.h / 2.0);
            }
            let inner = (x1 - x0 + 60.0) * (y1 - y0 + 60.0);
            worst = worst.max(c.w * c.h / inner);
        }
        let m = flowmaid::quality::measure_source(&src, false)
            .map(|r| (r.statik.edge_edge_x, r.statik.defects()))
            .unwrap_or((0, 0));
        let name = std::path::Path::new(path)
            .file_stem()
            .unwrap()
            .to_string_lossy();
        println!(
            "{}\t{:.0}\t{:.0}\t{:.2}\t{}\t{}\t{:.2}\t{}\t{}",
            name,
            sc.width,
            sc.height,
            sc.width / sc.height,
            along,
            ne,
            worst,
            m.0,
            m.1
        );
    }
}
