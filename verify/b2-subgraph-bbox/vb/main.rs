fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = &args[0];
    for path in &args[1..] {
        let src = std::fs::read_to_string(path).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() { flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g, _ => continue };
        let s = flowmaid::scene::scene(&g);
        let pos: Vec<(f64,f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = flowmaid::scene::route(&g, &pos);
        let name = path.rsplit('/').next().unwrap().trim_end_matches(".mmd");
        std::fs::write(format!("{out}/{name}.route.svg"), flowmaid::scene::to_svg(&r)).unwrap();
        // a drag: move the middle node by (+40,+25)
        let mut m = pos.clone(); let k = m.len()/2; m[k].0 += 40.0; m[k].1 += 25.0;
        let d = flowmaid::scene::route(&g, &m);
        std::fs::write(format!("{out}/{name}.drag.svg"), flowmaid::scene::to_svg(&d)).unwrap();
    }
}
