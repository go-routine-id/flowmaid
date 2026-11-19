// dump <on|off> <file> <node> <dx> <dy> <outsvg> [from to]
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
        if a.len() > 8 {
            for (i, (e, b)) in p.edges.iter().zip(&base.edges).enumerate() {
                if e.from == a[7] && e.to == a[8] {
                    println!("edge {i} base wps: {:?}", b.waypoints.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>());
                    println!("edge {i} now  wps: {:?}", e.waypoints.iter().map(|p| ((p.0*10.0).round()/10.0, (p.1*10.0).round()/10.0)).collect::<Vec<_>>());
                }
            }
        }
    });
}
