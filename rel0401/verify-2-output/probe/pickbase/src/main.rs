// pick <file> <label> <x|=|+dx> <y|=|+dy> [partial] [outsvg]
// Drags the node with that label to (x, y) (= keeps, +d adds), routes,
// writes the SVG, and picks at the node's drawn centre mapped through the
// viewBox as a host would (SVG user space = scene space).
use flowmaid::model::Document;
use flowmaid::scene::{self, Hit};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let (g, title) = match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) => (g, "Flowchart diagram"),
        Document::State(g) => (g, "State diagram"),
        _ => panic!("not a flowchart/state"),
    };
    let laid = scene::scene(&g);
    let c: Vec<(f64, f64)> = laid.nodes.iter().map(|n| (n.x, n.y)).collect();
    let k = laid.nodes.iter().position(|n| n.label == a[2]).expect("label");
    let val = |s: &str, cur: f64| {
        if s == "=" { cur } else if let Some(d) = s.strip_prefix('+') { cur + d.parse::<f64>().unwrap() } else { s.parse().unwrap() }
    };
    let mut d = c.clone();
    d[k] = (val(&a[3], c[k].0), val(&a[4], c[k].1));
    let partial = a.get(5).map(|s| s == "partial").unwrap_or(false);
    let sc = if partial { scene::route_partial(&g, &d, &laid, &c) } else { scene::route(&g, &d) };
    let svg = scene::to_svg_titled(&sc, title);
    if let Some(p) = a.get(6) { std::fs::write(p, &svg).unwrap(); }
    let vb = svg.lines().next().unwrap();
    let vb = &vb[vb.find("viewBox=\"").unwrap() + 9..];
    let vb = &vb[..vb.find('"').unwrap()];
    // Drawn rect of the node.
    let title_line = format!("<g><title>{}</title>", sc.nodes[k].label);
    let lines: Vec<&str> = svg.lines().collect();
    let at = lines.iter().position(|l| *l == title_line).expect("node group");
    let el = lines[at + 1];
    let num = |key: &str| -> Option<f64> {
        let p = format!(" {key}=\"");
        let i = el.find(&p)? + p.len();
        el[i..i + el[i..].find('"')?].parse().ok()
    };
    let drawn = if el.starts_with("<rect") {
        (num("x").unwrap() + num("width").unwrap() / 2.0, num("y").unwrap() + num("height").unwrap() / 2.0)
    } else if el.starts_with("<circle") || el.starts_with("<ellipse") {
        (num("cx").unwrap(), num("cy").unwrap())
    } else {
        let t = lines[at + 1..].iter().find(|l| l.starts_with("<text ")).unwrap();
        let el = *t;
        let num = |key: &str| -> f64 { let p = format!(" {key}=\""); let i = el.find(&p).unwrap() + p.len(); el[i..i + el[i..].find('"').unwrap()].parse().unwrap() };
        (num("x"), num("y"))
    };
    let pick = sc.hit_test(drawn.0, drawn.1, 2.0);
    let others_moved = sc.nodes.iter().enumerate().filter(|(i, n)| *i != k && (n.x, n.y) != c[*i]).count();
    println!(
        "node {k} '{}' given ({:.1}, {:.1}) scene ({:.1}, {:.1}) box {:.0}x{:.0}; viewBox [{vb}]; Scene::origin ({:.1}, {:.1}); drawn centre ({:.1}, {:.1}) -> off by ({:.1}, {:.1}); pick there: {:?} (want Node({k})); other nodes moved in scene: {others_moved}",
        sc.nodes[k].label, d[k].0, d[k].1, sc.nodes[k].x, sc.nodes[k].y, sc.nodes[k].w, sc.nodes[k].h,
        0.0, 0.0, drawn.0, drawn.1, drawn.0 - d[k].0, drawn.1 - d[k].1, pick
    );
    let _ = Hit::Node(0);
}
