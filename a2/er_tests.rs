#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Direction, Document};
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

    /// Every number in an SVG path `d`.
    fn path_nums(line: &str) -> Vec<f64> {
        let i = line.find("d=\"").unwrap() + 3;
        let d = &line[i..line[i..].find('"').unwrap() + i];
        d.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect()
    }

    /// What an ER SVG draws for its relationships: (label boxes, glyph
    /// strokes `M a L b`, glyph circles `(centre, r)`).
    #[allow(clippy::type_complexity)]
    fn drawn_marks(svg: &str) -> (Vec<Rect>, Vec<[(f64, f64); 2]>, Vec<((f64, f64), f64)>) {
        let mut labels = Vec::new();
        let mut strokes = Vec::new();
        let mut circles = Vec::new();
        for l in svg.lines() {
            if l.starts_with("<rect") && l.contains("height=\"20\"") && l.contains("stroke=\"#d5d9ec\"")
            {
                let (x, y) = (attr_f(l, "x"), attr_f(l, "y"));
                labels.push((x, y, x + attr_f(l, "width"), y + attr_f(l, "height")));
            } else if l.starts_with("<path d=\"M ") && l.contains(" L ") && !l.contains(" C ") {
                let n = path_nums(l);
                if n.len() == 4 {
                    strokes.push([(n[0], n[1]), (n[2], n[3])]);
                }
            } else if l.starts_with("<circle") {
                circles.push(((attr_f(l, "cx"), attr_f(l, "cy")), attr_f(l, "r")));
            }
        }
        (labels, strokes, circles)
    }

    /// Report P0 #4: no label box may cover any part of a crow's foot.
    fn assert_no_glyph_under_a_label(svg: &str, what: &str) {
        let (labels, strokes, circles) = drawn_marks(svg);
        assert!(!strokes.is_empty(), "{what}: no glyphs drawn");
        for r in &labels {
            let g = Glyph {
                segments: strokes.clone(),
                circle: None,
            };
            assert!(
                !rect_hits_glyph(*r, &g, 0.0),
                "{what}: label {r:?} covers a glyph stroke"
            );
            for &(c, rad) in &circles {
                let g = Glyph {
                    segments: Vec::new(),
                    circle: Some((c, rad)),
                };
                assert!(
                    !rect_hits_glyph(*r, &g, 0.0),
                    "{what}: label {r:?} covers the circle at {c:?}"
                );
            }
        }
    }

    const CARDS: [&str; 4] = ["||", "|o", "}o", "}|"];
    const CARDS_R: [&str; 4] = ["||", "o|", "o{", "|{"];

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
            for xy in path_nums(line).chunks(2) {
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
        // Left to right, B dragged far right is entered at its left
        // border.
        let d = er("erDiagram\ndirection LR\nA ||--o{ B : owns");
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
    fn route_follows_dragged_entity_top_down() {
        // The default flow: B dragged far down is entered at its top.
        let d = er("erDiagram\nA ||--o{ B : owns");
        let s0 = scene(&d);
        let mut pos: Vec<(f64, f64)> = s0.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        pos[1].1 += 400.0;
        let s1 = route(&d, &pos);
        let end = s1.scene.edges[0].bezier[3];
        let b = &s1.scene.nodes[1];
        assert!(
            (end.1 - (b.y - b.h / 2.0)).abs() < 1.0,
            "relationship must enter at B's top border after the drag"
        );
        assert_no_glyph_under_a_label(&to_svg(&s1), "dragged");
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

    /// Report P0 #4: the corpus label sat on the "zero" circle, so `o{`
    /// read as `|{`; on a line grown just for its label both ends hid.
    #[test]
    fn no_glyph_inside_a_label_rect() {
        assert_no_glyph_under_a_label(&to_svg(&scene(&fixture())), "fixture");
        let one_row = "A {\n  int id PK\n}\nB {\n  int id PK\n}\n";
        for dir in ["TB", "LR", "BT", "RL"] {
            for (l, cl) in CARDS.iter().enumerate() {
                for (r, cr) in CARDS_R.iter().enumerate() {
                    for (line, label) in [("--", "has many"), ("..", "x"), ("--", "a much longer role")] {
                        let src = format!(
                            "erDiagram\ndirection {dir}\nA {cl}{line}{cr} B : \"{label}\"\n{one_row}"
                        );
                        let what = format!("{dir} {l}/{r} {label}");
                        assert_no_glyph_under_a_label(&to_svg(&scene(&er(&src))), &what);
                    }
                }
            }
        }
    }

    #[test]
    fn layout_reserves_room_for_both_glyphs_and_the_label() {
        // One-row tables: the label alone would size the gap, and the
        // glyphs used to land under it by construction.
        let src = "erDiagram\ndirection LR\nA ||--o{ B : \"has many\"\n\
                   A {\n  int id PK\n}\nB {\n  int id PK\n}\n";
        let es = scene(&er(src));
        let (a, b) = (&es.scene.nodes[0], &es.scene.nodes[1]);
        let gap = (b.x - b.w / 2.0) - (a.x + a.w / 2.0);
        let (_, at, w) = es.scene.edges[0].label.clone().unwrap();
        let need = glyph_reach(Card::One) + w + glyph_reach(Card::ZeroMany) + 2.0 * GLYPH_AIR;
        assert!(gap >= need, "gap {gap} < glyphs + label {need}");
        // The label sits between the glyphs, not on them.
        assert!(at.0 - w / 2.0 >= a.x + a.w / 2.0 + glyph_reach(Card::One));
        assert!(at.0 + w / 2.0 <= b.x - b.w / 2.0 - glyph_reach(Card::ZeroMany));
        // Unlabelled, the two widest glyphs still fit facing each other.
        let es = scene(&er("erDiagram\nA }o--o{ B\n"));
        let (a, b) = (&es.scene.nodes[0], &es.scene.nodes[1]);
        let gap = (b.y - b.h / 2.0) - (a.y + a.h / 2.0);
        assert!(gap >= 2.0 * glyph_reach(Card::ZeroMany), "gap {gap}");
    }

    /// Report P0 #8: all 33 attribute comments of the corpus were
    /// silently dropped.
    #[test]
    fn every_attribute_comment_is_drawn() {
        let d = fixture();
        let svg = to_svg(&scene(&d));
        let mut n = 0;
        for a in d.entities.iter().flat_map(|e| &e.attrs) {
            if let Some(c) = &a.comment {
                n += 1;
                let shown = format!(">{}</text>", escape(c));
                assert!(svg.contains(&shown), "comment {c:?} not drawn");
            }
        }
        assert_eq!(n, 33);
    }

    #[test]
    fn comment_column_follows_type_name_and_keys() {
        let src = "erDiagram\nT {\n  uuid id PK \"default gen_random_uuid()\"\n  \
                   varchar(255) slug \"not null\"\n  int n\n}\n";
        let es = scene(&er(src));
        let t = &es.tables[0];
        let n = &es.scene.nodes[0];
        assert_eq!(t.rows[0].comment, "default gen_random_uuid()");
        assert_eq!(t.rows[2].comment, "");
        assert!(t.comment_col_w > 0.0 && t.keys_col_w > 0.0);
        // type | name | keys | comment, each column clear of the last.
        let x0 = n.x - n.w / 2.0;
        assert!(t.name_x() >= PAD + t.ty_col_w + COL_GAP - 1e-9);
        assert!(t.keys_end_x(n.w) - t.keys_col_w >= t.name_x() + t.name_col_w + COL_GAP - 1e-9);
        assert!(t.comment_x() >= t.keys_end_x(n.w) + COL_GAP - 1e-9);
        assert!(t.comment_x() + t.comment_col_w <= n.w - PAD + 1e-9);
        let svg = to_svg(&es);
        let line = svg
            .lines()
            .find(|l| l.contains(">not null</text>"))
            .expect("comment drawn");
        assert!((attr_f(line, "x") - (x0 + t.comment_x())).abs() < 0.06);
        assert!(line.contains("font-style=\"italic\""));
        assert!(line.contains(&format!("fill=\"{TYPE_COLOR}\"")));
        let pk = svg.lines().find(|l| l.contains(">PK</text>")).unwrap();
        assert!((attr_f(pk, "x") - (x0 + t.keys_end_x(n.w))).abs() < 0.06);
        assert!(pk.contains("text-anchor=\"end\""));
    }

    #[test]
    fn table_without_comments_keeps_keys_at_its_border() {
        let es = scene(&er("erDiagram\nT {\n  int id PK\n  int other_id FK\n}\n"));
        let (t, n) = (&es.tables[0], &es.scene.nodes[0]);
        assert_eq!(t.comment_col_w, 0.0);
        assert_eq!(t.keys_end_x(n.w), n.w - PAD);
    }

    #[test]
    fn overlong_comment_is_cut_and_kept_as_tooltip() {
        let long = "a very long comment ".repeat(12);
        let long = long.trim_end();
        let src = format!("erDiagram\nT {{\n  int id PK \"{long}\"\n}}\n");
        let es = scene(&er(&src));
        let row = &es.tables[0].rows[0];
        assert!(row.comment.ends_with('…'));
        assert!(text_width(&row.comment) <= COMMENT_MAX_W);
        assert_eq!(row.comment_full.as_deref(), Some(long));
        assert!(es.scene.nodes[0].w <= COMMENT_MAX_W + 200.0);
        let svg = to_svg(&es);
        assert!(svg.contains(&format!("<title>{long}</title>")));
        // A short comment carries no tooltip.
        let es = scene(&er("erDiagram\nT {\n  int id \"short\"\n}\n"));
        assert_eq!(es.tables[0].rows[0].comment_full, None);
    }

    /// Report P1 / Q13: the layout was hard-coded left to right, which
    /// stacks every source table in one tall column; mermaid's default
    /// is top to bottom.
    #[test]
    fn default_flow_is_top_down_like_mermaid() {
        let d = fixture();
        assert_eq!(d.direction, Direction::TD);
        let es = scene(&d);
        let y = |name: &str| {
            let i = d.entities.iter().position(|e| e.name == name).unwrap();
            es.scene.nodes[i].y - es.scene.nodes[i].h / 2.0
        };
        // categories ||--o{ questions: questions below its parent.
        assert!(y("questions") > y("categories") + es.scene.nodes[0].h / 2.0);
        // The unconnected tables share the first row, not a column.
        let top = |name: &str| {
            let i = d.entities.iter().position(|e| e.name == name).unwrap();
            es.scene.nodes[i].y
        };
        assert_eq!(top("schedules"), top("settings"));
        assert!(es.scene.width > es.scene.height, "landscape, as mermaid");
    }

    #[test]
    fn direction_statement_turns_the_layout() {
        let at = |dir: &str| {
            let es = scene(&er(&format!("erDiagram\ndirection {dir}\nA ||--o{{ B\n")));
            let (a, b) = (&es.scene.nodes[0], &es.scene.nodes[1]);
            (b.x - a.x, b.y - a.y)
        };
        let (dx, dy) = at("LR");
        assert!(dx > 0.0 && dy.abs() < 1.0, "LR: B right of A");
        let (dx, dy) = at("RL");
        assert!(dx < 0.0 && dy.abs() < 1.0, "RL: B left of A");
        let (dx, dy) = at("TB");
        assert!(dy > 0.0 && dx.abs() < 1.0, "TB: B below A");
        let (dx, dy) = at("BT");
        assert!(dy < 0.0 && dx.abs() < 1.0, "BT: B above A");
    }

    #[test]
    fn alias_is_drawn_in_place_of_the_name() {
        let d = er("erDiagram\np[Person] {\n  string name\n}\nc[\"Customer Account\"]\np ||--o{ c : has\n");
        let es = scene(&d);
        assert_eq!(es.tables[0].name, "p");
        assert_eq!(es.tables[0].title, "Person");
        assert_eq!(es.tables[1].title, "Customer Account");
        // Identity stays the entity name; the label is what is shown.
        assert_eq!(es.scene.nodes[0].id, "p");
        assert_eq!(es.scene.nodes[0].label, "Person");
        let svg = to_svg(&es);
        assert!(svg.contains(">Person</text>") && svg.contains(">Customer Account</text>"));
        assert!(!svg.contains(">p</text>"));
    }

    /// Report P0 #6 (ER half): a rerouted relationship arrives with its
    /// route in `waypoints` and a degenerate `bezier` chord; the writer
    /// drew the chord straight through whatever it was routed around,
    /// and the glyphs, aimed at a control point ON the end, collapsed.
    #[test]
    fn rerouted_relationship_follows_its_waypoints() {
        let mut es = scene(&er("erDiagram\ndirection LR\nA ||--o{ B\n"));
        let e = &mut es.scene.edges[0];
        let (p0, p3) = (e.bezier[0], e.bezier[3]);
        // A detour above the straight line, leaving and entering level,
        // densely sampled as the router delivers its routes.
        let up = p0.1 - 80.0;
        let corners = [
            p0,
            (p0.0 + 40.0, p0.1),
            (p0.0 + 40.0, up),
            (p3.0 - 40.0, up),
            (p3.0 - 40.0, p3.1),
            p3,
        ];
        let mut wps = vec![p0];
        for s in corners.windows(2) {
            let n = ((s[1].0 - s[0].0).hypot(s[1].1 - s[0].1) / 4.0).ceil();
            for k in 1..=n as usize {
                let t = k as f64 / n;
                wps.push((s[0].0 + (s[1].0 - s[0].0) * t, s[0].1 + (s[1].1 - s[0].1) * t));
            }
        }
        e.waypoints = wps;
        e.bezier = [p0, p0, p3, p3];
        let svg = to_svg(&es);
        let line = svg
            .lines()
            .find(|l| l.starts_with("<path") && l.contains(" C ") && !l.contains("fill=\"#"))
            .expect("relationship path");
        let ys: Vec<f64> = path_nums(line).chunks(2).map(|p| p[1]).collect();
        assert!(
            ys.iter().any(|&y| y < p0.1 - 40.0),
            "path must follow the detour: {line}"
        );
        // Both crow's feet lie along the level stubs: full-size, ticks
        // across (vertical), prongs reaching back to the border.
        let (_, strokes, circles) = drawn_marks(&svg);
        for [a, b] in &strokes {
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            assert!(len > 10.0, "collapsed glyph stroke {a:?}-{b:?}");
        }
        assert_eq!(circles.len(), 1);
        assert!((circles[0].0 .1 - p3.1).abs() < 0.11, "circle on the stub");
    }

    #[test]
    fn header_band_is_square_where_it_meets_the_body() {
        let svg = to_svg(&scene(&er("erDiagram\nT {\n  int id\n}\n")));
        let accent = crate::style::accent(0);
        let band = svg
            .lines()
            .find(|l| l.starts_with("<path") && l.contains(&format!("fill=\"{accent}\"")))
            .expect("header band path");
        // Two curves (the top corners), then straight down and across.
        assert_eq!(band.matches(" Q ").count(), 2, "{band}");
        // The accent outline is painted after the band, on top.
        let band_at = svg.find(band).unwrap();
        let outline = format!("fill=\"none\" stroke=\"{accent}\"");
        assert!(svg[band_at..].contains(&outline), "outline after band");
        // A title-only table is all band: rounded all round.
        let svg = to_svg(&scene(&er("erDiagram\nT\n")));
        assert!(svg.contains(&format!("rx=\"4\" fill=\"{accent}\"")));
    }

    #[test]
    fn glyphs_paint_over_the_tables() {
        let svg = to_svg(&scene(&er("erDiagram\nA ||--o{ B\n")));
        let last_table = svg.rfind("fill=\"none\" stroke=\"#").unwrap();
        let first_glyph = svg.find("<circle").unwrap();
        assert!(first_glyph > last_table);
    }

    #[test]
    fn glyph_sizes_and_reach() {
        // Bars 15px across, forks 16 deep: readable next to 22px rows.
        let g = glyph((0.0, 0.0), (1.0, 0.0), Card::One);
        let [a, b] = g.segments[0];
        assert!(((b.1 - a.1).abs() - 2.0 * TICK_HALF).abs() < 1e-9);
        let g = glyph((0.0, 0.0), (1.0, 0.0), Card::ZeroMany);
        assert_eq!(g.segments[0][0], (FORK_LEN, 0.0));
        let ((cx, _), r) = g.circle.unwrap();
        // The circle sits past the fork, never on it.
        assert!(cx - r > FORK_LEN);
        assert!((glyph_reach(Card::ZeroMany) - (cx + r + LINE_W / 2.0)).abs() < 1e-9);
        // Every cardinality reaches past its farthest stroke.
        for c in [Card::One, Card::ZeroOne, Card::ZeroMany, Card::OneMany] {
            let g = glyph((0.0, 0.0), (1.0, 0.0), c);
            let far = g.segments.iter().flatten().map(|p| p.0).fold(0.0, f64::max);
            assert!(glyph_reach(c) > far, "{c:?}");
        }
    }

    /// The label must stay ON its line wherever the layout sized the
    /// gap: a reserve without the glyph air pushed `contains` beside a
    /// 60px TD line that fit the label and both glyphs only touching.
    #[test]
    fn laid_out_labels_stay_on_their_line() {
        let src = "erDiagram
CUSTOMER only one to zero or more ORDER : places
                   ORDER one or more optionally to one ITEM : contains
";
        let es = scene(&er(src));
        for e in &es.scene.edges {
            let (text, at, _) = e.label.clone().unwrap();
            let d = edge_polyline(e)
                .iter()
                .map(|p| (p.0 - at.0).hypot(p.1 - at.1))
                .fold(f64::INFINITY, f64::min);
            assert!(d < 1.5, "{text:?} left its line by {d}");
        }
        assert_no_glyph_under_a_label(&to_svg(&es), "text cardinalities");
    }

    #[test]
    fn crowded_label_still_clears_glyphs_after_a_tight_drag() {
        // Tables dragged almost together leave no free stretch: the
        // label steps beside the line rather than over a glyph.
        let d = er("erDiagram\ndirection LR\nA ||--o{ B : \"has many\"\n");
        let s0 = scene(&d);
        let a = &s0.scene.nodes[0];
        let b = &s0.scene.nodes[1];
        let pos = vec![(a.x, a.y), (a.x + a.w / 2.0 + b.w / 2.0 + 40.0, a.y)];
        let s1 = route(&d, &pos);
        assert_no_glyph_under_a_label(&to_svg(&s1), "tight drag");
    }
}
