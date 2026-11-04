//! Stages 2 and 4 of [`crate::edgepass::finish`]: obstacle-aware repair
//! of edges that pass through a node, a frame they do not belong to or
//! a frame's title chip, run on top of or cramped beside another edge,
//! or ride a frame border ([`repair`]); then, after the ports are spaced
//! again, the channel nudge over every orthogonal path ([`nudge`]).
//!
//! The pass leaves every good edge exactly as the builder drew it and
//! only reroutes the broken ones:
//!
//! 1. **Trigger.** Each drawn edge is sampled as it is painted
//!    ([`edge_polyline`]) and scored ([`Score`]): unrelated nodes it
//!    touches, frames it enters that hold neither of its ends, title
//!    chips it touches, long parallel runs on top of or within
//!    [`EDGE_SEP`] of an edge already accepted, and runs along frame
//!    borders or node sides within [`CLEAR_GAP`]. Clean edges are
//!    accepted as they are.
//! 2. **Lattice.** Per broken edge, a sparse Hanan grid over a window
//!    round its ends only: the bounds of the nodes and title chips grown
//!    by their separation, a track [`FRAME_SEP`] off each frame border,
//!    the midlines of the wider channels, offset tracks [`EDGE_SEP`]
//!    beside the accepted legs that took a channel line, and the edge's
//!    own stubs. A step through a node or chip is blocked; a step
//!    through its clearance band, or through the ring round a foreign
//!    frame, is banded; inside a frame, it is tagged. The accepted edges
//!    in the window (and the broken ones not yet searched, as drawn)
//!    price the steps they cross or run beside. Its size follows what
//!    lies near the edge, not the drawing.
//! 3. **Search.** A\* over (vertex, axis) states, from a source stub to
//!    a target stub, paying for length, bends, crossings and runs beside
//!    accepted edges, and never stepping back along the flow (G8); a
//!    backward flood from the goal first prunes the vertices from which
//!    no such path exists. The first try treats bands and foreign frames
//!    as walls; when it finds nothing, a greedy search over a wider
//!    window (the whole drawing when static) prices them instead, so an
//!    edge boxed in by the flow crosses a frame between its nodes rather
//!    than the nodes. Everything — lattice vertices built, pruned and
//!    priced, A\* pops, candidates scored — is charged to one work
//!    budget per repair (much smaller in [`crate::scene::route`], G13).
//!    When no try clears an edge of the nodes and frames it hits (its
//!    window over the lattice cap, or the budget spent), a cheap
//!    orthogonal [`detour`] along the channel lines between the boxes
//!    replaces it if it hits fewer: never a free curve across the
//!    drawing. An original whose only faults are frames or chips crossed
//!    between its nodes keeps its free curve over a reroute squeezed
//!    along a border, beside other edges or through a channel narrower
//!    than two frame separations ([`trades`], the narrow-channel rule).
//!    In the static scene a labelled edge is not rerouted for spacing
//!    alone (its label stays on its own curve, G11).
//! 4. **Nudge** (after [`crate::ports::respace`]). Every leg of every
//!    orthogonal path, port legs included, is spread [`EDGE_SEP`] from
//!    the legs sharing its channel and moved off the nodes, frame
//!    borders and chips closer than their separation, inside the free
//!    slab round it ([`spread`]); a channel too narrow for its legs is
//!    split at a frame beside it ([`split_channel`]), and no leg is put
//!    on a frame border. Stair steps under [`JOG_MIN`] are merged
//!    ([`merge_jog`]). In the static scene each side's ends are then put
//!    in the order their paths head ([`reorder`]).
//! 5. **Smooth.** Corners get two extra points either side, so the
//!    `curveBasis` spline the painter draws through the waypoints rounds
//!    them: [`CORNER_R`] while searching, up to [`CORNER_MAX`] on the
//!    finished path where the room allows.
//!
//! Every candidate is scored the same way as the original and replaces
//! it only when it is strictly better, and every nudge is kept only when
//! it scores no worse, so neither stage makes an edge worse. The pass is bounded by a work budget, never by time, so the
//! output is a function of the input alone. Above [`ROUTE_MAX_EDGES`]
//! edges, or on a drawing past [`ROUTE_MAX_EXTENT`], it is skipped.

use crate::edgepass::{along, normal, side_of, Ctx, EndRef, Mode, Side};
use crate::layout::LABEL_BOX_H;
use crate::model::{Graph, Shape};
use crate::ports::{PORT_INSET, PORT_SPACING};
// The clearance tests share their thresholds with the harness, so a
// reroute the router calls clean is clean by the metrics too.
use crate::clearance::{
    BORDER_HUG_GAP, CHIP_SEP, CLEAR_GAP, CRAMP_NODE_GAP, CROSS_MERGE, EDGE_SEP, EDGE_SEP_MIN,
    EDGE_SPACING, FRAME_INSET, FRAME_SEP, MIN_OVERLAP_RUN, NODE_CLEARANCE, NODE_SEP, PARALLEL_COS,
    PORT_ZONE, SAMPLE_STEP, TIGHT_RUN_MIN, WIGGLE_TOL,
};
use crate::scene::{edge_polyline, grow_scene, title_head, Bbox, Scene, ARROW_MARKER_PX, MARGIN};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// How far (px) past a separation a placed line goes, so a leg put at
/// exactly [`NODE_SEP`] / [`EDGE_SEP`] is clear of the strict `<`
/// threshold the harness measures with, whatever the float rounding.
const SEP_AIR: f64 = 0.5;
/// Clearance the lattice keeps between a rerouted edge and any node
/// (px): [`NODE_SEP`], so a leg along a node's side never reads as
/// cramped.
const ROUTE_CLEAR: f64 = NODE_SEP + SEP_AIR;
/// Clearance kept from a frame the edge must stay out of (px): the
/// ring this wide round it is a band like a node's ([`BAND_COST`] when
/// priced, a wall in the strict tries), so a channel between two
/// frames narrower than twice this is closed unless nothing else is
/// open (the narrow-channel rule, G4/G6).
const ROUTE_FRAME_CLEAR: f64 = FRAME_SEP / 2.0 + SEP_AIR;
/// A path running along a frame's border line (inside or out) closer
/// than this (px) hugs it (G4): it scores worse ([`Score::hug`]) and the
/// channel nudge moves the run off it where there is room.
const FRAME_BAND: f64 = CLEAR_GAP;
/// Per px of path running along a frame's border within
/// [`FRAME_BAND`] (G4): a quarter of a run beside another edge, so a
/// path takes a free channel over a border, but a border over a
/// crossing or a long way round.
const HUG_COST: f64 = RUN_COST / 4.0;
/// Room (px) a path keeps from another edge's label box; inside it the
/// path pays [`LABEL_COST`] per px.
const LABEL_CLEAR: f64 = CHIP_CLEAR;
/// Per px of path under another edge's label (grown by
/// [`LABEL_CLEAR`]): half a run beside another edge, so of two
/// otherwise equal channels the one clear of labels wins. Never a wall,
/// and the lattice gets no lines of its own round labels: labels move
/// after routing, and the extra lines would cost lattice cells that
/// the node and frame detours need (G13).
const LABEL_COST: f64 = RUN_COST / 2.0;
/// A port stub reaches this much beyond the clearance (px), so it ends
/// on free lattice just outside its own inflated end.
const STUB_EXTRA: f64 = 2.0;
/// Extra room beyond the drawing the lattice may use (px): half the
/// canvas margin, so a detour round the outermost node stays on the
/// canvas.
const ROUTE_OUTER: f64 = MARGIN / 2.0;
/// What a corner costs, in px of path length.
const BEND_COST: f64 = 24.0;
/// What crossing an accepted edge costs, in px of path length: five
/// corners, so a path takes a few bends round before it crosses (G1:
/// cross only when no free channel exists).
const CROSS_COST: f64 = 120.0;
/// Per px of path within [`EDGE_SPACING`] of an accepted edge and
/// parallel to it (on top of it, G1): a shared track longer than
/// [`MIN_OVERLAP_RUN`] costs more than two bends and two
/// [`EDGE_SEP`]s of extra length, so the parallel track wins.
const RUN_COST: f64 = 8.0;
/// Per px of path within [`EDGE_SEP`] of an accepted edge and parallel
/// to it, but not on top of it: cramped (G4), a quarter of a run.
const TIGHT_COST: f64 = RUN_COST / 4.0;
/// Per px a path steps back along the flow, on the last try of an edge
/// that already wiggles (G8). Every earlier try forbids it.
const BACKTRACK_COST: f64 = 8.0;
/// Per px of path inside a frame the edge should stay out of. Entering
/// one is a defect, but a cheaper one than cutting through the nodes
/// inside it: when the flow leaves no way round a frame (G8), the path
/// crosses it between its nodes instead.
const FRAME_COST: f64 = 30.0;
/// Per px of path inside the clearance band of a node or title chip:
/// high enough that any way round wins, finite so two nodes closer than
/// twice the clearance still leave a (priced) channel between them.
const BAND_COST: f64 = 12.0;
/// What leaving or entering a node by a side the builder did not
/// choose costs a back-edge (px): two bends' worth, so the chosen side
/// wins unless the other is clearly shorter.
const SIDE_SWITCH_COST: f64 = 2.0 * BEND_COST;
/// A\* pops allowed per search in [`crate::scene::route`]; when
/// exhausted the try finds nothing.
const ROUTE_WORK_BUDGET: usize = 6_000;
/// A\* pops allowed per search in the static scene, which is drawn once
/// and so can afford a wider hunt.
const STATIC_WORK_BUDGET: usize = 20_000;
/// Heuristic weight of the priced fallback search, where bands and
/// frames make the true cost far exceed the distance: it takes the
/// first fair path rather than flood the drawing for the best.
const H_WEIGHT_GREEDY: f64 = 40.0;
/// Work allowed over a whole repair in [`crate::scene::route`]: once
/// spent, the remaining edges keep their geometry (hard defects are
/// searched first). A unit is one lattice vertex built, pruned or
/// priced (about 17 ns native); A\* pops and dead-end floods are
/// weighted to match ([`POP_WORK`], [`FLOOD_PER_WORK`]), and scoring
/// is charged by its looks ([`SCORE_LOOKS_PER_UNIT`]), so the budget
/// bounds the router's share of a drag frame (G13) whatever the
/// drawing. It is about the least that keeps the fixtures' routed
/// quality gates (`tests/quality.rs`) green on the round-2 router: at
/// 280 000 bench-75 keeps overlaps and defects the gates reject (320 000
/// also passes, a few ms slower per dense frame). The label stage comes
/// on top, so a whole `route()` frame does NOT meet G13's 8 ms on the
/// densest diagrams; `route_partial` ([`PARTIAL_TOTAL_WORK`]) meets it
/// for a drag from the auto layout with few nodes already moved, and
/// once more than half the edges are redrawn it IS a `route()` frame.
const ROUTE_TOTAL_WORK: usize = 300_000;
/// Work allowed over a whole repair of the static scene (same units).
/// About 17 ms native at the most: the user's largest diagrams and the
/// fixtures spend under 700 000 of it (the largest about 694 000, so
/// the headroom is about 30%, not half), while a dense synthetic one
/// (hundreds of edges, most of them crossing) stops here instead of
/// searching every edge.
const STATIC_TOTAL_WORK: usize = 1_000_000;
/// [`detour`] work allowed over a whole repair in
/// [`crate::scene::route`] (the detour's own units, see
/// [`DETOUR_TOTAL_WORK`], which is the static scene's allowance). With
/// the round-2 detour (two lanes per wall, candidates ranked against the
/// accepted edges) 80 000 already leaves a large hand-written diagram
/// with edges through nodes that 100 000 clears; this keeps a margin.
const ROUTE_DETOUR_WORK: usize = 120_000;
/// The least work of a repair with fixed edges
/// ([`crate::scene::route_partial`]): what is left to reroute lies near
/// the drag, and the router's share of a drag frame of the editor's API
/// stays near 1 ms native whatever the drawing (G13). A repair that
/// redraws a larger share of the edges gets that share of
/// [`ROUTE_TOTAL_WORK`] when it is more (see [`Work::of`]), up to
/// [`PARTIAL_MAX_WORK`].
const PARTIAL_TOTAL_WORK: usize = 40_000;
/// The most search and scoring work a repair with fixed edges gets,
/// however much it redraws: at 90 000 the dense fixtures still fit
/// G13's 8 ms but the slowest hand-written diagram measured no longer
/// does; 60 000 keeps every one under it. A frame that redraws most of
/// the drawing is a full `route()` instead (see
/// [`crate::scene::route_partial`]).
const PARTIAL_MAX_WORK: usize = 60_000;
/// [`ROUTE_DETOUR_WORK`] floor for a repair with fixed edges.
const PARTIAL_DETOUR_WORK: usize = 20_000;
/// Vertices one search lattice may have in [`crate::scene::route`]; a
/// window holding more obstacles than that is not searched.
const ROUTE_MAX_CELLS: usize = 6_000;
/// Vertices one search lattice may have in the static scene.
const STATIC_MAX_CELLS: usize = 250_000;
/// The first searches of an edge build their lattice only inside the
/// stubs' bounding box grown by this (px).
const ROUTE_WINDOW: f64 = 160.0;
/// The priced fallback of [`crate::scene::route`] searches the stubs'
/// bounding box grown by this (px); the static scene's searches the
/// whole drawing.
const ROUTE_WALLS_WINDOW: f64 = 480.0;
/// Weight on the A\* distance heuristic. Above 1 the search is greedy
/// towards the goal: it settles for a path at most this much costlier
/// than the cheapest, in return for popping far fewer states — the
/// priced crossings and runs make the plain distance a weak bound.
const H_WEIGHT: f64 = 1.5;
/// Above this many edges the pass leaves the drawing as built: its
/// clearance scoring (outside the work budget) grows with the square of
/// the edges meeting at one hub, and a diagram that large is past
/// interactive editing anyway. The cut-off is abrupt: one edge more
/// turns the whole repair off.
const ROUTE_MAX_EDGES: usize = 400;
/// Corner rounding distance (px) of a candidate while it is searched
/// and scored, capped at half the shorter leg.
const CORNER_R: f64 = 14.0;
/// The widest corner rounding (px) a finished path gets where its legs
/// and the free room round the corner allow (one soft-orthogonal style
/// next to the builder's free curves).
const CORNER_MAX: f64 = 36.0;
/// A stair step (two opposite corners) shorter than this (px) is a
/// micro-jog: merged into one leg when that stays clear (G8 taste).
const JOG_MIN: f64 = 2.0 * CORNER_R;
/// Passes of the channel nudge: a second pass settles the groups the
/// first one's moves brought together.
const NUDGE_ROUNDS: usize = 2;
/// Work the nudge stage of [`crate::scene::route`] may spend scoring
/// its paths (units of [`SCORE_POINT_WORK`] per point): a drag frame
/// with a hundred orthogonal paths stays inside its time (G13).
const NUDGE_ROUTE_WORK: usize = 60_000;
/// Work [`spread`] may spend in [`crate::scene::route`] over both its
/// rounds (units of [`SPREAD_LOOKS_PER_UNIT`] looks): the real and
/// fixture diagrams need under 30 000, so only a pathological drag
/// frame is cut short (G13).
const SPREAD_ROUTE_WORK: usize = 40_000;
/// Bucket width (px) along a leg axis for the free curves' pieces that
/// [`spread`] fences a group of legs with.
const SPREAD_BUCKET: f64 = 64.0;
/// Obstacle, leg and curve-segment looks [`spread`] makes per work
/// unit when placing a group of legs: a look costs a few comparisons,
/// about this many per scored point's [`SCORE_POINT_WORK`] share.
const SPREAD_LOOKS_PER_UNIT: usize = 4;
/// Scorings one path's nudge is charged for up front (the current
/// path, the nudged one, a stair step and both roundings).
const NUDGE_SCORES: usize = 5;
/// An original that only enters a frame between its nodes is kept
/// over a reroute running along a border or node side for longer than
/// this (px): the free curve across the frame's empty area reads
/// better than a line squeezed along a border (G4 over G6, which asks
/// for a clear channel).
const HUG_TRADE: f64 = 120.0;
/// The clearance multiplier of the retry when the first route's
/// smoothed curve still touches something.
const RETRY_CLEAR_SCALE: f64 = 1.5;
/// Node clearance of each lattice: the first try's, then the retry's.
const CLEARANCES: [f64; 2] = [ROUTE_CLEAR, ROUTE_CLEAR * RETRY_CLEAR_SCALE];
/// An accepted curve is indexed, priced, and scored against others as a
/// polyline that strays at most this far from it (px): well inside
/// the harness's edge spacing, and it turns a sampled curve into a few
/// segments.
const SIMPLIFY_TOL: f64 = 2.0;
/// Most accepted legs per axis one lattice adds offset tracks beside.
const OFFSET_TRACKS: usize = 16;
/// Two lattice lines closer than this are one line (px).
const LINE_EPS: f64 = 0.5;
/// A channel midline is added only between bounds at least this far
/// apart (px): a narrower gap is already a channel of one route, and
/// its midline would only make the lattice denser.
const MIDLINE_MIN_GAP: f64 = 40.0;
/// Float tolerance for coincident coordinates.
const EPS: f64 = 1e-6;

/// Clearance kept from a title chip (px).
const CHIP_CLEAR: f64 = CHIP_SEP;
/// A clearance test samples one segment at most this many times: a
/// longer segment is sampled more coarsely, so the cost of scoring an
/// edge never grows with its length in px (host-given centres can be
/// arbitrarily far apart).
const MAX_SEG_SAMPLES: usize = 512;
/// A drawing wider or taller than this (px), or with bounds that are
/// not finite, is left as built: no diagram that size is edited by hand,
/// and its coordinates are past what the lattice resolves.
const ROUTE_MAX_EXTENT: f64 = 1.0e6;
/// Cell size of the accepted-segment index (px): a few edge spacings,
/// so a clearance probe touches at most four cells.
const SEG_CELL: f64 = 32.0;
/// Upper bound on index cells; a bigger drawing gets coarser cells.
const SEG_MAX_CELLS: f64 = 65_536.0;
/// Work units charged per accepted segment priced onto a lattice, on
/// top of the steps it touches.
const PRICE_SEG_WORK: usize = 1;
/// Work units charged per point of a candidate scored: scoring a route
/// against the scene and the accepted edges costs about this many
/// lattice vertices' worth per drawn point.
const SCORE_POINT_WORK: usize = 8;
/// Scoring looks (a segment tested against an obstacle, a distance
/// sampled) per work unit: about a lattice vertex's worth (G13).
const SCORE_LOOKS_PER_UNIT: usize = 4;
/// Work units charged per A\* pop: a pop prices four steps and pushes
/// up to four states, about six lattice vertices' worth of building
/// (~100 ns against ~17 ns native).
const POP_WORK: usize = 6;
/// Lattice vertices the dead-end flood ([`mark_live`]) visits per work
/// unit: a visit is a few compares, a third of a vertex built.
const FLOOD_PER_WORK: usize = 3;

type Pt = (f64, f64);
/// Axis-aligned rect `(x0, y0, x1, y1)`.
type R = (f64, f64, f64, f64);

fn grow(r: R, d: f64) -> R {
    (r.0 - d, r.1 - d, r.2 + d, r.3 + d)
}

fn overlaps(a: R, b: R) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// Strictly inside (borders are free).
fn inside(r: R, p: Pt) -> bool {
    p.0 > r.0 && p.0 < r.2 && p.1 > r.1 && p.1 < r.3
}

fn rect_dist(r: R, p: Pt) -> f64 {
    let dx = (r.0 - p.0).max(p.0 - r.2).max(0.0);
    let dy = (r.1 - p.1).max(p.1 - r.3).max(0.0);
    norm(dx, dy)
}

fn bounds(pts: &[Pt]) -> R {
    pts.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |r, p| (r.0.min(p.0), r.1.min(p.1), r.2.max(p.0), r.3.max(p.1)),
    )
}

/// Length of `(dx, dy)`. A plain square root: `hypot`'s overflow
/// guard is a libm call, and these lengths are always small.
fn norm(dx: f64, dy: f64) -> f64 {
    (dx * dx + dy * dy).sqrt()
}

fn dist(a: Pt, b: Pt) -> f64 {
    norm(a.0 - b.0, a.1 - b.1)
}

fn seg_dist(p: Pt, a: Pt, b: Pt) -> f64 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let l2 = vx * vx + vy * vy;
    if l2 == 0.0 {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2).clamp(0.0, 1.0);
    dist(p, (a.0 + t * vx, a.1 + t * vy))
}

/// How many steps a segment of length `l` is sampled in: one per
/// [`SAMPLE_STEP`] (the harness's sampling), at most
/// [`MAX_SEG_SAMPLES`].
fn sample_count(l: f64) -> usize {
    ((l / SAMPLE_STEP).ceil().max(1.0) as usize).min(MAX_SEG_SAMPLES)
}

/// The part of segment `a`–`b` inside the closed rect `r`
/// (Liang-Barsky), or `None` when it misses.
fn clip(a: Pt, b: Pt, r: R) -> Option<(Pt, Pt)> {
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
                return None;
            }
        } else if p < 0.0 {
            t0 = t0.max(q / p);
        } else {
            t1 = t1.min(q / p);
        }
    }
    (t0 <= t1).then_some((
        (a.0 + t0 * dx, a.1 + t0 * dy),
        (a.0 + t1 * dx, a.1 + t1 * dy),
    ))
}

/// Distance from `p` to a node's drawn outline (0 inside).
fn outline_gap(r: R, shape: Shape, p: Pt) -> f64 {
    let (hw, hh) = ((r.2 - r.0) / 2.0, (r.3 - r.1) / 2.0);
    let (cx, cy) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
    let (dx, dy) = ((p.0 - cx).abs(), (p.1 - cy).abs());
    match shape {
        Shape::Diamond if hw > 0.0 && hh > 0.0 => {
            let f = dx / hw + dy / hh - 1.0;
            (f * hw * hh / hw.hypot(hh)).max(0.0)
        }
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            (norm(dx, dy) - hw).max(0.0)
        }
        _ => rect_dist(r, p),
    }
}

/// How bad one drawn edge is, compared lexicographically: unrelated
/// nodes it crosses (G6: the worst, a line through a box reads as a
/// connection); then foreign frames it enters and title chips it
/// touches; then a flow-axis backtrack (so of two equal routes the
/// monotone one wins); then px on top of accepted edges, crossings at
/// an arrowhead, cramped nodes,
/// crossings with accepted edges, px cramped beside accepted edges,
/// other edges' labels it runs under, and px along frame borders and
/// node sides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    node: usize,
    frame: usize,
    /// Frame title chips the path touches (G6).
    chip: usize,
    wiggle: usize,
    /// px of parallel run on top of accepted edges (within
    /// [`EDGE_SPACING`]), over the pairs whose run passes
    /// [`MIN_OVERLAP_RUN`] (G1).
    overlap: usize,
    /// Crossings with accepted edges within [`HEAD_ZONE`] of either
    /// edge's end: a line through another's arrowhead reads as a
    /// T-junction, not a crossing (G1, G11).
    head: usize,
    near: usize,
    cross: usize,
    /// px of parallel run within [`EDGE_SEP`] of accepted edges that
    /// share no end with it, over the pairs whose run passes
    /// [`TIGHT_RUN_MIN`] (G4).
    tight: usize,
    /// Other edges' labels the path runs under: only a tie-break, as a
    /// label moves off a stroke more cheaply than a path takes a
    /// crossing to go round it.
    label: usize,
    /// px the path runs along frame borders and unrelated node sides
    /// within [`CLEAR_GAP`], over the sides it rides for more than
    /// [`TIGHT_RUN_MIN`] (G4): of two otherwise equal routes, the one in
    /// the middle of its channel wins.
    hug: usize,
}

impl Score {
    /// Node, frame and chip hits together: what makes an edge broken.
    fn hard(&self) -> usize {
        self.node + self.frame + self.chip
    }

    /// The part scored against the scene alone (what
    /// [`World::score`] gives without the accepted edges).
    fn scene(self) -> Score {
        Score {
            overlap: 0,
            head: 0,
            cross: 0,
            tight: 0,
            ..self
        }
    }
}

/// The fixed geometry of the scene every edge is routed against.
struct World {
    dir: crate::model::Direction,
    /// How close (px) a path may pass a title chip that is not its own
    /// frame's before it counts as through it: [`CHIP_SEP`] in the
    /// static scene (G6, G11); in `route()` only a touch
    /// ([`NODE_CLEARANCE`]), as the wider rule costs a drag frame
    /// reroutes that lie on other edges instead (G1, G13).
    chip_reach: f64,
    nodes: Vec<(R, Shape)>,
    frames: Vec<R>,
    chips: Vec<R>,
    /// (edge, label box) of every labelled edge, as placed so far.
    labels: Vec<(usize, R)>,
    /// The drawing's bounds plus [`ROUTE_OUTER`]: the lattice never
    /// leaves it.
    outer: R,
    /// Segment-against-obstacle tests and distance samples the scoring
    /// made since [`World::spent`] last read it (what the repair
    /// charges a scoring, G13).
    looks: std::cell::Cell<usize>,
}

/// What the router knows about one edge.
struct Info {
    ends: (EndRef, EndRef),
    /// Frames the edge may enter: ancestors of either end, and an end
    /// frame itself (as the harness allows).
    allowed: Vec<usize>,
    /// Frames the lattice keeps it out of: every frame not in
    /// `allowed`, plus an end frame that does not hold the other end.
    blocked_frames: Vec<usize>,
    back: bool,
    /// The two ends overlap along the flow (same rank): no path round
    /// what lies between them can be monotone, so none is asked for.
    sideways: bool,
}

/// The accepted edges as later edges see them: their drawn segments in
/// a uniform grid, so the clearance tests and the lattice pricing only
/// look at the nearby ones.
struct Accepted {
    /// Per edge: its ends, when it is accepted.
    ends: Vec<Option<(EndRef, EndRef)>>,
    /// Per edge: the ids of its live segments.
    of_edge: Vec<Vec<u32>>,
    /// Every segment ever added: (edge, a, b, live).
    segs: Vec<(usize, Pt, Pt, bool)>,
    x0: f64,
    y0: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
    /// Per segment: the query generation that last returned it.
    seen: Vec<u32>,
    gen: u32,
}

impl Accepted {
    fn new(ne: usize, outer: R) -> Accepted {
        let (w, h) = ((outer.2 - outer.0).max(1.0), (outer.3 - outer.1).max(1.0));
        let cell = SEG_CELL.max((w * h / SEG_MAX_CELLS).sqrt());
        let nx = (w / cell).ceil() as usize + 1;
        let ny = (h / cell).ceil() as usize + 1;
        Accepted {
            ends: vec![None; ne],
            of_edge: vec![Vec::new(); ne],
            segs: Vec::new(),
            x0: outer.0,
            y0: outer.1,
            cell,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
            seen: Vec::new(),
            gen: 0,
        }
    }

