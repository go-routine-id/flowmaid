//! Layer assignment for the layered layout ([`crate::layout`] steps 1
//! and 2): which links break the cycles, then which layer every node
//! takes (report §2.9, B2).
//!
//! - **Cycle breaking**: a greedy Eades–Lin–Smyth feedback-arc set,
//!   refined by sifting, run inside each strongly connected component
//!   only — a link that lies on no cycle never turns, and among the
//!   links that do, as few (by weight) as the heuristics find. The
//!   depth-first walk it replaces turned whatever its declaration-order
//!   walk met last: bench-75's `svc2 --> db` and `svc4 --> cache` (the
//!   majority direction) became back-edges, 9 reversals where 5 do.
//!   With subgraphs it runs level by level ([`flow_order`]): the sibling
//!   units of one box (its own nodes and its child boxes, each box as
//!   one unit) are ordered over the links between them first, then each
//!   child box inside, so every box stays contiguous along the flow and
//!   two sibling boxes linked both ways turn the minority links instead
//!   of interleaving (cluster-level acyclicity).
//! - **Ranking**: network simplex (Gansner et al., the ranker dagre and
//!   so mermaid use) over the links, cycle breakers reversed, plus
//!   nesting links ([`nesting`]) that weigh every box's span of layers
//!   above all other lengths, so a box takes as few layers as its own
//!   edges allow, an edge-less member sits inside its box's layers (not
//!   on the first one), and a source sits right above its nearest
//!   target. It starts from the longest-path layering and only moves a
//!   node when that shortens the drawing, so where nothing is gained the
//!   layering stays exactly what longest-path gives.
//!
//! Every choice breaks ties by index (declaration order): the result is
//! deterministic and does not depend on the order edges are written in,
//! beyond those ties.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// One ranking constraint: `to` sits at least `minlen` layers below
/// `from`, and each layer the link spans costs `weight`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Link {
    pub from: usize,
    pub to: usize,
    pub minlen: i64,
    pub weight: i64,
}

/// Sifting passes per strongly connected component (each pass moves
/// every node once to its best place; most graphs settle in two).
const SIFT_PASSES: usize = 6;
/// Components larger than this skip sifting: a pass costs the square of
/// their size, and the greedy order alone is already good.
const SIFT_MAX: usize = 3_000;
/// Work allowed for the network simplex pivots, counted as the nodes
/// plus links each pivot walks. A graph that would need more keeps the
/// feasible layering reached so far (every constraint still holds;
/// only the total edge length is not minimal). Small diagrams never
/// come near it.
const SIMPLEX_WORK: usize = 40_000_000;

/// The cycle breakers of `links` over `n` nodes: `true` for a link that
/// points against the flow order ([`flow_order`]), and for a self-loop.
/// `paths[v]`, when given, is node `v`'s subgraph path (outermost
/// first): boxes stay contiguous in the order. `weights` scale each
/// link's cost of being turned.
pub(crate) fn feedback_links(
    n: usize,
    links: &[(usize, usize)],
    weights: &[i64],
    paths: Option<&[Vec<usize>]>,
) -> Vec<bool> {
    let pos = flow_order(n, links, weights, paths);
    links
        .iter()
        .map(|&(a, b)| a == b || pos[a] > pos[b])
        .collect()
}

/// A unit of one level of [`flow_order`]: a node, or a child box.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Node(usize),
    Box(usize),
}

