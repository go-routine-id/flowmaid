/// How much broader (px) than its tightest packing a level of a block
/// layout may stand and keep Brandes-Köpf's columns (B12 X2.3): float
/// noise and a wall's rounding, not a column that lies apart.
const BLOCK_SLACK: f64 = 4.0;

/// Room (px) a run of a squeezed level of a block layout keeps beside
/// each neighbour on top of [`pair_sep`] (B12 X2): one comfortable
/// clearance ([`crate::clearance::CLEAR_GAP`]). Brandes-Köpf rarely
/// packs a channel against its neighbours; the squeeze packs every
/// column of a level as tight as its order allows, and runs packed at
/// the bare [`EDGE_GAP`] left the edge stages no room for the ports,
/// steps and tracks they lay beside them: the router rerouted the
/// channels across their hub's fan (dense-td drew 78 crossings, 15 of
/// them inside one fan, where it draws 44 with this room).
const RUN_ROOM: f64 = crate::clearance::CLEAR_GAP;

/// One level of a block layout as [`block_compact`] squeezes it: the
/// level's own items (nodes, runs), its child blocks as rigid units,
/// and, below the root, the block's two walls.
struct LevelUnits {
    /// Per unit: its members (augmented nodes).
    members: Vec<Vec<usize>>,
    /// The level's entries per layer, in layer order.
    rows: Vec<Vec<usize>>,
    /// The two walls' units (below the root): pinned at the level's
    /// two ends.
    walls: Option<(usize, usize)>,
}

/// What a block layout's squeeze ([`block_compact`]) must keep.
struct SqueezeHold {
    /// Per subgraph: the least breadth of its frame while it holds
    /// columns past its title chip (X2.2's keep rule; 0 for none).
    least: Vec<f64>,
    /// The runs that cross a block's title side past its chip, each with
    /// the column it must not come back over (X2.5; the chip's far end,
    /// and the columns of the block's displaced entries past it).
    past: Vec<(usize, f64)>,
}

