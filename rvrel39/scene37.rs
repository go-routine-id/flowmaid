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
    layout_clustered_ranked, layout_sized, text_width, title_extra, LayoutResult, Placed,
    RankGroup, BAR_SIZE, FRAME_GAP, GAP_B, LABEL_BOX_H, TITLE_LINE_H, TITLE_SCALE,
};
use crate::model::{
    Direction, Edge, EdgeAnimation, EdgeCurve, EdgeEnds, EdgeKind, EdgeMeta, End, Graph, Marker,
    NodeLink, NodeStyle, Shape, SubEdge,
};
use crate::style::{Palette, Theme, ThemeVars};
use std::borrow::Cow;
use std::collections::HashMap;

// Edge-label placement lives in `labels`; these keep the call sites
// (and the tests) here compiling unchanged.
pub(crate) use crate::labels::{
    avoid_label_overlaps, channel_label_point, fit_moved_labels, FRAME_BORDER_BAND, LABEL_GAP,
    LABEL_ROUTE_WORK, LABEL_STATIC_WORK,
};
#[cfg(test)]
pub(crate) use crate::labels::{covered, ObstacleGrid, Rect};

/// Gutter (px) round the drawing — everything [`grow_canvas`] covers,
/// every stroke and end glyph included — in a laid-out scene's own
/// coordinates and canvas (`Scene::width` / `height`), and the padding
/// [`to_svg`] gives the SVG, so a host drawing the scene sees the SVG's
/// canvas, coordinate for coordinate. Report Q10's 16 px would move
/// every laid-out coordinate, and a translation flips the router's
/// float-exact tie-breaks (a sixth of the corpus reroutes): a layout
/// change Q10 rules out.
pub(crate) const MARGIN: f64 = 28.0;
/// Gap between parallel edges (connecting the same node pair).
const PARALLEL_GAP: f64 = 16.0;
/// How far (px) the controls of a labelled 2-cycle's two curves across
/// one gap bow apart across the flow in LR / RL ([`twin_bows`]).
const TWIN_LABEL_BOW: f64 = LABEL_BOX_H + 2.0 * crate::labels::STROKE_PAD;
/// Most (degrees) a twin's bow ([`bowed`]) tilts an end off square: the
/// bow shrinks to what keeps both ends within it.
const TWIN_BOW_TILT: f64 = 9.5;
/// Most (degrees) a [`skewed`] twin's far end may bend over its last
/// arrowhead length (the chord there against the flow, on the curve as
/// built): the painter squares a head in from up to about that bend
/// (state r212's return head, 34 deg, enters within 10 deg), not from
/// much more (bench-75's `cache -> svc4`, its ends 410 px apart across a
/// 60 px gap, bent 55 deg and swung into `svc4` at 75 deg). A twin bent
/// past it already keeps its course, and the router reroutes it off its
/// twin as before.
const TWIN_SKEW_BEND: f64 = 35.0;
/// How far (px) a self-loop's controls reach past its node's side.
const LOOP_EXT: f64 = 48.0;
/// Half the spread (px) of a self-loop's two ends along its node's side
/// ([`loop_points`]): a node's loops stack [`PARALLEL_GAP`] apart, each
/// starting where the one before ends.
const LOOP_HALF: f64 = PARALLEL_GAP / 2.0;
/// How much farther (px) along the side than its ends a self-loop's
/// controls lie: the loop flares out round its ends.
const LOOP_FLARE: f64 = 20.0;
/// How far (px) a stack of self-loops squeezed onto its node's side
/// keeps its outermost ends from the side's corners ([`loop_points`]);
/// two 16 px loops on a 38 px box, 3 px from them, keep their place.
const LOOP_CORNER: f64 = 3.0;
/// Room (px) a parallel edge's label keeps from the neighbouring line of
/// its pair ([`parallel_offsets`]): a stroke's pad and a clear gap.
const PARALLEL_LABEL_AIR: f64 = crate::labels::STROKE_PAD + crate::clearance::EDGE_SEP;
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
    /// The label as drawn: `\n` line breaks — the source's own and, in
    /// a label without any, the automatic wrap at the graph's
    /// [`Graph::wrapping_width`] ([`crate::layout::wrap_label`]) — and
    /// `<b>` / `<i>` markup. [`scene`], [`route`] and [`route_partial`]
    /// size the node for exactly these lines; [`scene_sized`] /
    /// [`route_sized`] wrap with the same built-in text estimate whatever
    /// sizes they are given, and [`scene_sized_with`] /
    /// [`route_sized_with`] with the embedder's own measure, so the lines
    /// are the ones a box sized by
    /// [`crate::layout::intrinsic_size_wrapped`] with that measure holds.
    /// The model's `Node::label` keeps a plain label as written.
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
    /// layers, free-routed edges) and as the fallback everywhere. The
    /// ends lie on the node borders; on a forward, side-by-side or nested
    /// curve the controls lie on the sides' normals, so a head drawn
    /// along the end tangent points square in.
    /// flowmaid's own SVG ends a curve still turning there in a straight
    /// run along that tangent, bending into it within a px of this curve
    /// outside the arrowhead, and stops the stroke under its arrowhead.
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
    /// The stroke width this edge is painted at when its scene's own
    /// painter does not follow the flowchart rule (the class painter's
    /// relationship and note-link widths, the ER painter's line);
    /// `None` = the flowchart rule. Set by those scene builders and read
    /// through [`edge_stroke_width`] by the painter and by picking alike.
    pub(crate) paint_width: Option<f64>,
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
    /// test is shape-precise for every shape whose outline leaves its
    /// box's corners empty — diamonds, (double) circles, hexagons,
    /// parallelograms, cylinders, stadiums and rounded boxes, as the
    /// flowchart painter draws them, on whose outline every edge end
    /// sits — and the bounding rectangle for plain boxes (`Rect`,
    /// `Subroutine`, a fork bar).
    pub fn node_at(&self, x: f64, y: f64) -> Option<usize> {
        self.nodes.iter().rposition(|n| node_contains(n, x, y))
    }

    /// Index of the edge closest to `(x, y)` within `tol` scene units,
    /// or `None`. Distance is measured to the drawn STROKE — the curve
    /// `to_svg` paints (the spline through the routed waypoints, a
    /// `curve` style's path, or the single cubic), widened by half its
    /// stroke width, so a point on a thick or `linkStyle`-widened line
    /// picks it even at `tol = 0` — and the NEAREST edge wins so
    /// overlapping edges resolve cleanly; where several strokes cover
    /// the point, the later-drawn (topmost) one wins. Exactly the edges
    /// `to_svg` paints are pickable: a `~~~` link only when a
    /// `linkStyle` gives it a visible stroke.
    pub fn edge_at(&self, x: f64, y: f64, tol: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (i, e) in self.edges.iter().enumerate() {
            if !edge_drawn(e) {
                continue; // never drawn — never picked
            }
            // Zero anywhere inside the painted stroke: inside it, the
            // topmost stroke is what the pointer is on. Curve parts
            // farther than the best pick so far are never flattened.
            let half = edge_stroke_width(e) / 2.0;
            let reach = best.map_or(tol, |(_, bd)| bd.min(tol)) + half;
            let d = (edge_distance(e, (x, y), reach) - half).max(0.0);
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
    /// paint behind nodes and edges, so prefer [`Self::hit_test`] when you
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
    /// then clusters, each in index order. An edge counts once its
    /// painted STROKE touches the rectangle — measured on the drawn
    /// curve widened by half its stroke width, as [`Self::edge_at`] measures
    /// it — and exactly the edges `edge_at` can pick are selectable.
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
            if !edge_drawn(e) {
                continue; // the same edges edge_at can pick
            }
            // The painted curve itself, not a sampling of it (which cuts
            // inside a bend by more than half a stroke): within half the
            // stroke width of the rectangle, the stroke reaches into it.
            let half = edge_stroke_width(e) / 2.0;
            let marquee = PickRect {
                x0: rx0,
                y0: ry0,
                x1: rx1,
                y1: ry1,
            };
            if edge_distance(e, marquee, half) <= half {
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

/// Whether a node's SHAPE (not just its bounding box) contains a point:
/// the region inside the outline `to_svg` paints ([`node_outline`],
/// exactly rather than flattened), which is also where
/// [`crate::edgepass`] puts every edge end ([`outline_along`]). Plain
/// boxes (`Rect`, `Subroutine`, a fork bar) are their bounding
/// rectangle: other diagrams' painters draw their `Rect` nodes with
/// corners of their own (a git commit's square, a class card), and a
/// 3 px corner is nothing to pick round.
fn node_contains(n: &SceneNode, x: f64, y: f64) -> bool {
    let (hw, hh) = (n.w / 2.0, n.h / 2.0);
    if hw <= 0.0 || hh <= 0.0 {
        return false;
    }
    let (dx, dy) = ((x - n.x).abs(), (y - n.y).abs());
    let circle = matches!(
        n.shape,
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd
    );
    // A circle is a disc of radius w/2 whatever its height; every other
    // outline lies within its box.
    if !circle && (dx > hw || dy > hh) {
        return false;
    }
    // Inside a box whose corners are elliptic arcs of radii (rx, ry),
    // each clamped to its half extent as an SVG `<rect rx>` draws it.
    let rounded = |rx: f64| {
        let (rx, ry) = (rx.min(hw), rx.min(hh));
        let (ox, oy) = (dx - (hw - rx), dy - (hh - ry));
        ox <= 0.0 || oy <= 0.0 || (ox / rx).powi(2) + (oy / ry).powi(2) <= 1.0
    };
    let k = SLANT_MAX.min(n.w / 4.0);
    match n.shape {
        // Rhombus: |dx|/hw + |dy|/hh <= 1.
        Shape::Diamond => dx / hw + dy / hh <= 1.0,
        // Circle: the renderer draws `<circle r=w/2>` (state
        // pseudostates too) — a DISC of radius w/2 on both axes, not an
        // ellipse — so match that exactly for any w/h.
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            dx * dx + dy * dy <= hw * hw
        }
        Shape::Rounded => rounded(ROUNDED_RX),
        Shape::Stadium => rounded(hh),
        // The side points sit `k` in from the box at the top and bottom
        // and on it at mid height.
        Shape::Hexagon => dx <= hw - k * dy / hh,
        // Each flank runs `k` across over the full height: `/ /` has
        // its top edge `k` in from the left and its bottom edge `k` in
        // from the right, `\ \` the mirror image.
        Shape::Parallelogram | Shape::ParallelogramAlt => {
            let (sx, sy) = (x - n.x, y - n.y);
            // How far in from the left (and from the right) the outline
            // is at this height: `k` at one edge, 0 at the other.
            let down = (sy + hh) / (2.0 * hh);
            let (left, right) = if n.shape == Shape::Parallelogram {
                (k * (1.0 - down), k * down)
            } else {
                (k * down, k * (1.0 - down))
            };
            sx >= -hw + left && sx <= hw - right
        }
        // The lid's back arc above, the bottom arc below, both of the
        // cap ellipse (`rx` = the half width) round the cap centres.
        Shape::Cylinder => {
            let ry = cylinder_ry(n.w, n.h);
            let over = dy - (hh - ry);
            over <= 0.0 || ry <= 0.0 || (dx / hw).powi(2) + (over / ry).powi(2) <= 1.0
        }
        Shape::Rect | Shape::Subroutine | Shape::ForkBar => true,
    }
}

/// What [`edge_distance`] measures an edge's curve to: the pointer of
/// [`Scene::edge_at`] (a point) or the marquee of [`Scene::hits_in_rect`]
/// ([`PickRect`]). Each answers the two questions the curve walk asks.
trait PickTarget: Copy {
    /// Distance from the target to the segment `a`–`b`.
    fn seg_dist(self, a: (f64, f64), b: (f64, f64)) -> f64;
    /// Distance from the target to the box `[x0, x1] × [y0, y1]`.
    fn box_gap(self, x0: f64, y0: f64, x1: f64, y1: f64) -> f64;
}

impl PickTarget for (f64, f64) {
    fn seg_dist(self, a: (f64, f64), b: (f64, f64)) -> f64 {
        point_seg_dist(self.0, self.1, a, b)
    }
    fn box_gap(self, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        (x0 - self.0)
            .max(self.0 - x1)
            .max(0.0)
            .hypot((y0 - self.1).max(self.1 - y1).max(0.0))
    }
}

/// An axis-aligned rectangle (`x0 <= x1`, `y0 <= y1`) as a pick target.
#[derive(Clone, Copy)]
struct PickRect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl PickTarget for PickRect {
    fn seg_dist(self, a: (f64, f64), b: (f64, f64)) -> f64 {
        if seg_aabb(a, b, self.x0, self.y0, self.x1, self.y1) {
            return 0.0;
        }
        // Apart, the nearest pair has an end of the segment or a corner
        // of the rectangle in it.
        let corner = |x: f64, y: f64| point_seg_dist(x, y, a, b);
        let end = |p: (f64, f64)| p.box_gap(self.x0, self.y0, self.x1, self.y1);
        end(a)
            .min(end(b))
            .min(corner(self.x0, self.y0))
            .min(corner(self.x1, self.y0))
            .min(corner(self.x0, self.y1))
            .min(corner(self.x1, self.y1))
    }
    fn box_gap(self, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        (x0 - self.x1)
            .max(self.x0 - x1)
            .max(0.0)
            .hypot((y0 - self.y1).max(self.y0 - y1).max(0.0))
    }
}

/// Distance from `target` (a point or a rectangle) to the centre line
/// of the curve an edge is DRAWN as — the very commands
/// [`edge_path_d`] emits — or infinity when no part of it comes within
/// `reach`. Measured on the curve
/// itself, not on [`edge_polyline`]: that fixed sampling (a few points
/// per spline span) cuts inside a long gentle bend by up to a couple of
/// units, more than half a stroke, so a pick on the painted line could
/// miss it — or land on a neighbour whose chord, not whose stroke,
/// passes there. Each cubic is first rejected by its control-point box
/// (the curve never leaves it), so the far-away parts of every edge cost
/// one box test per pointer move and only the spans under the pointer
/// are flattened. Once a part touches the target (distance 0) nothing
/// nearer can follow, and the rest of the walk is skipped.
fn edge_distance<T: PickTarget>(e: &SceneEdge, target: T, reach: f64) -> f64 {
    let walk = |start: (f64, f64), segs: &[Seg], reach: f64| {
        let mut best = f64::INFINITY;
        let mut cur = start;
        for &seg in segs {
            if best <= 0.0 {
                break;
            }
            let lim = best.min(reach);
            let d = match seg {
                Seg::L(p) => target.seg_dist(cur, p),
                // A quadratic is the cubic with its control points 2/3
                // of the way toward the shared one.
                Seg::Q(c, p) => cubic_dist(
                    target,
                    &[cur, lerp(cur, c, 2.0 / 3.0), lerp(p, c, 2.0 / 3.0), p],
                    lim,
                    0,
                ),
                Seg::C(c1, c2, p) => cubic_dist(target, &[cur, c1, c2, p], lim, 0),
            };
            best = best.min(d);
            cur = seg.end();
        }
        best
    };
    // The very commands edge_path_d emits, span for span. Squaring an
    // end ([`square_end`]) costs far more than a pick, so it is done only
    // where it can change the answer. It changes nothing farther than
    // SQUARE_END_REACH from that end's tip (`far`; more under a very
    // wide stroke's grown head, square_end_reach), and keeps the tip
    // itself; outside the head its new stretch keeps within
    // SQUARE_END_PICK_SLACK of the scene curve, and under the head
    // within SQUARE_END_HEAD_REACH of the tip (ditto). So the scene curve's own
    // distance (`d`, measured out to `reach` and that slack) settles the
    // pick whenever it is nearer than everything the end could change,
    // or when even the nearest the new stretch could come is out of
    // reach — a pointer away from every tip, or right on a line, never
    // squares an end; one away from every tip measures the curve alone,
    // as far out as `reach`.
    let (start, segs) = plain_segs(e, |p| p);
    let ends = squared_ends(e);
    if ends == [false; 2] {
        return walk(start, &segs, reach);
    }
    let tip = segs.last().map_or(start, |s| s.end());
    let gap = |q: (f64, f64)| target.box_gap(q.0, q.1, q.0, q.1);
    let tips = [start, tip];
    if (0..2).any(|k| ends[k] && gap(tips[k]) <= 0.0) {
        return 0.0; // on a tip, which the squared stroke keeps
    }
    let near = walk(start, &segs, reach);
    let (end_reach, head_reach) = square_end_reach(e);
    let far = |k: usize| (gap(tips[k]) - end_reach).max(0.0);
    if (0..2).all(|k| !ends[k] || far(k) > reach) {
        return near;
    }
    let d = if near <= reach {
        near
    } else {
        walk(start, &segs, reach + SQUARE_END_PICK_SLACK)
    };
    // Whether squaring end k could change `answer`, the distance to a
    // path that has that end as the scene curve has it.
    let open = |k: usize, answer: f64| {
        let far = far(k);
        let nearest = (d - SQUARE_END_PICK_SLACK).min(gap(tips[k]) - head_reach);
        ends[k] && far <= reach && far <= answer && nearest <= reach
    };
    if !open(0, d) && !open(1, d) {
        return d;
    }
    // The target end alone squares exactly as with both; the source end
    // only after it.
    if !open(0, d) {
        let (s, g, ..) = square_ends_of(e, start, segs.clone(), [false, true]);
        let d1 = walk(s, &g, reach);
        if !open(0, d1) {
            return d1;
        }
    }
    let (s, g, ..) = square_ends(e, start, segs);
    walk(s, &g, reach)
}

/// `a + (b - a) * t`.
fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Distance from `p` (a point or a rectangle) to the cubic `b`, or
/// infinity when its control-point box is farther than `lim`: halve the
/// curve until each piece is flat to within [`PICK_FLATNESS`] of its
/// chord, then measure to the chord.
fn cubic_dist<T: PickTarget>(p: T, b: &[(f64, f64); 4], lim: f64, depth: u32) -> f64 {
    /// How far a flattened piece may bow off the chord it is measured
    /// by — well under the default stroke's half width.
    const PICK_FLATNESS: f64 = 0.05;
    let (mut x0, mut y0, mut x1, mut y1) = (b[0].0, b[0].1, b[0].0, b[0].1);
    for q in &b[1..] {
        (x0, y0, x1, y1) = (x0.min(q.0), y0.min(q.1), x1.max(q.0), y1.max(q.1));
    }
    if p.box_gap(x0, y0, x1, y1) > lim {
        return f64::INFINITY;
    }
    let bow =
        point_seg_dist(b[1].0, b[1].1, b[0], b[3]).max(point_seg_dist(b[2].0, b[2].1, b[0], b[3]));
    if bow <= PICK_FLATNESS || depth >= 16 {
        return p.seg_dist(b[0], b[3]);
    }
    // de Casteljau split at t = 1/2.
    let ab = lerp(b[0], b[1], 0.5);
    let bc = lerp(b[1], b[2], 0.5);
    let cd = lerp(b[2], b[3], 0.5);
    let abc = lerp(ab, bc, 0.5);
    let bcd = lerp(bc, cd, 0.5);
    let mid = lerp(abc, bcd, 0.5);
    let left = cubic_dist(p, &[b[0], ab, abc, mid], lim, depth + 1);
    if left <= 0.0 {
        return left; // touching: the other half cannot come nearer
    }
    left.min(cubic_dist(
        p,
        &[mid, bcd, cd, b[3]],
        lim.min(left),
        depth + 1,
    ))
}

/// A dense sampling of the curve an edge describes — the one the
/// routing, port and label stages reason on: a routed edge follows the
/// same curveBasis spline [`spline_segs`] draws (which pulls away from
/// the raw waypoints), a plain edge its single cubic. The flowchart
/// painter squares each glyph end to its glyph ([`square_end`]): within
/// [`SQUARE_END_DEV`] of this curve outside the arrowhead (up to half the
/// width of a very wide `linkStyle` stroke), so those
/// stages keep reasoning on the curve the scene holds (a host draws it
/// too), and a route never changes for the way its head is drawn.
/// Picking, the crossing casing and the quality yardstick follow the
/// painted stroke ([`edge_painted_polyline`]).
pub(crate) fn edge_polyline(e: &SceneEdge) -> Vec<(f64, f64)> {
    if let Some((start, segs)) = curve_geometry(e) {
        return curve_sample(start, &segs);
    }
    if e.waypoints.len() >= 2 {
        return basis_sample(&e.waypoints);
    }
    sample_cubic(&e.bezier, 24)
}

/// A dense sampling of the stroke the flowchart painter draws for `e`
/// (the commands of [`drawn_segs`], uncut by the arrow trims): the
/// [`edge_polyline`] curve, its glyph ends squared ([`square_ends`]) —
/// sampled as that one is, the new bends as finely as a curve span.
/// What the crossing casing cuts gaps in and the quality yardstick
/// measures.
pub(crate) fn edge_painted_polyline(e: &SceneEdge) -> Vec<(f64, f64)> {
    painted_polyline(e, &painted_edge(e))
}

/// The stroke the flowchart painter draws for `e`, worked out once per
/// paint: its scene curve ([`plain_segs`], scene coordinates) with each
/// glyph end squared ([`square_ends`]), whole (uncut by the arrow
/// trims). [`to_svg`] strokes it ([`painted_path_d`]) and hands its
/// samples ([`painted_polyline`]) to the crossing casing, so no paint
/// squares an end twice.
pub(crate) struct PaintedEdge {
    start: (f64, f64),
    segs: Vec<Seg>,
    cuts: Cuts,
    runs: Runs,
}

impl PaintedEdge {
    /// One end of the stroke as `(tip, heading into it)`: the end point
    /// and the way its glyph faces (squared ends face their own tangent).
    fn end(&self, at_start: bool) -> Option<((f64, f64), (f64, f64))> {
        if at_start {
            let (from, back) = reverse_segs(self.start, self.segs.get(..1)?);
            Some((self.start, end_heading(from, *back.first()?)?))
        } else {
            let n = self.segs.len();
            let from = if n >= 2 {
                self.segs[n - 2].end()
            } else {
                self.start
            };
            let last = *self.segs.last()?;
            Some((last.end(), end_heading(from, last)?))
        }
    }
}

/// Least gap (px) the painter keeps between two arrowheads standing
/// side by side ([`head_sizes`]).
const HEAD_GAP: f64 = 1.5;
/// Narrowest (px across its base) an arrowhead is drawn where heads
/// crowd ([`head_sizes`]): still plainly a head.
const HEAD_MIN_WIDTH: f64 = 6.0;

/// How the flowchart painter draws one arrowhead (or, for a cross or
/// circle end, its glyph box): how far (px) the stroke stops short of
/// the tip, the glyph's length and its width across the base (px).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Head {
    trim: f64,
    len: f64,
    width: f64,
}

impl Head {
    /// The usual end glyph: [`ARROW_MARKER_PX`] square, its stroke cut
    /// [`ARROW_TRIM`] short of an arrow's tip.
    const PLAIN: Head = Head {
        trim: ARROW_TRIM,
        len: ARROW_MARKER_PX,
        width: ARROW_MARKER_PX,
    };
}

/// Least length (px) an arrowhead shrunk onto a curve too short for its
/// cut is still drawn at ([`head_sizes`]); shorter, it is left out.
const HEAD_MIN_LEN: f64 = 1.0;

/// How each drawn edge's end glyphs are painted, as (source, target)
/// ([`Head`]), given each painted edge's length when its curve is too
/// short to lose its cuts (`whole`, [`painted_path`]). An arrowhead is
/// [`arrow_head_len`] long and wide — longer than [`ARROW_MARKER_PX`]
/// only over a very wide `linkStyle` stroke — its tip on the end point
/// with the stroke cut [`arrow_trims`] short of it; and
///
/// - where parallel heads stand side by side closer than a head and
///   [`HEAD_GAP`] (an over-full side's ports, about 10 px apart), that
///   pitch less the gap wide, to the half px, so neighbouring heads keep
///   apart instead of running together into one saw-tooth band — never
///   narrower than [`HEAD_MIN_WIDTH`], nor than covering the stroke's
///   butt end where the painter cuts it (heads sharing a port are left
///   as they are);
/// - on a curve too short to lose its cuts (a drag frame's nodes a few
///   px apart: the stroke is drawn whole), cut nowhere, the tip on the
///   curve's end, and no longer than its share of the curve (the curve's
///   length over its arrowheads, to the tenth of a px below) — so it
///   never reaches past the curve's end into the node, where picking
///   would not see it; under [`HEAD_MIN_LEN`] it is left out (length 0).
///
/// Painter-only, like the head itself.
fn head_sizes(
    sc: &Scene,
    painted: &[Option<PaintedEdge>],
    whole: &[Option<f64>],
) -> Vec<[Head; 2]> {
    let mut out = vec![[Head::PLAIN; 2]; sc.edges.len()];
    for (i, e) in sc.edges.iter().enumerate() {
        let (t0, t3) = match painted.get(i).and_then(|p| p.as_ref()) {
            Some(p) => painted_trims(e, p),
            None => arrow_trims(e),
        };
        let len = edge_head_len(e);
        let arrows = [
            e.ends.start_marker(e.kind) == Some(Marker::Arrow),
            e.ends.end_marker(e.kind) == Some(Marker::Arrow),
        ];
        for (k, trim) in [t0, t3].into_iter().enumerate() {
            if arrows[k] {
                out[i][k] = Head {
                    trim,
                    len,
                    width: len,
                };
            }
        }
        if let Some(l) = whole.get(i).copied().flatten() {
            let n = arrows.iter().filter(|&&a| a).count().max(1) as f64;
            let fit = ((l / n * 10.0).floor() / 10.0).min(len);
            let fit = if fit < HEAD_MIN_LEN { 0.0 } else { fit };
            for k in (0..2).filter(|&k| arrows[k]) {
                out[i][k] = Head {
                    trim: 0.0,
                    len: fit,
                    width: fit,
                };
            }
        }
    }
    // (edge, end: 0 source / 1 target, tip, heading)
    let mut heads = Vec::new();
    for (i, (e, p)) in sc.edges.iter().zip(painted).enumerate() {
        let Some(p) = p.as_ref().filter(|_| e.paint_width.is_none()) else {
            continue;
        };
        for k in 0..2 {
            let m = if k == 0 {
                e.ends.start_marker(e.kind)
            } else {
                e.ends.end_marker(e.kind)
            };
            // The usual head alone: a wide stroke's or a shrunk one is
            // its own size.
            if m == Some(Marker::Arrow) && out[i][k].len == ARROW_MARKER_PX && out[i][k].trim > 0.0
            {
                if let Some((tip, d)) = p.end(k == 0) {
                    heads.push((i, k, tip, d));
                }
            }
        }
    }
    if heads.len() < 2 {
        return out;
    }
    // Neighbours are found in a grid of cells larger than any pitch
    // that crowds.
    const CELL: f64 = 16.0;
    let cell = |q: (f64, f64)| ((q.0 / CELL).floor() as i64, (q.1 / CELL).floor() as i64);
    let mut grid: std::collections::BTreeMap<(i64, i64), Vec<usize>> = Default::default();
    for (h, &(_, _, tip, _)) in heads.iter().enumerate() {
        grid.entry(cell(tip)).or_default().push(h);
    }
    let parallel = 10f64.to_radians().cos();
    for (h, &(i, k, tip, d)) in heads.iter().enumerate() {
        let (cx, cy) = cell(tip);
        let mut pitch = f64::INFINITY;
        for o in (cx - 1..=cx + 1)
            .flat_map(|gx| (cy - 1..=cy + 1).map(move |gy| (gx, gy)))
            .filter_map(|c| grid.get(&c))
            .flatten()
        {
            let (_, _, t, d2) = heads[*o];
            if *o == h || d.0 * d2.0 + d.1 * d2.1 < parallel {
                continue;
            }
            // Side by side: tips level along the heading, apart across it.
            let (dx, dy) = (t.0 - tip.0, t.1 - tip.1);
            let lat = (dx * d.1 - dy * d.0).abs();
            if (dx * d.0 + dy * d.1).abs() <= 2.0 && lat >= 1.0 {
                pitch = pitch.min(lat);
            }
        }
        if pitch >= ARROW_MARKER_PX + HEAD_GAP {
            continue;
        }
        let e = &sc.edges[i];
        let trim = out[i][k].trim;
        // `trim` back from the tip, where the stroke stops, a head `w`
        // wide is `w * trim / ARROW_MARKER_PX` across: it must cover the
        // stroke's width there.
        let least = HEAD_MIN_WIDTH.max(edge_stroke_width(e) * ARROW_MARKER_PX / trim.max(1e-9));
        let w = ((pitch - HEAD_GAP) * 2.0).floor() / 2.0;
        out[i][k].width = w.max(least).min(ARROW_MARKER_PX);
    }
    out
}

/// [`PaintedEdge`] of `e`.
pub(crate) fn painted_edge(e: &SceneEdge) -> PaintedEdge {
    let (start, segs) = plain_segs(e, |p| p);
    let (start, segs, cuts, runs) = square_ends(e, start, segs);
    PaintedEdge {
        start,
        segs,
        cuts,
        runs,
    }
}

/// [`edge_painted_polyline`] of `e` from its [`PaintedEdge`] `p`.
pub(crate) fn painted_polyline(e: &SceneEdge, p: &PaintedEdge) -> Vec<(f64, f64)> {
    let (start, segs, cuts) = (p.start, &p.segs, p.cuts);
    let orig = edge_polyline(e);
    if cuts == [None, None] {
        return orig;
    }
    // The curve's own samples between the squared ends, by arc length
    // along them; each squared end's new stretch sampled as finely as a
    // curve span.
    let mut arc = Vec::with_capacity(orig.len());
    let mut run = 0.0;
    for (k, p) in orig.iter().enumerate() {
        if k > 0 {
            run += (p.0 - orig[k - 1].0).hypot(p.1 - orig[k - 1].1);
        }
        arc.push(run);
    }
    let (lo, hi) = (cuts[0].unwrap_or(0.0), run - cuts[1].unwrap_or(0.0));
    let sample = |out: &mut Vec<(f64, f64)>, from: (f64, f64), tail: &[Seg]| {
        let mut cur = from;
        for &seg in tail {
            match seg {
                Seg::L(p) => out.push(p),
                _ => {
                    let b = seg.cubic(cur);
                    out.extend((1..=12).map(|k| cubic_at(&b, k as f64 / 12.0)));
                }
            }
            cur = seg.end();
        }
    };
    let n = segs.len();
    let mut out = vec![start];
    if cuts[0].is_some() {
        sample(&mut out, start, &segs[..2]);
    }
    let inner = orig.iter().zip(&arc).filter(|&(_, &s)| s > lo && s < hi);
    if cuts[0].is_some() && cuts[1].is_some() {
        out.extend(inner.map(|(&p, _)| p));
    } else if cuts[0].is_some() {
        out.extend(inner.map(|(&p, _)| p));
        out.push(orig[orig.len() - 1]);
    } else {
        out.clear();
        out.push(orig[0]);
        out.extend(inner.map(|(&p, _)| p));
    }
    if cuts[1].is_some() {
        let j = if n >= 3 { segs[n - 3].end() } else { start };
        out.push(j);
        sample(&mut out, j, &segs[n - 2..]);
    }
    out
}

/// One drawing command of an edge's path (the start point is held
/// apart): every edge is drawn as a run of these ([`drawn_segs`]).
#[derive(Debug, Clone, Copy)]
enum Seg {
    L((f64, f64)),
    Q((f64, f64), (f64, f64)),
    C((f64, f64), (f64, f64), (f64, f64)),
}

impl Seg {
    /// Where the command ends.
    fn end(self) -> (f64, f64) {
        match self {
            Seg::L(p) | Seg::Q(_, p) | Seg::C(_, _, p) => p,
        }
    }

    /// The command with every point mapped through `t`.
    fn map(self, t: &impl Fn((f64, f64)) -> (f64, f64)) -> Seg {
        match self {
            Seg::L(p) => Seg::L(t(p)),
            Seg::Q(c, p) => Seg::Q(t(c), t(p)),
            Seg::C(c1, c2, p) => Seg::C(t(c1), t(c2), t(p)),
        }
    }

    /// The command as a cubic from `from` (a line or a quadratic raised
    /// to its exact cubic form).
    fn cubic(self, from: (f64, f64)) -> [(f64, f64); 4] {
        match self {
            Seg::L(p) => [from, lerp(from, p, 1.0 / 3.0), lerp(from, p, 2.0 / 3.0), p],
            Seg::Q(c, p) => [from, lerp(from, c, 2.0 / 3.0), lerp(p, c, 2.0 / 3.0), p],
            Seg::C(c1, c2, p) => [from, c1, c2, p],
        }
    }

    /// The part of the command from `from` between parameters `t0` and
    /// `t1` (0 ≤ t0 < t1 ≤ 1), as (its start, the command), in the same
    /// form (de Casteljau).
    fn part(self, from: (f64, f64), t0: f64, t1: f64) -> ((f64, f64), Seg) {
        match self {
            Seg::L(p) => (lerp(from, p, t0), Seg::L(lerp(from, p, t1))),
            Seg::Q(c, p) => {
                let at = |t: f64| lerp(lerp(from, c, t), lerp(c, p, t), t);
                // The control point of the part: where the tangents at
                // its ends meet, i.e. the blossom at (t0, t1).
                let ctl = lerp(lerp(from, c, t0), lerp(c, p, t0), t1);
                (at(t0), Seg::Q(ctl, at(t1)))
            }
            Seg::C(..) => {
                let b = self.cubic(from);
                // Blossom of the cubic at (u, v, w).
                let bl = |u: f64, v: f64, w: f64| {
                    let l1 = [
                        lerp(b[0], b[1], u),
                        lerp(b[1], b[2], u),
                        lerp(b[2], b[3], u),
                    ];
                    let l2 = [lerp(l1[0], l1[1], v), lerp(l1[1], l1[2], v)];
                    lerp(l2[0], l2[1], w)
                };
                (
                    bl(t0, t0, t0),
                    Seg::C(bl(t0, t0, t1), bl(t0, t1, t1), bl(t1, t1, t1)),
                )
            }
        }
    }
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

/// The commands an edge is DRAWN with, mapped through `t` (scene →
/// output coordinates), as (start point, commands): its scene curve
/// ([`plain_segs`]) with each end glyph's end squared to the glyph
/// ([`square_ends`]).
fn drawn_segs(e: &SceneEdge, t: impl Fn((f64, f64)) -> (f64, f64)) -> ((f64, f64), Vec<Seg>) {
    let (start, segs) = plain_segs(e, t);
    let (start, segs, ..) = square_ends(e, start, segs);
    (start, segs)
}

/// The commands of `e`'s scene curve as drawn, mapped through `t`
/// (scene → output coordinates), as (start point, commands): a
/// non-default mermaid `curve` through its points ([`curve_geometry`]),
/// else the `curveBasis` spline through routed waypoints
/// ([`spline_segs`]), else the single cubic `bezier`. The points are
/// mapped before the spline's arithmetic, as the painter always did.
fn plain_segs(e: &SceneEdge, t: impl Fn((f64, f64)) -> (f64, f64)) -> ((f64, f64), Vec<Seg>) {
    if let Some((start, segs)) = curve_geometry(e) {
        (t(start), segs.iter().map(|s| s.map(&t)).collect())
    } else if e.waypoints.len() >= 2 {
        let q: Vec<(f64, f64)> = e.waypoints.iter().map(|&p| t(p)).collect();
        spline_segs(&q)
    } else {
        let q = e.bezier.map(&t);
        (q[0], vec![Seg::C(q[1], q[2], q[3])])
    }
}

/// The path `(start, segs)` of `e` ([`plain_segs`]) with each glyph end
/// squared to its glyph ([`square_end`]), and for (source, target) how
/// far back along the curve that end's new stretch leaves it (`None`:
/// unchanged; the new stretch is the first two commands at the source,
/// the last two at the target): an end glyph
/// an end glyph of an edge the flowchart painter strokes (an ER or class
/// scene's painter draws its own ends; a self-loop keeps its loop). An
/// arrowhead's end gets at least the straight run the painter cuts under
/// it ([`arrow_trims`]), so the head faces along the curve's own end
/// tangent with its tip on the end point; the default curve gets up to
/// a [`GLYPH_STUB`] — a mermaid `curve` is the author's and only gets
/// what its head needs.
fn square_ends(e: &SceneEdge, start: (f64, f64), segs: Vec<Seg>) -> SquaredEnds {
    square_ends_of(e, start, segs, [true; 2])
}

/// A path [`square_ends`] squared: its start, its commands, how far back
/// each end's new stretch leaves the curve ([`Cuts`]) and the straight
/// run each end ends in ([`Runs`]).
type SquaredEnds = ((f64, f64), Vec<Seg>, Cuts, Runs);

/// How deep (px) the painter may cut a squared path under a head at
/// (source, target) and keep the tip on the end point ([`square_end`]:
/// in the straight run the end runs in on); infinite at an end left alone
/// (no glyph, or one [`squared_ends`] does not square): the painter cuts
/// it as it always has.
type Runs = [f64; 2];

/// Which ends of `e` [`square_ends`] may change, as (source, target):
/// those with an end glyph, on an edge the flowchart painter strokes —
/// on a self-loop, which keeps its loop, only an arrowhead over a very
/// wide stroke (its cut is wider than [`ARROW_TRIM`]): cut along the
/// loop, its head would face along the loop's tangent at the cut, its
/// tip well off the end.
fn squared_ends(e: &SceneEdge) -> [bool; 2] {
    if e.paint_width.is_some() || !edge_drawn(e) {
        return [false; 2];
    }
    let ends = [e.ends.start_marker(e.kind), e.ends.end_marker(e.kind)];
    if e.from != e.to {
        return ends.map(|m| m.is_some());
    }
    let (t0, t3) = arrow_trims(e);
    let wide = |m: Option<Marker>, trim: f64| m == Some(Marker::Arrow) && trim > ARROW_TRIM;
    [wide(ends[0], t0), wide(ends[1], t3)]
}

/// [`square_ends`] of the ends `which` (source, target) alone. The
/// target is squared first, so squaring it alone gives that end exactly
/// as squaring both does (the source is squared on the result).
fn square_ends_of(
    e: &SceneEdge,
    start: (f64, f64),
    segs: Vec<Seg>,
    which: [bool; 2],
) -> SquaredEnds {
    let can = squared_ends(e);
    if can == [false; 2] {
        return (start, segs, [None; 2], [f64::INFINITY; 2]);
    }
    let author = e.meta.curve.is_some_and(|c| c != EdgeCurve::Basis);
    let trims = arrow_trims(e);
    // (half the stroke, the head's length) under an arrowhead.
    let head = Some((edge_stroke_width(e) / 2.0, edge_head_len(e)));
    // Two glyphs share the curve: each end's bend keeps to its half.
    let share = if e.ends.start_marker(e.kind).is_some() && e.ends.end_marker(e.kind).is_some() {
        0.5
    } else {
        1.0
    };
    let (mut start, mut segs, mut cuts, mut runs) = (start, segs, [None; 2], [f64::INFINITY; 2]);
    for (at_start, m, trim) in [
        (false, e.ends.end_marker(e.kind), trims.1),
        (true, e.ends.start_marker(e.kind), trims.0),
    ] {
        let k = usize::from(!at_start);
        let Some(m) = m.filter(|_| which[k] && can[k]) else {
            continue;
        };
        let head = if m == Marker::Arrow { head } else { None };
        let want = glyph_run(author, trim, head.map(|h| h.1));
        if at_start {
            let (s, g) = reverse_segs(start, &segs);
            let (s, g, cut, run) = square_end(s, g, want, trim, head, share);
            if cut.is_some() {
                (start, segs) = reverse_segs(s, &g);
                cuts[0] = cut;
            }
            runs[0] = run;
        } else {
            let (cut, run);
            (start, segs, cut, run) = square_end(start, segs, want, trim, head, share);
            cuts[1] = cut;
            runs[1] = run;
        }
    }
    (start, segs, cuts, runs)
}

/// The straight run (px) [`square_end`] wants an end glyph's stroke to end
/// in: a [`GLYPH_STUB`] (an arrowhead's cut at least) on the default
/// curve, the cut alone on a mermaid `curve` (the author's; it only gets
/// what its head needs) — and, under an arrowhead longer than
/// [`ARROW_MARKER_PX`] (a very wide stroke's, [`arrow_head_len`]), the
/// whole head either way, so the wide stroke's butt and its join onto the
/// run hide in it.
fn glyph_run(author: bool, trim: f64, head_len: Option<f64>) -> f64 {
    match head_len {
        Some(len) if len > ARROW_MARKER_PX => len,
        _ if author => trim,
        _ => GLYPH_STUB.max(trim),
    }
}

/// How far (px of arc) back from (source, target) a squared end
/// ([`square_ends`]) leaves the scene's curve; `None`: unchanged.
type Cuts = [Option<f64>; 2];

/// The path `(start, segs)` run backwards, as (its end, the commands
/// back to its start): the same curve exactly.
fn reverse_segs(start: (f64, f64), segs: &[Seg]) -> ((f64, f64), Vec<Seg>) {
    let mut froms = Vec::with_capacity(segs.len());
    let mut cur = start;
    for s in segs {
        froms.push(cur);
        cur = s.end();
    }
    let back = segs
        .iter()
        .zip(&froms)
        .rev()
        .map(|(&s, &f)| match s {
            Seg::L(_) => Seg::L(f),
            Seg::Q(c, _) => Seg::Q(c, f),
            Seg::C(c1, c2, _) => Seg::C(c2, c1, f),
        })
        .collect();
    (cur, back)
}

/// Most (px) a squared end ([`square_end`]) strays from the curve the
/// scene holds, outside the arrowhead drawn over it: the routing, port
/// and label stages spaced every clearance (chips, node corners, other
/// lines, crossing gaps) against that curve, and a host draws it.
const SQUARE_END_DEV: f64 = 1.0;
/// Straight runs [`square_end`] tries, longest first, in parts of the
/// run it wants.
const SQUARE_END_RUNS: [f64; 3] = [1.0, 0.75, 0.5];
/// Lengths (in [`GLYPH_STUB`]s) of the bend from the curve into the
/// straight run that [`square_end`] tries, gentlest first.
const SQUARE_END_BENDS: [f64; 5] = [3.0, 2.0, 1.5, 1.0, 0.5];
/// Most (px of arc) a squared end ([`square_end`]) takes off the curve
/// it replaces: the longest run it wants (a [`GLYPH_STUB`], or an
/// arrowhead's cut, which is never longer than a head) and the longest
/// bend into it.
const SQUARE_END_ARC: f64 = GLYPH_STUB + SQUARE_END_BENDS[0] * GLYPH_STUB;
/// How far (px) from its end anything a squared end changes can lie:
/// the curve it replaces is at most [`SQUARE_END_ARC`] long, and the
/// new stretch keeps within the hull of where it leaves the curve, the
/// bend's handles (a third of its chord, which is at most that arc and
/// a run — or the straight entry's corner — long) and the run. Picking a target farther than this from an
/// end measures the scene curve there instead ([`edge_distance`]).
const SQUARE_END_REACH: f64 =
    SQUARE_END_ARC + (SQUARE_END_ARC + ARROW_MARKER_PX + SQUARE_END_ENTRY) / 3.0 + 0.5;
/// Most (px) a squared end that turns into its arrowhead from the side
/// may stray from the scene curve outside the head ([`square_end`]'s
/// straight entry and its least-straying bend): a few px, two heads from
/// the tip at most — farther, the head turns with the curve instead (the
/// chord into the end, within a run and a px of the tip).
const SQUARE_END_MAX_STRAY: f64 = 6.0;
/// How far (px) any point of a squared end's new stretch can lie from
/// the scene curve, outside its arrowhead: [`SQUARE_END_MAX_STRAY`]
/// (bends that keep within a px take [`SQUARE_END_DEV`]), measured at
/// samples a few px apart on a bend, so half that spacing more between
/// them. Picking measures the scene curve alone wherever that settles
/// it ([`edge_distance`]).
const SQUARE_END_PICK_SLACK: f64 = SQUARE_END_MAX_STRAY + 3.0;
/// How far (px) from the tip a squared end's points under its
/// arrowhead can lie: inside the head's triangle (as long as the head,
/// half as wide either side at its base), or on the run into the tip —
/// the last resort's chord too, which may stray farther from the scene
/// curve than [`SQUARE_END_PICK_SLACK`] but lies within a run and a px of
/// the tip ([`square_end`]).
const SQUARE_END_HEAD_REACH: f64 = ARROW_MARKER_PX * 1.12;

/// [`SQUARE_END_REACH`] and [`SQUARE_END_HEAD_REACH`] for edge `e`: the
/// same bounds, for an arrowhead longer than [`ARROW_MARKER_PX`] (a very
/// wide stroke's, [`arrow_head_len`]) and the longer run its end wants
/// under it ([`glyph_run`]).
fn square_end_reach(e: &SceneEdge) -> (f64, f64) {
    let len = edge_head_len(e);
    if len <= ARROW_MARKER_PX {
        return (SQUARE_END_REACH, SQUARE_END_HEAD_REACH);
    }
    let entry = square_end_entry(arrow_trim(edge_stroke_width(e)));
    let arc = len + SQUARE_END_BENDS[0] * GLYPH_STUB;
    (arc + (arc + len + entry) / 3.0 + 0.5, len * 1.12)
}
/// How far (px) up the end tangent beyond the arrow cut a curve that
/// swings in from the side meets it ([`square_end`]): the last stretch
/// before the cut runs along the tangent, so the head faces that way
/// (long enough that the SVG's 0.1 px rounding turns it under 4°), and
/// the corner onto it lies well inside the head. A very wide stroke's
/// longer cut takes a longer run ([`square_end_entry`]).
const SQUARE_END_ENTRY: f64 = 1.5;
/// Part of an arrow cut the straight run past it is at least
/// ([`square_end_entry`]).
const SQUARE_END_ENTRY_PART: f64 = 0.25;

/// The straight run (px) past an arrow cut of `trim` px that the stroke
/// ends in ([`square_end`]): [`SQUARE_END_ENTRY`], or a quarter of a
/// very wide stroke's cut (a `linkStyle stroke-width`, [`arrow_trim`]).
/// The run sets the way the head faces (`orient="auto"` reads the last
/// stretch the SVG strokes, its ends rounded to 0.1 px), and the tip lies
/// the cut's length on along it: a run a quarter of the cut keeps the tip
/// within about 0.3 px of the end point, as the usual run does under the
/// usual cut. 1.5 px up to a 6 px cut (the usual heads, thick included).
pub(crate) fn square_end_entry(trim: f64) -> f64 {
    SQUARE_END_ENTRY.max(trim * SQUARE_END_ENTRY_PART)
}
/// How near (degrees) the curve's heading must come to the line from
/// the straight entry's corner for [`square_end`] to leave the curve
/// there: the join shows no S, and the nearest such point to the end
/// keeps the straight line short (it runs beside less of what the
/// routing spaced against the curve).
const SQUARE_END_TOUCH: f64 = 5.0;
/// Pieces a squared end's new bend is sampled in where [`square_end`]
/// measures its stray (each a px or two long).
const SQUARE_END_BEND_STEPS: usize = 24;
/// Every how many samples of the curve's tail [`square_end`] tries to
/// leave it for the straight entry under the head (about every 2 px).
const SQUARE_END_ENTRY_STEP: usize = 2;
/// Arc step (px) the curve is sampled at where [`square_end`] measures
/// its stray.
const SQUARE_END_STEP: f64 = 1.0;
/// How far (px) the curve may lie off its end tangent and still count
/// as already running straight into the end.
const SQUARE_END_FLAT: f64 = 0.05;

/// The path `(start, segs)` with its END squared to the glyph drawn
/// there (report §2.3, M1), how far back along the curve (px of arc)
/// its new stretch — the last two commands — leaves it (`None`:
/// unchanged), and how deep the painter may cut it under a head and keep
/// the tip on the end point (inside the straight run it ends in,
/// [`painted_trims`]). The curve's last stretch gives way to a straight run
/// along its own end tangent into the end point — so the glyph,
/// oriented along it, points square into the side — and a bend that
/// leaves the curve smoothly some way back and joins the run, so the
/// stroke meets the glyph's base already heading its way instead of
/// still turning under it. The run is `want` px, or shorter, when the
/// bend then keeps within [`SQUARE_END_DEV`] of the curve (measured
/// outside the arrowhead; a stroke wider than an edge's within its extra
/// half width, up to [`SQUARE_END_MAX_STRAY`]). Failing that, below an arrowhead (`need`, the
/// stretch the painter cuts under it, > 0) the stroke enters the head
/// straight along a line touching the curve, onto a run just past the
/// cut — the head must face along the end tangent with its tip on the
/// end point, whatever the curve does — leaving the curve where the
/// line touches it (within [`SQUARE_END_ARC`] of the end), so the stroke
/// runs on straight, with no S or hump before the head, straying at
/// most [`SQUARE_END_MAX_STRAY`]. A curve that slips in under the head
/// first is kept up to there, joined to the run inside the head (no
/// stray at all). Last of all, under an arrowhead, the head turns with
/// a curve too tight for any of that: the stroke runs straight into the
/// end along the chord from where the curve first lies a run's length
/// away, so the tip stays on the end point. The new stretch leaves
/// the curve within `share` of its length from the end (half, when the
/// other end is squared too). Unchanged when the curve already runs
/// straight that far, or has no direction at its end, or (a curve too
/// short or too curled for even the chord) no way is found — it may then
/// be cut only within however far it already runs straight; the rest
/// of the curve is its own. `head` is (the stroke's half width, the head's
/// length) when an arrowhead covers the end.
fn square_end(
    start: (f64, f64),
    segs: Vec<Seg>,
    want: f64,
    need: f64,
    head: Option<(f64, f64)>,
    share: f64,
) -> Squared {
    // Commands without length at the end (a `step` curve's last corner
    // landing on its end point: [`curve_geometry`] then ends in a
    // zero-length `L`) have no heading: the end runs in along the last
    // command that has one, so it is squared — and its cut capped to the
    // straight run it ends in — as if they were not there. Left
    // unchanged, the path keeps them (its bytes as before).
    let live = live_len(start, &segs);
    if live < segs.len() {
        let (s, g, cut, run) = square_end(start, segs[..live].to_vec(), want, need, head, share);
        return if cut.is_some() {
            (s, g, cut, run)
        } else {
            (start, segs, None, run)
        };
    }
    #[cfg(test)]
    SQUARED.with(|c| c.set(c.get() + 1));
    // (A path with no end, or none with a direction, is left to be cut
    // as it always was.)
    let n = segs.len();
    if n == 0 || want <= 0.0 {
        return (start, segs, None, f64::INFINITY);
    }
    let mut froms = Vec::with_capacity(n);
    let mut cur = start;
    for s in &segs {
        froms.push(cur);
        cur = s.end();
    }
    let b = cur;
    let Some(d) = end_heading(froms[n - 1], segs[n - 1]) else {
        return (start, segs, None, f64::INFINITY);
    };
    // Each command's arc-length table, worked out once: every
    // candidate below cuts the curve somewhere along its tail. Measured
    // from the end back past the farthest any candidate reaches (the
    // whole path when the other end shares it): a long route's other
    // commands are never cut, and its length beyond is only ever
    // compared with what is reached (`total` stands at infinity then).
    let longest = want + SQUARE_END_BENDS[0] * GLYPH_STUB;
    let mut tabs = vec![[0.0; TRIM_STEPS + 1]; n];
    let (mut lo, mut reached) = (n, 0.0);
    while lo > 0 && (share < 1.0 || reached <= longest + 1.0) {
        lo -= 1;
        tabs[lo] = seg_lengths(froms[lo], segs[lo]);
        reached += tabs[lo][TRIM_STEPS];
    }
    let lens: Vec<f64> = tabs.iter().map(|t| t[TRIM_STEPS]).collect();
    let total: f64 = if lo == 0 {
        lens.iter().sum()
    } else {
        f64::INFINITY
    };
    let tail = tail_samples(&froms, &segs, &tabs, longest.min(total));
    // Off the end tangent (signed), and back along it from the end.
    let lat = |q: (f64, f64)| (q.0 - b.0) * d.1 - (q.1 - b.1) * d.0;
    let along = |q: (f64, f64)| (b.0 - q.0) * d.0 + (b.1 - q.1) * d.1;
    // How many samples back from the end (and how far, px of arc) the
    // curve already runs straight into it: off its end tangent by no more
    // than SQUARE_END_FLAT, and backing steadily away from the end along
    // it (a curve that doubles back on itself — a drag frame's cusp —
    // runs straight no farther).
    let (mut flat_n, mut flat_run, mut last) = (0, 0.0, -SQUARE_END_FLAT);
    for &(s, q) in &tail {
        let a = along(q);
        if lat(q).abs() > SQUARE_END_FLAT || a < last - 1e-9 {
            break;
        }
        (flat_n, flat_run, last) = (flat_n + 1, s, a);
    }
    // The deepest cut under a head that keeps its tip on the end point in
    // a straight run `run` px long that is entered at a corner (or where
    // the curve stops running straight): the rest of the run sets the
    // way the head faces ([`SQUARE_END_RUN_USE`]). One entered along a
    // smooth bend (the curve's own tangent at the join) may be cut right
    // to the join.
    let at_corner = |run: f64| run * SQUARE_END_RUN_USE;
    if flat_n >= tail.iter().take_while(|&&(s, _)| s <= want).count() {
        return (start, segs, None, at_corner(flat_run));
    }
    // The straight run into the end of a new stretch whose bend (or
    // straight line) ends at the corner `seg.end()`.
    let run_to = |seg: Seg| (b.0 - seg.end().0).hypot(b.1 - seg.end().1);
    // A point the arrowhead covers: inside its triangle (tip on the end,
    // a head long and wide) with the stroke's whole width.
    let covered = |q: (f64, f64)| {
        head.is_some_and(|(hw, len)| {
            let a = along(q);
            (0.0..=len).contains(&a) && lat(q).abs() + hw <= a / 2.0
        })
    };
    // Where command `i` ends, in arc back from the end.
    let mut back = vec![0.0; n];
    for i in (0..n - 1).rev() {
        back[i] = back[i + 1] + lens[i + 1];
    }
    // The finer arc tables of the pieces cut so far, by (command,
    // piece): the candidates below all cut the same few pieces.
    let pieces: std::cell::RefCell<Vec<(usize, usize, [f64; TRIM_STEPS + 1])>> =
        std::cell::RefCell::new(Vec::new());
    let param_at = |i: usize, len: f64| {
        seg_param_with(froms[i], segs[i], len, &tabs[i], &mut |k| {
            let mut c = pieces.borrow_mut();
            if let Some(&(_, _, f)) = c.iter().find(|&&(ci, ck, _)| (ci, ck) == (i, k)) {
                return f;
            }
            let f = piece_lengths(froms[i], segs[i], k);
            c.push((i, k, f));
            f
        })
    };
    // Where the curve is cut `s_j` px of arc back from the end (its
    // start, when shorter): the command and its parameter there, the
    // point, and the heading; `None` past this end's share or where it
    // has no heading. What is kept up to it ([`Kept`]) is built for the
    // winning candidate alone.
    let cut_at = |s_j: f64| -> Option<Kept> {
        if share < 1.0 && s_j > share * total + 1e-9 {
            return None;
        }
        let (i, u) = if s_j >= total - 1e-9 {
            (0, 0.0)
        } else {
            let i = (0..n).find(|&i| back[i] <= s_j && s_j < back[i] + lens[i])?;
            (i, param_at(i, lens[i] - (s_j - back[i])))
        };
        let j = if u > 0.0 {
            segs[i].part(froms[i], 0.0, u).1.end()
        } else {
            froms[i]
        };
        Some((i, u, j, seg_heading_at(froms[i], segs[i], u)?))
    };
    // The curve kept up to a cut ([`Kept`]), then `seg` into the run and
    // the run into the end.
    let joined = |(i, u, ..): Kept, seg: Seg| -> Vec<Seg> {
        let mut out: Vec<Seg> = Vec::with_capacity(i + 3);
        out.extend_from_slice(&segs[..i]);
        if u > 0.0 {
            out.push(segs[i].part(froms[i], 0.0, u).1);
        }
        out.extend([seg, Seg::L(b)]);
        out
    };
    // A new stretch's samples: the cubic `seg` from `j`, then the run
    // from its end into the end point.
    let samples = |j: (f64, f64), seg: Seg| -> [(f64, f64); SQUARE_END_BEND_STEPS + 3] {
        let bb = seg.cubic(j);
        let mut pts = [b; SQUARE_END_BEND_STEPS + 3];
        for (k, q) in pts.iter_mut().enumerate().take(SQUARE_END_BEND_STEPS + 1) {
            *q = cubic_at(&bb, k as f64 / SQUARE_END_BEND_STEPS as f64);
        }
        pts[SQUARE_END_BEND_STEPS + 1] = lerp(seg.end(), b, 0.5);
        pts
    };
    // How far the new stretch (sampled as `pts`) strays from the curve
    // it replaces (its last `s_j` px), outside the head: where it could
    // come nearer anything the curve kept clear of. What it leaves out
    // only gives room. `None` once it is over `limit`: every caller
    // only wants a stray within some bound (or the least of several),
    // so a candidate stops being measured the moment it cannot be had.
    let tail_pts: Vec<(f64, f64)> = tail.iter().map(|&(_, q)| q).collect();
    let tail_boxes = stray_boxes(&tail_pts);
    let stray = |pts: &[(f64, f64)], s_j: f64, limit: f64| -> Option<f64> {
        let m = tail
            .iter()
            .take_while(|&&(s, _)| s <= s_j + SQUARE_END_STEP)
            .count();
        let outside = pts.iter().copied().filter(|&q| !covered(q));
        stray_from(outside, &tail_pts[..m], &tail_boxes, limit)
    };
    // Whether it keeps within `limit` (its stray itself not wanted).
    let strays_within = |pts: &[(f64, f64)], s_j: f64, limit: f64| -> bool {
        let m = tail
            .iter()
            .take_while(|&&(s, _)| s <= s_j + SQUARE_END_STEP)
            .count();
        let outside = pts.iter().copied().filter(|&q| !covered(q));
        stray_within(outside, &tail_pts[..m], &tail_boxes, limit)
    };
    // A straight run of `run` px, the bend into it leaving the curve
    // `run + bend` px back, smooth at both ends. As (stray, commands);
    // `None` for a bend that would bulge back up from the run before
    // reaching it (a curve turning hard right at the end), or one
    // straying over `limit`.
    let smooth = |run: f64, bend: f64, limit: f64, exact: bool| -> Option<Candidate> {
        let s_j = run + bend;
        let cut = cut_at(s_j)?;
        let (j, tj) = (cut.2, cut.3);
        let p = (b.0 - d.0 * run, b.1 - d.1 * run);
        let k = (p.0 - j.0).hypot(p.1 - j.1) / 3.0;
        let bent = Seg::C(
            (j.0 + tj.0 * k, j.1 + tj.1 * k),
            (p.0 - d.0 * k, p.1 - d.1 * k),
            p,
        );
        let pts = samples(j, bent);
        if pts[..=SQUARE_END_BEND_STEPS]
            .windows(2)
            .any(|w| along(w[1]) > along(w[0]) + 1e-9)
        {
            return None;
        }
        let st = if exact {
            stray(&pts, s_j, limit)?
        } else if strays_within(&pts, s_j, limit) {
            limit
        } else {
            return None;
        };
        Some((st, s_j.min(total), cut, bent))
    };
    // A curve swinging in from the side right at its end: it is kept up
    // to where the line from a point `run_in` px up the end tangent
    // touches it (within SQUARE_END_ARC), then runs straight down that
    // line in under the head — it leaves the curve along its own heading,
    // so with no wiggle, and the corner onto the tangent hides under the
    // head — and on along the tangent into the end.
    let run_in = need + square_end_entry(need);
    let entry = || -> Option<Candidate> {
        let p = (b.0 - d.0 * run_in, b.1 - d.1 * run_in);
        let lim = SQUARE_END_ARC.min(if share < 1.0 { share * total } else { total });
        let turn = |k: usize| {
            // The curve's heading at sample k (toward the end) against the
            // line from it to `p`.
            let (q, h) = (tail[k].1, (tail[k - 1].1, tail[k + 1].1));
            let (hx, hy) = (h.0 .0 - h.1 .0, h.0 .1 - h.1 .1);
            let (lx, ly) = (p.0 - q.0, p.1 - q.1);
            let den = hx.hypot(hy) * lx.hypot(ly);
            if den <= 1e-12 {
                return f64::INFINITY;
            }
            -(hx * lx + hy * ly) / den
        };
        // Off the curve along its own heading at sample k, then straight
        // to the corner. As (stray, how far back it leaves the curve,
        // commands); `None` past `limit`.
        let at = |k: usize, limit: f64, exact: bool| -> Option<Candidate> {
            let cut = cut_at(tail[k].0)?;
            let (j, tj) = (cut.2, cut.3);
            let len = (p.0 - j.0).hypot(p.1 - j.1);
            let h = len / 3.0;
            let dir = ((p.0 - j.0) / len, (p.1 - j.1) / len);
            let into = Seg::C(
                (j.0 + tj.0 * h, j.1 + tj.1 * h),
                (p.0 - dir.0 * h, p.1 - dir.1 * h),
                p,
            );
            let pts = samples(j, into);
            let st = if exact {
                stray(&pts, tail[k].0, limit)?
            } else if strays_within(&pts, tail[k].0, limit) {
                limit
            } else {
                return None;
            };
            Some((st, tail[k].0, cut, into))
        };
        // Where the line touches the curve most nearly — the curve there
        // heading at the corner, so the stroke runs on straight, with no
        // S where it leaves the curve and no hump before the head —
        // leaving it no nearer the end along the tangent than the corner
        // (the stroke would double back towards the corner, a hook), or,
        // for a curve that never gets that far back within reach,
        // anywhere.
        let ks: Vec<usize> = (1..tail.len().saturating_sub(1))
            .filter(|&k| tail[k].0 >= run_in && tail[k].0 <= lim)
            .collect();
        let far: Vec<usize> = ks
            .iter()
            .copied()
            .filter(|&k| along(tail[k].1) >= run_in)
            .collect();
        let pool = if far.is_empty() { &ks } else { &far };
        // The point nearest the end within SQUARE_END_TOUCH of touching
        // most nearly: the shortest straight line, leaving the curve with
        // no visible S.
        let turns: Vec<f64> = pool.iter().map(|&k| turn(k)).collect();
        let most = turns.iter().copied().fold(f64::INFINITY, f64::min);
        let near = |deg: f64| {
            let slack = 1.0 - deg.to_radians().cos();
            pool.iter()
                .zip(&turns)
                .filter(move |&(_, &t)| t <= most + slack)
                .map(|(&k, _)| k)
        };
        let k = near(SQUARE_END_TOUCH).next()?;
        if let Some(c) = at(k, SQUARE_END_MAX_STRAY, false) {
            return Some(c);
        }
        // Touching it there strays too far: the nearest point to the end
        // a little farther from touching that keeps within the stray,
        // then where it strays least, to the 0.01 px (the first such of
        // least turn among equals; a candidate stops being measured once
        // its stray rounds above the best so far).
        for deg in [2.0, 4.0].map(|f| f * SQUARE_END_TOUCH) {
            if let Some(c) = near(deg).find_map(|k| at(k, SQUARE_END_MAX_STRAY, false)) {
                return Some(c);
            }
        }
        let key = |s: f64| (s * 100.0).round();
        let mut best: Option<(f64, Candidate)> = None;
        for &k in pool
            .iter()
            .step_by(SQUARE_END_ENTRY_STEP)
            .chain(pool.last())
        {
            let limit = best
                .as_ref()
                .map_or(SQUARE_END_MAX_STRAY, |b| (key(b.1 .0) + 0.51) / 100.0);
            let Some(c) = at(k, limit, true) else {
                continue;
            };
            let tk = turn(k);
            let better = best.as_ref().map_or(true, |b| {
                key(c.0)
                    .total_cmp(&key(b.1 .0))
                    .then(tk.total_cmp(&b.0))
                    .is_lt()
            });
            if better {
                best = Some((tk, c));
            }
        }
        best.map(|(_, c)| c)
    };
    // The first bend that keeps within the stray with a run longer than
    // the head's own cut (the stroke reaching the head's base already
    // heading square — what the run is for), longest run first.
    let runs: Vec<f64> = SQUARE_END_RUNS
        .iter()
        .map(|f| f * want)
        .filter(|&r| r > need)
        .collect();
    // A stroke wider than an edge's may stray by the extra half width
    // (up to the most any squared end strays): its own ink already
    // covers that much round the curve.
    let dev = head.map_or(SQUARE_END_DEV, |(hw, _)| {
        (hw - EDGE_WIDTH / 2.0).clamp(SQUARE_END_DEV, SQUARE_END_MAX_STRAY)
    });
    let bent = runs.iter().find_map(|&run| {
        SQUARE_END_BENDS
            .iter()
            .find_map(|&bend| smooth(run, bend * GLYPH_STUB, dev, false))
    });
    if let Some((_, cut, at, seg)) = bent {
        return (start, joined(at, seg), Some(cut), run_to(seg));
    }
    if need <= 0.0 {
        return (start, segs, None, at_corner(flat_run));
    }
    // Under an arrowhead, a curve that turns square only in its last px
    // (a hook: an S-cubic whose end handle is short for its sideways
    // reach) mostly slips in under the head first. It is kept up to the
    // point nearest the end, a run's length or more up the tangent,
    // where its stroke lies wholly inside the head, and joined there to
    // the run — straight, so the joins stay inside the triangle, where
    // the head's ink hides them. Outside the head the stroke is the
    // scene curve itself, curling into the head as it always has, and
    // the head still faces along the end tangent with its tip on the end
    // point.
    if let Some((hw, len)) = head {
        let inside = |q: (f64, f64)| {
            let a = along(q);
            (run_in..=len).contains(&a) && lat(q).abs() + hw <= a / 2.0
        };
        let under = (1..tail.len())
            .filter(|&k| tail[k].0 >= run_in)
            .find(|&k| inside(tail[k].1))
            .and_then(|k| Some((tail[k].0, cut_at(tail[k].0)?)));
        if let Some((s_j, cut)) = under {
            let p = (b.0 - d.0 * run_in, b.1 - d.1 * run_in);
            if inside(cut.2) {
                return (
                    start,
                    joined(cut, Seg::L(p)),
                    Some(s_j.min(total)),
                    at_corner(run_in),
                );
            }
        }
    }
    // Under an arrowhead, else: the curve turns too hard over its last
    // stretch to meet a run within a px. The straight entry — a bend into
    // the cut alone would turn all the remaining turn under and beside
    // the head, a visible bulge; the entry leaves the curve where it
    // touches it, and strays (up to a few px, for a curve swinging in
    // from the side) only within a head or two of the tip. A curve too
    // short for it: the bend into the cut straying least.
    if let Some((_, cut, at, seg)) = entry() {
        return (start, joined(at, seg), Some(cut), at_corner(run_to(seg)));
    }
    // The first of least stray (a later one must stray less to win).
    let mut best: Option<Candidate> = None;
    for &bend in &SQUARE_END_BENDS {
        let limit = best.as_ref().map_or(SQUARE_END_MAX_STRAY, |b| b.0);
        if let Some(c) = smooth(need, bend * GLYPH_STUB, limit, true) {
            if best.as_ref().map_or(true, |b| c.0 < b.0) {
                best = Some(c);
            }
        }
    }
    if let Some((_, cut, at, seg)) = best {
        return (start, joined(at, seg), Some(cut), run_to(seg));
    }
    // Under an arrowhead, last of all: the curve turns too hard over too
    // short a stretch for any run along its end tangent to keep within
    // the stray (a back-edge bow's short stub, a routed turn just short
    // of the end, a tight S, under a very wide stroke's long cut). The
    // head turns with the curve instead of leaving it: the curve is kept
    // up to where it first lies a run's length (and half a px) from the
    // end, going back, and runs straight from there into the end — so
    // the stroke ends along that chord, and the head, facing along it,
    // keeps its tip on the end point (cut along the curve, the head would
    // face along the curve's tangent at the cut, its tip off the end by
    // up to the cut's length). Never a head facing back out of its end.
    // The new stretch lies within a run and a px of the tip, inside the
    // head's reach ([`SQUARE_END_HEAD_REACH`]), and the stretch it
    // replaces within that of the end ([`SQUARE_END_REACH`]).
    let far_enough =
        |q: (f64, f64)| (q.0 - b.0).hypot(q.1 - b.1) >= run_in + SQUARE_END_CHORD_SLACK;
    let chord = || -> Option<(f64, Kept, Seg)> {
        let k = tail.iter().position(|&(_, q)| far_enough(q))?;
        let (mut lo, mut hi) = (tail[k - 1].0, tail[k].0);
        let mut cut = cut_at(hi)?;
        for _ in 0..SQUARE_END_CHORD_STEPS {
            let mid = (lo + hi) / 2.0;
            match cut_at(mid) {
                Some(c) if far_enough(c.2) => (hi, cut) = (mid, c),
                _ => lo = mid,
            }
        }
        let j = cut.2;
        let l = (b.0 - j.0).hypot(b.1 - j.1);
        let u = ((b.0 - j.0) / l, (b.1 - j.1) / l);
        if u.0 * d.0 + u.1 * d.1 <= 0.0 {
            return None;
        }
        let p = (b.0 - u.0 * run_in, b.1 - u.1 * run_in);
        Some((hi.min(total), cut, Seg::L(p)))
    };
    if need > 0.0 {
        if let Some((cut, at, seg)) = chord() {
            return (start, joined(at, seg), Some(cut), at_corner(run_to(seg)));
        }
    }
    (start, segs, None, at_corner(flat_run))
}

/// Part of a straight run entered at a corner (or where the curve stops
/// running straight) that the painter may cut under a head
/// ([`square_end`], [`painted_trims`]): the fifth left over sets the way
/// the head faces — at least a quarter of the cut, as the run past the
/// usual cut is ([`square_end_entry`]), so the SVG's 0.1 px rounding
/// keeps the tip within about 0.3 px of the end point. A cut right at
/// the corner would leave the head facing along whatever comes before
/// it (a step curve's last corner, a cusp mid-drag).
const SQUARE_END_RUN_USE: f64 = 0.8;

/// How much farther (px) than its run from the end [`square_end`]'s last
/// resort leaves the curve, so the chord into the end has a stretch
/// before the run.
const SQUARE_END_CHORD_SLACK: f64 = 0.5;
/// Halvings [`square_end`]'s last resort narrows where the curve first
/// lies far enough from the end by (to well under 0.01 px of arc).
const SQUARE_END_CHORD_STEPS: usize = 12;

#[cfg(test)]
thread_local! {
    /// How many ends [`square_end`] has squared on this thread: the
    /// tests hold the painter and picking to squaring an end only where
    /// it must.
    static SQUARED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Where [`square_end`] cuts a curve to keep it up to there: the
/// command, its parameter, the point, and the unit heading there.
type Kept = (usize, f64, (f64, f64), (f64, f64));

/// A path [`square_end`] squared at its end: its start, its commands,
/// how far back (px of arc) the new stretch leaves the curve (`None`:
/// unchanged), and how deep (px) the painter may cut it under a head and
/// keep the tip on the end point (in the straight run it ends in).
type Squared = ((f64, f64), Vec<Seg>, Option<f64>, f64);

/// A new stretch [`square_end`] weighs: its stray, how far back (px of
/// arc) it leaves the curve, where it cuts it, and the cubic from the
/// cut into the straight run.
type Candidate = (f64, f64, Kept, Seg);

/// Segments of a sampled tail [`stray_from`] boxes together.
const STRAY_BLOCK: usize = 8;

/// The bounding box `[x0, y0, x1, y1]` of each run of [`STRAY_BLOCK`]
/// segments of the polyline `pts` (block `b` holds segments
/// `b * STRAY_BLOCK ..`), for [`stray_from`] to pass over whole runs at
/// once.
fn stray_boxes(pts: &[(f64, f64)]) -> Vec<[f64; 4]> {
    let nseg = pts.len().saturating_sub(1);
    (0..nseg.div_ceil(STRAY_BLOCK))
        .map(|b| {
            let hi = ((b + 1) * STRAY_BLOCK).min(nseg);
            pts[b * STRAY_BLOCK..=hi].iter().fold(
                [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ],
                |[x0, y0, x1, y1], q| [x0.min(q.0), y0.min(q.1), x1.max(q.0), y1.max(q.1)],
            )
        })
        .collect()
}

/// The largest distance from one of `pts` to the polyline `old`
/// (infinity for one of fewer than two points), or `None` once it is
/// over `limit` — exactly the max of the min distances, found without
/// measuring every point against every segment. `boxes` are
/// [`stray_boxes`] of a polyline `old` is a leading part of (a box of
/// more points still bounds the ones `old` keeps). A point that lies
/// within the largest distance so far of some segment cannot raise it,
/// and the segment that was nearest the last point is tried first (the
/// points run along `old`); only a point farther than that from all of
/// `old` is measured to the end, skipping each run of segments whose
/// box is no nearer than the best so far.
fn stray_from(
    pts: impl Iterator<Item = (f64, f64)>,
    old: &[(f64, f64)],
    boxes: &[[f64; 4]],
    limit: f64,
) -> Option<f64> {
    stray_scan(pts, old, boxes, limit, true)
}

/// Whether every one of `pts` lies within `limit` of the polyline `old`
/// ([`stray_from`] of them is at most `limit`): a point is cleared by
/// the first segment found within `limit`, so this costs far less than
/// measuring the stray when only the bound matters.
fn stray_within(
    pts: impl Iterator<Item = (f64, f64)>,
    old: &[(f64, f64)],
    boxes: &[[f64; 4]],
    limit: f64,
) -> bool {
    stray_scan(pts, old, boxes, limit, false).is_some()
}

/// [`stray_from`] (`exact`), or [`stray_within`] (else: `Some(limit)`
/// when every point is within `limit`, a point clearing as soon as a
/// segment within `limit` of it turns up).
fn stray_scan(
    pts: impl Iterator<Item = (f64, f64)>,
    old: &[(f64, f64)],
    boxes: &[[f64; 4]],
    limit: f64,
    exact: bool,
) -> Option<f64> {
    let nseg = old.len().saturating_sub(1);
    // Compared squared (no root per segment); the distance a point
    // raises the max to is measured as `point_seg_dist` measures it.
    let seg2 = |q: (f64, f64), k: usize| {
        let (a, b) = (old[k], old[k + 1]);
        let (abx, aby) = (b.0 - a.0, b.1 - a.1);
        let len2 = abx * abx + aby * aby;
        let t = if len2 <= f64::EPSILON {
            0.0
        } else {
            (((q.0 - a.0) * abx + (q.1 - a.1) * aby) / len2).clamp(0.0, 1.0)
        };
        let (dx, dy) = (a.0 + t * abx - q.0, a.1 + t * aby - q.1);
        dx * dx + dy * dy
    };
    let (mut worst, mut worst2): (f64, f64) = (0.0, 0.0);
    // A point within this (squared) of a segment cannot change the
    // answer.
    let clear2 = |worst2: f64| if exact { worst2 } else { limit * limit };
    let mut hint = 0;
    for q in pts {
        if nseg == 0 {
            worst = f64::INFINITY;
            break;
        }
        let mut best2 = seg2(q, hint);
        if best2 <= clear2(worst2) {
            continue;
        }
        let mut arg = hint;
        'blocks: for (b, bx) in boxes.iter().enumerate() {
            let lo = b * STRAY_BLOCK;
            if lo >= nseg {
                break;
            }
            let gx = (bx[0] - q.0).max(q.0 - bx[2]).max(0.0);
            let gy = (bx[1] - q.1).max(q.1 - bx[3]).max(0.0);
            if gx * gx + gy * gy >= best2 {
                continue;
            }
            for k in lo..(lo + STRAY_BLOCK).min(nseg) {
                let d2 = seg2(q, k);
                if d2 < best2 {
                    (best2, arg) = (d2, k);
                    if best2 <= clear2(worst2) {
                        break 'blocks;
                    }
                }
            }
        }
        hint = arg;
        if best2 > clear2(worst2) {
            worst2 = best2;
            worst = worst.max(point_seg_dist(q.0, q.1, old[arg], old[arg + 1]));
            if worst > limit {
                return None;
            }
        }
    }
    if !exact {
        return (worst <= limit).then_some(limit);
    }
    (worst <= limit).then_some(worst)
}

/// How many of the commands `segs` (from `start`) are left once those
/// without length at the end are dropped (every control point on the
/// point it starts from, as a `step` curve's last zero-length `L`).
fn live_len(start: (f64, f64), segs: &[Seg]) -> usize {
    let flat = |from: (f64, f64), s: Seg| {
        let at = |q: (f64, f64)| (q.0 - from.0).hypot(q.1 - from.1) <= 1e-9;
        match s {
            Seg::L(p) => at(p),
            Seg::Q(c, p) => at(c) && at(p),
            Seg::C(c1, c2, p) => at(c1) && at(c2) && at(p),
        }
    };
    let mut n = segs.len();
    while n > 0 {
        let from = if n >= 2 { segs[n - 2].end() } else { start };
        if !flat(from, segs[n - 1]) {
            break;
        }
        n -= 1;
    }
    n
}

/// The unit heading of `seg` (from `from`) INTO its end, read off the
/// nearest control point that is not the end itself; `None` for a
/// command without length.
fn end_heading(from: (f64, f64), seg: Seg) -> Option<(f64, f64)> {
    let p = seg.end();
    let ctl: Vec<(f64, f64)> = match seg {
        Seg::L(_) => vec![from],
        Seg::Q(c, _) => vec![c, from],
        Seg::C(c1, c2, _) => vec![c2, c1, from],
    };
    let q = ctl
        .into_iter()
        .find(|q| (p.0 - q.0).hypot(p.1 - q.1) > 1e-9)?;
    let l = (p.0 - q.0).hypot(p.1 - q.1);
    Some(((p.0 - q.0) / l, (p.1 - q.1) / l))
}

/// The unit heading of `seg` (from `from`) at parameter `u`, forward;
/// `None` where it has no direction.
fn seg_heading_at(from: (f64, f64), seg: Seg, u: f64) -> Option<(f64, f64)> {
    let b = seg.cubic(from);
    let v = 1.0 - u;
    let mut t = (
        3.0 * (v * v * (b[1].0 - b[0].0)
            + 2.0 * u * v * (b[2].0 - b[1].0)
            + u * u * (b[3].0 - b[2].0)),
        3.0 * (v * v * (b[1].1 - b[0].1)
            + 2.0 * u * v * (b[2].1 - b[1].1)
            + u * u * (b[3].1 - b[2].1)),
    );
    if t.0.hypot(t.1) < 1e-9 {
        // A cusp or a coincident control point: the chord either side.
        let (a, c) = (
            cubic_at(&b, (u - 1e-3).max(0.0)),
            cubic_at(&b, (u + 1e-3).min(1.0)),
        );
        t = (c.0 - a.0, c.1 - a.1);
    }
    let l = t.0.hypot(t.1);
    (l > 1e-12).then(|| (t.0 / l, t.1 / l))
}

/// The last `reach` px of arc of a path (`froms[i]` where command `i`
/// starts, `tabs[i]` its [`seg_lengths`]), end first, as (arc back from
/// the end, point) every [`SQUARE_END_STEP`] px or so — the end itself
/// and the stretch's far end included.
fn tail_samples(
    froms: &[(f64, f64)],
    segs: &[Seg],
    tabs: &[[f64; TRIM_STEPS + 1]],
    reach: f64,
) -> Vec<(f64, (f64, f64))> {
    let n = segs.len();
    let mut out = vec![(0.0, segs[n - 1].end())];
    let mut back = 0.0;
    for i in (0..n).rev() {
        let left = reach - back;
        if left <= 0.0 {
            break;
        }
        let len = tabs[i][TRIM_STEPS];
        if len <= 0.0 {
            continue;
        }
        let u0 = if left >= len {
            0.0
        } else {
            seg_param_in(froms[i], segs[i], len - left, &tabs[i])
        };
        let steps = (left.min(len) / SQUARE_END_STEP).ceil().max(1.0) as usize;
        let b = segs[i].cubic(froms[i]);
        let (mut prev, mut s) = (segs[i].end(), back);
        for k in 1..=steps {
            let q = cubic_at(&b, 1.0 - (1.0 - u0) * k as f64 / steps as f64);
            s += (q.0 - prev.0).hypot(q.1 - prev.1);
            out.push((s, q));
            prev = q;
        }
        back += len;
    }
    out
}

/// The commands of a smooth B-spline (d3's `curveBasis` — what mermaid
/// uses) through `pts` (>= 2 points): starts and ends exactly at the
/// endpoints while only *approximating* the channel waypoints, so a
/// routed edge flows in gentle lanes instead of ballooning through every
/// point the way an interpolating spline would.
fn spline_segs(pts: &[(f64, f64)]) -> ((f64, f64), Vec<Seg>) {
    let n = pts.len();
    if n < 3 {
        return (pts[0], vec![Seg::L(pts[n - 1])]);
    }
    let mut segs = Vec::with_capacity(n + 1);
    segs.push(Seg::L((
        (5.0 * pts[0].0 + pts[1].0) / 6.0,
        (5.0 * pts[0].1 + pts[1].1) / 6.0,
    )));
    let bez = |a: (f64, f64), b: (f64, f64), p: (f64, f64)| {
        Seg::C(
            ((2.0 * a.0 + b.0) / 3.0, (2.0 * a.1 + b.1) / 3.0),
            ((a.0 + 2.0 * b.0) / 3.0, (a.1 + 2.0 * b.1) / 3.0),
            ((a.0 + 4.0 * b.0 + p.0) / 6.0, (a.1 + 4.0 * b.1 + p.1) / 6.0),
        )
    };
    for i in 2..n {
        segs.push(bez(pts[i - 2], pts[i - 1], pts[i]));
    }
    segs.push(bez(pts[n - 2], pts[n - 1], pts[n - 1]));
    segs.push(Seg::L(pts[n - 1]));
    (pts[0], segs)
}

/// The SVG path `d` an edge is DRAWN with, mapped through `t` (scene →
/// output coordinates): the commands of [`drawn_segs`]. Every painter
/// of a [`SceneEdge`] goes through this one builder: the router
/// delivers a rerouted edge in `waypoints` and leaves `bezier` a
/// degenerate `[p0, p0, p3, p3]` chord, so a painter that strokes
/// `bezier` alone draws it straight through the boxes it was routed
/// around. [`edge_polyline`] samples exactly this curve (picking) and
/// [`edge_end_chord`] reads its ends (glyph placement). The flowchart
/// painter strokes it less the stretch under an arrowhead
/// ([`painted_path_d`]).
pub(crate) fn edge_path_d(e: &SceneEdge, t: impl Fn((f64, f64)) -> (f64, f64)) -> String {
    let (start, segs) = drawn_segs(e, t);
    curve_d(start, &segs, |p| p)
}

/// How far (px) the flowchart painter stops `e`'s stroke short of each
/// end, as (source, target): [`arrow_trim`] of its width under an
/// arrowhead, so the stroke's butt end hides inside it; nothing at a
/// cross, a circle or a bare end (the stroke runs on to the border
/// beneath those), nor on an edge an ER or class painter strokes (they
/// draw their own ends).
pub(crate) fn arrow_trims(e: &SceneEdge) -> (f64, f64) {
    let at = |m: Option<Marker>| {
        if edge_drawn(e) && e.paint_width.is_none() && m == Some(Marker::Arrow) {
            arrow_trim(edge_stroke_width(e))
        } else {
            0.0
        }
    };
    (
        at(e.ends.start_marker(e.kind)),
        at(e.ends.end_marker(e.kind)),
    )
}

/// How far (px) the flowchart painter stops `e`'s stroke `p` short of
/// each end, as (source, target): [`arrow_trims`], cut no deeper than the
/// squared end allows ([`Runs`]). The head faces the way the stroke heads
/// where it stops and has its tip the cut's length on along that line, so
/// a cut inside the straight run the end runs in on puts the tip on the
/// end point. Only a curve too short or too curled for [`square_end`] to
/// give it a run longer than its cut (nodes all but touching mid-drag, a
/// self-loop smaller than its very wide stroke, a step curve's short last
/// leg) is cut less — the stroke's butt may then show round the head, but
/// the tip stays on the end.
fn painted_trims(e: &SceneEdge, p: &PaintedEdge) -> (f64, f64) {
    // (Cut to the 0.1 px below what is allowed: the cut names the head's
    // marker.)
    let cap = |trim: f64, run: f64| {
        if run < trim - 1e-6 {
            ((run * 10.0).floor() / 10.0).max(0.0)
        } else {
            trim
        }
    };
    let (t0, t3) = arrow_trims(e);
    (cap(t0, p.runs[0]), cap(t3, p.runs[1]))
}

/// [`painted_path_d`] of `e`, its stroke worked out afresh.
#[cfg(test)]
fn edge_path_d_painted(e: &SceneEdge, t: impl Fn((f64, f64)) -> (f64, f64)) -> String {
    painted_path_d(e, &painted_edge(e), t)
}

/// The path `d` the flowchart painter strokes for `e`, mapped through
/// `t`: [`edge_path_d`] (here from `e`'s [`PaintedEdge`] `p`) cut back
/// by [`arrow_trims`] at each end, the cut taken off the drawn curve by
/// arc length. Painter-only — picking and the scene keep the whole
/// curve (a host draws its own heads), and a curve too short to lose
/// that much is stroked whole. An edge with neither end squared is
/// drawn off its scene curve mapped through `t` first ([`plain_segs`]),
/// as the painter always drew it.
#[cfg(test)]
fn painted_path_d(e: &SceneEdge, p: &PaintedEdge, t: impl Fn((f64, f64)) -> (f64, f64)) -> String {
    painted_path(e, p, t).0
}

/// [`painted_path_d`], and the drawn curve's length (px) when it is too
/// short to lose its cuts and is stroked whole (`None`: cut as usual) —
/// its heads then shrink onto it ([`head_sizes`]). Too short for its
/// full cuts ([`arrow_trims`]) means whole even where its ends allow
/// less ([`painted_trims`]): its heads shrink onto it all the same.
fn painted_path(
    e: &SceneEdge,
    p: &PaintedEdge,
    t: impl Fn((f64, f64)) -> (f64, f64),
) -> (String, Option<f64>) {
    let (start, segs) = if p.cuts == [None, None] {
        plain_segs(e, t)
    } else {
        (t(p.start), p.segs.iter().map(|s| s.map(&t)).collect())
    };
    let (t0, t3) = painted_trims(e, p);
    let (a0, a3) = arrow_trims(e);
    if (t0, t3) != (a0, a3) {
        if let (_, _, Some(len)) = trim_segs_or_len(start, segs.clone(), a0, a3) {
            return (curve_d(start, &segs, |p| p), Some(len));
        }
    }
    let (start, segs, whole) = trim_segs_or_len(start, segs, t0, t3);
    (curve_d(start, &segs, |p| p), whole)
}

/// How far (px) the painter stops a stroke `width` px wide short of an
/// arrow tip: [`ARROW_TRIM`], or the width itself for a stroke wider
/// than that (a `linkStyle stroke-width`) — a head is as wide as it is
/// long, so `width` back from the tip it covers a butt end that wide;
/// its head grows to hold it ([`arrow_head_len`]).
pub(crate) fn arrow_trim(width: f64) -> f64 {
    width.max(ARROW_TRIM)
}

/// Length (px, and width across its base) of the arrowhead over a
/// stroke the painter stops `trim` px short of its tip ([`arrow_trim`]):
/// [`ARROW_MARKER_PX`], or — over a stroke so wide its cut is its own
/// width (a `linkStyle stroke-width`) — long enough to hold the butt end
/// at the cut, the corner onto the straight run past it
/// ([`square_end_entry`]), and half the stroke beyond that, where the
/// stroke's join reaches: `trim + square_end_entry(trim) + trim / 2`
/// (14 px for an 8 px stroke, 42 px for 24 px). So a stroke wider than half a
/// head ends inside its head instead of in a butt as wide as it (or
/// wider), notched where it joins the run. Painter-only, like the head.
pub(crate) fn arrow_head_len(trim: f64) -> f64 {
    if trim <= ARROW_TRIM {
        ARROW_MARKER_PX
    } else {
        ARROW_MARKER_PX.max(trim + square_end_entry(trim) + trim / 2.0)
    }
}

/// [`arrow_head_len`] of edge `e`'s arrowheads (both ends share its
/// stroke), or [`ARROW_MARKER_PX`] for an edge the flowchart painter
/// draws no head on (an ER or class painter draws its own ends).
pub(crate) fn edge_head_len(e: &SceneEdge) -> f64 {
    if e.paint_width.is_some() {
        return ARROW_MARKER_PX;
    }
    arrow_head_len(arrow_trim(edge_stroke_width(e)))
}

/// The `<marker>` def of the end arrowhead with id `id`, painted in
/// `color`, for a path stopping `trim` px short of the tip
/// ([`arrow_trim`] of its width, [`polyline_d_painted`]), as long as
/// that cut needs ([`arrow_head_len`]): the flowchart's own glyph, so
/// the advance painter's heads match it ([`marker_def`]).
pub(crate) fn arrow_marker_def(id: &str, color: &str, trim: f64) -> String {
    let len = arrow_head_len(trim);
    marker_def_sized(id, Marker::Arrow, false, color, trim, len, len)
}

/// SVG path `d` of the polyline `pts` (straight `L` legs, as the
/// advance painter draws its routes) cut `trim_end` short of its last
/// point by arc length — [`ARROW_TRIM`] under an [`arrow_marker_def`]
/// head, 0 for a bare end; whole when shorter than the cut and a pixel.
/// The head faces along the last leg: a last leg shorter than the cut
/// and its run ([`square_end_entry`]) would end the stroke on the leg
/// before it, and `orient="auto"` would tilt the head that way, so the
/// route is kept up to that far back and runs straight from there to a
/// point that far up the last leg's line, and on along it (the corner
/// under the head, as [`square_end`] squares a flowchart end).
pub(crate) fn polyline_d_painted(pts: &[(f64, f64)], trim_end: f64) -> String {
    if pts.len() < 2 {
        return String::new();
    }
    let segs: Vec<Seg> = pts[1..].iter().map(|&p| Seg::L(p)).collect();
    let tip = pts[pts.len() - 1];
    let run = trim_end + square_end_entry(trim_end);
    let leg_from = pts[..pts.len() - 1]
        .iter()
        .rev()
        .find(|q| (tip.0 - q.0).hypot(tip.1 - q.1) > 1e-9);
    if let Some(&q) = leg_from.filter(|_| trim_end > 0.0) {
        let leg = (tip.0 - q.0).hypot(tip.1 - q.1);
        if leg < run {
            let d = ((tip.0 - q.0) / leg, (tip.1 - q.1) / leg);
            // (Left whole, a route too short to lose that much is stroked
            // whole as before.)
            let (start, mut kept) = trim_segs(pts[0], segs.clone(), 0.0, run);
            if kept.last().map(|g| g.end()) != Some(tip) {
                kept.push(Seg::L((tip.0 - d.0 * run, tip.1 - d.1 * run)));
                kept.push(Seg::L(tip));
                let (start, kept) = trim_segs(start, kept, 0.0, trim_end);
                return curve_d(start, &kept, |p| p);
            }
        }
    }
    let (start, segs) = trim_segs(pts[0], segs, 0.0, trim_end);
    curve_d(start, &segs, |p| p)
}

/// Arc length of `seg` from `from`, from its parameter 0 up to each of
/// [`TRIM_STEPS`] even steps (`out[k]` at `k / TRIM_STEPS`).
fn seg_lengths(from: (f64, f64), seg: Seg) -> [f64; TRIM_STEPS + 1] {
    let b = seg.cubic(from);
    let mut out = [0.0; TRIM_STEPS + 1];
    let mut prev = from;
    for (k, o) in out.iter_mut().enumerate().skip(1) {
        let q = cubic_at(&b, k as f64 / TRIM_STEPS as f64);
        *o = (q.0 - prev.0).hypot(q.1 - prev.1);
        prev = q;
    }
    for k in 1..=TRIM_STEPS {
        out[k] += out[k - 1];
    }
    out
}

/// Pieces a command is measured in when a path is cut by arc length:
/// on the short curves near an end, well under a tenth of a px off.
const TRIM_STEPS: usize = 32;

/// The parameter at which `seg` from `from` has run `len` of arc length
/// (clamped to the command).
fn seg_param_at(from: (f64, f64), seg: Seg, len: f64) -> f64 {
    if let Seg::L(p) = seg {
        let l = (p.0 - from.0).hypot(p.1 - from.1);
        return if l > 0.0 {
            (len / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    seg_param_in(from, seg, len, &seg_lengths(from, seg))
}

/// [`seg_param_at`] of a command whose [`seg_lengths`] are `lens`
/// (worked out once for a command measured many times).
fn seg_param_in(from: (f64, f64), seg: Seg, len: f64, lens: &[f64; TRIM_STEPS + 1]) -> f64 {
    seg_param_with(from, seg, len, lens, &mut |k| piece_lengths(from, seg, k))
}

/// Arc length of piece `k` (1-based, of [`TRIM_STEPS`]) of `seg` from
/// `from`, from its start up to each of [`TRIM_STEPS`] finer steps.
fn piece_lengths(from: (f64, f64), seg: Seg, k: usize) -> [f64; TRIM_STEPS + 1] {
    let b = seg.cubic(from);
    let (t0, t1) = (
        (k - 1) as f64 / TRIM_STEPS as f64,
        k as f64 / TRIM_STEPS as f64,
    );
    let mut fine = [0.0; TRIM_STEPS + 1];
    let mut prev = cubic_at(&b, t0);
    for (j, f) in fine.iter_mut().enumerate().skip(1) {
        let q = cubic_at(&b, t0 + (t1 - t0) * j as f64 / TRIM_STEPS as f64);
        *f = (q.0 - prev.0).hypot(q.1 - prev.1);
        prev = q;
    }
    for j in 1..=TRIM_STEPS {
        fine[j] += fine[j - 1];
    }
    fine
}

/// [`seg_param_in`], piece `k`'s [`piece_lengths`] given by `piece(k)`
/// (a caller cutting one curve many times keeps them).
fn seg_param_with(
    from: (f64, f64),
    seg: Seg,
    len: f64,
    lens: &[f64; TRIM_STEPS + 1],
    piece: &mut dyn FnMut(usize) -> [f64; TRIM_STEPS + 1],
) -> f64 {
    if let Seg::L(p) = seg {
        let l = (p.0 - from.0).hypot(p.1 - from.1);
        return if l > 0.0 {
            (len / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    let k = lens.partition_point(|&l| l < len).clamp(1, TRIM_STEPS);
    let (l0, l1) = (lens[k - 1], lens[k]);
    if l1 <= l0 {
        return (k - 1) as f64 / TRIM_STEPS as f64;
    }
    // Within the piece, by its own finer pieces (their lengths scaled to
    // the piece's): the speed along a long cubic's piece varies, and a
    // straight interpolation there is off by a tenth of a px.
    let (t0, t1) = (
        (k - 1) as f64 / TRIM_STEPS as f64,
        k as f64 / TRIM_STEPS as f64,
    );
    let fine = piece(k);
    let scale = (l1 - l0) / fine[TRIM_STEPS].max(f64::MIN_POSITIVE);
    let want = (len - l0) / scale;
    let j = fine.partition_point(|&l| l < want).clamp(1, TRIM_STEPS);
    let (f0, f1) = (fine[j - 1], fine[j]);
    let f = if f1 > f0 {
        (want - f0) / (f1 - f0)
    } else {
        0.0
    };
    t0 + (t1 - t0) * ((j - 1) as f64 + f.clamp(0.0, 1.0)) / TRIM_STEPS as f64
}

/// The path (start, commands) with `t0` of arc length cut off its start
/// and `t3` off its end; unchanged when it is no longer than both cuts
/// plus a pixel.
fn trim_segs(start: (f64, f64), segs: Vec<Seg>, t0: f64, t3: f64) -> ((f64, f64), Vec<Seg>) {
    let (start, segs, _) = trim_segs_or_len(start, segs, t0, t3);
    (start, segs)
}

/// [`trim_segs`], and the path's length when it is too short to lose
/// both cuts and a pixel and comes back whole (`None` when cut, or when
/// there is nothing to cut).
fn trim_segs_or_len(
    start: (f64, f64),
    segs: Vec<Seg>,
    t0: f64,
    t3: f64,
) -> ((f64, f64), Vec<Seg>, Option<f64>) {
    if t0 <= 0.0 && t3 <= 0.0 || segs.is_empty() {
        return (start, segs, None);
    }
    let mut froms = Vec::with_capacity(segs.len());
    let mut cur = start;
    for s in &segs {
        froms.push(cur);
        cur = s.end();
    }
    // Each command's length, measured only once a cut or the length
    // check below reaches it: a cut takes a few px off a long path.
    let mut known: Vec<Option<f64>> = vec![None; segs.len()];
    let mut len_of =
        |i: usize| *known[i].get_or_insert_with(|| seg_lengths(froms[i], segs[i])[TRIM_STEPS]);
    // Longer than both cuts and a pixel, summing from both ends until
    // that is plain (the lengths are never negative).
    let need = t0 + t3 + 1.0;
    let (mut sum, mut lo, mut hi) = (0.0, 0, segs.len());
    while sum <= need && lo < hi {
        sum += len_of(lo);
        lo += 1;
        if sum <= need && lo < hi {
            hi -= 1;
            sum += len_of(hi);
        }
    }
    if sum <= need {
        // Every command was summed: the whole length.
        return (start, segs, Some(sum));
    }
    // (command index, parameter) where the kept part starts and ends.
    let (mut i0, mut u0) = (0, 0.0);
    let mut left = t0;
    while i0 < segs.len() && left > 0.0 {
        let l = len_of(i0);
        if l > left {
            u0 = seg_param_at(froms[i0], segs[i0], left);
            break;
        }
        left -= l;
        i0 += 1;
    }
    let (mut i3, mut u3) = (segs.len() - 1, 1.0);
    let mut left = t3;
    while left > 0.0 {
        let l = len_of(i3);
        if l > left {
            u3 = seg_param_at(froms[i3], segs[i3], l - left);
            break;
        }
        left -= l;
        i3 -= 1;
    }
    let mut out = Vec::with_capacity(i3 + 1 - i0);
    let mut first = start;
    for i in i0..=i3 {
        let (a, b) = (
            if i == i0 { u0 } else { 0.0 },
            if i == i3 { u3 } else { 1.0 },
        );
        if a == 0.0 && b == 1.0 {
            out.push(segs[i]);
        } else {
            let (from, part) = segs[i].part(froms[i], a, b);
            if i == i0 {
                first = from;
            }
            out.push(part);
        }
        if i == i0 && a == 0.0 {
            first = froms[i];
        }
    }
    (first, out, None)
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

/// One end of an edge's DRAWN curve as `(tip, heading)`: the end point
/// and the unit direction its end glyph faces, the way an
/// `orient="auto"` marker reads it off the path the flowchart painter
/// strokes ([`painted_path_d`], cut short under an arrowhead) — the
/// curve's heading INTO the end at the target, the reverse of the way it
/// sets off at the source. Read off the control point next to the
/// stroke's end (a coincident one defers to the next): along the
/// scene curve's own end tangent wherever the end is squared to its
/// glyph ([`square_end`]), else the last command of a mermaid `curve`,
/// the closing `L` of the `curveBasis` spline, the cubic's end tangent.
/// `(0, 0)` only for a curve without length.
pub(crate) fn edge_end_tangent(e: &SceneEdge, at_start: bool) -> ((f64, f64), (f64, f64)) {
    let p = painted_edge(e);
    let (t0, t3) = painted_trims(e, &p);
    let (start, segs) = (p.start, p.segs);
    let tip = if at_start {
        start
    } else {
        segs.last().map_or(start, |s| s.end())
    };
    let (start, segs) = trim_segs(start, segs, t0, t3);
    // The stroke's end, then the points that set the tangent there,
    // nearest first.
    let mut seq = vec![start];
    for seg in &segs {
        match *seg {
            Seg::L(p) => seq.push(p),
            Seg::Q(c, p) => seq.extend([c, p]),
            Seg::C(c1, c2, p) => seq.extend([c1, c2, p]),
        }
    }
    if !at_start {
        seq.reverse();
    }
    let end = seq[0];
    let toward = seq[1..]
        .iter()
        .find(|q| (q.0 - end.0).hypot(q.1 - end.1) > 1e-9)
        .copied()
        .unwrap_or(end);
    let (dx, dy) = (end.0 - toward.0, end.1 - toward.1);
    let l = dx.hypot(dy);
    if l == 0.0 {
        return (tip, (0.0, 0.0));
    }
    (tip, (dx / l, dy / l))
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
/// [`spline_segs`] draws through `pts`, so hit geometry == drawn geometry.
pub(crate) fn basis_sample(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = pts.len();
    if n < 3 {
        return vec![pts[0], pts[n - 1]];
    }
    let mut out = vec![pts[0]];
    // Straight run into the first control point (mirrors spline_segs' `L`).
    let mut cur = (
        (5.0 * pts[0].0 + pts[1].0) / 6.0,
        (5.0 * pts[0].1 + pts[1].1) / 6.0,
    );
    out.push(cur);
    // One cubic per interior triple, then the closing cubic — identical
    // control points to spline_segs' `bez`.
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
///
/// A subgraph (state composite) with a `direction` of its own other
/// than the one round it is laid out on its own in that direction while
/// none of its nodes links to a node outside it (mermaid's rule; edges
/// to the subgraph itself are fine), and placed in the drawing like one
/// node the size of its frame, edges to it meeting the frame. [`route`]
/// and [`route_partial`] route such a block in its own direction too.
pub fn scene(g: &Graph) -> Scene {
    let (labels, sizes) = crate::layout::node_texts(g);
    scene_labelled(g, &sizes, &labels)
}

/// Same as [`scene`] but with caller-provided node sizes — for
/// nodes whose size doesn't come from the label (ER entity tables,
/// icon nodes, ...). A fork/join bar's size is fitted to its branches
/// (laid across the flow and spanning them, see `fit_bars`); every
/// other valid size is used as given, and a negative or non-finite
/// width or height counts as 0, as in [`route_sized`].
///
/// Each node's label (`SceneNode::label`) is wrapped at the graph's
/// width with the built-in text estimate, as [`scene`] wraps it; sizes
/// measured with a real font go with [`scene_sized_with`].
pub fn scene_sized(g: &Graph, sizes: &[(f64, f64)]) -> Scene {
    scene_labelled(g, sizes, &crate::layout::node_texts(g).0)
}

/// [`scene_sized`] for an embedder that measures text with its own
/// font: each node's label is wrapped at the graph's width
/// ([`Graph::wrapping_width`]) by `measure` — a label line's width at
/// [`crate::layout::TEXT_CALIBRATION`], as
/// [`crate::layout::intrinsic_size_wrapped`] takes it — so
/// `SceneNode::label` holds exactly the lines a box sized by
/// `intrinsic_size_wrapped(&g.nodes[i], g.wrapping_width(), measure)`
/// was fitted to. [`scene_sized`] wraps with the built-in estimate,
/// whose lines a real font's boxes may not hold.
pub fn scene_sized_with(g: &Graph, sizes: &[(f64, f64)], measure: impl Fn(&str) -> f64) -> Scene {
    scene_labelled(g, sizes, &crate::layout::node_labels(g, &measure))
}

/// [`scene_sized`] with each node's drawn label given (index-parallel
/// with `g.nodes`). A subgraph laid out in its own direction is a block
/// of its own ([`crate::compound`], report B3): the graph is cut into
/// such blocks and the rest, each laid out here on its own.
pub(crate) fn scene_labelled(g: &Graph, sizes: &[(f64, f64)], labels: &[String]) -> Scene {
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );
    let sizes = sane_sizes(sizes);
    // Before the bars are fitted: each part fits its own across its flow.
    if let Some(split) = crate::compound::split(g) {
        if let Some(sc) = crate::compound::scene(g, &split, &sizes, labels) {
            return sc;
        }
    }
    let sizes = &fit_bars(g, &sizes)[..];
    let mut sc = if g.subgraphs.is_empty() {
        scene_flat(g, sizes, labels)
    } else {
        scene_clustered(g, sizes, labels)
    };
    crate::edgepass::finish(g, &mut sc, crate::edgepass::Mode::Static);
    sc
}

/// The classic single-level pipeline (no subgraphs).
fn scene_flat(g: &Graph, sizes: &[(f64, f64)], labels: &[String]) -> Scene {
    let lo = layout_sized(g, sizes);
    let lanes = static_lanes(g, &lo, None, g.edges.len());
    let mut sc = scene_from_layout(g, sizes, labels, lo, &lanes, g.edges.len());
    let moved = avoid_label_overlaps(&sc.nodes, &[], &mut sc.edges, LABEL_STATIC_WORK);
    fit_moved_labels(&mut sc, &moved);
    sc
}

/// The lane of each of `g.edges` that is a back-edge with one, in
/// `lo`'s abstract coordinates:
/// planned on the layers as the layout planned them
/// ([`crate::layout::back_lanes`]). Only the first `real` edges get one
/// (the rest are `scene_clustered`'s stand-ins).
fn static_lanes(
    g: &Graph,
    lo: &LayoutResult,
    node_cluster: Option<&[Vec<usize>]>,
    real: usize,
) -> Vec<Option<crate::lanes::Lane>> {
    // The nodes with their layer index as `l`, and each channel's dummies
    // on their layers.
    let at: Vec<Placed> = lo
        .nodes
        .iter()
        .map(|p| Placed {
            b: p.b,
            l: p.layer as f64,
            bsize: p.bsize,
            lsize: p.lsize,
            layer: p.layer,
        })
        .collect();
    let mut dummies: Vec<(f64, f64)> = Vec::new();
    let chains: Vec<Vec<usize>> = g
        .edges
        .iter()
        .zip(&lo.edge_paths)
        .map(|(e, path)| {
            let (la, lb) = (lo.nodes[e.from].layer, lo.nodes[e.to].layer);
            path.iter()
                .enumerate()
                .map(|(k, &(b, _))| {
                    let layer = if la < lb {
                        la + 1 + k
                    } else {
                        la.saturating_sub(1 + k)
                    };
                    dummies.push((b, layer as f64));
                    dummies.len() - 1
                })
                .collect()
        })
        .collect();
    let mut v = vec![None; g.edges.len()];
    for (ei, lane) in crate::layout::back_lanes(g, &at, node_cluster, &chains, |d| dummies[d]) {
        if ei < real {
            v[ei] = Some(lane);
        }
    }
    v
}

/// Turn an abstract [`LayoutResult`] into a positioned [`Scene`]:
/// edge geometry, canvas bbox, and the direction transform. Shared by
/// the flat and clustered pipelines so both route edges identically.
/// A back-edge runs in its lane ([`crate::lanes`], planned by
/// [`crate::layout::back_lanes`] exactly as the layout planned it), or
/// across the gap when nothing lies between its ends (`lanes`, from
/// [`static_lanes`]); in the clustered pipeline the lane is scoped to
/// what it actually passes: members of the innermost box
/// holding both its ends (anything outside sits beyond that box's
/// walls) — so it stays beside them instead of looping round the whole
/// diagram, unless that would take it over a neighbouring box. Only the
/// first `real` of `g.edges` are drawn as themselves (the rest are
/// `scene_clustered`'s stand-ins, see [`edge_chords`]).
fn scene_from_layout(
    g: &Graph,
    sizes: &[(f64, f64)],
    labels: &[String],
    lo: LayoutResult,
    lanes: &[Option<crate::lanes::Lane>],
    real: usize,
) -> Scene {
    // Edge geometry (abstract coordinates) with parallel-edge separation.
    let offs = parallel_offsets(g);
    let bars = bar_degrees(g);
    // Only a state note card's link needs them (see `note_elbow`).
    let chords = if g
        .nodes
        .iter()
        .any(|n| crate::style::is_note_card(&n.id, n.shape, &n.style))
    {
        edge_chords(g, &lo.nodes, real)
    } else {
        Vec::new()
    };
    // The frames' extents along the flow, for a lane's legs out of them.
    let frames = if g.subgraphs.is_empty() || lanes.iter().all(Option::is_none) {
        Vec::new()
    } else {
        flow_frame_spans(g, &lo.nodes, g.direction)
    };
    let curve = |ei: usize| {
        let e = &g.edges[ei];
        ei < real
            && lo.edge_paths[ei].is_empty()
            && lanes[ei].is_none()
            && lo.nodes[e.from].layer != lo.nodes[e.to].layer
    };
    let bows = twin_bows(g, &offs, curve);
    let sides = twin_sides(g, &offs, curve, |ei| {
        let e = &g.edges[ei];
        lo.nodes[e.from].layer > lo.nodes[e.to].layer
    });
    let mut abs_edges: Vec<AbsEdge> = Vec::with_capacity(g.edges.len());
    // How far each node's self-loop reaches past its side, and its
    // stack of loops along it.
    let loops = loop_reach(g);
    let spans = loop_spans(g, &offs);
    for (ei, (e, off)) in g.edges.iter().zip(offs).enumerate() {
        let a = &lo.nodes[e.from];
        let b = &lo.nodes[e.to];
        let lone = lone_ends(g, &bars, e);
        let card = |v: usize| {
            let n = &g.nodes[v];
            crate::style::is_note_card(&n.id, n.shape, &n.style)
        };
        let elbow = if lo.edge_paths[ei].is_empty() && (card(e.from) || card(e.to)) {
            note_elbow(
                a,
                b,
                g.nodes[e.from].shape,
                g.nodes[e.to].shape,
                (card(e.from), card(e.to)),
                lo.nodes
                    .iter()
                    .enumerate()
                    .filter(|&(v, _)| v != e.from && v != e.to)
                    .map(|(_, p)| p),
                chords.iter().copied(),
            )
        } else {
            None
        };
        // A back-edge with a lane: its corners rounded as the router
        // rounds them (a host's spline through the waypoints draws the
        // same), the curve an approximation through the lane, the label
        // riding the lane's middle.
        if let (None, Some(lane)) = (elbow, lanes[ei]) {
            let ends = ((a, g.nodes[e.from].shape), (b, g.nodes[e.to].shape));
            // Where another edge heads from node `v`: its channel's point
            // next to `v`, else its other end.
            let heads = |k: usize, v: usize| {
                let (f, path) = (&g.edges[k], &lo.edge_paths[k]);
                let near = if f.from == v {
                    path.first()
                } else {
                    path.last()
                };
                let far = if f.from == v { f.to } else { f.from };
                near.map_or(lo.nodes[far].b, |p| p.0)
            };
            // Self-loops sit on their node's high side ([`free_edge`]).
            let loop_side = |v: usize| if loops[v] > 0.0 { 1.0 } else { 0.0 };
            let side = lane_sides(g, &lo.nodes, e, lane.x, heads, loop_side);
            let lane = lane_room(g, e, lane, &lo.nodes, &frames);
            let (pts, wps, mid) = lane_geometry(ends.0, ends.1, lane, side);
            let label = e.label.as_ref().map(|l| (mid, text_width(l) + 14.0));
            abs_edges.push((pts, wps, label));
            continue;
        }
        let pts = elbow.unwrap_or_else(|| {
            let pts = edge_points(
                a,
                b,
                g.nodes[e.from].shape,
                g.nodes[e.to].shape,
                (e.from == e.to).then(|| spans[e.from]),
                lo.total_b,
                off,
                lone,
            );
            twin_curve(pts, bows[ei], sides[ei], chip_extents(e, g.direction))
        });
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
            let (c0, c1) = (chain[0], *chain.last().unwrap());
            // A bar side holding only this edge meets it at its middle.
            let c0 = if lone.0 { (a.b, c0.1) } else { c0 };
            let c1 = if lone.1 { (b.b, c1.1) } else { c1 };
            let p0 = anchor(a, g.nodes[e.from].shape, c0, off, a_bottom);
            let p3 = anchor(b, g.nodes[e.to].shape, c1, off, !a_bottom);
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

    // Canvas from the bounding box of nodes + curves + labels: each
    // curve to its exact extremes, not its control points — a back-edge
    // bow's handles reach far past its apex (report Q2). The edge stages
    // refit the canvas to everything painted at the end (strokes, end
    // glyphs, rerouted paths; `edgepass::refit`).
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
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
            let (b0, b1, l0, l1) = cubic_bounds(pts);
            bb.add(b0, l0);
            bb.add(b1, l1);
        } else {
            for &(bp, lp) in wps {
                bb.add(bp, lp);
            }
        }
        // A label's width runs along screen x: across the flow in TB /
        // BT, along it in LR / RL (it used to be laid across the flow in
        // every direction, so a wide LR label made `scene()` far taller
        // than its drawing, every node shifted down by the excess).
        if let Some((m, w)) = label {
            let (hb, hl) = if horizontal {
                (LABEL_BOX_H / 2.0, w / 2.0)
            } else {
                (w / 2.0, LABEL_BOX_H / 2.0)
            };
            bb.add(m.0 - hb, m.1 - hl);
            bb.add(m.0 + hb, m.1 + hl);
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
                label: labels[i].clone(),
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
            paint_width: g.edge_paint_width,
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
fn scene_clustered(g: &Graph, sizes: &[(f64, f64)], labels: &[String]) -> Scene {
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
    // A member's edge to its own box is drawn as its stand-in, whole:
    // an edge to the box's entry member (see `own_box_targets`).
    let own_drawn: Vec<bool> = g
        .sub_edges
        .iter()
        .zip(&stand_in)
        .map(|(e, si)| {
            si.is_some()
                && matches!((e.from, e.to), (End::Node(v), End::Sub(s))
                    if node_cluster[v].contains(&s))
        })
        .collect();
    // The clusters a stand-in's ends cut back to (its box ends).
    let cut_at = |ei: usize| -> Vec<usize> {
        let Some(i) = stand_in.iter().position(|&x| x == Some(ei)) else {
            return Vec::new();
        };
        if own_drawn[i] {
            return Vec::new();
        }
        let e = &g.sub_edges[i];
        [e.from, e.to]
            .into_iter()
            .filter_map(|end| match end {
                End::Sub(s) => Some(s),
                End::Node(_) => None,
            })
            .collect()
    };
    // Each stand-in cut back to a frame tells the layout which: its
    // label is drawn between those borders (B7 review r4 #2).
    ga.frame_cut = vec![[None, None]; ga.edges.len()];
    for (i, si) in stand_in.iter().enumerate() {
        let Some(ei) = *si else { continue };
        if own_drawn[i] {
            continue;
        }
        let cut = |end: End| match end {
            End::Sub(s) => Some(s),
            End::Node(_) => None,
        };
        ga.frame_cut[ei] = [cut(g.sub_edges[i].from), cut(g.sub_edges[i].to)];
    }
    let (lo, groups) = layout_clustered_ranked(&ga, sizes, &node_cluster, &rank_only);
    let lanes = static_lanes(&ga, &lo, Some(&node_cluster), ne);
    let mut sc = scene_from_layout(&ga, sizes, labels, lo, &lanes, ne);
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
                // A back-edge's lane kept beside the members of the
                // innermost box holding both its ends lies in that box
                // (and its ancestors), its label too: the frame grows over
                // them, so the edge never leaves its own frame (report
                // §2.2).
                let home = lanes.get(ei).copied().flatten().is_some_and(|l| l.local)
                    && node_cluster[ge.from].contains(&si)
                    && node_cluster[ge.to].contains(&si);
                if home {
                    if let Some((mb, half)) = lane_label_reach(se, horizontal) {
                        b0 = b0.min(mb - half - LANE_PAD);
                        b1 = b1.max(mb + half + LANE_PAD);
                    }
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
                    let inside =
                        home || path.contains(&si) && !cut.iter().any(|c| path.contains(c));
                    let (wb, wl) = if horizontal { (wy, wx) } else { (wx, wy) };
                    if inside && wl >= l0 && wl <= l1 {
                        b0 = b0.min(wb - LANE_PAD);
                        b1 = b1.max(wb + LANE_PAD);
                    }
                }
            }
            let (b0, b1) = stop_at_siblings(g, si, &raw, &boxes, horizontal, (b0, b1, l0, l1));
            boxes[si] = Some(if horizontal {
                (x, b0, w, b1 - b0)
            } else {
                (b0, y, b1 - b0, h)
            });
        }
        renest_boxes(g, &depth, &mut boxes);
        let out = clusters_of(g, &boxes, &depth);
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
            // A member to its own enclosing box: its stand-in to the
            // box's entry member, drawn whole (routed, labelled and
            // ranked like any edge between the two).
            if let (true, Some(ei)) = (own_drawn[i], stand_in[i]) {
                sc.edges.push(SceneEdge {
                    from: end_id(e.from),
                    to: end_id(e.to),
                    ..stand_edges[ei - ne].clone()
                });
                continue;
            }
            // A routed stand-in, cut back to the frame(s), when it has a
            // channel; else a curve to the facing frame side; else (no
            // side faces the other end) the old box-to-box curve.
            // A box drawn to its own member (or a member to its own box
            // with no other entry member to lead): a short stub between
            // the member and the nearer flow-axis side of the frame, its
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
                    paint_width: g.edge_paint_width,
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
                            free_edge(&pa, &pb, sa, sb, None, 0.0, Some(g.direction), NO_LONE)
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
                paint_width: g.edge_paint_width,
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
    // Flow-space extent of an end: (breadth centre, layer lo, layer hi,
    // breadth lo, breadth hi).
    let extent = |end: End| -> Option<(f64, f64, f64, f64, f64)> {
        let (x, y, w, h) = match end {
            End::Node(v) => {
                let n = &nodes[v];
                (n.x - n.w / 2.0, n.y - n.h / 2.0, n.w, n.h)
            }
            End::Sub(s) => frames.get(s).copied().flatten()?,
        };
        let (b0, l0) = to_flow(dir, (x, y));
        let (b1, l1) = to_flow(dir, (x + w, y + h));
        Some((
            (b0 + b1) / 2.0,
            l0.min(l1),
            l0.max(l1),
            b0.min(b1),
            b0.max(b1),
        ))
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
    // without one). Against the flow (`Rollback --> Platform` from
    // below) the proxies sit at the box's far side — its entry members
    // head it, its exit members close it — so the port faces the other
    // end instead, kept `SUB_PAD` inside the frame's corners: one short
    // run straight across the gap, not an S towards a member out of
    // sight beyond the box.
    let port = |members: &[usize], ext: (f64, f64, f64, f64, f64), toward: f64, exit: bool| {
        let b = if ahead {
            nearest(members, toward).unwrap_or(ext.0)
        } else {
            toward
                .max(ext.3 + SUB_PAD)
                .min((ext.4 - SUB_PAD).max(ext.3 + SUB_PAD))
        };
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
    let members = |s: usize, entry: bool| -> Vec<usize> {
        let flags = if entry { &has_in } else { &has_out };
        let m: Vec<usize> = (0..g.nodes.len())
            .filter(|&v| node_cluster[v].contains(&s) && !flags.contains(&(s, v)))
            .collect();
        if !m.is_empty() {
            return m;
        }
        // A cycle inside the box leaves every member an internal in-edge
        // (out-edge): `Cache --> Gate` in the showcase's Platform. Then
        // the box stood for no member, and an edge to (from) it shaped
        // no rank at all — `Reg --> Platform` set the CI box beside
        // Platform instead of above it. The members whose only internal
        // in-edges (out-edges) close a cycle stand for it instead.
        cycle_ends(g, node_cluster, grid, s, entry)
    };
    let ends = |end: End, entry: bool| match end {
        End::Node(v) => vec![v],
        End::Sub(s) => members(s, entry),
    };
    g.sub_edges
        .iter()
        .map(|e| match (e.from, e.to) {
            // A member joined to its own enclosing box (`Dev --> CI`,
            // Dev inside CI) is drawn to the box's entry member and ranks
            // before it ([`own_box_targets`]). Neither the node itself nor
            // a member it already has an edge to (one entering through a
            // cycle's closing edge) is an entry it could lead: that line
            // would lie on the edge drawn already.
            (End::Node(v), End::Sub(s)) if node_cluster[v].contains(&s) => {
                let linked = |m: usize| g.edges.iter().any(|e| e.from == v && e.to == m);
                let to = members(s, true)
                    .into_iter()
                    .filter(|&m| m != v && !linked(m));
                (vec![v], to.collect())
            }
            // A box to its own member is a stub from the frame; its exit
            // members, inside around the member, give it no rank (one
            // would only pull the member below them, through a stand-in
            // nobody sees).
            (End::Sub(s), End::Node(v)) if node_cluster[v].contains(&s) => (Vec::new(), Vec::new()),
            _ => (ends(e.from, false), ends(e.to, true)),
        })
        .collect()
}

/// Index-parallel with `g.sub_edges`: for a sub-edge from a member to
/// its own enclosing box (`Dev --> CI` with `Dev` inside `CI`), the
/// member it is drawn to — the box's first entry member other than the
/// node itself and the members it already links to
/// ([`sub_edge_proxies`]), the stand-in `scene_clustered` lays out and
/// ranks. The edge is drawn and routed as an edge between
/// the two nodes, in the static scene, in [`route`] and in every edge
/// stage (`edgepass::ctx`): no line can reach the box from inside it,
/// and a stub out to the frame's border pointed at nothing (against the
/// flow, beside the title, report P0 #3). `None` for every other
/// sub-edge, and for a member with no other entry member to lead (a
/// stub to the frame stands for it then).
pub(crate) fn own_box_targets(g: &Graph) -> Vec<Option<usize>> {
    let mut out = vec![None; g.sub_edges.len()];
    // Cheap checks first: a drag frame pays for the proxies only when
    // the graph has such an edge.
    if !g
        .sub_edges
        .iter()
        .any(|e| matches!((e.from, e.to), (End::Node(_), End::Sub(_))))
    {
        return out;
    }
    let node_cluster = node_cluster_paths(g);
    let own = |e: &crate::model::SubEdge| matches!((e.from, e.to), (End::Node(v), End::Sub(s)) if node_cluster[v].contains(&s));
    if !g.sub_edges.iter().any(own) {
        return out;
    }
    let grid = grid_pairs(g, &node_cluster);
    let proxies = sub_edge_proxies(g, &node_cluster, &grid);
    for (i, (e, (_, to))) in g.sub_edges.iter().zip(&proxies).enumerate() {
        if let (End::Node(v), End::Sub(s)) = (e.from, e.to) {
            if node_cluster[v].contains(&s) {
                out[i] = to.first().copied();
            }
        }
    }
    out
}

/// The members of box `s` that enter it (`entry`) or leave it once
/// the edges closing a cycle inside it are set aside: those a
/// depth-first walk over the box's own edges (and `grid` pairs), in
/// declaration order, finds running back to a node still on its path.
/// The members with no other internal in-edge (out-edge) — never empty
/// for a box with members: the root of the walk's last tree has none
/// (an edge into it from an earlier tree would have joined that tree),
/// and the first node it finishes has no other out-edge (all its
/// out-edges run back up its path). Members come in declaration order.
fn cycle_ends(
    g: &Graph,
    node_cluster: &[Vec<usize>],
    grid: &[RankGroup],
    s: usize,
    entry: bool,
) -> Vec<usize> {
    let inside = |v: usize| node_cluster[v].contains(&s);
    let n = g.nodes.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let pairs = g.edges.iter().map(|e| (e.from, e.to));
    for (a, b) in pairs.chain(grid.iter().map(|(a, b)| (a[0], b[0]))) {
        if a != b && inside(a) && inside(b) {
            adj[a].push(b);
        }
    }
    // 0 unvisited, 1 on the walk's path, 2 done.
    let mut state = vec![0u8; n];
    let mut keep_in = vec![false; n];
    let mut keep_out = vec![false; n];
    for root in (0..n).filter(|&v| inside(v)) {
        if state[root] != 0 {
            continue;
        }
        state[root] = 1;
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some(&mut (u, ref mut next)) = stack.last_mut() {
            let Some(&w) = adj[u].get(*next) else {
                state[u] = 2;
                stack.pop();
                continue;
            };
            *next += 1;
            if state[w] == 1 {
                continue; // closes a cycle: set aside
            }
            keep_out[u] = true;
            keep_in[w] = true;
            if state[w] == 0 {
                state[w] = 1;
                stack.push((w, 0));
            }
        }
    }
    let flags = if entry { &keep_in } else { &keep_out };
    (0..n).filter(|&v| inside(v) && !flags[v]).collect()
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
    let (labels, sizes) = crate::layout::node_texts(g);
    route_labelled(g, centers, &sizes, &labels)
}

/// Same as [`route`] but with caller-provided node sizes (a fork/join
/// bar's fitted as [`scene_sized`] fits it).
///
/// A size need not be valid: a host may not know one yet, or hold a
/// collapsed node. A negative or non-finite (NaN, infinite) width or
/// height counts as 0, and a box 0 wide and 0 high is a POINT node:
/// its edges meet at its centre (with one zero side it is a line, met
/// where it lies). Nodes may also share a centre. For finite centres
/// the returned geometry (every bezier point, waypoint and label spot)
/// is finite whatever the sizes; valid sizes are used exactly as given.
///
/// Labels are wrapped with the built-in text estimate, as in
/// [`scene_sized`]; sizes measured with a real font go with
/// [`route_sized_with`].
pub fn route_sized(g: &Graph, centers: &[(f64, f64)], sizes: &[(f64, f64)]) -> Scene {
    route_labelled(g, centers, sizes, &crate::layout::node_texts(g).0)
}

/// [`route_sized`] with each node's label wrapped by `measure`, as
/// [`scene_sized_with`] wraps it: the drag counterpart for an embedder
/// that sizes its nodes with its own font.
pub fn route_sized_with(
    g: &Graph,
    centers: &[(f64, f64)],
    sizes: &[(f64, f64)],
    measure: impl Fn(&str) -> f64,
) -> Scene {
    route_labelled(g, centers, sizes, &crate::layout::node_labels(g, &measure))
}

/// [`route_sized`] with each node's drawn label given (index-parallel
/// with `g.nodes`). A graph with blocks laid out in their own directions
/// ([`crate::compound`]) is routed block by block, as [`scene`] lays it
/// out.
pub(crate) fn route_labelled(
    g: &Graph,
    centers: &[(f64, f64)],
    sizes: &[(f64, f64)],
    labels: &[String],
) -> Scene {
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
    let sizes = sane_sizes(sizes);
    if let Some(split) = crate::compound::split(g) {
        if let Some(sc) = crate::compound::route(g, &split, centers, &sizes, labels) {
            return sc;
        }
    }
    let sizes = &fit_bars(g, &sizes)[..];
    let mut sc = route_built(g, centers, sizes, labels, true);
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
    labels: &[String],
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
            label: labels[i].clone(),
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
    let bars = bar_degrees(g);
    let mut edges = Vec::with_capacity(g.edges.len());
    // State note cards' links elbow round as the static layout draws
    // them (see `note_elbow`), judged in flow coordinates.
    let card = |v: usize| {
        let n = &g.nodes[v];
        crate::style::is_note_card(&n.id, n.shape, &n.style)
    };
    let flow: Vec<Placed> = if (0..g.nodes.len()).any(card) {
        placed.iter().map(|p| flow_placed(g.direction, p)).collect()
    } else {
        Vec::new()
    };
    let chords = if flow.is_empty() {
        Vec::new()
    } else {
        edge_chords(g, &flow, g.edges.len())
    };
    // Back-edges run in their lanes, or across the gap with nothing
    // between their ends, by the static layout's rule ([`route_backs`]).
    let flow_at: Vec<Placed> = placed.iter().map(|p| flow_placed(g.direction, p)).collect();
    let backs = route_backs(g, &flow_at);
    let frames = if g.subgraphs.is_empty() || !backs.iter().any(|b| matches!(b, Back::Lane(_))) {
        Vec::new()
    } else {
        flow_frame_spans(g, &flow_at, g.direction)
    };
    // How far each node's self-loop reaches past its side, and its
    // stack of loops along it.
    let loops_at = loop_reach(g);
    let spans = loop_spans(g, &offs);
    // A labelled 2-cycle across one gap bows apart as in the static
    // layout ([`twin_bows`]).
    let curve = |ei: usize| {
        let e = &g.edges[ei];
        match backs[ei] {
            Back::Direct => true,
            Back::No => e.from != e.to && top_of(e.from) == top_of(e.to),
            Back::Lane(_) => false,
        }
    };
    let bows = twin_bows(g, &offs, curve);
    let sides = twin_sides(g, &offs, curve, |ei| backs[ei] == Back::Direct);
    for (ei, (e, off)) in g.edges.iter().zip(offs).enumerate() {
        let a = &placed[e.from];
        let b = &placed[e.to];
        let elbow = if !flow.is_empty() && e.from != e.to && (card(e.from) || card(e.to)) {
            note_elbow(
                &flow[e.from],
                &flow[e.to],
                g.nodes[e.from].shape,
                g.nodes[e.to].shape,
                (card(e.from), card(e.to)),
                flow.iter()
                    .enumerate()
                    .filter(|&(v, _)| v != e.from && v != e.to)
                    .map(|(_, p)| p),
                chords.iter().copied(),
            )
            .map(|pts| pts.map(|p| from_flow(g.direction, p)))
        } else {
            None
        };
        let (fa, fb) = (&flow_at[e.from], &flow_at[e.to]);
        let (sa, sb) = (g.nodes[e.from].shape, g.nodes[e.to].shape);
        let back = if elbow.is_some() { Back::No } else { backs[ei] };
        if let Back::Lane(lane) = back {
            let heads = |k: usize, v: usize| {
                let f = &g.edges[k];
                flow_at[if f.from == v { f.to } else { f.from }].b
            };
            // Where `free_edge` draws each self-loop: the high side,
            // the low one when a drag crowded that ([`loop_on_low_side`]).
            let loop_side = |v: usize| match loops_at[v] > 0.0 {
                false => 0.0,
                true if loop_on_low_side(&flow_at, v, &loops_at) => -1.0,
                true => 1.0,
            };
            let side = lane_sides(g, &flow_at, e, lane.x, heads, loop_side);
            let lane = lane_room(g, e, lane, &flow_at, &frames);
            let (pts, wps, mid) = lane_geometry((fa, sa), (fb, sb), lane, side);
            let tf = |p: (f64, f64)| from_flow(g.direction, p);
            edges.push(SceneEdge {
                from: g.nodes[e.from].id.clone(),
                to: g.nodes[e.to].id.clone(),
                bezier: pts.map(tf),
                waypoints: wps.into_iter().map(tf).collect(),
                kind: e.kind,
                ends: e.ends,
                meta: e.meta.clone(),
                label: e
                    .label
                    .as_ref()
                    .map(|l| (l.clone(), tf(mid), text_width(l) + 14.0)),
                paint_width: g.edge_paint_width,
            });
            continue;
        }
        let pts = match (elbow, back) {
            (Some(pts), _) => pts,
            (None, Back::Direct) => twin_curve(
                direct_back(fa, fb, sa, sb, off, lone_ends(g, &bars, e)),
                bows[ei],
                sides[ei],
                chip_extents(e, g.direction),
            )
            .map(|p| from_flow(g.direction, p)),
            _ => {
                let pts = free_edge(
                    a,
                    b,
                    sa,
                    sb,
                    (e.from == e.to).then(|| spans[e.from]),
                    off,
                    Some(g.direction),
                    lone_ends(g, &bars, e),
                );
                if e.from == e.to && loop_on_low_side(&flow_at, e.from, &loops_at) {
                    // Its high side crowded by a drag: the loop goes on
                    // the free low side, mirrored across its node.
                    let c = flow_at[e.from].b;
                    pts.map(|p| {
                        let (b, l) = to_flow(g.direction, p);
                        from_flow(g.direction, (2.0 * c - b, l))
                    })
                } else if bows[ei] == 0.0 && sides[ei] == 0.0 {
                    pts
                } else {
                    twin_curve(
                        pts.map(|p| to_flow(g.direction, p)),
                        bows[ei],
                        sides[ei],
                        chip_extents(e, g.direction),
                    )
                    .map(|p| from_flow(g.direction, p))
                }
            }
        };
        let wps = if e.from != e.to && back == Back::No && top_of(e.from) != top_of(e.to) {
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
            paint_width: g.edge_paint_width,
        });
    }

    // A lane kept beside its box's members grows that frame over it, as
    // in the static layout.
    let home: Vec<bool> = backs
        .iter()
        .map(|b| matches!(b, Back::Lane(l) if l.local))
        .collect();
    let clusters = if home.iter().any(|&h| h) {
        route_lane_clusters(g, &nodes, &edges, &home)
    } else {
        route_clusters(g, &nodes)
    };

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
        let targets = own_box_targets(g);
        for ((e, &off), target) in g.sub_edges.iter().zip(&sub_offs).zip(targets) {
            let (Some((pa, sa)), Some((pb, sb))) = (end_placed(e.from), end_placed(e.to)) else {
                continue;
            };
            // A member to its own enclosing box: an edge to the box's
            // entry member, as the static layout draws it.
            let to_member = match (e.from, target) {
                (End::Node(v), Some(m)) => Some((v, m)),
                _ => None,
            };
            // A box drawn to its own member (or a member to its own box
            // with no other entry member): the static layout's stub, its
            // label just outside the frame (a label on the short inner
            // run would sit on the member).
            let own = match (e.from, e.to) {
                _ if to_member.is_some() => None,
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
            let (pts, own_mid) = match (to_member, own) {
                (Some((v, m)), _) => {
                    let (sv, sm) = (g.nodes[v].shape, g.nodes[m].shape);
                    let dir = Some(g.direction);
                    let pts = free_edge(&placed[v], &placed[m], sv, sm, None, off, dir, NO_LONE);
                    (pts, None)
                }
                (None, Some((pts, mid))) => (pts, Some(mid)),
                (None, None) => (
                    free_edge(&pa, &pb, sa, sb, None, off, Some(g.direction), NO_LONE),
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
                paint_width: g.edge_paint_width,
            });
        }
    }

    // Labels off what they cover before the edge stages: the router
    // prices its paths round them, and the label stage then re-places
    // only the labels of the edges it changed.
    if place_labels {
        avoid_label_overlaps(&nodes, &clusters, &mut edges, LABEL_ROUTE_WORK);
    }
    // The boxes and labels only: the edge stages' refit grows the canvas
    // over every edge as painted (`edgepass::refit`), never over a
    // curve's control points (report Q2), as in the static scene.
    let mut bb = Bbox::new();
    grow_boxes(&mut bb, &nodes, &clusters);
    for e in edges.iter().filter(|e| edge_drawn(e)) {
        grow_label(&mut bb, e);
    }
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
/// `PARTIAL_CLEAR` of a moved node's old or new box, of the strip a
/// frame grew by or of its moved title chip keep their `base` path and
/// ends but are rerouted when it now hits a node, a frame or a title
/// chip, or runs on top of another edge (never for spacing alone).
/// Every other edge keeps its
/// `base` geometry (channel waypoints, label slot; only a port shared
/// with a redrawn edge is spaced again, and its label moves when it now
/// covers something) and is an obstacle the affected edges are routed
/// round (see `edgepass::Ctx::fixed`), so a drag frame costs
/// what lies near the drag, not the whole drawing. A frame holding a
/// moved node is drawn round its members as [`route`] draws it; other
/// frames keep their `base` box. When more than
/// `PARTIAL_ROUTE_SHARE` of the edges would be drawn afresh the frame
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
    route_partial_with(g, centers, base, base_centers, &[])
}

/// [`route_partial`] (lengths checked) with the boxes of some nodes
/// given: `sized` holds (node, size, whether it differs from its `base`
/// box) for each proxy of a block laid out in its own direction
/// ([`crate::compound`]) — its box is the block's frame, not one sized
/// from a label — and a proxy whose box changed counts as moved however
/// its centre lies. A graph with such blocks is re-routed block by block.
pub(crate) fn route_partial_with(
    g: &Graph,
    centers: &[(f64, f64)],
    base: &Scene,
    base_centers: &[(f64, f64)],
    sized: &[(usize, (f64, f64), bool)],
) -> Scene {
    let n = g.nodes.len();
    let mut moved: Vec<bool> = (0..n)
        .map(|i| {
            (centers[i].0 - base_centers[i].0).abs() > 0.01
                || (centers[i].1 - base_centers[i].1).abs() > 0.01
        })
        .collect();
    for &(v, _, resized) in sized {
        moved[v] |= resized;
    }
    if !moved.iter().any(|&m| m) {
        return base.clone();
    }
    if let Some(split) = crate::compound::split(g) {
        return crate::compound::route_partial(g, &split, centers, base, base_centers)
            .unwrap_or_else(|| route(g, centers));
    }
    // Each label wrapped once for the frame (the fallbacks below reuse it).
    let (labels, mut raw_sizes) = crate::layout::node_texts(g);
    for &(v, size, _) in sized {
        raw_sizes[v] = size;
    }
    let sizes = fit_bars(g, &raw_sizes);
    // No builder label pass: a redrawn edge's label starts at its spot
    // on the new line and the label stage moves it only if it covers
    // something. route() slides every label first so the router can
    // price paths round them; that second pass costs 3-4 ms of a dense
    // drag frame and is left out here (G13).
    let mut sc = route_built(g, centers, &sizes, &labels, false);
    // A kept edge is copied from `base` whole (style, end glyphs,
    // label text), and a node's box follows its text: an edit to any of
    // them must not keep stale geometry.
    let label_text = |e: &SceneEdge| e.label.as_ref().map(|l| l.0.clone());
    let same_graph = sc.edges.len() == base.edges.len()
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
            .all(|(a, b)| a.id == b.id && a.label == b.label && a.shape == b.shape);
    // A full `route()` still keeps the note links a drag leaves (see
    // `note_link_kept`) while the graph is the base's: a static frame
    // that does not stand for `route()`'s (one the layout drew wider
    // than its members, as composite states' often are) sends every
    // drag of such a diagram here.
    let full = |mut sc: Scene| {
        if same_graph {
            for i in 0..g.edges.len().min(sc.edges.len()) {
                if let Some(kept) = note_link_kept(g, base, centers, base_centers, &moved, i) {
                    sc.edges[i] = kept;
                }
            }
        }
        sc
    };
    if !same_graph || !clusters_line_up(g, base) {
        return full(route_labelled(g, centers, &raw_sizes, &labels));
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
    // A member's edge to its own box ends on the box's entry member
    // (`own_box_targets`), which may move inside a frame that keeps its
    // box: (member id, box id) of each such edge whose entry member moved.
    let own_moved: Vec<(&str, &str)> = g
        .sub_edges
        .iter()
        .zip(own_box_targets(g))
        .filter_map(|(e, m)| match (e.from, e.to, m) {
            (End::Node(v), End::Sub(s), Some(m)) if moved[m] => {
                Some((g.nodes[v].id.as_str(), g.subgraphs[s].id.as_str()))
            }
            _ => None,
        })
        .collect();
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
    // Every edge but the redrawn ones holds its ends (`Ctx::held`): one
    // kept near the drag (not `fixed`) is rerouted only where it now
    // hits something or runs on top of another edge, its ends left
    // where the base put them.
    let mut held = vec![true; base.edges.len()];
    let mut redrawn = 0;
    for (i, e) in sc.edges.iter_mut().enumerate() {
        let b = &base.edges[i];
        let own = own_moved.contains(&(b.from.as_str(), b.to.as_str()));
        if let Some(kept) = (!own && !clash[i])
            .then(|| note_link_kept(g, base, centers, base_centers, &moved, i))
            .flatten()
        {
            // A state note card's link drawn as the drag leaves it.
            *e = kept;
        } else if moved_end(&b.from) || moved_end(&b.to) || own || clash[i] {
            fixed[i] = false;
            held[i] = false;
            redrawn += 1;
        } else {
            fixed[i] = !near(b);
            *e = b.clone();
        }
    }
    // Most of the drawing redrawn afresh (not merely near the drag): a
    // full route() costs about the same and draws it better.
    if redrawn as f64 > PARTIAL_ROUTE_SHARE * fixed.len() as f64 {
        return full(route_labelled(g, centers, &raw_sizes, &labels));
    }
    sc.clusters = clusters;
    // The base canvas already covers the kept geometry; the pipeline's
    // refit grows it for whatever the drag pushed outward.
    sc.width = base.width;
    sc.height = base.height;
    crate::edgepass::finish_held(g, &mut sc, crate::edgepass::Mode::Routed, &fixed, &held);
    sc
}

/// How far (px) a drag may move a state note card or the node it notes
/// before [`route_partial`] redraws their link instead of letting it
/// follow ([`note_link_kept`]): a node's clearance from the edges round
/// it ([`crate::clearance::CRAMP_NODE_GAP`]), so a link that follows
/// never comes nearer anything than its node already did.
const NOTE_FOLLOW: f64 = crate::clearance::CRAMP_NODE_GAP;

/// The drawing of `base`'s edge `i` that [`route_partial`] keeps when it
/// links a state note card ([`crate::style::is_note_card`]) to a node
/// and the drag moved neither end, or moved one of them (or both alike)
/// by at most [`NOTE_FOLLOW`]: the base link itself, its moved end (and
/// the run into it) carried along — unless one of the moved nodes' new
/// boxes now meets it, which redraws it as `route()` would. `None` for
/// any other edge. A note link is short and its ends are pinned, but
/// redrawn afresh it took `route()`'s course, which can differ from
/// `scene()`'s (a router path round its state's composite): a 1 px drag
/// near it, or of its card or state, threw it 20-70 px (verifier: 22 of
/// 206 composite-state diagrams).
fn note_link_kept(
    g: &Graph,
    base: &Scene,
    centers: &[(f64, f64)],
    base_centers: &[(f64, f64)],
    moved: &[bool],
    i: usize,
) -> Option<SceneEdge> {
    let ge = g.edges.get(i)?;
    let b = base.edges.get(i)?;
    let card = |v: usize| {
        base.nodes
            .get(v)
            .is_some_and(|n| crate::style::is_note_card(&n.id, n.shape, &n.style))
    };
    if ge.from == ge.to || !(card(ge.from) || card(ge.to)) {
        return None;
    }
    let delta = |v: usize| {
        (
            centers[v].0 - base_centers[v].0,
            centers[v].1 - base_centers[v].1,
        )
    };
    let small = |d: (f64, f64)| d.0.hypot(d.1) <= NOTE_FOLLOW;
    let (da, db) = (delta(ge.from), delta(ge.to));
    let mut e = b.clone();
    match (moved[ge.from], moved[ge.to]) {
        (false, false) => {}
        (true, true) if (da.0 - db.0).abs() < 1e-9 && (da.1 - db.1).abs() < 1e-9 && small(da) => {
            for p in e.bezier.iter_mut().chain(e.waypoints.iter_mut()) {
                *p = (p.0 + da.0, p.1 + da.1);
            }
        }
        (true, false) if small(da) => follow_end(&mut e, true, da),
        (false, true) if small(db) => follow_end(&mut e, false, db),
        _ => return None,
    }
    // A moved node's new box newly over the link (closer than the
    // clearance that counts as crossing it): redraw it.
    let (was, pts) = (edge_polyline(b), edge_polyline(&e));
    let c = crate::clearance::NODE_CLEARANCE;
    let meets = |pts: &[(f64, f64)], v: usize, at: (f64, f64)| {
        let n = &base.nodes[v];
        let r = (
            at.0 - n.w / 2.0 - c,
            at.1 - n.h / 2.0 - c,
            at.0 + n.w / 2.0 + c,
            at.1 + n.h / 2.0 + c,
        );
        pts.windows(2).any(|w| seg_meets_rect(w[0], w[1], r))
    };
    let hit = (0..g.nodes.len())
        .filter(|&v| moved[v] && v != ge.from && v != ge.to)
        .any(|v| meets(&pts, v, centers[v]) && !meets(&was, v, base_centers[v]));
    (!hit).then_some(e)
}

/// Carry the start (`src`) or the end of `e` along by `d`, with the run
/// into it: a curve's control at that end; a path's waypoints on its
/// first leg (those sharing the end's line across the leg) slid across
/// the leg only, so an orthogonal leg stays straight and the corner it
/// turns into keeps its other leg.
fn follow_end(e: &mut SceneEdge, src: bool, d: (f64, f64)) {
    let add = |p: &mut (f64, f64), dx: f64, dy: f64| {
        p.0 += dx;
        p.1 += dy;
    };
    let (pi, ci) = if src { (0, 1) } else { (3, 2) };
    if e.waypoints.len() < 2 {
        add(&mut e.bezier[pi], d.0, d.1);
        add(&mut e.bezier[ci], d.0, d.1);
        return;
    }
    let n = e.waypoints.len();
    let order: Vec<usize> = if src {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    let end = e.waypoints[order[0]];
    let vertical = (e.waypoints[order[1]].0 - end.0).abs() < 1e-9;
    let level = (e.waypoints[order[1]].1 - end.1).abs() < 1e-9;
    add(&mut e.waypoints[order[0]], d.0, d.1);
    for &k in &order[1..] {
        let q = e.waypoints[k];
        if vertical && (q.0 - end.0).abs() < 1e-9 {
            add(&mut e.waypoints[k], d.0, 0.0);
        } else if level && !vertical && (q.1 - end.1).abs() < 1e-9 {
            add(&mut e.waypoints[k], 0.0, d.1);
        } else {
            break;
        }
    }
    let (w0, w1) = (e.waypoints[0], e.waypoints[n - 1]);
    if e.bezier[0] == e.bezier[1] && e.bezier[2] == e.bezier[3] {
        e.bezier = [w0, w0, w1, w1];
    } else {
        add(&mut e.bezier[pi], d.0, d.1);
    }
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
    clusters_of(g, &boxes, &depth)
}

/// [`route_clusters`] with each frame grown over the back-edge lanes kept
/// beside the members of the innermost box holding both their ends
/// (`home`, index-parallel with `g.edges`; `edges` the scene's edges
/// with those lanes drawn) and their labels, as `scene_clustered` grows
/// its frames: [`route`] draws such a lane inside its own frame too,
/// so it never leaves the box its two ends sit in (report §2.2), and a
/// drag does not move the frame off it.
fn route_lane_clusters(
    g: &Graph,
    nodes: &[SceneNode],
    edges: &[SceneEdge],
    home: &[bool],
) -> Vec<SceneCluster> {
    if g.subgraphs.is_empty() {
        return Vec::new();
    }
    let (mut boxes, depth) = cluster_raw_boxes(g, nodes);
    let raw = boxes.clone();
    let paths = node_cluster_paths(g);
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    for si in 0..g.subgraphs.len() {
        let Some((x, y, w, h)) = boxes[si] else {
            continue;
        };
        let (mut b0, mut b1, l0, l1) = if horizontal {
            (y, y + h, x, x + w)
        } else {
            (x, x + w, y, y + h)
        };
        let mut grew = false;
        for (ei, (ge, se)) in g.edges.iter().zip(edges).enumerate() {
            if !home.get(ei).copied().unwrap_or(false)
                || !paths[ge.from].contains(&si)
                || !paths[ge.to].contains(&si)
            {
                continue;
            }
            grew = true;
            if let Some((mb, half)) = lane_label_reach(se, horizontal) {
                b0 = b0.min(mb - half - LANE_PAD);
                b1 = b1.max(mb + half + LANE_PAD);
            }
            for &(wx, wy) in &se.waypoints {
                let (wb, wl) = if horizontal { (wy, wx) } else { (wx, wy) };
                if wl >= l0 && wl <= l1 {
                    b0 = b0.min(wb - LANE_PAD);
                    b1 = b1.max(wb + LANE_PAD);
                }
            }
        }
        if !grew {
            continue;
        }
        let (b0, b1) = stop_at_siblings(g, si, &raw, &boxes, horizontal, (b0, b1, l0, l1));
        boxes[si] = Some(if horizontal {
            (x, b0, w, b1 - b0)
        } else {
            (b0, y, b1 - b0, h)
        });
    }
    renest_boxes(g, &depth, &mut boxes);
    clusters_of(g, &boxes, &depth)
}

/// Where a back-edge lane's label lies across the flow, `(centre, half
/// breadth)`: its box's height in LR / RL ([`crate::lanes::label_breadth`]),
/// its chip's width in TD / BT.
fn lane_label_reach(se: &SceneEdge, horizontal: bool) -> Option<(f64, f64)> {
    let (text, (mx, my), w) = se.label.as_ref()?;
    Some(if horizontal {
        (*my, crate::lanes::label_breadth(Some(text), true) / 2.0)
    } else {
        (*mx, w / 2.0)
    })
}

/// A frame `si` widened across the flow to `(b0, b1)` (its extent along
/// the flow `l0..l1`), stopped short of its siblings. A lane may not
/// widen a frame into a sibling's (G2): it stops FRAME_GAP short, and
/// the router takes over the lane waypoint left outside. The side is
/// judged on the member-only boxes (`raw`); the stop is the sibling's
/// frame as it stands (`boxes`), which is already widened when that
/// sibling came first, so two siblings widening towards each other
/// cannot meet.
fn stop_at_siblings(
    g: &Graph,
    si: usize,
    raw: &[RawBox],
    boxes: &[RawBox],
    horizontal: bool,
    (mut b0, mut b1, l0, l1): (f64, f64, f64, f64),
) -> (f64, f64) {
    let Some((x, y, w, h)) = raw[si] else {
        return (b0, b1);
    };
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
    (b0, b1)
}

/// Re-enforce nesting after frames were widened: a parent must still
/// enclose its children. Deepest-first, one level up.
fn renest_boxes(g: &Graph, depth: &[usize], boxes: &mut [RawBox]) {
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
}

/// Per-subgraph boxes as paintable [`SceneCluster`]s, outermost-first.
fn clusters_of(g: &Graph, boxes: &[RawBox], depth: &[usize]) -> Vec<SceneCluster> {
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
/// in the same coordinate space as the inputs. Sizes follow
/// [`route_sized`]: a negative or non-finite side counts as 0, a
/// zero-size box is a point the curve meets at its centre, and finite
/// centres (coincident ones too) give a finite curve.
pub fn box_edge_bezier(
    a_center: (f64, f64),
    a_size: (f64, f64),
    b_center: (f64, f64),
    b_size: (f64, f64),
    offset: f64,
    self_loop: bool,
) -> [(f64, f64); 4] {
    let sizes = sane_sizes(&[a_size, b_size]).into_owned();
    let make = |c: (f64, f64), s: (f64, f64)| Placed {
        b: c.0,
        l: c.1,
        bsize: s.0,
        lsize: s.1,
        layer: 0,
    };
    let (a_size, b_size) = (sizes[0], sizes[1]);
    free_edge(
        &make(a_center, a_size),
        &make(b_center, b_size),
        Shape::Rect,
        Shape::Rect,
        // A host's loop keeps its offset as given: it alone knows the
        // node's other loops, which `scene` squeezes onto a short side
        // together ([`loop_points`]).
        self_loop.then_some(0.0),
        offset,
        None,
        NO_LONE,
    )
}

/// The `<marker>` id of an unstyled edge end's glyph drawn in `line`,
/// the theme's edge colour. The end arrow keeps its historical `#arrow`
/// id in the default colour, so the default output is unchanged; in any
/// other colour every id carries it (`arrow_d0d0d0`), as a `linkStyle`
/// stroke's does — marker ids are page-global, and two diagrams of
/// different themes inlined on one page must not draw each other's
/// heads.
fn themed_marker_id(start: bool, m: Marker, line: &str) -> String {
    let base = marker_base_id(start, m);
    if line == EDGE_COLOR {
        return base.to_string();
    }
    let mut id = base.to_string();
    id.push('_');
    id.extend(
        line.chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }),
    );
    id
}

/// [`themed_marker_id`] in the default edge colour.
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
pub(crate) const EDGE_WIDTH: f64 = 2.0;
pub(crate) const EDGE_WIDTH_THICK: f64 = 3.5;
/// Default stroke width of a node's outline — and of the strokes drawn
/// with it: a cylinder's lid, a subroutine's bars, a final state's ring.
const NODE_STROKE_WIDTH: f64 = 1.8;
/// Dash pattern of a dotted (`-.->`) edge stroked `width` px wide:
/// square dots one stroke width long, a stroke and a half apart — `2 3`
/// at [`EDGE_WIDTH`]. Mermaid draws `2 2` at its 2 px edge; the slightly
/// wider gap keeps the line reading as dots rather than as a fainter
/// solid line, and long dashes (the old `5 4`) read as dashed, which is
/// what `linkStyle stroke-dasharray` is for. The pattern scales with the
/// stroke, so a `linkStyle stroke-width` dotted edge stays dotted (a
/// fixed `2 3` under a 6 px stroke reads as stripes). Shared with the
/// advance writer.
pub(crate) fn dotted_dash(width: f64) -> String {
    let r = |v: f64| (v * 10.0).round() / 10.0;
    format!("{} {}", r(width), r(width * 1.5))
}

/// Least run (px) of stroke a styled edge keeps in sight on either side
/// of a label lying on its line (report M3, B7 review r3 #1): three dots
/// of a dotted edge's dash at the default width, two at a 3 px
/// `linkStyle` width; a run a thick or coloured stroke reads on.
const STYLE_STUB: f64 = 15.0;
/// The most [`label_stub`] asks for, whatever a `stroke-dasharray`.
const STYLE_STUB_MAX: f64 = 3.0 * STYLE_STUB;

/// Report M3, B7 review r3 #1: how much stroke (px) an edge drawn by the
/// flowchart painter must show on either side of a label chip lying on
/// its line for its style to read — 0 for a plain solid edge, which
/// needs only its line; otherwise [`STYLE_STUB`], or two periods of its
/// dash where those are longer (capped at [`STYLE_STUB_MAX`]). Styled
/// means dotted (`-.->`), thick (`==>`), animated, or painted by
/// `linkStyle` with a colour, a width other than the plain
/// [`EDGE_WIDTH`] or a dash (`stroke-dasharray:0` draws it solid). A label centred in a gap sized only for itself and
/// its head left about 6 px either side: a single dash, and a `==>` /
/// `linkStyle` edge that read as plain. The layout sizes an LR / RL
/// labelled gap for this much (`layout`'s head room) and the label
/// stage keeps a chip off it (or beside its line where the gap is
/// shorter, as a drag may make it).
pub(crate) fn label_stub(kind: EdgeKind, meta: &crate::model::EdgeMeta) -> f64 {
    let st = &meta.style;
    let width = st.stroke_width.unwrap_or(match kind {
        EdgeKind::Thick | EdgeKind::ThickOpen => EDGE_WIDTH_THICK,
        _ => EDGE_WIDTH,
    });
    // One period of the dash the painter draws (0 = solid); a list of
    // odd length repeats twice per period, as SVG has it.
    let period = match (&st.stroke_dasharray, meta.animation) {
        (_, Some(_)) => dash_period(EDGE_ANIMATION_DASH),
        (Some(d), None) => dash_period(d),
        (None, None) if matches!(kind, EdgeKind::Dotted | EdgeKind::DottedOpen) => 2.5 * width,
        (None, None) => 0.0,
    };
    // A width is a style where it draws other than the plain one: a
    // `==>`'s, or a `linkStyle` width that is not the default (B7 review
    // r4 #7: `linkStyle default stroke-width:2px` grew every labelled LR
    // gap by about 30 px with nothing to show for it). Any colour counts:
    // the layout cannot know which theme paints the drawing, and one
    // that repeats a theme's own line reads as plain under that theme
    // alone.
    let styled = period > 0.0 || (width - EDGE_WIDTH).abs() > 1e-9 || st.stroke.is_some();
    if styled {
        (2.0 * period).clamp(STYLE_STUB, STYLE_STUB_MAX)
    } else {
        0.0
    }
}

/// One period (px) of an SVG `stroke-dasharray` value (`5 5`, `5,5`,
/// `5px 3px`, `4 2 1`): its lengths summed, twice for an odd count. 0
/// for a solid line (`0`, `none`); a value that does not parse counts as
/// a short dash.
fn dash_period(d: &str) -> f64 {
    let mut sum = 0.0;
    let mut count = 0usize;
    for part in d.split(|c: char| c == ',' || c.is_whitespace()) {
        let part = part.trim().trim_end_matches("px");
        if part.is_empty() {
            continue;
        }
        match part.parse::<f64>() {
            Ok(v) if v.is_finite() && v >= 0.0 => {
                sum += v;
                count += 1;
            }
            _ if part.eq_ignore_ascii_case("none") => return 0.0,
            _ => return STYLE_STUB / 2.0,
        }
    }
    if count % 2 == 1 {
        sum *= 2.0;
    }
    sum
}
/// Size of an end glyph in default edge strokes.
const ARROW_MARKER_UNITS: f64 = 5.5;
/// Length and width (px, user space) of every edge end glyph — the
/// arrowhead at either end, the cross, the circle — whatever the edge's
/// stroke width: a thick `==>` or a `linkStyle stroke-width` edge keeps
/// the default edge's head (mermaid's markers don't scale either). The
/// room the label placer, the crossing casing, the router and the layout
/// keep for a head is this one size, so it is right on every edge.
pub(crate) const ARROW_MARKER_PX: f64 = ARROW_MARKER_UNITS * EDGE_WIDTH;
/// How far (px) the painter stops an edge's stroke short of an arrow
/// tip, the marker's `refX` putting the tip back on the end point: half
/// a head, where the head is [`ARROW_TRIM`] / 2 either side of its axis
/// — wider than half a thick stroke, so the stroke's butt end never
/// shows beside the tip. Painter-only: the scene geometry still ends on
/// the node, where a host draws its own head.
pub(crate) const ARROW_TRIM: f64 = ARROW_MARKER_PX / 2.0;
/// The arrowhead glyph in its `0 0 10 10` view box, tip at x = 10, and
/// its mirror for the source end (tip at x = 0).
const ARROW_GLYPH_END: &str = "M 0 0 L 10 5 L 0 10 z";
const ARROW_GLYPH_START: &str = "M 10 0 L 0 5 L 10 10 z";
/// Straight run (px) the flowchart painter wants a stroke to end in at
/// an end glyph, along the curve's end tangent (the side's normal): a
/// whole head, so the head and the stroke entering its base point the
/// same way, square into the side — never along a curve still turning
/// under the head (report §2.3). It gets that run where bending into it
/// keeps close to the scene's curve ([`square_end`]).
pub(crate) const GLYPH_STUB: f64 = ARROW_MARKER_PX;

/// Corner radius (px) `to_svg` paints a plain box with: `Rect`,
/// `Subroutine` and a fork/join bar.
const BOX_RX: f64 = 3.0;
/// Corner radius (px) of a `Rounded` node, as every painter draws it
/// (an architecture service card too) and [`Scene::hit_test`] picks it.
pub(crate) const ROUNDED_RX: f64 = 9.0;
/// Most inset (px) of a hexagon's side points and of a parallelogram's
/// slant; a quarter of the width on a narrower node.
pub(crate) const SLANT_MAX: f64 = 14.0;
/// Half height (px) of a cylinder's cap ellipse as drawn (and picked):
/// [`crate::layout::cylinder_cap_ry`] for its width, which the layout
/// sized it for, and at most a quarter of its height (a host-sized node).
pub(crate) fn cylinder_ry(w: f64, h: f64) -> f64 {
    crate::layout::cylinder_cap_ry(w).min(h / 4.0)
}
/// The point of cylinder `n`'s shoulder nearest `p`, for an end on its
/// flank past the straight run (within a cap's height of its top or
/// bottom): on the cap ellipse's quarter there, which curves in so flatly
/// that a run along the flank's normal meets it side-on, far in (a hub's
/// crowded flank: 19 px along a horizontal run, landing among the lid's
/// own ends). `None` for an end on the straight run or off the flank.
/// Golden-section search on the quarter's angle, exact to far below a
/// pixel (the distance to an outside point is unimodal along it).
pub(crate) fn cylinder_shoulder_point(n: &SceneNode, p: (f64, f64)) -> Option<(f64, f64)> {
    let (rx, ry) = (n.w / 2.0, cylinder_ry(n.w, n.h));
    let (dx, dy) = (p.0 - n.x, p.1 - n.y);
    let run = n.h / 2.0 - ry;
    if !(rx > 0.0 && ry > 0.0) || dy.abs() <= run || dx.abs() < rx - 0.5 {
        return None;
    }
    // The quarter facing `p`: centre (x, cap centre), angle 0 at the
    // flank, PI/2 at the lid's (or bottom's) middle.
    let (sx, sy) = (dx.signum(), dy.signum());
    let (ox, oy) = (n.x, n.y + sy * run);
    let at = |a: f64| (ox + sx * rx * a.cos(), oy + sy * ry * a.sin());
    let d2 = |a: f64| {
        let q = at(a);
        (q.0 - p.0).powi(2) + (q.1 - p.1).powi(2)
    };
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (0.0, std::f64::consts::FRAC_PI_2);
    while b - a > 1e-12 {
        let (c, d) = (b - g * (b - a), a + g * (b - a));
        if d2(c) <= d2(d) {
            b = d;
        } else {
            a = c;
        }
    }
    Some(at((a + b) / 2.0))
}

/// How far (in cap radii) below the box centre a cylinder's label sits:
/// in the middle of the body under the lid, where mermaid sets it, not
/// up against the lid's front rim (the box centre is a cap radius above
/// the body's).
pub(crate) const CYLINDER_LABEL_DROP: f64 = 0.75;
/// How far (px) a flattened arc of [`node_outline`] may bow off its
/// chords: well under a pixel, so a distance to the polygon is a
/// distance to the painted curve.
const OUTLINE_FLATNESS: f64 = 0.05;
/// Most chords per arc of [`node_outline`]: a node thousands of px
/// across is flattened a little coarser rather than into a huge polygon.
const OUTLINE_MAX_STEPS: f64 = 256.0;

/// The outline `to_svg` paints node `n` with — its outer one for the
/// double ring and the final state — as a closed polygon (the closing
/// segment from the last point back to the first is implied), its
/// rounded corners, caps and circles flattened to within
/// [`OUTLINE_FLATNESS`]. The yardstick for where an edge end should sit
/// ([`crate::quality`]): the box `w` × `h` is only the outline of the
/// rectangular shapes.
pub(crate) fn node_outline(n: &SceneNode) -> Vec<(f64, f64)> {
    let (cx, cy, w, h) = (n.x, n.y, n.w.max(0.0), n.h.max(0.0));
    let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
    // Elliptic arc round (ox, oy) from angle a0 to a1 (radians, y down).
    let arc = |out: &mut Vec<(f64, f64)>, o: (f64, f64), rx: f64, ry: f64, a0: f64, a1: f64| {
        let rmax = rx.max(ry);
        let step = if rmax > OUTLINE_FLATNESS {
            2.0 * (1.0 - OUTLINE_FLATNESS / rmax).acos()
        } else {
            a1 - a0
        };
        let k = ((a1 - a0) / step).ceil().clamp(1.0, OUTLINE_MAX_STEPS) as usize;
        out.extend((0..=k).map(|i| {
            let a = a0 + (a1 - a0) * i as f64 / k as f64;
            (o.0 + rx * a.cos(), o.1 + ry * a.sin())
        }));
    };
    use std::f64::consts::{FRAC_PI_2, PI};
    // An SVG `<rect rx>`: ry = rx, each clamped to its half extent.
    let rounded = |rx: f64| {
        let (rx, ry) = (rx.min(w / 2.0), rx.min(h / 2.0));
        let mut out = Vec::new();
        arc(&mut out, (l + rx, t + ry), rx, ry, PI, PI + FRAC_PI_2);
        arc(&mut out, (r - rx, t + ry), rx, ry, PI + FRAC_PI_2, 2.0 * PI);
        arc(&mut out, (r - rx, b - ry), rx, ry, 0.0, FRAC_PI_2);
        arc(&mut out, (l + rx, b - ry), rx, ry, FRAC_PI_2, PI);
        out
    };
    let k = SLANT_MAX.min(w / 4.0);
    match n.shape {
        Shape::Rect | Shape::Subroutine | Shape::ForkBar => rounded(BOX_RX),
        Shape::Rounded => rounded(ROUNDED_RX),
        Shape::Stadium => rounded(h / 2.0),
        Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd => {
            let mut out = Vec::new();
            arc(&mut out, (cx, cy), w / 2.0, w / 2.0, 0.0, 2.0 * PI);
            out.pop(); // the closing point repeats the first
            out
        }
        Shape::Diamond => vec![(cx, t), (r, cy), (cx, b), (l, cy)],
        Shape::Hexagon => vec![
            (l, cy),
            (l + k, t),
            (r - k, t),
            (r, cy),
            (r - k, b),
            (l + k, b),
        ],
        Shape::Parallelogram => vec![(l + k, t), (r, t), (r - k, b), (l, b)],
        Shape::ParallelogramAlt => vec![(l, t), (r - k, t), (r, b), (l + k, b)],
        Shape::Cylinder => {
            // The top cap's upper half, the right side, the bottom
            // cap's lower half; the left side closes the polygon.
            let ry = cylinder_ry(w, h);
            let mut out = Vec::new();
            arc(&mut out, (cx, t + ry), w / 2.0, ry, PI, 2.0 * PI);
            arc(&mut out, (cx, b - ry), w / 2.0, ry, 0.0, PI);
            out
        }
    }
}

/// Step (px) [`outline_along`] marches along its line to find a point
/// on the other side of the outline: finer than any sliver of a shape a
/// line could cross between two steps unseen.
const OUTLINE_MARCH: f64 = 0.5;
/// How close (px) [`outline_along`] brackets the outline: far below what
/// any writer prints, so the end it returns is the outline's point.
const OUTLINE_EXACT: f64 = 1e-9;

/// Where the line through `p` along `out` (a unit vector pointing out of
/// node `n`, back along the stroke from its end at `p`) crosses the
/// outline `to_svg` paints, on the side the stroke comes in from: the
/// end point that keeps the stroke's final run on its line while it
/// touches the outline (report §2.5: the stages meet a slanted or round
/// shape as its box, so an end floated beside a parallelogram's flank,
/// a hexagon's point or a cylinder's lid). The outline is
/// [`node_contains`]'s, so the end is on the border picking sees.
/// From outside the line is followed in to where it meets the shape;
/// from inside (an end sunk into it) out by at most [`OUTLINE_SINK`],
/// so a run that heads back across the shape — a spilled end's leg
/// leaving a lid over the drum's shoulder — never carries its end
/// through the node to the far side. `None` when `p` already lies on
/// the outline, for a plain box (its outline is its box, where the
/// stages put the ends), or when the line misses the shape. Exact to
/// [`OUTLINE_EXACT`].
pub(crate) fn outline_along(n: &SceneNode, p: (f64, f64), out: (f64, f64)) -> Option<(f64, f64)> {
    if matches!(n.shape, Shape::Rect | Shape::Subroutine | Shape::ForkBar) {
        return None;
    }
    let contains = |q: (f64, f64)| node_contains(n, q.0, q.1);
    // `s` px into the node from `p` along the line.
    let at = |s: f64| (p.0 - s * out.0, p.1 - s * out.1);
    let inside = |s: f64| contains(at(s));
    // A bracket (outside, inside) of the crossing, marching from `p`:
    // outwards from a point inside, inwards from one outside.
    let (mut lo, mut hi) = if inside(0.0) {
        let e = OUTLINE_EXACT;
        let near = [(e, 0.0), (-e, 0.0), (0.0, e), (0.0, -e)];
        if near.iter().any(|d| !contains((p.0 + d.0, p.1 + d.1))) {
            return None;
        }
        let mut s = 0.0;
        loop {
            let next = (s - OUTLINE_MARCH).max(-OUTLINE_SINK);
            if !inside(next) {
                break (next, s);
            }
            if next <= -OUTLINE_SINK {
                return None;
            }
            s = next;
        }
    } else {
        let reach = n.w.max(0.0) + n.h.max(0.0) + OUTLINE_MARCH;
        let mut s = 0.0;
        loop {
            let next = s + OUTLINE_MARCH;
            if inside(next) {
                break (s, next);
            }
            if next > reach {
                return None;
            }
            s = next;
        }
    };
    while hi - lo > OUTLINE_EXACT {
        let mid = (lo + hi) / 2.0;
        if inside(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let q = at(hi);
    ((q.0 - p.0).hypot(q.1 - p.1) > OUTLINE_SNAP_EPS).then_some(q)
}

/// Most (px) [`outline_along`] carries an end sunk into its node back out
/// to the outline: the stages leave an end on its box, so one inside the
/// shape by more is not a sunk end but a run heading back in.
const OUTLINE_SINK: f64 = 1.0;

/// An end [`outline_along`] would move less than this (px) is on the
/// outline already and keeps its exact bytes.
const OUTLINE_SNAP_EPS: f64 = 1e-6;

/// A marker variant a `linkStyle`d edge end needs: the glyph in the
/// edge's stroke colour, and an arrowhead set further ahead of a stroke
/// wider than [`ARROW_TRIM`] (cut back further, [`arrow_trim`]) and, over
/// a very wide one, longer ([`arrow_head_len`]). Every glyph is sized in
/// user space, [`ARROW_MARKER_PX`] unless [`head_sizes`] says otherwise.
#[derive(Clone, PartialEq)]
struct MarkerVariant {
    marker: Marker,
    start: bool,
    color: Option<String>,
    /// How far (px) the stroke stops short of an arrow tip.
    trim: f64,
    /// An arrowhead's width (px) across its base: its length, or less
    /// where heads crowd one side ([`head_sizes`]).
    width: f64,
    /// An arrowhead's length (px): [`ARROW_MARKER_PX`], longer over a very
    /// wide stroke, shorter on a curve too short for its cut.
    len: f64,
}

/// The marker ids every drawn edge end uses, plus the defs of the
/// styled variants among them. An unstyled end keeps its base id, so a
/// diagram without `linkStyle` colours or wide strokes gets just the
/// base defs. A styled id is the base id, then `-t` and the trim (`.`
/// as `_`) for a wide stroke's arrowhead, `-s` and the length for a head
/// not [`ARROW_MARKER_PX`] long, `-w` and the width for one narrower than
/// it is long, then `_` and the colour with every non-alphanumeric
/// character as `_` (mermaid's marker id rule); two colours that sanitise
/// alike get a counter. A head of length 0 ([`HEAD_MIN_LEN`]) gets no
/// marker.
struct EdgeMarkers {
    /// Per scene edge: (start id, end id).
    ids: Vec<(Option<String>, Option<String>)>,
    defs: String,
}

fn edge_markers(edges: &[SceneEdge], heads: &[[Head; 2]], line: &str) -> EdgeMarkers {
    let mut made: Vec<(MarkerVariant, String)> = Vec::new();
    let mut defs = String::new();
    let mut id_for = |i: usize, e: &SceneEdge, start: bool| -> Option<String> {
        if !edge_drawn(e) {
            return None;
        }
        let m = if start {
            e.ends.start_marker(e.kind)
        } else {
            e.ends.end_marker(e.kind)
        }?;
        let head = if m == Marker::Arrow {
            heads.get(i).map_or(Head::PLAIN, |h| h[usize::from(!start)])
        } else {
            Head::PLAIN
        };
        if head.len <= 0.0 {
            return None;
        }
        let v = MarkerVariant {
            marker: m,
            start,
            color: e.meta.style.stroke.clone(),
            trim: head.trim,
            width: head.width,
            len: head.len,
        };
        if v.color.is_none() && Head::PLAIN == head {
            return Some(themed_marker_id(start, m, line));
        }
        if let Some((_, id)) = made.iter().find(|(w, _)| *w == v) {
            return Some(id.clone());
        }
        // A styled stroke's colour names its own ids; an unstyled
        // variant's carries the theme's.
        let mut id = if v.color.is_some() {
            marker_base_id(start, m).to_string()
        } else {
            themed_marker_id(start, m, line)
        };
        if v.trim != ARROW_TRIM {
            id.push_str("-t");
            id.push_str(&format!("{}", v.trim).replace('.', "_"));
        }
        if v.len != ARROW_MARKER_PX {
            id.push_str("-s");
            id.push_str(&format!("{}", v.len).replace('.', "_"));
        }
        if v.width != v.len {
            id.push_str("-w");
            id.push_str(&format!("{}", v.width).replace('.', "_"));
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
        let color = style_attr(v.color.as_deref(), line);
        defs.push_str(&marker_def_sized(
            &id, m, start, &color, v.trim, v.width, v.len,
        ));
        made.push((v, id.clone()));
        Some(id)
    };
    let ids = edges
        .iter()
        .enumerate()
        .map(|(i, e)| (id_for(i, e, true), id_for(i, e, false)))
        .collect();
    EdgeMarkers { ids, defs }
}

/// The `<marker>` def of one end glyph in `color`: every glyph sized
/// [`ARROW_MARKER_PX`] in user space and oriented along the drawn end.
/// An arrow's `refX` puts its tip on the end point although the painter
/// stops the stroke `trim` px short of it ([`arrow_trims`],
/// [`painted_path_d`]); a cross or circle sits just outside the
/// node, the stroke still running to the border beneath it.
fn marker_def(id: &str, m: Marker, start: bool, color: &str, trim: f64) -> String {
    marker_def_sized(id, m, start, color, trim, ARROW_MARKER_PX, ARROW_MARKER_PX)
}

/// [`marker_def`] of an arrowhead `len` px long and `width` px across its
/// base ([`head_sizes`]): narrower where heads crowd one side, longer
/// over a very wide stroke, shorter on a curve too short for its cut. A
/// cross or circle keeps [`ARROW_MARKER_PX`].
fn marker_def_sized(
    id: &str,
    m: Marker,
    start: bool,
    color: &str,
    trim: f64,
    width: f64,
    len: f64,
) -> String {
    let px = if m == Marker::Arrow {
        len
    } else {
        ARROW_MARKER_PX
    };
    match m {
        Marker::Arrow => {
            // View-box units per px, and the tip's own x in the box.
            let units = 10.0 / px;
            // A narrower head's base corners, inset from the box's edges.
            let inset = format!("{:.3}", 5.0 - 5.0 * width / px)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            let far = format!("{:.3}", 5.0 + 5.0 * width / px)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            let narrow = width < px;
            let (refx, d) = if start {
                let d = if narrow {
                    format!("M 10 {inset} L 0 5 L 10 {far} z")
                } else {
                    ARROW_GLYPH_START.to_string()
                };
                (trim * units, d)
            } else {
                let d = if narrow {
                    format!("M 0 {inset} L 10 5 L 0 {far} z")
                } else {
                    ARROW_GLYPH_END.to_string()
                };
                (10.0 - trim * units, d)
            };
            // To a thousandth of a unit, no trailing zeros (`5`, `2.727`).
            let refx = format!("{refx:.3}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            format!(
                "<marker id=\"{id}\" viewBox=\"0 0 10 10\" refX=\"{refx}\" refY=\"5\" \
                 markerUnits=\"userSpaceOnUse\" markerWidth=\"{px}\" markerHeight=\"{px}\" \
                 orient=\"auto\"><path d=\"{d}\" fill=\"{color}\"/></marker>"
            )
        }
        Marker::Cross => format!(
            "<marker id=\"{id}\" viewBox=\"0 0 11 11\" refX=\"{}\" refY=\"5.5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"{px}\" markerHeight=\"{px}\" \
             orient=\"auto\"><path d=\"M 1 1 L 10 10 M 10 1 L 1 10\" fill=\"none\" \
             stroke=\"{color}\" stroke-width=\"2\"/></marker>",
            if start { 0 } else { 11 }
        ),
        Marker::Circle => format!(
            "<marker id=\"{id}\" viewBox=\"0 0 10 10\" refX=\"{}\" refY=\"5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"{px}\" markerHeight=\"{px}\" \
             orient=\"auto\"><circle cx=\"5\" cy=\"5\" r=\"4\" fill=\"{color}\" \
             stroke=\"{color}\" stroke-width=\"1\"/></marker>",
            if start { 0 } else { 10 }
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

/// Stroke width edge `e` is painted with: the kind's width, unless a
/// `linkStyle` sets one — or, in an ER or class scene, the width its own
/// painter strokes ([`SceneEdge::paint_width`]). One rule for the
/// painters, the crossing casing, [`Scene::edge_at`] and
/// [`Scene::hits_in_rect`], so a pick covers exactly the pixels the
/// stroke does.
pub(crate) fn edge_stroke_width(e: &SceneEdge) -> f64 {
    if let Some(w) = e.paint_width {
        return w;
    }
    let base = match e.kind {
        EdgeKind::Thick | EdgeKind::ThickOpen => EDGE_WIDTH_THICK,
        _ => EDGE_WIDTH,
    };
    e.meta.style.stroke_width.unwrap_or(base)
}

/// `<marker>` defs beyond the classic end arrow, emitted only for the
/// glyphs some drawn edge actually uses — a diagram without `<-->`,
/// `--x` or `--o` gets just `#arrow`. Same geometry as the styled
/// variants ([`marker_def`]), in the theme's edge colour `line`.
fn extra_marker_defs(ids: &[(Option<String>, Option<String>)], line: &str) -> String {
    let mut out = String::new();
    for (start, m) in [
        (true, Marker::Arrow),
        (false, Marker::Cross),
        (true, Marker::Cross),
        (false, Marker::Circle),
        (true, Marker::Circle),
    ] {
        let base = themed_marker_id(start, m, line);
        let used = ids.iter().any(|(s, e)| {
            let got = if start { s } else { e };
            got.as_deref() == Some(base.as_str())
        });
        if used {
            out.push_str(&marker_def(&base, m, start, line, ARROW_TRIM));
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
/// translated to start inside the SVG's 16 px padding, so negative
/// coordinates are safe.
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
    let pal = opts.palette();
    let mut bb = Bbox::new();
    grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    let (minx, maxx, miny, maxy) = bb.finish();
    let tx = MARGIN - minx;
    let ty = MARGIN - miny;
    let width = (maxx - minx) + 2.0 * MARGIN;
    let height = (maxy - miny) + 2.0 * MARGIN;
    let t = |p: (f64, f64)| (p.0 + tx, p.1 + ty);

    // Endpoint display name: pseudostates announce "start"/"end", and a
    // state note card "note", not their parser-synthesized `__start_*` /
    // `__end_*` / `__note_*` ids (#16). Keyed on Shape (and a card's
    // colours, not the id prefix alone) so a user node literally named
    // "__start_x" is unaffected. BTreeMap keeps it zero-dep + ordered.
    let node_of: std::collections::BTreeMap<&str, &SceneNode> =
        sc.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let display = |id: &str| -> String {
        match node_of.get(id) {
            Some(n) if n.shape == Shape::StateStart => "start".to_string(),
            Some(n) if n.shape == Shape::StateEnd => "end".to_string(),
            Some(n) if is_note_card(n) => "note".to_string(),
            _ => id.to_string(),
        }
    };

    let dom_id = diagram_dom_id(sc, title, desc);
    let dom_id = dom_id.as_deref();
    let mut s = String::new();
    svg_open_root(&mut s, width, height, 14, title, desc, dom_id, opts);
    // Each drawn edge's stroke, worked out once for the casing, the
    // heads and the paths.
    let painted: Vec<Option<PaintedEdge>> = sc
        .edges
        .iter()
        .map(|e| edge_drawn(e).then(|| painted_edge(e)))
        .collect();
    // The path each one is stroked with, and its length where it is too
    // short to lose its cuts (its heads shrink onto it).
    let paths: Vec<Option<(String, Option<f64>)>> = sc
        .edges
        .iter()
        .zip(&painted)
        .map(|(e, p)| p.as_ref().map(|p| painted_path(e, p, t)))
        .collect();
    let whole: Vec<Option<f64>> = paths.iter().map(|p| p.as_ref().and_then(|p| p.1)).collect();
    let markers = edge_markers(&sc.edges, &head_sizes(sc, &painted, &whole), &pal.line);
    let line_of = |i: usize| match &painted[i] {
        Some(p) => painted_polyline(&sc.edges[i], p),
        None => edge_polyline(&sc.edges[i]),
    };
    let casing = crate::casing::masks(sc, &line_of, &t, dom_id);
    s.push_str(&format!(
        "<defs>{}{}{}{}{}</defs>\n",
        marker_def(
            &themed_marker_id(false, Marker::Arrow, &pal.line),
            Marker::Arrow,
            false,
            &pal.line,
            ARROW_TRIM
        ),
        extra_marker_defs(&markers.ids, &pal.line),
        markers.defs,
        edge_animation_css(&sc.edges, dom_id),
        casing.defs
    ));

    // Cluster boxes first (outermost-first), behind everything. The
    // TITLES are deferred to the very end (`cluster_titles`) so edges
    // passing the header strip never strike through the text — each
    // gets a translucent chip, mermaid-style.
    let mut cluster_titles = String::new();
    // Each nesting depth a step deeper (Palette::cluster_style), its
    // border readable on the canvas and on the frame round it (Q6).
    for (ci, c) in sc.clusters.iter().enumerate() {
        let (x, y) = t((c.x, c.y));
        let (cs_fill, cs_stroke) = pal.cluster_style(c.depth);
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.4\"/>\n",
            x, y, c.w, c.h, cs_fill, cs_stroke
        ));
        let title = fit_cluster_title(&sc.clusters, ci, maxx + MARGIN);
        let tw = text_width(&title) * TITLE_SCALE;
        cluster_titles.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{}\" rx=\"4\" \
             fill=\"{}\" fill-opacity=\"0.9\"/>\n",
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
            },
            cs_fill
        ));
        for (k, line) in title.split('\n').enumerate() {
            cluster_titles.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{}\" font-weight=\"bold\" \
                 fill=\"{}\">{}</text>\n",
                x + 10.0,
                y + 17.0 + k as f64 * TITLE_LINE_H,
                crate::layout::TITLE_FONT,
                pal.cluster_title,
                rich(line)
            ));
        }
    }

    let mut edge_labels = String::new();
    for (i, (e, (m_start, m_end))) in sc.edges.iter().zip(&markers.ids).enumerate() {
        if !edge_drawn(e) {
            continue; // layout-only link — never drawn
        }
        // `linkStyle` paint overrides the kind's look per property.
        let st = &e.meta.style;
        let sw = edge_stroke_width(e);
        let dotted =
            matches!(e.kind, EdgeKind::Dotted | EdgeKind::DottedOpen).then(|| dotted_dash(sw));
        let dash = st
            .stroke_dasharray
            .as_deref()
            .or(dotted.as_deref())
            .map_or(String::new(), |d| {
                format!(" stroke-dasharray=\"{}\"", escape(d))
            });
        let marker = format!(
            "{}{}",
            marker_attr("start", m_start.as_deref()),
            marker_attr("end", m_end.as_deref())
        );
        // The arrow marker orients to the end tangent of the stroke,
        // which stops short under an arrowhead.
        let path_d = match &paths[i] {
            Some((d, _)) => d,
            None => continue,
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
            "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}>\
             <title>{}</title></path>\n",
            path_d,
            style_attr(st.fill.as_deref(), "none"),
            style_attr(st.stroke.as_deref(), &pal.line),
            sw,
            dash,
            marker,
            edge_meta_attrs(&e.meta, dom_id),
            casing.attr.get(i).map_or("", String::as_str),
            escape(&edge_title)
        ));
        if let Some((text, m, w)) = &e.label {
            let fill = style_attr(st.color.as_deref(), &pal.text);
            svg_label_box_with(&mut edge_labels, text, t(*m), *w, &fill, &pal);
        }
    }

    for n in &sc.nodes {
        let (cx, cy) = t((n.x, n.y));
        let (w, h) = (n.w, n.h);
        // Group per node so the <title> (a11y + hover tooltip) covers
        // shape AND label together (issue #16). An unlabelled start/end
        // pseudostate announces its role (its id is synthesized); an
        // unlabelled choice or fork/join bar its own id (`decide`,
        // `forked`), the name the edge titles use for it too. Never an
        // empty `<title>` (a11y anti-pattern).
        let node_title = {
            let lbl = plain_text(&n.label);
            if !lbl.is_empty() {
                lbl
            } else {
                match n.shape {
                    Shape::StateStart => "start".to_string(),
                    Shape::StateEnd => "end".to_string(),
                    Shape::Diamond | Shape::ForkBar => n.id.clone(),
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
        // The width applies to the strokes drawn with the outline too:
        // a cylinder's lid, a subroutine's bars, a final state's ring.
        // A state note card's colours are the parser's mark of a card,
        // not the author's: it takes the theme's note paint.
        let note = is_note_card(n);
        let (fill, stroke, ink) = if note {
            (&*pal.note_fill, &*pal.note_stroke, &*pal.note_text)
        } else {
            let (f, s) = pal.node_style(n.shape);
            (f, s, &*pal.node_text)
        };
        let (own_fill, own_stroke) = if note {
            (None, None)
        } else {
            (n.style.fill.as_deref(), n.style.stroke.as_deref())
        };
        let ink = pal.ink_on_fill(own_fill, ink);
        let sw = n.style.stroke_width.unwrap_or(NODE_STROKE_WIDTH);
        let stroke = style_attr(own_stroke, stroke);
        let style = format!(
            "fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"",
            style_attr(own_fill, fill),
            stroke,
            sw
        );
        match n.shape {
            Shape::Rect | Shape::Rounded | Shape::Stadium => {
                let rx = match n.shape {
                    Shape::Rounded => ROUNDED_RX,
                    Shape::Stadium => h / 2.0,
                    _ => BOX_RX,
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
                let ry = cylinder_ry(w, h); // cap ellipse radius
                                            // The whole drum, over the lid's back arc, filled and
                                            // stroked, so the lid is filled like the body (the old
                                            // body ran under the front arc and left the lid hollow);
                                            // then the lid's front arc on top, in the node's own
                                            // stroke and width.
                s.push_str(&format!(
                    "<path d=\"M {l:.1} {ty:.1} A {rx:.1} {ry:.1} 0 0 1 {r:.1} {ty:.1} \
                     L {r:.1} {by:.1} A {rx:.1} {ry:.1} 0 0 1 {l:.1} {by:.1} Z\" {style}/>\n\
                     <path d=\"M {l:.1} {ty:.1} A {rx:.1} {ry:.1} 0 0 0 {r:.1} {ty:.1}\" \
                     fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                    l = l,
                    r = r,
                    ty = t + ry,
                    by = b - ry,
                    rx = w / 2.0,
                    ry = ry,
                    stroke = stroke,
                    style = style,
                ));
            }
            Shape::Subroutine => {
                let (l, t) = (cx - w / 2.0, cy - h / 2.0);
                s.push_str(&format!(
                    "<rect x=\"{l:.1}\" y=\"{t:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"3\" {style}/>\n\
                     <line x1=\"{l1:.1}\" y1=\"{t:.1}\" x2=\"{l1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n\
                     <line x1=\"{r1:.1}\" y1=\"{t:.1}\" x2=\"{r1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                    l = l, t = t, w = w, h = h, b = t + h, l1 = l + 8.0, r1 = l + w - 8.0,
                    stroke = stroke, style = style,
                ));
            }
            Shape::Hexagon => {
                let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
                let k = SLANT_MAX.min(w / 4.0);
                s.push_str(&format!(
                    "<polygon points=\"{a:.1},{cy:.1} {b1:.1},{t:.1} {c:.1},{t:.1} {r:.1},{cy:.1} {c:.1},{b:.1} {b1:.1},{b:.1}\" {style}/>\n",
                    a = l, b1 = l + k, c = r - k, r = r, t = t, b = b, cy = cy, style = style,
                ));
            }
            Shape::Parallelogram | Shape::ParallelogramAlt => {
                let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
                let k = SLANT_MAX.min(w / 4.0);
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
                    "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.1}\" fill=\"{hole}\" \
                     stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n\
                     <circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{ri:.1}\" {style}/>\n",
                    r = w / 2.0,
                    ri = w / 2.0 - 4.0,
                    hole = pal.surface,
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
        // A cylinder's label sits in its body, under the lid.
        let ly = if n.shape == Shape::Cylinder {
            cy + CYLINDER_LABEL_DROP * cylinder_ry(w, h)
        } else {
            cy
        };
        svg_text_multiline(
            &mut s,
            cx,
            ly,
            &style_attr(n.style.color.as_deref(), ink),
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

/// Grow `bb` over the drawing as the layout and edge stages bound it:
/// frames, nodes, edge labels and each drawn edge's `bezier` points
/// (a mermaid `curve` sampled). The router's outer lanes and the frame
/// titles it keeps clear of are measured from it, so it stays as it was
/// while routes are searched; the canvas is [`grow_canvas`].
pub(crate) fn grow_scene(
    bb: &mut Bbox,
    nodes: &[SceneNode],
    edges: &[SceneEdge],
    clusters: &[SceneCluster],
) {
    grow_boxes(bb, nodes, clusters);
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
        grow_label(bb, e);
    }
}

/// Grow `bb` over every frame and node box.
fn grow_boxes(bb: &mut Bbox, nodes: &[SceneNode], clusters: &[SceneCluster]) {
    for c in clusters {
        bb.add(c.x, c.y);
        bb.add(c.x + c.w, c.y + c.h);
    }
    for n in nodes {
        bb.add(n.x - n.w / 2.0, n.y - n.h / 2.0);
        bb.add(n.x + n.w / 2.0, n.y + n.h / 2.0);
    }
}

/// Grow `bb` over edge `e`'s label chip, if it has one.
fn grow_label(bb: &mut Bbox, e: &SceneEdge) {
    if let Some((_, m, w)) = &e.label {
        bb.add(m.0 - w / 2.0, m.1 - LABEL_BOX_H / 2.0);
        bb.add(m.0 + w / 2.0, m.1 + LABEL_BOX_H / 2.0);
    }
}

/// Grow `bb` over everything painted: the frames, nodes and edge labels
/// [`grow_scene`] covers, and the ink of every drawn edge
/// ([`grow_drawn_edge`]) — a route the router took round the outermost
/// node, every stroke's width, every end glyph. Never an edge's bezier
/// control points: a bowed back-edge's handles reach far past the curve
/// they shape, and bounding by them left a wide empty strip on one side
/// of the drawing (report Q2: demo 65 px right of the bow against 27 on
/// the left). The canvas — [`to_svg`]'s, a laid-out scene's own
/// (`edgepass::refit`), the quality yardstick's — is this and [`MARGIN`]
/// round it, so nothing drawn is clipped and the margins match.
pub(crate) fn grow_canvas(
    bb: &mut Bbox,
    nodes: &[SceneNode],
    edges: &[SceneEdge],
    clusters: &[SceneCluster],
) {
    grow_boxes(bb, nodes, clusters);
    for e in edges.iter().filter(|e| edge_drawn(e)) {
        grow_label(bb, e);
        grow_drawn_edge(bb, e);
    }
}

/// Grow `bb` over the ink of drawn edge `e`: the curve it is drawn
/// along ([`plain_segs`]: the routed spline through its waypoints, a
/// mermaid `curve`, or its single cubic), to its exact extremes, half
/// its stroke width out; and, on an edge the flowchart painter strokes,
/// each end glyph's box ([`glyph_box`]). The painter's squared ends keep
/// within a few px of that curve outside the glyph ([`square_end`]:
/// [`SQUARE_END_MAX_STRAY`]) and a usual head within its length of the
/// end point, well inside the [`MARGIN`] round it. Cheap (no squaring, no
/// sampling), so every canvas — a drag frame's too — is sized by it;
/// only an edge with a very wide stroke's longer heads ([`arrow_head_len`])
/// is sized by its squared ends ([`painted_edge`]): a head that turns
/// with a tight curve ([`square_end`]'s last resort) reaches farther
/// round its end point than the margin.
fn grow_drawn_edge(bb: &mut Bbox, e: &SceneEdge) {
    let half = edge_stroke_width(e) / 2.0;
    let grown = e.paint_width.is_none()
        && edge_head_len(e) > ARROW_MARKER_PX
        && squared_ends(e) != [false; 2];
    let (start, segs) = if grown {
        let p = painted_edge(e);
        (p.start, p.segs)
    } else {
        plain_segs(e, |p| p)
    };
    let mut from = start;
    for &seg in &segs {
        let (x0, x1, y0, y1) = cubic_bounds(&seg.cubic(from));
        bb.add(x0 - half, y0 - half);
        bb.add(x1 + half, y1 + half);
        from = seg.end();
    }
    if segs.is_empty() || e.paint_width.is_some() {
        return;
    }
    for at_start in [true, false] {
        let m = if at_start {
            e.ends.start_marker(e.kind)
        } else {
            e.ends.end_marker(e.kind)
        };
        let Some(m) = m else { continue };
        let end = if at_start {
            let (from, back) = reverse_segs(start, &segs[..1]);
            end_heading(from, back[0]).map(|d| (start, d))
        } else {
            let n = segs.len();
            let from = if n >= 2 { segs[n - 2].end() } else { start };
            end_heading(from, segs[n - 1]).map(|d| (segs[n - 1].end(), d))
        };
        if let Some((tip, d)) = end {
            for q in glyph_box(tip, d, glyph_px(e, m)) {
                bb.add(q.0, q.1);
            }
        }
    }
}

/// The corners of the box an end glyph `px` px across is drawn in, its
/// tip (an arrowhead's point, a cross's or circle's outer side) on the
/// end point `tip`, `d` the unit heading into that end: `px` back along
/// the stroke, `px / 2` either side of it — the marker's own view box
/// ([`marker_def_sized`]), which clips whatever it draws.
fn glyph_box(tip: (f64, f64), d: (f64, f64), px: f64) -> [(f64, f64); 4] {
    let (n, h) = ((-d.1, d.0), px / 2.0);
    let base = (tip.0 - d.0 * px, tip.1 - d.1 * px);
    [
        (tip.0 + n.0 * h, tip.1 + n.1 * h),
        (tip.0 - n.0 * h, tip.1 - n.1 * h),
        (base.0 + n.0 * h, base.1 + n.1 * h),
        (base.0 - n.0 * h, base.1 - n.1 * h),
    ]
}

/// Most (px, long and across) the flowchart painter draws edge `e`'s end
/// glyph `m` at: [`ARROW_MARKER_PX`], or a very wide stroke's longer
/// arrowhead ([`edge_head_len`]); [`head_sizes`] only ever narrows or
/// shortens one.
fn glyph_px(e: &SceneEdge, m: Marker) -> f64 {
    if m == Marker::Arrow {
        edge_head_len(e)
    } else {
        ARROW_MARKER_PX
    }
}

/// The corners of each end glyph's box as the flowchart painter draws
/// `e` ([`glyph_box`] on the painted end, [`edge_end_tangent`]): what
/// the quality yardstick checks the canvas holds. None on an edge an ER
/// or class painter strokes (it draws its own ends).
pub(crate) fn painted_glyph_corners(e: &SceneEdge) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    if !edge_drawn(e) || e.paint_width.is_some() {
        return out;
    }
    for at_start in [true, false] {
        let m = if at_start {
            e.ends.start_marker(e.kind)
        } else {
            e.ends.end_marker(e.kind)
        };
        if let Some(m) = m {
            let (tip, d) = edge_end_tangent(e, at_start);
            if d != (0.0, 0.0) {
                out.extend(glyph_box(tip, d, glyph_px(e, m)));
            }
        }
    }
    out
}

/// Tight bounds `(minx, maxx, miny, maxy)` of the cubic `b`: its ends
/// and, on each axis, where its tangent turns level or plumb.
fn cubic_bounds(b: &[(f64, f64); 4]) -> (f64, f64, f64, f64) {
    let axis = |p0: f64, p1: f64, p2: f64, p3: f64| -> (f64, f64) {
        let (mut lo, mut hi) = (p0.min(p3), p0.max(p3));
        // Handles within the ends: the curve keeps within them (hull).
        if p1 >= lo && p1 <= hi && p2 >= lo && p2 <= hi {
            return (lo, hi);
        }
        // B'(t) / 3 = a t² + b t + c.
        let (a, b, c) = (
            -p0 + 3.0 * p1 - 3.0 * p2 + p3,
            2.0 * (p0 - 2.0 * p1 + p2),
            p1 - p0,
        );
        let mut roots = [f64::NAN; 2];
        if a.abs() < 1e-12 {
            if b.abs() > 1e-12 {
                roots[0] = -c / b;
            }
        } else {
            let disc = b * b - 4.0 * a * c;
            if disc >= 0.0 {
                let r = disc.sqrt();
                roots = [(-b + r) / (2.0 * a), (-b - r) / (2.0 * a)];
            }
        }
        for t in roots {
            if t > 0.0 && t < 1.0 {
                let u = 1.0 - t;
                let v =
                    u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3;
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        (lo, hi)
    };
    let (x0, x1) = axis(b[0].0, b[1].0, b[2].0, b[3].0);
    let (y0, y1) = axis(b[0].1, b[1].1, b[2].1, b[3].1);
    (x0, x1, y0, y1)
}

fn pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Per-edge offsets so parallel edges (same node pair) separate:
/// [`PARALLEL_GAP`] apart (a node's self-loops always), or, next to a
/// labelled one in a TD / BT flow,
/// far enough that the label (on its own line, half its width either
/// side) clears the neighbouring line by [`PARALLEL_LABEL_AIR`] (report
/// P0 #13: a label between two lines 16 px apart covers the unlabelled
/// one and reads as its). An LR / RL label lies along the flow, its box
/// height across it, and fits the plain gap; a compact branch point's
/// twins get their room from the layout, labels beside their lines.
pub(crate) fn parallel_offsets(g: &Graph) -> Vec<f64> {
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    // Half the breadth each edge's label takes across the flow (0 in LR
    // / RL and without a label).
    let branch = |v: usize| crate::layout::is_branch_point(g.nodes[v].shape, &g.nodes[v].label);
    let half = |e: &Edge| {
        // A compact branch point's twins get their label room from the
        // layout instead (`fan_twin_room`), their labels beside the lines.
        // A node's self-loops stack ALONG the flow ([`loop_points`] puts
        // each `off` along its side), so a label's breadth across the
        // flow says nothing about their pitch: widening it moved each
        // loop off its node by about its label's width, ends and heads
        // in empty space (B3 review r5 #1). Their labels sit past their
        // loops' apexes and keep apart there (`avoid_label_overlaps`).
        if e.from == e.to || horizontal || branch(e.from) || branch(e.to) {
            0.0
        } else {
            crate::lanes::label_breadth(e.label.as_deref(), false) / 2.0
        }
    };
    // Per pair: each member's position along the run, from 0.
    let mut runs: HashMap<(usize, usize), (f64, f64)> = HashMap::new(); // (last pos, last half)
    let pos: Vec<f64> = g
        .edges
        .iter()
        .map(|e| {
            let h = half(e);
            let r = runs.entry(pair(e.from, e.to)).or_insert((f64::NAN, 0.0));
            let p = if r.0.is_nan() {
                0.0
            } else {
                let labelled = h > 0.0 || r.1 > 0.0;
                let gap = if labelled {
                    PARALLEL_GAP.max(h + r.1 + PARALLEL_LABEL_AIR)
                } else {
                    PARALLEL_GAP
                };
                r.0 + gap
            };
            *r = (p, h);
            p
        })
        .collect();
    g.edges
        .iter()
        .zip(&pos)
        .map(|(e, &p)| p - runs[&pair(e.from, e.to)].0 / 2.0)
        .collect()
}

/// Per edge of `g`: how far (flow coordinates, across the flow) its
/// curve's two controls bow out — the two edges of a 2-cycle drawn as
/// single curves across one gap (`curve(ei)`) in LR / RL, one of them
/// labelled, each away from its twin by [`TWIN_LABEL_BOW`] on the side
/// its parallel offset `offs` puts it (0 for every other edge). The
/// labels lie along the flow there, each centred on its own line: from
/// two ports a 38 px side apart the twins ran too close for a label
/// between them, and the return edge's label covered its twin near its
/// end (wide-labels). Bowed apart, each label rides its own line clear
/// of the other's, and the curves still leave and enter their sides
/// within a few degrees of the normal.
fn twin_bows(g: &Graph, offs: &[f64], curve: impl Fn(usize) -> bool) -> Vec<f64> {
    let mut out = vec![0.0; g.edges.len()];
    if !matches!(g.direction, Direction::LR | Direction::RL) {
        return out;
    }
    for (i, e) in g.edges.iter().enumerate() {
        if e.from == e.to || !curve(i) || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let twins = g.edges.iter().enumerate().filter(|&(j, f)| {
            j != i
                && f.from == e.to
                && f.to == e.from
                && curve(j)
                && !matches!(f.kind, EdgeKind::Invisible)
        });
        let mut any = false;
        let mut labelled = e.label.is_some();
        for (_, f) in twins {
            any = true;
            labelled |= f.label.is_some();
        }
        if any && labelled {
            out[i] = offs.get(i).map_or(0.0, |o| o.signum()) * TWIN_LABEL_BOW;
        }
    }
    out
}

/// `pts` (flow coordinates) bowed `bow` across the flow ([`twin_bows`]):
/// the control at the end lying less far out moves out first, as far
/// as the other end lies further out (the curve then swings out early
/// and runs on level into the other end, never past it and back); what
/// is left of the bow moves both controls.
fn bowed(mut pts: [(f64, f64); 4], bow: f64) -> [(f64, f64); 4] {
    // No more than each end leaves within TWIN_BOW_TILT of square: in a
    // short gap the full bow swung the curve out of its source at 35 deg
    // (state `lr_right`, a 74 px gap), wider than any label clearance is
    // worth.
    let reach = (pts[1].1 - pts[0].1).abs().min((pts[3].1 - pts[2].1).abs());
    let bow = bow.signum() * bow.abs().min(reach * TWIN_BOW_TILT.to_radians().tan());
    if bow == 0.0 {
        return pts;
    }
    let s = bow.signum();
    let room = (pts[3].0 - pts[0].0) * s;
    let first = bow.abs().min(room.abs());
    // The source lies less far out: its control swings out.
    let near = if room > 0.0 { 1 } else { 2 };
    pts[near].0 += s * first;
    let rest = bow.abs() - first;
    pts[1].0 += s * rest;
    pts[2].0 += s * rest;
    pts
}

/// Per edge of `g`: for the return edge (`back(ei)`) of a 2-cycle drawn
/// as single curves across one gap (`curve(ei)`, as for [`twin_bows`]) —
/// the only two edges between their ends —
/// the breadth side its parallel offset `offs` puts it on (-1 low, +1
/// high), 0 for every other edge: the return edge [`skewed`] away from
/// its twin along the flow where no [`twin_bows`] bow parts them. Only
/// the return edge moves: the forward one keeps the course the layout
/// cleared for it (state r215: skewed too, `S0 -> T0` clipped the
/// corner of the frame beside it and the router sent it round, across
/// its twin).
fn twin_sides(
    g: &Graph,
    offs: &[f64],
    curve: impl Fn(usize) -> bool,
    back: impl Fn(usize) -> bool,
) -> Vec<f64> {
    let drawn = |e: &Edge| e.from != e.to && !matches!(e.kind, EdgeKind::Invisible);
    g.edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            // Exactly the two of them: among more parallel edges a move
            // away from one runs into the next (q27's `n4 ==> n0` beside
            // three `n0 --> n4`).
            let pair = |f: &Edge| {
                drawn(f) && (f.from == e.from && f.to == e.to || f.from == e.to && f.to == e.from)
            };
            let twin = drawn(e)
                && back(i)
                && curve(i)
                && g.edges.iter().filter(|f| pair(f)).count() == 2
                && g.edges.iter().enumerate().any(|(j, f)| {
                    j != i && f.from == e.to && f.to == e.from && drawn(f) && curve(j)
                });
            if twin {
                offs.get(i).map_or(0.0, |o| o.signum())
            } else {
                0.0
            }
        })
        .collect()
}

/// `pts` (flow coordinates), a 2-cycle's return curve on breadth side
/// `side` of its twin ([`twin_sides`]), skewed along the flow away from
/// the twin; `None` where it needs no skew. Twins whose ends lie far
/// apart across a short gap run their middles nearly across the flow,
/// there only their ports' spacing times the slope apart (state r215:
/// 60 px within 6 px of each other, one thick band, and r085's return
/// edge was taken off its twin by the router and run beside a note
/// link); the return curve's middle moves until the two lie
/// [`PARALLEL_GAP`] apart square to their run — a labelled one as far
/// as its chip (`chip`: its extents across and along the flow) reaches
/// square to the run, plus [`crate::clearance::EDGE_SPACING`], so the
/// label riding its middle keeps off the twin (r175: `back` over `S0 ->
/// T0`) — and not at all where the run is steep enough already (within
/// a px of it) or runs nearer along the flow than across it. Only the control of the
/// end the middle moves away from moves, along the flow and out from
/// its end — that tangent grows (by at most its own length, so the
/// control never passes the other end), the middle moves 3/8 of it, and
/// the other end keeps its tangent: neither end swings in any harder
/// (shortening the other tangent instead tilted r036's return head 25
/// deg). On the low side the middle moves downstream when it heads to
/// higher breadths downstream (it lies below its twin there), upstream
/// otherwise; on the high side the other way.
fn skewed(mut pts: [(f64, f64); 4], side: f64, chip: (f64, f64)) -> Option<[(f64, f64); 4]> {
    let dir = (
        pts[3].0 + pts[2].0 - pts[1].0 - pts[0].0,
        pts[3].1 + pts[2].1 - pts[1].1 - pts[0].1,
    );
    let len = dir.0.hypot(dir.1);
    if side == 0.0 || len.is_nan() || len <= 0.0 || dir.0 == 0.0 {
        return None;
    }
    let (sin, cos) = (dir.1.abs() / len, dir.0.abs() / len);
    // A run nearer along the flow than across it gains little from a
    // move along the flow: its twin lies across the flow from it already.
    if cos < 0.5 {
        return None;
    }
    // The separation wanted square to the run: a label riding the middle
    // keeps its chip off the twin too.
    let reach = if chip.0 > 0.0 {
        chip.0 / 2.0 * sin + chip.1 / 2.0 * cos + crate::clearance::EDGE_SPACING
    } else {
        0.0
    };
    // The separation missing along the flow between the middles.
    let missing = (PARALLEL_GAP.max(reach) - PARALLEL_GAP * sin) / cos;
    if missing.is_nan() || missing < 1.0 {
        return None;
    }
    let toward = -side * (dir.0 * dir.1).signum();
    // The end the middle moves away from, and its control.
    let (end, ctrl) = if (pts[3].1 - pts[0].1) * toward > 0.0 {
        (0, 1)
    } else {
        (3, 2)
    };
    let mut grow = (missing / 0.375).min((pts[ctrl].1 - pts[end].1).abs());
    // The other end's last arrowhead length bends harder the further
    // the middle moves toward it: no further than TWIN_SKEW_BEND.
    let far = 3 - end;
    let tilt = |g: f64| {
        let mut q = pts;
        q[ctrl].1 += toward * g;
        let tip = q[far];
        let mut t = 0.0;
        let mut at = tip;
        while t < 1.0 && (at.0 - tip.0).hypot(at.1 - tip.1) < ARROW_MARKER_PX {
            t += 1.0 / 128.0;
            at = cubic_at(&q, if far == 0 { t } else { 1.0 - t });
        }
        (at.0 - tip.0)
            .abs()
            .atan2((at.1 - tip.1).abs())
            .to_degrees()
    };
    let allowed = TWIN_SKEW_BEND;
    if tilt(grow) > allowed {
        let (mut lo, mut hi) = (0.0, grow);
        for _ in 0..10 {
            let mid = (lo + hi) / 2.0;
            if tilt(mid) > allowed {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        grow = lo;
    }
    // Less than a px of the middle's move is none.
    if grow * 0.375 < 1.0 {
        return None;
    }
    pts[ctrl].1 += toward * grow;
    Some(pts)
}

/// A twin curve's shape: [`bowed`] by `bow` ([`twin_bows`]), then, for
/// the return edge on `side` ([`twin_sides`]), [`skewed`] — a bow kept
/// within [`TWIN_BOW_TILT`] parts twins whose ends lie far apart across
/// a short gap by a few px only (state r085). A return curve it skews
/// takes the static layout's tangents first (half the gap, at least 20
/// px, as [`edge_points`] draws a curve across one gap): [`route`]'s
/// free curves take [`TANGENT_SHARE`] of it, and skewed from there the
/// return head swung in at 12 deg against 10 in `scene()`.
fn twin_curve(pts: [(f64, f64); 4], bow: f64, side: f64, chip: (f64, f64)) -> [(f64, f64); 4] {
    let plain = bowed(pts, bow);
    if side == 0.0 {
        return plain;
    }
    let mut q = pts;
    if q[1].0 == q[0].0 && q[2].0 == q[3].0 {
        let s = (q[3].1 - q[0].1).signum();
        let dl = ((q[3].1 - q[0].1).abs() * 0.5).max(20.0);
        q[1].1 = q[0].1 + s * dl;
        q[2].1 = q[3].1 - s * dl;
    }
    skewed(bowed(q, bow), side, chip).unwrap_or(plain)
}

/// The extents (px) of `e`'s label chip across and along the flow
/// (flow coordinates; a chip is wide on screen, so across the flow in
/// TD / BT), `(0, 0)` without a label.
fn chip_extents(e: &Edge, dir: Direction) -> (f64, f64) {
    let Some(l) = e.label.as_deref() else {
        return (0.0, 0.0);
    };
    let w = text_width(l) + 14.0;
    match dir {
        Direction::LR | Direction::RL => (LABEL_BOX_H, w),
        Direction::TD | Direction::BT => (w, LABEL_BOX_H),
    }
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

/// How far (px) a fork/join bar reaches past the outermost branch it
/// spans ([`fit_bars`]): more than [`crate::ports::PORT_INSET`], so the
/// port right over the outermost branch still lies on the usable span.
const BAR_OVERHANG: f64 = 16.0;

/// Host-given node sizes made usable: a negative or non-finite (NaN,
/// infinite) width or height becomes 0 — a point or line node whose
/// edges meet at its centre, see [`route_sized`] — so no box arithmetic
/// downstream meets a NaN. Valid sizes pass through untouched (borrowed
/// when all are valid).
pub(crate) fn sane_sizes(sizes: &[(f64, f64)]) -> Cow<'_, [(f64, f64)]> {
    let ok = |v: f64| v.is_finite() && v >= 0.0;
    if sizes.iter().all(|&(w, h)| ok(w) && ok(h)) {
        return Cow::Borrowed(sizes);
    }
    let fix = |v: f64| if ok(v) { v } else { 0.0 };
    Cow::Owned(sizes.iter().map(|&(w, h)| (fix(w), fix(h))).collect())
}

/// Node sizes with every fork/join bar fitted to its branches: laid
/// across the flow (upright in `LR`/`RL`, mermaid-style) and long enough
/// to reach every state on its busier side, so each branch drops
/// straight onto its state (see [`anchor`]) instead of S-bending off a
/// 60px bar. The reach is estimated before layout ([`bar_half`]), so the
/// static and the interactive builders fit the same sizes and a drag
/// never resizes a bar; fitting fitted sizes again changes nothing.
/// A bar in a composite laid out in its own direction
/// ([`crate::compound`]) lies across that direction's flow.
/// Other sizes pass through untouched (borrowed when there is no bar).
pub(crate) fn fit_bars<'a>(g: &Graph, sizes: &'a [(f64, f64)]) -> Cow<'a, [(f64, f64)]> {
    if !g.nodes.iter().any(|n| n.shape == Shape::ForkBar) {
        return Cow::Borrowed(sizes);
    }
    // Whether the flow round node `v` runs along x: the diagram's, or
    // its block's own.
    let flows = g
        .subgraphs
        .iter()
        .any(|s| s.direction.is_some())
        .then(|| crate::compound::Flows::new(g));
    let across = |v: usize| {
        let d = flows
            .as_ref()
            .map_or(g.direction, |f| f.of(End::Node(v), End::Node(v)));
        matches!(d, Direction::LR | Direction::RL)
    };
    let is_bar = |v: usize| g.nodes[v].shape == Shape::ForkBar;
    // A state's breadth across the flow (a bar inside a composite counts
    // at an unfitted bar's length, see below).
    let breadth = |v: usize, across: bool| match (is_bar(v), across) {
        (true, _) => BAR_SIZE.0,
        (false, true) => sizes[v].1,
        (false, false) => sizes[v].0,
    };
    let end_breadth = |end: End, across: bool| match end {
        End::Node(v) => breadth(v, across),
        End::Sub(si) => frame_breadth(g, si, &|v| breadth(v, across)),
    };
    let bar = |end: End| match end {
        End::Node(v) if is_bar(v) => Some(v),
        _ => None,
    };
    // Every (from, to) a bar may span: the states it joins, composite
    // ones (frames) included.
    let ends: Vec<(End, End)> = g
        .edges
        .iter()
        .filter(|e| bar_counts(g, e))
        .map(|e| (End::Node(e.from), End::Node(e.to)))
        .chain(
            g.sub_edges
                .iter()
                .filter(|e| e.from != e.to && e.kind != EdgeKind::Invisible)
                .map(|e| (e.from, e.to)),
        )
        .collect();
    // A bar with a note card beside it: its link leaves past the
    // branches, a port's room beyond the outermost one at either end
    // (the layout leaves the card beside the branches, see
    // `layout::unlink_bar_cards`).
    let card = |v: usize| {
        let n = &g.nodes[v];
        crate::style::is_note_card(&n.id, n.shape, &n.style)
    };
    let noted: Vec<usize> = g
        .edges
        .iter()
        .filter_map(|e| match (card(e.from), card(e.to)) {
            (false, true) => Some(e.from),
            (true, false) => Some(e.to),
            _ => None,
        })
        .filter(|&v| is_bar(v))
        .collect();
    // Per bar: the length (0 with no fan) that reaches every branch on
    // its busier side (in, out), `b(branch, bar)` the branch's breadth.
    let reach = |b: &dyn Fn(End, usize) -> f64| -> HashMap<usize, f64> {
        let mut fan: HashMap<usize, [Vec<f64>; 2]> = HashMap::new();
        for &(from, to) in &ends {
            if let Some(v) = bar(to) {
                fan.entry(v).or_default()[0].push(b(from, v));
            }
            if let Some(v) = bar(from) {
                fan.entry(v).or_default()[1].push(b(to, v));
            }
        }
        fan.into_iter()
            .map(|(v, mut sides)| {
                let half = sides.iter_mut().map(|b| bar_half(b)).fold(0.0, f64::max);
                let card = if noted.contains(&v) {
                    crate::ports::PORT_SPACING
                } else {
                    0.0
                };
                let len = if half > 0.0 {
                    2.0 * (half + BAR_OVERHANG + card)
                } else {
                    0.0
                };
                (v, len)
            })
            .collect()
    };
    // A bar branching to another bar (`f --> j`, nested forks) cannot
    // read the far bar's fitted length: the two would feed each other,
    // each refit stretching both, and fitting fitted sizes again
    // (`fold::scene_compact_sized`, a host handing `route_sized` the
    // sizes it drew) must change nothing. So every length here comes
    // from the states alone: a far bar counts at an unfitted bar's
    // length, then, when it lies right beside the bar (nested forks), at
    // the fit that gives it. When another path joins the two (a fork
    // straight to its join), the edge spans layers: the layer beside the
    // bar holds its lane, not the far bar.
    let first = reach(&|end, near| end_breadth(end, across(near)));
    let lengths = reach(&|end, near| match bar(end) {
        Some(far) if beside(g, near, far) => {
            first.get(&far).copied().unwrap_or(0.0).max(BAR_SIZE.0)
        }
        _ => end_breadth(end, across(near)),
    });
    let mut out = sizes.to_vec();
    for v in (0..g.nodes.len()).filter(|&v| is_bar(v)) {
        let (w, h) = sizes[v];
        let len = w.max(h).max(lengths.get(&v).copied().unwrap_or(0.0));
        out[v] = if across(v) {
            (w.min(h), len)
        } else {
            (len, w.min(h))
        };
    }
    Cow::Owned(out)
}

/// How far from its centre a bar must reach to meet the centres of the
/// states it fans out to (breadths `b`; 0 for fewer than two), laid side
/// by side a node gap apart in an order the layout picks. An even fan
/// centres the bar between its middle two: half the widest span, with
/// the two narrowest states at the ends. An odd one aligns the bar with
/// its middle state: the farthest reach puts the `k / 2 + 1` broadest on
/// one side, the two narrowest of those at the middle and the end.
fn bar_half(b: &mut [f64]) -> f64 {
    let k = b.len();
    if k < 2 {
        return 0.0;
    }
    b.sort_by(|x, y| y.total_cmp(x));
    let ends = |top: &[f64]| (top[top.len() - 1] + top[top.len() - 2]) / 2.0;
    if k % 2 == 0 {
        (b.iter().sum::<f64>() - ends(b) + (k - 1) as f64 * GAP_B) / 2.0
    } else {
        let top = &b[..=k / 2];
        top.iter().sum::<f64>() - ends(top) + (k / 2) as f64 * GAP_B
    }
}

/// Breadth of composite state (frame) `si` before layout, its members'
/// breadths `breadth`: its broadest member (a nested frame estimated
/// alike) plus the frame's pad each side, since a frame lays out its
/// members along the diagram's own flow.
fn frame_breadth(g: &Graph, si: usize, breadth: &dyn Fn(usize) -> f64) -> f64 {
    let nested = g
        .subgraphs
        .iter()
        .enumerate()
        .filter(|&(ci, c)| c.parent == Some(si) && ci != si)
        .map(|(ci, _)| frame_breadth(g, ci, breadth));
    g.subgraphs[si]
        .nodes
        .iter()
        .map(|&v| breadth(v))
        .chain(nested)
        .fold(0.0, f64::max)
        + 2.0 * SUB_PAD
}

/// Whether nodes `a` and `b` (an edge joins them) lie in neighbouring
/// layers as far as the graph tells before layout: no other path of
/// [`bar_counts`] edges leads from one to the other (which would push
/// the far one a layer further, its edge a long one).
fn beside(g: &Graph, a: usize, b: usize) -> bool {
    let detour = |from: usize, to: usize| {
        let mut seen = vec![false; g.nodes.len()];
        seen[from] = true;
        let mut stack: Vec<usize> = g
            .edges
            .iter()
            .filter(|e| e.from == from && e.to != to && bar_counts(g, e))
            .map(|e| e.to)
            .collect();
        while let Some(v) = stack.pop() {
            if v == to {
                return true;
            }
            if std::mem::replace(&mut seen[v], true) {
                continue;
            }
            stack.extend(
                g.edges
                    .iter()
                    .filter(|e| e.from == v && bar_counts(g, e))
                    .map(|e| e.to),
            );
        }
        false
    };
    !detour(a, b) && !detour(b, a)
}

/// Whether an edge counts towards a fork/join bar's branches: drawn,
/// joining two nodes, and not a note card's link (a note beside a bar
/// is no branch of it).
fn bar_counts(g: &Graph, e: &Edge) -> bool {
    let card = |v: usize| {
        let n = &g.nodes[v];
        crate::style::is_note_card(&n.id, n.shape, &n.style)
    };
    e.from != e.to && e.kind != EdgeKind::Invisible && !card(e.from) && !card(e.to)
}

/// Per node: the (incoming, outgoing) edges it has, when the graph has
/// a fork/join bar (else empty) — what [`lone_ends`] reads.
fn bar_degrees(g: &Graph) -> Vec<(usize, usize)> {
    if !g.nodes.iter().any(|n| n.shape == Shape::ForkBar) {
        return Vec::new();
    }
    let mut deg = vec![(0, 0); g.nodes.len()];
    for e in g.edges.iter().filter(|e| bar_counts(g, e)) {
        deg[e.from].1 += 1;
        deg[e.to].0 += 1;
    }
    for e in &g.sub_edges {
        if let End::Node(v) = e.from {
            deg[v].1 += 1;
        }
        if let End::Node(v) = e.to {
            deg[v].0 += 1;
        }
    }
    deg
}

/// No end of the edge is a bar side's only edge.
const NO_LONE: (bool, bool) = (false, false);

/// For edge `e`: is its start the only edge leaving a fork/join bar,
/// and is its end the only edge entering one? Such a side (a fork's
/// way in, a join's way out) meets its edge at the bar's middle.
fn lone_ends(g: &Graph, deg: &[(usize, usize)], e: &Edge) -> (bool, bool) {
    // A note card's link is no branch: it meets the bar where the card
    // lies, past the branches.
    if !bar_counts(g, e) {
        return NO_LONE;
    }
    let bar = |v: usize| g.nodes[v].shape == Shape::ForkBar;
    let one = |v: usize, out: bool| {
        deg.get(v)
            .is_some_and(|d| (if out { d.1 } else { d.0 }) == 1)
    };
    (
        bar(e.from) && one(e.from, true),
        bar(e.to) && one(e.to, false),
    )
}

/// Half the stretch of a fork/join bar its branches may meet it on.
fn bar_reach(p: &Placed) -> f64 {
    (p.bsize / 2.0 - crate::ports::PORT_INSET).max(0.0)
}

/// The breadth coordinate the end at `me` aims its port at (the
/// `other` of [`anchor`]): the far end's port when that is a fork/join
/// bar — which meets each branch right where the branch lies — else the
/// far node's centre. A bar end alone on its side (`me_lone`) aims at
/// its own middle, and an edge into such a side at that middle too.
fn toward(me: (&Placed, Shape, bool), far: (&Placed, Shape, bool)) -> f64 {
    let ((p, shape, lone), (q, far_shape, far_lone)) = (me, far);
    if shape == Shape::ForkBar && lone {
        p.b
    } else if far_shape == Shape::ForkBar && !far_lone {
        let r = bar_reach(q);
        p.b.max(q.b - r).min(q.b + r)
    } else {
        q.b
    }
}

/// How far (along the flow-facing side, from the centre) a boxy end's
/// port leans toward the other end, `toward` (its breadth less this
/// box's centre) away: 0.35 of the way, shifted by the parallel-edge
/// offset `off`, kept on the straight part of the side (`size` is the
/// box's (breadth, layer) size). See [`anchor`].
pub(crate) fn port_lean(shape: Shape, size: (f64, f64), toward: f64, off: f64) -> f64 {
    let flat = match shape {
        Shape::Stadium => size.0 / 2.0 - size.1 / 2.0 - 4.0,
        Shape::Hexagon | Shape::Parallelogram | Shape::ParallelogramAlt => slant_flat_half(size.0),
        _ => size.0 / 2.0 - 12.0,
    }
    .max(0.0);
    (toward * 0.35 + off).clamp(-flat, flat)
}

/// How far either way from its middle an end may lean along the side of
/// a hexagon or a parallelogram `bsize` px across that faces along the
/// flow: the straight run between the slants (a slant at most
/// [`SLANT_MAX`] in from each corner), a port inset short of them. An
/// end leaned further lands on the slant, its head half over the
/// outline's corner (advanced's `fail` lane: its tip past Developer's
/// bottom-right vertex). Shared by [`port_lean`] and
/// [`crate::lanes::port_span`]; may be negative on a narrow node.
pub(crate) fn slant_flat_half(bsize: f64) -> f64 {
    bsize / 2.0 - 12.0f64.max(SLANT_MAX.min(bsize / 4.0) + crate::ports::PORT_INSET)
}

/// Edge exit/entry point on a node's border.
///
/// For boxy shapes, points are spread along the top/bottom side
/// towards the opposite node (instead of piling up at the centre);
/// stadiums are constrained to their flat section so anchors don't
/// float on the rounded caps. Circles/diamonds use exact border
/// intersection. A fork/join bar meets the edge right where `other`
/// lies (clamped to the bar), so a branch drops straight down to it.
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
        Shape::ForkBar => {
            let r = bar_reach(p);
            let l = if bottom {
                p.l + p.lsize / 2.0
            } else {
                p.l - p.lsize / 2.0
            };
            // `max`/`min`, not `clamp`: a NaN centre must not panic.
            ((other.0 + off).max(p.b - r).min(p.b + r), l)
        }
        _ => {
            let bias = port_lean(shape, (p.bsize, p.lsize), other.0 - p.b, off);
            let l = if bottom {
                p.l + p.lsize / 2.0
            } else {
                p.l - p.lsize / 2.0
            };
            (p.b + bias, l)
        }
    }
}

/// `pt`, an end on the border of `p` (flow coordinates) of a back-edge
/// drawn across the gap, kept on the part of a diamond's or a circle's
/// border that faces along the flow — its side facing upstream (`up`)
/// or downstream — within the span the ports stage spreads that side's
/// ends over ([`crate::lanes::port_span`]). `anchor` aims such an end
/// at its other end shifted by its parallel offset, which can carry it
/// round onto the diamond's flank: there the ports stage took it for an
/// end on that flank and never spaced it from its twin's on the flow
/// side, 4 px away (r048: `Parse emit --> Emit`'s head on the start of
/// `Emit --> Parse emit` at Emit's top). Any other shape: as it is.
fn on_flow_side(p: &Placed, shape: Shape, pt: (f64, f64), up: bool) -> (f64, f64) {
    if !matches!(
        shape,
        Shape::Diamond | Shape::Circle | Shape::DoubleCircle | Shape::StateStart | Shape::StateEnd
    ) {
        return pt;
    }
    let (lo, hi) = crate::lanes::port_span(shape, p);
    if pt.0 >= lo && pt.0 <= hi {
        return pt;
    }
    crate::lanes::port_at(shape, p, pt.0.max(lo).min(hi), up)
}

/// Edge geometry for the layered layout (used by `scene`). A back-edge
/// that comes here has no lane ([`crate::lanes`]: nothing lies between
/// its ends): it crosses the gap as a forward edge does, leaving its
/// source's flow-in side and entering its target's flow-out side.
/// `looped` holds a self-loop's stack span ([`loop_spans`]), `None` for
/// any other edge.
#[allow(clippy::too_many_arguments)]
fn edge_points(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    looped: Option<f64>,
    total_b: f64,
    off: f64,
    lone: (bool, bool),
) -> [(f64, f64); 4] {
    if let Some(span) = looped {
        return loop_points(a, off, span);
    }

    if b.layer != a.layer {
        let down = b.layer > a.layer;
        let (ea, eb) = ((a, sa, lone.0), (b, sb, lone.1));
        let p0 = anchor(a, sa, (toward(ea, eb), b.l), off, down);
        let p3 = anchor(b, sb, (toward(eb, ea), a.l), off, !down);

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
        // Back across the gap: the forward curve run the other way.
        let (p0, p3) = (
            on_flow_side(a, sa, p0, true),
            on_flow_side(b, sb, p3, false),
        );
        let dl = ((p0.1 - p3.1) * 0.5).max(20.0);
        return [p0, (p0.0, p0.1 - dl), (p3.0, p3.1 + dl), p3];
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

/// A lane's curve, waypoints and label spot ([`lane_geometry`]).
type LaneGeometry = ([(f64, f64); 4], Vec<(f64, f64)>, (f64, f64));

/// A back-edge's `lane` from `a` (downstream) to `b`, in
/// flow coordinates ([`crate::lanes::corners`]): its curve (through the
/// lane's corners, an approximation of the drawn path for a host that
/// draws `bezier`), its waypoints (the corners rounded as the router
/// rounds its paths, [`crate::router::smooth`], so a host's spline
/// through them draws the same rounded orthogonal path) and its label
/// spot (the middle of the lane).
fn lane_geometry(
    a: (&Placed, Shape),
    b: (&Placed, Shape),
    lane: crate::lanes::Lane,
    side: [bool; 2],
) -> LaneGeometry {
    let x = lane.x;
    let c = crate::lanes::corners(a, b, lane, side);
    let (s0, s3) = crate::lanes::legs(a.0, b.0, &lane);
    let (p0, p3) = (c[0], c[c.len() - 1]);
    // The lane's own run: from its first corner on the lane to its last.
    let on = |p: &(f64, f64)| (p.0 - x).abs() < 1e-6;
    let c1 = c.iter().copied().find(on).unwrap_or((x, s0));
    let c2 = c.iter().copied().rfind(on).unwrap_or((x, s3));
    let wps = crate::router::smooth(&c, crate::router::CORNER_R);
    ([p0, c1, c2, p3], wps, (x, (s0 + s3) / 2.0))
}

/// `lane` of back-edge `e` with the free gap beyond each end's side
/// ([`crate::lanes::room`]) among the nodes `at` (flow coordinates,
/// index-parallel with `g.nodes`), which caps its stubs, and the least
/// stub that takes a leg out past a frame the lane leaves its end's box
/// through (`frames`: per subgraph, its extent along the flow,
/// [`flow_frame_spans`]) when that frame's border lies within
/// [`crate::lanes::LANE_STUB`] + [`crate::clearance::CLEAR_GAP`] of the end's side: a leg
/// there would run along the border (G4).
fn lane_room(
    g: &Graph,
    e: &Edge,
    lane: crate::lanes::Lane,
    at: &[Placed],
    frames: &[Option<(f64, f64)>],
) -> crate::lanes::Lane {
    use crate::lanes::LANE_STUB;
    let (a, b) = (&at[e.from], &at[e.to]);
    let mut out = [0.0f64; 2];
    if !frames.is_empty() {
        let paths = node_cluster_paths(g);
        let (pa, pb) = (&paths[e.from], &paths[e.to]);
        // Per end: its frames the other end is not in, and how far along
        // the flow beyond the end's side such a frame's border lies.
        let ends = [
            (pa, pb, a.l - a.lsize / 2.0, true),
            (pb, pa, b.l + b.lsize / 2.0, false),
        ];
        for (k, (mine, other, side, up)) in ends.into_iter().enumerate() {
            for &si in mine.iter().filter(|si| !other.contains(si)) {
                let Some((l0, l1)) = frames.get(si).copied().flatten() else {
                    continue;
                };
                let d = if up { side - l0 } else { l1 - side };
                if (0.0..LANE_STUB + crate::clearance::CLEAR_GAP).contains(&d) {
                    out[k] = out[k].max(d + LANE_STUB);
                }
            }
        }
    }
    crate::lanes::Lane {
        room: [
            crate::lanes::room(a, true, at.iter()),
            crate::lanes::room(b, false, at.iter()),
        ],
        out,
        ..lane
    }
}

/// Per subgraph of `g`, the extent `(l0, l1)` along the flow of its
/// frame as it is wrapped round the boxes `at` (flow coordinates of
/// direction `dir`, index-parallel with `g.nodes`), as
/// [`cluster_raw_boxes`] wraps it: its members and child frames padded
/// by `SUB_PAD`, its title band on the side that is the screen's top.
/// `None` for an empty subgraph.
fn flow_frame_spans(g: &Graph, at: &[Placed], dir: Direction) -> Vec<Option<(f64, f64)>> {
    let n = g.subgraphs.len();
    let mut depth = vec![0usize; n];
    for (i, d) in depth.iter_mut().enumerate() {
        let mut p = g.subgraphs[i].parent;
        while let Some(pi) = p {
            *d += 1;
            p = g.subgraphs[pi].parent;
        }
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(depth[i]));
    let mut spans: Vec<Option<(f64, f64)>> = vec![None; n];
    for &i in &order {
        let (mut l0, mut l1) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in g.subgraphs[i].nodes.iter().filter_map(|&v| at.get(v)) {
            l0 = l0.min(p.l - p.lsize / 2.0);
            l1 = l1.max(p.l + p.lsize / 2.0);
        }
        for (j, s) in g.subgraphs.iter().enumerate() {
            if s.parent == Some(i) {
                if let Some((c0, c1)) = spans[j] {
                    l0 = l0.min(c0);
                    l1 = l1.max(c1);
                }
            }
        }
        if l0 > l1 {
            continue;
        }
        let head = SUB_HEADER + title_extra(&g.subgraphs[i].title);
        let (h0, h1) = match dir {
            Direction::TD => (head, 0.0),
            Direction::BT => (0.0, head),
            Direction::LR | Direction::RL => (0.0, 0.0),
        };
        spans[i] = Some((l0 - SUB_PAD - h0, l1 + SUB_PAD + h1));
    }
    spans
}

/// Which ends of back-edge `e` its lane at breadth `x` meets on the side
/// facing the lane ([`crate::lanes::meets_side`]), in flow coordinates
/// `at` (index-parallel with `g.nodes`): `heads(k, v)` is the breadth
/// edge `k` heads to from its end at node `v` (its far end, or its
/// first channel point). `loop_side(v)` is the breadth side node `v`'s
/// self-loop is drawn on (+1 high, -1 low, 0 without one): a flank
/// carrying the loop is never met — the turn in would run through the
/// loop, the head stacked on the loop's own (m6: `again` into `Parse`
/// through its loop) — so the lane crosses the end's flow side there,
/// its leg past the end's layer, clear of the loop.
fn lane_sides(
    g: &Graph,
    at: &[Placed],
    e: &Edge,
    x: f64,
    heads: impl Fn(usize, usize) -> f64,
    loop_side: impl Fn(usize) -> f64,
) -> [bool; 2] {
    let level = |p: &Placed, q: &Placed| {
        p.l - p.lsize / 2.0 < q.l + q.lsize / 2.0 && p.l + p.lsize / 2.0 > q.l - q.lsize / 2.0
    };
    [(e.from, true), (e.to, false)].map(|(v, upstream)| {
        let p = &at[v];
        if (x - p.b) * loop_side(v) > 0.0 {
            return false;
        }
        // The end's other edges on the flow side the lane meets: into the
        // source from upstream, out of the target downstream.
        let fans: Vec<f64> = g
            .edges
            .iter()
            .enumerate()
            .filter(|&(_, f)| {
                f.from != f.to
                    && !matches!(f.kind, EdgeKind::Invisible)
                    && !std::ptr::eq(f, e)
                    && if upstream {
                        f.to == v && at[f.from].l < p.l
                    } else {
                        f.from == v && at[f.to].l > p.l
                    }
            })
            .map(|(k, _)| heads(k, v))
            .collect();
        let beside: Vec<(f64, f64)> = at
            .iter()
            .enumerate()
            .filter(|&(w, q)| w != e.from && w != e.to && level(q, p))
            .map(|(_, q)| (q.b - q.bsize / 2.0, q.b + q.bsize / 2.0))
            .collect();
        crate::lanes::meets_side((p, g.nodes[v].shape), x, &fans, &beside)
    })
}

/// How [`route`] draws one of `g.edges` ([`route_backs`]).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Back {
    /// Not a back-edge: a free curve ([`free_edge`]).
    No,
    /// A back-edge with nothing between its ends: across the gap
    /// ([`direct_back`]).
    Direct,
    /// A back-edge in this lane (flow coordinates).
    Lane(crate::lanes::Lane),
}

/// For each of `g.edges`, how [`route`] draws it: a back-edge — its
/// target wholly upstream of its source, by at least [`FLOW_MIN_GAP`] —
/// in its lane or across the gap, by the static layout's rule
/// ([`crate::lanes`], [`crate::layout::back_lanes`]) on the boxes as they
/// now stand (`flow`, flow coordinates): what reaches into its band is
/// every node whose extent along the flow overlaps the gap between its
/// ends by more than [`crate::lanes::LANE_REACH`] (on a layout, the
/// layers strictly between and any node on an end's layer that much
/// longer — the static rule), inside subgraphs scoped to the
/// innermost box holding both ends; crossings are judged against the
/// other edges' centre-to-centre lines. O(nodes + edges) per back-edge,
/// so a drag frame affords it.
fn route_backs(g: &Graph, flow: &[Placed]) -> Vec<Back> {
    use crate::lanes::{
        blocks_direct, crossings, direct_port, judged_path, label_breadth, legs, plan, port_span,
        Lane, Skeleton, Want, LANE_CLEAR, LANE_REACH,
    };
    let drawn = |e: &Edge| e.from != e.to && !matches!(e.kind, EdgeKind::Invisible);
    let gap = |e: &Edge| {
        let (a, b) = (&flow[e.from], &flow[e.to]);
        (a.l - a.lsize / 2.0) - (b.l + b.lsize / 2.0)
    };
    let mut out = vec![Back::No; g.edges.len()];
    // A state note card's link that does not elbow crosses the gap, as
    // the static layout draws it: never a lane (nor an obstacle to one).
    let card = |v: usize| {
        let n = &g.nodes[v];
        crate::style::is_note_card(&n.id, n.shape, &n.style)
    };
    let mut backs: Vec<usize> = Vec::new();
    for (ei, e) in g.edges.iter().enumerate() {
        if !drawn(e) || gap(e) < FLOW_MIN_GAP {
            continue;
        }
        if card(e.from) || card(e.to) {
            out[ei] = Back::Direct;
        } else {
            backs.push(ei);
        }
    }
    if backs.is_empty() {
        return out;
    }
    let skeletons: Vec<Skeleton> = g
        .edges
        .iter()
        .enumerate()
        .filter(|&(ei, e)| drawn(e) && backs.binary_search(&ei).is_err())
        .map(|(_, e)| Skeleton::new((e.from, e.to), forward_skeleton(flow, e.from, e.to)))
        .collect();
    let node_cluster = if g.subgraphs.is_empty() {
        Vec::new()
    } else {
        node_cluster_paths(g)
    };
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    // A self-loop beside a node is in a lane's way too (on its high
    // breadth side, as `free_edge` draws it with the static layout's
    // room there; the low side when a drag crowded that, [`loop_side`]).
    let loops = loop_reach(g);
    let wants: Vec<Want> = backs
        .iter()
        .map(|&ei| {
            let e = &g.edges[ei];
            let (a, b) = (&flow[e.from], &flow[e.to]);
            let (bot, top) = (b.l + b.lsize / 2.0, a.l - a.lsize / 2.0);
            let (near_bot, near_top) = (bot + LANE_REACH, top - LANE_REACH);
            let shared: &[usize] = match node_cluster.get(e.from).zip(node_cluster.get(e.to)) {
                Some((x, y)) => {
                    let k = x.iter().zip(y).take_while(|(p, q)| p == q).count();
                    &x[..k]
                }
                None => &[],
            };
            let (mut obstacles, mut foreign) = (Vec::new(), Vec::new());
            let (mut between, mut beside) = (Vec::new(), Vec::new());
            // Overlapping an end along the flow: beside that end.
            let along = |p: &Placed, q: &Placed| {
                p.l - p.lsize / 2.0 < q.l + q.lsize / 2.0
                    && p.l + p.lsize / 2.0 > q.l - q.lsize / 2.0
            };
            for (v, p) in flow.iter().enumerate() {
                if v == e.from || v == e.to {
                    continue;
                }
                if p.l - p.lsize / 2.0 >= near_top || p.l + p.lsize / 2.0 <= near_bot {
                    continue;
                }
                let span = (p.b - p.bsize / 2.0, p.b + p.bsize / 2.0 + loops[v]);
                if along(p, a) {
                    beside.push((span, true));
                } else if along(p, b) {
                    beside.push((span, false));
                } else {
                    between.push(span);
                }
                if node_cluster
                    .get(v)
                    .map_or(true, |nc| nc.starts_with(shared))
                {
                    obstacles.push(span);
                } else {
                    foreign.push(span);
                }
            }
            let label = label_breadth(e.label.as_deref(), horizontal);
            let (sa, sb) = (g.nodes[e.from].shape, g.nodes[e.to].shape);
            let ports = (direct_port(sa, a, b.b), direct_port(sb, b, a.b));
            Want {
                key: top - bot,
                band: (bot, top),
                obstacles,
                foreign,
                blocked: blocks_direct(&between, &beside, ports, label / 2.0),
                clear: LANE_CLEAR + label / 2.0,
                label_half: label / 2.0,
                spans: [port_span(sa, a), port_span(sb, b)],
                ends: (e.from, e.to),
            }
        })
        .collect();
    let lanes = plan(&wants, |i, x, placed| {
        let e = &g.edges[backs[i]];
        let (a, b) = (&flow[e.from], &flow[e.to]);
        let (s0, s3) = legs(a, b, &Lane::plain(0.0));
        let path = judged_path(x, wants[i].spans, a.l, b.l, s0, s3);
        // The lanes planned so far, as the layout judges them too
        // (`layout::back_lanes`), so both pick the same side.
        let planned = crate::lanes::planned_skeletons(&wants, placed, i, |j, xj| {
            let f = &g.edges[backs[j]];
            let (pa, pb) = (&flow[f.from], &flow[f.to]);
            let (t0, t3) = legs(pa, pb, &Lane::plain(0.0));
            judged_path(xj, wants[j].spans, pa.l, pb.l, t0, t3)
        });
        let ends = (e.from, e.to);
        crossings(&path, ends, wants[i].spans, &skeletons)
            + crossings(&path, ends, wants[i].spans, &planned)
    });
    for (&ei, lane) in backs.iter().zip(lanes) {
        out[ei] = lane.map_or(Back::Direct, Back::Lane);
    }
    out
}

/// The skeleton [`route_backs`] judges a forward edge from node `from`
/// to `to` by, in flow coordinates `flow`: its centre-to-centre chord,
/// stepped round each node it runs through — held at its breadth when
/// the line already lies beside the box there and a run along the flow
/// at that breadth passes clear of the boxes before it, else
/// [`crate::clearance::NODE_SEP`] past the side its target lies on (the
/// side the chord passes nearer when the target lies within the box's
/// breadth too) — and on along the flow from the last such step to the
/// target's level: the shape of the router's orthogonal path round the
/// box (r158: `Done -> Done` holds its source's breadth past the
/// cylinder and turns into its target). The static layout judges a
/// forward edge by its channel points, which the layering threads past
/// the boxes between its ends, and `route()` reroutes a free curve that
/// runs through a box round it: judged by its bare chord through such a
/// box, a lane took the side the edge is drawn on (r280: `again` on the
/// right of `Done store -> Queue validate` in `route()`, across `Store
/// check -> Done store` rerouted there, on the left in `scene()`).
/// O(nodes) per edge.
fn forward_skeleton(flow: &[Placed], from: usize, to: usize) -> Vec<(f64, f64)> {
    let (a, b) = (&flow[from], &flow[to]);
    let (p, q) = ((a.b, a.l), (b.b, b.l));
    let (l0, l1) = (a.l.min(b.l), a.l.max(b.l));
    // The boxes between the ends along the flow, nearest the source
    // first, each judged against the line on from the last step.
    let mut between: Vec<&Placed> = flow
        .iter()
        .enumerate()
        .filter(|&(v, n)| v != from && v != to && n.l > l0 && n.l < l1)
        .map(|(_, n)| n)
        .collect();
    if between.is_empty() {
        return vec![p, q];
    }
    between.sort_by(|x, y| (x.l - p.1).abs().total_cmp(&(y.l - p.1).abs()));
    // Whether a run along the flow at breadth `b` from `l0` to `l1`
    // passes clear of every box between.
    let clear = |b: f64, l0: f64, l1: f64| {
        let (l0, l1) = (l0.min(l1), l0.max(l1));
        !between
            .iter()
            .any(|n| n.l > l0 && n.l <= l1 && (b - n.b).abs() < n.bsize / 2.0)
    };
    let mut pts = vec![p];
    let mut cur = p;
    for &n in &between {
        // Where the line on crosses the box's centre line along the flow.
        let t = (n.l - cur.1) / (q.1 - cur.1);
        let at = cur.0 + t * (q.0 - cur.0);
        let (lo, hi) = (n.b - n.bsize / 2.0, n.b + n.bsize / 2.0);
        if at <= lo || at >= hi {
            continue;
        }
        // Past the box on the side the path is already on (it holds its
        // breadth along the flow past it, as the router's path does), else
        // on the side its target lies, else the nearer one.
        let (below, above) = (
            lo - crate::clearance::NODE_SEP,
            hi + crate::clearance::NODE_SEP,
        );
        let side = if (cur.0 <= lo || cur.0 >= hi) && clear(cur.0, cur.1, n.l) {
            cur.0
        } else if q.0 <= lo {
            below
        } else if q.0 >= hi || at - lo >= hi - at {
            above
        } else {
            below
        };
        cur = (side, n.l);
        pts.push(cur);
    }
    if pts.len() > 1 {
        pts.push((cur.0, q.1));
    }
    pts.push(q);
    pts
}

/// A back-edge with nothing between its ends, from `a` (downstream) to
/// `b`, in flow coordinates: out of `a`'s flow-in side, into `b`'s
/// flow-out side, the forward curve of [`free_edge`] run the other way.
fn direct_back(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    off: f64,
    lone: (bool, bool),
) -> [(f64, f64); 4] {
    let (ea, eb) = ((a, sa, lone.0), (b, sb, lone.1));
    let p0 = on_flow_side(
        a,
        sa,
        anchor(a, sa, (toward(ea, eb), b.l), off, false),
        true,
    );
    let p3 = on_flow_side(
        b,
        sb,
        anchor(b, sb, (toward(eb, ea), a.l), off, true),
        false,
    );
    let dl = tangent(p0.1 - p3.1);
    [p0, (p0.0, p0.1 - dl), (p3.0, p3.1 + dl), p3]
}

/// Edge geometry for free positions (used by `route`): the
/// vertical/horizontal classification follows the actual relative
/// position of the two nodes.
/// Waypoints (final coords) that route a cross-cluster edge OUT of its
/// source cluster/box and INTO the target's, so the edge threads the
/// gap between clusters instead of cutting through their interiors.
/// `a_box`/`b_box` = `(x, y, w, h)` of the endpoint's top-level cluster
/// (or the node's own box when it has no cluster). Returns empty when
/// the clusters aren't cleanly stacked along the flow axis, or stand
/// closer than the two `GAP`s the path steps out and in by (fall back
/// to the direct curve then, as the static layout draws such an edge):
/// there the step out of one box would pass the step into the other
/// and the path double back on itself (B7 review r1 #8: BT, with the
/// frames 18 px apart, `Third row --> Deep` rose 14 px past the gap,
/// fell back and ended in a slanted run under the head).
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
        // flow axis (source fully above target, or vice-versa), with
        // room for both steps.
        let ok = if down {
            a_box.1 + a_box.3 + 2.0 * GAP <= b_box.1
        } else {
            b_box.1 + b_box.3 + 2.0 * GAP <= a_box.1
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
            a_box.0 + a_box.2 + 2.0 * GAP <= b_box.0
        } else {
            b_box.0 + b_box.2 + 2.0 * GAP <= a_box.0
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
/// `looped` holds a self-loop's stack span ([`loop_spans`]), `None` for
/// any other edge.
#[allow(clippy::too_many_arguments)]
fn free_edge(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    looped: Option<f64>,
    off: f64,
    dir: Option<Direction>,
    lone: (bool, bool),
) -> [(f64, f64); 4] {
    let Some(dir) = dir else {
        if let Some(span) = looped {
            return loop_points(a, off, span);
        }
        return axis_edge(a, b, sa, sb, off);
    };
    if let Some(span) = looped {
        // On the high breadth side, as the static layout draws it (and
        // keeps the room for it): on screen right only in TD / BT — in
        // LR / RL, below the node, not in the gap its edges leave by.
        return loop_points(&flow_placed(dir, a), off, span).map(|p| from_flow(dir, p));
    }
    let (fa, fb) = (flow_placed(dir, a), flow_placed(dir, b));
    if let Some(pts) = nested_edge(&fa, &fb, sa, sb, off) {
        return pts.map(|p| from_flow(dir, p));
    }
    let fwd_gap = (fb.l - fb.lsize / 2.0) - (fa.l + fa.lsize / 2.0);
    let back_gap = (fa.l - fa.lsize / 2.0) - (fb.l + fb.lsize / 2.0);
    // Where each end aims, as the static layout aims it (`edge_points`).
    let (ea, eb) = ((&fa, sa, lone.0), (&fb, sb, lone.1));
    let (ta, tb) = (toward(ea, eb), toward(eb, ea));
    let pts = if fwd_gap >= FLOW_MIN_GAP {
        let p0 = anchor(&fa, sa, (ta, fb.l), off, true);
        let p3 = anchor(&fb, sb, (tb, fa.l), off, false);
        let dl = tangent(p3.1 - p0.1);
        [p0, (p0.0, p0.1 + dl), (p3.0, p3.1 - dl), p3]
    } else if back_gap >= FLOW_MIN_GAP {
        // Out of the source's flow-in side, into the target's flow-out
        // side, bowing past the wider box on the side the source lies.
        let p0 = anchor(&fa, sa, (ta, fb.l), off, false);
        let p3 = anchor(&fb, sb, (tb, fa.l), off, true);
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

/// How far (px) a self-loop's control points reach past its node's
/// side, for its parallel offset `off` ([`loop_points`]).
fn loop_ext(off: f64) -> f64 {
    LOOP_EXT + off.abs() * 0.8
}

/// Per node: how far (px) its self-loops reach past its high breadth
/// side (the curve's apex, three quarters of the controls' reach), 0
/// without one — the room the layout keeps beside it and what a
/// back-edge's lane clears (report P0 #13: a loop was nobody's
/// obstacle, so a channel ran through its apex).
pub(crate) fn loop_reach(g: &Graph) -> Vec<f64> {
    let mut reach = vec![0.0f64; g.nodes.len()];
    for (e, off) in g.edges.iter().zip(parallel_offsets(g)) {
        if e.from == e.to && !matches!(e.kind, EdgeKind::Invisible) {
            reach[e.from] = reach[e.from].max(0.75 * loop_ext(off));
        }
    }
    reach
}

/// Whether the self-loop of node `v` goes on its low breadth side in
/// [`route`]: when a node level with it along the flow (`at`, flow
/// coordinates) lies within its loop's reach (`reach`, [`loop_reach`])
/// plus [`crate::clearance::NODE_SEP`] of its high side and none that
/// close to its low side. The static layout keeps that room on the high
/// side (`layout` keeps a looped node's high-side neighbour off), so
/// over the scene's own centres the loop stays where `scene()` draws
/// it; only a drag that crowds it there sends it to the free side.
fn loop_on_low_side(at: &[Placed], v: usize, reach: &[f64]) -> bool {
    let p = &at[v];
    let room = reach.get(v).copied().unwrap_or(0.0) + crate::clearance::NODE_SEP;
    let level = |q: &Placed| {
        q.l - q.lsize / 2.0 < p.l + p.lsize / 2.0 && q.l + q.lsize / 2.0 > p.l - p.lsize / 2.0
    };
    let crowded = |high: bool| {
        at.iter().enumerate().any(|(w, q)| {
            if w == v || !level(q) {
                return false;
            }
            let gap = if high {
                (q.b - q.bsize / 2.0) - (p.b + p.bsize / 2.0)
            } else {
                (p.b - p.bsize / 2.0) - (q.b + q.bsize / 2.0)
            };
            (0.0..room).contains(&gap) || (gap < 0.0 && if high { q.b > p.b } else { q.b < p.b })
        })
    };
    crowded(true) && !crowded(false)
}

/// Small self-loop on the node's right side; parallel loops stack
/// vertically, each starting where the one before ends. `span` is how
/// far the node's stack of loops reaches either side of its centre
/// ([`loop_spans`]): a stack longer than the side (three loops on a
/// 38 px box reach 24 px either way) is squeezed evenly onto it, short
/// of the corners by [`LOOP_CORNER`], so every loop starts and ends on
/// its node's border and the loops still nest in order (B3 review r5
/// #1). A stack that fits keeps its 16 px loops exactly.
fn loop_points(a: &Placed, off: f64, span: f64) -> [(f64, f64); 4] {
    let r = a.b + a.bsize / 2.0;
    let ext = loop_ext(off);
    let room = (a.lsize / 2.0 - LOOP_CORNER).max(0.0);
    let k = if span > room && span > 0.0 {
        room / span
    } else {
        1.0
    };
    // The loop's place on the side and its ends' half spread, squeezed
    // alike; its controls keep their flare past the ends.
    let (c, h) = (off * k, LOOP_HALF * k);
    let flare = h + LOOP_FLARE;
    [
        (r, a.l - h + c),
        (r + ext, a.l - flare + c),
        (r + ext, a.l + flare + c),
        (r, a.l + h + c),
    ]
}

/// Per node: how far (px) the ends of its stack of self-loops reach
/// either side of its centre along its side, for parallel offsets
/// `offs` (index-parallel with `g.edges`): the farthest loop's offset
/// plus [`LOOP_HALF`]; 0 without a loop. [`loop_points`] squeezes a
/// stack longer than its side onto it.
fn loop_spans(g: &Graph, offs: &[f64]) -> Vec<f64> {
    let mut span = vec![0.0f64; g.nodes.len()];
    for (e, off) in g.edges.iter().zip(offs) {
        if e.from == e.to {
            span[e.from] = span[e.from].max(off.abs() + LOOP_HALF);
        }
    }
    span
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
        // Diamond: |x/hw| + |y/hh| = 1. One with a zero side is the
        // same line (or point) as a zero-sided box, which the
        // rectangle arm below meets without dividing 0 by 0.
        Shape::Diamond if hw > 0.0 && hh > 0.0 => 1.0 / (dx.abs() / hw + dy.abs() / hh),
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

/// A segment between two points, (from, to).
type Chord = ((f64, f64), (f64, f64));

/// The rough courses [`note_elbow`] keeps its elbow off, in flow
/// coordinates (`at`, index-parallel with `g.nodes`): for each of the
/// first `real` edges the segment between its ends' centres, then for
/// each sub-edge its drawn course — the segment between its ends cut
/// off where it meets a box end's frame (aimed at the centre of the
/// box's members, see [`frame_entry`]); for a member's edge to its own
/// box the segment to the entry member it is drawn to
/// ([`own_box_targets`]), and for any other edge between a member and its
/// own box the stub along the flow to the members' nearer flow side (see
/// [`own_box_stub`]).
/// Edges past `real` are `scene_clustered`'s stand-ins, never drawn as
/// such: their sub-edges' courses stand for them, so the static layout
/// and [`route`] judge an elbow against the same lines from the same
/// centres. Left out: self-loops, invisible links, and any link touching
/// a note card — two cards beside one state elbow out side by side, and
/// one's chord (straight across to its card) would strike the other's
/// elbow although the drawn links never cross.
fn edge_chords(g: &Graph, at: &[Placed], real: usize) -> Vec<Chord> {
    let card = |v: usize| {
        let n = &g.nodes[v];
        crate::style::is_note_card(&n.id, n.shape, &n.style)
    };
    let centre = |v: usize| (at[v].b, at[v].l);
    let mut out: Vec<Chord> = g.edges[..real]
        .iter()
        .filter(|e| {
            e.from != e.to && !matches!(e.kind, EdgeKind::Invisible) && !card(e.from) && !card(e.to)
        })
        .map(|e| (centre(e.from), centre(e.to)))
        .collect();
    if g.sub_edges.is_empty() {
        return out;
    }
    let node_cluster = node_cluster_paths(g);
    // Flow-coordinate bbox (b0, b1, l0, l1) of a box's members, nested
    // boxes' included.
    let members = |si: usize| {
        (0..g.nodes.len())
            .filter(|&v| node_cluster[v].contains(&si))
            .fold(None, |acc: Option<(f64, f64, f64, f64)>, v| {
                let p = &at[v];
                let r = (
                    p.b - p.bsize / 2.0,
                    p.b + p.bsize / 2.0,
                    p.l - p.lsize / 2.0,
                    p.l + p.lsize / 2.0,
                );
                Some(acc.map_or(r, |a| {
                    (a.0.min(r.0), a.1.max(r.1), a.2.min(r.2), a.3.max(r.3))
                }))
            })
    };
    // The entry members are those of `g`'s own edges: a stand-in past
    // `real` would count as an in-edge of the member it stands at.
    let own_box = |e: &crate::model::SubEdge| matches!((e.from, e.to), (End::Node(v), End::Sub(s)) if node_cluster[v].contains(&s));
    let targets = if !g.sub_edges.iter().any(own_box) {
        vec![None; g.sub_edges.len()]
    } else if real < g.edges.len() {
        let mut own = g.clone();
        own.edges.truncate(real);
        own_box_targets(&own)
    } else {
        own_box_targets(g)
    };
    for (e, target) in g.sub_edges.iter().zip(targets) {
        let ends = [e.from, e.to];
        if matches!(e.kind, EdgeKind::Invisible)
            || ends
                .iter()
                .any(|&end| matches!(end, End::Node(v) if card(v)))
        {
            continue;
        }
        let chord = match (e.from, e.to) {
            (End::Node(v), End::Sub(_)) if target.is_some() => {
                target.map(|m| (centre(v), centre(m)))
            }
            (End::Node(v), End::Sub(si)) | (End::Sub(si), End::Node(v))
                if node_cluster[v].contains(&si) =>
            {
                members(si).map(|(_, _, l0, l1)| {
                    let (b, l) = centre(v);
                    (centre(v), (b, if l - l0 < l1 - l { l0 } else { l1 }))
                })
            }
            _ => {
                let point = |end: End| match end {
                    End::Node(v) => Some(centre(v)),
                    End::Sub(si) => {
                        members(si).map(|(b0, b1, l0, l1)| ((b0 + b1) / 2.0, (l0 + l1) / 2.0))
                    }
                };
                // The drawn edge stops at a box end's frame: past it the
                // line to the members' centre is no course at all, and
                // would refuse elbows inside the box nothing crosses.
                // The frame here is the members' bbox grown by `SUB_PAD`
                // (as boxes re-wrap round dragged members), the same in
                // the static layout and in `route`.
                let clip = |from: (f64, f64), end: End, to: (f64, f64)| match end {
                    End::Sub(si) => members(si).map_or(to, |(b0, b1, l0, l1)| {
                        frame_entry(
                            from,
                            to,
                            (b0 - SUB_PAD, b1 + SUB_PAD, l0 - SUB_PAD, l1 + SUB_PAD),
                        )
                    }),
                    End::Node(_) => to,
                };
                point(e.from)
                    .zip(point(e.to))
                    .map(|(a, b)| (clip(b, e.from, a), clip(a, e.to, b)))
            }
        };
        out.extend(chord);
    }
    out
}

/// Where the segment `from` → `to` first enters the box `(b0, b1, l0,
/// l1)`: `to` itself when `from` already lies inside the box or the
/// segment misses it.
fn frame_entry(
    from: (f64, f64),
    to: (f64, f64),
    (b0, b1, l0, l1): (f64, f64, f64, f64),
) -> (f64, f64) {
    let d = (to.0 - from.0, to.1 - from.1);
    let (mut t_in, mut t_out) = (0.0_f64, 1.0_f64);
    for (p, dp, lo, hi) in [(from.0, d.0, b0, b1), (from.1, d.1, l0, l1)] {
        if dp.abs() < 1e-9 {
            if p < lo || p > hi {
                return to;
            }
            continue;
        }
        let (ta, tb) = ((lo - p) / dp, (hi - p) / dp);
        t_in = t_in.max(ta.min(tb));
        t_out = t_out.min(ta.max(tb));
    }
    if t_in <= 0.0 || t_in > t_out {
        return to;
    }
    (from.0 + d.0 * t_in, from.1 + d.1 * t_in)
}

/// A state `note` card (see [`crate::style::is_note_card`]).
fn is_note_card(n: &SceneNode) -> bool {
    crate::style::is_note_card(&n.id, n.shape, &n.style)
}

/// A state note card's link (`card`: which end is the card, see
/// [`crate::style::is_note_card`]) when the card lies wholly to one side
/// of its state across the flow and apart from it along the flow: out of
/// the state's breadth side facing the card, round into the card's flow
/// side facing the state. Aimed from the state's flow side, the link
/// would cross the channel the state's own transitions and their labels
/// use (in `LR`, a `note right of` card below the next state). `None`
/// for any other edge, for a state with no flat side (a `<<choice>>`'s
/// cards would share its corner; a fork/join bar's card sits past its
/// branches), when another box (`others`, the two ends left out) stands
/// within [`crate::clearance::NODE_SEP`] of the elbow, or when another
/// edge (`chords`, see [`edge_chords`]) crosses it: the elbow runs
/// round the outside of the state's own fan, so a transition out
/// between the card and the state (in `LR`, to a state below the card)
/// would be crossed. Flow coordinates.
fn note_elbow<'a>(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    card: (bool, bool),
    others: impl IntoIterator<Item = &'a Placed>,
    chords: impl IntoIterator<Item = Chord>,
) -> Option<[(f64, f64); 4]> {
    let (s, c, shape, flip) = match card {
        (false, true) => (a, b, sa, false),
        (true, false) => (b, a, sb, true),
        _ => return None,
    };
    let apart = |d: f64, x: f64, y: f64| d.abs() > (x + y) / 2.0;
    let flat = !matches!(
        shape,
        Shape::Diamond
            | Shape::Circle
            | Shape::DoubleCircle
            | Shape::StateStart
            | Shape::StateEnd
            | Shape::ForkBar
    );
    if !flat || !apart(c.b - s.b, c.bsize, s.bsize) || !apart(c.l - s.l, c.lsize, s.lsize) {
        return None;
    }
    let p0 = side_point(s, shape, (c.b - s.b).signum(), s.l);
    let p3 = (c.b, c.l - (c.l - s.l).signum() * c.lsize / 2.0);
    let corner = (c.b, s.l);
    // The elbow's two legs: across the flow at the state's layer, then
    // along it at the card's breadth.
    let span = |u: f64, v: f64| (u.min(v), u.max(v));
    let legs = [
        (span(p0.0, corner.0), span(s.l, s.l)),
        (span(c.b, c.b), span(s.l, p3.1)),
    ];
    let pad = crate::clearance::NODE_SEP;
    let blocked = others.into_iter().any(|o| {
        let (b0, b1) = (o.b - o.bsize / 2.0 - pad, o.b + o.bsize / 2.0 + pad);
        let (l0, l1) = (o.l - o.lsize / 2.0 - pad, o.l + o.lsize / 2.0 + pad);
        legs.iter()
            .any(|&((x0, x1), (y0, y1))| x0 < b1 && b0 < x1 + 1e-9 && y0 < l1 && l0 < y1 + 1e-9)
    });
    // Whether segments p–q and u–v properly cross.
    let cross = |p: (f64, f64), q: (f64, f64), u: (f64, f64), v: (f64, f64)| {
        let side = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
        };
        side(p, q, u) * side(p, q, v) < 0.0 && side(u, v, p) * side(u, v, q) < 0.0
    };
    let elbow = [((p0.0, s.l), corner), (corner, (c.b, p3.1))];
    if blocked
        || chords
            .into_iter()
            .any(|(u, v)| elbow.iter().any(|&(p, q)| cross(p, q, u, v)))
    {
        return None;
    }
    // Control points KAPPA of the way to the corner: a quarter-ellipse.
    const KAPPA: f64 = 0.552;
    let toward = |p: (f64, f64)| {
        (
            p.0 + KAPPA * (corner.0 - p.0),
            p.1 + KAPPA * (corner.1 - p.1),
        )
    };
    let run = [p0, toward(p0), toward(p3), p3];
    Some(if flip {
        [run[3], run[2], run[1], run[0]]
    } else {
        run
    })
}

/// Midpoint of a cubic bezier (t = 0.5), used to place labels.
fn cubic_mid(p0: (f64, f64), c1: (f64, f64), c2: (f64, f64), p3: (f64, f64)) -> (f64, f64) {
    (
        (p0.0 + 3.0 * c1.0 + 3.0 * c2.0 + p3.0) / 8.0,
        (p0.1 + 3.0 * c1.1 + 3.0 * c2.1 + p3.1) / 8.0,
    )
}

/// Viewport, background and colour-theme options for
/// [`crate::render_svg_advanced`] and every `*_with` writer.
///
/// Defaults reproduce the historical fixed-pixel output byte-for-byte, so
/// existing callers (and `render_svg`) are unaffected. More options
/// (scale, max width, …) can be added here without new entry
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
    /// When true, the full-canvas background `<rect>` is omitted so
    /// the diagram sits on whatever is behind it (e.g. a dark page).
    /// Shapes, label boxes and text keep their own fills, in the
    /// theme's colours: on a dark page pick [`Theme::Dark`] as well,
    /// whose text and lines are light.
    pub transparent: bool,
    /// The colour theme (report M10). `None`, the default: the theme
    /// the diagram's source asks for — `%%{init: {"theme": …,
    /// "themeVariables": {…}}}%%` or `theme:` / `themeVariables:` under
    /// a YAML frontmatter's `config:` — when the source has been read
    /// ([`crate::render_svg_advanced`] does, or see
    /// [`SvgOptions::with_source_theme`]), else [`Theme::Flowmaid`].
    /// `Some(theme)` always paints in `theme`, whatever the source
    /// says: an explicit option beats the source's directive, its
    /// theme name and its `themeVariables` alike (`Some(Flowmaid)`
    /// renders a themed source in flowmaid's own colours).
    ///
    /// ```
    /// use flowmaid::style::Theme;
    /// let src = "%%{init: {\"theme\": \"neutral\"}}%%\nflowchart TD\nA --> B";
    /// // The source's theme: white nodes with a #333333 border.
    /// let svg = flowmaid::render_svg_advanced(src, &Default::default()).unwrap();
    /// assert!(svg.contains("fill=\"#ffffff\" stroke=\"#333333\""));
    /// // An explicit theme wins.
    /// let mut opts = flowmaid::SvgOptions::default();
    /// opts.theme = Some(Theme::Dark);
    /// let dark = flowmaid::render_svg_advanced(src, &opts).unwrap();
    /// assert!(dark.contains("fill=\"#1e1f22\"/>")); // the dark canvas
    /// ```
    pub theme: Option<Theme>,
    /// The theme the source asked for, read by
    /// [`SvgOptions::with_source_theme`]; used while `theme` is `None`.
    pub(crate) source_theme: Option<Theme>,
    /// The source's `themeVariables`, likewise.
    pub(crate) theme_vars: ThemeVars,
}

impl SvgOptions {
    /// These options with the colour theme `source` asks for: its
    /// `%%{init}%%` directives' `theme` and `themeVariables`
    /// (`background`, `primaryColor`, `primaryBorderColor`,
    /// `primaryTextColor`, `lineColor`, `textColor`) and those under a
    /// YAML frontmatter's `config:`, the directives winning. A theme
    /// name [`Theme::from_name`] does not know paints the default
    /// theme, as in mermaid; colours that do not parse are ignored.
    /// Has no effect while [`SvgOptions::theme`] is set — an explicit
    /// theme beats the source. [`crate::render_svg_advanced`] calls
    /// this itself; callers that parse a document and render it with a
    /// `render::*_with` writer (as the CLI does) call it to honour the
    /// source's theme too.
    pub fn with_source_theme(mut self, source: &str) -> SvgOptions {
        if self.theme.is_none() {
            let (theme, vars) = crate::parser::theme_directive(source);
            self.source_theme = theme;
            self.theme_vars = vars;
        }
        self
    }

    /// The colours these options paint with: [`SvgOptions::theme`],
    /// else the source's theme and `themeVariables` (see
    /// [`SvgOptions::with_source_theme`]), else [`Theme::Flowmaid`].
    pub fn palette(&self) -> Palette {
        match self.theme {
            Some(theme) => Palette::new(theme),
            None => Palette::with_vars(self.source_theme.unwrap_or_default(), &self.theme_vars),
        }
    }
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
    // The canvas is opt-out (issue #62) so output can sit on a page of
    // its own colour; the default keeps it for byte-identical output.
    // Its colour is the theme's (white unless a theme says otherwise).
    if !opts.transparent {
        s.push_str(&format!(
            "<rect width=\"{:.0}\" height=\"{:.0}\" fill=\"{}\"/>\n",
            width,
            height,
            opts.palette().background
        ));
    }
}

/// Label chip with centred text — used for flowchart edge labels and
/// ER / class / architecture relationship labels alike: the theme's
/// chip ([`Palette::label_fill`], white in the default theme) and text.
/// `center` is in final (already translated) coordinates.
pub(crate) fn svg_label_box(
    s: &mut String,
    text: &str,
    center: (f64, f64),
    box_w: f64,
    pal: &Palette,
) {
    svg_label_box_with(s, text, center, box_w, &pal.text, pal);
}

/// [`svg_label_box`] with the text colour given (an already-escaped
/// attribute value), for `linkStyle … color:`.
pub(crate) fn svg_label_box_with(
    s: &mut String,
    text: &str,
    center: (f64, f64),
    box_w: f64,
    fill: &str,
    pal: &Palette,
) {
    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"20\" rx=\"4\" \
         fill=\"{}\" stroke=\"{}\"/>\n",
        center.0 - box_w / 2.0,
        center.1 - 10.0,
        box_w,
        pal.label_fill,
        pal.label_border
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
    svg_text_lines(s, cx, cy, fill, label, "", false);
}

/// [`svg_text_multiline`] with extra attributes written into the
/// `<text>` tag (`attrs`: already escaped, each with a leading space,
/// e.g. ` font-weight="bold"`). `literal` escapes each line as plain
/// text instead of interpreting `<b>`/`<i>` runs — for diagram types
/// whose mermaid renderer shows such tags verbatim.
pub(crate) fn svg_text_lines(
    s: &mut String,
    cx: f64,
    cy: f64,
    fill: &str,
    label: &str,
    attrs: &str,
    literal: bool,
) {
    let markup = |l: &str| if literal { escape(l) } else { rich(l) };
    let lines: Vec<&str> = label.split('\n').collect();
    if lines.len() <= 1 {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\"{} \
             fill=\"{}\">{}</text>\n",
            cx,
            cy,
            attrs,
            fill,
            markup(label)
        ));
        return;
    }
    let lh = crate::layout::LINE_H;
    let top = cy - (lines.len() as f64 - 1.0) * lh / 2.0;
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\"{} fill=\"{}\">",
        cx, top, attrs, fill
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
            markup(line)
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
    use crate::layout::{intrinsic_size, LINE_H};
    use crate::parser::parse;

    fn svg_src(src: &str) -> String {
        to_svg(&scene(&parse(src).unwrap_or_else(|e| panic!("{e}\n{src}"))))
    }

    fn edge_paths(svg: &str) -> Vec<&str> {
        svg.lines()
            .filter(|l| l.starts_with("<path d=") && l.contains("<title>"))
            .collect()
    }

    /// Every edge whose ink `svg` paints off its own canvas (`0 0 width
    /// height`), as "<title> by <px>": each edge path's curve to its
    /// exact extremes (every `M` / `L` point, each `C` segment's
    /// [`cubic_bounds`] — its control points are no ink, report Q2), half
    /// the stroke width out, and each end marker's whole box — its
    /// `viewBox` placed by `refX` / `refY` and `markerWidth`, turned along
    /// the path's end as `orient="auto"` turns it. Read off the SVG's own
    /// numbers.
    fn ink_off_canvas(svg: &str) -> Vec<String> {
        let attr = |el: &str, name: &str| -> Option<String> {
            let rest = el.split(&format!(" {name}=\"")).nth(1)?;
            Some(rest.split('"').next()?.to_string())
        };
        let nums = |v: &str| -> Vec<f64> {
            v.split([' ', ','])
                .filter_map(|t| t.parse::<f64>().ok())
                .collect()
        };
        let vb = nums(&attr(svg.lines().next().unwrap(), "viewBox").expect("viewBox"));
        let (w, h) = (vb[2], vb[3]);
        // Marker id -> (view box, refX, refY, px).
        let markers: HashMap<String, (Vec<f64>, f64, f64, f64)> = svg
            .split("<marker ")
            .skip(1)
            .map(|m| {
                let num = |k: &str| attr(m, k).unwrap().parse::<f64>().unwrap();
                (
                    attr(&format!(" {m}"), "id").unwrap(),
                    (
                        nums(&attr(m, "viewBox").unwrap()),
                        num("refX"),
                        num("refY"),
                        num("markerWidth"),
                    ),
                )
            })
            .collect();
        let mut out = Vec::new();
        for p in edge_paths(svg) {
            let d = attr(p, "d").unwrap();
            let half = attr(p, "stroke-width").map_or(1.0, |v| v.parse().unwrap()) / 2.0;
            let c = nums(&d);
            let pts: Vec<(f64, f64)> = c.chunks(2).map(|q| (q[0], q[1])).collect();
            // The painted curve's extremes: `M` / `L` points as they are,
            // a `C` segment by its exact bounds.
            let mut ink: Vec<(f64, f64)> = Vec::new();
            let mut at = (0.0, 0.0);
            let (mut op, mut args) = ('M', Vec::new());
            for tok in d.split([' ', ',']).filter(|t| !t.is_empty()) {
                match tok.parse::<f64>() {
                    Ok(v) => args.push(v),
                    Err(_) => {
                        op = tok.chars().next().unwrap();
                        args.clear();
                    }
                }
                let arity = if op == 'C' { 6 } else { 2 };
                if args.len() == arity {
                    let q: Vec<(f64, f64)> = args.chunks(2).map(|q| (q[0], q[1])).collect();
                    if op == 'C' {
                        let (x0, x1, y0, y1) = cubic_bounds(&[at, q[0], q[1], q[2]]);
                        ink.extend([(x0, y0), (x1, y1)]);
                    } else {
                        ink.push(q[0]);
                    }
                    at = q[q.len() - 1];
                    args.clear();
                }
            }
            let mut worst = f64::NEG_INFINITY;
            let mut off = |q: (f64, f64), pad: f64| {
                worst = worst
                    .max(pad - q.0)
                    .max(pad - q.1)
                    .max(q.0 + pad - w)
                    .max(q.1 + pad - h);
            };
            for &q in &ink {
                off(q, half);
            }
            for (key, at_start) in [("marker-start", true), ("marker-end", false)] {
                let Some(url) = attr(p, key) else { continue };
                let id = url.trim_start_matches("url(#").trim_end_matches(')');
                let (mvb, rx, ry, px) = &markers[id];
                // The path's end and its heading there (forward at the
                // start, into the end at the end), off the nearest
                // coordinate apart from it.
                let seq: Vec<(f64, f64)> = if at_start {
                    pts.clone()
                } else {
                    pts.iter().rev().copied().collect()
                };
                let at = seq[0];
                let q = *seq[1..]
                    .iter()
                    .find(|q| (q.0 - at.0).hypot(q.1 - at.1) > 1e-9)
                    .expect("a heading");
                let l = (q.0 - at.0).hypot(q.1 - at.1);
                let mut dir = ((q.0 - at.0) / l, (q.1 - at.1) / l);
                if !at_start {
                    dir = (-dir.0, -dir.1);
                }
                let n = (-dir.1, dir.0);
                let k = px / mvb[2];
                for (u, v) in [
                    (mvb[0], mvb[1]),
                    (mvb[0] + mvb[2], mvb[1]),
                    (mvb[0], mvb[1] + mvb[3]),
                    (mvb[0] + mvb[2], mvb[1] + mvb[3]),
                ] {
                    let (a, b) = ((u - rx) * k, (v - ry) * k);
                    off(
                        (at.0 + dir.0 * a + n.0 * b, at.1 + dir.1 * a + n.1 * b),
                        0.0,
                    );
                }
            }
            if worst > 0.0 {
                let title = p.split("<title>").nth(1).unwrap_or("?");
                let title = title.split('<').next().unwrap_or("?");
                out.push(format!("{title} by {worst:.1}"));
            }
        }
        out
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
            p[1].contains("stroke-dasharray=\"2 3\""),
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
            paint_width: None,
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
        let spline_d = |pts: &[(f64, f64)]| {
            let (start, segs) = spline_segs(pts);
            curve_d(start, &segs, |p| p)
        };
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
        grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, _, miny, _) = bb.finish();
        let (tx, ty) = (MARGIN - minx, MARGIN - miny);
        let paths = edge_paths(&svg);
        assert_eq!(paths.len(), sc.edges.len());
        for (e, p) in sc.edges.iter().zip(paths) {
            let d = edge_path_d_painted(e, |(x, y)| (x + tx, y + ty));
            assert!(p.starts_with(&format!("<path d=\"{d}\"")), "{p}\n{d}");
            // The same curve as edge_path_d, cut short under the head:
            // the same start, an end ARROW_TRIM back along the curve.
            let whole = edge_path_d(e, |(x, y)| (x + tx, y + ty));
            let (a, b) = (path_d_points(&d, 1.0), path_d_points(&whole, 1.0));
            assert_eq!(a[0], b[0], "{d}\n{whole}");
            let (pa, pb) = (a[a.len() - 1], b[b.len() - 1]);
            assert!(
                ((pa.0 - pb.0).hypot(pa.1 - pb.1) - ARROW_TRIM).abs() < 0.2,
                "{d}\n{whole}"
            );
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
    fn edge_end_tangent_is_the_drawn_heading_into_each_end() {
        let near =
            |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
        // Routed L: the spline leaves straight down and closes along the
        // last leg, rightwards; at the start the heading points back.
        let e = routed_edge(
            (0.0, 0.0),
            (80.0, 50.0),
            vec![(0.0, 0.0), (0.0, 50.0), (80.0, 50.0)],
        );
        let (tip, head) = edge_end_tangent(&e, false);
        assert!(
            near(tip, (80.0, 50.0)) && near(head, (1.0, 0.0)),
            "{head:?}"
        );
        let (tip, head) = edge_end_tangent(&e, true);
        assert!(near(tip, (0.0, 0.0)) && near(head, (0.0, -1.0)), "{head:?}");
        // One cubic: its end tangents; a control point on the tip defers
        // to the next one (the degenerate chord runs along itself).
        let mut c = routed_edge((0.0, 0.0), (30.0, 40.0), Vec::new());
        c.bezier = [(0.0, 0.0), (0.0, 20.0), (30.0, 10.0), (30.0, 40.0)];
        assert!(near(edge_end_tangent(&c, false).1, (0.0, 1.0)));
        assert!(near(edge_end_tangent(&c, true).1, (0.0, -1.0)));
        let chord = routed_edge((0.0, 0.0), (30.0, 40.0), Vec::new());
        assert!(near(edge_end_tangent(&chord, false).1, (0.6, 0.8)));
        // A mermaid curve: its own last command (rounded corners end on
        // a straight run).
        let mut r = e.clone();
        r.meta.curve = Some(EdgeCurve::Rounded);
        assert!(near(edge_end_tangent(&r, false).1, (1.0, 0.0)));
        assert!(near(edge_end_tangent(&r, true).1, (0.0, -1.0)));
        let dot = routed_edge((5.0, 5.0), (5.0, 5.0), Vec::new());
        assert_eq!(edge_end_tangent(&dot, false).1, (0.0, 0.0));
    }

    #[test]
    fn node_outline_traces_each_painted_shape() {
        let sc = scene(
            &parse(
                "flowchart TD\nA[rect]-->B(rounded)-->C([stadium])-->D((circle))\n\
                 D-->E{diamond}-->F[(cylinder)]-->G[[subroutine]]-->H{{hexagon}}\n\
                 H-->I[/lean right/]-->J[\\lean left\\]-->K(((double)))",
            )
            .unwrap(),
        );
        let area = |p: &[(f64, f64)]| {
            let n = p.len();
            let twice: f64 = (0..n)
                .map(|i| p[i].0 * p[(i + 1) % n].1 - p[(i + 1) % n].0 * p[i].1)
                .sum();
            twice.abs() / 2.0
        };
        let pi = std::f64::consts::PI;
        for n in &sc.nodes {
            let o = node_outline(n);
            // Inside the box, reaching all four of its sides (a round
            // side up to the flattening).
            let (l, r, t, b) = (
                n.x - n.w / 2.0,
                n.x + n.w / 2.0,
                n.y - n.h / 2.0,
                n.y + n.h / 2.0,
            );
            let eps = 1e-9;
            assert!(
                o.iter().all(|p| p.0 >= l - eps && p.0 <= r + eps),
                "{}",
                n.id
            );
            assert!(
                o.iter().all(|p| p.1 >= t - eps && p.1 <= b + eps),
                "{}",
                n.id
            );
            for (k, side) in [l, r, t, b].into_iter().enumerate() {
                let on = |p: &(f64, f64)| {
                    (if k < 2 { p.0 } else { p.1 } - side).abs() <= OUTLINE_FLATNESS
                };
                assert!(o.iter().any(on), "{} misses side {k}", n.id);
            }
            let k = SLANT_MAX.min(n.w / 4.0);
            let want = match n.shape {
                Shape::Rect | Shape::Subroutine => n.w * n.h - (4.0 - pi) * BOX_RX * BOX_RX,
                Shape::Rounded => n.w * n.h - (4.0 - pi) * ROUNDED_RX * ROUNDED_RX,
                Shape::Stadium => n.w * n.h - (4.0 - pi) * (n.h / 2.0).powi(2),
                Shape::Circle | Shape::DoubleCircle => pi * (n.w / 2.0).powi(2),
                Shape::Diamond => n.w * n.h / 2.0,
                Shape::Hexagon | Shape::Parallelogram | Shape::ParallelogramAlt => (n.w - k) * n.h,
                Shape::Cylinder => {
                    let ry = cylinder_ry(n.w, n.h);
                    n.w * (n.h - 2.0 * ry) + pi * (n.w / 2.0) * ry
                }
                _ => unreachable!("not in this chart"),
            };
            // Flattened arcs lose at most a flattening's depth all
            // along the perimeter (at most the box's).
            let got = area(&o);
            let slack = OUTLINE_FLATNESS * 2.0 * (n.w + n.h);
            assert!((got - want).abs() <= slack, "{}: {got} vs {want}", n.id);
        }
        // A node thousands of px across stays a bounded polygon.
        let mut big = sc.nodes[3].clone();
        (big.w, big.h) = (1e6, 1e6);
        assert!(node_outline(&big).len() <= OUTLINE_MAX_STEPS as usize + 1);
    }

    #[test]
    fn picking_takes_the_painted_outline_of_every_shape() {
        // Report M6: what `node_at` picks is the outline the painter draws
        // and the edge ends sit on — a box corner a hexagon, a
        // parallelogram, a drum, a pill or a rounded box leaves empty is
        // not the node; plain boxes stay their box.
        let sc = scene(
            &parse(
                "flowchart TD\nA[rect]-->B(rounded)-->C([stadium])-->F[(cylinder)]\n\
                 F-->H{{hexagon}}-->I[/lean right/]-->J[\\lean left\\]-->G[[sub]]",
            )
            .unwrap(),
        );
        for n in &sc.nodes {
            assert!(node_contains(n, n.x, n.y), "{} centre", n.id);
            let o = node_outline(n);
            // Each painted vertex is inside up to rounding; a point a px
            // beyond it, away from the centre, is outside.
            for &(px, py) in &o {
                let (dx, dy) = (px - n.x, py - n.y);
                let l = dx.hypot(dy);
                let at = |s: f64| (n.x + dx * (1.0 + s / l), n.y + dy * (1.0 + s / l));
                let inner = at(-0.01);
                assert!(node_contains(n, inner.0, inner.1), "{} at {px},{py}", n.id);
                let outer = at(1.0);
                let boxy = matches!(n.shape, Shape::Rect | Shape::Subroutine);
                assert!(
                    boxy || !node_contains(n, outer.0, outer.1),
                    "{} past {px},{py}",
                    n.id
                );
            }
            // Box corners: all four a plain box's, the two a
            // parallelogram's slants keep, none of the others'.
            let (hw, hh) = (n.w / 2.0 - 0.5, n.h / 2.0 - 0.5);
            let inside = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
                .iter()
                .filter(|c| node_contains(n, n.x + c.0, n.y + c.1))
                .count();
            let want = match n.shape {
                Shape::Rect | Shape::Subroutine => 4,
                Shape::Parallelogram | Shape::ParallelogramAlt => 2,
                _ => 0,
            };
            assert_eq!(inside, want, "{} box corners", n.id);
        }
    }

    #[test]
    fn outline_along_puts_an_end_on_the_painted_outline() {
        let sc = scene(&parse("flowchart LR\nA[/lean/] --> B[(db)] --> C{{hex}}").unwrap());
        let node = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().clone();
        let a = node("A");
        let (l, r, t) = (a.x - a.w / 2.0, a.x + a.w / 2.0, a.y - a.h / 2.0);
        let k = SLANT_MAX.min(a.w / 4.0);
        // `/ /`'s right flank runs from (r, t) to (r - k, b): an end on
        // the box side at mid height, run leaving to the right, goes in
        // k/2 along its run.
        let q = outline_along(&a, (r, a.y), (1.0, 0.0)).expect("moves in");
        assert!(
            (q.0 - (r - k / 2.0)).abs() < 1e-6 && (q.1 - a.y).abs() < 1e-9,
            "{q:?}"
        );
        // On the outline already, it stays.
        assert_eq!(outline_along(&a, q, (1.0, 0.0)), None);
        assert_eq!(outline_along(&a, (l + k, t), (0.0, -1.0)), None);
        // An end sunk half a px comes back out; a run heading back across
        // the shape (leaving a point deep inside) never carries the end
        // through the node to its far side.
        let sunk = outline_along(&a, (q.0 - 0.5, q.1), (1.0, 0.0)).expect("comes out");
        assert!((sunk.0 - q.0).abs() < 1e-6, "{sunk:?}");
        assert_eq!(outline_along(&a, (a.x, a.y), (1.0, 0.0)), None);
        // A run that misses the shape leaves the end where it is.
        assert_eq!(outline_along(&a, (r + 0.2, t - 0.2), (0.0, -1.0)), None);
        // The drum's lid: an end on the box top, a quarter of the way in,
        // goes down onto the back arc.
        let b = node("B");
        let (rx, ry) = (b.w / 2.0, cylinder_ry(b.w, b.h));
        let x = b.x + rx / 2.0;
        let q = outline_along(&b, (x, b.y - b.h / 2.0), (0.0, -1.0)).expect("onto the lid");
        let want = b.y - b.h / 2.0 + ry * (1.0 - (1.0f64 - 0.25).sqrt());
        assert!(
            (q.0 - x).abs() < 1e-9 && (q.1 - want).abs() < 1e-6,
            "{q:?} vs {want}"
        );
        // A plain box is its outline.
        let mut boxed = a.clone();
        boxed.shape = Shape::Rect;
        assert_eq!(outline_along(&boxed, (r, a.y), (1.0, 0.0)), None);
        // The shoulder's nearest point lies on the cap ellipse, nearer
        // than the box corner beside it.
        let p = (b.x + rx, b.y - b.h / 2.0 + ry / 3.0);
        let s = cylinder_shoulder_point(&b, p).expect("on the shoulder");
        let cap = (b.y - b.h / 2.0) + ry;
        let on = ((s.0 - b.x) / rx).powi(2) + ((s.1 - cap) / ry).powi(2);
        assert!((on - 1.0).abs() < 1e-6, "{s:?} off the ellipse ({on})");
        assert!((s.0 - p.0).hypot(s.1 - p.1) < ry, "{s:?}");
        assert_eq!(cylinder_shoulder_point(&b, (b.x + rx, b.y)), None);
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
    fn cylinders_fill_their_lid_centre_their_label_and_take_their_stroke() {
        // Report Q7. A narrow and a wide drum, one with a style width.
        let src = "flowchart TD\n  A[(DB)] --> B[(A rather wide database name)]\n  \
                   C[[Sub]]\n  style B stroke-width:4px\n  style C stroke-width:3px\n";
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let svg = to_svg(&sc);
        let node = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
        let (a, b) = (node("A"), node("B"));
        // The lid deepens with the width (mermaid's rx / (2.5 + w / 50),
        // capped), within the drum's unchanged size.
        let (ra, rb) = (cylinder_ry(a.w, a.h), cylinder_ry(b.w, b.h));
        assert!(ra < rb && ra >= 7.0 && rb <= 10.0, "{ra} {rb}");
        assert_eq!((a.h, b.h), (54.0, 54.0));
        for (n, ry, sw) in [(a, ra, NODE_STROKE_WIDTH), (b, rb, 4.0)] {
            let (l, r) = (n.x - n.w / 2.0, n.x + n.w / 2.0);
            let ty = n.y - n.h / 2.0 + ry;
            // The drum runs over the lid's back arc (sweep 1), so the lid
            // is filled; the front arc is stroked on top in the node's
            // own width (a style width reaches it).
            let body = format!(
                "<path d=\"M {l:.1} {ty:.1} A {:.1} {ry:.1} 0 0 1 {r:.1} {ty:.1} ",
                n.w / 2.0
            );
            assert!(svg.contains(&body), "{body}\n{svg}");
            let lid = format!(
                "<path d=\"M {l:.1} {ty:.1} A {:.1} {ry:.1} 0 0 0 {r:.1} {ty:.1}\" fill=\"none\" \
                 stroke=\"{}\" stroke-width=\"{sw}\"/>",
                n.w / 2.0,
                crate::style::shape_style(Shape::Cylinder).stroke
            );
            assert!(svg.contains(&lid), "{lid}\n{svg}");
            // The label sits in the body under the lid.
            let ly = n.y + CYLINDER_LABEL_DROP * ry;
            assert!(svg.contains(&format!("y=\"{ly:.1}\"")), "{ly}\n{svg}");
        }
        // A subroutine's bars take its style width too.
        let bars = svg
            .lines()
            .filter(|l| l.starts_with("<line ") && l.ends_with("stroke-width=\"3\"/>"))
            .count();
        assert_eq!(bars, 2, "{svg}");
        // The advance writer draws the same drum.
        let adv = crate::advance::render_advance_text_svg("lane l \"L\"\na[(DB)]\n").unwrap();
        assert!(adv.contains(" 0 0 1 ") && adv.contains(" 0 0 0 "), "{adv}");
    }

    #[test]
    fn ends_on_a_cylinders_cap_sit_on_its_arc() {
        // Report Q7 / §2.5: an end off the middle of a drum's top floated
        // above the lid's back arc, one under its bottom below the
        // bottom arc. Fan in and out of a wide drum, both modes.
        let src = "flowchart TD\n  A --> D[(A rather wide database)]\n  B --> D\n  C --> D\n  \
                   D --> E\n  D --> F\n  D --> G\n";
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let routed = route(&g, &at);
        for sc in [&sc, &routed] {
            let d = sc.nodes.iter().find(|n| n.id == "D").unwrap();
            let outline = node_outline(d);
            let dist = |p: (f64, f64)| {
                outline
                    .windows(2)
                    .map(|w| point_seg_dist(p.0, p.1, w[0], w[1]))
                    .fold(f64::INFINITY, f64::min)
            };
            let mut off_centre = 0;
            for e in &sc.edges {
                let ends = edge_polyline(e);
                let p = if e.to == "D" {
                    ends[ends.len() - 1]
                } else if e.from == "D" {
                    ends[0]
                } else {
                    continue;
                };
                assert!(
                    dist(p) < 0.5,
                    "{} -> {} ends {:.1} px off D's outline",
                    e.from,
                    e.to,
                    dist(p)
                );
                off_centre += usize::from((p.0 - d.x).abs() > 10.0);
            }
            assert!(off_centre >= 4, "the fans spread over the caps");
        }
    }

    #[test]
    fn nested_frames_step_by_depth_and_title_chips_take_their_fill() {
        // Report Q6: every frame looked the same at any depth, its border
        // 1.55:1 on white. Each depth now takes its own fill and a border
        // readable on the frame round it; a title's chip takes its
        // frame's fill, and titles still paint last (never struck).
        let svg = svg_src(
            "flowchart TD\n  subgraph A [Outer]\n    subgraph B [Middle]\n      subgraph C [Inner]\n        \
             x --> y\n      end\n    end\n  end\n  q --> x\n",
        );
        for d in 0..3 {
            let c = crate::style::cluster_style(d);
            let frame = format!(
                "rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.4\"/>",
                c.fill, c.stroke
            );
            assert_eq!(svg.matches(&frame).count(), 1, "depth {d}\n{svg}");
            let chip = format!("rx=\"4\" fill=\"{}\" fill-opacity=\"0.9\"/>", c.fill);
            assert_eq!(svg.matches(&chip).count(), 1, "depth {d} chip\n{svg}");
        }
        let title = svg.find(">Inner</text>").unwrap();
        assert!(
            svg[title..].find("<path").is_none(),
            "titles paint after the edges"
        );
        assert!(svg.contains(&format!(
            "fill=\"{}\">Outer</text>",
            crate::style::CLUSTER_TITLE
        )));
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

    /// A point `off` scene units to the left of edge `e`'s drawn curve,
    /// at its middle sample.
    fn beside_mid(e: &SceneEdge, off: f64) -> (f64, f64) {
        let pts = edge_polyline(e);
        let k = pts.len() / 2;
        let (a, b) = (pts[k - 1], pts[k]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let (nx, ny) = (-(b.1 - a.1) / len, (b.0 - a.0) / len);
        let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        (m.0 + nx * off, m.1 + ny * off)
    }

    #[test]
    fn edge_at_covers_the_painted_stroke_not_just_its_centre_line() {
        // A thick edge is 3.4 wide and a `linkStyle` one 10: a point on
        // the painted stroke but off its centre line picks the edge
        // even at zero tolerance, and a point just past the stroke does
        // not.
        let g = parse("flowchart LR\nA ==> B\nC --> D\nlinkStyle 1 stroke-width:10px").unwrap();
        let s = scene(&g);
        let thick = beside_mid(&s.edges[0], EDGE_WIDTH_THICK / 2.0 - 0.1);
        assert_eq!(s.edge_at(thick.0, thick.1, 0.0), Some(0));
        let past = beside_mid(&s.edges[0], EDGE_WIDTH_THICK / 2.0 + 0.1);
        assert_eq!(s.edge_at(past.0, past.1, 0.0), None);
        let wide = beside_mid(&s.edges[1], 4.9);
        assert_eq!(s.edge_at(wide.0, wide.1, 0.0), Some(1));
        assert_eq!(s.hit_test(wide.0, wide.1, 0.0), Some(Hit::Edge(1)));
    }

    #[test]
    fn a_styled_invisible_link_is_picked_because_it_is_painted() {
        // `~~~` with a `linkStyle` stroke width is drawn (edge_drawn), so
        // it must be pickable and marquee-selectable like any edge; the
        // unstyled `~~~` beside it stays unpickable.
        let g = parse("flowchart LR\nA ~~~ B\nC ~~~ D\nlinkStyle 0 stroke-width:2px").unwrap();
        let s = scene(&g);
        assert!(edge_drawn(&s.edges[0]) && !edge_drawn(&s.edges[1]));
        let p = beside_mid(&s.edges[0], 0.0);
        assert_eq!(s.edge_at(p.0, p.1, 0.0), Some(0));
        let q = beside_mid(&s.edges[1], 0.0);
        assert_eq!(s.edge_at(q.0, q.1, 4.0), None);
        let all = s.hits_in_rect(-1e6, -1e6, 1e6, 1e6);
        assert!(all.contains(&Hit::Edge(0)) && !all.contains(&Hit::Edge(1)));
    }

    /// Points along an edge path `d` as the edge painters emit it
    /// (`M`/`L`/`Q`/`C`, absolute), no further apart than `step` on
    /// straight runs and finely sampled on curves.
    fn path_d_points(d: &str, step: f64) -> Vec<(f64, f64)> {
        let toks: Vec<&str> = d.split([' ', ',']).filter(|t| !t.is_empty()).collect();
        let num = |i: usize| toks[i].parse::<f64>().unwrap();
        let (mut out, mut cur, mut i) = (Vec::new(), (0.0, 0.0), 0);
        while i < toks.len() {
            let pts: Vec<(f64, f64)> = match toks[i] {
                "M" | "L" => vec![(num(i + 1), num(i + 2))],
                "Q" => vec![(num(i + 1), num(i + 2)), (num(i + 3), num(i + 4))],
                "C" => vec![
                    (num(i + 1), num(i + 2)),
                    (num(i + 3), num(i + 4)),
                    (num(i + 5), num(i + 6)),
                ],
                other => panic!("unexpected path command {other:?} in {d}"),
            };
            let cmd = toks[i];
            i += 1 + 2 * pts.len();
            let end = pts[pts.len() - 1];
            match cmd {
                "M" => out.push(end),
                "L" => {
                    let n = ((end.0 - cur.0).hypot(end.1 - cur.1) / step)
                        .ceil()
                        .max(1.0) as usize;
                    out.extend((1..=n).map(|k| {
                        let t = k as f64 / n as f64;
                        (cur.0 + (end.0 - cur.0) * t, cur.1 + (end.1 - cur.1) * t)
                    }));
                }
                _ => {
                    let b = if cmd == "Q" {
                        // Degree-elevate the quadratic to the same cubic.
                        let c = pts[0];
                        [
                            cur,
                            (
                                cur.0 + 2.0 * (c.0 - cur.0) / 3.0,
                                cur.1 + 2.0 * (c.1 - cur.1) / 3.0,
                            ),
                            (
                                end.0 + 2.0 * (c.0 - end.0) / 3.0,
                                end.1 + 2.0 * (c.1 - end.1) / 3.0,
                            ),
                            end,
                        ]
                    } else {
                        [cur, pts[0], pts[1], end]
                    };
                    out.extend((1..=64).map(|k| cubic_at(&b, k as f64 / 64.0)));
                }
            }
            cur = end;
        }
        out
    }

    #[test]
    fn edge_picking_matches_the_painted_strokes_on_every_fixture() {
        // pick == render, end to end: for every flowchart, state, ER and
        // class fixture,
        // walk each painted edge along the path `d` that `to_svg` actually
        // writes (parsed back out of the SVG's own numbers, not through
        // `edge_polyline`), and at every point outside the nodes and the
        // label chips `edge_at` must answer that edge — or another edge
        // whose painted stroke covers the same point (a shared trunk, a
        // crossing). A middle routed waypoint or a bezier midpoint is NOT
        // such a point: the spline through the waypoints cuts corners.
        const TOL: f64 = 0.0;
        // The SVG writes coordinates to 0.1, so the curve parsed back out
        // of it sits up to ~0.07 off the one edge_at measures.
        const SVG_PRECISION: f64 = 0.1;
        let root = env!("CARGO_MANIFEST_DIR");
        let mut files: Vec<std::path::PathBuf> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        let (mut graphs, mut probes) = (0, 0);
        for f in &files {
            let src = std::fs::read_to_string(f).unwrap();
            use crate::model::Document;
            let (s, svg, (tx, ty)) = match crate::parser::parse_document(&src) {
                Ok(Document::Flowchart(g) | Document::State(g)) => {
                    let s = scene(&g);
                    let svg = to_svg(&s);
                    // Undo to_svg's margin shift to get back to scene units.
                    let mut bb = Bbox::new();
                    grow_canvas(&mut bb, &s.nodes, &s.edges, &s.clusters);
                    let (minx, _, miny, _) = bb.finish();
                    (s, svg, (MARGIN - minx, MARGIN - miny))
                }
                // ER and class scenes pick through this same edge_at, and
                // their painters stroke edge_path_d in scene units.
                Ok(Document::Er(d)) => {
                    let es = crate::er::scene(&d);
                    let svg = crate::er::to_svg(&es);
                    (es.scene, svg, (0.0, 0.0))
                }
                Ok(Document::Class(d)) => {
                    let cs = crate::class::scene(&d);
                    let svg = crate::class::to_svg(&cs);
                    (cs.scene, svg, (0.0, 0.0))
                }
                _ => continue,
            };
            graphs += 1;
            let curves: Vec<Option<Vec<(f64, f64)>>> = s
                .edges
                .iter()
                .map(|e| {
                    edge_drawn(e).then(|| {
                        // The stroke as painted, cut short under an
                        // arrowhead; the head inks that stretch, so the
                        // whole curve is what a pick must cover.
                        let painted = edge_path_d_painted(e, |p| (p.0 + tx, p.1 + ty));
                        assert!(
                            svg.contains(&format!("<path d=\"{painted}\"")),
                            "{f:?}: {painted} not painted"
                        );
                        let d = edge_path_d(e, |p| (p.0 + tx, p.1 + ty));
                        path_d_points(&d, 2.0)
                            .into_iter()
                            .map(|p| (p.0 - tx, p.1 - ty))
                            .collect()
                    })
                })
                .collect();
            let covers = |j: usize, p: (f64, f64)| {
                let c = curves[j].as_ref().expect("picked edge is painted");
                let d = c
                    .windows(2)
                    .map(|w| point_seg_dist(p.0, p.1, w[0], w[1]))
                    .fold(f64::INFINITY, f64::min);
                d - edge_stroke_width(&s.edges[j]) / 2.0
            };
            let in_chip = |p: (f64, f64)| {
                s.edges.iter().any(|e| {
                    e.label.as_ref().is_some_and(|(text, m, w)| {
                        let lines = 1 + text.matches("<br").count() + text.matches('\n').count();
                        (p.0 - m.0).abs() <= w / 2.0 + 2.0
                            && (p.1 - m.1).abs()
                                <= LABEL_BOX_H / 2.0 + LINE_H * (lines - 1) as f64 + 2.0
                    })
                })
            };
            for (i, c) in curves.iter().enumerate() {
                let Some(c) = c else { continue };
                for &p in c {
                    if s.node_at(p.0, p.1).is_some() || in_chip(p) {
                        continue;
                    }
                    probes += 1;
                    let got = s.edge_at(p.0, p.1, TOL);
                    // How far outside a painted stroke the pick lands.
                    let off = match got {
                        Some(j) if j == i => 0.0,
                        Some(j) => covers(j, p),
                        None => covers(i, p),
                    };
                    assert!(
                        got.is_some() && off <= SVG_PRECISION,
                        "{f:?}: edge {i} ({} -> {}) at {p:?} picked {got:?}, {off:.3} off its stroke",
                        s.edges[i].from,
                        s.edges[i].to
                    );
                }
            }
        }
        assert!(
            graphs >= 15 && probes > 1000,
            "{graphs} graphs, {probes} probes"
        );
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

    /// B7 review r1 (#8): two boxes closer than the channel's two steps
    /// get no channel (its step out of one would pass its step into the
    /// other and the path double back); far enough apart they do, in
    /// order along the flow, either way and in either direction.
    #[test]
    fn a_cross_cluster_channel_never_doubles_back() {
        // BT: the source box below, the target's 18 px above it.
        let (a, b) = ((0.0, 302.0, 134.0, 288.0), (0.0, 28.0, 134.0, 256.0));
        assert!(cross_cluster_route(
            (95.0, 361.0),
            (92.0, 38.0),
            a,
            (95.0, 237.0),
            (65.0, 38.0),
            b,
            true
        )
        .is_empty());
        let a = (0.0, 330.0, 134.0, 288.0);
        let up = cross_cluster_route(
            (95.0, 391.0),
            (92.0, 38.0),
            a,
            (95.0, 237.0),
            (65.0, 38.0),
            b,
            true,
        );
        assert!(up.windows(2).all(|w| w[1].1 <= w[0].1), "{up:?}");
        // LR: the same, sideways.
        let (a, b) = ((0.0, 0.0, 100.0, 80.0), (120.0, 0.0, 100.0, 80.0));
        assert!(cross_cluster_route(
            (50.0, 40.0),
            (60.0, 30.0),
            a,
            (170.0, 40.0),
            (60.0, 30.0),
            b,
            false
        )
        .is_empty());
        let b = (140.0, 0.0, 100.0, 80.0);
        let right = cross_cluster_route(
            (50.0, 40.0),
            (60.0, 30.0),
            a,
            (190.0, 40.0),
            (60.0, 30.0),
            b,
            false,
        );
        assert!(
            right.len() == 4 && right.windows(2).all(|w| w[1].0 >= w[0].0),
            "{right:?}"
        );
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

    /// Fork/join bars span their branches, which then drop straight
    /// onto their states (no S-bends off a 60px bar); the bar's single
    /// way in (fork) or out (join) meets it at its middle. The static
    /// layout and the interactive builders agree on all of it.
    #[test]
    fn composite_branches_and_bar_notes_size_the_bar() {
        let state = |src: &str| match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            _ => unreachable!(),
        };
        // Where an edge is drawn from and to.
        let ends = |e: &SceneEdge| match (e.waypoints.first(), e.waypoints.last()) {
            (Some(&a), Some(&b)) => (a, b),
            _ => (e.bezier[0], e.bezier[3]),
        };
        // A composite state is a branch too: the bars span the frame, so
        // its branches drop straight like the others (fit_bars counts
        // sub-edges, sizing the frame from its members).
        let g = state(
            "stateDiagram-v2\nstate f <<fork>>\nstate j <<join>>\n[*] --> f\n\
             f --> Work\nf --> Audit\nstate Work {\n[*] --> step1\nstep1 --> step2\n\
             step2 --> [*]\n}\nWork --> j\nAudit --> j\nj --> Done\nf --> Log : side\n\
             Log --> Done",
        );
        let sc = scene(&g);
        let branches: Vec<&SceneEdge> = sc
            .edges
            .iter()
            .filter(|e| (e.from == "f" && e.to != "f") || (e.to == "j" && e.from != "j"))
            .collect();
        assert_eq!(
            branches.len(),
            5,
            "fork: Work, Audit, Log; join: Work, Audit"
        );
        for e in branches {
            // Within a px: a frame's port sits under its box's centre, a
            // hair off its members'.
            let (p0, p3) = ends(e);
            assert!((p0.0 - p3.0).abs() < 1.0, "{} -> {} bends", e.from, e.to);
        }
        // A note beside a bar is no branch: the bar spans its branches
        // (and a port's room for the note's link), the card sits beside
        // them, the branches drop straight, a lone way in or out meets
        // the bar's middle and the link leaves past the branches.
        let plain = "stateDiagram-v2\nstate forked <<fork>>\nstate joined <<join>>\n\
                     [*] --> forked\nforked --> A\nforked --> B\nA --> joined\n\
                     B --> joined\njoined --> Done";
        let noted =
            format!("{plain}\nnote right of forked : starts both\nnote right of joined : waits");
        let base = scene(&state(plain));
        let g = state(&noted);
        let sc = scene(&g);
        let centres: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (s, why) in [(&sc, "scene"), (&route(&g, &centres), "route")] {
            let at = |id: &str| &s.nodes[g.node_index(id).unwrap()];
            for bar in ["forked", "joined"] {
                let b = at(bar);
                assert_eq!(
                    b.w,
                    base.nodes[g.node_index(bar).unwrap()].w + 2.0 * crate::ports::PORT_SPACING,
                    "{why}: {bar} spans its branches, not the card"
                );
                let link = s
                    .edges
                    .iter()
                    .find(|e| e.from == bar && e.to.starts_with("__note_"));
                let (p0, _) = ends(link.expect("the note link"));
                let outer = at("B").x.max(at("A").x);
                assert!(p0.0 > outer + 1.0, "{why}: {bar}'s note link past B");
            }
            for e in s.edges.iter().take(g.edges.len()) {
                let (p0, p3) = ends(e);
                if ["A", "B"].contains(&e.to.as_str()) || e.to == "joined" {
                    assert!(
                        (p0.0 - p3.0).abs() < 0.5,
                        "{why}: {} -> {} bends",
                        e.from,
                        e.to
                    );
                }
                if e.from == "joined" && e.to == "Done" {
                    assert!((p0.0 - at("joined").x).abs() < 0.5, "{why}: way out");
                }
            }
        }
    }

    #[test]
    fn fork_join_branches_drop_straight_and_drags_agree() {
        let state = |src: &str| match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            _ => unreachable!(),
        };
        let g = state(include_str!("../examples/state.mmd"));
        let ix = |id: &str| g.node_index(id).unwrap();
        let (fork, join) = (ix("forked"), ix("joined"));
        // Branch edges: straight (start and end on one x). The lone
        // in/out edge: at the bar's middle.
        let check = |s: &Scene, why: &str| {
            for e in s.edges.iter().take(g.edges.len()) {
                let (from, to) = (ix(&e.from), ix(&e.to));
                let (p0, p3) = (e.bezier[0], e.bezier[3]);
                if from == fork || to == join {
                    assert!((p0.0 - p3.0).abs() < 0.5, "{why}: {e:?} bends");
                }
                if to == fork {
                    assert!((p3.0 - s.nodes[fork].x).abs() < 0.5, "{why}: fork way in");
                }
                if from == join {
                    assert!((p0.0 - s.nodes[join].x).abs() < 0.5, "{why}: join way out");
                }
            }
            // The bar reaches past its outermost branch.
            let reach = (s.nodes[ix("Notify")].x - s.nodes[ix("Persist")].x).abs() / 2.0;
            assert!(s.nodes[fork].w / 2.0 >= reach + BAR_OVERHANG - 0.5, "{why}");
        };
        let sc = scene(&g);
        check(&sc, "scene");
        let centres: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route(&g, &centres);
        check(&rt, "route");
        for (a, b) in sc.nodes.iter().zip(&rt.nodes) {
            assert_eq!((a.w, a.h), (b.w, b.h), "{}: drag keeps the size", a.id);
        }
        // Drag a branch sideways (still under the bar): its branch
        // follows it straight down, in a full and a partial reroute.
        let mut moved = centres.clone();
        moved[ix("Persist")].0 += 6.0;
        for s in [route(&g, &moved), route_partial(&g, &moved, &sc, &centres)] {
            assert_eq!(s.nodes[fork].w, sc.nodes[fork].w);
            let e = s
                .edges
                .iter()
                .find(|e| e.from == "forked" && e.to == "Persist");
            let e = e.unwrap();
            assert!(
                (e.bezier[0].0 - moved[ix("Persist")].0).abs() < 0.5,
                "{e:?}"
            );
            assert!(
                (e.bezier[3].0 - moved[ix("Persist")].0).abs() < 0.5,
                "{e:?}"
            );
        }
        // Across an `LR` flow the bar stands upright.
        let g = state("stateDiagram-v2\ndirection LR\nstate f <<fork>>\nA --> f\nf --> B\nf --> C");
        let s = scene(&g);
        let bar = &s.nodes[g.node_index("f").unwrap()];
        assert!(bar.h > 2.0 * bar.w, "upright: {}x{}", bar.w, bar.h);
        for e in s.edges.iter().filter(|e| e.from == "f") {
            assert!((e.bezier[0].1 - e.bezier[3].1).abs() < 0.5, "{e:?}");
        }
    }

    #[test]
    fn bar_reach_covers_any_branch_order() {
        // Two branches: the bar sits between them, half the span.
        assert_eq!(bar_half(&mut [60.0, 80.0]), (70.0 + GAP_B) / 2.0);
        // Three: aligned with the middle one, the farthest reach puts
        // the two broadest on one side.
        let three = bar_half(&mut [190.0, 66.0, 56.0]);
        assert_eq!(three, (190.0 + 66.0) / 2.0 + GAP_B);
        assert_eq!(bar_half(&mut [40.0]), 0.0);
        // A bar with no fan keeps its intrinsic size; fitting is
        // idempotent (the static and route builders both fit).
        let g = parse("flowchart TD\nA@{ shape: fork } --> B").unwrap();
        let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
        assert_eq!(fit_bars(&g, &sizes)[0], (60.0, 8.0));
        let g = parse("flowchart TD\nA@{ shape: fork } --> B & C & D").unwrap();
        let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
        let once = fit_bars(&g, &sizes).into_owned();
        assert!(once[0].0 > 60.0);
        assert_eq!(fit_bars(&g, &once).as_ref(), &once[..]);
        // No bar: the caller's sizes come back borrowed.
        let g = parse("flowchart TD\nA --> B").unwrap();
        let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
        assert!(matches!(fit_bars(&g, &sizes), Cow::Borrowed(_)));
    }

    #[test]
    fn a_note_card_beside_its_state_links_from_the_facing_side() {
        let state = |src: &str| match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            other => panic!("not a state diagram: {other:?}"),
        };
        // LR: `note right of Draft` lands a layer on, below Review,
        // wholly past Draft across the flow. Its link used to leave
        // Draft's flow side and run under the 'changes' label between
        // Draft and Review; it now leaves Draft's bottom and rounds
        // into the card's left side.
        let g = state(
            "stateDiagram-v2\ndirection LR\n[*] --> Draft\nDraft --> Review : submit\n\
             Review --> Draft : changes\nReview --> Published : approve\n\
             note right of Draft : author edits here\nPublished --> [*]",
        );
        let link = |sc: &Scene| {
            let ei = sc
                .edges
                .iter()
                .position(|e| {
                    e.from == "Draft" && sc.nodes.iter().any(|n| n.id == e.to && is_note_card(n))
                })
                .expect("Draft's note link");
            let e = &sc.edges[ei];
            let node = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().clone();
            (ei, e.clone(), node("Draft"), node(&e.to))
        };
        let s = scene(&g);
        let centres: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (what, sc) in [("scene", s.clone()), ("route", route(&g, &centres))] {
            let (ei, e, draft, card) = link(&sc);
            let (p0, p3) = (e.bezier[0], e.bezier[3]);
            assert!(
                (p0.1 - (draft.y + draft.h / 2.0)).abs() < 0.5
                    && (p0.0 - draft.x).abs() < draft.w / 2.0,
                "{what}: leaves Draft's bottom: {e:?}"
            );
            assert!(
                (p3.0 - (card.x - card.w / 2.0)).abs() < 0.5,
                "{what}: enters the card's left side: {e:?}"
            );
            let line = edge_polyline(&e);
            for (oj, o) in sc.edges.iter().enumerate() {
                let Some((text, c, w)) = &o.label else {
                    continue;
                };
                let r = (c.0 - w / 2.0, c.1 - 10.0, c.0 + w / 2.0, c.1 + 10.0);
                let hit = line
                    .iter()
                    .any(|p| p.0 > r.0 && p.0 < r.2 && p.1 > r.1 && p.1 < r.3);
                assert!(oj == ei || !hit, "{what}: the link runs under '{text}'");
            }
        }
        // A card level with its state (overlapping it across the flow)
        // keeps the ordinary link, off Draft's flow side.
        let (_, _, draft, card) = link(&s);
        let mut moved = centres.clone();
        let ci = g.node_index(&card.id).unwrap();
        moved[ci].1 = draft.y;
        let r = route(&g, &moved);
        let (_, e, draft, _) = link(&r);
        // On the flank — near its corner, on the rounded corner's arc
        // (`edgepass::snap_ends`), within 1.5 px of the box side.
        assert!(
            (e.bezier[0].0 - (draft.x + draft.w / 2.0)).abs() < 1.5
                && (e.bezier[0].1 - draft.y).abs() < draft.h / 2.0,
            "flow side: {e:?}"
        );
        // `note right of S` lands past S's own fan, a transition from S
        // running to a state beyond the card: the elbow would cross it,
        // so the link keeps the ordinary course and crosses no line.
        let g = state(
            "stateDiagram-v2\ndirection LR\n[*] --> S\nS --> T1\nS --> T2\nS --> T3\n\
             T1 --> S : back\nnote right of S : busy state\nnote left of T2 : t2 note",
        );
        let s = scene(&g);
        let centres: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let cross = |p: (f64, f64), q: (f64, f64), u: (f64, f64), v: (f64, f64)| {
            let side = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
                (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
            };
            side(p, q, u) * side(p, q, v) < 0.0 && side(u, v, p) * side(u, v, q) < 0.0
        };
        // Dragging the card a little re-routes its link through the
        // same rule; it still keeps clear.
        let mut dragged = centres.clone();
        let ci = (0..g.nodes.len())
            .find(|&v| is_note_card(&s.nodes[v]) && s.nodes[v].label == "busy state")
            .unwrap();
        dragged[ci].0 += 10.0;
        for (what, sc) in [
            ("scene", s.clone()),
            ("route", route(&g, &centres)),
            ("route_partial", route_partial(&g, &dragged, &s, &centres)),
        ] {
            let ei = sc
                .edges
                .iter()
                .position(|e| {
                    e.from == "S" && sc.nodes.iter().any(|n| n.id == e.to && is_note_card(n))
                })
                .expect("S's note link");
            let line = edge_polyline(&sc.edges[ei]);
            for (oj, o) in sc.edges.iter().enumerate() {
                let other = edge_polyline(o);
                let hit = oj != ei
                    && line
                        .windows(2)
                        .any(|a| other.windows(2).any(|b| cross(a[0], a[1], b[0], b[1])));
                assert!(!hit, "{what}: the link crosses {} -> {}", o.from, o.to);
            }
        }
        // A <<choice>> has no flat side: its cards' links stay as they were.
        let g = state(
            "stateDiagram-v2\nstate c <<choice>>\n[*] --> A\nA --> c\nc --> B : yes\n\
             c --> C : no\nnote right of c : decide here",
        );
        let s = scene(&g);
        let ci = g.node_index("c").unwrap();
        let (c, n) = (&s.nodes[ci], g.edges.len());
        let e = s.edges[..n]
            .iter()
            .find(|e| e.from == "c" && e.label.is_none())
            .unwrap();
        assert!(
            e.bezier[0].1 > c.y + 0.5,
            "off the choice's lower half: {e:?}"
        );
    }

    #[test]
    fn bars_branching_to_bars_fit_the_same_every_time() {
        let state = |src: &str| match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            other => panic!("not a state diagram: {other:?}"),
        };
        // A fork straight to its join (an empty branch), across both
        // flows, and nested forks: a bar is a branch of another bar.
        let fork_join = "state f <<fork>>\nstate j <<join>>\n[*] --> f\nf --> A\nf --> B\n\
                         f --> j\nA --> j\nB --> j\nj --> End\nEnd --> [*]";
        let nested = "state f1 <<fork>>\nstate f2 <<fork>>\n[*] --> f1\nf1 --> f2\nf1 --> Z\n\
                      f2 --> X\nf2 --> Y";
        for src in [
            format!("stateDiagram-v2\n{fork_join}"),
            format!("stateDiagram-v2\ndirection LR\n{fork_join}"),
            format!("stateDiagram-v2\n{nested}"),
        ] {
            let g = state(&src);
            let intrinsic: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
            let once = fit_bars(&g, &intrinsic).into_owned();
            assert_eq!(fit_bars(&g, &once).as_ref(), &once[..], "{src}");
            let s = scene(&g);
            let centres: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let drawn: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.w, n.h)).collect();
            // The compact builder fits, then draws through `scene_sized`
            // (fitting again); skipped, it is the ordinary scene.
            let c = crate::fold::scene_compact(&g, &crate::fold::CompactOptions::for_extent(1e5));
            assert!(c.skipped.is_some(), "{src}");
            let compact: Vec<(f64, f64)> = c.scene.nodes.iter().map(|n| (n.w, n.h)).collect();
            assert_eq!(compact, drawn, "{src}");
            let same_edges = c
                .scene
                .edges
                .iter()
                .zip(&s.edges)
                .all(|(a, b)| a.bezier == b.bezier);
            assert!(same_edges, "{src}");
            // A host routing with the sizes it drew, or with intrinsic
            // ones, gets the drawn bars back.
            for r in [route_sized(&g, &centres, &drawn), route(&g, &centres)] {
                let sized: Vec<(f64, f64)> = r.nodes.iter().map(|n| (n.w, n.h)).collect();
                assert_eq!(sized, drawn, "{src}");
            }
            // Each bar still spans its own branches.
            for (v, n) in g.nodes.iter().enumerate() {
                if n.shape == Shape::ForkBar {
                    assert!(drawn[v].0.max(drawn[v].1) > BAR_SIZE.0, "{src}: {}", n.id);
                }
            }
        }
        // A fork right above another (nested) reaches that fork's
        // middle: the far bar counts at its own fit, so both branches
        // drop straight; a fork straight to its join counts as a lane.
        let g = state(&format!("stateDiagram-v2\n{nested}"));
        let s = scene(&g);
        for e in s.edges.iter().filter(|e| e.from == "f1") {
            assert!((e.bezier[0].0 - e.bezier[3].0).abs() < 0.5, "{e:?}");
        }
        let g = state(&format!("stateDiagram-v2\n{fork_join}"));
        let (f, j) = (g.node_index("f").unwrap(), g.node_index("j").unwrap());
        assert!(!beside(&g, f, j));
        let g = state(&format!("stateDiagram-v2\n{nested}"));
        let (f1, f2) = (g.node_index("f1").unwrap(), g.node_index("f2").unwrap());
        assert!(beside(&g, f1, f2));
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
    fn an_edge_into_a_box_with_a_cycle_still_ranks_the_box_below() {
        // Every member of B has an internal in-edge (b2 --> b1 closes a
        // cycle), so B stood for no member: `a2 --> B` shaped no rank,
        // and the boxes sat side by side. The cycle's closing edge is set
        // aside: b1 enters B, b2 leaves it.
        let src = "flowchart TD\nsubgraph A\n a1-->a2\nend\na2-->B\n\
                   subgraph B\n b1-->b2\n b2-->b1\nend\nB-->z";
        let g = parse(src).unwrap();
        let nc = node_cluster_paths(&g);
        let groups = sub_edge_proxies(&g, &nc, &[]);
        let id = |v: &[usize]| -> Vec<&str> { v.iter().map(|&i| g.nodes[i].id.as_str()).collect() };
        assert_eq!(id(&groups[0].1), ["b1"], "B entered at b1");
        assert_eq!(id(&groups[1].0), ["b2"], "B left at b2");
        // A box that is one cycle still has an entry and an exit.
        let g2 = parse("flowchart TD\nx-->C\nsubgraph C\n c1-->c2-->c3-->c1\nend\nC-->y").unwrap();
        let groups = sub_edge_proxies(&g2, &node_cluster_paths(&g2), &[]);
        assert_eq!(groups[0].1, [g2.node_index("c1").unwrap()]);
        assert_eq!(groups[1].0, [g2.node_index("c3").unwrap()]);
        let s = scene(&g);
        let frame = |id: &str| s.clusters.iter().find(|c| c.id == id).unwrap();
        let (a, b) = (frame("A"), frame("B"));
        assert!(
            a.y + a.h < b.y,
            "A ({}..{}) above B ({})",
            a.y,
            a.y + a.h,
            b.y
        );
        // The edge drops straight from a2 onto B's top side.
        let e = s.edges.iter().find(|e| e.to == "B").unwrap();
        let pl = edge_polyline(e);
        let (p0, p3) = (pl[0], pl[pl.len() - 1]);
        assert!((p3.1 - b.y).abs() < 0.5, "ends on B's top, got {p3:?}");
        assert!((p0.0 - p3.0).abs() < 0.5, "straight drop: {p0:?} -> {p3:?}");
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
        grow_canvas(&mut bb, &s.nodes, &s.edges, &s.clusters);
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
            // The node label kept on one line (it would wrap at the
            // default width), so it reaches past the title.
            &parse(&format!(
                "%%{{init: {{\"flowchart\": {{\"wrappingWidth\": 2000}}}}}}%%\n\
                 flowchart TB\nsubgraph R [\"{long}\"]\n  r\nend\n\
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
            p[0].contains("stroke=\"#44507a\" stroke-width=\"2\" marker-end=\"url(#arrow)\""),
            "{svg}"
        );
        assert!(
            p[1].contains("stroke=\"#ff3\" stroke-width=\"4\" marker-end=\"url(#arrow__ff3)\""),
            "{svg}"
        );
        // Width-independent head (the one every arrow has), coloured
        // like the stroke.
        assert!(svg.contains(
            "<marker id=\"arrow__ff3\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"11\" markerHeight=\"11\" \
             orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#ff3\"/></marker>"
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
        assert!(p[0].contains("stroke=\"#888\" stroke-width=\"2\"") && p[0].contains("arrow__888"));
        assert!(
            p[1].contains("stroke=\"#e11d48\" stroke-width=\"2\"")
                && p[1].contains("arrow__e11d48")
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
    fn dotted_edges_draw_dots_that_scale_with_their_stroke() {
        // Report Q9: `-.->` reads as dotted — dots one stroke long, a
        // stroke and a half apart (mermaid: `2 2` at 2 px), not the
        // long `5 4` dashes that read as dashed.
        let svg = svg_src(
            "flowchart LR\nA -.-> B\nC -.- D\nE -.-> F\nG -.-> H\n\
             linkStyle 2 stroke-width:4px\nlinkStyle 3 stroke-dasharray: 5 5\n",
        );
        let p = edge_paths(&svg);
        assert!(p[0].contains("stroke-dasharray=\"2 3\""), "{svg}");
        assert!(p[1].contains("stroke-dasharray=\"2 3\""), "{svg}");
        // A wider stroke keeps the same look, not stripes.
        assert!(p[2].contains("stroke-dasharray=\"4 6\""), "{svg}");
        // A user dash still wins.
        assert!(p[3].contains("stroke-dasharray=\"5 5\""), "{svg}");
        assert_eq!(dotted_dash(EDGE_WIDTH), "2 3");
        assert_eq!(dotted_dash(EDGE_WIDTH_THICK), "3.5 5.3");
        // The advance writer draws the same dots.
        let adv = crate::advance::render_advance_text_svg("lane l \"L\"\na[A]\nb[B]\na -.-> b\n")
            .unwrap();
        assert!(adv.contains("stroke-dasharray=\"2 3\""), "{adv}");
    }

    #[test]
    fn link_style_dash_and_width_override_the_kind() {
        let svg = svg_src(
            "flowchart LR\nA -.-> B\nC ==> D\nE --> F\nlinkStyle 0 stroke-dasharray:0\n\
             linkStyle 1 stroke-width:1px\nlinkStyle 2 stroke-dasharray: 5 5\n",
        );
        let p = edge_paths(&svg);
        assert!(
            p[0].contains("stroke-dasharray=\"0\"") && !p[0].contains("stroke-dasharray=\"2 3\"")
        );
        assert!(p[1].contains("stroke-width=\"1\"") && p[1].contains("url(#arrow)"));
        assert!(p[2].contains("stroke-dasharray=\"5 5\""));
        // A width alone needs no variant: every head has one size.
        assert_eq!(svg.matches("<marker id=").count(), 1, "{svg}");
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

    // ── end glyphs (report Q1 + M1) ─────────────────────────────

    /// The `d` of the painted path and its points, end last.
    fn painted_points(path: &str) -> Vec<(f64, f64)> {
        let d = path
            .strip_prefix("<path d=\"")
            .and_then(|r| r.split('"').next())
            .expect("a path");
        path_d_points(d, 0.5)
    }

    #[test]
    fn every_edge_kind_and_width_gets_one_head_size() {
        // A thick edge, a styled wider one and a plain one share `#arrow`,
        // sized in user space: the head no longer doubles on `==>`.
        let svg = svg_src(
            "flowchart LR\nA --> B\nC ==> D\nE -.-> F\nG --> H\nlinkStyle 3 stroke-width:5px\n",
        );
        for p in edge_paths(&svg) {
            assert!(p.contains("marker-end=\"url(#arrow)\""), "{p}");
        }
        assert_eq!(svg.matches("<marker id=").count(), 1, "{svg}");
        // A stroke wider than the cut keeps the head's size too, set
        // further ahead of its own deeper cut (`arrow-t6`).
        let wide = svg_src("flowchart LR\nG --> H\nlinkStyle 0 stroke-width:6px\n");
        assert!(wide.contains("marker-end=\"url(#arrow-t6)\""), "{wide}");
        assert_eq!(wide.matches("markerWidth=\"11\"").count(), 2, "{wide}");
        assert!(svg.contains(&format!(
            "markerUnits=\"userSpaceOnUse\" markerWidth=\"{ARROW_MARKER_PX}\""
        )));
        // The cut leaves the stroke's butt end inside the head: half the
        // head's width there covers half the thick stroke.
        let sc = scene(&parse("flowchart LR\nC ==> D\n").unwrap());
        let half_stroke = edge_stroke_width(&sc.edges[0]) / 2.0;
        assert!(ARROW_TRIM / 2.0 > half_stroke, "{half_stroke}");
        assert_eq!(svg.matches("markerWidth=\"11\"").count(), 1);
        // Every glyph shares the size (cross and circle too).
        let svg = svg_src("flowchart LR\nA <--> B\nC x--o D\n");
        assert_eq!(
            svg.matches(&format!("markerWidth=\"{ARROW_MARKER_PX}\""))
                .count(),
            svg.matches("<marker id=").count(),
            "{svg}"
        );
    }

    #[test]
    fn arrow_tips_land_on_the_border_with_the_stroke_cut_under_the_head() {
        for op in ["-->", "==>", "-.->", "<-->"] {
            let src = format!("flowchart TD\nA {op} B\n");
            let sc = scene(&parse(&src).unwrap());
            let svg = to_svg(&sc);
            let pts = painted_points(edge_paths(&svg)[0]);
            // SVG coordinates: the drawing starts MARGIN in.
            let mut bb = Bbox::new();
            grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
            let (minx, _, miny, _) = bb.finish();
            let t = |p: (f64, f64)| (p.0 + MARGIN - minx, p.1 + MARGIN - miny);
            let b = &sc.nodes[1];
            let top = t((b.x, b.y - b.h / 2.0)).1;
            // The stroke stops ARROW_TRIM short, heading straight down;
            // the marker's refX puts the tip back on the border.
            let (end, before) = (pts[pts.len() - 1], pts[pts.len() - 2]);
            assert!(before.0 == end.0 && before.1 < end.1, "{op}: {pts:?}");
            assert!(
                (end.1 + ARROW_TRIM - top).abs() < 0.11,
                "{op}: {end:?} vs {top}"
            );
            let refx: f64 = svg
                .split("<marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"")
                .nth(1)
                .and_then(|r| r.split('"').next())
                .and_then(|r| r.parse().ok())
                .expect("refX");
            assert!(((10.0 - refx) * ARROW_MARKER_PX / 10.0 - ARROW_TRIM).abs() < 1e-9);
            if op == "<-->" {
                let a = &sc.nodes[0];
                let bottom = t((a.x, a.y + a.h / 2.0)).1;
                assert!((pts[0].1 - ARROW_TRIM - bottom).abs() < 0.11, "{pts:?}");
            }
            // Picking keeps the whole curve: under the head it is ink.
            let e = &sc.edges[0];
            assert_eq!(sc.edge_at(e.bezier[3].0, e.bezier[3].1 - 2.0, 0.0), Some(0));
        }
    }

    /// `pts` sampled `steps` per command of the path (start, commands).
    fn dense_path(start: (f64, f64), segs: &[Seg], steps: usize) -> Vec<(f64, f64)> {
        let mut out = vec![start];
        let mut cur = start;
        for seg in segs {
            let b = seg.cubic(cur);
            out.extend((1..=steps).map(|k| cubic_at(&b, k as f64 / steps as f64)));
            cur = seg.end();
        }
        out
    }

    /// How far the squared end at `tip` (heading `d` into it) strays from
    /// the scene curve `plain`: the farthest point of `painted` from it,
    /// over the points outside the arrowhead (half stroke `hw`, `None`
    /// for a cross or a circle), as (beyond `near` of the tip, within).
    fn end_stray(
        painted: &[(f64, f64)],
        plain: &[(f64, f64)],
        tip: (f64, f64),
        d: (f64, f64),
        hw: Option<f64>,
        near: f64,
    ) -> (f64, f64) {
        let reach = 4.0 * near;
        let close = |q: &&(f64, f64)| (q.0 - tip.0).hypot(q.1 - tip.1) <= reach;
        // The scene curve's stretch round the tip, as a polyline (its
        // pieces whose either end is that close).
        let line: Vec<(f64, f64)> = plain
            .windows(2)
            .filter(|w| close(&&w[0]) || close(&&w[1]))
            .flat_map(|w| [w[0], w[1]])
            .collect();
        let mut out = (0.0f64, 0.0f64);
        for &q in painted.iter().filter(close) {
            let along = (tip.0 - q.0) * d.0 + (tip.1 - q.1) * d.1;
            let lat = ((q.0 - tip.0) * d.1 - (q.1 - tip.1) * d.0).abs();
            if hw.is_some_and(|hw| {
                (0.0..=ARROW_MARKER_PX).contains(&along) && lat + hw <= along / 2.0
            }) {
                continue;
            }
            let off = line
                .windows(2)
                .map(|w| point_seg_dist(q.0, q.1, w[0], w[1]))
                .fold(f64::INFINITY, f64::min);
            if (q.0 - tip.0).hypot(q.1 - tip.1) > near {
                out.0 = out.0.max(off);
            } else {
                out.1 = out.1.max(off);
            }
        }
        out
    }

    #[test]
    fn a_turning_curve_runs_straight_into_its_end_glyph_near_its_scene_curve() {
        // A fan: the outer branches are S-curves whose cubic still turns
        // under the head. The painter squares each end to the head: the
        // head faces the cubic's own end tangent (plumb onto the top side)
        // with its tip on the end, the stroke cut back to that tangent;
        // where bending into a straight run keeps within SQUARE_END_DEV of
        // the cubic, the stroke runs straight into the head's base.
        // Nowhere farther than two heads from the tip does the painted
        // curve stray more than that — the end point stays put, unlike a
        // stub that moves it and drags the whole tail along — and the
        // scene keeps its cubic (a host draws it).
        let g = parse("flowchart TD\nA --> B & C & D & E & F\n").unwrap();
        let sc = scene(&g);
        let mut longer = 0;
        for e in &sc.edges {
            let (tip, heading) = edge_end_tangent(e, false);
            assert_eq!(tip, e.bezier[3]);
            assert!(heading.0.abs() < 1e-9 && heading.1 > 0.0, "{heading:?}");
            assert_eq!(e.bezier[2].0, e.bezier[3].0, "the scene's cubic");
            let (start, segs) = drawn_segs(e, |p| p);
            assert_eq!(segs.last().map(|s| s.end()), Some(tip));
            let (_, cut) = trim_segs(start, segs.clone(), 0.0, ARROW_TRIM);
            let end = cut.last().unwrap().end();
            assert!((end.0 - tip.0).abs() < 1e-9, "{end:?}");
            // (On the straight spine the cut falls on the cubic, found by
            // arc length over its samples: to a hundredth of a px.)
            assert!((tip.1 - end.1 - ARROW_TRIM).abs() < 0.01, "{end:?}");
            // How far the drawn curve runs plumb into the tip.
            let pts = dense_path(start, &segs, 64);
            let run = pts
                .iter()
                .rev()
                .take_while(|q| (q.0 - tip.0).abs() < 1e-6)
                .last()
                .map_or(0.0, |q| tip.1 - q.1);
            assert!(run >= ARROW_TRIM + SQUARE_END_ENTRY - 1e-6, "{run}");
            longer += usize::from(run > ARROW_TRIM + SQUARE_END_ENTRY + 1e-6);
            let plain = sample_cubic(&e.bezier, 400);
            let (far, near) = end_stray(
                &pts,
                &plain,
                tip,
                heading,
                Some(edge_stroke_width(e) / 2.0),
                SQUARE_END_ARC,
            );
            assert!(far <= SQUARE_END_DEV + 0.05, "{}->{}: {far}", e.from, e.to);
            assert!(
                near <= ARROW_MARKER_PX / 2.0,
                "{}->{}: {near}",
                e.from,
                e.to
            );
        }
        // The spine runs straight anyway and its neighbours bend into a
        // run longer than the cut; the outermost swing in too hard for one
        // within a px and enter the head along the end tangent.
        assert_eq!(longer, 3);
    }

    /// B1 review r3 (#3): an S-cubic whose end handle is short for its
    /// sideways reach turns square only in its last px (fanout-frames
    /// n0 -> g1: 23 px handles over a 351 px rise; dense-td n16 -> n23, a
    /// shallow curve hooking down onto a top side). Its stroke used to
    /// leave the curve at an angle and make an S, or a hump before the
    /// head, then an elbow. Now it leaves the curve where the line to the
    /// corner under the head touches it, and runs on straight: no turn
    /// back anywhere along the new stretch outside the head, the head
    /// square, the tip on the end, the stray within its bound.
    #[test]
    fn a_curve_swinging_in_from_the_side_enters_its_head_straight() {
        let cubic = |b: [(f64, f64); 4]| SceneEdge {
            bezier: b,
            waypoints: vec![],
            ..routed_edge(b[0], b[3], vec![])
        };
        for e in [
            cubic([
                (215.0, 908.5),
                (238.0, 908.5),
                (238.0, 557.0),
                (261.0, 557.0),
            ]),
            cubic([
                (400.0, 100.0),
                (400.0, 140.0),
                (100.0, 160.0),
                (100.0, 180.0),
            ]),
            cubic([(0.0, 0.0), (40.0, 0.0), (40.0, 90.0), (80.0, 90.0)]),
        ] {
            let tip = e.bezier[3];
            let d = {
                let (dx, dy) = (tip.0 - e.bezier[2].0, tip.1 - e.bezier[2].1);
                let l = dx.hypot(dy);
                (dx / l, dy / l)
            };
            let (t, head) = edge_end_tangent(&e, false);
            assert_eq!(t, tip);
            assert!(
                (head.0 - d.0).abs() < 1e-6 && (head.1 - d.1).abs() < 1e-6,
                "{head:?}"
            );
            let (start, segs) = drawn_segs(&e, |p| p);
            let plain = sample_cubic(&e.bezier, 400);
            let pts = dense_path(start, &segs, 64);
            // The new stretch: where the drawn curve leaves the scene's.
            let off = |q: (f64, f64)| {
                plain
                    .windows(2)
                    .map(|w| point_seg_dist(q.0, q.1, w[0], w[1]))
                    .fold(f64::INFINITY, f64::min)
            };
            let first = pts.iter().position(|&q| off(q) > 0.05).expect("squared");
            let along = |q: (f64, f64)| (tip.0 - q.0) * d.0 + (tip.1 - q.1) * d.1;
            let lat = |q: (f64, f64)| (q.0 - tip.0) * d.1 - (q.1 - tip.1) * d.0;
            // Outside the head, the heading never swings back by more
            // than the join's SQUARE_END_TOUCH (no S), and the stroke
            // never heads back away from the side (no hump).
            let outside: Vec<(f64, f64)> = pts[first.saturating_sub(1)..]
                .iter()
                .copied()
                .filter(|&q| along(q) > ARROW_MARKER_PX)
                .collect();
            let heads: Vec<f64> = outside
                .windows(2)
                .filter(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1) > 1e-6)
                .map(|w| (w[1].1 - w[0].1).atan2(w[1].0 - w[0].0))
                .collect();
            if heads.len() >= 2 {
                let turn = |a: f64, b: f64| {
                    let t = b - a;
                    (t + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                        - std::f64::consts::PI
                };
                let net = turn(heads[0], heads[heads.len() - 1]).signum();
                let mut acc: f64 = 0.0;
                let mut back: f64 = 0.0;
                let mut peak: f64 = 0.0;
                for w in heads.windows(2) {
                    acc += turn(w[0], w[1]) * net;
                    peak = peak.max(acc);
                    back = back.max(peak - acc);
                }
                assert!(
                    back.to_degrees() <= SQUARE_END_TOUCH + 0.5,
                    "{:?}: an S, {:.1} deg back",
                    e.bezier,
                    back.to_degrees()
                );
            }
            assert!(
                outside
                    .windows(2)
                    .all(|w| along(w[1]) <= along(w[0]) + 1e-6),
                "{:?}: a hump",
                e.bezier
            );
            // The head's own run: the last stretch before the cut lies on
            // the end tangent.
            let cut = trim_segs(start, segs.clone(), 0.0, ARROW_TRIM).1;
            let end = cut.last().unwrap().end();
            assert!(
                lat(end).abs() < 1e-6 && (along(end) - ARROW_TRIM).abs() < 0.01,
                "{end:?}"
            );
            let stray = pts.iter().map(|&q| off(q)).fold(0.0, f64::max);
            assert!(stray <= SQUARE_END_MAX_STRAY + 0.5, "{stray}");
        }
    }

    /// B1 review r3 (#5): the advance painter's routes are polylines
    /// cut by arc length under the head. A last leg shorter than the cut
    /// ended the stroke on the leg before it, and `orient="auto"` tilted
    /// the head that way; now the stroke's last stretch lies on the last
    /// leg's line, ending a cut short of the tip, so the head faces along
    /// the last leg with its tip on the route's end.
    #[test]
    fn an_advance_route_with_a_short_last_leg_keeps_its_head_on_that_leg() {
        let last_two = |d: &str| -> [(f64, f64); 2] {
            let v: Vec<f64> = d.split([' ', ',']).filter_map(|t| t.parse().ok()).collect();
            let n = v.len();
            [(v[n - 4], v[n - 3]), (v[n - 2], v[n - 1])]
        };
        for (pts, trim) in [
            (vec![(0.0, 0.0), (0.0, 50.0), (3.0, 50.0)], ARROW_TRIM),
            (vec![(0.0, 0.0), (0.0, 50.0), (6.0, 50.0)], ARROW_TRIM),
            (vec![(0.0, 0.0), (40.0, 0.0), (40.0, -4.0)], 8.0),
        ] {
            let d = polyline_d_painted(&pts, trim);
            let n = pts.len();
            let (a, tip) = (pts[n - 2], pts[n - 1]);
            let l = (tip.0 - a.0).hypot(tip.1 - a.1);
            let dir = ((tip.0 - a.0) / l, (tip.1 - a.1) / l);
            let [p, end] = last_two(&d);
            // Ends a cut short of the tip, on the last leg's line …
            let want = (tip.0 - dir.0 * trim, tip.1 - dir.1 * trim);
            assert!(
                (end.0 - want.0).abs() < 0.06 && (end.1 - want.1).abs() < 0.06,
                "{d}"
            );
            // … and its last stretch heads along that leg.
            let (hx, hy) = (end.0 - p.0, end.1 - p.1);
            let cos = (hx * dir.0 + hy * dir.1) / hx.hypot(hy);
            assert!(cos > 0.9995, "{d}: {cos}");
        }
        // A long last leg is simply cut; a bare end is not cut.
        let pts = [(0.0, 0.0), (0.0, 50.0), (30.0, 50.0)];
        assert_eq!(
            polyline_d_painted(&pts, ARROW_TRIM),
            "M 0.0 0.0 L 0.0 50.0 L 24.5 50.0"
        );
        assert_eq!(
            polyline_d_painted(&pts, 0.0),
            "M 0.0 0.0 L 0.0 50.0 L 30.0 50.0"
        );
    }

    /// B1 review r3 (#4): an over-full side takes its ports about 10 px
    /// apart, and 11 px heads there ran together into one saw-tooth band
    /// (x2_fan's Sink: six heads 10.3 px apart). Heads standing side by
    /// side closer than a head and HEAD_GAP are drawn narrower, so a gap
    /// stays between them; a head with room keeps its full width, and a
    /// wide stroke keeps the head that covers its end.
    #[test]
    fn crowded_heads_narrow_so_neighbours_keep_apart() {
        let g = parse(
            "flowchart TD\nH --> A1 & A2 & A3 & A4 & A5 & A6 & A7 & A8\n\
             A1 & A2 & A3 & A4 & A5 & A6 & A7 & A8 --> S\nS --> T\n",
        )
        .unwrap();
        let sc = scene(&g);
        let painted: Vec<Option<PaintedEdge>> = sc
            .edges
            .iter()
            .map(|e| edge_drawn(e).then(|| painted_edge(e)))
            .collect();
        let heads = head_sizes(&sc, &painted, &vec![None; sc.edges.len()]);
        let widths: Vec<[f64; 2]> = heads.iter().map(|h| [h[0].width, h[1].width]).collect();
        let tips: Vec<((f64, f64), f64)> = sc
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.to == "S")
            .map(|(i, _)| {
                (
                    painted[i].as_ref().unwrap().end(false).unwrap().0,
                    widths[i][1],
                )
            })
            .collect();
        let mut narrowed = 0;
        for (a, wa) in &tips {
            for (b, wb) in &tips {
                let pitch = (a.0 - b.0).abs();
                if (a.1 - b.1).abs() < 0.5 && pitch > 1.0 && pitch < ARROW_MARKER_PX + HEAD_GAP {
                    // Half of each head, and the gap, fit in the pitch.
                    assert!(
                        (wa + wb) / 2.0 + HEAD_GAP <= pitch + 1e-9,
                        "{pitch}: {wa} {wb}"
                    );
                    narrowed += 1;
                }
            }
        }
        assert!(narrowed >= 4, "{tips:?}");
        // S -> T has room: a whole head, the base #arrow.
        let st = sc.edges.iter().position(|e| e.from == "S").unwrap();
        assert_eq!(widths[st][1], ARROW_MARKER_PX);
        let svg = to_svg(&sc);
        assert!(
            svg.contains("marker-end=\"url(#arrow-w"),
            "narrow marker used"
        );
        assert!(
            svg.contains("marker-end=\"url(#arrow)\""),
            "full marker kept"
        );
        // Never narrower than a head can be and still cover the stroke
        // where it stops.
        for (i, e) in sc.edges.iter().enumerate() {
            let least =
                HEAD_MIN_WIDTH.max(edge_stroke_width(e) * ARROW_MARKER_PX / arrow_trims(e).1);
            assert!(widths[i][1] >= least.min(ARROW_MARKER_PX) - 1e-9);
        }
    }

    #[test]
    fn a_head_faces_the_end_tangent_where_the_final_straight_is_shorter_than_the_cut() {
        // B1 review #1: a spline whose closing straight (a sixth of its
        // last waypoint leg, here 16 / 6 px) is shorter than the cut under
        // the head. Cutting the drawn curve by arc length used to end the
        // stroke on the bend before it, so `orient="auto"` tilted the head
        // ~15° off the side (subgraph.mmd routed, C1/C2 -> Store). The
        // squared end runs along the end tangent past the cut: the head is
        // square, its tip on the end.
        let wps = vec![(0.0, 0.0), (60.0, 0.0), (100.0, 0.0), (100.0, 16.0)];
        let e = routed_edge((0.0, 0.0), (100.0, 16.0), wps.clone());
        let (tip, head) = edge_end_tangent(&e, false);
        assert_eq!(tip, (100.0, 16.0));
        assert!(
            head.0.abs() < 1e-9 && (head.1 - 1.0).abs() < 1e-9,
            "{head:?}"
        );
        let (start, segs) = drawn_segs(&e, |p| p);
        let (_, cut) = trim_segs(start, segs, 0.0, ARROW_TRIM);
        let end = cut.last().unwrap().end();
        assert!((end.0 - 100.0).abs() < 1e-9 && (16.0 - ARROW_TRIM - end.1).abs() < 1e-6);
        // The scene's waypoints are untouched: a host draws them.
        assert_eq!(e.waypoints, wps);
        // The plain cut, for the record: its stroke heads off square.
        let (s0, g0) = plain_segs(&e, |p| p);
        let (_, plain_cut) = trim_segs(s0, g0, 0.0, ARROW_TRIM);
        let Some(&Seg::C(_, c2, p)) = plain_cut.last() else {
            panic!("{plain_cut:?}")
        };
        let tilt = (p.0 - c2.0).atan2(p.1 - c2.1).to_degrees().abs();
        assert!(tilt > 5.0, "{tilt}");
    }

    /// Every flowchart and state fixture's scene, by file name.
    fn flow_fixture_scenes() -> Vec<(String, Scene)> {
        let root = env!("CARGO_MANIFEST_DIR");
        let mut files: Vec<std::path::PathBuf> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        files
            .iter()
            .filter_map(|f| {
                let src = std::fs::read_to_string(f).unwrap();
                match crate::parser::parse_document(&src) {
                    Ok(crate::model::Document::Flowchart(g) | crate::model::Document::State(g)) => {
                        Some((f.file_name()?.to_string_lossy().into_owned(), scene(&g)))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    fn squared_count() -> usize {
        SQUARED.with(|c| c.get())
    }

    /// B1 review (r3 #1): a paint squares each glyph end once — the
    /// casing's crossings, its masks and the paths share one
    /// [`PaintedEdge`] per edge (it squared every end three times, the
    /// SVG of a 200-node chart taking ten times as long).
    #[test]
    fn a_paint_squares_each_glyph_end_once() {
        let mut checked = 0;
        for (name, sc) in flow_fixture_scenes() {
            let ends: usize = sc
                .edges
                .iter()
                .map(|e| squared_ends(e).iter().filter(|&&b| b).count())
                .sum();
            let before = squared_count();
            let svg = to_svg(&sc);
            assert_eq!(squared_count() - before, ends, "{name}");
            checked += usize::from(svg.contains("<mask") && ends > 0);
        }
        assert!(checked >= 3, "fixtures with casing masks: {checked}");
    }

    /// The pick distance as squaring both ends every time measures it:
    /// the reference [`edge_distance`]'s shortcuts must agree with.
    fn edge_distance_squaring_always<T: PickTarget>(e: &SceneEdge, target: T, reach: f64) -> f64 {
        let (start, segs) = drawn_segs(e, |p| p);
        let mut best = f64::INFINITY;
        let mut cur = start;
        for seg in segs {
            let lim = best.min(reach);
            let d = cubic_dist(target, &seg.cubic(cur), lim, 0);
            best = best.min(d);
            cur = seg.end();
        }
        best
    }

    /// B1 review (r3 #2): picking squares an end only where squaring can
    /// change the answer — a marquee over the whole canvas squares none,
    /// nor does a pointer far from every tip — and gives exactly what
    /// measuring the squared curve every time gives: the same edges in
    /// reach, at the same distance (to the flattening's 0.05 px).
    #[test]
    fn picking_squares_ends_only_where_it_can_change_the_answer() {
        let (mut probes, mut squarings, mut near_tip) = (0usize, 0usize, 0usize);
        // Very wide strokes too: longer heads, longer squared ends.
        let wide = "flowchart LR\nA[Source] --> B[Target one]\nA --> C[Target two]\n\
                    A --> D[Target three]\nE[Other] <--> D\nA e1@--> E\n\
                    e1@{ curve: linear }\nlinkStyle 0 stroke-width:14px\n\
                    linkStyle 1 stroke-width:24px\nlinkStyle 2 stroke-width:9px\n\
                    linkStyle 3,4 stroke-width:12px\n";
        let wide = ["LR", "TD"].map(|d| {
            (
                format!("wide {d}"),
                scene(&parse(&wide.replace("LR", d)).unwrap()),
            )
        });
        for (name, sc) in flow_fixture_scenes().into_iter().chain(wide) {
            let before = squared_count();
            let all = sc.hits_in_rect(-50.0, -50.0, sc.width + 50.0, sc.height + 50.0);
            assert_eq!(squared_count(), before, "{name}: whole-canvas marquee");
            let drawn = sc.edges.iter().filter(|e| edge_drawn(e)).count();
            let edges = all.iter().filter(|h| matches!(h, Hit::Edge(_))).count();
            assert_eq!(edges, drawn, "{name}");
            // A pointer grid, and small marquees, against the reference.
            let step = (sc.width * sc.height / 1500.0).sqrt().max(6.0);
            let mut y = -10.0;
            while y < sc.height + 10.0 {
                let mut x = -10.0;
                while x < sc.width + 10.0 {
                    for e in sc.edges.iter().filter(|e| edge_drawn(e)) {
                        let half = edge_stroke_width(e) / 2.0;
                        let reach = 4.0 + half;
                        let before = squared_count();
                        let got = edge_distance(e, (x, y), reach);
                        let tips = {
                            let (st, sg) = plain_segs(e, |p| p);
                            [st, sg.last().map_or(st, |g| g.end())]
                        };
                        let tip_gap = tips
                            .iter()
                            .map(|t| (t.0 - x).hypot(t.1 - y))
                            .fold(f64::INFINITY, f64::min);
                        let squared = squared_count() - before;
                        let far = tip_gap > square_end_reach(e).0 + reach;
                        if far {
                            assert_eq!(squared, 0, "{name}: far from both tips");
                        } else if !name.starts_with("wide") {
                            // (The wide strokes' fat pick bands sit on their
                            // curves near every tip: counted for equality only.)
                            near_tip += 1;
                            squarings += usize::from(squared > 0);
                        }
                        probes += 1;
                        if far && got > reach + 1.0 {
                            continue; // the scene curve settles it (nothing squared)
                        }
                        // Equal up to the flattening (a squared end's
                        // kept part is a piece of the scene's cubic, flattened
                        // into other chords).
                        let want = edge_distance_squaring_always(e, (x, y), reach);
                        let tol = 0.06;
                        if want <= reach - tol {
                            assert!((got - want).abs() <= tol, "{name}: {x},{y} {got} vs {want}");
                        } else if want > reach + tol {
                            assert!(got > reach, "{name}: {x},{y} {got} vs {want}");
                        }
                        let r = PickRect {
                            x0: x,
                            y0: y,
                            x1: x + step / 3.0,
                            y1: y + step / 5.0,
                        };
                        let (got, want) = (
                            edge_distance(e, r, half),
                            edge_distance_squaring_always(e, r, half),
                        );
                        if want <= half - tol || want > half + tol {
                            assert_eq!(got <= half, want <= half, "{name}: rect at {x},{y}");
                        }
                    }
                    x += step;
                }
                y += step;
            }
        }
        assert!(
            probes > 100_000 && near_tip > 5_000,
            "{probes} probes, {near_tip} near a tip"
        );
        // Squaring is the exception: well under a tenth of the probes
        // near a tip (each on an edge whose curve passes near the
        // pointer there).
        assert!(
            squarings * 10 < near_tip,
            "{squarings} squarings, {near_tip} near a tip"
        );
    }

    #[test]
    fn squared_ends_face_the_curve_and_keep_near_it_on_every_fixture() {
        // Report §2.3 / M1, B1 review #1 and #2: on every flowchart and
        // state fixture, static and routed, each end glyph faces along its
        // scene curve's own end tangent and an arrowhead's stroke stops
        // exactly its cut back on that tangent (the tip on the end, never
        // tilted by a cut into a curve still turning); outside the head the
        // painted curve keeps within SQUARE_END_DEV of the scene curve the
        // routing, port and label stages spaced, beyond two heads of the
        // tip — and within half a head nearer, where a curve swinging in
        // from the side enters the head.
        let root = env!("CARGO_MANIFEST_DIR");
        let mut files: Vec<std::path::PathBuf> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1) < 1e-6;
        // A curve straight to within SQUARE_END_FLAT over a head is drawn
        // as it is: its heading at the cut within half a degree.
        let along =
            |a: (f64, f64), b: (f64, f64)| a.0 * b.0 + a.1 * b.1 > 0.5f64.to_radians().cos();
        // The cut is found by arc length over a sampled curve, on a curve
        // that may lie SQUARE_END_FLAT off its tangent (the SVG writes
        // tenths).
        let cut_at =
            |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1) < SQUARE_END_FLAT + 0.01;
        let (mut ends, mut squared) = (0, 0);
        for f in &files {
            use crate::model::Document;
            let src = std::fs::read_to_string(f).unwrap();
            let g = match crate::parser::parse_document(&src) {
                Ok(Document::Flowchart(g) | Document::State(g)) => g,
                _ => continue,
            };
            let st = scene(&g);
            let centres: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = route(&g, &centres);
            for sc in [&st, &rt] {
                let mode = if std::ptr::eq(sc, &st) {
                    "static"
                } else {
                    "routed"
                };
                for e in sc.edges.iter().filter(|e| edge_drawn(e) && e.from != e.to) {
                    let (s0, g0) = plain_segs(e, |p| p);
                    let (s1, g1) = drawn_segs(e, |p| p);
                    let trims = arrow_trims(e);
                    let (plain, painted) = (dense_path(s0, &g0, 48), dense_path(s1, &g1, 48));
                    for (at_start, m, trim) in [
                        (true, e.ends.start_marker(e.kind), trims.0),
                        (false, e.ends.end_marker(e.kind), trims.1),
                    ] {
                        let Some(m) = m else { continue };
                        ends += 1;
                        let (rs, rg) = reverse_segs(s0, &g0);
                        let (from, last) = if at_start {
                            let n = rg.len();
                            (if n > 1 { rg[n - 2].end() } else { rs }, rg[n - 1])
                        } else {
                            let n = g0.len();
                            (if n > 1 { g0[n - 2].end() } else { s0 }, g0[n - 1])
                        };
                        let Some(d) = end_heading(from, last) else {
                            continue;
                        };
                        let (tip, head) = edge_end_tangent(e, at_start);
                        let what = format!("{f:?} {mode} {}->{} start={at_start}", e.from, e.to);
                        assert!(along(head, d), "{what}: {head:?} vs {d:?}");
                        if trim > 0.0 {
                            let (s, g) = drawn_segs(e, |p| p);
                            let (s, g) = trim_segs(s, g, trims.0, trims.1);
                            let end = if at_start { s } else { g.last().unwrap().end() };
                            let want = (tip.0 - d.0 * trim, tip.1 - d.1 * trim);
                            // A curve too short to lose the cut is stroked whole.
                            if !near(end, tip) {
                                assert!(cut_at(end, want), "{what}: {end:?} vs {want:?}");
                            }
                        }
                        let hw = (m == Marker::Arrow).then(|| edge_stroke_width(e) / 2.0);
                        let (far, close) = end_stray(&painted, &plain, tip, d, hw, SQUARE_END_ARC);
                        squared += usize::from(far + close > 0.0);
                        assert!(far <= SQUARE_END_DEV + 0.1, "{what}: {far}");
                        assert!(close <= ARROW_MARKER_PX / 2.0, "{what}: {close}");
                    }
                }
            }
        }
        assert!(
            ends > 500 && squared > 100,
            "{ends} ends, {squared} squared"
        );
    }

    #[test]
    fn a_wide_stroke_stops_its_width_short_under_its_own_head() {
        // B1 review #4: a `linkStyle` stroke wider than the usual cut
        // showed its butt end's shoulders beside the head. Its stroke stops
        // its own width short (a head is as wide as it is long), its head
        // set that much further ahead so the tip stays on the border — and
        // (B1 rescue C) as long as the cut, the corner onto the run past it
        // and half the stroke need: 8 + 2 + 4 = 14 px, in a variant (B1 r5:
        // the run past a wide cut is a quarter of it, square_end_entry, so
        // the SVG's rounding keeps the tip on the border; 13.5 px before).
        let src = "flowchart LR\nA --> B\nC --> D\nlinkStyle 1 stroke-width:8px\n";
        let sc = scene(&parse(src).unwrap());
        assert_eq!(arrow_trims(&sc.edges[0]), (0.0, ARROW_TRIM));
        assert_eq!(arrow_trims(&sc.edges[1]), (0.0, 8.0));
        let svg = to_svg(&sc);
        let paths = edge_paths(&svg);
        assert!(
            paths[0].contains("marker-end=\"url(#arrow)\""),
            "{}",
            paths[0]
        );
        assert!(
            paths[1].contains("marker-end=\"url(#arrow-t8-s14)\""),
            "{}",
            paths[1]
        );
        assert!(svg.contains(
            "<marker id=\"arrow-t8-s14\" viewBox=\"0 0 10 10\" refX=\"4.286\" refY=\"5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"14\" markerHeight=\"14\""
        ));
        let mut bb = Bbox::new();
        grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, _, _, _) = bb.finish();
        let d = &sc.nodes[3];
        let left = d.x - d.w / 2.0 + MARGIN - minx;
        let pts = painted_points(paths[1]);
        let end = pts[pts.len() - 1];
        assert!((left - end.0 - 8.0).abs() < 0.11, "{end:?} vs {left}");
        // Its own width however wide, under a head grown to hold it; a
        // hairline and the thick edge keep the usual cut and head.
        assert_eq!(arrow_trim(30.0), 30.0);
        assert_eq!(arrow_head_len(arrow_trim(30.0)), 30.0 + 7.5 + 15.0);
        assert_eq!(square_end_entry(30.0), 7.5);
        assert_eq!(arrow_trim(1.0), ARROW_TRIM);
        for w in [1.0, EDGE_WIDTH, EDGE_WIDTH_THICK, ARROW_TRIM, 6.0] {
            assert_eq!(arrow_head_len(arrow_trim(w)), ARROW_MARKER_PX, "{w}");
        }
    }

    /// B1 review (rescue C): a 14 px `linkStyle` stroke ended in a butt
    /// wider than its 11 px head, notched where its slanted curve met a
    /// 1.5 px run. Under a head grown to hold it ([`arrow_head_len`]) the
    /// stroke is cut its own width back, where the head is as wide as it,
    /// and runs straight along the head's axis past the cut (a straight
    /// stretch the stroke's corner and join hide behind, inside the head)
    /// — through the whole head where its curve arrives straight. In
    /// either direction, at either end.
    #[test]
    fn a_very_wide_stroke_ends_inside_its_grown_head() {
        let src = "flowchart LR\nA[Source] --> B[Target one]\nA --> C[Target two]\n\
                   A --> D[Target three]\nE[Other] <--> D\nlinkStyle 0 stroke-width:14px\n\
                   linkStyle 1 stroke-width:24px\nlinkStyle 2 stroke-width:14px\n\
                   linkStyle 3 stroke-width:10px\n";
        for dir in ["LR", "TD"] {
            let g = parse(&src.replace("LR", dir)).unwrap();
            let sc = scene(&g);
            let svg = to_svg(&sc);
            let paths = edge_paths(&svg);
            assert_eq!(paths.len(), 4);
            for (k, (e, p)) in sc.edges.iter().zip(&paths).enumerate() {
                let w = edge_stroke_width(e);
                let (trim, len) = (arrow_trim(w), edge_head_len(e));
                assert!(
                    trim >= w && len >= trim + square_end_entry(trim) + w / 2.0,
                    "{w}"
                );
                assert!(
                    p.contains(&format!("-s{}", format!("{len}").replace('.', "_"))),
                    "{p}"
                );
                let pts = painted_points(p);
                let mut ends = vec![pts.iter().rev().copied().collect::<Vec<_>>()];
                if e.ends.start_marker(e.kind).is_some() {
                    ends.push(pts.clone());
                }
                for back in ends {
                    let end = back[0];
                    let q = *back[1..]
                        .iter()
                        .find(|q| (q.0 - end.0).hypot(q.1 - end.1) > 1e-6)
                        .unwrap();
                    let l = (end.0 - q.0).hypot(end.1 - q.1);
                    let d = ((end.0 - q.0) / l, (end.1 - q.1) / l);
                    let tip = (end.0 + d.0 * trim, end.1 + d.1 * trim);
                    let along = |r: (f64, f64)| (tip.0 - r.0) * d.0 + (tip.1 - r.1) * d.1;
                    let lat = |r: (f64, f64)| ((r.0 - tip.0) * d.1 - (r.1 - tip.1) * d.0).abs();
                    // How far back from the tip the stroke runs on the axis
                    // (the SVG's 0.1 px rounding aside).
                    let run = back
                        .iter()
                        .take_while(|&&r| lat(r) <= 0.15)
                        .map(|&r| along(r))
                        .fold(trim, f64::max);
                    assert!(
                        run >= trim + square_end_entry(trim) - 0.15,
                        "{dir} {w}px: a run {:.2} past the cut\n{p}",
                        run - trim
                    );
                    // The 24 px stroke arrives straight: it fills its head.
                    if k == 1 {
                        assert!(run >= len - 0.15, "{dir}: {run} of {len}\n{p}");
                    }
                }
            }
        }
    }

    #[test]
    fn class_and_er_edges_keep_their_painters_geometry() {
        // Their painters draw their own ends and stroke edge_path_d
        // whole: no glyph stub, no arrow trim, from the first stage on.
        let cs = crate::class::scene(&match crate::parser::parse_document(
            "classDiagram\nA --> B\nB <|-- C\nC ..> D\n",
        )
        .unwrap()
        {
            crate::model::Document::Class(d) => d,
            _ => unreachable!(),
        });
        let es = crate::er::scene(&match crate::parser::parse_document(
            "erDiagram\nA ||--o{ B : has\nB }|..|{ C : uses\n",
        )
        .unwrap()
        {
            crate::model::Document::Er(d) => d,
            _ => unreachable!(),
        });
        for e in cs.scene.edges.iter().chain(&es.scene.edges) {
            assert!(e.paint_width.is_some());
            let (start, segs) = plain_segs(e, |p| p);
            assert_eq!(square_ends(e, start, segs).2, [None, None]);
            assert_eq!(arrow_trims(e), (0.0, 0.0));
            assert_eq!(edge_path_d(e, |p| p), edge_path_d_painted(e, |p| p));
        }
    }

    /// Flowcharts whose routes reach past every node: the B1 review's long
    /// detours round a whole frame (`x3_long`) and random graphs whose
    /// routed or dragged edges ran off the canvas (`r31`, `r56`).
    const CANVAS_PROBES: [&str; 3] = [
        "flowchart TD\n  subgraph F1 [Frame one]\n    a1[Alpha] --> a2[Beta]\n    \
         a2 --> a3[Gamma]\n  end\n  subgraph F2 [Frame two]\n    b1[Delta] --> b2[Epsilon]\n  \
         end\n  start([Start]) --> a1\n  start --> b1\n  start ==> z[(Store)]\n  a3 --> z\n  \
         b2 --> F1\n  start --> a3\n  z --> start\n  a1 --> a1\n  b1 --o b2\n  a2 --x z\n  \
         F2 --> z\n  q{{Hex}} --> start\n  a3 --> q\n",
        "flowchart TD\n  subgraph F0 [Frame 0]\n    n6[Node 6]\n    n4[Node 4]\n  end\n  \
         subgraph F1 [Frame 1]\n    n1[Node 1]\n  end\n  n1 --> n2[Node 2]\n  \
         n8[Node 8] -.-> n10[Node 10]\n  n7[Node 7] --> n3[Node 3]\n  n8 --> n0[Node 0]\n  \
         n7 --> n9[Node 9]\n  n10 --> n6\n  n0 ==> n7\n  n6 --> n10\n  n2 --> n1\n  \
         n1 ==> n9\n  n1 --o n8\n  n6 --> n3\n  n9 ==> n7\n  n0 -.-> n2\n  n4 --> n0\n",
        "flowchart BT\n  subgraph F0 [Frame 0]\n    n12[Node 12]\n    n5[Node 5]\n  end\n  \
         subgraph F1 [Frame 1]\n    n3[Node 3]\n    n2[Node 2]\n  end\n  n2 --o n11[Node 11]\n  \
         n7[Node 7] ==> n6[Node 6]\n  n2 --> n1[Node 1]\n  n2 -.-> n0[Node 0]\n  \
         n0 -.-> n8[Node 8]\n  n6 --> n12\n  n9[Node 9] ==> n12\n  n1 ==> n4[Node 4]\n  \
         n0 --o n10[Node 10]\n  n2 -.-> n8\n  n2 --> n4\n  n0 --> n3\n  n1 -.-> n2\n  \
         n2 -.-> n5\n  n9 -.-> n1\n  n1 --> n9\n  n9 --> n12\n  n10 --> n11\n  \
         n7 --> n11\n  n5 --o n10\n  n2 ==> n12\n  n6 -.-> n0\n  n1 -.-> n8\n  \
         n12 ==> n0\n  n8 --> n6\n",
    ];

    /// A seeded random flowchart: 7-14 nodes, a few frames, every edge
    /// kind and end glyph, any direction, now and then a wide or dashed
    /// `linkStyle` — the shapes the canvas test sweeps.
    fn random_flowchart(seed: u64) -> String {
        let mut st = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mut next = |m: u64| {
            st = st
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (st >> 33) % m
        };
        let dir = ["TD", "LR", "BT", "RL"][next(4) as usize];
        let n = 7 + next(8) as usize;
        let mut s = format!("flowchart {dir}\n");
        let frames = next(3) as usize;
        for f in 0..frames {
            s.push_str(&format!("  subgraph F{f} [Frame {f}]\n"));
            for k in 0..2 {
                s.push_str(&format!("    n{}[Node {}]\n", f * 3 + k, f * 3 + k));
            }
            s.push_str("  end\n");
        }
        let ops = ["-->", "==>", "-.->", "--o", "--x", "<-->", "---"];
        let edges = n + next(n as u64) as usize;
        for _ in 0..edges {
            let (a, b) = (next(n as u64), next(n as u64));
            let op = ops[next(ops.len() as u64) as usize];
            s.push_str(&format!("  n{a}[Node {a}] {op} n{b}[Node {b}]\n"));
        }
        if next(2) == 0 {
            s.push_str(&format!(
                "  linkStyle {} stroke-width:{}px\n",
                next(edges as u64),
                2 + 3 * next(5)
            ));
        }
        s
    }

    /// Report §2.3 B1 review: nothing drawn falls off the canvas. For
    /// every flowchart and state fixture, the long-detour probes and
    /// seeded random graphs, in the static scene, `route` fed its
    /// centres, and drag frames (`route` and `route_partial` with a node
    /// moved), every painted edge path to its exact extremes (half its
    /// stroke out) and every end marker's box lies inside the SVG's
    /// view box; and the scene's own canvas (`width` × `height` from
    /// the origin) holds the drawing with its gutter, a static one
    /// starting [`MARGIN`] in exactly as the SVG does.
    #[test]
    fn nothing_painted_falls_off_the_canvas() {
        let root = env!("CARGO_MANIFEST_DIR");
        let mut srcs: Vec<(String, String)> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .map(|p| {
                (
                    p.display().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                )
            })
            .collect();
        srcs.sort();
        for (k, src) in CANVAS_PROBES.iter().enumerate() {
            srcs.push((format!("probe {k}"), src.to_string()));
        }
        for seed in 0..12 {
            srcs.push((format!("random {seed}"), random_flowchart(seed)));
        }
        let mut bad = Vec::new();
        let mut checked = 0;
        for (name, src) in &srcs {
            use crate::model::Document;
            let g = match crate::parser::parse_document(src) {
                Ok(Document::Flowchart(g) | Document::State(g)) => g,
                _ => continue,
            };
            let st = scene(&g);
            let c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            let mut frames = vec![("static", st.clone()), ("routed", route(&g, &c))];
            let n = c.len();
            for (v, d) in [(0, (-40.0, -40.0)), (n / 2, (60.0, 60.0))] {
                let mut moved = c.clone();
                moved[v] = (moved[v].0 + d.0, moved[v].1 + d.1);
                frames.push(("drag", route(&g, &moved)));
                frames.push(("partial", route_partial(&g, &moved, &st, &c)));
            }
            for (mode, sc) in &frames {
                checked += 1;
                for off in ink_off_canvas(&to_svg(sc)) {
                    bad.push(format!("{name} {mode}: {off}"));
                }
                let mut bb = Bbox::new();
                grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
                let (minx, maxx, miny, maxy) = bb.finish();
                if maxx + MARGIN > sc.width + 1e-6 || maxy + MARGIN > sc.height + 1e-6 {
                    bad.push(format!(
                        "{name} {mode}: canvas {}x{} short",
                        sc.width, sc.height
                    ));
                }
                if *mode == "static" && (minx < MARGIN - 1e-6 || miny < MARGIN - 1e-6) {
                    bad.push(format!(
                        "{name} {mode}: drawing in the gutter at {minx},{miny}"
                    ));
                }
            }
        }
        assert!(checked >= 150, "{checked} scenes");
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// Report Q2 + P1: a static flowchart's canvas is its drawing — the
    /// curves to their exact extremes, never their control points — and
    /// [`MARGIN`] on every side, in the scene (`width` × `height`, node
    /// centres) exactly as in the SVG. A back-edge bow's handles used to
    /// pad one side (demo 65 px right of the bow against 27 px left), and
    /// an LR label's width was laid across the flow, so `scene()` of
    /// wide-labels was 624 px tall for a 413 px SVG, every node 119 px
    /// lower than drawn. `route`'s canvas reaches [`MARGIN`] past its
    /// drawing alike.
    #[test]
    fn a_static_canvas_is_its_drawing_with_even_margins() {
        let root = env!("CARGO_MANIFEST_DIR");
        let fixture = |name: &str| {
            std::fs::read_to_string(format!("{root}/tests/fixtures/quality/{name}.mmd")).unwrap()
        };
        let example =
            |name: &str| std::fs::read_to_string(format!("{root}/examples/{name}.mmd")).unwrap();
        let mut srcs: Vec<(String, String)> = ["back-edge", "wide-labels", "dense-td", "bench-75"]
            .iter()
            .map(|n| (n.to_string(), fixture(n)))
            .chain(
                ["demo", "stress", "lr", "state", "advanced"]
                    .iter()
                    .map(|n| (n.to_string(), example(n))),
            )
            .collect();
        for (k, src) in CANVAS_PROBES.iter().enumerate() {
            srcs.push((format!("probe {k}"), src.to_string()));
        }
        for dir in ["LR", "RL", "TD", "BT"] {
            srcs.push((
                format!("bow {dir}"),
                format!("flowchart {dir}\nA --> B --> C --> D\nD -->|a rather long label on the way back| A"),
            ));
        }
        let mut sizes = HashMap::new();
        for (name, src) in &srcs {
            use crate::model::Document;
            let g = match crate::parser::parse_document(src) {
                Ok(Document::Flowchart(g) | Document::State(g)) => g,
                other => panic!("{name}: {:?}", other.err()),
            };
            let sc = scene(&g);
            let mut bb = Bbox::new();
            grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
            let (minx, maxx, miny, maxy) = bb.finish();
            for (what, got) in [
                ("left", minx),
                ("top", miny),
                ("right", sc.width - maxx),
                ("bottom", sc.height - maxy),
            ] {
                assert!((got - MARGIN).abs() < 1e-6, "{name}: {what} margin {got}");
            }
            // The SVG draws the scene where it stands, on its canvas.
            let svg = to_svg(&sc);
            let vb = format!("viewBox=\"0 0 {:.0} {:.0}\"", sc.width, sc.height);
            assert!(svg.contains(&vb), "{name}: {vb} not in the SVG");
            sizes.insert(name.clone(), (sc.width, sc.height));
            // `route` keeps the host's centres; its canvas reaches
            // MARGIN past the drawing too, not past control points.
            let centres: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &centres);
            let mut bb = Bbox::new();
            grow_canvas(&mut bb, &r.nodes, &r.edges, &r.clusters);
            let (_, maxx, _, maxy) = bb.finish();
            assert!(
                (r.width - maxx - MARGIN).abs() < 1e-6,
                "{name}: routed width {}",
                r.width
            );
            assert!(
                (r.height - maxy - MARGIN).abs() < 1e-6,
                "{name}: routed height {}",
                r.height
            );
        }
        // The report's yardsticks: B3's lanes run beside what lies between
        // a back-edge's ends, not round its ends' own boxes (demo 383 ->
        // 321 wide in the report), and a wide LR label no longer spreads
        // its back-edge across the flow (wide-labels 413 -> 201 tall at
        // most). Stress grows instead (221 -> 270): its loop keeps its
        // room from the thick edge's channel — what the loop reaches past
        // the node's half of the gap, not the whole reach on top of it
        // (293 wide) — and the dashed back-edge takes the side that
        // crosses nothing (report P0 #13).
        assert!(
            sizes["back-edge"].0 < 300.0,
            "back-edge {:?}",
            sizes["back-edge"]
        );
        assert!(sizes["stress"].0 < 275.0, "stress {:?}", sizes["stress"]);
        assert!(sizes["demo"].0 <= 321.5, "demo {:?}", sizes["demo"]);
        assert!(
            sizes["wide-labels"].1 <= 201.0,
            "wide-labels {:?}",
            sizes["wide-labels"]
        );
        // The layout's own canvas lays an LR label along the flow: a
        // straight labelled edge is no taller than its nodes and a chip
        // beside them (the label's 300 px width made it 350 px tall).
        let g = parse("flowchart LR\nA -->|a very very long label for one short edge| B").unwrap();
        let (labels, sizes) = crate::layout::node_texts(&g);
        let sc = scene_flat(&g, &sizes, &labels);
        let tallest = sc.nodes.iter().map(|n| n.h).fold(0.0, f64::max);
        assert!(
            sc.height <= tallest + LABEL_BOX_H + 2.0 * MARGIN,
            "LR layout canvas {} tall for {tallest} px of drawing",
            sc.height
        );
    }

    /// Each arrowhead `to_svg` paints for `sc`, read off the SVG's own
    /// numbers as a browser places it: (edge index, at the source?, the
    /// tip, the stroke's painted end, the unit way the head faces, the
    /// scene curve's end point) — all in SVG coordinates. The head faces
    /// along the painted path's end (the nearest coordinate apart from
    /// it, as `orient="auto"` reads a command's tangent) and has its tip
    /// `refX` (in `markerWidth` px) on along that line.
    #[allow(clippy::type_complexity)]
    fn painted_heads(
        sc: &Scene,
    ) -> Vec<(usize, bool, (f64, f64), (f64, f64), (f64, f64), (f64, f64))> {
        let svg = to_svg(sc);
        let attr = |el: &str, name: &str| -> Option<String> {
            let rest = el.split(&format!(" {name}=\"")).nth(1)?;
            Some(rest.split('"').next()?.to_string())
        };
        let nums = |v: &str| -> Vec<f64> {
            v.split([' ', ','])
                .filter_map(|t| t.parse::<f64>().ok())
                .collect()
        };
        let mut bb = Bbox::new();
        grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, _, miny, _) = bb.finish();
        let t = |p: (f64, f64)| (p.0 + MARGIN - minx, p.1 + MARGIN - miny);
        let drawn: Vec<usize> = (0..sc.edges.len())
            .filter(|&i| edge_drawn(&sc.edges[i]))
            .collect();
        let paths = edge_paths(&svg);
        assert_eq!(paths.len(), drawn.len());
        let mut out = Vec::new();
        for (&i, p) in drawn.iter().zip(&paths) {
            let e = &sc.edges[i];
            let (s0, g0) = plain_segs(e, |q| q);
            let ends = [s0, g0.last().map_or(s0, |g| g.end())];
            let pts: Vec<(f64, f64)> = nums(&attr(p, "d").unwrap())
                .chunks(2)
                .map(|q| (q[0], q[1]))
                .collect();
            for (key, at_start) in [("marker-start", true), ("marker-end", false)] {
                let Some(url) = attr(p, key) else { continue };
                let id = url.trim_start_matches("url(#").trim_end_matches(')');
                if !id.starts_with("arrow") {
                    continue;
                }
                let at = svg.find(&format!("<marker id=\"{id}\"")).unwrap();
                let def = &svg[at..at + svg[at..].find('>').unwrap()];
                let refx: f64 = attr(def, "refX").unwrap().parse().unwrap();
                let px: f64 = attr(def, "markerWidth").unwrap().parse().unwrap();
                let seq: Vec<(f64, f64)> = if at_start {
                    pts.clone()
                } else {
                    pts.iter().rev().copied().collect()
                };
                let end = seq[0];
                let q = *seq[1..]
                    .iter()
                    .find(|q| (q.0 - end.0).hypot(q.1 - end.1) > 1e-9)
                    .expect("a heading");
                let l = (end.0 - q.0).hypot(end.1 - q.1);
                // The way the head faces: into the end, out of the start.
                let d = ((end.0 - q.0) / l, (end.1 - q.1) / l);
                let off = if at_start { refx } else { 10.0 - refx } * px / 10.0;
                let tip = (end.0 + d.0 * off, end.1 + d.1 * off);
                let want = t(ends[usize::from(!at_start)]);
                out.push((i, at_start, tip, end, d, want));
            }
        }
        out
    }

    /// B1 review r5 (blocking): under a stroke about 15 px wide or wider
    /// the grown head is cut its stroke's width back, and where the end
    /// gave it no straight run that long (a back-edge bow, a routed turn
    /// just short of the end, an S-cubic) the head faced along the
    /// curve's tangent at the cut, its tip up to 19 px off the end (y2's
    /// c -> a, q53's routed n2 -> n4) and 37 px mid-drag, beside the node
    /// instead of on it — and the head's ink lay where picking does not
    /// reach. The head now turns with such a curve (the stroke runs
    /// straight into the end along the chord from where the curve first
    /// lies a run's length away), and a curve too short or too curled
    /// even for that is cut no deeper than its straight run: the tip is on
    /// the scene curve's end, static, routed and mid-drag, for 16, 24
    /// and 40 px strokes (a self-loop's too, which is squared under so
    /// wide a stroke), and the stroke picking walks runs along the
    /// head's axis from the cut to the tip.
    #[test]
    fn a_very_wide_strokes_head_keeps_its_tip_on_the_end_point() {
        let inputs = [
            // A back-edge bow (m1_back24).
            "flowchart TD\n  a[Alpha] --> b[Beta]\n  b --> c[Gamma]\n  c --> a\n  \
                 linkStyle 2 stroke-width:{w}px,stroke:#d55\n",
            // A routed turn just short of the end: the bow back into the
            // frame turns up into Alpha 14.5 px below it (y2_widetd).
            "flowchart TD\n  subgraph S [Frame]\n    a[Alpha] --> b[Beta]\n    \
                 b --> c[Gamma]\n  end\n  c --> a\n  c -->|back| top[Top node]\n  \
                 top --> a\n  top e2@--> c\n  e2@{ curve: step }\n  b --> b\n  \
                 linkStyle 2 stroke-width:{w}px,stroke:#d55\n  linkStyle 3 stroke-width:18px\n  \
                 linkStyle 5 stroke-width:12px,stroke:#55d\n  linkStyle 6 stroke-width:10px\n",
            // A short two-headed S-cubic (q04's n0 <==> n5).
            "flowchart RL\n  subgraph F0 [Frame 0]\n    n2[Node 2]\n  end\n  \
                 n4 -->|lbl| F0\n  n0 -.-> n1\n  n1 <--> n0\n  n3 ==> F0\n  n3 x--x F0\n  \
                 n0 -.->|dot| n1\n  n5 o--o n3\n  n0 <==> n5\n  \
                 linkStyle 7 stroke:#c33,stroke-width:{w}px\n  \
                 linkStyle 6 stroke:#3a3,stroke-width:24px\n  linkStyle 3 stroke-width:9px\n",
            // A routed lane that turns up into n4 just short of it (q53's
            // routed n2 -> n4).
            "flowchart BT\n  subgraph F0 [Frame 0]\n    n5((Node 5))\n  end\n  \
             subgraph F1 [Frame 1]\n    n2{{Node 2 with a longer label}}\n    \
             n3(Node 3 with a longer label)\n    n6[(Node 6)]\n  end\n  n0 <-.-> n2\n  \
             n7 <==> n5\n  n4 -.-> n8\n  n5 --o n9\n  n4 <-.-> n3\n  n5 -->|lbl| F0\n  \
             n2 -.->|dot| n4\n  n2 --- n3\n  n7 x--x n4\n  n0 <-.-> n9\n  n5 -->|lbl| n6\n  \
             n0 <==> F1\n  n6 -.->|dot| n1\n  n6 o--o n7\n  n2 --- n3\n  n2 --- n0\n  \
             n3 --o n4\n  n5 <-.-> n6\n  n7 --x n8\n  n9 <--> n5\n  n0 ==> F0\n  \
             n8 -->|lbl| n9\n  linkStyle 6 stroke:#c33,stroke-width:{w}px\n  \
             linkStyle 0 stroke:#c33,stroke-width:3px\n  linkStyle 19 stroke-width:3px\n",
            // Self-loops smaller than their strokes (q79's n4 <==> n4).
            "flowchart TD\n  A[Loop] --> A\n  B[Two] <--> B\n  A --> B\n  \
             linkStyle 0 stroke-width:{w}px\n  linkStyle 1 stroke-width:12px\n",
            // A long S from Source up into Target two (y1_wide's A -> C).
            "flowchart LR\n  A[Source] --> B[Target one]\n  A --> C[Target two]\n  \
                 A <--> D[Target three]\n  E[Other] --o D\n  E --x B\n  \
                 linkStyle 1 stroke-width:{w}px,stroke:#a33\n  linkStyle 2 stroke-width:9px\n",
        ];
        let (mut heads, mut turned) = (0, 0);
        for src in inputs {
            for w in [16, 24, 40] {
                let g = parse(&src.replace("{w}", &w.to_string())).unwrap();
                let st = scene(&g);
                // The wide edge, and its ends' node indices.
                let k = st
                    .edges
                    .iter()
                    .position(|e| edge_stroke_width(e) == f64::from(w))
                    .unwrap();
                let node = |id: &str| st.nodes.iter().position(|n| n.id == id).unwrap();
                let (a, b) = (node(&st.edges[k].from), node(&st.edges[k].to));
                let c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut frames = vec![
                    ("static".to_string(), st.clone()),
                    ("routed".to_string(), route(&g, &c)),
                ];
                // Mid-drag: the wide edge's target set beside or below its
                // source, a few px and a gap apart.
                let (na, nb) = (&st.nodes[a], &st.nodes[b]);
                for gap in [3.0, 12.0] {
                    for below in [false, true] {
                        let mut pos = c.clone();
                        pos[b] = if below {
                            (c[a].0, c[a].1 + na.h / 2.0 + gap + nb.h / 2.0)
                        } else {
                            (c[a].0 + na.w / 2.0 + gap + nb.w / 2.0, c[a].1)
                        };
                        frames.push((format!("drag {gap} {below}"), route(&g, &pos)));
                        frames.push((
                            format!("partial {gap} {below}"),
                            route_partial(&g, &pos, &st, &c),
                        ));
                    }
                }
                for (what, sc) in &frames {
                    let mut bb = Bbox::new();
                    grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
                    let (minx, _, miny, _) = bb.finish();
                    let back = |p: (f64, f64)| (p.0 - MARGIN + minx, p.1 - MARGIN + miny);
                    for (i, at_start, tip, end, d, want) in painted_heads(sc) {
                        let e = &sc.edges[i];
                        let err = (tip.0 - want.0).hypot(tip.1 - want.1);
                        assert!(
                            err <= 0.5,
                            "{w}px {what} e{i} {}->{} {}: tip {tip:?} vs end {want:?} \
                             ({err:.2} px off)\n{src}",
                            e.from,
                            e.to,
                            if at_start { "start" } else { "end" }
                        );
                        heads += 1;
                        if i == k {
                            // Pick == render: the stroke picking walks runs
                            // along the head's axis from the cut to the tip
                            // (the SVG's 0.1 px rounding aside).
                            let len = (tip.0 - end.0).hypot(tip.1 - end.1);
                            let n = (len / 2.0).ceil().max(1.0) as usize;
                            for s in 0..=n {
                                let f = s as f64 / n as f64;
                                let q = back((
                                    end.0 + (tip.0 - end.0) * f,
                                    end.1 + (tip.1 - end.1) * f,
                                ));
                                let dist = edge_distance(e, q, 2.0);
                                assert!(dist <= 0.5, "{w}px {what} e{i}: {dist} at {q:?}");
                            }
                            // Facing off the curve's own end tangent: the
                            // head turned with the curve.
                            let (_, tan) = edge_end_tangent(e, at_start);
                            let face = if at_start { (-tan.0, -tan.1) } else { tan };
                            turned += usize::from(d.0 * face.0 + d.1 * face.1 < 0.999);
                        }
                    }
                }
            }
        }
        assert!(heads > 500, "{heads} heads");
        assert!(turned > 0, "no head turned with its curve");
    }

    /// B1 review r6: an author step curve (`step`, `stepBefore`,
    /// `stepAfter`, `linkStyle … interpolate step`) whose last leg is
    /// axis-aligned ends in a zero-length `L` ([`curve_geometry`]); its end
    /// went unsquared and its cut uncapped, so a wide stroke's cut (its
    /// width) wrapped round the step's last corner and the head pointed
    /// sideways beside the node, its tip up to 42 px off the border — in
    /// the static scene, route() and every drag frame — and picking
    /// missed that head's ink. A 2 px step whose last leg fell under the
    /// usual cut mid-drag had its tip 6 px off. Every head keeps its tip
    /// on the end point, and picking walks its axis.
    #[test]
    fn a_step_curves_head_keeps_its_tip_on_the_end_point() {
        let inputs = [
            // The r6 reproduction (min/m1): Alpha's step into Gamma.
            "flowchart TD\n  a[Alpha] --> b[Beta]\n  b --> c[Gamma]\n  a e1@--> c\n  \
             e1@{ curve: step }\n  linkStyle 2 stroke-width:{w}px\n",
            "flowchart TD\n  a[Alpha] --> b[Beta]\n  b --> c[Gamma]\n  a e1@--> c\n  \
             e1@{ curve: stepBefore }\n  linkStyle 2 stroke-width:{w}px\n",
            "flowchart LR\n  a[Alpha] --> b[Beta]\n  b --> c[Gamma]\n  a e1@<--> c\n  \
             e1@{ curve: stepAfter }\n  linkStyle 2 stroke-width:{w}px\n",
            // `interpolate step` on the link, and as everyone's default
            // (min/m3, min/m4).
            "flowchart TD\n  a[Alpha] --> b[Beta]\n  b --> c[Gamma]\n  a --> c\n  \
             linkStyle 2 stroke-width:{w}px,interpolate step\n",
            "flowchart BT\n  a[Alpha] --> b[Beta]\n  b --> c[Gamma]\n  a --> c\n  \
             c <--> d[Delta]\n  a --> d\n  linkStyle default interpolate step\n  \
             linkStyle 2 stroke-width:{w}px\n",
        ];
        let mut heads = 0;
        for src in inputs {
            for w in [2, 16, 20, 30] {
                let g = parse(&src.replace("{w}", &w.to_string())).unwrap();
                let st = scene(&g);
                let c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
                let mut frames = vec![
                    ("static".to_string(), st.clone()),
                    ("routed".to_string(), route(&g, &c)),
                ];
                // The r6 drags: each node moved by a few offsets (Alpha by
                // (-90, 60) and Beta by (40, 25) turned the head sideways).
                for k in 0..c.len() {
                    for (dx, dy) in [(-90.0, 60.0), (40.0, 25.0), (25.0, -40.0)] {
                        let mut pos = c.clone();
                        pos[k] = (c[k].0 + dx, c[k].1 + dy);
                        frames.push((format!("drag {k} {dx},{dy}"), route(&g, &pos)));
                        frames.push((
                            format!("partial {k} {dx},{dy}"),
                            route_partial(&g, &pos, &st, &c),
                        ));
                    }
                }
                for (what, sc) in &frames {
                    let mut bb = Bbox::new();
                    grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
                    let (minx, _, miny, _) = bb.finish();
                    let back = |p: (f64, f64)| (p.0 - MARGIN + minx, p.1 - MARGIN + miny);
                    for (i, at_start, tip, end, _, want) in painted_heads(sc) {
                        let e = &sc.edges[i];
                        let err = (tip.0 - want.0).hypot(tip.1 - want.1);
                        assert!(
                            err <= 0.5,
                            "{w}px {what} e{i} {}->{} {}: tip {tip:?} vs end {want:?} \
                             ({err:.2} px off)\n{src}",
                            e.from,
                            e.to,
                            if at_start { "start" } else { "end" }
                        );
                        heads += 1;
                        // Pick == render along the head's axis, cut to tip.
                        let len = (tip.0 - end.0).hypot(tip.1 - end.1);
                        let n = (len / 2.0).ceil().max(1.0) as usize;
                        for s in 0..=n {
                            let f = s as f64 / n as f64;
                            let q =
                                back((end.0 + (tip.0 - end.0) * f, end.1 + (tip.1 - end.1) * f));
                            let dist = edge_distance(e, q, 2.0);
                            assert!(dist <= 0.5, "{w}px {what} e{i}: {dist} at {q:?}\n{src}");
                        }
                    }
                }
            }
        }
        assert!(heads > 1000, "{heads} heads");

        // The r6 normal-width case (genstep_w0/s02's e8 mid-drag): a 2 px
        // step whose last, vertical leg (4.5 px) is shorter than its cut.
        let g = parse("flowchart TD\n  a --> b\n  linkStyle 0 interpolate step\n").unwrap();
        let mut sc = scene(&g);
        sc.edges[0].waypoints = vec![
            (164.8, 106.6375),
            (93.15, 106.6375),
            (80.4, 106.6375),
            (80.4, 102.1375),
        ];
        let hs = painted_heads(&sc);
        assert_eq!(hs.len(), 1);
        let (_, _, tip, _, _, want) = hs[0];
        let err = (tip.0 - want.0).hypot(tip.1 - want.1);
        assert!(err <= 0.5, "tip {tip:?} vs end {want:?} ({err:.2} px off)");

        // A trailing command without length changes nothing about how the
        // end is squared or how deep it may be cut.
        let segs = vec![
            Seg::L((40.0, 0.0)),
            Seg::L((40.0, 6.0)),
            Seg::L((60.0, 6.0)),
        ];
        let mut tailed = segs.clone();
        tailed.push(Seg::L((60.0, 6.0)));
        assert_eq!(live_len((0.0, 0.0), &tailed), 3);
        let head = Some((10.0, arrow_head_len(20.0)));
        let a = square_end((0.0, 0.0), segs, 35.0, 20.0, head, 1.0);
        let b = square_end((0.0, 0.0), tailed, 35.0, 20.0, head, 1.0);
        assert_eq!((a.2, a.3), (b.2, b.3));
        assert!(b.3.is_finite());
    }

    /// B1 review: on a curve too short to lose its cut (a drag frame's
    /// nodes a few px apart) the stroke is drawn whole, and its head used
    /// to keep its usual cut — its tip [`ARROW_TRIM`] past the curve's end,
    /// inside the node, where picking does not reach. Such a head is cut
    /// nowhere (tip on the curve's end) and shrunk onto its share of the
    /// curve; a head that would come out under [`HEAD_MIN_LEN`] is left out.
    #[test]
    fn a_curve_shorter_than_its_cut_keeps_its_head_on_it() {
        let g =
            parse("flowchart LR\nA[aa] --> B[bb]\nC[cc] <--> D[dd]\nE[ee] --> F[ff]\n").unwrap();
        let st = scene(&g);
        let mut c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        // Each target `gap` px right of its source's right side.
        let n = &st.nodes;
        for (a, b, gap) in [(0, 1, 4.0), (2, 3, 7.0), (4, 5, 0.5)] {
            c[b] = (c[a].0 + n[a].w / 2.0 + gap + n[b].w / 2.0, c[a].1);
        }
        let sc = route(&g, &c);
        let svg = to_svg(&sc);
        let attr = |el: &str, name: &str| -> Option<String> {
            let rest = el.split(&format!(" {name}=\"")).nth(1)?;
            Some(rest.split('"').next()?.to_string())
        };
        let def = |id: &str| -> &str {
            let at = svg
                .find(&format!("<marker id=\"{id}\""))
                .expect("marker def");
            &svg[at..at + svg[at..].find('>').unwrap()]
        };
        let paths = edge_paths(&svg);
        for (k, p) in paths.iter().enumerate() {
            let pts = painted_points(p);
            let len: f64 = pts
                .windows(2)
                .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
                .sum();
            // The tip of a head as long as its glyph box, refX into it.
            let heads = [("marker-start", 0.0), ("marker-end", 10.0)];
            let drawn: Vec<(f64, f64)> = heads
                .iter()
                .filter_map(|(key, tip_x)| {
                    let url = attr(p, key)?;
                    let d = def(url.trim_start_matches("url(#").trim_end_matches(')'));
                    let refx: f64 = attr(d, "refX").unwrap().parse().unwrap();
                    let px: f64 = attr(d, "markerWidth").unwrap().parse().unwrap();
                    // How far past the path's end the tip lands.
                    Some(((tip_x - refx).abs() * px / 10.0, px))
                })
                .collect();
            match k {
                // 4 px apart: one head, tip on the end, no longer than it.
                0 => {
                    assert!(len < ARROW_TRIM + 1.0, "{len} {p}");
                    assert_eq!(drawn.len(), 1, "{p}");
                    let (past, px) = drawn[0];
                    assert!(
                        past < 1e-9 && px <= len + 0.05 && px >= HEAD_MIN_LEN,
                        "{drawn:?} {len}"
                    );
                }
                // Two heads share the 7 px curve.
                1 => {
                    assert!(len < 2.0 * ARROW_TRIM + 1.0, "{len} {p}");
                    assert_eq!(drawn.len(), 2, "{p}");
                    for &(past, px) in &drawn {
                        assert!(past < 1e-9 && px <= len / 2.0 + 0.05, "{drawn:?} {len}");
                    }
                }
                // Half a px: no room for a head at all.
                _ => assert!(drawn.is_empty(), "{p}"),
            }
        }
        // The usual edge keeps its usual head.
        assert!(to_svg(&st).contains("marker-end=\"url(#arrow)\""));
        assert!(ink_off_canvas(&svg).is_empty());
    }

    #[test]
    fn the_svg_pads_the_drawing_as_the_scene_and_strokes_with_the_q10_tokens() {
        let src = "flowchart TD\nA[box] --> B[(db)]\nB ==> C[[sub]]\n";
        let sc = scene(&parse(src).unwrap());
        let svg = to_svg(&sc);
        let mut bb = Bbox::new();
        grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, maxx, miny, maxy) = bb.finish();
        let attr = |name: &str| -> f64 {
            let head = svg.lines().next().unwrap();
            head.split(&format!(" {name}=\""))
                .nth(1)
                .and_then(|r| r.split('"').next())
                .and_then(|v| v.parse().ok())
                .expect("a size")
        };
        // Whole px, as the root always writes them: the drawing and the
        // scene's own MARGIN gutter round it — the scene's canvas, the
        // drawing starting MARGIN in (no layout moved for Q10's 16 px).
        assert!((attr("width") - (maxx - minx + 2.0 * MARGIN)).abs() <= 0.5);
        assert!((attr("height") - (maxy - miny + 2.0 * MARGIN)).abs() <= 0.5);
        assert!((attr("width") - sc.width).abs() <= 0.5);
        assert!((attr("height") - sc.height).abs() <= 0.5);
        assert_eq!((minx, miny), (MARGIN, MARGIN));
        assert_eq!(sc.nodes[0].y - sc.nodes[0].h / 2.0, MARGIN);
        // Edges 2 / 3.5 px; every node stroke 1.8 px, the cylinder's
        // lid and the subroutine's bars included.
        assert!(svg.contains("stroke-width=\"2\" marker-end"));
        assert!(svg.contains("stroke-width=\"3.5\" marker-end"));
        assert!(!svg.contains("stroke-width=\"1.6\""), "{svg}");
        assert_eq!(svg.matches("stroke-width=\"1.8\"").count(), 6, "{svg}");
    }

    // ── B3: back-edge lanes (report §2.2, Q4 + M4, P0 #13) ─────────

    /// The scene and `route()` at its centres, for a flowchart source.
    fn static_and_routed(src: &str) -> [Scene; 2] {
        let g = parse(src).unwrap();
        let s = scene(&g);
        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&g, &c);
        [s, r]
    }

    fn the_edge<'a>(sc: &'a Scene, from: &str, to: &str) -> &'a SceneEdge {
        sc.edges
            .iter()
            .find(|e| e.from == from && e.to == to)
            .unwrap_or_else(|| panic!("no {from} -> {to}"))
    }

    fn node_box(sc: &Scene, id: &str) -> (f64, f64, f64, f64) {
        let n = sc.nodes.iter().find(|n| n.id == id).expect("node");
        (n.x - n.w / 2.0, n.y - n.h / 2.0, n.w, n.h)
    }

    /// Degrees between the drawn end's last stretch and the axis it is
    /// nearest to (0 = square to a side).
    fn end_skew(e: &SceneEdge, at_start: bool) -> f64 {
        let p = edge_polyline(e);
        let n = p.len();
        let (a, b) = if at_start {
            (p[1], p[0])
        } else {
            (p[n - 2], p[n - 1])
        };
        let ang = (b.1 - a.1).atan2(b.0 - a.0).to_degrees().abs();
        [ang, (ang - 90.0).abs(), (ang - 180.0).abs()]
            .into_iter()
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn a_back_edge_runs_in_a_square_lane_beside_what_lies_between() {
        // demo's E -> B: in a lane past Valid? only (not round Show
        // error's own box), out of and into its ends square, clear of
        // every other node — in the static scene and in route() alike.
        let src = "flowchart TD\nA([Start]) --> B[Read input]\nB --> C{Valid?}\n\
                   C -->|yes| D[Process data]\nC -->|no| E[Show error]\nE --> B\nD ==> F((Done))";
        for sc in static_and_routed(src) {
            let e = the_edge(&sc, "E", "B");
            assert!(e.waypoints.len() > 2, "{e:?}");
            assert!(
                end_skew(e, true) < 10.0 && end_skew(e, false) < 10.0,
                "{e:?}"
            );
            for id in ["A", "C", "D", "F"] {
                assert!(!edge_hits(e, node_box(&sc, id)), "E -> B hits {id}");
            }
            // The lane runs within Show error's breadth: leaves it straight.
            let (ex, _, ew, _) = node_box(&sc, "E");
            let x0 = e.waypoints[0].0;
            assert!(e.waypoints[1].0 == x0 && x0 > ex && x0 < ex + ew, "{e:?}");
        }
        let [s, _] = static_and_routed(src);
        assert!(s.width <= 321.5, "demo {}", s.width);
    }

    #[test]
    fn a_lane_round_a_fan_meets_the_hub_on_its_side() {
        // fanout: the `no` loop goes round the workers (it used to cut
        // through them and be rerouted across four edges), and meets the
        // dispatcher on the side facing it, clear of its fan.
        let src = "flowchart TD\nA[Dispatcher] --> B1[Worker 1] & B2[Worker 2] & B3[Worker 3]\n\
                   B1 & B2 & B3 --> C[Aggregator]\nC --> D{All succeeded?}\nD -->|yes| E([Done])\nD -->|no| A";
        for sc in static_and_routed(src) {
            let e = the_edge(&sc, "D", "A");
            let a = node_box(&sc, "A");
            let last = e.waypoints[e.waypoints.len() - 1];
            assert!(
                (last.0 - a.0).abs() < 0.5 || (last.0 - (a.0 + a.2)).abs() < 0.5,
                "enters A's side: {e:?}"
            );
            for id in ["B1", "B2", "B3", "C"] {
                assert!(!edge_hits(e, node_box(&sc, id)), "D -> A hits {id}");
            }
            let others: Vec<&SceneEdge> = sc.edges.iter().filter(|o| o.from != "D").collect();
            let lane = edge_polyline(e);
            for o in others {
                let p = edge_polyline(o);
                let crosses = lane.windows(2).any(|w| {
                    p.windows(2)
                        .any(|v| crate::lanes::tests_seg_cross(w[0], w[1], v[0], v[1]))
                });
                assert!(!crosses, "D -> A crosses {} -> {}", o.from, o.to);
            }
        }
    }

    #[test]
    fn a_lane_never_turns_across_an_edge_arriving_beside_its_end() {
        // state.mmd: `retry` (Rejected -> waiting for input) took the
        // shorter way between the Validating frame and Rejected, judged
        // crossing-free because the straight line of `invalid` reached
        // Rejected's centre past the lane's turn; drawn, it left
        // Rejected's side across `invalid`. Now it runs straight up out
        // of Rejected round the outside, as mermaid draws it, crossing
        // nothing — in the static scene and in route() alike.
        let g = match crate::parser::parse_document(include_str!("../examples/state.mmd")).unwrap()
        {
            crate::model::Document::State(g) => g,
            other => panic!("not a state diagram: {other:?}"),
        };
        let s = scene(&g);
        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&g, &c);
        for sc in [&s, &r] {
            let e = the_edge(sc, "Rejected", "Idle");
            assert!(
                end_skew(e, true) < 10.0 && end_skew(e, false) < 10.0,
                "{e:?}"
            );
            let (rx, ry, rw, _) = node_box(sc, "Rejected");
            let first = e.waypoints[0];
            assert!(
                (first.1 - ry).abs() < 0.5 && first.0 > rx + rw / 2.0,
                "leaves Rejected's top, right of its centre: {e:?}"
            );
            let lane = edge_polyline(e);
            for o in sc.edges.iter().filter(|o| !std::ptr::eq(*o, e)) {
                let p = edge_polyline(o);
                let crosses = lane.windows(2).any(|w| {
                    p.windows(2)
                        .any(|v| crate::lanes::tests_seg_cross(w[0], w[1], v[0], v[1]))
                });
                assert!(!crosses, "retry crosses {} -> {}", o.from, o.to);
            }
        }
    }

    #[test]
    fn a_wide_lr_label_takes_its_height_across_the_flow() {
        // wide-labels: the back-edge's breadth room is its label's 20 px
        // height in LR, not its 337 px width (413 -> at most 201 tall).
        let src = include_str!("../tests/fixtures/quality/wide-labels.mmd");
        let [s, _] = static_and_routed(src);
        assert!(s.height <= 201.0, "{}", s.height);
        assert_eq!(
            crate::lanes::label_breadth(Some("a wide label"), true),
            LABEL_BOX_H
        );
    }

    #[test]
    fn a_labelled_lr_two_cycle_bows_apart_so_each_label_rides_its_own_line() {
        // wide-labels: Client <-> Gateway left their 38 px sides 14 px
        // apart, and the return edge's 337 px label, centred on its own
        // line, covered the request edge near Client. The twins now bow
        // apart (TWIN_LABEL_BOW): no label sits on another edge, in either
        // mode, and both still meet their sides near square.
        let src = include_str!("../tests/fixtures/quality/wide-labels.mmd");
        let r = crate::quality::measure_source(src, false).unwrap();
        assert_eq!((r.statik.lbl_edge, r.routed.lbl_edge), (0, 0));
        for sc in static_and_routed(src) {
            for (a, b) in [("A", "B"), ("B", "A")] {
                let e = the_edge(&sc, a, b);
                assert!(
                    end_skew(e, true) < 10.0 && end_skew(e, false) < 10.0,
                    "{e:?}"
                );
            }
        }
        // Only a labelled LR / RL 2-cycle across one gap bows.
        let g = parse("flowchart TD\nA -->|go| B\nB -->|back| A").unwrap();
        assert!(twin_bows(&g, &parallel_offsets(&g), |_| true)
            .iter()
            .all(|&b| b == 0.0));
        let g = parse("flowchart LR\nA --> B\nB --> A").unwrap();
        assert!(twin_bows(&g, &parallel_offsets(&g), |_| true)
            .iter()
            .all(|&b| b == 0.0));
        let g = parse("flowchart LR\nA -->|go| B\nB --> A").unwrap();
        let bows = twin_bows(&g, &parallel_offsets(&g), |_| true);
        assert_eq!(bows, vec![-TWIN_LABEL_BOW, TWIN_LABEL_BOW]);
    }

    #[test]
    fn far_apart_back_edges_and_frame_legs_square_up_alike_in_route() {
        // bench-75: `svc2 -> db` (reversed by the layout, 592 px across a
        // 60 px gap) swept along the gap as a reversed S and entered db at
        // 71 deg in route(); past DIRECT_REACH it runs in a lane, square
        // at both ends. `job1 -> svc1`'s leg out of the jobs frame ran
        // 6 px past its border; it now runs a stub past it, and route()
        // puts the lane where the static layout does (it took the other
        // side, 1 400 px away, on the first drag).
        let src = include_str!("../tests/fixtures/quality/bench-75.mmd");
        let [s, r] = static_and_routed(src);
        for sc in [&s, &r] {
            let e = the_edge(sc, "svc2", "db");
            assert!(!e.waypoints.is_empty(), "{e:?}");
            assert!(
                end_skew(e, true) < 10.0 && end_skew(e, false) < 10.0,
                "{e:?}"
            );
            let jobs = sc.clusters.iter().find(|c| c.id == "jobs").unwrap();
            let lane = the_edge(sc, "job1", "svc1");
            let leg = lane.waypoints[2].0;
            assert!(
                jobs.x - leg >= crate::lanes::LANE_STUB - 0.5,
                "leg at {leg}, frame at {}",
                jobs.x
            );
        }
        let lane_y = |sc: &Scene| {
            let w = &the_edge(sc, "job1", "svc1").waypoints;
            w[w.len() / 2].1
        };
        assert!(
            (lane_y(&s) - lane_y(&r)).abs() < 1.0,
            "{} vs {}",
            lane_y(&s),
            lane_y(&r)
        );
    }

    #[test]
    fn a_two_cycle_runs_side_by_side_across_its_gap() {
        // lr: SVC <-> CACHE across one gap, parallel and uncrossed, in
        // every direction (it hung a V-shaped loop below the row).
        for dir in ["LR", "RL", "TD", "BT"] {
            let src = format!(
                "flowchart {dir}\nU[User] --> FE(Frontend) --> API{{Auth?}}\nAPI -->|ok| SVC[Service Layer]\n\
                 API -.->|denied| ERR[401 Response]\nSVC ==> DB[Database]\nSVC --> CACHE([Redis Cache])\nCACHE --> SVC"
            );
            for sc in static_and_routed(&src) {
                let (f, b) = (the_edge(&sc, "SVC", "CACHE"), the_edge(&sc, "CACHE", "SVC"));
                assert!(b.waypoints.is_empty(), "{dir}: across the gap {b:?}");
                let (pf, pb) = (edge_polyline(f), edge_polyline(b));
                let crosses = pf.windows(2).any(|w| {
                    pb.windows(2)
                        .any(|v| crate::lanes::tests_seg_cross(w[0], w[1], v[0], v[1]))
                });
                assert!(!crosses, "{dir}: the twins cross");
            }
        }
        let [s, _] = static_and_routed(
            "flowchart LR\nU[User] --> FE(Frontend) --> API{Auth?}\nAPI -->|ok| SVC[Service Layer]\n\
             API -.->|denied| ERR[401 Response]\nSVC ==> DB[Database]\nSVC --> CACHE([Redis Cache])\nCACHE --> SVC",
        );
        assert!(s.height <= 226.0, "lr {}", s.height);
    }

    #[test]
    fn a_lane_between_two_members_stays_in_their_box() {
        // advanced: the dashed `fail` edge's lane and label stay inside
        // the CI frame (it bulged out of it).
        let src = include_str!("../examples/advanced.mmd");
        let [s, _] = static_and_routed(src);
        let ci = s.clusters.iter().find(|c| c.id == "CI").expect("CI frame");
        let e = the_edge(&s, "Scan", "Dev");
        for (x, y) in edge_polyline(e) {
            assert!(
                x >= ci.x && x <= ci.x + ci.w && y >= ci.y && y <= ci.y + ci.h,
                "({x}, {y})"
            );
        }
        let (_, (lx, _), lw) = e.label.clone().expect("fail label");
        assert!(lx - lw / 2.0 >= ci.x && lx + lw / 2.0 <= ci.x + ci.w);
    }

    #[test]
    fn a_lane_stays_in_its_frames_in_route_and_on_a_drag_too() {
        // A labelled loop inside a nested box: route() drew the lane
        // and its label outside both frames (it grew them only in the
        // static layout), so the edge jumped out of its box on the first
        // drag frame. Every frame holding both ends holds the lane and
        // its label, in scene(), route() and route_partial() alike.
        let src = "flowchart TD\n  subgraph Outer[Outer system]\n    subgraph Inner[Inner loop]\n      I1[Fetch] --> I2[Decode]\n      I2 --> I3{More?}\n      I3 -->|more data please| I1\n    end\n    O1[Collect] --> Inner\n    I3 -->|done| O2[Report]\n    O2 -.->|again| O1\n  end\n  X[External] --> O1\n  O2 --> Y[(Archive)]\n  Y -->|replay| X\n";
        let g = parse(src).unwrap();
        let s = scene(&g);
        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = route(&g, &c);
        // Drag the External box a little: Inner and Outer stay put.
        let mut moved = c.clone();
        let x = g.node_index("X").unwrap();
        moved[x].1 += 6.0;
        let p = route_partial(&g, &moved, &s, &c);
        for (name, sc) in [("scene", &s), ("route", &r), ("route_partial", &p)] {
            let e = the_edge(sc, "I3", "I1");
            assert!(!e.waypoints.is_empty(), "{name}: no lane {e:?}");
            let (_, (lx, ly), lw) = e.label.clone().expect("loop label");
            for id in ["Inner", "Outer"] {
                let f = sc.clusters.iter().find(|c| c.id == id).unwrap();
                let inside = |(x, y): (f64, f64)| {
                    x >= f.x - 1e-6
                        && x <= f.x + f.w + 1e-6
                        && y >= f.y - 1e-6
                        && y <= f.y + f.h + 1e-6
                };
                for q in edge_polyline(e) {
                    assert!(inside(q), "{name}: {q:?} outside {id} {f:?}");
                }
                assert!(
                    inside((lx - lw / 2.0, ly)) && inside((lx + lw / 2.0, ly)),
                    "{name}: label at ({lx}, {ly}) outside {id}"
                );
            }
        }
        // route() draws the frames the static layout drew.
        for (a, b) in s.clusters.iter().zip(&r.clusters) {
            assert_eq!(a.id, b.id);
            assert!(
                (a.x - b.x).abs() < 0.5 && (a.w - b.w).abs() < 0.5,
                "{a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn a_wide_strokes_routed_head_lies_on_a_straight_stub() {
        // B3 review r5 #5 (q72): in route() the 16 px `linkStyle` edge
        // n1 ==> n7 turned into n7's side 14.5 px out — the router's
        // plain stub — under a 28 px head, so the painter's head turned
        // with the corner and hooked past the side into it (wiggles
        // 0 -> 1). A glyph end under a grown head now takes a stub the
        // head's length plus a corner's radius long where that costs
        // about the same way round.
        let src = "flowchart RL\n  n10 ==>|thick lbl| n6\n  n1 ==> n7\n  n6 <-.-> n10\n  \
                   n7 -->|lbl| n9\n  n2 o--o n5\n  n3 <--> n7\n  n10 o--o n7\n  \
                   n10 -.->|dot| n3\n  n1 ==> n10\n  n6 --- n3\n  n10 <-.-> n8\n  \
                   n2 -->|lbl| n3\n  n3 <-.-> n10\n  n4 ==> n3\n  n7 ==>|thick lbl| n7\n  \
                   n5 <--> n10\n  n0 ==> n8\n  n1 <-.-> n6\n  n7 -.-> n2\n  n10 --o n0\n  \
                   linkStyle 8 stroke:#c33,stroke-width:7px\n  linkStyle 14 stroke-width:9px\n  \
                   linkStyle 1 stroke:#c33,stroke-width:16px\n";
        let g = parse(src).unwrap();
        let [_, r] = static_and_routed(src);
        let e = the_edge(&r, "n1", "n7");
        // The route's last corner lies a head's length out at least.
        let w = &e.waypoints;
        let n = w.len();
        let end = w[n - 1];
        let on_last = |p: (f64, f64)| (p.0 - end.0).abs() < 0.5 || (p.1 - end.1).abs() < 0.5;
        let straight = (0..n - 1)
            .rev()
            .take_while(|&k| on_last(w[k]) && on_last(w[k + 1]))
            .map(|k| (w[k].0 - end.0).hypot(w[k].1 - end.1))
            .fold(0.0, f64::max);
        let head = edge_head_len(e);
        assert!(head > ARROW_MARKER_PX, "{head}");
        assert!(
            straight >= head,
            "stub {straight:.1} px under a {head:.1} px head"
        );
        // And nothing hooks back along the flow.
        let m = crate::quality::measure(&g, &r);
        assert_eq!((m.wiggles, m.backtrack), (0, 0.0));
    }

    #[test]
    fn a_rerouted_edge_between_members_stays_in_their_frame_in_route() {
        // B3 review r5 #3 (q21): in route() a member's edge to its own
        // box (drawn to the box's entry member, `n5 <--> F0` to Node 6)
        // was rerouted out through the frame's bottom and back in, 798
        // px outside it, because the router priced leaving home below a
        // backtrack along the flow. It now ranks with the frames: of the
        // candidates the one that stays inside wins, in route() as in
        // scene().
        let src = "flowchart LR\n  subgraph F0 [Frame 0]\n    n6[/Node 6/]\n    \
                   n5(Node 5 with a longer label)\n  end\n  n4 -->|lbl| n2\n  n10 --o F0\n  \
                   n9 ==> n7\n  n3 o--o n5\n  n7 o--o F0\n  n8 <-.-> n13\n  n1 --x n0\n  \
                   n13 --o n13\n  n13 -->|lbl| n2\n  n7 x--x F0\n  n1 -->|lbl| n8\n  \
                   n5 <--> F0\n  n6 --x F0\n  n0 ==> n3\n  n12 --- n11\n  n9 -.->|dot| n8\n  \
                   n2 <==> n8\n  n8 -->|lbl| n1\n  n0 <-.-> n7\n  n5 -.-> n8\n  \
                   n1 -.->|dot| F0\n  n7 <==> n8\n  n6 --o n7\n  n5 --- n9\n  n10 ==> F0\n  \
                   n8 -.-> n0\n  n9 --o n12\n  n4 <-.-> n5\n";
        for sc in static_and_routed(src) {
            let f = sc.clusters.iter().find(|c| c.id == "F0").unwrap();
            for e in sc.edges.iter().filter(|e| e.from == "n5" && e.to == "F0") {
                for (x, y) in edge_polyline(e) {
                    assert!(
                        x >= f.x - 0.5
                            && x <= f.x + f.w + 0.5
                            && y >= f.y - 0.5
                            && y <= f.y + f.h + 0.5,
                        "({x:.1}, {y:.1}) outside {f:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_two_cycle_pulls_its_pair_once_whichever_edge_comes_first() {
        // Report Q8: a 2-cycle listed its pair twice in Brandes-Köpf's
        // neighbours when the return edge was written first, so the
        // median landed on the pair and Service Layer sat level with
        // Redis Cache instead of on the middle of its fan. Either order
        // now gives lr's symmetric fans.
        let lr = include_str!("../examples/lr.mmd");
        let swapped = "flowchart LR\n  U[User] --> FE(Frontend) --> API{Auth?}\n  \
                       API -->|ok| SVC[Service Layer]\n  API -.->|denied| ERR[401 Response]\n  \
                       CACHE([Redis Cache]) --> SVC\n  SVC ==> DB[Database]\n  SVC --> CACHE\n";
        for src in [lr, swapped] {
            let sc = scene(&parse(src).unwrap());
            let y = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().y;
            assert!(
                (y("API") - (y("SVC") + y("ERR")) / 2.0).abs() < 0.5,
                "Auth? off the middle of its ok/denied fan: {src}"
            );
            assert!(
                (y("SVC") - (y("DB") + y("CACHE")) / 2.0).abs() < 0.5,
                "Service Layer off the middle of its fan: {src}"
            );
            assert!((y("U") - y("API")).abs() < 0.5, "the spine stays straight");
        }
        // Parallel forward edges keep their weight: a twin pair runs
        // straight down its own column (the twin curves are drawn for
        // that), the third branch beside it.
        let sc =
            scene(&parse("flowchart TD\n  A --> B\n  B --> C\n  B --> C\n  B --> D\n").unwrap());
        let x = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().x;
        assert!((x("B") - x("C")).abs() < 0.5);
    }

    #[test]
    fn a_parallel_label_and_a_loop_keep_off_the_other_lines() {
        // stress (report P0 #13): the `parallel` label clears the
        // unlabelled twin, and the thick A ==> D keeps clear of B's loop.
        let src = include_str!("../examples/stress.mmd");
        let g = parse(src).unwrap();
        let sc = scene(&g);
        let plain = sc
            .edges
            .iter()
            .find(|e| e.from == "A" && e.to == "B" && e.label.is_none())
            .unwrap();
        let labelled = sc
            .edges
            .iter()
            .find(|e| e.from == "A" && e.to == "B" && e.label.is_some())
            .unwrap();
        let (_, (cx, cy), w) = labelled.label.clone().unwrap();
        let r = (cx - w / 2.0, cy - LABEL_BOX_H / 2.0, w, LABEL_BOX_H);
        assert!(!edge_hits(plain, r), "the label covers its twin");
        let thick = the_edge(&sc, "A", "D");
        let lp = edge_polyline(the_edge(&sc, "B", "B"));
        let gap = edge_polyline(thick)
            .iter()
            .flat_map(|&p| lp.iter().map(move |&q| (p.0 - q.0).hypot(p.1 - q.1)))
            .fold(f64::INFINITY, f64::min);
        assert!(
            gap >= crate::clearance::NODE_SEP,
            "A ==> D {gap:.1} px from the loop"
        );
        // The dashed back-edge enters Source square, crossing nothing.
        let back = the_edge(&sc, "C", "A");
        assert!(end_skew(back, false) < 10.0, "{back:?}");
    }

    /// Whether `pts` (a polyline) passes through the label box of `e`.
    fn crosses_label(e: &SceneEdge, pts: &[(f64, f64)]) -> bool {
        let Some((_, c, w)) = &e.label else {
            return false;
        };
        let (hw, hh) = (w / 2.0, crate::layout::LABEL_BOX_H / 2.0);
        pts.windows(2).any(|s| {
            let n = ((s[1].0 - s[0].0).hypot(s[1].1 - s[0].1) / 2.0)
                .ceil()
                .max(1.0) as usize;
            (0..=n).any(|k| {
                let t = k as f64 / n as f64;
                let p = (
                    s[0].0 + t * (s[1].0 - s[0].0),
                    s[0].1 + t * (s[1].1 - s[0].1),
                );
                (p.0 - c.0).abs() < hw && (p.1 - c.1).abs() < hh
            })
        })
    }

    #[test]
    fn an_lr_label_beside_a_lanes_legs_gets_its_gap_widened() {
        // x6-rl-loops: the two `Gamma -> Alpha` lanes enter Alpha's
        // downstream side by legs across the Beta-Alpha gap, where the
        // `forward` and `parallel` labels sat centred over them. The
        // layout keeps the legs' depth clear at both ends of a gap whose
        // label reaches across the span edge the legs leave from.
        let src =
            "flowchart RL\n  A[Alpha] -->|forward| B[Beta]\n  A -->|parallel| B\n  A --> B\n  \
                   B --> C[Gamma]\n  C --> A\n  B --> B\n  C -.->|long back label here| A\n  \
                   C --> D[Delta] --> E[Epsilon]\n  E --> C";
        for sc in static_and_routed(src) {
            for lbl in sc
                .edges
                .iter()
                .filter(|e| e.label.is_some() && e.from == "A")
            {
                for lane in sc.edges.iter().filter(|e| e.from == "C" && e.to == "A") {
                    assert!(
                        !crosses_label(lbl, &edge_polyline(lane)),
                        "{:?} over a lane",
                        lbl.label
                    );
                }
            }
        }
    }

    #[test]
    fn a_choice_twins_middle_label_keeps_beside_its_line() {
        // State twins3: three labelled transitions from a <<choice>> to
        // one state. The middle label, beside its own line and clipping
        // one neighbour's, read as the neighbour's once a tie was judged
        // by the nearer line to its centre, and moved onto its own line
        // between both neighbours, covering two strokes. A labelled line
        // carries its own label and never takes this one over.
        let src = "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> B : one\n\
                   c --> B : two\nc --> B : three";
        let g = match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            _ => panic!("state"),
        };
        let s = scene(&g);
        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        for sc in [s.clone(), route(&g, &c)] {
            let twins: Vec<&SceneEdge> = sc
                .edges
                .iter()
                .filter(|e| e.from == "c" && e.to == "B")
                .collect();
            let covered: usize = twins
                .iter()
                .map(|l| {
                    twins
                        .iter()
                        .filter(|o| !std::ptr::eq(**o, *l) && crosses_label(l, &edge_polyline(o)))
                        .count()
                })
                .sum();
            assert!(covered <= 1, "{covered} twin strokes under labels");
        }
    }

    #[test]
    fn nested_lanes_take_the_same_sides_in_scene_and_route() {
        // x2-nested-backs: `fail` (Store -> Parse) tied between its two
        // sides in the layout's planning and took the bottom, so
        // `rollback` (Transform -> Source) went over the top across
        // `retry`'s lane; route() put `fail` beside `retry` on top and
        // `rollback` underneath, crossing nothing, so the two lanes
        // swapped sides on the first drag. A tie now goes to the side the
        // planned lanes sharing its end took, and planned lanes count
        // among what a later lane crosses, in both.
        let src = "flowchart LR\n  S[Source] --> P1[Parse] --> P2[Validate] --> P3[Transform] --> P4[Store]\n  \
                   P1 --> Q[Queue]\n  Q --> P3\n  P4 -->|fail| P1\n  P4 -->|retry| P2\n  \
                   P3 -.->|rollback| S\n  P4 ==>|done| S";
        let [st, rt] = static_and_routed(src);
        let side = |sc: &Scene, f: &str, t: &str| {
            let mid = sc.nodes.iter().map(|n| n.y).sum::<f64>() / sc.nodes.len() as f64;
            let p = edge_polyline(the_edge(sc, f, t));
            let (lo, hi) = p.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |r, q| {
                (r.0.min(q.1), r.1.max(q.1))
            });
            (mid - lo) > (hi - mid)
        };
        for (f, t) in [("P4", "P1"), ("P3", "S"), ("P4", "P2"), ("P4", "S")] {
            assert_eq!(side(&st, f, t), side(&rt, f, t), "{f} -> {t}");
        }
        for sc in [&st, &rt] {
            let backs: Vec<Vec<(f64, f64)>> = [("P4", "P1"), ("P3", "S"), ("P4", "P2")]
                .iter()
                .map(|&(f, t)| edge_polyline(the_edge(sc, f, t)))
                .collect();
            for a in 0..backs.len() {
                for b in a + 1..backs.len() {
                    let crosses = backs[a].windows(2).any(|w| {
                        backs[b]
                            .windows(2)
                            .any(|v| crate::lanes::tests_seg_cross(w[0], w[1], v[0], v[1]))
                    });
                    assert!(!crosses, "lanes {a} and {b} cross");
                }
            }
        }
    }

    #[test]
    fn a_lane_meets_its_ends_from_outside() {
        // r274 (RL): `rollback`'s lane ran 11 px inside `Check queue`'s
        // 38 px breadth, outside its ports' span: met on the flank, the
        // lane ran on along the box to its centre level and turned to the
        // flank from inside, its head painted under the box pointing out
        // (end tilt 180 deg). r232 (LR) and r164 (BT) in `scene()`, r046
        // (RL) in `route()` and state `choice_lr` (`Check -> c` left
        // Check's bottom and ran 40 px inside it) the same. A lane meets
        // a flank only past it by LANE_CLEAR (`lanes::meets_side`): every
        // drawn point of a lane but its two ends lies outside both end
        // boxes, and its head points into its target.
        let flow = |n: &str| format!("flowchart {n}");
        let srcs = [
            format!(
                "{}\nN0(Done archive)\nN1[Check]\nN2[Done emit]\nN3([Validate])\n\
                 N4[Check queue]\nN5(Report)\nN0 --> N1\nN1 -->|next| N2\nN0 --> N3\n\
                 N1 --> N4\nN3 --> N5\nN0 --> N3\nN4 -.-> N3\nN5 -->|rollback| N4",
                flow("RL")
            ),
            format!(
                "{}\nN0[Queue]\nN1(Report notify)\nN2[Report]\nN3[Queue fetch]\nN4{{Emit}}\n\
                 N5([Done])\nN6(Archive notify)\nN7{{Store done}}\nN0 --> N1\nN1 --> N2\n\
                 N1 --> N3\nN2 --> N4\nN3 --> N5\nN5 --> N6\nN6 --> N7\nN3 --> N7\n\
                 N4 -.-> N1\nN4 --> N0\nN6 --> N3",
                flow("LR")
            ),
            format!(
                "{}\nN0(Validate parse)\nN1([Notify])\nN2{{{{Notify}}}}\nN3([Check fetch])\n\
                 N4[Check]\nN5{{Load}}\nN6[Report]\nN0 --> N1\nN0 -->|fail| N2\nN2 --> N3\n\
                 N3 -->|next| N4\nN3 --> N5\nN5 --> N6\nN2 --> N4\nN1 --> N6\nN3 --> N5\n\
                 N6 ==>|retry| N3\nN3 --> N2\nN6 -->|retry| N4",
                flow("BT")
            ),
            format!(
                "{}\nN0(Store)\nN1(Notify check)\nN2([Parse])\nN3((Retry emit))\n\
                 N4([Check report])\nN5([Emit load])\nN6(Notify store)\nN7{{Queue parse}}\n\
                 N0 --> N1\nN1 --> N2\nN2 --> N3\nN2 --> N4\nN4 -->|ok| N5\nN5 --> N6\n\
                 N6 --> N7\nN3 --> N5\nN2 --> N7\nN4 --> N5\nN6 -->|rollback| N0\n\
                 N5 ==> N2\nN5 ==>|rollback| N4",
                flow("RL")
            ),
            "stateDiagram-v2\ndirection LR\nstate c <<choice>>\n[*] --> Check\nCheck --> c\n\
             c --> Ok : yes\nc --> Fail : no\nFail --> Check : retry\nOk --> [*]"
                .to_string(),
        ];
        for src in &srcs {
            let g = match crate::parser::parse_document(src).unwrap() {
                crate::model::Document::Flowchart(g) | crate::model::Document::State(g) => g,
                _ => panic!("flowchart or state"),
            };
            let s = scene(&g);
            let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
            let sign = if matches!(g.direction, Direction::BT | Direction::RL) {
                -1.0
            } else {
                1.0
            };
            let mut lanes = 0;
            for (mode, sc) in [("static", s.clone()), ("routed", route(&g, &c))] {
                for (i, e) in g.edges.iter().enumerate() {
                    let se = &sc.edges[i];
                    let (a, b) = (&sc.nodes[e.from], &sc.nodes[e.to]);
                    let at = |n: &SceneNode| sign * if horizontal { n.x } else { n.y };
                    if e.from == e.to || se.waypoints.len() < 3 || at(b) >= at(a) {
                        continue;
                    }
                    lanes += 1;
                    let p = edge_polyline(se);
                    let (p0, p3) = (p[0], p[p.len() - 1]);
                    // Strictly inside the outline (a px in from it).
                    let inside = |n: &SceneNode, q: (f64, f64)| {
                        let (dx, dy) = ((q.0 - n.x).abs(), (q.1 - n.y).abs());
                        let (hw, hh) = (n.w / 2.0 - 1.0, n.h / 2.0 - 1.0);
                        match n.shape {
                            Shape::Diamond => dx / hw + dy / hh < 1.0,
                            Shape::Circle | Shape::DoubleCircle => dx.hypot(dy) < hw.min(hh),
                            Shape::Stadium => {
                                let r = hh.min(hw);
                                dy < hh && (dx < hw - r || (dx - (hw - r)).hypot(dy) < r)
                            }
                            _ => dx < hw && dy < hh,
                        }
                    };
                    let near = |q: (f64, f64), r: (f64, f64)| (q.0 - r.0).hypot(q.1 - r.1) < 1.5;
                    for &q in p.iter().filter(|&&q| !near(q, p0) && !near(q, p3)) {
                        assert!(
                            !inside(a, q) && !inside(b, q),
                            "{mode} {} -> {}: {q:?} inside an end in\n{src}",
                            a.id,
                            b.id
                        );
                    }
                    // The head points into the target.
                    let q = p[p.len() - 2];
                    let (dx, dy) = (p3.0 - q.0, p3.1 - q.1);
                    assert!(
                        dx * (b.x - p3.0) + dy * (b.y - p3.1) > 0.0,
                        "{mode} {} -> {}: head points out in\n{src}",
                        a.id,
                        b.id
                    );
                }
            }
            assert!(lanes > 0, "no lane in\n{src}");
        }
    }

    #[test]
    fn route_lanes_take_the_static_side_and_no_leg_slides_into_a_node() {
        // r280 (BT): `route()` judged `Store check -> Done store` by its
        // bare chord, up through `Queue validate`; the router then drew
        // it round the right, where the `again` lane went too (on the
        // left in `scene()`), and the ports stage, spacing Done store's
        // bottom, slid that edge's 400 px leg 20 px into Queue validate.
        // r046 (RL): `Emit load ==> Parse` ran its lane in `route()` 1 px
        // off the chord of `Parse -> Check report` (on top of it), not
        // over the top as in `scene()`. Each lane now takes the static
        // side, and no edge runs through a node in either drawing.
        let r280 = "flowchart BT\nN0(Store check)\nN1[Notify]\nN2[Queue validate]\n\
                    N3(Done store)\nN4(Queue)\nN5([Load notify])\nN6{Fetch}\nN0 --> N1\n\
                    N0 -->|yes| N2\nN0 --> N3\nN2 --> N4\nN3 --> N5\nN4 --> N6\n\
                    N6 -->|retry| N3\nN3 --> N2\nN3 -->|again| N2";
        let r046 = "flowchart RL\nN0(Store)\nN1(Notify check)\nN2([Parse])\nN3((Retry emit))\n\
                    N4([Check report])\nN5([Emit load])\nN6(Notify store)\nN7{Queue parse}\n\
                    N0 --> N1\nN1 --> N2\nN2 --> N3\nN2 --> N4\nN4 -->|ok| N5\nN5 --> N6\n\
                    N6 --> N7\nN3 --> N5\nN2 --> N7\nN4 --> N5\nN6 -->|rollback| N0\n\
                    N5 ==> N2\nN5 ==>|rollback| N4";
        // r158 (RL): `Done -> Done` (circle to diamond) runs its chord
        // through the cylinder the lane leaves, but the router holds its
        // source's breadth past the cylinder: judged round its nearer
        // side, `rollback` took the bottom in `route()`, across 3 edges.
        let r158 = "flowchart RL\nN0{Check load}\nN1([Check])\nN2(Store emit)\nN3((Done))\n\
                    N4(Parse)\nN5([Emit])\nN6([Validate queue])\nN7([Load])\nN8[(Done)]\n\
                    N9{Done}\nN0 --> N1\nN1 -->|next| N2\nN1 --> N3\nN2 --> N4\n\
                    N4 -->|yes| N5\nN3 --> N6\nN6 -->|yes| N7\nN6 --> N8\nN7 --> N9\n\
                    N3 --> N9\nN3 --> N5\nN8 -.->|rollback| N1";
        // (source, edge index, the lane's side: its mean breadth less its
        // target's, across the flow — x in BT, y in RL).
        for (src, ei, across_x) in [(r280, 8, true), (r046, 11, false), (r158, 11, false)] {
            let g = parse(src).unwrap();
            let sides: Vec<f64> = static_and_routed(src)
                .iter()
                .map(|sc| {
                    assert_eq!(
                        crate::quality::measure(&g, sc).edge_node_x,
                        0,
                        "an edge through a node in\n{src}"
                    );
                    let e = &sc.edges[ei];
                    assert!(e.waypoints.len() > 2, "{e:?}");
                    let t = &sc.nodes[g.edges[ei].to];
                    let k = e.waypoints.len() as f64;
                    let mean =
                        |f: fn(&(f64, f64)) -> f64| e.waypoints.iter().map(f).sum::<f64>() / k;
                    if across_x {
                        mean(|p| p.0) - t.x
                    } else {
                        mean(|p| p.1) - t.y
                    }
                })
                .collect();
            assert!(
                sides[0] * sides[1] > 0.0,
                "edge {ei}: static side {:.0}, routed side {:.0} in\n{src}",
                sides[0],
                sides[1]
            );
        }
    }

    #[test]
    fn a_two_cycle_into_a_diamond_keeps_its_twin_ends_apart() {
        // r048 (BT): `Parse emit --> Emit` crossing the gap back aimed at
        // Parse emit shifted by its parallel offset and landed on Emit's
        // upper-right flank, 4 px from where `Emit -> Parse emit` leaves
        // its top: the ports stage took it for an end on the flank and
        // never spaced the two, so the curves crossed and the head sat on
        // the twin's start. The end now stays on the diamond's flow side.
        let src = "flowchart BT\nN0(Done)\nN1(Queue notify)\nN2{Emit}\nN3[Store emit]\n\
                   N4{Notify}\nN5{Parse emit}\nN6[Load]\nN0 -->|yes| N1\nN1 -->|fail| N2\n\
                   N1 -->|ok| N3\nN2 -->|no| N4\nN2 -->|no| N5\nN5 --> N6\nN2 --> N4\n\
                   N5 --> N2\nN2 ==> N0\nN4 --> N3";
        let g = parse(src).unwrap();
        for sc in static_and_routed(src) {
            let m = crate::quality::measure(&g, &sc);
            assert_eq!((m.shared_ports, m.arrow_on_start), (0, 0), "{m:?}");
            assert!(end_skew(&sc.edges[7], false) < 10.0, "{:?}", sc.edges[7]);
        }
    }

    #[test]
    fn a_lane_meets_a_slanted_shape_on_its_flat_run() {
        // advanced's `fail` lane (TD): its port span was a box's (12 px
        // in from the bounding box's corners), so its tip landed past
        // Developer's bottom-right vertex (1.9 px off the outline, half
        // the head over the slant) and its tail just off Security scan's
        // top-right vertex. Both ends now lie on the flat runs.
        let src = std::fs::read_to_string("examples/advanced.mmd").unwrap();
        let g = parse(&src).unwrap();
        let s = scene(&g);
        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let ei = g
            .edges
            .iter()
            .position(|e| e.label.as_deref() == Some("fail"))
            .expect("fail edge");
        for sc in [s.clone(), route(&g, &c)] {
            let e = &sc.edges[ei];
            assert!(e.waypoints.len() > 2, "a lane: {e:?}");
            let p = edge_polyline(e);
            for (q, v) in [(p[0], g.edges[ei].from), (p[p.len() - 1], g.edges[ei].to)] {
                let n = &sc.nodes[v];
                let k = SLANT_MAX.min(n.w / 4.0);
                assert!(
                    (q.0 - n.x).abs() <= n.w / 2.0 - k - ARROW_MARKER_PX / 2.0,
                    "{} at {q:?} off the flat run of {:?}",
                    n.id,
                    (n.x, n.w)
                );
                assert!(
                    (q.1 - n.y).abs() >= n.h / 2.0 - 0.5,
                    "{q:?} on the flat side"
                );
            }
        }
    }

    #[test]
    fn a_lane_into_an_over_full_side_stays_on_it() {
        // q10 (LR): n11's right side holds more ends than fit 5 px apart,
        // so the ports stage spilled its outermost round the corner — a
        // lane's end, whose last leg `turn_end` swung into n11's top at
        // 34 deg. A lane's end stays on its side; every lane's head
        // enters square.
        let src = "flowchart LR\nn6 --- n6\nn11 -->|lbl| n3\nn8 --x n10\nn9 o--o n11\n\
                   n9 x--x n12\nn1 x--x n0\nn11 -.-> n4\nn2 -.-> n8\nn12 --- n11\n\
                   n1 -.->|dot| n11\nn10 x--x n11\nn0 <-.-> n9\nn0 --o n4\nn1 <-.-> n3\n\
                   n4 ==>|thick lbl| n3\nn3 <==> n5\nn1 --- n10\nn0 <-.-> n5\n\
                   n3 o--o n11\nn4 --- n0\nn9 -.->|dot| n8\nn5 --o n12\nn0 o--o n1\n\
                   n7 -->|lbl| n5\nn1 <==> n8\nn12 x--x n1";
        for sc in static_and_routed(src) {
            let g = parse(src).unwrap();
            for (i, e) in g.edges.iter().enumerate() {
                let se = &sc.edges[i];
                let (a, b) = (&sc.nodes[e.from], &sc.nodes[e.to]);
                if e.from == e.to || se.waypoints.len() < 3 || b.x >= a.x {
                    continue;
                }
                assert!(
                    end_skew(se, false) <= 10.0,
                    "{} -> {}: head {:.1} deg",
                    a.id,
                    b.id,
                    end_skew(se, false)
                );
            }
        }
    }

    #[test]
    fn a_short_bowed_twin_leaves_and_enters_near_square() {
        // State lr_right: the labelled return edge of a 2-cycle across a
        // 74 px LR gap bowed its full TWIN_LABEL_BOW at its source (35 deg
        // off square), and the ports stage, taking it for an obstacle
        // bow, pulled its target control back when it spread the ends
        // (the head entered Draft at 30 deg). The bow now keeps both ends
        // within TWIN_BOW_TILT, and a control running straight out of
        // the end being moved moves with it.
        for src in [
            "stateDiagram-v2\ndirection LR\n[*] --> Draft\nDraft --> Review\n\
             Review --> Draft : changes\nReview --> Done\nnote right of Draft : author edits",
            "stateDiagram-v2\ndirection LR\n[*] --> S\nS --> T1\nS --> T2\nS --> T3\n\
             T1 --> S : back\nnote right of S : busy state\nnote left of T2 : t2 note",
        ] {
            let g = match crate::parser::parse_document(src).unwrap() {
                crate::model::Document::State(g) => g,
                _ => panic!("state"),
            };
            let s = scene(&g);
            let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            for sc in [s.clone(), route(&g, &c)] {
                let back = sc
                    .edges
                    .iter()
                    .find(|e| {
                        e.label
                            .as_ref()
                            .is_some_and(|l| l.0 == "changes" || l.0 == "back")
                    })
                    .expect("return edge");
                assert!(
                    end_skew(back, false) <= 10.0,
                    "head {:.1}",
                    end_skew(back, false)
                );
            }
            let back = s
                .edges
                .iter()
                .find(|e| e.label.as_ref().is_some_and(|l| l.0 == "changes"));
            if let Some(b) = back {
                assert!(end_skew(b, true) <= 10.0, "tail {:.1}", end_skew(b, true));
            }
        }
    }

    #[test]
    fn twin_lanes_leave_and_enter_in_their_nesting_order() {
        // x6-rl-loops: two back-edges Gamma -> Alpha in lanes on one side
        // left Gamma from ports in the wrong order in route() (which does
        // not re-sort its ports): the outer lane's leg crossed the inner
        // one twice. The outer lane now takes the port nearest its side.
        let src =
            "flowchart RL\n  A[Alpha] -->|forward| B[Beta]\n  A -->|parallel| B\n  A --> B\n  \
                   B --> C[Gamma]\n  C --> A\n  B --> B\n  C -.->|long back label here| A\n  \
                   C --> D[Delta] --> E[Epsilon]\n  E --> C";
        for sc in static_and_routed(src) {
            let lanes: Vec<Vec<(f64, f64)>> = sc
                .edges
                .iter()
                .filter(|e| e.from == "C" && e.to == "A")
                .map(edge_polyline)
                .collect();
            assert_eq!(lanes.len(), 2);
            let crosses = lanes[0].windows(2).any(|w| {
                lanes[1]
                    .windows(2)
                    .any(|v| crate::lanes::tests_seg_cross(w[0], w[1], v[0], v[1]))
            });
            assert!(!crosses, "{lanes:?}");
        }
    }

    #[test]
    fn a_self_loop_sits_where_scene_draws_it_and_moves_to_a_free_side() {
        // route() drew every loop on the screen's right — in LR / RL the
        // side its node's edges leave by, where the static layout draws
        // none (x6-rl-loops: Beta's loop hid under the A -> B edges). It
        // now sits on the flow's high breadth side as in scene(), the
        // router keeps other edges off it (stress: A ==> D ran 5 px from
        // B's loop in route()), and a drag that crowds that side sends it
        // to the free one.
        let loop_box = |sc: &Scene, id: &str| {
            let p = edge_polyline(the_edge(sc, id, id));
            p.iter().fold(
                (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ),
                |r, q| (r.0.min(q.0), r.1.min(q.1), r.2.max(q.0), r.3.max(q.1)),
            )
        };
        let src = "flowchart LR\nA --> B\nB --> B\nB --> C";
        for sc in static_and_routed(src) {
            let (_, by, _, bh) = node_box(&sc, "B");
            let r = loop_box(&sc, "B");
            assert!(r.1 >= by + bh - 1.0, "below B: {r:?}");
        }
        let src = include_str!("../examples/stress.mmd");
        let [_, r] = static_and_routed(src);
        let lb = loop_box(&r, "B");
        let gap = edge_polyline(the_edge(&r, "A", "D"))
            .iter()
            .map(|&(x, y)| (lb.0 - x).max(x - lb.2).max(lb.1 - y).max(y - lb.3))
            .fold(f64::INFINITY, f64::min);
        assert!(
            gap >= crate::clearance::CRAMP_NODE_GAP,
            "A ==> D {gap:.1} px off the loop"
        );
        // A node dragged up beside B's loop: the loop moves to B's left.
        let g = parse("flowchart TD\nA --> B\nB --> B\nB --> C\nA --> D").unwrap();
        let s = scene(&g);
        let mut c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let (b, d) = (g.node_index("B").unwrap(), g.node_index("D").unwrap());
        c[d] = (
            c[b].0 + s.nodes[b].w / 2.0 + s.nodes[d].w / 2.0 + 20.0,
            c[b].1,
        );
        let r = route(&g, &c);
        let lb = loop_box(&r, "B");
        assert!(
            lb.2 <= c[b].0 - s.nodes[b].w / 2.0 + 1.0,
            "left of B: {lb:?}"
        );
    }

    #[test]
    fn a_nodes_self_loops_start_and_end_on_its_border() {
        // B3 review r5 #1: the label room a labelled parallel edge takes
        // across the flow was given to self-loops too, whose offsets run
        // ALONG their node's side — two labelled loops on Poll ended
        // 22.5 px off its border, a state's 30 px, three on Worker 85 px
        // (beside Start and Next step), heads in empty space, in scene(),
        // route() and drags alike. And three plain loops on a 38 px box
        // already overhung it by 5 px: a stack longer than the side is
        // squeezed onto it, in order.
        let off_border = |sc: &Scene, e: &SceneEdge| {
            let n = sc.nodes.iter().find(|n| n.id == e.from).unwrap();
            let p = edge_polyline(e);
            [p[0], p[p.len() - 1]]
                .iter()
                .map(|q| {
                    let dx = ((q.0 - n.x).abs() - n.w / 2.0).max(0.0);
                    let dy = ((q.1 - n.y).abs() - n.h / 2.0).max(0.0);
                    dx.hypot(dy)
                })
                .fold(0.0, f64::max)
        };
        for src in [
            "flowchart TD\nA[Start] --> B[Poll]\nB -->|timeout| B\nB -->|busy| B\nB --> C[Done]",
            "flowchart BT\nA[Start] --> B[Poll]\nB -->|timeout| B\nB -->|busy| B\nB --> C[Done]",
            "flowchart LR\nA[Start] --> B[Poll]\nB -->|timeout| B\nB -->|busy| B\nB --> C[Done]",
            "flowchart TD\nS[Start] --> A[Worker]\nA -->|retry once| A\nA -->|retry twice| A\n\
             A -->|give up later| A\nA --> B[Next step]\nS --> C[Side]\nC --> B\nB --> B",
            "flowchart TD\nA --> B\nB --> B\nB --> B\nB --> B\nB --> B\nB --> C",
            "stateDiagram-v2\n[*] --> Idle\nIdle --> Active : go\nActive --> Active : tick\n\
             Active --> Active : refresh cache\nActive --> Idle : stop\nActive --> [*]",
        ] {
            let g = match crate::parser::parse_document(src).unwrap() {
                crate::Document::Flowchart(g) | crate::Document::State(g) => g,
                _ => unreachable!(),
            };
            let s = scene(&g);
            let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let mut dragged = c.clone();
            dragged[0].0 += 37.0;
            dragged[0].1 -= 23.0;
            let r = route(&g, &c);
            let d = route(&g, &dragged);
            let p = route_partial(&g, &dragged, &s, &c);
            for (mode, sc) in [
                ("static", &s),
                ("routed", &r),
                ("dragged", &d),
                ("partial", &p),
            ] {
                for (ei, e) in g.edges.iter().enumerate() {
                    if e.from == e.to {
                        let off = off_border(sc, &sc.edges[ei]);
                        assert!(off < 0.5, "{src}\n{mode}: loop {ei} {off:.1} px off");
                    }
                }
            }
        }
        // Two loops on a box still sit where they did, 16 px each.
        let s = scene(&parse("flowchart TD\nA --> B\nB --> B\nB --> B\nB --> C").unwrap());
        let ends: Vec<f64> = s.edges[1..3]
            .iter()
            .flat_map(|e| [e.bezier[0].1, e.bezier[3].1])
            .collect();
        let y = s.nodes[1].y;
        assert_eq!(ends, [y - 16.0, y, y, y + 16.0]);
    }

    #[test]
    fn a_lane_never_meets_the_flank_its_end_loops_on() {
        // A lane met its end on the flank facing it even when that flank
        // carried the end's self-loop: the turn in ran through the loop,
        // the head stacked on the loop's own (m6: `again` into Parse;
        // w3: `reindex` into Parse; l5: `again` out of End in scene()
        // only, so the static and routed lanes left End by different
        // sides). The lane now crosses the end's flow side there, its
        // leg past the end's layer, in both modes alike.
        let srcs = [
            "flowchart BT\nA[Load]-->B[Parse]\nB-->B\nB-->C[Check]\nB-->D[Warn]\n\
             C-->E[Emit]\nD-->E\nE-->|again|B",
            "flowchart TD\nA[Start] --> B[Work]\nB --> B\nB --> C[Check]\nC --> D[End]\n\
             D --> D\nD -->|again| B\nC --> E[Extra]\nE --> D",
            "flowchart TD\nA[Ingest] --> B[Parse]\nA --> C[Enrich]\nB --> B\nC --> C\n\
             A ==> D[Store]\nB --> D\nC --> D\nD --> E[Index]\nE --> E\n\
             E -.->|reindex| B\nD -->|fix| A\nsubgraph W[Workers]\nB\nC\nend",
        ];
        let bbox = |p: &[(f64, f64)]| {
            p.iter().fold(
                (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ),
                |r, q| (r.0.min(q.0), r.1.min(q.1), r.2.max(q.0), r.3.max(q.1)),
            )
        };
        type Pt = (f64, f64);
        let cut = |p: Pt, q: Pt, r: Pt, s: Pt| {
            let side = |a: Pt, b: Pt, c: Pt| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
            side(p, q, r) * side(p, q, s) < 0.0 && side(r, s, p) * side(r, s, q) < 0.0
        };
        let crosses = |p: &[Pt], q: &[Pt]| {
            p.windows(2)
                .any(|a| q.windows(2).any(|b| cut(a[0], a[1], b[0], b[1])))
        };
        for src in srcs {
            let g = parse(src).unwrap();
            for (mode, sc) in ["static", "routed"].iter().zip(static_and_routed(src)) {
                let mut lanes = 0;
                for (ei, e) in g.edges.iter().enumerate() {
                    let looped = |v: usize| g.edges.iter().position(|f| f.from == v && f.to == v);
                    if e.from == e.to || sc.edges[ei].waypoints.len() < 3 {
                        continue;
                    }
                    let path = edge_painted_polyline(&sc.edges[ei]);
                    for v in [e.from, e.to] {
                        let Some(li) = looped(v) else { continue };
                        lanes += 1;
                        let lp = edge_painted_polyline(&sc.edges[li]);
                        let id = (&g.nodes[e.from].id, &g.nodes[e.to].id);
                        assert!(!crosses(&path, &lp), "{mode} {id:?} crosses the loop");
                        let r = bbox(&lp);
                        assert!(
                            !path.iter().any(|&(x, y)| x > r.0 + 1.0
                                && x < r.2 - 1.0
                                && y > r.1 + 1.0
                                && y < r.3 - 1.0),
                            "{mode} {id:?} inside the loop {r:?}"
                        );
                    }
                }
                assert!(lanes > 0, "{mode}: no lane at a looped end in {src}");
            }
        }
    }

    #[test]
    fn a_two_cycle_across_a_short_gap_keeps_its_twins_apart() {
        // Twins whose ends lie far apart across a short gap ran their
        // middles nearly across the flow a port spacing times the slope
        // apart: state r215's `S0 -> T0` / `T0 -> S0 : back` lay within
        // 6 px of each other for 60 px, one thick band; in r085 (LR) the
        // router took the return edge off its twin and ran it beside a
        // note link instead. The return curve now moves its middle away
        // along the flow, its ends still square. (r215 in route(): the
        // forward twin's free curve clips the frame beside it and the
        // router sends it round, across its twin, as before.)
        let srcs = [
            (
                "stateDiagram-v2\n[*] --> S0\nstate C0 {\n[*] --> C0i0\nC0i0 --> C0i1\n\
                 note right of C0i0 : n69\n}\nstate C1 {\n[*] --> C1i0\nC1i0 --> C1i1\n\
                 C1i0 --> C1i2\nnote right of C1i1 : n81\n}\nS0 --> C0\nC0 --> C1\n\
                 S0 --> T0\nnote left of T0 : m86\nnote left of S0 : m28\nT0 --> S0 : back",
                false,
            ),
            (
                "stateDiagram-v2\ndirection LR\n[*] --> S0\nstate C0 {\n[*] --> C0i0\n\
                 C0i0 --> C0i1\nC0i0 --> C0i2\nnote left of C0i2 : n28\n\
                 note left of C0i2 : n68\n}\nS0 --> C0\nS0 --> T0\nnote left of T0 : m0\n\
                 note right of S0 : m60\nnote left of S0 : m87\nT0 --> S0 : back",
                true,
            ),
        ];
        type Pt = (f64, f64);
        let seg_dist = |p: Pt, a: Pt, b: Pt| {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let l2 = dx * dx + dy * dy;
            let t = if l2 > 0.0 {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
        };
        for (src, routed) in srcs {
            let crate::model::Document::State(g) = crate::parser::parse_document(src).unwrap()
            else {
                panic!("state diagram");
            };
            let s = scene(&g);
            let c: Vec<Pt> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let mut modes = vec![("static", s.clone())];
            if routed {
                modes.push(("routed", route(&g, &c)));
            }
            for (mode, sc) in modes {
                let fwd = edge_polyline(the_edge(&sc, "S0", "T0"));
                let back = the_edge(&sc, "T0", "S0");
                assert!(back.waypoints.is_empty(), "{mode}: rerouted {back:?}");
                let skew = end_skew(back, false);
                assert!(skew <= 10.0, "{mode}: return head {skew:.1} deg");
                // The middle half of the return curve, against its twin.
                let back = edge_polyline(back);
                let n = back.len();
                let gap = back[n / 4..3 * n / 4]
                    .iter()
                    .map(|&p| {
                        fwd.windows(2)
                            .map(|w| seg_dist(p, w[0], w[1]))
                            .fold(f64::INFINITY, f64::min)
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    gap > crate::clearance::EDGE_SPACING + 2.0,
                    "{mode}: twins {gap:.1} px apart"
                );
            }
        }
    }

    #[test]
    fn only_a_lone_two_cycle_skews_its_return_curve() {
        // Among more parallel edges a return curve moved away from one
        // runs into the next (q27: `n4 ==> n0` beside three `n0 --> n4`
        // ends rerouted and swung `n0 --> n4` in at 62 deg): only the two
        // edges of a lone 2-cycle get a side.
        let sides = |src: &str| {
            let g = parse(src).unwrap();
            let back = |ei: usize| g.nodes[g.edges[ei].from].id == "B";
            twin_sides(&g, &parallel_offsets(&g), |_| true, back)
        };
        let lone = sides("flowchart TD\nA --> B\nB --> A");
        assert_eq!(lone[0], 0.0, "{lone:?}");
        assert_ne!(lone[1], 0.0, "{lone:?}");
        assert!(sides("flowchart TD\nA --> B\nB --> A\nA -.-> B")
            .iter()
            .all(|&s| s == 0.0));
    }

    #[test]
    fn diagrams_without_link_style_render_exactly_as_before() {
        // Byte-level: no styled defs, the kind's stroke attrs (report
        // Q10: 2 px, thick 3.5 px) and one head for every arrow (Q1:
        // a thick edge shares `#arrow`, sized in user space).
        let svg = svg_src("flowchart LR\nA --> B\nC -.-> D\nE ==> F\nG x--x H\n");
        assert!(!svg.contains("_"), "{svg}");
        assert!(svg.contains("stroke-dasharray=\"2 3\""));
        assert!(svg.contains("stroke-width=\"3.5\" marker-end=\"url(#arrow)\""));
        assert!(svg.contains("stroke-width=\"2\" marker-end=\"url(#arrow)\""));
        assert_eq!(svg.matches("<marker id=\"arrow\"").count(), 1, "{svg}");
        assert!(svg.contains(
            "<marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"11\" markerHeight=\"11\" \
             orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#44507a\"/></marker>"
        ));
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
                "fill=\"#4a5070\"><tspan font-style=\"italic\">Italic</tspan> title</text>"
            ),
            "{svg}"
        );
        // Plain titles are byte-identical to the old escape().
        let svg = svg_src("flowchart LR\n  subgraph s1 [a & b]\n    A\n  end\n");
        assert!(svg.contains("fill=\"#4a5070\">a &amp; b</text>"));
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
    fn a_member_edge_to_its_own_box_leads_to_the_box_entry_member() {
        // `D --> Sys` from inside Sys (the showcase's `Dev --> CI`) was
        // ranked as D --> B (B enters Sys) but drawn as a stub up to the
        // frame's top border beside the title, against the flow, with an
        // empty rank gap where D --> B ran. It is now drawn as that edge:
        // from D down onto B, in the static scene, in route() and in a
        // drag frame alike; no end sits on Sys's frame.
        for dir in ["TD", "LR", "BT", "RL"] {
            let src =
                format!("flowchart {dir}\nD --> S\nsubgraph S [Sys]\n  B --> C\n  C -.-> D\nend");
            let g = parse(&src).unwrap();
            assert_eq!(
                own_box_targets(&g),
                [g.node_index("B")],
                "{dir}: D leads to B"
            );
            let s = scene(&g);
            let pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &pos);
            let mut moved = pos.clone();
            let b = g.node_index("B").unwrap();
            let along = if matches!(dir, "TD" | "BT") {
                (24.0, 0.0)
            } else {
                (0.0, 24.0)
            };
            moved[b] = (pos[b].0 + along.0, pos[b].1 + along.1);
            let p = route_partial(&g, &moved, &r, &pos);
            for (what, sc) in [("scene", &s), ("route", &r), ("drag", &p)] {
                let e = sc
                    .edges
                    .iter()
                    .find(|e| e.from == "D" && e.to == "S")
                    .unwrap();
                let pl = edge_polyline(e);
                let (p0, p3) = (pl[0], pl[pl.len() - 1]);
                let node = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
                let on_border = |n: &SceneNode, q: (f64, f64)| {
                    let (x0, y0, w, h) = node_rect(n);
                    let (x1, y1) = (x0 + w, y0 + h);
                    let inside =
                        q.0 > x0 - 1.0 && q.0 < x1 + 1.0 && q.1 > y0 - 1.0 && q.1 < y1 + 1.0;
                    let edge = (q.0 - x0).abs().min((q.0 - x1).abs()) < 1.0
                        || (q.1 - y0).abs().min((q.1 - y1).abs()) < 1.0;
                    inside && edge
                };
                assert!(
                    on_border(node("D"), p0),
                    "{dir} {what}: starts on D: {p0:?}"
                );
                assert!(on_border(node("B"), p3), "{dir} {what}: ends on B: {p3:?}");
                let f = sc.clusters.iter().find(|c| c.id == "S").unwrap();
                let on_frame = |q: (f64, f64)| {
                    (q.0 - f.x).abs().min((q.0 - f.x - f.w).abs()) < 1.0
                        || (q.1 - f.y).abs().min((q.1 - f.y - f.h).abs()) < 1.0
                };
                assert!(
                    !on_frame(p0) && !on_frame(p3),
                    "{dir} {what}: ends on the frame"
                );
                for n in sc.nodes.iter().filter(|n| n.id == "C") {
                    assert!(!edge_hits(e, node_rect(n)), "{dir} {what}: crosses C");
                }
            }
            // Ranked as drawn: D one layer before B. (A box's first
            // layer still leaves the header slot after it, report §2.8 /
            // M8, so that gap is the others' plus at most SUB_HEADER.)
            let (d, bn, c) = (
                s.nodes.iter().find(|n| n.id == "D").unwrap(),
                s.nodes.iter().find(|n| n.id == "B").unwrap(),
                s.nodes.iter().find(|n| n.id == "C").unwrap(),
            );
            let flow = |n: &SceneNode| to_flow(g.direction, (n.x, n.y)).1;
            let gap = |a: &SceneNode, b: &SceneNode| (flow(b) - flow(a)).abs();
            assert!(flow(d) < flow(bn), "{dir}: D before B");
            assert!(
                gap(d, bn) <= gap(bn, c) + SUB_HEADER + 1.0,
                "{dir}: D..B {} vs B..C {}",
                gap(d, bn),
                gap(bn, c)
            );
        }
        // A box to its own member, or a member whose only entry it
        // already links to, keeps the stub: no line on top of another.
        let g = parse("flowchart TD\nsubgraph G\n  A --> B --> C --> A\nend\nC --> G\nG --> A")
            .unwrap();
        assert_eq!(own_box_targets(&g), [None, None]);
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

    /// Report M8 (§2.8): a box's title strip takes its room above the
    /// box's first row only — on the screen top, the layers' far side in
    /// BT — so the next row inside follows at the usual layer gap (the
    /// slot grown on both sides left a strip's worth of dead room under
    /// every first row: 86 px where any other gap is 60).
    #[test]
    fn a_title_strip_leaves_no_dead_room_under_its_first_row() {
        for dir in ["TD", "BT"] {
            let src = format!(
                "flowchart {dir}\n  C[Client] --> A\n  subgraph P [Platform]\n    \
                 A[Auth service] --> U[User DB]\n    U --> W[Worker]\n  end\n"
            );
            let sc = scene(&parse(&src).unwrap());
            let n = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
            let (top, next) = if dir == "TD" {
                (n("A"), n("U"))
            } else {
                (n("W"), n("U"))
            };
            // Down the screen from the first row to the next either way.
            let gap = (next.y - next.h / 2.0) - (top.y + top.h / 2.0);
            assert!((gap - crate::layout::GAP_L).abs() < 1e-6, "{dir}: {gap}");
            // The strip itself still sits above the first row, inside the
            // frame: its title clear of the row.
            let f = &sc.clusters[0];
            let first = if dir == "TD" { n("A") } else { n("W") };
            assert!(
                first.y - first.h / 2.0 - f.y >= SUB_HEADER + SUB_PAD - 1e-6,
                "{dir}: frame top {} over {}",
                f.y,
                first.y - first.h / 2.0
            );
        }
    }

    /// B7 review r1 (#2): an edge crossing into a box from the row above
    /// with a sideways run keeps the box's border off the run's middle by
    /// [`crate::clearance::FRAME_SEP`] (the strip no longer doubled, M8,
    /// had put it 3 px off: advanced's `Gate ==> Job Queue` rode Async
    /// Workers' top for 122 px and was rerouted into an L inside it); a
    /// straight drop into a box keeps the plain gap. In advanced, no edge
    /// cuts a foreign frame (Auth -> Users' stub into the comb on Users'
    /// lid swung it round Async Workers' corner) nor rides Async Workers'
    /// border, and `Gate ==> Job Queue` stays a curve, in either mode.
    #[test]
    fn a_run_into_a_box_crosses_its_border_clear_of_the_run() {
        let src = "flowchart TD\n  G[Gateway] --> Q\n  G --> X[Elsewhere]\n  \
                   subgraph W [Workers]\n    Q[Queue] --> A[Worker]\n  end\n";
        let sc = scene(&parse(src).unwrap());
        let n = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
        let frame = |id: &str| sc.clusters.iter().find(|c| c.id == id).unwrap();
        let (g, q) = (n("G"), n("Q"));
        assert!((g.x - q.x).abs() > 24.0, "the run into W must be sideways");
        let mid = ((g.y + g.h / 2.0) + (q.y - q.h / 2.0)) / 2.0;
        let top = frame("W").y;
        assert!(
            top - mid >= crate::clearance::FRAME_SEP - 1e-6,
            "border {top} vs run middle {mid}"
        );
        // A straight drop keeps the plain gap and the strip over it.
        let src =
            "flowchart TD\n  C[Client] --> S\n  subgraph P [Platform]\n    S[Straight]\n  end\n";
        let sc = scene(&parse(src).unwrap());
        let n = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
        let (c, st) = (n("C"), n("S"));
        assert!((c.x - st.x).abs() <= 24.0);
        let gap = (st.y - st.h / 2.0) - (c.y + c.h / 2.0);
        assert!(gap <= crate::layout::GAP_L + SUB_HEADER + 1e-6, "{gap}");

        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/advanced.mmd"
        ))
        .unwrap();
        let rep = crate::quality::measure_source(&src, false).unwrap();
        for m in [&rep.statik, &rep.routed] {
            assert_eq!(m.edge_frame_x, 0, "{:?}", m.details);
            let bad = |d: &&String| d.contains("border Workers") || d.starts_with("Gate->Q ortho");
            assert!(!m.details.iter().any(|d| bad(&d)), "{:?}", m.details);
        }
    }

    /// Report M3 (§2.6): a decision's labelled branches between adjacent
    /// layers get a gap that holds the label, the head and a stub either
    /// side; each label sits in the middle of its curve, where the S is
    /// straight, clear of the arrowhead; and each arrival lands in the
    /// middle of its target (long-labels: 43.7 px off-centre once).
    #[test]
    fn decision_labels_sit_mid_curve_clear_of_their_heads() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/long-labels.mmd"
        ))
        .unwrap();
        for sc in static_and_routed(&src) {
            for to in ["C", "D"] {
                let e = sc
                    .edges
                    .iter()
                    .find(|e| e.from == "B" && e.to == to)
                    .unwrap();
                let (b, t) = (
                    sc.nodes.iter().find(|n| n.id == "B").unwrap(),
                    sc.nodes.iter().find(|n| n.id == to).unwrap(),
                );
                let gap = (t.y - t.h / 2.0) - (b.y + b.h / 2.0);
                assert!(gap >= 72.0 - 1e-6, "B -> {to}: gap {gap}");
                let (_, m, _) = e.label.as_ref().unwrap();
                let mid = cubic_mid(e.bezier[0], e.bezier[1], e.bezier[2], e.bezier[3]);
                // (Within a step of the label stage's spots along it.)
                assert!(
                    (m.0 - mid.0).hypot(m.1 - mid.1) <= 6.0,
                    "B -> {to}: label at {m:?}, curve middle {mid:?}"
                );
                let tip = e.bezier[3];
                let clear = tip.1 - (m.1 + LABEL_BOX_H / 2.0);
                assert!(
                    clear >= ARROW_MARKER_PX + crate::layout::LABEL_CLEAR,
                    "B -> {to}: {clear} px from label to tip"
                );
                assert!((tip.0 - t.x).abs() < 5.0, "B -> {to}: {} vs {}", tip.0, t.x);
            }
        }
    }

    /// B7 review r1 (#6, report M3): a hub's labelled branches across one
    /// gap get a gap deep enough that each label, centred on its curve's
    /// middle, clears the fan's other curves; the label sits there, not
    /// in the half near the head (`express` sat 5.7 px off its head's
    /// base, on the bend, where at the middle its box met `Pack`'s
    /// curve). In both modes.
    #[test]
    fn a_hub_s_labels_sit_mid_curve_clear_of_their_fan_and_heads() {
        let src = "flowchart TD\n  A[Order received] --> B[Pack]\n  A -->|express| C[Ship now]\n  \
                   A --> D[Invoice]\n  A -->|gift wrap| E[Wrap]\n  B --> F[Done]\n  C --> F\n";
        for sc in static_and_routed(src) {
            for to in ["C", "E"] {
                let e = sc
                    .edges
                    .iter()
                    .find(|e| e.from == "A" && e.to == to)
                    .unwrap();
                let (_, m, w) = e.label.clone().unwrap();
                let mid = cubic_mid(e.bezier[0], e.bezier[1], e.bezier[2], e.bezier[3]);
                assert!(
                    (m.0 - mid.0).hypot(m.1 - mid.1) <= 6.0,
                    "A -> {to}: label at {m:?}, curve middle {mid:?}"
                );
                let clear = e.bezier[3].1 - (m.1 + LABEL_BOX_H / 2.0);
                assert!(
                    clear >= ARROW_MARKER_PX + crate::layout::LABEL_CLEAR,
                    "A -> {to}: {clear} px from label to tip"
                );
                // No other stroke of the fan runs through the label.
                let r = (
                    m.0 - w / 2.0,
                    m.1 - LABEL_BOX_H / 2.0,
                    m.0 + w / 2.0,
                    m.1 + LABEL_BOX_H / 2.0,
                );
                for f in sc.edges.iter().filter(|f| f.from == "A" && f.to != to) {
                    let pts = edge_polyline(f);
                    assert!(
                        pts.iter()
                            .all(|p| p.0 < r.0 || p.0 > r.2 || p.1 < r.1 || p.1 > r.3),
                        "A -> {to}'s label is crossed by A -> {}",
                        f.to
                    );
                }
            }
        }
        // Nor does the harness find a label on another stroke.
        let rep = crate::quality::measure_source(src, false).unwrap();
        assert_eq!((rep.statik.lbl_edge, rep.routed.lbl_edge), (0, 0));
    }

    #[test]
    fn lr_colliding_labels_grow_their_gap_only_to_the_widest_one() {
        // Labels share the gap after `laptop`, and their spans collide
        // across the flow. They slide apart across it, so the gap needs
        // room for the widest label — not for a row of labels laid side
        // by side along the flow (which once more than doubled it, all
        // of it empty). STANDBY, fed from `laptop`, sits in the column
        // after it with its source `tunnel`, and PROD after STANDBY,
        // whose `tunnel` leads into it: mermaid's three columns (B8b
        // P1; `tunnel` used to sit level with `laptop`, its STANDBY
        // frame spanning `laptop`'s column).
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
        let frame = |id: &str| sc.clusters.iter().find(|c| c.id == id).unwrap();
        let laptop = node("laptop");
        let (standby, prod) = (frame("standby"), frame("prod"));
        assert!(
            laptop.x + laptop.w / 2.0 < standby.x,
            "STANDBY after laptop"
        );
        assert!(standby.x + standby.w < prod.x, "PROD after STANDBY");
        let gap = (laptop.x + laptop.w / 2.0, standby.x);
        let widest = sc
            .edges
            .iter()
            .filter_map(|e| e.label.as_ref())
            .filter(|l| gap.0 < l.1 .0 && l.1 .0 < gap.1)
            .map(|l| l.2)
            .fold(0.0, f64::max);
        assert!(widest > 0.0, "a label in the gap");
        assert!(
            gap.1 - gap.0 >= widest,
            "gap {gap:?} cannot hold a {widest} label"
        );
        // Plus the arrowhead the label keeps clear of (report M3).
        assert!(
            gap.1 - gap.0 <= widest + 2.0 * LABEL_BOX_H + ARROW_MARKER_PX,
            "gap {gap:?} for {widest}"
        );
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

    /// Composite-and-note state diagrams for
    /// [`note_links_match_between_scene_and_route`]: the verifiers'
    /// repros plus ones generated from a fixed seed.
    fn composite_note_sources() -> Vec<String> {
        let mut out: Vec<String> = [
            "stateDiagram-v2\ndirection LR\n[*] --> Outer\nstate Outer {\n  direction TB\n  \
             [*] --> I1\n  state c <<choice>>\n  I1 --> c\n  c --> I2 : ok\n  c --> I3 : fail\n  \
             c --> I2 : retry\n  note right of I1 : inner note\n  state Deep {\n    \
             [*] --> D1\n    D1 --> D2\n    note left of D2 : deep note\n  }\n  I3 --> Deep\n}\n\
             Outer --> Done\nnote right of Outer : outer note\nnote left of Done : done note\n\
             Done --> [*]",
            "stateDiagram-v2\ndirection LR\n[*] --> S0\nstate C0 {\n  [*] --> C0i0\n  \
             C0i0 --> C0i1\n  C0i0 --> C0i2\n  C0i0 --> C0i3\n  note right of C0i0 : n62\n  \
             note left of C0i3 : n68\n}\nS0 --> C0\nC0 --> T0\nS0 --> T1\n\
             note right of S0 : m14\nnote right of T1 : m82\nnote right of T1 : m39\n\
             T1 --> S0 : back",
            "stateDiagram-v2\ndirection LR\n[*] --> A\nA --> B : go\nA --> C : stop\n\
             note right of A : first note\nnote right of A : second note",
            "stateDiagram-v2\ndirection LR\n[*] --> A\nA --> B : go\nB --> A : back\n\
             note right of A : first note\nnote right of A : second note",
            "stateDiagram-v2\ndirection LR\n[*] --> A\nA --> B : go\nB --> A : back\n\
             note left of B : first note\nnote left of B : second note",
            "stateDiagram-v2\n[*] --> A\nA --> B\nA --> B\nA --> B : one\nA --> B : one\n\
             B --> A\nB --> A : back\nnote right of A : dup\nnote right of A : dup again",
            "stateDiagram-v2\ndirection LR\n[*] --> S0\nstate C0 {\n  [*] --> C0i0\n  \
             C0i0 --> C0i1 : e1\n  state C0ch <<choice>>\n  C0i0 --> C0ch\n  \
             C0ch --> C0i1 : y\n  C0ch --> C0i1 : n\n  note left of C0i0 : n924\n  \
             note left of C0i1 : n552\n  note right of C0i1 : n118\n}\nstate C1 {\n  \
             [*] --> C1i0\n  state C1ch <<choice>>\n  C1i0 --> C1ch\n  C1ch --> C1i0 : y\n  \
             C1ch --> C1i0 : n\n  note right of C1i0 : n856\n}\nS0 --> C0\nS0 --> C1 : go2\n\
             S0 --> S0 : back\nC0 --> C0 : self\nnote left of C1 : m383\nnote left of C1 : m445\n\
             note right of S0 : m343",
        ]
        .map(String::from)
        .to_vec();
        let mut seed: u64 = 0x5eed_f10e;
        let mut rnd = |n: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % n
        };
        for _ in 0..60 {
            let mut src = String::from("stateDiagram-v2\n");
            if rnd(2) == 0 {
                src.push_str("direction LR\n");
            }
            let mut outer: Vec<String> = Vec::new();
            src.push_str("[*] --> S0\n");
            outer.push("S0".into());
            for c in 0..1 + rnd(2) {
                let inner: Vec<String> = (0..2 + rnd(3)).map(|i| format!("C{c}i{i}")).collect();
                src.push_str(&format!("state C{c} {{\n  [*] --> {}\n", inner[0]));
                for i in 1..inner.len() {
                    let from = &inner[rnd(i as u64) as usize];
                    src.push_str(&format!("  {from} --> {}\n", inner[i]));
                }
                for _ in 0..rnd(3) {
                    let side = ["left", "right"][rnd(2) as usize];
                    let at = &inner[rnd(inner.len() as u64) as usize];
                    src.push_str(&format!("  note {side} of {at} : n{}\n", rnd(100)));
                }
                src.push_str("}\n");
                outer.push(format!("C{c}"));
            }
            for t in 0..1 + rnd(3) {
                outer.push(format!("T{t}"));
            }
            for k in 1..outer.len() {
                let from = &outer[rnd(k as u64) as usize];
                src.push_str(&format!("{from} --> {}\n", outer[k]));
            }
            for _ in 0..1 + rnd(3) {
                let side = ["left", "right"][rnd(2) as usize];
                let at = &outer[rnd(outer.len() as u64) as usize];
                src.push_str(&format!("note {side} of {at} : m{}\n", rnd(100)));
            }
            if rnd(2) == 0 {
                let (a, b) = (rnd(outer.len() as u64), rnd(outer.len() as u64));
                src.push_str(&format!(
                    "{} --> {} : back\n",
                    outer[a.max(b) as usize],
                    outer[a.min(b) as usize]
                ));
            }
            out.push(src);
        }
        out
    }

    /// Whether `e` is drawn as [`note_elbow`]'s quarter-ellipse: one
    /// control point straight across from one end, the other straight
    /// along from the other (a plain curve leaves both ends along the
    /// same axis). The ends may have been spread apart since, so the
    /// controls' distances are not checked.
    fn elbow_of(e: &SceneEdge) -> bool {
        let [p0, c1, c2, p3] = e.bezier;
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        e.waypoints.is_empty()
            && !near(p0.0, p3.0)
            && !near(p0.1, p3.1)
            && ((near(c1.0, p0.0) && !near(c1.1, p0.1) && near(c2.1, p3.1) && !near(c2.0, p3.0))
                || (near(c1.1, p0.1) && !near(c1.0, p0.0) && near(c2.0, p3.0) && !near(c2.1, p3.1)))
    }

    /// Verifier leftovers (B7): a note card's link whose node side also
    /// holds another edge's end — a router path one way, none the other —
    /// had its elbow end spread 7 px off in `scene()` and not in
    /// `route()` (`spread-face`, `spread-bottom`); and a link the router
    /// drew had 36 px corners in `scene()`, 14 px in `route()`
    /// (`nested-composite`). The link end is a pin to the ports stage and
    /// a link keeps the router's corners, so both draw every link alike
    /// from the same centres, and a 1 px drag of a node no link ends on
    /// moves none of them. `routed-around` keeps a link the router draws
    /// round the nested composite's other state (`dn290`) in the set,
    /// as `nested-composite`'s `in458` link no longer is (B8b P5).
    #[test]
    fn note_links_hold_their_ends_and_corners_in_scene_route_and_drag() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/notes");
        for name in [
            "spread-face",
            "spread-bottom",
            "nested-composite",
            "straight-link",
            "wide-frames",
            "routed-around",
        ] {
            let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
            let g = match crate::parser::parse_document(&src).unwrap() {
                crate::model::Document::State(g) => g,
                other => panic!("not a state diagram: {other:?}"),
            };
            let s = scene(&g);
            let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &at);
            let links: Vec<usize> = (0..g.edges.len())
                .filter(|&ei| {
                    let e = &g.edges[ei];
                    is_note_card(&s.nodes[e.from]) || is_note_card(&s.nodes[e.to])
                })
                .collect();
            let same = |a: &SceneEdge, b: &SceneEdge| {
                let close = |p: &(f64, f64), q: &(f64, f64)| {
                    (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6
                };
                a.waypoints.len() == b.waypoints.len()
                    && a.waypoints
                        .iter()
                        .zip(&b.waypoints)
                        .all(|(p, q)| close(p, q))
                    && (!a.waypoints.is_empty()
                        || a.bezier.iter().zip(&b.bezier).all(|(p, q)| close(p, q)))
            };
            let mut held = 0;
            for &ei in &links {
                let (a, b) = (&s.edges[ei], &r.edges[ei]);
                // An elbow, or the nested composite's link the router
                // drew round its box (`in458`, whose corners differed;
                // other router paths may take another course in
                // `route()`, whose search is a drag frame's): one and
                // the same.
                let card = |id: &str| {
                    s.nodes
                        .iter()
                        .find(|n| n.id == id)
                        .map(|n| n.label.as_str())
                };
                // (Only while the router draws it: since the cycle
                // breakers take the walk reversing fewer transitions, B8
                // review r1 #3, the composites read left to right and
                // the link is a plain curve, which `scene()` and
                // `route()` bend as they bend any free curve.)
                let routed = !a.waypoints.is_empty()
                    && match name {
                        "nested-composite" => {
                            card(&a.from) == Some("in458") || card(&a.to) == Some("in458")
                        }
                        "routed-around" => {
                            card(&a.from) == Some("dn290") || card(&a.to) == Some("dn290")
                        }
                        _ => false,
                    };
                if name == "routed-around"
                    && (card(&a.from) == Some("dn290") || card(&a.to) == Some("dn290"))
                {
                    assert!(
                        routed && !elbow_of(a),
                        "routed-around: dn290's link is no router path"
                    );
                }
                // A straight link (B7 review r1 #4, `straight-link`: a
                // comb kept in `scene()` and not in `route()` pushed its
                // pinned end 4.2 px aside): the same line.
                let straight = a.waypoints.is_empty()
                    && (a.bezier[0].0 - a.bezier[3].0).abs() < 1e-6
                    && a.bezier.iter().all(|p| (p.0 - a.bezier[0].0).abs() < 1e-6);
                if straight {
                    assert!(
                        b.waypoints.is_empty()
                            && b.bezier.iter().all(|p| (p.0 - a.bezier[0].0).abs() < 1e-6),
                        "{name} {} -> {}: {:?} vs {:?}",
                        a.from,
                        a.to,
                        a.bezier,
                        b.bezier
                    );
                    held += 1;
                } else if elbow_of(a) || routed {
                    assert!(
                        same(a, b),
                        "{name} {} -> {}: {:?} {:?}\nvs {:?} {:?}",
                        a.from,
                        a.to,
                        a.bezier,
                        a.waypoints,
                        b.bezier,
                        b.waypoints
                    );
                    held += 1;
                }
            }
            assert!(held >= 1, "{name}: no link compared");
            // A 1 px drag of any node, card or state, its frames redrawn
            // or the whole drawing handed to `route()` (B7 review r1 #4:
            // `wide-frames`' composites are drawn wider than `route()`
            // draws them, so every drag of it is a full `route()`): a
            // link neither end of which moved stays as drawn, and one
            // whose card or state moved follows it by that 1 px.
            for v in 0..g.nodes.len() {
                let mut moved = at.clone();
                moved[v].0 += 1.0;
                let p = route_partial(&g, &moved, &s, &at);
                for &ei in &links {
                    let e = &g.edges[ei];
                    let (a, b) = (&s.edges[ei], &p.edges[ei]);
                    if e.from != v && e.to != v {
                        assert!(
                            same(a, b),
                            "{name}: dragging {} moved {} -> {}",
                            g.nodes[v].id,
                            a.from,
                            a.to
                        );
                        continue;
                    }
                    let pts = |e: &SceneEdge| -> Vec<(f64, f64)> {
                        e.bezier.iter().chain(&e.waypoints).copied().collect()
                    };
                    let (pa, pb) = (pts(a), pts(b));
                    assert_eq!(pa.len(), pb.len(), "{name}: {} -> {}", a.from, a.to);
                    for (x, y) in pa.iter().zip(&pb) {
                        assert!(
                            (x.0 - y.0).abs() <= 1.0 + 1e-6 && (x.1 - y.1).abs() < 1e-6,
                            "{name}: dragging {} threw {} -> {}: {x:?} -> {y:?}",
                            g.nodes[v].id,
                            a.from,
                            a.to
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn note_links_match_between_scene_and_route() {
        // The static layout once judged a note link's elbow against the
        // chords of `scene_clustered`'s never-drawn stand-ins while
        // `route` used the real edges only: the link jumped (60-80px) to
        // another course on a composite diagram's first drag. Both now
        // judge it against the same lines from the same centres.
        let mut elbows = 0;
        for src in composite_note_sources() {
            let g = match crate::parser::parse_document(&src).unwrap() {
                crate::model::Document::State(g) => g,
                other => panic!("not a state diagram: {other:?}"),
            };
            let s = scene(&g);
            let centres: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &centres);
            for (ei, e) in g.edges.iter().enumerate() {
                if !(is_note_card(&s.nodes[e.from]) || is_note_card(&s.nodes[e.to])) {
                    continue;
                }
                let (a, b) = (&s.edges[ei], &r.edges[ei]);
                assert_eq!(
                    elbow_of(a),
                    elbow_of(b),
                    "{} -> {}: {:?} vs {:?}\n{src}",
                    a.from,
                    a.to,
                    a.bezier,
                    b.bezier
                );
                if elbow_of(a) {
                    elbows += 1;
                    for (p, q) in a.bezier.iter().zip(&b.bezier) {
                        assert!(
                            (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6,
                            "{} -> {}: {:?} vs {:?}\n{src}",
                            a.from,
                            a.to,
                            a.bezier,
                            b.bezier
                        );
                    }
                }
            }
        }
        // 67 elbows; 56 while a sub-edge's course ran on past its box's
        // frame to the members' centre. 63 since B3: a 2-cycle's return
        // edge is one alignment link with its twin, so a note beside one
        // of its states lands where its elbow would cross the return
        // edge drawn across the gap, and the link keeps its plain course.
        // 61 since B9: the first source's `Outer` composite sets
        // `direction TB` in an LR diagram and nothing links out of it, so
        // it is laid out top-down as a block of its own; 'deep note' now
        // sits level with D2 beside the TB run into it, and the LR rest
        // puts 'done note' level with Done — both links run straight.
        assert!(
            elbows >= 61,
            "only {elbows} elbows: the sources lost their point"
        );
        // A sub-edge's course stops at its box's frame: run on to the
        // centre of the box's members, it once refused elbows inside the
        // box that no drawn line crosses ('inner note' in the nested
        // composite, `C1i0`'s card under its choice loop).
        for (k, state, text) in [(0, "I1", "inner note"), (6, "C1i0", "n856")] {
            let src = &composite_note_sources()[k];
            let g = match crate::parser::parse_document(src).unwrap() {
                crate::model::Document::State(g) => g,
                other => panic!("not a state diagram: {other:?}"),
            };
            let (s, ei) = (
                scene(&g),
                (0..g.edges.len()).find(|&ei| {
                    let e = &g.edges[ei];
                    [(e.from, e.to), (e.to, e.from)]
                        .iter()
                        .any(|&(a, b)| g.nodes[a].id == state && g.nodes[b].label == text)
                }),
            );
            let ei = ei.unwrap_or_else(|| panic!("no {state} - {text} link\n{src}"));
            let centres: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &centres);
            assert!(
                elbow_of(&s.edges[ei]) && elbow_of(&r.edges[ei]),
                "{state} - {text}: {:?} vs {:?}\n{src}",
                s.edges[ei].bezier,
                r.edges[ei].bezier
            );
        }
        // Two cards right of one state: the second's link no longer
        // blocks the first's elbow (it used to keep the plain course).
        let src = &composite_note_sources()[2];
        let g = match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::State(g) => g,
            other => panic!("not a state diagram: {other:?}"),
        };
        let s = scene(&g);
        let links: Vec<&SceneEdge> = s.edges[..g.edges.len()]
            .iter()
            .filter(|e| e.from == "A" && s.nodes.iter().any(|n| n.id == e.to && is_note_card(n)))
            .collect();
        assert_eq!(links.len(), 2, "{src}");
        assert!(links.iter().all(|e| elbow_of(e)), "{links:?}");
    }

    /// The `.mmd` fixtures every scene-wide test walks.
    fn fixture_files() -> Vec<std::path::PathBuf> {
        let root = env!("CARGO_MANIFEST_DIR");
        let mut files: Vec<std::path::PathBuf> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        files
    }

    /// Why `sc`'s edge geometry is unusable — a non-finite bezier
    /// point, waypoint, label spot or label width, or a non-finite
    /// number written into `svg` — or `None` when all of it is finite.
    fn non_finite(sc: &Scene, svg: &str) -> Option<String> {
        let ok = |p: &(f64, f64)| p.0.is_finite() && p.1.is_finite();
        for (i, e) in sc.edges.iter().enumerate() {
            let label_ok = e
                .label
                .as_ref()
                .map_or(true, |(_, m, w)| ok(m) && w.is_finite());
            if !(e.bezier.iter().all(ok) && e.waypoints.iter().all(ok) && label_ok) {
                return Some(format!("edge {i} {}->{}: {e:?}", e.from, e.to));
            }
        }
        // Rust writes NaN and the infinities as `NaN` / `inf` / `-inf`.
        svg.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
            .find(|t| t.parse::<f64>().is_ok_and(|v| !v.is_finite()))
            .map(|t| format!("svg writes {t:?}"))
    }

    /// Report M7, the host contract: an embedder that measures text with
    /// its own font sizes its nodes with `intrinsic_size_wrapped` and
    /// hands the same measure to `scene_sized_with` / `route_sized_with`,
    /// and every label line it gets back is one its box was sized for —
    /// with the built-in estimate's lines, a 15 % wider font's boxes
    /// (two lines tall) got one 218 px line into 124 px of room.
    #[test]
    fn an_embedders_measure_gives_the_lines_its_boxes_hold() {
        use crate::layout::{intrinsic_size_wrapped, wrap_label, BASE_H, PAD_X};
        let labels = [
            "Escalate the ticket to level two",
            "Notify the account manager",
            "Send the invoice to customer",
            "Check stock in the warehouse",
        ];
        let mut src = String::from("flowchart TD\n");
        for (i, l) in labels.iter().enumerate() {
            src += &format!("N{i}[{l}]\n");
        }
        src += "N0 --> Q{Is the payment valid for this order?} --> N1\nN2 --> N3\n";
        let g = parse(&src).unwrap();
        let ww = g.wrapping_width().unwrap();
        let real = |s: &str| text_width(s) * 1.15;
        let sizes: Vec<(f64, f64)> = g
            .nodes
            .iter()
            .map(|n| intrinsic_size_wrapped(n, Some(ww), real))
            .collect();
        let sc = scene_sized_with(&g, &sizes, real);
        let centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route_sized_with(&g, &centers, &sizes, real);
        for s in [&sc, &rt] {
            for (n, m) in s.nodes.iter().zip(&g.nodes) {
                assert_eq!(n.label, wrap_label(m, ww, real), "{}", n.id);
                let widest = n.label.lines().map(real).fold(0.0, f64::max);
                let lines = n.label.lines().count() as f64;
                match n.shape {
                    Shape::Rect => {
                        assert!(widest + 2.0 * PAD_X <= n.w + 1e-9, "{}", n.id);
                        assert_eq!(n.h, BASE_H + (lines - 1.0) * LINE_H, "{}", n.id);
                    }
                    Shape::Diamond => {
                        assert!(lines > 1.0);
                        assert!(((widest + 24.0) * 1.6 - n.w).abs() < 1e-9, "{}", n.id);
                    }
                    _ => {}
                }
            }
        }
        // The built-in estimate breaks a 15 % wider font's labels in
        // other places: its lines do not fit these boxes.
        let est = scene_sized(&g, &sizes);
        assert!(est
            .nodes
            .iter()
            .any(|n| n.label.lines().map(real).fold(0.0, f64::max) + 2.0 * PAD_X > n.w + 1.0));
    }

    #[test]
    fn routing_stays_finite_for_any_node_sizes_and_coincident_centres() {
        // A host may pass sizes it does not know yet (0 × 0), a
        // collapsed node, or bad input (negative, NaN, infinite), and
        // may drop nodes onto one another. Every public routing entry
        // point must still hand back finite geometry: a zero-size box
        // is a point its edges meet at its centre (a 0 × 0 diamond
        // used to divide 0 by 0 on the way to its border).
        let mut graphs = 0;
        for f in &fixture_files() {
            let src = std::fs::read_to_string(f).unwrap();
            use crate::model::Document;
            let check = |what: &str, sc: &Scene, svg: &str| {
                if let Some(why) = non_finite(sc, svg) {
                    panic!("{f:?} {what}: {why}");
                }
            };
            match crate::parser::parse_document(&src) {
                Ok(Document::Flowchart(g) | Document::State(g)) => {
                    graphs += 1;
                    let base = scene(&g);
                    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let own: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.w, n.h)).collect();
                    let n = at.len();
                    let size_sets: [(&str, Vec<(f64, f64)>); 5] = [
                        ("all zero", vec![(0.0, 0.0); n]),
                        (
                            "some zero",
                            (0..n)
                                .map(|i| match i % 3 {
                                    0 => (0.0, 0.0),
                                    1 => (own[i].0, 0.0),
                                    _ => own[i],
                                })
                                .collect(),
                        ),
                        ("negative", own.iter().map(|&(w, h)| (-w, -h)).collect()),
                        (
                            "non-finite",
                            (0..n)
                                .map(|i| match i % 3 {
                                    0 => (f64::NAN, f64::NAN),
                                    1 => (f64::INFINITY, own[i].1),
                                    _ => (own[i].0, f64::NEG_INFINITY),
                                })
                                .collect(),
                        ),
                        ("huge", vec![(1e6, 1e6); n]),
                    ];
                    for (name, sizes) in &size_sets {
                        let sc = route_sized(&g, &at, sizes);
                        check(&format!("route_sized {name}"), &sc, &to_svg(&sc));
                    }
                    // The static layout cleans sizes the same way.
                    for (name, sizes) in [&size_sets[0], &size_sets[3]] {
                        let sc = scene_sized(&g, sizes);
                        check(&format!("scene_sized {name}"), &sc, &to_svg(&sc));
                    }
                    // Every second node dropped onto its predecessor, with
                    // and without sizes; then everything on one point.
                    let pairs: Vec<(f64, f64)> = (0..n).map(|i| at[i - i % 2]).collect();
                    let sc = route_sized(&g, &pairs, &size_sets[0].1);
                    check("coincident pairs, zero sizes", &sc, &to_svg(&sc));
                    let one = vec![(40.0, 40.0); n];
                    let sc = route(&g, &one);
                    check("route, all on one point", &sc, &to_svg(&sc));
                    let sc = route_partial(&g, &pairs, &base, &at);
                    check("route_partial, coincident pairs", &sc, &to_svg(&sc));
                }
                Ok(Document::Er(d)) => {
                    graphs += 1;
                    let n = d.entities.len();
                    let es = crate::er::route(&d, &vec![(40.0, 40.0); n]);
                    check(
                        "er::route, all on one point",
                        &es.scene,
                        &crate::er::to_svg(&es),
                    );
                }
                Ok(Document::Class(d)) => {
                    graphs += 1;
                    let n = d.classes.len() + d.notes.len();
                    let cs = crate::class::route(&d, &vec![(40.0, 40.0); n]);
                    check(
                        "class::route, all on one point",
                        &cs.scene,
                        &crate::class::to_svg(&cs),
                    );
                }
                _ => {}
            }
        }
        assert!(graphs >= 15, "only {graphs} graphs checked");
        // The per-edge helper too, for every pairing of size kinds and
        // for coincident centres.
        let sizes = [
            (0.0, 0.0),
            (0.0, 30.0),
            (-5.0, 20.0),
            (f64::NAN, f64::INFINITY),
            (80.0, 40.0),
        ];
        for &a in &sizes {
            for &b in &sizes {
                for (bc, self_loop) in [
                    ((0.0, 0.0), false),
                    ((0.0, 90.0), false),
                    ((0.0, 0.0), true),
                ] {
                    let c = box_edge_bezier((0.0, 0.0), a, bc, b, 6.0, self_loop);
                    assert!(
                        c.iter().all(|p| p.0.is_finite() && p.1.is_finite()),
                        "{a:?} {b:?} {bc:?} {self_loop}: {c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn valid_sizes_route_exactly_as_before() {
        // Sizes are only cleaned when invalid: valid ones (zero
        // included) are used exactly as given, so a host passing the
        // intrinsic sizes gets the very scene `route` draws.
        let g = parse("flowchart TD\nA{ok?} -->|yes| B\nA --> C((c))\nB --> D[(db)]").unwrap();
        let s = scene(&g);
        let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let own: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.w, n.h)).collect();
        assert_eq!(to_svg(&route_sized(&g, &at, &own)), to_svg(&route(&g, &at)));
        assert!(matches!(sane_sizes(&own), Cow::Borrowed(_)));
        let bad = [(f64::NAN, 3.0), (-1.0, f64::INFINITY), (0.0, 2.0)];
        assert_eq!(&*sane_sizes(&bad), &[(0.0, 3.0), (0.0, 0.0), (0.0, 2.0)]);
        // A zero-size diamond is a point: its edges meet at its centre.
        let mut sizes = own.clone();
        sizes[0] = (0.0, 0.0);
        let sc = route_sized(&g, &at, &sizes);
        for e in sc.edges.iter().filter(|e| e.from == "A") {
            assert!(
                (e.bezier[0].0 - at[0].0).hypot(e.bezier[0].1 - at[0].1) < 1e-9,
                "{e:?}"
            );
        }
    }

    /// A marquee probe beside a painted curve: the square of half side
    /// `h` whose nearest side faces `p` from `dist` away along the unit
    /// normal `n` (so the square is within `dist + h` of `p` and no
    /// nearer than `dist`).
    fn square_beside(p: (f64, f64), n: (f64, f64), dist: f64, h: f64) -> [f64; 4] {
        let c = (
            p.0 + n.0 * (dist + h * (n.0.abs() + n.1.abs())),
            p.1 + n.1 * (dist + h * (n.0.abs() + n.1.abs())),
        );
        [c.0 - h, c.1 - h, c.0 + h, c.1 + h]
    }

    #[test]
    fn marquee_selects_an_edge_once_its_painted_stroke_touches() {
        // hits_in_rect used to test the rectangle against edge_polyline,
        // the sampled centre line: a marquee that covered only the
        // stroke beside it missed, and on a long gentle bend the chords
        // cut up to ~2 units inside the drawn curve. Now it measures
        // the painted curve, widened by half its stroke width. For every
        // painted edge of curvy fixtures, a small square just inside the
        // stroke (on either side of the line) selects it, and one a unit
        // clear of the whole stroke does not.
        let root = env!("CARGO_MANIFEST_DIR");
        let mut sources: Vec<String> = ["dense-td", "bench-75", "back-edge"]
            .iter()
            .map(|f| {
                std::fs::read_to_string(format!("{root}/tests/fixtures/quality/{f}.mmd")).unwrap()
            })
            .collect();
        sources.push(
            "flowchart LR\nA --> B\nA e1@--> C\nB ==> D\nC -.-> D\nD --> A\n\
             e1@{ curve: natural }\nlinkStyle 4 stroke-width:6px"
                .to_string(),
        );
        let (mut touched, mut cleared, mut chord_misses) = (0, 0, 0);
        for src in &sources {
            let g = parse(src).unwrap();
            let s = scene(&g);
            for (i, e) in s.edges.iter().enumerate() {
                if !edge_drawn(e) {
                    continue;
                }
                let half = edge_stroke_width(e) / 2.0;
                // The curve as painted: the path `d` itself, densely
                // sampled (written to 0.1, so up to ~0.07 off).
                let pts = path_d_points(&edge_path_d(e, |p| p), 0.5);
                let gap = |r: &[f64; 4]| {
                    pts.iter()
                        .map(|&p| PickTarget::box_gap(p, r[0], r[1], r[2], r[3]))
                        .fold(f64::INFINITY, f64::min)
                };
                let chord = edge_polyline(e);
                for k in (2..pts.len().saturating_sub(2)).step_by(7) {
                    let (a, b) = (pts[k - 2], pts[k + 2]);
                    let len = (b.0 - a.0).hypot(b.1 - a.1);
                    if len < 1e-6 {
                        continue;
                    }
                    for side in [1.0, -1.0] {
                        let n = (-(b.1 - a.1) / len * side, (b.0 - a.0) / len * side);
                        // 0.15 inside the stroke's edge (past the SVG
                        // rounding), a 0.1 square.
                        let r = square_beside(pts[k], n, half - 0.15, 0.05);
                        let hits = s.hits_in_rect(r[0], r[1], r[2], r[3]);
                        assert!(
                            hits.contains(&Hit::Edge(i)),
                            "{src}\nedge {i}: square {r:?} touching its stroke missed"
                        );
                        touched += 1;
                        if !chord
                            .windows(2)
                            .any(|w| seg_aabb(w[0], w[1], r[0], r[1], r[2], r[3]))
                        {
                            chord_misses += 1; // what the old test answered
                        }
                        // A unit square a unit clear of the stroke: only
                        // where no other part of the curve comes nearer.
                        let r = square_beside(pts[k], n, half + 1.0, 0.5);
                        if gap(&r) - half < 1.0 - 0.1 {
                            continue;
                        }
                        let hits = s.hits_in_rect(r[2], r[3], r[0], r[1]); // unordered
                        assert!(
                            !hits.contains(&Hit::Edge(i)),
                            "{src}\nedge {i}: square {r:?} clear of its stroke selected it"
                        );
                        cleared += 1;
                    }
                }
            }
        }
        assert!(
            touched > 500 && cleared > 500,
            "{touched} / {cleared} probes"
        );
        assert!(chord_misses * 2 > touched, "{chord_misses} of {touched}");
    }

    #[test]
    fn er_and_class_edges_are_picked_at_the_width_they_are_painted() {
        // The class painter strokes relationships 1.6 wide and note
        // links 1.2, the ER painter its lines LINE_W wide; picking used
        // to assume the flowchart's 1.7 for all of them. The painted
        // `stroke-width` and the picked one are now one value.
        let src = "classDiagram\nAnimal <|-- Duck\nAnimal ..> Food\nnote for Duck \"quacks\"";
        let d = match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::Class(d) => d,
            _ => unreachable!(),
        };
        let cs = crate::class::scene(&d);
        let svg = crate::class::to_svg(&cs);
        let fitted = fit_top_left(&cs.scene);
        let mut widths = Vec::new();
        for (j, e) in fitted.edges.iter().enumerate() {
            let w = edge_stroke_width(e);
            let painted = format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{EDGE_COLOR}\" stroke-width=\"{w}\"",
                edge_path_d(e, |p| p)
            );
            assert!(
                svg.contains(&painted),
                "edge {j} not painted {w} wide:\n{svg}"
            );
            widths.push(w);
        }
        assert_eq!(widths, [1.6, 1.6, 1.2]);
        // Just inside the note link's 1.2 stroke picks it at zero
        // tolerance; just outside (still inside a 1.7 one) does not.
        let s = &cs.scene;
        let link = s.edges.len() - 1;
        let pts = edge_polyline(&s.edges[link]);
        let k = pts.len() / 2;
        let (a, b) = (pts[k - 1], pts[k + 1]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let n = (-(b.1 - a.1) / len, (b.0 - a.0) / len);
        let at = |off: f64| (pts[k].0 + n.0 * off, pts[k].1 + n.1 * off);
        let inside = at(0.55);
        assert_eq!(s.edge_at(inside.0, inside.1, 0.0), Some(link));
        let outside = at(0.7);
        assert_eq!(s.edge_at(outside.0, outside.1, 0.0), None);
        let r = square_beside(pts[k], n, 0.7, 0.05);
        assert!(!s
            .hits_in_rect(r[0], r[1], r[2], r[3])
            .contains(&Hit::Edge(link)));
        // ER: every relationship line is picked LINE_W wide, as painted.
        let src = "erDiagram\nCUSTOMER ||--o{ ORDER : places\nORDER }|..|{ ITEM : has";
        let d = match crate::parser::parse_document(src).unwrap() {
            crate::model::Document::Er(d) => d,
            _ => unreachable!(),
        };
        let es = crate::er::scene(&d);
        let svg = crate::er::to_svg(&es);
        for e in &fit_top_left(&es.scene).edges {
            let w = edge_stroke_width(e);
            assert_eq!(w, 1.7);
            assert!(svg.contains(&format!(
                "{}\" fill=\"none\" stroke=\"{EDGE_COLOR}\" stroke-width=\"{w}\"",
                edge_path_d(e, |p| p)
            )));
        }
    }
}
