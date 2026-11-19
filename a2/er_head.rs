//! Entity-Relationship rendering: entities drawn as attribute
//! tables, relationships drawn with crow's foot notation.
//!
//! Layout is fully reused from the flowchart pipeline: each entity
//! becomes one node of a synthetic graph (top-to-bottom by default, as
//! in mermaid; `direction LR` etc. turn it) whose size comes from its
//! attribute table, each relationship becomes one edge, and
//! `scene_sized` produces the final geometry.
//!
//! Mirroring the `scene` module, interactive apps use the same API
//! shape: [`scene`] for automatic layout, [`route`] to follow
//! dragged entity positions, [`to_svg`] to export any arrangement.
//! [`glyph`] exposes crow's foot geometry as plain segments and an
//! optional circle so GUI painters draw exactly what SVG exports.

use crate::layout::{text_width, LABEL_BOX_H};
use crate::model::{Attr, Card, EdgeKind, Entity, ErDiagram, Graph, Shape};
use crate::scene::{
    edge_end_chord, edge_path_d, edge_polyline, escape, fit_top_left, route_sized, scene_sized,
    seg_aabb, svg_label_box, svg_open_a11y, A11y, Scene, SceneEdge, SvgOptions, EDGE_COLOR,
    TEXT_COLOR,
};

/// Table header height in pixels.
pub const HEADER_H: f64 = 30.0;
/// Attribute row height in pixels.
pub const ROW_H: f64 = 22.0;
/// Horizontal padding inside a table.
pub const PAD: f64 = 10.0;
/// Gap between the type / name / key / comment columns.
pub const COL_GAP: f64 = 14.0;
/// Widest an attribute comment is drawn; a longer one is cut with an
/// ellipsis (the SVG keeps the whole text as its tooltip), so one
/// paragraph-long comment cannot stretch its table across the canvas.
pub const COMMENT_MAX_W: f64 = 280.0;

/// Muted colour for the attribute type column (and comments).
pub const TYPE_COLOR: &str = "#6a7086";

/// Stroke width of relationship lines and their crow's foot glyphs.
const LINE_W: f64 = 1.7;
/// Crow's foot proportions, in px from the entity border along the
/// line (`k`) or across it. Sized near mermaid's (18px bars, r=6
/// circle) yet a little smaller, to suit flowmaid's 22px rows.
const TICK_HALF: f64 = 7.5;
/// Where a fork's three prongs meet (they spread to `FORK_HALF` at
/// the border).
const FORK_LEN: f64 = 16.0;
const FORK_HALF: f64 = 8.0;
const CIRCLE_R: f64 = 5.5;
/// How far back along the drawn curve a glyph takes its direction:
/// the length of a fork, so the glyph lies on the stroke it ends.
const GLYPH_CHORD: f64 = FORK_LEN;
/// Least air between a glyph and a relationship label.
const GLYPH_AIR: f64 = 4.0;

/// One display row of an entity table (pre-formatted).
/// `#[non_exhaustive]`: rows are built by [`scene`] / [`route`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ErRow {
    pub ty: String,
    pub name: String,
    /// Joined key tags, e.g. `"PK"` or `"PK,FK"`; empty when none.
    pub keys: String,
    /// The attribute's `"comment"` as drawn: whole, or cut with an
    /// ellipsis to [`COMMENT_MAX_W`]; empty when none.
    pub comment: String,
    /// The whole comment when `comment` had to be cut (the SVG writes
    /// it as the row's tooltip), else `None`.
    pub comment_full: Option<String>,
}

/// Drawing data for one entity table. Index-aligned with the
/// corresponding [`Scene::nodes`] entry, whose rect is the table's
/// outer box. A row draws, left to right: type (at [`PAD`]), name (at
/// [`ErTable::name_x`]), keys (right-aligned at
/// [`ErTable::keys_end_x`]) and comment (at [`ErTable::comment_x`]).
/// `#[non_exhaustive]`: tables are built by [`scene`] / [`route`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ErTable {
    /// The entity's name (its id).
    pub name: String,
    /// What the header shows: the entity's alias (`p[Person]`) when it
    /// has one, else `name`.
    pub title: String,
    pub rows: Vec<ErRow>,
    /// Width of the type column, so attribute names align.
    pub ty_col_w: f64,
    /// Width of the name column.
    pub name_col_w: f64,
    /// Width of the key column; 0 when no row has a key.
    pub keys_col_w: f64,
    /// Width of the comment column; 0 when no row has a comment.
    pub comment_col_w: f64,
}