/// B12 X2 and X3: the breadths of a block layout, fixed one level at a
/// time, deepest first, up to the root.
///
/// A level's units are its own nodes and runs (each its own unit), its
/// child blocks (already fixed: rigid units that move as a whole) and,
/// below the root, the block's two walls. When the breadth
/// Brandes-Köpf gave the level lies within [`BLOCK_SLACK`] of the least
/// breadth its order allows (the longest path over the left-of relation
/// with [`pair_sep`] distances, title room included: the walls carry
/// it), its columns stay. Otherwise the level is squeezed to that
/// breadth, seeded by Brandes-Köpf: every *spine* (two units on
/// adjacent layers that are each other's only link inside the level in
/// that direction; at a parent level a border link between two real
/// nodes, X3) becomes one rigid column when that keeps the least
/// breadth and the order and lays no two of the merged members over
/// each other on a layer (two sibling blocks joined by a border spine
/// may share layers); each unit's Brandes-Köpf offset is clamped
/// into its feasible interval for that breadth, left to right so the
/// order holds; then [`COMPACT_SWEEPS`] Gauss-Seidel sweeps move each
/// unit to the weighted median of its links inside the level (1 / 2 / 8
/// as [`compact_breadth`]), between its neighbours. A child block's
/// links are its members' links to the rest of the level, so a block as
/// a whole is pulled toward its border links. A fixed block keeps its
/// centre where its parent placed it until the parent's own pass moves
/// it. A level whose order the walls cannot hold (a unit split on a
/// layer) keeps its positions.
///
/// Chain dummies of different cluster paths are no spine (X1: a chain
/// bends where it crosses a border, in a gap).
#[allow(clippy::too_many_arguments)] // internal: one bundle of layered-graph state
fn block_compact(
    n: usize,
    layers: &[Vec<usize>],
    preds: &[Vec<usize>],
    succs: &[Vec<usize>],
    alayer: &[usize],
    absize: &[f64],
    apath: &[Vec<usize>],
    border: &[u8],
    header_b: Option<&[f64]>,
    hold: &SqueezeHold,
    bpos: &mut [f64],
) {
    let title_least = &hold.least[..];
    let na = bpos.len();
    if na == 0 {
        return;
    }
    // Levels: every block (by its walls' path), deepest first, then the
    // root.
    let mut spans: BTreeMap<Vec<usize>, (usize, usize)> = BTreeMap::new();
    for v in 0..na {
        if border[v] != 0 {
            let e = spans
                .entry(apath[v].clone())
                .or_insert((alayer[v], alayer[v]));
            e.0 = e.0.min(alayer[v]);
            e.1 = e.1.max(alayer[v]);
        }
    }
    let mut levels: Vec<(Vec<usize>, usize, usize)> =
        spans.into_iter().map(|(p, (lo, hi))| (p, lo, hi)).collect();
    levels.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(&b.0)));
    levels.push((Vec::new(), 0, layers.len().saturating_sub(1)));
    let mut unit_of = vec![usize::MAX; na];
    let mut off = vec![0.0f64; na];
    let sep = |u: usize, v: usize| {
        let run = |x: usize| x >= n && border[x] == 0;
        pair_sep(u, v, n, absize, Some(apath), Some(border), header_b)
            + if run(u) || run(v) { RUN_ROOM } else { 0.0 }
    };
    for (path, lo, hi) in levels {
        let k = path.len();
        let Some(lv) = level_units(&path, lo, hi, layers, apath, border, &mut unit_of) else {
            continue;
        };
        for (u, ms) in lv.members.iter().enumerate() {
            let anchor = ms[0];
            for &m in ms {
                off[m] = bpos[m] - bpos[anchor];
                unit_of[m] = u;
            }
        }
        // X2.5's columns: a run past a block's title chip keeps its
        // distance from the block's left wall (the chip's side), measured
        // on its layer, at least where it reaches past the chip.
        let past: Vec<(usize, f64)> = match lv.walls {
            Some((wl, _)) => hold
                .past
                .iter()
                .filter(|&&(d, _)| unit_of[d] != usize::MAX && apath[d].len() == k)
                .filter_map(|&(d, need)| {
                    let wm = lv.members[wl].iter().copied().find(|&m| alayer[m] == alayer[d])?;
                    Some((d, need - bpos[wm] + off[wm]))
                })
                .collect(),
            None => Vec::new(),
        };
        // Each member's unit before any spine is merged.
        let orig = unit_of.clone();
        let in_level = |w: usize| apath[w].starts_with(&path);
        // Links inside the level between two units: (member, other end,
        // weight).
        // A chain's segment across a border is no spine (X1): it bends
        // there, in the gap, so it pulls a block as a border link does,
        // not with the weight that keeps a run straight. (With 8, the
        // bounce of the hub fixture's email block, a back edge leaving
        // the block, outweighed the block's two forward links and took
        // it out from under the delivery block: the hub's edge into it
        // then ran down the far side and back, out of target order.)
        let weight = |u: usize, v: usize| match (u < n, v < n) {
            (true, true) => 1.0,
            (false, false) if apath[u] != apath[v] => COMPACT_MIXED_WEIGHT,
            (false, false) => COMPACT_DUMMY_WEIGHT,
            _ => COMPACT_MIXED_WEIGHT,
        };
        let links = |m: usize, unit_of: &[usize]| -> Vec<(usize, f64)> {
            if border[m] != 0 {
                return Vec::new();
            }
            let mut out: Vec<(usize, f64)> = Vec::new();
            for &w in preds[m].iter().chain(&succs[m]) {
                if border[w] == 0
                    && alayer[w].abs_diff(alayer[m]) == 1
                    && in_level(w)
                    && unit_of[w] != unit_of[m]
                {
                    out.push((w, weight(m, w)));
                }
            }
            out
        };
        let packed0 = pack_units(&lv, &unit_of, &orig, &off, absize, &sep, &past);
        let Some((_, w_min)) = packed0.as_ref().map(|p| (p.0.clone(), p.2)) else {
            clear_units(&lv, &mut unit_of);
            continue;
        };
        // The breadth Brandes-Köpf gave the level.
        let (mut bk_lo, mut bk_hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for row in &lv.rows {
            for &v in row {
                bk_lo = bk_lo.min(bpos[v] - absize[v] / 2.0);
                bk_hi = bk_hi.max(bpos[v] + absize[v] / 2.0);
            }
        }
        if bk_hi - bk_lo <= w_min + BLOCK_SLACK {
            clear_units(&lv, &mut unit_of);
            continue;
        }
        // Spines: (member, other end) on adjacent layers, each the
        // other's only link inside the level in that direction.
        let only = |m: usize, down: bool, unit_of: &[usize]| -> Option<usize> {
            let mut it = links(m, unit_of)
                .into_iter()
                .map(|(w, _)| w)
                .filter(|&w| (alayer[w] > alayer[m]) == down);
            let w = it.next()?;
            it.all(|x| x == w).then_some(w)
        };
        let mut spines: Vec<(u8, usize, usize)> = Vec::new();
        for (u, ms) in lv.members.iter().enumerate() {
            if lv.walls.is_some_and(|(a, b)| u == a || u == b) {
                continue;
            }
            for &m in ms {
                let direct = |x: usize| apath[x].len() == k;
                let Some(w) = only(m, true, &unit_of) else {
                    continue;
                };
                if only(w, false, &unit_of) != Some(m) {
                    continue;
                }
                let class = match (m < n, w < n) {
                    // A run's own column (X1: never across a border).
                    (false, false) if direct(m) && direct(w) && apath[m] == apath[w] => 0,
                    // A run meeting its node, inside the level.
                    (true, false) | (false, true) if direct(m) && direct(w) => 1,
                    // Two nodes: a spine, or a border spine (X3).
                    (true, true) => 2,
                    _ => continue,
                };
                spines.push((class, m, w));
            }
        }
        spines.sort_by_key(|&(c, m, w)| (c, alayer[m], m, w));
        // All at once first; one at a time, each kept only if it keeps
        // the order and the least breadth, when that fails.
        let save = (lv_clone(&lv), unit_of.clone(), off.clone());
        let mut lv = lv;
        for &(_, m, w) in &spines {
            merge_units(&mut lv, &mut unit_of, &mut off, m, w);
        }
        let mut packed = pack_units(&lv, &unit_of, &orig, &off, absize, &sep, &past)
            .filter(|p| p.2 <= w_min + TITLE_FIT_EPS);
        if packed.is_none() {
            (lv, unit_of, off) = (save.0, save.1, save.2);
            packed = packed0;
            for &(_, m, w) in &spines {
                if unit_of[m] == unit_of[w] {
                    continue;
                }
                let keep = (lv_clone(&lv), unit_of.clone(), off.clone());
                merge_units(&mut lv, &mut unit_of, &mut off, m, w);
                match pack_units(&lv, &unit_of, &orig, &off, absize, &sep, &past) {
                    Some(p) if p.2 <= w_min + TITLE_FIT_EPS => packed = Some(p),
                    _ => (lv, unit_of, off) = (keep.0, keep.1, keep.2),
                }
            }
        }
        let Some((left, mut right, mut w, topo, edges)) = packed else {
            clear_units(&lv, &mut unit_of);
            continue;
        };
        if w_min > w {
            for r in right.iter_mut() {
                *r += w_min - w;
            }
            w = w_min;
        }
        let nu = lv.members.len();
        // Seed: Brandes-Köpf's offset from the level's left, clamped into
        // the unit's feasible interval, then left to right so the order
        // holds.
        let anchor_x = |u: usize| lv.members[u].first().map_or(0.0, |&m| bpos[m] - off[m]);
        let base = match lv.walls {
            Some((wl, _)) => anchor_x(wl),
            None => bk_lo,
        };
        let mut x: Vec<f64> = (0..nu)
            .map(|u| (anchor_x(u) - base).max(left[u]).min(right[u]))
            .collect();
        let mut into: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nu];
        let mut outof: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nu];
        for &(a, b, s) in &edges {
            outof[a].push((b, s));
            into[b].push((a, s));
        }
        for &u in &topo {
            for &(p, s) in &into[u] {
                x[u] = x[u].max(x[p] + s);
            }
        }
        if let Some((wl, wr)) = lv.walls {
            x[wl] = 0.0;
            x[wr] = w;
        }
        let ulinks: Vec<Vec<(usize, usize, f64)>> = lv
            .members
            .iter()
            .map(|ms| {
                ms.iter()
                    .flat_map(|&m| {
                        links(m, &unit_of)
                            .into_iter()
                            .map(move |(o, wt)| (m, o, wt))
                    })
                    .collect()
            })
            .collect();
        let head = |u: usize| {
            lv.members[u]
                .iter()
                .map(|&m| absize[m] / 2.0 - off[m])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        let tail = |u: usize| {
            lv.members[u]
                .iter()
                .map(|&m| off[m] + absize[m] / 2.0)
                .fold(f64::NEG_INFINITY, f64::max)
        };
        let (heads, tails): (Vec<f64>, Vec<f64>) = (0..nu).map(|u| (head(u), tail(u))).unzip();
        // A block keeps its centre where Brandes-Köpf put it (its
        // parent's pass moves it); the root keeps its left.
        let shift = match lv.walls {
            Some((wl, wr)) => (anchor_x(wl) + anchor_x(wr)) / 2.0 - w / 2.0,
            None => bk_lo,
        };
        for round in 0..COMPACT_SWEEPS {
            let order: Vec<usize> = if round % 2 == 0 {
                topo.clone()
            } else {
                topo.iter().rev().copied().collect()
            };
            for u in order {
                if lv.walls.is_some_and(|(a, b)| u == a || u == b) {
                    continue;
                }
                let mut lo = heads[u];
                for &(d, at) in &past {
                    if unit_of[d] == u {
                        lo = lo.max(at - off[d]);
                    }
                }
                for &(p, s) in &into[u] {
                    lo = lo.max(x[p] + s);
                }
                let mut hi = w - tails[u];
                for &(s2, s) in &outof[u] {
                    hi = hi.min(x[s2] - s);
                }
                if lo > hi {
                    continue;
                }
                let mut pull: Vec<(f64, f64)> = ulinks[u]
                    .iter()
                    .map(|&(m, o, wt)| (x[unit_of[o]] + off[o] - off[m], wt))
                    .collect();
                if pull.is_empty() {
                    continue;
                }
                let want = weighted_median(&mut pull);
                if want.is_finite() {
                    x[u] = want.max(lo).min(hi);
                }
            }
        }
        // X2.2: a block whose title side takes displaced entries and
        // whose members Brandes-Köpf spread broader than its title keeps
        // those breadths when the squeeze would pack its members under
        // the title: the frame would grow to its title instead, and a
        // title-bound frame holds no column past its chip for them (the
        // public `blocks/lr` drawn BT: `pager` under the chip, its two
        // entries went round through the lateral walls and crossed).
        let least = path.last().and_then(|&c| title_least.get(c)).copied().unwrap_or(0.0);
        if least > 0.0 {
            let span = |at: &dyn Fn(usize) -> f64| {
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for ms in &lv.members {
                    for &m in ms.iter().filter(|&&m| m < n) {
                        lo = lo.min(at(m) - absize[m] / 2.0);
                        hi = hi.max(at(m) + absize[m] / 2.0);
                    }
                }
                hi - lo + 2.0 * FRAME_PAD
            };
            let before = span(&|m| bpos[m]);
            let after = span(&|m| x[unit_of[m]] + off[m] + shift);
            if before >= least - TITLE_FIT_EPS && after < least - TITLE_FIT_EPS {
                clear_units(&lv, &mut unit_of);
                continue;
            }
        }
        for (u, ms) in lv.members.iter().enumerate() {
            for &m in ms {
                bpos[m] = x[u] + off[m] + shift;
            }
        }
        clear_units(&lv, &mut unit_of);
    }
}