    fn cell_range(&self, r: R) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
        let ix = |x: f64| (((x - self.x0) / self.cell).floor().max(0.0) as usize).min(self.nx - 1);
        let iy = |y: f64| (((y - self.y0) / self.cell).floor().max(0.0) as usize).min(self.ny - 1);
        (ix(r.0)..ix(r.2) + 1, iy(r.1)..iy(r.3) + 1)
    }

    /// Accept edge `i` drawn as `pts`, replacing what it was before.
    fn put(&mut self, i: usize, ends: (EndRef, EndRef), pts: &[Pt]) {
        for &id in &self.of_edge[i] {
            self.segs[id as usize].3 = false;
        }
        self.of_edge[i].clear();
        self.ends[i] = Some(ends);
        for w in pts.windows(2) {
            let id = self.segs.len() as u32;
            self.segs.push((i, w[0], w[1], true));
            self.seen.push(0);
            self.of_edge[i].push(id);
            let (xr, yr) = self.cell_range(bounds(w));
            for y in yr {
                for x in xr.clone() {
                    self.cells[y * self.nx + x].push(id);
                }
            }
        }
    }

    /// Live segments of edges other than `me` whose cells meet `r`,
    /// each once, in id order.
    fn near(&mut self, r: R, me: usize) -> Vec<u32> {
        self.gen = self.gen.wrapping_add(1);
        if self.gen == 0 {
            self.seen.iter_mut().for_each(|s| *s = 0);
            self.gen = 1;
        }
        let (xr, yr) = self.cell_range(r);
        let mut out = Vec::new();
        for y in yr {
            for x in xr.clone() {
                let cell = &self.cells[y * self.nx + x];
                for &id in cell {
                    let s = &self.segs[id as usize];
                    if s.3 && s.0 != me && self.seen[id as usize] != self.gen {
                        self.seen[id as usize] = self.gen;
                        out.push(id);
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }
}

fn frames_of(cx: &Ctx, e: EndRef) -> &[usize] {
    match e {
        EndRef::Node(v) => cx.node_frames.get(v).map_or(&[], Vec::as_slice),
        EndRef::Frame(f) => cx.frame_anc.get(f).map_or(&[], Vec::as_slice),
    }
}

fn end_node(e: EndRef) -> Option<usize> {
    match e {
        EndRef::Node(v) => Some(v),
        EndRef::Frame(_) => None,
    }
}

impl World {
    fn new(g: &Graph, sc: &Scene) -> World {
        let nodes = sc
            .nodes
            .iter()
            .map(|n| {
                (
                    (
                        n.x - n.w / 2.0,
                        n.y - n.h / 2.0,
                        n.x + n.w / 2.0,
                        n.y + n.h / 2.0,
                    ),
                    n.shape,
                )
            })
            .collect();
        let frames: Vec<R> = sc
            .clusters
            .iter()
            .map(|c| (c.x, c.y, c.x + c.w, c.y + c.h))
            .collect();
        let mut bb = Bbox::new();
        grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        for e in &sc.edges {
            for &(x, y) in &e.waypoints {
                bb.add(x, y);
            }
        }
        let (minx, maxx, miny, maxy) = bb.finish();
        // Title chips with the header corner they sit in, as `to_svg`
        // draws them (same math as the harness).
        let chips = (0..sc.clusters.len())
            .map(|ci| title_head(&sc.clusters, ci, maxx + MARGIN))
            .collect();
        let labels = sc
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let (_, c, w) = e.label.as_ref()?;
                let h = LABEL_BOX_H / 2.0;
                Some((i, (c.0 - w / 2.0, c.1 - h, c.0 + w / 2.0, c.1 + h)))
            })
            .collect();
        World {
            dir: g.direction,
            chip_reach: CHIP_SEP,
            nodes,
            frames,
            chips,
            labels,
            outer: grow((minx, miny, maxx, maxy), ROUTE_OUTER),
            looks: std::cell::Cell::new(0),
        }
    }

    /// Count `n` more looks of the scoring.
    fn look(&self, n: usize) {
        self.looks.set(self.looks.get().saturating_add(n));
    }

    /// The work units the scorings since the last call cost (see
    /// [`SCORE_LOOKS_PER_UNIT`]), and start counting afresh.
    fn spent(&self) -> usize {
        self.looks.take().div_ceil(SCORE_LOOKS_PER_UNIT)
    }

    /// Whether the drawing is one the lattice can resolve (see
    /// [`ROUTE_MAX_EXTENT`]).
    fn sane(&self) -> bool {
        let o = self.outer;
        [o.0, o.1, o.2, o.3].iter().all(|v| v.is_finite())
            && o.2 - o.0 <= ROUTE_MAX_EXTENT
            && o.3 - o.1 <= ROUTE_MAX_EXTENT
    }

    /// The solid obstacles for node clearance `clear`: every node, and
    /// every title chip at a clearance scaled the same way.
    fn solids(&self, clear: f64) -> Vec<(R, f64)> {
        let chip = CHIP_CLEAR * clear / ROUTE_CLEAR;
        self.nodes
            .iter()
            .map(|&(r, _)| (r, clear))
            .chain(self.chips.iter().map(|&r| (r, chip)))
            .collect()
    }

    /// Whether a stub at `p` of an edge `info` stands in the open: off
    /// every node and chip, and out of every frame it must avoid.
    fn stub_free(&self, info: &Info, p: Pt) -> bool {
        self.nodes.iter().all(|&(r, _)| !inside(r, p))
            && self.chips.iter().all(|&r| !inside(r, p))
            && info
                .blocked_frames
                .iter()
                .all(|&f| !inside(self.frames[f], p))
    }

    fn end_rect(&self, e: EndRef) -> R {
        match e {
            EndRef::Node(v) => self.nodes[v].0,
            EndRef::Frame(f) => self.frames[f],
        }
    }

    /// Score the drawn polyline `pts` of edge `me` against the scene
    /// and, when given, the accepted edges.
    fn score(&self, info: &Info, me: usize, pts: &[Pt], acc: Option<&mut Accepted>) -> Score {
        let mut s = Score::default();
        if pts.len() < 2 {
            return s;
        }
        let bb = bounds(pts);
        let ends = [end_node(info.ends.0), end_node(info.ends.1)];
        for (v, &(r, shape)) in self.nodes.iter().enumerate() {
            if ends.contains(&Some(v)) || !overlaps(grow(bb, CRAMP_NODE_GAP), r) {
                continue;
            }
            // Only the part of each segment near the node is sampled, so
            // the cost follows the node's size, not the edge's length.
            let near = grow(r, CRAMP_NODE_GAP);
            let mut gap = f64::INFINITY;
            self.look(pts.len());
            // Done once it is through the node: only the class counts.
            'node: for w in pts.windows(2) {
                let Some((a, b)) = clip(w[0], w[1], near) else {
                    continue;
                };
                let k = sample_count(dist(a, b));
                for q in 0..=k {
                    let t = q as f64 / k as f64;
                    let p = (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1));
                    self.look(1);
                    gap = gap.min(outline_gap(r, shape, p));
                    if gap < NODE_CLEARANCE {
                        break 'node;
                    }
                }
            }
            if gap < NODE_CLEARANCE {
                s.node += 1;
            } else if gap < CRAMP_NODE_GAP {
                s.near += 1;
            }
        }
        for (f, &r) in self.frames.iter().enumerate() {
            if info.allowed.contains(&f) || !overlaps(bb, r) {
                continue;
            }
            // Entered when some piece of a segment lies strictly inside.
            self.look(pts.len());
            let inner = grow(r, -FRAME_INSET);
            let enters = pts.windows(2).any(|w| {
                clip(w[0], w[1], inner)
                    .is_some_and(|(a, b)| inside(inner, ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)))
            });
            if enters {
                s.frame += 1;
            }
        }
        // Title chips it touches or passes closer than `chip_reach`,
        // its own frames' included; one of a frame it ends on only when
        // touched (as the harness counts them).
        for (f, &c) in self.chips.iter().enumerate() {
            let own = [info.ends.0, info.ends.1].contains(&EndRef::Frame(f));
            let g = grow(c, if own { NODE_CLEARANCE } else { self.chip_reach });
            if overlaps(grow(bb, EPS), g) && pts.windows(2).any(|w| clip(w[0], w[1], g).is_some()) {
                s.chip += 1;
            }
        }
        // Other edges' labels the path runs under.
        for &(j, r) in &self.labels {
            if j == me || !overlaps(bb, r) {
                continue;
            }
            self.look(pts.len());
            let under = pts.windows(2).any(|w| {
                clip(w[0], w[1], r)
                    .is_some_and(|(a, b)| inside(r, ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)))
            });
            s.label += usize::from(under);
        }
        // Frame borders (inside or out) and unrelated node sides the path
        // runs along, away from its own ends; a frame it ends on is its
        // own business. Not looked at for a path through a node: its
        // border rides are moot (the last tie-break) and, in a crowded
        // drag, every node is near it.
        let simple = simplify(pts);
        let own: Vec<R> = ends.iter().flatten().map(|&v| self.nodes[v].0).collect();
        let end_frames = [info.ends.0, info.ends.1].map(|e| match e {
            EndRef::Frame(f) => Some(f),
            EndRef::Node(_) => None,
        });
        let probe = grow(bb, CLEAR_GAP);
        let sides = self
            .frames
            .iter()
            .enumerate()
            .filter(|(f, _)| !end_frames.contains(&Some(*f)))
            .map(|(_, &r)| r)
            .chain(
                self.nodes
                    .iter()
                    .enumerate()
                    .filter(|(v, _)| !ends.contains(&Some(*v)))
                    .map(|(_, n)| n.0),
            );
        for r in sides.filter(|_| s.node == 0) {
            if overlaps(probe, grow(r, EPS)) {
                self.look(simple.len());
                s.hug += side_run(r, &simple, &own).round() as usize;
            }
        }
        // Backtracking along the flow, both ways, as the harness counts.
        let (mut fwd, mut back) = (0.0f64, 0.0f64);
        for w in pts.windows(2) {
            let d = along(self.dir, w[1]) - along(self.dir, w[0]);
            if d > 0.0 {
                fwd += d;
            } else {
                back -= d;
            }
        }
        if fwd.min(back) > WIGGLE_TOL {
            s.wiggle = 1;
        }
        let Some(acc) = acc else { return s };
        // The tests against other edges run on the few segments that
        // stay within [`SIMPLIFY_TOL`] of the drawn curve.
        self.against(info, me, &simple, acc, s, true)
    }

    /// `s` (a scene score) with the parts scored against the accepted
    /// edges `acc` filled in for edge `me` drawn as the simplified
    /// polyline `pts`: px on top of them and, when `full`, px cramped
    /// beside them and crossings (left 0 otherwise: a quick test for an
    /// edge on top of another).
    fn against(
        &self,
        info: &Info,
        me: usize,
        pts: &[Pt],
        acc: &mut Accepted,
        mut s: Score,
        full: bool,
    ) -> Score {
        // How far a segment may lie to count: the separation, or only
        // on top when not `full`.
        let reach = if full { EDGE_SEP } else { EDGE_SPACING };
        // Shared-end exclusion zones per accepted edge, built on demand.
        let shared = |acc: &Accepted, j: usize| -> Vec<R> {
            let Some(other) = acc.ends[j] else {
                return Vec::new();
            };
            [info.ends.0, info.ends.1]
                .into_iter()
                .filter(|x| *x == other.0 || *x == other.1)
                .map(|x| self.end_rect(x))
                .collect()
        };
        // Parallel runs: per sample, the nearest segment of each nearby
        // accepted edge; (edge, px on top, px within [`EDGE_SEP`]).
        let mut run: Vec<(usize, f64, f64)> = Vec::new();
        let mut zones: Vec<(usize, Vec<R>)> = Vec::new();
        let zone_of = |zones: &mut Vec<(usize, Vec<R>)>, acc: &Accepted, j: usize| -> usize {
            if let Some(k) = zones.iter().position(|z| z.0 == j) {
                return k;
            }
            zones.push((j, shared(acc, j)));
            zones.len() - 1
        };
        // Crossings, merged per accepted edge, from the same nearby
        // segments (a superset of those its box meets).
        let mut hits: Vec<(usize, Pt)> = Vec::new();
        for w in pts.windows(2) {
            let mut ids = acc.near(grow(bounds(w), reach), me);
            self.look(ids.len());
            for &id in ids.iter().filter(|_| full) {
                let (j, a, b, _) = acc.segs[id as usize];
                // Counted up to a shared end, as the harness does.
                let Some(p) = seg_cross(w[0], w[1], a, b) else {
                    continue;
                };
                if hits
                    .iter()
                    .all(|&(hj, q)| hj != j || dist(p, q) >= CROSS_MERGE)
                {
                    hits.push((j, p));
                }
            }
            let l = dist(w[0], w[1]);
            if l == 0.0 {
                continue;
            }
            let d = ((w[1].0 - w[0].0) / l, (w[1].1 - w[0].1) / l);
            // Only nearly parallel segments that come within the
            // separation at all can hold a sample's run: the rest are
            // dropped before sampling.
            ids.retain(|&id| {
                let (_, a, b, _) = acc.segs[id as usize];
                let (vx, vy) = (b.0 - a.0, b.1 - a.1);
                let lv = norm(vx, vy);
                lv > 0.0
                    && ((d.0 * vx + d.1 * vy) / lv).abs() > PARALLEL_COS
                    && seg_seg_dist(w[0], w[1], a, b) < reach
            });
            if ids.is_empty() {
                continue;
            }
            let k = sample_count(l);
            // What one sample stands for: [`SAMPLE_STEP`], as the harness
            // counts it, unless the segment is sampled more coarsely.
            let step = SAMPLE_STEP.max(l / k as f64);
            let at = |q: usize| {
                let t = q as f64 / k as f64;
                (
                    w[0].0 + t * (w[1].0 - w[0].0),
                    w[0].1 + t * (w[1].1 - w[0].1),
                )
            };
            // Per nearby segment: the samples within [`EDGE_SEP`] and
            // within [`EDGE_SPACING`] of it, two runs of consecutive
            // samples (the distance to a segment is convex along `w`),
            // looked for only where its projection reaches (a sample
            // further along is further away).
            let mut per: Vec<(usize, Run, Run)> = Vec::new();
            for &id in &ids {
                let (j, a, b, _) = acc.segs[id as usize];
                let along = |p: Pt| (p.0 - w[0].0) * d.0 + (p.1 - w[0].1) * d.1;
                let (ta, tb) = (along(a), along(b));
                let lo = ((ta.min(tb) - reach) / l * k as f64).floor().max(0.0) as usize;
                let hi =
                    (((ta.max(tb) + reach) / l * k as f64).ceil().max(0.0) as usize).min(k - 1);
                let f = |q: usize| {
                    self.look(1);
                    seg_dist(at(q), a, b)
                };
                let Some((m, fm)) = convex_min(lo, hi, &f) else {
                    continue;
                };
                if fm >= reach {
                    continue;
                }
                per.push((
                    j,
                    below(lo, hi, m, reach, &f),
                    (fm < EDGE_SPACING)
                        .then(|| below(lo, hi, m, EDGE_SPACING, &f))
                        .flatten(),
                ));
            }
            // Per accepted edge: the samples near any of its segments,
            // less those near an end the two edges share.
            per.sort_by_key(|x| x.0);
            let mut from = 0;
            while from < per.len() {
                let j = per[from].0;
                let to = from + per[from..].iter().take_while(|x| x.0 == j).count();
                let grp = &per[from..to];
                from = to;
                let z = zone_of(&mut zones, acc, j);
                let skip: Vec<(usize, usize)> = zones[z]
                    .1
                    .iter()
                    .filter_map(|&r| {
                        let f = |q: usize| rect_dist(r, at(q));
                        let (m, fm) = convex_min(0, k - 1, &f)?;
                        (fm < PORT_ZONE)
                            .then(|| below(0, k - 1, m, PORT_ZONE, &f))
                            .flatten()
                    })
                    .collect();
                let sep: Vec<(usize, usize)> = grp.iter().filter_map(|x| x.1).collect();
                let on: Vec<(usize, usize)> = grp.iter().filter_map(|x| x.2).collect();
                let n_sep = covered(sep, &skip);
                if n_sep == 0 {
                    continue;
                }
                let n_on = covered(on, &skip);
                let (on, sep) = (n_on as f64 * step, n_sep as f64 * step);
                match run.iter_mut().find(|x| x.0 == j) {
                    Some(x) => {
                        x.1 += on;
                        x.2 += sep;
                    }
                    None => run.push((j, on, sep)),
                }
            }
        }
        let px = |v: f64, min: f64| if v > min { v.round() as usize } else { 0 };
        s.overlap = run.iter().map(|x| px(x.1, MIN_OVERLAP_RUN)).sum();
        // Edges sharing an end (a fan into a hub) converge by design:
        // only their runs on top of each other count, not how close they
        // come (the nudge spreads them where they are orthogonal).
        if full {
            s.tight = run
                .iter()
                .filter(|x| zones.iter().all(|z| z.0 != x.0 || z.1.is_empty()))
                .map(|x| px(x.2, TIGHT_RUN_MIN))
                .sum();
        }
        s.cross = hits.len();
        // Ends as drawn: this path's own and each crossed edge's first
        // and last points.
        let mut tips = vec![pts[0], pts[pts.len() - 1]];
        let mut seen: Vec<usize> = Vec::new();
        for &(j, _) in &hits {
            if seen.contains(&j) {
                continue;
            }
            seen.push(j);
            let ids = &acc.of_edge[j];
            if let (Some(&f), Some(&l)) = (ids.first(), ids.last()) {
                tips.extend([acc.segs[f as usize].1, acc.segs[l as usize].2]);
            }
        }
        s.head = hits
            .iter()
            .filter(|&&(_, p)| tips.iter().any(|&q| dist(p, q) < HEAD_ZONE))
            .count();
        s
    }
}

/// An inclusive run of sample indices.
type Run = Option<(usize, usize)>;

/// The first sample in `lo..=hi` where the convex `f` is least, and its
/// value; `None` for an empty range.
fn convex_min(lo: usize, hi: usize, f: &dyn Fn(usize) -> f64) -> Option<(usize, f64)> {
    if lo > hi {
        return None;
    }
    let (mut a, mut b) = (lo, hi);
    while b - a > 2 {
        let (m1, m2) = (a + (b - a) / 3, b - (b - a) / 3);
        let (f1, f2) = (f(m1), f(m2));
        if f1 < f2 {
            b = m2 - 1;
        } else if f1 > f2 {
            a = m1 + 1;
        } else {
            (a, b) = (m1, m2);
        }
    }
    (a..=b)
        .map(|q| (q, f(q)))
        .fold(None, |best: Option<(usize, f64)>, x| match best {
            Some(b) if b.1 <= x.1 => Some(b),
            _ => Some(x),
        })
}

/// The samples of `lo..=hi` where the convex `f`, least at `m`, is
/// below `r` (`f(m) < r`): one run round `m`.
fn below(lo: usize, hi: usize, m: usize, r: f64, f: &dyn Fn(usize) -> f64) -> Run {
    if f(m) >= r {
        return None;
    }
    // The first sample of [lo, m] below r, and the last of [m, hi].
    let (mut a, mut b) = (lo, m);
    while a < b {
        let c = a + (b - a) / 2;
        if f(c) < r {
            b = c;
        } else {
            a = c + 1;
        }
    }
    let first = a;
    let (mut a, mut b) = (m, hi);
    while a < b {
        let c = b - (b - a) / 2;
        if f(c) < r {
            a = c;
        } else {
            b = c - 1;
        }
    }
    Some((first, a))
}

/// How many samples the runs `runs` cover outside the runs `skip`.
fn covered(mut runs: Vec<(usize, usize)>, skip: &[(usize, usize)]) -> usize {
    runs.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for r in runs {
        match merged.last_mut() {
            Some(m) if r.0 <= m.1 + 1 => m.1 = m.1.max(r.1),
            _ => merged.push(r),
        }
    }
    merged
        .iter()
        .map(|&(a, b)| {
            // Samples of [a, b] inside some skipped run, each once.
            let mut cut: Vec<(usize, usize)> = skip
                .iter()
                .filter_map(|&(c, d)| (c.max(a) <= d.min(b)).then_some((c.max(a), d.min(b))))
                .collect();
            cut.sort_unstable();
            let (mut gone, mut upto) = (0, a);
            for (c, d) in cut {
                let c = c.max(upto);
                if c <= d {
                    gone += d - c + 1;
                    upto = d + 1;
                }
            }
            (b - a + 1) - gone
        })
        .sum()
}

/// Most middle-leg lines one [`detour`] builds a path on (nearest the
/// straight route first).
const DETOUR_TRIES: usize = 12;
/// Most lanes [`detour`] scans for each of its two cross legs, and
/// most middle-leg lines it builds a path on, crowded ones included.
const DETOUR_LANES: usize = 24;
/// Work a repair may spend on [`detour`]s, apart from the lattice
/// budget (which a long edge may have found exhausted): one unit per
/// obstacle set up, per line-against-obstacle test, per lane looked at
/// and per candidate point scored.
const DETOUR_TOTAL_WORK: usize = 120_000;
/// Work a repair keeps apart for the rescue [`detour`] (frames no
/// walls) of the edges still through a node once the searches, the
/// channel detour or their budgets gave out: never left as a free curve
/// across the nodes between its ends (G6). A rescue sets up one wall per
/// node and chip and tests a few lanes against them, so this covers a
/// handful of such edges in the densest fixtures.
const RESCUE_TOTAL_WORK: usize = 80_000;
/// How near (px) an edge's end a crossing reads as touching its
/// arrowhead: the head itself plus a port zone beyond it (see
/// [`Score::head`]).
const HEAD_ZONE: f64 = ARROW_MARKER_PX + PORT_ZONE;
/// px of run on top of other edges a `route()` reroute may add per
/// foreign frame it no longer enters (see `frames_hard` in [`repair`]):
/// two cramped runs, about a port stub shared beside a hub's other legs,
/// which the nudge spreads; a longer stack is the lesser trade (G1).
const FRAME_OVERLAP_TRADE: usize = 2 * TIGHT_RUN_MIN as usize;
/// Detour candidates, best against the scene first, that are also
/// scored against the accepted edges to pick one (the first clear of
/// them ends it): enough to step past the lanes beside other edges'
/// legs that [`detour`] adds.
const DETOUR_RANKED: usize = 6;
/// The most of [`DETOUR_TOTAL_WORK`] one edge's detour may spend, so an
/// edge with no clear leg set cannot starve the (longer) edges after it.
const DETOUR_EDGE_WORK: usize = 30_000;

/// One [`detour`] candidate: its score against the scene, how many of
/// its legs run beside an accepted one, its length, corners and drawn
/// points.
type DetourCand = (Score, usize, f64, Vec<Pt>, Vec<Pt>);

/// Shortest accepted leg (px) [`detour`] keeps its lanes an edge
/// separation off: the long straight runs other paths would lie on;
/// the short pieces of free curves only cross.
const DETOUR_LEG_MIN: f64 = 4.0 * TIGHT_RUN_MIN;

/// Strictly inside `r`.
fn within(r: R, p: Pt) -> bool {
    p.0 > r.0 + EPS && p.0 < r.2 - EPS && p.1 > r.1 + EPS && p.1 < r.3 - EPS
}

/// How far (px) beyond an obstacle's wall (already grown by its
/// separation) [`detour`] puts its first lane.
const DETOUR_AIR: f64 = SEP_AIR;

/// Channel lines along one axis from the obstacle bounds `v` (pairs of
/// low, high): the middle of every gap between consecutive bounds, two
/// tracks outside every bound ([`DETOUR_AIR`], then one [`EDGE_SEP`]
/// further, so two detours past one wall need not share a line), the
/// outer bounds and `extra`.
fn lanes(v: Vec<f64>, outer: (f64, f64), extra: &[f64]) -> Vec<f64> {
    let next = DETOUR_AIR + EDGE_SEP;
    let mut out: Vec<f64> = v
        .chunks(2)
        .flat_map(|w| {
            [
                w[0] - DETOUR_AIR,
                w[1] + DETOUR_AIR,
                w[0] - next,
                w[1] + next,
            ]
        })
        .collect();
    let v = sorted_lines(v);
    out.extend(v.windows(2).map(|w| (w[0] + w[1]) / 2.0));
    out.extend([outer.0, outer.1]);
    out.extend_from_slice(extra);
    sorted_lines(out)
}

