//! Layout-quality metrics for flowcharts — the yardstick layout and
//! routing changes are judged by (`tests/quality.rs` gates on it,
//! `examples/quality.rs` prints it for a directory of `.mmd` files).
//!
//! Everything is measured on the geometry as DRAWN: each edge is the
//! dense polyline [`crate::scene`] uses for picking (the same curve
//! `to_svg` paints), each node its shape, each frame its rectangle,
//! each label its white box. A diagram is measured twice: the static
//! layout ([`scene::scene`]) and the interactive path a host takes
//! after `auto_positions` — [`scene::route`] fed the static node
//! centres, exactly what the playground's `render_routed` draws before
//! the first drag.
//!
//! Not a stable API: names, fields and thresholds change whenever the
//! crew tightens the gate.

use crate::layout::{text_width, title_extra, LABEL_BOX_H, TITLE_SCALE};
use crate::model::{Direction, Graph, Shape};
use crate::parser::parse_document;
use crate::scene::{self, edge_drawn, edge_polyline, Scene, SceneEdge, SceneNode, MARGIN};
use crate::Document;
use std::collections::HashMap;

// The separation thresholds are shared with the edge stages (one
// definition, see `crate::clearance`); re-exported under their old
// names, so the metrics read exactly as before.
pub use crate::clearance::{
    BORDER_HUG_GAP, CLEAR_GAP, CRAMP_NODE_GAP, CROSS_MERGE, EDGE_SPACING, FRAME_INSET,
    MIN_OVERLAP_RUN, NODE_CLEARANCE, PARALLEL_COS, PORT_ZONE, SAMPLE_STEP, TIGHT_RUN_MIN,
    WIGGLE_TOL,
};
pub(crate) use crate::clearance::{CHIP_H, CHIP_INSET_X, CHIP_INSET_Y, CHIP_PAD_W, CHIP_SEP};

/// Two edge ends on the same endpoint closer than this share one
/// port (px, G9).
pub const PORT_MIN_GAP: f64 = 4.0;
/// Edge-edge gaps are only searched this far out (px); a diagram whose
/// closest pair of unrelated edges is further apart reports this.
pub const GAP_PROBE: f64 = 40.0;
/// Unrelated edge pairs drawn closer than this (px) get a `--details`
/// line, so a low `min_edge_gap` can be chased down.
pub const GAP_DETAIL: f64 = 8.0;
/// Waypoint coordinates closer than this (px) are on one line: how an
/// axis-aligned router leg is told from a curve's sample.
pub const LINE_EPS: f64 = 0.01;
/// An edge needs at least this many waypoints (two legs and the
/// rounded corner between them) to count as an orthogonal router path.
pub const ORTHO_MIN_POINTS: usize = 4;
/// Two boxes overlap when they share more than this much depth (px)
/// on both axes — touching borders do not count.
pub const OVERLAP_EPS: f64 = 0.5;
/// Arrowhead length measured back from the tip (px): the default
/// marker is 7 stroke widths of 1.7 px, its tip at 85% of that.
pub const ARROW_LEN: f64 = 11.0;
/// A run along one frame border counts once it is longer than this
/// (px); shorter ones are an edge crossing or leaving the border.
pub const BORDER_RUN_MIN: f64 = 40.0;
/// Two edges touching closer than this (px) at a bend, without
/// crossing and away from a shared end, read as one junction: a shared
/// corner counts as an overlap like a shared run (G1).
pub const TOUCH_GAP: f64 = EDGE_SPACING / 2.0;
/// Samples this close (px) to a crossing of the pair are not a touch:
/// a shallow crossing runs within [`TOUCH_GAP`] on both sides of it.
pub const TOUCH_CROSS_SKIP: f64 = 2.0 * CRAMP_NODE_GAP;
/// `route()` repetitions per timing; the minimum is reported.
pub const ROUTE_RUNS: usize = 5;
/// Crossing gaps on one edge closer than this to the next one along it
/// (px) chain up; a chain of [`DASH_RUN`] reads as a dashed stroke
/// (G1: dashes mean `-.->` in mermaid).
pub const DASH_PERIOD: f64 = 60.0;
/// Gaps in one chain (see [`DASH_PERIOD`]) from which an edge reads as
/// dashed; also the cuts (gaps or notches) within [`DASH_WINDOW`] from
/// which it does.
pub const DASH_RUN: usize = 3;
/// Span (px) along an edge in which [`DASH_RUN`] crossing cuts, notches
/// included, read as a dashed stroke: measured on long under-edges cut
/// four to six times, which read as `-.->` even with every third cut a
/// notch (G1).
pub const DASH_WINDOW: f64 = 100.0;
/// Two edge labels closer than this (px) without overlapping crowd each
/// other (G11, label piles at hubs).
pub const LABEL_CROWD_GAP: f64 = 4.0;
/// Drag benchmark: positions per node along the drawing's diagonal
/// (from its bottom-left to its top-right corner).
pub const DRAG_SWEEP: usize = 8;
/// Drag benchmark: the local nudge (px) each node is also moved by, up,
/// down, left and right of where it sits — a short drag.
pub const DRAG_NUDGE: f64 = 60.0;
/// Drag benchmark: `route()` repetitions per frame; the minimum counts.
pub const DRAG_RUNS: usize = 3;
/// Drag benchmark: a frame slower than this (ms, native release) is
/// over budget — the proxy for 16 ms in wasm (G13).
pub const DRAG_FRAME_MS: f64 = 8.0;

/// How a metric folds across many diagrams in a totals row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agg {
    Sum,
    Min,
    Max,
    Mean,
}