/// The units of one level of a block layout ([`block_compact`]): path
/// `path`, layers `lo..=hi`. `unit_of` is left marked with each
/// member's unit. `None` for an empty level.
#[allow(clippy::too_many_arguments)] // internal: one bundle of layered-graph state
fn level_units(
    path: &[usize],
    lo: usize,
    hi: usize,
    layers: &[Vec<usize>],
    apath: &[Vec<usize>],
    border: &[u8],
    unit_of: &mut [usize],
) -> Option<LevelUnits> {
    let k = path.len();
    let mut members: Vec<Vec<usize>> = Vec::new();
    let mut child: BTreeMap<usize, usize> = BTreeMap::new();
    let (mut wl, mut wr) = (None, None);
    let mut rows: Vec<Vec<usize>> = Vec::with_capacity(hi + 1 - lo);
    for lay in &layers[lo..=hi.min(layers.len().saturating_sub(1))] {
        let mut row = Vec::new();
        for &v in lay {
            if !apath[v].starts_with(path) {
                continue;
            }
            let u = if apath[v].len() > k {
                *child.entry(apath[v][k]).or_insert_with(|| {
                    members.push(Vec::new());
                    members.len() - 1
                })
            } else if k > 0 && border[v] == 1 {
                *wl.get_or_insert_with(|| {
                    members.push(Vec::new());
                    members.len() - 1
                })
            } else if k > 0 && border[v] == 2 {
                *wr.get_or_insert_with(|| {
                    members.push(Vec::new());
                    members.len() - 1
                })
            } else {
                members.push(Vec::new());
                members.len() - 1
            };
            members[u].push(v);
            unit_of[v] = u;
            row.push(v);
        }
        rows.push(row);
    }
    if members.is_empty() {
        return None;
    }
    let walls = match (k > 0, wl, wr) {
        (true, Some(a), Some(b)) => Some((a, b)),
        (true, _, _) => {
            clear_units(
                &LevelUnits {
                    members,
                    rows,
                    walls: None,
                },
                unit_of,
            );
            return None;
        }
        _ => None,
    };
    Some(LevelUnits {
        members,
        rows,
        walls,
    })
}

