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
use crate::model::{Direction, EdgeCurve, Graph, Shape};
use crate::parser::parse_document;
use crate::scene::{
    self, edge_drawn, edge_end_tangent, edge_painted_polyline, edge_stroke_width, node_outline,
    Scene, SceneEdge, SceneNode, MARGIN,
};
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
/// Arrowhead length measured back from the tip (px): the painter's one
/// head size, on every edge whatever its stroke width.
pub const ARROW_LEN: f64 = crate::scene::ARROW_MARKER_PX;
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
/// An end glyph whose drawn heading is more than this (degrees) off
/// square to the side of the node it lands on reads as askew (report
/// §2.3; M1's target is ≤ 10° for nearly every end).
pub const END_TILT_TOL: f64 = 10.0;
/// An edge end further than this (px) from the outline its endpoint is
/// painted with floats off it, or sinks into it (report §2.5).
pub const END_GAP_TOL: f64 = 1.0;
/// Two edges at least [`EDGE_SPACING`] and at most this far apart (px),
/// running parallel, still read as one cable rather than two lines
/// (report §2.13: bundles 12–26 px apart for 1100 px went unseen).
pub const PARALLEL_BAND: f64 = 26.0;
/// A parallel run within [`PARALLEL_BAND`] counts once longer than this
/// (px): about a rank gap plus a node, so two edges that stay side by
/// side past a whole rank.
pub const BAND_RUN_MIN: f64 = 100.0;
/// Control points closer than this (px) to their cubic's chord lie on
/// it, and a leg whose ends differ by less than this on one axis runs
/// along it: how a straight edge, an S and a bow are told apart.
pub const STRAIGHT_TOL: f64 = 0.5;

/// How a metric folds across many diagrams in a totals row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agg {
    Sum,
    Min,
    Max,
    Mean,
}

/// Quality numbers for one laid-out scene. Counts are "lower is
/// better" unless noted. Not exhaustive: the yardstick gains columns as
/// it learns to see more defects; read the fields, or start from
/// [`Metrics::default`].
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
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
    /// Edge pairs are pruned by bounding box, which an exactly level or
    /// plumb edge does not have: [`Metrics::x_drawn`] counts their
    /// crossings too.
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
    /// End glyphs (arrowheads, crosses, circles) whose drawn heading is
    /// more than [`END_TILT_TOL`] off square to the side they land on —
    /// the box side (top / bottom / left / right) the tip is on, the
    /// radius on a round node or state, either way across a frame's
    /// border (report §2.3). The heading as painted: an arrowhead faces
    /// along the stroke the painter cuts short under it.
    pub end_tilt: usize,
    /// Largest such angle (degrees, 0 = square onto the side).
    pub end_tilt_max: f64,
    /// Edge ends further than [`END_GAP_TOL`] from the outline their
    /// node (or frame) is painted with — a slanted, curved or capped
    /// side measured as drawn, not as its box (report §2.5).
    pub end_gaps: usize,
    /// Largest end-to-outline distance (px).
    pub end_gap_max: f64,
    /// Edges backtracking sideways (across the flow) by more than
    /// [`WIGGLE_TOL`]: hooks, hairpins and dips the flow-axis
    /// `wiggles` cannot see (report §2.13). Reversals are read with
    /// that tolerance; a back-edge (drawn against the flow) may
    /// overshoot its ends' sideways span once, on one side, to get
    /// round.
    pub lat_wiggles: usize,
    /// Total sideways backtracking of those edges (px): half the
    /// sideways travel beyond the net sideways shift, less a
    /// back-edge's one overshoot.
    pub lat_backtrack: f64,
    /// Edges with both ends inside a frame that leave the innermost
    /// frame holding both ends by more than [`FRAME_INSET`] (report
    /// §2.2 #5, §2.8).
    pub own_frame_x: usize,
    /// Total drawn length outside that frame (px).
    pub own_frame_len: f64,
    /// Edge pairs running parallel [`EDGE_SPACING`]..=[`PARALLEL_BAND`]
    /// apart for more than [`BAND_RUN_MIN`], away from shared ends and
    /// crossings: long cables `tight_pairs` (under [`CLEAR_GAP`]) does
    /// not see.
    pub band_pairs: usize,
    /// Total length of those runs (px).
    pub band_len: f64,
    /// Share of the canvas covered by ink (higher = denser): node
    /// shapes, edge label boxes and edge strokes (length × stroke
    /// width), over `width` × `height`. Frames are containers: the
    /// empty room they enclose is what this exposes.
    pub ink: f64,
    /// Edges drawn in the orthogonal idiom: axis-aligned legs with
    /// rounded corners (a router path, a step curve) or one straight
    /// axis-aligned stroke. The four idiom counts partition `edges`.
    pub idiom_ortho: usize,
    /// … as one free cubic between their ends (the adjacent-rank S, a
    /// diagonal; report §2.1).
    pub idiom_scubic: usize,
    /// … as a spline through waypoints that are not an orthogonal
    /// path (a dummy chain's curveBasis; report §2.1).
    pub idiom_spline: usize,
    /// … as one cubic bowing to one side of its chord, both control
    /// points beyond it (the back-edge bow; report §2.2).
    pub idiom_bow: usize,
    /// Edge-edge crossing points as drawn, counted like `edge_edge_x`
    /// but over every pair of edges: that one's box prune skips an
    /// exactly level or plumb edge (an aligned spine), and with it the
    /// crossings a reader sees on it (report §2.13).
    pub x_drawn: usize,
    /// End glyphs whose stroke meets the glyph's base — a head's length
    /// ([`ARROW_LEN`]) back from the tip, as painted — more than
    /// [`END_TILT_TOL`] off square to the side, as `end_tilt` judges it: a
    /// curve still turning under the head kinks at its base however
    /// square the head itself (report §2.3, M1: the last head length runs
    /// straight).
    pub end_kink: usize,
    /// Drawn edges with ink off the canvas: a stroke as painted (half its
    /// width out) or an end glyph's box outside the drawing's bounds
    /// plus the padding `to_svg` gives them, i.e. clipped by the SVG's
    /// view box (B1 review). Gated at 0.
    pub ink_out: usize,
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

    /// Metrics added after the timing columns (report M0, §2.13: the
    /// defects the earlier columns cannot see), printed after them —
    /// at the very end of a row — so every earlier column keeps its
    /// number.
    pub fn tail_fields(&self) -> Vec<(&'static str, f64, Agg)> {
        use Agg::*;
        vec![
            ("end_tilt", self.end_tilt as f64, Sum),
            ("end_tilt_max", self.end_tilt_max, Max),
            ("end_gaps", self.end_gaps as f64, Sum),
            ("end_gap_max", self.end_gap_max, Max),
            ("lat_wiggles", self.lat_wiggles as f64, Sum),
            ("lat_backtrack", self.lat_backtrack, Sum),
            ("own_frame_x", self.own_frame_x as f64, Sum),
            ("own_frame_len", self.own_frame_len, Sum),
            ("band_pairs", self.band_pairs as f64, Sum),
            ("band_len", self.band_len, Sum),
            ("ink", self.ink, Mean),
            ("idiom_ortho", self.idiom_ortho as f64, Sum),
            ("idiom_scubic", self.idiom_scubic as f64, Sum),
            ("idiom_spline", self.idiom_spline as f64, Sum),
            ("idiom_bow", self.idiom_bow as f64, Sum),
            ("x_drawn", self.x_drawn as f64, Sum),
            ("end_kink", self.end_kink as f64, Sum),
            ("ink_out", self.ink_out as f64, Sum),
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

/// Distance from `p` to the boundary of the closed polygon `poly`
/// (inside or out).
fn poly_dist(poly: &[Pt], p: Pt) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| seg_dist(p, poly[i], poly[(i + 1) % n]))
        .fold(f64::INFINITY, f64::min)
}

