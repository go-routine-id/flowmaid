// dumpdrag <on|off> <outdir> <file> [node:dx:dy ...]: block-mode scene and route_partial frames as SVG.
use flowmaid::model::{Document, Graph};
fn graph_of(src: &str) -> Option<Graph> {
    match flowmaid::parser::parse_document(src).ok()? {
        Document::Flowchart(g) | Document::State(g) => Some(g),
        _ => None,
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let on = a[1] == "on";
    let out = std::path::Path::new(&a[2]);
    std::fs::create_dir_all(out).unwrap();
    let f = &a[3];
    let g = graph_of(&std::fs::read_to_string(f).unwrap()).unwrap();
    let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
    flowmaid::quality::with_blocks(on, || {
        let base = flowmaid::scene::scene(&g);
        std::fs::write(out.join(format!("{name}.scene.svg")), flowmaid::scene::to_svg(&base)).unwrap();
        let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = flowmaid::scene::route(&g, &at);
        std::fs::write(out.join(format!("{name}.route.svg")), flowmaid::scene::to_svg(&r)).unwrap();
        for d in &a[4..] {
            // node:dx:dy[>dx2:dy2...]: a drag from the scene, then chained frames.
            let (head, rest) = d.split_once(':').unwrap();
            let v = g.node_index(head).unwrap_or_else(|| panic!("no node {}", head));
            let (mut pb, mut pc) = (base.clone(), at.clone());
            let mut tag = head.to_string();
            for step in rest.split('>') {
                let p: Vec<&str> = step.split(':').collect();
                let (dx, dy): (f64, f64) = (p[0].parse().unwrap(), p[1].parse().unwrap());
                let mut c = at.clone();
                c[v] = (c[v].0 + dx, c[v].1 + dy);
                let sc = flowmaid::scene::route_partial(&g, &c, &pb, &pc);
                tag = format!("{tag}_{}_{}", p[0], p[1]);
                std::fs::write(out.join(format!("{name}.{tag}.svg")), flowmaid::scene::to_svg(&sc)).unwrap();
                pb = sc;
                pc = c;
            }
        }
    });
}
