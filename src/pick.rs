//! Shared geometry of the pick methods of the scenes without a generic
//! [`crate::scene::Scene`] (mindmap, sequence, pie, journey, timeline,
//! sankey): each tests a scene point against the shapes its painter
//! draws, in the painter's own terms (its rectangles, rounded corners,
//! polygons, strokes and text boxes), so a pick lands where the SVG
//! shows the element: a filled shape the painter outlines takes in its
//! outline's stroke, and a line ends as the painter ends it (square,
//! the SVG default, unless it draws round caps).
//!
//! The conventions are [`crate::scene::Scene::hit_test`]'s: a point in
//! scene space, a tolerance in scene units that widens only strokes
//! (lines, connectors), never a filled shape, and `tol <= 0` for exact
//! geometry. Elements are offered in layers, topmost first; within a
//! layer the nearest wins and, at equal distance (on the ink, distance
//! 0), the one painted later.

type Pt = (f64, f64);

/// Whether `(x, y)` lies on the canvas `[0, w] x [0, h]` (and is a
/// number): off it, nothing is picked.
pub(crate) fn on_canvas(x: f64, y: f64, w: f64, h: f64) -> bool {
    (0.0..=w).contains(&x) && (0.0..=h).contains(&y)
}

/// Distance from `p` to the segment `a`-`b`.
pub(crate) fn seg_dist(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

/// How far `p` lies off the stroke `width` wide that SVG paints along
/// the segment `a`-`b` with butt ends (the default `stroke-linecap`):
/// 0 on that rectangle, else the distance to it. `ext` (at `a`, at `b`)
/// first pushes the ends out along the segment: half the width at a
/// polyline's inner corner, where a right-angle miter join fills the
/// square two butt ends leave open. A segment of no length paints
/// nothing: infinity.
pub(crate) fn butt_dist(p: Pt, a: Pt, b: Pt, width: f64, ext: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dy);
    if !(len > 0.0 && len.is_finite()) {
        return f64::INFINITY;
    }
    let (ux, uy) = (dx / len, dy / len);
    let (qx, qy) = (p.0 - a.0, p.1 - a.1);
    let along = qx * ux + qy * uy;
    let across = (qx * uy - qy * ux).abs();
    let off_along = (-ext.0 - along).max(along - len - ext.1).max(0.0);
    let off_across = (across - width / 2.0).max(0.0);
    off_along.hypot(off_across)
}

