use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let g = flowmaid::parser::parse(&std::fs::read_to_string(&a[0]).unwrap()).unwrap();
    std::env::remove_var("COMBDBG");
    let st = scene::scene(&g);
    let c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
    if a.get(1).map(String::as_str) == Some("static") {
        std::env::set_var("COMBDBG", "1");
        let _ = scene::scene(&g);
    } else {
        std::env::set_var("COMBDBG", "1");
        let _ = scene::route(&g, &c);
    }
}
