// ------------------------------------------------------------------
// Edge label chips
// ------------------------------------------------------------------

/// Room a label chip keeps from every box, junction dot, group title
/// and other chip.
const CHIP_GAP: f64 = 3.0;
/// Room kept between a route's end (its port, and any arrowhead there)
/// and a chip on the route's first or last leg.
const CHIP_END: f64 = 14.0;
/// Room kept between a route's corner and a chip on one of its legs,
/// so the chip covers neither the rounded bend nor the next leg.
const CHIP_CORNER: f64 = CORNER_R + 2.0;
/// How far off its line a chip set beside a leg keeps, and how much
/// further out each fallback row beside it goes.
const BESIDE_GAP: f64 = 3.0;
const BESIDE_STEP: f64 = 14.0;
/// Rows of fallback spots beside each leg, on each side.
const BESIDE_ROWS: usize = 3;
/// Spots tried along one leg, spread end to end (plus its middle).
const CHIP_SLIDES: usize = 9;
/// What a chip spot costs. Covering a box, a junction, a title or
/// another chip is never done while any other spot is free; a line
/// through the chip (another edge's, or a bend of its own route) is
/// next worst: the line reads as stopping at the chip. A chip on a
/// trunk it shares with another edge (a fan-in) labels both, so it
/// costs more than one set just beside its own line, and each row
/// further out more again. The rest only picks the tidiest of the free
/// spots: the middle of a long leg, flat legs over upright ones, inside
/// a frame rather than across its side, and on the canvas already
/// drawn rather than widening it.
const CHIP_BLOCKED: f64 = 1e6;
const CHIP_CROSS: f64 = 2_000.0;
const CHIP_SHARED: f64 = 400.0;
const CHIP_BESIDE: f64 = 150.0;
const CHIP_FAR: f64 = 120.0;
const CHIP_SPILL: f64 = 50.0;
const CHIP_FRAME: f64 = 25.0;
const CHIP_UPRIGHT: f64 = 20.0;
/// Per px a spot sits off the middle of its leg, and per px its leg is
/// shorter than the route's longest.
const CHIP_OFF_MID: f64 = 0.25;
const CHIP_SHORT_LEG: f64 = 0.05;

/// A place a label chip may go: its centre, the leg of its route (by
/// index of the leg's first corner) it labels, whether it sits on that
/// leg's line (else beside it), and its price before collisions.
struct Spot {
    c: Pt,
    leg: usize,
    on: bool,
    base: f64,
}

/// The box a chip of width `w` centred on `c` draws.
fn chip_rect(c: Pt, w: f64) -> Rect {
    Rect {
        x0: c.0 - w / 2.0,
        y0: c.1 - LABEL_BOX_H / 2.0,
        x1: c.0 + w / 2.0,
        y1: c.1 + LABEL_BOX_H / 2.0,
    }
}

/// Whether two rects share interior.
fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// Whether `a` lies within `b`.
fn within(a: &Rect, b: &Rect) -> bool {
    a.x0 >= b.x0 - EPS && a.x1 <= b.x1 + EPS && a.y0 >= b.y0 - EPS && a.y1 <= b.y1 + EPS
}

/// Whether the segment `p`–`q` passes through the interior of `r`
/// (Liang–Barsky clipping; a segment along `r`'s outline does not).
fn seg_hits(p: Pt, q: Pt, r: &Rect) -> bool {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (pk, qk) in [
        (-dx, p.0 - r.x0),
        (dx, r.x1 - p.0),
        (-dy, p.1 - r.y0),
        (dy, r.y1 - p.1),
    ] {
        if pk.abs() < EPS {
            if qk <= EPS {
                return false;
            }
        } else if pk < 0.0 {
            t0 = t0.max(qk / pk);
        } else {
            t1 = t1.min(qk / pk);
        }
    }
    t0 < t1
}

