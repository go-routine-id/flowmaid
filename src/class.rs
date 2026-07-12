//! UML class diagram rendering: classes drawn as three-compartment
//! boxes (name / fields / methods), relationships drawn with UML end
//! glyphs (triangles, diamonds, open arrows).
//!
//! Like the `er` module, layout is fully reused from the flowchart
//! pipeline: each class becomes one node of a synthetic top-down
//! graph whose size comes from its member list, each relationship
//! becomes one edge, and `scene_sized` produces the final geometry.
//!
//! Interactive apps use the same API shape as `scene`/`er`: [`scene`]
//! for automatic layout, [`route`] to follow dragged class positions,
//! [`to_svg`] to export any arrangement. [`head`] exposes the end
//! glyph as plain polygons/segments so GUI painters draw exactly what
//! SVG exports.

use crate::layout::text_width;
use crate::model::{Class, ClassDiagram, Direction, EdgeKind, Graph, RelKind, Shape};
use crate::scene::{
    escape, route_sized, scene_sized, svg_label_box, svg_open, Scene, EDGE_COLOR, LABEL_BORDER,
    TEXT_COLOR,
};

/// Name-compartment height in pixels.
pub const HEADER_H: f64 = 28.0;
/// Member (field / method) row height in pixels.
pub const ROW_H: f64 = 20.0;
/// Horizontal padding inside a class box.
pub const PAD: f64 = 10.0;
/// Height of an empty compartment (shown as a thin strip).
pub const EMPTY_H: f64 = 8.0;

/// One display row of a class box (pre-formatted with its glyph).
#[derive(Debug, Clone)]
pub struct ClassRow {
    /// Visibility glyph + member text, e.g. `"+name: String"`.
    pub text: String,
}

/// Drawing data for one class box. Index-aligned with the
/// corresponding [`Scene::nodes`] entry, whose rect is the box's
/// outer bounds.
#[derive(Debug, Clone)]
pub struct ClassBox {
    pub name: String,
    pub fields: Vec<ClassRow>,
    pub methods: Vec<ClassRow>,
}

/// Geometry + drawing data for a whole class diagram. `scene` holds
/// class boxes and relationship curves (dashed via `EdgeKind::Dotted`
/// for realization / dependency); `boxes[i]` describes
/// `scene.nodes[i]`; `rels[j]` mirrors `scene.edges[j]`'s UML style
/// and cardinalities.
#[derive(Debug, Clone)]
pub struct ClassScene {
    pub scene: Scene,
    pub boxes: Vec<ClassBox>,
    pub rels: Vec<RelStyle>,
}

/// End style + cardinalities for one relationship, aligned with a
/// scene edge. The glyph always sits at the `to` end (the parser
/// normalises relations so the decorated side is `to`).
#[derive(Debug, Clone)]
pub struct RelStyle {
    pub kind: RelKind,
    pub from_card: Option<String>,
    pub to_card: Option<String>,
}

/// The end glyph for one relationship: a closed polygon (triangle or
/// diamond) and/or open-arrow segments. Same geometry feeds both the
/// SVG writer and GUI painters.
#[derive(Debug, Clone, Default)]
pub struct Head {
    /// Closed polygon (triangle or diamond); empty when none.
    pub polygon: Vec<(f64, f64)>,
    /// Fill the polygon solid (composition) vs. hollow white.
    pub filled: bool,
    /// Open-arrow segments (association / dependency).
    pub segments: Vec<[(f64, f64); 2]>,
}

/// Automatic layout for a class diagram.
pub fn scene(d: &ClassDiagram) -> ClassScene {
    let (g, boxes, sizes) = build(d);
    assemble(d, boxes, scene_sized(&g, &sizes))
}

/// Re-route relationships for custom class positions (drags).
/// `centers[i]` = centre of class i in final coordinates.
pub fn route(d: &ClassDiagram, centers: &[(f64, f64)]) -> ClassScene {
    let (g, boxes, sizes) = build(d);
    assemble(d, boxes, route_sized(&g, centers, &sizes))
}

