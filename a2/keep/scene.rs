//! Ready-to-draw geometry for interactive apps (drag & drop
//! editors, hit-testing, export). Every coordinate in this module
//! is FINAL (screen pixels, already following the diagram
//! direction) — unlike the abstract coordinates inside `layout`.
//!
//! Typical drag & drop flow:
//! 1. `scene(&graph)` — automatic layout + edge geometry.
//! 2. Keep node positions in app state; draw with your painter.
//! 3. When a node is dragged, call `route(&graph, &positions)` to
//!    recompute edge curves for the new positions (edge direction
//!    classification follows actual relative positions, not layers).
//! 4. `to_svg(&scene)` exports any arrangement to SVG.

use crate::layout::{
    intrinsic_size, layout_clustered_ranked, layout_sized, line_count, text_width, title_extra,
    LayoutResult, Placed, RankGroup, FRAME_GAP, LABEL_BOX_H, LINE_H, TITLE_LINE_H, TITLE_SCALE,
};
use crate::model::{
    Direction, Edge, EdgeAnimation, EdgeCurve, EdgeEnds, EdgeKind, EdgeMeta, End, Graph, Marker,
    NodeLink, NodeStyle, Shape, SubEdge,
};
use std::collections::HashMap;

// Edge-label placement lives in `labels`; these keep the call sites
// (and the tests) here compiling unchanged.
pub(crate) use crate::labels::{
    avoid_label_overlaps, channel_label_point, fit_moved_labels, FRAME_BORDER_BAND, LABEL_GAP,
    LABEL_ROUTE_WORK, LABEL_STATIC_WORK,
};
#[cfg(test)]
pub(crate) use crate::labels::{covered, ObstacleGrid, Rect};

pub(crate) const MARGIN: f64 = 28.0;
/// Gap between parallel edges (connecting the same node pair).
const PARALLEL_GAP: f64 = 16.0;
/// Wanted spacing between edge ends that share a node side when one
/// of them carries a cross/circle or start glyph (the glyphs are about
/// 11-12px across).
pub(crate) const GLYPH_PORT_GAP: f64 = 14.0;

/// An edge in abstract layout coordinates: bezier points + routed
/// waypoints (empty = none) + optional (label centre, label box width).
type AbsEdge = ([(f64, f64); 4], Vec<(f64, f64)>, Option<((f64, f64), f64)>);

/// Cluster box as (centre, size) — used for subgraph edge anchors.
type BoxCS = ((f64, f64), (f64, f64));
/// Cluster box as (x, y, w, h); `None` = empty subgraph.
type RawBox = Option<(f64, f64, f64, f64)>;

pub(crate) const EDGE_COLOR: &str = "#44507a";
pub(crate) const TEXT_COLOR: &str = "#232840";
pub(crate) const LABEL_BORDER: &str = "#d5d9ec";

/// Node with final position & size. Centre at (x, y).
///
/// Order guarantee: `Scene::nodes[i]` always describes `Graph::nodes[i]`
/// (index-parallel) — and each node also carries its source `id` so
/// consumers can correlate geometry robustly without relying on it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SceneNode {
    /// Source node id from the parsed model (`Graph::nodes[i].id`;
    /// entity/class name for ER and class diagrams).
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub shape: Shape,
    pub label: String,
    /// Custom colors from `style`/`classDef`; empty = shape theme.
    pub style: NodeStyle,
    /// Mermaid `click` link / tooltip (`Graph::links`), so a host can
    /// open the url or show the tooltip itself; `None` = none.
    pub link: Option<NodeLink>,
}

/// Edge with a final cubic bezier curve.
///
/// Order guarantee: the first `Graph::edges.len()` entries of
/// `Scene::edges` are index-parallel with `Graph::edges`; any
/// subgraph-touching edges (`Graph::sub_edges`) follow after them.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SceneEdge {
    /// Source-endpoint id: the `from` node's id, or the subgraph id
    /// for a sub-edge anchored on a cluster box.
    pub from: String,
    /// Target-endpoint id (node id, or subgraph id for sub-edges).
    pub to: String,
    /// Bezier points: start, control1, control2, end. This is the
    /// single-curve form used when `waypoints` is empty (adjacent
    /// layers, free-routed edges) and as the fallback everywhere.
    pub bezier: [(f64, f64); 4],
    /// Routed polyline (final coords) a long edge threads through — its
    /// node-boundary start, the per-layer channel points, and its end.
    /// Empty = draw `bezier`. A painter splines a smooth curve through
    /// these so the edge stays in its channel instead of cutting across.
    pub waypoints: Vec<(f64, f64)>,
    pub kind: EdgeKind,
    /// Cross / circle / two-way end glyphs layered over `kind` (see
    /// [`EdgeEnds`]); resolve with `ends.end_marker(kind)` /
    /// `ends.start_marker(kind)`.
    pub ends: EdgeEnds,
    /// Mermaid edge id + metadata (`A e1@--> B`, `e1@{ animate: true }`):
    /// the id is written as the path's `id` / `data-id`, the animation
    /// as a CSS class, the curve as the path shape.
    pub meta: EdgeMeta,
    /// (text, box centre, box width) when the edge has a label.
    pub label: Option<(String, (f64, f64), f64)>,
}

/// A rendered `subgraph` box. `(x, y)` is the TOP-LEFT corner
/// (unlike nodes, which are centre-based) since clusters are drawn
/// as container rectangles. Ordered outermost-first for painting.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SceneCluster {
    /// Source subgraph id from the parsed model.
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub title: String,
    /// Nesting depth, 0 = outermost.
    pub depth: usize,
}

/// Positioned geometry for one diagram, ready to paint or serialise.
///
/// The output structs (`Scene`, `SceneNode`, `SceneEdge`,
/// `SceneCluster`) are `#[non_exhaustive]`: future flowmaid versions
/// may add fields without a breaking release. Construct an empty scene
/// with [`Scene::empty`]; field ACCESS and mutation stay ordinary.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Scene {
    /// Index-parallel with `Graph::nodes` (see [`SceneNode`]).
    pub nodes: Vec<SceneNode>,
    /// `Graph::edges` first (index-parallel), then any sub-edges.
    pub edges: Vec<SceneEdge>,
    pub clusters: Vec<SceneCluster>,
    pub width: f64,
    pub height: f64,
}

/// An element picked out of a [`Scene`] by [`Scene::hit_test`] and
/// friends. Carries the INDEX into the matching `scene.{nodes,edges,
/// clusters}` vec — O(1), and `scene.nodes[i].id` recovers the stable
/// id. Coordinates handed to the pick methods are SCENE-space: a host
/// converts screen→scene once (`(screen - pan) / zoom`) and passes a
/// tolerance in scene units (`screen_tol / zoom`), so zoom/pan never
/// enter the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Node(usize),
    Edge(usize),
    Cluster(usize),
}

impl Scene {
    /// An empty scene with the given canvas size — the only way to
    /// construct a `Scene` outside this crate (the structs are
    /// non-exhaustive), for hosts that need a blank placeholder.
    pub fn empty(width: f64, height: f64) -> Scene {
        Scene {
            nodes: Vec::new(),
            edges: Vec::new(),
            clusters: Vec::new(),
            width,
            height,
        }
    }

    /// The topmost element at scene point `(x, y)`, or `None`. Z-order
    /// mirrors paint order: a node beats an overlapping edge, which
    /// beats a cluster box behind them. `tol` (scene units) widens edge
    /// picking so thin curves are still selectable.
    pub fn hit_test(&self, x: f64, y: f64, tol: f64) -> Option<Hit> {
        if let Some(i) = self.node_at(x, y) {
            return Some(Hit::Node(i));
        }
        if let Some(i) = self.edge_at(x, y, tol) {
            return Some(Hit::Edge(i));
        }
        self.cluster_at(x, y).map(Hit::Cluster)
    }

    /// Index of the topmost node whose shape contains `(x, y)`. Nodes
    /// are tested back-to-front (later-drawn wins, as painted), and the
    /// test is shape-precise for diamonds and (double) circles — their
    /// bounding box would over-select the empty corners — and the
    /// bounding rectangle for every other shape.
    pub fn node_at(&self, x: f64, y: f64) -> Option<usize> {
        self.nodes.iter().rposition(|n| node_contains(n, x, y))
    }

