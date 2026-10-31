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
    edge_path_d, edge_polyline, edge_stroke_width, escape, fit_top_left, route_sized, scene_sized,
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
/// line (`k`) or across it. Across, a glyph stays 12px wide like the
/// flowchart's cross/circle markers: two relationships may end
/// `GLYPH_PORT_GAP` (14px) apart on one side, and wider glyphs there
/// would merge. Along the line they spread out more than before (bars
/// 6px apart, a 15px fork) so each part reads on its own.
const TICK_HALF: f64 = 6.0;
/// Where a fork's three prongs meet (they spread to `FORK_HALF` at
/// the border).
const FORK_LEN: f64 = 15.0;
const FORK_HALF: f64 = 6.0;
const CIRCLE_R: f64 = 5.0;
/// Clear line between a fork's tip and the circle or bar beyond it.
const FORK_GAP: f64 = 2.0;
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
    assemble(d, tables, scene_sized(&g, &sizes), &g)
}

/// Re-route relationships for custom entity positions (drags).
/// `centers[i]` = centre of entity i in final coordinates.
pub fn route(d: &ErDiagram, centers: &[(f64, f64)]) -> ErScene {
    let (g, tables, sizes) = build(d);
    assemble(d, tables, route_sized(&g, centers, &sizes), &g)
}

fn assemble(d: &ErDiagram, tables: Vec<ErTable>, mut scene: Scene, g: &Graph) -> ErScene {
    // The synthetic graph is built strictly in entity/relation
    // order, so indexes align 1:1. This is load-bearing for every
    // consumer of ErScene; fail loudly if a refactor breaks it.
    assert_eq!(scene.nodes.len(), d.entities.len());
    assert_eq!(scene.edges.len(), d.relations.len());
    // A relationship laid out reversed (see `layout_flips`) is turned
    // back: `scene.edges[j]` runs from `relations[j].from` to `.to`.
    for (e, (se, r)) in g.edges.iter().zip(scene.edges.iter_mut().zip(&d.relations)) {
        if e.from != r.from {
            std::mem::swap(&mut se.from, &mut se.to);
            se.bezier.reverse();
            se.waypoints.reverse();
        }
    }
    let cards: Vec<(Card, Card)> = d
        .relations
        .iter()
        .map(|r| (r.card_from, r.card_to))
        .collect();
    // Here, not in the SVG writer, so GUI painters get the same spots.
    labels_off_glyphs(&mut scene, &cards);
    // The width `to_svg` strokes, kept on the edges so picking
    // (`Scene::edge_at` / `hits_in_rect`) covers the very same pixels.
    for e in &mut scene.edges {
        e.paint_width = Some(LINE_W);
    }
    ErScene {
        scene,
        tables,
        cards,
    }
}

