//! UML class diagram rendering: classes drawn as three-compartment
//! boxes (name / fields / methods), relationships drawn with UML end
//! glyphs (triangles, diamonds, open arrows), notes drawn as yellow
//! cards.
//!
//! Like the `er` module, layout is fully reused from the flowchart
//! pipeline: each class becomes one node of a synthetic graph whose
//! size comes from its member list, each relationship becomes one
//! edge, and `scene_sized` produces the final geometry. Relationships
//! rank in SOURCE order — the left operand of `Animal <|-- Dog` leads
//! — as mermaid ranks them, so parents, interfaces and wholes sit
//! above their children in a top-down diagram. Each note is one more
//! node, after the classes, linked to its class by one more edge,
//! after the relationships.
//!
//! Interactive apps use the same API shape as `scene`/`er`: [`scene`]
//! for automatic layout, [`route`] to follow dragged class positions,
//! [`to_svg`] to export any arrangement. [`rel_head`] and
//! [`card_spots`] expose the end glyphs and cardinality positions as
//! plain geometry so GUI painters draw exactly what SVG exports.

use std::collections::HashMap;

use crate::layout::{text_width, LINE_H};
use crate::model::{
    generics_to_angles, Class, ClassDiagram, Direction, EdgeEnds, EdgeKind, Graph, Marker, Member,
    RelKind, Shape,
};
use crate::scene::{
    edge_end_chord, edge_path_d, edge_polyline, escape, fit_top_left, route_sized, scene_sized,
    svg_label_box, svg_open_a11y, svg_text_multiline, A11y, Scene, SceneEdge, SceneNode,
    SvgOptions, EDGE_COLOR, TEXT_COLOR,
};

/// Name-compartment height in pixels (without stereotype lines).
pub const HEADER_H: f64 = 28.0;
/// Extra name-compartment height per stereotype line (`«interface»`).
pub const ANNOTATION_H: f64 = 14.0;
/// Member (field / method) row height in pixels.
pub const ROW_H: f64 = 20.0;
/// Horizontal padding inside a class box.
pub const PAD: f64 = 10.0;
/// Height of an empty compartment: a band tall enough to read as an
/// empty attributes / operations compartment, not as a doubled border.
pub const EMPTY_H: f64 = 12.0;
/// Space between a compartment's divider and its first row, and
/// between its last row and the next divider (filled compartments).
pub const COMP_PAD: f64 = 4.0;
/// Padding around the text of a note card.
pub const NOTE_PAD: f64 = 10.0;
/// Font size of cardinalities and stereotype lines.
const SMALL_FONT: f64 = 11.0;
/// Gap between a class and a note that [`route`] places for it when
/// the caller passes no position for the note.
const NOTE_GAP: f64 = 40.0;
/// Most an end glyph tilts away from the outward normal of the face it
/// sits on. Past this the curve's final stretch grazes the face (a
/// back-edge bow reaching sideways), and a glyph along it would lie
/// half inside the box.
const HEAD_MAX_TILT_DEG: f64 = 35.0;
/// Clearance kept between a cardinality and every stroke, glyph, box
/// and label around it.
const CARD_CLEAR: f64 = 3.0;

/// One display row of a class box (pre-formatted with its glyph).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ClassRow {
    /// Visibility glyph + member text as mermaid shows it, e.g.
    /// `"+name: String"`, `"+total() : Money"`, `"+List<int> items"`.
    pub text: String,
    /// A static member (`$` classifier): UML draws it underlined.
    pub is_static: bool,
    /// An abstract member (`*` classifier): UML draws it in italics.
    pub is_abstract: bool,
}

/// Drawing data for one class box. Index-aligned with the
/// corresponding [`Scene::nodes`] entry, whose rect is the box's
/// outer bounds.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ClassBox {
    /// Header text: the class name, with `<T>` for a generic class.
    pub name: String,
    /// Stereotype lines drawn above the name, guillemets included
    /// (`«interface»`).
    pub annotations: Vec<String>,
    pub fields: Vec<ClassRow>,
    pub methods: Vec<ClassRow>,
}

impl ClassBox {
    /// Height of the name compartment: [`HEADER_H`] plus
    /// [`ANNOTATION_H`] per stereotype line.
    pub fn header_h(&self) -> f64 {
        HEADER_H + self.annotations.len() as f64 * ANNOTATION_H
    }
}

