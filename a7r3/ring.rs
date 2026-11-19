/// Cone (radians) a disc of radius `disc` fills seen from the root at
/// distance `r` — the angle between its two tangents (ring layout).
fn cone(disc: f64, r: f64) -> f64 {
    if r <= 0.0 {
        return 0.0;
    }
    2.0 * (disc / r).min(1.0).asin()
}

/// Radius of every ring of the fallback layout (depth `k` on ring `k`)
/// for ring gaps `gap` (from ring `k - 1` out to ring `k`). Every node
/// counts as its disc `disc[i]`, and `band[k]` is the widest disc of
/// depth `k`, so each ring is at least the two bands out from the one
/// inside it: boxes of different depths never meet. A ring whose discs
/// do not fit side by side round it grows until they do, and if the
/// subtree wedges (see [`ring_extents`]) still overrun the circle, every
/// ring scales out together. Both growths are exact in one step: a
/// disc's cone shrinks at least in proportion as its ring grows
/// (`asin` is convex), so scaling the radius by `need / TAU` brings the
/// need to `TAU` or under.
fn ring_radii(
    d: &Mindmap,
    pre: &[usize],
    disc: &[f64],
    band: &[f64],
    gap: &[f64],
    e: &mut [f64],
) -> Vec<f64> {
    let mut rad = vec![0.0f64; band.len()];
    let mut by_depth: Vec<Vec<usize>> = vec![Vec::new(); band.len()];
    for (i, n) in d.nodes.iter().enumerate() {
        by_depth[n.depth].push(i);
    }
    for k in 1..band.len() {
        rad[k] = rad[k - 1] + gap[k].max(band[k - 1] + band[k]);
        let need: f64 = by_depth[k].iter().map(|&i| cone(disc[i], rad[k])).sum();
        if need > TAU {
            rad[k] *= need / TAU * (1.0 + 1e-9);
        }
    }
    for _ in 0..8 {
        let need = ring_extents(d, pre, disc, &rad, e);
        if need <= TAU {
            break;
        }
        for r in rad.iter_mut() {
            *r *= need / TAU * (1.0 + 1e-9);
        }
    }
    rad
}

/// Bottom-up angular extent of every subtree on the rings (fallback):
/// the larger of its own node's cone and its children's extents
/// together. Returns the root's — the circle the top-level wedges need.
fn ring_extents(d: &Mindmap, pre: &[usize], disc: &[f64], rad: &[f64], e: &mut [f64]) -> f64 {
    for &i in pre.iter().rev() {
        let n = &d.nodes[i];
        let own = cone(disc[i], rad[n.depth]);
        let kids: f64 = n.children.iter().map(|&c| e[c]).sum();
        e[i] = own.max(kids);
    }
    e[0]
}

/// Ring gaps whose connectors break the ring layout's guarantee (see
/// [`ring_centres`]): `true` at depth `k + 1` when a node on ring `k`
/// has a connector to a child that does not run outward all the way
/// (it bends back towards the root, where it can cross another) or
/// that runs through one of that child's siblings. The root's own
/// connectors are rays and always clear.
fn ring_faults(d: &Mindmap, sizes: &[(f64, f64)], c: &[(f64, f64)]) -> Vec<bool> {
    let max_depth = d.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let mut faults = vec![false; max_depth + 1];
    for (i, n) in d.nodes.iter().enumerate() {
        if n.depth == 0 || faults[n.depth + 1.min(max_depth - n.depth)] {
            continue;
        }
        let p = c[i];
        let bad = n.children.iter().any(|&ch| {
            let q = c[ch];
            // The distance from the root grows all along p→q exactly
            // when it grows at p (it is convex along a line).
            let outward = p.0 * (q.0 - p.0) + p.1 * (q.1 - p.1) > 0.0;
            !outward
                || n.children.iter().any(|&o| {
                    let (w, h) = sizes[o];
                    o != ch
                        && segment_hits_box(
                            p,
                            q,
                            (
                                c[o].0 - w / 2.0 - SEG_CLEAR,
                                c[o].1 - h / 2.0 - SEG_CLEAR,
                                c[o].0 + w / 2.0 + SEG_CLEAR,
                                c[o].1 + h / 2.0 + SEG_CLEAR,
                            ),
                        )
                })
        });
        if bad {
            faults[n.depth + 1] = true;
        }
    }
    faults
}

