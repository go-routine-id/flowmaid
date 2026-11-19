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
//!    ports in the order their paths head).
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

use crate::model::{Direction, End, Graph};
use crate::scene::{edge_drawn, grow_scene, shift_scene, Bbox, Scene, MARGIN};

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
/// (its subgraph has no frame) is skipped by matching ids in order.
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
    let mut next = g.edges.len();
    for e in &g.sub_edges {
        let Some(se) = sc.edges.get(next) else { break };
        let (Some((ra, ia)), Some((rb, ib))) = (end_of(e.from), end_of(e.to)) else {
            continue;
        };
        if se.from == ia && se.to == ib {
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
    let budget = match mode {
        Mode::Static => crate::labels::LABEL_STATIC_WORK,
        Mode::Routed => crate::labels::LABEL_ROUTE_WORK,
    };
    crate::labels::relabel(g, sc, &cx, &moved, budget);
    refit(sc, mode);
}

/// Grow the canvas over everything drawn; never shrink it.
///
/// A `Static` scene that reaches below zero is shifted so its minimum
/// becomes [`MARGIN`]. A `Routed` scene is never shifted: its node
/// positions are the host's, exactly as given (the export handles a
/// negative origin, see `fit_top_left`).
pub(crate) fn refit(sc: &mut Scene, mode: Mode) {
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    if !bb.0.is_finite() {
        return;
    }
    let (minx, mut maxx, miny, mut maxy) = bb.finish();
    if mode == Mode::Static && (minx < 0.0 || miny < 0.0) {
        let dx = if minx < 0.0 { MARGIN - minx } else { 0.0 };
        let dy = if miny < 0.0 { MARGIN - miny } else { 0.0 };
        shift_scene(sc, dx, dy);
        sc.width += dx;
        sc.height += dy;
        maxx += dx;
        maxy += dy;
    }
    sc.width = sc.width.max(maxx + MARGIN);
    sc.height = sc.height.max(maxy + MARGIN);
}

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
            refit(&mut sc, mode);
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
