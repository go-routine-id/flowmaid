//! dragsvg <file.mmd> <scene|route|partial> [<node> <dx> <dy>]... : the SVG of a flowchart / state
//! diagram's static scene, of route() at the (dragged) centres, or of route_partial() from the scene.
use flowmaid::model::Document;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!("flowchart or state"),
    };
    let s = flowmaid::scene::scene(&g);
    let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut to = at.clone();
    if a[2] == "pidx" {
        // pidx <frac%> <dx> <dy>: route_partial with the node at that share of the node list dragged
        let n = g.nodes.len();
        if n == 0 {
            return;
        }
        let i = (a[3].parse::<usize>().unwrap() * n / 100).min(n - 1);
        to[i].0 += a[4].parse::<f64>().unwrap();
        to[i].1 += a[5].parse::<f64>().unwrap();
        print!("{}", flowmaid::scene::to_svg(&flowmaid::scene::route_partial(&g, &to, &s, &at)));
        return;
    }
    for d in a[3..].chunks(3) {
        let i = g.node_index(&d[0]).expect("node id");
        to[i].0 += d[1].parse::<f64>().unwrap();
        to[i].1 += d[2].parse::<f64>().unwrap();
    }
    let out = match a[2].as_str() {
        "scene" => s,
        "route" => flowmaid::scene::route(&g, &to),
        _ => flowmaid::scene::route_partial(&g, &to, &s, &at),
    };
    print!("{}", flowmaid::scene::to_svg(&out));
}
