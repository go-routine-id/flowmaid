import re
p='src/layout.rs'; s=open(p).read()
# remove wrap_block_rows
i=s.index('/// Block mode: wrap the long rows of unlinked members.')
j=s.index('/// A strongly connected component of a level\'s units is ordered exactly')
s=s[:i]+s[j:]
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''    let mut groups: Vec<RankGroup> = rank_only.to_vec();''','''    let groups: Vec<RankGroup> = rank_only.to_vec();
    let drawn = groups.len();''')
rep('''    let mut rk = rank(
        g,
        &groups,
        groups.len(),
        node_cluster,
        walk,
        blocks.as_ref(),
    );
    if let (Some(nc), Some(links)) = (node_cluster, blocks.as_ref()) {
        wrap_block_rows(g, sizes, nc, &mut groups, &mut rk, walk, links);
    }
    // The wraps pack a box like the grid pairs: they belong to the
    // groups narrowing starts from.
    let drawn = groups.len();
    let mut layer = rk.layer.clone();''','''    let rk = rank(g, &groups, drawn, node_cluster, walk, blocks.as_ref());
    let mut layer = rk.layer.clone();''')
rep('''/// Both layerings of the clustered layout graph `g` — the one the
/// block layout starts from ([`rank`] with `links`, long rows wrapped)
/// and [`rank`]'s without blocks — for the reports and tests that
/// compare them: the inputs whose rows the block layout leaves as they
/// were are judged by "not worse than before".
pub(crate) fn layerings(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],''','''/// Both layerings of the clustered layout graph `g` ([`rank`] with and
/// without `links`), for the reports and tests that compare them: the
/// inputs whose rows the block layout leaves as they were are judged by
/// "not worse than before".
pub(crate) fn layerings(
    g: &Graph,
    node_cluster: &[Vec<usize>],''')
rep('''    let mut block = rank(g, rank_only, drawn, Some(node_cluster), walk, Some(links));
    let mut groups = rank_only.to_vec();
    wrap_block_rows(g, sizes, node_cluster, &mut groups, &mut block, walk, links);
''','''    let block = rank(g, rank_only, drawn, Some(node_cluster), walk, Some(links));
''')
open(p,'w').write(s)
p='src/scene.rs'; s=open(p).read()
rep('''    let raw = crate::layout::node_sizes(g);
    let sane = sane_sizes(&raw);
    let sizes = fit_bars(g, &sane);
    Some(crate::layout::layerings(
        &ga,
        &sizes,
''','''    Some(crate::layout::layerings(
        &ga,
''')
open(p,'w').write(s)
