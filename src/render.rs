//! SVG wrapper: `render(graph)` = automatic scene -> SVG serialisation.
//! All geometry & serialisation logic lives in the `scene` module,
//! so the output of render() and the interactive flow
//! (scene/route/to_svg) are guaranteed identical.

use crate::model::{
    Architecture, ClassDiagram, ErDiagram, GitGraph, Graph, Journey, Mindmap, PieChart,
    SankeyDiagram, SequenceDiagram, Timeline,
};
use crate::scene::{A11y, SvgOptions};

/// A graph's `accTitle` / `accDescr`, as the SVG writer takes them.
pub(crate) fn graph_a11y(g: &Graph) -> A11y {
    A11y::new(g.acc_title.clone(), g.acc_descr.clone())
}

pub fn render(g: &Graph) -> String {
    render_titled(g, "Flowchart diagram")
}

/// [`render`] with explicit viewport options (see [`SvgOptions`]).
pub fn render_with(g: &Graph, opts: &SvgOptions) -> String {
    render_titled_with(g, "Flowchart diagram", opts)
}

/// [`render`] with a caller-chosen accessible name (a `stateDiagram-v2`
/// rides the flowchart `Graph` but should announce "State diagram").
/// A non-empty `g.acc_title` (mermaid's `accTitle:`) still wins over
/// `title`, and `g.acc_descr` becomes the SVG's `<desc>`.
pub fn render_titled(g: &Graph, title: &str) -> String {
    render_titled_with(g, title, &SvgOptions::default())
}

/// [`render_titled`] with explicit viewport options.
pub fn render_titled_with(g: &Graph, title: &str, opts: &SvgOptions) -> String {
    crate::scene::to_svg_a11y_with(&crate::scene::scene(g), title, &graph_a11y(g), opts)
}

/// Render an Entity-Relationship diagram (tables + crow's foot).
/// Geometry & serialisation live in the `er` module.
pub fn render_er(d: &ErDiagram) -> String {
    render_er_with(d, &SvgOptions::default())
}

/// [`render_er`] with explicit viewport options.
pub fn render_er_with(d: &ErDiagram, opts: &SvgOptions) -> String {
    let a11y = A11y::new(d.acc_title.clone(), d.acc_descr.clone());
    crate::er::to_svg_a11y_with(&crate::er::scene(d), &a11y, opts)
}

/// Render a UML class diagram (three-compartment boxes + UML end
/// glyphs). Geometry & serialisation live in the `class` module.
pub fn render_class(d: &ClassDiagram) -> String {
    render_class_with(d, &SvgOptions::default())
}

/// [`render_class`] with explicit viewport options.
pub fn render_class_with(d: &ClassDiagram, opts: &SvgOptions) -> String {
    let a11y = A11y::new(d.acc_title.clone(), d.acc_descr.clone());
    crate::class::to_svg_a11y_with(&crate::class::scene(d), &a11y, opts)
}

/// Render a pie chart (proportional slices + legend). Geometry &
/// serialisation live in the `pie` module.
pub fn render_pie(d: &PieChart) -> String {
    render_pie_with(d, &SvgOptions::default())
}

/// [`render_pie`] with explicit viewport options.
pub fn render_pie_with(d: &PieChart, opts: &SvgOptions) -> String {
    let a11y = A11y::new(d.acc_title.clone(), d.acc_descr.clone());
    crate::pie::to_svg_a11y_with(&crate::pie::scene(d), &a11y, opts)
}

/// Render a sequence diagram (lifelines + linear message rows).
/// Geometry & serialisation live in the `seq` module.
pub fn render_seq(d: &SequenceDiagram) -> String {
    render_seq_with(d, &SvgOptions::default())
}

/// [`render_seq`] with explicit viewport options.
pub fn render_seq_with(d: &SequenceDiagram, opts: &SvgOptions) -> String {
    let a11y = A11y::new(d.acc_title.clone(), d.acc_descr.clone());
    crate::seq::to_svg_a11y_with(&crate::seq::scene(d), &a11y, opts)
}

/// Render a mindmap (radial: the root at the centre, children fanned
/// round their parents, colored branches). Geometry & serialisation
/// live in the `mindmap` module.
pub fn render_mindmap(d: &Mindmap) -> String {
    crate::mindmap::to_svg(&crate::mindmap::scene(d))
}

