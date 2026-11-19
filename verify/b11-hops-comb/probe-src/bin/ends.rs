// ends <file> <node> [route] : every edge end on the node, with the first corners of its path
use flowmaid::model::Document;
use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let src = std::fs::read_to_string(&a[0]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!(),
    };
    let st = scene::scene(&g);
    let c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
    let sc = if a.get(2).map(String::as_str) == Some("route") { scene::route(&g, &c) } else { st };
    let n = sc.nodes.iter().find(|n| n.id == a[1]).unwrap();
    println!("node {} rect x {:.1}..{:.1} y {:.1}..{:.1}", n.id, n.x - n.w / 2.0, n.x + n.w / 2.0, n.y - n.h / 2.0, n.y + n.h / 2.0);
    for (i, e) in sc.edges.iter().enumerate() {
        if e.from != a[1] && e.to != a[1] {
            continue;
        }
        let w: Vec<(f64, f64)> = if e.waypoints.len() >= 2 { e.waypoints.clone() } else { e.bezier.to_vec() };
        let w: Vec<(f64, f64)> = if e.from == a[1] { w } else { w.into_iter().rev().collect() };
        let s: Vec<String> = w.iter().take(6).map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect();
        println!("  edge {i:2} {}->{} wps={} from-node: {}", e.from, e.to, e.waypoints.len(), s.join(" "));
    }
}
