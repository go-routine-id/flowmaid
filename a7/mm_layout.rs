
// ------------------------------------------------------------------
// Layout
// ------------------------------------------------------------------

/// Distance from the centre of a `w × h` box to its border along the
/// unit vector `u`.
fn ext(w: f64, h: f64, u: (f64, f64)) -> f64 {
    let ex = if u.0.abs() > 1e-9 {
        w / 2.0 / u.0.abs()
    } else {
        f64::INFINITY
    };
    let ey = if u.1.abs() > 1e-9 {
        h / 2.0 / u.1.abs()
    } else {
        f64::INFINITY
    };
    ex.min(ey)
}

/// Centre distance from a parent box to a child box placed along `u`:
/// their borders `gap` apart on the line between their centres, and
/// never closer than [`SEP`] anywhere (two boxes stopping short along
/// a diagonal can still clip corners, so the boxes' Minkowski sum is
/// checked too).
fn spoke_len(p: (f64, f64), c: (f64, f64), u: (f64, f64), gap: f64) -> f64 {
    let along = ext(p.0, p.1, u) + ext(c.0, c.1, u) + gap;
    let apart = ext(p.0 + c.0 + 2.0 * SEP, p.1 + c.1 + 2.0 * SEP, u);
    along.max(apart)
}

/// `a` wrapped into `(-π, π]`.
fn wrap(a: f64) -> f64 {
    let mut a = a % TAU;
    if a <= -PI {
        a += TAU;
    } else if a > PI {
        a -= TAU;
    }
    a
}

/// The parent-centred fan layout. Works on centres relative to a
/// subtree's own root, built bottom-up inside a top-down pass:
/// [`Fan::subtree`] lays out a node's children (and, recursively, theirs)
/// around it and records where they sit relative to it.
struct Fan<'a> {
    d: &'a Mindmap,
    size: &'a [(f64, f64)],
    /// Centre of each node relative to the subtree root that last laid
    /// it out (after a commit, relative to its parent's frame).
    rel: Vec<(f64, f64)>,
    /// Preorder of the tree and each node's `[start, end)` span in it,
    /// so a subtree's members are one slice.
    pre: Vec<usize>,
    span: Vec<(usize, usize)>,
    work: usize,
}