    /// Index of the edge closest to `(x, y)` within `tol` scene units,
    /// or `None`. Distance is measured to the drawn geometry — the
    /// routed waypoint polyline, or a sampling of the cubic bezier —
    /// and the NEAREST edge wins so overlapping edges resolve cleanly.
    pub fn edge_at(&self, x: f64, y: f64, tol: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (i, e) in self.edges.iter().enumerate() {
            if matches!(e.kind, EdgeKind::Invisible) {
                continue; // never drawn — never picked
            }
            let d = edge_distance(e, x, y);
            // `<=` so a later-drawn edge wins a distance tie, matching
            // node_at's topmost-wins z-order.
            if d <= tol && best.map_or(true, |(_, bd)| d <= bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Index of the deepest cluster whose box contains `(x, y)`. Nested
    /// clusters resolve to the innermost (largest `depth`). Clusters
    /// paint behind nodes and edges, so prefer [`hit_test`] when you
    /// want proper z-order.
    pub fn cluster_at(&self, x: f64, y: f64) -> Option<usize> {
        let mut best: Option<(usize, usize)> = None; // (index, depth)
        for (i, c) in self.clusters.iter().enumerate() {
            let inside = x >= c.x && x <= c.x + c.w && y >= c.y && y <= c.y + c.h;
            // Deepest wins; at equal depth prefer the SMALLER (more
            // specific) box so the pick is deterministic by geometry.
            let better = best.map_or(true, |(bi, d)| {
                c.depth > d
                    || (c.depth == d && c.w * c.h < self.clusters[bi].w * self.clusters[bi].h)
            });
            if inside && better {
                best = Some((i, c.depth));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Every element intersecting the (unordered) rectangle — for
    /// rubber-band / marquee multi-select. Returns nodes, then edges,
    /// then clusters, each in index order.
    pub fn hits_in_rect(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Hit> {
        let (rx0, rx1) = (x0.min(x1), x0.max(x1));
        let (ry0, ry1) = (y0.min(y1), y0.max(y1));
        let overlaps = |ax0: f64, ay0: f64, ax1: f64, ay1: f64| {
            ax0 <= rx1 && ax1 >= rx0 && ay0 <= ry1 && ay1 >= ry0
        };
        let mut out = Vec::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if overlaps(
                n.x - n.w / 2.0,
                n.y - n.h / 2.0,
                n.x + n.w / 2.0,
                n.y + n.h / 2.0,
            ) {
                out.push(Hit::Node(i));
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            if matches!(e.kind, EdgeKind::Invisible) {
                continue;
            }
            // Segment-vs-rect, not vertex-in-rect: an edge that crosses
            // the marquee between two far-apart samples must still count.
            if edge_polyline(e)
                .windows(2)
                .any(|w| seg_aabb(w[0], w[1], rx0, ry0, rx1, ry1))
            {
                out.push(Hit::Edge(i));
            }
        }
        for (i, c) in self.clusters.iter().enumerate() {
            if overlaps(c.x, c.y, c.x + c.w, c.y + c.h) {
                out.push(Hit::Cluster(i));
            }
        }
        out
    }

    /// The node nearest to `(x, y)` and its distance in scene units
    /// (0 when the point is inside the node's box). For edge-drawing
    /// snap — "drop near B → connect to B".
    pub fn nearest_node(&self, x: f64, y: f64) -> Option<(usize, f64)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let dx = (x - n.x).abs() - n.w / 2.0;
                let dy = (y - n.y).abs() - n.h / 2.0;
                (i, dx.max(0.0).hypot(dy.max(0.0)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// Whether a node's SHAPE (not just its bounding box) contains a point.
fn node_contains(n: &SceneNode, x: f64, y: f64) -> bool {
    let (hw, hh) = (n.w / 2.0, n.h / 2.0);
    if hw <= 0.0 || hh <= 0.0 {
        return false;
    }
    let (dx, dy) = ((x - n.x).abs(), (y - n.y).abs());
    match n.shape {
        // Rhombus: |dx|/hw + |dy|/hh <= 1.
        Shape::Diamond => dx / hw + dy / hh <= 1.0,
        // Circle: the renderer draws `<circle r=w/2>` (state
        // pseudostates too) — a DISC of radius w/2 on both axes, not an
        // ellipse — so match that exactly for any w/h.
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            dx * dx + dy * dy <= hw * hw
        }
        // Everything else: bounding rectangle.
        _ => dx <= hw && dy <= hh,
    }
}

/// Distance from `(x, y)` to an edge's drawn geometry.
fn edge_distance(e: &SceneEdge, x: f64, y: f64) -> f64 {
    let pts = edge_polyline(e);
    pts.windows(2)
        .map(|w| point_seg_dist(x, y, w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
}

/// A dense sampling of the curve an edge is actually DRAWN as, so
/// picking matches the pixels: a routed edge follows the same
/// curveBasis spline `spline_d` emits (which pulls away from the raw
/// waypoints), a plain edge follows its single cubic.
pub(crate) fn edge_polyline(e: &SceneEdge) -> Vec<(f64, f64)> {
    if let Some((start, segs)) = curve_geometry(e) {
        return curve_sample(start, &segs);
    }
    if e.waypoints.len() >= 2 {
        return basis_sample(&e.waypoints);
    }
    sample_cubic(&e.bezier, 24)
}

/// One drawing command of a non-default edge curve (the start point is
/// held apart).
#[derive(Debug, Clone, Copy)]
enum Seg {
    L((f64, f64)),
    Q((f64, f64), (f64, f64)),
    C((f64, f64), (f64, f64), (f64, f64)),
}

/// Corner radius of mermaid's `rounded` edge curve (`generateRoundedPath`).
const ROUNDED_CURVE_RADIUS: f64 = 5.0;

/// The geometry of an edge drawn with a non-default mermaid `curve`, as
/// (start point, commands) in scene coordinates; `None` for the default
/// (`basis`) curve, which keeps the historical path exactly. The curve
/// runs through the routed waypoints, or — for a single-cubic edge —
/// its two endpoints (a self-loop uses its four bezier points so it
/// still loops). Interpolating curves mermaid gets from d3
/// (`natural`, `monotoneX/Y`, `cardinal`, `catmullRom`) are drawn as a
/// Catmull-Rom spline through the same points; with two points d3
/// draws all of them as a straight line, and so does this.
fn curve_geometry(e: &SceneEdge) -> Option<((f64, f64), Vec<Seg>)> {
    let curve = e.meta.curve.filter(|&c| c != EdgeCurve::Basis)?;
    let p: Vec<(f64, f64)> = if e.waypoints.len() >= 2 {
        e.waypoints.clone()
    } else if e.from == e.to {
        e.bezier.to_vec()
    } else {
        vec![e.bezier[0], e.bezier[3]]
    };
    let n = p.len();
    let mid = |a: f64, b: f64| (a + b) / 2.0;
    let mut segs = Vec::new();
    match curve {
        EdgeCurve::Linear | EdgeCurve::Basis => segs.extend(p[1..].iter().map(|&q| Seg::L(q))),
        EdgeCurve::Rounded => {
            for i in 1..n - 1 {
                let (a, b, c) = (p[i - 1], p[i], p[i + 1]);
                let d1 = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                let d2 = ((c.0 - b.0).powi(2) + (c.1 - b.1).powi(2)).sqrt();
                if d1 == 0.0 || d2 == 0.0 {
                    segs.push(Seg::L(b));
                    continue;
                }
                let r = ROUNDED_CURVE_RADIUS.min(d1 / 2.0).min(d2 / 2.0);
                let p1 = (b.0 - (b.0 - a.0) / d1 * r, b.1 - (b.1 - a.1) / d1 * r);
                let p2 = (b.0 + (c.0 - b.0) / d2 * r, b.1 + (c.1 - b.1) / d2 * r);
                segs.push(Seg::L(p1));
                segs.push(Seg::Q(b, p2));
            }
            segs.push(Seg::L(p[n - 1]));
        }
        // d3 curveStep / curveStepBefore / curveStepAfter.
        EdgeCurve::Step => {
            for w in p.windows(2) {
                let x = mid(w[0].0, w[1].0);
                segs.push(Seg::L((x, w[0].1)));
                segs.push(Seg::L((x, w[1].1)));
            }
            segs.push(Seg::L(p[n - 1]));
        }
        EdgeCurve::StepBefore => {
            for w in p.windows(2) {
                segs.push(Seg::L((w[0].0, w[1].1)));
                segs.push(Seg::L(w[1]));
            }
        }
        EdgeCurve::StepAfter => {
            for w in p.windows(2) {
                segs.push(Seg::L((w[1].0, w[0].1)));
                segs.push(Seg::L(w[1]));
            }
        }
        // d3 curveBumpX / curveBumpY.
        EdgeCurve::BumpX => segs.extend(p.windows(2).map(|w| {
            let x = mid(w[0].0, w[1].0);
            Seg::C((x, w[0].1), (x, w[1].1), w[1])
        })),
        EdgeCurve::BumpY => segs.extend(p.windows(2).map(|w| {
            let y = mid(w[0].1, w[1].1);
            Seg::C((w[0].0, y), (w[1].0, y), w[1])
        })),
        _ if n == 2 => segs.push(Seg::L(p[1])),
        _ => {
            for i in 0..n - 1 {
                let p0 = p[i.saturating_sub(1)];
                let (p1, p2) = (p[i], p[i + 1]);
                let p3 = p[(i + 2).min(n - 1)];
                segs.push(Seg::C(
                    (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0),
                    (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0),
                    p2,
                ));
            }
        }
    }
    Some((p[0], segs))
}

/// SVG path data for [`curve_geometry`], mapped through `t`.
fn curve_d(start: (f64, f64), segs: &[Seg], t: impl Fn((f64, f64)) -> (f64, f64)) -> String {
    let s0 = t(start);
    let mut d = format!("M {:.1} {:.1}", s0.0, s0.1);
    for seg in segs {
        match *seg {
            Seg::L(p) => {
                let p = t(p);
                d.push_str(&format!(" L {:.1} {:.1}", p.0, p.1));
            }
            Seg::Q(c, p) => {
                let (c, p) = (t(c), t(p));
                d.push_str(&format!(" Q {:.1} {:.1}, {:.1} {:.1}", c.0, c.1, p.0, p.1));
            }
            Seg::C(c1, c2, p) => {
                let (c1, c2, p) = (t(c1), t(c2), t(p));
                d.push_str(&format!(
                    " C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
                    c1.0, c1.1, c2.0, c2.1, p.0, p.1
                ));
            }
        }
    }
    d
}

/// The SVG path `d` an edge is DRAWN with, mapped through `t` (scene →
/// output coordinates): a non-default mermaid `curve` through its
/// points ([`curve_geometry`]), else the `curveBasis` spline through
/// routed waypoints ([`spline_d`]), else the single cubic `bezier`.
/// Every painter of a [`SceneEdge`] goes through this one builder: the
/// router delivers a rerouted edge in `waypoints` and leaves `bezier`
/// a degenerate `[p0, p0, p3, p3]` chord, so a painter that strokes
/// `bezier` alone draws it straight through the boxes it was routed
/// around. [`edge_polyline`] samples exactly this curve (picking) and
/// [`edge_end_chord`] reads its ends (glyph placement).
pub(crate) fn edge_path_d(e: &SceneEdge, t: impl Fn((f64, f64)) -> (f64, f64)) -> String {
    if let Some((start, segs)) = curve_geometry(e) {
        curve_d(start, &segs, t)
    } else if e.waypoints.len() >= 2 {
        let q: Vec<(f64, f64)> = e.waypoints.iter().map(|&p| t(p)).collect();
        spline_d(&q)
    } else {
        let q: Vec<(f64, f64)> = e.bezier.iter().map(|&p| t(p)).collect();
        format!(
            "M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
            q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1, q[3].0, q[3].1
        )
    }
}

/// One end of an edge's DRAWN curve (the one [`edge_path_d`] emits) as
/// `(tip, toward)`: the end point itself and the point `reach` scene
/// units back along the curve by arc length — or the far end, when the
/// whole curve is shorter. The chord `tip → toward` is the direction
/// of the final drawn stretch, so an end glyph oriented along it (UML
/// head, crow's foot, cardinality) sits ON the stroke, even for a
/// rerouted edge whose degenerate `bezier` controls coincide with its
/// ends (where "toward the adjacent control point" has no direction at
/// all and the glyph collapses to a zero-size polygon). `toward ==
/// tip` only for a curve of zero length.
// The class / ER painters adopt this (with `edge_path_d`) in their own
// work packages; until then only the tests call it.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn edge_end_chord(
    e: &SceneEdge,
    at_start: bool,
    reach: f64,
) -> ((f64, f64), (f64, f64)) {
    let mut pts = edge_polyline(e);
    if !at_start {
        pts.reverse();
    }
    let tip = pts[0];
    let mut left = reach.max(f64::EPSILON);
    for w in pts.windows(2) {
        let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        if d >= left {
            let k = left / d;
            let toward = (
                w[0].0 + (w[1].0 - w[0].0) * k,
                w[0].1 + (w[1].1 - w[0].1) * k,
            );
            return (tip, toward);
        }
        left -= d;
    }
    (tip, pts[pts.len() - 1])
}

/// Dense sample of [`curve_geometry`] for picking and the bounding box.
fn curve_sample(start: (f64, f64), segs: &[Seg]) -> Vec<(f64, f64)> {
    const STEPS: usize = 12;
    let mut out = vec![start];
    let mut cur = start;
    for seg in segs {
        match *seg {
            Seg::L(p) => out.push(p),
            Seg::Q(c, p) => {
                for k in 1..=STEPS {
                    let t = k as f64 / STEPS as f64;
                    let u = 1.0 - t;
                    out.push((
                        u * u * cur.0 + 2.0 * u * t * c.0 + t * t * p.0,
                        u * u * cur.1 + 2.0 * u * t * c.1 + t * t * p.1,
                    ));
                }
            }
            Seg::C(c1, c2, p) => {
                let b = [cur, c1, c2, p];
                out.extend((1..=STEPS).map(|k| cubic_at(&b, k as f64 / STEPS as f64)));
            }
        }
        cur = match *seg {
            Seg::L(p) | Seg::Q(_, p) | Seg::C(_, _, p) => p,
        };
    }
    out
}

/// Sample a cubic bezier `[p0, c1, c2, p3]` into `steps + 1` points.
pub(crate) fn sample_cubic(b: &[(f64, f64); 4], steps: usize) -> Vec<(f64, f64)> {
    (0..=steps)
        .map(|i| cubic_at(b, i as f64 / steps as f64))
        .collect()
}

fn cubic_at(b: &[(f64, f64); 4], t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    (
        u * u * u * b[0].0
            + 3.0 * u * u * t * b[1].0
            + 3.0 * u * t * t * b[2].0
            + t * t * t * b[3].0,
        u * u * u * b[0].1
            + 3.0 * u * u * t * b[1].1
            + 3.0 * u * t * t * b[2].1
            + t * t * t * b[3].1,
    )
}

/// Densely sample the same open-uniform B-spline (`curveBasis`) that
/// [`spline_d`] draws through `pts`, so hit geometry == drawn geometry.
pub(crate) fn basis_sample(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = pts.len();
    if n < 3 {
        return vec![pts[0], pts[n - 1]];
    }
    let mut out = vec![pts[0]];
    // Straight run into the first control point (mirrors spline_d's `L`).
    let mut cur = (
        (5.0 * pts[0].0 + pts[1].0) / 6.0,
        (5.0 * pts[0].1 + pts[1].1) / 6.0,
    );
    out.push(cur);
    // One cubic per interior triple, then the closing cubic — identical
    // control points to spline_d's `bez`.
    let emit = |out: &mut Vec<(f64, f64)>,
                cur: &mut (f64, f64),
                a: (f64, f64),
                b: (f64, f64),
                p: (f64, f64)| {
        let seg = [
            *cur,
            ((2.0 * a.0 + b.0) / 3.0, (2.0 * a.1 + b.1) / 3.0),
            ((a.0 + 2.0 * b.0) / 3.0, (a.1 + 2.0 * b.1) / 3.0),
            ((a.0 + 4.0 * b.0 + p.0) / 6.0, (a.1 + 4.0 * b.1 + p.1) / 6.0),
        ];
        for k in 1..=6 {
            out.push(cubic_at(&seg, k as f64 / 6.0));
        }
        *cur = seg[3];
    };
    for i in 2..n {
        emit(&mut out, &mut cur, pts[i - 2], pts[i - 1], pts[i]);
    }
    emit(&mut out, &mut cur, pts[n - 2], pts[n - 1], pts[n - 1]);
    out.push(pts[n - 1]);
    out
}

/// A cluster's title as drawn (lines joined by `\n`, one per `<br/>`):
/// whole, unless it runs past its own frame INTO another frame (a
/// sibling, not one it sits inside) or past `canvas_right` (the drawn
/// canvas's right edge, same coordinates) — then each line still too
/// wide for its frame is cut with an ellipsis (after any dangling
/// separator), so neighbouring titles never overprint each other and
/// none is clipped mid-word by the canvas. Measured at the title font.
/// The header of frame `ci` its title occupies, as (x0, y0, x1, y1):
/// the title chip `to_svg` draws (fitted as [`fit_cluster_title`] fits
/// it) grown up to the frame's top border, so no edge squeezes between
/// the chip and the border. An edge
/// that comes within [`crate::clearance::CHIP_SEP`] of it runs through
/// or along the frame's name (G6, G11).
pub(crate) fn title_head(
    clusters: &[SceneCluster],
    ci: usize,
    canvas_right: f64,
) -> (f64, f64, f64, f64) {
    use crate::clearance::{CHIP_H, CHIP_INSET_X, CHIP_INSET_Y, CHIP_PAD_W};
    let c = &clusters[ci];
    let title = fit_cluster_title(clusters, ci, canvas_right);
    let (x, y) = (c.x + CHIP_INSET_X, c.y + CHIP_INSET_Y);
    (
        x,
        c.y,
        x + text_width(&title) * TITLE_SCALE + CHIP_PAD_W,
        y + CHIP_H + title_extra(&title),
    )
}

pub(crate) fn fit_cluster_title(clusters: &[SceneCluster], ci: usize, canvas_right: f64) -> String {
    let c = &clusters[ci];
    let (x, y, w, h) = (
        c.x + TITLE_INSET,
        c.y,
        text_width(&c.title) * TITLE_SCALE + 8.0,
        TITLE_STRIP + title_extra(&c.title),
    );
    if x + w <= c.x + c.w - TITLE_INSET {
        return c.title.clone();
    }
    let within = |a: &SceneCluster, b: &SceneCluster| {
        a.x >= b.x && a.y >= b.y && a.x + a.w <= b.x + b.w && a.y + a.h <= b.y + b.h
    };
    let collides = x + w > canvas_right
        || clusters.iter().enumerate().any(|(oi, o)| {
            oi != ci
                && !within(c, o)
                && !within(o, c)
                && x < o.x + o.w
                && o.x < x + w
                && y < o.y + o.h
                && o.y < y + h
        });
    if !collides {
        return c.title.clone();
    }
    let room = c.w - 2.0 * TITLE_INSET - 8.0;
    let fit_line = |line: &str| -> String {
        if text_width(line) * TITLE_SCALE <= room {
            return line.to_string();
        }
        let mut cut = line.to_string();
        while !cut.is_empty() && text_width(&format!("{cut}…")) * TITLE_SCALE > room {
            cut.pop();
        }
        // A cut that ends on a separator reads as a dangling "· …".
        let kept = cut
            .trim_end_matches(|ch: char| ch.is_whitespace() || TITLE_CUT_SEPARATORS.contains(&ch));
        format!("{kept}…")
    };
    c.title
        .split('\n')
        .map(fit_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Characters trimmed off the end of a cut title line before its
/// ellipsis: list separators that would otherwise dangle.
const TITLE_CUT_SEPARATORS: &[char] = &['·', '-', '–', '—', ',', ':', ';', '|', '/'];

/// Does segment `a`–`b` intersect the axis-aligned rectangle
/// `[rx0,rx1] × [ry0,ry1]`? Liang-Barsky clip (true when either
/// endpoint is inside, too).
pub(crate) fn seg_aabb(
    a: (f64, f64),
    b: (f64, f64),
    rx0: f64,
    ry0: f64,
    rx1: f64,
    ry1: f64,
) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let p = [-dx, dx, -dy, dy];
    let q = [a.0 - rx0, rx1 - a.0, a.1 - ry0, ry1 - a.1];
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for i in 0..4 {
        if p[i] == 0.0 {
            if q[i] < 0.0 {
                return false; // parallel to this edge and outside it
            }
        } else {
            let r = q[i] / p[i];
            if p[i] < 0.0 {
                if r > t1 {
                    return false;
                }
                t0 = t0.max(r);
            } else {
                if r < t0 {
                    return false;
                }
                t1 = t1.min(r);
            }
        }
    }
    true
}

/// Distance from a point to a line segment `a`–`b`.
fn point_seg_dist(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    let t = if len2 <= f64::EPSILON {
        0.0
    } else {
        (((px - a.0) * abx + (py - a.1) * aby) / len2).clamp(0.0, 1.0)
    };
    ((a.0 + t * abx) - px).hypot((a.1 + t * aby) - py)
}

/// Automatic layout: run the layout engine, then map all geometry
/// to final coordinates. This is the source of truth that
/// `render::render` uses as well.
pub fn scene(g: &Graph) -> Scene {
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    scene_sized(g, &sizes)
}

/// Same as [`scene`] but with caller-provided node sizes — for
/// nodes whose size doesn't come from the label (ER entity tables,
/// icon nodes, ...).
pub fn scene_sized(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );
    let mut sc = if g.subgraphs.is_empty() {
        scene_flat(g, sizes)
    } else {
        scene_clustered(g, sizes)
    };
    crate::edgepass::finish(g, &mut sc, crate::edgepass::Mode::Static);
    sc
}

/// The classic single-level pipeline (no subgraphs).
fn scene_flat(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    let mut sc = scene_from_layout(g, sizes, layout_sized(g, sizes), None);
    let moved = avoid_label_overlaps(&sc.nodes, &[], &mut sc.edges, LABEL_STATIC_WORK);
    fit_moved_labels(&mut sc, &moved);
    sc
}

/// Does a bow (abstract coordinates) or the label at its apex cover a
/// node other than its own ends — or reach into the column of a
/// `foreign` node (one outside the bow's box) on the layers it spans,
/// i.e. over a neighbouring box? `label` = (width, line count).
fn bow_blocked(
    nodes: &[Placed],
    pts: &[(f64, f64); 4],
    ends: (usize, usize),
    layers: (usize, usize),
    foreign: impl Fn(usize) -> bool,
    label: (f64, usize),
) -> bool {
    let curve = sample_cubic(pts, 24);
    let mid = cubic_mid(pts[0], pts[1], pts[2], pts[3]);
    let lh = LINE_H * label.1 as f64;
    let lw = if label.1 > 0 { label.0 / 2.0 } else { 0.0 };
    nodes.iter().enumerate().any(|(v, p)| {
        if v == ends.0 || v == ends.1 {
            return false;
        }
        let hb = p.bsize / 2.0;
        // A foreign node blocks its whole column on the bow's layers.
        let hl = if foreign(v) && p.layer >= layers.0 && p.layer <= layers.1 {
            f64::INFINITY
        } else {
            p.lsize / 2.0
        };
        let hit = |q: &(f64, f64)| (q.0 - p.b).abs() < hb && (q.1 - p.l).abs() < hl;
        curve.iter().any(hit)
            || (label.1 > 0 && (mid.0 - p.b).abs() < hb + lw && (mid.1 - p.l).abs() < hl + lh / 2.0)
    })
}

/// Breadth extents per layer of the nodes `keep` selects — what a
/// back-edge bows around on the layers it passes.
fn layer_extents(lo: &LayoutResult, keep: impl Fn(usize) -> bool) -> Vec<(f64, f64)> {
    let nlayers = lo.nodes.iter().map(|p| p.layer).max().map_or(0, |m| m + 1);
    let mut lay_ext = vec![(f64::INFINITY, f64::NEG_INFINITY); nlayers];
    for (v, p) in lo.nodes.iter().enumerate() {
        if !keep(v) {
            continue;
        }
        let e = &mut lay_ext[p.layer];
        e.0 = e.0.min(p.b - p.bsize / 2.0);
        e.1 = e.1.max(p.b + p.bsize / 2.0);
    }
    lay_ext
}

/// Turn an abstract [`LayoutResult`] into a positioned [`Scene`]:
/// edge geometry, canvas bbox, and the direction transform. Shared by
/// the flat and clustered pipelines so both route edges identically.
/// `node_cluster` (clustered pipeline only) scopes a back-edge's bow to
/// what it actually passes: members of the innermost box holding both
/// its ends (anything outside sits beyond that box's walls) that reach
/// into the band between its ends — so it stays beside them instead of
/// looping round the whole diagram.
fn scene_from_layout(
    g: &Graph,
    sizes: &[(f64, f64)],
    lo: LayoutResult,
    node_cluster: Option<&[Vec<usize>]>,
) -> Scene {
    // Breadth extents per layer, used by back-edges to route
    // themselves around every node on the layers they pass.
    let lay_ext = layer_extents(&lo, |_| true);

    // Edge geometry (abstract coordinates) with parallel-edge separation.
    let offs = parallel_offsets(g);
    let mut abs_edges: Vec<AbsEdge> = Vec::with_capacity(g.edges.len());
    for (ei, (e, off)) in g.edges.iter().zip(offs).enumerate() {
        let a = &lo.nodes[e.from];
        let b = &lo.nodes[e.to];
        let shared: Vec<usize> = node_cluster.map_or_else(Vec::new, |nc| {
            nc[e.from]
                .iter()
                .zip(&nc[e.to])
                .take_while(|(x, y)| x == y)
                .map(|(x, _)| *x)
                .collect()
        });
        // Lebar box label (0 bila tanpa label) — busur back-edge
        // melebar sebesar ini supaya label di apex tidak menabrak node.
        let lbl_w = e.label.as_ref().map_or(0.0, |l| text_width(l) + 14.0);
        let bow = |ext: &[(f64, f64)]| {
            edge_points(
                a,
                b,
                g.nodes[e.from].shape,
                g.nodes[e.to].shape,
                e.from == e.to,
                lo.total_b,
                off,
                ext,
                lbl_w,
            )
        };
        let pts = match node_cluster {
            Some(nc) if b.layer < a.layer => {
                // The bow runs between the target's bottom and the
                // source's top: only nodes reaching into that band are
                // in its way (all of an in-between layer, but on the end
                // layers just the ends and anything taller).
                let (top, bot) = (b.l + b.lsize / 2.0, a.l - a.lsize / 2.0);
                let in_band = |v: usize| {
                    let p = &lo.nodes[v];
                    v == e.from
                        || v == e.to
                        || (p.layer >= b.layer
                            && p.layer <= a.layer
                            && p.l + p.lsize / 2.0 > top
                            && p.l - p.lsize / 2.0 < bot)
                };
                let local = bow(&layer_extents(&lo, |v| {
                    in_band(v) && (v == e.from || v == e.to || nc[v].starts_with(&shared))
                }));
                // Clearing only the box's own members can swing the bow
                // (or its label) out over a neighbour box: then fall back
                // to clearing every node on its layers.
                let lines = e.label.as_ref().map_or(0, |l| line_count(l));
                let foreign = |v: usize| !nc[v].starts_with(&shared);
                if bow_blocked(
                    &lo.nodes,
                    &local,
                    (e.from, e.to),
                    (b.layer, a.layer),
                    foreign,
                    (lbl_w, lines),
                ) {
                    bow(&lay_ext)
                } else {
                    local
                }
            }
            _ => bow(&lay_ext),
        };
        // A long edge threads through its virtual-node channel: its
        // node-boundary start, the per-layer channel points, then its
        // end. Short/adjacent edges keep the single curve (no waypoints).
        // The endpoints anchor towards the ADJACENT CHANNEL POINT (not
        // the far node's centre): the lanes are already spread apart,
        // so converging arrows land at distinct spots along the border
        // instead of stabbing one point — mermaid's fan-in look.
        let wps: Vec<(f64, f64)> = if lo.edge_paths[ei].is_empty() {
            Vec::new()
        } else {
            let chain = &lo.edge_paths[ei];
            let a_bottom = b.layer > a.layer; // exit side of the source
            let p0 = anchor(a, g.nodes[e.from].shape, chain[0], off, a_bottom);
            let p3 = anchor(
                b,
                g.nodes[e.to].shape,
                *chain.last().unwrap(),
                off,
                !a_bottom,
            );
            let mut v = Vec::with_capacity(chain.len() + 2);
            v.push(p0);
            v.extend(chain.iter().copied());
            v.push(p3);
            v
        };
        let label = e.label.as_ref().map(|l| {
            // On a routed edge the label sits at a channel point, so
            // labels of converging edges spread out instead of piling.
            let mid = if wps.is_empty() {
                cubic_mid(pts[0], pts[1], pts[2], pts[3])
            } else {
                channel_label_point(&wps)
            };
            (mid, text_width(l) + 14.0)
        });
        abs_edges.push((pts, wps, label));
    }

    // Canvas from the bounding box of nodes + curves + labels. A
    // bezier curve always stays inside the convex hull of its
    // control points, so the control-point bbox guarantees nothing
    // gets clipped.
    let mut bb = Bbox::new();
    for p in &lo.nodes {
        bb.add(p.b - p.bsize / 2.0, p.l - p.lsize / 2.0);
        bb.add(p.b + p.bsize / 2.0, p.l + p.lsize / 2.0);
    }
    for (e, (pts, wps, label)) in g.edges.iter().zip(&abs_edges) {
        // Invisible links are layout-only — keep their curve out of
        // the canvas bbox so it isn't padded with empty space.
        if matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        // Routed edges are drawn through their waypoints, not the
        // fallback curve — bound the canvas by whichever is drawn.
        if wps.is_empty() {
            for &(bp, lp) in pts {
                bb.add(bp, lp);
            }
        } else {
            for &(bp, lp) in wps {
                bb.add(bp, lp);
            }
        }
        if let Some((m, w)) = label {
            bb.add(m.0 - w / 2.0, m.1 - 10.0);
            bb.add(m.0 + w / 2.0, m.1 + 10.0);
        }
    }
    // Use finish() (guarded against the empty bbox) — the raw
    // fields are ±infinity for an empty level, which used to poison
    // every coordinate downstream with NaN (found by a bughunter
    // via empty nested subgraphs).
    let (minb, maxb, minl, maxl) = bb.finish();
    let offb = MARGIN - minb;
    let offl = MARGIN - minl;
    let total_b = (maxb - minb) + 2.0 * MARGIN;
    let total_l = (maxl - minl) + 2.0 * MARGIN;

    // Map abstract coordinates (b, l) -> final (x, y).
    let dir = g.direction;
    let tf = move |p: (f64, f64)| -> (f64, f64) {
        let b = p.0 + offb;
        let l = p.1 + offl;
        match dir {
            Direction::TD => (b, l),
            Direction::BT => (b, total_l - l),
            Direction::LR => (l, b),
            Direction::RL => (total_l - l, b),
        }
    };
    let horizontal = matches!(dir, Direction::LR | Direction::RL);
    let (width, height) = if horizontal {
        (total_l, total_b)
    } else {
        (total_b, total_l)
    };

    let nodes = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let (x, y) = tf((lo.nodes[i].b, lo.nodes[i].l));
            let (w, h) = sizes[i]; // final size is direction-independent
            SceneNode {
                id: n.id.clone(),
                x,
                y,
                w,
                h,
                shape: n.shape,
                label: n.label.clone(),
                style: n.style.clone(),
                link: g.links.get(&i).and_then(NodeLink::sanitized),
            }
        })
        .collect();

    let edges = g
        .edges
        .iter()
        .zip(abs_edges)
        .map(|(e, (pts, wps, label))| SceneEdge {
            from: g.nodes[e.from].id.clone(),
            to: g.nodes[e.to].id.clone(),
            bezier: [tf(pts[0]), tf(pts[1]), tf(pts[2]), tf(pts[3])],
            waypoints: wps.iter().map(|&p| tf(p)).collect(),
            kind: e.kind,
            ends: e.ends,
            meta: e.meta.clone(),
            label: e.label.clone().zip(label).map(|(t, (m, w))| (t, tf(m), w)),
        })
        .collect();

    Scene {
        nodes,
        edges,
        clusters: Vec::new(),
        width,
        height,
    }
}

/// Extra headroom above a subgraph's content for its title strip.
const SUB_HEADER: f64 = 26.0;
/// Padding when re-wrapping cluster boxes around dragged members.
const SUB_PAD: f64 = 14.0;
/// Left inset of a cluster title chip inside its frame.
const TITLE_INSET: f64 = 6.0;
/// Height of the strip a cluster title chip occupies at the frame top.
const TITLE_STRIP: f64 = 21.0;
/// Half-width of the strip a channel lane needs inside its cluster box.
const LANE_PAD: f64 = 12.0;
/// A subgraph needs at least this many members fed only by one outside
/// node before they are packed into a grid instead of sharing one rank.
const GRID_MIN_MEMBERS: usize = 3;
/// The same for edge-less members: three notes side by side still read
/// as one row, and packing them only trades breadth for depth.
const GRID_MIN_EDGELESS: usize = 4;

/// Global cluster-aware pipeline: lay every real node out in ONE
/// Sugiyama pass (no supernode collapse) with subgraph members kept
/// contiguous, then wrap each subgraph around its final members. This
/// keeps cross-cluster edges ordered globally so they route through the
/// gaps between boxes instead of tangling straight through them.
fn scene_clustered(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    let node_cluster = node_cluster_paths(g);
    let grid = grid_pairs(g, &node_cluster);
    let proxies = sub_edge_proxies(g, &node_cluster, &grid);
    let mut rank_only = proxies.clone();
    rank_only.extend_from_slice(&grid);
    // Each sub-edge also gets one routed STAND-IN edge between a proxy
    // pair — mermaid lays `A --> box` out as an edge to one of the box's
    // members — so it owns a channel (and label slot) through the ranks
    // it spans; it is cut back to the frame when drawn. `ga` is `g` plus
    // those edges, appended after the real ones.
    let ne = g.edges.len();
    let mut ga = g.clone();
    let mut stand_in: Vec<Option<usize>> = vec![None; g.sub_edges.len()];
    for (i, (e, (from, to))) in g.sub_edges.iter().zip(&proxies).enumerate() {
        let Some(&a) = from.first() else { continue };
        let Some(&b) = to.iter().find(|&&b| b != a) else {
            continue;
        };
        stand_in[i] = Some(ga.edges.len());
        ga.edges.push(Edge {
            from: a,
            to: b,
            label: e.label.clone(),
            kind: e.kind,
            ends: e.ends,
            meta: e.meta.clone(),
            end_room: (0.0, 0.0),
        });
    }
    // The clusters a stand-in's ends cut back to (its box ends).
    let cut_at = |ei: usize| -> Vec<usize> {
        let Some(i) = stand_in.iter().position(|&x| x == Some(ei)) else {
            return Vec::new();
        };
        let e = &g.sub_edges[i];
        [e.from, e.to]
            .into_iter()
            .filter_map(|end| match end {
                End::Sub(s) => Some(s),
                End::Node(_) => None,
            })
            .collect()
    };
    let (lo, groups) = layout_clustered_ranked(&ga, sizes, &node_cluster, &rank_only);
    let mut sc = scene_from_layout(&ga, sizes, lo, Some(&node_cluster));
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);

    // Wrap boxes around the laid-out members, then widen each box to
    // also cover the channel lanes laid out INSIDE it: a lane that
    // enters the box from the top may sit beside the outermost member
    // node (between it and the invisible border wall), which the
    // node-only bbox would leave poking outside the drawn rectangle.
    // Only waypoints whose dummy the layout placed in this cluster
    // count (plus an end anchor on one of its own members) — a lane
    // running outside, or inside another box, must not drag the frame
    // over to it. Lanes spread along the breadth axis (x in TB/BT,
    // y in LR/RL).
    let frames: Vec<RawBox>;
    sc.clusters = {
        let (mut boxes, depth) = cluster_raw_boxes(g, &sc.nodes);
        // Member-only boxes: what a widened frame must stay clear of.
        let raw = boxes.clone();
        for si in 0..g.subgraphs.len() {
            let Some((x, y, w, h)) = boxes[si] else {
                continue;
            };
            // (breadth lo, breadth hi, layer lo, layer hi)
            let (mut b0, mut b1, l0, l1) = if horizontal {
                (y, y + h, x, x + w)
            } else {
                (x, x + w, y, y + h)
            };
            for (ei, (ge, se)) in ga.edges.iter().zip(sc.edges.iter()).enumerate() {
                if se.waypoints.is_empty() {
                    continue;
                }
                // A stand-in's run inside its own box ends is never drawn.
                let cut = if ei >= ne { cut_at(ei) } else { Vec::new() };
                let last = se.waypoints.len() - 1;
                for (k, &(wx, wy)) in se.waypoints.iter().enumerate() {
                    let path: &[usize] = if k == 0 {
                        &node_cluster[ge.from]
                    } else if k == last {
                        &node_cluster[ge.to]
                    } else {
                        groups[ei].get(k - 1).map_or(&[], |p| p.as_slice())
                    };
                    let inside = path.contains(&si) && !cut.iter().any(|c| path.contains(c));
                    let (wb, wl) = if horizontal { (wy, wx) } else { (wx, wy) };
                    if inside && wl >= l0 && wl <= l1 {
                        b0 = b0.min(wb - LANE_PAD);
                        b1 = b1.max(wb + LANE_PAD);
                    }
                }
            }
            // A lane may not widen a frame into a sibling's (G2): it
            // stops FRAME_GAP short, and the router takes over the lane
            // waypoint left outside. The side is judged on the member-only
            // boxes; the stop is the sibling's frame as it stands, which
            // is already widened when that sibling came first, so two
            // siblings widening towards each other cannot meet.
            let (ob0, ob1) = if horizontal { (y, y + h) } else { (x, x + w) };
            let span = |r: (f64, f64, f64, f64)| {
                let (ox, oy, ow, oh) = r;
                if horizontal {
                    (oy, oy + oh, ox, ox + ow)
                } else {
                    (ox, ox + ow, oy, oy + oh)
                }
            };
            for (sj, other) in raw.iter().enumerate() {
                let (Some(other), Some(now)) = (*other, boxes[sj]) else {
                    continue;
                };
                if sj == si || g.subgraphs[sj].parent != g.subgraphs[si].parent {
                    continue;
                }
                let (sb0, sb1, sl0, sl1) = span(other);
                let (nb0, nb1, _, _) = span(now);
                if sl1 <= l0 || sl0 >= l1 {
                    continue; // not beside it along the flow
                }
                if sb0 >= ob1 {
                    b1 = b1.min(nb0 - FRAME_GAP).max(ob1);
                } else if sb1 <= ob0 {
                    b0 = b0.max(nb1 + FRAME_GAP).min(ob0);
                }
            }
            boxes[si] = Some(if horizontal {
                (x, b0, w, b1 - b0)
            } else {
                (b0, y, b1 - b0, h)
            });
        }
        // Re-enforce nesting: a parent must still enclose its
        // (possibly widened) children. Deepest-first, one level up.
        let mut order: Vec<usize> = (0..g.subgraphs.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(depth[i]));
        for &c in &order {
            let (Some(p), Some((cx, cy, cw, ch))) = (g.subgraphs[c].parent, boxes[c]) else {
                continue;
            };
            if let Some((px, py, pw, ph)) = boxes[p] {
                let nx0 = px.min(cx - SUB_PAD);
                let ny0 = py.min(cy - SUB_PAD);
                let nx1 = (px + pw).max(cx + cw + SUB_PAD);
                let ny1 = (py + ph).max(cy + ch + SUB_PAD);
                boxes[p] = Some((nx0, ny0, nx1 - nx0, ny1 - ny0));
            }
        }
        let mut out: Vec<SceneCluster> = (0..g.subgraphs.len())
            .filter_map(|i| {
                boxes[i].map(|(x, y, w, h)| SceneCluster {
                    id: g.subgraphs[i].id.clone(),
                    x,
                    y,
                    w,
                    h,
                    title: g.subgraphs[i].title.clone(),
                    depth: depth[i],
                })
            })
            .collect();
        out.sort_by_key(|c| c.depth);
        frames = boxes;
        out
    };

    // Edges that touch a whole subgraph box (state-diagram composites,
    // `A --> subgraph`) route against the cluster rectangle rather than
    // a node — the node-to-node pass in `scene_from_layout` skips them.
    // When the box lies wholly before or after the other end along the
    // flow, the edge is drawn like mermaid's: towards the member the
    // layout ranked it onto, cut at the frame — so it runs through the
    // rank gap the layout left, not diagonally across other nodes.
    let stand_edges = sc.edges.split_off(ne);
    if !g.sub_edges.is_empty() {
        let (raw, _) = cluster_raw_boxes(g, &sc.nodes);
        let end_placed = |end: End| -> Option<(Placed, Shape)> {
            match end {
                End::Node(v) => Some((
                    Placed {
                        b: sc.nodes[v].x,
                        l: sc.nodes[v].y,
                        bsize: sc.nodes[v].w,
                        lsize: sc.nodes[v].h,
                        layer: 0,
                    },
                    g.nodes[v].shape,
                )),
                End::Sub(si) => raw.get(si).copied().flatten().map(|(x, y, w, h)| {
                    (
                        Placed {
                            b: x + w / 2.0,
                            l: y + h / 2.0,
                            bsize: w,
                            lsize: h,
                            layer: 0,
                        },
                        Shape::Rect,
                    )
                }),
            }
        };
        let end_id = |end: End| -> String {
            match end {
                End::Node(v) => g.nodes[v].id.clone(),
                End::Sub(si) => g.subgraphs[si].id.clone(),
            }
        };
        for (i, (e, (from_m, to_m))) in g.sub_edges.iter().zip(&proxies).enumerate() {
            // A routed stand-in, cut back to the frame(s), when it has a
            // channel; else a curve to the facing frame side; else (no
            // side faces the other end) the old box-to-box curve.
            // A member drawn to (or from) its own enclosing box: a
            // short stub to the nearer flow-axis side of the frame, its
            // label just outside — no curve can reach the box from
            // outside without crossing the member itself.
            let own = match (e.from, e.to) {
                (End::Node(v), End::Sub(s)) | (End::Sub(s), End::Node(v))
                    if node_cluster[v].contains(&s) =>
                {
                    frames.get(s).copied().flatten().map(|f| {
                        let exit = matches!(e.from, End::Node(_));
                        let lw = e.label.as_ref().map_or(0.0, |l| text_width(l) + 14.0);
                        own_box_stub(g.direction, &sc.nodes[v], f, exit, lw)
                    })
                }
                _ => None,
            };
            if let Some((pts, mid)) = own {
                sc.edges.push(SceneEdge {
                    from: end_id(e.from),
                    to: end_id(e.to),
                    bezier: pts,
                    waypoints: Vec::new(),
                    kind: e.kind,
                    ends: e.ends,
                    meta: e.meta.clone(),
                    label: e
                        .label
                        .as_ref()
                        .map(|l| (l.clone(), mid, text_width(l) + 14.0)),
                });
                continue;
            }
            let routed = stand_in[i].and_then(|ei| {
                cut_to_frames(
                    g.direction,
                    &stand_edges[ei - ne].waypoints,
                    &frames,
                    e.from,
                    e.to,
                )
            });
            let (pts, wps) = match routed {
                Some(wps) => (
                    [wps[0], wps[0], wps[wps.len() - 1], wps[wps.len() - 1]],
                    wps,
                ),
                None => {
                    let ported =
                        sub_edge_ported(g, &sc.nodes, &frames, (e.from, from_m), (e.to, to_m));
                    let pts = match ported {
                        Some(pts) => pts,
                        None => {
                            let (Some((pa, sa)), Some((pb, sb))) =
                                (end_placed(e.from), end_placed(e.to))
                            else {
                                continue;
                            };
                            free_edge(&pa, &pb, sa, sb, false, 0.0, Some(g.direction))
                        }
                    };
                    (pts, Vec::new())
                }
            };
            let label = e.label.as_ref().map(|l| {
                let mid = if wps.is_empty() {
                    cubic_mid(pts[0], pts[1], pts[2], pts[3])
                } else {
                    channel_label_point(&wps)
                };
                (l.clone(), mid, text_width(l) + 14.0)
            });
            sc.edges.push(SceneEdge {
                from: end_id(e.from),
                to: end_id(e.to),
                bezier: pts,
                waypoints: wps,
                kind: e.kind,
                ends: e.ends,
                meta: e.meta.clone(),
                label,
            });
        }
    }

    // Labels last, once every edge (sub-edges too) is drawn.
    let moved = avoid_label_overlaps(&sc.nodes, &sc.clusters, &mut sc.edges, LABEL_STATIC_WORK);
    fit_moved_labels(&mut sc, &moved);

    // Re-normalise so the boxes' padding/header and any box-routed
    // sub-edges (which reach past the nodes) can't clip the canvas.
    if !sc.clusters.is_empty() {
        let mut bb = Bbox::new();
        grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, maxx, miny, maxy) = bb.finish();
        let (dx, dy) = (MARGIN - minx, MARGIN - miny);
        if dx != 0.0 || dy != 0.0 {
            shift_scene(&mut sc, dx, dy);
        }
        sc.width = (maxx - minx) + 2.0 * MARGIN;
        sc.height = (maxy - miny) + 2.0 * MARGIN;
    }
    sc
}

/// A stand-in's routed waypoints (screen coordinates) cut back to the
/// frames of its box ends: the run before a source box's far side and
/// past a target box's near side is dropped, and the new end sits on
/// that side, in the lane of the waypoint next to it outside the box
/// (kept within the frame). `None` without a channel, or when the run
/// doesn't cross the side (the boxes aren't stacked along the flow) or
/// too little of it is left to curve.
fn cut_to_frames(
    dir: Direction,
    wps: &[(f64, f64)],
    frames: &[RawBox],
    from: End,
    to: End,
) -> Option<Vec<(f64, f64)>> {
    if wps.len() < 3 {
        return None;
    }
    // Flow-space extent of a box end: (breadth lo, hi, layer lo, hi).
    let span = |end: End| -> Option<Option<(f64, f64, f64, f64)>> {
        match end {
            End::Node(_) => Some(None),
            End::Sub(s) => {
                let (x, y, w, h) = frames.get(s).copied().flatten()?;
                let (b0, l0) = to_flow(dir, (x, y));
                let (b1, l1) = to_flow(dir, (x + w, y + h));
                Some(Some((b0.min(b1), b0.max(b1), l0.min(l1), l0.max(l1))))
            }
        }
    };
    // A lane's breadth, pulled inside the frame (clear of its corners).
    let clamp = |b: f64, (b0, b1, _, _): (f64, f64, f64, f64)| {
        b.max(b0 + SUB_PAD).min((b1 - SUB_PAD).max(b0 + SUB_PAD))
    };
    let mut pts: Vec<(f64, f64)> = wps.iter().map(|&p| to_flow(dir, p)).collect();
    // The channel runs forward along the flow, or this isn't one.
    if pts.windows(2).any(|w| w[1].1 < w[0].1) {
        return None;
    }
    if let Some(f) = span(from)? {
        let k = pts.iter().rposition(|p| p.1 < f.3)?;
        if k + 1 >= pts.len() {
            return None;
        }
        let b = clamp(pts[k + 1].0, f);
        pts.drain(..=k);
        pts.insert(0, (b, f.3));
    }
    if let Some(t) = span(to)? {
        let k = pts.iter().position(|p| p.1 > t.2)?;
        if k == 0 {
            return None;
        }
        let b = clamp(pts[k - 1].0, t);
        pts.truncate(k);
        pts.push((b, t.2));
    }
    if pts.len() < 3 {
        return None;
    }
    Some(pts.into_iter().map(|p| from_flow(dir, p)).collect())
}

/// A straight stub between member `n` and the frame `f` of a box
/// enclosing it, on the frame side nearer to `n` along the flow axis —
/// from the node out (`exit`) or from the frame in — plus its label
/// point: just outside that side, clear of the frame's border line, for
/// a label `label_w` wide. Screen coordinates.
fn own_box_stub(
    dir: Direction,
    n: &SceneNode,
    f: (f64, f64, f64, f64),
    exit: bool,
    label_w: f64,
) -> ([(f64, f64); 4], (f64, f64)) {
    let horizontal = matches!(dir, Direction::LR | Direction::RL);
    let (b, l) = to_flow(dir, (n.x, n.y));
    let (_, fa) = to_flow(dir, (f.0, f.1));
    let (_, fb) = to_flow(dir, (f.0 + f.2, f.1 + f.3));
    let (l0, l1) = (fa.min(fb), fa.max(fb));
    let half = if horizontal { n.w } else { n.h } / 2.0;
    let label_half = if horizontal { label_w } else { LABEL_BOX_H } / 2.0;
    // Outward along the flow axis: -1 at the low side, +1 at the high.
    let s = if l - l0 < l1 - l { -1.0 } else { 1.0 };
    let side = if s < 0.0 { l0 } else { l1 };
    let at_node = (b, l + s * half);
    let at_frame = (b, side);
    let (p0, p3) = if exit {
        (at_node, at_frame)
    } else {
        (at_frame, at_node)
    };
    let lerp = |t: f64| (p0.0 + (p3.0 - p0.0) * t, p0.1 + (p3.1 - p0.1) * t);
    let pts = [p0, lerp(1.0 / 3.0), lerp(2.0 / 3.0), p3];
    let mid = (b, side + s * (FRAME_BORDER_BAND + LABEL_GAP + label_half));
    (pts.map(|p| from_flow(dir, p)), from_flow(dir, mid))
}

/// Screen `(x, y)` -> flow `(breadth, layer)`, the layer axis growing
/// along the diagram's direction.
fn to_flow(dir: Direction, p: (f64, f64)) -> (f64, f64) {
    match dir {
        Direction::TD => p,
        Direction::BT => (p.0, -p.1),
        Direction::LR => (p.1, p.0),
        Direction::RL => (p.1, -p.0),
    }
}

/// Inverse of [`to_flow`].
fn from_flow(dir: Direction, p: (f64, f64)) -> (f64, f64) {
    match dir {
        Direction::TD => p,
        Direction::BT => (p.0, -p.1),
        Direction::LR => (p.1, p.0),
        Direction::RL => (-p.1, p.0),
    }
}

/// The curve of a sub-edge drawn to / from the frame of its box end(s),
/// in screen coordinates: a box end becomes a port on the frame side
/// facing the other end, at the breadth of the member nearest to it
/// (`members` = the rank proxies, see [`sub_edge_proxies`]). `None`
/// when a box and the other end overlap along the flow axis (a member
/// pointing at its own box, a node beside the box) — no side faces it.
fn sub_edge_ported(
    g: &Graph,
    nodes: &[SceneNode],
    frames: &[RawBox],
    from: (End, &[usize]),
    to: (End, &[usize]),
) -> Option<[(f64, f64); 4]> {
    let dir = g.direction;
    let horizontal = matches!(dir, Direction::LR | Direction::RL);
    // Flow-space extent of an end: (breadth centre, layer lo, layer hi).
    let extent = |end: End| -> Option<(f64, f64, f64)> {
        let (x, y, w, h) = match end {
            End::Node(v) => {
                let n = &nodes[v];
                (n.x - n.w / 2.0, n.y - n.h / 2.0, n.w, n.h)
            }
            End::Sub(s) => frames.get(s).copied().flatten()?,
        };
        let (b0, l0) = to_flow(dir, (x, y));
        let (b1, l1) = to_flow(dir, (x + w, y + h));
        Some(((b0 + b1) / 2.0, l0.min(l1), l0.max(l1)))
    };
    let (fa, ta) = (extent(from.0)?, extent(to.0)?);
    // Does `to` lie after `from` along the flow (else before)?
    let ahead = if fa.2 <= ta.1 {
        true
    } else if ta.2 <= fa.1 {
        false
    } else {
        return None;
    };
    let nearest = |members: &[usize], b: f64| -> Option<f64> {
        members
            .iter()
            .map(|&v| to_flow(dir, (nodes[v].x, nodes[v].y)).0)
            .min_by(|x, y| (x - b).abs().total_cmp(&(y - b).abs()))
    };
    // A box end's port: its frame side facing the other end, at the
    // breadth of the proxy member nearest to `toward` (the box centre
    // without one).
    let port = |members: &[usize], ext: (f64, f64, f64), toward: f64, exit: bool| {
        let b = nearest(members, toward).unwrap_or(ext.0);
        (b, if ahead == exit { ext.2 } else { ext.1 })
    };
    // A node end sits on the node's own border, like any edge.
    let node_point = |v: usize, other: (f64, f64), exit: bool| {
        let n = &nodes[v];
        let (b, l) = to_flow(dir, (n.x, n.y));
        let (bsize, lsize) = if horizontal { (n.h, n.w) } else { (n.w, n.h) };
        let placed = Placed {
            b,
            l,
            bsize,
            lsize,
            layer: 0,
        };
        anchor(&placed, g.nodes[v].shape, other, 0.0, ahead == exit)
    };
    let (p0, p3) = match (from.0, to.0) {
        (End::Sub(_), End::Sub(_)) => {
            let p0 = port(from.1, fa, ta.0, true);
            (p0, port(to.1, ta, p0.0, false))
        }
        (End::Node(v), End::Sub(_)) => {
            let p3 = port(to.1, ta, fa.0, false);
            (node_point(v, p3, true), p3)
        }
        (End::Sub(_), End::Node(v)) => {
            let p0 = port(from.1, fa, ta.0, true);
            (p0, node_point(v, p0, false))
        }
        (End::Node(_), End::Node(_)) => return None,
    };
    let dl = (p3.1 - p0.1) * 0.5;
    let pts = [p0, (p0.0, p0.1 + dl), (p3.0, p3.1 - dl), p3];
    Some(pts.map(|p| from_flow(dir, p)))
}

/// Rank-only groups standing in for the edges that end on a whole
/// subgraph box (`A --> sub`, state composites) — mermaid re-points
/// such an edge at the cluster's contents for layout. Without them the
/// layering never sees the edge: every member of an otherwise
/// unconnected box lands on the first rank, beside its source. A box
/// as TARGET stands for its entry members (no in-edge from inside the
/// box), as SOURCE for its exit members (no out-edge to inside it).
/// `grid` pairs count as internal edges, so a packed box is entered
/// through its first row only. One group per entry of `g.sub_edges`.
fn sub_edge_proxies(g: &Graph, node_cluster: &[Vec<usize>], grid: &[RankGroup]) -> Vec<RankGroup> {
    if g.sub_edges.is_empty() {
        return Vec::new();
    }
    // (cluster, node) pairs with an in-edge / out-edge inside that
    // cluster: an edge is internal to every cluster both ends share.
    let mut has_in: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut has_out: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let pairs = g.edges.iter().map(|e| (e.from, e.to));
    let grid_pairs = grid.iter().map(|(a, b)| (a[0], b[0]));
    for (from, to) in pairs.chain(grid_pairs) {
        if from == to {
            continue;
        }
        let (pa, pb) = (&node_cluster[from], &node_cluster[to]);
        for (ca, cb) in pa.iter().zip(pb) {
            if ca != cb {
                break;
            }
            has_in.insert((*ca, to));
            has_out.insert((*ca, from));
        }
    }
    let members = |s: usize, flags: &std::collections::HashSet<(usize, usize)>| -> Vec<usize> {
        (0..g.nodes.len())
            .filter(|&v| node_cluster[v].contains(&s) && !flags.contains(&(s, v)))
            .collect()
    };
    let ends = |end: End, flags| match end {
        End::Node(v) => vec![v],
        End::Sub(s) => members(s, flags),
    };
    g.sub_edges
        .iter()
        .map(|e| (ends(e.from, &has_out), ends(e.to, &has_in)))
        .collect()
}

/// Rank-only pairs that pack a subgraph's loose members into a roughly
/// square grid: with `k` such members, member `i` is ranked above member
/// `i + ceil(sqrt(k))`, so they fill rows of that width in declaration
/// order. Left alone they would all share one rank — a list of 20 notes
/// drawn as one 10k px row. Loose means edge-less, or fed only by one
/// node outside the subgraph (a hub fanning out into a box of leaves);
/// each such set of at least [`GRID_MIN_MEMBERS`] (edge-less:
/// [`GRID_MIN_EDGELESS`]) is packed on its own.
/// Skipped when the subgraph sets a `direction` across the diagram's
/// flow (`direction LR` in a TB chart): there one row is what the
/// author asked for. Each pair is a one-to-one [`RankGroup`].
/// `node_cluster` holds each node's [`node_cluster_paths`] entry.
fn grid_pairs(g: &Graph, node_cluster: &[Vec<usize>]) -> Vec<RankGroup> {
    let horizontal = |d: Direction| matches!(d, Direction::LR | Direction::RL);
    let n = g.nodes.len();
    // Per node: an out-edge or box edge (never loose), and its in-edge
    // sources.
    let mut fixed = vec![false; n];
    let mut sources: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in &g.edges {
        fixed[e.from] = true;
        if e.from == e.to {
            fixed[e.to] = true;
        } else {
            sources[e.to].push(e.from);
        }
    }
    for e in &g.sub_edges {
        for end in [e.from, e.to] {
            if let End::Node(v) = end {
                fixed[v] = true;
            }
        }
    }
    let mut out = Vec::new();
    for (si, s) in g.subgraphs.iter().enumerate() {
        if s.direction
            .is_some_and(|d| horizontal(d) != horizontal(g.direction))
        {
            continue;
        }
        // Loose members keyed by their one feeding node (None: edge-less).
        // A feeder inside a nested box is still inside this one.
        let mut sets: std::collections::BTreeMap<Option<usize>, Vec<usize>> =
            std::collections::BTreeMap::new();
        for &v in &s.nodes {
            if fixed[v] {
                continue;
            }
            let src = sources[v].first().copied();
            let one = sources[v].iter().all(|&u| Some(u) == src);
            if one && src.map_or(true, |u| !node_cluster[u].contains(&si)) {
                sets.entry(src).or_default().push(v);
            }
        }
        for (src, free) in sets {
            let least = if src.is_some() {
                GRID_MIN_MEMBERS
            } else {
                GRID_MIN_EDGELESS
            };
            if free.len() < least {
                continue;
            }
            let cols = (free.len() as f64).sqrt().ceil() as usize;
            for i in cols..free.len() {
                out.push((vec![free[i - cols]], vec![free[i]]));
            }
        }
    }
    out
}

/// Cluster path (outermost subgraph id first) for every node; empty
/// for a top-level node. Constrains the global clustered layout.
fn node_cluster_paths(g: &Graph) -> Vec<Vec<usize>> {
    let mut owner: Vec<Option<usize>> = vec![None; g.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &v in &s.nodes {
            owner[v] = Some(si);
        }
    }
    (0..g.nodes.len())
        .map(|v| {
            let mut path = Vec::new();
            let mut cur = owner[v];
            while let Some(s) = cur {
                path.push(s);
                cur = g.subgraphs[s].parent;
            }
            path.reverse(); // outermost first
            path
        })
        .collect()
}

/// Translate every node, edge and cluster box in a scene by `(dx, dy)`.
/// Mirror a scene along its FLOW axis (`horizontal=false` flips y in
/// `[0, extent]`, `true` flips x) — used by the serpentine fold to make
/// odd bands run backwards. Reflection is an isometry, so every curve,
/// waypoint and label the pipeline produced stays exactly as valid.
pub(crate) fn flip_scene(sc: &mut Scene, extent: f64, horizontal: bool) {
    let f = |p: (f64, f64)| -> (f64, f64) {
        if horizontal {
            (extent - p.0, p.1)
        } else {
            (p.0, extent - p.1)
        }
    };
    for n in &mut sc.nodes {
        let (x, y) = f((n.x, n.y));
        n.x = x;
        n.y = y;
    }
    for e in &mut sc.edges {
        for p in e.bezier.iter_mut() {
            *p = f(*p);
        }
        for p in e.waypoints.iter_mut() {
            *p = f(*p);
        }
        if let Some((_, m, _)) = &mut e.label {
            *m = f(*m);
        }
    }
    for c in &mut sc.clusters {
        // Boxes are top-left anchored: re-anchor after mirroring.
        if horizontal {
            c.x = extent - c.x - c.w;
        } else {
            c.y = extent - c.y - c.h;
        }
    }
}

/// `sc` moved right/down just enough that nothing sits inside the
/// top-left `MARGIN` gutter, canvas grown to match. [`route`] keeps
/// caller positions verbatim, so a node dragged past the origin would
/// otherwise fall outside the `0 0 W H` viewBox of renderers that
/// draw `width`/`height` as-is (class, ER). Scenes already clear of
/// the gutter — every laid-out one — come back untouched.
pub(crate) fn fit_top_left(sc: &Scene) -> std::borrow::Cow<'_, Scene> {
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    if !bb.0.is_finite() {
        return std::borrow::Cow::Borrowed(sc);
    }
    let dx = (MARGIN - bb.0).max(0.0);
    let dy = (MARGIN - bb.2).max(0.0);
    if dx == 0.0 && dy == 0.0 {
        return std::borrow::Cow::Borrowed(sc);
    }
    let mut out = sc.clone();
    shift_scene(&mut out, dx, dy);
    out.width += dx;
    out.height += dy;
    std::borrow::Cow::Owned(out)
}

pub(crate) fn shift_scene(sc: &mut Scene, dx: f64, dy: f64) {
    for n in &mut sc.nodes {
        n.x += dx;
        n.y += dy;
    }
    for e in &mut sc.edges {
        for p in e.bezier.iter_mut() {
            *p = (p.0 + dx, p.1 + dy);
        }
        for p in e.waypoints.iter_mut() {
            *p = (p.0 + dx, p.1 + dy);
        }
        if let Some((_, m, _)) = &mut e.label {
            *m = (m.0 + dx, m.1 + dy);
        }
    }
    for c in &mut sc.clusters {
        c.x += dx;
        c.y += dy;
    }
}

/// Re-route edges for custom node positions (e.g. after a drag).
/// `centers[i]` = centre of node i in final coordinates. Node
/// positions are NOT normalised (they stay exactly as given) so the
/// diagram doesn't "swim" while dragging; `to_svg` handles
/// translation at export time.
pub fn route(g: &Graph, centers: &[(f64, f64)]) -> Scene {
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    route_sized(g, centers, &sizes)
}

/// Same as [`route`] but with caller-provided node sizes.
pub fn route_sized(g: &Graph, centers: &[(f64, f64)], sizes: &[(f64, f64)]) -> Scene {
    assert_eq!(
        centers.len(),
        g.nodes.len(),
        "number of positions must match number of nodes"
    );
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );
    let mut sc = route_built(g, centers, sizes, true);
    crate::edgepass::finish(g, &mut sc, crate::edgepass::Mode::Routed);
    sc
}

/// The scene [`route_sized`] draws before the edge stages: free curves
/// and cross-cluster channels for the current boxes, frames wrapped
/// round the current nodes, and (when `place_labels`) labels slid off
/// what they cover. `centers` and `sizes` are index-parallel with
/// `g.nodes`.
fn route_built(
    g: &Graph,
    centers: &[(f64, f64)],
    sizes: &[(f64, f64)],
    place_labels: bool,
) -> Scene {
    let placed: Vec<Placed> = (0..g.nodes.len())
        .map(|i| Placed {
            b: centers[i].0,
            l: centers[i].1,
            bsize: sizes[i].0,
            lsize: sizes[i].1,
            layer: 0,
        })
        .collect();

    let nodes: Vec<SceneNode> = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| SceneNode {
            id: n.id.clone(),
            x: centers[i].0,
            y: centers[i].1,
            w: sizes[i].0,
            h: sizes[i].1,
            shape: n.shape,
            label: n.label.clone(),
            style: n.style.clone(),
            link: g.links.get(&i).and_then(NodeLink::sanitized),
        })
        .collect();

    // Cross-cluster edge routing — same as `scene_clustered` so the
    // interactive canvas matches the static layout. Uses the CURRENT
    // cluster boxes, so it follows drags too.
    let vertical = matches!(g.direction, Direction::TD | Direction::BT);
    let mut owner: Vec<Option<usize>> = vec![None; g.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &nn in &s.nodes {
            owner[nn] = Some(si);
        }
    }
    let top_of = |v: usize| -> Option<usize> {
        let mut s = owner[v]?;
        while let Some(p) = g.subgraphs[s].parent {
            s = p;
        }
        Some(s)
    };
    let (raw_boxes, _) = cluster_raw_boxes(g, &nodes);
    let box_of = |v: usize| -> (f64, f64, f64, f64) {
        top_of(v)
            .and_then(|s| raw_boxes.get(s).copied().flatten())
            .unwrap_or((
                centers[v].0 - sizes[v].0 / 2.0,
                centers[v].1 - sizes[v].1 / 2.0,
                sizes[v].0,
                sizes[v].1,
            ))
    };

    let offs = parallel_offsets(g);
    let mut edges = Vec::with_capacity(g.edges.len());
    for (e, off) in g.edges.iter().zip(offs) {
        let a = &placed[e.from];
        let b = &placed[e.to];
        let pts = free_edge(
            a,
            b,
            g.nodes[e.from].shape,
            g.nodes[e.to].shape,
            e.from == e.to,
            off,
            Some(g.direction),
        );
        let wps = if e.from != e.to && top_of(e.from) != top_of(e.to) {
            // Parallel edges between one pair get the same channel: each
            // runs `off` across the flow from it, so they never coincide.
            cross_cluster_route(
                centers[e.from],
                sizes[e.from],
                box_of(e.from),
                centers[e.to],
                sizes[e.to],
                box_of(e.to),
                vertical,
            )
            .into_iter()
            .map(|(x, y)| if vertical { (x + off, y) } else { (x, y + off) })
            .collect()
        } else {
            Vec::new()
        };
        let label = e.label.as_ref().map(|l| {
            let mid = if wps.is_empty() {
                cubic_mid(pts[0], pts[1], pts[2], pts[3])
            } else {
                wps[wps.len() / 2]
            };
            (l.clone(), mid, text_width(l) + 14.0)
        });
        edges.push(SceneEdge {
            from: g.nodes[e.from].id.clone(),
            to: g.nodes[e.to].id.clone(),
            bezier: pts,
            waypoints: wps,
            kind: e.kind,
            ends: e.ends,
            meta: e.meta.clone(),
            label,
        });
    }

    let clusters = route_clusters(g, &nodes);

    // Subgraph-touching edges: route against the current cluster
    // boxes so they follow drags too. `route_clusters` returns
    // boxes in subgraph order among the non-empty ones — match by
    // title-free lookup via the subgraph index.
    if !g.sub_edges.is_empty() {
        let (raw, _) = cluster_raw_boxes(g, &nodes);
        let mut boxes: HashMap<usize, BoxCS> = HashMap::new();
        for (si, b) in raw.iter().enumerate() {
            if let Some((x, y, w, h)) = *b {
                boxes.insert(si, ((x + w / 2.0, y + h / 2.0), (w, h)));
            }
        }
        let end_placed = |end: End| -> Option<(Placed, Shape)> {
            match end {
                End::Node(v) => Some((
                    Placed {
                        b: centers[v].0,
                        l: centers[v].1,
                        bsize: sizes[v].0,
                        lsize: sizes[v].1,
                        layer: 0,
                    },
                    g.nodes[v].shape,
                )),
                End::Sub(si) => boxes.get(&si).map(|&((cx, cy), (w, h))| {
                    (
                        Placed {
                            b: cx,
                            l: cy,
                            bsize: w,
                            lsize: h,
                            layer: 0,
                        },
                        Shape::Rect,
                    )
                }),
            }
        };
        let end_id = |end: End| -> String {
            match end {
                End::Node(v) => g.nodes[v].id.clone(),
                End::Sub(si) => g.subgraphs[si].id.clone(),
            }
        };
        let node_cluster = node_cluster_paths(g);
        let sub_offs = sub_parallel_offsets(g);
        for (e, &off) in g.sub_edges.iter().zip(&sub_offs) {
            let (Some((pa, sa)), Some((pb, sb))) = (end_placed(e.from), end_placed(e.to)) else {
                continue;
            };
            // A member drawn to (or from) its own enclosing box: the
            // static layout's stub, its label just outside the frame
            // (a label on the short inner run would sit on the member).
            let own = match (e.from, e.to) {
                (End::Node(v), End::Sub(si)) | (End::Sub(si), End::Node(v))
                    if node_cluster[v].contains(&si) =>
                {
                    raw.get(si).copied().flatten().map(|f| {
                        let exit = matches!(e.from, End::Node(_));
                        let lw = e.label.as_ref().map_or(0.0, |l| text_width(l) + 14.0);
                        own_box_stub(g.direction, &nodes[v], f, exit, lw)
                    })
                }
                _ => None,
            };
            let (pts, own_mid) = match own {
                Some((pts, mid)) => (pts, Some(mid)),
                None => (
                    free_edge(&pa, &pb, sa, sb, false, off, Some(g.direction)),
                    None,
                ),
            };
            let label = e.label.as_ref().map(|l| {
                (
                    l.clone(),
                    own_mid.unwrap_or_else(|| cubic_mid(pts[0], pts[1], pts[2], pts[3])),
                    text_width(l) + 14.0,
                )
            });
            edges.push(SceneEdge {
                from: end_id(e.from),
                to: end_id(e.to),
                bezier: pts,
                waypoints: Vec::new(),
                kind: e.kind,
                ends: e.ends,
                meta: e.meta.clone(),
                label,
            });
        }
    }

    // Labels off what they cover before the edge stages: the router
    // prices its paths round them, and the label stage then re-places
    // only the labels of the edges it changed.
    if place_labels {
        avoid_label_overlaps(&nodes, &clusters, &mut edges, LABEL_ROUTE_WORK);
    }
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &nodes, &edges, &clusters);
    let (_, maxx, _, maxy) = bb.finish();
    Scene {
        nodes,
        edges,
        clusters,
        width: maxx + MARGIN,
        height: maxy + MARGIN,
    }
}

/// How close (px) an unmoved edge of [`route_partial`]'s base may come
/// to what a drag changed (a dragged node's old or new box, the strip a
/// frame grew by) and still keep its geometry: the clearance the router
/// keeps between an edge and an unrelated node
/// ([`crate::clearance::NODE_SEP`]).
const PARTIAL_CLEAR: f64 = crate::clearance::NODE_SEP;

/// Share of the edges above which [`route_partial`] hands the frame to
/// [`route`]: once most of the drawing is drawn afresh (a host that
/// compares every drag with the auto layout, after many nodes were
/// moved), keeping the rest saves little time, and the kept edges are
/// not spread round the redrawn ones the way a full repair spreads them.
const PARTIAL_ROUTE_SHARE: f64 = 0.5;

/// Re-route after a drag LOCALLY. Only the edges a drag can affect go
/// through the edge stages again: those ending on a moved node or on a
/// frame redrawn round one, and those whose `base` label covers or
/// touches a node or an arrowhead (with the edge owning that head),
/// are drawn afresh; those whose `base` path comes within
/// [`PARTIAL_CLEAR`] of a moved node's old or new box, of the strip a
/// frame grew by or of its moved title chip keep their `base` path but
/// are rerouted when it now hits something. Every other edge keeps its
/// `base` geometry (channel waypoints, label slot; only a port shared
/// with a redrawn edge is spaced again, and its label moves when it now
/// covers something) and is an obstacle the affected edges are routed
/// round (see [`crate::edgepass::Ctx::fixed`]), so a drag frame costs
/// what lies near the drag, not the whole drawing. A frame holding a
/// moved node is drawn round its members as [`route`] draws it; other
/// frames keep their `base` box. When more than
/// [`PARTIAL_ROUTE_SHARE`] of the edges would be drawn afresh the frame
/// is a plain [`route`], at its cost.
///
/// `base` must be a full drawing of the graph at `base_centers`: the
/// [`scene`] output with `base_centers` its node centres (what an editor
/// keeps from its auto layout), or a [`route`] of `base_centers`. Do not
/// chain a previous `route_partial` output as `base`: what it kept from
/// its own base is not a full drawing, and once its frames no longer
/// fit their members the drag falls back to [`route`] and pays its full
/// cost.
/// Falls back to [`route`] whenever `base` no longer lines up with the
/// graph (e.g. after an edit to nodes, edges, edge labels or styles, or
/// subgraphs).
///
/// # Panics
///
/// Like [`route`], when `centers.len()` differs from the graph's node
/// count.
pub fn route_partial(
    g: &Graph,
    centers: &[(f64, f64)],
    base: &Scene,
    base_centers: &[(f64, f64)],
) -> Scene {
    let n = g.nodes.len();
    if base.nodes.len() != n || base_centers.len() != n || centers.len() != n {
        return route(g, centers);
    }
    let moved: Vec<bool> = (0..n)
        .map(|i| {
            (centers[i].0 - base_centers[i].0).abs() > 0.01
                || (centers[i].1 - base_centers[i].1).abs() > 0.01
        })
        .collect();
    if !moved.iter().any(|&m| m) {
        return base.clone();
    }
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    // No builder label pass: a redrawn edge's label starts at its spot
    // on the new line and the label stage moves it only if it covers
    // something. route() slides every label first so the router can
    // price paths round them; that second pass costs 3-4 ms of a dense
    // drag frame and is left out here (G13).
    let mut sc = route_built(g, centers, &sizes, false);
    // A kept edge is copied from `base` whole (style, end glyphs,
    // label text), and a node's box follows its text: an edit to any of
    // them must not keep stale geometry.
    let label_text = |e: &SceneEdge| e.label.as_ref().map(|l| l.0.clone());
    let lines_up = sc.edges.len() == base.edges.len()
        && sc.edges.iter().zip(&base.edges).all(|(a, b)| {
            a.from == b.from
                && a.to == b.to
                && a.kind == b.kind
                && a.ends == b.ends
                && a.meta == b.meta
                && label_text(a) == label_text(b)
        })
        && sc
            .nodes
            .iter()
            .zip(&base.nodes)
            .all(|(a, b)| a.id == b.id && a.label == b.label && a.shape == b.shape)
        && clusters_line_up(g, base);
    if !lines_up {
        return route(g, centers);
    }
    type R = (f64, f64, f64, f64);
    let node_rect = |nd: &SceneNode| -> R {
        (
            nd.x - nd.w / 2.0,
            nd.y - nd.h / 2.0,
            nd.x + nd.w / 2.0,
            nd.y + nd.h / 2.0,
        )
    };
    // What changed: the moved nodes' old and new boxes, and the strips
    // each frame holding a moved node grows by (it keeps its base box,
    // grown over the new one).
    let mut zones: Vec<R> = Vec::new();
    for v in (0..n).filter(|&v| moved[v]) {
        zones.push(node_rect(&base.nodes[v]));
        zones.push(node_rect(&sc.nodes[v]));
    }
    let paths = node_cluster_paths(g);
    let mut grew: Vec<String> = Vec::new();
    let mut clusters = base.clusters.clone();
    for c in clusters.iter_mut() {
        let holds_moved = (0..n).any(|v| {
            moved[v]
                && paths[v]
                    .iter()
                    .any(|&si| g.subgraphs.get(si).is_some_and(|s| s.id == c.id))
        });
        let Some(ci) = sc.clusters.iter().position(|f| f.id == c.id) else {
            continue;
        };
        let now = &sc.clusters[ci];
        if !holds_moved {
            continue;
        }
        let o: R = (c.x, c.y, c.x + c.w, c.y + c.h);
        let f: R = (now.x, now.y, now.x + now.w, now.y + now.h);
        if f == o {
            continue;
        }
        // The frame is redrawn round its members as route() draws it;
        // what it now covers beyond its base box, and its title chip,
        // is where kept edges may now hit it.
        let u: R = (o.0.min(f.0), o.1.min(f.1), o.2.max(f.2), o.3.max(f.3));
        for strip in [
            (u.0, u.1, o.0, u.3),
            (o.2, u.1, u.2, u.3),
            (u.0, u.1, u.2, o.1),
            (u.0, o.3, u.2, u.3),
        ] {
            if strip.2 > strip.0 && strip.3 > strip.1 {
                zones.push(strip);
            }
        }
        zones.push(crate::labels::chip_rect(&sc.clusters, ci, f64::INFINITY));
        *c = now.clone();
        grew.push(c.id.clone());
    }
    let moved_end =
        |id: &str| grew.iter().any(|f| f == id) || g.node_index(id).is_some_and(|v| moved[v]);
    let near = |e: &SceneEdge| {
        let pts = edge_polyline(e);
        zones.iter().any(|&z| {
            let z = (
                z.0 - PARTIAL_CLEAR,
                z.1 - PARTIAL_CLEAR,
                z.2 + PARTIAL_CLEAR,
                z.3 + PARTIAL_CLEAR,
            );
            pts.windows(2).any(|w| seg_meets_rect(w[0], w[1], z))
        })
    };
    // A base label on (or touching) a node or an arrowhead stays there,
    // or lands on the head once its port is respaced, while the lines
    // are kept: the label's edge and the head's edge are drawn afresh
    // like a moved node's (G11).
    let clash = crate::labels::label_clashes(&base.nodes, &base.edges);
    let mut fixed = vec![true; base.edges.len()];
    let mut redrawn = 0;
    for (i, e) in sc.edges.iter_mut().enumerate() {
        let b = &base.edges[i];
        if moved_end(&b.from) || moved_end(&b.to) || clash[i] {
            fixed[i] = false;
            redrawn += 1;
        } else {
            fixed[i] = !near(b);
            *e = b.clone();
        }
    }
    // Most of the drawing redrawn afresh (not merely near the drag): a
    // full route() costs about the same and draws it better.
    if redrawn as f64 > PARTIAL_ROUTE_SHARE * fixed.len() as f64 {
        return route(g, centers);
    }
    sc.clusters = clusters;
    // The base canvas already covers the kept geometry; the pipeline's
    // refit grows it for whatever the drag pushed outward.
    sc.width = base.width;
    sc.height = base.height;
    crate::edgepass::finish_fixed(g, &mut sc, crate::edgepass::Mode::Routed, &fixed);
    sc
}

/// Whether `base`'s frames still stand for `g`'s subgraphs, so
/// [`route_partial`] may keep them: one frame per non-empty subgraph,
/// with the same id, title and depth, holding every member's box and
/// no other node's centre. An edit that only renames a subgraph or
/// moves a node into or out of one fails this, and the drag falls back
/// to [`route`].
fn clusters_line_up(g: &Graph, base: &Scene) -> bool {
    let fresh = route_clusters(g, &base.nodes);
    if fresh.len() != base.clusters.len() {
        return false;
    }
    let paths = node_cluster_paths(g);
    base.clusters.iter().all(|c| {
        let Some(f) = fresh.iter().find(|f| f.id == c.id) else {
            return false;
        };
        if f.title != c.title || f.depth != c.depth {
            return false;
        }
        let (x0, y0, x1, y1) = (c.x, c.y, c.x + c.w, c.y + c.h);
        base.nodes.iter().enumerate().all(|(v, nd)| {
            let member = paths[v]
                .iter()
                .any(|&si| g.subgraphs.get(si).is_some_and(|s| s.id == c.id));
            if member {
                nd.x - nd.w / 2.0 >= x0 - CLUSTER_FIT_EPS
                    && nd.y - nd.h / 2.0 >= y0 - CLUSTER_FIT_EPS
                    && nd.x + nd.w / 2.0 <= x1 + CLUSTER_FIT_EPS
                    && nd.y + nd.h / 2.0 <= y1 + CLUSTER_FIT_EPS
            } else {
                !(nd.x > x0 && nd.x < x1 && nd.y > y0 && nd.y < y1)
            }
        })
    })
}

/// Slack (px) [`clusters_line_up`] allows a member's box past its
/// frame's edge: float noise from the layout's arithmetic, far below
/// any padding a frame draws.
const CLUSTER_FIT_EPS: f64 = 0.5;

/// Whether segment `a`-`b` meets the closed rect `r` `(x0, y0, x1, y1)`
/// (Liang-Barsky clipping).
fn seg_meets_rect(a: (f64, f64), b: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - r.0),
        (dx, r.2 - a.0),
        (-dy, a.1 - r.1),
        (dy, r.3 - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

/// Per-subgraph box `(x, y, w, h)` wrapped around the CURRENT node
/// positions, indexed by subgraph. Computed deepest-first so a
/// parent encloses its children's boxes. `None` = empty subgraph.
/// Also returns each subgraph's nesting depth.
fn cluster_raw_boxes(g: &Graph, nodes: &[SceneNode]) -> (Vec<RawBox>, Vec<usize>) {
    let nsub = g.subgraphs.len();
    let mut depth = vec![0usize; nsub];
    for i in 0..nsub {
        let mut p = g.subgraphs[i].parent;
        while let Some(pi) = p {
            depth[i] += 1;
            p = g.subgraphs[pi].parent;
        }
    }
    let mut order: Vec<usize> = (0..nsub).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(depth[i]));
    let mut boxes: Vec<RawBox> = vec![None; nsub];
    for &i in &order {
        let mut bb = Bbox::new();
        for &v in &g.subgraphs[i].nodes {
            let n = &nodes[v];
            bb.add(n.x - n.w / 2.0, n.y - n.h / 2.0);
            bb.add(n.x + n.w / 2.0, n.y + n.h / 2.0);
        }
        for (j, s) in g.subgraphs.iter().enumerate() {
            if s.parent == Some(i) {
                if let Some((x, y, w, h)) = boxes[j] {
                    bb.add(x, y);
                    bb.add(x + w, y + h);
                }
            }
        }
        if !bb.0.is_finite() {
            continue; // empty subgraph: nothing to wrap
        }
        let (minx, maxx, miny, maxy) = bb.finish();
        boxes[i] = Some((
            minx - SUB_PAD,
            miny - SUB_PAD - SUB_HEADER - title_extra(&g.subgraphs[i].title),
            (maxx - minx) + 2.0 * SUB_PAD,
            (maxy - miny) + 2.0 * SUB_PAD + SUB_HEADER + title_extra(&g.subgraphs[i].title),
        ));
    }
    (boxes, depth)
}

/// Cluster boxes as paintable [`SceneCluster`]s, outermost-first.
fn route_clusters(g: &Graph, nodes: &[SceneNode]) -> Vec<SceneCluster> {
    if g.subgraphs.is_empty() {
        return Vec::new();
    }
    let (boxes, depth) = cluster_raw_boxes(g, nodes);
    let mut out: Vec<SceneCluster> = (0..g.subgraphs.len())
        .filter_map(|i| {
            boxes[i].map(|(x, y, w, h)| SceneCluster {
                id: g.subgraphs[i].id.clone(),
                x,
                y,
                w,
                h,
                title: g.subgraphs[i].title.clone(),
                depth: depth[i],
            })
        })
        .collect();
    out.sort_by_key(|c| c.depth);
    out
}

/// Cubic bezier for a single edge between two freely-positioned
/// rectangular boxes — the same per-edge geometry [`route`] uses,
/// exposed for hosts that own their node model (icon nodes, custom
/// canvases) and only want flowmaid's curve: side-aware anchors
/// with fan-out spreading, and self-loops.
///
/// `offset` separates parallel edges between the same pair (pass 0
/// when unused); `self_loop` draws the loop stub instead (`a` is
/// used, `b` ignored). Returns `[start, control1, control2, end]`
/// in the same coordinate space as the inputs.
pub fn box_edge_bezier(
    a_center: (f64, f64),
    a_size: (f64, f64),
    b_center: (f64, f64),
    b_size: (f64, f64),
    offset: f64,
    self_loop: bool,
) -> [(f64, f64); 4] {
    let make = |c: (f64, f64), s: (f64, f64)| Placed {
        b: c.0,
        l: c.1,
        bsize: s.0,
        lsize: s.1,
        layer: 0,
    };
    free_edge(
        &make(a_center, a_size),
        &make(b_center, b_size),
        Shape::Rect,
        Shape::Rect,
        self_loop,
        offset,
        None,
    )
}

/// The `<marker>` id of an unstyled edge end's glyph. The end arrow
/// keeps its historical `#arrow` id so marker-free output is unchanged.
fn marker_base_id(start: bool, m: Marker) -> &'static str {
    match (start, m) {
        (false, Marker::Arrow) => "arrow",
        (true, Marker::Arrow) => "arrow-start",
        (false, Marker::Cross) => "cross-end",
        (true, Marker::Cross) => "cross-start",
        (false, Marker::Circle) => "circle-end",
        (true, Marker::Circle) => "circle-start",
    }
}

/// `marker-start` / `marker-end` attribute for one edge end (empty
/// when the end has no glyph).
fn marker_attr(side: &str, id: Option<&str>) -> String {
    match id {
        Some(id) => format!(" marker-{side}=\"url(#{id})\""),
        None => String::new(),
    }
}

/// The XLink namespace, declared on each `<a>` so `xlink:href` works
/// in SVG 1.1 tools without touching the root `<svg>` header.
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

/// `<a …>` opening a `click`-linked node: `href` for SVG 2 browsers,
/// `xlink:href` for SVG 1.1 tools, `target` only when the author gave
/// one (no target = the same tab, as in mermaid), and
/// `rel="noopener noreferrer"` with `_blank`.
fn svg_link_open(url: &str, target: Option<crate::model::LinkTarget>) -> String {
    let u = escape(url);
    let mut out = format!("<a xmlns:xlink=\"{XLINK_NS}\" href=\"{u}\" xlink:href=\"{u}\"");
    if let Some(t) = target {
        out.push_str(&format!(" target=\"{}\"", t.as_str()));
        if t == crate::model::LinkTarget::Blank {
            out.push_str(" rel=\"noopener noreferrer\"");
        }
    }
    out.push_str(">\n");
    out
}

/// Default stroke width of a drawn edge, and of a thick (`==>`) one.
pub(crate) const EDGE_WIDTH: f64 = 1.7;
pub(crate) const EDGE_WIDTH_THICK: f64 = 3.4;
/// Dash pattern of a dotted (`-.->`) edge.
const EDGE_DOTTED_DASH: &str = "5 4";
/// Size of the classic arrowhead in `#arrow`'s units (multiples of the
/// stroke width).
const ARROW_MARKER_UNITS: f64 = 7.0;
/// Arrowhead size in user space for an edge whose `linkStyle` sets a
/// stroke width: the size `#arrow` has on a default edge, so a wide
/// styled line keeps a normal head (mermaid's markers don't scale).
pub(crate) const ARROW_MARKER_PX: f64 = ARROW_MARKER_UNITS * EDGE_WIDTH;

/// A marker variant a `linkStyle`d edge end needs: the glyph in the
/// edge's stroke colour and/or at a fixed user-space size.
#[derive(Clone, PartialEq)]
struct MarkerVariant {
    marker: Marker,
    start: bool,
    color: Option<String>,
    /// Sized in user space rather than stroke-width units (arrows only;
    /// cross and circle always are).
    user_space: bool,
}

/// The marker ids every drawn edge end uses, plus the defs of the
/// styled variants among them. An unstyled end keeps its base id, so a
/// diagram without `linkStyle` gets exactly the defs it always had. A
/// styled id is the base id, `-u` for a fixed-size arrow, then `_` and
/// the colour with every non-alphanumeric character as `_` (mermaid's
/// marker id rule); two colours that sanitise alike get a counter.
struct EdgeMarkers {
    /// Per scene edge: (start id, end id).
    ids: Vec<(Option<String>, Option<String>)>,
    defs: String,
}

fn edge_markers(edges: &[SceneEdge]) -> EdgeMarkers {
    let mut made: Vec<(MarkerVariant, String)> = Vec::new();
    let mut defs = String::new();
    let mut id_for = |e: &SceneEdge, start: bool| -> Option<String> {
        if !edge_drawn(e) {
            return None;
        }
        let m = if start {
            e.ends.start_marker(e.kind)
        } else {
            e.ends.end_marker(e.kind)
        }?;
        let st = &e.meta.style;
        let v = MarkerVariant {
            marker: m,
            start,
            color: st.stroke.clone(),
            user_space: m == Marker::Arrow && st.stroke_width.is_some(),
        };
        if v.color.is_none() && !v.user_space {
            return Some(marker_base_id(start, m).to_string());
        }
        if let Some((_, id)) = made.iter().find(|(w, _)| *w == v) {
            return Some(id.clone());
        }
        let mut id = marker_base_id(start, m).to_string();
        if v.user_space {
            id.push_str("-u");
        }
        if let Some(c) = &v.color {
            id.push('_');
            id.extend(
                c.chars()
                    .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }),
            );
        }
        let base = id.clone();
        let mut n = 1;
        while made.iter().any(|(_, x)| *x == id) {
            n += 1;
            id = format!("{base}_{n}");
        }
        defs.push_str(&marker_def(&id, &v));
        made.push((v, id.clone()));
        Some(id)
    };
    let ids = edges
        .iter()
        .map(|e| (id_for(e, true), id_for(e, false)))
        .collect();
    EdgeMarkers { ids, defs }
}

