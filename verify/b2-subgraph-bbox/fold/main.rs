fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for path in &args[1..] {
        let src = std::fs::read_to_string(path).unwrap();
        let g = match flowmaid::parser::parse_document(&src).unwrap() { flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g, _ => continue };
        let o = flowmaid::fold::CompactOptions::for_extent(400.0);
        let a = flowmaid::fold::render_compact(&g, &o);
        let b = flowmaid::fold::render_compact(&g, &o);
        assert_eq!(a, b, "fold not deterministic {path}");
        let name = path.rsplit('/').next().unwrap().trim_end_matches(".mmd");
        std::fs::write(format!("{}/{name}.fold.svg", args[0]), a).unwrap();
    }
    println!("ok");
}