fn assemble(d: &ClassDiagram, boxes: Vec<ClassBox>, scene: Scene) -> ClassScene {
    // The synthetic graph is built strictly in class/relation order,
    // so indexes align 1:1 — load-bearing for every consumer.
    assert_eq!(scene.nodes.len(), d.classes.len());
    assert_eq!(scene.edges.len(), d.relations.len());
    let rels = d
        .relations
        .iter()
        .map(|r| RelStyle {
            kind: r.kind,
            from_card: r.from_card.clone(),
            to_card: r.to_card.clone(),
        })
        .collect();
    ClassScene {
        scene,
        boxes,
        rels,
    }
}

/// Synthetic flowchart graph + box drawing data + node sizes.
///
/// INVARIANT: `g.nodes[i]` corresponds to `d.classes[i]` and
/// `g.edges[j]` to `d.relations[j]` — class names are unique
/// (`ensure_class` dedupes), `ensure_node` appends in call order, and
/// edges are added in relation order. `assemble` asserts the counts.
fn build(d: &ClassDiagram) -> (Graph, Vec<ClassBox>, Vec<(f64, f64)>) {
    let mut g = Graph::default();
    g.direction = Direction::TD;
    for c in &d.classes {
        g.ensure_node(&c.name, Some(c.name.clone()), Some(Shape::Rect));
    }
    for r in &d.relations {
        // Dashed lines (realization / dependency) carry into the
        // Scene as Dotted so any consumer styles them right.
        let kind = if r.dashed {
            EdgeKind::Dotted
        } else {
            EdgeKind::Open
        };
        g.add_edge(r.from, r.to, r.label.clone(), kind);
    }

    let mut boxes = Vec::with_capacity(d.classes.len());
    let mut sizes = Vec::with_capacity(d.classes.len());
    for c in &d.classes {
        let (b, size) = box_of(c);
        boxes.push(b);
        sizes.push(size);
    }
    (g, boxes, sizes)
}

fn box_of(c: &Class) -> (ClassBox, (f64, f64)) {
    let fields: Vec<ClassRow> = c.fields.iter().map(row_of).collect();
    let methods: Vec<ClassRow> = c.methods.iter().map(row_of).collect();
    let mut member_w = 0.0f64;
    for r in fields.iter().chain(&methods) {
        member_w = member_w.max(text_width(&r.text));
    }
    let w = (text_width(&c.name) + 28.0)
        .max(member_w + 2.0 * PAD)
        .max(96.0);
    let h = HEADER_H + compartment_h(fields.len()) + compartment_h(methods.len());
    (
        ClassBox {
            name: c.name.clone(),
            fields,
            methods,
        },
        (w, h),
    )
}

fn row_of(m: &crate::model::Member) -> ClassRow {
    ClassRow {
        text: format!("{}{}", m.visibility.glyph(), m.text),
    }
}

/// Height of one member compartment: real rows, or a thin strip when
/// empty (UML shows the empty attributes / operations divider).
fn compartment_h(rows: usize) -> f64 {
    if rows == 0 {
        EMPTY_H
    } else {
        rows as f64 * ROW_H
    }
}

/// Serialise any class arrangement (automatic or dragged) to SVG.
pub fn to_svg(cs: &ClassScene) -> String {
    let sc = &cs.scene;
    let mut s = String::new();
    svg_open(&mut s, sc.width, sc.height, 13);

    // Relationship lines + UML end glyphs (under the boxes).
    for (e, rel) in sc.edges.iter().zip(&cs.rels) {
        let q = e.bezier;
        let dash = if matches!(e.kind, EdgeKind::Dotted) {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"1.6\"{}/>\n",
            q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1, q[3].0, q[3].1, EDGE_COLOR, dash
        ));
        write_head(&mut s, &head(q[3], q[2], rel.kind));
        if let Some(c) = &rel.from_card {
            card_label(&mut s, q[0], q[1], c);
        }
        if let Some(c) = &rel.to_card {
            card_label(&mut s, q[3], q[2], c);
        }
    }

    // Class boxes — each gets a stable accent color.
    for (i, (n, b)) in sc.nodes.iter().zip(&cs.boxes).enumerate() {
        let accent = crate::style::accent(i);
        let x0 = n.x - n.w / 2.0;
        let y0 = n.y - n.h / 2.0;
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            x0, y0, n.w, n.h, accent
        ));
        // Name compartment.
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"{}\"/>\n",
            x0, y0, n.w, HEADER_H, accent
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"#ffffff\">{}</text>\n",
            n.x,
            y0 + HEADER_H / 2.0,
            escape(&b.name)
        ));

        // Divider under the name, then fields, then a divider, then
        // methods. Empty compartments still get their strip + line.
        let fields_top = y0 + HEADER_H;
        divider(&mut s, x0, fields_top, n.w);
        write_rows(&mut s, &b.fields, x0, fields_top, n.w);

        let methods_top = fields_top + compartment_h(b.fields.len());
        divider(&mut s, x0, methods_top, n.w);
        write_rows(&mut s, &b.methods, x0, methods_top, n.w);
    }

    // Relationship labels on top of everything.
    for e in &sc.edges {
        if let Some((text, m, w)) = &e.label {
            svg_label_box(&mut s, text, *m, *w);
        }
    }

    s.push_str("</svg>\n");
    s
}