/// Drawing data for one note card: `notes[k]` describes
/// `scene.nodes[boxes.len() + k]`, whose rect is the card.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NoteBox {
    /// Note text; `\n` separates lines.
    pub text: String,
    /// The class the note is attached to (`note for Class`), as an
    /// index into [`ClassScene::boxes`]; `None` = free-standing.
    pub class: Option<usize>,
    /// The dotted connector to that class, as an index into
    /// `scene.edges` (after every relationship).
    pub edge: Option<usize>,
}

/// Geometry + drawing data for a whole class diagram. `scene` holds
/// class boxes and relationship curves (dashed via `EdgeKind::Dotted`
/// for realization / dependency); `boxes[i]` describes
/// `scene.nodes[i]`; `rels[j]` mirrors `scene.edges[j]`'s UML style
/// and cardinalities. Notes follow: `notes[k]` describes
/// `scene.nodes[boxes.len() + k]`, and each attached note's dotted
/// connector (`EdgeKind::DottedOpen`) follows the relationships in
/// `scene.edges`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ClassScene {
    pub scene: Scene,
    pub boxes: Vec<ClassBox>,
    pub rels: Vec<RelStyle>,
    pub notes: Vec<NoteBox>,
}

/// End style + cardinalities for one relationship, aligned with a
/// scene edge. The glyph always sits at the `to` end — the end of
/// the edge's curve (the parser normalises relations so the decorated
/// side is `to`; the layout ranks them in source order and the scene
/// edge is turned back to `from` → `to` after).
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
    let b = build(d);
    let sc = scene_sized(&b.g, &b.sizes);
    assemble(d, b, sc)
}

/// Re-route relationships for custom class positions (drags).
/// `centers[i]` = centre of class i in final coordinates, then
/// optionally one centre per note (the order `scene.nodes` lists them
/// in). Without note centres each note is placed [`NOTE_GAP`] before
/// its class along the flow (free notes before the topmost class), so
/// pass the scene's full node list to keep notes where they are.
///
/// # Panics
///
/// When `centers` holds neither one centre per class nor one per
/// class and note.
pub fn route(d: &ClassDiagram, centers: &[(f64, f64)]) -> ClassScene {
    let b = build(d);
    let n = d.classes.len();
    assert!(
        centers.len() == n || centers.len() == n + d.notes.len(),
        "number of positions must match the classes (or classes + notes)"
    );
    let mut all = centers.to_vec();
    if all.len() == n {
        all.extend(note_fallback(d, centers, &b.sizes));
    }
    let sc = route_sized(&b.g, &all, &b.sizes);
    assemble(d, b, sc)
}

/// Note centres for a [`route`] call that passed class centres only:
/// an attached note [`NOTE_GAP`] before its class along the flow, free
/// notes side by side [`NOTE_GAP`] before the first class.
fn note_fallback(d: &ClassDiagram, centers: &[(f64, f64)], sizes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = d.classes.len();
    // Unit step "against the flow" and the breadth axis.
    let (back, across) = match d.direction {
        Direction::TD => ((0.0, -1.0), (1.0, 0.0)),
        Direction::BT => ((0.0, 1.0), (1.0, 0.0)),
        Direction::LR => ((-1.0, 0.0), (0.0, 1.0)),
        Direction::RL => ((1.0, 0.0), (0.0, 1.0)),
    };
    let along = |(w, h): (f64, f64)| (w * back.0).abs() + (h * back.1).abs();
    let breadth = |(w, h): (f64, f64)| (w * across.0).abs() + (h * across.1).abs();
    let dot = |p: (f64, f64), u: (f64, f64)| p.0 * u.0 + p.1 * u.1;
    // The class edge farthest against the flow, and the breadth start.
    let (mut lead, mut start) = (f64::NEG_INFINITY, f64::INFINITY);
    for (c, &s) in centers.iter().zip(sizes) {
        lead = lead.max(dot(*c, back) + along(s) / 2.0);
        start = start.min(dot(*c, across) - breadth(s) / 2.0);
    }
    if !lead.is_finite() {
        (lead, start) = (0.0, 0.0);
    }
    let mut cursor = start;
    d.notes
        .iter()
        .enumerate()
        .map(|(k, note)| {
            let s = sizes[n + k];
            let (l, b) = match note.class {
                Some(ci) => {
                    let c = centers[ci];
                    let l = dot(c, back) + along(sizes[ci]) / 2.0 + NOTE_GAP + along(s) / 2.0;
                    (l, dot(c, across))
                }
                None => {
                    let b = cursor + breadth(s) / 2.0;
                    cursor += breadth(s) + NOTE_GAP;
                    (lead + NOTE_GAP + along(s) / 2.0, b)
                }
            };
            (
                back.0 * l + across.0 * b,
                back.1 * l + across.1 * b,
            )
        })
        .collect()
}

