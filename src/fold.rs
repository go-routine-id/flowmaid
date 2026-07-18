//! Serpentine / compact layout for long linear chains (issue #15).
//!
//! `a --> b --> … --> z` normally renders as one very long ribbon.
//! [`scene_compact`] folds it into balanced bands that fit a flow-axis
//! budget: fold points come from a Knuth-Plass-style dynamic program
//! (boxes = node flow extents, glue = the engine's rank gap, cost =
//! squared band slack + a penalty per fold and per folded-through
//! label), each band is laid out by the untouched standard pipeline,
//! odd bands are mirrored (boustrophedon), and the broken edges become
//! U-turn connectors through the fold gutters.
//!
//! Contract highlights:
//! - `CompactScene::scene.edges` stays 1:1 with `Graph::edges` and
//!   `scene.nodes` index-parallel with `Graph::nodes`, exactly like
//!   [`crate::scene::scene`].
//! - Refusal is byte-identical: whenever folding does not apply
//!   (`skipped` is `Some`), the returned scene IS the ordinary
//!   `scene()` output, never a half-processed layout.
//! - Interactivity: re-call `scene_compact` after text edits; for
//!   drags use `scene::route_partial` with the compact scene as base
//!   (bare `route()` would free-route the fold turns away).

use crate::layout::{intrinsic_size, Placed, GAP_L};
use crate::model::{Direction, Edge, EdgeKind, Graph};
use crate::scene::{
    anchor, flip_scene, grow_scene, parallel_offsets, scene_sized, shift_scene, Bbox, Scene,
    SceneEdge, MARGIN,
};

/// Minimum chain length worth folding — below this the fold would
/// save less space than its gutters cost.
const MIN_RUN: usize = 5;
/// Breadth gap between adjacent bands (content edge to content edge).
const BAND_GAP: f64 = 44.0;
/// How far a U-turn bows past the outermost node edge at a fold.
const TURN_DROP: f64 = 36.0;
/// Lane spacing when several parallel edges share one fold gutter.
const LANE: f64 = 14.0;
/// Flow room reserved per folded band end so the U-turn stays inside
/// the caller's budget.
const TURN_PAD: f64 = TURN_DROP;
/// Flat cost per fold: a diagram barely over budget prefers one fold
/// over several near-equal splits.
const BREAK_PENALTY: f64 = 60.0 * 60.0;
/// Extra cost for breaking at a labelled edge (the label has to ride
/// the U-turn's crossbar, which reads worse than a straight run).
const FOLD_LABEL_PENALTY: f64 = 40.0 * 40.0;
/// Breadth distance between fold anchors beyond which the U-turn gets
/// a mid-crossbar waypoint (curveBasis only approximates interior
/// points; the extra point keeps a wide crossbar flat).
const WIDE_TURN: f64 = 160.0;

/// Options for [`scene_compact`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CompactOptions {
    /// Budget along the FLOW axis (height for TD/BT, width for LR/RL),
    /// in px. Bands are folded so none exceeds it.
    pub max_extent: f64,
    /// Chains shorter than this many nodes are never folded.
    pub min_run: usize,
    /// Breadth gap between adjacent bands, in px.
    pub band_gap: f64,
}

impl CompactOptions {
    /// Fold to fit `max_extent` px along the flow axis.
    pub fn for_extent(max_extent: f64) -> CompactOptions {
        CompactOptions {
            max_extent,
            min_run: MIN_RUN,
            band_gap: BAND_GAP,
        }
    }

    /// Derive the flow budget from a viewport: picks the viewport's
    /// flow-axis extent for `dir` — a live re-wrap on window resize is
    /// one call.
    pub fn fit(viewport_w: f64, viewport_h: f64, dir: Direction) -> CompactOptions {
        match dir {
            Direction::TD | Direction::BT => CompactOptions::for_extent(viewport_h),
            Direction::LR | Direction::RL => CompactOptions::for_extent(viewport_w),
        }
    }
}

/// Why [`scene_compact`] returned the ordinary layout instead of a
/// folded one. Every variant guarantees the scene is byte-identical
/// to `scene()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FoldSkip {
    /// The ordinary layout already fits the budget.
    AlreadyFits,
    /// Subgraphs / sub-edges present — folding boxes is a v2 concern.
    HasSubgraphs,
    /// A node with fan-in/out > 1, or a cycle: not a linear chain.
    NotLinear,
    /// More than one weakly-connected component (or isolated nodes).
    MultipleComponents,
    /// Chain shorter than `CompactOptions::min_run`.
    TooShort,
    /// A single node's flow extent exceeds the budget — no fold can fix
    /// that.
    NodeTooLong,
}

