//! Stage 1 of [`crate::edgepass::finish`]: port assignment (G9).
//!
//! The builders anchor each edge end on its own, so edges heading the
//! same way leave one node side from (nearly) one point, and an
//! incoming arrowhead can land right on an outgoing edge's start. Here
//! every drawn end is grouped by the side of the node (or frame) it
//! touches; each group is ordered by where its edges head, so the spread
//! ends never make their curves cross at the node, and then placed with
//! the smallest total move that keeps neighbouring ends a port spacing
//! apart (an isotonic fit) inside the usable part of the side.

use crate::clearance::{CHIP_SEP, NODE_SEP};
use crate::edgepass::{end_rect, normal, side_of, Ctx, EndRef, Mode, Side};
use crate::layout::LABEL_BOX_H;
use crate::model::{Graph, Marker, Shape};
use crate::scene::{
    edge_drawn, edge_polyline, grow_scene, title_head, Bbox, Scene, SceneEdge, GLYPH_PORT_GAP,
    MARGIN,
};
use std::collections::BTreeMap;

/// Wanted spacing between neighbouring edge ends on one side (px).
pub(crate) const PORT_SPACING: f64 = 14.0;
/// Distance kept between an end and the corners of a rect side (px).
pub(crate) const PORT_INSET: f64 = 8.0;
/// The least distance (px) a crowded rect side's ends keep from its
/// corners.
const PORT_INSET_MIN: f64 = 4.0;
/// Usable half-span of a diamond / circle side as a fraction of its
/// half-size: ends stay on the part of the border that faces the side.
const PORT_ROUND_SPAN: f64 = 0.45;
/// Usable half-span of a stadium's round cap as a fraction of its
/// radius: a cap faces its side more squarely than a circle's quadrant.
const PORT_CAP_SPAN: f64 = 0.7;
/// Least spacing (px) of the ends a side keeps before its outermost
/// ends spill round the corners onto the neighbouring sides.
const PORT_MIN_SPACING: f64 = 6.0;
/// Most the spacing of two parallel edges' ends (one pair of ends) is
/// stretched when their run is flatter than the side's normal: up to
/// 1:8 the runs keep the plain [`PORT_SPACING`] between them.
const PARALLEL_SLANT_MAX: f64 = 8.0;
/// Tolerance (px) when checking whether a group is already placed.
const SETTLE_EPS: f64 = 1e-6;
/// Spacing (px) below which even back-edge ends spill round a corner.
const PORT_HARD_SPACING: f64 = 5.0;
/// How far (px) the control point of an end moved round a corner is
/// lifted off its new side.
const SPILL_LIFT: f64 = 8.0;
/// End moves below this (px) are skipped, so untouched ends keep their
/// exact bytes.
const PORT_EPS: f64 = 0.25;
/// An end moving at least this far (px) marks its edge as moved.
const MOVED_MIN: f64 = 1.0;
/// How far (px) from its end an edge's drawn leg is checked against its
/// neighbour's when deciding whether a spaced side is out of order.
const PORT_LEG_REACH: f64 = 3.0 * PORT_SPACING;
/// A waypoint within this (px) of an end's tangent coordinate is the
/// end's straight stub and moves with it.
const STUB_EPS: f64 = 0.5;

type Pt = (f64, f64);

/// How a side's border runs: the usable tangent span and the normal
/// coordinate of the border point at a tangent coordinate.
#[derive(Clone, Copy)]
enum Profile {
    /// A straight side at normal coordinate `n` (rect-like nodes,
    /// frames, the flat part of a stadium).
    Flat { n: f64 },
    /// A circle of radius `r` whose centre sits at normal coordinate
    /// `n` (circles, stadium caps); `sgn` = the outward direction.
    Round { n: f64, r: f64, sgn: f64 },
    /// A diamond corner: `n` = centre, `hn`/`ht` = half sizes along the
    /// normal / tangent; `sgn` = the outward direction.
    Diamond { n: f64, hn: f64, ht: f64, sgn: f64 },
}

impl Profile {
    /// Normal coordinate of the border at tangent offset `dt` from the
    /// side's centre.
    fn normal_at(self, dt: f64) -> f64 {
        match self {
            Profile::Flat { n } => n,
            Profile::Round { n, r, sgn } => n + sgn * (r * r - dt * dt).max(0.0).sqrt(),
            Profile::Diamond { n, hn, ht, sgn } => n + sgn * hn * (1.0 - dt.abs() / ht).max(0.0),
        }
    }
}

/// One edge end taking part in a side group.
struct PortEnd {
    edge: usize,
    src: bool,
    /// Heading of the edge seen from the side (see [`heading`]).
    key: f64,
    /// Carries a glyph wider than an arrowhead.
    glyph: bool,
    /// The end as drawn now.
    at: Pt,
    /// The point the edge steers towards from the end.
    steer: Pt,
    /// Wanted tangent coordinate on the group's side.
    want: f64,
    /// The over-full side this end was moved off, round a corner.
    spilled: Option<Side>,
    /// The edge runs against the flow: its controls shape a bow round
    /// the nodes it passes, so its end spills round a corner last.
    back: bool,
}

/// Tangent / normal coordinates of a screen point for `side`: the
/// tangent axis increases with screen x (top/bottom) or y (left/right).
fn split(side: Side, p: Pt) -> (f64, f64) {
    match side {
        Side::Top | Side::Bottom => (p.0, p.1),
        Side::Left | Side::Right => (p.1, p.0),
    }
}

/// Inverse of [`split`].
fn join(side: Side, t: f64, n: f64) -> Pt {
    match side {
        Side::Top | Side::Bottom => (t, n),
        Side::Left | Side::Right => (n, t),
    }
}

