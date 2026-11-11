//! Crossing hops (report M15, §2.10): where a level leg of one
//! flowchart edge crosses a plumb leg of another, the level one steps
//! over on a small semicircle, so both strokes stay whole — a dotted
//! edge keeps its dots, a solid one never reads as dashed, and which
//! line is which is plain at a glance. (The casing gap of
//! [`crate::casing`] cut one of the two; on a dense drawing every third
//! line came out chopped.)
//!
//! One rule decides every hop, so a drawing reads alike everywhere and
//! a line never hops at one crossing and ducks at the next: **the level
//! leg hops, upwards**. Neighbouring hops of one leg closer than a
//! radius join into one bridge, flat on top, over the whole bundle they
//! cross. A crossing gets a hop only where both strokes run straight
//! through it — clear of the level leg's rounded corners, of the
//! arrowheads and of the ends, a little way past a corner of the line
//! stepped over ([`HOP_OVER_CORNER`]) — and the hop's ink runs into
//! nothing: no other level line right above, no node, label chip, frame
//! border or title, no curve passing by ([`Legs::clear`]), and its feet
//! come down clear of any crossing that takes no hop. Every other
//! crossing — a curve's, one in a corner's rounding or under a head —
//! keeps the casing's gap, which next to a hop is cut into the other
//! line ([`crate::casing`]), so a line that steps over one crossing
//! never ducks under the next.
//!
//! A hop is paint: [`crate::scene::to_svg`] strokes it and picking
//! ([`crate::scene::Scene::edge_at`]) covers it, but the scene's
//! geometry stays the plain path, which is what a host draws from. The
//! painter and a pick work the hops out from the same legs by the same
//! rule ([`Legs::leg_hops`]), which reads them in one order however
//! they are listed: pick == render.

use crate::router::{ortho_corners_of, CORNER_R};
use crate::scene::{edge_drawn, edge_head_len, edge_stroke_width, Scene, SceneEdge};

type Pt = (f64, f64);
type Rect = (f64, f64, f64, f64);

/// Radius (px) of a hop over a default stroke: the under-line keeps
/// 3 px of air either side between the two strokes, about the casing
/// gap's, and two hops fit side by side on neighbouring tracks.
pub(crate) const HOP_R: f64 = 5.0;
/// Air (px) a hop keeps between its stroke and the stroke it steps
/// over, which sets the radius over wider strokes (a thick `==>` edge,
/// a `linkStyle stroke-width`).
const HOP_AIR: f64 = 3.0;
/// Widest hop (px of radius): a crossing of strokes too wide for it
/// keeps the casing's gap.
pub(crate) const HOP_R_MAX: f64 = 9.0;
/// Two hops of one leg whose arcs come closer than this (px) join into
/// one bridge: a flat between them shorter than a radius reads as a
/// kink, not as a stretch of line.
pub(crate) const HOP_JOIN: f64 = HOP_R;
/// Room (px) a hop keeps past a corner's rounding ([`CORNER_R`]) on its
/// own leg, so its arcs spring from the straight stretch.
const HOP_CORNER_AIR: f64 = 2.0;
/// On a long leg the stroke leaves its line sooner before a corner: the
/// spline through a path's waypoints turns over the whole stretch from
/// the leg's far end, and lies 0.3 px off the line a 50th of the leg
/// and [`HOP_LONG_AIR`] before the corner — the room of a rounding and
/// its air on a leg of 470 px, 42 px on one of 1 800 px. A foot keeps
/// that far from the corner, so both its arcs spring from the line and
/// come out the quarter circles they are (B11 review r2 #5: dense-td's
/// `n16 -> n23`, a foot 24 px before the corner of a leg 1 792 px long
/// stood 0.5 px off its line).
const HOP_LONG_LEG: f64 = 50.0;
/// See [`HOP_LONG_LEG`].
const HOP_LONG_AIR: f64 = 6.5;
/// Least distance (px) of a hop from a corner of the line it steps
/// over — of its line where the corner lies below it, of its crown
/// where the corner lies above: from there on that line's stroke runs
/// on its leg again (within a fraction of a px of it: the rounding
/// takes [`CORNER_R`], most of its turn in the last third), so the hop
/// sits over a straight stretch. Nearer the corner the crossing is one
/// with the rounding's curve and keeps the casing's gap.
pub(crate) const HOP_OVER_CORNER: f64 = CORNER_R - 2.0;
/// Room (px) a hop keeps from an arrowhead's base on its own leg.
const HOP_HEAD_AIR: f64 = 3.0;
/// Room (px) a hop keeps from a bare end of its own leg (the node's
/// border).
const HOP_END_AIR: f64 = 4.0;
/// Air (px) a hop's crown keeps under a level line running above it,
/// and its ink from a node, a label chip, a frame's border or its
/// title.
const HOP_ABOVE_AIR: f64 = 2.5;
/// Farthest (px) the first or last leg of a path may lie off its axis
/// for the rest of the path to take part in hops: an end the outline
/// snap moved along its shape leaves that one leg tilted a few px
/// (`orders --> pg[(Postgres)]`), and the stroke of the legs before it
/// runs on their lines all the same (a steeper leg pulls the spline off
/// the leg next to it).
const END_TILT_MAX: f64 = 6.0;
/// Float tolerance (px).
const EPS: f64 = 1e-6;

/// One hop of an edge's stroke: on its level leg at `y`, from `r`
/// before the line it crosses at `x0` over to `r` past the one at `x1`
/// (`x0 == x1`: a single semicircle), `r` high.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hop {
    pub y: f64,
    pub x0: f64,
    pub x1: f64,
    pub r: f64,
    /// How many lines it steps over.
    pub over: usize,
}

impl Hop {
    /// Where the stroke leaves its line and where it is back on it.
    pub(crate) fn span(&self) -> (f64, f64) {
        (self.x0 - self.r, self.x1 + self.r)
    }

