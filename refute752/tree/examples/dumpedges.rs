fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&args[1]).unwrap();
    let g = flowmaid::parser::parse(&src).unwrap();
    let s = flowmaid::scene::scene(&g);
    let routed = args.get(2).map_or(false, |a| a == "route");
    let s = if routed {
        let pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        flowmaid::scene::route(&g, &pos)
    } else { s };
    for (i, e) in s.edges.iter().enumerate() {
        let b = e.bezier;
        let f = |p: (f64, f64)| format!("({:.2},{:.2})", p.0, p.1);
        let w = &e.waypoints;
        // corners: points where direction changes on a polyline of axis legs (approx: middle of smoothing triples)
        let mut corners = vec![];
        if w.len() >= 2 {
            corners.push(w[0]);
            let mut k = 1;
            while k + 1 < w.len() {
                // a smoothing triple (pre, c, post) has pre->c and c->post perpendicular
                if k + 1 < w.len() {
                    let (a, c, d) = (w[k], w[k + 1].clone(), w.get(k + 2).copied());
                    let _ = (a, c, d);
                }
                k += 1;
            }
            corners.push(w[w.len() - 1]);
        }
        let straight = b[0] == b[1] && b[2] == b[3];
        println!(
            "#{i} {}->{} bez=[{} {} {} {}]{} wps({})={}",
            e.from, e.to, f(b[0]), f(b[1]), f(b[2]), f(b[3]),
            if straight { " STRAIGHT" } else { "" },
            w.len(),
            w.iter().map(|p| f(*p)).collect::<Vec<_>>().join(" ")
        );
    }
}
