//! Entity-Relationship rendering: entities drawn as attribute
//! tables, relationships drawn with crow's foot notation.
//!
//! Layout is fully reused from the flowchart pipeline: each entity
//! becomes one node of a synthetic left-to-right graph whose size
//! comes from its attribute table, each relationship becomes one
//! edge, and `scene_sized` produces the final geometry.
//!
//! Mirroring the `scene` module, interactive apps use the same API
//! shape: [`scene`] for automatic layout, [`route`] to follow
//! dragged entity positions, [`to_svg`] to export any arrangement.
//! [`glyph`] exposes crow's foot geometry as plain segments and an
//! optional circle so GUI painters draw exactly what SVG exports.

use crate::layout::text_width;
use crate::model::{Attr, Card, Direction, EdgeKind, Entity, ErDiagram, Graph, Shape};
use crate::scene::{
    escape, route_sized, scene_sized, svg_label_box, svg_open, Scene, EDGE_COLOR, TEXT_COLOR,
};

/// Table header height in pixels.
pub const HEADER_H: f64 = 30.0;
/// Attribute row height in pixels.
pub const ROW_H: f64 = 22.0;
/// Horizontal padding inside a table.
pub const PAD: f64 = 10.0;
/// Gap between the type / name / key columns.
pub const COL_GAP: f64 = 14.0;

/// Muted colour for the attribute type column.
pub const TYPE_COLOR: &str = "#6a7086";

/// One display row of an entity table (pre-formatted).
#[derive(Debug, Clone)]
pub struct ErRow {
    pub ty: String,
    pub name: String,
    /// Joined key tags, e.g. `"PK"` or `"PK,FK"`; empty when none.
    pub keys: String,
}

/// Drawing data for one entity table. Index-aligned with the
/// corresponding [`Scene::nodes`] entry, whose rect is the table's
/// outer box.
#[derive(Debug, Clone)]
pub struct ErTable {
    pub name: String,
    pub rows: Vec<ErRow>,
    /// Width of the type column, so attribute names align.
    pub ty_col_w: f64,
}

/// Geometry + drawing data for a whole ER diagram. `scene` holds
/// node boxes and relationship curves (dashed = non-identifying via
/// `EdgeKind::Dotted`); `tables[i]` describes `scene.nodes[i]`;
/// `cards[j]` holds the (from, to) cardinalities of `scene.edges[j]`.
#[derive(Debug, Clone)]
pub struct ErScene {
    pub scene: Scene,
    pub tables: Vec<ErTable>,
    pub cards: Vec<(Card, Card)>,
}

/// Crow's foot glyph for one relationship end: plain line segments
/// plus an optional hollow circle `(centre, radius)`. Same geometry
/// feeds both the SVG writer and GUI painters.
#[derive(Debug, Clone)]
pub struct Glyph {
    pub segments: Vec<[(f64, f64); 2]>,
    pub circle: Option<((f64, f64), f64)>,
}

/// Automatic layout for an ER diagram.
pub fn scene(d: &ErDiagram) -> ErScene {
    let (g, tables, sizes) = build(d);
    assemble(d, tables, scene_sized(&g, &sizes))
}

/// Re-route relationships for custom entity positions (drags).
/// `centers[i]` = centre of entity i in final coordinates.
pub fn route(d: &ErDiagram, centers: &[(f64, f64)]) -> ErScene {
    let (g, tables, sizes) = build(d);
    assemble(d, tables, route_sized(&g, centers, &sizes))
}

fn assemble(d: &ErDiagram, tables: Vec<ErTable>, scene: Scene) -> ErScene {
    // The synthetic graph is built strictly in entity/relation
    // order, so indexes align 1:1. This is load-bearing for every
    // consumer of ErScene; fail loudly if a refactor breaks it.
    assert_eq!(scene.nodes.len(), d.entities.len());
    assert_eq!(scene.edges.len(), d.relations.len());
    let cards = d.relations.iter().map(|r| (r.card_from, r.card_to)).collect();
    ErScene {
        scene,
        tables,
        cards,
    }
}

/// Synthetic flowchart graph + table drawing data + node sizes.
///
/// INVARIANT: `g.nodes[i]` corresponds to `d.entities[i]` and
/// `g.edges[j]` to `d.relations[j]`. This holds because entity
/// names are unique (`ErDiagram::ensure_entity` dedupes at parse
/// time), `ensure_node` appends in call order, and edges are added
/// in relation order. `assemble` asserts the node/edge counts.
fn build(d: &ErDiagram) -> (Graph, Vec<ErTable>, Vec<(f64, f64)>) {
    let mut g = Graph::default();
    g.direction = Direction::LR;
    for e in &d.entities {
        g.ensure_node(&e.name, Some(e.name.clone()), Some(Shape::Rect));
    }
    for r in &d.relations {
        // Dotted carries the non-identifying dash into the Scene so
        // any consumer (SVG writer, GUI painter) styles it right.
        let kind = if r.identifying {
            EdgeKind::Open
        } else {
            EdgeKind::Dotted
        };
        g.add_edge(r.from, r.to, r.label.clone(), kind);
    }

    let mut tables = Vec::with_capacity(d.entities.len());
    let mut sizes = Vec::with_capacity(d.entities.len());
    for e in &d.entities {
        let (table, size) = table_of(e);
        tables.push(table);
        sizes.push(size);
    }
    (g, tables, sizes)
}