/// Position of every node in a linear order where as few links (by
/// weight) as found point backwards, boxes contiguous. Without paths
/// all nodes form one level.
fn flow_order(
    n: usize,
    links: &[(usize, usize)],
    weights: &[i64],
    paths: Option<&[Vec<usize>]>,
) -> Vec<usize> {
    let empty: Vec<Vec<usize>> = Vec::new();
    let paths: &[Vec<usize>] = paths.unwrap_or(&empty);
    let path = |v: usize| -> &[usize] { paths.get(v).map_or(&[], |p| p.as_slice()) };
    // Each link is decided on the level of the deepest box holding both
    // its ends: keyed by that box (`None` = the top level).
    let mut level_links: std::collections::BTreeMap<Option<usize>, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (li, &(a, b)) in links.iter().enumerate() {
        if a == b {
            continue;
        }
        let (pa, pb) = (path(a), path(b));
        let cp = pa.iter().zip(pb).take_while(|(x, y)| x == y).count();
        let key = cp.checked_sub(1).map(|i| pa[i]);
        level_links.entry(key).or_default().push(li);
    }
    let mut order: Vec<usize> = Vec::with_capacity(n);
    // Scratch: unit index per node on the level being ordered.
    let mut uidx = vec![usize::MAX; n];
    // Levels to expand, depth-first so each box's members land where
    // its unit stands: (depth, box, members ascending).
    enum Step {
        Level(usize, Option<usize>, Vec<usize>),
        Emit(usize),
    }
    let mut stack: Vec<Step> = vec![Step::Level(0, None, (0..n).collect())];
    while let Some(step) = stack.pop() {
        let (depth, key, members) = match step {
            Step::Emit(v) => {
                order.push(v);
                continue;
            }
            Step::Level(d, k, m) => (d, k, m),
        };
        let mut units: Vec<Unit> = Vec::new();
        let mut boxed: Vec<Vec<usize>> = Vec::new();
        for &v in &members {
            let p = path(v);
            let unit = if p.len() <= depth {
                Unit::Node(v)
            } else {
                Unit::Box(p[depth])
            };
            let ui = match units.iter().position(|&u| u == unit) {
                Some(i) => i,
                None => {
                    units.push(unit);
                    boxed.push(Vec::new());
                    units.len() - 1
                }
            };
            uidx[v] = ui;
            if let Unit::Box(_) = unit {
                boxed[ui].push(v);
            }
        }
        let ulinks: Vec<(usize, usize, i64)> = level_links
            .get(&key)
            .map(|ls| {
                ls.iter()
                    .map(|&li| {
                        let (a, b) = links[li];
                        (uidx[a], uidx[b], weights[li])
                    })
                    .collect()
            })
            .unwrap_or_default();
        let uorder = linear_order(units.len(), &ulinks);
        // Pushed in reverse: the stack pops them in order.
        for &ui in uorder.iter().rev() {
            match units[ui] {
                Unit::Node(v) => stack.push(Step::Emit(v)),
                Unit::Box(c) => {
                    stack.push(Step::Level(depth + 1, Some(c), std::mem::take(&mut boxed[ui])))
                }
            }
        }
    }
    let mut pos = vec![0usize; n];
    for (i, &v) in order.iter().enumerate() {
        pos[v] = i;
    }
    pos
}

/// A linear order of `k` items where few `links` `(from, to, weight)`
/// point backwards: strongly connected components in topological order,
/// each ordered by [`greedy_order`] and [`sift`]. Ties go to the lower
/// index.
fn linear_order(k: usize, links: &[(usize, usize, i64)]) -> Vec<usize> {
    if k <= 1 {
        return (0..k).collect();
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); k];
    for &(a, b, _) in links {
        if a != b {
            adj[a].push(b);
        }
    }
    let comps = sccs(&adj);
    let mut comp_of = vec![0usize; k];
    for (ci, c) in comps.iter().enumerate() {
        for &v in c {
            comp_of[v] = ci;
        }
    }
    let mut local = vec![0usize; k];
    let mut out: Vec<usize> = Vec::with_capacity(k);
    for (ci, c) in comps.iter().enumerate() {
        if c.len() == 1 {
            out.push(c[0]);
            continue;
        }
        for (i, &v) in c.iter().enumerate() {
            local[v] = i;
        }
        // Links inside the component, merged per ordered pair.
        let mut pairs: std::collections::BTreeMap<(usize, usize), i64> = Default::default();
        for &(a, b, w) in links {
            if a != b && comp_of[a] == ci && comp_of[b] == ci {
                *pairs.entry((local[a], local[b])).or_default() += w.max(1);
            }
        }
        let s = c.len();
        let mut outs: Vec<Vec<(usize, i64)>> = vec![Vec::new(); s];
        let mut ins: Vec<Vec<(usize, i64)>> = vec![Vec::new(); s];
        for (&(a, b), &w) in &pairs {
            outs[a].push((b, w));
            ins[b].push((a, w));
        }
        // The component's entries (linked in from an earlier one) start
        // it. A component nothing leads into starts at one root: its
        // first item, the one with the largest out-weight minus
        // in-weight, or the one with the least in-weight — whichever
        // turns the least weight (the first on a tie: the author's
        // start, where nothing else decides).
        let mut entries = vec![false; s];
        for &(a, b, _) in links {
            if comp_of[b] == ci && comp_of[a] != ci {
                entries[local[b]] = true;
            }
        }
        let roots: Vec<Vec<bool>> = if entries.contains(&true) {
            vec![entries]
        } else {
            let wsum = |l: &Vec<(usize, i64)>| l.iter().map(|x| x.1).sum::<i64>();
            let delta = |v: usize| wsum(&outs[v]) - wsum(&ins[v]);
            let most = (0..s)
                .max_by(|&a, &b| delta(a).cmp(&delta(b)).then(b.cmp(&a)))
                .unwrap_or(0);
            let least_in = (0..s).min_by_key(|&v| (wsum(&ins[v]), v)).unwrap_or(0);
            let mut cands: Vec<usize> = vec![0];
            if s <= SIFT_MAX {
                for r in [most, least_in] {
                    if !cands.contains(&r) {
                        cands.push(r);
                    }
                }
            }
            cands
                .into_iter()
                .map(|r| (0..s).map(|v| v == r).collect())
                .collect()
        };
        let mut best: Option<(i64, Vec<usize>)> = None;
        for free in &roots {
            let mut ord = greedy_order(&outs, &ins, free);
            if s <= SIFT_MAX {
                sift(&mut ord, &outs, &ins, free);
            }
            let mut at = vec![0usize; s];
            for (i, &v) in ord.iter().enumerate() {
                at[v] = i;
            }
            let turned: i64 = pairs
                .iter()
                .filter(|(&(a, b), _)| at[a] > at[b])
                .map(|(_, &w)| w)
                .sum();
            if best.as_ref().map_or(true, |b| turned < b.0) {
                best = Some((turned, ord));
            }
        }
        let ord = best.map(|b| b.1).unwrap_or_default();
        out.extend(ord.into_iter().map(|i| c[i]));
    }
    out
}

