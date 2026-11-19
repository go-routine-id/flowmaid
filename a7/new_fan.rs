    /// Sweep `i`'s children round it, clockwise in source order: child
    /// `j` first tries direction `first[j]` (or one step past its
    /// predecessor, when that is later), then steps [`DEG_STEP`] at a
    /// time to the first direction where its whole subtree stays clear
    /// of `i` and of every child already placed. `None` once a child
    /// would pass `first[0]` plus `limit`. Every candidate is tested with
    /// the child's exact subtree for that direction, so the result is
    /// clean by construction.
    fn sweep(
        &mut self,
        i: usize,
        kids: &[usize],
        first: &[i32],
        limit: i32,
        gap: f64,
        inbound: Option<Seg>,
    ) -> Option<Vec<(i32, At)>> {
        let start = first[0];
        let mut out: Vec<(i32, At)> = Vec::with_capacity(kids.len());
        for (j, &c) in kids.iter().enumerate() {
            let mut t = out
                .last()
                .map_or(start, |&(p, _)| first[j].max(p + DEG_STEP));
            loop {
                if t - start > limit || self.spent() {
                    return None;
                }
                // Cheap necessary test first: the child's own box and
                // spoke against the siblings already placed. Most blocked
                // directions fail here, before the child's subtree has to
                // be laid out for them.
                if !self.own_box_clear(i, c, t, gap, &out) {
                    t += DEG_STEP;
                    continue;
                }
                if let Some(a) = self.put(i, c, t, gap) {
                    if self.fits_among(i, a, &out, usize::MAX, inbound) {
                        out.push((t, a));
                        break;
                    }
                }
                t += DEG_STEP;
            }
        }
        Some(out)
    }

    /// Does child placement `a` stay clear of `i` (see
    /// [`Fan::clear_of_parent`]) and of every sibling in `placed` but
    /// slot `skip`?
    fn fits_among(
        &self,
        i: usize,
        a: At,
        placed: &[(i32, At)],
        skip: usize,
        inbound: Option<Seg>,
    ) -> bool {
        // Newest siblings first: they are the likeliest clash.
        self.clear_of_parent(i, a, inbound)
            && (0..placed.len())
                .rev()
                .all(|q| q == skip || self.siblings_clear(a, placed[q].1))
    }

    /// A lone child of a non-root node: straight on along `dir` when it
    /// fits there, else the nearest direction either side (no sibling to
    /// measure a fan against, so no tight sweep to run first).
    fn single(
        &mut self,
        i: usize,
        c: usize,
        dir: i32,
        gap: f64,
        inbound: Option<Seg>,
    ) -> Option<Vec<(i32, At)>> {
        let half = (MAX_FAN_WIDE * 90.0 / PI) as i32;
        let mut s = 0;
        while s <= half {
            for t in [dir + s, dir - s] {
                if self.spent() {
                    return None;
                }
                if let Some(a) = self.put(i, c, t, gap) {
                    if self.clear_of_parent(i, a, inbound) {
                        return Some(vec![(t, a)]);
                    }
                }
                if s == 0 {
                    break;
                }
            }
            s += DEG_STEP;
        }
        None
    }

    /// `i`'s children placed round it with every spoke `gap` long, or
    /// `None` when they do not fit at that length. A tight sweep (every
    /// child as close to its predecessor as it can go) measures the arc
    /// they need; the real sweep then hands the spare arc out evenly,
    /// ADDED to the separation each child needed in the tight sweep —
    /// the root shares it round the full circle from 12 o'clock, any
    /// other node up to [`FAN_PER_CHILD`] between neighbours, centred on
    /// `dir`.
    fn arrange(
        &mut self,
        i: usize,
        kids: &[usize],
        dir: i32,
        gap: f64,
        inbound: Option<Seg>,
    ) -> Option<Vec<(i32, At)>> {
        let root = inbound.is_none();
        let k = kids.len();
        if !root && k == 1 {
            return self.single(i, kids[0], dir, gap, inbound);
        }
        let deg = |r: f64| (r * 180.0 / PI).round() as i32;
        let snap = |d: f64| (d.round() as i32).div_euclid(DEG_STEP) * DEG_STEP;
        // Child j's first direction in the spread sweep: where the tight
        // sweep put it, turned to `start`, plus j shares of `extra`.
        let spread = |tight: &[(i32, At)], start: f64, extra: f64| -> Vec<i32> {
            tight
                .iter()
                .enumerate()
                .map(|(j, &(t, _))| snap(start + f64::from(t - tight[0].0) + extra * j as f64))
                .collect()
        };
        if root {
            let tight = self.sweep(i, kids, &vec![dir; k], 360 - DEG_STEP, gap, None)?;
            // Room needed to close the circle: child 0 again, past the
            // last child, must clear children 1.. .
            let t0 = tight[0].0;
            let mut t = tight[k - 1].0.max(t0 + DEG_STEP);
            let mut closed = None;
            while t <= t0 + 360 && !self.spent() {
                if let Some(a) = self.put(i, kids[0], t, gap) {
                    if (1..k).all(|q| self.siblings_clear(a, tight[q].1)) {
                        closed = Some(t - t0);
                        break;
                    }
                }
                t += DEG_STEP;
            }
            let total = closed?;
            if total + DEG_STEP * k as i32 > 360 {
                return None;
            }
            // Child 0's subtree starts just clockwise of 12 o'clock,
            // half a share of the spare arc past it: offset by its own
            // counter-clockwise half-width.
            let a0 = tight[0].1;
            let lo = self.entries[a0.e]
                .shape
                .boxes
                .iter()
                .map(|&b| shift(b, a0.o))
                .flat_map(|b| [(b.0, b.1), (b.2, b.1), (b.2, b.3), (b.0, b.3)])
                .map(|(x, y)| wrap(y.atan2(x) - rad(t0)))
                .fold(0.0f64, f64::min);
            // A spread that its re-laid-out subtrees cannot keep is
            // retried with half the spare arc before the gap grows.
            for part in [1.0, 0.5] {
                let extra = part * f64::from(360 - total) / k as f64;
                let start = -90.0 - lo * 180.0 / PI + extra / 2.0;
                let first = spread(&tight, start, extra);
                let limit = 360 - DEG_STEP - (extra / 2.0) as i32;
                if let Some(out) = self.sweep(i, kids, &first, limit, gap, None) {
                    return Some(out);
                }
            }
            None
        } else {
            // The tight sweep starts half the widest fan back from `dir`
            // (never across the band behind `i`, where its connector
            // arrives) and may use all of it.
            let limit = deg(MAX_FAN_WIDE);
            let from = snap(f64::from(dir) - f64::from(limit) / 2.0);
            let tight = self.sweep(i, kids, &vec![from; k], limit, gap, inbound)?;
            // A fan keeps to MAX_FAN when it fits there and opens only as
            // wide as it must (up to MAX_FAN_WIDE).
            let span = tight[k - 1].0 - tight[0].0;
            let arc = span.max(deg(MAX_FAN));
            let want = (deg(FAN_PER_CHILD) * (k as i32 - 1)).min(arc);
            let extra = f64::from((want - span).max(0)) / (k - 1) as f64;
            let width = f64::from(span) + extra * (k - 1) as f64;
            let first = spread(&tight, f64::from(dir) - width / 2.0, extra);
            self.sweep(i, kids, &first, limit, gap, inbound)
                .or_else(|| {
                    // Turning the fan reshaped it; try it tight.
                    let first = spread(&tight, f64::from(dir - span / 2), 0.0);
                    self.sweep(i, kids, &first, limit, gap, inbound)
                })
        }
    }

    /// Lay out `i`'s descendants round it (at the origin), its own
    /// connector arriving along `dir` degrees (screen coordinates:
    /// 0 = right, 90 = down). Returns every strict descendant's centre
    /// relative to `i`, in preorder; `None` when the fan cannot be made
    /// to fit or the work budget is spent.
    ///
    /// Each child sits on a spoke from `i` (see [`spoke_len`]), arranged
    /// by [`Fan::arrange`]. The base length ([`TRUNK`] from the root,
    /// [`SPOKE`] below) is tried first; a fan that does not fit there
    /// grows its spokes (×1.25, or straight to [`Fan::gap_estimate`])
    /// until it does, then bisects back down between the last length
    /// that failed and the first that fitted — so the shared length is
    /// no longer than the fan needs. Finally every child's own spoke is
    /// shortened as far as it stays clear (see [`Fan::shorten`]): a
    /// small branch sits close in even when a bushy sibling needs room.
    fn fan(&mut self, i: usize, dir: i32) -> Option<Vec<(f64, f64)>> {
        let kids = self.d.nodes[i].children.clone();
        let base = self.span[i].0 + 1;
        let mut pos = vec![(0.0, 0.0); self.span[i].1 - base];
        if kids.is_empty() {
            return Some(pos);
        }
        let root = self.d.nodes[i].parent.is_none();
        let floor = if root { TRUNK } else { SPOKE };
        let inbound = (!root).then(|| {
            let back = rad(dir) + PI;
            (
                (0.0, 0.0),
                (back.cos() * INBOUND_RAY, back.sin() * INBOUND_RAY),
            )
        });
        let mut gap = self.fitted[i].unwrap_or(floor);
        let mut estimated = self.fitted[i].is_some();
        // The longest gap known not to fit (or the base length, untried
        // when a memoised gap fitted first time).
        let mut failed = floor;
        let mut placed = None;
        for _ in 0..FAN_GROWTHS {
            if self.spent() {
                return None;
            }
            if let Some(p) = self.arrange(i, &kids, dir, gap, inbound) {
                placed = Some(p);
                break;
            }
            failed = gap;
            gap *= 1.25;
            if !estimated {
                // The base length failed: jump straight to (most of) the
                // estimate rather than ×1.25 a step at a time; the
                // bisection below takes back any overshoot.
                estimated = true;
                gap = gap.max(self.gap_estimate(i, dir, &kids)?);
            }
        }
        let mut placed = placed?;
        let mut tries = 0;
        while gap > failed * GAP_TOLERANCE && tries < GAP_BISECTIONS {
            tries += 1;
            let mid = (failed + gap) / 2.0;
            match self.arrange(i, &kids, dir, mid, inbound) {
                Some(p) => {
                    placed = p;
                    gap = mid;
                }
                None => failed = mid,
            }
        }
        if gap > floor {
            self.shorten(i, &kids, &mut placed, floor, gap, inbound);
        }
        self.fitted[i] = Some(self.fitted[i].map_or(gap, |g| g.min(gap)));
        for (&c, &(_, a)) in kids.iter().zip(&placed) {
            let at = self.span[c].0 - base;
            pos[at] = a.o;
            for (t, &q) in self.entries[a.e].pos.iter().enumerate() {
                pos[at + 1 + t] = (q.0 + a.o.0, q.1 + a.o.1);
            }
        }
        Some(pos)
    }

    /// Give each child of `i` its own spoke: every child, smallest
    /// subtree first and twice round, slides in along its direction to
    /// the shortest gap in `[floor, gap]` where it stays clear of `i`
    /// and of its siblings as they now stand. A shared length sized for
    /// the bushiest child would otherwise leave a leaf branch at the
    /// end of a long empty trunk; the result is still clean, since
    /// every accepted position passed the same tests as the sweep.
    fn shorten(
        &mut self,
        i: usize,
        kids: &[usize],
        placed: &mut [(i32, At)],
        floor: f64,
        gap: f64,
        inbound: Option<Seg>,
    ) {
        let mut order: Vec<usize> = (0..kids.len()).collect();
        order.sort_by_key(|&j| (self.span[kids[j]].1 - self.span[kids[j]].0, j));
        for _ in 0..2 {
            for &j in &order {
                let (t, now) = placed[j];
                let c = kids[j];
                let fits = |fan: &mut Self, g: f64| {
                    fan.put(i, c, t, g)
                        .filter(|&a| fan.fits_among(i, a, placed, j, inbound))
                };
                // Where it sits now is the upper bound; try the floor
                // outright (the common case for a small branch), then
                // bisect between them.
                let now_len = now.o.0.hypot(now.o.1);
                let (mut lo, mut hi) = (floor, gap);
                let mut best = None;
                if let Some(a) = fits(self, floor) {
                    best = Some(a);
                } else {
                    for _ in 0..GAP_BISECTIONS {
                        if hi <= lo * GAP_TOLERANCE || self.spent() {
                            break;
                        }
                        let mid = (lo + hi) / 2.0;
                        match fits(self, mid) {
                            Some(a) => {
                                best = Some(a);
                                hi = mid;
                            }
                            None => lo = mid,
                        }
                    }
                }
                if let Some(a) = best {
                    if a.o.0.hypot(a.o.1) < now_len - 0.5 {
                        placed[j] = (t, a);
                    }
                }
            }
        }
    }
