use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let v: usize = a[2].parse().unwrap();
    let ei: usize = a[3].parse().unwrap();
    let (dx, dy): (f64, f64) = (a[4].parse().unwrap(), a[5].parse().unwrap());
    let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => return };
    let on = with_blocks(true, || scene::scene(&g));
    let c: Vec<(f64, f64)> = on.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut pos = c.clone();
    pos[v].0 += dx; pos[v].1 += dy;
    let p = with_blocks(true, || scene::route_partial(&g, &pos, &on, &c));
    let e = &p.edges[ei];
    println!("edge {} > {} waypoints {:?}", e.from, e.to, e.waypoints);
    println!("bezier {:?} curve {:?}", e.bezier, e.meta.curve);
    for w in e.waypoints.windows(2) {
        let m = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
        println!("mid {:.1},{:.1} node_at {:?} edge_at {:?} hit {:?}", m.0, m.1, p.node_at(m.0, m.1), p.edge_at(m.0, m.1, 4.0), p.hit_test(m.0, m.1, 4.0));
    }
    let svg = scene::to_svg(&p);
    std::fs::write(&a[6], svg).unwrap();
    println!("canvas {}x{} moved node {} to {:?}", p.width, p.height, p.nodes[v].id, (p.nodes[v].x, p.nodes[v].y));
}
