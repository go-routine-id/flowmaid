//! Stage 1b of [`crate::edgepass::finish`]: every flowchart edge in one
//! idiom, orthogonal with rounded corners (report §2.1, B1, M2).
//!
//! The builders draw an edge between adjacent ranks as one cubic, a
//! long edge as a spline through its dummy chain, and the router only
//! redraws the edges it repairs as orthogonal paths: three idioms side
//! by side, an S wherever an edge's ends do not line up. Here, once the
//! ports stage has put every end on its side, each edge the flowchart
//! painter strokes with its default curve is redrawn on the axes:
//!
//! - an edge between two flow sides becomes a Z: straight out of its
//!   source side, across the gap between the ranks it joins, straight
//!   into its target side (a straight line when its ends line up); a
//!   long edge runs straight down each column its dummy chain holds and
//!   steps across in the gaps between them;
//! - an edge between two breadth sides (boxes side by side) a Z across
//!   the flow, and one from a breadth side to a flow side (a state note
//!   card's link) an L.
//!
//! Each step across a gap (a jog) is placed on its own track: the jogs
//! of one gap whose spans overlap are ordered so their turns do not
//! cross (libavoid's rule, as [`crate::router`]'s channel nudge orders
//! legs) and spaced [`EDGE_SEP`] apart round the gap's middle, clear of
//! the frames' borders and title chips they pass. The gaps are read off
//! the scene — the free bands along the flow between the rows of boxes
//! — so the static layout and a host's own centres ([`crate::scene::route`],
//! every drag frame) are drawn by one rule. The corners are rounded as
//! the router rounds its own paths ([`crate::router::smooth`]), so a
//! host's spline through the waypoints draws the same rounded path, and
//! `bezier` holds the chord through the first and last corner. A label
//! rides its Z's middle run (where the cubic's middle was).
//!
//! An edge with an explicit mermaid `curve` (`e1@{ curve: … }`,
//! `linkStyle … interpolate …`) keeps it; an edge a class or ER painter
//! strokes, a self-loop, a back-edge in its lane (orthogonal already),
//! a state note card's link (its elbow, pinned alike in every mode) and
//! an edge a drag keeps ([`Ctx::fixed`]) are left alone, as is an edge
//! whose ends face no way an orthogonal path of these shapes joins.

use crate::clearance::{EDGE_SEP, EDGE_SPACING, NODE_SEP};
use crate::edgepass::{end_rect, side_of, Ctx, EndRef, Mode, Side};
use crate::model::{Direction, Graph};
use crate::scene::{Scene, SceneEdge, ARROW_MARKER_PX};

type Pt = (f64, f64);
/// A box `(x0, y0, x1, y1)`, or in flow coordinates (breadth lo, hi, flow
/// lo, hi).
type Rect = (f64, f64, f64, f64);

/// How far past a separation a placed jog goes, so one placed at
/// exactly [`NODE_SEP`] / [`EDGE_SEP`] is clear of the harness's strict
/// thresholds whatever the float rounding (the router's `SEP_AIR`).
const SEP_AIR: f64 = 0.5;
/// Pitch (px) of the tracks of one gap's jogs: the edge separation the
/// router and its nudge keep between parallel legs.
pub(crate) const TRACK_PITCH: f64 = EDGE_SEP + SEP_AIR;
/// The least pitch (px) of a gap's tracks when the gap is too shallow
/// for [`TRACK_PITCH`]: two strokes rather than one on top of the other.
const TRACK_PITCH_MIN: f64 = EDGE_SPACING + SEP_AIR;
/// Room (px) a jog keeps from the row of boxes either side of its gap.
pub(crate) const ROW_CLEAR: f64 = NODE_SEP + SEP_AIR;
/// The least straight run (px) into an end under an arrowhead (or any
/// end glyph): the head and a corner's rounding, so the head sits on a
/// straight stretch square to its side.
pub(crate) const HEAD_LEG: f64 = ARROW_MARKER_PX + crate::router::CORNER_R;
/// The least straight run (px) out of a bare end before its first
/// corner.
const BARE_LEG: f64 = crate::router::CORNER_R / 2.0;
/// Room (px) a jog keeps from a frame's border it passes along: the
/// router's frame ring.
const BORDER_CLEAR: f64 = crate::clearance::FRAME_SEP / 2.0 + SEP_AIR;
/// [`BORDER_CLEAR`] in a block layout (B12 G2): a frame's whole
/// clearance and air, past the router's frame band
/// ([`crate::clearance::CLEAR_GAP`]) even where the router judges the
/// path simplified (a rounded corner cut short tilts the run beside it
/// a px or two towards the border), so a track the layout made room
/// for is not a border ride the repair then reroutes. At
/// [`BORDER_CLEAR`] the first track of a fan stepping under a block ran
/// 7.5 px under its border for 570 px, and the repair took it off,
/// across the rest of the fan (dense-td).
const BLOCK_BORDER_CLEAR: f64 = crate::clearance::FRAME_SEP + SEP_AIR;
/// Room (px) a jog keeps from a frame's title chip — and, in a block
/// layout, a port from a chip's shadow ([`crate::ports`]): the room the
/// router asks for and a little air, so a stroke placed by it does not
/// sit exactly on the router's limit.
pub(crate) const CHIP_CLEAR: f64 = crate::clearance::CHIP_SEP + SEP_AIR;
/// Room (px) a step across keeps from the label of its own edge riding
/// the run below it (the label stage's own gap).
const LABEL_SEP: f64 = crate::labels::LABEL_GAP;
/// Air (px) a labelled twin's run keeps beyond its label's half extent
/// from its twin's line: a stroke's pad and the edge separation (the
/// builders' `PARALLEL_LABEL_AIR`).
const TWIN_AIR: f64 = crate::labels::STROKE_PAD + EDGE_SEP;
/// Two ends this close (px) across their run are drawn straight: no
/// jog.
const ALIGNED: f64 = 0.5;
/// Float tolerance.
const EPS: f64 = 1e-6;

/// A point in flow coordinates: (breadth, along the flow), the flow
/// running towards larger values (see [`to_flow`]).
fn to_flow(dir: Direction, p: Pt) -> Pt {
    match dir {
        Direction::TD => p,
        Direction::BT => (p.0, -p.1),
        Direction::LR => (p.1, p.0),
        Direction::RL => (p.1, -p.0),
    }
}

/// Inverse of [`to_flow`].
fn from_flow(dir: Direction, p: Pt) -> Pt {
    match dir {
        Direction::TD => p,
        Direction::BT => (p.0, -p.1),
        Direction::LR => (p.1, p.0),
        Direction::RL => (-p.1, p.0),
    }
}

/// A rect `(x0, y0, x1, y1)` in flow coordinates as (breadth lo, breadth
/// hi, flow lo, flow hi).
fn flow_rect(dir: Direction, r: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    let (a, b) = (to_flow(dir, (r.0, r.1)), to_flow(dir, (r.2, r.3)));
    (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1))
}

/// The outward normal of a side in flow coordinates.
fn flow_normal(dir: Direction, s: Side) -> Pt {
    to_flow(dir, crate::edgepass::normal(s))
}

/// One step across a gap of one edge, waiting for its track.
struct Jog {
    edge: usize,
    /// Which gap (index into the gaps, or past them for a run with no
    /// gap between its ends: its own).
    gap: usize,
    /// Breadth where the jog meets the run from the low flow side, and
    /// where it meets the run on to the high flow side.
    lo_at: f64,
    hi_at: f64,
    /// The span along the flow it may take, and where it would go.
    room: (f64, f64),
    want: f64,
    /// Where it was placed.
    at: f64,
    /// It steps across between its source and its edge's label, which
    /// rides the run on into the target.
    above_label: bool,
    /// Its room and wanted place under the label instead (the step's
    /// middle), for a channel whose tracks cannot all nest above their
    /// labels in the order that keeps them from crossing.
    plain: ((f64, f64), f64),
    /// Half its edge's label's extent along the flow when the label
    /// sits on this step's run (a plain labelled Z's only step drawn
    /// under its label), else 0: the tracks beside it keep that, and the
    /// label's gap, off its run.
    label_half: f64,
    /// Where along the flow the runs either side of it start: the run
    /// into it from the low flow side at `lo_at`, the run on from it to
    /// the high side at `hi_at` (what [`uncross`] counts crossings on).
    runs: (f64, f64),
}

impl Jog {
    fn span(&self) -> (f64, f64) {
        (self.lo_at.min(self.hi_at), self.lo_at.max(self.hi_at))
    }
}

/// One corner of a path being built: a fixed point, or the start or
/// end of jog `k` at the breadth given (its flow coordinate is the
/// jog's track).
#[derive(Clone, Copy)]
enum Corner {
    At(Pt),
    Jog(usize, f64),
}

/// Redraw every eligible edge orthogonally (see the module docs). Run
/// after [`crate::ports::assign`], before [`crate::router::repair`].
/// Sets [`Ctx::chain`] for each edge drawn down a dummy chain (a long
/// edge's channel), which the router accepts before the others as it
/// did the splines, and [`Ctx::shaped`] for each labelled twin's bulge.
pub(crate) fn orthogonalize(g: &Graph, sc: &mut Scene, cx: &mut Ctx, mode: Mode) {
    // Past [`crate::router::ROUTE_MAX_EDGES`] the router and its nudge
    // leave a drawing as built, so the builders' curves stay too: drawn
    // on the axes there, every edge's rounded corners were sampled into
    // waypoints no later stage spaces, `route()` of a 531-edge chart
    // took 25-30 ms instead of 2.4-2.9 and picking walked 4.6x the
    // points (B10 review r3 #8).
    if g.edge_paint_width.is_some()
        || sc.edges.is_empty()
        || sc.edges.len() > crate::router::ROUTE_MAX_EDGES
    {
        return;
    }
    if mode != Mode::Static {
        ortho_pass(g, sc, cx, mode);
        return;
    }
    // (What a pass changes of an edge: its curve, its waypoints and its
    // label's spot.)
    let snap = |edges: &[SceneEdge]| -> Vec<Geo> {
        edges
            .iter()
            .map(|e| (e.bezier, e.waypoints.clone(), e.label.as_ref().map(|l| l.1)))
            .collect()
    };
    let before = snap(&sc.edges);
    ortho_pass(g, sc, cx, mode);
    // The ports stage orders a side's ends by where their edges head,
    // which suits the builders' curves; drawn on the axes, two ends of a
    // side whose paths turn the same way cross twice when the one turning
    // later lies outside (y2_state_nested: `Off --> decide`'s L round the
    // outside of `Off --> On`'s Z), and an end beyond a note card's link
    // on its side runs across the link's elbow. Such ends move (two
    // swap places, one beyond a link goes to the link's other side) and
    // the stage draws again, kept only when the edges it changed cross
    // fewer lines.
    let moves = untangle(sc, cx, &before);
    if moves.is_empty() {
        return;
    }
    let first = snap(&sc.edges);
    restore(&mut sc.edges, before);
    let kept = (cx.chain.clone(), cx.shaped.clone());
    for (i, src, to) in moves {
        shift_end(&mut sc.edges[i], src, to);
    }
    let lines_first: Vec<Vec<Pt>> = first
        .iter()
        .zip(&sc.edges)
        .map(|(f, e)| {
            let mut e = e.clone();
            (e.bezier, e.waypoints) = (f.0, f.1.clone());
            line_of(&e)
        })
        .collect();
    ortho_pass(g, sc, cx, mode);
    let changed: Vec<usize> = (0..sc.edges.len())
        .filter(|&i| {
            let (a, b) = (&sc.edges[i], &first[i]);
            a.waypoints != b.1 || a.bezier != b.0
        })
        .collect();
    let lines_now: Vec<Vec<Pt>> = sc.edges.iter().map(line_of).collect();
    if crossings_of(&lines_now, &changed) >= crossings_of(&lines_first, &changed) {
        restore(&mut sc.edges, first);
        (cx.chain, cx.shaped) = kept;
    }
}

/// Whether waypoints `w` run on the axes (as
/// [`crate::router::ortho_corners`] finds a path).
fn on_axes(w: &[Pt]) -> bool {
    w.len() >= 2
        && w.windows(2)
            .all(|q| (q[0].0 - q[1].0).abs() < EPS || (q[0].1 - q[1].1).abs() < EPS)
}

/// An edge's geometry as a pass of [`orthogonalize`] may change it:
/// curve, waypoints, label spot.
type Geo = ([Pt; 4], Vec<Pt>, Option<Pt>);

/// Put back what [`orthogonalize`] snapped of every edge.
fn restore(edges: &mut [SceneEdge], geo: Vec<Geo>) {
    for (e, (b, w, l)) in edges.iter_mut().zip(geo) {
        (e.bezier, e.waypoints) = (b, w);
        if let (Some(lab), Some(at)) = (e.label.as_mut(), l) {
            lab.1 = at;
        }
    }
}

/// Move the start (`src`) or end of a path the stage has yet to draw to
/// `to`, with the handle or stub next to it, so the stage plans it from
/// there.
fn shift_end(e: &mut SceneEdge, src: bool, to: Pt) {
    let (pi, ci) = if src { (0, 1) } else { (3, 2) };
    let old = e.bezier[pi];
    let d = (to.0 - old.0, to.1 - old.1);
    e.bezier[pi] = to;
    e.bezier[ci] = (e.bezier[ci].0 + d.0, e.bezier[ci].1 + d.1);
    if let Some(w) = if src {
        e.waypoints.first_mut()
    } else {
        e.waypoints.last_mut()
    } {
        *w = to;
    }
}

/// An edge's line as drawn, by its corners where it runs on the axes,
/// for counting crossings.
fn line_of(e: &SceneEdge) -> Vec<Pt> {
    match crate::router::ortho_corners(e) {
        Some(c) => c,
        None if e.waypoints.len() >= 2 => e.waypoints.clone(),
        None => crate::scene::sample_cubic(&e.bezier, 12),
    }
}

/// How many times two lines properly cross.
fn lines_cross(a: &[Pt], b: &[Pt]) -> usize {
    a.windows(2)
        .map(|u| {
            b.windows(2)
                .filter(|v| segs_cross(u[0], u[1], v[0], v[1]))
                .count()
        })
        .sum()
}

/// The crossings of the lines in `which` with every other line (each
/// pair once).
fn crossings_of(lines: &[Vec<Pt>], which: &[usize]) -> usize {
    let mut n = 0;
    for (k, &i) in which.iter().enumerate() {
        for j in 0..lines.len() {
            if j == i || which[..k].contains(&j) {
                continue;
            }
            n += lines_cross(&lines[i], &lines[j]);
        }
    }
    n
}