/// Outward sign of a side along its normal axis.
fn out_sign(side: Side) -> f64 {
    match side {
        Side::Top | Side::Left => -1.0,
        Side::Bottom | Side::Right => 1.0,
    }
}

/// The angle of `(dt, dn)` from the outward normal, as a trig-free
/// monotone key in `[-2, 2]` (`dt` = tangent part, `dn` = outward
/// normal part): ends sorted by it fan out without crossing.
fn heading(dt: f64, dn: f64) -> f64 {
    let s = dt.abs() + dn.abs();
    if s < 1e-9 {
        return 0.0;
    }
    let r = dt / s;
    if dn >= 0.0 {
        r
    } else if dt >= 0.0 {
        2.0 - r
    } else {
        -2.0 - r
    }
}

/// The side a point on a node's border belongs to. Diamonds and round
/// shapes have no flat sides: their quadrant decides.
fn end_side(sc: &Scene, end: EndRef, p: Pt) -> Side {
    let rect = end_rect(sc, end);
    if let EndRef::Node(v) = end {
        let n = &sc.nodes[v];
        if is_round(n.shape) || n.shape == Shape::Diamond {
            let qx = (p.0 - n.x) / (n.w / 2.0).max(1e-9);
            let qy = (p.1 - n.y) / (n.h / 2.0).max(1e-9);
            return if qx.abs() > qy.abs() {
                if qx > 0.0 {
                    Side::Right
                } else {
                    Side::Left
                }
            } else if qy > 0.0 {
                Side::Bottom
            } else {
                Side::Top
            };
        }
    }
    side_of(rect, p)
}

/// Shapes whose border is a circle (see `scene::border`).
fn is_round(s: Shape) -> bool {
    matches!(
        s,
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd
    )
}

/// The usable tangent span `[lo, hi]` of `side` of `end`, its centre
/// and the border profile along it.
fn span(sc: &Scene, end: EndRef, side: Side) -> (f64, f64, f64, Profile) {
    let (x0, y0, x1, y1) = end_rect(sc, end);
    let (t0, t1) = split_range(side, (x0, y0, x1, y1));
    let (n0, n1) = match side {
        Side::Top | Side::Bottom => (y0, y1),
        Side::Left | Side::Right => (x0, x1),
    };
    let sgn = out_sign(side);
    let tc = (t0 + t1) / 2.0;
    let nc = (n0 + n1) / 2.0;
    let (ht, hn) = ((t1 - t0) / 2.0, (n1 - n0) / 2.0);
    let flat = Profile::Flat {
        n: if sgn > 0.0 { n1 } else { n0 },
    };
    let rect = || {
        let (lo, hi) = (t0 + PORT_INSET, t1 - PORT_INSET);
        if lo <= hi {
            (lo, hi, tc, flat)
        } else {
            (tc, tc, tc, flat)
        }
    };
    let EndRef::Node(v) = end else {
        return rect();
    };
    let n = &sc.nodes[v];
    let round = |half: f64| (tc - half, tc + half, tc);
    match n.shape {
        Shape::Diamond => {
            let (lo, hi, c) = round(PORT_ROUND_SPAN * ht);
            (lo, hi, c, Profile::Diamond { n: nc, hn, ht, sgn })
        }
        s if is_round(s) => {
            // `scene::border` uses the width as the diameter.
            let r = n.w / 2.0;
            let (lo, hi, c) = round(PORT_ROUND_SPAN * r);
            (lo, hi, c, Profile::Round { n: nc, r, sgn })
        }
        Shape::Stadium => {
            // Long sides: the flat run between the caps; short sides:
            // the cap, a half circle of the short half-size.
            if ht >= hn {
                let flat_half = (ht - hn).max(0.0);
                (tc - flat_half, tc + flat_half, tc, flat)
            } else {
                let r = ht;
                let (lo, hi, c) = round(PORT_CAP_SPAN * r);
                let n = nc + sgn * (hn - r);
                (lo, hi, c, Profile::Round { n, r, sgn })
            }
        }
        _ => rect(),
    }
}

/// Tangent range of a rect for `side`.
fn split_range(side: Side, (x0, y0, x1, y1): (f64, f64, f64, f64)) -> (f64, f64) {
    match side {
        Side::Top | Side::Bottom => (x0, x1),
        Side::Left | Side::Right => (y0, y1),
    }
}

/// Order-keeping placement with the least squared move: `x[i] >=
/// x[i-1] + s`, `lo + i*s <= x[i] <= hi - (k-1-i)*s`, close to `d`.
/// Pool-adjacent-violators on `d[i] - i*s`, then a per-end clamp (which
/// keeps the gaps, as its bounds step by `s` too).
pub(crate) fn place(d: &[f64], s: f64, lo: f64, hi: f64) -> Vec<f64> {
    let k = d.len();
    // Blocks of (sum, count) over y = d - i*s, merged while decreasing.
    let mut blocks: Vec<(f64, usize)> = Vec::with_capacity(k);
    for (i, &di) in d.iter().enumerate() {
        blocks.push((di - i as f64 * s, 1));
        while blocks.len() >= 2 {
            let (sb, cb) = blocks[blocks.len() - 1];
            let (sa, ca) = blocks[blocks.len() - 2];
            if sa / ca as f64 <= sb / cb as f64 {
                break;
            }
            blocks.pop();
            let last = blocks.len() - 1;
            blocks[last] = (sa + sb, ca + cb);
        }
    }
    let mut out = Vec::with_capacity(k);
    for (sum, cnt) in blocks {
        let mean = sum / cnt as f64;
        for _ in 0..cnt {
            let i = out.len() as f64;
            out.push(mean + i * s);
        }
    }
    for (i, x) in out.iter_mut().enumerate() {
        let a = lo + i as f64 * s;
        let b = hi - (k - 1 - i) as f64 * s;
        // `min` then `max`: rounding may leave `b` an ulp under `a`.
        *x = x.min(b).max(a);
    }
    out
}