/// Last resort for an edge the lattice left hitting a node or frame
/// (its window over the cap, or the budget spent): an orthogonal path
/// out along its source stub, across on one channel line, and in along
/// its target stub — `stub, (xa, ·), (xa, m), (xb, m), (xb, ·), stub` —
/// on the channel lines between the boxes and round the drawing's
/// outside, every leg clear of every node, chip and foreign frame. Of
/// those, the best scored against the scene and the accepted edges (so
/// a lane another detour already took ranks below the next one), then
/// the shortest; `None` when no leg set is clear or `work` runs out.
/// Never a free curve across the drawing (G6).
///
/// `cross_frames`: the foreign frames are no walls (the rescue when no
/// leg set clears them, see [`RESCUE_TOTAL_WORK`]): the legs still keep
/// clear of every node and chip, and the scene score prices each frame
/// they enter.
fn detour(
    w: &World,
    info: &Info,
    me: usize,
    e: &crate::scene::SceneEdge,
    acc: &mut Accepted,
    work: &mut usize,
    cross_frames: bool,
) -> Option<(Score, Vec<Pt>, Vec<Pt>)> {
    let blocked: &[usize] = if cross_frames {
        &[]
    } else {
        &info.blocked_frames
    };
    // Setting up costs a unit per obstacle (walls and lanes).
    let setup = w.nodes.len() + w.chips.len() + blocked.len();
    *work = work.checked_sub(setup)?;
    let (p0, p3) = drawn_ends(e);
    let (r0, r3) = (w.end_rect(info.ends.0), w.end_rect(info.ends.1));
    let (s0, s3) = (side_of(r0, p0), side_of(r3, p3));
    let len = ROUTE_CLEAR + STUB_EXTRA;
    // Transposed so the source stub leaves along x.
    let flip = matches!(s0, Side::Top | Side::Bottom);
    let t = |p: Pt| if flip { (p.1, p.0) } else { p };
    let tr = |r: R| if flip { (r.1, r.0, r.3, r.2) } else { r };
    let (a, b) = (t(stub_of(r0, s0, p0, len)), t(stub_of(r3, s3, p3, len)));
    let (n0, n3) = (t(normal(s0)), t(normal(s3)));
    let walls: Vec<R> = w
        .nodes
        .iter()
        .map(|&(r, _)| grow(r, NODE_SEP))
        .chain(w.chips.iter().map(|&r| grow(r, CHIP_SEP)))
        .chain(blocked.iter().map(|&f| grow(w.frames[f], FRAME_SEP)))
        .map(tr)
        .collect();
    // The stubs stand in the open, or no leg set can be clear.
    if walls.iter().any(|&r| within(r, a) || within(r, b)) {
        return None;
    }
    let o = tr(w.outer);
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for r in &walls {
        xs.extend([r.0, r.2]);
        ys.extend([r.1, r.3]);
    }
    // Tracks beside the accepted edges' long straight legs near the
    // ends, an edge separation off each side: a lane on such a leg's
    // own line would lie on top of it (G1), so a detour gets a parallel
    // track at the right distance instead.
    let (mut ex, mut ey) = (vec![a.0, b.0], vec![a.1, b.1]);
    let probe = grow(bounds(&[p0, p3]), ROUTE_WINDOW);
    let near = acc.near(probe, me);
    *work = work.checked_sub(near.len())?;
    // Those legs as (runs along y, cross position, span along, the
    // separation a lane keeps off it).
    let mut legs: Vec<(bool, f64, f64, f64, f64)> = Vec::new();
    for id in near {
        let (_, q0, q1, _) = acc.segs[id as usize];
        let (q0, q1) = (t(q0), t(q1));
        // Nearly axis-parallel (a simplified rounded path is off by
        // a pixel or so), at its mean cross position.
        let (dx, dy) = ((q1.0 - q0.0).abs(), (q1.1 - q0.1).abs());
        let l = norm(dx, dy);
        if dx > DETOUR_LEG_MIN && dx / l > PARALLEL_COS {
            let c = (q0.1 + q1.1) / 2.0;
            ey.extend([c - EDGE_SEP, c + EDGE_SEP]);
            legs.push((false, c, q0.0.min(q1.0), q0.0.max(q1.0), EDGE_SEP));
        } else if dy > DETOUR_LEG_MIN && dy / l > PARALLEL_COS {
            let c = (q0.0 + q1.0) / 2.0;
            ex.extend([c - EDGE_SEP, c + EDGE_SEP]);
            legs.push((true, c, q0.1.min(q1.1), q0.1.max(q1.1), EDGE_SEP));
        }
    }
    // The borders of the frames the legs may cross or run inside (G4):
    // kept [`FRAME_SEP`] off like an accepted leg, with a track that far
    // off each side.
    let border = FRAME_SEP + DETOUR_AIR;
    for (f, &r) in w.frames.iter().enumerate() {
        if blocked.contains(&f) || !overlaps(r, probe) {
            continue;
        }
        let r = tr(r);
        *work = work.checked_sub(1)?;
        for c in [r.1, r.3] {
            ey.extend([c - border, c + border]);
            legs.push((false, c, r.0, r.2, FRAME_SEP));
        }
        for c in [r.0, r.2] {
            ex.extend([c - border, c + border]);
            legs.push((true, c, r.1, r.3, FRAME_SEP));
        }
    }
    // Does a leg along y (`vert`) at `c` from `lo` to `hi` run within an
    // edge separation of one of them for over MIN_OVERLAP_RUN?
    let crowded = |vert: bool, c: f64, lo: f64, hi: f64| {
        let (lo, hi) = (lo.min(hi), lo.max(hi));
        legs.iter().any(|&(v, lc, l0, l1, sep)| {
            v == vert && (lc - c).abs() < sep - EPS && hi.min(l1) - lo.max(l0) > MIN_OVERLAP_RUN
        })
    };
    let xs = lanes(xs, (o.0, o.2), &ex);
    let ys = lanes(ys, (o.1, o.3), &ey);
    // The free stretch of the line through `p` along x (`horiz`) or y,
    // round `p`: no wall's open box meets it. A leg from `p` along that
    // line is clear exactly when its far end lies inside the stretch.
    let free = |p: Pt, horiz: bool| -> (f64, f64) {
        let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
        for r in &walls {
            let (c, v, a0, a1, b0, b1) = if horiz {
                (p.1, p.0, r.1, r.3, r.0, r.2)
            } else {
                (p.0, p.1, r.0, r.2, r.1, r.3)
            };
            if c <= a0 + EPS || c >= a1 - EPS {
                continue;
            }
            if b1 <= v + EPS {
                lo = lo.max(b1);
            } else if b0 >= v - EPS {
                hi = hi.min(b0);
            } else {
                return (v, v);
            }
        }
        (lo, hi)
    };
    let within_span = |v: f64, s: (f64, f64)| v >= s.0 - EPS && v <= s.1 + EPS;
    // A cross leg leaves the source stub outward (or straight on) and
    // reaches the target stub from outside, along a clear stub line;
    // each lane keeps the free stretch of its own cross line.
    let (ha, hb) = (free(a, true), free(b, true));
    let out_a = |x: f64| (n0.0 == 0.0 || (x - a.0) * n0.0 >= -EPS) && within_span(x, ha);
    let out_b = |x: f64| (n3.0 == 0.0 || (x - b.0) * n3.0 >= -EPS) && within_span(x, hb);
    let nearest = |from: f64, y: f64, ok: &dyn Fn(f64) -> bool| -> Vec<(f64, (f64, f64))> {
        let mut v: Vec<f64> = xs.iter().copied().filter(|&x| ok(x)).collect();
        v.sort_by(|p, q| {
            (p - from)
                .abs()
                .total_cmp(&(q - from).abs())
                .then(p.total_cmp(q))
        });
        v.truncate(DETOUR_LANES);
        v.into_iter().map(|x| (x, free((x, y), false))).collect()
    };
    let (ka, kb) = (nearest(a.0, a.1, &out_a), nearest(b.0, b.1, &out_b));
    *work = work.checked_sub((ka.len() + kb.len() + 2) * walls.len())?;
    let mut ms: Vec<f64> = ys.clone();
    ms.sort_by(|p, q| {
        ((p - a.1).abs() + (p - b.1).abs())
            .total_cmp(&((q - a.1).abs() + (q - b.1).abs()))
            .then(p.total_cmp(q))
    });
    // (scene score, legs beside accepted ones, length, corners, points)
    let mut found: Vec<DetourCand> = Vec::new();
    let (mut tried, mut looked) = (0, 0);
    for m in ms {
        if tried == DETOUR_TRIES || looked == DETOUR_LANES {
            break;
        }
        let Some(left) = work.checked_sub(ka.len() + kb.len() + walls.len()) else {
            break; // out of work: the best so far
        };
        *work = left;
        // The nearest lane each side whose cross line reaches `m`, kept
        // an edge separation off the accepted legs beside it when one
        // is (else the nearest).
        let pick = |k: &[(f64, (f64, f64))], from: f64| {
            let reach = k.iter().filter(|l| within_span(m, l.1));
            reach
                .clone()
                .find(|l| !crowded(true, l.0, from, m))
                .or_else(|| reach.clone().next())
                .map(|l| l.0)
        };
        let (Some(xa), Some(xb)) = (pick(&ka, a.1), pick(&kb, b.1)) else {
            continue;
        };
        if !within_span(xb, free((xa, m), true)) {
            continue;
        }
        // Legs within a separation of an accepted one or a frame
        // border: ranked after the scene's hard hits, before the rest of
        // its score (a run on top of another edge is worse than a
        // longer way round, G1/G4). Only clear leg sets count towards
        // [`DETOUR_TRIES`], so a crowded channel cannot use them up.
        let busy = usize::from(crowded(true, xa, a.1, m))
            + usize::from(crowded(false, m, xa, xb))
            + usize::from(crowded(true, xb, b.1, m));
        looked += 1;
        if busy == 0 {
            tried += 1;
        }
        let un = |p: Pt| if flip { (p.1, p.0) } else { p };
        let corners = collapse(
            [a, (xa, a.1), (xa, m), (xb, m), (xb, b.1), b]
                .into_iter()
                .map(un)
                .collect::<Vec<_>>(),
        );
        let mut c = vec![p0];
        c.extend(corners);
        c.push(p3);
        let corners = collapse(c);
        let pts = drawn_with(e, &smooth(&corners, CORNER_R));
        let Some(left) = work.checked_sub(pts.len() * SCORE_POINT_WORK) else {
            break;
        };
        *work = left;
        let sc = w.score(info, me, &pts, None);
        let l: f64 = corners.windows(2).map(|q| dist(q[0], q[1])).sum();
        found.push((sc, busy, l, corners, pts));
    }
    // Ranked against the scene, then the best few against the accepted
    // edges too, so a lane another detour already took loses.
    found.sort_by(|a, b| {
        a.0.hard()
            .cmp(&b.0.hard())
            .then(a.1.cmp(&b.1))
            .then(a.0.cmp(&b.0))
            .then(a.2.total_cmp(&b.2))
    });
    found.truncate(DETOUR_RANKED);
    let mut best: Option<(Score, f64, Vec<Pt>, Vec<Pt>)> = None;
    for (_, _, l, corners, pts) in found {
        let s = w.score(info, me, &pts, Some(&mut *acc));
        if best
            .as_ref()
            .map_or(true, |b| s.cmp(&b.0).then(l.total_cmp(&b.1)).is_lt())
        {
            best = Some((s, l, corners, pts));
        }
        // Clear of the accepted edges too: no later one can beat it
        // but by length.
        if s.overlap == 0 && s.tight == 0 {
            break;
        }
        let Some(left) = work.checked_sub(w.spent()) else {
            break;
        };
        *work = left;
    }
    let _ = w.spent();
    best.map(|b| (b.0, b.2, b.3))
}

/// How long (px) the polyline `pts` runs along the sides of `r`,
/// nearly parallel to one and within [`CLEAR_GAP`] of it (inside or
/// out), not counting what lies within [`PORT_ZONE`] of a rect in `own`
/// (the edge's own ends): summed over the sides ridden for more than
/// [`TIGHT_RUN_MIN`], as the harness counts a hug.
fn side_run(r: R, pts: &[Pt], own: &[R]) -> f64 {
    // Per side (top, bottom, left, right): px along it.
    let mut run = [0.0f64; 4];
    for w in pts.windows(2) {
        let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let l = norm(dx, dy);
        if l <= EPS {
            continue;
        }
        let m = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
        // (horizontal, across, along lo, along hi) of the segment.
        let (horiz, c, a, b) = if dx.abs() / l > PARALLEL_COS {
            (true, m.1, w[0].0.min(w[1].0), w[0].0.max(w[1].0))
        } else if dy.abs() / l > PARALLEL_COS {
            (false, m.0, w[0].1.min(w[1].1), w[0].1.max(w[1].1))
        } else {
            continue;
        };
        let (lines, lo, hi, k0) = if horiz {
            ([r.1, r.3], r.0, r.2, 0)
        } else {
            ([r.0, r.2], r.1, r.3, 2)
        };
        let (a, b) = (a.max(lo), b.min(hi));
        if b <= a {
            continue;
        }
        // The part near an own end does not count.
        let mut cut = 0.0;
        for o in own {
            let (olo, ohi, clo, chi) = if horiz {
                (o.0, o.2, o.1, o.3)
            } else {
                (o.1, o.3, o.0, o.2)
            };
            let dc = (clo - c).max(c - chi).max(0.0);
            if dc < PORT_ZONE {
                let reach = (PORT_ZONE * PORT_ZONE - dc * dc).sqrt();
                cut += (b.min(ohi + reach) - a.max(olo - reach)).max(0.0);
            }
        }
        let len = (b - a - cut).max(0.0);
        for (k, line) in lines.into_iter().enumerate() {
            if (c - line).abs() < CLEAR_GAP {
                run[k0 + k] += len;
            }
        }
    }
    run.iter().filter(|&&v| v > TIGHT_RUN_MIN).sum()
}

/// Px of `pts` running along a frame border, inside or out, within
/// [`BORDER_HUG_GAP`] of the border line and inside its span: what the
/// harness counts as riding a border (G4), its own frames' included,
/// without the harness's minimum run, so any added ride shows.
fn border_px(frames: &[R], pts: &[Pt]) -> f64 {
    let bb = grow(bounds(pts), BORDER_HUG_GAP);
    let mut px = 0.0;
    for &r in frames.iter().filter(|&&r| overlaps(bb, r)) {
        for w in pts.windows(2) {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let l = norm(dx, dy);
            if l <= EPS {
                continue;
            }
            let m = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
            let (c, a, b, lines, lo, hi) = if dx.abs() / l > PARALLEL_COS {
                (
                    m.1,
                    w[0].0.min(w[1].0),
                    w[0].0.max(w[1].0),
                    [r.1, r.3],
                    r.0,
                    r.2,
                )
            } else if dy.abs() / l > PARALLEL_COS {
                (
                    m.0,
                    w[0].1.min(w[1].1),
                    w[0].1.max(w[1].1),
                    [r.0, r.2],
                    r.1,
                    r.3,
                )
            } else {
                continue;
            };
            let along = (b.min(hi) - a.max(lo)).max(0.0);
            px += along
                * lines
                    .iter()
                    .filter(|&&v| (c - v).abs() < BORDER_HUG_GAP)
                    .count() as f64;
        }
    }
    px
}

/// Shortest distance between segments `a`–`b` and `c`–`d`.
fn seg_seg_dist(a: Pt, b: Pt, c: Pt, d: Pt) -> f64 {
    if seg_cross(a, b, c, d).is_some() {
        return 0.0;
    }
    seg_dist(a, c, d)
        .min(seg_dist(b, c, d))
        .min(seg_dist(c, a, b))
        .min(seg_dist(d, a, b))
}

/// Proper intersection of segments `a`–`b` and `c`–`d`.
fn seg_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    let r = (b.0 - a.0, b.1 - a.1);
    let s = (d.0 - c.0, d.1 - c.1);
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-9 {
        return None;
    }
    let t = ((c.0 - a.0) * s.1 - (c.1 - a.1) * s.0) / den;
    let u = ((c.0 - a.0) * r.1 - (c.1 - a.1) * r.0) / den;
    ((0.0..1.0).contains(&t) && (0.0..1.0).contains(&u)).then_some((a.0 + t * r.0, a.1 + t * r.1))
}

fn sorted_lines(mut v: Vec<f64>) -> Vec<f64> {
    v.retain(|x| x.is_finite());
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < LINE_EPS);
    v
}

/// Lattice lines along one axis: the given bounds plus the midline of
/// every gap between consecutive bounds (the channel centres).
fn with_midlines(v: Vec<f64>) -> Vec<f64> {
    let v = sorted_lines(v);
    let mut out = v.clone();
    for w in v.windows(2) {
        if w[1] - w[0] > MIDLINE_MIN_GAP {
            out.push((w[0] + w[1]) / 2.0);
        }
    }
    sorted_lines(out)
}

/// `lines` with every stub coordinate in `stubs` put in exactly: a line
/// within [`LINE_EPS`] of a stub gives way to it, so a stub always sits
/// on a lattice vertex of its own.
fn with_stubs(lines: Vec<f64>, mut stubs: Vec<f64>) -> Vec<f64> {
    stubs.retain(|x| x.is_finite());
    stubs.sort_by(f64::total_cmp);
    stubs.dedup();
    let mut out: Vec<f64> = lines
        .into_iter()
        .filter(|x| {
            let k = stubs.partition_point(|s| *s < x - LINE_EPS);
            k >= stubs.len() || (stubs[k] - x).abs() >= LINE_EPS
        })
        .collect();
    out.extend(stubs);
    out.sort_by(f64::total_cmp);
    out
}

/// What a lattice step meets, fixed for the whole repair.
#[derive(Clone, Copy)]
struct Fixed {
    /// Band price of the step, or infinity when it runs through an
    /// obstacle.
    cost: f64,
    /// The step runs inside some obstacle's clearance band.
    banded: bool,
}

/// One sparse orthogonal lattice over a search window, with blocked,
/// banded, frame-tagged and priced steps. Built per edge from the
/// obstacles and accepted segments inside the window only, so its size
/// follows what lies near the edge, not the whole drawing.
/// `h[j * (nx - 1) + i]` is the step `(i, j) -> (i + 1, j)`;
/// `v[j * nx + i]` the step `(i, j) -> (i, j + 1)`.
struct Lattice {
    xs: Vec<f64>,
    ys: Vec<f64>,
    fixed_h: Vec<Fixed>,
    fixed_v: Vec<Fixed>,
    /// The drawing's indices of the frames this lattice tags, ascending:
    /// only those meeting its window, so the bitsets follow the window,
    /// not the drawing's frame count.
    frame_ids: Vec<usize>,
    /// u64 words of frame bits per step.
    words: usize,
    /// Per step, `words` words: the frames whose clearance it runs inside,
    /// bit `k` standing for `frame_ids[k]`.
    frame_h: Vec<u64>,
    frame_v: Vec<u64>,
    /// Price of the accepted edges per step.
    cost_h: Vec<f64>,
    /// [`Self::cost_h`] for the vertical steps.
    cost_v: Vec<f64>,
}

impl Lattice {
    fn new(xs: Vec<f64>, ys: Vec<f64>, frame_ids: Vec<usize>) -> Lattice {
        let (nx, ny) = (xs.len(), ys.len());
        let nh = nx.saturating_sub(1) * ny;
        let nv = nx * ny.saturating_sub(1);
        let words = Self::words_for(frame_ids.len());
        let free = Fixed {
            cost: 0.0,
            banded: false,
        };
        Lattice {
            xs,
            ys,
            fixed_h: vec![free; nh],
            fixed_v: vec![free; nv],
            frame_ids,
            words,
            frame_h: vec![0; nh * words],
            frame_v: vec![0; nv * words],
            cost_h: vec![0.0; nh],
            cost_v: vec![0.0; nv],
        }
    }

    /// Bitset words per step for `n` frames.
    fn words_for(n: usize) -> usize {
        n.div_ceil(64)
    }

    /// The lattice of the drawing `w` inside the window `o` for
    /// clearance `clear`, with a vertex on every stub in `stubs`, priced
    /// with the accepted segments of edges other than `me`, the frames
    /// in `blocked` (ascending) ringed by a band. `None` when it would
    /// exceed the per-lattice cap or the work left; the work it does is
    /// charged to `work`.
    #[allow(clippy::too_many_arguments)]
    fn build(
        w: &World,
        clear: f64,
        stubs: &[Pt],
        o: R,
        acc: &mut Accepted,
        me: usize,
        blocked: &[usize],
        tracks: bool,
        work: &mut Work,
    ) -> Option<Lattice> {
        let solids: Vec<(R, f64)> = w
            .solids(clear)
            .into_iter()
            .filter(|&(r, c)| overlaps(grow(r, c), o))
            .collect();
        // Every frame meeting the window: a blocked one is tagged over its
        // rect and banded round it; any frame gets a track
        // [`ROUTE_FRAME_CLEAR`] off each border, inside and out.
        let fences: Vec<(usize, R)> = w
            .frames
            .iter()
            .copied()
            .enumerate()
            .filter(|&(_, f)| overlaps(grow(f, ROUTE_FRAME_CLEAR), o))
            .collect();
        let near_ids = acc.near(o, me);
        let labels: Vec<R> = w
            .labels
            .iter()
            .filter(|l| l.0 != me)
            .map(|l| grow(l.1, LABEL_CLEAR))
            .filter(|&g| overlaps(g, o))
            .collect();
        let clip = |v: f64, lo: f64, hi: f64| v > lo && v < hi;
        let mut xs = vec![o.0, o.2];
        let mut ys = vec![o.1, o.3];
        let grown = solids.iter().map(|&(r, c)| grow(r, c));
        // A blocked frame's track runs outside it, an allowed one's
        // inside: the side the path may be on.
        let rings = fences.iter().map(|f| {
            if blocked.binary_search(&f.0).is_ok() {
                grow(f.1, ROUTE_FRAME_CLEAR)
            } else {
                grow(f.1, -ROUTE_FRAME_CLEAR)
            }
        });
        for g in grown.chain(rings) {
            xs.extend([g.0, g.2].into_iter().filter(|&x| clip(x, o.0, o.2)));
            ys.extend([g.1, g.3].into_iter().filter(|&y| clip(y, o.1, o.3)));
        }
        // Offset tracks beside the accepted long legs that took a channel
        // line: a path sharing that channel gets a parallel line
        // [`EDGE_SEP`] off instead of the leg's own (G1/G4). Only those,
        // and at most [`OFFSET_TRACKS`] per axis, so the lattice stays
        // the size of what lies in the window.
        let (base_x, base_y) = (sorted_lines(xs.clone()), sorted_lines(ys.clone()));
        let taken = |axis: &[f64], v: f64| {
            let k = axis.partition_point(|a| *a < v - EDGE_SPACING);
            k < axis.len() && (axis[k] - v).abs() < EDGE_SPACING
        };
        let off = EDGE_SEP + SEP_AIR;
        let (mut nx, mut ny) = (0, 0);
        for &id in near_ids.iter().filter(|_| tracks) {
            let (_, a, b, _) = acc.segs[id as usize];
            if (a.1 - b.1).abs() < EPS && (a.0 - b.0).abs() > TIGHT_RUN_MIN {
                if ny < OFFSET_TRACKS && taken(&base_y, a.1) {
                    ny += 1;
                    ys.extend(
                        [a.1 - off, a.1 + off]
                            .into_iter()
                            .filter(|&y| clip(y, o.1, o.3)),
                    );
                }
            } else if (a.0 - b.0).abs() < EPS
                && (a.1 - b.1).abs() > TIGHT_RUN_MIN
                && nx < OFFSET_TRACKS
                && taken(&base_x, a.0)
            {
                nx += 1;
                xs.extend(
                    [a.0 - off, a.0 + off]
                        .into_iter()
                        .filter(|&x| clip(x, o.0, o.2)),
                );
            }
        }
        let inside_o = |s: &&Pt| clip(s.0, o.0, o.2) && clip(s.1, o.1, o.3);
        let xs = with_stubs(
            with_midlines(xs),
            stubs.iter().filter(inside_o).map(|s| s.0).collect(),
        );
        let ys = with_stubs(
            with_midlines(ys),
            stubs.iter().filter(inside_o).map(|s| s.1).collect(),
        );
        let cells = xs.len() * ys.len();
        // One unit per vertex covers its fixed and priced steps and the
        // first frame word; every further word is charged too.
        let words = Self::words_for(fences.len());
        let units = cells.saturating_mul(words.max(1));
        if cells > work.max_cells || !work.take(units) {
            return None;
        }
        let ids = fences.iter().map(|f| f.0).collect();
        let mut lat = Lattice::new(xs, ys, ids);
        for &(r, c) in &solids {
            lat.solid(r, c);
        }
        for &(f, r) in &fences {
            if !tracks {
                // `route()`: the ring is tagged with the frame (a wall
                // to the strict tries, [`FRAME_COST`] when priced), one
                // pass instead of two (G13).
                lat.frame(f, grow(r, ROUTE_FRAME_CLEAR));
            } else {
                lat.frame(f, r);
                if blocked.binary_search(&f).is_ok() {
                    lat.ring(r, ROUTE_FRAME_CLEAR);
                }
            }
        }
        for &g in &labels {
            lat.soft(g, LABEL_COST);
        }
        // Frame borders, inside and out: a path keeps off them where its
        // channel has room and rides one only when nothing else is free.
        for &r in w
            .frames
            .iter()
            .filter(|&&r| overlaps(grow(r, FRAME_BAND), o))
        {
            lat.rails(r, HUG_COST);
        }
        for id in near_ids {
            let (_, a, b, _) = acc.segs[id as usize];
            let n = lat.price_seg(a, b, EDGE_SEP);
            if !work.take(n + PRICE_SEG_WORK) {
                return None;
            }
        }
        Some(lat)
    }

    /// Indices of axis values strictly inside `(lo, hi)`.
    fn strictly(axis: &[f64], lo: f64, hi: f64) -> std::ops::Range<usize> {
        let a = axis.partition_point(|v| *v <= lo + EPS);
        let b = axis.partition_point(|v| *v < hi - EPS);
        a..b.max(a)
    }

    /// Indices of steps `(k, k + 1)` whose open span meets `(lo, hi)`.
    fn steps(axis: &[f64], lo: f64, hi: f64) -> std::ops::Range<usize> {
        if axis.len() < 2 {
            return 0..0;
        }
        let a = axis.partition_point(|v| *v <= lo + EPS).saturating_sub(1);
        let b = axis.partition_point(|v| *v < hi - EPS).min(axis.len() - 1);
        a..b.max(a)
    }

    /// Mark a solid obstacle: steps through the open raw box are
    /// blocked, steps through its clearance band cost [`BAND_COST`] per
    /// px inside it.
    fn solid(&mut self, r: R, clear: f64) {
        let g = grow(r, clear);
        let nx = self.xs.len();
        let open = |lo: f64, hi: f64, a: f64, b: f64| a.min(hi) - b.max(lo) > EPS;
        for j in Self::strictly(&self.ys, g.1, g.3) {
            let y = self.ys[j];
            let in_raw = y > r.1 + EPS && y < r.3 - EPS;
            for i in Self::steps(&self.xs, g.0, g.2) {
                let (a, b) = (self.xs[i], self.xs[i + 1]);
                let f = &mut self.fixed_h[j * (nx - 1) + i];
                if in_raw && open(r.0, r.2, b, a) {
                    f.cost = f64::INFINITY;
                } else {
                    f.cost += BAND_COST * (b.min(g.2) - a.max(g.0)).max(0.0);
                    f.banded = true;
                }
            }
        }
        for i in Self::strictly(&self.xs, g.0, g.2) {
            let x = self.xs[i];
            let in_raw = x > r.0 + EPS && x < r.2 - EPS;
            for j in Self::steps(&self.ys, g.1, g.3) {
                let (a, b) = (self.ys[j], self.ys[j + 1]);
                let f = &mut self.fixed_v[j * nx + i];
                if in_raw && open(r.1, r.3, b, a) {
                    f.cost = f64::INFINITY;
                } else {
                    f.cost += BAND_COST * (b.min(g.3) - a.max(g.1)).max(0.0);
                    f.banded = true;
                }
            }
        }
    }

    /// Band the ring `clear` wide round the rect `r`: steps through it
    /// (but not through `r`) cost [`BAND_COST`] per px inside the ring
    /// and are walls to the strict tries, like a node's clearance.
    fn ring(&mut self, r: R, clear: f64) {
        let g = grow(r, clear);
        let nx = self.xs.len();
        // Length of `(a, b)` inside `(lo, hi)` but outside `(ilo, ihi)`.
        let band = |a: f64, b: f64, lo: f64, hi: f64, ilo: f64, ihi: f64, core: bool| {
            let all = (b.min(hi) - a.max(lo)).max(0.0);
            if core {
                all - (b.min(ihi) - a.max(ilo)).max(0.0)
            } else {
                all
            }
        };
        for j in Self::strictly(&self.ys, g.1, g.3) {
            let core = self.ys[j] > r.1 + EPS && self.ys[j] < r.3 - EPS;
            for i in Self::steps(&self.xs, g.0, g.2) {
                let ov = band(self.xs[i], self.xs[i + 1], g.0, g.2, r.0, r.2, core);
                if ov > EPS {
                    let f = &mut self.fixed_h[j * (nx - 1) + i];
                    f.cost += BAND_COST * ov;
                    f.banded = true;
                }
            }
        }
        for i in Self::strictly(&self.xs, g.0, g.2) {
            let core = self.xs[i] > r.0 + EPS && self.xs[i] < r.2 - EPS;
            for j in Self::steps(&self.ys, g.1, g.3) {
                let ov = band(self.ys[j], self.ys[j + 1], g.1, g.3, r.1, r.3, core);
                if ov > EPS {
                    let f = &mut self.fixed_v[j * nx + i];
                    f.cost += BAND_COST * ov;
                    f.banded = true;
                }
            }
        }
    }

    /// Price every step running inside `r` at `cost` per px of it,
    /// without blocking any.
    fn soft(&mut self, r: R, cost: f64) {
        let nx = self.xs.len();
        for j in Self::strictly(&self.ys, r.1, r.3) {
            for i in Self::steps(&self.xs, r.0, r.2) {
                let ov = self.xs[i + 1].min(r.2) - self.xs[i].max(r.0);
                self.fixed_h[j * (nx - 1) + i].cost += cost * ov.max(0.0);
            }
        }
        for i in Self::strictly(&self.xs, r.0, r.2) {
            for j in Self::steps(&self.ys, r.1, r.3) {
                let ov = self.ys[j + 1].min(r.3) - self.ys[j].max(r.1);
                self.fixed_v[j * nx + i].cost += cost * ov.max(0.0);
            }
        }
    }

    /// Price every step running along a border line of `r` (parallel
    /// to it, within [`FRAME_BAND`] of it, inside or out) at `cost` per
    /// px; steps crossing the border are not charged.
    fn rails(&mut self, r: R, cost: f64) {
        let nx = self.xs.len();
        for y in [r.1, r.3] {
            for j in Self::strictly(&self.ys, y - FRAME_BAND, y + FRAME_BAND) {
                for i in Self::steps(&self.xs, r.0, r.2) {
                    let ov = self.xs[i + 1].min(r.2) - self.xs[i].max(r.0);
                    self.fixed_h[j * (nx - 1) + i].cost += cost * ov.max(0.0);
                }
            }
        }
        for x in [r.0, r.2] {
            for i in Self::strictly(&self.xs, x - FRAME_BAND, x + FRAME_BAND) {
                for j in Self::steps(&self.ys, r.1, r.3) {
                    let ov = self.ys[j + 1].min(r.3) - self.ys[j].max(r.1);
                    self.fixed_v[j * nx + i].cost += cost * ov.max(0.0);
                }
            }
        }
    }

