p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)

rep('''    /// Longest-path layer per REAL node.
    layer: Vec<usize>,
}

/// Steps 1 and 2: mark back-edges, then longest-path layering; with
/// `node_cluster`, every box then takes the members nothing ranks into
/// down to its first row ([`seat_box_sources`]). The groups from
/// `rank_only[drawn..]` on are narrowing's stackings and wraps: they
/// rank like the others but say nothing about where a box starts.
fn rank(
    g: &Graph,
    rank_only: &[RankGroup],
    drawn: usize,
    node_cluster: Option<&[Vec<usize>]>,
    walk: Walk,
) -> Ranking {
    let n = g.nodes.len();
    // Ranking graph: the real nodes, then one HUB per rank-only group
    // (`from -> hub` weighs one layer, `hub -> to` none), so a group
    // costs |from| + |to| links instead of every from x to pair.
    let nx = n + rank_only.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nx];
    for (ei, e) in g.edges.iter().enumerate() {
        adj[e.from].push((e.to, ei));
    }
    // Hub links ride the same DFS / layering as real edges, numbered
    // after them; `group_eid[k]` is group k's first link id.
    let ne = g.edges.len();
    let mut nlinks = ne;
    let mut group_eid = Vec::with_capacity(rank_only.len());
    for (k, (from, to)) in rank_only.iter().enumerate() {
        group_eid.push(nlinks);
        for &a in from {
            adj[a].push((n + k, nlinks));
            nlinks += 1;
        }
        for &b in to {
            adj[n + k].push((b, nlinks));
            nlinks += 1;
        }
    }
''','''    /// Longest-path layer per REAL node.
    layer: Vec<usize>,
    /// Block mode: what the block layering adds ([`block_rank`]).
    /// `None`: the layering as it was before blocks.
    blocks: Option<BlockRanking>,
}

/// What a block layering ([`block_rank`]) hands on with its layers.
#[derive(Clone, Debug, Default)]
struct BlockRanking {
    /// Per edge: a flagged stand-in ([`BlockLinks`]) — no link, so the
    /// placement gives it no channel and no pull.
    flagged: Vec<bool>,
    /// Per node: its chain of blocks as the layering took them,
    /// outermost first — its subgraph path without the subgraphs the
    /// lane rule dissolved, whose members are units of the level round
    /// them.
    paths: Vec<Vec<usize>>,
    /// The levels the lane rule fired on, in firing order (`None`: the
    /// top level).
    fired: Vec<Option<usize>>,
}

/// Steps 1 and 2: mark back-edges, then longest-path layering; with
/// `node_cluster`, every box then takes the members nothing ranks into
/// down to its first row ([`seat_box_sources`]). The groups from
/// `rank_only[drawn..]` on are narrowing's stackings and wraps: they
/// rank like the others but say nothing about where a box starts.
///
/// With `blocks` (block mode) only the back-edge marks are taken from
/// here; the layers are [`block_rank`]'s, and a flagged pair is no link.
fn rank(
    g: &Graph,
    rank_only: &[RankGroup],
    drawn: usize,
    node_cluster: Option<&[Vec<usize>]>,
    walk: Walk,
    blocks: Option<&BlockLinks>,
) -> Ranking {
    let n = g.nodes.len();
    // Block mode needs the subgraph paths; without them it is off.
    let blocks = blocks.filter(|_| node_cluster.is_some());
    let flagged_edge = |ei: usize| blocks.is_some_and(|b| b.flagged_edge(ei));
    let flagged_group = |k: usize| blocks.is_some_and(|b| b.flagged_group(k));
    // Ranking graph: the real nodes, then one HUB per rank-only group
    // (`from -> hub` weighs one layer, `hub -> to` none), so a group
    // costs |from| + |to| links instead of every from x to pair.
    let nx = n + rank_only.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nx];
    for (ei, e) in g.edges.iter().enumerate() {
        if !flagged_edge(ei) {
            adj[e.from].push((e.to, ei));
        }
    }
    // Hub links ride the same DFS / layering as real edges, numbered
    // after them; `group_eid[k]` is group k's first link id.
    let ne = g.edges.len();
    let mut nlinks = ne;
    let mut group_eid = Vec::with_capacity(rank_only.len());
    for (k, (from, to)) in rank_only.iter().enumerate() {
        group_eid.push(nlinks);
        if flagged_group(k) {
            nlinks += from.len() + to.len();
            continue;
        }
        for &a in from {
            adj[a].push((n + k, nlinks));
            nlinks += 1;
        }
        for &b in to {
            adj[n + k].push((b, nlinks));
            nlinks += 1;
        }
    }
''')

