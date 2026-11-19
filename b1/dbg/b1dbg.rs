// Temporary debugging helper (not committed): dump edge geometry.
use flowmaid::Document;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let want: Vec<&str> = a[3..].iter().map(|s| s.as_str()).collect();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!(),
    };
    let st = flowmaid::scene::scene(&g);
    let sc = if a[2] == "routed" {
        let c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        flowmaid::scene::route(&g, &c)
    } else { st };
    for e in &sc.edges {
        let key = format!("{}->{}", e.from, e.to);
        if !want.is_empty() && !want.contains(&key.as_str()) { continue; }
        let n = sc.nodes.iter().find(|n| n.id == e.to);
        println!("{key} bez={:?}", e.bezier.map(|p| ((p.0*10.0).round()/10.0, (p.1*10.0).round()/10.0)));
        let w = &e.waypoints;
        if !w.is_empty() {
            let k = w.len().saturating_sub(6);
            println!("   wps(n={}) tail={:?}", w.len(), w[k..].iter().map(|p| ((p.0*10.0).round()/10.0, (p.1*10.0).round()/10.0)).collect::<Vec<_>>());
        }
        if let Some(n) = n { println!("   to box x {:.1}..{:.1} y {:.1}..{:.1}", n.x-n.w/2.0, n.x+n.w/2.0, n.y-n.h/2.0, n.y+n.h/2.0); }
    }
}