/// Turn the layout scene into the class scene: relationship edges go
/// back to their semantic direction (`from` → `to`, glyph at the end)
/// and to the line style hosts read (`Dotted` = dashed, else `Open`).
fn assemble(d: &ClassDiagram, b: Built, mut scene: Scene) -> ClassScene {
    // The synthetic graph is built strictly in class / note / relation
    // / connector order, so indexes align 1:1 — load-bearing for every
    // consumer.
    assert_eq!(scene.nodes.len(), d.classes.len() + d.notes.len());
    assert_eq!(scene.edges.len(), d.relations.len() + b.connectors);
    for (e, r) in scene.edges.iter_mut().zip(&d.relations) {
        if r.head_left {
            reverse_edge(e);
        }
        e.kind = if r.dashed {
            EdgeKind::Dotted
        } else {
            EdgeKind::Open
        };
        e.ends = EdgeEnds::default();
    }
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
        boxes: b.boxes,
        rels,
        notes: b.notes,
    }
}

/// The same edge drawn the other way round: ends, curve and route
/// reversed; the label stays where it is.
fn reverse_edge(e: &mut SceneEdge) {
    std::mem::swap(&mut e.from, &mut e.to);
    e.bezier.reverse();
    e.waypoints.reverse();
}

/// The synthetic layout graph plus drawing data.
struct Built {
    g: Graph,
    boxes: Vec<ClassBox>,
    notes: Vec<NoteBox>,
    /// Node sizes, index-parallel with `g.nodes`.
    sizes: Vec<(f64, f64)>,
    /// Number of note connector edges after the relationships.
    connectors: usize,
}

/// Synthetic flowchart graph + box / note drawing data + node sizes.
///
/// INVARIANT: `g.nodes[i]` corresponds to `d.classes[i]`, then
/// `g.nodes[classes + k]` to `d.notes[k]`; `g.edges[j]` to
/// `d.relations[j]`, then one connector per attached note — class
/// names are unique (`ensure_class` dedupes), note ids cannot collide
/// with them (`-` is no class-name character), `ensure_node` appends
/// in call order, and edges are added in relation order. `assemble`
/// asserts the counts.
fn build(d: &ClassDiagram) -> Built {
    let mut g = Graph::default();
    g.direction = d.direction;
    for c in &d.classes {
        g.ensure_node(&c.name, Some(c.display_name()), Some(Shape::Rect));
    }
    let n = d.classes.len();
    for (k, note) in d.notes.iter().enumerate() {
        g.ensure_node(&format!("note-{k}"), Some(note.text.clone()), Some(Shape::Rect));
    }
    for r in &d.relations {
        // Laid out in SOURCE order (mermaid's id1 → id2), so the left
        // operand ranks first. The glyph end is marked for the label
        // and port stages (a triangle / diamond as wider than an
        // arrowhead); `assemble` restores the scene edge's
        // direction and style.
        let (a, b) = r.source_order();
        let glyph = match r.kind {
            RelKind::Inheritance
            | RelKind::Realization
            | RelKind::Composition
            | RelKind::Aggregation => Some(Marker::Circle),
            RelKind::Association | RelKind::Dependency => Some(Marker::Arrow),
            RelKind::Link => None,
        };
        let ends = if r.head_left {
            EdgeEnds {
                start: glyph,
                end: None,
            }
        } else {
            EdgeEnds {
                start: None,
                end: glyph,
            }
        };
        let kind = if r.dashed {
            EdgeKind::DottedOpen
        } else {
            EdgeKind::Open
        };
        g.add_edge_with_ends(a, b, r.label.clone(), kind, ends);
    }
    let mut connectors = 0;
    let mut notes = Vec::with_capacity(d.notes.len());
    for (k, note) in d.notes.iter().enumerate() {
        // Note → class, as mermaid links them: the card ranks before
        // its class.
        let edge = note.class.map(|ci| {
            g.add_edge(n + k, ci, None, EdgeKind::DottedOpen);
            connectors += 1;
            d.relations.len() + connectors - 1
        });
        notes.push(NoteBox {
            text: note.text.clone(),
            class: note.class,
            edge,
        });
    }

    let mut boxes = Vec::with_capacity(n);
    let mut sizes = Vec::with_capacity(n + d.notes.len());
    for c in &d.classes {
        let (b, size) = box_of(c);
        boxes.push(b);
        sizes.push(size);
    }
    for note in &d.notes {
        sizes.push(note_size(&note.text));
    }
    Built {
        g,
        boxes,
        notes,
        sizes,
        connectors,
    }
}

