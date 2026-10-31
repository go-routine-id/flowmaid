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
//! above their children in a top-down diagram; as dagre does, a source
//! ranks just above its nearest target and a hierarchy's children stay
//! in one row. Each note is one more node, after the classes, linked to
//! its class by one more edge, after the relationships (so it ranks
//! just above its class).
//!
//! Interactive apps use the same API shape as `scene`/`er`: [`scene`]
//! for automatic layout, [`route`] to follow dragged class positions,
//! [`to_svg`] to export any arrangement. [`rel_head`] and
//! [`card_spots`] expose the end glyphs and cardinality positions as
//! plain geometry so GUI painters draw exactly what SVG exports.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};

use crate::edgepass::{refit, Mode};
use crate::labels::{avoid_label_overlaps, LABEL_GAP, LABEL_ROUTE_WORK, LABEL_STATIC_WORK};
use crate::layout::{text_width, GAP_L, LABEL_BOX_H, LINE_H};
use crate::model::{
    generics_to_angles, Class, ClassDiagram, Direction, EdgeEnds, EdgeKind, Graph, Marker, Member,
    RelKind, Shape,
};
use crate::scene::{
    edge_end_chord, edge_path_d, edge_polyline, edge_stroke_width, escape, fit_top_left,
    route_sized, sample_cubic, scene_sized, svg_label_box, svg_open_a11y, svg_text_multiline, A11y,
    Scene, SceneEdge, SceneNode, SvgOptions, EDGE_COLOR, TEXT_COLOR,
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
const NOTE_PAD: f64 = 10.0;
/// Stroke width of relationship lines (and their glyphs' outlines).
const REL_W: f64 = 1.6;
/// Stroke width of the dashed links from notes to their classes.
const NOTE_LINK_W: f64 = 1.2;
/// Gap between ranks when any relationship carries a label or a
/// cardinality: a short edge then holds a label, two cardinalities,
/// an end glyph and the jog of its route, which the flowchart's 60 px
/// crammed together (mermaid leaves 160+).
const RANK_GAP: f64 = 84.0;
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
    /// A lollipop interface ([`crate::model::Class::lollipop`]): only
    /// `name` is drawn, centred in the node's rect, and no box — the
    /// rect is the text's room, its relation's circle just outside it.
    pub lollipop: bool,
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
/// `scene.nodes[i]` for `i < boxes.len()`; `rels[j]` mirrors
/// `scene.edges[j]`'s UML style and cardinalities for `j < rels.len()`.
/// Notes follow: `notes[k]` describes `scene.nodes[boxes.len() + k]`,
/// and each attached note's dotted connector (`EdgeKind::DottedOpen`,
/// see [`NoteBox::edge`]) follows the relationships in `scene.edges`.
///
/// So with notes, `scene.nodes` / `scene.edges` hold more entries than
/// `boxes` / `rels`: index those only below their length (or zip them
/// with the scene's lists). A host that paints no notes can clear
/// [`ClassDiagram::notes`] before [`scene`] / [`route`] to get one box
/// per node and one style per edge, as before notes were drawn.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ClassScene {
    pub scene: Scene,
    pub boxes: Vec<ClassBox>,
    pub rels: Vec<RelStyle>,
    pub notes: Vec<NoteBox>,
}

/// End style + cardinalities for one relationship, aligned with a
/// scene edge. The glyph (`kind`) sits at the `to` end — the end of
/// the edge's curve (the parser normalises relations so the decorated
/// side is `to`; the layout ranks them in source order and the scene
/// edge is turned back to `from` → `to` after). A two-way relation
/// also has one at the start of the curve: `from_kind`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RelStyle {
    pub kind: RelKind,
    pub from_card: Option<String>,
    pub to_card: Option<String>,
    /// The glyph at the `from` end of a two-way relation
    /// ([`crate::model::ClassRel::from_kind`]); [`rel_tail`] places it.
    pub from_kind: Option<RelKind>,
}

/// The end glyph for one relationship: a closed polygon (triangle or
/// diamond), open-arrow segments or a lollipop circle. Same geometry
/// feeds both the SVG writer and GUI painters.
#[derive(Debug, Clone, Default)]
pub struct Head {
    /// Closed polygon (triangle or diamond); empty when none.
    pub polygon: Vec<(f64, f64)>,
    /// Fill the polygon solid (composition) vs. hollow white.
    pub filled: bool,
    /// Open-arrow segments (association / dependency).
    pub segments: Vec<[(f64, f64); 2]>,
    /// A hollow white circle as `(centre, radius)`, touching the class
    /// border (the lollipop, [`RelKind::Lollipop`]); `None` when none.
    pub circle: Option<((f64, f64), f64)>,
}

/// Automatic layout for a class diagram.
pub fn scene(d: &ClassDiagram) -> ClassScene {
    let b = build(d);
    let sc = scene_sized(&b.g, &b.sizes);
    assemble(d, b, sc, Mode::Static)
}

/// Re-route relationships for custom class positions (drags).
/// `centers[i]` = centre of class i in final coordinates, then
/// optionally one centre per note (the order `scene.nodes` lists them
/// in). Without note centres each note is placed 40 px before its
/// class along the flow (free notes before the topmost class), so pass
/// the scene's full node list to keep notes where they are.
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
    assemble(d, b, sc, Mode::Routed)
}

/// Note centres for a [`route`] call that passed class centres only:
/// an attached note [`NOTE_GAP`] before its class along the flow, free
/// notes side by side [`NOTE_GAP`] before the first class.
fn note_fallback(
    d: &ClassDiagram,
    centers: &[(f64, f64)],
    sizes: &[(f64, f64)],
) -> Vec<(f64, f64)> {
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
            (back.0 * l + across.0 * b, back.1 * l + across.1 * b)
        })
        .collect()
}

/// Turn the layout scene into the class scene: relationship edges go
/// back to their semantic direction (`from` → `to`, glyph at the end),
/// are redrawn orthogonal ([`orthogonalize`]) with their labels moved
/// off whatever the new lines brought them onto, and take the line
/// style hosts read (`Dotted` = dashed, else `Open`). `mode` tells an
/// automatic layout from a drag (the label stage's work budget, and
/// whether the canvas may shift to hold self-loops moved to a box's
/// other side).
fn assemble(d: &ClassDiagram, b: Built, mut scene: Scene, mode: Mode) -> ClassScene {
    // The synthetic graph is built strictly in class / note / relation
    // / connector order, so indexes align 1:1 — load-bearing for every
    // consumer.
    assert_eq!(scene.nodes.len(), d.classes.len() + d.notes.len());
    assert_eq!(scene.edges.len(), d.relations.len() + b.connectors);
    for (e, r) in scene.edges.iter_mut().zip(&d.relations) {
        if r.head_left {
            reverse_edge(e);
            std::mem::swap(&mut e.ends.start, &mut e.ends.end);
        }
    }
    let carded: Vec<bool> = d
        .relations
        .iter()
        .map(|r| r.from_card.is_some() || r.to_card.is_some())
        .collect();
    part_self_loops(&mut scene, &carded);
    let decos: Vec<[EndDeco; 2]> = d
        .relations
        .iter()
        .map(|r| {
            let card = |c: &Option<String>| c.as_deref().map(|t| (card_w(t), CARD_H));
            [
                EndDeco {
                    half: r.from_kind.map_or(0.0, |k| glyph_size(k).1),
                    card: card(&r.from_card),
                },
                EndDeco {
                    half: glyph_size(r.kind).1,
                    card: card(&r.to_card),
                },
            ]
        })
        .collect();
    orthogonalize(&mut scene, &decos, mode);
    // Rerouting moved labels onto their new lines, where one may now
    // cover another label or sit under a stroke; the glyph ends are
    // still marked, so labels keep off them too.
    let budget = match mode {
        Mode::Static => LABEL_STATIC_WORK,
        Mode::Routed => LABEL_ROUTE_WORK,
    };
    avoid_label_overlaps(&scene.nodes, &scene.clusters, &mut scene.edges, budget);
    refit(&mut scene, mode);
    for (e, r) in scene.edges.iter_mut().zip(&d.relations) {
        e.kind = if r.dashed {
            EdgeKind::Dotted
        } else {
            EdgeKind::Open
        };
        e.ends = EdgeEnds::default();
    }
    // The widths `to_svg` strokes, kept on the edges so picking
    // (`Scene::edge_at` / `hits_in_rect`) covers the very same pixels.
    for (j, e) in scene.edges.iter_mut().enumerate() {
        e.paint_width = Some(if j < d.relations.len() {
            REL_W
        } else {
            NOTE_LINK_W
        });
    }
    let rels = d
        .relations
        .iter()
        .map(|r| RelStyle {
            kind: r.kind,
            from_card: r.from_card.clone(),
            to_card: r.to_card.clone(),
            from_kind: r.from_kind,
        })
        .collect();
    ClassScene {
        scene,
        boxes: b.boxes,
        rels,
        notes: b.notes,
    }
}

/// How far a self-loop's two ends sit either side of its place on the
/// face, and how far its curve reaches out (its controls' offset along
/// the face is [`LOOP_LIFT`]) — the layout's loop.
const LOOP_HALF: f64 = 8.0;
const LOOP_REACH: f64 = 48.0;
const LOOP_LIFT: f64 = 28.0;
/// How much each loop nested round another on one face widens its ends,
/// its reach and its lift: its ends stay clear of the inner loop's
/// glyph, and its curve clear of the inner curve.
const LOOP_NEST: (f64, f64, f64) = (12.0, 28.0, 16.0);

/// Draw each class's self-loops apart and clear of the rest. The
/// layout stacks a node's loops on one side, each starting where the
/// one before ends, so two loops wind round each other and an end glyph
/// can land on the other loop's end (both glyphs on one point when the
/// source order turns one loop round); and it sets a loop's label on
/// the loop's apex, where on a small box the chip hides most of the
/// loop. Here the loops alternate between that side and the opposite
/// one, and loops sharing a side nest, the outer one's ends past the
/// inner one's, their glyphs at opposite ends. Loops whose edge has a
/// cardinality (`carded[j]`) nest outside the rest, so each card sits
/// beside its own end instead of out past the outer loops, next to
/// theirs. The labels of a side sit past its outermost loop, side by
/// side along the face. A side whose loops would run into another box
/// (a sibling close beside the class) or over another edge's end gives
/// them to a clear face ([`loop_faces`]). A class's only loop keeps the
/// layout's curve while it stays on the layout's side.
fn part_self_loops(sc: &mut Scene, carded: &[bool]) {
    let idx = node_index(sc);
    let mut loops: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (j, e) in sc.edges.iter().enumerate() {
        if e.from == e.to && e.meta.curve.is_none() {
            if let Some(&i) = idx.get(e.from.as_str()) {
                loops.entry(i).or_default().push(j);
            }
        }
    }
    let rects: Vec<Rect> = sc.nodes.iter().map(node_rect).collect();
    for (i, js) in loops {
        let n = sc.nodes[i].clone();
        let side = face_normal(&n, sc.edges[js[0]].bezier[0]);
        // The faces in order of preference: the layout's side, the
        // opposite one, then the two across it.
        let outs = [
            side,
            (-side.0, -side.1),
            (-side.1, side.0),
            (side.1, -side.0),
        ];
        // The other edges' ends on each face: a loop there crosses them.
        let mut ends = [0usize; 4];
        for e in sc.edges.iter().filter(|e| e.from != e.to) {
            for (id, p) in [(&e.from, e.bezier[0]), (&e.to, e.bezier[3])] {
                if *id == n.id {
                    let f = face_normal(&n, p);
                    if let Some(k) = outs.iter().position(|&o| o == f) {
                        ends[k] += 1;
                    }
                }
            }
        }
        let widths: Vec<Option<f64>> = js
            .iter()
            .map(|&j| sc.edges[j].label.as_ref().map(|l| l.2))
            .collect();
        let others: Vec<Rect> = rects
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != i)
            .map(|(_, &r)| r)
            .collect();
        let faces = loop_faces(&n, &outs, &ends, &widths, &others);
        for (f, &out) in outs.iter().enumerate() {
            let mut on: Vec<usize> = (0..js.len()).filter(|&k| faces[k] == f).collect();
            // Innermost first: the loops without cardinalities, then
            // those with, each in source order.
            on.sort_by_key(|&k| carded.get(js[k]).copied().unwrap_or(false));
            let labels: Vec<Option<f64>> = on.iter().map(|&k| widths[k]).collect();
            for (&k, (curve, spot)) in on.iter().zip(face_loops(&n, out, &labels)) {
                let e = &mut sc.edges[js[k]];
                if js.len() > 1 || f != 0 {
                    e.bezier = curve;
                    e.waypoints.clear();
                }
                if let (Some((_, m, _)), Some(spot)) = (&mut e.label, spot) {
                    *m = spot;
                }
            }
        }
    }
}

/// How far a self-loop's region keeps from another box.
const LOOP_CLEAR: f64 = 4.0;
/// Air between two loop labels in a row past their loops: more than
/// the label stage keeps between labels ([`LABEL_GAP`] plus its overlap
/// tolerance), so it leaves the row as drawn. Closer, it would take
/// the pair for crowded and could move one onto the loops, where an
/// inner loop's apex leaves a spot clear of boxes and labels (the
/// stage does not mind strokes there).
const LOOP_LABEL_GAP: f64 = LABEL_GAP + 2.0;

/// A self-loop as [`face_loops`] draws it: its curve, and its label's
/// centre when it has a label.
type SelfLoop = ([(f64, f64); 4], Option<(f64, f64)>);

/// Self-loops nested on the face of `n` with outward normal `out`, one
/// per entry of `labels` (its label's width, if it has one), innermost
/// first, as [`part_self_loops`] draws them: each loop's curve, and its
/// label's centre past the outermost loop.
fn face_loops(n: &SceneNode, out: (f64, f64), labels: &[Option<f64>]) -> Vec<SelfLoop> {
    let t = (-out.1, out.0);
    let face = (n.x + out.0 * n.w / 2.0, n.y + out.1 * n.h / 2.0);
    let at = |along: f64, off: f64| {
        (
            face.0 + t.0 * along + out.0 * off,
            face.1 + t.1 * along + out.1 * off,
        )
    };
    let on_side = labels.len();
    // Each label's room along the face, and where the row of them
    // (centred on the face) puts each one's centre.
    let wide = |w: f64| LABEL_BOX_H * out.0.abs() + w * out.1.abs() + LOOP_LABEL_GAP;
    let row: f64 = labels.iter().flatten().map(|&w| wide(w)).sum();
    let mut next = -row / 2.0;
    labels
        .iter()
        .enumerate()
        .map(|(q, w)| {
            let qf = q as f64;
            let half = LOOP_HALF + qf * LOOP_NEST.0;
            let reach = LOOP_REACH + qf * LOOP_NEST.1;
            let lift = LOOP_LIFT + qf * LOOP_NEST.2;
            // The glyph (at the curve's end) alternates between the
            // ends of loops nested on one side.
            let s = if q % 2 == 0 { 1.0 } else { -1.0 };
            let curve = [
                at(-half * s, 0.0),
                at(-lift * s, reach),
                at(lift * s, reach),
                at(half * s, 0.0),
            ];
            // Past the outermost loop on the side (a cubic's apex
            // reaches 3/4 of its controls' offset), the labels of
            // nested loops side by side along the face.
            let spot = w.map(|w| {
                let outer = LOOP_REACH + (on_side - 1) as f64 * LOOP_NEST.1;
                let chip = (w / 2.0) * out.0.abs() + 11.0 * out.1.abs();
                let along = next + wide(w) / 2.0;
                next += wide(w);
                at(along, 0.75 * outer + 6.0 + chip)
            });
            (curve, spot)
        })
        .collect()
}

/// The face (an index into `outs`, the four outward normals of `n` in
/// order of preference) for each of a class's self-loops, `widths`
/// their labels' widths: alternately the first two faces, unless the
/// loops and labels there would meet a box of `others`, lie over
/// `ends` (other edges' ends per face) or meet the loops of another
/// face. Then each way of sharing the loops among the four faces is
/// tried, round robin as before, the clearest kept; of two as clear,
/// the one that uses the preferred faces and nests least.
fn loop_faces(
    n: &SceneNode,
    outs: &[(f64, f64); 4],
    ends: &[usize; 4],
    widths: &[Option<f64>],
    others: &[Rect],
) -> Vec<usize> {
    let m = widths.len();
    // Round robin over the faces with loops still to take.
    let share = |counts: [usize; 4]| {
        let mut left = counts;
        let mut faces = Vec::with_capacity(m);
        let mut f = 0;
        while faces.len() < m {
            if left[f] > 0 {
                left[f] -= 1;
                faces.push(f);
            }
            f = (f + 1) % 4;
        }
        faces
    };
    // What a sharing costs: (hits, preference).
    let cost = |faces: &[usize]| {
        let mut regions: Vec<Rect> = Vec::new();
        let mut hits = 0;
        let mut pref = 0;
        for (f, &out) in outs.iter().enumerate() {
            let labels: Vec<Option<f64>> = (0..m)
                .filter(|&k| faces[k] == f)
                .map(|k| widths[k])
                .collect();
            let c = labels.len();
            if c == 0 {
                continue;
            }
            pref += 2 * f + 3 * c * (c - 1) / 2;
            hits += ends[f];
            let mut r = (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            );
            let mut grow =
                |p: (f64, f64)| r = (r.0.min(p.0), r.1.min(p.1), r.2.max(p.0), r.3.max(p.1));
            for ((curve, spot), w) in face_loops(n, out, &labels).iter().zip(&labels) {
                for p in sample_cubic(curve, 16) {
                    grow(p);
                }
                if let (Some(m), Some(w)) = (spot, w) {
                    grow((m.0 - w / 2.0 - 1.0, m.1 - 11.0));
                    grow((m.0 + w / 2.0 + 1.0, m.1 + 11.0));
                }
            }
            let r = (
                r.0 - LOOP_CLEAR,
                r.1 - LOOP_CLEAR,
                r.2 + LOOP_CLEAR,
                r.3 + LOOP_CLEAR,
            );
            hits += others.iter().filter(|&&b| rects_meet(r, b)).count();
            hits += regions.iter().filter(|&&b| rects_meet(r, b)).count();
            regions.push(r);
        }
        (hits, pref)
    };
    let default = share([m.div_ceil(2), m / 2, 0, 0]);
    let mut best = (cost(&default), default);
    // Up to eight loops (165 ways to share them); more keep the default.
    if best.0 .0 == 0 || m > 8 {
        return best.1;
    }
    for a in 0..=m {
        for b in 0..=m - a {
            for c in 0..=m - a - b {
                let faces = share([a, b, c, m - a - b - c]);
                let k = cost(&faces);
                if k < best.0 {
                    best = (k, faces);
                }
            }
        }
    }
    best.1
}

/// The same edge drawn the other way round: ends, curve and route
/// reversed; the label stays where it is.
fn reverse_edge(e: &mut SceneEdge) {
    std::mem::swap(&mut e.from, &mut e.to);
    e.bezier.reverse();
    e.waypoints.reverse();
}

/// Corner radius of the orthogonal class routes (mermaid's class
/// edges round their corners by about this much).
const ORTHO_R: f64 = 7.0;
/// Shortest straight run an orthogonal route leaves or enters a face
/// with — room for the longest end glyph (the 18 px diamond) plus air.
const ORTHO_STUB: f64 = 22.0;
/// Two horizontal (or vertical) runs closer than this that overlap
/// along their length read as one line — two edges merged into one
/// read as the wrong connection — so a route that would lie on a run
/// already placed is refused like one through a box: the edge takes
/// another lane, or keeps its curve.
const ORTHO_SEP: f64 = 8.0;
/// Parallel runs closer than this (but [`ORTHO_SEP`] or more apart)
/// still read as one double line over a long stretch: a route keeps
/// this far from them when the gap has room, at [`ORTHO_CRAMPED`].
const ORTHO_ROOM: f64 = 14.0;
/// Cost of an orthogonal stretch running closer than [`ORTHO_ROOM`]
/// alongside another route's run (less than a crossing).
const ORTHO_CRAMPED: usize = 3;
/// Cost of an orthogonal stretch running under another edge's label
/// (the chip hides it there).
const ORTHO_UNDER_LABEL: usize = 60;
/// Cost of an orthogonal route crossing another.
const ORTHO_CROSSING: usize = 10;
/// Step between the spots tried for a rerouted edge's label.
const LABEL_SPOT_STEP: f64 = 6.0;
/// How far from its route's middle a rerouted edge's label is sought.
/// A node dragged far off leaves routes thousands of px long, often
/// beside each other all the way (a fan's lanes), so no spot is clear:
/// searching the whole length would test thousands of spots against
/// every run, box and label, for each route tried. Laid-out routes are
/// far shorter than twice this.
const LABEL_SPOT_REACH: f64 = 2000.0;
/// An edge end alone on its face keeps at least this far from the
/// face's corners when it is moved.
const ORTHO_INSET: f64 = 24.0;

/// Redraw every free edge as mermaid draws class edges: an orthogonal
/// route with rounded corners, delivered as router-style waypoints
/// (`router::smooth`, so a host's spline reproduces it). Between
/// facing faces an edge goes straight when an end alone on its face
/// can slide into line with the other, else it jogs at mid gap (Z); a
/// long edge runs straight down each channel its layout chain holds
/// and jogs between them; one whose way between is blocked by boxes (a
/// back-edge bow) runs a lane at the bow's outer extent; an edge
/// between perpendicular faces turns once (L). Jogs step aside from
/// runs already placed, then restack in nesting order
/// ([`nest_jogs`]); a route that would cross a box, or lie on another
/// route, keeps its curve. Edges the router routed
/// (axis-aligned waypoints) are kept as routed, unless they lie along
/// another or enter their glyph on a short leg; a label moves onto its
/// new line. `decos[j]` is what edge `j` holds at its start and its end
/// (missing = nothing), so ends sharing a face keep their glyphs and
/// cardinalities apart. `mode` picks the work budgets
/// ([`ORTHO_NUDGE_WORK`], [`UNCROSS_WORK`]).
fn orthogonalize(sc: &mut Scene, decos: &[[EndDeco; 2]], mode: Mode) {
    let (nudges, swaps) = match mode {
        Mode::Static => (ORTHO_NUDGE_WORK.0, UNCROSS_WORK.0),
        Mode::Routed => (ORTHO_NUDGE_WORK.1, UNCROSS_WORK.1),
    };
    let pass = |edges: &mut [SceneEdge], room: bool, soft: bool| {
        ortho_pass(&sc.nodes, edges, decos, room, soft, nudges)
    };
    let laid = sc.edges.clone();
    let (mut score, mut cramped) = pass(&mut sc.edges, true, false);
    let mut soft = false;
    if score.0 > 0 {
        // Long and routed ends held where they are can leave another
        // edge no way out: let them spread too when that routes more.
        let mut alt = laid.clone();
        let (alt_score, alt_cramped) = pass(&mut alt, true, true);
        if alt_score < score {
            (sc.edges, score, cramped, soft) = (alt, alt_score, alt_cramped, true);
        }
    }
    if cramped && score > (0, 0) {
        // Roomy lanes can use up a crowded gap: pack it tight instead
        // when that leaves fewer edges on their curves, or as many
        // with fewer crossings and covered labels.
        let mut tight = laid;
        if pass(&mut tight, false, soft).0 < score {
            sc.edges = tight;
        }
    }
    nest_jogs(&sc.nodes, &mut sc.edges);
    uncross_side_ends(&sc.nodes, &mut sc.edges, decos, swaps);
}

/// Swap the hub ends of two routes into one class that cross where one
/// enters a side face of the hub (one turn) and the other the face the
/// two come toward (a jog). The router sends the legs of a fan too wide
/// for the hub's face to its side faces, but not always the outermost
/// ones, so a side leg can cross the outer leg's jog (`v1_fan10_labels`:
/// Square's leg across Circle's). Swapped, the outer route takes the
/// side face and the inner one its slot on the near face. Only ends
/// holding the same glyph and cardinality swap, and a swap stays only
/// when the whole drawing then costs less ([`drawing_cost`]). At most
/// `work` swaps are tried ([`UNCROSS_WORK`]).
fn uncross_side_ends(
    nodes: &[SceneNode],
    edges: &mut [SceneEdge],
    decos: &[[EndDeco; 2]],
    mut work: usize,
) {
    let idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let rects: Vec<Rect> = nodes.iter().map(node_rect).collect();
    let deco = |j: usize, start: bool| {
        decos
            .get(j)
            .map(|d| d[usize::from(!start)])
            .map(|d| (d.half, d.card))
    };
    let dot = |a: (f64, f64), b: (f64, f64)| a.0 * b.0 + a.1 * b.1;
    // Each edge's (from, to) boxes, and its corners when orthogonal.
    let boxes: Vec<Option<(usize, usize)>> = edges
        .iter()
        .map(|e| Some((*idx.get(e.from.as_str())?, *idx.get(e.to.as_str())?)))
        .collect();
    let mut corners: Vec<Option<Vec<(f64, f64)>>> = edges.iter().map(ortho_corners).collect();
    // The drawing's cost as it stands, once a swap is tried.
    let mut now: Option<usize> = None;
    for s in 0..edges.len() {
        for z in 0..edges.len() {
            for start in [false, true] {
                if work == 0 {
                    return;
                }
                // (hub, far box) of an edge: the hub at its start or end.
                let ends_of = |j: usize| boxes[j].map(|(f, t)| if start { (f, t) } else { (t, f) });
                let (Some((h, fs)), Some((hz_box, fz))) = (ends_of(s), ends_of(z)) else {
                    continue;
                };
                if s == z || h != hz_box || fs == h || fz == h || deco(s, start) != deco(z, start) {
                    continue;
                }
                let (Some(s_route), Some(z_route)) = (&corners[s], &corners[z]) else {
                    continue;
                };
                let (es, ez) = (&edges[s], &edges[z]);
                // (hub end, far end) of each, as the corners run from
                // `from` to `to`.
                let ends = |c: &[(f64, f64)]| {
                    let (a, b) = (c[0], c[c.len() - 1]);
                    if start {
                        (a, b)
                    } else {
                        (b, a)
                    }
                };
                let ((hs, os), (hz, oz)) = (ends(s_route), ends(z_route));
                let (ns, nz) = (face_normal(&nodes[h], hs), face_normal(&nodes[h], hz));
                let (nos, noz) = (face_normal(&nodes[fs], os), face_normal(&nodes[fz], oz));
                // `s` turns once into a side face, `z` jogs into the
                // near face, and the two cross.
                if dot(ns, nz).abs() > 0.5
                    || dot(nos, nz) > -0.5
                    || dot(noz, nz) > -0.5
                    || !routes_cross(s_route, z_route)
                {
                    continue;
                }
                work -= 1;
                let mut runs = Runs::default();
                let mut labels = Vec::new();
                for (k, e) in edges.iter().enumerate() {
                    if k != s && k != z {
                        for q in straight_runs(e) {
                            runs.push(q);
                        }
                        labels.extend(label_chip(e));
                    }
                }
                let cramped = Cell::new(false);
                let ctx = |runs, ia, ib, j| Ortho {
                    rects: &rects,
                    labels: &labels,
                    ia,
                    ib,
                    j,
                    runs,
                    room: false,
                    cramped: &cramped,
                };
                // Each takes the other's route with its far leg slid over
                // to its own far end — `s` keeps `z`'s jog in the fan's
                // nesting — when that leaves its face square and runs
                // clear; else a fresh route: `s` a jog into `z`'s slot,
                // `z` one turn into `s`'s side face.
                let (sa, sb, sna, snb) = if start {
                    (hz, os, nz, nos)
                } else {
                    (os, hz, nos, nz)
                };
                let (za, zb, zna, znb) = if start {
                    (hs, oz, ns, noz)
                } else {
                    (oz, hs, noz, ns)
                };
                let slid = |c: &[(f64, f64)], far: (f64, f64), n: (f64, f64)| {
                    if c.len() < 3 {
                        return None;
                    }
                    let mut c = c.to_vec();
                    slide(&mut c, !start, far);
                    let knee = if start { c[c.len() - 2] } else { c[1] };
                    (dot((knee.0 - far.0, knee.1 - far.1), n) >= ORTHO_STUB).then_some(c)
                };
                let (si, zi) = if start {
                    ((h, fs), (h, fz))
                } else {
                    ((fs, h), (fz, h))
                };
                let mut straight = es.clone();
                straight.waypoints.clear();
                straight.bezier = [sa, sa, sb, sb];
                let s_ctx = ctx(&runs, si.0, si.1, s);
                let Some(new_s) = slid(z_route, os, nos)
                    .filter(|c| s_ctx.cost(c).is_some())
                    .or_else(|| s_ctx.opposite(&straight, sa, sb, sna, (false, false)))
                else {
                    continue;
                };
                let spot_s = es
                    .label
                    .as_ref()
                    .map(|l| s_ctx.label_spot(&new_s, l.2, l.1));
                let mut runs_z = Runs::default();
                for q in runs
                    .all()
                    .copied()
                    .chain(new_s.windows(2).map(|w| Run::of(w[0], w[1])))
                {
                    runs_z.push(q);
                }
                let z_ctx = ctx(&runs_z, zi.0, zi.1, z);
                let Some(new_z) = slid(s_route, oz, noz)
                    .filter(|c| z_ctx.cost(c).is_some())
                    .or_else(|| z_ctx.perpendicular(za, zb, zna, znb))
                else {
                    continue;
                };
                let spot_z = ez
                    .label
                    .as_ref()
                    .map(|l| z_ctx.label_spot(&new_z, l.2, l.1));
                let before = *now.get_or_insert_with(|| drawing_cost(&rects, &idx, edges));
                let kept = (edges[s].clone(), edges[z].clone());
                draw_route(&mut edges[s], &new_s, (sna, snb), spot_s);
                draw_route(&mut edges[z], &new_z, (zna, znb), spot_z);
                let after = drawing_cost(&rects, &idx, edges);
                if after < before {
                    now = Some(after);
                    corners[s] = ortho_corners(&edges[s]);
                    corners[z] = ortho_corners(&edges[z]);
                } else {
                    (edges[s], edges[z]) = kept;
                }
            }
        }
    }
}

