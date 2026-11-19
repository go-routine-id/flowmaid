// routebase <file> <node> dx1 dy1: drag node to +d1 (route() as the new base), then back to the scene centre.
// Prints the frame boxes: scene, route(c1), route_partial(back from route(c1)), route(back).
use flowmaid::model::{Document, Graph};
fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? { Document::Flowchart(g) | Document::State(g) => Some(g), _ => None }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let g = graph_of(&std::fs::read_to_string(&a[1]).unwrap()).unwrap();
    let v = g.node_index(&a[2]).unwrap();
    let (dx, dy): (f64, f64) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    let out = std::path::Path::new(&a[5]);
    flowmaid::quality::with_blocks(true, || {
        let base = flowmaid::scene::scene(&g);
        let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut c1 = at.clone();
        c1[v] = (c1[v].0 + dx, c1[v].1 + dy);
        let r1 = flowmaid::scene::route(&g, &c1);
        let back = flowmaid::scene::route_partial(&g, &at, &r1, &c1);
        let rback = flowmaid::scene::route(&g, &at);
        for (name, sc) in [("scene", &base), ("route(c1)", &r1), ("partial back from route(c1)", &back), ("route(back)", &rback)] {
            let f: Vec<String> = sc.clusters.iter().map(|c| format!("{}:[{:.0},{:.0} {:.0}x{:.0}]", c.id, c.x, c.y, c.w, c.h)).collect();
            println!("{name:30} {}", f.join(" "));
        }
        std::fs::write(out.join("rb-back.svg"), flowmaid::scene::to_svg(&back)).unwrap();
        std::fs::write(out.join("rb-r1.svg"), flowmaid::scene::to_svg(&r1)).unwrap();
        std::fs::write(out.join("rb-rback.svg"), flowmaid::scene::to_svg(&rback)).unwrap();
    });
}
