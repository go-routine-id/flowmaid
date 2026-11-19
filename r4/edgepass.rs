//! Edge finishing pipeline shared by the static [`scene`] and the
//! interactive [`route`] flowchart builders.
//!
//! Both builders hand their finished [`Scene`] to [`finish`], which
//! runs the same stages in a fixed order:
//!
//! 1. [`crate::ports::assign`]: distinct, ordered ports on every edge end.
//! 2. [`crate::router::repair`]: reroute edges that hit a node or frame
//!    or run on top of another edge.
//! 3. [`crate::ports::respace`]: space again the ends a reroute moved
//!    onto a side another edge already uses.
//! 4. [`crate::router::nudge`]: spread shared channels, merge stair
//!    steps and round corners (in the static scene, then put each side's
//!    ports in the order their paths head); then
//!    [`crate::router::spread_close`] moves one of two unrelated edges
//!    still side by side under the spacing clear.
//! 5. [`crate::labels::relabel`]: re-place the labels of changed edges,
//!    and any label that covers something or sits on another stroke
//!    where a clearer spot exists.
//! 6. [`refit`]: grow the canvas over whatever the stages moved.
//!
//! The emit side adds the crossing gaps ([`crate::casing::masks`]).
//! Every stage reads the shared [`Ctx`] built once by [`ctx`]; the
//! helpers here are the vocabulary the stages share (end rects, sides,
//! flow axes).
//!
//! [`crate::scene::route_partial`] runs the same stages with some edges
//! *fixed* ([`Ctx::fixed`]): they keep the geometry they came with and
//! are only obstacles to the others, so a drag pays for the edges near
//! what moved, not for the whole drawing.
//!
//! [`scene`]: crate::scene::scene
//! [`route`]: crate::scene::route

use crate::clearance::{CLEAR_GAP, CRAMP_NODE_GAP, EDGE_SPACING, FRAME_INSET};
use crate::model::{Direction, End, Graph};
use crate::scene::{
    basis_sample, edge_drawn, edge_polyline, grow_canvas, own_box_targets, sample_cubic,
    shift_scene, Bbox, Scene, ARROW_MARKER_PX, MARGIN,
};

type Pt = (f64, f64);

/// Which builder produced the scene. `Routed` scenes keep their node
/// positions exactly as the host gave them (no normalisation), so the
/// stages and [`refit`] must never shift them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    /// [`crate::scene::scene_sized`]: automatic layout.
    Static,
    /// [`crate::scene::route_sized`]: host-given node centres.
    Routed,
}

/// One end of a scene edge: a node or a subgraph frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum EndRef {
    /// Index into `sc.nodes` (index-parallel with `g.nodes`).
    Node(usize),
    /// Index into `sc.clusters`.
    Frame(usize),
}

/// A side of a rectangle in final screen coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

/// What every stage knows about the scene's edges, computed once.
pub(crate) struct Ctx {
    /// Index-parallel with `sc.edges`: the (source, target) ends, or
    /// `None` when an edge could not be matched to the graph.
    pub ends: Vec<Option<(EndRef, EndRef)>>,
    /// Per node: indices of every cluster containing it (its own frame
    /// and all ancestors).
    pub node_frames: Vec<Vec<usize>>,
    /// Per cluster: its own index plus its ancestor clusters' indices.
    pub frame_anc: Vec<Vec<usize>>,
    /// Index-parallel with `sc.edges`: the target lies behind the
    /// source along the flow (a back-edge).
    pub back: Vec<bool>,
    /// Index-parallel with `sc.edges`: the edge is painted and is not a
    /// self-loop, i.e. the stages may touch it.
    pub drawn: Vec<bool>,
    /// Index-parallel with `sc.edges`: the edge keeps its geometry. The
    /// router never reroutes it and prices every other edge against it;
    /// the ports and label stages still respace or re-place it when a
    /// changed edge crowds it. All `false` except in
    /// [`crate::scene::route_partial`].
    pub fixed: Vec<bool>,
}

