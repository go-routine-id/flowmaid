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
