//! Drag & drop simulation without a GUI: take the automatic
//! layout, "drag" two nodes, re-route the edges with
//! `scene::route`, then export SVG. This is exactly the pipeline
//! an interactive desktop app uses.

fn main() {
    let src = std::fs::read_to_string("examples/demo.mmd").unwrap();
    let g = flowmaid::parser::parse(&src).unwrap();
    let s = flowmaid::scene::scene(&g);
    let mut pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
    // "drag": E (Show error) up-left, F (Done) to the right.
    pos[4].0 -= 260.0;
    pos[4].1 -= 180.0;
    pos[5].0 += 240.0;
    pos[5].1 -= 120.0;
    let s2 = flowmaid::scene::route(&g, &pos);
    std::fs::write("drag-sim.svg", flowmaid::scene::to_svg(&s2)).unwrap();
    println!("saved: drag-sim.svg");
}