impl ErTable {
    /// Where the name column starts, from the table's left border.
    pub fn name_x(&self) -> f64 {
        PAD + self.ty_col_w + COL_GAP
    }

    /// Where keys END (they are right-aligned), from the left border of
    /// a table `w` wide: against the right border (less [`PAD`]) in a
    /// table without comments, else just before the comment column.
    pub fn keys_end_x(&self, w: f64) -> f64 {
        if self.comment_col_w > 0.0 {
            self.name_x() + self.name_col_w + COL_GAP + self.keys_col_w
        } else {
            w - PAD
        }
    }

    /// Where comments start, from the left border: after the key
    /// column, or the name column when no row has a key.
    pub fn comment_x(&self) -> f64 {
        let x = self.name_x() + self.name_col_w + COL_GAP;
        if self.keys_col_w > 0.0 {
            x + self.keys_col_w + COL_GAP
        } else {
            x
        }
    }
}

/// Geometry + drawing data for a whole ER diagram. `scene` holds
/// node boxes and relationship curves (dashed = non-identifying via
/// `EdgeKind::Dotted`); `tables[i]` describes `scene.nodes[i]`;
/// `cards[j]` holds the (from, to) cardinalities of `scene.edges[j]`.
/// Relationship labels already sit clear of the crow's feet.
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