/// Positions to try along a leg for a chip centre between `first` and
/// `last`: [`CHIP_SLIDES`] spread end to end, and the leg's middle `mid`
/// (where it falls between them).
fn slides(first: f64, last: f64, mid: f64) -> Vec<f64> {
    let mut v = vec![mid.clamp(first, last)];
    for j in 0..CHIP_SLIDES {
        let u = first + (last - first) * j as f64 / (CHIP_SLIDES - 1) as f64;
        if v.iter().all(|&o| (o - u).abs() >= 1.0) {
            v.push(u);
        }
    }
    v
}

/// Every spot a chip of width `w` may take on `route`: on each straight
/// leg that holds it clear of the leg's ends ([`CHIP_END`] at a port,
/// [`CHIP_CORNER`] at a bend), and in [`BESIDE_ROWS`] rows beside every
/// leg on both sides.
fn spots(route: &[Pt], w: f64) -> Vec<Spot> {
    let n = route.len();
    let longest = route
        .windows(2)
        .map(|s| dist(s[0], s[1]))
        .fold(0.0, f64::max);
    let inset = |corner: usize| {
        if corner == 0 || corner + 1 == n {
            CHIP_END
        } else {
            CHIP_CORNER
        }
    };
    let mut out = Vec::new();
    for (i, s) in route.windows(2).enumerate() {
        let (a, b) = (s[0], s[1]);
        let flat = (a.1 - b.1).abs() < EPS;
        if !flat && (a.0 - b.0).abs() >= EPS {
            continue;
        }
        // Coordinates along the leg (x on a flat leg) and of its line.
        let (u0, u1, line) = if flat { (a.0, b.0, a.1) } else { (a.1, b.1, a.0) };
        let (lo, hi) = (u0.min(u1), u0.max(u1));
        if hi - lo < EPS {
            continue;
        }
        let (in_lo, in_hi) = if u0 <= u1 {
            (inset(i), inset(i + 1))
        } else {
            (inset(i + 1), inset(i))
        };
        let (along, across) = if flat { (w, LABEL_BOX_H) } else { (LABEL_BOX_H, w) };
        let mid = (lo + hi) / 2.0;
        let (first, last) = (lo + in_lo + along / 2.0, hi - in_hi - along / 2.0);
        let fits = first <= last;
        // Too short to hold the chip: beside it only, centred, or
        // flush with either end (overhanging the other).
        let at = if fits {
            slides(first, last, mid)
        } else {
            vec![mid, lo + along / 2.0, hi - along / 2.0]
        };
        let pt = |u: f64, off: f64| if flat { (u, line + off) } else { (line + off, u) };
        let shape = (longest - (hi - lo)) * CHIP_SHORT_LEG + if flat { 0.0 } else { CHIP_UPRIGHT };
        if fits {
            for &u in &at {
                out.push(Spot {
                    c: pt(u, 0.0),
                    leg: i,
                    on: true,
                    base: shape + (u - mid).abs() * CHIP_OFF_MID,
                });
            }
        }
        for row in 0..BESIDE_ROWS {
            let off = across / 2.0 + BESIDE_GAP + row as f64 * BESIDE_STEP;
            // Above (or right of) the line first: a tie goes there.
            for (tie, sign) in [(0.0, -1.0), (1.0, 1.0)] {
                let sign = if flat { sign } else { -sign };
                for &u in &at {
                    out.push(Spot {
                        c: pt(u, sign * off),
                        leg: i,
                        on: false,
                        base: CHIP_BESIDE
                            + row as f64 * CHIP_FAR
                            + shape
                            + (u - mid).abs() * CHIP_OFF_MID
                            + tie,
                    });
                }
            }
        }
    }
    out
}

/// What label chips must keep off and should keep within.
struct ChipField<'a> {
    /// Every edge's route, as its corner points.
    routes: &'a [Vec<Pt>],
    /// Boxes, junction dots and group titles, grown by [`CHIP_GAP`].
    blocks: &'a [Rect],
    frames: &'a [Rect],
    /// The drawing so far: boxes, frames and routes, with margins.
    bounds: Rect,
}

