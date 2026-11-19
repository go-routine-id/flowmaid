p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''    let groups: Vec<RankGroup> = rank_only.to_vec();
    let drawn = groups.len();''','''    let mut groups: Vec<RankGroup> = rank_only.to_vec();''')
rep('''    let rk = rank(g, &groups, drawn, node_cluster, walk, blocks.as_ref());
    let mut layer = rk.layer.clone();''','''    let mut rk = rank(
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
    let mut layer = rk.layer.clone();''')
rep('''/// A strongly connected component of a level's units is ordered exactly''','''/// Block mode: wrap the long rows of members without inner links.
///
/// A member of a block with no link to anything else inside the block
/// lies on the block's first row, so a block of such members is one
/// row however many they are. Where that row is more than
/// [`RIBBON_ENTRY`] times as broad as the block is deep, and the block
/// has room below it in its band, the row is wrapped: member `i` ranks
/// above member `i + cols` (rank-only pairs, as the grid of loose
/// members is packed), in as many rows as bring the block under that
/// ratio — or as fit: a wrap that would add a layer to the drawing is
/// not taken, and one row fewer is tried. So no block row is longer
/// than its band needs, and a block alone in its band keeps its row.
///
/// `groups` gains the pairs of the wraps taken and `rk` is the layering
/// with them.
fn wrap_block_rows(
    g: &Graph,
    sizes: &[(f64, f64)],
    nc: &[Vec<usize>],
    groups: &mut Vec<RankGroup>,
    rk: &mut Ranking,
    walk: Walk,
    links: &BlockLinks,
) {
    let n = g.nodes.len();
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let breadth = |v: usize| if horizontal { sizes[v].1 } else { sizes[v].0 };
    let along = |v: usize| if horizontal { sizes[v].0 } else { sizes[v].1 };
    let layers = |rk: &Ranking| rk.layer.iter().copied().max().map_or(0, |m| m + 1);
    for c in 0..g.subgraphs.len() {
        let Some(paths) = rk.blocks.as_ref().map(|b| &b.paths) else {
            return;
        };
        let inside = |v: usize| paths[v].contains(&c);
        if !(0..n).any(inside) {
            continue;
        }
        // Members with a link to something else inside the block.
        let mut linked = vec![false; n];
        let mut join = |a: usize, b: usize| {
            if a != b && inside(a) && inside(b) {
                linked[a] = true;
                linked[b] = true;
            }
        };
        for (ei, e) in g.edges.iter().enumerate() {
            if !links.flagged_edge(ei) {
                join(e.from, e.to);
            }
        }
        for (k, (from, to)) in groups.iter().enumerate() {
            if links.flagged_group(k) {
                continue;
            }
            for &a in from {
                for &b in to {
                    join(a, b);
                }
            }
        }
        let loose: Vec<usize> = (0..n)
            .filter(|&v| paths[v].last() == Some(&c) && !linked[v])
            .collect();
        let k = loose.len();
        if k < 3 || loose.iter().any(|&v| rk.layer[v] != rk.layer[loose[0]]) {
            continue;
        }
        // The block's depth: its rows' deepest members and the gaps
        // between the rows.
        let mut row_depth: BTreeMap<usize, f64> = BTreeMap::new();
        for v in (0..n).filter(|&v| inside(v)) {
            let d = row_depth.entry(rk.layer[v]).or_insert(0.0);
            *d = d.max(along(v));
        }
        let depth: f64 =
            row_depth.values().sum::<f64>() + GAP_L * (row_depth.len() as f64 - 1.0).max(0.0);
        let row: f64 = loose.iter().map(|&v| breadth(v)).sum::<f64>() + GAP_B * (k as f64 - 1.0);
        if row <= RIBBON_ENTRY * depth {
            continue;
        }
        // The fewest rows that bring the wrapped row under the ratio.
        let step = loose.iter().map(|&v| along(v)).fold(0.0, f64::max) + GAP_L;
        let fits = |rows: usize| {
            let cols = k.div_ceil(rows);
            row * cols as f64 / k as f64 <= RIBBON_ENTRY * (depth + step * (rows as f64 - 1.0))
        };
        let want = (2..k).find(|&rows| fits(rows)).unwrap_or(k);
        let before = layers(rk);
        for rows in (2..=want).rev() {
            let cols = k.div_ceil(rows);
            let mut cand = groups.clone();
            for i in cols..k {
                cand.push((vec![loose[i - cols]], vec![loose[i]]));
            }
            let crk = rank(g, &cand, cand.len(), Some(nc), walk, Some(links));
            if layers(&crk) <= before {
                *groups = cand;
                *rk = crk;
                break;
            }
        }
    }
}

/// A strongly connected component of a level's units is ordered exactly''')
open(p,'w').write(s)
