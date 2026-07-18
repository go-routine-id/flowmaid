//! Mindmap rendering, in the Mermaid style: the root sits at the centre
//! and branches radiate outward. Nodes at depth `k` land on a ring of
//! radius `k · RING`; each subtree owns an angular wedge sized by its
//! leaf count, so busy branches get more room. Every top-level branch
//! gets a stable accent that its whole subtree fills, and the six node
//! shapes (rounded / square / circle / hexagon / bang / cloud) are drawn
//! for real.
//!
//! Like `pie`/`seq` there is nothing draggable and no `route()`:
//! [`scene`] computes every coordinate a painter needs and [`to_svg`]
//! serialises it. [`perimeter`] exposes the polygon points for the
//! non-box shapes so a GUI painter can match the SVG exactly.

use crate::layout::{line_count, text_width};
use crate::model::{MindShape, Mindmap};
use crate::scene::{escape, svg_open, TEXT_COLOR};
use std::f64::consts::{FRAC_PI_2, TAU};

/// Horizontal padding inside a node box.
pub const PAD_X: f64 = 12.0;
/// Vertical padding inside a node box.
pub const PAD_Y: f64 = 7.0;
/// Height of one label line.
pub const LINE_H: f64 = 18.0;
/// Radius added per depth level (root = 0).
pub const RING: f64 = 172.0;
/// Canvas margin around the whole tree.
pub const MARGIN: f64 = 28.0;
/// Base font size (px).
pub const FONT: u32 = 14;

/// Root fill (neutral grey) + its dark text.
const ROOT_FILL: &str = "#c9cfe0";
const ROOT_TEXT: &str = TEXT_COLOR;
/// Branch nodes fill with their accent and use white text.
const BRANCH_TEXT: &str = "#ffffff";

/// One laid-out mindmap node. `(x, y)` is the top-left; the label is
/// drawn centred. Colors are `#rrggbb` so painters match the SVG.
#[derive(Debug, Clone)]
pub struct MindNodeBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub text: String,
    pub shape: MindShape,
    pub depth: usize,
    pub fill: &'static str,
    pub text_color: &'static str,
}

impl MindNodeBox {
    pub fn cx(&self) -> f64 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f64 {
        self.y + self.h / 2.0
    }
}

/// A parent→child connector as a cubic Bézier (start, c1, c2, end),
/// centre-to-centre; both ends are hidden behind the node boxes.
#[derive(Debug, Clone)]
pub struct MindLink {
    pub bezier: [(f64, f64); 4],
    pub color: &'static str,
    pub width: f64,
}

/// Everything needed to draw a mindmap: canvas size, node boxes (parse
/// order, root first), and the connectors.
#[derive(Debug, Clone)]
pub struct MindScene {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<MindNodeBox>,
    pub edges: Vec<MindLink>,
}

/// Intrinsic size of a node from its label and shape.
fn node_size(text: &str, shape: MindShape) -> (f64, f64) {
    let lines = line_count(text) as f64;
    let tw = text_width(text);
    let base_w = tw + 2.0 * PAD_X;
    let base_h = lines * LINE_H + 2.0 * PAD_Y;
    match shape {
        MindShape::Circle => {
            let d = base_w.max(base_h * 1.6);
            (d, base_h.max(d * 0.62))
        }
        MindShape::Cloud => (base_w * 1.4 + 8.0, base_h * 1.35 + 6.0),
        MindShape::Bang => (base_w * 1.25 + 6.0, base_h * 1.3 + 6.0),
        MindShape::Hexagon => (base_w + base_h * 0.7, base_h),
        _ => (base_w, base_h),
    }
}

/// Leaf-count weight of each subtree (min 1), for angular allocation.
fn weigh(d: &Mindmap, i: usize, w: &mut [usize]) -> usize {
    let kids = &d.nodes[i].children;
    let total = if kids.is_empty() {
        1
    } else {
        kids.iter().map(|&c| weigh(d, c, w)).sum()
    };
    w[i] = total;
    total
}

/// Place subtree `i` inside the angular wedge `[a0, a1]`. Node centres
/// are relative to the root at the origin; converted to boxes later.
fn place(d: &Mindmap, w: &[usize], i: usize, a0: f64, a1: f64, c: &mut [(f64, f64)]) {
    let kids = &d.nodes[i].children;
    if kids.is_empty() {
        return;
    }
    let total: f64 = kids.iter().map(|&k| w[k] as f64).sum();
    let mut acc = a0;
    for &k in kids {
        let span = (a1 - a0) * (w[k] as f64 / total);
        let (cs, ce) = (acc, acc + span);
        let mid = (cs + ce) / 2.0;
        let r = RING * d.nodes[k].depth as f64;
        c[k] = (r * mid.cos(), r * mid.sin());
        place(d, w, k, cs, ce, c);
        acc = ce;
    }
}

