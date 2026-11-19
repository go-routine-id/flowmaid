import sys
p='src/layout.rs'; s=open(p).read()
def rep(a,b,cnt=1):
    global s
    assert s.count(a)>=1, a[:70]
    if cnt==1: assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)

# --- public entry points
rep('''pub fn layout_clustered(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
) -> LayoutResult {
    layout_core(g, sizes, Some(node_cluster), &[]).0
}''','''pub fn layout_clustered(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
) -> LayoutResult {
    // The switch of the block layout is read here, once, for a call that
    // comes in through the public function ([`crate::levels`]).
    let blocks = (crate::levels::blocks_on() && crate::levels::border_edge(g, node_cluster))
        .then(BlockLinks::default);
    layout_core(g, sizes, Some(node_cluster), &[], blocks).0
}''')
rep('''    layout_core(g, sizes, None, &[]).0
}''','''    layout_core(g, sizes, None, &[], None).0
}''')
rep('''    layout_core(g, sizes, Some(node_cluster), rank_only)
}''','''    layout_core(
        g,
        sizes,
        Some(node_cluster),
        rank_only,
        g.block_links.clone(),
    )
}''')
rep('''    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
) -> (LayoutResult, EdgeGroups) {
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );''','''    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
    blocks: Option<BlockLinks>,
) -> (LayoutResult, EdgeGroups) {
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );''')
rep('''    let rk = rank(g, &groups, drawn, node_cluster, walk);
    let mut layer = rk.layer.clone();
    if g.tight_sources {
        if let Some(tight) = tighten_sources(g, &groups, &rk, &layer) {''','''    let rk = rank(g, &groups, drawn, node_cluster, walk, blocks.as_ref());
    let mut layer = rk.layer.clone();
    if g.tight_sources {
        if let Some(tight) = tighten_sources(g, &groups, &rk, &layer) {''')
rep('''        groups,
        drawn,
        walk,
        rk,
        layer,
        best: first,''','''        groups,
        drawn,
        walk,
        blocks,
        rk,
        layer,
        best: first,''')
rep('''    /// The walk every candidate layering takes its cycle breakers from.
    walk: Walk,
    rk: Ranking,''','''    /// The walk every candidate layering takes its cycle breakers from.
    walk: Walk,
    /// Block mode ([`rank`]): every candidate layering is a block
    /// layering.
    blocks: Option<BlockLinks>,
    rk: Ranking,''')
rep('''                let crk = rank(g, &cand_groups, st.drawn, node_cluster, st.walk);''','''                let crk = rank(
                    g,
                    &cand_groups,
                    st.drawn,
                    node_cluster,
                    st.walk,
                    st.blocks.as_ref(),
                );''')
# tests
rep('''rank(&g, &[], 0, None, w)''','''rank(&g, &[], 0, None, w, None)''')
rep('''layout_core(&g, &sizes, None, &[])''','''layout_core(&g, &sizes, None, &[], None)''')
rep('''rank(&g, &[], 0, None, Walk::Fewest)''','''rank(&g, &[], 0, None, Walk::Fewest, None)''',cnt=0)
rep('''rank(&g, &[], 0, None, Walk::Source)''','''rank(&g, &[], 0, None, Walk::Source, None)''')
rep('''layout_core(&g, &sizes, Some(&nc), &[])''','''layout_core(&g, &sizes, Some(&nc), &[], None)''')
rep('''rank(&g2, &[], 0, None, Walk::Fewest)''','''rank(&g2, &[], 0, None, Walk::Fewest, None)''')
# imports
rep('''use crate::clearance::{FRAME_SEP, NODE_SEP, TIGHT_RUN_MIN};''','''use crate::clearance::{FRAME_SEP, NODE_SEP, TIGHT_RUN_MIN};
use crate::levels::{BlockLinks, Tree, Unit as LevelUnit};''')
open(p,'w').write(s)
