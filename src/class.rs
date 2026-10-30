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

use std::collections::{BTreeMap, HashMap};

use crate::labels::{avoid_label_overlaps, LABEL_ROUTE_WORK, LABEL_STATIC_WORK};
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
const NOTE_PAD: f64 = 10.0;
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
    assemble(d, b, sc, LABEL_STATIC_WORK)
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
    assemble(d, b, sc, LABEL_ROUTE_WORK)
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
/// style hosts read (`Dotted` = dashed, else `Open`). `budget` is the
/// label stage's work budget (static scene or drag).
fn assemble(d: &ClassDiagram, b: Built, mut scene: Scene, budget: usize) -> ClassScene {
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
    orthogonalize(&mut scene);
    // Rerouting moved labels onto their new lines, where one may now
    // cover another label or sit under a stroke; the glyph ends are
    // still marked, so labels keep off them too.
    avoid_label_overlaps(&scene.nodes, &scene.clusters, &mut scene.edges, budget);
    for (e, r) in scene.edges.iter_mut().zip(&d.relations) {
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

/// Corner radius of the orthogonal class routes (mermaid's class
/// edges round their corners by about this much).
const ORTHO_R: f64 = 7.0;
/// Shortest straight run an orthogonal route leaves or enters a face
/// with — room for the longest end glyph (the 18 px diamond) plus air.
const ORTHO_STUB: f64 = 22.0;
/// Two horizontal (or vertical) runs closer than this that overlap
/// along their length read as one line: the later one steps aside.
const ORTHO_SEP: f64 = 8.0;
/// Cost of an orthogonal stretch lying on another route's run: two
/// edges merged into one line read as the wrong connection.
const ORTHO_OVERLAP: usize = 100;
/// Cost of an orthogonal stretch running under another edge's label
/// (the chip hides it there).
const ORTHO_UNDER_LABEL: usize = 60;
/// Cost of an orthogonal route crossing another.
const ORTHO_CROSSING: usize = 10;
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
/// runs already placed; a route that would cross a box keeps its
/// curve. Edges the router routed (axis-aligned waypoints) are kept as
/// routed; a label moves onto its new line.
fn orthogonalize(sc: &mut Scene) {
    let rects: Vec<Rect> = sc
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
    let idx: HashMap<String, usize> = sc
        .nodes
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
    // along the face, free to move). A long edge's ends stay with its
    // layout channels; a routed edge's with its route.
    let mut faces: BTreeMap<(usize, (i8, i8)), Vec<FaceEnd>> = BTreeMap::new();
    for (j, e) in sc.edges.iter().enumerate() {
        let Some((ia, ib)) = ends_of(e) else { continue };
        let (a, b) = edge_ends(e);
        let movable = free(e) && e.waypoints.is_empty();
        for (i, p, start) in [(ia, a, true), (ib, b, false)] {
            let n = face_normal(&sc.nodes[i], p);
            let along = if n.0 == 0.0 { p.0 } else { p.1 };
            faces
                .entry((i, face_key(n)))
                .or_default()
                .push((j, start, along, movable));
        }
    }
    // Free ends sharing a face spread evenly over the stretch between
    // its fixed ends (or corners), in the order the port stage gave
    // them: mermaid's class edges leave a box well inside its
    // corners. An end alone on its face slides later, to line up with
    // its other end.
    let mut moved: HashMap<(usize, bool), (f64, f64)> = HashMap::new();
    for (&(i, key), ends) in &faces {
        if ends.len() < 2 {
            continue;
        }
        let r = rects[i];
        let (lo, hi) = if key.0 == 0 { (r.0, r.2) } else { (r.1, r.3) };
        let mut ends = ends.clone();
        ends.sort_by(|p, q| p.2.total_cmp(&q.2));
        let mut from = (lo, 0);
        for k in 0..=ends.len() {
            let fixed = ends.get(k).map(|e| !e.3);
            if fixed == Some(false) {
                continue;
            }
            // Free ends `from.1 .. k` share the stretch up to here.
            let to = ends.get(k).map_or(hi, |e| e.2);
            let group = &ends[from.1..k];
            let step = (to - from.0) / (group.len() + 1) as f64;
            if step >= ORTHO_SEP * 2.0 {
                for (slot, &(j, start, _, _)) in group.iter().enumerate() {
                    let t = from.0 + (slot + 1) as f64 * step;
                    let p = if key.0 == 0 {
                        (t, if key.1 > 0 { r.3 } else { r.1 })
                    } else {
                        (if key.0 > 0 { r.2 } else { r.0 }, t)
                    };
                    moved.insert((j, start), p);
                }
            }
            from = (to, k + 1);
        }
    }
    let mut runs: Vec<Run> = sc
        .edges
        .iter()
        .filter(|e| routed(e))
        .flat_map(|e| e.waypoints.windows(2).map(|w| Run::of(w[0], w[1])))
        .collect();
    // Shortest first: short edges have the fewest ways to go, and a
    // back-edge bow — the longest way round — fits its lane and legs
    // around routes (and labels) already final.
    let mut order: Vec<usize> = (0..sc.edges.len()).collect();
    order.sort_by(|&p, &q| {
        let span = |e: &SceneEdge| {
            let (a, b) = (e.bezier[0], e.bezier[3]);
            (a.0 - b.0).abs() + (a.1 - b.1).abs()
        };
        span(&sc.edges[p]).total_cmp(&span(&sc.edges[q]))
    });
    for j in order {
        let e = &sc.edges[j];
        if !free(e) {
            continue;
        }
        let Some((ia, ib)) = ends_of(e) else { continue };
        let (a, b) = edge_ends(e);
        let (na, nb) = (face_normal(&sc.nodes[ia], a), face_normal(&sc.nodes[ib], b));
        let alone = (
            faces[&(ia, face_key(na))].len() == 1,
            faces[&(ib, face_key(nb))].len() == 1,
        );
        let a = moved.get(&(j, true)).copied().unwrap_or(a);
        let b = moved.get(&(j, false)).copied().unwrap_or(b);
        let labels: Vec<Rect> = sc
            .edges
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
            runs: &runs,
        };
        let corners = if e.waypoints.len() >= 2 {
            ctx.chain(&e.waypoints, na, nb)
        } else if na.0 * nb.0 + na.1 * nb.1 < -0.5 {
            ctx.opposite(e, a, b, na, alone)
        } else if (na.0 * nb.0 + na.1 * nb.1).abs() < 0.5 {
            ctx.perpendicular(a, b, na, nb)
        } else {
            None
        };
        let Some(corners) = corners else { continue };
        runs.extend(corners.windows(2).map(|w| Run::of(w[0], w[1])));
        let (a, b) = (corners[0], corners[corners.len() - 1]);
        let e = &mut sc.edges[j];
        if corners.len() == 2 {
            // Straight: a straight cubic, no waypoints — sampled as
            // finely as any curve (two bare waypoints would be sampled
            // as just their ends, which the label stage would read as
            // one long arrowhead).
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
            e.waypoints = crate::router::smooth(&corners, ORTHO_R);
        }
        let line = edge_polyline(e);
        if let Some((_, m, _)) = &mut e.label {
            *m = nearest_on(&line, *m);
        }
    }
}

/// An edge end on a box face: (edge, at its start, place along the
/// face, free to move).
type FaceEnd = (usize, bool, f64, bool);

/// One straight stretch of a placed orthogonal route: at `c` on the
/// fixed axis, from `l` to `h` along the other; `vertical` = runs
/// along y.
#[derive(Clone, Copy)]
struct Run {
    c: f64,
    l: f64,
    h: f64,
    vertical: bool,
}

/// What an orthogonal route is checked against: every box (`ia`, `ib`
/// are its own end boxes), the other edges' labels and the runs placed
/// so far.
struct Ortho<'a> {
    rects: &'a [Rect],
    labels: &'a [Rect],
    ia: usize,
    ib: usize,
    runs: &'a [Run],
}