rep('''            if reversed(&alt) < reversed(&back) {
                back = alt;
            }
        }
    }

    // --- 2. Longest-path layering on the DAG (back-edges excluded) ---''','''            if reversed(&alt) < reversed(&back) {
                back = alt;
            }
        }
    }

    // Block mode: the marks above are the raw marks of the block
    // layering, which does the rest.
    if let (Some(nc), Some(links)) = (node_cluster, blocks) {
        let exact = walk == Walk::Fewest;
        let (layer, blocks) = block_rank(g, nc, rank_only, &group_eid, &back, links, exact);
        // An edge runs against the flow where its layers say so (the raw
        // marks no longer do: one dropped, or turned back by the block
        // order, points down again). A flagged group pulls nothing.
        let mut back = vec![false; nlinks];
        for (ei, e) in g.edges.iter().enumerate() {
            back[ei] = e.from == e.to || layer[e.to] < layer[e.from];
        }
        for (k, (from, to)) in rank_only.iter().enumerate() {
            if flagged_group(k) {
                for id in group_eid[k]..group_eid[k] + from.len() + to.len() {
                    back[id] = true;
                }
            }
        }
        return Ranking {
            back,
            group_eid,
            layer,
            blocks: Some(blocks),
        };
    }

    // --- 2. Longest-path layering on the DAG (back-edges excluded) ---''')

