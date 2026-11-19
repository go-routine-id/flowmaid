//! Back-edge lanes (report §2.2): the one rule for an edge that runs
//! against the flow, shared by the static layout
//! ([`crate::scene::scene`]), the layout's own estimate of what it will
//! draw (`layout::drawn_breadth`, which centres the drawing on it) and
//! [`crate::scene::route`], so the three agree on where such an edge
//! goes.
//!
//! Everything here is in flow coordinates: `b` across the flow, `l`
//! along it, growing downstream. A back-edge leaves its source (the
//! downstream end) by the side facing upstream and enters its target by
//! the side facing downstream. When nothing in the band between those
//! two sides stands in its way — adjacent layers, the return edge of a
//! 2-cycle, nodes between that lie off to one side ([`blocks_direct`]) —
//! it is drawn across the gap like a forward edge, a reversed S (no
//! lane). Otherwise it runs in an orthogonal lane, as mermaid
//! draws it: out of the source along its side's normal, across to the
//! lane, along the lane past everything in the band, across, and into
//! the target along its side's normal, so the arrowhead meets the side
//! square ([`corners`]). Only what reaches into the band is in its way
//! — the layers strictly between the ends, and anything on the ends'
//! own layers reaching [`LANE_REACH`] past the end's side (longer along
//! the flow than the end by twice that) — so the lane may run within an
//! end's own breadth, where it leaves or enters that end straight. The
//! lane keeps [`LANE_CLEAR`] plus half its label (the label rides the
//! lane, [`label_breadth`]) from what it passes, and takes the side
//! that costs the fewest crossings, then the shortest way round
//! ([`lane_at`]), never threading a gap between the nodes it passes.
//! Lanes are planned innermost first (the shortest band), each planned
//! lane an obstacle to the later ones whose band it shares, so nested
//! lanes never run on top of each other ([`plan`]).

use crate::clearance::NODE_SEP;
use crate::layout::{line_count, text_width, Placed, LABEL_BOX_H, LABEL_PAD_W};
use crate::model::Shape;
use crate::ports::{PORT_INSET, PORT_ROUND_SPAN};

type Pt = (f64, f64);

/// How far (px) a back-edge's lane clears the nodes it passes, before
/// half its label.
pub(crate) const LANE_CLEAR: f64 = 24.0;
/// Length (px) of a lane's stub out of each end along the side's
/// normal, before it turns across to the lane (at most a third of the
/// gap between the ends): an arrowhead and a rounded corner fit in it,
/// so the head enters its side square.
pub(crate) const LANE_STUB: f64 = 20.0;
/// How far (px) past an end's side something must reach into the band
/// to stand in the lane's way: the lane's legs run [`LANE_STUB`] out, so
/// a neighbour of an end only a little longer along the flow than the
/// end itself (an LR box a few px wider) is passed by.
pub(crate) const LANE_REACH: f64 = LANE_STUB / 2.0;
/// Room (px) the direct curve of a back-edge keeps from the nodes in its
/// band (besides half its label), else it takes a lane
/// ([`blocks_direct`]).
pub(crate) const DIRECT_CLEAR: f64 = LANE_CLEAR / 2.0;
/// Farthest (px) apart across the flow a back-edge's two ports may lie
/// for it to be drawn across the gap ([`blocks_direct`]): a reversed S
/// sweeps along the gap over that whole breadth, through every edge
/// crossing the gap there, and once the ports stage moves its end it
/// meets its side at a slant (bench-75's svc2 -> db, 592 px across a
/// 60 px gap, entered db at 71 deg in `route()`). Past it, a lane, whose
/// legs meet both sides square.
pub(crate) const DIRECT_REACH: f64 = 450.0;
/// How much (px) longer a lane's stub is for each lane nested round it
/// from the same end layer: the nested legs run side by side, the
/// outer lane's nearer its end, so they never cross.
pub(crate) const LANE_STAGGER: f64 = 12.0;
/// What one crossing costs a lane choosing its side, in px of way
/// round: a back-edge loops round the outside rather than thread
/// through the edges between its ends (mermaid's look), but not round a
/// whole wide drawing to save a crossing or two.
pub(crate) const LANE_CROSS_COST: f64 = 400.0;
/// Breadth positions closer than this (px) are one.
const LANE_EPS: f64 = 1e-6;
/// How near (px) across the flow a stretch of another edge running
/// along the flow lies on a lane's run ([`crossings`]): half the room a
/// node keeps, so the two would be drawn on top of each other.
const LANE_ON: f64 = NODE_SEP / 2.0;

/// The breadth a back-edge's label takes across its lane (the label is
/// centred on the lane): its box's height in LR / RL, where a label
/// lies along the flow, its box's width in TD / BT. (The width used to
/// count in every direction, so a wide LR label looped its back-edge
/// hundreds of px out, report §2.2.)
pub(crate) fn label_breadth(label: Option<&str>, horizontal: bool) -> f64 {
    match label {
        None => 0.0,
        Some(l) if horizontal => LABEL_BOX_H * line_count(l) as f64,
        Some(l) => text_width(l) + LABEL_PAD_W,
    }
}

/// The usable breadth span `(lo, hi)` of a port on a side of `p` that
/// faces along the flow: a box's straight run (as
/// [`crate::scene::port_lean`] keeps it), the part of a diamond's or a
/// circle's border the ports stage keeps an end on, a fork/join bar's
/// reach.
pub(crate) fn port_span(shape: Shape, p: &Placed) -> (f64, f64) {
    let half = match shape {
        Shape::Diamond
        | Shape::Circle
        | Shape::DoubleCircle
        | Shape::StateStart
        | Shape::StateEnd => PORT_ROUND_SPAN * p.bsize / 2.0,
        Shape::ForkBar => p.bsize / 2.0 - PORT_INSET,
        Shape::Stadium => p.bsize / 2.0 - p.lsize / 2.0 - 4.0,
        _ => p.bsize / 2.0 - 12.0,
    }
    .max(0.0);
    (p.b - half, p.b + half)
}

/// The breadth of the port at which the direct curve of a back-edge
/// (no lane) leaves or enters `p`, its other end's centre at breadth
/// `other`: leaning 0.35 of the way toward it within the side's usable
/// span, as [`crate::scene::anchor`] leans a port.
pub(crate) fn direct_port(shape: Shape, p: &Placed, other: f64) -> f64 {
    clamp_to(p.b + 0.35 * (other - p.b), port_span(shape, p))
}