/// The drawn end point of edge `i` (`src` = its start) and the point
/// the edge steers towards from there.
fn end_and_steer(sc: &Scene, i: usize, src: bool, side: Option<Side>) -> (Pt, Pt) {
    let e = &sc.edges[i];
    let w = &e.waypoints;
    if w.len() >= 2 {
        let p = if src { w[0] } else { w[w.len() - 1] };
        // The first waypoint off the end's straight stub (else the far
        // end), so a stub along the normal does not tie every heading.
        let inner: Box<dyn Iterator<Item = &Pt>> = if src {
            Box::new(w.iter().skip(1))
        } else {
            Box::new(w.iter().rev().skip(1))
        };
        let mut q = if src { w[w.len() - 1] } else { w[0] };
        for &c in inner {
            let off = match side {
                Some(s) => (split(s, c).0 - split(s, p).0).abs(),
                None => 1.0,
            };
            if off > STUB_EPS {
                q = c;
                break;
            }
        }
        (p, q)
    } else {
        let b = &e.bezier;
        let p = if src { b[0] } else { b[3] };
        // The curve's midpoint: for a bowed back-edge it is the apex,
        // which says better than the far end where the edge heads.
        let q = (
            (b[0].0 + 3.0 * b[1].0 + 3.0 * b[2].0 + b[3].0) / 8.0,
            (b[0].1 + 3.0 * b[1].1 + 3.0 * b[2].1 + b[3].1) / 8.0,
        );
        (p, q)
    }
}

/// The key a group of ends is sorted by: heading, then outgoing before
/// incoming, then edge index.
fn port_order(a: &PortEnd, b: &PortEnd) -> std::cmp::Ordering {
    a.key
        .total_cmp(&b.key)
        .then(b.src.cmp(&a.src))
        .then(a.edge.cmp(&b.edge))
}

/// The sides next to `side` at its low and high tangent ends.
fn neighbours(side: Side) -> (Side, Side) {
    match side {
        Side::Top | Side::Bottom => (Side::Left, Side::Right),
        Side::Left | Side::Right => (Side::Top, Side::Bottom),
    }
}

/// How many ends fit on a span of length `len` at
/// [`PORT_MIN_SPACING`].
fn capacity(len: f64) -> usize {
    (len.max(0.0) / PORT_MIN_SPACING).floor() as usize + 1
}

/// Give every drawn edge end its own port on the side it touches; sets
/// `moved[i]` for each edge whose ends moved by [`MOVED_MIN`] or more.
/// Both modes run the same rule.
pub(crate) fn assign(g: &Graph, sc: &mut Scene, cx: &Ctx, mode: Mode, moved: &mut [bool]) {
    place_ports(g, sc, cx, mode, moved, true, true, None);
}

/// Space again the ends the router left too close, after it ran: the
/// same rule as [`assign`], but an over-full side keeps its ends. A
/// rerouted path leaves its port straight along a planned lattice leg,
/// and turning that end round a corner would bend the leg back on
/// itself. Groups already spaced stay byte-identical.
pub(crate) fn respace(g: &Graph, sc: &mut Scene, cx: &Ctx, mode: Mode, moved: &mut [bool]) {
    place_ports(g, sc, cx, mode, moved, false, false, None);
}

/// What judges one side's re-placed ends in [`reorder`]: the scene with
/// them moved and the moved edges as they were; `false` puts them back.
pub(crate) type Judge<'a> = &'a mut dyn FnMut(&Scene, &[(usize, SceneEdge)]) -> bool;

/// The last word on each side's order, after the channel nudge (G9):
/// a side two of whose neighbouring ends cross on their drawn legs is
/// re-placed in the order of where the edges now head (the first leg
/// off each port, rerouted and nudged), as [`respace`] places it, and
/// kept when `judge` says so. Sides whose legs do not cross stay
/// byte-identical.
pub(crate) fn reorder(
    g: &Graph,
    sc: &mut Scene,
    cx: &Ctx,
    mode: Mode,
    moved: &mut [bool],
    judge: Judge,
) {
    place_ports(g, sc, cx, mode, moved, false, true, Some(judge));
}

