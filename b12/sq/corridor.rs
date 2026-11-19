/// A corridor edge of [`corridors`] (B12 R3): one of `g.edges` whose ends
/// lie in two different units of the drawing's top level (a top-level
/// block, or a node outside every block), with its source's and its
/// target's centre and size and the indices of the two units in the
/// unit list.
pub(crate) struct CorridorEdge {
    /// The source's centre and size.
    pub a: ((f64, f64), (f64, f64)),
    /// The target's centre and size.
    pub b: ((f64, f64), (f64, f64)),
    /// The source's and the target's unit (indices into the units).
    pub units: (usize, usize),
}

/// What [`corridors`] gives an edge.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Corridor {
    /// The two units do not stand one after the other along the flow
    /// with room for a step out and a step in: drawn as before.
    Fallback,
    /// Nothing between the two units stands in the way of a path that
    /// steps once (today's channel, [`cross_cluster_route`]).
    Channel,
    /// The bare corners (on the axes, final coordinates) of a path past
    /// the units that stand between: one breadth per band.
    Path(Vec<(f64, f64)>),
}

/// One band of a corridor plan: the leg the path takes past it.
struct Leg {
    /// The band's extent along the flow (edge-local coordinates).
    l: (f64, f64),
    /// The leg's breadth before slots are dealt.
    at: f64,
    /// 0: the leg keeps the breadth it came with; -1 / +1: it shifted
    /// to the low / high side, to the free gap whose near edge is
    /// `edge`.
    side: i8,
    /// The near edge of the gap the leg shifted to (its slot 0).
    edge: f64,
    /// The far edge of that gap: no slot lies past it.
    far: f64,
}

/// A corridor's plan before slots are dealt.
struct Plan {
    /// +1 when the target lies below the source on the screen, -1 above:
    /// edge-local flow coordinate `l = s * y`.
    s: f64,
    /// The source's port, the target's (edge-local).
    p0: (f64, f64),
    p3: (f64, f64),
    /// Where the path may turn between two bands (edge-local, along the
    /// flow): one per gap, before the first band to after the last.
    turns: Vec<f64>,
    legs: Vec<Leg>,
}