/// [`render_mindmap`] with explicit viewport options.
pub fn render_mindmap_with(d: &Mindmap, opts: &SvgOptions) -> String {
    crate::mindmap::to_svg_with(&crate::mindmap::scene(d), opts)
}

/// Render a user-journey (horizontal band of score-colored faces).
/// Geometry & serialisation live in the `journey` module.
pub fn render_journey(d: &Journey) -> String {
    render_journey_with(d, &SvgOptions::default())
}

/// [`render_journey`] with explicit viewport options.
pub fn render_journey_with(d: &Journey, opts: &SvgOptions) -> String {
    let a11y = A11y::new(d.acc_title.clone(), d.acc_descr.clone());
    crate::journey::to_svg_a11y_with(&crate::journey::scene(d), &a11y, opts)
}

/// Render a git graph (branch lanes + commit circles + merge curves).
/// Geometry & serialisation live in the `gitgraph` module.
pub fn render_gitgraph(d: &GitGraph) -> String {
    crate::gitgraph::to_svg(&crate::gitgraph::scene(d))
}

/// [`render_gitgraph`] with explicit viewport options.
pub fn render_gitgraph_with(d: &GitGraph, opts: &SvgOptions) -> String {
    crate::gitgraph::to_svg_with(&crate::gitgraph::scene(d), opts)
}

/// Render an architecture-beta diagram (nested groups + services + edges).
/// Geometry & serialisation live in the `architecture` module.
pub fn render_architecture(d: &Architecture) -> String {
    render_architecture_with(d, &SvgOptions::default())
}

/// [`render_architecture`] with explicit viewport options.
pub fn render_architecture_with(d: &Architecture, opts: &SvgOptions) -> String {
    let a11y = A11y::new(d.acc_title.clone(), d.acc_descr.clone());
    crate::architecture::to_svg_a11y_with(&crate::architecture::scene(d), &a11y, opts)
}

/// Render a timeline (period columns on an axis, events stacked beneath,
/// colored section bands). Geometry & serialisation live in the
/// `timeline` module.
pub fn render_timeline(d: &Timeline) -> String {
    render_timeline_with(d, &SvgOptions::default())
}

/// [`render_timeline`] with explicit viewport options.
pub fn render_timeline_with(d: &Timeline, opts: &SvgOptions) -> String {
    let a11y = A11y::new(d.acc_title.clone(), d.acc_descr.clone());
    crate::timeline::to_svg_a11y_with(&crate::timeline::scene(d), &a11y, opts)
}

/// Render a sankey diagram (nodes in columns, flows as ribbons whose
/// thickness tracks their value). Geometry & serialisation live in the
/// `sankey` module.
pub fn render_sankey(d: &SankeyDiagram) -> String {
    crate::sankey::to_svg(&crate::sankey::scene(d))
}