fn box_of(c: &Class) -> (ClassBox, (f64, f64)) {
    let fields: Vec<ClassRow> = c.fields.iter().map(|m| row_of(m, false)).collect();
    let methods: Vec<ClassRow> = c.methods.iter().map(|m| row_of(m, true)).collect();
    let annotations: Vec<String> = c.annotations.iter().map(|a| format!("«{a}»")).collect();
    let mut member_w = 0.0f64;
    for r in fields.iter().chain(&methods) {
        member_w = member_w.max(text_width(&r.text));
    }
    let name = c.display_name();
    // The name is drawn bold: size it bold, as the layout sizes bold
    // runs (`**…**` = ×1.06).
    let mut w = (text_width(&name) * 1.06 + 28.0)
        .max(member_w + 2.0 * PAD)
        .max(96.0);
    for a in &annotations {
        w = w.max(text_width(a) * SMALL_FONT / 13.0 + 28.0);
    }
    let b = ClassBox {
        name,
        annotations,
        fields,
        methods,
    };
    let h = b.header_h() + compartment_h(b.fields.len()) + compartment_h(b.methods.len());
    (b, (w, h))
}

/// A member row as mermaid displays it (`parseMember` /
/// `getDisplayDetails`): a method's return type follows `" : "`,
/// `~T~` generics read `<T>`, and a `$` (static) or `*` (abstract)
/// classifier — right after a method's `)` or at the very end — is
/// dropped from the text and kept as the row's style.
fn row_of(m: &Member, method: bool) -> ClassRow {
    let mut text = m.text.trim();
    let (mut is_static, mut is_abstract) = (false, false);
    let mut classify = |c: char| match c {
        '$' => is_static = true,
        '*' => is_abstract = true,
        _ => {}
    };
    if let Some(c) = text.chars().last().filter(|c| matches!(c, '$' | '*')) {
        classify(c);
        text = text[..text.len() - 1].trim_end();
    }
    let body = match text.rfind(')').filter(|_| method) {
        Some(p) => {
            let sig = &text[..=p];
            let mut ret = text[p + 1..].trim();
            if let Some(c) = ret.chars().next().filter(|c| matches!(c, '$' | '*')) {
                classify(c);
                ret = ret[1..].trim_start();
            }
            if ret.is_empty() {
                sig.to_string()
            } else {
                format!("{sig} : {ret}")
            }
        }
        None => text.to_string(),
    };
    ClassRow {
        text: format!("{}{}", m.visibility.glyph(), generics_to_angles(&body)),
        is_static,
        is_abstract,
    }
}

/// Height of one member compartment: real rows plus [`COMP_PAD`] above
/// and below, or an [`EMPTY_H`] band when empty (UML shows the empty
/// attributes / operations compartment). Row `i` of a compartment
/// whose divider is at `top` is centred on
/// `top + COMP_PAD + (i + 0.5) * ROW_H`.
pub fn compartment_h(rows: usize) -> f64 {
    if rows == 0 {
        EMPTY_H
    } else {
        rows as f64 * ROW_H + 2.0 * COMP_PAD
    }
}

/// A note card's size: its widest line and its line count, padded.
fn note_size(text: &str) -> (f64, f64) {
    let lines = text.split('\n').count() as f64;
    (
        text_width(text) + 2.0 * NOTE_PAD,
        lines * LINE_H + 2.0 * NOTE_PAD - 4.0,
    )
}

