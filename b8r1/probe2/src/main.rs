fn main() {
    let f = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&f).unwrap();
    let g = flowmaid::parser::parse(&src).unwrap();
    let t = std::time::Instant::now();
    let lo = flowmaid::layout::layout(&g);
    println!("layout {:.0} ms, {} nodes", t.elapsed().as_secs_f64() * 1000.0, lo.nodes.len());
}