    /// Whether the crossing at `p` lies under this hop.
    pub(crate) fn covers(&self, p: Pt) -> bool {
        let (lo, hi) = self.span();
        (p.1 - self.y).abs() <= 1.0 && p.0 >= lo - 1.0 && p.0 <= hi + 1.0
    }

    /// How far (px) `p` lies from the hop: from the box its arcs take,
    /// feet to crown.
    pub(crate) fn gap(&self, p: Pt) -> f64 {
        let (lo, hi) = self.span();
        let dx = (lo - p.0).max(p.0 - hi).max(0.0);
        let dy = (self.y - self.r - p.1).max(p.1 - self.y).max(0.0);
        dx.hypot(dy)
    }
}

/// One axis leg of an edge's path that takes part in hops.
#[derive(Clone, Copy)]
struct Leg {
    edge: usize,
    /// The line it runs on: `y` of a level leg, `x` of a plumb one.
    at: f64,
    /// Its whole span along the line.
    lo: f64,
    hi: f64,
    /// The part of it a hop may use: for a level leg where its own
    /// hops' feet may stand, for a plumb one where it may be hopped
    /// over (empty when `ok_lo > ok_hi`).
    ok_lo: f64,
    ok_hi: f64,
    /// Whether its low and its high end are ends of its edge's path —
    /// a head or a node's border, which no hop is drawn across — rather
    /// than corners.
    ends: [bool; 2],
    /// Its stroke's width.
    width: f64,
}

/// The legs of a scene's edges that take part in hops, level and plumb,
/// and what a hop must keep clear of.
pub(crate) struct Legs {
    level: Vec<Leg>,
    plumb: Vec<Leg>,
    edges: usize,
    /// Each list is sorted by its line (see [`legs`], [`legs_for_pick`]).
    sorted: bool,
    /// The widest stroke of a level leg.
    widest: f64,
    /// What no hop's ink may touch: the node boxes, the label chips,
    /// the frames' titles and — as boxes of no thickness — their
    /// borders.
    walls: Vec<Rect>,
    /// And the strokes that are no legs, as (edge, from, to) along each:
    /// a curve (an authored `curve:`, a self-loop, a state note's link)
    /// or a tilted end leg passing through where a hop would stand.
    strays: Vec<(usize, Pt, Pt)>,
}

/// Whether edge `e` may hop and be hopped over: one the flowchart
/// painter strokes with its default curve between two ends (an authored
/// `curve:` keeps its own shape, a self-loop its loop, and a class or
/// ER painter draws its own strokes).
pub(crate) fn hops_edge(e: &SceneEdge) -> bool {
    edge_drawn(e) && e.from != e.to && e.paint_width.is_none() && e.meta.curve.is_none()
}

/// The corners of `e`'s path where it runs on the axes, and whether the
/// first and the last of them are the path's own ends: a routed path's
/// ([`crate::router::ortho_corners`]) — less a first or last leg the
/// outline snap tilted off its axis ([`END_TILT_MAX`]), whose corner
/// then stands for no end — or the two ends of a straight stroke.
fn corners_of(e: &SceneEdge) -> Option<(Vec<Pt>, [bool; 2])> {
    let w = &e.waypoints;
    let n = w.len();
    if n >= 2 {
        // How far a leg lies off the nearer axis.
        let tilt = |k: usize| (w[k].0 - w[k + 1].0).abs().min((w[k].1 - w[k + 1].1).abs());
        let (t0, t1) = (tilt(0), tilt(n - 2));
        if t0 > END_TILT_MAX || t1 > END_TILT_MAX {
            return None;
        }
        let ends = [t0 < EPS, t1 < EPS];
        let (from, to) = (usize::from(!ends[0]), n - usize::from(!ends[1]));
        if to < from + 2 {
            return None;
        }
        return ortho_corners_of(&w[from..to]).map(|c| (c, ends));
    }
    let [a, c1, c2, b] = e.bezier;
    let on = |k: usize| {
        let v = |p: Pt| if k == 0 { p.0 } else { p.1 };
        let w = |p: Pt| if k == 0 { p.1 } else { p.0 };
        let (lo, hi) = (w(a).min(w(b)), w(a).max(w(b)));
        [c1, c2, b].iter().all(|&p| (v(p) - v(a)).abs() < EPS)
            && [c1, c2]
                .iter()
                .all(|&p| w(p) >= lo - EPS && w(p) <= hi + EPS)
            && hi - lo > EPS
    };
    (on(0) || on(1)).then(|| (vec![a, b], [true, true]))
}

/// The legs of `sc` that take part in hops ([`Legs`]); none in a
/// drawing past the casing's cap
/// ([`crate::casing::CASING_MAX_EDGES`]), which draws its crossings
/// plain.
pub(crate) fn legs(sc: &Scene) -> Legs {
    let mut out = legs_for_pick(sc);
    let by_line = |a: &Leg, b: &Leg| {
        a.at.total_cmp(&b.at)
            .then(a.lo.total_cmp(&b.lo))
            .then(a.edge.cmp(&b.edge))
    };
    out.level.sort_by(by_line);
    out.plumb.sort_by(by_line);
    out.sorted = true;
    out
}

