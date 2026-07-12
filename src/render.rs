//! SVG wrapper: `render(graph)` = automatic scene -> SVG serialisation.
//! All geometry & serialisation logic lives in the `scene` module,
//! so the output of render() and the interactive flow
//! (scene/route/to_svg) are guaranteed identical.

use crate::model::{ErDiagram, Graph};

pub fn render(g: &Graph) -> String {
    crate::scene::to_svg(&crate::scene::scene(g))
}

/// Render an Entity-Relationship diagram (tables + crow's foot).
/// Geometry & serialisation live in the `er` module.
pub fn render_er(d: &ErDiagram) -> String {
    crate::er::to_svg(&crate::er::scene(d))
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
        let nums: Vec<f64> = d
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        nums.chunks(2).map(|c| (c[0], c[1])).collect()
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