/// [`render_sankey`] with explicit viewport options.
pub fn render_sankey_with(d: &SankeyDiagram, opts: &SvgOptions) -> String {
    crate::sankey::to_svg_with(&crate::sankey::scene(d), opts)
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
        assert_eq!(
            workers.parent,
            Some(g.subgraphs.iter().position(|s| s.id == "Platform").unwrap())
        );
        assert_eq!(workers.direction, Some(crate::model::Direction::LR));
        // Edges targeting a subgraph landed in sub_edges (Dev->CI,
        // Reg->Platform, Platform->Mon, Rollback->Platform).
        assert!(
            g.sub_edges.len() >= 4,
            "got {} sub_edges",
            g.sub_edges.len()
        );

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

    /// End-to-end showcase for stateDiagram-v2 riding the flowchart
    /// pipeline: pseudostates, composite with its own [*]s and
    /// direction, choice/fork/join, labelled transitions, sub-edge
    /// to a composite box — rendered with no NaN and full containment.
    #[test]
    fn state_showcase_parses_and_renders() {
        use crate::model::{Document, End, Shape};
        let src = include_str!("../examples/state.mmd");
        let Document::State(g) = crate::parser::parse_document(src).unwrap() else {
            panic!("state.mmd is a state diagram");
        };
        let shapes: Vec<Shape> = g.nodes.iter().map(|n| n.shape).collect();
        for want in [
            Shape::StateStart,
            Shape::StateEnd,
            Shape::Diamond,
            Shape::ForkBar,
        ] {
            assert!(shapes.contains(&want), "missing {want:?}");
        }
        assert_eq!(g.subgraphs.len(), 1, "Validating composite");
        assert_eq!(g.subgraphs[0].direction, Some(crate::model::Direction::LR));
        // Idle --> Validating and Validating --> decide touch the box.
        assert!(g.sub_edges.iter().any(|e| matches!(e.to, End::Sub(0))));
        assert!(g.sub_edges.iter().any(|e| matches!(e.from, End::Sub(0))));
        // The description replaced Idle's label.
        assert!(g.nodes.iter().any(|n| n.label == "waiting for input"));

        let svg = render(&g);
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.contains("Validating") && svg.contains(">submit</text>"));
        // The note is drawn (it used to vanish without a trace), its
        // link named after the card, the choice and bars after their ids.
        assert!(svg.contains(">shown to the user</text>"));
        assert!(svg.contains("<title>Rejected → note</title>"));
        for id in ["decide", "forked", "joined"] {
            assert!(svg.contains(&format!("<g><title>{id}</title>")), "{id}");
        }
        let (w, h) = (attr(&svg, "width"), attr(&svg, "height"));
        for line in paths(&svg).iter().filter(|p| p.contains(" C ")) {
            for (x, y) in coords(line) {
                assert!(x >= -0.5 && x <= w + 0.5, "x={x} outside {w}");
                assert!(y >= -0.5 && y <= h + 0.5, "y={y} outside {h}");
            }
        }
        // Interactive parity: route() over auto positions keeps counts.
        let s0 = crate::scene::scene(&g);
        let pos: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
        let s1 = crate::scene::route(&g, &pos);
        assert_eq!(s0.nodes.len(), s1.nodes.len());
        assert_eq!(s0.edges.len(), s1.edges.len());
    }

    /// State notes ride the flowchart pipeline as boxes of their own:
    /// placed by `right of` / `left of` along the flow like mermaid,
    /// joined by a dotted line, and picked, dragged and rerouted like
    /// any node.
    #[test]
    fn state_note_cards_are_placed_and_interactive() {
        use crate::model::Document;
        use crate::scene::{route, route_partial, scene, Hit};
        let src = "stateDiagram-v2\n[*] --> A\nA --> B\nB --> C\nC --> [*]\n\
                   note right of A : after A\nnote left of C : before C";
        let Document::State(g) = crate::parser::parse_document(src).unwrap() else {
            panic!("a state diagram");
        };
        let sc = scene(&g);
        let at = |id: &str| &sc.nodes[g.node_index(id).unwrap()];
        let card = |text: &str| sc.nodes.iter().position(|n| n.label == text).unwrap();
        let (after, before) = (card("after A"), card("before C"));
        let (a, b, c) = (at("A"), at("B"), at("C"));
        // `right of A` sits on B's layer (after A); `left of C` on B's
        // layer too (just before C), not parked on the top layer.
        assert!((sc.nodes[after].y - b.y).abs() < 1.0, "right of: after A");
        assert!((sc.nodes[before].y - b.y).abs() < 1.0, "left of: before C");
        assert!(a.y < b.y && b.y < c.y);
        // The link is drawn dotted, without an arrowhead.
        let svg = render(&g);
        let link = svg
            .lines()
            .find(|l| l.contains("<title>A → note</title>"))
            .expect("the note link is drawn");
        assert!(link.contains("stroke-dasharray") && !link.contains("marker-end"));
        // Interactive: a card is picked where it is drawn, and dragging
        // it reroutes its link from the new spot.
        let n = &sc.nodes[after];
        assert_eq!(sc.hit_test(n.x, n.y, 4.0), Some(Hit::Node(after)));
        let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut moved = base.clone();
        // Down and left, into free space below B.
        moved[after].0 -= 100.0;
        moved[after].1 += 110.0;
        assert_eq!(sc.node_at(moved[after].0, moved[after].1), None);
        let link_of = |s: &crate::scene::Scene| {
            s.edges
                .iter()
                .find(|e| e.to == g.nodes[after].id)
                .map(|e| e.bezier[3])
                .unwrap()
        };
        for s in [route(&g, &moved), route_partial(&g, &moved, &sc, &base)] {
            assert_eq!(s.node_at(moved[after].0, moved[after].1), Some(after));
            let end = link_of(&s);
            assert!((end.0 - moved[after].0).abs() < n.w / 2.0 + 0.5, "{end:?}");
        }
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
        let src =
            "flowchart TD\nA --> B1\nA --> B2\nA --> B3\nB1 --> C\nB2 --> C\nB3 --> C\nC --> A";
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
