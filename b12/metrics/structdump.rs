//! Dump a flowchart / state diagram's STRUCTURE (and flowmaid's own scene
//! geometry) as JSON, for `blockmetrics`.
//!
//! Not part of the crate: copy this file into a tree copy's `examples/`
//! and build it there, public API only:
//!
//! ```text
//! cp structdump.rs <tree>/examples/
//! cd <tree> && cargo build --release --example structdump
//! <target>/release/examples/structdump in.mmd > in.struct.json
//! ```
//!
//! The `structure` part (nodes, which subgraph holds each node, subgraph
//! parents, edge ends, the flow direction) is a property of the INPUT, so
//! one dump serves every engine and every flowmaid version that parses the
//! file the same way. The `scene` part is the geometry of the flowmaid
//! version this was built against; `blockmetrics --check-scene` compares
//! it with what it read from that version's SVG.
//!
//! Output (one JSON object on stdout):
//!   kind       "flowchart" | "state"
//!   direction  "TD" | "LR" | "RL" | "BT"
//!   nodes      [{id, label, shape, sub}]          sub = index of the innermost subgraph, or null
//!   subgraphs  [{id, title, parent, direction, nodes}]   nodes = DIRECT member node indices
//!   edges      [{from, to, label, kind, drawn}]   from/to = {"n": node index} | {"s": subgraph index};
//!                                                 `Graph::edges` first, then `Graph::sub_edges`
//!   scene      {width, height,
//!               nodes:    [{id, x, y, w, h}]      x,y = top-left
//!               clusters: [{id, x, y, w, h, depth}]
//!               edges:    [{from, to, label: {text, cx, cy, w} | null, points: [[x, y], ...]}]}
//!                         points = `waypoints`, or the four bezier points when there are none

use flowmaid::model::{Direction, EdgeKind, End, Graph};
use flowmaid::Document;
use std::fmt::Write as _;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn dir(d: Direction) -> &'static str {
    match d {
        Direction::TD => "TD",
        Direction::LR => "LR",
        Direction::RL => "RL",
        Direction::BT => "BT",
    }
}

fn opt_idx(i: Option<usize>) -> String {
    i.map_or("null".to_string(), |i| i.to_string())
}

fn end(e: End) -> String {
    match e {
        End::Node(i) => format!("{{\"n\":{i}}}"),
        End::Sub(i) => format!("{{\"s\":{i}}}"),
    }
}

fn num(v: f64) -> String {
    if v.is_finite() {
        format!("{:.3}", v)
    } else {
        "null".to_string()
    }
}

