//! Stage 4b of [`crate::edgepass::finish`]: a fan's comb (report B4).
//!
//! Three or more edges leaving (or entering) one side of a box form a
//! fan. Drawn on the axes ([`crate::ortho`]), each leaves its port on a
//! stub straight out of the side, steps across on a track in the gap in
//! front of it and runs on from the place it turns (its *turn*): a comb.
//! The comb reads as one when its teeth nest — the ports, the tracks and
//! the turns in one order, so no tooth crosses another: of the teeth
//! stepping one way, the one turning furthest out leaves from the
//! outermost port on the track nearest the side (libavoid's channel
//! order, which the ortho stage and the router's nudge keep for the
//! tracks of one gap). The stages after the ortho stage move ports for
//! their own reasons — a reroute takes another column, the nudge spreads
//! the port legs of one channel, the static reorder (G9) re-places a
//! side by where its legs head — and a hub's fan came out with its ports
//! in one order and its tracks in another: dense-td's `n16` had six
//! edges stepping right whose tracks nested and whose ports did not,
//! each stub crossing the tracks of the teeth beyond it, and a seventh
//! stepping left across four stubs to a column of its own (14 crossings
//! under one box).
//!
//! Here, last of the path stages, every such side is combed:
//!
//! - its teeth this stage may redraw (see [`Comb`]) take the side's
//!   ports — those they already hold, so the side keeps the even band
//!   the ports stage spread ([`crate::ports::PORT_EVEN_MAX`]) — in the
//!   order of their turns ([`Order::Turn`]);
//! - the tracks of the teeth stepping one way are put in the same order,
//!   moving as few as that takes ([`Deal::Anchored`]: a tooth out of
//!   order gets a track of its own between its new neighbours'); or
//!   each as near where it lies as the order and a pitch between two
//!   allow ([`Deal::Fitted`]); or, where the gap has no room for either,
//!   dealing out again the tracks the teeth hold — those of each way
//!   among themselves ([`Deal::Ways`]), then all of them
//!   ([`Deal::Pooled`]);
//! - where the tracks cannot follow the turns — a hub whose leaves wrap
//!   onto two rows steps to the near row in the gap in front of it and
//!   to the far row in the gap beyond, whatever the turns — the ports
//!   follow the gaps instead: the teeth of each gap nest among
//!   themselves ([`Order::Rows`]), and of the teeth stepping one way
//!   those in the gap nearest the side leave from the outermost ports,
//!   so no tooth's track runs across the stub of one that steps further
//!   out (fan047 of the hub-fan set: `hub --> l4` to the near row left
//!   from a port inside that of `hub --> l1` to the far row, across its
//!   stub);
//! - where no track can move — a hub beside a column of leaves, each
//!   tooth running straight into its leaf — the ports follow the tracks
//!   as they lie ([`Order::Track`]);
//! - or the ports alone take the order of the turns ([`Deal::Keep`]).
//!
//! A side's new comb is kept only when its edges cross fewer lines than
//! before and hit no more boxes, frame borders, title chips or labels,
//! nor run on top of or closer beside another edge ([`keeps`]): what the
//! router and the nudge spaced stays spaced. The first deal kept is not
//! the last word: the side is dealt again as it then stands, until no
//! deal undoes another crossing ([`ROUNDS`]).
//!
//! What a comb cannot undo: a path that is no tooth. An edge the router
//! led out one way and back under the fan, or one that jogs sideways
//! before its track, is dealt by its first step alone, and a tooth whose
//! track runs straight into its far end (an L) crosses whatever stands
//! between — on the reviewer's 120 hub fans (one hub, 5 to 16 leaves) 3
//! drawings in `scene()` and 2 in `route()` keep such crossings inside
//! the fan, 7 and 4 in all, where 47 and 64 drawings had 104 and 150
//! before the rows were dealt. Two stubs of the side
//! itself are the exception: they stand as far apart as its ports,
//! whatever order its teeth take them in, so a hub with more ends than
//! its side holds a spacing clear apart (ten on one side of a small
//! box) is combed like any other — its stubs side by side, as they
//! were, instead of across each other.
//!
//! The edges stay separate strokes, each its own port and track: merged
//! onto one trunk, a fan's edges could not be told apart, picked or
//! styled one by one (a dotted or thick tooth would lend its look to the
//! others), and the trunk would read as the "black bar" report §6 asks
//! to keep out.
//!
//! The same rule runs in [`crate::scene::scene`], [`crate::scene::route`]
//! and [`crate::scene::route_sized`]: a few sorts per fan and a crossing
//! count over the edges it changes. In [`crate::scene::route_partial`] an
//! end a drag keeps ([`Ctx::held`]) is left where it is, like any other
//! end this stage does not move, and the other teeth comb round it — as
//! far as the frame's time allows ([`DRAG_WORK`]): a drag frame combs
//! its small fans only, and none at all once it redraws a hub's worth
//! of edges. Such a fan stays as the stages before drew it, on the axes
//! like any other, until the drag ends and the host's `route()` combs
//! it.

use crate::clearance::{
    CLEAR_GAP, CRAMP_NODE_GAP, EDGE_SPACING, MIN_OVERLAP_RUN, NODE_CLEARANCE, NODE_SEP, PORT_ZONE,
    TIGHT_RUN_MIN,
};
use crate::edgepass::{end_rect, normal, side_of, Ctx, EndRef, Side};
use crate::ortho::{HEAD_LEG, ROW_CLEAR, TRACK_PITCH};
use crate::router::{ortho_corners, smooth, CORNER_R};
use crate::scene::{edge_polyline, Scene, SceneEdge};
use std::collections::BTreeMap;

type Pt = (f64, f64);
type Rect = (f64, f64, f64, f64);

/// Fewest ends on one side that make it a fan (report B4): from there
/// its ports are spread one pitch apart ([`plan_even`]).
pub(crate) const FAN_MIN: usize = 3;
/// Fewest teeth of one side that are put in order: two of a side that
/// cross each other are out of order like a fan's (fanout-frames'
/// `n0 --> g6` over `n0 --> n21` in `route()`, the two ends of its hub's
/// bottom side).
const PAIR: usize = 2;
/// Float tolerance (px).
const EPS: f64 = 1e-6;
/// How far (px) a port may lie off its side's line and still be dealt
/// along it (an end on a shape's outline inside its box is left alone).
const SIDE_SLACK: f64 = 0.5;
/// Least straight run (px) out of a bare end, or on past a turn into
/// the rest of the path, before the next corner: half a rounding.
const BARE_LEG: f64 = CORNER_R / 2.0;
/// Least step (px) across a tooth takes from its port to its turn: a
/// shorter one is a kink, not a step.
const STEP_MIN: f64 = CORNER_R / 2.0;
/// How far (px) a side's ports may lie off one even pitch and be left
/// as they are ([`plan_even`]): what a reader's eye does not follow.
const EVEN_SLACK: f64 = 1.0;
/// Least pitch (px) of two tracks of one comb: two strokes rather than
/// one on top of the other (the ortho stage's own least pitch).
const PITCH_MIN: f64 = EDGE_SPACING + 0.5;

/// The two axes of a side: outward normal and tangent (both unit,
/// along the screen axes).
#[derive(Clone, Copy, PartialEq)]
struct Frame {
    nv: Pt,
    tv: Pt,
}

impl Frame {
    /// The frame of `side`.
    fn of(side: Side) -> Frame {
        let nv = normal(side);
        let tv = match side {
            Side::Top | Side::Bottom => (1.0, 0.0),
            Side::Left | Side::Right => (0.0, 1.0),
        };
        Frame { nv, tv }
    }

    /// A screen point as (along the side, out of it).
    fn local(self, p: Pt) -> Pt {
        (
            p.0 * self.tv.0 + p.1 * self.tv.1,
            p.0 * self.nv.0 + p.1 * self.nv.1,
        )
    }

    /// Inverse of [`Frame::local`].
    fn screen(self, q: Pt) -> Pt {
        (
            q.0 * self.tv.0 + q.1 * self.nv.0,
            q.0 * self.tv.1 + q.1 * self.nv.1,
        )
    }
}

/// One end on a fanned side, in the side's own coordinates
/// ([`Frame::local`]): `t` along the side (growing with screen x on a
/// top or bottom side, screen y on a left or right one), `n` straight
/// out of it.
struct Tooth {
    edge: usize,
    /// The end is its edge's start.
    src: bool,
    /// Port, along the side.
    t: f64,
    /// How this stage may redraw it, if it may.
    comb: Option<Comb>,
}

/// A tooth this stage may redraw: its edge is free ([`free`]) and its
/// path leaves the end as a comb's tooth does — a stub straight out to a
/// track `depth` out, across it to the `turn`, and on from there.
struct Comb {
    /// The path's corners from this end, in side coordinates.
    path: Vec<Pt>,
    depth: f64,
    turn: f64,
    /// The depths its track may take: from the straight run its end
    /// needs out to where the path runs on past the turn (less the run
    /// that leg needs), and its own depth where that lies outside. One
    /// depth, its own, when the track is pinned: it runs into the
    /// edge's far end (an L), the path heads back towards the side past
    /// the turn, or the two runs leave no room between them.
    lo: f64,
    hi: f64,
}

impl Comb {
    fn pinned(&self) -> bool {
        self.hi - self.lo < EPS
    }
}

/// The order a side's ports are dealt to its teeth in.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Order {
    /// The order of their turns: with the tracks in the same order the
    /// teeth nest.
    Turn,
    /// Row by row ([`rows`]: the teeth whose tracks share a gap), in
    /// the order of their turns within each: the teeth stepping low
    /// first, from the row nearest the side, then those stepping high,
    /// down to it. The teeth of a row nest like a side's under
    /// [`Order::Turn`], and no track runs across the stub of a tooth
    /// that steps in a row further out. Each tooth keeps stepping the
    /// way it does.
    Rows,
    /// The order of the tracks they hold, likewise: no track then runs
    /// across the stub of a tooth stepping the same way, whatever the
    /// turns.
    Track,
}

/// The ways a side is combed, the first that has an answer and leaves
/// the drawing better ([`keeps`]) being taken: the ports by the turns
/// with the tracks put in that order — over the whole side, then row by
/// row —, then by the tracks as they lie, then by the turns alone.
/// (Dealing a row's tracks out again never found a comb the two deals
/// that move them had not, over some 1 500 generated drawings.)
const DEALS: [(Order, Deal); 8] = [
    (Order::Turn, Deal::Anchored),
    (Order::Turn, Deal::Fitted),
    (Order::Turn, Deal::Ways),
    (Order::Turn, Deal::Pooled),
    (Order::Rows, Deal::Anchored),
    (Order::Rows, Deal::Fitted),
    (Order::Track, Deal::Keep),
    (Order::Turn, Deal::Keep),
];

/// How often a side is combed again after a deal that left it with
/// fewer crossings (see [`comb`]).
const ROUNDS: usize = 4;

/// How a side's tracks are put in the order of its ports.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Deal {
    /// The teeth already in order keep their tracks; each of the others
    /// gets a track of its own between its new neighbours'.
    Anchored,
    /// Every track as near where it lies as the order allows: a pitch
    /// beyond the one before it, within its tooth's range.
    Fitted,
    /// The tracks of the teeth stepping each way are dealt out again
    /// among those, in order.
    Ways,
    /// The tracks the side's teeth hold are dealt out again in order,
    /// to the two ways in turn.
    Pooled,
    /// Every tooth keeps its track: the ports alone change hands.
    Keep,
}

/// Whether the stages may move edge `i`'s ends here: a flowchart edge
/// on the axes that no drag keeps, that is no labelled twin's bulge, no
/// authored `curve:` and no note card's link. A back-edge's lane is
/// combed like any path — its stub and the step onto its lane are a
/// tooth of the side it leaves, the lane beyond its turn stays as
/// planned (dense-td's `n43 -> n16`, whose stub crossed the track of
/// the arrival beside it).
fn free(sc: &Scene, cx: &Ctx, i: usize) -> bool {
    let e = &sc.edges[i];
    let flag = |v: &[bool]| v.get(i).copied().unwrap_or(false);
    flag(&cx.drawn)
        && !flag(&cx.fixed)
        && !flag(&cx.held)
        && !flag(&cx.shaped)
        && e.paint_width.is_none()
        && e.meta.curve.is_none()
        && !crate::ports::note_link(sc, cx, i)
}

/// Whether an end of `e` (its start when `src`) carries a glyph, which
/// needs a head's straight run square to its side.
fn glyph_at(e: &SceneEdge, src: bool) -> bool {
    if src {
        e.ends.start_marker(e.kind).is_some()
    } else {
        e.ends.end_marker(e.kind).is_some()
    }
}

/// The straight run (px) an end needs: a head's and a rounding under a
/// glyph ([`HEAD_LEG`]), else half a rounding.
fn leg_need(glyph: bool) -> f64 {
    if glyph {
        HEAD_LEG
    } else {
        BARE_LEG
    }
}

/// The comb's tooth the path `c` (side coordinates, from the end of
/// edge `e` that is its start when `src`) draws, if it draws one.
fn comb_of(e: &SceneEdge, src: bool, c: Vec<Pt>) -> Option<Comb> {
    if c.len() < 3 {
        return None;
    }
    let (t0, n0) = c[0];
    let depth = c[1].1 - n0;
    let turn = c[2].0;
    if (c[1].0 - t0).abs() > EPS
        || depth < EPS
        || (c[2].1 - c[1].1).abs() > EPS
        || (turn - t0).abs() < EPS
    {
        return None;
    }
    let lo = leg_need(glyph_at(e, src)).max(ROW_CLEAR);
    // On past the turn, outwards: the track may move between the end's
    // own run and that leg's. Anything else pins it.
    let room = (c.len() >= 4 && (c[3].0 - turn).abs() < EPS && c[3].1 - c[2].1 > EPS).then(|| {
        let far = c.len() == 4 && glyph_at(e, !src);
        c[3].1 - n0 - leg_need(far)
    });
    // (A track outside that room — a head's run cut short where the
    // stages found no better — may still move back towards it: its own
    // depth bounds the range on that side.)
    let (lo, hi) = match room {
        Some(hi) if hi - lo > EPS => (lo.min(depth), hi.max(depth)),
        _ => (depth, depth),
    };
    Some(Comb {
        path: c,
        depth,
        turn,
        lo,
        hi,
    })
}

