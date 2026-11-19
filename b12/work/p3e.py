p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''            let mut turned = block_order(m, &arcs, exact);
''','''            let is_block: Vec<bool> = units
                .iter()
                .map(|u| matches!(u, LevelUnit::Block(_)))
                .collect();
            let mut turned = block_order(&is_block, &arcs, exact);
''')
rep('''            let has_block = units.iter().any(|u| matches!(u, LevelUnit::Block(_)));
            if has_block && sizes_w.iter().any(|&k| k >= 2) {''','''            let has_block = is_block.iter().any(|&b| b);
            if has_block && sizes_w.iter().any(|&k| k >= 2) {''')
rep('''            let is_block: Vec<bool> = units
                .iter()
                .map(|u| matches!(u, LevelUnit::Block(_)))
                .collect();
            let mut start = level_rows(&unit_rows, &is_block, &mut cons);''','''            let mut start = level_rows(&unit_rows, &is_block, &mut cons);''')
rep('''/// Which arcs of a level's unit graph run against the flow (block
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
    let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); m];''','''/// Which arcs of a level's unit graph run against the flow (block
/// order, see [`block_rank`]): `arcs` are `((from, to), drawn weight)`
/// between the units (`is_block` per unit, in declaration order),
/// sorted. Two walks, the one turning less drawn weight taken (the walk
/// from the sources on a tie); then, per strongly connected component
/// solved exactly (`exact`, up to [`BLOCK_ORDER_EXACT_MAX`] units), its
/// least-turning order instead where the walk turns more than
/// `2 * M + 1` there. The arcs left are acyclic: a walk turns every arc
/// into a unit on its path, an arc between two components lies on no
/// cycle, and a component that takes its exact order is a line.
///
/// A plain node nothing links into starts neither walk. It sits on the
/// first row wherever it is declared, so its place in the file must not
/// order the blocks below it: with the walk started at the first such
/// node, two actors declared the other way round stacked the public
/// fixture's blocks the other way round (the shared services below the
/// clusters they feed). The walk from the sources starts at what those
/// nodes link to, in declaration order of the targets, then at the
/// blocks nothing links into; the declared walk leaves them for last.
fn block_order(is_block: &[bool], arcs: &[((usize, usize), usize)], exact: bool) -> Vec<bool> {
    let m = is_block.len();
    let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); m];''')
rep('''    let from_sources = walk(&mut (0..m).filter(|&u| indeg[u] == 0).chain(0..m));
    let declared = walk(&mut (0..m));''','''    let loose = |u: usize| indeg[u] == 0 && !is_block[u];
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
    );
    let declared = walk(
        &mut (0..m)
            .filter(|&u| !loose(u))
            .chain((0..m).filter(|&u| loose(u))),
    );''')
open(p,'w').write(s)