/// Branch accent for a top-level subtree, by its order under the root.
fn branch_color(order: usize) -> &'static str {
    crate::style::accent(order)
}

/// Auto-layout: radial placement, then [`route`] with those centres.
/// An empty mindmap yields a minimal canvas.
pub fn scene(d: &Mindmap) -> MindScene {
    if d.nodes.is_empty() {
        return route(d, &[]);
    }
    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(|n| node_size(&n.text, n.shape)).collect();

    // Angular allocation by leaf weight; radiate from the top, clockwise.
    let mut w = vec![0usize; d.nodes.len()];
    weigh(d, 0, &mut w);
    let mut c = vec![(0.0f64, 0.0f64); d.nodes.len()];
    place(d, &w, 0, -FRAC_PI_2, TAU - FRAC_PI_2, &mut c);

    // Normalise so the whole tree's bounding box starts at MARGIN, then
    // hand the resulting centres to route().
    let (mut minx, mut miny) = (f64::MAX, f64::MAX);
    for (i, &(x, y)) in c.iter().enumerate() {
        minx = minx.min(x - sizes[i].0 / 2.0);
        miny = miny.min(y - sizes[i].1 / 2.0);
    }
    let centres: Vec<(f64, f64)> = c
        .iter()
        .map(|&(x, y)| (x + MARGIN - minx, y + MARGIN - miny))
        .collect();
    route(d, &centres)
}

/// Build a scene from EXPLICIT node centres (`pos[i]` = centre of node
/// `i`, in world coords). Used for interactive drag: the caller moves a
/// centre and the connectors are recomputed to follow, without touching
/// the radial layout. Positions are taken as-is (not re-normalised), so
/// dragging never makes the diagram slide. `pos` must match node count.
pub fn route(d: &Mindmap, pos: &[(f64, f64)]) -> MindScene {
    if d.nodes.is_empty() || pos.len() != d.nodes.len() {
        return MindScene {
            width: 2.0 * MARGIN,
            height: 2.0 * MARGIN,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }

    let mut branch_order = vec![0usize; d.nodes.len()];
    for (order, &ch) in d.nodes[0].children.iter().enumerate() {
        branch_order[ch] = order;
    }

    let mut nodes = Vec::with_capacity(d.nodes.len());
    for (i, n) in d.nodes.iter().enumerate() {
        let (w2, h2) = node_size(&n.text, n.shape);
        let (fill, text_color) = if n.depth == 0 {
            (ROOT_FILL, ROOT_TEXT)
        } else {
            (branch_color(branch_order[n.branch.unwrap_or(i)]), BRANCH_TEXT)
        };
        nodes.push(MindNodeBox {
            x: pos[i].0 - w2 / 2.0,
            y: pos[i].1 - h2 / 2.0,
            w: w2,
            h: h2,
            text: n.text.clone(),
            shape: n.shape,
            depth: n.depth,
            fill,
            text_color,
        });
    }

    // Connectors: centre-to-centre, colored by the child's branch and
    // thicker nearer the root; the node boxes hide both endpoints.
    let mut edges = Vec::new();
    for (i, n) in d.nodes.iter().enumerate() {
        let Some(p) = n.parent else { continue };
        let (a, b) = ((nodes[p].cx(), nodes[p].cy()), (nodes[i].cx(), nodes[i].cy()));
        let c1 = (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0);
        let c2 = (a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0);
        edges.push(MindLink {
            bezier: [a, c1, c2, b],
            color: nodes[i].fill,
            width: (5.5 - n.depth as f64).max(2.0),
        });
    }

    let width = nodes.iter().map(|n| n.x + n.w).fold(0.0, f64::max) + MARGIN;
    let height = nodes.iter().map(|n| n.y + n.h).fold(0.0, f64::max) + MARGIN;
    MindScene {
        width,
        height,
        nodes,
        edges,
    }
}

/// Perimeter points (absolute coords) for the polygon shapes — hexagon,
/// bang (spiky), cloud (scalloped). `None` for box/ellipse shapes, which
/// a painter draws with its own rect/ellipse primitive. Exposed so the
/// SVG and any GUI painter share identical geometry.
pub fn perimeter(n: &MindNodeBox) -> Option<Vec<(f64, f64)>> {
    match n.shape {
        MindShape::Hexagon => Some(hexagon_pts(n)),
        MindShape::Bang => Some(star_pts(n, 16, 0.74)),
        MindShape::Cloud => Some(cloud_pts(n)),
        _ => None,
    }
}

fn hexagon_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (x, y, w, h) = (n.x, n.y, n.w, n.h);
    let k = (h * 0.5).min(w / 3.0);
    vec![
        (x + k, y),
        (x + w - k, y),
        (x + w, y + h / 2.0),
        (x + w - k, y + h),
        (x + k, y + h),
        (x, y + h / 2.0),
    ]
}

