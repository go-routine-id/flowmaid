// wp <file> [substr...]: waypoints of the static block scene's edges whose "from->to" contains a substr
use flowmaid::model::Document;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => panic!() };
    flowmaid::quality::with_blocks(true, || {
        let sc = flowmaid::scene::scene(&g);
        for e in &sc.edges {
            let k = format!("{}->{}", e.from, e.to);
            if a.len() > 2 && !a[2..].iter().any(|s| k.contains(s.as_str())) { continue; }
            println!("{k}: {}", e.waypoints.iter().map(|p| format!("({:.2},{:.2})", p.0, p.1)).collect::<Vec<_>>().join(" "));
        }
        for n in &sc.nodes { if a.len() > 2 && a[2..].iter().any(|s| n.id == *s) { println!("node {} {:.2} {:.2} {:.2} {:.2}", n.id, n.x, n.y, n.w, n.h); } }
    });
}