impl Fan<'_> {
    /// Every strict descendant of `i`.
    fn below(&self, i: usize) -> &[usize] {
        let (s, e) = self.span[i];
        &self.pre[s + 1..e]
    }

    /// Lay out `i`'s descendants with `i` at the origin, its own
    /// connector arriving along direction `dir` (radians, screen
    /// coordinates: 0 = right, π/2 = down). Writes `rel` for every
    /// descendant, relative to `i`. `false` when the work budget ran out
    /// or a fan could not be made to fit; the caller then falls back.
    ///
    /// Each child sits on a spoke from `i` (see [`spoke_len`]). The
    /// children's whole subtrees are measured as angular intervals seen
    /// from `i` and packed side by side, [`FAN_GAP`] apart, so sibling
    /// subtrees live in disjoint cones from `i` — they cannot overlap
    /// and their connectors cannot cross. Packing moves the children,
    /// which reshapes their (axis-aligned) subtrees, so it re-measures
    /// until the cones it measured are the cones it packed; a fan that
    /// will not fit its arc lengthens its spokes and tries again.
    fn subtree(&mut self, i: usize, dir: f64) -> bool {
        let kids = self.d.nodes[i].children.clone();
        if kids.is_empty() {
            return true;
        }
        self.work += 1;
        if self.work > FAN_WORK {
            return false;
        }
        let root = self.d.nodes[i].parent.is_none();
        let k = kids.len();
        let max_arc = if root { TAU } else { MAX_FAN };
        let mut gap = if root { TRUNK } else { SPOKE };
        // First guess: an even spread, clockwise.
        let initial: Vec<f64> = if root {
            (0..k)
                .map(|j| -FRAC_PI_2 + TAU * (j as f64 + 0.5) / k as f64)
                .collect()
        } else {
            let arc = (FAN_PER_CHILD * (k as f64 - 1.0)).min(max_arc);
            (0..k)
                .map(|j| {
                    if k == 1 {
                        dir
                    } else {
                        dir - arc / 2.0 + arc * j as f64 / (k as f64 - 1.0)
                    }
                })
                .collect()
        };
        let (pw, ph) = self.size[i];
        for _ in 0..FAN_GROWTHS {
            let mut dirs = initial.clone();
            for round in 0..FAN_ROUNDS {
                let mut iv: Vec<(f64, f64)> = Vec::with_capacity(k);
                let mut off: Vec<(f64, f64)> = Vec::with_capacity(k);
                let mut hits_parent = false;
                for (j, &c) in kids.iter().enumerate() {
                    let u = (dirs[j].cos(), dirs[j].sin());
                    let r = spoke_len(self.size[i], self.size[c], u, gap);
                    let o = (u.0 * r, u.1 * r);
                    if !self.subtree(c, dirs[j]) {
                        return false;
                    }
                    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                    let members = std::iter::once((c, o)).chain(self.below(c).iter().map(|&m| {
                        let q = self.rel[m];
                        (m, (o.0 + q.0, o.1 + q.1))
                    }));
                    for (m, q) in members {
                        let (w, h) = self.size[m];
                        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                            let a = wrap((q.1 + sy * h / 2.0).atan2(q.0 + sx * w / 2.0) - dirs[j]);
                            lo = lo.min(a);
                            hi = hi.max(a);
                        }
                        // A grandchild swung back onto this node itself.
                        if m != c
                            && (q.0.abs() < (pw + w) / 2.0 + SEP / 2.0)
                            && (q.1.abs() < (ph + h) / 2.0 + SEP / 2.0)
                        {
                            hits_parent = true;
                        }
                    }
                    iv.push((lo, hi));
                    off.push(o);
                }
                let widths: Vec<f64> = iv.iter().map(|(lo, hi)| hi - lo).collect();
                let gaps = if root { k } else { k - 1 } as f64;
                let total: f64 = widths.iter().sum::<f64>() + FAN_GAP * gaps;
                // A cone must stay convex (< π) for the disjointness
                // argument to hold.
                if hits_parent || total > max_arc || widths.iter().any(|&w| w > PI * 0.9) {
                    break;
                }
                // Are the cones we just measured already disjoint?
                let mut disjoint = round > 0;
                for j in 0..k {
                    let next = if j + 1 < k {
                        dirs[j + 1] + iv[j + 1].0
                    } else if root && k > 1 {
                        dirs[0] + iv[0].0 + TAU
                    } else {
                        continue;
                    };
                    if dirs[j] + iv[j].1 + FAN_GAP * 0.5 > next + 1e-9 {
                        disjoint = false;
                    }
                }
                if disjoint {
                    for (j, &c) in kids.iter().enumerate() {
                        let o = off[j];
                        self.rel[c] = o;
                        for idx in self.span[c].0 + 1..self.span[c].1 {
                            let m = self.pre[idx];
                            self.rel[m] = (self.rel[m].0 + o.0, self.rel[m].1 + o.1);
                        }
                    }
                    return true;
                }
                // Pack the measured cones: the root shares its slack out
                // evenly round the full circle from 12 o'clock; any other
                // node centres its fan on `dir`, spread up to
                // FAN_PER_CHILD a child where its arc has room.
                let (mut at, step) = if root {
                    let g = FAN_GAP + (TAU - total) / k as f64;
                    (-FRAC_PI_2 + g / 2.0, g)
                } else {
                    let want = (FAN_PER_CHILD * k as f64).min(max_arc);
                    let g = if k > 1 {
                        FAN_GAP + (want - total).max(0.0) / (k as f64 - 1.0)
                    } else {
                        0.0
                    };
                    let span = widths.iter().sum::<f64>() + g * (k as f64 - 1.0);
                    (dir - span / 2.0, g)
                };
                for j in 0..k {
                    dirs[j] = at - iv[j].0;
                    at += widths[j] + step;
                }
            }
            gap *= 1.25;
        }
        false
    }
}

/// Centres (root at the origin) from the parent-centred fan layout, or
/// `None` when it could not produce a clean layout — see [`scene`].
fn fan_centres(d: &Mindmap, size: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let n = d.nodes.len();
    let mut pre = Vec::with_capacity(n);
    let mut span = vec![(0, 0); n];
    let mut stack = vec![(0usize, false)];
    while let Some((i, done)) = stack.pop() {
        if done {
            span[i].1 = pre.len();
            continue;
        }
        span[i].0 = pre.len();
        pre.push(i);
        stack.push((i, true));
        for &c in d.nodes[i].children.iter().rev() {
            stack.push((c, false));
        }
    }
    if pre.len() != n {
        return None;
    }
    let mut fan = Fan {
        d,
        size,
        rel: vec![(0.0, 0.0); n],
        pre,
        span,
        work: 0,
    };
    if !fan.subtree(0, -FRAC_PI_2) {
        return None;
    }
    let pos = fan.rel;
    clean(d, size, &pos).then_some(pos)
}

