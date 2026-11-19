p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''/// Both layerings of the clustered layout graph `g` ([`rank`] with and
/// without `links`), for the reports and tests that compare them: the
/// inputs whose rows the block layout leaves as they were are judged by
/// "not worse than before".
pub(crate) fn layerings(
    g: &Graph,
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
    links: &BlockLinks,
) -> Layerings {
    let walk = if g.nodes.len() + g.edges.len() <= WALK_MAX_ELEMENTS {
        Walk::Fewest
    } else {
        Walk::Source
    };
    let drawn = rank_only.len();
    let before = rank(g, rank_only, drawn, Some(node_cluster), walk, None);
    let block = rank(g, rank_only, drawn, Some(node_cluster), walk, Some(links));
    let upstream''','''/// Both layerings of the clustered layout graph `g` — the one the
/// block layout starts from ([`rank`] with `links`, long rows wrapped)
/// and [`rank`]'s without blocks — for the reports and tests that
/// compare them: the inputs whose rows the block layout leaves as they
/// were are judged by "not worse than before".
pub(crate) fn layerings(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
    links: &BlockLinks,
) -> Layerings {
    let walk = if g.nodes.len() + g.edges.len() <= WALK_MAX_ELEMENTS {
        Walk::Fewest
    } else {
        Walk::Source
    };
    let drawn = rank_only.len();
    let before = rank(g, rank_only, drawn, Some(node_cluster), walk, None);
    let mut block = rank(g, rank_only, drawn, Some(node_cluster), walk, Some(links));
    let mut groups = rank_only.to_vec();
    wrap_block_rows(g, sizes, node_cluster, &mut groups, &mut block, walk, links);
    let upstream''')
open(p,'w').write(s)
p='src/scene.rs'; s=open(p).read()
rep('''    let links = aug.block_links(&stand_in, ga.edges.len());
    Some(crate::layout::layerings(
        &ga,
        &aug.node_cluster,''','''    let links = aug.block_links(&stand_in, ga.edges.len());
    let sizes = fit_bars(g, &sane_sizes(&crate::layout::node_sizes(g)));
    Some(crate::layout::layerings(
        &ga,
        &sizes,
        &aug.node_cluster,''')
open(p,'w').write(s)