/// Quality numbers for one laid-out scene. Counts are "lower is
/// better" unless noted.
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    /// Nodes / drawn non-loop edges / frames measured.
    pub nodes: usize,
    pub edges: usize,
    pub frames: usize,
    /// Edges that cross at least one node they don't attach to (G6).
    pub edge_node_x: usize,
    /// (edge, unrelated node) crossing pairs.
    pub edge_node_pairs: usize,
    /// Edges that enter a frame that isn't an ancestor of (or) one of
    /// their endpoints (G6).
    pub edge_frame_x: usize,
    /// (edge, frame) pairs of the above.
    pub edge_frame_pairs: usize,
    /// Edge-edge crossing points, away from shared endpoints (G1).
    pub edge_edge_x: usize,
    /// Edge pairs running parallel closer than [`EDGE_SPACING`] for
    /// more than [`MIN_OVERLAP_RUN`] (G1/G4).
    pub overlap_pairs: usize,
    /// Total length of those parallel runs (px).
    pub overlap_len: f64,
    /// … of the overlap pairs, those that only touch at a shared bend
    /// (closer than [`TOUCH_GAP`], too short to be a parallel run).
    pub corner_touch: usize,
    /// (edge, frame side) runs where an edge rides a frame border for
    /// more than [`BORDER_RUN_MIN`] within [`BORDER_HUG_GAP`] (G4).
    pub border_runs: usize,
    /// Total length of those runs (px).
    pub border_len: f64,
    /// Longest parallel run of one overlapping pair (px, G1).
    pub overlap_max: f64,
    /// Unrelated edge pairs running parallel within [`CLEAR_GAP`] for
    /// more than [`TIGHT_RUN_MIN`], away from shared ends (G4).
    pub tight_pairs: usize,
    /// Total length of those runs (px).
    pub tight_len: f64,
    /// Runs where an edge goes along a frame border or an unrelated
    /// node's side within [`CLEAR_GAP`] for more than
    /// [`TIGHT_RUN_MIN`], away from its own ends (G4).
    pub hug_runs: usize,
    /// Total length of those runs (px).
    pub hug_len: f64,
    /// Longest such run (px).
    pub hug_max: f64,
    /// (edge, frame title) pairs where the edge comes within
    /// `CHIP_SEP` of the title chip or the header corner it sits in
    /// (G6).
    pub edge_chip: usize,
    /// Crossings within [`PORT_ZONE`] of an end both edges share: they
    /// read as a junction, not as a crossing (G1, G9).
    pub end_cross: usize,
    /// Most crossing gaps `to_svg` cuts into one edge (G1).
    pub xgap_max: usize,
    /// Edges whose crossing cuts read as a dashed look ([`DASH_RUN`]
    /// gaps, each within [`DASH_PERIOD`] of the next, or [`DASH_RUN`]
    /// cuts, notches included, within [`DASH_WINDOW`], G1).
    pub dash_edges: usize,
    /// Crossings drawn as a notch (the over-edge's halo alone) because
    /// a full gap would have chained into a dash on either line; not
    /// counted by `xgap_max` / `dash_edges`.
    pub xnotches: usize,
    /// Edge-label pairs closer than [`LABEL_CROWD_GAP`] without
    /// overlapping (G11).
    pub lbl_crowd: usize,
    /// Edges drawn as orthogonal router paths (axis-aligned legs with
    /// rounded corners) rather than free curves: a style mix to watch.
    pub ortho_edges: usize,
    /// (edge, unrelated node) pairs closer than [`CRAMP_NODE_GAP`]
    /// without crossing (G4).
    pub node_near: usize,
    /// Smallest edge-to-unrelated-node gap among non-crossing pairs
    /// (px, higher is better; capped at [`GAP_PROBE`]).
    pub min_node_gap: f64,
    /// Smallest gap between two edges that neither cross nor share an
    /// endpoint (px, higher is better; capped at [`GAP_PROBE`]).
    pub min_edge_gap: f64,
    /// Overlapping node pairs (G2).
    pub node_overlap: usize,
    /// Node / frame pairs overlapping where the frame doesn't contain
    /// the node (G2).
    pub node_frame: usize,
    /// Overlapping frame pairs where neither nests the other (G2).
    pub frame_overlap: usize,
    /// Edge labels covering a node (G11).
    pub lbl_node: usize,
    /// Overlapping edge-label pairs (G11).
    pub lbl_lbl: usize,
    /// Edge labels covering a frame's title chip (G11).
    pub lbl_chip: usize,
    /// (label, other edge) pairs where the edge runs under the label.
    pub lbl_edge: usize,
    /// (label, edge) pairs where the label covers the arrowhead (G11).
    pub lbl_arrow: usize,
    /// Pairs of edge ends on one endpoint closer than
    /// [`PORT_MIN_GAP`] (G9).
    pub shared_ports: usize,
    /// … of which one end is an arrowhead coming in and the other an
    /// edge going out (G9).
    pub arrow_on_start: usize,
    /// Edges backtracking along the flow axis by more than
    /// [`WIGGLE_TOL`] (G8).
    pub wiggles: usize,
    /// Total backtracking (px).
    pub backtrack: f64,
    /// Edge ends on a frame that leave/enter through a side across the
    /// flow (top/bottom in LR, left/right in TD) (G7).
    pub frame_side: usize,
    /// Distance of the largest object's centre from the drawing's
    /// centre across the flow, as a fraction of the half extent: 0 =
    /// centred, 1 = at the edge (G3).
    pub center_off: f64,
    /// Canvas size as `to_svg` emits it (px).
    pub width: f64,
    pub height: f64,
    /// Long side / short side (G12).
    pub aspect: f64,
    /// Canvas area per node (px², G12).
    pub area_per_node: f64,
    /// One line per node/frame crossing, parallel overlap, close edge
    /// pair, label on a node or arrowhead, wiggle, wrong frame side and
    /// shared port, naming the elements (`from->to kind other`), for
    /// chasing a number down. Not part of [`Metrics::fields`].
    pub details: Vec<String>,
}

impl Metrics {
    /// Every metric as (column name, value, how totals fold it), in
    /// TSV column order.
    pub fn fields(&self) -> Vec<(&'static str, f64, Agg)> {
        use Agg::*;
        vec![
            ("nodes", self.nodes as f64, Sum),
            ("edges", self.edges as f64, Sum),
            ("frames", self.frames as f64, Sum),
            ("edge_node_x", self.edge_node_x as f64, Sum),
            ("edge_node_pairs", self.edge_node_pairs as f64, Sum),
            ("edge_frame_x", self.edge_frame_x as f64, Sum),
            ("edge_frame_pairs", self.edge_frame_pairs as f64, Sum),
            ("edge_edge_x", self.edge_edge_x as f64, Sum),
            ("overlap_pairs", self.overlap_pairs as f64, Sum),
            ("overlap_len", self.overlap_len, Sum),
            ("node_near", self.node_near as f64, Sum),
            ("min_node_gap", self.min_node_gap, Min),
            ("min_edge_gap", self.min_edge_gap, Min),
            ("node_overlap", self.node_overlap as f64, Sum),
            ("node_frame", self.node_frame as f64, Sum),
            ("frame_overlap", self.frame_overlap as f64, Sum),
            ("lbl_node", self.lbl_node as f64, Sum),
            ("lbl_lbl", self.lbl_lbl as f64, Sum),
            ("lbl_chip", self.lbl_chip as f64, Sum),
            ("lbl_edge", self.lbl_edge as f64, Sum),
            ("lbl_arrow", self.lbl_arrow as f64, Sum),
            ("shared_ports", self.shared_ports as f64, Sum),
            ("arrow_on_start", self.arrow_on_start as f64, Sum),
            ("wiggles", self.wiggles as f64, Sum),
            ("backtrack", self.backtrack, Sum),
            ("frame_side", self.frame_side as f64, Sum),
            ("center_off", self.center_off, Mean),
            ("width", self.width, Max),
            ("height", self.height, Max),
            ("aspect", self.aspect, Max),
            ("area_per_node", self.area_per_node, Mean),
        ]
    }

