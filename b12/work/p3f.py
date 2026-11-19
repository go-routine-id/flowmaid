p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''/// A plain node nothing links into starts neither walk. It sits on the
/// first row wherever it is declared, so its place in the file must not
/// order the blocks below it: with the walk started at the first such
/// node, two actors declared the other way round stacked the public
/// fixture's blocks the other way round (the shared services below the
/// clusters they feed). The walk from the sources starts at what those
/// nodes link to, in declaration order of the targets, then at the
/// blocks nothing links into; the declared walk leaves them for last.''','''/// A plain node nothing links into starts neither walk. It sits on the
/// first row wherever it is declared, so its place in the file must not
/// order the blocks below it: with the walk started at the first such
/// node, two actors declared the other way round stacked the public
/// fixture's blocks the other way round (the shared services below the
/// clusters they feed). The walk from the sources starts at what those
/// nodes link to — the unit most of their drawn links enter first, then
/// declaration order — and then at the blocks nothing links into; the
/// declared walk leaves those nodes for last.''')
rep('''    let loose = |u: usize| indeg[u] == 0 && !is_block[u];
    let mut fed = vec![false; m];
    for u in (0..m).filter(|&u| loose(u)) {
        for &(v, _) in &out[u] {
            fed[v] = true;
        }
    }
    let from_sources = walk(
        &mut (0..m)
            .filter(|&u| fed[u])
            .chain((0..m).filter(|&u| indeg[u] == 0 && is_block[u]))
            .chain(0..m),
    );''','''    let loose = |u: usize| indeg[u] == 0 && !is_block[u];
    // Per unit: the drawn links into it from loose nodes.
    let mut fed = vec![0usize; m];
    let mut entered = vec![false; m];
    for u in (0..m).filter(|&u| loose(u)) {
        for &(v, id) in &out[u] {
            fed[v] += arcs[id].1;
            entered[v] = true;
        }
    }
    let mut entries: Vec<usize> = (0..m).filter(|&u| entered[u]).collect();
    entries.sort_by_key(|&u| (std::cmp::Reverse(fed[u]), u));
    let from_sources = walk(
        &mut entries
            .into_iter()
            .chain((0..m).filter(|&u| indeg[u] == 0 && is_block[u]))
            .chain(0..m),
    );''')
open(p,'w').write(s)