/// Area of the closed polygon `poly` (shoelace).
fn poly_area(poly: &[Pt]) -> f64 {
    let n = poly.len();
    let twice: f64 = (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    twice.abs() / 2.0
}

/// The heading (unit vector, into the end) of the drawn polyline `pts`
/// where it meets an end glyph's base: on the piece holding the point a
/// head's length ([`ARROW_LEN`]) of arc back from the
/// end at the source (`at_start`) or the target, ties going to the piece
/// nearer the end (a stub exactly a head long is its own heading).
/// `None` for a line shorter than a head.
fn head_base_heading(pts: &[Pt], at_start: bool) -> Option<Pt> {
    let mut left = ARROW_LEN;
    let n = pts.len();
    for k in 0..n.saturating_sub(1) {
        // The k-th piece from the end, as (farther, nearer) point.
        let (far, near) = if at_start {
            (pts[k + 1], pts[k])
        } else {
            (pts[n - 2 - k], pts[n - 1 - k])
        };
        let d = dist(far, near);
        if d >= left - 1e-6 && d > 0.0 {
            return Some(((near.0 - far.0) / d, (near.1 - far.1) / d));
        }
        left -= d;
    }
    None
}

/// Inward unit normal of the side of the box centred at `c` with half
/// extents `half` that `tip` lands on, judged relative to the half
/// extents — so a diamond's or a hexagon's face counts as the box side
/// it faces, the side an orthogonal edge enters square.
fn box_side_in(c: Pt, half: Pt, tip: Pt) -> Pt {
    let u = (tip.0 - c.0) / half.0.max(f64::EPSILON);
    let v = (tip.1 - c.1) / half.1.max(f64::EPSILON);
    if u.abs() > v.abs() {
        (-u.signum(), 0.0)
    } else {
        (0.0, -v.signum())
    }
}

/// Inward unit normal of the side of node `n` that `tip` lands on: the
/// radius on a round node or state (an end aims at its centre), else
/// the box side ([`box_side_in`]). `(0, 0)` at a circle's very centre.
fn side_in(n: &SceneNode, tip: Pt) -> Pt {
    match n.shape {
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            let (dx, dy) = (n.x - tip.0, n.y - tip.1);
            let l = dx.hypot(dy);
            if l < f64::EPSILON {
                (0.0, 0.0)
            } else {
                (dx / l, dy / l)
            }
        }
        _ => box_side_in((n.x, n.y), (n.w / 2.0, n.h / 2.0), tip),
    }
}