    /// Metrics added after the TSV layout was fixed, printed after the
    /// columns [`Metrics::fields`] and `defects` (and the example's
    /// `route_ms` / `deterministic`), so existing column numbers stay.
    pub fn late_fields(&self) -> Vec<(&'static str, f64, Agg)> {
        use Agg::*;
        vec![
            ("corner_touch", self.corner_touch as f64, Sum),
            ("border_runs", self.border_runs as f64, Sum),
            ("border_len", self.border_len, Sum),
            ("overlap_max", self.overlap_max, Max),
            ("tight_pairs", self.tight_pairs as f64, Sum),
            ("tight_len", self.tight_len, Sum),
            ("hug_runs", self.hug_runs as f64, Sum),
            ("hug_len", self.hug_len, Sum),
            ("hug_max", self.hug_max, Max),
            ("edge_chip", self.edge_chip as f64, Sum),
            ("end_cross", self.end_cross as f64, Sum),
            ("xgap_max", self.xgap_max as f64, Max),
            ("dash_edges", self.dash_edges as f64, Sum),
            ("lbl_crowd", self.lbl_crowd as f64, Sum),
            ("ortho_edges", self.ortho_edges as f64, Sum),
            ("xnotches", self.xnotches as f64, Sum),
        ]
    }

    /// Hard-defect count: everything a reader sees as broken (node /
    /// frame crossings, overlaps, label collisions, shared ports,
    /// wiggles, wrong frame sides). Edge-edge crossings are excluded —
    /// some are unavoidable.
    pub fn defects(&self) -> usize {
        self.edge_node_pairs
            + self.edge_frame_pairs
            + self.overlap_pairs
            + self.node_overlap
            + self.node_frame
            + self.frame_overlap
            + self.lbl_node
            + self.lbl_lbl
            + self.lbl_chip
            + self.lbl_arrow
            + self.shared_ports
            + self.wiggles
            + self.frame_side
    }
}

/// Both measurements of one diagram plus the interactive-path cost.
#[derive(Debug, Clone)]
pub struct Report {
    /// The static layout (`scene::scene`).
    pub statik: Metrics,
    /// `scene::route` fed the static node centres.
    pub routed: Metrics,
    /// Fastest of [`ROUTE_RUNS`] `route()` calls (ms; 0 when not timed).
    pub route_ms: f64,
    /// Fastest of [`ROUTE_RUNS`] `scene()` calls (ms; 0 when not timed).
    pub scene_ms: f64,
    /// Whether two renders of each path gave byte-identical SVG (G14).
    pub deterministic: bool,
    /// The two scenes, for callers that also want the SVGs.
    pub static_scene: Scene,
    pub routed_scene: Scene,
}

/// Fastest of [`ROUTE_RUNS`] calls of `f` (ms).
///
/// Uses `std::time::Instant`, which panics on wasm32-unknown-unknown:
/// native harness use only.
pub fn fastest_ms<T>(runs: usize, mut f: impl FnMut() -> T) -> f64 {
    (0..runs)
        .map(|_| {
            let t = std::time::Instant::now();
            let out = f();
            let ms = t.elapsed().as_secs_f64() * 1e3;
            drop(out);
            ms
        })
        .fold(f64::INFINITY, f64::min)
}

/// Parse a flowchart (or state diagram) and measure it both ways.
/// `time_route` = also time [`scene::route`] and [`scene::scene`]
/// (skip it in debug tests; native only, see [`fastest_ms`]).
pub fn measure_source(src: &str, time_route: bool) -> Result<Report, String> {
    let g = match parse_document(src).map_err(|e| e.to_string())? {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => return Err("not a flowchart".to_string()),
    };
    let statik = scene::scene(&g);
    let centers: Vec<(f64, f64)> = statik.nodes.iter().map(|n| (n.x, n.y)).collect();
    let routed = scene::route(&g, &centers);
    let (route_ms, scene_ms) = if time_route {
        (
            fastest_ms(ROUTE_RUNS, || scene::route(&g, &centers)),
            fastest_ms(ROUTE_RUNS, || scene::scene(&g)),
        )
    } else {
        (0.0, 0.0)
    };
    let deterministic = scene::to_svg(&statik) == scene::to_svg(&scene::scene(&g))
        && scene::to_svg(&routed) == scene::to_svg(&scene::route(&g, &centers));
    Ok(Report {
        statik: measure(&g, &statik),
        routed: measure(&g, &routed),
        route_ms,
        scene_ms,
        deterministic,
        static_scene: statik,
        routed_scene: routed,
    })
}

/// The drag benchmark's frames for a laid-out scene: per node, the
/// node centres with that node moved to each of [`DRAG_SWEEP`] points
/// on the drawing's diagonal and by [`DRAG_NUDGE`] each way, as
/// (dragged node, centres). Deterministic; time `scene::route` on each
/// (G13) with [`fastest_ms`], which is native only.
pub fn drag_frames(sc: &Scene) -> Vec<(usize, Vec<Pt>)> {
    let base: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let (x0, y0, x1, y1) = base.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)),
    );
    let mut out = Vec::new();
    for v in 0..base.len() {
        let mut at: Vec<Pt> = (0..DRAG_SWEEP)
            .map(|k| {
                let t = k as f64 / (DRAG_SWEEP - 1).max(1) as f64;
                (x0 + (x1 - x0) * t, y0 + (y1 - y0) * (1.0 - t))
            })
            .collect();
        let (bx, by) = base[v];
        at.extend([
            (bx + DRAG_NUDGE, by),
            (bx - DRAG_NUDGE, by),
            (bx, by + DRAG_NUDGE),
            (bx, by - DRAG_NUDGE),
        ]);
        for p in at {
            let mut c = base.clone();
            c[v] = p;
            out.push((v, c));
        }
    }
    out
}

