p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)

# place: a flagged stand-in is no link
rep('''    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to {
            continue; // self-loop — no layered path
        }
        // Order the endpoints by layer (lo below hi) to build the chain.''','''    // Block mode: a flagged stand-in is no link (no channel, no pull).
    let flagged = |ei: usize| rk.blocks.as_ref().is_some_and(|b| b.flagged[ei]);
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to || flagged(ei) {
            continue; // self-loop — no layered path
        }
        // Order the endpoints by layer (lo below hi) to build the chain.''')

# Fence + tighten_sources
rep('''/// Each movable source one layer above its nearest target, or `None`
/// when no source moves.
fn tighten_sources(
    g: &Graph,
    groups: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
) -> Option<Vec<usize>> {
    let nl = node_links(g, groups, rk, layer);
    let mut out = layer.to_vec();
    let mut changed = false;
    for v in 0..layer.len() {
        if nl.pinned[v] || !nl.ins[v].is_empty() {
            continue;
        }
        // A source's targets all have an in-link, so none of them moves.
        let Some(to) = nl.outs[v].iter().map(|&w| layer[w]).min() else {
            continue;
        };
        let to = (to - 1).min(nl.deepest[v]);
        if to > layer[v] {
            out[v] = to;
            changed = true;
        }
    }
    changed.then(|| compact_layers(out))
}
''','''/// The fence a block layering puts round later layer moves
/// ([`tighten_sources`], [`stagger_proposals`]): a node never leaves the
/// rows of a block it is in, and no block's rows change.
struct Fence<'a> {
    /// Each node's chain of blocks ([`BlockRanking::paths`]).
    paths: &'a [Vec<usize>],
    /// Per block: its first and last layer.
    span: Vec<(usize, usize)>,
    /// Members per (block, layer).
    count: BTreeMap<(usize, usize), usize>,
}

impl<'a> Fence<'a> {
    fn new(blocks: &'a BlockRanking, layer: &[usize]) -> Fence<'a> {
        let subs = blocks
            .paths
            .iter()
            .flatten()
            .max()
            .map_or(0, |&c| c + 1);
        let mut span = vec![(usize::MAX, 0usize); subs];
        let mut count: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (v, path) in blocks.paths.iter().enumerate() {
            for &c in path {
                span[c] = (span[c].0.min(layer[v]), span[c].1.max(layer[v]));
                *count.entry((c, layer[v])).or_insert(0) += 1;
            }
        }
        Fence {
            paths: &blocks.paths,
            span,
            count,
        }
    }

    /// Whether node `v` stays within the rows of every block round it
    /// on layer `to`.
    fn holds(&self, v: usize, to: usize) -> bool {
        self.paths[v]
            .iter()
            .all(|&c| self.span[c].0 <= to && to <= self.span[c].1)
    }

    /// Whether node `v` may move from layer `from` to layer `to`: it
    /// stays within its blocks' rows and leaves none of their rows
    /// empty.
    fn allows(&self, v: usize, from: usize, to: usize) -> bool {
        self.holds(v, to)
            && self.paths[v]
                .iter()
                .all(|&c| self.count.get(&(c, from)).copied().unwrap_or(0) > 1)
    }

    /// Record that node `v` moved from layer `from` to layer `to`.
    fn moved(&mut self, v: usize, from: usize, to: usize) {
        for &c in &self.paths[v] {
            if let Some(k) = self.count.get_mut(&(c, from)) {
                *k -= 1;
            }
            *self.count.entry((c, to)).or_insert(0) += 1;
        }
    }

    /// Whether `layer` (each node's new layer) leaves every block the
    /// rows it had.
    fn unchanged(&self, layer: &[usize]) -> bool {
        let mut span = vec![(usize::MAX, 0usize); self.span.len()];
        for (v, path) in self.paths.iter().enumerate() {
            for &c in path {
                span[c] = (span[c].0.min(layer[v]), span[c].1.max(layer[v]));
            }
        }
        span == self.span
    }
}

/// Each movable source one layer above its nearest target, or `None`
/// when no source moves. In block mode a source moves only within the
/// rows of its blocks ([`Fence`]).
fn tighten_sources(
    g: &Graph,
    groups: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
) -> Option<Vec<usize>> {
    let nl = node_links(g, groups, rk, layer);
    let mut fence = rk.blocks.as_ref().map(|b| Fence::new(b, layer));
    let mut out = layer.to_vec();
    let mut changed = false;
    for v in 0..layer.len() {
        if nl.pinned[v] || !nl.ins[v].is_empty() {
            continue;
        }
        // A source's targets all have an in-link, so none of them moves.
        let Some(to) = nl.outs[v].iter().map(|&w| layer[w]).min() else {
            continue;
        };
        let to = (to - 1).min(nl.deepest[v]);
        if to > layer[v] {
            if let Some(f) = fence.as_mut() {
                if !f.allows(v, layer[v], to) {
                    continue;
                }
                f.moved(v, layer[v], to);
            }
            out[v] = to;
            changed = true;
        }
    }
    changed.then(|| compact_layers(out))
}
''')