fn assemble(d: &ErDiagram, tables: Vec<ErTable>, mut scene: Scene) -> ErScene {
    // The synthetic graph is built strictly in entity/relation
    // order, so indexes align 1:1. This is load-bearing for every
    // consumer of ErScene; fail loudly if a refactor breaks it.
    assert_eq!(scene.nodes.len(), d.entities.len());
    assert_eq!(scene.edges.len(), d.relations.len());
    let cards: Vec<(Card, Card)> = d
        .relations
        .iter()
        .map(|r| (r.card_from, r.card_to))
        .collect();
    // Here, not in the SVG writer, so GUI painters get the same spots.
    labels_off_glyphs(&mut scene, &cards);
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
    g.direction = d.direction;
    for e in &d.entities {
        g.ensure_node(&e.name, Some(e.title().to_string()), Some(Shape::Rect));
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
        // The crow's feet are drawn here, not by the flowchart writer:
        // tell the layout how much line they take (with the air a label
        // keeps from them), so a short relationship still fits both
        // glyphs and its label between.
        if let Some(e) = g.edges.last_mut() {
            e.end_room = (
                glyph_reach(r.card_from) + GLYPH_AIR,
                glyph_reach(r.card_to) + GLYPH_AIR,
            );
        }
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
    let mut comment_w = 0.0f64;
    for r in &rows {
        ty_w = ty_w.max(text_width(&r.ty));
        name_w = name_w.max(text_width(&r.name));
        keys_w = keys_w.max(text_width(&r.keys));
        comment_w = comment_w.max(text_width(&r.comment));
    }
    let mut row_w = ty_w + COL_GAP + name_w;
    if keys_w > 0.0 {
        row_w += COL_GAP + keys_w;
    }
    if comment_w > 0.0 {
        row_w += COL_GAP + comment_w;
    }
    let title = e.title().to_string();
    let w = (text_width(&title) + 28.0)
        .max(row_w + 2.0 * PAD)
        .max(120.0);
    let h = HEADER_H + rows.len() as f64 * ROW_H;
    (
        ErTable {
            name: e.name.clone(),
            title,
            rows,
            ty_col_w: ty_w,
            name_col_w: name_w,
            keys_col_w: keys_w,
            comment_col_w: comment_w,
        },
        (w, h),
    )
}

fn row_of(a: &Attr) -> ErRow {
    let (comment, comment_full) = match &a.comment {
        Some(c) => fit_comment(c),
        None => (String::new(), None),
    };
    ErRow {
        ty: a.ty.clone(),
        name: a.name.clone(),
        keys: a.keys.iter().map(|k| k.tag()).collect::<Vec<_>>().join(","),
        comment,
        comment_full,
    }
}

/// A comment as drawn, plus the whole text when it had to be cut to
/// [`COMMENT_MAX_W`] (at a character, trailing blanks dropped).
fn fit_comment(c: &str) -> (String, Option<String>) {
    if text_width(c) <= COMMENT_MAX_W {
        return (c.to_string(), None);
    }
    let mut cut = c.to_string();
    while !cut.is_empty() && text_width(&format!("{cut}…")) > COMMENT_MAX_W {
        cut.pop();
    }
    (format!("{}…", cut.trim_end()), Some(c.to_string()))
}

/// Serialise any ER arrangement (automatic or dragged) to SVG.
///
/// A node dragged left of or above the origin shifts the whole drawing
/// right/down onto the canvas, so SVG coordinates can then differ from
/// scene coordinates by that offset: map pointer events through the
/// rendered SVG, as with [`crate::scene::to_svg`].
pub fn to_svg(es: &ErScene) -> String {
    to_svg_with(es, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(es: &ErScene, opts: &SvgOptions) -> String {
    to_svg_a11y_with(es, &A11y::default(), opts)
}

/// [`to_svg_with`] plus mermaid's `accTitle` / `accDescr` (see [`A11y`]).
pub(crate) fn to_svg_a11y_with(es: &ErScene, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve("Entity-relationship diagram");
    let fitted = fit_top_left(&es.scene);
    let sc = &*fitted;
    let mut s = String::new();
    svg_open_a11y(&mut s, sc.width, sc.height, 13, &title, desc, opts);

    // Relationship lines (under the tables). A rerouted relationship
    // follows its waypoints, like a flowchart edge.
    for e in &sc.edges {
        let dash = if matches!(e.kind, EdgeKind::Dotted) {
            " stroke-dasharray=\"5 4\""
        } else {
            ""
        };
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>\n",
            edge_path_d(e, |p| p),
            EDGE_COLOR,
            LINE_W,
            dash
        ));
    }

    // Entity tables — each entity gets a stable accent color.
    for (i, (n, table)) in sc.nodes.iter().zip(&es.tables).enumerate() {
        let accent = crate::style::accent(i);
        let x0 = n.x - n.w / 2.0;
        let y0 = n.y - n.h / 2.0;
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"#ffffff\"/>\n",
            x0, y0, n.w, n.h
        ));
        // The band is square where it meets the body: rounding its
        // bottom corners left white notches at the join. Its top
        // corners are quadratic, so the path is all coordinate pairs.
        if table.rows.is_empty() {
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
                 fill=\"{}\"/>\n",
                x0, y0, n.w, HEADER_H, accent
            ));
        } else {
            let (x1, y1, r) = (x0 + n.w, y0 + HEADER_H, 4.0);
            s.push_str(&format!(
                "<path d=\"M {:.1} {:.1} Q {:.1} {:.1}, {:.1} {:.1} L {:.1} {:.1} \
                 Q {:.1} {:.1}, {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z\" fill=\"{}\"/>\n",
                x0,
                y0 + r,
                x0,
                y0,
                x0 + r,
                y0,
                x1 - r,
                y0,
                x1,
                y0,
                x1,
                y0 + r,
                x1,
                y1,
                x0,
                y1,
                accent
            ));
        }
        // Header text takes white or dark ink by contrast with the
        // accent band (white failed AA on six of the eight accents).
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"{}\">{}</text>\n",
            n.x,
            y0 + HEADER_H / 2.0,
            crate::style::text_on(accent),
            escape(&table.title)
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
                x0 + table.name_x(),
                tyc,
                "",
                TEXT_COLOR,
                &row.name,
            );
            if !row.keys.is_empty() {
                text_at(
                    &mut s,
                    x0 + table.keys_end_x(n.w),
                    tyc,
                    " text-anchor=\"end\" font-weight=\"bold\"",
                    EDGE_COLOR,
                    &row.keys,
                );
            }
            if !row.comment.is_empty() {
                // Muted like the type, italic so the two never read as
                // one column of data. A cut comment keeps its whole
                // text as the tooltip.
                let tip = row
                    .comment_full
                    .as_ref()
                    .map(|full| format!("<title>{}</title>", escape(full)))
                    .unwrap_or_default();
                s.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" font-style=\"italic\" \
                     fill=\"{}\">{}{}</text>\n",
                    x0 + table.comment_x(),
                    tyc,
                    TYPE_COLOR,
                    tip,
                    escape(&row.comment)
                ));
            }
        }
        // The outline goes on last, over the band and the row rules
        // that would otherwise cover its inner half.
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            x0, y0, n.w, n.h, accent
        ));
    }

    // Crow's feet over the tables: their prongs end ON the border, and
    // a table painted later would cover the tips (or, at a header, sink
    // them into the band).
    for (e, &(card_from, card_to)) in sc.edges.iter().zip(&es.cards) {
        let [a, b] = end_glyphs(e, (card_from, card_to));
        write_glyph(&mut s, &a);
        write_glyph(&mut s, &b);
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
             stroke-width=\"{}\"/>\n",
            a.0, a.1, b.0, b.1, EDGE_COLOR, LINE_W
        ));
    }
    if let Some((c, r)) = g.circle {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#ffffff\" stroke=\"{}\" \
             stroke-width=\"{}\"/>\n",
            c.0, c.1, r, EDGE_COLOR, LINE_W
        ));
    }
}