/// The end moves [`orthogonalize`] tries on the paths of its first
/// pass (`sc`; `before` holds what that pass started from), as (edge,
/// its start?, new point): per side of a node or frame, two ends whose
/// paths cross each other twice swap places, and an end whose path
/// crosses a note card's link out of the same side, lying on the side
/// of the link's end the card is, goes to the mirror place on the link's
/// other side where that stays on the side, a port's pitch from every
/// other end. Only ends of paths the stage itself planned move (never a
/// drag's kept ones, a lane's, an authored curve's or a note link's).
fn untangle(sc: &Scene, cx: &Ctx, before: &[Geo]) -> Vec<(usize, bool, Pt)> {
    struct End {
        edge: usize,
        src: bool,
        at: Pt,
        movable: bool,
        note: bool,
    }
    let ends_at = |e: &SceneEdge| {
        if e.waypoints.len() >= 2 {
            (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
        } else {
            (e.bezier[0], e.bezier[3])
        }
    };
    let flat = |end: EndRef| match end {
        EndRef::Frame(_) => true,
        EndRef::Node(v) => !matches!(
            sc.nodes[v].shape,
            crate::model::Shape::Diamond
                | crate::model::Shape::Circle
                | crate::model::Shape::DoubleCircle
                | crate::model::Shape::StateStart
                | crate::model::Shape::StateEnd
                | crate::model::Shape::ForkBar
        ),
    };
    let side = |end: EndRef, p: Pt| match end {
        EndRef::Node(v) => crate::ports::outline_side(sc.nodes[v].shape, end_rect(sc, end), p),
        EndRef::Frame(_) => side_of(end_rect(sc, end), p),
    };
    let card = |end: EndRef| match end {
        EndRef::Node(v) => {
            let n = &sc.nodes[v];
            crate::style::is_note_card(&n.id, n.shape, &n.style)
        }
        EndRef::Frame(_) => false,
    };
    let mut groups: Vec<((EndRef, Side), Vec<End>)> = Vec::new();
    let mut any_note = false;
    for (i, e) in sc.edges.iter().enumerate() {
        let Some((from, to)) = cx.ends.get(i).copied().flatten() else {
            continue;
        };
        if from == to || !cx.drawn.get(i).copied().unwrap_or(false) || e.paint_width.is_some() {
            continue;
        }
        let note = crate::ports::note_link(sc, cx, i);
        let movable = !note
            && !cx.fixed.get(i).copied().unwrap_or(false)
            && !cx.held.get(i).copied().unwrap_or(false)
            && e.meta.curve.is_none()
            && before.get(i).is_some_and(|b| !on_axes(&b.1));
        any_note |= note;
        let (p0, p3) = ends_at(e);
        for (src, end, at) in [(true, from, p0), (false, to, p3)] {
            if note && card(end) {
                continue;
            }
            let key = (end, side(end, at));
            let item = End {
                edge: i,
                src,
                at,
                movable,
                note,
            };
            match groups.iter_mut().find(|g| g.0 == key) {
                Some(g) => g.1.push(item),
                None => groups.push((key, vec![item])),
            }
        }
    }
    let mut moves: Vec<(usize, bool, Pt)> = Vec::new();
    let mut lines: Vec<Option<Vec<Pt>>> = vec![None; sc.edges.len()];
    let mut line = |i: usize| -> Vec<Pt> {
        lines[i]
            .get_or_insert_with(|| line_of(&sc.edges[i]))
            .clone()
    };
    let tangent = |s: Side, p: Pt| match s {
        Side::Top | Side::Bottom => p.0,
        Side::Left | Side::Right => p.1,
    };
    let along = |s: Side, p: Pt, t: f64| match s {
        Side::Top | Side::Bottom => (t, p.1),
        Side::Left | Side::Right => (p.0, t),
    };
    for ((end, s), ends) in &groups {
        if ends.len() < 2 {
            continue;
        }
        let taken = |moves: &[(usize, bool, Pt)], e: &End| {
            moves.iter().any(|m| m.0 == e.edge && m.1 == e.src)
        };
        // Two of the stage's paths crossing each other twice: swapped.
        for a in 0..ends.len() {
            for b in a + 1..ends.len() {
                let (x, y) = (&ends[a], &ends[b]);
                if !x.movable || !y.movable || x.edge == y.edge {
                    continue;
                }
                if taken(&moves, x) || taken(&moves, y) {
                    continue;
                }
                if lines_cross(&line(x.edge), &line(y.edge)) >= 2 {
                    moves.push((x.edge, x.src, y.at));
                    moves.push((y.edge, y.src, x.at));
                }
            }
        }
        if !any_note || !flat(*end) {
            continue;
        }
        // An end beyond a note card's link: to the link's other side.
        let r = end_rect(sc, *end);
        let (lo, hi) = match s {
            Side::Top | Side::Bottom => (r.0, r.2),
            Side::Left | Side::Right => (r.1, r.3),
        };
        let inset = crate::ports::PORT_INSET;
        for pin in ends.iter().filter(|e| e.note) {
            let l = line(pin.edge);
            let far = if pin.src { l[l.len() - 1] } else { l[0] };
            let t0 = tangent(*s, pin.at);
            let d = tangent(*s, far) - t0;
            if d.abs() < 1.0 {
                continue;
            }
            for e in ends.iter().filter(|e| e.movable) {
                let t = tangent(*s, e.at);
                if (t - t0) * d <= 0.0 || taken(&moves, e) || lines_cross(&line(e.edge), &l) == 0 {
                    continue;
                }
                let m = 2.0 * t0 - t;
                let clear = ends.iter().all(|o| {
                    std::ptr::eq(o, e)
                        || (tangent(*s, o.at) - m).abs() >= crate::ports::PORT_SPACING - EPS
                });
                if m >= lo + inset - EPS && m <= hi - inset + EPS && clear {
                    moves.push((e.edge, e.src, along(*s, e.at, m)));
                }
            }
        }
    }
    moves
}

/// One pass of [`orthogonalize`] over the scene as the ports stage left
/// it (or with the ends [`untangle`] moved).
fn ortho_pass(g: &Graph, sc: &mut Scene, cx: &mut Ctx, mode: Mode) {
    let mut chain = vec![false; sc.edges.len()];
    let dir = g.direction;
    // The rows of boxes along the flow, and the free gaps between them.
    let mut rows: Vec<(f64, f64)> = sc
        .nodes
        .iter()
        .map(|n| {
            let r = flow_rect(
                dir,
                (
                    n.x - n.w / 2.0,
                    n.y - n.h / 2.0,
                    n.x + n.w / 2.0,
                    n.y + n.h / 2.0,
                ),
            );
            (r.2, r.3)
        })
        .filter(|r| r.0.is_finite() && r.1.is_finite())
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut merged: Vec<(f64, f64)> = Vec::with_capacity(rows.len());
    for r in rows {
        match merged.last_mut() {
            Some(m) if r.0 <= m.1 => m.1 = m.1.max(r.1),
            _ => merged.push(r),
        }
    }
    let gaps: Vec<(f64, f64)> = merged.windows(2).map(|w| (w[0].1, w[1].0)).collect();
    // Boxes in flow coordinates, to judge which gap a long run steps in.
    let boxes: Vec<(f64, f64, f64, f64)> = sc
        .nodes
        .iter()
        .map(|n| {
            flow_rect(
                dir,
                (
                    n.x - n.w / 2.0,
                    n.y - n.h / 2.0,
                    n.x + n.w / 2.0,
                    n.y + n.h / 2.0,
                ),
            )
        })
        .collect();
    // What a jog keeps clear of: the frames' borders across the flow and
    // their title chips (breadth span, flow span).
    let mut walls: Vec<Wall> = Vec::new();
    if !sc.clusters.is_empty() {
        let right = sc.origin.0 + sc.width;
        let clear = if cx.blocks {
            BLOCK_BORDER_CLEAR
        } else {
            BORDER_CLEAR
        };
        for (ci, c) in sc.clusters.iter().enumerate() {
            let r = flow_rect(dir, (c.x, c.y, c.x + c.w, c.y + c.h));
            for l in [r.2, r.3] {
                walls.push(((r.0, r.1), (l - clear, l + clear)));
            }
            // A block layout's chip reaches its title's ink, as every
            // stage of its static scene measures it
            // ([`crate::scene::title_head_measured`]).
            let h = flow_rect(
                dir,
                crate::scene::title_head_measured(&sc.clusters, ci, right, cx.blocks),
            );
            walls.push(((h.0, h.1), (h.2 - CHIP_CLEAR, h.3 + CHIP_CLEAR)));
        }
    }

    let frames: Vec<(f64, f64, f64, f64)> = sc
        .clusters
        .iter()
        .map(|c| flow_rect(dir, (c.x, c.y, c.x + c.w, c.y + c.h)))
        .collect();
    let load = std::cell::RefCell::new(vec![Vec::new(); gaps.len()]);
    // Every drawn edge's line (flow coordinates), simplified to its
    // corners where it has them: what a long run's gap is judged by
    // (`plan_jog`): a long run with no dummy columns to follow (every
    // long edge in `route()`) chooses its gap.
    let lines: Vec<Vec<Pt>> = if gaps.len() >= 2 {
        sc.edges
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if !cx.drawn.get(i).copied().unwrap_or(false) {
                    return Vec::new();
                }
                let pts = match crate::router::ortho_corners(e) {
                    Some(c) => c,
                    None if e.waypoints.len() >= 2 => e.waypoints.clone(),
                    None => crate::scene::sample_cubic(&e.bezier, 8),
                };
                pts.into_iter().map(|p| to_flow(dir, p)).collect()
            })
            .collect()
    } else {
        Vec::new()
    };
    let stubs = breadth_stubs(sc, cx, dir, &boxes);
    let end_stubs = breadth_ends(sc, cx, dir, &boxes);
    let titles = TitleSides::of(sc, cx, dir, &frames);
    let mut jogs: Vec<Jog> = Vec::new();
    let mut plans: Vec<(usize, Vec<Corner>)> = Vec::new();
    // The edges between one pair of ends, either way round (twins): a
    // labelled twin's run keeps its label off the other's line.
    let pair_of = |i: usize| {
        let (a, b) = cx.ends.get(i).copied().flatten()?;
        let ok = cx.drawn.get(i).copied().unwrap_or(false) && a != b;
        ok.then_some(if a <= b { (a, b) } else { (b, a) })
    };
    let pairs: Vec<Option<(EndRef, EndRef)>> = (0..sc.edges.len()).map(pair_of).collect();
    let mut bulged = vec![false; sc.edges.len()];
    // Per labelled twin drawn plain where its bulge would cross other
    // ends' legs: the breadth its label's middle keeps beyond (towards
    // `.1`), clear of its twin's line, and the twin (`.2`).
    let mut beside: Vec<Option<(f64, f64, usize)>> = vec![None; sc.edges.len()];
    // Every long edge's dummy columns as the layout placed them, and
    // every label box (flow coordinates): what a column kept apart from
    // as it is straightened ([`straighten`]).
    let columns: Vec<(usize, Pt)> = if mode == Mode::Static {
        sc.edges
            .iter()
            .enumerate()
            .filter(|(i, e)| {
                e.waypoints.len() >= 3
                    && cx.drawn.get(*i).copied().unwrap_or(false)
                    && crate::router::ortho_corners(e).is_none()
            })
            .flat_map(|(i, e)| {
                e.waypoints[1..e.waypoints.len() - 1]
                    .iter()
                    .map(move |&p| (i, to_flow(dir, p)))
            })
            .collect()
    } else {
        Vec::new()
    };
    // The runs along the flow of the paths on the axes already (a
    // back-edge's lane above all): (breadth, flow span). A column keeps
    // off them as it does off another column, or the lane the layout
    // planned beside a dummy would run on top of the straightened chain.
    let lane_runs: Vec<(f64, (f64, f64))> = if columns.is_empty() {
        Vec::new()
    } else {
        sc.edges
            .iter()
            .filter_map(crate::router::ortho_corners)
            .flat_map(|c| {
                c.windows(2)
                    .filter_map(|w| {
                        let (a, b) = (to_flow(dir, w[0]), to_flow(dir, w[1]));
                        ((a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() > EPS)
                            .then_some((a.0, (a.1.min(b.1), a.1.max(b.1))))
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let loop_boxes: Vec<(usize, Rect)> = if columns.is_empty() {
        Vec::new()
    } else {
        sc.edges
            .iter()
            .filter(|e| e.from == e.to)
            .filter_map(|e| {
                let v = sc.nodes.iter().position(|n| n.id == e.from)?;
                let pts = crate::scene::edge_polyline(e);
                let (mut a, mut b) = (
                    (f64::INFINITY, f64::INFINITY),
                    (f64::NEG_INFINITY, f64::NEG_INFINITY),
                );
                for p in pts {
                    a = (a.0.min(p.0), a.1.min(p.1));
                    b = (b.0.max(p.0), b.1.max(p.1));
                }
                a.0.is_finite()
                    .then(|| (v, flow_rect(dir, (a.0, a.1, b.0, b.1))))
            })
            .collect()
    };
    let label_boxes: Vec<(usize, Rect)> = if columns.is_empty() {
        Vec::new()
    } else {
        sc.edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let (_, c, w) = e.label.as_ref()?;
                let h = crate::layout::LABEL_BOX_H / 2.0;
                Some((
                    i,
                    flow_rect(dir, (c.0 - w / 2.0, c.1 - h, c.0 + w / 2.0, c.1 + h)),
                ))
            })
            .collect()
    };
    // The steps of the edges this stage leaves as they are (a drag's kept
    // edges, lanes, paths on the axes already): (along the flow, span
    // across it). A new step keeps a track's pitch off them.
    let mut kept: Vec<(f64, (f64, f64))> = Vec::new();
    let mut kept_lines: Vec<Vec<Pt>> = Vec::new();
    for i in 0..sc.edges.len() {
        let e = &sc.edges[i];
        let Some((from, to)) = cx.ends.get(i).copied().flatten() else {
            continue;
        };
        let steps_of = |e: &SceneEdge, kept: &mut Vec<(f64, (f64, f64))>| {
            if e.waypoints.len() < 2 || !cx.drawn.get(i).copied().unwrap_or(false) {
                return;
            }
            for w in e.waypoints.windows(2) {
                let (a, b) = (to_flow(dir, w[0]), to_flow(dir, w[1]));
                if (a.1 - b.1).abs() < EPS && (a.0 - b.0).abs() > EPS {
                    kept.push((a.1, (a.0.min(b.0), a.0.max(b.0))));
                }
            }
        };
        // A state note card's link keeps the elbow its builder drew
        // (`note_elbow`): its ends are pins to the ports stage and it is
        // drawn alike in `scene()`, `route()` and every drag frame, which
        // handing it to the router's stages as a path would not keep.
        if !cx.drawn.get(i).copied().unwrap_or(false)
            || cx.fixed.get(i).copied().unwrap_or(false)
            || e.meta.curve.is_some()
            || from == to
            || crate::ports::note_link(sc, cx, i)
        {
            steps_of(e, &mut kept);
            // A curve this stage leaves (a note card's link, an author's
            // `curve:`): the steps keep from crossing it where their room
            // allows ([`uncross`]).
            if from != to
                && cx.drawn.get(i).copied().unwrap_or(false)
                && crate::router::ortho_corners(e).is_none()
            {
                kept_lines.push(
                    crate::scene::edge_polyline(e)
                        .into_iter()
                        .map(|p| to_flow(dir, p))
                        .collect(),
                );
            }
            continue;
        }
        let is_chain = e.waypoints.len() >= 3;
        if let Some(c) = (!e.waypoints.is_empty())
            .then(|| crate::router::ortho_corners(e))
            .flatten()
        {
            // A lane, or a path on the axes already: its bare corners
            // rounded (a channel of `route()` comes as bare corners, which
            // a spline through them draws as a slack S). One rounded
            // already keeps its points, so a second pass changes nothing.
            let wps = crate::router::smooth(&c, crate::router::CORNER_R);
            if c.len() >= 3 && c.len() == e.waypoints.len() && wps != e.waypoints {
                sc.edges[i].waypoints = wps;
            }
            steps_of(&sc.edges[i], &mut kept);
            continue;
        }
        let (p0, p3) = if e.waypoints.len() >= 2 {
            (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
        } else {
            (e.bezier[0], e.bezier[3])
        };
        let side = |end: EndRef, p: Pt| match end {
            EndRef::Node(v) => crate::ports::outline_side(sc.nodes[v].shape, end_rect(sc, end), p),
            EndRef::Frame(_) => side_of(end_rect(sc, end), p),
        };
        // An end on the border of a frame that holds the other end (a
        // state composite's edge to one of its own states) leaves or
        // enters the border from inside: its side faces in.
        let facing = |end: EndRef, other: EndRef, p: Pt| {
            let n = flow_normal(dir, side(end, p));
            if crate::edgepass::holds_other(cx, end, other) {
                (-n.0, -n.1)
            } else {
                n
            }
        };
        let (n0, n3) = (facing(from, to, p0), facing(to, from, p3));
        let (mut f0, mut f3) = (to_flow(dir, p0), to_flow(dir, p3));
        // A fork / join bar meets each branch right where the branch lies
        // (`scene::toward`): a branch a few px off its bar's port drops
        // straight from the bar instead of stepping.
        let bar = |end: EndRef| match end {
            EndRef::Node(v) if sc.nodes[v].shape == crate::model::Shape::ForkBar => {
                Some(flow_rect(dir, end_rect(sc, end)))
            }
            _ => None,
        };
        if n0.0 == 0.0 && n3.0 == 0.0 && (f0.0 - f3.0).abs() < 2.0 * crate::router::CORNER_R {
            let inset = crate::ports::PORT_INSET;
            if bar(from).is_some_and(|r| f3.0 > r.0 + inset && f3.0 < r.1 - inset) {
                f0.0 = f3.0;
            } else if bar(to).is_some_and(|r| f0.0 > r.0 + inset && f0.0 < r.1 - inset) {
                f3.0 = f0.0;
            }
        }
        let glyph = (
            e.ends.start_marker(e.kind).is_some(),
            e.ends.end_marker(e.kind).is_some(),
        );
        let ends_of = |end: EndRef| match end {
            EndRef::Node(v) => Some(v),
            EndRef::Frame(_) => None,
        };
        let own = [ends_of(from), ends_of(to)];
        // A static long edge's dummy chain holds the columns the layout
        // kept free for it; `route()`'s channel between two frames only
        // keeps its step off them, which the gap choice does for every
        // step (and spread over the gaps between, instead of all in the
        // one past the first frame).
        let cols: Vec<Pt> = if is_chain && mode == Mode::Static {
            let cols: Vec<Pt> = e.waypoints[1..e.waypoints.len() - 1]
                .iter()
                .map(|&p| to_flow(dir, p))
                .collect();
            if e.label.is_some() {
                // (A labelled edge's label keeps the slot its dummy
                // reserved, its column with it.)
                cols
            } else {
                let room = ColumnRoom {
                    rows: &merged,
                    boxes: &boxes,
                    loops: &loop_boxes,
                    frames: &frames,
                    columns: &columns,
                    runs: &lane_runs,
                    labels: &label_boxes,
                    own: [ends_of(from), ends_of(to)],
                    edge: i,
                };
                // A column whose run crosses a block's title side keeps
                // the place the layout gave it past the chip and the
                // columns of the block's displaced entries (X2.2, X2.5).
                // Straightened towards its neighbours it moved back into
                // that room (blocks/nested-chain: `a -> c2` 1 px past the
                // chip, `b2 -> c` displaced past it and across it).
                let pts: Vec<Pt> = std::iter::once(f0)
                    .chain(cols.iter().copied())
                    .chain(std::iter::once(f3))
                    .collect();
                let pinned: Vec<bool> = (1..pts.len() - 1)
                    .map(|k| {
                        !titles.all_crossings(&pts[k - 1..=k]).is_empty()
                            || !titles.all_crossings(&pts[k..=k + 1]).is_empty()
                    })
                    .collect();
                straighten(&room, f0.0, &cols, f3.0, &pinned)
            }
        } else {
            Vec::new()
        };
        // Clear of the title chips whose sides it crosses (B12 2.8).
        let cols = titles.clear(i, f0, cols, f3);
        let mut home: Vec<usize> = Vec::new();
        for end in [from, to] {
            let fr = match end {
                EndRef::Node(v) => cx.node_frames.get(v),
                EndRef::Frame(c) => cx.frame_anc.get(c),
            };
            home.extend(fr.into_iter().flatten().copied());
        }
        let head_leg = HEAD_LEG.max(crate::scene::edge_head_len(e) + crate::router::CORNER_R);
        let ctx = Along {
            gaps: &gaps,
            boxes: &boxes,
            walls: &walls,
            frames: &frames,
            home: &home,
            blocks: cx.blocks,
            axis_ends: cx.axis_ends,
            load: &load,
            own,
            head_leg,
            lines: &lines,
            titles: &titles,
        };
        // The label's spot along the flow and its half extent there.
        let label = e.label.as_ref().map(|(_, c, w)| {
            let half = match dir {
                Direction::TD | Direction::BT => crate::layout::LABEL_BOX_H / 2.0,
                Direction::LR | Direction::RL => w / 2.0,
            };
            (to_flow(dir, *c).1, half)
        });
        // A labelled twin bulges clear of its twin's line.
        if let (Some((_, _, w)), Some(key)) = (&e.label, pairs[i]) {
            let nearest = (0..sc.edges.len())
                .filter(|&j| j != i && pairs[j] == Some(key))
                .map(|j| {
                    let f = &sc.edges[j];
                    let (a, b) = if f.waypoints.len() >= 2 {
                        (f.waypoints[0], f.waypoints[f.waypoints.len() - 1])
                    } else {
                        (f.bezier[0], f.bezier[3])
                    };
                    (j, (to_flow(dir, a).0 + to_flow(dir, b).0) / 2.0)
                })
                .min_by(|x, y| {
                    let me = (f0.0 + f3.0) / 2.0;
                    (x.1 - me).abs().total_cmp(&(y.1 - me).abs())
                });
            let twin = nearest.map(|t| t.1);
            let across = match dir {
                Direction::TD | Direction::BT => w / 2.0,
                Direction::LR | Direction::RL => crate::layout::LABEL_BOX_H / 2.0,
            };
            if let (Some(tb), true) = (twin, cols.is_empty()) {
                if let Some(plan) =
                    twin_bulge((f0, n0, glyph.0), (f3, n3, glyph.1), tb, across, head_leg)
                {
                    // The bulge steps out across the ends beside its own
                    // on either side it leaves or enters, and crossed
                    // each of their legs (stress's thick `A ==> D` at
                    // Source's bottom, wide-labels' `health probe` at
                    // Client's flank). With such an end in its way the
                    // twin is drawn plain instead, its label set beside
                    // its run off its twin's line.
                    let run = match plan[2] {
                        Corner::At(p) => p.0,
                        Corner::Jog(..) => f0.0,
                    };
                    let swept = |end: EndRef, p: Pt, n: Pt| {
                        (0..sc.edges.len()).filter(|&j| j != i).any(|j| {
                            let Some((a, b)) = cx.ends.get(j).copied().flatten() else {
                                return false;
                            };
                            let f = &sc.edges[j];
                            let (q0, q3) = if f.waypoints.len() >= 2 {
                                (f.waypoints[0], f.waypoints[f.waypoints.len() - 1])
                            } else {
                                (f.bezier[0], f.bezier[3])
                            };
                            [(a, q0), (b, q3)].into_iter().any(|(at, q)| {
                                let q = to_flow(dir, q);
                                at == end
                                    && flow_normal(dir, side(end, from_flow(dir, q))) == n
                                    && (q.0 - p.0) * (run - p.0) > EPS
                                    && (q.0 - run).abs() + (q.0 - p.0).abs()
                                        <= (run - p.0).abs() + EPS
                            })
                        })
                    };
                    // Nor does it leave a frame holding both ends: its
                    // run kept a corner's two roundings off the ends, and
                    // in a narrow composite that put it on the frame's
                    // border, its label outside the frame (x3_state's
                    // `Saving --> Dirty : conflict` 2 px past `Editing`).
                    let held = |end: EndRef| match end {
                        EndRef::Node(v) => cx.node_frames.get(v).cloned().unwrap_or_default(),
                        EndRef::Frame(c) => cx.frame_anc.get(c).cloned().unwrap_or_default(),
                    };
                    let (hf, ht) = (held(from), held(to));
                    let inside = hf.iter().filter(|k| ht.contains(k)).all(|&k| {
                        frames.get(k).map_or(true, |r| {
                            run - across > r.0 + BORDER_CLEAR && run + across < r.1 - BORDER_CLEAR
                        })
                    });
                    if inside && !swept(from, f0, n0) && !swept(to, f3, n3) {
                        bulged[i] = true;
                        plans.push((i, plan));
                        continue;
                    }
                }
                // Drawn plain: its label goes onto its own line, off its
                // twin's where that covers it (after the loop below).
                let away = if (f0.0 + f3.0) / 2.0 >= tb { 1.0 } else { -1.0 };
                let j = nearest.map_or(i, |t| t.0);
                beside[i] = Some((tb + away * (across + TWIN_AIR), away, j));
            }
        }
        let plan = flow_path(
            &ctx,
            i,
            (f0, n0, glyph.0),
            &cols,
            (f3, n3, glyph.1),
            label,
            (
                stubs.get(i).copied().flatten(),
                end_stubs.get(i).copied().flatten(),
            ),
            &mut jogs,
        );
        let Some(plan) = plan else { continue };
        // Only a plain Z's one step carries its label on its run.
        let steps: Vec<usize> = plan
            .iter()
            .filter_map(|c| match *c {
                Corner::Jog(k, _) => Some(k),
                Corner::At(_) => None,
            })
            .collect();
        if steps.len() != 2 || plan.len() != 4 {
            for k in steps {
                jogs[k].label_half = 0.0;
            }
        }
        if is_chain {
            chain[i] = true;
        }
        plans.push((i, plan));
    }
    place_tracks(&mut jogs, &kept, cx.blocks);
    uncross(&mut jogs, &kept_lines);

    for (i, plan) in plans {
        let corners: Vec<Pt> = plan
            .iter()
            .map(|c| match *c {
                Corner::At(p) => p,
                Corner::Jog(k, b) => (b, jogs[k].at),
            })
            .collect();
        let corners = collapse(corners);
        if corners.len() < 2 {
            // Both ends on one point (a drag laid a node on another):
            // the builder's path stays.
            continue;
        }
        let e = &mut sc.edges[i];
        if corners.len() == 2 && e.waypoints.is_empty() {
            // Straight: the builder's cubic along its run stays, or (a
            // bar's end moved under its branch) a cubic along the run.
            let (a, b) = (from_flow(dir, corners[0]), from_flow(dir, corners[1]));
            if a != e.bezier[0] || b != e.bezier[3] {
                e.bezier = [
                    a,
                    (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0),
                    (a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0),
                    b,
                ];
            }
            continue;
        }
        let screen: Vec<Pt> = corners.iter().map(|&p| from_flow(dir, p)).collect();
        // Its label rides the run into the target when its step lies
        // between the source and the label.
        let rides = plan
            .iter()
            .any(|c| matches!(*c, Corner::Jog(k, _) if jogs[k].above_label));
        if bulged[i] && screen.len() == 6 {
            // On the middle of its bulge's run.
            if let Some((_, at, _)) = e.label.as_mut() {
                let (a, b) = (screen[2], screen[3]);
                *at = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            }
        } else if cx.blocks && chain[i] {
            // A block layout's long edge: its label keeps the slot the
            // layout reserved on the run between its two end blocks
            // ([`crate::layout::label_slot`]), on the new path. Moved
            // with the path's middle, the label of a chain whose
            // columns lined up into one run and a last step went onto
            // that step, inside the block the edge ends in.
            if let Some((_, at, _)) = e.label.as_mut() {
                *at = nearest_on(&screen, *at);
            }
        } else {
            relabel(e, &screen, rides.then_some(dir));
        }

        let n = screen.len();
        if n == 2 {
            // A long edge whose columns line up: one straight run.
            let (a, b) = (screen[0], screen[1]);
            e.bezier = [
                a,
                (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0),
                (a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0),
                b,
            ];
            e.waypoints = Vec::new();
            continue;
        }
        e.bezier = [screen[0], screen[1], screen[n - 2], screen[n - 1]];
        e.waypoints = crate::router::smooth(&screen, crate::router::CORNER_R);
    }
    // A plain twin's label goes onto its own line; where it covers its
    // twin there, or comes within the label stage's keep-out of another
    // stroke, it moves along one of its own runs along the flow to a
    // spot clear of every stroke and box (its spot along the flow kept
    // where the run allows, else the nearest that keeps the label off
    // the boxes at the run's ends), else steps beside its run, off the
    // twin.
    if beside.iter().any(Option::is_some) {
        let lines: Vec<Vec<Pt>> = sc.edges.iter().map(crate::scene::edge_polyline).collect();
        for (i, b) in beside.iter().enumerate() {
            let Some((edge, away, _)) = *b else { continue };
            let own: Vec<Pt> = match crate::router::ortho_corners(&sc.edges[i]) {
                Some(c) => c,
                None => vec![sc.edges[i].bezier[0], sc.edges[i].bezier[3]],
            };
            let Some((_, at, w)) = sc.edges[i].label.as_mut() else {
                continue;
            };
            // (Onto its own line: the builder's offset bowed it off its
            // twin's curve, which this line no longer follows.)
            *at = nearest_on(&own, *at);
            let h = crate::layout::LABEL_BOX_H / 2.0;
            let w = *w;
            let rect = |c: Pt| (c.0 - w / 2.0, c.1 - h, c.0 + w / 2.0, c.1 + h);
            let keep = crate::labels::LABEL_GAP + crate::labels::STROKE_PAD;
            let grown = |r: (f64, f64, f64, f64), d: f64| (r.0 - d, r.1 - d, r.2 + d, r.3 + d);
            let off_boxes = |r: (f64, f64, f64, f64)| {
                sc.nodes.iter().all(|n| {
                    r.2 <= n.x - n.w / 2.0
                        || r.0 >= n.x + n.w / 2.0
                        || r.3 <= n.y - n.h / 2.0
                        || r.1 >= n.y + n.h / 2.0
                })
            };
            let clear = |c: Pt| {
                let r = rect(c);
                let near = grown(r, keep);
                off_boxes(grown(r, crate::labels::LABEL_GAP))
                    && lines.iter().enumerate().all(|(k, l)| {
                        k == i || !l.windows(2).any(|s| seg_hits_rect(s[0], s[1], near))
                    })
            };
            if clear(*at) {
                continue;
            }
            let f = to_flow(dir, *at);
            // The label's half extent along the flow.
            let half = match dir {
                Direction::TD | Direction::BT => h,
                Direction::LR | Direction::RL => w / 2.0,
            };
            // Spots along its own legs (flow coordinates), each also slid
            // along the flow while the label still covers its leg: on a
            // run along the flow the label slides along it, on a step
            // across it the step stays inside the label. The nearest
            // clear one to where the label was wins.
            let mut spot: Option<(f64, Pt)> = None;
            for seg in own.windows(2) {
                let (a, b) = (to_flow(dir, seg[0]), to_flow(dir, seg[1]));
                let len = (b.0 - a.0).abs() + (b.1 - a.1).abs();
                let n = ((len / SPOT_STEP).ceil() as usize).clamp(1, SPOT_MAX);
                let step_across = (a.1 - b.1).abs() < EPS;
                let slides: Vec<f64> = if step_across {
                    let k = ((half / SPOT_STEP) as i64).min(SPOT_MAX as i64);
                    (-k..=k).map(|m| m as f64 * SPOT_STEP).collect()
                } else {
                    vec![0.0]
                };
                for q in 0..=n {
                    let t = q as f64 / n as f64;
                    let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                    for &dx in &slides {
                        let c = (p.0, p.1 + dx);
                        let d = (c.0 - f.0).abs() + (c.1 - f.1).abs();
                        if spot.is_some_and(|(best, _)| best <= d) {
                            continue;
                        }
                        let s = from_flow(dir, c);
                        if clear(s) {
                            spot = Some((d, s));
                        }
                    }
                }
            }
            let spot = spot.map(|s| s.1);
            match spot {
                Some(c) => *at = c,
                None if (edge - f.0) * away > 0.0 => *at = from_flow(dir, (edge, f.1)),
                None => {}
            }
        }
    }
    cx.chain = chain;
    cx.shaped = bulged;
}

/// Whether segment `a`-`b` meets the rect `(x0, y0, x1, y1)`.
fn seg_hits_rect(a: Pt, b: Pt, r: (f64, f64, f64, f64)) -> bool {
    let inside = |p: Pt| p.0 >= r.0 && p.0 <= r.2 && p.1 >= r.1 && p.1 <= r.3;
    if inside(a) || inside(b) {
        return true;
    }
    let c = [(r.0, r.1), (r.2, r.1), (r.2, r.3), (r.0, r.3)];
    (0..4).any(|k| segs_cross(a, b, c[k], c[(k + 1) % 4]))
}

/// What a step keeps clear of across the flow: a span along the breadth
/// and the span along the flow it blocks there (flow coordinates).
type Wall = ((f64, f64), (f64, f64));

/// What [`flow_path`] reads.
struct Along<'a> {
    gaps: &'a [(f64, f64)],
    boxes: &'a [(f64, f64, f64, f64)],
    walls: &'a [Wall],
    /// The frames (flow coordinates), and those holding an end of the
    /// edge (never in its way).
    frames: &'a [(f64, f64, f64, f64)],
    home: &'a [usize],
    /// A block layout ([`Ctx::blocks`]): a step keeps out of the frames
    /// that hold neither of its ends ([`plan_jog`]).
    blocks: bool,
    /// A block layout's scene, static or routed ([`Ctx::axis_ends`]): a
    /// run reaching its port within `ALIGNED` of its line is laid on it.
    axis_ends: bool,
    /// The spans of the jogs each gap holds so far.
    load: &'a std::cell::RefCell<Vec<Vec<(f64, f64)>>>,
    /// The edge's own end nodes (never in its way).
    own: [Option<usize>; 2],
    /// The straight run (px) it keeps into an end glyph: [`HEAD_LEG`], or
    /// a head grown under a wide stroke and a corner's rounding.
    head_leg: f64,
    /// Every edge's line as the stage found it (flow coordinates; empty
    /// where none is judged), for the gap a long run steps across in.
    lines: &'a [Vec<Pt>],
    /// The frames' title sides (a block layout's; [`TitleSides`]).
    titles: &'a TitleSides,
}

impl Along<'_> {
    /// Whether a straight run at breadth `b` from `l0` to `l1` (flow
    /// coordinates) keeps [`ROW_CLEAR`] off every box but the edge's own
    /// ends.
    fn clear_run(&self, b: f64, l0: f64, l1: f64) -> bool {
        let (l0, l1) = (l0.min(l1), l0.max(l1));
        self.boxes.iter().enumerate().all(|(k, r)| {
            self.own.contains(&Some(k))
                || b <= r.0 - ROW_CLEAR
                || b >= r.1 + ROW_CLEAR
                || l1 <= r.2
                || l0 >= r.3
        })
    }
}

/// Per edge, the breadth (flow coordinates) of the stub its start takes
/// off a breadth side whose ends would turn onto their runs through a
/// box of the node's own row ([`flow_path`]): a `<<choice>>` diamond's
/// fan spilled round its left and right corners ran along the diamond's
/// row to each target's column, through the note cards beside it, and
/// the router rerouted the lot across each other (B10 review r2 #2: a
/// state set's 8-way labelled fan with notes, 3 crossings as curves, 15
/// so, 2 now). Once one start of a side would, every start leaving that
/// side goes out a stub past the side's outermost port and takes its
/// step in a gap instead, each stub a track longer for every start
/// further along the flow on that side, so their runs along the flow
/// nest instead of crossing the others' stubs. (An end turning into a
/// breadth side is a diamond's vertex or a lone port: the router reroutes
/// the rare one blocked, which stubbing every such end cost more boxes
/// shared and runs crowded than it saved, on the 964-file state set.)
fn breadth_stubs(sc: &Scene, cx: &Ctx, dir: Direction, boxes: &[Rect]) -> Vec<Option<f64>> {
    struct Start {
        edge: usize,
        end: EndRef,
        side: Side,
        /// Along the flow, and the way the run heads along it.
        at: f64,
        way: f64,
        /// Its run across to its first column hits a box.
        blocked: bool,
        /// The port's breadth, and the side's outward normal across the
        /// flow.
        port: f64,
        out: f64,
    }
    let mut starts: Vec<Start> = Vec::new();
    let clear = EDGE_SEP / 2.0;
    for (i, e) in sc.edges.iter().enumerate() {
        let Some((from, to)) = cx.ends.get(i).copied().flatten() else {
            continue;
        };
        if !cx.drawn.get(i).copied().unwrap_or(false)
            || cx.fixed.get(i).copied().unwrap_or(false)
            || e.meta.curve.is_some()
            || from == to
            || crate::ports::note_link(sc, cx, i)
            || crate::edgepass::holds_other(cx, from, to)
            || (!e.waypoints.is_empty() && crate::router::ortho_corners(e).is_some())
        {
            continue;
        }
        let (p0, p3) = if e.waypoints.len() >= 2 {
            (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
        } else {
            (e.bezier[0], e.bezier[3])
        };
        let (f0, f3) = (to_flow(dir, p0), to_flow(dir, p3));
        let way = (f3.1 - f0.1).signum();
        let side = match from {
            EndRef::Node(v) => {
                crate::ports::outline_side(sc.nodes[v].shape, end_rect(sc, from), p0)
            }
            EndRef::Frame(_) => side_of(end_rect(sc, from), p0),
        };
        let n = flow_normal(dir, side);
        if way == 0.0 || n.1 != 0.0 || n.0 == 0.0 {
            continue;
        }
        // The column the run passes first (a dummy chain's), and its
        // run across to it, turning onto the flow ahead of the side.
        let first = if e.waypoints.len() >= 3 {
            to_flow(dir, e.waypoints[1]).0
        } else {
            f3.0
        };
        let own = [from, to].map(|r| match r {
            EndRef::Node(v) => Some(v),
            EndRef::Frame(_) => None,
        });
        let (b0, b1) = (f0.0.min(first), f0.0.max(first));
        let blocked = (first - f0.0) * n.0 > 0.0
            && boxes.iter().enumerate().any(|(k, r)| {
                !own.contains(&Some(k))
                    && f0.1 > r.2 - clear
                    && f0.1 < r.3 + clear
                    && b1 > r.0 - clear
                    && b0 < r.1 + clear
            });
        starts.push(Start {
            edge: i,
            end: from,
            side,
            at: f0.1,
            way,
            blocked,
            port: f0.0,
            out: n.0,
        });
    }
    let mut out = vec![None; sc.edges.len()];
    for m in &starts {
        let group = || starts.iter().filter(|o| o.end == m.end && o.side == m.side);
        if !group().any(|o| o.blocked) {
            continue;
        }
        // Past the side's outermost port (a diamond's vertex lies beyond
        // the ports on its faces).
        let edge = group()
            .map(|o| o.port * m.out)
            .fold(f64::NEG_INFINITY, f64::max)
            * m.out;
        let rank = group()
            .filter(|o| o.way == m.way && (o.at - m.at) * m.way > EPS)
            .count();
        out[m.edge] = Some(edge + m.out * (ROW_CLEAR + rank as f64 * TRACK_PITCH));
    }
    out
}

/// A frame's title side as [`TitleSides`] reads it (flow coordinates):
/// the frame's breadth span, the flow coordinate of its title side,
/// which way along the flow the frame lies from it, the chip's breadth
/// span widened by [`CHIP_CLEAR`], and the breadth its first displaced
/// column may take ([`TitleSides::sides`]).
type TitleSide = (f64, f64, f64, f64, (f64, f64), f64);

/// The title sides of a block layout's frames (TD / BT; B12 2.8,
/// "Title-clear entries", rule 2 in its smallest form): an edge end on a
/// side of a member of the frame's title row that lies wholly under the
/// frame's title chip widened by [`CHIP_CLEAR`] — an entry into such a
/// member, or an exit out of it going back up — is *displaced*: the edge
/// crosses the title side at a column past the chip's end, the columns
/// of one side [`TRACK_PITCH`] apart in the order of the breadths they
/// replace, and steps over to its port inside the frame, in the jog room
/// between the title strip and the title row that the layout reserves
/// for them (`layout::title_jogs`). Through the chip, the router took
/// such an edge round it and in through the frame's lateral wall. Only
/// edge ends are displaced: a run of the frame's own under the chip is
/// the layout's to move past it (X2.5, `layout::BlockOrder`'s
/// `clear_titles`), and a channel column that crosses a title side keeps
/// the column the layout gave it ([`straighten`]'s pins). A column that
/// would leave the frame is not taken: the edge is drawn as before.
struct TitleSides {
    /// Per frame: its breadth span, the flow coordinate of its title
    /// side, which way along the flow the frame lies from it, the
    /// chip's breadth span widened by [`CHIP_CLEAR`] (flow coordinates),
    /// when it has a title on a flow side, and where its displaced
    /// columns start: a corner's rounding past that, and at least
    /// [`CHIP_CLEAR`] past the title's bold ink
    /// ([`crate::scene::title_head_measured`]), as every other stage of
    /// a block layout measures the chip. (Which ends are displaced is
    /// judged at the chip: judged at the ink as well, the ends of a fan
    /// into a block whose side reaches past the chip but not past the
    /// ink were displaced, and quality/dense-td drawn TD took a crossing
    /// and a line through a node.)
    sides: Vec<Option<TitleSide>>,
    /// The displaced crossings: (edge, frame) -> the column it takes.
    cols: std::collections::BTreeMap<(usize, usize), f64>,
}

impl TitleSides {
    fn of(sc: &Scene, cx: &Ctx, dir: Direction, frames: &[Rect]) -> TitleSides {
        let mut out = TitleSides {
            sides: Vec::new(),
            cols: std::collections::BTreeMap::new(),
        };
        if !cx.blocks || !matches!(dir, Direction::TD | Direction::BT) || sc.clusters.is_empty() {
            return out;
        }
        let right = sc.origin.0 + sc.width;
        out.sides = (0..sc.clusters.len())
            .map(|ci| {
                if sc.clusters[ci].title.trim().is_empty() {
                    return None;
                }
                let r = frames[ci];
                let head = |inked| {
                    flow_rect(
                        dir,
                        crate::scene::title_head_measured(&sc.clusters, ci, right, inked),
                    )
                };
                let (h, ink) = (head(false), head(true));
                // The side the chip sits on, and which way along the flow
                // the frame lies from it.
                let (t, inward) = if (h.2 - r.2).abs() <= (r.3 - h.3).abs() {
                    (r.2, 1.0)
                } else {
                    (r.3, -1.0)
                };
                // The first column: a corner's rounding past the chip's
                // clearance, so the turn onto the track under the chip
                // stays out of the chip's ring, and never nearer the
                // title's ink than its clearance.
                let start = (h.1 + CHIP_CLEAR + crate::router::CORNER_R).max(ink.1 + CHIP_CLEAR);
                Some((
                    r.0,
                    r.1,
                    t,
                    inward,
                    (h.0 - CHIP_CLEAR, h.1 + CHIP_CLEAR),
                    start,
                ))
            })
            .collect();
        // Every displaced crossing, by frame: (breadth it replaces, edge);
        // and the breadths the side's other crossings take past the chip.
        let mut by: std::collections::BTreeMap<usize, Vec<(f64, usize)>> =
            std::collections::BTreeMap::new();
        let mut taken: std::collections::BTreeMap<usize, Vec<f64>> =
            std::collections::BTreeMap::new();
        for (i, e) in sc.edges.iter().enumerate() {
            if !cx.drawn.get(i).copied().unwrap_or(false)
                || e.waypoints.len() < 2 && e.bezier[0] == e.bezier[3]
            {
                continue;
            }
            let pts: Vec<Pt> = if e.waypoints.len() >= 2 {
                e.waypoints.iter().map(|&p| to_flow(dir, p)).collect()
            } else {
                vec![to_flow(dir, e.bezier[0]), to_flow(dir, e.bezier[3])]
            };
            // Only an end on a side that lies wholly under the chip: the
            // ports stage keeps the others' ends on the free stretch
            // beside it (rule 1), and drawn displaced as well, the
            // entries of a fan into a narrow block's partly shaded
            // member crossed each other above it (dense-td's `n43`).
            let shaded = |end: Option<EndRef>, k: usize| {
                let (Some(EndRef::Node(v)), Some((_, _, _, _, (c0, c1), _))) = (end, out.sides[k])
                else {
                    return false;
                };
                let r = flow_rect(dir, end_rect(sc, EndRef::Node(v)));
                r.0 >= c0 && r.1 <= c1
            };
            let ends = cx.ends.get(i).copied().flatten();
            for (k, b, displaced) in out.all_crossings(&pts) {
                let end = ends.map(|(f, t)| {
                    let t_flow = flow_rect(dir, end_rect(sc, t));
                    if b >= t_flow.0 && b <= t_flow.1 {
                        t
                    } else {
                        f
                    }
                });
                if displaced && shaded(end, k) {
                    by.entry(k).or_default().push((b, i));
                } else {
                    taken.entry(k).or_default().push(b);
                }
            }
        }
        for (k, mut v) in by {
            let Some((_, f1, _, _, _, start)) = out.sides[k] else {
                continue;
            };
            let mut taken = taken.remove(&k).unwrap_or_default();
            taken.sort_by(f64::total_cmp);
            v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            // The nearest free columns past the chip, in the order of
            // the breadths they replace, a track apart and a track off
            // the side's other crossings, from the side's `start`.
            let mut next = start;
            for &(b, i) in &v {
                let col = column_clear_of(next, &taken);
                if col > f1 - crate::scene::LANE_PAD {
                    break;
                }
                // Nor past another edge's crossing of the side between
                // its port and the column: its step back to the port
                // would cross that edge's run inside the frame
                // (blocks/lr drawn BT, `cdn -> gateway` out past
                // `ship -> pager` and back over it and `cart -> pager`).
                // Drawn as before; so is every entry after it.
                if taken.iter().any(|&o| (o - b) * (o - col) < 0.0) {
                    break;
                }
                out.cols.entry((i, k)).or_insert(col);
                next = col + TRACK_PITCH;
            }
        }
        out
    }

    /// The displaced crossings of a path through `pts` (flow
    /// coordinates): (frame, breadth it would cross the title side at).
    fn crossings(&self, pts: &[Pt]) -> Vec<(usize, f64)> {
        self.all_crossings(pts)
            .into_iter()
            .filter(|c| c.2)
            .map(|c| (c.0, c.1))
            .collect()
    }

    /// Every crossing of a title side by a path through `pts` (flow
    /// coordinates) inside the frame's breadth: (frame, breadth, whether
    /// it is displaced: under the chip).
    fn all_crossings(&self, pts: &[Pt]) -> Vec<(usize, f64, bool)> {
        let mut out = Vec::new();
        for (k, side) in self.sides.iter().enumerate() {
            let Some((f0, f1, t, inward, (c0, c1), _)) = *side else {
                continue;
            };
            let last = pts.len().saturating_sub(1);
            for (j, w) in pts.windows(2).enumerate() {
                let (a, b) = (w[0], w[1]);
                if (a.1 - t) * (b.1 - t) >= 0.0 {
                    continue;
                }
                // The end inside the frame sets the breadth it crosses at.
                let (p, at) = if (b.1 - t) * inward > 0.0 {
                    (b, j + 1)
                } else {
                    (a, j)
                };
                // Displaced only where that end is the edge's own, a port
                // of the title row's (an entry, or an exit going back up):
                // a run's column under the chip is the coordinates' to
                // move (X2.5), and jogged clear of it here, the runs of a
                // fan into a block crossed each other above it (dense-td).
                let port = at == 0 || at == last;
                if p.0 > f0 && p.0 < f1 {
                    out.push((k, p.0, port && p.0 > c0 && p.0 < c1));
                }
            }
        }
        out
    }

    /// The columns of edge `i`'s path from `f0` through `cols` to `f3`
    /// (flow coordinates), with a column past the chip of each title
    /// side it crosses displaced.
    fn clear(&self, i: usize, f0: Pt, cols: Vec<Pt>, f3: Pt) -> Vec<Pt> {
        if self.cols.is_empty() {
            return cols;
        }
        let pts: Vec<Pt> = std::iter::once(f0)
            .chain(cols.iter().copied())
            .chain(std::iter::once(f3))
            .collect();
        let mut out: Vec<Pt> = Vec::with_capacity(cols.len() + 2);
        for (j, w) in pts.windows(2).enumerate() {
            if j > 0 {
                out.push(w[0]);
            }
            let (a, b) = (w[0], w[1]);
            for (k, side) in self.sides.iter().enumerate() {
                let (Some((_, _, t, _, _, _)), Some(&col)) = (side, self.cols.get(&(i, k))) else {
                    continue;
                };
                if (a.1 - t) * (b.1 - t) < 0.0 && self.crossings(&[a, b]).iter().any(|c| c.0 == k) {
                    out.push((col, *t));
                }
            }
        }
        out
    }
}

/// Per edge, in a block layout ([`Ctx::blocks`]): the breadth (flow
/// coordinates) of the run its end takes into a breadth side, where the
/// straight leg off its run onto that side would pass through another
/// box (B12 2.8, an over-full side's corner legs). A block whose members
/// stand side by side across the flow (a `direction` block, stacked in
/// the other direction) spills the ends of its members' over-full flow
/// sides round their corners, and the leg from the run into a spilled
/// end then crossed the members stacked beside it (bench-75: the edges
/// into `db` and `cache` dropped through `queue` and `db`, 11 boxes hit,
/// which the router rerouted one by one, out of their fans' order).
/// The end takes its last run in the gap beside its side instead, just
/// outside it, a track further out for every end of that side reached
/// before it along the flow, so the runs nest round each other's turns,
/// and the step to it is taken in the gap before the end's row. As for
/// starts ([`breadth_stubs`]), once one end of a side would pass through
/// a box, every end into that side takes its run so. Outside a block
/// layout every end keeps its turn (`None`).
fn breadth_ends(sc: &Scene, cx: &Ctx, dir: Direction, boxes: &[Rect]) -> Vec<Option<f64>> {
    let mut out = vec![None; sc.edges.len()];
    if !cx.blocks {
        return out;
    }
    struct End {
        edge: usize,
        end: EndRef,
        side: Side,
        at: f64,
        way: f64,
        blocked: bool,
        port: f64,
        out: f64,
    }
    let mut ends: Vec<End> = Vec::new();
    let clear = EDGE_SEP / 2.0;
    for (i, e) in sc.edges.iter().enumerate() {
        let Some((from, to)) = cx.ends.get(i).copied().flatten() else {
            continue;
        };
        if !cx.drawn.get(i).copied().unwrap_or(false)
            || cx.fixed.get(i).copied().unwrap_or(false)
            || e.meta.curve.is_some()
            || from == to
            || crate::ports::note_link(sc, cx, i)
            || crate::edgepass::holds_other(cx, to, from)
            || (!e.waypoints.is_empty() && crate::router::ortho_corners(e).is_some())
        {
            continue;
        }
        let (p0, p3) = if e.waypoints.len() >= 2 {
            (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
        } else {
            (e.bezier[0], e.bezier[3])
        };
        let (f0, f3) = (to_flow(dir, p0), to_flow(dir, p3));
        let way = (f3.1 - f0.1).signum();
        let side = match to {
            EndRef::Node(v) => crate::ports::outline_side(sc.nodes[v].shape, end_rect(sc, to), p3),
            EndRef::Frame(_) => side_of(end_rect(sc, to), p3),
        };
        let n = flow_normal(dir, side);
        if way == 0.0 || n.1 != 0.0 || n.0 == 0.0 {
            continue;
        }
        // The column the run passes last, and the leg from it onto the
        // side, turning off the flow just short of the end.
        let last = if e.waypoints.len() >= 3 {
            to_flow(dir, e.waypoints[e.waypoints.len() - 2]).0
        } else {
            f0.0
        };
        let own = [from, to].map(|r| match r {
            EndRef::Node(v) => Some(v),
            EndRef::Frame(_) => None,
        });
        let (b0, b1) = (f3.0.min(last), f3.0.max(last));
        let blocked = (last - f3.0) * n.0 > 0.0
            && boxes.iter().enumerate().any(|(k, r)| {
                !own.contains(&Some(k))
                    && f3.1 > r.2 - clear
                    && f3.1 < r.3 + clear
                    && b1 > r.0 - clear
                    && b0 < r.1 + clear
            });
        ends.push(End {
            edge: i,
            end: to,
            side,
            at: f3.1,
            way,
            blocked,
            port: f3.0,
            out: n.0,
        });
    }
    for m in &ends {
        let group = || ends.iter().filter(|o| o.end == m.end && o.side == m.side);
        if !group().any(|o| o.blocked) {
            continue;
        }
        let edge = group()
            .map(|o| o.port * m.out)
            .fold(f64::NEG_INFINITY, f64::max)
            * m.out;
        let rank = group()
            .filter(|o| o.way == m.way && (o.at - m.at) * m.way < -EPS)
            .count();
        let lead = HEAD_LEG.max(ROW_CLEAR);
        out[m.edge] = Some(edge + m.out * (lead + rank as f64 * TRACK_PITCH));
    }
    out
}

/// The corners of an edge from `a` (point, its side's outward normal,
/// a glyph on it; flow coordinates) through each column of `cols` (a
/// dummy chain's points) to `b`: along the flow, with a jog in a gap
/// wherever the breadth changes (pushed onto `jogs`). An end on a
/// breadth side (a port the ports stage spilled round a corner, a note
/// card's link, boxes side by side) turns onto the run with one corner
/// where the run's first (last) column passes it. `None` when no such
/// path fits the ends' sides (one would have to leave its side
/// backwards).
#[allow(clippy::too_many_arguments)]
fn flow_path(
    cx: &Along,
    edge: usize,
    a: (Pt, Pt, bool),
    cols: &[Pt],
    b: (Pt, Pt, bool),
    label: Option<(f64, f64)>,
    (stub, end_stub): (Option<f64>, Option<f64>),
    jogs: &mut Vec<Jog>,
) -> Option<Vec<Corner>> {
    let ((p0, n0, g0), (p3, n3, g3)) = (a, b);
    let across = |n: Pt| n.1 == 0.0 && n.0 != 0.0;
    // Boxes side by side: a Z across the flow, its jog midway.
    let between = cx
        .gaps
        .iter()
        .any(|&(lo, hi)| hi.min(p0.1.max(p3.1)) - lo.max(p0.1.min(p3.1)) > EPS);
    if across(n0) && across(n3) && !between && cols.is_empty() {
        return across_flow(p0, n0.0, p3, n3.0);
    }
    let h = (p3.1 - p0.1).signum();
    if h == 0.0 {
        return None;
    }
    // Where the run along the flow starts and ends: the port itself on a
    // side facing along the run, else the corner turning onto it.
    let first_b = cols.first().map_or(p3.0, |c| c.0);
    let last_b = cols.last().map_or(p0.0, |c| c.0);
    let ahead = |from: f64, to: f64, n: f64| (to - from) * n > BARE_LEG - EPS;
    // How far past a breadth side a turn onto (off) the run lies when
    // the run's column does not pass outside it: clear of the row, and a
    // head's straight run into a glyph end.
    let lead = |glyph: bool| if glyph { cx.head_leg } else { ROW_CLEAR };
    let mut out = vec![Corner::At(p0)];
    let start = if n0 == (0.0, h) {
        (p0, true, g0)
    } else if across(n0) {
        // (Two breadth sides with a stub at either end, in a block layout
        // ([`breadth_ends`]): each end's own run, a step between.)
        let stubbed = cx.blocks && (stub.is_some() || end_stub.is_some());
        let b = if cols.is_empty() && across(n3) && !stubbed {
            if n0.0 == -n3.0 && ahead(p0.0, (p0.0 + p3.0) / 2.0, n0.0) {
                (p0.0 + p3.0) / 2.0
            } else if n0.0 == n3.0 {
                // Both sides look one way: one run out past both.
                let reach = ROW_CLEAR.max(lead(g0)).max(lead(g3));
                if n0.0 > 0.0 {
                    p0.0.max(p3.0) + reach
                } else {
                    p0.0.min(p3.0) - reach
                }
            } else {
                p0.0 + n0.0 * lead(g0)
            }
        } else if let Some(x) = stub {
            // Its side's run across to the first column would pass
            // through a box of its own row: out a stub
            // ([`breadth_stubs`]), then along the flow to a gap.
            let least = p0.0 + n0.0 * lead(g0);
            if n0.0 > 0.0 {
                x.max(least)
            } else {
                x.min(least)
            }
        } else if ahead(p0.0, first_b, n0.0) {
            first_b
        } else {
            p0.0 + n0.0 * lead(g0)
        };
        let c = (b, p0.1);
        out.push(Corner::At(c));
        (c, false, false)
    } else {
        return None;
    };
    // The end: square into a side facing back along the run, else a turn
    // off the run into a breadth side.
    let (stop, turn) = if n3 == (0.0, -h) {
        ((p3, true, g3), None)
    } else if across(n3) {
        let run = if cols.is_empty() { start.0 .0 } else { last_b };
        let b = if let Some(x) = end_stub {
            // Its run beside its side ([`breadth_ends`]), at least its
            // own lead out.
            let least = p3.0 + n3.0 * lead(g3);
            if n3.0 > 0.0 {
                x.max(least)
            } else {
                x.min(least)
            }
        } else if ahead(p3.0, run, n3.0) {
            run
        } else {
            p3.0 + n3.0 * lead(g3)
        };
        let c = (b, p3.1);
        ((c, false, false), Some(c))
    } else {
        return None;
    };
    let mut cur = start;
    let stops: Vec<(Pt, bool, bool)> = cols
        .iter()
        .map(|&c| (c, false, false))
        .chain(std::iter::once(stop))
        .collect();
    for (si, &(q, port, gq)) in stops.iter().enumerate() {
        // A column behind the run (a chain that doubles back) is passed by.
        if (q.1 - cur.0 .1) * h <= 0.0 && !port && turn != Some(q) {
            continue;
        }
        // A column a stair step's merge length or less off the run is
        // passed on the run's own line where that runs clear of the boxes
        // to the next stop: the layout's dummies drift a few px from
        // layer to layer, and a step for each read as a ragged line.
        // (Not a column whose run crosses a block's title side: the
        // layout placed it past the chip and the columns of the block's
        // displaced entries, X2.2 and X2.5; on the run's line it moved
        // back into their room.)
        let titled = || {
            let next = stops.get(si + 1).map_or(q, |n| n.0);
            !cx.titles.all_crossings(&[cur.0, q]).is_empty()
                || !cx.titles.all_crossings(&[q, next]).is_empty()
        };
        let q = if !port
            && turn != Some(q)
            && (q.0 - cur.0 .0).abs() < 2.0 * crate::router::CORNER_R
            && !titled()
        {
            let next = stops.get(si + 1).map_or(q.1, |n| n.0 .1);
            if cx.clear_run(cur.0 .0, cur.0 .1, next) {
                (cur.0 .0, q.1)
            } else {
                q
            }
        } else {
            q
        };
        if (q.0 - cur.0 .0).abs() > ALIGNED {
            if (q.1 - cur.0 .1) * h <= EPS {
                return None;
            }
            let k = jogs.len();
            // A label rides the run into the target of a plain Z.
            let lab = label.filter(|_| cols.is_empty() && cur.1 && port);
            jogs.push(plan_jog(cx, edge, cur, (q, port, gq), h, lab));
            out.push(Corner::Jog(k, cur.0 .0));
            out.push(Corner::Jog(k, q.0));
        } else if cx.axis_ends && port && (q.0 - cur.0 .0).abs() > EPS {
            // In a block layout (its static scene, and `route()` with its
            // frames, so a drag draws it as the scene does) a run that
            // reaches its port within `ALIGNED` of its line is laid on
            // the port's line from its last turn on: the ports stage
            // spread the end a fraction of a px off the chain's last
            // column, and a run left on the column met its head at a
            // slant, the painted spline kinked under it (dense-lr
            // `n22 -.-> n44`, 0.5 px over 316 px, `idiom_spline` and
            // `end_tilt_max` 0.8 deg in the static `DEFECT_BASELINE`;
            // `route()` of the public fixture drawn LR left `REG ==> ORD`
            // 0.56 px off its port). Its own source port stays where it
            // is.
            let first = out.len() == 1;
            match out.last_mut() {
                Some(Corner::Jog(k, b)) => {
                    *b = q.0;
                    let j = &mut jogs[*k];
                    if h > 0.0 {
                        j.hi_at = q.0;
                    } else {
                        j.lo_at = q.0;
                    }
                }
                Some(Corner::At(c)) if !first => c.0 = q.0,
                _ => {}
            }
        }
        cur = (q, port, gq);
    }
    if let Some(c) = turn {
        out.push(Corner::At(c));
    }
    out.push(Corner::At(p3));
    Some(out)
}

/// Room (px) a straightened column keeps from a box, a frame's border or
/// a label it runs past: the layout's own room between a dummy and the
/// box beside it ([`crate::layout::GAP_B`] less half a dummy's breadth),
/// halved — still clear of the boxes by a good deal more than a
/// reroute keeps ([`ROW_CLEAR`]).
const COLUMN_CLEAR: f64 = 21.0;

/// What [`straighten`] reads (flow coordinates).
struct ColumnRoom<'a> {
    /// The rows of boxes along the flow.
    rows: &'a [(f64, f64)],
    boxes: &'a [(f64, f64, f64, f64)],
    /// Every self-loop's box with its node: drawn beside the node, in
    /// a column's way as much as the node (the router's rule).
    loops: &'a [(usize, Rect)],
    frames: &'a [(f64, f64, f64, f64)],
    /// Every long edge's dummy columns: (edge, point).
    columns: &'a [(usize, Pt)],
    /// The runs along the flow of the paths on the axes already:
    /// (breadth, flow span).
    runs: &'a [(f64, (f64, f64))],
    /// Every label box: (edge, box).
    labels: &'a [(usize, Rect)],
    /// The edge's own end nodes.
    own: [Option<usize>; 2],
    edge: usize,
}

impl ColumnRoom<'_> {
    /// The breadth a straight run through column `q` may take: the free
    /// span round the dummy, [`COLUMN_CLEAR`] off the boxes, frame
    /// borders and labels beside it along its row, and half way to the
    /// next long edge's column there less half a track (two columns
    /// straightened towards each other stay a track apart, in the
    /// layout's order).
    fn slot(&self, q: Pt) -> (f64, f64) {
        let band = self
            .rows
            .iter()
            .find(|r| q.1 >= r.0 - EPS && q.1 <= r.1 + EPS)
            .copied()
            .unwrap_or((q.1, q.1));
        let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
        let mut fence = |a: f64, b: f64, clear: f64| {
            if b <= q.0 {
                lo = lo.max(b + clear);
            } else if a >= q.0 {
                hi = hi.min(a - clear);
            } else {
                // The dummy lies in it (a label over its own column):
                // nowhere to go.
                lo = lo.max(q.0);
                hi = hi.min(q.0);
            }
        };
        let meets = |r: &(f64, f64, f64, f64)| r.3 > band.0 - EPS && r.2 < band.1 + EPS;
        for (k, r) in self.boxes.iter().enumerate() {
            if !self.own.contains(&Some(k)) && meets(r) {
                fence(r.0, r.1, COLUMN_CLEAR);
            }
        }
        for (j, r) in self.labels {
            if *j != self.edge && meets(r) {
                fence(r.0, r.1, COLUMN_CLEAR);
            }
        }
        for (v, r) in self.loops {
            if !self.own.contains(&Some(*v)) && meets(r) {
                fence(r.0, r.1, COLUMN_CLEAR);
            }
        }
        for r in self.frames.iter().filter(|r| meets(r)) {
            for b in [r.0, r.1] {
                fence(b, b, COLUMN_CLEAR);
            }
        }
        for &(j, c) in self.columns {
            if j != self.edge && c.1 >= band.0 - EPS && c.1 <= band.1 + EPS {
                let mid = (q.0 + c.0) / 2.0;
                fence(mid, mid, TRACK_PITCH / 2.0);
            }
        }
        for &(b, (l0, l1)) in self.runs {
            if l1 > band.0 - EPS && l0 < band.1 + EPS {
                let mid = (q.0 + b) / 2.0;
                fence(mid, mid, TRACK_PITCH / 2.0);
            }
        }
        (lo.min(q.0), hi.max(q.0))
    }
}

/// A long edge's dummy columns (flow coordinates) straightened: each
/// moved within its [`ColumnRoom::slot`] so that runs of columns whose
/// slots share a breadth take one straight run (the fewest steps), at
/// the breadth nearest the run before it — the source's port for the
/// first — or the target's port for the last. The layout spreads a
/// dummy chain a whole box gap off its neighbours and drifts it from
/// layer to layer; on the axes each drift is a step, and a chain the
/// layout ordered past a row's last box ran its gap's width out beyond
/// it, widening the drawing. A `pinned` column keeps its breadth.
fn straighten(room: &ColumnRoom, from: f64, cols: &[Pt], to: f64, pinned: &[bool]) -> Vec<Pt> {
    let slots: Vec<(f64, f64)> = cols
        .iter()
        .enumerate()
        .map(|(k, &q)| {
            if pinned.get(k).copied().unwrap_or(false) {
                (q.0, q.0)
            } else {
                room.slot(q)
            }
        })
        .collect();
    let mut out: Vec<Pt> = cols.to_vec();
    let mut prev = from;
    let mut s = 0;
    while s < cols.len() {
        let (mut lo, mut hi) = slots[s];
        let mut t = s + 1;
        while t < cols.len() && slots[t].0 <= hi.min(slots[t].1) && slots[t].1 >= lo.max(slots[t].0)
        {
            (lo, hi) = (lo.max(slots[t].0), hi.min(slots[t].1));
            t += 1;
        }
        let last = t == cols.len();
        let b = if prev >= lo && prev <= hi {
            prev
        } else if last && to >= lo && to <= hi {
            to
        } else if last {
            // Nearest the stretch between the run before and the target.
            let (a, c) = (prev.min(to), prev.max(to));
            if hi < a {
                hi
            } else if lo > c {
                lo
            } else {
                prev.max(lo).min(hi)
            }
        } else {
            prev.max(lo).min(hi)
        };
        for q in &mut out[s..t] {
            q.0 = b;
        }
        prev = b;
        s = t;
    }
    out
}

/// The jog of `edge` between `u` (point, a port or a column, a glyph on
/// it) and `v`, the run heading `h` along the flow: the gap it steps
/// in, the room it has there and where it would go.
fn plan_jog(
    cx: &Along,
    edge: usize,
    u: (Pt, bool, bool),
    v: (Pt, bool, bool),
    h: f64,
    label: Option<(f64, f64)>,
) -> Jog {
    let (lo_pt, hi_pt) = if h > 0.0 { (u, v) } else { (v, u) };
    let (l0, l1) = (lo_pt.0 .1, hi_pt.0 .1);
    // The gaps strictly between the two, clipped to them.
    let cands: Vec<(usize, (f64, f64))> = cx
        .gaps
        .iter()
        .enumerate()
        .filter_map(|(k, &(a, b))| {
            let (a, b) = (a.max(l0), b.min(l1));
            (b - a > EPS).then_some((k, (a, b)))
        })
        .collect();
    let (bu, bv) = (u.0 .0, v.0 .0);
    // Boxes a straight run at breadth `x` from `y0` to `y1` passes
    // through (its own ends' aside).
    let hits = |x: f64, y0: f64, y1: f64| {
        let (y0, y1) = (y0.min(y1), y0.max(y1));
        cx.boxes
            .iter()
            .enumerate()
            .filter(|&(k, r)| {
                !cx.own.contains(&Some(k))
                    && x > r.0 - EPS
                    && x < r.1 + EPS
                    && y1 > r.2 + EPS
                    && y0 < r.3 - EPS
            })
            .count()
    };
    let mid = (l0 + l1) / 2.0;
    let (gap, (g_lo, g_hi)) = match cands.len() {
        0 => (usize::MAX - edge, (l0, l1)),
        1 => cands[0],
        _ => {
            // A long run with no columns: the gap whose step leaves both
            // straight runs clear of the most boxes, then of the frames
            // holding neither end, then the one with a track to spare for
            // it (a host's centres give `route()` no columns: a channel
            // between two frames would take every step), nearest the
            // middle.
            let span = (bu.min(bv), bu.max(bv));
            let foreign = |t: f64| {
                let inside = |r: &(f64, f64, f64, f64), b: f64, l0: f64, l1: f64| {
                    let (l0, l1) = (l0.min(l1), l0.max(l1));
                    b > r.0 && b < r.1 && l1 > r.2 && l0 < r.3
                };
                cx.frames
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| !cx.home.contains(k))
                    .filter(|(_, r)| {
                        (t > r.2 && t < r.3 && span.1 > r.0 && span.0 < r.1)
                            || inside(r, bu, u.0 .1, t)
                            || inside(r, bv, t, v.0 .1)
                    })
                    .count()
            };
            let load = cx.load.borrow();
            let over = |k: usize, a: f64, b: f64| {
                let width = (b - a - 2.0 * ROW_CLEAR).max(0.0);
                let room = (width / TRACK_PITCH).floor() as usize + 1;
                let taken = load.get(k).map_or(0, |l| {
                    l.iter()
                        .filter(|s| s.0 < span.1 + TRACK_PITCH && span.0 < s.1 + TRACK_PITCH)
                        .count()
                });
                (taken + 1).saturating_sub(room)
            };
            // The other edges' lines its three legs would cross there (as
            // drawn when the stage began: a long run stepping across in
            // a gap the rest fan through crossed each of them, where the
            // free curve it replaced slipped past most).
            let crossed = |t: f64| {
                let legs = [
                    ((bu, u.0 .1), (bu, t)),
                    ((bu, t), (bv, t)),
                    ((bv, t), (bv, v.0 .1)),
                ];
                cx.lines
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != edge)
                    .map(|(_, l)| {
                        legs.iter()
                            .filter(|&&(a, b)| l.windows(2).any(|w| segs_cross(a, b, w[0], w[1])))
                            .count()
                    })
                    .sum::<usize>()
            };
            let mut best = cands[0];
            let mut key = (
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
                f64::INFINITY,
            );
            for &(k, (a, b)) in &cands {
                let t = (a + b) / 2.0;
                let n = hits(bu, u.0 .1, t) + hits(bv, t, v.0 .1);
                let k2 = (n, foreign(t), crossed(t), over(k, a, b), (t - mid).abs());
                if (k2.0, k2.1, k2.2, k2.3) < (key.0, key.1, key.2, key.3)
                    || (k2.0, k2.1, k2.2, k2.3) == (key.0, key.1, key.2, key.3)
                        && k2.4 < key.4 - EPS
                {
                    key = k2;
                    best = (k, (a, b));
                }
            }
            best
        }
    };
    if let Some(l) = cx.load.borrow_mut().get_mut(gap) {
        l.push((bu.min(bv), bu.max(bv)));
    }
    // Its room: clear of the rows either side, a straight run into each
    // port (a head's worth under a glyph).
    let leg = |port: bool, glyph: bool| match (port, glyph) {
        (true, true) => cx.head_leg,
        (true, false) => BARE_LEG,
        _ => 0.0,
    };
    let lo_in = if lo_pt.1 {
        l0 + leg(true, lo_pt.2)
    } else {
        f64::NEG_INFINITY
    };
    let hi_in = if hi_pt.1 {
        l1 - leg(true, hi_pt.2)
    } else {
        f64::INFINITY
    };
    let row = |g: f64, end: f64| {
        if (g - end).abs() < EPS {
            0.0
        } else {
            ROW_CLEAR
        }
    };
    let mut lo = (g_lo + row(g_lo, l0)).max(lo_in);
    let mut hi = (g_hi - row(g_hi, l1)).min(hi_in);
    if lo > hi {
        // Too shallow for the clearances: the middle of what there is.
        let m = ((g_lo.max(l0) + g_hi.min(l1)) / 2.0).max(l0).min(l1);
        (lo, hi) = (m, m);
    }
    // Where it would go: midway between the ends it joins (where the
    // layout keeps a labelled run's chip and a crossed border's room),
    // midway along the gap for a column.
    let e0 = if lo_pt.1 { l0 } else { g_lo };
    let e1 = if hi_pt.1 { l1 } else { g_hi };
    let mut want = ((e0 + e1) / 2.0).max(lo).min(hi);
    let span = (bu.min(bv), bu.max(bv));
    // In a block layout a step lies between the blocks: one that would lie
    // inside a frame holding neither of its ends moves out of it, to the
    // side of the frame nearer its place, where the room it has keeps it.
    // A gap between two rows is the band's gap and, either side of it,
    // the pad and title strip of the frames closing and opening there; a
    // step placed mid-gap ran along inside the title strip of the block
    // below (dense-td: an edge to a block past the first band, 1 300 px
    // under that band's title). The blocks an end lies in are left alone:
    // a hub's fan steps inside its own block, and a step into the block
    // it enters crosses its border on the straight run. So is a step in a
    // border's own band, which the walls below take off the border: moved
    // out of the frame ahead of its neighbours' steps there, it no longer
    // nested with them (quality/fanout-frames: a hub's fan into six
    // frames and a node past them, one step 1.5 px past a frame's border,
    // crossed its neighbour once moved).
    if cx.blocks {
        for (k, r) in cx.frames.iter().enumerate() {
            let (w0, w1) = (r.2 - BLOCK_BORDER_CLEAR, r.3 + BLOCK_BORDER_CLEAR);
            if cx.home.contains(&k)
                || lo > hi
                || r.1 <= span.0 + EPS
                || r.0 >= span.1 - EPS
                || want <= r.2 + BLOCK_BORDER_CLEAR
                || want >= r.3 - BLOCK_BORDER_CLEAR
            {
                continue;
            }
            let (a, b) = if want - w0 > w1 - want {
                (lo.max(w1), hi)
            } else {
                (lo, hi.min(w0))
            };
            if a <= b {
                (lo, hi) = (a, b);
                want = want.max(lo).min(hi);
            }
        }
    }
    // Off the frames' borders and title chips it passes along.
    for &((b0, b1), (w0, w1)) in cx.walls {
        if b1 <= span.0 + EPS || b0 >= span.1 - EPS || w1 <= lo || w0 >= hi {
            continue;
        }
        if w1 <= want || (w0 < want && want - w0 > w1 - want) {
            lo = lo.max(w1);
        } else {
            hi = hi.min(w0);
        }
    }
    if lo > hi {
        // Hemmed in: back to the plain room's middle.
        lo = (lo + hi) / 2.0;
        hi = lo;
    }
    want = want.max(lo).min(hi);
    // A labelled Z steps across between its source and its label, where
    // there is room (as mermaid draws a labelled branch): the label then
    // rides the straight run into the target instead of the step, and the
    // steps of a labelled fan nest by their source, clear of the chips.
    let mut above_label = false;
    let plain = ((lo, hi), want);
    if let Some((at, half)) = label {
        let (a, b) = if h > 0.0 {
            (lo, hi.min(at - half - LABEL_SEP))
        } else {
            (lo.max(at + half + LABEL_SEP), hi)
        };
        if b >= a {
            (lo, hi, want) = (a, b, (a + b) / 2.0);
            above_label = true;
        }
    }
    let (lo_at, hi_at) = if h > 0.0 { (bu, bv) } else { (bv, bu) };
    Jog {
        edge,
        gap,
        lo_at,
        hi_at,
        room: (lo, hi),
        want,
        at: want,
        above_label,
        plain,
        label_half: label.map_or(0.0, |l| l.1),
        runs: (l0, l1),
    }
}

