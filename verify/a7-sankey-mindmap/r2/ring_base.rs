fn angular_half(diag: f64, r: f64) -> f64 {
    if r <= 0.0 {
        return 0.0;
    }
    ((diag * 0.5) / r).min(1.0).asin()
}

/// Bottom-up angular extent (radians) of each subtree: the node's own
/// footprint, or the sum of its children's extents — whichever is wider.
/// `rad` maps a depth to its ring radius.
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

/// Place subtree `i` inside the angular wedge `[a0, a1]` (radians from
/// 12 o'clock, clockwise). Children sit on the next ring and share the
/// wedge proportionally to their computed extents; a leaf records only
/// its centre. Node centres are relative to the root at the origin.
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

/// Branch accent for a top-level subtree, by its order under the root.
fn branch_color(order: usize) -> &'static str {
