p='src/layout.rs'; s=open(p).read()
def rep(a,b,cnt=1):
    global s
    assert s.count(a)==cnt, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''            &alayer,
            bpos,
            &edge_chain,
        )
    };''','''            &alayer,
            bpos,
            &edge_chain,
            block_paths,
        )
    };''')
open(p,'w').write(s)

p='src/scene.rs'; s=open(p).read()
rep('''    let lo = layout_sized(g, sizes);
    let lanes = static_lanes(g, &lo, None, g.edges.len());''','''    let lo = layout_sized(g, sizes);
    let lanes = static_lanes(g, &lo, None, None, g.edges.len());''')
rep('''/// ([`crate::layout::back_lanes`]). Only the first `real` edges get one
/// (the rest are `scene_clustered`'s stand-ins).
fn static_lanes(
    g: &Graph,
    lo: &LayoutResult,
    node_cluster: Option<&[Vec<usize>]>,
    real: usize,
) -> Vec<Option<crate::lanes::Lane>> {''','''/// ([`crate::layout::back_lanes`]). Only the first `real` edges get one
/// (the rest are `scene_clustered`'s stand-ins). In a block layout
/// (`blocks`, its [`crate::layout::BlockPaths`]) only an upstream edge
/// between two plain nodes of one level has a lane.
fn static_lanes(
    g: &Graph,
    lo: &LayoutResult,
    node_cluster: Option<&[Vec<usize>]>,
    blocks: Option<&[Vec<usize>]>,
    real: usize,
) -> Vec<Option<crate::lanes::Lane>> {''')
rep('''    for (ei, lane) in crate::layout::back_lanes(g, &at, node_cluster, &chains, |d| dummies[d]) {''','''    let lanes = crate::layout::back_lanes(g, &at, node_cluster, blocks, &chains, |d| dummies[d]);
    for (ei, lane) in lanes {''')
rep('''    let (lo, groups) = layout_clustered_ranked(&ga, sizes, &node_cluster, &rank_only);
    let lanes = static_lanes(&ga, &lo, Some(&node_cluster), ne);
    let mut sc = scene_from_layout(&ga, sizes, labels, lo, &lanes, ne);''','''    let (lo, groups, block_paths) =
        layout_clustered_ranked(&ga, sizes, &node_cluster, &rank_only);
    let lanes = static_lanes(
        &ga,
        &lo,
        Some(&node_cluster),
        block_paths.as_deref(),
        ne,
    );
    let mut sc = scene_from_layout(&ga, sizes, labels, lo, &lanes, ne);
    // A block layout's chain carries its label at the slot the layout
    // sized for it: the middle of its run between the end blocks
    // ([`crate::layout::label_slot`]), not the middle of the whole chain.
    if block_paths.is_some() {
        for (ei, (ge, se)) in ga.edges.iter().zip(sc.edges.iter_mut()).enumerate() {
            let (Some(label), false) = (se.label.as_mut(), groups[ei].is_empty()) else {
                continue;
            };
            let common: Vec<usize> = node_cluster[ge.from]
                .iter()
                .zip(&node_cluster[ge.to])
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| *a)
                .collect();
            let slot = crate::layout::label_slot(&groups[ei], &common);
            if let Some(&at) = se.waypoints.get(slot + 1) {
                label.1 = at;
            }
        }
    }''')
open(p,'w').write(s)