fn table_of(e: &Entity) -> (ErTable, (f64, f64)) {
    let rows: Vec<ErRow> = e.attrs.iter().map(row_of).collect();
    let mut ty_w = 0.0f64;
    let mut name_w = 0.0f64;
    let mut keys_w = 0.0f64;
    for r in &rows {
        ty_w = ty_w.max(text_width(&r.ty));
        name_w = name_w.max(text_width(&r.name));
        keys_w = keys_w.max(text_width(&r.keys));
    }
    let mut row_w = ty_w + COL_GAP + name_w;
    if keys_w > 0.0 {
        row_w += COL_GAP + keys_w;
    }
    let w = (text_width(&e.name) + 28.0)
        .max(row_w + 2.0 * PAD)
        .max(120.0);
    let h = HEADER_H + rows.len() as f64 * ROW_H;
    (
        ErTable {
            name: e.name.clone(),
            rows,
            ty_col_w: ty_w,
        },
        (w, h),
    )
}

fn row_of(a: &Attr) -> ErRow {
    ErRow {
        ty: a.ty.clone(),
        name: a.name.clone(),
        keys: a
            .keys
            .iter()
            .map(|k| k.tag())
            .collect::<Vec<_>>()
            .join(","),
    }
}

/// Serialise any ER arrangement (automatic or dragged) to SVG.
pub fn to_svg(es: &ErScene) -> String {
    let sc = &es.scene;
    let mut s = String::new();
    svg_open(&mut s, sc.width, sc.height, 13);

    // Relationship lines + crow's foot glyphs (under the tables).
    for (e, &(card_from, card_to)) in sc.edges.iter().zip(&es.cards) {
        let q = e.bezier;
        let dash = if matches!(e.kind, EdgeKind::Dotted) {
            " stroke-dasharray=\"5 4\""
        } else {
            ""
        };
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"1.7\"{}/>\n",
            q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1, q[3].0, q[3].1, EDGE_COLOR, dash
        ));
        write_glyph(&mut s, &glyph(q[0], q[1], card_from));
        write_glyph(&mut s, &glyph(q[3], q[2], card_to));
    }

    // Entity tables — each entity gets a stable accent color.
    for (i, (n, table)) in sc.nodes.iter().zip(&es.tables).enumerate() {
        let accent = crate::style::accent(i);
        let x0 = n.x - n.w / 2.0;
        let y0 = n.y - n.h / 2.0;
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            x0, y0, n.w, n.h, accent
        ));
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
            escape(&table.name)
        ));
        for (r, row) in table.rows.iter().enumerate() {
            let ry = y0 + HEADER_H + r as f64 * ROW_H;
            if r > 0 {
                s.push_str(&format!(
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
                     stroke=\"{}\"/>\n",
                    x0,
                    ry,
                    x0 + n.w,
                    ry,
                    crate::scene::LABEL_BORDER
                ));
            }
            let tyc = ry + ROW_H / 2.0;
            text_at(&mut s, x0 + PAD, tyc, "", TYPE_COLOR, &row.ty);
            text_at(
                &mut s,
                x0 + PAD + table.ty_col_w + COL_GAP,
                tyc,
                "",
                TEXT_COLOR,
                &row.name,
            );
            if !row.keys.is_empty() {
                text_at(
                    &mut s,
                    x0 + n.w - PAD,
                    tyc,
                    " text-anchor=\"end\" font-weight=\"bold\"",
                    EDGE_COLOR,
                    &row.keys,
                );
            }
        }
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

fn text_at(s: &mut String, x: f64, y: f64, extra: &str, fill: &str, text: &str) {
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\"{} fill=\"{}\">{}</text>\n",
        x,
        y,
        extra,
        fill,
        escape(text)
    ));
}

fn write_glyph(s: &mut String, g: &Glyph) {
    for [a, b] in &g.segments {
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"{}\" \
             stroke-width=\"1.7\"/>\n",
            a.0, a.1, b.0, b.1, EDGE_COLOR
        ));
    }
    if let Some((c, r)) = g.circle {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#ffffff\" stroke=\"{}\" \
             stroke-width=\"1.7\"/>\n",
            c.0, c.1, r, EDGE_COLOR
        ));
    }
}

/// Crow's foot geometry at endpoint `e` of a curve whose adjacent
/// bezier control point is `c` (which gives the inward tangent).
/// Fork prongs touch the entity border at `e`; ticks and circles
/// sit further along the line.
pub fn glyph(e: (f64, f64), c: (f64, f64), card: Card) -> Glyph {
    let (dx, dy) = (c.0 - e.0, c.1 - e.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let u = (dx / len, dy / len); // unit vector pointing away from the entity
    let nv = (-u.1, u.0); // unit normal
    // Point k units inward along the line, t units sideways.
    let p = |k: f64, t: f64| (e.0 + u.0 * k + nv.0 * t, e.1 + u.1 * k + nv.1 * t);
    let tick = |k: f64| [p(k, -5.5), p(k, 5.5)];
    let fork = || {
        let f = p(12.0, 0.0);
        [[f, p(0.0, -6.0)], [f, p(0.0, 6.0)], [f, e]]
    };

    match card {
        Card::One => Glyph {
            segments: vec![tick(8.0), tick(13.0)],
            circle: None,
        },
        Card::ZeroOne => Glyph {
            segments: vec![tick(8.0)],
            circle: Some((p(17.0, 0.0), 4.5)),
        },
        Card::ZeroMany => Glyph {
            segments: fork().to_vec(),
            circle: Some((p(19.0, 0.0), 4.5)),
        },
        Card::OneMany => {
            let mut segments = fork().to_vec();
            segments.push(tick(16.0));
            Glyph {
                segments,
                circle: None,
            }
        }
    }
}

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
        assert!(svg.contains(">has</text>"), "relationship label must render");
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
                assert!(xy[0] >= -0.5 && xy[0] <= w + 0.5, "x={} outside w={}", xy[0], w);
                assert!(xy[1] >= -0.5 && xy[1] <= h + 0.5, "y={} outside h={}", xy[1], h);
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
        let mut pos: Vec<(f64, f64)> =
            s0.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
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
}