/// Whether `end`'s sides are straight lines its ports may be dealt
/// along: a box (no diamond's or round shape's, no fork bar's, whose
/// branches meet it where they lie) or a frame.
fn straight_side(sc: &Scene, end: EndRef) -> bool {
    use crate::model::Shape;
    match end {
        EndRef::Node(v) => !matches!(
            sc.nodes[v].shape,
            Shape::Diamond
                | Shape::Circle
                | Shape::DoubleCircle
                | Shape::StateStart
                | Shape::StateEnd
                | Shape::ForkBar
        ),
        EndRef::Frame(_) => true,
    }
}

/// An edge's line as drawn, by its corners where it runs on the axes.
fn line_of(e: &SceneEdge) -> Vec<Pt> {
    match ortho_corners(e) {
        Some(c) => c,
        None => edge_polyline(e),
    }
}

/// The two ends of `e` as drawn.
fn ends_of(e: &SceneEdge) -> (Pt, Pt) {
    if e.waypoints.len() >= 2 {
        (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
    } else {
        (e.bezier[0], e.bezier[3])
    }
}

/// The ends on each side this stage combs — (its edge, the edge's
/// start?) — in edge order.
type Sides = BTreeMap<(EndRef, Side), Vec<(usize, bool)>>;

/// What [`keeps`] measures of one pair of lines: (edge, other edge, the
/// edge's line, the other's).
type PairFn<'a> = &'a mut dyn FnMut(usize, usize, &[Pt], &[Pt]);

/// Under a drag ([`Ctx::held`]: a [`crate::scene::route_partial`]
/// frame), the judging one frame may spend ([`Budget`]): pairs of a
/// redrawn edge and a line near the legs that change ([`pair_count`]).
/// A drag frame has 6 ms in all (the drag budget), and the frames that
/// redraw a hub's edges — dense-td's `n16`, bench-75's `gw` dragged
/// across the drawing — take 5 to 5.7 of them before this stage: there
/// a comb judged in full (a dozen long teeth against every line they
/// pass, a tenth of a millisecond and more a deal) put the frame over
/// (B11 review r2 #1: 6.4 ms). So under a drag:
///
/// - the budget shrinks by [`DRAG_WORK_EDGE`] for every edge the frame
///   redraws, and a frame that redraws [`DRAG_WORK`] /
///   [`DRAG_WORK_EDGE`] edges or more is not combed at all;
/// - a side with more than [`DRAG_FAN`] teeth to deal is left as it is
///   (most of a hub's cost, and the same in every frame of its drag);
/// - a side is dealt once, not [`ROUNDS`] times.
///
/// A side left out stays as the stages before drew it, every edge on
/// the axes like any other, until a full [`crate::scene::route`] combs
/// it (what a host calls when the drag ends); a fan of a few teeth
/// under a drag that redraws few edges is combed as there.
const DRAG_WORK: usize = 96;
/// See [`DRAG_WORK`].
const DRAG_WORK_EDGE: usize = 6;
/// See [`DRAG_WORK`].
const DRAG_FAN: usize = 6;

/// What judging may still cost in this call ([`DRAG_WORK`]; no limit
/// outside a drag).
struct Budget(Option<usize>);

impl Budget {
    /// Whether `plan` may be judged, taking its cost if so.
    fn take(&mut self, w: &World, plan: &[Redraw]) -> bool {
        let Some(left) = self.0.as_mut() else {
            return true;
        };
        let cost = pair_count(w, plan);
        if cost > *left {
            return false;
        }
        *left -= cost;
        true
    }
}

/// How many pairs of lines [`keeps`] measures for `plan` at its widest
/// reach.
fn pair_count(w: &World, plan: &[Redraw]) -> usize {
    let mut n = plan.len() * plan.len().saturating_sub(1) / 2;
    for p in plan {
        n += (0..w.lines.len())
            .filter(|&j| {
                w.drawn[j]
                    && p.changed.near(w.bounds[j], CLEAR_GAP)
                    && plan.iter().all(|q| q.edge != j)
            })
            .count();
    }
    n
}

/// Comb every fanned side of every box (see the module docs). Sets
/// `moved[i]` for each edge it redraws, so the label stage looks at
/// their labels again.
pub(crate) fn comb(sc: &mut Scene, cx: &Ctx, moved: &mut [bool]) {
    if sc.edges.len() < PAIR || sc.edges.len() > crate::router::ROUTE_MAX_EDGES {
        return;
    }
    let drawn = |i: usize| cx.drawn.get(i).copied().unwrap_or(false);
    // Under a drag, to the frame's budget (see [`DRAG_WORK`]).
    let dragging = cx.held.iter().any(|&h| h);
    let mut budget = Budget(None);
    if dragging {
        let held = |i: usize| cx.held.get(i).copied().unwrap_or(false);
        let redrawn = (0..sc.edges.len())
            .filter(|&i| drawn(i) && !held(i))
            .count();
        let left = DRAG_WORK.saturating_sub(DRAG_WORK_EDGE * redrawn);
        if left == 0 {
            return;
        }
        budget = Budget(Some(left));
    }
    // The two ends of edge `i` — (the edge's start?, where it ends, the
    // side it lies on) — if it is drawn between two ends.
    let ends = |sc: &Scene, i: usize| -> Option<[(bool, EndRef, Side); 2]> {
        let (from, to) = cx.ends.get(i).copied().flatten()?;
        if from == to || !drawn(i) {
            return None;
        }
        let (p0, p3) = ends_of(&sc.edges[i]);
        let side = |end: EndRef, p: Pt| {
            let r = end_rect(sc, end);
            match end {
                EndRef::Node(v) => crate::ports::outline_side(sc.nodes[v].shape, r, p),
                EndRef::Frame(_) => side_of(r, p),
            }
        };
        Some([(true, from, side(from, p0)), (false, to, side(to, p3))])
    };
    // The sides holding ends this stage may move, [`PAIR`] or more of
    // them (under a drag most edges are kept and few sides do)...
    let mut movable: BTreeMap<(EndRef, Side), usize> = BTreeMap::new();
    for i in 0..sc.edges.len() {
        if !free(sc, cx, i) || ortho_corners(&sc.edges[i]).is_none() {
            continue;
        }
        for (_, end, side) in ends(sc, i).into_iter().flatten() {
            *movable.entry((end, side)).or_default() += 1;
        }
    }
    movable.retain(|k, n| *n >= PAIR && !(dragging && *n > DRAG_FAN) && straight_side(sc, k.0));
    if movable.is_empty() {
        return;
    }
    // ...and every drawn end on each of them.
    let mut sides: Sides = movable.into_keys().map(|k| (k, Vec::new())).collect();
    for i in 0..sc.edges.len() {
        for (src, end, side) in ends(sc, i).into_iter().flatten() {
            if let Some(members) = sides.get_mut(&(end, side)) {
                members.push((i, src));
            }
        }
    }
    // The drawn lines, for judging a side's new comb: read once a side
    // has one to judge.
    let mut world: Option<World> = None;
    for ((end, side), members) in sides {
        let fr = Frame::of(side);
        let r = end_rect(sc, end);
        // The box side's own line, out of the box.
        let line_n = fr.local((r.0, r.1)).1.max(fr.local((r.2, r.3)).1);
        // (Afresh per side: an edge combed at its other end has moved.)
        let mut teeth = teeth_of(sc, cx, &members, fr, line_n);
        // (A side past the frame's budget is left as the stages before
        // drew it, its ports evened neither.)
        let mut afford = true;
        // A deal that leaves the side with fewer crossings is kept, and
        // the side is looked at again as it then stands: with its ports
        // in the order of its tracks, say, a deal that had no answer
        // before may have one. Every deal kept undoes a crossing, so
        // the rounds end; [`ROUNDS`] bounds what a side may cost.
        for _ in 0..if dragging { 1 } else { ROUNDS } {
            let mut tried: Vec<Vec<Redraw>> = Vec::new();
            let mut combed = false;
            // (Looked at once a deal has an answer: most sides are in
            // order.)
            let mut crossed: Option<bool> = None;
            for (order, deal) in DEALS {
                let Some(plan) = plan_side(sc, &teeth, fr, order, deal) else {
                    continue;
                };
                if plan.is_empty() {
                    // In order under a deal that puts its tracks in
                    // order: nothing left to comb. (Under any other, the
                    // deal merely changes nothing here.)
                    if matches!(deal, Deal::Anchored | Deal::Fitted) {
                        break;
                    }
                    continue;
                }
                // Where nothing crosses the legs a deal redraws, none
                // leaves the side with fewer crossings.
                let world = world.get_or_insert_with(|| World::new(sc, cx));
                if !*crossed.get_or_insert_with(|| is_crossed(world, &teeth, fr)) {
                    break;
                }
                if tried.iter().any(|p| same_plan(p, &plan)) {
                    continue;
                }
                afford = budget.take(world, &plan);
                if !afford {
                    break;
                }
                if keeps(sc, world, &plan, &members, true) {
                    apply(sc, world, moved, &plan);
                    combed = true;
                    break;
                }
                tried.push(plan);
            }
            if !combed {
                break;
            }
            teeth = teeth_of(sc, cx, &members, fr, line_n);
        }
        if !afford {
            continue;
        }
        // Then its ports one pitch apart (report B4, §2.4: a hub's ports
        // in one even band), where the stages after the ports stage
        // left them otherwise and nothing gets worse for it.
        if let Some(plan) = plan_even(sc, &teeth, fr) {
            let world = world.get_or_insert_with(|| World::new(sc, cx));
            if budget.take(world, &plan) && keeps(sc, world, &plan, &members, false) {
                apply(sc, world, moved, &plan);
            }
        }
    }
}

/// Redraw the edges of `plan`.
fn apply(sc: &mut Scene, world: &mut World, moved: &mut [bool], plan: &[Redraw]) {
    for r in plan {
        redraw(&mut sc.edges[r.edge], r);
        world.set(r.edge, r.corners.clone());
        // Its label rode its leg to the new place ([`relabel`]) and sits
        // on its line as it did: the label stage looks at it afresh only
        // where it rode a leg that moved (and at any label a stroke now
        // crosses, whoever moved).
        moved[r.edge] |= r.label.is_some();
    }
}

/// The teeth of one side (`members`: each end's edge, and whether it is
/// the edge's start), in port order. `line_n` is the side's own line in
/// its frame `fr`.
fn teeth_of(sc: &Scene, cx: &Ctx, members: &[(usize, bool)], fr: Frame, line_n: f64) -> Vec<Tooth> {
    let mut teeth: Vec<Tooth> = members
        .iter()
        .map(|&(i, src)| {
            let e = &sc.edges[i];
            let (p0, p3) = ends_of(e);
            let p = fr.local(if src { p0 } else { p3 });
            let comb = (free(sc, cx, i) && (p.1 - line_n).abs() < SIDE_SLACK)
                .then_some(e)
                .and_then(ortho_corners)
                .and_then(|c| {
                    let local: Vec<Pt> = if src {
                        c.iter().map(|&q| fr.local(q)).collect()
                    } else {
                        c.iter().rev().map(|&q| fr.local(q)).collect()
                    };
                    comb_of(e, src, local)
                });
            Tooth {
                edge: i,
                src,
                t: p.0,
                comb,
            }
        })
        .collect();
    teeth.sort_by(|a, b| a.t.total_cmp(&b.t).then(a.edge.cmp(&b.edge)));
    teeth
}

/// Draw edge `e` along its new corners.
fn redraw(e: &mut SceneEdge, r: &Redraw) {
    let c = &r.corners;
    let m = c.len();
    if m == 2 {
        // A straight drop: the straight cubic the later stages make of a
        // two-point path ([`crate::edgepass`]'s `straight_cubics`).
        let (a, b) = (c[0], c[1]);
        e.bezier = [
            a,
            (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0),
            (a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0),
            b,
        ];
        e.waypoints = Vec::new();
    } else {
        // As the ortho stage draws a path: the chord through its first
        // and last corner, its corners rounded.
        e.bezier = [c[0], c[1], c[m - 2], c[m - 1]];
        e.waypoints = smooth(c, CORNER_R);
    }
    if let (Some(l), Some(at)) = (e.label.as_mut(), r.label) {
        l.1 = at;
    }
}

/// One redrawn edge of a combed side.
struct Redraw {
    edge: usize,
    /// Its new path's corners (screen), from its start.
    corners: Vec<Pt>,
    /// The legs that change — its stub, its track and the leg on past
    /// its turn, as they were and as they will be; past them the path
    /// stays as it is.
    changed: Changed,
    /// How far (px) its port moves along the side.
    shift: f64,
    /// Its label's new spot, when the label rode a moved leg.
    label: Option<Pt>,
}

/// The legs of a redrawn tooth that change ([`Redraw::changed`]): the
/// bounds of each, as it was and as it will be, and of them all.
#[derive(Clone, Copy)]
struct Changed {
    legs: [Rect; 6],
    count: usize,
    all: Rect,
}

impl Changed {
    /// Of a tooth's first legs as they were (`old`) and as they will be
    /// (`new`, both in the side coordinates of `fr`): those between its
    /// first `corners` corners, as many as the path has.
    fn of(fr: Frame, old: &[Pt], new: &[Pt], corners: usize) -> Changed {
        let mut out = Changed {
            legs: [(0.0, 0.0, 0.0, 0.0); 6],
            count: 0,
            all: (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
        };
        let old = old[..old.len().min(4)].windows(2);
        let new = new[..new.len().min(corners)].windows(2);
        for leg in old.chain(new) {
            let (a, b) = (fr.screen(leg[0]), fr.screen(leg[1]));
            let r = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
            out.legs[out.count] = r;
            out.count += 1;
            let all = out.all;
            out.all = (
                all.0.min(r.0),
                all.1.min(r.1),
                all.2.max(r.2),
                all.3.max(r.3),
            );
        }
        out
    }

    /// Whether a changed leg comes within `gap` of the bounds `r`.
    fn near(&self, r: Rect, gap: f64) -> bool {
        near(self.all, r, gap) && self.legs[..self.count].iter().any(|&l| near(l, r, gap))
    }
}

/// Whether two plans redraw the same edges alike.
fn same_plan(a: &[Redraw], b: &[Redraw]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.edge == y.edge && x.corners == y.corners)
}

/// The comb of one side (`teeth` in port order) under `deal`: the
/// redrawn edges, empty when the side is combed already, `None` when
/// the deal has no answer here — a track would leave its tooth's range,
/// or a step across come out shorter than [`STEP_MIN`].
fn plan_side(
    sc: &Scene,
    teeth: &[Tooth],
    fr: Frame,
    order: Order,
    deal: Deal,
) -> Option<Vec<Redraw>> {
    let movable: Vec<usize> = (0..teeth.len())
        .filter(|&k| teeth[k].comb.is_some())
        .collect();
    let comb = |k: usize| teeth[k].comb.as_ref().expect("movable tooth");
    // The movable ports, low to high, go to the teeth in the order of
    // their turns; or of their rows or tracks: those stepping low from
    // the one nearest the side, then those stepping high down to it.
    let by_turn = |a: usize, b: usize| {
        comb(a)
            .turn
            .total_cmp(&comb(b).turn)
            .then(teeth[a].t.total_cmp(&teeth[b].t))
            .then(teeth[a].edge.cmp(&teeth[b].edge))
    };
    let steps_low = |k: usize| comb(k).turn < teeth[k].t;
    let mut seq = movable.clone();
    match order {
        Order::Turn => seq.sort_by(|&a, &b| by_turn(a, b)),
        Order::Rows => {
            let mut row = vec![0; teeth.len()];
            for low in [true, false] {
                let way: Vec<usize> = movable
                    .iter()
                    .copied()
                    .filter(|&k| steps_low(k) == low)
                    .collect();
                for (k, r) in rows(teeth, &way) {
                    row[k] = r;
                }
            }
            seq.sort_by(|&a, &b| {
                let (la, lb) = (steps_low(a), steps_low(b));
                lb.cmp(&la)
                    .then(if la {
                        row[a].cmp(&row[b])
                    } else {
                        row[b].cmp(&row[a])
                    })
                    .then(by_turn(a, b))
            });
        }
        Order::Track => seq.sort_by(|&a, &b| {
            let (la, lb) = (steps_low(a), steps_low(b));
            let (da, db) = (comb(a).depth, comb(b).depth);
            lb.cmp(&la)
                .then(if la {
                    da.total_cmp(&db)
                } else {
                    db.total_cmp(&da)
                })
                .then(by_turn(a, b))
        }),
    }
    let mut port = vec![0.0; teeth.len()];
    for (j, &k) in seq.iter().enumerate() {
        port[k] = teeth[movable[j]].t;
    }
    // A tooth whose turn is its new port drops straight; one that steps
    // across steps at least [`STEP_MIN`], and under an order that goes
    // by the way it steps, that way still.
    let step = |k: usize| comb(k).turn - port[k];
    let straight = |k: usize| step(k).abs() <= EPS;
    for &k in &movable {
        let c = comb(k);
        if order != Order::Turn && (step(k) < 0.0) != steps_low(k) {
            return None;
        }
        if straight(k) {
            // (Only a tooth that runs on outwards past its turn.)
            if c.pinned() || c.path.len() < 4 {
                return None;
            }
        } else if step(k).abs() < STEP_MIN && (port[k] - teeth[k].t).abs() > EPS {
            return None;
        }
    }
    // The teeth stepping low (their turn below their port), from the
    // track nearest the side: the lowest port first; those stepping
    // high, the highest port first.
    let low: Vec<usize> = seq.iter().copied().filter(|&k| step(k) < -EPS).collect();
    let high: Vec<usize> = seq
        .iter()
        .rev()
        .copied()
        .filter(|&k| step(k) > EPS)
        .collect();
    let mut depth: Vec<f64> = (0..teeth.len())
        .map(|k| teeth[k].comb.as_ref().map_or(0.0, |c| c.depth))
        .collect();
    match deal {
        Deal::Keep => {}
        Deal::Anchored => {
            for seq in [&low, &high] {
                for (k, d) in deal_anchored(teeth, seq)? {
                    depth[k] = d;
                }
            }
        }
        Deal::Fitted => {
            for seq in [&low, &high] {
                for (k, d) in deal_fitted(teeth, seq)? {
                    depth[k] = d;
                }
            }
        }
        Deal::Ways => {
            for seq in [&low, &high] {
                for (k, d) in deal_ways(teeth, seq)? {
                    depth[k] = d;
                }
            }
        }
        Deal::Pooled => {
            for (k, d) in deal_pooled(teeth, &low, &high)? {
                depth[k] = d;
            }
        }
    }
    let mut out = Vec::new();
    for &k in &movable {
        let (tooth, c) = (&teeth[k], comb(k));
        let same_port = (port[k] - tooth.t).abs() <= EPS;
        let same_depth = straight(k) || (depth[k] - c.depth).abs() <= EPS;
        if same_port && same_depth {
            continue;
        }
        if !straight(k) && (depth[k] < c.lo - EPS || depth[k] > c.hi + EPS) {
            return None;
        }
        let e = &sc.edges[tooth.edge];
        let n0 = c.path[0].1;
        let mut local: Vec<Pt> = vec![(port[k], n0)];
        if !straight(k) {
            local.push((port[k], n0 + depth[k]));
            local.push((c.turn, n0 + depth[k]));
        }
        local.extend_from_slice(&c.path[3..]);
        let mut corners: Vec<Pt> = collapse(local.iter().map(|&q| fr.screen(q)).collect());
        if !tooth.src {
            corners.reverse();
        }
        // The label rides the same leg of the new path.
        let label = e
            .label
            .as_ref()
            .and_then(|(_, at, _)| relabel(&c.path, &local, fr, *at, straight(k)));
        let legs = if straight(k) { 2 } else { 4 };
        out.push(Redraw {
            edge: tooth.edge,
            corners,
            changed: Changed::of(fr, &c.path, &local, legs),
            shift: (port[k] - tooth.t).abs(),
            label,
        });
    }
    Some(out)
}

/// The rows of the teeth `way` (stepping one way): `(tooth, row)`, the
/// row nearest the side 0. Read off the tracks from the side outwards,
/// a tooth whose track lies beyond what every tooth before it could
/// take ([`Comb::hi`]: where its path runs on past its turn) starts a
/// new row — a hub's leaves on two rows are stepped to in the gap in
/// front of the hub and in the gap between the rows, and no tooth to
/// the near row has a track in the far gap.
fn rows(teeth: &[Tooth], way: &[usize]) -> Vec<(usize, usize)> {
    let comb = |k: usize| teeth[k].comb.as_ref().expect("movable tooth");
    let mut by_depth = way.to_vec();
    by_depth.sort_by(|&a, &b| {
        comb(a)
            .depth
            .total_cmp(&comb(b).depth)
            .then(teeth[a].edge.cmp(&teeth[b].edge))
    });
    let (mut row, mut reach) = (0, f64::NEG_INFINITY);
    by_depth
        .into_iter()
        .enumerate()
        .map(|(j, k)| {
            let c = comb(k);
            if j > 0 && c.depth > reach + EPS {
                row += 1;
                reach = c.hi;
            } else {
                reach = reach.max(c.hi);
            }
            (k, row)
        })
        .collect()
}

/// The ports of one side (`teeth` in port order) spread one pitch
/// apart between its outermost two, each tooth keeping its place in the
/// order and its track: the redrawn edges, or `None` when the side has
/// an end this stage may not move (it keeps its place, and the others
/// theirs round it), is even already ([`EVEN_SLACK`]), or a tooth's step
/// across would come out shorter than [`STEP_MIN`] or turn round.
fn plan_even(sc: &Scene, teeth: &[Tooth], fr: Frame) -> Option<Vec<Redraw>> {
    let k = teeth.len();
    if k < FAN_MIN || teeth.iter().any(|t| t.comb.is_none()) {
        return None;
    }
    let (lo, hi) = (teeth[0].t, teeth[k - 1].t);
    let slot = |j: usize| lo + (hi - lo) * j as f64 / (k - 1) as f64;
    if (0..k).all(|j| (slot(j) - teeth[j].t).abs() < EVEN_SLACK) {
        return None;
    }
    let mut out = Vec::new();
    for (j, tooth) in teeth.iter().enumerate() {
        let c = tooth.comb.as_ref().expect("checked above");
        let port = slot(j);
        if (port - tooth.t).abs() < EPS {
            continue;
        }
        let (was, now) = (c.turn - tooth.t, c.turn - port);
        if was * now <= 0.0 || now.abs() < STEP_MIN.min(was.abs()) {
            return None;
        }
        let n0 = c.path[0].1;
        let mut local: Vec<Pt> = vec![(port, n0), (port, n0 + c.depth), (c.turn, n0 + c.depth)];
        local.extend_from_slice(&c.path[3..]);
        let mut corners: Vec<Pt> = collapse(local.iter().map(|&q| fr.screen(q)).collect());
        if !tooth.src {
            corners.reverse();
        }
        let label = sc.edges[tooth.edge]
            .label
            .as_ref()
            .and_then(|(_, at, _)| relabel(&c.path, &local, fr, *at, false));
        out.push(Redraw {
            edge: tooth.edge,
            corners,
            changed: Changed::of(fr, &c.path, &local, 4),
            shift: (port - tooth.t).abs(),
            label,
        });
    }
    Some(out)
}

/// [`Deal::Anchored`] of the teeth `seq` (stepping one way, from the
/// track nearest the side): `(tooth, depth)` of each whose track moves.
/// The pinned tracks and the longest run of the others already in order
/// stay; each tooth left over takes a track between its neighbours' — a
/// [`TRACK_PITCH`] beyond the nearer one where the gap between them has
/// the room, else evenly between them, no closer than [`PITCH_MIN`].
/// `None` when the pinned tracks are themselves out of order or a track
/// finds no room in its tooth's range.
fn deal_anchored(teeth: &[Tooth], seq: &[usize]) -> Option<Vec<(usize, f64)>> {
    let comb = |k: usize| teeth[k].comb.as_ref().expect("movable tooth");
    let m = seq.len();
    let depth: Vec<f64> = seq.iter().map(|&k| comb(k).depth).collect();
    let pinned: Vec<bool> = seq.iter().map(|&k| comb(k).pinned()).collect();
    // The pinned tracks keep their order, or the deal has no answer.
    let pins: Vec<usize> = (0..m).filter(|&j| pinned[j]).collect();
    if pins
        .windows(2)
        .any(|w| depth[w[1]] - depth[w[0]] < PITCH_MIN)
    {
        return None;
    }
    // A free tooth may keep its track only between the pinned ones
    // either side of it.
    let fits = |j: usize| {
        pins.iter().all(|&p| {
            if p < j {
                depth[j] - depth[p] >= PITCH_MIN
            } else {
                depth[p] - depth[j] >= PITCH_MIN
            }
        })
    };
    // The longest run in order among those (the earliest of equals).
    let cand: Vec<usize> = (0..m).filter(|&j| !pinned[j] && fits(j)).collect();
    let mut best: Vec<(usize, Option<usize>)> = Vec::with_capacity(cand.len());
    for (a, &j) in cand.iter().enumerate() {
        let mut b = (1, None);
        for (c, &i) in cand[..a].iter().enumerate() {
            if depth[j] - depth[i] >= PITCH_MIN && best[c].0 + 1 > b.0 {
                b = (best[c].0 + 1, Some(c));
            }
        }
        best.push(b);
    }
    let mut keep = pinned.clone();
    let mut at = (0..best.len()).fold(None, |m: Option<usize>, a| match m {
        Some(b) if best[b].0 >= best[a].0 => Some(b),
        _ => Some(a),
    });
    while let Some(a) = at {
        keep[cand[a]] = true;
        at = best[a].1;
    }
    // Each run of teeth left over, between the kept tracks either side.
    let mut out = Vec::new();
    let mut j = 0;
    while j < m {
        if keep[j] {
            j += 1;
            continue;
        }
        let end = (j..m).find(|&q| keep[q]).unwrap_or(m);
        let below = (j > 0).then(|| depth[j - 1]);
        let above = (end < m).then(|| depth[end]);
        let count = (end - j) as f64;
        let (first, pitch) = match (below, above) {
            (Some(a), Some(b)) => {
                let pitch = ((b - a) / (count + 1.0)).min(TRACK_PITCH);
                (a + pitch, pitch)
            }
            (Some(a), None) => (a + TRACK_PITCH, TRACK_PITCH),
            (None, Some(b)) => (b - count * TRACK_PITCH, TRACK_PITCH),
            (None, None) => return None,
        };
        if pitch < PITCH_MIN {
            return None;
        }
        for (q, slot) in (j..end).enumerate() {
            let d = first + q as f64 * pitch;
            let c = comb(seq[slot]);
            if d < c.lo - EPS || d > c.hi + EPS {
                return None;
            }
            out.push((seq[slot], d));
        }
        // (The run's tracks stand in for kept ones from here on.)
        j = end;
    }
    Some(out)
}

/// [`Deal::Fitted`] of the teeth `seq` (stepping one way, from the
/// track nearest the side): `(tooth, depth)` of each whose track moves.
/// Each track stays as near where it lies as the order allows — a
/// [`TRACK_PITCH`] beyond the one before it and as far before the one
/// after it, within its tooth's range. `None` when the ranges leave no
/// room for that.
fn deal_fitted(teeth: &[Tooth], seq: &[usize]) -> Option<Vec<(usize, f64)>> {
    let comb = |k: usize| teeth[k].comb.as_ref().expect("movable tooth");
    // The furthest out each track may lie and leave those after it
    // their room...
    let mut most = vec![f64::INFINITY; seq.len()];
    let mut next = f64::INFINITY;
    for (j, &k) in seq.iter().enumerate().rev() {
        most[j] = comb(k).hi.min(next - TRACK_PITCH);
        next = most[j];
    }
    // ...and from the side outwards, each where it lies or as near as
    // the one before it and that bound allow.
    let mut out = Vec::new();
    let mut last = f64::NEG_INFINITY;
    for (j, &k) in seq.iter().enumerate() {
        let c = comb(k);
        let least = c.lo.max(last + TRACK_PITCH);
        if least > most[j] + EPS {
            return None;
        }
        let d = c.depth.max(least).min(most[j]);
        if (d - c.depth).abs() > EPS {
            out.push((k, d));
        }
        last = d;
    }
    Some(out)
}

/// [`Deal::Ways`] of the teeth `seq` (stepping one way, from the track
/// nearest the side): the tracks its free teeth hold, dealt out again
/// among them nearest the side first. `(tooth, depth)` of each free
/// tooth; `None` when a track leaves its tooth's range, or comes to
/// lie on the wrong side of a pinned one.
fn deal_ways(teeth: &[Tooth], seq: &[usize]) -> Option<Vec<(usize, f64)>> {
    let comb = |k: usize| teeth[k].comb.as_ref().expect("movable tooth");
    let mut pool: Vec<f64> = seq
        .iter()
        .filter(|&&k| !comb(k).pinned())
        .map(|&k| comb(k).depth)
        .collect();
    pool.sort_by(f64::total_cmp);
    let mut pool = pool.into_iter();
    let mut out = Vec::new();
    let mut last = f64::NEG_INFINITY;
    for &k in seq {
        let c = comb(k);
        let d = if c.pinned() {
            c.depth
        } else {
            let d = pool.next()?;
            if d < c.lo - EPS || d > c.hi + EPS {
                return None;
            }
            out.push((k, d));
            d
        };
        if d - last < PITCH_MIN {
            return None;
        }
        last = d;
    }
    Some(out)
}

/// [`Deal::Pooled`]: the tracks the free teeth of `low` and `high`
/// hold, dealt out again nearest the side first — to the two ways in
/// turn, by the depth each tooth held, so a tooth keeps its place among
/// the others where the order allows. `(tooth, depth)` of each free
/// tooth; `None` when a track leaves its tooth's range.
fn deal_pooled(teeth: &[Tooth], low: &[usize], high: &[usize]) -> Option<Vec<(usize, f64)>> {
    let comb = |k: usize| teeth[k].comb.as_ref().expect("movable tooth");
    let free = |seq: &[usize]| -> Vec<usize> {
        seq.iter().copied().filter(|&k| !comb(k).pinned()).collect()
    };
    let (low, high) = (free(low), free(high));
    let mut pool: Vec<f64> = low.iter().chain(&high).map(|&k| comb(k).depth).collect();
    pool.sort_by(f64::total_cmp);
    let mut out = Vec::with_capacity(pool.len());
    let (mut a, mut b) = (0, 0);
    for &d in &pool {
        let take_low = match (low.get(a), high.get(b)) {
            (Some(&x), Some(&y)) => comb(x).depth <= comb(y).depth,
            (Some(_), None) => true,
            _ => false,
        };
        let k = if take_low {
            a += 1;
            low[a - 1]
        } else {
            b += 1;
            high[b - 1]
        };
        if d < comb(k).lo - EPS || d > comb(k).hi + EPS {
            return None;
        }
        out.push((k, d));
    }
    Some(out)
}

/// Drop repeated points and points in the middle of a straight leg.
fn collapse(pts: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out
            .last()
            .is_some_and(|&q: &Pt| (q.0 - p.0).abs() < EPS && (q.1 - p.1).abs() < EPS)
        {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let on_x = (a.0 - b.0).abs() < EPS && (b.0 - p.0).abs() < EPS;
            let on_y = (a.1 - b.1).abs() < EPS && (b.1 - p.1).abs() < EPS;
            if on_x || on_y {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

/// The spot of a label at `at` (screen) on the leg of the new comb
/// `new` (side coordinates) that matches the leg of `old` it rides: the
/// stub, the track or the run on past the turn, its place along the
/// leg kept where the new leg reaches it. `None`: it rides no moved leg.
fn relabel(old: &[Pt], new: &[Pt], fr: Frame, at: Pt, straight: bool) -> Option<Pt> {
    let q = fr.local(at);
    let on = |a: Pt, b: Pt| {
        let (lo_t, hi_t) = (a.0.min(b.0), a.0.max(b.0));
        let (lo_n, hi_n) = (a.1.min(b.1), a.1.max(b.1));
        q.0 >= lo_t - 1.0 && q.0 <= hi_t + 1.0 && q.1 >= lo_n - 1.0 && q.1 <= hi_n + 1.0
    };
    let legs = (old.len() - 1).min(3);
    let leg = (0..legs).find(|&l| on(old[l], old[l + 1]))?;
    // The new path's matching leg: a straight drop's one leg stands for
    // all three.
    let (a, b) = if straight {
        (new[0], new[1])
    } else {
        (new[leg], new[leg + 1])
    };
    let clamp = |v: f64, x: f64, y: f64| v.max(x.min(y)).min(x.max(y));
    let spot = if (a.0 - b.0).abs() < EPS {
        (a.0, clamp(q.1, a.1, b.1))
    } else {
        (clamp(q.0, a.0, b.0), a.1)
    };
    Some(fr.screen(spot))
}

/// What a side's new comb is judged against: every edge's line as
/// drawn (its corners where it runs on the axes) with its bounds, and
/// the boxes, borders, chips and labels it must not newly hit.
struct World {
    lines: Vec<Vec<Pt>>,
    bounds: Vec<Rect>,
    drawn: Vec<bool>,
    /// Each edge's ends ([`Ctx::ends`]).
    ends: Vec<Option<(EndRef, EndRef)>>,
    /// The frames' title chips.
    heads: Vec<Rect>,
}

/// The bounds of a line.
fn bounds_of(l: &[Pt]) -> Rect {
    let mut r = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in l {
        r = (r.0.min(p.0), r.1.min(p.1), r.2.max(p.0), r.3.max(p.1));
    }
    r
}

/// Whether two bounds come within `gap` of each other.
fn near(a: Rect, b: Rect, gap: f64) -> bool {
    a.0 <= b.2 + gap && b.0 <= a.2 + gap && a.1 <= b.3 + gap && b.1 <= a.3 + gap
}

impl World {
    fn new(sc: &Scene, cx: &Ctx) -> World {
        let lines: Vec<Vec<Pt>> = sc.edges.iter().map(line_of).collect();
        let bounds = lines.iter().map(|l| bounds_of(l)).collect();
        let drawn = (0..sc.edges.len())
            .map(|i| cx.drawn.get(i).copied().unwrap_or(false))
            .collect();
        let heads = (0..sc.clusters.len())
            .map(|ci| crate::scene::title_head(&sc.clusters, ci, f64::INFINITY))
            .collect();
        let ends = (0..sc.edges.len())
            .map(|i| cx.ends.get(i).copied().flatten())
            .collect();
        World {
            lines,
            bounds,
            drawn,
            ends,
            heads,
        }
    }

    /// Edge `i` is drawn along `line` from here on.
    fn set(&mut self, i: usize, line: Vec<Pt>) {
        self.bounds[i] = bounds_of(&line);
        self.lines[i] = line;
    }
}

/// Whether any line crosses the legs a deal redraws ([`Redraw::changed`])
/// of a tooth of `teeth` (in the side coordinates of `fr`).
fn is_crossed(w: &World, teeth: &[Tooth], fr: Frame) -> bool {
    teeth.iter().any(|t| {
        let Some(c) = &t.comb else {
            return false;
        };
        let legs: Vec<Pt> = c.path[..c.path.len().min(4)]
            .iter()
            .map(|&q| fr.screen(q))
            .collect();
        let bb = bounds_of(&legs);
        (0..w.lines.len()).any(|j| {
            j != t.edge
                && w.drawn[j]
                && near(bb, w.bounds[j], 0.0)
                && cross_count(&legs, &w.lines[j]) > 0
        })
    })
}

/// Whether the redrawn edges `plan` cross fewer lines than they do now
/// (no more, unless `fewer`) and hit nothing more: boxes (but their
/// own, and no nearer one than they came), frame borders, title chips
/// and other edges' labels, nor run on top of another edge, closer
/// beside one than the spacing clear, or along a frame's border for
/// longer than they did — what the quality yardstick counts as defects,
/// by its own clearances ([`crate::clearance`]). `fan` lists the ends
/// of the side being combed (each end's edge, and whether it is the
/// edge's start): two of its stubs side by side are as far apart as the
/// ports stage put the side's ports, in whatever order the teeth take
/// them, and are not held against the comb ([`stub_of`]) — unless they
/// come to lie on top of each other.
fn keeps(sc: &Scene, w: &World, plan: &[Redraw], fan: &[(usize, bool)], fewer: bool) -> bool {
    // The changed edges as (edge, line now, bounds now, the legs that
    // change). What lies clear of those legs, before and after, an edge
    // meets alike either way: every count below is taken of the two
    // states alike, so it is left out of both.
    type State<'a> = [(usize, &'a [Pt], Rect, &'a Changed)];
    let new: Vec<(usize, &[Pt], Rect, &Changed)> = plan
        .iter()
        .map(|p| {
            (
                p.edge,
                p.corners.as_slice(),
                bounds_of(&p.corners),
                &p.changed,
            )
        })
        .collect();
    let old: Vec<(usize, &[Pt], Rect, &Changed)> = plan
        .iter()
        .map(|p| {
            (
                p.edge,
                w.lines[p.edge].as_slice(),
                w.bounds[p.edge],
                &p.changed,
            )
        })
        .collect();
    // Every pair of a changed edge and another drawn edge, each pair
    // once, the other edge as drawn in the same state.
    let pairs = |state: &State, gap: f64, f: PairFn| {
        for (k, &(i, a, ba, ca)) in state.iter().enumerate() {
            for &(j, b, bb, _) in &state[k + 1..] {
                if near(ba, bb, gap) {
                    f(i, j, a, b);
                }
            }
            for j in 0..w.lines.len() {
                if w.drawn[j] && ca.near(w.bounds[j], gap) && plan.iter().all(|p| p.edge != j) {
                    f(i, j, a, &w.lines[j]);
                }
            }
        }
    };
    let crossings = |state: &State| {
        let mut n = 0;
        pairs(state, 0.0, &mut |_, _, a, b| n += cross_count(a, b));
        n
    };
    let (was, now) = (crossings(&old), crossings(&new));
    if now > was || (fewer && now == was) {
        return false;
    }
    // Boxes, frame borders, title chips and labels.
    let hits = |state: &State| {
        let mut n = 0;
        for &(i, line, _, ch) in state {
            let own = w.ends[i];
            for (v, nd) in sc.nodes.iter().enumerate() {
                let r = (
                    nd.x - nd.w / 2.0,
                    nd.y - nd.h / 2.0,
                    nd.x + nd.w / 2.0,
                    nd.y + nd.h / 2.0,
                );
                let end = EndRef::Node(v);
                if own.is_some_and(|(a, b)| a == end || b == end) || !ch.near(r, NODE_SEP) {
                    continue;
                }
                // Through or grazing a box it does not end on (the
                // yardstick's crossed node), cramped beside it, or
                // inside the room the stages keep round one: each a
                // step worse than the next.
                let gap = line
                    .windows(2)
                    .map(|s| seg_gap(s[0], s[1], r))
                    .fold(f64::INFINITY, f64::min);
                n += if gap < NODE_CLEARANCE {
                    NODE_HIT
                } else if gap < CRAMP_NODE_GAP {
                    NODE_CRAMP
                } else {
                    usize::from(gap < NODE_SEP)
                };
            }
            for c in &sc.clusters {
                let r = (c.x, c.y, c.x + c.w, c.y + c.h);
                if ch.near(r, 0.0) {
                    n += NODE_CRAMP
                        * line
                            .windows(2)
                            .map(|s| border_crossings(s[0], s[1], r))
                            .sum::<usize>();
                }
            }
            for &h in &w.heads {
                if ch.near(h, 0.0) {
                    n += line
                        .windows(2)
                        .filter(|s| seg_hits_rect(s[0], s[1], h))
                        .count();
                }
            }
        }
        n
    };
    // Labels over lines: a changed line through any label, any line
    // through a changed edge's label (moved with its leg) — how often
    // each label is crossed, in edge order.
    let labels = |state: &State, after: bool| -> Vec<usize> {
        let mut out = Vec::new();
        for (j, e) in sc.edges.iter().enumerate() {
            let Some((_, c, lw)) = &e.label else {
                continue;
            };
            let own = plan.iter().find(|p| p.edge == j);
            let c = own.filter(|_| after).and_then(|p| p.label).unwrap_or(*c);
            let r = label_box(e, c, *lw);
            let through = |l: &[Pt]| {
                l.windows(2)
                    .filter(|s| seg_hits_rect(s[0], s[1], r))
                    .count()
            };
            let mut n = 0;
            if own.is_some() {
                for (i, l) in w.lines.iter().enumerate() {
                    if i == j || !w.drawn[i] {
                        continue;
                    }
                    let (l, bb) = state
                        .iter()
                        .find(|s| s.0 == i)
                        .map_or((l.as_slice(), w.bounds[i]), |s| (s.1, s.2));
                    if near(bb, r, 0.0) {
                        n += through(l);
                    }
                }
            } else {
                // (By the whole line's bounds: a label counts as it was
                // once any line crosses it, see below.)
                for &(_, line, bb, _) in state {
                    if near(bb, r, 0.0) {
                        n += through(line);
                    }
                }
            }
            out.push(n);
        }
        out
    };
    // (A label some line crosses already is placed afresh by the label
    // stage whatever the comb does — it looks again at every label a
    // stroke crosses — so its crossings count as they were: a comb that
    // undoes three crossings is not refused for one stroke more through
    // a label that is about to move.)
    let (lbl_was, lbl_now) = (labels(&old, false), labels(&new, true));
    let lbl_now: usize = lbl_was
        .iter()
        .zip(&lbl_now)
        .map(|(&was, &now)| if was > 0 { was } else { now })
        .sum();
    if hits(&new) + lbl_now > hits(&old) + lbl_was.iter().sum::<usize>() {
        return false;
    }
    // Nor do the changed lines run beside another closer than two
    // strokes read as two ([`EDGE_SPACING`]) — or closer than the spacing
    // clear ([`CLEAR_GAP`]) — for longer than they did: pairs over the
    // yardstick's least run, and their length, for each of the two.
    const GAPS: [(f64, f64); 2] = [(EDGE_SPACING, MIN_OVERLAP_RUN), (CLEAR_GAP, TIGHT_RUN_MIN)];
    let close = |state: &State| {
        let mut out = [(0usize, 0.0); 2];
        pairs(state, CLEAR_GAP, &mut |i, j, a, b| {
            // (Most pairs whose bounds come that near run nowhere beside
            // each other: nothing to measure. A changed edge's line is
            // all axis legs, so what runs beside it, it runs beside.)
            if !comes_beside(a, b, CLEAR_GAP) {
                return;
            }
            // (Round an end the two share they run in side by side, as
            // its ports space them: the yardstick's port zone.)
            let mut zones = [(0.0, 0.0, 0.0, 0.0); 2];
            let mut shared = 0;
            if let (Some((a0, a1)), Some((b0, b1))) = (w.ends[i], w.ends[j]) {
                for x in [a0, a1] {
                    if x == b0 || x == b1 {
                        let r = end_rect(sc, x);
                        zones[shared] = (
                            r.0 - PORT_ZONE,
                            r.1 - PORT_ZONE,
                            r.2 + PORT_ZONE,
                            r.3 + PORT_ZONE,
                        );
                        shared += 1;
                    }
                }
            }
            let zones = &zones[..shared];
            for (k, (gap, least)) in GAPS.into_iter().enumerate() {
                // (Beside, not on top of each other: less what two
                // stubs of the combed side run side by side. A fan whose
                // ports stand closer than the spacing clear has them run
                // so in any order, and for longer the better its teeth
                // nest — nested, each stub runs the whole length of the
                // shorter one beside it. Held against the comb, no such
                // fan could be put in order.)
                let stubs = match (stub_of(fan, i, a), stub_of(fan, j, b)) {
                    (Some(sa), Some(sb)) if gap > EDGE_SPACING => {
                        beside(&sa, &sb, gap, zones).max(beside(&sb, &sa, gap, zones))
                    }
                    _ => 0.0,
                };
                let l = beside(a, b, gap, zones).max(beside(b, a, gap, zones)) - stubs;
                out[k].0 += usize::from(l > least);
                out[k].1 += l.max(0.0);
            }
        });
        out
    };
    let (was, now) = (close(&old), close(&new));
    // (A tooth whose port moves along its side has its track that much
    // longer or shorter, and runs that much longer beside whatever line
    // it ran beside within the spacing clear: that is no new pair of
    // lines side by side, and not held against the comb — fan073 of the
    // hub-fan set, where two ports changing places to undo a crossing
    // lengthened by their pitch the run of two tracks 10 px apart in
    // the gap between the leaves' rows. On top of each other is another
    // matter, whatever the length.)
    let shifts: f64 = plan.iter().map(|p| p.shift).sum();
    let slack = [1.0, 1.0 + shifts];
    if (0..GAPS.len()).any(|k| now[k].0 > was[k].0 || now[k].1 > was[k].1 + slack[k]) {
        return false;
    }
    // Nor along a frame's border.
    let along = |state: &State| -> f64 {
        state
            .iter()
            .map(|&(_, line, _, ch)| {
                sc.clusters
                    .iter()
                    .map(|c| (c.x, c.y, c.x + c.w, c.y + c.h))
                    .filter(|&r| ch.near(r, CLEAR_GAP))
                    .map(|r| border_run(line, r))
                    .sum::<f64>()
            })
            .sum()
    };
    along(&new) <= along(&old) + 1.0
}

/// The stub of edge `i`'s `line` on the combed side `fan` (the side's
/// ends, see [`keeps`]) — its first leg out of the side — if the edge
/// ends there. (An edge with both ends on the side: its start's.)
fn stub_of(fan: &[(usize, bool)], i: usize, line: &[Pt]) -> Option<[Pt; 2]> {
    let &(_, src) = fan.iter().find(|m| m.0 == i)?;
    let n = line.len();
    if n < 2 {
        return None;
    }
    let at = |k: usize| if src { line[k] } else { line[n - 1 - k] };
    // As far as the line runs straight on from its end (a straight
    // drop's line is its samples, all one leg).
    let (p, q) = (at(0), at(1));
    let len = (q.0 - p.0).hypot(q.1 - p.1).max(EPS);
    let on = |r: Pt| {
        ((r.0 - p.0) * (q.1 - p.1) - (r.1 - p.1) * (q.0 - p.0)).abs() < SIDE_SLACK * len
            && (r.0 - q.0) * (q.0 - p.0) + (r.1 - q.1) * (q.1 - p.1) >= -EPS
    };
    let last = (2..n).take_while(|&k| on(at(k))).last().unwrap_or(1);
    Some([p, at(last)])
}

/// What a line through (or grazing) a box it does not end on weighs in
/// [`keeps`], and one cramped beside it, against one merely inside the
/// room the stages keep round a box.
const NODE_HIT: usize = 100;
const NODE_CRAMP: usize = 10;

/// The gap (px) between segment `a`-`b` and the rect `r` (0 where they
/// meet). A slanted segment is measured by its ends and the rect's
/// corners, which is exact unless it crosses the rect (then 0).
fn seg_gap(a: Pt, b: Pt, r: Rect) -> f64 {
    if seg_hits_rect(a, b, r) {
        return 0.0;
    }
    let to_rect = |p: Pt| {
        (r.0 - p.0)
            .max(p.0 - r.2)
            .max(0.0)
            .hypot((r.1 - p.1).max(p.1 - r.3).max(0.0))
    };
    let to_seg = |p: Pt| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l2 = dx * dx + dy * dy;
        let t = if l2 < EPS {
            0.0
        } else {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
        };
        (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
    };
    [(r.0, r.1), (r.2, r.1), (r.2, r.3), (r.0, r.3)]
        .into_iter()
        .map(to_seg)
        .chain([to_rect(a), to_rect(b)])
        .fold(f64::INFINITY, f64::min)
}

/// How long (px) the axis legs of `line` run along a side of the frame
/// `r` within [`CLEAR_GAP`] of it (inside or out), over the side's span.
fn border_run(line: &[Pt], r: Rect) -> f64 {
    let mut len = 0.0;
    for s in line.windows(2) {
        let Some((vertical, c, lo, hi)) = axis_leg(s[0], s[1]) else {
            continue;
        };
        let (sides, span) = if vertical {
            ([r.0, r.2], (r.1, r.3))
        } else {
            ([r.1, r.3], (r.0, r.2))
        };
        if sides.iter().any(|&side| (c - side).abs() < CLEAR_GAP) {
            len += (hi.min(span.1) - lo.max(span.0)).max(0.0);
        }
    }
    len
}

/// How long (px) the axis legs of `a` run beside `b` closer than `gap`
/// (on top of it included), outside the `zones`: beside an axis leg of
/// `b` on a line within `gap`, or under a slanted stretch of it (a
/// curve's sample) while that keeps within `gap` of the leg's line.
fn beside(a: &[Pt], b: &[Pt], gap: f64, zones: &[Rect]) -> f64 {
    let mut len = 0.0;
    runs_beside(a, b, gap, |vertical, c, lo, hi| {
        len += outside(vertical, c, lo, hi, zones);
        true
    });
    len
}

/// Whether an axis leg of `a` runs beside `b` closer than `gap` at all
/// ([`beside`], with no zones, is more than nothing).
fn comes_beside(a: &[Pt], b: &[Pt], gap: f64) -> bool {
    let mut any = false;
    runs_beside(a, b, gap, |_, _, _, _| {
        any = true;
        false
    });
    any
}

/// The runs [`beside`] measures, each handed to `f` as (the leg is
/// vertical?, its line, the run's span along it) until `f` says stop.
fn runs_beside(a: &[Pt], b: &[Pt], gap: f64, mut f: impl FnMut(bool, f64, f64, f64) -> bool) {
    for u in a.windows(2) {
        let Some((au, cu, u0, u1)) = axis_leg(u[0], u[1]) else {
            continue;
        };
        for v in b.windows(2) {
            // In the leg's own coordinates: (along it, across it).
            let flip = |p: Pt| if au { (p.1, p.0) } else { p };
            let (p, q) = (flip(v[0]), flip(v[1]));
            let (lo, hi) = if (p.1 - q.1).abs() < EPS {
                if (p.1 - cu).abs() >= gap {
                    continue;
                }
                (p.0.min(q.0), p.0.max(q.0))
            } else {
                // Where the stretch lies within `gap` of the leg's line.
                let t = |c: f64| ((c - p.1) / (q.1 - p.1)).clamp(0.0, 1.0);
                let (t0, t1) = (t(cu - gap), t(cu + gap));
                let at = |t: f64| p.0 + (q.0 - p.0) * t;
                (at(t0).min(at(t1)), at(t0).max(at(t1)))
            };
            let (lo, hi) = (lo.max(u0), hi.min(u1));
            if hi > lo && !f(au, cu, lo, hi) {
                return;
            }
        }
    }
}

/// The length of the run `lo..hi` along the axis line at `c` (a
/// vertical one when `vertical`) that lies in none of the `zones` (two
/// at most: the ends two edges share).
fn outside(vertical: bool, c: f64, lo: f64, hi: f64, zones: &[Rect]) -> f64 {
    // (Each zone cuts a run in two at most.)
    let mut runs = [(lo, hi), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)];
    let mut n = 1;
    for z in zones.iter().take(2) {
        let (c0, c1, z0, z1) = if vertical {
            (z.0, z.2, z.1, z.3)
        } else {
            (z.1, z.3, z.0, z.2)
        };
        if c < c0 || c > c1 {
            continue;
        }
        let mut cut = [(0.0, 0.0); 4];
        let mut m = 0;
        for &(a, b) in &runs[..n] {
            for r in [(a, b.min(z0)), (a.max(z1), b)] {
                if r.1 > r.0 {
                    cut[m] = r;
                    m += 1;
                }
            }
        }
        (runs, n) = (cut, m);
    }
    runs[..n].iter().map(|r| r.1 - r.0).sum()
}

/// An axis leg as (vertical?, its line, span low, span high).
fn axis_leg(p: Pt, q: Pt) -> Option<(bool, f64, f64, f64)> {
    if (p.0 - q.0).abs() < EPS && (p.1 - q.1).abs() > EPS {
        Some((true, p.0, p.1.min(q.1), p.1.max(q.1)))
    } else if (p.1 - q.1).abs() < EPS && (p.0 - q.0).abs() > EPS {
        Some((false, p.1, p.0.min(q.0), p.0.max(q.0)))
    } else {
        None
    }
}

/// The box of edge `e`'s label centred at `c`, `w` wide.
pub(crate) fn label_box(e: &SceneEdge, c: Pt, w: f64) -> Rect {
    let text = e.label.as_ref().map_or("", |l| l.0.as_str());
    let lines = 1 + text.matches("<br").count() + text.matches('\n').count();
    let h = crate::layout::LABEL_BOX_H / 2.0 + crate::layout::LINE_H * (lines - 1) as f64 / 2.0;
    (c.0 - w / 2.0, c.1 - h, c.0 + w / 2.0, c.1 + h)
}

/// Proper crossings of two polylines.
fn cross_count(a: &[Pt], b: &[Pt]) -> usize {
    let mut n = 0;
    for u in a.windows(2) {
        let (x0, x1) = (u[0].0.min(u[1].0), u[0].0.max(u[1].0));
        let (y0, y1) = (u[0].1.min(u[1].1), u[0].1.max(u[1].1));
        for v in b.windows(2) {
            // (Two segments whose bounds lie apart cross nowhere.)
            if v[0].0.max(v[1].0) < x0
                || v[0].0.min(v[1].0) > x1
                || v[0].1.max(v[1].1) < y0
                || v[0].1.min(v[1].1) > y1
            {
                continue;
            }
            n += usize::from(segs_cross(u[0], u[1], v[0], v[1]));
        }
    }
    n
}

/// Whether segments `a`-`b` and `c`-`d` properly cross.
fn segs_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let o = |p: Pt, q: Pt, r: Pt| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2) = (o(c, d, a), o(c, d, b));
    let (d3, d4) = (o(a, b, c), o(a, b, d));
    d1 * d2 < -EPS && d3 * d4 < -EPS
}

/// Whether segment `a`-`b` meets the rect `(x0, y0, x1, y1)`.
fn seg_hits_rect(a: Pt, b: Pt, r: Rect) -> bool {
    if r.2 <= r.0 || r.3 <= r.1 {
        return false;
    }
    // Liang-Barsky clip.
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - r.0),
        (dx, r.2 - a.0),
        (-dy, a.1 - r.1),
        (dy, r.3 - a.1),
    ] {
        if p.abs() < EPS {
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
        }
    }
    t0 < t1
}

