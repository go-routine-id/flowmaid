//! Crossing casing (G1): where two edges must cross, the one going
//! UNDER gets a small gap cut out of its stroke on both sides of the
//! other, so the crossing reads as one line passing beneath another
//! instead of a blot.
//!
//! The gap is an SVG `<mask>` per under-edge (a white box with a black
//! disc at each crossing), so it shows on frames and on a transparent
//! canvas alike, where a background-coloured halo would not. A diagram
//! without crossings gets no masks and renders byte-identically.
//!
//! Which edge goes under is chosen per crossing, so that no edge
//! collects a run of closely spaced gaps: a stroke cut every few dozen
//! px reads as dashed, and a dashed edge means `-.->` in mermaid.

use crate::labels::seg_cells;
use crate::model::EdgeKind;
use crate::scene::{
    edge_drawn, edge_polyline, DefsKey, Scene, SceneEdge, ARROW_MARKER_PX, EDGE_WIDTH,
    EDGE_WIDTH_THICK,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Clear air the under-edge keeps on each side of the over-edge's
/// stroke at a crossing, in under-edge stroke widths: a little more
/// than its own stroke, so at 1:1 the gap still reads as the line
/// passing beneath, while staying short enough (about 7.7 px on a
/// default edge) that a few of them on one line do not read as dashes.
pub(crate) const CASING_GAP_STROKES: f64 = 1.25;
/// Background halo (px) added to that air: the band of canvas (or frame
/// fill) the gap shows round the over-edge's stroke.
pub(crate) const CASING_HALO: f64 = 1.5;
/// Two crossing points of one edge pair closer than this (px) are one
/// crossing (sampled polylines can hit twice).
const CROSS_MERGE: f64 = 4.0;
/// Side (px) of a cell of the segment grid that finds candidate pairs.
const CASING_CELL: f64 = 64.0;
/// Room (px) the mask box leaves around the under-edge's points, so
/// its stroke and its end markers are never clipped by the box itself.
const MASK_PAD: f64 = 2.0 * ARROW_MARKER_PX;
/// Above this many edges no crossing gets a gap: finding the crossings
/// grows with the square of the edges sharing a grid cell, and a
/// drawing that large has so many crossings that the masks would
/// double the SVG without making any single one easier to follow.
pub(crate) const CASING_MAX_EDGES: usize = 400;
/// Two gaps on one edge closer than this (px) along it chain up, and a
/// chain of gaps reads as a dashed stroke: the under-edge choice keeps
/// each edge's chains as short as it can. The quality yardstick's own
/// spacing, so what the choice avoids is what it counts.
const GAP_PERIOD: f64 = crate::quality::DASH_PERIOD;
/// Prefix of the mask ids when the diagram has no dom id of its own.
const MASK_ID_PREFIX: &str = "flowmaid-";

/// What [`masks`] adds to an SVG: `defs` goes inside `<defs>`, and
/// `attr[i]` is appended to edge `i`'s `<path>` (empty = unchanged).
pub(crate) struct Masks {
    pub defs: String,
    pub attr: Vec<String>,
}

type Pt = (f64, f64);
/// The grid cells one segment is bucketed into.
type Cells = Vec<(i64, i64)>;

/// Stroke width `to_svg` paints edge `e` with.
fn stroke_width(e: &SceneEdge) -> f64 {
    let base = match e.kind {
        EdgeKind::Thick | EdgeKind::ThickOpen => EDGE_WIDTH_THICK,
        _ => EDGE_WIDTH,
    };
    e.meta.style.stroke_width.unwrap_or(base)
}

/// Proper intersection point of segments `a`–`b` and `c`–`d`, half-open
/// on both (the same rule as the quality yardstick).
fn seg_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    let r = (b.0 - a.0, b.1 - a.1);
    let s = (d.0 - c.0, d.1 - c.1);
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-9 {
        return None;
    }
    let t = ((c.0 - a.0) * s.1 - (c.1 - a.1) * s.0) / den;
    let u = ((c.0 - a.0) * r.1 - (c.1 - a.1) * r.0) / den;
    ((0.0..1.0).contains(&t) && (0.0..1.0).contains(&u)).then_some((a.0 + t * r.0, a.1 + t * r.1))
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Every forced crossing of the scene's drawn edges that gets a gap, as
/// (under-edge index, over-edge index, point, notch) in scene
/// coordinates, ordered by edge pair and then along the first edge. A
/// crossing within an arrowhead's length of an edge end is left alone:
/// a gap there would eat into the head. `notch` marks a crossing cut
/// only by the over-edge's halo, and a crossing on a line already cut
/// [`CUTS_PER_WINDOW`] times within [`CUT_WINDOW`] is drawn plain, with
/// no cut, and left out (see [`cut_kinds`]). The quality yardstick reads
/// this too, to count the cuts each edge collects (G1).
pub(crate) fn crossings(sc: &Scene) -> Vec<(usize, usize, Pt, bool)> {
    crossings_with(sc, true)
        .into_iter()
        .filter(|x| x.3 != Cut::Plain)
        .map(|(u, o, p, c)| (u, o, p, c == Cut::Notch))
        .collect()
}

/// How a crossing is drawn on its under-line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cut {
    /// The full gap: the line passes beneath (G1).
    Gap,
    /// The over-edge's halo alone ([`CASING_HALO`] each side of its
    /// stroke, under half the full gap).
    Notch,
    /// No cut: one stroke simply crosses the other.
    Plain,
}

/// Span (px) along a line over which [`CUTS_PER_WINDOW`] cuts are the
/// most it takes: the quality yardstick's dash window, so what the
/// casing avoids is what it counts.
const CUT_WINDOW: f64 = crate::quality::DASH_WINDOW;
/// Most cuts (gaps or notches) one line takes within [`CUT_WINDOW`]:
/// one under a dashed look ([`crate::quality::DASH_RUN`]).
const CUTS_PER_WINDOW: usize = crate::quality::DASH_RUN - 1;

