// dump <on|off> <file> <node> <dx> <dy> <outsvg> [from to]: route_partial of one drag from the static scene
use flowmaid::model::Document;
use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let on = a[1] == "on";
    let src = std::fs::read_to_string(&a[2]).unwrap();
    let g = match flowmaid::parser::parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => panic!() };
    let (dx, dy): (f64, f64) = (a[4].parse().unwrap(), a[5].parse().unwrap());
    flowmaid::quality::with_blocks(on, || {
        let base = scene::scene(&g);
        let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut c = at.clone();
        let v = g.node_index(&a[3]).unwrap();
        c[v] = (c[v].0 + dx, c[v].1 + dy);
        let p = scene::route_partial(&g, &c, &base, &at);
        std::fs::write(&a[6], scene::to_svg(&p)).unwrap();
        std::fs::write(format!("{}.base.svg", a[6]), scene::to_svg(&base)).unwrap();
        let m = flowmaid::quality::measure(&g, &p);
        let mut fl = m.fields(); fl.extend(m.late_fields()); fl.extend(m.tail_fields());
        let pick = ["x_drawn", "edge_node_x", "edge_frame_pairs", "lateral_x", "fan_x", "edge_chip"];
        println!("defects={} {}", m.defects(), fl.iter().filter(|x| pick.contains(&x.0)).map(|x| format!("{}={}", x.0, x.1)).collect::<Vec<_>>().join(" "));
        let poly = |e: &flowmaid::scene::SceneEdge| -> Vec<(f64, f64)> {
            if e.waypoints.len() >= 2 { return e.waypoints.clone(); }
            let b = e.bezier;
            (0..=24).map(|k| { let t = k as f64 / 24.0; let u = 1.0 - t;
                (u*u*u*b[0].0 + 3.0*u*u*t*b[1].0 + 3.0*u*t*t*b[2].0 + t*t*t*b[3].0,
                 u*u*u*b[0].1 + 3.0*u*u*t*b[1].1 + 3.0*u*t*t*b[2].1 + t*t*t*b[3].1) }).collect()
        };
        let segx = |a: (f64,f64), b: (f64,f64), c: (f64,f64), d: (f64,f64)| {
            let o = |p: (f64,f64), q: (f64,f64), r: (f64,f64)| (q.0-p.0)*(r.1-p.1) - (q.1-p.1)*(r.0-p.0);
            o(c,d,a)*o(c,d,b) < -1e-9 && o(a,b,c)*o(a,b,d) < -1e-9
        };
        let ps: Vec<_> = p.edges.iter().map(poly).collect();
        for i in 0..ps.len() { for j in i+1..ps.len() {
            let mut k = 0; for u in ps[i].windows(2) { for v in ps[j].windows(2) { if segx(u[0],u[1],v[0],v[1]) { k += 1; } } }
            if k > 0 { println!("cross {}->{} x {}->{} : {k}", p.edges[i].from, p.edges[i].to, p.edges[j].from, p.edges[j].to); }
        }}
        for d in &m.details { if d.contains("edge_node") || d.contains("edge_frame") || d.contains("fan") { println!("detail {d}"); } }
        for c in &p.clusters { println!("cluster {} {:.1} {:.1} {:.1} {:.1}", c.id, c.x, c.y, c.w, c.h); }
        if a.len() > 8 {
            for (i, (e, b)) in p.edges.iter().zip(&base.edges).enumerate() {
                if e.from == a[7] && e.to == a[8] {
                    println!("edge {i} base wps: {:?}", b.waypoints.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>());
                    println!("edge {i} now  wps: {:?}", e.waypoints.iter().map(|p| ((p.0*10.0).round()/10.0, (p.1*10.0).round()/10.0)).collect::<Vec<_>>());
                    println!("edge {i} label {:?}", e.label.as_ref().map(|l| l.1));
                }
            }
        }
    });
}