/// [`assign`], [`respace`] and [`reorder`]: `spill_over` lets an
/// over-full side move its outermost ends onto the neighbouring sides;
/// `uncross` re-places a side whose neighbouring ends cross on their
/// legs, each side's change kept only when `judge` (if any) says so.
#[allow(clippy::too_many_arguments)]
fn place_ports(
    _g: &Graph,
    sc: &mut Scene,
    cx: &Ctx,
    _mode: Mode,
    moved: &mut [bool],
    spill_over: bool,
    uncross: bool,
    mut judge: Option<Judge>,
) {
    let mut groups: BTreeMap<(EndRef, Side), Vec<PortEnd>> = BTreeMap::new();
    for i in 0..sc.edges.len() {
        if !cx.drawn.get(i).copied().unwrap_or(false) {
            continue;
        }
        let Some((from, to)) = cx.ends.get(i).copied().flatten() else {
            continue;
        };
        let e = &sc.edges[i];
        for (src, end) in [(true, from), (false, to)] {
            let (p, _) = end_and_steer(sc, i, src, None);
            let side = end_side(sc, end, p);
            let (p, q) = end_and_steer(sc, i, src, Some(side));
            let glyph = if src {
                e.ends.start_marker(e.kind).is_some()
            } else {
                matches!(
                    e.ends.end_marker(e.kind),
                    Some(Marker::Cross | Marker::Circle)
                )
            };
            groups.entry((end, side)).or_default().push(PortEnd {
                edge: i,
                src,
                key: heading_on(side, p, q),
                glyph,
                at: p,
                steer: q,
                want: split(side, p).0,
                spilled: None,
                back: cx.back.get(i).copied().unwrap_or(false),
            });
        }
    }
    if spill_over {
        spill(sc, &mut groups);
    }

    // Per edge: the move of its start and of its end.
    let mut deltas: Vec<[Pt; 2]> = vec![[(0.0, 0.0); 2]; sc.edges.len()];
    let heads = title_heads(sc);
    for ((end, side), mut ports) in groups {
        let (lo, hi, tc, prof) = span(sc, end, side);
        // A straight side right under a frame's title: its ends keep
        // out of the chip's shadow, so no edge reaches them past the
        // frame's name (G6).
        let (lo, hi, shaded) = match prof {
            Profile::Flat { n } => unshaded(&heads, side, n, lo, hi, ports.len()),
            _ => (lo, hi, false),
        };
        let shaded = shaded && ports.iter().any(|p| p.want < lo || p.want > hi);
        if ports.len() < 2 && !shaded && !ports.iter().any(|p| p.spilled.is_some()) {
            continue;
        }
        ports.sort_by(port_order);
        let k = ports.len();
        let glyph = ports.iter().any(|p| p.glyph);
        let want = if glyph {
            PORT_SPACING.max(GLYPH_PORT_GAP)
        } else {
            PORT_SPACING
        };
        // A crowded straight side may use more of its length, up to
        // [`PORT_INSET_MIN`] from the corners.
        let (lo, hi) = match prof {
            Profile::Flat { .. } if k > 1 && hi - lo < want * (k - 1) as f64 && hi > lo => {
                let grow =
                    (PORT_INSET - PORT_INSET_MIN).min((want * (k - 1) as f64 - (hi - lo)) / 2.0);
                (lo - grow, hi + grow)
            }
            _ => (lo, hi),
        };
        let room = if k > 1 {
            (hi - lo) / (k - 1) as f64
        } else {
            0.0
        };
        // Neighbouring ends of parallel edges (one pair of ends) on a
        // run flatter than the side's normal come that much closer in
        // between: spread them by the run's slant (G1).
        let sibling = |a: &PortEnd, b: &PortEnd| {
            a.src == b.src && cx.ends.get(a.edge) == cx.ends.get(b.edge) && a.edge != b.edge
        };
        let slant = ports
            .windows(2)
            .filter(|w| sibling(&w[0], &w[1]))
            .map(|w| {
                let far = end_and_steer(sc, w[0].edge, !w[0].src, None).0;
                let (f, a) = (split(side, far), split(side, w[0].at));
                let (t, n) = ((f.0 - a.0).abs(), (f.1 - a.1).abs());
                // A free curve leaves and enters along the normal, so
                // its middle runs about twice as flat as its chord.
                ((2.0 * t).hypot(n) / n.max(SETTLE_EPS)).min(PARALLEL_SLANT_MAX)
            })
            .fold(1.0, f64::max);
        let mut s = (PORT_SPACING * slant).min(room);
        if glyph && GLYPH_PORT_GAP <= room {
            s = s.max(GLYPH_PORT_GAP);
        }
        // A spaced side whose neighbouring ends cross on their legs in
        // the wrong order is re-placed in order (G9): before routing,
        // and after the nudge.
        let crossed = |a: &PortEnd, b: &PortEnd| uncross && legs_cross(sc, a, b);
        if !shaded
            && ports.iter().all(|p| p.spilled.is_none())
            && settled(&ports, side, s, lo, hi, crossed)
        {
            continue;
        }
        let d: Vec<f64> = ports.iter().map(|p| p.want).collect();
        let x = place(&d, s, lo, hi);
        // The moved edges as they were, for the judge.
        let mut was: Vec<(usize, SceneEdge)> = Vec::new();
        let mut moves: Vec<(usize, usize, Pt)> = Vec::new();
        for (j, p) in ports.iter().enumerate() {
            let old = p.at;
            let new = join(side, x[j], prof.normal_at(x[j] - tc));
            let delta = (new.0 - old.0, new.1 - old.1);
            if p.spilled.is_none() && delta.0.abs() < PORT_EPS && delta.1.abs() < PORT_EPS {
                continue;
            }
            if judge.is_some() && was.iter().all(|w| w.0 != p.edge) {
                was.push((p.edge, sc.edges[p.edge].clone()));
            }
            moves.push((p.edge, usize::from(!p.src), delta));
            let e = &mut sc.edges[p.edge];
            if p.spilled.is_some() {
                turn_end(e, p.src, old, new, side, p.back);
            } else {
                move_end(e, p.src, old, delta, side, p.back);
            }
        }
        if let Some(judge) = judge.as_mut() {
            if !moves.is_empty() && !judge(sc, &was) {
                for (i, e) in was {
                    sc.edges[i] = e;
                }
                continue;
            }
        }
        for (i, end, delta) in moves {
            deltas[i][end] = delta;
            if delta.0.hypot(delta.1) >= MOVED_MIN {
                if let Some(m) = moved.get_mut(i) {
                    *m = true;
                }
            }
        }
    }
    // A single-curve edge's label rides the curve's midpoint, which
    // moves by the mean of its two end moves: follow it, unless that
    // makes the label hit more than where it sits (the label stage
    // re-places moved edges' labels anyway).
    for i in 0..sc.edges.len() {
        let [d0, d3] = deltas[i];
        let e = &sc.edges[i];
        if !e.waypoints.is_empty() || (d0 == (0.0, 0.0) && d3 == (0.0, 0.0)) {
            continue;
        }
        let Some((_, m, w)) = &e.label else {
            continue;
        };
        let to = (m.0 + (d0.0 + d3.0) / 2.0, m.1 + (d0.1 + d3.1) / 2.0);
        if label_hits(sc, i, to, *w) <= label_hits(sc, i, *m, *w) {
            if let Some((_, m, _)) = &mut sc.edges[i].label {
                *m = to;
            }
        }
    }
}