/// How each crossing is cut into its under-line (G1: never dashed),
/// walking each line's crossings in order. A crossing on a line already
/// cut [`CUTS_PER_WINDOW`] times within the last [`CUT_WINDOW`] is drawn
/// plain: any cut there, even a notch, makes the line read as `-.->`,
/// while a plain crossing still reads as two lines crossing. Of the rest,
/// one whose full gap would make [`DASH_RUN`](crate::quality::DASH_RUN)
/// gaps in a row, each closer than [`GAP_PERIOD`] to the last, is a
/// notch, and the run goes on from the last full gap. The under-edge
/// choice has already moved every gap it could to the other line; what
/// is left are lines crossed by more edges than either line can hold
/// sparse gaps for. `gaps` lists, per crossing, its under-line and where
/// it falls along it.
fn cut_kinds(gaps: &[(usize, f64)]) -> Vec<Cut> {
    let mut per: BTreeMap<usize, Vec<(f64, usize)>> = BTreeMap::new();
    for (c, &(line, s)) in gaps.iter().enumerate() {
        per.entry(line).or_default().push((s, c));
    }
    let mut kind = vec![Cut::Gap; gaps.len()];
    for along in per.values_mut() {
        along.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let (mut last, mut run) = (f64::NEG_INFINITY, 0usize);
        let mut cuts: Vec<f64> = Vec::new();
        for &(s, c) in along.iter() {
            if cuts.iter().filter(|&&q| s - q < CUT_WINDOW).count() >= CUTS_PER_WINDOW {
                kind[c] = Cut::Plain;
                continue;
            }
            cuts.push(s);
            let next = if s - last < GAP_PERIOD { run + 1 } else { 1 };
            if is_dash(next) {
                kind[c] = Cut::Notch;
            } else {
                (last, run) = (s, next);
            }
        }
    }
    kind
}

/// [`crossings`], optionally keeping those next to an edge end too (the
/// set the quality yardstick counts as `edge_edge_x`).
fn crossings_with(sc: &Scene, skip_ends: bool) -> Vec<(usize, usize, Pt, Cut)> {
    let idx: Vec<usize> = (0..sc.edges.len())
        .filter(|&i| {
            let e = &sc.edges[i];
            edge_drawn(e) && e.from != e.to
        })
        .collect();
    if idx.len() < 2 {
        return Vec::new();
    }
    let lines: Vec<Vec<Pt>> = idx.iter().map(|&i| edge_polyline(&sc.edges[i])).collect();

    // Segment grid: cell -> (line, segment). A segment too long to
    // bucket (a node dragged far away) is checked against every other
    // segment instead (`big`), so the grid stays linear in the segments.
    let mut cells: BTreeMap<(i64, i64), Vec<(usize, usize)>> = BTreeMap::new();
    let mut big: Vec<(usize, usize)> = Vec::new();
    let mut seg_cells_of: Vec<Vec<Option<Cells>>> = Vec::with_capacity(lines.len());
    for (k, pts) in lines.iter().enumerate() {
        let mut per = Vec::with_capacity(pts.len());
        for (s, w) in pts.windows(2).enumerate() {
            let cs = seg_cells(w[0], w[1], CASING_CELL);
            match &cs {
                Some(cs) => {
                    for &c in cs {
                        cells.entry(c).or_default().push((k, s));
                    }
                }
                None => big.push((k, s)),
            }
            per.push(cs);
        }
        seg_cells_of.push(per);
    }

    let back = back_edges(&lines);
    let length: Vec<f64> = lines
        .iter()
        .map(|p| p.windows(2).map(|w| dist(w[0], w[1])).sum())
        .collect();

    // Arc length (px) of each line up to the start of each segment, so a
    // crossing's position along both lines is known where it is found.
    let starts: Vec<Vec<f64>> = lines
        .iter()
        .map(|p| {
            let mut acc = 0.0;
            p.windows(2)
                .map(|w| {
                    let at = acc;
                    acc += dist(w[0], w[1]);
                    at
                })
                .collect()
        })
        .collect();

    // Hits per line pair, found segment by segment of the lower line in
    // order (so merges are deterministic), with where each falls along
    // the lower and the higher line.
    let mut hits: BTreeMap<(usize, usize), Vec<Hit>> = BTreeMap::new();
    for (ka, pa) in lines.iter().enumerate() {
        for (sa, wa) in pa.windows(2).enumerate() {
            let mut cand: Vec<(usize, usize)> =
                big.iter().copied().filter(|&(kb, _)| kb > ka).collect();
            match &seg_cells_of[ka][sa] {
                Some(cs) => {
                    for c in cs {
                        if let Some(v) = cells.get(c) {
                            cand.extend(v.iter().filter(|&&(kb, _)| kb > ka));
                        }
                    }
                }
                // Too long to bucket: every segment of the later lines.
                None => {
                    for (kb, pb) in lines.iter().enumerate().skip(ka + 1) {
                        cand.extend((0..pb.len().saturating_sub(1)).map(|sb| (kb, sb)));
                    }
                }
            }
            cand.sort_unstable();
            cand.dedup();
            for (kb, sb) in cand {
                let pb = &lines[kb];
                let Some(p) = seg_cross(wa[0], wa[1], pb[sb], pb[sb + 1]) else {
                    continue;
                };
                // Counted up to an end the two share (the yardstick's
                // rule): ends crossing on their port legs cross too.
                let near_end = skip_ends
                    && [pa[0], pa[pa.len() - 1], pb[0], pb[pb.len() - 1]]
                        .iter()
                        .any(|&q| dist(p, q) < ARROW_MARKER_PX);
                if near_end {
                    continue;
                }
                let v = hits.entry((ka, kb)).or_default();
                if v.iter().all(|h| dist(p, h.p) >= CROSS_MERGE) {
                    v.push(Hit {
                        p,
                        at: [
                            starts[ka][sa] + dist(wa[0], p),
                            starts[kb][sb] + dist(pb[sb], p),
                        ],
                    });
                }
            }
        }
    }

    let mut a_under = choose_under(&hits, &back, &length).into_iter();
    let mut out = Vec::new();
    let mut gaps = Vec::new();
    for (&(ka, kb), v) in &hits {
        for h in v {
            let (u, o, s) = if a_under.next() == Some(true) {
                (ka, kb, h.at[0])
            } else {
                (kb, ka, h.at[1])
            };
            out.push((idx[u], idx[o], h.p, Cut::Gap));
            gaps.push((u, s));
        }
    }
    for (x, kind) in out.iter_mut().zip(cut_kinds(&gaps)) {
        x.3 = kind;
    }
    out
}

/// One crossing of a line pair `(a, b)`, `a < b`: the point, and how
/// far along `a` and along `b` (px) it falls.
struct Hit {
    p: Pt,
    at: [f64; 2],
}

/// Gaps of one edge in a row, each closer than [`GAP_PERIOD`] to the
/// next, that a new gap at `s` would join (itself included). `gaps` is
/// sorted.
fn chain_with(gaps: &[f64], s: f64) -> usize {
    let pos = gaps.partition_point(|&g| g < s);
    let mut n = 1;
    let mut cur = s;
    for &g in gaps[..pos].iter().rev() {
        if cur - g >= GAP_PERIOD {
            break;
        }
        n += 1;
        cur = g;
    }
    cur = s;
    for &g in &gaps[pos..] {
        if g - cur >= GAP_PERIOD {
            break;
        }
        n += 1;
        cur = g;
    }
    n
}

