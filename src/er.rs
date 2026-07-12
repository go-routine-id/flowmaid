//! Entity-Relationship rendering: entities drawn as attribute
//! tables, relationships drawn with crow's foot notation.
//!
//! Layout is fully reused from the flowchart pipeline: each entity
//! becomes one node of a synthetic left-to-right graph whose size
//! comes from its attribute table, each relationship becomes one
//! edge, and `scene_sized` produces the final geometry. Only the
//! SVG writer differs (tables instead of plain shapes, crow's foot
//! glyphs instead of arrowheads).

use crate::layout::text_width;
use crate::model::{Attr, Card, Direction, EdgeKind, Entity, ErDiagram, Graph, Shape};
use crate::scene::{escape, scene_sized, Scene};

const HEADER_H: f64 = 30.0;
const ROW_H: f64 = 22.0;
const PAD: f64 = 10.0;
const COL_GAP: f64 = 14.0;

const EDGE_COLOR: &str = "#44507a";
const HEADER_FILL: &str = "#5b6dc0";
const BODY_FILL: &str = "#ffffff";
const ROW_LINE: &str = "#d5d9ec";
const TYPE_COLOR: &str = "#6a7086";
const TEXT_COLOR: &str = "#232840";

/// Table dimensions for one entity; `ty_w` is the width of the
/// type column so attribute names align across rows.
struct Dims {
    w: f64,
    h: f64,
    ty_w: f64,
}

fn keys_text(a: &Attr) -> String {
    a.keys
        .iter()
        .map(|k| k.tag())
        .collect::<Vec<_>>()
        .join(",")
}

fn dims(e: &Entity) -> Dims {
    let ty_w = e.attrs.iter().map(|a| text_width(&a.ty)).fold(0.0, f64::max);
    let name_w = e
        .attrs
        .iter()
        .map(|a| text_width(&a.name))
        .fold(0.0, f64::max);
    let keys_w = e
        .attrs
        .iter()
        .map(|a| text_width(&keys_text(a)))
        .fold(0.0, f64::max);
    let mut row_w = ty_w + COL_GAP + name_w;
    if keys_w > 0.0 {
        row_w += COL_GAP + keys_w;
    }
    let w = (text_width(&e.name) + 28.0)
        .max(row_w + 2.0 * PAD)
        .max(120.0);
    let h = HEADER_H + e.attrs.len() as f64 * ROW_H;
    Dims { w, h, ty_w }
}

/// Synthetic flowchart graph: one Rect node per entity, one edge
/// per relationship. Left-to-right reads best for schemas. The
/// edge kind only carries the dash style; crow's feet are drawn by
/// this module, not by markers.
fn to_graph(d: &ErDiagram) -> Graph {
    let mut g = Graph::default();
    g.direction = Direction::LR;
    for e in &d.entities {
        g.ensure_node(&e.name, Some(e.name.clone()), Some(Shape::Rect));
    }
    for r in &d.relations {
        let kind = if r.identifying {
            EdgeKind::Open
        } else {
            EdgeKind::Dotted
        };
        g.add_edge(r.from, r.to, r.label.clone(), kind);
    }
    g
}

/// Render an ER diagram to SVG.
pub fn to_svg(d: &ErDiagram) -> String {
    let g = to_graph(d);
    let ds: Vec<Dims> = d.entities.iter().map(dims).collect();
    let sizes: Vec<(f64, f64)> = ds.iter().map(|t| (t.w, t.h)).collect();
    let sc = scene_sized(&g, &sizes);
    write_svg(d, &ds, &sc)
}