/// The fan layout's safety net: no two boxes closer than half of
/// [`SEP`], no connector crossing another, none running through a box
/// other than its own two ends. The cone packing guarantees these by
/// construction; this re-checks the finished geometry (skipped on huge
/// maps, where the quadratic check would cost more than the layout).
fn clean(d: &Mindmap, size: &[(f64, f64)], pos: &[(f64, f64)]) -> bool {
    let n = pos.len();
    if n > 1500 {
        return true;
    }
    let boxes: Vec<(f64, f64, f64, f64)> = (0..n)
        .map(|i| {
            let (w, h) = size[i];
            (pos[i].0 - w / 2.0, pos[i].1 - h / 2.0, pos[i].0 + w / 2.0, pos[i].1 + h / 2.0)
        })
        .collect();
    let m = SEP / 2.0 - 1e-6;
    for a in 0..n {
        for b in a + 1..n {
            let (p, q) = (boxes[a], boxes[b]);
            if p.0 < q.2 + m && q.0 < p.2 + m && p.1 < q.3 + m && q.1 < p.3 + m {
                return false;
            }
        }
    }
    let edges: Vec<(usize, usize)> = d
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, nd)| nd.parent.map(|p| (p, i)))
        .collect();
    for (x, &(a, b)) in edges.iter().enumerate() {
        for &(c, e) in &edges[x + 1..] {
            if a == c || a == e || b == c || b == e {
                continue;
            }
            if segments_cross(pos[a], pos[b], pos[c], pos[e]) {
                return false;
            }
        }
        for (i, bx) in boxes.iter().enumerate() {
            if i != a && i != b && segment_hits_box(pos[a], pos[b], *bx) {
                return false;
            }
        }
    }
    true
}