/// A star/explosion: `spikes` outer points alternating with inner points
/// at `inner` × the radius. Used for the "bang" shape.
fn star_pts(n: &MindNodeBox, spikes: usize, inner: f64) -> Vec<(f64, f64)> {
    let (cx, cy) = (n.cx(), n.cy());
    let (rx, ry) = (n.w / 2.0, n.h / 2.0);
    (0..spikes * 2)
        .map(|i| {
            let ang = -FRAC_PI_2 + std::f64::consts::PI * i as f64 / spikes as f64;
            let f = if i % 2 == 0 { 1.0 } else { inner };
            (cx + rx * f * ang.cos(), cy + ry * f * ang.sin())
        })
        .collect()
}

/// A scalloped ellipse (cloud): many points whose radius bulges outward
/// on a low-frequency wave.
fn cloud_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (cx, cy) = (n.cx(), n.cy());
    let (rx, ry) = (n.w / 2.0, n.h / 2.0);
    let steps = 48;
    let bumps = 9.0;
    (0..steps)
        .map(|i| {
            let ang = TAU * i as f64 / steps as f64;
            let f = 0.86 + 0.14 * (ang * bumps).sin().abs();
            (cx + rx * f * ang.cos(), cy + ry * f * ang.sin())
        })
        .collect()
}

/// Serialise a [`MindScene`] to a standalone SVG document.
pub fn to_svg(ms: &MindScene) -> String {
    let mut s = String::new();
    svg_open(&mut s, ms.width, ms.height, FONT, "Mind map");

    // Connectors behind the nodes.
    for e in &ms.edges {
        let [a, c1, c2, b] = e.bezier;
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1} {:.1} {:.1} {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linecap=\"round\"/>\n",
            a.0, a.1, c1.0, c1.1, c2.0, c2.1, b.0, b.1, e.color, e.width
        ));
    }

    for n in &ms.nodes {
        s.push_str(&shape_svg(n));
        s.push_str(&label_svg(n));
    }

    s.push_str("</svg>\n");
    s
}

/// SVG for a node's filled outline.
fn shape_svg(n: &MindNodeBox) -> String {
    let (x, y, w, h) = (n.x, n.y, n.w, n.h);
    if let Some(pts) = perimeter(n) {
        let points = pts
            .iter()
            .map(|(px, py)| format!("{:.1},{:.1}", px, py))
            .collect::<Vec<_>>()
            .join(" ");
        return format!("<polygon points=\"{}\" fill=\"{}\"/>\n", points, n.fill);
    }
    match n.shape {
        MindShape::Circle => format!(
            "<ellipse cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"{}\"/>\n",
            n.cx(), n.cy(), w / 2.0, h / 2.0, n.fill
        ),
        _ => {
            let rx = match n.shape {
                MindShape::Square => 3.0,
                _ => (h / 2.0).min(14.0),
            };
            format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                 rx=\"{:.1}\" fill=\"{}\"/>\n",
                x, y, w, h, rx, n.fill
            )
        }
    }
}