/// The (from, to) crow's feet of a relationship, laid on its DRAWN
/// curve: a rerouted edge's `bezier` is a degenerate chord whose
/// controls sit on its ends, which gives no direction at all.
fn end_glyphs(e: &SceneEdge, (card_from, card_to): (Card, Card)) -> [Glyph; 2] {
    let (p0, toward0) = edge_end_chord(e, true, GLYPH_CHORD);
    let (p1, toward1) = edge_end_chord(e, false, GLYPH_CHORD);
    [glyph(p0, toward0, card_from), glyph(p1, toward1, card_to)]
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
    let tick = |k: f64| [p(k, -TICK_HALF), p(k, TICK_HALF)];
    let fork = || {
        let f = p(FORK_LEN, 0.0);
        [[f, p(0.0, -FORK_HALF)], [f, p(0.0, FORK_HALF)], [f, e]]
    };

    match card {
        Card::One => Glyph {
            segments: vec![tick(9.0), tick(15.0)],
            circle: None,
        },
        Card::ZeroOne => Glyph {
            segments: vec![tick(9.0)],
            circle: Some((p(21.0, 0.0), CIRCLE_R)),
        },
        Card::ZeroMany => Glyph {
            segments: fork().to_vec(),
            circle: Some((p(FORK_LEN + 2.5 + CIRCLE_R, 0.0), CIRCLE_R)),
        },
        Card::OneMany => {
            let mut segments = fork().to_vec();
            segments.push(tick(FORK_LEN + 5.0));
            Glyph {
                segments,
                circle: None,
            }
        }
    }
}

/// How far along the line, from the entity border, `card`'s glyph
/// reaches (its stroke included) — measured off [`glyph`] itself, so
/// the two never disagree.
fn glyph_reach(card: Card) -> f64 {
    let g = glyph((0.0, 0.0), (1.0, 0.0), card);
    let ends = g.segments.iter().flatten().map(|p| p.0);
    let circle = g.circle.map(|(c, r)| c.0 + r);
    ends.chain(circle).fold(0.0, f64::max) + LINE_W / 2.0
}

/// Axis-aligned box `(x0, y0, x1, y1)`.
type Rect = (f64, f64, f64, f64);

/// The box a label `w` wide centred at `c` is drawn in.
fn label_rect(c: (f64, f64), w: f64) -> Rect {
    (
        c.0 - w / 2.0,
        c.1 - LABEL_BOX_H / 2.0,
        c.0 + w / 2.0,
        c.1 + LABEL_BOX_H / 2.0,
    )
}