fn lv_clone(lv: &LevelUnits) -> LevelUnits {
    LevelUnits {
        members: lv.members.clone(),
        rows: lv.rows.clone(),
        walls: lv.walls,
    }
}

fn clear_units(lv: &LevelUnits, unit_of: &mut [usize]) {
    for ms in &lv.members {
        for &m in ms {
            unit_of[m] = usize::MAX;
        }
    }
}

/// Merge the unit of `w` into the unit of `m` so that `w` stands in
/// `m`'s column (a spine of [`block_compact`]).
fn merge_units(lv: &mut LevelUnits, unit_of: &mut [usize], off: &mut [f64], m: usize, w: usize) {
    let (a, b) = (unit_of[m], unit_of[w]);
    if a == b
        || lv
            .walls
            .is_some_and(|(l, r)| [l, r].contains(&a) || [l, r].contains(&b))
    {
        return;
    }
    let delta = off[m] - off[w];
    let moved = std::mem::take(&mut lv.members[b]);
    for &x in &moved {
        off[x] += delta;
        unit_of[x] = a;
    }
    lv.members[a].extend(moved);
}

/// A packing of a level's units ([`block_compact`]): per unit its least
/// and greatest position for the least breadth, that breadth, the units
/// in topological order of the left-of relation, and the relation's
/// edges `(left, right, least distance)`. `None` when the relation has
/// a cycle (a unit split on a layer).
type Packing = (
    Vec<f64>,
    Vec<f64>,
    f64,
    Vec<usize>,
    Vec<(usize, usize, f64)>,
);