rep('''    layer.truncate(n);
    Ranking {
        back,
        group_eid,
        layer,
    }
}
''','''    layer.truncate(n);
    Ranking {
        back,
        group_eid,
        layer,
        blocks: None,
    }
}

/// A strongly connected component of a level's units is ordered exactly
/// (the fewest drawn links against the flow, by a programme over its
/// subsets) up to this many units: 4 096 subsets of 12 x 12 steps each.
/// A larger one keeps the walk's order.
const BLOCK_ORDER_EXACT_MAX: usize = 12;

/// The lane rule's share, as a fraction: a level whose every stacking
/// order turns MORE than this share of its drawn links (and in which a
/// path enters one block twice) keeps its blocks side by side.
const LANE_SHARE: (usize, usize) = (1, 3);

/// One link of the block layering between two nodes: a real edge or a
/// stand-in (`weight` 1, also an invisible one: the author asked for
/// that order; 0 for a note card's link, which is never worth turning
/// another link for), or one `from x to` pair of a rank-only group
/// (`weight` 0).
struct BlockLink {
    a: usize,
    b: usize,
    weight: usize,
    /// A cycle breaker of [`rank`]'s walk (a raw mark).
    raw: bool,
}

/// Block layering: every subgraph is ONE unit of the level round it.
/// Returns each node's layer and what the placement needs with it.
///
/// A level is the top level or a subgraph with members; its units are
/// its own nodes and its child subgraphs (blocks), in declaration order
/// ([`Tree`]). Every link belongs to the level of the lowest common
/// ancestor of its two ends and joins two units there. Deepest level
/// first:
///
/// - **Raw marks** (`raw`, [`rank`]'s cycle breakers) are kept only
///   where they break a cycle of their own level: the link's two units
///   lie in one strongly connected component of the level's links as
///   written. A kept mark is used reversed. Every other mark is dropped:
///   the node cycle it broke leaves its block and is broken once, at the
///   level of the frames it crosses (kept, the cycle would cost two
///   edges against the flow).
/// - **Block order**: two depth-first walks over the units — from the
///   units nothing links into first, and in plain declaration order,
///   each following a unit's links in declaration order of their targets
///   — turn the links into a unit still on the walk's path; the walk
///   that turns fewer drawn links is taken, the first on a tie. Per
///   strongly connected component of up to [`BLOCK_ORDER_EXACT_MAX`]
///   units the exact minimum `M` is known (ties to the earliest
///   declaration order), and the component takes that order instead when
///   the walk turns more than `2 * M + 1` of its drawn links: the
///   author's order stands until it costs over twice the minimum.
/// - **Lane rule** ([`LANE_SHARE`]): where every order of the units
///   turns more than a third of the level's drawn links as written AND
///   some path enters one child block twice, the level's child blocks
///   are dissolved for layering — their members and their own child
///   blocks become units of this level — and the level is solved again:
///   two swimlanes that hand a job back and forth stay side by side,
///   every arrow forward. A there-and-back exchange meets the first test
///   only and is stacked.
/// - **Rows** ([`level_rows`]): longest path, a unit as deep as its own
///   rows; a turned link constrains the other way round.
///
/// A node's layer is its own start plus the starts of the blocks round
/// it. So, at every level the lane rule left alone: a block's members
/// fill consecutive layers, two units a link joins never share a layer,
/// and sibling blocks never interleave.
///
/// `exact` is off past [`WALK_MAX_ELEMENTS`]: no component is solved.
/// Note cards are members of their composite and take a row there like
/// any member; a lead card ([`Graph::lead_nodes`]) then sits one row
/// before its target inside its level.
fn block_rank(
    g: &Graph,
    nc: &[Vec<usize>],
    rank_only: &[RankGroup],
    group_eid: &[usize],
    raw: &[bool],
    blocks: &BlockLinks,
    exact: bool,
) -> (Vec<usize>, BlockRanking) {
    let n = g.nodes.len();
    let ns = g.subgraphs.len();
    let card = |v: usize| {
        let node = &g.nodes[v];
        crate::style::is_note_card(&node.id, node.shape, &node.style)
    };
    let mut links: Vec<BlockLink> = Vec::new();
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from != e.to && !blocks.flagged_edge(ei) {
            links.push(BlockLink {
                a: e.from,
                b: e.to,
                weight: usize::from(!card(e.from) && !card(e.to)),
                raw: raw[ei],
            });
        }
    }
    // The drawn links end here: the lane rule's path test reads them.
    let drawn_links = links.len();
    for (k, (from, to)) in rank_only.iter().enumerate() {
        if blocks.flagged_group(k) {
            continue;
        }
        let nf = from.len();
        for (fi, &a) in from.iter().enumerate() {
            for (ti, &b) in to.iter().enumerate() {
                if a != b {
                    let raw = raw[group_eid[k] + fi] || raw[group_eid[k] + nf + ti];
                    links.push(BlockLink {
                        a,
                        b,
                        weight: 0,
                        raw,
                    });
                }
            }
        }
    }

    let mut paths: Vec<Vec<usize>> = nc.to_vec();
    let mut tree = Tree::new(&paths, ns);
    // Per level of `tree`, its links.
    let bucket = |tree: &Tree| -> Vec<Vec<usize>> {
        let mut by: Vec<Vec<usize>> = vec![Vec::new(); tree.levels.len()];
        for (k, l) in links.iter().enumerate() {
            by[tree.join(l.a, l.b).0].push(k);
        }
        by
    };
    let mut by_level = bucket(&tree);
    // Deepest level first, the top level last. A level the lane rule
    // dissolves has been solved by then, and is no level afterwards.
    let mut order: Vec<(usize, usize)> = tree
        .levels
        .iter()
        .filter_map(|l| l.sub.map(|s| (l.depth, s)))
        .collect();
    order.sort_by_key(|&(depth, s)| (std::cmp::Reverse(depth), s));
    let order: Vec<Option<usize>> = order
        .into_iter()
        .map(|(_, s)| Some(s))
        .chain([None])
        .collect();

    let mut rows = vec![1usize; ns];
    let mut start_node = vec![0usize; n];
    let mut start_sub = vec![0usize; ns];
    let mut fired: Vec<Option<usize>> = Vec::new();
    // Unit index within the level being solved.
    let mut at_node = vec![usize::MAX; n];
    let mut at_sub = vec![usize::MAX; ns];
    // The node graph of the drawn links with every raw mark reversed
    // (acyclic), in topological order: built when the lane rule first
    // asks for it.
    let mut dag: Option<(Vec<usize>, Vec<Vec<usize>>)> = None;
    for lvl in order {
        loop {
            let li = match lvl {
                None => 0,
                Some(s) => match tree.level_of_sub[s] {
                    Some(li) => li,
                    None => break,
                },
            };
            let units: Vec<LevelUnit> = tree.levels[li].units.clone();
            let m = units.len();
            for (i, &u) in units.iter().enumerate() {
                match u {
                    LevelUnit::Node(v) => at_node[v] = i,
                    LevelUnit::Block(s) => at_sub[s] = i,
                }
            }
            let at = |u: LevelUnit| match u {
                LevelUnit::Node(v) => at_node[v],
                LevelUnit::Block(s) => at_sub[s],
            };
            // The level's links as written, between unit indices.
            let written: Vec<(usize, usize)> = by_level[li]
                .iter()
                .map(|&k| {
                    let (_, ua, ub) = tree.join(links[k].a, links[k].b);
                    (at(ua), at(ub))
                })
                .collect();
            let weight = |i: usize| links[by_level[li][i]].weight;
            let comp_w = components(m, &written);
            // Kept marks reversed, dropped ones as written.
            let kept = |i: usize| {
                links[by_level[li][i]].raw && comp_w[written[i].0] == comp_w[written[i].1]
            };
            let oriented: Vec<(usize, usize)> = (0..written.len())
                .map(|i| {
                    let (x, y) = written[i];
                    if kept(i) {
                        (y, x)
                    } else {
                        (x, y)
                    }
                })
                .collect();
            // Parallel links are one arc with their summed weight.
            let mut arcs: BTreeMap<(usize, usize), usize> = BTreeMap::new();
            for (i, &xy) in oriented.iter().enumerate() {
                *arcs.entry(xy).or_insert(0) += weight(i);
            }
            let arcs: Vec<((usize, usize), usize)> = arcs.into_iter().collect();
            let arc_of = |xy: (usize, usize)| {
                arcs.binary_search_by_key(&xy, |a| a.0)
                    .expect("every oriented link has its arc")
            };
            let mut turned = block_order(m, &arcs, exact);

            // Lane rule.
            let mut sizes_w = vec![0usize; m];
            for &c in &comp_w {
                sizes_w[c] += 1;
            }
            let has_block = units.iter().any(|u| matches!(u, LevelUnit::Block(_)));
            if has_block && sizes_w.iter().any(|&k| k >= 2) {
                let total: usize = (0..written.len()).map(weight).sum();
                // The least number of drawn links, as written, any order
                // turns: exact per component where it is small enough,
                // else what the order found above turns.
                let mut least = 0usize;
                for c in (0..m).filter(|&c| sizes_w[c] >= 2) {
                    let inside = |i: usize| comp_w[written[i].0] == c && comp_w[written[i].1] == c;
                    if exact && sizes_w[c] <= BLOCK_ORDER_EXACT_MAX {
                        let members: Vec<usize> = (0..m).filter(|&u| comp_w[u] == c).collect();
                        let local = |u: usize| members.binary_search(&u).unwrap_or(0);
                        let mut w = vec![vec![0u64; members.len()]; members.len()];
                        for i in (0..written.len()).filter(|&i| inside(i)) {
                            w[local(written[i].0)][local(written[i].1)] += weight(i) as u64;
                        }
                        least += least_turned(&w).0 as usize;
                    } else {
                        least += (0..written.len())
                            .filter(|&i| inside(i) && (kept(i) != turned[arc_of(oriented[i])]))
                            .map(weight)
                            .sum::<usize>();
                    }
                }
                if LANE_SHARE.1 * least > LANE_SHARE.0 * total {
                    let (topo, out) = dag.get_or_insert_with(|| {
                        let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
                        let mut indeg = vec![0usize; n];
                        for (k, l) in links[..drawn_links].iter().enumerate() {
                            if l.weight == 0 {
                                continue;
                            }
                            let (a, b) = if l.raw { (l.b, l.a) } else { (l.a, l.b) };
                            out[a].push(k);
                            indeg[b] += 1;
                        }
                        let mut topo: Vec<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
                        let mut i = 0;
                        while i < topo.len() {
                            let u = topo[i];
                            i += 1;
                            for &k in &out[u] {
                                let l = &links[k];
                                let v = if l.raw { l.a } else { l.b };
                                indeg[v] -= 1;
                                if indeg[v] == 0 {
                                    topo.push(v);
                                }
                            }
                        }
                        (topo, out)
                    });
                    // Does a path enter one child block twice through
                    // drawn links of this level?
                    let twice = units.iter().any(|&block| {
                        if !matches!(block, LevelUnit::Block(_)) {
                            return false;
                        }
                        let mut best = vec![0u8; n];
                        for &u in topo.iter() {
                            for &k in &out[u] {
                                let l = &links[k];
                                let v = if l.raw { l.a } else { l.b };
                                let (lv, cu, cv) = tree.join(u, v);
                                let enters = lv == li && cv == block && cu != cv;
                                let reach = best[u] + u8::from(enters);
                                if reach >= 2 {
                                    return true;
                                }
                                best[v] = best[v].max(reach);
                            }
                        }
                        false
                    });
                    if twice {
                        let mut gone = vec![false; ns];
                        for &u in &units {
                            match u {
                                LevelUnit::Node(v) => at_node[v] = usize::MAX,
                                LevelUnit::Block(s) => {
                                    at_sub[s] = usize::MAX;
                                    gone[s] = true;
                                }
                            }
                        }
                        for p in paths.iter_mut() {
                            p.retain(|&c| !gone[c]);
                        }
                        tree = Tree::new(&paths, ns);
                        by_level = bucket(&tree);
                        fired.push(lvl);
                        continue;
                    }
                }
            }

            // Rows: a link not turned puts its target after its source.
            let mut cons: Vec<(usize, usize)> = oriented
                .iter()
                .map(|&(x, y)| {
                    if turned[arc_of((x, y))] {
                        (y, x)
                    } else {
                        (x, y)
                    }
                })
                .collect();
            cons.sort_unstable();
            cons.dedup();
            turned.clear();
            let unit_rows: Vec<usize> = units
                .iter()
                .map(|&u| match u {
                    LevelUnit::Node(_) => 1,
                    LevelUnit::Block(s) => rows[s],
                })
                .collect();
            let is_block: Vec<bool> = units
                .iter()
                .map(|u| matches!(u, LevelUnit::Block(_)))
                .collect();
            let mut start = level_rows(&unit_rows, &is_block, &mut cons);
            // A lead card one row before its target inside its level.
            for (i, &u) in units.iter().enumerate() {
                let LevelUnit::Node(v) = u else { continue };
                if !g.lead_nodes.contains(&v) || cons.iter().any(|&(_, j)| j == i) {
                    continue;
                }
                let to = cons
                    .iter()
                    .filter(|&&(a, _)| a == i)
                    .map(|&(_, j)| start[j])
                    .min();
                if let Some(to) = to.filter(|&t| t > start[i] + 1) {
                    start[i] = to - 1;
                }
            }
            for (i, &u) in units.iter().enumerate() {
                match u {
                    LevelUnit::Node(v) => {
                        start_node[v] = start[i];
                        at_node[v] = usize::MAX;
                    }
                    LevelUnit::Block(s) => {
                        start_sub[s] = start[i];
                        at_sub[s] = usize::MAX;
                    }
                }
            }
            if let Some(s) = lvl {
                rows[s] = (0..m)
                    .map(|i| start[i] + unit_rows[i])
                    .max()
                    .unwrap_or(1)
                    .max(1);
            }
            break;
        }
    }
    let layer: Vec<usize> = (0..n)
        .map(|v| start_node[v] + paths[v].iter().map(|&c| start_sub[c]).sum::<usize>())
        .collect();
    let flagged = (0..g.edges.len())
        .map(|ei| blocks.flagged_edge(ei))
        .collect();
    (
        layer,
        BlockRanking {
            flagged,
            paths,
            fired,
        },
    )
}

/// Which arcs of a level's unit graph run against the flow (block
/// order, see [`block_rank`]): `arcs` are `((from, to), drawn weight)`
/// between `m` units in declaration order, sorted. Two walks, the one
/// turning less drawn weight taken (the walk from the sources on a tie);
/// then, per strongly connected component solved exactly (`exact`, up to
/// [`BLOCK_ORDER_EXACT_MAX`] units), its least-turning order instead
/// where the walk turns more than `2 * M + 1` there. The arcs left are
/// acyclic: a walk turns every arc into a unit on its path, an arc
/// between two components lies on no cycle, and a component that takes
/// its exact order is a line.
fn block_order(m: usize, arcs: &[((usize, usize), usize)], exact: bool) -> Vec<bool> {
    let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); m];
    let mut indeg = vec![0usize; m];
    for (id, &((x, y), _)) in arcs.iter().enumerate() {
        out[x].push((y, id));
        indeg[y] += 1;
    }
    let walk = |roots: &mut dyn Iterator<Item = usize>| -> Vec<bool> {
        let mut state = vec![0u8; m]; // 0 unvisited, 1 on the path, 2 done
        let mut turned = vec![false; arcs.len()];
        for s in roots {
            if state[s] != 0 {
                continue;
            }
            state[s] = 1;
            let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
            while let Some(&(u, ci)) = stack.last() {
                let Some(&(v, id)) = out[u].get(ci) else {
                    state[u] = 2;
                    stack.pop();
                    continue;
                };
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                match state[v] {
                    0 => {
                        state[v] = 1;
                        stack.push((v, 0));
                    }
                    1 => turned[id] = true,
                    _ => {}
                }
            }
        }
        turned
    };
    let cost = |turned: &[bool]| -> usize {
        arcs.iter()
            .zip(turned)
            .filter(|(_, &t)| t)
            .map(|(a, _)| a.1)
            .sum()
    };
    let from_sources = walk(&mut (0..m).filter(|&u| indeg[u] == 0).chain(0..m));
    let declared = walk(&mut (0..m));
    let mut turned = if cost(&declared) < cost(&from_sources) {
        declared
    } else {
        from_sources
    };
    if !exact || !turned.iter().any(|&t| t) {
        return turned;
    }
    // The safety net, per strongly connected component.
    let pairs: Vec<(usize, usize)> = arcs.iter().map(|a| a.0).collect();
    let comp = components(m, &pairs);
    let mut size = vec![0usize; m];
    for &c in &comp {
        size[c] += 1;
    }
    for c in (0..m).filter(|&c| (2..=BLOCK_ORDER_EXACT_MAX).contains(&size[c])) {
        let members: Vec<usize> = (0..m).filter(|&u| comp[u] == c).collect();
        let local = |u: usize| members.binary_search(&u).unwrap_or(0);
        let inside: Vec<usize> = (0..arcs.len())
            .filter(|&id| comp[arcs[id].0 .0] == c && comp[arcs[id].0 .1] == c)
            .collect();
        let walked: usize = inside
            .iter()
            .filter(|&&id| turned[id])
            .map(|&id| arcs[id].1)
            .sum();
        if walked <= 1 {
            continue; // never above 2 * M + 1
        }
        // Drawn weight first; among equals, fewer undrawn arcs turned.
        let big = inside.len() as u64 + 1;
        let mut w = vec![vec![0u64; members.len()]; members.len()];
        for &id in &inside {
            let ((x, y), drawn) = arcs[id];
            w[local(x)][local(y)] += drawn as u64 * big + u64::from(drawn == 0);
        }
        let (least, pos) = least_turned(&w);
        if walked as u64 > 2 * (least / big) + 1 {
            for &id in &inside {
                let (x, y) = arcs[id].0;
                turned[id] = pos[local(x)] > pos[local(y)];
            }
        }
    }
    turned
}

/// The order of `w.len()` units (at most [`BLOCK_ORDER_EXACT_MAX`]) that
/// turns the least weight, `w[i][j]` being the weight of the links
/// `i -> j` (turned when `j` comes before `i`): that weight, and each
/// unit's position. Among the best orders the lexicographically smallest
/// sequence of unit indices. A programme over the subsets placed so far.
fn least_turned(w: &[Vec<u64>]) -> (u64, Vec<usize>) {
    let k = w.len();
    debug_assert!(k <= BLOCK_ORDER_EXACT_MAX);
    let full = (1usize << k) - 1;
    // Placing `j` next turns every link into it from a unit not placed.
    let step = |set: usize, j: usize| -> u64 {
        (0..k)
            .filter(|&i| i != j && set & (1 << i) == 0)
            .map(|i| w[i][j])
            .sum()
    };
    let mut best = vec![0u64; full + 1];
    for set in (0..full).rev() {
        best[set] = (0..k)
            .filter(|&j| set & (1 << j) == 0)
            .map(|j| step(set, j) + best[set | (1 << j)])
            .min()
            .unwrap_or(0);
    }
    let mut pos = vec![0usize; k];
    let mut set = 0usize;
    for place in 0..k {
        let next = (0..k)
            .filter(|&j| set & (1 << j) == 0)
            .find(|&j| step(set, j) + best[set | (1 << j)] == best[set]);
        let Some(j) = next else { break };
        pos[j] = place;
        set |= 1 << j;
    }
    (best[0], pos)
}

/// Strongly connected components of the graph of `m` vertices and
/// `arcs`: each vertex's component, named by its smallest vertex.
/// Kosaraju's two passes, without recursion.
fn components(m: usize, arcs: &[(usize, usize)]) -> Vec<usize> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); m];
    let mut into: Vec<Vec<usize>> = vec![Vec::new(); m];
    for &(x, y) in arcs {
        out[x].push(y);
        into[y].push(x);
    }
    // Vertices by finishing time of a depth-first walk.
    let mut seen = vec![false; m];
    let mut finished: Vec<usize> = Vec::with_capacity(m);
    for s in 0..m {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
        while let Some(&(u, ci)) = stack.last() {
            match out[u].get(ci) {
                Some(&v) => {
                    if let Some(top) = stack.last_mut() {
                        top.1 += 1;
                    }
                    if !seen[v] {
                        seen[v] = true;
                        stack.push((v, 0));
                    }
                }
                None => {
                    finished.push(u);
                    stack.pop();
                }
            }
        }
    }
    let mut comp = vec![usize::MAX; m];
    for &s in finished.iter().rev() {
        if comp[s] != usize::MAX {
            continue;
        }
        let mut members = vec![s];
        comp[s] = s;
        let mut i = 0;
        while i < members.len() {
            let u = members[i];
            i += 1;
            for &v in &into[u] {
                if comp[v] == usize::MAX {
                    comp[v] = s;
                    members.push(v);
                }
            }
        }
        let name = members.iter().copied().min().unwrap_or(s);
        for v in members {
            comp[v] = name;
        }
    }
    comp
}

/// Rows of one level (see [`block_rank`]): each unit's first row, by
/// longest path over `cons` (`(i, j)`: unit `j` starts after unit `i`'s
/// last row; `rows[i]` rows per unit), then two refinements repeated in
/// this order until neither changes anything:
///
/// 1. *A source block sits next to what it feeds.* A block nothing
///    constrains starts just above the first unit it constrains (the
///    units it links to and the blocks rule 2 put after it) when that is
///    later, and keeps that start as a floor. Plain source nodes stay on
///    the first row.
/// 2. *No interleaving.* While two blocks `A`, `B` overlap in rows
///    without one holding the other (`start(A) < start(B) < end(A) <
///    end(B)`), `B` starts after `A`: the pair with the earliest
///    `start(A)`, then the earliest `start(B)`, then declaration order.
///    The constraint points from the earlier start to the later, so it
///    closes no cycle, and it is added to `cons`.
///
/// Rule 1 moves a block into its own slack only and rule 2 adds at most
/// one constraint per pair of blocks, so the loop ends within (pairs of
/// blocks + 2) passes.
fn level_rows(rows: &[usize], is_block: &[bool], cons: &mut Vec<(usize, usize)>) -> Vec<usize> {
    let m = rows.len();
    let solve = |cons: &[(usize, usize)], floor: &[usize]| -> Vec<usize> {
        let mut out: Vec<Vec<usize>> = vec![Vec::new(); m];
        let mut indeg = vec![0usize; m];
        for &(i, j) in cons {
            out[i].push(j);
            indeg[j] += 1;
        }
        let mut start = floor.to_vec();
        let mut queue: VecDeque<usize> = (0..m).filter(|&u| indeg[u] == 0).collect();
        let mut done = 0;
        while let Some(u) = queue.pop_front() {
            done += 1;
            for &v in &out[u] {
                start[v] = start[v].max(start[u] + rows[u]);
                indeg[v] -= 1;
                if indeg[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
        debug_assert_eq!(done, m, "the constraints of a level are acyclic");
        start
    };
    let mut floor = vec![0usize; m];
    let mut start = solve(cons, &floor);
    let blocks: Vec<usize> = (0..m).filter(|&u| is_block[u]).collect();
    let pairs = blocks.len() * blocks.len().saturating_sub(1) / 2;
    for pass in 0.. {
        debug_assert!(pass < pairs + 2, "the rows of a level settle");
        if pass >= pairs + 2 {
            break;
        }
        let mut changed = false;
        for &s in &blocks {
            if cons.iter().any(|&(_, j)| j == s) {
                continue;
            }
            let near = cons
                .iter()
                .filter(|&&(i, _)| i == s)
                .map(|&(_, j)| start[j])
                .min();
            if let Some(to) = near.and_then(|t| t.checked_sub(rows[s])) {
                if to > start[s] {
                    floor[s] = to;
                    start = solve(cons, &floor);
                    changed = true;
                }
            }
        }
        loop {
            let end = |u: usize| start[u] + rows[u];
            let pair = blocks
                .iter()
                .flat_map(|&a| blocks.iter().map(move |&b| (a, b)))
                .filter(|&(a, b)| start[a] < start[b] && start[b] < end(a) && end(a) < end(b))
                .min_by_key(|&(a, b)| (start[a], start[b], a, b));
            let Some(pair) = pair else { break };
            if cons.contains(&pair) {
                break;
            }
            cons.push(pair);
            start = solve(cons, &floor);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    start
}
''')
open(p,'w').write(s)