/// How many swaps [`uncross_side_ends`] tries, for an automatic layout
/// and for a drag: the widest fans laid out need about a dozen, and a
/// drag that tangles a big fan would otherwise try every crossing pair,
/// each costed over the whole drawing.
const UNCROSS_WORK: (usize, usize) = (48, 12);

/// Whether two axis-aligned routes (their corners) cross: a stretch of
/// one passes through a stretch of the other, clear of both ends.
fn routes_cross(p: &[(f64, f64)], q: &[(f64, f64)]) -> bool {
    p.windows(2).any(|s| {
        let a = Run::of(s[0], s[1]);
        q.windows(2).any(|t| {
            let b = Run::of(t[0], t[1]);
            a.vertical != b.vertical
                && a.l + 1.0 < b.c
                && b.c < a.h - 1.0
                && b.l + 1.0 < a.c
                && a.c < b.h - 1.0
        })
    })
}

/// How far [`ortho_pass`] nudges an end along its face, nearest first,
/// when the end as spread leaves its edge no orthogonal route (after
/// the places just clear of each lane beside it, within the farthest
/// of these plus [`ORTHO_SEP`]).
const ORTHO_NUDGES: [f64; 8] = [4.0, -4.0, 8.0, -8.0, 12.0, -12.0, 16.0, -16.0];

/// How many routes the nudged ends of one [`ortho_pass`] may try, for
/// an automatic layout and for a drag: a layout needs a few dozen at
/// most, while a drag that leaves many edges without a square exit
/// would otherwise try every place for each of them, each try a full
/// route search.
const ORTHO_NUDGE_WORK: (usize, usize) = (64, 16);

/// Whether end `end` (edge, at its start) may move to `p` on face
/// `(i, key)` of box `rect` (`faces`, `moved` and `decos` as
/// [`ortho_pass`] holds them): it keeps its place in the order along
/// the face, its glyph stays on the face, and it keeps from its
/// neighbours (where spread put them) the room [`spread`] keeps
/// between two ends, their glyphs and cardinalities apart.
fn end_fits(
    (faces, moved, rect, decos): (&Faces, &Moved, &Rect, &[[EndDeco; 2]]),
    (i, key): (usize, (i8, i8)),
    end: (usize, bool),
    p: (f64, f64),
) -> bool {
    let horizontal = key.0 == 0;
    let along = |q: (f64, f64)| if horizontal { q.0 } else { q.1 };
    let (lo, hi) = if horizontal {
        (rect.0, rect.2)
    } else {
        (rect.1, rect.3)
    };
    let mut ends: Vec<((usize, bool), f64)> = faces.get(&(i, key)).map_or(Vec::new(), |f| {
        f.iter()
            .map(|&(j, start, at, _)| {
                ((j, start), moved.get(&(j, start)).map_or(at, |&q| along(q)))
            })
            .collect()
    });
    ends.sort_by(|a, b| a.1.total_cmp(&b.1));
    let deco = |(j, start): (usize, bool)| {
        decos
            .get(j)
            .map_or(EndDeco::default(), |d| d[usize::from(!start)])
    };
    let t = along(p);
    let half = deco(end).half;
    let Some(k) = ends.iter().position(|e| e.0 == end) else {
        return t - lo >= half + 2.0 && hi - t >= half + 2.0;
    };
    let t_up = -f64::from(key.1) + f64::from(key.0) > 0.0;
    let decs: Vec<EndDeco> = ends.iter().map(|e| deco(e.0)).collect();
    let rooms = face_rooms(&decs, horizontal, t_up);
    let need = |a: f64, b: f64| (a + b + GLYPH_GAP).max(2.0 * ORTHO_SEP);
    let after_prev = k.checked_sub(1).map_or(t - lo >= half + 2.0, |b| {
        t - ends[b].1 >= need(rooms[b].hi, rooms[k].lo)
    });
    let before_next = ends.get(k + 1).map_or(hi - t >= half + 2.0, |e| {
        e.1 - t >= need(rooms[k].hi, rooms[k + 1].lo)
    });
    after_prev && before_next
}

/// One jog of an orthogonal route — the leg across the flow between a
/// leg coming down from above and one going on below — in the frame
/// [`Ortho::opposite`] works in (`flip` swaps x / y, so the jog runs
/// along x): route `j`'s corners `k` and `k + 1`, at `y`; the leg above
/// at `u` reaches up to `top`, the one below at `l` down to `bot`.
/// A Z route across one gap has one jog, a lane round boxes two.
struct Zig {
    j: usize,
    k: usize,
    flip: bool,
    top: f64,
    bot: f64,
    u: f64,
    l: f64,
    y: f64,
    /// Least and most `y` leaves the legs: a route's end leg at least
    /// [`ORTHO_STUB`] long, any other [`ORTHO_SEP`].
    lo: f64,
    hi: f64,
    /// Where the jog would best sit: mid gap for a Z, else where it is.
    mid: f64,
}

impl Zig {
    /// Whether `x` lies inside this jog's span (clear of its ends).
    fn spans(&self, x: f64) -> bool {
        self.u.min(self.l) + 1.0 < x && x < self.u.max(self.l) - 1.0
    }

    /// Whether `o` must jog farther down the gap than this one for the
    /// two not to cross: `o`'s lower leg drops inside this jog's span,
    /// or this one's upper leg inside `o`'s.
    fn over(&self, o: &Zig) -> bool {
        self.spans(o.l) || o.spans(self.u)
    }

    /// How many times this jog crosses `o`'s legs.
    fn crosses(&self, o: &Zig) -> usize {
        usize::from(self.spans(o.u) && o.top + 1.0 < self.y && self.y < o.y - 1.0)
            + usize::from(self.spans(o.l) && o.y + 1.0 < self.y && self.y < o.bot - 1.0)
    }

    /// Whether the two routes cross the same gap (and so may cross).
    fn shares(&self, o: &Zig) -> bool {
        self.flip == o.flip && self.j != o.j && self.lo < o.hi && o.lo < self.hi
    }
}

/// Stack the jogs that share a gap in nesting order, as channel routing
/// does. Routes are drawn one at a time, the shortest first and each
/// nearest mid gap, so an inner route of a fan takes the middle and an
/// outer one — which must jog nearer the face the fan converges on —
/// finds no lane left on that side and crosses the inner one twice
/// (fan-ins of six subclasses, or of labelled relations whose chips
/// push the inner jogs aside). Here each group of Z routes tied by such
/// an order, with two of them crossing, is drawn again in that order
/// ([`nest_group`]). The group keeps the new routes only when the whole
/// drawing then costs less (crossings, lines merged, label chips
/// struck) than before.
fn nest_jogs(nodes: &[SceneNode], edges: &mut [SceneEdge]) {
    let idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut zigs: Vec<Zig> = Vec::new();
    for (j, e) in edges.iter().enumerate() {
        if e.from == e.to || e.meta.curve.is_some() || e.waypoints.is_empty() {
            continue;
        }
        let c = corners_of(&e.waypoints);
        let n = c.len();
        if n < 4 || !idx.contains_key(e.from.as_str()) || !idx.contains_key(e.to.as_str()) {
            continue;
        }
        // The frame in which the route's first leg runs along y.
        let flip = (c[0].1 - c[1].1).abs() < 0.5;
        let q: Vec<(f64, f64)> = c.iter().map(|&p| f_pt(p, flip)).collect();
        if !q.windows(2).enumerate().all(|(i, w)| {
            let along = if i % 2 == 0 {
                (w[0].0, w[1].0)
            } else {
                (w[0].1, w[1].1)
            };
            (along.0 - along.1).abs() < 0.5
        }) {
            continue;
        }
        for k in (1..n - 2).step_by(2) {
            let y = q[k].1;
            let ((top, u, top_end), (bot, l, bot_end)) = if q[k - 1].1 < q[k + 2].1 {
                (
                    (q[k - 1].1, q[k].0, k == 1),
                    (q[k + 2].1, q[k + 1].0, k + 2 == n - 1),
                )
            } else {
                (
                    (q[k + 2].1, q[k + 1].0, k + 2 == n - 1),
                    (q[k - 1].1, q[k].0, k == 1),
                )
            };
            if !(top < y && y < bot) {
                continue; // both legs on one side: the turn of a lane
            }
            let leg = |end: bool| if end { ORTHO_STUB } else { ORTHO_SEP };
            zigs.push(Zig {
                j,
                k,
                flip,
                top,
                bot,
                u,
                l,
                y,
                lo: top + leg(top_end),
                hi: bot - leg(bot_end),
                mid: if n == 4 { (top + bot) / 2.0 } else { y },
            });
        }
    }
    let n = zigs.len();
    if n < 2 {
        return;
    }
    // above[a][b]: route a must jog above route b (not both ways: then
    // they cross once whichever way, and no order helps).
    let above: Vec<Vec<bool>> = (0..n)
        .map(|a| {
            (0..n)
                .map(|b| {
                    let (p, q) = (&zigs[a], &zigs[b]);
                    a != b && p.shares(q) && p.over(q) && !q.over(p)
                })
                .collect()
        })
        .collect();
    fn root(g: &mut [usize], mut i: usize) -> usize {
        while g[i] != i {
            g[i] = g[g[i]];
            i = g[i];
        }
        i
    }
    let mut group: Vec<usize> = (0..n).collect();
    let mut tangled = vec![false; n];
    for a in 0..n {
        for b in a + 1..n {
            if above[a][b] || above[b][a] {
                let (ra, rb) = (root(&mut group, a), root(&mut group, b));
                group[ra] = rb;
                if zigs[a].crosses(&zigs[b]) + zigs[b].crosses(&zigs[a]) > 0 {
                    tangled[a] = true;
                }
            }
        }
    }
    let rects: Vec<Rect> = nodes.iter().map(node_rect).collect();
    let mut done = vec![false; n];
    for start in 0..n {
        let r = root(&mut group, start);
        if done[r] {
            continue;
        }
        done[r] = true;
        let members: Vec<usize> = (0..n).filter(|&k| root(&mut group, k) == r).collect();
        // A group holding two jogs of one route is left as it is (a
        // route is drawn again whole, one jog at a time).
        let mut routes: Vec<usize> = members.iter().map(|&k| zigs[k].j).collect();
        routes.sort_unstable();
        routes.dedup();
        if !members.iter().any(|&k| tangled[k]) || routes.len() < members.len() {
            continue;
        }
        // Topological order of `above`, the jog highest now first of
        // those free to go.
        let mut order: Vec<usize> = Vec::with_capacity(members.len());
        let mut left = members.clone();
        while !left.is_empty() {
            let free = left
                .iter()
                .copied()
                .filter(|&b| !left.iter().any(|&a| above[a][b]))
                .min_by(|&a, &b| zigs[a].y.total_cmp(&zigs[b].y));
            let Some(next) = free else { break };
            order.push(next);
            left.retain(|&k| k != next);
        }
        if !left.is_empty() {
            continue; // a cycle: no order clears every pair
        }
        let g = Group {
            zigs: &zigs,
            above: &above,
            order: &order,
        };
        let before = drawing_cost(&rects, &idx, edges);
        let mut best: Option<(usize, Vec<SceneEdge>)> = None;
        for (respect, step, pack) in [
            (true, ORTHO_ROOM, false),
            (true, ORTHO_SEP, false),
            (true, ORTHO_SEP, true),
            (false, ORTHO_ROOM, false),
            (false, ORTHO_SEP, false),
            (false, ORTHO_SEP, true),
        ] {
            let Some(trial) = nest_group(nodes, &rects, &idx, edges, &g, (respect, step, pack))
            else {
                continue;
            };
            let c = drawing_cost(&rects, &idx, &trial);
            if c < best.as_ref().map_or(before, |b| b.0) {
                best = Some((c, trial));
            }
        }
        if let Some((_, trial)) = best {
            edges.clone_from_slice(&trial);
        }
    }
}

/// A group of tied jogs (one per route) for [`nest_group`]: `order`
/// (indexes into `zigs`) lists them so that each comes after every jog
/// `above[a][b]` says must run above it.
struct Group<'a> {
    zigs: &'a [Zig],
    above: &'a [Vec<bool>],
    order: &'a [usize],
}

/// `edges` with the jogs of group `g` drawn again in its order: each
/// the nearest its best place it can (or, with `pack`, the highest), `step`
/// or more below the jogs it must clear and leaving `step` above each
/// place a jog still to come below it can take, its label moved onto
/// its new line. With `respect`, the
/// jogs keep off the other edges' labels; without, they may run through
/// them, and each label struck is moved along its own line to where it
/// is clear (a chip on a straight edge beside a fan's inner leg, which
/// no nesting clears). `None` when a jog finds no place.
fn nest_group(
    nodes: &[SceneNode],
    rects: &[Rect],
    idx: &HashMap<&str, usize>,
    edges: &[SceneEdge],
    g: &Group,
    (respect, step, pack): (bool, f64, bool),
) -> Option<Vec<SceneEdge>> {
    let ours: Vec<usize> = g.order.iter().map(|&a| g.zigs[a].j).collect();
    // The lowest each jog may go and still leave the jogs below it room.
    let mut cap: HashMap<usize, f64> = HashMap::new();
    for &a in g.order.iter().rev() {
        let below = cap
            .iter()
            .filter(|&(&b, _)| g.above[a][b])
            .map(|(_, &c)| c - step)
            .fold(g.zigs[a].hi, f64::min);
        cap.insert(a, below);
    }
    let mut runs = Runs::default();
    let mut labels: Vec<Rect> = Vec::new();
    for (k, e) in edges.iter().enumerate() {
        if !ours.contains(&k) {
            for q in straight_runs(e) {
                runs.push(q);
            }
            if respect {
                labels.extend(label_chip(e));
            }
        }
    }
    let cramped = Cell::new(false);
    let mut trial: Vec<SceneEdge> = edges.to_vec();
    // The jog each route of the group takes, as placed.
    let mut jog: HashMap<usize, f64> = HashMap::new();
    for &a in g.order {
        let z = &g.zigs[a];
        let e = &edges[z.j];
        let (ia, ib) = (idx[e.from.as_str()], idx[e.to.as_str()]);
        let mut lo = z.lo;
        for (&p, &y) in &jog {
            if g.above[p][a] {
                lo = lo.max(y + step);
            }
        }
        let hi = cap[&a];
        if lo > hi + 1e-9 {
            return None;
        }
        // Within that tolerance `lo` may still pass `hi` by a rounding
        // error (the two come from sums taken in different orders), and
        // `f64::clamp` panics on such a range: the jog then has the one
        // place `lo`.
        let hi = hi.max(lo);
        let ctx = Ortho {
            rects,
            labels: &labels,
            ia,
            ib,
            j: z.j,
            runs: &runs,
            room: true,
            cramped: &cramped,
        };
        let q: Vec<(f64, f64)> = corners_of(&e.waypoints)
            .into_iter()
            .map(|p| f_pt(p, z.flip))
            .collect();
        // The route as an earlier group left it: a lane route's other
        // jog may sit in that group, and its corners must still hold
        // this jog's.
        if z.k + 1 >= q.len() {
            return None;
        }
        let span = (z.u.min(z.l), z.u.max(z.l));
        let want = if pack { lo } else { z.mid.clamp(lo, hi) };
        let mut ys = ctx.jog_places(want, (lo, hi), span, z.flip);
        ys.extend([lo, hi]);
        let back = |y: f64| {
            let mut q = q.clone();
            (q[z.k].1, q[z.k + 1].1) = (y, y);
            q.into_iter().map(|p| f_pt(p, z.flip)).collect::<Vec<_>>()
        };
        let (_, pts) = ctx.best(ys.into_iter().map(back))?;
        let spot = e.label.as_ref().map(|l| ctx.label_spot(&pts, l.2, l.1));
        for w in pts.windows(2) {
            runs.push(Run::of(w[0], w[1]));
        }
        let n = (
            face_normal(&nodes[ia], pts[0]),
            face_normal(&nodes[ib], pts[pts.len() - 1]),
        );
        draw_route(&mut trial[z.j], &pts, n, spot);
        labels.extend(label_chip(&trial[z.j]));
        jog.insert(a, f_pt(pts[z.k], z.flip).1);
    }
    // Move each label a jog drawn after it strikes along its own line:
    // the group's own (a jog placed later can pass under an earlier
    // one's label), and without `respect` the others' too.
    let others_too = (0..trial.len()).filter(|k| !respect && !ours.contains(k));
    for k in ours.iter().copied().chain(others_too) {
        let (Some(chip), Some(pts)) = (label_chip(&trial[k]), ortho_corners(&trial[k])) else {
            continue;
        };
        let struck = ours.iter().any(|&j| {
            j != k
                && straight_runs(&trial[j]).iter().any(|q| {
                    let (a, b) = q.ends();
                    rect_hits_segment(chip, a, b)
                })
        });
        if !struck {
            continue;
        }
        let mut others = Runs::default();
        let mut chips = Vec::new();
        for (i, o) in trial.iter().enumerate() {
            if i != k {
                for q in straight_runs(o) {
                    others.push(q);
                }
                chips.extend(label_chip(o));
            }
        }
        let e = &trial[k];
        let ctx = Ortho {
            rects,
            labels: &chips,
            ia: idx.get(e.from.as_str()).copied().unwrap_or(usize::MAX),
            ib: idx.get(e.to.as_str()).copied().unwrap_or(usize::MAX),
            j: k,
            runs: &others,
            room: false,
            cramped: &cramped,
        };
        let Some((_, m, w)) = &e.label else { continue };
        let spot = ctx.label_spot(&pts, *w, *m);
        let line = edge_polyline(e);
        if let Some((_, m, _)) = &mut trial[k].label {
            *m = nearest_on(&line, spot);
        }
    }
    Some(trial)
}

/// What the whole drawing costs ([`Ortho::cost`] of every orthogonal
/// line among all the others, their labels included; a line through a
/// box or along another counts as four labels struck), to compare two
/// versions of it.
fn drawing_cost(rects: &[Rect], idx: &HashMap<&str, usize>, edges: &[SceneEdge]) -> usize {
    // Every line's runs, owned by their edge so its own never count.
    let mut runs = Runs::default();
    for (j, e) in edges.iter().enumerate() {
        for mut q in straight_runs(e) {
            q.owner = j;
            runs.push(q);
        }
    }
    let chips: Vec<Option<Rect>> = edges.iter().map(label_chip).collect();
    let cramped = Cell::new(false);
    let mut total = 0;
    for (j, e) in edges.iter().enumerate() {
        let Some(pts) = ortho_corners(e) else {
            continue;
        };
        let labels: Vec<Rect> = chips
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != j)
            .filter_map(|(_, c)| *c)
            .collect();
        let ctx = Ortho {
            rects,
            labels: &labels,
            ia: idx.get(e.from.as_str()).copied().unwrap_or(usize::MAX),
            ib: idx.get(e.to.as_str()).copied().unwrap_or(usize::MAX),
            j,
            runs: &runs,
            room: false,
            cramped: &cramped,
        };
        total += ctx.cost(&pts).unwrap_or(4 * ORTHO_UNDER_LABEL);
    }
    total
}

/// The corners of `e` as drawn when it is orthogonal: its route's, or
/// the two ends of a straight axis-aligned cubic; `None` for a curve
/// or a self-loop.
fn ortho_corners(e: &SceneEdge) -> Option<Vec<(f64, f64)>> {
    if e.from == e.to || e.meta.curve.is_some() {
        return None;
    }
    let pts = if e.waypoints.is_empty() {
        let q = e.bezier;
        let line = [q[1], q[2]].iter().all(|c| {
            ((q[3].0 - q[0].0) * (c.1 - q[0].1) - (q[3].1 - q[0].1) * (c.0 - q[0].0)).abs() < 1e-6
        });
        if !line {
            return None;
        }
        vec![q[0], q[3]]
    } else {
        corners_of(&e.waypoints)
    };
    pts.windows(2)
        .all(|w| (w[0].0 - w[1].0).abs() < 0.5 || (w[0].1 - w[1].1).abs() < 0.5)
        .then_some(pts)
}

/// The box a label chip covers, as routes keep off it.
fn label_chip(e: &SceneEdge) -> Option<Rect> {
    e.label.as_ref().map(|(_, m, w)| {
        (
            m.0 - w / 2.0 - 1.0,
            m.1 - 11.0,
            m.0 + w / 2.0 + 1.0,
            m.1 + 11.0,
        )
    })
}

/// A node's box.
fn node_rect(n: &SceneNode) -> Rect {
    (
        n.x - n.w / 2.0,
        n.y - n.h / 2.0,
        n.x + n.w / 2.0,
        n.y + n.h / 2.0,
    )
}

/// `p` with x / y swapped when `flip`.
fn f_pt(p: (f64, f64), flip: bool) -> (f64, f64) {
    if flip {
        (p.1, p.0)
    } else {
        p
    }
}