/// Result of [`scene_compact`]: the geometry plus what happened.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CompactScene {
    pub scene: Scene,
    /// `None` = the scene is folded; `Some(reason)` = it is the
    /// ordinary `scene()` output, untouched.
    pub skipped: Option<FoldSkip>,
    /// Number of serpentine bands (1 when not folded).
    pub bands: usize,
}

/// Compact layout with intrinsic node sizes (the [`crate::scene::scene`]
/// counterpart).
pub fn scene_compact(g: &Graph, opts: &CompactOptions) -> CompactScene {
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    scene_compact_sized(g, &sizes, opts)
}

/// Compact layout with caller-provided node sizes (the
/// [`crate::scene::scene_sized`] counterpart).
pub fn scene_compact_sized(g: &Graph, sizes: &[(f64, f64)], opts: &CompactOptions) -> CompactScene {
    let base = scene_sized(g, sizes);
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let flow_extent = if horizontal { base.width } else { base.height };
    let skip = |scene: Scene, why: FoldSkip| CompactScene {
        scene,
        skipped: Some(why),
        bands: 1,
    };
    if flow_extent <= opts.max_extent {
        return skip(base, FoldSkip::AlreadyFits);
    }
    let run = match chain_run(g) {
        Ok(r) => r,
        Err(why) => return skip(base, why),
    };
    if run.len() < opts.min_run.max(2) {
        return skip(base, FoldSkip::TooShort);
    }

    // Flow extent per run node, in run order.
    let ext: Vec<f64> = run
        .iter()
        .map(|&v| if horizontal { sizes[v].0 } else { sizes[v].1 })
        .collect();
    // Does the (visible) edge between run positions i and i+1 carry a
    // label? Breaking there costs extra.
    let mut pos_of = vec![usize::MAX; g.nodes.len()];
    for (i, &v) in run.iter().enumerate() {
        pos_of[v] = i;
    }
    let mut labelled_gap = vec![false; run.len().saturating_sub(1)];
    for e in &g.edges {
        if e.from == e.to || e.label.is_none() {
            continue;
        }
        let (a, b) = (pos_of[e.from], pos_of[e.to]);
        let lo = a.min(b);
        if lo + 1 == a.max(b) {
            labelled_gap[lo] = true;
        }
    }

    let bands = match fold_points(&ext, &labelled_gap, opts.max_extent) {
        Ok(b) => b,
        Err(why) => return skip(base, why),
    };
    if bands.len() <= 1 {
        return skip(base, FoldSkip::AlreadyFits);
    }
    let scene = compose(g, sizes, &run, &pos_of, &bands, horizontal, opts);
    CompactScene {
        bands: bands.len(),
        scene,
        skipped: None,
    }
}

/// SVG convenience: fold, then serialise (accessible name
/// "Flowchart diagram"; use [`render_compact_titled`] for others).
pub fn render_compact(g: &Graph, opts: &CompactOptions) -> String {
    render_compact_titled(g, opts, "Flowchart diagram")
}

/// [`render_compact`] with a caller-chosen accessible name.
pub fn render_compact_titled(g: &Graph, opts: &CompactOptions, title: &str) -> String {
    crate::scene::to_svg_titled(&scene_compact(g, opts).scene, title)
}