/// [`legs`] left in edge order: what one pick needs of them — the hops
/// of an edge or two ([`Legs::of`], which then looks through every leg)
/// — costs less than sorting them all. The hops come out the same
/// either way ([`Legs::leg_hops`]).
pub(crate) fn legs_for_pick(sc: &Scene) -> Legs {
    let mut out = Legs {
        level: Vec::new(),
        plumb: Vec::new(),
        edges: sc.edges.len(),
        sorted: false,
        widest: 0.0,
        walls: Vec::new(),
        strays: Vec::new(),
    };
    if sc.edges.len() > crate::casing::CASING_MAX_EDGES {
        return out;
    }
    // The drawn edges that give no legs, and the tilted end legs of
    // those that do: strays, sampled once the scene is known to hop.
    let mut curves: Vec<usize> = Vec::new();
    for (i, e) in sc.edges.iter().enumerate() {
        if !edge_drawn(e) {
            continue;
        }
        let Some((c, path_ends)) = Some(e).filter(|e| hops_edge(e)).and_then(corners_of) else {
            curves.push(i);
            continue;
        };
        let w = &e.waypoints;
        if !path_ends[0] {
            out.strays.push((i, w[0], w[1]));
        }
        if !path_ends[1] {
            out.strays.push((i, w[w.len() - 2], w[w.len() - 1]));
        }
        let width = edge_stroke_width(e);
        let glyph = [
            e.ends.start_marker(e.kind).is_some(),
            e.ends.end_marker(e.kind).is_some(),
        ];
        let head = edge_head_len(e);
        let n = c.len();
        for k in 0..n - 1 {
            let (p, q) = (c[k], c[k + 1]);
            let level = (p.1 - q.1).abs() < EPS && (p.0 - q.0).abs() > EPS;
            let plumb = (p.0 - q.0).abs() < EPS && (p.1 - q.1).abs() > EPS;
            if !level && !plumb {
                continue;
            }
            // Whether an end of the leg is an end of the path.
            let is_end = |at_start: bool| {
                if at_start {
                    k == 0 && path_ends[0]
                } else {
                    k + 1 == n - 1 && path_ends[1]
                }
            };
            // What each end of the leg keeps clear: for its own hops'
            // feet, and for the middle of a hop over it.
            let long = ((p.0 - q.0).abs() + (p.1 - q.1).abs()) / HOP_LONG_LEG + HOP_LONG_AIR;
            let clear = |at_start: bool| -> (f64, f64) {
                let g = glyph[usize::from(!at_start)];
                match (is_end(at_start), g) {
                    (false, _) => ((CORNER_R + HOP_CORNER_AIR).max(long), HOP_OVER_CORNER),
                    (true, true) => (head + HOP_HEAD_AIR, head + HOP_R + 1.0),
                    (true, false) => (HOP_END_AIR, HOP_R + 2.0),
                }
            };
            let v = |r: Pt| if level { r.0 } else { r.1 };
            let (a, b) = (v(p), v(q));
            let (ca, cb) = (clear(true), clear(false));
            // (Low end first.)
            let (lo, hi, c_lo, c_hi, ends) = if a <= b {
                (a, b, ca, cb, [is_end(true), is_end(false)])
            } else {
                (b, a, cb, ca, [is_end(false), is_end(true)])
            };
            let own = usize::from(!level);
            let pick = |c: (f64, f64)| if own == 0 { c.0 } else { c.1 };
            let leg = Leg {
                edge: i,
                at: if level { p.1 } else { p.0 },
                lo,
                hi,
                ok_lo: lo + pick(c_lo),
                ok_hi: hi - pick(c_hi),
                ends,
                width,
            };
            if level {
                out.widest = out.widest.max(width);
                out.level.push(leg);
            } else {
                out.plumb.push(leg);
            }
        }
    }
    if out.is_empty() {
        return out;
    }
    for i in curves {
        let line = crate::scene::edge_polyline(&sc.edges[i]);
        out.strays.extend(line.windows(2).map(|q| (i, q[0], q[1])));
    }
    for n in &sc.nodes {
        let (hw, hh) = (n.w / 2.0, n.h / 2.0);
        out.walls.push((n.x - hw, n.y - hh, n.x + hw, n.y + hh));
    }
    for e in sc.edges.iter().filter(|e| edge_drawn(e)) {
        if let Some((_, at, w)) = &e.label {
            out.walls.push(crate::comb::label_box(e, *at, *w));
        }
    }
    for (ci, c) in sc.clusters.iter().enumerate() {
        let (x0, y0, x1, y1) = (c.x, c.y, c.x + c.w, c.y + c.h);
        out.walls.extend([
            (x0, y0, x1, y0),
            (x0, y1, x1, y1),
            (x0, y0, x0, y1),
            (x1, y0, x1, y1),
            crate::scene::title_head(&sc.clusters, ci, f64::INFINITY),
        ]);
    }
    out
}

/// The order an edge's hops are listed in: by line, then place (two
/// level legs of one edge on one line never overlap).
fn by_place(a: &Hop, b: &Hop) -> std::cmp::Ordering {
    a.y.total_cmp(&b.y).then(a.x0.total_cmp(&b.x0))
}

impl Legs {
    /// Whether no edge of the scene hops.
    pub(crate) fn is_empty(&self) -> bool {
        self.level.is_empty() || self.plumb.is_empty()
    }

    /// The hops of every edge of the scene (index-parallel with its
    /// edges), each edge's sorted by line and place.
    pub(crate) fn all(&self) -> Vec<Vec<Hop>> {
        let mut out = vec![Vec::new(); self.edges];
        if self.is_empty() {
            return out;
        }
        for leg in &self.level {
            out[leg.edge].extend(self.leg_hops(leg));
        }
        if !self.sorted {
            out.iter_mut().for_each(|h| h.sort_by(by_place));
        }
        out
    }

    /// The hops of edge `i` alone, as [`Legs::all`] lists them.
    pub(crate) fn of(&self, i: usize) -> Vec<Hop> {
        if self.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<Hop> = self
            .level
            .iter()
            .filter(|l| l.edge == i)
            .flat_map(|l| self.leg_hops(l))
            .collect();
        if !self.sorted {
            out.sort_by(by_place);
        }
        out
    }

