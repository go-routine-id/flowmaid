//! Sugiyama-style layout engine (compact edition):
//!
//! 1. Detect back-edges via DFS so cycles don't break layering.
//! 2. Assign layers with longest-path (topological, Kahn-style).
//! 3. Order within layers using the barycenter heuristic
//!    (reduces edge crossings).
//! 4. Assign coordinates: per-layer packing + alignment towards
//!    neighbours (parents/children) without overlap.
//!
//! Everything is computed in abstract coordinates (b = breadth,
//! l = layer / depth); the renderer maps them to final x,y
//! according to the diagram direction (TD/LR/BT/RL).

use crate::model::{Direction, EdgeKind, Graph, Node, Shape};
use std::collections::VecDeque;

/// Position & size of one node in abstract coordinates.
pub struct Placed {
    /// Centre point on the breadth axis.
    pub b: f64,
    /// Centre point on the layer axis.
    pub l: f64,
    /// Node size along the breadth axis.
    pub bsize: f64,
    /// Node size along the layer axis.
    pub lsize: f64,
    /// Layer index.
    pub layer: usize,
}

pub struct LayoutResult {
    pub nodes: Vec<Placed>,
    pub total_b: f64,
    pub total_l: f64,
    /// Per original edge: the abstract `(b, l)` waypoints its virtual-
    /// node chain passes through (empty for adjacent-layer edges). A
    /// renderer can spline through these so a long edge routes in the
    /// channel between layers instead of straight across the nodes.
    pub edge_paths: Vec<Vec<(f64, f64)>>,
}

const PAD_X: f64 = 16.0;
const BASE_H: f64 = 38.0;
const MIN_W: f64 = 54.0;
const GAP_B: f64 = 62.0; // gap between nodes within a layer
const GAP_L: f64 = 84.0; // gap between layers
const MARGIN: f64 = 28.0;

/// Line height for multi-line labels (`<br/>` → newline).
pub const LINE_H: f64 = 17.0;

/// Estimated rendered width of the WIDEST line in `s` (labels may
/// be multi-line after `<br/>` normalisation).
pub fn text_width(s: &str) -> f64 {
    s.split('\n').map(line_width).fold(0.0, f64::max)
}

/// Number of text lines in a label (at least 1).
pub fn line_count(s: &str) -> usize {
    s.split('\n').count().max(1)
}

/// Width of a single line (Helvetica ~14px) per character class.
/// Without real font metrics this stays approximate, but it is far
/// more accurate than a flat average: capitals ~9.7px, i/l ~3.4px,
/// m/W ~12-13px, CJK/emoji ~14px.
fn line_width(s: &str) -> f64 {
    s.chars()
        .map(|c| match c {
            'i' | 'l' | 'j' => 3.4,
            ' ' | '.' | ',' | ':' | ';' | '!' | '\'' | 't' | 'f' | 'I' | '|' => 3.9,
            'r' | '(' | ')' | '[' | ']' | '-' | '/' => 4.7,
            's' | 'c' | 'k' | 'v' | 'x' | 'y' | 'z' | 'J' => 7.0,
            'm' | 'M' => 11.7,
            'w' => 10.1,
            'W' => 13.2,
            'A'..='Z' => 9.7,
            c if (c as u32) >= 0x2E80 => 14.0, // CJK, emoji, wide symbols
            _ => 7.8,
        })
        .sum()
}

/// Intrinsic node size (width, height) in pixels, based on shape,
/// the estimated widest-line width, and the number of label lines.
pub fn intrinsic_size(node: &Node) -> (f64, f64) {
    let tw = text_width(&node.label);
    // Height grows with extra label lines beyond the first.
    let extra = (line_count(&node.label) - 1) as f64 * LINE_H;
    let base_h = BASE_H + extra;
    match node.shape {
        Shape::Rect | Shape::Rounded => ((tw + 2.0 * PAD_X).max(MIN_W), base_h),
        Shape::Stadium => ((tw + 2.0 * PAD_X + 12.0).max(MIN_W + 12.0), base_h),
        // Subroutine has inner side bars; parallelograms slant — both
        // need a bit of extra horizontal room.
        Shape::Subroutine | Shape::Parallelogram | Shape::ParallelogramAlt => {
            ((tw + 2.0 * PAD_X + 24.0).max(MIN_W + 24.0), base_h)
        }
        // Hexagon points eat horizontal space.
        Shape::Hexagon => ((tw + 2.0 * PAD_X + 28.0).max(MIN_W + 28.0), base_h),
        // Cylinder caps add vertical room.
        Shape::Cylinder => ((tw + 2.0 * PAD_X).max(MIN_W), base_h + 16.0),
        // Diamonds need extra room so the text fits in the middle.
        Shape::Diamond => (((tw + 24.0) * 1.6).max(80.0), base_h * 1.7),
        Shape::Circle => {
            let d = (tw + 24.0).max(52.0).max(base_h);
            (d, d)
        }
        Shape::DoubleCircle => {
            let d = (tw + 32.0).max(60.0).max(base_h);
            (d, d)
        }
        // stateDiagram pseudostates: fixed size, no label.
        Shape::StateStart => (14.0, 14.0),
        Shape::StateEnd => (18.0, 18.0),
        Shape::ForkBar => (60.0, 8.0),
    }
}