/// One [`orthogonalize`] pass over `edges` among `nodes` (`decos` as
/// there); `room` = keep [`ORTHO_ROOM`] between parallel runs where a
/// gap allows; `soft` = [`Hold::Soft`] ends spread like free ones;
/// `nudges` = how many routes nudged ends may try
/// ([`ORTHO_NUDGE_WORK`]). Returns (how many free edges kept their
/// curve, what the routes drawn cost apart from cramping) and whether
/// any candidate was cramped (else a tight pass would choose the same).
fn ortho_pass(
    nodes: &[SceneNode],
    edges: &mut [SceneEdge],
    decos: &[[EndDeco; 2]],
    room: bool,
    soft: bool,
    mut nudges: usize,
) -> ((usize, usize), bool) {
    let rects: Vec<Rect> = nodes
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
    let idx: HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();
    let face_key = |n: (f64, f64)| (n.0.round() as i8, n.1.round() as i8);
    let ends_of =
        |e: &SceneEdge| -> Option<(usize, usize)> { Some((*idx.get(&e.from)?, *idx.get(&e.to)?)) };
    // Routes the router already made (axis-aligned waypoints) stay as
    // they are; the others step aside from their runs.
    let routed = |e: &SceneEdge| {
        e.waypoints.len() >= 2
            && e.waypoints
                .windows(2)
                .all(|w| (w[0].0 - w[1].0).abs() < 0.5 || (w[0].1 - w[1].1).abs() < 0.5)
    };
    let free = |e: &SceneEdge| e.from != e.to && e.meta.curve.is_none() && !routed(e);
    // The edge ends on each face of each box: (edge, at start, place
    // along the face, free to move). A routed edge's ends stay with its
    // route; a long edge's move too, its chain jogging into its first
    // channel.
    let mut faces: BTreeMap<(usize, (i8, i8)), Vec<FaceEnd>> = BTreeMap::new();
    for (j, e) in edges.iter().enumerate() {
        let Some((ia, ib)) = ends_of(e) else { continue };
        let (a, b) = edge_ends(e);
        let corners = if routed(e) {
            corners_of(&e.waypoints)
        } else {
            Vec::new()
        };
        for (i, p, start) in [(ia, a, true), (ib, b, false)] {
            let n = face_normal(&nodes[i], p);
            let along = if n.0 == 0.0 { p.0 } else { p.1 };
            // A routed end moves when its final leg can slide with it.
            let hold = if free(e) && e.waypoints.is_empty() {
                Hold::Free
            } else if free(e) || slides(&corners, start, n) {
                Hold::Soft
            } else {
                Hold::Fixed
            };
            faces
                .entry((i, face_key(n)))
                .or_default()
                .push((j, start, along, hold));
        }
    }
    // Free ends sharing a face spread evenly over the stretch between
    // its fixed ends (or corners), in the order the port stage gave
    // them: mermaid's class edges leave a box well inside its
    // corners. An end alone on its face slides later, to line up with
    // its other end.
    let mut moved = spread_faces(&faces, &rects, decos, soft);
    // A routed edge whose end moved slides its final leg along with it
    // (the leg before stretches to meet it) — unless a leg would then
    // cross a box or run along another routed edge: such an edge stays
    // as routed, and its faces spread again around it.
    let routed_runs: Vec<(usize, Run)> = edges
        .iter()
        .enumerate()
        .filter(|(_, e)| routed(e))
        .flat_map(|(j, e)| {
            corners_of(&e.waypoints)
                .windows(2)
                .map(|w| (j, Run::of(w[0], w[1])))
                .collect::<Vec<_>>()
        })
        .collect();
    let slid = |j: usize, e: &SceneEdge, moved: &HashMap<(usize, bool), (f64, f64)>| {
        let (ma, mb) = (moved.get(&(j, true)), moved.get(&(j, false)));
        if !routed(e) || (ma.is_none() && mb.is_none()) {
            return None;
        }
        let (ia, ib) = ends_of(e)?;
        let mut c = corners_of(&e.waypoints);
        for (at_start, to) in [(true, ma), (false, mb)] {
            if let Some(&p) = to {
                slide(&mut c, at_start, p);
            }
        }
        let n = c.len();
        let clear = c.windows(2).enumerate().all(|(k, s)| {
            let off_boxes = rects.iter().enumerate().all(|(b, &r)| {
                let r = if b == ia || b == ib {
                    (r.0 + 0.5, r.1 + 0.5, r.2 - 0.5, r.3 - 0.5)
                } else {
                    r
                };
                !rect_hits_segment(r, s[0], s[1])
            });
            // The legs a slide moved: the two at each moved end.
            let moved_leg = (ma.is_some() && k < 2) || (mb.is_some() && k + 3 > n);
            let r = Run::of(s[0], s[1]);
            off_boxes
                && (!moved_leg
                    || routed_runs.iter().all(|&(o, q)| {
                        o == j
                            || q.vertical != r.vertical
                            || (q.c - r.c).abs() >= ORTHO_SEP
                            || q.h <= r.l + 2.0
                            || r.h <= q.l + 2.0
                    }))
        });
        clear.then_some(c)
    };
    let stuck: Vec<usize> = (0..edges.len())
        .filter(|&j| {
            let e = &edges[j];
            routed(e)
                && (moved.contains_key(&(j, true)) || moved.contains_key(&(j, false)))
                && slid(j, e, &moved).is_none()
        })
        .collect();
    if !stuck.is_empty() {
        for end in faces.values_mut().flatten() {
            if stuck.contains(&end.0) {
                end.3 = Hold::Fixed;
            }
        }
        moved = spread_faces(&faces, &rects, decos, soft);
    }
    for j in 0..edges.len() {
        let Some(c) = slid(j, &edges[j], &moved) else {
            continue;
        };
        let e = &mut edges[j];
        let old = edge_ends(e);
        let (a, b) = (c[0], c[c.len() - 1]);
        for (k, d) in [
            (0, (a.0 - old.0 .0, a.1 - old.0 .1)),
            (2, (b.0 - old.1 .0, b.1 - old.1 .1)),
        ] {
            for q in &mut e.bezier[k..k + 2] {
                *q = (q.0 + d.0, q.1 + d.1);
            }
        }
        e.waypoints = crate::router::smooth(&c, ORTHO_R);
        let line = edge_polyline(e);
        if let Some((_, m, _)) = &mut e.label {
            *m = nearest_on(&line, *m);
        }
    }
    // A routed edge lying along an earlier one (the router can leave
    // two runs a pixel or two apart), or entering its glyph end on a leg
    // shorter than ORTHO_STUB (the router's stub is about the glyph's
    // length, so the corner sits under the glyph and tilts it), is
    // routed again here, and keeps the router's route only if that
    // fails.
    let mut redo = vec![false; edges.len()];
    let mut runs = Runs::default();
    for (j, e) in edges.iter().enumerate().filter(|(_, e)| routed(e)) {
        let mine: Vec<Run> = e
            .waypoints
            .windows(2)
            .map(|w| Run::of(w[0], w[1]))
            .collect();
        let c = corners_of(&e.waypoints);
        let stubby = c.len() >= 3 && decos.get(j).is_some_and(|d| d[1].half > 0.0) && {
            let (p, q) = (c[c.len() - 1], c[c.len() - 2]);
            (p.0 - q.0).abs() + (p.1 - q.1).abs() < ORTHO_STUB
        };
        redo[j] = e.from != e.to
            && (stubby
                || mine.iter().any(|r| {
                    runs.within(r.vertical, r.c - ORTHO_SEP, r.c + ORTHO_SEP)
                        .iter()
                        .any(|q| q.l < r.h - 2.0 && r.l + 2.0 < q.h)
                }));
        if !redo[j] {
            for r in mine {
                runs.push(r);
            }
        }
    }
    // Each end on a shared face reserves its square exit where the
    // spread put it: until its own edge is drawn (or for good, if that
    // keeps its curve) no other route may run along it.
    for (&(_, key), ends) in &faces {
        if ends.len() < 2 {
            continue;
        }
        let n = (f64::from(key.0), f64::from(key.1));
        for &(j, start, _, _) in ends {
            let e = &edges[j];
            if !free(e) && !redo[j] {
                continue;
            }
            let (a, b) = edge_ends(e);
            let p = moved
                .get(&(j, start))
                .copied()
                .unwrap_or(if start { a } else { b });
            let mut stub = Run::of(p, (p.0 + n.0 * ORTHO_STUB, p.1 + n.1 * ORTHO_STUB));
            stub.owner = j;
            runs.push(stub);
        }
    }
    // Shortest first: short edges have the fewest ways to go, and a
    // back-edge bow — the longest way round — fits its lane and legs
    // around routes (and labels) already final.
    let mut order: Vec<usize> = (0..edges.len()).collect();
    // Edges whose line (and label) is final: the kept ones, then each
    // free one once handled. Only their labels count against a route —
    // a free edge's label still sits on its old curve and moves onto
    // its new line when that edge's turn comes.
    let mut settled: Vec<bool> = edges
        .iter()
        .zip(&redo)
        .map(|(e, &redo)| !free(e) && !redo)
        .collect();
    let (mut curves, mut cost) = (0, 0);
    let cramped = Cell::new(false);
    // Whether an end on box `i`'s face with outward normal `n` is the
    // only end there. Ends are looked up by the face their point is
    // nearest; one that no longer finds the face it was collected on (a
    // point on a corner, which two faces tie for) is alone on its face.
    let alone_on =
        |i: usize, n: (f64, f64)| faces.get(&(i, face_key(n))).map_or(true, |f| f.len() == 1);
    order.sort_by(|&p, &q| {
        let span = |e: &SceneEdge| {
            let (a, b) = (e.bezier[0], e.bezier[3]);
            (a.0 - b.0).abs() + (a.1 - b.1).abs()
        };
        span(&edges[p]).total_cmp(&span(&edges[q]))
    });
    for j in order {
        let e = &edges[j];
        if !free(e) && !redo[j] {
            continue;
        }
        let Some((ia, ib)) = ends_of(e) else { continue };
        let (a, b) = edge_ends(e);
        let (na, nb) = (face_normal(&nodes[ia], a), face_normal(&nodes[ib], b));
        let alone = (alone_on(ia, na), alone_on(ib, nb));
        let a = moved.get(&(j, true)).copied().unwrap_or(a);
        let b = moved.get(&(j, false)).copied().unwrap_or(b);
        let labels: Vec<Rect> = edges
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != j && settled[k])
            .filter_map(|(_, o)| o.label.as_ref())
            .map(|(_, m, w)| {
                (
                    m.0 - w / 2.0 - 1.0,
                    m.1 - 11.0,
                    m.0 + w / 2.0 + 1.0,
                    m.1 + 11.0,
                )
            })
            .collect();
        settled[j] = true;
        let ctx = Ortho {
            rects: &rects,
            labels: &labels,
            ia,
            ib,
            j,
            runs: &runs,
            room,
            cramped: &cramped,
        };
        let corners = if e.waypoints.len() >= 2 && !redo[j] {
            // From the ends as spread: the chain jogs from a moved end
            // into its first channel.
            let mut chain = e.waypoints.clone();
            let last = chain.len() - 1;
            (chain[0], chain[last]) = (a, b);
            ctx.chain(&chain, na, nb)
        } else if na.0 * nb.0 + na.1 * nb.1 < -0.5 {
            ctx.opposite(e, a, b, na, alone)
        } else if (na.0 * nb.0 + na.1 * nb.1).abs() < 0.5 {
            ctx.perpendicular(a, b, na, nb)
        } else {
            None
        };
        // Spread places the ends on a face blind to the routes past it,
        // so a free edge's end can sit a few px beside a lane another
        // route keeps by that face, with no square exit left. Before
        // the edge keeps its curve, each end tries places along its
        // face (as far as the other ends there allow): just clear of
        // each such lane, nearest first, then small nudges either way
        // (while the pass has [`ORTHO_NUDGE_WORK`] left).
        let nudged =
            if nudges > 0 && corners.is_none() && free(e) && na.0 * nb.0 + na.1 * nb.1 < -0.5 {
                // Along the face: x on a horizontal face, else y.
                let horizontal = na.0 == 0.0;
                let along = |p: (f64, f64)| if horizontal { p.0 } else { p.1 };
                let across = |p: (f64, f64)| if horizontal { p.1 } else { p.0 };
                let (s0, s1) = (across(a).min(across(b)), across(a).max(across(b)));
                let mut tries: Vec<(bool, f64)> = Vec::new();
                for (start, p) in [(true, a), (false, b)] {
                    let t = along(p);
                    let near = ORTHO_SEP + ORTHO_NUDGES[ORTHO_NUDGES.len() - 1].abs();
                    for q in runs.within(horizontal, t - near, t + near) {
                        if q.owner != j && q.l < s1 && s0 < q.h {
                            let clear = ORTHO_SEP + 0.5;
                            tries.extend([(start, q.c - clear - t), (start, q.c + clear - t)]);
                        }
                    }
                }
                tries.sort_by(|p, q| p.1.abs().total_cmp(&q.1.abs()));
                for &d in &ORTHO_NUDGES {
                    tries.extend([(true, d), (false, d)]);
                }
                let shift = |p: (f64, f64), d: f64| {
                    if horizontal {
                        (p.0 + d, p.1)
                    } else {
                        (p.0, p.1 + d)
                    }
                };
                tries
                    .into_iter()
                    .map(|(start, d)| {
                        if start {
                            (shift(a, d), b, (ia, na, start))
                        } else {
                            (a, shift(b, d), (ib, nb, start))
                        }
                    })
                    .filter(|&(a2, b2, (i, n, start))| {
                        let p = if start { a2 } else { b2 };
                        let face = (&faces, &moved, &rects[i], decos);
                        end_fits(face, (i, face_key(n)), (j, start), p)
                    })
                    .find_map(|(a2, b2, (_, _, start))| {
                        nudges = nudges.checked_sub(1)?;
                        let c = ctx.opposite(e, a2, b2, na, alone)?;
                        Some((c, (j, start), if start { a2 } else { b2 }))
                    })
            } else {
                None
            };
        let corners = match nudged {
            Some((c, end, p)) => {
                moved.insert(end, p);
                Some(c)
            }
            None => corners,
        };
        let Some(corners) = corners else {
            if redo[j] {
                // The router's route stays: later routes keep off it.
                for w in edges[j].waypoints.windows(2) {
                    runs.push(Run::of(w[0], w[1]));
                }
            } else {
                // The curve stays, from its ends as spread (where the
                // other ends on those faces left it room).
                let e = &mut edges[j];
                let (a0, b0) = edge_ends(e);
                let (da, db) = ((a.0 - a0.0, a.1 - a0.1), (b.0 - b0.0, b.1 - b0.1));
                for (k, d) in [(0, da), (2, db)] {
                    for q in &mut e.bezier[k..k + 2] {
                        *q = (q.0 + d.0, q.1 + d.1);
                    }
                }
                let n = e.waypoints.len();
                if n >= 2 {
                    e.waypoints[0] = a;
                    e.waypoints[n - 1] = b;
                } else {
                    // A cubic leaves each face square, along its control
                    // leg, for about the leg's first third: later routes
                    // keep off that stretch.
                    let q = e.bezier;
                    for (p, c) in [(q[0], q[1]), (q[3], q[2])] {
                        let r = (p.0 + (c.0 - p.0) / 3.0, p.1 + (c.1 - p.1) / 3.0);
                        if (p.0 - r.0).abs() < 0.5 || (p.1 - r.1).abs() < 0.5 {
                            runs.push(Run::of(p, r));
                        }
                    }
                }
                curves += 1;
            }
            continue;
        };
        cost += Ortho { room: false, ..ctx }.cost(&corners).unwrap_or(0);
        let spot = edges[j]
            .label
            .as_ref()
            .map(|l| ctx.label_spot(&corners, l.2, l.1));
        for w in corners.windows(2) {
            runs.push(Run::of(w[0], w[1]));
        }
        draw_route(&mut edges[j], &corners, (na, nb), spot);
    }
    // Last, a line still lying along another (two the router drew, a
    // slid leg, a curve kept from a spread end) is drawn again against
    // all the others, and stays as it is if it cannot be.
    let mut lines: Vec<Vec<Run>> = edges.iter().map(straight_runs).collect();
    let on = |r: &Run, q: &Run| {
        q.vertical == r.vertical
            && (q.c - r.c).abs() < ORTHO_SEP - 0.5
            && q.l < r.h - 2.0
            && r.l + 2.0 < q.h
    };
    for j in 0..edges.len() {
        let e = &edges[j];
        let merged = lines[j].iter().any(|r| {
            lines
                .iter()
                .enumerate()
                .any(|(k, l)| k != j && l.iter().any(|q| on(r, q)))
        });
        if !merged || e.from == e.to || e.meta.curve.is_some() {
            continue;
        }
        let Some((ia, ib)) = ends_of(e) else { continue };
        let mut others = Runs::default();
        for (k, l) in lines.iter().enumerate() {
            if k != j {
                for &q in l {
                    others.push(q);
                }
            }
        }
        let labels: Vec<Rect> = edges
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != j)
            .filter_map(|(_, o)| o.label.as_ref())
            .map(|(_, m, w)| {
                (
                    m.0 - w / 2.0 - 1.0,
                    m.1 - 11.0,
                    m.0 + w / 2.0 + 1.0,
                    m.1 + 11.0,
                )
            })
            .collect();
        let ctx = Ortho {
            rects: &rects,
            labels: &labels,
            ia,
            ib,
            j,
            runs: &others,
            room: false,
            cramped: &cramped,
        };
        let (a, b) = edge_ends(e);
        let (na, nb) = (face_normal(&nodes[ia], a), face_normal(&nodes[ib], b));
        let alone = (alone_on(ia, na), alone_on(ib, nb));
        let corners = if na.0 * nb.0 + na.1 * nb.1 < -0.5 {
            ctx.opposite(e, a, b, na, alone)
        } else if (na.0 * nb.0 + na.1 * nb.1).abs() < 0.5 {
            ctx.perpendicular(a, b, na, nb)
        } else {
            None
        };
        if let Some(corners) = corners {
            let spot = e.label.as_ref().map(|l| ctx.label_spot(&corners, l.2, l.1));
            draw_route(&mut edges[j], &corners, (na, nb), spot);
            lines[j] = straight_runs(&edges[j]);
        }
    }
    ((curves, cost), cramped.get())
}

/// Draw `e` along orthogonal route `corners` (its ends square to faces
/// with outward normals `n`), its label at `spot` brought onto the line.
fn draw_route(
    e: &mut SceneEdge,
    corners: &[(f64, f64)],
    (na, nb): ((f64, f64), (f64, f64)),
    spot: Option<(f64, f64)>,
) {
    let (a, b) = (corners[0], corners[corners.len() - 1]);
    if corners.len() == 2 {
        // Straight: a straight cubic, no waypoints — sampled as finely
        // as any curve (two bare waypoints would be sampled as just
        // their ends, which the label stage would read as one long
        // arrowhead).
        let at = |t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        e.bezier = [a, at(1.0 / 3.0), at(2.0 / 3.0), b];
        e.waypoints.clear();
    } else {
        // The single-cubic fallback keeps square ends too.
        let k = ORTHO_STUB;
        e.bezier = [
            a,
            (a.0 + na.0 * k, a.1 + na.1 * k),
            (b.0 + nb.0 * k, b.1 + nb.1 * k),
            b,
        ];
        e.waypoints = crate::router::smooth(corners, ORTHO_R);
    }
    let line = edge_polyline(e);
    if let (Some((_, m, _)), Some(spot)) = (&mut e.label, spot) {
        *m = nearest_on(&line, spot);
    }
}

/// The straight axis-aligned stretches of `e` as drawn: its route's
/// legs, the whole of a straight cubic, or a curve's ends as far as it
/// runs straight out of its faces (within a pixel of the line it
/// leaves along).
fn straight_runs(e: &SceneEdge) -> Vec<Run> {
    let pts = if e.waypoints.is_empty() {
        let q = e.bezier;
        let line = [q[1], q[2]].iter().all(|c| {
            ((q[3].0 - q[0].0) * (c.1 - q[0].1) - (q[3].1 - q[0].1) * (c.0 - q[0].0)).abs() < 1e-6
        });
        if !line {
            let pts = edge_polyline(e);
            let mut tails = Vec::new();
            for (p, rest) in [
                (q[0], pts.iter().collect::<Vec<_>>()),
                (q[3], pts.iter().rev().collect()),
            ] {
                // Along x (a horizontal stretch) or along y.
                for along_x in [true, false] {
                    let off = |r: &(f64, f64)| {
                        if along_x {
                            (r.1 - p.1).abs()
                        } else {
                            (r.0 - p.0).abs()
                        }
                    };
                    let reach = rest.iter().take_while(|r| off(r) < 1.0).last();
                    if let Some(&&r) = reach {
                        let r = if along_x { (r.0, p.1) } else { (p.0, r.1) };
                        if (r.0 - p.0).abs() + (r.1 - p.1).abs() > 4.0 {
                            tails.push(Run::of(p, r));
                        }
                    }
                }
            }
            return tails;
        }
        vec![q[0], q[3]]
    } else {
        corners_of(&e.waypoints)
    };
    pts.windows(2)
        .filter(|w| (w[0].0 - w[1].0).abs() < 0.5 || (w[0].1 - w[1].1).abs() < 0.5)
        .map(|w| Run::of(w[0], w[1]))
        .collect()
}

/// An edge end on a box face: (edge, at its start, place along the
/// face, how it may move).
type FaceEnd = (usize, bool, f64, Hold);

/// The ends on each face of each box, keyed by (box, outward normal
/// rounded), as [`ortho_pass`] collects them.
type Faces = BTreeMap<(usize, (i8, i8)), Vec<FaceEnd>>;

/// Where [`spread_faces`] (or a nudge) moved each end, keyed by (edge,
/// at its start).
type Moved = HashMap<(usize, bool), (f64, f64)>;

/// How an end may move when the ends on its face spread.
#[derive(Clone, Copy, PartialEq)]
enum Hold {
    /// A free edge's plain curve: moves.
    Free,
    /// A long edge's chain end, or a routed end that can slide: moves
    /// only when, held, the face would crowd glyphs — moving it costs
    /// its route a jog.
    Soft,
    /// Stays.
    Fixed,
}

/// The corners of an axis-aligned route `pts` (its points with each
/// one in line with both neighbours dropped).
fn corners_of(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut c: Vec<(f64, f64)> = Vec::with_capacity(pts.len());
    for &p in pts {
        if c.last()
            .is_some_and(|q| (q.0 - p.0).abs() < 0.5 && (q.1 - p.1).abs() < 0.5)
        {
            continue;
        }
        if let [.., u, v] = c[..] {
            let in_line = |i: usize| {
                let k = |q: (f64, f64)| if i == 0 { q.0 } else { q.1 };
                (k(u) - k(v)).abs() < 0.5 && (k(v) - k(p)).abs() < 0.5
            };
            if in_line(0) || in_line(1) {
                c.pop();
            }
        }
        c.push(p);
    }
    c
}

/// Whether the end (`at_start` or last) of a route with corners `c`,
/// on a face with outward normal `n`, can slide along that face: its
/// final leg leaves the face square, at least [`ORTHO_STUB`] long, and
/// turns at a knee, so moving the leg sideways only stretches the leg
/// before it. (A shorter final leg is a stair step into a channel
/// right by the face; moving it would leave that channel where the
/// end was.)
fn slides(c: &[(f64, f64)], at_start: bool, n: (f64, f64)) -> bool {
    if c.len() < 3 {
        return false;
    }
    let (end, knee) = if at_start {
        (c[0], c[1])
    } else {
        (c[c.len() - 1], c[c.len() - 2])
    };
    // Square to a horizontal face = a vertical leg, and vice versa.
    let vertical = (end.0 - knee.0).abs() < 0.5;
    let long = (end.0 - knee.0).abs() + (end.1 - knee.1).abs() >= ORTHO_STUB;
    vertical == (n.0 == 0.0) && long
}

/// Move the end (`at_start` or last) of corners `c` ([`slides`]) to
/// `p`: its final leg moves sideways with it.
fn slide(c: &mut [(f64, f64)], at_start: bool, p: (f64, f64)) {
    let n = c.len();
    let (end, knee) = if at_start { (0, 1) } else { (n - 1, n - 2) };
    if (c[end].0 - c[knee].0).abs() < 0.5 {
        c[knee].0 = p.0;
    } else {
        c[knee].1 = p.1;
    }
    c[end] = p;
}

/// Clear space kept between the outlines of two end glyphs side by
/// side on one face.
const GLYPH_GAP: f64 = 4.0;

/// What an edge end holds beside its line: its glyph's half width (0 =
/// none) and its cardinality's size, if it has one.
#[derive(Clone, Copy, Default)]
struct EndDeco {
    half: f64,
    card: Option<(f64, f64)>,
}

/// The room an end keeps along its face toward the face's low corner
/// (`lo`) and its high one (`hi`), and its glyph's half width.
#[derive(Clone, Copy)]
struct Room {
    lo: f64,
    hi: f64,
    half: f64,
}

/// Room a cardinality takes along a face beside its end's glyph (`half`
/// its half width): as [`place_cards`] puts it, [`CARD_CLEAR`] and a
/// pixel past the glyph, `along` long (its extent along the face) — and
/// past it as much again as the glyph takes, so the card sits nearer
/// its own end than its neighbour's.
fn card_room(along: f64, half: f64) -> f64 {
    1.0 + CARD_CLEAR + along + half
}

/// The rooms of the `k` ends on a face, `decos` theirs in order along
/// it (`horizontal` = the face runs along x): each glyph's half width
/// to both sides, and the cardinality's room to the side
/// [`place_cards`] puts it — toward the end's own corner, the middle end
/// of an odd count along the face's tangent (`t_up` = that runs toward
/// the high corner).
fn face_rooms(decos: &[EndDeco], horizontal: bool, t_up: bool) -> Vec<Room> {
    let k = decos.len();
    decos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let card = d.card.map_or(0.0, |(w, h)| {
                card_room(if horizontal { w } else { h }, d.half)
            });
            let low = 2 * i + 1 < k || (2 * i + 1 == k && !t_up);
            Room {
                lo: d.half + if low { card } else { 0.0 },
                hi: d.half + if low { 0.0 } else { card },
                half: d.half,
            }
        })
        .collect()
}

/// Length a face needs for ends holding `ends` — (glyph half width,
/// cardinality room) each, in no known order — to sit apart as
/// [`spread`] places them when crowded: each outer glyph just inside a
/// corner, neighbours the widest pair's spacing apart plus the
/// cardinalities between them. The two outermost ends put theirs past
/// the corners; which ends those are is not known yet, so the two
/// smallest are taken. 0 for fewer than two ends.
fn face_room(ends: &[(f64, f64)]) -> f64 {
    if ends.len() < 2 {
        return 0.0;
    }
    let widest = ends.iter().map(|e| e.0).fold(0.0, f64::max);
    let step = (2.0 * widest + GLYPH_GAP).max(2.0 * ORTHO_SEP);
    let mut cards: Vec<f64> = ends.iter().map(|e| e.1).collect();
    cards.sort_by(f64::total_cmp);
    2.0 * (widest + 2.0) + (ends.len() - 1) as f64 * step + cards[2..].iter().sum::<f64>()
}

/// Where [`spread`] puts the moving ends of every face with two ends or
/// more (`faces` as `ortho_pass` collects them, `rects` the boxes,
/// `decos` what each edge holds at its ends): edge end -> its new place.
/// Soft ends move only on a face that would crowd without them moving,
/// or everywhere with `soft`.
fn spread_faces(
    faces: &BTreeMap<(usize, (i8, i8)), Vec<FaceEnd>>,
    rects: &[Rect],
    decos: &[[EndDeco; 2]],
    soft: bool,
) -> HashMap<(usize, bool), (f64, f64)> {
    let deco = |&(j, start, _, _): &FaceEnd| {
        decos
            .get(j)
            .map_or(EndDeco::default(), |d| d[usize::from(!start)])
    };
    let mut moved = HashMap::new();
    for (&(i, key), ends) in faces {
        if ends.len() < 2 {
            continue;
        }
        let r = rects[i];
        let (lo, hi) = if key.0 == 0 { (r.0, r.2) } else { (r.1, r.3) };
        let mut ends = ends.clone();
        ends.sort_by(|p, q| p.2.total_cmp(&q.2));
        let t_up = -f64::from(key.1) + f64::from(key.0) > 0.0;
        let decs: Vec<EndDeco> = ends.iter().map(deco).collect();
        let rooms = face_rooms(&decs, key.0 == 0, t_up);
        // Places along the face for the ends that move — the soft ones
        // too when `soft`.
        let place = |soft: bool| {
            let mut got: Vec<((usize, bool), f64)> = Vec::new();
            // (place, index of the next end, room toward the group of
            // the fixed end there — `None` at a corner).
            let mut from = (lo, 0, None);
            for k in 0..=ends.len() {
                let fixed = ends
                    .get(k)
                    .map(|e| !(e.3 == Hold::Free || (soft && e.3 == Hold::Soft)));
                if fixed == Some(false) {
                    continue;
                }
                // Moving ends `from.1 .. k` share the stretch up to here.
                let to = ends.get(k).map_or((hi, None), |e| (e.2, Some(rooms[k].lo)));
                let group = &ends[from.1..k];
                if let Some(places) = spread((from.0, from.2), to, &rooms[from.1..k]) {
                    for (&(j, start, _, _), t) in group.iter().zip(places) {
                        got.push(((j, start), t));
                    }
                }
                from = (to.0, k + 1, rooms.get(k).map(|r| r.hi));
            }
            got
        };
        // Whether, placed so, two neighbours' glyphs or cardinalities
        // touch or their lanes lie closer than ORTHO_SEP.
        let crowded = |got: &[((usize, bool), f64)]| {
            let mut at: Vec<(f64, Room)> = ends
                .iter()
                .zip(&rooms)
                .map(|(e, &r)| {
                    let t = got.iter().find(|g| g.0 == (e.0, e.1)).map_or(e.2, |g| g.1);
                    (t, r)
                })
                .collect();
            at.sort_by(|p, q| p.0.total_cmp(&q.0));
            at.windows(2).any(|w| {
                let gap = w[1].0 - w[0].0;
                gap < ORTHO_SEP || gap < w[0].1.hi + w[1].1.lo + GLYPH_GAP
            })
        };
        let mut got = place(soft);
        if !soft && ends.iter().any(|e| e.3 == Hold::Soft) && crowded(&got) {
            got = place(true);
        }
        for ((j, start), t) in got {
            let p = if key.0 == 0 {
                (t, if key.1 > 0 { r.3 } else { r.1 })
            } else {
                (if key.0 > 0 { r.2 } else { r.0 }, t)
            };
            moved.insert((j, start), p);
        }
    }
    moved
}