/// How an edge is drawn (see [`Metrics::idiom_ortho`] and its
/// siblings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Idiom {
    Ortho,
    SCubic,
    Spline,
    Bow,
}

/// The idiom edge `e` is drawn in, read off the geometry `to_svg`
/// strokes: its waypoints (an orthogonal router path when its inner
/// legs are axis-aligned, as `ortho_edges` counts it), a mermaid step
/// curve, or its one cubic — straight on an axis, bowing to one side
/// of its chord (both control points beyond it on the same side), or
/// else a free S.
fn idiom(e: &SceneEdge) -> Idiom {
    let along_axis = |a: Pt, b: Pt, tol: f64| (a.0 - b.0).abs() <= tol || (a.1 - b.1).abs() <= tol;
    let stepped = matches!(
        e.meta.curve,
        Some(EdgeCurve::Step | EdgeCurve::StepBefore | EdgeCurve::StepAfter)
    );
    let wps = &e.waypoints;
    let n = wps.len();
    if n >= 2 {
        let ortho = stepped
            || if n >= ORTHO_MIN_POINTS {
                wps[1..n - 1]
                    .windows(2)
                    .all(|w| along_axis(w[0], w[1], LINE_EPS))
            } else {
                wps.windows(2).all(|w| along_axis(w[0], w[1], STRAIGHT_TOL))
            };
        return if ortho { Idiom::Ortho } else { Idiom::Spline };
    }
    let [p0, c1, c2, p3] = e.bezier;
    if stepped {
        return Idiom::Ortho;
    }
    if e.meta.curve.is_some_and(|c| c != EdgeCurve::Basis) {
        // A mermaid curve through the two ends: a straight line, or a
        // bump that is straight between ends on one axis.
        return if along_axis(p0, p3, STRAIGHT_TOL) {
            Idiom::Ortho
        } else {
            Idiom::SCubic
        };
    }
    let on = |q: Pt, k: usize| {
        if k == 0 {
            (q.0 - p0.0).abs() <= STRAIGHT_TOL
        } else {
            (q.1 - p0.1).abs() <= STRAIGHT_TOL
        }
    };
    if (0..2).any(|k| on(c1, k) && on(c2, k) && on(p3, k)) {
        return Idiom::Ortho;
    }
    let (dx, dy) = (p3.0 - p0.0, p3.1 - p0.1);
    let l = dx.hypot(dy);
    if l < f64::EPSILON {
        return Idiom::SCubic;
    }
    let side = |q: Pt| (dx * (q.1 - p0.1) - dy * (q.0 - p0.0)) / l;
    let (s1, s2) = (side(c1), side(c2));
    if s1.abs() > STRAIGHT_TOL && s2.abs() > STRAIGHT_TOL && s1.signum() == s2.signum() {
        Idiom::Bow
    } else {
        Idiom::SCubic
    }
}

