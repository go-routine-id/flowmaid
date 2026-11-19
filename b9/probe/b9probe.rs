// TEMPORARY probe for B9 (not committed).
use flowmaid::model::Document;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&args[1]).unwrap();
    let out = &args[2];
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!(),
    };
    let s = flowmaid::scene::scene(&g);
    let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
    std::fs::write(format!("{out}.static.svg"), flowmaid::scene::to_svg(&s)).unwrap();
    let r = flowmaid::scene::route(&g, &at);
    std::fs::write(format!("{out}.routed.svg"), flowmaid::scene::to_svg(&r)).unwrap();
    // drag node named args[3] by (dx, dy)
    if args.len() > 5 {
        let v = g.node_index(&args[3]).unwrap();
        let (dx, dy): (f64, f64) = (args[4].parse().unwrap(), args[5].parse().unwrap());
        let mut moved = at.clone();
        moved[v].0 += dx;
        moved[v].1 += dy;
        let t = std::time::Instant::now();
        let p = flowmaid::scene::route_partial(&g, &moved, &s, &at);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        std::fs::write(format!("{out}.partial.svg"), flowmaid::scene::to_svg(&p)).unwrap();
        let f = flowmaid::scene::route(&g, &moved);
        std::fs::write(format!("{out}.full.svg"), flowmaid::scene::to_svg(&f)).unwrap();
        println!("partial {ms:.2} ms; clusters {:?}", p.clusters.iter().map(|c| (c.id.clone(), c.x.round(), c.y.round(), c.w.round(), c.h.round())).collect::<Vec<_>>());
        // pick check: midpoint of each edge's polyline
        for (i, e) in p.edges.iter().enumerate() {
            let pts = &e.waypoints;
            let _ = (i, pts);
        }
    }
    println!("static clusters {:?}", s.clusters.iter().map(|c| (c.id.clone(), c.x.round(), c.y.round(), c.w.round(), c.h.round(), c.depth)).collect::<Vec<_>>());
    println!("routed clusters {:?}", r.clusters.iter().map(|c| (c.id.clone(), c.x.round(), c.y.round(), c.w.round(), c.h.round(), c.depth)).collect::<Vec<_>>());
    println!("canvas static {}x{} routed {}x{}", s.width, s.height, r.width, r.height);
}
