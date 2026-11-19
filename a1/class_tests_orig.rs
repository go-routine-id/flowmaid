#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn cd(src: &str) -> ClassDiagram {
        match parse_document(src).unwrap() {
            Document::Class(d) => d,
            other => panic!("expected class diagram, got {:?}", other),
        }
    }

    /// Report Q5: the name compartment hard-coded white text on its
    /// accent, below WCAG AA on six of the eight (amber 2.6:1).
    #[test]
    fn header_text_reads_on_every_accent() {
        let names: Vec<String> = (0..crate::style::ACCENTS.len())
            .map(|i| format!("Klass{i}"))
            .collect();
        let src: String = names.iter().map(|n| format!("class {n}\n")).collect();
        let svg = crate::render_svg(&format!("classDiagram\n{src}")).unwrap();
        for (i, n) in names.iter().enumerate() {
            let accent = crate::style::accent(i);
            let ink = crate::style::assert_text_reads(&svg, n, accent);
            assert_eq!(ink, crate::style::text_on(accent), "{n}");
        }
    }

    #[test]
    fn parses_members_and_visibility() {
        let d = cd("classDiagram\n\
            class Account {\n\
                +String owner\n\
                -Double balance\n\
                +deposit(amount) bool\n\
                #audit()\n\
            }");
        assert_eq!(d.classes.len(), 1);
        let a = &d.classes[0];
        assert_eq!(a.name, "Account");
        assert_eq!(a.fields.len(), 2, "owner + balance");
        assert_eq!(a.methods.len(), 2, "deposit + audit");
        assert_eq!(a.fields[0].visibility.glyph(), "+");
        assert_eq!(a.fields[1].visibility.glyph(), "-");
        assert_eq!(a.methods[1].visibility.glyph(), "#");
    }

    #[test]
    fn inline_members_accumulate() {
        let d = cd("classDiagram\n\
            Dog : +String name\n\
            Dog : +bark() void");
        assert_eq!(d.classes.len(), 1);
        assert_eq!(d.classes[0].fields.len(), 1);
        assert_eq!(d.classes[0].methods.len(), 1);
    }

    #[test]
    fn inheritance_normalises_glyph_to_parent() {
        // Both spellings put the triangle on Animal (the parent).
        for src in [
            "classDiagram\nAnimal <|-- Dog",
            "classDiagram\nDog --|> Animal",
        ] {
            let d = cd(src);
            let r = &d.relations[0];
            assert_eq!(r.kind, RelKind::Inheritance);
            assert_eq!(d.classes[r.to].name, "Animal", "src: {src}");
            assert_eq!(d.classes[r.from].name, "Dog", "src: {src}");
        }
    }

    #[test]
    fn all_relation_kinds_parse() {
        let cases = [
            ("A <|-- B", RelKind::Inheritance, false),
            ("A *-- B", RelKind::Composition, false),
            ("A o-- B", RelKind::Aggregation, false),
            ("A --> B", RelKind::Association, false),
            ("A ..> B", RelKind::Dependency, true),
            ("A ..|> B", RelKind::Realization, true),
            ("A -- B", RelKind::Link, false),
        ];
        for (line, kind, dashed) in cases {
            let d = cd(&format!("classDiagram\n{line}"));
            assert_eq!(d.relations[0].kind, kind, "line: {line}");
            assert_eq!(d.relations[0].dashed, dashed, "line: {line}");
        }
    }

    #[test]
    fn cardinalities_and_label_parse() {
        let d = cd("classDiagram\nCustomer \"1\" --> \"*\" Order : places");
        let r = &d.relations[0];
        assert_eq!(d.classes[r.from].name, "Customer");
        assert_eq!(d.classes[r.to].name, "Order");
        assert_eq!(r.from_card.as_deref(), Some("1"));
        assert_eq!(r.to_card.as_deref(), Some("*"));
        assert_eq!(r.label.as_deref(), Some("places"));
    }

    #[test]
    fn renders_compartments_and_glyphs() {
        let d = cd("classDiagram\n\
            class Animal {\n\
                +String name\n\
                +move() void\n\
            }\n\
            Animal <|-- Dog\n\
            Animal \"1\" --> \"*\" Toy : plays");
        let svg = to_svg(&scene(&d));
        assert!(svg.contains(">Animal</text>") && svg.contains(">Dog</text>"));
        assert!(svg.contains("+String name"));
        assert!(svg.contains("+move() void"));
        // Inheritance triangle => a polygon end glyph.
        assert!(svg.contains("<polygon"), "inheritance needs a triangle");
        // Cardinality + label text present.
        assert!(svg.contains(">plays</text>"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn route_follows_dragged_class() {
        let d = cd("classDiagram\nA <|-- B");
        let s0 = scene(&d);
        let mut pos: Vec<(f64, f64)> = s0.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        pos[1].0 += 400.0;
        let s1 = route(&d, &pos);
        assert_eq!(s1.scene.nodes[1].x, pos[1].0);
        assert!(to_svg(&s1).ends_with("</svg>\n"));
    }

    #[test]
    fn no_nan_in_output() {
        let d = cd("classDiagram\nA <|-- B\nB *-- C\nC o-- A\nA ..> C");
        let svg = to_svg(&scene(&d));
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
    }

    // --- regression: parser bugs found by the bug hunt ---

    #[test]
    fn member_text_with_operator_stays_a_member() {
        // `--` / `..` inside member text must not be read as a relation.
        let d = cd("classDiagram\nCounter : -count -- decrement");
        assert_eq!(d.classes.len(), 1);
        assert_eq!(d.relations.len(), 0);
        assert_eq!(d.classes[0].fields[0].text, "count -- decrement");
    }

    #[test]
    fn quoted_cardinality_may_contain_colon_and_operator() {
        // A colon or operator INSIDE a "card" quote must be protected.
        let d = cd("classDiagram\nA \"1:n\" --> \"*--\" B : owns");
        assert_eq!(d.relations.len(), 1);
        let r = &d.relations[0];
        assert_eq!(r.kind, RelKind::Association);
        assert_eq!(d.classes[r.from].name, "A");
        assert_eq!(d.classes[r.to].name, "B");
        assert_eq!(r.from_card.as_deref(), Some("1:n"));
        assert_eq!(r.to_card.as_deref(), Some("*--"));
        assert_eq!(r.label.as_deref(), Some("owns"));
    }

    #[test]
    fn comma_list_declares_multiple_classes() {
        let d = cd("classDiagram\nclass Duck, Fish, Whale");
        assert_eq!(d.classes.len(), 3);
        assert_eq!(
            d.classes
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["Duck", "Fish", "Whale"]
        );
    }

    #[test]
    fn trailing_comment_is_stripped() {
        let d = cd("classDiagram\nFoo --> Baz %% inline trailing comment");
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.classes[d.relations[0].to].name, "Baz");
    }

    #[test]
    fn direction_and_note_directives_are_ignored() {
        let d = cd("classDiagram\ndirection LR\nnote for A \"hi\"\nnote \"x\"\nA <|-- B");
        assert_eq!(d.classes.len(), 2);
        assert_eq!(d.relations.len(), 1);
    }

    #[test]
    fn no_space_operators_parse() {
        let d = cd("classDiagram\nAnimal<|--Dog");
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.classes[d.relations[0].to].name, "Animal");
        assert_eq!(d.classes[d.relations[0].from].name, "Dog");
    }

    #[test]
    fn unclosed_block_reports_the_opening_line() {
        let err = parse_document("classDiagram\nclass Alpha {\n+x\n").unwrap_err();
        assert!(err.to_string().contains("line 2"), "got: {err}");
    }

    // --- regression: render geometry ---

    #[test]
    fn long_member_and_cardinality_fit_the_canvas() {
        let d = cd("classDiagram\n\
            class A {\n\
                +aVeryLongFieldNameThatMustStillFitInsideItsOwnBox: SomeLongTypeName\n\
            }\n\
            A \"0..manymanymanymany\" --> \"1..*exactly-one-or-more-longlabel\" B");
        let svg = to_svg(&scene(&d));
        // R1: members are not drawn in a wider font than they were sized in.
        assert!(!svg.contains("monospace"));
        let w = svg_attr(&svg, "width");
        let h = svg_attr(&svg, "height");
        for line in svg.lines().filter(|l| l.starts_with("<text")) {
            let x = svg_attr(line, "x");
            let y = svg_attr(line, "y");
            let inner = &line[line.find('>').unwrap() + 1..line.rfind("</text>").unwrap()];
            let tw = text_width(inner);
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

    fn svg_attr(s: &str, name: &str) -> f64 {
        let pat = format!("{name}=\"");
        let i = s.find(&pat).unwrap() + pat.len();
        s[i..i + s[i..].find('"').unwrap()].parse().unwrap()
    }

    /// End-to-end showcase guarding the whole feature set as one
    /// interacting whole: every UML relation kind, cardinalities and
    /// labels, three-compartment boxes, comma-list declaration, and
    /// the `direction` / `note` / trailing-`%%` lines together.
    #[test]
    fn class_showcase_parses_and_renders() {
        let d = cd(include_str!("../examples/class.mmd"));
        // 8 classes, incl. the two from `class CreditCard, PayPal`.
        assert_eq!(d.classes.len(), 8);
        for name in ["PaymentMethod", "Customer", "Order", "CreditCard", "PayPal"] {
            assert!(d.classes.iter().any(|c| c.name == name), "missing {name}");
        }
        // All six decorated relation kinds are exercised.
        let kinds: Vec<RelKind> = d.relations.iter().map(|r| r.kind).collect();
        for want in [
            RelKind::Inheritance,
            RelKind::Realization,
            RelKind::Composition,
            RelKind::Aggregation,
            RelKind::Association,
            RelKind::Dependency,
        ] {
            assert!(kinds.contains(&want), "missing relation {want:?}");
        }
        // Cardinalities and labels reached the model (the `1..*` sits
        // on LineItem, which the composition normalises to the `from`).
        assert!(d.relations.iter().any(
            |r| r.from_card.as_deref() == Some("1..*") || r.to_card.as_deref() == Some("1..*")
        ));
        assert!(d
            .relations
            .iter()
            .any(|r| r.label.as_deref() == Some("contains")));

        // Render: finite canvas, glyphs present, dashed lines for
        // realization/dependency, no NaN, everything inside the canvas.
        let svg = to_svg(&scene(&d));
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.contains("<polygon"), "triangles/diamonds");
        assert!(
            svg.contains("stroke-dasharray"),
            "dashed realization/dependency"
        );
        let w = svg_attr(&svg, "width");
        let h = svg_attr(&svg, "height");
        assert!(w > 0.0 && h > 0.0);
        for line in svg.lines().filter(|l| l.starts_with("<text")) {
            let x = svg_attr(line, "x");
            let y = svg_attr(line, "y");
            let inner = &line[line.find('>').unwrap() + 1..line.rfind("</text>").unwrap()];
            let tw = text_width(inner);
            let (lo, hi) = if line.contains("text-anchor=\"middle\"") {
                (x - tw / 2.0, x + tw / 2.0)
            } else {
                (x, x + tw)
            };
            assert!(
                lo >= -1.0 && hi <= w + 1.0,
                "text {inner:?} outside width {w}"
            );
            assert!((0.0..=h).contains(&y), "text y {y} outside height {h}");
        }
    }

    #[test]
    fn drag_past_origin_stays_inside_viewbox() {
        // A class dragged left of and/or above the canvas origin (the
        // playground's drag path) must not be clipped by the `0 0 W H`
        // viewBox — nor may any text, edge or marker it pulls along.
        // The last case keeps the centre in bounds but the box past it.
        let d = cd(include_str!("../examples/class.mmd"));
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
        let sc = scene(&cd(include_str!("../examples/class.mmd")));
        assert!(matches!(
            fit_top_left(&sc.scene),
            std::borrow::Cow::Borrowed(_)
        ));
    }
}
