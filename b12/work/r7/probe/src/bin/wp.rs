//! wp <file.mmd> <edge index>: raw waypoints and bezier of one scene edge (debug, scratch only)
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
        _ => panic!(),
    };
    let sc = flowmaid::scene::scene(&g);
    let e = &sc.edges[a[2].parse::<usize>().unwrap()];
    println!("{}->{} bezier {:?}", e.from, e.to, e.bezier);
    for p in &e.waypoints { println!("{:.3} {:.3}", p.0, p.1); }
}