pub fn layout(g: &Graph) -> LayoutResult {
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    layout_sized(g, &sizes)
}

/// Same as [`layout`] but with caller-provided node sizes (width,
/// height in pixels) — used by pipelines where node size doesn't
/// come from the label, e.g. ER entity tables or icon nodes.
pub fn layout_sized(g: &Graph, sizes: &[(f64, f64)]) -> LayoutResult {
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );
    let n = g.nodes.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (ei, e) in g.edges.iter().enumerate() {
        adj[e.from].push((e.to, ei));
    }

    // --- 1. Mark back-edges (cycle breakers) with iterative DFS ---
    let mut state = vec![0u8; n]; // 0 unvisited, 1 on stack, 2 done
    let mut back = vec![false; g.edges.len()];
    for s in 0..n {
        if state[s] != 0 {
            continue;
        }
        state[s] = 1;
        let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
        while !stack.is_empty() {
            let (u, ci) = *stack.last().unwrap();
            if ci < adj[u].len() {
                stack.last_mut().unwrap().1 += 1;
                let (v, ei) = adj[u][ci];
                if v == u {
                    back[ei] = true; // self-loop
                    continue;
                }
                match state[v] {
                    0 => {
                        state[v] = 1;
                        stack.push((v, 0));
                    }
                    1 => back[ei] = true, // edge back to an ancestor = cycle
                    _ => {}
                }
            } else {
                state[u] = 2;
                stack.pop();
            }
        }
    }

    // --- 2. Longest-path layering on the DAG (back-edges excluded) ---
    let mut indeg = vec![0usize; n];
    for (ei, e) in g.edges.iter().enumerate() {
        if !back[ei] {
            indeg[e.to] += 1;
        }
    }
    let mut layer = vec![0usize; n];
    let mut q: VecDeque<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    while let Some(u) = q.pop_front() {
        for &(v, ei) in &adj[u] {
            if back[ei] {
                continue;
            }
            if layer[u] + 1 > layer[v] {
                layer[v] = layer[u] + 1;
            }
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push_back(v);
            }
        }
    }

    // --- Virtual (dummy) nodes over an AUGMENTED graph. An edge that
    // spans >1 layer is broken into a chain of dummies (one per crossed
    // layer) so it takes part in ordering and RESERVES a routing channel
    // — real nodes spread around it instead of it cutting straight
    // across them. Augmented index space: real nodes `0..n`, dummies
    // `n..`. `absize`/`alsize` are the breadth/layer sizes per aug node.
    const DUMMY_B: f64 = 16.0;
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let mut alayer = layer.clone();
    let mut absize: Vec<f64> = (0..n)
        .map(|v| if horizontal { sizes[v].1 } else { sizes[v].0 })
        .collect();
    let mut alsize: Vec<f64> = (0..n)
        .map(|v| if horizontal { sizes[v].0 } else { sizes[v].1 })
        .collect();
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    // Per original edge: dummy chain in the edge's own from→to order.
    let mut edge_chain: Vec<Vec<usize>> = vec![Vec::new(); g.edges.len()];
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to {
            continue; // self-loop — no layered path
        }
        // Order the endpoints by layer (lo below hi) to build the chain.
        let ascending = alayer[e.from] <= alayer[e.to];
        let (lo, hi) = if ascending { (e.from, e.to) } else { (e.to, e.from) };
        let (llo, lhi) = (alayer[lo], alayer[hi]);
        if lhi <= llo + 1 {
            if llo < lhi {
                succs[lo].push(hi);
                preds[hi].push(lo);
            }
            continue; // same or adjacent layer — no dummies
        }
        // Invisible links (ranking-only) and back-edges get NO channel:
        // the former must not inflate the canvas, and a back-edge routed
        // up the middle would cross every forward edge — it keeps the
        // sideways bow instead. Both still influence ordering directly.
        if matches!(e.kind, EdgeKind::Invisible) || back[ei] {
            succs[lo].push(hi);
            preds[hi].push(lo);
            continue;
        }
        let mut prev = lo;
        let mut chain = Vec::with_capacity(lhi - llo - 1);
        for lay in (llo + 1)..lhi {
            let d = alayer.len();
            alayer.push(lay);
            absize.push(DUMMY_B);
            alsize.push(0.0);
            preds.push(Vec::new());
            succs.push(Vec::new());
            succs[prev].push(d);
            preds[d].push(prev);
            chain.push(d);
            prev = d;
        }
        succs[prev].push(hi);
        preds[hi].push(prev);
        if !ascending {
            chain.reverse(); // store low→high chain in from→to order
        }
        edge_chain[ei] = chain;
    }
    let na = alayer.len();
    let nlayers = alayer.iter().copied().max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); nlayers];
    for v in 0..na {
        layers[alayer[v]].push(v);
    }

    // --- 3. Reduce crossings: dagre-style ordering. Weighted-median
    // sweeps (down via preds, up via succs) followed by a local
    // adjacent-swap transpose each round. The median heuristic can
    // transiently worsen an ordering, so we keep the layering with the
    // fewest crossings seen across all rounds (keep-best) — this also
    // guards against regressing the natural insertion order on ties.
    let mut pos = vec![0.0f64; na];
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }
    let mut best_layers = layers.clone();
    let mut best_cross = count_crossings(&layers, &succs, &alayer, nlayers);
    for _ in 0..8 {
        for li in 1..nlayers {
            reorder(&mut layers[li], &preds, &mut pos);
        }
        for li in (0..nlayers.saturating_sub(1)).rev() {
            reorder(&mut layers[li], &succs, &mut pos);
        }
        transpose(&mut layers, &preds, &succs, &mut pos, nlayers);
        let c = count_crossings(&layers, &succs, &alayer, nlayers);
        if c < best_cross {
            best_cross = c;
            best_layers = layers.clone();
        }
        if best_cross == 0 {
            break;
        }
    }
    layers = best_layers;
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }

    // --- 4. Coordinates ---
    // Layer positions (l axis): each layer is as tall as its tallest
    // REAL node (dummies contribute zero layer-size).
    let mut lcoord = vec![0.0f64; nlayers];
    let mut cursor = MARGIN;
    for li in 0..nlayers {
        let lh = layers[li].iter().map(|&v| alsize[v]).fold(0.0f64, f64::max);
        lcoord[li] = cursor + lh / 2.0;
        cursor += lh + GAP_L;
    }
    let total_l = cursor - GAP_L + MARGIN;

    // Initial packing per layer, then centre each layer.
    let mut bpos = vec![0.0f64; na];
    let mut widths = vec![0.0f64; nlayers];
    for li in 0..nlayers {
        let mut c = 0.0;
        for &v in &layers[li] {
            bpos[v] = c + absize[v] / 2.0;
            c += absize[v] + GAP_B;
        }
        widths[li] = if layers[li].is_empty() { 0.0 } else { c - GAP_B };
    }
    let maxw = widths.iter().fold(0.0f64, |a, &b| a.max(b));
    for li in 0..nlayers {
        let off = MARGIN + (maxw - widths[li]) / 2.0;
        for &v in &layers[li] {
            bpos[v] += off;
        }
    }

    // Alignment: pull each node towards its neighbours' mean, order &
    // min-gaps preserved (dummies included → channels stay open).
    for li in 1..nlayers {
        align_pass(&layers[li], &preds, &mut bpos, &absize);
    }
    for li in (0..nlayers.saturating_sub(1)).rev() {
        align_pass(&layers[li], &succs, &mut bpos, &absize);
    }
    for li in 1..nlayers {
        align_pass(&layers[li], &preds, &mut bpos, &absize);
    }

    // Normalise so the diagram starts at MARGIN (real-node extent).
    let mut minb = f64::INFINITY;
    let mut maxb = f64::NEG_INFINITY;
    for v in 0..n {
        minb = minb.min(bpos[v] - absize[v] / 2.0);
        maxb = maxb.max(bpos[v] + absize[v] / 2.0);
    }
    if n == 0 {
        minb = 0.0;
        maxb = 0.0;
    }
    let shift = MARGIN - minb;
    for v in 0..na {
        bpos[v] += shift;
    }
    let total_b = (maxb - minb) + 2.0 * MARGIN;

    let nodes = (0..n)
        .map(|v| Placed {
            b: bpos[v],
            l: lcoord[alayer[v]],
            bsize: absize[v],
            lsize: alsize[v],
            layer: alayer[v],
        })
        .collect();

    // Edge waypoints (abstract b,l) from each edge's dummy chain, for a
    // renderer that wants to spline the edge through its channel.
    let edge_paths = edge_chain
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|&d| (bpos[d], lcoord[alayer[d]]))
                .collect()
        })
        .collect();

    LayoutResult {
        nodes,
        total_b,
        total_l,
        edge_paths,
    }
}

