// grid <file> <node|-> <dx> <dy> <mode> <x0> <y0> <x1> <y1> <step> : edge_at(tol 0.3) map
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
    if a[1] != "-" {
        let v = g.nodes.iter().position(|n| n.id == a[1]).expect("node");
        c[v].0 += a[2].parse::<f64>().unwrap();
        c[v].1 += a[3].parse::<f64>().unwrap();
    }
    let sc = match a[4].as_str() {
        "static" => st.clone(),
        "route" => scene::route(&g, &c),
        _ => scene::route_partial(&g, &c, &st, &base),
    };
    let f = |k: usize| a[k].parse::<f64>().unwrap();
    let (x0, y0, x1, y1, step) = (f(5), f(6), f(7), f(8), f(9));
    let tol: f64 = std::env::var("TOL").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let mut y = y0;
    while y <= y1 + 1e-9 {
        let mut line = format!("{y:7.1} ");
        let mut x = x0;
        while x <= x1 + 1e-9 {
            line.push_str(&match sc.edge_at(x, y, tol) {
                Some(i) => format!("{:>3}", i),
                None => "  .".to_string(),
            });
            x += step;
        }
        println!("{line}");
        y += step;
    }
}