/// Does `r`, grown by `air`, touch any stroke of `g`?
fn rect_hits_glyph(r: Rect, g: &Glyph, air: f64) -> bool {
    let k = air + LINE_W / 2.0;
    let (x0, y0, x1, y1) = (r.0 - k, r.1 - k, r.2 + k, r.3 + k);
    g.segments
        .iter()
        .any(|&[a, b]| seg_aabb(a, b, x0, y0, x1, y1))
        || g.circle.is_some_and(|(c, rad)| {
            let dx = (x0 - c.0).max(c.0 - x1).max(0.0);
            let dy = (y0 - c.1).max(c.1 - y1).max(0.0);
            dx.hypot(dy) <= rad
        })
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// Keep every relationship label off the crow's feet. The flowchart
/// label stages know only arrowheads (and an ER line has none), so a
/// label lands on its line's midpoint whatever the glyphs cover — a
/// pill over the circle of `o{` reads as `|{`, and on a short line it
/// hides both ends. A label that covers a glyph moves to the middle
/// of the stretch the glyphs leave free (the layout reserved one via
/// `Edge::end_room`), else beside the line; a clear label stays put.
fn labels_off_glyphs(sc: &mut Scene, cards: &[(Card, Card)]) {
    let glyphs: Vec<[Glyph; 2]> = sc
        .edges
        .iter()
        .zip(cards)
        .map(|(e, &c)| end_glyphs(e, c))
        .collect();
    let boxes: Vec<Rect> = sc
        .nodes
        .iter()
        .map(|n| {
            (
                n.x - n.w / 2.0,
                n.y - n.h / 2.0,
                n.x + n.w / 2.0,
                n.y + n.h / 2.0,
            )
        })
        .collect();
    let mut grow: Option<Rect> = None;
    for j in 0..sc.edges.len() {
        let Some((_, at, w)) = sc.edges[j].label.clone() else {
            continue;
        };
        let clear = |r: Rect| glyphs.iter().flatten().all(|g| !rect_hits_glyph(r, g, GLYPH_AIR));
        if clear(label_rect(at, w)) {
            continue;
        }
        let others: Vec<Rect> = sc
            .edges
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != j)
            .filter_map(|(_, e)| e.label.as_ref().map(|(_, c, w)| label_rect(*c, *w)))
            .collect();
        // Cost of a spot that clears the glyphs: boxes and labels it
        // would cover.
        let crowd = |r: Rect| {
            boxes.iter().chain(&others).filter(|&&o| rects_overlap(r, o)).count()
        };
        let reach = (
            glyph_reach(cards[j].0) + GLYPH_AIR,
            glyph_reach(cards[j].1) + GLYPH_AIR,
        );
        let best = free_stretch_spots(&sc.edges[j], reach, w)
            .into_iter()
            .map(|c| (c, label_rect(c, w)))
            .filter(|&(_, r)| clear(r))
            .min_by_key(|&(_, r)| crowd(r));
        if let Some((c, r)) = best {
            if let Some(lbl) = &mut sc.edges[j].label {
                lbl.1 = c;
            }
            grow = Some(match grow {
                Some(g) => (g.0.min(r.0), g.1.min(r.1), g.2.max(r.2), g.3.max(r.3)),
                None => r,
            });
        }
    }
    // A label moved beside a line near the canvas edge still fits.
    if let Some(r) = grow {
        sc.width = sc.width.max(r.2 + crate::scene::MARGIN);
        sc.height = sc.height.max(r.3 + crate::scene::MARGIN);
    }
}

/// Candidate centres for a `w`-wide label on edge `e` whose glyphs
/// take `reach` = (from, to) of its drawn length, best first: on the
/// line, from the middle of the free stretch outwards, then beside the
/// line at that middle, either side — just past the label's own
/// half-extent across the line.
fn free_stretch_spots(e: &SceneEdge, reach: (f64, f64), w: f64) -> Vec<(f64, f64)> {
    let pts = edge_polyline(e);
    let mut acc = vec![0.0];
    for s in pts.windows(2) {
        let d = (s[1].0 - s[0].0).hypot(s[1].1 - s[0].1);
        acc.push(acc[acc.len() - 1] + d);
    }
    let total = acc[acc.len() - 1];
    // (point, unit tangent) at arc length `t`.
    let at = |t: f64| -> ((f64, f64), (f64, f64)) {
        let t = t.clamp(0.0, total);
        let i = acc.partition_point(|&a| a < t).clamp(1, pts.len().max(2) - 1);
        let (a, b) = (pts[i - 1], pts[i]);
        let seg = acc[i] - acc[i - 1];
        let k = if seg > 0.0 { (t - acc[i - 1]) / seg } else { 0.0 };
        let len = seg.max(1e-9);
        (
            (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k),
            ((b.0 - a.0) / len, (b.1 - a.1) / len),
        )
    };
    let (lo, hi) = (reach.0, total - reach.1);
    let mid = if lo <= hi { (lo + hi) / 2.0 } else { total / 2.0 };
    let mut out = Vec::new();
    if lo <= hi {
        let mut step = 0.0;
        while mid - step >= lo || mid + step <= hi {
            for t in [mid - step, mid + step] {
                if (lo..=hi).contains(&t) {
                    out.push(at(t).0);
                }
            }
            step += 2.0;
        }
    }
    let (p, u) = at(mid);
    let nv = (-u.1, u.0);
    // Half the label's extent along the normal, plus air.
    let half = (w / 2.0 * nv.0).abs() + (LABEL_BOX_H / 2.0 * nv.1).abs() + GLYPH_AIR + LINE_W;
    for side in [1.0, -1.0] {
        for extra in [0.0, FORK_HALF, 2.0 * FORK_HALF] {
            let d = side * (half + extra);
            out.push((p.0 + nv.0 * d, p.1 + nv.1 * d));
        }
    }
    out
}