impl ChipField<'_> {
    /// Price of spot `s` for edge `k`'s chip of width `w`, before other
    /// chips are considered (see [`CHIP_BLOCKED`] and the costs after it).
    fn price(&self, k: usize, s: &Spot, w: f64) -> f64 {
        let r = chip_rect(s.c, w);
        let mut cost = s.base;
        if self.blocks.iter().any(|b| overlaps(&r, b)) {
            cost += CHIP_BLOCKED;
        }
        let route = &self.routes[k];
        let (a, b) = (route[s.leg], route[s.leg + 1]);
        let flat = (a.1 - b.1).abs() < EPS;
        let line = if flat { a.1 } else { a.0 };
        // The chip's extent along its leg, and how much of that another
        // edge runs along too.
        let (lo, hi) = if flat { (r.x0, r.x1) } else { (r.y0, r.y1) };
        let mut shared = 0.0;
        // A line that grazes the chip's outline reads as running into it.
        let near = r.grow(1.5);
        for (j, other) in self.routes.iter().enumerate() {
            for (i, q) in other.windows(2).enumerate() {
                if j == k && i == s.leg {
                    continue;
                }
                let (p0, p1) = (q[0], q[1]);
                if j != k {
                    let along = if flat {
                        ((p0.1 - p1.1).abs() < EPS && (p0.1 - line).abs() < NEAR)
                            .then(|| (p0.0.min(p1.0), p0.0.max(p1.0)))
                    } else {
                        ((p0.0 - p1.0).abs() < EPS && (p0.0 - line).abs() < NEAR)
                            .then(|| (p0.1.min(p1.1), p0.1.max(p1.1)))
                    };
                    if let Some((q0, q1)) = along {
                        shared += (hi.min(q1) - lo.max(q0)).max(0.0);
                        if s.on {
                            continue;
                        }
                    }
                }
                if seg_hits(p0, p1, &near) {
                    cost += CHIP_CROSS;
                }
            }
        }
        cost += CHIP_SHARED * (shared / (hi - lo)).min(1.0);
        for f in self.frames {
            if overlaps(&r, f) && !within(&r, f) {
                cost += CHIP_FRAME;
            }
        }
        if !within(&r, &self.bounds) {
            cost += CHIP_SPILL;
        }
        cost
    }
}

/// Centre of each edge's label chip, index-parallel with `widths` (a
/// chip's width, `None` for an unlabelled edge): the cheapest of its
/// [`spots`] by [`ChipField::price`], never on a chip placed before it.
/// The edges with the fewest free spots choose first, so a chip with
/// room to spare does not take the only spot another one had.
fn place_chips(field: &ChipField, widths: &[Option<f64>]) -> Vec<Option<Pt>> {
    let priced: Vec<Vec<(Spot, f64)>> = widths
        .iter()
        .enumerate()
        .map(|(k, w)| match w {
            Some(w) => spots(&field.routes[k], *w)
                .into_iter()
                .map(|s| {
                    let p = field.price(k, &s, *w);
                    (s, p)
                })
                .collect(),
            None => Vec::new(),
        })
        .collect();
    let mut order: Vec<usize> = (0..widths.len()).filter(|&k| widths[k].is_some()).collect();
    order.sort_by_key(|&k| priced[k].iter().filter(|(_, p)| *p < CHIP_SHARED).count());
    let mut out = vec![None; widths.len()];
    let mut placed: Vec<Rect> = Vec::new();
    for k in order {
        let w = widths[k].unwrap_or_default();
        let mut best: Option<(f64, Pt)> = None;
        for (s, p) in &priced[k] {
            let r = chip_rect(s.c, w).grow(CHIP_GAP);
            let cost = p + if placed.iter().any(|o| overlaps(&r, o)) {
                CHIP_BLOCKED
            } else {
                0.0
            };
            if best.map_or(true, |(b, _)| cost < b) {
                best = Some((cost, s.c));
            }
        }
        // A route always has a leg, so it always has spots; a
        // degenerate one-point route labels its only point.
        let c = best.map_or(field.routes[k][0], |(_, c)| c);
        placed.push(chip_rect(c, w));
        out[k] = Some(c);
    }
    out
}
