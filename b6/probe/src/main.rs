use flowmaid::{parser::parse_document, Document, scene, quality};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&args[1]).unwrap();
    let (from, to) = (args[2].clone(), args[3].clone());
    let g = match parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => panic!() };
    let st = scene::scene(&g);
    let at: Vec<(f64,f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
    let rt = scene::route(&g, &at);
    let show = |w: &str, sc: &scene::Scene| { for e in &sc.edges { if e.from == from && e.to == to { println!("{w} bez={:?} wps={:?}", e.bezier, e.waypoints); } } };
    show("static", &st); show("routed", &rt);
    for n in &rt.nodes { if n.id == to || n.id == from { println!("node {} {:?} {} {} {} {}", n.id, n.shape, n.x, n.y, n.w, n.h); } }
    let carried = std::env::var("CARRY").is_ok();
    let (mut prev, mut base) = (at.clone(), rt.clone());
    for (k, (v, c)) in quality::drag_frames(&rt).into_iter().enumerate() {
        let sc = if carried { scene::route_partial(&g, &c, &base, &prev) } else { scene::route_partial(&g, &c, &rt, &at) };
        let pb = base.clone();
        if carried { prev = c.clone(); base = sc.clone(); }
        let m = quality::measure(&g, &sc);
        let want: Option<usize> = args.get(4).map(|a| a.parse().unwrap());
        if want.map_or(m.end_gap_max > 0.1, |w| w == k) { println!("frame {k} node {v} gap {:.3}", m.end_gap_max); show("  frame", &sc); show("  prevbase", &pb); for n in &sc.nodes { if n.id == to { println!("  node {} {} {}", n.id, n.x, n.y); } } let full = scene::route(&g, &c); show("  full", &full); break; }
    }
}
