// dragq <off|on> <files...>: summed quality of route_partial drag frames (from the static scene),
// per file and drag kind; WHO=1 prints each drag's numbers.
use flowmaid::model::Document;
use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let on = a[1] == "on";
    let want = ["defects", "x_drawn", "edge_node_x", "edge_frame_pairs", "lateral_x", "fan_x", "edge_chip", "overlap_pairs"];
    let who = std::env::var("WHO").is_ok();
    for f in &a[2..] {
        let src = std::fs::read_to_string(f).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => continue };
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        flowmaid::quality::with_blocks(on, || {
            let st = scene::scene(&g);
            let at: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            for (dname, d) in [("1px", (1.0, 0.0)), ("mid", (60.0, 25.0)), ("far", (-150.0, 80.0))] {
                let mut sums = vec![0.0; want.len()];
                for v in 0..g.nodes.len() {
                    let mut c = at.clone();
                    c[v] = (c[v].0 + d.0, c[v].1 + d.1);
                    let p = scene::route_partial(&g, &c, &st, &at);
                    let m = flowmaid::quality::measure(&g, &p);
                    let mut fl = m.fields(); fl.extend(m.late_fields()); fl.extend(m.tail_fields());
                    let mut row = vec![0.0; want.len()];
                    row[0] = m.defects() as f64;
                    for (k, w) in want.iter().enumerate().skip(1) {
                        if let Some(x) = fl.iter().find(|x| x.0 == *w) { row[k] = x.1; }
                    }
                    if who { println!("  {name} {dname} {} {}", g.nodes[v].id, want.iter().zip(&row).map(|(w, x)| format!("{w}={x}")).collect::<Vec<_>>().join(" ")); }
                    for k in 0..want.len() { sums[k] += row[k]; }
                }
                let s: Vec<String> = want.iter().zip(&sums).map(|(w, x)| format!("{w}={x}")).collect();
                println!("{name}\t{}\t{dname}\t{}", if on {"on"} else {"off"}, s.join("\t"));
            }
        });
    }
}
