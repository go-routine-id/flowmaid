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

use crate::layout::{intrinsic_size, layout_sized, text_width, Placed};
use crate::model::{Direction, EdgeKind, Graph, NodeStyle, Shape};
use std::collections::HashMap;

const MARGIN: f64 = 28.0;
/// Gap between parallel edges (connecting the same node pair).
const PARALLEL_GAP: f64 = 16.0;

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
    /// Bezier points: start, control1, control2, end.
    pub bezier: [(f64, f64); 4],
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
    let lo = layout_sized(g, sizes);

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
    /// Bezier points + optional (label centre, label box width).
    type AbsEdge = ([(f64, f64); 4], Option<((f64, f64), f64)>);
    let mut abs_edges: Vec<AbsEdge> = Vec::with_capacity(g.edges.len());
    for (e, off) in g.edges.iter().zip(offs) {
        let a = &lo.nodes[e.from];
        let b = &lo.nodes[e.to];
        let pts = edge_points(
            a,
            b,
            g.nodes[e.from].shape,
            g.nodes[e.to].shape,
            e.from == e.to,
            lo.total_b,
            off,
            &lay_ext,
        );
        let label = e.label.as_ref().map(|l| {
            let mid = cubic_mid(pts[0], pts[1], pts[2], pts[3]);
            (mid, text_width(l) + 14.0)
        });
        abs_edges.push((pts, label));
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
    for (pts, label) in &abs_edges {
        for &(bp, lp) in pts {
            bb.add(bp, lp);
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
        .map(|(e, (pts, label))| SceneEdge {
            bezier: [tf(pts[0]), tf(pts[1]), tf(pts[2]), tf(pts[3])],
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

/// One laid-out subtree during hierarchical layout: node centres
/// and cluster boxes in coordinates local to the fragment.
struct Frag {
    w: f64,
    h: f64,
    nodes: Vec<(usize, f64, f64)>,
    clusters: Vec<(usize, f64, f64, f64, f64, usize)>, // sub, x, y, w, h, depth
}

/// Hierarchical layout for graphs with subgraphs: every subgraph
/// lays out its own members (recursively), then appears in its
/// parent as one supernode sized to fit. Edge geometry uses the
/// free-position router, so cross-cluster edges attach to the real
/// nodes rather than the cluster boxes.
fn scene_clustered(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    let nsub = g.subgraphs.len();
    let mut child_subs: Vec<Vec<usize>> = vec![Vec::new(); nsub];
    let mut root_subs: Vec<usize> = Vec::new();
    for (i, s) in g.subgraphs.iter().enumerate() {
        match s.parent {
            Some(p) => child_subs[p].push(i),
            None => root_subs.push(i),
        }
    }
    let mut owner: Vec<Option<usize>> = vec![None; g.nodes.len()];
    for (i, s) in g.subgraphs.iter().enumerate() {
        for &n in &s.nodes {
            owner[n] = Some(i);
        }
    }

    let frag = layout_frag(g, sizes, None, &root_subs, &child_subs, &owner, g.direction);

    // Absolute node centres (root-local == world, pre-normalisation).
    let mut centers = vec![(0.0, 0.0); g.nodes.len()];
    for &(i, x, y) in &frag.nodes {
        centers[i] = (x, y);
    }
    let placed: Vec<Placed> = (0..g.nodes.len())
        .map(|i| Placed {
            b: centers[i].0,
            l: centers[i].1,
            bsize: sizes[i].0,
            lsize: sizes[i].1,
            layer: 0,
        })
        .collect();

    // Edge geometry: free-position routing (position-aware sides),
    // identical to what `route` uses after a drag.
    let offs = parallel_offsets(g);
    let mut edges: Vec<SceneEdge> = Vec::with_capacity(g.edges.len());
    for (e, off) in g.edges.iter().zip(offs) {
        let pts = free_edge(
            &placed[e.from],
            &placed[e.to],
            g.nodes[e.from].shape,
            g.nodes[e.to].shape,
            e.from == e.to,
            off,
        );
        let label = e.label.as_ref().map(|l| {
            (
                l.clone(),
                cubic_mid(pts[0], pts[1], pts[2], pts[3]),
                text_width(l) + 14.0,
            )
        });
        edges.push(SceneEdge {
            bezier: pts,
            kind: e.kind,
            label,
        });
    }

    // Canvas bbox over nodes, cluster boxes, curves, and labels;
    // then translate everything so content starts at MARGIN.
    let mut bb = Bbox::new();
    for i in 0..g.nodes.len() {
        bb.add(centers[i].0 - sizes[i].0 / 2.0, centers[i].1 - sizes[i].1 / 2.0);
        bb.add(centers[i].0 + sizes[i].0 / 2.0, centers[i].1 + sizes[i].1 / 2.0);
    }
    for &(_, x, y, w, h, _) in &frag.clusters {
        bb.add(x, y);
        bb.add(x + w, y + h);
    }
    for e in &edges {
        for &(x, y) in &e.bezier {
            bb.add(x, y);
        }
        if let Some((_, m, w)) = &e.label {
            bb.add(m.0 - w / 2.0, m.1 - 10.0);
            bb.add(m.0 + w / 2.0, m.1 + 10.0);
        }
    }
    let (minx, maxx, miny, maxy) = bb.finish();
    let (tx, ty) = (MARGIN - minx, MARGIN - miny);

    let nodes: Vec<SceneNode> = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| SceneNode {
            x: centers[i].0 + tx,
            y: centers[i].1 + ty,
            w: sizes[i].0,
            h: sizes[i].1,
            shape: n.shape,
            label: n.label.clone(),
            style: n.style.clone(),
        })
        .collect();
    for e in edges.iter_mut() {
        for p in e.bezier.iter_mut() {
            *p = (p.0 + tx, p.1 + ty);
        }
        if let Some((_, m, _)) = &mut e.label {
            *m = (m.0 + tx, m.1 + ty);
        }
    }
    let mut clusters: Vec<SceneCluster> = frag
        .clusters
        .iter()
        .map(|&(sub, x, y, w, h, depth)| SceneCluster {
            x: x + tx,
            y: y + ty,
            w,
            h,
            title: g.subgraphs[sub].title.clone(),
            depth,
        })
        .collect();
    clusters.sort_by_key(|c| c.depth); // outermost first for painting

    Scene {
        nodes,
        edges,
        clusters,
        width: (maxx - minx) + 2.0 * MARGIN,
        height: (maxy - miny) + 2.0 * MARGIN,
    }
}

/// Lay out one cluster level (`level == None` for the root):
/// recurse into child subgraphs, then run the flat pipeline over
/// this level's direct nodes plus one supernode per child.
fn layout_frag(
    g: &Graph,
    sizes: &[(f64, f64)],
    level: Option<usize>,
    subs_here: &[usize],
    child_subs: &[Vec<usize>],
    owner: &[Option<usize>],
    inherited_dir: Direction,
) -> Frag {
    let dir = level
        .and_then(|l| g.subgraphs[l].direction)
        .unwrap_or(inherited_dir);

    let child_frags: Vec<(usize, Frag)> = subs_here
        .iter()
        .map(|&cs| {
            (
                cs,
                layout_frag(g, sizes, Some(cs), &child_subs[cs], child_subs, owner, dir),
            )
        })
        .collect();

    let direct: Vec<usize> = match level {
        Some(l) => g.subgraphs[l].nodes.clone(),
        None => (0..g.nodes.len()).filter(|&v| owner[v].is_none()).collect(),
    };

    // Synthetic single-level graph: direct nodes, then supernodes.
    let mut syn = Graph::default();
    syn.direction = dir;
    let mut slot_of_node: HashMap<usize, usize> = HashMap::new();
    for (k, &v) in direct.iter().enumerate() {
        syn.ensure_node(&format!("n{}", k), None, Some(Shape::Rect));
        slot_of_node.insert(v, k);
    }
    let mut slot_of_sub: HashMap<usize, usize> = HashMap::new();
    for (k, (cs, _)) in child_frags.iter().enumerate() {
        syn.ensure_node(&format!("s{}", k), None, Some(Shape::Rect));
        slot_of_sub.insert(*cs, direct.len() + k);
    }
    let mut syn_sizes: Vec<(f64, f64)> = direct.iter().map(|&v| sizes[v]).collect();
    syn_sizes.extend(child_frags.iter().map(|(_, f)| (f.w, f.h + SUB_HEADER)));

    // Project every real edge onto this level's entities: a node
    // maps to itself when it lives here, or to the child supernode
    // whose subtree contains it; edges that don't cross this level
    // are handled elsewhere (deeper or higher).
    let project = |v: usize| -> Option<usize> {
        if owner[v] == level {
            return slot_of_node.get(&v).copied();
        }
        let mut s = owner[v];
        while let Some(si) = s {
            if g.subgraphs[si].parent == level {
                return slot_of_sub.get(&si).copied();
            }
            s = g.subgraphs[si].parent;
        }
        None
    };
    for e in &g.edges {
        if let (Some(sa), Some(sb)) = (project(e.from), project(e.to)) {
            if sa != sb {
                syn.add_edge(sa, sb, None, EdgeKind::Arrow);
            }
        }
    }

    let sc = scene_flat(&syn, &syn_sizes);

    let mut frag = Frag {
        w: sc.width,
        h: sc.height,
        nodes: Vec::new(),
        clusters: Vec::new(),
    };
    for (k, &v) in direct.iter().enumerate() {
        frag.nodes.push((v, sc.nodes[k].x, sc.nodes[k].y));
    }
    for (k, (cs, cf)) in child_frags.into_iter().enumerate() {
        let slot = &sc.nodes[direct.len() + k];
        let (bw, bh) = (cf.w, cf.h + SUB_HEADER);
        let x0 = slot.x - bw / 2.0;
        let y0 = slot.y - bh / 2.0;
        for (i, x, y) in cf.nodes {
            frag.nodes.push((i, x + x0, y + y0 + SUB_HEADER));
        }
        for (sub, x, y, w, h, depth) in cf.clusters {
            frag.clusters
                .push((sub, x + x0, y + y0 + SUB_HEADER, w, h, depth + 1));
        }
        frag.clusters.push((cs, x0, y0, bw, bh, 0));
    }
    frag
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
        let label = e.label.as_ref().map(|l| {
            (
                l.clone(),
                cubic_mid(pts[0], pts[1], pts[2], pts[3]),
                text_width(l) + 14.0,
            )
        });
        edges.push(SceneEdge {
            bezier: pts,
            kind: e.kind,
            label,
        });
    }

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

    let clusters = route_clusters(g, &nodes);
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

/// Cluster boxes wrapped around the CURRENT node positions —
/// computed deepest-first so parents enclose their children's
/// boxes; returned outermost-first for painting.
fn route_clusters(g: &Graph, nodes: &[SceneNode]) -> Vec<SceneCluster> {
    let nsub = g.subgraphs.len();
    if nsub == 0 {
        return Vec::new();
    }
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
    let mut boxes: Vec<Option<(f64, f64, f64, f64)>> = vec![None; nsub];
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
    let mut out: Vec<SceneCluster> = (0..nsub)
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

    // Cluster boxes first (outermost-first), behind everything.
    for c in &sc.clusters {
        let (x, y) = t((c.x, c.y));
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\" \
             fill=\"#f7f8fd\" stroke=\"#c9cfe8\" stroke-width=\"1.4\"/>\n",
            x, y, c.w, c.h
        ));
        s.push_str(&format!(
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
        let q: Vec<(f64, f64)> = e.bezier.iter().map(|&p| t(p)).collect();
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
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>\n",
            q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1, q[3].0, q[3].1,
            EDGE_COLOR, sw, dash, marker
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
        }
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             fill=\"{}\">{}</text>\n",
            cx,
            cy,
            n.style.color.as_deref().unwrap_or(TEXT_COLOR),
            escape(&n.label)
        ));
    }

    s.push_str(&edge_labels);
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
        Shape::Diamond | Shape::Circle => border(p, shape, (other.0 + off * 4.0, other.1)),
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
        // apex (t=0.5) sits ~24px outside the outermost node:
        // x(0.5) = 0.125*(x0+x3) + 0.75*BT.
        let (l0, l1) = (b.layer, a.layer);
        let mut ext_l = f64::INFINITY;
        let mut ext_r = f64::NEG_INFINITY;
        for li in l0..=l1 {
            ext_l = ext_l.min(lay_ext[li].0);
            ext_r = ext_r.max(lay_ext[li].1);
        }
        let ends = p0.0 + p3.0;
        let mid = ends / 2.0;
        let bt_r = ((4.0 / 3.0) * (ext_r + 24.0) - ends / 6.0).max(ext_r + 40.0);
        let bt_l = ((4.0 / 3.0) * (ext_l - 24.0) - ends / 6.0).min(ext_l - 40.0);
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
        Shape::Circle => hw / (dx * dx + dy * dy).sqrt(),
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
