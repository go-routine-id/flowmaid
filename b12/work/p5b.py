p='src/layout.rs'; s=open(p).read()
def rep(a,b,cnt=1):
    global s
    assert s.count(a)==cnt, (s.count(a), a[:70])
    s=s.replace(a,b)

# place signature + Placement
rep('''    /// The depth the ribbon tests read (G12): the judged layer lines'
    /// (see `place`'s `layer_lines`).
    depth: f64,
}''','''    /// The depth the ribbon tests read (G12): the judged layer lines'
    /// (see `place`'s `layer_lines`).
    depth: f64,
    /// A block layout's final order ([`OrderSeed`]).
    order: Option<OrderSeed>,
}''')
rep('''/// Steps 3 to 5 for one layering: dummy chains, ordering and
/// coordinates. `layer` holds one layer per real node and must keep
/// every non-back ranking link pointing down by at least its weight.
fn place(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
    opts: Opts,
) -> Placement {''','''/// Steps 3 to 5 for one layering: dummy chains, ordering and
/// coordinates. `layer` holds one layer per real node and must keep
/// every non-back ranking link pointing down by at least its weight.
/// `seed`: the order a block layout's later placement starts from
/// ([`BlockOrder::run`]).
#[allow(clippy::too_many_arguments)] // internal: one bundle of layering state
fn place(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
    opts: Opts,
    seed: Option<&OrderSeed>,
) -> Placement {''')

# ordering branch
rep('''    let mut pos = vec![0.0f64; na];
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }
    if clustered {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    let mut best_layers = layers.clone();
    let mut best_cross = count_crossings(&layers, &succs, &alayer, nlayers);
    for _ in 0..8 {''','''    let mut pos = vec![0.0f64; na];
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }
    // A block layout orders level first ([`BlockOrder`]).
    let mut dummy_edge = vec![usize::MAX; na];
    for (ei, chain) in edge_chain.iter().enumerate() {
        for &d in chain {
            dummy_edge[d] = ei;
        }
    }
    let element = |v: usize| -> Option<(usize, usize)> {
        if v < n {
            Some((v, usize::MAX))
        } else {
            (dummy_edge[v] != usize::MAX).then(|| (dummy_edge[v], alayer[v]))
        }
    };
    let mut block_order = match (block_paths, node_cluster) {
        (Some(_), Some(nc)) => {
            let ns = g.subgraphs.len();
            let mut first = vec![usize::MAX; ns];
            let mut path_of: Vec<Option<Vec<usize>>> = vec![None; ns];
            for (v, path) in nc.iter().enumerate() {
                for (i, &c) in path.iter().enumerate() {
                    first[c] = first[c].min(v);
                    if path_of[c].is_none() {
                        path_of[c] = Some(path[..=i].to_vec());
                    }
                }
            }
            let mut canon = vec![(0usize, 0u8, 0usize, 0usize); na];
            for (v, key) in canon.iter_mut().enumerate().take(n) {
                *key = (v, 0, 0, 0);
            }
            let mut run = vec![usize::MAX; na];
            let mut runs = 0;
            for (ei, chain) in edge_chain.iter().enumerate() {
                let e = &g.edges[ei];
                let mut prev: Option<usize> = None;
                for &d in chain {
                    canon[d] = (e.from, 1, e.to, ei);
                    match prev {
                        Some(p) if apath[p] == apath[d] => run[d] = run[p],
                        _ => {
                            run[d] = runs;
                            runs += 1;
                        }
                    }
                    prev = Some(d);
                }
            }
            for (d, side, path) in &border_meta {
                let c = path.last().copied().unwrap_or(0);
                canon[*d] = (first[c], 2, usize::from(*side), 0);
            }
            let mut levels: Vec<(Vec<usize>, usize, usize)> =
                vec![(Vec::new(), 0, nlayers.saturating_sub(1))];
            let mut subs: Vec<usize> = cluster_span.keys().copied().collect();
            subs.sort_by_key(|&c| (path_of[c].as_ref().map_or(0, Vec::len), first[c]));
            for c in subs {
                if let (Some(path), Some(&(lo, hi))) = (path_of[c].clone(), cluster_span.get(&c)) {
                    levels.push((path, lo, hi));
                }
            }
            Some(BlockOrder {
                apath: &apath,
                border: &border_side,
                preds: &preds,
                succs: &succs,
                alayer: &alayer,
                nlayers,
                canon,
                first,
                run,
                runs,
                levels,
                brank: vec![0.0; ns],
                slot: vec![0.0; na],
            })
        }
        _ => None,
    };
    if let Some(order) = block_order.as_mut() {
        let links: usize = succs.iter().map(Vec::len).sum();
        let starts = if opts.judge {
            (BLOCK_ORDER_WORK / (na + links).max(1)).clamp(1, BLOCK_ORDER_STARTS)
        } else {
            1
        };
        order.run(
            starts,
            seed.map(|s| (s, &element as &dyn Fn(usize) -> Option<(usize, usize)>)),
            &mut layers,
            &mut pos,
        );
    }
    if clustered && block_order.is_none() {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    let mut best_layers = layers.clone();
    let mut best_cross = count_crossings(&layers, &succs, &alayer, nlayers);
    for _ in 0..8 {
        if block_order.is_some() {
            break;
        }''')