/// The point at breadth `x` on the side of `p` facing upstream (`up`)
/// or downstream: on the outline of a diamond or a circle (as
/// `scene::border` draws them), on the flat side of anything else.
pub(crate) fn port_at(shape: Shape, p: &Placed, x: f64, up: bool) -> Pt {
    let (hb, hl) = (p.bsize / 2.0, p.lsize / 2.0);
    let dx = x - p.b;
    let reach = match shape {
        Shape::Diamond if hb > 0.0 => hl * (1.0 - dx.abs() / hb).max(0.0),
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            (hb * hb - dx * dx).max(0.0).sqrt()
        }
        _ => hl,
    };
    (x, if up { p.l - reach } else { p.l + reach })
}

/// The breadth `x` clamped into `span`.
fn clamp_to(x: f64, span: (f64, f64)) -> f64 {
    x.max(span.0).min(span.1)
}

/// Where along the flow a lane's two cross legs run: [`LANE_STUB`] off
/// the source's upstream side and off the target's downstream side, one
/// [`LANE_STAGGER`] further per lane nested round it from that end's
/// layer (see [`Lane::stagger`]) — within `room` (per end, the free gap
/// along the flow beyond its side, [`NODE_SEP`] kept from what lies
/// there) and a third of the gap between the ends.
pub(crate) fn legs(a: &Placed, b: &Placed, lane: &Lane) -> (f64, f64) {
    let top = a.l - a.lsize / 2.0;
    let bot = b.l + b.lsize / 2.0;
    let stub = |k: usize| {
        let want = LANE_STUB + lane.stagger[k] as f64 * LANE_STAGGER;
        want.max(lane.out[k])
            .min((lane.room[k] - NODE_SEP).max(LANE_STUB))
            .min((top - bot) / 3.0)
            .max(0.0)
    };
    (top - stub(0), bot + stub(1))
}

/// The free gap (px) along the flow beyond `end`'s side facing upstream
/// (`up`) or downstream, to the nearest of `others` there (any breadth):
/// how far its lane's leg may reach out ([`legs`]).
pub(crate) fn room<'a>(end: &Placed, up: bool, others: impl Iterator<Item = &'a Placed>) -> f64 {
    let side = if up {
        end.l - end.lsize / 2.0
    } else {
        end.l + end.lsize / 2.0
    };
    others
        .map(|p| {
            if up {
                side - (p.l + p.lsize / 2.0)
            } else {
                (p.l - p.lsize / 2.0) - side
            }
        })
        .filter(|&d| d >= 0.0)
        .fold(f64::INFINITY, f64::min)
}

/// The corners of a back-edge's `lane`, from the source
/// `a` (downstream) to the target `b`: each end's port as near the lane
/// as its side allows ([`port_span`]), a stub along the side's normal,
/// across to the lane, along it. An end whose side reaches the lane
/// runs straight along it, without a stub. An end flagged in `side`
/// (source, target; see [`meets_side`]) is met instead on its side
/// facing the lane, level with its centre (one [`LANE_STAGGER`] toward
/// the other end per lane nested round it there): the lane runs on to
/// there and turns straight in.
pub(crate) fn corners(
    a: (&Placed, Shape),
    b: (&Placed, Shape),
    lane: Lane,
    side: [bool; 2],
) -> Vec<Pt> {
    let ((pa, sa), (pb, sb)) = (a, b);
    let x = lane.x;
    let (s0, s3) = legs(pa, pb, &lane);
    // An end's port: as near the lane as its side allows — one
    // LANE_STAGGER further in per twin lane (between the same two ends)
    // nested round it, so twins leave and enter in their nesting order,
    // the outermost nearest the lane, and their legs never cross.
    let port = |p: &Placed, shape: Shape| {
        let span = port_span(shape, p);
        let near = clamp_to(x, span);
        let inward = if x > span.1 { -1.0 } else { 1.0 };
        if (near - x).abs() > LANE_EPS {
            clamp_to(near + inward * lane.twins as f64 * LANE_STAGGER, span)
        } else {
            near
        }
    };
    let (xa, xb) = (port(pa, sa), port(pb, sb));
    // Met on its side, an end is entered level with its centre, moved
    // one LANE_STAGGER toward the other end per lane nested round it
    // there (within the side's straight run): the lanes meeting one side
    // turn in one after the other instead of on top of each other, the
    // innermost nearest the other end, so none runs across another.
    let level = |p: &Placed, k: usize, toward: f64| {
        let room = (p.lsize / 2.0 - PORT_INSET).max(0.0);
        p.l + toward.signum() * (lane.stagger[k] as f64 * LANE_STAGGER).min(room)
    };
    let (la, lb) = (level(pa, 0, pb.l - pa.l), level(pb, 1, pa.l - pb.l));
    let flank = |p: &Placed, l: f64| (p.b + (x - p.b).signum() * p.bsize / 2.0, l);
    let mut c = Vec::with_capacity(6);
    if side[0] && (xa - x).abs() > LANE_EPS {
        c.push(flank(pa, la));
        c.push((x, la));
    } else {
        c.push(port_at(sa, pa, xa, true));
        if (xa - x).abs() > LANE_EPS {
            c.push((xa, s0));
            c.push((x, s0));
        }
    }
    if side[1] && (xb - x).abs() > LANE_EPS {
        c.push((x, lb));
        c.push(flank(pb, lb));
    } else {
        if (xb - x).abs() > LANE_EPS {
            c.push((x, s3));
            c.push((xb, s3));
        }
        c.push(port_at(sb, pb, xb, false));
    }
    c
}

