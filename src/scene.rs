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

const MARGIN: f64 = 28.0;
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
#[derive(Debug, Clone)]
pub struct SceneNode {
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
#[derive(Debug, Clone)]
pub struct SceneEdge {
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
pub struct SceneCluster {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub title: String,
    /// Nesting depth, 0 = outermost.
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub edges: Vec<SceneEdge>,
    pub clusters: Vec<SceneCluster>,
    pub width: f64,
    pub height: f64,
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
        let wps: Vec<(f64, f64)> = if lo.edge_paths[ei].is_empty() {
            Vec::new()
        } else {
            let mut v = Vec::with_capacity(lo.edge_paths[ei].len() + 2);
            v.push(pts[0]);
            v.extend(lo.edge_paths[ei].iter().copied());
            v.push(pts[3]);
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
fn shift_scene(sc: &mut Scene, dx: f64, dy: f64) {
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

/// Whether a subgraph has any member node, directly or through a
/// nested child. Member-less subgraphs have no geometric anchor and
/// are dropped from both `scene` and `route` for consistency.

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
pub fn to_svg(sc: &Scene) -> String {
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    let (minx, maxx, miny, maxy) = bb.finish();
    let tx = MARGIN - minx;
    let ty = MARGIN - miny;
    let width = (maxx - minx) + 2.0 * MARGIN;
    let height = (maxy - miny) + 2.0 * MARGIN;
    let t = |p: (f64, f64)| (p.0 + tx, p.1 + ty);

    let mut s = String::new();
    svg_open(&mut s, width, height, 14);
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
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>\n",
            path_d, EDGE_COLOR, sw, dash, marker
        ));
        if let Some((text, m, w)) = &e.label {
            svg_label_box(&mut edge_labels, text, t(*m), *w);
        }
    }

    for n in &sc.nodes {
        let (cx, cy) = t((n.x, n.y));
        let (w, h) = (n.w, n.h);
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
struct Bbox(f64, f64, f64, f64);
impl Bbox {
    fn new() -> Self {
        Bbox(
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        )
    }
    fn add(&mut self, x: f64, y: f64) {
        self.0 = self.0.min(x);
        self.1 = self.1.max(x);
        self.2 = self.2.min(y);
        self.3 = self.3.max(y);
    }
    fn finish(&self) -> (f64, f64, f64, f64) {
        if self.0.is_finite() {
            (self.0, self.1, self.2, self.3)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    }
}

fn grow_scene(bb: &mut Bbox, nodes: &[SceneNode], edges: &[SceneEdge], clusters: &[SceneCluster]) {
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
fn parallel_offsets(g: &Graph) -> Vec<f64> {
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
fn anchor(p: &Placed, shape: Shape, other: (f64, f64), off: f64, bottom: bool) -> (f64, f64) {
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

/// Opening `<svg>` tag + white background, shared by every SVG writer.
pub(crate) fn svg_open(s: &mut String, width: f64, height: f64, font_size: u32) {
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"Helvetica, Arial, sans-serif\" \
         font-size=\"{fs}\">\n",
        w = width,
        h = height,
        fs = font_size
    ));
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
        escape(text)
    ));
}

/// Centred `<text>` supporting `\n` line breaks — one `<tspan>`
/// per line, block vertically centred on `(cx, cy)`.
pub(crate) fn svg_text_multiline(s: &mut String, cx: f64, cy: f64, fill: &str, label: &str) {
    let lines: Vec<&str> = label.split('\n').collect();
    if lines.len() <= 1 {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             fill=\"{}\">{}</text>\n",
            cx,
            cy,
            fill,
            escape(label)
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
            escape(line)
        ));
    }
    s.push_str("</text>\n");
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
