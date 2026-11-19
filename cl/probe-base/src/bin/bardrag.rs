//! bardrag <file.mmd> <node> <dx> <dy> : each fork/join bar's size in scene(), route(scene centres),
//! route(dragged), route_partial(dragged), route_partial dragged back, route_sized(scene sizes, dragged).
use flowmaid::model::{Document, Shape};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!("flowchart or state"),
    };
    let s = flowmaid::scene::scene(&g);
    let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
    let sizes: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.w, n.h)).collect();
    let i = g.node_index(&a[2]).expect("node id");
    let mut to = at.clone();
    to[i].0 += a[3].parse::<f64>().unwrap();
    to[i].1 += a[4].parse::<f64>().unwrap();
    let r0 = flowmaid::scene::route(&g, &at);
    let r1 = flowmaid::scene::route(&g, &to);
    let p1 = flowmaid::scene::route_partial(&g, &to, &s, &at);
    let p2 = flowmaid::scene::route_partial(&g, &at, &r1, &to);
    let rs = flowmaid::scene::route_sized(&g, &to, &sizes);
    let p3 = flowmaid::scene::route_partial(&g, &to, &r0, &at);
    for (v, n) in g.nodes.iter().enumerate() {
        if n.shape != Shape::ForkBar {
            continue;
        }
        let sz = |sc: &flowmaid::scene::Scene| format!("{:.0}x{:.0}", sc.nodes[v].w, sc.nodes[v].h);
        println!(
            "{}: scene {} | route {} | route(drag) {} | partial(drag) {} | partial(back from route) {} | route_sized(drag) {} | partial(drag, base route) {}",
            n.id, sz(&s), sz(&r0), sz(&r1), sz(&p1), sz(&p2), sz(&rs), sz(&p3)
        );
    }
}