/// Serialise any class arrangement (automatic or dragged) to SVG.
///
/// A node dragged left of or above the origin shifts the whole drawing
/// right/down onto the canvas, so SVG coordinates can then differ from
/// scene coordinates by that offset: map pointer events through the
/// rendered SVG, as with [`crate::scene::to_svg`].
pub fn to_svg(cs: &ClassScene) -> String {
    to_svg_with(cs, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(cs: &ClassScene, opts: &SvgOptions) -> String {
    to_svg_a11y_with(cs, &A11y::default(), opts)
}

/// [`to_svg_with`] plus mermaid's `accTitle` / `accDescr` (see [`A11y`]).
pub(crate) fn to_svg_a11y_with(cs: &ClassScene, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve("Class diagram");
    let fitted = fit_top_left(&cs.scene);
    let sc = &*fitted;
    let mut s = String::new();
    svg_open_a11y(&mut s, sc.width, sc.height, 13, &title, desc, opts);

    // Relationship lines and note connectors first, under the boxes —
    // drawn as routed (a rerouted edge follows its waypoints around
    // the boxes, never straight through them).
    for (j, e) in sc.edges.iter().enumerate() {
        let (dash, width) = if j >= cs.rels.len() {
            (" stroke-dasharray=\"2 3\"", "1.2")
        } else if matches!(e.kind, EdgeKind::Dotted) {
            (" stroke-dasharray=\"6 4\"", "1.6")
        } else {
            ("", "1.6")
        };
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>\n",
            edge_path_d(e, |p| p),
            EDGE_COLOR,
            width,
            dash
        ));
    }

    // Class boxes — each gets a stable accent color.
    for (i, (n, b)) in sc.nodes.iter().zip(&cs.boxes).enumerate() {
        write_box(&mut s, n, b, crate::style::accent(i));
    }
    // Note cards.
    let note = crate::style::NOTE_STYLE;
    for (n, nb) in sc.nodes[cs.boxes.len()..].iter().zip(&cs.notes) {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"2\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\"/>\n",
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.w,
            n.h,
            note.fill,
            note.stroke
        ));
        svg_text_multiline(&mut s, n.x, n.y, TEXT_COLOR, &escape(&nb.text));
    }

    // End glyphs over the boxes they touch (a glyph under its box
    // loses whatever part the box covers), then the cardinalities.
    let heads = heads_of(sc, &cs.rels);
    for h in &heads {
        write_head(&mut s, h);
    }
    for ((rel, spots), e) in cs.rels.iter().zip(place_cards(sc, &cs.rels, &heads)).zip(&sc.edges) {
        let _ = e;
        for (text, at) in [(&rel.from_card, spots[0]), (&rel.to_card, spots[1])] {
            if let (Some(text), Some(at)) = (text, at) {
                card_text(&mut s, at, text, sc.width, sc.height);
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

/// One class box: white body, accent name band (only its top corners
/// rounded, so it meets the body square), stereotype lines and name,
/// compartments with accent dividers, then the border on top so no
/// divider breaks it.
fn write_box(s: &mut String, n: &SceneNode, b: &ClassBox, accent: &str) {
    let x0 = n.x - n.w / 2.0;
    let y0 = n.y - n.h / 2.0;
    let (x1, hh) = (x0 + n.w, b.header_h());
    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
         fill=\"#ffffff\"/>\n",
        x0, y0, n.w, n.h
    ));
    // Name compartment: rounded top corners, square bottom ones.
    s.push_str(&format!(
        "<path d=\"M {:.1} {:.1} L {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1} L {:.1} {:.1} \
         Q {:.1} {:.1} {:.1} {:.1} L {:.1} {:.1} Z\" fill=\"{}\"/>\n",
        x0,
        y0 + hh,
        x0,
        y0 + 4.0,
        x0,
        y0,
        x0 + 4.0,
        y0,
        x1 - 4.0,
        y0,
        x1,
        y0,
        x1,
        y0 + 4.0,
        x1,
        y0 + hh,
        accent
    ));
    // Header text takes white or dark ink by contrast with the
    // accent band (white failed AA on six of the eight accents).
    let ink = crate::style::text_on(accent);
    for (k, a) in b.annotations.iter().enumerate() {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-size=\"{}\" fill=\"{}\">{}</text>\n",
            n.x,
            y0 + 7.0 + (k as f64 + 0.5) * ANNOTATION_H,
            SMALL_FONT,
            ink,
            escape(a)
        ));
    }
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
         font-weight=\"bold\" fill=\"{}\">{}</text>\n",
        n.x,
        y0 + hh - HEADER_H / 2.0,
        ink,
        escape(&b.name)
    ));

    // Divider under the name, then fields, then a divider, then
    // methods. Empty compartments still get their band + line.
    let fields_top = y0 + hh;
    divider(s, x0, fields_top, n.w, accent);
    write_rows(s, &b.fields, x0, fields_top);
    let methods_top = fields_top + compartment_h(b.fields.len());
    divider(s, x0, methods_top, n.w, accent);
    write_rows(s, &b.methods, x0, methods_top);

    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
         fill=\"none\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
        x0, y0, n.w, n.h, accent
    ));
}