fn write_svg(d: &ErDiagram, ds: &[Dims], sc: &Scene) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"Helvetica, Arial, sans-serif\" \
         font-size=\"13\">\n",
        w = sc.width,
        h = sc.height
    ));
    s.push_str(&format!(
        "<rect width=\"{:.0}\" height=\"{:.0}\" fill=\"#ffffff\"/>\n",
        sc.width, sc.height
    ));

    // Relationship lines + crow's foot glyphs (under the tables).
    for (rel, e) in d.relations.iter().zip(&sc.edges) {
        let q = e.bezier;
        let dash = if rel.identifying {
            ""
        } else {
            " stroke-dasharray=\"5 4\""
        };
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"1.7\"{}/>\n",
            q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1, q[3].0, q[3].1, EDGE_COLOR, dash
        ));
        glyph(&mut s, q[0], q[1], rel.card_from);
        glyph(&mut s, q[3], q[2], rel.card_to);
    }

    // Entity tables.
    for (i, (ent, dim)) in d.entities.iter().zip(ds).enumerate() {
        let n = &sc.nodes[i];
        let x0 = n.x - n.w / 2.0;
        let y0 = n.y - n.h / 2.0;
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            x0, y0, n.w, n.h, BODY_FILL, HEADER_FILL
        ));
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"{}\"/>\n",
            x0, y0, n.w, HEADER_H, HEADER_FILL
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"#ffffff\">{}</text>\n",
            n.x,
            y0 + HEADER_H / 2.0,
            escape(&ent.name)
        ));
        for (r, a) in ent.attrs.iter().enumerate() {
            let ry = y0 + HEADER_H + r as f64 * ROW_H;
            if r > 0 {
                s.push_str(&format!(
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
                     stroke=\"{}\"/>\n",
                    x0,
                    ry,
                    x0 + n.w,
                    ry,
                    ROW_LINE
                ));
            }
            let ty_x = x0 + PAD;
            let name_x = x0 + PAD + dim.ty_w + COL_GAP;
            let tyc = ry + ROW_H / 2.0;
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
                ty_x,
                tyc,
                TYPE_COLOR,
                escape(&a.ty)
            ));
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
                name_x,
                tyc,
                TEXT_COLOR,
                escape(&a.name)
            ));
            if !a.keys.is_empty() {
                s.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"end\" \
                     font-weight=\"bold\" fill=\"{}\">{}</text>\n",
                    x0 + n.w - PAD,
                    tyc,
                    EDGE_COLOR,
                    keys_text(a)
                ));
            }
        }
    }

    // Relationship labels on top of everything.
    for e in &sc.edges {
        if let Some((text, m, w)) = &e.label {
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"20\" rx=\"4\" \
                 fill=\"#ffffff\" stroke=\"{}\"/>\n",
                m.0 - w / 2.0,
                m.1 - 10.0,
                w,
                ROW_LINE
            ));
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 fill=\"{}\">{}</text>\n",
                m.0,
                m.1,
                TEXT_COLOR,
                escape(text)
            ));
        }
    }

    s.push_str("</svg>\n");
    s
}

/// Crow's foot glyph at endpoint `e` of a curve whose adjacent
/// bezier control point is `c` (which gives the inward tangent).
/// The fork prongs touch the entity border at `e`; circles and
/// ticks sit further along the line.
fn glyph(s: &mut String, e: (f64, f64), c: (f64, f64), card: Card) {
    let (dx, dy) = (c.0 - e.0, c.1 - e.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let u = (dx / len, dy / len); // unit vector pointing away from the entity
    let nv = (-u.1, u.0); // unit normal
    // Point k units inward along the line, t units sideways.
    let p = |k: f64, t: f64| (e.0 + u.0 * k + nv.0 * t, e.1 + u.1 * k + nv.1 * t);

    let mut line = |a: (f64, f64), b: (f64, f64)| {
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"{}\" \
             stroke-width=\"1.7\"/>\n",
            a.0, a.1, b.0, b.1, EDGE_COLOR
        ));
    };
    let tick = |line: &mut dyn FnMut((f64, f64), (f64, f64)), k: f64| {
        line(p(k, -5.5), p(k, 5.5));
    };
    let fork = |line: &mut dyn FnMut((f64, f64), (f64, f64))| {
        let f = p(12.0, 0.0);
        line(f, p(0.0, -6.0));
        line(f, p(0.0, 6.0));
        line(f, e);
    };
    let circle = |s: &mut String, k: f64| {
        let cc = p(k, 0.0);
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4.5\" fill=\"#ffffff\" stroke=\"{}\" \
             stroke-width=\"1.7\"/>\n",
            cc.0, cc.1, EDGE_COLOR
        ));
    };

    match card {
        Card::One => {
            tick(&mut line, 8.0);
            tick(&mut line, 13.0);
        }
        Card::ZeroOne => {
            tick(&mut line, 8.0);
            drop(line);
            circle(s, 17.0);
        }
        Card::ZeroMany => {
            fork(&mut line);
            drop(line);
            circle(s, 19.0);
        }
        Card::OneMany => {
            tick(&mut line, 16.0);
            fork(&mut line);
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
        let svg = to_svg(&fixture());
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
        let svg = to_svg(&fixture());
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
        let svg = to_svg(&er("erDiagram\nA ||--o{ B : owns"));
        assert!(svg.contains(">A</text>") && svg.contains(">B</text>"));
        assert!(svg.contains(">owns</text>"));
    }

    #[test]
    fn non_identifying_relationship_is_dashed() {
        let svg = to_svg(&er("erDiagram\nA ||..o{ B"));
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn zero_one_and_one_many_draw_circle_and_ticks() {
        // |o on the left = ZeroOne (circle); |{ on the right = OneMany.
        let svg = to_svg(&er("erDiagram\nA |o--|{ B"));
        assert!(svg.contains("<circle"), "ZeroOne side needs a circle");
    }
}