/// The corners of a labelled twin between two flow sides facing each
/// other along its run (`a`, `b`: point, side normal, glyph; flow
/// coordinates), its twin's line at breadth `twin`: a bulge whose middle
/// run lies `across` (its label's half extent across the flow) and a
/// stroke's air beyond the twin's line, stepping out just past its
/// source and back just short of its target's head — where the builders
/// bowed the twin's curve so its label rode its own line. `None` when
/// its own run lies that far off already (a plain Z draws it) or the gap
/// is too shallow for both steps.
fn twin_bulge(
    a: (Pt, Pt, bool),
    b: (Pt, Pt, bool),
    twin: f64,
    across: f64,
    head_leg: f64,
) -> Option<Vec<Corner>> {
    let ((p0, n0, g0), (p3, n3, g3)) = (a, b);
    let h = n0.1;
    if n0.0 != 0.0 || n3 != (0.0, -h) || h == 0.0 || (p3.1 - p0.1) * h <= 0.0 {
        return None;
    }
    let me = (p0.0 + p3.0) / 2.0;
    let s = if me >= twin { 1.0 } else { -1.0 };
    let run = twin + s * (across + TWIN_AIR);
    if (p0.0 - run) * s >= 0.0 && (p3.0 - run) * s >= 0.0 {
        return None;
    }
    // Each step at least a stair step's merge length out, or the nudge
    // takes it for a micro-jog and draws the twin straight again.
    let step = 2.0 * crate::router::CORNER_R;
    // Ends far apart across the flow: a plain Z's long step holds the
    // label on a track of its own, its legs outside its twin's (a bulge
    // there jogged a few px off its port first, r215's `T0 -> S0`).
    if (p3.0 - p0.0).abs() > 2.0 * (across + step) {
        return None;
    }
    let run = if s > 0.0 {
        run.max(p0.0 + step).max(p3.0 + step)
    } else {
        run.min(p0.0 - step).min(p3.0 - step)
    };
    // (Out of a bare end just a corner's half: the bulge's leg there is
    // a stroke a wide label in the gap must clear.)
    let lead = |glyph: bool| if glyph { head_leg } else { BARE_LEG };
    let (t1, t2) = (p0.1 + h * lead(g0), p3.1 - h * lead(g3));
    if (t2 - t1) * h < 2.0 * crate::router::CORNER_R {
        return None;
    }
    Some(vec![
        Corner::At(p0),
        Corner::At((p0.0, t1)),
        Corner::At((run, t1)),
        Corner::At((run, t2)),
        Corner::At((p3.0, t2)),
        Corner::At(p3),
    ])
}