    /// The hops of one level leg (see the module docs). The crossings
    /// are read in one order — by place, then radius — whichever order
    /// the legs are listed in, so the painter's sorted legs and a pick's
    /// unsorted ones give the same hops.
    fn leg_hops(&self, leg: &Leg) -> Vec<Hop> {
        let across = if self.sorted {
            let from = self.plumb.partition_point(|v| v.at <= leg.lo + EPS);
            let to = self.plumb.partition_point(|v| v.at < leg.hi - EPS);
            &self.plumb[from..to.max(from)]
        } else {
            &self.plumb[..]
        };
        // The plumb legs crossing it: where a hop may stand, as (x, its
        // radius there); where none may be drawn at all — over a head,
        // a bare end or a stroke too wide for one; and the crossings in
        // a corner's rounding, which take no hop of their own and bar
        // none over them (two edges down one trunk, one of them turning
        // off, are stepped over as the one line they are there) but
        // leave no room for a hop's foot right beside them.
        let mut xs: Vec<(f64, f64)> = Vec::new();
        let mut barred: Vec<f64> = Vec::new();
        let mut soft: Vec<f64> = Vec::new();
        for v in across {
            if v.at <= leg.lo + EPS
                || v.at >= leg.hi - EPS
                || v.edge == leg.edge
                || leg.at <= v.lo + EPS
                || leg.at >= v.hi - EPS
            {
                continue;
            }
            let r = HOP_R.max((leg.width + v.width) / 2.0 + HOP_AIR);
            // (The hop stands on the low side of its line: past a
            // corner there its crown, not its line, keeps the room.)
            let crown = if v.ends[0] { 0.0 } else { r };
            let (before, past) = (leg.at < v.ok_lo + crown - EPS, leg.at > v.ok_hi + EPS);
            if r > HOP_R_MAX || (before && v.ends[0]) || (past && v.ends[1]) {
                barred.push(v.at);
            } else if before || past {
                soft.push(v.at);
            } else {
                xs.push((v.at, r));
            }
        }
        xs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let bars = |lo: f64, hi: f64| barred.iter().any(|&x| x >= lo - EPS && x <= hi + EPS);
        // (A crossing right beside one that bars a hop takes none
        // either, and like any that takes none leaves no room for a
        // foot beside it.)
        xs.retain(|&(x, r)| {
            let free = !bars(x - r, x + r);
            if !free {
                soft.push(x);
            }
            free
        });
        // Whether a crossing that takes no hop lies under a foot of the
        // hop from `x0` to `x1`, `r` high, or within a join of it: the
        // hop would spring from that line.
        let cramped = |x0: f64, x1: f64, r: f64| {
            let (lo, hi) = (x0 - r - HOP_JOIN, x1 + r + HOP_JOIN);
            bars(lo, hi)
                || soft
                    .iter()
                    .any(|&x| (x >= lo && x < x0 - EPS) || (x > x1 + EPS && x <= hi))
        };
        // Neighbouring hops join into bridges (none across a crossing
        // that bars one); a bridge whose feet find no straight stretch
        // gives up its outermost crossings until they do. A crossing
        // given up takes no hop, so the foot keeps as clear of it as of
        // any other that takes none: the next one within a foot and a
        // join of it is given up as well.
        let mut out = Vec::new();
        let mut k = 0;
        while k < xs.len() {
            let (mut end, mut r) = (k, xs[k].1);
            while let Some(&(x, rn)) = xs.get(end + 1) {
                if (x - rn) - (xs[end].0 + r.max(rn)) >= HOP_JOIN || bars(xs[end].0, x) {
                    break;
                }
                (end, r) = (end + 1, r.max(rn));
            }
            let reach = r + HOP_JOIN;
            let low = (k..=end).find(|&a| {
                xs[a].0 - r >= leg.ok_lo - EPS && (a == k || xs[a].0 - xs[a - 1].0 > reach)
            });
            let high = (k..=end).rev().find(|&b| {
                xs[b].0 + r <= leg.ok_hi + EPS && (b == end || xs[b + 1].0 - xs[b].0 > reach)
            });
            if let (Some(a), Some(b)) = (low, high) {
                if a <= b {
                    let hop = Hop {
                        y: leg.at,
                        x0: xs[a].0,
                        x1: xs[b].0,
                        r,
                        over: b - a + 1,
                    };
                    if !cramped(hop.x0, hop.x1, r) && self.clear(leg, &hop) {
                        out.push(hop);
                    }
                }
            }
            k = end + 1;
        }
        out
    }

    /// Whether `hop`'s ink runs into nothing: no other level leg right
    /// above, where its crown would meet it, no wall and no stray
    /// stroke ([`Legs`]) — a hop half under a label chip, with its
    /// crown on a frame's border or with a curve through its arc reads
    /// as neither a hop nor a line.
    fn clear(&self, leg: &Leg, hop: &Hop) -> bool {
        let (lo, hi) = hop.span();
        let top = hop.y - hop.r - HOP_ABOVE_AIR;
        // (The legs that can reach it: no nearer than its own line,
        // no further than the widest stroke carries.)
        let above = if self.sorted {
            let reach = top - (self.widest + leg.width) / 2.0;
            let from = self.level.partition_point(|l| l.at < reach);
            let to = self.level.partition_point(|l| l.at < hop.y - EPS);
            &self.level[from..to.max(from)]
        } else {
            &self.level[..]
        };
        let free = above.iter().all(|l| {
            let reach = top - (l.width + leg.width) / 2.0;
            l.at >= hop.y - EPS || l.at < reach || l.hi <= lo || l.lo >= hi
        });
        let (x0, x1) = (lo - HOP_ABOVE_AIR, hi + HOP_ABOVE_AIR);
        let y0 = top - leg.width / 2.0;
        free && !self
            .walls
            .iter()
            .any(|w| w.0 <= x1 && w.2 >= x0 && w.1 <= hop.y && w.3 >= y0)
            && !self
                .strays
                .iter()
                .any(|&(e, a, b)| e != leg.edge && crate::scene::seg_aabb(a, b, x0, y0, x1, hop.y))
    }
}