fn dump(kind: &str, g: &Graph) -> String {
    let mut o = String::new();
    let _ = write!(
        o,
        "{{\"tool\":\"structdump\",\"flowmaid\":{},\"kind\":{},\"direction\":{},\n",
        esc(env!("CARGO_PKG_VERSION")),
        esc(kind),
        esc(dir(g.direction))
    );

    // Innermost subgraph of each node: membership is direct ("nested
    // members belong to the child, not the parent").
    let mut sub_of: Vec<Option<usize>> = vec![None; g.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &n in &s.nodes {
            if n < sub_of.len() {
                sub_of[n] = Some(si);
            }
        }
    }

    o.push_str("\"nodes\":[");
    for (i, n) in g.nodes.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(
            o,
            "\n {{\"id\":{},\"label\":{},\"shape\":{},\"sub\":{}}}",
            esc(&n.id),
            esc(&n.label),
            esc(&format!("{:?}", n.shape)),
            opt_idx(sub_of[i])
        );
    }
    o.push_str("],\n\"subgraphs\":[");
    for (i, s) in g.subgraphs.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let members: Vec<String> = s.nodes.iter().map(|n| n.to_string()).collect();
        let _ = write!(
            o,
            "\n {{\"id\":{},\"title\":{},\"parent\":{},\"direction\":{},\"nodes\":[{}]}}",
            esc(&s.id),
            esc(&s.title),
            opt_idx(s.parent),
            s.direction.map_or("null".to_string(), |d| esc(dir(d))),
            members.join(",")
        );
    }
    o.push_str("],\n\"edges\":[");
    let drawn = |kind: EdgeKind, width: Option<f64>| {
        !matches!(kind, EdgeKind::Invisible) || width.is_some_and(|w| w > 0.0)
    };
    let mut first = true;
    for e in &g.edges {
        if !first {
            o.push(',');
        }
        first = false;
        let _ = write!(
            o,
            "\n {{\"from\":{{\"n\":{}}},\"to\":{{\"n\":{}}},\"label\":{},\"kind\":{},\"drawn\":{}}}",
            e.from,
            e.to,
            e.label.as_deref().map_or("null".to_string(), esc),
            esc(&format!("{:?}", e.kind)),
            drawn(e.kind, e.meta.style.stroke_width)
        );
    }
    for e in &g.sub_edges {
        if !first {
            o.push(',');
        }
        first = false;
        let _ = write!(
            o,
            "\n {{\"from\":{},\"to\":{},\"label\":{},\"kind\":{},\"drawn\":{}}}",
            end(e.from),
            end(e.to),
            e.label.as_deref().map_or("null".to_string(), esc),
            esc(&format!("{:?}", e.kind)),
            drawn(e.kind, e.meta.style.stroke_width)
        );
    }
    o.push_str("],\n");

    let sc = flowmaid::scene::scene(g);
    let _ = write!(
        o,
        "\"scene\":{{\"width\":{},\"height\":{},\n \"nodes\":[",
        num(sc.width),
        num(sc.height)
    );
    for (i, n) in sc.nodes.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(
            o,
            "\n  {{\"id\":{},\"x\":{},\"y\":{},\"w\":{},\"h\":{}}}",
            esc(&n.id),
            num(n.x - n.w / 2.0),
            num(n.y - n.h / 2.0),
            num(n.w),
            num(n.h)
        );
    }
    o.push_str("],\n \"clusters\":[");
    for (i, c) in sc.clusters.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(
            o,
            "\n  {{\"id\":{},\"x\":{},\"y\":{},\"w\":{},\"h\":{},\"depth\":{}}}",
            esc(&c.id),
            num(c.x),
            num(c.y),
            num(c.w),
            num(c.h),
            c.depth
        );
    }
    o.push_str("],\n \"edges\":[");
    for (i, e) in sc.edges.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let pts: Vec<(f64, f64)> = if e.waypoints.is_empty() {
            e.bezier.to_vec()
        } else {
            e.waypoints.clone()
        };
        let pts: Vec<String> = pts
            .iter()
            .map(|p| format!("[{},{}]", num(p.0), num(p.1)))
            .collect();
        let label = match &e.label {
            Some((text, (cx, cy), w)) => format!(
                "{{\"text\":{},\"cx\":{},\"cy\":{},\"w\":{}}}",
                esc(text),
                num(*cx),
                num(*cy),
                num(*w)
            ),
            None => "null".to_string(),
        };
        let _ = write!(
            o,
            "\n  {{\"from\":{},\"to\":{},\"routed\":{},\"label\":{},\"points\":[{}]}}",
            esc(&e.from),
            esc(&e.to),
            !e.waypoints.is_empty(),
            label,
            pts.join(",")
        );
    }
    o.push_str("]}}\n");
    o
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: structdump <file.mmd>");
    let src = std::fs::read_to_string(&path).expect("cannot read input");
    match flowmaid::parser::parse_document(&src) {
        Ok(Document::Flowchart(g)) => print!("{}", dump("flowchart", &g)),
        Ok(Document::State(g)) => print!("{}", dump("state", &g)),
        Ok(_) => {
            eprintln!("structdump: {path}: not a flowchart or state diagram");
            std::process::exit(3);
        }
        Err(e) => {
            eprintln!("structdump: {path}: {e:?}");
            std::process::exit(1);
        }
    }
}