/// An edge between two breadth sides facing each other (boxes side by
/// side): straight across, or a Z with its jog midway between them.
fn across_flow(p0: Pt, h0: f64, p3: Pt, h3: f64) -> Option<Vec<Corner>> {
    if h0 == 0.0 || h3 != -h0 || (p3.0 - p0.0) * h0 <= 0.0 {
        return None;
    }
    if (p3.1 - p0.1).abs() <= ALIGNED {
        return Some(vec![Corner::At(p0), Corner::At(p3)]);
    }
    let m = (p0.0 + p3.0) / 2.0;
    Some(vec![
        Corner::At(p0),
        Corner::At((m, p0.1)),
        Corner::At((m, p3.1)),
        Corner::At(p3),
    ])
}

/// How far a channel of [`place_tracks`] reaches once a jog meeting the
/// runs at breadths `lo_at` (from the low flow side) and `hi_at` (on to
/// the high side) reaches furthest in it: its span's far end, and
/// whether its run on to the high side leaves from there.
fn channel_reach(lo_at: f64, hi_at: f64) -> (f64, bool) {
    (lo_at.max(hi_at), hi_at >= lo_at)
}

/// Whether the next jog of a gap (by where its span starts; meeting the
/// runs at `lo_at` and `hi_at`) joins the channel reaching `reach`
/// ([`channel_reach`]) in [`place_tracks`]: its span starts within a
/// track's pitch of the channel's, and the two do not only touch where
/// both turn the same way (a fork: two channels).
fn joins_channel(reach: (f64, bool), lo_at: f64, hi_at: f64) -> bool {
    let start = lo_at.min(hi_at);
    let near_hi = hi_at <= lo_at;
    let fork = start >= reach.0 + 1.0 && near_hi == reach.1;
    start < reach.0 + TRACK_PITCH && !fork
}

