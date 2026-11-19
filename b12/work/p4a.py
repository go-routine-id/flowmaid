p='src/layout.rs'; s=open(p).read()
def rep(a,b,cnt=1):
    global s
    assert s.count(a)==cnt, (s.count(a), a[:70])
    s=s.replace(a,b)

# --- return type of layout_core / layout_clustered_ranked
rep('''/// edge, the cluster path of each channel waypoint, so the caller can
/// tell which box a lane runs inside.
pub(crate) fn layout_clustered_ranked(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
) -> (LayoutResult, EdgeGroups) {''','''/// edge, the cluster path of each channel waypoint, so the caller can
/// tell which box a lane runs inside; and, when the graph was laid out
/// as blocks (`Graph::block_links`), each node's chain of blocks as the
/// layering took them ([`BlockPaths`]).
pub(crate) fn layout_clustered_ranked(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
) -> (LayoutResult, EdgeGroups, Option<BlockPaths>) {''')
rep('''/// A rank-only link `(from, to)`: every `to` node ranks at least one
/// layer below every `from` node.''','''/// Per node of a graph laid out as blocks: its chain of blocks as the
/// layering took them, outermost first — its subgraph path without the
/// subgraphs the lane rule dissolved ([`block_rank`]). Two nodes with
/// the same chain are plain nodes of one level: an upstream edge
/// between them keeps its lane ([`keeps_lane`]).
pub(crate) type BlockPaths = Vec<Vec<usize>>;

/// Whether an edge that runs against the flow is drawn as a back-edge
/// lane: always before blocks; in a block layout (`blocks`) only
/// between two plain nodes of one level. Every other upstream edge of a
/// block layout runs in a channel, out of its source's upstream side
/// and into its target's downstream side.
pub(crate) fn keeps_lane(blocks: Option<&[Vec<usize>]>, from: usize, to: usize) -> bool {
    blocks.map_or(true, |paths| paths[from] == paths[to])
}

/// The channel point that carries a chain's label in a block layout:
/// the middle one of the chain's run at the level of its ends' common
/// ancestor (`common`, their shared subgraph path), which lies outside
/// both end blocks; without such a run, the middle of the whole chain.
/// `paths` holds each channel point's subgraph path.
pub(crate) fn label_slot(paths: &[Vec<usize>], common: &[usize]) -> usize {
    let run: Vec<usize> = (0..paths.len())
        .filter(|&i| paths[i].as_slice() == common)
        .collect();
    if run.is_empty() {
        paths.len() / 2
    } else {
        run[run.len() / 2]
    }
}

/// A rank-only link `(from, to)`: every `to` node ranks at least one
/// layer below every `from` node.''')
rep('''    blocks: Option<BlockLinks>,
) -> (LayoutResult, EdgeGroups) {''','''    blocks: Option<BlockLinks>,
) -> (LayoutResult, EdgeGroups, Option<BlockPaths>) {''')
rep('''    if !search {
        return (st.best.lo, st.best.groups);
    }''','''    if !search {
        return (st.best.lo, st.best.groups, st.rk.blocks.map(|b| b.paths));
    }''')
rep('''    centre_largest(g, sizes, node_cluster, &mut st);
    (st.best.lo, st.best.groups)
}''','''    centre_largest(g, sizes, node_cluster, &mut st);
    (st.best.lo, st.best.groups, st.rk.blocks.map(|b| b.paths))
}''')
rep('''let (lo, _) = layout_core(&g, &sizes, None, &[], None);''','''let (lo, ..) = layout_core(&g, &sizes, None, &[], None);''')
rep('''let (lo, _) = layout_core(&g, &sizes, Some(&nc), &[], None);''','''let (lo, ..) = layout_core(&g, &sizes, Some(&nc), &[], None);''')

# --- place: chains both ways
rep('''    // Block mode: a flagged stand-in is no link (no channel, no pull).
    let flagged = |ei: usize| rk.blocks.as_ref().is_some_and(|b| b.flagged[ei]);''','''    // Block mode: a flagged stand-in is no link (no channel, no pull).
    let flagged = |ei: usize| rk.blocks.as_ref().is_some_and(|b| b.flagged[ei]);
    let block_paths = rk.blocks.as_ref().map(|b| b.paths.as_slice());
    // A back-edge without a channel: every one before blocks; in a
    // block layout only one between two plain nodes of a level
    // ([`keeps_lane`]) and a note card's link — the others run up a
    // channel of their own, in the model the ordering minimises.
    let no_channel = |ei: usize| {
        let e = &g.edges[ei];
        let card = |v: usize| {
            let node = &g.nodes[v];
            crate::style::is_note_card(&node.id, node.shape, &node.style)
        };
        back[ei] && (keeps_lane(block_paths, e.from, e.to) || card(e.from) || card(e.to))
    };''')
rep('''        if matches!(e.kind, EdgeKind::Invisible) || back[ei] {
            succs[lo].push(hi);
            preds[hi].push(lo);
            continue;
        }''','''        if matches!(e.kind, EdgeKind::Invisible) || no_channel(ei) {
            succs[lo].push(hi);
            preds[hi].push(lo);
            continue;
        }''')