/// Length of the chain the gap `gaps[i]` belongs to (`gaps` sorted).
fn chain_at(gaps: &[f64], i: usize) -> usize {
    let left = (1..=i)
        .rev()
        .take_while(|&k| gaps[k] - gaps[k - 1] < GAP_PERIOD)
        .count();
    let right = (i + 1..gaps.len())
        .take_while(|&k| gaps[k] - gaps[k - 1] < GAP_PERIOD)
        .count();
    1 + left + right
}

/// Gaps of one edge within [`GAP_PERIOD`] of `s` along it (`gaps` is
/// sorted).
fn local(gaps: &[f64], s: f64) -> usize {
    gaps.partition_point(|&g| g < s + GAP_PERIOD) - gaps.partition_point(|&g| g <= s - GAP_PERIOD)
}

/// Which line of each crossing goes under, for every hit of `hits` in
/// order: `true` when the lower line of its pair takes the gap.
///
/// A greedy pass ([`Assign::greedy`]) that keeps every line's gaps
/// sparse, then a bounded local search ([`Assign::repair`]) that moves
/// gaps out of chains that read as dashed, then kicks
/// ([`Assign::kick`]) for lines still dashed. If a line is still dashed
/// and budget is left, the whole run starts over from the other greedy
/// order and the cheaper result wins.
fn choose_under(
    hits: &BTreeMap<(usize, usize), Vec<Hit>>,
    back: &[bool],
    length: &[f64],
) -> Vec<bool> {
    // Crossings numbered in order: line pair, then hit.
    let pair: Vec<[usize; 2]> = hits
        .iter()
        .flat_map(|(&(a, b), v)| v.iter().map(move |_| [a, b]))
        .collect();
    let at: Vec<[f64; 2]> = hits.values().flatten().map(|h| h.at).collect();
    let mut members = vec![Vec::new(); back.len()];
    for (c, p) in pair.iter().enumerate() {
        members[p[0]].push(c);
        members[p[1]].push(c);
    }
    let ctx = Ctx {
        pair: &pair,
        at: &at,
        members: &members,
        back,
        length,
    };
    let budget = (REPAIR_BUDGET_PER_CROSSING * pair.len()).max(REPAIR_BUDGET_MIN);
    let mut best = Assign::solve(&ctx, true, budget);
    if best.dashed.iter().any(|&d| d > 0) && best.budget > 0 {
        let other = Assign::solve(&ctx, false, best.budget);
        if other.cost() < best.cost() {
            best = other;
        }
    }
    best.under_a
}

/// The crossings [`choose_under`] decides, numbered `0..n`.
struct Ctx<'a> {
    /// The two lines of each crossing, lower first.
    pair: &'a [[usize; 2]],
    /// Where each crossing falls along its lower and its higher line.
    at: &'a [[f64; 2]],
    /// The crossings of each line.
    members: &'a [Vec<usize>],
    /// Which lines run against the flow.
    back: &'a [bool],
    /// Length of each line (px).
    length: &'a [f64],
}

impl Ctx<'_> {
    /// Where crossing `c` falls along line `e` (one of its two).
    fn at_on(&self, c: usize, e: usize) -> f64 {
        self.at[c][usize::from(e != self.pair[c][0])]
    }
}

/// Under-edge choices in progress: the choice per crossing and the gaps
/// (sorted, px along the line) each line has so far.
struct Assign<'a> {
    ctx: &'a Ctx<'a>,
    /// Whether the lower line of crossing `c` takes its gap.
    under_a: Vec<bool>,
    gaps: Vec<Vec<f64>>,
    /// The crossing each gap of `gaps` belongs to, in the same order.
    ids: Vec<Vec<usize>>,
    /// Chains of each line long enough to read as dashed.
    dashed: Vec<usize>,
    /// Summed [`chain_cost`] change of every flip so far.
    score: i64,
    /// The flips since a kick began, so the kick can be undone.
    journal: Vec<usize>,
    /// Flips the search may still try: it bounds the time on a drawing
    /// with very many crossings close together, and being a count (not
    /// a clock) keeps the result deterministic.
    budget: usize,
}

impl<'a> Assign<'a> {
    /// Greedy, repair and kicks from one greedy order (see
    /// [`Self::greedy`]), within `budget` flips.
    fn solve(ctx: &'a Ctx<'a>, tight_first: bool, budget: usize) -> Self {
        let lines = ctx.members.len();
        let mut st = Assign {
            ctx,
            under_a: vec![false; ctx.pair.len()],
            gaps: vec![Vec::new(); lines],
            ids: vec![Vec::new(); lines],
            dashed: vec![0; lines],
            score: 0,
            journal: Vec::new(),
            budget,
        };
        st.greedy(tight_first);
        st.repair((0..ctx.pair.len()).collect());
        st.journal.clear();
        for line in 0..lines {
            if st.dashed[line] > 0 {
                st.kick(line);
            }
        }
        st.balance();
        st
    }

