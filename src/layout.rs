//! Sugiyama layout engine, following dagre (the algorithm mermaid.js
//! uses):
//!
//! 1. Detect back-edges via DFS so cycles don't break layering.
//! 2. Assign layers with longest-path (topological, Kahn-style).
//! 3. Split long edges into per-layer dummy chains (routing channels);
//!    a labelled long edge sizes its middle dummy to the label.
//! 4. Order within layers to cut crossings: weighted-median sweeps plus
//!    a local adjacent-swap transpose, keeping the fewest-crossing round.
//!    Subgraph members are held contiguous, bracketed by border walls.
//! 5. Assign coordinates with Brandes-Köpf: four vertical alignments
//!    blended by the per-node median (straight long edges, centred fans).
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

/// Horizontal padding inside a node box, each side (issue #14: public
/// so embedders measuring text themselves can reproduce node sizes).
pub const PAD_X: f64 = 16.0;
/// Base node height for a single-line label.
pub const BASE_H: f64 = 38.0;
/// Minimum node width.
pub const MIN_W: f64 = 54.0;
// Mermaid's dagre defaults (nodeSpacing / rankSpacing ≈ 50) — labels
// no longer ride between ranks (they own dummy slots), so the extra
// breathing room the old 62/84 reserved is dead space now.
const GAP_B: f64 = 50.0; // gap between REAL nodes within a layer
pub(crate) const GAP_L: f64 = 60.0; // gap between layers (fold DP glue too)
const MARGIN: f64 = 28.0;
/// Gap contribution of an edge DUMMY within a layer (dagre's
/// `edgesep`): parallel long edges bundle into tight lanes instead of
/// spreading a full node-gap apart — key to mermaid's flowing look.
const EDGE_GAP: f64 = 18.0;
/// Extra half-gap added per subgraph boundary crossed between two
/// adjacent nodes in a layer — reserves each cluster box its padding.
const CLUSTER_PAD: f64 = 18.0;
/// Gap between a cluster's invisible border wall and its members —
/// small, since the wall only pins the band, not the visible box.
const BORDER_GAP: f64 = 4.0;
/// Rank room reserved for a cluster's title strip (mirrors the
/// scene-side `SUB_HEADER` box headroom).
const CLUSTER_HEADER: f64 = 26.0;

/// Line height for multi-line labels (`<br/>` → newline).
pub const LINE_H: f64 = 17.0;

/// Font size (px) the estimated-width table is calibrated at:
/// [`text_width`]'s per-character advances are Helvetica values at
/// this size. To lay the same text out at another size, rescale —
/// `text_width(s) * size / TEXT_CALIBRATION` — instead of guessing
/// the table's base size.
pub const TEXT_CALIBRATION: f64 = 14.0;

/// Estimated rendered width of the WIDEST line in `s` (labels may
/// be multi-line after `<br/>` normalisation), in px at
/// [`TEXT_CALIBRATION`]. `<b>`/`<i>` styling
/// tags are interpreted, not measured: bold runs count ~6% wider,
/// the tag characters themselves count zero.
pub fn text_width(s: &str) -> f64 {
    s.split('\n')
        .map(|l| {
            spans(l)
                .iter()
                .map(|(t, b, _)| line_width(t) * if *b { 1.06 } else { 1.0 })
                .sum()
        })
        .fold(0.0, f64::max)
}

