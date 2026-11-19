// el <file> [route] : every edge with its waypoints' slanted legs
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
    let sc = if a.get(1).map(String::as_str) == Some("route") { scene::route(&g, &c) } else { st };
    let full = a.iter().any(|x| x == "full");
    for (i, e) in sc.edges.iter().enumerate() {
        let w = &e.waypoints;
        let mut slant = Vec::new();
        for (k, s) in w.windows(2).enumerate() {
            if (s[0].0 - s[1].0).abs() >= 1e-6 && (s[0].1 - s[1].1).abs() >= 1e-6 {
                slant.push(format!("leg {k}/{}: ({:.2},{:.2})->({:.2},{:.2})", w.len() - 1, s[0].0, s[0].1, s[1].0, s[1].1));
            }
        }
        println!("edge {i:3} {}->{} wps={} curve={:?} label={:?} slanted={:?}", e.from, e.to, w.len(), e.meta.curve, e.label.as_ref().map(|l| (&l.0, l.1, l.2)), slant);
        if full {
            println!("    bez {:?}\n    wps {:?}", e.bezier, w);
        }
    }
}