/// How far (px) out of a side a frame title's chip shades its ports:
/// the router's port stub plus the spacing an edge keeps off a chip.
const CHIP_SHADE: f64 = NODE_SEP + CHIP_SEP + PORT_SPACING;

/// Every frame's title header ([`crate::scene::title_head`]).
fn title_heads(sc: &Scene) -> Vec<(f64, f64, f64, f64)> {
    if sc.clusters.is_empty() {
        return Vec::new();
    }
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    let right = bb.finish().1 + MARGIN;
    (0..sc.clusters.len())
        .map(|ci| title_head(&sc.clusters, ci, right))
        .collect()
}

/// The usable tangent range `lo..hi` of a straight side at normal `n`,
/// less the stretch a title chip shades (within [`CHIP_SHADE`] out of
/// the side and [`CHIP_SEP`] along it): the longer free part when it
/// still holds `k` ends [`PORT_SPACING`] apart, else the whole range.
/// `true` when a chip shades the side.
fn unshaded(
    heads: &[(f64, f64, f64, f64)],
    side: Side,
    n: f64,
    lo: f64,
    hi: f64,
    k: usize,
) -> (f64, f64, bool) {
    let sgn = out_sign(side);
    let (a, b) = if sgn > 0.0 {
        (n, n + CHIP_SHADE)
    } else {
        (n - CHIP_SHADE, n)
    };
    let mut free = vec![(lo, hi)];
    let mut shaded = false;
    for &h in heads {
        let (t0, n0) = split(side, (h.0, h.1));
        let (t1, n1) = split(side, (h.2, h.3));
        if n1 + CHIP_SEP <= a || n0 - CHIP_SEP >= b {
            continue;
        }
        let (c0, c1) = (t0 - CHIP_SEP, t1 + CHIP_SEP);
        if c1 <= lo || c0 >= hi {
            continue;
        }
        shaded = true;
        free = free
            .into_iter()
            .flat_map(|(p, q)| [(p, q.min(c0)), (p.max(c1), q)])
            .filter(|(p, q)| q >= p)
            .collect();
    }
    let need = PORT_SPACING * k.saturating_sub(1) as f64;
    match free
        .into_iter()
        .filter(|(p, q)| q - p >= need)
        .max_by(|x, y| (x.1 - x.0).total_cmp(&(y.1 - y.0)))
    {
        Some((p, q)) if shaded => (p, q, true),
        _ => (lo, hi, false),
    }
}

/// What a label box of width `w` centred at `m` would hit: nodes, frame
/// border lines and the other edges' labels.
fn label_hits(sc: &Scene, own: usize, m: Pt, w: f64) -> usize {
    let r = (
        m.0 - w / 2.0,
        m.1 - LABEL_BOX_H / 2.0,
        m.0 + w / 2.0,
        m.1 + LABEL_BOX_H / 2.0,
    );
    let meets = |o: (f64, f64, f64, f64)| r.0 < o.2 && o.0 < r.2 && r.1 < o.3 && o.1 < r.3;
    let nodes = sc
        .nodes
        .iter()
        .filter(|n| {
            meets((
                n.x - n.w / 2.0,
                n.y - n.h / 2.0,
                n.x + n.w / 2.0,
                n.y + n.h / 2.0,
            ))
        })
        .count();
    let borders = sc
        .clusters
        .iter()
        .filter(|c| {
            let (x0, y0, x1, y1) = (c.x, c.y, c.x + c.w, c.y + c.h);
            let inside = r.0 > x0 && r.1 > y0 && r.2 < x1 && r.3 < y1;
            let outside = r.2 < x0 || r.0 > x1 || r.3 < y0 || r.1 > y1;
            !inside && !outside
        })
        .count();
    let labels = sc
        .edges
        .iter()
        .enumerate()
        .filter(|&(j, e)| j != own && edge_drawn(e))
        .filter_map(|(_, e)| e.label.as_ref())
        .filter(|(_, c, lw)| {
            meets((
                c.0 - lw / 2.0,
                c.1 - LABEL_BOX_H / 2.0,
                c.0 + lw / 2.0,
                c.1 + LABEL_BOX_H / 2.0,
            ))
        })
        .count();
    nodes + borders + labels
}

/// Heading key of an end at `p` steering to `q`, seen from `side`.
fn heading_on(side: Side, p: Pt, q: Pt) -> f64 {
    let (pt, pn) = split(side, p);
    let (qt, qn) = split(side, q);
    heading(qt - pt, (qn - pn) * out_sign(side))
}