/// Places along a face for free ends keeping `rooms` (in order) between
/// `lo` and `hi`: each bound a place and the room the fixed end there
/// keeps toward these ends, `None` for a corner. Evenly, as far from the
/// bounds as from each other, when that clears every glyph and
/// cardinality (and leaves lanes [`ORTHO_SEP`] apart); crowded, each
/// outer glyph just inside its corner and neighbours as far apart as
/// their rooms need, the slack shared out evenly (or, short of room,
/// as far apart as the stretch allows). `None` leaves the ends where
/// the port stage put them (nothing reads better there).
fn spread(
    (lo, lo_room): (f64, Option<f64>),
    (hi, hi_room): (f64, Option<f64>),
    rooms: &[Room],
) -> Option<Vec<f64>> {
    let k = rooms.len();
    let (first, last) = (*rooms.first()?, *rooms.last()?);
    let need = |a: f64, b: f64| (a + b + GLYPH_GAP).max(2.0 * ORTHO_SEP);
    let needs: Vec<f64> = rooms.windows(2).map(|w| need(w[0].hi, w[1].lo)).collect();
    let mut worst = needs.iter().copied().fold(0.0, f64::max);
    for (bound, own) in [(lo_room, first.lo), (hi_room, last.hi)] {
        if let Some(b) = bound {
            worst = worst.max(need(b, own));
        }
    }
    let even = (hi - lo) / (k + 1) as f64;
    if even >= worst.max(2.0 * ORTHO_SEP) {
        return Some((1..=k).map(|i| lo + i as f64 * even).collect());
    }
    if k < 2 {
        return None;
    }
    // Crowded: the outer ends close to the bounds — a glyph's half
    // width plus a little inside a corner, a full spacing from a fixed
    // end — and the rest as their rooms need, or evenly between.
    let margin =
        |bound: Option<f64>, own: f64, half: f64| bound.map_or(half + 2.0, |b| need(b, own));
    let (m_lo, m_hi) = (
        margin(lo_room, first.lo, first.half),
        margin(hi_room, last.hi, last.half),
    );
    let avail = hi - lo - m_lo - m_hi;
    let slack = avail - needs.iter().sum::<f64>();
    if slack >= 0.0 {
        let add = slack / (k - 1) as f64;
        let mut t = lo + m_lo;
        let mut out = vec![t];
        for n in needs {
            t += n + add;
            out.push(t);
        }
        return Some(out);
    }
    let step = avail / (k - 1) as f64;
    if step >= even && step >= ORTHO_SEP {
        Some((0..k).map(|i| lo + m_lo + i as f64 * step).collect())
    } else if even >= ORTHO_SEP {
        Some((1..=k).map(|i| lo + i as f64 * even).collect())
    } else {
        None
    }
}

/// One straight stretch of a placed orthogonal route: at `c` on the
/// fixed axis, from `l` to `h` along the other; `vertical` = runs
/// along y.
#[derive(Clone, Copy)]
struct Run {
    c: f64,
    l: f64,
    h: f64,
    vertical: bool,
    /// The edge it belongs to when that edge is still to be drawn (an
    /// end's reserved exit), else `usize::MAX`: an edge's own runs never
    /// count against it.
    owner: usize,
}

/// What an orthogonal route is checked against: every box (`ia`, `ib`
/// are its own end boxes), the other edges' labels and the runs placed
/// so far.
struct Ortho<'a> {
    rects: &'a [Rect],
    labels: &'a [Rect],
    ia: usize,
    ib: usize,
    /// The edge being routed (its own reserved runs are no obstacle).
    j: usize,
    runs: &'a Runs,
    /// Keep [`ORTHO_ROOM`] between parallel runs where there is room.
    room: bool,
    /// Set once a candidate pays [`ORTHO_CRAMPED`]: only then can the
    /// room kept have cost another edge its lane.
    cramped: &'a Cell<bool>,
}

impl Ortho<'_> {
    /// Whether polyline `pts` crosses a box; its own end boxes may be
    /// touched (at the ends) but not entered.
    fn blocked(&self, pts: &[(f64, f64)]) -> bool {
        pts.windows(2).any(|s| {
            let (x0, x1) = (s[0].0.min(s[1].0), s[0].0.max(s[1].0));
            let (y0, y1) = (s[0].1.min(s[1].1), s[0].1.max(s[1].1));
            self.rects.iter().enumerate().any(|(k, &r)| {
                if r.2 < x0 || x1 < r.0 || r.3 < y0 || y1 < r.1 {
                    return false;
                }
                let r = if k == self.ia || k == self.ib {
                    (r.0 + 0.5, r.1 + 0.5, r.2 - 0.5, r.3 - 0.5)
                } else {
                    r
                };
                rect_hits_segment(r, s[0], s[1])
            })
        })
    }

    /// What polyline `pts` costs among the routes placed so far:
    /// `None` through a box or along a placed run (closer than
    /// [`ORTHO_SEP`]); else [`ORTHO_UNDER_LABEL`] per stretch under
    /// another edge's label, [`ORTHO_CROSSING`] per crossing and
    /// [`ORTHO_CRAMPED`] per stretch closer than [`ORTHO_ROOM`] beside
    /// a placed run.
    fn cost(&self, pts: &[(f64, f64)]) -> Option<usize> {
        self.cost_under(pts, usize::MAX)
    }

    /// [`Ortho::cost`], or `None` as soon as it reaches `bound` (a
    /// candidate that cannot beat the best so far).
    fn cost_under(&self, pts: &[(f64, f64)], bound: usize) -> Option<usize> {
        if self.blocked(pts) {
            return None;
        }
        let mut c = 0;
        let near = if self.room { ORTHO_ROOM } else { ORTHO_SEP };
        for w in pts.windows(2) {
            let r = Run::of(w[0], w[1]);
            for q in self.runs.within(r.vertical, r.c - near, r.c + near) {
                if q.owner != self.j && q.l < r.h - 2.0 && r.l + 2.0 < q.h {
                    let gap = (q.c - r.c).abs();
                    if gap < ORTHO_SEP {
                        return None;
                    }
                    if self.room && gap < ORTHO_ROOM {
                        c += ORTHO_CRAMPED;
                        self.cramped.set(true);
                    }
                }
            }
            for q in self.runs.within(!r.vertical, r.l + 1.0, r.h - 1.0) {
                if q.owner != self.j && q.l + 1.0 < r.c && r.c < q.h - 1.0 {
                    c += ORTHO_CROSSING;
                }
            }
            if self
                .labels
                .iter()
                .any(|&b| rect_hits_segment(b, w[0], w[1]))
            {
                c += ORTHO_UNDER_LABEL;
            }
            if c >= bound {
                return None;
            }
        }
        Some(c)
    }

    /// The cheapest of `cands` ([`Ortho::cost`]; the earlier wins a
    /// tie, so candidates come in order of preference); `None` when
    /// each one crosses a box or lies on a placed run.
    fn best(
        &self,
        cands: impl Iterator<Item = Vec<(f64, f64)>>,
    ) -> Option<(usize, Vec<(f64, f64)>)> {
        let mut best: Option<(usize, Vec<(f64, f64)>)> = None;
        for pts in cands {
            let bound = best.as_ref().map_or(usize::MAX, |b| b.0);
            if let Some(c) = self.cost_under(&pts, bound) {
                best = Some((c, pts));
                // Nothing beats free, and the earlier is preferred.
                if c == 0 {
                    break;
                }
            }
        }
        best
    }

    /// Opposite faces `a` (normal `na`) and `b`: straight, a Z, or —
    /// blocked between — the bow's lane. `alone` = each end is the
    /// only one on its face, free to slide along it.
    fn opposite(
        &self,
        e: &SceneEdge,
        a: (f64, f64),
        b: (f64, f64),
        na: (f64, f64),
        alone: (bool, bool),
    ) -> Option<Vec<(f64, f64)>> {
        // Work as if the faces were horizontal (`flip` swaps x / y).
        let flip = na.0 != 0.0;
        let f = |p: (f64, f64)| if flip { (p.1, p.0) } else { p };
        let back = |pts: Vec<(f64, f64)>| pts.into_iter().map(f).collect::<Vec<_>>();
        let (ra, rb) = (
            f_rect(self.rects[self.ia], flip),
            f_rect(self.rects[self.ib], flip),
        );
        let (mut fa, mut fb) = (f(a), f(b));
        let s = f(na).1; // +1: a's face looks down (+y), -1: up
        if (fb.1 - fa.1) * s < 2.0 * ORTHO_STUB {
            return None;
        }
        let span = |r: Rect| (r.0 + ORTHO_INSET, r.2 - ORTHO_INSET);
        let ((alo, ahi), (blo, bhi)) = (span(ra), span(rb));
        let (lo, hi) = (alo.max(blo), ahi.min(bhi));
        let (sa, sb) = (fa.0, fb.0);
        if alone.0 && alone.1 && lo <= hi {
            let x = ((fa.0 + fb.0) / 2.0).clamp(lo, hi);
            (fa.0, fb.0) = (x, x);
        } else if alone.0 && (alo..=ahi).contains(&fb.0) {
            fa.0 = fb.0;
        } else if alone.1 && (blo..=bhi).contains(&fa.0) {
            fb.0 = fa.0;
        } else {
            if alone.0 && alo <= ahi {
                fa.0 = fa.0.clamp(alo, ahi);
            }
            if alone.1 && blo <= bhi {
                fb.0 = fb.0.clamp(blo, bhi);
            }
        }
        let (glo, ghi) = (fa.1.min(fb.1) + ORTHO_STUB, fa.1.max(fb.1) - ORTHO_STUB);
        let direct = if (fa.0 - fb.0).abs() < 0.5 {
            self.best(std::iter::once(back(vec![fa, (fa.0, fb.1)])))
        } else {
            let mid = (fa.1 + fb.1) / 2.0;
            let span = (fa.0.min(fb.0), fa.0.max(fb.0));
            self.best(
                self.jog_places(mid, (glo, ghi), span, flip)
                    .into_iter()
                    .map(|y| back(vec![fa, (fa.0, y), (fb.0, y), fb])),
            )
        };
        if let Some((c, pts)) = &direct {
            if *c < ORTHO_UNDER_LABEL {
                return Some(pts.clone());
            }
        }
        // Blocked between (or crowded): a lane beside the way, entered
        // and left square to the faces from the ends as laid out — at
        // the curve's outer extent when it bows out past both ends (a
        // back-edge bow); else, with no direct way left, just past the
        // boxes in the way (or the ends), on either side.
        let (fa, fb) = ((sa, fa.1), (sb, fb.1));
        let mid = (fa.0 + fb.0) / 2.0;
        let (near, far) = (fa.0.min(fb.0) - ORTHO_STUB, fa.0.max(fb.0) + ORTHO_STUB);
        let bow = edge_polyline(e)
            .into_iter()
            .map(|p| f(p).0)
            .max_by(|p, q| (p - mid).abs().total_cmp(&(q - mid).abs()))
            .unwrap_or(mid);
        let starts = if bow < near {
            [Some((bow, -1.0)), None]
        } else if bow > far {
            [Some((bow, 1.0)), None]
        } else if direct.is_none() {
            // Past every box between the faces across the ends' span.
            let (ylo, yhi) = (fa.1.min(fb.1), fa.1.max(fb.1));
            let (mut lo, mut hi) = (near, far);
            for (k, &r) in self.rects.iter().enumerate() {
                let r = f_rect(r, flip);
                if k != self.ia
                    && k != self.ib
                    && r.1 < yhi
                    && ylo < r.3
                    && r.0 <= fa.0.max(fb.0)
                    && fa.0.min(fb.0) <= r.2
                {
                    lo = lo.min(r.0 - ORTHO_STUB);
                    hi = hi.max(r.2 + ORTHO_STUB);
                }
            }
            [Some((lo, -1.0)), Some((hi, 1.0))]
        } else {
            return direct.map(|d| d.1);
        };
        // Each leg as close to its face as is cheap; the lane at its
        // start, or a step in or out of it.
        let half = ((fb.1 - fa.1) * s / 2.0).max(ORTHO_STUB);
        let leg = |from: (f64, f64), dir: f64, x: f64| {
            self.best(
                (0..6)
                    .map(|k| from.1 + dir * (ORTHO_STUB + k as f64 * ORTHO_SEP))
                    .filter(|y| (y - from.1).abs() <= half)
                    .map(|y| back(vec![from, (from.0, y), (x, y)])),
            )
            .map(|(_, p)| f(p[1]).1)
        };
        let lanes = self.best(starts.into_iter().flatten().flat_map(|(lane, out)| {
            [0, -1, 1, -2, 2, 3, 4, 5].into_iter().filter_map(move |k| {
                let x = lane + out * f64::from(k) * ORTHO_SEP;
                let (ya, yb) = (leg(fa, s, x)?, leg(fb, -s, x)?);
                Some(back(vec![fa, (fa.0, ya), (x, ya), (x, yb), (fb.0, yb), fb]))
            })
        }));
        match (direct, lanes) {
            (Some(d), Some(l)) => Some(if l.0 < d.0 { l.1 } else { d.1 }),
            (d, l) => d.or(l).map(|r| r.1),
        }
    }

    /// Perpendicular faces: one turn, square to both.
    fn perpendicular(
        &self,
        a: (f64, f64),
        b: (f64, f64),
        na: (f64, f64),
        nb: (f64, f64),
    ) -> Option<Vec<(f64, f64)>> {
        let c = if na.0 == 0.0 { (a.0, b.1) } else { (b.0, a.1) };
        let out_a = (c.0 - a.0) * na.0 + (c.1 - a.1) * na.1;
        let out_b = (c.0 - b.0) * nb.0 + (c.1 - b.1) * nb.1;
        let pts = vec![a, c, b];
        (out_a >= ORTHO_STUB && out_b >= ORTHO_STUB && self.cost(&pts).is_some()).then_some(pts)
    }

    /// A long edge's layout chain (`pts`, its ends on faces `na` and
    /// `nb` across the flow): straight down each channel, a jog
    /// between channels in the gap between their points.
    fn chain(&self, pts: &[(f64, f64)], na: (f64, f64), nb: (f64, f64)) -> Option<Vec<(f64, f64)>> {
        let flip = na.0 != 0.0;
        if (nb.0 != 0.0) != flip || na.0 * nb.0 + na.1 * nb.1 > -0.5 {
            return None;
        }
        let f = |p: (f64, f64)| if flip { (p.1, p.0) } else { p };
        let back = |pts: Vec<(f64, f64)>| pts.into_iter().map(f).collect::<Vec<_>>();
        let q: Vec<(f64, f64)> = pts.iter().map(|&p| f(p)).collect();
        let last = q.len() - 1;
        let mut out = vec![q[0]];
        let mut x = q[0].0;
        for i in 1..=last {
            let p = q[i];
            // A channel within a few px of the current run continues
            // it (the end slides along its face to meet it).
            if (p.0 - x).abs() < 4.0 {
                continue;
            }
            let lo = q[i - 1].1.min(p.1) + if i == 1 { ORTHO_STUB } else { 8.0 };
            let hi = q[i - 1].1.max(p.1) - if i == last { ORTHO_STUB } else { 8.0 };
            let from = *out.last().unwrap_or(&q[0]);
            let span = (x.min(p.0), x.max(p.0));
            let (_, jog) = self.best(
                self.jog_places((q[i - 1].1 + p.1) / 2.0, (lo, hi), span, flip)
                    .into_iter()
                    .map(|y| back(vec![from, (x, y), (p.0, y), p])),
            )?;
            let y = f(jog[1]).1;
            out.push((x, y));
            out.push((p.0, y));
            x = p.0;
        }
        out.push((x, q[last].1));
        let out = back(out);
        self.cost(&out).is_some().then_some(out)
    }

    /// Where a label `w` wide sits on route `pts` (its corners): where
    /// it was (`was`, brought onto the route) when its chip covers no
    /// other route's run, box or settled label there; else the clear
    /// spot nearest the route's middle, else the one covering the
    /// fewest. A chip over another route's corner reads as joining
    /// that route, and the label stage would leave it there (it only
    /// moves labels off boxes, glyphs and labels).
    fn label_spot(&self, pts: &[(f64, f64)], w: f64, was: (f64, f64)) -> (f64, f64) {
        let seg_len = |s: &[(f64, f64)]| (s[1].0 - s[0].0).abs() + (s[1].1 - s[0].1).abs();
        let total: f64 = pts.windows(2).map(seg_len).sum();
        // The point `d` along the route, and the stretch it lies on.
        let at = |mut d: f64| {
            for (i, s) in pts.windows(2).enumerate() {
                let l = seg_len(s);
                if d <= l && l > 0.0 {
                    let t = d / l;
                    let p = (
                        s[0].0 + (s[1].0 - s[0].0) * t,
                        s[0].1 + (s[1].1 - s[0].1) * t,
                    );
                    return (p, Some(i));
                }
                d -= l;
            }
            (pts[pts.len() - 1], Some(pts.len() - 2))
        };
        let mid = total / 2.0;
        let was = nearest_on(pts, was);
        if total <= 2.0 * ORTHO_STUB {
            return was;
        }
        let chip_at = |p: (f64, f64)| {
            (
                p.0 - w / 2.0 - 1.0,
                p.1 - 11.0,
                p.0 + w / 2.0 + 1.0,
                p.1 + 11.0,
            )
        };
        let hits = |chip: Rect,
                    runs: &mut dyn Iterator<Item = &Run>,
                    boxes: &mut dyn Iterator<Item = &Rect>| {
            runs.filter(|q| {
                let (a, b) = q.ends();
                rect_hits_segment(chip, a, b)
            })
            .count()
                + boxes.filter(|&&r| rects_meet(chip, r)).count()
        };
        // What a chip sliding along stretch `i` can meet — the runs,
        // boxes and labels across the area it sweeps (a px wider) —
        // gathered the first time a spot lands on the stretch, so each
        // spot is tested against those alone (the same count).
        let gather = |i: usize| -> (Vec<Run>, Vec<Rect>) {
            let (a, b) = (chip_at(pts[i]), chip_at(pts[i + 1]));
            let swept = (
                a.0.min(b.0) - 1.0,
                a.1.min(b.1) - 1.0,
                a.2.max(b.2) + 1.0,
                a.3.max(b.3) + 1.0,
            );
            let runs = self
                .runs
                .all()
                .filter(|q| {
                    let (a, b) = q.ends();
                    rect_hits_segment(swept, a, b)
                })
                .copied()
                .collect();
            let boxes = self
                .rects
                .iter()
                .chain(self.labels)
                .filter(|&&r| rects_meet(swept, r))
                .copied()
                .collect();
            (runs, boxes)
        };
        let mut near: Vec<Option<(Vec<Run>, Vec<Rect>)>> = vec![None; pts.len() - 1];
        // Where it was, then every LABEL_SPOT_STEP px from stub to stub,
        // nearest the middle first, within LABEL_SPOT_REACH of it.
        let n = ((mid - ORTHO_STUB).min(LABEL_SPOT_REACH) / LABEL_SPOT_STEP).floor() as i32;
        let steps = (0..=2 * n).map(|k| if k % 2 == 1 { -(k + 1) / 2 } else { k / 2 });
        let mut best = (usize::MAX, was);
        for (p, seg) in std::iter::once((was, None))
            .chain(steps.map(|k| at(mid + f64::from(k) * LABEL_SPOT_STEP)))
        {
            let chip = chip_at(p);
            let hits = match seg {
                Some(i) => {
                    let (runs, boxes) = near[i].get_or_insert_with(|| gather(i));
                    hits(chip, &mut runs.iter(), &mut boxes.iter())
                }
                None => hits(
                    chip,
                    &mut self.runs.all(),
                    &mut self.rects.iter().chain(self.labels),
                ),
            };
            if hits < best.0 {
                best = (hits, p);
                if hits == 0 {
                    break;
                }
            }
        }
        best.1
    }

    /// Where a jog across a gap may run, nearest `mid` first, within
    /// `lo..=hi` (coordinates as `opposite` / `chain` work in: `flip`
    /// swaps x / y, the jog spans `span` across): the [`ORTHO_SEP`]
    /// grid around `mid`, plus places [`ORTHO_SEP`] (and, with `room`,
    /// [`ORTHO_ROOM`]) to either side of each parallel run already
    /// placed over `span`.
    /// Jogs sharing a gap but centred on different mids (ends on boxes
    /// of different heights) then stack at a readable spacing, or as
    /// tight as allowed, instead of losing a lane between two grids.
    fn jog_places(&self, mid: f64, (lo, hi): (f64, f64), span: (f64, f64), flip: bool) -> Vec<f64> {
        let mut ys: Vec<f64> = offsets(mid).collect();
        // Parallel to the jog = horizontal in the unflipped frame.
        let reach = ORTHO_ROOM + 1.0;
        for q in self.runs.within(flip, lo - reach, hi + reach) {
            if q.l < span.1 && span.0 < q.h {
                ys.extend([q.c - ORTHO_SEP, q.c + ORTHO_SEP]);
                if self.room {
                    ys.extend([q.c - ORTHO_ROOM, q.c + ORTHO_ROOM]);
                }
            }
        }
        ys.retain(|y| (lo..=hi).contains(y));
        // Nearest the middle first; of two as near, the later one (as
        // `offsets` orders them).
        ys.sort_by(|a, b| {
            (a - mid)
                .abs()
                .total_cmp(&(b - mid).abs())
                .then(b.total_cmp(a))
        });
        ys.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        ys
    }
}

/// `mid`, then steps of [`ORTHO_SEP`] alternately after and before it.
fn offsets(mid: f64) -> impl Iterator<Item = f64> {
    (0..13).map(move |k| {
        let step = ((k + 1) / 2) as f64 * ORTHO_SEP;
        if k % 2 == 1 {
            mid + step
        } else {
            mid - step
        }
    })
}

/// The runs placed so far, horizontal and vertical apart, each sorted
/// by its fixed coordinate so a check visits only the runs near it (a
/// big diagram places hundreds).
#[derive(Default)]
struct Runs {
    h: Vec<Run>,
    v: Vec<Run>,
}

impl Runs {
    fn push(&mut self, r: Run) {
        let list = if r.vertical { &mut self.v } else { &mut self.h };
        let at = list.partition_point(|q| q.c <= r.c);
        list.insert(at, r);
    }

    /// The runs along y (`vertical`) or x with `lo < c < hi`.
    fn within(&self, vertical: bool, lo: f64, hi: f64) -> &[Run] {
        let list = if vertical { &self.v } else { &self.h };
        let a = list.partition_point(|q| q.c <= lo);
        let b = list.partition_point(|q| q.c < hi).max(a);
        &list[a..b]
    }

    fn all(&self) -> impl Iterator<Item = &Run> {
        self.h.iter().chain(&self.v)
    }
}

impl Run {
    /// Its two ends.
    fn ends(&self) -> ((f64, f64), (f64, f64)) {
        if self.vertical {
            ((self.c, self.l), (self.c, self.h))
        } else {
            ((self.l, self.c), (self.h, self.c))
        }
    }

    /// The run from `a` to `b` (an axis-aligned stretch).
    fn of(a: (f64, f64), b: (f64, f64)) -> Run {
        if (a.0 - b.0).abs() < 0.5 {
            Run {
                c: a.0,
                l: a.1.min(b.1),
                h: a.1.max(b.1),
                vertical: true,
                owner: usize::MAX,
            }
        } else {
            Run {
                c: a.1,
                l: a.0.min(b.0),
                h: a.0.max(b.0),
                vertical: false,
                owner: usize::MAX,
            }
        }
    }
}

/// `r` with x / y swapped when `flip`.
fn f_rect(r: Rect, flip: bool) -> Rect {
    if flip {
        (r.1, r.0, r.3, r.2)
    } else {
        r
    }
}

/// The two ends of an edge's drawn curve.
fn edge_ends(e: &SceneEdge) -> ((f64, f64), (f64, f64)) {
    match e.waypoints.as_slice() {
        [a, .., b] => (*a, *b),
        _ => (e.bezier[0], e.bezier[3]),
    }
}

/// The point of polyline `pts` nearest to `p`.
fn nearest_on(pts: &[(f64, f64)], p: (f64, f64)) -> (f64, f64) {
    let mut best = (f64::INFINITY, p);
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = (a.0 + dx * t, a.1 + dy * t);
        let d = (q.0 - p.0).hypot(q.1 - p.1);
        if d < best.0 {
            best = (d, q);
        }
    }
    best.1
}

/// Layout-graph id for note `k`: `note-k`, with a `'` added until no
/// node of `g` has it yet. Flowmaid's parser accepts a class named
/// `note-0` (mermaid reserves `note`), and `Graph::ensure_node` merges
/// by id, so a plain `note-k` would fold the note into that class and
/// leave one size too many for the layout (a panic in `scene_sized` /
/// `route_sized`).
/// The layout node id of lollipop interface `i` (class index):
/// `interface-<i>`, made unique against every class name and node so
/// far — a lollipop is outside the class index and may share its name
/// with a class, so its name cannot be its id.
fn lollipop_id(d: &ClassDiagram, g: &Graph, i: usize) -> String {
    let mut id = format!("interface-{i}");
    while d.class_index(&id).is_some() || g.node_index(&id).is_some() {
        id.push('\'');
    }
    id
}