/// Build the shared context. `sc.edges` holds `g.edges` first, then the
/// resolvable `g.sub_edges` in order; a sub-edge the builder dropped
/// (its subgraph has no frame) is skipped by matching ids in order. A
/// member's edge to its own enclosing box ends on the member it is
/// drawn to ([`own_box_targets`]), not on the frame.
pub(crate) fn ctx(g: &Graph, sc: &Scene) -> Ctx {
    let ns = g.subgraphs.len();
    // Subgraph <-> cluster, matched by id (first match wins).
    let mut sub_to_cl: Vec<Option<usize>> = vec![None; ns];
    let mut cl_to_sub: Vec<Option<usize>> = vec![None; sc.clusters.len()];
    for (ci, c) in sc.clusters.iter().enumerate() {
        if let Some(si) = g.subgraphs.iter().position(|s| s.id == c.id) {
            cl_to_sub[ci] = Some(si);
            if sub_to_cl[si].is_none() {
                sub_to_cl[si] = Some(ci);
            }
        }
    }
    // Clusters of subgraph `si` and all its ancestors, innermost first.
    // The walk is bounded by the subgraph count so a malformed parent
    // chain cannot loop.
    let chain = |si: usize| -> Vec<usize> {
        let mut out = Vec::new();
        let mut cur = Some(si);
        for _ in 0..ns {
            let Some(s) = cur else { break };
            if let Some(ci) = sub_to_cl[s] {
                out.push(ci);
            }
            cur = g.subgraphs[s].parent;
        }
        out
    };
    let frame_anc: Vec<Vec<usize>> = (0..sc.clusters.len())
        .map(|ci| match cl_to_sub[ci] {
            Some(si) => {
                let mut v = chain(si);
                if !v.contains(&ci) {
                    v.insert(0, ci);
                }
                v
            }
            None => vec![ci],
        })
        .collect();
    // Innermost owner of each node: nested members belong to the child.
    let mut owner: Vec<Option<usize>> = vec![None; g.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &v in &s.nodes {
            if let Some(o) = owner.get_mut(v) {
                *o = Some(si);
            }
        }
    }
    let node_frames: Vec<Vec<usize>> = owner.iter().map(|o| o.map_or(Vec::new(), chain)).collect();

    let node_ok = |v: usize, id: &str| v < sc.nodes.len() && sc.nodes[v].id == id;
    let mut ends: Vec<Option<(EndRef, EndRef)>> = vec![None; sc.edges.len()];
    for (i, e) in g.edges.iter().enumerate().take(sc.edges.len()) {
        let se = &sc.edges[i];
        let (Some(a), Some(b)) = (g.nodes.get(e.from), g.nodes.get(e.to)) else {
            continue;
        };
        if se.from == a.id && se.to == b.id && node_ok(e.from, &a.id) && node_ok(e.to, &b.id) {
            ends[i] = Some((EndRef::Node(e.from), EndRef::Node(e.to)));
        }
    }
    // Sub-edges: walk both lists in order, skipping graph sub-edges the
    // builder dropped.
    let end_of = |end: End| -> Option<(EndRef, &str)> {
        match end {
            End::Node(v) => {
                let id = g.nodes.get(v)?.id.as_str();
                node_ok(v, id).then_some((EndRef::Node(v), id))
            }
            End::Sub(si) => {
                let ci = (*sub_to_cl.get(si)?)?;
                Some((EndRef::Frame(ci), g.subgraphs[si].id.as_str()))
            }
        }
    };
    // A member's edge to its own box is drawn to the box's entry member
    // (`own_box_targets`): its stages treat it as an edge between the two.
    let targets = own_box_targets(g);
    let mut next = g.edges.len();
    for (e, target) in g.sub_edges.iter().zip(targets) {
        let Some(se) = sc.edges.get(next) else { break };
        let (Some((ra, ia)), Some((rb, ib))) = (end_of(e.from), end_of(e.to)) else {
            continue;
        };
        if se.from == ia && se.to == ib {
            let rb = match target {
                Some(m) if node_ok(m, &g.nodes[m].id) => EndRef::Node(m),
                _ => rb,
            };
            ends[next] = Some((ra, rb));
            next += 1;
        }
    }

    let back = ends
        .iter()
        .map(|en| {
            en.is_some_and(|(a, b)| {
                let ca = rect_centre(end_rect(sc, a));
                let cb = rect_centre(end_rect(sc, b));
                along(g.direction, cb) < along(g.direction, ca)
            })
        })
        .collect();
    let drawn = sc
        .edges
        .iter()
        .zip(&ends)
        .map(|(e, en)| {
            let self_loop = match en {
                Some((a, b)) => a == b,
                None => e.from == e.to,
            };
            edge_drawn(e) && !self_loop
        })
        .collect();
    let fixed = vec![false; sc.edges.len()];
    Ctx {
        ends,
        node_frames,
        frame_anc,
        back,
        drawn,
        fixed,
    }
}

/// Centre of an `(x0, y0, x1, y1)` rect.
fn rect_centre(r: (f64, f64, f64, f64)) -> (f64, f64) {
    ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0)
}

/// The bounding rect `(x0, y0, x1, y1)` of an edge end in final
/// coordinates: the node's box or the cluster's frame.
pub(crate) fn end_rect(sc: &Scene, e: EndRef) -> (f64, f64, f64, f64) {
    match e {
        EndRef::Node(v) => {
            let n = &sc.nodes[v];
            (
                n.x - n.w / 2.0,
                n.y - n.h / 2.0,
                n.x + n.w / 2.0,
                n.y + n.h / 2.0,
            )
        }
        EndRef::Frame(ci) => {
            let c = &sc.clusters[ci];
            (c.x, c.y, c.x + c.w, c.y + c.h)
        }
    }
}

/// The side of `rect` nearest to `p` (ties: top, bottom, left, right).
pub(crate) fn side_of(rect: (f64, f64, f64, f64), p: (f64, f64)) -> Side {
    let cands = [
        (Side::Top, (p.1 - rect.1).abs()),
        (Side::Bottom, (p.1 - rect.3).abs()),
        (Side::Left, (p.0 - rect.0).abs()),
        (Side::Right, (p.0 - rect.2).abs()),
    ];
    let mut best = cands[0];
    for &c in &cands[1..] {
        if c.1 < best.1 {
            best = c;
        }
    }
    best.0
}

/// Outward unit normal of a side (screen coordinates, y down).
pub(crate) fn normal(s: Side) -> (f64, f64) {
    match s {
        Side::Top => (0.0, -1.0),
        Side::Bottom => (0.0, 1.0),
        Side::Left => (-1.0, 0.0),
        Side::Right => (1.0, 0.0),
    }
}

/// Signed position of `p` along the flow, increasing downstream.
pub(crate) fn along(dir: Direction, p: (f64, f64)) -> f64 {
    match dir {
        Direction::TD => p.1,
        Direction::BT => -p.1,
        Direction::LR => p.0,
        Direction::RL => -p.0,
    }
}

/// Run the edge stages over a finished flowchart scene, then refit the
/// canvas. Idempotent while the stages are. The router is idempotent
/// only while its reroutes come out clean: an edge it improved but left
/// still touching something is searched again on a second run, and may
/// move.
pub(crate) fn finish(g: &Graph, sc: &mut Scene, mode: Mode) {
    finish_fixed(g, sc, mode, &[]);
}