/// The graph as one linear run of node indices, or the reason it
/// isn't one. Self-loops and `~~~` invisible ranking links don't
/// count against linearity (an invisible link never draws, so it can
/// cross bands freely).
fn chain_run(g: &Graph) -> Result<Vec<usize>, FoldSkip> {
    if !g.subgraphs.is_empty() || !g.sub_edges.is_empty() {
        return Err(FoldSkip::HasSubgraphs);
    }
    let n = g.nodes.len();
    if n < 2 {
        return Err(FoldSkip::TooShort);
    }
    let mut next: Vec<Option<usize>> = vec![None; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    for e in &g.edges {
        if e.from == e.to || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        match next[e.from] {
            None => next[e.from] = Some(e.to),
            Some(t) if t == e.to => {} // parallel edge — same hop
            Some(_) => return Err(FoldSkip::NotLinear),
        }
        match prev[e.to] {
            None => prev[e.to] = Some(e.from),
            Some(f) if f == e.from => {}
            Some(_) => return Err(FoldSkip::NotLinear),
        }
    }
    let starts: Vec<usize> = (0..n).filter(|&v| prev[v].is_none()).collect();
    if starts.len() != 1 {
        // 0 starts = cycle; >1 = several chains / isolated nodes.
        return Err(if starts.is_empty() {
            FoldSkip::NotLinear
        } else {
            FoldSkip::MultipleComponents
        });
    }
    let mut run = Vec::with_capacity(n);
    let mut cur = Some(starts[0]);
    while let Some(v) = cur {
        run.push(v);
        cur = next[v];
        if run.len() > n {
            return Err(FoldSkip::NotLinear); // lasso: chain into a cycle
        }
    }
    if run.len() != n {
        return Err(FoldSkip::MultipleComponents);
    }
    Ok(run)
}

/// Knuth-Plass-style fold DP: choose band boundaries over the run so
/// every band's flow extent (nodes + rank gaps + turn headroom) fits
/// the budget, minimising squared slack — which, for a fixed number of
/// folds, prefers BALANCED bands over one full band plus a stub.
/// Returns bands as inclusive `(start, end)` run-position ranges.
fn fold_points(
    ext: &[f64],
    labelled_gap: &[bool],
    max_extent: f64,
) -> Result<Vec<(usize, usize)>, FoldSkip> {
    let n = ext.len();
    let mut prefix = vec![0.0f64; n + 1];
    for i in 0..n {
        prefix[i + 1] = prefix[i] + ext[i];
    }
    // Band of run positions j..=i: node extents + inter-rank gaps +
    // U-turn headroom for each folded end.
    let band_len = |j: usize, i: usize| -> f64 {
        let body = prefix[i + 1] - prefix[j] + GAP_L * (i - j) as f64;
        let turns = (j > 0) as u32 + (i + 1 < n) as u32;
        body + TURN_PAD * f64::from(turns)
    };
    for (i, _) in ext.iter().enumerate() {
        if band_len(i, i) > max_extent {
            return Err(FoldSkip::NodeTooLong);
        }
    }
    // best[i] = (cost, band start) for an optimal split of 0..i.
    let mut best: Vec<(f64, usize)> = vec![(f64::INFINITY, 0); n + 1];
    best[0] = (0.0, 0);
    for i in 1..=n {
        for j in (0..i).rev() {
            let len = band_len(j, i - 1);
            if len > max_extent {
                break; // longer bands only get longer
            }
            let slack = max_extent - len;
            let mut cost = best[j].0 + slack * slack;
            if j > 0 {
                cost += BREAK_PENALTY;
                if labelled_gap[j - 1] {
                    cost += FOLD_LABEL_PENALTY;
                }
            }
            if cost < best[i].0 {
                best[i] = (cost, j);
            }
        }
        if best[i].0.is_infinite() {
            return Err(FoldSkip::NodeTooLong); // unreachable given the check above
        }
    }
    let mut bands = Vec::new();
    let mut end = n;
    while end > 0 {
        let start = best[end].1;
        bands.push((start, end - 1));
        end = start;
    }
    bands.reverse();
    Ok(bands)
}

/// Lay out each band with the standard pipeline, mirror odd bands,
/// place bands along the breadth axis, then stitch nodes, in-band
/// edges and U-turn fold connectors back into ONE scene in original
/// graph order.
fn compose(
    g: &Graph,
    sizes: &[(f64, f64)],
    run: &[usize],
    pos_of: &[usize],
    bands: &[(usize, usize)],
    horizontal: bool,
    opts: &CompactOptions,
) -> Scene {
    let nb = bands.len();
    let band_of = |v: usize| -> usize {
        let p = pos_of[v];
        bands.iter().position(|&(s, e)| p >= s && p <= e).unwrap()
    };

    // Per-band mini graphs (nodes in run order) via the untouched
    // pipeline; edge_map[ei] = (band, band-edge index) for in-band
    // edges, None for fold-crossing (or cross-band invisible) ones.
    let mut edge_map: Vec<Option<(usize, usize)>> = vec![None; g.edges.len()];
    let mut scenes: Vec<Scene> = Vec::with_capacity(nb);
    for (k, &(s, e)) in bands.iter().enumerate() {
        let slice: Vec<usize> = run[s..=e].to_vec();
        let mut local = vec![usize::MAX; g.nodes.len()];
        let mut bg = Graph::default();
        bg.direction = g.direction;
        let mut bsizes = Vec::with_capacity(slice.len());
        for (li, &v) in slice.iter().enumerate() {
            local[v] = li;
            bg.nodes.push(g.nodes[v].clone());
            bsizes.push(sizes[v]);
        }
        for (ei, edge) in g.edges.iter().enumerate() {
            if local[edge.from] != usize::MAX && local[edge.to] != usize::MAX {
                // Cross-band invisible links are geometry-only (never
                // drawn); in-band invisible links keep influencing the
                // band's ranking like they do in the flat layout.
                edge_map[ei] = Some((k, bg.edges.len()));
                bg.edges.push(Edge {
                    from: local[edge.from],
                    to: local[edge.to],
                    label: edge.label.clone(),
                    kind: edge.kind,
                });
            }
        }
        scenes.push(scene_sized(&bg, &bsizes));
    }

    // Mirror odd bands, then stack bands along the breadth axis.
    let mut off = 0.0f64;
    for (k, sc) in scenes.iter_mut().enumerate() {
        if k % 2 == 1 {
            let extent = if horizontal { sc.width } else { sc.height };
            flip_scene(sc, extent, horizontal);
        }
        let (dx, dy) = if horizontal { (0.0, off) } else { (off, 0.0) };
        shift_scene(sc, dx, dy);
        let breadth = if horizontal { sc.height } else { sc.width };
        off += breadth - 2.0 * MARGIN + opts.band_gap;
    }

    // Nodes back in ORIGINAL graph order.
    let nodes: Vec<crate::scene::SceneNode> = (0..g.nodes.len())
        .map(|v| {
            let k = band_of(v);
            let (s, _) = bands[k];
            scenes[k].nodes[pos_of[v] - s].clone()
        })
        .collect();

    // Edges back in original order: in-band geometry from the band
    // scenes, fold-crossers as U-turn connectors.
    let offs = parallel_offsets(g);
    // Lane index per gutter so parallel fold edges nest concentrically.
    let mut gutter_lane = vec![0usize; nb];
    let flow = |p: &crate::scene::SceneNode| if horizontal { p.x } else { p.y };
    let global_top = nodes
        .iter()
        .map(|p| flow(p))
        .fold(f64::INFINITY, f64::min);
    let global_bot = nodes
        .iter()
        .map(|p| flow(p))
        .fold(f64::NEG_INFINITY, f64::max);
    let edges: Vec<SceneEdge> = g
        .edges
        .iter()
        .enumerate()
        .map(|(ei, e)| {
            if let Some((k, bei)) = edge_map[ei] {
                return scenes[k].edges[bei].clone();
            }
            let (a, b) = (&nodes[e.from], &nodes[e.to]);
            if matches!(e.kind, EdgeKind::Invisible) {
                // Ranking-only link that happens to cross bands:
                // geometry is a straight (never painted) line.
                return SceneEdge {
                    from: a.id.clone(),
                    to: b.id.clone(),
                    bezier: [(a.x, a.y), (a.x, a.y), (b.x, b.y), (b.x, b.y)],
                    waypoints: Vec::new(),
                    kind: e.kind,
                    label: None,
                };
            }
            let gutter = band_of(e.from).min(band_of(e.to));
            let lane = gutter_lane[gutter];
            gutter_lane[gutter] += 1;
            fold_connector(g, sizes, e, a, b, horizontal, lane, offs[ei], global_top, global_bot)
        })
        .collect();

    let mut sc = Scene::empty(0.0, 0.0);
    sc.nodes = nodes;
    sc.edges = edges;
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    let (minx, maxx, miny, maxy) = bb.finish();
    shift_scene(&mut sc, MARGIN - minx, MARGIN - miny);
    sc.width = (maxx - minx) + 2.0 * MARGIN;
    sc.height = (maxy - miny) + 2.0 * MARGIN;
    sc
}

/// U-turn connector for one fold-crossing edge: exits the band on the
/// fold side, runs a crossbar through the gutter (staggered per lane),
/// and enters the next band from the same side. The waypoints feed the
/// standard curveBasis spline, so the turn renders as a smooth hook.
#[allow(clippy::too_many_arguments)] // internal: one bundle of fold state
fn fold_connector(
    g: &Graph,
    sizes: &[(f64, f64)],
    e: &Edge,
    a: &crate::scene::SceneNode,
    b: &crate::scene::SceneNode,
    horizontal: bool,
    lane: usize,
    off: f64,
    global_top: f64,
    global_bot: f64,
) -> SceneEdge {
    let (fl, br): (fn(&crate::scene::SceneNode) -> f64, fn(&crate::scene::SceneNode) -> f64) =
        if horizontal {
            (|n| n.x, |n| n.y)
        } else {
            (|n| n.y, |n| n.x)
        };
    let mk = |flow: f64, breadth: f64| -> (f64, f64) {
        if horizontal {
            (flow, breadth)
        } else {
            (breadth, flow)
        }
    };
    // Fold side: both endpoints sit at the same extreme by
    // construction — bow past whichever global edge they are closer to.
    let to_top = fl(a).min(fl(b)) - global_top;
    let to_bot = global_bot - fl(a).max(fl(b));
    let bottom = to_bot <= to_top;
    let half = |n: &crate::scene::SceneNode| if horizontal { n.w / 2.0 } else { n.h / 2.0 };
    let apex = if bottom {
        fl(a).max(fl(b)) + half(a).max(half(b)) + TURN_DROP + lane as f64 * LANE
    } else {
        fl(a).min(fl(b)) - half(a).max(half(b)) - TURN_DROP - lane as f64 * LANE
    };
    let placed = |n: &crate::scene::SceneNode, i: usize| Placed {
        b: n.x,
        l: n.y,
        bsize: sizes[i].0,
        lsize: sizes[i].1,
        layer: 0,
    };
    // `anchor`'s bottom flag is along the scene's own y for vertical
    // flow; for horizontal flow the "outer" side is along x, which
    // anchor() handles via the target point's position.
    let pa = placed(a, e.from);
    let pb = placed(b, e.to);
    let p0 = anchor(&pa, g.nodes[e.from].shape, mk(apex, br(a)), off, bottom == !horizontal || (horizontal && fl(a) < apex));
    let p3 = anchor(&pb, g.nodes[e.to].shape, mk(apex, br(b)), off, bottom == !horizontal || (horizontal && fl(b) < apex));
    let mut wps = Vec::with_capacity(5);
    wps.push(p0);
    wps.push(mk(apex, br(a)));
    let wide = (br(a) - br(b)).abs() > WIDE_TURN;
    if wide {
        wps.push(mk(apex, (br(a) + br(b)) / 2.0));
    }
    wps.push(mk(apex, br(b)));
    wps.push(p3);
    let label = e.label.as_ref().map(|l| {
        (
            l.clone(),
            mk(apex, (br(a) + br(b)) / 2.0),
            crate::layout::text_width(l) + 14.0,
        )
    });
    SceneEdge {
        from: a.id.clone(),
        to: b.id.clone(),
        bezier: [p0, mk(apex, br(a)), mk(apex, br(b)), p3],
        waypoints: wps,
        kind: e.kind,
        label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn chain(n: usize) -> String {
        let mut s = String::from("flowchart TD\n");
        for i in 0..n - 1 {
            s.push_str(&format!("N{} --> N{}\n", i, i + 1));
        }
        s
    }

    #[test]
    fn long_chain_folds_into_balanced_bands_within_budget() {
        let g = parse(&chain(12)).unwrap();
        let c = scene_compact(&g, &CompactOptions::for_extent(420.0));
        assert!(c.skipped.is_none(), "must fold: {:?}", c.skipped);
        assert!(c.bands >= 2);
        // Contracts: index-parallel nodes, 1:1 edges, budget respected.
        assert_eq!(c.scene.nodes.len(), g.nodes.len());
        assert_eq!(c.scene.edges.len(), g.edges.len());
        for (sn, n) in c.scene.nodes.iter().zip(&g.nodes) {
            assert_eq!(sn.id, n.id);
        }
        assert!(
            c.scene.height <= 420.0 + 2.0 * (TURN_DROP + MARGIN) + 40.0,
            "flow extent {} blew the budget",
            c.scene.height
        );
        // The whole point: flow extent collapses vs the plain ribbon,
        // paid for by breadth growth.
        let plain = crate::scene::scene(&g);
        assert!(
            c.scene.height < plain.height / 2.0,
            "folded {} vs plain {}",
            c.scene.height,
            plain.height
        );
        assert!(c.scene.width > plain.width, "bands spread along breadth");
        // Fold connectors thread waypoints; nothing is NaN.
        let folded = c.scene.edges.iter().filter(|e| e.waypoints.len() >= 4).count();
        assert!(folded >= c.bands - 1);
        let svg = crate::scene::to_svg(&c.scene);
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn every_direction_folds_on_its_own_flow_axis() {
        for (dir, taller_than_wide) in
            [("TD", false), ("BT", false), ("LR", true), ("RL", true)]
        {
            let src = chain(12).replace("flowchart TD", &format!("flowchart {dir}"));
            let g = parse(&src).unwrap();
            let c = scene_compact(&g, &CompactOptions::for_extent(420.0));
            assert!(c.skipped.is_none(), "{dir} must fold");
            assert!(c.bands >= 2, "{dir}");
            let plain = crate::scene::scene(&g);
            let (flow, plain_flow) = if taller_than_wide {
                (c.scene.width, plain.width) // LR/RL: flow axis = x
            } else {
                (c.scene.height, plain.height)
            };
            assert!(
                flow < plain_flow / 2.0,
                "{dir}: folded flow {flow} vs plain {plain_flow}"
            );
            let svg = crate::scene::to_svg(&c.scene);
            assert!(!svg.contains("NaN"), "{dir}");
        }
    }

    #[test]
    fn refusals_return_byte_identical_plain_scenes() {
        let same = |src: &str, why: FoldSkip, max: f64| {
            let g = parse(src).unwrap();
            let c = scene_compact(&g, &CompactOptions::for_extent(max));
            assert_eq!(c.skipped, Some(why), "{src:?}");
            assert_eq!(c.bands, 1);
            let plain = crate::scene::scene(&g);
            assert_eq!(
                crate::scene::to_svg(&c.scene),
                crate::scene::to_svg(&plain),
                "refusal must be byte-identical for {src:?}"
            );
        };
        // Fits: huge budget.
        same(&chain(12), FoldSkip::AlreadyFits, 100_000.0);
        // Non-linear: diamond fan-out.
        same("flowchart TD\nA-->B\nA-->C\nB-->D\nC-->D\nD-->E\nE-->F\nF-->G\nG-->H\nH-->I\nI-->J", FoldSkip::NotLinear, 100.0);
        // Subgraphs.
        same(
            "flowchart TD\nsubgraph S\nA-->B\nend\nB-->C\nC-->D\nD-->E\nE-->F\nF-->G",
            FoldSkip::HasSubgraphs,
            100.0,
        );
        // Two disjoint chains.
        same(
            "flowchart TD\nA-->B\nB-->C\nC-->D\nD-->E\nX-->Y\nY-->Z",
            FoldSkip::MultipleComponents,
            100.0,
        );
        // Cycle.
        same("flowchart TD\nA-->B\nB-->C\nC-->D\nD-->E\nE-->A", FoldSkip::NotLinear, 100.0);
        // Too short.
        same("flowchart TD\nA-->B\nB-->C", FoldSkip::TooShort, 10.0);
    }

    #[test]
    fn labels_ride_the_crossbar_and_drag_reroutes_locally() {
        let mut src = chain(12);
        src = src.replace("N5 --> N6", "N5 -->|hop| N6");
        let g = parse(&src).unwrap();
        let c = scene_compact(&g, &CompactOptions::for_extent(420.0));
        assert!(c.skipped.is_none());
        // Every edge label survives, folded or not.
        let labelled = c
            .scene
            .edges
            .iter()
            .filter(|e| e.label.is_some())
            .count();
        assert_eq!(labelled, 1);
        // route_partial keeps untouched fold geometry verbatim.
        let auto: Vec<(f64, f64)> = c.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut dragged = auto.clone();
        dragged[0].0 += 80.0;
        let r = crate::scene::route_partial(&g, &dragged, &c.scene, &auto);
        assert_eq!(r.edges.len(), c.scene.edges.len());
        for (ri, ci) in r.edges.iter().zip(c.scene.edges.iter()).skip(2) {
            assert_eq!(ri.waypoints, ci.waypoints, "untouched edges keep fold turns");
        }
    }
}