/// Split one label line into styled runs `(text, bold, italic)`,
/// interpreting `<b>`/`<strong>` and `<i>`/`<em>` tags the way
/// mermaid does. Unknown tags stay literal text; unclosed tags just
/// style to the end of the line. Always returns at least one run.
///
/// KaTeX-style math (`$$…$$`, mermaid's math syntax) is tokenized as
/// its own *italic* run with the `$$` fences stripped — issue #12
/// Phase A: mermaid sources with math render as recognizable italic
/// TeX text (and are measured without the fences) instead of
/// breaking; real math layout is a later phase. An unclosed `$$`
/// stays literal.
pub fn spans(line: &str) -> Vec<(String, bool, bool)> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    let (mut bold, mut italic) = (false, false);
    let mut cur = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '$' {
            // Find the closing `$$` after a non-empty body.
            let close = (i + 4..chars.len())
                .find(|&k| chars[k - 1] == '$' && chars[k] == '$')
                .map(|k| k - 1);
            if let Some(k) = close {
                if !cur.is_empty() {
                    out.push((std::mem::take(&mut cur), bold, italic));
                }
                out.push((chars[i + 2..k].iter().collect(), bold, true));
                i = k + 2;
                continue;
            }
        }
        if chars[i] == '<' {
            if let Some(j) = chars[i..].iter().position(|&c| c == '>') {
                let tag: String = chars[i + 1..i + j]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase();
                let known = matches!(
                    tag.as_str(),
                    "b" | "/b" | "strong" | "/strong" | "i" | "/i" | "em" | "/em"
                );
                if known {
                    if !cur.is_empty() {
                        out.push((std::mem::take(&mut cur), bold, italic));
                    }
                    match tag.as_str() {
                        "b" | "strong" => bold = true,
                        "/b" | "/strong" => bold = false,
                        "i" | "em" => italic = true,
                        _ => italic = false,
                    }
                    i += j + 1;
                    continue;
                }
            }
        }
        cur.push(chars[i]);
        i += 1;
    }
    if !cur.is_empty() || out.is_empty() {
        out.push((cur, bold, italic));
    }
    out
}

/// Number of text lines in a label (at least 1).
pub fn line_count(s: &str) -> usize {
    s.split('\n').count().max(1)
}

/// Width of a single line (Helvetica at [`TEXT_CALIBRATION`] px)
/// per character class.
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
///
/// Uses the built-in Helvetica estimate ([`text_width`]). Embedders
/// whose renderer uses a REAL font should use [`intrinsic_size_with`]
/// and feed the sizes to `scene::scene_sized` — that is the documented
/// path for accurate node boxes (issue #14).
pub fn intrinsic_size(node: &Node) -> (f64, f64) {
    intrinsic_size_with(node, text_width)
}

/// [`intrinsic_size`] with an injectable text measurer: `measure`
/// receives one raw label LINE (may contain `<b>`/`<i>` tags — strip
/// or style them via [`spans`]) and returns its rendered width in px
/// at the diagram's 14px base font ([`TEXT_CALIBRATION`]). Shape
/// padding, multi-line growth and minimums are applied on top, using
/// the same public constants ([`PAD_X`], [`BASE_H`], [`MIN_W`],
/// [`LINE_H`]) the engine uses.
pub fn intrinsic_size_with(node: &Node, measure: impl Fn(&str) -> f64) -> (f64, f64) {
    let tw = node.label.split('\n').map(&measure).fold(0.0, f64::max);
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
    layout_core(g, sizes, None)
}

/// Cluster-aware layout. `node_cluster[v]` is v's subgraph path,
/// outermost id first (empty = top level). All real nodes and edges
/// are laid out in one global Sugiyama pass — no supernode collapse —
/// while each subgraph's members are kept contiguous within every
/// layer and separated from siblings, so cross-cluster edges route in
/// the channels between boxes instead of tangling through them. The
/// caller builds cluster rectangles from the returned member centres.
pub fn layout_clustered(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
) -> LayoutResult {
    layout_core(g, sizes, Some(node_cluster))
}

