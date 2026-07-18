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

use crate::layout::{intrinsic_size, layout_clustered, layout_sized, text_width, LayoutResult, Placed};
use crate::model::{Direction, EdgeKind, End, Graph, NodeStyle, Shape};
use std::collections::HashMap;

pub(crate) const MARGIN: f64 = 28.0;
/// Gap between parallel edges (connecting the same node pair).
const PARALLEL_GAP: f64 = 16.0;

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
        self.nodes
            .iter()
            .rposition(|n| node_contains(n, x, y))
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
            if d <= tol && best.map_or(true, |(_, bd)| d < bd) {
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
            if inside && best.map_or(true, |(_, d)| c.depth >= d) {
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
            if overlaps(n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0) {
                out.push(Hit::Node(i));
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            if matches!(e.kind, EdgeKind::Invisible) {
                continue;
            }
            if edge_polyline(e).iter().any(|&(px, py)| {
                px >= rx0 && px <= rx1 && py >= ry0 && py <= ry1
            }) {
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
        // Ellipse (circle when hw == hh).
        Shape::Circle | Shape::DoubleCircle => {
            let (px, py) = (dx / hw, dy / hh);
            px * px + py * py <= 1.0
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

/// The polyline an edge is drawn as: its routed waypoints, else a
/// sampling of the cubic bezier (matches how it renders).
fn edge_polyline(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 {
        return e.waypoints.clone();
    }
    let [p0, c1, c2, p3] = e.bezier;
    (0..=16)
        .map(|i| {
            let t = i as f64 / 16.0;
            let u = 1.0 - t;
            (
                u * u * u * p0.0 + 3.0 * u * u * t * c1.0 + 3.0 * u * t * t * c2.0 + t * t * t * p3.0,
                u * u * u * p0.1 + 3.0 * u * u * t * c1.1 + 3.0 * u * t * t * c2.1 + t * t * t * p3.1,
            )
        })
        .collect()
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
    scene_from_layout(g, sizes, layout_sized(g, sizes))
}

/// Turn an abstract [`LayoutResult`] into a positioned [`Scene`]:
/// edge geometry, canvas bbox, and the direction transform. Shared by
/// the flat and clustered pipelines so both route edges identically.
fn scene_from_layout(g: &Graph, sizes: &[(f64, f64)], lo: LayoutResult) -> Scene {
    // Breadth extents per layer, used by back-edges to route
    // themselves around every node on the layers they pass.
    let nlayers = lo.nodes.iter().map(|p| p.layer).max().map_or(0, |m| m + 1);
    let mut lay_ext = vec![(f64::INFINITY, f64::NEG_INFINITY); nlayers];
    for p in &lo.nodes {
        let e = &mut lay_ext[p.layer];
        e.0 = e.0.min(p.b - p.bsize / 2.0);
        e.1 = e.1.max(p.b + p.bsize / 2.0);
    }

    // Edge geometry (abstract coordinates) with parallel-edge separation.
    let offs = parallel_offsets(g);
    /// Bezier points + routed waypoints (empty = none) + optional
    /// (label centre, label box width).
    type AbsEdge = ([(f64, f64); 4], Vec<(f64, f64)>, Option<((f64, f64), f64)>);
    let mut abs_edges: Vec<AbsEdge> = Vec::with_capacity(g.edges.len());
    for (ei, (e, off)) in g.edges.iter().zip(offs).enumerate() {
        let a = &lo.nodes[e.from];
        let b = &lo.nodes[e.to];
        // Lebar box label (0 bila tanpa label) — busur back-edge
        // melebar sebesar ini supaya label di apex tidak menabrak node.
        let lbl_w = e.label.as_ref().map_or(0.0, |l| text_width(l) + 14.0);
        let pts = edge_points(
            a,
            b,
            g.nodes[e.from].shape,
            g.nodes[e.to].shape,
            e.from == e.to,
            lo.total_b,
            off,
            &lay_ext,
            lbl_w,
        );
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
            let p3 = anchor(b, g.nodes[e.to].shape, *chain.last().unwrap(), off, !a_bottom);
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
                wps[wps.len() / 2]
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
            label: e
                .label
                .clone()
                .zip(label)
                .map(|(t, (m, w))| (t, tf(m), w)),
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

/// Global cluster-aware pipeline: lay every real node out in ONE
/// Sugiyama pass (no supernode collapse) with subgraph members kept
/// contiguous, then wrap each subgraph around its final members. This
/// keeps cross-cluster edges ordered globally so they route through the
/// gaps between boxes instead of tangling straight through them.
fn scene_clustered(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    let node_cluster = node_cluster_paths(g);
    let lo = layout_clustered(g, sizes, &node_cluster);
    let mut sc = scene_from_layout(g, sizes, lo);

    // Wrap boxes around the laid-out members, then widen each box to
    // also cover the channel lanes of its members' edges: a lane that
    // enters the box from the top may sit beside the outermost member
    // node (between it and the invisible border wall), which the
    // node-only bbox would leave poking outside the drawn rectangle.
    sc.clusters = {
        let (mut boxes, depth) = cluster_raw_boxes(g, &sc.nodes);
        for si in 0..g.subgraphs.len() {
            let Some((x, y, w, h)) = boxes[si] else { continue };
            let (mut x0, mut x1) = (x, x + w);
            for (ge, se) in g.edges.iter().zip(sc.edges.iter()) {
                if se.waypoints.is_empty()
                    || !(node_cluster[ge.from].contains(&si)
                        || node_cluster[ge.to].contains(&si))
                {
                    continue;
                }
                for &(wx, wy) in &se.waypoints {
                    if wy >= y && wy <= y + h {
                        x0 = x0.min(wx - 12.0);
                        x1 = x1.max(wx + 12.0);
                    }
                }
            }
            boxes[si] = Some((x0, y, x1 - x0, h));
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
        out
    };

    // Edges that touch a whole subgraph box (state-diagram composites,
    // `A --> subgraph`) route against the cluster rectangle rather than
    // a node — the node-to-node pass in `scene_from_layout` skips them.
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
            sc.edges.push(SceneEdge {
                from: end_id(e.from),
                to: end_id(e.to),
                bezier: pts,
                waypoints: Vec::new(),
                kind: e.kind,
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
        if horizontal { (extent - p.0, p.1) } else { (p.0, extent - p.1) }
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
fn cluster_raw_boxes(
    g: &Graph,
    nodes: &[SceneNode],
) -> (Vec<RawBox>, Vec<usize>) {
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

/// Serialise any Scene (automatic or dragged) to SVG. Content is
/// translated to start at MARGIN, so negative coordinates are safe.
/// The accessible name is `"Flowchart diagram"`; use [`to_svg_titled`]
/// for another diagram class (e.g. state machines).
pub fn to_svg(sc: &Scene) -> String {
    to_svg_titled(sc, "Flowchart diagram")
}

/// [`to_svg`] with a caller-supplied accessible name — the root
/// `<title>` / `aria-label`. Callers that know the document type (a
/// `stateDiagram-v2` rides the flowchart `Scene`) pass the right one
/// so screen readers and hover tooltips announce it correctly (#16).
pub fn to_svg_titled(sc: &Scene, title: &str) -> String {
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
    svg_open(&mut s, width, height, 14, title);
    s.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"{}\"/></marker></defs>\n",
        EDGE_COLOR
    ));

    // Cluster boxes first (outermost-first), behind everything. The
    // TITLES are deferred to the very end (`cluster_titles`) so edges
    // passing the header strip never strike through the text — each
    // gets a translucent chip, mermaid-style.
    let mut cluster_titles = String::new();
    for c in &sc.clusters {
        let (x, y) = t((c.x, c.y));
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\" \
             fill=\"#f7f8fd\" stroke=\"#c9cfe8\" stroke-width=\"1.4\"/>\n",
            x, y, c.w, c.h
        ));
        let tw = text_width(&c.title) * 12.0 / 14.0; // title font is 12px
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
            escape(&c.title)
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
        let marker = if e.kind.has_arrow() {
            " marker-end=\"url(#arrow)\""
        } else {
            ""
        };
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
        let edge_title = match &e.label {
            Some((text, ..)) => {
                format!("{} \u{2192} {}: {}", display(&e.from), display(&e.to), plain_text(text))
            }
            None => format!("{} \u{2192} {}", display(&e.from), display(&e.to)),
        };
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}>\
             <title>{}</title></path>\n",
            path_d, EDGE_COLOR, sw, dash, marker, escape(&edge_title)
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
            n.style.fill.as_deref().unwrap_or(ss.fill),
            n.style.stroke.as_deref().unwrap_or(ss.stroke),
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
                    l = l, r = r, ty = t + ry, by = b - ry, rx = w / 2.0, ry = ry,
                    stroke = n.style.stroke.as_deref().unwrap_or(ss.stroke),
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
                    stroke = n.style.stroke.as_deref().unwrap_or(ss.stroke), style = style,
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
                    format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", l + k, t, r, t, r - k, b, l, b)
                } else {
                    // \  \
                    format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", l, t, r - k, t, r, b, l + k, b)
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
                    stroke = n.style.stroke.as_deref().unwrap_or(ss.stroke),
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
            n.style.color.as_deref().unwrap_or(TEXT_COLOR),
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

pub(crate) fn grow_scene(bb: &mut Bbox, nodes: &[SceneNode], edges: &[SceneEdge], clusters: &[SceneCluster]) {
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
pub(crate) fn anchor(p: &Placed, shape: Shape, other: (f64, f64), off: f64, bottom: bool) -> (f64, f64) {
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
        let bt = (if (bt_r - mid) <= (mid - bt_l) { bt_r } else { bt_l }) + off;
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
        let a_edge = if down { a_c.1 + a_sz.1 / 2.0 } else { a_c.1 - a_sz.1 / 2.0 };
        let a_out = if down { a_box.1 + a_box.3 + GAP } else { a_box.1 - GAP };
        let b_in = if down { b_box.1 - GAP } else { b_box.1 + b_box.3 + GAP };
        let b_edge = if down { b_c.1 - b_sz.1 / 2.0 } else { b_c.1 + b_sz.1 / 2.0 };
        vec![(a_c.0, a_edge), (a_c.0, a_out), (b_c.0, b_in), (b_c.0, b_edge)]
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
        let a_edge = if right { a_c.0 + a_sz.0 / 2.0 } else { a_c.0 - a_sz.0 / 2.0 };
        let a_out = if right { a_box.0 + a_box.2 + GAP } else { a_box.0 - GAP };
        let b_in = if right { b_box.0 - GAP } else { b_box.0 + b_box.2 + GAP };
        let b_edge = if right { b_c.0 - b_sz.0 / 2.0 } else { b_c.0 + b_sz.0 / 2.0 };
        vec![(a_edge, a_c.1), (a_out, a_c.1), (b_in, b_c.1), (b_edge, b_c.1)]
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
        [
            p0,
            (p0.0, p0.1 + s * dl),
            (p3.0, p3.1 - s * dl),
            p3,
        ]
    } else {
        // Horizontally dominant: border to border.
        let p0 = border(a, sa, (b.b, b.l + off * 4.0));
        let p3 = border(b, sb, (a.b, a.l + off * 4.0));
        let s = if dx >= 0.0 { 1.0 } else { -1.0 };
        let dl = (dx.abs() * 0.45).max(24.0);
        [
            p0,
            (p0.0 + s * dl, p0.1),
            (p3.0 - s * dl, p3.1),
            p3,
        ]
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
            let tx = if dx.abs() > 1e-6 { hw / dx.abs() } else { f64::INFINITY };
            let ty = if dy.abs() > 1e-6 { hh / dy.abs() } else { f64::INFINITY };
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
pub(crate) fn svg_open(s: &mut String, width: f64, height: f64, font_size: u32, title: &str) {
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"Helvetica, Arial, sans-serif\" \
         font-size=\"{fs}\" role=\"img\" aria-label=\"{t}\">\n",
        w = width,
        h = height,
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
            if i == 0 { "0.33em".to_string() } else { format!("{lh:.1}") },
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

pub(crate) fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
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
            let corner = super::node_contains(
                n,
                n.x + n.w / 2.0 - 0.5,
                n.y + n.h / 2.0 - 0.5,
            );
            assert_eq!(corner, inside_corner, "{id} corner should be outside its shape");
        }
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
        let inv = s.edges.iter().position(|e| matches!(e.kind, EdgeKind::Invisible));
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
        let svg = crate::render_svg(
            "flowchart TD\nA[\"<b>Start</b> here\"] -->|go| B",
        )
        .unwrap();
        assert!(svg.contains("role=\"img\""));
        assert!(svg.contains("aria-label=\"Flowchart diagram\""));
        assert!(svg.contains("<title>Flowchart diagram</title>"));
        assert!(svg.contains("<title>Start here</title>"), "node title, tags stripped");
        assert!(svg.contains("<title>A \u{2192} B: go</title>"), "edge title with label");
        // State diagrams announce themselves correctly (issue #16 fix):
        // right accessible name, no synthesized `__start_*`/`__end_*`
        // ids in tooltips, no empty <title> for pseudostate nodes.
        let st = crate::render_svg("stateDiagram-v2\n[*] --> Idle\nIdle --> [*]").unwrap();
        assert!(st.contains("aria-label=\"State diagram\""), "state aria-label");
        assert!(st.contains("<title>State diagram</title>"), "state root title");
        assert!(!st.contains("__start_"), "no synthesized start id leaked");
        assert!(!st.contains("__end_"), "no synthesized end id leaked");
        assert!(st.contains("start \u{2192} Idle"), "pseudostate named 'start'");
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
            assert_eq!(crate::render_svg(src).unwrap(), crate::render_svg(src).unwrap());
        }
    }

    #[test]
    fn scene_carries_stable_identity_and_stays_index_parallel() {
        // Issue #13 contract: nodes are index-parallel with the graph
        // AND carry ids; edges carry from/to ids; sub-edges use the
        // subgraph id; clusters carry their subgraph id.
        let g = parse(
            "flowchart TD\nsubgraph grp [Group]\n  B\nend\nA --> B\nA --> grp",
        )
        .unwrap();
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
        let src = "flowchart TD\nsubgraph S1\n  A\nend\nsubgraph S2\n  B\n  C\nend\nA-->B\nA-->C\nB-->C";
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
        assert!(lp[1].0 > a.x + a.w / 2.0, "loop must extend right of the box");
    }

    fn cluster_contains(c: &SceneCluster, n: &SceneNode) -> bool {
        n.x - n.w / 2.0 >= c.x
            && n.y - n.h / 2.0 >= c.y
            && n.x + n.w / 2.0 <= c.x + c.w
            && n.y + n.h / 2.0 <= c.y + c.h
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
        assert!(cluster_contains(inner, &s1.nodes[w2]), "cluster must follow the drag");
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
}