/// The `<marker>` def of one styled variant: the same geometry as the
/// base glyph, painted in the edge colour (arrow fill, cross stroke,
/// circle fill and stroke).
fn marker_def(id: &str, v: &MarkerVariant) -> String {
    let color = style_attr(v.color.as_deref(), EDGE_COLOR);
    match v.marker {
        Marker::Arrow => {
            let (refx, d) = if v.start {
                ("1.5", "M 10 1 L 1 5 L 10 9 z")
            } else {
                ("8.5", "M 0 1 L 9 5 L 0 9 z")
            };
            let size = if v.user_space {
                format!(
                    "markerUnits=\"userSpaceOnUse\" markerWidth=\"{ARROW_MARKER_PX:.1}\" \
                     markerHeight=\"{ARROW_MARKER_PX:.1}\""
                )
            } else {
                format!(
                    "markerWidth=\"{ARROW_MARKER_UNITS}\" markerHeight=\"{ARROW_MARKER_UNITS}\""
                )
            };
            format!(
                "<marker id=\"{id}\" viewBox=\"0 0 10 10\" refX=\"{refx}\" refY=\"5\" \
                 {size} orient=\"auto\"><path d=\"{d}\" fill=\"{color}\"/></marker>"
            )
        }
        Marker::Cross => format!(
            "<marker id=\"{id}\" viewBox=\"0 0 11 11\" refX=\"{}\" refY=\"5.5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"11\" markerHeight=\"11\" \
             orient=\"auto\"><path d=\"M 1 1 L 10 10 M 10 1 L 1 10\" fill=\"none\" \
             stroke=\"{color}\" stroke-width=\"2\"/></marker>",
            if v.start { 0 } else { 11 }
        ),
        Marker::Circle => format!(
            "<marker id=\"{id}\" viewBox=\"0 0 10 10\" refX=\"{}\" refY=\"5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"10\" markerHeight=\"10\" \
             orient=\"auto\"><circle cx=\"5\" cy=\"5\" r=\"4\" fill=\"{color}\" \
             stroke=\"{color}\" stroke-width=\"1\"/></marker>",
            if v.start { 0 } else { 10 }
        ),
    }
}

/// Whether an edge is painted: everything but `~~~`, which shows only
/// when a `linkStyle` gives it a positive stroke width (mermaid's
/// inline style beats the invisible class's `stroke-width:0`), and then
/// without markers.
pub(crate) fn edge_drawn(e: &SceneEdge) -> bool {
    !matches!(e.kind, EdgeKind::Invisible) || e.meta.style.stroke_width.is_some_and(|w| w > 0.0)
}