/// The turning points of `v`: its first value, then the extreme of
/// each monotone run, a change of direction read only once `v` has come
/// back more than `tol` from the run's extreme (so sampling noise and
/// sub-`tol` jitter turn nothing).
fn zigzag(v: &[f64], tol: f64) -> Vec<f64> {
    let Some(&first) = v.first() else {
        return Vec::new();
    };
    let mut turns = vec![first];
    // The current run's extreme so far and its direction (0 until `v`
    // first strays more than `tol` from where it started).
    let (mut ext, mut dir) = (first, 0.0f64);
    for &x in &v[1..] {
        if dir == 0.0 {
            if (x - first).abs() > tol {
                (ext, dir) = (x, (x - first).signum());
            }
        } else if (x - ext) * dir > 0.0 {
            ext = x;
        } else if (ext - x) * dir > tol {
            turns.push(ext);
            (ext, dir) = (x, -dir);
        }
    }
    if dir != 0.0 {
        turns.push(ext);
    }
    turns
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
    // A member's edge to its own box is drawn to the box's entry member
    // (`scene::own_box_targets`), and ends there: (member id, box id,
    // entry member) of each.
    let own_box: Vec<(&str, &str, usize)> = g
        .sub_edges
        .iter()
        .zip(scene::own_box_targets(g))
        .filter_map(|(e, m)| match (e.from, e.to, m) {
            (crate::model::End::Node(v), crate::model::End::Sub(s), Some(m)) => {
                Some((g.nodes[v].id.as_str(), g.subgraphs[s].id.as_str(), m))
            }
            _ => None,
        })
        .collect();
    let to_of = |e: &SceneEdge| {
        own_box
            .iter()
            .find(|o| o.0 == e.from && o.1 == e.to)
            .and_then(|o| node_idx.get(g.nodes[o.2].id.as_str()))
            .map(|&i| End::Node(i))
            .or_else(|| end_of(&e.to))
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
            // The stroke as painted (a cubic's straight glyph stubs
            // included), as the casing cuts its gaps in it.
            let pts = edge_painted_polyline(e);
            let (from, to) = (end_of(&e.from), to_of(e));
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
    scene::grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
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
        let (count, name) = match idiom(me.e) {
            Idiom::Ortho => (&mut m.idiom_ortho, None),
            Idiom::SCubic => (&mut m.idiom_scubic, Some("scubic")),
            Idiom::Spline => (&mut m.idiom_spline, Some("spline")),
            Idiom::Bow => (&mut m.idiom_bow, Some("bow")),
        };
        *count += 1;
        if let Some(name) = name {
            m.details
                .push(format!("{}->{} idiom {name}", me.e.from, me.e.to));
        }
    }

    // Edge vs edge: crossings, parallel overlap, gaps.
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let (a, b) = (&edges[i], &edges[j]);
            // `b`'s box grown a hair: a straight axis-aligned edge has
            // a box of no width, which overlaps nothing.
            if !a.bb.grow(GAP_PROBE).overlaps(b.bb.grow(OVERLAP_EPS)) {
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
            m.x_drawn += hits.len();
            // The metrics older than `x_drawn` keep their own pair prune
            // by `b`'s bare box, so they read as they always have: an
            // exactly level or plumb `b` is not measured against.
            let legacy = a.bb.grow(GAP_PROBE).overlaps(b.bb);
            if legacy {
                m.edge_edge_x += hits.len();
                for &p in hits.iter().filter(|&&p| near_shared(p)) {
                    m.end_cross += 1;
                    m.details.push(format!(
                        "{}->{} end_cross {}->{} at ({:.0},{:.0})",
                        a.e.from, a.e.to, b.e.from, b.e.to, p.0, p.1
                    ));
                }
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
            let mut band = 0.0;
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
                    if (EDGE_SPACING..=PARALLEL_BAND).contains(&best.0) {
                        band += SAMPLE_STEP;
                    }
                }
            }
            if band > BAND_RUN_MIN {
                m.band_pairs += 1;
                m.band_len += band;
                m.details.push(format!(
                    "{}->{} band {}->{} {band:.0}px",
                    a.e.from, a.e.to, b.e.from, b.e.to
                ));
            }
            if !legacy {
                continue;
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

    // Ends against the outlines as painted, and the heading of each end
    // glyph against the side it lands on (report M0, §2.3 / §2.5).
    let outlines: Vec<Vec<Pt>> = sc.nodes.iter().map(node_outline).collect();
    for me in &edges {
        for (end, at_start) in [(me.from, true), (me.to, false)] {
            let Some(end) = end else { continue };
            let (tip, head) = edge_end_tangent(me.e, at_start);
            let (gap, normal) = match end {
                End::Node(v) => (poly_dist(&outlines[v], tip), side_in(&sc.nodes[v], tip)),
                End::Frame(f) => {
                    let r = frame_rect[f];
                    let poly = [(r.0, r.1), (r.2, r.1), (r.2, r.3), (r.0, r.3)];
                    let (c, half) = (
                        ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0),
                        ((r.2 - r.0) / 2.0, (r.3 - r.1) / 2.0),
                    );
                    (poly_dist(&poly, tip), box_side_in(c, half, tip))
                }
            };
            let which = if at_start { "start" } else { "end" };
            m.end_gap_max = m.end_gap_max.max(gap);
            if gap > END_GAP_TOL {
                m.end_gaps += 1;
                m.details.push(format!(
                    "{}->{} end_gap {which} on {} {gap:.1}px",
                    me.e.from,
                    me.e.to,
                    owner_id(end)
                ));
            }
            let glyph = if at_start {
                me.e.ends.start_marker(me.e.kind)
            } else {
                me.e.ends.end_marker(me.e.kind)
            };
            if glyph.is_none() || head == (0.0, 0.0) || normal == (0.0, 0.0) {
                continue;
            }
            // Off square, in degrees; a frame is reached from inside as
            // well as from out, so either way across its border is square.
            let off = |h: Pt| {
                let c = h.0 * normal.0 + h.1 * normal.1;
                let c = if matches!(end, End::Frame(_)) {
                    c.abs()
                } else {
                    c
                };
                c.clamp(-1.0, 1.0).acos().to_degrees()
            };
            let tilt = off(head);
            m.end_tilt_max = m.end_tilt_max.max(tilt);
            if tilt > END_TILT_TOL {
                m.end_tilt += 1;
                m.details.push(format!(
                    "{}->{} end_tilt {which} on {} {tilt:.1}deg",
                    me.e.from,
                    me.e.to,
                    owner_id(end)
                ));
            }
            let base = head_base_heading(&me.pts, at_start);
            if let Some(kink) = base.map(off).filter(|&k| k > END_TILT_TOL) {
                m.end_kink += 1;
                m.details.push(format!(
                    "{}->{} end_kink {which} on {} {kink:.1}deg",
                    me.e.from,
                    me.e.to,
                    owner_id(end)
                ));
            }
        }
    }

    // Sideways reversals (report §2.13): what `wiggles` measures along
    // the flow, across it. A back-edge keeps its deepest excursion.
    let across = |p: Pt| if horizontal { p.1 } else { p.0 };
    let flow = if matches!(g.direction, Direction::BT | Direction::RL) {
        -1.0
    } else {
        1.0
    };
    for me in &edges {
        let lat: Vec<f64> = me.pts.iter().map(|&p| across(p)).collect();
        let turns = zigzag(&lat, WIGGLE_TOL);
        let (c0, c1) = (turns[0], turns[turns.len() - 1]);
        let travel: f64 = turns.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        // Half the sideways travel beyond the net shift: what `wiggles`
        // takes along the flow (the lesser of the two ways' totals).
        let mut lb = (travel - (c1 - c0).abs()) / 2.0;
        let (p0, p1) = (me.pts[0], me.pts[me.pts.len() - 1]);
        if (along(p1) - along(p0)) * flow < -WIGGLE_TOL {
            // A back-edge gets round on one side: its overshoot beyond
            // its ends' sideways span there is no wiggle.
            let (hi, lo) = turns
                .iter()
                .fold((c0, c0), |b, &c| (b.0.max(c), b.1.min(c)));
            lb -= (hi - c0.max(c1)).max(c0.min(c1) - lo);
        }
        if lb > WIGGLE_TOL {
            m.lat_wiggles += 1;
            m.lat_backtrack += lb;
            m.details
                .push(format!("{}->{} lat_wiggle {lb:.1}", me.e.from, me.e.to));
        }
    }

    // Leaving the innermost frame that holds both ends (report §2.2 #5):
    // the frames enclosing each end (a frame end's own excluded — the
    // edge lands on its border).
    let holders = |e: Option<End>| -> Vec<usize> {
        match e {
            Some(End::Node(n)) => node_frames[n].clone(),
            Some(End::Frame(f)) => frame_chain[f].iter().copied().filter(|&c| c != f).collect(),
            None => Vec::new(),
        }
    };
    for me in &edges {
        let (a, b) = (holders(me.from), holders(me.to));
        let home = a
            .iter()
            .copied()
            .filter(|c| b.contains(c))
            .max_by_key(|&c| sc.clusters[c].depth);
        let Some(home) = home else { continue };
        let r = frame_rect[home];
        let mut out = 0.0;
        for w in me.dense.windows(2) {
            let (p, q) = (w[0].0, w[1].0);
            if r.dist(((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0)) > FRAME_INSET {
                out += dist(p, q);
            }
        }
        if out > 0.0 {
            m.own_frame_x += 1;
            m.own_frame_len += out;
            m.details.push(format!(
                "{}->{} own_frame {} {out:.0}px",
                me.e.from, me.e.to, sc.clusters[home].id
            ));
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
    // Ink: shapes as painted, label boxes, strokes (length × width).
    let mut ink: f64 = outlines.iter().map(|o| poly_area(o)).sum();
    ink += labels
        .iter()
        .map(|(_, r)| (r.2 - r.0) * (r.3 - r.1))
        .sum::<f64>();
    for e in sc.edges.iter().filter(|e| edge_drawn(e)) {
        let pts = edge_painted_polyline(e);
        let len: f64 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
        ink += len * edge_stroke_width(e);
    }
    m.ink = ink / (m.width * m.height).max(1.0);
    // Ink off the canvas `to_svg` writes: the drawing's bounds and the
    // padding round them.
    let (x0, x1, y0, y1) = (minx - MARGIN, maxx + MARGIN, miny - MARGIN, maxy + MARGIN);
    for e in sc.edges.iter().filter(|e| edge_drawn(e)) {
        let half = edge_stroke_width(e) / 2.0;
        let stroke = edge_painted_polyline(e).into_iter().map(|p| (p, half));
        let glyphs = scene::painted_glyph_corners(e)
            .into_iter()
            .map(|p| (p, 0.0));
        let out = stroke
            .chain(glyphs)
            .map(|((x, y), pad)| {
                (x0 - (x - pad))
                    .max(y0 - (y - pad))
                    .max(x + pad - x1)
                    .max(y + pad - y1)
            })
            .fold(f64::NEG_INFINITY, f64::max);
        if out > 0.0 {
            m.ink_out += 1;
            m.details
                .push(format!("{}->{} ink_out {out:.1}px", e.from, e.to));
        }
    }
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

    /// A flowchart and its static scene, for hand-drawn edge geometry.
    fn laid(src: &str) -> (Graph, Scene) {
        let g = match parse_document(src).unwrap() {
            Document::Flowchart(g) => g,
            _ => unreachable!("a flowchart"),
        };
        let sc = scene::scene(&g);
        (g, sc)
    }

    /// Redraw `e` as straight segments through `pts` exactly (mermaid's
    /// `linear` curve), so a test knows every drawn point.
    fn draw_linear(e: &mut SceneEdge, pts: &[Pt]) {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        e.waypoints = pts.to_vec();
        e.bezier = [a, a, b, b];
        e.meta.curve = Some(EdgeCurve::Linear);
    }

    /// Redraw `e` as the one default cubic `b`.
    fn draw_cubic(e: &mut SceneEdge, b: [Pt; 4]) {
        e.waypoints.clear();
        e.bezier = b;
        e.meta.curve = None;
    }

    fn bottom(n: &SceneNode) -> Pt {
        (n.x, n.y + n.h / 2.0)
    }

    fn top(n: &SceneNode) -> Pt {
        (n.x, n.y - n.h / 2.0)
    }

    #[test]
    fn end_tilt_reads_the_drawn_heading_against_the_side() {
        let (g, mut sc) = laid("flowchart TD\nA-->B");
        // As laid out: one straight spine, square onto both sides.
        let m = measure(&g, &sc);
        assert_eq!((m.end_tilt, m.end_tilt_max), (0, 0.0), "{m:?}");
        // The arrowhead comes in at 45° from the upper left onto B's
        // top side; the start (no glyph) is not judged.
        let (p0, p3) = (bottom(&sc.nodes[0]), top(&sc.nodes[1]));
        let c2 = (p3.0 - 30.0, p3.1 - 30.0);
        draw_cubic(&mut sc.edges[0], [p0, (p0.0, p0.1 + 20.0), c2, p3]);
        let m = measure(&g, &sc);
        assert_eq!(m.end_tilt, 1, "{m:?}");
        assert!((m.end_tilt_max - 45.0).abs() < 1e-6, "{}", m.end_tilt_max);
        assert_eq!(m.end_kink, 1, "{m:?}");
        // A line still turning under the head kinks at its base however
        // square the head: an S whose last 6 px drop straight onto the
        // side meets the head's base (a head's length up) at 45°.
        let tip = top(&sc.nodes[1]);
        let mid = ((p0.1 + tip.1) / 2.0, tip.1 - 6.0);
        draw_linear(
            &mut sc.edges[0],
            &[
                p0,
                (p0.0, mid.0),
                (tip.0 - 20.0, mid.1 - 20.0),
                (tip.0, mid.1),
                tip,
            ],
        );
        let m = measure(&g, &sc);
        assert_eq!((m.end_tilt, m.end_kink), (0, 1), "{m:?}");
        // The heading is the drawn one: a polyline whose last leg drops
        // straight onto the side is square, however it got there.
        let mid = (p3.0 - 40.0, (p0.1 + p3.1) / 2.0);
        draw_linear(&mut sc.edges[0], &[p0, mid, (p3.0, mid.1), p3]);
        assert_eq!(measure(&g, &sc).end_tilt, 0);
        // Onto a circle an end aims at the centre: radial is square,
        // straight down beside the centre is not.
        let (g, mut sc) = laid("flowchart TD\nA-->B((round))");
        let (p0, c) = (bottom(&sc.nodes[0]), (sc.nodes[1].x, sc.nodes[1].y));
        let r = sc.nodes[1].w / 2.0;
        let off = r * 0.5;
        let tip = (c.0 + off, c.1 - (r * r - off * off).sqrt());
        draw_linear(&mut sc.edges[0], &[p0, (tip.0, p0.1), tip]);
        let m = measure(&g, &sc);
        assert_eq!(m.end_tilt, 1, "{m:?}");
        assert!((m.end_tilt_max - 30.0).abs() < 1e-6, "{}", m.end_tilt_max);
        assert!(m.end_gap_max < END_GAP_TOL, "{}", m.end_gap_max);
    }

    #[test]
    fn end_gaps_measure_the_painted_outline() {
        // Every shape's own ends, as laid out, sit on the outline or
        // the metric says by how much (the slanted and capped shapes
        // are the known offenders, report §2.5); a plain chain is clean.
        let (g, sc) = laid("flowchart TD\nA-->B(r)-->C([s])-->D((c))-->E{d}");
        let m = measure(&g, &sc);
        assert_eq!(m.end_gaps, 0, "{m:?}");
        // An end at a hexagon's box corner floats off its slanted face
        // by the corner's distance to that face.
        let (g, mut sc) = laid("flowchart TD\nA-->B{{hexagon node}}");
        let n = &sc.nodes[1];
        let (l, t, hh) = (n.x - n.w / 2.0, n.y - n.h / 2.0, n.h / 2.0);
        let k = 14.0f64.min(n.w / 4.0);
        let want = k * hh / k.hypot(hh);
        let p0 = bottom(&sc.nodes[0]);
        draw_linear(&mut sc.edges[0], &[p0, (l, p0.1), (l, t)]);
        let m = measure(&g, &sc);
        assert_eq!(m.end_gaps, 1, "{m:?}");
        assert!(
            (m.end_gap_max - want).abs() < 1e-6,
            "{} vs {want}",
            m.end_gap_max
        );
        // A cylinder's top is its cap: the box's top corner is off it,
        // the cap's crown is on it.
        let (g, mut sc) = laid("flowchart TD\nA-->B[(database)]");
        let n = &sc.nodes[1];
        let (l, t, crown) = (n.x - n.w / 2.0, n.y - n.h / 2.0, top(n));
        let p0 = bottom(&sc.nodes[0]);
        draw_linear(&mut sc.edges[0], &[p0, (l + 1.0, p0.1), (l + 1.0, t)]);
        assert_eq!(measure(&g, &sc).end_gaps, 1);
        draw_linear(&mut sc.edges[0], &[p0, crown]);
        let m = measure(&g, &sc);
        assert_eq!(m.end_gaps, 0, "{m:?}");
        assert!(m.end_gap_max < 1e-9, "{}", m.end_gap_max);
    }

    #[test]
    fn sideways_hairpins_are_wiggles_a_back_edges_lane_is_not() {
        let (g, mut sc) = laid("flowchart TD\nA-->B\nB-->A");
        let m = measure(&g, &sc);
        assert_eq!(m.lat_wiggles, 0, "{m:?}");
        let (a, b) = (bottom(&sc.nodes[0]), top(&sc.nodes[1]));
        let (y1, y2) = (a.1 + 10.0, a.1 + 20.0);
        // Forward A->B: 30 px right, 30 px back left, then down. Monotone
        // along the flow, so the flow-axis `wiggles` cannot see it.
        draw_linear(
            &mut sc.edges[0],
            &[
                a,
                (a.0, y1),
                (a.0 + 30.0, y1),
                (a.0 + 30.0, y2),
                (a.0, y2),
                b,
            ],
        );
        // B->A up a lane 60 px to the side: one excursion, allowed.
        let (bs, ae) = ((b.0 + 8.0, b.1), (a.0 + 8.0, a.1));
        let lane = [
            bs,
            (bs.0, bs.1 - 4.0),
            (bs.0 + 60.0, bs.1 - 4.0),
            (bs.0 + 60.0, ae.1 + 4.0),
            (ae.0, ae.1 + 4.0),
            ae,
        ];
        draw_linear(&mut sc.edges[1], &lane);
        let m = measure(&g, &sc);
        assert_eq!(m.wiggles, 0, "{m:?}");
        assert_eq!(m.lat_wiggles, 1, "{m:?}");
        assert!((m.lat_backtrack - 30.0).abs() < 1e-9, "{}", m.lat_backtrack);
        // A second, 20 px hook on the lane is one excursion too many.
        let mut hooked = lane.to_vec();
        let y = (bs.1 + ae.1) / 2.0;
        hooked.splice(
            3..3,
            [
                (bs.0 + 60.0, y),
                (bs.0 + 40.0, y),
                (bs.0 + 40.0, y - 4.0),
                (bs.0 + 60.0, y - 4.0),
            ],
        );
        draw_linear(&mut sc.edges[1], &hooked);
        let m = measure(&g, &sc);
        assert_eq!(m.lat_wiggles, 2, "{m:?}");
        assert!((m.lat_backtrack - 50.0).abs() < 1e-9, "{}", m.lat_backtrack);
        // Runs read with the tolerance: jitter below it is no run.
        assert_eq!(
            zigzag(&[0.0, 10.0, 5.0, 20.0], WIGGLE_TOL),
            [0.0, 10.0, 5.0, 20.0]
        );
        assert_eq!(zigzag(&[0.0, 10.0, 8.0, 20.0], WIGGLE_TOL), [0.0, 20.0]);
        assert_eq!(zigzag(&[0.0, 2.0, -1.0], WIGGLE_TOL), [0.0]);
    }

    #[test]
    fn leaving_the_frame_both_ends_share_is_an_escape() {
        let (g, mut sc) = laid("flowchart TD\nsubgraph S\nA-->B\nend\nC-->A");
        let m = measure(&g, &sc);
        assert_eq!((m.own_frame_x, m.own_frame_len), (0, 0.0), "{m:?}");
        let f = &sc.clusters[0];
        let right = f.x + f.w;
        let (a, b) = (bottom(&sc.nodes[0]), top(&sc.nodes[1]));
        let (y1, y2) = (a.1 + 5.0, b.1 - 5.0);
        // Out 20 px past S's right border and back, inside it again.
        draw_linear(
            &mut sc.edges[0],
            &[
                a,
                (a.0, y1),
                (right + 20.0, y1),
                (right + 20.0, y2),
                (a.0, y2),
                b,
            ],
        );
        let m = measure(&g, &sc);
        assert_eq!(m.own_frame_x, 1, "{m:?}");
        let want = 2.0 * 19.0 + (y2 - y1);
        assert!(
            (m.own_frame_len - want).abs() <= 2.0 * SAMPLE_STEP,
            "{}",
            m.own_frame_len
        );
        // C->A has one end outside S: it may (must) cross the border.
        assert!(m.details.iter().all(|d| !d.starts_with("C->A own_frame")));
    }

    #[test]
    fn long_parallel_cables_and_idioms() {
        let (g, mut sc) = laid("flowchart TD\nA-->B\nC-->D");
        let m = measure(&g, &sc);
        assert_eq!((m.idiom_ortho, m.band_pairs), (2, 0), "{m:?}");
        // Two lines side by side for 200 px, 10 px apart: one cable.
        let (x, y) = (sc.nodes[0].x, sc.nodes[0].y);
        draw_linear(&mut sc.edges[0], &[(x, y), (x, y + 200.0)]);
        draw_linear(&mut sc.edges[1], &[(x + 10.0, y), (x + 10.0, y + 200.0)]);
        let m = measure(&g, &sc);
        assert_eq!(m.band_pairs, 1, "{m:?}");
        assert!(
            (m.band_len - 200.0).abs() <= 2.0 * SAMPLE_STEP,
            "{}",
            m.band_len
        );
        // Further apart than the band they read as two lines.
        draw_linear(&mut sc.edges[1], &[(x + 30.0, y), (x + 30.0, y + 200.0)]);
        assert_eq!(measure(&g, &sc).band_pairs, 0);
        // The idioms: an S, a bow, a spline through a dummy chain, and
        // an orthogonal path; together they are every edge.
        let s = [(0.0, 0.0), (0.0, 30.0), (50.0, 30.0), (50.0, 60.0)];
        draw_cubic(&mut sc.edges[0], s);
        assert_eq!(idiom(&sc.edges[0]), Idiom::SCubic);
        let bow = [(0.0, 100.0), (80.0, 60.0), (80.0, 40.0), (0.0, 0.0)];
        draw_cubic(&mut sc.edges[0], bow);
        assert_eq!(idiom(&sc.edges[0]), Idiom::Bow);
        let mut chain = sc.edges[0].clone();
        chain.waypoints = vec![(0.0, 0.0), (10.0, 40.0), (30.0, 80.0), (30.0, 120.0)];
        assert_eq!(idiom(&chain), Idiom::Spline);
        chain.waypoints =
            crate::router::smooth(&[(0.0, 0.0), (0.0, 40.0), (30.0, 40.0), (30.0, 120.0)], 6.0);
        assert_eq!(idiom(&chain), Idiom::Ortho);
        let m = measure(&g, &sc);
        assert_eq!((m.idiom_bow, m.idiom_ortho), (1, 1), "{m:?}");
        assert_eq!(
            m.idiom_ortho + m.idiom_scubic + m.idiom_spline + m.idiom_bow,
            m.edges
        );
    }

    #[test]
    fn drawn_crossings_include_level_and_plumb_edges() {
        let (g, mut sc) = laid("flowchart TD\nA-->B\nC-->D");
        let (x, y) = (sc.nodes[0].x, sc.nodes[0].y);
        // A plumb line crossed by a level one: neither has a box with
        // any width, so the older count's prune never pairs them.
        draw_linear(&mut sc.edges[0], &[(x, y), (x, y + 200.0)]);
        draw_linear(
            &mut sc.edges[1],
            &[(x - 50.0, y + 100.0), (x + 50.0, y + 100.0)],
        );
        let m = measure(&g, &sc);
        assert_eq!((m.edge_edge_x, m.x_drawn), (0, 1), "{m:?}");
        // A curve has a box: both counts see it cross.
        draw_cubic(
            &mut sc.edges[1],
            [
                (x - 50.0, y + 90.0),
                (x, y + 90.0),
                (x, y + 110.0),
                (x + 50.0, y + 110.0),
            ],
        );
        let m = measure(&g, &sc);
        assert_eq!((m.edge_edge_x, m.x_drawn), (1, 1), "{m:?}");
    }

    #[test]
    fn ink_is_the_painted_share_of_the_canvas() {
        let (g, sc) = laid("flowchart TD\nA[a lone box]");
        let m = measure(&g, &sc);
        let n = &sc.nodes[0];
        // The box less its four 3 px corner roundings.
        let area = n.w * n.h - (4.0 - std::f64::consts::PI) * 9.0;
        let want = area / (m.width * m.height);
        assert!((m.ink - want).abs() < 1e-3 * want, "{} vs {want}", m.ink);
        // An edge adds its stroke.
        let (g, sc) = laid("flowchart TD\nA[a lone box]-->B[another]");
        assert!(measure(&g, &sc).ink > 0.0);
    }
}