    /// Tag every step running inside `r` with frame `f` (the drawing's
    /// index; one of [`Self::frame_ids`]).
    fn frame(&mut self, f: usize, r: R) {
        let (nx, wd) = (self.xs.len(), self.words);
        let k = self.frame_ids.binary_search(&f).expect("frame on lattice");
        let (word, bit) = (k / 64, 1u64 << (k % 64));
        for j in Self::strictly(&self.ys, r.1, r.3) {
            for i in Self::steps(&self.xs, r.0, r.2) {
                self.frame_h[(j * (nx - 1) + i) * wd + word] |= bit;
            }
        }
        for i in Self::strictly(&self.xs, r.0, r.2) {
            for j in Self::steps(&self.ys, r.1, r.3) {
                self.frame_v[(j * nx + i) * wd + word] |= bit;
            }
        }
    }

    /// The frame bits of `frames` (the drawing's indices), as a step
    /// mask; a frame outside the window tags no step and is skipped.
    fn mask(&self, frames: &[usize]) -> Vec<u64> {
        let mut m = vec![0u64; self.words];
        for &f in frames {
            if let Ok(k) = self.frame_ids.binary_search(&f) {
                m[k / 64] |= 1u64 << (k % 64);
            }
        }
        m
    }

    /// Price one accepted segment: [`CROSS_COST`] on every step it
    /// crosses, [`RUN_COST`] per px on steps running on top of it and
    /// [`TIGHT_COST`] on steps running beside it within [`EDGE_SEP`].
    /// Returns how many steps it priced.
    fn price_seg(&mut self, a: Pt, b: Pt, band: f64) -> usize {
        let mut n = 0;
        let nx = self.xs.len();
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        // Crossings of vertical steps (x lines inside the segment's span).
        if dx.abs() > EPS {
            for i in Self::strictly(&self.xs, x0, x1) {
                let y = a.1 + (self.xs[i] - a.0) / dx * dy;
                let j = self.ys.partition_point(|v| *v <= y);
                if j >= 1 && j < self.ys.len() {
                    self.cost_v[(j - 1) * nx + i] += CROSS_COST;
                    n += 1;
                }
            }
        }
        if dy.abs() > EPS {
            for j in Self::strictly(&self.ys, y0, y1) {
                let x = a.0 + (self.ys[j] - a.1) / dy * dx;
                let i = self.xs.partition_point(|v| *v <= x);
                if i >= 1 && i < self.xs.len() {
                    self.cost_h[j * (nx - 1) + i - 1] += CROSS_COST;
                    n += 1;
                }
            }
        }
        // Runs: a near-horizontal segment prices horizontal steps on the
        // lines within the spacing; near-vertical likewise.
        let l = norm(dx, dy);
        if l <= EPS {
            return n;
        }
        let price = |d: f64| {
            if d < EDGE_SPACING {
                RUN_COST
            } else {
                TIGHT_COST
            }
        };
        if dx.abs() / l > PARALLEL_COS {
            let ym = (a.1 + b.1) / 2.0;
            for j in Self::strictly(&self.ys, ym - band, ym + band) {
                let k = price((self.ys[j] - ym).abs());
                for i in Self::steps(&self.xs, x0, x1) {
                    let ov = self.xs[i + 1].min(x1) - self.xs[i].max(x0);
                    if ov > 0.0 {
                        self.cost_h[j * (nx - 1) + i] += k * ov;
                        n += 1;
                    }
                }
            }
        } else if dy.abs() / l > PARALLEL_COS {
            let xm = (a.0 + b.0) / 2.0;
            for i in Self::strictly(&self.xs, xm - band, xm + band) {
                let k = price((self.xs[i] - xm).abs());
                for j in Self::steps(&self.ys, y0, y1) {
                    let ov = self.ys[j + 1].min(y1) - self.ys[j].max(y0);
                    if ov > 0.0 {
                        self.cost_v[j * nx + i] += k * ov;
                        n += 1;
                    }
                }
            }
        }
        n
    }

    /// Index of the line at exactly `v` (stubs are put in exactly).
    fn at(axis: &[f64], v: f64) -> Option<usize> {
        let k = axis.partition_point(|a| *a < v - EPS);
        (k < axis.len() && (axis[k] - v).abs() <= EPS).then_some(k)
    }

    fn locate(&self, p: Pt) -> Option<(usize, usize)> {
        Some((Self::at(&self.xs, p.0)?, Self::at(&self.ys, p.1)?))
    }

    /// Cost of the step from `(i, j)` by `(di, dj)` beyond its length,
    /// or `None` when it is blocked. Unless `soft`, a step through a
    /// clearance band or a frame in `mask` is blocked too; when `soft`
    /// it is priced ([`BAND_COST`], [`FRAME_COST`] per px).
    fn step(&self, i: usize, j: usize, di: i64, dj: i64, mask: &[u64], soft: bool) -> Option<f64> {
        let nx = self.xs.len();
        let (k, fixed, frames, cost) = if di != 0 {
            let lo = if di > 0 { i } else { i - 1 };
            (
                j * (nx - 1) + lo,
                &self.fixed_h,
                &self.frame_h,
                &self.cost_h,
            )
        } else {
            let lo = if dj > 0 { j } else { j - 1 };
            (lo * nx + i, &self.fixed_v, &self.frame_v, &self.cost_v)
        };
        let f = fixed[k];
        if f.cost == f64::INFINITY || (f.banded && !soft) {
            return None;
        }
        let tags = &frames[k * self.words..(k + 1) * self.words];
        let mut c = f.cost + cost[k];
        if tags.iter().zip(mask).any(|(t, m)| t & m != 0) {
            if !soft {
                return None;
            }
            let len = if di != 0 {
                (self.xs[(i as i64 + di) as usize] - self.xs[i]).abs()
            } else {
                (self.ys[(j as i64 + dj) as usize] - self.ys[j]).abs()
            };
            c += FRAME_COST * len;
        }
        Some(c)
    }
}

/// One end of a search: a port on a node side and its stub.
#[derive(Clone, Copy, Debug)]
struct Term {
    /// The port on the end's outline, and the stub just outside it.
    port: Pt,
    stub: Pt,
    /// Unit outward normal of the side, as a lattice move.
    out: (i64, i64),
    /// Extra cost of using this terminal.
    cost: f64,
}

fn axis_of(d: (i64, i64)) -> usize {
    usize::from(d.0 == 0)
}

/// A queue entry: cost `f` and state packed into one integer ordered by
/// cost, then by state, so ties break the same way on every run and the
/// output stays a function of the input alone. A non-negative `f64`'s
/// bits order like its value; reversed, as `BinaryHeap` pops the
/// largest.
fn queued(f: f64, state: usize) -> Reverse<u128> {
    let bits = if f > 0.0 { f.to_bits() } else { 0 };
    Reverse((u128::from(bits) << 64) | state as u128)
}

/// The cost and state of a queue entry.
fn unqueued(q: Reverse<u128>) -> (f64, usize) {
    (f64::from_bits((q.0 >> 64) as u64), q.0 as u64 as usize)
}

/// The A\* working arrays, reused across searches. A state's entry is
/// valid only when its stamp is the current generation, so a new search
/// never has to clear them.
#[derive(Default)]
struct Scratch {
    best: Vec<f64>,
    came: Vec<u32>,
    stamp: Vec<u32>,
    gen: u32,
    heap: BinaryHeap<Reverse<u128>>,
    /// Per vertex: a goal can still be reached from it without stepping
    /// back along the flow (the dead-end prune).
    live: Vec<bool>,
    queue: Vec<(usize, usize)>,
}

/// The work left in one repair, and the caps per search (G13). The only
/// overdraw is scoring the candidate a paid search found when its
/// points cost more than is left (see [`repair`]).
struct Work {
    /// Units left (see [`ROUTE_TOTAL_WORK`]): lattice vertices built or
    /// priced, flooded vertices, A\* pops and scoring looks.
    left: usize,
    /// A\* pops allowed per search.
    per_search: usize,
    /// Vertices allowed per search lattice.
    max_cells: usize,
    /// The priced fallback's window round the stubs (px); infinite for
    /// the whole drawing.
    walls_reach: f64,
    /// [`detour`] units left, apart from `left`: a hard edge still gets
    /// its channel detour once the searches have spent `left`.
    detour: usize,
}

impl Work {
    /// The budget of one repair. `free` is the share (0..=1) of the
    /// edges that are not fixed ([`Ctx::fixed`]); below 1 a `Routed`
    /// budget shrinks to that share of itself, never below the
    /// `PARTIAL_*` floors (and its search and scoring part never above
    /// [`PARTIAL_MAX_WORK`]), so a frame pays for what it redraws.
    fn of(mode: Mode, free: f64) -> Work {
        let mut w = Work::full(mode);
        if free < 1.0 && mode == Mode::Routed {
            let share = |full: usize, floor: usize| -> usize {
                ((full as f64 * free.max(0.0)).ceil() as usize).clamp(floor.min(full), full)
            };
            w.left = share(w.left, PARTIAL_TOTAL_WORK).min(PARTIAL_MAX_WORK);
            w.detour = share(w.detour, PARTIAL_DETOUR_WORK);
        }
        w
    }

    fn full(mode: Mode) -> Work {
        match mode {
            Mode::Static => Work {
                left: STATIC_TOTAL_WORK,
                per_search: STATIC_WORK_BUDGET,
                max_cells: STATIC_MAX_CELLS,
                walls_reach: f64::INFINITY,
                detour: DETOUR_TOTAL_WORK,
            },
            Mode::Routed => Work {
                left: ROUTE_TOTAL_WORK,
                per_search: ROUTE_WORK_BUDGET,
                max_cells: ROUTE_MAX_CELLS,
                walls_reach: ROUTE_WALLS_WINDOW,
                detour: ROUTE_DETOUR_WORK,
            },
        }
    }

    /// Spend `n` units; `false` (and nothing left) when they are not
    /// there.
    fn take(&mut self, n: usize) -> bool {
        if n > self.left {
            self.left = 0;
            return false;
        }
        self.left -= n;
        true
    }
}

/// No predecessor: a start state.
const NO_STATE: u32 = u32::MAX;

/// How a search treats the flow axis.
#[derive(Clone, Copy)]
struct Flow {
    dir: crate::model::Direction,
    /// +1 when the path runs downstream, -1 upstream.
    sign: f64,
    /// Price per px of stepping back; `None` forbids it (G8).
    back: Option<f64>,
}

/// What one search may do.
#[derive(Clone, Copy)]
struct Rules<'a> {
    /// The flow rule, or `None` for a sideways edge.
    flow: Option<Flow>,
    /// Frames the path should stay out of (see [`Lattice::step`]).
    mask: &'a [u64],
    /// Bands and masked frames are priced instead of blocked, and the
    /// search is greedy ([`H_WEIGHT_GREEDY`]).
    soft: bool,
}

/// Mark in `sc.live` every vertex of `lat` from which one of `goals`
/// is reachable under `rules` without stepping back along the flow: a
/// backward flood from the goals, one unit of work per vertex. The
/// forward search then never enters a dead end, so an edge with no
/// monotone way through costs a flood, not a whole pop budget.
fn mark_live(lat: &Lattice, goals: &[(usize, usize)], fl: Flow, rules: Rules, sc: &mut Scratch) {
    let (nx, ny) = (lat.xs.len(), lat.ys.len());
    sc.live.clear();
    sc.live.resize(nx * ny, false);
    sc.queue.clear();
    for &(i, j) in goals {
        if !sc.live[j * nx + i] {
            sc.live[j * nx + i] = true;
            sc.queue.push((i, j));
        }
    }
    while let Some((i, j)) = sc.queue.pop() {
        // Every vertex `p` one step `(di, dj)` before `(i, j)`.
        for (di, dj) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
            let (pi, pj) = (i as i64 - di, j as i64 - dj);
            if pi < 0 || pj < 0 || pi >= nx as i64 || pj >= ny as i64 {
                continue;
            }
            let (pi, pj) = (pi as usize, pj as usize);
            if sc.live[pj * nx + pi] {
                continue;
            }
            let (from, to) = ((lat.xs[pi], lat.ys[pj]), (lat.xs[i], lat.ys[j]));
            if fl.sign * (along(fl.dir, from) - along(fl.dir, to)) > EPS {
                continue;
            }
            if lat.step(pi, pj, di, dj, rules.mask, rules.soft).is_none() {
                continue;
            }
            sc.live[pj * nx + pi] = true;
            sc.queue.push((pi, pj));
        }
    }
}

/// Cheapest lattice path from any start stub to any goal stub, as the
/// list of lattice vertices (stubs included) with the start and goal
/// used, or `None`.
fn search(
    lat: &Lattice,
    starts: &[Term],
    goals: &[Term],
    rules: Rules,
    work: &mut Work,
    sc: &mut Scratch,
) -> Option<(Vec<Pt>, usize, usize)> {
    let Rules { flow, mask, soft } = rules;
    let (nx, ny) = (lat.xs.len(), lat.ys.len());
    if nx < 2 || ny < 2 {
        return None;
    }
    let n_states = nx * ny * 2;
    if sc.best.len() < n_states {
        sc.best.resize(n_states, f64::INFINITY);
        sc.came.resize(n_states, NO_STATE);
        sc.stamp.resize(n_states, 0);
    }
    sc.gen = sc.gen.wrapping_add(1);
    if sc.gen == 0 {
        sc.stamp.iter_mut().for_each(|s| *s = 0);
        sc.gen = 1;
    }
    let gen = sc.gen;
    let state = |i: usize, j: usize, a: usize| (j * nx + i) * 2 + a;
    let unpack = |k: usize| ((k / 2) % nx, (k / 2) / nx, k % 2);
    let start_at: Vec<Option<(usize, usize)>> = starts.iter().map(|t| lat.locate(t.stub)).collect();
    let goal_at: Vec<Option<(usize, usize)>> = goals.iter().map(|t| lat.locate(t.stub)).collect();
    // Under a strict flow rule, prune the dead ends first.
    let prune = flow.filter(|f| f.back.is_none());
    if let Some(fl) = prune {
        if !work.take((nx * ny).div_ceil(FLOOD_PER_WORK)) {
            return None;
        }
        let at: Vec<(usize, usize)> = goal_at.iter().flatten().copied().collect();
        mark_live(lat, &at, fl, rules, sc);
        if !start_at.iter().flatten().any(|&(i, j)| sc.live[j * nx + i]) {
            return None;
        }
    }
    let live = |sc: &Scratch, i: usize, j: usize| prune.is_none() || sc.live[j * nx + i];
    let hw = if soft { H_WEIGHT_GREEDY } else { H_WEIGHT };
    let budget = work.per_search;
    let h = |i: usize, j: usize| {
        goals
            .iter()
            .map(|t| (lat.xs[i] - t.stub.0).abs() + (lat.ys[j] - t.stub.1).abs() + t.cost)
            .fold(f64::INFINITY, f64::min)
            * hw
    };
    sc.heap.clear();
    for (t, at) in starts.iter().zip(&start_at) {
        let Some((i, j)) = *at else { continue };
        let s = state(i, j, axis_of(t.out));
        if sc.stamp[s] != gen || t.cost < sc.best[s] {
            sc.stamp[s] = gen;
            sc.best[s] = t.cost;
            sc.came[s] = NO_STATE;
            sc.heap.push(queued(t.cost + h(i, j), s));
        }
    }
    // (total cost, state before the goal, goal index, goal vertex).
    let mut found: Option<(f64, usize, usize, (usize, usize))> = None;
    let mut pops = 0usize;
    while let Some((f, cur)) = sc.heap.pop().map(unqueued) {
        if found.is_some_and(|b| f >= b.0) {
            break;
        }
        pops += 1;
        if pops > budget || !work.take(POP_WORK) {
            break;
        }
        let (i, j, ax) = unpack(cur);
        let g_cur = sc.best[cur];
        if f > g_cur + h(i, j) + 1e-9 {
            continue; // stale entry
        }
        let came_from_start = sc.came[cur] == NO_STATE;
        for (di, dj) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
            let (ni, nj) = (i as i64 + di, j as i64 + dj);
            if ni < 0 || nj < 0 || ni >= nx as i64 || nj >= ny as i64 {
                continue;
            }
            // The first move never walks back into the node it left.
            if came_from_start
                && starts.iter().zip(&start_at).any(|(t, at)| {
                    *at == Some((i, j)) && axis_of(t.out) == ax && (di, dj) == (-t.out.0, -t.out.1)
                })
            {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            if !live(sc, ni, nj) {
                continue;
            }
            let (from, to) = ((lat.xs[i], lat.ys[j]), (lat.xs[ni], lat.ys[nj]));
            let mut c = 0.0;
            if let Some(fl) = flow {
                let back = fl.sign * (along(fl.dir, from) - along(fl.dir, to));
                if back > EPS {
                    match fl.back {
                        Some(k) => c += k * back,
                        None => continue,
                    }
                }
            }
            let Some(extra) = lat.step(i, j, di, dj, mask, soft) else {
                continue;
            };
            let nax = axis_of((di, dj));
            c += (from.0 - to.0).abs() + (from.1 - to.1).abs() + extra;
            if nax != ax {
                c += BEND_COST;
            }
            let g_next = g_cur + c;
            let next = state(ni, nj, nax);
            if sc.stamp[next] != gen || g_next < sc.best[next] - 1e-9 {
                sc.stamp[next] = gen;
                sc.best[next] = g_next;
                sc.came[next] = cur as u32;
                sc.heap.push(queued(g_next + h(ni, nj), next));
            }
            // Arriving at a goal stub, from outside its node.
            for (gk, t) in goals.iter().enumerate() {
                if goal_at[gk] != Some((ni, nj)) || (di, dj) == t.out {
                    continue;
                }
                let bend = if nax != axis_of(t.out) {
                    BEND_COST
                } else {
                    0.0
                };
                let total = g_next + t.cost + bend;
                if found.map_or(true, |b| total < b.0 - 1e-9) {
                    found = Some((total, cur, gk, (ni, nj)));
                }
            }
        }
    }
    let (_, end, gk, last) = found?;
    let mut pts = vec![(lat.xs[last.0], lat.ys[last.1])];
    let mut at = end;
    loop {
        let (i, j, _) = unpack(at);
        pts.push((lat.xs[i], lat.ys[j]));
        let prev = sc.came[at];
        if prev == NO_STATE {
            break;
        }
        at = prev as usize;
        if pts.len() > nx * ny * 2 {
            return None; // defensive: a cycle would be a bug
        }
    }
    pts.reverse();
    let (si, sj, _) = unpack(at);
    let sk = start_at
        .iter()
        .position(|a| *a == Some((si, sj)))
        .unwrap_or(0);
    Some((pts, sk, gk))
}