/// The channels [`place_tracks`] forms of one gap's steps, each given by
/// where it meets the runs from the low flow side and on to the high
/// side (breadth): their sizes, in the order of their first steps. Each
/// step of a channel takes a track of its own, so the largest is the
/// tracks the gap needs (a block layout's room, B12 G2: see
/// `layout::place`).
pub(crate) fn channel_sizes(steps: &[(f64, f64)]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..steps.len()).collect();
    idx.sort_by(|&a, &b| {
        let start = |k: usize| steps[k].0.min(steps[k].1);
        start(a).total_cmp(&start(b)).then(a.cmp(&b))
    });
    let mut sizes = Vec::new();
    let mut s = 0;
    while s < idx.len() {
        let mut reach = channel_reach(steps[idx[s]].0, steps[idx[s]].1);
        let mut t = s + 1;
        while t < idx.len() {
            let (lo_at, hi_at) = steps[idx[t]];
            if !joins_channel(reach, lo_at, hi_at) {
                break;
            }
            if lo_at.max(hi_at) > reach.0 {
                reach = channel_reach(lo_at, hi_at);
            }
            t += 1;
        }
        sizes.push(t - s);
        s = t;
    }
    sizes
}

/// The first column at or past `from` that lies a [`TRACK_PITCH`] or
/// more from every one of `taken` (ascending): a column within a pitch
/// of one moves on to a pitch past it, which can bring it within a
/// pitch of the next. One pass over `taken` in order finds it: the
/// column only moves forward, so a crossing it has passed stays behind
/// it, and once it stands a pitch short of the next crossing it stands
/// clear of every later one. (Searched again from the start after each
/// move until nothing was near, as `TitleSides::of` did, the loop
/// never ended once `o + TRACK_PITCH` rounded back within a pitch of
/// `o`: at a breadth of 1e16 the sum loses the half pixel. The result
/// is the same column wherever that search ended.)
fn column_clear_of(from: f64, taken: &[f64]) -> f64 {
    debug_assert!(taken.windows(2).all(|w| w[0].total_cmp(&w[1]).is_le()));
    let mut col = from;
    for &o in taken {
        if (o - col).abs() < TRACK_PITCH {
            col = o + TRACK_PITCH;
        }
    }
    col
}

