//! Edge-label placement (G11).
//!
//! A label is a [`LABEL_BOX_H`]-tall box centred on its own edge. It
//! must never cover a node, a subgraph frame's border line, a frame's
//! title chip, an arrowhead or another label (the *hard* obstacles),
//! and it prefers spots no other edge's stroke runs through (the *soft*
//! ones, priced but never forbidden). Spots are taken along the curve
//! the edge is actually drawn with, so a label always stays on its own
//! line.
//!
//! Two entry points share one placer:
//! - [`avoid_label_overlaps`]: run by every scene builder once all
//!   edges are drawn; frees only the labels that hit a hard obstacle
//!   (or crowd another label), so a label keeps its layout slot
//!   otherwise and the edge stages route round where labels will be.
//! - [`relabel`]: the edge pipeline's last label stage, once every edge
//!   is drawn for good; also moves labels of edges a later stage
//!   changed (`moved`) and labels another edge's stroke runs through,
//!   each only to a strictly better spot, so clean labels keep their
//!   exact spot.
//!
//! What a spot covers is ranked, never weighed: hidden text (a node, a
//! title chip, another label) loses to a covered arrowhead, which
//! loses to a frame border line or a label nearer than [`LABEL_KEEP`],
//! which loses to merely crossing strokes (see [`SpotKey`]). A label
//! looks along the middle of its curve first, then just beside it
//! (never more than [`LABEL_DRIFT_MAX`] from its line and never nearer
//! another edge than its own, so it always reads as its own line's
//! label), then anywhere on its curve. The most constrained labels go
//! first, and a few rounds let a label take a spot another one left.

use crate::edgepass::Ctx;
use crate::layout::{
    is_branch_point, text_width, title_extra, LABEL_BOX_H, LABEL_CLEAR, LINE_H, TITLE_SCALE,
};
use crate::model::{EdgeKind, Graph};
use crate::scene::{
    basis_sample, edge_drawn, edge_polyline, fit_cluster_title, shift_scene, Bbox, Scene,
    SceneCluster, SceneEdge, SceneNode, ARROW_MARKER_PX, MARGIN,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

/// How far a routed edge's label may sit from the curve actually drawn
/// before it is moved onto it — and the farthest a label box stepped
/// beside its line may be from that line.
pub(crate) const LABEL_DRIFT_MAX: f64 = LINE_H;

/// Where a routed edge's label goes: its middle channel point — the
/// slot the layout reserved — unless the drawn spline, which only
/// approximates the points, bends more than [`LABEL_DRIFT_MAX`] away
/// from it there (a zig-zagging lane); then the nearest point of the
/// curve, so the label stays on its own edge.
pub(crate) fn channel_label_point(wps: &[(f64, f64)]) -> (f64, f64) {
    let mid = wps[wps.len() / 2];
    let (d, near) = basis_sample(wps)
        .into_iter()
        .map(|p| ((p.0 - mid.0).hypot(p.1 - mid.1), p))
        .fold((f64::INFINITY, mid), |a, b| if b.0 < a.0 { b } else { a });
    if d > LABEL_DRIFT_MAX {
        near
    } else {
        mid
    }
}

/// Least room a MOVED edge label keeps from node boxes and other labels.
pub(crate) const LABEL_GAP: f64 = 4.0;
/// How many of the nearest spots a covering label sorts and tries
/// first; each further batch doubles.
const LABEL_SPOT_CHUNK: usize = 32;
/// Spacing (px) of the spots along its edge a covering label tries.
const LABEL_SLIDE_STEP: f64 = 6.0;
/// Overlap (px, per axis) below which two boxes count as just touching.
const TOUCH_EPS: f64 = 0.5;
/// Half-thickness of the band a subgraph frame's border line blocks
/// for edge labels (the line is drawn 1.4px wide).
pub(crate) const FRAME_BORDER_BAND: f64 = 1.0;
/// Margin (px) around another edge's stroke inside which a label counts
/// as lying on it — the stroke half-width plus a little air.
pub(crate) const STROKE_PAD: f64 = 3.0;
/// The preferred label spot (fraction of the drawn curve's length):
/// its middle. Spots farther along the curve rank lower only among
/// spots that are equally clear.
const LABEL_PREFER: f64 = 0.5;
/// Preferred spot on an edge leaving a hub (more edge ends than its
/// other end): the middle of the half away from it (mirrored for an
/// edge entering one).
const LABEL_HUB_PREFER: f64 = 0.75;
/// Least share of its line's length, counted from a compact branch
/// point (see [`is_branch_point`]), at which a label of the point's fan
/// keeps its spot: a little past the middle, so a label still at the
/// builder's midpoint slot (or nudged off it by the port stage) is
/// stacked afresh (see [`stack_fans`]).
pub(crate) const FAN_FAR: f64 = 0.55;
/// Least room (px) across the flow between two labels of one compact
/// branch point's fan side by side in a row: closer, two pills read as
/// one phrase ("invalid valid").
pub(crate) const FAN_SIBLING_GAP: f64 = 20.0;
/// How far (px) a fan label's centre may lie off its own line before it
/// is stacked afresh: rounding only.
const FAN_ON_LINE: f64 = 0.5;
/// Clear air (px) between a label set beside its line and that line's
/// centre beyond the box's own half-extent: half an arrowhead plus
/// [`LABEL_GAP`], so the box clears both the stroke and the head.
pub(crate) const LABEL_SIDE_AIR: f64 = ARROW_MARKER_PX / 2.0 + LABEL_GAP;
/// How far (px) a box stepped half off its line still reaches over the
/// line's centre: it covers its own stroke, so it plainly sits on it.
pub(crate) const LABEL_STRADDLE: f64 = 1.0;
/// Farthest (px) a label's centre may sit beside its line: twice the
/// reach of a box set one text line past a flat line, so a wide label
/// can also step beside a steep line. The box itself still keeps within
/// [`LABEL_DRIFT_MAX`] of its line, so it reads as that line's label.
const LABEL_SIDE_MAX: f64 = 2.0 * (LABEL_BOX_H / 2.0 + LABEL_SIDE_AIR + LINE_H);
/// Fraction of a curve's length at each end the first scan leaves out:
/// a label there crowds the end node and its arrowhead, so such spots
/// are tried only after every spot beside the middle of the line.
const LABEL_SCAN_END: f64 = 0.15;
/// Placement rounds: the first re-places every wanted label, most
/// constrained first; each further one re-tries the labels still not
/// clear of everything, now that the others have moved.
const LABEL_ROUNDS: usize = 3;
/// Most spots one curve is densified into: a longer curve (a node
/// dragged far away) is sampled at a coarser step, so placement stays
/// linear in the edge count, not in the canvas size.
const LABEL_MAX_DENSE: usize = 4096;
/// Work units one label pass of `route()` may spend: items (obstacles,
/// stroke segments, labels) its grid queries look at, 6-30 ns each
/// natively (1 ms is about 100k on crowded drawings). The real and
/// fixture diagrams spend at most a third of it; past it, the labels
/// not yet searched keep their own line's spot, so many labelled edges
/// sharing one channel cannot grow a drag frame without bound (G13).
/// Charged like the router's work budgets: by what the queries visit,
/// not by call.
pub(crate) const LABEL_ROUTE_WORK: usize = 100_000;
/// [`LABEL_ROUTE_WORK`] for the static scene, laid out once: at most
/// a few ms however many labels share a channel.
pub(crate) const LABEL_STATIC_WORK: usize = 400_000;
/// Most spots along its curve one label tries (a longer curve is
/// scanned at a coarser stride; the nearest-clear fallback still walks
/// every spot).
const LABEL_MAX_SPOTS: usize = 64;
/// Room (px) a label keeps from an arrowhead's box: it may touch it,
/// but [`TOUCH_EPS`] of overlap would already reach the drawn chord.
const ARROW_TOUCH_GAP: f64 = 2.0 * TOUCH_EPS;
/// Room (px) a label keeps from every other label, a hard keep-out:
/// [`LABEL_GAP`] of air plus the [`TOUCH_EPS`] an overlap test lets
/// through, so two labels never end up closer than [`LABEL_GAP`] (the
/// yardstick's `LABEL_CROWD_GAP`) without overlapping.
const LABEL_KEEP: f64 = LABEL_GAP + TOUCH_EPS;
/// Most spots of the middle stretch a label steps beside (evenly
/// strided, nearest the preferred spot first): a few dozen px apart is
/// fine enough beside a line, and every non-perfect label tries them.
const LABEL_BESIDE_SPOTS: usize = 8;
/// Most spots along the whole curve the fine sweep steps across the
/// line at: it tries every [`LABEL_BESIDE_STEP`] across each, so it is
/// the costliest search a label can need.
const LABEL_FINE_SPOTS: usize = 64;
/// Most steps each side of a line the fine sweep takes per spot.
const LABEL_FINE_STEPS: usize = 8;
/// Step (px) of the fine sweep across a line when no set distance
/// beside it is clear: half a [`LABEL_SLIDE_STEP`].
const LABEL_BESIDE_STEP: f64 = LABEL_SLIDE_STEP / 2.0;
/// Cluster title chip, as `to_svg` draws it: left / top inset inside
/// the frame, padding added to the title width, one-line height (px).
const CHIP_INSET_X: f64 = 6.0;
const CHIP_INSET_Y: f64 = 5.0;
const CHIP_PAD_W: f64 = 8.0;
const CHIP_H: f64 = 16.0;

/// Axis-aligned box as (x0, y0, x1, y1).
pub(crate) type Rect = (f64, f64, f64, f64);
/// A point (x, y).
type Pt = (f64, f64);

/// How much of `a` (grown by `gap` each side) the `obstacles` cover —
/// the summed overlap areas.
pub(crate) fn covered(a: Rect, gap: f64, obstacles: &[Rect]) -> f64 {
    obstacles
        .iter()
        .map(|o| {
            let w = (a.2 + gap).min(o.2) - (a.0 - gap).max(o.0);
            let h = (a.3 + gap).min(o.3) - (a.1 - gap).max(o.1);
            if w > TOUCH_EPS && h > TOUCH_EPS {
                w * h
            } else {
                0.0
            }
        })
        .sum()
}

/// Side (px) of a cell of the [`ObstacleGrid`] — about one label box.
const OBSTACLE_CELL: f64 = 96.0;
/// A rect spanning more cells than this is checked against every query
/// instead of being bucketed (a huge node or frame border line).
const OBSTACLE_MAX_CELLS: i64 = 256;

/// Cheap multiplicative hash for grid-cell keys — the default SipHash
/// dominated label placement on big charts. Keys are our own integers,
/// so no hash-flooding concern.
#[derive(Default)]
struct CellHasher(u64);

impl Hasher for CellHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(u64::from(b));
        }
    }
    fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

type CellMap<V> = HashMap<(i64, i64), Vec<V>, BuildHasherDefault<CellHasher>>;

/// Cell range (inclusive) a rect touches on a `cell`-sized grid, or
/// None when not finite.
fn cell_span(r: Rect, cell: f64) -> Option<(i64, i64, i64, i64)> {
    if !(r.0.is_finite() && r.1.is_finite() && r.2.is_finite() && r.3.is_finite()) {
        return None;
    }
    let c = |v: f64| (v / cell).floor() as i64;
    Some((c(r.0), c(r.1), c(r.2), c(r.3)))
}

/// Cells in an inclusive cell span, or None when that count does not
/// fit an i64 (a coordinate past +-OBSTACLE_CELL * 2^63 saturates its
/// cell index, and `x1 - x0 + 1` would wrap negative).
fn span_cells((x0, y0, x1, y1): (i64, i64, i64, i64)) -> Option<i64> {
    let w = x1.checked_sub(x0)?.checked_add(1)?;
    let h = y1.checked_sub(y0)?.checked_add(1)?;
    w.checked_mul(h)
}

/// Is query cell `c` the first cell (from the query's corner `q0`) in
/// which a walk over the query's cells meets `r`? Then a rect filed in
/// several cells is counted once, with no set to dedupe in.
fn first_cell(r: &Rect, q0: (i64, i64), c: (i64, i64)) -> bool {
    let k = |v: f64| (v / OBSTACLE_CELL).floor() as i64;
    c == (k(r.0).max(q0.0), k(r.1).max(q0.1))
}

/// The boxes edge labels must keep off, bucketed on a uniform grid so a
/// query looks only at those near it: label placement stays near-linear
/// in the number of edges instead of rescanning every node, frame and
/// earlier label for each of an edge's spots.
pub(crate) struct ObstacleGrid {
    pub(crate) rects: Vec<Rect>,
    cells: CellMap<usize>,
    /// Rects too large to bucket, checked by every query.
    big: Vec<usize>,
}

impl ObstacleGrid {
    pub(crate) fn new() -> Self {
        ObstacleGrid {
            rects: Vec::new(),
            cells: HashMap::default(),
            big: Vec::new(),
        }
    }

    /// Cell range (inclusive) a rect touches, or None when not finite.
    fn span(r: Rect) -> Option<(i64, i64, i64, i64)> {
        cell_span(r, OBSTACLE_CELL)
    }

    pub(crate) fn push(&mut self, r: Rect) {
        let i = self.rects.len();
        self.rects.push(r);
        match Self::span(r) {
            Some((x0, y0, x1, y1))
                if span_cells((x0, y0, x1, y1)).is_some_and(|n| n <= OBSTACLE_MAX_CELLS) =>
            {
                for cx in x0..=x1 {
                    for cy in y0..=y1 {
                        self.cells.entry((cx, cy)).or_default().push(i);
                    }
                }
            }
            _ => self.big.push(i),
        }
    }