type Pt = (f64, f64);

/// Axis-aligned box (x0, y0, x1, y1).
#[derive(Debug, Clone, Copy)]
struct Rect(f64, f64, f64, f64);

impl Rect {
    fn of_node(n: &SceneNode) -> Rect {
        Rect(
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.x + n.w / 2.0,
            n.y + n.h / 2.0,
        )
    }
    fn grow(self, d: f64) -> Rect {
        Rect(self.0 - d, self.1 - d, self.2 + d, self.3 + d)
    }
    fn contains(self, p: Pt) -> bool {
        p.0 > self.0 && p.0 < self.2 && p.1 > self.1 && p.1 < self.3
    }
    fn overlaps(self, o: Rect) -> bool {
        self.2.min(o.2) - self.0.max(o.0) > OVERLAP_EPS
            && self.3.min(o.3) - self.1.max(o.1) > OVERLAP_EPS
    }
    /// Distance from `p` to the box (0 inside).
    fn dist(self, p: Pt) -> f64 {
        let dx = (self.0 - p.0).max(p.0 - self.2).max(0.0);
        let dy = (self.1 - p.1).max(p.1 - self.3).max(0.0);
        dx.hypot(dy)
    }
    /// Whether segment `a`–`b` touches the box.
    fn hits_seg(self, a: Pt, b: Pt) -> bool {
        if self.contains(a) || self.contains(b) {
            return true;
        }
        let corners = [
            (self.0, self.1),
            (self.2, self.1),
            (self.2, self.3),
            (self.0, self.3),
        ];
        (0..4).any(|k| seg_cross(a, b, corners[k], corners[(k + 1) % 4]).is_some())
    }
}

/// Distance from `p` to a node's drawn outline (0 inside).
fn node_gap(n: &SceneNode, p: Pt) -> f64 {
    let (hw, hh) = (n.w / 2.0, n.h / 2.0);
    let (dx, dy) = ((p.0 - n.x).abs(), (p.1 - n.y).abs());
    match n.shape {
        Shape::Diamond if hw > 0.0 && hh > 0.0 => {
            // Distance to the rhombus face the point sits against.
            let f = dx / hw + dy / hh - 1.0;
            (f * hw * hh / hw.hypot(hh)).max(0.0)
        }
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            (dx.hypot(dy) - hw).max(0.0)
        }
        _ => Rect::of_node(n).dist(p),
    }
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Distance from `p` to segment `a`–`b`.
fn seg_dist(p: Pt, a: Pt, b: Pt) -> f64 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let l2 = vx * vx + vy * vy;
    if l2 == 0.0 {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2).clamp(0.0, 1.0);
    dist(p, (a.0 + t * vx, a.1 + t * vy))
}

/// Proper intersection point of segments `a`–`b` and `c`–`d`.
fn seg_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    let r = (b.0 - a.0, b.1 - a.1);
    let s = (d.0 - c.0, d.1 - c.1);
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-9 {
        return None;
    }
    let t = ((c.0 - a.0) * s.1 - (c.1 - a.1) * s.0) / den;
    let u = ((c.0 - a.0) * r.1 - (c.1 - a.1) * r.0) / den;
    ((0.0..1.0).contains(&t) && (0.0..1.0).contains(&u)).then_some((a.0 + t * r.0, a.1 + t * r.1))
}

/// `pts` resampled so no step exceeds [`SAMPLE_STEP`]; each sample
/// carries the unit direction of the segment it lies on.
fn densify(pts: &[Pt]) -> Vec<(Pt, Pt)> {
    let mut out = Vec::new();
    for w in pts.windows(2) {
        let l = dist(w[0], w[1]);
        if l == 0.0 {
            continue;
        }
        let dir = ((w[1].0 - w[0].0) / l, (w[1].1 - w[0].1) / l);
        let k = (l / SAMPLE_STEP).ceil().max(1.0) as usize;
        for i in 0..k {
            let t = i as f64 / k as f64;
            out.push((
                (
                    w[0].0 + t * (w[1].0 - w[0].0),
                    w[0].1 + t * (w[1].1 - w[0].1),
                ),
                dir,
            ));
        }
    }
    if let (Some(&last), Some(&(_, dir))) = (pts.last(), out.last()) {
        out.push((last, dir));
    }
    out
}

fn bounds(pts: &[Pt]) -> Rect {
    pts.iter().fold(
        Rect(
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |r, p| Rect(r.0.min(p.0), r.1.min(p.1), r.2.max(p.0), r.3.max(p.1)),
    )
}

/// Distance along the polyline `pts` (px) of its point nearest `p`.
fn arc_at(pts: &[Pt], p: Pt) -> f64 {
    let (mut acc, mut best) = (0.0, (f64::INFINITY, 0.0));
    for w in pts.windows(2) {
        let l = dist(w[0], w[1]);
        let d = seg_dist(p, w[0], w[1]);
        if d < best.0 {
            let t = if l > 0.0 {
                (((p.0 - w[0].0) * (w[1].0 - w[0].0) + (p.1 - w[0].1) * (w[1].1 - w[0].1))
                    / (l * l))
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            best = (d, acc + t * l);
        }
        acc += l;
    }
    best.1
}

/// One endpoint of a scene edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum End {
    Node(usize),
    Frame(usize),
}

/// A measured edge: drawn polyline, its dense samples, endpoints and
/// the frames it may legitimately enter.
struct Measured<'a> {
    /// Index of the edge in the scene.
    idx: usize,
    e: &'a SceneEdge,
    pts: Vec<Pt>,
    dense: Vec<(Pt, Pt)>,
    bb: Rect,
    from: Option<End>,
    to: Option<End>,
    /// Scene-cluster indices the edge may enter (ancestors of either
    /// endpoint, and an endpoint frame itself).
    allowed: Vec<usize>,
}

impl Measured<'_> {
    fn ends(&self) -> impl Iterator<Item = End> + '_ {
        self.from.into_iter().chain(self.to)
    }
}

