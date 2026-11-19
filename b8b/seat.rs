/// Seat every box below its feeders (report §2.9, B8 review r1 #1 and
/// B8b P1). Longest-path layering parks each member nothing ranks into
/// on the first layer: a hub fanning out into boxes then sat level with
/// the edge-less members of the boxes it feeds through one member, and
/// with their members that only link onward, out of the box — its edges
/// struck through their frames to reach the next layer, and the frame
/// spanned the hub's row. Mermaid (dagre's nesting edges from the box's
/// top border) draws the whole box below the hub instead.
///
/// A box's first row is where its feeders put it: the row just below
/// the topmost node outside the box with a link into it (a real edge,
/// an edge to the box's frame through its rank-only stand-in). Each
/// SOURCE member — a member with no link in at all but from a `lead`
/// note card, edge-less or linking onward — ranks no higher than the
/// first row of the outermost box on its path that has feeders, and
/// everything below it moves along. A box with no feeder keeps its
/// sources where they are, and so does a box whose feeders all hang
/// below its own sources (the box starts with them: a feeder its
/// sources reach, through any box seated on the way, never counts).
/// Inside its outermost box a source may stay above an inner box's own
/// feeders: the rows it keeps lie within the outer frame anyway, and
/// seating it on the inner box re-laid a dense nested drawing for no
/// frame it cleared (dense-td's lone member of a third-level box, 4
/// rows down: 20 % more crossings, and its `route()` left two long
/// edges through nodes and ran 30 % slower).
///
/// Links with an id from `stacks_from` on (narrowing's stackings and
/// wraps) and cycle breakers neither feed a box nor make a member less
/// of a source: a stacking hub links every member of the lower box, so
/// counting it left the box's edge-less members level with the hub
/// again once narrowing stacked or wrapped it. Stackings still rank
/// the box below its sibling: they are links like any other in the
/// layering. `layer` (real nodes `0..nc.len()`, then rank-only hubs)
/// is relaid by longest path with those floors; `lead` cards are placed
/// after this, one layer before their target.
fn seat_box_sources(
    nc: &[Vec<usize>],
    lead: &[usize],
    adj: &[Vec<(usize, usize)>],
    back: &[bool],
    stacks_from: usize,
    layer: &mut [usize],
) {
    let n = nc.len();
    let nx = adj.len();
    let seats = |ei: usize| !back[ei] && ei < stacks_from;
    let is_lead = {
        let mut v = vec![false; n];
        for &l in lead.iter().filter(|&&l| l < n) {
            v[l] = true;
        }
        v
    };
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); nx];
    for (u, links) in adj.iter().enumerate() {
        for &(v, ei) in links {
            if seats(ei) && u != v {
                preds[v].push(u);
            }
        }
    }
    // A feeder of box `c`: a real node outside it (no note card), or a
    // hub one of whose sources is.
    let outside = |x: usize, c: usize| -> bool {
        let out = |f: usize| f < n && !is_lead[f] && !nc[f].contains(&c);
        if x < n {
            out(x)
        } else {
            preds[x].iter().any(|&f| out(f))
        }
    };
    let boxes = nc.iter().flatten().map(|&c| c + 1).max().unwrap_or(0);
    let mut feeders: Vec<Vec<usize>> = vec![Vec::new(); boxes];
    for v in (0..n).filter(|&v| !nc[v].is_empty()) {
        for &x in &preds[v] {
            for &c in &nc[v] {
                if outside(x, c) && !feeders[c].contains(&x) {
                    feeders[c].push(x);
                }
            }
        }
    }
    // Each source member's seat: the outermost fed box on its path.
    let seat: Vec<Option<usize>> = (0..n)
        .map(|v| {
            if is_lead[v] || preds[v].iter().any(|&p| p >= n || !is_lead[p]) {
                return None;
            }
            nc[v].iter().copied().find(|&c| !feeders[c].is_empty())
        })
        .collect();
    if seat.iter().all(Option::is_none) {
        return;
    }
    // The layering graph plus one min-node per box (`nx + c`): its
    // feeders link in, it links to the sources it seats.
    let top = |c: usize| nx + c;
    let total = nx + boxes;
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); total];
    for (u, links) in adj.iter().enumerate() {
        for &(v, ei) in links {
            if !back[ei] {
                out[u].push(v);
            }
        }
    }
    let mut seated = vec![false; boxes];
    for (v, s) in seat.iter().enumerate() {
        if let Some(c) = *s {
            out[top(c)].push(v);
            seated[c] = true;
        }
    }
    let mut fed: Vec<Vec<usize>> = vec![Vec::new(); boxes];
    for c in (0..boxes).filter(|&c| seated[c]) {
        for &x in &feeders[c] {
            out[x].push(top(c));
        }
    }
    // A feeder the box's own sources reach hangs below them: dropping
    // it leaves every cycle through a min-node cut.
    for c in (0..boxes).filter(|&c| seated[c]) {
        let mut seen = vec![false; total];
        let mut stack = vec![top(c)];
        while let Some(u) = stack.pop() {
            if std::mem::replace(&mut seen[u], true) {
                continue;
            }
            stack.extend(out[u].iter().copied());
        }
        fed[c] = feeders[c].iter().copied().filter(|&x| !seen[x]).collect();
    }
    for c in (0..boxes).filter(|&c| seated[c]) {
        for &x in &feeders[c] {
            if !fed[c].contains(&x) {
                let at = out[x].iter().position(|&w| w == top(c)).unwrap();
                out[x].remove(at);
            }
        }
    }
    // Longest path, a min-node taking its topmost feeder's row.
    let step = |u: usize| usize::from(u < n);
    let mut indeg = vec![0usize; total];
    for links in &out {
        for &v in links {
            indeg[v] += 1;
        }
    }
    let mut value = vec![0usize; total];
    let mut row: Vec<Option<usize>> = vec![None; boxes];
    let mut q: VecDeque<usize> = (0..total).filter(|&v| indeg[v] == 0).collect();
    let mut done = 0;
    while let Some(u) = q.pop_front() {
        done += 1;
        if u >= nx {
            value[u] = row[u - nx].unwrap_or(0);
        }
        for &v in &out[u] {
            let at = value[u] + step(u);
            if v >= nx {
                let r = &mut row[v - nx];
                *r = Some(r.map_or(at, |r| r.min(at)));
            } else if at > value[v] {
                value[v] = at;
            }
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push_back(v);
            }
        }
    }
    if done == total {
        layer.copy_from_slice(&value[..nx]);
    }
}
