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
        want.min((lane.room[k] - NODE_SEP).max(LANE_STUB))
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
/// facing the lane, level with its centre: the lane runs on to there
/// and turns straight in.
pub(crate) fn corners(
    a: (&Placed, Shape),
    b: (&Placed, Shape),
    lane: Lane,
    side: [bool; 2],
) -> Vec<Pt> {
    let ((pa, sa), (pb, sb)) = (a, b);
    let x = lane.x;
    let (s0, s3) = legs(pa, pb, &lane);
    let xa = clamp_to(x, port_span(sa, pa));
    let xb = clamp_to(x, port_span(sb, pb));
    let flank = |p: &Placed| (p.b + (x - p.b).signum() * p.bsize / 2.0, p.l);
    let mut c = Vec::with_capacity(6);
    if side[0] && (xa - x).abs() > LANE_EPS {
        c.push(flank(pa));
        c.push((x, pa.l));
    } else {
        c.push(port_at(sa, pa, xa, true));
        if (xa - x).abs() > LANE_EPS {
            c.push((xa, s0));
            c.push((x, s0));
        }
    }
    if side[1] && (xb - x).abs() > LANE_EPS {
        c.push((x, pb.l));
        c.push(flank(pb));
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
/// instead of across its flow side: when the lane passes outside that
/// side's usable span, another edge on that flow side heads out toward
/// the lane (`fans`: the breadths the end's other edges there head to)
/// — the leg along the side would run beside its curve, as beside a
/// hub's fan — and nothing beside the end along the flow (`beside`:
/// breadth spans of the nodes level with it) lies between it and the
/// lane. A fork/join bar is always met across.
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
/// between) with the `skeletons`. Near an end it shares with another
/// edge, the two are spread apart by the ports stage rather than
/// crossed, so that end's stub and leg are not judged against it (a
/// twin, sharing both ends, not at all); past them, the lane can still
/// cross it.
pub(crate) fn crossings(path: &[Pt], ends: (usize, usize), skeletons: &[Skeleton]) -> usize {
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
    let mut n = 0;
    for s in skeletons {
        if s.bbox.0 > bb.2 || s.bbox.2 < bb.0 || s.bbox.1 > bb.3 || s.bbox.3 < bb.1 {
            continue;
        }
        let has = |v: usize| s.ends.0 == v || s.ends.1 == v;
        let from = if has(ends.0) { 2 } else { 0 };
        let to = if has(ends.1) {
            segs.saturating_sub(2)
        } else {
            segs
        };
        for w in path.windows(2).take(to).skip(from) {
            for v in s.pts.windows(2) {
                if seg_cross(w[0], w[1], v[0], v[1]) {
                    n += 1;
                }
            }
        }
    }
    n
}

/// The breadth a back-edge's lane runs at: past all the `obstacles`
/// (breadth spans of what reaches into its band) by `clear`, on the low
/// or the high side, whichever costs fewer [`LANE_CROSS_COST`]-priced
/// crossings (`cross` counts them for a breadth) plus way round — the
/// legs from each end's nearest port (`spans`, source then target) to
/// the lane; ties go to the high side. On each side the lane takes the
/// shortest spot, the one nearest the obstacles among equals: within an
/// end's own span when that reaches past them, so the lane leaves or
/// enters it straight. Never between two obstacles: a gap there is a
/// channel the forward edges use, and a lane threading it crosses them
/// near their ends where no skeleton tells (mermaid loops round the
/// outside too).
pub(crate) fn lane_at(
    obstacles: &[(f64, f64)],
    clear: f64,
    spans: [(f64, f64); 2],
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
    // The shortest spot in `[f0, f1]`, nearest the obstacles' side
    // (`near`) among equals: a bound or an end of a span.
    let spot = |f0: f64, f1: f64, near: f64| -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64, f64)> = None;
        for x in [f0, f1, spans[0].0, spans[0].1, spans[1].0, spans[1].1] {
            if !x.is_finite() || x < f0 || x > f1 {
                continue;
            }
            let k = (detour(x), (x - near).abs(), x);
            let better = best.map_or(true, |b| {
                k.0 < b.0 - LANE_EPS || (k.0 <= b.0 + LANE_EPS && k.1 < b.1)
            });
            if better {
                best = Some(k);
            }
        }
        best.map(|(d, _, x)| (d, x))
    };
    if lo > hi {
        // Nothing in the way: the shortest spot anywhere.
        return spot(f64::NEG_INFINITY, f64::INFINITY, 0.0).map_or(0.0, |s| s.1);
    }
    let sides = [spot(f64::NEG_INFINITY, lo, lo), spot(hi, f64::INFINITY, hi)];
    let mut best: Option<(f64, f64)> = None; // (cost, x)
    for (d, x) in sides.into_iter().flatten() {
        let cost = cross(x) as f64 * LANE_CROSS_COST + d;
        if best.map_or(true, |(bc, _)| cost <= bc + LANE_EPS) {
            best = Some((cost, x));
        }
    }
    best.map_or(hi, |b| b.1)
}