fn pack_units(
    lv: &LevelUnits,
    unit_of: &[usize],
    orig: &[usize],
    off: &[f64],
    absize: &[f64],
    sep: &dyn Fn(usize, usize) -> f64,
    past: &[(usize, f64)],
) -> Option<Packing> {
    let nu = lv.members.len();
    let live: Vec<bool> = lv.members.iter().map(|m| !m.is_empty()).collect();
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    for row in &lv.rows {
        for p in row.windows(2) {
            let (a, b) = (unit_of[p[0]], unit_of[p[1]]);
            if a != b {
                edges.push((a, b, sep(p[0], p[1]) + off[p[0]] - off[p[1]]));
            } else if orig[p[0]] != orig[p[1]]
                && off[p[1]] - off[p[0]] < sep(p[0], p[1]) - TITLE_FIT_EPS
            {
                // Two neighbours that a merged spine made one rigid unit
                // and that would then overlap: the merge is infeasible.
                return None;
            }
        }
    }
    let mut indeg = vec![0usize; nu];
    let mut outof: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nu];
    for &(a, b, s) in &edges {
        outof[a].push((b, s));
        indeg[b] += 1;
    }
    let ext = |u: usize, f: &dyn Fn(usize) -> f64| {
        lv.members[u]
            .iter()
            .map(|&m| f(m))
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let mut left: Vec<f64> = (0..nu)
        .map(|u| {
            if live[u] {
                ext(u, &|m| absize[m] / 2.0 - off[m])
            } else {
                0.0
            }
        })
        .collect();
    if let Some((wl, _)) = lv.walls {
        left[wl] = 0.0;
    }
    for &(d, at) in past {
        let u = unit_of[d];
        if u < nu && live[u] {
            left[u] = left[u].max(at - off[d]);
        }
    }
    let mut queue: Vec<usize> = (0..nu).filter(|&u| live[u] && indeg[u] == 0).collect();
    queue.reverse();
    let mut topo: Vec<usize> = Vec::with_capacity(nu);
    while let Some(u) = queue.pop() {
        topo.push(u);
        for &(b, s) in &outof[u] {
            left[b] = left[b].max(left[u] + s);
            indeg[b] -= 1;
            if indeg[b] == 0 {
                queue.push(b);
            }
        }
    }
    if topo.len() != live.iter().filter(|&&l| l).count() {
        return None;
    }
    let tails: Vec<f64> = (0..nu)
        .map(|u| {
            if live[u] {
                ext(u, &|m| off[m] + absize[m] / 2.0)
            } else {
                0.0
            }
        })
        .collect();
    let w = match lv.walls {
        Some((_, wr)) => left[wr],
        None => topo
            .iter()
            .map(|&u| left[u] + tails[u])
            .fold(0.0f64, f64::max),
    };
    let mut right: Vec<f64> = (0..nu).map(|u| w - tails[u]).collect();
    if let Some((_, wr)) = lv.walls {
        right[wr] = w;
    }
    for &u in topo.iter().rev() {
        for &(b, s) in &outof[u] {
            right[u] = right[u].min(right[b] - s);
        }
    }
    Some((left, right, w, topo, edges))
}

