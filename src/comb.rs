//! Stage 4b of [`crate::edgepass::finish`]: a fan's comb (report B4).
//!
//! Three or more edges leaving (or entering) one side of a box form a
//! fan. Drawn on the axes ([`crate::ortho`]), each leaves its port on a
//! stub straight out of the side, steps across on a track in the gap in
//! front of it and runs on at the place it turns (its *turn*): a comb.
//! The comb reads as one when its teeth nest — the ports, the tracks and
//! the turns in one order, so no tooth crosses another: of the edges
//! stepping one way, the one turning furthest out leaves from the
//! outermost port on the shallowest track (libavoid's channel order,
//! which the ortho stage and the router's nudge keep for the tracks).
//! The stages after the ortho stage move ports for their own reasons — a
//! reroute takes another column, the nudge's port legs are spread as one
//! channel and the static reorder (G9) re-places a side by where its
//! legs head — and a hub's fan came out with its ports in one order and
//! its tracks in another: dense-td's `n16` had six edges stepping right
//! whose tracks nested and whose ports did not, each stub crossing the
//! tracks of the edges beyond it (4 to 8 crossings under one box).
//!
//! Here, last of the path stages, every such side is combed: its free
//! teeth (see [`Tooth`]) between two ends this stage leaves as they are
//! take the side's ports in the order of their turns, and the tracks of
//! the teeth stepping one way are dealt out again so the tooth turning
//! furthest out runs nearest the side. Only ports and tracks the side
//! already holds are handed out, so the comb keeps its even port band
//! (the ports stage's [`crate::ports::PORT_EVEN_MAX`] spread) and the
//! gap keeps its track pitch. A side's new comb is kept only when its
//! edges cross fewer lines than before and hit no more boxes, frame
//! borders, title chips or labels, nor run on top of another edge.
//!
//! The edges stay separate strokes (each its own port and track, as the
//! ports stage spaces them): merged onto one trunk, a fan's edges could
//! not be told apart, picked or styled one by one, and a dotted or thick
//! tooth would lend its look to the others.
//!
//! The same rule runs in [`crate::scene::scene`], [`crate::scene::route`]
//! and every drag frame: it is a few sorts per fan and a crossing count
//! over the edges it changes. In [`crate::scene::route_partial`] an end a
//! drag keeps ([`Ctx::held`]) is left where it is, like any other end
//! this stage does not move, and the free teeth on either side of it
//! comb among themselves.

use crate::edgepass::{end_rect, normal, side_of, Ctx, EndRef, Side};
use crate::clearance::{CLEAR_GAP, TIGHT_RUN_MIN};
use crate::router::{ortho_corners, smooth, CORNER_R};
use crate::scene::{edge_polyline, Scene, SceneEdge};

type Pt = (f64, f64);

/// Fewest ends on one side that make it a fan (report B4).
const FAN_MIN: usize = 3;
/// Float tolerance (px).
const EPS: f64 = 1e-6;
/// How far (px) the ports of one straight side may lie off one line.
const SIDE_SLACK: f64 = 0.5;
/// Least straight run (px) out of a bare end, or on past a turn into
/// the rest of the path, before the next corner: half a rounding.
const BARE_LEG: f64 = CORNER_R / 2.0;

/// One end on a fanned side, in the side's own coordinates: `t` along
/// the side (growing with screen x on a top or bottom side, screen y on
/// a left or right one), `n` straight out of it.
struct Tooth {
    edge: usize,
    /// The end is its edge's start.
    src: bool,
    /// Port, along the side.
    t: f64,
    /// The path from this end as corners (side coordinates), when this
    /// stage may comb it: a stub straight out to a track `depth` out,
    /// across it to the `turn`, and on outwards from there (at least a
    /// corner further out than the track).
    path: Option<Vec<Pt>>,
    depth: f64,
    turn: f64,
}

/// The two axes of a side: outward normal and tangent (both unit,
/// along the screen axes).
#[derive(Clone, Copy, PartialEq)]
struct Frame {
    nv: Pt,
    tv: Pt,
}