/// The fallback ring layout (flowmaid's original, made safe for any
/// tree): depth `k` on a ring round the root, at least [`RING`] out from
/// the ring inside it. Every box counts as a disc (half its diagonal
/// plus half of [`SEP`]):
/// - each ring sits far enough out that its discs clear the previous
///   ring's, and far enough round that they fit side by side (see
///   [`ring_radii`]), so boxes of different depths never meet;
/// - each subtree owns a wedge as wide as its own disc's cone and its
///   children's wedges together; the root's children share the whole
///   circle clockwise from 12 o'clock, so boxes of one depth never meet
///   either;
/// - a node's children sit packed side by side centred on it, not spread
///   over its whole wedge, so every connector stays inside its parent's
///   wedge.
///
/// A connector from ring `k` to ring `k + 1` that runs outward all the
/// way therefore crosses no other and can pass through no box but its
/// own child's siblings'. Wherever one does not run outward or does pass
/// through a sibling (a wide family on a ring close to its parent's),
/// the gap out to that ring is widened (×[`RING_GROWTH`]) and the layout
/// redone: the children's cones narrow and their connectors straighten,
/// until none is left ([`ring_faults`]). Sparse, but clean.
fn ring_centres(d: &Mindmap, sizes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = d.nodes.len();
    let pre = preorder(d);
    let disc: Vec<f64> = sizes
        .iter()
        .map(|&(w, h)| w.hypot(h) / 2.0 + SEP / 2.0)
        .collect();
    let max_depth = d.nodes.iter().map(|nd| nd.depth).max().unwrap_or(0);
    let mut band = vec![0.0f64; max_depth + 1];
    for (i, nd) in d.nodes.iter().enumerate() {
        band[nd.depth] = band[nd.depth].max(disc[i]);
    }
    let mut gap = vec![RING; max_depth + 1];
    let mut e = vec![0.0f64; n];
    let mut c = vec![(0.0f64, 0.0f64); n];
    for _ in 0..RING_GROWTHS {
        let rad = ring_radii(d, &pre, &disc, &band, &gap, &mut e);
        ring_extents(d, &pre, &disc, &rad, &mut e);
        // Each node's wedge, top-down; a node sits at its wedge's middle.
        let mut wedge = vec![(0.0f64, 0.0f64); n];
        wedge[0] = (-FRAC_PI_2, TAU - FRAC_PI_2);
        for &i in &pre {
            let kids = &d.nodes[i].children;
            if kids.is_empty() {
                continue;
            }
            let total: f64 = kids.iter().map(|&k| e[k]).sum();
            let (a0, a1) = wedge[i];
            // The root shares its circle out in proportion; anyone else
            // packs its children centred on itself.
            let (mut at, scale) = if i == 0 {
                (a0, (a1 - a0) / total.max(1e-12))
            } else {
                ((a0 + a1 - total) / 2.0, 1.0)
            };
            for &k in kids {
                let span = e[k] * scale;
                wedge[k] = (at, at + span);
                let mid = at + span / 2.0;
                let r = rad[d.nodes[k].depth];
                c[k] = (r * mid.cos(), r * mid.sin());
                at += span;
            }
        }
        let faults = ring_faults(d, sizes, &c);
        if !faults.contains(&true) {
            break;
        }
        for (g, f) in gap.iter_mut().zip(&faults) {
            if *f {
                *g *= RING_GROWTH;
            }
        }
    }
    c
}