/// Reorder one layer by the weighted median of each node's neighbour
/// positions (dagre's heuristic — more robust to outliers than the
/// barycenter mean). Nodes without neighbours keep their position;
/// ties break by node index so the pass is deterministic.
fn reorder(layer: &mut Vec<usize>, nbrs: &[Vec<usize>], pos: &mut [f64]) {
    let mut keyed: Vec<(f64, usize)> = layer
        .iter()
        .map(|&v| {
            let ns = &nbrs[v];
            let key = if ns.is_empty() {
                pos[v]
            } else {
                wmedian(ns.iter().map(|&u| pos[u]))
            };
            (key, v)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    layer.clear();
    for (i, (_, v)) in keyed.into_iter().enumerate() {
        layer.push(v);
        pos[v] = i as f64;
    }
}

/// Dagre's weighted median of a set of neighbour positions. For an even
/// count the two central values are blended by the widths of the gaps
/// on either side, which biases towards the denser cluster.
fn wmedian(vals: impl Iterator<Item = f64>) -> f64 {
    let mut ps: Vec<f64> = vals.collect();
    ps.sort_by(f64::total_cmp);
    let m = ps.len();
    match m {
        0 => -1.0,
        1 => ps[0],
        2 => (ps[0] + ps[1]) / 2.0,
        _ => {
            let mid = m / 2;
            if m % 2 == 1 {
                ps[mid]
            } else {
                let left = ps[mid - 1] - ps[0];
                let right = ps[m - 1] - ps[mid];
                if left + right == 0.0 {
                    (ps[mid - 1] + ps[mid]) / 2.0
                } else {
                    (ps[mid - 1] * right + ps[mid] * left) / (left + right)
                }
            }
        }
    }
}

/// Local adjacent-swap pass (dagre's transpose). Repeatedly walk every
/// layer and swap neighbouring nodes whenever doing so lowers the count
/// of crossings they induce with the layers above and below. Converges
/// quickly; a small guard caps the worst case.
fn transpose(
    layers: &mut [Vec<usize>],
    preds: &[Vec<usize>],
    succs: &[Vec<usize>],
    pos: &mut [f64],
    nlayers: usize,
) {
    let mut improved = true;
    let mut guard = 0;
    while improved && guard < 4 {
        improved = false;
        guard += 1;
        for li in 0..nlayers {
            let len = layers[li].len();
            for i in 0..len.saturating_sub(1) {
                let v = layers[li][i];
                let w = layers[li][i + 1];
                let before = local_crossings(v, w, preds, pos)
                    + local_crossings(v, w, succs, pos);
                let after = local_crossings(w, v, preds, pos)
                    + local_crossings(w, v, succs, pos);
                if after < before {
                    layers[li].swap(i, i + 1);
                    pos[v] = (i + 1) as f64;
                    pos[w] = i as f64;
                    improved = true;
                }
            }
        }
    }
}

/// Crossings induced by placing `v` immediately left of `w`: every pair
/// of edges (v→a, w→b) into the same adjacent layer crosses when a sits
/// to the right of b.
fn local_crossings(v: usize, w: usize, nbrs: &[Vec<usize>], pos: &[f64]) -> usize {
    let mut c = 0;
    for &a in &nbrs[v] {
        for &b in &nbrs[w] {
            if pos[a] > pos[b] {
                c += 1;
            }
        }
    }
    c
}

/// Total edge crossings across every adjacent-layer boundary, counted
/// as position inversions among the lower endpoints. Only genuine
/// layer+1 segments participate (back/invisible links route elsewhere).
fn count_crossings(
    layers: &[Vec<usize>],
    succs: &[Vec<usize>],
    alayer: &[usize],
    nlayers: usize,
) -> usize {
    let mut pos = vec![0usize; alayer.len()];
    for lv in layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i;
        }
    }
    let mut total = 0;
    for li in 0..nlayers.saturating_sub(1) {
        let mut es: Vec<(usize, usize)> = Vec::new();
        for &u in &layers[li] {
            for &v in &succs[u] {
                if alayer[v] == li + 1 {
                    es.push((pos[u], pos[v]));
                }
            }
        }
        es.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        for i in 0..es.len() {
            for j in (i + 1)..es.len() {
                if es[i].1 > es[j].1 {
                    total += 1;
                }
            }
        }
    }
    total
}

/// Shift each node (order preserved) towards the mean position of
/// its neighbours, as long as it doesn't collide with the node to
/// its left.
fn align_pass(order: &[usize], nbrs: &[Vec<usize>], bpos: &mut [f64], bsize: &[f64]) {
    let mut min_edge = f64::NEG_INFINITY;
    for &v in order {
        let ns = &nbrs[v];
        let desired = if ns.is_empty() {
            bpos[v]
        } else {
            ns.iter().map(|&u| bpos[u]).sum::<f64>() / ns.len() as f64
        };
        let c = desired.max(min_edge + bsize[v] / 2.0);
        bpos[v] = c;
        min_edge = c + bsize[v] / 2.0 + GAP_B;
    }
}