fn divider(s: &mut String, x0: f64, y: f64, w: f64) {
    s.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\"/>\n",
        x0,
        y,
        x0 + w,
        y,
        LABEL_BORDER
    ));
}

fn write_rows(s: &mut String, rows: &[ClassRow], x0: f64, top: f64, _w: f64) {
    for (i, row) in rows.iter().enumerate() {
        let ty = top + i as f64 * ROW_H + ROW_H / 2.0;
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\" \
             font-family=\"monospace\">{}</text>\n",
            x0 + PAD,
            ty,
            TEXT_COLOR,
            escape(&row.text)
        ));
    }
}

fn write_head(s: &mut String, h: &Head) {
    if !h.polygon.is_empty() {
        let pts = h
            .polygon
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", x, y))
            .collect::<Vec<_>>()
            .join(" ");
        let fill = if h.filled { EDGE_COLOR } else { "#ffffff" };
        s.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            pts, fill, EDGE_COLOR
        ));
    }
    for [a, b] in &h.segments {
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"{}\" \
             stroke-width=\"1.6\"/>\n",
            a.0, a.1, b.0, b.1, EDGE_COLOR
        ));
    }
}

fn card_label(s: &mut String, e: (f64, f64), c: (f64, f64), text: &str) {
    // Offset the cardinality a little inward and to the side of the
    // line so it clears the class border and the curve.
    let (dx, dy) = (c.0 - e.0, c.1 - e.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let u = (dx / len, dy / len);
    let nv = (-u.1, u.0);
    let px = e.0 + u.0 * 14.0 + nv.0 * 9.0;
    let py = e.1 + u.1 * 14.0 + nv.1 * 9.0;
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
         fill=\"{}\" font-size=\"11\">{}</text>\n",
        px,
        py,
        TEXT_COLOR,
        escape(text)
    ));
}

/// UML end glyph at endpoint `e` of a curve whose adjacent control
/// point is `c` (giving the inward tangent). The glyph points at `e`
/// (the class border).
pub fn head(e: (f64, f64), c: (f64, f64), kind: RelKind) -> Head {
    let (dx, dy) = (c.0 - e.0, c.1 - e.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let u = (dx / len, dy / len); // unit vector pointing inward (away from the class)
    let nv = (-u.1, u.0); // unit normal
    let p = |k: f64, t: f64| (e.0 + u.0 * k + nv.0 * t, e.1 + u.1 * k + nv.1 * t);

    match kind {
        // Hollow triangle at the parent — inheritance & realization.
        RelKind::Inheritance | RelKind::Realization => Head {
            polygon: vec![e, p(14.0, -8.0), p(14.0, 8.0)],
            filled: false,
            segments: vec![],
        },
        // Diamond at the aggregate / composite.
        RelKind::Composition | RelKind::Aggregation => Head {
            polygon: vec![e, p(9.0, -6.5), p(18.0, 0.0), p(9.0, 6.5)],
            filled: matches!(kind, RelKind::Composition),
            segments: vec![],
        },
        // Open V arrow — association & dependency.
        RelKind::Association | RelKind::Dependency => Head {
            polygon: vec![],
            filled: false,
            segments: vec![[p(12.0, -7.0), e], [p(12.0, 7.0), e]],
        },
        // Plain link — no glyph.
        RelKind::Link => Head::default(),
    }
}

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
}