impl Frame {
    /// The frame of `side`.
    fn of(side: Side) -> Frame {
        let nv = normal(side);
        let tv = match side {
            Side::Top | Side::Bottom => (1.0, 0.0),
            Side::Left | Side::Right => (0.0, 1.0),
        };
        Frame { nv, tv }
    }

    /// A screen point as (along the side, out of it).
    fn local(self, p: Pt) -> Pt {
        (
            p.0 * self.tv.0 + p.1 * self.tv.1,
            p.0 * self.nv.0 + p.1 * self.nv.1,
        )
    }

    /// Inverse of [`Frame::local`].
    fn screen(self, q: Pt) -> Pt {
        (
            q.0 * self.tv.0 + q.1 * self.nv.0,
            q.0 * self.tv.1 + q.1 * self.nv.1,
        )
    }
}

/// Whether the stages may move edge `i`'s ends here: a flowchart edge
/// on the axes that no drag keeps, that is no back-edge's lane (planned
/// round the nodes it passes), no labelled twin's bulge, no authored
/// `curve:` and no note card's link.
fn free(sc: &Scene, cx: &Ctx, i: usize) -> bool {
    let e = &sc.edges[i];
    let flag = |v: &[bool]| v.get(i).copied().unwrap_or(false);
    flag(&cx.drawn)
        && !flag(&cx.fixed)
        && !flag(&cx.held)
        && !flag(&cx.back)
        && !flag(&cx.shaped)
        && e.paint_width.is_none()
        && e.meta.curve.is_none()
        && !crate::ports::note_link(sc, cx, i)
}

/// Whether an end of `e` (its start when `src`) carries a glyph, which
/// needs a head's straight run square to its side.
fn glyph_at(e: &SceneEdge, src: bool) -> bool {
    if src {
        e.ends.start_marker(e.kind).is_some()
    } else {
        e.ends.end_marker(e.kind).is_some()
    }
}

/// The comb a tooth's path takes from its end, checked: a stub, a track
/// and a turn outwards, as (depth, turn). `c` in side coordinates from
/// the end.
fn shape(c: &[Pt]) -> Option<(f64, f64)> {
    if c.len() < 4 {
        return None;
    }
    let (t0, n0) = c[0];
    let depth = c[1].1 - n0;
    let turn = c[2].0;
    ((c[1].0 - t0).abs() < EPS
        && depth > EPS
        && (c[2].1 - c[1].1).abs() < EPS
        && (turn - t0).abs() > EPS
        && (c[3].0 - turn).abs() < EPS
        && c[3].1 - c[2].1 > EPS)
        .then_some((depth, turn))
}