/// B12 R3, in [`route`] and [`route_partial`] of a block layout drawn TD
/// or BT: each cross-unit edge's corridor past the units of the top
/// level that stand between its two end units along the flow.
///
/// For an edge whose end units (`S`, `T`) stand one after the other with
/// room for a step out and a step in ([`cross_cluster_route`]'s test),
/// its obstacles are the other units (`units`, every top-level frame and
/// every node outside the frames) whose extent along the flow meets the
/// open interval between `S` and `T`. They group into *bands*: maximal
/// runs that overlap along the flow. When the source's column can run
/// down to some gap between two bands and the target's column on from
/// there, nothing is in the way of today's channel, which steps once
/// ([`Corridor::Channel`]). Otherwise the path takes one breadth per
/// band: the breadth it comes with where that lies in a free gap of the
/// band ([`LANE_PAD`] clear of every obstacle), else the free gap nearest
/// it (R5; the lower breadth on a tie), at that gap's near edge, and it
/// turns in the gaps between the bands, at their middles. The edges that
/// shift into one gap of one band from one side take *slots* there, a
/// [`crate::ortho::TRACK_PITCH`] apart from the near edge, in the order
/// of the breadth their path takes next (the next band's, or the
/// target's column): the one heading furthest that way lies furthest
/// that way, so a hub's edges down one corridor nest instead of
/// crossing (by edge index alone they would follow the order of the
/// statements). Ties keep edge order. A path that would have to turn in
/// a gap shorter than two corner roundings keeps today's drawing
/// ([`Corridor::Fallback`]).
///
/// The same rule draws an edge against the flow (E3: its target unit
/// upstream of its source unit), mirrored: out of the source's upstream
/// side, in through the target's downstream side, never in a lane. Only
/// the units' boxes as they stand are read, so a drag moves a corridor
/// continuously within one choice of gaps (R5).
pub(crate) fn corridors(edges: &[CorridorEdge], units: &[(f64, f64, f64, f64)]) -> Vec<Corridor> {
    let mut plans: Vec<Option<Plan>> = Vec::with_capacity(edges.len());
    let mut out: Vec<Corridor> = Vec::with_capacity(edges.len());
    for e in edges {
        match corridor_plan(e, units) {
            Ok(p) => {
                plans.push(Some(p));
                out.push(Corridor::Fallback);
            }
            Err(c) => {
                plans.push(None);
                out.push(c);
            }
        }
    }
    // Slots: the legs that shifted into one gap of one band from one
    // side, in the order of the breadth each path takes next.
    let next_of = |p: &Plan, k: usize| -> f64 { p.legs.get(k + 1).map_or(p.p3.0, |l| l.at) };
    let key = |p: &Plan, k: usize| -> (i64, i64, i64, i64, i8) {
        let r = |x: f64| (x * 100.0).round() as i64;
        let l = &p.legs[k];
        let (l0, l1) = (p.s * l.l.0, p.s * l.l.1);
        (r(l0.min(l1)), r(l0.max(l1)), r(l.edge), r(l.far), l.side)
    };
    let mut groups: BTreeMap<(i64, i64, i64, i64, i8), Vec<(f64, usize, usize)>> = BTreeMap::new();
    for (i, p) in plans.iter().enumerate() {
        let Some(p) = p else { continue };
        for (k, l) in p.legs.iter().enumerate() {
            if l.side != 0 {
                groups
                    .entry(key(p, k))
                    .or_default()
                    .push((next_of(p, k), i, k));
            }
        }
    }
    let pitch = crate::ortho::TRACK_PITCH;
    let mut slot_at: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for ((_, _, _, _, side), mut legs) in groups {
        let side = f64::from(side);
        // Slot 0 at the near edge; the next one a pitch further in.
        legs.sort_by(|a, b| (side * a.0).total_cmp(&(side * b.0)).then(a.1.cmp(&b.1)));
        for (slot, &(_, i, k)) in legs.iter().enumerate() {
            let l = &plans[i].as_ref().expect("planned").legs[k];
            let at = l.edge + side * slot as f64 * pitch;
            // Never past the gap's far edge: the last slots share it.
            let at = if side > 0.0 { at.min(l.far) } else { at.max(l.far) };
            slot_at.insert((i, k), at);
        }
    }
    for (i, p) in plans.iter().enumerate() {
        let Some(p) = p else { continue };
        let breadth: Vec<f64> = (0..p.legs.len())
            .map(|k| slot_at.get(&(i, k)).copied().unwrap_or(p.legs[k].at))
            .collect();
        let mut pts: Vec<(f64, f64)> = vec![p.p0];
        let mut at = p.p0.0;
        let wanted = breadth.iter().copied().chain(std::iter::once(p.p3.0));
        for (k, b) in wanted.enumerate() {
            if (b - at).abs() > 1e-9 {
                pts.push((at, p.turns[k]));
                pts.push((b, p.turns[k]));
                at = b;
            }
        }
        pts.push(p.p3);
        out[i] = Corridor::Path(pts.into_iter().map(|(b, l)| (b, p.s * l)).collect());
    }
    out
}