rep('''    if clustered && !bar_cards(g).is_empty() {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    // A subgraph [`centre_largest`] moves: its key changes, every layer
    // regroups around it.
    if let (true, Some((c, key))) = (clustered, opts.hint) {''','''    if let (Some(order), false) = (block_order.as_ref(), bar_cards(g).is_empty()) {
        for lv in layers.iter_mut() {
            order.regroup(lv, &mut pos);
        }
    } else if clustered && !bar_cards(g).is_empty() {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    // A subgraph [`centre_largest`] moves: its key changes, every layer
    // regroups around it. (Not in a block layout: the level orders
    // placed its blocks.)
    if let (true, Some((c, key)), true) = (clustered, opts.hint, block_order.is_none()) {''')
rep('''        groups: edge_groups,
        crossings,
        hits,
        ckeys,
        ext,
        depth,
    }
}''','''        groups: edge_groups,
        crossings,
        hits,
        ckeys,
        ext,
        depth,
        order,
    }
}''')
rep('''    let edge_groups = edge_chain
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|&d| apath.get(d).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
''','''    let edge_groups = edge_chain
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|&d| apath.get(d).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    let order: Option<OrderSeed> = block_paths.map(|_| {
        layers
            .iter()
            .map(|lv| lv.iter().filter_map(|&v| element(v)).collect())
            .collect()
    });
''')
# callers
rep('''    let first = place(g, sizes, node_cluster, &groups, &rk, &layer, plain);
    let mut st = State {''','''    let first = place(g, sizes, node_cluster, &groups, &rk, &layer, plain, None);
    let mut st = State {
        seed: first.order.clone(),''')
rep('''    /// Block mode ([`rank`]): every candidate layering is a block
    /// layering.
    blocks: Option<BlockLinks>,''','''    /// Block mode ([`rank`]): every candidate layering is a block
    /// layering.
    blocks: Option<BlockLinks>,
    /// A block layout: the first placement's final order, which every
    /// later placement starts from ([`BlockOrder::run`]).
    seed: Option<OrderSeed>,''')
rep('''        &st.rk,
        &st.layer,
        packed_opts,
    );
    if packed.lo.total_b <= st.best.lo.total_b && packed.crossings <= cap {''','''        &st.rk,
        &st.layer,
        packed_opts,
        st.seed.as_ref(),
    );
    if packed.lo.total_b <= st.best.lo.total_b && packed.crossings <= cap {''')
rep('''                    &cand_groups,
                    &crk,
                    &crk.layer,
                    packed_opts,
                );''','''                    &cand_groups,
                    &crk,
                    &crk.layer,
                    packed_opts,
                    st.seed.as_ref(),
                );''')
rep('''                &st.groups,
                &st.rk,
                &tight,
                packed_opts,
            );''','''                &st.groups,
                &st.rk,
                &tight,
                packed_opts,
                st.seed.as_ref(),
            );''')
rep('''                &st.groups,
                &st.rk,
                &cand_layer,
                packed_opts,
            );''','''                &st.groups,
                &st.rk,
                &cand_layer,
                packed_opts,
                st.seed.as_ref(),
            );''')
rep('''        let cand = place(g, sizes, node_cluster, &st.groups, &st.rk, &st.layer, opts);''','''        let cand = place(
            g,
            sizes,
            node_cluster,
            &st.groups,
            &st.rk,
            &st.layer,
            opts,
            st.seed.as_ref(),
        );''')
open(p,'w').write(s)
