p='src/layout.rs'; s=open(p).read()
def rep(a,b,cnt=1):
    global s
    assert s.count(a)==cnt, (s.count(a), a[:70])
    s=s.replace(a,b)

code = r'''
/// The most starts the ordering of a block layout takes ([`BlockOrder`]):
/// the canonical order and six shuffles of it. A shuffled start finds
/// orders no sweep reaches from the canonical one (the public fixture's
/// crossings fall with the starts), and seven is what a layered layout
/// spends by default elsewhere (ELK's thoroughness).
const BLOCK_ORDER_STARTS: usize = 7;

/// The work the starts of one ordering may cost, in elements (dummies
/// and walls included) plus links, summed over the starts: a drawing of
/// `e` elements takes `BLOCK_ORDER_WORK / e` starts, at least one and at
/// most [`BLOCK_ORDER_STARTS`], so the static scene of a large diagram
/// stays inside its time budget.
const BLOCK_ORDER_WORK: usize = 9_000;

/// The final order of a block layout's placement, by element: per
/// layer, each real node `(node, usize::MAX)` and each channel dummy
/// `(edge, layer)`. The later placements of one layout call start from
/// the first one's ([`BlockOrder::warm`]).
pub(crate) type OrderSeed = Vec<Vec<(usize, usize)>>;

/// A unit of a level on one layer, as the level's ordering sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OrderUnit {
    /// A direct node of the level (a real node; by augmented index).
    Node(usize),
    /// A child block: its members on the layer, kept together.
    Block(usize),
    /// A dummy of a run: the dummies of one channel that lie in the
    /// level itself on consecutive layers.
    Run(usize),
}

/// The ordering of a block layout (level first; see [`BlockOrder::run`]).
struct BlockOrder<'a> {
    apath: &'a [Vec<usize>],
    border: &'a [u8],
    preds: &'a [Vec<usize>],
    succs: &'a [Vec<usize>],
    alayer: &'a [usize],
    nlayers: usize,
    /// Per augmented node: its canonical key as a unit of its own level
    /// — a real node by its index (first mention), a channel dummy by
    /// its edge's source node, then (after a node of that key) by the
    /// target node and the edge. Nothing here reads the order of the
    /// edge statements, two parallel edges of one node pair excepted.
    canon: Vec<(usize, u8, usize, usize)>,
    /// Per subgraph: its least node index, a block's canonical key.
    first: Vec<usize>,
    /// Per augmented node: its run, or `usize::MAX`.
    run: Vec<usize>,
    runs: usize,
    /// The levels, top-down: (subgraph path, first layer, last layer).
    levels: Vec<(Vec<usize>, usize, usize)>,
    /// Per subgraph: its place among its sibling blocks (the level
    /// order's result), one for every layer they share.
    brank: Vec<f64>,
    /// Per augmented node: its slot in its own level on its layer —
    /// just before the block of that rank.
    slot: Vec<f64>,
}

impl BlockOrder<'_> {
    /// One layer's members regrouped: at every level the blocks in
    /// their order, the level's nodes and runs in their slots between
    /// them by `key` (their sweep positions, or nothing: the canonical
    /// order decides), every block's members together.
    fn arrange(&self, items: &[usize], depth: usize, key: &dyn Fn(usize) -> f64) -> Vec<usize> {
        if items.len() <= 1 {
            return items.to_vec();
        }
        let mut groups: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
        let mut index: BTreeMap<usize, usize> = BTreeMap::new();
        for &v in items {
            match self.apath[v].get(depth) {
                Some(&c) => match index.get(&c) {
                    Some(&gi) => groups[gi].1.push(v),
                    None => {
                        index.insert(c, groups.len());
                        groups.push((Some(c), vec![v]));
                    }
                },
                None => groups.push((None, vec![v])),
            }
        }
        type Key = (f64, f64, (usize, u8, usize, usize));
        let mut keyed: Vec<(Key, Option<usize>, Vec<usize>)> = groups
            .into_iter()
            .map(|(c, members)| {
                let v = members[0];
                let k = match (members.len(), self.border[v], c) {
                    // A bare wall anchors to its block's edge.
                    (1, 1, _) => (f64::NEG_INFINITY, 0.0, self.canon[v]),
                    (1, 2, _) => (f64::INFINITY, 0.0, self.canon[v]),
                    (_, _, Some(c)) => (self.brank[c], 0.0, (self.first[c], 0, 0, 0)),
                    _ => (self.slot[v], key(v), self.canon[v]),
                };
                (k, c, members)
            })
            .collect();
        keyed.sort_by(|a, b| {
            a.0 .0
                .total_cmp(&b.0 .0)
                .then(a.0 .1.total_cmp(&b.0 .1))
                .then(a.0 .2.cmp(&b.0 .2))
        });
        let mut out = Vec::with_capacity(items.len());
        for (_, c, members) in keyed {
            if c.is_some() && members.len() > 1 {
                out.extend(self.arrange(&members, depth + 1, key));
            } else {
                out.extend(members);
            }
        }
        out
    }

    fn regroup(&self, layer: &mut Vec<usize>, pos: &mut [f64]) {
        if layer.len() > 1 {
            let arranged = self.arrange(layer, 0, &|v| pos[v]);
            *layer = arranged;
        }
        for (i, &v) in layer.iter().enumerate() {
            pos[v] = i as f64;
        }
    }

    /// The units of the level `path` on `layer`, in its order, with
    /// where each starts in the layer and how many members it has. They
    /// are contiguous: the level's own walls stand at their two ends.
    fn units(&self, layer: &[usize], path: &[usize]) -> Vec<(OrderUnit, usize, usize)> {
        let k = path.len();
        let mut out: Vec<(OrderUnit, usize, usize)> = Vec::new();
        for (i, &v) in layer.iter().enumerate() {
            let p = &self.apath[v];
            if p.len() < k || p[..k] != *path {
                continue;
            }
            match p.get(k) {
                Some(&c) => match out.last_mut() {
                    Some((OrderUnit::Block(b), _, len)) if *b == c => *len += 1,
                    _ => out.push((OrderUnit::Block(c), i, 1)),
                },
                None if self.border[v] != 0 => {}
                None if self.run[v] != usize::MAX => out.push((OrderUnit::Run(self.run[v]), i, 1)),
                None => out.push((OrderUnit::Node(v), i, 1)),
            }
        }
        out
    }

    /// Write the level's units back to their layer in the order `order`
    /// (indices into `units`), each block's members as they stand.
    fn place_units(
        layer: &mut [usize],
        pos: &mut [f64],
        units: &[(OrderUnit, usize, usize)],
        order: &[usize],
    ) {
        let Some(&(_, start, _)) = units.first() else {
            return;
        };
        let members: Vec<usize> = order
            .iter()
            .flat_map(|&i| layer[units[i].1..units[i].1 + units[i].2].iter().copied())
            .collect();
        for (i, v) in members.into_iter().enumerate() {
            layer[start + i] = v;
            pos[v] = (start + i) as f64;
        }
    }

    /// Layered crossings in the gaps from layer `from` to layer `to`.
    fn crossings(&self, layers: &[Vec<usize>], pos: &[f64], from: usize, to: usize) -> usize {
        let mut total = 0;
        let mut es: Vec<(usize, usize)> = Vec::new();
        let mut buf: Vec<usize> = Vec::new();
        for li in from..to.min(self.nlayers.saturating_sub(1)) {
            es.clear();
            for &u in &layers[li] {
                for &v in &self.succs[u] {
                    if self.alayer[v] == li + 1 {
                        es.push((pos[u] as usize, pos[v] as usize));
                    }
                }
            }
            es.sort_unstable();
            let mut lower: Vec<usize> = es.iter().map(|e| e.1).collect();
            buf.resize(lower.len(), 0);
            total += inversions(&mut lower, &mut buf);
        }
        total
    }

    /// One half sweep of a level's order (down: every layer by the
    /// layer above it; up: by the layer below). Each unit's barycentre
    /// is the mean place of its links' other ends in that layer, links
    /// inside one child block left out; a long unit (a block, a run)
    /// takes ONE key for all its layers, the mean of its barycentres,
    /// so two blocks keep one order on every layer they share and a run
    /// stays on one side of a block. Each layer is then the long units
    /// in that order with the level's nodes merged in by their own
    /// barycentres; ties fall back to the canonical order.
    fn half_sweep(
        &self,
        path: &[usize],
        (lo, hi): (usize, usize),
        down: bool,
        layers: &mut [Vec<usize>],
        pos: &mut [f64],
    ) {
        let k = path.len();
        let ns = self.first.len();
        let mut block = vec![(0.0f64, 0.0f64); ns];
        let mut runs = vec![(0.0f64, 0.0f64); self.runs];
        let mut own = vec![0.0f64; self.alayer.len()];
        let order: Vec<usize> = if down {
            (lo..=hi).collect()
        } else {
            (lo..=hi).rev().collect()
        };
        let canon = |u: OrderUnit, at: usize, layer: &[usize]| match u {
            OrderUnit::Block(c) => (self.first[c], 0, 0, 0),
            _ => self.canon[layer[at]],
        };
        for &l in &order {
            let units = self.units(&layers[l], path);
            if units.is_empty() {
                continue;
            }
            let beside = if down {
                l.checked_sub(1)
            } else {
                (l + 1 < self.nlayers).then_some(l + 1)
            };
            let len = layers[l].len() as f64;
            let mut keys: Vec<f64> = Vec::with_capacity(units.len());
            for &(u, at, n) in &units {
                let (mut sum, mut links) = (0.0, 0.0);
                if let Some(b) = beside {
                    let blen = layers[b].len() as f64;
                    for &w in &layers[l][at..at + n] {
                        let nbrs = if down { &self.preds[w] } else { &self.succs[w] };
                        for &x in nbrs {
                            let inside = matches!(u, OrderUnit::Block(c)
                                if self.apath[x].len() > k
                                    && self.apath[x][k] == c
                                    && self.apath[x][..k] == *path);
                            if self.alayer[x] == b && !inside {
                                sum += (pos[x] + 0.5) / blen;
                                links += 1.0;
                            }
                        }
                    }
                }
                let key = if links > 0.0 {
                    sum / links
                } else {
                    (at as f64 + n as f64 / 2.0) / len
                };
                // Running mean for a long unit, so the sweep carries on
                // from where its earlier layers put it.
                let key = match u {
                    OrderUnit::Block(c) => {
                        block[c] = (block[c].0 + key, block[c].1 + 1.0);
                        block[c].0 / block[c].1
                    }
                    OrderUnit::Run(r) => {
                        runs[r] = (runs[r].0 + key, runs[r].1 + 1.0);
                        runs[r].0 / runs[r].1
                    }
                    OrderUnit::Node(v) => {
                        own[v] = key;
                        key
                    }
                };
                keys.push(key);
            }
            let mut by: Vec<usize> = (0..units.len()).collect();
            by.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]).then(a.cmp(&b)));
            Self::place_units(&mut layers[l], pos, &units, &by);
        }
        // Every layer by the final keys.
        for l in lo..=hi {
            let units = self.units(&layers[l], path);
            let key = |u: OrderUnit| match u {
                OrderUnit::Block(c) => block[c].0 / block[c].1.max(1.0),
                OrderUnit::Run(r) => runs[r].0 / runs[r].1.max(1.0),
                OrderUnit::Node(v) => own[v],
            };
            let mut by: Vec<usize> = (0..units.len()).collect();
            by.sort_by(|&a, &b| {
                key(units[a].0)
                    .total_cmp(&key(units[b].0))
                    .then_with(|| {
                        canon(units[a].0, units[a].1, &layers[l]).cmp(&canon(
                            units[b].0,
                            units[b].1,
                            &layers[l],
                        ))
                    })
            });
            Self::place_units(&mut layers[l], pos, &units, &by);
        }
    }

    /// The order of one level's units (the level `li` of `levels`):
    /// half sweeps down and up from the order they stand in and from
    /// its reverse, judged after each half by the layered crossings of
    /// the layers the level spans (and the gaps just outside it, whose
    /// far layers stay as the levels above left them); the least wins,
    /// the earlier on a tie. The result is each child block's rank and
    /// each node's and run dummy's slot between the blocks, which the
    /// sweeps over the whole graph then keep.
    fn order_level(&mut self, li: usize, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        let (path, lo, hi) = self.levels[li].clone();
        let span = (lo, hi);
        let judge = |me: &Self, layers: &[Vec<usize>], pos: &[f64]| {
            me.crossings(layers, pos, lo.saturating_sub(1), hi + 1)
        };
        let restore = |layers: &mut [Vec<usize>], pos: &mut [f64], from: &[Vec<usize>]| {
            layers[lo..=hi].clone_from_slice(from);
            for lv in &layers[lo..=hi] {
                for (i, &v) in lv.iter().enumerate() {
                    pos[v] = i as f64;
                }
            }
        };
        let long = (lo..=hi).any(|l| {
            self.units(&layers[l], &path)
                .iter()
                .any(|u| !matches!(u.0, OrderUnit::Node(_)))
        });
        if long {
            let base: Vec<Vec<usize>> = layers[lo..=hi].to_vec();
            let mut best = (judge(self, layers, pos), base.clone());
            for start in 0..2 {
                if best.0 == 0 {
                    break;
                }
                restore(layers, pos, &base);
                if start == 1 {
                    for l in lo..=hi {
                        let units = self.units(&layers[l], &path);
                        let by: Vec<usize> = (0..units.len()).rev().collect();
                        Self::place_units(&mut layers[l], pos, &units, &by);
                    }
                    let c = judge(self, layers, pos);
                    if c < best.0 {
                        best = (c, layers[lo..=hi].to_vec());
                    }
                }
                for _ in 0..LEVEL_ROUNDS {
                    let before = best.0;
                    for down in [true, false] {
                        self.half_sweep(&path, span, down, layers, pos);
                        let c = judge(self, layers, pos);
                        if c < best.0 {
                            best = (c, layers[lo..=hi].to_vec());
                        }
                    }
                    if best.0 == 0 || best.0 >= before {
                        break;
                    }
                }
            }
            restore(layers, pos, &best.1);
        }
        // The blocks' ranks: one sequence that every layer's order is a
        // part of.
        let k = path.len();
        let mut blocks: Vec<usize> = Vec::new();
        let mut after: Vec<(usize, usize)> = Vec::new();
        for lv in &layers[lo..=hi] {
            let mut prev: Option<usize> = None;
            for (u, _, _) in self.units(lv, &path) {
                if let OrderUnit::Block(c) = u {
                    if !blocks.contains(&c) {
                        blocks.push(c);
                    }
                    if let Some(p) = prev {
                        after.push((p, c));
                    }
                    prev = Some(c);
                }
            }
        }
        blocks.sort_by_key(|&c| self.first[c]);
        let mut rank = 0.0;
        let mut left: Vec<usize> = blocks.clone();
        while !left.is_empty() {
            // The first block (canonically) no other waits before.
            let at = left
                .iter()
                .position(|&c| !after.iter().any(|&(p, q)| q == c && left.contains(&p)))
                .unwrap_or(0);
            let c = left.remove(at);
            self.brank[c] = rank;
            rank += 1.0;
        }
        for lv in &layers[lo..=hi] {
            let units = self.units(lv, &path);
            let mut next = rank;
            for &(u, at, _) in units.iter().rev() {
                match u {
                    OrderUnit::Block(c) => next = self.brank[c],
                    _ => self.slot[lv[at]] = next - 0.5,
                }
            }
            // The level's own walls keep to its ends.
            for &v in lv {
                if self.border[v] != 0 && self.apath[v].len() == k && self.apath[v] == path {
                    self.slot[v] = 0.0;
                }
            }
        }
    }

    /// The sweeps over the whole graph inside the slots the level
    /// orders left: weighted-median reorders and adjacent swaps as
    /// before blocks, every layer regrouped as soon as it is reordered,
    /// judged after the downward half and after the upward half (judged
    /// after the upward half only, every chain parks where its target
    /// is). Returns the crossings of the order kept.
    fn sweep(&self, layers: &mut Vec<Vec<usize>>, pos: &mut [f64]) -> usize {
        let n = self.nlayers;
        for lv in layers.iter_mut() {
            self.regroup(lv, pos);
        }
        let mut best = (self.crossings(layers, pos, 0, n), layers.clone());
        for _ in 0..SLOT_ROUNDS {
            if best.0 == 0 {
                break;
            }
            let before = best.0;
            for li in 1..n {
                reorder(&mut layers[li], self.preds, pos);
                self.regroup(&mut layers[li], pos);
            }
            let c = self.crossings(layers, pos, 0, n);
            if c < best.0 {
                best = (c, layers.clone());
            }
            for li in (0..n.saturating_sub(1)).rev() {
                reorder(&mut layers[li], self.succs, pos);
                self.regroup(&mut layers[li], pos);
            }
            transpose(
                layers,
                self.preds,
                self.succs,
                pos,
                n,
                Some(self.apath),
                Some(self.border),
            );
            for lv in layers.iter_mut() {
                self.regroup(lv, pos);
            }
            let c = self.crossings(layers, pos, 0, n);
            if c < best.0 {
                best = (c, layers.clone());
            }
            if best.0 >= before {
                break;
            }
        }
        *layers = best.1;
        for lv in layers.iter() {
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
        best.0
    }

    /// The canonical order: on every layer the units of each level form
    /// one sequence by key, a block's members together inside it.
    fn canonical(&mut self, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        self.brank.iter_mut().for_each(|r| *r = 0.0);
        self.slot.iter_mut().for_each(|s| *s = 0.0);
        for lv in layers.iter_mut() {
            if lv.len() > 1 {
                let arranged = self.arrange(lv, 0, &|_| 0.0);
                *lv = arranged;
            }
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
    }

    /// Start `k > 0`: the canonical order shuffled — per level one
    /// permutation of its long units, and per layer one of the level's
    /// own nodes; never a layer as a whole, so two blocks start in one
    /// order on every layer and a run on one side of a block. One
    /// generator stream per start, consumed level by level (top-down),
    /// the long units first and then the layers in ascending order;
    /// integer arithmetic only.
    fn shuffle(&self, k: usize, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        const MUL: u64 = 6_364_136_223_846_793_005;
        const INC: u64 = 1_442_695_040_888_963_407;
        let mut x = (k as u64).wrapping_mul(MUL).wrapping_add(INC);
        let mut draw = |below: usize| -> usize {
            x = x.wrapping_mul(MUL).wrapping_add(INC);
            ((x >> 33) % below as u64) as usize
        };
        let mut fisher_yates = |n: usize| -> Vec<usize> {
            let mut p: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                p.swap(i, draw(i + 1));
            }
            p
        };
        for (path, lo, hi) in &self.levels {
            // The level's long units, in canonical order of first sight.
            let mut long: Vec<OrderUnit> = Vec::new();
            for lv in &layers[*lo..=*hi] {
                for (u, _, _) in self.units(lv, path) {
                    if !matches!(u, OrderUnit::Node(_)) && !long.contains(&u) {
                        long.push(u);
                    }
                }
            }
            let perm = fisher_yates(long.len());
            let place = |u: OrderUnit| long.iter().position(|&x| x == u).map_or(0, |i| perm[i]);
            for l in *lo..=*hi {
                let units = self.units(&layers[l], path);
                let (mut longs, mut shorts): (Vec<usize>, Vec<usize>) =
                    (0..units.len()).partition(|&i| !matches!(units[i].0, OrderUnit::Node(_)));
                longs.sort_by_key(|&i| place(units[i].0));
                let p = fisher_yates(shorts.len());
                shorts = p.iter().map(|&i| shorts[i]).collect();
                // The layer's pattern of long and short places stays.
                let (mut li, mut si) = (0, 0);
                let by: Vec<usize> = units
                    .iter()
                    .map(|u| {
                        if matches!(u.0, OrderUnit::Node(_)) {
                            si += 1;
                            shorts[si - 1]
                        } else {
                            li += 1;
                            longs[li - 1]
                        }
                    })
                    .collect();
                Self::place_units(&mut layers[l], pos, &units, &by);
            }
        }
    }

    /// The order a later placement of one layout call starts from: on
    /// each layer the elements `seed` (the first placement's final
    /// order) has there stay in that order, and a node or dummy new to
    /// the layer goes before the first kept element that follows it in
    /// the canonical order (at the end without one), inside its block.
    /// Returns whether any element is new.
    fn warm(
        &mut self,
        seed: &OrderSeed,
        element: &dyn Fn(usize) -> Option<(usize, usize)>,
        layers: &mut [Vec<usize>],
        pos: &mut [f64],
    ) -> bool {
        self.canonical(layers, pos);
        let mut fresh = false;
        let mut key = vec![0.0f64; self.alayer.len()];
        for (l, lv) in layers.iter().enumerate() {
            let kept: BTreeMap<(usize, usize), usize> = seed
                .get(l)
                .into_iter()
                .flatten()
                .enumerate()
                .map(|(i, &e)| (e, i))
                .collect();
            // Canonical order of the layer, back to front: each new
            // element takes the key just before the kept one after it.
            let mut next = kept.len() as f64;
            for &v in lv.iter().rev() {
                match element(v).and_then(|e| kept.get(&e)) {
                    Some(&i) => {
                        next = i as f64;
                        key[v] = next;
                    }
                    None => {
                        fresh |= self.border[v] == 0;
                        key[v] = next - 0.5;
                    }
                }
            }
        }
        // A block's place: the mean of its members' keys over all its
        // layers (one for every layer, as the level orders keep it).
        let ns = self.first.len();
        let mut acc = vec![(0.0f64, 0.0f64); ns];
        for (v, path) in self.apath.iter().enumerate() {
            if self.border[v] != 0 {
                continue;
            }
            let share = key[v] / layers[self.alayer[v]].len().max(1) as f64;
            for &c in path {
                acc[c] = (acc[c].0 + share, acc[c].1 + 1.0);
            }
        }
        for c in 0..ns {
            self.brank[c] = acc[c].0 / acc[c].1.max(1.0);
        }
        for lv in layers.iter_mut() {
            let len = lv.len().max(1) as f64;
            for &v in lv.iter() {
                self.slot[v] = key[v] / len;
            }
            if lv.len() > 1 {
                let arranged = self.arrange(lv, 0, &|_| 0.0);
                *lv = arranged;
            }
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
        fresh
    }

    /// Source order among equals: after the winning start, two adjacent
    /// nodes or runs of one level in one slot swap when the left one is
    /// later in the canonical order and the layered crossings do not
    /// rise. Each swap removes one inversion against the canonical
    /// order, so it ends. (A shuffled start leaves the members that
    /// change no crossing wherever the shuffle put them.)
    fn tidy(&self, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        for li in 0..self.nlayers {
            let mut swapped = true;
            while swapped {
                swapped = false;
                for i in 0..layers[li].len().saturating_sub(1) {
                    let (v, w) = (layers[li][i], layers[li][i + 1]);
                    if self.border[v] != 0
                        || self.border[w] != 0
                        || self.apath[v] != self.apath[w]
                        || self.canon[v] <= self.canon[w]
                    {
                        continue;
                    }
                    let now = local_crossings(v, w, self.preds, pos)
                        + local_crossings(v, w, self.succs, pos);
                    let then = local_crossings(w, v, self.preds, pos)
                        + local_crossings(w, v, self.succs, pos);
                    if then <= now {
                        layers[li].swap(i, i + 1);
                        pos[v] = (i + 1) as f64;
                        pos[w] = i as f64;
                        swapped = true;
                    }
                }
            }
        }
    }

    /// One run from the order the layers stand in: the level orders
    /// top-down (each level's blocks and runs first, then inside each
    /// block), then the sweeps inside their slots. Returns the layered
    /// crossings.
    fn settle(&mut self, layers: &mut Vec<Vec<usize>>, pos: &mut [f64]) -> usize {
        for li in 0..self.levels.len() {
            self.order_level(li, layers, pos);
        }
        self.sweep(layers, pos)
    }

    /// The ordering of a block layout, level first.
    ///
    /// Every level's units are ordered before the members inside them:
    /// its child blocks and the channels passing it as whole units
    /// ([`BlockOrder::order_level`]), the root first, then each block's
    /// inside with everything outside it fixed. Flat sweeps then order
    /// the nodes and channel dummies inside the slots that leaves
    /// ([`BlockOrder::sweep`]). Flat sweeps alone cannot move a block
    /// past its sibling, and a block-swap judged by crossings changes
    /// nothing useful.
    ///
    /// This runs from `starts` starts — the canonical order, then
    /// shuffles of it ([`BlockOrder::shuffle`]) — and the start with the
    /// fewest layered crossings wins, the lowest on a tie; then the
    /// crossing-neutral neighbours go back to source order
    /// ([`BlockOrder::tidy`]). With `seed` (a later placement of the
    /// same layout call) there is one run, from the first placement's
    /// final order ([`BlockOrder::warm`]): its layers differ from the
    /// winner's, and from a start of its own it would lose to the
    /// multi-start best by construction.
    fn run(
        &mut self,
        starts: usize,
        seed: Option<(&OrderSeed, &dyn Fn(usize) -> Option<(usize, usize)>)>,
        layers: &mut Vec<Vec<usize>>,
        pos: &mut [f64],
    ) {
        if let Some((seed, element)) = seed {
            if self.warm(seed, element, layers, pos) {
                self.settle(layers, pos);
            }
            self.tidy(layers, pos);
            return;
        }
        self.canonical(layers, pos);
        let base = layers.clone();
        let mut best: Option<(usize, Vec<Vec<usize>>)> = None;
        for k in 0..starts.max(1) {
            *layers = base.clone();
            for lv in layers.iter() {
                for (i, &v) in lv.iter().enumerate() {
                    pos[v] = i as f64;
                }
            }
            if k > 0 {
                self.shuffle(k, layers, pos);
            }
            let c = self.settle(layers, pos);
            if best.as_ref().map_or(true, |b| c < b.0) {
                best = Some((c, layers.clone()));
            }
            if c == 0 {
                break;
            }
        }
        if let Some((_, order)) = best {
            *layers = order;
        }
        for lv in layers.iter() {
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
        self.tidy(layers, pos);
    }
}

/// Most rounds (a half sweep down, one up) of a level's order.
const LEVEL_ROUNDS: usize = 12;
/// Most rounds of the sweeps inside the slots.
const SLOT_ROUNDS: usize = 16;

'''
anchor = "/// Reorder one layer by the weighted median of each node's neighbour"
assert s.count(anchor)==1
s = s.replace(anchor, code.lstrip('\n') + anchor)
open(p,'w').write(s)