    /// Decides every crossing once, keeping each line's gaps sparse.
    ///
    /// With `tight_first`, crossings that sit in the densest runs of
    /// crossings on both their lines go first, and a gap goes to a line
    /// where it can never complete a dash when the other line is not
    /// such a line. Otherwise the crossings of the most crossed lines
    /// go first. Then by line pair and position along the lower line.
    /// Each goes to the line where it adds least to [`greedy_cost`],
    /// then where it joins the shorter chain (see [`chain_with`]), then
    /// where fewer gaps sit near it. Ties: a back-edge goes under a
    /// forward one, then the line with fewer gaps so far, then the
    /// shorter line (with `tight_first`; the longer one otherwise, so
    /// the two orders explore different choices), then the later line.
    fn greedy(&mut self, tight_first: bool) {
        let ctx = self.ctx;
        let (pair, at) = (ctx.pair, ctx.at);
        // How many crossings each crossing's run along each of its lines
        // holds (crossings each closer than a gap period to the next):
        // in a run shorter than a dash, no choice can dash that line.
        let mut run = vec![[0usize; 2]; pair.len()];
        for (e, cs) in ctx.members.iter().enumerate() {
            let mut along: Vec<(f64, usize)> = cs.iter().map(|&c| (ctx.at_on(c, e), c)).collect();
            along.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
            let mut start = 0;
            for i in 1..=along.len() {
                if i == along.len() || along[i].0 - along[i - 1].0 >= GAP_PERIOD {
                    for &(_, c) in &along[start..i] {
                        run[c][usize::from(e != pair[c][0])] = i - start;
                    }
                    start = i;
                }
            }
        }
        let crossed: Vec<usize> = ctx.members.iter().map(Vec::len).collect();
        let busy = |c: usize| crossed[pair[c][0]].max(crossed[pair[c][1]]);
        let tight = |c: usize| {
            if tight_first {
                run[c][0].min(run[c][1])
            } else {
                0
            }
        };
        let mut order: Vec<usize> = (0..pair.len()).collect();
        order.sort_by(|&x, &y| {
            tight(y)
                .cmp(&tight(x))
                .then(busy(y).cmp(&busy(x)))
                .then(pair[x].cmp(&pair[y]))
                .then(at[x][0].total_cmp(&at[y][0]))
                .then(x.cmp(&y))
        });
        let can_dash = |c: usize, side: usize| tight_first && is_dash(run[c][side]);
        let grow = |g: &[f64], x: f64| {
            let mut h = g.to_vec();
            add_gap(&mut h, x);
            greedy_cost(&h) - greedy_cost(g)
        };
        for c in order {
            let [a, b] = pair[c];
            let (ga, gb) = (&self.gaps[a], &self.gaps[b]);
            let (la, lb) = (ctx.length[a], ctx.length[b]);
            let pick = can_dash(c, 0)
                .cmp(&can_dash(c, 1))
                .then(grow(ga, at[c][0]).cmp(&grow(gb, at[c][1])))
                .then(chain_with(ga, at[c][0]).cmp(&chain_with(gb, at[c][1])))
                .then(local(ga, at[c][0]).cmp(&local(gb, at[c][1])))
                // `true` sorts after `false`: the back-edge is the lesser.
                .then(ctx.back[b].cmp(&ctx.back[a]))
                .then(ga.len().cmp(&gb.len()))
                .then(if tight_first {
                    la.total_cmp(&lb)
                } else {
                    lb.total_cmp(&la)
                })
                // The later line goes under: `b` is the later.
                .then(Ordering::Greater);
            self.under_a[c] = pick == Ordering::Less;
            let side = usize::from(!self.under_a[c]);
            self.put(pair[c][side], at[c][side], c);
        }
        for (d, g) in self.dashed.iter_mut().zip(&self.gaps) {
            *d = runs(g).into_iter().filter(|&n| is_dash(n)).count();
        }
    }

    /// A line left dashed sits in a local minimum the repair cannot
    /// leave a few gaps at a time. Hand every other gap of its chains to
    /// the other lines at once, or else one chained gap at a time,
    /// repair round them, and keep the first result that makes the
    /// whole drawing cheaper.
    fn kick(&mut self, line: usize) {
        let ctx = self.ctx;
        let mut mine: Vec<usize> = ctx.members[line]
            .iter()
            .copied()
            .filter(|&c| self.under_line(c) == line)
            .collect();
        mine.sort_by(|&x, &y| {
            ctx.at_on(x, line)
                .total_cmp(&ctx.at_on(y, line))
                .then(x.cmp(&y))
        });
        let near = |i: usize, j: usize| {
            (ctx.at_on(mine[i], line) - ctx.at_on(mine[j], line)).abs() < GAP_PERIOD
        };
        let chained: Vec<usize> = (0..mine.len())
            .filter(|&i| (i > 0 && near(i, i - 1)) || (i + 1 < mine.len() && near(i, i + 1)))
            .map(|i| mine[i])
            .collect();
        let every_other: Vec<usize> = chained.iter().copied().skip(1).step_by(2).collect();
        let kicks = std::iter::once(every_other).chain(chained.iter().map(|&c| vec![c]));
        for kick in kicks {
            if self.budget == 0 {
                return;
            }
            let (before, mark) = (self.score, self.journal.len());
            let mut seed = BTreeSet::new();
            for &c in &kick {
                self.flip(c);
                for e in ctx.pair[c] {
                    seed.extend(ctx.members[e].iter().copied());
                }
            }
            self.repair(seed);
            if self.score < before {
                self.journal.clear();
                return;
            }
            // Undo every flip since the kick, newest first.
            while self.journal.len() > mark {
                if let Some(c) = self.journal.pop() {
                    self.flip_quiet(c);
                }
            }
        }
        self.journal.clear();
    }

    /// Evens out how many gaps the lines hold: a gap moves from the line
    /// with the most gaps to its other line when that line would still
    /// hold fewer and no chain gets longer than a dash allows (the move
    /// does not raise [`chain_cost`]). A back-edge keeps its gaps (it
    /// goes under by preference). The busiest lines go first, and a line
    /// is done once none of its gaps can move.
    fn balance(&mut self) {
        let ctx = self.ctx;
        let mut lines: Vec<usize> = (0..ctx.members.len()).collect();
        lines.sort_by(|&x, &y| self.gaps[y].len().cmp(&self.gaps[x].len()).then(x.cmp(&y)));
        for line in lines {
            for i in 0..ctx.members[line].len() {
                let c = ctx.members[line][i];
                if self.under_line(c) != line {
                    continue;
                }
                let other = ctx.pair[c][usize::from(ctx.pair[c][0] == line)];
                if self.gaps[other].len() + 1 >= self.gaps[line].len()
                    || (ctx.back[line] && !ctx.back[other])
                {
                    continue;
                }
                if self.flip_quiet(c) > 0 {
                    self.flip_quiet(c);
                }
            }
        }
    }

    /// Adds crossing `c`'s gap at `s` to `line`.
    fn put(&mut self, line: usize, s: f64, c: usize) {
        let pos = self.gaps[line].partition_point(|&g| g < s);
        self.gaps[line].insert(pos, s);
        self.ids[line].insert(pos, c);
    }

    /// Removes crossing `c`'s gap at `s` from `line`.
    fn take(&mut self, line: usize, s: f64, c: usize) {
        let from = self.gaps[line].partition_point(|&g| g < s);
        if let Some(i) = (from..self.ids[line].len()).find(|&i| self.ids[line][i] == c) {
            self.gaps[line].remove(i);
            self.ids[line].remove(i);
        }
    }

    /// The crossings whose gaps sit in a dashed chain of `line`.
    fn dashed_on(&self, line: usize) -> Vec<usize> {
        let (g, ids) = (&self.gaps[line], &self.ids[line]);
        let mut out = Vec::new();
        let mut start = 0;
        for i in 1..=g.len() {
            if i == g.len() || g[i] - g[i - 1] >= GAP_PERIOD {
                if is_dash(i - start) {
                    out.extend_from_slice(&ids[start..i]);
                }
                start = i;
            }
        }
        out
    }

