#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn er(src: &str) -> ErDiagram {
        match parse_document(src).unwrap() {
            Document::Er(d) => d,
            other => panic!("expected ER, got {:?}", other),
        }
    }

    fn fixture() -> ErDiagram {
        er(include_str!("../examples/er.mmd"))
    }

    fn attr_f(line: &str, name: &str) -> f64 {
        let pat = format!("{}=\"", name);
        let i = line.find(&pat).unwrap() + pat.len();
        let rest = &line[i..];
        rest[..rest.find('"').unwrap()].parse().unwrap()
    }

    #[test]
    fn fixture_renders_tables_keys_and_label() {
        let svg = to_svg(&scene(&fixture()));
        for name in ["categories", "questions", "schedules", "settings"] {
            assert!(svg.contains(name), "missing entity {}", name);
        }
        assert!(svg.contains(">PK</text>"), "PK tag must be visible");
        assert!(svg.contains(">FK</text>"), "FK tag must be visible");
        assert!(
            svg.contains(">has</text>"),
            "relationship label must render"
        );
        // The single fixture relationship is identifying: solid line.
        assert!(!svg.contains("stroke-dasharray"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn all_geometry_stays_inside_canvas() {
        let svg = to_svg(&scene(&fixture()));
        let head = svg.lines().next().unwrap();
        let w = attr_f(head, "width");
        let h = attr_f(head, "height");
        for line in svg.lines().filter(|l| l.starts_with("<path d=")) {
            let i = line.find("d=\"").unwrap() + 3;
            let d = &line[i..line[i..].find('"').unwrap() + i];
            let nums: Vec<f64> = d
                .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter(|s| !s.is_empty())
                .map(|s| s.parse().unwrap())
                .collect();
            for xy in nums.chunks(2) {
                assert!(
                    xy[0] >= -0.5 && xy[0] <= w + 0.5,
                    "x={} outside w={}",
                    xy[0],
                    w
                );
                assert!(
                    xy[1] >= -0.5 && xy[1] <= h + 0.5,
                    "y={} outside h={}",
                    xy[1],
                    h
                );
            }
        }
        for line in svg.lines().filter(|l| l.starts_with("<circle")) {
            let (cx, cy) = (attr_f(line, "cx"), attr_f(line, "cy"));
            assert!(cx >= 0.0 && cx <= w && cy >= 0.0 && cy <= h);
        }
    }

    #[test]
    fn relation_only_entity_renders_as_title_only_table() {
        let svg = to_svg(&scene(&er("erDiagram\nA ||--o{ B : owns")));
        assert!(svg.contains(">A</text>") && svg.contains(">B</text>"));
        assert!(svg.contains(">owns</text>"));
    }

    #[test]
    fn non_identifying_relationship_is_dashed() {
        let svg = to_svg(&scene(&er("erDiagram\nA ||..o{ B")));
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn zero_one_draws_circle() {
        let svg = to_svg(&scene(&er("erDiagram\nA |o--|{ B")));
        assert!(svg.contains("<circle"), "ZeroOne side needs a circle");
    }

    #[test]
    fn route_follows_dragged_entity() {
        let d = er("erDiagram\nA ||--o{ B : owns");
        let s0 = scene(&d);
        let mut pos: Vec<(f64, f64)> = s0.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        pos[1].0 += 500.0; // drag B far right
        let s1 = route(&d, &pos);
        assert_eq!(s1.scene.nodes[1].x, pos[1].0);
        let end = s1.scene.edges[0].bezier[3];
        let b = &s1.scene.nodes[1];
        assert!(
            (end.0 - (b.x - b.w / 2.0)).abs() < 1.0,
            "relationship must enter at B's left border after the drag"
        );
        // Export of the dragged arrangement keeps working.
        assert!(to_svg(&s1).ends_with("</svg>\n"));
    }

    #[test]
    fn drag_past_origin_stays_inside_viewbox() {
        // A table dragged left of and/or above the canvas origin (the
        // playground's drag path) must not be clipped by the `0 0 W H`
        // viewBox — nor may any text, edge or marker it pulls along.
        // The last case keeps the centre in bounds but the box past it.
        let d = fixture();
        let sc = scene(&d);
        let laid: Vec<(f64, f64)> = sc.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let (x0, y0) = laid[0];
        for to in [(-120.0, y0), (x0, -80.0), (-120.0, -80.0), (5.0, 5.0)] {
            let mut centers = laid.clone();
            centers[0] = to;
            crate::scene::assert_drawn_inside_canvas(&to_svg(&route(&d, &centers)));
        }
        // Laid out, the canvas already holds everything.
        crate::scene::assert_drawn_inside_canvas(&to_svg(&sc));
    }

    #[test]
    fn laid_out_scene_is_not_shifted() {
        // The shift only kicks in past the origin: a laid-out scene
        // renders byte-for-byte as before.
        let sc = scene(&fixture());
        assert!(matches!(
            fit_top_left(&sc.scene),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    /// Report Q5: entity headers hard-coded white text on their accent,
    /// below WCAG AA on six of the eight (amber 2.6:1, green 3.2:1).
    #[test]
    fn header_text_reads_on_every_accent() {
        let names: Vec<String> = (0..crate::style::ACCENTS.len())
            .map(|i| format!("ENT{i}"))
            .collect();
        let src: String = names
            .iter()
            .map(|n| format!("{n} {{\n  int id\n}}\n"))
            .collect();
        let svg = crate::render_svg(&format!("erDiagram\n{src}")).unwrap();
        for (i, n) in names.iter().enumerate() {
            let accent = crate::style::accent(i);
            let ink = crate::style::assert_text_reads(&svg, n, accent);
            assert_eq!(ink, crate::style::text_on(accent), "{n}");
        }
    }
}
