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

use crate::clearance::{CHIP_SEP, EDGE_SPACING, NODE_SEP};
use crate::edgepass::{end_rect, normal, side_of, Ctx, EndRef, Mode, Side};
use crate::layout::LABEL_BOX_H;
use crate::model::{Graph, Marker, Shape};
use crate::scene::{
    edge_drawn, edge_polyline, grow_scene, title_head, Bbox, Scene, SceneEdge, ARROW_MARKER_PX,
    GLYPH_PORT_GAP, MARGIN, SLANT_MAX,
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
pub(crate) const PORT_ROUND_SPAN: f64 = 0.45;
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
/// Widest pitch (px) the evenly spread ends of a crowded box side take
/// (report M3, §2.4): a comb of three or more ends spreads over the
/// side at up to this, centred where its edges head, so a wide box's
/// fan keeps its ends together instead of reaching for the corners.
pub(crate) const PORT_EVEN_MAX: f64 = 2.0 * PORT_SPACING;
/// Least pitch (px) of an even comb ([`even_comb`]) holding two or more
/// arrowheads, where its side has the room: a head's width and the
/// spacing at which two strokes read as two, so neighbouring heads keep
/// a clear gap. At the port spacing a narrow box's five arrivals sat
/// 3 px apart and read as one bar of heads (fanout's Aggregator, B7
/// review r1 #7).
const HEAD_COMB_MIN: f64 = ARROW_MARKER_PX + EDGE_SPACING;
/// How far (px) along their sides the ends of a 1:1 pair on facing
/// sides may lie apart and still be drawn as one straight line (report
/// M3): a lone end is slid that far to meet its partner, and a comb
/// keeps such an end where its partner lies.
const STRAIGHT_SNAP: f64 = PORT_SPACING / 2.0;
/// How much more a straight end (its partner right across on the facing
/// side, [`STRAIGHT_SNAP`]) weighs than a plain one when a side's ends
/// are spaced: the others make way, so a straight edge never gains a
/// sideways wobble from its neighbours' spacing (report §2.4).
const STRAIGHT_WEIGHT: f64 = 8.0;
/// What a state note card's link end weighs: a pin. It stays where its
/// elbow meets its node's side whatever other edges share that side
/// (they space round it), so `scene()` and `route()` draw it alike
/// where their other edges differ, and a drag never moves it while its
/// ends stay put (verifier: the links' elbow ends sat 7-18 px apart in
/// the two, a router path spread past them in one and not the other).
const NOTE_WEIGHT: f64 = 1.0e6;
/// How far (px) spacing a side afresh may slide the end of an edge a
/// drag keeps ([`Ctx::fixed`]) before the side's redrawn ends are
/// placed round the kept ones instead ([`around_kept`]): half a port
/// spacing, a nudge the eye does not follow.
const KEPT_NUDGE: f64 = PORT_SPACING / 2.0;
/// What a back-edge's end weighs: its lane or curve is planned round
/// the nodes it passes, not to the side's spacing, so it gives way
/// first.
const BACK_WEIGHT: f64 = 0.25;

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
    /// A straight slanted side (a parallelogram's flank): normal
    /// coordinate `n` at the side's centre, moving `slope` px along the
    /// normal per px along the tangent; `edge` = the box side's.
    Line { n: f64, slope: f64, edge: f64 },
    /// A pointed side (a hexagon's flank): its tip at normal coordinate
    /// `n` (the side's centre), each half falling back inwards `slope`
    /// px per px along the tangent; `sgn` = the outward direction.
    Point { n: f64, slope: f64, sgn: f64 },
    /// Half an ellipse (a cylinder's lid or bottom arc) round normal
    /// coordinate `n`, radii `rt` / `rn` along the tangent / normal;
    /// `sgn` = the outward direction.
    Arc { n: f64, rt: f64, rn: f64, sgn: f64 },
    /// A drum's flank: straight at normal coordinate `n` (the box side)
    /// for `flat` px either way of the side's centre, then the shoulder,
    /// a quarter of the cap ellipse with radii `rt` / `rn` along the
    /// tangent / normal; `sgn` = the outward direction.
    Drum {
        n: f64,
        flat: f64,
        rt: f64,
        rn: f64,
        sgn: f64,
    },
}

impl Profile {
    /// Normal coordinate of the outline at tangent offset `dt` from the
    /// side's centre, as `scene::to_svg` paints it.
    fn normal_at(self, dt: f64) -> f64 {
        match self {
            Profile::Flat { n } => n,
            Profile::Round { n, r, sgn } => n + sgn * (r * r - dt * dt).max(0.0).sqrt(),
            Profile::Diamond { n, hn, ht, sgn } => n + sgn * hn * (1.0 - dt.abs() / ht).max(0.0),
            Profile::Line { n, slope, .. } => n + slope * dt,
            Profile::Point { n, slope, sgn } => n - sgn * slope * dt.abs(),
            Profile::Arc { n, rt, rn, sgn } => {
                let k = if rt > 0.0 { dt / rt } else { 0.0 };
                n + sgn * rn * (1.0 - k * k).max(0.0).sqrt()
            }
            Profile::Drum {
                n,
                flat,
                rt,
                rn,
                sgn,
            } => {
                let over = dt.abs() - flat;
                if over <= 0.0 || rt <= 0.0 {
                    n
                } else {
                    let k = (over / rt).min(1.0);
                    n - sgn * rn * (1.0 - (1.0 - k * k).sqrt())
                }
            }
        }
    }

    /// Normal coordinate an end at tangent offset `dt` is placed at: on
    /// the outline for the shapes every stage meets as drawn (diamonds,
    /// circles, a stadium's caps: the router never slides their ports),
    /// on the box side for a slanted flank, a hexagon's point or a
    /// drum's arc — the router's stubs and slides run along box sides,
    /// so an end there must sit on one until the final stage puts it on
    /// the outline along its stub (`edgepass::snap_ends`).
    fn placed_at(self, dt: f64) -> f64 {
        match self {
            Profile::Point { n, .. } => n,
            _ => self.straight().unwrap_or_else(|| self.normal_at(dt)),
        }
    }

