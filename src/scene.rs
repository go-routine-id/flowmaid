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
    intrinsic_size, layout_clustered_ranked, layout_sized, line_count, text_width, LayoutResult,
    Placed, RankGroup, LINE_H,
};
use crate::model::{Direction, Edge, EdgeEnds, EdgeKind, End, Graph, Marker, NodeStyle, Shape};
use std::collections::HashMap;

pub(crate) const MARGIN: f64 = 28.0;
/// Gap between parallel edges (connecting the same node pair).
const PARALLEL_GAP: f64 = 16.0;
/// Wanted spacing between edge ends that share a node side when one
/// of them carries a cross/circle or start glyph (the glyphs are about
/// 11-12px across).
const GLYPH_PORT_GAP: f64 = 14.0;

/// An edge in abstract layout coordinates: bezier points + routed
/// waypoints (empty = none) + optional (label centre, label box width).
type AbsEdge = ([(f64, f64); 4], Vec<(f64, f64)>, Option<((f64, f64), f64)>);
/// An edge end on a node side: (edge index, is source end, has glyph).
type Port = (usize, bool, bool);

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
fn edge_polyline(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 {
        return basis_sample(&e.waypoints);
    }
    sample_cubic(&e.bezier, 24)
}

/// Sample a cubic bezier `[p0, c1, c2, p3]` into `steps + 1` points.
fn sample_cubic(b: &[(f64, f64); 4], steps: usize) -> Vec<(f64, f64)> {
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
fn basis_sample(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
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

/// A cluster's title as drawn: whole, unless it runs past its own frame
/// INTO another frame (a sibling, not one it sits inside) — then cut
/// with an ellipsis to fit its frame, so neighbouring titles never
/// overprint each other. Measured at the 12px title font.
fn fit_cluster_title(clusters: &[SceneCluster], ci: usize) -> String {
    let c = &clusters[ci];
    let scale = 12.0 / 14.0;
    let chip = |w: f64| (c.x + TITLE_INSET, c.y, w + 8.0, TITLE_STRIP);
    let (x, y, w, h) = chip(text_width(&c.title) * scale);
    if x + w <= c.x + c.w - TITLE_INSET {
        return c.title.clone();
    }
    let within = |a: &SceneCluster, b: &SceneCluster| {
        a.x >= b.x && a.y >= b.y && a.x + a.w <= b.x + b.w && a.y + a.h <= b.y + b.h
    };
    let collides = clusters.iter().enumerate().any(|(oi, o)| {
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
    let mut cut: String = c.title.clone();
    while !cut.is_empty() && text_width(&format!("{cut}…")) * scale > room {
        cut.pop();
    }
    format!("{}…", cut.trim_end())
}

/// How far a routed edge's label may sit from the curve actually drawn
/// before it is moved onto it.
const LABEL_DRIFT_MAX: f64 = LINE_H;

/// Where a routed edge's label goes: its middle channel point — the
/// slot the layout reserved — unless the drawn spline, which only
/// approximates the points, bends more than [`LABEL_DRIFT_MAX`] away
/// from it there (a zig-zagging lane); then the nearest point of the
/// curve, so the label stays on its own edge.
fn channel_label_point(wps: &[(f64, f64)]) -> (f64, f64) {
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

/// Does segment `a`–`b` intersect the axis-aligned rectangle
/// `[rx0,rx1] × [ry0,ry1]`? Liang-Barsky clip (true when either
/// endpoint is inside, too).
fn seg_aabb(a: (f64, f64), b: (f64, f64), rx0: f64, ry0: f64, rx1: f64, ry1: f64) -> bool {
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
    if g.subgraphs.is_empty() {
        scene_flat(g, sizes)
    } else {
        scene_clustered(g, sizes)
    }
}

/// The classic single-level pipeline (no subgraphs).
fn scene_flat(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    scene_from_layout(g, sizes, layout_sized(g, sizes), None)
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
    spread_glyph_ports(g, &lo.nodes, &mut abs_edges);

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
/// A subgraph needs at least this many edge-less members before they
/// are packed into a grid instead of sharing one rank.
const GRID_MIN_MEMBERS: usize = 4;

/// Global cluster-aware pipeline: lay every real node out in ONE
/// Sugiyama pass (no supernode collapse) with subgraph members kept
/// contiguous, then wrap each subgraph around its final members. This
/// keeps cross-cluster edges ordered globally so they route through the
/// gaps between boxes instead of tangling straight through them.
fn scene_clustered(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    let node_cluster = node_cluster_paths(g);
    let grid = grid_pairs(g);
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
                            free_edge(&pa, &pb, sa, sb, false, 0.0)
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
                label,
            });
        }
    }

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
        b.clamp(b0 + SUB_PAD, (b1 - SUB_PAD).max(b0 + SUB_PAD))
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

/// Rank-only pairs that pack a subgraph's edge-less members into a
/// roughly square grid: with `k` such members, member `i` is ranked
/// above member `i + ceil(sqrt(k))`, so they fill rows of that width in
/// declaration order. Left alone they would all share one rank — a
/// list of 20 notes drawn as one 10k px row. Skipped when the subgraph
/// sets a `direction` across the diagram's flow (`direction LR` in a
/// TB chart): there one row is what the author asked for. Each pair is
/// a one-to-one [`RankGroup`].
fn grid_pairs(g: &Graph) -> Vec<RankGroup> {
    let horizontal = |d: Direction| matches!(d, Direction::LR | Direction::RL);
    let mut linked = vec![false; g.nodes.len()];
    for e in &g.edges {
        linked[e.from] = true;
        linked[e.to] = true;
    }
    for e in &g.sub_edges {
        for end in [e.from, e.to] {
            if let End::Node(v) = end {
                linked[v] = true;
            }
        }
    }
    let mut out = Vec::new();
    for s in &g.subgraphs {
        if s.direction
            .is_some_and(|d| horizontal(d) != horizontal(g.direction))
        {
            continue;
        }
        let free: Vec<usize> = s.nodes.iter().copied().filter(|&v| !linked[v]).collect();
        if free.len() < GRID_MIN_MEMBERS {
            continue;
        }
        let cols = (free.len() as f64).sqrt().ceil() as usize;
        for i in cols..free.len() {
            out.push((vec![free[i - cols]], vec![free[i]]));
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
        );
        let wps = if e.from != e.to && top_of(e.from) != top_of(e.to) {
            cross_cluster_route(
                centers[e.from],
                sizes[e.from],
                box_of(e.from),
                centers[e.to],
                sizes[e.to],
                box_of(e.to),
                vertical,
            )
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
        for e in &g.sub_edges {
            let (Some((pa, sa)), Some((pb, sb))) = (end_placed(e.from), end_placed(e.to)) else {
                continue;
            };
            let pts = free_edge(&pa, &pb, sa, sb, false, 0.0);
            let label = e.label.as_ref().map(|l| {
                (
                    l.clone(),
                    cubic_mid(pts[0], pts[1], pts[2], pts[3]),
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
                label,
            });
        }
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

/// Re-route after a drag LOCALLY: an edge whose BOTH endpoints still
/// sit at their auto-layout position keeps its `base` geometry from
/// [`scene`] verbatim (channel waypoints, spread anchors, label slot);
/// only edges touching a moved node are re-routed freely, and cluster
/// boxes are reused wholesale while nothing has moved. Dragging one
/// node thus stays a local operation instead of degrading the whole
/// diagram to free-route quality. Falls back to [`route`] whenever
/// `base` no longer lines up with the graph (e.g. after an edit).
pub fn route_partial(
    g: &Graph,
    centers: &[(f64, f64)],
    base: &Scene,
    base_centers: &[(f64, f64)],
) -> Scene {
    let n = g.nodes.len();
    if base.nodes.len() != n || base_centers.len() != n || base.edges.len() < g.edges.len() {
        return route(g, centers);
    }
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    let moved: Vec<bool> = (0..n)
        .map(|i| {
            (centers[i].0 - base_centers[i].0).abs() > 0.01
                || (centers[i].1 - base_centers[i].1).abs() > 0.01
        })
        .collect();
    if !moved.iter().any(|&m| m) {
        return base.clone();
    }

    let full = route_sized(g, centers, &sizes);
    let mut out = full;
    for ei in 0..g.edges.len() {
        let e = &g.edges[ei];
        if !moved[e.from] && !moved[e.to] {
            out.edges[ei] = base.edges[ei].clone();
        }
    }
    // The base canvas already covers the preserved geometry; grow it
    // for whatever the drag pushed outward.
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &out.nodes, &out.edges, &out.clusters);
    let (_, maxx, _, maxy) = bb.finish();
    out.width = (maxx + MARGIN).max(base.width);
    out.height = (maxy + MARGIN).max(base.height);
    out
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
            miny - SUB_PAD - SUB_HEADER,
            (maxx - minx) + 2.0 * SUB_PAD,
            (maxy - miny) + 2.0 * SUB_PAD + SUB_HEADER,
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
    )
}

/// `marker-start` / `marker-end` attribute for one edge end (empty
/// when the end has no glyph). The end arrow keeps its historical
/// `#arrow` id so marker-free output is unchanged.
fn marker_attr(side: &str, m: Option<Marker>) -> String {
    let id = match (side, m) {
        (_, None) => return String::new(),
        ("end", Some(Marker::Arrow)) => "arrow",
        (_, Some(Marker::Arrow)) => "arrow-start",
        ("end", Some(Marker::Cross)) => "cross-end",
        (_, Some(Marker::Cross)) => "cross-start",
        ("end", Some(Marker::Circle)) => "circle-end",
        (_, Some(Marker::Circle)) => "circle-start",
    };
    format!(" marker-{side}=\"url(#{id})\"")
}

/// `<marker>` defs beyond the classic end arrow, emitted only for the
/// glyphs some drawn edge actually uses — a diagram without `<-->`,
/// `--x` or `--o` gets exactly the defs it always had. Cross and
/// circle are sized in user space (mermaid-like, independent of the
/// stroke width) and sit just outside the node border: the path still
/// ends ON the border, the glyph is offset back along the tangent.
fn extra_marker_defs(edges: &[SceneEdge]) -> String {
    let uses = |start: bool, m: Marker| {
        edges.iter().any(|e| {
            let got = if start {
                e.ends.start_marker(e.kind)
            } else {
                e.ends.end_marker(e.kind)
            };
            got == Some(m)
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

    let mut s = String::new();
    svg_open(&mut s, width, height, 14, title, opts);
    s.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"{}\"/></marker>{}</defs>\n",
        EDGE_COLOR,
        extra_marker_defs(&sc.edges)
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
        let title = fit_cluster_title(&sc.clusters, ci);
        let tw = text_width(&title) * 12.0 / 14.0; // title font is 12px
        cluster_titles.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"16\" rx=\"4\" \
             fill=\"#f7f8fd\" fill-opacity=\"0.9\"/>\n",
            x + 6.0,
            y + 5.0,
            tw + 8.0
        ));
        cluster_titles.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" font-weight=\"bold\" \
             fill=\"#6a7086\">{}</text>\n",
            x + 10.0,
            y + 17.0,
            escape(&title)
        ));
    }

    let mut edge_labels = String::new();
    for e in &sc.edges {
        if matches!(e.kind, EdgeKind::Invisible) {
            continue; // layout-only link — never drawn
        }
        let (dash, sw) = match e.kind {
            EdgeKind::Dotted | EdgeKind::DottedOpen => (" stroke-dasharray=\"5 4\"", 1.7),
            EdgeKind::Thick | EdgeKind::ThickOpen => ("", 3.4),
            _ => ("", 1.7),
        };
        let marker = format!(
            "{}{}",
            marker_attr("start", e.ends.start_marker(e.kind)),
            marker_attr("end", e.ends.end_marker(e.kind))
        );
        // A routed edge splines through its waypoints; otherwise the
        // single cubic. The arrow marker orients to the end tangent
        // (last segment) either way.
        let path_d = if e.waypoints.len() >= 2 {
            let q: Vec<(f64, f64)> = e.waypoints.iter().map(|&p| t(p)).collect();
            spline_d(&q)
        } else {
            let q: Vec<(f64, f64)> = e.bezier.iter().map(|&p| t(p)).collect();
            format!(
                "M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
                q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1, q[3].0, q[3].1
            )
        };
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
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}>\
             <title>{}</title></path>\n",
            path_d,
            EDGE_COLOR,
            sw,
            dash,
            marker,
            escape(&edge_title)
        ));
        if let Some((text, m, w)) = &e.label {
            svg_label_box(&mut edge_labels, text, t(*m), *w);
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
        if matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        for &(x, y) in &e.bezier {
            bb.add(x, y);
        }
        if let Some((_, m, w)) = &e.label {
            bb.add(m.0 - w / 2.0, m.1 - 10.0);
            bb.add(m.0 + w / 2.0, m.1 + 10.0);
        }
    }
}

/// Spread apart the edge ends that share one top/bottom side of a box
/// node when any of them carries a glyph beyond the plain arrowhead
/// (a start marker, a cross or a circle). [`anchor`] keeps fan-out
/// ports within a few px of each other, which plain lines tolerate but
/// which piles the wider glyphs into one blob. Sides without such a
/// glyph are left untouched, so marker-free diagrams render as before.
/// Abstract coordinates; endpoints move along the breadth axis only,
/// each with its neighbouring control point so the curve still leaves
/// the border square.
fn spread_glyph_ports(g: &Graph, placed: &[Placed], edges: &mut [AbsEdge]) {
    // (node, bottom side) -> the edge ends on it.
    let mut sides: HashMap<(usize, bool), Vec<Port>> = HashMap::new();
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let (pts, wps, _) = &edges[ei];
        for (src, n) in [(true, e.from), (false, e.to)] {
            if matches!(
                g.nodes[n].shape,
                Shape::Diamond
                    | Shape::Circle
                    | Shape::DoubleCircle
                    | Shape::StateStart
                    | Shape::StateEnd
            ) {
                continue;
            }
            let pt = match (src, wps.is_empty()) {
                (true, true) => pts[0],
                (false, true) => pts[3],
                (true, false) => wps[0],
                (false, false) => wps[wps.len() - 1],
            };
            let p = &placed[n];
            let bottom = if (pt.1 - (p.l + p.lsize / 2.0)).abs() < 0.5 {
                true
            } else if (pt.1 - (p.l - p.lsize / 2.0)).abs() < 0.5 {
                false
            } else {
                continue; // a side port (same-layer edge)
            };
            let glyph = if src {
                e.ends.start_marker(e.kind).is_some()
            } else {
                matches!(
                    e.ends.end_marker(e.kind),
                    Some(Marker::Cross | Marker::Circle)
                )
            };
            sides.entry((n, bottom)).or_default().push((ei, src, glyph));
        }
    }
    // (endpoint index, neighbour index) into the drawn point list.
    let ends_of = |pts: &[(f64, f64); 4], wps: &Vec<(f64, f64)>, src: bool| {
        let (pt, nb) = match (src, wps.is_empty()) {
            (true, true) => (pts[0], pts[1]),
            (false, true) => (pts[3], pts[2]),
            (true, false) => (wps[0], wps[1]),
            (false, false) => (wps[wps.len() - 1], wps[wps.len() - 2]),
        };
        (pt.0, nb.0)
    };
    let mut keys: Vec<_> = sides.keys().copied().collect();
    keys.sort_unstable(); // deterministic output
    for key in keys {
        let mut ports = sides.remove(&key).unwrap_or_default();
        if ports.len() < 2 || !ports.iter().any(|&(.., glyph)| glyph) {
            continue;
        }
        // Order by where each edge heads next (then by where it sits
        // now), so the spread ends don't make their curves cross.
        let key_of = |&(ei, src, _): &Port| {
            let (at, nb) = ends_of(&edges[ei].0, &edges[ei].1, src);
            (nb, at)
        };
        ports.sort_by(|a, b| {
            key_of(a)
                .partial_cmp(&key_of(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let at: Vec<f64> = ports
            .iter()
            .map(|&(ei, src, _)| ends_of(&edges[ei].0, &edges[ei].1, src).0)
            .collect();
        if at.windows(2).all(|w| w[1] - w[0] >= GLYPH_PORT_GAP) {
            continue;
        }
        let p = &placed[key.0];
        let half = match g.nodes[key.0].shape {
            Shape::Stadium => p.bsize / 2.0 - p.lsize / 2.0,
            _ => p.bsize / 2.0 - 4.0,
        }
        .max(0.0);
        let n = at.len() as f64;
        let step = GLYPH_PORT_GAP.min(2.0 * half / (n - 1.0));
        let run = step * (n - 1.0);
        let mean = at.iter().sum::<f64>() / n;
        let first = (mean - run / 2.0).clamp(p.b - half, p.b + half - run);
        for (i, &(ei, src, _)) in ports.iter().enumerate() {
            let d = first + step * i as f64 - at[i];
            let (pts, wps, label) = &mut edges[ei];
            let (pi, ni) = if src { (0, 1) } else { (3, 2) };
            pts[pi].0 += d;
            pts[ni].0 += d;
            if wps.is_empty() {
                if let Some((m, _)) = label {
                    *m = cubic_mid(pts[0], pts[1], pts[2], pts[3]);
                }
            } else {
                let wi = if src { 0 } else { wps.len() - 1 };
                wps[wi].0 += d;
            }
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

fn free_edge(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    self_loop: bool,
    off: f64,
) -> [(f64, f64); 4] {
    if self_loop {
        return loop_points(a, off);
    }
    let dx = b.b - a.b;
    let dy = b.l - a.l;
    if dy.abs() >= dx.abs() {
        // Vertically dominant: exit bottom/top, enter top/bottom.
        let down = dy >= 0.0;
        let p0 = anchor(a, sa, (b.b, b.l), off, down);
        let p3 = anchor(b, sb, (a.b, a.l), off, !down);
        let s = if down { 1.0 } else { -1.0 };
        let dl = (dy.abs() * 0.45).max(24.0);
        [p0, (p0.0, p0.1 + s * dl), (p3.0, p3.1 - s * dl), p3]
    } else {
        // Horizontally dominant: border to border.
        let p0 = border(a, sa, (b.b, b.l + off * 4.0));
        let p3 = border(b, sb, (a.b, a.l + off * 4.0));
        let s = if dx >= 0.0 { 1.0 } else { -1.0 };
        let dl = (dx.abs() * 0.45).max(24.0);
        [p0, (p0.0 + s * dl, p0.1), (p3.0 - s * dl, p3.1), p3]
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

/// Opening `<svg>` tag + white background, shared by every SVG
/// writer. `title` makes the diagram screen-reader friendly (issue
/// #16): it becomes the root `<title>`, the `aria-label`, and the
/// element gets `role="img"`.
/// Viewport / responsive rendering options for [`crate::render_svg_advanced`].
///
/// Defaults reproduce the historical fixed-pixel output byte-for-byte, so
/// existing callers (and `render_svg`) are unaffected. More options
/// (theme, scale, max width, …) can be added here without new entry
/// points.
#[derive(Debug, Clone, Copy, Default)]
pub struct SvgOptions {
    /// When true, the root `<svg>` fills its container (`width="100%"`)
    /// instead of a fixed pixel size; the `viewBox` stays so the content
    /// scales with it.
    pub responsive: bool,
    /// Overrides the SVG `preserveAspectRatio` attribute. `None` keeps the
    /// SVG default `xMidYMid meet` (uniform scale, letterboxed).
    pub preserve_aspect_ratio: Option<&'static str>,
}

pub(crate) fn svg_open(
    s: &mut String,
    width: f64,
    height: f64,
    font_size: u32,
    title: &str,
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
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         viewBox=\"0 0 {vw:.0} {vh:.0}\"{par} font-family=\"Helvetica, Arial, sans-serif\" \
         font-size=\"{fs}\" role=\"img\" aria-label=\"{t}\">\n",
        w = w_attr,
        h = h_attr,
        vw = width,
        vh = height,
        fs = font_size,
        t = escape(title)
    ));
    s.push_str(&format!("<title>{}</title>\n", escape(title)));
    s.push_str(&format!(
        "<rect width=\"{:.0}\" height=\"{:.0}\" fill=\"#ffffff\"/>\n",
        width, height
    ));
}

/// White label box with centred text — used for flowchart edge
/// labels and ER relationship labels alike. `center` is in final
/// (already translated) coordinates.
pub(crate) fn svg_label_box(s: &mut String, text: &str, center: (f64, f64), box_w: f64) {
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
        TEXT_COLOR,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

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
            },
        );
        assert!(par.contains("preserveAspectRatio=\"none\""), "{par}");
        assert!(
            !resp.contains("preserveAspectRatio"),
            "unset -> default meet, no attribute"
        );
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
        // Plain fan-out keeps its tight ports (output unchanged).
        let plain = starts("flowchart LR\nA --> B\nA --> C\nA --> D\n");
        assert!(
            gaps(&plain).iter().all(|&d| d < GLYPH_PORT_GAP),
            "{plain:?}"
        );
        // Ends stay on the node's border.
        let s = scene(&parse("flowchart LR\nA <--> B\nA x--x C\nA o--o D\n").unwrap());
        let a = &s.nodes[0];
        for e in &s.edges {
            let p = e.bezier[0];
            assert!((p.1 - a.y).abs() <= a.h / 2.0, "{p:?}");
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
        let src =
            "flowchart TD\nsubgraph S1\n  A\nend\nsubgraph S2\n  B\n  C\nend\nA-->B\nA-->C\nB-->C";
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
        dragged[ic].0 += 150.0;
        dragged[ic].1 += 40.0;
        let s1 = route_partial(&g, &dragged, &s0, &auto);
        assert_eq!(s1.edges[0].bezier, s0.edges[0].bezier, "A->B unmoved");
        assert_eq!(s1.edges[0].waypoints, s0.edges[0].waypoints);
        assert_ne!(s1.edges[1].bezier, s0.edges[1].bezier, "A->C re-routed");
        // Mismatched base (edited graph) falls back to full route().
        let g2 = parse("flowchart TD\nA-->B").unwrap();
        let p2: Vec<(f64, f64)> = vec![(40.0, 40.0), (40.0, 160.0)];
        let fb = route_partial(&g2, &p2, &s0, &auto);
        assert_eq!(fb.nodes.len(), 2);
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
        let ((lx, _), lw) = back.label.as_ref().map(|l| (l.1, l.2)).unwrap();
        assert!(
            lx + lw / 2.0 < w.x || lx - lw / 2.0 > w.x + w.w,
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
        let cut = fit_cluster_title(&s.clusters, left);
        assert!(cut.ends_with('…'), "left title cut: {cut}");
        assert!(
            text_width(&cut) * 12.0 / 14.0 + 8.0 <= s.clusters[left].w - 2.0 * TITLE_INSET + 0.5
        );
        // The rightmost one overflows into nothing: kept whole.
        assert_eq!(fit_cluster_title(&s.clusters, right), long);
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
}
