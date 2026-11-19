//! pick <file.mmd> <edge> <x0> <y0> <x1> <y1>: hits_in_rect and the edge's waypoints near the box (debug, scratch only)
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        flowmaid::Document::Flowchart(g) | flowmaid::Document::State(g) => g,
        _ => panic!(),
    };
    let sc = flowmaid::scene::scene(&g);
    let i: usize = a[2].parse().unwrap();
    let r: Vec<f64> = a[3..7].iter().map(|s| s.parse().unwrap()).collect();
    let e = &sc.edges[i];
    println!("edge {} {}->{} hits {:?}", i, e.from, e.to, sc.hits_in_rect(r[0], r[1], r[2], r[3]));
    for (k, p) in e.waypoints.iter().enumerate() {
        if (p.0 - r[0]).abs() < 40.0 && (p.1 - r[1]).abs() < 40.0 { println!("  wp {k} {:.3} {:.3}", p.0, p.1); }
    }
}
