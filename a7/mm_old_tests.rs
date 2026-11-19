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
    fn wide_siblings_do_not_overlap() {
        // Many long labels on the same ring: the old leaf-count layout
        // collapsed them onto one another; the footprint-aware wedge
        // grows the ring so every box keeps its own slice.
        let mut src = String::from("mindmap\n  root(Center)\n");
        for i in 0..12 {
            src.push_str(&format!("    A very long branch label number {i:02}\n"));
        }
        let ms = scene(&mind(&src));
        let boxes: Vec<(f64, f64, f64, f64)> = ms
            .nodes
            .iter()
            .map(|n| (n.x, n.y, n.x + n.w, n.y + n.h))
            .collect();
        for i in 0..boxes.len() {
            for j in (i + 1)..boxes.len() {
                let (a, b) = (boxes[i], boxes[j]);
                let overlap = a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
                assert!(!overlap, "node {i} ({:?}) overlaps node {j} ({:?})", a, b);
            }
        }
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
        let bang = ms
            .nodes
            .iter()
            .find(|n| n.shape == MindShape::Bang)
            .unwrap();
        let cloud = ms
            .nodes
            .iter()
            .find(|n| n.shape == MindShape::Cloud)
            .unwrap();
        assert!(perimeter(bang).unwrap().len() >= 12);
        assert!(perimeter(cloud).unwrap().len() >= 12);
        // A plain rounded node has no polygon perimeter.
        assert!(perimeter(&ms.nodes[0]).is_none());
    }

    /// Report Q5: branch nodes hard-coded white text on their accent
    /// (amber 2.6:1, green 3.2:1); the scene's per-node `text_color`
    /// (what host painters use) must agree with the SVG.
    #[test]
    fn branch_text_reads_on_every_accent() {
        let n = crate::style::ACCENTS.len();
        let mut src = String::from("mindmap\nroot((R))\n");
        for i in 0..n {
            src.push_str(&format!("  Br{i}\n    Leaf{i}\n"));
        }
        let ms = scene(&mind(&src));
        let svg = to_svg(&ms);
        for i in 0..n {
            let accent = crate::style::accent(i);
            for label in [format!("Br{i}"), format!("Leaf{i}")] {
                let ink = crate::style::assert_text_reads(&svg, &label, accent);
                assert_eq!(ink, crate::style::text_on(accent), "{label}");
                let node = ms.nodes.iter().find(|b| b.text == label).unwrap();
                assert_eq!((node.fill, node.text_color), (accent, ink));
            }
        }
    }
}
