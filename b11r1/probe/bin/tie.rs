// Pick != render repro: two plumb strokes on one x, one of which takes no hop; a third 10 px away.
use flowmaid::scene;
fn main() {
    let order: Vec<usize> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let src = "flowchart TD\n a0 --- b0\n a1 --- b1\n a2 --- b2\n a3 --- b3";
    let mut sc = scene::scene(&flowmaid::parser::parse(src).unwrap());
    // strokes: [short plumb at x=100 (its end 4 px above the level line), level, plumb x=90, plumb x=100]
    let lines = [((100.0, 46.0), (100.0, 100.0)), ((0.0, 50.0), (200.0, 50.0)), ((90.0, 0.0), (90.0, 100.0)), ((100.0, 0.0), (100.0, 100.0))];
    for (k, e) in sc.edges.iter_mut().enumerate() {
        let (a, b): ((f64, f64), (f64, f64)) = lines[order[k]];
        let at = |t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        e.bezier = [a, at(1.0 / 3.0), at(2.0 / 3.0), b];
        e.waypoints.clear();
    }
    for n in &mut sc.nodes {
        n.x = -1000.0;
        n.y = -1000.0;
    }
    let level = order.iter().position(|&k| k == 1).unwrap();
    let svg = scene::to_svg(&sc);
    let mut k = 0;
    for chunk in svg.split("<path d=\"").skip(1) {
        let q = chunk.find('"').unwrap();
        if chunk[q..].contains("><title>") && chunk[q..].find('>').map_or(false, |g| chunk[q + g..].starts_with("><title>")) {
            if k == level {
                println!("painted level stroke: {}", &chunk[..q]);
            }
            k += 1;
        }
    }
    // the scene shift: the level stroke starts at scene (0,50)
    for (x, y, what) in [(95.0, 45.0, "bridge top, mid"), (95.0, 50.0, "on the line between the two"), (90.0, 45.0, "crown over x=90"), (100.0, 45.0, "crown over x=100")] {
        println!("  edge_at({x},{y}, 0.5) = {:?}   [{what}; level edge is {level}]", sc.edge_at(x, y, 0.5));
    }
}