/// A compartment divider in the box's accent, a little thinner than
/// the border (the pale `LABEL_BORDER` hairline read as a rendering
/// glitch, 1.4:1 on white).
fn divider(s: &mut String, x0: f64, y: f64, w: f64, accent: &str) {
    s.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
         stroke-width=\"1.2\"/>\n",
        x0,
        y,
        x0 + w,
        y,
        accent
    ));
}

fn write_rows(s: &mut String, rows: &[ClassRow], x0: f64, top: f64) {
    for (i, row) in rows.iter().enumerate() {
        let ty = top + COMP_PAD + i as f64 * ROW_H + ROW_H / 2.0;
        let style = match (row.is_static, row.is_abstract) {
            (true, true) => " text-decoration=\"underline\" font-style=\"italic\"",
            (true, false) => " text-decoration=\"underline\"",
            (false, true) => " font-style=\"italic\"",
            (false, false) => "",
        };
        // Drawn in the default font (not monospace) so the width the
        // box was sized with (`text_width`, a Helvetica metric) matches
        // what is rendered — otherwise long members overflow the box.
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\"{}>{}</text>\n",
            x0 + PAD,
            ty,
            TEXT_COLOR,
            style,
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
            "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\" \
             stroke-linejoin=\"round\"/>\n",
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

fn card_text(s: &mut String, at: (f64, f64), text: &str, cw: f64, ch: f64) {
    // Clamp to the canvas so a long cardinality can't spill off-edge
    // (the scene bbox is sized for boxes/edges, not these labels).
    let hw = card_w(text) / 2.0;
    let px = at.0.clamp(hw, (cw - hw).max(hw));
    let py = at.1.clamp(7.0, (ch - 7.0).max(7.0));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
         fill=\"{}\" font-size=\"{}\">{}</text>\n",
        px,
        py,
        TEXT_COLOR,
        SMALL_FONT,
        escape(text)
    ));
}

/// Width of a cardinality's box (its text at [`SMALL_FONT`], padded).
fn card_w(text: &str) -> f64 {
    text_width(text) * SMALL_FONT / 13.0 + 4.0
}
/// Height of a cardinality's box.
const CARD_H: f64 = 13.0;

/// Length of an end glyph along its edge, and its half width.
fn glyph_size(kind: RelKind) -> (f64, f64) {
    match kind {
        RelKind::Inheritance | RelKind::Realization => (14.0, 8.0),
        RelKind::Composition | RelKind::Aggregation => (18.0, 6.5),
        RelKind::Association | RelKind::Dependency => (12.0, 7.0),
        RelKind::Link => (0.0, 0.0),
    }
}

/// Index of the scene node with `id`.
fn node_index(sc: &Scene) -> HashMap<&str, usize> {
    sc.nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect()
}

/// Outward unit normal of the face of `n` nearest to `p` (an edge
/// end on its border).
fn face_normal(n: &SceneNode, p: (f64, f64)) -> (f64, f64) {
    let faces = [
        ((p.0 - (n.x - n.w / 2.0)).abs(), (-1.0, 0.0)),
        ((p.0 - (n.x + n.w / 2.0)).abs(), (1.0, 0.0)),
        ((p.1 - (n.y - n.h / 2.0)).abs(), (0.0, -1.0)),
        ((p.1 - (n.y + n.h / 2.0)).abs(), (0.0, 1.0)),
    ];
    faces
        .iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|f| f.1)
        .unwrap_or((0.0, -1.0))
}