/// Move the outermost ends of an over-full side onto the neighbouring
/// sides, the end heading most sideways first, until the side holds
/// what fits at [`PORT_MIN_SPACING`]: a hub with a dozen edges on one
/// short side otherwise squeezes them into one blob. A moved end starts
/// at the corner it came round.
fn spill(sc: &Scene, groups: &mut BTreeMap<(EndRef, Side), Vec<PortEnd>>) {
    let cap_of = |key: (EndRef, Side)| {
        let (lo, hi, _, _) = span(sc, key.0, key.1);
        capacity(hi - lo)
    };
    // Fullest sides first (then by key), so a side that takes spilled
    // ends has already given up its own.
    let mut keys: Vec<(EndRef, Side)> = groups.keys().copied().collect();
    keys.sort_by_key(|k| (std::cmp::Reverse(groups[k].len()), *k));
    for key in keys {
        let (end, side) = key;
        let cap = cap_of(key);
        if groups[&key].len() <= cap {
            continue;
        }
        let (low_side, high_side) = neighbours(side);
        let room = |groups: &BTreeMap<(EndRef, Side), Vec<PortEnd>>, s: Side| {
            let k = (end, s);
            cap_of(k).saturating_sub(groups.get(&k).map_or(0, Vec::len))
        };
        let (mut low_room, mut high_room) = (room(groups, low_side), room(groups, high_side));
        let ports = groups.get_mut(&key).expect("listed key");
        ports.sort_by(port_order);
        // Back-edge ends stay while the side can still hold its ends
        // [`PORT_HARD_SPACING`] apart: their bow is planned round this
        // side.
        let (lo, hi, _, _) = span(sc, end, side);
        let hard_cap = (((hi - lo).max(0.0) / PORT_HARD_SPACING).floor() as usize + 1).max(cap);
        let mut out: Vec<(Side, PortEnd)> = Vec::new();
        loop {
            let n = ports.len();
            if n <= cap {
                break;
            }
            let any = n > hard_cap;
            let can_low = low_room > 0 && (any || !ports[0].back);
            let can_high = high_room > 0 && (any || !ports[n - 1].back);
            if !(can_low || can_high) {
                break;
            }
            let first = ports[0].key.abs();
            let last = ports[n - 1].key.abs();
            if can_high && (last > first || !can_low) {
                out.push((high_side, ports.pop().expect("over capacity")));
                high_room -= 1;
            } else {
                out.push((low_side, ports.remove(0)));
                low_room -= 1;
            }
        }
        // The neighbour's span end next to `side`.
        let near_low = matches!(side, Side::Top | Side::Left);
        for (ns, mut p) in out {
            let (nlo, nhi, _, _) = span(sc, end, ns);
            p.want = if near_low { nlo } else { nhi };
            p.key = heading_on(ns, join(ns, p.want, split(ns, p.at).1), p.steer);
            p.spilled = Some(side);
            groups.entry((end, ns)).or_default().push(p);
        }
    }
}

/// Whether a group's ends (in [`port_order`]) already sit `s` apart
/// inside `[lo, hi]`: such a group is left exactly as drawn (and a
/// second pass changes nothing). Two neighbours sitting against that
/// order whose legs `crossed` (an end whose edge turns towards the low
/// end of the side below one heading straight on) are not settled
/// either: they cross right at the node. Only a crossing counts, not
/// the order alone, so a pass that swapped them never swaps them back.
fn settled(
    ports: &[PortEnd],
    side: Side,
    s: f64,
    lo: f64,
    hi: f64,
    crossed: impl Fn(&PortEnd, &PortEnd) -> bool,
) -> bool {
    let mut by_at: Vec<(f64, usize)> = ports
        .iter()
        .enumerate()
        .map(|(k, p)| (split(side, p.at).0, k))
        .collect();
    by_at.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    by_at
        .iter()
        .all(|&(t, _)| t >= lo - SETTLE_EPS && t <= hi + SETTLE_EPS)
        && by_at.windows(2).all(|w| {
            w[1].0 - w[0].0 >= s - SETTLE_EPS
                && (w[0].1 < w[1].1 || !crossed(&ports[w[0].1], &ports[w[1].1]))
        })
}

/// Whether the drawn legs of two ends (each edge's polyline from its
/// end out to [`PORT_LEG_REACH`]) cross.
fn legs_cross(sc: &Scene, a: &PortEnd, b: &PortEnd) -> bool {
    let leg = |p: &PortEnd| {
        let mut pts = edge_polyline(&sc.edges[p.edge]);
        if !p.src {
            pts.reverse();
        }
        let far = pts
            .iter()
            .position(|q| (q.0 - p.at.0).hypot(q.1 - p.at.1) > PORT_LEG_REACH)
            .map_or(pts.len(), |k| k + 1);
        pts.truncate(far);
        pts
    };
    let (la, lb) = (leg(a), leg(b));
    la.windows(2)
        .any(|u| lb.windows(2).any(|v| segs_cross(u[0], u[1], v[0], v[1])))
}

/// Whether segments `a`–`b` and `c`–`d` properly cross.
fn segs_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let side = |p: Pt, q: Pt, r: Pt| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

