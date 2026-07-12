//! SVG wrapper: `render(graph)` = automatic scene -> SVG serialisation.
//! All geometry & serialisation logic lives in the `scene` module,
//! so the output of render() and the interactive flow
//! (scene/route/to_svg) are guaranteed identical.

use crate::model::{ClassDiagram, ErDiagram, Graph, SequenceDiagram};

pub fn render(g: &Graph) -> String {
    crate::scene::to_svg(&crate::scene::scene(g))
}

/// Render an Entity-Relationship diagram (tables + crow's foot).
/// Geometry & serialisation live in the `er` module.
pub fn render_er(d: &ErDiagram) -> String {
    crate::er::to_svg(&crate::er::scene(d))
}

/// Render a UML class diagram (three-compartment boxes + UML end
/// glyphs). Geometry & serialisation live in the `class` module.
pub fn render_class(d: &ClassDiagram) -> String {
    crate::class::to_svg(&crate::class::scene(d))
}

/// Render a sequence diagram (lifelines + linear message rows).
/// Geometry & serialisation live in the `seq` module.
pub fn render_seq(d: &SequenceDiagram) -> String {
    crate::seq::to_svg(&crate::seq::scene(d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn paths(svg: &str) -> Vec<String> {
        svg.lines()
            .filter(|l| l.starts_with("<path d="))
            .map(|s| s.to_string())
            .collect()
    }

    fn attr(svg: &str, name: &str) -> f64 {
        let pat = format!("{}=\"", name);
        let i = svg.find(&pat).unwrap() + pat.len();
        let rest = &svg[i..];
        rest[..rest.find('"').unwrap()].parse().unwrap()
    }

    fn coords(path_line: &str) -> Vec<(f64, f64)> {
        let i = path_line.find("d=\"").unwrap() + 3;
        let rest = &path_line[i..];
        let d = &rest[..rest.find('"').unwrap()];
        // Skip non-numeric tokens (arc flags / path commands) so the
        // helper works on cylinder `A…` paths too.
        let nums: Vec<f64> = d
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter_map(|s| s.parse().ok())
            .collect();
        nums.chunks_exact(2).map(|c| (c[0], c[1])).collect()
    }

    /// End-to-end regression over the advanced showcase — nested
    /// subgraphs, edges-to-subgraph, every node shape, `<br/>`
    /// labels, custom colors, fan-out, and all link types together.
    /// Guards the whole feature set as one interacting whole.
    #[test]
    fn advanced_showcase_parses_and_renders() {
        use crate::model::{Document, EdgeKind, Shape};
        let src = include_str!("../examples/advanced.mmd");
        let doc = crate::parser::parse_document(src).unwrap();
        let Document::Flowchart(g) = doc else {
            panic!("advanced.mmd is a flowchart");
        };

        // Structure: 3 subgraphs (CI, Platform, Workers nested).
        assert_eq!(g.subgraphs.len(), 3);
        let workers = g.subgraphs.iter().find(|s| s.id == "Workers").unwrap();
        assert_eq!(workers.parent, Some(g.subgraphs.iter().position(|s| s.id == "Platform").unwrap()));
        assert_eq!(workers.direction, Some(crate::model::Direction::LR));
        // Edges targeting a subgraph landed in sub_edges (Dev->CI,
        // Reg->Platform, Platform->Mon, Rollback->Platform).
        assert!(g.sub_edges.len() >= 4, "got {} sub_edges", g.sub_edges.len());

        // Every classic shape is present.
        let shapes: Vec<Shape> = g.nodes.iter().map(|n| n.shape).collect();
        for want in [
            Shape::Subroutine,
            Shape::Hexagon,
            Shape::Cylinder,
            Shape::Parallelogram,
            Shape::ParallelogramAlt,
            Shape::DoubleCircle,
            Shape::Rounded,
        ] {
            assert!(shapes.contains(&want), "missing shape {want:?}");
        }
        // A `<br/>` label became multi-line.
        assert!(
            g.nodes.iter().any(|n| n.label.contains('\n')),
            "expected a multi-line <br/> label"
        );
        // Custom colors from :::/style reached a node.
        assert!(g.nodes.iter().any(|n| n.style.fill.is_some()));
        // One invisible `~~~` link exists in the model...
        assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Invisible));

        // Render: finite canvas, all path coords inside it, no NaN,
        // and the invisible link is NOT drawn as a path.
        let svg = render(&g);
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        let (w, h) = (attr(&svg, "width"), attr(&svg, "height"));
        assert!(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0);
        // Check containment only on edge bezier curves (`M…C…`);
        // cylinder shape paths use arc flags that aren't coordinates.
        for line in paths(&svg).iter().filter(|p| p.contains(" C ")) {
            for (x, y) in coords(line) {
                assert!(x >= -0.5 && x <= w + 0.5, "x={x} outside {w}");
                assert!(y >= -0.5 && y <= h + 0.5, "y={y} outside {h}");
            }
        }
        // Shape variety shows up in the SVG: cylinders/hexagons =>
        // <path>/<polygon>, double circle => 2 close circles, plus
        // the subgraph title strips.
        assert!(svg.contains("<polygon"), "hexagon/parallelogram polygons");
        assert!(svg.matches("<circle").count() >= 2, "double circle");
        assert!(svg.contains("CI/CD Pipeline") && svg.contains("Production Platform"));

        // Interactive path stays consistent: route() over the auto
        // positions yields the same node/edge/cluster counts.
        let s0 = crate::scene::scene(&g);
        let pos: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
        let s1 = crate::scene::route(&g, &pos);
        assert_eq!(s0.nodes.len(), s1.nodes.len());
        assert_eq!(s0.edges.len(), s1.edges.len());
        assert_eq!(s0.clusters.len(), s1.clusters.len());
    }

    #[test]
    fn parallel_edges_are_separated() {
        let g = parse("A -->|x| B\nA -->|y| B").unwrap();
        let svg = render(&g);
        let p = paths(&svg);
        assert_eq!(p.len(), 2);
        assert_ne!(p[0], p[1], "parallel edges must not overlap exactly");
    }

    #[test]
    fn all_curve_points_inside_canvas() {
        let g = parse("flowchart TD\nA --> B\nB --> B\nB --> C\nC --> A").unwrap();
        let svg = render(&g);
        let w = attr(&svg, "width");
        let h = attr(&svg, "height");
        for line in svg.lines().filter(|l| l.starts_with("<path d=")) {
            for (x, y) in coords(line) {
                assert!(x >= -0.5 && x <= w + 0.5, "x={} outside canvas w={}", x, w);
                assert!(y >= -0.5 && y <= h + 0.5, "y={} outside canvas h={}", y, h);
            }
        }
    }

    #[test]
    fn bt_and_rl_directions_do_not_panic() {
        for d in ["BT", "RL"] {
            let src = format!(
                "flowchart {}\nA[Start] --> B{{Check}}\nB -->|yes| C((Ok))\nC --> A",
                d
            );
            let svg = render(&parse(&src).unwrap());
            assert!(svg.contains("Start") && svg.contains("</svg>"));
        }
    }

    #[test]
    fn fanout_exit_points_spread() {
        let g = parse("A[Very Wide Parent Node] --> B\nA --> C\nA --> D").unwrap();
        let svg = render(&g);
        let starts: Vec<(f64, f64)> = paths(&svg).iter().map(|p| coords(p)[0]).collect();
        assert_eq!(starts.len(), 3);
        assert!(
            starts[0] != starts[1] && starts[1] != starts[2] && starts[0] != starts[2],
            "exit points must spread out: {:?}",
            starts
        );
    }

    #[test]
    fn back_edge_routes_around_widest_layer() {
        let src = "flowchart TD\nA --> B1\nA --> B2\nA --> B3\nB1 --> C\nB2 --> C\nB3 --> C\nC --> A";
        let g = parse(src).unwrap();
        let svg = render(&g);
        let back = paths(&svg).pop().unwrap(); // last edge = C --> A
        let c = coords(&back);
        let apex_x = 0.125 * (c[0].0 + c[3].0) + 0.75 * c[1].0;
        let mut min_left = f64::INFINITY;
        let mut max_right = f64::NEG_INFINITY;
        for line in svg.lines().filter(|l| l.starts_with("<rect x=")) {
            let x = attr(line, "x");
            let w = attr(line, "width");
            min_left = min_left.min(x);
            max_right = max_right.max(x + w);
        }
        assert!(
            apex_x > max_right + 8.0 || apex_x < min_left - 8.0,
            "back-edge apex ({:.0}) must clear the nodes [{:.0}..{:.0}]",
            apex_x,
            min_left,
            max_right
        );
    }
}