/// How many times segment `a`-`b` crosses the border of rect `r`.
fn border_crossings(a: Pt, b: Pt, r: Rect) -> usize {
    let corners = [(r.0, r.1), (r.2, r.1), (r.2, r.3), (r.0, r.3)];
    (0..4)
        .filter(|&k| segs_cross(a, b, corners[k], corners[(k + 1) % 4]))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::scene::{route, scene};

    fn fixture(name: &str) -> crate::model::Graph {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
        parse(&std::fs::read_to_string(format!("{dir}/{name}.mmd")).unwrap()).unwrap()
    }

    /// The fan on `side` of node `id`: how often two of its edges cross
    /// each other, and its ports along the side in order.
    fn fan(sc: &Scene, id: &str, side: Side) -> (usize, Vec<f64>) {
        let n = sc.nodes.iter().find(|n| n.id == id).expect("node");
        let r = (
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.x + n.w / 2.0,
            n.y + n.h / 2.0,
        );
        let fr = Frame::of(side);
        let mut ports = Vec::new();
        let mut lines: Vec<Vec<Pt>> = Vec::new();
        for e in sc.edges.iter().filter(|e| e.from != e.to) {
            let (p0, p3) = ends_of(e);
            for (end, p) in [(&e.from, p0), (&e.to, p3)] {
                if end == id && side_of(r, p) == side {
                    ports.push(fr.local(p).0);
                    lines.push(line_of(e));
                }
            }
        }
        ports.sort_by(f64::total_cmp);
        let mut x = 0;
        for a in 0..lines.len() {
            for b in a + 1..lines.len() {
                x += cross_count(&lines[a], &lines[b]);
            }
        }
        (x, ports)
    }

    /// A tooth at port `t` out of a side at n = 0 whose path runs
    /// through the side-coordinate corners `c` (from its end).
    fn tooth(sc: &Scene, edge: usize, c: &[Pt]) -> Tooth {
        Tooth {
            edge,
            src: true,
            t: c[0].0,
            comb: comb_of(&sc.edges[edge], true, c.to_vec()),
        }
    }

    /// A scene with `n` plain edges to hang teeth on (`---`: no heads).
    fn plain(n: usize) -> Scene {
        let src: String = std::iter::once("flowchart TD".to_string())
            .chain((0..n).map(|k| format!("a{k} --- b{k}")))
            .collect::<Vec<_>>()
            .join("\n");
        scene(&parse(&src).unwrap())
    }

    #[test]
    fn a_tooth_is_a_stub_a_track_and_a_turn() {
        let sc = plain(1);
        // A Z on past its turn: its track may move between the end's own
        // run and the far leg's.
        let z = tooth(
            &sc,
            0,
            &[(0.0, 0.0), (0.0, 40.0), (100.0, 40.0), (100.0, 120.0)],
        );
        let c = z.comb.as_ref().expect("a tooth");
        assert_eq!((c.depth, c.turn), (40.0, 100.0));
        assert!((c.lo - ROW_CLEAR).abs() < 1e-9 && (c.hi - (120.0 - BARE_LEG)).abs() < 1e-9);
        assert!(!c.pinned());
        // An L's track runs into the far end: pinned, its port still free.
        let l = tooth(&sc, 0, &[(0.0, 0.0), (0.0, 40.0), (100.0, 40.0)]);
        assert!(l.comb.as_ref().is_some_and(Comb::pinned));
        // A path heading back towards the side past its turn: pinned too.
        let back = tooth(
            &sc,
            0,
            &[(0.0, 0.0), (0.0, 40.0), (100.0, 40.0), (100.0, 28.0)],
        );
        assert!(back.comb.as_ref().is_some_and(Comb::pinned));
        // A track beyond its room — past the turn the far leg has less
        // than its end's run left — may still move back towards it: its
        // own depth bounds the range (B11 review r1 #4; before, such a
        // tooth was pinned and a fan round it combed by its ports alone).
        let deep = tooth(
            &sc,
            0,
            &[(0.0, 0.0), (0.0, 116.0), (100.0, 116.0), (100.0, 120.0)],
        );
        let c = deep.comb.as_ref().expect("a tooth");
        assert!((c.lo - ROW_CLEAR).abs() < 1e-9 && (c.hi - 116.0).abs() < 1e-9);
        assert!(!c.pinned());
        // A straight drop, and a path that leaves along its side: no teeth.
        assert!(tooth(&sc, 0, &[(0.0, 0.0), (0.0, 80.0)]).comb.is_none());
        let along = [(0.0, 0.0), (60.0, 0.0), (60.0, 80.0)];
        assert!(tooth(&sc, 0, &along).comb.is_none());
    }

    #[test]
    fn a_side_takes_its_ports_in_the_order_of_its_turns() {
        // Three teeth stepping high whose tracks nest (the furthest
        // turn nearest the side) and whose ports do not: the ports
        // change hands, the tracks stay.
        let sc = plain(3);
        let z = |t: f64, d: f64, turn: f64| vec![(t, 0.0), (t, d), (turn, d), (turn, 200.0)];
        let teeth = vec![
            tooth(&sc, 0, &z(0.0, 20.0, 300.0)),
            tooth(&sc, 1, &z(14.0, 60.0, 100.0)),
            tooth(&sc, 2, &z(28.0, 40.0, 200.0)),
        ];
        let fr = Frame::of(Side::Bottom);
        let plan = plan_side(&sc, &teeth, fr, Order::Turn, Deal::Anchored).expect("a plan");
        let port = |edge: usize| {
            plan.iter()
                .find(|r| r.edge == edge)
                .map_or(teeth[edge].t, |r| r.corners[0].0)
        };
        assert_eq!((port(1), port(2), port(0)), (0.0, 14.0, 28.0));
        // Each on the track it held.
        for r in &plan {
            let d = teeth[r.edge].comb.as_ref().unwrap().depth;
            assert_eq!(r.corners[1].1, d, "edge {}", r.edge);
        }
        // Nested: no tooth crosses another.
        let line = |edge: usize| {
            plan.iter()
                .find(|r| r.edge == edge)
                .map(|r| r.corners.clone())
                .unwrap()
        };
        for a in 0..3 {
            for b in a + 1..3 {
                assert_eq!(cross_count(&line(a), &line(b)), 0, "{a} x {b}");
            }
        }
        // Combed, the side is left alone.
        let again: Vec<Tooth> = (0..3)
            .map(|k| {
                let c: Vec<Pt> = line(k).iter().map(|&q| fr.local(q)).collect();
                tooth(&sc, k, &c)
            })
            .collect();
        let mut again = again;
        again.sort_by(|a, b| a.t.total_cmp(&b.t));
        let settled = plan_side(&sc, &again, fr, Order::Turn, Deal::Anchored);
        assert!(settled.is_some_and(|p| p.is_empty()));
    }

    #[test]
    fn a_track_out_of_order_gets_one_between_its_neighbours() {
        // Stepping low from the highest port on the track nearest the
        // side, across the stubs of the two beside it (dense-td's
        // `n16 -> n31`): it gets a track of its own past theirs, and they
        // keep theirs.
        let sc = plain(3);
        let z = |t: f64, d: f64, turn: f64| vec![(t, 0.0), (t, d), (turn, d), (turn, 300.0)];
        let teeth = vec![
            tooth(&sc, 0, &z(0.0, 40.0, -300.0)),
            tooth(&sc, 1, &z(14.0, 60.0, -200.0)),
            tooth(&sc, 2, &z(28.0, 14.5, -10.0)),
        ];
        let low: Vec<usize> = vec![0, 1, 2];
        assert_eq!(
            deal_anchored(&teeth, &low),
            Some(vec![(2, 60.0 + TRACK_PITCH)])
        );
        // Dealt out again instead, every tooth moves one track down.
        assert_eq!(
            deal_pooled(&teeth, &low, &[]),
            Some(vec![(0, 14.5), (1, 40.0), (2, 60.0)])
        );
        // Between two kept tracks: a pitch past the nearer one where the
        // gap has the room, midway where it has not, none under the
        // least pitch.
        let between = |gap: f64| {
            let teeth = vec![
                tooth(&sc, 0, &z(0.0, 40.0, -300.0)),
                tooth(&sc, 1, &z(14.0, 14.5, -200.0)),
                tooth(&sc, 2, &z(28.0, 40.0 + gap, -100.0)),
            ];
            deal_anchored(&teeth, &low)
        };
        assert_eq!(between(40.0), Some(vec![(1, 40.0 + TRACK_PITCH)]));
        assert_eq!(between(20.0), Some(vec![(1, 50.0)]));
        assert_eq!(between(2.0 * PITCH_MIN - 1.0), None);
        // A pinned track (an L's) stays; the free tooth past it in the
        // order gets a track beyond it, never one short of it.
        let teeth = vec![
            tooth(&sc, 0, &[(0.0, 0.0), (0.0, 60.0), (-300.0, 60.0)]),
            tooth(&sc, 1, &z(14.0, 40.0, -200.0)),
        ];
        assert_eq!(
            deal_anchored(&teeth, &[0, 1]),
            Some(vec![(1, 60.0 + TRACK_PITCH)])
        );
    }

    #[test]
    fn ports_spread_one_pitch_apart_where_every_end_may_move() {
        let sc = plain(3);
        let z = |t: f64, d: f64, turn: f64| vec![(t, 0.0), (t, d), (turn, d), (turn, 200.0)];
        let fr = Frame::of(Side::Bottom);
        let teeth = vec![
            tooth(&sc, 0, &z(0.0, 60.0, 100.0)),
            tooth(&sc, 1, &z(22.0, 40.0, 200.0)),
            tooth(&sc, 2, &z(36.0, 20.0, 300.0)),
        ];
        let plan = plan_even(&sc, &teeth, fr).expect("uneven");
        assert_eq!(plan.len(), 1);
        assert_eq!((plan[0].edge, plan[0].corners[0].0), (1, 18.0));
        assert_eq!(plan[0].corners[1], (18.0, 40.0), "its track kept");
        // Even already, or holding an end that may not move: left alone.
        let even = vec![
            tooth(&sc, 0, &z(0.0, 60.0, 100.0)),
            tooth(&sc, 1, &z(18.0, 40.0, 200.0)),
            tooth(&sc, 2, &z(36.0, 20.0, 300.0)),
        ];
        assert!(plan_even(&sc, &even, fr).is_none());
        let pinned = vec![
            tooth(&sc, 0, &z(0.0, 60.0, 100.0)),
            tooth(&sc, 1, &[(22.0, 0.0), (22.0, 80.0)]),
            tooth(&sc, 2, &z(36.0, 20.0, 300.0)),
        ];
        assert!(plan_even(&sc, &pinned, fr).is_none());
    }

    #[test]
    fn a_hubs_fan_is_a_comb_in_scene_and_route() {
        // Report B4: dense-td's hub `n16` — twelve edges out of its
        // bottom side and one in — crossed itself 14 times under the
        // box (8 in `route()`); its ports stand one pitch apart.
        let g = fixture("dense-td");
        let sc = scene(&g);
        let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (mode, s) in [("scene", &sc), ("route", &route(&g, &at))] {
            let (x, ports) = fan(s, "n16", Side::Bottom);
            assert!(ports.len() >= 12, "{mode}: {} ends", ports.len());
            assert_eq!(x, 0, "{mode}: crossings inside the fan");
            let gaps: Vec<f64> = ports.windows(2).map(|w| w[1] - w[0]).collect();
            let (lo, hi) = gaps.iter().fold((f64::INFINITY, 0.0f64), |(lo, hi), &g| {
                (lo.min(g), hi.max(g))
            });
            assert!(hi - lo < 1.0, "{mode}: pitch {lo:.1}..{hi:.1}");
        }
        // fanout-frames' fan (report B4's other case) stays one too.
        let g = fixture("fanout-frames");
        let sc = scene(&g);
        let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for s in [&sc, &route(&g, &at)] {
            for side in [Side::Right, Side::Bottom] {
                assert_eq!(fan(s, "n0", side).0, 0, "{side:?}");
            }
        }
    }

    /// B11 review r1 #6: a hub with more ends on a side than stand a
    /// spacing clear apart (ten on `hub`'s bottom side, 10.9 px between
    /// ports) is combed in `route()` too. Its stubs run side by side in
    /// any order, the longer the better they nest; held against the comb,
    /// every re-order of such a side was refused and the fan kept its
    /// crossings (`hub --> d1` over `hub -->|yes| c4` and `hub -.-> d3`).
    #[test]
    fn a_crowded_side_is_combed_in_route() {
        let src = "flowchart TD\n  hub[Hub service]\n  a1[Alpha] --> hub\n  a2[Beta] --> hub\n\
                   hub --> c1[One]\n  hub -.-> c2[Two]\n  hub ==> c3[Three]\n\
                   hub -->|yes| c4[Four]\n  hub --> c5[Five]\n  hub -->|no| c6[Six]\n\
                   hub --> c7[Seven]\n  c1 --> d1[Deep one]\n  c2 --> d1\n\
                   c7 --> d2[Deep two]\n  hub --> d2\n  hub --> d1\n\
                   hub -.-> d3[Deep three]\n  c4 --> d3\n  c3 --> d4[Deep four]\n  hub ==> d4\n";
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (mode, s) in [("scene", &sc), ("route", &route(&g, &at))] {
            let (x, ports) = fan(s, "hub", Side::Bottom);
            let k = ports.len();
            assert!(k >= 10, "{mode}: {k} ends");
            let pitch = (ports[k - 1] - ports[0]) / (k - 1) as f64;
            assert!(
                pitch < CLEAR_GAP,
                "{mode}: a crowded side, pitch {pitch:.1}"
            );
            assert_eq!(x, 0, "{mode}: crossings inside the fan");
        }
        // The stub of a fan's edge: its first leg out of the side, the
        // whole of a straight drop (its line is the curve's samples).
        let members = [(0, true), (1, false)];
        let tooth = [(0.0, 0.0), (0.0, 40.0), (60.0, 40.0), (60.0, 90.0)];
        assert_eq!(
            stub_of(&members, 0, &tooth),
            Some([(0.0, 0.0), (0.0, 40.0)])
        );
        let drop: Vec<Pt> = (0..=8).map(|k| (11.0, 90.0 - 10.0 * k as f64)).collect();
        assert_eq!(
            stub_of(&members, 1, &drop),
            Some([(11.0, 10.0), (11.0, 90.0)])
        );
        assert_eq!(stub_of(&members, 2, &tooth), None);
        // Two stubs of one side closer than the spacing clear run
        // beside each other for the length of the shorter.
        let (a, b) = (
            stub_of(&members, 0, &tooth).unwrap(),
            stub_of(&members, 1, &drop).unwrap(),
        );
        assert_eq!(beside(&a, &b, CLEAR_GAP, &[]), 30.0);
    }

    /// B11 review r1 #4 (a generated state diagram, bulk set 11002b70):
    /// in `route()` the comb put `S0`'s bottom side in the order of its
    /// turns by its ports alone — the track of `S0 --> C0` lay beyond
    /// its room and was pinned — and `C0 --> S0 : back` crossed it
    /// twice. A track may move back towards its room, the two tracks
    /// change places and the fan has no crossing left. (A label a line
    /// crosses already, here `back`'s, is no reason to refuse that: the
    /// label stage places it afresh.)
    #[test]
    fn a_fan_with_a_track_beyond_its_room_is_combed() {
        let src = "stateDiagram-v2\n[*] --> S0\nstate C0 {\n  [*] --> C0i0\n  C0i0 --> C0i1\n\
                   note right of C0i0 : n791\n  note right of C0i0 : n480\n\
                   note right of C0i1 : n615\n}\nS0 --> C0\nC0i1 --> S0 : x\nC0i1 --> S0 : x\n\
                   C0 --> S0 : back\nnote right of C0 : m960\nnote right of C0 : m378\n";
        let g = match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            _ => panic!("a state diagram"),
        };
        let sc = scene(&g);
        let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (mode, s) in [("scene", &sc), ("route", &route(&g, &at))] {
            let (x, ports) = fan(s, "S0", Side::Bottom);
            assert_eq!(ports.len(), 4, "{mode}");
            assert_eq!(x, 0, "{mode}: crossings inside the fan");
            let m = crate::quality::measure(&g, s);
            assert_eq!(
                (m.overlap_pairs, m.edge_edge_x),
                (0, 0),
                "{mode}: {:?}",
                m.details
            );
        }
    }

    #[test]
    fn a_comb_is_judged_by_the_yardsticks_clearances() {
        let r = (100.0, 100.0, 200.0, 150.0);
        // The gap to a box: through it, along its border, beside it.
        assert_eq!(seg_gap((50.0, 120.0), (250.0, 120.0), r), 0.0);
        assert_eq!(seg_gap((50.0, 100.0), (150.0, 100.0), r), 0.0);
        assert!((seg_gap((50.0, 92.0), (250.0, 92.0), r) - 8.0).abs() < 1e-9);
        assert!((seg_gap((203.0, 154.0), (260.0, 154.0), r) - 5.0).abs() < 1e-9);
        // Along a frame's border: the stretch within the spacing clear of
        // a side, over that side's span.
        let line = [(90.0, 0.0), (90.0, 130.0), (300.0, 130.0)];
        assert!((border_run(&line, r) - 30.0).abs() < 1e-9);
        assert_eq!(border_run(&[(60.0, 0.0), (60.0, 300.0)], r), 0.0);
        // Beside another line: an axis leg within the gap, or a slanted
        // stretch (a curve's sample) while it keeps within it — less the
        // port zone round an end the two share.
        let a = [(0.0, 0.0), (100.0, 0.0)];
        assert_eq!(
            beside(&a, &[(20.0, 4.0), (70.0, 4.0)], EDGE_SPACING, &[]),
            50.0
        );
        assert_eq!(
            beside(&a, &[(20.0, 8.0), (70.0, 8.0)], EDGE_SPACING, &[]),
            0.0
        );
        assert_eq!(
            beside(&a, &[(20.0, 8.0), (70.0, 8.0)], CLEAR_GAP, &[]),
            50.0
        );
        let slanted = beside(&a, &[(0.0, -12.0), (100.0, 12.0)], EDGE_SPACING, &[]);
        assert!((slanted - 50.0).abs() < 1e-9, "{slanted}");
        // (A stroke crossing it square runs beside it for no length.)
        assert!(beside(&a, &[(50.0, -40.0), (50.0, 40.0)], EDGE_SPACING, &[]) < 1e-9);
        let zones = [(-16.0, -16.0, 36.0, 16.0)];
        assert_eq!(
            beside(&a, &[(20.0, 4.0), (70.0, 4.0)], EDGE_SPACING, &zones),
            34.0
        );
    }

    #[test]
    fn a_comb_never_lays_a_track_along_a_box_it_passes() {
        // A generated chart (bulk set, 60d1b361): in `route()` the lane
        // of `n10 -.-> n0`, put in order beside `n0 --- n5` on `n0`'s
        // bottom side, took a track of its own a pitch past that edge's —
        // on the top border of `n4`, which it then ran along. A new
        // track keeps the room the stages keep round a box.
        let src = "flowchart TD\n  subgraph F0 [Frame 0]\n    n7>Node 7]\n  end\n\
                   n1 ==>|thick lbl| n7\n  n3 -.-> n0\n  n2 -.->|dot| n0\n  n4 --x n5\n\
                   n6 --- n0\n  n3 -.-> n0\n  n6 --- n5\n  n5 ==>|thick lbl| n6\n\
                   n10 -.-> n0\n  n6 <--> n3\n  n1 -.-> n9\n  n0 --- n5\n  n4 <--> n10\n\
                   n4 o--o n9\n  n2 x--x n6\n  n0 ==>|thick lbl| F0\n  n0 o--o n4\n\
                   linkStyle 13 stroke:#3a3,stroke-width:24px\n\
                   linkStyle 8 stroke:#3a3,stroke-width:7px\n";
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (mode, s) in [("scene", &sc), ("route", &route(&g, &at))] {
            let m = crate::quality::measure(&g, s);
            assert_eq!(m.edge_node_pairs, 0, "{mode}: {:?}", m.details);
        }
    }

    #[test]
    fn combing_is_settled_after_one_pass() {
        // A second run over a combed drawing finds nothing to do: no
        // side is combed back and forth.
        for name in ["dense-td", "bench-75", "fanout-frames"] {
            let g = fixture(name);
            let mut sc = scene(&g);
            let cx = crate::edgepass::ctx(&g, &sc);
            // (As the stage sees them: ends on their box sides, before
            // the final stage puts them on the outlines.)
            let before: Vec<(Vec<Pt>, [Pt; 4])> = sc
                .edges
                .iter()
                .map(|e| (e.waypoints.clone(), e.bezier))
                .collect();
            let mut moved = vec![false; sc.edges.len()];
            comb(&mut sc, &cx, &mut moved);
            let changed = sc
                .edges
                .iter()
                .zip(&before)
                .filter(|(e, b)| e.waypoints != b.0 || e.bezier != b.1)
                .count();
            assert_eq!(changed, 0, "{name}");
            assert!(moved.iter().all(|m| !m), "{name}");
        }
    }

    /// A hub fan of the reviewer's set (`tests/fixtures/fans`: one hub,
    /// 5 to 16 leaves wrapped onto two rows or stacked in a column).
    fn hub_fan(name: &str) -> crate::model::Graph {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fans");
        parse(&std::fs::read_to_string(format!("{dir}/{name}.mmd")).unwrap()).unwrap()
    }

    /// B11 review r2 #3: a hub whose leaves wrap onto two rows steps to
    /// the near row in the gap in front of it and to the far row in the
    /// gap beyond, so its tracks cannot follow its turns, and the first
    /// rounds left such a fan as it came: fan047, `hub --> l4` to the
    /// near row across the stub of `hub --> l1` to the far row (byte
    /// for byte the drawing from before the package); fan085, 14
    /// crossings under its hub. The ports follow the rows. Likewise a
    /// hub beside a column of leaves: fan099's and fan050's teeth run
    /// straight into their leaves (their tracks are where the leaves
    /// lie, and the ports follow them); fan038 and fan011 take the
    /// tracks of each way among themselves; fan020's far row is dealt
    /// tracks a pitch apart under the leaf that leaves least room.
    #[test]
    fn a_hub_fan_of_the_reviewers_set_is_a_comb_in_scene_and_route() {
        for name in [
            "fan011", "fan020", "fan038", "fan047", "fan050", "fan073", "fan085", "fan099",
        ] {
            let g = hub_fan(name);
            let sc = scene(&g);
            let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            for (mode, s) in [("scene", &sc), ("route", &route(&g, &at))] {
                let m = crate::quality::measure(&g, s);
                assert_eq!(
                    (m.fan_x, m.overlap_pairs, m.edge_node_pairs),
                    (0, 0, 0),
                    "{name} {mode}: {:?}",
                    m.details
                );
            }
        }
        // (The two-row fans leave their hub by one side.)
        for name in ["fan047", "fan073", "fan085"] {
            let (x, ports) = fan(&scene(&hub_fan(name)), "hub", Side::Bottom);
            assert!(ports.len() >= 5, "{name}: {} ends", ports.len());
            assert_eq!(x, 0, "{name}: crossings inside the fan");
        }
    }

    #[test]
    fn the_ports_follow_the_rows_where_the_tracks_cannot_follow_the_turns() {
        // Five teeth stepping low off a side at n = 0: three to a near
        // row (their paths run on to 62 and 71 px out), two to a far one
        // with their tracks in the gap beyond it. The fourth port's
        // tooth, to the near row, steps across the stub of the third's,
        // to the far row (fan047's `hub --> l4` and `hub --> l1`).
        let sc = plain(5);
        let z =
            |t: f64, d: f64, turn: f64, far: f64| vec![(t, 0.0), (t, d), (turn, d), (turn, far)];
        let teeth = vec![
            tooth(&sc, 0, &z(0.0, 20.0, -380.0, 62.0)),
            tooth(&sc, 1, &z(14.0, 33.0, -195.0, 62.0)),
            tooth(&sc, 2, &z(28.0, 136.0, -374.0, 177.0)),
            tooth(&sc, 3, &z(42.0, 45.0, -64.0, 71.0)),
            tooth(&sc, 4, &z(56.0, 148.5, -170.0, 186.0)),
        ];
        let line = |k: usize| teeth[k].comb.as_ref().unwrap().path.clone();
        assert_eq!(cross_count(&line(2), &line(3)), 1);
        let fr = Frame::of(Side::Bottom);
        // By the turns the far row's teeth would take the second and the
        // fourth port, and no deal has tracks for that: a tooth to the
        // near row cannot step in the far gap.
        assert!(plan_side(&sc, &teeth, fr, Order::Turn, Deal::Anchored).is_none());
        assert!(plan_side(&sc, &teeth, fr, Order::Turn, Deal::Pooled).is_none());
        // The rows: a track beyond what every tooth before it could
        // take starts a new one.
        let mut by_tooth = rows(&teeth, &[0, 1, 2, 3, 4]);
        by_tooth.sort_unstable();
        assert_eq!(by_tooth, vec![(0, 0), (1, 0), (2, 1), (3, 0), (4, 1)]);
        // Row by row — and by the tracks as they lie, which here is the
        // same — the two change ports, every track stays, and no tooth
        // crosses another.
        for (order, deal) in [(Order::Rows, Deal::Anchored), (Order::Track, Deal::Keep)] {
            let plan = plan_side(&sc, &teeth, fr, order, deal).expect("a plan");
            let mut moved: Vec<(usize, Pt)> = plan.iter().map(|r| (r.edge, r.corners[1])).collect();
            moved.sort_by_key(|m| m.0);
            assert_eq!(
                moved,
                vec![(2, (42.0, 136.0)), (3, (28.0, 45.0))],
                "{order:?}"
            );
            assert!(plan.iter().all(|r| r.shift == 14.0), "{order:?}");
            let now = |k: usize| {
                plan.iter()
                    .find(|r| r.edge == k)
                    .map_or_else(|| line(k), |r| r.corners.clone())
            };
            for a in 0..5 {
                for b in a + 1..5 {
                    assert_eq!(cross_count(&now(a), &now(b)), 0, "{order:?}: {a} x {b}");
                }
            }
        }
        // A tooth that would have to step the other way keeps the side
        // out of these orders: its turn lies between the ports.
        let mut inside = teeth;
        inside[3] = tooth(&sc, 3, &z(42.0, 45.0, 35.0, 71.0));
        assert!(plan_side(&sc, &inside, fr, Order::Rows, Deal::Anchored).is_none());
        assert!(plan_side(&sc, &inside, fr, Order::Track, Deal::Keep).is_none());
    }

    /// B11 review r2 #1: under a drag the judging works to a budget
    /// ([`DRAG_WORK`]), counted in the pairs of lines a plan has
    /// [`keeps`] measure; outside a drag there is none.
    #[test]
    fn a_drag_frame_judges_to_a_budget() {
        let g = parse("flowchart TD\n  a0 --- b0\n  a1 --- b1\n  a2 --- b2\n  a3 --- b3").unwrap();
        let sc = scene(&g);
        let cx = crate::edgepass::ctx(&g, &sc);
        let w = World::new(&sc, &cx);
        let fr = Frame::of(Side::Bottom);
        // A redrawn edge with a changed leg right across the drawing —
        // every other line comes near it — and one changed far away.
        let plan = |legs: &[Pt]| {
            vec![Redraw {
                edge: 0,
                corners: w.lines[0].clone(),
                changed: Changed::of(fr, legs, legs, 4),
                shift: 0.0,
                label: None,
            }]
        };
        let mid = (w.bounds[1].1 + w.bounds[1].3) / 2.0;
        let wide = plan(&[(-1e4, mid), (1e4, mid)]);
        let far = plan(&[(9e5, 9e5), (9e5, 9.1e5)]);
        assert_eq!((pair_count(&w, &wide), pair_count(&w, &far)), (3, 0));
        // (Two redrawn edges are a pair themselves.)
        let mut two = plan(&[(9e5, 9e5), (9e5, 9.1e5)]);
        two.extend(plan(&[(9e5, 9e5), (9e5, 9.1e5)]));
        two[1].edge = 1;
        assert_eq!(pair_count(&w, &two), 1);
        let mut free = Budget(None);
        assert!((0..100).all(|_| free.take(&w, &wide)));
        // A plan past what is left is not judged and costs nothing; a
        // cheaper one after it still is.
        let mut drag = Budget(Some(4));
        assert!(drag.take(&w, &wide));
        assert!(!drag.take(&w, &wide));
        assert_eq!(drag.0, Some(1));
        assert!(drag.take(&w, &two) && drag.take(&w, &far));
        assert_eq!(drag.0, Some(0));
    }

    /// Edge `a`'s and edge `b`'s ports on the bottom side of their
    /// common source change hands, each keeping its track: a fan put
    /// out of order by hand.
    fn swap_ports(sc: &mut Scene, a: usize, b: usize) {
        let corners = |e: &SceneEdge| ortho_corners(e).expect("on the axes");
        let (ca, cb) = (corners(&sc.edges[a]), corners(&sc.edges[b]));
        for (i, mut c, port) in [(a, ca.clone(), cb[0].0), (b, cb, ca[0].0)] {
            (c[0].0, c[1].0) = (port, port);
            let m = c.len();
            sc.edges[i].bezier = [c[0], c[1], c[m - 2], c[m - 1]];
            sc.edges[i].waypoints = smooth(&c, CORNER_R);
        }
    }

    /// B11 review r2 #1: what the stage does under a drag ([`DRAG_WORK`]).
    /// A fan of a few teeth in a frame that redraws few edges is combed
    /// as in `route()`; a side with more than [`DRAG_FAN`] teeth to deal
    /// is left as it is, and a frame that redraws [`DRAG_WORK`] /
    /// [`DRAG_WORK_EDGE`] edges or more is not combed at all.
    #[test]
    fn a_drag_frame_combs_small_fans_only() {
        let edge = |sc: &Scene, to: &str| sc.edges.iter().position(|e| e.to == to).unwrap();
        // fan047, its hub's fan put out of order: `hub --> l4` back
        // across the stub of `hub --> l1`.
        let g = hub_fan("fan047");
        let combed = scene(&g);
        let mut sc = combed.clone();
        let (l1, l4) = (edge(&sc, "l1"), edge(&sc, "l4"));
        swap_ports(&mut sc, l1, l4);
        assert_eq!(fan(&sc, "hub", Side::Bottom).0, 1);
        let run = |sc: &Scene, held: &[usize]| {
            let mut sc = sc.clone();
            let mut cx = crate::edgepass::ctx(&g, &sc);
            for &i in held {
                cx.held[i] = true;
            }
            let mut moved = vec![false; sc.edges.len()];
            comb(&mut sc, &cx, &mut moved);
            sc
        };
        // Outside a drag, and under one that keeps an edge of the fan:
        // five teeth to deal, five edges redrawn.
        for held in [vec![], vec![edge(&sc, "l0")]] {
            assert_eq!(fan(&run(&sc, &held), "hub", Side::Bottom).0, 0, "{held:?}");
        }

        // fan085: sixteen leaves, its hub's bottom side out of order.
        let g = hub_fan("fan085");
        let combed = scene(&g);
        let fr = Frame::of(Side::Bottom);
        let cx = crate::edgepass::ctx(&g, &combed);
        let hub = combed.nodes.iter().position(|n| n.id == "hub").unwrap();
        let members: Vec<(usize, bool)> = (0..combed.edges.len())
            .filter(|&i| combed.edges[i].from == "hub")
            .filter(|&i| {
                let r = end_rect(&combed, EndRef::Node(hub));
                side_of(r, ends_of(&combed.edges[i]).0) == Side::Bottom
            })
            .map(|i| (i, true))
            .collect();
        let line_n = end_rect(&combed, EndRef::Node(hub)).3;
        let teeth = teeth_of(&combed, &cx, &members, fr, line_n);
        let low: Vec<usize> = teeth
            .iter()
            .filter(|t| t.comb.as_ref().is_some_and(|c| c.turn < t.t - 60.0))
            .map(|t| t.edge)
            .collect();
        assert!(
            teeth.len() > DRAG_FAN + 2 && low.len() >= 2,
            "{}",
            teeth.len()
        );
        let mut sc = combed.clone();
        swap_ports(&mut sc, low[0], low[1]);
        let out_of_order = fan(&sc, "hub", Side::Bottom).0;
        assert!(out_of_order > 0);
        let run = |held: &[usize]| {
            let mut out = sc.clone();
            let mut cx = crate::edgepass::ctx(&g, &out);
            for &i in held {
                cx.held[i] = true;
            }
            let mut moved = vec![false; out.edges.len()];
            comb(&mut out, &cx, &mut moved);
            fan(&out, "hub", Side::Bottom).0
        };
        // Outside a drag the side is combed again.
        assert_eq!(run(&[]), 0);
        // A drag that keeps one edge redraws sixteen: no comb at all.
        let src = sc.edges.iter().position(|e| e.to == "hub").unwrap();
        assert!(sc.edges.len() > DRAG_WORK / DRAG_WORK_EDGE);
        assert_eq!(run(&[src]), out_of_order);
        // One that redraws eight teeth of the side, the two among them:
        // within the frame's budget, but more teeth than a side is
        // dealt under a drag.
        let keep: Vec<usize> = teeth.iter().map(|t| t.edge).take(DRAG_FAN + 2).collect();
        assert!(keep.contains(&low[0]) && keep.contains(&low[1]));
        let held: Vec<usize> = (0..sc.edges.len()).filter(|i| !keep.contains(i)).collect();
        assert_eq!(run(&held), out_of_order);
        // The same with the side's other teeth kept but four: dealt.
        let keep: Vec<usize> = keep.into_iter().take(4).collect();
        assert!(keep.contains(&low[0]) && keep.contains(&low[1]));
        let held: Vec<usize> = (0..sc.edges.len()).filter(|i| !keep.contains(i)).collect();
        assert_eq!(run(&held), 0);
    }

    /// A small fan dragged by its hub is a comb in every frame, as in
    /// `route()` (the budget is for the hubs of dense drawings).
    #[test]
    fn a_small_fan_is_combed_under_a_drag() {
        for name in ["fan047", "fan073"] {
            let g = hub_fan(name);
            let sc = scene(&g);
            let base: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let hub = sc.nodes.iter().position(|n| n.id == "hub").unwrap();
            for d in [(1.0, 0.0), (40.0, 0.0)] {
                let mut at = base.clone();
                at[hub] = (at[hub].0 + d.0, at[hub].1 + d.1);
                let part = crate::scene::route_partial(&g, &at, &sc, &base);
                assert_eq!(
                    fan(&part, "hub", Side::Bottom).0,
                    0,
                    "{name} dragged {d:?}: crossings inside the fan"
                );
            }
        }
    }

    /// B11 review r2 #3: the tracks of the teeth stepping each way are
    /// dealt among those ([`Deal::Ways`]). Dealt to the two ways in turn
    /// ([`Deal::Pooled`]) a track changed sides, and two teeth of one
    /// way came to run on one line (fan038 of the hub-fan set: 114 px).
    #[test]
    fn each_way_deals_its_own_tracks() {
        // One tooth stepping low on the track 75 px out; two stepping
        // high, the one that turns further out on that same depth and
        // from the inner port, across the other's leg past its turn.
        let sc = plain(3);
        let z =
            |t: f64, d: f64, turn: f64, far: f64| vec![(t, 0.0), (t, d), (turn, d), (turn, far)];
        let teeth = vec![
            tooth(&sc, 0, &z(0.0, 75.0, -100.0, 160.0)),
            tooth(&sc, 1, &z(20.0, 75.0, 245.0, 140.0)),
            tooth(&sc, 2, &z(30.0, 14.5, 144.0, 90.0)),
        ];
        let line = |k: usize| teeth[k].comb.as_ref().unwrap().path.clone();
        assert_eq!(cross_count(&line(1), &line(2)), 1);
        let fr = Frame::of(Side::Bottom);
        // No room past the far tooth's track for the near one's.
        assert!(plan_side(&sc, &teeth, fr, Order::Turn, Deal::Anchored).is_none());
        let now = |plan: &[Redraw], k: usize| {
            plan.iter()
                .find(|r| r.edge == k)
                .map_or_else(|| line(k), |r| r.corners.clone())
        };
        // Among the two stepping high: ports and tracks change hands,
        // the third tooth is left alone, nothing crosses.
        let ways = plan_side(&sc, &teeth, fr, Order::Turn, Deal::Ways).expect("a plan");
        assert_eq!(ways.len(), 2);
        assert_eq!(now(&ways, 1)[..2], [(30.0, 0.0), (30.0, 14.5)]);
        assert_eq!(now(&ways, 2)[..2], [(20.0, 0.0), (20.0, 75.0)]);
        assert_eq!(cross_count(&now(&ways, 1), &now(&ways, 2)), 0);
        // In turn to the two ways, both end up 75 px out.
        let pooled = plan_side(&sc, &teeth, fr, Order::Turn, Deal::Pooled).expect("a plan");
        assert_eq!(now(&pooled, 1)[1].1, now(&pooled, 2)[1].1);
        // A track the deal would hand a tooth outside its range, or on
        // the wrong side of a pinned one: no answer.
        let mut short = teeth;
        short[2] = tooth(&sc, 2, &z(30.0, 14.5, 144.0, 60.0));
        assert!(plan_side(&sc, &short, fr, Order::Turn, Deal::Ways).is_none());
    }

    /// B11 review r2 #3: a row's tracks are dealt a pitch apart as near
    /// where they lie as the order allows ([`Deal::Fitted`]) — fan020 of
    /// the hub-fan set: three teeth to the far row, the middle one's
    /// track beyond the last one's, whose leaf (a taller box) leaves no
    /// room for a track past it. No tooth keeps its track and has the
    /// others fit round it ([`Deal::Anchored`]).
    #[test]
    fn tracks_fit_a_pitch_apart_in_their_teeths_room() {
        let sc = plain(3);
        let z =
            |t: f64, d: f64, turn: f64, far: f64| vec![(t, 0.0), (t, d), (turn, d), (turn, far)];
        // (Turn order = port order: only the tracks are out of order.)
        let teeth = vec![
            tooth(&sc, 0, &z(0.0, 222.4, -500.0, 264.7)),
            tooth(&sc, 1, &z(10.0, 259.9, -350.0, 264.7)),
            tooth(&sc, 2, &z(20.0, 239.9, -150.0, 256.2)),
        ];
        let fr = Frame::of(Side::Bottom);
        assert!(plan_side(&sc, &teeth, fr, Order::Turn, Deal::Anchored).is_none());
        let plan = plan_side(&sc, &teeth, fr, Order::Turn, Deal::Fitted).expect("a plan");
        let depth = |k: usize| {
            plan.iter()
                .find(|r| r.edge == k)
                .map_or(teeth[k].comb.as_ref().unwrap().depth, |r| r.corners[1].1)
        };
        // The last as far out as its leaf lets it, the middle one a
        // pitch before it, the first where it was.
        let hi = teeth[2].comb.as_ref().unwrap().hi;
        assert_eq!(depth(0), 222.4);
        assert!((depth(2) - hi).abs() < 1e-9 && (depth(1) - (hi - TRACK_PITCH)).abs() < 1e-9);
        // In order already: nothing to do. No room: no answer.
        let done: Vec<Tooth> = (0..3)
            .map(|k| {
                tooth(
                    &sc,
                    k,
                    &z(k as f64 * 10.0, depth(k), -500.0 + 150.0 * k as f64, 264.7),
                )
            })
            .collect();
        assert!(plan_side(&sc, &done, fr, Order::Turn, Deal::Fitted).is_some_and(|p| p.is_empty()));
        let tight: Vec<Tooth> = (0..3)
            .map(|k| {
                tooth(
                    &sc,
                    k,
                    &z(k as f64 * 10.0, 20.0, -500.0 + 150.0 * k as f64, 40.0),
                )
            })
            .collect();
        assert!(plan_side(&sc, &tight, fr, Order::Turn, Deal::Fitted).is_none());
    }

    /// B11 review r2 #3: a deal that changes nothing does not end the
    /// combing unless it is one that puts the tracks in order. fan099 of
    /// the hub-fan set — three teeth running straight into a column of
    /// leaves, their tracks where the leaves lie — had its tracks dealt
    /// out again to no effect ([`Deal::Pooled`]: all three are pinned),
    /// and was left with `hub --> l9` across the stub of `hub --> l10`;
    /// its ports follow its tracks ([`Order::Track`]).
    #[test]
    fn a_deal_that_changes_nothing_is_not_the_last_word() {
        let sc = plain(3);
        let l = |t: f64, d: f64, turn: f64| vec![(t, 0.0), (t, d), (turn, d)];
        let teeth = vec![
            tooth(&sc, 0, &l(0.0, 262.0, -184.6)),
            tooth(&sc, 1, &l(14.0, 455.0, -180.7)),
            tooth(&sc, 2, &l(28.0, 358.5, -95.5)),
        ];
        assert!(teeth
            .iter()
            .all(|t| t.comb.as_ref().is_some_and(Comb::pinned)));
        let fr = Frame::of(Side::Bottom);
        let line = |k: usize| teeth[k].comb.as_ref().unwrap().path.clone();
        assert_eq!(cross_count(&line(1), &line(2)), 1);
        // By the turns the ports are in order, and no track may move.
        assert!(plan_side(&sc, &teeth, fr, Order::Turn, Deal::Anchored).is_none());
        for deal in [Deal::Ways, Deal::Pooled] {
            let plan = plan_side(&sc, &teeth, fr, Order::Turn, deal);
            assert!(
                deal == Deal::Ways || plan.is_some_and(|p| p.is_empty()),
                "{deal:?}"
            );
        }
        // By the tracks, the two change ports.
        let plan = plan_side(&sc, &teeth, fr, Order::Track, Deal::Keep).expect("a plan");
        let mut ports: Vec<(usize, f64)> = plan.iter().map(|r| (r.edge, r.corners[0].0)).collect();
        ports.sort_by_key(|p| p.0);
        assert_eq!(ports, vec![(1, 28.0), (2, 14.0)]);
        // And so does the stage (fan099 itself, in both modes).
        let g = hub_fan("fan099");
        let sc = scene(&g);
        let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for s in [&sc, &route(&g, &at)] {
            assert_eq!(crate::quality::measure(&g, s).fan_x, 0);
        }
    }
}
