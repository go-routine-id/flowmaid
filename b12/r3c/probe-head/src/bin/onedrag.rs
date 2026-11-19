use flowmaid::model::Document;
use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => panic!() };
    let (dx, dy): (f64, f64) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    flowmaid::quality::with_blocks(true, || {
        let st = scene::scene(&g);
        let at: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let v = g.node_index(&a[2]).unwrap();
        let mut c = at.clone(); c[v] = (c[v].0 + dx, c[v].1 + dy);
        for _ in 0..3 { let t = std::time::Instant::now(); let _ = scene::route_partial(&g, &c, &st, &at); eprintln!("total {:.2}", t.elapsed().as_secs_f64()*1e3); }
    });
}