/// Drop the middle of three points on one axis-aligned line when it lies
/// between the other two, and repeated points.
fn collapse(pts: Vec<Pt>) -> Vec<Pt> {
    let between = |a: f64, b: f64, c: f64| (a - b) * (c - b) <= EPS;
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_some_and(|&q| dist(p, q) < EPS) {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let same_x =
                (a.0 - b.0).abs() < EPS && (b.0 - p.0).abs() < EPS && between(a.1, b.1, p.1);
            let same_y =
                (a.1 - b.1).abs() < EPS && (b.1 - p.1).abs() < EPS && between(a.0, b.0, p.0);
            if same_x || same_y {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

/// `pts` with every point dropped that lies within [`SIMPLIFY_TOL`] of
/// the chord it would be dropped into (Ramer-Douglas-Peucker): the few
/// segments an accepted curve is priced and indexed by.
fn simplify(pts: &[Pt]) -> Vec<Pt> {
    let n = pts.len();
    if n <= 2 {
        return pts.to_vec();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((a, b)) = stack.pop() {
        let (mut far, mut k) = (0.0, 0);
        for (m, &p) in pts.iter().enumerate().take(b).skip(a + 1) {
            let d = seg_dist(p, pts[a], pts[b]);
            if d > far {
                (far, k) = (d, m);
            }
        }
        if far > SIMPLIFY_TOL {
            keep[k] = true;
            stack.push((a, k));
            stack.push((k, b));
        }
    }
    pts.iter()
        .zip(keep)
        .filter_map(|(&p, k)| k.then_some(p))
        .collect()
}

/// Waypoints the `curveBasis` painter draws as a rounded orthogonal
/// path: every interior corner gets a point `rmax` (at most half of
/// each leg) before and after it, so the spline hugs the corner.
fn smooth(corners: &[Pt], rmax: f64) -> Vec<Pt> {
    let n = corners.len();
    if n < 3 {
        return corners.to_vec();
    }
    let mut out = vec![corners[0]];
    for k in 1..n - 1 {
        let (a, c, b) = (corners[k - 1], corners[k], corners[k + 1]);
        let (l1, l2) = (dist(a, c), dist(c, b));
        if l1 < EPS || l2 < EPS {
            out.push(c);
            continue;
        }
        let r = rmax.min(l1 / 2.0).min(l2 / 2.0);
        out.push((c.0 + (a.0 - c.0) / l1 * r, c.1 + (a.1 - c.1) / l1 * r));
        out.push(c);
        out.push((c.0 + (b.0 - c.0) / l2 * r, c.1 + (b.1 - c.1) / l2 * r));
    }
    out.push(corners[n - 1]);
    collapse_exact(out)
}

/// Drop consecutive duplicate points.
fn collapse_exact(pts: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().map_or(true, |&q| dist(p, q) >= EPS) {
            out.push(p);
        }
    }
    out
}

/// The drawn polyline of `e` with its waypoints replaced by `wps`.
fn drawn_with(e: &crate::scene::SceneEdge, wps: &[Pt]) -> Vec<Pt> {
    let mut t = e.clone();
    t.waypoints = wps.to_vec();
    t.bezier = [wps[0], wps[0], wps[wps.len() - 1], wps[wps.len() - 1]];
    edge_polyline(&t)
}

/// The drawn ends (first, last point) of `e`.
fn drawn_ends(e: &crate::scene::SceneEdge) -> (Pt, Pt) {
    if e.waypoints.len() >= 2 {
        (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
    } else {
        (e.bezier[0], e.bezier[3])
    }
}

fn side_move(s: Side) -> (i64, i64) {
    let n = normal(s);
    (n.0 as i64, n.1 as i64)
}

/// The stub of a port on `side` of `rect`, `len` beyond the side (not
/// beyond the port, which on a diamond or circle lies inside the box).
fn stub_of(rect: R, side: Side, port: Pt, len: f64) -> Pt {
    match side {
        Side::Top => (port.0, rect.1 - len),
        Side::Bottom => (port.0, rect.3 + len),
        Side::Left => (rect.0 - len, port.1),
        Side::Right => (rect.2 + len, port.1),
    }
}

fn side_mid(rect: R, side: Side) -> Pt {
    let (cx, cy) = ((rect.0 + rect.2) / 2.0, (rect.1 + rect.3) / 2.0);
    match side {
        Side::Top => (cx, rect.1),
        Side::Bottom => (cx, rect.3),
        Side::Left => (rect.0, cy),
        Side::Right => (rect.2, cy),
    }
}

/// The point on `side` of `end` (rect `rect`) nearest the side's middle
/// that stands [`PORT_SPACING`] clear of every other edge's end there
/// (see [`terms`]); the middle itself on a shape without straight sides
/// or when no slot is free.
fn free_slot(
    w: &World,
    end: EndRef,
    rect: R,
    side: Side,
    taken: &[(EndRef, Pt, usize)],
    me: usize,
) -> Pt {
    let mid = side_mid(rect, side);
    let Some((_, lo, hi)) = slide_range(
        w,
        end,
        mid,
        usize::from(matches!(side, Side::Top | Side::Bottom)),
    ) else {
        return mid;
    };
    let others: Vec<f64> = taken
        .iter()
        .filter(|t| t.0 == end && t.2 != me && side_of(rect, t.1) == side)
        .map(|t| cross_along(usize::from(matches!(side, Side::Left | Side::Right)), t.1).1)
        .collect();
    let at = |t: f64| match side {
        Side::Top | Side::Bottom => (t, mid.1),
        Side::Left | Side::Right => (mid.0, t),
    };
    let c = cross_along(usize::from(matches!(side, Side::Left | Side::Right)), mid).1;
    let free = |t: f64| others.iter().all(|o| (o - t).abs() >= PORT_SPACING);
    let steps = ((hi - lo) / PORT_SPACING).ceil() as usize;
    (0..=steps)
        .flat_map(|k| [c + k as f64 * PORT_SPACING, c - k as f64 * PORT_SPACING])
        .find(|&t| t >= lo - EPS && t <= hi + EPS && free(t))
        .map_or(mid, at)
}

/// The terminals of one end: its own port first and, when `alt`, a port
/// on each of the node's other sides at [`SIDE_SWITCH_COST`]: the side's
/// midpoint, or on a straight side the free slot nearest it, at least
/// [`PORT_SPACING`] from every end in `taken` there but edge `me`'s (so
/// two reroutes never land on one port, G9).
fn terms(
    w: &World,
    end: EndRef,
    port: Pt,
    alt: bool,
    clear: f64,
    taken: &[(EndRef, Pt, usize)],
    me: usize,
) -> Vec<Term> {
    let rect = w.end_rect(end);
    let side = side_of(rect, port);
    let len = clear + STUB_EXTRA;
    let mut v = vec![Term {
        port,
        stub: stub_of(rect, side, port, len),
        out: side_move(side),
        cost: 0.0,
    }];
    if alt && matches!(end, EndRef::Node(_)) {
        for s in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
            if s == side {
                continue;
            }
            let p = free_slot(w, end, rect, s, taken, me);
            v.push(Term {
                port: p,
                stub: stub_of(rect, s, p, len),
                out: side_move(s),
                cost: SIDE_SWITCH_COST,
            });
        }
    }
    v
}

/// How one routing attempt may run.
#[derive(Clone, Copy, Debug)]
struct Try {
    /// Which lattice: 0 = [`ROUTE_CLEAR`], 1 = the wider retry.
    lat: usize,
    /// Every end may also use its node's other sides.
    alt: bool,
    /// Stepping back along the flow is priced instead of forbidden.
    soft: bool,
    /// Clearance bands and foreign frames are priced instead of
    /// blocked.
    soft_walls: bool,
    /// The search window: the stubs' bounds grown by this (px; infinite
    /// for the whole drawing).
    reach: f64,
}

/// The lattices one edge has built so far, by (clearance index,
/// reach): its tries share them. `None` records a lattice over the cap.
type Built = Vec<(usize, f64, Option<Lattice>)>;

/// What [`route_edge`] works against besides the edge itself.
struct Env<'a> {
    w: &'a World,
    /// Every edge's ends as they stand: (end, point, edge).
    taken: &'a [(EndRef, Pt, usize)],
    /// Lattices get offset tracks beside the accepted legs and a banded
    /// ring round each foreign frame (the static scene; a drag keeps its
    /// lattices to the obstacles and tags the ring with the frame, G13).
    tracks: bool,
    acc: &'a mut Accepted,
    built: &'a mut Built,
    work: &'a mut Work,
    sc: &'a mut Scratch,
}

/// Route edge `me` (drawn as `e`) under try `t`, returning its corner
/// list (port, stub, …, stub, port), or `None`. The lattice is built
/// over the try's window on first use.
fn route_edge(
    env: &mut Env,
    info: &Info,
    me: usize,
    e: &crate::scene::SceneEdge,
    t: Try,
) -> Option<Vec<Pt>> {
    let w = env.w;
    let clear = CLEARANCES[t.lat];
    let (p0, p3) = drawn_ends(e);
    let o = w.outer;
    let usable = |t: &Term| inside(o, t.stub) && w.stub_free(info, t.stub);
    let starts: Vec<Term> = terms(w, info.ends.0, p0, t.alt, clear, env.taken, me)
        .into_iter()
        .filter(usable)
        .collect();
    let goals: Vec<Term> = terms(w, info.ends.1, p3, t.alt, clear, env.taken, me)
        .into_iter()
        .filter(usable)
        .collect();
    let stubs: Vec<Pt> = starts.iter().chain(&goals).map(|t| t.stub).collect();
    if stubs.is_empty() {
        return None;
    }
    let k = match env
        .built
        .iter()
        .position(|b| b.0 == t.lat && b.1 == t.reach)
    {
        Some(k) => k,
        None => {
            let r = grow(bounds(&stubs), t.reach);
            let win = (r.0.max(o.0), r.1.max(o.1), r.2.min(o.2), r.3.min(o.3));
            let lat = Lattice::build(
                w,
                clear,
                &stubs,
                win,
                env.acc,
                me,
                &info.blocked_frames,
                env.tracks,
                env.work,
            );
            env.built.push((t.lat, t.reach, lat));
            env.built.len() - 1
        }
    };
    let lat = env.built[k].2.as_ref()?;
    // The path runs one way along the flow, the way its ends lie; a stub
    // pointing the other way would be a backtrack of its own.
    let sign = if along(w.dir, p3) >= along(w.dir, p0) {
        1.0
    } else {
        -1.0
    };
    let free_flow = info.sideways || t.soft;
    let heading = |t: &Term| sign * along(w.dir, (t.out.0 as f64, t.out.1 as f64));
    let on_lat = |t: &Term| lat.locate(t.stub).is_some();
    let starts: Vec<Term> = starts
        .into_iter()
        .filter(|t| on_lat(t) && (free_flow || heading(t) >= -EPS))
        .collect();
    let goals: Vec<Term> = goals
        .into_iter()
        .filter(|t| on_lat(t) && (free_flow || heading(t) <= EPS))
        .collect();
    if starts.is_empty() || goals.is_empty() {
        return None;
    }
    let flow = (!info.sideways).then_some(Flow {
        dir: w.dir,
        sign,
        back: t.soft.then_some(BACKTRACK_COST),
    });
    let mask = lat.mask(&info.blocked_frames);
    let rules = Rules {
        flow,
        mask: &mask,
        soft: t.soft_walls,
    };
    let (path, sk, gk) = search(lat, &starts, &goals, rules, env.work, env.sc)?;
    let mut pts = vec![starts[sk].port];
    pts.extend(path);
    pts.push(goals[gk].port);
    let pts = straighten(w, info.ends.0, env.taken, me, collapse(pts));
    let mut back: Vec<Pt> = pts.into_iter().rev().collect();
    back = straighten(w, info.ends.1, env.taken, me, back);
    back.reverse();
    Some(back)
}

/// Longest stair step (px) between a port's stub and the next lattice
/// line that [`straighten`] folds into the port leg: a jog that short
/// reads as a kink at the node, not as a turn (G8).
const PORT_JOG_MAX: f64 = 20.0;

/// `pts` (an orthogonal path from a port of `end`) with a stair step
/// under [`PORT_JOG_MAX`] right after the port's stub folded away: the
/// port slides along its straight side onto the next leg's line, when
/// that stays on the side and [`PORT_SPACING`] off the other ends there
/// (G9). Otherwise `pts` as it was.
fn straighten(
    w: &World,
    end: EndRef,
    taken: &[(EndRef, Pt, usize)],
    me: usize,
    pts: Vec<Pt>,
) -> Vec<Pt> {
    if pts.len() < 4 {
        return pts;
    }
    let (p, s, j, q) = (pts[0], pts[1], pts[2], pts[3]);
    // [`cross_along`] gives (across, along) a leg of `axis`.
    let axis = usize::from((p.0 - s.0).abs() < EPS);
    let ca = |a: Pt| cross_along(axis, a);
    let (_, along_ps) = ca((s.0 - p.0, s.1 - p.1));
    let (_, along_jq) = ca((q.0 - j.0, q.1 - j.1));
    let (cross_s, cross_j) = (ca(s).0, ca(j).0);
    let jog = (cross_j - cross_s).abs();
    let straight = (ca(q).0 - cross_j).abs() < EPS && (ca(p).0 - cross_s).abs() < EPS;
    if !(EPS..PORT_JOG_MAX).contains(&jog) || !straight || along_ps * along_jq <= 0.0 {
        return pts;
    }
    let Some((_, lo, hi)) = slide_range(w, end, p, axis) else {
        return pts;
    };
    let free = taken
        .iter()
        .filter(|t| t.0 == end && t.2 != me)
        .all(|t| (ca(t.1).0 - cross_j).abs() >= PORT_SPACING);
    if cross_j < lo - EPS || cross_j > hi + EPS || !free {
        return pts;
    }
    let mut out = vec![with_cross(axis, p, cross_j)];
    out.extend_from_slice(&pts[3..]);
    collapse(out)
}

/// One axis-aligned leg of an orthogonal path: `pts[k]`–`pts[k + 1]`
/// of edge `edge`.
#[derive(Clone, Copy, Debug)]
struct Seg {
    edge: usize,
    k: usize,
    /// 0 = horizontal (moves in y), 1 = vertical (moves in x).
    axis: usize,
    at: f64,
    lo: f64,
    hi: f64,
    /// The leg may move: an inner leg, or a port leg whose port can
    /// slide along a flat side.
    free: bool,
}

/// The leg `a`–`b` (the `k`th of edge `edge`) when it is axis-aligned
/// and not a point.
fn leg(edge: usize, k: usize, a: Pt, b: Pt) -> Option<Seg> {
    let (axis, at, lo, hi) = if (a.1 - b.1).abs() < EPS {
        (0, a.1, a.0.min(b.0), a.0.max(b.0))
    } else if (a.0 - b.0).abs() < EPS {
        (1, a.0, a.1.min(b.1), a.1.max(b.1))
    } else {
        return None;
    };
    (hi - lo > EPS).then_some(Seg {
        edge,
        k,
        axis,
        at,
        lo,
        hi,
        free: true,
    })
}

/// (across, along) coordinates of `p` for a leg of `axis`.
fn cross_along(axis: usize, p: Pt) -> (f64, f64) {
    if axis == 0 {
        (p.1, p.0)
    } else {
        (p.0, p.1)
    }
}

/// `p` with its coordinate across a leg of `axis` set to `to`.
fn with_cross(axis: usize, p: Pt, to: f64) -> Pt {
    if axis == 0 {
        (p.0, to)
    } else {
        (to, p.1)
    }
}

/// Node shapes whose sides are straight, so a port slides along one and
/// stays on the outline.
fn flat_shape(s: Shape) -> bool {
    !matches!(
        s,
        Shape::Diamond
            | Shape::Circle
            | Shape::DoubleCircle
            | Shape::StateStart
            | Shape::StateEnd
            | Shape::Stadium
    )
}

/// The side a port at `p` of `end` sits on and the tangent range it may
/// slide along, when its leg (of `axis`) leaves along the side's normal
/// and the side is straight; `None` otherwise.
fn slide_range(w: &World, end: EndRef, p: Pt, axis: usize) -> Option<(Side, f64, f64)> {
    let r = w.end_rect(end);
    if let EndRef::Node(v) = end {
        if !flat_shape(w.nodes[v].1) {
            return None;
        }
    }
    let side = side_of(r, p);
    let (moves, t0, t1) = match side {
        Side::Top | Side::Bottom => (1, r.0, r.2),
        Side::Left | Side::Right => (0, r.1, r.3),
    };
    let (lo, hi) = (t0 + PORT_INSET, t1 - PORT_INSET);
    (moves == axis && lo <= hi).then_some((side, lo, hi))
}

/// What [`spread`] keeps a group of legs off: the scene, the legs it
/// does not move and the edges drawn as free curves.
struct Fences<'a> {
    w: &'a World,
    infos: &'a [Option<Info>],
    /// Per edge: its drawn ends before the nudge.
    ends: &'a [(Pt, Pt)],
    /// Per leg axis: the free curves' segments nearly parallel to it, as
    /// (edge, across low, across high, along low, along high).
    par: [Vec<Piece>; 2],
    /// Per leg axis: [`Fences::par`] bucketed along the axis.
    par_at: [Buckets; 2],
    /// A leg sharing its channel with no other is placed too (off the
    /// borders and nodes beside it); in `route()` only shared channels
    /// are spread (G13: most groups are single legs).
    singles: bool,
    /// `route()`: a channel too narrow at the clear separations is also
    /// split at the cramped ones (its detours leave more legs in one
    /// channel than the static search does).
    routed: bool,
}

/// Indices of spans bucketed by where they lie along an axis, so a
/// group of legs looks only at the spans beside it.
struct Buckets {
    lo: f64,
    cells: Vec<Vec<u32>>,
}

impl Buckets {
    /// `spans` (along low, along high) in [`SPREAD_BUCKET`] px buckets.
    fn new(spans: impl Iterator<Item = (f64, f64)> + Clone) -> Buckets {
        let lo = spans.clone().map(|s| s.0).fold(f64::INFINITY, f64::min);
        let hi = spans.clone().map(|s| s.1).fold(f64::NEG_INFINITY, f64::max);
        if !(lo.is_finite() && hi.is_finite()) || hi - lo > ROUTE_MAX_EXTENT {
            return Buckets {
                lo: 0.0,
                cells: Vec::new(),
            };
        }
        let n = ((hi - lo) / SPREAD_BUCKET) as usize + 1;
        let mut cells = vec![Vec::new(); n];
        for (k, (a, b)) in spans.enumerate() {
            for c in cells
                .iter_mut()
                .take(Self::at(lo, n, b) + 1)
                .skip(Self::at(lo, n, a))
            {
                c.push(k as u32);
            }
        }
        Buckets { lo, cells }
    }

    fn at(lo: f64, n: usize, v: f64) -> usize {
        (((v - lo) / SPREAD_BUCKET).max(0.0) as usize).min(n.saturating_sub(1))
    }

    /// The spans in the buckets `[a, b]` meets (some more than once).
    fn meeting(&self, a: f64, b: f64) -> impl Iterator<Item = usize> + '_ {
        let n = self.cells.len();
        let (i, j) = (Self::at(self.lo, n, a), Self::at(self.lo, n, b));
        self.cells
            .iter()
            .take(if n == 0 { 0 } else { j + 1 })
            .skip(i)
            .flatten()
            .map(|&k| k as usize)
    }
}

/// A free curve's segment beside a leg axis: (edge, across low, across
/// high, along low, along high).
type Piece = (usize, f64, f64, f64, f64);

/// The segments of the free curves `curves` nearly parallel to each leg
/// axis (see [`Fences::par`]).
fn parallel_pieces(curves: &[Vec<Pt>]) -> [Vec<Piece>; 2] {
    let mut par = [Vec::new(), Vec::new()];
    for (axis, out) in par.iter_mut().enumerate() {
        for (j, c) in curves.iter().enumerate() {
            for q in c.windows(2) {
                let ((ca, la), (cb, lb)) = (cross_along(axis, q[0]), cross_along(axis, q[1]));
                let l = norm(ca - cb, la - lb);
                if l > EPS && (la - lb).abs() / l > PARALLEL_COS {
                    out.push((j, ca.min(cb), ca.max(cb), la.min(lb), la.max(lb)));
                }
            }
        }
    }
    par
}

impl Fences<'_> {
    /// The ends of edge `j` as they stand: its path's when it has one.
    fn ends_now(&self, routes: &[Option<Vec<Pt>>], j: usize) -> (Pt, Pt) {
        match &routes[j] {
            Some(p) => (p[0], p[p.len() - 1]),
            None => self.ends[j],
        }
    }

    /// Whether edge `i`'s end may slide from `old` to `p` on `side` of
    /// `end`: it passes no other end there and keeps from each at least
    /// [`PORT_SPACING`] or what it kept before, whichever is less (so the
    /// port stage, run again, finds the side as spaced as it left it).
    fn port_free(
        &self,
        routes: &[Option<Vec<Pt>>],
        i: usize,
        end: EndRef,
        side: Side,
        old: Pt,
        p: Pt,
    ) -> bool {
        let r = self.w.end_rect(end);
        let tangent = |q: Pt| match side {
            Side::Top | Side::Bottom => q.0,
            Side::Left | Side::Right => q.1,
        };
        (0..self.infos.len()).all(|j| {
            let Some(info) = self.infos[j].as_ref().filter(|_| j != i) else {
                return true;
            };
            let (a, b) = self.ends_now(routes, j);
            [(info.ends.0, a), (info.ends.1, b)].iter().all(|&(e, q)| {
                let (now, was) = (tangent(q) - tangent(p), tangent(q) - tangent(old));
                e != end
                    || side_of(r, q) != side
                    || (now * was > 0.0 && now.abs() >= PORT_SPACING.min(was.abs()) - EPS)
            })
        })
    }
}

