// rsvg <file> <static|route|partial> <out.svg> [node dx dy]
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
    let base: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut c = base.clone();
    if a.len() > 3 {
        let v = g.nodes.iter().position(|n| n.id == a[3]).expect("node");
        c[v].0 += a[4].parse::<f64>().unwrap();
        c[v].1 += a[5].parse::<f64>().unwrap();
    }
    let sc = match a[1].as_str() {
        "static" => st.clone(),
        "route" => scene::route(&g, &c),
        _ => scene::route_partial(&g, &c, &st, &base),
    };
    std::fs::write(&a[2], scene::to_svg(&sc)).unwrap();
}