    /// The normal coordinate of a side whose ends are placed along a
    /// straight box side (a frame's, a box's, a slanted flank's, a drum's
    /// lid or flank), `None` for a round, a diamond's or a hexagon's
    /// pointed side: a straight side takes a title chip's shadow and
    /// lends a crowd of ends its corners.
    fn straight(self) -> Option<f64> {
        match self {
            Profile::Flat { n } | Profile::Drum { n, .. } => Some(n),
            Profile::Line { edge, .. } => Some(edge),
            Profile::Arc { n, rn, sgn, .. } => Some(n + sgn * rn),
            Profile::Round { .. } | Profile::Diamond { .. } | Profile::Point { .. } => None,
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
    /// How hard the end holds on to `want` when the side is spaced
    /// ([`place_weighted`]): 1 for a plain end, see [`end_weight`].
    weight: f64,
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
/// shapes have no flat sides: their quadrant decides; a slanted or
/// capped shape's end may sit on its outline, inside the box
/// ([`outline_side`]).
fn end_side(sc: &Scene, end: EndRef, p: Pt) -> Side {
    let rect = end_rect(sc, end);
    if let EndRef::Node(v) = end {
        let n = &sc.nodes[v];
        if !is_round(n.shape) && n.shape != Shape::Diamond {
            return outline_side(n.shape, rect, p);
        }
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
    side_of(rect, p)
}

/// The side of a node of `shape` with box `rect` that end `p` belongs
/// to: the box side nearest it ([`side_of`]) for an end on the box, as
/// the stages place every end. On a hexagon, a parallelogram or a
/// cylinder the final stage then puts it on the outline, inside the box
/// (`edgepass::snap_ends`), and a drag that keeps the edge
/// (`route_partial`) hands it back so: a parallelogram's flank end near
/// its bottom corner lies nearer the box's bottom than its side. For
/// such an end within the box the side whose outline profile
/// ([`shape_span`]) passes nearest it decides; on a drum's shoulder
/// (past a flank's straight run, beyond the span the lid's own ends keep
/// to) the flank does.
pub(crate) fn outline_side(shape: Shape, rect: (f64, f64, f64, f64), p: Pt) -> Side {
    let (x0, y0, x1, y1) = rect;
    let inside = (p.0 - x0).min(x1 - p.0).min(p.1 - y0).min(y1 - p.1);
    if inside <= ON_BOX
        || !matches!(
            shape,
            Shape::Hexagon | Shape::Parallelogram | Shape::ParallelogramAlt | Shape::Cylinder
        )
    {
        return side_of(rect, p);
    }
    if shape == Shape::Cylinder {
        let ry = crate::scene::cylinder_ry(x1 - x0, y1 - y0);
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let (hw, hh) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
        if (p.1 - cy).abs() > hh - ry && (p.0 - cx).abs() > hw - PORT_INSET {
            return if p.0 < cx { Side::Left } else { Side::Right };
        }
    }
    let mut best = (Side::Top, f64::INFINITY);
    for side in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
        let (_, _, tc, prof) = shape_span(Some(shape), rect, side);
        let (t, n) = split(side, p);
        let d = (n - prof.normal_at(t - tc)).abs();
        if d < best.1 {
            best = (side, d);
        }
    }
    best.0
}

/// How far (px) inside its box an end still counts as on it for
/// [`outline_side`]: the stages put ends exactly on a box side.
const ON_BOX: f64 = 0.5;

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
    let shape = match end {
        EndRef::Node(v) => Some(sc.nodes[v].shape),
        EndRef::Frame(_) => None,
    };
    shape_span(shape, end_rect(sc, end), side)
}

/// The usable tangent span `[lo, hi]` of `side` of a node of `shape`
/// (`None`: a frame) with box `(x0, y0, x1, y1)`, its centre and the
/// outline's profile along it, as `scene::to_svg` paints the outline.
fn shape_span(
    shape: Option<Shape>,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    side: Side,
) -> (f64, f64, f64, Profile) {
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
    let Some(shape) = shape else {
        return rect();
    };
    let (w, h) = (x1 - x0, y1 - y0);
    let round = |half: f64| (tc - half, tc + half, tc);
    match shape {
        Shape::Diamond => {
            let (lo, hi, c) = round(PORT_ROUND_SPAN * ht);
            (lo, hi, c, Profile::Diamond { n: nc, hn, ht, sgn })
        }
        s if is_round(s) => {
            // `scene::border` uses the width as the diameter.
            let r = w / 2.0;
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
        // The outlines `scene::to_svg` paints (report M6, §2.5): ends
        // spread on these sides sit on the slant, the point or the arc,
        // not on the box, and a top or bottom keeps them on its straight
        // run, off the slants.
        Shape::Hexagon | Shape::Parallelogram | Shape::ParallelogramAlt => {
            let k = SLANT_MAX.min(w / 4.0);
            match side {
                // The straight run between the slants: a hexagon's is
                // `k` in at both ends; `/ /` runs `k` in from the left
                // along its top and from the right along its bottom,
                // `\ \` the other way round.
                Side::Top | Side::Bottom => {
                    let top = side == Side::Top;
                    let (cut_lo, cut_hi) = match shape {
                        Shape::Hexagon => (k, k),
                        Shape::Parallelogram if top => (k, 0.0),
                        Shape::Parallelogram => (0.0, k),
                        _ if top => (0.0, k),
                        _ => (k, 0.0),
                    };
                    let (lo, hi) = (t0 + cut_lo + PORT_INSET, t1 - cut_hi - PORT_INSET);
                    if lo <= hi {
                        (lo, hi, tc, flat)
                    } else {
                        let c = (t0 + cut_lo + t1 - cut_hi) / 2.0;
                        (c, c, tc, flat)
                    }
                }
                // A hexagon's point: like a diamond's corner, the ends
                // keep to the part facing the side.
                _ if shape == Shape::Hexagon => {
                    let slope = if ht > 0.0 { k / ht } else { 0.0 };
                    let (lo, hi, c) = round(PORT_ROUND_SPAN * ht);
                    let tip = if sgn > 0.0 { n1 } else { n0 };
                    (lo, hi, c, Profile::Point { n: tip, slope, sgn })
                }
                // A parallelogram's flank: straight, slanted `k` over
                // the height, its middle `k / 2` in from the box.
                _ => {
                    let slope = if ht > 0.0 { k / (2.0 * ht) } else { 0.0 };
                    let slope = if shape == Shape::Parallelogram {
                        -slope
                    } else {
                        slope
                    };
                    let (mid, edge) = if sgn > 0.0 {
                        (n1 - k / 2.0, n1)
                    } else {
                        (n0 + k / 2.0, n0)
                    };
                    let (lo, hi, c, _) = rect();
                    (
                        lo,
                        hi,
                        c,
                        Profile::Line {
                            n: mid,
                            slope,
                            edge,
                        },
                    )
                }
            }
        }
        Shape::Cylinder => {
            let ry = crate::scene::cylinder_ry(w, h);
            match side {
                // The lid's back arc / the bottom arc.
                Side::Top | Side::Bottom => {
                    let (lo, hi, c, _) = rect();
                    let n = nc + sgn * (hn - ry);
                    (
                        lo,
                        hi,
                        c,
                        Profile::Arc {
                            n,
                            rt: ht,
                            rn: ry,
                            sgn,
                        },
                    )
                }
                // A flank: the box's span. Past the straight run between
                // the caps the drum's shoulder curves in, flatly (a wide,
                // shallow lid), so a crowded flank's corner ends land
                // beside it; the final stage puts them on its arc
                // (`edgepass::snap_ends`). Keeping them off the shoulders
                // instead spilled a hub's ends round its corners and
                // rerouted its fan (dense-lr: more crossings and border
                // runs).
                _ => {
                    let (lo, hi, c, _) = rect();
                    let n = if sgn > 0.0 { n1 } else { n0 };
                    let flat = (ht - ry).max(0.0);
                    let drum = Profile::Drum {
                        n,
                        flat,
                        rt: ry,
                        rn: hn,
                        sgn,
                    };
                    (lo, hi, c, drum)
                }
            }
        }
        _ => rect(),
    }
}

/// The tangent range a router leg's port may slide along on `side` of a
/// node of `shape` with box `rect` and stay where the ports stage would
/// put it ([`shape_span`]): `None` on a pointed side (a hexagon's point,
/// a diamond's or a circle's quadrant), where a slid end leaves the part
/// facing the side.
pub(crate) fn slide_span(
    shape: Shape,
    rect: (f64, f64, f64, f64),
    side: Side,
) -> Option<(f64, f64)> {
    let (lo, hi, _, prof) = shape_span(Some(shape), rect, side);
    match prof {
        // A side too short for any span collapses onto its middle:
        // nothing to slide along there.
        Profile::Flat { .. }
        | Profile::Line { .. }
        | Profile::Arc { .. }
        | Profile::Drum { .. } => (hi > lo).then_some((lo, hi)),
        Profile::Round { .. } | Profile::Diamond { .. } | Profile::Point { .. } => None,
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
    place_weighted(d, &vec![1.0; d.len()], s, lo, hi)
}

/// [`place`] with the squared move of end `i` weighted by `w[i]`
/// (report M3, §2.4): where the ends must spread, the light ones move
/// and the heavy ones (a straight edge's) keep their place.
/// Pool-adjacent-violators pooling weighted means. An end weighing
/// [`NOTE_WEIGHT`] is a pin: it stays exactly at its want, the ends
/// before and after it spaced within the span left on either side, at
/// a closer pitch where that is too short for them (a short side's span
/// would otherwise shift the pin with them, or push them past its end).
fn place_weighted(d: &[f64], w: &[f64], s: f64, lo: f64, hi: f64) -> Vec<f64> {
    if let Some(p) = w.iter().position(|&x| x >= NOTE_WEIGHT) {
        let pin = d[p];
        // The pitch `m` ends fit at in `room` past the pin.
        let pitch = |m: usize, room: f64| {
            if m == 0 {
                s
            } else {
                s.min(room.max(0.0) / m as f64)
            }
        };
        let (sl, sr) = (pitch(p, pin - lo), pitch(d.len() - p - 1, hi - pin));
        let mut out = place_weighted(&d[..p], &w[..p], sl, lo, pin - sl);
        out.push(pin);
        out.extend(place_weighted(&d[p + 1..], &w[p + 1..], sr, pin + sr, hi));
        return out;
    }
    let k = d.len();
    if k == 0 {
        return Vec::new();
    }
    // Blocks of (weighted sum, weight, count) over y = d - i*s, merged
    // while decreasing.
    let mut blocks: Vec<(f64, f64, usize)> = Vec::with_capacity(k);
    for (i, &di) in d.iter().enumerate() {
        let wi = w.get(i).copied().unwrap_or(1.0).max(SETTLE_EPS);
        blocks.push(((di - i as f64 * s) * wi, wi, 1));
        while blocks.len() >= 2 {
            let (sb, wb, cb) = blocks[blocks.len() - 1];
            let (sa, wa, ca) = blocks[blocks.len() - 2];
            if sa / wa <= sb / wb {
                break;
            }
            blocks.pop();
            let last = blocks.len() - 1;
            blocks[last] = (sa + sb, wa + wb, ca + cb);
        }
    }
    let mut out = Vec::with_capacity(k);
    for (sum, wsum, cnt) in blocks {
        let mean = sum / wsum;
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

/// Re-sorts each run of neighbouring parallel ends (one pair of ends,
/// either way round) in `ports`, sorted by [`port_order`], by where they
/// are drawn, ties by edge index. The builders' parallel offsets order
/// such edges alike at both ends, but seen from one end two of them
/// converging on nearly one point at the other (a small `<<choice>>`
/// diamond, a port the lean pinned to a box's corner) head the other
/// way round: spaced in heading order, they would cross on the way, and
/// the router would then reroute one of them. A 2-cycle's two edges
/// drawn as single curves across one gap are such a pair too: heading
/// order puts the outgoing end first at each end, which crosses them;
/// drawn order — the edge with the lower parallel offset (the lower
/// index) first wherever the lean pinned both to one spot — keeps them
/// side by side. (A return edge in a lane heads out to its lane, and
/// heading order keeps it outside its twin.)
fn keep_parallel_order(ports: &mut [PortEnd], cx: &Ctx, sc: &Scene) {
    let mut i = 0;
    while i < ports.len() {
        let mut j = i + 1;
        while j < ports.len() && same_pair(cx, sc, ports[i].edge, ports[j].edge) {
            j += 1;
        }
        ports[i..j].sort_by(|a, b| a.want.total_cmp(&b.want).then(a.edge.cmp(&b.edge)));
        i = j;
    }
}

/// Whether edges `a` and `b` join the same two ends: the same way round,
/// or the other way when both are drawn as single curves (a 2-cycle
/// across one gap).
fn same_pair(cx: &Ctx, sc: &Scene, a: usize, b: usize) -> bool {
    let curve = |i: usize| sc.edges.get(i).is_some_and(|e| e.waypoints.is_empty());
    match (
        cx.ends.get(a).copied().flatten(),
        cx.ends.get(b).copied().flatten(),
    ) {
        (Some(x), Some(y)) => x == y || (x.0 == y.1 && x.1 == y.0 && curve(a) && curve(b)),
        _ => false,
    }
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
/// Without a `judge`, parallel edges keep the order they are drawn in
/// (see [`keep_parallel_order`]).
#[allow(clippy::too_many_arguments)]
fn place_ports(
    g: &Graph,
    sc: &mut Scene,
    cx: &Ctx,
    _mode: Mode,
    moved: &mut [bool],
    spill_over: bool,
    uncross: bool,
    mut judge: Option<Judge>,
) {
    // Report M3 (§2.4), in a drawing the flowchart painter draws (not
    // a class or ER scene, whose painters set their own ends) and only
    // in `assign`, on the builders' ends: an end alone on its side
    // meets it in the middle, a box side's comb of three or more ends
    // spreads evenly, and a 1:1 pair on facing sides that nearly lines
    // up is drawn straight. The router then starts from these ports,
    // and the later passes only space what it moved.
    let flow = g.edge_paint_width.is_none();
    let shaping = flow && spill_over && judge.is_none();
    let mut groups = side_groups(sc, cx);
    if spill_over {
        spill(sc, &mut groups);
    }
    // Sides a self-loop starts and ends on: its two ends are no part of
    // the group, so an end there is not alone and keeps its place.
    // `respace` draws a nearly straight pair straight again: the plain
    // rule re-spacing a side the router changed slides a straight
    // edge's end along it too (advanced: `Gate --> Auth` 2.9 px off
    // straight, Gate's side respaced after the router moved `Gate ==>
    // Job Queue`'s end).
    let resnap = flow && !spill_over && judge.is_none();
    let looped = if shaping || resnap {
        looped_sides(sc, cx)
    } else {
        Vec::new()
    };
    // Lone ends first, so a comb sees where its straight edges' partners
    // end up.
    let mut order: Vec<(EndRef, Side)> = groups.keys().copied().collect();
    if shaping {
        order.sort_by_key(|k| (groups[k].len() != 1, *k));
    }

    // Per edge: the move of its start and of its end.
    let mut deltas: Vec<[Pt; 2]> = vec![[(0.0, 0.0); 2]; sc.edges.len()];
    let heads = title_heads(sc);
    for key in order {
        let (end, side) = key;
        let Some(mut ports) = groups.remove(&key) else {
            continue;
        };
        // A side every end of which a drag keeps ([`Ctx::fixed`], in
        // `route_partial`) is left as the base scene drew it: no changed
        // edge crowds it, and the base's ends there came out of the
        // router, the respacing and the nudge, so the plain rule may
        // find them unsettled (a parallel pair's slant asks for more
        // than the port spacing) and re-place them in heading order. A
        // 1 px drag of any node threw a choice's twins on such a side
        // 15-35 px across each other (verify spread-face, g1, g92: 68-75
        // unrelated ends over 20 px per diagram).
        if ports
            .iter()
            .all(|p| p.spilled.is_none() && cx.fixed.get(p.edge).copied().unwrap_or(false))
        {
            continue;
        }
        // Weighed on the ends as they are now: a lone end placed above
        // is where a comb's straight edge must meet it.
        for p in ports.iter_mut() {
            p.weight = end_weight(sc, cx, p, side, flow);
        }
        let (lo, hi, tc, prof) = span(sc, end, side);
        // A straight side right under a frame's title: its ends keep
        // out of the chip's shadow, so no edge reaches them past the
        // frame's name (G6).
        let (lo, hi, shaded) = match prof.straight() {
            Some(n) => unshaded(&heads, side, n, lo, hi, ports.len()),
            None => (lo, hi, false),
        };
        let shaded = shaded && ports.iter().any(|p| p.want < lo || p.want > hi);
        // A box side (not a frame's, not a bar's, whose branches meet it
        // where they lie, nor one a self-loop also uses) the shaping
        // above applies to.
        let shaped = shaping
            && prof.straight().is_some()
            && !looped.contains(&key)
            && matches!(end, EndRef::Node(v) if sc.nodes[v].shape != Shape::ForkBar)
            && ports.iter().all(|p| p.spilled.is_none());
        // Alone on its side, a forward edge meets it in the middle
        // (report §2.4: the builders lean every end toward the other
        // node, so a single arrival landed up to 44 px off its target's
        // centre and the flow through it jogged). A back-edge's lane
        // keeps the port at its span's edge, a note card's link the
        // spot its elbow was drawn to.
        let lone = shaped && ports.len() == 1 && free_end(sc, cx, &ports[0]);
        if ports.len() < 2 && !shaded && !lone && !ports.iter().any(|p| p.spilled.is_some()) {
            continue;
        }
        ports.sort_by(port_order);
        // Spacing keeps parallel edges in the order they are drawn;
        // only `reorder` re-places them in the order they head.
        if judge.is_none() {
            keep_parallel_order(&mut ports, cx, sc);
        }
        let k = ports.len();
        let glyph = ports.iter().any(|p| p.glyph);
        let want = if glyph {
            PORT_SPACING.max(GLYPH_PORT_GAP)
        } else {
            PORT_SPACING
        };
        // A crowded straight side may use more of its length, up to
        // [`PORT_INSET_MIN`] from the corners.
        let (lo, hi) = match prof.straight() {
            Some(_) if k > 1 && hi - lo < want * (k - 1) as f64 && hi > lo => {
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
        let sibling =
            |a: &PortEnd, b: &PortEnd| a.edge != b.edge && same_pair(cx, sc, a.edge, b.edge);
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
        let kept = |p: &PortEnd| cx.fixed.get(p.edge).copied().unwrap_or(false);
        let x = if lone {
            // The middle of the side's straight run (a slanted shape's
            // top or bottom is offset from its box's middle).
            vec![(lo + hi) / 2.0]
        } else if let Some(x) = (shaped
            && k >= 3
            && !ports
                .iter()
                .enumerate()
                .any(|(j, a)| ports[j + 1..].iter().any(|b| sibling(a, b))))
        .then(|| {
            // Two or more heads in the comb keep a gap between them.
            let heads = ports
                .iter()
                .filter(|p| {
                    let e = &sc.edges[p.edge];
                    if p.src {
                        e.ends.start_marker(e.kind).is_some()
                    } else {
                        e.ends.end_marker(e.kind).is_some()
                    }
                })
                .count();
            let head_min = if heads >= 2 { HEAD_COMB_MIN } else { 0.0 };
            even_comb(&ports, kept, head_min, s, room, lo, hi)
        })
        .filter(|x| {
            ports.iter().zip(x).all(|(p, &t)| {
                let held = kept(p) || p.weight >= NOTE_WEIGHT;
                !held || (t - split(side, p.at).0).abs() < PORT_EPS
            })
        }) {
            // (Parallel edges — a 2-cycle's twins too, even with a note
            // link's end between them — keep the spacing their run's
            // slant needs and the order they are drawn in: a comb would
            // pull one of a pair across under the other's label, or skew
            // a twin's bow into its head.)
            //
            // In `route_partial` the ends a drag keeps ([`Ctx::fixed`])
            // pin the comb, and a comb that would move one — or a note
            // link's pinned end, which the span may push it off — is not
            // used: the plain rule below then leaves the side as drawn
            // while it stays spaced, and spaces the others round a pin.
            // (A comb kept in `scene()` and dropped in `route()`, where
            // a back-edge's end heads the other way, moved a note link's
            // end 4.2 px between the two.) The base scene's ports came out of the
            // router, the respacing and the nudge after this comb, and
            // combing them afresh made a hub's fan jump 20-50 px under
            // a 1 px drag of a node none of its edges touch.
            x
        } else {
            if !shaded
                && ports.iter().all(|p| p.spilled.is_none())
                && settled(&ports, side, s, lo, hi, crossed)
            {
                continue;
            }
            let d: Vec<f64> = ports.iter().map(|p| p.want).collect();
            let w: Vec<f64> = ports.iter().map(|p| p.weight).collect();
            let x = place_weighted(&d, &w, s, lo, hi);
            // A drag's kept end the spacing would throw by more than a
            // nudge: the redrawn ends are placed round the kept ones
            // instead (see `around_kept`). A nudge stands, so the edges
            // on a dragged node match `route()`'s.
            let thrown = ports
                .iter()
                .zip(&x)
                .any(|(p, &t)| kept(p) && (t - split(side, p.at).0).abs() > KEPT_NUDGE);
            if thrown && !shaded && ports.iter().all(|p| p.spilled.is_none()) {
                around_kept(&ports, kept, side, s, lo, hi).unwrap_or(x)
            } else {
                x
            }
        };
        // The moved edges as they were, for the judge and the leg check
        // below.
        let mut was: Vec<(usize, SceneEdge)> = Vec::new();
        // The moved waypoint edges and the nodes they ran through then.
        let mut legs_was: Vec<(usize, usize)> = Vec::new();
        let mut moves: Vec<(usize, usize, Pt)> = Vec::new();
        for (j, p) in ports.iter().enumerate() {
            let old = p.at;
            let new = join(side, x[j], prof.placed_at(x[j] - tc));
            let delta = (new.0 - old.0, new.1 - old.1);
            if p.spilled.is_none() && delta.0.abs() < PORT_EPS && delta.1.abs() < PORT_EPS {
                continue;
            }
            // (Only after the router: `assign`'s moves are the router's
            // input, and an edge they push through a node is rerouted.)
            if (judge.is_some() || !spill_over) && was.iter().all(|w| w.0 != p.edge) {
                was.push((p.edge, sc.edges[p.edge].clone()));
                if !spill_over && !sc.edges[p.edge].waypoints.is_empty() {
                    legs_was.push((p.edge, legs_through_nodes(sc, cx, p.edge)));
                }
            }
            moves.push((p.edge, usize::from(!p.src), delta));
            let e = &mut sc.edges[p.edge];
            if p.spilled.is_some() {
                turn_end(e, p.src, old, new, side, p.back);
            } else {
                move_end(e, p.src, old, delta, side, p.back);
            }
        }
        // A move never drags an orthogonal path's leg through a node:
        // `move_end` slides the straight leg out of a moved end along
        // with it, and a long leg the router ran a dozen px past a box
        // slid 20 px lands in it (r280 in `route()`: `N0 --> N3` up
        // through `Queue validate`, its end re-spaced beside two lane
        // legs). Such a side keeps its ends where they were: the router
        // placed them clear of the taken ports.
        if legs_was
            .iter()
            .any(|&(i, hits)| legs_through_nodes(sc, cx, i) > hits)
        {
            for (i, e) in was {
                sc.edges[i] = e;
            }
            continue;
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
    if shaping || resnap {
        snap_straight_pairs(sc, cx, &looped, &mut deltas, moved);
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

/// Every drawn edge end, grouped by the side of the node or frame it
/// touches, as [`place_ports`] spaces them.
fn side_groups(sc: &Scene, cx: &Ctx) -> BTreeMap<(EndRef, Side), Vec<PortEnd>> {
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
                weight: 1.0,
            });
        }
    }
    groups
}

/// Whether edge `i` links a state note card ([`crate::style::is_note_card`]).
pub(crate) fn note_link(sc: &Scene, cx: &Ctx, i: usize) -> bool {
    let card = |r: EndRef| match r {
        EndRef::Node(v) => sc
            .nodes
            .get(v)
            .is_some_and(|n| crate::style::is_note_card(&n.id, n.shape, &n.style)),
        EndRef::Frame(_) => false,
    };
    cx.ends
        .get(i)
        .copied()
        .flatten()
        .is_some_and(|(a, b)| card(a) || card(b))
}

/// Whether the end `p`'s edge may be reshaped by the ports stage's own
/// rules (report M3): a forward edge the stages may move (not one a
/// drag keeps, [`Ctx::fixed`]) that links no note card.
fn free_end(sc: &Scene, cx: &Ctx, p: &PortEnd) -> bool {
    !p.back && !cx.fixed.get(p.edge).copied().unwrap_or(false) && !note_link(sc, cx, p.edge)
}

/// Whether sides `a` and `b` face each other (top and bottom, left and
/// right).
fn facing(a: Side, b: Side) -> bool {
    matches!(
        (a, b),
        (Side::Top, Side::Bottom)
            | (Side::Bottom, Side::Top)
            | (Side::Left, Side::Right)
            | (Side::Right, Side::Left)
    )
}

/// The tangent coordinate (along `side`) of the other end of `p`'s
/// edge when that end lies straight out of `side`, on the facing side
/// of its node, within [`STRAIGHT_SNAP`] of `p` along it.
fn straight_partner(sc: &Scene, cx: &Ctx, p: &PortEnd, side: Side) -> Option<f64> {
    let (from, to) = cx.ends.get(p.edge).copied().flatten()?;
    let other = if p.src { to } else { from };
    let q = end_and_steer(sc, p.edge, !p.src, None).0;
    if !facing(side, end_side(sc, other, q)) {
        return None;
    }
    let ((pt, pn), (qt, qn)) = (split(side, p.at), split(side, q));
    ((qn - pn) * out_sign(side) > 0.0 && (qt - pt).abs() <= STRAIGHT_SNAP).then_some(qt)
}

/// How hard end `p` on `side` holds its place when the side is spaced
/// (report M3, §2.4; flowchart drawings, `flow`, only — elsewhere every
/// end weighs alike): a note card's link most ([`NOTE_WEIGHT`]), then a
/// straight edge's end ([`STRAIGHT_WEIGHT`]); a back-edge's least
/// ([`BACK_WEIGHT`]).
fn end_weight(sc: &Scene, cx: &Ctx, p: &PortEnd, side: Side, flow: bool) -> f64 {
    if !flow {
        1.0
    } else if note_link(sc, cx, p.edge) {
        NOTE_WEIGHT
    } else if p.back {
        BACK_WEIGHT
    } else if straight_partner(sc, cx, p, side).is_some() {
        STRAIGHT_WEIGHT
    } else {
        1.0
    }
}

/// The sides self-loops start or end on.
fn looped_sides(sc: &Scene, cx: &Ctx) -> Vec<(EndRef, Side)> {
    let mut out = Vec::new();
    for (i, e) in sc.edges.iter().enumerate() {
        let Some((a, b)) = cx.ends.get(i).copied().flatten() else {
            continue;
        };
        if a != b {
            continue;
        }
        for p in [e.bezier[0], e.bezier[3]] {
            let key = (a, end_side(sc, a, p));
            if !out.contains(&key) {
                out.push(key);
            }
        }
    }
    out
}

/// The ends of a side holding both ends a drag keeps (`kept`, see
/// [`Ctx::fixed`]) and redrawn ones, placed with the kept ends where
/// they are: each redrawn end (in [`port_order`]) at the spot nearest
/// where it wants to be that stays `s` from every end placed so far and
/// on the span `[lo, hi]` (or as far out as a kept end lies: a crowded
/// side's span reaches nearer its corners, and the base scene may have
/// spaced more ends here). `None` when no end or every end is kept,
/// when the kept ends are not spaced `s` apart themselves, or when a
/// redrawn end finds no such spot: the side is then spaced afresh.
/// Used where spacing it afresh would throw a kept end more than
/// [`KEPT_NUDGE`]: in heading order it re-sorted the kept ends (a
/// router path heads elsewhere than the builder's curve it replaced)
/// and shifted a whole fan a port spacing under a 1 px drag.
fn around_kept(
    ports: &[PortEnd],
    kept: impl Fn(&PortEnd) -> bool,
    side: Side,
    s: f64,
    lo: f64,
    hi: f64,
) -> Option<Vec<f64>> {
    let n_kept = ports.iter().filter(|p| kept(p)).count();
    if n_kept == 0 || n_kept == ports.len() {
        return None;
    }
    let at = |p: &PortEnd| split(side, p.at).0;
    let mut taken: Vec<f64> = ports.iter().filter(|p| kept(p)).map(at).collect();
    taken.sort_by(f64::total_cmp);
    if taken.windows(2).any(|w| w[1] - w[0] < s - SETTLE_EPS) {
        return None;
    }
    let (lo, hi) = (lo.min(taken[0]), hi.max(taken[taken.len() - 1]));
    let on_span = |t: f64| t >= lo - SETTLE_EPS && t <= hi + SETTLE_EPS;
    let mut x = vec![0.0; ports.len()];
    for (j, p) in ports.iter().enumerate() {
        if kept(p) {
            x[j] = at(p);
            continue;
        }
        let want = p.want.max(lo).min(hi);
        // The nearest free spot is the want itself or the edge of a
        // taken spot's reach (or of the span).
        let mut tries = vec![want, lo, hi];
        for &t in &taken {
            tries.push(t - s);
            tries.push(t + s);
        }
        let best = tries
            .into_iter()
            .filter(|&c| on_span(c) && taken.iter().all(|&t| (c - t).abs() >= s - SETTLE_EPS))
            .min_by(|a, b| {
                (a - want)
                    .abs()
                    .total_cmp(&(b - want).abs())
                    .then(a.total_cmp(b))
            })?;
        x[j] = best;
        taken.push(best);
    }
    Some(x)
}

/// Report M3 (§2.4): a box side's three or more ends (in order) at one
/// even pitch, the side's usable span `[lo, hi]` shared out (never
/// under the spacing `s` nor over the even room `room`, at most
/// [`PORT_EVEN_MAX`], and at least `head_min` where the room allows:
/// [`HEAD_COMB_MIN`] for a comb of arrowheads), the comb set where its
/// ends want to be — their
/// weighted mean, so a straight edge's or a note link's end stays put —
/// and kept on the span. The plain spacing packed equal wants a port
/// spacing apart and left wide gaps to the middle end: a fan's heads
/// landed in near-touching pairs (fanout's 14 / 29 / 29 / 14 px).
fn even_comb(
    ports: &[PortEnd],
    kept: impl Fn(&PortEnd) -> bool,
    head_min: f64,
    s: f64,
    room: f64,
    lo: f64,
    hi: f64,
) -> Vec<f64> {
    let k = ports.len();
    let pitch = ((hi - lo) / (k + 1) as f64)
        .min(PORT_EVEN_MAX)
        .max(s)
        .max(head_min)
        .min(room.max(s));
    let slot = |j: usize| (j as f64 - (k - 1) as f64 / 2.0) * pitch;
    // The ends `kept` (a drag's kept edges'), if any, alone set the
    // centre.
    let pinned = ports.iter().any(&kept);
    let (mut num, mut den) = (0.0, 0.0);
    for (j, p) in ports.iter().enumerate() {
        let w = match (pinned, kept(p)) {
            (false, _) => p.weight,
            (true, true) => 1.0,
            (true, false) => 0.0,
        };
        num += w * (p.want - slot(j));
        den += w;
    }
    let c = num / den.max(SETTLE_EPS);
    let (cmin, cmax) = (lo - slot(0), hi - slot(k - 1));
    let c = if cmin <= cmax {
        c.max(cmin).min(cmax)
    } else {
        (cmin + cmax) / 2.0
    };
    let d: Vec<f64> = (0..k).map(|j| c + slot(j)).collect();
    let w: Vec<f64> = ports.iter().map(|p| p.weight).collect();
    place_weighted(&d, &w, pitch, lo, hi)
}

/// Report M3 (§2.4): a free curve whose ends sit on facing box sides
/// within [`STRAIGHT_SNAP`] of each other along them is drawn straight —
/// one end slides to the other's spot: an end alone on its side first,
/// else the arrival, else the start, each only where its side's other
/// ends stay a port spacing away and the spot stays on the side's span.
/// The spacing stages leave such a pair a few px apart (a comb's pitch,
/// a box a px off its neighbour's column), and the curve between then
/// wobbles sideways by that much. Adds each move to `deltas`.
fn snap_straight_pairs(
    sc: &mut Scene,
    cx: &Ctx,
    looped: &[(EndRef, Side)],
    deltas: &mut [[Pt; 2]],
    moved: &mut [bool],
) {
    let mut groups = side_groups(sc, cx);
    let mut key_of: BTreeMap<(usize, bool), (EndRef, Side)> = BTreeMap::new();
    for (key, ports) in &groups {
        for p in ports {
            key_of.insert((p.edge, p.src), *key);
        }
    }
    for i in 0..sc.edges.len() {
        let (Some(&ka), Some(&kb)) = (key_of.get(&(i, true)), key_of.get(&(i, false))) else {
            continue;
        };
        let box_side = |k: (EndRef, Side)| matches!(k.0, EndRef::Node(_)) && !looped.contains(&k);
        let bez = sc.edges[i].bezier;
        if !sc.edges[i].waypoints.is_empty()
            || !facing(ka.1, kb.1)
            || !box_side(ka)
            || !box_side(kb)
        {
            continue;
        }
        let Some(pa) = groups[&ka].iter().find(|p| p.edge == i && p.src) else {
            continue;
        };
        if !free_end(sc, cx, pa) {
            continue;
        }
        let ((ta, na), (tb, nb)) = (split(ka.1, bez[0]), split(kb.1, bez[3]));
        if (nb - na) * out_sign(ka.1) <= 0.0
            || !((PORT_EPS..=STRAIGHT_SNAP).contains(&(tb - ta).abs()))
        {
            continue;
        }
        let lone = |k: (EndRef, Side)| groups[&k].len() == 1;
        let mut tries = [(false, kb, ta), (true, ka, tb)];
        if lone(ka) && !lone(kb) {
            tries.swap(0, 1);
        }
        for (src, key, to) in tries {
            let (lo, hi, tc, prof) = span(sc, key.0, key.1);
            let clear = groups[&key]
                .iter()
                .filter(|p| !(p.edge == i && p.src == src))
                .all(|p| (split(key.1, p.at).0 - to).abs() >= PORT_SPACING - SETTLE_EPS);
            if !clear || to < lo - SETTLE_EPS || to > hi + SETTLE_EPS {
                continue;
            }
            let side = key.1;
            let old = if src { bez[0] } else { bez[3] };
            let new = join(side, to, prof.placed_at(to - tc));
            let delta = (new.0 - old.0, new.1 - old.1);
            move_end(&mut sc.edges[i], src, old, delta, side, false);
            let end = usize::from(!src);
            deltas[i][end] = (deltas[i][end].0 + delta.0, deltas[i][end].1 + delta.1);
            if delta.0.hypot(delta.1) >= MOVED_MIN {
                if let Some(m) = moved.get_mut(i) {
                    *m = true;
                }
            }
            if let Some(p) = groups
                .get_mut(&key)
                .and_then(|g| g.iter_mut().find(|p| p.edge == i && p.src == src))
            {
                p.at = new;
            }
            break;
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

/// How many nodes other than its own ends the straight legs between the
/// waypoints of edge `i` run through (each box less a pixel all round,
/// so a leg along a border does not count): what a port move must not
/// add ([`place_ports`]).
fn legs_through_nodes(sc: &Scene, cx: &Ctx, i: usize) -> usize {
    let w = &sc.edges[i].waypoints;
    if w.len() < 2 {
        return 0;
    }
    let own = |v: usize| {
        cx.ends
            .get(i)
            .copied()
            .flatten()
            .is_some_and(|(a, b)| a == EndRef::Node(v) || b == EndRef::Node(v))
    };
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in w {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    sc.nodes
        .iter()
        .enumerate()
        .filter(|&(v, n)| {
            let r = (
                n.x - n.w / 2.0 + 1.0,
                n.y - n.h / 2.0 + 1.0,
                n.x + n.w / 2.0 - 1.0,
                n.y + n.h / 2.0 - 1.0,
            );
            r.0 < r.2
                && r.1 < r.3
                && r.0 < x1
                && x0 < r.2
                && r.1 < y1
                && y0 < r.3
                && !own(v)
                && w.windows(2).any(|q| seg_meets_rect(q[0], q[1], r))
        })
        .count()
}

/// Whether segment `a`-`b` meets the open rectangle `r` = `(x0, y0, x1,
/// y1)` (Liang-Barsky clipping).
fn seg_meets_rect(a: Pt, b: Pt, r: (f64, f64, f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - r.0),
        (dx, r.2 - a.0),
        (-dy, a.1 - r.1),
        (dy, r.3 - a.1),
    ] {
        if p == 0.0 {
            if q <= 0.0 {
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
        // A back-edge's lane (an orthogonal path into this side, square to
        // it) never goes round the corner: `turn_end` would swing its last
        // leg into the neighbouring side at a slant (q10: a lane into an
        // over-full side entered the top at 34 deg).
        let lane = |p: &PortEnd| p.back && sc.edges[p.edge].waypoints.len() > 2;
        let mut out: Vec<(Side, PortEnd)> = Vec::new();
        loop {
            let n = ports.len();
            if n <= cap {
                break;
            }
            let any = n > hard_cap;
            let can_low = low_room > 0 && !lane(&ports[0]) && (any || !ports[0].back);
            let can_high = high_room > 0 && !lane(&ports[n - 1]) && (any || !ports[n - 1].back);
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
        // A back-edge drawn across the gap leaves along its side's
        // normal like a forward edge: its control moves with the end —
        // as does one whose control at this end runs straight out of it
        // (a labelled 2-cycle's twin bowed at its other end only,
        // `scene::bowed`: pulled back, its head swung 30 deg off square
        // in state `lr_right`).
        let (ct, _) = split(side, e.bezier[ci]);
        let (ot, _) = split(side, old);
        let straight = (ct - ot).abs() < STUB_EPS;
        let bowed = back && !straight && !across(&e.bezier, src, old);
        add(&mut e.bezier[pi]);
        if bowed {
            // A bowed back-edge's controls set its bow round the nodes it
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
    // A bowed back-edge's control pulls back by a third of the move
    // instead, so its bow's apex stays (see [`move_end`]).
    let bowed = back && !across(&e.bezier, src, old);
    let k = if bowed { -1.0 / 3.0 } else { 1.0 };
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

/// Whether a back-edge's curve `b` crosses the gap as a forward edge
/// does (the end being moved, at `old`, is its start when `src`): each
/// control straight out of its end along one and the same axis, the
/// sides' normal — not a bow round what lies between, nor an elbow
/// turning from one axis to the other (a note card's link).
fn across(b: &[Pt; 4], src: bool, old: Pt) -> bool {
    let (p0, p3) = if src { (old, b[3]) } else { (b[0], old) };
    let same_x = (b[1].0 - p0.0).abs() < STUB_EPS && (b[2].0 - p3.0).abs() < STUB_EPS;
    let same_y = (b[1].1 - p0.1).abs() < STUB_EPS && (b[2].1 - p3.1).abs() < STUB_EPS;
    same_x || same_y
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

    /// Report M6: each side's profile is the painted outline — every
    /// point on it sits on the border picking and the final stage use
    /// (`scene::outline_along` leaves it put) — and an end the final
    /// stage put there, inside the box, still belongs to its own side.
    #[test]
    fn profiles_trace_the_painted_outline_and_keep_their_ends_side() {
        let sc = scene(
            &parse(
                "flowchart TD\nH{{hexagon}} --> P[/lean right/] --> Q[\\lean left\\] --> C[(drum)]",
            )
            .unwrap(),
        );
        for n in &sc.nodes {
            let rect = (
                n.x - n.w / 2.0,
                n.y - n.h / 2.0,
                n.x + n.w / 2.0,
                n.y + n.h / 2.0,
            );
            for side in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
                let (lo, hi, tc, prof) = shape_span(Some(n.shape), rect, side);
                assert!(lo <= hi, "{} {side:?} span", n.id);
                for k in 0..=8 {
                    let t = lo + (hi - lo) * k as f64 / 8.0;
                    let p = join(side, t, prof.normal_at(t - tc));
                    let out = normal(side);
                    assert_eq!(
                        crate::scene::outline_along(n, p, out),
                        None,
                        "{} {side:?} at {t}: {p:?} off the outline",
                        n.id
                    );
                    assert_eq!(outline_side(n.shape, rect, p), side, "{} {p:?}", n.id);
                    // The port is placed on the box, where the router's
                    // stubs and slides run, and belongs to the same side.
                    let b = join(side, t, prof.placed_at(t - tc));
                    assert_eq!(outline_side(n.shape, rect, b), side, "{} {b:?}", n.id);
                }
            }
        }
        // A hexagon's top keeps its ends off the slants (a port inset in
        // from where they start); a parallelogram's top off its one.
        let hex = sc.nodes.iter().find(|n| n.id == "H").unwrap();
        let k = SLANT_MAX.min(hex.w / 4.0);
        let r = (hex.x - hex.w / 2.0, 0.0, hex.x + hex.w / 2.0, hex.h);
        let (lo, hi, _, _) = shape_span(Some(Shape::Hexagon), r, Side::Top);
        assert!(
            (lo - (r.0 + k + PORT_INSET)).abs() < 1e-9
                && (hi - (r.2 - k - PORT_INSET)).abs() < 1e-9
        );
        let (lo, hi, _, _) = shape_span(Some(Shape::Parallelogram), r, Side::Top);
        assert!(
            (lo - (r.0 + k + PORT_INSET)).abs() < 1e-9 && (hi - (r.2 - PORT_INSET)).abs() < 1e-9
        );
        // A router port slides along a hexagon's top, never its point.
        assert!(slide_span(Shape::Hexagon, r, Side::Top).is_some());
        assert_eq!(slide_span(Shape::Hexagon, r, Side::Left), None);
        // A flank end near a parallelogram's bottom corner, put on the
        // slant, lies nearer the box's bottom than its side: still a
        // right-flank end.
        let p = sc.nodes.iter().find(|n| n.id == "P").unwrap();
        let rect = (
            p.x - p.w / 2.0,
            p.y - p.h / 2.0,
            p.x + p.w / 2.0,
            p.y + p.h / 2.0,
        );
        let k = SLANT_MAX.min(p.w / 4.0);
        let y = p.y + p.h / 2.0 - 8.0;
        let x = rect.2 - k * (y - rect.1) / p.h;
        assert_eq!(side_of(rect, (x, y)), Side::Bottom);
        assert_eq!(outline_side(p.shape, rect, (x, y)), Side::Right);
    }

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
    fn parallel_edges_keep_the_order_they_are_drawn_in() {
        // Two transitions from a 28px <<choice>> to one state leave the
        // diamond at almost one point and lean on one side of B: seen
        // from B they head the other way round, so spacing B's ends in
        // heading order crossed them, and the router then rerouted one
        // of them into B's side. Spacing keeps their drawn order.
        for dir in ["TB", "BT", "LR", "RL"] {
            let src = format!(
                "stateDiagram-v2\ndirection {dir}\nstate c <<choice>>\n[*] --> P\nP --> c\n\
                 c --> B : low\nc --> B : medium\nc --> C : high\nc --> D : critical"
            );
            let crate::model::Document::State(g) = crate::parser::parse_document(&src).unwrap()
            else {
                panic!("a state diagram");
            };
            let s = scene(&g);
            let c: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &c);
            for (what, sc) in [("scene", &s), ("route", &r)] {
                let twins: Vec<Vec<Pt>> = sc
                    .edges
                    .iter()
                    .filter(|e| e.from == "c" && e.to == "B")
                    .inspect(|e| assert!(e.waypoints.is_empty(), "{dir} {what}: rerouted"))
                    .map(edge_polyline)
                    .collect();
                let (a, b) = (&twins[0], &twins[1]);
                let crossed = a
                    .windows(2)
                    .any(|u| b.windows(2).any(|v| segs_cross(u[0], u[1], v[0], v[1])));
                assert!(!crossed, "{dir} {what}: the twins cross");
            }
        }
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
    fn weighted_placement_moves_the_light_ends_and_never_a_pin() {
        // Unit weights: the plain fit.
        assert_eq!(
            place_weighted(&[10.0, 10.0, 10.0], &[1.0; 3], 14.0, -50.0, 50.0),
            place(&[10.0, 10.0, 10.0], 14.0, -50.0, 50.0)
        );
        // A straight end weighing STRAIGHT_WEIGHT barely moves; its
        // light neighbour takes the spacing (report §2.4: no S-wobble).
        let x = place_weighted(&[0.0, 3.0], &[STRAIGHT_WEIGHT, 1.0], 14.0, -50.0, 50.0);
        assert!(x[0].abs() < 2.0 && x[1] - x[0] >= 14.0 - 1e-9, "{x:?}");
        // A pin stays exactly put, even where the span is too short for
        // the spacing: the others squeeze against it.
        let x = place_weighted(&[0.0, 11.0], &[NOTE_WEIGHT, 1.0], 14.0, -11.0, 11.0);
        assert_eq!(x, [0.0, 11.0]);
        let x = place_weighted(&[-3.0, 0.0], &[1.0, NOTE_WEIGHT], 14.0, -11.0, 11.0);
        assert_eq!(x, [-11.0, 0.0]);
        // Two past it on a short span: a closer pitch, never past its end.
        let x = place_weighted(
            &[0.0, 5.0, 9.0],
            &[NOTE_WEIGHT, 1.0, 1.0],
            10.0,
            -11.0,
            11.0,
        );
        assert_eq!(x, [0.0, 5.5, 11.0]);
    }

    /// Report M3 (§2.4): an end alone on its box side meets it in the
    /// middle (long-labels' arrivals landed 35-44 px off their targets'
    /// centres), a box side's comb of three or more ends is evenly
    /// spaced (fanout's Aggregator took 14 / 29 / 29 / 14 px), in
    /// `scene()` and in `route()` alike.
    #[test]
    fn lone_ends_meet_their_side_in_the_middle_and_combs_space_evenly() {
        let root = env!("CARGO_MANIFEST_DIR");
        let read =
            |name: &str| std::fs::read_to_string(format!("{root}/examples/{name}.mmd")).unwrap();
        for sc in both(&read("long-labels")) {
            for (from, to) in [("B", "C"), ("B", "D"), ("C", "E")] {
                let n = sc.nodes.iter().find(|n| n.id == to).unwrap();
                let tip = ends(edge(&sc, from, to)).1;
                assert!(
                    (tip.0 - n.x).abs() < 5.0,
                    "{from} -> {to}: {tip:?} vs {}",
                    n.x
                );
            }
        }
        for sc in both(&read("fanout")) {
            let agg = sc.nodes.iter().find(|n| n.id == "C").unwrap();
            let mut xs: Vec<f64> = sc
                .edges
                .iter()
                .filter(|e| e.to == "C")
                .map(|e| ends(e).1 .0)
                .collect();
            xs.sort_by(f64::total_cmp);
            assert_eq!(xs.len(), 5, "{xs:?}");
            let gaps: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
            let (lo, hi) = gaps
                .iter()
                .fold((f64::MAX, 0.0f64), |a, &g| (a.0.min(g), a.1.max(g)));
            assert!(hi - lo <= 2.0, "uneven comb {gaps:?}");
            // Five heads keep a clear gap between them (B7 review r1
            // #7: at the port spacing they sat 3 px apart, one bar).
            assert!(lo >= HEAD_COMB_MIN - 1e-6, "{gaps:?}");
            assert!(lo - ARROW_MARKER_PX >= EDGE_SPACING - 1e-6, "{gaps:?}");
            // Centred: Worker 3 drops straight into its middle.
            assert!((xs[2] - agg.x).abs() < 1e-6, "{xs:?} vs {}", agg.x);
            for w in ["B1", "B2", "B4", "B5"] {
                let n = sc.nodes.iter().find(|n| n.id == w).unwrap();
                let tip = ends(edge(&sc, "A", w)).1;
                assert!((tip.0 - n.x).abs() < 1e-6, "A -> {w}: {tip:?} vs {}", n.x);
            }
        }
    }

    /// Report M3: a 1:1 pair on facing sides whose ends lie a few px
    /// apart along them is drawn straight — the lone end slides to its
    /// partner's spot — while a pair further apart keeps its S.
    #[test]
    fn a_nearly_straight_pair_is_drawn_straight() {
        let g =
            parse("flowchart TD\nA[a wide source box] --> B\nA --> C\nA --> D\nC --> E").unwrap();
        for (shift, straight) in [(4.0, true), (20.0, false)] {
            let mut sc = scene(&g);
            // E (C's only child) a few px off C's column.
            let ei = sc.nodes.iter().position(|n| n.id == "E").unwrap();
            sc.nodes[ei].x += shift;
            let k = sc
                .edges
                .iter()
                .position(|e| e.from == "C" && e.to == "E")
                .unwrap();
            for p in &mut sc.edges[k].bezier[2..] {
                p.0 += shift;
            }
            crate::edgepass::finish(&g, &mut sc, Mode::Static);
            let (p0, p3) = ends(&sc.edges[k]);
            assert_eq!(
                (p0.0 - p3.0).abs() < 1e-6,
                straight,
                "shift {shift}: {p0:?} {p3:?}"
            );
        }
        // B7 review r2 (#6): a pair straightened before routing stays so
        // when the respacing after the router slides its end along a
        // side the router changed (advanced: Gate's bottom, re-spaced
        // for `Gate ==> Job Queue`, left `Gate --> Auth` 2.9 px off).
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/advanced.mmd"
        ))
        .unwrap();
        for sc in both(&src) {
            let (p0, p3) = ends(edge(&sc, "Gate", "Auth"));
            assert!((p0.0 - p3.0).abs() < 1e-6, "{p0:?} {p3:?}");
        }
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
            weight: 1.0,
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
    fn a_drag_s_kept_ends_stay_and_pin_the_comb() {
        // B7 review r1 (#1): in `route_partial` the ends a drag keeps
        // hold their spots; the redrawn end takes the nearest free spot
        // (here below the kept fan, where the base scene had it, not
        // between two kept ends a port spacing apart), and a comb takes
        // its centre from the kept ends alone.
        let end = |edge: usize, want: f64, key: f64| PortEnd {
            edge,
            src: false,
            key,
            glyph: false,
            at: (0.0, want),
            steer: (0.0, want),
            want,
            spilled: None,
            back: false,
            weight: 1.0,
        };
        // Heading order puts the redrawn edge 3 between the kept ones.
        let ports = [
            end(0, 116.0, 0.0),
            end(3, 110.0, 0.1),
            end(1, 103.0, 0.2),
            end(2, 129.0, 0.3),
        ];
        let kept = |p: &PortEnd| p.edge != 3;
        let x = around_kept(&ports, kept, Side::Left, 13.0, 80.0, 140.0).unwrap();
        assert_eq!(x, vec![116.0, 90.0, 103.0, 129.0]);
        // Kept ends closer than the spacing: the side is spaced afresh.
        let tight = [end(0, 100.0, 0.0), end(1, 105.0, 0.1), end(3, 130.0, 0.2)];
        assert!(around_kept(&tight, kept, Side::Left, 13.0, 80.0, 140.0).is_none());
        // Every end kept, or none: nothing to place round.
        assert!(around_kept(&ports, |_| true, Side::Left, 13.0, 80.0, 140.0).is_none());
        assert!(around_kept(&ports, |_| false, Side::Left, 13.0, 80.0, 140.0).is_none());
        // A comb whose kept ends sit on its slots keeps them there,
        // wherever the redrawn end wants to be.
        let comb = [end(0, 100.0, 0.0), end(1, 114.0, 0.1), end(3, 140.0, 0.2)];
        let x = even_comb(&comb, kept, 0.0, 14.0, 14.0, 60.0, 160.0);
        assert!(
            (x[0] - 100.0).abs() < 1e-9 && (x[1] - 114.0).abs() < 1e-9,
            "{x:?}"
        );
        assert!((x[2] - 128.0).abs() < 1e-9, "{x:?}");
        // Off its 25 px slots they would move (`place_ports` then spaces
        // the side by the plain rule): the centre is theirs all the same.
        let x = even_comb(&comb, kept, 0.0, 14.0, 30.0, 60.0, 160.0);
        assert!((x[1] - 119.5).abs() < 1e-9, "{x:?}");
    }

    #[test]
    fn a_side_whose_every_end_a_drag_keeps_is_left_as_drawn() {
        // B7 review r2 (#1): the base scene's ends on a side the drag
        // does not touch may be closer than the plain rule's spacing
        // (the nudge and the respacing ran after it); every end kept,
        // the side is left exactly as drawn, never re-placed.
        let g = parse("flowchart TD\nA --> B\nA --> C\nA --> D").unwrap();
        let mut sc = scene(&g);
        let (b, c) = (ends(edge(&sc, "A", "B")).0, ends(edge(&sc, "A", "C")).0);
        let i = sc.edges.iter().position(|e| e.to == "B").unwrap();
        // Squeeze A -> B's start to 3 px off A -> C's.
        let dx = c.0 - 3.0 - b.0;
        let e = &mut sc.edges[i];
        e.bezier[0].0 += dx;
        e.bezier[1].0 += dx;
        let was: Vec<(Pt, Pt)> = sc.edges.iter().map(ends).collect();
        let mut kept = sc.clone();
        crate::edgepass::finish_fixed(&g, &mut kept, Mode::Routed, &vec![true; sc.edges.len()]);
        let now: Vec<(Pt, Pt)> = kept.edges.iter().map(ends).collect();
        assert_eq!(was, now);
        // With the squeezed edge redrawn the side is spaced again.
        let mut fixed = vec![true; sc.edges.len()];
        fixed[i] = false;
        let mut redrawn = sc.clone();
        crate::edgepass::finish_fixed(&g, &mut redrawn, Mode::Routed, &fixed);
        let (b, c) = (
            ends(edge(&redrawn, "A", "B")).0,
            ends(edge(&redrawn, "A", "C")).0,
        );
        assert!(dist(b, c) >= PORT_SPACING - 1e-6, "{b:?} {c:?}");
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