    /// Indices of the rects that may overlap `q`, ascending, once each.
    #[cfg(test)]
    pub(crate) fn near(&self, q: Rect) -> Vec<usize> {
        let mut out = self.big.clone();
        match Self::span(q) {
            Some((x0, y0, x1, y1))
                if span_cells((x0, y0, x1, y1)).is_some_and(|n| n <= OBSTACLE_MAX_CELLS) =>
            {
                for cx in x0..=x1 {
                    for cy in y0..=y1 {
                        if let Some(v) = self.cells.get(&(cx, cy)) {
                            out.extend_from_slice(v);
                        }
                    }
                }
            }
            _ => out = (0..self.rects.len()).collect(),
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Same as [`covered`] over every rect pushed so far (summed in
    /// push order, so the result is bit-identical).
    #[cfg(test)]
    pub(crate) fn covered(&self, a: Rect, gap: f64) -> f64 {
        let q = (a.0 - gap, a.1 - gap, a.2 + gap, a.3 + gap);
        self.near(q)
            .into_iter()
            .map(|i| covered(a, gap, &self.rects[i..=i]))
            .sum()
    }

    /// Calls `f` once on the index of every rect that may overlap `q`,
    /// without collecting them (a rect filed in several cells is
    /// visited from the first of them only, see [`first_cell`]), until
    /// it returns false; then so does this.
    fn each(&self, q: Rect, mut f: impl FnMut(usize) -> bool) -> bool {
        if !self.big.iter().all(|&i| f(i)) {
            return false;
        }
        match Self::span(q) {
            Some((x0, y0, x1, y1))
                if span_cells((x0, y0, x1, y1)).is_some_and(|n| n <= OBSTACLE_MAX_CELLS) =>
            {
                for cx in x0..=x1 {
                    for cy in y0..=y1 {
                        let Some(v) = self.cells.get(&(cx, cy)) else {
                            continue;
                        };
                        for &i in v {
                            if first_cell(&self.rects[i], (x0, y0), (cx, cy)) && !f(i) {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            _ => {
                let big = &self.big;
                (0..self.rects.len()).all(|i| big.contains(&i) || f(i))
            }
        }
    }

    /// Does nothing pushed so far cover `a` grown by `gap`?
    #[cfg(test)]
    pub(crate) fn clear(&self, a: Rect, gap: f64) -> bool {
        let free = |i: &usize| covered(a, gap, &self.rects[*i..=*i]) == 0.0;
        if !self.big.iter().all(free) {
            return false;
        }
        let q = (a.0 - gap, a.1 - gap, a.2 + gap, a.3 + gap);
        match Self::span(q) {
            Some((x0, y0, x1, y1))
                if span_cells((x0, y0, x1, y1)).is_some_and(|n| n <= OBSTACLE_MAX_CELLS) =>
            {
                (x0..=x1).all(|cx| {
                    (y0..=y1).all(|cy| {
                        self.cells
                            .get(&(cx, cy))
                            .map_or(true, |v| v.iter().all(free))
                    })
                })
            }
            _ => (0..self.rects.len()).all(|i| free(&i)),
        }
    }
}

/// The edge labels as currently placed (index-parallel with the
/// edges), bucketed on an [`OBSTACLE_CELL`] grid so a spot only checks
/// the labels near it, and a label can move.
struct LabelGrid {
    rects: Vec<Option<Rect>>,
    cells: CellMap<usize>,
    /// Labels looked at by queries so far (the stage's work, see
    /// [`LABEL_ROUTE_WORK`]).
    visits: Cell<usize>,
}

impl LabelGrid {
    fn new(rects: Vec<Option<Rect>>) -> Self {
        let mut g = LabelGrid {
            rects: Vec::new(),
            cells: HashMap::default(),
            visits: Cell::new(0),
        };
        g.rects = vec![None; rects.len()];
        for (i, r) in rects.into_iter().enumerate() {
            if let Some(r) = r {
                g.set(i, r);
            }
        }
        g
    }

    fn cells_of(r: Rect) -> Vec<(i64, i64)> {
        match cell_span(r, OBSTACLE_CELL) {
            Some((x0, y0, x1, y1)) => (x0..=x1)
                .flat_map(|cx| (y0..=y1).map(move |cy| (cx, cy)))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Move label `i` to `r`.
    fn set(&mut self, i: usize, r: Rect) {
        if let Some(old) = self.rects[i] {
            for c in Self::cells_of(old) {
                if let Some(v) = self.cells.get_mut(&c) {
                    v.retain(|&j| j != i);
                }
            }
        }
        self.rects[i] = Some(r);
        for c in Self::cells_of(r) {
            self.cells.entry(c).or_default().push(i);
        }
    }

    /// Calls `f` once on every label other than `own` that may overlap
    /// `a` grown by `gap`, without collecting them (see [`first_cell`]).
    fn each(&self, own: usize, a: Rect, gap: f64, mut f: impl FnMut(&Rect)) {
        let Some((x0, y0, x1, y1)) =
            cell_span((a.0 - gap, a.1 - gap, a.2 + gap, a.3 + gap), OBSTACLE_CELL)
        else {
            return;
        };
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                let Some(v) = self.cells.get(&(cx, cy)) else {
                    continue;
                };
                self.visits.set(self.visits.get() + v.len());
                for &j in v {
                    if let Some(r) = &self.rects[j] {
                        if j != own && first_cell(r, (x0, y0), (cx, cy)) {
                            f(r);
                        }
                    }
                }
            }
        }
    }

    /// Summed overlap of `a` (grown by `gap`) with the other labels.
    fn cover(&self, own: usize, a: Rect, gap: f64) -> f64 {
        let mut sum = 0.0;
        self.each(own, a, gap, |r| {
            sum += covered(a, gap, std::slice::from_ref(r));
        });
        sum
    }

    /// How many other labels `a` (grown by `gap`) overlaps.
    fn hits(&self, own: usize, a: Rect, gap: f64) -> usize {
        let mut n = 0;
        self.each(own, a, gap, |r| {
            n += usize::from(covered(a, gap, std::slice::from_ref(r)) > 0.0);
        });
        n
    }

    /// Does no label other than `own` overlap `a` grown by `gap`? Walks
    /// the cells in place: it is asked for every candidate spot.
    fn clear(&self, own: usize, a: Rect, gap: f64) -> bool {
        let Some((x0, y0, x1, y1)) =
            cell_span((a.0 - gap, a.1 - gap, a.2 + gap, a.3 + gap), OBSTACLE_CELL)
        else {
            return true;
        };
        (x0..=x1).all(|cx| {
            (y0..=y1).all(|cy| {
                self.cells.get(&(cx, cy)).map_or(true, |v| {
                    self.visits.set(self.visits.get() + v.len());
                    v.iter().all(|&j| {
                        j == own || self.rects[j].map_or(true, |r| covered(a, gap, &[r]) == 0.0)
                    })
                })
            })
        })
    }
}

/// Side (px) of a cell of the [`StrokeGrid`]: a few sampled segments.
const STROKE_CELL: f64 = 64.0;

/// One sampled stroke segment: (edge index, start, end).
type StrokeSeg = (usize, (f64, f64), (f64, f64));

/// Do two boxes touch (closed)?
fn boxes_touch(a: &Rect, b: &Rect) -> bool {
    a.0 <= b.2 && a.2 >= b.0 && a.1 <= b.3 && a.3 >= b.1
}

/// Most cells along each side of the [`StrokeGrid`]: a drawing wider
/// than this many [`STROKE_CELL`]s (a node dragged far away) gets
/// coarser cells, so the grid's memory stays bounded whatever the
/// coordinates.
const STROKE_GRID_SIDE: f64 = 256.0;

/// Most grid cells one segment is bucketed into. A longer segment (a
/// node dragged thousands of px away) goes on a short list every query
/// checks instead, so building a grid stays linear in the segment count
/// whatever the coordinates.
pub(crate) const SEG_MAX_CELLS: usize = 256;

/// The cells (sorted, once each) of a `cell`-sized grid that segment
/// `a`–`b` passes through, or `None` when that is more than
/// [`SEG_MAX_CELLS`]. The segment is cut into pieces at most half a
/// cell long and each piece's box is bucketed, so the cost is linear in
/// the segment's length, not quadratic like its bounding box. A
/// non-finite segment touches no cell.
pub(crate) fn seg_cells(a: (f64, f64), b: (f64, f64), cell: f64) -> Option<Vec<(i64, i64)>> {
    if !(a.0.is_finite() && a.1.is_finite() && b.0.is_finite() && b.1.is_finite()) {
        return Some(Vec::new());
    }
    let len = (b.0 - a.0).hypot(b.1 - a.1);
    // Half a cell per piece: its box spans at most 2x2 cells.
    let pieces = (len / (cell / 2.0)).ceil().max(1.0);
    if !pieces.is_finite() || pieces > SEG_MAX_CELLS as f64 {
        return None;
    }
    let n = pieces as usize;
    let at = |k: usize| {
        let t = k as f64 / n as f64;
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    };
    let mut out = Vec::new();
    for k in 0..n {
        let (p, q) = (at(k), at(k + 1));
        let r = (p.0.min(q.0), p.1.min(q.1), p.0.max(q.0), p.1.max(q.1));
        if let Some((x0, y0, x1, y1)) = cell_span(r, cell) {
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    out.push((cx, cy));
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    (out.len() <= SEG_MAX_CELLS).then_some(out)
}

/// Every drawn edge's sampled stroke, bucketed by segment on a dense
/// grid over the drawing (cell lists packed in one array), so a label
/// spot asks only the strokes near it which edges run through it.
struct StrokeGrid {
    segs: Vec<StrokeSeg>,
    /// Bounding box of each segment.
    boxes: Vec<Rect>,
    /// Cell side (px), first cell and cell counts of the grid.
    cell: f64,
    org: (i64, i64),
    dim: (i64, i64),
    /// Cell `c`'s segments are `slots[start[c]..start[c + 1]]`, in
    /// segment order.
    start: Vec<u32>,
    slots: Vec<u32>,
    /// Segments too long to bucket, checked by every query.
    big: Vec<u32>,
    /// Per edge, the query that last counted it (so a count is linear
    /// in the segments it looks at), and the current query's stamp.
    seen: RefCell<Vec<u32>>,
    stamp: Cell<u32>,
    /// Segments looked at by queries so far (the stage's work).
    visits: Cell<usize>,
}

impl StrokeGrid {
    fn new(lines: &[Option<Vec<(f64, f64)>>]) -> Self {
        let mut segs = Vec::new();
        for (ei, pts) in lines.iter().enumerate() {
            let Some(pts) = pts else { continue };
            segs.extend(pts.windows(2).map(|w| (ei, w[0], w[1])));
        }
        let boxes: Vec<Rect> = segs.iter().map(|&(_, a, b)| seg_box(a, b)).collect();
        let mut bb = Bbox::new();
        for r in boxes
            .iter()
            .filter(|r| r.0.is_finite() && r.1.is_finite() && r.2.is_finite() && r.3.is_finite())
        {
            bb.add(r.0, r.1);
            bb.add(r.2, r.3);
        }
        let (x0, x1, y0, y1) = if bb.0.is_finite() {
            bb.finish()
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };
        let cell = STROKE_CELL.max((x1 - x0).max(y1 - y0) / STROKE_GRID_SIDE);
        let k = |v: f64| (v / cell).floor() as i64;
        let org = (k(x0), k(y0));
        let dim = (k(x1) - org.0 + 1, k(y1) - org.1 + 1);
        let mut g = StrokeGrid {
            segs,
            boxes,
            cell,
            org,
            dim,
            start: vec![0; (dim.0 * dim.1) as usize + 1],
            slots: Vec::new(),
            big: Vec::new(),
            seen: RefCell::new(vec![0; lines.len()]),
            stamp: Cell::new(0),
            visits: Cell::new(0),
        };
        // Two passes, counting then filling: one packed array, no
        // per-cell allocation.
        let mut start = std::mem::take(&mut g.start);
        for i in 0..g.segs.len() {
            if !g.cells_of(i, |c| start[c + 1] += 1) {
                g.big.push(i as u32);
            }
        }
        for c in 1..start.len() {
            start[c] += start[c - 1];
        }
        g.start = start;
        let mut fill = g.start.clone();
        let mut slots = vec![0; g.start[g.start.len() - 1] as usize];
        for i in 0..g.segs.len() {
            g.cells_of(i, |c| {
                slots[fill[c] as usize] = i as u32;
                fill[c] += 1;
            });
        }
        g.slots = slots;
        g
    }

    /// Calls `f` on the packed index of every cell segment `i` is filed
    /// in — a short one (most of a sampled curve) in the few cells of its
    /// box, a longer one cell by cell — or returns false, calling
    /// nothing, when it is too long to bucket.
    fn cells_of(&self, i: usize, mut f: impl FnMut(usize)) -> bool {
        let (_, a, b) = self.segs[i];
        let Some(span) = cell_span(self.boxes[i], self.cell) else {
            // Not finite: it touches no cell.
            return true;
        };
        if span.2 - span.0 <= 1 && span.3 - span.1 <= 1 {
            for cx in span.0..=span.2 {
                for cy in span.1..=span.3 {
                    if let Some(c) = self.index((cx, cy)) {
                        f(c);
                    }
                }
            }
            return true;
        }
        match seg_cells(a, b, self.cell) {
            Some(cells) => {
                cells.into_iter().filter_map(|c| self.index(c)).for_each(f);
                true
            }
            None => false,
        }
    }

    /// Index of cell `c` in the packed grid, if inside it.
    fn index(&self, c: (i64, i64)) -> Option<usize> {
        let (x, y) = (c.0 - self.org.0, c.1 - self.org.1);
        (x >= 0 && y >= 0 && x < self.dim.0 && y < self.dim.1)
            .then(|| (y * self.dim.0 + x) as usize)
    }

    /// Calls `f` on every segment that may touch `q` (the long ones
    /// first, then cell by cell; a segment can repeat across cells)
    /// until it returns false. A query box spanning too many cells (or
    /// not finite) visits every segment.
    fn each_seg(&self, q: Rect, mut f: impl FnMut(usize) -> bool) {
        let mut f = |i: usize| {
            self.visits.set(self.visits.get() + 1);
            f(i)
        };
        match cell_span(q, self.cell) {
            Some((x0, y0, x1, y1))
                if span_cells((x0, y0, x1, y1)).is_some_and(|n| n <= OBSTACLE_MAX_CELLS) =>
            {
                if !self.big.iter().all(|&i| f(i as usize)) {
                    return;
                }
                // Clipped to the grid: no segment lies outside it.
                let (x0, y0) = (x0.max(self.org.0), y0.max(self.org.1));
                let (x1, y1) = (
                    x1.min(self.org.0 + self.dim.0 - 1),
                    y1.min(self.org.1 + self.dim.1 - 1),
                );
                for cy in y0..=y1 {
                    for cx in x0..=x1 {
                        let Some(c) = self.index((cx, cy)) else {
                            continue;
                        };
                        let cell = &self.slots[self.start[c] as usize..self.start[c + 1] as usize];
                        if !cell.iter().all(|&i| f(i as usize)) {
                            return;
                        }
                    }
                }
            }
            _ => {
                let _ = (0..self.segs.len()).all(f);
            }
        }
    }

    /// How many edges other than `own` have a stroke running through
    /// `a` grown by [`STROKE_PAD`] — counting stops past `cap` (the
    /// caller only needs to know a spot is no better than its best).
    fn count(&self, a: Rect, own: usize, cap: usize) -> usize {
        let q = (
            a.0 - STROKE_PAD,
            a.1 - STROKE_PAD,
            a.2 + STROKE_PAD,
            a.3 + STROKE_PAD,
        );
        let stamp = self.stamp.get().wrapping_add(1);
        self.stamp.set(stamp);
        let mut seen = self.seen.borrow_mut();
        if stamp == 0 {
            seen.iter_mut().for_each(|v| *v = u32::MAX);
        }
        let mut n = 0;
        self.each_seg(q, |si| {
            let (ei, p, r) = self.segs[si];
            if ei != own
                && seen[ei] != stamp
                && boxes_touch(&self.boxes[si], &q)
                && seg_hits_rect(p, r, q)
            {
                seen[ei] = stamp;
                n += 1;
            }
            n <= cap
        });
        n
    }

    /// Squared distances from box `a` to the nearest stroke of edge
    /// `own` and to the nearest stroke of any other edge, each capped at
    /// `reach`² (nothing within `reach`). Only the strokes the grid
    /// files near the box are measured, never the whole line.
    fn dists2(&self, a: Rect, own: usize, reach: f64) -> (f64, f64) {
        let cap = reach * reach;
        let q = (a.0 - reach, a.1 - reach, a.2 + reach, a.3 + reach);
        let (mut mine, mut other) = (cap, cap);
        self.each_seg(q, |si| {
            let (ei, p, r) = self.segs[si];
            let best = if ei == own { &mut mine } else { &mut other };
            // A segment whose box lies past the best so far cannot be
            // nearer: skip the exact test.
            if rect_gap2(a, self.boxes[si]) < *best {
                *best = best.min(rect_seg_dist2(a, p, r));
            }
            true
        });
        (mine, other)
    }
}

/// Bounding box of segment `a`–`b`.
fn seg_box(a: (f64, f64), b: (f64, f64)) -> Rect {
    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

/// Squared gap between two boxes (0 when they touch or overlap).
fn rect_gap2(a: Rect, b: Rect) -> f64 {
    let dx = (b.0 - a.2).max(a.0 - b.2).max(0.0);
    let dy = (b.1 - a.3).max(a.1 - b.3).max(0.0);
    dx * dx + dy * dy
}

/// Squared distance from box `r` to segment `a`–`b` (0 when they
/// touch): squared, so the hot label search needs no square root.
fn rect_seg_dist2(r: Rect, a: (f64, f64), b: (f64, f64)) -> f64 {
    if seg_hits_rect(a, b, r) {
        return 0.0;
    }
    let to_rect = |p: (f64, f64)| {
        let dx = (r.0 - p.0).max(p.0 - r.2).max(0.0);
        let dy = (r.1 - p.1).max(p.1 - r.3).max(0.0);
        dx * dx + dy * dy
    };
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let to_seg = |p: (f64, f64)| {
        let t = if l2 > 0.0 {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (ex, ey) = (p.0 - (a.0 + t * dx), p.1 - (a.1 + t * dy));
        ex * ex + ey * ey
    };
    [(r.0, r.1), (r.2, r.1), (r.0, r.3), (r.2, r.3)]
        .into_iter()
        .map(to_seg)
        .chain([to_rect(a), to_rect(b)])
        .fold(f64::INFINITY, f64::min)
}

/// Distance from box `r` to the polyline `line`.
fn rect_line_dist(r: Rect, line: &[(f64, f64)]) -> f64 {
    line.windows(2)
        .map(|w| rect_seg_dist2(r, w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
        .sqrt()
}

/// Does segment `a`–`b` touch rect `r`? Liang-Barsky clip.
fn seg_hits_rect(a: (f64, f64), b: (f64, f64), r: Rect) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let p = [-dx, dx, -dy, dy];
    let q = [a.0 - r.0, r.2 - a.0, a.1 - r.1, r.3 - a.1];
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for i in 0..4 {
        if p[i] == 0.0 {
            if q[i] < 0.0 {
                return false;
            }
        } else {
            let v = q[i] / p[i];
            if p[i] < 0.0 {
                t0 = t0.max(v);
            } else {
                t1 = t1.min(v);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

/// Points along a drawn curve (an [`edge_polyline`]) at most
/// [`LABEL_SLIDE_STEP`] apart — or, on a curve too long for
/// [`LABEL_MAX_DENSE`] such spots, evenly coarser.
fn densify(coarse: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let Some(&first) = coarse.first() else {
        return Vec::new();
    };
    let total: f64 = coarse
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        .sum();
    if !total.is_finite() {
        return vec![first];
    }
    let step = LABEL_SLIDE_STEP.max(total / LABEL_MAX_DENSE as f64);
    let mut out = vec![first];
    for w in coarse.windows(2) {
        let k = ((w[1].0 - w[0].0).hypot(w[1].1 - w[0].1) / step).ceil();
        let k = (k as usize).max(1);
        for i in 1..=k {
            let t = i as f64 / k as f64;
            out.push((
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            ));
        }
    }
    out
}

/// Box of a `w`-wide label centred at `c`.
fn label_rect(c: (f64, f64), w: f64) -> Rect {
    (
        c.0 - w / 2.0,
        c.1 - LABEL_BOX_H / 2.0,
        c.0 + w / 2.0,
        c.1 + LABEL_BOX_H / 2.0,
    )
}

/// The title chip `to_svg` paints over cluster `ci`, for a canvas whose
/// right edge is `canvas_right` (pass `f64::INFINITY` for the widest
/// the chip can ever be: only a canvas edge cuts it shorter).
pub(crate) fn chip_rect(clusters: &[SceneCluster], ci: usize, canvas_right: f64) -> Rect {
    let c = &clusters[ci];
    let title = fit_cluster_title(clusters, ci, canvas_right);
    let (x, y) = (c.x + CHIP_INSET_X, c.y + CHIP_INSET_Y);
    (
        x,
        y,
        x + text_width(&title) * TITLE_SCALE + CHIP_PAD_W,
        y + CHIP_H + title_extra(&title),
    )
}

/// Box an arrowhead drawn at the end `tip` of a polyline (walked
/// backwards by `back`) occupies: [`ARROW_MARKER_PX`] wide, from the tip
/// back to the first sample at least that far (the far end of a shorter
/// curve), oriented on that chord, as its bounding box.
fn arrow_box(tip: (f64, f64), back: impl Iterator<Item = (f64, f64)>) -> Option<Rect> {
    // A curve shorter than a head is all head: its far end then.
    let mut from = None;
    for p in back {
        from = Some(p);
        if (p.0 - tip.0).hypot(p.1 - tip.1) >= ARROW_MARKER_PX {
            break;
        }
    }
    let from = from?;
    let len = (tip.0 - from.0).hypot(tip.1 - from.1);
    if len <= f64::EPSILON {
        return None;
    }
    let (dx, dy) = ((tip.0 - from.0) / len, (tip.1 - from.1) / len);
    let s = ARROW_MARKER_PX;
    // Back to the sample the tangent was taken from: on a coarsely
    // sampled curve that is past the head, and the yardstick (and the
    // eye) follows the drawn chord.
    let base = from;
    let (nx, ny) = (-dy * s / 2.0, dx * s / 2.0);
    let corners = [
        (tip.0 + nx, tip.1 + ny),
        (tip.0 - nx, tip.1 - ny),
        (base.0 + nx, base.1 + ny),
        (base.0 - nx, base.1 - ny),
    ];
    Some(corners.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |r, p| (r.0.min(p.0), r.1.min(p.1), r.2.max(p.0), r.3.max(p.1)),
    ))
}

/// Which labels a placement pass may move. Either way a label only ever
/// moves to a strictly better spot (see [`SpotKey`]), so a clean label
/// keeps its exact slot.
#[derive(Clone, Copy)]
enum Pass<'a> {
    /// Labels covering something hard or crowding another label, or
    /// left detached from their line, each only until it is clear: the
    /// scene builders' pass. The edge stages redraw most edges
    /// afterwards, so labels merely crossed by a stroke, and the
    /// costliest search, are left to [`relabel`].
    All,
    /// The same, plus labels crossed by another edge's stroke and every
    /// `moved` edge's label: the edge pipeline's last stage.
    Changed(&'a [bool]),
}

/// What a label spot covers, compared field by field (fewer is better):
/// hidden text (nodes, title chips, other labels) first, then
/// arrowheads, then the lesser hard hits (frame border lines, another
/// label nearer than [`LABEL_KEEP`]), then the covered area (px²). So a
/// spot on an arrowhead always loses to one that only crosses strokes,
/// and a spot hiding text to one on an arrowhead.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
struct Hits {
    text: usize,
    arrow: usize,
    minor: usize,
    area: f64,
}

impl Hits {
    /// Covering nothing at all.
    fn none(&self) -> bool {
        self.text == 0 && self.arrow == 0 && self.minor == 0 && self.area == 0.0
    }
}

/// Where a spot was found: on the middle of the line, then beside it,
/// then anywhere on the line (near its ends, or beyond a self-loop).
#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Tier {
    Line,
    Beside,
    Ends,
}

/// Ranking of one label spot, compared field by field: its [`Hits`],
/// then the fewest foreign strokes counting a lack of room
/// ([`LABEL_GAP`], [`LABEL_KEEP`]) as one more, then having room, then
/// its [`Tier`], then the closest to the preferred spot (arc-length
/// px), then curve order.
#[derive(Clone, Copy)]
struct SpotKey {
    hits: Hits,
    blocked: bool,
    strokes: usize,
    tier: Tier,
    dev: f64,
    order: usize,
}

impl SpotKey {
    /// Worse than any real spot: a detached label's old spot.
    const WORST: SpotKey = SpotKey {
        hits: Hits {
            text: usize::MAX,
            arrow: usize::MAX,
            minor: usize::MAX,
            area: f64::INFINITY,
        },
        blocked: true,
        strokes: usize::MAX,
        tier: Tier::Ends,
        dev: f64::INFINITY,
        order: usize::MAX,
    };

    /// Foreign strokes, plus one for lacking room: a spot just off a
    /// node is as bad as one crossed by an edge, and better than one
    /// crossed by two.
    fn crowding(&self) -> usize {
        self.strokes.saturating_add(usize::from(self.blocked))
    }

    fn better(&self, o: &SpotKey) -> bool {
        let key = |k: &SpotKey| {
            (
                k.hits,
                k.crowding(),
                k.blocked,
                k.strokes,
                k.tier,
                k.dev,
                k.order,
            )
        };
        key(self).partial_cmp(&key(o)) == Some(std::cmp::Ordering::Less)
    }

    /// Clear with room and crossed by no other edge: nothing can beat
    /// it but a spot nearer the preferred one.
    fn perfect(&self) -> bool {
        !self.blocked && self.strokes == 0
    }

    /// The most strokes a rival spot is worth counting: past this it
    /// can no longer win (any count matters while this one hits
    /// something).
    fn stroke_cap(&self) -> usize {
        if self.hits.none() {
            self.crowding()
        } else {
            usize::MAX
        }
    }
}

/// What a fixed obstacle is, for ranking what a label covers.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// A node or a frame title chip: it carries text.
    Text,
    /// A subgraph frame's border line.
    Border,
    /// An arrowhead: it may be touched but never covered.
    Arrow,
}

impl Kind {
    /// Room (px) an open label box keeps from an obstacle of this kind.
    fn room(self) -> f64 {
        match self {
            Kind::Arrow => ARROW_TOUCH_GAP,
            Kind::Text | Kind::Border => LABEL_GAP,
        }
    }
}

/// The fixed obstacles (nodes, frame border lines, title chips and
/// arrowheads) in one [`ObstacleGrid`], each filed under its box grown
/// by [`FIXED_ROOM`], so one walk answers every question a spot asks.
struct Fixed {
    /// The grown boxes, index-parallel with `items`.
    grid: ObstacleGrid,
    items: Vec<(Rect, Kind)>,
    /// Items looked at by queries so far (the stage's work).
    visits: Cell<usize>,
}

/// The most room a label box keeps from any fixed obstacle.
const FIXED_ROOM: f64 = if LABEL_GAP > ARROW_TOUCH_GAP {
    LABEL_GAP
} else {
    ARROW_TOUCH_GAP
};

impl Fixed {
    fn new() -> Self {
        Fixed {
            grid: ObstacleGrid::new(),
            items: Vec::new(),
            visits: Cell::new(0),
        }
    }

    fn push(&mut self, r: Rect, kind: Kind) {
        let g = FIXED_ROOM;
        self.grid.push((r.0 - g, r.1 - g, r.2 + g, r.3 + g));
        self.items.push((r, kind));
    }

    /// Calls `f` once on every item that may come within
    /// [`FIXED_ROOM`] of box `a`, until it returns false (then so does
    /// this).
    fn each(&self, a: Rect, mut f: impl FnMut(&Rect, Kind) -> bool) -> bool {
        self.grid.each(a, |i| {
            self.visits.set(self.visits.get() + 1);
            f(&self.items[i].0, self.items[i].1)
        })
    }

    /// Is box `a` clear of every item, keeping `room(kind)` from it?
    fn clear(&self, a: Rect, room: impl Fn(Kind) -> f64) -> bool {
        self.each(a, |r, k| {
            covered(a, room(k), std::slice::from_ref(r)) == 0.0
        })
    }
}

/// What one placement pass checks label spots against.
struct Field<'a> {
    fixed: Fixed,
    /// The drawn lines the stroke grid is built from on first use (the
    /// builders' pass rarely needs it).
    lines: &'a [Option<Vec<(f64, f64)>>],
    stroke_grid: OnceCell<StrokeGrid>,
    /// Work units (items looked at by the grids' queries) this pass may
    /// spend; past it, labels keep their own line's spot (see
    /// [`LABEL_ROUTE_WORK`]).
    budget: usize,
    /// Whether a clear spot crossed by strokes is worth searching on
    /// for one crossed by fewer: not yet in the builders' pass (see
    /// [`Pass::All`]), which only frees labels from what they cover.
    strokes_matter: bool,
}

impl Field<'_> {
    fn strokes(&self) -> &StrokeGrid {
        self.stroke_grid.get_or_init(|| StrokeGrid::new(self.lines))
    }

    /// Has the pass spent its work budget? Counted as the items the
    /// fixed obstacles, the stroke grid and the label grid `placed`
    /// looked at, so the cost tracks time however crowded the drawing
    /// (parallel labelled edges pile every stroke and label into the
    /// same cells).
    fn spent(&self, placed: &LabelGrid) -> bool {
        let strokes = self.stroke_grid.get().map_or(0, |g| g.visits.get());
        self.fixed.visits.get() + strokes + placed.visits.get() >= self.budget
    }

    /// Does box `r` still read as label `ei`'s: within the drift limit
    /// of its own line and no other line nearer?
    fn reads_own(&self, r: Rect, ei: usize) -> bool {
        let (own2, other2) = self.strokes().dists2(r, ei, LABEL_DRIFT_MAX);
        own2 < LABEL_DRIFT_MAX * LABEL_DRIFT_MAX && other2 >= own2
    }

    /// Nothing could beat `k` but a spot nearer the preferred one.
    fn settled(&self, k: &SpotKey) -> bool {
        if self.strokes_matter {
            k.perfect()
        } else {
            !k.blocked
        }
    }

    /// Clear of nodes, borders, chips (with [`LABEL_GAP`]) and
    /// arrowheads (touching allowed): the part of "clear" that does not
    /// depend on where the other labels are.
    fn open(&self, r: Rect) -> bool {
        self.fixed.clear(r, Kind::room)
    }

    /// Covering nothing fixed (an arrowhead not even touched).
    fn untouched(&self, r: Rect) -> bool {
        self.fixed.clear(r, |k| {
            if k == Kind::Arrow {
                ARROW_TOUCH_GAP
            } else {
                0.0
            }
        })
    }

    /// Clear of everything hard, other labels included.
    fn clear(&self, placed: &LabelGrid, ei: usize, r: Rect) -> bool {
        self.open(r) && placed.clear(ei, r, LABEL_KEEP)
    }

    /// How box `r` of label `ei` (`open` of the fixed obstacles) ranks:
    /// what it covers and whether it lacks room — or None when it hits
    /// something while `best` hits nothing, so it cannot win.
    fn rank(
        &self,
        placed: &LabelGrid,
        ei: usize,
        r: Rect,
        open: bool,
        best: &SpotKey,
    ) -> Option<(Hits, bool)> {
        // Lacking room counts as a stroke (see `SpotKey::crowding`), so
        // such a spot cannot beat a clean one crossing at most one.
        let hopeless = best.hits.none() && best.crowding() <= 1;
        if !open && hopeless {
            return None;
        }
        let free = placed.clear(ei, r, LABEL_KEEP);
        if open && free {
            return Some((Hits::default(), false));
        }
        if hopeless {
            return None;
        }
        // Merely near a node, border or chip: it covers nothing.
        if free && self.untouched(r) {
            return Some((Hits::default(), true));
        }
        if best.hits.none() {
            return None;
        }
        Some((self.hits_within(placed, ei, r, &best.hits)?, true))
    }

    /// [`Field::hits`], or None as soon as it is plainly worse than
    /// `cap` (more text hidden, or as much and more arrowheads).
    fn hits_within(&self, placed: &LabelGrid, ei: usize, r: Rect, cap: &Hits) -> Option<Hits> {
        let labels = placed.hits(ei, r, 0.0);
        let mut h = Hits {
            text: labels,
            ..Hits::default()
        };
        self.fixed.each(r, |o, k| {
            let o = std::slice::from_ref(o);
            let area = covered(r, 0.0, o);
            h.area += area;
            match k {
                Kind::Text => h.text += usize::from(area > 0.0),
                Kind::Border => h.minor += usize::from(area > 0.0),
                Kind::Arrow => h.arrow += usize::from(covered(r, ARROW_TOUCH_GAP, o) > 0.0),
            }
            true
        });
        if (h.text, h.arrow) > (cap.text, cap.arrow) {
            return None;
        }
        h.minor += placed.hits(ei, r, LABEL_KEEP) - labels;
        h.area += placed.cover(ei, r, 0.0);
        Some(h)
    }

    /// What box `r` of label `ei` covers.
    fn hits(&self, placed: &LabelGrid, ei: usize, r: Rect) -> Hits {
        self.hits_within(placed, ei, r, &SpotKey::WORST.hits)
            .unwrap_or(SpotKey::WORST.hits)
    }
}

/// One label's spots along its drawn curve, kept across rounds.
struct Track {
    /// The densified curve.
    spots: Vec<(f64, f64)>,
    /// Arc length (px) of each spot, and that of the preferred spot.
    arc: Vec<f64>,
    mid: f64,
    /// The first scan: spots of the middle stretch (all but
    /// [`LABEL_SCAN_END`] at each end), nearest the preferred spot first,
    /// as (deviation, index into `spots`).
    scan: Vec<(f64, usize)>,
    /// Per `scan` spot, whether it lies between two of every
    /// [`LABEL_STROKE_STRIDE`]th spot along the curve: skipped while
    /// only fewer strokes are sought.
    between: Vec<bool>,
    /// Per `scan` spot, whether it is open of the fixed obstacles, once
    /// asked: [`OPEN_UNKNOWN`], [`OPEN_YES`] or [`OPEN_NO`].
    open: Vec<Cell<u8>>,
    /// How many of every [`LABEL_FREE_STRIDE`]th `scan` spot are open:
    /// the fewer, the more constrained the label, and the earlier it is
    /// placed.
    free: usize,
    /// Every box the label may try lies inside this one: another label
    /// moving outside it cannot change the label's best spot.
    reach: Rect,
}

impl Track {
    fn new(pts: &[(f64, f64)], prefer: f64, w: f64, field: &Field) -> Self {
        let spots = densify(pts);
        let mut arc = Vec::with_capacity(spots.len());
        let mut s = 0.0;
        for (k, p) in spots.iter().enumerate() {
            if k > 0 {
                s += (p.0 - spots[k - 1].0).hypot(p.1 - spots[k - 1].1);
            }
            arc.push(s);
        }
        let (lo, hi) = (s * LABEL_SCAN_END, s * (1.0 - LABEL_SCAN_END));
        let mid = s * prefer;
        let mut inner: Vec<usize> = (0..spots.len())
            .filter(|&k| arc[k] >= lo && arc[k] <= hi)
            .collect();
        if inner.is_empty() {
            inner = (0..spots.len()).collect();
        }
        // A long curve is scanned at a coarser stride, so a label costs
        // at most LABEL_MAX_SPOTS spots (route() runs on every drag).
        let stride = inner.len().div_ceil(LABEL_MAX_SPOTS).max(1);
        let mut scan: Vec<(f64, usize, bool)> = inner
            .into_iter()
            .step_by(stride)
            .enumerate()
            .map(|(j, k)| ((arc[k] - mid).abs(), k, j % LABEL_STROKE_STRIDE != 0))
            .collect();
        scan.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let between = scan.iter().map(|s| s.2).collect();
        let scan = scan.into_iter().map(|s| (s.0, s.1)).collect::<Vec<_>>();
        let open = vec![Cell::new(OPEN_UNKNOWN); scan.len()];
        let mut bb = Bbox::new();
        for p in &spots {
            bb.add(p.0, p.1);
        }
        let (x0, x1, y0, y1) = bb.finish();
        // Half the box, the farthest step beside the line, and the
        // keep-out it is checked with, around the whole curve (a
        // self-loop's spots past its apex stay within a box height).
        let gx = w / 2.0 + LABEL_SIDE_MAX + LABEL_KEEP + LABEL_BOX_H;
        let gy = LABEL_BOX_H / 2.0 + LABEL_SIDE_MAX + LABEL_KEEP + w;
        let reach = (x0 - gx, y0 - gy, x1 + gx, y1 + gy);
        let mut t = Track {
            spots,
            arc,
            mid,
            scan,
            between,
            open,
            free: 0,
            reach,
        };
        t.free = (0..t.scan.len())
            .step_by(LABEL_FREE_STRIDE)
            .filter(|&i| t.is_open(i, w, field))
            .count();
        t
    }

    /// The spot of the scan nearest the preferred one, on the line: where
    /// a label goes when the stage has no work left to search.
    fn own_spot(&self) -> (f64, f64) {
        self.scan
            .first()
            .map_or(self.spots[0], |&(_, k)| self.spots[k])
    }

    /// Share of the curve's length (from its start) at which its spot
    /// nearest `p` lies.
    fn share_at(&self, p: (f64, f64)) -> f64 {
        let total = self.arc.last().copied().unwrap_or(0.0);
        if total <= f64::EPSILON {
            return LABEL_PREFER;
        }
        let d2 = |k: usize| (self.spots[k].0 - p.0).powi(2) + (self.spots[k].1 - p.1).powi(2);
        let k = (0..self.spots.len())
            .min_by(|&a, &b| d2(a).total_cmp(&d2(b)))
            .unwrap_or(0);
        self.arc[k] / total
    }

    /// Is `scan` spot `i` open of the fixed obstacles (asked once)?
    fn is_open(&self, i: usize, w: f64, field: &Field) -> bool {
        let c = &self.open[i];
        if c.get() == OPEN_UNKNOWN {
            let r = label_rect(self.spots[self.scan[i].1], w);
            c.set(if field.open(r) { OPEN_YES } else { OPEN_NO });
        }
        c.get() == OPEN_YES
    }

    /// [`LABEL_FINE_SPOTS`] spots evenly along the whole curve, nearest
    /// the preferred spot first, as (deviation, index into `spots`): the
    /// fine sweep beside the line also tries its ends.
    fn whole(&self) -> Vec<(f64, usize)> {
        let stride = self.spots.len().div_ceil(LABEL_FINE_SPOTS).max(1);
        let mut whole: Vec<(f64, usize)> = (0..self.spots.len())
            .step_by(stride)
            .map(|k| ((self.arc[k] - self.mid).abs(), k))
            .collect();
        whole.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        whole
    }
}

/// While a label is clear and only fewer strokes are sought, the scan
/// tries every this many-th spot along its curve.
const LABEL_STROKE_STRIDE: usize = 2;
/// [`Track::open`] states.
const OPEN_UNKNOWN: u8 = 0;
const OPEN_YES: u8 = 1;
const OPEN_NO: u8 = 2;
/// Every how many-th scan spot the constraint estimate asks: enough
/// to order the labels, a quarter of the cost.
const LABEL_FREE_STRIDE: usize = 4;

/// Slide every edge label that covers a node box, a subgraph frame's
/// border line, a frame title chip, an arrowhead or another label (or
/// crowds one) to the best spot (see
/// [`SpotKey`]) on its OWN drawn edge — or, failing that, just beside
/// it (see [`beside_line`]). A label only moves to a strictly better
/// spot, so clean labels keep their exact spot and only colliding
/// diagrams change. Returns the indices of the edges whose label moved.
pub(crate) fn avoid_label_overlaps(
    nodes: &[SceneNode],
    frames: &[SceneCluster],
    edges: &mut [SceneEdge],
    budget: usize,
) -> Vec<usize> {
    place(nodes, frames, edges, Pass::All, budget)
}

/// The edge pipeline's label stage: re-place the labels of the edges a
/// port or routing stage changed (`moved`), and any label that covers
/// a hard obstacle or is crossed by another edge's stroke, now that
/// every edge is drawn for good. The canvas is refitted by the
/// pipeline's next stage.
pub(crate) fn relabel(_g: &Graph, sc: &mut Scene, _cx: &Ctx, moved: &[bool], budget: usize) {
    place(
        &sc.nodes,
        &sc.clusters,
        &mut sc.edges,
        Pass::Changed(moved),
        budget,
    );
}

fn place(
    nodes: &[SceneNode],
    frames: &[SceneCluster],
    edges: &mut [SceneEdge],
    pass: Pass,
    budget: usize,
) -> Vec<usize> {
    if edges.iter().all(|e| e.label.is_none()) {
        return Vec::new();
    }
    let lines: Vec<Option<Vec<(f64, f64)>>> = edges
        .iter()
        .map(|e| {
            (edge_drawn(e) && !matches!(e.kind, EdgeKind::Invisible)).then(|| edge_polyline(e))
        })
        .collect();

    let mut fixed = Fixed::new();
    for n in nodes {
        let r = (
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.x + n.w / 2.0,
            n.y + n.h / 2.0,
        );
        fixed.push(r, Kind::Text);
    }
    // A frame is only its outline: a label inside it is fine, one
    // straddling its border hides the line (and blurs which box the
    // label belongs to).
    let b = FRAME_BORDER_BAND;
    for f in frames {
        let (x0, y0, x1, y1) = (f.x, f.y, f.x + f.w, f.y + f.h);
        for r in [
            (x0 - b, y0 - b, x1 + b, y0 + b),
            (x0 - b, y1 - b, x1 + b, y1 + b),
            (x0 - b, y0 - b, x0 + b, y1 + b),
            (x1 - b, y0 - b, x1 + b, y1 + b),
        ] {
            fixed.push(r, Kind::Border);
        }
    }
    // Title chips at their widest: the final canvas can only cut one
    // shorter.
    for ci in 0..frames.len() {
        fixed.push(chip_rect(frames, ci, f64::INFINITY), Kind::Text);
    }
    // Arrowheads may be touched but never covered: a label kept off
    // one by LABEL_GAP no longer fits the gap the layout sized for it.
    for (e, pts) in edges.iter().zip(&lines) {
        let Some(pts) = pts else { continue };
        let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
            continue;
        };
        if e.ends.end_marker(e.kind).is_some() {
            if let Some(r) = arrow_box(last, pts.iter().rev().copied()) {
                fixed.push(r, Kind::Arrow);
            }
        }
        if e.ends.start_marker(e.kind).is_some() {
            if let Some(r) = arrow_box(first, pts.iter().copied()) {
                fixed.push(r, Kind::Arrow);
            }
        }
    }
    let field = Field {
        fixed,
        lines: &lines,
        stroke_grid: OnceCell::new(),
        budget,
        strokes_matter: !matches!(pass, Pass::All),
    };

    // Fan-outs: how many drawn edges end on each endpoint.
    let mut degree: BTreeMap<&str, usize> = BTreeMap::new();
    for (e, l) in edges.iter().zip(&lines) {
        if l.is_some() && e.from != e.to {
            *degree.entry(e.from.as_str()).or_default() += 1;
            *degree.entry(e.to.as_str()).or_default() += 1;
        }
    }
    // A compact branch point's fan (a state `<<choice>>`; an edge with
    // exactly one end such a point, as `layout` sizes their gaps): its
    // edges leave the small diamond within a few px of each other, so in
    // the near half of their lines a label sits between, or on, its
    // siblings' strokes and reads as theirs. Its labels belong to the
    // far part, where the branches have spread apart (see `FAN_FAR`).
    // Per edge: `Some(true)` when its source is the point, `Some(false)`
    // when its target is.
    let branch: BTreeSet<&str> = nodes
        .iter()
        .filter(|n| is_branch_point(n.shape, &n.label))
        .map(|n| n.id.as_str())
        .collect();
    let fan: Vec<Option<bool>> = edges
        .iter()
        .map(|e| {
            if !e.waypoints.is_empty() {
                // A long edge's label keeps the slot its layout dummy
                // reserved mid-channel.
                return None;
            }
            match (
                branch.contains(e.from.as_str()),
                branch.contains(e.to.as_str()),
            ) {
                (true, false) => Some(true),
                (false, true) => Some(false),
                _ => None,
            }
        })
        .collect();
    // (preferred spot, fan-out of a hub end or 0) per edge.
    let hub: Vec<(f64, usize)> = edges
        .iter()
        .zip(&fan)
        .map(|(e, &fan)| {
            let df = degree.get(e.from.as_str()).copied().unwrap_or(0);
            let dt = degree.get(e.to.as_str()).copied().unwrap_or(0);
            // A hub end piles every label of its fan near it: prefer
            // the middle of the half away from the busier end — away
            // from a compact branch point whatever the counts.
            if fan == Some(true) || fan.is_none() && e.from != e.to && df > 1 && df > dt {
                (LABEL_HUB_PREFER, df)
            } else if fan == Some(false) || fan.is_none() && e.from != e.to && dt > 1 && dt > df {
                (1.0 - LABEL_HUB_PREFER, dt)
            } else {
                (LABEL_PREFER, 0)
            }
        })
        .collect();

    let n = edges.len();
    let mut tracks: Vec<Option<Track>> = (0..n).map(|_| None).collect();
    // A fan label left in the near half of its line (the builder's
    // midpoint slot, say) is stacked into the far part first; one that
    // finds no spot there is re-placed below like a detached one.
    let near_hub = |tracks: &mut [Option<Track>], ei: usize, at: Pt, w: f64| {
        let (Some(from_point), Some(pts)) = (fan[ei], &lines[ei]) else {
            return false;
        };
        let track = tracks[ei].get_or_insert_with(|| Track::new(pts, hub[ei].0, w, &field));
        let f = track.share_at(at);
        (if from_point { f } else { 1.0 - f }) < FAN_FAR
    };
    // So is one a stage left off its line (the port stage moves a
    // single curve's label by the mean of its end moves: right for the
    // midpoint only) and, in the final pass, one another stroke now
    // runs through. A fan is stacked as a whole: its rows are one
    // packing.
    let mut restack: BTreeSet<String> = BTreeSet::new();
    for ei in 0..n {
        let (Some((_, at, w)), Some(from_point), Some(pts)) =
            (&edges[ei].label, fan[ei], &lines[ei])
        else {
            continue;
        };
        let point = if from_point {
            &edges[ei].from
        } else {
            &edges[ei].to
        };
        if restack.contains(point) {
            continue;
        }
        let r = label_rect(*at, *w);
        if rect_line_dist((at.0, at.1, at.0, at.1), pts) > FAN_ON_LINE
            || field.strokes_matter && field.strokes().count(r, ei, 0) > 0
            || near_hub(&mut tracks, ei, *at, *w)
        {
            restack.insert(point.clone());
        }
    }
    let lift: Vec<bool> = (0..n)
        .map(|ei| {
            let (Some(from_point), Some(_), Some(pts)) = (fan[ei], &edges[ei].label, &lines[ei])
            else {
                return false;
            };
            let e = &edges[ei];
            let up = restack.contains(if from_point { &e.from } else { &e.to });
            if up {
                let w = e.label.as_ref().map_or(0.0, |l| l.2);
                tracks[ei].get_or_insert_with(|| Track::new(pts, hub[ei].0, w, &field));
            }
            up
        })
        .collect();
    let mut moved = stack_fans(&field, nodes, edges, &lines, &fan, &lift, &tracks);
    let mut stacked = vec![false; n];
    for &ei in &moved {
        stacked[ei] = true;
    }

    // Every label is an obstacle at its current spot from the start, so
    // an early label never slides onto a later one's slot.
    let mut placed = LabelGrid::new(
        edges
            .iter()
            .zip(&lines)
            .map(|(e, l)| {
                l.as_ref()?;
                e.label.as_ref().map(|(_, c, w)| label_rect(*c, *w))
            })
            .collect(),
    );

    // Labels already searched, and the boxes labels left or took in the
    // last round (by mover): a searched label is tried again only when
    // one of those lies within its reach.
    let mut tried = vec![false; n];
    let mut changes: Vec<(usize, Rect)> = Vec::new();
    for round in 0..LABEL_ROUNDS {
        // The labels this round may move, with whether their old spot
        // left their line.
        let mut todo: Vec<(usize, bool)> = Vec::new();
        for ei in 0..n {
            let Some(pts) = &lines[ei] else { continue };
            let Some((_, at, w)) = &edges[ei].label else {
                continue;
            };
            let r = label_rect(*at, *w);
            // After the first round only the labels moved since can
            // change anything: a label searched already is tried again
            // only when one moved within its reach, any other only when
            // one moved within its keep-out.
            if round > 0 {
                let reach = match &tracks[ei] {
                    Some(t) if tried[ei] => t.reach,
                    _ => (
                        r.0 - LABEL_KEEP,
                        r.1 - LABEL_KEEP,
                        r.2 + LABEL_KEEP,
                        r.3 + LABEL_KEEP,
                    ),
                };
                if !changes
                    .iter()
                    .any(|(j, c)| *j != ei && boxes_touch(c, &reach))
                {
                    continue;
                }
            }
            let changed = round == 0
                && match pass {
                    Pass::All => false,
                    Pass::Changed(m) => m.get(ei).copied().unwrap_or(false),
                };
            // A label left farther than LABEL_DRIFT_MAX from its own
            // line no longer reads as its label: it is re-placed like a
            // covering one (a self-loop's may sit beyond its loop).
            let own = rect_line_dist(r, pts);
            let self_loop = edges[ei].from == edges[ei].to;
            let detached = own > LABEL_DRIFT_MAX && !self_loop;
            // A fan label of a compact branch point left in the near
            // half of its line is re-placed like a detached one, from
            // its preferred spot in the far half.
            let near_hub = !stacked[ei] && near_hub(&mut tracks, ei, *at, *w);
            // A changed edge's old spot competes as is while it still
            // lies on the new line; off it (or detached), it stays only
            // when no spot is any better. A fan label just stacked beside
            // its line (no row on it was free) was set there on purpose.
            let off_line = detached || near_hub || (changed && own > 0.0 && !stacked[ei]);
            // A clean label on its line is perfect where it is. The
            // builders' pass does not mind strokes (see `Pass::All`).
            let clean = match pass {
                Pass::All => field.hits(&placed, ei, r).none(),
                Pass::Changed(_) => {
                    field.strokes().count(r, ei, 0) == 0
                        && field.untouched(r)
                        && placed.clear(ei, r, LABEL_KEEP)
                }
            };
            if clean && !off_line {
                continue;
            }
            if tracks[ei].is_none() {
                tracks[ei] = Some(Track::new(pts, hub[ei].0, *w, &field));
            }
            todo.push((ei, off_line));
        }
        changes.clear();
        if todo.is_empty() {
            break;
        }
        // Most constrained first: the fewest open spots, then hub edges
        // (busiest first), ties by edge index.
        todo.sort_by_key(|&(ei, _)| {
            let free = tracks[ei].as_ref().map_or(0, |t| t.free);
            (free, std::cmp::Reverse(hub[ei].1), ei)
        });
        let mut any = false;
        for (ei, off_line) in todo {
            let Some(track) = &tracks[ei] else {
                continue;
            };
            let Some((_, at, w)) = edges[ei].label.clone() else {
                continue;
            };
            // Out of work: a label off its line goes back onto it at the
            // preferred spot; any other stays where it is.
            let to = if field.spent(&placed) {
                if off_line {
                    track.own_spot()
                } else {
                    at
                }
            } else {
                best_spot(
                    &field, &placed, &edges[ei], nodes, ei, at, w, track, off_line,
                )
            };
            tried[ei] = true;
            if to != at {
                if let Some((_, m, _)) = &mut edges[ei].label {
                    *m = to;
                }
                changes.push((ei, label_rect(at, w)));
                changes.push((ei, label_rect(to, w)));
                placed.set(ei, label_rect(to, w));
                moved.push(ei);
                any = true;
            }
        }
        // Nothing moved: a further round would find the same spots.
        if !any {
            break;
        }
    }
    moved.sort_unstable();
    moved.dedup();
    moved
}

/// Part of a fan line walked from its far end: (spot, depth back along
/// the flow).
type Walk = Vec<(Pt, f64)>;

/// Stacks the labels of the compact branch points' fans flagged in
/// `lift` (a fan is flagged whole) in rows across the far half of their
/// lines, the way the layout packs them: it sizes the gap for rows past
/// [`FAN_FAR`] (see `layout::fan_rows_gap`), and the rest of the half
/// absorbs its estimate's error (a label stacked short of FAN_FAR is
/// stacked afresh by each later pass, to the same spot while nothing
/// near it moved). Row 0 lies as near the far ends as every label of
/// the point can sit (clear of the arrowheads), each further row a
/// label box plus the layout's row room (`LABEL_CLEAR`) back along the
/// flow. A label's candidates are the rows where its spot on its own
/// line is open of the fixed obstacles, clear of every other label and
/// crossed by no other stroke; [`fan_rows`] picks one per label, in
/// reading order across the flow, siblings side by side
/// [`FAN_SIBLING_GAP`] apart. A label with no pick keeps its spot (the
/// caller re-places it). `tracks` holds every flagged label's track.
/// Returns the edges whose label moved.
fn stack_fans(
    field: &Field,
    nodes: &[SceneNode],
    edges: &mut [SceneEdge],
    lines: &[Option<Vec<Pt>>],
    fan: &[Option<bool>],
    lift: &[bool],
    tracks: &[Option<Track>],
) -> Vec<usize> {
    let mut moved = Vec::new();
    if !lift.contains(&true) {
        return moved;
    }
    // Every other label is an obstacle where it is.
    let mut placed = LabelGrid::new(
        edges
            .iter()
            .zip(lines)
            .zip(lift)
            .map(|((e, l), &up)| {
                l.as_ref().filter(|_| !up)?;
                e.label.as_ref().map(|(_, c, w)| label_rect(*c, *w))
            })
            .collect(),
    );
    // Every labelled fan line per branch point, with its far end.
    let mut groups: BTreeMap<String, Vec<(usize, Pt)>> = BTreeMap::new();
    for (ei, e) in edges.iter().enumerate() {
        let (Some(from_point), Some(pts), Some(_)) = (fan[ei], &lines[ei], &e.label) else {
            continue;
        };
        let (point, far) = if from_point {
            (&e.from, pts.last())
        } else {
            (&e.to, pts.first())
        };
        if let Some(&far) = far {
            groups.entry(point.clone()).or_default().push((ei, far));
        }
    }
    for (point, mut group) in groups {
        // A fan is stacked as a whole (the caller flags it so).
        if !group.iter().any(|&(ei, _)| lift[ei]) {
            continue;
        }
        // The flow: the axis nearest the way from the point's centre to
        // the middle of the far ends (down when they coincide).
        let k = group.len() as f64;
        let mid = group
            .iter()
            .fold((0.0, 0.0), |m, &(_, f)| (m.0 + f.0 / k, m.1 + f.1 / k));
        let c = nodes
            .iter()
            .find(|n| n.id == point)
            .map_or(mid, |n| (n.x, n.y));
        let (dx, dy) = (mid.0 - c.0, mid.1 - c.1);
        let (ux, uy): (f64, f64) = if dx.abs() > dy.abs() {
            (dx.signum(), 0.0)
        } else if dy < 0.0 {
            (0.0, -1.0)
        } else {
            (0.0, 1.0)
        };
        // Reading order across it: left to right in TB/BT, top to
        // bottom in LR/RL.
        let across = |p: Pt| p.0 * uy.abs() + p.1 * ux.abs();
        group.sort_by(|a, b| across(a.1).total_cmp(&across(b.1)).then(a.0.cmp(&b.0)));
        // Per label: the far half of its line walked from its far end,
        // as (spot, depth back along the flow from the farthest end, so
        // rows are level across the fan even when its far ends are not).
        let reach = group
            .iter()
            .map(|&(_, f)| f.0 * ux + f.1 * uy)
            .fold(f64::NEG_INFINITY, f64::max);
        let mut walks: Vec<(usize, f64, Walk)> = Vec::new();
        for &(ei, _) in &group {
            let (Some(track), Some(from_point), Some((_, _, w))) =
                (&tracks[ei], fan[ei], &edges[ei].label)
            else {
                continue;
            };
            let total = track.arc.last().copied().unwrap_or(0.0);
            if total <= f64::EPSILON {
                continue;
            }
            let share = |k: usize| {
                let f = track.arc[k] / total;
                if from_point {
                    f
                } else {
                    1.0 - f
                }
            };
            let ks: Vec<usize> = if from_point {
                (0..track.spots.len()).rev().collect()
            } else {
                (0..track.spots.len()).collect()
            };
            let walk = ks
                .into_iter()
                .take_while(|&k| share(k) >= LABEL_PREFER)
                .map(|k| {
                    let p = track.spots[k];
                    (p, reach - (p.0 * ux + p.1 * uy))
                })
                .collect();
            walks.push((ei, *w, walk));
        }
        // Row 0: the deepest spot open on every line of the fan; rows
        // one box (the deepest any label reaches along the flow) apart,
        // deeper ones too where a line reaches past row 0 (its far end
        // dragged away, say).
        let open_at = |w: f64, walk: &[(Pt, f64)]| {
            walk.iter()
                .find(|(p, _)| field.open(label_rect(*p, w)))
                .map(|&(_, d)| d)
        };
        let opens: Vec<Option<f64>> = walks.iter().map(|(_, w, walk)| open_at(*w, walk)).collect();
        let Some(row0) = opens
            .iter()
            .try_fold(f64::NEG_INFINITY, |m, d| d.map(|d| m.max(d)))
        else {
            continue;
        };
        let pitch = walks
            .iter()
            .map(|&(_, w, _)| w * ux.abs() + LABEL_BOX_H * uy.abs())
            .fold(0.0, f64::max)
            + LABEL_CLEAR;
        // Each label's candidate spots: the rows its line reaches,
        // nearest its far end first, that are open of the fixed
        // obstacles, clear of every other label (its siblings aside) and
        // crossed by no other stroke.
        let free = |ei: usize, r: Rect| {
            field.open(r) && placed.clear(ei, r, LABEL_KEEP) && field.strokes().count(r, ei, 0) == 0
        };
        // Per label: the spots of the rows its line reaches.
        let rows: Vec<(usize, f64, Vec<Pt>)> = walks
            .iter()
            .zip(&opens)
            .map(|((ei, w, walk), open)| {
                let first = -((row0 - open.unwrap_or(row0)) / pitch).floor() as i64;
                // The spot at `depth` on the walk, between its samples.
                let at_depth = |depth: f64| {
                    walk.windows(2).find_map(|s| {
                        let ((p, dp), (q, dq)) = (s[0], s[1]);
                        (dp <= depth && depth <= dq && dq > dp).then(|| {
                            let t = (depth - dp) / (dq - dp);
                            (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t)
                        })
                    })
                };
                let spots = (first..)
                    .map(|r| at_depth(row0 + r as f64 * pitch))
                    .take_while(Option::is_some)
                    .flatten()
                    .collect();
                (*ei, *w, spots)
            })
            .collect();
        // A label's spots just beside its line in those rows, away from
        // the nearer sibling first: LABEL_SIDE_AIR off, else half off
        // (covering its own stroke only), while no other line is nearer.
        let beside = |&(ei, w, ref rows): &(usize, f64, Vec<Pt>)| -> Vec<Pt> {
            let mine = group
                .iter()
                .find(|g| g.0 == ei)
                .map_or(0.0, |g| across(g.1));
            let toward = group
                .iter()
                .filter(|g| g.0 != ei)
                .map(|g| across(g.1) - mine)
                .min_by(|a, b| a.abs().total_cmp(&b.abs()))
                .unwrap_or(0.0);
            let side = if toward > 0.0 { -1.0 } else { 1.0 };
            let half = (w * uy.abs() + LABEL_BOX_H * ux.abs()) / 2.0;
            let offs = [half + LABEL_SIDE_AIR, half - LABEL_STRADDLE];
            rows.iter()
                .flat_map(|&p| {
                    [side, -side].into_iter().flat_map(move |s| {
                        offs.map(|o| (p.0 + s * o * uy.abs(), p.1 + s * o * ux.abs()))
                    })
                })
                .filter(|&q| {
                    let r = label_rect(q, w);
                    free(ei, r) && field.reads_own(r, ei)
                })
                .collect()
        };
        // Each label's candidates: the rows where its spot on its line
        // is open of the fixed obstacles, clear of every other label
        // (its siblings aside) and crossed by no other stroke. With none
        // (a sibling runs within a box of it all the way: two
        // transitions to one state, their lines PARALLEL_GAP apart), its
        // spots beside it.
        let mut lined = Vec::with_capacity(rows.len());
        let mut cands: Vec<(usize, f64, Vec<Pt>)> = rows
            .iter()
            .map(|row @ &(ei, w, ref spots)| {
                let on: Vec<Pt> = spots
                    .iter()
                    .copied()
                    .filter(|&p| free(ei, label_rect(p, w)))
                    .collect();
                lined.push(!on.is_empty());
                (ei, w, if on.is_empty() { beside(row) } else { on })
            })
            .collect();
        if field.spent(&placed) {
            return moved;
        }
        // Siblings side by side keep FAN_SIBLING_GAP across the flow
        // (LABEL_KEEP along it, between rows).
        let (gx, gy) = (
            FAN_SIBLING_GAP * uy.abs() + LABEL_KEEP * ux.abs(),
            FAN_SIBLING_GAP * ux.abs() + LABEL_KEEP * uy.abs(),
        );
        let apart =
            |a: Rect, b: Rect| covered((a.0 - gx, a.1 - gy, a.2 + gx, a.3 + gy), 0.0, &[b]) == 0.0;
        let picks = match fan_rows(&cands, &apart) {
            Some(picks) => picks,
            // No assignment keeps every label on its line: the rows are
            // too few for them (the point dragged toward its far ends,
            // say). Then any label may sit beside its line as well, after
            // its spots on it.
            None => {
                for ((c, r), &lined) in cands.iter_mut().zip(&rows).zip(&lined) {
                    if lined {
                        c.2.extend(beside(r));
                    }
                }
                fan_rows(&cands, &apart).unwrap_or_else(|| fan_first_fits(&cands, &apart))
            }
        };
        for ((ei, w, _), pick) in cands.iter().zip(picks) {
            let Some(p) = pick else { continue };
            if let Some((_, m, _)) = &mut edges[*ei].label {
                *m = p;
            }
            placed.set(*ei, label_rect(p, *w));
            moved.push(*ei);
        }
    }
    moved
}

/// Most candidate spots [`fan_rows`] tries before it settles for first
/// fits: a fan has a handful of labels and rows, so a real one needs a
/// few dozen at most.
const FAN_SEARCH: usize = 4096;

/// One spot per fan label from its candidates `cands` (edge, width,
/// spots in preference order), siblings pairwise `apart`: the first such
/// assignment in order (each label's spots tried in turn, depth first),
/// a label without candidates left out (`None` = keep its spot). `None`
/// with no full assignment, or past [`FAN_SEARCH`] steps.
fn fan_rows(
    cands: &[(usize, f64, Vec<Pt>)],
    apart: &dyn Fn(Rect, Rect) -> bool,
) -> Option<Vec<Option<Pt>>> {
    fn search(
        cands: &[(usize, f64, Vec<Pt>)],
        apart: &dyn Fn(Rect, Rect) -> bool,
        picks: &mut Vec<Option<Pt>>,
        steps: &mut usize,
    ) -> bool {
        let Some((_, w, spots)) = cands.get(picks.len()) else {
            return true;
        };
        if spots.is_empty() {
            picks.push(None);
            if search(cands, apart, picks, steps) {
                return true;
            }
            picks.pop();
            return false;
        }
        for &p in spots {
            *steps += 1;
            if *steps > FAN_SEARCH {
                return false;
            }
            let r = label_rect(p, *w);
            let free = picks
                .iter()
                .zip(cands)
                .all(|(q, c)| q.map_or(true, |q| apart(r, label_rect(q, c.1))));
            if free {
                picks.push(Some(p));
                if search(cands, apart, picks, steps) {
                    return true;
                }
                picks.pop();
            }
        }
        false
    }
    let mut picks = Vec::with_capacity(cands.len());
    search(cands, apart, &mut picks, &mut 0).then_some(picks)
}

/// [`fan_rows`]' fallback: each label takes its first spot apart from
/// those taken before it (`None` = keep its spot).
fn fan_first_fits(
    cands: &[(usize, f64, Vec<Pt>)],
    apart: &dyn Fn(Rect, Rect) -> bool,
) -> Vec<Option<Pt>> {
    let mut taken: Vec<Rect> = Vec::new();
    cands
        .iter()
        .map(|(_, w, spots)| {
            let p = spots.iter().copied().find(|&p| {
                let r = label_rect(p, *w);
                taken.iter().all(|&t| apart(r, t))
            })?;
            taken.push(label_rect(p, *w));
            Some(p)
        })
        .collect()
}

/// The best spot for label `ei` (`w` wide, now at `at`) of edge `e`,
/// its drawn line densified and scanned as `track`. In order: the
/// middle stretch of the line; just beside it; the nearest spot clear of
/// everything anywhere on it; and only then the spot covering least —
/// never a spot nearer another edge than its own, where it would read
/// as that edge's label. The old spot competes as is unless `off_line`.
#[allow(clippy::too_many_arguments)]
fn best_spot(
    field: &Field,
    placed: &LabelGrid,
    e: &SceneEdge,
    nodes: &[SceneNode],
    ei: usize,
    at: (f64, f64),
    w: f64,
    track: &Track,
    off_line: bool,
) -> (f64, f64) {
    let mut best = (at, SpotKey::WORST);
    if !off_line {
        // The old spot keeps its slot unless it hits something: being
        // merely close to a node is how the layout left it.
        let r = label_rect(at, w);
        let hits = field.hits(placed, ei, r);
        best.1 = SpotKey {
            hits,
            blocked: !hits.none(),
            strokes: field.strokes().count(r, ei, usize::MAX),
            tier: Tier::Line,
            dev: 0.0,
            order: 0,
        };
    }
    // 1. The middle stretch, nearest the preferred spot first, so the
    // scan stops at the first perfect spot: no later one can beat it.
    for (i, &(dev, k)) in track.scan.iter().enumerate() {
        if field.settled(&best.1) && dev >= best.1.dev || field.spent(placed) {
            break;
        }
        // Clear already, it only seeks fewer strokes: a stroke crosses
        // a box over many neighbouring spots, so a coarser step finds a
        // way off it as well.
        if !best.1.blocked && track.between[i] {
            continue;
        }
        let p = track.spots[k];
        let r = label_rect(p, w);
        let open = track.is_open(i, w, field);
        let Some((hits, blocked)) = field.rank(placed, ei, r, open, &best.1) else {
            continue;
        };
        let key = SpotKey {
            hits,
            blocked,
            strokes: field.strokes().count(r, ei, best.1.stroke_cap()),
            tier: Tier::Line,
            dev,
            order: k + 1,
        };
        if key.better(&best.1) {
            best = (p, key);
        }
    }
    // 2. Just beside the line, where it is clear and crossed by fewer
    // strokes.
    if !field.settled(&best.1) {
        if let Some((q, key)) = beside_line(field, placed, track, w, ei, &best.1, false) {
            best = (q, key);
        }
        // Still hitting something: sweep every offset in reach, finer
        // (in the final pass: the builders' one has more labels to free
        // and leaves the costliest search to it).
        if !best.1.hits.none() && field.strokes_matter {
            if let Some((q, key)) = beside_line(field, placed, track, w, ei, &best.1, true) {
                best = (q, key);
            }
        }
    }
    if !best.1.blocked {
        return best.0;
    }
    // Out of work: the best spot so far, or (none found off the line)
    // the line's own preferred one.
    if field.spent(placed) {
        return if off_line && best.1.dev.is_infinite() {
            track.own_spot()
        } else {
            best.0
        };
    }
    // 3. Anywhere on the line (near its ends, past a self-loop): the
    // nearest spot clear of everything, else just touching what it must
    // not cover.
    let (spots, clear) = nearest_clear(e, nodes, at, w, &track.spots, |p| {
        field.clear(placed, ei, label_rect(p, w))
    });
    let tier_key = |p: (f64, f64), hits: Hits, blocked: bool, order: usize| SpotKey {
        hits,
        blocked,
        strokes: field.strokes().count(label_rect(p, w), ei, usize::MAX),
        tier: Tier::Ends,
        dev: (p.0 - at.0).hypot(p.1 - at.1),
        order,
    };
    if let Some(p) = clear {
        let key = tier_key(p, Hits::default(), false, 0);
        if key.better(&best.1) {
            return p;
        }
    }
    let touch = |p: &(f64, f64)| {
        let r = label_rect(*p, w);
        field.untouched(r) && placed.clear(ei, r, 0.0)
    };
    if let Some(&p) = spots.iter().find(|p| touch(p)) {
        let key = tier_key(p, Hits::default(), true, 0);
        if key.better(&best.1) {
            best = (p, key);
        }
    }
    // 4. The spot covering least; `spots` is nearest first, and a
    // coarser stride keeps a long curve's scan bounded.
    if !best.1.hits.none() {
        let stride = spots.len().div_ceil(LABEL_MAX_SPOTS).max(1);
        for (i, &p) in spots.iter().enumerate().step_by(stride) {
            let hits = field.hits(placed, ei, label_rect(p, w));
            if hits.partial_cmp(&best.1.hits) == Some(std::cmp::Ordering::Less) {
                best = (p, tier_key(p, hits, true, i + 1));
            }
        }
    }
    best.0
}

/// Fallback when the scans found nothing clear: every spot along the
/// WHOLE densified curve `pts` (a self-loop also tries beyond its
/// loop's apex), nearest `at` first with ties in curve order, and the
/// first of them clear by `is_clear`.
fn nearest_clear(
    e: &SceneEdge,
    nodes: &[SceneNode],
    at: (f64, f64),
    w: f64,
    pts: &[(f64, f64)],
    is_clear: impl Fn((f64, f64)) -> bool,
) -> (Vec<Pt>, Option<Pt>) {
    // Sorted lazily in doubling chunks: most labels clear within a few
    // spots, and a long edge has thousands.
    let mut keyed: Vec<(f64, usize)> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| ((p.0 - at.0).hypot(p.1 - at.1), i))
        .collect();
    let order = |a: &(f64, usize), b: &(f64, usize)| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1));
    let mut clear = None;
    let (mut lo, mut chunk) = (0, LABEL_SPOT_CHUNK);
    while lo < keyed.len() && clear.is_none() {
        let hi = (lo + chunk).min(keyed.len());
        if hi < keyed.len() {
            keyed[lo..].select_nth_unstable_by(hi - lo - 1, order);
        }
        keyed[lo..hi].sort_unstable_by(order);
        clear = keyed[lo..hi]
            .iter()
            .map(|&(_, i)| pts[i])
            .find(|p| is_clear(*p));
        (lo, chunk) = (hi, chunk * 2);
    }
    // Only reached in full when no spot clears — then `keyed` is
    // completely sorted.
    let mut spots: Vec<(f64, f64)> = keyed.iter().map(|&(_, i)| pts[i]).collect();
    // A self-loop hugs its node, so no spot ON it may clear the node:
    // then the label steps out past the loop's apex.
    if clear.is_none() && e.from == e.to {
        if let Some(n) = nodes.iter().find(|n| n.id == e.from) {
            let out = beyond_loop(&spots, (n.x, n.y), w);
            clear = out.iter().copied().find(|p| is_clear(*p));
            spots.extend(out);
        }
    }
    (spots, clear)
}

/// The best spot half off or just beside the line — the box's own
/// half-extent less [`LABEL_STRADDLE`], or plus [`LABEL_SIDE_AIR`], or
/// as far as [`LABEL_DRIFT_MAX`] allows, off it, on either side of a
/// scanned spot of `track`; with `fine`, at most [`LABEL_FINE_STEPS`]
/// steps of at least [`LABEL_BESIDE_STEP`] across it up to there,
/// beside any spot of the whole curve — if it beats `best` (see
/// [`SpotKey`]). A spot is only taken while it still reads as its own
/// line's label: centre at most [`LABEL_SIDE_MAX`] beside it, box at
/// most [`LABEL_DRIFT_MAX`] from the drawn line, and no other edge
/// nearer than its own.
fn beside_line(
    field: &Field,
    placed: &LabelGrid,
    track: &Track,
    w: f64,
    ei: usize,
    best: &SpotKey,
    fine: bool,
) -> Option<((f64, f64), SpotKey)> {
    let spots = &track.spots;
    let mut found: Option<((f64, f64), SpotKey)> = None;
    // Just inside the drift limit, so rounding cannot push it past.
    let reach = LABEL_DRIFT_MAX - TOUCH_EPS;
    let bases: Vec<(f64, usize)> = if fine {
        track.whole()
    } else {
        let stride = track.scan.len().div_ceil(LABEL_BESIDE_SPOTS).max(1);
        track.scan.iter().step_by(stride).copied().collect()
    };
    for (dev, k) in bases {
        if field.spent(placed) {
            break;
        }
        let cur = found.as_ref().map_or(best, |f| &f.1);
        // Spots further from the preferred one only lose ties.
        if field.settled(cur) && dev >= cur.dev {
            break;
        }
        // The fine sweep only frees a label from what it covers: the
        // first base with a spot covering nothing ends it.
        if fine && cur.hits.none() {
            break;
        }
        let p = spots[k];
        let (a, b) = (
            spots[k.saturating_sub(1)],
            spots[(k + 1).min(spots.len() - 1)],
        );
        let l = (b.0 - a.0).hypot(b.1 - a.1);
        if l <= f64::EPSILON {
            continue;
        }
        let (nx, ny) = (-(b.1 - a.1) / l, (b.0 - a.0) / l);
        let half = w / 2.0 * nx.abs() + LABEL_BOX_H / 2.0 * ny.abs();
        // Signed offsets across the line: three set distances each
        // side, or even steps between the drift limits.
        let lim = (half + reach).min(LABEL_SIDE_MAX);
        let offs: Vec<f64> = if fine {
            // At most LABEL_FINE_STEPS steps each side: a wide label
            // beside a steep line reaches far, in coarser steps.
            let step = LABEL_BESIDE_STEP.max(lim / LABEL_FINE_STEPS as f64);
            let n = (lim / step).floor() as i64;
            (-n..=n).map(|i| i as f64 * step).collect()
        } else {
            [half - LABEL_STRADDLE, half + LABEL_SIDE_AIR, half + reach]
                .into_iter()
                .filter(|&o| o <= lim)
                .flat_map(|o| [-o, o])
                .collect()
        };
        let width = offs.len();
        for (side, o) in offs.into_iter().enumerate() {
            let (off, sign) = (o.abs(), o.signum());
            let q = (p.0 + sign * nx * off, p.1 + sign * ny * off);
            let r = label_rect(q, w);
            let cur = found.as_ref().map_or(best, |f| &f.1);
            let Some((hits, blocked)) = field.rank(placed, ei, r, field.open(r), cur) else {
                continue;
            };
            // Beside the line it must cross fewer strokes than a clear
            // spot on it: counting stops at the first one too many.
            let cap = cur.stroke_cap();
            let cap = if cur.tier < Tier::Beside && !cur.blocked {
                cap.saturating_sub(1)
            } else {
                cap
            };
            let crossed = field.strokes().count(r, ei, cap);
            if crossed > cap {
                continue;
            }
            let key = SpotKey {
                hits,
                blocked,
                strokes: crossed,
                tier: Tier::Beside,
                dev: dev + off,
                order: width * k + side,
            };
            if !key.better(cur) {
                continue;
            }
            if field.reads_own(r, ei) {
                found = Some((q, key));
            }
        }
    }
    found
}

/// Spots for a self-loop's label beyond its loop: from the drawn
/// loop's apex (its point farthest from the node centre `c`) outward,
/// [`LABEL_SLIDE_STEP`] apart, far enough for a `w`-wide label box to
/// clear the node whichever side the loop sticks out of.
fn beyond_loop(loop_pts: &[(f64, f64)], c: (f64, f64), w: f64) -> Vec<(f64, f64)> {
    let dist = |p: &(f64, f64)| (p.0 - c.0).hypot(p.1 - c.1);
    let Some(apex) = loop_pts
        .iter()
        .copied()
        .max_by(|a, b| dist(a).total_cmp(&dist(b)))
    else {
        return Vec::new();
    };
    let len = dist(&apex);
    if len <= f64::EPSILON {
        return Vec::new();
    }
    let (dx, dy) = ((apex.0 - c.0) / len, (apex.1 - c.1) / len);
    let steps = ((w / 2.0 + LABEL_BOX_H + LABEL_GAP) / LABEL_SLIDE_STEP).ceil() as usize;
    (1..=steps)
        .map(|k| {
            let t = k as f64 * LABEL_SLIDE_STEP;
            (apex.0 + dx * t, apex.1 + dy * t)
        })
        .collect()
}

/// Grow (and shift) the canvas so the labels of `moved` edges stay on
/// it with a [`MARGIN`] — the rest of the scene already fits.
pub(crate) fn fit_moved_labels(sc: &mut Scene, moved: &[usize]) {
    if moved.is_empty() {
        return;
    }
    let mut bb = Bbox::new();
    for &ei in moved {
        if let Some((_, m, w)) = &sc.edges[ei].label {
            bb.add(m.0 - w / 2.0, m.1 - LABEL_BOX_H / 2.0);
            bb.add(m.0 + w / 2.0, m.1 + LABEL_BOX_H / 2.0);
        }
    }
    let (minx, maxx, miny, maxy) = bb.finish();
    let (dx, dy) = ((MARGIN - minx).max(0.0), (MARGIN - miny).max(0.0));
    if dx > 0.0 || dy > 0.0 {
        shift_scene(sc, dx, dy);
    }
    sc.width = (sc.width + dx).max(maxx + dx + MARGIN);
    sc.height = (sc.height + dy).max(maxy + dy + MARGIN);
}

/// The edges whose label covers a node box or another edge's
/// arrowhead box (G11's hardest misses), and the edges whose arrowheads
/// it covers, flagged index-parallel with `edges`. A label the placement
/// stage could not clear on its line stays there while the lines are
/// kept, and a label merely touching a head's box ends up on the head
/// once a redrawn edge respaces that head's port, so
/// [`crate::scene::route_partial`] redraws these edges instead.
pub(crate) fn label_clashes(nodes: &[SceneNode], edges: &[SceneEdge]) -> Vec<bool> {
    let mut out = vec![false; edges.len()];
    let lines: Vec<Option<Vec<(f64, f64)>>> = edges
        .iter()
        .map(|e| {
            (edge_drawn(e) && !matches!(e.kind, EdgeKind::Invisible)).then(|| edge_polyline(e))
        })
        .collect();
    let mut heads: Vec<(usize, Rect)> = Vec::new();
    for (j, (e, pts)) in edges.iter().zip(&lines).enumerate() {
        let Some(pts) = pts else { continue };
        let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
            continue;
        };
        if e.ends.end_marker(e.kind).is_some() {
            heads.extend(arrow_box(last, pts.iter().rev().copied()).map(|r| (j, r)));
        }
        if e.ends.start_marker(e.kind).is_some() {
            heads.extend(arrow_box(first, pts.iter().copied()).map(|r| (j, r)));
        }
    }
    let meets = |a: Rect, b: Rect| a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
    for (i, e) in edges.iter().enumerate() {
        let Some((_, c, w)) = &e.label else { continue };
        if lines[i].is_none() {
            continue;
        }
        let r = label_rect(*c, *w);
        if nodes.iter().any(|n| {
            meets(
                r,
                (
                    n.x - n.w / 2.0,
                    n.y - n.h / 2.0,
                    n.x + n.w / 2.0,
                    n.y + n.h / 2.0,
                ),
            )
        }) {
            out[i] = true;
        }
        for &(j, _) in heads.iter().filter(|&&(j, h)| j != i && meets(r, h)) {
            out[i] = true;
            out[j] = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seg_hits_rect_clips() {
        let r = (0.0, 0.0, 10.0, 10.0);
        assert!(seg_hits_rect((-5.0, 5.0), (15.0, 5.0), r));
        assert!(seg_hits_rect((5.0, 5.0), (6.0, 6.0), r));
        assert!(!seg_hits_rect((-5.0, -5.0), (-1.0, 20.0), r));
        assert!(!seg_hits_rect((11.0, 0.0), (20.0, 10.0), r));
    }

    #[test]
    fn label_clashes_flag_a_label_on_a_node_or_a_head() {
        use crate::scene::{scene, SceneEdge};
        let g = crate::parser::parse("flowchart LR\nA-->|go|B\nC-->D").unwrap();
        let sc = scene(&g);
        assert_eq!(label_clashes(&sc.nodes, &sc.edges), vec![false, false]);
        // A->B's label moved onto C->D's arrowhead (both flagged), then
        // onto node A (only A->B).
        let mut edges: Vec<SceneEdge> = sc.edges.clone();
        let tip = edge_polyline(&edges[1]).last().copied().unwrap();
        if let Some((_, c, _)) = &mut edges[0].label {
            *c = tip;
        }
        assert_eq!(label_clashes(&sc.nodes, &edges), vec![true, true]);
        let a = (sc.nodes[0].x, sc.nodes[0].y);
        if let Some((_, c, _)) = &mut edges[0].label {
            *c = a;
        }
        assert_eq!(label_clashes(&sc.nodes, &edges), vec![true, false]);
    }

    #[test]
    fn arrow_box_sits_behind_the_tip() {
        let pts = [(0.0, 0.0), (0.0, 50.0), (0.0, 100.0)];
        let r = arrow_box((0.0, 100.0), pts.iter().rev().copied()).unwrap();
        assert!((r.3 - 100.0).abs() < 1e-9 && (r.1 - 50.0).abs() < 1e-9);
        assert!((r.2 - r.0 - ARROW_MARKER_PX).abs() < 1e-9);
    }

    /// Checks every label of a compact branch point's fan in `sc` (the
    /// point named `point`, flowing along y when `tb`, else along x):
    /// in the far half of its own line, crossed by no other stroke, and
    /// FAN_SIBLING_GAP clear across the flow of any sibling beside it.
    fn assert_fan_labels_read_as_their_own(sc: &Scene, point: &str, tb: bool, what: &str) {
        let lines: Vec<_> = sc.edges.iter().map(|e| Some(edge_polyline(e))).collect();
        let strokes = StrokeGrid::new(&lines);
        let mut boxes: Vec<(String, Rect)> = Vec::new();
        for (ei, e) in sc.edges.iter().enumerate() {
            let Some((text, c, w)) = &e.label else {
                continue;
            };
            if e.from != point {
                continue;
            }
            let r = label_rect(*c, *w);
            assert_eq!(
                strokes.count(r, ei, usize::MAX),
                0,
                "{what}: '{text}' lies on another line"
            );
            // Share of its own line (from the point) at its spot.
            let pts = densify(lines[ei].as_ref().unwrap());
            let mut arc = vec![0.0];
            for k in 1..pts.len() {
                let d = (pts[k].0 - pts[k - 1].0).hypot(pts[k].1 - pts[k - 1].1);
                arc.push(arc[k - 1] + d);
            }
            let d2 = |k: usize| (pts[k].0 - c.0).powi(2) + (pts[k].1 - c.1).powi(2);
            let k = (0..pts.len())
                .min_by(|&a, &b| d2(a).total_cmp(&d2(b)))
                .unwrap();
            let on = rect_line_dist((c.0, c.1, c.0, c.1), lines[ei].as_ref().unwrap());
            assert!(on < 0.5, "{what}: '{text}' {on:.1}px off its line");
            let share = arc[k] / arc[pts.len() - 1];
            assert!(share >= 0.5, "{what}: '{text}' at {share:.2} of its line");
            boxes.push((text.clone(), r));
        }
        assert!(boxes.len() >= 2, "{what}: a labelled fan");
        for (i, (ta, a)) in boxes.iter().enumerate() {
            for (tb_, b) in &boxes[i + 1..] {
                let (along, across) = if tb {
                    (a.1 < b.3 && b.1 < a.3, (b.0 - a.2).max(a.0 - b.2))
                } else {
                    (a.0 < b.2 && b.0 < a.2, (b.1 - a.3).max(a.1 - b.3))
                };
                assert!(
                    !along || across >= FAN_SIBLING_GAP - 0.01,
                    "{what}: '{ta}' and '{tb_}' {across:.1}px apart side by side"
                );
                assert_eq!(covered(*a, 0.0, &[*b]), 0.0, "{what}: '{ta}' on '{tb_}'");
            }
        }
    }

    #[test]
    fn choice_fan_labels_stack_clear_of_their_siblings_lines() {
        // A <<choice>> is a 28px diamond: its branches leave it a few px
        // apart, so near it any label sits on a sibling's line (the
        // 3-way fan used to put 'if n >= 0' on the Zero edge). Each
        // label is stacked in rows across the far half of its own line,
        // in the static scene, after a full reroute and after a drag.
        let cases = [
            (
                "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\n\
                 c --> False: if n < 0\nc --> True : if n >= 0\nc --> Zero : if n == 0",
                true,
            ),
            (
                "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\n\
                 c --> False: if n < 0\nc --> True : if n >= 0",
                true,
            ),
            (
                "stateDiagram-v2\nstate c <<choice>>\n[*] --> Order\nOrder --> c\n\
                 c --> Express : priority customer\nc --> Standard : regular customer\n\
                 c --> Hold : payment pending\nc --> Cancel : fraud suspected",
                true,
            ),
            (
                "stateDiagram-v2\ndirection LR\nstate c <<choice>>\n[*] --> Order\n\
                 Order --> c\nc --> A : one\nc --> B : two\nc --> C : three",
                false,
            ),
        ];
        for (src, tb) in cases {
            let crate::model::Document::State(g) = crate::parser::parse_document(src).unwrap()
            else {
                panic!("a state diagram");
            };
            let sc = crate::scene::scene(&g);
            assert_fan_labels_read_as_their_own(&sc, "c", tb, &format!("scene {src}"));
            let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let routed = crate::scene::route(&g, &at);
            assert_fan_labels_read_as_their_own(&routed, "c", tb, &format!("route {src}"));
            // Drag the first branch's target further out along the flow.
            let target = g
                .edges
                .iter()
                .find(|e| g.nodes[e.from].id == "c")
                .unwrap()
                .to;
            let mut moved = at.clone();
            if tb {
                moved[target].1 += 40.0;
            } else {
                moved[target].0 += 40.0;
            }
            let dragged = crate::scene::route_partial(&g, &moved, &sc, &at);
            assert_fan_labels_read_as_their_own(&dragged, "c", tb, &format!("drag {src}"));
        }
    }

    /// Every label of `point`'s fan in `sc` reads as its own: crossed by
    /// no other edge's stroke, its own line the nearest (on it or just
    /// beside it), and FAN_SIBLING_GAP clear across the flow of any
    /// sibling beside it. Returns each label's signed offset across the
    /// flow from its own line's far end (`tb`: x, else y), by text.
    fn assert_fan_labels_clear(
        sc: &Scene,
        point: &str,
        tb: bool,
        what: &str,
    ) -> Vec<(String, f64)> {
        let lines: Vec<_> = sc.edges.iter().map(|e| Some(edge_polyline(e))).collect();
        let strokes = StrokeGrid::new(&lines);
        let mut boxes: Vec<(String, Rect)> = Vec::new();
        let mut sides = Vec::new();
        for (ei, e) in sc.edges.iter().enumerate() {
            let Some((text, c, w)) = &e.label else {
                continue;
            };
            if e.from != point {
                continue;
            }
            let r = label_rect(*c, *w);
            assert_eq!(
                strokes.count(r, ei, usize::MAX),
                0,
                "{what}: '{text}' lies on another line"
            );
            let (own2, other2) = strokes.dists2(r, ei, LABEL_DRIFT_MAX);
            assert!(
                own2 < LABEL_DRIFT_MAX * LABEL_DRIFT_MAX && other2 >= own2,
                "{what}: '{text}' reads as another line's ({own2:.0} vs {other2:.0})"
            );
            let far = *lines[ei].as_ref().unwrap().last().unwrap();
            sides.push((text.clone(), if tb { c.0 - far.0 } else { c.1 - far.1 }));
            boxes.push((text.clone(), r));
        }
        assert!(boxes.len() >= 2, "{what}: a labelled fan");
        for (i, (ta, a)) in boxes.iter().enumerate() {
            for (tb_, b) in &boxes[i + 1..] {
                let (along, across) = if tb {
                    (a.1 < b.3 && b.1 < a.3, (b.0 - a.2).max(a.0 - b.2))
                } else {
                    (a.0 < b.2 && b.0 < a.2, (b.1 - a.3).max(a.1 - b.3))
                };
                assert!(
                    !along || across >= FAN_SIBLING_GAP - 0.01,
                    "{what}: '{ta}' and '{tb_}' {across:.1}px apart side by side"
                );
            }
        }
        sides
    }

    #[test]
    fn choice_twins_label_beside_their_lines_wide_fans_spread() {
        let state = |src: &str| match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            other => panic!("not a state diagram: {other:?}"),
        };
        // Two transitions from a choice to one state run PARALLEL_GAP
        // apart all the way: no row on either line is clear of the
        // other, so each label steps beside its own line, outward ('one'
        // used to sit on the line, 'two' over both strokes).
        let g = state(
            "stateDiagram-v2\nstate c <<choice>>\n[*] --> A\nA --> c\n\
             c --> B : one\nc --> B : two\nc --> C : three",
        );
        let sc = crate::scene::scene(&g);
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let routed = crate::scene::route(&g, &at);
        let mut moved = at.clone();
        moved[g.node_index("C").unwrap()].1 += 30.0;
        let dragged = crate::scene::route_partial(&g, &moved, &sc, &at);
        for (what, s) in [("scene", &sc), ("route", &routed), ("drag", &dragged)] {
            let sides = assert_fan_labels_clear(s, "c", true, what);
            let side = |t: &str| sides.iter().find(|s| s.0 == t).unwrap().1;
            assert!(side("one") < 0.0 && side("two") > 0.0, "{what}: {sides:?}");
        }
        // Labels wider than the room between their targets: no row can
        // keep one clear of its neighbours' lines, so the targets spread
        // and each label sits on its own line, mermaid-like.
        let wide = "stateDiagram-v2\nstate c <<choice>>\n[*] --> A\nA --> c\n\
                    c --> B : this is a very long label for the first branch\n\
                    c --> C : and an equally long label for branch two\n\
                    c --> D : third one is also quite long indeed";
        let g = state(wide);
        let sc = crate::scene::scene(&g);
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        assert_fan_labels_read_as_their_own(&sc, "c", true, "wide scene");
        let routed = crate::scene::route(&g, &at);
        assert_fan_labels_read_as_their_own(&routed, "c", true, "wide route");
        // A choice dragged toward its far ends leaves too few rows for
        // its labels on their lines: they step beside them instead of
        // crowding a sibling or its line.
        let g = state(
            "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\n\
             c --> False: if n < 0\nc --> True : if n >= 0\nc --> Zero : if n == 0",
        );
        let sc = crate::scene::scene(&g);
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut moved = at.clone();
        moved[g.node_index("c").unwrap()].1 += 50.0;
        let routed = crate::scene::route(&g, &moved);
        assert_fan_labels_clear(&routed, "c", true, "dragged choice, route");
        let partial = crate::scene::route_partial(&g, &moved, &sc, &at);
        assert_fan_labels_clear(&partial, "c", true, "dragged choice, route_partial");
    }

    #[test]
    fn a_fan_label_at_its_midpoint_slot_is_stacked_afresh() {
        // The builders' midpoint slot lies short of FAN_FAR; once
        // stacked, a label stays put on a further pass.
        let src = "stateDiagram-v2\nstate c <<choice>>\n[*] --> c\nc --> A : yes\nc --> B : no";
        let crate::model::Document::State(g) = crate::parser::parse_document(src).unwrap() else {
            panic!("a state diagram");
        };
        let mut sc = crate::scene::scene(&g);
        let before: Vec<_> = sc.edges.iter().map(|e| e.label.clone()).collect();
        let none = vec![false; sc.edges.len()];
        place(
            &sc.nodes,
            &sc.clusters,
            &mut sc.edges,
            Pass::Changed(&none),
            LABEL_STATIC_WORK,
        );
        let after: Vec<_> = sc.edges.iter().map(|e| e.label.clone()).collect();
        assert_eq!(before, after, "a stacked fan label keeps its spot");
        // Put one back on its midpoint: the next pass stacks it again.
        let ei = sc.edges.iter().position(|e| e.label.is_some()).unwrap();
        let b = sc.edges[ei].bezier;
        let mid = (
            (b[0].0 + 3.0 * b[1].0 + 3.0 * b[2].0 + b[3].0) / 8.0,
            (b[0].1 + 3.0 * b[1].1 + 3.0 * b[2].1 + b[3].1) / 8.0,
        );
        if let Some((_, m, _)) = &mut sc.edges[ei].label {
            *m = mid;
        }
        let moved = place(
            &sc.nodes,
            &sc.clusters,
            &mut sc.edges,
            Pass::Changed(&none),
            LABEL_STATIC_WORK,
        );
        assert!(moved.contains(&ei));
        assert_eq!(sc.edges[ei].label, before[ei], "back to its row");
    }

    #[test]
    fn wide_lr_labels_step_beside_their_line_off_the_arrowhead() {
        // Each label is about as long as its gap: centred on the line it
        // would hide the arrowhead, so it sits just beside the line.
        let src = "flowchart LR\nA -->|a label as long as the whole gap is| B\nB -->|another fairly long edge label| C";
        let r = crate::quality::measure_source(src, false).unwrap();
        for m in [&r.statik, &r.routed] {
            assert_eq!(
                (m.lbl_node, m.lbl_lbl, m.lbl_chip, m.lbl_arrow),
                (0, 0, 0, 0),
                "{m:?}"
            );
        }
    }

    #[test]
    fn a_label_crossed_by_another_edge_slides_off_it() {
        // C->D crosses the long A->B edge near A->B's middle label.
        let src = "flowchart TD\nA -->|crossed| B\nA --> X --> Y --> B\nC --> D\nX --> D\nC --> Y";
        let r = crate::quality::measure_source(src, false).unwrap();
        assert_eq!(r.statik.lbl_edge, 0, "{:?}", r.statik);
    }

    /// A straight, unmarked edge `{id}0 -> {id}1` along y = `y`.
    fn line(id: &str, y: f64, label: Option<(String, (f64, f64), f64)>) -> SceneEdge {
        SceneEdge {
            from: format!("{id}0"),
            to: format!("{id}1"),
            bezier: [(-50.0, y), (100.0, y), (300.0, y), (450.0, y)],
            waypoints: Vec::new(),
            kind: EdgeKind::Open,
            ends: Default::default(),
            meta: Default::default(),
            label,
        }
    }

    /// A flat node centred at (`x`, `y`).
    fn bar(x: f64, y: f64, w: f64, h: f64) -> SceneNode {
        SceneNode {
            id: "bar".into(),
            x,
            y,
            w,
            h,
            shape: crate::model::Shape::Rect,
            label: String::new(),
            style: Default::default(),
            link: None,
        }
    }

    /// Box distance from edge `ei`'s label to its own line and to the
    /// nearest other drawn line.
    fn label_dists(edges: &[SceneEdge], ei: usize) -> (f64, f64) {
        let (_, c, w) = edges[ei].label.clone().unwrap();
        let r = label_rect(c, w);
        let mut other = f64::INFINITY;
        for (j, e) in edges.iter().enumerate() {
            if j != ei && edge_drawn(e) && !matches!(e.kind, EdgeKind::Invisible) {
                other = other.min(rect_line_dist(r, &edge_polyline(e)));
            }
        }
        (rect_line_dist(r, &edge_polyline(&edges[ei])), other)
    }

    #[test]
    fn a_clear_label_in_a_bundle_stays_on_its_line() {
        // Strokes run through the label's own spot and just beside its
        // line: a clear label keeps its slot, and a re-placed one slides
        // along its curve but never off it.
        let beside = LABEL_BOX_H / 2.0 + LABEL_SIDE_AIR;
        let w = 60.0;
        let mut edges = vec![line("L", 0.0, Some(("l".into(), (200.0, 0.0), w)))];
        for (k, y) in [LABEL_BOX_H / 4.0, -LABEL_BOX_H / 4.0, beside, -beside]
            .into_iter()
            .enumerate()
        {
            edges.push(line(&format!("b{k}"), y, None));
        }
        assert!(place(&[], &[], &mut edges, Pass::All, LABEL_STATIC_WORK).is_empty());
        let mut moved = vec![false; edges.len()];
        moved[0] = true;
        place(
            &[],
            &[],
            &mut edges,
            Pass::Changed(&moved),
            LABEL_STATIC_WORK,
        );
        assert_eq!(label_dists(&edges, 0).0, 0.0, "{:?}", edges[0].label);
    }

    #[test]
    fn a_label_steps_beside_its_line_only_off_other_lines() {
        // A flat node lies along the whole line, so no spot ON it is
        // clear: the label steps just beside it.
        let w = 60.0;
        let node = [bar(200.0, 0.0, 700.0, 6.0)];
        let mut edges = vec![line("L", 0.0, Some(("l".into(), (200.0, 0.0), w)))];
        assert_eq!(
            place(&node, &[], &mut edges, Pass::All, LABEL_STATIC_WORK),
            vec![0]
        );
        let (own, _) = label_dists(&edges, 0);
        assert!(own > 0.0 && own <= LABEL_DRIFT_MAX, "{own}");

        // Another line just past one side: the label takes the other.
        let far = LABEL_BOX_H + LABEL_SIDE_AIR + LABEL_GAP;
        for sign in [-1.0, 1.0] {
            let mut edges = vec![
                line("L", 0.0, Some(("l".into(), (200.0, 0.0), w))),
                line("o", sign * far, None),
            ];
            place(&node, &[], &mut edges, Pass::All, LABEL_STATIC_WORK);
            let (own, other) = label_dists(&edges, 0);
            let y = edges[0].label.as_ref().unwrap().1 .1;
            assert!(y * sign < 0.0, "sign {sign}: {y}");
            assert!(own <= LABEL_DRIFT_MAX && other >= own, "{own} {other}");
        }

        // Lines through both sides: it stays on its own line.
        let mut edges = vec![
            line("L", 0.0, Some(("l".into(), (200.0, 0.0), w))),
            line("o", far, None),
            line("p", -far, None),
        ];
        place(&node, &[], &mut edges, Pass::All, LABEL_STATIC_WORK);
        assert_eq!(label_dists(&edges, 0).0, 0.0, "{:?}", edges[0].label);
    }

    /// Every label of every quality fixture, static and routed, sits
    /// within [`LABEL_DRIFT_MAX`] of its own drawn line and no nearer to
    /// another edge than to its own (measured here, not by the quality
    /// yardstick). Self-loop labels are exempt: they may sit beyond
    /// their loop's apex.
    #[test]
    fn every_fixture_label_reads_as_its_own_lines() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        assert!(!files.is_empty());
        let mut bad = Vec::new();
        for f in files {
            let g = crate::parser::parse(&std::fs::read_to_string(&f).unwrap()).unwrap();
            let sc = crate::scene::scene(&g);
            let centres: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rs = crate::scene::route(&g, &centres);
            for (mode, s) in [("static", &sc), ("routed", &rs)] {
                for (ei, e) in s.edges.iter().enumerate() {
                    if e.label.is_none()
                        || e.from == e.to
                        || !edge_drawn(e)
                        || matches!(e.kind, EdgeKind::Invisible)
                    {
                        continue;
                    }
                    let (own, other) = label_dists(&s.edges, ei);
                    if own > LABEL_DRIFT_MAX || (own > TOUCH_EPS && other < own) {
                        bad.push(format!(
                            "{} {mode} {}->{}: own {own:.1} other {other:.1}",
                            f.display(),
                            e.from,
                            e.to
                        ));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    #[test]
    fn relabel_moves_a_changed_edges_label_back_onto_its_line() {
        let g = crate::parser::parse("flowchart TD\nA -->|x| B\nA -->|y| C\nB --> C").unwrap();
        let mut sc = crate::scene::scene(&g);
        let cx = crate::edgepass::ctx(&g, &sc);
        let before: Vec<_> = sc.edges.iter().map(|e| e.label.clone()).collect();
        // A later stage re-drew edge 0 one label width to the side: its
        // old label spot is now off its line.
        let dx = 80.0;
        let e = &mut sc.edges[0];
        for p in e.bezier.iter_mut().chain(e.waypoints.iter_mut()) {
            p.0 += dx;
        }
        let mut moved = vec![false; sc.edges.len()];
        moved[0] = true;
        relabel(&g, &mut sc, &cx, &moved, LABEL_ROUTE_WORK);
        let (own, _) = label_dists(&sc.edges, 0);
        assert!(own <= TOUCH_EPS, "label left {own}px off its line");
        assert_ne!(sc.edges[0].label, before[0]);
        for (i, b) in before.iter().enumerate().skip(1) {
            assert_eq!(&sc.edges[i].label, b, "unmoved edge {i} kept its label");
        }
    }

    #[test]
    fn a_spent_label_budget_keeps_labels_on_their_own_lines() {
        // Every edge redrawn one label width aside, and no work to search
        // with: each label goes back onto its own line, at the spot the
        // scan would try first, instead of staying where it was left.
        let g = crate::parser::parse("flowchart TD\nA -->|x| B\nA -->|y| C\nB -->|z| C").unwrap();
        let mut sc = crate::scene::scene(&g);
        let cx = crate::edgepass::ctx(&g, &sc);
        for e in &mut sc.edges {
            for p in e.bezier.iter_mut().chain(e.waypoints.iter_mut()) {
                p.0 += 80.0;
            }
        }
        let moved = vec![true; sc.edges.len()];
        relabel(&g, &mut sc, &cx, &moved, 0);
        for i in 0..sc.edges.len() {
            let (own, _) = label_dists(&sc.edges, i);
            assert!(own <= TOUCH_EPS, "label {i} left {own}px off its line");
        }
    }

    /// Many labelled edges in one channel, with a tenth of the drag
    /// budget: the stage stops searching early (G13) and still leaves
    /// every label reading as its own line's.
    #[test]
    fn a_short_label_budget_still_leaves_labels_on_their_lines() {
        let mut src = String::from("flowchart TD\n");
        for i in 0..40 {
            src += &format!("A -->|lorem {i}| B\n");
        }
        let g = crate::parser::parse(&src).unwrap();
        let mut sc = crate::scene::scene(&g);
        let cx = crate::edgepass::ctx(&g, &sc);
        let n = sc.edges.len();
        relabel(&g, &mut sc, &cx, &vec![true; n], LABEL_ROUTE_WORK / 10);
        for i in 0..n {
            let (own, _) = label_dists(&sc.edges, i);
            assert!(
                own <= LABEL_DRIFT_MAX,
                "label {i} left {own}px off its line"
            );
        }
    }

    #[test]
    fn far_coordinates_keep_the_stroke_grid_small() {
        // A segment millions of px long is not bucketed cell by cell.
        assert_eq!(seg_cells((0.0, 0.0), (1e7, 1e7), STROKE_CELL), None);
        let c = seg_cells((0.0, 0.0), (1000.0, 1000.0), STROKE_CELL).unwrap();
        // The diagonal passes ~2 cells per step, not its 16x16 box.
        assert!(c.len() < 3 * 16 + 2, "{}", c.len());
        assert!(c.contains(&(0, 0)) && c.contains(&(15, 15)) && !c.contains(&(15, 0)));
        assert_eq!(
            seg_cells((f64::NAN, 0.0), (1.0, 1.0), STROKE_CELL),
            Some(vec![])
        );
    }

    #[test]
    fn relabel_without_changes_is_a_no_op() {
        let g = crate::parser::parse("flowchart TD\nA -->|x| B\nA -->|y| C\nB --> C").unwrap();
        let mut sc = crate::scene::scene(&g);
        let before = crate::scene::to_svg(&sc);
        let cx = crate::edgepass::ctx(&g, &sc);
        let n = sc.edges.len();
        relabel(&g, &mut sc, &cx, &vec![false; n], LABEL_ROUTE_WORK);
        assert_eq!(before, crate::scene::to_svg(&sc));
        relabel(&g, &mut sc, &cx, &vec![true; n], LABEL_ROUTE_WORK);
        assert_eq!(before, crate::scene::to_svg(&sc), "clean labels stay put");
    }

    #[test]
    fn spot_key_ranks_text_over_arrowheads_over_room_over_strokes() {
        let key = |text, arrow, minor, blocked, strokes| SpotKey {
            hits: Hits {
                text,
                arrow,
                minor,
                area: 0.0,
            },
            blocked: blocked || text + arrow + minor > 0,
            strokes,
            tier: Tier::Line,
            dev: 0.0,
            order: 0,
        };
        // Crossing many strokes always beats covering an arrowhead,
        // which always beats hiding text.
        assert!(key(0, 0, 0, false, 9).better(&key(0, 1, 0, true, 0)));
        assert!(key(0, 1, 0, true, 0).better(&key(1, 0, 0, true, 0)));
        assert!(key(0, 0, 1, true, 0).better(&key(0, 1, 0, true, 0)));
        // Lacking room counts as one stroke: it ties with a clear spot
        // crossed once (the clear one wins), beats one crossed twice.
        assert!(key(0, 0, 0, false, 1).better(&key(0, 0, 0, true, 0)));
        assert!(key(0, 0, 0, true, 0).better(&key(0, 0, 0, false, 2)));
    }

    #[test]
    fn the_keep_out_keeps_the_yardsticks_crowd_gap() {
        // Clear by `covered` lets TOUCH_EPS through: what is left must
        // still be the gap the yardstick calls crowding.
        const _: () = assert!(LABEL_KEEP - TOUCH_EPS >= crate::quality::LABEL_CROWD_GAP);
        let mut edges = vec![
            line("a", 0.0, Some(("a".into(), (200.0, 0.0), 60.0))),
            line(
                "b",
                LABEL_BOX_H + 1.0,
                Some(("b".into(), (210.0, 21.0), 60.0)),
            ),
        ];
        let moved = vec![true; edges.len()];
        place(
            &[],
            &[],
            &mut edges,
            Pass::Changed(&moved),
            LABEL_STATIC_WORK,
        );
        let r: Vec<Rect> = edges
            .iter()
            .map(|e| {
                let (_, c, w) = e.label.clone().unwrap();
                label_rect(c, w)
            })
            .collect();
        let gx = (r[1].0 - r[0].2).max(r[0].0 - r[1].2).max(0.0);
        let gy = (r[1].1 - r[0].3).max(r[0].1 - r[1].3).max(0.0);
        assert!(gx.hypot(gy) >= crate::quality::LABEL_CROWD_GAP, "{r:?}");
    }

    /// A vertical edge from `(x, y0)` down to an arrowhead at `(x, y1)`.
    fn drop_to(id: &str, x: f64, y0: f64, y1: f64) -> SceneEdge {
        SceneEdge {
            from: format!("{id}0"),
            to: format!("{id}1"),
            bezier: [
                (x, y0),
                (x, y0 + (y1 - y0) / 3.0),
                (x, y1 - (y1 - y0) / 3.0),
                (x, y1),
            ],
            waypoints: Vec::new(),
            kind: EdgeKind::Arrow,
            ends: Default::default(),
            meta: Default::default(),
            label: None,
        }
    }

    #[test]
    fn a_label_crosses_strokes_rather_than_cover_an_arrowhead() {
        // An arrowhead ends on the label's line at its preferred spot,
        // and plain lines cross the rest of it: the label takes a spot
        // that only crosses a stroke.
        let w = 60.0;
        let mut edges = vec![line("L", 0.0, Some(("l".into(), (200.0, 0.0), w)))];
        edges.push(drop_to("v", 200.0, -150.0, -1.0));
        for (k, x) in [40.0, 90.0, 140.0, 260.0, 310.0, 360.0]
            .into_iter()
            .enumerate()
        {
            let mut e = drop_to(&format!("s{k}"), x, -150.0, 150.0);
            e.kind = EdgeKind::Open;
            edges.push(e);
        }
        let moved = vec![true; edges.len()];
        place(
            &[],
            &[],
            &mut edges,
            Pass::Changed(&moved),
            LABEL_STATIC_WORK,
        );
        let (_, c, _) = edges[0].label.clone().unwrap();
        let pts = edge_polyline(&edges[1]);
        let head = arrow_box(*pts.last().unwrap(), pts.iter().rev().copied()).unwrap();
        assert_eq!(covered(label_rect(c, w), 0.0, &[head]), 0.0, "{c:?}");
        let (own, other) = label_dists(&edges, 0);
        assert!(own <= LABEL_DRIFT_MAX && (own <= TOUCH_EPS || other >= own));
    }

    #[test]
    fn relabel_moves_a_label_off_a_stroke_even_when_nothing_moved() {
        // A stroke runs through the label: the final pass slides it
        // along its own line, off the stroke.
        let w = 60.0;
        let mut edges = vec![line("L", 0.0, Some(("l".into(), (200.0, 0.0), w)))];
        let mut cross = drop_to("x", 200.0, -150.0, 150.0);
        cross.kind = EdgeKind::Open;
        edges.push(cross);
        let lines: Vec<_> = edges.iter().map(|e| Some(edge_polyline(e))).collect();
        let crossed = |e: &[SceneEdge]| {
            let (_, c, w) = e[0].label.clone().unwrap();
            StrokeGrid::new(&lines).count(label_rect(c, w), 0, usize::MAX)
        };
        assert_eq!(crossed(&edges), 1);
        // The builders' pass leaves strokes alone.
        assert!(place(&[], &[], &mut edges, Pass::All, LABEL_STATIC_WORK).is_empty());
        let none = vec![false; edges.len()];
        assert_eq!(
            place(
                &[],
                &[],
                &mut edges,
                Pass::Changed(&none),
                LABEL_STATIC_WORK
            ),
            vec![0]
        );
        assert_eq!(crossed(&edges), 0);
        assert_eq!(label_dists(&edges, 0).0, 0.0);
    }

    /// Deterministic pseudo-random lines for the grid checks.
    fn scribbles(n: usize, scale: f64) -> Vec<Option<Vec<(f64, f64)>>> {
        let mut x: u64 = 11;
        let mut unit = move || {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (x >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..n)
            .map(|_| Some((0..6).map(|_| (unit() * scale, unit() * scale)).collect()))
            .collect()
    }

    #[test]
    fn the_stroke_grid_answers_like_a_scan_of_every_segment() {
        for scale in [600.0, 3e5] {
            let lines = scribbles(12, scale);
            let g = StrokeGrid::new(&lines);
            for k in 0..40 {
                let t = k as f64 / 40.0;
                let r = label_rect((scale * t, scale * (1.0 - t)), 80.0);
                for own in [0, 5] {
                    // Every other edge with a segment through the padded box.
                    let q = (
                        r.0 - STROKE_PAD,
                        r.1 - STROKE_PAD,
                        r.2 + STROKE_PAD,
                        r.3 + STROKE_PAD,
                    );
                    let want = lines
                        .iter()
                        .enumerate()
                        .filter(|(ei, l)| {
                            *ei != own
                                && l.as_ref()
                                    .unwrap()
                                    .windows(2)
                                    .any(|w| seg_hits_rect(w[0], w[1], q))
                        })
                        .count();
                    assert_eq!(g.count(r, own, usize::MAX), want, "{scale} {k}");
                    // Nearest own and other strokes, capped at the reach.
                    let reach = 50.0;
                    let mut want: (f64, f64) = (reach * reach, reach * reach);
                    for (ei, l) in lines.iter().enumerate() {
                        for w in l.as_ref().unwrap().windows(2) {
                            let d = rect_seg_dist2(r, w[0], w[1]);
                            let m = if ei == own { &mut want.0 } else { &mut want.1 };
                            *m = m.min(d);
                        }
                    }
                    assert_eq!(g.dists2(r, own, reach), want, "{scale} {k}");
                }
            }
        }
    }

    #[test]
    fn squared_box_segment_distance_needs_no_root() {
        let r = (0.0, 0.0, 10.0, 10.0);
        // Crossing, beside a side, off a corner, a point segment.
        assert_eq!(rect_seg_dist2(r, (-5.0, 5.0), (15.0, 5.0)), 0.0);
        assert_eq!(rect_seg_dist2(r, (13.0, -5.0), (13.0, 20.0)), 9.0);
        assert_eq!(rect_seg_dist2(r, (13.0, 14.0), (20.0, 30.0)), 25.0);
        assert_eq!(rect_seg_dist2(r, (-3.0, -4.0), (-3.0, -4.0)), 25.0);
        assert_eq!(rect_line_dist(r, &[(13.0, 14.0), (20.0, 30.0)]), 5.0);
    }

    /// The label stage of one `scene()` or `route()` on the dense
    /// fixtures: the builders' pass from every label at the middle of
    /// its line, then the final pass with every edge changed — fastest
    /// of a few runs, native release. Run with
    /// `cargo test --release --lib -- --ignored --test-threads=1`.
    #[test]
    #[ignore]
    fn label_stage_perf_guard() {
        /// Most ms both passes may take together on one dense fixture.
        const CEILING_MS: f64 = 1.0;
        /// Timed runs per fixture (the fastest counts).
        const RUNS: usize = 20;
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
        for name in ["dense-lr", "dense-td"] {
            let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).unwrap();
            let g = crate::parser::parse(&src).unwrap();
            let sc = crate::scene::scene(&g);
            let mut fresh = sc.edges.clone();
            for e in &mut fresh {
                let mid = {
                    let pts = edge_polyline(e);
                    pts[pts.len() / 2]
                };
                if let Some((_, c, _)) = &mut e.label {
                    *c = mid;
                }
            }
            let all = vec![true; fresh.len()];
            let ms = crate::quality::fastest_ms(RUNS, || {
                let mut edges = fresh.clone();
                place(
                    &sc.nodes,
                    &sc.clusters,
                    &mut edges,
                    Pass::All,
                    LABEL_STATIC_WORK,
                );
                place(
                    &sc.nodes,
                    &sc.clusters,
                    &mut edges,
                    Pass::Changed(&all),
                    LABEL_STATIC_WORK,
                );
                edges
            });
            println!("{name}: label stage {ms:.3} ms");
            assert!(ms <= CEILING_MS, "{name}: label stage {ms:.3} ms");
        }
    }

    #[test]
    fn obstacle_grid_span_past_i64_cells_is_big_not_enumerated() {
        let mut g = ObstacleGrid::new();
        // Cell indices 0..=i64::MAX: the old guard wrapped to i64::MIN.
        g.push((98.0, 464.0, 5e97, 481.0));
        g.push((-5e97, 0.0, 10.0, 10.0));
        assert_eq!(g.big.len(), 2);
        assert!(g.cells.is_empty());
        assert_eq!(span_cells((0, 0, i64::MAX, 0)), None);
        assert_eq!(span_cells((i64::MIN, 0, 0, 0)), None);
        assert_eq!(span_cells((0, 0, 1, 1)), Some(4));
    }
}
