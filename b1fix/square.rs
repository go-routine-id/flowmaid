/// The commands of `e`'s scene curve as drawn, mapped through `t`
/// (scene → output coordinates), as (start point, commands): a
/// non-default mermaid `curve` through its points ([`curve_geometry`]),
/// else the `curveBasis` spline through routed waypoints
/// ([`spline_segs`]), else the single cubic `bezier`. The points are
/// mapped before the spline's arithmetic, as the painter always did.
fn plain_segs(e: &SceneEdge, t: impl Fn((f64, f64)) -> (f64, f64)) -> ((f64, f64), Vec<Seg>) {
    if let Some((start, segs)) = curve_geometry(e) {
        (t(start), segs.iter().map(|s| s.map(&t)).collect())
    } else if e.waypoints.len() >= 2 {
        let q: Vec<(f64, f64)> = e.waypoints.iter().map(|&p| t(p)).collect();
        spline_segs(&q)
    } else {
        let q = e.bezier.map(&t);
        (q[0], vec![Seg::C(q[1], q[2], q[3])])
    }
}

/// The path `(start, segs)` of `e` ([`plain_segs`]) with each glyph end
/// squared to its glyph ([`square_end`]), and whether any end changed:
/// an end glyph of an edge the flowchart painter strokes (an ER or class
/// scene's painter draws its own ends; a self-loop keeps its loop). An
/// arrowhead's end gets at least the straight run the painter cuts under
/// it ([`arrow_trims`]), so the head faces along the curve's own end
/// tangent with its tip on the end point; the default curve gets up to
/// a [`GLYPH_STUB`] — a mermaid `curve` is the author's and only gets
/// what its head needs.
fn square_ends(e: &SceneEdge, start: (f64, f64), segs: Vec<Seg>) -> ((f64, f64), Vec<Seg>, bool) {
    if e.paint_width.is_some() || e.from == e.to || !edge_drawn(e) {
        return (start, segs, false);
    }
    let author = e.meta.curve.is_some_and(|c| c != EdgeCurve::Basis);
    let trims = arrow_trims(e);
    let head = Some(edge_stroke_width(e) / 2.0);
    let mut out = (start, segs, false);
    for (at_start, m, trim) in [
        (false, e.ends.end_marker(e.kind), trims.1),
        (true, e.ends.start_marker(e.kind), trims.0),
    ] {
        let Some(m) = m else { continue };
        let want = if author { trim } else { GLYPH_STUB.max(trim) };
        let head = if m == Marker::Arrow { head } else { None };
        let (s, g, changed) = if at_start {
            let (s, g) = reverse_segs(out.0, &out.1);
            let (s, g, changed) = square_end(s, g, want, trim, head);
            if changed {
                let (s, g) = reverse_segs(s, &g);
                (s, g, true)
            } else {
                (out.0, out.1, false)
            }
        } else {
            square_end(out.0, out.1, want, trim, head)
        };
        out = (s, g, out.2 || changed);
    }
    out
}

/// The path `(start, segs)` run backwards, as (its end, the commands
/// back to its start): the same curve exactly.
fn reverse_segs(start: (f64, f64), segs: &[Seg]) -> ((f64, f64), Vec<Seg>) {
    let mut froms = Vec::with_capacity(segs.len());
    let mut cur = start;
    for s in segs {
        froms.push(cur);
        cur = s.end();
    }
    let back = segs
        .iter()
        .zip(&froms)
        .rev()
        .map(|(&s, &f)| match s {
            Seg::L(_) => Seg::L(f),
            Seg::Q(c, _) => Seg::Q(c, f),
            Seg::C(c1, c2, _) => Seg::C(c2, c1, f),
        })
        .collect();
    (cur, back)
}

/// Most (px) a squared end ([`square_end`]) strays from the curve the
/// scene holds, outside the arrowhead drawn over it: the routing, port
/// and label stages spaced every clearance (chips, node corners, other
/// lines, crossing gaps) against that curve, and a host draws it.
const SQUARE_END_DEV: f64 = 1.0;
/// Straight runs [`square_end`] tries, longest first, in parts of the
/// run it wants.
const SQUARE_END_RUNS: [f64; 3] = [1.0, 0.75, 0.5];
/// Lengths (in [`GLYPH_STUB`]s) of the bend from the curve into the
/// straight run that [`square_end`] tries, gentlest first.
const SQUARE_END_BENDS: [f64; 5] = [3.0, 2.0, 1.5, 1.0, 0.5];
/// Arc step (px) the curve is sampled at where [`square_end`] measures
/// its stray.
const SQUARE_END_STEP: f64 = 0.5;
/// How far (px) the curve may lie off its end tangent and still count
/// as already running straight into the end.
const SQUARE_END_FLAT: f64 = 0.05;