/// Move one end of `e` (its start when `src`) from `old` by `delta`,
/// with the control point (for a `back`-edge, so its bow's apex stays)
/// or stub waypoint next to it, so the edge still leaves the border the
/// way it did.
fn move_end(e: &mut SceneEdge, src: bool, old: Pt, delta: Pt, side: Side, back: bool) {
    let add = |p: &mut Pt| {
        p.0 += delta.0;
        p.1 += delta.1;
    };
    let (pi, ci) = if src { (0, 1) } else { (3, 2) };
    if e.waypoints.is_empty() {
        add(&mut e.bezier[pi]);
        if back {
            // A back-edge's controls set its bow round the nodes it
            // passes: pull the control back by a third of the move, which
            // keeps the bow's apex (`(p0 + 3 c1 + 3 c2 + p3) / 8`) where
            // it was.
            e.bezier[ci].0 -= delta.0 / 3.0;
            e.bezier[ci].1 -= delta.1 / 3.0;
        } else {
            add(&mut e.bezier[ci]);
        }
        return;
    }
    let n = e.waypoints.len();
    let wi = if src { 0 } else { n - 1 };
    add(&mut e.waypoints[wi]);
    // The straight leg out of the end follows along the side: the stub
    // and, on a rerouted lattice path, every later point still on that
    // leg up to its rounded corner, so the leg stays one straight run
    // instead of kinking back (never the far end itself).
    let (ot, _) = split(side, old);
    let (dt, _) = split(side, delta);
    let mut k = 1;
    while k + 1 < n {
        let j = if src { k } else { n - 1 - k };
        let (nt, nn) = split(side, e.waypoints[j]);
        if (nt - ot).abs() >= STUB_EPS {
            break;
        }
        e.waypoints[j] = join(side, nt + dt, nn);
        k += 1;
    }
    // The next leg runs along the side. When the end moved the way that
    // leg heads, the leg's points the moved leg now reaches past would
    // make the path double back: drop them (the corner is rounded by the
    // drawn spline instead). Only that leg's own points go: when the
    // move reaches past its far corner, the leg after it (along the
    // normal) slides along with the end instead, so the path stays
    // orthogonal rather than cutting a diagonal across the drawing.
    let at = |k: usize| if src { k } else { n - 1 - k };
    let mut drop = 0;
    if k + 1 < n {
        let (head, leg) = split(side, e.waypoints[at(k)]);
        let head = head - ot;
        if head * dt > 0.0 {
            while k + drop + 1 < n {
                let (t, nn) = split(side, e.waypoints[at(k + drop)]);
                if (t - ot) * head.signum() >= dt * head.signum() || (nn - leg).abs() >= STUB_EPS {
                    break;
                }
                drop += 1;
            }
            let next = k + drop;
            if drop > 0 && next + 1 < n {
                let (tc, nn) = split(side, e.waypoints[at(next)]);
                if (nn - leg).abs() >= STUB_EPS {
                    // Past the leg's far corner: slide the normal leg.
                    let mut q = next;
                    while q + 1 < n {
                        let (t, nq) = split(side, e.waypoints[at(q)]);
                        if (t - tc).abs() >= STUB_EPS {
                            break;
                        }
                        e.waypoints[at(q)] = join(side, ot + dt, nq);
                        q += 1;
                    }
                }
            }
        }
    }
    if drop > 0 {
        if src {
            e.waypoints.drain(k..k + drop);
        } else {
            e.waypoints.drain(n - k - drop..n - k);
        }
    }
    // Keep the fallback curve's matching end on the drawn end.
    if same(e.bezier[pi], old) {
        if same(e.bezier[ci], old) {
            add(&mut e.bezier[ci]);
        }
        add(&mut e.bezier[pi]);
    }
}

/// Move one end of `e` from `old` round a corner to `new` on `side`.
/// The control point next to it moves along, keeping the direction the
/// curve left in (so a curve monotone along the flow stays monotone),
/// and lifts [`SPILL_LIFT`] off the new side so the curve visibly
/// leaves it instead of running along the border.
fn turn_end(e: &mut SceneEdge, src: bool, old: Pt, new: Pt, side: Side, back: bool) {
    let (pi, ci) = if src { (0, 1) } else { (3, 2) };
    let nv = normal(side);
    // A back-edge's control pulls back by a third of the move instead,
    // so its bow's apex stays (see [`move_end`]).
    let k = if back { -1.0 / 3.0 } else { 1.0 };
    let reach = |c: Pt| {
        (
            c.0 + k * (new.0 - old.0) + nv.0 * SPILL_LIFT,
            c.1 + k * (new.1 - old.1) + nv.1 * SPILL_LIFT,
        )
    };
    if e.waypoints.is_empty() {
        e.bezier[ci] = reach(e.bezier[ci]);
        e.bezier[pi] = new;
        return;
    }
    let n = e.waypoints.len();
    let wi = if src { 0 } else { n - 1 };
    e.waypoints[wi] = new;
    if same(e.bezier[pi], old) {
        if same(e.bezier[ci], old) {
            e.bezier[ci] = new;
        }
        e.bezier[pi] = new;
    }
}