/// `<marker>` defs beyond the classic end arrow, emitted only for the
/// glyphs some drawn edge actually uses — a diagram without `<-->`,
/// `--x` or `--o` gets exactly the defs it always had. Cross and
/// circle are sized in user space (mermaid-like, independent of the
/// stroke width) and sit just outside the node border: the path still
/// ends ON the border, the glyph is offset back along the tangent.
fn extra_marker_defs(ids: &[(Option<String>, Option<String>)]) -> String {
    let uses = |start: bool, m: Marker| {
        let base = marker_base_id(start, m);
        ids.iter().any(|(s, e)| {
            let got = if start { s } else { e };
            got.as_deref() == Some(base)
        })
    };
    let mut out = String::new();
    if uses(true, Marker::Arrow) {
        // Mirror of `#arrow`: tip at x=1 pointing back at the source.
        out.push_str(&format!(
            "<marker id=\"arrow-start\" viewBox=\"0 0 10 10\" refX=\"1.5\" refY=\"5\" \
             markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
             <path d=\"M 10 1 L 1 5 L 10 9 z\" fill=\"{}\"/></marker>",
            EDGE_COLOR
        ));
    }
    for (start, id, refx) in [(false, "cross-end", 11), (true, "cross-start", 0)] {
        if uses(start, Marker::Cross) {
            out.push_str(&format!(
                "<marker id=\"{id}\" viewBox=\"0 0 11 11\" refX=\"{refx}\" refY=\"5.5\" \
                 markerUnits=\"userSpaceOnUse\" markerWidth=\"11\" markerHeight=\"11\" \
                 orient=\"auto\"><path d=\"M 1 1 L 10 10 M 10 1 L 1 10\" fill=\"none\" \
                 stroke=\"{}\" stroke-width=\"2\"/></marker>",
                EDGE_COLOR
            ));
        }
    }
    for (start, id, refx) in [(false, "circle-end", 10), (true, "circle-start", 0)] {
        if uses(start, Marker::Circle) {
            out.push_str(&format!(
                "<marker id=\"{id}\" viewBox=\"0 0 10 10\" refX=\"{refx}\" refY=\"5\" \
                 markerUnits=\"userSpaceOnUse\" markerWidth=\"10\" markerHeight=\"10\" \
                 orient=\"auto\"><circle cx=\"5\" cy=\"5\" r=\"4\" fill=\"{0}\" \
                 stroke=\"{0}\" stroke-width=\"1\"/></marker>",
                EDGE_COLOR
            ));
        }
    }
    out
}

/// Prefix of the per-diagram DOM id (`flowmaid-<hash>`) written on the
/// root `<svg>` when the diagram needs one — like mermaid's
/// `mermaid-<n>` svg id, it namespaces the animation CSS and user edge
/// ids so two SVGs inlined on one page never share them.
const DIAGRAM_DOM_ID_PREFIX: &str = "flowmaid-";
/// Mermaid's edge-animation CSS (`styles.ts`), ported verbatim: a
/// `9,5` dash marching from offset 900 to 0, every `FAST`/`SLOW`
/// seconds. `!important` overrides a dotted edge's own dash, as there.
const EDGE_ANIMATION_DASH: &str = "9,5";
const EDGE_ANIMATION_DASH_OFFSET: u32 = 900;
const EDGE_ANIMATION_FAST_SECS: u32 = 20;
const EDGE_ANIMATION_SLOW_SECS: u32 = 50;
/// Mermaid's keyframes name; written as `<diagram-id>-dash` so it
/// cannot clash with a host page's own `dash` animation.
const EDGE_ANIMATION_KEYFRAMES: &str = "dash";

/// Mermaid's CSS class for an animated edge.
fn edge_animation_class(a: EdgeAnimation) -> &'static str {
    match a {
        EdgeAnimation::Fast => "edge-animation-fast",
        EdgeAnimation::Slow => "edge-animation-slow",
    }
}

/// The root `<svg>` id, or `None` when no drawn edge carries an id or
/// an animation — those diagrams keep their historical bytes. The id
/// hashes the diagram's content (see [`DefsKey`]), so the same input
/// always gets the same id and different diagrams practically never
/// share one. Two byte-identical diagrams inlined on one page do share
/// it; their edge ids collide as their content does.
fn diagram_dom_id(sc: &Scene, title: &str, desc: Option<&str>) -> Option<String> {
    let needed = sc
        .edges
        .iter()
        .any(|e| edge_drawn(e) && (e.meta.id.is_some() || e.meta.animation.is_some()));
    if !needed {
        return None;
    }
    let mut key = DefsKey::new();
    key.eat(title.as_bytes());
    key.eat(&[0]);
    key.eat(desc.unwrap_or_default().as_bytes());
    for n in &sc.nodes {
        key.eat(&[0]);
        key.eat(n.id.as_bytes());
        key.eat(&[0]);
        key.eat(n.label.as_bytes());
        key.eat_f64(n.x);
        key.eat_f64(n.y);
    }
    for e in &sc.edges {
        key.eat(&[0]);
        key.eat(e.from.as_bytes());
        key.eat(&[0]);
        key.eat(e.to.as_bytes());
        key.eat(&[0]);
        key.eat(e.meta.id.as_deref().unwrap_or_default().as_bytes());
        key.eat(&[e.meta.animation.map_or(0, |a| a as u8 + 1)]);
        for &(x, y) in e.bezier.iter().chain(&e.waypoints) {
            key.eat_f64(x);
            key.eat_f64(y);
        }
        if let Some((text, ..)) = &e.label {
            key.eat(text.as_bytes());
        }
    }
    Some(format!("{DIAGRAM_DOM_ID_PREFIX}{}", key.finish()))
}

/// ` id="<diagram-id>-…" data-id="…"` for a user edge id (mermaid's
/// `${diagramId}-${edge.id}` plus the raw `data-id`) and ` class="…"`
/// for an animated edge; empty for a plain edge, so output is
/// unchanged. `diagram_id` is `Some` whenever `m` has an id.
fn edge_meta_attrs(m: &EdgeMeta, diagram_id: Option<&str>) -> String {
    let mut out = String::new();
    if let (Some(id), Some(dom)) = (&m.id, diagram_id) {
        out.push_str(&format!(" id=\"{dom}-{0}\" data-id=\"{0}\"", escape(id)));
    }
    if let Some(a) = m.animation {
        out.push_str(&format!(" class=\"{}\"", edge_animation_class(a)));
    }
    out
}

/// The animation `<style>`, written once and only when some drawn edge
/// is animated. Every rule is scoped to the root `<svg>`'s id and the
/// keyframes carry it in their name, as mermaid namespaces its styles,
/// so inlined SVGs never restyle the host page or each other.
fn edge_animation_css(edges: &[SceneEdge], diagram_id: Option<&str>) -> String {
    let animated = edges
        .iter()
        .any(|e| e.meta.animation.is_some() && edge_drawn(e));
    let Some(dom) = diagram_id.filter(|_| animated) else {
        return String::new();
    };
    let frames = format!("{dom}-{EDGE_ANIMATION_KEYFRAMES}");
    let rule = |a: EdgeAnimation, secs: u32| {
        format!(
            "#{dom} .{}{{stroke-dasharray:{EDGE_ANIMATION_DASH} !important;\
             stroke-dashoffset:{EDGE_ANIMATION_DASH_OFFSET};\
             animation:{frames} {secs}s linear infinite;stroke-linecap:round}}",
            edge_animation_class(a)
        )
    };
    format!(
        "<style>@keyframes {frames}{{to{{stroke-dashoffset:0}}}} {} {}</style>",
        rule(EdgeAnimation::Slow, EDGE_ANIMATION_SLOW_SECS),
        rule(EdgeAnimation::Fast, EDGE_ANIMATION_FAST_SECS)
    )
}

/// Serialise any Scene (automatic or dragged) to SVG. Content is
/// translated to start at MARGIN, so negative coordinates are safe.
/// The accessible name is `"Flowchart diagram"`; use [`to_svg_titled`]
/// for another diagram class (e.g. state machines).
pub fn to_svg(sc: &Scene) -> String {
    to_svg_titled(sc, "Flowchart diagram")
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(sc: &Scene, opts: &SvgOptions) -> String {
    to_svg_titled_with(sc, "Flowchart diagram", opts)
}

/// [`to_svg`] with a caller-supplied accessible name — the root
/// `<title>` / `aria-label`. Callers that know the document type (a
/// `stateDiagram-v2` rides the flowchart `Scene`) pass the right one
/// so screen readers and hover tooltips announce it correctly (#16).
pub fn to_svg_titled(sc: &Scene, title: &str) -> String {
    to_svg_titled_with(sc, title, &SvgOptions::default())
}

/// [`to_svg_titled`] with explicit viewport options.
pub fn to_svg_titled_with(sc: &Scene, title: &str, opts: &SvgOptions) -> String {
    to_svg_a11y_with(sc, title, &A11y::default(), opts)
}

/// [`to_svg_titled_with`] plus mermaid's `accTitle` / `accDescr` (see
/// [`A11y`]): a non-empty `a11y.title` replaces `default_title`, and a
/// description adds the root `<desc>`. `render` / `render_svg` pass a
/// graph's `acc_title` / `acc_descr` through here; callers of the
/// interactive scene API pass them themselves.
pub fn to_svg_a11y_with(sc: &Scene, default_title: &str, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve(default_title);
    let title = title.as_str();
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    let (minx, maxx, miny, maxy) = bb.finish();
    let tx = MARGIN - minx;
    let ty = MARGIN - miny;
    let width = (maxx - minx) + 2.0 * MARGIN;
    let height = (maxy - miny) + 2.0 * MARGIN;
    let t = |p: (f64, f64)| (p.0 + tx, p.1 + ty);

    // Endpoint display name: pseudostates announce "start"/"end", not
    // their parser-synthesized `__start_*`/`__end_*` ids (#16). Keyed
    // on Shape (not the id prefix) so a user node literally named
    // "__start_x" is unaffected. BTreeMap keeps it zero-dep + ordered.
    let shape_of: std::collections::BTreeMap<&str, Shape> =
        sc.nodes.iter().map(|n| (n.id.as_str(), n.shape)).collect();
    let display = |id: &str| -> String {
        match shape_of.get(id) {
            Some(Shape::StateStart) => "start".to_string(),
            Some(Shape::StateEnd) => "end".to_string(),
            _ => id.to_string(),
        }
    };

    let dom_id = diagram_dom_id(sc, title, desc);
    let dom_id = dom_id.as_deref();
    let mut s = String::new();
    svg_open_root(&mut s, width, height, 14, title, desc, dom_id, opts);
    let markers = edge_markers(&sc.edges);
    let casing = crate::casing::masks(sc, &t, dom_id);
    s.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"{}\"/></marker>{}{}{}{}</defs>\n",
        EDGE_COLOR,
        extra_marker_defs(&markers.ids),
        markers.defs,
        edge_animation_css(&sc.edges, dom_id),
        casing.defs
    ));

    // Cluster boxes first (outermost-first), behind everything. The
    // TITLES are deferred to the very end (`cluster_titles`) so edges
    // passing the header strip never strike through the text — each
    // gets a translucent chip, mermaid-style.
    let mut cluster_titles = String::new();
    for (ci, c) in sc.clusters.iter().enumerate() {
        let (x, y) = t((c.x, c.y));
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\" \
             fill=\"#f7f8fd\" stroke=\"#c9cfe8\" stroke-width=\"1.4\"/>\n",
            x, y, c.w, c.h
        ));
        let title = fit_cluster_title(&sc.clusters, ci, maxx + MARGIN);
        let tw = text_width(&title) * TITLE_SCALE;
        cluster_titles.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{}\" rx=\"4\" \
             fill=\"#f7f8fd\" fill-opacity=\"0.9\"/>\n",
            x + 6.0,
            y + 5.0,
            tw + 8.0,
            {
                // Whole px print bare, as the one-line chip always has.
                let h = 16.0 + title_extra(&title);
                if h.fract() == 0.0 {
                    format!("{h}")
                } else {
                    format!("{h:.1}")
                }
            }
        ));
        for (k, line) in title.split('\n').enumerate() {
            cluster_titles.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{}\" font-weight=\"bold\" \
                 fill=\"#6a7086\">{}</text>\n",
                x + 10.0,
                y + 17.0 + k as f64 * TITLE_LINE_H,
                crate::layout::TITLE_FONT,
                rich(line)
            ));
        }
    }

    let mut edge_labels = String::new();
    for (i, (e, (m_start, m_end))) in sc.edges.iter().zip(&markers.ids).enumerate() {
        if !edge_drawn(e) {
            continue; // layout-only link — never drawn
        }
        let (dash, sw) = match e.kind {
            EdgeKind::Dotted | EdgeKind::DottedOpen => (Some(EDGE_DOTTED_DASH), EDGE_WIDTH),
            EdgeKind::Thick | EdgeKind::ThickOpen => (None, EDGE_WIDTH_THICK),
            _ => (None, EDGE_WIDTH),
        };
        // `linkStyle` paint overrides the kind's look per property.
        let st = &e.meta.style;
        let sw = st.stroke_width.unwrap_or(sw);
        let dash = st
            .stroke_dasharray
            .as_deref()
            .or(dash)
            .map_or(String::new(), |d| {
                format!(" stroke-dasharray=\"{}\"", escape(d))
            });
        let marker = format!(
            "{}{}",
            marker_attr("start", m_start.as_deref()),
            marker_attr("end", m_end.as_deref())
        );
        // The arrow marker orients to the end tangent (last segment).
        let path_d = edge_path_d(e, t);
        // <title> child = screen-reader / hover description (issue #16).
        // A two-way edge reads ↔ rather than →.
        let dir = if e.ends.start_marker(e.kind).is_some() {
            '\u{2194}'
        } else {
            '\u{2192}'
        };
        let edge_title = match &e.label {
            Some((text, ..)) => {
                format!(
                    "{} {dir} {}: {}",
                    display(&e.from),
                    display(&e.to),
                    plain_text(text)
                )
            }
            None => format!("{} {dir} {}", display(&e.from), display(&e.to)),
        };
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}>\
             <title>{}</title></path>\n",
            path_d,
            style_attr(st.fill.as_deref(), "none"),
            style_attr(st.stroke.as_deref(), EDGE_COLOR),
            sw,
            dash,
            marker,
            edge_meta_attrs(&e.meta, dom_id),
            casing.attr.get(i).map_or("", String::as_str),
            escape(&edge_title)
        ));
        if let Some((text, m, w)) = &e.label {
            let fill = style_attr(st.color.as_deref(), TEXT_COLOR);
            svg_label_box_with(&mut edge_labels, text, t(*m), *w, &fill);
        }
    }

    for n in &sc.nodes {
        let (cx, cy) = t((n.x, n.y));
        let (w, h) = (n.w, n.h);
        // Group per node so the <title> (a11y + hover tooltip) covers
        // shape AND label together (issue #16). An unlabelled node
        // (pseudostate / choice / fork) uses a shape-derived name, and
        // never emits an empty `<title>` (a11y anti-pattern).
        let node_title = {
            let lbl = plain_text(&n.label);
            if !lbl.is_empty() {
                lbl
            } else {
                match n.shape {
                    Shape::StateStart => "start".to_string(),
                    Shape::StateEnd => "end".to_string(),
                    Shape::ForkBar => "fork/join".to_string(),
                    _ => String::new(),
                }
            }
        };
        // A `click` tooltip replaces the label-derived title, and a
        // safe url wraps the whole group in a link.
        let link = n.link.as_ref();
        let node_title = link
            .and_then(|l| l.tooltip.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or(node_title);
        let href = link
            .and_then(|l| l.href.as_deref())
            .and_then(crate::model::sanitize_href);
        if let Some(u) = &href {
            s.push_str(&svg_link_open(u, link.and_then(|l| l.target)));
        }
        if node_title.is_empty() {
            s.push_str("<g>\n");
        } else {
            s.push_str(&format!("<g><title>{}</title>\n", escape(&node_title)));
        }
        // Shape theme, overridden by any custom style/classDef colors.
        let ss = crate::style::shape_style(n.shape);
        let style = format!(
            "fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"",
            style_attr(n.style.fill.as_deref(), ss.fill),
            style_attr(n.style.stroke.as_deref(), ss.stroke),
            n.style.stroke_width.unwrap_or(1.6)
        );
        match n.shape {
            Shape::Rect | Shape::Rounded | Shape::Stadium => {
                let rx = match n.shape {
                    Shape::Rounded => 9.0,
                    Shape::Stadium => h / 2.0,
                    _ => 3.0,
                };
                s.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                     rx=\"{:.1}\" {}/>\n",
                    cx - w / 2.0,
                    cy - h / 2.0,
                    w,
                    h,
                    rx,
                    style
                ));
            }
            Shape::Circle => {
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" {}/>\n",
                    cx,
                    cy,
                    w / 2.0,
                    style
                ));
            }
            Shape::Diamond => {
                s.push_str(&format!(
                    "<polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" {}/>\n",
                    cx,
                    cy - h / 2.0,
                    cx + w / 2.0,
                    cy,
                    cx,
                    cy + h / 2.0,
                    cx - w / 2.0,
                    cy,
                    style
                ));
            }
            Shape::DoubleCircle => {
                for r in [w / 2.0, w / 2.0 - 4.0] {
                    s.push_str(&format!(
                        "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.1}\" {style}/>\n"
                    ));
                }
            }
            Shape::Cylinder => {
                let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
                let ry = 8.0_f64.min(h / 4.0); // cap ellipse radius
                                               // Body + bottom arc, then the top ellipse on top.
                s.push_str(&format!(
                    "<path d=\"M {l:.1} {ty:.1} A {rx:.1} {ry:.1} 0 0 0 {r:.1} {ty:.1} \
                     L {r:.1} {by:.1} A {rx:.1} {ry:.1} 0 0 1 {l:.1} {by:.1} Z\" {style}/>\n\
                     <path d=\"M {l:.1} {ty:.1} A {rx:.1} {ry:.1} 0 0 1 {r:.1} {ty:.1}\" \
                     fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.6\"/>\n",
                    l = l,
                    r = r,
                    ty = t + ry,
                    by = b - ry,
                    rx = w / 2.0,
                    ry = ry,
                    stroke = style_attr(n.style.stroke.as_deref(), ss.stroke),
                    style = style,
                ));
            }
            Shape::Subroutine => {
                let (l, t) = (cx - w / 2.0, cy - h / 2.0);
                s.push_str(&format!(
                    "<rect x=\"{l:.1}\" y=\"{t:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"3\" {style}/>\n\
                     <line x1=\"{l1:.1}\" y1=\"{t:.1}\" x2=\"{l1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"1.6\"/>\n\
                     <line x1=\"{r1:.1}\" y1=\"{t:.1}\" x2=\"{r1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"1.6\"/>\n",
                    l = l, t = t, w = w, h = h, b = t + h, l1 = l + 8.0, r1 = l + w - 8.0,
                    stroke = style_attr(n.style.stroke.as_deref(), ss.stroke), style = style,
                ));
            }
            Shape::Hexagon => {
                let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
                let k = 14.0_f64.min(w / 4.0);
                s.push_str(&format!(
                    "<polygon points=\"{a:.1},{cy:.1} {b1:.1},{t:.1} {c:.1},{t:.1} {r:.1},{cy:.1} {c:.1},{b:.1} {b1:.1},{b:.1}\" {style}/>\n",
                    a = l, b1 = l + k, c = r - k, r = r, t = t, b = b, cy = cy, style = style,
                ));
            }
            Shape::Parallelogram | Shape::ParallelogramAlt => {
                let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
                let k = 14.0_f64.min(w / 4.0);
                let pts = if matches!(n.shape, Shape::Parallelogram) {
                    // bottom-left slanted right: /  /
                    format!(
                        "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                        l + k,
                        t,
                        r,
                        t,
                        r - k,
                        b,
                        l,
                        b
                    )
                } else {
                    // \  \
                    format!(
                        "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                        l,
                        t,
                        r - k,
                        t,
                        r,
                        b,
                        l + k,
                        b
                    )
                };
                s.push_str(&format!("<polygon points=\"{pts}\" {style}/>\n"));
            }
            Shape::StateStart => {
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" {}/>\n",
                    cx,
                    cy,
                    w / 2.0,
                    style
                ));
            }
            Shape::StateEnd => {
                // Outer ring + filled core (UML final-state notation).
                s.push_str(&format!(
                    "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.1}\" fill=\"#ffffff\" \
                     stroke=\"{stroke}\" stroke-width=\"1.6\"/>\n\
                     <circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{ri:.1}\" {style}/>\n",
                    r = w / 2.0,
                    ri = w / 2.0 - 4.0,
                    stroke = style_attr(n.style.stroke.as_deref(), ss.stroke),
                ));
            }
            Shape::ForkBar => {
                s.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" {}/>\n",
                    cx - w / 2.0,
                    cy - h / 2.0,
                    w,
                    h,
                    style
                ));
            }
        }
        svg_text_multiline(
            &mut s,
            cx,
            cy,
            &style_attr(n.style.color.as_deref(), TEXT_COLOR),
            &n.label,
        );
        s.push_str("</g>\n");
        if href.is_some() {
            s.push_str("</a>\n");
        }
    }

    s.push_str(&edge_labels);
    s.push_str(&cluster_titles);
    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------
// Internal geometry
// ---------------------------------------------------------------

/// Simple bounding box (minx, maxx, miny, maxy).
pub(crate) struct Bbox(pub f64, pub f64, pub f64, pub f64);
impl Bbox {
    pub(crate) fn new() -> Self {
        Bbox(
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        )
    }
    pub(crate) fn add(&mut self, x: f64, y: f64) {
        self.0 = self.0.min(x);
        self.1 = self.1.max(x);
        self.2 = self.2.min(y);
        self.3 = self.3.max(y);
    }
    pub(crate) fn finish(&self) -> (f64, f64, f64, f64) {
        if self.0.is_finite() {
            (self.0, self.1, self.2, self.3)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    }
}

pub(crate) fn grow_scene(
    bb: &mut Bbox,
    nodes: &[SceneNode],
    edges: &[SceneEdge],
    clusters: &[SceneCluster],
) {
    for c in clusters {
        bb.add(c.x, c.y);
        bb.add(c.x + c.w, c.y + c.h);
    }
    for n in nodes {
        bb.add(n.x - n.w / 2.0, n.y - n.h / 2.0);
        bb.add(n.x + n.w / 2.0, n.y + n.h / 2.0);
    }
    for e in edges {
        // Invisible links shape the layout but are never drawn, so
        // their curve must not inflate the canvas (bughunter: they
        // used to leave mysterious empty space).
        if !edge_drawn(e) {
            continue;
        }
        for &(x, y) in &e.bezier {
            bb.add(x, y);
        }
        // A non-default curve (catmullRom overshoots) is bounded by
        // what is drawn, not by the bezier hull.
        if let Some((start, segs)) = curve_geometry(e) {
            for (x, y) in curve_sample(start, &segs) {
                bb.add(x, y);
            }
        }
        if let Some((_, m, w)) = &e.label {
            bb.add(m.0 - w / 2.0, m.1 - 10.0);
            bb.add(m.0 + w / 2.0, m.1 + 10.0);
        }
    }
}

fn pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Per-edge offsets so parallel edges (same node pair) separate.
pub(crate) fn parallel_offsets(g: &Graph) -> Vec<f64> {
    let mut count: HashMap<(usize, usize), usize> = HashMap::new();
    for e in &g.edges {
        *count.entry(pair(e.from, e.to)).or_insert(0) += 1;
    }
    let mut seen: HashMap<(usize, usize), usize> = HashMap::new();
    g.edges
        .iter()
        .map(|e| {
            let k = pair(e.from, e.to);
            let i = *seen.entry(k).and_modify(|x| *x += 1).or_insert(0);
            (i as f64 - (count[&k] as f64 - 1.0) / 2.0) * PARALLEL_GAP
        })
        .collect()
}

/// [`parallel_offsets`] for `g.sub_edges`: several edges between one
/// node / frame pair (either way round) fan out [`PARALLEL_GAP`] apart.
fn sub_parallel_offsets(g: &Graph) -> Vec<f64> {
    let key = |e: End| match e {
        End::Node(v) => (0, v),
        End::Sub(s) => (1, s),
    };
    let pair_of = |e: &SubEdge| {
        let (a, b) = (key(e.from), key(e.to));
        (a.min(b), a.max(b))
    };
    let mut count: HashMap<_, usize> = HashMap::new();
    for e in &g.sub_edges {
        *count.entry(pair_of(e)).or_insert(0) += 1;
    }
    let mut seen: HashMap<_, usize> = HashMap::new();
    g.sub_edges
        .iter()
        .map(|e| {
            let k = pair_of(e);
            let i = *seen.entry(k).and_modify(|x| *x += 1).or_insert(0);
            (i as f64 - (count[&k] as f64 - 1.0) / 2.0) * PARALLEL_GAP
        })
        .collect()
}

/// Edge exit/entry point on a node's border.
///
/// For boxy shapes, points are spread along the top/bottom side
/// towards the opposite node (instead of piling up at the centre);
/// stadiums are constrained to their flat section so anchors don't
/// float on the rounded caps. Circles/diamonds use exact border
/// intersection.
pub(crate) fn anchor(
    p: &Placed,
    shape: Shape,
    other: (f64, f64),
    off: f64,
    bottom: bool,
) -> (f64, f64) {
    match shape {
        Shape::Diamond
        | Shape::Circle
        | Shape::DoubleCircle
        | Shape::StateStart
        | Shape::StateEnd => border(p, shape, (other.0 + off * 4.0, other.1)),
        _ => {
            let flat = match shape {
                Shape::Stadium => p.bsize / 2.0 - p.lsize / 2.0 - 4.0,
                _ => p.bsize / 2.0 - 12.0,
            }
            .max(0.0);
            let bias = ((other.0 - p.b) * 0.35 + off).clamp(-flat, flat);
            let l = if bottom {
                p.l + p.lsize / 2.0
            } else {
                p.l - p.lsize / 2.0
            };
            (p.b + bias, l)
        }
    }
}

/// Edge geometry for the layered layout (used by `scene`).
#[allow(clippy::too_many_arguments)]
fn edge_points(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    self_loop: bool,
    total_b: f64,
    off: f64,
    lay_ext: &[(f64, f64)],
    label_w: f64,
) -> [(f64, f64); 4] {
    if self_loop {
        return loop_points(a, off);
    }

    if b.layer != a.layer {
        let down = b.layer > a.layer;
        let p0 = anchor(a, sa, (b.b, b.l), off, down);
        let p3 = anchor(b, sb, (a.b, a.l), off, !down);

        if down {
            let dl = ((p3.1 - p0.1) * 0.5).max(20.0);
            if b.layer - a.layer > 1 && (a.b - b.b).abs() < 30.0 {
                // Mitigation: a long edge aligned with a column of
                // nodes bows sideways so it doesn't run behind the
                // nodes on intermediate layers. (Real fix: virtual nodes.)
                let bow = if a.b >= total_b / 2.0 {
                    60.0 + off
                } else {
                    -60.0 + off
                };
                return [
                    p0,
                    (p0.0 + bow, p0.1 + dl * 0.6),
                    (p3.0 + bow, p3.1 - dl * 0.6),
                    p3,
                ];
            }
            return [p0, (p0.0, p0.1 + dl), (p3.0, p3.1 - dl), p3];
        }

        // Back-edge: route around the nearest side of ALL layers it
        // crosses. Controls are solved analytically so the curve
        // apex (t=0.5) sits outside the outermost node by 24px PLUS
        // half the label box (the label rides the apex — without the
        // extra clearance it would overlap the nodes it bows around):
        // x(0.5) = 0.125*(x0+x3) + 0.75*BT.
        let (l0, l1) = (b.layer, a.layer);
        let mut ext_l = f64::INFINITY;
        let mut ext_r = f64::NEG_INFINITY;
        for li in l0..=l1 {
            ext_l = ext_l.min(lay_ext[li].0);
            ext_r = ext_r.max(lay_ext[li].1);
        }
        let clear = 24.0 + label_w / 2.0;
        let ends = p0.0 + p3.0;
        let mid = ends / 2.0;
        let bt_r = ((4.0 / 3.0) * (ext_r + clear) - ends / 6.0).max(ext_r + 40.0);
        let bt_l = ((4.0 / 3.0) * (ext_l - clear) - ends / 6.0).min(ext_l - 40.0);
        let bt = (if (bt_r - mid) <= (mid - bt_l) {
            bt_r
        } else {
            bt_l
        }) + off;
        return [p0, (bt, p0.1 - 40.0), (bt, p3.1 + 40.0), p3];
    }

    // Same layer: side to side, bowing downwards.
    let p0 = border(a, sa, (b.b, b.l));
    let p3 = border(b, sb, (a.b, a.l));
    let drop = a.lsize.max(b.lsize) / 2.0 + 22.0 + off;
    [
        p0,
        (p0.0 * 0.65 + p3.0 * 0.35, a.l + drop),
        (p0.0 * 0.35 + p3.0 * 0.65, b.l + drop),
        p3,
    ]
}

/// Edge geometry for free positions (used by `route`): the
/// vertical/horizontal classification follows the actual relative
/// position of the two nodes.
/// Waypoints (final coords) that route a cross-cluster edge OUT of its
/// source cluster/box and INTO the target's, so the edge threads the
/// gap between clusters instead of cutting through their interiors.
/// `a_box`/`b_box` = `(x, y, w, h)` of the endpoint's top-level cluster
/// (or the node's own box when it has no cluster). Returns empty when
/// the clusters aren't cleanly stacked along the flow axis (fall back
/// to the direct curve then).
fn cross_cluster_route(
    a_c: (f64, f64),
    a_sz: (f64, f64),
    a_box: (f64, f64, f64, f64),
    b_c: (f64, f64),
    b_sz: (f64, f64),
    b_box: (f64, f64, f64, f64),
    vertical: bool,
) -> Vec<(f64, f64)> {
    const GAP: f64 = 16.0;
    if vertical {
        let down = b_c.1 >= a_c.1;
        // Only route when the two boxes are clearly separated along the
        // flow axis (source fully above target, or vice-versa).
        let ok = if down {
            a_box.1 + a_box.3 < b_box.1
        } else {
            b_box.1 + b_box.3 < a_box.1
        };
        if !ok {
            return Vec::new();
        }
        let a_edge = if down {
            a_c.1 + a_sz.1 / 2.0
        } else {
            a_c.1 - a_sz.1 / 2.0
        };
        let a_out = if down {
            a_box.1 + a_box.3 + GAP
        } else {
            a_box.1 - GAP
        };
        let b_in = if down {
            b_box.1 - GAP
        } else {
            b_box.1 + b_box.3 + GAP
        };
        let b_edge = if down {
            b_c.1 - b_sz.1 / 2.0
        } else {
            b_c.1 + b_sz.1 / 2.0
        };
        vec![
            (a_c.0, a_edge),
            (a_c.0, a_out),
            (b_c.0, b_in),
            (b_c.0, b_edge),
        ]
    } else {
        let right = b_c.0 >= a_c.0;
        let ok = if right {
            a_box.0 + a_box.2 < b_box.0
        } else {
            b_box.0 + b_box.2 < a_box.0
        };
        if !ok {
            return Vec::new();
        }
        let a_edge = if right {
            a_c.0 + a_sz.0 / 2.0
        } else {
            a_c.0 - a_sz.0 / 2.0
        };
        let a_out = if right {
            a_box.0 + a_box.2 + GAP
        } else {
            a_box.0 - GAP
        };
        let b_in = if right {
            b_box.0 - GAP
        } else {
            b_box.0 + b_box.2 + GAP
        };
        let b_edge = if right {
            b_c.0 - b_sz.0 / 2.0
        } else {
            b_c.0 + b_sz.0 / 2.0
        };
        vec![
            (a_edge, a_c.1),
            (a_out, a_c.1),
            (b_in, b_c.1),
            (b_edge, b_c.1),
        ]
    }
}

/// Smallest border gap along the flow (px) for which two boxes count as
/// ranked one after the other; closer than that they sit side by side.
const FLOW_MIN_GAP: f64 = 12.0;
/// Preferred tangent length of a free curve (px), capped at half the
/// border gap so the curve never doubles back (G8).
const TANGENT_MIN: f64 = 24.0;
/// Share of the border gap a free curve's tangents take.
const TANGENT_SHARE: f64 = 0.45;
/// How far (px) a back-edge's controls reach back along the flow,
/// capped at half the border gap.
const BACK_LEAD: f64 = 40.0;
/// Clearance (px) of a back-edge's bow past the wider of its two ends.
const BACK_CLEAR: f64 = 24.0;
/// Distance (px) a side-by-side edge end keeps from a box's corners.
const SIDE_INSET: f64 = 8.0;

/// Tangent length for a curve spanning a border gap of `gap` px: never
/// more than half of it, so the two controls cannot pass each other and
/// the curve stays monotone.
fn tangent(gap: f64) -> f64 {
    (TANGENT_SHARE * gap).max(TANGENT_MIN.min(0.5 * gap))
}

/// A box in flow coordinates (breadth, layer) for direction `dir`.
fn flow_placed(dir: Direction, p: &Placed) -> Placed {
    let (b, l) = to_flow(dir, (p.b, p.l));
    let (bsize, lsize) = match dir {
        Direction::TD | Direction::BT => (p.bsize, p.lsize),
        Direction::LR | Direction::RL => (p.lsize, p.bsize),
    };
    Placed {
        b,
        l,
        bsize,
        lsize,
        layer: 0,
    }
}

/// Edge geometry between two free boxes in screen coordinates (`b` =
/// x, `l` = y), used by `route` and for sub-edges.
///
/// With a flow direction (`dir`), the sides follow the flow (G7): a
/// target ahead is left by the flow-out side and entered by its
/// flow-in side, so in `LR` a box is always entered from the left; a
/// target behind gets a back-edge that leaves the flow-in side and bows
/// past both boxes; boxes side by side join on their facing sides.
/// Without one (`dir = None`, a host's free boxes), the side follows
/// the dominant axis between the centres. Either way the tangents are
/// capped at half the border gap, so curves never double back (G8).
#[allow(clippy::too_many_arguments)]
fn free_edge(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    self_loop: bool,
    off: f64,
    dir: Option<Direction>,
) -> [(f64, f64); 4] {
    if self_loop {
        return loop_points(a, off);
    }
    let Some(dir) = dir else {
        return axis_edge(a, b, sa, sb, off);
    };
    let (fa, fb) = (flow_placed(dir, a), flow_placed(dir, b));
    if let Some(pts) = nested_edge(&fa, &fb, sa, sb, off) {
        return pts.map(|p| from_flow(dir, p));
    }
    let fwd_gap = (fb.l - fb.lsize / 2.0) - (fa.l + fa.lsize / 2.0);
    let back_gap = (fa.l - fa.lsize / 2.0) - (fb.l + fb.lsize / 2.0);
    let pts = if fwd_gap >= FLOW_MIN_GAP {
        let p0 = anchor(&fa, sa, (fb.b, fb.l), off, true);
        let p3 = anchor(&fb, sb, (fa.b, fa.l), off, false);
        let dl = tangent(p3.1 - p0.1);
        [p0, (p0.0, p0.1 + dl), (p3.0, p3.1 - dl), p3]
    } else if back_gap >= FLOW_MIN_GAP {
        // Out of the source's flow-in side, into the target's flow-out
        // side, bowing past the wider box on the side the source lies.
        let p0 = anchor(&fa, sa, (fb.b, fb.l), off, false);
        let p3 = anchor(&fb, sb, (fa.b, fa.l), off, true);
        let lead = BACK_LEAD.min(0.5 * (p0.1 - p3.1).max(0.0));
        let right = fa.b >= fb.b;
        let bt = if right {
            (fa.b + fa.bsize / 2.0).max(fb.b + fb.bsize / 2.0) + BACK_CLEAR
        } else {
            (fa.b - fa.bsize / 2.0).min(fb.b - fb.bsize / 2.0) - BACK_CLEAR
        } + off;
        [p0, (bt, p0.1 - lead), (bt, p3.1 + lead), p3]
    } else {
        // Side by side: the facing breadth sides.
        let s = if fb.b >= fa.b { 1.0 } else { -1.0 };
        let gap = (fb.b - fa.b).abs() - (fa.bsize + fb.bsize) / 2.0;
        if gap <= 0.0 {
            return axis_edge(a, b, sa, sb, off);
        }
        let p0 = side_point(&fa, sa, s, fb.l + off);
        let p3 = side_point(&fb, sb, -s, fa.l + off);
        let dl = tangent(gap);
        [p0, (p0.0 + s * dl, p0.1), (p3.0 - s * dl, p3.1), p3]
    };
    pts.map(|p| from_flow(dir, p))
}

/// A node joined to a box around it (a member and its own frame): a
/// straight run between the member's flow side and the frame side
/// nearer to it along the flow, like the static layout's stub. `None`
/// unless one box holds the other. Flow coordinates.
fn nested_edge(a: &Placed, b: &Placed, sa: Shape, sb: Shape, off: f64) -> Option<[(f64, f64); 4]> {
    let holds = |o: &Placed, i: &Placed| {
        o.b - o.bsize / 2.0 <= i.b - i.bsize / 2.0
            && i.b + i.bsize / 2.0 <= o.b + o.bsize / 2.0
            && o.l - o.lsize / 2.0 <= i.l - i.lsize / 2.0
            && i.l + i.lsize / 2.0 <= o.l + o.lsize / 2.0
    };
    let (outer, inner, si, a_inner) = if holds(b, a) {
        (b, a, sa, true)
    } else if holds(a, b) {
        (a, b, sb, false)
    } else {
        return None;
    };
    let (lo, hi) = (outer.l - outer.lsize / 2.0, outer.l + outer.lsize / 2.0);
    let s = if inner.l - lo < hi - inner.l {
        -1.0
    } else {
        1.0
    };
    let pi = anchor(inner, si, (inner.b, inner.l + s), off, s > 0.0);
    let po = (pi.0, if s > 0.0 { hi } else { lo });
    let dl = tangent((po.1 - pi.1).abs());
    let run = [pi, (pi.0, pi.1 + s * dl), (po.0, po.1 - s * dl), po];
    Some(if a_inner {
        run
    } else {
        [run[3], run[2], run[1], run[0]]
    })
}

/// The point on `p`'s breadth side facing `s` (+1 / -1), at the layer
/// coordinate nearest `toward` that stays clear of the corners; round
/// shapes and diamonds use their extreme point. Flow coordinates.
fn side_point(p: &Placed, shape: Shape, s: f64, toward: f64) -> (f64, f64) {
    match shape {
        Shape::Diamond
        | Shape::Circle
        | Shape::DoubleCircle
        | Shape::StateStart
        | Shape::StateEnd => border(p, shape, (p.b + s, p.l)),
        _ => {
            let room = (p.lsize / 2.0 - SIDE_INSET).max(0.0);
            (
                p.b + s * p.bsize / 2.0,
                // `max`/`min`, not `clamp`: a NaN centre must not panic.
                toward.max(p.l - room).min(p.l + room),
            )
        }
    }
}

/// The geometric rule for boxes with no flow (`box_edge_bezier`): the
/// dominant axis between the centres picks the sides.
fn axis_edge(a: &Placed, b: &Placed, sa: Shape, sb: Shape, off: f64) -> [(f64, f64); 4] {
    let dx = b.b - a.b;
    let dy = b.l - a.l;
    if dy.abs() >= dx.abs() {
        // Vertically dominant: exit bottom/top, enter top/bottom.
        let down = dy >= 0.0;
        let p0 = anchor(a, sa, (b.b, b.l), off, down);
        let p3 = anchor(b, sb, (a.b, a.l), off, !down);
        let s = if down { 1.0 } else { -1.0 };
        let dl = capped(dy.abs(), s * (p3.1 - p0.1));
        [p0, (p0.0, p0.1 + s * dl), (p3.0, p3.1 - s * dl), p3]
    } else {
        // Horizontally dominant: border to border.
        let p0 = border(a, sa, (b.b, b.l + off * 4.0));
        let p3 = border(b, sb, (a.b, a.l + off * 4.0));
        let s = if dx >= 0.0 { 1.0 } else { -1.0 };
        let dl = capped(dx.abs(), s * (p3.0 - p0.0));
        [p0, (p0.0 + s * dl, p0.1), (p3.0 - s * dl, p3.1), p3]
    }
}

/// The tangent for a border `gap` ([`tangent`], as with a flow) when
/// the borders do not overlap; else the legacy share of the centre
/// distance, at least [`TANGENT_MIN`].
fn capped(centre_dist: f64, gap: f64) -> f64 {
    if gap > 0.0 {
        tangent(gap)
    } else {
        (centre_dist * TANGENT_SHARE).max(TANGENT_MIN)
    }
}

/// Small self-loop on the node's right side; parallel loops stack
/// vertically.
fn loop_points(a: &Placed, off: f64) -> [(f64, f64); 4] {
    let r = a.b + a.bsize / 2.0;
    let ext = 48.0 + off.abs() * 0.8;
    [
        (r, a.l - 8.0 + off),
        (r + ext, a.l - 28.0 + off),
        (r + ext, a.l + 28.0 + off),
        (r, a.l + 8.0 + off),
    ]
}

/// Intersection of the line (from the node centre towards `toward`)
/// with the node's shape border — so arrows attach at the border,
/// not the centre.
fn border(p: &Placed, shape: Shape, toward: (f64, f64)) -> (f64, f64) {
    let dx = toward.0 - p.b;
    let dy = toward.1 - p.l;
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (p.b, p.l);
    }
    let hw = p.bsize / 2.0;
    let hh = p.lsize / 2.0;
    let t = match shape {
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            hw / (dx * dx + dy * dy).sqrt()
        }
        // Diamond: |x/hw| + |y/hh| = 1
        Shape::Diamond => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        // Other shapes are approximated as rectangles.
        _ => {
            let tx = if dx.abs() > 1e-6 {
                hw / dx.abs()
            } else {
                f64::INFINITY
            };
            let ty = if dy.abs() > 1e-6 {
                hh / dy.abs()
            } else {
                f64::INFINITY
            };
            tx.min(ty)
        }
    };
    (p.b + dx * t, p.l + dy * t)
}