/// [`butt_dist`] to the polyline through `pts` stroked `width` wide,
/// butt ends and miter joins, as a `<path>` of `L` steps paints it:
/// exact along straight runs and at right-angle corners (a sequence
/// message's self-loop). Fewer than two points: infinity.
pub(crate) fn butt_polyline_dist(p: Pt, pts: &[Pt], width: f64) -> f64 {
    let n = pts.len();
    let h = width / 2.0;
    (1..n)
        .map(|k| {
            let ext = (if k > 1 { h } else { 0.0 }, if k + 1 < n { h } else { 0.0 });
            butt_dist(p, pts[k - 1], pts[k], width, ext)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Whether `p` lies on the SVG `<rect x y width height rx>` filled and
/// stroked `sw` wide: [`in_round_rect`] grown by half the stroke on each
/// side, its round corners `rx + sw / 2` round (square ones stay square,
/// as a miter join paints them).
pub(crate) fn in_stroked_round_rect(
    p: Pt,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    rx: f64,
    sw: f64,
) -> bool {
    let d = (sw / 2.0).max(0.0);
    let r = if rx > 0.0 { rx + d } else { 0.0 };
    in_round_rect(p, x - d, y - d, w + 2.0 * d, h + 2.0 * d, r)
}

/// Distance from `p` to the polyline through `pts` (a lone point: the
/// distance to it; none: infinity).
pub(crate) fn polyline_dist(p: Pt, pts: &[Pt]) -> f64 {
    match pts {
        [] => f64::INFINITY,
        [a] => (p.0 - a.0).hypot(p.1 - a.1),
        _ => pts
            .windows(2)
            .map(|w| seg_dist(p, w[0], w[1]))
            .fold(f64::INFINITY, f64::min),
    }
}

/// How far a flattened piece of a cubic may bow off the chord it is
/// measured by: far below any stroke width or the 0.1 px the painters
/// round to.
const FLATNESS: f64 = 0.01;

/// Distance from `p` to the cubic Bézier `b`, or infinity when the curve
/// lies farther than `lim`: halved until each piece is flat to within
/// [`FLATNESS`] of its chord, then measured to the chord, pieces whose
/// control box lies farther than the nearest so far left unmeasured. A
/// straight cubic (its controls on its chord, as a mindmap connector's)
/// is its chord at once.
pub(crate) fn cubic_dist(p: Pt, b: &[Pt; 4], lim: f64) -> f64 {
    fn mid(a: Pt, b: Pt) -> Pt {
        ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
    }
    fn go(p: Pt, b: &[Pt; 4], lim: f64, depth: u32) -> f64 {
        if box_gap(p, b) > lim {
            return f64::INFINITY;
        }
        // The curve lies in its control points' hull, so within `bow`
        // of its chord (a curve off the numbers is not split).
        let bow = seg_dist(b[1], b[0], b[3]).max(seg_dist(b[2], b[0], b[3]));
        if !bow.is_finite() || bow <= FLATNESS || depth >= 24 {
            return seg_dist(p, b[0], b[3]);
        }
        // de Casteljau split at t = 1/2.
        let (ab, bc, cd) = (mid(b[0], b[1]), mid(b[1], b[2]), mid(b[2], b[3]));
        let (abc, bcd) = (mid(ab, bc), mid(bc, cd));
        let m = mid(abc, bcd);
        let left = go(p, &[b[0], ab, abc, m], lim, depth + 1);
        if left <= 0.0 {
            return left;
        }
        left.min(go(p, &[m, bcd, cd, b[3]], lim.min(left), depth + 1))
    }
    go(p, b, lim, 0)
}

/// Distance from `p` to the bounding box of `pts` (0 inside it).
pub(crate) fn box_gap(p: Pt, pts: &[Pt]) -> f64 {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for q in pts {
        (x0, y0, x1, y1) = (x0.min(q.0), y0.min(q.1), x1.max(q.0), y1.max(q.1));
    }
    let dx = (x0 - p.0).max(p.0 - x1).max(0.0);
    let dy = (y0 - p.1).max(p.1 - y1).max(0.0);
    dx.hypot(dy)
}

/// Whether `p` lies in the rectangle at `(x, y)` (top-left), `w` x `h`.
pub(crate) fn in_rect(p: Pt, x: f64, y: f64, w: f64, h: f64) -> bool {
    p.0 >= x && p.0 <= x + w && p.1 >= y && p.1 <= y + h
}

/// Whether `p` lies in the SVG `<rect x y width height rx>` (`ry` =
/// `rx`): each radius clamped to half its side, as SVG draws it.
pub(crate) fn in_round_rect(p: Pt, x: f64, y: f64, w: f64, h: f64, rx: f64) -> bool {
    if !in_rect(p, x, y, w, h) {
        return false;
    }
    let (rx, ry) = (rx.min(w / 2.0).max(0.0), rx.min(h / 2.0).max(0.0));
    if rx <= 0.0 || ry <= 0.0 {
        return true;
    }
    // Distance into the corner zone on each axis (<= 0: not in it).
    let ox = (x + rx - p.0).max(p.0 - (x + w - rx));
    let oy = (y + ry - p.1).max(p.1 - (y + h - ry));
    ox <= 0.0 || oy <= 0.0 || (ox / rx).powi(2) + (oy / ry).powi(2) <= 1.0
}

/// Whether `p` lies in the disc of radius `r` round `c`.
pub(crate) fn in_disc(p: Pt, c: Pt, r: f64) -> bool {
    (p.0 - c.0).hypot(p.1 - c.1) <= r
}

/// Whether `p` lies in the polygon `pts` (closed; nonzero winding, the
/// SVG default fill rule).
pub(crate) fn in_polygon(p: Pt, pts: &[Pt]) -> bool {
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let mut wind = 0i32;
    for k in 0..n {
        let (a, b) = (pts[k], pts[(k + 1) % n]);
        // A point on the outline is inside.
        if seg_dist(p, a, b) <= 1e-9 {
            return true;
        }
        let cross = (b.0 - a.0) * (p.1 - a.1) - (p.0 - a.0) * (b.1 - a.1);
        if a.1 <= p.1 {
            if b.1 > p.1 && cross > 0.0 {
                wind += 1;
            }
        } else if b.1 <= p.1 && cross < 0.0 {
            wind -= 1;
        }
    }
    wind != 0
}

/// The box `(x, y, w, h)` of one text line `w` wide whose anchor point
/// is `(ax, cy)` (`cy` its vertical middle, as `dy="0.33em"` draws it)
/// and which is `h` tall, for the SVG `text-anchor` `anchor`.
pub(crate) fn text_box(ax: f64, cy: f64, w: f64, h: f64, anchor: &str) -> (f64, f64, f64, f64) {
    let x = match anchor {
        "middle" => ax - w / 2.0,
        "end" => ax - w,
        _ => ax,
    };
    (x, cy - h / 2.0, w, h)
}

/// Whether `p` lies in the box `b` (`(x, y, w, h)`).
pub(crate) fn in_box(p: Pt, b: (f64, f64, f64, f64)) -> bool {
    in_rect(p, b.0, b.1, b.2, b.3)
}

/// The pick of one layer: the element nearest the point within the
/// tolerance (0 on its ink), the later one at equal distance.
pub(crate) struct Nearest {
    tol: f64,
    best: Option<(usize, f64)>,
}

impl Nearest {
    /// A layer picked with tolerance `tol` (`<= 0`: exact).
    pub(crate) fn new(tol: f64) -> Nearest {
        Nearest {
            tol: if tol > 0.0 { tol } else { 0.0 },
            best: None,
        }
    }

    /// Element `i` lies `d` from the point (0 on its ink; infinity off
    /// a filled shape). Offer elements in paint order.
    pub(crate) fn offer(&mut self, i: usize, d: f64) {
        if d <= self.tol && self.best.map_or(true, |(_, b)| d <= b) {
            self.best = Some((i, d));
        }
    }

    /// Offer element `i` as a filled shape: on it or not.
    pub(crate) fn offer_area(&mut self, i: usize, inside: bool) {
        if inside {
            self.offer(i, 0.0);
        }
    }

    /// Offer element `i` as a stroke `width` wide, `d` from its centre
    /// line.
    pub(crate) fn offer_stroke(&mut self, i: usize, d: f64, width: f64) {
        self.offer(i, (d - width / 2.0).max(0.0));
    }

    /// The element picked, if any.
    pub(crate) fn get(&self) -> Option<usize> {
        self.best.map(|(i, _)| i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rect_leaves_its_corners_empty() {
        // 40 x 20 at the origin, corners 6 round.
        assert!(in_round_rect((20.0, 10.0), 0.0, 0.0, 40.0, 20.0, 6.0));
        assert!(in_round_rect((6.0, 0.0), 0.0, 0.0, 40.0, 20.0, 6.0));
        assert!(!in_round_rect((0.5, 0.5), 0.0, 0.0, 40.0, 20.0, 6.0));
        assert!(!in_round_rect((39.5, 19.5), 0.0, 0.0, 40.0, 20.0, 6.0));
        // On the corner arc's 45-degree point, and just past it.
        let k = 6.0 - 6.0 / 2f64.sqrt();
        assert!(in_round_rect(
            (k + 0.01, k + 0.01),
            0.0,
            0.0,
            40.0,
            20.0,
            6.0
        ));
        assert!(!in_round_rect(
            (k - 0.05, k - 0.05),
            0.0,
            0.0,
            40.0,
            20.0,
            6.0
        ));
        // A radius past half a side is clamped, as SVG clamps it: a pill.
        assert!(!in_round_rect((1.0, 1.0), 0.0, 0.0, 40.0, 20.0, 30.0));
        assert!(in_round_rect((20.0, 0.5), 0.0, 0.0, 40.0, 20.0, 30.0));
        assert!(!in_round_rect((40.5, 10.0), 0.0, 0.0, 40.0, 20.0, 0.0));
    }

    #[test]
    fn polygon_and_disc() {
        let diamond = [(0.0, -5.0), (5.0, 0.0), (0.0, 5.0), (-5.0, 0.0)];
        assert!(in_polygon((0.0, 0.0), &diamond));
        assert!(in_polygon((2.5, 2.5), &diamond), "on the outline");
        assert!(!in_polygon((3.0, 3.0), &diamond));
        assert!(!in_polygon((0.0, 0.0), &diamond[..2]));
        assert!(in_disc((3.0, 4.0), (0.0, 0.0), 5.0));
        assert!(!in_disc((3.0, 4.1), (0.0, 0.0), 5.0));
    }

    #[test]
    fn distances_to_lines_and_curves() {
        assert_eq!(seg_dist((5.0, 3.0), (0.0, 0.0), (10.0, 0.0)), 3.0);
        assert_eq!(seg_dist((13.0, 4.0), (0.0, 0.0), (10.0, 0.0)), 5.0);
        assert_eq!(polyline_dist((5.0, 3.0), &[]), f64::INFINITY);
        // A straight cubic is its chord.
        let inf = f64::INFINITY;
        let b = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0)];
        assert!((cubic_dist((15.0, 2.0), &b, inf) - 2.0).abs() < 1e-9);
        assert!((cubic_dist((33.0, 4.0), &b, inf) - 5.0).abs() < 1e-9);
        // A bent one: its apex at t = 0.5 is 7.5 above the chord.
        let arc = [(0.0, 0.0), (0.0, -10.0), (30.0, -10.0), (30.0, 0.0)];
        assert!(cubic_dist((15.0, -7.5), &arc, inf) < 1e-6);
        assert!((cubic_dist((15.0, 0.0), &arc, inf) - 7.5).abs() < 0.01);
        // Against a dense sampling of the curve, at points round it.
        for k in 0..200 {
            let q = (-5.0 + 0.2 * k as f64, -12.0 + 0.09 * k as f64);
            let dense = (0..=20_000)
                .map(|i| {
                    let t = i as f64 / 20_000.0;
                    let u = 1.0 - t;
                    let (k0, k1, k2, k3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    let x = k0 * arc[0].0 + k1 * arc[1].0 + k2 * arc[2].0 + k3 * arc[3].0;
                    let y = k0 * arc[0].1 + k1 * arc[1].1 + k2 * arc[2].1 + k3 * arc[3].1;
                    (q.0 - x).hypot(q.1 - y)
                })
                .fold(f64::INFINITY, f64::min);
            assert!((cubic_dist(q, &arc, inf) - dense).abs() < 0.011, "{q:?}");
        }
        // Farther than `lim`: not measured.
        assert_eq!(cubic_dist((15.0, 40.0), &arc, 3.0), f64::INFINITY);
        assert_eq!(box_gap((15.0, -5.0), &arc), 0.0);
        assert_eq!(box_gap((33.0, 4.0), &arc), 5.0);
    }

    #[test]
    fn butt_ends_stop_at_the_segment_ends() {
        let (a, b) = ((0.0, 0.0), (10.0, 0.0));
        // On the stroke, 2 wide: across it and up to each end.
        assert_eq!(butt_dist((5.0, 1.0), a, b, 2.0, (0.0, 0.0)), 0.0);
        assert_eq!(butt_dist((10.0, -1.0), a, b, 2.0, (0.0, 0.0)), 0.0);
        // Past an end by 0.5 on the axis: off it (a round cap would
        // cover it), and 0.5 away.
        assert!((butt_dist((10.5, 0.0), a, b, 2.0, (0.0, 0.0)) - 0.5).abs() < 1e-12);
        assert!((butt_dist((-0.5, 0.0), a, b, 2.0, (0.0, 0.0)) - 0.5).abs() < 1e-12);
        // Off a corner of the painted rectangle: the distance to it.
        assert!((butt_dist((13.0, 5.0), a, b, 2.0, (0.0, 0.0)) - 5.0).abs() < 1e-12);
        // An end pushed out covers what lies before it.
        assert_eq!(butt_dist((10.5, 0.0), a, b, 2.0, (0.0, 1.0)), 0.0);
        assert_eq!(butt_dist((1.0, 0.0), a, a, 2.0, (0.0, 0.0)), f64::INFINITY);
        // A right-angle polyline: the outer square of its corner is
        // painted (miter), its two free ends are butt.
        let pts = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        assert_eq!(butt_polyline_dist((10.9, -0.9), &pts, 2.0), 0.0);
        assert!(butt_polyline_dist((-0.5, 0.0), &pts, 2.0) > 0.0);
        assert!(butt_polyline_dist((10.0, 10.5), &pts, 2.0) > 0.0);
        assert_eq!(
            butt_polyline_dist((1.0, 0.0), &pts[..1], 2.0),
            f64::INFINITY
        );
    }

    #[test]
    fn a_stroked_shape_covers_its_outline_stroke() {
        // 40 x 20, corners 4 round, stroked 1.6: 0.8 more on each side.
        let r = |p| in_stroked_round_rect(p, 0.0, 0.0, 40.0, 20.0, 4.0, 1.6);
        assert!(r((-0.7, 10.0)) && r((20.0, 20.7)));
        assert!(!r((-0.9, 10.0)) && !r((20.0, 20.9)));
        // The corner arc grows by the same 0.8: radius 4.8 round (4, 4).
        let k = 4.0 - 4.8 / 2f64.sqrt();
        assert!(r((k + 0.01, k + 0.01)) && !r((k - 0.05, k - 0.05)));
        // Square corners stay square.
        assert!(in_stroked_round_rect(
            (-0.5, -0.5),
            0.0,
            0.0,
            4.0,
            4.0,
            0.0,
            1.4
        ));
        assert!(!in_stroked_round_rect(
            (-0.8, 0.0),
            0.0,
            0.0,
            4.0,
            4.0,
            0.0,
            1.4
        ));
    }

    #[test]
    fn nearest_prefers_the_closer_then_the_later() {
        let mut n = Nearest::new(3.0);
        n.offer(0, 2.0);
        n.offer(1, 1.0);
        n.offer(2, 1.0);
        n.offer(3, 3.5);
        assert_eq!(n.get(), Some(2));
        // tol <= 0: only what lies on the ink.
        let mut exact = Nearest::new(-1.0);
        exact.offer_stroke(0, 1.0, 1.6);
        assert_eq!(exact.get(), None);
        exact.offer_stroke(1, 0.7, 1.6);
        assert_eq!(exact.get(), Some(1));
        assert!(!on_canvas(f64::NAN, 1.0, 10.0, 10.0));
        assert!(on_canvas(10.0, 0.0, 10.0, 10.0));
        let b = text_box(10.0, 5.0, 8.0, 4.0, "end");
        assert_eq!(b, (2.0, 3.0, 8.0, 4.0));
    }
}