/// The hops of every edge of `sc` ([`Legs::all`]).
pub(crate) fn find(sc: &Scene) -> Vec<Vec<Hop>> {
    legs(sc).all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::scene::scene;

    /// A scene of straight strokes: `(from, to)` per edge.
    fn strokes(lines: &[(Pt, Pt)]) -> Scene {
        let src: String = std::iter::once("flowchart TD".to_string())
            .chain((0..lines.len()).map(|k| format!("a{k} --- b{k}")))
            .collect::<Vec<_>>()
            .join("\n");
        let mut sc = scene(&parse(&src).unwrap());
        for (e, &(a, b)) in sc.edges.iter_mut().zip(lines) {
            let at = |t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            e.bezier = [a, at(1.0 / 3.0), at(2.0 / 3.0), b];
            e.waypoints.clear();
        }
        // (Far from the strokes: the nodes take no part.)
        for n in &mut sc.nodes {
            (n.x, n.y) = (-1000.0, -1000.0);
        }
        sc
    }

    /// A scene of routed paths: each edge drawn along its corners, as
    /// the stages draw an orthogonal path ([`crate::router::smooth`]).
    fn paths(corners: &[&[Pt]]) -> Scene {
        let lines: Vec<(Pt, Pt)> = corners.iter().map(|c| (c[0], c[c.len() - 1])).collect();
        let mut sc = strokes(&lines);
        for (e, c) in sc.edges.iter_mut().zip(corners) {
            if c.len() > 2 {
                e.waypoints = crate::router::smooth(c, CORNER_R);
            }
        }
        sc
    }

    /// The hops a pick works out ([`legs_for_pick`]) are the painter's
    /// ([`legs`]), edge by edge.
    fn assert_pick_is_paint(sc: &Scene, what: &str) -> usize {
        let painted = find(sc);
        let pick = legs_for_pick(sc);
        for (i, hops) in painted.iter().enumerate() {
            assert_eq!(&pick.of(i), hops, "{what}: edge {i}");
        }
        painted.iter().map(Vec::len).sum()
    }

    #[test]
    fn the_level_stroke_hops_over_the_plumb_one() {
        let sc = strokes(&[((0.0, 50.0), (100.0, 50.0)), ((40.0, 0.0), (40.0, 100.0))]);
        let hops = find(&sc);
        assert_eq!(
            hops[0],
            vec![Hop {
                y: 50.0,
                x0: 40.0,
                x1: 40.0,
                r: HOP_R,
                over: 1
            }]
        );
        assert!(hops[1].is_empty(), "the plumb stroke runs straight on");
        assert_eq!(hops[0][0].span(), (35.0, 45.0));
        assert!(hops[0][0].covers((40.0, 50.0)) && !hops[0][0].covers((40.0, 60.0)));
    }

    #[test]
    fn hops_over_a_bundle_join_into_one_bridge() {
        // Three tracks 12.5 px apart: one bridge; a fourth line 40 px on
        // gets a hop of its own.
        let mut lines = vec![((0.0, 50.0), (200.0, 50.0))];
        for x in [40.0, 52.5, 65.0, 105.0] {
            lines.push(((x, 0.0), (x, 100.0)));
        }
        let hops = find(&strokes(&lines));
        assert_eq!(hops[0].len(), 2, "{:?}", hops[0]);
        assert_eq!((hops[0][0].x0, hops[0][0].x1), (40.0, 65.0));
        assert_eq!((hops[0][0].over, hops[0][1].over), (3, 1));
        assert_eq!((hops[0][1].x0, hops[0][1].x1), (105.0, 105.0));
        assert_eq!(hops[0][0].span(), (35.0, 70.0));
    }

    #[test]
    fn a_crossing_next_to_an_end_or_under_a_line_takes_no_hop() {
        // Too near the level stroke's own end for the hop's foot.
        let near_end = strokes(&[((0.0, 50.0), (100.0, 50.0)), ((6.0, 0.0), (6.0, 100.0))]);
        assert!(find(&near_end)[0].is_empty());
        // Too near the plumb stroke's end: the hop would sit on it.
        let on_end = strokes(&[((0.0, 50.0), (100.0, 50.0)), ((40.0, 46.0), (40.0, 100.0))]);
        assert!(find(&on_end)[0].is_empty());
        // Another level line 6.5 px above: no room for the crown; the
        // upper line itself hops.
        let under = strokes(&[
            ((0.0, 50.0), (100.0, 50.0)),
            ((0.0, 43.5), (100.0, 43.5)),
            ((40.0, 0.0), (40.0, 100.0)),
        ]);
        let hops = find(&under);
        assert!(hops[0].is_empty(), "{:?}", hops[0]);
        assert_eq!(hops[1].len(), 1);
    }

    #[test]
    fn wider_strokes_take_a_wider_hop_or_none() {
        let mut sc = strokes(&[((0.0, 50.0), (100.0, 50.0)), ((40.0, 0.0), (40.0, 100.0))]);
        sc.edges[1].meta.style.stroke_width = Some(6.0);
        let hops = find(&sc);
        assert_eq!(hops[0].len(), 1);
        assert!((hops[0][0].r - 7.0).abs() < 1e-9, "{:?}", hops[0]);
        sc.edges[1].meta.style.stroke_width = Some(24.0);
        assert!(find(&sc)[0].is_empty(), "past the widest hop: a gap");
    }

    #[test]
    fn a_curve_or_a_big_drawing_takes_no_hops() {
        let mut sc = strokes(&[((0.0, 50.0), (100.0, 50.0)), ((40.0, 0.0), (40.0, 100.0))]);
        // A slanted stroke is no plumb leg.
        sc.edges[1].bezier = [(40.0, 0.0), (42.0, 33.0), (44.0, 66.0), (46.0, 100.0)];
        assert!(find(&sc).iter().all(Vec::is_empty));
        // Past the casing's cap crossings are drawn plain.
        let n = crate::casing::CASING_MAX_EDGES + 1;
        let lines: Vec<(Pt, Pt)> = (0..n)
            .map(|k| {
                if k == 0 {
                    ((0.0, 50.0), (100.0, 50.0))
                } else {
                    ((40.0, 0.0), (40.0, 100.0))
                }
            })
            .collect();
        assert!(find(&strokes(&lines)).iter().all(Vec::is_empty));
    }
    /// B11 review r1 #2: two plumb strokes on one line, one of which
    /// bars a hop (its end is right there), read the same whichever
    /// order the edges come in — the painter sorts the legs, a pick does
    /// not — and the barred crossing takes no hop, nor does a bridge
    /// reach across it or a hop stand right beside it.
    #[test]
    fn hops_do_not_depend_on_the_order_of_the_edges() {
        let lines = |far: f64| {
            [
                ((100.0, 46.0), (100.0, 100.0)),
                ((0.0, 50.0), (200.0, 50.0)),
                ((far, 0.0), (far, 100.0)),
                ((100.0, 0.0), (100.0, 100.0)),
            ]
        };
        let orders: [[usize; 4]; 6] = [
            [0, 1, 2, 3],
            [3, 1, 2, 0],
            [1, 0, 3, 2],
            [2, 3, 0, 1],
            [3, 2, 1, 0],
            [1, 2, 3, 0],
        ];
        for order in orders {
            let level = order.iter().position(|&k| k == 1).unwrap();
            // Over the stroke at 78 alone: the one at 100 ends 4 px
            // above the line, and a hop there would sit on its end.
            let sc = strokes(&order.map(|k| lines(78.0)[k]));
            assert_eq!(assert_pick_is_paint(&sc, "tie"), 1, "{order:?}");
            assert_eq!(
                find(&sc)[level],
                vec![Hop {
                    y: 50.0,
                    x0: 78.0,
                    x1: 78.0,
                    r: HOP_R,
                    over: 1
                }],
                "{order:?}"
            );
            // A stroke 10 px from the barred crossing takes none
            // either: its hop would come down right beside it.
            let sc = strokes(&order.map(|k| lines(90.0)[k]));
            assert_eq!(assert_pick_is_paint(&sc, "cramped"), 0, "{order:?}");
        }
        // With the short stroke clear of the line the two at 90 and 100
        // join into one bridge, again in any order.
        let mut clear = lines(90.0);
        clear[0] = ((100.0, 60.0), (100.0, 100.0));
        for order in orders {
            let sc = strokes(&order.map(|k| clear[k]));
            assert_eq!(assert_pick_is_paint(&sc, "bridge"), 1, "{order:?}");
            let level = order.iter().position(|&k| k == 1).unwrap();
            assert_eq!(find(&sc)[level][0].span(), (85.0, 105.0), "{order:?}");
        }
    }

    /// The painter's hops are a pick's on every fixture, as laid out, as
    /// `route()` draws it and under a drag (`route_partial`): one rule,
    /// read in one order (B11 review r1 #1, #2).
    #[test]
    fn a_pick_finds_the_painted_hops_on_every_fixture() {
        use crate::model::Document;
        let root = env!("CARGO_MANIFEST_DIR");
        let mut files: Vec<std::path::PathBuf> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        let (mut scenes, mut hops) = (0, 0);
        for f in &files {
            let src = std::fs::read_to_string(f).unwrap();
            let g = match crate::parser::parse_document(&src) {
                Ok(Document::Flowchart(g) | Document::State(g)) => g,
                _ => continue,
            };
            let st = scene(&g);
            let base: Vec<Pt> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            hops += assert_pick_is_paint(&st, &name);
            hops += assert_pick_is_paint(&crate::scene::route(&g, &base), &name);
            scenes += 2;
            // The busiest node dragged: a nudge, and far across the
            // drawing (where edges come to lie on one another).
            let mut deg = vec![0usize; g.nodes.len()];
            for e in &g.edges {
                deg[e.from] += 1;
                deg[e.to] += 1;
            }
            let Some(v) = (0..deg.len()).max_by_key(|&v| (deg[v], std::cmp::Reverse(v))) else {
                continue;
            };
            for d in [(37.0, 23.0), (180.0, 140.0)] {
                let mut c = base.clone();
                c[v] = (c[v].0 + d.0, c[v].1 + d.1);
                let part = crate::scene::route_partial(&g, &c, &st, &base);
                hops += assert_pick_is_paint(&part, &name);
                scenes += 1;
            }
        }
        assert!(scenes >= 40 && hops > 500, "{scenes} scenes, {hops} hops");
    }

    /// A line is stepped over from [`HOP_OVER_CORNER`] past its corner
    /// on, where its stroke is back on its leg; in the rounding itself
    /// the crossing keeps the casing's gap. Two edges down one trunk,
    /// one turning off right there, are stepped over as one line.
    #[test]
    fn a_line_is_stepped_over_just_past_its_corner() {
        // A path down x = 100 that turns right at y = 100.
        let turn: &[Pt] = &[(100.0, 0.0), (100.0, 100.0), (300.0, 100.0)];
        for (y, hop) in [
            (100.0 - HOP_OVER_CORNER, true),
            (100.0 - HOP_OVER_CORNER + 2.0, false),
            (40.0, true),
        ] {
            let level: &[Pt] = &[(0.0, y), (200.0, y)];
            let sc = paths(&[level, turn]);
            assert_eq!(find(&sc)[0].len(), usize::from(hop), "level at {y}");
            assert_pick_is_paint(&sc, "corner");
            // Mirrored, the corner above the line: the hop's crown
            // keeps that room, the line a radius more.
            let up: &[Pt] = &[(100.0, 200.0), (100.0, 100.0), (300.0, 100.0)];
            let (below, hop) = (200.0 - y, hop && y < 100.0 - HOP_OVER_CORNER - HOP_R + EPS);
            let level: &[Pt] = &[(0.0, below), (200.0, below)];
            let sc = paths(&[level, up]);
            assert_eq!(find(&sc)[0].len(), usize::from(hop), "level at {below}");
            let level: &[Pt] = &[(0.0, y), (200.0, y)];
            // A second edge straight down the same trunk is stepped
            // over wherever the line crosses it; the one turning off
            // bars nothing.
            let trunk: &[Pt] = &[(100.0, 0.0), (100.0, 200.0)];
            for sc in [paths(&[level, turn, trunk]), paths(&[trunk, level, turn])] {
                let hops: Vec<Hop> = find(&sc).into_iter().flatten().collect();
                assert_eq!(hops.len(), 1, "level at {y}: {hops:?}");
                assert_eq!((hops[0].x0, hops[0].x1), (100.0, 100.0));
                assert_pick_is_paint(&sc, "trunk");
            }
        }
    }

    /// A hop's foot keeps a join clear of a crossing that takes no hop:
    /// a line crossed in its corner's rounding 7 px before one that is
    /// stepped over would have the hop spring from it (the reviewer's
    /// bipartite input, `a3 ==> b3` at `a2 -.-> b4`).
    #[test]
    fn a_hop_does_not_spring_from_a_line_it_cannot_step_over() {
        let level: &[Pt] = &[(0.0, 50.0), (300.0, 50.0)];
        let trunk: &[Pt] = &[(100.0, 0.0), (100.0, 200.0)];
        // Turning into its plumb leg 10 px above the level line.
        let turning = |x: f64| [(x - 60.0, 40.0), (x, 40.0), (x, 200.0)];
        for (x, hops) in [(92.5, 0), (84.0, 1), (100.0, 1)] {
            let turn = turning(x);
            let sc = paths(&[level, trunk, &turn]);
            assert_eq!(find(&sc)[0].len(), hops, "turning at {x}");
            assert_pick_is_paint(&sc, "cramped");
        }
    }

    /// B11 review r2 #2: a bridge that gives up its outermost crossing —
    /// the foot would stand in the level leg's own corner room — keeps
    /// its foot a join clear of that crossing like of any other that
    /// takes no hop. Before, the hop over the next line was drawn with
    /// its foot on the one given up, which then got neither hop nor gap
    /// and read as joined to the level line (fan085 of the hub-fan set;
    /// the reviewer's bipartite input in `route()`, `a1 --> b4` at
    /// `a4 --> b1`).
    #[test]
    fn a_foot_keeps_clear_of_a_crossing_its_bridge_gave_up() {
        // A path turning into its level leg at x = 100: a foot stands
        // no nearer the corner than the rounding and its air.
        let level: &[Pt] = &[(100.0, 0.0), (100.0, 50.0), (400.0, 50.0)];
        let room = 100.0 + CORNER_R + HOP_CORNER_AIR;
        // Too near the corner for a hop of its own.
        let near = room + HOP_R - 3.0;
        let plumb = |x: f64| [(x, 0.0), (x, 100.0)];
        for (gap, hops) in [
            // Right beside it (the reviewer's 4.6 px): the foot of a hop
            // here would stand on the line given up.
            (4.6, 0),
            (HOP_R + HOP_JOIN, 0),
            // A foot and a join on: a hop of its own, clear of it.
            (HOP_R + HOP_JOIN + 1.0, 1),
        ] {
            let (first, second) = (plumb(near), plumb(near + gap));
            let sc = paths(&[level, &first, &second]);
            let found = find(&sc);
            assert_eq!(found[0].len(), hops, "{gap} px on: {:?}", found[0]);
            if let Some(h) = found[0].first() {
                assert_eq!((h.x0, h.x1, h.over), (near + gap, near + gap, 1));
                assert!(h.span().0 - near > HOP_JOIN, "{:?}", h.span());
            }
            assert_pick_is_paint(&sc, "given up");
        }
        // Three lines in one bridge, the first given up: the hop spans
        // the other two only where its foot is clear of the first.
        for (gap, over) in [(7.0, 0), (11.0, 2)] {
            let (a, b, c) = (plumb(near), plumb(near + gap), plumb(near + gap + 9.0));
            let sc = paths(&[level, &a, &b, &c]);
            let found = find(&sc);
            let n: usize = found[0].iter().map(|h| h.over).sum();
            assert_eq!(n, over, "{gap} px on: {:?}", found[0]);
            assert_pick_is_paint(&sc, "bridge");
        }
        // The same at the far end of the leg, mirrored.
        let level: &[Pt] = &[(0.0, 50.0), (300.0, 50.0), (300.0, 0.0)];
        let near = 300.0 - CORNER_R - HOP_CORNER_AIR - HOP_R + 3.0;
        for (gap, hops) in [(4.6, 0), (HOP_R + HOP_JOIN + 1.0, 1)] {
            let (first, second) = (plumb(near), plumb(near - gap));
            let sc = paths(&[level, &first, &second]);
            assert_eq!(find(&sc)[0].len(), hops, "{gap} px before");
            assert_pick_is_paint(&sc, "given up, far end");
        }
    }

    /// B11 review r2 #2, the same beside a line that bars a hop: a
    /// crossing a hop's width from a plumb stroke's bare end takes no
    /// hop (the hop would sit on the end), and the next line along is
    /// stepped over only where the foot stands a join clear of it.
    #[test]
    fn a_foot_keeps_clear_of_a_crossing_beside_a_barred_one() {
        let level = ((0.0, 50.0), (300.0, 50.0));
        // A plumb stroke that ends 4 px above the level one bars a hop
        // at x = 100; the line 4 px on has no room for one either.
        let end = ((100.0, 46.0), (100.0, 100.0));
        let beside = ((104.0, 0.0), (104.0, 100.0));
        for (on, hops) in [(8.0, 0), (HOP_R + HOP_JOIN, 0), (HOP_R + HOP_JOIN + 1.0, 1)] {
            let third = ((104.0 + on, 0.0), (104.0 + on, 100.0));
            let sc = strokes(&[level, end, beside, third]);
            let found = find(&sc);
            assert_eq!(found[0].len(), hops, "{on} px on: {:?}", found[0]);
            assert!(found[0].iter().all(|h| h.x0 == 104.0 + on && h.over == 1));
            assert_pick_is_paint(&sc, "beside a barred crossing");
        }
    }

    /// B11 review r2 #5: on a long leg the stroke leaves its line well
    /// before the corner's rounding, and a foot keeps that much further
    /// from the corner ([`HOP_LONG_LEG`]) — on dense-td's `n16 -> n23`
    /// a foot 24 px before the corner of a 1 792 px leg stood 0.5 px off
    /// the line, and its arc came out 5.5 px high.
    #[test]
    fn a_foot_keeps_further_from_the_corner_of_a_long_leg() {
        let plumb = |x: f64| [(x, 0.0), (x, 100.0)];
        for (len, room) in [
            (300.0, CORNER_R + HOP_CORNER_AIR),
            (1800.0, 1800.0 / HOP_LONG_LEG + HOP_LONG_AIR),
        ] {
            // A level leg `len` long that turns down at its far end.
            let level: &[Pt] = &[(0.0, 50.0), (len, 50.0), (len, 300.0)];
            for (before, hop) in [(room + HOP_R - 1.0, false), (room + HOP_R + 1.0, true)] {
                let line = plumb(len - before);
                let sc = paths(&[level, &line]);
                assert_eq!(
                    find(&sc)[0].len(),
                    usize::from(hop),
                    "a {len} px leg, {before} px before its corner"
                );
                assert_pick_is_paint(&sc, "long leg");
            }
        }
    }

    /// B11 review r1 #3: an end the outline snap moved leaves its leg
    /// tilted a few px; the rest of the path hops and is hopped over all
    /// the same (before, such an edge took part in no hop and every
    /// crossing with it was a gap). A leg tilted further takes its whole
    /// path out, as an edge off the axes always was.
    #[test]
    fn a_path_with_a_tilted_end_leg_still_takes_part() {
        let level: &[Pt] = &[(0.0, 50.0), (200.0, 50.0)];
        let turn: &[Pt] = &[(100.0, 0.0), (100.0, 100.0), (300.0, 100.0)];
        for (tilt, hops) in [(0.0, 1), (1.5, 1), (END_TILT_MAX + 2.0, 0)] {
            let mut sc = paths(&[level, turn]);
            sc.edges[1].waypoints.last_mut().unwrap().1 += tilt;
            assert_eq!(find(&sc)[0].len(), hops, "tilt {tilt}");
            assert_pick_is_paint(&sc, "tilt");
        }
        // The tilted leg's own crossings take no hop: it is no leg on
        // the axes, and neither is the short stretch before it.
        let down: &[Pt] = &[(200.0, 60.0), (200.0, 200.0)];
        let mut sc = paths(&[level, turn, down]);
        sc.edges[1].waypoints.last_mut().unwrap().1 += 1.5;
        let hops = find(&sc);
        assert_eq!((hops[0].len(), hops[1].len()), (1, 0), "{hops:?}");
    }

    /// A stroke that is no leg — a curve, a slanted line — passing
    /// through where a hop would stand bars it (two generated state
    /// diagrams: a note's link ran through the hop of the edge beside
    /// it).
    #[test]
    fn a_hop_keeps_clear_of_a_curve_passing_by() {
        let level = ((0.0, 50.0), (200.0, 50.0));
        let plumb = ((100.0, 0.0), (100.0, 100.0));
        // A slanted stroke through the hop's arc, and one well beside it.
        for (x, hop) in [(96.0, false), (60.0, true)] {
            let slanted = ((x - 20.0, 0.0), (x + 20.0, 100.0));
            let sc = strokes(&[level, plumb, slanted]);
            assert_eq!(find(&sc)[0].len(), usize::from(hop), "slanted at {x}");
            assert_pick_is_paint(&sc, "stray");
        }
    }

    /// B11 review r1 #5: a hop is drawn only where its ink runs into
    /// nothing — no label chip (its own included), no frame border or
    /// title, no node; such a crossing keeps the casing's gap.
    #[test]
    fn a_hop_keeps_clear_of_chips_borders_and_nodes() {
        let lines = [((0.0, 50.0), (200.0, 50.0)), ((100.0, 0.0), (100.0, 100.0))];
        let clear = strokes(&lines);
        assert_eq!(find(&clear)[0].len(), 1);
        // A label chip over the crossing, or reaching its foot.
        for (x, hop) in [(100.0, false), (125.0, false), (140.0, true)] {
            let mut sc = strokes(&lines);
            sc.edges[0].label = Some(("audit".to_string(), (x, 50.0), 40.0));
            assert_eq!(find(&sc)[0].len(), usize::from(hop), "chip at {x}");
            assert_pick_is_paint(&sc, "chip");
        }
        // A node whose border the crown would touch.
        for (y, hop) in [(30.0, false), (20.0, true)] {
            let mut sc = strokes(&lines);
            let n = &mut sc.nodes[0];
            (n.x, n.y, n.w, n.h) = (100.0, y, 60.0, 30.0);
            assert_eq!(find(&sc)[0].len(), usize::from(hop), "node at {y}");
        }
        // A frame: its border just above the line (the crown would lie
        // on it), or crossing the line under the hop.
        let framed = |x: f64, y: f64| {
            let src = "flowchart TD\nsubgraph F\na0 --- b0\nend\na1 --- b1";
            let mut sc = scene(&parse(src).unwrap());
            for (e, &(a, b)) in sc.edges.iter_mut().zip(&lines) {
                let at = |t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                e.bezier = [a, at(1.0 / 3.0), at(2.0 / 3.0), b];
                e.waypoints.clear();
            }
            for n in &mut sc.nodes {
                (n.x, n.y) = (-1000.0, -1000.0);
            }
            let c = &mut sc.clusters[0];
            (c.x, c.y, c.w, c.h) = (x, y, 400.0, 300.0);
            sc
        };
        // (The frame's bottom border at y - 300 + 300.)
        for (x, y, hop) in [
            (-100.0, -255.5, false), // bottom border 5.5 px above the line
            (-100.0, -270.0, true),  // 20 px above: clear
            (97.0, 20.0, false),     // left border under the hop
            (120.0, 60.0, true),     // the frame beside and below
        ] {
            let sc = framed(x, y);
            assert_eq!(find(&sc)[0].len(), usize::from(hop), "frame at {x}, {y}");
            assert_pick_is_paint(&sc, "frame");
        }
    }
}