# stagger: leaves and branches
rep('''    // Leaves: +1 = steps down (sink or edge-less), -1 = steps up.
    let leaves = every_second(&|v| {
        if nl.outs[v].is_empty() && layer[v] < nl.deepest[v] {
            Some(1)
        } else if nl.ins[v].is_empty()
            && !nl.outs[v].is_empty()
            && layer[v] as isize > nl.shallowest[v]
            && box_top(v)
        {
            Some(-1)
        } else {
            None
        }
    });''','''    // Block mode: a leaf steps only within the rows of its blocks. (The
    // first of every bucket stays, so no row of a block empties.)
    let fence = rk.blocks.as_ref().map(|b| Fence::new(b, layer));
    let fenced = |v: usize, d: isize| match (&fence, layer[v].checked_add_signed(d)) {
        (None, _) => true,
        (Some(f), Some(to)) => f.holds(v, to),
        // A fresh layer above the first: outside every block.
        (Some(f), None) => f.paths[v].is_empty(),
    };
    // Leaves: +1 = steps down (sink or edge-less), -1 = steps up.
    let leaves = every_second(&|v| {
        if nl.outs[v].is_empty() && layer[v] < nl.deepest[v] {
            Some(1).filter(|&d| fenced(v, d))
        } else if nl.ins[v].is_empty()
            && !nl.outs[v].is_empty()
            && layer[v] as isize > nl.shallowest[v]
            && box_top(v)
        {
            Some(-1).filter(|&d| fenced(v, d))
        } else {
            None
        }
    });''')
rep('''        // A cycle breaker inside the moved set could now point the
        // other way: skip rather than re-break cycles.
        if !(0..n).any(|v| down[v] && nl.pinned[v]) {
            out.push(compact_layers(moved));
        }''','''        // A cycle breaker inside the moved set could now point the
        // other way: skip rather than re-break cycles. In block mode the
        // shift must leave every block its rows.
        if !(0..n).any(|v| down[v] && nl.pinned[v])
            && fence.as_ref().map_or(true, |f| f.unchanged(&moved))
        {
            out.push(compact_layers(moved));
        }''')
open(p,'w').write(s)

p='src/scene.rs'; s=open(p).read()
rep('''    let mut sc = if g.subgraphs.is_empty() {
        #[cfg(test)]
        crate::levels::note_used(crate::levels::Entry::Scene, g, false);
        scene_flat(g, sizes, labels)
    } else {
        scene_clustered(g, sizes, labels)
    };''','''    let mut sc = if g.subgraphs.is_empty() {
        #[cfg(test)]
        crate::levels::note_used(crate::levels::Entry::Scene, g, false);
        scene_flat(g, sizes, labels)
    } else {
        // The switch of the block layout, read once for this call.
        scene_clustered(g, sizes, labels, crate::levels::blocks_on())
    };''')
rep('''fn scene_clustered(g: &Graph, sizes: &[(f64, f64)], labels: &[String]) -> Scene {''','''///
/// With `blocks` (the block layout is switched on) a graph with a link
/// across a subgraph's border is laid out as blocks
/// ([`crate::levels::Augmented::block_mode`]).
fn scene_clustered(g: &Graph, sizes: &[(f64, f64)], labels: &[String], blocks: bool) -> Scene {''')
rep('''    let aug = crate::levels::Augmented::of(g);
    #[cfg(test)]
    crate::levels::note_used(
        crate::levels::Entry::Scene,
        g,
        crate::levels::blocks_on() && aug.block_mode(g),
    );
    let rank_only = aug.rank_only();
    let ne = g.edges.len();
    let (mut ga, stand_in) = aug.graph(g);
    let (node_cluster, proxies) = (aug.node_cluster, aug.proxies);''','''    let aug = crate::levels::Augmented::of(g);
    let block_mode = blocks && aug.block_mode(g);
    #[cfg(test)]
    crate::levels::note_used(crate::levels::Entry::Scene, g, block_mode);
    let rank_only = aug.rank_only();
    let ne = g.edges.len();
    let (mut ga, stand_in) = aug.graph(g);
    // Block mode goes to the layout with the augmented graph.
    ga.block_links = block_mode.then(|| aug.block_links(&stand_in, ga.edges.len()));
    let (node_cluster, proxies) = (aug.node_cluster, aug.proxies);''')
open(p,'w').write(s)
