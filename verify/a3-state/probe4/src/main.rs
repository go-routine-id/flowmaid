use flowmaid::model::Document;
use flowmaid::scene::{route, scene, Scene};
use std::fmt::Write;

fn state(src: &str) -> flowmaid::model::Graph {
    match flowmaid::parser::parse_document(src).unwrap() {
        Document::State(g) => g,
        _ => panic!(),
    }
}

fn paint(sc: &Scene) -> String {
    let mut s = String::new();
    let (w, h) = (sc.width + 40.0, sc.height + 40.0);
    let _ = writeln!(s, "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='-20 -20 {w} {h}' font-family='Helvetica' font-size='14'><rect x='-20' y='-20' width='{w}' height='{h}' fill='white'/>");
    for n in &sc.nodes {
        let _ = writeln!(s, "<rect x='{}' y='{}' width='{}' height='{}' fill='#e4f5f4' stroke='#199a8e'/><text x='{}' y='{}' text-anchor='middle' dy='0.33em'>{}</text>", n.x - n.w / 2.0, n.y - n.h / 2.0, n.w, n.h, n.x, n.y, n.label);
    }
    for e in &sc.edges {
        let d = if e.waypoints.len() >= 2 {
            let mut d = format!("M {} {}", e.waypoints[0].0, e.waypoints[0].1);
            for p in &e.waypoints[1..] {
                let _ = write!(d, " L {} {}", p.0, p.1);
            }
            d
        } else {
            let b = e.bezier;
            format!("M {} {} C {} {}, {} {}, {} {}", b[0].0, b[0].1, b[1].0, b[1].1, b[2].0, b[2].1, b[3].0, b[3].1)
        };
        let _ = writeln!(s, "<path d='{d}' fill='none' stroke='#44507a' stroke-width='1.7'/>");
    }
    for e in &sc.edges {
        if let Some((t, c, w)) = &e.label {
            let t = t.replace('<', "&lt;").replace('>', "&gt;");
            let _ = writeln!(s, "<rect x='{}' y='{}' width='{w}' height='20' rx='4' fill='white' fill-opacity='0.85' stroke='#d5d9ec'/><text x='{}' y='{}' text-anchor='middle' dy='0.33em'>{t}</text>", c.0 - w / 2.0, c.1 - 10.0, c.0, c.1);
        }
    }
    s.push_str("</svg>\n");
    s
}

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let cases = [
        ("choice3", "stateDiagram-v2\nstate c <<choice>>\n[*] --> Order\nOrder --> c\nc --> Express : priority customer\nc --> Standard : regular customer\nc --> Hold : payment pending\nc --> Cancel : fraud suspected", (0.0, 50.0)),
        ("choice1", "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> False: if n < 0\nc --> True : if n >= 0\nc --> Zero : if n == 0", (0.0, 50.0)),
        ("lr", "stateDiagram-v2\ndirection LR\nstate c <<choice>>\n[*] --> Order\nOrder --> c\nc --> A : one\nc --> B : two\nc --> C : three", (60.0, 0.0)),
    ];
    for (name, src, (dx, dy)) in cases {
        let g = state(src);
        let sc = scene(&g);
        let mut m: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let ci = g.node_index("c").unwrap();
        m[ci].0 += dx;
        m[ci].1 += dy;
        let rt = route(&g, &m);
        std::fs::write(format!("{out}/{name}_drag.svg"), paint(&rt)).unwrap();
        std::fs::write(format!("{out}/{name}_static.svg"), paint(&sc)).unwrap();
    }
}