    /// The line that takes crossing `c`'s gap.
    fn under_line(&self, c: usize) -> usize {
        self.ctx.pair[c][usize::from(!self.under_a[c])]
    }

    /// Summed [`chain_cost`] of every line.
    fn cost(&self) -> u64 {
        self.gaps.iter().map(|g| chain_cost(g)).sum()
    }

    /// Hands crossing `c`'s gap to its other line, whatever it costs,
    /// and returns how much that changed the summed [`chain_cost`]
    /// (worked out round the two spots alone, not over whole lines).
    fn flip(&mut self, c: usize) -> i64 {
        self.journal.push(c);
        self.flip_quiet(c)
    }

    /// [`Self::flip`] without recording it in the journal.
    fn flip_quiet(&mut self, c: usize) -> i64 {
        let from = usize::from(!self.under_a[c]);
        let (p, a) = (self.ctx.pair[c], self.ctx.at[c]);
        self.take(p[from], a[from], c);
        let (out_runs, out_dashed) = insert_delta(&self.gaps[p[from]], a[from]);
        let (in_runs, in_dashed) = insert_delta(&self.gaps[p[1 - from]], a[1 - from]);
        self.put(p[1 - from], a[1 - from], c);
        self.under_a[c] = !self.under_a[c];
        let mut delta = in_runs - out_runs;
        for (line, d) in [(p[from], -out_dashed), (p[1 - from], in_dashed)] {
            let was = self.dashed[line] > 0;
            self.dashed[line] = self.dashed[line].wrapping_add_signed(d);
            let now = self.dashed[line] > 0;
            delta += (i64::from(now) - i64::from(was)) * DASHED_LINE_COST as i64;
        }
        self.score += delta;
        delta
    }

    /// Whether crossing `c`'s gap on line `e` sits in a dashed chain.
    fn in_dash(&self, c: usize, e: usize) -> bool {
        if self.dashed[e] == 0 {
            return false;
        }
        let g = &self.gaps[e];
        is_dash(chain_at(
            g,
            g.partition_point(|&x| x < self.ctx.at_on(c, e)),
        ))
    }

    /// Local search from the crossings in `todo`: the gap of a crossing
    /// that sits in a dashed chain goes to the other line, alone or as
    /// an ejection chain (see [`Self::eject`]), whenever that lowers the
    /// summed [`chain_cost`]. After a move every crossing of the
    /// lines it touched is looked at again. Crossings are taken in
    /// order, so the result depends on nothing but the geometry.
    fn repair(&mut self, mut todo: BTreeSet<usize>) {
        while let Some(c) = todo.pop_first() {
            if self.budget == 0 {
                return;
            }
            // Only a gap in a dashed chain is worth moving: anywhere else
            // taking it away saves nothing (see [`run_cost`]).
            if !self.in_dash(c, self.under_line(c)) {
                continue;
            }
            let mut chain = Vec::new();
            if self.eject(c, 0, &mut chain) {
                for &f in &chain {
                    for e in self.ctx.pair[f] {
                        todo.extend(self.ctx.members[e].iter().copied());
                    }
                }
            }
        }
    }

    /// Hands crossing `c`'s gap to its other line and, while that alone
    /// does not pay off, passes on in turn a gap from a dashed chain of
    /// a line the chain handed a gap to (an ejection chain, depth-first,
    /// up to [`EJECT_DEPTH`] flips). `delta` is what the flips so far changed the cost by.
    /// Keeps the first chain that lowers the cost, with its crossings in
    /// `chain`, and says so; otherwise undoes every flip.
    fn eject(&mut self, c: usize, delta: i64, chain: &mut Vec<usize>) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        let delta = delta + self.flip(c);
        chain.push(c);
        if delta < 0 {
            return true;
        }
        if chain.len() < EJECT_DEPTH {
            // Pass on any gap left in a dashed chain on a line this
            // chain of flips handed a gap to, newest first. The list is
            // taken up front: a deeper chain that fails leaves every
            // choice as it was.
            for j in (0..chain.len()).rev() {
                let to = self.under_line(chain[j]);
                if self.dashed[to] == 0 {
                    continue;
                }
                for c2 in self.dashed_on(to) {
                    if !chain.contains(&c2) && self.eject(c2, delta, chain) {
                        return true;
                    }
                }
            }
        }
        chain.pop();
        self.flip(c);
        false
    }
}

/// Moves the repair may try, per crossing of the drawing. A move is
/// kept only when it strictly lowers the cost, so the search ends on
/// its own; this only caps its time on a drawing with very many
/// crossings close together (and makes that cap independent of timing).
///
/// `to_svg` runs on every drag frame, so this is kept low: on a drawing
/// whose bands cross more densely than any choice can keep sparse, the
/// kicks spend whatever budget is left for little gain (on the densest
/// quality fixture, 12 per crossing bought 2 fewer dashed lines of 10
/// for about 2 ms more per frame; 3 adds well under 1 ms to the frame).
const REPAIR_BUDGET_PER_CROSSING: usize = 3;
/// Least flips the search may try, so a small or medium drawing still
/// gets a thorough search (a thousand flips take a fraction of a
/// millisecond).
const REPAIR_BUDGET_MIN: usize = 1000;
/// Most flips in one repair move: a gap, plus the gaps it pushes on
/// down the lines it lands on.
const EJECT_DEPTH: usize = 2;
/// Cost of one line whose gaps chain into a dashed look: it outweighs
/// any number of shorter chains, so the search first makes fewer lines
/// read as dashed and only then shortens chains.
const DASHED_LINE_COST: u64 = 1 << 20;

/// Whether a chain of `n` gaps reads as dashed (the yardstick's run).
fn is_dash(n: usize) -> bool {
    n >= crate::quality::DASH_RUN
}

/// Cost of one chain of `n` gaps: 0 while it does not read as dashed,
/// then growing with the square of its length, so the search shortens
/// a dashed chain even where it cannot yet break it.
fn run_cost(n: usize) -> i64 {
    if is_dash(n) {
        let k = (n + 1 - crate::quality::DASH_RUN) as i64;
        k * k
    } else {
        0
    }
}

/// The chains of gaps (sorted) as lengths, in order.
fn runs(gaps: &[f64]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    let mut prev: Option<f64> = None;
    for &g in gaps {
        match (prev, out.last_mut()) {
            (Some(p), Some(n)) if g - p < GAP_PERIOD => *n += 1,
            _ => out.push(1),
        }
        prev = Some(g);
    }
    out
}

