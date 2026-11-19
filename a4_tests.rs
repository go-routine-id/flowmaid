#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn sd(src: &str) -> SequenceDiagram {
        match parse_document(src).unwrap() {
            Document::Sequence(d) => d,
            other => panic!("expected a sequence diagram, got {:?}", other),
        }
    }

    // ----------------------------- parsing -----------------------------

    #[test]
    fn participants_aliases_and_actors() {
        let d =
            sd("sequenceDiagram\nparticipant A as Alice\nactor B as Bob the User\nparticipant C");
        let got: Vec<(&str, &str, bool)> = d
            .participants
            .iter()
            .map(|p| (p.id.as_str(), p.label.as_str(), p.actor))
            .collect();
        assert_eq!(
            got,
            [
                ("A", "Alice", false),
                ("B", "Bob the User", true),
                ("C", "C", false)
            ]
        );
    }

    #[test]
    fn implicit_participants_appear_in_message_order() {
        let d = sd("sequenceDiagram\nA->>B: x\nC->>A: y");
        let ids: Vec<&str> = d.participants.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["A", "B", "C"]);
        // A later declaration updates an implicit participant in place.
        let d = sd("sequenceDiagram\nA->>B: x\nactor B as Bee");
        assert_eq!(d.participants[1].label, "Bee");
        assert!(d.participants[1].actor);
    }

    #[test]
    fn all_eight_arrow_ops_parse() {
        let cases = [
            ("->>", false, SeqHead::Filled),
            ("-->>", true, SeqHead::Filled),
            ("->", false, SeqHead::None),
            ("-->", true, SeqHead::None),
            ("-x", false, SeqHead::Cross),
            ("--x", true, SeqHead::Cross),
            ("-)", false, SeqHead::Async),
            ("--)", true, SeqHead::Async),
        ];
        for (op, want_dashed, want_head) in cases {
            let d = sd(&format!("sequenceDiagram\nA{op}B: hi"));
            let SeqItem::Message {
                text, dashed, head, ..
            } = &d.items[0]
            else {
                panic!("expected a message for op {op}");
            };
            assert_eq!(text, "hi", "op: {op}");
            assert_eq!(*dashed, want_dashed, "op: {op}");
            assert_eq!(*head, want_head, "op: {op}");
        }
    }

    #[test]
    fn activation_shorthand_and_keywords() {
        let d = sd("sequenceDiagram\nA->>+B: q\nB-->>-A: r\nactivate A\ndeactivate A");
        let SeqItem::Message {
            activate,
            deactivate,
            ..
        } = &d.items[0]
        else {
            panic!()
        };
        assert!(*activate && !*deactivate, "`+` activates the target");
        let SeqItem::Message {
            activate,
            deactivate,
            ..
        } = &d.items[1]
        else {
            panic!()
        };
        assert!(!*activate && *deactivate, "`-` deactivates the sender");
        assert!(matches!(d.items[2], SeqItem::Activate(0)));
        assert!(matches!(d.items[3], SeqItem::Deactivate(0)));
    }

    #[test]
    fn unbalanced_deactivate_is_a_line_error() {
        let e = parse_document("sequenceDiagram\nA->>B: hi\ndeactivate B").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(
            e.message.contains("without a matching activate"),
            "{}",
            e.message
        );
        // `-` shorthand deactivates the sender — B was never activated.
        let e = parse_document("sequenceDiagram\nA->>B: hi\nB-->>-A: bye").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("not activated"), "{}", e.message);
    }

    #[test]
    fn autonumber_and_all_note_forms() {
        let d = sd(
            "sequenceDiagram\nautonumber\nA->>B: one\nNote over A,B: pair\n\
             note over A: solo\nNote left of A: l\nNOTE RIGHT OF B: r",
        );
        assert!(d.autonumber);
        let sides: Vec<NoteSide> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::Note { side, .. } => Some(*side),
                _ => None,
            })
            .collect();
        assert_eq!(
            sides,
            [
                NoteSide::Over(0, Some(1)),
                NoteSide::Over(0, None),
                NoteSide::LeftOf(0),
                NoteSide::RightOf(1),
            ]
        );
        // Rejecting autonumber arguments keeps the claim honest.
        assert!(parse_document("sequenceDiagram\nautonumber 10").is_err());
    }

    #[test]
    fn trailing_comment_is_stripped() {
        let d = sd("sequenceDiagram\nA->>B: hi %% trailing comment");
        let SeqItem::Message { text, .. } = &d.items[0] else {
            panic!()
        };
        assert_eq!(text, "hi");
    }

    #[test]
    fn note_over_same_participant_collapses_to_single() {
        // Bug hunt: `Note over A,A` used to become a zero-width span
        // Over(0, Some(0)); it must normalise to a single-participant
        // note Over(0, None).
        let d = sd("sequenceDiagram\nparticipant A\nNote over A,A: same");
        assert!(matches!(
            d.items[0],
            SeqItem::Note {
                side: NoteSide::Over(0, None),
                ..
            }
        ));
    }

    #[test]
    fn frames_parse_and_validate() {
        let d = sd(
            "sequenceDiagram\nalt ok\nA->>B: x\nelse bad\nB->>A: y\nend\n\
             par one\nA->>B: p\nand two\nA->>B: q\nend\n\
             loop each\nA->>B: l\nend\nopt maybe\nA->>B: o\nend",
        );
        let kinds: Vec<FrameKind> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::FrameStart { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            [
                FrameKind::Alt,
                FrameKind::Par,
                FrameKind::Loop,
                FrameKind::Opt
            ]
        );
        let elses = d
            .items
            .iter()
            .filter(|i| matches!(i, SeqItem::FrameElse { .. }))
            .count();
        let ends = d
            .items
            .iter()
            .filter(|i| matches!(i, SeqItem::FrameEnd))
            .count();
        assert_eq!((elses, ends), (2, 4));

        let e = parse_document("sequenceDiagram\nend").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("without an open"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nloop x\nA->>B: y").unwrap_err();
        assert_eq!(e.line, 2, "unclosed frame reports its opening line");
        assert!(e.message.contains("never closed"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nloop x\nelse y\nend").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("'alt'"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nalt x\nand y\nend").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("'par'"), "{}", e.message);
    }

    #[test]
    fn unsupported_sequence_elements_error_cleanly() {
        for src in [
            "sequenceDiagram\nrect rgb(0,0,0)",
            "sequenceDiagram\nbox Purple team",
            "sequenceDiagram\ncreate participant D",
            "sequenceDiagram\nbreak overload",
        ] {
            let e = parse_document(src).unwrap_err();
            assert_eq!(e.line, 2, "src: {src}");
            assert!(e.message.contains("not supported yet"), "{}", e.message);
        }
    }

    #[test]
    fn message_errors_have_line_numbers() {
        let e = parse_document("sequenceDiagram\nA->>B hi").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("expected ':"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nA==>B: hi").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(
            e.message.contains("unknown message operator"),
            "{}",
            e.message
        );
    }

    // ----------------------------- layout -----------------------------

    #[test]
    fn columns_follow_declaration_order_and_rows_increase() {
        let d = sd("sequenceDiagram\nA->>B: x\nC->>A: a much longer label here\nB-->>C: z");
        let sc = scene(&d);
        assert_eq!(sc.boxes.len(), 3);
        for i in 1..sc.boxes.len() {
            let (a, b) = (&sc.boxes[i - 1], &sc.boxes[i]);
            assert!(a.x + a.w < b.x, "box {i} must sit right of box {}", i - 1);
        }
        let ys: Vec<f64> = sc.messages.iter().map(|m| m.points[0].1).collect();
        for i in 1..ys.len() {
            assert!(ys[i] > ys[i - 1], "message rows must descend: {ys:?}");
        }
        // Lifelines run under the box centers.
        for (b, l) in sc.boxes.iter().zip(&sc.lifelines) {
            assert!((b.x + b.w / 2.0 - l.x).abs() < 1e-6);
            assert!(l.y1 > l.y0);
        }
    }

    #[test]
    fn self_message_stays_right_of_its_lifeline() {
        let d = sd("sequenceDiagram\nA->>A: think about it carefully\nA->>B: go");
        let sc = scene(&d);
        let m = &sc.messages[0];
        let lx = sc.lifelines[0].x;
        assert_eq!(m.points.len(), 4, "self-message is a loop");
        for &(px, _) in &m.points {
            assert!(px >= lx - 1e-6, "loop must not cross left of the lifeline");
        }
        assert!(m.label_pos.0 > lx, "label sits right of the lifeline");
        assert!(!m.label_centered);
        // The head points back into the lifeline at the loop's end.
        assert!((m.points[3].0 - lx).abs() < 1e-6);
        // Label fits the canvas and clears the next column's box.
        let w = label_width(m.number, &m.text);
        assert!(m.label_pos.0 + w <= sc.width);
        assert!(m.label_pos.0 + w <= sc.boxes[1].x);
    }

    #[test]
    fn long_labels_widen_the_gap() {
        let d = sd(
            "sequenceDiagram\nA->>B: an extremely long message label that must fit between lifelines",
        );
        let sc = scene(&d);
        let m = &sc.messages[0];
        let span = (m.points[1].0 - m.points[0].0).abs();
        assert!(
            span >= label_width(m.number, &m.text),
            "span {span} too small for the label"
        );
    }

    #[test]
    fn activation_bars_span_activate_to_deactivate() {
        let d = sd("sequenceDiagram\nA->>+B: q\nB->>B: work\nB-->>-A: r");
        let sc = scene(&d);
        assert_eq!(sc.activations.len(), 1);
        let bar = &sc.activations[0];
        assert_eq!(bar.participant, 1);
        assert!(
            (bar.y0 - sc.messages[0].points[0].1).abs() < 1e-6,
            "starts at the + message"
        );
        assert!(
            (bar.y1 - sc.messages[2].points[0].1).abs() < 1e-6,
            "ends at the - message"
        );
        assert!((bar.x - sc.lifelines[1].x).abs() < 1e-6);
        // An unclosed activation extends to the lifeline bottom.
        let sc = scene(&sd("sequenceDiagram\nA->>+B: q"));
        assert_eq!(sc.activations.len(), 1);
        assert!((sc.activations[0].y1 - sc.lifelines[1].y1).abs() < 1e-6);
    }

    // ----------------------------- SVG -----------------------------

    #[test]
    fn head_glyph_geometry() {
        let tip = (100.0, 50.0);
        let back = (0.0, 50.0);
        assert_eq!(head(tip, back, SeqHead::Filled).polygon.len(), 3);
        assert_eq!(head(tip, back, SeqHead::Cross).segments.len(), 2);
        assert_eq!(head(tip, back, SeqHead::Async).segments.len(), 2);
        assert_eq!(head(tip, back, SeqHead::Open).segments.len(), 2);
        let none = head(tip, back, SeqHead::None);
        assert!(none.polygon.is_empty() && none.segments.is_empty());
        // Filled triangle points at the tip, its base back along the line.
        let poly = head(tip, back, SeqHead::Filled).polygon;
        assert_eq!(poly[0], tip);
        assert!(poly[1].0 < tip.0 && poly[2].0 < tip.0);
    }

    #[test]
    fn svg_has_dashes_heads_and_autonumber_prefix() {
        let d =
            sd("sequenceDiagram\nautonumber\nA-->>B: dashed filled\nA-xB: crossed\nA->B: plain");
        let svg = to_svg(&scene(&d));
        assert!(
            svg.contains("stroke-dasharray=\"6 4\""),
            "dashed message line"
        );
        assert!(svg.contains("<polygon"), "filled head triangle");
        assert!(svg.contains("<tspan font-weight=\"bold\">1. </tspan>dashed filled"));
        assert!(svg.contains("<tspan font-weight=\"bold\">2. </tspan>crossed"));
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.ends_with("</svg>\n"));
        // The cross adds two bare segments beyond the 3 message lines.
        let paths = svg.lines().filter(|l| l.starts_with("<path d=")).count();
        assert_eq!(paths, 3 + 2, "3 message lines + 2 cross segments");
    }

    fn svg_attr(s: &str, name: &str) -> f64 {
        let pat = format!("{name}=\"");
        let i = s.find(&pat).unwrap() + pat.len();
        s[i..i + s[i..].find('"').unwrap()].parse().unwrap()
    }

    /// Text content of a `<text>` line with inner tags removed
    /// (the bold autonumber `<tspan>`).
    fn text_content(line: &str) -> String {
        let inner = &line[line.find('>').unwrap() + 1..line.rfind("</text>").unwrap()];
        let mut out = String::new();
        let mut in_tag = false;
        for c in inner.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => out.push(c),
                _ => {}
            }
        }
        out
    }

    fn assert_text_contained(svg: &str) {
        let w = svg_attr(svg, "width");
        let h = svg_attr(svg, "height");
        for line in svg.lines().filter(|l| l.starts_with("<text")) {
            let x = svg_attr(line, "x");
            let y = svg_attr(line, "y");
            let inner = text_content(line);
            let tw = crate::layout::text_width(&inner);
            let (lo, hi) = if line.contains("text-anchor=\"middle\"") {
                (x - tw / 2.0, x + tw / 2.0)
            } else {
                (x, x + tw)
            };
            assert!(
                lo >= -1.0 && hi <= w + 1.0,
                "text {inner:?} [{lo:.0}..{hi:.0}] outside width {w}"
            );
            assert!((0.0..=h).contains(&y), "text y {y} outside height {h}");
        }
    }

    #[test]
    fn notes_and_edge_columns_stay_inside_the_canvas() {
        // A wide note LEFT of the first participant and RIGHT of the
        // last exercise the shift-right normalisation on both sides.
        let d = sd(
            "sequenceDiagram\nA->>B: hi\nNote left of A: a rather wide note text\n\
             Note right of B: another wide note on the right side\nNote over A,B: spanning both",
        );
        let svg = to_svg(&scene(&d));
        assert!(!svg.contains("NaN"));
        assert_text_contained(&svg);
        // Note card colors present.
        assert!(svg.contains(NOTE_FILL) && svg.contains(NOTE_STROKE));
    }

    /// End-to-end showcase guarding the whole feature set as one
    /// interacting whole: participants/actors with aliases, all
    /// eight arrows, autonumber, every note form, explicit and
    /// shorthand activations, and loop/opt/alt/par frames.
    #[test]
    fn sequence_showcase_parses_and_renders() {
        let d = sd(include_str!("../examples/sequence.mmd"));
        assert!(d.autonumber);
        let ids: Vec<&str> = d.participants.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["U", "FE", "API", "DB"]);
        assert!(d.participants[0].actor, "U is an actor");
        assert_eq!(d.participants[1].label, "Frontend");

        // All 8 (dashed, head) arrow combinations are exercised.
        let mut combos: Vec<(bool, SeqHead)> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::Message { dashed, head, .. } => Some((*dashed, *head)),
                _ => None,
            })
            .collect();
        combos.sort_by_key(|(d, h)| (*d, *h as usize));
        combos.dedup();
        assert_eq!(combos.len(), 8, "all 8 arrow variants: {combos:?}");
        // A self-message is present.
        assert!(d
            .items
            .iter()
            .any(|i| matches!(i, SeqItem::Message { from, to, .. } if from == to)));
        // Explicit + shorthand activations.
        assert!(d.items.iter().any(|i| matches!(i, SeqItem::Activate(_))));
        assert!(d.items.iter().any(|i| matches!(i, SeqItem::Deactivate(_))));
        assert!(d
            .items
            .iter()
            .any(|i| matches!(i, SeqItem::Message { activate: true, .. })));
        // Every note form.
        let sides: Vec<NoteSide> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::Note { side, .. } => Some(*side),
                _ => None,
            })
            .collect();
        assert!(sides
            .iter()
            .any(|s| matches!(s, NoteSide::Over(_, Some(_)))));
        assert!(sides.iter().any(|s| matches!(s, NoteSide::Over(_, None))));
        assert!(sides.iter().any(|s| matches!(s, NoteSide::LeftOf(_))));
        assert!(sides.iter().any(|s| matches!(s, NoteSide::RightOf(_))));
        // All four frame kinds, with else + and dividers.
        let kinds: Vec<FrameKind> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::FrameStart { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        for want in [
            FrameKind::Loop,
            FrameKind::Opt,
            FrameKind::Alt,
            FrameKind::Par,
        ] {
            assert!(kinds.contains(&want), "missing frame {want:?}");
        }
        assert_eq!(
            d.items
                .iter()
                .filter(|i| matches!(i, SeqItem::FrameElse { .. }))
                .count(),
            2
        );

        // Scene sanity: bars exist, frames closed with real heights.
        let sc = scene(&d);
        assert_eq!(sc.frames.len(), 4);
        assert!(sc.frames.iter().all(|f| f.h > 0.0 && f.w > 0.0));
        assert!(sc.activations.len() >= 2);
        assert!(!sc.messages.is_empty() && sc.messages.len() == 14);

        // Render: finite canvas, numbered labels, dashes, frames'
        // chips, note colors, no NaN, and every text inside.
        let svg = to_svg(&sc);
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.contains(">loop</text>") && svg.contains(">alt</text>"));
        assert!(svg.contains("[expired]"), "else divider label");
        assert!(svg.contains("<tspan font-weight=\"bold\">14. </tspan>"));
        assert!(svg.contains("stroke-dasharray=\"6 4\""));
        assert!(svg.contains("<polygon"), "filled arrowheads");
        assert_text_contained(&svg);
        // render_svg dispatches here too.
        let via_lib = crate::render_svg(include_str!("../examples/sequence.mmd")).unwrap();
        assert_eq!(via_lib, svg);
    }

    #[test]
    fn empty_scene_is_finite() {
        let sc = scene(&SequenceDiagram::default());
        assert!(sc.width > 0.0 && sc.height > 0.0);
        assert!(to_svg(&sc).ends_with("</svg>\n"));
    }

    /// Report Q5: participant boxes hard-coded white text on their
    /// accent fill, and actor boxes wrote the raw accent on white —
    /// both below WCAG AA for most accents.
    #[test]
    fn participant_and_actor_labels_read_on_every_accent() {
        let n = crate::style::ACCENTS.len();
        // Every accent once as a filled participant, and once as an
        // outlined actor (index + n wraps to the same accent).
        let mut src = String::from("sequenceDiagram\n");
        for i in 0..2 * n {
            let kind = if i < n { "participant" } else { "actor" };
            src.push_str(&format!("{kind} P{i}\n"));
        }
        src.push_str("P0->>P1: hi\n");
        let svg = crate::render_svg(&src).unwrap();
        for i in 0..2 * n {
            let label = format!("P{i}");
            if i < n {
                let accent = crate::style::accent(i);
                let ink = crate::style::assert_text_reads(&svg, &label, accent);
                assert_eq!(ink, crate::style::text_on(accent), "{label}");
            } else {
                let ink = crate::style::assert_text_reads(&svg, &label, "#ffffff");
                assert_eq!(ink, crate::style::accent_ink(i), "{label}");
            }
        }
    }
}