/// Measure one scene of `g` (the static layout or a routed one).
pub fn measure(g: &Graph, sc: &Scene) -> Metrics {
    let node_idx: HashMap<&str, usize> = sc
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let frame_idx: HashMap<&str, usize> = sc
        .clusters
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.as_str(), i))
        .collect();
    let frame_rect: Vec<Rect> = sc
        .clusters
        .iter()
        .map(|c| Rect(c.x, c.y, c.x + c.w, c.y + c.h))
        .collect();
    // Scene-cluster ancestors (self included) of each subgraph, and the
    // frames enclosing each node.
    let sub_chain = |mut si: Option<usize>| {
        let mut out = Vec::new();
        while let Some(s) = si {
            if let Some(&ci) = frame_idx.get(g.subgraphs[s].id.as_str()) {
                out.push(ci);
            }
            si = g.subgraphs[s].parent;
        }
        out
    };
    let mut node_frames: Vec<Vec<usize>> = vec![Vec::new(); sc.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &n in &s.nodes {
            if let Some(v) = node_frames.get_mut(n) {
                v.extend(sub_chain(Some(si)));
            }
        }
    }
    let frame_chain: Vec<Vec<usize>> = sc
        .clusters
        .iter()
        .map(|c| sub_chain(g.subgraphs.iter().position(|s| s.id == c.id)))
        .collect();
    let end_of = |id: &str| {
        node_idx
            .get(id)
            .map(|&i| End::Node(i))
            .or_else(|| frame_idx.get(id).map(|&i| End::Frame(i)))
    };
    let frames_of = |e: Option<End>| match e {
        Some(End::Node(n)) => node_frames[n].clone(),
        Some(End::Frame(f)) => frame_chain[f].clone(),
        None => Vec::new(),
    };

    let edges: Vec<Measured> = sc
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| edge_drawn(e) && e.from != e.to)
        .map(|(idx, e)| {
            let pts = edge_polyline(e);
            let (from, to) = (end_of(&e.from), end_of(&e.to));
            let mut allowed = frames_of(from);
            allowed.extend(frames_of(to));
            Measured {
                idx,
                e,
                dense: densify(&pts),
                bb: bounds(&pts),
                pts,
                from,
                to,
                allowed,
            }
        })
        .collect();
    let end_box = |end: End| match end {
        End::Node(n) => Rect::of_node(&sc.nodes[n]),
        End::Frame(f) => frame_rect[f],
    };

    let mut m = Metrics {
        nodes: sc.nodes.len(),
        edges: edges.len(),
        frames: sc.clusters.len(),
        min_node_gap: GAP_PROBE,
        min_edge_gap: GAP_PROBE,
        ..Metrics::default()
    };

    // Frame title chips, as `to_svg` draws them.
    let mut bb = scene::Bbox::new();
    scene::grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    let (minx, maxx, miny, maxy) = bb.finish();
    let chips: Vec<Rect> = (0..sc.clusters.len())
        .map(|ci| {
            let c = &sc.clusters[ci];
            let title = scene::fit_cluster_title(&sc.clusters, ci, maxx + MARGIN);
            let (x, y) = (c.x + CHIP_INSET_X, c.y + CHIP_INSET_Y);
            Rect(
                x,
                y,
                x + text_width(&title) * TITLE_SCALE + CHIP_PAD_W,
                y + CHIP_H + title_extra(&title),
            )
        })
        .collect();
    // The headers those chips sit in, which an edge keeps
    // [`CHIP_SEP`] off (G6).
    let heads: Vec<Rect> = (0..sc.clusters.len())
        .map(|ci| {
            let h = scene::title_head(&sc.clusters, ci, maxx + MARGIN);
            Rect(h.0, h.1, h.2, h.3)
        })
        .collect();
    // Every side a line may run along: frame borders and node box sides,
    // as (horizontal, position across, span lo, span hi, owner).
    let mut sides: Vec<(bool, f64, f64, f64, End)> = Vec::new();
    let rects = frame_rect
        .iter()
        .enumerate()
        .map(|(f, &r)| (r, End::Frame(f)))
        .chain(
            sc.nodes
                .iter()
                .enumerate()
                .map(|(v, n)| (Rect::of_node(n), End::Node(v))),
        );
    for (r, owner) in rects {
        sides.extend([
            (true, r.1, r.0, r.2, owner),
            (true, r.3, r.0, r.2, owner),
            (false, r.0, r.1, r.3, owner),
            (false, r.2, r.1, r.3, owner),
        ]);
    }
    let owner_id = |e: End| match e {
        End::Node(n) => sc.nodes[n].id.as_str(),
        End::Frame(f) => sc.clusters[f].id.as_str(),
    };

    // Edge vs unrelated node / frame.
    for me in &edges {
        let mut crossed_node = false;
        for (ni, n) in sc.nodes.iter().enumerate() {
            if me.ends().any(|x| x == End::Node(ni)) {
                continue;
            }
            if !me.bb.grow(GAP_PROBE).overlaps(Rect::of_node(n)) {
                continue;
            }
            let gap = me
                .dense
                .iter()
                .map(|&(p, _)| node_gap(n, p))
                .fold(f64::INFINITY, f64::min);
            if gap < NODE_CLEARANCE {
                m.edge_node_pairs += 1;
                m.details
                    .push(format!("{}->{} edge_node {}", me.e.from, me.e.to, n.id));
                crossed_node = true;
            } else {
                m.min_node_gap = m.min_node_gap.min(gap);
                if gap < CRAMP_NODE_GAP {
                    m.node_near += 1;
                }
            }
        }
        m.edge_node_x += usize::from(crossed_node);
        let mut crossed_frame = false;
        for (fi, r) in frame_rect.iter().enumerate() {
            if me.allowed.contains(&fi) {
                continue;
            }
            let inner = r.grow(-FRAME_INSET);
            if me.dense.iter().any(|&(p, _)| inner.contains(p)) {
                m.edge_frame_pairs += 1;
                m.details.push(format!(
                    "{}->{} edge_frame {}",
                    me.e.from, me.e.to, sc.clusters[fi].id
                ));
                crossed_frame = true;
            }
        }
        m.edge_frame_x += usize::from(crossed_frame);
        // Riding a frame border (any frame, its own included): the
        // longest run of samples along each side of each frame.
        for (fi, r) in frame_rect.iter().enumerate() {
            if !me.bb.grow(BORDER_HUG_GAP).overlaps(*r) {
                continue;
            }
            for (k, side) in ["left", "right", "top", "bottom"].iter().enumerate() {
                let on = |p: Pt| match k {
                    0 => (p.0 - r.0).abs() < BORDER_HUG_GAP && p.1 > r.1 && p.1 < r.3,
                    1 => (p.0 - r.2).abs() < BORDER_HUG_GAP && p.1 > r.1 && p.1 < r.3,
                    2 => (p.1 - r.1).abs() < BORDER_HUG_GAP && p.0 > r.0 && p.0 < r.2,
                    _ => (p.1 - r.3).abs() < BORDER_HUG_GAP && p.0 > r.0 && p.0 < r.2,
                };
                let (mut cur, mut best) = (0.0f64, 0.0f64);
                let mut prev: Option<Pt> = None;
                for &(p, _) in &me.dense {
                    if on(p) {
                        if let Some(q) = prev {
                            cur += dist(p, q);
                        }
                        prev = Some(p);
                        best = best.max(cur);
                    } else {
                        cur = 0.0;
                        prev = None;
                    }
                }
                if best > BORDER_RUN_MIN {
                    m.border_runs += 1;
                    m.border_len += best;
                    m.details.push(format!(
                        "{}->{} border {} {side} {best:.0}px",
                        me.e.from, me.e.to, sc.clusters[fi].id
                    ));
                }
            }
        }
        // Cramped runs along any frame border or unrelated node side,
        // away from the edge's own ends (G4). A frame the edge ends on
        // is its own business.
        let own: Vec<Rect> = me
            .ends()
            .filter(|x| matches!(x, End::Node(_)))
            .map(end_box)
            .collect();
        let probe = me.bb.grow(CLEAR_GAP);
        for &(horiz, at, lo, hi, owner) in &sides {
            if me.ends().any(|x| x == owner) {
                continue;
            }
            let sb = if horiz {
                Rect(lo, at, hi, at)
            } else {
                Rect(at, lo, at, hi)
            };
            if !probe.overlaps(sb.grow(OVERLAP_EPS)) {
                continue;
            }
            let on = |p: Pt, d: Pt| {
                let (c, v, dv) = if horiz {
                    (p.1, p.0, d.0)
                } else {
                    (p.0, p.1, d.1)
                };
                (c - at).abs() < CLEAR_GAP
                    && v > lo
                    && v < hi
                    && dv.abs() > PARALLEL_COS
                    && own.iter().all(|r| r.dist(p) >= PORT_ZONE)
            };
            let mut cur = 0.0f64;
            let mut prev: Option<Pt> = None;
            let flush = |cur: f64, m: &mut Metrics| {
                if cur > TIGHT_RUN_MIN {
                    m.hug_runs += 1;
                    m.hug_len += cur;
                    m.hug_max = m.hug_max.max(cur);
                    m.details.push(format!(
                        "{}->{} hug {} {cur:.0}px",
                        me.e.from,
                        me.e.to,
                        owner_id(owner)
                    ));
                }
            };
            for &(p, d) in &me.dense {
                if on(p, d) {
                    if let Some(q) = prev {
                        cur += dist(p, q);
                    }
                    prev = Some(p);
                } else {
                    flush(cur, &mut m);
                    cur = 0.0;
                    prev = None;
                }
            }
            flush(cur, &mut m);
        }
        // Through a frame's title chip (G6), or past it closer than
        // [`CHIP_SEP`]; an edge ending on the frame itself only when it
        // touches the chip (its arrowhead lands on the frame's side).
        for (ci, c) in heads.iter().enumerate() {
            let reach = if me.ends().any(|x| x == End::Frame(ci)) {
                NODE_CLEARANCE
            } else {
                CHIP_SEP
            };
            if me.bb.grow(reach).overlaps(*c) && me.dense.iter().any(|&(p, _)| c.dist(p) < reach) {
                m.edge_chip += 1;
                m.details.push(format!(
                    "{}->{} edge_chip {}",
                    me.e.from, me.e.to, sc.clusters[ci].id
                ));
            }
        }
        // Drawn as an orthogonal router path: every inner leg of its
        // waypoints axis-aligned.
        let wps = &me.e.waypoints;
        let aligned =
            |w: &[Pt]| (w[0].0 - w[1].0).abs() < LINE_EPS || (w[0].1 - w[1].1).abs() < LINE_EPS;
        if wps.len() >= ORTHO_MIN_POINTS && wps[1..wps.len() - 1].windows(2).all(aligned) {
            m.ortho_edges += 1;
            m.details.push(format!("{}->{} ortho", me.e.from, me.e.to));
        }
    }

    // Edge vs edge: crossings, parallel overlap, gaps.
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let (a, b) = (&edges[i], &edges[j]);
            if !a.bb.grow(GAP_PROBE).overlaps(b.bb) {
                continue;
            }
            let shared: Vec<Rect> = a
                .ends()
                .filter(|x| b.ends().any(|y| y == *x))
                .map(end_box)
                .collect();
            let near_shared = |p: Pt| shared.iter().any(|r| r.dist(p) < PORT_ZONE);
            let mut hits: Vec<Pt> = Vec::new();
            for sa in a.pts.windows(2) {
                for sb in b.pts.windows(2) {
                    // Counted right up to a shared end too: two ends on
                    // one side in the wrong order cross on their port
                    // legs, and that is a crossing like any other (G9).
                    if let Some(p) = seg_cross(sa[0], sa[1], sb[0], sb[1]) {
                        if hits.iter().all(|&q| dist(p, q) >= CROSS_MERGE) {
                            hits.push(p);
                        }
                    }
                }
            }
            m.edge_edge_x += hits.len();
            for &p in hits.iter().filter(|&&p| near_shared(p)) {
                m.end_cross += 1;
                m.details.push(format!(
                    "{}->{} end_cross {}->{} at ({:.0},{:.0})",
                    a.e.from, a.e.to, b.e.from, b.e.to, p.0, p.1
                ));
            }
            let probe = b.bb.grow(GAP_PROBE);
            let mut run = 0.0;
            let mut tight = 0.0;
            let mut gap = f64::INFINITY;
            // Closest parallel approach away from shared ends and
            // crossings: a shared corner when below [`TOUCH_GAP`]. Two
            // crossings closer than [`TOUCH_CROSS_SKIP`] are one graze
            // of two bends, not two lines crossing: a touch too.
            let graze = hits
                .iter()
                .enumerate()
                .any(|(k, &p)| hits[k + 1..].iter().any(|&q| dist(p, q) < TOUCH_CROSS_SKIP));
            let mut touch = if graze { 0.0 } else { f64::INFINITY };
            for &(p, dir) in &a.dense {
                if !probe.contains(p) || near_shared(p) {
                    continue;
                }
                let mut best = (f64::INFINITY, (0.0, 0.0));
                for sb in b.pts.windows(2) {
                    let d = seg_dist(p, sb[0], sb[1]);
                    if d < best.0 {
                        best = (d, (sb[1].0 - sb[0].0, sb[1].1 - sb[0].1));
                    }
                }
                gap = gap.min(best.0);
                let l = best.1 .0.hypot(best.1 .1);
                let parallel =
                    l > 0.0 && ((dir.0 * best.1 .0 + dir.1 * best.1 .1) / l).abs() > PARALLEL_COS;
                if best.0 < EDGE_SPACING && parallel {
                    run += SAMPLE_STEP;
                }
                if best.0 < CLEAR_GAP
                    && parallel
                    && hits.iter().all(|&q| dist(p, q) >= TOUCH_CROSS_SKIP)
                {
                    tight += SAMPLE_STEP;
                }
                if parallel && hits.iter().all(|&q| dist(p, q) >= TOUCH_CROSS_SKIP) {
                    touch = touch.min(best.0);
                }
            }
            if tight > TIGHT_RUN_MIN {
                m.tight_pairs += 1;
                m.tight_len += tight;
                m.details.push(format!(
                    "{}->{} tight {}->{} {tight:.0}px",
                    a.e.from, a.e.to, b.e.from, b.e.to
                ));
            }
            if run > MIN_OVERLAP_RUN {
                m.overlap_pairs += 1;
                m.overlap_len += run;
                m.overlap_max = m.overlap_max.max(run);
                m.details.push(format!(
                    "{}->{} overlap {}->{} {run:.0}px",
                    a.e.from, a.e.to, b.e.from, b.e.to
                ));
            } else if touch < TOUCH_GAP {
                m.overlap_pairs += 1;
                m.corner_touch += 1;
                m.details.push(format!(
                    "{}->{} touch {}->{} {touch:.1}px",
                    a.e.from, a.e.to, b.e.from, b.e.to
                ));
            }
            if hits.is_empty() && shared.is_empty() {
                if gap < GAP_DETAIL {
                    m.details.push(format!(
                        "{}->{} edge_gap {}->{} {gap:.1}px",
                        a.e.from, a.e.to, b.e.from, b.e.to
                    ));
                }
                m.min_edge_gap = m.min_edge_gap.min(gap);
            }
        }
    }

    // Nodes and frames.
    for i in 0..sc.nodes.len() {
        let ri = Rect::of_node(&sc.nodes[i]);
        for j in i + 1..sc.nodes.len() {
            m.node_overlap += usize::from(ri.overlaps(Rect::of_node(&sc.nodes[j])));
        }
        for (fi, r) in frame_rect.iter().enumerate() {
            if !node_frames[i].contains(&fi) && ri.overlaps(*r) {
                m.node_frame += 1;
            }
        }
    }
    for i in 0..frame_rect.len() {
        for j in i + 1..frame_rect.len() {
            let nested = frame_chain[i].contains(&j) || frame_chain[j].contains(&i);
            if !nested && frame_rect[i].overlaps(frame_rect[j]) {
                m.frame_overlap += 1;
            }
        }
    }

    // Labels.
    let labels: Vec<(usize, Rect)> = edges
        .iter()
        .enumerate()
        .filter_map(|(i, me)| {
            me.e.label.as_ref().map(|(_, c, w)| {
                let r = Rect(
                    c.0 - w / 2.0,
                    c.1 - LABEL_BOX_H / 2.0,
                    c.0 + w / 2.0,
                    c.1 + LABEL_BOX_H / 2.0,
                );
                (i, r)
            })
        })
        .collect();
    for (k, &(i, r)) in labels.iter().enumerate() {
        let owner = format!("{}->{}", edges[i].e.from, edges[i].e.to);
        for n in sc.nodes.iter().filter(|n| r.overlaps(Rect::of_node(n))) {
            m.lbl_node += 1;
            m.details.push(format!("{owner} lbl_node {}", n.id));
        }
        m.lbl_lbl += labels[k + 1..]
            .iter()
            .filter(|(_, o)| r.overlaps(*o))
            .count();
        for &(j, o) in &labels[k + 1..] {
            let gx = (o.0 - r.2).max(r.0 - o.2).max(0.0);
            let gy = (o.1 - r.3).max(r.1 - o.3).max(0.0);
            if !r.overlaps(o) && gx.hypot(gy) < LABEL_CROWD_GAP {
                m.lbl_crowd += 1;
                m.details.push(format!(
                    "{owner} lbl_crowd {}->{}",
                    edges[j].e.from, edges[j].e.to
                ));
            }
        }
        m.lbl_chip += chips.iter().filter(|c| r.overlaps(**c)).count();
        for (j, me) in edges.iter().enumerate() {
            if j != i && me.pts.windows(2).any(|s| r.hits_seg(s[0], s[1])) {
                m.lbl_edge += 1;
            }
            if me.e.ends.end_marker(me.e.kind).is_some() {
                let n = me.pts.len();
                let tip = me.pts[n - 1];
                let back = me.pts[..n - 1]
                    .iter()
                    .rev()
                    .find(|&&p| dist(p, tip) >= ARROW_LEN)
                    .copied()
                    .unwrap_or(me.pts[0]);
                if r.hits_seg(back, tip) {
                    m.lbl_arrow += 1;
                    m.details
                        .push(format!("{owner} lbl_arrow {}->{}", me.e.from, me.e.to));
                }
            }
        }
    }

    // Crossing gaps as `to_svg` cuts them: per under-edge, where they
    // fall along it, and whether they chain into a dashed look (G1).
    let xs = if sc.edges.len() > crate::casing::CASING_MAX_EDGES {
        Vec::new()
    } else {
        crate::casing::crossings(sc)
    };
    let mut gaps: std::collections::BTreeMap<usize, Vec<f64>> = std::collections::BTreeMap::new();
    // A notch (the over-edge's halo alone, see `casing::crossings`)
    // is under half a gap and does not chain into a dash.
    m.xnotches = xs.iter().filter(|x| x.3).count();
    let mut cuts: std::collections::BTreeMap<usize, Vec<f64>> = std::collections::BTreeMap::new();
    for (u, _, p, notch) in xs {
        if let Some(me) = edges.iter().find(|me| me.idx == u) {
            let s = arc_at(&me.pts, p);
            cuts.entry(u).or_default().push(s);
            if !notch {
                gaps.entry(u).or_default().push(s);
            }
        }
    }
    // Any cut counts towards the window: a notch is shorter than a gap
    // but still breaks the stroke.
    let mut dense: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for (u, mut at) in cuts {
        at.sort_by(f64::total_cmp);
        let packed = at
            .windows(DASH_RUN)
            .any(|w| w[DASH_RUN - 1] - w[0] < DASH_WINDOW);
        if packed {
            dense.insert(u);
        }
    }
    for (u, mut at) in gaps {
        at.sort_by(f64::total_cmp);
        m.xgap_max = m.xgap_max.max(at.len());
        let (mut chain, mut longest) = (1usize, 1usize);
        for w in at.windows(2) {
            chain = if w[1] - w[0] < DASH_PERIOD {
                chain + 1
            } else {
                1
            };
            longest = longest.max(chain);
        }
        let packed = dense.remove(&u);
        if longest >= DASH_RUN || packed {
            m.dash_edges += 1;
            let e = &sc.edges[u];
            m.details.push(format!(
                "{}->{} dash {} gaps, {longest} chained",
                e.from,
                e.to,
                at.len()
            ));
        }
    }
    // Lines cut by notches alone.
    for u in dense {
        m.dash_edges += 1;
        let e = &sc.edges[u];
        m.details
            .push(format!("{}->{} dash notches packed", e.from, e.to));
    }

    // Ports: edge ends sharing one point on an endpoint.
    let mut by_end: HashMap<End, Vec<(Pt, bool, &str)>> = HashMap::new();
    for me in &edges {
        if let Some(f) = me.from {
            by_end
                .entry(f)
                .or_default()
                .push((me.pts[0], false, me.e.to.as_str()));
        }
        if let Some(t) = me.to {
            by_end
                .entry(t)
                .or_default()
                .push((me.pts[me.pts.len() - 1], true, me.e.from.as_str()));
        }
    }
    let mut ends: Vec<_> = by_end.into_iter().collect();
    ends.sort_by_key(|(k, _)| *k);
    for (end, v) in &ends {
        for i in 0..v.len() {
            for j in i + 1..v.len() {
                if dist(v[i].0, v[j].0) < PORT_MIN_GAP {
                    m.shared_ports += 1;
                    m.arrow_on_start += usize::from(v[i].1 != v[j].1);
                    m.details
                        .push(format!("port {} {} {}", owner_id(*end), v[i].2, v[j].2));
                }
            }
        }
    }

    // Wiggles along the flow axis.
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let along = |p: Pt| if horizontal { p.0 } else { p.1 };
    for me in &edges {
        let (mut fwd, mut back) = (0.0f64, 0.0f64);
        for w in me.pts.windows(2) {
            let d = along(w[1]) - along(w[0]);
            if d > 0.0 {
                fwd += d;
            } else {
                back -= d;
            }
        }
        let bt = fwd.min(back);
        if bt > WIGGLE_TOL {
            m.wiggles += 1;
            m.details
                .push(format!("{}->{} wiggle {bt:.1}", me.e.from, me.e.to));
            m.backtrack += bt;
        }
    }

    // Frame sides.
    for me in &edges {
        let ends = [(me.from, me.pts[0]), (me.to, me.pts[me.pts.len() - 1])];
        for (end, p) in ends {
            if let Some(End::Frame(f)) = end {
                let r = frame_rect[f];
                let dx = (p.0 - r.0).abs().min((p.0 - r.2).abs());
                let dy = (p.1 - r.1).abs().min((p.1 - r.3).abs());
                // Nearest side across the flow = wrong side.
                let across = if horizontal { dy < dx } else { dx < dy };
                if across {
                    m.frame_side += 1;
                    m.details
                        .push(format!("{}->{} frame_side", me.e.from, me.e.to));
                }
            }
        }
    }

    // Largest object centring + canvas.
    let span_lo = if horizontal { miny } else { minx };
    let span_hi = if horizontal { maxy } else { maxx };
    let objects = sc.nodes.iter().map(|n| (n.w * n.h, n.x, n.y)).chain(
        sc.clusters
            .iter()
            .filter(|c| c.depth == 0)
            .map(|c| (c.w * c.h, c.x + c.w / 2.0, c.y + c.h / 2.0)),
    );
    if let Some((_, cx, cy)) = objects.fold(None, |best: Option<(f64, f64, f64)>, o| match best {
        Some(b) if b.0 >= o.0 => Some(b),
        _ => Some(o),
    }) {
        let half = (span_hi - span_lo) / 2.0;
        if half > 0.0 {
            let c = if horizontal { cy } else { cx };
            m.center_off = ((c - (span_lo + span_hi) / 2.0).abs() / half).min(1.0);
        }
    }
    m.width = (maxx - minx) + 2.0 * MARGIN;
    m.height = (maxy - miny) + 2.0 * MARGIN;
    m.aspect = m.width.max(m.height) / m.width.min(m.height).max(1.0);
    m.area_per_node = m.width * m.height / m.nodes.max(1) as f64;
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_and_clean_cases() {
        // A straight chain: nothing to complain about.
        let r = measure_source("flowchart TD\nA-->B\nB-->C", false).unwrap();
        assert_eq!(r.statik.defects(), 0, "{:?}", r.statik);
        assert!(r.deterministic);
        // Geometry helpers.
        assert!(seg_cross((0.0, 0.0), (2.0, 2.0), (0.0, 2.0), (2.0, 0.0)).is_some());
        assert!(seg_cross((0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)).is_none());
        assert!(Rect(0.0, 0.0, 10.0, 10.0).hits_seg((-5.0, 5.0), (15.0, 5.0)));
    }

    #[test]
    fn round_two_yardsticks() {
        // Distance along a polyline to the point nearest a probe.
        let l = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        assert!((arc_at(&l, (10.0, 4.0)) - 14.0).abs() < 1e-9);
        assert!((arc_at(&l, (3.0, -1.0)) - 3.0).abs() < 1e-9);
        // Every node gets the sweep plus four nudges.
        let r = measure_source("flowchart LR\nA-->B\nB-->C", false).unwrap();
        let frames = drag_frames(&r.static_scene);
        assert_eq!(frames.len(), 3 * (DRAG_SWEEP + 4));
        assert!(frames.iter().all(|(v, c)| c.len() == 3 && *v < 3));
        // A clean chain hugs nothing, chains no gaps, crowds no label.
        let m = &r.statik;
        assert_eq!(
            (
                m.hug_runs,
                m.tight_pairs,
                m.edge_chip,
                m.dash_edges,
                m.lbl_crowd
            ),
            (0, 0, 0, 0, 0)
        );
    }
}