/// Strongly connected components of `adj` (Tarjan, iterative), in
/// topological order (a component's links only lead to later ones),
/// each listed in ascending order.
fn sccs(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let k = adj.len();
    let mut index = vec![usize::MAX; k];
    let mut low = vec![0usize; k];
    let mut on_stack = vec![false; k];
    let mut stack: Vec<usize> = Vec::new();
    let mut comps: Vec<Vec<usize>> = Vec::new();
    let mut next = 0usize;
    for root in 0..k {
        if index[root] != usize::MAX {
            continue;
        }
        // (node, next neighbour slot)
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (v, ref mut slot)) = call.last_mut() {
            if let Some(&w) = adj[v].get(*slot) {
                *slot += 1;
                if index[w] == usize::MAX {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            call.pop();
            if let Some(&(p, _)) = call.last() {
                low[p] = low[p].min(low[v]);
            }
            if low[v] == index[v] {
                let mut comp = Vec::new();
                while let Some(w) = stack.pop() {
                    on_stack[w] = false;
                    comp.push(w);
                    if w == v {
                        break;
                    }
                }
                comp.sort_unstable();
                comps.push(comp);
            }
        }
    }
    // Tarjan finishes a component after every component it reaches.
    comps.reverse();
    comps
}

/// Eades–Lin–Smyth, rooted: sinks go to the back, sources to the
/// front, and when neither is left the node with the largest out-weight
/// minus in-weight (over the nodes still unplaced) goes to the front
/// (lowest index on a tie) — chosen only among the nodes a placed node
/// links to, or the `free` ones (the component's entries). So every
/// node keeps a forward in-link: the flow starts where the author's
/// does, never in the middle of a cycle. Plain Eades–Lin–Smyth would
/// start bench-75's big cycle at `gw` (the largest out-weight), make
/// `user` a sink and turn both `user --> gw` and `user --> lb` where
/// `dash --> user` alone does; at the level of its boxes it would put
/// the jobs zone first, fed by nothing, because its single
/// `job1 --> svc1` and `queue --> job*` (3) against `job* --> db` (4)
/// say so. Returns local indices.
fn greedy_order(outs: &[Vec<(usize, i64)>], ins: &[Vec<(usize, i64)>], free: &[bool]) -> Vec<usize> {
    let s = outs.len();
    let mut alive = vec![true; s];
    let mut reached: Vec<bool> = free.to_vec();
    let mut outw: Vec<i64> = outs.iter().map(|l| l.iter().map(|x| x.1).sum()).collect();
    let mut inw: Vec<i64> = ins.iter().map(|l| l.iter().map(|x| x.1).sum()).collect();
    let mut outc: Vec<usize> = outs.iter().map(Vec::len).collect();
    let mut inc: Vec<usize> = ins.iter().map(Vec::len).collect();
    let mut sinks: BinaryHeap<Reverse<usize>> = (0..s)
        .filter(|&v| outc[v] == 0)
        .map(Reverse)
        .collect();
    let mut sources: BinaryHeap<Reverse<usize>> = (0..s)
        .filter(|&v| inc[v] == 0)
        .map(Reverse)
        .collect();
    let mut front: Vec<usize> = Vec::with_capacity(s);
    let mut back: Vec<usize> = Vec::new();
    let mut left = s;
    while left > 0 {
        let pick = if let Some(v) = pop_live(&mut sinks, &alive, |v| outc[v] == 0) {
            back.push(v);
            v
        } else if let Some(v) = pop_live(&mut sources, &alive, |v| inc[v] == 0) {
            front.push(v);
            v
        } else {
            let delta = |v: usize| outw[v] - inw[v];
            let best = |pool: &dyn Fn(usize) -> bool| {
                (0..s)
                    .filter(|&v| alive[v] && pool(v))
                    .max_by(|&a, &b| delta(a).cmp(&delta(b)).then(b.cmp(&a)))
            };
            // Some live node is always reached (a component is strongly
            // connected); the fallback only guards that.
            let v = best(&|v| reached[v])
                .or_else(|| best(&|_| true))
                .expect("a live node remains");
            front.push(v);
            v
        };
        alive[pick] = false;
        left -= 1;
        for &(u, w) in &outs[pick] {
            reached[u] = true;
            if alive[u] {
                inw[u] -= w;
                inc[u] -= 1;
                if inc[u] == 0 {
                    sources.push(Reverse(u));
                }
            }
        }
        for &(u, w) in &ins[pick] {
            if alive[u] {
                outw[u] -= w;
                outc[u] -= 1;
                if outc[u] == 0 {
                    sinks.push(Reverse(u));
                }
            }
        }
    }
    back.reverse();
    front.extend(back);
    front
}

/// The lowest live node of `heap` that still passes `ok`.
fn pop_live(
    heap: &mut BinaryHeap<Reverse<usize>>,
    alive: &[bool],
    ok: impl Fn(usize) -> bool,
) -> Option<usize> {
    while let Some(Reverse(v)) = heap.pop() {
        if alive[v] && ok(v) {
            return Some(v);
        }
    }
    None
}

/// Sifting (Brandenburg & Hanauer): each node in turn moves to the place
/// in the order where the fewest link weight points backwards, until a
/// pass moves nothing. A node stays put on a tie, else takes the
/// nearest best place. Moves keep [`greedy_order`]'s promise: every node
/// but the `free` ones keeps an in-link from a node before it.
fn sift(
    order: &mut Vec<usize>,
    outs: &[Vec<(usize, i64)>],
    ins: &[Vec<(usize, i64)>],
    free: &[bool],
) {
    let s = order.len();
    let mut pos = vec![0usize; s];
    for (i, &v) in order.iter().enumerate() {
        pos[v] = i;
    }
    // In-links from nodes placed before, per node.
    let support = |v: usize, pos: &[usize]| ins[v].iter().filter(|&&(u, _)| pos[u] < pos[v]).count();
    let mut nb: Vec<(usize, i64)> = Vec::new();
    for _ in 0..SIFT_PASSES {
        let mut moved = false;
        let snapshot = order.clone();
        for &v in &snapshot {
            let cur = pos[v];
            // Slot q of the order without v; v placed at slot p turns
            // its link to an item at q < p (out-links) or q >= p
            // (in-links): cost(p) = base + sum of `delta` over q < p.
            nb.clear();
            let mut base = 0i64;
            let slot = |u: usize| if pos[u] > cur { pos[u] - 1 } else { pos[u] };
            // The slots v may take: after one of its in-neighbours
            // (unless free), and before every out-neighbour only v
            // supports.
            let mut lo_ok = if free[v] { 0 } else { s };
            let mut hi_ok = s - 1;
            for &(u, w) in &outs[v] {
                nb.push((slot(u), w));
                let only_v = !free[u] && pos[v] < pos[u] && support(u, &pos) == 1;
                if only_v {
                    hi_ok = hi_ok.min(slot(u));
                }
            }
            for &(u, w) in &ins[v] {
                nb.push((slot(u), -w));
                base += w;
                lo_ok = lo_ok.min(slot(u) + 1);
            }
            if lo_ok > hi_ok {
                continue;
            }
            nb.sort_unstable();
            let now = base + nb.iter().filter(|x| x.0 < cur).map(|x| x.1).sum::<i64>();
            // Best (cost, distance from cur, slot) over the intervals of
            // constant cost, clipped to the allowed slots.
            let mut best = (now, 0usize, cur);
            let (mut c, mut lo, mut i) = (base, 0usize, 0usize);
            loop {
                let hi = nb.get(i).map_or(s - 1, |x| x.0);
                let (a, b) = (lo.max(lo_ok), hi.min(hi_ok));
                if a <= b {
                    let cand = cur.clamp(a, b);
                    let dist = cand.abs_diff(cur);
                    if (c, dist, cand) < best {
                        best = (c, dist, cand);
                    }
                }
                let Some(&(q, _)) = nb.get(i) else { break };
                while let Some(&(q2, d)) = nb.get(i) {
                    if q2 != q {
                        break;
                    }
                    c += d;
                    i += 1;
                }
                lo = q + 1;
            }
            if best.0 < now && best.2 != cur {
                let p = best.2;
                order.remove(cur);
                order.insert(p, v);
                for j in cur.min(p)..=cur.max(p) {
                    pos[order[j]] = j;
                }
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

/// Network simplex layering of `n` nodes under `links` (which must form
/// no cycle). Starts from the longest-path layering, grows a tight
/// spanning tree per connected component and pivots on negative cut
/// values (dagre's `networkSimplex`) until none is left or
/// [`SIMPLEX_WORK`] is spent. Each component's smallest layer is 0.
/// Parallel links merge (largest `minlen`, summed `weight`); ties go to
/// the lowest node index, so the result never depends on link order.
pub(crate) fn network_simplex(n: usize, links: &[Link]) -> Vec<i64> {
    // Merge parallel links, sorted by (from, to).
    let mut sorted: Vec<Link> = links.iter().copied().filter(|l| l.from != l.to).collect();
    sorted.sort_by_key(|l| (l.from, l.to));
    let mut edges: Vec<Link> = Vec::with_capacity(sorted.len());
    for l in sorted {
        match edges.last_mut() {
            Some(e) if e.from == l.from && e.to == l.to => {
                e.minlen = e.minlen.max(l.minlen);
                e.weight += l.weight;
            }
            _ => edges.push(l),
        }
    }
    let m = edges.len();
    let mut inc: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (ei, e) in edges.iter().enumerate() {
        inc[e.from].push(ei);
        inc[e.to].push(ei);
    }
    // --- Longest-path start (sources on 0). ---
    let mut rank = vec![0i64; n];
    let mut indeg = vec![0usize; n];
    for e in &edges {
        indeg[e.to] += 1;
    }
    let mut queue: std::collections::VecDeque<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    let mut seen = 0usize;
    while let Some(u) = queue.pop_front() {
        seen += 1;
        for &ei in &inc[u] {
            let e = edges[ei];
            if e.from != u {
                continue;
            }
            rank[e.to] = rank[e.to].max(rank[u] + e.minlen);
            indeg[e.to] -= 1;
            if indeg[e.to] == 0 {
                queue.push_back(e.to);
            }
        }
    }
    if seen < n {
        // A cycle: the caller broke them all, so this never happens;
        // keep the partial longest-path layering rather than loop.
        debug_assert!(false, "network_simplex given a cyclic graph");
        normalise(&mut rank, &inc, &edges);
        return rank;
    }
    if std::env::var_os("FMDBG_LP").is_some() {
        normalise(&mut rank, &inc, &edges);
        return rank;
    }
    let slack = |rank: &[i64], e: &Link| rank[e.to] - rank[e.from] - e.minlen;
    // --- Connected components (undirected). ---
    let mut comp = vec![usize::MAX; n];
    let mut comps: Vec<Vec<usize>> = Vec::new();
    for s in 0..n {
        if comp[s] != usize::MAX {
            continue;
        }
        let ci = comps.len();
        let mut members = vec![s];
        comp[s] = ci;
        let mut k = 0;
        while k < members.len() {
            let u = members[k];
            k += 1;
            for &ei in &inc[u] {
                let e = edges[ei];
                let w = if e.from == u { e.to } else { e.from };
                if comp[w] == usize::MAX {
                    comp[w] = ci;
                    members.push(w);
                }
            }
        }
        members.sort_unstable();
        comps.push(members);
    }
    let mut comp_edges: Vec<Vec<usize>> = vec![Vec::new(); comps.len()];
    for (ei, e) in edges.iter().enumerate() {
        comp_edges[comp[e.from]].push(ei);
    }
    // --- Feasible tight tree per component. ---
    let mut in_tree = vec![false; n];
    let mut tree_edge = vec![false; m];
    let mut tree_adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (ci, members) in comps.iter().enumerate() {
        if members.len() < 2 {
            continue;
        }
        let root = members[0];
        in_tree[root] = true;
        let mut tree_nodes: Vec<usize> = vec![root];
        let mut frontier: Vec<usize> = vec![root];
        loop {
            // Grow through tight links from every newly added node.
            while let Some(u) = frontier.pop() {
                for &ei in &inc[u] {
                    let e = edges[ei];
                    let w = if e.from == u { e.to } else { e.from };
                    if !in_tree[w] && slack(&rank, &e) == 0 {
                        in_tree[w] = true;
                        tree_edge[ei] = true;
                        tree_adj[u].push(ei);
                        tree_adj[w].push(ei);
                        tree_nodes.push(w);
                        frontier.push(w);
                    }
                }
            }
            if tree_nodes.len() == members.len() {
                break;
            }
            // The least-slack link leaving the tree; shift the tree
            // onto it so it becomes tight.
            let pick = comp_edges[ci]
                .iter()
                .copied()
                .filter(|&ei| in_tree[edges[ei].from] != in_tree[edges[ei].to])
                .min_by_key(|&ei| (slack(&rank, &edges[ei]), ei));
            let Some(ei) = pick else { break };
            let e = edges[ei];
            let d = slack(&rank, &e);
            let delta = if in_tree[e.from] { d } else { -d };
            for &v in &tree_nodes {
                rank[v] += delta;
            }
            frontier.push(if in_tree[e.from] { e.from } else { e.to });
        }
    }
    // --- Pivots. ---
    let lp_only = std::env::var_os("FMDBG_LP").is_some();
    let mut low = vec![0usize; n];
    let mut lim = vec![0usize; n];
    let mut parent_edge = vec![usize::MAX; n];
    let mut cut = vec![0i64; m];
    let mut work = 0usize;
    for (ci, members) in comps.iter().enumerate() {
        if members.len() < 2 || lp_only {
            continue;
        }
        let root = members[0];
        let size = members.len() + comp_edges[ci].len();
        loop {
            let post = low_lim(root, &tree_adj, &edges, &mut low, &mut lim, &mut parent_edge);
            cut_values(&post, &edges, &inc, &tree_edge, &parent_edge, &mut cut);
            work += size;
            if work > SIMPLEX_WORK {
                break;
            }
            let Some(leave) = comp_edges[ci]
                .iter()
                .copied()
                .find(|&ei| tree_edge[ei] && cut[ei] < 0)
            else {
                break;
            };
            let (v, w) = (edges[leave].from, edges[leave].to);
            let (tail, flip) = if lim[v] > lim[w] { (w, true) } else { (v, false) };
            let desc = |x: usize| low[tail] <= lim[x] && lim[x] <= lim[tail];
            let Some(enter) = comp_edges[ci]
                .iter()
                .copied()
                .filter(|&ei| flip == desc(edges[ei].from) && flip != desc(edges[ei].to))
                .min_by_key(|&ei| (slack(&rank, &edges[ei]), ei))
            else {
                break;
            };
            tree_edge[leave] = false;
            tree_edge[enter] = true;
            for x in [edges[leave].from, edges[leave].to] {
                tree_adj[x].retain(|&t| t != leave);
            }
            tree_adj[edges[enter].from].push(enter);
            tree_adj[edges[enter].to].push(enter);
            // Re-tighten: every tree link at its minimum length, from
            // the root down.
            let post = low_lim(root, &tree_adj, &edges, &mut low, &mut lim, &mut parent_edge);
            for &v in post.iter().rev() {
                let pe = parent_edge[v];
                if pe == usize::MAX {
                    continue;
                }
                let e = edges[pe];
                rank[v] = if e.to == v {
                    rank[e.from] + e.minlen
                } else {
                    rank[e.to] - e.minlen
                };
            }
        }
    }
    normalise(&mut rank, &inc, &edges);
    rank
}

/// Post-order numbering of the tree rooted at `root`: `lim` (the
/// post-order number), `low` (the smallest `lim` in the subtree) and the
/// tree link to each node's parent. Returns the nodes in post-order.
fn low_lim(
    root: usize,
    tree_adj: &[Vec<usize>],
    edges: &[Link],
    low: &mut [usize],
    lim: &mut [usize],
    parent_edge: &mut [usize],
) -> Vec<usize> {
    let mut post: Vec<usize> = Vec::new();
    // (node, next slot, low so far)
    let mut stack: Vec<(usize, usize, usize)> = vec![(root, 0, usize::MAX)];
    parent_edge[root] = usize::MAX;
    let mut next = 1usize;
    while let Some(&mut (v, ref mut slot, _)) = stack.last_mut() {
        if let Some(&ei) = tree_adj[v].get(*slot) {
            *slot += 1;
            if ei == parent_edge[v] {
                continue;
            }
            let e = edges[ei];
            let w = if e.from == v { e.to } else { e.from };
            parent_edge[w] = ei;
            stack.push((w, 0, usize::MAX));
            continue;
        }
        let (_, _, sub_low) = stack.pop().expect("non-empty");
        lim[v] = next;
        low[v] = sub_low.min(next);
        next += 1;
        post.push(v);
        if let Some(top) = stack.last_mut() {
            top.2 = top.2.min(low[v]);
        }
    }
    post
}

/// Cut value of every tree link, children before parents (`post`).
fn cut_values(
    post: &[usize],
    edges: &[Link],
    inc: &[Vec<usize>],
    tree_edge: &[bool],
    parent_edge: &[usize],
    cut: &mut [i64],
) {
    for &child in post {
        let pe = parent_edge[child];
        if pe == usize::MAX {
            continue;
        }
        let child_is_tail = edges[pe].from == child;
        let mut c = edges[pe].weight;
        for &ei in &inc[child] {
            if ei == pe {
                continue;
            }
            let e = edges[ei];
            let is_out = e.from == child;
            let points_to_head = is_out == child_is_tail;
            c += if points_to_head { e.weight } else { -e.weight };
            if tree_edge[ei] {
                c += if points_to_head { -cut[ei] } else { cut[ei] };
            }
        }
        cut[pe] = c;
    }
}

/// Shift each connected component so its smallest layer is 0.
fn normalise(rank: &mut [i64], inc: &[Vec<usize>], edges: &[Link]) {
    let n = rank.len();
    let mut seen = vec![false; n];
    for s in 0..n {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        let mut members = vec![s];
        let mut k = 0;
        while k < members.len() {
            let u = members[k];
            k += 1;
            for &ei in &inc[u] {
                let e = edges[ei];
                let w = if e.from == u { e.to } else { e.from };
                if !seen[w] {
                    seen[w] = true;
                    members.push(w);
                }
            }
        }
        let lo = members.iter().map(|&v| rank[v]).min().unwrap_or(0);
        for &v in &members {
            rank[v] -= lo;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(from: usize, to: usize) -> Link {
        Link {
            from,
            to,
            minlen: 1,
            weight: 1,
        }
    }

    /// Total weighted length of `links` under `rank`.
    fn cost(rank: &[i64], links: &[Link]) -> i64 {
        links
            .iter()
            .map(|l| l.weight * (rank[l.to] - rank[l.from]))
            .sum()
    }

    fn feasible(rank: &[i64], links: &[Link]) -> bool {
        links
            .iter()
            .all(|l| l.from == l.to || rank[l.to] - rank[l.from] >= l.minlen)
    }

    #[test]
    fn a_source_sits_right_above_its_target() {
        // a -> b -> c -> d, and x -> d: longest-path puts x on 0.
        let links = [link(0, 1), link(1, 2), link(2, 3), link(4, 3)];
        let r = network_simplex(5, &links);
        assert_eq!(&r[..4], &[0, 1, 2, 3]);
        assert_eq!(r[4], 2, "x one layer above d");
    }

    #[test]
    fn components_each_start_on_layer_zero() {
        let links = [link(0, 1), link(2, 3), link(3, 4)];
        let r = network_simplex(6, &links);
        assert_eq!(r, vec![0, 1, 0, 1, 2, 0]);
    }

    #[test]
    fn simplex_finds_the_least_total_length() {
        // Small random DAGs against exhaustive search over layers 0..5.
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut rnd = |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        for _ in 0..200 {
            let n = 2 + rnd(4) as usize;
            let mut links = Vec::new();
            for _ in 0..(1 + rnd(7)) {
                let a = rnd(n as u64) as usize;
                let b = rnd(n as u64) as usize;
                if a < b {
                    links.push(Link {
                        from: a,
                        to: b,
                        minlen: rnd(2) as i64,
                        weight: 1 + rnd(3) as i64,
                    });
                }
            }
            let r = network_simplex(n, &links);
            assert!(feasible(&r, &links), "{links:?} -> {r:?}");
            let mut best = i64::MAX;
            let mut cand = vec![0i64; n];
            let total = 6usize.pow(n as u32);
            for code in 0..total {
                let mut c = code;
                for v in cand.iter_mut() {
                    *v = (c % 6) as i64;
                    c /= 6;
                }
                if feasible(&cand, &links) {
                    best = best.min(cost(&cand, &links));
                }
            }
            assert_eq!(cost(&r, &links), best, "{links:?} -> {r:?}");
        }
    }

    #[test]
    fn greedy_fas_turns_the_minority_link() {
        // 0 -> 1 three times over, 1 -> 0 once: the single link turns,
        // whichever is written first.
        let links = [(1, 0), (0, 1), (0, 1), (0, 1)];
        let back = feedback_links(2, &links, &[1; 4], None);
        assert_eq!(back, vec![true, false, false, false]);
    }

    #[test]
    fn a_cycle_starts_where_the_fewest_links_turn() {
        // 0 -> 1 once, 1 -> 0 three times: starting at the first node
        // would turn three links; starting at 1 turns one.
        let links = [(0, 1), (1, 0), (1, 0), (1, 0)];
        let back = feedback_links(2, &links, &[1; 4], None);
        assert_eq!(back, vec![true, false, false, false]);
    }

    #[test]
    fn a_link_on_no_cycle_never_turns() {
        // Two 2-cycles joined by one link that lies on neither.
        let links = [(0, 1), (1, 0), (2, 3), (3, 2), (3, 0)];
        let back = feedback_links(4, &links, &[1; 5], None);
        assert!(!back[4]);
        assert_eq!(back.iter().filter(|&&b| b).count(), 2);
    }

    #[test]
    fn the_flow_starts_at_the_first_node_and_turns_one_return_link() {
        // user(0) -> gw(1), lb(2); gw -> lb, a(3), b(4), c(5); lb -> a,
        // b; a, b, c -> x(6); x -> user. Plain Eades–Lin–Smyth starts at
        // gw (largest out-weight) and ends with user as a sink: both
        // its out-links turn. Rooted at user, only x -> user turns.
        let links = [
            (0, 1),
            (0, 2),
            (1, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (2, 3),
            (2, 4),
            (3, 6),
            (4, 6),
            (5, 6),
            (6, 0),
        ];
        let back = feedback_links(7, &links, &[1; 12], None);
        let mut want = vec![false; 12];
        want[11] = true;
        assert_eq!(back, want);
    }

    #[test]
    fn sifting_moves_a_node_past_the_links_it_turns() {
        // Order 0, 1, 2, 3 with 0 -> 1, 1 -> 3, 2 -> 3 and 3 -> 1 (weight
        // 2): moving 1 after 3 turns weight 1 instead of 2.
        let outs = vec![vec![(1, 1)], vec![(3, 1)], vec![(3, 1)], vec![(1, 2)]];
        let ins = vec![vec![], vec![(0, 1), (3, 2)], vec![], vec![(1, 1), (2, 1)]];
        let free = [true, false, true, false];
        let mut order = vec![0, 1, 2, 3];
        sift(&mut order, &outs, &ins, &free);
        assert_eq!(order, vec![0, 2, 3, 1]);
        // Without 2 -> 3, 1 alone feeds 3: moving 1 would leave 3 with
        // no forward in-link, so 1 stays.
        let outs = vec![vec![(1, 1)], vec![(3, 1)], vec![], vec![(1, 2)]];
        let ins = vec![vec![], vec![(0, 1), (3, 2)], vec![], vec![(1, 1)]];
        let mut order = vec![0, 1, 2, 3];
        sift(&mut order, &outs, &ins, &free);
        assert_eq!(order, vec![0, 1, 2, 3]);
    }

    #[test]
    fn boxes_stay_contiguous_and_turn_their_minority_links() {
        // Box 0 = {0, 1}, box 1 = {2, 3}: 0 -> 2, 1 -> 3, 2 -> 1. The
        // node order 0, 2, 1, 3 turns nothing but interleaves the boxes;
        // box order turns the one link from box 1 back into box 0.
        let links = [(0, 2), (1, 3), (2, 1)];
        let paths = vec![vec![0], vec![0], vec![1], vec![1]];
        let back = feedback_links(4, &links, &[1; 3], Some(&paths));
        assert_eq!(back, vec![false, false, true]);
        assert_eq!(
            feedback_links(4, &links, &[1; 3], None),
            vec![false; 3],
            "without boxes no link needs to turn"
        );
    }
}