/// Midpoint of a cubic bezier (t = 0.5), used to place labels.
fn cubic_mid(p0: (f64, f64), c1: (f64, f64), c2: (f64, f64), p3: (f64, f64)) -> (f64, f64) {
    (
        (p0.0 + 3.0 * c1.0 + 3.0 * c2.0 + p3.0) / 8.0,
        (p0.1 + 3.0 * c1.1 + 3.0 * c2.1 + p3.1) / 8.0,
    )
}

/// SVG path `d` for a smooth B-spline (d3's `curveBasis` — what
/// mermaid uses) through `pts` (>= 2 points): starts and ends exactly
/// at the endpoints while only *approximating* the channel waypoints,
/// so a routed edge flows in gentle lanes instead of ballooning
/// through every point the way an interpolating spline would.
fn spline_d(pts: &[(f64, f64)]) -> String {
    let n = pts.len();
    let mut d = format!("M {:.1} {:.1}", pts[0].0, pts[0].1);
    if n < 3 {
        d.push_str(&format!(" L {:.1} {:.1}", pts[n - 1].0, pts[n - 1].1));
        return d;
    }
    d.push_str(&format!(
        " L {:.1} {:.1}",
        (5.0 * pts[0].0 + pts[1].0) / 6.0,
        (5.0 * pts[0].1 + pts[1].1) / 6.0
    ));
    let bez = |d: &mut String, a: (f64, f64), b: (f64, f64), p: (f64, f64)| {
        d.push_str(&format!(
            " C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
            (2.0 * a.0 + b.0) / 3.0,
            (2.0 * a.1 + b.1) / 3.0,
            (a.0 + 2.0 * b.0) / 3.0,
            (a.1 + 2.0 * b.1) / 3.0,
            (a.0 + 4.0 * b.0 + p.0) / 6.0,
            (a.1 + 4.0 * b.1 + p.1) / 6.0
        ));
    };
    for i in 2..n {
        bez(&mut d, pts[i - 2], pts[i - 1], pts[i]);
    }
    bez(&mut d, pts[n - 2], pts[n - 1], pts[n - 1]);
    d.push_str(&format!(" L {:.1} {:.1}", pts[n - 1].0, pts[n - 1].1));
    d
}

/// Viewport / responsive rendering options for [`crate::render_svg_advanced`].
///
/// Defaults reproduce the historical fixed-pixel output byte-for-byte, so
/// existing callers (and `render_svg`) are unaffected. More options
/// (theme, scale, max width, …) can be added here without new entry
/// points. The struct is `#[non_exhaustive]`, so outside this crate build
/// it from [`Default`] and set the fields you need; adding a field then
/// never breaks a caller:
///
/// ```
/// let src = "flowchart TD\nA --> B";
/// let mut opts = flowmaid::SvgOptions::default();
/// opts.transparent = true;
/// let svg = flowmaid::render_svg_advanced(src, &opts).unwrap();
/// // Same output as the default, minus the white canvas `<rect>`.
/// let base = flowmaid::render_svg(src).unwrap();
/// let canvas = base.split("</title>\n").nth(1).unwrap().lines().next().unwrap();
/// assert!(canvas.ends_with("fill=\"#ffffff\"/>"));
/// assert_eq!(svg, base.replacen(&format!("{canvas}\n"), "", 1));
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct SvgOptions {
    /// When true, the root `<svg>` fills its container (`width="100%"`)
    /// instead of a fixed pixel size; the `viewBox` stays so the content
    /// scales with it.
    pub responsive: bool,
    /// Overrides the SVG `preserveAspectRatio` attribute. `None` keeps the
    /// SVG default `xMidYMid meet` (uniform scale, letterboxed).
    pub preserve_aspect_ratio: Option<&'static str>,
    /// When true, the full-canvas white background `<rect>` is omitted so
    /// the diagram sits on whatever is behind it (e.g. a dark page).
    /// Shapes, label boxes and text keep their own fills.
    /// Text and edge colours are unchanged (still dark, tuned for a light
    /// background), so on a dark page they can be hard to read; theming
    /// is tracked in issue #57.
    pub transparent: bool,
}

/// Opening `<svg>` tag + white background (unless
/// [`SvgOptions::transparent`]), shared by every SVG writer. `title`
/// makes the diagram screen-reader friendly (issue #16): it becomes the
/// root `<title>`, the `aria-label`, and the element gets `role="img"`.
pub(crate) fn svg_open(
    s: &mut String,
    width: f64,
    height: f64,
    font_size: u32,
    title: &str,
    opts: &SvgOptions,
) {
    svg_open_a11y(s, width, height, font_size, title, None, opts);
}

/// Prefix of the `<desc>` element id an accessible description gets;
/// the root `aria-describedby` points at it.
const A11Y_DESC_ID_PREFIX: &str = "flowmaid-desc-";

/// Mermaid's `accTitle` / `accDescr` for one diagram, as the SVG
/// writers take it. An absent or empty title keeps the writer's
/// default accessible name ("Flowchart diagram", …); a description
/// adds a `<desc>` right after the root `<title>` and an
/// `aria-describedby` pointing at it. The default (both `None`)
/// reproduces the historical output byte for byte.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct A11y {
    /// Replaces the default accessible name (`<title>`, `aria-label`).
    pub title: Option<String>,
    /// Written as the root `<desc>`.
    pub descr: Option<String>,
}

impl A11y {
    pub fn new(title: Option<String>, descr: Option<String>) -> Self {
        A11y { title, descr }
    }

    /// The accessible name to write (`<b>`/`<i>` interpreted away) and
    /// the description, if any.
    pub(crate) fn resolve(&self, default_title: &str) -> (String, Option<&str>) {
        let title = self
            .title
            .as_deref()
            .map(plain_text)
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| default_title.to_string());
        let descr = self.descr.as_deref().filter(|d| !d.trim().is_empty());
        (title, descr)
    }
}

/// [`svg_open`] plus an optional accessible description: a `<desc>`
/// right after the root `<title>` (mermaid's title, desc, … order) and
/// an `aria-describedby` on the root. The `<desc>` id hashes the title
/// and description (FNV-1a, see [`DefsKey`]) so two inlined SVGs with
/// different descriptions never share an id. `desc: None` is exactly
/// [`svg_open`].
pub(crate) fn svg_open_a11y(
    s: &mut String,
    width: f64,
    height: f64,
    font_size: u32,
    title: &str,
    desc: Option<&str>,
    opts: &SvgOptions,
) {
    svg_open_root(s, width, height, font_size, title, desc, None, opts);
}

/// [`svg_open_a11y`] plus an optional root `id` (the per-diagram id
/// scoped CSS and edge ids hang off); `None` writes no `id` attribute.
#[allow(clippy::too_many_arguments)]
fn svg_open_root(
    s: &mut String,
    width: f64,
    height: f64,
    font_size: u32,
    title: &str,
    desc: Option<&str>,
    root_id: Option<&str>,
    opts: &SvgOptions,
) {
    // `width`/`height` become percentages only in responsive mode; the
    // `viewBox` stays intrinsic so the content scales with the container.
    let (w_attr, h_attr) = if opts.responsive {
        ("100%".to_string(), "100%".to_string())
    } else {
        (format!("{width:.0}"), format!("{height:.0}"))
    };
    // `preserveAspectRatio` is written whenever set; when absent the SVG
    // default (`xMidYMid meet`) applies.
    let par = opts
        .preserve_aspect_ratio
        .map(|p| format!(" preserveAspectRatio=\"{p}\""))
        .unwrap_or_default();
    let desc_id = desc.map(|d| {
        let mut key = DefsKey::new();
        key.eat(title.as_bytes());
        key.eat(&[0]);
        key.eat(d.as_bytes());
        format!("{A11Y_DESC_ID_PREFIX}{}", key.finish())
    });
    s.push_str(&format!(
        "<svg{id} xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         viewBox=\"0 0 {vw:.0} {vh:.0}\"{par} font-family=\"Helvetica, Arial, sans-serif\" \
         font-size=\"{fs}\" role=\"img\" aria-label=\"{t}\"{described}>\n",
        id = root_id
            .map(|i| format!(" id=\"{}\"", escape(i)))
            .unwrap_or_default(),
        w = w_attr,
        h = h_attr,
        vw = width,
        vh = height,
        fs = font_size,
        t = escape(title),
        described = desc_id
            .as_ref()
            .map(|id| format!(" aria-describedby=\"{id}\""))
            .unwrap_or_default()
    ));
    s.push_str(&format!("<title>{}</title>\n", escape(title)));
    if let (Some(d), Some(id)) = (desc, &desc_id) {
        s.push_str(&format!("<desc id=\"{id}\">{}</desc>\n", escape(d)));
    }
    // The white canvas is opt-out (issue #62) so output can sit on a
    // dark page; the default keeps it for byte-identical output.
    if !opts.transparent {
        s.push_str(&format!(
            "<rect width=\"{:.0}\" height=\"{:.0}\" fill=\"#ffffff\"/>\n",
            width, height
        ));
    }
}

/// White label box with centred text — used for flowchart edge
/// labels and ER relationship labels alike. `center` is in final
/// (already translated) coordinates.
pub(crate) fn svg_label_box(s: &mut String, text: &str, center: (f64, f64), box_w: f64) {
    svg_label_box_with(s, text, center, box_w, TEXT_COLOR);
}

/// [`svg_label_box`] with the text colour given (an already-escaped
/// attribute value), for `linkStyle … color:`.
pub(crate) fn svg_label_box_with(
    s: &mut String,
    text: &str,
    center: (f64, f64),
    box_w: f64,
    fill: &str,
) {
    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"20\" rx=\"4\" \
         fill=\"#ffffff\" stroke=\"{}\"/>\n",
        center.0 - box_w / 2.0,
        center.1 - 10.0,
        box_w,
        LABEL_BORDER
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
         fill=\"{}\">{}</text>\n",
        center.0,
        center.1,
        fill,
        rich(text)
    ));
}

/// Centred `<text>` supporting `\n` line breaks — one `<tspan>`
/// per line, block vertically centred on `(cx, cy)`. `<b>`/`<i>`
/// runs inside a line render as real bold/italic tspans.
pub(crate) fn svg_text_multiline(s: &mut String, cx: f64, cy: f64, fill: &str, label: &str) {
    let lines: Vec<&str> = label.split('\n').collect();
    if lines.len() <= 1 {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             fill=\"{}\">{}</text>\n",
            cx,
            cy,
            fill,
            rich(label)
        ));
        return;
    }
    let lh = crate::layout::LINE_H;
    let top = cy - (lines.len() as f64 - 1.0) * lh / 2.0;
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"{}\">",
        cx, top, fill
    ));
    for (i, line) in lines.iter().enumerate() {
        s.push_str(&format!(
            "<tspan x=\"{:.1}\" dy=\"{}\">{}</tspan>",
            cx,
            if i == 0 {
                "0.33em".to_string()
            } else {
                format!("{lh:.1}")
            },
            rich(line)
        ));
    }
    s.push_str("</text>\n");
}