rep('''        if let (Some(lbl), false) = (&e.label, chain.is_empty()) {
            let mid = chain[chain.len() / 2];''','''        // (A block layout picks the dummy once the chain's subgraph
        // paths are known, below.)
        if let (Some(lbl), false, None) = (&e.label, chain.is_empty(), block_paths) {
            let mid = chain[chain.len() / 2];''')
rep('''            for &d in chain {
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
        }''','''            for &d in chain {
                let l = alayer[d];
                let kv = fit(&nc[e.to], l);
                let ku = fit(&nc[e.from], l);
                // A block layout: the deeper fit wins. Below the top
                // level the shared ancestor makes the target's prefix
                // fit on every layer, so a chain out of a nested block
                // took the ancestor's path on the block's remaining
                // layers, lay beside the block and crossed its side
                // wall. (Both fits past the common prefix: a level the
                // lane rule keeps side by side; the target's, as ever.)
                p[d] = if block_paths.is_some() && ku > kv {
                    nc[e.from][..ku].to_vec()
                } else if kv > 0 {
                    nc[e.to][..kv].to_vec()
                } else if ku > 0 {
                    nc[e.from][..ku].to_vec()
                } else {
                    common_prefix(&nc[e.from], &nc[e.to])
                };
            }
        }
        // A block layout's label slot: the chain's middle dummy at the
        // level of its ends' common ancestor, outside both end blocks
        // ([`label_slot`]); sized, it is a real obstacle (see above).
        if block_paths.is_some() {
            for (ei, chain) in edge_chain.iter().enumerate() {
                let e = &g.edges[ei];
                let (Some(lbl), false) = (&e.label, chain.is_empty()) else {
                    continue;
                };
                let paths: Vec<Vec<usize>> = chain.iter().map(|&d| p[d].clone()).collect();
                let mid = chain[label_slot(&paths, &common_prefix(&nc[e.from], &nc[e.to]))];
                let lw = text_width(lbl) + LABEL_PAD_W;
                let lh = LINE_H * line_count(lbl) as f64 + 6.0;
                if horizontal {
                    absize[mid] = absize[mid].max(lh);
                    alsize[mid] = alsize[mid].max(lw);
                } else {
                    absize[mid] = absize[mid].max(lw);
                    alsize[mid] = alsize[mid].max(lh);
                }
            }
        }''')

# --- drawn_breadth / back_lanes
rep('''    bpos: &[f64],
    edge_chain: &[Vec<usize>],
) -> (f64, f64) {
    let n = g.nodes.len();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);''','''    bpos: &[f64],
    edge_chain: &[Vec<usize>],
    blocks: Option<&[Vec<usize>]>,
) -> (f64, f64) {
    let n = g.nodes.len();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);''')
rep('''    for (ei, lane) in back_lanes(g, &at, node_cluster, edge_chain, |d| {
        (bpos[d], alayer[d] as f64)
    }) {''','''    for (ei, lane) in back_lanes(g, &at, node_cluster, blocks, edge_chain, |d| {
        (bpos[d], alayer[d] as f64)
    }) {''')
rep('''/// The lane is judged for crossings against the skeletons of the other
/// edges (end centres and channel points, `dummy(d)` = a dummy's
/// (breadth, layer) — `edge_chain` lists each edge's), on the layers.
pub(crate) fn back_lanes(
    g: &Graph,
    at: &[Placed],
    node_cluster: Option<&[Vec<usize>]>,
    edge_chain: &[Vec<usize>],''','''/// The lane is judged for crossings against the skeletons of the other
/// edges (end centres and channel points, `dummy(d)` = a dummy's
/// (breadth, layer) — `edge_chain` lists each edge's), on the layers.
///
/// In a block layout (`blocks`, its [`BlockPaths`]) only an upstream
/// edge between two plain nodes of one level is a lane
/// ([`keeps_lane`]); every other upstream edge is drawn through its
/// channel, or as a step across the gap, and counts among the
/// skeletons.
pub(crate) fn back_lanes(
    g: &Graph,
    at: &[Placed],
    node_cluster: Option<&[Vec<usize>]>,
    blocks: Option<&[Vec<usize>]>,
    edge_chain: &[Vec<usize>],''')
rep('''            drawn(e) && at[e.to].layer < at[e.from].layer && !card(e.from) && !card(e.to)
        })
        .collect();
    if backs.is_empty() {
        return Vec::new();
    }''','''            drawn(e)
                && at[e.to].layer < at[e.from].layer
                && !card(e.from)
                && !card(e.to)
                && keeps_lane(blocks, e.from, e.to)
        })
        .collect();
    if backs.is_empty() {
        return Vec::new();
    }''')
rep('''        .filter(|(_, e)| drawn(e) && at[e.to].layer >= at[e.from].layer)
        .map(|(ei, e)| {
            let mut pts = vec![(at[e.from].b, at[e.from].l)];''','''        .filter(|(_, e)| {
            drawn(e)
                && (at[e.to].layer >= at[e.from].layer
                    || !(keeps_lane(blocks, e.from, e.to) || card(e.from) || card(e.to)))
        })
        .map(|(ei, e)| {
            let mut pts = vec![(at[e.from].b, at[e.from].l)];''')
open(p,'w').write(s)