/// What the local search minimises on one line: [`DASHED_LINE_COST`]
/// if any chain reads as dashed, plus each chain's [`run_cost`].
fn chain_cost(gaps: &[f64]) -> u64 {
    let r = runs(gaps);
    let dashed = r.iter().any(|&n| is_dash(n));
    r.iter().map(|&n| run_cost(n) as u64).sum::<u64>() + if dashed { DASHED_LINE_COST } else { 0 }
}

/// What the greedy weighs adding a gap by: [`chain_cost`]'s dashed
/// flag, the square of every chain beyond its first gap (two short
/// chains beat one long one, even below a dash), and the square of the
/// line's gap count (so no line collects most of the gaps).
fn greedy_cost(gaps: &[f64]) -> u64 {
    let r = runs(gaps);
    let dashed = r.iter().any(|&n| is_dash(n));
    let n = gaps.len() as u64;
    r.iter().map(|&k| ((k - 1) * (k - 1)) as u64).sum::<u64>()
        + n * n
        + if dashed { DASHED_LINE_COST } else { 0 }
}

/// How inserting a gap at `s` into the sorted `gaps` changes the summed
/// [`run_cost`] of its chains and the number of its dashed chains:
/// only the chains on either side of `s` can merge.
fn insert_delta(gaps: &[f64], s: f64) -> (i64, isize) {
    let pos = gaps.partition_point(|&g| g < s);
    let chain_left = |mut i: usize| {
        let mut n = 1;
        while i > 0 && gaps[i] - gaps[i - 1] < GAP_PERIOD {
            n += 1;
            i -= 1;
        }
        n
    };
    let chain_right = |mut i: usize| {
        let mut n = 1;
        while i + 1 < gaps.len() && gaps[i + 1] - gaps[i] < GAP_PERIOD {
            n += 1;
            i += 1;
        }
        n
    };
    let left = (pos > 0 && s - gaps[pos - 1] < GAP_PERIOD).then(|| chain_left(pos - 1));
    let right = (pos < gaps.len() && gaps[pos] - s < GAP_PERIOD).then(|| chain_right(pos));
    let (a, b) = match (left, right) {
        // `s` falls inside one chain: it only grows by one.
        (Some(l), Some(r)) if gaps[pos] - gaps[pos - 1] < GAP_PERIOD => (l + r, 0),
        (l, r) => (l.unwrap_or(0), r.unwrap_or(0)),
    };
    let after = a + b + 1;
    let cost = run_cost(after) - run_cost(a) - run_cost(b);
    let dashed = isize::from(is_dash(after)) - isize::from(is_dash(a)) - isize::from(is_dash(b));
    (cost, dashed)
}

/// Inserts a gap at `s` into the sorted `gaps`.
fn add_gap(gaps: &mut Vec<f64>, s: f64) {
    let pos = gaps.partition_point(|&g| g < s);
    gaps.insert(pos, s);
}

/// Which lines run against the flow (back-edges, which go under). The
/// scene carries no flow direction, so it is read off the lines: a
/// flowchart edge leaves its source along the flow, so the flow axis
/// and sign are the ones most lines' first step agrees on. That is a
/// count of lines, and of where they head off rather than where they
/// land, so a wide fan-out across the flow never outvotes it. A tie
/// goes to top-down, mermaid's default.
fn back_edges(lines: &[Vec<Pt>]) -> Vec<bool> {
    let delta = |p: &Vec<Pt>| (p[p.len() - 1].0 - p[0].0, p[p.len() - 1].1 - p[0].1);
    let first_step = |p: &Vec<Pt>| {
        p.iter()
            .map(|q| (q.0 - p[0].0, q.1 - p[0].1))
            .find(|d| d.0 != 0.0 || d.1 != 0.0)
            .unwrap_or((0.0, 0.0))
    };
    let (mut xs, mut ys) = (0i64, 0i64);
    for d in lines.iter().map(first_step) {
        // Each line votes on its dominant axis only.
        if d.0.abs() > d.1.abs() {
            xs += if d.0 > 0.0 { 1 } else { -1 };
        } else if d.1 != 0.0 {
            ys += if d.1 > 0.0 { 1 } else { -1 };
        }
    }
    let (on_x, sign) = if xs.abs() > ys.abs() {
        (true, xs.signum() as f64)
    } else {
        (false, if ys < 0 { -1.0 } else { 1.0 })
    };
    lines
        .iter()
        .map(|p| {
            let d = delta(p);
            (if on_x { d.0 } else { d.1 }) * sign < 0.0
        })
        .collect()
}

/// Masks cutting a gap into every under-edge at each of its crossings.
/// `t` maps scene to SVG coordinates (as `to_svg` draws); ids are
/// `{dom}-x{edge}`, `dom` being the diagram's dom id or, when it has
/// none, a hash of the scene's geometry, so several SVGs on one page
/// never share an id unless they draw the same thing.
pub(crate) fn masks(sc: &Scene, t: &dyn Fn(Pt) -> Pt, dom_id: Option<&str>) -> Masks {
    let mut attr = vec![String::new(); sc.edges.len()];
    let xs = if sc.edges.len() > CASING_MAX_EDGES {
        Vec::new()
    } else {
        crossings(sc)
    };
    if xs.is_empty() {
        return Masks {
            defs: String::new(),
            attr,
        };
    }
    let dom = match dom_id {
        Some(d) => d.to_string(),
        None => format!("{MASK_ID_PREFIX}{}", scene_hash(sc)),
    };
    let mut per: BTreeMap<usize, Vec<(Pt, f64)>> = BTreeMap::new();
    for (u, o, p, notch) in xs {
        let air = if notch {
            0.0
        } else {
            CASING_GAP_STROKES * stroke_width(&sc.edges[u])
        };
        let r = stroke_width(&sc.edges[o]) / 2.0 + air + CASING_HALO;
        per.entry(u).or_default().push((t(p), r));
    }
    let mut defs = String::new();
    for (u, holes) in per {
        let pts = edge_polyline(&sc.edges[u]);
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for p in pts.iter().map(|&p| t(p)) {
            x0 = x0.min(p.0);
            y0 = y0.min(p.1);
            x1 = x1.max(p.0);
            y1 = y1.max(p.1);
        }
        let (x, y) = (x0 - MASK_PAD, y0 - MASK_PAD);
        let (w, h) = (x1 - x0 + 2.0 * MASK_PAD, y1 - y0 + 2.0 * MASK_PAD);
        let id = format!("{dom}-x{u}");
        defs.push_str(&format!(
            "<mask id=\"{id}\" maskUnits=\"userSpaceOnUse\" x=\"{x:.1}\" y=\"{y:.1}\" \
             width=\"{w:.1}\" height=\"{h:.1}\"><rect x=\"{x:.1}\" y=\"{y:.1}\" \
             width=\"{w:.1}\" height=\"{h:.1}\" fill=\"#fff\"/>"
        ));
        for ((cx, cy), r) in holes {
            defs.push_str(&format!(
                "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.2}\" fill=\"#000\"/>"
            ));
        }
        defs.push_str("</mask>");
        attr[u] = format!(" mask=\"url(#{id})\"");
    }
    Masks { defs, attr }
}