/// The direction an end glyph points along, away from its class: the
/// chord of the curve's last `reach` units as DRAWN (waypoints
/// included), tilted toward the face's outward normal when it grazes
/// the face by more than [`HEAD_MAX_TILT_DEG`]; the normal itself for
/// a curve of zero length.
fn end_axis(e: &SceneEdge, at_start: bool, reach: f64, node: Option<&SceneNode>) -> ((f64, f64), (f64, f64)) {
    let (tip, toward) = edge_end_chord(e, at_start, reach.max(1.0));
    let (dx, dy) = (toward.0 - tip.0, toward.1 - tip.1);
    let len = dx.hypot(dy);
    let Some(n) = node.map(|n| face_normal(n, tip)) else {
        return (tip, if len > 1e-9 { (dx / len, dy / len) } else { (0.0, -1.0) });
    };
    if len <= 1e-9 {
        return (tip, n);
    }
    let u = (dx / len, dy / len);
    let max = HEAD_MAX_TILT_DEG.to_radians();
    if u.0 * n.0 + u.1 * n.1 >= max.cos() {
        return (tip, u);
    }
    let side = if n.0 * u.1 - n.1 * u.0 >= 0.0 { 1.0 } else { -1.0 };
    let (c, s) = (max.cos(), max.sin() * side);
    (tip, (n.0 * c - n.1 * s, n.0 * s + n.1 * c))
}

/// The UML end glyph of relationship `j` of `cs`, exactly as
/// [`to_svg`] draws it (before any shift for nodes dragged past the
/// origin): at the end of the drawn curve, on the `to` class, pointing
/// along the curve's final stretch — never collapsed for a rerouted
/// edge, never lying inside the box when a curve grazes its face.
pub fn rel_head(cs: &ClassScene, j: usize) -> Head {
    let idx = node_index(&cs.scene);
    let e = &cs.scene.edges[j];
    let kind = cs.rels[j].kind;
    let node = idx.get(e.to.as_str()).map(|&i| &cs.scene.nodes[i]);
    let (tip, u) = end_axis(e, false, glyph_size(kind).0, node);
    head(tip, (tip.0 + u.0, tip.1 + u.1), kind)
}

fn heads_of(sc: &Scene, rels: &[RelStyle]) -> Vec<Head> {
    let idx = node_index(sc);
    sc.edges
        .iter()
        .zip(rels)
        .map(|(e, rel)| {
            let node = idx.get(e.to.as_str()).map(|&i| &sc.nodes[i]);
            let (tip, u) = end_axis(e, false, glyph_size(rel.kind).0, node);
            head(tip, (tip.0 + u.0, tip.1 + u.1), rel.kind)
        })
        .collect()
}

/// Centres of the cardinalities of every relationship of `cs` as
/// `[from_card, to_card]` (`None` where there is none), exactly where
/// [`to_svg`] draws them (before its canvas clamp and any shift for
/// nodes dragged past the origin), in [`SMALL_FONT`]-sized text.
pub fn card_spots(cs: &ClassScene) -> Vec<[Option<(f64, f64)>; 2]> {
    let heads = heads_of(&cs.scene, &cs.rels);
    place_cards(&cs.scene, &cs.rels, &heads)
}

/// Axis-aligned box `(x0, y0, x1, y1)`.
type Rect = (f64, f64, f64, f64);

fn rect_hits_segment(r: Rect, a: (f64, f64), b: (f64, f64)) -> bool {
    // Liang–Barsky clip of the segment against the box.
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - r.0),
        (dx, r.2 - a.0),
        (-dy, a.1 - r.1),
        (dy, r.3 - a.1),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