/// Whether a lane at breadth `x` meets `end` on its side facing the lane
/// instead of across its flow side: when the lane passes outside the
/// end itself by [`LANE_CLEAR`], another edge on that flow side heads
/// out toward the lane (`fans`: the breadths the end's other edges there
/// head to) — the leg along the side would run beside its curve, as
/// beside a hub's fan — and nothing beside the end along the flow
/// (`beside`: breadth spans of the nodes level with it) lies between it
/// and the lane. A fork/join bar is always met across.
///
/// Outside the side's usable span ([`port_span`]) is not enough: a lane
/// within the end's own breadth runs on along the end to its centre
/// level through the box and meets the flank from inside, its head
/// painted under the end pointing out (r274: `rollback` into `Check
/// queue`, the lane 11 px inside its 38 px breadth). Past the flank by
/// [`LANE_CLEAR`], the turn in has room for a rounded corner and the
/// arrowhead square to the side.
pub(crate) fn meets_side(
    end: (&Placed, Shape),
    x: f64,
    fans: &[f64],
    beside: &[(f64, f64)],
) -> bool {
    let (p, shape) = end;
    if shape == Shape::ForkBar {
        return false;
    }
    let span = port_span(shape, p);
    let edge = if x > span.1 + LANE_EPS {
        span.1
    } else if x < span.0 - LANE_EPS {
        span.0
    } else {
        return false;
    };
    if (x - p.b).abs() < p.bsize / 2.0 + LANE_CLEAR {
        return false;
    }
    let out = x - edge;
    let (lo, hi) = (p.b.min(x) - DIRECT_CLEAR, p.b.max(x) + DIRECT_CLEAR);
    fans.iter().any(|&f| (f - edge) * out > 0.0)
        && !beside.iter().any(|&(s0, s1)| s0 < hi && lo < s1)
}

/// The polyline a lane at breadth `x` is judged by for crossings: from
/// the source's centre `ca` along its port's line to the leg at `la`,
/// across, along the lane, across at `lb`, to the target's centre `cb`
/// — in whatever coordinates the caller counts in (layer indices in the
/// layout, px in `route`). `spans` are the two ends' [`port_span`]s.
pub(crate) fn judged_path(
    x: f64,
    spans: [(f64, f64); 2],
    ca: f64,
    cb: f64,
    la: f64,
    lb: f64,
) -> Vec<Pt> {
    let (xa, xb) = (clamp_to(x, spans[0]), clamp_to(x, spans[1]));
    vec![(xa, ca), (xa, la), (x, la), (x, lb), (xb, lb), (xb, cb)]
}

/// The straight skeleton of a drawn edge another edge's lane may cross
/// (its end centres and channel points), with its ends.
pub(crate) struct Skeleton {
    /// Source and target node.
    pub ends: (usize, usize),
    /// The polyline.
    pub pts: Vec<Pt>,
    /// Its bounding box `(b0, l0, b1, l1)`.
    pub bbox: (f64, f64, f64, f64),
}

impl Skeleton {
    /// The skeleton through `pts` between nodes `ends`.
    pub(crate) fn new(ends: (usize, usize), pts: Vec<Pt>) -> Skeleton {
        let mut bbox = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for &(b, l) in &pts {
            bbox = (bbox.0.min(b), bbox.1.min(l), bbox.2.max(b), bbox.3.max(l));
        }
        Skeleton { ends, pts, bbox }
    }
}

