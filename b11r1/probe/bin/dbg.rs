// dbg <file> <node-id|-> <dx> <dy> <mode: static|route|partial> <x> <y> [edges...]
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
    let (x, y): (f64, f64) = (a[5].parse().unwrap(), a[6].parse().unwrap());
    for tol in [0.0, 0.75, 2.0, 5.0] {
        println!("edge_at({x},{y},{tol}) = {:?}", sc.edge_at(x, y, tol));
    }
    for d in [-6.0, -5.0, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0] {
        println!("  edge_at({x},{:.1},0.3) = {:?}", y + d, sc.edge_at(x, y + d, 0.3));
    }
    for e in &a[7..] {
        let i: usize = e.parse().unwrap();
        let ed = &sc.edges[i];
        println!("edge {i} {}->{} kind {:?} curve {:?}\n  bezier {:?}\n  waypoints {:?}", ed.from, ed.to, ed.kind, ed.meta.curve, ed.bezier, ed.waypoints);
    }
    if let Ok(r) = std::env::var("DBG_LEGS") {
        // x0,x1,y : every waypoint segment crossing the level line y between x0 and x1
        let v: Vec<f64> = r.split(',').map(|t| t.parse().unwrap()).collect();
        for (i, ed) in sc.edges.iter().enumerate() {
            let w: Vec<(f64, f64)> = if ed.waypoints.len() >= 2 { ed.waypoints.clone() } else { vec![ed.bezier[0], ed.bezier[3]] };
            for s in w.windows(2) {
                let (a, b) = (s[0], s[1]);
                let (lo, hi) = (a.1.min(b.1), a.1.max(b.1));
                let (xl, xh) = (a.0.min(b.0), a.0.max(b.0));
                if lo <= v[2] + 12.0 && hi >= v[2] - 12.0 && xh >= v[0] && xl <= v[1] {
                    println!("leg edge {i} {}->{}: ({:.4},{:.4}) -> ({:.4},{:.4}) wps={}", ed.from, ed.to, a.0, a.1, b.0, b.1, ed.waypoints.len());
                }
            }
        }
    }
    let svg = scene::to_svg(&sc);
    if let Some(out) = std::env::var_os("DBG_SVG") {
        std::fs::write(out, &svg).unwrap();
    }
    let mut k = 0;
    for chunk in svg.split("<path d=\"").skip(1) {
        let q = chunk.find('"').unwrap();
        let rest = &chunk[q..];
        let gt = rest.find('>').unwrap();
        if !rest[gt..].starts_with("><title>") {
            continue;
        }
        let title = &rest[gt + 8..rest.find("</title>").unwrap()];
        if a[7..].iter().any(|e| e.parse::<usize>().unwrap() == k) {
            println!("svg path #{k} [{title}]: {}", &chunk[..q]);
        }
        k += 1;
    }
    println!("svg edge paths: {k}, scene edges {}", sc.edges.len());
}