/// `group` (a channel's jogs, [`place_tracks`]) split into the sets
/// whose rooms chain, in a block layout (`blocks`, [`Ctx::blocks`]: the
/// static scene's; `route()` and `route_partial()` stack a channel as
/// one set until they carry the block paths, B12.13); else as it is.
///
/// A jog's room here is its plain room ([`Jog::plain`]), the widest
/// [`place_channel`] may give it, not the room above its label it has
/// now: a channel drops its steps back to their plain rooms when one of
/// them is not drawn above its label or their order does not hold, and
/// orders them by where they turn so their legs do not cross. Split by
/// the rooms above their labels, a labelled step left alone in its set
/// kept that room beside an unlabelled one, and two sets were stacked
/// in the order of their rooms instead of their turns: a two-way pair
/// between two blocks crossed, `uncross` swapped its ends and it was
/// drawn as two stepped Zs, one up the target block's wall
/// (blocks/twoway-noted drawn LR; a labelled return between a hub's
/// labelled fan's steps went under the fan's last one, across its
/// leg). Sets whose plain rooms do not chain lie apart whatever their
/// channels do (a frame's border or title strip between them: the jog
/// room's steps and the steps between two stacked frames).
///
/// Two steps chain only where they would stack: their spans meet (within
/// a track's pitch, as a channel's do) and so do their rooms (within the
/// least pitch). A step whose room reaches past a frame's wall, beside
/// the frame, chained the frame's jog room to the gap above it through
/// rooms alone, though it met neither of the steps it chained across:
/// stacked as one set, a displaced entry's step in the jog room ordered
/// among the gap's steps clamped the gap's steps on its far side onto
/// one track at their rooms' end, two long runs on top of each other
/// that the router took round the frame's lateral wall (drawn BT: the
/// exit of a title-row member wholly under its frame's chip, displaced
/// past the chip, then along the gap above the frame on the run of
/// another block's link to a frame further up; see
/// `tests::a_jog_room_step_does_not_chain_a_gaps_steps_onto_one_track`).
fn split_by_room(jogs: &[Jog], group: Vec<usize>, blocks: bool) -> Vec<Vec<usize>> {
    if !blocks || group.len() < 2 {
        return vec![group];
    }
    let room = |k: usize| jogs[k].plain.0;
    let n = group.len();
    let mut root: Vec<usize> = (0..n).collect();
    fn find(root: &mut [usize], mut a: usize) -> usize {
        while root[a] != a {
            root[a] = root[root[a]];
            a = root[a];
        }
        a
    }
    for a in 0..n {
        for b in a + 1..n {
            let (sa, sb) = (jogs[group[a]].span(), jogs[group[b]].span());
            let (ra, rb) = (room(group[a]), room(group[b]));
            let spans = sa.0 < sb.1 + TRACK_PITCH && sb.0 < sa.1 + TRACK_PITCH;
            let rooms = ra.0 < rb.1 + TRACK_PITCH_MIN && rb.0 < ra.1 + TRACK_PITCH_MIN;
            if spans && rooms {
                let (x, y) = (find(&mut root, a), find(&mut root, b));
                root[x.max(y)] = x.min(y);
            }
        }
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut at: Vec<Option<usize>> = vec![None; n];
    for a in 0..n {
        let r = find(&mut root, a);
        match at[r] {
            Some(s) => out[s].push(group[a]),
            None => {
                at[r] = Some(out.len());
                out.push(vec![group[a]]);
            }
        }
    }
    // Each set in the order the jogs had (their gap's, then span's).
    for set in &mut out {
        set.sort_by(|&a, &b| {
            jogs[a]
                .span()
                .0
                .total_cmp(&jogs[b].span().0)
                .then(a.cmp(&b))
        });
    }
    out
}

/// Put one channel's jogs (`group`, two or more) on their tracks
/// ([`place_tracks`]).
fn place_channel(jogs: &mut [Jog], group: Vec<usize>, blocks: bool) {
    let lo = group
        .iter()
        .map(|&k| jogs[k].span().0)
        .fold(f64::INFINITY, f64::min);
    let hi = group
        .iter()
        .map(|&k| jogs[k].span().1)
        .fold(f64::NEG_INFINITY, f64::max);
    // libavoid's key: a jog turns towards the low flow side at the end
    // it meets the run from there, towards the high side at the other.
    let key = |j: &Jog| -> f64 {
        let (a, b) = (j.lo_at, j.hi_at);
        if (a - b).abs() < EPS {
            0.0
        } else if a < b {
            (lo + hi) - (a + b)
        } else {
            (a + b) - (lo + hi)
        }
    };
    let ordered = |jogs: &[Jog]| {
        let mut order = group.clone();
        order.sort_by(|&x, &y| {
            key(&jogs[x])
                .total_cmp(&key(&jogs[y]))
                .then(jogs[x].want.total_cmp(&jogs[y].want))
                .then(jogs[x].edge.cmp(&jogs[y].edge))
                .then(x.cmp(&y))
        });
        order
    };
    // A step drawn above its label nests with the others only when
    // every step of its channel does, and when their rooms hold the
    // channel's order at the least pitch: one under its label among
    // steps in the middle of the gap would leave the order that keeps
    // their turns from crossing, and a room that ends above the
    // place its order asks for clamped two steps onto one track, which
    // the router then rerouted one by one, across each other
    // (x1_hub_td: `H -->|beta| B` and `H -->|gamma long label here| C`
    // crossed twice, the labelled fan's longest label ending its
    // step's room a track above the two before it).
    let mut order = ordered(jogs);
    if blocks {
        part_runs(&mut order, jogs);
    }
    let holds = |order: &[usize], jogs: &[Jog]| {
        let mut last = f64::NEG_INFINITY;
        order.iter().all(|&k| {
            let (lo, hi) = jogs[k].room;
            last = lo.max(last + TRACK_PITCH_MIN);
            last <= hi + EPS
        })
    };
    if group.iter().any(|&k| !jogs[k].above_label) || !holds(&order, jogs) {
        for &k in &group {
            let j = &mut jogs[k];
            (j.room, j.want, j.above_label) = (j.plain.0, j.plain.1, false);
            j.at = j.want;
        }
        order = ordered(jogs);
        if blocks {
            part_runs(&mut order, jogs);
        }
    }
    // The span the channel's tracks share: from the lowest room's
    // start to the highest room's end, each track then kept in its
    // own jog's room.
    let r_lo = order
        .iter()
        .map(|&k| jogs[k].room.0)
        .fold(f64::INFINITY, f64::min);
    let r_hi = order
        .iter()
        .map(|&k| jogs[k].room.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let m = order.len();
    // A step whose run carries its label keeps the label's half and
    // its gap off the tracks beside it, where the channel has room
    // for that (a hub's labelled branch stepped a pitch from its
    // neighbour's run, and the label stage moved the label off its
    // run onto the corner).
    let half = |k: usize| {
        let j = &jogs[k];
        if j.label_half > 0.0 && !j.above_label {
            (j.label_half + LABEL_SEP).max(TRACK_PITCH / 2.0)
        } else {
            TRACK_PITCH / 2.0
        }
    };
    let mut extra: Vec<f64> = vec![0.0; m];
    for r in 1..m {
        extra[r] = extra[r - 1] + half(order[r - 1]) + half(order[r]) - TRACK_PITCH;
    }
    if extra[m - 1] > 0.0 && (r_hi - r_lo) < TRACK_PITCH * (m - 1) as f64 + extra[m - 1] {
        extra.iter_mut().for_each(|x| *x = 0.0);
    }
    let mut step = TRACK_PITCH;
    if (r_hi - r_lo) < step * (m - 1) as f64 {
        step = ((r_hi - r_lo) / (m - 1) as f64).max(TRACK_PITCH_MIN);
    }
    // Too shallow even at the least pitch: the tracks spread evenly
    // about its middle.
    let (r_lo, r_hi) = if (r_hi - r_lo) < step * (m - 1) as f64 {
        let c = (r_lo + r_hi) / 2.0;
        let half = step * (m - 1) as f64 / 2.0;
        (c - half, c + half)
    } else {
        (r_lo, r_hi)
    };
    let want: Vec<f64> = order
        .iter()
        .zip(&extra)
        .map(|(&k, x)| jogs[k].want - x)
        .collect();
    let mut at = crate::ports::place(&want, step, r_lo, r_hi - extra[m - 1]);
    for (a, x) in at.iter_mut().zip(&extra) {
        *a += x;
    }
    // Each in its own room, the order kept where the rooms allow.
    let gap_to = |r: usize| step + extra[r] - extra[r - 1];
    for (r, &k) in order.iter().enumerate() {
        let (lo, hi) = jogs[k].room;
        let floor = if r > 0 { at[r - 1] + gap_to(r) } else { lo };
        at[r] = at[r].max(floor.min(hi)).max(lo).min(hi);
    }
    for r in (0..m.saturating_sub(1)).rev() {
        let (lo, hi) = jogs[order[r]].room;
        at[r] = at[r]
            .min((at[r + 1] - gap_to(r + 1)).max(lo))
            .max(lo)
            .min(hi);
    }
    for (&k, &a) in order.iter().zip(&at) {
        jogs[k].at = a;
    }
}

/// Put each gap's jogs on their tracks: the jogs whose spans overlap
/// (within a track's pitch) form a channel, ordered so their turns do
/// not cross — of two jogs turning the same way, the one turning
/// further out lies away from the turn, as [`crate::router`]'s nudge
/// orders the legs of a channel — and spaced [`TRACK_PITCH`] apart (down
/// to [`TRACK_PITCH_MIN`] in a shallow gap) as near their wanted places
/// as the room they share allows. In a block layout (`blocks`) the order
/// also keeps a step's run on off the run in of the steps after it
/// ([`part_runs`]).
fn place_tracks(jogs: &mut [Jog], kept: &[(f64, (f64, f64))], blocks: bool) {
    let mut idx: Vec<usize> = (0..jogs.len()).collect();
    idx.sort_by(|&a, &b| {
        jogs[a]
            .gap
            .cmp(&jogs[b].gap)
            .then(jogs[a].span().0.total_cmp(&jogs[b].span().0))
            .then(a.cmp(&b))
    });
    let mut s = 0;
    while s < idx.len() {
        // One channel: a run of jogs in one gap whose spans chain.
        // Two that only come within a pitch of each other, both turning
        // the same way at the ends they come close at (a fan's steps to
        // either side of its node, from neighbouring ports), fork and
        // may share a track: they are two channels. (Stacked as one, a
        // dozen-edge hub's two halves of a fan took twice the tracks the
        // gap holds, and the router rerouted the steps squeezed out.)
        let gap = jogs[idx[s]].gap;
        let mut reach = channel_reach(jogs[idx[s]].lo_at, jogs[idx[s]].hi_at);
        let mut t = s + 1;
        while t < idx.len() && jogs[idx[t]].gap == gap {
            let j = &jogs[idx[t]];
            if !joins_channel(reach, j.lo_at, j.hi_at) {
                break;
            }
            if j.span().1 > reach.0 {
                reach = channel_reach(j.lo_at, j.hi_at);
            }
            t += 1;
        }
        let group: Vec<usize> = idx[s..t].to_vec();
        s = t;
        // In a block layout one gap holds a frame's title strip and jog
        // room as well as the space between the frames: a channel's
        // steps whose plain rooms do not meet (a two-way pair's steps
        // between two stacked frames and its displaced steps in the
        // lower frame's jog room) never share a track, and ordered as
        // one stack they interleaved, the jog room's two steps 5 px
        // apart (blocks/t9c-frames-8px-two-way drawn TD), which the
        // router then took round the frames' lateral walls. Each set of
        // steps whose plain rooms chain is a channel of its own there
        // ([`split_by_room`]).
        for group in split_by_room(jogs, group, blocks) {
            if group.len() > 1 {
                place_channel(jogs, group, blocks);
            }
        }
    }
    // Last, off the steps already drawn: a step within a pitch of one
    // whose span meets its own moves to the nearest place in its room
    // clear of them and of the other new steps there (a drag's redrawn
    // edge used to step right on top of a kept one, which the router
    // then rerouted, throwing it 20-120 px).
    if kept.is_empty() {
        return;
    }
    let meets = |a: (f64, f64), b: (f64, f64)| a.0 < b.1 + TRACK_PITCH && b.0 < a.1 + TRACK_PITCH;
    for k in 0..jogs.len() {
        let span = jogs[k].span();
        let near = |t: f64, jogs: &[Jog]| {
            kept.iter()
                .any(|&(l, s)| (l - t).abs() < TRACK_PITCH - EPS && meets(s, span))
                || jogs.iter().enumerate().any(|(m, j)| {
                    m != k
                        && j.gap == jogs[k].gap
                        && (j.at - t).abs() < TRACK_PITCH_MIN - EPS
                        && meets(j.span(), span)
                })
        };
        if !kept
            .iter()
            .any(|&(l, s)| (l - jogs[k].at).abs() < TRACK_PITCH - EPS && meets(s, span))
        {
            continue;
        }
        let (lo, hi) = jogs[k].room;
        let at = jogs[k].at;
        let half = TRACK_PITCH / 2.0;
        let mut found = None;
        for step in 1..=24 {
            for t in [at + step as f64 * half, at - step as f64 * half] {
                if t >= lo - EPS && t <= hi + EPS && !near(t, jogs) {
                    found = Some(t);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        if let Some(t) = found {
            jogs[k].at = t;
        }
    }
}

/// The pairs of a channel's steps, in `order` (by track), where the run
/// on of the earlier lies along the run in of the later: their breadths
/// within a track's pitch. Ordered the other way round the two runs
/// share no stretch of the flow (the earlier step's run on starts past
/// the later one's step). And the crossings among the steps' legs, with
/// the tracks spread over the channel's rooms in that order.
fn channel_cost(order: &[usize], jogs: &[Jog]) -> (usize, usize) {
    let m = order.len();
    let lo = order
        .iter()
        .map(|&k| jogs[k].room.0)
        .fold(f64::INFINITY, f64::min);
    let hi = order
        .iter()
        .map(|&k| jogs[k].room.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let t = |r: usize| lo + (hi - lo) * (r as f64 + 0.5) / m as f64;
    let step = |k: usize| (jogs[k].hi_at - jogs[k].lo_at).abs() >= EPS;
    let (mut along, mut cross) = (0, 0);
    for (r, &a) in order.iter().enumerate() {
        for (s, &b) in order.iter().enumerate().skip(r + 1) {
            if jogs[a].edge == jogs[b].edge {
                continue;
            }
            if step(a) && step(b) && (jogs[a].hi_at - jogs[b].lo_at).abs() < TRACK_PITCH - EPS {
                along += 1;
            }
            let (la, lb) = (jog_legs(&jogs[a], t(r)), jog_legs(&jogs[b], t(s)));
            cross += la
                .iter()
                .map(|&(p, q)| lb.iter().filter(|&&(u, v)| segs_cross(p, q, u, v)).count())
                .sum::<usize>();
        }
    }
    (along, cross)
}

/// A block layout's channel in the order that keeps a step's run on off
/// the run in of a later step ([`channel_cost`]): such a step goes right
/// after the later one when that lays fewer runs along each other and
/// crosses no more legs. The key of [`place_tracks`] orders by where
/// each step turns; a step turning back across the channel whose run on
/// ends within a pitch of where another step's run in comes from was
/// placed first and ran along that run to its node (bench-75 as blocks:
/// the turned-up `svc3 -> metrics` 5 px from `svc5 -> queue` and `svc5
/// -> objects` over 110-150 px), and the router took the two off it
/// round svc5's corner, across their fan.
fn part_runs(order: &mut Vec<usize>, jogs: &[Jog]) {
    let m = order.len();
    if m < 2 {
        return;
    }
    let mut now = channel_cost(order, jogs);
    // Each kept move lowers the runs laid along each other: it ends.
    while now.0 > 0 {
        let mut kept = false;
        'pairs: for r in 0..m {
            for s in r + 1..m {
                let (a, b) = (order[r], order[s]);
                let along = (jogs[a].hi_at - jogs[b].lo_at).abs() < TRACK_PITCH - EPS;
                if !along || jogs[a].edge == jogs[b].edge {
                    continue;
                }
                // The earlier step right after the later one, or the
                // later one right before it.
                for later in [false, true] {
                    let mut next = order.clone();
                    if later {
                        let k = next.remove(s);
                        next.insert(r, k);
                    } else {
                        let k = next.remove(r);
                        next.insert(s, k);
                    }
                    let cost = channel_cost(&next, jogs);
                    if cost.0 < now.0 && cost.1 <= now.1 + (now.0 - cost.0) {
                        (*order, now, kept) = (next, cost, true);
                        break 'pairs;
                    }
                }
            }
        }
        if !kept {
            break;
        }
    }
}

/// The three legs of jog `j` with its step at `t` (flow coordinates):
/// the run in from the low flow side, the step, the run on.
fn jog_legs(j: &Jog, t: f64) -> [(Pt, Pt); 3] {
    [
        ((j.lo_at, j.runs.0), (j.lo_at, t)),
        ((j.lo_at, t), (j.hi_at, t)),
        ((j.hi_at, t), (j.hi_at, j.runs.1)),
    ]
}

/// Whether segments `a`-`b` and `c`-`d` properly cross.
fn segs_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let side = |p: Pt, q: Pt, r: Pt| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Off the curves this stage leaves ([`orthogonalize`]'s `kept_lines`:
/// a note card's link, an author's `curve:`) and off each other: a jog
/// whose legs cross one of those lines or another jog's legs moves to
/// the place in its room where its legs cross the fewest of them,
/// nearest where it was, never within [`TRACK_PITCH_MIN`] of another
/// step of its gap it overlaps. (A branch from a state to the row below
/// its note card stepped down across the gap mid-way and crossed the
/// card's link, which its curve had passed under; a hub's branch to the
/// far box of a frame stepped below its neighbour's step, its label's
/// room squeezing the channel's order, and crossed its leg.)
fn uncross(jogs: &mut [Jog], lines: &[Vec<Pt>]) {
    if jogs.is_empty() {
        return;
    }
    // A jog's extent with its step at `t` (breadth lo, hi, flow lo, hi).
    let extent = |j: &Jog, t: f64| {
        let (b0, b1) = j.span();
        let (l0, l1) = (j.runs.0.min(j.runs.1).min(t), j.runs.0.max(j.runs.1).max(t));
        (b0, b1, l0, l1)
    };
    let meets = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
        a.0 <= b.1 && b.0 <= a.1 && a.2 <= b.3 && b.2 <= a.3
    };
    let line_box: Vec<(f64, f64, f64, f64)> = lines
        .iter()
        .map(|l| {
            l.iter().fold(
                (
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                ),
                |r, p| (r.0.min(p.0), r.1.max(p.0), r.2.min(p.1), r.3.max(p.1)),
            )
        })
        .collect();
    let hits = |jogs: &[Jog], k: usize, t: f64| -> usize {
        let legs = jog_legs(&jogs[k], t);
        let me = extent(&jogs[k], t);
        let on_lines = lines
            .iter()
            .zip(&line_box)
            .filter(|(_, b)| meets(me, **b))
            .map(|(l, _)| {
                legs.iter()
                    .filter(|&&(a, b)| l.windows(2).any(|w| segs_cross(a, b, w[0], w[1])))
                    .count()
            })
            .sum::<usize>();
        let on_jogs = jogs
            .iter()
            .enumerate()
            .filter(|&(m, o)| m != k && o.edge != jogs[k].edge && meets(me, extent(o, o.at)))
            .map(|(_, o)| {
                let other = jog_legs(o, o.at);
                legs.iter()
                    .filter(|&&(a, b)| other.iter().any(|&(c, d)| segs_cross(a, b, c, d)))
                    .count()
            })
            .sum::<usize>();
        on_lines + on_jogs
    };
    // Within a least pitch of another step of its gap that it overlaps.
    let crowds = |jogs: &[Jog], k: usize, t: f64| {
        let (s0, s1) = jogs[k].span();
        jogs.iter().enumerate().any(|(m, o)| {
            let (o0, o1) = o.span();
            m != k
                && o.gap == jogs[k].gap
                && o0 < s1 + TRACK_PITCH
                && s0 < o1 + TRACK_PITCH
                && (o.at - t).abs() < TRACK_PITCH_MIN - EPS
        })
    };
    for k in 0..jogs.len() {
        let at = jogs[k].at;
        let now = hits(jogs, k, at);
        if now == 0 {
            continue;
        }
        let (lo, hi) = jogs[k].room;
        let mut best = (now, 0.0f64, at);
        let n = ((hi - lo) / (TRACK_PITCH / 2.0)).ceil().clamp(1.0, 64.0) as usize;
        for step in 0..=n {
            let t = lo + (hi - lo) * step as f64 / n as f64;
            if crowds(jogs, k, t) {
                continue;
            }
            let h = hits(jogs, k, t);
            let d = (t - at).abs();
            if h < best.0 || h == best.0 && d < best.1 - EPS {
                best = (h, d, t);
            }
        }
        jogs[k].at = best.2;
    }
}

/// Drop repeated points and the middle of three on one axis-aligned
/// line.
fn collapse(pts: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out
            .last()
            .is_some_and(|&q: &Pt| (p.0 - q.0).abs() < EPS && (p.1 - q.1).abs() < EPS)
        {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let same_x = (a.0 - b.0).abs() < EPS && (b.0 - p.0).abs() < EPS;
            let same_y = (a.1 - b.1).abs() < EPS && (b.1 - p.1).abs() < EPS;
            if same_x || same_y {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

/// Move `e`'s label with its run, keeping the offset the builder gave it
/// from its curve's middle: on a Z, to the middle of the step — or, when
/// `rides` (the Z's flow direction) says the step lies between the
/// source and the label, onto the run into the target, the label's spot
/// along the flow kept; on a longer path, onto the nearest point of the
/// new path.
fn relabel(e: &mut SceneEdge, corners: &[Pt], rides: Option<Direction>) {
    let Some((_, at, _)) = e.label.as_mut() else {
        return;
    };
    let old = if e.waypoints.len() >= 2 {
        crate::labels::channel_label_point(&e.waypoints)
    } else {
        let b = &e.bezier;
        (
            (b[0].0 + 3.0 * b[1].0 + 3.0 * b[2].0 + b[3].0) / 8.0,
            (b[0].1 + 3.0 * b[1].1 + 3.0 * b[2].1 + b[3].1) / 8.0,
        )
    };
    let new = match (corners.len(), rides) {
        (4, Some(dir)) => {
            // Onto the target's run, where it lay along the flow (the
            // builders' stacking of a fan's labels kept), whatever offset
            // across the flow kept it off the old curve's neighbours.
            let (last, me) = (to_flow(dir, corners[3]), to_flow(dir, *at));
            *at = from_flow(dir, (last.0, me.1));
            return;
        }
        (4, None) => {
            let (a, b) = (corners[1], corners[2]);
            ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
        }
        _ => nearest_on(corners, old),
    };
    *at = (at.0 + new.0 - old.0, at.1 + new.1 - old.1);
    // A label a stage before nudged a px or two off its curve (the ports
    // stage moves a curve's label by its ends' mean move) lands on the
    // new line, as one on its curve does; one set beside its line on
    // purpose (a fan's stacked rows) keeps its offset.
    let on = nearest_on(corners, *at);
    if (on.0 - at.0).hypot(on.1 - at.1) < LABEL_SNAP {
        *at = on;
    }
}

/// Pitch (px) of the spots along a twin's own legs its label tries, and
/// the most it tries per leg and slide (see [`orthogonalize`]).
const SPOT_STEP: f64 = 4.0;
const SPOT_MAX: usize = 48;

/// How far (px) off its new line [`relabel`] still sets a label onto it.
const LABEL_SNAP: f64 = 2.0;

/// The point of polyline `pts` nearest `p`.
fn nearest_on(pts: &[Pt], p: Pt) -> Pt {
    let mut best = (f64::INFINITY, pts[0]);
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l2 = dx * dx + dy * dy;
        let t = if l2 < EPS {
            0.0
        } else {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
        };
        let q = (a.0 + t * dx, a.1 + t * dy);
        let d = (q.0 - p.0).hypot(q.1 - p.1);
        if d < best.0 {
            best = (d, q);
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::scene::scene;

    #[test]
    fn a_displaced_column_is_found_in_one_pass_and_always_ends() {
        // The search it replaces: from the start again after every move,
        // until no crossing is within a pitch (bounded here; unbounded it
        // never ended once `o + TRACK_PITCH` rounded back within a pitch
        // of `o`).
        let search = |from: f64, taken: &[f64]| {
            let mut col = from;
            for _ in 0..10_000 {
                match taken.iter().find(|&&o| (o - col).abs() < TRACK_PITCH) {
                    Some(&o) => col = o + TRACK_PITCH,
                    None => return Some(col),
                }
            }
            None
        };
        // Deterministic draws of up to eight crossings near the start.
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut draw = || {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (x >> 33) as f64 / (1u64 << 31) as f64
        };
        for _ in 0..2000 {
            let from = 100.0 + 50.0 * draw();
            let n = (draw() * 9.0) as usize;
            let mut taken: Vec<f64> = (0..n).map(|_| 80.0 + 120.0 * draw()).collect();
            let want = search(from, &taken).expect("the old search ends here");
            taken.sort_by(f64::total_cmp);
            let got = column_clear_of(from, &taken);
            assert_eq!(got.to_bits(), want.to_bits(), "{from} {taken:?}");
            assert!(got >= from && taken.iter().all(|&o| (o - got).abs() >= TRACK_PITCH));
        }
        // Where a pitch is lost to rounding the old search went round
        // for ever; one pass ends past the crossing.
        let o = 1e16;
        assert!((o + TRACK_PITCH) - o < TRACK_PITCH);
        assert_eq!(search(o, &[o]), None);
        let col = column_clear_of(o, &[o - 4.0, o]);
        assert!(col > o);
    }

    const DIRS: [Direction; 4] = [Direction::TD, Direction::BT, Direction::LR, Direction::RL];

    /// A jog of `edge` in gap `gap` from breadth `lo_at` (where the run
    /// from the low flow side meets it) to `hi_at`, free in `room`.
    fn jog(edge: usize, gap: usize, lo_at: f64, hi_at: f64, room: (f64, f64), want: f64) -> Jog {
        Jog {
            edge,
            gap,
            lo_at,
            hi_at,
            room,
            want,
            at: want,
            above_label: false,
            plain: (room, want),
            label_half: 0.0,
            runs: (room.0 - 20.0, room.1 + 20.0),
        }
    }

    #[test]
    fn a_drawing_past_the_routers_cap_keeps_the_builders_curves() {
        // 20 sources each linked to 21 targets: 420 edges, past
        // `ROUTE_MAX_EDGES`, where the router leaves a drawing as built.
        let mut flat = String::from("flowchart TD\n");
        for a in 0..20 {
            for b in 0..21 {
                flat.push_str(&format!("s{a} --> t{b}\n"));
            }
        }
        // 19 x 21 = 399 edges inside a box and two from the box to its
        // own members: 401 drawn, though the layout graph, which holds
        // no stand-in for an edge from a box to its own member, has 399.
        let mut boxed = String::from("flowchart TD\nsubgraph K\n");
        for a in 0..19 {
            for b in 0..21 {
                boxed.push_str(&format!("s{a} --> t{b}\n"));
            }
        }
        boxed.push_str("end\nK --> s0\nK --> t0\n");
        for src in [flat, boxed] {
            let g = parse(&src).unwrap();
            let sc = scene(&g);
            assert!(sc.edges.len() > crate::router::ROUTE_MAX_EDGES);
            // Nor does the layout grow the gap for tracks no edge takes:
            // the rows sit the plain gap apart (0.37.0's depth), not the
            // 2 650 px (`flat`, 210 tracks at the busiest point) or
            // 2 525 px (`boxed`, 200) the axes would take.
            let row_gap = {
                let span = |p: &str| {
                    sc.nodes
                        .iter()
                        .filter(|n| n.id.starts_with(p))
                        .map(|n| (n.y - n.h / 2.0, n.y + n.h / 2.0))
                        .fold((f64::MAX, f64::MIN), |a, b| (a.0.min(b.0), a.1.max(b.1)))
                };
                span("t").0 - span("s").1
            };
            assert!(
                row_gap < crate::layout::GAP_L + 2.0 * TRACK_PITCH,
                "rows {row_gap} px apart past the cap"
            );
            let pos: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            for s in [sc, crate::scene::route(&g, &pos)] {
                let squared = s
                    .edges
                    .iter()
                    .filter(|e| crate::router::ortho_corners(e).is_some_and(|c| c.len() >= 3))
                    .count();
                // (Only the few S-cubics a rounding or two across, which
                // the last stage squares whatever the size, come out as
                // Zs.)
                assert!(
                    squared * 20 < s.edges.len(),
                    "{squared} of {} edges drawn on the axes past the cap",
                    s.edges.len()
                );
            }
        }
    }

    /// Whether some leg of jog `a` properly crosses some leg of `b`.
    fn jogs_cross(a: &Jog, b: &Jog) -> bool {
        let (la, lb) = (jog_legs(a, a.at), jog_legs(b, b.at));
        la.iter()
            .any(|&(p, q)| lb.iter().any(|&(r, s)| segs_cross(p, q, r, s)))
    }

    #[test]
    fn flow_coordinates_run_downstream_in_every_direction() {
        let p = (12.5, -40.0);
        for dir in DIRS {
            assert_eq!(from_flow(dir, to_flow(dir, p)), p, "{dir:?}");
        }
        // One step downstream (TD down, BT up, LR right, RL left) is +1
        // along the flow, and the side it leaves by faces +1 along it.
        let steps = [
            (Direction::TD, (0.0, 1.0), Side::Bottom),
            (Direction::BT, (0.0, -1.0), Side::Top),
            (Direction::LR, (1.0, 0.0), Side::Right),
            (Direction::RL, (-1.0, 0.0), Side::Left),
        ];
        for (dir, step, side) in steps {
            assert_eq!(to_flow(dir, step).1, 1.0, "{dir:?}");
            assert_eq!(flow_normal(dir, side), (0.0, 1.0), "{dir:?}");
        }
        assert_eq!(
            flow_rect(Direction::BT, (0.0, 0.0, 10.0, 20.0)),
            (0.0, 10.0, -20.0, 0.0)
        );
        assert_eq!(
            flow_rect(Direction::LR, (0.0, 0.0, 10.0, 20.0)),
            (0.0, 20.0, 0.0, 10.0)
        );
    }

    #[test]
    fn segment_tests_count_proper_crossings_only() {
        assert!(segs_cross(
            (0.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (10.0, 0.0)
        ));
        // Touching at an end, collinear, or apart: no crossing.
        assert!(!segs_cross(
            (0.0, 0.0),
            (10.0, 0.0),
            (5.0, 0.0),
            (5.0, 10.0)
        ));
        assert!(!segs_cross((0.0, 0.0), (10.0, 0.0), (2.0, 0.0), (8.0, 0.0)));
        assert!(!segs_cross(
            (0.0, 0.0),
            (10.0, 0.0),
            (0.0, 1.0),
            (10.0, 1.0)
        ));
        let r = (0.0, 0.0, 10.0, 10.0);
        assert!(seg_hits_rect((-5.0, 5.0), (15.0, 5.0), r));
        assert!(seg_hits_rect((5.0, 5.0), (5.0, 50.0), r));
        assert!(!seg_hits_rect((-5.0, -5.0), (-1.0, 20.0), r));
    }

    #[test]
    fn collapse_drops_repeats_and_points_on_a_straight_leg() {
        let pts = vec![
            (0.0, 0.0),
            (0.0, 0.0),
            (0.0, 5.0),
            (0.0, 10.0),
            (5.0, 10.0),
            (9.0, 10.0),
            (9.0, 20.0),
        ];
        assert_eq!(
            collapse(pts),
            vec![(0.0, 0.0), (0.0, 10.0), (9.0, 10.0), (9.0, 20.0)]
        );
        assert_eq!(
            nearest_on(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], (12.0, 5.0)),
            (10.0, 5.0)
        );
    }

    #[test]
    fn boxes_side_by_side_join_straight_or_by_a_midway_z() {
        let flat = across_flow((0.0, 0.0), 1.0, (20.0, 0.3), -1.0).unwrap();
        assert_eq!(flat.len(), 2);
        let z = across_flow((0.0, 0.0), 1.0, (20.0, 6.0), -1.0).unwrap();
        let at: Vec<Pt> = z
            .iter()
            .map(|c| match *c {
                Corner::At(p) => p,
                Corner::Jog(..) => unreachable!(),
            })
            .collect();
        assert_eq!(at, vec![(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (20.0, 6.0)]);
        // Sides not facing each other, or the target behind: not this
        // shape.
        assert!(across_flow((0.0, 0.0), 1.0, (20.0, 6.0), 1.0).is_none());
        assert!(across_flow((0.0, 0.0), 1.0, (-20.0, 6.0), -1.0).is_none());
    }

    #[test]
    fn a_labelled_twin_bulges_past_its_twin_or_not_at_all() {
        let (across, head_leg) = (20.0, HEAD_LEG);
        let a = ((0.0, 0.0), (0.0, 1.0), false);
        let b = ((0.0, 100.0), (0.0, -1.0), true);
        let plan = twin_bulge(a, b, 10.0, across, head_leg).expect("a bulge");
        let pts: Vec<Pt> = plan
            .iter()
            .map(|c| match *c {
                Corner::At(p) => p,
                Corner::Jog(..) => unreachable!(),
            })
            .collect();
        assert_eq!(pts.len(), 6);
        // Out past its twin's line by the label's half and a stroke's
        // air, on its own side, a corner's half out of its bare source
        // and a head's run short of its target.
        let run = pts[2].0;
        assert!(run <= 10.0 - across - TWIN_AIR + EPS, "{pts:?}");
        assert_eq!(pts[1], (0.0, BARE_LEG));
        assert_eq!(pts[4], (0.0, 100.0 - head_leg));
        for w in pts.windows(2) {
            assert!(w[0].0 == w[1].0 || w[0].1 == w[1].1, "{pts:?}");
        }
        // Its own run already that far off its twin: a plain Z draws it.
        assert!(twin_bulge(a, b, 100.0, across, head_leg).is_none());
        // A gap too shallow for both steps.
        let near = ((0.0, 40.0), (0.0, -1.0), true);
        assert!(twin_bulge(a, near, 10.0, across, head_leg).is_none());
    }

    #[test]
    fn a_staircase_of_steps_is_one_channel_a_track_each() {
        // A hub's fan stepping one way under its block: each step
        // overlaps the next only, so no point lies under more than two,
        // but their order nests all of them: one channel, a track a
        // step (B12 G2). The layout reserves its gap by that count.
        let stair: Vec<(f64, f64)> = (0..6)
            .map(|k| (300.0 + 40.0 * k as f64, 220.0 + 40.0 * k as f64))
            .collect();
        assert_eq!(channel_sizes(&stair), vec![6]);
        let mut jogs: Vec<Jog> = stair
            .iter()
            .enumerate()
            .map(|(k, &(a, b))| jog(k, 0, a, b, (0.0, 200.0), 100.0))
            .collect();
        place_tracks(&mut jogs, &[], false);
        let mut at: Vec<f64> = jogs.iter().map(|j| j.at).collect();
        at.sort_by(f64::total_cmp);
        assert!(
            at.windows(2).all(|w| w[1] - w[0] >= TRACK_PITCH - EPS),
            "{at:?}"
        );
        // Steps a pitch and more apart are channels of their own, and
        // two that only meet where both turn the same way fork.
        assert_eq!(
            channel_sizes(&[(0.0, 100.0), (130.0, 230.0), (250.0, 330.0)]),
            vec![1, 1, 1]
        );
        assert_eq!(channel_sizes(&[(100.0, 0.0), (101.0, 200.0)]), vec![1, 1]);
        assert_eq!(channel_sizes(&[]), Vec::<usize>::new());
    }

    #[test]
    fn a_jog_room_step_does_not_chain_a_gaps_steps_onto_one_track() {
        // One gap's channel (flow coordinates, synthetic): steps 2 to 5
        // run along the space between two stacked frames, their rooms
        // from 100; step 1 is a displaced exit's step in the lower
        // frame's jog room under its title strip (room 10 .. 28), and
        // step 2 the same edge's run on along the space between the
        // frames; step 0 runs beside the lower frame, its room reaching
        // from that space down past the frame's wall to the jog room's
        // breadths, its span apart from step 1's. Step 0 meets the jog
        // room's step only through its room. Chained through it into one
        // set, step 1 sat among the other steps in their order and
        // clamped two of them onto one track at their rooms' end.
        let fresh = || {
            vec![
                jog(0, 0, 600.0, 200.0, (15.0, 160.0), 95.0),
                jog(1, 0, 900.0, 1100.0, (10.0, 28.0), 28.0),
                jog(1, 0, 1100.0, 220.0, (100.0, 160.0), 130.0),
                jog(2, 0, 2000.0, 240.0, (100.0, 150.0), 100.0),
                jog(3, 0, 500.0, 1110.0, (100.0, 175.0), 104.0),
                jog(4, 0, 2300.0, 1500.0, (100.0, 165.0), 100.0),
            ]
        };
        // The jog room's step is a set of its own; the others are one.
        let mut sets = split_by_room(&fresh(), (0..6).collect(), true);
        sets.iter_mut().for_each(|s| s.sort_unstable());
        sets.sort();
        assert_eq!(sets, vec![vec![0, 2, 3, 4, 5], vec![1]]);
        let mut jogs = fresh();
        place_tracks(&mut jogs, &[], true);
        for (a, x) in jogs.iter().enumerate() {
            for (b, y) in jogs.iter().enumerate().skip(a + 1) {
                let (sx, sy) = (x.span(), y.span());
                if x.edge != y.edge && sx.0 < sy.1 && sy.0 < sx.1 && x.room.1 > y.room.0 {
                    assert!(
                        (x.at - y.at).abs() >= TRACK_PITCH_MIN - EPS,
                        "steps {a} and {b} on one track: {} / {}",
                        x.at,
                        y.at
                    );
                }
            }
        }
        // The jog room's step keeps its place; every step its room.
        assert_eq!(jogs[1].at, 28.0);
        for j in &jogs {
            assert!(j.at >= j.room.0 - EPS && j.at <= j.room.1 + EPS);
        }
        // Outside a block layout the six are one channel stacked as one
        // set, as before: what `place_channel` gives the whole group.
        assert_eq!(
            split_by_room(&fresh(), (0..6).collect(), false),
            vec![(0..6).collect::<Vec<usize>>()]
        );
        let mut flat = fresh();
        place_tracks(&mut flat, &[], false);
        let mut one = fresh();
        let mut group: Vec<usize> = (0..6).collect();
        group.sort_by(|&a, &b| one[a].span().0.total_cmp(&one[b].span().0));
        place_channel(&mut one, group, false);
        for (k, (f, o)) in flat.iter().zip(&one).enumerate() {
            assert_eq!(
                f.at.to_bits(),
                o.at.to_bits(),
                "step {k}: {} / {}",
                f.at,
                o.at
            );
        }
        assert!(flat.iter().zip(&jogs).any(|(f, j)| f.at != j.at));
    }

    #[test]
    fn a_turned_step_keeps_its_run_on_off_a_later_steps_run_in() {
        // Two steps of a node's fan go down from ports at 180 and 190; a
        // step from below turns up to end at 185, between them. Ordered
        // by where they turn, the turned step comes first and its run on
        // lies 5 px from both runs in, all the way to their steps. In a
        // block layout it goes after them: as many crossings, and no run
        // along another.
        let fresh = || {
            vec![
                jog(0, 0, 190.0, 415.0, (0.0, 300.0), 150.0),
                jog(1, 0, 180.0, 292.0, (0.0, 300.0), 150.0),
                jog(2, 0, 388.0, 185.0, (0.0, 300.0), 150.0),
            ]
        };
        let along = |jogs: &[Jog]| {
            let mut n = 0;
            for a in jogs {
                for b in jogs {
                    let near = (a.hi_at - b.lo_at).abs() < TRACK_PITCH;
                    n += usize::from(a.edge != b.edge && near && a.at < b.at);
                }
            }
            n
        };
        let mut before = fresh();
        place_tracks(&mut before, &[], false);
        assert_eq!(along(&before), 2, "the key alone");
        let mut jogs = fresh();
        place_tracks(&mut jogs, &[], true);
        assert_eq!(along(&jogs), 0);
        assert!(jogs[2].at > jogs[0].at && jogs[2].at > jogs[1].at);
        let order: Vec<usize> = (0..3).collect();
        let by_at = |jogs: &[Jog]| {
            let mut o = order.clone();
            o.sort_by(|&a, &b| jogs[a].at.total_cmp(&jogs[b].at));
            o
        };
        assert!(
            channel_cost(&by_at(&jogs), &jogs).1 <= channel_cost(&by_at(&before), &before).1,
            "no more crossings"
        );
    }

    #[test]
    fn a_channels_tracks_keep_a_pitch_apart_and_never_cross() {
        // Two steps turning the same way across one gap, one starting
        // further out: libavoid's order puts the one turning further out
        // away from the turn, so neither crosses the other's leg.
        let mut jogs = vec![
            jog(0, 0, 0.0, 100.0, (30.0, 70.0), 50.0),
            jog(1, 0, 20.0, 120.0, (30.0, 70.0), 50.0),
        ];
        place_tracks(&mut jogs, &[], false);
        for (i, a) in jogs.iter().enumerate() {
            assert!(
                a.at >= a.room.0 - EPS && a.at <= a.room.1 + EPS,
                "{i}: {}",
                a.at
            );
            for b in &jogs[i + 1..] {
                assert!(
                    (a.at - b.at).abs() >= TRACK_PITCH_MIN - EPS,
                    "{} {}",
                    a.at,
                    b.at
                );
                assert!(!jogs_cross(a, b), "{} {}", a.at, b.at);
            }
        }
        // One turning the other way across them crosses one of them
        // whatever the order (two Zs facing each other): it still keeps
        // a track of its own.
        let mut facing = vec![
            jog(0, 0, 0.0, 100.0, (30.0, 70.0), 50.0),
            jog(1, 0, 140.0, 40.0, (30.0, 70.0), 50.0),
        ];
        place_tracks(&mut facing, &[], false);
        assert!((facing[0].at - facing[1].at).abs() >= TRACK_PITCH_MIN - EPS);
        // A step in another gap is its own channel.
        let mut alone = vec![
            jog(0, 0, 0.0, 100.0, (30.0, 70.0), 50.0),
            jog(1, 1, 20.0, 120.0, (30.0, 70.0), 50.0),
        ];
        place_tracks(&mut alone, &[], false);
        assert_eq!((alone[0].at, alone[1].at), (50.0, 50.0));
        // Off a kept step a pitch away where its room allows.
        let mut off = vec![jog(0, 0, 0.0, 100.0, (30.0, 70.0), 50.0)];
        place_tracks(&mut off, &[(50.0, (10.0, 90.0))], false);
        assert!(
            (off[0].at - 50.0).abs() >= TRACK_PITCH - EPS,
            "{}",
            off[0].at
        );
    }

    #[test]
    fn a_channel_whose_label_rooms_cannot_hold_its_order_steps_under_its_labels() {
        // B10 review r2 #1 (x1_hub_td): three steps of a labelled fan
        // turning the same way, each above its label; the longest label
        // ends the last step's room a track above where the order puts
        // it. Clamped into those rooms two steps shared a track and the
        // router rerouted them across each other; under their labels
        // (their plain rooms) they keep the order, a pitch apart.
        let above = |edge: usize, lo_at: f64, hi_at: f64, room: (f64, f64)| {
            let mut j = jog(edge, 0, lo_at, hi_at, room, (room.0 + room.1) / 2.0);
            j.above_label = true;
            j.plain = ((105.0, 167.0), 140.0);
            j.label_half = 10.0;
            // (From the hub's side down to the targets' row.)
            j.runs = (80.0, 215.0);
            j
        };
        let mut jogs = vec![
            above(0, 371.0, 110.0, (105.0, 145.0)),
            above(1, 383.0, 240.0, (105.0, 167.0)),
            above(2, 396.0, 346.0, (99.6, 108.8)),
        ];
        place_tracks(&mut jogs, &[], false);
        for (i, a) in jogs.iter().enumerate() {
            assert!(!a.above_label, "{i}");
            assert!(a.at >= 105.0 - EPS && a.at <= 167.0 + EPS, "{i}: {}", a.at);
            for b in &jogs[i + 1..] {
                assert!(
                    (a.at - b.at).abs() >= TRACK_PITCH_MIN - EPS,
                    "{} {}",
                    a.at,
                    b.at
                );
                assert!(!jogs_cross(a, b), "{} {}", a.at, b.at);
            }
        }
        // (The one turning nearest its side, the furthest across, first.)
        assert!(jogs[0].at < jogs[1].at && jogs[1].at < jogs[2].at);
        // Rooms that hold the order keep the steps above their labels.
        let mut fits = vec![
            above(0, 371.0, 110.0, (105.0, 145.0)),
            above(1, 383.0, 240.0, (105.0, 167.0)),
        ];
        place_tracks(&mut fits, &[], false);
        assert!(fits.iter().all(|j| j.above_label));
    }

    /// Pairs of edges out of one node (or into one) whose drawn lines
    /// cross twice or more, as "a b" ids.
    fn double_crossings(sc: &Scene) -> Vec<String> {
        let lines: Vec<Vec<Pt>> = sc.edges.iter().map(crate::scene::edge_polyline).collect();
        let mut out = Vec::new();
        for i in 0..sc.edges.len() {
            for j in i + 1..sc.edges.len() {
                let (a, b) = (&sc.edges[i], &sc.edges[j]);
                if a.from == a.to || b.from == b.to || (a.from != b.from && a.to != b.to) {
                    continue;
                }
                if lines_cross(&lines[i], &lines[j]) >= 2 {
                    out.push(format!("{}->{} {}->{}", a.from, a.to, b.from, b.to));
                }
            }
        }
        out
    }

    #[test]
    fn steps_out_of_one_side_turning_one_way_never_cross_twice() {
        // B10 review r2 #1. x1_hub_td: `H -->|beta| B` and `H -->|gamma
        // long label here| C` crossed twice in `scene()` (the label rooms
        // above; `route()` drew them nested). y2_state_nested: `Off -->
        // decide`'s L ran round the outside of `Off --> On`'s Z and crossed
        // it twice; the ends swap places (`untangle`) and cross once.
        let hub = "flowchart TD\n    H{Route?} -->|alpha| A[Alpha service]\n    \
                   H -->|beta| B[Beta]\n    H -->|gamma long label here| C[Gamma]\n    \
                   H --> D[Delta]\n    H -.->|epsilon| E[(Epsilon store)]\n    H ==> F[Phi]\n    \
                   A --> Z[Sink]\n    B --> Z\n    C --> Z\n    D --> Z\n    E --> Z\n    \
                   F --> Z\n    Start([Start]) --> H\n    Start --> Z\n    Z -->|retry| Start\n    \
                   subgraph grp [Group]\n      B\n      C\n    end\n";
        let state = "stateDiagram-v2\n    [*] --> Off\n    Off --> On : power\n    state On {\n        \
                     direction LR\n        [*] --> Idle\n        Idle --> Busy : job\n        \
                     Busy --> Idle : done\n        state Busy {\n            [*] --> Fetch\n            \
                     Fetch --> Exec\n            Exec --> Write\n            Write --> [*]\n        }\n        \
                     Busy --> Fault : error\n        Fault --> Idle : clear\n    }\n    \
                     state fork_state <<fork>>\n    On --> fork_state\n    fork_state --> LogA\n    \
                     fork_state --> LogB\n    state join_state <<join>>\n    LogA --> join_state\n    \
                     LogB --> join_state\n    join_state --> Off\n    On --> Off : shutdown\n    \
                     note right of Off : cold start\n    state decide <<choice>>\n    Off --> decide\n    \
                     decide --> On : warm\n    decide --> [*] : cold\n";
        for src in [hub, state] {
            let g = match crate::parser::parse_document(src).unwrap() {
                crate::model::Document::Flowchart(g) | crate::model::Document::State(g) => g,
                _ => unreachable!(),
            };
            let sc = scene(&g);
            assert_eq!(double_crossings(&sc), Vec::<String>::new());
            let at: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            assert_eq!(
                double_crossings(&crate::scene::route(&g, &at)),
                Vec::<String>::new()
            );
        }
        let g = parse(hub).unwrap();
        let sc = scene(&g);
        let line = |to: &str| {
            let e = sc
                .edges
                .iter()
                .find(|e| e.from == "H" && e.to == to)
                .unwrap();
            crate::scene::edge_polyline(e)
        };
        assert_eq!(lines_cross(&line("B"), &line("C")), 0);
    }

    #[test]
    fn a_fan_off_a_diamonds_corners_steps_clear_of_its_rows_note_cards() {
        // B10 review r2 #2: an 8-way labelled `<<choice>>` fan with notes
        // beside the hub, the choice and two targets. The fan spilled
        // round the diamond's corners ran along the diamond's row through
        // the note cards and the router rerouted the lot across each
        // other: 15 crossings, against 3 as the builders' curves. Its
        // starts now stub out of the corners and step in the gap below.
        let src = "stateDiagram-v2\nstate c <<choice>>\n[*] --> Hub\nHub --> c\n\
                   c --> A : alpha\nc --> B : beta\nc --> C : gamma\nc --> D : delta\n\
                   c --> E : epsilon\nc --> F : zeta\nc --> G : eta\nc --> H : theta\n\
                   note right of Hub : the hub\nnote left of A : a note\nnote right of D : d note\n\
                   note left of H : h note\nnote right of c : choice note\n";
        let g = match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            _ => unreachable!(),
        };
        let sc = scene(&g);
        let lines: Vec<Vec<Pt>> = sc.edges.iter().map(crate::scene::edge_polyline).collect();
        let mut n = 0;
        for i in 0..lines.len() {
            for j in i + 1..lines.len() {
                n += lines_cross(&lines[i], &lines[j]);
            }
        }
        assert!(n <= 3, "{n} crossings");
        assert_eq!(double_crossings(&sc), Vec::<String>::new());
        // No branch out of the diamond runs through a box.
        for e in sc.edges.iter().filter(|e| e.from == "c") {
            let pts = crate::scene::edge_polyline(e);
            for nd in sc
                .nodes
                .iter()
                .filter(|nd| nd.id != e.from && nd.id != e.to)
            {
                let r = (
                    nd.x - nd.w / 2.0,
                    nd.y - nd.h / 2.0,
                    nd.x + nd.w / 2.0,
                    nd.y + nd.h / 2.0,
                );
                let inside = |p: Pt| {
                    p.0 > r.0 + 1.0 && p.0 < r.2 - 1.0 && p.1 > r.1 + 1.0 && p.1 < r.3 - 1.0
                };
                assert!(
                    !pts.iter().any(|&p| inside(p)),
                    "{} -> {} through {}",
                    e.from,
                    e.to,
                    nd.id
                );
            }
        }
    }

    #[test]
    fn a_step_crossing_a_kept_curve_moves_clear_of_it_in_its_room() {
        // A note card's link (kept as drawn) down at breadth 50 to flow
        // 45: a step at 40 across from 0 to 100 crosses it.
        let line = vec![(50.0, 20.0), (50.0, 45.0)];
        let mut jogs = vec![jog(0, 0, 0.0, 100.0, (30.0, 70.0), 40.0)];
        assert!(jog_legs(&jogs[0], 40.0)
            .iter()
            .any(|&(a, b)| segs_cross(a, b, line[0], line[1])));
        uncross(&mut jogs, std::slice::from_ref(&line));
        let t = jogs[0].at;
        assert!((30.0..=70.0).contains(&t), "{t}");
        assert!(
            !jog_legs(&jogs[0], t)
                .iter()
                .any(|&(a, b)| segs_cross(a, b, line[0], line[1])),
            "{t}"
        );
        // Nearest where it was: just past the line's end.
        assert!((45.0 - EPS..=45.0 + TRACK_PITCH).contains(&t), "{t}");
        // A step crossing nothing stays.
        let mut free = vec![jog(0, 0, 0.0, 100.0, (30.0, 70.0), 60.0)];
        uncross(&mut free, std::slice::from_ref(&line));
        assert_eq!(free[0].at, 60.0);
    }

    /// The room a dummy column may move in keeps [`COLUMN_CLEAR`] off a
    /// box beside it on its row, and [`straighten`] runs a drifting
    /// chain straight wherever the rooms share a breadth.
    #[test]
    fn a_drifting_chain_runs_straight_where_its_rooms_allow() {
        let rows = [(40.0, 60.0), (90.0, 110.0), (140.0, 160.0)];
        let cols = [(100.0, 50.0), (130.0, 100.0), (96.0, 150.0)];
        let room = |boxes: &[Rect]| -> Vec<Pt> {
            let r = ColumnRoom {
                rows: &rows,
                boxes,
                loops: &[],
                frames: &[],
                columns: &[],
                runs: &[],
                labels: &[],
                own: [None, None],
                edge: 0,
            };
            straighten(&r, 100.0, &cols, 100.0, &[])
        };
        // Nothing beside it: one straight run under its source.
        assert!(room(&[]).iter().all(|q| q.0 == 100.0));
        // A box right of the middle column, clear of 100: still straight.
        let right = [(140.0, 200.0, 90.0, 110.0)];
        assert!(room(&right).iter().all(|q| q.0 == 100.0));
        // A box left of it reaching past 100 on its row: the column keeps
        // its clearance, and the chain runs straight beside the box.
        let left = [(60.0, 120.0, 90.0, 110.0)];
        let out = room(&left);
        assert!(out[1].0 >= 130.0 - EPS, "{out:?}");
        assert!(out.iter().all(|q| q.0 == out[0].0), "{out:?}");
        let r = ColumnRoom {
            rows: &rows,
            boxes: &left,
            loops: &[],
            frames: &[],
            columns: &[],
            runs: &[],
            labels: &[],
            own: [None, None],
            edge: 0,
        };
        assert_eq!(r.slot((150.0, 100.0)).0, 120.0 + COLUMN_CLEAR);
    }

    /// A label moves with its run: to the middle of a plain Z's step,
    /// or onto the target's run along the flow where it rides there;
    /// one a px or two off its old curve lands on the new line, one set
    /// beside its line keeps its offset.
    #[test]
    fn a_label_follows_its_run_onto_the_new_line() {
        let sc = scene(&parse("flowchart TD\nA -->|yes| B").unwrap());
        let mut e = sc.edges[0].clone();
        e.waypoints.clear();
        e.bezier = [(0.0, 0.0), (0.0, 50.0), (40.0, 50.0), (40.0, 100.0)];
        let mid = (20.0, 50.0);
        let z = [(0.0, 0.0), (0.0, 30.0), (40.0, 30.0), (40.0, 100.0)];
        let label = |e: &mut SceneEdge, at: Pt| {
            if let Some((_, c, _)) = e.label.as_mut() {
                *c = at;
            }
        };
        let at = |e: &SceneEdge| e.label.as_ref().unwrap().1;
        // On the curve's middle: onto the step's middle.
        label(&mut e, mid);
        let mut f = e.clone();
        relabel(&mut f, &z, None);
        assert_eq!(at(&f), (20.0, 30.0));
        // A px off: onto the line.
        label(&mut e, (mid.0, mid.1 + 1.5));
        let mut f = e.clone();
        relabel(&mut f, &z, None);
        assert_eq!(at(&f), (20.0, 30.0));
        // Set beside its line on purpose: keeps the offset.
        label(&mut e, (mid.0, mid.1 + 12.0));
        let mut f = e.clone();
        relabel(&mut f, &z, None);
        assert_eq!(at(&f), (20.0, 42.0));
        // Riding the run into the target: its spot along the flow kept.
        label(&mut e, (25.0, 70.0));
        let mut f = e.clone();
        relabel(&mut f, &z, Some(Direction::TD));
        assert_eq!(at(&f), (40.0, 70.0));
    }

    /// Every flowchart edge between distinct nodes the stage takes comes
    /// out on the axes in each direction, a straight one as a straight
    /// cubic, the rest with square corners only.
    #[test]
    fn edges_come_out_on_the_axes_in_every_direction() {
        for d in ["TD", "BT", "LR", "RL"] {
            let src = format!(
                "flowchart {d}\nA --> B & C & D\nB -->|label| E\nC --> E\nD --> F --> G\nA --> G"
            );
            let sc = scene(&parse(&src).unwrap());
            for e in &sc.edges {
                if e.waypoints.is_empty() {
                    let [p0, c1, c2, p3] = e.bezier;
                    let line = |p: Pt| {
                        ((p3.0 - p0.0) * (p.1 - p0.1) - (p3.1 - p0.1) * (p.0 - p0.0)).abs() < 1e-6
                    };
                    assert!(line(c1) && line(c2), "{d}: {e:?}");
                    assert!(p0.0 == p3.0 || p0.1 == p3.1, "{d}: {e:?}");
                } else {
                    assert!(crate::router::ortho_corners(e).is_some(), "{d}: {e:?}");
                }
            }
        }
    }
}