/// Do segments `p`-`q` and `r`-`s` cross (properly, not at an end)?
fn seg_cross(p: Pt, q: Pt, r: Pt, s: Pt) -> bool {
    let side = |a: Pt, b: Pt, c: Pt| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let (d1, d2) = (side(p, q, r), side(p, q, s));
    let (d3, d4) = (side(r, s, p), side(r, s, q));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Crossings of `path` (an edge between nodes `ends`, as
/// [`judged_path`] lays it: a stub and a leg at each end, the lane
/// between; `spans` the two ends' [`port_span`]s) with the `skeletons`.
/// Near an end it shares with another edge, the two are spread apart by
/// the ports stage rather than crossed, so that end's stub and leg are
/// not judged against it (a twin, sharing both ends, not at all); past
/// them, the lane can still cross it. What decides there is where the
/// other edge heads from the shared end ([`crosses_at_end`]): its first
/// segment is judged by that alone.
pub(crate) fn crossings(
    path: &[Pt],
    ends: (usize, usize),
    spans: [(f64, f64); 2],
    skeletons: &[Skeleton],
) -> usize {
    let mut bb = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &(b, l) in path {
        bb = (bb.0.min(b), bb.1.min(l), bb.2.max(b), bb.3.max(l));
    }
    let segs = path.len().saturating_sub(1);
    let (Some(&first), Some(&last)) = (path.first(), path.last()) else {
        return 0;
    };
    // The lane's breadth: its first corner past the source's leg.
    let x = path.get(2).map_or(first.0, |p| p.0);
    let mut n = 0;
    for s in skeletons {
        let has = |v: usize| s.ends.0 == v || s.ends.1 == v;
        let (at_a, at_b) = (has(ends.0), has(ends.1));
        // Its first segment at each end it shares, judged by where it
        // heads ([`crosses_at_end`]) rather than by the straight line.
        let k = s.pts.len();
        let mut skip = (0, k.saturating_sub(1));
        for (shared, v, port, upstream, span) in [
            (at_a, ends.0, first, true, spans[0]),
            (at_b, ends.1, last, false, spans[1]),
        ] {
            if !shared || k < 2 {
                continue;
            }
            let (from_v, next) = if s.ends.0 == v {
                (true, s.pts[1])
            } else {
                (false, s.pts[k - 2])
            };
            if from_v {
                skip.0 = skip.0.max(1);
            } else {
                skip.1 = skip.1.min(k - 2);
            }
            // Only an edge on the flow side the lane leaves (enters) by.
            let same_side = if upstream {
                next.1 < port.1
            } else {
                next.1 > port.1
            };
            // A lane running straight into its target's downstream side
            // arrives against the edges leaving that side: its head among
            // their tails, it runs alongside each until it turns away (the
            // router then redraws the crowd round it). Each counts.
            let along = !upstream && (x - port.0).abs() <= LANE_EPS;
            if same_side {
                n += usize::from(along || crosses_at_end(port.0, x, span, next.0));
            }
        }
        if s.bbox.0 > bb.2 || s.bbox.2 < bb.0 || s.bbox.1 > bb.3 || s.bbox.3 < bb.1 {
            continue;
        }
        // A straight edge (a bare chord, as `route()` judges a free curve)
        // running along the flow within LANE_ON of the lane's run lies on
        // the lane, ends shared or not: drawn on top of each other (or a
        // band once nudged apart), worse than a crossing, so it counts as
        // one (r046 in `route()`: `Emit load ==> Parse` ran its lane 1 px
        // off the chord of `Parse -> Check report` with no crossing,
        // instead of round the top as in `scene()`). A channel's points
        // (the layout's skeletons) lie where the layering put the edge's
        // dummies, which a lane beside them passes as a parallel run.
        if let (2, Some(&r0), Some(&r1)) = (k, path.get(2), path.get(3)) {
            let (l0, l1) = (r0.1.min(r1.1), r0.1.max(r1.1));
            let (v0, v1) = (s.pts[0], s.pts[1]);
            if (v0.0 - x).abs() < LANE_ON
                && (v1.0 - x).abs() < LANE_ON
                && v0.1.min(v1.1) < l1
                && v0.1.max(v1.1) > l0
            {
                n += 1;
                continue;
            }
        }
        let from = if at_a { 2 } else { 0 };
        let to = if at_b { segs.saturating_sub(2) } else { segs };
        let rest = s.pts.get(skip.0..=skip.1).unwrap_or(&[]);
        for w in path.windows(2).take(to).skip(from) {
            for v in rest.windows(2) {
                if seg_cross(w[0], w[1], v[0], v[1]) {
                    n += 1;
                }
            }
        }
    }
    n
}

/// The skeletons of the `placed` lanes (want, breadth) want `i`'s lane
/// may cross: those whose band overlaps its own and that share no end
/// with it (lanes meeting at an end nest by their stagger instead), each
/// laid by `path` (want, breadth) as the caller judges a lane. Without
/// them a lane took the side its planned neighbour's leg runs across
/// (x2-nested-backs: `rollback` went over the top, across `retry`'s lane,
/// in `scene()` and under in `route()`).
pub(crate) fn planned_skeletons(
    wants: &[Want],
    placed: &[(usize, f64)],
    i: usize,
    path: impl Fn(usize, f64) -> Vec<Pt>,
) -> Vec<Skeleton> {
    let w = &wants[i];
    let (a, b) = w.ends;
    placed
        .iter()
        .filter(|&&(j, _)| {
            let o = &wants[j];
            o.band.0 < w.band.1
                && w.band.0 < o.band.1
                && ![o.ends.0, o.ends.1].iter().any(|&v| v == a || v == b)
        })
        .map(|&(j, x)| Skeleton::new(wants[j].ends, path(j, x)))
        .collect()
}

/// Whether a lane meeting an end at port breadth `xa` (within the end's
/// port `span`) and running at breadth `x` crosses another edge leaving
/// that end on the same flow side toward breadth `f`. The ports stage
/// puts the other edge's port on the side it heads to, so the two part
/// — unless the lane turns toward that side and the other edge heads on
/// past the lane (its leg, or the side entry [`meets_side`] gives it,
/// lies across the other's way), or the lane runs straight out at the
/// span's end on that side, leaving it no room there. (Judging the
/// straight line from the end's centre instead let a lane turn across
/// an edge arriving beside it: state `retry` across `invalid`.)
pub(crate) fn crosses_at_end(xa: f64, x: f64, span: (f64, f64), f: f64) -> bool {
    let d = x - xa;
    if d.abs() > LANE_EPS {
        (f - x) * d > 0.0
    } else {
        (f < xa - LANE_EPS && xa <= span.0 + LANE_EPS)
            || (f > xa + LANE_EPS && xa >= span.1 - LANE_EPS)
    }
}

/// The breadth a back-edge's lane runs at: past all the `obstacles`
/// (breadth spans of what reaches into its band) by `clear`, on the low
/// or the high side, at the spot that costs the fewest
/// [`LANE_CROSS_COST`]-priced crossings (`cross` counts them for a
/// breadth) plus way round — the legs from each end's nearest port
/// (`spans`, source then target) to the lane. The spots on a side are
/// its bound by the obstacles and the ends of the two spans past it; of
/// equal cost, the one nearest the obstacles, and of the two sides' best
/// the high one on a tie (the low one when `tie_low`). So the lane takes
/// the shortest way, within an
/// end's own span when that reaches past the obstacles (it leaves or
/// enters that end straight) — unless that way crosses more: then a spot
/// further out within an end's span, which runs straight out of the end
/// past an edge arriving beside it, or the other side. Never between two
/// obstacles: a gap there is a channel the forward edges use, and a lane
/// threading it crosses them near their ends where no skeleton tells
/// (mermaid loops round the outside too).
pub(crate) fn lane_at(
    obstacles: &[(f64, f64)],
    clear: f64,
    spans: [(f64, f64); 2],
    tie_low: bool,
    mut cross: impl FnMut(f64) -> usize,
) -> f64 {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(a, b) in obstacles {
        if a.is_finite() && b.is_finite() {
            lo = lo.min(a - clear);
            hi = hi.max(b + clear);
        }
    }
    let off = |x: f64, s: (f64, f64)| (s.0 - x).max(x - s.1).max(0.0);
    let detour = |x: f64| off(x, spans[0]) + off(x, spans[1]);
    // The best spot in `[f0, f1]` — a bound or an end of a span — by
    // cost (`priced`: with its crossings), nearest `near` among equals:
    // `(cost, x, crossings, way round)`. Spots are tried shortest
    // first, so one longer than the best cost so far is never priced.
    let mut spot = |f0: f64, f1: f64, near: f64, priced: bool| -> Option<(f64, f64, usize, f64)> {
        let mut xs: Vec<(f64, f64, f64)> = [f0, f1, spans[0].0, spans[0].1, spans[1].0, spans[1].1]
            .into_iter()
            .filter(|&x| x.is_finite() && x >= f0 && x <= f1)
            .map(|x| (detour(x), (x - near).abs(), x))
            .collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let mut best: Option<(f64, f64, f64, usize, f64)> = None;
        for (d, dist, x) in xs {
            if best.is_some_and(|b| d > b.0 + LANE_EPS || (b.2 - x).abs() <= LANE_EPS) {
                continue;
            }
            let n = if priced { cross(x) } else { 0 };
            let cost = n as f64 * LANE_CROSS_COST + d;
            let better = best.map_or(true, |b| {
                cost < b.0 - LANE_EPS || (cost <= b.0 + LANE_EPS && dist < b.1)
            });
            if better {
                best = Some((cost, dist, x, n, d));
            }
        }
        best.map(|(c, _, x, n, d)| (c, x, n, d))
    };
    if lo > hi {
        // Nothing in the way: the shortest spot anywhere.
        return spot(f64::NEG_INFINITY, f64::INFINITY, 0.0, false).map_or(0.0, |s| s.1);
    }
    let (low, high) = (
        spot(f64::NEG_INFINITY, lo, lo, true),
        spot(hi, f64::INFINITY, hi, true),
    );
    // Of the two sides' best, the cheaper; on a tie the high side, or
    // the low one when `tie_low`.
    match (low, high) {
        (Some(l), Some(h)) if l.0 < h.0 - LANE_EPS || tie_low && l.0 <= h.0 + LANE_EPS => l.1,
        (_, Some(h)) => h.1,
        (Some(l), None) => l.1,
        (None, None) => hi,
    }
}

/// Whether the direct curve across the gap (the forward curve run the
/// other way, leaving and entering along the flow) will not do for a
/// back-edge: its ends lie more than [`DIRECT_REACH`] apart across the
/// flow, or a node in its band stands in its way. `between` holds the breadth spans of
/// what lies on the layers strictly between the ends, which the curve
/// sweeps across from the source's port breadth `ports.0` to the
/// target's `ports.1`; `beside` those of the nodes on an end's own layer
/// reaching into the band, with whether that end is the source — near
/// an end the curve still runs at that end's port. `label` is half the
/// label's breadth (it rides the curve's middle, among the in-between
/// layers).
pub(crate) fn blocks_direct(
    between: &[(f64, f64)],
    beside: &[((f64, f64), bool)],
    ports: (f64, f64),
    label: f64,
) -> bool {
    let near = |s: (f64, f64), lo: f64, hi: f64, c: f64| s.0 < hi + c && lo - c < s.1;
    let (lo, hi) = (ports.0.min(ports.1), ports.0.max(ports.1));
    hi - lo > DIRECT_REACH
        || between
            .iter()
            .any(|&s| near(s, lo, hi, DIRECT_CLEAR + label))
        || beside.iter().any(|&(s, src)| {
            let p = if src { ports.0 } else { ports.1 };
            near(s, p, p, DIRECT_CLEAR)
        })
}

/// A planned lane ([`plan`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Lane {
    /// Its breadth.
    pub x: f64,
    /// It kept beside its box: it cleared only its obstacles, not the
    /// foreign spans.
    pub local: bool,
    /// Per end (source, target): how many lanes planned round it (on its
    /// side) leave or enter the same layer — its stub there is that many
    /// [`LANE_STAGGER`]s longer ([`legs`]).
    pub stagger: [usize; 2],
    /// Per end: the free gap along the flow beyond its side ([`room`]),
    /// which caps the stub; set by the caller that draws the lane
    /// (infinite from [`plan`]).
    pub room: [f64; 2],
    /// Per end: the least stub that takes the leg [`LANE_STUB`] past the
    /// border of a frame the lane leaves the end's box through, when
    /// that border lies just beyond the end's side (a leg there would
    /// run along the border); set by the caller that draws the lane (0
    /// from [`plan`]).
    pub out: [f64; 2],
    /// How many lanes of edges between the same two ends run farther out
    /// on its side: its ports sit that many [`LANE_STAGGER`]s further in
    /// at both ends ([`corners`]), so twins leave and enter in their
    /// nesting order.
    pub twins: usize,
}

impl Lane {
    /// A lane at breadth `x` with no stagger and no room limit.
    pub(crate) fn plain(x: f64) -> Lane {
        Lane {
            x,
            local: true,
            stagger: [0, 0],
            room: [f64::INFINITY; 2],
            out: [0.0; 2],
            twins: 0,
        }
    }
}

/// One back-edge for [`plan`], in the caller's coordinates.
pub(crate) struct Want {
    /// Planning order (innermost first): the length of its band.
    pub key: f64,
    /// Its band along the flow, `(upstream end, downstream end)`: two
    /// lanes are in each other's way only where their bands overlap.
    pub band: (f64, f64),
    /// Breadth spans of what reaches into the band and is in the lane's
    /// way: everything there, or (inside a subgraph) the members of the
    /// innermost box holding both ends.
    pub obstacles: Vec<(f64, f64)>,
    /// Breadth spans of the rest of what reaches into the band (outside
    /// that box): the lane keeps beside the box while it clears them,
    /// else it goes round them too.
    pub foreign: Vec<(f64, f64)>,
    /// Whether something in the band stands in the direct curve's way
    /// ([`blocks_direct`]); if not, the edge is drawn across the gap.
    pub blocked: bool,
    /// [`LANE_CLEAR`] plus half the label's breadth.
    pub clear: f64,
    /// Half the label's breadth: what a later lane clears round this.
    pub label_half: f64,
    /// The two ends' [`port_span`]s, source then target.
    pub spans: [(f64, f64); 2],
    /// The two end nodes, source then target.
    pub ends: (usize, usize),
}

/// Plan the lanes of `wants`: each one's [`Lane`], or `None` for one
/// drawn directly (nothing in its band stands in the direct curve's
/// way). Innermost (shortest band) first — of equal bands, the one
/// whose ends lie farthest from what it passes — each planned lane an
/// obstacle to the later ones sharing its band; lanes on one side
/// leaving or entering one layer then stagger their stubs, the inner
/// lane's longest, so the legs nest. `cross(i, x, placed)` counts the
/// crossings of want `i`'s lane at breadth `x`, the lanes planned so far
/// (`placed`: want, breadth) among what it may cross (see
/// [`planned_skeletons`]).
pub(crate) fn plan(
    wants: &[Want],
    mut cross: impl FnMut(usize, f64, &[(usize, f64)]) -> usize,
) -> Vec<Option<Lane>> {
    let mut order: Vec<usize> = (0..wants.len()).collect();
    // Of lanes with one band, the one whose ends lie farthest from what
    // they pass goes innermost: nested round it, the others (their ends
    // nearer) never cross its legs.
    let block = |w: &Want| {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for &(a, b) in w.obstacles.iter().chain(&w.foreign) {
            lo = lo.min(a);
            hi = hi.max(b);
        }
        (lo, hi)
    };
    let far = |w: &Want| {
        let (lo, hi) = block(w);
        if lo > hi {
            return 0.0;
        }
        let mid = (lo + hi) / 2.0;
        w.spans
            .iter()
            .map(|s| ((s.0 + s.1) / 2.0 - mid).abs())
            .fold(0.0, f64::max)
    };
    order.sort_by(|&i, &j| {
        wants[i]
            .key
            .total_cmp(&wants[j].key)
            .then(far(&wants[j]).total_cmp(&far(&wants[i])))
            .then(i.cmp(&j))
    });
    let mut out: Vec<Option<Lane>> = vec![None; wants.len()];
    let mut placed: Vec<(usize, f64)> = Vec::new();
    let hits =
        |spans: &[(f64, f64)], lo: f64, hi: f64| spans.iter().any(|&(s0, s1)| s0 < hi && lo < s1);
    for &i in &order {
        let w = &wants[i];
        if !w.blocked {
            continue;
        }
        // Each planned lane sharing its band is in its way — but one
        // running straight out of its own end (no leg there) beside this
        // one's end, into the end they share, runs alongside it rather
        // than round it: lanes from ends stacked on one layer into one
        // node run side by side out of their ends (they turn into the
        // shared end one after the other), so this lane may too, when
        // it also runs straight out of its own end.
        let legless = |j: usize, x: f64, k: usize| {
            let s = wants[j].spans[k];
            x >= s.0 - LANE_EPS && x <= s.1 + LANE_EPS
        };
        let beside = |j: usize, x: f64| -> Option<usize> {
            let o = &wants[j];
            if o.band != w.band {
                return None;
            }
            if o.ends.1 == w.ends.1 && o.ends.0 != w.ends.0 && legless(j, x, 0) {
                Some(0)
            } else if o.ends.0 == w.ends.0 && o.ends.1 != w.ends.1 && legless(j, x, 1) {
                Some(1)
            } else {
                None
            }
        };
        let mut obs = w.obstacles.clone();
        let mut alongside: Vec<((f64, f64), usize)> = Vec::new();
        for &(j, x) in &placed {
            let o = &wants[j];
            if o.band.0 < w.band.1 && w.band.0 < o.band.1 {
                let span = (x - o.label_half, x + o.label_half);
                match beside(j, x) {
                    Some(k) => alongside.push((span, k)),
                    None => obs.push(span),
                }
            }
        }
        // Of two sides that cost the same, the one the planned lanes
        // sharing an end and its band with it took: nested round them
        // there, it leaves the other side to the lanes still to come
        // (x2-nested-backs: `fail` beside `retry`, out of one end, so
        // `rollback` loops the other way crossing nothing, in `scene()`
        // and `route()` alike).
        let mid = (w.spans[0].0 + w.spans[0].1 + w.spans[1].0 + w.spans[1].1) / 4.0;
        let kin = placed
            .iter()
            .filter(|&&(j, _)| {
                let o = &wants[j];
                o.band.0 < w.band.1
                    && w.band.0 < o.band.1
                    && [o.ends.0, o.ends.1]
                        .iter()
                        .any(|&v| v == w.ends.0 || v == w.ends.1)
            })
            .map(|&(_, x)| if x < mid { -1 } else { 1 })
            .sum::<i32>();
        let tie_low = kin < 0;
        let mut x = lane_at(&obs, w.clear, w.spans, tie_low, |x| cross(i, x, &placed));
        let reach = (x - w.label_half - LANE_CLEAR, x + w.label_half + LANE_CLEAR);
        let local = !hits(&w.foreign, reach.0, reach.1);
        if !local {
            obs.extend_from_slice(&w.foreign);
            x = lane_at(&obs, w.clear, w.spans, tie_low, |x| cross(i, x, &placed));
        }
        // Not straight out of its own end: round those lanes too.
        if alongside.iter().any(|&(_, k)| !legless(i, x, k)) {
            obs.extend(alongside.iter().map(|&(s, _)| s));
            x = lane_at(&obs, w.clear, w.spans, tie_low, |x| cross(i, x, &placed));
        }
        out[i] = Some(Lane {
            x,
            local,
            stagger: [0, 0],
            room: [f64::INFINITY; 2],
            out: [0.0; 2],
            twins: 0,
        });
        placed.push((i, x));
    }
    // Lanes of one band on one side, leaving one layer from different
    // ends, nest only one way round: the end nearest the side they run
    // to takes the lane farthest out (and, below, the shortest stub), so
    // every other leg turns before it and no lane runs across another's
    // leg. Planned one by one, the lanes came out in any order (the
    // first within its end's own span, the others round it across its
    // run), so their breadths are dealt out again in that order.
    let side = |i: usize, x: f64| {
        let (lo, hi) = block(&wants[i]);
        (x - (lo + hi) / 2.0).signum()
    };
    let centre = |s: (f64, f64)| (s.0 + s.1) / 2.0;
    let mut dealt = vec![false; wants.len()];
    for &(i, xi) in &placed {
        if dealt[i] {
            continue;
        }
        let w = &wants[i];
        let group: Vec<usize> = placed
            .iter()
            .filter(|&&(j, xj)| {
                let o = &wants[j];
                !dealt[j]
                    && o.band == w.band
                    && o.label_half == w.label_half
                    && side(j, xj) == side(i, xi)
                    && out[j].map(|l| l.local) == out[i].map(|l| l.local)
            })
            .map(|&(j, _)| j)
            .collect();
        for &j in &group {
            dealt[j] = true;
        }
        if group.len() < 2 {
            continue;
        }
        let xs: Vec<f64> = group.iter().filter_map(|&j| out[j].map(|l| l.x)).collect();
        let sum_b: f64 = group.iter().map(|&j| centre(wants[j].spans[0])).sum();
        let dir = (xs.iter().sum::<f64>() - sum_b).signum();
        let mut ends = group.clone();
        ends.sort_by(|&a, &b| {
            let (ba, bb) = (centre(wants[a].spans[0]), centre(wants[b].spans[0]));
            (bb * dir).total_cmp(&(ba * dir)).then(a.cmp(&b))
        });
        let mut lanes = xs;
        lanes.sort_by(|a, b| (b * dir).total_cmp(&(a * dir)));
        for (&j, &x) in ends.iter().zip(&lanes) {
            if let Some(l) = out[j].as_mut() {
                l.x = x;
            }
        }
    }
    // Stagger: per lane end, the lanes on its side sharing that end's
    // layer that run farther out from their ends — its stub is that many
    // LANE_STAGGERs longer, so the lane farthest out turns first and the
    // nested legs never cross (at a shared end as at ends side by side).
    let laid: Vec<(usize, f64)> = placed
        .iter()
        .filter_map(|&(i, _)| out[i].map(|l| (i, l.x)))
        .collect();
    for &(i, xi) in &laid {
        let mut stagger = [0usize; 2];
        let twins = laid
            .iter()
            .filter(|&&(j, xj)| {
                j != i
                    && wants[j].ends == wants[i].ends
                    && side(j, xj) == side(i, xi)
                    && (xj - xi) * side(i, xi) > LANE_EPS
            })
            .count();
        for (k, layer) in [(0, wants[i].band.1), (1, wants[i].band.0)] {
            let out_dir = |j: usize, x: f64| {
                let d = x - centre(wants[j].spans[k]);
                if d.abs() > LANE_EPS {
                    d.signum()
                } else {
                    side(j, x)
                }
            };
            let di = out_dir(i, xi);
            for &(j, xj) in &laid {
                let lj = if k == 0 {
                    wants[j].band.1
                } else {
                    wants[j].band.0
                };
                if j == i || lj != layer || side(j, xj) != side(i, xi) || out_dir(j, xj) != di {
                    continue;
                }
                stagger[k] += usize::from((xj - xi) * di > LANE_EPS);
            }
        }
        if let Some(l) = out[i].as_mut() {
            l.stagger = stagger;
            l.twins = twins;
        }
    }
    out
}

/// [`seg_cross`] for other modules' tests.
#[cfg(test)]
pub(crate) fn tests_seg_cross(p: Pt, q: Pt, r: Pt, s: Pt) -> bool {
    seg_cross(p, q, r, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(b: f64, l: f64, bsize: f64, lsize: f64) -> Placed {
        Placed {
            b,
            l,
            bsize,
            lsize,
            layer: 0,
        }
    }

    #[test]
    fn a_tied_lane_takes_the_side_asked_for() {
        // Both sides equally far and crossing nothing: the high one by
        // default, the low one when `tie_low` (the side the planned
        // lanes sharing its end took, see `plan`).
        let spans = [(40.0, 60.0), (40.0, 60.0)];
        let obs = [(0.0, 100.0)];
        assert!(lane_at(&obs, LANE_CLEAR, spans, false, |_| 0) > 100.0);
        assert!(lane_at(&obs, LANE_CLEAR, spans, true, |_| 0) < 0.0);
        // Not a tie: the cheaper side whatever is asked.
        let x = lane_at(&obs, LANE_CLEAR, spans, true, |x| usize::from(x < 0.0));
        assert!(x > 100.0);
    }

    #[test]
    fn a_lane_clears_only_what_lies_between_and_hugs_it() {
        // One obstacle 0..100: the lane runs LANE_CLEAR past its nearer
        // side to the ends, inside the end's own span when it reaches.
        let spans = [(90.0, 180.0), (40.0, 60.0)];
        let x = lane_at(&[(0.0, 100.0)], LANE_CLEAR, spans, false, |_| 0);
        assert_eq!(x, 100.0 + LANE_CLEAR);
        // A crossing on that side sends it round the other.
        let x = lane_at(&[(0.0, 100.0)], LANE_CLEAR, spans, false, |x| {
            usize::from(x > 50.0)
        });
        assert_eq!(x, -LANE_CLEAR);
        // Never through a gap between two obstacles, however short.
        let spans = [(140.0, 160.0), (140.0, 160.0)];
        let x = lane_at(
            &[(0.0, 100.0), (200.0, 300.0)],
            LANE_CLEAR,
            spans,
            false,
            |_| 0,
        );
        assert_eq!(x, 300.0 + LANE_CLEAR);
    }

    #[test]
    fn a_lane_leaves_and_enters_along_the_sides_normal() {
        let a = placed(100.0, 300.0, 120.0, 40.0);
        let b = placed(60.0, 100.0, 120.0, 40.0);
        // The lane within a's straight run: straight out of it.
        let lane = Lane::plain;
        let c = corners(
            (&a, Shape::Rect),
            (&b, Shape::Rect),
            lane(140.0),
            [false; 2],
        );
        assert_eq!(c[0], (140.0, 280.0));
        assert_eq!(c[1], (140.0, 120.0 + LANE_STUB));
        assert_eq!(c[2], (108.0, 120.0 + LANE_STUB));
        assert_eq!(c[3], (108.0, 120.0));
        // Every leg is axis-aligned.
        for side in [[false; 2], [true; 2]] {
            let c = corners((&a, Shape::Diamond), (&b, Shape::Rect), lane(300.0), side);
            assert!(
                c.windows(2).all(|w| w[0].0 == w[1].0 || w[0].1 == w[1].1),
                "{c:?}"
            );
        }
        // A hub's fan heading out toward the lane: met on the side.
        let fans = [250.0];
        assert!(meets_side((&b, Shape::Rect), 300.0, &fans, &[]));
        let c = corners(
            (&a, Shape::Rect),
            (&b, Shape::Rect),
            lane(300.0),
            [false, true],
        );
        assert_eq!(&c[c.len() - 2..], &[(300.0, 100.0), (120.0, 100.0)]);
        // Not with a node beside it in the way, nor without such a fan.
        assert!(!meets_side(
            (&b, Shape::Rect),
            300.0,
            &fans,
            &[(200.0, 240.0)]
        ));
        assert!(!meets_side((&b, Shape::Rect), 300.0, &[60.0], &[]));
        // Not within the end's own breadth (0..120), though outside its
        // ports' span (12..108): the lane would run on along the end
        // through it and meet the flank from inside (r274). Nor just past
        // the flank: the turn in needs LANE_CLEAR for its corner and head.
        assert!(!meets_side((&b, Shape::Rect), 115.0, &fans, &[]));
        assert!(!meets_side(
            (&b, Shape::Rect),
            120.0 + LANE_CLEAR - 1.0,
            &fans,
            &[]
        ));
        assert!(meets_side(
            (&b, Shape::Rect),
            120.0 + LANE_CLEAR,
            &[250.0],
            &[]
        ));
    }

    #[test]
    fn a_lane_counts_the_edge_it_turns_across_at_a_shared_end() {
        // The lane leaves its end at port 100 (span 80..120) and turns
        // left to run at 40: an edge from that end heading to 20 lies
        // past the lane (crossed), one heading to 60 between (the side
        // entry or the ports part them), one heading right never.
        let span = (80.0, 120.0);
        assert!(crosses_at_end(80.0, 40.0, span, 20.0));
        assert!(!crosses_at_end(80.0, 40.0, span, 60.0));
        assert!(!crosses_at_end(80.0, 40.0, span, 200.0));
        // Straight out at the span's left end: no room left of it for an
        // edge heading left; at its right end, none heading right.
        assert!(crosses_at_end(80.0, 80.0, span, 20.0));
        assert!(!crosses_at_end(120.0, 120.0, span, 20.0));
        assert!(crosses_at_end(120.0, 120.0, span, 200.0));
        // Judged on a path: state `retry` from Rejected (node 0) to Idle
        // (node 1) with `invalid` (2 -> 0) arriving from upper left: the
        // short way left of Rejected crosses it, straight up does not.
        let spans = [(318.5, 381.7), (158.3, 265.3)];
        let invalid = Skeleton::new((2, 0), vec![(211.8, 5.0), (350.1, 6.0)]);
        let short = judged_path(294.0, spans, 6.0, 1.0, 5.5, 1.5);
        assert_eq!(
            crossings(&short, (0, 1), spans, std::slice::from_ref(&invalid)),
            1
        );
        let out = judged_path(381.7, spans, 6.0, 1.0, 5.5, 1.5);
        assert_eq!(
            crossings(&out, (0, 1), spans, std::slice::from_ref(&invalid)),
            0
        );
        let x = lane_at(&[(150.0, 274.0)], LANE_CLEAR, spans, false, |x| {
            let p = judged_path(x, spans, 6.0, 1.0, 5.5, 1.5);
            crossings(&p, (0, 1), spans, std::slice::from_ref(&invalid))
        });
        assert_eq!(x, 381.7);
    }

    #[test]
    fn a_lane_straight_into_its_target_counts_the_edges_leaving_beside_it() {
        // bench-75's job1 -> svc1 in route(): straight into svc1's
        // downstream side, the lane arrives against an edge leaving that
        // side (target node 1, the edge to node 5 downstream of it): it
        // counts. With a leg there instead, an edge heading away from
        // the lane's side does not.
        let spans = [(1731.0, 1745.0), (451.0, 465.0)];
        let out = Skeleton::new((1, 5), vec![(458.0, 1.0), (900.0, 3.0)]);
        let one = std::slice::from_ref(&out);
        let straight = judged_path(451.0, spans, 6.0, 1.0, 5.5, 1.5);
        assert_eq!(crossings(&straight, (0, 1), spans, one), 1);
        let leg = judged_path(1800.0, spans, 6.0, 1.0, 5.5, 1.5);
        assert_eq!(crossings(&leg, (0, 1), spans, one), 0);
    }

    #[test]
    fn lanes_from_stacked_ends_into_one_node_run_side_by_side() {
        // bench-75's jobs: three ends stacked across the flow on one
        // layer, each with a back-edge into one node, what they pass all
        // on one side. A spot within an end's own span is the cheaper one
        // (a leg there would cross the edges into the ends): each lane
        // runs straight out of its own end beside the others — the
        // planned ones no obstacle to it — and they turn into the shared
        // end one after the other, the one nearest it first (its stub
        // the longest), so no lane crosses another's leg.
        let want = |src: usize, b: f64| Want {
            key: 2.0,
            band: (0.0, 2.0),
            obstacles: vec![(0.0, 100.0)],
            foreign: Vec::new(),
            blocked: true,
            clear: LANE_CLEAR,
            label_half: 0.0,
            spans: [(b - 7.0, b + 7.0), (40.0, 54.0)],
            ends: (src, 9),
        };
        let wants = [want(1, 200.0), want(2, 290.0), want(3, 380.0)];
        let lanes = plan(&wants, |i, x, _| {
            let s = wants[i].spans[0];
            usize::from(x < s.0 - 1e-9 || x > s.1 + 1e-9)
        });
        for (w, l) in wants.iter().zip(&lanes) {
            let x = l.unwrap().x;
            assert!(x >= w.spans[0].0 && x <= w.spans[0].1, "{x}: {lanes:?}");
        }
        let st: Vec<usize> = lanes.iter().map(|l| l.unwrap().stagger[1]).collect();
        assert_eq!(st, vec![2, 1, 0], "{lanes:?}");
        // Planned round each other instead (no cheaper spot of their
        // own), they are dealt out so the end nearest their side takes
        // the lane farthest out, with the shortest stub.
        let lanes = plan(&wants, |_, _, _| 0);
        let xs: Vec<f64> = lanes.iter().map(|l| l.unwrap().x).collect();
        assert!(xs[0] < xs[1] && xs[1] < xs[2], "{xs:?}");
        let st: Vec<usize> = lanes.iter().map(|l| l.unwrap().stagger[0]).collect();
        assert_eq!(st, vec![0, 1, 2], "{lanes:?}");
    }

    #[test]
    fn a_far_reaching_back_edge_takes_a_lane() {
        // Nothing between its ends, but its ports lie DIRECT_REACH apart
        // across the flow: no reversed S sweeping along the gap.
        assert!(!blocks_direct(&[], &[], (0.0, DIRECT_REACH), 0.0));
        assert!(blocks_direct(&[], &[], (0.0, DIRECT_REACH + 1.0), 0.0));
    }

    #[test]
    fn nested_lanes_keep_apart_innermost_first() {
        let want = |key: f64, band: (f64, f64)| Want {
            key,
            band,
            obstacles: vec![(0.0, 100.0)],
            foreign: Vec::new(),
            blocked: true,
            clear: LANE_CLEAR,
            label_half: 0.0,
            spans: [(110.0, 130.0), (110.0, 130.0)],
            ends: (0, 1),
        };
        let lanes = plan(&[want(3.0, (0.0, 3.0)), want(1.0, (1.0, 2.0))], |_, _, _| 0);
        let (outer, inner) = (lanes[0].unwrap().x, lanes[1].unwrap().x);
        assert_eq!(inner, 100.0 + LANE_CLEAR);
        assert_eq!(outer, inner + LANE_CLEAR);
        // Nothing between the ends: drawn directly.
        let mut w = want(1.0, (0.0, 1.0));
        w.blocked = false;
        assert_eq!(plan(&[w], |_, _, _| 0), vec![None]);
    }
}