/// Stable hash of what the scene draws (ids and edge geometry).
fn scene_hash(sc: &Scene) -> String {
    let mut key = DefsKey::new();
    for n in &sc.nodes {
        key.eat(n.id.as_bytes());
        key.eat(&[0]);
        key.eat_f64(n.x);
        key.eat_f64(n.y);
    }
    for e in &sc.edges {
        key.eat(e.from.as_bytes());
        key.eat(&[0]);
        key.eat(e.to.as_bytes());
        key.eat(&[0]);
        for &(x, y) in e.bezier.iter().chain(&e.waypoints) {
            key.eat_f64(x);
            key.eat_f64(y);
        }
    }
    key.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::scene::{route, scene, to_svg};

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    /// Least share (numerator, denominator) of the fixtures' crossings
    /// of a back-edge with a forward edge that put the back-edge under.
    /// Measured: 142 of 248 (57%). A layout change that moves crossings
    /// can shift this; re-measure before loosening it.
    const BACK_UNDER_SHARE: (usize, usize) = (1, 2);

    /// A line crossed every 20 px: never more than [`CUTS_PER_WINDOW`]
    /// cuts (gaps or notches) within any [`CUT_WINDOW`]; the rest are
    /// drawn plain, and no full gap chains into a dash (G1).
    #[test]
    fn a_densely_crossed_line_keeps_its_cuts_sparse() {
        let gaps: Vec<(usize, f64)> = (0..30).map(|k| (0, 20.0 * k as f64)).collect();
        let kinds = cut_kinds(&gaps);
        let cut: Vec<f64> = gaps
            .iter()
            .zip(&kinds)
            .filter(|(_, k)| **k != Cut::Plain)
            .map(|(g, _)| g.1)
            .collect();
        assert!(cut.len() > 1 && kinds.contains(&Cut::Plain));
        for (i, &s) in cut.iter().enumerate() {
            let within = cut[i..].iter().filter(|&&t| t - s < CUT_WINDOW).count();
            assert!(within <= CUTS_PER_WINDOW, "{within} cuts from {s}");
        }
        // A sparse line keeps every full gap.
        let sparse: Vec<(usize, f64)> = (0..5).map(|k| (1, 150.0 * k as f64)).collect();
        assert!(cut_kinds(&sparse).iter().all(|k| *k == Cut::Gap));
    }

    fn fixtures() -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = std::fs::read_dir(FIXTURES)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .map(|p| {
                let name = p.file_stem().unwrap().to_string_lossy().to_string();
                (name, std::fs::read_to_string(&p).unwrap())
            })
            .collect();
        v.sort();
        v
    }

    /// Brute force over every segment pair of every edge pair, with the
    /// same skip and merge rules, as the reference for the grid search.
    fn brute(sc: &Scene) -> usize {
        let idx: Vec<usize> = (0..sc.edges.len())
            .filter(|&i| edge_drawn(&sc.edges[i]) && sc.edges[i].from != sc.edges[i].to)
            .collect();
        let mut total = 0;
        for x in 0..idx.len() {
            for y in x + 1..idx.len() {
                let (ea, eb) = (&sc.edges[idx[x]], &sc.edges[idx[y]]);
                let (a, b) = (edge_polyline(ea), edge_polyline(eb));
                let mut hits: Vec<Pt> = Vec::new();
                for sa in a.windows(2) {
                    for sb in b.windows(2) {
                        let Some(p) = seg_cross(sa[0], sa[1], sb[0], sb[1]) else {
                            continue;
                        };
                        if hits.iter().all(|&q| dist(p, q) >= CROSS_MERGE) {
                            hits.push(p);
                        }
                    }
                }
                total += hits.len();
            }
        }
        total
    }

    #[test]
    fn crossings_match_the_yardstick_and_each_mask_has_one_owner() {
        for (name, src) in fixtures() {
            let g = parse(&src).unwrap();
            let sc = scene(&g);
            let all = crossings_with(&sc, false).len();
            assert_eq!(all, brute(&sc), "{name}: grid search vs brute force");
            // The yardstick skips pairs whose bounding boxes it finds
            // disjoint, which misses an exactly level or plumb edge.
            let m = crate::quality::measure(&g, &sc);
            assert!(all >= m.edge_edge_x, "{name}: {all} < {}", m.edge_edge_x);
            let svg = to_svg(&sc);
            let holes = svg.matches("fill=\"#000\"/>").count();
            assert_eq!(holes, crossings(&sc).len(), "{name}: one hole per crossing");
            // Every mask is defined once and used by exactly one path.
            for def in svg.split("<mask id=\"").skip(1) {
                let id = &def[..def.find('"').unwrap()];
                assert_eq!(
                    svg.matches(&format!("url(#{id})")).count(),
                    1,
                    "{name}: {id}"
                );
            }
        }
    }

    #[test]
    fn crossing_free_diagrams_get_no_mask() {
        let g = parse("flowchart TD\nA-->B\nA-->C\nB-->D\nC-->D").unwrap();
        let sc = scene(&g);
        let m = masks(&sc, &|p| p, None);
        assert!(m.defs.is_empty() && m.attr.iter().all(String::is_empty));
        assert!(!to_svg(&sc).contains("<mask"));
    }

    #[test]
    fn back_edges_mostly_go_under_and_ids_are_stable() {
        // Judged against the graph's own flow direction (the pipeline
        // context), not the direction `crossings` reads off the lines.
        // A back-edge going under is a tie-break: spreading the gaps so
        // no line reads dashed comes first, so it holds for most mixed
        // crossings, not all.
        let (mut mixed, mut back_under) = (0, 0);
        for (_, src) in fixtures() {
            let g = parse(&src).unwrap();
            let sc = scene(&g);
            let centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            for s in [sc.clone(), route(&g, &centers)] {
                let back = crate::edgepass::ctx(&g, &s).back;
                for (u, o, _, _) in crossings(&s) {
                    if back[u] == back[o] {
                        continue;
                    }
                    mixed += 1;
                    back_under += usize::from(back[u]);
                }
            }
        }
        assert!(
            mixed > 0,
            "no fixture crosses a back-edge with a forward one"
        );
        assert!(
            back_under * BACK_UNDER_SHARE.1 >= mixed * BACK_UNDER_SHARE.0,
            "back-edge under at {back_under} of {mixed} mixed crossings"
        );

        let g =
            parse("flowchart LR\nA-->B\nB-->C\nC-->D\nX-->Y\nY-->Z\nD-->B\nX-->C\nA-->Z").unwrap();
        let sc = scene(&g);
        assert!(!crossings(&sc).is_empty());
        // Here every choice is free, so the back-edge always goes under.
        let back = crate::edgepass::ctx(&g, &sc).back;
        for (u, o, _, _) in crossings(&sc) {
            assert!(back[u] || !back[o], "back-edge {o} went over edge {u}");
        }
        let a = to_svg(&sc);
        assert_eq!(a, to_svg(&scene(&g)), "deterministic");
        assert!(a.contains("<mask id=\"flowmaid-"), "hash-prefixed id");
        // route() output carries masks too, deterministically.
        let centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = to_svg(&route(&g, &centers));
        assert!(r.contains("<mask id=\"flowmaid-"));
        assert_eq!(r, to_svg(&route(&g, &centers)));
    }

    /// Crossings of lines `0..lines` given as (lower line, higher line,
    /// px along the lower, px along the higher), in key order.
    fn hits_of(spec: &[(usize, usize, f64, f64)]) -> BTreeMap<(usize, usize), Vec<Hit>> {
        let mut hits: BTreeMap<(usize, usize), Vec<Hit>> = BTreeMap::new();
        for &(a, b, sa, sb) in spec {
            hits.entry((a, b)).or_default().push(Hit {
                p: (sa, sb),
                at: [sa, sb],
            });
        }
        hits
    }

    /// Gap positions per line for the choice `under` over `hits`.
    fn gaps_of(
        hits: &BTreeMap<(usize, usize), Vec<Hit>>,
        under: &[bool],
        lines: usize,
    ) -> Vec<Vec<f64>> {
        let mut gaps = vec![Vec::new(); lines];
        for ((&(a, b), h), &a_under) in hits
            .iter()
            .flat_map(|(k, v)| v.iter().map(move |h| (k, h)))
            .zip(under)
        {
            let (line, s) = if a_under { (a, h.at[0]) } else { (b, h.at[1]) };
            add_gap(&mut gaps[line], s);
        }
        gaps
    }

    #[test]
    fn insert_delta_matches_a_recount() {
        // Every subset of a few close positions, and every spot between
        // and around them: the local delta equals a full recount.
        let spots = [0.0, 25.0, 50.0, 70.0, 140.0, 150.0];
        let probes = [-30.0, 10.0, 40.0, 60.0, 100.0, 145.0, 200.0];
        for mask in 0u32..(1 << spots.len()) {
            let gaps: Vec<f64> = (0..spots.len())
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| spots[i])
                .collect();
            let dashes = |g: &[f64]| runs(g).into_iter().filter(|&n| is_dash(n)).count() as isize;
            let sum = |g: &[f64]| runs(g).into_iter().map(run_cost).sum::<i64>();
            for &s in &probes {
                let mut after = gaps.clone();
                add_gap(&mut after, s);
                let (cost, dashed) = insert_delta(&gaps, s);
                assert_eq!(cost, sum(&after) - sum(&gaps), "{gaps:?} + {s}");
                assert_eq!(dashed, dashes(&after) - dashes(&gaps), "{gaps:?} + {s}");
            }
        }
    }

    #[test]
    fn a_long_line_crossed_by_a_comb_stays_whole() {
        // Line 0 is crossed by ten short lines every 20 px: each short
        // line takes its one gap, the long one none (it would read as
        // `-.->` with ten gaps 20 px apart).
        let spec: Vec<(usize, usize, f64, f64)> =
            (1..=10).map(|k| (0, k, 20.0 * k as f64, 50.0)).collect();
        let hits = hits_of(&spec);
        let mut length = vec![100.0; 11];
        length[0] = 1000.0;
        let under = choose_under(&hits, &[false; 11], &length);
        assert!(under.iter().all(|&a| !a), "{under:?}");
    }

    #[test]
    fn a_dense_grid_leaves_no_line_dashed_and_is_deterministic() {
        // Six lines across six others, 40 px apart both ways: every line
        // has room for two gaps in three, so none needs a dashed chain.
        // Always giving the gap to one side would dash all six.
        const N: usize = 6;
        let mut spec = Vec::new();
        for h in 0..N {
            for v in 0..N {
                spec.push((h, N + v, 40.0 * v as f64, 40.0 * h as f64));
            }
        }
        let hits = hits_of(&spec);
        let length = [400.0; 2 * N];
        let under = choose_under(&hits, &[false; 2 * N], &length);
        let gaps = gaps_of(&hits, &under, 2 * N);
        for (line, g) in gaps.iter().enumerate() {
            assert!(runs(g).iter().all(|&n| !is_dash(n)), "line {line}: {g:?}");
        }
        assert_eq!(under, choose_under(&hits, &[false; 2 * N], &length));
    }

    #[test]
    fn a_wide_top_down_fan_out_does_not_flip_the_flow() {
        // Eight edges leave downward but land far to the right (their
        // summed width outweighs their height), one bows back up: only
        // that one runs against the flow.
        let mut lines: Vec<Vec<Pt>> = (1..=8)
            .map(|k| {
                let x = k as f64 * 200.0;
                vec![(0.0, 0.0), (0.0, 30.0), (x, 70.0), (x, 100.0)]
            })
            .collect();
        lines.push(vec![
            (800.0, 100.0),
            (700.0, 60.0),
            (100.0, 40.0),
            (0.0, 0.0),
        ]);
        let back = back_edges(&lines);
        assert_eq!(back.iter().filter(|b| **b).count(), 1, "{back:?}");
        assert!(back[lines.len() - 1]);
    }

    #[test]
    fn a_node_dragged_far_away_routes_fast() {
        // Segments millions of px long used to be bucketed into every
        // cell of their bounding box: quadratic time and memory.
        let src = "flowchart LR\nA-->|x|B\nC-->|y|D\nA-->D\nC-->B\nB-->A\nA-->A\nA-->B\nA==>B";
        let g = parse(src).unwrap();
        let sc = scene(&g);
        for far in [1e6, 1e9] {
            let mut centers: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            centers[1] = (far, far);
            let t = std::time::Instant::now();
            let svg = to_svg(&route(&g, &centers));
            let took = t.elapsed();
            assert!(!svg.is_empty());
            assert!(took.as_secs_f64() < 2.0, "far={far:e}: {took:?}");
        }
    }
}