/// Comb every fanned side of every box (see the module docs). Sets
/// `moved[i]` for each edge it redraws, so the label stage looks at
/// their labels again.
pub(crate) fn comb(sc: &mut Scene, cx: &Ctx, moved: &mut [bool]) {
    if sc.edges.len() > crate::router::ROUTE_MAX_EDGES {
        return;
    }
    // Every drawn end by (box, side).
    let mut groups: Vec<((EndRef, Side), Vec<Tooth>)> = Vec::new();
    let mut any = false;
    for i in 0..sc.edges.len() {
        let Some((from, to)) = cx.ends.get(i).copied().flatten() else {
            continue;
        };
        if from == to || !cx.drawn.get(i).copied().unwrap_or(false) {
            continue;
        }
        let e = &sc.edges[i];
        let corners = ortho_corners(e).filter(|_| free(sc, cx, i));
        let (p0, p3) = if e.waypoints.len() >= 2 {
            (e.waypoints[0], e.waypoints[e.waypoints.len() - 1])
        } else {
            (e.bezier[0], e.bezier[3])
        };
        for (src, end, p) in [(true, from, p0), (false, to, p3)] {
            let r = end_rect(sc, end);
            let side = side_of(r, p);
            let fr = Frame::of(side);
            // The box side's own line, out of the box.
            let line_n = fr.local((r.0, r.1)).1.max(fr.local((r.2, r.3)).1);
            let path = corners.as_ref().and_then(|c| {
                let from_end: Vec<Pt> = if src {
                    c.clone()
                } else {
                    c.iter().rev().copied().collect()
                };
                let local: Vec<Pt> = from_end.iter().map(|&q| fr.local(q)).collect();
                // (A port off the side's line — a shape's outline inside
                // its box — is left where it is.)
                ((local[0].1 - line_n).abs() < SIDE_SLACK)
                    .then(|| shape(&local).map(|s| (local, s)))
                    .flatten()
            });
            any |= path.is_some();
            let t = fr.local(p).0;
            let (path, (depth, turn)) = match path {
                Some((l, s)) => (Some(l), s),
                None => (None, (0.0, t)),
            };
            let tooth = Tooth {
                edge: i,
                src,
                t,
                path,
                depth,
                turn,
            };
            match groups.iter_mut().find(|g| g.0 == (end, side)) {
                Some(g) => g.1.push(tooth),
                None => groups.push(((end, side), vec![tooth])),
            }
        }
    }
    if !any {
        return;
    }
    // The drawn lines, for judging a side's new comb.
    let mut lines: Vec<Vec<Pt>> = sc.edges.iter().map(line_of).collect();
    for ((end, side), mut teeth) in groups {
        if teeth.len() < FAN_MIN || !straight_side(sc, end) {
            continue;
        }
        let fr = Frame::of(side);
        teeth.sort_by(|a, b| a.t.total_cmp(&b.t).then(a.edge.cmp(&b.edge)));
        // Runs of combable teeth between the ends this stage leaves.
        let mut runs: Vec<Vec<usize>> = Vec::new();
        let mut cur: Vec<usize> = Vec::new();
        for (k, tooth) in teeth.iter().enumerate() {
            if tooth.path.is_some() {
                cur.push(k);
            } else if !cur.is_empty() {
                runs.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            runs.push(cur);
        }
        for run in runs.into_iter().filter(|r| r.len() >= 2) {
            let Some(plan) = plan_run(sc, &teeth, &run, fr) else {
                continue;
            };
            if plan.is_empty() {
                continue;
            }
            if keeps(sc, cx, &lines, &plan) {
                for r in plan {
                    let e = &mut sc.edges[r.edge];
                    let c = &r.corners;
                    let m = c.len();
                    if m == 2 {
                        // A straight drop: the straight cubic the later
                        // stages make of a two-point path.
                        let (a, b) = (c[0], c[1]);
                        e.bezier = [
                            a,
                            (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0),
                            (a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0),
                            b,
                        ];
                        e.waypoints = Vec::new();
                    } else {
                        // As the ortho stage draws a path: the chord through
                        // its first and last corner, its corners rounded.
                        e.bezier = [c[0], c[1], c[m - 2], c[m - 1]];
                        e.waypoints = smooth(c, CORNER_R);
                    }
                    if let (Some(l), Some(at)) = (e.label.as_mut(), r.label) {
                        l.1 = at;
                    }
                    lines[r.edge] = r.corners;
                    moved[r.edge] = true;
                }
            }
        }
    }
}

/// Whether `end`'s sides are straight lines its ports may be dealt
/// along: a box (no diamond's or round shape's, no fork bar's, whose
/// branches meet it where they lie) or a frame. (A port off its side's
/// line, on a shape's outline inside its box, is never combed.)
fn straight_side(sc: &Scene, end: EndRef) -> bool {
    use crate::model::Shape;
    match end {
        EndRef::Node(v) => !matches!(
            sc.nodes[v].shape,
            Shape::Diamond
                | Shape::Circle
                | Shape::DoubleCircle
                | Shape::StateStart
                | Shape::StateEnd
                | Shape::ForkBar
        ),
        EndRef::Frame(_) => true,
    }
}

/// An edge's line as drawn, by its corners where it runs on the axes.
fn line_of(e: &SceneEdge) -> Vec<Pt> {
    match ortho_corners(e) {
        Some(c) => c,
        None => edge_polyline(e),
    }
}

/// One redrawn edge of a combed run.
struct Redraw {
    edge: usize,
    /// Its new path's corners (screen), from its start.
    corners: Vec<Pt>,
    /// Its label's new spot, when the label rode a moved leg.
    label: Option<Pt>,
}

/// The comb of one run of combable teeth (`run`, indices into the
/// side's `teeth` in port order): the redrawn edges, empty when the run
/// is combed already, `None` when its new comb would leave a stub or a
/// leg on past a turn too short.
fn plan_run(sc: &Scene, teeth: &[Tooth], run: &[usize], fr: Frame) -> Option<Vec<Redraw>> {
    // The ports, low to high, go to the teeth in the order of their turns.
    let ports: Vec<f64> = run.iter().map(|&k| teeth[k].t).collect();
    let mut by_turn: Vec<usize> = run.to_vec();
    by_turn.sort_by(|&a, &b| {
        teeth[a]
            .turn
            .total_cmp(&teeth[b].turn)
            .then(teeth[a].t.total_cmp(&teeth[b].t))
            .then(teeth[a].edge.cmp(&teeth[b].edge))
    });
    // Each tooth's new port.
    let mut port = vec![0.0; teeth.len()];
    for (j, &k) in by_turn.iter().enumerate() {
        port[k] = ports[j];
    }
    // The tracks: the teeth stepping low (their turn below their port)
    // nest shallowest-first from the lowest turn, those stepping high
    // from the highest; a tooth whose turn is its new port drops
    // straight and frees its track. The run's tracks are dealt out again
    // in that order, the two ways merged by the depth each tooth held, so
    // a tooth keeps its place among the others where the nest allows.
    let low: Vec<usize> = by_turn
        .iter()
        .copied()
        .filter(|&k| teeth[k].turn < port[k] - EPS)
        .collect();
    let high: Vec<usize> = by_turn
        .iter()
        .rev()
        .copied()
        .filter(|&k| teeth[k].turn > port[k] + EPS)
        .collect();
    let mut pool: Vec<f64> = low
        .iter()
        .chain(&high)
        .map(|&k| teeth[k].depth)
        .collect();
    pool.sort_by(f64::total_cmp);
    let mut depth = vec![0.0; teeth.len()];
    let (mut a, mut b) = (0, 0);
    for &d in &pool {
        let take_low = match (low.get(a), high.get(b)) {
            (Some(&x), Some(&y)) => teeth[x].depth <= teeth[y].depth,
            (Some(_), None) => true,
            _ => false,
        };
        if take_low {
            depth[low[a]] = d;
            a += 1;
        } else {
            depth[high[b]] = d;
            b += 1;
        }
    }
    let changed = run.iter().any(|&k| {
        let tooth = &teeth[k];
        (port[k] - tooth.t).abs() > EPS
            || ((tooth.turn - port[k]).abs() > EPS && (depth[k] - tooth.depth).abs() > EPS)
    });
    if !changed {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    for &k in run {
        let tooth = &teeth[k];
        let path = tooth.path.as_ref().expect("combable");
        let straight = (tooth.turn - port[k]).abs() <= EPS;
        if (port[k] - tooth.t).abs() <= EPS && (straight || (depth[k] - tooth.depth).abs() <= EPS)
        {
            continue;
        }
        let e = &sc.edges[tooth.edge];
        let n0 = path[0].1;
        let d = depth[k];
        // A glyph's straight run square to its side, a bare end's half a
        // rounding; on past the track, a glyph at a far end the same.
        let need = |glyph: bool| {
            if glyph {
                crate::ortho::HEAD_LEG
            } else {
                BARE_LEG
            }
        };
        let far_glyph = path.len() == 4 && glyph_at(e, !tooth.src);
        if !straight && d < need(glyph_at(e, tooth.src)) - EPS {
            return None;
        }
        if path[3].1 - (n0 + d) < need(far_glyph) - EPS {
            return None;
        }
        let mut local: Vec<Pt> = vec![(port[k], n0)];
        if !straight {
            local.push((port[k], n0 + d));
            local.push((tooth.turn, n0 + d));
        }
        local.extend_from_slice(&path[3..]);
        let mut corners: Vec<Pt> = local.iter().map(|&q| fr.screen(q)).collect();
        // Drop a corner a straight drop left on its line.
        corners = collapse(corners);
        if !tooth.src {
            corners.reverse();
        }
        // The label rides the same leg of the new path.
        let label = e
            .label
            .as_ref()
            .and_then(|(_, at, _)| relabel(path, &local, fr, *at, straight));
        out.push(Redraw {
            edge: tooth.edge,
            corners,
            label,
        });
    }
    Some(out)
}

/// Drop repeated points and points in the middle of a straight leg.
fn collapse(pts: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out
            .last()
            .is_some_and(|&q: &Pt| (q.0 - p.0).abs() < EPS && (q.1 - p.1).abs() < EPS)
        {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let on_x = (a.0 - b.0).abs() < EPS && (b.0 - p.0).abs() < EPS;
            let on_y = (a.1 - b.1).abs() < EPS && (b.1 - p.1).abs() < EPS;
            if on_x || on_y {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

/// The spot of a label at `at` (screen) on the leg of the new comb
/// `new` (side coordinates) that matches the leg of `old` it rides: the
/// stub, the track or the run on past the turn, its place along the
/// leg kept where the new leg reaches it. `None`: it rides no moved leg.
fn relabel(old: &[Pt], new: &[Pt], fr: Frame, at: Pt, straight: bool) -> Option<Pt> {
    let q = fr.local(at);
    let on = |a: Pt, b: Pt| {
        let (lo_t, hi_t) = (a.0.min(b.0), a.0.max(b.0));
        let (lo_n, hi_n) = (a.1.min(b.1), a.1.max(b.1));
        q.0 >= lo_t - 1.0 && q.0 <= hi_t + 1.0 && q.1 >= lo_n - 1.0 && q.1 <= hi_n + 1.0
    };
    let leg = (0..3).find(|&l| on(old[l], old[l + 1]))?;
    // The new path's matching leg: a straight drop's one leg stands for
    // all three.
    let (a, b) = if straight {
        (new[0], new[1])
    } else {
        (new[leg], new[leg + 1])
    };
    let clamp = |v: f64, x: f64, y: f64| v.max(x.min(y)).min(x.max(y));
    let spot = if (a.0 - b.0).abs() < EPS {
        (a.0, clamp(q.1, a.1, b.1))
    } else {
        (clamp(q.0, a.0, b.0), a.1)
    };
    Some(fr.screen(spot))
}

/// Whether the redrawn edges `plan` cross fewer lines than they do now
/// and hit nothing more: boxes (but their own), frame borders, title
/// chips and other edges' labels, nor run on top of another edge.
fn keeps(sc: &Scene, cx: &Ctx, lines: &[Vec<Pt>], plan: &[Redraw]) -> bool {
    let changed: Vec<usize> = plan.iter().map(|p| p.edge).collect();
    let new_line = |i: usize| -> Vec<Pt> {
        plan.iter()
            .find(|p| p.edge == i)
            .map_or_else(|| lines[i].clone(), |p| p.corners.clone())
    };
    let now: Vec<Vec<Pt>> = (0..lines.len()).map(new_line).collect();
    let crossings = |ls: &[Vec<Pt>]| {
        let mut n = 0;
        for (k, &i) in changed.iter().enumerate() {
            for j in 0..ls.len() {
                if j == i || changed[..k].contains(&j) || !cx.drawn.get(j).copied().unwrap_or(false)
                {
                    continue;
                }
                n += cross_count(&ls[i], &ls[j]);
            }
        }
        n
    };
    if crossings(&now) >= crossings(lines) {
        return false;
    }
    let overlaps = |ls: &[Vec<Pt>]| {
        let mut n = 0;
        for (k, &i) in changed.iter().enumerate() {
            for j in 0..ls.len() {
                if j == i || changed[..k].contains(&j) || !cx.drawn.get(j).copied().unwrap_or(false)
                {
                    continue;
                }
                n += overlap_count(&ls[i], &ls[j]);
            }
        }
        n
    };
    if overlaps(&now) > overlaps(lines) {
        return false;
    }
    // Boxes, frame borders, title chips and the other edges' labels.
    let heads: Vec<(f64, f64, f64, f64)> = if sc.clusters.is_empty() {
        Vec::new()
    } else {
        (0..sc.clusters.len())
            .map(|ci| crate::scene::title_head(&sc.clusters, ci, f64::INFINITY))
            .collect()
    };
    let hits = |ls: &[Vec<Pt>], after: bool| {
        let mut n = 0;
        for &i in &changed {
            let own = cx.ends.get(i).copied().flatten();
            let is_own = |v: usize| {
                own.is_some_and(|(a, b)| a == EndRef::Node(v) || b == EndRef::Node(v))
            };
            for w in ls[i].windows(2) {
                for (v, nd) in sc.nodes.iter().enumerate() {
                    if is_own(v) {
                        continue;
                    }
                    let r = (
                        nd.x - nd.w / 2.0 + 1.0,
                        nd.y - nd.h / 2.0 + 1.0,
                        nd.x + nd.w / 2.0 - 1.0,
                        nd.y + nd.h / 2.0 - 1.0,
                    );
                    n += usize::from(seg_hits_rect(w[0], w[1], r));
                }
                for c in &sc.clusters {
                    let r = (c.x, c.y, c.x + c.w, c.y + c.h);
                    n += border_crossings(w[0], w[1], r);
                }
                for &h in &heads {
                    n += usize::from(seg_hits_rect(w[0], w[1], h));
                }
            }
        }
        // Labels over lines: a changed line through any label, any line
        // through a changed edge's label (moved with its leg).
        for (j, e) in sc.edges.iter().enumerate() {
            let Some((_, c, lw)) = &e.label else {
                continue;
            };
            let r = label_box(sc, j, *c, *lw, after.then_some(plan));
            for (i, l) in ls.iter().enumerate() {
                if i == j
                    || !(changed.contains(&i) || changed.contains(&j))
                    || !cx.drawn.get(i).copied().unwrap_or(false)
                {
                    continue;
                }
                n += l
                    .windows(2)
                    .filter(|w| seg_hits_rect(w[0], w[1], r))
                    .count();
            }
        }
        n
    };
    if hits(&now, true) > hits(lines, false) {
        return false;
    }
    // Nor do the changed lines run closer beside another than the
    // spacing clear ([`CLEAR_GAP`]) for longer than they did.
    // (Pairs over [`TIGHT_RUN_MIN`], as the quality yardstick counts
    // them, and their length.)
    let tight = |ls: &[Vec<Pt>]| {
        let (mut pairs, mut len) = (0, 0.0);
        for (k, &i) in changed.iter().enumerate() {
            for j in 0..ls.len() {
                if j == i || changed[..k].contains(&j) || !cx.drawn.get(j).copied().unwrap_or(false)
                {
                    continue;
                }
                let l = beside(&ls[i], &ls[j]);
                pairs += usize::from(l > TIGHT_RUN_MIN);
                len += l;
            }
        }
        (pairs, len)
    };
    let (was, now) = (tight(lines), tight(&now));
    now.0 <= was.0 && now.1 <= was.1 + 1.0
}

/// How long (px) legs of `a` run beside legs of `b` closer than
/// [`CLEAR_GAP`] (on top of them included).
fn beside(a: &[Pt], b: &[Pt]) -> f64 {
    let mut len = 0.0;
    for u in a.windows(2) {
        for v in b.windows(2) {
            let (lu, lv) = (axis_leg(u[0], u[1]), axis_leg(v[0], v[1]));
            if let (Some((au, cu, u0, u1)), Some((av, cv, v0, v1))) = (lu, lv) {
                if au == av && (cu - cv).abs() < CLEAR_GAP {
                    len += (u1.min(v1) - u0.max(v0)).max(0.0);
                }
            }
        }
    }
    len
}

/// An axis leg as (vertical?, its line, span low, span high).
fn axis_leg(p: Pt, q: Pt) -> Option<(bool, f64, f64, f64)> {
    if (p.0 - q.0).abs() < EPS && (p.1 - q.1).abs() > EPS {
        Some((true, p.0, p.1.min(q.1), p.1.max(q.1)))
    } else if (p.1 - q.1).abs() < EPS && (p.0 - q.0).abs() > EPS {
        Some((false, p.1, p.0.min(q.0), p.0.max(q.0)))
    } else {
        None
    }
}

/// The box of edge `j`'s label (centred at `c`, `w` wide), or where the
/// comb `plan` (if given) moves it.
fn label_box(
    sc: &Scene,
    j: usize,
    c: Pt,
    w: f64,
    plan: Option<&[Redraw]>,
) -> (f64, f64, f64, f64) {
    let c = plan
        .and_then(|plan| plan.iter().find(|p| p.edge == j))
        .and_then(|p| p.label)
        .unwrap_or(c);
    let text = sc.edges[j].label.as_ref().map_or("", |l| l.0.as_str());
    let lines = 1 + text.matches("<br").count() + text.matches('\n').count();
    let h = crate::layout::LABEL_BOX_H / 2.0 + crate::layout::LINE_H * (lines - 1) as f64 / 2.0;
    (c.0 - w / 2.0, c.1 - h, c.0 + w / 2.0, c.1 + h)
}

/// Proper crossings of two polylines.
fn cross_count(a: &[Pt], b: &[Pt]) -> usize {
    let mut n = 0;
    for u in a.windows(2) {
        for v in b.windows(2) {
            n += usize::from(segs_cross(u[0], u[1], v[0], v[1]));
        }
    }
    n
}

/// Whether segments `a`-`b` and `c`-`d` properly cross.
fn segs_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let o = |p: Pt, q: Pt, r: Pt| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2) = (o(c, d, a), o(c, d, b));
    let (d3, d4) = (o(a, b, c), o(a, b, d));
    d1 * d2 < -EPS && d3 * d4 < -EPS
}

/// Legs of `a` running on top of legs of `b` (on one axis line within a
/// px, overlapping by more than a px).
fn overlap_count(a: &[Pt], b: &[Pt]) -> usize {
    let mut n = 0;
    for u in a.windows(2) {
        for v in b.windows(2) {
            let horiz = |p: Pt, q: Pt| (p.1 - q.1).abs() < EPS && (p.0 - q.0).abs() > EPS;
            let vert = |p: Pt, q: Pt| (p.0 - q.0).abs() < EPS && (p.1 - q.1).abs() > EPS;
            let over = |a0: f64, a1: f64, b0: f64, b1: f64| {
                a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1)) > 1.0
            };
            if horiz(u[0], u[1]) && horiz(v[0], v[1]) && (u[0].1 - v[0].1).abs() < 1.0 {
                n += usize::from(over(u[0].0, u[1].0, v[0].0, v[1].0));
            } else if vert(u[0], u[1]) && vert(v[0], v[1]) && (u[0].0 - v[0].0).abs() < 1.0 {
                n += usize::from(over(u[0].1, u[1].1, v[0].1, v[1].1));
            }
        }
    }
    n
}

/// Whether segment `a`-`b` meets the rect `(x0, y0, x1, y1)`.
fn seg_hits_rect(a: Pt, b: Pt, r: (f64, f64, f64, f64)) -> bool {
    if r.2 <= r.0 || r.3 <= r.1 {
        return false;
    }
    // Liang-Barsky clip.
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - r.0),
        (dx, r.2 - a.0),
        (-dy, a.1 - r.1),
        (dy, r.3 - a.1),
    ] {
        if p.abs() < EPS {
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

/// How many times segment `a`-`b` crosses the border of rect `r`.
fn border_crossings(a: Pt, b: Pt, r: (f64, f64, f64, f64)) -> usize {
    let corners = [(r.0, r.1), (r.2, r.1), (r.2, r.3), (r.0, r.3)];
    (0..4)
        .filter(|&k| segs_cross(a, b, corners[k], corners[(k + 1) % 4]))
        .count()
}