fn layout_core(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
) -> LayoutResult {
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
        // Reserve space for a long edge's label at the middle dummy of
        // its chain: sizing that dummy makes it a real obstacle, so the
        // labels of converging edges spread apart instead of piling up
        // (dagre models edge labels as nodes for exactly this reason).
        if let (Some(lbl), false) = (&e.label, chain.is_empty()) {
            let mid = chain[chain.len() / 2];
            let lw = text_width(lbl) + 14.0;
            let lh = LINE_H * line_count(lbl) as f64 + 6.0;
            if horizontal {
                absize[mid] = absize[mid].max(lh);
                alsize[mid] = alsize[mid].max(lw);
            } else {
                absize[mid] = absize[mid].max(lw);
                alsize[mid] = alsize[mid].max(lh);
            }
        }
        edge_chain[ei] = chain;
    }
    let clustered = node_cluster.is_some();

    // Border walls: for every subgraph, a left and a right dummy at each
    // layer it spans, chained vertically so BK keeps the wall straight.
    // The ordering brackets each subgraph's members between its walls, so
    // members stay in a compact band even when external edges tug them —
    // and other nodes are held outside the band. `border_meta` records
    // (node, side, cluster-path-incl-c) for the nodes pushed here.
    let mut border_meta: Vec<(usize, u8, Vec<usize>)> = Vec::new();
    // cluster id -> (lo, hi) layer span over members — walls cover it,
    // and edge dummies use it to know when they are inside a cluster.
    // Only point-read (`get`), never iterated: BTreeMap purely for
    // hygiene / consistency with `span` below (whose KEY ITERATION is
    // the one that must stay ordered for deterministic node numbering).
    let mut cluster_span: std::collections::BTreeMap<usize, (usize, usize)> =
        std::collections::BTreeMap::new();
    if let Some(nc) = node_cluster {
        // cluster id -> (path incl. c, lo layer, hi layer over members)
        let mut span: std::collections::BTreeMap<usize, (Vec<usize>, usize, usize)> =
            std::collections::BTreeMap::new();
        for v in 0..n {
            let p = &nc[v];
            for i in 0..p.len() {
                let e = span
                    .entry(p[i])
                    .or_insert_with(|| (p[..=i].to_vec(), alayer[v], alayer[v]));
                e.1 = e.1.min(alayer[v]);
                e.2 = e.2.max(alayer[v]);
            }
        }
        for (&c, &(_, lo, hi)) in &span {
            cluster_span.insert(c, (lo, hi));
        }
        // Reserve rank room for each box's TITLE STRIP: a member at its
        // cluster's top layer grows its layer-slot by a header's worth
        // per cluster that starts there (dagre solves this with ranked
        // border-top nodes; the inflated slot is the compact stand-in),
        // so labels between ranks never land on the header text.
        for v in 0..n {
            let mut extra = 0.0;
            for &c in &nc[v] {
                if span[&c].1 == alayer[v] {
                    extra += CLUSTER_HEADER;
                }
            }
            alsize[v] += extra * 2.0;
        }
        // BTreeMap iterates keys in order — wall numbering is
        // deterministic by construction.
        let cids: Vec<usize> = span.keys().copied().collect();
        for c in cids {
            let (prefix, lo, hi) = span[&c].clone();
            let (mut prev_bl, mut prev_br): (Option<usize>, Option<usize>) = (None, None);
            for l in lo..=hi {
                for side in [1u8, 2u8] {
                    let d = alayer.len();
                    alayer.push(l);
                    absize.push(0.0);
                    alsize.push(0.0);
                    preds.push(Vec::new());
                    succs.push(Vec::new());
                    let prev = if side == 1 { &mut prev_bl } else { &mut prev_br };
                    if let Some(p) = *prev {
                        succs[p].push(d);
                        preds[d].push(p);
                    }
                    *prev = Some(d);
                    border_meta.push((d, side, prefix.clone()));
                }
            }
        }
    }

    let na = alayer.len();
    let nlayers = alayer.iter().copied().max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); nlayers];
    for v in 0..na {
        layers[alayer[v]].push(v);
    }

    // Cluster path per augmented node: real nodes take the caller's
    // path; an edge dummy takes a PROGRESSIVE path (dagre's nesting
    // behaviour) — at layers inside an endpoint's cluster span it
    // adopts that endpoint's cluster path (target wins over source),
    // so a cross-cluster edge exits its source box through the bottom,
    // runs outside only while between boxes, then enters the target
    // box from the top and descends inside it — instead of orbiting
    // around the walled band. A border dummy carries its own cluster's
    // path. `border_side[v]` is 0 (none) / 1 (left) / 2 (right wall).
    let mut border_side = vec![0u8; na];
    let apath: Vec<Vec<usize>> = if let Some(nc) = node_cluster {
        let mut p: Vec<Vec<usize>> = vec![Vec::new(); na];
        p[..n].clone_from_slice(nc);
        // Longest prefix of `path` whose every cluster spans layer `l`.
        let fit = |path: &[usize], l: usize| -> usize {
            let mut k = 0;
            for &c in path {
                match cluster_span.get(&c) {
                    Some(&(lo, hi)) if lo <= l && l <= hi => k += 1,
                    _ => break,
                }
            }
            k
        };
        for (ei, chain) in edge_chain.iter().enumerate() {
            if chain.is_empty() {
                continue;
            }
            let e = &g.edges[ei];
            for &d in chain {
                let l = alayer[d];
                let kv = fit(&nc[e.to], l);
                let ku = fit(&nc[e.from], l);
                p[d] = if kv > 0 {
                    nc[e.to][..kv].to_vec()
                } else if ku > 0 {
                    nc[e.from][..ku].to_vec()
                } else {
                    common_prefix(&nc[e.from], &nc[e.to])
                };
            }
        }
        for (d, side, path) in &border_meta {
            p[*d] = path.clone();
            border_side[*d] = *side;
        }
        p
    } else {
        Vec::new()
    };

    // --- 3. Reduce crossings: dagre-style ordering. Weighted-median
    // sweeps (down via preds, up via succs) followed by a local
    // adjacent-swap transpose each round. The median heuristic can
    // transiently worsen an ordering, so we keep the layering with the
    // fewest crossings seen across all rounds (keep-best) — this also
    // guards against regressing the natural insertion order on ties.
    // When clustered, each round finishes by regrouping every layer so
    // subgraph members stay contiguous, and the crossing count (hence
    // keep-best) is measured on that contiguous ordering.
    let mut pos = vec![0.0f64; na];
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }
    if clustered {
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos);
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
        transpose(
            &mut layers,
            &preds,
            &succs,
            &mut pos,
            nlayers,
            if clustered { Some(&apath) } else { None },
            if clustered { Some(&border_side) } else { None },
        );
        if clustered {
            for lv in layers.iter_mut() {
                enforce_contiguity(lv, &apath, &border_side, &mut pos);
            }
        }
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

    // Breadth positions (b axis) via Brandes-Köpf: four vertical
    // alignments (up/down × left/right), each compacted independently,
    // then combined by the per-node median. This is dagre's coordinate
    // assignment — it pins each long edge's dummy chain into a straight
    // vertical run and centres nodes over their aligned neighbours.
    let mut bpos = coordinates_bk(
        n,
        na,
        nlayers,
        &layers,
        &preds,
        &succs,
        &alayer,
        &absize,
        if clustered { Some(&apath) } else { None },
        if clustered { Some(&border_side) } else { None },
    );

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
/// of crossings they induce with the layers above and below, running to
/// convergence like dagre (a generous cap guards pathological inputs).
/// In clustered mode only same-cluster-path pairs swap — a cross-group
/// swap would be undone by the contiguity pass anyway (its "gain" was
/// an illusion), and border walls never move off their group's edge.
fn transpose(
    layers: &mut [Vec<usize>],
    preds: &[Vec<usize>],
    succs: &[Vec<usize>],
    pos: &mut [f64],
    nlayers: usize,
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
) {
    let mut improved = true;
    let mut guard = 0;
    while improved && guard < 20 {
        improved = false;
        guard += 1;
        for li in 0..nlayers {
            let len = layers[li].len();
            for i in 0..len.saturating_sub(1) {
                let v = layers[li][i];
                let w = layers[li][i + 1];
                if border.is_some_and(|b| b[v] != 0 || b[w] != 0)
                    || apath.is_some_and(|p| p[v] != p[w])
                {
                    continue;
                }
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

/// Brandes-Köpf coordinate assignment (breadth axis). Runs the four
/// vertical alignments (down/up × left/right), compacts each into a
/// non-overlapping layout, aligns them to the narrowest, and returns
/// the per-node median of the four candidates. Long-edge dummy chains
/// share a block per alignment, so they come out as straight vertical
/// runs — the hallmark of dagre's output.
#[allow(clippy::too_many_arguments)] // internal: one bundle of layered-graph state
fn coordinates_bk(
    n: usize,
    na: usize,
    nlayers: usize,
    layers: &[Vec<usize>],
    preds: &[Vec<usize>],
    succs: &[Vec<usize>],
    alayer: &[usize],
    absize: &[f64],
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
) -> Vec<f64> {
    if na == 0 {
        return Vec::new();
    }

    // Adjacent-layer neighbour sets (back/invisible links that skip a
    // layer are excluded — BK only reasons about layer±1 segments).
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); na];
    let mut down: Vec<Vec<usize>> = vec![Vec::new(); na];
    for v in 0..na {
        for &u in &preds[v] {
            if alayer[u] + 1 == alayer[v] {
                up[v].push(u);
            }
        }
        for &w in &succs[v] {
            if alayer[v] + 1 == alayer[w] {
                down[v].push(w);
            }
        }
    }

    // Position of each node within its layer (natural, un-adjusted).
    let mut order0 = vec![0usize; na];
    for lay in layers {
        for (i, &v) in lay.iter().enumerate() {
            order0[v] = i;
        }
    }

    let conflicts = type1_conflicts(n, nlayers, layers, &up, &order0);

    // Four candidate assignments, keyed by (vert_up, horiz_right).
    let mut cands: Vec<Vec<f64>> = Vec::with_capacity(4);
    for &vert_up in &[false, true] {
        for &horiz_right in &[false, true] {
            // Build the adjusted layering: reverse layer order for the
            // "up" sweeps, reverse within-layer order for the "right".
            let mut al: Vec<Vec<usize>> = layers.to_vec();
            if vert_up {
                al.reverse();
            }
            if horiz_right {
                for lay in al.iter_mut() {
                    lay.reverse();
                }
            }
            let mut aorder = vec![0usize; na];
            for lay in &al {
                for (i, &v) in lay.iter().enumerate() {
                    aorder[v] = i;
                }
            }
            // Align towards the already-processed layer: predecessors
            // for a downward sweep, successors for an upward one.
            let neighbor = if vert_up { &down } else { &up };
            let (root, _align) =
                vertical_alignment(na, &al, neighbor, &aorder, &conflicts);
            let mut xs = horizontal_compaction(n, na, &al, &root, absize, apath, border);
            if horiz_right {
                for x in xs.iter_mut() {
                    *x = -*x;
                }
            }
            cands.push(xs);
        }
    }

    // Align the four to the narrowest, then take each node's median.
    align_candidates(&mut cands, n, absize);
    let mut bpos = vec![0.0f64; na];
    for v in 0..na {
        let mut q = [cands[0][v], cands[1][v], cands[2][v], cands[3][v]];
        q.sort_by(f64::total_cmp);
        bpos[v] = (q[1] + q[2]) / 2.0;
    }
    bpos
}

/// Mark type-1 conflicts: a non-inner segment that crosses an inner
/// segment (one strung between two dummy nodes). Aligning across such a
/// pair would kink the long edge, so BK forbids it. Stored symmetrically.
fn type1_conflicts(
    n: usize,
    nlayers: usize,
    layers: &[Vec<usize>],
    up: &[Vec<usize>],
    order: &[usize],
) -> std::collections::HashSet<(usize, usize)> {
    let is_dummy = |v: usize| v >= n;
    let mut conflicts = std::collections::HashSet::new();
    for li in 1..nlayers {
        let lower = &layers[li];
        let prev_len = layers[li - 1].len();
        let mut k0 = 0usize;
        let mut scan = 0usize;
        for (l1, &v) in lower.iter().enumerate() {
            // Inner segment: v is a dummy whose upper neighbour is a dummy.
            let w = if is_dummy(v) {
                up[v].iter().copied().find(|&u| is_dummy(u))
            } else {
                None
            };
            let is_last = l1 + 1 == lower.len();
            if w.is_some() || is_last {
                let k1 = w.map(|ww| order[ww]).unwrap_or(prev_len);
                for &scan_node in &lower[scan..=l1] {
                    for &u in &up[scan_node] {
                        let upos = order[u];
                        if (upos < k0 || upos > k1) && !(is_dummy(u) && is_dummy(scan_node)) {
                            let pair = if u < scan_node { (u, scan_node) } else { (scan_node, u) };
                            conflicts.insert(pair);
                        }
                    }
                }
                scan = l1 + 1;
                k0 = k1;
            }
        }
    }
    conflicts
}

/// One BK vertical alignment. Walks layers top-to-bottom (in adjusted
/// order); each node tries to align with its median neighbour in the
/// already-placed layer, forming blocks identified by a shared root.
fn vertical_alignment(
    na: usize,
    al: &[Vec<usize>],
    neighbor: &[Vec<usize>],
    aorder: &[usize],
    conflicts: &std::collections::HashSet<(usize, usize)>,
) -> (Vec<usize>, Vec<usize>) {
    let mut root: Vec<usize> = (0..na).collect();
    let mut align: Vec<usize> = (0..na).collect();
    for lay in al {
        let mut prev_idx: i64 = -1;
        for &v in lay {
            let mut ws: Vec<usize> = neighbor[v].clone();
            if ws.is_empty() {
                continue;
            }
            ws.sort_by_key(|&w| aorder[w]);
            let m = ws.len();
            let lo = (m - 1) / 2;
            let hi = m / 2;
            for &w in &ws[lo..=hi] {
                let pair = if v < w { (v, w) } else { (w, v) };
                if align[v] == v
                    && prev_idx < aorder[w] as i64
                    && !conflicts.contains(&pair)
                {
                    align[w] = v;
                    root[v] = root[w];
                    align[v] = root[w];
                    prev_idx = aorder[w] as i64;
                }
            }
        }
    }
    (root, align)
}

/// Compact the blocks of one alignment along the breadth axis. Builds
/// the block graph (min-separation edges between consecutive roots in a
/// layer), pushes every block as far left as its predecessors allow,
/// then pulls it back right toward its successors without overlap.
fn horizontal_compaction(
    n: usize,
    na: usize,
    al: &[Vec<usize>],
    root: &[usize],
    absize: &[f64],
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
) -> Vec<f64> {
    // Block graph as adjacency lists over root nodes.
    let mut bin: Vec<Vec<(usize, f64)>> = vec![Vec::new(); na]; // (pred_root, sep)
    let mut bout: Vec<Vec<(usize, f64)>> = vec![Vec::new(); na]; // (succ_root, sep)
    let mut is_block = vec![false; na];
    // Each node contributes half its separation class (dagre): a real
    // node half of GAP_B, an edge dummy half of EDGE_GAP — so parallel
    // edge channels bundle tightly while real nodes keep their room.
    let half = |v: usize| if v < n { GAP_B / 2.0 } else { EDGE_GAP / 2.0 };
    for lay in al {
        let mut prev: Option<usize> = None;
        for &v in lay {
            let vr = root[v];
            is_block[vr] = true;
            if let Some(u) = prev {
                let ur = root[u];
                // Widen the gap across a subgraph boundary so each box
                // gets its padding and cross-cluster channels have room.
                let cgap = apath.map_or(0.0, |p| cluster_gap(&p[u], &p[v]));
                // A wall hugs its members (only pins the band); ordinary
                // neighbours pay their separation-class halves.
                let is_wall = border.is_some_and(|b| b[u] != 0 || b[v] != 0);
                let base = if is_wall { BORDER_GAP } else { half(u) + half(v) };
                let sep = absize[u] / 2.0 + base + absize[v] / 2.0 + cgap;
                // Merge parallel separations by their max.
                if let Some(e) = bout[ur].iter_mut().find(|(t, _)| *t == vr) {
                    if sep > e.1 {
                        e.1 = sep;
                    }
                    if let Some(e2) = bin[vr].iter_mut().find(|(t, _)| *t == ur) {
                        e2.1 = e.1;
                    }
                } else {
                    bout[ur].push((vr, sep));
                    bin[vr].push((ur, sep));
                }
            }
            prev = Some(v);
        }
    }

    let mut xs = vec![0.0f64; na];
    // Pass 1 — leftmost feasible: post-order DFS so every predecessor
    // block is placed before the block that leans on it.
    let mut visited = vec![false; na];
    let mut stack: Vec<usize> = (0..na).filter(|&v| is_block[v]).collect();
    while let Some(elem) = stack.pop() {
        if visited[elem] {
            let mut x = 0.0f64;
            for &(p, sep) in &bin[elem] {
                x = x.max(xs[p] + sep);
            }
            xs[elem] = x;
        } else {
            visited[elem] = true;
            stack.push(elem);
            for &(p, _) in &bin[elem] {
                stack.push(p);
            }
        }
    }
    // Pass 2 — pull right toward successors to centre, never overlapping.
    let mut visited2 = vec![false; na];
    let mut stack2: Vec<usize> = (0..na).filter(|&v| is_block[v]).collect();
    while let Some(elem) = stack2.pop() {
        if visited2[elem] {
            let mut min = f64::INFINITY;
            for &(s, sep) in &bout[elem] {
                min = min.min(xs[s] - sep);
            }
            if min.is_finite() {
                xs[elem] = xs[elem].max(min);
            }
        } else {
            visited2[elem] = true;
            stack2.push(elem);
            for &(s, _) in &bout[elem] {
                stack2.push(s);
            }
        }
    }

    // Project block coordinates back onto every member node.
    let mut out = vec![0.0f64; na];
    for v in 0..na {
        out[v] = xs[root[v]];
    }
    out
}

/// Shift the four alignments so they share a reference frame, then leave
/// them for the median blend. Left-biased alignments align on their min
/// edge, right-biased on their max, matching dagre's `alignCoordinates`.
fn align_candidates(cands: &mut [Vec<f64>], n: usize, absize: &[f64]) {
    // Pick the narrowest (by real-node extent) as the anchor frame.
    let extent = |xs: &[f64]| -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for v in 0..n {
            lo = lo.min(xs[v] - absize[v] / 2.0);
            hi = hi.max(xs[v] + absize[v] / 2.0);
        }
        (lo, hi)
    };
    let mut anchor = 0usize;
    let mut best_w = f64::INFINITY;
    for (i, xs) in cands.iter().enumerate() {
        let (lo, hi) = extent(xs);
        if hi - lo < best_w {
            best_w = hi - lo;
            anchor = i;
        }
    }
    let (amin, amax) = extent(&cands[anchor]);
    for (i, xs) in cands.iter_mut().enumerate() {
        if i == anchor {
            continue;
        }
        let (lo, hi) = extent(xs);
        // Even indices are left-biased (l), odd are right-biased (r).
        let delta = if i % 2 == 0 { amin - lo } else { amax - hi };
        if delta != 0.0 {
            for x in xs.iter_mut() {
                *x += delta;
            }
        }
    }
}

/// Longest shared prefix of two cluster paths (their deepest common
/// ancestor subgraph, as a path).
fn common_prefix(a: &[usize], b: &[usize]) -> Vec<usize> {
    a.iter()
        .zip(b)
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| *x)
        .collect()
}