/// Proper crossing of segments `p1p2` and `p3p4`.
fn segments_cross(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> bool {
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let (d1, d2) = (cross(p3, p4, p1), cross(p3, p4, p2));
    let (d3, d4) = (cross(p1, p2, p3), cross(p1, p2, p4));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Does segment `a`→`b` pass through the box `(x0, y0, x1, y1)`?
/// (Liang–Barsky clip.)
fn segment_hits_box(a: (f64, f64), b: (f64, f64), bx: (f64, f64, f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - bx.0),
        (dx, bx.2 - a.0),
        (-dy, a.1 - bx.1),
        (dy, bx.3 - a.1),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t0 < t1
}

/// Angular half-width (radians) a node occupies on a ring of radius
/// `r` (fallback layout): the box diagonal projected onto the circle.
fn angular_half(diag: f64, r: f64) -> f64 {
    if r <= 0.0 {
        return 0.0;
    }
    ((diag * 0.5) / r).min(1.0).asin()
}

/// Bottom-up angular extent of each subtree on the rings (fallback).
fn extent(d: &Mindmap, i: usize, diag: &[f64], rad: &[f64], e: &mut [f64]) -> f64 {
    let own = if d.nodes[i].depth == 0 {
        0.0
    } else {
        2.0 * angular_half(diag[i], rad[d.nodes[i].depth]) + GAP
    };
    let kids = &d.nodes[i].children;
    let total = if kids.is_empty() {
        own
    } else {
        own.max(kids.iter().map(|&c| extent(d, c, diag, rad, e)).sum())
    };
    e[i] = total;
    total
}

/// Place subtree `i` inside the wedge `[a0, a1]` on the rings (fallback).
fn place(d: &Mindmap, e: &[f64], rad: &[f64], i: usize, a0: f64, a1: f64, c: &mut [(f64, f64)]) {
    let kids = &d.nodes[i].children;
    if kids.is_empty() {
        return;
    }
    let total: f64 = kids.iter().map(|&k| e[k]).sum();
    let mut acc = a0;
    for &k in kids {
        let span = if total > 0.0 {
            (a1 - a0) * (e[k] / total)
        } else {
            0.0
        };
        let (cs, ce) = (acc, acc + span);
        let mid = (cs + ce) / 2.0;
        let r = rad[d.nodes[k].depth];
        c[k] = (r * mid.cos(), r * mid.sin());
        place(d, e, rad, k, cs, ce, c);
        acc = ce;
    }
}

/// The fallback ring layout (flowmaid's original): depth `k` on a ring
/// of radius `k · RING` (grown until its nodes fit), each subtree owning
/// a wedge sized by its boxes' diagonals. Sparse, but overlap- and
/// crossing-free for any tree, so it backs the fan layout up.
fn ring_centres(d: &Mindmap, sizes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let diag: Vec<f64> = sizes.iter().map(|&(w, h)| w.hypot(h)).collect();
    let max_depth = d.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let mut rad: Vec<f64> = (0..=max_depth).map(|k| RING * k as f64).collect();
    let mut demand: Vec<f64> = vec![0.0; max_depth + 1];
    for (i, n) in d.nodes.iter().enumerate() {
        if n.depth > 0 {
            demand[n.depth] += 2.0 * angular_half(diag[i], rad[n.depth]) + GAP;
        }
    }
    for k in 1..=max_depth {
        if demand[k] > TAU {
            rad[k] *= demand[k] / TAU;
        }
    }
    let mut e = vec![0.0f64; d.nodes.len()];
    extent(d, 0, &diag, &rad, &mut e);
    let mut c = vec![(0.0f64, 0.0f64); d.nodes.len()];
    place(d, &e, &rad, 0, -FRAC_PI_2, TAU - FRAC_PI_2, &mut c);
    c
}

/// Branch accent for a top-level subtree, by its order under the root.
fn branch_color(order: usize) -> &'static str {
    crate::style::accent(order)
}

/// Auto-layout: the parent-centred fan layout (see the module docs),
/// then [`route`] with those centres. Should a fan ever fail to fit or
/// the finished geometry fail its overlap/crossing check, the whole map
/// falls back to the ring layout, which is sparse but always clean. An
/// empty mindmap yields a minimal canvas.
pub fn scene(d: &Mindmap) -> MindScene {
    if d.nodes.is_empty() {
        return route(d, &[]);
    }
    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(mind_node_size).collect();
    let c = fan_centres(d, &sizes).unwrap_or_else(|| ring_centres(d, &sizes));

    // Normalise so the whole tree's bounding box starts at MARGIN, then
    // hand the resulting centres to route().
    let (mut minx, mut miny) = (f64::MAX, f64::MAX);
    for (i, &(x, y)) in c.iter().enumerate() {
        minx = minx.min(x - sizes[i].0 / 2.0);
        miny = miny.min(y - sizes[i].1 / 2.0);
    }
    let centres: Vec<(f64, f64)> = c
        .iter()
        .map(|&(x, y)| (x + MARGIN - minx, y + MARGIN - miny))
        .collect();
    route(d, &centres)
}

/// Connector width by the child's depth: a heavy trunk from the root
/// halving per level (9, 4.5, 2.25, then 2), so the hierarchy reads
/// from the strokes alone (mermaid steps 11 → 5).
fn link_width(depth: usize) -> f64 {
    (9.0 * 0.5f64.powi(depth.saturating_sub(1) as i32)).max(2.0)
}

/// Build a scene from EXPLICIT node centres (`pos[i]` = centre of node
/// `i`, in world coords). Used for interactive drag: the caller moves a
/// centre and the connectors are recomputed to follow, without touching
/// the layout. Positions are taken as-is (not re-normalised), so
/// dragging never makes the diagram slide. `pos` must match node count.
pub fn route(d: &Mindmap, pos: &[(f64, f64)]) -> MindScene {
    if d.nodes.is_empty() || pos.len() != d.nodes.len() {
        return MindScene {
            width: 2.0 * MARGIN,
            height: 2.0 * MARGIN,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }

    let mut branch_order = vec![0usize; d.nodes.len()];
    for (order, &ch) in d.nodes[0].children.iter().enumerate() {
        branch_order[ch] = order;
    }

    let mut nodes = Vec::with_capacity(d.nodes.len());
    for (i, n) in d.nodes.iter().enumerate() {
        let (w2, h2) = mind_node_size(n);
        // The root is the deep ink; branches fill with their accent.
        let fill = if n.depth == 0 {
            ROOT_FILL
        } else {
            branch_color(branch_order[n.branch.unwrap_or(i)])
        };
        // Text is white or dark ink by contrast with the fill actually
        // painted (white alone failed AA on most accents).
        let text_color = text_on(n.style.fill.as_deref().unwrap_or(fill));
        let (font_size, font_weight) = font_of(n.depth);
        nodes.push(MindNodeBox {
            x: pos[i].0 - w2 / 2.0,
            y: pos[i].1 - h2 / 2.0,
            w: w2,
            h: h2,
            text: n.text.clone(),
            shape: n.shape,
            depth: n.depth,
            fill,
            text_color,
            font_size,
            font_weight,
            icon: n.icon.clone(),
            classes: n.classes.clone(),
            style: n.style.clone(),
        });
    }

    // Connectors: centre-to-centre, coloured by the child's branch and
    // tapering away from the root; the node boxes hide both endpoints.
    let mut edges = Vec::new();
    for (i, n) in d.nodes.iter().enumerate() {
        let Some(p) = n.parent else { continue };
        let (a, b) = (
            (nodes[p].cx(), nodes[p].cy()),
            (nodes[i].cx(), nodes[i].cy()),
        );
        let c1 = (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0);
        let c2 = (a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0);
        edges.push(MindLink {
            bezier: [a, c1, c2, b],
            color: nodes[i].fill,
            width: link_width(n.depth),
        });
    }

    let width = nodes.iter().map(|n| n.x + n.w).fold(0.0, f64::max) + MARGIN;
    let height = nodes.iter().map(|n| n.y + n.h).fold(0.0, f64::max) + MARGIN;
    MindScene {
        width,
        height,
        nodes,
        edges,
    }
}