/// Spread the legs of the orthogonal paths `routes` (corner lists,
/// ports included) that share a channel [`EDGE_SEP`] apart, and move
/// every leg off the nodes, frame borders, title chips and other legs
/// closer than their separation, inside the free slab round it
/// (libavoid-style nudging). A port leg moves by sliding its port along
/// a straight side, in order and spaced from the side's other ends (see
/// [`Fences::port_free`]); a move that would flip or shorten a neighbouring leg past use
/// is skipped. The caller keeps a moved path only when it scores no
/// worse.
fn spread(f: &Fences, routes: &mut [Option<Vec<Pt>>], work: &mut usize) {
    let w = f.w;
    let mut segs: Vec<Seg> = Vec::new();
    for (edge, r) in routes.iter().enumerate() {
        let Some(pts) = r else { continue };
        let Some(info) = f.infos[edge].as_ref() else {
            continue;
        };
        let n = pts.len();
        for k in 0..n - 1 {
            let Some(mut sg) = leg(edge, k, pts[k], pts[k + 1]) else {
                continue;
            };
            sg.free = (k != 0 || slide_range(w, info.ends.0, pts[0], sg.axis).is_some())
                && (k + 2 != n || slide_range(w, info.ends.1, pts[n - 1], sg.axis).is_some());
            segs.push(sg);
        }
    }
    // Union the free legs of different edges sharing a channel.
    let n = segs.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    let close = EDGE_SEP + SEP_AIR;
    for a in 0..n {
        for b in a + 1..n {
            let (s, t) = (segs[a], segs[b]);
            if s.free
                && t.free
                && s.edge != t.edge
                && s.axis == t.axis
                && (s.at - t.at).abs() < close - EPS
                && s.hi.min(t.hi) - s.lo.max(t.lo) > EPS
            {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n];
    for a in 0..n {
        if segs[a].free {
            let r = find(&mut parent, a);
            groups[r].push(a);
        }
    }
    // What placing one group looks at: the scene's obstacles, every leg
    // and every free curve's segment.
    let scene = w.nodes.len() + w.frames.len() + w.chips.len();
    for grp in groups
        .iter()
        .filter(|g| g.len() > 1 || g.len() == 1 && f.singles)
    {
        let (axis, lo, hi) = grp.iter().fold(
            (segs[grp[0]].axis, f64::INFINITY, f64::NEG_INFINITY),
            |a, &s| (a.0, a.1.min(segs[s].lo), a.2.max(segs[s].hi)),
        );
        let per_group = scene + n + f.par_at[axis].meeting(lo, hi).count();
        let Some(left) = work.checked_sub(per_group.div_ceil(SPREAD_LOOKS_PER_UNIT)) else {
            return; // the other groups stay where they are (G13)
        };
        *work = left;
        place_group(f, routes, &segs, grp);
    }
}

/// Place one group of legs sharing a channel (see [`spread`]).
fn place_group(f: &Fences, routes: &mut [Option<Vec<Pt>>], segs: &[Seg], grp: &[usize]) {
    let w = f.w;
    let axis = segs[grp[0]].axis;
    let lo = grp
        .iter()
        .map(|&s| segs[s].lo)
        .fold(f64::INFINITY, f64::min);
    let hi = grp
        .iter()
        .map(|&s| segs[s].hi)
        .fold(f64::NEG_INFINITY, f64::max);
    let a_min = grp
        .iter()
        .map(|&s| segs[s].at)
        .fold(f64::INFINITY, f64::min);
    let a_max = grp
        .iter()
        .map(|&s| segs[s].at)
        .fold(f64::NEG_INFINITY, f64::max);
    let mean = grp.iter().map(|&s| segs[s].at).sum::<f64>() / grp.len() as f64;
    // The ends a member's port leg leaves from: their own box is no
    // fence to it.
    let mut own: Vec<EndRef> = Vec::new();
    for &s in grp {
        let sg = segs[s];
        let pts = routes[sg.edge].as_ref().expect("routed");
        let info = f.infos[sg.edge].as_ref().expect("routed");
        if sg.k == 0 {
            own.push(info.ends.0);
        }
        if sg.k + 2 == pts.len() {
            own.push(info.ends.1);
        }
    }
    // The free slab across the channel over [lo, hi]: `hard` from the
    // scene (nodes, frames, chips), `soft` from the other legs and
    // curves. Slab bounds as (low, high).
    let open = (f64::NEG_INFINITY, f64::INFINITY);
    let mut soft = open;
    let both = |x: (f64, f64), y: (f64, f64)| (x.0.max(y.0), x.1.min(y.1));
    // An obstacle across [c_lo, c_hi] over [s_lo, s_hi] along bounds
    // the slab round `at` (the members' lowest, highest and mean line)
    // on its side.
    let fence_at =
        |slab: &mut (f64, f64), at: (f64, f64, f64), c_lo: f64, c_hi: f64, s_lo: f64, s_hi: f64| {
            if s_hi <= lo + EPS || s_lo >= hi - EPS {
                return;
            }
            if c_hi <= at.0 + EPS || (c_lo < at.1 - EPS && (c_lo + c_hi) / 2.0 <= at.2) {
                slab.0 = slab.0.max(c_hi);
            } else {
                slab.1 = slab.1.min(c_lo);
            }
        };
    let fence = |slab: &mut (f64, f64), c_lo: f64, c_hi: f64, s_lo: f64, s_hi: f64| {
        fence_at(slab, (a_min, a_max, mean), c_lo, c_hi, s_lo, s_hi)
    };
    let rect = |r: R| {
        if axis == 0 {
            (r.1, r.3, r.0, r.2)
        } else {
            (r.0, r.2, r.1, r.3)
        }
    };
    // The scene's slab round `at` keeping (node, frame, chip)
    // clearances `c`, off both borders of a frame round the channel.
    let scene_slab_at = |c: (f64, f64, f64), at: (f64, f64, f64)| {
        let mut slab = open;
        for (v, &(r, _)) in w.nodes.iter().enumerate() {
            if own.contains(&EndRef::Node(v)) {
                continue;
            }
            let (a, b, lo, hi) = rect(grow(r, c.0));
            fence_at(&mut slab, at, a, b, lo, hi);
        }
        for (k, &r) in w.frames.iter().enumerate() {
            if own.contains(&EndRef::Frame(k)) {
                continue;
            }
            let (a, b, s_lo, s_hi) = rect(r);
            if at.2 > a && at.2 < b && hi > s_lo && lo < s_hi {
                // Inside: off both its borders.
                fence_at(&mut slab, at, a - c.1, a + c.1, s_lo, s_hi);
                fence_at(&mut slab, at, b - c.1, b + c.1, s_lo, s_hi);
            } else {
                fence_at(&mut slab, at, a - c.1, b + c.1, s_lo - c.1, s_hi + c.1);
            }
        }
        for &r in &w.chips {
            let (a, b, s_lo, s_hi) = rect(grow(r, c.2));
            fence_at(&mut slab, at, a, b, s_lo, s_hi);
        }
        slab
    };
    let scene_slab = |c: (f64, f64, f64)| scene_slab_at(c, (a_min, a_max, mean));
    let clear = (NODE_SEP + SEP_AIR, FRAME_SEP + SEP_AIR, CHIP_SEP + SEP_AIR);
    let hard = scene_slab(clear);
    let gap = EDGE_SEP + SEP_AIR;
    for (k, sg) in segs.iter().enumerate() {
        if sg.axis == axis && !grp.contains(&k) {
            fence(&mut soft, sg.at - gap, sg.at + gap, sg.lo, sg.hi);
        }
    }
    let members: Vec<usize> = grp.iter().map(|&s| segs[s].edge).collect();
    for k in f.par_at[axis].meeting(lo, hi) {
        let (j, c0, c1, l0, l1) = f.par[axis][k];
        if !members.contains(&j) {
            fence(&mut soft, c0 - gap, c1 + gap, l0, l1);
        }
    }
    // What the members' own port legs allow: a leg next to a port leg
    // keeps it as long as [`shifted`] asks, and a port leg stays on its
    // side's range.
    let mut stub = (f64::NEG_INFINITY, f64::INFINITY);
    for &s in grp {
        let sg = segs[s];
        let pts = routes[sg.edge].as_ref().expect("routed");
        let info = f.infos[sg.edge].as_ref().expect("routed");
        let n = pts.len();
        for port in [(sg.k == 1).then_some(0), (sg.k + 3 == n).then_some(n - 1)]
            .into_iter()
            .flatten()
        {
            let pc = cross_along(axis, pts[port]).0;
            let keep = (sg.at - pc).abs().min(PORT_ZONE);
            if pc < sg.at {
                stub.0 = stub.0.max(pc + keep);
            } else {
                stub.1 = stub.1.min(pc - keep);
            }
        }
        for (end, port) in [(info.ends.0, 0), (info.ends.1, n - 1)] {
            if sg.k == port.min(n - 2) {
                if let Some((_, a, b)) = slide_range(w, end, pts[port], axis) {
                    stub = (stub.0.max(a), stub.1.min(b));
                }
            }
        }
    }
    // Order the members so their turns do not cross (libavoid): at each
    // end a leg turns towards its neighbour; of two legs turning the
    // same way, the one turning further out lies away from the turn.
    let span = hi - lo + 1.0;
    let key = |sg: &Seg| -> f64 {
        let pts = routes[sg.edge].as_ref().expect("routed");
        let n = pts.len();
        let (a, b) = (pts[sg.k], pts[sg.k + 1]);
        let before = (sg.k > 0).then(|| pts[sg.k - 1]);
        let after = (sg.k + 2 < n).then(|| pts[sg.k + 2]);
        // (along, turn sign) at the leg's two ends.
        let turn = |p: Pt, q: Option<Pt>| {
            let (c, l) = cross_along(axis, p);
            let s = q.map_or(0.0, |q| {
                let d = cross_along(axis, q).0 - c;
                if d.abs() < EPS {
                    0.0
                } else {
                    d.signum()
                }
            });
            (l, s)
        };
        let (ea, eb) = (turn(a, before), turn(b, after));
        let ((l0, s0), (l1, s1)) = if ea.0 <= eb.0 { (ea, eb) } else { (eb, ea) };
        s0 * (span + l0 - lo) + s1 * (span + hi - l1)
    };
    let mut order: Vec<usize> = grp.to_vec();
    order.sort_by(|&x, &y| {
        key(&segs[x])
            .total_cmp(&key(&segs[y]))
            .then(segs[x].at.total_cmp(&segs[y].at))
            .then(segs[x].edge.cmp(&segs[y].edge))
            .then(segs[x].k.cmp(&segs[y].k))
    });
    let m = order.len();
    // The first slab the members fit in, with the least spacing it may
    // squeeze them to: the channel at [`EDGE_SEP_MIN`], then the scene
    // alone (over the other legs, which move in their own groups) at
    // that, then at just over [`EDGE_SPACING`], still two strokes rather
    // than one on top of the other (G1 over G4); then the channel split
    // at a frame beside it ([`split_channel`]), and last the scene kept
    // only at the cramped gap. No tier puts a leg nearer a frame border
    // than a ride along it ([`BORDER_HUG_GAP`], G4). A group that fits
    // none stays.
    let channel = both(both(hard, soft), stub);
    let scene = both(hard, stub);
    let least = EDGE_SPACING + SEP_AIR;
    let fits = |x: (f64, f64), step: f64| x.1 - x.0 >= (m - 1) as f64 * step - EPS;
    let tiers = [
        (channel, EDGE_SEP_MIN),
        (scene, EDGE_SEP_MIN),
        (scene, least),
    ];
    // Each part of the group (members in `order`'s order), the slab it
    // goes in and the least spacing it may squeeze to.
    let mut plan: Vec<(Vec<usize>, (f64, f64), f64)> = Vec::new();
    // The split and the cramped slabs are built only when the clear ones
    // are too narrow (most groups fit the first tier).
    let split = |c: (f64, f64, f64)| {
        split_channel(SplitIn {
            f,
            segs,
            order: &order,
            own: &own,
            axis,
            channel: both(scene_slab(c), stub),
            frame_sep: c.1,
            inner: &|at| both(scene_slab_at(c, at), stub),
        })
    };
    let cramped = (
        CRAMP_NODE_GAP + SEP_AIR,
        BORDER_HUG_GAP + SEP_AIR,
        NODE_CLEARANCE + SEP_AIR,
    );
    if let Some((slab, step)) = tiers.into_iter().find(|t| fits(t.0, t.1)) {
        plan.push((order.clone(), slab, step));
    } else if let Some(parts) = split(clear).or_else(|| f.routed.then(|| split(cramped)).flatten())
    {
        plan = parts
            .into_iter()
            .map(|(p, slab)| (p, slab, EDGE_SEP_MIN))
            .collect();
    } else {
        let squeezed = both(scene_slab(cramped), stub);
        if fits(squeezed, least) {
            plan.push((order.clone(), squeezed, least));
        }
    }
    for (part, (fl, fh), min_step) in plan {
        let m = part.len();
        let room = fh - fl;
        let mut step = EDGE_SEP + SEP_AIR;
        if m > 1 && room < step * (m - 1) as f64 {
            step = (room / (m - 1) as f64).max(min_step);
        }
        let want: Vec<f64> = part.iter().map(|&s| segs[s].at).collect();
        let to = if m == 1 {
            vec![want[0].max(fl).min(fh)]
        } else {
            crate::ports::place(&want, step, fl, fh)
        };
        for (r, &s) in part.iter().enumerate() {
            let sg = segs[s];
            if let Some(m) = shifted(f, routes, sg, to[r], None) {
                routes[sg.edge] = Some(m);
            }
        }
    }
}

/// Some members of a group of legs (indices into the stage's legs, low
/// to high) and the slab across the channel they are placed in.
type Part = (Vec<usize>, (f64, f64));

/// What [`split_channel`] looks at.
struct SplitIn<'a> {
    f: &'a Fences<'a>,
    segs: &'a [Seg],
    /// The group's legs, low to high across the channel.
    order: &'a [usize],
    /// The ends the members' port legs leave from.
    own: &'a [EndRef],
    axis: usize,
    /// The channel's slab, clear of the scene (and of the members'
    /// port ranges).
    channel: (f64, f64),
    /// What a leg keeps from a frame border.
    frame_sep: f64,
    /// The free slab round a line (lowest, highest, mean) inside a
    /// frame: off its borders, nodes, chips, other legs and curves.
    inner: &'a dyn Fn((f64, f64, f64)) -> (f64, f64),
}

/// A channel too narrow for its legs at [`EDGE_SEP_MIN`] next to a frame
/// some of them may enter (an ancestor of one of their ends): the
/// fewest members on that frame's side of the group run inside it, a
/// frame separation in from its border, and the others stay in the
/// channel (G4: two clear strokes each side of the border instead of
/// legs squeezed onto it). The parts as (members, slab), or `None` when
/// no frame beside the channel holds them.
fn split_channel(x: SplitIn) -> Option<Vec<Part>> {
    let SplitIn {
        f,
        segs,
        order,
        own,
        axis,
        channel,
        frame_sep,
        inner,
    } = x;
    let m = order.len();
    let fits = |x: (f64, f64), n: usize| n == 0 || x.1 - x.0 >= (n - 1) as f64 * EDGE_SEP_MIN - EPS;
    // Whether member `s` may cross border `line` of frame `k` into it
    // (leaving a frame is always allowed).
    let may_cross = |s: usize, k: usize, into: bool, span: (f64, f64)| {
        !into
            || segs[s].lo >= span.0 - EPS
                && segs[s].hi <= span.1 + EPS
                && f.infos[segs[s].edge]
                    .as_ref()
                    .is_some_and(|i| i.allowed.contains(&k))
    };
    for (k, &r) in f.w.frames.iter().enumerate() {
        if own.contains(&EndRef::Frame(k)) {
            continue;
        }
        let (a, b, s_lo, s_hi) = if axis == 0 {
            (r.1, r.3, r.0, r.2)
        } else {
            (r.0, r.2, r.1, r.3)
        };
        // (border, the channel's high side, crossing into the frame).
        let lines = [
            (a, true, true),
            (b, false, true),
            (a, false, false),
            (b, true, false),
        ];
        for (line, high, into) in lines {
            let bound = if high {
                channel.1 - (line - frame_sep)
            } else {
                channel.0 - (line + frame_sep)
            };
            if bound.abs() >= LINE_EPS {
                continue;
            }
            // Just past the border on its other side, past its
            // separation.
            let at = if high {
                line + frame_sep + LINE_EPS
            } else {
                line - frame_sep - LINE_EPS
            };
            let slab = inner((at, at, at));
            if !(slab.0 <= at && at <= slab.1) {
                continue;
            }
            for j in 1..=m {
                let (stay, go) = if high {
                    (order[..m - j].to_vec(), order[m - j..].to_vec())
                } else {
                    (order[j..].to_vec(), order[..j].to_vec())
                };
                if !go.iter().all(|&s| may_cross(s, k, into, (s_lo, s_hi))) {
                    break;
                }
                if fits(channel, stay.len()) && fits(slab, go.len()) {
                    let mut parts = vec![(go, slab)];
                    if !stay.is_empty() {
                        parts.push((stay, channel));
                    }
                    return Some(parts);
                }
            }
        }
    }
    None
}

/// Leg `sg`'s path with the leg moved to `to` across, with the
/// neighbouring corners; a port leg slides its port. `None` when a
/// neighbouring leg would flip (a backtrack) or a port leg would get
/// shorter than both its old length and [`PORT_ZONE`], or when the port
/// leaves its side's range or crowds another end there
/// ([`Fences::port_free`]). Leg `vanish` (a stair step being merged)
/// may shrink to nothing.
fn shifted(
    f: &Fences,
    routes: &[Option<Vec<Pt>>],
    sg: Seg,
    to: f64,
    vanish: Option<usize>,
) -> Option<Vec<Pt>> {
    if (to - sg.at).abs() < EPS || !to.is_finite() {
        return None;
    }
    let pts = routes[sg.edge].as_ref().expect("routed");
    let info = f.infos[sg.edge].as_ref().expect("routed");
    let n = pts.len();
    let mut moved = pts.clone();
    for q in [sg.k, sg.k + 1] {
        moved[q] = with_cross(sg.axis, moved[q], to);
    }
    let last = n - 1;
    let ok = |q: usize| {
        let (a, b) = (moved[q], moved[q + 1]);
        let (oa, ob) = (pts[q], pts[q + 1]);
        if vanish == Some(q) {
            return dist(a, b) < EPS;
        }
        let keep = (b.0 - a.0) * (ob.0 - oa.0) + (b.1 - a.1) * (ob.1 - oa.1) > 0.0;
        let min = if q == 0 || q + 1 == last {
            dist(oa, ob).min(PORT_ZONE)
        } else {
            EPS
        };
        keep && dist(a, b) >= min - EPS
    };
    if (sg.k > 0 && !ok(sg.k - 1)) || (sg.k + 2 < n && !ok(sg.k + 1)) {
        return None;
    }
    for (src, idx, end) in [(true, 0, info.ends.0), (false, last, info.ends.1)] {
        let touches = if src { sg.k == 0 } else { sg.k + 1 == last };
        if !touches {
            continue;
        }
        let (side, lo, hi) = slide_range(f.w, end, pts[idx], sg.axis)?;
        if to < lo - EPS
            || to > hi + EPS
            || !f.port_free(routes, sg.edge, end, side, pts[idx], moved[idx])
        {
            return None;
        }
    }
    Some(moved)
}

/// `pts` with its first stair step (two opposite corners closer than
/// [`JOG_MIN`]) merged into one leg, trying the shorter neighbouring leg
/// first: it moves onto the other's line (a port leg slides its port).
/// `None` when there is no step to merge or no move is allowed.
fn merge_jog(f: &Fences, routes: &[Option<Vec<Pt>>], i: usize, skip: usize) -> Option<Vec<Pt>> {
    let pts = routes[i].as_ref()?;
    let n = pts.len();
    let mut seen = 0;
    for k in 1..n.saturating_sub(2) {
        let (a, b, c, d) = (pts[k - 1], pts[k], pts[k + 1], pts[k + 2]);
        if dist(b, c) >= JOG_MIN {
            continue;
        }
        let (Some(l0), Some(l2)) = (leg(i, k - 1, a, b), leg(i, k + 1, c, d)) else {
            continue;
        };
        // The same way on both sides of the jog: a stair step.
        let same = (b.0 - a.0) * (d.0 - c.0) + (b.1 - a.1) * (d.1 - c.1) > 0.0;
        if l0.axis != l2.axis || !same {
            continue;
        }
        if seen < skip {
            seen += 1;
            continue;
        }
        let mut tries = [(l0, l2.at), (l2, l0.at)];
        if l2.hi - l2.lo < l0.hi - l0.lo {
            tries.swap(0, 1);
        }
        return tries
            .into_iter()
            .find_map(|(sg, to)| shifted(f, routes, sg, to, Some(k)))
            .map(collapse);
    }
    None
}

/// The corner list of an edge drawn as an orthogonal path (every leg of
/// its waypoints axis-aligned, its ports' included), else `None`.
fn ortho_corners(e: &crate::scene::SceneEdge) -> Option<Vec<Pt>> {
    if e.waypoints.len() < 2 {
        return None;
    }
    let c = collapse(e.waypoints.clone());
    let aligned = c
        .windows(2)
        .all(|q| (q[0].0 - q[1].0).abs() < EPS || (q[0].1 - q[1].1).abs() < EPS);
    (aligned && c.len() >= 2).then_some(c)
}

/// Stages 4 and 5 of [`crate::edgepass::finish`], after the ports were
/// spaced again: the channel nudge over every orthogonal path (rerouted or
/// drawn so by the builder), ports included ([`spread`], G1/G4); then
/// each path's stair steps under [`JOG_MIN`] merged and its corners
/// rounded up to [`CORNER_MAX`] where the room allows (one soft
/// orthogonal style). Every change of an edge is kept only when it
/// scores no worse against the scene and the other edges as drawn by
/// then. Sets `moved[i]` for each edge it redraws.
pub(crate) fn nudge(g: &Graph, sc: &mut Scene, cx: &Ctx, mode: Mode, moved: &mut [bool]) {
    let ne = sc.edges.len();
    if ne == 0 || ne > ROUTE_MAX_EDGES {
        return;
    }
    let mut w = World::new(g, sc);
    if !w.sane() {
        return;
    }
    if mode == Mode::Routed {
        w.chip_reach = NODE_CLEARANCE;
    }
    let infos: Vec<Option<Info>> = (0..ne).map(|i| info_of(g, &w, cx, i)).collect();
    // As drawn before this stage, to find the labels its moves cross.
    let was: Vec<(Vec<Pt>, [Pt; 4])> = sc
        .edges
        .iter()
        .map(|e| (e.waypoints.clone(), e.bezier))
        .collect();
    // A fixed edge ([`Ctx::fixed`]) keeps its path: like a free curve,
    // it only fences the legs that may move.
    let mut routes: Vec<Option<Vec<Pt>>> = (0..ne)
        .map(|i| {
            infos[i].as_ref()?;
            if cx.fixed.get(i).copied().unwrap_or(false) {
                return None;
            }
            ortho_corners(&sc.edges[i])
        })
        .collect();
    if routes.iter().all(Option::is_none) {
        reorder(g, sc, cx, mode, moved, &w, &infos, None);
        mark_crossed_labels(&w, sc, &was, moved);
        return;
    }
    let polys: Vec<Vec<Pt>> = sc.edges.iter().map(edge_polyline).collect();
    let curves: Vec<Vec<Pt>> = (0..ne)
        .map(|i| {
            if routes[i].is_none() && infos[i].is_some() {
                simplify(&polys[i])
            } else {
                Vec::new()
            }
        })
        .collect();
    let ends: Vec<(Pt, Pt)> = sc.edges.iter().map(drawn_ends).collect();
    let par = parallel_pieces(&curves);
    let par_at = [0, 1].map(|a| Buckets::new(par[a].iter().map(|p| (p.3, p.4))));
    let f = Fences {
        w: &w,
        infos: &infos,
        ends: &ends,
        par,
        par_at,
        singles: mode == Mode::Static,
        routed: mode == Mode::Routed,
    };
    let before = routes.clone();
    // In `route()` the spread is charged to [`SPREAD_ROUTE_WORK`] and
    // the scoring below to [`NUDGE_ROUTE_WORK`]: once either is spent,
    // the other groups or paths stay as the router left them (G13).
    let (mut spread_work, mut work) = match mode {
        Mode::Static => (usize::MAX, usize::MAX),
        Mode::Routed => (SPREAD_ROUTE_WORK, NUDGE_ROUTE_WORK),
    };
    for _ in 0..NUDGE_ROUNDS {
        spread(&f, &mut routes, &mut spread_work);
    }
    // Each edge is judged against where the others were nudged to (and
    // put back where it is not kept).
    let nudged: Vec<Option<Vec<Pt>>> = (0..ne)
        .map(|i| {
            let new = routes[i].as_ref()?;
            (Some(new) != before[i].as_ref())
                .then(|| drawn_with(&sc.edges[i], &smooth(new, CORNER_R)))
        })
        .collect();
    // The index is built on first use (a scene whose paths neither
    // moved nor have a step to merge needs none); until then `held`
    // records what an edge settled on.
    let mut acc: Option<Accepted> = None;
    let mut held: Vec<Option<Vec<Pt>>> = vec![None; ne];
    let index = |held: &[Option<Vec<Pt>>]| {
        let mut a = Accepted::new(ne, w.outer);
        for (i, info) in infos.iter().enumerate() {
            if let Some(info) = info {
                let pts = held[i].as_ref().or(nudged[i].as_ref()).unwrap_or(&polys[i]);
                accept(&mut a, i, info.ends, pts);
            }
        }
        a
    };
    // Per edge: the nudged path if no worse, then its stair steps, then
    // the wider rounding, each kept only if no worse (charged to the
    // same work).
    let mut kept = before.clone();
    for i in 0..ne {
        let (Some(info), Some(old)) = (infos[i].as_ref(), before[i].as_ref()) else {
            continue;
        };
        let e = &sc.edges[i];
        // About five scorings of the path's points.
        let Some(left) = work.checked_sub(polys[i].len() * NUDGE_SCORES * SCORE_POINT_WORK) else {
            // Out of work: the edge stays as drawn, and the index, which
            // held its nudged path for the edges judged before it, gets
            // the drawn one back for those judged after.
            if nudged[i].is_some() {
                match acc.as_mut() {
                    Some(a) => accept(a, i, info.ends, &polys[i]),
                    None => held[i] = Some(polys[i].clone()),
                }
            }
            continue;
        };
        work = left;
        let mut cur = old.clone();
        let mut pts = polys[i].clone();
        let mut redraw = false;
        // The score of `pts` when it is known.
        let mut known: Option<Score> = None;
        // Scored against the others only when the path may change.
        let nudged_i = routes[i].as_ref().zip(nudged[i].clone());
        if nudged_i.is_some() || merge_jog(&f, &kept, i, 0).is_some() {
            let acc = acc.get_or_insert_with(|| index(&held));
            let mut s_cur = w.score(info, i, &pts, Some(&mut *acc));
            if let Some((new, p)) = nudged_i {
                let s = w.score(info, i, &p, Some(&mut *acc));
                if s <= s_cur {
                    (cur, pts, s_cur, redraw) = (new.clone(), p, s, true);
                }
            }
            kept[i] = Some(cur.clone());
            // Stair steps, one at a time; a step that cannot merge is
            // passed over.
            let mut skip = 0;
            for _ in 0..cur.len() {
                let Some(c) = merge_jog(&f, &kept, i, skip) else {
                    break;
                };
                let p = drawn_with(e, &smooth(&c, CORNER_R));
                let s = w.score(info, i, &p, Some(&mut *acc));
                if s <= s_cur {
                    (cur, pts, s_cur, redraw) = (c, p, s, true);
                    kept[i] = Some(cur.clone());
                } else {
                    skip += 1;
                }
            }
            known = Some(s_cur);
        }
        // The wider rounding only cuts its corners: kept when the scene
        // scores it no worse. A path with no inner corner has nothing to
        // round: a straight builder edge keeps its waypoints, and its
        // label its spot.
        // In `route()` only a path this stage redraws anyway: the others
        // keep the router's corners, unjudged (G13).
        let wide = (cur.len() > 2 && (redraw || mode == Mode::Static))
            .then(|| {
                let narrow = drawn_with(e, &smooth(&cur, CORNER_R));
                let wide = drawn_with(e, &smooth(&cur, CORNER_MAX));
                // Legs too short for a wider corner: nothing to score.
                if wide == narrow {
                    return None;
                }
                let s_narrow = match known {
                    Some(s) if pts == narrow => s.scene(),
                    _ => w.score(info, i, &narrow, None),
                };
                (w.score(info, i, &wide, None) <= s_narrow).then_some(wide)
            })
            .flatten();
        let r = if let Some(wide) = wide {
            (pts, redraw) = (wide, true);
            CORNER_MAX
        } else {
            CORNER_R
        };
        match acc.as_mut() {
            Some(a) => accept(a, i, info.ends, &pts),
            None => held[i] = Some(pts),
        }
        if redraw {
            let wps = smooth(&cur, r);
            let e = &mut sc.edges[i];
            if e.waypoints != wps {
                let (p0, p3) = (wps[0], wps[wps.len() - 1]);
                e.waypoints = wps;
                e.bezier = [p0, p0, p3, p3];
                moved[i] = true;
            }
        }
    }
    reorder(g, sc, cx, mode, moved, &w, &infos, acc);
    mark_crossed_labels(&w, sc, &was, moved);
}

/// Flag in `moved` every labelled edge whose label a path redrawn since
/// `was` (each edge's waypoints and curve then) now runs through, so the
/// label stage re-places it off the stroke and off the moved arrowhead
/// (G11), not only the redrawn edges' own labels.
fn mark_crossed_labels(w: &World, sc: &Scene, was: &[(Vec<Pt>, [Pt; 4])], moved: &mut [bool]) {
    let lines: Vec<Vec<Pt>> = sc
        .edges
        .iter()
        .zip(was)
        .filter(|(e, (wps, bez))| e.waypoints != *wps || e.bezier != *bez)
        .map(|(e, _)| edge_polyline(e))
        .collect();
    for &(j, r) in &w.labels {
        if moved.get(j).copied().unwrap_or(true) {
            continue;
        }
        let r = grow(r, LABEL_CLEAR);
        let hit = lines.iter().any(|pts| {
            overlaps(bounds(pts), r) && pts.windows(2).any(|q| clip(q[0], q[1], r).is_some())
        });
        if hit {
            moved[j] = true;
        }
    }
}

/// The end of [`nudge`] in the static scene: [`crate::ports::reorder`]
/// (G9, end crossings)
/// in `w`, each side's re-placed ends kept only when the side's moved
/// edges, scored against the scene and every other edge as drawn
/// (`acc` when the caller has them indexed), cross fewer edges without
/// hitting more nodes, frames or chips, wiggling more, running under
/// more labels, or running on top of or cramped beside more edges.
#[allow(clippy::too_many_arguments)]
fn reorder(
    g: &Graph,
    sc: &mut Scene,
    cx: &Ctx,
    mode: Mode,
    moved: &mut [bool],
    w: &World,
    infos: &[Option<Info>],
    mut acc: Option<Accepted>,
) {
    // Static only: in `route()` the builder's curves keep the order
    // [`crate::ports::assign`] gave their ends, and judging the rerouted
    // sides cost a drag frame over a millisecond (G13).
    if mode == Mode::Routed {
        return;
    }
    let ne = sc.edges.len();
    // The other edges as drawn, built on the first side that needs it
    // (with that side's edges as they were) and kept current.
    let mut judge = |sc: &Scene, was: &[(usize, crate::scene::SceneEdge)]| -> bool {
        let old_of = |i: usize| was.iter().find(|x| x.0 == i).map(|x| &x.1);
        let acc = acc.get_or_insert_with(|| {
            let mut a = Accepted::new(ne, w.outer);
            for (i, info) in infos.iter().enumerate() {
                if let Some(info) = info {
                    let e = old_of(i).unwrap_or(&sc.edges[i]);
                    accept(&mut a, i, info.ends, &edge_polyline(e));
                }
            }
            a
        });
        // Summed over the moved edges: against the other edges first
        // (where a better order shows), the scene only when that passes.
        let total = |acc: Option<&mut Accepted>, of: &dyn Fn(usize) -> Vec<Pt>| {
            let mut acc = acc;
            let mut t = Score::default();
            for &(i, _) in was {
                let Some(info) = infos[i].as_ref() else {
                    continue;
                };
                let pts = of(i);
                let s = match acc.as_mut() {
                    // The drawn polyline itself: a reorder's gain or loss
                    // is a few px, under what simplifying may shift.
                    Some(a) => w.against(info, i, &pts, a, Score::default(), true),
                    None => w.score(info, i, &pts, None),
                };
                t = Score {
                    node: t.node + s.node,
                    frame: t.frame + s.frame,
                    chip: t.chip + s.chip,
                    wiggle: t.wiggle + s.wiggle,
                    head: t.head + s.head,
                    overlap: t.overlap + s.overlap,
                    near: t.near + s.near,
                    cross: t.cross + s.cross,
                    tight: t.tight + s.tight,
                    label: t.label + s.label,
                    hug: t.hug + s.hug,
                };
            }
            t
        };
        let old_pts = |i: usize| edge_polyline(old_of(i).expect("side edge"));
        let new_pts = |i: usize| edge_polyline(&sc.edges[i]);
        let old = total(Some(&mut *acc), &old_pts);
        let put = |acc: &mut Accepted, of: &dyn Fn(usize) -> Vec<Pt>| {
            for &(i, _) in was {
                if let Some(info) = infos[i].as_ref() {
                    accept(acc, i, info.ends, &of(i));
                }
            }
        };
        put(acc, &new_pts);
        let new = total(Some(&mut *acc), &new_pts);
        let keep =
            new.cross < old.cross && new.overlap <= old.overlap && new.tight <= old.tight && {
                let (old, new) = (total(None, &old_pts), total(None, &new_pts));
                new.node <= old.node
                    && new.frame <= old.frame
                    && new.chip <= old.chip
                    && new.wiggle <= old.wiggle
                    && new.label <= old.label
            };
        if !keep {
            put(acc, &old_pts);
        }
        keep
    };
    crate::ports::reorder(g, sc, cx, mode, moved, &mut judge);
}

/// Who edge `i` is, what it may enter, and which way it runs; `None`
/// for an edge the router leaves alone.
fn info_of(g: &Graph, w: &World, cx: &Ctx, i: usize) -> Option<Info> {
    let (a, b) = cx.ends.get(i).copied().flatten()?;
    if !cx.drawn.get(i).copied().unwrap_or(false) || a == b {
        return None;
    }
    let (fa, fb) = (frames_of(cx, a), frames_of(cx, b));
    // An edge from a frame to something inside it has no outside side
    // to leave by: left alone.
    for (end, other) in [(a, fb), (b, fa)] {
        if let EndRef::Frame(f) = end {
            if other.contains(&f) {
                return None;
            }
        }
    }
    let mut allowed: Vec<usize> = fa.iter().chain(fb).copied().collect();
    allowed.sort_unstable();
    allowed.dedup();
    let mut blocked_frames: Vec<usize> = (0..w.frames.len())
        .filter(|f| !allowed.contains(f))
        .collect();
    for end in [a, b] {
        if let EndRef::Frame(f) = end {
            blocked_frames.push(f);
        }
    }
    blocked_frames.sort_unstable();
    blocked_frames.dedup();
    let span = |r: R| {
        let (p, q) = (
            along(g.direction, (r.0, r.1)),
            along(g.direction, (r.2, r.3)),
        );
        (p.min(q), p.max(q))
    };
    let ((a0, a1), (b0, b1)) = (span(w.end_rect(a)), span(w.end_rect(b)));
    Some(Info {
        ends: (a, b),
        allowed,
        blocked_frames,
        back: cx.back.get(i).copied().unwrap_or(false),
        sideways: a1.min(b1) > a0.max(b0),
    })
}

/// How a candidate squeezes through the scene, for [`trades`] (px).
#[derive(Clone, Copy)]
struct Squeeze {
    /// The candidate's run through a narrow channel ([`narrow_px`]).
    narrow: f64,
    /// The candidate's ride along frame borders ([`border_px`]).
    border: f64,
    /// The original's ride along frame borders.
    base_border: f64,
}

/// The narrow-channel rule (G4 over G6): an original whose only faults
/// are frames or title chips it crosses between its nodes (on top of no
/// other edge) keeps its free curve over a candidate that threads a
/// channel narrower than two frame separations for more than
/// [`TIGHT_RUN_MIN`] while riding frame borders and running cramped
/// beside other edges for longer, together, than the original: such a
/// channel is not the clear one G6 asks for. An
/// original crossing only a frame also keeps its curve over a candidate
/// that rides a border or node side for [`HUG_TRADE`] more, or runs
/// cramped beside other edges for [`TIGHT_RUN_MIN`] more.
fn trades(base: &Score, s: &Score, squeeze: Squeeze) -> bool {
    let crowded =
        s.hug > base.hug || s.tight > base.tight || squeeze.border > squeeze.base_border + EPS;
    base.node == 0
        && base.overlap == 0
        && (base.frame > 0 || base.chip > 0)
        && (squeeze.narrow > TIGHT_RUN_MIN && crowded
            || base.chip == 0
                && (s.hug as f64 > base.hug as f64 + HUG_TRADE
                    || s.tight as f64 > base.tight as f64 + TIGHT_RUN_MIN))
}

/// Px of `pts` running through a channel narrower than two frame
/// separations: between two parallel frame borders or node sides (not
/// its own ends') closer than `2 * FRAME_SEP` across it, each beside the
/// stretch counted (G4).
fn narrow_px(w: &World, info: &Info, pts: &[Pt]) -> f64 {
    let ends = [end_node(info.ends.0), end_node(info.ends.1)];
    let probe = grow(bounds(pts), 2.0 * FRAME_SEP);
    // Every obstacle side line near the path: (rect, is a frame).
    let rects: Vec<R> = w
        .frames
        .iter()
        .copied()
        .chain(
            w.nodes
                .iter()
                .enumerate()
                .filter(|(v, _)| !ends.contains(&Some(*v)))
                .map(|(_, n)| n.0),
        )
        .filter(|&r| overlaps(probe, r))
        .collect();
    let mut px = 0.0;
    for q in simplify(pts).windows(2) {
        let (dx, dy) = (q[1].0 - q[0].0, q[1].1 - q[0].1);
        let l = norm(dx, dy);
        if l <= EPS {
            continue;
        }
        let axis = if dx.abs() / l > PARALLEL_COS {
            0
        } else if dy.abs() / l > PARALLEL_COS {
            1
        } else {
            continue;
        };
        let (c, _) = cross_along(axis, ((q[0].0 + q[1].0) / 2.0, (q[0].1 + q[1].1) / 2.0));
        let (a, b) = {
            let (l0, l1) = (cross_along(axis, q[0]).1, cross_along(axis, q[1]).1);
            (l0.min(l1), l0.max(l1))
        };
        // The nearest side line each way across, with the stretch it
        // runs beside.
        let (mut below, mut above) = ((f64::NEG_INFINITY, 0.0), (f64::INFINITY, 0.0));
        for &r in &rects {
            let (c0, c1, s0, s1) = if axis == 0 {
                (r.1, r.3, r.0, r.2)
            } else {
                (r.0, r.2, r.1, r.3)
            };
            let beside = b.min(s1) - a.max(s0);
            if beside <= EPS {
                continue;
            }
            for line in [c0, c1] {
                if line <= c && line > below.0 {
                    below = (line, beside);
                } else if line > c && line < above.0 {
                    above = (line, beside);
                }
            }
        }
        if above.0 - below.0 < 2.0 * FRAME_SEP {
            px += f64::min(below.1, above.1);
        }
    }
    px
}

/// Accept edge `i` as `pts` for the edges searched after it: its
/// simplified polyline goes into the segment index, which later
/// lattices are priced from.
fn accept(acc: &mut Accepted, i: usize, ends: (EndRef, EndRef), pts: &[Pt]) {
    acc.put(i, ends, &simplify(pts));
}

/// Reroute the edges that violate clearance; sets `moved[i]` for each
/// replaced edge.
pub(crate) fn repair(g: &Graph, sc: &mut Scene, cx: &Ctx, mode: Mode, moved: &mut [bool]) {
    let ne = sc.edges.len();
    if ne == 0 || ne > ROUTE_MAX_EDGES {
        return;
    }
    let mut w = World::new(g, sc);
    if !w.sane() {
        return;
    }
    if mode == Mode::Routed {
        w.chip_reach = NODE_CLEARANCE;
    }
    let infos: Vec<Option<Info>> = (0..ne).map(|i| info_of(g, &w, cx, i)).collect();
    let polys: Vec<Vec<Pt>> = sc.edges.iter().map(edge_polyline).collect();
    let mut accepted = Accepted::new(ne, w.outer);
    // The whole repair is paid from one budget (G13), scoring included;
    // a fixed edge is an obstacle only and costs no scoring.
    let free = if ne == 0 {
        1.0
    } else {
        cx.fixed.iter().filter(|&&f| !f).count() as f64 / ne as f64
    };
    let mut work = Work::of(mode, free);
    let fixed = |i: usize| cx.fixed.get(i).copied().unwrap_or(false);
    // Obstacle score alone, then accept the clean edges — channel edges
    // (with waypoints) first, so a free curve yields to them.
    let mut order: Vec<usize> = (0..ne).filter(|&i| infos[i].is_some()).collect();
    // Builder channel edges first; a path drawn orthogonally (a reroute
    // of an earlier pass) keeps its place, so a second pass sees the
    // edges in the order the first did.
    let channel = |i: usize| {
        let e = &sc.edges[i];
        !e.waypoints.is_empty() && ortho_corners(e).is_none()
    };
    order.sort_by_key(|&i| (!channel(i), i));
    let mut bad: Vec<(usize, usize)> = Vec::new(); // (class, edge): 0 hard, 1 soft
                                                   // Per edge: its score against the scene alone.
    let mut alone: Vec<Score> = vec![Score::default(); ne];
    // The accepted edges drawn as the builder drew them (no orthogonal
    // path of an earlier pass): a cramped run beside a reroute is that
    // reroute's own trade, so it never sends an edge the first pass left
    // alone into a search on the second.
    let mut built_acc = Accepted::new(ne, w.outer);
    // Fixed edges ([`Ctx::fixed`]) are accepted first, as drawn, and
    // never scored or searched: only obstacles to the others.
    for &i in order.iter().filter(|&&i| fixed(i)) {
        let info = infos[i].as_ref().expect("filtered");
        accept(&mut accepted, i, info.ends, &polys[i]);
        if ortho_corners(&sc.edges[i]).is_none() {
            accept(&mut built_acc, i, info.ends, &polys[i]);
        }
    }
    for &i in order.iter().filter(|&&i| !fixed(i)) {
        let info = infos[i].as_ref().expect("filtered");
        alone[i] = w.score(info, i, &polys[i], None);
        if alone[i].hard() > 0 {
            bad.push((0, i));
            continue;
        }
        // In `route()` only a run on top of an accepted edge matters
        // here (the quick test).
        let simple = simplify(&polys[i]);
        let s = w.against(
            info,
            i,
            &simple,
            &mut accepted,
            alone[i],
            mode == Mode::Static,
        );
        if s.overlap > 0 {
            bad.push((0, i));
        } else {
            // Cramped by a node, beside an accepted edge or along a
            // border: searched for a clearer channel, kept when none is.
            // In `route()` only a cramped node is worth a search: a drag
            // has no budget to spare for spacing, which the nudge does.
            // A labelled edge is not rerouted for spacing alone unless it
            // rides a border or node side for over [`HUG_TRADE`]: its
            // label sits on its own curve, while on a reroute the label
            // stage must find it a new spot, and in the crowded places
            // that spacing is wanted that lands on other strokes or
            // arrowheads (G11 over G4).
            let soft = mode == Mode::Static
                && (sc.edges[i].label.is_none() || s.hug as f64 > HUG_TRADE)
                && (s.hug > 0
                    || s.tight > 0
                        && w.against(info, i, &simple, &mut built_acc, alone[i], true)
                            .tight
                            > 0);
            if s.near > 0 || soft {
                bad.push((1, i));
            }
            accept(&mut accepted, i, info.ends, &polys[i]);
            if ortho_corners(&sc.edges[i]).is_none() {
                accept(&mut built_acc, i, info.ends, &polys[i]);
            }
        }
    }
    if bad.is_empty() {
        return;
    }
    // A broken edge stays where it is until its turn comes (and for good
    // when no reroute beats it), so the edges searched before it are
    // priced against it too.
    for &(class, i) in &bad {
        if class == 0 {
            let info = infos[i].as_ref().expect("filtered");
            accept(&mut accepted, i, info.ends, &polys[i]);
        }
    }
    // Hard before soft, and edges through a node or frame before those
    // that only overlap another (so a spent budget leaves the lesser
    // defects); forward before back; short before long.
    let straight = |i: usize| {
        let (a, b) = drawn_ends(&sc.edges[i]);
        dist(a, b)
    };
    let is_back = |i: usize| infos[i].as_ref().is_some_and(|f| f.back);
    bad.sort_by(|x, y| {
        x.0.cmp(&y.0)
            .then((alone[x.1].hard() == 0).cmp(&(alone[y.1].hard() == 0)))
            .then(is_back(x.1).cmp(&is_back(y.1)))
            .then(straight(x.1).total_cmp(&straight(y.1)))
            .then(x.1.cmp(&y.1))
    });
    let mut taken: Vec<(EndRef, Pt, usize)> = Vec::new();
    for (i, info) in infos.iter().enumerate() {
        if let Some(info) = info {
            let (a, b) = drawn_ends(&sc.edges[i]);
            taken.extend([(info.ends.0, a, i), (info.ends.1, b, i)]);
        }
    }
    let mut scratch = Scratch::default();
    // The first pass is not charged: every edge is scored once.
    let _ = w.spent();
    let mut routes: Vec<Option<Vec<Pt>>> = vec![None; ne];
    let mut detour_work = work.detour;
    // Per edge: its node hits as it now stands, for the rescue below.
    let mut through: Vec<usize> = alone.iter().map(|s| s.node).collect();
    // In `route()` a foreign frame the edge cuts through outweighs a
    // short run on top of another edge (up to [`FRAME_OVERLAP_TRADE`]
    // per frame it leaves): a curve across frames and along their
    // borders reads as belonging to them (G6), while the nudge spreads
    // a short shared stretch. The static scene keeps its curve instead
    // (G1 over G6), as its channels have the room to clear both.
    let frames_hard = |s: &Score, base: &Score| {
        mode == Mode::Routed
            && s.frame < base.frame
            && s.overlap.saturating_sub(base.overlap)
                <= FRAME_OVERLAP_TRADE * (base.frame - s.frame)
    };
    for &(class, i) in &bad {
        if work.left == 0 && (class == 1 || alone[i].hard() == 0 || detour_work == 0) {
            continue; // keeps its geometry (already accepted)
        }
        let info = infos[i].as_ref().expect("filtered");
        let base0 = alone[i];
        // Against the accepted edges too (a pending original is in the
        // index already; a soft edge is scored without itself). Scored
        // up front only when the narrow-channel rule may apply (its
        // scene part says so); otherwise only when no reroute was taken
        // outright, as the search leaves the index as it was.
        // Out of search work and of detour work for this edge's setup:
        // nothing can change it.
        let setup = w.nodes.len() + w.chips.len() + info.blocked_frames.len();
        if work.left == 0 && detour_work.min(DETOUR_EDGE_WORK) < setup {
            continue;
        }
        let mut base_acc: Option<Score> = None;
        let can_trade = base0.node == 0 && (base0.frame > 0 || base0.chip > 0);
        // The original's border ride, once a candidate needs it.
        let base_border = std::cell::OnceCell::new();
        // The narrow channel counts in the static scene only: a drag's
        // reroute is searched in a small window and budget, and the
        // curve it would keep often crosses the frame (G13 over G4).
        let squeeze = |pts: &[Pt]| Squeeze {
            narrow: if mode == Mode::Static {
                narrow_px(&w, info, pts)
            } else {
                0.0
            },
            border: border_px(&w.frames, pts),
            base_border: *base_border.get_or_init(|| border_px(&w.frames, &polys[i])),
        };
        let mut best: Option<(Score, Vec<Pt>, Vec<Pt>)> = None;
        // A clean, monotone route beats an original that hits a node or
        // frame whatever the other edges say (the score is ordered), so
        // neither is scored against them.
        let mut decided = false;
        // The builder's ports are free (every other side costs
        // [`SIDE_SWITCH_COST`]), and the path never steps back along the
        // flow (G8) unless the edge already does.
        // Inside a window round the ends, any side, clear of every band
        // and foreign frame.
        let first = Try {
            lat: 0,
            alt: true,
            soft: false,
            soft_walls: false,
            reach: ROUTE_WINDOW,
        };
        // Its smoothed curve still touches something: a wider clearance.
        let wider = Try { lat: 1, ..first };
        // No such path: over a wider window (the whole drawing when
        // static), bands and frames priced.
        let walls = Try {
            soft_walls: true,
            reach: work.walls_reach,
            ..first
        };
        // Last, for an edge that wiggles already: steps back priced too.
        let back = Try {
            soft: true,
            ..walls
        };
        let mut n_found = 0;
        let mut built: Built = Vec::new();
        for t in [first, wider, walls, back] {
            // (A frame the path still enters is no matter of clearance: in
            // `route()` a wider lattice is not tried for it.)
            if t.lat == 1
                && !best.as_ref().is_some_and(|b| {
                    b.0.node + b.0.chip > 0
                        || b.0.near > 0
                        || mode == Mode::Static
                        || b.0.frame > 0 && b.0.frame < base0.frame
                })
            {
                continue; // nothing to widen, or nothing it touches
            }
            // A clear path exists (in `route()`, any path: a drag cannot
            // afford the wide window for a run beside another edge).
            let enough = |b: &(Score, Vec<Pt>, Vec<Pt>)| {
                b.0.hard() == 0 && (b.0.overlap == 0 || mode == Mode::Routed)
            };
            if t.soft_walls
                && !t.soft
                && (n_found > 0 && mode == Mode::Routed || best.as_ref().is_some_and(enough))
            {
                continue;
            }
            // In `route()` an edge that only enters a frame keeps it for
            // the channel detour below: when no strict path got by, the
            // priced one crosses a frame too, at the cost of a wide
            // window a drag cannot spare (G13).
            if t.soft_walls && mode == Mode::Routed && base0.node + base0.chip == 0 {
                continue;
            }
            if t.soft && base0.wiggle == 0 {
                continue; // a backtrack would add a wiggle (G8)
            }
            if work.left == 0 {
                break; // the detour below still runs
            }
            let e = &sc.edges[i];
            let mut env = Env {
                w: &w,
                taken: &taken,
                tracks: mode == Mode::Static,
                acc: &mut accepted,
                built: &mut built,
                work: &mut work,
                sc: &mut scratch,
            };
            let Some(corners) = route_edge(&mut env, info, i, e, t) else {
                continue;
            };
            n_found += 1;
            let pts = drawn_with(e, &smooth(&corners, CORNER_R));
            // Charged by what the scoring looked at, even when that
            // overdraws: the search is paid for, so the candidate is
            // still scored, and the next check stops the repair. One
            // candidate's scoring is the most a repair goes over its
            // budget.
            let s = w.score(info, i, &pts, Some(&mut accepted));
            let _ = work.take(w.spent());
            let clean = base0.hard() > 0 && s.hard() == 0 && s.wiggle == 0;
            // Clean of the scene and on top of no accepted edge (G1): taken
            // at once; otherwise it still beats the original below, but a
            // later try may find one that is.
            let lone = s.overlap == 0;
            // A candidate the original keeps its curve over is passed
            // by: a later try may find a clear channel.
            let traded = clean
                && lone
                && can_trade
                && trades(
                    base_acc.get_or_insert_with(|| {
                        w.against(info, i, &simplify(&polys[i]), &mut accepted, base0, true)
                    }),
                    &s,
                    squeeze(&pts),
                );
            if traded {
                continue;
            }
            if clean && lone {
                best = Some((s, corners, pts));
                decided = true;
                break;
            }
            if best.as_ref().map_or(true, |b| s < b.0) {
                best = Some((s, corners, pts));
            }
            if s.hard() == 0 && s.wiggle == 0 && lone {
                break;
            }
        }
        // Still through a node or frame: the channel detour, when it
        // hits fewer of them (G6) and adds no wiggle.
        let fixed = |s: &Score| (s.node, s.frame, s.chip) < (base0.node, base0.frame, base0.chip);
        if !decided && base0.hard() > 0 && !best.as_ref().is_some_and(|b| fixed(&b.0)) {
            let mut mine = detour_work.min(DETOUR_EDGE_WORK);
            let found = detour(&w, info, i, &sc.edges[i], &mut accepted, &mut mine, false);
            detour_work -= detour_work.min(DETOUR_EDGE_WORK) - mine;
            if let Some((s, corners, pts)) = found {
                let trade = can_trade
                    && trades(
                        base_acc.get_or_insert_with(|| {
                            w.against(info, i, &simplify(&polys[i]), &mut accepted, base0, true)
                        }),
                        &s,
                        squeeze(&pts),
                    );
                // On top of another edge for over MIN_OVERLAP_RUN (G1):
                // not the clear channel G6 asks for. An original that
                // only crosses frames keeps its curve; one through a node
                // or a title chip still takes it (hiding a node or a
                // frame's name is worse). See [`frames_hard`].
                let stacked =
                    s.overlap > 0 && base0.node + base0.chip == 0 && !frames_hard(&s, &base0);
                if fixed(&s) && s.wiggle <= base0.wiggle && !trade && !stacked {
                    best = Some((s, corners, pts));
                    decided = true;
                }
            }
        }
        if !decided && best.is_none() {
            continue; // nothing to compare: the edge keeps its geometry
        }
        let base = if decided {
            base0
        } else {
            base_acc.unwrap_or_else(|| {
                w.against(info, i, &simplify(&polys[i]), &mut accepted, base0, true)
            })
        };
        let _ = work.take(w.spent());
        let better = |s: &Score, pts: &[Pt]| {
            if decided {
                return true;
            }
            // Never add a wiggle (G8), nor squeeze along a border where
            // the original only crossed a frame's empty area.
            s.wiggle <= base.wiggle
                && !trades(&base, s, squeeze(pts))
                // Never trade a frame crossing for a run on top of
                // another edge (G1 over G6, see the detour above), but
                // in `route()` (see [`frames_hard`]).
                && !(s.overlap > base.overlap
                    && base.node + base.chip == 0
                    && !frames_hard(s, &base))
                // A static curve that hides no node or chip is kept over a
                // reroute that rides along a node it cramps, or along a
                // side while crossing another edge at its arrowhead (G4,
                // G11; a shape worse than the curve), unless the reroute
                // also takes it off a long run on top of another edge
                // (G1).
                && !(mode == Mode::Static
                    && base.node + base.chip == 0
                    && base.overlap <= s.overlap + FRAME_OVERLAP_TRADE
                    && (s.near > base.near || s.head > base.head)
                    && s.hug as f64 > base.hug as f64 + HUG_TRADE)
                && if class == 0 {
                    s < &base
                } else {
                    s.hard() == 0 && s.overlap == 0 && s < &base
                }
        };
        match best {
            Some((s, corners, pts)) if better(&s, &pts) => {
                // Its two entries, source first.
                let ends = [corners[0], corners[corners.len() - 1]];
                for (t, p) in taken.iter_mut().filter(|t| t.2 == i).zip(ends) {
                    t.1 = p;
                }
                routes[i] = Some(corners);
                accept(&mut accepted, i, info.ends, &pts);
                through[i] = s.node;
            }
            // Otherwise a hard edge keeps its pending original and a soft
            // one stays accepted as it was.
            _ => {}
        }
    }
    // The rescue: every edge still through a node (the searches, the
    // channel detour or their budgets gave out) gets a detour that may
    // cross frames but keeps clear of every node and chip, the edge
    // through the most nodes first (G6 asks for the node first; a frame
    // entry is the lesser defect). Never left a free curve across the
    // nodes between its ends.
    let mut rescue_work = RESCUE_TOTAL_WORK;
    let mut still: Vec<usize> = (0..ne)
        .filter(|&i| through[i] > 0 && infos[i].is_some() && !fixed(i))
        .collect();
    still.sort_by_key(|&i| (Reverse(through[i]), i));
    for i in still {
        let info = infos[i].as_ref().expect("filtered");
        let e = &sc.edges[i];
        let was = match routes[i].as_ref() {
            Some(c) => drawn_with(e, &smooth(c, CORNER_R)),
            None => polys[i].clone(),
        };
        let now = w.score(info, i, &was, None);
        let found = detour(&w, info, i, e, &mut accepted, &mut rescue_work, true);
        if let Some((s, corners, pts)) = found {
            if s.node < now.node && s.chip <= now.chip && s.wiggle <= now.wiggle {
                routes[i] = Some(corners);
                accept(&mut accepted, i, info.ends, &pts);
            }
        }
    }
    for i in 0..ne {
        let Some(corners) = routes[i].as_ref() else {
            continue;
        };
        let wps = smooth(corners, CORNER_R);
        let e = &mut sc.edges[i];
        let (p0, p3) = (wps[0], wps[wps.len() - 1]);
        e.waypoints = wps;
        e.bezier = [p0, p0, p3, p3];
        moved[i] = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::quality::measure_source;
    use crate::scene::{route, scene, to_svg};

    /// Bug (a): the loop back from the error step.
    const BUG_A: &str = "flowchart TD\nA([Start])-->B[Read input]\nB-->C{Valid?}\nC-->|yes|D\nC-->|no|E[Show error]\nE-->B\nD==>F((Done))";

    /// Bug (b): a hub fanning out to stacked frames in LR, plus one
    /// plain node below them.
    const BUG_B: &str = "flowchart LR\nhub[hub]\nsubgraph P[prod zone]\ndirection TB\nc1[svc1]\nc2[svc2]\nc3[svc3]\nc4[svc4]\nend\nsubgraph D[dev zone]\ndirection TB\ne1[d1]\ne2[d2]\ne3[d3]\ne4[d4]\ne5[d5]\ne6[d6]\nend\nsubgraph R[dr zone]\ndirection TB\nh1[r1]\nh2[r2]\nend\nsubgraph N[g4]\nn1[n1]\nend\next[external]\nhub --> P\nhub --> D\nhub --> R\nhub -.-> N\nhub --> ext";

    fn unlimited() -> Work {
        Work {
            left: usize::MAX,
            per_search: usize::MAX,
            max_cells: usize::MAX,
            walls_reach: f64::INFINITY,
            detour: usize::MAX,
        }
    }

    fn centres(g: &Graph) -> Vec<Pt> {
        scene(g).nodes.iter().map(|n| (n.x, n.y)).collect()
    }

    /// The (edge, unrelated node) and (edge, foreign frame) hits of one
    /// scene edge, measured like the harness.
    fn hits(g: &Graph, sc: &Scene, from: &str, to: &str) -> Score {
        let cx = crate::edgepass::ctx(g, sc);
        // A chip counts when touched, as in `route()`.
        let mut w = World::new(g, sc);
        w.chip_reach = NODE_CLEARANCE;
        let i = sc
            .edges
            .iter()
            .position(|e| e.from == from && e.to == to)
            .expect("edge");
        let (a, b) = cx.ends[i].unwrap();
        let mut allowed: Vec<usize> = frames_of(&cx, a)
            .iter()
            .chain(frames_of(&cx, b))
            .copied()
            .collect();
        allowed.sort_unstable();
        let info = Info {
            ends: (a, b),
            allowed,
            blocked_frames: Vec::new(),
            back: false,
            sideways: false,
        };
        w.score(&info, i, &edge_polyline(&sc.edges[i]), None)
    }

    /// The same hits as [`hits`], counted by `crate::quality` (the
    /// harness's own metric code), so a fault in [`World::score`]
    /// cannot hide itself.
    fn measured_hits(g: &Graph, sc: &Scene, from: &str, to: &str) -> Vec<String> {
        let key = format!("{from}->{to} ");
        crate::quality::measure(g, sc)
            .details
            .into_iter()
            .filter(|d| {
                d.strip_prefix(&key).is_some_and(|rest| {
                    rest.starts_with("edge_node ") || rest.starts_with("edge_frame ")
                })
            })
            .collect()
    }

    #[test]
    fn routed_back_edge_bows_round_the_diamond() {
        let g = parse(BUG_A).unwrap();
        let sc = route(&g, &centres(&g));
        assert_eq!(hits(&g, &sc, "E", "B").hard(), 0, "{:?}", sc.edges[4]);
        assert_eq!(measured_hits(&g, &sc, "E", "B"), Vec::<String>::new());
        // And the static layout keeps it clear too.
        let st = scene(&g);
        assert_eq!(hits(&g, &st, "E", "B").hard(), 0);
        assert_eq!(measured_hits(&g, &st, "E", "B"), Vec::<String>::new());
    }

    #[test]
    fn routed_hub_edge_stays_out_of_foreign_frames() {
        let g = parse(BUG_B).unwrap();
        let sc = route(&g, &centres(&g));
        for to in ["P", "D", "R", "N", "ext"] {
            assert_eq!(hits(&g, &sc, "hub", to).hard(), 0, "hub->{to}");
            assert_eq!(measured_hits(&g, &sc, "hub", to), Vec::<String>::new());
        }
    }

    #[test]
    fn routed_back_edge_bows_round_in_every_direction() {
        for dir in ["BT", "RL", "LR"] {
            let src = BUG_A.replacen("TD", dir, 1);
            let g = parse(&src).unwrap();
            let sc = route(&g, &centres(&g));
            assert_eq!(hits(&g, &sc, "E", "B").hard(), 0, "{dir}");
            assert_eq!(
                measured_hits(&g, &sc, "E", "B"),
                Vec::<String>::new(),
                "{dir}"
            );
        }
    }

    /// `n` one-node frames in a row with a hub before them and a sink
    /// after: every hub edge has frames it must stay out of, and with
    /// more than 64 frames the masks span several words.
    fn many_frames(n: usize) -> String {
        let mut s = String::from("flowchart LR\nhub\n");
        for k in 0..n {
            s += &format!("subgraph F{k}[f{k}]\nv{k}\nend\nhub --> v{k}\n");
        }
        s += &format!("hub --> sink\nv{} --> sink\n", n - 1);
        s
    }

    #[test]
    fn frame_masks_span_more_than_one_word() {
        let g = parse(&many_frames(70)).unwrap();
        let c = centres(&g);
        let sc = route(&g, &c);
        // Deterministic, and no rerouted edge enters a frame it avoided
        // as drawn, the high-index ones included.
        assert_eq!(to_svg(&sc), to_svg(&route(&g, &c)));
        for k in [0, 63, 64, 69] {
            let v = format!("v{k}");
            assert_eq!(hits(&g, &sc, "hub", &v).frame, 0, "hub->{v}");
        }
    }

    #[test]
    fn far_apart_centres_stay_cheap() {
        // Host-given centres can be arbitrarily far apart: scoring must
        // not sample per px, and an absurd extent skips the repair.
        let g = parse("flowchart TD\nA-->B\nB-->C\nA-->C").unwrap();
        for d in [5.0e5, 1.0e7, 1.0e12] {
            let t = std::time::Instant::now();
            let sc = route(&g, &[(0.0, 0.0), (0.0, d), (0.0, 2.0 * d)]);
            assert_eq!(sc.edges.len(), 3);
            assert!(t.elapsed().as_secs_f64() < 1.0, "{d}: {:?}", t.elapsed());
        }
    }

    #[test]
    fn non_finite_centres_do_not_panic() {
        // A bad drag value (NaN / inf) from the host must not trap the
        // wasm module: route() returns a scene, it does not panic.
        let srcs = [
            include_str!("../tests/fixtures/quality/bench-75.mmd"),
            include_str!("../tests/fixtures/quality/dense-lr.mmd"),
            include_str!("../tests/fixtures/quality/fanout-frames.mmd"),
            include_str!("../tests/fixtures/quality/wide-labels.mmd"),
        ];
        for src in srcs {
            let g = parse(src).unwrap();
            let base = centres(&g);
            for bad in [
                (f64::NAN, 0.0),
                (f64::NAN, f64::INFINITY),
                (f64::INFINITY, f64::NEG_INFINITY),
            ] {
                let mut c = base.clone();
                c[0] = bad;
                let sc = route(&g, &c);
                assert_eq!(sc.edges.len(), route(&g, &base).edges.len());
            }
        }
    }

    /// A synthetic flowchart: `n` nodes in `subs` frames, `ne` edges
    /// between pseudo-random pairs (a fixed LCG, so every run is the
    /// same diagram).
    fn synthetic(n: usize, ne: usize, subs: usize, dir: &str) -> String {
        let mut s = format!("flowchart {dir}\n");
        let per = n / subs;
        for k in 0..subs {
            s += &format!("subgraph S{k}[grp {k}]\n");
            for v in k * per..(k + 1) * per {
                s += &format!("n{v}[node {v}]\n");
            }
            s += "end\n";
        }
        let mut x: u64 = 12_345;
        let mut next = || {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (x >> 33) as usize % n
        };
        for _ in 0..ne {
            let (a, b) = (next(), next());
            s += &format!("n{a}-->n{b}\n");
        }
        s
    }

    /// Runs a frame over its ceiling is timed again (fastest of), so a
    /// burst of load on a shared machine, which can hit all of the first
    /// few runs, does not fail a drag guard; a frame that is really slow
    /// stays slow.
    const RETIME_RUNS: usize = 10;

    /// `ms` of a frame timed by `time(runs)`, timed again over
    /// [`RETIME_RUNS`] when it is over `ceiling`.
    fn steady(ms: f64, ceiling: f64, time: impl Fn(usize) -> f64) -> f64 {
        if ms < ceiling {
            ms
        } else {
            ms.min(time(RETIME_RUNS))
        }
    }

    /// Fastest of `runs` `route()` calls on centres `c` (ms), so one
    /// scheduler hiccup does not fail the guard.
    fn route_ms(g: &Graph, c: &[Pt], runs: usize) -> f64 {
        (0..runs)
            .map(|_| {
                let t = std::time::Instant::now();
                route(g, c);
                t.elapsed().as_secs_f64() * 1e3
            })
            .fold(f64::INFINITY, f64::min)
    }

    /// Native release perf guard for the interactive path (G13): run
    /// with `cargo test --release --lib -- --ignored --test-threads=1
    /// perf_guard` (one test at a time: they time wall clock, and a
    /// parallel run measures the other guards' load). The ceilings leave
    /// headroom over the measured times; a debug build is far slower and
    /// is not what they bound.
    #[test]
    #[ignore]
    fn route_perf_guard() {
        for (n, ne, subs, dir, ceiling_ms) in [
            (75, 75, 5, "LR", 8.0),
            (150, 200, 10, "TD", 15.0),
            (300, 400, 20, "LR", 25.0),
        ] {
            let g = parse(&synthetic(n, ne, subs, dir)).unwrap();
            let best = route_ms(&g, &centres(&g), 5);
            eprintln!("route {ne} edges: {best:.2} ms");
            assert!(best < ceiling_ms, "{ne} edges: {best:.2} ms");
        }
    }

    /// `examples/bench`'s diagram: `layers` x `width` nodes, two edges
    /// from each node to pseudo-random nodes of the next layer, and
    /// `layers` back-edges from the last layer to the first.
    fn layered(layers: usize, width: usize) -> String {
        let mut s = String::from("flowchart TD\n");
        let mut x: u64 = 42;
        let mut next = |n: usize| {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (x >> 33) as usize % n
        };
        for l in 0..layers - 1 {
            for w in 0..width {
                for _ in 0..2 {
                    let to = (l + 1) * width + next(width);
                    s += &format!(
                        "N{}[Node {}] --> N{to}[Node {to}]\n",
                        l * width + w,
                        l * width + w
                    );
                }
            }
        }
        for _ in 0..layers {
            let a = (layers - 1) * width + next(width);
            let b = next(width);
            s += &format!("N{a} --> N{b}\n");
        }
        s
    }

    /// Native release perf guard for the static path: `scene()` plus
    /// the SVG it emits (the crossing masks) on [`layered`] diagrams.
    /// Same run line as [`route_perf_guard`]. The ceilings leave
    /// headroom over the measured times; the largest row checks that
    /// the repair and the masks stay off past their edge caps.
    #[test]
    #[ignore]
    fn scene_perf_guard() {
        let static_ms = |g: &Graph| {
            (0..3)
                .map(|_| {
                    let t = std::time::Instant::now();
                    let svg = to_svg(&scene(g));
                    assert!(!svg.is_empty());
                    t.elapsed().as_secs_f64() * 1e3
                })
                .fold(f64::INFINITY, f64::min)
        };
        for (layers, width, ceiling_ms) in [(10, 5, 40.0), (20, 10, 150.0), (50, 20, 1_500.0)] {
            let g = parse(&layered(layers, width)).unwrap();
            let best = static_ms(&g);
            let ne = g.edges.len();
            eprintln!("scene {ne} edges: {best:.2} ms");
            assert!(best < ceiling_ms, "{ne} edges: {best:.2} ms");
        }
        // The fixtures, against 0.31.1 (`scene()+to_svg`, fastest of 7,
        // native release; ms): the ceilings bound the ratio the repair
        // and the layout search may cost over it, at about 1.5x the
        // measured ratio (bench-75 23x, the generated dense ones 15x,
        // fanout-frames 9.4x, wide-labels 3.7x, back-edge 2.1x; the
        // label-heavy cases 58x and 35x, router-bound since the label
        // stage got its budget), so growth shows. Not the 5x aimed at:
        // the static repair, nudge and layout search are most of it,
        // and lowering their budgets far enough loses the round-1 wins
        // (see the CHANGELOG); that trade is the owner's call.
        for (name, v0311_ms, max_ratio) in [
            ("bench-75", 0.90, 35.0),
            ("dense-lr", 0.92, 23.0),
            ("dense-td", 1.08, 23.0),
            ("wide-labels", 0.07, 6.0),
            ("back-edge", 0.04, 5.0),
            ("fanout-frames", 0.18, 14.0),
            ("fan-40-labelled", 0.30, 88.0),
            ("parallel-labelled", 0.20, 53.0),
        ] {
            let g = parse(&fixture(name)).unwrap();
            let best = static_ms(&g);
            let ratio = best / v0311_ms;
            eprintln!("scene {name}: {best:.2} ms ({ratio:.1}x 0.31.1)");
            assert!(
                ratio < max_ratio,
                "{name}: {best:.2} ms = {ratio:.1}x 0.31.1"
            );
        }
    }

    /// A committed quality fixture's source, or one of the label-heavy
    /// cases built here: `fan-40-labelled` (one node, 40 labelled
    /// out-edges) and `parallel-labelled` (25 labelled edges between the
    /// same two nodes), where the label stage, not the router, used to
    /// grow without bound.
    fn fixture(name: &str) -> String {
        match name {
            "fan-40-labelled" => (0..40).fold("flowchart LR\n".to_string(), |s, i| {
                s + &format!("h -->|step {i}| n{i}\n")
            }),
            "parallel-labelled" => (0..25).fold("flowchart TD\n".to_string(), |s, i| {
                s + &format!("A -->|lbl {i}| B\n")
            }),
            _ => {
                let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
                std::fs::read_to_string(format!("{dir}/{name}.mmd")).unwrap()
            }
        }
    }

    /// What G13 bounds is a drag frame, not the first layout: the guard
    /// drags nodes of the 75-edge diagram across the drawing and also
    /// scatters every node, and bounds the worst frame. Same run line
    /// as [`route_perf_guard`].
    #[test]
    #[ignore]
    fn route_drag_perf_guard() {
        /// Nodes dragged, and drag frames per node.
        const DRAGGED: usize = 20;
        const STEPS: usize = 10;
        /// Fully scattered layouts timed.
        const SCATTERS: usize = 10;
        /// Worst frame allowed (ms, native release): the 75-edge row of
        /// [`route_perf_guard`], i.e. G13's 16 ms wasm at ~2x native.
        const CEILING_MS: f64 = 8.0;
        let g = parse(&synthetic(75, 75, 5, "LR")).unwrap();
        let base = centres(&g);
        let b = bounds(&base);
        let at = |tx: f64, ty: f64| (b.0 + (b.2 - b.0) * tx, b.1 + (b.3 - b.1) * ty);
        let mut worst = (0.0f64, String::new());
        let mut note = |ms: f64, what: String| {
            if ms > worst.0 {
                worst = (ms, what);
            }
        };
        for v in 0..DRAGGED.min(base.len()) {
            for k in 0..STEPS {
                let t = k as f64 / (STEPS - 1) as f64;
                let mut c = base.clone();
                c[v] = at(t, 1.0 - t);
                let ms = steady(route_ms(&g, &c, 3), CEILING_MS, |n| route_ms(&g, &c, n));
                note(ms, format!("drag n{v} step {k}"));
            }
        }
        let mut x: u64 = 7;
        let mut unit = || {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (x >> 11) as f64 / (1u64 << 53) as f64
        };
        for s in 0..SCATTERS {
            let c: Vec<Pt> = base.iter().map(|_| at(unit(), unit())).collect();
            let ms = steady(route_ms(&g, &c, 3), CEILING_MS, |n| route_ms(&g, &c, n));
            note(ms, format!("scatter {s}"));
        }
        let squeezed: Vec<Pt> = base.iter().map(|p| (p.0 / 3.0, p.1 / 3.0)).collect();
        note(route_ms(&g, &squeezed, 3), "squeezed".into());
        note(
            route_ms(&g, &vec![(0.0, 0.0); base.len()], 3),
            "coincident".into(),
        );
        eprintln!("route drag worst: {:.2} ms ({})", worst.0, worst.1);
        assert!(worst.0 < CEILING_MS, "{:.2} ms at {}", worst.0, worst.1);
    }

    /// G13 on the committed dense fixtures: every node dragged to every
    /// point of [`crate::quality::drag_frames`], the fastest of
    /// [`crate::quality::DRAG_RUNS`] per frame. `route_partial` (the
    /// editor's drag API, with the static scene as its base) must stay
    /// under the 8 ms proxy for 16 ms in wasm. This covers one node
    /// dragged from the auto layout: a frame where more than half the
    /// edges are redrawn (many nodes already moved) is a `route()` frame
    /// and is reported with it.
    ///
    /// A full `route()` frame is timed against the same 8 ms and
    /// reported, not asserted: G13 is not met for `route()` yet (its
    /// worst frames take 7-9 ms here, the repair being most of it, and
    /// a router budget low enough to fit breaks the routed quality
    /// gates). The two label-heavy cases are reported for
    /// `route_partial` too: their label stage is budgeted, but dragging
    /// the hub redraws every edge, and the first scoring pass over 40
    /// edges sharing one port side is unbudgeted (10-11 ms). No looser
    /// ceiling is written down for either, so the misses stay visible.
    /// Same run line as [`route_perf_guard`].
    #[test]
    #[ignore]
    fn fixture_drag_perf_guard() {
        use crate::quality::{drag_frames, fastest_ms, DRAG_FRAME_MS, DRAG_RUNS};
        use crate::scene::route_partial;
        for (name, asserted) in [
            ("dense-lr", true),
            ("dense-td", true),
            ("bench-75", true),
            ("fan-40-labelled", false),
            ("parallel-labelled", false),
        ] {
            let g = parse(&fixture(name)).unwrap();
            let st = scene(&g);
            let base: Vec<Pt> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            let (mut full, mut part) = ((0.0f64, 0), (0.0f64, 0));
            let mut full_over = 0;
            for (v, c) in drag_frames(&st) {
                let time = |n: usize| fastest_ms(n, || route(&g, &c));
                let ms = steady(time(DRAG_RUNS), DRAG_FRAME_MS, time);
                full_over += usize::from(ms > DRAG_FRAME_MS);
                if ms > full.0 {
                    full = (ms, v);
                }
                let time = |n: usize| fastest_ms(n, || route_partial(&g, &c, &st, &base));
                let ms = steady(time(DRAG_RUNS), DRAG_FRAME_MS, time);
                if ms > part.0 {
                    part = (ms, v);
                }
            }
            eprintln!(
                "{name}: route_partial worst {:.2} ms (node {}); route() worst {:.2} ms (node {}), \
                 {full_over} frames over {DRAG_FRAME_MS} ms (route() reported, not asserted)",
                part.0, part.1, full.0, full.1
            );
            assert!(
                part.0 < DRAG_FRAME_MS || !asserted,
                "{name} route_partial: {:.2} ms",
                part.0
            );
        }
    }

    #[test]
    fn route_is_deterministic() {
        for src in [BUG_A, BUG_B] {
            let g = parse(src).unwrap();
            let c = centres(&g);
            assert_eq!(to_svg(&route(&g, &c)), to_svg(&route(&g, &c)));
            assert_eq!(to_svg(&scene(&g)), to_svg(&scene(&g)));
        }
    }

    #[test]
    fn lattice_route_goes_round_an_obstacle() {
        // Two ports facing each other with a box in between.
        let xs = with_midlines(vec![0.0, 40.0, 60.0, 100.0]);
        let ys = with_midlines(vec![0.0, 30.0, 70.0, 100.0, 50.0]);
        let mut lat = Lattice::new(xs, ys, Vec::new());
        lat.solid((40.0, 30.0, 60.0, 70.0), 0.0);
        let t = |p: Pt, out: (i64, i64)| Term {
            port: p,
            stub: p,
            out,
            cost: 0.0,
        };
        let (path, _, _) = search(
            &lat,
            &[t((0.0, 50.0), (1, 0))],
            &[t((100.0, 50.0), (-1, 0))],
            Rules {
                flow: None,
                mask: &[],
                soft: true,
            },
            &mut unlimited(),
            &mut Scratch::default(),
        )
        .expect("a path exists");
        let path = collapse(path);
        // Never inside the box, and it does get round.
        for w in path.windows(2) {
            let mid = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
            assert!(!inside((40.0, 30.0, 60.0, 70.0), mid), "{path:?}");
        }
        assert_eq!(path.first(), Some(&(0.0, 50.0)));
        assert_eq!(path.last(), Some(&(100.0, 50.0)));
    }

    #[test]
    fn frame_mask_keeps_a_path_out_of_a_foreign_frame() {
        // A frame between the ports: blocked for the edge, it must be
        // walked round; allowed, the path goes straight through.
        let frame = (40.0, 20.0, 60.0, 80.0);
        let xs = with_midlines(vec![0.0, 40.0, 60.0, 100.0]);
        let ys = with_midlines(vec![0.0, 20.0, 80.0, 100.0, 50.0]);
        // Frame 66 of 70: its bit lies in the second mask word.
        let mut lat = Lattice::new(xs, ys, (0..70).collect());
        lat.frame(66, frame);
        let t = |p: Pt, out: (i64, i64)| Term {
            port: p,
            stub: p,
            out,
            cost: 0.0,
        };
        let (s, g) = ([t((0.0, 50.0), (1, 0))], [t((100.0, 50.0), (-1, 0))]);
        let mut scr = Scratch::default();
        let mut work = unlimited();
        let open = search(
            &lat,
            &s,
            &g,
            Rules {
                flow: None,
                mask: &lat.mask(&[]),
                soft: false,
            },
            &mut work,
            &mut scr,
        )
        .unwrap();
        assert_eq!(collapse(open.0).len(), 2, "straight through");
        let round = search(
            &lat,
            &s,
            &g,
            Rules {
                flow: None,
                mask: &lat.mask(&[66]),
                soft: false,
            },
            &mut work,
            &mut scr,
        )
        .unwrap();
        let round = collapse(round.0);
        assert!(round.len() > 2);
        for w in round.windows(2) {
            let mid = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
            assert!(!inside(frame, mid), "{round:?}");
        }
    }

    #[test]
    fn monotone_search_never_steps_back() {
        // A box right in front of the target: in LR the path may go
        // round it, but never leftwards.
        let xs = with_midlines(vec![0.0, 40.0, 60.0, 100.0]);
        let ys = with_midlines(vec![0.0, 30.0, 70.0, 100.0, 50.0]);
        let mut lat = Lattice::new(xs, ys, Vec::new());
        lat.solid((40.0, 30.0, 60.0, 70.0), 0.0);
        let t = |p: Pt, out: (i64, i64)| Term {
            port: p,
            stub: p,
            out,
            cost: 0.0,
        };
        let flow = Flow {
            dir: crate::model::Direction::LR,
            sign: 1.0,
            back: None,
        };
        let (path, _, _) = search(
            &lat,
            &[t((0.0, 50.0), (1, 0))],
            &[t((100.0, 50.0), (-1, 0))],
            Rules {
                flow: Some(flow),
                mask: &[],
                soft: true,
            },
            &mut unlimited(),
            &mut Scratch::default(),
        )
        .unwrap();
        for w in path.windows(2) {
            assert!(w[1].0 >= w[0].0, "{path:?}");
        }
    }

    #[test]
    fn simplify_keeps_corners_and_drops_collinear_samples() {
        let pts: Vec<Pt> = (0..=10)
            .map(|k| (k as f64 * 10.0, 0.0))
            .chain((1..=5).map(|k| (100.0, k as f64 * 10.0)))
            .collect();
        assert_eq!(
            simplify(&pts),
            vec![(0.0, 0.0), (100.0, 0.0), (100.0, 50.0)]
        );
    }

    /// A bare world: `nodes` as plain boxes, `frames`, nothing else.
    fn bare(nodes: &[R], frames: &[R], outer: R) -> World {
        World {
            dir: crate::model::Direction::LR,
            chip_reach: CHIP_SEP,
            nodes: nodes.iter().map(|&r| (r, Shape::Rect)).collect(),
            frames: frames.to_vec(),
            chips: Vec::new(),
            labels: Vec::new(),
            outer,
            looks: std::cell::Cell::new(0),
        }
    }

    /// A port leg that jogs a few px onto the next lattice line right
    /// after its stub slides the port onto that line instead (G8), but
    /// not off its side nor onto another end's port.
    #[test]
    fn a_short_jog_after_the_port_folds_into_the_port_leg() {
        let w = bare(
            &[(0.0, 0.0, 100.0, 40.0)],
            &[],
            (-500.0, -500.0, 500.0, 500.0),
        );
        let end = EndRef::Node(0);
        // Leaves the bottom at x 50, jogs 8 px right, runs down.
        let pts = vec![(50.0, 40.0), (50.0, 54.0), (58.0, 54.0), (58.0, 200.0)];
        assert_eq!(
            straighten(&w, end, &[], 0, pts.clone()),
            vec![(58.0, 40.0), (58.0, 200.0)]
        );
        // Another edge's port there: kept as it was.
        let taken = [(end, (60.0, 40.0), 1)];
        assert_eq!(straighten(&w, end, &taken, 0, pts.clone()), pts);
        // A jog past PORT_JOG_MAX is a turn, kept.
        let wide = vec![(50.0, 40.0), (50.0, 54.0), (80.0, 54.0), (80.0, 200.0)];
        assert_eq!(straighten(&w, end, &[], 0, wide.clone()), wide);
    }

    #[test]
    fn detour_goes_round_what_a_free_curve_cuts_through() {
        // A -> B drawn straight through C, as a spent lattice budget
        // leaves it: the detour reaches B with C on one side.
        let w = bare(
            &[
                (0.0, 0.0, 40.0, 20.0),
                (300.0, 0.0, 340.0, 20.0),
                (150.0, -20.0, 190.0, 40.0),
            ],
            &[],
            (-60.0, -80.0, 400.0, 120.0),
        );
        let info = Info {
            ends: (EndRef::Node(0), EndRef::Node(1)),
            allowed: Vec::new(),
            blocked_frames: Vec::new(),
            back: false,
            sideways: false,
        };
        let mut e = scene(&parse("flowchart LR\nA-->B").unwrap()).edges[0].clone();
        e.waypoints.clear();
        e.bezier = [(40.0, 10.0), (100.0, 10.0), (240.0, 10.0), (300.0, 10.0)];
        assert_eq!(w.score(&info, 0, &edge_polyline(&e), None).node, 1);
        let mut work = DETOUR_TOTAL_WORK;
        let mut acc = Accepted::new(1, w.outer);
        let (s, corners, _) =
            detour(&w, &info, 0, &e, &mut acc, &mut work, false).expect("a clear detour");
        assert_eq!((s.node, s.frame, s.wiggle), (0, 0, 0), "{corners:?}");
        assert_eq!(corners[0], (40.0, 10.0));
        assert_eq!(corners[corners.len() - 1], (300.0, 10.0));
        // Every leg is axis-aligned: an orthogonal path, never a curve.
        assert!(corners
            .windows(2)
            .all(|q| (q[0].0 - q[1].0).abs() < EPS || (q[0].1 - q[1].1).abs() < EPS));
        assert!(work < DETOUR_TOTAL_WORK, "the detour is charged");
    }

    /// An edge between two ends of `w`, allowed nowhere special.
    fn info(a: EndRef, b: EndRef) -> Option<Info> {
        Some(Info {
            ends: (a, b),
            allowed: Vec::new(),
            blocked_frames: Vec::new(),
            back: false,
            sideways: false,
        })
    }

    /// Run [`spread`] over `routes` in `w` with no curves.
    fn spread_all(w: &World, infos: &[Option<Info>], routes: &mut [Option<Vec<Pt>>]) {
        let ends: Vec<(Pt, Pt)> = routes
            .iter()
            .map(|r| {
                r.as_ref()
                    .map_or(((0.0, 0.0), (0.0, 0.0)), |p| (p[0], p[p.len() - 1]))
            })
            .collect();
        let f = Fences {
            w,
            infos,
            ends: &ends,
            par: [Vec::new(), Vec::new()],
            par_at: [0, 1].map(|_| Buckets::new(std::iter::empty())),
            singles: true,
            routed: false,
        };
        for _ in 0..NUDGE_ROUNDS {
            let mut unlimited = usize::MAX;
            spread(&f, routes, &mut unlimited);
        }
    }

    #[test]
    fn spread_moves_a_leg_off_a_frame_border() {
        let frame = (100.0, 100.0, 300.0, 300.0);
        let (a, b) = ((0.0, 30.0, 20.0, 70.0), (420.0, 330.0, 440.0, 370.0));
        let w = bare(&[a, b], &[frame], (0.0, 0.0, 500.0, 500.0));
        // The middle leg runs 4px above the frame's top border.
        let path = vec![
            (20.0, 50.0),
            (60.0, 50.0),
            (60.0, 96.0),
            (400.0, 96.0),
            (400.0, 350.0),
            (420.0, 350.0),
        ];
        assert!(side_run(frame, &path, &[]) > TIGHT_RUN_MIN);
        let infos = vec![info(EndRef::Node(0), EndRef::Node(1))];
        let mut routes = vec![Some(path)];
        spread_all(&w, &infos, &mut routes);
        let moved = routes[0].as_ref().unwrap();
        assert!(moved[2].1 <= 100.0 - FRAME_SEP, "{moved:?}");
        assert_eq!(moved[2].1, moved[3].1, "the leg stays straight");
        assert_eq!(side_run(frame, moved, &[]), 0.0);
        // The ports stay where they are.
        assert_eq!((moved[0], moved[5]), ((20.0, 50.0), (420.0, 350.0)));
    }

    #[test]
    fn spread_separates_shared_channels_port_legs_included() {
        // Two nodes left, one right; both paths share the vertical leg at
        // x = 100, and their ports sit 4px apart on the left node.
        let (a, b) = ((0.0, 0.0, 20.0, 60.0), (280.0, 180.0, 300.0, 260.0));
        let w = bare(&[a, b], &[], (0.0, 0.0, 400.0, 400.0));
        let path = |y0: f64, y1: f64| vec![(20.0, y0), (100.0, y0), (100.0, y1), (280.0, y1)];
        let mut routes = vec![Some(path(26.0, 200.0)), Some(path(30.0, 240.0))];
        let infos = vec![
            info(EndRef::Node(0), EndRef::Node(1)),
            info(EndRef::Node(0), EndRef::Node(1)),
        ];
        spread_all(&w, &infos, &mut routes);
        let (p, q) = (routes[0].as_ref().unwrap(), routes[1].as_ref().unwrap());
        // The shared vertical legs are a full separation apart.
        assert!((p[1].0 - q[1].0).abs() >= EDGE_SEP, "{p:?} {q:?}");
        // The port legs slid their ports apart along the node's side,
        // and stayed on it.
        assert!((p[0].1 - q[0].1).abs() >= EDGE_SEP_MIN, "{p:?} {q:?}");
        assert!(p[0].0 == 20.0 && q[0].0 == 20.0);
        assert!([p[0].1, q[0].1]
            .iter()
            .all(|&y| (PORT_INSET..=60.0 - PORT_INSET).contains(&y)));
        // Every leg is still axis-aligned and ordered so they do not cross.
        for r in [p, q] {
            assert!(
                r.windows(2).all(|s| s[0].0 == s[1].0 || s[0].1 == s[1].1),
                "{r:?}"
            );
        }
        assert!((p[0].1 < q[0].1) == (p[1].0 > q[1].0), "{p:?} {q:?}");
    }

    #[test]
    fn a_stair_step_merges_into_one_leg() {
        let (a, b) = ((0.0, 0.0, 20.0, 40.0), (380.0, 0.0, 400.0, 40.0));
        let w = bare(&[a, b], &[], (0.0, 0.0, 400.0, 400.0));
        // Out at y = 20, a 10px step down at x = 150, in at y = 30.
        let path = vec![(20.0, 20.0), (150.0, 20.0), (150.0, 30.0), (380.0, 30.0)];
        let infos = vec![info(EndRef::Node(0), EndRef::Node(1))];
        let routes = vec![Some(path)];
        let f = Fences {
            w: &w,
            infos: &infos,
            ends: &[((20.0, 20.0), (380.0, 30.0))],
            par: [Vec::new(), Vec::new()],
            par_at: [0, 1].map(|_| Buckets::new(std::iter::empty())),
            singles: true,
            routed: false,
        };
        let merged = merge_jog(&f, &routes, 0, 0).expect("a step to merge");
        assert_eq!(merged.len(), 2, "{merged:?}");
        assert_eq!(merged[0].1, merged[1].1);
    }

    #[test]
    fn a_rerouted_end_takes_a_free_slot_on_its_side() {
        let n = (100.0, 100.0, 200.0, 140.0);
        let w = bare(&[n], &[], (0.0, 0.0, 400.0, 400.0));
        // Two other edges already end at the top side's middle.
        let taken = [
            (EndRef::Node(0), (150.0, 100.0), 1),
            (EndRef::Node(0), (150.0 + PORT_SPACING, 100.0), 2),
        ];
        let p = free_slot(&w, EndRef::Node(0), n, Side::Top, &taken, 0);
        assert_eq!(p.1, 100.0);
        assert!(
            taken.iter().all(|t| (t.1 .0 - p.0).abs() >= PORT_SPACING),
            "{p:?}"
        );
        // Its own end does not count.
        let p = free_slot(&w, EndRef::Node(0), n, Side::Top, &taken[..1], 1);
        assert_eq!(p, (150.0, 100.0));
    }

    /// Two frames 20 px apart (one frame separation and a half): a path
    /// threading the gap into the far frame runs through a narrow
    /// channel, one passing beside a single frame does not, and the
    /// narrow-channel rule keeps an original that only hits a frame or a
    /// chip over the threaded one once it rides a border or runs cramped
    /// more than the original.
    #[test]
    fn a_reroute_through_a_narrow_gap_trades_for_the_curve() {
        let (near, far) = ((100.0, 100.0, 300.0, 400.0), (320.0, 300.0, 500.0, 400.0));
        let (a, b) = ((150.0, 20.0, 250.0, 50.0), (350.0, 340.0, 450.0, 370.0));
        let w = bare(&[a, b], &[near, far], (0.0, 0.0, 600.0, 500.0));
        let info = info(EndRef::Node(0), EndRef::Node(1)).unwrap();
        // Down the gap at x = 310, into the far node's left side.
        let threaded = [(250.0, 35.0), (310.0, 35.0), (310.0, 355.0), (350.0, 355.0)];
        let gap = narrow_px(&w, &info, &threaded);
        assert!(gap > TIGHT_RUN_MIN, "{gap}");
        // Down the open side of the far frame instead.
        let open = [(250.0, 35.0), (540.0, 35.0), (540.0, 355.0), (450.0, 355.0)];
        assert_eq!(narrow_px(&w, &info, &open), 0.0);
        // The original only clips the far frame's title chip.
        let base = Score {
            chip: 1,
            ..Score::default()
        };
        let cand = w.score(&info, 0, &threaded, None);
        let squeeze = Squeeze {
            narrow: gap,
            border: border_px(&w.frames, &threaded),
            base_border: 0.0,
        };
        assert!(squeeze.border > 0.0 || cand.hug > 0, "{cand:?}");
        assert!(trades(&base, &cand, squeeze), "{cand:?}");
        // Not when the original hits a node, nor for a wide channel.
        assert!(!trades(&Score { node: 1, ..base }, &cand, squeeze));
        assert!(!trades(
            &base,
            &cand,
            Squeeze {
                narrow: 0.0,
                ..squeeze
            }
        ));
    }

    /// Two edges into a frame wedged 20 px beside a wider sibling (a
    /// synthetic stand-in, generic ids): the static scene keeps them off
    /// the gap between the frames, never riding a border or cramped
    /// beside each other there (G4 over G6, the narrow-channel rule).
    #[test]
    fn edges_into_a_wedged_frame_keep_out_of_the_gap() {
        let src = "flowchart TB
  subgraph g0[Lorem]
    subgraph g1[Ipsum]
      n1[dolor sit]
      n2[amet elit]
    end
    subgraph g2[Consectetur]
      n3[adipiscing lorem one]
      n4[sed do eiusmod two]
      n5[tempor ipsum three]
      n6[magna aliqua four]
    end
    subgraph g3[Aliqua]
      n7[incididunt ut labore et]
    end
  end
  n1 --> n2
  n2 --> n3
  n2 --> n5
  n1 -->|labore| n7
  n2 -->|magna| n7
";
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let frame = |id: &str| sc.clusters.iter().find(|c| c.id == id).expect("frame");
        let (wide, far) = (frame("g2"), frame("g3"));
        let gap = far.x - (wide.x + wide.w);
        assert!(
            gap > 0.0 && gap < 2.0 * FRAME_SEP,
            "frames {gap:.1} px apart"
        );
        let m = crate::quality::measure(&g, &sc);
        for d in &m.details {
            let into_far = d.starts_with("n1->n7 ") || d.starts_with("n2->n7 ");
            let beside = d.contains("tight n1->n7") || d.contains("tight n2->n7");
            assert!(!(into_far && (d.contains(" border ") || beside)), "{d}");
        }
    }

    #[test]
    fn chips_and_borders_count_in_the_score() {
        let frame = (100.0, 100.0, 300.0, 300.0);
        let mut w = bare(&[], &[frame], (0.0, 0.0, 400.0, 400.0));
        w.chips = vec![(106.0, 105.0, 160.0, 121.0)];
        let info = Info {
            ends: (EndRef::Frame(0), EndRef::Frame(0)),
            allowed: vec![0],
            blocked_frames: Vec::new(),
            back: false,
            sideways: true,
        };
        // Through the chip, then along the top border inside.
        let s = w.score(
            &info,
            0,
            &[(130.0, 50.0), (130.0, 110.0), (290.0, 110.0)],
            None,
        );
        assert_eq!(s.chip, 1);
        assert!(s.hard() > 0, "a chip hit breaks the edge");
        // A frame the edge ends on is its own: no hug.
        assert_eq!(s.hug, 0);
        let info = Info {
            ends: (EndRef::Node(0), EndRef::Node(0)),
            ..info
        };
        let mut w2 = bare(&[(0.0, 0.0, 10.0, 10.0)], &[frame], w.outer);
        w2.chips = w.chips.clone();
        let s = w2.score(
            &info,
            0,
            &[(200.0, 50.0), (200.0, 110.0), (290.0, 110.0)],
            None,
        );
        assert_eq!(s.chip, 0);
        assert!(s.hug >= 80, "{s:?}");
    }

    /// The runs [`convex_min`] and [`below`] find are exactly the
    /// samples a scan finds, and [`covered`] counts their union outside
    /// the skipped runs.
    #[test]
    fn distance_runs_match_a_sample_scan() {
        // Samples every 2 px along y = 0.
        let k = 100;
        let at = |q: usize| (2.0 * q as f64, 0.0);
        for (c, d) in [
            ((50.0, 5.0), (120.0, 5.0)),
            ((-30.0, 9.0), (60.0, 2.0)),
            ((150.0, -3.0), (260.0, 20.0)),
            ((80.0, 30.0), (90.0, 30.0)),
        ] {
            let f = |q: usize| seg_dist(at(q), c, d);
            let (m, fm) = convex_min(0, k - 1, &f).expect("non-empty");
            for r in [EDGE_SPACING, EDGE_SEP] {
                let scan: Vec<usize> = (0..k).filter(|&q| f(q) < r).collect();
                let run = if fm < r {
                    below(0, k - 1, m, r, &f)
                } else {
                    None
                };
                match run {
                    Some((lo, hi)) => assert_eq!(scan, (lo..=hi).collect::<Vec<_>>()),
                    None => assert!(scan.is_empty(), "{c:?}-{d:?} at {r}: {scan:?}"),
                }
            }
        }
        assert_eq!(
            covered(vec![(0, 9), (5, 14), (20, 24)], &[(3, 6), (22, 30)]),
            13
        );
        assert_eq!(covered(vec![(0, 4)], &[]), 5);
    }

    #[test]
    fn smoothing_rounds_corners_inside_the_legs() {
        let wps = smooth(
            &[(0.0, 0.0), (0.0, 12.0), (100.0, 12.0), (100.0, 40.0)],
            CORNER_R,
        );
        assert_eq!(wps.first(), Some(&(0.0, 0.0)));
        assert_eq!(wps.last(), Some(&(100.0, 40.0)));
        // Corner at (0, 12): legs 12 and 100 -> r = 6.
        assert!(
            wps.contains(&(0.0, 6.0)) && wps.contains(&(6.0, 12.0)),
            "{wps:?}"
        );
    }

    #[test]
    fn repair_never_adds_node_crossings_on_the_fixtures() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
        for name in ["back-edge", "fanout-frames", "wide-labels"] {
            let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).unwrap();
            let rep = measure_source(&src, false).unwrap();
            assert_eq!(rep.statik.edge_node_pairs, 0, "{name} static");
            assert_eq!(rep.routed.edge_node_pairs, 0, "{name} routed");
        }
    }
}