/// [`finish`] with the edges flagged in `fixed` (index-parallel with
/// `sc.edges`; missing entries are `false`) kept as they are: see
/// [`Ctx::fixed`]. As in [`finish`], a label counts as changed only once
/// a stage moves its edge: a redrawn edge comes with its label at the
/// builder's spot on its new line, and the label stage moves it only
/// when it covers something there. (Counting every redrawn edge as
/// changed made the stage drop labels it could not improve onto worse
/// spots, such as their own node.)
pub(crate) fn finish_fixed(g: &Graph, sc: &mut Scene, mode: Mode, fixed: &[bool]) {
    let mut cx = ctx(g, sc);
    for (f, &x) in cx.fixed.iter_mut().zip(fixed) {
        *f = x;
    }
    let mut moved = vec![false; sc.edges.len()];
    crate::ports::assign(g, sc, &cx, mode, &mut moved);
    crate::router::repair(g, sc, &cx, mode, &mut moved);
    // A rerouted end may have switched to another side of its node and
    // landed on a port another edge already holds: space those groups
    // again. Groups the router left alone are already spaced and stay
    // byte-identical.
    crate::ports::respace(g, sc, &cx, mode, &mut moved);
    // After the ports are spaced again, so the channel nudge sees (and
    // keeps) the final ends; in the static scene it ends by putting each
    // side's ends in the order their paths now head (G9).
    crate::router::nudge(g, sc, &cx, mode, &mut moved);
    // Then two unrelated edges the nudge (or the builder) left side by
    // side under the spacing get one moved clear, where that costs
    // nothing else (G4).
    crate::router::spread_close(g, sc, &cx, mode, &mut moved);
    glyph_stubs(sc, &cx, &mut moved);
    snap_ends(sc, &cx);
    let budget = match mode {
        Mode::Static => crate::labels::LABEL_STATIC_WORK,
        Mode::Routed => crate::labels::LABEL_ROUTE_WORK,
    };
    crate::labels::relabel(g, sc, &cx, &moved, budget);
    // A scene whose painter strokes its own width (ER, class) is drawn
    // on its canvas as-is; a flowchart's is fitted like its SVG.
    refit(sc, mode, g.edge_paint_width.is_none());
}

/// How far (px) out of its end the straight stub [`glyph_stubs`] gives a
/// waypoint edge's glyph end reaches: its two waypoints sit this far and
/// half as far out, so the `curveBasis` spline through them runs
/// straight for 1.25 heads ((2.5 + 4 × 1.25) / 6 × [`ARROW_MARKER_PX`])
/// into the end — the head and the stroke entering its base square to
/// the side.
pub(crate) const WAYPOINT_STUB: f64 = 2.5 * ARROW_MARKER_PX;

/// Give every glyph end of a waypoint edge the flowchart painter draws
/// (not an ER or class scene's, see `paint_width`) a straight run into
/// it along its side's normal, a head long at least (report §2.3): the
/// spline the painter draws through the waypoints otherwise still turns
/// under the head — a long edge's last leg runs straight from its last
/// channel point, slanting into the side, and a port the ports stage
/// slid along the side leaves its old stub askew. Where the run is
/// short, the waypoints within [`WAYPOINT_STUB`] of the end along the
/// normal give way to a stub's two points, when the path beyond reaches
/// past it by a head (room to turn; a router path's own stub, cut short
/// by a corner, is left as it is) and the reshaped curve keeps as clear
/// of the nodes and edges round it as it was ([`stub_keeps_clear`]).
/// Runs after the ports and the nudge settle the ends, before the
/// labels; idempotent (a stubbed end is straight enough, a stub left out
/// is weighed alike again).
fn glyph_stubs(sc: &mut Scene, cx: &Ctx, moved: &mut [bool]) {
    for i in 0..sc.edges.len() {
        let e = &sc.edges[i];
        let (Some((from, to)), true) = (cx.ends[i], cx.drawn[i]) else {
            continue;
        };
        if e.waypoints.len() < 2 || e.paint_width.is_some() || e.meta.curve.is_some() {
            continue;
        }
        let glyphs = (
            e.ends.start_marker(e.kind).is_some(),
            e.ends.end_marker(e.kind).is_some(),
        );
        // Each end on its own: its stub is weighed against the curve as
        // it stands (the other end's stub in, if that one was kept).
        for (glyph, end, at_start) in [(glyphs.0, from, true), (glyphs.1, to, false)] {
            if !glyph {
                continue;
            }
            let mut wps = sc.edges[i].waypoints.clone();
            if at_start {
                wps.reverse();
            }
            let n = wps.len();
            let p = wps[n - 1];
            // A drag's kept edge may end on its node's outline, inside
            // the box (`snap_ends`): the outline decides its side.
            let rect = end_rect(sc, end);
            let out = normal(match end {
                EndRef::Node(v) => crate::ports::outline_side(sc.nodes[v].shape, rect, p),
                EndRef::Frame(_) => side_of(rect, p),
            });
            // How far out along the normal, and how far off its line.
            let along = |q: (f64, f64)| (q.0 - p.0) * out.0 + (q.1 - p.1) * out.1;
            let off = |q: (f64, f64)| ((q.0 - p.0) * out.1 - (q.1 - p.1) * out.0).abs();
            // The run of waypoints on the normal's ray, end first.
            let mut run = 1;
            while run < n && off(wps[n - 1 - run]) < STUB_ON_LINE && along(wps[n - 1 - run]) > 0.0 {
                run += 1;
            }
            // The straight stretch the spline draws into the end.
            let straight = match run {
                1 => 0.0,
                2 => along(wps[n - 2]) / 6.0,
                _ => {
                    let k = n - run;
                    (along(wps[k]) + 4.0 * along(wps[k + 1]) + along(wps[k + 2])) / 6.0
                }
            };
            if straight >= ARROW_MARKER_PX - STUB_ON_LINE {
                continue;
            }
            // The points within the stub give way to it, and what lies
            // beyond must leave a head's length to turn in.
            let keep = wps[..n - 1]
                .iter()
                .rposition(|&q| along(q) > WAYPOINT_STUB || off(q) >= STUB_ON_LINE);
            let room = keep.is_some_and(|k| along(wps[k]) >= WAYPOINT_STUB + ARROW_MARKER_PX);
            let (Some(k), true) = (keep, room) else {
                continue;
            };
            let at = |d: f64| (p.0 + out.0 * d, p.1 + out.1 * d);
            wps.truncate(k + 1);
            wps.extend([at(WAYPOINT_STUB), at(WAYPOINT_STUB / 2.0), p]);
            if at_start {
                wps.reverse();
            }
            let node: Vec<usize> = match end {
                EndRef::Node(v) => vec![v],
                EndRef::Frame(_) => Vec::new(),
            };
            if stub_keeps_clear(sc, cx, i, &wps, &node) {
                sc.edges[i].waypoints = wps;
                moved[i] = true;
            }
        }
    }
}

