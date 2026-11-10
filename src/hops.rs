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
//! through it: clear of each leg's rounded corners, of the arrowheads
//! and of the ends, with no other level line right above for the hop
//! to run into. Every other crossing — a curve's, one in a corner's
//! rounding or under a head — keeps the casing's gap.
//!
//! A hop is paint: [`crate::scene::to_svg`] strokes it and picking
//! ([`crate::scene::Scene::edge_at`]) covers it, but the scene's
//! geometry stays the plain path, which is what a host draws from.

use crate::router::{ortho_corners, CORNER_R};
use crate::scene::{edge_drawn, edge_head_len, edge_stroke_width, Scene, SceneEdge};

type Pt = (f64, f64);

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
/// Room (px) a hop keeps from an arrowhead's base on its own leg.
const HOP_HEAD_AIR: f64 = 3.0;
/// Room (px) a hop keeps from a bare end of its own leg (the node's
/// border).
const HOP_END_AIR: f64 = 4.0;
/// Air (px) a hop's crown keeps under a level line running above it.
const HOP_ABOVE_AIR: f64 = 2.5;
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
    /// Its stroke's width.
    width: f64,
}

/// The legs of a scene's edges that take part in hops, level and plumb,
/// each sorted by its line.
pub(crate) struct Legs {
    level: Vec<Leg>,
    plumb: Vec<Leg>,
    edges: usize,
    /// Each list is sorted by its line (see [`legs`], [`legs_for_pick`]).
    sorted: bool,
}

/// Whether edge `e` may hop and be hopped over: one the flowchart
/// painter strokes with its default curve between two ends (an authored
/// `curve:` keeps its own shape, a self-loop its loop, and a class or
/// ER painter draws its own strokes).
pub(crate) fn hops_edge(e: &SceneEdge) -> bool {
    edge_drawn(e) && e.from != e.to && e.paint_width.is_none() && e.meta.curve.is_none()
}

/// The corners of `e`'s path where it runs on the axes: a routed path's
/// ([`ortho_corners`]), or the two ends of a straight stroke.
fn corners_of(e: &SceneEdge) -> Option<Vec<Pt>> {
    if e.waypoints.len() >= 2 {
        return ortho_corners(e);
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
    (on(0) || on(1)).then(|| vec![a, b])
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
/// — costs less than sorting them all.
pub(crate) fn legs_for_pick(sc: &Scene) -> Legs {
    let mut out = Legs {
        level: Vec::new(),
        plumb: Vec::new(),
        edges: sc.edges.len(),
        sorted: false,
    };
    if sc.edges.len() > crate::casing::CASING_MAX_EDGES {
        return out;
    }
    for (i, e) in sc.edges.iter().enumerate() {
        if !hops_edge(e) {
            continue;
        }
        let Some(c) = corners_of(e) else { continue };
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
            // What each end of the leg keeps clear: for its own hops'
            // feet, and for the middle of a hop over it.
            let clear = |at_start: bool| -> (f64, f64) {
                let end = if at_start { k == 0 } else { k + 1 == n - 1 };
                let g = glyph[usize::from(!at_start)];
                match (end, g) {
                    (false, _) => (CORNER_R + HOP_CORNER_AIR, CORNER_R + HOP_R),
                    (true, true) => (head + HOP_HEAD_AIR, head + HOP_R + 1.0),
                    (true, false) => (HOP_END_AIR, HOP_R + 2.0),
                }
            };
            let v = |r: Pt| if level { r.0 } else { r.1 };
            let (a, b) = (v(p), v(q));
            let (ca, cb) = (clear(true), clear(false));
            // (Low end first.)
            let (lo, hi, c_lo, c_hi) = if a <= b {
                (a, b, ca, cb)
            } else {
                (b, a, cb, ca)
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
                width,
            };
            if level {
                out.level.push(leg);
            } else {
                out.plumb.push(leg);
            }
        }
    }
    out
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
        out
    }

    /// The hops of edge `i` alone, as [`Legs::all`] lists them.
    pub(crate) fn of(&self, i: usize) -> Vec<Hop> {
        if self.is_empty() {
            return Vec::new();
        }
        self.level
            .iter()
            .filter(|l| l.edge == i)
            .flat_map(|l| self.leg_hops(l))
            .collect()
    }

    /// The hops of one level leg (see the module docs).
    fn leg_hops(&self, leg: &Leg) -> Vec<Hop> {
        // The plumb legs crossing it, by where: (x, radius of a hop
        // there, or none where the crossing takes no hop).
        let across = if self.sorted {
            let from = self.plumb.partition_point(|v| v.at <= leg.lo + EPS);
            let to = self.plumb.partition_point(|v| v.at < leg.hi - EPS);
            &self.plumb[from..to.max(from)]
        } else {
            &self.plumb[..]
        };
        let mut xs: Vec<(f64, Option<f64>)> = Vec::new();
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
            let hop = r <= HOP_R_MAX && leg.at >= v.ok_lo - EPS && leg.at <= v.ok_hi + EPS;
            xs.push((v.at, hop.then_some(r)));
        }
        if !self.sorted {
            xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        // Neighbouring hops join into bridges; one whose feet find no
        // straight stretch gives up its outermost crossings until they
        // do.
        let mut out = Vec::new();
        let mut k = 0;
        while k < xs.len() {
            let Some(r) = xs[k].1 else {
                k += 1;
                continue;
            };
            let (mut end, mut r) = (k, r);
            while let Some(&(x, Some(rn))) = xs.get(end + 1) {
                if (x - rn) - (xs[end].0 + r.max(rn)) >= HOP_JOIN {
                    break;
                }
                (end, r) = (end + 1, r.max(rn));
            }
            let (mut a, mut b) = (k, end);
            while a <= b && xs[a].0 - r < leg.ok_lo - EPS {
                a += 1;
            }
            while b >= a && b > 0 && xs[b].0 + r > leg.ok_hi + EPS {
                b -= 1;
            }
            if a <= b && xs[b].0 + r <= leg.ok_hi + EPS {
                let hop = Hop {
                    y: leg.at,
                    x0: xs[a].0,
                    x1: xs[b].0,
                    r,
                    over: b - a + 1,
                };
                if self.clear_above(leg, &hop) {
                    out.push(hop);
                }
            }
            k = end + 1;
        }
        out
    }

    /// Whether no other level leg runs right above `hop`, where its
    /// crown would meet it.
    fn clear_above(&self, leg: &Leg, hop: &Hop) -> bool {
        let (lo, hi) = hop.span();
        let top = hop.y - hop.r - HOP_ABOVE_AIR;
        // (Any stroke a hop steps under is narrower than this.)
        let above = if self.sorted {
            let from = self.level.partition_point(|l| l.at < top - 4.0 * HOP_R_MAX);
            let to = self.level.partition_point(|l| l.at < hop.y - EPS);
            &self.level[from..to.max(from)]
        } else {
            &self.level[..]
        };
        above.iter().all(|l| {
            let reach = top - (l.width + leg.width) / 2.0;
            l.at >= hop.y - EPS || l.at < reach || l.hi <= lo || l.lo >= hi
        })
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
        sc
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
}