/// Label as plain text: lines joined with spaces, `<b>`/`<i>` styling
/// tags interpreted away — for `<title>`/`aria-label` content where
/// markup must not leak (issue #16).
fn plain_text(label: &str) -> String {
    label
        .split('\n')
        .map(|l| {
            crate::layout::spans(l)
                .into_iter()
                .map(|(t, ..)| t)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Escaped SVG runs for one label line: `<b>`/`<i>` spans become
/// nested styled tspans — mermaid renders these tags as real
/// formatting, so flowmaid does too instead of showing them literally.
fn rich(line: &str) -> String {
    let spans = crate::layout::spans(line);
    if spans.len() == 1 && !spans[0].1 && !spans[0].2 {
        return escape(&spans[0].0);
    }
    let mut s = String::new();
    for (t, b, i) in spans {
        if !b && !i {
            s.push_str(&escape(&t));
        } else {
            s.push_str("<tspan");
            if b {
                s.push_str(" font-weight=\"bold\"");
            }
            if i {
                s.push_str(" font-style=\"italic\"");
            }
            s.push('>');
            s.push_str(&escape(&t));
            s.push_str("</tspan>");
        }
    }
    s
}

/// A content hash for namespacing the ids a diagram puts in its
/// `<defs>` — arrow markers, gradients.
///
/// Ids must be unique per diagram: two SVGs inlined on one page would
/// otherwise share `#foo-1`, and every reference in the second would
/// resolve to the first one's definition. A running counter gives
/// uniqueness only WITHIN one process — two files rendered separately
/// both start at 1 — and costs byte-identical output. Hashing the
/// content keeps both: the same diagram always hashes the same, and a
/// different one practically never collides.
///
/// FNV-1a, 64-bit: a few lines, no dependency, and stable across runs
/// and platforms, unlike `DefaultHasher`.
pub(crate) struct DefsKey(u64);

impl DefsKey {
    pub(crate) fn new() -> Self {
        DefsKey(0xcbf2_9ce4_8422_2325)
    }

    pub(crate) fn eat(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    /// Feed a coordinate. Bit patterns, so `-0.0` and `0.0` stay
    /// distinct and no rounding creeps in.
    pub(crate) fn eat_f64(&mut self, v: f64) {
        self.eat(&v.to_bits().to_le_bytes());
    }

    pub(crate) fn finish(&self) -> String {
        format!("{:x}", self.0)
    }
}

/// A user-supplied style value on its way into an SVG attribute.
///
/// Colours arrive here from `style` / `classDef` directives AND from
/// callers who build a [`crate::model::Graph`] by hand through the public
/// scene API, so neither the parser nor the constructor can be trusted to
/// have vetted them. [`escape`] neutralises both quote forms, so a value
/// cannot close the attribute whichever delimiter the render site used.
pub(crate) fn style_attr(v: Option<&str>, fallback: &str) -> String {
    // The fallback is escaped too. It is a palette constant in most
    // callers, where escaping is a no-op — but in `advance` it is the
    // diagram-level style, which is every bit as user-supplied as the
    // override. Escaping both makes the helper safe whoever calls it.
    escape(v.unwrap_or(fallback))
}

/// Escape one string for XML, in an attribute or in text.
///
/// Control characters are DROPPED rather than escaped: XML 1.0 forbids
/// them outright, even written as a numeric reference, so a single NUL
/// arriving from an untrusted diagram would make the whole document
/// unparseable — a browser shows an XML error instead of the drawing.
/// Tab, newline and carriage return are the three that are legal.
pub(crate) fn escape(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\r'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Test helper for renderers that draw `width` × `height` as a
/// `0 0 W H` viewBox (class, ER): panics unless every drawn coordinate
/// — `x`/`y`/`x1`…`cy` attributes, rect far corners, path `d` points
/// and polygon `points` — lies on that canvas.
#[cfg(test)]
pub(crate) fn assert_drawn_inside_canvas(svg: &str) {
    fn attr<'a>(line: &'a str, name: &str) -> Option<&'a str> {
        let pat = format!(" {name}=\"");
        let i = line.find(&pat)? + pat.len();
        Some(&line[i..i + line[i..].find('"')?])
    }
    fn num(v: &str) -> f64 {
        v.parse().unwrap_or_else(|_| panic!("not a number: {v:?}"))
    }
    let root = svg.lines().find(|l| l.starts_with("<svg ")).unwrap();
    let (w, h) = (
        num(attr(root, "width").unwrap()),
        num(attr(root, "height").unwrap()),
    );
    let check = |line: &str, x: f64, y: f64| {
        assert!(
            (0.0..=w).contains(&x) && (0.0..=h).contains(&y),
            "({x}, {y}) is off the {w}×{h} canvas: {line}"
        );
    };
    for line in svg
        .lines()
        .filter(|l| l.starts_with('<') && !l.starts_with("<svg "))
    {
        for (kx, ky) in [("x", "y"), ("x1", "y1"), ("x2", "y2"), ("cx", "cy")] {
            if let (Some(x), Some(y)) = (attr(line, kx), attr(line, ky)) {
                check(line, num(x), num(y));
            }
        }
        if line.starts_with("<rect ") {
            if let (Some(x), Some(y)) = (attr(line, "x"), attr(line, "y")) {
                let far_x = num(x) + num(attr(line, "width").unwrap());
                let far_y = num(y) + num(attr(line, "height").unwrap());
                check(line, far_x, far_y);
            }
        }
        let pts = attr(line, "d").or_else(|| attr(line, "points"));
        if let Some(p) = pts {
            let ns: Vec<f64> = p
                .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter(|t| !t.is_empty())
                .map(num)
                .collect();
            for xy in ns.chunks(2) {
                check(line, xy[0], xy[1]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn svg_src(src: &str) -> String {
        to_svg(&scene(&parse(src).unwrap_or_else(|e| panic!("{e}\n{src}"))))
    }

    fn edge_paths(svg: &str) -> Vec<&str> {
        svg.lines()
            .filter(|l| l.starts_with("<path d=") && l.contains("<title>"))
            .collect()
    }

    /// The root `<svg id="…">`, if any.
    fn root_id(svg: &str) -> Option<String> {
        let open = svg.split('>').next()?;
        let rest = open.strip_prefix("<svg id=\"")?;
        Some(rest.split('"').next()?.to_string())
    }

    /// `svg` with its root id and the edge-id attributes removed.
    fn strip_ids(svg: &str, edge_id: &str) -> String {
        let dom = root_id(svg).expect("root id");
        svg.replacen(&format!(" id=\"{dom}\""), "", 1).replace(
            &format!(" id=\"{dom}-{edge_id}\" data-id=\"{edge_id}\""),
            "",
        )
    }

    #[test]
    fn edge_ids_become_prefixed_dom_ids() {
        let svg = svg_src("flowchart LR\n  A e1@--> B\n");
        let dom = root_id(&svg).expect("an edge id gives the svg an id");
        assert!(dom.starts_with(DIAGRAM_DOM_ID_PREFIX), "{dom}");
        let p = edge_paths(&svg);
        assert!(
            p[0].contains(&format!(" id=\"{dom}-e1\" data-id=\"e1\"")),
            "{svg}"
        );
        assert!(!svg.contains("<style>") && !svg.contains("class=\""));
        // Deterministic: same input, same bytes.
        assert_eq!(svg, svg_src("flowchart LR\n  A e1@--> B\n"));
        // Another diagram gets another prefix, so inlined SVGs never
        // share `#…-e1`.
        let other = svg_src("flowchart LR\n  A e1@--> C\n");
        assert_ne!(root_id(&other).unwrap(), dom);
        // Same geometry as the id-less edge; only the attributes differ.
        let plain = svg_src("flowchart LR\n  A --> B\n");
        assert!(root_id(&plain).is_none());
        assert_eq!(strip_ids(&svg, "e1"), plain);
        // An id named like a marker never collides with it.
        let svg = svg_src("flowchart LR\n  A arrow@--> B\n");
        let dom = root_id(&svg).unwrap();
        assert!(
            svg.contains(&format!("id=\"{dom}-arrow\"")) && svg.contains("<marker id=\"arrow\"")
        );
        assert_eq!(svg.matches(" id=\"arrow\"").count(), 1);
        // XML-escaped.
        let svg = svg_src("flowchart LR\n  A e<1@--> B\n");
        let dom = root_id(&svg).unwrap();
        assert!(
            svg.contains(&format!("id=\"{dom}-e&lt;1\" data-id=\"e&lt;1\"")),
            "{svg}"
        );
        // Invisible edges are not drawn, so neither are their ids.
        let svg = svg_src("flowchart LR\n  A e1@~~~ B\n  e1@{ animate: true }\n");
        assert!(!svg.contains("e1") && !svg.contains("<style>"));
        assert!(root_id(&svg).is_none());
    }

    #[test]
    fn animated_edges_get_mermaids_class_and_one_style_block() {
        let svg = svg_src(
            "flowchart LR\n  A e1@--> B\n  B e2@-.-> C\n  C e3@--> D\n  e1@{ animate: true }\n  e2@{ animation: slow }\n",
        );
        let p = edge_paths(&svg);
        assert!(p[0].contains("class=\"edge-animation-fast\""), "{svg}");
        assert!(p[1].contains("class=\"edge-animation-slow\""));
        assert!(
            p[1].contains("stroke-dasharray=\"5 4\""),
            "dotted dash stays"
        );
        assert!(!p[2].contains("class="));
        assert_eq!(svg.matches("<style>").count(), 1);
        // Scoped to this SVG, keyframes included, so an inlined SVG
        // never restyles the host page or another diagram.
        let dom = root_id(&svg).unwrap();
        assert!(svg.contains(&format!(
            "@keyframes {dom}-dash{{to{{stroke-dashoffset:0}}}}"
        )));
        assert!(svg.contains(&format!("#{dom} .edge-animation-fast{{stroke-dasharray:9,5 !important;stroke-dashoffset:900;animation:{dom}-dash 20s linear infinite;stroke-linecap:round}}")));
        assert!(svg.contains(&format!("#{dom} .edge-animation-slow{{stroke-dasharray:9,5 !important;stroke-dashoffset:900;animation:{dom}-dash 50s linear infinite;stroke-linecap:round}}")));
        let css = svg
            .split("<style>")
            .nth(1)
            .unwrap()
            .split("</style>")
            .next()
            .unwrap();
        let unscoped: Vec<&str> = css
            .split('}')
            .map(str::trim)
            .filter(|r| r.starts_with('.'))
            .collect();
        assert!(unscoped.is_empty(), "{unscoped:?}");
        assert!(!css.contains("@keyframes dash"));
        // Bogus animation: no class, no style block.
        let svg = svg_src("flowchart LR\n  A e1@--> B\n  e1@{ animate: true, animation: bogus }\n");
        assert!(!svg.contains("class=\"edge-animation") && !svg.contains("<style>"));
    }

    #[test]
    fn edge_curves_change_only_their_own_path() {
        let plain = svg_src("flowchart TD\n  A --> B\n");
        let wiggly = svg_src("flowchart TD\n  A e1@--> B\n  e1@{ curve: wiggly }\n");
        let basis = svg_src("flowchart TD\n  A e1@--> B\n  e1@{ curve: basis }\n");
        let strip = |s: &str| strip_ids(s, "e1");
        assert_eq!(strip(&wiggly), plain, "unknown curve = basis");
        assert_eq!(strip(&basis), plain);
        let d = |s: &str| {
            edge_paths(s)[0]
                .split("d=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .to_string()
        };
        let base_d = d(&plain);
        // d3's bumpY between two vertically aligned points IS the
        // default cubic; it differs once the ends are offset.
        let bump = svg_src("flowchart LR\n  A e1@--> B\n  e1@{ curve: bumpY }\n");
        assert!(d(&bump).contains(" C "));
        for c in [
            "linear",
            "rounded",
            "step",
            "stepBefore",
            "stepAfter",
            "bumpX",
            "natural",
            "catmullRom",
            "cardinal",
            "monotoneX",
            "monotoneY",
        ] {
            let svg = svg_src(&format!(
                "flowchart TD\n  A e1@--> B\n  e1@{{ curve: {c} }}\n"
            ));
            assert_ne!(d(&svg), base_d, "{c}");
            assert!(
                edge_paths(&svg)[0].contains("marker-end=\"url(#arrow)\""),
                "{c}"
            );
        }
        // A routed (multi-waypoint) edge: each curve draws its own kind
        // of commands through the waypoints.
        let src = |c: &str| {
            format!("flowchart TD\n  A --> B --> C --> D\n  A e1@--> D\n  e1@{{ curve: {c} }}\n")
        };
        let g = parse(&src("linear")).unwrap();
        let sc = scene(&g);
        assert!(sc.edges[3].waypoints.len() >= 3, "the long edge is routed");
        let lin = edge_paths(&svg_src(&src("linear")))[3].to_string();
        assert!(lin.contains(" L ") && !lin.contains(" C "), "{lin}");
        let rnd = edge_paths(&svg_src(&src("rounded")))[3].to_string();
        assert!(rnd.contains(" Q "), "{rnd}");
        let cr = edge_paths(&svg_src(&src("catmullRom")))[3].to_string();
        assert!(cr.contains(" C "), "{cr}");
    }

    /// A bare edge for the path-builder tests: router-style degenerate
    /// `bezier` chord `[p0, p0, p3, p3]` plus the given waypoints.
    fn routed_edge(p0: (f64, f64), p3: (f64, f64), waypoints: Vec<(f64, f64)>) -> SceneEdge {
        SceneEdge {
            from: "a".into(),
            to: "b".into(),
            bezier: [p0, p0, p3, p3],
            waypoints,
            kind: EdgeKind::Arrow,
            ends: EdgeEnds::default(),
            meta: EdgeMeta::default(),
            label: None,
        }
    }

    /// Report P0 #6: class / ER painters stroked `bezier` alone, so a
    /// rerouted edge (route in `waypoints`, chord in `bezier`) was
    /// drawn straight through the boxes. The shared builder draws the
    /// route.
    #[test]
    fn edge_path_d_draws_the_route_not_the_chord() {
        let wps = vec![(0.0, 0.0), (0.0, 50.0), (80.0, 50.0)];
        let e = routed_edge((0.0, 0.0), (80.0, 50.0), wps.clone());
        assert_eq!(edge_path_d(&e, |p| p), spline_d(&wps));
        // `t` maps every point (the flowchart painter's MARGIN shift).
        let moved: Vec<(f64, f64)> = wps.iter().map(|&(x, y)| (x + 10.0, y + 5.0)).collect();
        assert_eq!(
            edge_path_d(&e, |(x, y)| (x + 10.0, y + 5.0)),
            spline_d(&moved)
        );
        // No waypoints: the single cubic, exactly as before.
        let plain = routed_edge((0.0, 0.0), (80.0, 50.0), Vec::new());
        assert_eq!(
            edge_path_d(&plain, |p| p),
            "M 0.0 0.0 C 0.0 0.0, 80.0 50.0, 80.0 50.0"
        );
        // A non-default curve goes through `curve_geometry`.
        let mut step = routed_edge((0.0, 0.0), (80.0, 50.0), wps);
        step.meta.curve = Some(EdgeCurve::StepBefore);
        let (start, segs) = curve_geometry(&step).unwrap();
        assert_eq!(edge_path_d(&step, |p| p), curve_d(start, &segs, |p| p));
    }

    /// The flowchart painter draws every edge with `edge_path_d` — the
    /// one builder the class / ER painters share — through its MARGIN
    /// shift, for routed, plain and curved edges alike.
    #[test]
    fn flowchart_edges_are_painted_by_edge_path_d() {
        let src = "flowchart TD\n  A --> B --> C --> D\n  A --> D\n  B e1@--> D\n  \
                   e1@{ curve: rounded }\n";
        let sc = scene(&parse(src).unwrap());
        assert!(
            sc.edges.iter().any(|e| e.waypoints.len() >= 3),
            "a routed edge"
        );
        let svg = to_svg(&sc);
        let mut bb = Bbox::new();
        grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, _, miny, _) = bb.finish();
        let (tx, ty) = (MARGIN - minx, MARGIN - miny);
        let paths = edge_paths(&svg);
        assert_eq!(paths.len(), sc.edges.len());
        for (e, p) in sc.edges.iter().zip(paths) {
            let d = edge_path_d(e, |(x, y)| (x + tx, y + ty));
            assert!(p.starts_with(&format!("<path d=\"{d}\"")), "{p}\n{d}");
        }
    }

    #[test]
    fn edge_end_chord_follows_the_drawn_final_stretch() {
        let near =
            |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6;
        // Routed L: down, then right. The end chord lies on the final
        // horizontal stretch (the spline ends with a straight run from
        // (66.7, 50)), the start chord on the first vertical one.
        let e = routed_edge(
            (0.0, 0.0),
            (80.0, 50.0),
            vec![(0.0, 0.0), (0.0, 50.0), (80.0, 50.0)],
        );
        let (tip, toward) = edge_end_chord(&e, false, 10.0);
        assert!(near(tip, (80.0, 50.0)));
        assert!(near(toward, (70.0, 50.0)), "{toward:?}");
        let (tip, toward) = edge_end_chord(&e, true, 5.0);
        assert!(near(tip, (0.0, 0.0)));
        assert!(near(toward, (0.0, 5.0)), "{toward:?}");
        // The degenerate chord `[p0, p0, p3, p3]` without waypoints:
        // "toward the adjacent control point" had zero length (the
        // zero-size inheritance triangles); the chord still points.
        let plain = routed_edge((0.0, 0.0), (100.0, 0.0), Vec::new());
        let (tip, toward) = edge_end_chord(&plain, false, 14.0);
        assert!(near(tip, (100.0, 0.0)));
        assert!(
            (toward.0 - 86.0).abs() < 1e-6 && toward.1.abs() < 1e-9,
            "{toward:?}"
        );
        // Longer reach than the curve: the far end.
        let (_, toward) = edge_end_chord(&plain, false, 1e6);
        assert!(near(toward, (0.0, 0.0)));
        // Only a zero-length curve has no direction.
        let dot = routed_edge((5.0, 5.0), (5.0, 5.0), Vec::new());
        let (tip, toward) = edge_end_chord(&dot, true, 14.0);
        assert!(near(tip, toward));
    }

    #[test]
    fn curved_edges_pick_and_bound_what_is_drawn() {
        let g = parse("flowchart TD\n  A e1@--> B\n  e1@{ curve: stepBefore }\n").unwrap();
        let sc = scene(&g);
        let e = &sc.edges[0];
        let (start, segs) = curve_geometry(e).expect("non-default curve");
        let pts = curve_sample(start, &segs);
        assert_eq!(edge_polyline(e), pts);
        let mut bb = Bbox::new();
        grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, maxx, miny, maxy) = bb.finish();
        for (x, y) in pts {
            assert!(x >= minx && x <= maxx && y >= miny && y <= maxy);
        }
        // Default curve: no special geometry.
        let g = parse("flowchart TD\n  A e1@--> B\n").unwrap();
        assert!(curve_geometry(&scene(&g).edges[0]).is_none());
    }

    #[test]
    fn sub_edges_carry_ids_and_animation() {
        let svg = svg_src(
            "flowchart LR\n  subgraph S\n    X\n  end\n  A e1@--> S\n  e1@{ animation: fast }\n",
        );
        let dom = root_id(&svg).unwrap();
        assert!(svg.contains(&format!("id=\"{dom}-e1\"")), "{svg}");
        assert!(svg.contains("class=\"edge-animation-fast\""));
    }

    #[test]
    fn node_metadata_draws_like_the_bracket_shape() {
        assert_eq!(
            svg_src("flowchart TD\n  A@{ shape: cyl, label: \"DB\" } --> B\n"),
            svg_src("flowchart TD\n  A[(DB)] --> B\n")
        );
        assert_eq!(
            svg_src("flowchart TD\n  A[x]@{ shape: hex }\n"),
            svg_src("flowchart TD\n  A{{x}}\n")
        );
        let svg = svg_src("flowchart TD\n  A@{ shape: text, label: hi }\n");
        assert!(svg.contains("fill=\"none\" stroke=\"none\""), "{svg}");
    }

    #[test]
    fn a11y_desc_ids_are_content_hashed_and_default_is_unchanged() {
        let g = parse("flowchart TD\nA --> B").unwrap();
        let sc = scene(&g);
        assert_eq!(
            to_svg_a11y_with(
                &sc,
                "Flowchart diagram",
                &A11y::default(),
                &SvgOptions::default()
            ),
            to_svg(&sc)
        );
        let a = A11y::new(Some("T".into()), Some("one".into()));
        let b = A11y::new(Some("T".into()), Some("two".into()));
        let opts = SvgOptions::default();
        let sa = to_svg_a11y_with(&sc, "x", &a, &opts);
        let sb = to_svg_a11y_with(&sc, "x", &b, &opts);
        let id = |s: &str| {
            s.split("<desc id=\"")
                .nth(1)
                .and_then(|r| r.split('"').next())
                .unwrap()
                .to_string()
        };
        assert_ne!(id(&sa), id(&sb), "two inlined SVGs never share a desc id");
        assert_eq!(sa, to_svg_a11y_with(&sc, "x", &a, &opts), "deterministic");
        assert!(sa.contains(&format!("aria-describedby=\"{}\"", id(&sa))));
        assert!(id(&sa).starts_with(A11Y_DESC_ID_PREFIX));
        // Title only: no desc, no aria-describedby.
        let t = to_svg_a11y_with(&sc, "x", &A11y::new(Some("<b>T</b>".into()), None), &opts);
        assert!(t.contains("<title>T</title>") && !t.contains("<desc"));
        assert!(!t.contains("aria-describedby"));
        // Blank title falls back to the default name.
        let t = to_svg_a11y_with(&sc, "Dflt", &A11y::new(Some("  ".into()), None), &opts);
        assert!(t.contains("<title>Dflt</title>"));
    }

    #[test]
    fn svg_options_default_matches_fixed_pixel_and_responsive_overrides() {
        let g = parse("flowchart TD\nA --> B").unwrap();
        let sc = scene(&g);

        // Default reproduces the historical fixed-pixel root.
        let base = to_svg(&sc);
        assert!(base.starts_with("<svg"), "default still emits svg");
        assert!(
            !base.contains("width=\"100%\""),
            "default must NOT be responsive"
        );

        // Responsive switches width/height to percentages but keeps viewBox.
        let resp = to_svg_with(
            &sc,
            &SvgOptions {
                responsive: true,
                ..Default::default()
            },
        );
        assert!(
            resp.contains("width=\"100%\"") && resp.contains("height=\"100%\""),
            "{resp}"
        );
        assert!(resp.contains("viewBox=\"0 0"), "viewBox preserved");

        // preserveAspectRatio is written only when set.
        let par = to_svg_with(
            &sc,
            &SvgOptions {
                responsive: true,
                preserve_aspect_ratio: Some("none"),
                ..Default::default()
            },
        );
        assert!(par.contains("preserveAspectRatio=\"none\""), "{par}");
        assert!(
            !resp.contains("preserveAspectRatio"),
            "unset -> default meet, no attribute"
        );
    }

    #[test]
    fn svg_options_transparent_drops_only_the_canvas_rect() {
        // The canvas rect is the first element after <title>, sized to the
        // whole viewBox — match it exactly so shape/label fills don't count.
        fn canvas_rect(svg: &str) -> String {
            let vb = svg.split("viewBox=\"0 0 ").nth(1).unwrap();
            let mut it = vb.split('"').next().unwrap().split(' ');
            let (w, h) = (it.next().unwrap(), it.next().unwrap());
            format!("<rect width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>\n")
        }
        let transparent = SvgOptions {
            transparent: true,
            ..Default::default()
        };
        // Flowchart plus every other writer that shares `svg_open`.
        for src in [
            "flowchart TD\nA --> B",
            "stateDiagram-v2\n[*] --> S",
            "erDiagram\nA ||--o{ B : has",
            "sequenceDiagram\nA->>B: hi",
            "pie\n\"a\" : 1\n\"b\" : 2",
            "timeline\nsection S\n2020 : x",
            "classDiagram\nAnimal <|-- Dog",
            "mindmap\n  root((R))\n    A\n    B",
            "journey\ntitle T\nsection S\n  Task: 5: Me",
            "gitGraph\n  commit\n  branch f\n  commit",
            "architecture-beta\n  service a(server)[A]\n  service b(database)[B]\n  a:R -- L:b",
            "sankey-beta\nA,B,10\nB,C,5",
        ] {
            let base = crate::render_svg(src).unwrap();
            let rect = canvas_rect(&base);
            // Default keeps the white canvas, right after <title>.
            assert!(base.contains(&format!("</title>\n{rect}")), "{src}: {base}");
            assert_eq!(
                base,
                crate::render_svg_advanced(src, &SvgOptions::default()).unwrap(),
                "{src}: default options must match render_svg byte-for-byte"
            );
            let t = crate::render_svg_advanced(src, &transparent).unwrap();
            assert!(!t.contains(&rect), "{src}: canvas rect must be gone: {t}");
            // Nothing else changes: removing the rect gives the default back.
            assert_eq!(base.replacen(&rect, "", 1), t, "{src}");
        }
    }

    #[test]
    fn hit_test_picks_nodes_edges_clusters_with_z_order() {
        // A diamond B in the middle, a subgraph around C, an edge A→B.
        let g = parse(
            "flowchart TD\nA[Start] --> B{Decide}\nsubgraph grp [Group]\n  C[Leaf]\nend\nB --> C",
        )
        .unwrap();
        let s = scene(&g);
        let node = |id: &str| s.nodes.iter().position(|n| n.id == id).unwrap();
        // A point at a node centre picks that node.
        let a = &s.nodes[node("A")];
        assert_eq!(s.hit_test(a.x, a.y, 4.0), Some(Hit::Node(node("A"))));
        assert_eq!(s.node_at(a.x, a.y), Some(node("A")));
        // A miss returns nothing.
        assert_eq!(s.hit_test(a.x, a.y - a.h, 0.5), None);
        // The A→B edge midpoint picks an edge (not a node/cluster).
        let (ai, bi) = (node("A"), node("B"));
        let e = s
            .edges
            .iter()
            .position(|e| e.from == "A" && e.to == "B")
            .unwrap();
        let mid = cubic_mid(
            s.edges[e].bezier[0],
            s.edges[e].bezier[1],
            s.edges[e].bezier[2],
            s.edges[e].bezier[3],
        );
        assert_eq!(s.edge_at(mid.0, mid.1, 6.0), Some(e));
        assert!(matches!(s.hit_test(mid.0, mid.1, 6.0), Some(Hit::Edge(_))));
        let _ = (ai, bi);
        // Inside the Group box but not on a node → the cluster.
        let c = &s.nodes[node("C")];
        let cl = s.clusters.iter().position(|c| c.title == "Group").unwrap();
        let corner = (s.clusters[cl].x + 3.0, s.clusters[cl].y + 3.0);
        assert_eq!(s.cluster_at(corner.0, corner.1), Some(cl));
        assert_eq!(s.hit_test(corner.0, corner.1, 2.0), Some(Hit::Cluster(cl)));
        // But a point on node C inside the cluster picks the NODE (z-order).
        assert_eq!(s.hit_test(c.x, c.y, 2.0), Some(Hit::Node(node("C"))));
    }

    #[test]
    fn node_hit_is_shape_precise_for_diamonds_and_circles() {
        let g = parse("flowchart TD\nX{Diamond} --> Y((Circle))").unwrap();
        let s = scene(&g);
        for (id, inside_corner) in [("X", false), ("Y", false)] {
            let n = &s.nodes[s.nodes.iter().position(|n| n.id == id).unwrap()];
            // Dead centre: inside.
            assert!(super::node_contains(n, n.x, n.y));
            // The bounding-box corner is OUTSIDE a diamond / circle.
            let corner = super::node_contains(n, n.x + n.w / 2.0 - 0.5, n.y + n.h / 2.0 - 0.5);
            assert_eq!(
                corner, inside_corner,
                "{id} corner should be outside its shape"
            );
        }
    }

    #[test]
    fn hit_geometry_matches_the_drawn_shape_after_review_fixes() {
        // (review #17) A non-square circle is drawn as a disc of radius
        // w/2; a point past h/2 but within w/2 must still hit it.
        let g = parse("flowchart TD\nA((C)) --> B").unwrap();
        let sizes = vec![(60.0, 40.0), crate::layout::intrinsic_size(&g.nodes[1])];
        let s = scene_sized(&g, &sizes);
        let a = &s.nodes[0];
        assert!(
            super::node_contains(a, a.x, a.y + 25.0),
            "disc cap pickable"
        );
        assert!(
            !super::node_contains(a, a.x + 29.0, a.y + 29.0),
            "corner still outside"
        );
        // (review #17) StateStart/StateEnd draw as circles → corner miss.
        let crate::model::Document::State(st) =
            crate::parser::parse_document("stateDiagram-v2\n[*] --> Idle\nIdle --> [*]").unwrap()
        else {
            panic!("state diagram");
        };
        let ss = scene(&st);
        let end = ss
            .nodes
            .iter()
            .find(|n| matches!(n.shape, Shape::StateEnd | Shape::StateStart))
            .unwrap();
        let (hw, hh) = (end.w / 2.0, end.h / 2.0);
        assert!(
            !super::node_contains(end, end.x + hw - 0.3, end.y + hh - 0.3),
            "pseudostate corner is outside its drawn circle"
        );
        // (review #17) A routed edge picks along the DRAWN curveBasis
        // spline: a point on the sampled curve is within a small tol.
        let g2 = parse("flowchart TD\nA-->B\nA-->C\nB-->C\nA-->D\nD-->C").unwrap();
        let s2 = scene(&g2);
        let routed = s2.edges.iter().position(|e| e.waypoints.len() >= 2);
        if let Some(ri) = routed {
            let poly = super::edge_polyline(&s2.edges[ri]);
            let mid = poly[poly.len() / 2];
            assert_eq!(
                s2.edge_at(mid.0, mid.1, 1.0),
                Some(ri),
                "curve point picks its edge"
            );
        }
    }

    #[test]
    fn marquee_catches_an_edge_that_crosses_without_a_vertex_inside() {
        // (review #17) A tall marquee sliver over the middle of a long
        // A→B edge must select it even though no sampled vertex lands
        // in the sliver.
        let g = parse("flowchart TD\nA[Start] --> B[End]").unwrap();
        let s = scene(&g);
        let a = &s.nodes[0];
        let b = &s.nodes[1];
        let midy = (a.y + b.y) / 2.0;
        let hits = s.hits_in_rect(a.x - 0.5, midy - 0.5, a.x + 0.5, midy + 0.5);
        assert!(
            hits.iter().any(|h| matches!(h, Hit::Edge(_))),
            "segment-crossing edge selected by a thin marquee"
        );
    }

    #[test]
    fn marquee_and_nearest_and_invisible_edges() {
        let g = parse("flowchart LR\nA --> B\nB ~~~ C\nC --> D").unwrap();
        let s = scene(&g);
        // A marquee over the whole canvas grabs every node + visible edge,
        // but NOT the invisible B~~~C link.
        let all = s.hits_in_rect(-1e6, -1e6, 1e6, 1e6);
        let node_hits = all.iter().filter(|h| matches!(h, Hit::Node(_))).count();
        let edge_hits = all.iter().filter(|h| matches!(h, Hit::Edge(_))).count();
        assert_eq!(node_hits, 4);
        assert_eq!(edge_hits, 2, "invisible link is not selectable");
        // nearest_node: a point on top of A returns A at distance 0.
        let a = &s.nodes[s.nodes.iter().position(|n| n.id == "A").unwrap()];
        assert_eq!(s.nearest_node(a.x, a.y), Some((0, 0.0)));
        // A far-away point still finds the closest node with a positive gap.
        let (idx, d) = s.nearest_node(-500.0, a.y).unwrap();
        assert!(d > 0.0 && idx < s.nodes.len());
        // Invisible links are never picked directly either.
        let inv = s
            .edges
            .iter()
            .position(|e| matches!(e.kind, EdgeKind::Invisible));
        if let Some(iv) = inv {
            let m = s.edges[iv].bezier[1];
            assert_ne!(s.edge_at(m.0, m.1, 100.0), Some(iv));
        }
    }

    #[test]
    fn scene_and_render_are_consistent() {
        let g = parse("flowchart LR\nA --> B{X}\nB -->|y| C").unwrap();
        let s = scene(&g);
        assert_eq!(s.nodes.len(), 3);
        assert_eq!(s.edges.len(), 2);
        assert!(s.width > 0.0 && s.height > 0.0);
        assert_eq!(to_svg(&s), crate::render::render(&g));
    }

    /// The single drawn edge `<path …>` of a one-edge flowchart.
    fn only_edge_path(svg: &str) -> &str {
        let i = svg.find("<path d=\"M").expect("edge path");
        &svg[i..i + svg[i..].find("</path>").unwrap()]
    }

    #[test]
    fn edge_markers_draw_at_the_right_ends() {
        let cases: &[(&str, &str, &str)] = &[
            ("--x", "", "cross-end"),
            ("-.-x", "", "cross-end"),
            ("==x", "", "cross-end"),
            ("--o", "", "circle-end"),
            ("-.-o", "", "circle-end"),
            ("==o", "", "circle-end"),
            ("-- t --x", "", "cross-end"),
            ("-. t .-o", "", "circle-end"),
            ("== t ==x", "", "cross-end"),
            ("--x|t|", "", "cross-end"),
            ("<-->", "arrow-start", "arrow"),
            ("<-.->", "arrow-start", "arrow"),
            ("<==>", "arrow-start", "arrow"),
            ("<-- t -->", "arrow-start", "arrow"),
            ("<-->|t|", "arrow-start", "arrow"),
            ("x--x", "cross-start", "cross-end"),
            ("o--o", "circle-start", "circle-end"),
        ];
        for (op, start, end) in cases {
            let svg = crate::render_svg(&format!("flowchart LR\nA {op} B\n")).unwrap();
            let path = only_edge_path(&svg);
            assert!(
                path.contains(&format!("marker-end=\"url(#{end})\"")),
                "{op}: {path}"
            );
            if start.is_empty() {
                assert!(!path.contains("marker-start"), "{op}: {path}");
            } else {
                assert!(
                    path.contains(&format!("marker-start=\"url(#{start})\"")),
                    "{op}: {path}"
                );
            }
            // Every referenced glyph is defined, and only those.
            for id in [*start, *end].into_iter().filter(|i| !i.is_empty()) {
                assert!(
                    svg.contains(&format!("<marker id=\"{id}\"")),
                    "{op}: def {id}"
                );
            }
            let defs = svg.matches("<marker id=").count();
            let want = 1 + [*start, *end]
                .iter()
                .filter(|i| !i.is_empty() && **i != "arrow")
                .count();
            assert_eq!(defs, want, "{op}: marker defs");
        }
    }

    #[test]
    fn subgraph_edges_keep_their_glyphs_in_scene_and_route() {
        let g = parse("flowchart LR\nsubgraph S\nH\nend\nS <--> A\nS --x B\n").unwrap();
        let s = scene(&g);
        let ends = |sc: &Scene, to: &str| {
            sc.edges
                .iter()
                .find(|e| e.from == "S" && e.to == to)
                .unwrap_or_else(|| panic!("no S -> {to}"))
                .ends
        };
        for sc in [
            &s,
            &route(&g, &s.nodes.iter().map(|n| (n.x, n.y)).collect::<Vec<_>>()),
        ] {
            assert_eq!(ends(sc, "A").start, Some(Marker::Arrow));
            assert_eq!(ends(sc, "B").end, Some(Marker::Cross));
            let svg = to_svg(sc);
            assert!(svg.contains("marker-start=\"url(#arrow-start)\""));
            assert!(svg.contains("marker-end=\"url(#cross-end)\""));
        }
    }

    #[test]
    fn glyph_ends_sharing_a_side_are_spread_apart() {
        let starts = |src: &str| {
            let s = scene(&parse(src).unwrap());
            let mut v: Vec<f64> = s
                .edges
                .iter()
                .filter(|e| e.from == "A")
                .map(|e| e.bezier[0].1)
                .collect();
            v.sort_by(f64::total_cmp);
            v
        };
        let gaps = |v: &[f64]| v.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>();
        let glyphs = starts("flowchart LR\nA <--> B\nA x--x C\nA o--o D\n");
        assert!(
            gaps(&glyphs).iter().all(|&d| d >= GLYPH_PORT_GAP - 1e-9),
            "{glyphs:?}"
        );
        // Plain fan-outs get distinct ports too (G9), as far apart as
        // the side allows.
        let plain = starts("flowchart LR\nA --> B\nA --> C\nA --> D\n");
        assert!(
            gaps(&plain)
                .iter()
                .all(|&d| d >= crate::ports::PORT_SPACING - 1e-9),
            "{plain:?}"
        );
        // Ends stay on the node's border.
        let s = scene(&parse("flowchart LR\nA <--> B\nA x--x C\nA o--o D\n").unwrap());
        let a = &s.nodes[0];
        for e in &s.edges {
            let p = e.bezier[0];
            assert!((p.1 - a.y).abs() <= a.h / 2.0, "{p:?}");
            assert!((p.0 - (a.x + a.w / 2.0)).abs() < 1e-9, "{p:?}");
        }
    }

    #[test]
    fn glyph_fan_in_never_panics_when_ports_fill_the_side() {
        // Enough glyph ends to fill a hub's side make the spread run
        // equal its width, where rounding once inverted the clamp
        // bounds (`N1..N5 --o H[abcd]` panicked).
        const MAX_FAN_IN: usize = 40;
        for dir in ["TD", "BT", "LR", "RL"] {
            for hub in ["a", "abcd", "a much wider hub label"] {
                for op in ["--o", "--x", "<-->"] {
                    for n in 1..=MAX_FAN_IN {
                        let mut src = format!("flowchart {dir}\nH[{hub}]\n");
                        for i in 1..=n {
                            src.push_str(&format!("N{i} {op} H\n"));
                        }
                        let s = scene(&parse(&src).unwrap());
                        assert_eq!(s.edges.len(), n, "{src}");
                    }
                }
            }
        }
    }

    #[test]
    fn marker_free_diagrams_keep_their_exact_defs_and_titles() {
        let svg = crate::render_svg("flowchart LR\nA --> B\nB --- C\nC -.-> D\n").unwrap();
        assert_eq!(svg.matches("<marker id=").count(), 1);
        assert!(svg.contains("</marker></defs>"));
        assert!(!svg.contains("marker-start"));
        assert!(svg.contains("<title>A \u{2192} B</title>"));
        // A two-way edge announces itself as such.
        let svg = crate::render_svg("flowchart LR\nA <--> B\n").unwrap();
        assert!(svg.contains("<title>A \u{2194} B</title>"), "{svg}");
    }

    #[test]
    fn two_way_edges_start_on_the_source_border_like_one_way_edges() {
        // The start glyph sits on the clipped endpoint, so a `<-->`
        // edge must have exactly the geometry of `-->` — clipped to
        // BOTH node borders, never to the centres.
        for dir in ["LR", "TD"] {
            let one = scene(&parse(&format!("flowchart {dir}\nA --> B\n")).unwrap());
            let two = scene(&parse(&format!("flowchart {dir}\nA <--> B\n")).unwrap());
            assert_eq!(one.edges[0].bezier, two.edges[0].bezier, "{dir}");
            let (a, p0) = (&two.nodes[0], two.edges[0].bezier[0]);
            let on_border =
                (p0.0 - a.x).abs() >= a.w / 2.0 - 0.5 || (p0.1 - a.y).abs() >= a.h / 2.0 - 0.5;
            assert!(on_border, "{dir}: start {p0:?} not on A's border");
            assert_eq!(two.edges[0].ends.start, Some(Marker::Arrow));
        }
        // route() (drag path) keeps the glyphs too.
        let g = parse("flowchart LR\nA x--x B\n").unwrap();
        let s = scene(&g);
        let centers: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x + 40.0, n.y)).collect();
        let r = route(&g, &centers);
        assert_eq!(r.edges[0].ends, g.edges[0].ends);
        assert!(to_svg(&r).contains("marker-start=\"url(#cross-start)\""));
    }

    #[test]
    fn cross_cluster_layout_separates_boxes_and_stays_contained() {
        // Two stacked subgraphs with an edge between their members: the
        // global cluster layout ranks them into separate layers, so the
        // boxes never overlap and the edge routes cleanly in the gap.
        let src = "flowchart TD\n\
                   subgraph Top\n  A\nend\n\
                   subgraph Bot\n  B\nend\n\
                   A --> B";
        let g = parse(src).unwrap();
        let s = scene(&g);
        assert_eq!(s.clusters.len(), 2);
        let top = s.clusters.iter().find(|c| c.title == "Top").unwrap();
        let bot = s.clusters.iter().find(|c| c.title == "Bot").unwrap();
        // Top box sits entirely above the Bot box (clean vertical split).
        assert!(top.y + top.h <= bot.y + 0.5, "Top must sit above Bot");
        // The drawn edge stays inside the canvas.
        for p in &s.edges[0].bezier {
            assert!(p.0 >= -0.5 && p.0 <= s.width + 0.5);
            assert!(p.1 >= -0.5 && p.1 <= s.height + 0.5);
        }
        // route() over the auto positions keeps the same counts.
        let centers: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&g, &centers);
        assert_eq!(s.edges.len(), r.edges.len());
        assert_eq!(s.clusters.len(), r.clusters.len());
        // A same-cluster adjacent edge stays a plain curve (no waypoints).
        let g2 = parse("flowchart TD\nsubgraph S\n  A\n  B\nend\nA --> B").unwrap();
        assert!(scene(&g2).edges[0].waypoints.is_empty());
    }

    #[test]
    fn svg_is_accessible_and_deterministic() {
        // Issue #16: role/aria/title on the root, <title> per node and
        // per edge (plain text, styling tags stripped) …
        let svg = crate::render_svg("flowchart TD\nA[\"<b>Start</b> here\"] -->|go| B").unwrap();
        assert!(svg.contains("role=\"img\""));
        assert!(svg.contains("aria-label=\"Flowchart diagram\""));
        assert!(svg.contains("<title>Flowchart diagram</title>"));
        assert!(
            svg.contains("<title>Start here</title>"),
            "node title, tags stripped"
        );
        assert!(
            svg.contains("<title>A \u{2192} B: go</title>"),
            "edge title with label"
        );
        // State diagrams announce themselves correctly (issue #16 fix):
        // right accessible name, no synthesized `__start_*`/`__end_*`
        // ids in tooltips, no empty <title> for pseudostate nodes.
        let st = crate::render_svg("stateDiagram-v2\n[*] --> Idle\nIdle --> [*]").unwrap();
        assert!(
            st.contains("aria-label=\"State diagram\""),
            "state aria-label"
        );
        assert!(
            st.contains("<title>State diagram</title>"),
            "state root title"
        );
        assert!(!st.contains("__start_"), "no synthesized start id leaked");
        assert!(!st.contains("__end_"), "no synthesized end id leaked");
        assert!(
            st.contains("start \u{2192} Idle"),
            "pseudostate named 'start'"
        );
        assert!(!st.contains("<title></title>"), "no empty node titles");
        // … and byte-identical output for repeated renders of every
        // bundled example (same-process guard; cross-process identity
        // is guaranteed by ordered collections in the layout path).
        for src in [
            include_str!("../examples/demo.mmd"),
            include_str!("../examples/advanced.mmd"),
            include_str!("../examples/subgraph.mmd"),
            include_str!("../examples/er.mmd"),
            include_str!("../examples/class.mmd"),
            include_str!("../examples/sequence.mmd"),
            include_str!("../examples/state.mmd"),
            include_str!("../examples/pie.mmd"),
        ] {
            assert_eq!(
                crate::render_svg(src).unwrap(),
                crate::render_svg(src).unwrap()
            );
        }
    }

    #[test]
    fn scene_carries_stable_identity_and_stays_index_parallel() {
        // Issue #13 contract: nodes are index-parallel with the graph
        // AND carry ids; edges carry from/to ids; sub-edges use the
        // subgraph id; clusters carry their subgraph id.
        let g = parse("flowchart TD\nsubgraph grp [Group]\n  B\nend\nA --> B\nA --> grp").unwrap();
        let s = scene(&g);
        for (sn, n) in s.nodes.iter().zip(&g.nodes) {
            assert_eq!(sn.id, n.id, "scene.nodes index-parallel with graph.nodes");
        }
        assert_eq!(s.edges[0].from, "A");
        assert_eq!(s.edges[0].to, "B");
        let sub = s.edges.last().unwrap();
        assert_eq!((sub.from.as_str(), sub.to.as_str()), ("A", "grp"));
        assert_eq!(s.clusters[0].id, "grp");
        // route() preserves the same identity.
        let centers: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&g, &centers);
        assert_eq!(r.nodes[0].id, s.nodes[0].id);
        assert_eq!(r.edges[0].from, "A");
        // ER / class scenes use the real entity / class names as ids.
        let crate::model::Document::Er(er) =
            crate::parser::parse_document("erDiagram\nusers ||--o{ posts : has").unwrap()
        else {
            panic!("er document");
        };
        let es = crate::er::scene(&er);
        assert_eq!(es.scene.nodes[0].id, "users");
        assert_eq!(es.scene.edges[0].to, "posts");
    }

    #[test]
    fn route_partial_keeps_unmoved_edges_and_reroutes_moved() {
        // D, E, F keep most edges away from the drag, so the frame is
        // not handed to route() (see PARTIAL_ROUTE_SHARE).
        let src = "flowchart TD\nsubgraph S1\n  A\nend\nsubgraph S2\n  B\n  C\nend\nA-->B\nA-->C\nB-->C\n\
                   D-->E\nE-->F\nD-->F";
        let g = parse(src).unwrap();
        let s0 = scene(&g);
        let auto: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
        // Untouched positions -> geometry identical to the base scene.
        let same = route_partial(&g, &auto, &s0, &auto);
        assert_eq!(same.edges.len(), s0.edges.len());
        assert_eq!(same.edges[0].bezier, s0.edges[0].bezier);
        assert_eq!(same.edges[0].waypoints, s0.edges[0].waypoints);
        // Drag C: A->B keeps its engine geometry, A->C follows the drag.
        let ic = g.node_index("C").unwrap();
        let mut dragged = auto.clone();
        dragged[ic].0 += 20.0;
        dragged[ic].1 += 150.0;
        let s1 = route_partial(&g, &dragged, &s0, &auto);
        // A->B keeps its path; only its port on A's side, shared with the
        // redrawn A->C, is spaced again.
        let (b0, b1) = (&s0.edges[0], &s1.edges[0]);
        assert_eq!(b1.bezier[3], b0.bezier[3], "A->B unmoved");
        assert_eq!(b1.waypoints.len(), b0.waypoints.len());
        for (p, q) in b1.waypoints.iter().zip(&b0.waypoints) {
            assert!((p.0 - q.0).abs() < 4.0 && p.1 == q.1, "{p:?} vs {q:?}");
        }
        assert_ne!(s1.edges[1].bezier, s0.edges[1].bezier, "A->C re-routed");
        // Mismatched base (edited graph) falls back to full route().
        let g2 = parse("flowchart TD\nA-->B").unwrap();
        let p2: Vec<(f64, f64)> = vec![(40.0, 40.0), (40.0, 160.0)];
        let fb = route_partial(&g2, &p2, &s0, &auto);
        assert_eq!(fb.nodes.len(), 2);
    }

    /// An edit that keeps every node and edge but renames a subgraph or
    /// moves a node out of one leaves the base's frames stale: the drag
    /// falls back to [`route`] instead of drawing them.
    #[test]
    fn route_partial_falls_back_when_only_subgraphs_changed() {
        let g0 = parse("flowchart TD\nA-->B\nB-->C\nsubgraph S1 [One]\n  B\n  C\nend").unwrap();
        let base = scene(&g0);
        let c0: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut c1 = c0.clone();
        c1[0].0 += 60.0;
        for edit in [
            "flowchart TD\nA-->B\nB-->C\nsubgraph S1 [Two]\n  B\n  C\nend",
            "flowchart TD\nA-->B\nB-->C\nsubgraph S1 [One]\n  C\nend",
        ] {
            let g = parse(edit).unwrap();
            let part = route_partial(&g, &c1, &base, &c0);
            let full = route(&g, &c1);
            assert_eq!(format!("{part:?}"), format!("{full:?}"), "{edit}");
        }
        // The unedited graph keeps the base frames (no fallback).
        let part = route_partial(&g0, &c1, &base, &c0);
        assert_eq!(part.clusters[0].title, "One");
        assert!(part.clusters[0].w >= base.clusters[0].w);
    }

    /// What [`route_partial`] redraws is exactly what [`route`] draws for
    /// the edges on the dragged node, when nothing else changed: the
    /// rest of the drawing is the same `route()` output either way.
    #[test]
    fn route_partial_matches_route_on_the_dragged_node() {
        let src = "flowchart LR\nA[alpha] --> B[beta]\nB -->|go| C[gamma]\nB --> D[delta]\n\
                   C --> E[eps]\nX[x one] --> Y[y two]\nY -->|on| Z[z three]\nX --> Z";
        let g = parse(src).unwrap();
        let c0: Vec<(f64, f64)> = scene(&g).nodes.iter().map(|n| (n.x, n.y)).collect();
        let base = route(&g, &c0);
        let ic = g.node_index("C").unwrap();
        for (dx, dy) in [(40.0, 0.0), (0.0, 30.0), (25.0, -20.0)] {
            let mut c1 = c0.clone();
            c1[ic] = (c0[ic].0 + dx, c0[ic].1 + dy);
            let part = route_partial(&g, &c1, &base, &c0);
            let full = route(&g, &c1);
            assert_eq!(format!("{:?}", part.nodes), format!("{:?}", full.nodes));
            let mut seen = 0;
            for (i, e) in full.edges.iter().enumerate() {
                if e.from == "C" || e.to == "C" {
                    let p = &part.edges[i];
                    assert_eq!(
                        format!("{p:?}"),
                        format!("{e:?}"),
                        "edge {i} at ({dx}, {dy})"
                    );
                    seen += 1;
                }
            }
            assert_eq!(seen, 2);
            // Far from the drag, the base geometry stands.
            let xy = g
                .edges
                .iter()
                .position(|e| g.nodes[e.from].id == "X")
                .unwrap();
            let (p, b) = (&part.edges[xy], &base.edges[xy]);
            assert_eq!(format!("{p:?}"), format!("{b:?}"));
        }
    }

    /// An edit that keeps every id but changes an edge's label or style,
    /// or a node's text (and so its box), must not keep the base's stale
    /// edge: the drag falls back to [`route`].
    #[test]
    fn route_partial_falls_back_after_a_label_or_style_edit() {
        let g0 = parse("flowchart TD\nA-->|x|B\nA-->C\nB-->D").unwrap();
        let base = scene(&g0);
        let c0: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut c1 = c0.clone();
        c1[g0.node_index("C").unwrap()].0 += 80.0;
        for edit in [
            "flowchart TD\nA-.->|yy|B\nA-->C\nB-->D",
            "flowchart TD\nA-->|x|B[a much longer text for B]\nA-->C\nB-->D",
        ] {
            let g = parse(edit).unwrap();
            let part = route_partial(&g, &c1, &base, &c0);
            let full = route(&g, &c1);
            assert_eq!(format!("{part:?}"), format!("{full:?}"), "{edit}");
        }
    }

    /// When most edges would be redrawn (here every node moved), the
    /// frame is exactly a [`route`].
    #[test]
    fn route_partial_hands_a_mostly_redrawn_frame_to_route() {
        let g = parse("flowchart LR\nsubgraph S\n  A-->B\nend\nB-->C\nC-->|on|D\nA-->D").unwrap();
        let base = scene(&g);
        let c0: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        let c1: Vec<(f64, f64)> = c0.iter().map(|p| (p.0 + 15.0, p.1 + 10.0)).collect();
        let part = route_partial(&g, &c1, &base, &c0);
        let full = route(&g, &c1);
        assert_eq!(format!("{part:?}"), format!("{full:?}"));
    }

    #[test]
    fn box_edge_bezier_matches_route_geometry() {
        // The standalone helper must produce the same curve route()
        // computes for the equivalent two-node graph.
        let g = parse("A[Left] --> B[Right]").unwrap();
        let s0 = scene(&g);
        let centers: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
        let s1 = route(&g, &centers);
        let a = &s1.nodes[0];
        let b = &s1.nodes[1];
        let bez = box_edge_bezier((a.x, a.y), (a.w, a.h), (b.x, b.y), (b.w, b.h), 0.0, false);
        assert_eq!(bez, s1.edges[0].bezier);
        // Self-loop variant returns a stub on the node's right side.
        let lp = box_edge_bezier((a.x, a.y), (a.w, a.h), (a.x, a.y), (a.w, a.h), 0.0, true);
        assert!(
            lp[1].0 > a.x + a.w / 2.0,
            "loop must extend right of the box"
        );
    }

    fn cluster_contains(c: &SceneCluster, n: &SceneNode) -> bool {
        n.x - n.w / 2.0 >= c.x
            && n.y - n.h / 2.0 >= c.y
            && n.x + n.w / 2.0 <= c.x + c.w
            && n.y + n.h / 2.0 <= c.y + c.h
    }

    // --- issue #60: subgraph layout geometry ---------------------------

    /// Every geometric promise a clustered scene makes: sibling frames
    /// never intersect (only nesting may), each node sits inside exactly
    /// the frames on its subgraph chain, an edge label box is at least
    /// as wide as its measured text and covers no node, and every drawn
    /// node-to-node edge ends on its target's border.
    fn assert_clean_clusters(g: &Graph, s: &Scene) {
        let idx = |id: &str| g.subgraphs.iter().position(|x| x.id == id).unwrap();
        let nested = |a: usize, b: usize| {
            let mut p = g.subgraphs[a].parent;
            while let Some(x) = p {
                if x == b {
                    return true;
                }
                p = g.subgraphs[x].parent;
            }
            false
        };
        let overlap = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
            let ox = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
            let oy = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
            ox > 0.5 && oy > 0.5
        };
        let rect = |c: &SceneCluster| (c.x, c.y, c.w, c.h);
        let nrect = |n: &SceneNode| (n.x - n.w / 2.0, n.y - n.h / 2.0, n.w, n.h);
        for (i, a) in s.clusters.iter().enumerate() {
            for b in &s.clusters[i + 1..] {
                let (ia, ib) = (idx(&a.id), idx(&b.id));
                if nested(ia, ib) || nested(ib, ia) {
                    continue;
                }
                assert!(
                    !overlap(rect(a), rect(b)),
                    "frames {} and {} intersect",
                    a.id,
                    b.id
                );
            }
        }
        let paths = node_cluster_paths(g);
        for (v, n) in s.nodes.iter().enumerate() {
            for c in &s.clusters {
                if paths[v].contains(&idx(&c.id)) {
                    assert!(cluster_contains(c, n), "{} pokes out of {}", n.id, c.id);
                } else {
                    assert!(
                        !overlap(rect(c), nrect(n)),
                        "{} drawn inside {}",
                        n.id,
                        c.id
                    );
                }
            }
        }
        for e in &s.edges {
            if let Some((t, (mx, my), w)) = &e.label {
                assert!(*w >= text_width(t), "label box narrower than '{t}'");
                let h = crate::layout::LINE_H * t.split('\n').count() as f64;
                let lr = (mx - w / 2.0, my - h / 2.0, *w, h);
                for n in &s.nodes {
                    assert!(!overlap(lr, nrect(n)), "label '{t}' covers {}", n.id);
                }
            }
        }
        for (ge, se) in g.edges.iter().zip(&s.edges) {
            if ge.from == ge.to || matches!(ge.kind, EdgeKind::Invisible) {
                continue;
            }
            let p = se.waypoints.last().copied().unwrap_or(se.bezier[3]);
            let n = &s.nodes[ge.to];
            let dx = (n.x - p.0).abs() - n.w / 2.0;
            let dy = (n.y - p.1).abs() - n.h / 2.0;
            assert!(
                dx.max(dy) > -3.0 && dx.max(dy) < 3.0,
                "{} -> {} ends {:.1}px off its target's border",
                se.from,
                se.to,
                dx.max(dy)
            );
        }
    }

    #[test]
    fn sibling_frames_never_intersect() {
        for src in [
            // A cross-cluster lane must not drag A's frame over B.
            "flowchart TB\nsubgraph A\n  a1\n  a3\nend\n\
             subgraph B\n  b1 --> b2 --> b3\nend\na1 --> b3",
            // LR: the title strip lies across the breadth axis.
            "flowchart LR\ni --> p1 & d1\nsubgraph P\n  p1\nend\nsubgraph D\n  d1\nend",
            // Siblings sharing layers keep one left-to-right order.
            "flowchart TB\nsubgraph S\n  hv --> ss --> es\nend\n\
             subgraph B\n  hv --> restic --> drill\n  devdb --> nodev\n  pgp --> rep\nend\n\
             subgraph O\n  prom --> am --> tg\n  graf --> prom & loki\n  drc --> tg\n  apmOff --> apmDev\nend",
            "flowchart RL\nx -->|to p| p1\nx -->|to d| d2\nsubgraph P\n  p1 --> p2\nend\n\
             subgraph D\n  d1 --> d2\n  subgraph E\n    e1\n  end\nend\nd2 --> e1",
            NESTED,
        ] {
            let g = parse(src).unwrap();
            assert_clean_clusters(&g, &scene(&g));
        }
    }

    #[test]
    fn walls_of_a_box_are_straight_so_layers_cannot_bulge() {
        // On every layer a cluster spans, its members must stay inside
        // the band of its frame — the frame IS the bounding box, so the
        // check is that no sibling member falls between its walls.
        let g = parse(
            "flowchart TB\nsubgraph L\n  a --> b --> c\nend\n\
             subgraph R\n  x --> y --> z\nend\na --> z\nx --> c",
        )
        .unwrap();
        assert_clean_clusters(&g, &scene(&g));
    }

    #[test]
    fn edges_into_a_subgraph_rank_its_members_below_the_source() {
        // `root --> a` names the box: the members must still rank under
        // root (TB = vertical ranks), not share its row.
        let g = parse(
            "flowchart TB\nroot\nsubgraph a\n  a1\n  a2\nend\n\
             subgraph b\n  b1\n  b2\nend\nroot --> a\nroot --> b",
        )
        .unwrap();
        let s = scene(&g);
        assert_clean_clusters(&g, &s);
        let root = &s.nodes[0];
        for n in &s.nodes[1..] {
            assert!(
                n.y - n.h / 2.0 > root.y + root.h / 2.0,
                "{} beside root",
                n.id
            );
        }
        for c in &s.clusters {
            assert!(c.y > root.y + root.h / 2.0, "box {} not below root", c.id);
        }
        // A box as SOURCE ranks above the target node.
        let g = parse("flowchart LR\nsubgraph s\n  p --> q\nend\ns --> t").unwrap();
        let s = scene(&g);
        let (q, t) = (&s.nodes[1], &s.nodes[2]);
        assert!(t.x > q.x, "t must follow the box's exit member");
    }

    #[test]
    fn state_composite_keeps_flow_order() {
        let src = "stateDiagram-v2\n[*] --> Idle\nIdle --> Checking\n\
                   state Checking {\n  [*] --> A\n  A --> B\n  B --> [*]\n}\n\
                   Checking --> Done\nDone --> [*]";
        let g = match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            _ => unreachable!(),
        };
        let s = scene(&g);
        let y = |id: &str| s.nodes.iter().find(|n| n.id == id).unwrap().y;
        let c = s.clusters.iter().find(|c| c.id == "Checking").unwrap();
        assert!(y("Idle") < c.y, "Idle above the composite");
        assert!(y("Done") > c.y + c.h, "Done below the composite");
    }

    #[test]
    fn edgeless_members_pack_into_a_grid_not_one_row() {
        let members: String = (0..9).map(|i| format!("  m{i}[\"note {i}\"]\n")).collect();
        let g = parse(&format!(
            "flowchart TB\nroot\nsubgraph s\n{members}end\nroot --> s"
        ))
        .unwrap();
        let s = scene(&g);
        assert_clean_clusters(&g, &s);
        let mut rows: Vec<i64> = s.nodes[1..].iter().map(|n| n.y.round() as i64).collect();
        rows.sort();
        rows.dedup();
        assert_eq!(rows.len(), 3, "9 edge-less members fill a 3x3 grid");
        // An explicit cross-flow `direction` keeps the author's row.
        let g = parse(&format!(
            "flowchart TB\nsubgraph s\n  direction LR\n{members}end"
        ))
        .unwrap();
        let s = scene(&g);
        assert!(s.nodes.iter().all(|n| (n.y - s.nodes[0].y).abs() < 0.5));
        // Below the threshold nothing changes.
        let g = parse("flowchart TB\nsubgraph s\n  a\n  b\n  c\nend").unwrap();
        let s = scene(&g);
        assert!(s.nodes.iter().all(|n| (n.y - s.nodes[0].y).abs() < 0.5));
    }

    #[test]
    fn hub_fed_leaves_in_a_box_pack_into_a_grid() {
        // D2: members whose only link is an in-edge from one outside hub
        // stack like edge-less ones (from GRID_MIN_MEMBERS on).
        let leaves: String = (0..4).map(|i| format!("  l{i}[\"leaf {i}\"]\n")).collect();
        let feeds: String = (0..4).map(|i| format!("hub --> l{i}\n")).collect();
        let g = parse(&format!(
            "flowchart TB\nhub\nsubgraph s\n{leaves}end\n{feeds}"
        ))
        .unwrap();
        let s = scene(&g);
        assert_clean_clusters(&g, &s);
        let mut rows: Vec<i64> = s.nodes[1..].iter().map(|n| n.y.round() as i64).collect();
        rows.sort();
        rows.dedup();
        assert_eq!(rows.len(), 2, "4 fed leaves fill a 2x2 grid");
        // Three edge-less notes still share one row (GRID_MIN_EDGELESS).
        let g = parse("flowchart TB\nsubgraph s\n  a\n  b\n  c\nend").unwrap();
        let s = scene(&g);
        assert!(s.nodes.iter().all(|n| (n.y - s.nodes[0].y).abs() < 0.5));
    }

    #[test]
    fn a_ribbon_of_leaves_is_staggered_into_rows() {
        // D1: 16 leaves under one root would be one 16-wide row (G12).
        let leaves: String = (0..16)
            .map(|i| format!("r --> l{i}[\"leaf number {i}\"]\n"))
            .collect();
        let g = parse(&format!("flowchart TB\n{leaves}")).unwrap();
        let s = scene(&g);
        let root = s.nodes[0].y;
        let mut rows: Vec<i64> = s.nodes[1..].iter().map(|n| n.y.round() as i64).collect();
        rows.sort();
        rows.dedup();
        assert_eq!(rows.len(), 2, "leaves not in two rows: {rows:?}");
        // Every second leaf steps out: two rows of eight.
        let in_row = |y: i64| {
            s.nodes[1..]
                .iter()
                .filter(|n| n.y.round() as i64 == y)
                .count()
        };
        assert_eq!((in_row(rows[0]), in_row(rows[1])), (8, 8));
        assert!(
            s.nodes[1..].iter().all(|n| n.y > root),
            "a leaf above its root"
        );
        // One row would need at least every leaf's width side by side.
        let row: f64 = s.nodes[1..].iter().map(|n| n.w).sum();
        assert!(s.width < row, "one row: {:.0} >= {row:.0}", s.width);
        // Deterministic.
        assert_eq!(format!("{:?}", scene(&g).nodes), format!("{:?}", s.nodes));
    }

    #[test]
    fn wide_sibling_boxes_stack_instead_of_a_ribbon() {
        // D1 stacking: two wide boxes sharing their layers sit one above
        // the other once side by side would be a ribbon.
        let row = |p: &str| -> String {
            (0..8)
                .map(|i| format!("  {p}a{i}[\"{p} item {i}\"] --> {p}b{i}[\"{p} out {i}\"]\n"))
                .collect()
        };
        let g = parse(&format!(
            "flowchart TB\nsubgraph A\n{}end\nsubgraph B\n{}end",
            row("x"),
            row("y")
        ))
        .unwrap();
        let s = scene(&g);
        assert_clean_clusters(&g, &s);
        let a = s.clusters.iter().find(|c| c.id == "A").unwrap();
        let b = s.clusters.iter().find(|c| c.id == "B").unwrap();
        assert!(
            a.y + a.h <= b.y || b.y + b.h <= a.y,
            "boxes side by side: A {:?} B {:?}",
            (a.x, a.y, a.w, a.h),
            (b.x, b.y, b.w, b.h)
        );
    }

    #[test]
    fn largest_box_is_centred_among_its_siblings() {
        // D3: declared first, the big box would sit at the left rim.
        let g = parse(
            "flowchart TB\nsubgraph Big\n  a[\"alpha\"] --> b[\"beta\"]\n  c[\"gamma\"] --> d[\"delta\"]\n  e[\"epsilon\"] --> f[\"zeta\"]\nend\nsubgraph S1\n  x\nend\nsubgraph S2\n  y\nend",
        )
        .unwrap();
        let s = scene(&g);
        assert_clean_clusters(&g, &s);
        let mid = |id: &str| {
            let c = s.clusters.iter().find(|c| c.id == id).unwrap();
            c.x + c.w / 2.0
        };
        let (big, s1, s2) = (mid("Big"), mid("S1"), mid("S2"));
        assert!(
            big > s1.min(s2) && big < s1.max(s2),
            "Big at {big:.0}, siblings at {s1:.0} / {s2:.0}"
        );
    }

    #[test]
    fn equal_boxes_keep_their_declaration_order() {
        // No node clearly outsizes the others, so none is centred (D3/D4)
        // and the author's order stands.
        let g = parse("flowchart TD; a; b; c; d; e").unwrap();
        let s = scene(&g);
        assert!(s.nodes.windows(2).all(|w| w[0].x < w[1].x), "row reordered");
        let g = parse("flowchart LR; A-->X; B-->X; C-->X; D-->X").unwrap();
        let s = scene(&g);
        let ys: Vec<f64> = ["A", "B", "C", "D"]
            .iter()
            .map(|id| s.nodes[g.nodes.iter().position(|n| n.id == *id).unwrap()].y)
            .collect();
        assert!(
            ys.windows(2).all(|w| w[0] < w[1]),
            "fan-in reordered: {ys:?}"
        );
    }

    #[test]
    fn a_straight_edge_into_the_largest_node_stays_straight() {
        // The diamond is the largest node, but it already sits on one
        // axis with `Read input`: the D4 slide must not kink B -> C (G8).
        let g = parse(
            "flowchart TD\nA([Start]) --> B[Read input]\nB --> C{Valid?}\nC -->|yes| D\nC -->|no| E[Show error]\nE --> B\nD ==> F((Done))",
        )
        .unwrap();
        let s = scene(&g);
        let x = |id: &str| s.nodes[g.nodes.iter().position(|n| n.id == id).unwrap()].x;
        assert!((x("B") - x("C")).abs() < 0.5, "B {} C {}", x("B"), x("C"));
    }

    #[test]
    fn largest_node_walks_to_the_middle_of_its_rank() {
        // D3/D4: declared last, the big node would end the row.
        let g = parse(
            "flowchart TB\nr --> x1\nr --> x2\nr --> x3\nr --> x4\nr --> big[\"a much larger node<br/>with<br/>several<br/>lines\"]",
        )
        .unwrap();
        let s = scene(&g);
        let xs: Vec<f64> = s.nodes[1..5].iter().map(|n| n.x).collect();
        let big = s.nodes[5].x;
        let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            big > lo && big < hi,
            "big node at the rim: {big:.0} not in {lo:.0}..{hi:.0}"
        );
    }

    #[test]
    fn edges_reach_the_border_of_a_box_top_member() {
        // The title-strip reservation must not move edge ends off the
        // visible node (it used to inflate the node's own layer size).
        for src in [
            "flowchart TD\nA --> B\nsubgraph S\n  B --> C\nend",
            "flowchart LR\nA --> B\nsubgraph S\n  B --> C\nend",
        ] {
            let g = parse(src).unwrap();
            assert_clean_clusters(&g, &scene(&g));
        }
    }

    // --- issue #60 review: nesting, bows, box edges, titles ----------

    /// A sampled drawn edge passes through a rect (shrunk by 3px).
    fn edge_hits(e: &SceneEdge, r: (f64, f64, f64, f64)) -> bool {
        edge_polyline(e).iter().any(|&(x, y)| {
            x > r.0 + 3.0 && x < r.0 + r.2 - 3.0 && y > r.1 + 3.0 && y < r.1 + r.3 - 3.0
        })
    }

    fn node_rect(n: &SceneNode) -> (f64, f64, f64, f64) {
        (n.x - n.w / 2.0, n.y - n.h / 2.0, n.w, n.h)
    }

    #[test]
    fn stacked_nested_frames_clear_each_other_in_every_direction() {
        // BT draws layers bottom-up: the title strip must be reserved on
        // a box's highest layer, and deep nests stack pads and strips
        // past the plain layer gap.
        for src in [
            "flowchart BT\nsubgraph A\n subgraph A2\n  a\n  c\n end\nend\n\
             subgraph B\n b\nend\nA2 --> b\nA --> a",
            "flowchart BT\nsubgraph A\n subgraph A2\n  c --> a\n end\nend\n\
             subgraph B\n b\nend\na --> b",
            "flowchart BT\nsubgraph S1\nsubgraph S1x\nsubgraph S1xx\n  n1\n  n4\nend\nend\nend\n\
             subgraph S1s\n  n8\nend\nS1x -->|lbl| n8\nS1 --> n1",
            "flowchart BT\nsubgraph S4\nsubgraph S4x\nsubgraph S4xx\n  n9\nend\nend\n\
             subgraph S4xs\n  n4\nend\nend\nsubgraph S4s\nsubgraph S4sx\nend\nend\n\
             n9 --> n3\nn10 -.-> S4xx",
            "flowchart TB\nsubgraph S\nsubgraph Sx\nsubgraph Sxx\n  n2\nend\nend\nend\n\
             subgraph T\nsubgraph Tx\nsubgraph Txx\nend\nsubgraph Txxs\n  n5\n  n3\nend\nend\nend\n\
             n3 --> Tx\nn0 ==> Sx\nn7 ==> n0",
        ] {
            let g = parse(src).unwrap();
            assert_clean_clusters(&g, &scene(&g));
        }
    }

    #[test]
    fn back_edge_inside_a_box_bows_beside_it_not_round_the_diagram() {
        // A wide sibling box shares the back-edge's layers; the bow only
        // has to clear what lies between its own ends.
        let wide: String = (0..8).map(|i| format!("  w{i} --> v{i}\n")).collect();
        let g = parse(&format!(
            "flowchart TB\nsubgraph P\n  g --> c\n  c -->|back| g\nend\nsubgraph W\n{wide}end"
        ))
        .unwrap();
        let s = scene(&g);
        assert_clean_clusters(&g, &s);
        let w = s.clusters.iter().find(|c| c.id == "W").unwrap();
        let p = s.clusters.iter().find(|c| c.id == "P").unwrap();
        let back = s
            .edges
            .iter()
            .find(|e| e.from == "c" && e.to == "g")
            .unwrap();
        assert!(!edge_hits(back, (w.x, w.y, w.w, w.h)), "bow crosses W");
        // Its label stays clear of W too, close beside P.
        // (W may sit beside P or, narrowed, below it: either way the
        // label box must not touch W's frame.)
        let ((lx, ly), lw) = back.label.as_ref().map(|l| (l.1, l.2)).unwrap();
        let lh = LABEL_BOX_H / 2.0;
        assert!(
            lx + lw / 2.0 < w.x
                || lx - lw / 2.0 > w.x + w.w
                || ly + lh < w.y
                || ly - lh > w.y + w.h,
            "label over W"
        );
        let reach = (lx - (p.x + p.w / 2.0)).abs() + lw / 2.0;
        assert!(reach < 2.0 * p.w, "label {reach:.0}px from P's centre");
    }

    #[test]
    fn edge_to_a_far_box_routes_round_nodes_and_stops_at_its_frame() {
        // `r --> s` ranks s three layers down; the stand-in channel keeps
        // the drawn edge clear of x / y and it ends on s's top side.
        let g = parse(
            "flowchart TB\nr --> x --> y\nsubgraph s\n  a\nend\ny --> a\n\
             r -->|deploy| s",
        )
        .unwrap();
        let s = scene(&g);
        assert_clean_clusters(&g, &s);
        let e = s.edges.iter().find(|e| e.to == "s").unwrap();
        assert!(e.waypoints.len() >= 3, "routed through a channel");
        let f = s.clusters.iter().find(|c| c.id == "s").unwrap();
        let end = *e.waypoints.last().unwrap();
        assert!((end.1 - f.y).abs() < 0.5, "ends on the frame top");
        assert!(end.0 > f.x && end.0 < f.x + f.w);
        for n in &s.nodes {
            assert!(!edge_hits(e, node_rect(n)), "crosses {}", n.id);
        }
        // An adjacent box: a short curve to the facing side, no overshoot
        // past either end (that padded the canvas with a blank band).
        let g = parse("flowchart TB\ns0((s)) --> O\nsubgraph O\n  o0((o)) --> M\n  subgraph M\n    m0((m)) --> i1\n  end\nend").unwrap();
        let s = scene(&g);
        let top = s
            .nodes
            .iter()
            .map(|n| n.y - n.h / 2.0)
            .fold(f64::INFINITY, f64::min);
        assert!(
            (top - MARGIN).abs() < 0.5,
            "first rank at the margin, got {top}"
        );
        for e in s.edges.iter().filter(|e| e.waypoints.is_empty()) {
            let (lo, hi) = (
                e.bezier[0].1.min(e.bezier[3].1),
                e.bezier[0].1.max(e.bezier[3].1),
            );
            assert!(e.bezier.iter().all(|p| p.1 >= lo - 0.5 && p.1 <= hi + 0.5));
        }
    }

    #[test]
    fn box_to_box_ranking_stays_linear_and_complete() {
        // Two big boxes joined by one edge: every member of B below every
        // member of A, via one hub per edge instead of |A| x |B| pairs.
        let k = 60;
        let a: String = (0..k).map(|i| format!("  ax{i} --> ay{i}\n")).collect();
        let b: String = (0..k).map(|i| format!("  bx{i} --> by{i}\n")).collect();
        let g = parse(&format!(
            "flowchart TB\nsubgraph A\n{a}end\nsubgraph B\n{b}end\nA --> B"
        ))
        .unwrap();
        let groups = sub_edge_proxies(&g, &node_cluster_paths(&g), &[]);
        assert_eq!(groups.len(), 1);
        assert_eq!((groups[0].0.len(), groups[0].1.len()), (k, k));
        let s = scene(&g);
        let low_a = s
            .nodes
            .iter()
            .filter(|n| n.id.starts_with('a'))
            .map(|n| n.y)
            .fold(0.0, f64::max);
        let high_b = s
            .nodes
            .iter()
            .filter(|n| n.id.starts_with('b'))
            .map(|n| n.y)
            .fold(f64::INFINITY, f64::min);
        assert!(high_b > low_a, "B ranks below A");
    }

    #[test]
    fn colliding_label_placement_stays_near_linear() {
        // A 1000-edge labelled fan used to take seconds (every spot of
        // every colliding label rescanned every obstacle). The grid keeps
        // each query local: a label far from the rest sees none of them.
        let k = 1000;
        let fan: String = (0..k)
            .map(|i| format!("  H -->|label {i}| T{i}\n"))
            .collect();
        let g = parse(&format!("flowchart TB\n{fan}")).unwrap();
        let t0 = std::time::Instant::now();
        let s = scene(&g);
        let took = t0.elapsed();
        assert_eq!(s.edges.iter().filter(|e| e.label.is_some()).count(), k);
        assert!(took.as_secs_f64() < 20.0, "label placement took {took:?}");

        let mut grid = ObstacleGrid::new();
        for i in 0..k {
            let x = i as f64 * 200.0;
            grid.push((x, 0.0, x + 60.0, LABEL_BOX_H));
        }
        let probe = (500.0 * 200.0, 0.0, 500.0 * 200.0 + 60.0, LABEL_BOX_H);
        assert!(grid.near(probe).len() <= 3, "query must stay local");
        assert!(!grid.clear(probe, 0.0));
        assert!(grid.clear((0.0, 100.0, 60.0, 120.0), LABEL_GAP));
        let all: Vec<Rect> = grid.rects.clone();
        assert_eq!(
            grid.covered(probe, LABEL_GAP),
            covered(probe, LABEL_GAP, &all)
        );
    }

    #[test]
    fn routed_label_stays_on_the_drawn_curve() {
        // A zig-zag lane: curveBasis cuts the corner far from the middle
        // point, so the label moves onto the curve.
        let zig = [
            (0.0, 0.0),
            (0.0, 100.0),
            (300.0, 200.0),
            (0.0, 300.0),
            (0.0, 400.0),
        ];
        let p = channel_label_point(&zig);
        let d = basis_sample(&zig)
            .iter()
            .map(|q| (q.0 - p.0).hypot(q.1 - p.1))
            .fold(f64::INFINITY, f64::min);
        assert!(d < 1e-9 && p != zig[2]);
        // A straight lane keeps its reserved slot exactly.
        let line = [(0.0, 0.0), (0.0, 100.0), (0.0, 200.0), (0.0, 300.0)];
        assert_eq!(channel_label_point(&line), line[2]);
    }

    /// The right edge of the canvas `to_svg` draws `s` on.
    fn canvas_right(s: &Scene) -> f64 {
        let mut bb = Bbox::new();
        grow_scene(&mut bb, &s.nodes, &s.edges, &s.clusters);
        bb.finish().1 + MARGIN
    }

    #[test]
    fn long_titles_are_cut_only_where_they_would_run_into_a_sibling() {
        let long = "a very long subgraph title that is much wider than its box";
        let g = parse(&format!(
            "flowchart TB\nsubgraph L [\"{long}\"]\n  l\nend\nsubgraph R [\"{long}\"]\n  r\nend"
        ))
        .unwrap();
        let s = scene(&g);
        let li = s.clusters.iter().position(|c| c.id == "L").unwrap();
        let ri = s.clusters.iter().position(|c| c.id == "R").unwrap();
        let (left, right) = if s.clusters[li].x < s.clusters[ri].x {
            (li, ri)
        } else {
            (ri, li)
        };
        let cut = fit_cluster_title(&s.clusters, left, f64::INFINITY);
        assert!(cut.ends_with('…'), "left title cut: {cut}");
        assert!(
            text_width(&cut) * TITLE_SCALE + 8.0 <= s.clusters[left].w - 2.0 * TITLE_INSET + 0.5
        );
        // The rightmost one overflows into no frame: kept whole...
        assert_eq!(fit_cluster_title(&s.clusters, right, f64::INFINITY), long);
        // ...unless it would run off the canvas, which clips it
        // mid-word: then it is cut like its sibling.
        let edge = fit_cluster_title(&s.clusters, right, canvas_right(&s));
        assert!(edge.ends_with('…'), "rightmost title cut: {edge}");
        assert!(!to_svg(&s).contains(&format!("{long}</text>")));
    }

    #[test]
    fn br_titles_draw_every_line_and_get_header_room_in_every_direction() {
        for dir in ["TB", "BT", "LR", "RL"] {
            let g = parse(&format!(
                "flowchart {dir}\nX --> a\n\
                 subgraph S [\"First line<br/>second · line<br/>third\"]\n  a --> b\nend\n\
                 subgraph T [\"One line\"]\n  c\nend\nb --> c"
            ))
            .unwrap();
            let s = scene(&g);
            let svg = to_svg(&s);
            for line in ["First line</text>", "second · line</text>", "third</text>"] {
                assert!(svg.contains(line), "{dir}: {line} missing");
            }
            let c = s.clusters.iter().find(|c| c.id == "S").unwrap();
            let strip_bottom = c.y + TITLE_STRIP + title_extra(&c.title);
            for n in s.nodes.iter().filter(|n| n.id == "a" || n.id == "b") {
                let (x0, y0, x1, _) = rect_of_node(n);
                let chip_right = c.x + TITLE_INSET + text_width(&c.title) * TITLE_SCALE + 8.0;
                assert!(
                    y0 >= strip_bottom || x0 >= chip_right || x1 <= c.x,
                    "{dir}: node {} under the 3-line title",
                    n.id
                );
            }
        }
    }

    #[test]
    fn cut_title_lines_drop_a_dangling_separator() {
        let mk = |id: &str, x: f64, title: &str| SceneCluster {
            id: id.into(),
            x,
            y: 0.0,
            w: 120.0,
            h: 80.0,
            title: title.into(),
            depth: 0,
        };
        // The cut falls right after " · ": without the trim the line
        // would read "ALPHA BRAV ·…".
        let title = "Short\nALPHA BRAV · CHARLIE DELTA ECHO FOXTROT";
        let cl = [mk("L", 0.0, title), mk("R", 130.0, "r")];
        let cut = fit_cluster_title(&cl, 0, f64::INFINITY);
        let lines: Vec<&str> = cut.split('\n').collect();
        assert_eq!(lines[0], "Short", "a fitting line stays whole");
        assert_eq!(lines[1], "ALPHA BRAV…");
        let kept = lines[1].trim_end_matches('…');
        assert!(
            !kept.ends_with(|ch: char| ch.is_whitespace() || TITLE_CUT_SEPARATORS.contains(&ch)),
            "dangling separator in {cut:?}"
        );
        assert!(text_width(lines[1]) * TITLE_SCALE <= 120.0 - 2.0 * TITLE_INSET - 8.0);
    }

    #[test]
    fn a_title_overflowing_its_frame_onto_the_canvas_stays_whole() {
        let long = "a long subgraph title, wider than its box";
        let s = scene(
            &parse(&format!(
                "flowchart TB\nsubgraph R [\"{long}\"]\n  r\nend\n\
                 r --> w[\"a node label that is far wider than the subgraph title above it, so \
                 the canvas reaches well past the title\"]"
            ))
            .unwrap(),
        );
        let ri = s.clusters.iter().position(|c| c.id == "R").unwrap();
        let c = &s.clusters[ri];
        assert!(text_width(long) * TITLE_SCALE + 8.0 > c.w - 2.0 * TITLE_INSET);
        assert_eq!(fit_cluster_title(&s.clusters, ri, canvas_right(&s)), long);
        assert!(to_svg(&s).contains(&format!("{long}</text>")));
    }

    const NESTED: &str = "flowchart TD\n\
        In[Request] --> A1\n\
        subgraph backend [Backend Services]\n\
        A1[API] --> W1\n\
        subgraph workers\n\
        W1[Worker 1] --> W2[Worker 2]\n\
        end\n\
        end\n\
        W2 --> Out[Response]\n";

    #[test]
    fn subgraph_scene_wraps_members_and_nests() {
        let g = parse(NESTED).unwrap();
        let s = scene(&g);
        assert_eq!(s.clusters.len(), 2);
        // Outermost first for painting.
        assert!(s.clusters[0].depth <= s.clusters[1].depth);
        let outer = &s.clusters[0];
        let inner = &s.clusters[1];
        assert_eq!(outer.title, "Backend Services");
        // Inner box fully inside outer box.
        assert!(
            inner.x >= outer.x
                && inner.y >= outer.y
                && inner.x + inner.w <= outer.x + outer.w
                && inner.y + inner.h <= outer.y + outer.h,
            "nested cluster must sit inside its parent"
        );
        // Members inside their boxes; outsiders outside the outer box.
        for (i, n) in s.nodes.iter().enumerate() {
            let id = g.nodes[i].id.as_str();
            match id {
                "W1" | "W2" => assert!(cluster_contains(inner, n), "{} in workers", id),
                "A1" => assert!(cluster_contains(outer, n), "A1 in backend"),
                _ => assert!(!cluster_contains(outer, n), "{} outside backend", id),
            }
        }
        // Everything inside the canvas.
        for c in &s.clusters {
            assert!(c.x >= 0.0 && c.y >= 0.0);
            assert!(c.x + c.w <= s.width && c.y + c.h <= s.height);
        }
        // Export contains the titled boxes.
        let svg = to_svg(&s);
        assert!(svg.contains("Backend Services") && svg.contains("workers"));
    }

    #[test]
    fn empty_and_nested_empty_subgraphs_stay_finite() {
        // Regression (bughunter): empty subgraph levels used to
        // produce -inf dimensions that poisoned the whole SVG.
        for src in [
            "graph TD\nA[Node1]\nsubgraph outer\nsubgraph inner\nend\nend",
            "graph TD\nA --> B\nsubgraph l1[L1]\nsubgraph l2[L2]\nsubgraph l3[L3]\nend\nend\nend",
            "flowchart TD\nsubgraph only\nend",
        ] {
            let s = scene(&parse(src).unwrap());
            assert!(s.width.is_finite() && s.height.is_finite(), "{}", src);
            for n in &s.nodes {
                assert!(n.x.is_finite() && n.y.is_finite(), "{}", src);
            }
            for c in &s.clusters {
                assert!(
                    c.x.is_finite() && c.y.is_finite() && c.w.is_finite() && c.h.is_finite(),
                    "cluster in {}",
                    src
                );
                assert!(c.w >= 0.0 && c.h >= 0.0);
            }
            let svg = to_svg(&s);
            assert!(!svg.contains("NaN") && !svg.contains("inf"), "{}", src);
        }
    }

    #[test]
    fn dragging_a_member_pulls_its_cluster_along() {
        let g = parse(NESTED).unwrap();
        let s0 = scene(&g);
        let mut pos: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
        let w2 = g.node_index("W2").unwrap();
        pos[w2].0 += 600.0;
        let s1 = route(&g, &pos);
        let inner = s1.clusters.iter().find(|c| c.title == "workers").unwrap();
        assert!(
            cluster_contains(inner, &s1.nodes[w2]),
            "cluster must follow the drag"
        );
    }

    #[test]
    fn route_attaches_to_node_borders() {
        let g = parse("A[Left] --> B[Right]").unwrap();
        let s0 = scene(&g);
        let mut pos: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
        pos[1] = (pos[0].0 + 400.0, pos[0].1); // "drag" B far to the right
        let s1 = route(&g, &pos);
        let e = s1.edges[0].bezier;
        let a = &s1.nodes[0];
        let b = &s1.nodes[1];
        assert!(
            (e[0].0 - (a.x + a.w / 2.0)).abs() < 1.0,
            "edge must leave from A's right side"
        );
        assert!(
            (e[3].0 - (b.x - b.w / 2.0)).abs() < 1.0,
            "edge must enter at B's left side"
        );
    }

    #[test]
    fn a_colour_set_through_the_public_api_cannot_close_its_attribute() {
        // `Graph` and `NodeStyle` are public with public fields, and the
        // README points embedders at exactly this: build a scene, feed it
        // colours, render. Validating in the PARSER would leave this path
        // wide open, which is why the escaping sits at the render site.
        let mut g = parse("flowchart TD\n  A[Hi] --> B\n").unwrap();
        g.nodes[0].style.fill = Some("x\" onload=alert(1)".to_string());
        g.nodes[0].style.stroke = Some("y' onload=alert(1)".to_string());
        g.nodes[0].style.color = Some("z\" onload=alert(1)".to_string());
        let svg = crate::render::render(&g);
        assert!(
            !svg.contains("\" onload") && !svg.contains("' onload"),
            "a colour escaped its attribute:\n{svg}"
        );
        assert!(svg.contains("&quot;"), "{svg}");
    }

    #[test]
    fn control_characters_are_dropped_not_escaped() {
        // XML 1.0 forbids C0 controls outright — even `&#0;` is illegal —
        // so one NUL from an untrusted diagram would make the document
        // unparseable and a browser would show an error instead of the
        // drawing. Tab, newline and return are the legal three.
        let out = escape("a\u{0}b\u{1}c\u{b}d");
        assert_eq!(out, "abcd", "control characters survived: {out:?}");
        assert_eq!(escape("a\tb\nc\rd"), "a\tb\nc\rd");
        // Ordinary escaping is unchanged.
        assert_eq!(
            escape("<a & \"b\" 'c'>"),
            "&lt;a &amp; &quot;b&quot; &#39;c&#39;&gt;"
        );
    }

    // ── linkStyle ───────────────────────────────────────────────

    #[test]
    fn link_style_paints_the_edge_and_colours_its_arrow() {
        let svg = svg_src(
            "flowchart TD\nA --> B\nB --> C\nlinkStyle 1 stroke:#ff3,stroke-width:4px,color:red;\n",
        );
        let p = edge_paths(&svg);
        assert!(
            p[0].contains("stroke=\"#44507a\" stroke-width=\"1.7\" marker-end=\"url(#arrow)\""),
            "{svg}"
        );
        assert!(
            p[1].contains("stroke=\"#ff3\" stroke-width=\"4\" marker-end=\"url(#arrow-u__ff3)\""),
            "{svg}"
        );
        // Width-independent head, coloured like the stroke.
        assert!(svg.contains(
            "<marker id=\"arrow-u__ff3\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"11.9\" markerHeight=\"11.9\" \
             orient=\"auto\"><path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"#ff3\"/></marker>"
        ));
    }

    #[test]
    fn link_style_on_a_sub_edge_and_default_markers_are_deduped() {
        let svg = svg_src(
            "flowchart LR\nsubgraph S\n  X\nend\nA & B --> C & D\nA --> S\nC ~~~ D\n\
             D --> E --> F\nlinkStyle 4 stroke:red\nlinkStyle 6,7 stroke:blue\n",
        );
        let p = edge_paths(&svg);
        let a_s = p
            .iter()
            .find(|l| l.contains("<title>A → S</title>"))
            .unwrap();
        assert!(a_s.contains("stroke=\"red\"") && a_s.contains("url(#arrow_red)"));
        for t in ["D → E", "E → F"] {
            let l = p.iter().find(|l| l.contains(t)).unwrap();
            assert!(l.contains("stroke=\"blue\"") && l.contains("url(#arrow_blue)"));
        }
        assert_eq!(svg.matches("<marker id=\"arrow_blue\"").count(), 1);
        // The invisible link stays hidden with a stroke-only style.
        assert!(!svg.contains("C → D"));
    }

    #[test]
    fn link_style_effective_stroke_drives_the_marker() {
        // Default grey 2px, edge 1 red: its arrow is red (flowmaid
        // follows the drawn stroke; mermaid would take the default's).
        let svg = svg_src(
            "flowchart TD\nlinkStyle default stroke:#888,stroke-width:2px\nA --> B --> C\n\
             linkStyle 1 stroke:#e11d48\n",
        );
        let p = edge_paths(&svg);
        assert!(
            p[0].contains("stroke=\"#888\" stroke-width=\"2\"") && p[0].contains("arrow-u__888")
        );
        assert!(
            p[1].contains("stroke=\"#e11d48\" stroke-width=\"2\"")
                && p[1].contains("arrow-u__e11d48")
        );
    }

    #[test]
    fn link_style_colours_every_marker_kind_and_only_used_defs() {
        let svg =
            svg_src("flowchart LR\nA <--> B\nC x--x D\nE o--o F\nlinkStyle 0,1,2 stroke:#16a34a\n");
        for id in [
            "arrow-start__16a34a",
            "arrow__16a34a",
            "cross-start__16a34a",
            "cross-end__16a34a",
            "circle-start__16a34a",
            "circle-end__16a34a",
        ] {
            assert!(svg.contains(&format!("<marker id=\"{id}\"")), "{id}\n{svg}");
            assert!(svg.contains(&format!("url(#{id})")), "{id}");
        }
        // No uncoloured extra defs (only the classic `#arrow` stays).
        for id in ["arrow-start\"", "cross-end\"", "circle-end\""] {
            assert!(!svg.contains(&format!("<marker id=\"{id}")), "{id}");
        }
        assert!(svg.contains("stroke=\"#16a34a\" stroke-width=\"2\"/></marker>"));
        assert!(svg.contains("fill=\"#16a34a\" stroke=\"#16a34a\" stroke-width=\"1\""));
    }

    #[test]
    fn link_style_dash_and_width_override_the_kind() {
        let svg = svg_src(
            "flowchart LR\nA -.-> B\nC ==> D\nE --> F\nlinkStyle 0 stroke-dasharray:0\n\
             linkStyle 1 stroke-width:1px\nlinkStyle 2 stroke-dasharray: 5 5\n",
        );
        let p = edge_paths(&svg);
        assert!(p[0].contains("stroke-dasharray=\"0\"") && !p[0].contains("5 4"));
        assert!(p[1].contains("stroke-width=\"1\"") && p[1].contains("url(#arrow-u)"));
        assert!(p[2].contains("stroke-dasharray=\"5 5\""));
        // The plain `-u` arrow keeps the theme colour.
        assert!(svg.contains("<marker id=\"arrow-u\""));
    }

    #[test]
    fn link_style_color_paints_only_the_label_text() {
        let svg = svg_src("flowchart LR\nA -- yes --> B\nlinkStyle 0 color:#2563eb\n");
        assert!(svg.contains("fill=\"#2563eb\">yes</text>"), "{svg}");
        let p = edge_paths(&svg);
        assert!(p[0].contains("stroke=\"#44507a\"") && p[0].contains("url(#arrow)"));
    }

    #[test]
    fn link_style_with_a_width_reveals_an_invisible_link_without_markers() {
        let svg =
            svg_src("flowchart LR\nA ~~~ B\nA --> C\nlinkStyle 0 stroke:red,stroke-width:2px\n");
        let p = edge_paths(&svg);
        let ab = p.iter().find(|l| l.contains("A → B")).unwrap();
        assert!(ab.contains("stroke=\"red\" stroke-width=\"2\"") && !ab.contains("marker"));
        let hidden = svg_src("flowchart LR\nA ~~~ B\nA --> C\nlinkStyle 0 stroke:red\n");
        assert!(!hidden.contains("A → B"));
        assert!(!svg_src("flowchart LR\nA ~~~ B\nlinkStyle 0 stroke-width:0\n").contains("A → B"));
    }

    #[test]
    fn link_style_fill_and_escaping() {
        let svg = svg_src("flowchart TD\nA --> B\nlinkStyle 0 fill:#fde68a,stroke:#b45309\n");
        assert!(edge_paths(&svg)[0].contains("fill=\"#fde68a\" stroke=\"#b45309\""));
        // Hostile values cannot break out of the attribute.
        let mut g = parse("flowchart TD\nA --> B\n").unwrap();
        g.edges[0].meta.style.stroke = Some("red\"/><script>".into());
        let svg = to_svg(&scene(&g));
        assert!(!svg.contains("<script>"));
        assert!(svg.contains("arrow_red____script_"));
    }

    #[test]
    fn link_style_colours_that_sanitise_alike_get_distinct_markers() {
        let svg = svg_src(
            "flowchart TD\nA --> B --> C\nlinkStyle 0 stroke:#f00\nlinkStyle 1 stroke:_f00\n",
        );
        assert!(svg.contains("<marker id=\"arrow__f00\""));
        assert!(svg.contains("<marker id=\"arrow__f00_2\""));
        let p = edge_paths(&svg);
        assert!(p[0].contains("url(#arrow__f00)") && p[1].contains("url(#arrow__f00_2)"));
    }

    #[test]
    fn link_style_interpolate_linear_draws_straight_segments() {
        let plain = svg_src("flowchart TD\nA --> B --> C --> D\nA --> D\n");
        let lin =
            svg_src("flowchart TD\nA --> B --> C --> D\nA --> D\nlinkStyle 3 interpolate linear\n");
        assert_ne!(plain, lin);
        let p = edge_paths(&lin);
        assert!(
            !p[3].contains(" C "),
            "linear has no cubic segments: {}",
            p[3]
        );
    }

    #[test]
    fn link_style_survives_compact_folding() {
        let g = parse(
            "flowchart LR\nA --> B --> C --> D --> E --> F --> G --> H\nlinkStyle 3 stroke:red\n",
        )
        .unwrap();
        let c = crate::fold::scene_compact(&g, &crate::fold::CompactOptions::for_extent(300.0));
        assert!(c.skipped.is_none() && c.bands > 1, "{:?}", c.skipped);
        assert_eq!(c.scene.edges[3].meta.style.stroke.as_deref(), Some("red"));
        let svg = to_svg(&c.scene);
        assert!(svg.contains("stroke=\"red\""));
    }

    #[test]
    fn diagrams_without_link_style_render_exactly_as_before() {
        // Byte-level: no styled defs, the historical stroke attrs.
        let svg = svg_src("flowchart LR\nA --> B\nC -.-> D\nE ==> F\nG x--x H\n");
        assert!(!svg.contains("-u\"") && !svg.contains("_"), "{svg}");
        assert!(svg.contains("stroke-dasharray=\"5 4\""));
        assert!(svg.contains("stroke-width=\"3.4\""));
    }

    // ── click ───────────────────────────────────────────────────

    #[test]
    fn click_wraps_the_node_in_a_link() {
        let svg = svg_src("flowchart LR\nA-->B\nclick A \"https://example.com\"\n");
        assert!(
            svg.contains(
                "<a xmlns:xlink=\"http://www.w3.org/1999/xlink\" href=\"https://example.com\" \
             xlink:href=\"https://example.com\">\n<g><title>A</title>\n"
            ),
            "{svg}"
        );
        assert_eq!(svg.matches("<a ").count(), 1);
        assert_eq!(svg.matches("</a>").count(), 1);
        assert!(!svg.contains("target="));
        // The root header is untouched.
        let plain = svg_src("flowchart LR\nA-->B\n");
        assert_eq!(svg.lines().next(), plain.lines().next());
        let svg = svg_src(
            "flowchart LR\nA-->B\nclick A href \"https://example.com\" \"Go to docs\" _blank\n",
        );
        assert!(svg.contains(
            "target=\"_blank\" rel=\"noopener noreferrer\">\n<g><title>Go to docs</title>"
        ));
        assert!(!svg.contains("<title>A</title>"));
        let svg = svg_src("flowchart LR\nA-->B\nclick A \"https://example.com\" _self\n");
        assert!(svg.contains("target=\"_self\">") && !svg.contains("rel="));
        let svg = svg_src("flowchart LR\nA-->B\nclick A \"/docs/page?a=1&b=2\"\n");
        assert!(svg.contains("href=\"/docs/page?a=1&amp;b=2\""));
    }

    #[test]
    fn click_tooltips_and_callbacks_without_links() {
        let plain = svg_src("flowchart LR\nA-->B\n");
        assert_eq!(svg_src("flowchart LR\nA-->B\nclick A callback\n"), plain);
        assert_eq!(
            svg_src("flowchart LR\nA-->B\nclick A call callback()\n"),
            plain
        );
        let svg = svg_src("flowchart LR\nA-->B\nclick A callback \"Tooltip for a callback\"\n");
        assert!(svg.contains("<g><title>Tooltip for a callback</title>") && !svg.contains("<a "));
        let svg = svg_src(
            "flowchart LR\nA-->B\nclick A \"https://x.example\" \"say #quot;hi#quot; <b>\"\n",
        );
        assert!(
            svg.contains("<title>say &quot;hi&quot; &lt;b&gt;</title>"),
            "{svg}"
        );
        // Exactly one <title> per node group.
        let g = svg.split("<g>").nth(1).unwrap_or_default();
        assert!(!g.contains("<title>A</title>"));
    }

    #[test]
    fn click_never_writes_a_dangerous_url() {
        for url in [
            "javascript:alert(1)",
            "javascript#58;alert(1)",
            "%6Aavascript:alert(1)",
            "data:text/html,x",
        ] {
            let svg = svg_src(&format!("flowchart LR\nA-->B\nclick A \"{url}\"\n"));
            assert!(!svg.contains("<a "), "{url}");
            assert!(!svg.to_lowercase().contains("javascript"), "{url}");
        }
        // A library user bypassing the parser is checked at emit time.
        let mut g = parse("flowchart LR\nA-->B\n").unwrap();
        g.links.insert(
            0,
            NodeLink {
                href: Some("javascript:alert(1)".into()),
                ..NodeLink::default()
            },
        );
        let svg = to_svg(&scene(&g));
        assert!(!svg.contains("javascript") && !svg.contains("<a "));
        // ...and so is every scene a host app reads the link from: the
        // unsafe url is dropped, the rest of the link kept.
        g.links.insert(
            0,
            NodeLink {
                href: Some("JaVaScRiPt:alert(1)".into()),
                tooltip: Some("tip".into()),
                ..NodeLink::default()
            },
        );
        g.links.insert(
            1,
            NodeLink {
                href: Some("%6Aavascript:alert(1)".into()),
                ..NodeLink::default()
            },
        );
        let sc = scene(&g);
        let l = sc.nodes[0].link.as_ref().unwrap();
        assert_eq!(
            (l.href.as_deref(), l.tooltip.as_deref()),
            (None, Some("tip"))
        );
        assert!(sc.nodes[1].link.is_none(), "nothing left, no link");
        let centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let routed = route(&g, &centers);
        assert!(routed.nodes[0].link.as_ref().unwrap().href.is_none());
        assert!(routed.nodes[1].link.is_none());
    }

    #[test]
    fn click_links_reach_every_scene_builder() {
        let g = parse(
            "flowchart LR\nA --> B --> C --> D --> E --> F --> G --> H\n\
             click A href \"https://example.com\" \"tip\" _top\n",
        )
        .unwrap();
        let sc = scene(&g);
        let l = sc.nodes[0].link.as_ref().unwrap();
        assert_eq!(l.href.as_deref(), Some("https://example.com"));
        assert_eq!(l.tooltip.as_deref(), Some("tip"));
        assert_eq!(l.target, Some(crate::model::LinkTarget::Top));
        assert!(sc.nodes[1].link.is_none());
        let centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        assert!(route(&g, &centers).nodes[0].link.is_some());
        let c = crate::fold::scene_compact(&g, &crate::fold::CompactOptions::for_extent(300.0));
        assert!(c.skipped.is_none());
        assert!(c.scene.nodes[0].link.is_some());
        assert!(to_svg(&c.scene).contains("href=\"https://example.com\""));
        // Clustered scenes too.
        let g =
            parse("flowchart LR\nsubgraph S\nA\nend\nA --> B\nclick A \"https://x\"\n").unwrap();
        assert!(scene(&g).nodes[0].link.is_some());
    }

    // ── markdown strings ────────────────────────────────────────

    #[test]
    fn markdown_labels_render_as_styled_tspans() {
        let svg = svg_src("flowchart LR\n  A[\"`This **is** _Markdown_`\"] --> B\n");
        assert!(
            svg.contains("<tspan font-weight=\"bold\">is</tspan>"),
            "{svg}"
        );
        assert!(svg.contains("<tspan font-style=\"italic\">Markdown</tspan>"));
        assert!(!svg.contains("**") && !svg.contains('`'));
        let svg = svg_src("flowchart LR\n  n[\"`Line1\n    Line 2\n    Line 3`\"]\n");
        assert_eq!(svg.matches("dy=\"17.0\"").count(), 2, "{svg}");
        let svg = svg_src("flowchart LR\n  A -->|\"`**pipe** label`\"| B\n");
        assert!(svg.contains("<tspan font-weight=\"bold\">pipe</tspan> label</text>"));
    }

    #[test]
    fn subgraph_titles_render_rich() {
        let svg = svg_src("flowchart LR\n  subgraph s1 [\"`*Italic* title`\"]\n    A\n  end\n");
        assert!(
            svg.contains(
                "fill=\"#6a7086\"><tspan font-style=\"italic\">Italic</tspan> title</text>"
            ),
            "{svg}"
        );
        // Plain titles are byte-identical to the old escape().
        let svg = svg_src("flowchart LR\n  subgraph s1 [a & b]\n    A\n  end\n");
        assert!(svg.contains("fill=\"#6a7086\">a &amp; b</text>"));
    }

    // --- issue #61: edge labels must not cover node boxes or each other ---

    fn rect_of_node(n: &SceneNode) -> Rect {
        (
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.x + n.w / 2.0,
            n.y + n.h / 2.0,
        )
    }

    fn rect_of_label(m: (f64, f64), w: f64) -> Rect {
        (
            m.0 - w / 2.0,
            m.1 - LABEL_BOX_H / 2.0,
            m.0 + w / 2.0,
            m.1 + LABEL_BOX_H / 2.0,
        )
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        covered(a, 0.0, &[b]) > 0.0
    }

    /// Geometry every rendered flowchart must satisfy: label boxes at
    /// least as wide as their text and clear of every node, other
    /// label and frame border line, rect targets reached ON their
    /// border, each node inside
    /// exactly the frames of the subgraphs holding it, and frames
    /// disjoint unless one nests in the other.
    fn assert_clean_geometry(src: &str) -> Scene {
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let labels: Vec<(String, Rect)> = sc
            .edges
            .iter()
            .filter_map(|e| e.label.as_ref())
            .map(|(t, m, w)| {
                assert!(*w >= text_width(t), "label box narrower than {t:?}");
                (t.clone(), rect_of_label(*m, *w))
            })
            .collect();
        for (i, (t, r)) in labels.iter().enumerate() {
            for n in &sc.nodes {
                assert!(
                    !overlaps(*r, rect_of_node(n)),
                    "label {t:?} covers node {}",
                    n.id
                );
            }
            for (u, o) in &labels[..i] {
                assert!(!overlaps(*r, *o), "label {t:?} covers label {u:?}");
            }
            for c in &sc.clusters {
                let (x0, y0, x1, y1) = (c.x, c.y, c.x + c.w, c.y + c.h);
                let inside = r.0 > x0 && r.1 > y0 && r.2 < x1 && r.3 < y1;
                let outside = r.2 < x0 || r.0 > x1 || r.3 < y0 || r.1 > y1;
                assert!(inside || outside, "label {t:?} on the border of {}", c.id);
            }
        }
        for (e, se) in g.edges.iter().zip(&sc.edges) {
            if !matches!(g.nodes[e.to].shape, Shape::Rect) || e.from == e.to {
                continue;
            }
            let end = se.waypoints.last().copied().unwrap_or(se.bezier[3]);
            let (x0, y0, x1, y1) = rect_of_node(&sc.nodes[e.to]);
            let on_x = ((end.0 - x0).abs() < 1.0 || (end.0 - x1).abs() < 1.0)
                && end.1 >= y0 - 1.0
                && end.1 <= y1 + 1.0;
            let on_y = ((end.1 - y0).abs() < 1.0 || (end.1 - y1).abs() < 1.0)
                && end.0 >= x0 - 1.0
                && end.0 <= x1 + 1.0;
            assert!(
                on_x || on_y,
                "edge {}->{} ends at {end:?}, off the border of {:?}",
                se.from,
                se.to,
                (x0, y0, x1, y1)
            );
        }
        let frame = |si: usize| -> Rect {
            let c = sc
                .clusters
                .iter()
                .find(|c| c.id == g.subgraphs[si].id)
                .unwrap();
            (c.x, c.y, c.x + c.w, c.y + c.h)
        };
        let paths = node_cluster_paths(&g);
        for (v, n) in sc.nodes.iter().enumerate() {
            let r = rect_of_node(n);
            for si in 0..g.subgraphs.len() {
                let f = frame(si);
                let inside = r.0 >= f.0 && r.1 >= f.1 && r.2 <= f.2 && r.3 <= f.3;
                assert_eq!(
                    inside,
                    paths[v].contains(&si),
                    "node {} vs frame {}",
                    n.id,
                    g.subgraphs[si].id
                );
            }
        }
        for a in 0..g.subgraphs.len() {
            for b in 0..a {
                let (fa, fb) = (frame(a), frame(b));
                let nests = |i: Rect, o: Rect| i.0 >= o.0 && i.1 >= o.1 && i.2 <= o.2 && i.3 <= o.3;
                assert!(
                    !overlaps(fa, fb) || nests(fa, fb) || nests(fb, fa),
                    "frames {} and {} cross",
                    g.subgraphs[a].id,
                    g.subgraphs[b].id
                );
            }
        }
        sc
    }

    #[test]
    fn edges_to_their_own_box_stay_off_its_members_in_every_direction() {
        // `C -->|..| G` from inside G used to curve across C in LR/RL,
        // its label on top of C.
        for dir in ["LR", "RL", "TB", "BT"] {
            let s = assert_clean_geometry(&format!(
                "flowchart {dir}\n\
                 subgraph G\n  A --> B --> C --> A\nend\n\
                 C -->|back to group| G\nG --> A"
            ));
            for e in s.edges.iter().filter(|e| e.from == "G" || e.to == "G") {
                let pts = sample_cubic(&e.bezier, 24);
                for n in &s.nodes {
                    let (x0, y0, x1, y1) = rect_of_node(n);
                    let inside = pts.iter().any(|p| {
                        p.0 > x0 + 1.0 && p.0 < x1 - 1.0 && p.1 > y0 + 1.0 && p.1 < y1 - 1.0
                    });
                    assert!(!inside, "{dir}: edge {}->{} crosses {}", e.from, e.to, n.id);
                }
            }
        }
    }

    #[test]
    fn lr_fan_out_labels_clear_their_ends_and_each_other() {
        // Labels wider than the layer gap used to sit on both ends.
        assert_clean_geometry(
            "flowchart LR\n\
             F[\"failover.sh\"]\n\
             F -->|\"1 stop tunnel + mirror\"| T[\"tunnel\"]\n\
             F -->|\"4 symlink + nginx -t + reload\"| N[\"nginx\"]\n\
             F -->|\"5 notif\"| X[\"telegram\"]",
        );
        assert_clean_geometry(
            "flowchart RL\nA -->|a rather long label here| B\nA -->|another long one| C",
        );
    }

    #[test]
    fn lr_colliding_labels_grow_their_gap_only_to_the_widest_one() {
        // Four labels share the gap after `laptop`, and their spans
        // collide across the flow. They slide apart across it, so the
        // gap needs room for the widest label — not for a row of
        // labels laid side by side along the flow (which once more
        // than doubled it, all of it empty).
        let sc = assert_clean_geometry(
            "flowchart LR\n\
             subgraph prod[\"PROD\"]\n  direction TB\n  sshd[\"sshd\"]\n  src[\"source\"]\nend\n\
             laptop([\"laptop\"])\n\
             subgraph standby[\"STANDBY\"]\n  direction TB\n  tunnel[\"tunnel\"]\n  static[\"static copy\"]\nend\n\
             tunnel ==>|\"ssh out of standby\"| sshd\n\
             laptop -->|\"read live state\"| src\n\
             laptop -->|\"one-off setup: role, hba, slot\"| sshd\n\
             laptop -->|\"write env, compose, cert\"| static",
        );
        let node = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
        let (laptop, sshd) = (node("laptop"), node("sshd"));
        let gap = (sshd.x - sshd.w / 2.0) - (laptop.x + laptop.w / 2.0);
        let widest = sc
            .edges
            .iter()
            .filter_map(|e| e.label.as_ref().map(|l| l.2))
            .fold(0.0, f64::max);
        assert!(gap >= widest, "gap {gap} cannot hold a {widest} label");
        assert!(gap <= widest + 2.0 * LABEL_BOX_H, "gap {gap} for {widest}");
    }

    #[test]
    fn labels_keep_off_frame_border_lines() {
        // The label of an edge from one frame into the next used to sit
        // mid-gap, astride both borders (and past the first's corner).
        assert_clean_geometry(
            "flowchart LR\n\
             subgraph prod[\"PROD\"]\n  direction TB\n  pg[(\"postgres primary\")]\n  minio[(\"minio\")]\nend\n\
             subgraph standby[\"STANDBY\"]\n  direction TB\n  rpg[(\"postgres replica\")]\n  rminio[(\"minio mirror\")]\nend\n\
             pg ==>|\"streaming replication async over tunnel :15432\"| rpg\n\
             minio -->|\"mc mirror over tunnel :19000\"| rminio",
        );
        assert_clean_geometry(
            "flowchart TD\n\
             subgraph top[\"Top\"]\n  a[\"alpha\"]\nend\n\
             subgraph low[\"Low\"]\n  b[\"beta\"]\nend\n\
             a -->|\"a label crossing from one box into the next\"| b",
        );
    }

    #[test]
    fn td_self_loop_labels_step_out_past_their_loop() {
        // A self-loop hugs the node's side: a label wider than the loop
        // found no clear spot on it and stayed over the node's text.
        for dir in ["TD", "BT", "LR", "RL"] {
            assert_clean_geometry(&format!(
                "flowchart {dir}\nA -->|go| B\nB -->|a self loop label, long| B"
            ));
        }
        assert_clean_geometry(
            "flowchart TD\nA -->|x| B\nB -->|y| A\nB -->|lbl 8 long text here| B",
        );
    }

    #[test]
    fn converging_labels_on_shared_targets_are_nudged_apart() {
        // Six labelled edges from one hub onto two shared stores: the
        // labels near the stores used to stack on one spot.
        let mut src = String::from("flowchart TD\nT[Traefik]\n");
        for s in ["general", "invoice", "vendor", "account", "mcp", "ocr"] {
            src.push_str(&format!(
                "T -->|{s}.core.example| {s}[core-{s}]\n\
                 {s} -->|via URL publik s3.core| S[(minio)]\n\
                 {s} -.->|via URL publik pg| P[(postgres)]\n"
            ));
        }
        assert_clean_geometry(&src);
    }

    #[test]
    fn clustered_labels_and_endpoints_stay_clean() {
        assert_clean_geometry(
            "flowchart LR\n\
             I([Internet])\n\
             subgraph dev[\"DEV\"]\n  a[\"alpha\"]\n  b[\"beta wide label here\"]\nend\n\
             subgraph prod[\"PROD\"]\n  c[\"gamma\"]\nend\n\
             I -->|https public ingress| a\n\
             I -->|ssh admin| b\n\
             a -->|replication over tunnel| c",
        );
        assert_clean_geometry(
            "flowchart TD\n\
             subgraph top[\"Top\"]\n  a[\"alpha\"] -->|first hop| b[\"beta\"]\nend\n\
             subgraph low[\"Low\"]\n  c[\"gamma\"]\n  d[\"delta\"]\nend\n\
             b -->|down into low| c\n\
             b -->|also down| d\n\
             a -->|skipping a layer| d",
        );
    }

    #[test]
    fn td_ranks_stay_vertical_and_fitting_labels_keep_their_spot() {
        let sc = assert_clean_geometry("flowchart TD\nA -->|yes| B -->|no| C");
        assert!(sc.nodes[0].y < sc.nodes[1].y && sc.nodes[1].y < sc.nodes[2].y);
        assert!((sc.nodes[0].x - sc.nodes[2].x).abs() < 1.0);
        // A label with room is left exactly at its curve's midpoint.
        for e in &sc.edges {
            let b = e.bezier;
            let (_, m, _) = e.label.as_ref().unwrap();
            assert_eq!(*m, cubic_mid(b[0], b[1], b[2], b[3]));
        }
    }

    #[test]
    fn lr_gap_grows_only_for_a_label_that_does_not_fit() {
        // A short label fits the default gap: layout is untouched.
        let short = scene(&parse("flowchart LR\nA -->|ok| B").unwrap());
        let plain = scene(&parse("flowchart LR\nA --> B").unwrap());
        assert_eq!(short.nodes[1].x, plain.nodes[1].x);
        // A long one pushes the next layer out until it clears both ends.
        let long = scene(&parse("flowchart LR\nA -->|a label far wider than the gap| B").unwrap());
        let (_, m, w) = long.edges[0].label.clone().unwrap();
        let (a, b) = (&long.nodes[0], &long.nodes[1]);
        assert!(m.0 - w / 2.0 >= a.x + a.w / 2.0);
        assert!(m.0 + w / 2.0 <= b.x - b.w / 2.0);
    }
}