/// Synthetic flowchart graph + table drawing data + node sizes.
///
/// INVARIANT: `g.nodes[i]` corresponds to `d.entities[i]` and
/// `g.edges[j]` to `d.relations[j]` (laid out reversed when
/// [`layout_flips`] says so; `assemble` turns it back). This holds
/// because entity names are unique (`ErDiagram::ensure_entity` dedupes
/// at parse time), `ensure_node` appends in call order, and edges are
/// added in relation order. `assemble` asserts the node/edge counts.
fn build(d: &ErDiagram) -> (Graph, Vec<ErTable>, Vec<(f64, f64)>) {
    let mut g = Graph::default();
    g.direction = d.direction;
    for e in &d.entities {
        g.ensure_node(&e.name, Some(e.title().to_string()), Some(Shape::Rect));
    }
    for (r, flip) in d.relations.iter().zip(layout_flips(d)) {
        // Dotted carries the non-identifying dash into the Scene so
        // any consumer (SVG writer, GUI painter) styles it right.
        let kind = if r.identifying {
            EdgeKind::Open
        } else {
            EdgeKind::Dotted
        };
        // The crow's feet are drawn here, not by the flowchart writer:
        // tell the layout how much line they take (with the air a label
        // keeps from them), so a short relationship still fits both
        // glyphs and its label between.
        let mut ends = [(r.from, r.card_from), (r.to, r.card_to)];
        if flip {
            ends.reverse();
        }
        let [(a, card_a), (b, card_b)] = ends;
        g.add_edge(a, b, r.label.clone(), kind);
        if let Some(e) = g.edges.last_mut() {
            e.end_room = (
                glyph_reach(card_a) + GLYPH_AIR,
                glyph_reach(card_b) + GLYPH_AIR,
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

/// Which relationships the layout takes reversed: the back edges of a
/// depth-first walk in entity and relation order, so the synthetic
/// graph has no cycle. A relationship has no direction to show (both
/// ends carry a cardinality and neither an arrow), so the flowchart's
/// back-edge bow — a loop round the side that comes in nearly along
/// the border, crushing the crow's foot there — buys nothing: like
/// mermaid, a cycle is drawn with plain lines. Self-relationships
/// stay loops.
fn layout_flips(d: &ErDiagram) -> Vec<bool> {
    let n = d.entities.len();
    let mut out_edges: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (j, r) in d.relations.iter().enumerate() {
        if r.from != r.to {
            out_edges[r.from].push((j, r.to));
        }
    }
    // 0 = unvisited, 1 = on the walk's stack, 2 = done.
    let mut state = vec![0u8; n];
    let mut flip = vec![false; d.relations.len()];
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        state[root] = 1;
        // (entity, next out-edge to try)
        let mut stack = vec![(root, 0usize)];
        while let Some(top) = stack.last_mut() {
            let (v, k) = *top;
            let Some(&(j, w)) = out_edges[v].get(k) else {
                state[v] = 2;
                stack.pop();
                continue;
            };
            top.1 += 1;
            match state[w] {
                0 => {
                    state[w] = 1;
                    stack.push((w, 0));
                }
                1 => flip[j] = true,
                _ => {}
            }
        }
    }
    flip
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
            edge_stroke_width(e), // LINE_W, as `assemble` set it
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
            text_at(&mut s, x0 + table.name_x(), tyc, "", TEXT_COLOR, &row.name);
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

/// The (from, to) crow's feet of relationship `e`, exactly as
/// [`to_svg`] draws them: laid along its DRAWN curve (see
/// [`crate::scene::SceneEdge`]: a rerouted edge's `bezier` is a
/// degenerate chord, its route is in `waypoints`), each bar and the
/// circle centred on the stroke and square to it, the fork's prongs
/// spread square to the stroke's direction where it meets the entity.
/// GUI painters call this per edge, with `cards` from
/// [`ErScene::cards`], to match the SVG.
pub fn end_glyphs(e: &SceneEdge, cards: (Card, Card)) -> [Glyph; 2] {
    let mut pts = edge_polyline(e);
    let from = glyph_along(&Walk::new(&pts), cards.0);
    pts.reverse();
    [from, glyph_along(&Walk::new(&pts), cards.1)]
}

/// Crow's foot geometry at endpoint `e` of a curve whose adjacent
/// bezier control point is `c` (which gives the inward tangent), for
/// a line straight over the glyph's length — [`end_glyphs`] lays it
/// along a curved one. Fork prongs touch the entity border at `e`;
/// bars and circles sit further along the line.
pub fn glyph(e: (f64, f64), c: (f64, f64), card: Card) -> Glyph {
    let (dx, dy) = (c.0 - e.0, c.1 - e.1);
    let len = dx.hypot(dy).max(1e-6);
    glyph_along(&Walk::new(&[e, (e.0 + dx / len, e.1 + dy / len)]), card)
}

/// [`glyph`] along the curve `w` walks, starting at the entity.
fn glyph_along(w: &Walk, card: Card) -> Glyph {
    // Point `k` along the curve, `t` to the side of it (square to the
    // curve there).
    let p = |k: f64, t: f64| {
        let (q, u) = w.at(k);
        (q.0 - u.1 * t, q.1 + u.0 * t)
    };
    let tick = |k: f64| [p(k, -TICK_HALF), p(k, TICK_HALF)];
    let fork = || {
        let (tip, f, mid) = (p(0.0, 0.0), p(FORK_LEN, 0.0), p(FORK_LEN / 2.0, 0.0));
        // The middle prong is the stroke itself, in two pieces so it
        // stays on a curving line.
        vec![
            [f, p(0.0, -FORK_HALF)],
            [f, p(0.0, FORK_HALF)],
            [f, mid],
            [mid, tip],
        ]
    };
    let beyond_fork = FORK_LEN + FORK_GAP;
    match card {
        Card::One => Glyph {
            segments: vec![tick(8.0), tick(14.0)],
            circle: None,
        },
        Card::ZeroOne => Glyph {
            segments: vec![tick(8.0)],
            circle: Some((p(14.0 + FORK_GAP + CIRCLE_R, 0.0), CIRCLE_R)),
        },
        Card::ZeroMany => Glyph {
            segments: fork(),
            circle: Some((p(beyond_fork + CIRCLE_R, 0.0), CIRCLE_R)),
        },
        Card::OneMany => {
            let mut segments = fork();
            segments.push(tick(beyond_fork + 3.0));
            Glyph {
                segments,
                circle: None,
            }
        }
    }
}

/// Arc-length walk along a polyline: the point and unit direction at
/// any distance from its start; past the far end it runs on straight.
struct Walk<'a> {
    pts: &'a [(f64, f64)],
    /// Arc length at each point.
    acc: Vec<f64>,
}

impl<'a> Walk<'a> {
    fn new(pts: &'a [(f64, f64)]) -> Self {
        let mut acc = Vec::with_capacity(pts.len());
        let mut sum = 0.0;
        for (i, p) in pts.iter().enumerate() {
            if i > 0 {
                sum += (p.0 - pts[i - 1].0).hypot(p.1 - pts[i - 1].1);
            }
            acc.push(sum);
        }
        Walk { pts, acc }
    }

    fn len(&self) -> f64 {
        self.acc.last().copied().unwrap_or(0.0)
    }

    /// `(point, unit direction)` at arc length `s`. The direction is
    /// the chord over ±[`Walk::SPAN`] around it, so sample joints of a
    /// densely drawn curve do not jitter it; a zero-length curve
    /// points along +x.
    fn at(&self, s: f64) -> ((f64, f64), (f64, f64)) {
        let q = self.point(s);
        let (a, b) = (self.point(s - Self::SPAN), self.point(s + Self::SPAN));
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len <= 1e-9 {
            return (q, (1.0, 0.0));
        }
        (q, (dx / len, dy / len))
    }

    const SPAN: f64 = 2.0;

    fn point(&self, s: f64) -> (f64, f64) {
        let n = self.pts.len();
        if n == 0 {
            return (0.0, 0.0);
        }
        if n == 1 {
            return self.pts[0];
        }
        let s = s.max(0.0);
        // The last segment of non-zero length carries a walk past the
        // end on along its direction.
        let i = self.acc.partition_point(|&a| a < s).clamp(1, n - 1);
        let (mut i0, mut i1) = (i - 1, i);
        if s > self.len() {
            i1 = n - 1;
            i0 = (0..i1)
                .rev()
                .find(|&k| self.acc[k] < self.acc[i1])
                .unwrap_or(0);
        }
        let (a, b) = (self.pts[i0], self.pts[i1]);
        let seg = self.acc[i1] - self.acc[i0];
        if seg <= 1e-12 {
            return a;
        }
        let k = (s - self.acc[i0]) / seg;
        (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k)
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

/// The box holding every stroke of `g`, grown by `pad`.
fn glyph_box(g: &Glyph, pad: f64) -> Rect {
    let mut b = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    let mut add = |x0: f64, y0: f64, x1: f64, y1: f64| {
        b = (b.0.min(x0), b.1.min(y0), b.2.max(x1), b.3.max(y1));
    };
    for p in g.segments.iter().flatten() {
        add(p.0, p.1, p.0, p.1);
    }
    if let Some((c, r)) = g.circle {
        add(c.0 - r, c.1 - r, c.0 + r, c.1 + r);
    }
    (b.0 - pad, b.1 - pad, b.2 + pad, b.3 + pad)
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
    // Each glyph with its bounding box (strokes and air included): a
    // label is tested stroke by stroke only against glyphs near it.
    let glyphs: Vec<(Glyph, Rect)> = sc
        .edges
        .iter()
        .zip(cards)
        .flat_map(|(e, &c)| end_glyphs(e, c))
        .map(|g| {
            let b = glyph_box(&g, GLYPH_AIR + LINE_W);
            (g, b)
        })
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
        let clear = |r: Rect| {
            glyphs
                .iter()
                .all(|(g, b)| !rects_overlap(r, *b) || !rect_hits_glyph(r, g, GLYPH_AIR))
        };
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
            boxes
                .iter()
                .chain(&others)
                .filter(|&&o| rects_overlap(r, o))
                .count()
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
    let walk = Walk::new(&pts);
    let total = walk.len();
    // Not `clamp`: that panics when a NaN centre from a host makes
    // `total` NaN.
    let at = |t: f64| walk.at(t.max(0.0).min(total));
    let (lo, hi) = (reach.0, total - reach.1);
    let mid = if lo <= hi {
        (lo + hi) / 2.0
    } else {
        total / 2.0
    };
    let mut out = Vec::new();
    if lo <= hi {
        out.push(at(mid).0);
        let mut step = 2.0;
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
            if l.starts_with("<rect")
                && l.contains("height=\"20\"")
                && l.contains("stroke=\"#d5d9ec\"")
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
                    for (line, label) in [
                        ("--", "has many"),
                        ("..", "x"),
                        ("--", "a much longer role"),
                    ] {
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
        let d = er(
            "erDiagram\np[Person] {\n  string name\n}\nc[\"Customer Account\"]\np ||--o{ c : has\n",
        );
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
                wps.push((
                    s[0].0 + (s[1].0 - s[0].0) * t,
                    s[0].1 + (s[1].1 - s[0].1) * t,
                ));
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
            assert!(len > 5.0, "collapsed glyph stroke {a:?}-{b:?}");
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

    /// On a curving end a rigid glyph aimed down a chord tilts off the
    /// stroke: every bar must cross the DRAWN curve square, centred on
    /// it, and the fork's prongs spread square to the stroke where it
    /// meets the entity.
    #[test]
    fn glyphs_follow_a_curving_end() {
        let es = scene(&er(
            "erDiagram\nHUB ||--o{ A : a\nHUB }|--|| B : b\nHUB |o--o| C : c\n",
        ));
        for (e, &cards) in es.scene.edges.iter().zip(&es.cards) {
            let pts = edge_polyline(e);
            let seg_dist = |q: (f64, f64), a: (f64, f64), b: (f64, f64)| {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let l2 = dx * dx + dy * dy;
                let t = if l2 > 0.0 {
                    (((q.0 - a.0) * dx + (q.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (a.0 + t * dx - q.0).hypot(a.1 + t * dy - q.1)
            };
            let near = |q: (f64, f64)| {
                pts.windows(2)
                    .map(|w| seg_dist(q, w[0], w[1]))
                    .fold(f64::INFINITY, f64::min)
            };
            for g in end_glyphs(e, cards) {
                for &[a, b] in &g.segments {
                    let len = (b.0 - a.0).hypot(b.1 - a.1);
                    if (len - 2.0 * TICK_HALF).abs() > 1e-6 {
                        continue; // a prong
                    }
                    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                    assert!(near(mid) < 0.3, "bar off the stroke by {}", near(mid));
                    // Both ends equally far off the curve: square to it.
                    assert!((near(a) - near(b)).abs() < 0.6, "bar tilted");
                }
                if let Some((c, _)) = g.circle {
                    assert!(near(c) < 0.3, "circle off the stroke by {}", near(c));
                }
            }
        }
        // The SVG draws exactly these glyphs (hosts paint the same).
        let svg = to_svg(&es);
        let (_, strokes, _) = drawn_marks(&svg);
        let mine: usize = es
            .scene
            .edges
            .iter()
            .zip(&es.cards)
            .flat_map(|(e, &c)| end_glyphs(e, c))
            .map(|g| g.segments.len())
            .sum();
        assert_eq!(strokes.len(), mine);
    }

    /// A relationship closing a cycle was laid out as a flowchart
    /// back edge: a bow round the side that came in almost along the
    /// border, crushing its crow's foot. It is drawn as a plain line
    /// now, still running from its own `from` entity to its `to`.
    #[test]
    fn cycle_is_drawn_with_plain_lines() {
        let d = er("erDiagram\n\
                    EMPLOYEE |o--o{ EMPLOYEE : manages\n\
                    EMPLOYEE ||--|{ DEPT : \"works in\"\n\
                    DEPT ||--o{ EMPLOYEE : employs\n\
                    DEPT }o--|| SITE : \"located at\"\n\
                    SITE ||..o{ EMPLOYEE : hosts\n");
        assert_eq!(layout_flips(&d), [false, false, true, false, true]);
        let s0 = scene(&d);
        let laid: Vec<(f64, f64)> = s0.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut moved = laid.clone();
        moved[2].0 += 60.0;
        for es in [s0, route(&d, &moved)] {
            let right = es
                .scene
                .nodes
                .iter()
                .map(|n| n.x + n.w / 2.0)
                .fold(f64::MIN, f64::max);
            for (e, r) in es.scene.edges.iter().zip(&d.relations) {
                let (a, b) = (&es.scene.nodes[r.from], &es.scene.nodes[r.to]);
                assert_eq!(
                    (e.from.as_str(), e.to.as_str()),
                    (a.id.as_str(), b.id.as_str())
                );
                let on = |p: (f64, f64), n: &crate::scene::SceneNode| {
                    let (dx, dy) = ((p.0 - n.x).abs() - n.w / 2.0, (p.1 - n.y).abs() - n.h / 2.0);
                    dx.max(dy).abs() < 1.0
                };
                let pts = edge_polyline(e);
                assert!(on(pts[0], a), "{} starts off {}", e.from, a.id);
                assert!(on(pts[pts.len() - 1], b), "{} ends off {}", e.to, b.id);
                if r.from != r.to {
                    let far = pts.iter().map(|p| p.0).fold(f64::MIN, f64::max);
                    assert!(
                        far <= right + 1.0,
                        "{}->{} bows out to x={far}",
                        e.from,
                        e.to
                    );
                }
            }
            assert_no_glyph_under_a_label(&to_svg(&es), "cycle");
        }
    }

    #[test]
    fn glyphs_fit_between_neighbouring_ports() {
        // Two ends may share a side GLYPH_PORT_GAP apart: their glyphs,
        // strokes included, must not touch.
        for c in [Card::One, Card::ZeroOne, Card::ZeroMany, Card::OneMany] {
            let g = glyph((0.0, 0.0), (1.0, 0.0), c);
            let mut w = g
                .segments
                .iter()
                .flatten()
                .map(|p| p.1.abs())
                .fold(0.0, f64::max);
            if let Some((_, r)) = g.circle {
                w = w.max(r);
            }
            assert!(
                2.0 * w + LINE_W < crate::scene::GLYPH_PORT_GAP,
                "{c:?} is {} across",
                2.0 * w
            );
        }
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