fn rects_meet(a: Rect, b: Rect) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// Place each cardinality beside its end: just outside the face, past
/// the side of the glyph, on the side the line leaves away from (or
/// toward the nearer corner for a line leaving straight). A spot that
/// would touch any stroke, glyph, box, label or earlier cardinality
/// gives way to the other side, then to spots farther out; failing all,
/// the first spot stands.
fn place_cards(sc: &Scene, rels: &[RelStyle], heads: &[Head]) -> Vec<[Option<(f64, f64)>; 2]> {
    let idx = node_index(sc);
    let lines: Vec<Vec<(f64, f64)>> = sc.edges.iter().map(edge_polyline).collect();
    let mut solid: Vec<Rect> = sc
        .nodes
        .iter()
        .map(|n| (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0))
        .collect();
    for e in &sc.edges {
        if let Some((_, m, w)) = &e.label {
            solid.push((m.0 - w / 2.0, m.1 - 10.0, m.0 + w / 2.0, m.1 + 10.0));
        }
    }
    let mut glyph_segs: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for h in heads {
        let k = h.polygon.len();
        for i in 0..k {
            glyph_segs.push((h.polygon[i], h.polygon[(i + 1) % k]));
        }
        glyph_segs.extend(h.segments.iter().map(|s| (s[0], s[1])));
    }
    let mut placed: Vec<Rect> = Vec::new();
    let mut out = vec![[None, None]; rels.len()];
    for (j, (e, rel)) in sc.edges.iter().zip(rels).enumerate() {
        for (slot, text, at_start) in [(0, &rel.from_card, true), (1, &rel.to_card, false)] {
            let Some(text) = text else { continue };
            let id = if at_start { &e.from } else { &e.to };
            let node = idx.get(id.as_str()).map(|&i| &sc.nodes[i]);
            let (glen, ghalf) = if at_start {
                (0.0, 0.0)
            } else {
                glyph_size(rel.kind)
            };
            let (tip, u) = end_axis(e, at_start, glen.max(12.0), node);
            let n = node.map_or(u, |n| face_normal(n, tip));
            let t = (-n.1, n.0);
            let (w, h) = (card_w(text), CARD_H);
            // Card extents along the normal and the face.
            let (ext_n, ext_t) = (
                (w * n.0).abs() + (h * n.1).abs(),
                (w * t.0).abs() + (h * t.1).abs(),
            );
            let un = (u.0 * n.0 + u.1 * n.1).max(0.2);
            let ut = u.0 * t.0 + u.1 * t.1;
            let off_face = node.map_or(0.0, |nd| {
                let c = (nd.x, nd.y);
                (tip.0 - c.0) * t.0 + (tip.1 - c.1) * t.1
            });
            let first = if ut.abs() > 0.15 {
                -ut.signum()
            } else if off_face.abs() > 1.0 {
                off_face.signum()
            } else {
                1.0
            };
            let mut best: Option<((f64, f64), Rect)> = None;
            'search: for depth in [0.0, 10.0, 20.0, 34.0] {
                for side in [first, -first] {
                    let dn = 2.0 + depth + ext_n / 2.0;
                    // Where the line is at that depth, then past the
                    // glyph (within its length) or the stroke.
                    let line_t = ut / un * dn;
                    let half = if dn - ext_n / 2.0 < glen { ghalf } else { 0.0 };
                    let dt = line_t + side * (half + CARD_CLEAR + 1.0 + ext_t / 2.0);
                    let c = (
                        tip.0 + n.0 * dn + t.0 * dt,
                        tip.1 + n.1 * dn + t.1 * dt,
                    );
                    let r = (c.0 - w / 2.0, c.1 - h / 2.0, c.0 + w / 2.0, c.1 + h / 2.0);
                    if best.is_none() {
                        best = Some((c, r));
                    }
                    let grown = (
                        r.0 - CARD_CLEAR,
                        r.1 - CARD_CLEAR,
                        r.2 + CARD_CLEAR,
                        r.3 + CARD_CLEAR,
                    );
                    let free = !solid.iter().any(|&b| rects_meet(r, b))
                        && !placed.iter().any(|&b| rects_meet(grown, b))
                        && !lines
                            .iter()
                            .any(|l| l.windows(2).any(|s| rect_hits_segment(grown, s[0], s[1])))
                        && !glyph_segs
                            .iter()
                            .any(|&(a, b)| rect_hits_segment(grown, a, b));
                    if free {
                        best = Some((c, r));
                        break 'search;
                    }
                }
            }
            if let Some((c, r)) = best {
                placed.push(r);
                out[j][slot] = Some(c);
            }
        }
    }
    out
}

/// UML end glyph at endpoint `e` of a curve whose adjacent control
/// point is `c` (giving the inward tangent). The glyph points at `e`
/// (the class border). [`rel_head`] places it on a scene edge the way
/// [`to_svg`] does.
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