/// The plan of one corridor edge before slots ([`corridors`]), or what
/// the edge keeps instead.
fn corridor_plan(e: &CorridorEdge, units: &[(f64, f64, f64, f64)]) -> Result<Plan, Corridor> {
    const GAP: f64 = 16.0;
    let ((ac, asz), (bc, bsz)) = (e.a, e.b);
    let (Some(&sb), Some(&tb)) = (units.get(e.units.0), units.get(e.units.1)) else {
        return Err(Corridor::Fallback);
    };
    // Edge-local flow: `l = s * y` grows from the source to the target.
    let s = if sb.1 + sb.3 + 2.0 * GAP <= tb.1 {
        1.0
    } else if tb.1 + tb.3 + 2.0 * GAP <= sb.1 {
        -1.0
    } else {
        return Err(Corridor::Fallback);
    };
    let span = |r: &(f64, f64, f64, f64)| {
        let (a, b) = (s * r.1, s * (r.1 + r.3));
        (r.0, r.0 + r.2, a.min(b), a.max(b))
    };
    let (sr, tr) = (span(&sb), span(&tb));
    let (s_out, t_in) = (sr.3, tr.2);
    let mut obstacles: Vec<(f64, f64, f64, f64)> = units
        .iter()
        .enumerate()
        .filter(|&(u, _)| u != e.units.0 && u != e.units.1)
        .map(|(_, r)| span(r))
        .filter(|r| r.3 > s_out + 1e-9 && r.2 < t_in - 1e-9 && r.0.is_finite() && r.1.is_finite())
        .collect();
    if obstacles.is_empty() {
        return Err(Corridor::Channel);
    }
    obstacles.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.total_cmp(&b.0)));
    // Bands: maximal runs of obstacles that overlap along the flow.
    let mut bands: Vec<((f64, f64), Vec<(f64, f64)>)> = Vec::new();
    for o in obstacles {
        match bands.last_mut() {
            Some((l, bs)) if o.2 < l.1 => {
                l.1 = l.1.max(o.3);
                bs.push((o.0, o.1));
            }
            _ => bands.push(((o.2, o.3), vec![(o.0, o.1)])),
        }
    }
    // Each band's free gaps across the flow, `LANE_PAD` clear of its
    // obstacles: the complement of their padded breadths.
    let free: Vec<Vec<(f64, f64)>> = bands
        .iter()
        .map(|(_, bs)| {
            let mut bs: Vec<(f64, f64)> = bs.iter().map(|&(a, b)| (a - LANE_PAD, b + LANE_PAD)).collect();
            bs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut out = Vec::new();
            let mut lo = f64::NEG_INFINITY;
            for (a, b) in bs {
                if a > lo {
                    out.push((lo, a));
                }
                lo = lo.max(b);
            }
            out.push((lo, f64::INFINITY));
            out
        })
        .collect();
    let clear = |k: usize, x: f64| free[k].iter().any(|&(a, b)| a <= x && x <= b);
    let a = (ac.0, s * ac.1 + asz.1 / 2.0);
    let b = (bc.0, s * bc.1 - bsz.1 / 2.0);
    // Where the path may turn: the middle of each gap between two bands,
    // before the first and after the last.
    let n = bands.len();
    let gaps: Vec<(f64, f64)> = (0..=n)
        .map(|k| {
            let lo = if k == 0 { s_out } else { bands[k - 1].0 .1 };
            let hi = if k == n { t_in } else { bands[k].0 .0 };
            (lo, hi)
        })
        .collect();
    let roomy = |k: usize| gaps[k].1 - gaps[k].0 >= 2.0 * crate::router::CORNER_R;
    // One step in a gap clears every band: today's channel.
    if (0..=n).any(|k| roomy(k) && (0..k).all(|j| clear(j, a.0)) && (k..n).all(|j| clear(j, b.0))) {
        return Err(Corridor::Channel);
    }
    let mut legs: Vec<Leg> = Vec::with_capacity(n);
    let mut at = a.0;
    for (k, ((l0, l1), _)) in bands.iter().enumerate() {
        let leg = if clear(k, at) {
            Leg {
                l: (*l0, *l1),
                at,
                side: 0,
                edge: at,
                far: at,
            }
        } else {
            // The free gap nearest the breadth the path comes with: its
            // near edge (the lower one on a tie).
            let mut best: Option<(f64, f64, f64, i8)> = None;
            for &(lo, hi) in &free[k] {
                let (edge, far, side) = if hi < at { (hi, lo, -1) } else { (lo, hi, 1) };
                if !edge.is_finite() {
                    continue;
                }
                let d = (edge - at).abs();
                let better = match best {
                    None => true,
                    Some((bd, be, _, _)) => d < bd - 1e-9 || ((d - bd).abs() <= 1e-9 && edge < be),
                };
                if better {
                    best = Some((d, edge, far, side));
                }
            }
            let Some((_, edge, far, side)) = best else {
                return Err(Corridor::Fallback);
            };
            if !roomy(k) {
                return Err(Corridor::Fallback);
            }
            Leg {
                l: (*l0, *l1),
                at: edge,
                side,
                edge,
                far,
            }
        };
        at = leg.at;
        legs.push(leg);
    }
    if (b.0 - at).abs() > 1e-9 && !roomy(n) {
        return Err(Corridor::Fallback);
    }
    Ok(Plan {
        s,
        p0: a,
        p3: b,
        turns: gaps.iter().map(|g| (g.0 + g.1) / 2.0).collect(),
        legs,
    })
}