/// The path `(start, segs)` with its END squared to the glyph drawn
/// there (report §2.3, M1), and whether it changed: the curve's last
/// stretch is replaced by a straight run along its own end tangent into
/// the end point — so the glyph, oriented along it, points square into
/// the side, and the stroke meets the glyph's base already heading its
/// way instead of still turning under it — and a bend that leaves the
/// curve smoothly (no kink) some way back and joins the run. The run is
/// `want` px when the bend then keeps within [`SQUARE_END_DEV`] of the
/// curve (measured outside the arrowhead, `head` the stroke's half
/// width when one covers the end), else shorter; never under `need`
/// (the stretch the painter cuts under an arrowhead), whatever the
/// stray: a head must face along the end tangent with its tip on the
/// end point. Unchanged when the curve already runs straight that far,
/// or has no direction at its end. The rest of the curve is its own.
fn square_end(
    start: (f64, f64),
    segs: Vec<Seg>,
    want: f64,
    need: f64,
    head: Option<f64>,
) -> ((f64, f64), Vec<Seg>, bool) {
    let n = segs.len();
    if n == 0 || want <= 0.0 {
        return (start, segs, false);
    }
    let mut froms = Vec::with_capacity(n);
    let mut cur = start;
    for s in &segs {
        froms.push(cur);
        cur = s.end();
    }
    let b = cur;
    let Some(d) = end_heading(froms[n - 1], segs[n - 1]) else {
        return (start, segs, false);
    };
    let lens: Vec<f64> = segs
        .iter()
        .zip(&froms)
        .map(|(&s, &f)| seg_lengths(f, s)[TRIM_STEPS])
        .collect();
    let total: f64 = lens.iter().sum();
    let longest = want + SQUARE_END_BENDS[0] * GLYPH_STUB;
    let tail = tail_samples(&froms, &segs, &lens, longest.min(total));
    // Off the end tangent (signed), and back along it from the end.
    let lat = |q: (f64, f64)| (q.0 - b.0) * d.1 - (q.1 - b.1) * d.0;
    let along = |q: (f64, f64)| (b.0 - q.0) * d.0 + (b.1 - q.1) * d.1;
    if tail
        .iter()
        .take_while(|&&(s, _)| s <= want)
        .all(|&(_, q)| lat(q).abs() <= SQUARE_END_FLAT && along(q) >= -SQUARE_END_FLAT)
    {
        return (start, segs, false);
    }
    // A point the arrowhead covers: inside its triangle (tip on the end,
    // a head long and wide) with the stroke's whole width.
    let covered = |q: (f64, f64)| {
        head.is_some_and(|hw| {
            let a = along(q);
            (0.0..=ARROW_MARKER_PX).contains(&a) && lat(q).abs() + hw <= a / 2.0
        })
    };
    // Where command `i` ends, in arc back from the end.
    let mut back = vec![0.0; n];
    for i in (0..n - 1).rev() {
        back[i] = back[i + 1] + lens[i + 1];
    }
    // One candidate: a straight run of `run` px, the bend leaving the
    // curve `run + bend` px back (its start, when shorter). As (stray,
    // commands).
    let candidate = |run: f64, bend: f64| -> Option<(f64, Vec<Seg>)> {
        let s_j = run + bend;
        let (i, u) = if s_j >= total - 1e-9 {
            (0, 0.0)
        } else {
            let i = (0..n).find(|&i| back[i] <= s_j && s_j < back[i] + lens[i])?;
            (i, seg_param_at(froms[i], segs[i], lens[i] - (s_j - back[i])))
        };
        let mut out: Vec<Seg> = segs[..i].to_vec();
        let j = if u > 0.0 {
            let (_, part) = segs[i].part(froms[i], 0.0, u);
            out.push(part);
            part.end()
        } else {
            froms[i]
        };
        let tj = seg_heading_at(froms[i], segs[i], u)?;
        let p = (b.0 - d.0 * run, b.1 - d.1 * run);
        let chord = (p.0 - j.0, p.1 - j.1);
        // A bend that would have to turn back on itself is no bend.
        if tj.0 * chord.0 + tj.1 * chord.1 <= 0.0 || d.0 * chord.0 + d.1 * chord.1 <= 0.0 {
            return None;
        }
        let k = chord.0.hypot(chord.1) / 3.0;
        let bend_seg = Seg::C(
            (j.0 + tj.0 * k, j.1 + tj.1 * k),
            (p.0 - d.0 * k, p.1 - d.1 * k),
            p,
        );
        // The stray, both ways, outside the head: the new stretch's
        // points from the curve's, the curve's from the new stretch's.
        let bb = bend_seg.cubic(j);
        let mut new_pts: Vec<(f64, f64)> = (0..=8).map(|k| cubic_at(&bb, k as f64 / 8.0)).collect();
        new_pts.extend([lerp(p, b, 0.5), b]);
        let old_pts: Vec<(f64, f64)> = tail
            .iter()
            .take_while(|&&(s, _)| s <= s_j + SQUARE_END_STEP)
            .map(|&(_, q)| q)
            .collect();
        let far = |pts: &[(f64, f64)], line: &[(f64, f64)]| {
            pts.iter()
                .filter(|&&q| !covered(q))
                .map(|&q| {
                    line.windows(2)
                        .map(|w| point_seg_dist(q.0, q.1, w[0], w[1]))
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(0.0, f64::max)
        };
        let stray = far(&new_pts, &old_pts).max(far(&old_pts, &new_pts));
        out.push(bend_seg);
        if run > 0.0 {
            out.push(Seg::L(b));
        }
        Some((stray, out))
    };
    let mut runs: Vec<f64> = SQUARE_END_RUNS
        .iter()
        .map(|f| f * want)
        .filter(|&r| r > need)
        .collect();
    if need > 0.0 {
        runs.push(need);
    }
    for &run in &runs {
        for &bend in &SQUARE_END_BENDS {
            if let Some((stray, out)) = candidate(run, bend * GLYPH_STUB) {
                if stray <= SQUARE_END_DEV {
                    return (start, out, true);
                }
            }
        }
    }
    if need <= 0.0 {
        return (start, segs, false);
    }
    // No run keeps that close: the head's own, with the bend that strays
    // least (a curve turning hard right at its end, a short edge).
    let best = SQUARE_END_BENDS
        .iter()
        .filter_map(|&bend| candidate(need, bend * GLYPH_STUB))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    match best {
        Some((_, out)) => (start, out, true),
        None => (start, segs, false),
    }
}

/// The unit heading of `seg` (from `from`) INTO its end, read off the
/// nearest control point that is not the end itself; `None` for a
/// command without length.
fn end_heading(from: (f64, f64), seg: Seg) -> Option<(f64, f64)> {
    let p = seg.end();
    let ctl: Vec<(f64, f64)> = match seg {
        Seg::L(_) => vec![from],
        Seg::Q(c, _) => vec![c, from],
        Seg::C(c1, c2, _) => vec![c2, c1, from],
    };
    let q = ctl
        .into_iter()
        .find(|q| (p.0 - q.0).hypot(p.1 - q.1) > 1e-9)?;
    let l = (p.0 - q.0).hypot(p.1 - q.1);
    Some(((p.0 - q.0) / l, (p.1 - q.1) / l))
}

/// The unit heading of `seg` (from `from`) at parameter `u`, forward;
/// `None` where it has no direction.
fn seg_heading_at(from: (f64, f64), seg: Seg, u: f64) -> Option<(f64, f64)> {
    let b = seg.cubic(from);
    let v = 1.0 - u;
    let mut t = (
        3.0 * (v * v * (b[1].0 - b[0].0) + 2.0 * u * v * (b[2].0 - b[1].0) + u * u * (b[3].0 - b[2].0)),
        3.0 * (v * v * (b[1].1 - b[0].1) + 2.0 * u * v * (b[2].1 - b[1].1) + u * u * (b[3].1 - b[2].1)),
    );
    if t.0.hypot(t.1) < 1e-9 {
        // A cusp or a coincident control point: the chord either side.
        let (a, c) = (cubic_at(&b, (u - 1e-3).max(0.0)), cubic_at(&b, (u + 1e-3).min(1.0)));
        t = (c.0 - a.0, c.1 - a.1);
    }
    let l = t.0.hypot(t.1);
    (l > 1e-12).then(|| (t.0 / l, t.1 / l))
}

/// The last `reach` px of arc of a path (`froms[i]` where command `i`
/// starts, `lens[i]` its length), end first, as (arc back from the
/// end, point) every [`SQUARE_END_STEP`] px or so — the end itself and
/// the stretch's far end included.
fn tail_samples(
    froms: &[(f64, f64)],
    segs: &[Seg],
    lens: &[f64],
    reach: f64,
) -> Vec<(f64, (f64, f64))> {
    let n = segs.len();
    let mut out = vec![(0.0, segs[n - 1].end())];
    let mut back = 0.0;
    for i in (0..n).rev() {
        let left = reach - back;
        if left <= 0.0 {
            break;
        }
        let len = lens[i];
        if len <= 0.0 {
            continue;
        }
        let u0 = if left >= len {
            0.0
        } else {
            seg_param_at(froms[i], segs[i], len - left)
        };
        let steps = (left.min(len) / SQUARE_END_STEP).ceil().max(1.0) as usize;
        let b = segs[i].cubic(froms[i]);
        let (mut prev, mut s) = (segs[i].end(), back);
        for k in 1..=steps {
            let q = cubic_at(&b, 1.0 - (1.0 - u0) * k as f64 / steps as f64);
            s += (q.0 - prev.0).hypot(q.1 - prev.1);
            out.push((s, q));
            prev = q;
        }
        back += len;
    }
    out
}