/// Whether a node in a back-edge's band stands in the way of the direct
/// curve across the gap (the forward curve run the other way, leaving
/// and entering along the flow): `between` holds the breadth spans of
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
    between
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
}

impl Lane {
    /// A lane at breadth `x` with no stagger and no room limit.
    pub(crate) fn plain(x: f64) -> Lane {
        Lane {
            x,
            local: true,
            stagger: [0, 0],
            room: [f64::INFINITY; 2],
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
}

/// Plan the lanes of `wants`: each one's [`Lane`], or `None` for one
/// drawn directly (nothing in its band stands in the direct curve's
/// way). Innermost (shortest band) first — of equal bands, the one
/// whose ends lie farthest from what it passes — each planned lane an
/// obstacle to the later ones sharing its band; lanes on one side
/// leaving or entering one layer then stagger their stubs, the inner
/// lane's longest, so the legs nest. `cross(i, x)` counts the crossings
/// of want `i`'s lane at breadth `x`.
pub(crate) fn plan(
    wants: &[Want],
    mut cross: impl FnMut(usize, f64) -> usize,
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
        let mut obs = w.obstacles.clone();
        for &(j, x) in &placed {
            let o = &wants[j];
            if o.band.0 < w.band.1 && w.band.0 < o.band.1 {
                obs.push((x - o.label_half, x + o.label_half));
            }
        }
        let mut x = lane_at(&obs, w.clear, w.spans, |x| cross(i, x));
        let reach = (x - w.label_half - LANE_CLEAR, x + w.label_half + LANE_CLEAR);
        let local = !hits(&w.foreign, reach.0, reach.1);
        if !local {
            obs.extend_from_slice(&w.foreign);
            x = lane_at(&obs, w.clear, w.spans, |x| cross(i, x));
        }
        out[i] = Some(Lane {
            x,
            local,
            stagger: [0, 0],
            room: [f64::INFINITY; 2],
        });
        placed.push((i, x));
    }
    // Stagger: per lane end, the lanes planned after it (round it) on its
    // side of what it passes that share that end's layer.
    for (k, &(i, xi)) in placed.iter().enumerate() {
        let (lo, hi) = block(&wants[i]);
        let mid = (lo + hi) / 2.0;
        let mut stagger = [0usize; 2];
        for &(j, xj) in &placed[k + 1..] {
            if (xi - mid) * (xj - mid) <= 0.0 {
                continue;
            }
            let (bi, bj) = (wants[i].band, wants[j].band);
            stagger[0] += usize::from(bi.1 == bj.1);
            stagger[1] += usize::from(bi.0 == bj.0);
        }
        if let Some(l) = out[i].as_mut() {
            l.stagger = stagger;
        }
    }
    out
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
    fn a_lane_clears_only_what_lies_between_and_hugs_it() {
        // One obstacle 0..100: the lane runs LANE_CLEAR past its nearer
        // side to the ends, inside the end's own span when it reaches.
        let spans = [(90.0, 180.0), (40.0, 60.0)];
        let x = lane_at(&[(0.0, 100.0)], LANE_CLEAR, spans, |_| 0);
        assert_eq!(x, 100.0 + LANE_CLEAR);
        // A crossing on that side sends it round the other.
        let x = lane_at(&[(0.0, 100.0)], LANE_CLEAR, spans, |x| {
            usize::from(x > 50.0)
        });
        assert_eq!(x, -LANE_CLEAR);
        // Never through a gap between two obstacles, however short.
        let spans = [(140.0, 160.0), (140.0, 160.0)];
        let x = lane_at(&[(0.0, 100.0), (200.0, 300.0)], LANE_CLEAR, spans, |_| 0);
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
        };
        let lanes = plan(&[want(3.0, (0.0, 3.0)), want(1.0, (1.0, 2.0))], |_, _| 0);
        let (outer, inner) = (lanes[0].unwrap().x, lanes[1].unwrap().x);
        assert_eq!(inner, 100.0 + LANE_CLEAR);
        assert_eq!(outer, inner + LANE_CLEAR);
        // Nothing between the ends: drawn directly.
        let mut w = want(1.0, (0.0, 1.0));
        w.blocked = false;
        assert_eq!(plan(&[w], |_, _| 0), vec![None]);
    }
}