fn note_id(g: &Graph, k: usize) -> String {
    let mut id = format!("note-{k}");
    while g.node_index(&id).is_some() {
        id.push('\'');
    }
    id
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
/// names are unique (`ensure_class` dedupes), a note's id is made
/// unique against them (`note_id`), `ensure_node` appends in call
/// order, and edges are added in relation order. `assemble` asserts
/// the counts.
fn build(d: &ClassDiagram) -> Built {
    let mut g = Graph::default();
    g.direction = d.direction;
    g.flat_leaves = true;
    g.tight_sources = true;
    let annotated = d
        .relations
        .iter()
        .any(|r| r.label.is_some() || r.from_card.is_some() || r.to_card.is_some());
    if annotated {
        g.rank_gap = Some(RANK_GAP);
        // A short edge's label sits mid-gap; past it, each end holds
        // its glyph and a cardinality from just off the face outward.
        // In LR / RL a cardinality is as long along the gap as it is
        // wide, so the gap grows to hold them clear of the label.
        let horizontal = matches!(d.direction, Direction::LR | Direction::RL);
        let card = |c: &Option<String>| {
            c.as_deref()
                .map_or(0.0, |t| 2.0 + if horizontal { card_w(t) } else { CARD_H })
        };
        g.label_end_room = d
            .relations
            .iter()
            .map(|r| {
                glyph_size(r.kind)
                    .0
                    .max(r.from_kind.map_or(0.0, |k| glyph_size(k).0))
                    .max(card(&r.to_card))
                    .max(card(&r.from_card))
            })
            .collect();
    }
    for (i, c) in d.classes.iter().enumerate() {
        let id = if c.lollipop {
            lollipop_id(d, &g, i)
        } else {
            c.name.clone()
        };
        let at = g.ensure_node(&id, Some(c.display_name()), Some(Shape::Rect));
        debug_assert_eq!(at, i, "class {i} merged into another node");
    }
    let n = d.classes.len();
    for (k, note) in d.notes.iter().enumerate() {
        let i = g.ensure_node(&note_id(&g, k), Some(note.text.clone()), Some(Shape::Rect));
        debug_assert_eq!(i, n + k, "note {k} merged into another node");
    }
    for r in &d.relations {
        // Laid out in SOURCE order (mermaid's id1 → id2), so the left
        // operand ranks first. The glyph end is marked for the label
        // and port stages (a triangle / diamond as wider than an
        // arrowhead); `assemble` restores the scene edge's
        // direction and style.
        let (a, b) = r.source_order();
        let glyph = |k: RelKind| match k {
            RelKind::Inheritance
            | RelKind::Realization
            | RelKind::Composition
            | RelKind::Aggregation
            | RelKind::Lollipop => Some(Marker::Circle),
            RelKind::Association | RelKind::Dependency => Some(Marker::Arrow),
            RelKind::Link => None,
        };
        // A two-way relation is in source order, its `from` glyph first.
        let ends = if r.head_left {
            EdgeEnds {
                start: glyph(r.kind),
                end: None,
            }
        } else {
            EdgeEnds {
                start: r.from_kind.and_then(glyph),
                end: glyph(r.kind),
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

    // The (glyph half width, cardinality room) of the ends each class
    // takes on the face toward earlier ranks and the one toward later
    // ranks (relations rank in source order; a note ranks before its
    // class).
    let horizontal = matches!(d.direction, Direction::LR | Direction::RL);
    let room = |c: &Option<String>, half: f64| {
        c.as_deref().map_or(0.0, |t| {
            card_room(if horizontal { CARD_H } else { card_w(t) }, half)
        })
    };
    let mut ends: Vec<[Vec<(f64, f64)>; 2]> = vec![[Vec::new(), Vec::new()]; n];
    for r in &d.relations {
        let (a, b) = r.source_order();
        if a != b {
            let h = glyph_size(r.kind).1;
            let fh = r.from_kind.map_or(0.0, |k| glyph_size(k).1);
            let (from, to) = ((fh, room(&r.from_card, fh)), (h, room(&r.to_card, h)));
            let (ea, eb) = if r.head_left { (to, from) } else { (from, to) };
            ends[a][1].push(ea);
            ends[b][0].push(eb);
        }
    }
    for note in &d.notes {
        if let Some(ci) = note.class {
            ends[ci][0].push((0.0, 0.0));
        }
    }
    // A fan's jogs nest across the gap beside its hub (`nest_jogs`), a
    // lane for each pair of ends past the middle one: the gap grows to
    // hold them ORTHO_ROOM apart.
    let widest = ends.iter().flatten().map(Vec::len).max().unwrap_or(0);
    let nested = 2.0 * ORTHO_STUB + (widest.div_ceil(2).saturating_sub(1)) as f64 * ORTHO_ROOM;
    if nested > g.rank_gap.unwrap_or(GAP_L) {
        g.rank_gap = Some(nested);
    }
    let mut boxes = Vec::with_capacity(n);
    let mut sizes = Vec::with_capacity(n + d.notes.len());
    for (c, ends) in d.classes.iter().zip(&ends) {
        let (b, mut size) = box_of(c);
        // A face across the flow grows to hold its ends' glyphs apart
        // (`spread`): a box's width in TB / BT, its height in LR / RL
        // (the operations compartment runs to the taller bottom).
        let room = face_room(&ends[0]).max(face_room(&ends[1]));
        if horizontal {
            size.1 = size.1.max(room);
        } else {
            size.0 = size.0.max(room);
        }
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
    if c.lollipop {
        // Just the name: its own width and a line's height, padded
        // so the circle outside the rect clears the text.
        let name = c.display_name();
        let size = (
            text_width(&name) + 2.0 * LOLLIPOP_PAD,
            LINE_H + 2.0 * LOLLIPOP_PAD,
        );
        let b = ClassBox {
            name,
            annotations: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            lollipop: true,
        };
        return (b, size);
    }
    let fields: Vec<ClassRow> = c.fields.iter().map(|m| row_of(m, false)).collect();
    let methods: Vec<ClassRow> = c.methods.iter().map(|m| row_of(m, true)).collect();
    let annotations: Vec<String> = c.annotations.iter().map(|a| format!("«{a}»")).collect();
    let mut member_w = 0.0f64;
    for r in fields.iter().chain(&methods) {
        member_w = member_w.max(text_width(&r.text));
    }
    let name = c.display_name();
    let mut w = (text_width(&name) + 28.0)
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
        lollipop: false,
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
    // Generics convert per part (name, parameters, return type), as
    // mermaid's `getDisplayDetails` does: a `~` in one never pairs
    // with one in another.
    let body = match (text.find('('), text.rfind(')')) {
        (Some(open), Some(close)) if method && open < close => {
            let mut ret = text[close + 1..].trim();
            if let Some(c) = ret.chars().next().filter(|c| matches!(c, '$' | '*')) {
                classify(c);
                ret = ret[1..].trim_start();
            }
            let sig = format!(
                "{}({})",
                generics_to_angles(text[..open].trim_end()),
                generics_to_angles(text[open + 1..close].trim())
            );
            if ret.is_empty() {
                sig
            } else {
                format!("{sig} : {}", generics_to_angles(ret))
            }
        }
        _ => generics_to_angles(text),
    };
    ClassRow {
        text: format!("{}{}", m.visibility.glyph(), body),
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
        let dash = if j >= cs.rels.len() {
            " stroke-dasharray=\"2 3\""
        } else if matches!(e.kind, EdgeKind::Dotted) {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        // REL_W / NOTE_LINK_W, as `assemble` set them for picking.
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>\n",
            edge_path_d(e, |p| p),
            EDGE_COLOR,
            edge_stroke_width(e),
            dash
        ));
    }

    // Class boxes — each gets a stable accent color, counted over the
    // boxes alone (a lollipop takes none, so adding one recolours no
    // class) — and lollipop interfaces' names.
    let mut accent = 0;
    for (n, b) in sc.nodes.iter().zip(&cs.boxes) {
        if b.lollipop {
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 fill=\"{}\">{}</text>\n",
                n.x,
                n.y,
                TEXT_COLOR,
                escape(&b.name)
            ));
        } else {
            write_box(&mut s, n, b, crate::style::accent(accent));
            accent += 1;
        }
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
    for (rel, spots) in cs.rels.iter().zip(place_cards(sc, &cs.rels, &heads)) {
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
    if let Some((c, r)) = h.circle {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#ffffff\" stroke=\"{}\" \
             stroke-width=\"1.6\"/>\n",
            c.0, c.1, r, EDGE_COLOR
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
/// Step farther out across the line for a cardinality whose spots
/// beside it are all taken.
const CARD_FAR: f64 = 12.0;
/// Weight of a cardinality covering a box, a label chip or another
/// cardinality, against touching a stroke: a chip is painted over it
/// and hides it, where a stroke only grazes it.
const CARD_HIDDEN: usize = 4;

/// Length of an end glyph along its edge, and its half width.
fn glyph_size(kind: RelKind) -> (f64, f64) {
    match kind {
        RelKind::Inheritance | RelKind::Realization => (14.0, 8.0),
        RelKind::Composition | RelKind::Aggregation => (18.0, 6.5),
        RelKind::Association | RelKind::Dependency => (12.0, 7.0),
        RelKind::Lollipop => (2.0 * LOLLIPOP_R, LOLLIPOP_R),
        RelKind::Link => (0.0, 0.0),
    }
}

/// Radius of a lollipop's circle ([`RelKind::Lollipop`]).
const LOLLIPOP_R: f64 = 6.0;
/// Room around a lollipop interface's name, inside its node's rect.
const LOLLIPOP_PAD: f64 = 6.0;

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
fn end_axis(
    e: &SceneEdge,
    at_start: bool,
    reach: f64,
    node: Option<&SceneNode>,
) -> ((f64, f64), (f64, f64)) {
    let (tip, toward) = edge_end_chord(e, at_start, reach.max(1.0));
    let (dx, dy) = (toward.0 - tip.0, toward.1 - tip.1);
    let len = dx.hypot(dy);
    let Some(n) = node.map(|n| face_normal(n, tip)) else {
        return (
            tip,
            if len > 1e-9 {
                (dx / len, dy / len)
            } else {
                (0.0, -1.0)
            },
        );
    };
    if len <= 1e-9 {
        return (tip, n);
    }
    let u = (dx / len, dy / len);
    let max = HEAD_MAX_TILT_DEG.to_radians();
    if u.0 * n.0 + u.1 * n.1 >= max.cos() {
        return (tip, u);
    }
    let side = if n.0 * u.1 - n.1 * u.0 >= 0.0 {
        1.0
    } else {
        -1.0
    };
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
    end_head(&cs.scene, &idx, &cs.scene.edges[j], cs.rels[j].kind, false)
}

/// The glyph at the `from` end of relationship `j` of `cs` when it is
/// two-way ([`RelStyle::from_kind`]), exactly as [`to_svg`] draws it:
/// as [`rel_head`] places the `to` one, at the start of the drawn
/// curve on the `from` class. `None` for a one-ended relationship.
pub fn rel_tail(cs: &ClassScene, j: usize) -> Option<Head> {
    let idx = node_index(&cs.scene);
    let e = &cs.scene.edges[j];
    cs.rels[j]
        .from_kind
        .map(|k| end_head(&cs.scene, &idx, e, k, true))
}

/// The `kind` glyph at one end of `e` (its start when `at_start`),
/// pointing along the drawn curve's last stretch there.
fn end_head(
    sc: &Scene,
    idx: &HashMap<&str, usize>,
    e: &SceneEdge,
    kind: RelKind,
    at_start: bool,
) -> Head {
    let id = if at_start { &e.from } else { &e.to };
    let node = idx.get(id.as_str()).map(|&i| &sc.nodes[i]);
    let (tip, u) = end_axis(e, at_start, glyph_size(kind).0, node);
    head(tip, (tip.0 + u.0, tip.1 + u.1), kind)
}

/// Every end glyph of `rels` on `sc`: each relationship's `to` glyph,
/// after its `from` one when it is two-way.
fn heads_of(sc: &Scene, rels: &[RelStyle]) -> Vec<Head> {
    let idx = node_index(sc);
    let mut out = Vec::with_capacity(rels.len());
    for (e, rel) in sc.edges.iter().zip(rels) {
        if let Some(k) = rel.from_kind {
            out.push(end_head(sc, &idx, e, k, true));
        }
        out.push(end_head(sc, &idx, e, rel.kind, false));
    }
    out
}

/// Centres of the cardinalities of every relationship of `cs` as
/// `[from_card, to_card]` (`None` where there is none), exactly where
/// [`to_svg`] draws them (before its canvas clamp and any shift for
/// nodes dragged past the origin), for 11 px text.
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
/// toward the nearer corner for a line leaving straight — the side
/// [`face_rooms`] keeps room on). A spot that would touch any stroke,
/// glyph, box, label or earlier cardinality gives way to the next
/// nearest the end: the other side, farther along the line, farther out
/// across it; failing all, the spot with the fewest collisions stands —
/// covering a box, label or card counts [`CARD_HIDDEN`] times a stroke,
/// a glyph twice.
fn place_cards(sc: &Scene, rels: &[RelStyle], heads: &[Head]) -> Vec<[Option<(f64, f64)>; 2]> {
    let idx = node_index(sc);
    // Every drawn line with its bounding box, so a spot far from a
    // line skips its segments.
    let lines: Vec<(Rect, Vec<(f64, f64)>)> = sc
        .edges
        .iter()
        .map(|e| {
            let pts = edge_polyline(e);
            let bb = pts.iter().fold(
                (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ),
                |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)),
            );
            (bb, pts)
        })
        .collect();
    // Boxes and label chips: a card on one is hidden (chips are
    // painted over the cards) or unreadable (member text).
    let mut solid: Vec<Rect> = sc
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
        // A circle as the square around it: a card clear of that is
        // clear of the circle.
        if let Some(((x, y), r)) = h.circle {
            let c = [
                (x - r, y - r),
                (x + r, y - r),
                (x + r, y + r),
                (x - r, y + r),
            ];
            glyph_segs.extend((0..4).map(|i| (c[i], c[(i + 1) % 4])));
        }
    }
    let mut placed: Vec<Rect> = Vec::new();
    let mut out = vec![[None, None]; rels.len()];
    for (j, (e, rel)) in sc.edges.iter().zip(rels).enumerate() {
        for (slot, text, at_start) in [(0, &rel.from_card, true), (1, &rel.to_card, false)] {
            let Some(text) = text else { continue };
            let id = if at_start { &e.from } else { &e.to };
            let node = idx.get(id.as_str()).map(|&i| &sc.nodes[i]);
            let (glen, ghalf) = if at_start {
                rel.from_kind.map_or((0.0, 0.0), glyph_size)
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
            // (weighted collisions, centre, box) of the best spot so far.
            let mut best: Option<(usize, (f64, f64), Rect)> = None;
            // Beside the line, past the glyph (within its length) or
            // the stroke, at a few depths along it and farther out
            // across it (past a label chip on the line near the end):
            // the spots nearest the end first, the first side first.
            let mut spots: Vec<(f64, f64)> = [0.0, CARD_FAR, 2.0 * CARD_FAR]
                .into_iter()
                .flat_map(|x| {
                    [0.0, 10.0, 20.0, 34.0]
                        .into_iter()
                        .flat_map(move |d| [(x, d, first), (x, d, -first)])
                })
                .map(|(extra, depth, side)| {
                    let dn = 2.0 + depth + ext_n / 2.0;
                    // Where the line is at that depth.
                    let line_t = ut / un * dn;
                    let half = if dn - ext_n / 2.0 < glen { ghalf } else { 0.0 };
                    let dt = line_t + side * (half + CARD_CLEAR + 1.0 + extra + ext_t / 2.0);
                    (tip.0 + n.0 * dn + t.0 * dt, tip.1 + n.1 * dn + t.1 * dt)
                })
                .collect();
            let far = |c: &(f64, f64)| (c.0 - tip.0).hypot(c.1 - tip.1);
            spots.sort_by(|a, b| far(a).total_cmp(&far(b)));
            'search: for c in spots {
                let r = (c.0 - w / 2.0, c.1 - h / 2.0, c.0 + w / 2.0, c.1 + h / 2.0);
                let grown = (
                    r.0 - CARD_CLEAR,
                    r.1 - CARD_CLEAR,
                    r.2 + CARD_CLEAR,
                    r.3 + CARD_CLEAR,
                );
                let hits = CARD_HIDDEN
                    * (solid.iter().filter(|&&b| rects_meet(r, b)).count()
                        + placed.iter().filter(|&&b| rects_meet(grown, b)).count())
                    + lines
                        .iter()
                        .filter(|(bb, l)| {
                            grown.0 <= bb.2
                                && bb.0 <= grown.2
                                && grown.1 <= bb.3
                                && bb.1 <= grown.3
                                && l.windows(2).any(|s| rect_hits_segment(grown, s[0], s[1]))
                        })
                        .count()
                    + 2 * glyph_segs
                        .iter()
                        .filter(|&&(a, b)| rect_hits_segment(grown, a, b))
                        .count();
                if best.map_or(true, |b| hits < b.0) {
                    best = Some((hits, c, r));
                }
                if hits == 0 {
                    break 'search;
                }
            }
            if let Some((_, c, r)) = best {
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
            ..Head::default()
        },
        // Diamond at the aggregate / composite.
        RelKind::Composition | RelKind::Aggregation => Head {
            polygon: vec![e, p(9.0, -6.5), p(18.0, 0.0), p(9.0, 6.5)],
            filled: matches!(kind, RelKind::Composition),
            ..Head::default()
        },
        // Open V arrow — association & dependency.
        RelKind::Association | RelKind::Dependency => Head {
            segments: vec![[p(12.0, -7.0), e], [p(12.0, 7.0), e]],
            ..Head::default()
        },
        // Lollipop: a circle touching the border.
        RelKind::Lollipop => Head {
            circle: Some((p(LOLLIPOP_R, 0.0), LOLLIPOP_R)),
            ..Head::default()
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
        assert!(svg.contains("+move() : void"));
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

    /// Report P1: `direction` and `note` lines were parsed and thrown
    /// away. Both are kept now: a `note for` attaches to its class
    /// (declared before or after the note), and one for a class the
    /// diagram never declares stays free — it creates no class.
    #[test]
    fn direction_and_note_lines_parse() {
        let d = cd("classDiagram\ndirection LR\nnote for A \"hi\"\nnote \"x\"\nA <|-- B");
        assert_eq!(d.classes.len(), 2);
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.direction, Direction::LR);
        assert_eq!(d.notes.len(), 2);
        assert_eq!(
            (d.notes[0].text.as_str(), d.notes[0].class),
            ("hi", Some(0))
        );
        assert_eq!((d.notes[1].text.as_str(), d.notes[1].class), ("x", None));
        // `\n` and `<br/>` break note lines, as in mermaid's own docs.
        let d = cd("classDiagram\nnote for Duck \"can fly\\ncan swim<br/>can dive\"\nclass Duck");
        assert_eq!(d.notes[0].text, "can fly\ncan swim\ncan dive");
        assert_eq!(d.notes[0].class, Some(0), "declared after its note");
        // As in mermaid: a note without text, or `note for` without a
        // class name, is an error (both used to be skipped silently).
        for bad in ["note for \"x\"", "note"] {
            assert!(
                parse_document(&format!("classDiagram\n{bad}\nA <|-- B")).is_err(),
                "{bad}"
            );
        }
        // Also as in mermaid: an unknown direction is ignored, and a
        // note for a class the diagram never declares stays free (it
        // does not create that class).
        let d = cd("classDiagram\ndirection XY\nnote for Ghost \"boo\"\nA <|-- B");
        assert_eq!(d.direction, Direction::TD);
        assert_eq!(d.classes.len(), 2);
        assert_eq!(d.notes[0].class, None);
    }

    /// A diagram of notes alone draws them (mermaid does); the CLI used
    /// to stop with "no classes defined".
    #[test]
    fn notes_alone_draw() {
        let d = cd("classDiagram\nnote \"just a note\"\nnote \"and another\"");
        assert!(d.classes.is_empty());
        let cs = scene(&d);
        assert_eq!((cs.scene.nodes.len(), cs.boxes.len()), (2, 0));
        let svg = to_svg(&cs);
        assert!(svg.contains(">just a note<") && svg.contains(">and another<"));
        crate::scene::assert_drawn_inside_canvas(&svg);
        let laid: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        assert!(to_svg(&route(&d, &laid)).contains(">just a note<"));
        assert!(to_svg(&route(&d, &[])).contains(">and another<"));
    }

    #[test]
    fn no_space_operators_parse() {
        let d = cd("classDiagram\nAnimal<|--Dog");
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.classes[d.relations[0].to].name, "Animal");
        assert_eq!(d.classes[d.relations[0].from].name, "Dog");
    }

    /// Relation ends as mermaid 12.2.0 names them: a hyphen glues into
    /// a name (`Animal-Name`, `A-`, `-B`), but `--` / `..` are always
    /// operators, scanned from the left.
    #[test]
    fn hyphenated_names_in_relations() {
        let ends = |line: &str| -> Vec<(String, String)> {
            let d = cd(&format!("classDiagram\n{line}"));
            let n = |i: usize| d.classes[i].name.clone();
            d.relations.iter().map(|r| (n(r.from), n(r.to))).collect()
        };
        let pair = |a: &str, b: &str| vec![(a.to_string(), b.to_string())];
        for (line, from, to) in [
            ("Animal-Name --> Dog", "Animal-Name", "Dog"),
            ("A-B --> C-D", "A-B", "C-D"),
            ("A-B-->C", "A-B", "C"),
            ("A-B--C", "A-B", "C"),
            ("A-B-C ..> D", "A-B-C", "D"),
            ("A --> B", "A", "B"),
            ("A--B", "A", "B"),
            ("A-- B", "A", "B"),
            ("A- --> B", "A-", "B"),
            ("A- -- B", "A-", "B"),
            ("A-->-B", "A", "-B"),
            ("A---B", "A", "-B"),
            ("-A --> B", "-A", "B"),
            ("A-B--|>C", "A-B", "C"),
            ("A-B--oC", "A-B", "oC"),
            ("my_o--B", "my_o", "B"),
        ] {
            assert_eq!(ends(line), pair(from, to), "{line}");
        }
        // Head on the left: `to` is the decorated (left) end.
        assert_eq!(ends("A<|--B-C"), pair("B-C", "A"));
        assert_eq!(ends("A-o--B"), pair("B", "A-"), "a hyphen is a boundary");
        let d = cd("classDiagram\nA-B \"1\" --> \"*\" C-D : owns");
        let r = &d.relations[0];
        assert_eq!(
            (r.from_card.as_deref(), r.to_card.as_deref()),
            (Some("1"), Some("*"))
        );
        assert_eq!(r.label.as_deref(), Some("owns"));
        // One class per spelling: declaration, inline member, block,
        // relation and note all name the same `Animal-Name`.
        let d = cd("classDiagram\n\
            class Animal-Name\n\
            Animal-Name : +age int\n\
            class Animal-Name {\n+eat()\n}\n\
            Animal-Name --> Dog\n\
            note for Animal-Name \"x\"\n\
            note for note-0 \"y\"\n\
            class note-0");
        assert_eq!(d.classes.len(), 3);
        let c = &d.classes[0];
        assert_eq!(
            (c.name.as_str(), c.fields.len(), c.methods.len()),
            ("Animal-Name", 1, 1)
        );
        assert_eq!((d.notes[0].class, d.notes[1].class), (Some(0), Some(2)));
        // A dangling operator still has no class on its far side.
        for bad in ["A--->B", "A --> ", "A-B -->"] {
            assert!(
                parse_document(&format!("classDiagram\n{bad}")).is_err(),
                "{bad}"
            );
        }
    }

    /// Backtick names are mermaid's literal class names: the text
    /// between the backticks is the class, so it may hold spaces,
    /// colons, quotes or operators.
    #[test]
    fn backtick_names() {
        let d = cd("classDiagram\n\
            class `Foo Bar`\n\
            `Foo Bar` --> Baz\n\
            `Foo Bar` : +x int\n\
            `Foo--Bar` \"1\" --> `Baz` : has\n\
            `Foo:Bar` <|-- `Foo \"q\"`\n\
            class `Gen Box`~T~\n\
            class `A, B`, C\n\
            <<interface>> `Foo Bar`\n\
            note for `Foo Bar` \"hi\"");
        let names: Vec<&str> = d.classes.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Foo Bar",
                "Baz",
                "Foo--Bar",
                "Foo:Bar",
                "Foo \"q\"",
                "Gen Box",
                "A, B",
                "C"
            ]
        );
        let c = &d.classes[0];
        assert_eq!((c.fields.len(), c.annotations.len()), (1, 1));
        assert_eq!(d.classes[5].display_name(), "Gen Box<T>");
        assert_eq!(d.notes[0].class, Some(0));
        let n = |i: usize| d.classes[i].name.as_str();
        let rels: Vec<(&str, &str)> = d.relations.iter().map(|r| (n(r.from), n(r.to))).collect();
        assert_eq!(
            rels,
            [
                ("Foo Bar", "Baz"),
                ("Foo--Bar", "Baz"),
                ("Foo \"q\"", "Foo:Bar")
            ]
        );
        assert_eq!(d.relations[1].from_card.as_deref(), Some("1"));
        assert_eq!(d.relations[1].label.as_deref(), Some("has"));
        let d = cd("classDiagram\nclass `Foo Bar` {\n+x int\n}\n`Foo Bar` --> B");
        assert_eq!((d.classes.len(), d.classes[0].fields.len()), (2, 1));
        // Mermaid rejects these too.
        for bad in ["`a`b --> C", "`` --> B", "`Foo --> B"] {
            assert!(
                parse_document(&format!("classDiagram\n{bad}")).is_err(),
                "{bad}"
            );
        }
        // Drawn under its own name, without backticks; the hyphenated
        // name too.
        let svg = crate::render_svg("classDiagram\n`Foo Bar` --> Animal-Name").unwrap();
        assert!(
            svg.contains(">Foo Bar</text>") && svg.contains(">Animal-Name</text>"),
            "{svg}"
        );
        assert!(!svg.contains('`'));
        crate::scene::assert_drawn_inside_canvas(&svg);
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

    /// The report's reroute repro (`cls_probe/t1.mmd`): an inheritance
    /// tree with a cycle through it, so the layout has back edges and
    /// the router reroutes some edges round boxes.
    const ZOO: &str = "classDiagram
    Animal <|-- Mammal
    Animal <|-- Bird
    Mammal <|-- Dog
    Mammal <|-- Cat
    Bird <|-- Eagle
    Dog <|-- Puppy
    Zoo \"1\" o-- \"*\" Animal : houses
    Zoo --> Puppy : adopts
    Keeper --> Zoo : works at
    Keeper --> Eagle : feeds
    Puppy ..> Keeper : likes";

    fn rect(n: &SceneNode) -> Rect {
        (
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.x + n.w / 2.0,
            n.y + n.h / 2.0,
        )
    }

    fn node<'a>(cs: &'a ClassScene, id: &str) -> &'a SceneNode {
        cs.scene.nodes.iter().find(|n| n.id == id).unwrap()
    }

    /// Report P0 #5 / M13: the parser normalises `A <|-- B` to B → A
    /// (glyph at `to`), and the layout ranked by that, so parents,
    /// interfaces and wholes sat BELOW their children, with a false
    /// cycle, a giant 'charges' arc and a crossing. Ranks follow source
    /// order now (mermaid's id1 → id2), in every direction.
    #[test]
    fn relations_rank_in_source_order_parents_first() {
        let d = cd(include_str!("../examples/class.mmd"));
        let cs = scene(&d);
        for r in &d.relations {
            let (l, rt) = r.source_order();
            let (a, b) = (&cs.scene.nodes[l], &cs.scene.nodes[rt]);
            assert!(
                a.y + a.h / 2.0 < b.y - b.h / 2.0,
                "{} must sit above {}",
                a.id,
                b.id
            );
        }
        for (parent, child) in [
            ("Person", "Customer"),
            ("PaymentMethod", "CreditCard"),
            ("PaymentMethod", "PayPal"),
            ("Order", "LineItem"),
            ("Customer", "PaymentMethod"),
        ] {
            assert!(
                node(&cs, parent).y < node(&cs, child).y,
                "{parent} / {child}"
            );
        }
        // Each direction lays the left operand first along its flow.
        let at = |dir: &str| {
            let cs = scene(&cd(&format!(
                "classDiagram\ndirection {dir}\nA <|-- B\nB <|-- C"
            )));
            let p: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            (p, cs.scene.width, cs.scene.height)
        };
        let (p, w, h) = at("LR");
        assert!(p[0].0 < p[1].0 && p[1].0 < p[2].0 && w > h, "{p:?}");
        let (p, w, h) = at("RL");
        assert!(p[0].0 > p[1].0 && p[1].0 > p[2].0 && w > h, "{p:?}");
        let (p, ..) = at("BT");
        assert!(p[0].1 > p[1].1 && p[1].1 > p[2].1, "{p:?}");
        let (p, ..) = at("TB");
        assert!(p[0].1 < p[1].1 && p[1].1 < p[2].1, "{p:?}");
    }

    /// The scene edge stays `from` → `to` with the glyph at its END
    /// (the contract hosts paint by), whichever operand the source
    /// wrote first; the triangle sits on the parent's border and
    /// points into it — up into a parent written first (above), down
    /// into one written second (below, as mermaid ranks `--|>`).
    #[test]
    fn glyph_sits_at_the_decorated_end() {
        for (src, parent_above) in [
            ("classDiagram\nAnimal <|-- Dog", true),
            ("classDiagram\nDog --|> Animal", false),
        ] {
            let cs = scene(&cd(src));
            let e = &cs.scene.edges[0];
            assert_eq!((e.from.as_str(), e.to.as_str()), ("Dog", "Animal"), "{src}");
            let animal = rect(node(&cs, "Animal"));
            let face = if parent_above { animal.3 } else { animal.1 };
            let tip = *e.waypoints.last().unwrap_or(&e.bezier[3]);
            assert!((tip.1 - face).abs() < 1e-6, "{src}: {tip:?} {animal:?}");
            let h = rel_head(&cs, 0);
            assert_eq!(h.polygon[0], tip, "{src}");
            let away = if parent_above { 1.0 } else { -1.0 };
            for p in &h.polygon[1..] {
                assert!((p.1 - tip.1) * away > 13.0, "{src}: {:?}", h.polygon);
            }
        }
    }

    /// Report P0 #6 / #7 / Q17: class.rs stroked `bezier` alone, so an
    /// edge the router rerouted (route in `waypoints`, degenerate
    /// chord in `bezier`) ran straight through boxes and its triangle
    /// collapsed to a zero-size polygon (repro: 4 edges through boxes,
    /// 3 triangles gone); a glyph following a grazing tangent lay half
    /// inside its box, under the box.
    #[test]
    fn edges_go_round_boxes_and_glyphs_stay_whole_and_outside() {
        for src in [ZOO, include_str!("../examples/class.mmd")] {
            let d = cd(src);
            let cs = scene(&d);
            let rects: Vec<Rect> = cs.scene.nodes.iter().map(rect).collect();
            for (j, e) in cs.scene.edges.iter().enumerate() {
                let ends = [&e.from, &e.to];
                let line = edge_polyline(e);
                for (k, n) in cs.scene.nodes.iter().enumerate() {
                    let r = rects[k];
                    let r = if ends.contains(&&n.id) {
                        (r.0 + 1.0, r.1 + 1.0, r.2 - 1.0, r.3 - 1.0)
                    } else {
                        r
                    };
                    assert!(
                        !line.windows(2).any(|s| rect_hits_segment(r, s[0], s[1])),
                        "edge {j} {}->{} crosses {}",
                        e.from,
                        e.to,
                        n.id
                    );
                }
                if j >= cs.rels.len() {
                    continue;
                }
                let h = rel_head(&cs, j);
                let to = rect(node(&cs, &e.to));
                let inner = (to.0 + 1.0, to.1 + 1.0, to.2 - 1.0, to.3 - 1.0);
                let mut pts = h.polygon.clone();
                pts.extend(h.segments.iter().flatten());
                for p in pts {
                    assert!(
                        !(inner.0 < p.0 && p.0 < inner.2 && inner.1 < p.1 && p.1 < inner.3),
                        "glyph of {}->{} inside its box: {p:?}",
                        e.from,
                        e.to
                    );
                }
                if h.polygon.len() >= 3 {
                    let p = &h.polygon;
                    let area: f64 = (0..p.len())
                        .map(|i| {
                            let (a, b) = (p[i], p[(i + 1) % p.len()]);
                            a.0 * b.1 - b.0 * a.1
                        })
                        .sum::<f64>()
                        .abs()
                        / 2.0;
                    assert!(area > 50.0, "{}->{}: area {area}", e.from, e.to);
                }
            }
            // Glyphs paint over the boxes they touch.
            let svg = to_svg(&cs);
            let last_box = svg.rfind("rx=\"4\" fill=\"none\" stroke=").unwrap();
            assert!(svg.find("<polygon").unwrap() > last_box);
        }
    }

    /// Report P0 #6: the 'contains' label sat 18.5px off its drawn
    /// line, which ran the plain cubic while the label sat on the
    /// routed channel. Every label sits on the line as drawn.
    #[test]
    fn labels_sit_on_their_drawn_line() {
        // Three edges between one pair: rerouting brings their labels
        // together, and they must still not cover each other.
        let parallel = "classDiagram\nA --> B : one\nA ..> B : two\nB --> A : back\n\
                        A \"1\" *-- \"many\" C\nC o-- D";
        for (src, on_line) in [
            (ZOO, true),
            (include_str!("../examples/class.mmd"), true),
            (parallel, false),
        ] {
            let cs = scene(&cd(src));
            let mut boxes: Vec<(String, Rect)> = Vec::new();
            for e in &cs.scene.edges {
                let Some((text, m, w)) = &e.label else {
                    continue;
                };
                let line = edge_polyline(e);
                let on = nearest_on(&line, *m);
                let off = (on.0 - m.0).hypot(on.1 - m.1);
                // On its line, or — crowded — just beside it, as the
                // label stage places any label it must move.
                let reach = if on_line {
                    1.5
                } else {
                    w / 2.0 + crate::labels::LABEL_DRIFT_MAX
                };
                assert!(off < reach, "{text}: {off:.1}px off its line");
                let r = (m.0 - w / 2.0, m.1 - 10.0, m.0 + w / 2.0, m.1 + 10.0);
                boxes.push((text.clone(), r));
            }
            for (k, (a, ra)) in boxes.iter().enumerate() {
                for (b, rb) in &boxes[k + 1..] {
                    assert!(!rects_meet(*ra, *rb), "labels {a} and {b} overlap");
                }
            }
        }
    }

    #[test]
    fn showcase_edges_are_orthogonal_and_square_to_their_faces() {
        let cs = scene(&cd(include_str!("../examples/class.mmd")));
        for e in &cs.scene.edges {
            // A straight one is a straight cubic; the rest are routes.
            let pts = if e.waypoints.is_empty() {
                let q = e.bezier;
                for c in [q[1], q[2]] {
                    let cross =
                        (q[3].0 - q[0].0) * (c.1 - q[0].1) - (q[3].1 - q[0].1) * (c.0 - q[0].0);
                    assert!(cross.abs() < 1e-6, "{}->{}: {q:?}", e.from, e.to);
                }
                vec![q[0], q[3]]
            } else {
                e.waypoints.clone()
            };
            for w in pts.windows(2) {
                let (dx, dy) = ((w[0].0 - w[1].0).abs(), (w[0].1 - w[1].1).abs());
                assert!(dx < 0.5 || dy < 0.5, "{}->{}: {w:?}", e.from, e.to);
            }
            for (id, p, q) in [
                (&e.from, pts[0], pts[1]),
                (&e.to, pts[pts.len() - 1], pts[pts.len() - 2]),
            ] {
                let n = node(&cs, id);
                let nv = face_normal(n, p);
                let d = ((q.0 - p.0) * nv.0 + (q.1 - p.1) * nv.1) / (q.0 - p.0).hypot(q.1 - p.1);
                assert!(d > 0.999, "{}->{} meets {id} askew", e.from, e.to);
                let r = rect(n);
                let along = if nv.0 == 0.0 {
                    (p.0 - r.0).min(r.2 - p.0)
                } else {
                    (p.1 - r.1).min(r.3 - p.1)
                };
                assert!(
                    along >= ORTHO_INSET - 1e-6,
                    "{}->{} at a corner of {id}",
                    e.from,
                    e.to
                );
            }
        }
    }

    /// `src` laid out in direction `dir` (its own `direction` line, if
    /// any, replaced).
    fn with_direction(src: &str, dir: &str) -> String {
        let mut out = String::new();
        for line in src.lines() {
            if line.trim_start().starts_with("direction ") {
                continue;
            }
            out.push_str(line);
            out.push('\n');
            if line.trim_start().starts_with("classDiagram") {
                out.push_str(&format!("direction {dir}\n"));
            }
        }
        out
    }

    /// Report M13 / "cardinality collisions": multiplicities sat on the
    /// strokes and arrowheads (fixed offsets along the tangent). Each
    /// one now clears every stroke, glyph, box, label and other
    /// cardinality — in every direction: in LR / RL the rank gap runs
    /// along x, where the label chip, the glyph and the cards share
    /// one short edge (verifier repros: the showcase in LR hid '1..*'
    /// under 'contains', x1m hid 'many' under 'holds').
    #[test]
    fn cardinalities_clear_strokes_glyphs_boxes_and_each_other() {
        let big = "classDiagram
    User \"1\" --> \"*\" Order : places
    Order \"1\" *-- \"1..*\" OrderLine
    OrderLine \"*\" --> \"1\" Product
    Customer \"0..1\" o-- \"many\" Order : holds
    Product \"1\" <|-- \"1\" Gift";
        let x1m = "classDiagram
    class Shape
    <<interface>> Shape
    class Box~T~ {
        +List~T~ items
        +size() int
    }
    class Map~K, V~
    Shape <|.. Box
    Box \"1\" *-- \"many\" Map : holds
    Shape <|-- Circle
    note for Box \"generic container\"";
        let srcs = [include_str!("../examples/class.mmd"), big, ZOO, x1m];
        for src in srcs
            .iter()
            .flat_map(|s| ["TB", "BT", "LR", "RL"].map(|dir| with_direction(s, dir)))
        {
            let src = src.as_str();
            let cs = scene(&cd(src));
            let heads: Vec<Head> = (0..cs.rels.len()).map(|j| rel_head(&cs, j)).collect();
            let mut boxes: Vec<Rect> = Vec::new();
            for (rel, spots) in cs.rels.iter().zip(card_spots(&cs)) {
                for (text, at) in [(&rel.from_card, spots[0]), (&rel.to_card, spots[1])] {
                    let (Some(t), Some(c)) = (text, at) else {
                        assert!(text.is_none(), "a cardinality without a spot");
                        continue;
                    };
                    let w = card_w(t);
                    boxes.push((
                        c.0 - w / 2.0,
                        c.1 - CARD_H / 2.0,
                        c.0 + w / 2.0,
                        c.1 + CARD_H / 2.0,
                    ));
                }
            }
            for (i, &b) in boxes.iter().enumerate() {
                for &o in &boxes[i + 1..] {
                    assert!(!rects_meet(b, o), "cards {b:?} {o:?} overlap\n{src}");
                }
                for n in &cs.scene.nodes {
                    assert!(!rects_meet(b, rect(n)), "card {b:?} on {}\n{src}", n.id);
                }
                for e in &cs.scene.edges {
                    let line = edge_polyline(e);
                    assert!(
                        !line.windows(2).any(|s| rect_hits_segment(b, s[0], s[1])),
                        "card {b:?} on {}->{}\n{src}",
                        e.from,
                        e.to
                    );
                    if let Some((t, m, w)) = &e.label {
                        let l = (m.0 - w / 2.0, m.1 - 10.0, m.0 + w / 2.0, m.1 + 10.0);
                        assert!(!rects_meet(b, l), "card {b:?} on label {t}\n{src}");
                    }
                }
                for h in &heads {
                    let k = h.polygon.len();
                    for s in 0..k {
                        let (p, q) = (h.polygon[s], h.polygon[(s + 1) % k]);
                        assert!(!rect_hits_segment(b, p, q), "card {b:?} on a glyph\n{src}");
                    }
                    for s in &h.segments {
                        assert!(
                            !rect_hits_segment(b, s[0], s[1]),
                            "card {b:?} on a head\n{src}"
                        );
                    }
                }
            }
        }
    }

    /// Verifier finding: ends sharing a face spread only when the even
    /// step reached 16 px — a triangle's width — and long or routed
    /// edges' ends never moved, so fan-ins drew glyphs touching or
    /// overlapping (six onto a 96 px box 14 px apart; x3's RL Vehicle
    /// five 16.3 px apart; x4's Logger arrows 12 px apart). Ends now
    /// spread by their glyphs' widths, routed ends slide their final
    /// leg, and a box whose face must hold more grows along it.
    #[test]
    fn end_glyphs_on_one_face_stay_apart() {
        let fan6 = "classDiagram\nP <|-- A\nP <|-- B\nP <|-- C\nP <|-- D\nP <|-- E\nP <|-- F";
        let rl_fan = "classDiagram
    class Vehicle {
        <<abstract>>
        +String vin
        +drive()* void
    }
    Vehicle <|-- Car
    Vehicle <|-- Truck
    Vehicle <|-- Bike
    Vehicle <|-- Bus
    Vehicle <|-- Van
    Garage \"1\" *-- \"1..*\" Bay : has
    Garage \"1\" o-- \"*\" Vehicle : stores
    Owner \"1\" --> \"*\" Vehicle : owns";
        let logger = "classDiagram
    Controller --> Service : calls
    Controller --> Validator : validates
    Service --> Repository : reads
    Service --> Cache : caches
    Service ..> Logger : logs
    Repository --> Database : queries
    Cache ..> Logger : logs
    Repository ..> Logger : logs
    Validator ..> Logger
    Controller ..> Logger";
        let cycle = "classDiagram
    A \"1\" --> \"0..*\" B : owns
    B \"1\" --> \"1\" C : uses
    C ..> A : calls back
    A <|-- D
    A <|-- E
    A <|-- F
    D o-- E : agg";
        let mut cases: Vec<String> = Vec::new();
        for src in [fan6, rl_fan, logger] {
            cases.extend(["TB", "BT", "LR", "RL"].map(|dir| with_direction(src, dir)));
        }
        cases.extend(["TB", "BT"].map(|dir| with_direction(cycle, dir)));
        for src in &cases {
            let cs = scene(&cd(src));
            let boxes: Vec<(String, Rect)> = (0..cs.rels.len())
                .map(|j| {
                    let h = rel_head(&cs, j);
                    let pts: Vec<(f64, f64)> = h
                        .polygon
                        .iter()
                        .chain(h.segments.iter().flatten())
                        .copied()
                        .collect();
                    let bb = pts.iter().fold(
                        (
                            f64::INFINITY,
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::NEG_INFINITY,
                        ),
                        |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)),
                    );
                    (cs.scene.edges[j].to.clone(), bb)
                })
                .filter(|(_, bb)| bb.0.is_finite())
                .collect();
            for (k, (ta, a)) in boxes.iter().enumerate() {
                for (tb, b) in &boxes[k + 1..] {
                    // Stroked 1.6 wide: a gap under that and they touch.
                    let gap = (a.0.max(b.0) - a.2.min(b.2)).max(a.1.max(b.1) - a.3.min(b.3));
                    assert!(gap >= 1.6, "glyphs on {ta} / {tb} {gap:.1} px apart\n{src}");
                }
            }
        }
    }

    /// Report P0 #8: `note for X "…"` and `note "…"` were dropped. They
    /// draw as yellow cards — nodes after the classes, so hosts pick
    /// and drag them like boxes — an attached one linked to its class
    /// by a dotted connector after the relationships.
    #[test]
    fn notes_draw_as_cards_linked_to_their_class() {
        let d = cd("classDiagram
    note \"From Duck till Zebra\"
    Animal <|-- Duck
    note for Duck \"can fly\\ncan swim\"
    Animal <|-- Zebra");
        let cs = scene(&d);
        assert_eq!(cs.scene.nodes.len(), d.classes.len() + 2);
        assert_eq!(cs.scene.edges.len(), d.relations.len() + 1);
        assert_eq!(cs.notes.len(), 2);
        assert_eq!((cs.notes[0].class, cs.notes[0].edge), (None, None));
        let duck = d.class_index("Duck").unwrap();
        assert_eq!(cs.notes[1].class, Some(duck));
        let ei = cs.notes[1].edge.unwrap();
        let conn = &cs.scene.edges[ei];
        let card = &cs.scene.nodes[cs.boxes.len() + 1];
        assert_eq!(
            (conn.from.as_str(), conn.to.as_str()),
            (card.id.as_str(), "Duck")
        );
        assert_eq!(conn.kind, EdgeKind::DottedOpen);
        // The card sits before its class along the flow.
        assert!(card.y < cs.scene.nodes[duck].y);
        let svg = to_svg(&cs);
        let note = crate::style::NOTE_STYLE;
        assert_eq!(svg.matches(&format!("fill=\"{}\"", note.fill)).count(), 2);
        for text in ["From Duck till Zebra", ">can fly<", ">can swim<"] {
            assert!(svg.contains(text), "{text}");
        }
        assert!(svg.contains("stroke-dasharray=\"2 3\""), "dotted connector");
        // Every card is hit-testable where it is drawn.
        for k in 0..2 {
            let n = &cs.scene.nodes[cs.boxes.len() + k];
            assert_eq!(cs.scene.node_at(n.x, n.y), Some(cs.boxes.len() + k));
        }
    }

    /// `route` takes class centres alone (notes then sit beside their
    /// class) or class + note centres (notes stay where given).
    #[test]
    fn route_places_notes_or_keeps_them() {
        let d = cd("classDiagram\nnote for B \"hi\"\nnote \"free\"\nA <|-- B");
        let laid = scene(&d);
        let all: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut moved = all.clone();
        moved[2] = (500.0, 40.0);
        let cs = route(&d, &moved);
        assert_eq!((cs.scene.nodes[2].x, cs.scene.nodes[2].y), (500.0, 40.0));
        let cs = route(&d, &all[..2]);
        let b = &cs.scene.nodes[d.class_index("B").unwrap()];
        let card = &cs.scene.nodes[2];
        assert!(card.y + card.h / 2.0 <= b.y - b.h / 2.0 - NOTE_GAP + 1e-6);
        assert_eq!(card.x, b.x);
        crate::scene::assert_drawn_inside_canvas(&to_svg(&cs));
    }

    #[test]
    #[should_panic(expected = "number of positions")]
    fn route_rejects_a_wrong_position_count() {
        let d = cd("classDiagram\nnote \"x\"\nA <|-- B");
        route(&d, &[(0.0, 0.0)]);
    }

    /// Report P0 #8 / P1: `<<interface>>` was skipped in a block, and a
    /// standalone `<<interface>> Name` line was a parse error. Both
    /// set the stereotype, drawn as «interface» above the name in a
    /// taller name compartment.
    #[test]
    fn stereotype_draws_above_the_name() {
        for src in [
            "classDiagram\nclass Shape {\n<<interface>>\n+area() double\n}",
            "classDiagram\nclass Shape\n<<interface>> Shape\nShape : +area() double",
            "classDiagram\n<<interface>> Shape\nShape : +area() double",
        ] {
            let d = cd(src);
            assert_eq!(d.classes.len(), 1, "{src}");
            assert_eq!(d.classes[0].annotations, ["interface"], "{src}");
            assert_eq!(d.classes[0].methods.len(), 1, "{src}");
            let cs = scene(&d);
            let b = &cs.boxes[0];
            assert_eq!(b.annotations, ["«interface»"]);
            assert_eq!(b.header_h(), HEADER_H + ANNOTATION_H);
            let svg = to_svg(&cs);
            let y_of = |needle: &str| {
                let line = svg.lines().find(|l| l.contains(needle)).unwrap();
                svg_attr(line, "y")
            };
            assert!(y_of(">«interface»<") < y_of(">Shape<"), "{src}");
        }
    }

    /// Report P0 #8 / "return type format": mermaid shows a method's
    /// return type after `" : "`, generics `~T~` as `<T>`, and a `$` /
    /// `*` classifier as underline / italics, not as text.
    #[test]
    fn member_rows_read_as_mermaid_shows_them() {
        let d = cd("classDiagram\nclass Box {\n\
            +placeOrder(cart) Order\n\
            +List~int~ items\n\
            +get()$ T\n\
            +area()* double\n\
            +make() Box~T~$\n\
            String count$\n\
            -reset()\n\
            +map(List~int~ a, Set~str~ b) Map~K, V~\n}");
        let cs = scene(&d);
        let rows: Vec<(&str, bool, bool)> = cs.boxes[0]
            .fields
            .iter()
            .chain(&cs.boxes[0].methods)
            .map(|r| (r.text.as_str(), r.is_static, r.is_abstract))
            .collect();
        assert_eq!(
            rows,
            [
                ("+List<int> items", false, false),
                ("String count", true, false),
                ("+placeOrder(cart) : Order", false, false),
                ("+get() : T", true, false),
                ("+area() : double", false, true),
                ("+make() : Box<T>", true, false),
                ("-reset()", false, false),
                ("+map(List<int> a, Set<str> b) : Map<K, V>", false, false),
            ]
        );
        let svg = to_svg(&cs);
        assert!(svg.contains("text-decoration=\"underline\">+get() : T<"));
        assert!(svg.contains("font-style=\"italic\">+area() : double<"));
    }

    #[test]
    fn generics_convert_by_mermaids_rule() {
        for (src, want) in [
            ("List~int~", "List<int>"),
            ("List~List~int~~", "List<List<int>>"),
            ("Map~K, V~", "Map<K, V>"),
            ("List~int~ a, Set~str~ b", "List<int> a, Set<str> b"),
            ("a~b", "a~b"),
            ("plain", "plain"),
        ] {
            assert_eq!(generics_to_angles(src), want, "{src}");
        }
    }

    /// Report P1: `class Box~T~` named a class `Box~T~`, so a later
    /// `Box : +size() int` made a second box. The generic is held apart
    /// and shown as `Box<T>`; relation operands may carry it too.
    #[test]
    fn generic_class_is_one_class() {
        let d =
            cd("classDiagram\nclass Box~T~ {\n+T item\n}\nBox : +size() int\nBox~T~ <|-- Crate");
        assert_eq!(d.classes.len(), 2);
        let b = &d.classes[0];
        assert_eq!((b.name.as_str(), b.generic.as_deref()), ("Box", Some("T")));
        assert_eq!((b.fields.len(), b.methods.len()), (1, 1));
        assert_eq!(b.display_name(), "Box<T>");
        assert_eq!(d.relations[0].to, 0);
        let svg = to_svg(&scene(&d));
        assert!(svg.contains(">Box&lt;T&gt;</text>"));
        // A comma inside a generic does not split a declaration list.
        let d = cd("classDiagram\nclass Map~K, V~, Set~T~");
        let names: Vec<String> = d.classes.iter().map(Class::display_name).collect();
        assert_eq!(names, ["Map<K, V>", "Set<T>"]);
    }

    /// Report "faint compartments" / "header notch" / "tight rows":
    /// the dividers were pale hairlines that broke the border, empty
    /// compartments 8px slivers, the name band rounded at the bottom
    /// too, rows flush with the dividers.
    #[test]
    fn compartments_read_clearly() {
        let cs = scene(&cd("classDiagram\nclass A {\n+x\n}"));
        let n = &cs.scene.nodes[0];
        let accent = crate::style::accent(0);
        let svg = to_svg(&cs);
        let dividers: Vec<&str> = svg.lines().filter(|l| l.starts_with("<line")).collect();
        assert_eq!(dividers.len(), 2);
        for l in &dividers {
            assert!(l.contains(&format!("stroke=\"{accent}\"")), "{l}");
        }
        // Name band: rounded top corners, square bottom ones.
        let (x0, y0) = (n.x - n.w / 2.0, n.y - n.h / 2.0);
        let band = svg
            .lines()
            .find(|l| l.starts_with("<path") && l.contains(accent))
            .unwrap();
        assert!(band.starts_with(&format!("<path d=\"M {:.1} {:.1} L", x0, y0 + HEADER_H)));
        assert_eq!(band.matches(" Q ").count(), 2, "{band}");
        // The border is stroked last, over the dividers.
        let border = svg
            .rfind(&format!("fill=\"none\" stroke=\"{accent}\""))
            .unwrap();
        assert!(border > svg.rfind("<line").unwrap());
        // Padded rows, a real empty band.
        assert_eq!(n.h, HEADER_H + ROW_H + 2.0 * COMP_PAD + EMPTY_H);
        let row = svg.lines().find(|l| l.contains(">+x<")).unwrap();
        assert_eq!(svg_attr(row, "y"), y0 + HEADER_H + COMP_PAD + ROW_H / 2.0);
    }

    /// [`rel_head`] / [`card_spots`] are what the SVG draws, so a GUI
    /// painter matches the export.
    #[test]
    fn rel_head_and_card_spots_match_the_svg() {
        let cs = scene(&cd(include_str!("../examples/class.mmd")));
        let svg = to_svg(&cs);
        for j in 0..cs.rels.len() {
            let h = rel_head(&cs, j);
            if !h.polygon.is_empty() {
                let pts: Vec<String> = h
                    .polygon
                    .iter()
                    .map(|(x, y)| format!("{x:.1},{y:.1}"))
                    .collect();
                assert!(
                    svg.contains(&format!("points=\"{}\"", pts.join(" "))),
                    "{j}"
                );
            }
            for s in &h.segments {
                let d = format!(
                    "d=\"M {:.1} {:.1} L {:.1} {:.1}\"",
                    s[0].0, s[0].1, s[1].0, s[1].1
                );
                assert!(svg.contains(&d), "{j}: {d}");
            }
        }
        for spots in card_spots(&cs) {
            for c in spots.into_iter().flatten() {
                assert!(
                    svg.contains(&format!("x=\"{:.1}\" y=\"{:.1}\"", c.0, c.1)),
                    "{c:?}"
                );
            }
        }
    }

    /// Drags keep the same rules: the scene a `route` returns is
    /// orthogonal where it can be and its edges stay off the boxes.
    #[test]
    fn dragged_scene_routes_round_boxes() {
        let d = cd(include_str!("../examples/class.mmd"));
        let laid = scene(&d);
        let mut pos: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let customer = d.class_index("Customer").unwrap();
        pos[customer].0 += 260.0;
        pos[customer].1 += 40.0;
        let cs = route(&d, &pos);
        for e in &cs.scene.edges {
            let line = edge_polyline(e);
            for n in &cs.scene.nodes {
                if n.id == e.from || n.id == e.to {
                    continue;
                }
                assert!(
                    !line
                        .windows(2)
                        .any(|s| rect_hits_segment(rect(n), s[0], s[1])),
                    "{}->{} crosses {}",
                    e.from,
                    e.to,
                    n.id
                );
            }
        }
        crate::scene::assert_drawn_inside_canvas(&to_svg(&cs));
    }

    /// The verifier's 16-class e-commerce model (`x6_realistic`, TB):
    /// six edges leave User's bottom face into one gap.
    const SHOP: &str = "classDiagram
    class User {
        +String id
        +String email
        +login() bool
    }
    class Admin
    class Guest
    class Session {
        +String token
        +Date expires
    }
    class Cart {
        +add(Item i) void
        +total() Money
    }
    class Item
    class Product
    class Category
    class Review
    class Address
    class Payment
    class Invoice
    class Shipment
    class Warehouse
    class Supplier
    User <|-- Admin
    User <|-- Guest
    User \"1\" --> \"*\" Session : opens
    User \"1\" *-- \"1\" Cart : owns
    Cart \"1\" o-- \"*\" Item : holds
    Item \"*\" --> \"1\" Product : of
    Product \"*\" --> \"1\" Category : in
    User \"1\" --> \"*\" Review : writes
    Review \"*\" --> \"1\" Product : about
    User \"1\" o-- \"*\" Address : lives at
    Cart ..> Payment : pays with
    Payment --> Invoice : produces
    Invoice --> Shipment : triggers
    Shipment --> Warehouse : from
    Warehouse \"*\" o-- \"*\" Product : stocks
    Supplier --> Product : supplies
    note for Cart \"one per user\"";

    /// Maximal straight axis-aligned stretches of an edge as drawn:
    /// (vertical, fixed coordinate, from, to).
    fn drawn_runs(e: &SceneEdge) -> Vec<(bool, f64, f64, f64)> {
        let pts = edge_polyline(e);
        let mut out = Vec::new();
        let mut i = 0;
        while i + 1 < pts.len() {
            let vertical = (pts[i].0 - pts[i + 1].0).abs() < 0.5;
            let horizontal = (pts[i].1 - pts[i + 1].1).abs() < 0.5;
            if !vertical && !horizontal {
                i += 1;
                continue;
            }
            let mut k = i + 1;
            while k + 1 < pts.len()
                && if vertical {
                    (pts[k + 1].0 - pts[i].0).abs() < 0.5
                } else {
                    (pts[k + 1].1 - pts[i].1).abs() < 0.5
                }
            {
                k += 1;
            }
            let (a, b) = (pts[i], pts[k]);
            let run = if vertical {
                (true, a.0, a.1.min(b.1), a.1.max(b.1))
            } else {
                (false, a.1, a.0.min(b.0), a.0.max(b.0))
            };
            if run.3 - run.2 > 4.0 {
                out.push(run);
            }
            i = k;
        }
        out
    }

    /// Pairs of edge indexes.
    type Pairs = Vec<(usize, usize)>;

    /// Pairs of edges whose drawn lines run along each other (closer
    /// than ORTHO_SEP over more than 4 px, less a pixel: the spline
    /// through a long run's rounded corners bulges by a fraction of
    /// one), and pairs that cross.
    fn merged_and_crossing(cs: &ClassScene) -> (Pairs, Pairs) {
        let runs: Vec<_> = cs.scene.edges.iter().map(drawn_runs).collect();
        let (mut merged, mut crossing) = (Vec::new(), Vec::new());
        for i in 0..runs.len() {
            for j in i + 1..runs.len() {
                for &(va, ca, la, ha) in &runs[i] {
                    for &(vb, cb, lb, hb) in &runs[j] {
                        if va == vb {
                            if (ca - cb).abs() < ORTHO_SEP - 1.0 && ha.min(hb) - la.max(lb) > 4.0 {
                                merged.push((i, j));
                            }
                        } else if la + 1.0 < cb && cb < ha - 1.0 && lb + 1.0 < ca && ca < hb - 1.0 {
                            crossing.push((i, j));
                        }
                    }
                }
            }
        }
        (merged, crossing)
    }

    /// Verifier repro (x6_realistic): two relationships out of User
    /// (`writes`, `lives at`) were drawn on one line for ~150 px, and
    /// Admin's leg crossed Guest's (inverted jog heights): the
    /// orthogonalizer took a merged run when it was the cheapest, and
    /// priced lanes against labels still on their old curves. A merged
    /// run is now refused, labels count once their edge is final, and
    /// jogs sharing a gap stack in nesting order — in every direction
    /// (r3: `writes` and `lives at` still crossed twice in LR / RL).
    #[test]
    fn routes_never_merge_and_fans_nest_without_crossings() {
        let d = cd(SHOP);
        for dir in ["TB", "BT", "LR", "RL"] {
            let d = cd(&with_direction(SHOP, dir));
            let cs = scene(&d);
            let (merged, crossing) = merged_and_crossing(&cs);
            assert!(merged.is_empty(), "{dir}: merged runs: {merged:?}");
            let user = d.class_index("User").unwrap();
            let fan: Vec<usize> = d
                .relations
                .iter()
                .enumerate()
                .filter(|(_, r)| r.from == user || r.to == user)
                .map(|(j, _)| j)
                .collect();
            assert_eq!(fan.len(), 6);
            for &(i, j) in &crossing {
                assert!(
                    !(fan.contains(&i) && fan.contains(&j)),
                    "{dir}: User's edges {i} and {j} cross"
                );
            }
            // Every one of them is orthogonal (none fell back to a curve).
            for &j in &fan {
                let e = &cs.scene.edges[j];
                let pts = if e.waypoints.is_empty() {
                    vec![e.bezier[0], e.bezier[3]]
                } else {
                    e.waypoints.clone()
                };
                let (a, b) = (pts[0], pts[pts.len() - 1]);
                assert!(
                    (a.0 - b.0).abs() < 0.5 || (a.1 - b.1).abs() < 0.5 || !e.waypoints.is_empty(),
                    "{dir}: {}->{} kept its curve",
                    e.from,
                    e.to
                );
            }
        }
        let cs = scene(&d);
        // Dragged: swapping two classes, or one far off, merges nothing,
        // and a box in a rerouted edge's way is gone round, not through
        // — save a box pushed flush against an end box (the swap lands
        // User, wider than Admin's slot, flush with Guest): no square
        // exit fits between the two, so a line from that face crosses it
        // whatever the route.
        let laid: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let n = d.classes.len();
        let mut swapped = laid[..n].to_vec();
        swapped.swap(0, 1);
        let mut far = laid[..n].to_vec();
        far[n / 2] = (-300.0, -200.0);
        for (k, centers) in [swapped, far].into_iter().enumerate() {
            let cs = route(&d, &centers);
            let (merged, _) = merged_and_crossing(&cs);
            assert!(merged.is_empty(), "drag {k}: merged runs: {merged:?}");
            for e in &cs.scene.edges {
                let line = edge_polyline(e);
                let ends: Vec<Rect> = cs
                    .scene
                    .nodes
                    .iter()
                    .filter(|b| b.id == e.from || b.id == e.to)
                    .map(rect)
                    .collect();
                let flush = |b: Rect| {
                    let g = ORTHO_STUB;
                    ends.iter()
                        .any(|&r| rects_meet((r.0 - g, r.1 - g, r.2 + g, r.3 + g), b))
                };
                if cs
                    .scene
                    .nodes
                    .iter()
                    .any(|b| b.id != e.from && b.id != e.to && flush(rect(b)))
                {
                    continue;
                }
                for b in &cs.scene.nodes {
                    if b.id == e.from || b.id == e.to {
                        continue;
                    }
                    let r = rect(b);
                    let r = (r.0 + 1.0, r.1 + 1.0, r.2 - 1.0, r.3 - 1.0);
                    assert!(
                        !line.windows(2).any(|s| rect_hits_segment(r, s[0], s[1])),
                        "drag {k}: {}->{} through {}",
                        e.from,
                        e.to,
                        b.id
                    );
                }
            }
        }
    }

    /// Five services into one Logger, each end with a cardinality
    /// (verifier r3 repro f4).
    const FAN_IN_CARDS: &str = "classDiagram
    class Logger {
      +log(msg) void
      +warn(msg) void
      +error(msg) void
    }
    ServiceA \"a1\" --> \"A2\" Logger
    ServiceB \"b1\" --> \"B2\" Logger
    ServiceC \"c1\" ..> \"C2\" Logger
    ServiceD \"d1\" o-- \"D2\" Logger
    ServiceE \"e1\" *-- \"E2\" Logger
    Logger <|-- FileLogger
    Logger \"L1\" --> \"S2\" Sink";

    /// Distance between two points.
    fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    /// Verifier repro (r3, f4 and its LR twin): the ends on Logger's
    /// face spread by their glyphs alone, 22 px apart, so a card beside
    /// its glyph lay on the next relation's line ('C2' struck by
    /// ServiceD's) and read as that line's. Ends sharing a face now keep
    /// room for each card on the side it takes, and cards try the spots
    /// nearest their end first: each clears every line and sits nearer
    /// its own end than any other.
    #[test]
    fn crowded_fan_in_cards_clear_the_other_lines() {
        for dir in ["TB", "BT", "LR", "RL"] {
            let cs = scene(&cd(&with_direction(FAN_IN_CARDS, dir)));
            let ends: Vec<[(f64, f64); 2]> = cs
                .scene
                .edges
                .iter()
                .take(cs.rels.len())
                .map(|e| {
                    let p = edge_polyline(e);
                    [p[0], p[p.len() - 1]]
                })
                .collect();
            for (j, (rel, spots)) in cs.rels.iter().zip(card_spots(&cs)).enumerate() {
                for (slot, text) in [(0, &rel.from_card), (1, &rel.to_card)] {
                    let (Some(t), Some(c)) = (text, spots[slot]) else {
                        continue;
                    };
                    let w = card_w(t);
                    let b = (
                        c.0 - w / 2.0,
                        c.1 - CARD_H / 2.0,
                        c.0 + w / 2.0,
                        c.1 + CARD_H / 2.0,
                    );
                    for e in &cs.scene.edges {
                        let line = edge_polyline(e);
                        assert!(
                            !line.windows(2).any(|s| rect_hits_segment(b, s[0], s[1])),
                            "{dir}: '{t}' on {}->{}",
                            e.from,
                            e.to
                        );
                    }
                    let own = dist(c, ends[j][slot]);
                    for (k, other) in ends.iter().enumerate() {
                        for (kk, &p) in other.iter().enumerate() {
                            if (k, kk) != (j, slot) {
                                assert!(
                                    own <= dist(c, p) + 6.0,
                                    "{dir}: '{t}' {own:.0} px from its end, {:.0} from \
                                     edge {k}'s",
                                    dist(c, p)
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Five labelled relations into one Logger (verifier r3 repro f5).
    const FAN_IN_LABELS: &str = "classDiagram
    class Logger {
      +log(msg) void
      +warn(msg) void
      +error(msg) void
    }
    ServiceA --> Logger : logs to
    ServiceB --> Logger : logs to
    ServiceC ..> Logger : uses
    ServiceD o-- Logger : shares
    ServiceE *-- Logger : owns
    Logger <|-- FileLogger : is a
    Logger --> Sink : writes";

    /// Verifier repros (r3): a fan's jogs were drawn one at a time, the
    /// shortest first and each nearest mid gap, so the inner ones took
    /// the middle and an outer one crossed its neighbour twice: five
    /// labelled relations into Logger (the chips pushed the inner jogs
    /// down) crossed 4 times; six subclasses, staggered into two rows
    /// whose legs interleaved on the parent's face, 5 times; eight 3
    /// times, two of them entering the parent's sides; a cycle's lane
    /// crossed the next jog in its gap twice. Mermaid draws none of
    /// these. Jogs tied by a nesting order are now drawn again in that
    /// order, a class diagram's leaves stay in one row, and the rank gap
    /// holds a wide fan's nested jogs.
    #[test]
    fn fans_nest_without_crossings() {
        let fan = |k: usize| {
            (0..k).fold(String::from("classDiagram\n"), |s, i| {
                s + &format!("P <|-- C{i}\n")
            })
        };
        let cycle = "classDiagram
    direction BT
    A \"1\" --> \"*\" B : makes
    B \"1\" --> \"*\" C : feeds
    C \"1\" --> \"1\" A : returns to
    C ..> D : depends
    D --|> A : is an
    B <|-- E
    B <|-- F
    E o-- F : peers";
        // Final verifier round: a fan too wide for its parent's face
        // sends legs to the side faces, but the router did not always
        // pick the outermost (`v1_fan10_labels`: Square's leg crossed
        // Circle's; `v7_twelve`: K1's crossed K12's long one).
        let twelve = fan(12) + "C2 --> C8 : talks\nC0 \"1\" --> \"*\" C11 : owns\n";
        for src in [
            fan(6),
            fan(8),
            FAN_IN_LABELS.to_string(),
            FAN_IN_CARDS.to_string(),
            FAN10_LABELS.to_string(),
            twelve,
        ] {
            for dir in ["TB", "BT", "LR", "RL"] {
                let cs = scene(&cd(&with_direction(&src, dir)));
                let (merged, crossing) = merged_and_crossing(&cs);
                assert!(merged.is_empty(), "{dir}: merged {merged:?}\n{src}");
                assert!(crossing.is_empty(), "{dir}: crossing {crossing:?}\n{src}");
            }
        }
        // The cycle's `returns to` (C -> A) must get past B's column,
        // crossing a line above B and one below it (or the two into B):
        // no more — it used to cross `feeds` twice as well.
        let cs = scene(&cd(cycle));
        let (_, crossing) = merged_and_crossing(&cs);
        assert!(crossing.len() <= 2, "{crossing:?}");
        // Eight subclasses: one row, every end on the parent's face.
        let cs = scene(&cd(&fan(8)));
        let p = rect(node(&cs, "P"));
        for k in 0..8 {
            let c = node(&cs, &format!("C{k}"));
            assert!((c.y - node(&cs, "C0").y).abs() < 0.5, "C{k} off the row");
            let (_, end) = edge_ends(&cs.scene.edges[k]);
            assert!((end.1 - p.3).abs() < 0.5, "C{k} enters P at {end:?}");
        }
    }

    /// Segments `a`-`b` and `c`-`d` cross properly.
    fn segments_cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
        let side = |o: (f64, f64), p: (f64, f64), q: (f64, f64)| {
            (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0)
        };
        side(c, d, a) * side(c, d, b) < 0.0 && side(a, b, c) * side(a, b, d) < 0.0
    }

    /// The box round a glyph.
    fn head_box(h: &Head) -> Rect {
        let pts: Vec<(f64, f64)> = h
            .polygon
            .iter()
            .copied()
            .chain(h.segments.iter().flatten().copied())
            .collect();
        pts.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)),
        )
    }

    /// Verifier repro (r3, s5 — a regression from 0.32): the layout
    /// stacks a class's self-loops on one side, each starting where the
    /// one before ends, and the source order turned `o--`'s loop round,
    /// so its diamond and the other loop's arrowhead met on one point
    /// (in LR the two loops wound round each other). Loops now
    /// alternate sides and nest where they share one: no two cross, and
    /// no two glyphs touch, with two loops or four, in every direction.
    #[test]
    fn self_loops_keep_their_glyphs_apart() {
        let two = "classDiagram\nclass Employee\nEmployee --> Employee : mentors\n\
                   Employee o-- Employee : manages";
        let four = "classDiagram
    class Employee {
      +String name
      +String title
      +manage() void
    }
    Employee \"1\" o-- \"*\" Employee : manages
    Employee --> Employee : mentors
    Employee <|-- Employee : refines
    Employee ..> Employee : calls
    Department \"1\" *-- \"*\" Employee : staff";
        for src in [two, four] {
            for dir in ["TB", "BT", "LR", "RL"] {
                let cs = scene(&cd(&with_direction(src, dir)));
                let loops: Vec<usize> = (0..cs.rels.len())
                    .filter(|&j| cs.scene.edges[j].from == cs.scene.edges[j].to)
                    .collect();
                for (i, &a) in loops.iter().enumerate() {
                    for &b in &loops[i + 1..] {
                        let (ha, hb) = (head_box(&rel_head(&cs, a)), head_box(&rel_head(&cs, b)));
                        assert!(!rects_meet(ha, hb), "{dir}: glyphs of {a} and {b} touch");
                        let (pa, pb) = (
                            edge_polyline(&cs.scene.edges[a]),
                            edge_polyline(&cs.scene.edges[b]),
                        );
                        for s in pa.windows(2) {
                            for t in pb.windows(2) {
                                assert!(
                                    !segments_cross(s[0], s[1], t[0], t[1]),
                                    "{dir}: loops {a} and {b} cross"
                                );
                            }
                        }
                    }
                }
                // Their cardinalities sit by their ends.
                for (j, spots) in card_spots(&cs).iter().enumerate() {
                    let p = edge_polyline(&cs.scene.edges[j]);
                    for (slot, end) in [(0, p[0]), (1, p[p.len() - 1])] {
                        if let Some(c) = spots[slot] {
                            assert!(dist(c, end) < 40.0, "{dir}: card of {j} far from its end");
                        }
                    }
                }
            }
        }
    }

    /// Verifier finding (r6, `w3_loops_three_sib` TB / BT): with three
    /// loops nested on one face, the innermost one ('manages', the only
    /// one with cardinalities) had its cards pushed out past both outer
    /// loops, the '1' about 18 px from the dashed 'reviews' loop's end,
    /// reading as that loop's. Carded loops nest outermost now, so each
    /// card sits by its own end, nearer it than any other loop's end.
    /// And a row of loop labels keeps more than the label stage's gap
    /// between chips: closer, the stage took the pair for crowded and
    /// moved one onto the loops (`a14` LR: 'four' over its loop).
    #[test]
    fn loop_cards_sit_by_their_own_end_and_labels_off_the_loops() {
        let w3 = "classDiagram
    class Employee {
      +String name
    }
    Org <|-- Employee
    Org <|-- Team
    Org <|-- Office
    Employee \"1\" o-- \"*\" Employee : manages
    Employee --> Employee : mentors
    Employee ..> Employee : reviews";
        let a14 = "classDiagram
    A \"1\" --> \"*\" A : one
    A \"0..1\" o-- \"many\" A : two
    A *-- A
    A ..> A : four
    A <|-- B
    A <|-- C";
        for src in [w3, a14] {
            for dir in ["TB", "BT", "LR", "RL"] {
                let cs = scene(&cd(&with_direction(src, dir)));
                let loops: Vec<usize> = (0..cs.rels.len())
                    .filter(|&j| cs.scene.edges[j].from == cs.scene.edges[j].to)
                    .collect();
                let ends: Vec<(usize, (f64, f64))> = loops
                    .iter()
                    .flat_map(|&j| {
                        let p = edge_polyline(&cs.scene.edges[j]);
                        [(j, p[0]), (j, p[p.len() - 1])]
                    })
                    .collect();
                for (j, spots) in card_spots(&cs).iter().enumerate() {
                    let p = edge_polyline(&cs.scene.edges[j]);
                    for (slot, end) in [(0, p[0]), (1, p[p.len() - 1])] {
                        let Some(c) = spots[slot] else { continue };
                        let own = dist(c, end);
                        assert!(own < 40.0, "{dir}: card of {j} {own:.0} px from its end");
                        for &(k, e) in ends.iter().filter(|&&(k, _)| k != j) {
                            assert!(
                                own < dist(c, e),
                                "{dir}: card of {j} nearer loop {k}'s end\n{src}"
                            );
                        }
                    }
                }
                for &j in &loops {
                    let Some((text, m, w)) = &cs.scene.edges[j].label else {
                        continue;
                    };
                    let chip = (m.0 - w / 2.0, m.1 - 10.0, m.0 + w / 2.0, m.1 + 10.0);
                    for &k in &loops {
                        let p = edge_polyline(&cs.scene.edges[k]);
                        assert!(
                            !p.windows(2).any(|s| rect_hits_segment(chip, s[0], s[1])),
                            "{dir}: label '{text}' on loop {k}\n{src}"
                        );
                    }
                }
            }
        }
    }

    /// The verifier's 57-class stress graph (`x5_big`).
    const BIG: &str = "classDiagram
    C20 \"1\" --> C9
    C23 \"1\" <|-- \"*\" C37
    C26 *-- C4 : r2
    C7 ..|> C14
    C25 *-- \"*\" C3 : r4
    C9 <|-- C34
    C6 ..> C37
    C4 <|-- C36
    C49 --> \"*\" C20
    C50 \"1\" ..|> C11
    C31 \"1\" o-- C56
    C7 --> C32 : r11
    C26 ..|> C2 : r12
    C52 o-- C20
    C29 \"1\" -- C4 : r14
    C4 ..|> C3
    C52 o-- C28
    C29 *-- \"*\" C22
    C18 ..|> C8 : r18
    C5 \"1\" --> \"*\" C10
    C27 ..> C55 : r20
    C56 \"1\" *-- \"*\" C24 : r21
    C14 \"1\" --> \"*\" C0
    C9 ..> C26 : r23
    C44 ..> C54
    C29 -- C57
    C25 --> C26 : r26
    C12 \"1\" *-- C4
    C6 \"1\" ..> C0 : r28
    C1 -- C4 : r29
    C22 \"1\" o-- C38
    C29 \"1\" --> C30 : r31
    C47 \"1\" --> \"*\" C16
    C33 *-- C23
    C19 -- C41 : r34
    C58 o-- C10
    C21 *-- C40
    C12 *-- \"*\" C51
    C33 o-- C31
    C30 *-- C16
    C59 o-- \"*\" C46
    C14 *-- C30 : r41
    C39 <|-- C53
    C5 ..|> C53 : r43
    C12 *-- \"*\" C30
    C46 --> C25
    C10 <|-- C8 : r46
    C9 -- C39
    C9 \"1\" ..> C35 : r48
    C41 \"1\" ..> C6
    C12 \"1\" -- \"*\" C52 : r50
    C15 ..> C48 : r51
    C3 ..|> C58 : r52
    C57 --> \"*\" C33
    C9 ..> \"*\" C33 : r54
    C0 \"1\" -- C49 : r55
    C7 <|-- C35 : r56
    C50 \"1\" <|-- \"*\" C49
    C2 <|-- C49
    C58 --> C4 : r59
    C12 o-- C44
    C15 ..> \"*\" C44
    C35 \"1\" *-- \"*\" C57
    C28 <|-- \"*\" C20
    C19 \"1\" <|-- C50
    C42 \"1\" *-- C23 : r65
    C47 \"1\" --> C6
    C14 ..|> C10
    C12 o-- C22 : r68
    C29 ..|> C28 : r69
    C32 <|-- C4
    C6 o-- \"*\" C5 : r71
    C48 -- C8
    C16 *-- C25
    C20 o-- C5 : r74
    C4 <|-- \"*\" C17
    C54 \"1\" <|-- \"*\" C14 : r76
    C35 \"1\" o-- C26
    C7 \"1\" o-- C10 : r78
    C19 -- C33 : r79
    C17 \"1\" -- \"*\" C22 : r80
    C46 ..> \"*\" C32
    C28 ..|> C6
    C53 --> \"*\" C56
    C14 *-- C21
    C25 \"1\" <|-- C22
    C56 \"1\" --> C16 : r86
    C55 ..|> C32
    C2 *-- \"*\" C29 : r88
    C21 o-- C35 : r89";

    /// Final verifier round (non-blocking): in `x5_big`, r31's end on
    /// C30's top face sat 6.4 px beside the lane that C30 -> C12 (a
    /// router-made route) keeps down to its jog just above that face
    /// (spread places ends blind to the routes past the face), so r31
    /// found no square exit and kept its curve,
    /// which ran within 3-6 px of r69's jog for 100 px, its label on
    /// the tangency. An end moved just clear of that lane routes it:
    /// every relationship is orthogonal, in every direction, and r31
    /// runs along no other line.
    #[test]
    fn an_end_beside_another_routes_lane_nudges_instead_of_curving() {
        for dir in ["TB", "BT", "LR", "RL"] {
            let cs = scene(&cd(&with_direction(BIG, dir)));
            for (j, e) in cs.scene.edges.iter().enumerate() {
                assert!(
                    e.from == e.to || ortho_corners(e).is_some(),
                    "{dir}: {}->{} (r{j}) kept its curve",
                    e.from,
                    e.to
                );
            }
            // (A router-made route beside C39 still runs 4.7 px from
            // r47 for 15 px: the router's own, kept as routed.)
            let (merged, _) = merged_and_crossing(&cs);
            assert!(
                !merged.iter().any(|&(i, j)| i == 31 || j == 31),
                "{dir}: r31 merges: {merged:?}"
            );
        }
    }

    /// Final verifier round (non-blocking): with siblings close beside
    /// a class, its self-loops (nested on one side, one on the other)
    /// ran into the siblings' boxes, a label over Office's name; and a
    /// lone loop's label sat on the loop's apex, hiding most of the loop
    /// on a small box. Now no loop or loop label meets another box, a
    /// loop label covers no loop and no other loop label, in every
    /// direction.
    #[test]
    fn self_loops_keep_clear_of_their_neighbours() {
        let siblings = "classDiagram\nP <|-- A\nP <|-- B\nP <|-- C\n\
                        B --> B : next\nB o-- B : parent";
        let lone = "classDiagram\nA --> A : parent\nA <|-- B";
        let cards = "classDiagram\nA \"1\" --> \"*\" A : parent";
        let chain = "classDiagram\nOrder --> Order : parent\nUser --> Order : places";
        for src in [LOOPS_SIBLINGS, siblings, lone, cards, chain] {
            for dir in ["TB", "BT", "LR", "RL"] {
                let cs = scene(&cd(&with_direction(src, dir)));
                let sc = &cs.scene;
                let loops: Vec<&SceneEdge> = sc.edges.iter().filter(|e| e.from == e.to).collect();
                assert!(!loops.is_empty());
                for e in &loops {
                    let line = edge_polyline(e);
                    let chip = label_chip(e).expect("labelled");
                    for n in sc.nodes.iter().filter(|n| n.id != e.from) {
                        let r = rect(n);
                        assert!(
                            !line.windows(2).any(|s| rect_hits_segment(r, s[0], s[1])),
                            "{dir}: a loop of {} runs into {}",
                            e.from,
                            n.id
                        );
                        assert!(!rects_meet(chip, r), "{dir}: a loop label covers {}", n.id);
                    }
                    for o in &loops {
                        let other = edge_polyline(o);
                        assert!(
                            !other
                                .windows(2)
                                .any(|s| rect_hits_segment(chip, s[0], s[1])),
                            "{dir}: {:?} covers a loop",
                            e.label
                        );
                        if !std::ptr::eq(*e, *o) {
                            let oc = label_chip(o).expect("labelled");
                            assert!(!rects_meet(chip, oc), "{dir}: loop labels overlap");
                        }
                    }
                }
            }
        }
    }

    /// Verifier finding (r3): every attached note sat on the first rank
    /// — longest-path layering parks every source there — its dotted
    /// connector running 300-400 px to its class. Sources now rank one
    /// layer above their nearest target, as dagre ranks them: a note
    /// sits just above its class, and a class pointing at one deep in
    /// the diagram (SHOP's Supplier) sits next to it.
    #[test]
    fn notes_and_sources_sit_next_to_their_class() {
        let src = "classDiagram
    note for Later \"attached to a class declared after me\"
    class Later {
      +int x
      +run() void
    }
    class Other
    Later <|-- Other : extends
    note for Other \"child note<br/>two lines\"";
        let cs = scene(&cd(src));
        for (k, nb) in cs.notes.iter().enumerate() {
            let note = &cs.scene.nodes[cs.boxes.len() + k];
            let class = &cs.scene.nodes[nb.class.unwrap()];
            let gap = (class.y - class.h / 2.0) - (note.y + note.h / 2.0);
            assert!(
                (0.0..120.0).contains(&gap),
                "note {k} {gap:.0} px above its class"
            );
            assert!(
                (note.x - class.x).abs() < (note.w + class.w) / 2.0 + 40.0,
                "note {k} off to the side"
            );
        }
        let cs = scene(&cd(SHOP));
        let (supplier, product) = (rect(node(&cs, "Supplier")), rect(node(&cs, "Product")));
        let gap = product.1 - supplier.3;
        assert!(
            (0.0..120.0).contains(&gap),
            "Supplier {gap:.0} px above Product"
        );
    }

    /// Verifier finding (r6, drag speed): dragging a node 1e5 px off
    /// left routes that long, a fan's lanes beside each other all the
    /// way, so no label spot was clear and `label_spot` tested every
    /// spot along the whole route against every run, box and label —
    /// most of a 2-3x slower drag. The search keeps within
    /// LABEL_SPOT_REACH of the middle, and tests each spot against what
    /// its stretch can meet, which picks a spot as good as testing
    /// everything.
    #[test]
    fn label_spots_are_sought_near_the_middle_and_tested_locally() {
        let cramped = Cell::new(false);
        let ortho = |runs, rects, labels| Ortho {
            rects,
            labels,
            ia: usize::MAX,
            ib: usize::MAX,
            j: usize::MAX,
            runs,
            room: false,
            cramped: &cramped,
        };
        // A route 1e5 px down from a box, lanes 10 px either side all
        // the way but for a clear stretch far past the middle; the label
        // was on the box.
        let mut lanes = Runs::default();
        for x in [-10.0, 10.0] {
            lanes.push(Run::of((x, 0.0), (x, 90_000.0)));
            lanes.push(Run::of((x, 90_400.0), (x, 100_000.0)));
        }
        let pts = [(0.0, 0.0), (0.0, 100_000.0)];
        let start = [(-40.0, -40.0, 40.0, 0.0)];
        let p = ortho(&lanes, &start, &[]).label_spot(&pts, 30.0, (0.0, 0.0));
        assert!(
            (p.1 - 50_000.0).abs() <= LABEL_SPOT_REACH,
            "label sought {:.0} px from the middle",
            (p.1 - 50_000.0).abs()
        );
        // On a short route with runs, boxes and labels about, the spot
        // chosen meets no more than any other spot tried, counted
        // against everything.
        let mut runs = Runs::default();
        runs.push(Run::of((-200.0, 300.0), (400.0, 300.0)));
        runs.push(Run::of((-200.0, 330.0), (400.0, 330.0)));
        runs.push(Run::of((8.0, 0.0), (8.0, 500.0)));
        runs.push(Run::of((150.0, 380.0), (150.0, 520.0)));
        runs.push(Run::of((-50.0, 140.0), (50.0, 140.0)));
        let rects = [(20.0, 200.0, 90.0, 260.0), (-90.0, 360.0, -12.0, 420.0)];
        let labels = [(-30.0, 90.0, 30.0, 110.0), (100.0, 440.0, 160.0, 460.0)];
        let pts = [(0.0, 0.0), (0.0, 450.0), (300.0, 450.0)];
        let ctx = ortho(&runs, &rects, &labels);
        let all_hits = |p: (f64, f64)| {
            let chip = (p.0 - 16.0, p.1 - 11.0, p.0 + 16.0, p.1 + 11.0);
            runs.all()
                .filter(|q| {
                    let (a, b) = q.ends();
                    rect_hits_segment(chip, a, b)
                })
                .count()
                + rects
                    .iter()
                    .chain(&labels)
                    .filter(|&&r| rects_meet(chip, r))
                    .count()
        };
        // The spots tried: every LABEL_SPOT_STEP px either side of the
        // middle (375 px along), from stub to stub.
        let mid = 375.0;
        let n = ((mid - ORTHO_STUB) / LABEL_SPOT_STEP).floor() as i32;
        for was in [(0.0, 0.0), (0.0, 300.0), (200.0, 450.0)] {
            let p = ctx.label_spot(&pts, 30.0, was);
            for k in -n..=n {
                let d = mid + f64::from(k) * LABEL_SPOT_STEP;
                let q = if d <= 450.0 {
                    (0.0, d)
                } else {
                    (d - 450.0, 450.0)
                };
                assert!(
                    all_hits(p) <= all_hits(q),
                    "{q:?} beats {p:?} (from {was:?})"
                );
            }
        }
    }

    /// Verifier finding (r6, BLOCKING): a class named `note-<k>` (the
    /// parser accepts it, though mermaid reserves `note`) took note k's
    /// layout id, so the note folded into the class and `scene_sized` /
    /// `route_sized` panicked on one size too many — through
    /// `render_svg`, `scene` and `route`. Each note is its own node now,
    /// after the classes, whatever the classes are called.
    #[test]
    fn a_class_named_like_a_note_id_keeps_its_note() {
        for src in [
            "classDiagram\nclass note-0\nnote \"hello\"",
            "classDiagram\nclass Order\nclass note-1\nnote for Order \"first\"\nnote \"second\"",
            "classDiagram\n<<interface>> note-0\nnote \"x\"",
            "classDiagram\nclass A, note-0\nnote \"x\"",
            "classDiagram\nclass note-0~T~\nnote \"x\"",
            "classDiagram\nclass note-0\nclass note-0'\nclass note-1\nA <|-- B\nnote \"a\"\nnote for A \"b\"",
        ] {
            let svg = crate::render_svg(src).unwrap_or_else(|e| panic!("{e:?}\n{src}"));
            assert!(svg.contains("</svg>"), "{src}");
            let d = cd(src);
            let cs = scene(&d);
            let n = cs.boxes.len();
            assert_eq!(n, d.classes.len(), "{src}");
            assert_eq!(cs.scene.nodes.len(), n + d.notes.len(), "{src}");
            for (k, note) in d.notes.iter().enumerate() {
                let node = &cs.scene.nodes[n + k];
                assert_eq!(node.label, note.text, "note {k}\n{src}");
                assert!(d.classes.iter().all(|c| c.name != node.id), "{src}");
            }
            let all: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            for centers in [&all[..], &all[..n]] {
                let r = route(&d, centers);
                assert_eq!(r.scene.nodes.len(), cs.scene.nodes.len(), "{src}");
                assert_eq!(r.scene.edges.len(), cs.scene.edges.len(), "{src}");
                to_svg(&r);
            }
        }
    }

    /// Verifier finding (r3, x4 in LR): a routed edge kept the router's
    /// 14.5 px final stub, about a triangle's length, so the corner sat
    /// under the triangle and tilted it. A routed edge entering its glyph
    /// end on a leg shorter than ORTHO_STUB is routed again.
    #[test]
    fn glyph_ends_enter_on_a_full_stub() {
        let x4 = "classDiagram
    class Service {
        <<service>>
        +handle(Request req) Response
    }
    Controller --> Service : calls
    Controller --> Validator : validates
    Service --> Repository : reads
    Service --> Cache : caches
    Service ..> Logger : logs
    Repository --> Database : queries
    Cache ..> Logger : logs
    Repository ..> Logger : logs
    Database \"1\" *-- \"*\" Table : contains
    Table \"1\" *-- \"*\" Column : has
    Column \"*\" --> \"1\" Type : typed
    Validator ..> Logger
    Controller ..> Logger
    Service \"1\" --> \"*\" Event : emits
    Event <|-- Created
    Event <|-- Deleted
    Event <|-- Updated";
        for dir in ["TB", "BT", "LR", "RL"] {
            let cs = scene(&cd(&with_direction(x4, dir)));
            for (j, rel) in cs.rels.iter().enumerate() {
                let c = corners_of(&cs.scene.edges[j].waypoints);
                let ortho = c
                    .windows(2)
                    .all(|w| (w[0].0 - w[1].0).abs() < 0.5 || (w[0].1 - w[1].1).abs() < 0.5);
                if rel.kind == RelKind::Link || c.len() < 3 || !ortho {
                    continue;
                }
                let leg = dist(c[c.len() - 1], c[c.len() - 2]);
                assert!(
                    leg >= ORTHO_STUB - 0.5,
                    "{dir}: edge {j} enters its glyph on {leg:.1} px"
                );
            }
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

    /// The verifier's inheritance fan of eight subclasses with a note on
    /// the parent: in LR / RL two of its jogs' bounds came out a
    /// rounding error apart in the wrong order, and `nest_group`'s
    /// `f64::clamp` panicked ('min > max') on the static render.
    const FAN8_NOTE: &str = "classDiagram
    class Shape {
      +area() double
    }
    Shape <|-- Circle
    Shape <|-- Square
    Shape <|-- Triangle
    Shape <|-- Hexagon
    Shape <|-- Pentagon
    Shape <|-- Octagon
    Shape <|-- Ellipse
    Shape <|-- Star
    note for Shape \"Base of every shape\"";

    /// Ten labelled subclasses and a note (`v1_fan10_labels`): the
    /// same panic, LR and RL.
    const FAN10_LABELS: &str = "classDiagram
    class Shape {
      <<abstract>>
      +area() double
    }
    Shape <|-- Circle : is a
    Shape <|-- Square : is a
    Shape <|-- Triangle : is a
    Shape <|-- Hexagon : is a
    Shape <|-- Pentagon : is a
    Shape <|-- Octagon : is a
    Shape <|-- Ellipse : is a
    Shape <|-- Star : is a
    Shape <|-- Rhombus : is a
    Shape <|-- Trapezoid : is a
    note for Shape \"Base of every shape\"";

    /// Five plain services into one logger (`f7_three_plain`): dragging
    /// Logger straight up hit the same panic through `route`.
    const FAN_IN_PLAIN: &str = "classDiagram
    class Logger {
      +log(msg) void
    }
    ServiceA --> Logger
    ServiceB --> Logger
    ServiceC --> Logger
    ServiceD --> Logger
    ServiceE --> Logger";

    /// Three self-relations on a class with close siblings
    /// (`w3_loops_three_sib`).
    const LOOPS_SIBLINGS: &str = "classDiagram
    class Employee {
      +String name
    }
    Org <|-- Employee
    Org <|-- Team
    Org <|-- Office
    Employee \"1\" o-- \"*\" Employee : manages
    Employee --> Employee : mentors
    Employee ..> Employee : reviews";

    /// Everything a host paints from `cs` is finite, and the SVG holds
    /// no NaN or infinity.
    fn assert_finite(cs: &ClassScene, what: &str) {
        let ok = |p: (f64, f64)| p.0.is_finite() && p.1.is_finite();
        let sc = &cs.scene;
        assert!(sc.width.is_finite() && sc.height.is_finite(), "{what}");
        for n in &sc.nodes {
            assert!(ok((n.x, n.y)) && ok((n.w, n.h)), "{what}: node {}", n.id);
        }
        for (j, e) in sc.edges.iter().enumerate() {
            assert!(e.bezier.iter().all(|&p| ok(p)), "{what}: edge {j} curve");
            assert!(e.waypoints.iter().all(|&p| ok(p)), "{what}: edge {j} route");
            assert!(
                e.label.as_ref().map_or(true, |l| ok(l.1)),
                "{what}: label {j}"
            );
        }
        for j in 0..cs.rels.len() {
            for h in std::iter::once(rel_head(cs, j)).chain(rel_tail(cs, j)) {
                assert!(h.polygon.iter().all(|&p| ok(p)), "{what}: head {j}");
                assert!(
                    h.circle.map_or(true, |(c, r)| ok(c) && r.is_finite()),
                    "{what}: circle {j}"
                );
            }
        }
        for spots in card_spots(cs) {
            assert!(spots.iter().flatten().all(|&p| ok(p)), "{what}: cards");
        }
        let svg = to_svg(cs);
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{what}");
    }

    /// Final verifier round, BLOCKING: the fans above panicked in
    /// `nest_group` on the static render (LR / RL), so `render_svg`
    /// panicked and the CLI exited 101 with no output.
    #[test]
    fn wide_fans_with_a_note_render_in_every_direction() {
        for src in [FAN8_NOTE, FAN10_LABELS] {
            for dir in ["TB", "BT", "LR", "RL"] {
                let src = with_direction(src, dir);
                let svg = crate::render_svg(&src).unwrap();
                assert!(svg.ends_with("</svg>\n"), "{dir}");
                assert_finite(&scene(&cd(&src)), dir);
            }
        }
    }

    /// Final verifier round, BLOCKING: dragging Logger straight up by
    /// 90-150 px (from its laid-out centre, RL or LR) panicked in
    /// `nest_group` through `route` — a host crashed mid-drag.
    #[test]
    fn dragging_a_fan_in_target_never_panics() {
        for dir in ["RL", "LR"] {
            let d = cd(&with_direction(FAN_IN_PLAIN, dir));
            let laid: Vec<(f64, f64)> = scene(&d).scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            let logger = d.class_index("Logger").unwrap();
            for dy in (0..=200).step_by(2) {
                let mut c = laid.clone();
                c[logger].1 -= f64::from(dy);
                assert_finite(&route(&d, &c), &format!("{dir} up {dy}"));
            }
        }
    }

    /// A drag sweep over every class input the tests hold, in all four
    /// directions: random but deterministic (a fixed xorshift seed)
    /// moves of the kinds a drag produces — a class dropped on another
    /// (coincident), onto part of one (overlapping), past the canvas
    /// origin (negative coordinates), the whole diagram jostled, two
    /// classes swapped, one moved far. No route may panic, and
    /// everything drawn stays finite. Laid out, every one renders too.
    #[test]
    fn drag_sweeps_never_panic_and_stay_finite() {
        let srcs = [
            include_str!("../examples/class.mmd"),
            ZOO,
            SHOP,
            FAN_IN_CARDS,
            FAN_IN_LABELS,
            FAN8_NOTE,
            FAN10_LABELS,
            FAN_IN_PLAIN,
            LOOPS_SIBLINGS,
            TWO_WAY,
        ];
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 10_000) as f64 / 10_000.0
        };
        for src in srcs {
            for dir in ["TB", "BT", "LR", "RL"] {
                let d = cd(&with_direction(src, dir));
                let laid = scene(&d);
                assert_finite(&laid, dir);
                let base: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let n = base.len();
                let mut pick = || (rnd() * n as f64) as usize % n;
                let mut moves: Vec<Vec<(f64, f64)>> = Vec::new();
                for k in 0..24 {
                    let mut c = base.clone();
                    let (i, j) = (pick(), pick());
                    match k % 6 {
                        0 => c[i] = c[j],
                        1 => c[i] = (c[j].0 + 15.0 * (k % 4) as f64, c[j].1 - 9.0),
                        2 => c[i] = (-40.0 - 13.0 * k as f64, -25.0 - 7.0 * k as f64),
                        3 => c.iter_mut().enumerate().for_each(|(m, p)| {
                            p.0 += 31.0 * ((m * 7 + k) % 5) as f64 - 62.0;
                            p.1 += 23.0 * ((m * 3 + k) % 5) as f64 - 46.0;
                        }),
                        4 => c.swap(i, j),
                        _ => c[i] = (c[i].0 + 37.0 * k as f64 - 400.0, c[i].1 - 29.0 * k as f64),
                    }
                    moves.push(c);
                }
                // The classes alone (notes placed beside them).
                moves.push(base[..d.classes.len()].to_vec());
                for (k, c) in moves.iter().enumerate() {
                    assert_finite(&route(&d, c), &format!("{dir} move {k}"));
                }
            }
        }
    }

    /// Every two-way and lollipop spelling mermaid 12.2.0 draws, with
    /// cardinalities, labels, a dotted lollipop, a lollipop sharing a
    /// class's name, lollipops against another glyph and a two-way
    /// self-relation.
    const TWO_WAY: &str = "classDiagram
    Animal <|--|> Zebra
    Zebra *--* Stripe
    Zebra o--o Herd
    Herd <--> Keeper : feeds
    Keeper <..> Zoo
    Zoo <|..|> Place
    Zoo \"1\" *--> \"many\" Animal : houses
    Herd o..|> Group
    IFeed ()-- Keeper
    Zoo --() IVisit
    ITrack ().. Stripe
    Herd ()-- Zoo : joins
    Keeper ()--* Zoo
    Keeper <--> Keeper";

    /// Mermaid's two-way relations (`A <|--|> B`, docs "Two-way
    /// relations") were parse errors: any end glyph on either side of
    /// `--` / `..`, the left one kept as `from_kind`, in source order.
    #[test]
    fn two_way_relations_parse() {
        use RelKind::*;
        for (line, l, r, dashed) in [
            ("Animal <|--|> Zebra", Inheritance, Inheritance, false),
            ("Animal *--* Zebra", Composition, Composition, false),
            ("Animal o--o Zebra", Aggregation, Aggregation, false),
            ("Animal <--> Zebra", Association, Association, false),
            ("Animal <..> Zebra", Dependency, Dependency, true),
            ("Animal <|..|> Zebra", Realization, Realization, true),
            ("Animal *--> Zebra", Composition, Association, false),
            ("Animal o..|> Zebra", Aggregation, Realization, true),
            ("Animal <--* Zebra", Association, Composition, false),
            ("Animal *--o Zebra", Composition, Aggregation, false),
            ("Animal ()--* Zebra", Lollipop, Composition, false),
            ("Animal ()--() Zebra", Lollipop, Lollipop, false),
            ("Animal<|--|>Zebra", Inheritance, Inheritance, false),
        ] {
            let d = cd(&format!("classDiagram\n{line}"));
            assert_eq!(d.classes.len(), 2, "{line}: both ends are classes");
            assert!(d.classes.iter().all(|c| !c.lollipop), "{line}");
            let rel = &d.relations[0];
            let names = (&d.classes[rel.from].name, &d.classes[rel.to].name);
            assert_eq!(names, (&"Animal".into(), &"Zebra".into()), "{line}");
            assert_eq!((rel.from_kind, rel.kind), (Some(l), r), "{line}");
            assert_eq!((rel.dashed, rel.head_left), (dashed, false), "{line}");
        }
        // Cardinalities and a label read as on a one-ended relation.
        let d = cd("classDiagram\nG \"1\" <--> \"many\" H : link");
        let rel = &d.relations[0];
        assert_eq!(rel.from_card.as_deref(), Some("1"));
        assert_eq!(rel.to_card.as_deref(), Some("many"));
        assert_eq!(rel.label.as_deref(), Some("link"));
        // Hyphenated and backtick names keep working beside the glyphs.
        let d = cd("classDiagram\n`Foo Bar` <|--|> my-class\nA-o--o-B\nX-Y*--*Z-W");
        let ends: Vec<(&str, &str)> = d
            .relations
            .iter()
            .map(|r| {
                (
                    d.classes[r.from].name.as_str(),
                    d.classes[r.to].name.as_str(),
                )
            })
            .collect();
        assert_eq!(
            ends,
            [("Foo Bar", "my-class"), ("A-", "-B"), ("X-Y", "Z-W")]
        );
        // A glyph on a dotted line that had no spelling before.
        let d = cd("classDiagram\nWhole *.. Part\nA ..o B");
        let r0 = &d.relations[0];
        assert_eq!(
            (r0.kind, r0.dashed, r0.head_left),
            (Composition, true, true)
        );
        assert_eq!(d.classes[r0.to].name, "Whole");
        assert_eq!(d.relations[1].kind, Aggregation);
        assert_eq!(d.relations[1].from_kind, None);
        // A marker glued to a name is still part of it.
        let d = cd("classDiagram\nA--oC\nFoo-- B");
        assert!(d
            .relations
            .iter()
            .all(|r| r.kind == Link && r.from_kind.is_none()));
        assert_eq!(d.classes[d.relations[0].to].name, "oC");
    }

    /// Mermaid's lollipop interfaces (`bar ()-- foo`, `foo --() bar`)
    /// were parse errors. Against a plain end the `()` operand is an
    /// interface — a new node per relation, outside the class index —
    /// and the circle sits at its end; source order ranks it.
    #[test]
    fn lollipop_interfaces_parse() {
        let d = cd("classDiagram\n\
            bar ()-- foo\n\
            foo --() baz\n\
            bar ()-- qux\n\
            P ().. Q : uses\n\
            class bar");
        let names: Vec<(&str, bool)> = d
            .classes
            .iter()
            .map(|c| (c.name.as_str(), c.lollipop))
            .collect();
        assert_eq!(
            names,
            [
                ("foo", false),
                ("bar", true),
                ("baz", true),
                ("qux", false),
                ("bar", true),
                ("Q", false),
                ("P", true),
                ("bar", false),
            ]
        );
        assert_eq!(d.class_index("bar"), Some(7), "the class, not a lollipop");
        let rels: Vec<(usize, usize, bool)> = d
            .relations
            .iter()
            .map(|r| {
                assert_eq!((r.kind, r.from_kind), (RelKind::Lollipop, None));
                (r.from, r.to, r.head_left)
            })
            .collect();
        assert_eq!(
            rels,
            [(0, 1, true), (0, 2, false), (3, 4, true), (5, 6, true)]
        );
        assert_eq!(d.relations[0].source_order(), (1, 0), "interface first");
        assert!(d.relations[3].dashed);
        assert_eq!(d.relations[3].label.as_deref(), Some("uses"));
        // A lollipop's layout id never takes a class's name.
        let d = cd("classDiagram\nbar ()-- foo\nclass interface-1\nclass bar");
        let cs = scene(&d);
        let ids: Vec<&str> = cs.scene.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["foo", "interface-1'", "interface-1", "bar"]);
    }

    /// Glyphs of two-way and lollipop relations, laid out and dragged:
    /// one at EACH end, its tip on that end's class border, never
    /// inside any box nor under a label chip; a lollipop's circle
    /// touches its interface's rect from outside; the SVG paints the
    /// glyphs after the boxes, and a lollipop as its name alone.
    #[test]
    fn two_way_and_lollipop_glyphs_at_both_ends() {
        for dir in ["TB", "LR", "BT", "RL"] {
            let d = cd(&with_direction(TWO_WAY, dir));
            let laid = scene(&d);
            let mut moved: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            // Zoo (a hub of both kinds) dragged clear of every box.
            let zoo = d.class_index("Zoo").unwrap();
            moved[zoo].0 = laid.scene.width + 160.0;
            moved[zoo].1 += 40.0;
            for (cs, what) in [(laid, "laid"), (route(&d, &moved), "dragged")] {
                let what = format!("{dir} {what}");
                let rects: Vec<Rect> = cs.scene.nodes.iter().map(rect).collect();
                let chips: Vec<Rect> = cs.scene.edges.iter().filter_map(label_chip).collect();
                let inside = |r: Rect, p: (f64, f64)| {
                    r.0 + 1.0 < p.0 && p.0 < r.2 - 1.0 && r.1 + 1.0 < p.1 && p.1 < r.3 - 1.0
                };
                let mut circles = 0;
                for (j, rel) in d.relations.iter().enumerate() {
                    let e = &cs.scene.edges[j];
                    let tail = rel_tail(&cs, j);
                    assert_eq!(tail.is_some(), rel.from_kind.is_some(), "{what} {j}");
                    let mut ends = vec![(rel_head(&cs, j), rel.to, false)];
                    ends.extend(tail.map(|h| (h, rel.from, true)));
                    for (h, ci, at_start) in ends {
                        let (tip, _) = edge_end_chord(e, at_start, 1.0);
                        let r = rects[ci];
                        let on_border = (tip.0 - r.0).abs().min((tip.0 - r.2).abs()) < 0.5
                            && (r.1 - 0.5..=r.3 + 0.5).contains(&tip.1)
                            || (tip.1 - r.1).abs().min((tip.1 - r.3).abs()) < 0.5
                                && (r.0 - 0.5..=r.2 + 0.5).contains(&tip.0);
                        assert!(on_border, "{what} {j}: tip {tip:?} off {r:?}");
                        let mut pts = h.polygon.clone();
                        pts.extend(h.segments.iter().flatten());
                        if let Some((c, rad)) = h.circle {
                            circles += 1;
                            let gap = (c.0 - tip.0).hypot(c.1 - tip.1);
                            assert!((gap - rad).abs() < 1e-6, "{what} {j}: circle off its tip");
                            pts.extend([(c.0, c.1 - rad), (c.0, c.1 + rad)]);
                            pts.extend([(c.0 - rad, c.1), (c.0 + rad, c.1), c]);
                        }
                        assert!(!pts.is_empty(), "{what} {j}: no glyph");
                        for p in pts {
                            for (k, &r) in rects.iter().enumerate() {
                                assert!(!inside(r, p), "{what} {j}: glyph in box {k}: {p:?}");
                            }
                            if what.ends_with("laid") {
                                for &c in &chips {
                                    assert!(!inside(c, p), "{what} {j}: glyph under chip");
                                }
                            }
                        }
                    }
                }
                let svg = to_svg(&cs);
                assert_eq!(svg.matches("<circle").count(), circles, "{what}");
                let boxes = d.classes.iter().filter(|c| !c.lollipop).count();
                let borders = svg.matches("rx=\"4\" fill=\"none\" stroke=").count();
                assert_eq!(borders, boxes, "{what}: a lollipop has no box");
                assert!(svg.contains(">IFeed</text>") && svg.contains(">IVisit</text>"));
                let last_box = svg.rfind("rx=\"4\" fill=\"none\" stroke=").unwrap();
                assert!(svg.find("<polygon").unwrap() > last_box, "{what}");
                assert!(svg.find("<circle").unwrap() > last_box, "{what}");
            }
        }
    }
}