/// Breadth padding to reserve between two adjacent nodes whose cluster
/// paths differ — one `CLUSTER_PAD` per subgraph boundary crossed
/// (leaving one nesting level + entering another counts both).
fn cluster_gap(a: &[usize], b: &[usize]) -> f64 {
    let common = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let boundaries = (a.len() - common) + (b.len() - common);
    boundaries as f64 * CLUSTER_PAD
}

/// Reorder one layer so every subgraph's members are contiguous
/// (recursively for nested subgraphs), while ordering the groups and
/// free nodes by their current mean position — keeping the crossing-
/// minimised arrangement as intact as the contiguity constraint allows.
fn enforce_contiguity(
    layer: &mut Vec<usize>,
    apath: &[Vec<usize>],
    border: &[u8],
    pos: &mut [f64],
) {
    if layer.len() <= 1 {
        return;
    }
    let arranged = arrange(layer, 0, apath, border, pos);
    *layer = arranged;
    for (i, &v) in layer.iter().enumerate() {
        pos[v] = i as f64;
    }
}

/// Recursive helper for [`enforce_contiguity`]: at `depth`, split items
/// into subgraph groups (by cluster id at that depth) and free
/// singletons, order those units by mean position, then recurse into
/// each multi-member group at `depth + 1`.
fn arrange(
    items: &[usize],
    depth: usize,
    apath: &[Vec<usize>],
    border: &[u8],
    pos: &[f64],
) -> Vec<usize> {
    if items.len() <= 1 {
        return items.to_vec();
    }
    let cid = |v: usize| -> Option<usize> {
        let p = &apath[v];
        if p.len() > depth {
            Some(p[depth])
        } else {
            None
        }
    };
    // Group by cluster id at this depth, preserving first-seen order;
    // each free node (no cluster here) becomes its own unit.
    let mut groups: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
    let mut index: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for &v in items {
        match cid(v) {
            Some(c) => {
                if let Some(&gi) = index.get(&c) {
                    groups[gi].1.push(v);
                } else {
                    index.insert(c, groups.len());
                    groups.push((Some(c), vec![v]));
                }
            }
            None => groups.push((None, vec![v])),
        }
    }
    let mut keyed: Vec<(f64, Option<usize>, Vec<usize>)> = groups
        .into_iter()
        .map(|(c, members)| {
            let mean = members.iter().map(|&v| pos[v]).sum::<f64>() / members.len() as f64;
            // A bare wall dummy anchors to its group's edge: the left
            // wall sorts before everything, the right wall after.
            let key = if members.len() == 1 {
                match border[members[0]] {
                    1 => f64::NEG_INFINITY,
                    2 => f64::INFINITY,
                    _ => mean,
                }
            } else {
                mean
            };
            (key, c, members)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::with_capacity(items.len());
    for (_, c, members) in keyed {
        if c.is_some() && members.len() > 1 {
            out.extend(arrange(&members, depth + 1, apath, border, pos));
        } else {
            out.extend(members);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_interpret_bold_italic_and_keep_unknown_tags() {
        assert_eq!(spans("plain"), vec![("plain".into(), false, false)]);
        assert_eq!(
            spans("<b>Go</b> · MongoDB"),
            vec![("Go".into(), true, false), (" · MongoDB".into(), false, false)]
        );
        assert_eq!(
            spans("a <i>b</i> <strong>c</strong>"),
            vec![
                ("a ".into(), false, false),
                ("b".into(), false, true),
                (" ".into(), false, false),
                ("c".into(), true, false),
            ]
        );
        // Unknown tags stay literal; unclosed styles run to line end.
        assert_eq!(spans("<x>y"), vec![("<x>y".into(), false, false)]);
        assert_eq!(spans("<b>y"), vec![("y".into(), true, false)]);
        // Comparison `a < b` is untouched (no closing '>').
        assert_eq!(spans("a < b"), vec![("a < b".into(), false, false)]);
    }

    #[test]
    fn math_spans_render_as_italic_tex_with_fences_stripped() {
        // #12 Phase A: mermaid math syntax tokenizes instead of
        // breaking; the TeX body shows as an italic run.
        assert_eq!(spans("$$x^2$$"), vec![("x^2".into(), false, true)]);
        assert_eq!(
            spans("area $$\\frac{1}{2}$$ done"),
            vec![
                ("area ".into(), false, false),
                ("\\frac{1}{2}".into(), false, true),
                (" done".into(), false, false),
            ]
        );
        // Inside bold, math keeps the bold and adds italic.
        assert_eq!(
            spans("<b>E = $$mc^2$$</b>"),
            vec![("E = ".into(), true, false), ("mc^2".into(), true, true)]
        );
        // Unclosed / empty fences stay literal (prices, shell text).
        assert_eq!(spans("$$ 100"), vec![("$$ 100".into(), false, false)]);
        assert_eq!(spans("$$$$"), vec![("$$$$".into(), false, false)]);
        assert_eq!(spans("a $5 b $7"), vec![("a $5 b $7".into(), false, false)]);
        // The fences don't count toward measured width.
        assert!(text_width("$$xy$$") < text_width("$$xy$$ ") + 1.0);
        assert_eq!(text_width("$$xy$$"), text_width("<i>xy</i>"));
    }

    #[test]
    fn styling_tags_do_not_inflate_text_width() {
        let tagged = text_width("<b>Rust</b> · Tonic gRPC");
        let plain = text_width("Rust · Tonic gRPC");
        assert!(tagged >= plain, "bold run measures slightly wider");
        assert!(
            tagged < plain * 1.1,
            "tags themselves must not count: {tagged} vs {plain}"
        );
    }

    #[test]
    fn bold_labels_render_as_svg_tspans_not_literal_tags() {
        let svg = crate::render_svg("flowchart TD\nA[\"<b>Go</b> service\"] --> B").unwrap();
        assert!(svg.contains("font-weight=\"bold\">Go</tspan>"), "bold tspan");
        assert!(!svg.contains("&lt;b&gt;"), "no literal <b> in output");
    }
}