/// Exactly the same point.
fn same(a: Pt, b: Pt) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edgepass::{finish, Mode};
    use crate::parser::parse;
    use crate::scene::{edge_polyline, route, scene};

    /// A frame's title chip shades the top side of the node just under
    /// it: its ends keep to the part of the side past the chip, so no
    /// edge reaches them along the frame's name (G6).
    #[test]
    fn ends_keep_out_of_a_title_chips_shadow() {
        // Chip header over x 0..100, ending 10 px above a top side
        // spanning x 50..250.
        let heads = [(0.0, 0.0, 100.0, 20.0)];
        let (lo, hi, shaded) = unshaded(&heads, Side::Top, 30.0, 58.0, 242.0, 2);
        assert!(shaded);
        assert!(lo >= 100.0 + CHIP_SEP && hi == 242.0, "{lo} {hi}");
        // Far below the chip: no shade.
        let (lo, hi, shaded) = unshaded(&heads, Side::Top, 300.0, 58.0, 242.0, 2);
        assert!(!shaded && lo == 58.0 && hi == 242.0);
        // No room past the chip for the ends: the side stays whole.
        let (lo, hi, _) = unshaded(&heads, Side::Top, 30.0, 58.0, 110.0, 2);
        assert!(lo == 58.0 && hi == 110.0);
    }

    /// The drawn start / end of an edge.
    fn ends(e: &SceneEdge) -> (Pt, Pt) {
        let p = edge_polyline(e);
        (p[0], p[p.len() - 1])
    }

    fn dist(a: Pt, b: Pt) -> f64 {
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    /// The static scene and the route() scene at the static centres.
    fn both(src: &str) -> [Scene; 2] {
        let g = parse(src).unwrap();
        let s = scene(&g);
        let c: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&g, &c);
        [s, r]
    }

    fn edge<'a>(sc: &'a Scene, from: &str, to: &str) -> &'a SceneEdge {
        sc.edges
            .iter()
            .find(|e| e.from == from && e.to == to)
            .unwrap_or_else(|| panic!("no {from} -> {to}"))
    }

    #[test]
    fn place_keeps_order_and_spacing_with_least_move() {
        // Already spaced: untouched.
        assert_eq!(
            place(&[0.0, 20.0, 40.0], 14.0, -50.0, 50.0),
            [0.0, 20.0, 40.0]
        );
        // Three ends on one point spread evenly round it.
        let x = place(&[10.0, 10.0, 10.0], 14.0, -50.0, 50.0);
        assert_eq!(x, [-4.0, 10.0, 24.0]);
        // Out of order ends pool; bounds push the run inside.
        let x = place(&[30.0, 0.0, 45.0], 10.0, 0.0, 40.0);
        assert!(x.windows(2).all(|w| w[1] - w[0] >= 10.0 - 1e-9), "{x:?}");
        assert!(x[0] >= 0.0 && x[2] <= 40.0, "{x:?}");
    }

    #[test]
    fn back_edge_never_starts_where_another_edge_arrives() {
        // A loop back from a branch: in both modes E -> B leaves E away
        // from where C -> E lands, and lands on B away from B -> C.
        let src = "flowchart TD\nA([Start])-->B[Read input]\nB-->C{Valid?}\n\
                   C-->|yes|D\nC-->|no|E[Show error]\nE-->B\nD==>F((Done))";
        for sc in both(src) {
            let (eb0, eb1) = ends(edge(&sc, "E", "B"));
            assert!(dist(eb0, ends(edge(&sc, "C", "E")).1) >= 10.0, "{eb0:?}");
            assert!(dist(eb1, ends(edge(&sc, "B", "C")).0) >= 10.0, "{eb1:?}");
        }
    }

    /// LR: one node feeding stacked boxes much taller than it.
    const FAN_TO_BOXES: &str = "flowchart LR\nsrc\n\
        subgraph g1\ndirection TB\na1\na2\na3\nend\n\
        subgraph g2\ndirection TB\nb1\nb2\nb3\nb4\nb5\nend\n\
        subgraph g3\ndirection TB\nc1\nc2\nend\n\
        ext\nsrc --> g1\nsrc --> g2\nsrc --> g3\nsrc --> ext\n";

    #[test]
    fn lr_boxes_are_entered_from_the_left_by_monotone_curves() {
        for (mode, sc) in ["static", "routed"].iter().zip(both(FAN_TO_BOXES)) {
            for e in sc.edges.iter().filter(|e| e.from == "src") {
                let pts = edge_polyline(e);
                let end = pts[pts.len() - 1];
                if let Some(c) = sc.clusters.iter().find(|c| c.id == e.to) {
                    assert!((end.0 - c.x).abs() < 1e-6, "{mode} {}: {end:?}", e.to);
                    assert!(end.1 > c.y && end.1 < c.y + c.h, "{mode} {}", e.to);
                }
                // Never backwards along the flow (G8).
                assert!(
                    pts.windows(2).all(|w| w[1].0 >= w[0].0 - 1e-6),
                    "{mode} src -> {} doubles back",
                    e.to
                );
            }
        }
    }

    #[test]
    fn hub_ends_get_distinct_ports_in_both_modes() {
        let mut src = String::from("flowchart LR\nhub\n");
        for i in 0..8 {
            src.push_str(&format!("in{i} --> hub\nhub --> out{i}\nout{i} -.-> hub\n"));
        }
        for sc in both(&src) {
            let mut at = Vec::new();
            for e in &sc.edges {
                let (p0, p1) = ends(e);
                if e.from == "hub" {
                    at.push(p0);
                }
                if e.to == "hub" {
                    at.push(p1);
                }
            }
            for i in 0..at.len() {
                for j in 0..i {
                    assert!(
                        dist(at[i], at[j]) >= crate::quality::PORT_MIN_GAP,
                        "{:?} {:?}",
                        at[i],
                        at[j]
                    );
                }
            }
        }
    }

    #[test]
    fn spaced_ends_crossing_on_their_legs_are_put_in_order() {
        // Right side of a node: one edge heads straight on, the other
        // turns down right away but leaves above it, so their legs
        // cross at the node. Before routing that side is not settled;
        // the same side in heading order is.
        let mut sc = scene(&parse("flowchart LR\nA-->B\nA-->C").unwrap());
        sc.edges[0].waypoints.clear();
        sc.edges[0].bezier = [(0.0, 10.0), (40.0, 10.0), (60.0, 10.0), (100.0, 10.0)];
        sc.edges[1].waypoints.clear();
        sc.edges[1].bezier = [(0.0, 0.0), (10.0, 0.0), (10.0, 60.0), (40.0, 60.0)];
        let end = |edge: usize, at: Pt, key: f64| PortEnd {
            edge,
            src: true,
            key,
            glyph: false,
            at,
            steer: at,
            want: at.1,
            spilled: None,
            back: false,
        };
        // In heading order: edge 0 (straight) first, edge 1 (down) last.
        let ports = [end(0, (0.0, 10.0), 0.0), end(1, (0.0, 0.0), 0.5)];
        assert!(legs_cross(&sc, &ports[0], &ports[1]));
        let crossed = |a: &PortEnd, b: &PortEnd| legs_cross(&sc, a, b);
        assert!(!settled(&ports, Side::Right, 10.0, -20.0, 20.0, crossed));
        // Without a crossing the spaced side stays as drawn.
        assert!(settled(&ports, Side::Right, 10.0, -20.0, 20.0, |_, _| {
            false
        }));
    }

    #[test]
    fn assigning_twice_changes_nothing() {
        let mut src = String::from("flowchart TD\nhub{hub}\n");
        for i in 0..12 {
            src.push_str(&format!("n{i} --> hub\nhub --> m{i}\n"));
        }
        let g = parse(&src).unwrap();
        for sc in both(&src) {
            let mut again = sc.clone();
            finish(&g, &mut again, Mode::Routed);
            assert_eq!(format!("{:?}", sc.edges), format!("{:?}", again.edges));
        }
    }
}