/// SVG for a node's centred (possibly multi-line) label.
fn label_svg(n: &MindNodeBox) -> String {
    let lines: Vec<&str> = n.text.split('\n').collect();
    let weight = if n.depth == 0 { " font-weight=\"bold\"" } else { "" };
    let cx = n.cx();
    let first_dy = 0.32 * FONT as f64 - (lines.len() as f64 - 1.0) * LINE_H / 2.0;
    let mut spans = String::new();
    for (i, line) in lines.iter().enumerate() {
        let dy = if i == 0 { first_dy } else { LINE_H };
        spans.push_str(&format!(
            "<tspan x=\"{:.1}\" dy=\"{:.1}\">{}</tspan>",
            cx,
            dy,
            escape(line)
        ));
    }
    format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"{}\"{}>{}</text>\n",
        cx,
        n.cy(),
        n.text_color,
        weight,
        spans
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn mind(src: &str) -> Mindmap {
        match parse_document(src).unwrap() {
            Document::Mindmap(m) => m,
            _ => panic!("expected a mindmap"),
        }
    }

    #[test]
    fn builds_tree_from_indentation() {
        let m = mind("mindmap\n  root((Root))\n    A\n      A1\n    B\n");
        assert_eq!(m.nodes.len(), 4);
        assert_eq!(m.nodes[0].text, "Root");
        assert_eq!(m.nodes[0].shape, MindShape::Circle);
        assert_eq!(m.nodes[0].children.len(), 2);
        let a = m.nodes[0].children[0];
        assert_eq!(m.nodes[m.nodes[a].children[0]].text, "A1");
    }

    #[test]
    fn shapes_and_br_parse() {
        let m = mind("mindmap\nroot\n  [sq]\n  (round)\n  ((circ))\n  {{hex}}\n  a<br/>b\n");
        let shapes: Vec<MindShape> = m.nodes[1..].iter().map(|n| n.shape).collect();
        assert_eq!(
            shapes,
            [
                MindShape::Square,
                MindShape::Rounded,
                MindShape::Circle,
                MindShape::Hexagon,
                MindShape::Rounded,
            ]
        );
        assert_eq!(m.nodes.last().unwrap().text, "a\nb");
    }

    #[test]
    fn multiword_text_before_paren_stays_literal() {
        // An id can't contain a space, so "call foo(x)" is plain text,
        // not a rounded node labeled "x".
        let m = mind("mindmap\nRoot\n  call foo(x)\n  id(real)\n");
        assert_eq!(m.nodes[1].text, "call foo(x)");
        assert_eq!(m.nodes[1].shape, MindShape::Rounded);
        // A genuine adjacent id still strips.
        assert_eq!(m.nodes[2].text, "real");
    }

    #[test]
    fn radial_layout_stays_inside_canvas() {
        let ms = scene(&mind(
            "mindmap\n  root((Order))\n    Customer\n      Buat Order\n      Lihat Order\n    \
             Owner\n      Kelola\n      State Machine\n    Sistem\n",
        ));
        assert!(ms.width > 0.0 && ms.height > 0.0);
        for n in &ms.nodes {
            assert!(n.x >= -0.01 && n.y >= -0.01, "node top-left inside");
            assert!(n.x + n.w <= ms.width + 0.01, "node right inside canvas");
            assert!(n.y + n.h <= ms.height + 0.01, "node bottom inside canvas");
        }
        assert_eq!(ms.edges.len(), ms.nodes.len() - 1);
    }

    #[test]
    fn root_sits_near_the_centre() {
        let ms = scene(&mind("mindmap\nroot((R))\n  A\n  B\n  C\n  D\n"));
        let root = &ms.nodes[0];
        // Children radiate around the root, so it lands near the middle.
        assert!((root.cx() - ms.width / 2.0).abs() < ms.width * 0.28);
        assert!((root.cy() - ms.height / 2.0).abs() < ms.height * 0.28);
    }

    #[test]
    fn to_svg_draws_shapes_and_links() {
        let svg = to_svg(&scene(&mind(
            "mindmap\nroot((R))\n  A\n  ))boom((\n  )fog(\n  {{hx}}\n",
        )));
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("<ellipse"), "circle root");
        assert!(svg.contains("<polygon"), "bang/cloud/hexagon polygons");
        assert!(svg.contains("<path"), "connectors");
        assert!(svg.contains("fill=\"#ffffff\""), "branch text is white");
    }

    #[test]
    fn route_follows_dragged_positions() {
        let m = mind("mindmap\nroot((R))\n  A\n  B\n");
        let auto = scene(&m);
        // Seed from auto, then "drag" node A far to the right.
        let mut pos: Vec<(f64, f64)> = auto.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        pos[1] = (pos[1].0 + 500.0, pos[1].1);
        let routed = route(&m, &pos);
        // Node A's box moved with it; the root→A connector's end follows.
        assert!((routed.nodes[1].cx() - pos[1].0).abs() < 0.01);
        let a_edge = &routed.edges[0];
        assert!((a_edge.bezier[3].0 - pos[1].0).abs() < routed.nodes[1].w);
        // Canvas grew to keep the dragged node inside.
        assert!(routed.width > auto.width);
    }

    #[test]
    fn bang_and_cloud_have_perimeters() {
        let ms = scene(&mind("mindmap\nR\n  ))b((\n  )c(\n"));
        let bang = ms.nodes.iter().find(|n| n.shape == MindShape::Bang).unwrap();
        let cloud = ms.nodes.iter().find(|n| n.shape == MindShape::Cloud).unwrap();
        assert!(perimeter(bang).unwrap().len() >= 12);
        assert!(perimeter(cloud).unwrap().len() >= 12);
        // A plain rounded node has no polygon perimeter.
        assert!(perimeter(&ms.nodes[0]).is_none());
    }
}
