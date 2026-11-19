/// Seat every loose part of a box on the box's top layer (report §2.9,
/// B8 review r1 #1). A part is a connected piece of the ranking graph
/// `adj` (real nodes `0..nc.len()`, then rank-only hubs; every link
/// counts, cycle breakers too) that lies wholly inside one box: an
/// edge-less member, a chain only its own members join, a packed grid
/// of notes. Longest-path layering parks each such part on the first
/// layer, beside whatever feeds the drawing — a hub fanning out into
/// boxes then sat level with the edge-less members of the boxes it
/// feeds through one member, its edges striking through their frames
/// to reach the next layer. Mermaid (dagre's nesting edges from the
/// box's top border) draws them in the box's first row instead.
/// A part moves down, whole, until its first layer is the top layer of
/// the members of its outermost box that link out of that box (the
/// anchors): the box's first row. An island box (no link leaves it)
/// hands the part on to the next box inside it that has anchors, and a
/// box no link leaves at any depth keeps its place. Inside its
/// outermost box a part may stay above an inner box's own anchors: the
/// rows it keeps lie within the outer frame anyway, and seating it on
/// the inner box re-laid a dense nested drawing for no frame it cleared
/// (dense-td's lone member of a third-level box, 4 rows down: 20 % more
/// crossings, and its `route()` left two long edges through nodes and
/// ran 30 % slower). Parts are seated
/// shallowest first, so an anchor never moves after a part was seated
/// on it. Every link stays inside its part, so each keeps its length
/// and direction.
fn seat_loose_parts(nc: &[Vec<usize>], adj: &[Vec<(usize, usize)>], layer: &mut [usize]) {
    let n = nc.len();
    let nx = adj.len();
    // Undirected parts, numbered by their first node.
    let mut und: Vec<Vec<usize>> = vec![Vec::new(); nx];
    for (u, links) in adj.iter().enumerate() {
        for &(v, _) in links {
            und[u].push(v);
            und[v].push(u);
        }
    }
    let mut part = vec![usize::MAX; nx];
    let mut parts: Vec<Vec<usize>> = Vec::new();
    for s in 0..nx {
        if part[s] != usize::MAX {
            continue;
        }
        let id = parts.len();
        part[s] = id;
        let mut nodes = vec![s];
        let mut i = 0;
        while let Some(&u) = nodes.get(i) {
            i += 1;
            for &v in &und[u] {
                if part[v] == usize::MAX {
                    part[v] = id;
                    nodes.push(v);
                }
            }
        }
        parts.push(nodes);
    }
    // Each part's innermost common box path (empty: it reaches the top
    // level, or holds no real node).
    let paths: Vec<Vec<usize>> = parts
        .iter()
        .map(|nodes| {
            let mut real = nodes.iter().filter(|&&v| v < n);
            let Some(&first) = real.next() else {
                return Vec::new();
            };
            let mut path = nc[first].clone();
            for &v in real {
                let k = path.iter().zip(&nc[v]).take_while(|(a, b)| a == b).count();
                path.truncate(k);
            }
            path
        })
        .collect();
    let mut order: Vec<usize> = (0..parts.len()).filter(|&p| !paths[p].is_empty()).collect();
    if order.is_empty() {
        return;
    }
    order.sort_by_key(|&p| paths[p].len());
    // Per box, its anchors' top layer, read once: an anchor's part
    // reaches no deeper than the box, so it was seated before any part
    // that lies inside the box.
    let mut tops: BTreeMap<usize, Option<usize>> = BTreeMap::new();
    for p in order {
        for (depth, &c) in paths[p].iter().enumerate() {
            let top = *tops.entry(c).or_insert_with(|| {
                (0..n)
                    .filter(|&v| {
                        nc[v].get(depth) == Some(&c) && paths[part[v]].get(depth) != Some(&c)
                    })
                    .map(|v| layer[v])
                    .min()
            });
            let Some(top) = top else {
                continue;
            };
            let first = parts[p]
                .iter()
                .filter(|&&v| v < n)
                .map(|&v| layer[v])
                .min()
                .unwrap_or(top);
            if top > first {
                for &v in &parts[p] {
                    layer[v] += top - first;
                }
            }
            break;
        }
    }
}