/// Put every end on a node onto its outline as painted (report M6, §2.5,
/// P0 #9; Q7 for cylinders). The stages above meet most shapes as their
/// box: an end on a parallelogram's flank floats beside the slant (7 px
/// at mid height), one on a hexagon's point beside the V, one off the
/// middle of a cylinder's top above the lid's back arc and one under its
/// bottom below the bottom arc, one near a rounded box's or a stadium's
/// corner off the arc there; a self-loop's ends sit on the box's side.
/// Each such end moves along its stroke's final run (the waypoint path's
/// last leg, a single curve's end tangent) onto the outline
/// ([`crate::scene::outline_along`]), so the run keeps its line and its
/// glyph its heading: a waypoint path's end point (the stub into the
/// side stays straight, [`glyph_stubs`] runs first), a single curve's
/// end (its control stays, so the curve keeps its course and only its
/// end lengthens or shortens; a control on the end itself goes with
/// it), and the chord's end in `bezier`. Geometry, not paint, so hosts
/// drawing the scene, picking and the SVG agree, in `scene()`, `route()`
/// and every drag frame alike — and the outline is the one
/// [`crate::scene::Scene::node_at`] picks by. Ends on frames and plain
/// boxes, and ends off their node's box by more than [`SNAP_REACH`],
/// stay. Idempotent: an end on the outline stays put.
fn snap_ends(sc: &mut Scene, cx: &Ctx) {
    for i in 0..sc.edges.len() {
        let Some((from, to)) = cx.ends[i] else {
            continue;
        };
        // Self-loops too: the stages leave them alone, but their ends
        // sit on the box side like any other.
        if !cx.drawn[i] && !(from == to && edge_drawn(&sc.edges[i])) {
            continue;
        }
        for (end, at_start) in [(from, true), (to, false)] {
            let EndRef::Node(v) = end else { continue };
            let Some(q) = snapped_end(&sc.nodes[v], &sc.edges[i], at_start) else {
                continue;
            };
            let e = &mut sc.edges[i];
            if e.waypoints.is_empty() {
                let (a, b) = if at_start { (0, 1) } else { (3, 2) };
                if e.bezier[b] == e.bezier[a] {
                    e.bezier[b] = q;
                }
                e.bezier[a] = q;
            } else {
                let k = if at_start { 0 } else { e.waypoints.len() - 1 };
                e.waypoints[k] = q;
                let a = if at_start { 0 } else { 3 };
                e.bezier[a] = q;
            }
            // Not `moved`: the end slides a few px along its own line,
            // which leaves the label stage nothing to re-place (and a
            // drag frame no label search per end).
        }
    }
}

/// Where [`snap_ends`] puts one end of edge `e` on node `n`: on the
/// outline along the stroke's final run, `None` where it stays. The run
/// is the waypoint path's last leg or the cubic's end tangent — what the
/// painter's spline or cubic leaves the end along; a mermaid `curve`
/// (step, bump, natural, …) leaves it along its own heading, which
/// [`crate::scene::edge_end_tangent`] reads off the painted path.
fn snapped_end(
    n: &crate::scene::SceneNode,
    e: &crate::scene::SceneEdge,
    at_start: bool,
) -> Option<Pt> {
    let pts: &[Pt] = if e.waypoints.is_empty() {
        &e.bezier
    } else {
        &e.waypoints
    };
    let p = if at_start { pts[0] } else { pts[pts.len() - 1] };
    // Only an end on or in its node's box: one the builders left off it
    // is not this stage's to pull in. Within the box an end beside the
    // shape (in a slanted shape's empty corner) is followed in along its
    // run, and one sunk into it comes out by at most
    // `scene::OUTLINE_SINK` (a router path's end can stop a little past
    // a lid, inside the drum).
    let (gx, gy) = ((p.0 - n.x).abs() - n.w / 2.0, (p.1 - n.y).abs() - n.h / 2.0);
    if gx.max(0.0).hypot(gy.max(0.0)) > SNAP_REACH {
        return None;
    }
    // The run out of the end; `(0, 0)` for a curve without length (two
    // nodes whose boxes touch), which goes in from its box side.
    let out = if e.meta.curve.is_some() {
        // The heading INTO the end at the target, out of it at the
        // source: the way out of the node is its reverse at the target.
        let (_, h) = crate::scene::edge_end_tangent(e, at_start);
        if at_start {
            h
        } else {
            (-h.0, -h.1)
        }
    } else {
        let apart = |q: &&Pt| (q.0 - p.0).hypot(q.1 - p.1) > 1e-9;
        let q = if at_start {
            pts.iter().find(apart)
        } else {
            pts.iter().rev().find(apart)
        };
        q.map_or((0.0, 0.0), |q| {
            let l = (q.0 - p.0).hypot(q.1 - p.1);
            ((q.0 - p.0) / l, (q.1 - p.1) / l)
        })
    };
    // A drum's shoulder meets a run along its flank's normal side-on:
    // the end goes to the nearest point of it instead, keeping its place
    // among the flank's ends.
    if n.shape == crate::model::Shape::Cylinder {
        if let Some(q) = crate::scene::cylinder_shoulder_point(n, p) {
            return ((q.0 - p.0).hypot(q.1 - p.1) > 1e-6).then_some(q);
        }
    }
    let rect = (
        n.x - n.w / 2.0,
        n.y - n.h / 2.0,
        n.x + n.w / 2.0,
        n.y + n.h / 2.0,
    );
    let side = normal(crate::ports::outline_side(n.shape, rect, p));
    // A run that leaves its side at a slant past [`SNAP_GRAZE`] — a
    // single curve setting off sideways from a lid, along the arc — would
    // carry the end far along the outline: it goes straight in from its
    // box side instead, as does one whose run misses the shape (a router
    // leg leaving a lid's corner across the drum's shoulder).
    let run = if out.0 * side.0 + out.1 * side.1 >= SNAP_GRAZE {
        out
    } else {
        side
    };
    crate::scene::outline_along(n, p, run).or_else(|| crate::scene::outline_along(n, p, side))
}