impl Ortho<'_> {
    /// Whether polyline `pts` crosses a box; its own end boxes may be
    /// touched (at the ends) but not entered.
    fn blocked(&self, pts: &[(f64, f64)]) -> bool {
        pts.windows(2).any(|s| {
            self.rects.iter().enumerate().any(|(k, &r)| {
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
    /// `None` through a box; else [`ORTHO_OVERLAP`] per stretch lying
    /// on a placed run, [`ORTHO_UNDER_LABEL`] per stretch under another
    /// edge's label and [`ORTHO_CROSSING`] per crossing.
    fn cost(&self, pts: &[(f64, f64)]) -> Option<usize> {
        if self.blocked(pts) {
            return None;
        }
        let mut c = 0;
        for w in pts.windows(2) {
            let r = Run::of(w[0], w[1]);
            for q in self.runs {
                if q.vertical == r.vertical {
                    if (q.c - r.c).abs() < ORTHO_SEP && q.l < r.h - 2.0 && r.l + 2.0 < q.h {
                        c += ORTHO_OVERLAP;
                    }
                } else if r.l + 1.0 < q.c && q.c < r.h - 1.0 && q.l + 1.0 < r.c && r.c < q.h - 1.0 {
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
        }
        Some(c)
    }

    /// The cheapest of `cands` ([`Ortho::cost`]; the earlier wins a
    /// tie, so candidates come in order of preference).
    fn best(
        &self,
        cands: impl Iterator<Item = Vec<(f64, f64)>>,
    ) -> Option<(usize, Vec<(f64, f64)>)> {
        let mut best: Option<(usize, Vec<(f64, f64)>)> = None;
        for pts in cands {
            if let Some(c) = self.cost(&pts) {
                if best.as_ref().map_or(true, |b| c < b.0) {
                    best = Some((c, pts));
                }
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
            self.best(
                offsets(mid)
                    .filter(|y| (glo..=ghi).contains(y))
                    .map(|y| back(vec![fa, (fa.0, y), (fb.0, y), fb])),
            )
        };
        if let Some((c, pts)) = &direct {
            if *c < ORTHO_UNDER_LABEL {
                return Some(pts.clone());
            }
        }
        // Blocked between (or crowded): a lane at the curve's outer
        // extent (the bow), entered and left square to the faces, from
        // the ends as laid out — if the curve bows out past both ends.
        let (fa, fb) = ((sa, fa.1), (sb, fb.1));
        let mid = (fa.0 + fb.0) / 2.0;
        let lane = edge_polyline(e)
            .into_iter()
            .map(|p| f(p).0)
            .max_by(|p, q| (p - mid).abs().total_cmp(&(q - mid).abs()))
            .unwrap_or(mid);
        let out = if lane < fa.0.min(fb.0) - ORTHO_STUB {
            -1.0
        } else if lane > fa.0.max(fb.0) + ORTHO_STUB {
            1.0
        } else {
            return direct.map(|d| d.1);
        };
        // Each leg as close to its face as is cheap; the lane at the
        // bow, or a step in or out of it.
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
        let lanes = self.best([0, -1, 1, -2, 2, 3, 4, 5].into_iter().filter_map(|k| {
            let x = lane + out * f64::from(k) * ORTHO_SEP;
            let (ya, yb) = (leg(fa, s, x)?, leg(fb, -s, x)?);
            Some(back(vec![fa, (fa.0, ya), (x, ya), (x, yb), (fb.0, yb), fb]))
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
        (out_a >= ORTHO_STUB && out_b >= ORTHO_STUB && !self.blocked(&pts)).then_some(pts)
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
            let (_, jog) = self.best(
                offsets((q[i - 1].1 + p.1) / 2.0)
                    .filter(|y| (lo..=hi).contains(y))
                    .map(|y| back(vec![from, (x, y), (p.0, y), p])),
            )?;
            let y = f(jog[1]).1;
            out.push((x, y));
            out.push((p.0, y));
            x = p.0;
        }
        out.push((x, q[last].1));
        let out = back(out);
        (!self.blocked(&out)).then_some(out)
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

impl Run {
    /// The run from `a` to `b` (an axis-aligned stretch).
    fn of(a: (f64, f64), b: (f64, f64)) -> Run {
        if (a.0 - b.0).abs() < 0.5 {
            Run {
                c: a.0,
                l: a.1.min(b.1),
                h: a.1.max(b.1),
                vertical: true,
            }
        } else {
            Run {
                c: a.1,
                l: a.0.min(b.0),
                h: a.0.max(b.0),
                vertical: false,
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
    let annotated = d
        .relations
        .iter()
        .any(|r| r.label.is_some() || r.from_card.is_some() || r.to_card.is_some());
    if annotated {
        g.rank_gap = Some(RANK_GAP);
    }
    for c in &d.classes {
        g.ensure_node(&c.name, Some(c.display_name()), Some(Shape::Rect));
    }
    let n = d.classes.len();
    for (k, note) in d.notes.iter().enumerate() {
        g.ensure_node(
            &format!("note-{k}"),
            Some(note.text.clone()),
            Some(Shape::Rect),
        );
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
/// toward the nearer corner for a line leaving straight). A spot that
/// would touch any stroke, glyph, box, label or earlier cardinality
/// gives way to the other side, then to spots farther out; failing all,
/// the spot touching the fewest of them stands.
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
            // (collisions, centre, box) of the best spot so far.
            let mut best: Option<(usize, (f64, f64), Rect)> = None;
            'search: for depth in [0.0, 10.0, 20.0, 34.0] {
                for side in [first, -first] {
                    let dn = 2.0 + depth + ext_n / 2.0;
                    // Where the line is at that depth, then past the
                    // glyph (within its length) or the stroke.
                    let line_t = ut / un * dn;
                    let half = if dn - ext_n / 2.0 < glen { ghalf } else { 0.0 };
                    let dt = line_t + side * (half + CARD_CLEAR + 1.0 + ext_t / 2.0);
                    let c = (tip.0 + n.0 * dn + t.0 * dt, tip.1 + n.1 * dn + t.1 * dt);
                    let r = (c.0 - w / 2.0, c.1 - h / 2.0, c.0 + w / 2.0, c.1 + h / 2.0);
                    let grown = (
                        r.0 - CARD_CLEAR,
                        r.1 - CARD_CLEAR,
                        r.2 + CARD_CLEAR,
                        r.3 + CARD_CLEAR,
                    );
                    let hits = solid.iter().filter(|&&b| rects_meet(r, b)).count()
                        + placed.iter().filter(|&&b| rects_meet(grown, b)).count()
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
                        + glyph_segs
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
    /// away. Both are kept now (a `note for` names its class, which it
    /// creates like a relation would).
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
        let d = cd("classDiagram\nnote for Duck \"can fly\\ncan swim<br/>can dive\"");
        assert_eq!(d.notes[0].text, "can fly\ncan swim\ncan dive");
        for bad in ["note for \"x\"", "note", "direction XY"] {
            assert!(
                parse_document(&format!("classDiagram\n{bad}\nA <|-- B")).is_err(),
                "{bad}"
            );
        }
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

    /// Report M13 / "cardinality collisions": multiplicities sat on the
    /// strokes and arrowheads (fixed offsets along the tangent). Each
    /// one now clears every stroke, glyph, box, label and other
    /// cardinality.
    #[test]
    fn cardinalities_clear_strokes_glyphs_boxes_and_each_other() {
        let big = "classDiagram
    User \"1\" --> \"*\" Order : places
    Order \"1\" *-- \"1..*\" OrderLine
    OrderLine \"*\" --> \"1\" Product
    Customer \"0..1\" o-- \"many\" Order : holds
    Product \"1\" <|-- \"1\" Gift";
        for src in [include_str!("../examples/class.mmd"), big, ZOO] {
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
                    assert!(!rects_meet(b, o), "cards {b:?} {o:?} overlap");
                }
                for n in &cs.scene.nodes {
                    assert!(!rects_meet(b, rect(n)), "card {b:?} on {}", n.id);
                }
                for e in &cs.scene.edges {
                    let line = edge_polyline(e);
                    assert!(
                        !line.windows(2).any(|s| rect_hits_segment(b, s[0], s[1])),
                        "card {b:?} on {}->{}",
                        e.from,
                        e.to
                    );
                    if let Some((t, m, w)) = &e.label {
                        let l = (m.0 - w / 2.0, m.1 - 10.0, m.0 + w / 2.0, m.1 + 10.0);
                        assert!(!rects_meet(b, l), "card {b:?} on label {t}");
                    }
                }
                for h in &heads {
                    let k = h.polygon.len();
                    for s in 0..k {
                        let (p, q) = (h.polygon[s], h.polygon[(s + 1) % k]);
                        assert!(!rect_hits_segment(b, p, q), "card {b:?} on a glyph");
                    }
                    for s in &h.segments {
                        assert!(!rect_hits_segment(b, s[0], s[1]), "card {b:?} on a head");
                    }
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