/// Least cosine between an end's run and its side's outward normal for
/// [`snap_ends`] to follow the run onto the outline (60°): a flatter run
/// meets a shallow outline (a lid's arc) nearly along it.
const SNAP_GRAZE: f64 = 0.5;

/// How far (px) outside its node's box an end may lie for [`snap_ends`]
/// to put it on the outline: the stages put an end exactly on the box
/// side (or on the outline already).
const SNAP_REACH: f64 = 0.5;

/// Whether edge `i` drawn through the stubbed waypoints `new`
/// ([`glyph_stubs`]) stays as clear of the scene as it was: where its
/// curve changes, it comes no nearer than [`CRAMP_NODE_GAP`] to a node
/// that is not one of its ends (unless its old curve came nearer
/// still), cuts into no frame holding neither end that its old curve
/// stayed out of, and runs no longer within [`CLEAR_GAP`] — nor within
/// [`EDGE_SPACING`] — of another edge than it did (only the latter for
/// one ending on the node the stub runs into, `stubbed`: ends
/// converging on a node run in side by side). The routing and port
/// stages spaced the old curve; a stub that would undo that is left
/// out, the painter squaring that end within a px instead
/// (`scene::square_end`).
fn stub_keeps_clear(sc: &Scene, cx: &Ctx, i: usize, new: &[Pt], stubbed: &[usize]) -> bool {
    let old = &edge_polyline(&sc.edges[i]);
    let fresh = basis_sample(new);
    // The stubs replace waypoints only near the ends, so the curves share
    // their samples away from them (the spline is local): the stretch of
    // each between its shared head and tail (a sample either side).
    let same = |a: Pt, b: Pt| (a.0 - b.0).abs() <= 1e-9 && (a.1 - b.1).abs() <= 1e-9;
    let head = fresh
        .iter()
        .zip(old)
        .take_while(|(a, b)| same(**a, **b))
        .count();
    let tail = fresh
        .iter()
        .rev()
        .zip(old.iter().rev())
        .take_while(|(a, b)| same(**a, **b))
        .count()
        .min(fresh.len().min(old.len()) - head);
    if head + tail >= fresh.len() && fresh.len() == old.len() {
        return true;
    }
    let cut = |a: &[Pt]| -> std::ops::RangeInclusive<usize> {
        head.saturating_sub(1)..=(a.len() - tail).min(a.len() - 1)
    };
    let (now, was) = (&fresh[cut(&fresh)], &old[cut(old)]);
    let mut bb = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in now {
        bb = (bb.0.min(p.0), bb.1.min(p.1), bb.2.max(p.0), bb.3.max(p.1));
    }
    let reach = CLEAR_GAP.max(CRAMP_NODE_GAP);
    let near = |r: (f64, f64, f64, f64)| {
        r.0 <= bb.2 + reach && r.2 >= bb.0 - reach && r.1 <= bb.3 + reach && r.3 >= bb.1 - reach
    };
    let Some((from, to)) = cx.ends[i] else {
        return true;
    };
    let own = |v: usize| {
        matches!(from, EndRef::Node(a) if a == v) || matches!(to, EndRef::Node(b) if b == v)
    };
    for (v, n) in sc.nodes.iter().enumerate() {
        let r = (
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.x + n.w / 2.0,
            n.y + n.h / 2.0,
        );
        if own(v) || !near(r) {
            continue;
        }
        let gap = |p: &Pt| {
            (r.0 - p.0)
                .max(p.0 - r.2)
                .max(0.0)
                .hypot((r.1 - p.1).max(p.1 - r.3).max(0.0))
        };
        let g_now = now.iter().map(gap).fold(f64::INFINITY, f64::min);
        let g_was = was.iter().map(gap).fold(f64::INFINITY, f64::min);
        if g_now < CRAMP_NODE_GAP && g_now < g_was - STUB_SAME {
            return false;
        }
    }
    // Nor does it newly cut into a frame holding neither end (B7 review
    // r1 #2: advanced's `Auth -> Users`, its stub into a comb's end on
    // the drum's lid, swung the spline's last span out round the corner
    // of the Async Workers frame it passed beside).
    let own_frame = |f: usize, end: EndRef| match end {
        EndRef::Node(v) => cx.node_frames.get(v).is_some_and(|fs| fs.contains(&f)),
        EndRef::Frame(c) => cx.frame_anc.get(c).is_some_and(|fs| fs.contains(&f)),
    };
    for (f, c) in sc.clusters.iter().enumerate() {
        let r = (c.x, c.y, c.x + c.w, c.y + c.h);
        if own_frame(f, from) || own_frame(f, to) || !near(r) {
            continue;
        }
        let inner = (
            r.0 + FRAME_INSET,
            r.1 + FRAME_INSET,
            r.2 - FRAME_INSET,
            r.3 - FRAME_INSET,
        );
        // Entered as the router and the harness count it: a piece of a
        // segment strictly inside the inset box.
        let enters = |run: &[Pt]| run.windows(2).any(|w| enters_box(w[0], w[1], inner));
        if enters(now) && !enters(was) {
            return false;
        }
    }
    // Length of `run` within `gap` of a line, its points' distances to
    // it given.
    let close = |run: &[Pt], dist: &[f64], gap: f64| -> f64 {
        run.windows(2)
            .zip(dist.windows(2))
            .filter(|(_, d)| d[0] < gap && d[1] < gap)
            .map(|(w, _)| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
            .sum()
    };
    for j in 0..sc.edges.len() {
        if j == i || !cx.drawn[j] {
            continue;
        }
        // Ends converging on the node a stub runs into run in side by
        // side, as their ports space them: only a run on top of one
        // (within EDGE_SPACING) counts there.
        let converging = cx.ends[j].is_some_and(|(a, b)| {
            [a, b]
                .iter()
                .any(|x| matches!(x, EndRef::Node(v) if stubbed.contains(v)))
        });
        let gaps: &[f64] = if converging {
            &[EDGE_SPACING]
        } else {
            &[EDGE_SPACING, CLEAR_GAP]
        };
        // Only its pieces near the changed stretch can come close.
        let piece = near_pieces(&sc.edges[j], &near);
        if piece.is_empty() {
            continue;
        }
        // Each point's distance to the line where it is under CLEAR_GAP
        // (beyond, any distance as good as far): pieces whose box lies
        // farther are skipped unmeasured.
        let gap_to = |p: Pt, r: (f64, f64, f64, f64)| {
            (r.0 - p.0)
                .max(p.0 - r.2)
                .max(0.0)
                .hypot((r.1 - p.1).max(p.1 - r.3).max(0.0))
        };
        let boxes: Vec<(f64, f64, f64, f64)> = piece
            .chunks(2)
            .map(|w| {
                (
                    w[0].0.min(w[1].0),
                    w[0].1.min(w[1].1),
                    w[0].0.max(w[1].0),
                    w[0].1.max(w[1].1),
                )
            })
            .collect();
        let pairs = |run: &[Pt]| -> Vec<f64> {
            run.iter()
                .map(|&p| {
                    piece
                        .chunks(2)
                        .zip(&boxes)
                        .filter(|(_, &r)| gap_to(p, r) < CLEAR_GAP)
                        .map(|(w, _)| seg_dist(p, w[0], w[1]))
                        .fold(f64::INFINITY, f64::min)
                })
                .collect()
        };
        let (d_now, d_was) = (pairs(now), pairs(was));
        if gaps
            .iter()
            .any(|&g| close(now, &d_now, g) > close(was, &d_was, g) + STUB_CLOSE_SLACK)
        {
            return false;
        }
    }
    true
}

/// Whether segment `a`-`b` has a piece strictly inside the box `r`
/// `(x0, y0, x1, y1)`: the midpoint of its clipped part lies inside
/// (Liang-Barsky), as `router::World::score` counts a frame entered.
fn enters_box(a: Pt, b: Pt, r: (f64, f64, f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - r.0),
        (dx, r.2 - a.0),
        (-dy, a.1 - r.1),
        (dy, r.3 - a.1),
    ] {
        if p == 0.0 {
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
    let m = (a.0 + dx * (t0 + t1) / 2.0, a.1 + dy * (t0 + t1) / 2.0);
    m.0 > r.0 && m.0 < r.2 && m.1 > r.1 && m.1 < r.3
}

/// The pieces (point pairs) of `e`'s curve ([`edge_polyline`]'s, as
/// sampled there) whose box `near` accepts, sampling only the spans of a
/// spline whose waypoints' box it accepts (a span lies within its
/// waypoints' hull): a stub is weighed against the few edges near it,
/// mid-drag, without sampling every edge.
fn near_pieces(
    e: &crate::scene::SceneEdge,
    near: &dyn Fn((f64, f64, f64, f64)) -> bool,
) -> Vec<Pt> {
    let boxed = |pts: &[Pt]| {
        let mut r = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for p in pts {
            r = (r.0.min(p.0), r.1.min(p.1), r.2.max(p.0), r.3.max(p.1));
        }
        r
    };
    let keep = |line: &[Pt]| -> Vec<Pt> {
        line.windows(2)
            .filter(|w| near(boxed(w)))
            .flat_map(|w| [w[0], w[1]])
            .collect()
    };
    let w = &e.waypoints;
    if e.meta.curve.is_some() || w.len() < 3 {
        // A mermaid curve may overshoot its points: sampled whole.
        if e.meta.curve.is_none() && w.len() < 2 && !near(boxed(&e.bezier)) {
            return Vec::new();
        }
        return keep(&edge_polyline(e));
    }
    // The `curveBasis` spline span by span (as basis_sample samples
    // it), each sampled only when the box of its waypoints is near.
    let n = w.len();
    let lerp6 =
        |a: Pt, b: Pt, c: Pt| ((a.0 + 4.0 * b.0 + c.0) / 6.0, (a.1 + 4.0 * b.1 + c.1) / 6.0);
    let first = ((5.0 * w[0].0 + w[1].0) / 6.0, (5.0 * w[0].1 + w[1].1) / 6.0);
    let mut out = keep(&[w[0], first]);
    let mut cur = first;
    for i in 2..=n {
        let (a, b) = (w[i - 2], w[i - 1]);
        let p = w[i.min(n - 1)];
        let end = lerp6(a, b, p);
        if near(boxed(&w[i.saturating_sub(3)..=i.min(n - 1)])) {
            let bez = [
                cur,
                ((2.0 * a.0 + b.0) / 3.0, (2.0 * a.1 + b.1) / 3.0),
                ((a.0 + 2.0 * b.0) / 3.0, (a.1 + 2.0 * b.1) / 3.0),
                end,
            ];
            out.extend(keep(&sample_cubic(&bez, 6)));
        }
        cur = end;
    }
    out.extend(keep(&[cur, w[n - 1]]));
    out
}

/// How far (px) two curves may lie apart and count as the same there
/// ([`stub_keeps_clear`]), and the least gap it counts as nearer.
const STUB_SAME: f64 = 0.5;
/// How much longer (px) a stub may make an edge's run close beside
/// another ([`stub_keeps_clear`]): sampling slack.
const STUB_CLOSE_SLACK: f64 = 2.0;

/// Distance from `p` to the segment `a`–`b`.
fn seg_dist(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

/// How far (px) a waypoint may lie off an end's normal and still count
/// as on its stub ([`glyph_stubs`]), and the slack its straight stretch
/// is judged with: rounding only.
const STUB_ON_LINE: f64 = 0.01;

/// Grow the canvas over everything painted ([`grow_canvas`]): a route
/// round the outermost node, every stroke's width, every end glyph.
///
/// A static flowchart (`Static`, `static_fit`) is fitted to its drawing
/// exactly: shifted so the drawing starts [`MARGIN`] in on both axes —
/// only a translation after every stage, so no route moves — and its
/// canvas reaches exactly [`MARGIN`] past the drawing's far side. So the
/// margins match on all four sides (room the layout left before the
/// drawing, or past a curve's control points, is dropped: report Q2),
/// and a host drawing the scene on its `width` × `height` sees the
/// canvas `to_svg` writes, coordinate for coordinate. A `Routed` scene
/// is never shifted: its node positions are the host's, exactly as given
/// (the export handles a negative origin, see `fit_top_left`); its canvas
/// only grows. `static_fit` is false for an ER or class scene, whose
/// painter draws `width` × `height` as-is, padded its own way: it is
/// shifted only out of negative space and its canvas only grows.
pub(crate) fn refit(sc: &mut Scene, mode: Mode, static_fit: bool) {
    let mut bb = Bbox::new();
    grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    if !bb.0.is_finite() {
        return;
    }
    let (minx, mut maxx, miny, mut maxy) = bb.finish();
    if mode == Mode::Static && static_fit {
        // Float noise from an earlier fit is no shift: the stages are
        // idempotent, and so is the fit (`finish` run twice).
        let (dx, dy) = (MARGIN - minx, MARGIN - miny);
        let (dx, dy) = (
            if dx.abs() > FIT_EPS { dx } else { 0.0 },
            if dy.abs() > FIT_EPS { dy } else { 0.0 },
        );
        if dx != 0.0 || dy != 0.0 {
            shift_scene(sc, dx, dy);
        }
        let (w, h) = (maxx - minx + 2.0 * MARGIN, maxy - miny + 2.0 * MARGIN);
        if (sc.width - w).abs() > FIT_EPS {
            sc.width = w;
        }
        if (sc.height - h).abs() > FIT_EPS {
            sc.height = h;
        }
        return;
    }
    let floor = if static_fit { MARGIN } else { 0.0 };
    if mode == Mode::Static && (minx < floor || miny < floor) {
        let dx = if minx < floor { MARGIN - minx } else { 0.0 };
        let dy = if miny < floor { MARGIN - miny } else { 0.0 };
        shift_scene(sc, dx, dy);
        sc.width += dx;
        sc.height += dy;
        maxx += dx;
        maxy += dy;
    }
    sc.width = sc.width.max(maxx + MARGIN);
    sc.height = sc.height.max(maxy + MARGIN);
}

/// How far (px) [`refit`] lets a static flowchart's drawing sit off its
/// exact fit before it moves it: float noise, far below a pixel.
const FIT_EPS: f64 = 1e-6;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::scene::{route, scene};

    /// `finish` again on a finished scene changes nothing.
    fn assert_idempotent(src: &str) {
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let mut again = sc.clone();
        finish(&g, &mut again, Mode::Static);
        assert_eq!(format!("{sc:?}"), format!("{again:?}"), "static: {src}");

        let centres: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rs = route(&g, &centres);
        let mut again = rs.clone();
        finish(&g, &mut again, Mode::Routed);
        assert_eq!(format!("{rs:?}"), format!("{again:?}"), "routed: {src}");
    }

    /// A long edge's spline runs straight into its arrowhead along the
    /// side's normal for a head's length at least (report M1): its last
    /// channel leg used to slant straight into the node, the head riding
    /// the slant; the stub points sit on the normal, the finished scene
    /// stays a fixed point of `finish`.
    #[test]
    fn long_edges_run_straight_into_their_arrowheads() {
        let src = "flowchart TD\nA --> B --> C --> D\nA --> D";
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let mut seen = 0;
        // The dummy chain A -> D past B and C (a router path keeps its
        // own stub, a corner one clearance out).
        for e in sc
            .edges
            .iter()
            .filter(|e| e.from == "A" && e.to == "D" && e.waypoints.len() >= 3)
        {
            let w = &e.waypoints;
            let n = w.len();
            let p = w[n - 1];
            // Plumb into the top side: the two stub points right above.
            assert_eq!((w[n - 2].0, w[n - 3].0), (p.0, p.0), "{w:?}");
            let up = |q: (f64, f64)| p.1 - q.1;
            assert!((up(w[n - 2]) - WAYPOINT_STUB / 2.0).abs() < 1e-9, "{w:?}");
            assert!((up(w[n - 3]) - WAYPOINT_STUB).abs() < 1e-9, "{w:?}");
            // The curveBasis spline's straight run into the end.
            let straight = (up(w[n - 3]) + 4.0 * up(w[n - 2])) / 6.0;
            assert!(straight >= ARROW_MARKER_PX, "{straight}");
            seen += 1;
        }
        assert!(seen > 0, "a long edge");
        assert_idempotent(src);
    }

    /// A stub that would bend a long edge's curve towards a node beside
    /// it, or along another edge, is left out (B1 review #3: a stubbed
    /// `gw -> svc5` passed 6.5 px from `lb` in bench-75, against 16.6):
    /// the routing and port stages spaced the curve as it was.
    #[test]
    fn a_stub_never_crowds_a_node_or_an_edge_its_curve_kept_clear_of() {
        let g = parse("flowchart TD\nA --> B --> C --> D\nA --> D").unwrap();
        let sc = scene(&g);
        let cx = ctx(&g, &sc);
        let i = (0..sc.edges.len())
            .find(|&i| sc.edges[i].from == "A" && sc.edges[i].to == "D")
            .unwrap();
        let node = |id: &str| sc.nodes.iter().position(|n| n.id == id).unwrap();
        let d = node("D");
        let old = sc.edges[i].waypoints.clone();
        // A's curve runs down beside B and C to its stub into D's top.
        let n = old.len();
        assert!(n >= 5, "{old:?}");
        // As it is: nothing changes, nothing crowds.
        assert!(stub_keeps_clear(&sc, &cx, i, &old, &[d]));
        // Its last channel point swung onto C: the curve (which only
        // approaches its waypoints) crowds C.
        let c = &sc.nodes[node("C")];
        let mut onto = old.clone();
        onto[n - 4] = (c.x, c.y);
        assert!(!stub_keeps_clear(&sc, &cx, i, &onto, &[d]));
        // ... swung onto the line C -> D, clear of the nodes: it runs along
        // that line (which ends on D, the node the stub runs into: only
        // running on top of it, within EDGE_SPACING, counts).
        let cd = (0..sc.edges.len())
            .find(|&j| sc.edges[j].from == "C" && sc.edges[j].to == "D")
            .unwrap();
        let line = edge_polyline(&sc.edges[cd]);
        let at = |f: f64| {
            let q = line[((line.len() - 1) as f64 * f) as usize];
            (q.0 + 2.0, q.1)
        };
        let mut along = old.clone();
        along.splice(n - 4..n - 1, [at(0.3), at(0.5), at(0.7)]);
        assert!(!stub_keeps_clear(&sc, &cx, i, &along, &[d]));
        // A stub moving the curve in open air is kept.
        let mut aside = old.clone();
        aside[n - 4].0 += 6.0;
        assert!(stub_keeps_clear(&sc, &cx, i, &aside, &[d]));
    }

    #[test]
    fn finish_is_idempotent_on_a_flat_diagram() {
        assert_idempotent("flowchart TD\nA-->B\nB-->C\nC-->|no|A\nB-->D");
    }

    #[test]
    fn finish_is_idempotent_with_sub_edges() {
        assert_idempotent(
            "flowchart LR\nx-->S\nsubgraph S\na-->b\nsubgraph T\nc\nend\nend\nS-->y\nT-->x",
        );
    }

    #[test]
    fn ctx_matches_edges_and_sub_edges() {
        let g =
            parse("flowchart LR\nx-->S\nsubgraph S\na-->b\nsubgraph T\nc\nend\nend\nS-->y\nT-->x")
                .unwrap();
        let sc = scene(&g);
        let cx = ctx(&g, &sc);
        assert_eq!(cx.ends.len(), sc.edges.len());
        assert!(cx.ends.iter().all(Option::is_some), "{:?}", cx.ends);
        let ci = |id: &str| sc.clusters.iter().position(|c| c.id == id).unwrap();
        let (s, t) = (ci("S"), ci("T"));
        let node = |id: &str| g.nodes.iter().position(|n| n.id == id).unwrap();
        // Sub-edges follow the plain edges, in source order.
        let subs: Vec<_> = cx.ends[g.edges.len()..]
            .iter()
            .map(|e| e.unwrap())
            .collect();
        assert_eq!(
            subs,
            vec![
                (EndRef::Node(node("x")), EndRef::Frame(s)),
                (EndRef::Frame(s), EndRef::Node(node("y"))),
                (EndRef::Frame(t), EndRef::Node(node("x"))),
            ]
        );
        assert_eq!(cx.frame_anc[t], vec![t, s]);
        assert_eq!(cx.node_frames[node("c")], vec![t, s]);
        assert_eq!(cx.node_frames[node("a")], vec![s]);
        assert!(cx.node_frames[node("x")].is_empty());
        // T -> x runs against the LR flow; x -> S follows it.
        assert!(cx.back[g.edges.len() + 2]);
        assert!(!cx.back[g.edges.len()]);
        assert!(cx.drawn.iter().all(|&d| d));
    }

    #[test]
    fn sides_and_axes() {
        let r = (0.0, 0.0, 100.0, 40.0);
        assert_eq!(side_of(r, (50.0, 1.0)), Side::Top);
        assert_eq!(side_of(r, (50.0, 39.0)), Side::Bottom);
        assert_eq!(side_of(r, (2.0, 20.0)), Side::Left);
        assert_eq!(side_of(r, (99.0, 20.0)), Side::Right);
        // Leaving by the flow-out side moves downstream.
        for (d, out) in [
            (Direction::TD, Side::Bottom),
            (Direction::BT, Side::Top),
            (Direction::LR, Side::Right),
            (Direction::RL, Side::Left),
        ] {
            assert!(along(d, normal(out)) > 0.0);
        }
    }

    #[test]
    fn refit_shifts_static_but_not_routed() {
        let g = parse("flowchart TD\nA-->B").unwrap();
        let base = scene(&g);
        for mode in [Mode::Static, Mode::Routed] {
            let mut sc = base.clone();
            shift_scene(&mut sc, -500.0, 0.0);
            refit(&mut sc, mode, true);
            let minx = sc
                .nodes
                .iter()
                .map(|n| n.x - n.w / 2.0)
                .fold(f64::INFINITY, f64::min);
            match mode {
                Mode::Static => assert!((minx - MARGIN).abs() < 1e-9),
                Mode::Routed => assert!(minx < 0.0),
            }
            assert!(sc.width >= base.width - 500.0);
        }
    }
}
