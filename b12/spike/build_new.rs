    // Registered legs (true flow coordinates), so later paths take a
    // free track.
    let mut vlegs: Vec<(f64, f64, f64)> = Vec::new(); // b, l0, l1
    let mut hlegs: Vec<(f64, f64, f64)> = Vec::new(); // l, b0, b1
    // One corridor from (x0, ys) down to (xt, yt), `obs` in the same
    // coordinates, mirrored along the flow when `m` = -1; `s1` / `t0`
    // the borders of the end units the bends keep outside of where they
    // can; `sw` / `tw` = the breadth extents of the end sides. `None`
    // when the sweep finds no way (today's builder then draws the edge).
    #[allow(clippy::too_many_arguments)]
    let build = |x0: f64, ys: f64, xt: f64, yt: f64, s1: f64, t0: f64, obs: &[R], chips: &[R], m: f64,
                 sw: (f64, f64), tw: (f64, f64),
                 vlegs: &mut Vec<(f64, f64, f64)>, hlegs: &mut Vec<(f64, f64, f64)>|
     -> Option<Vec<(f64, f64)>> {
        const CL: f64 = 6.5;
        let tl = |a: f64, b: f64| ((a * m).min(b * m), (a * m).max(b * m));
        let free_b = |vlegs: &[(f64, f64, f64)], b: f64, l0: f64, l1: f64| {
            let (l0, l1) = tl(l0, l1);
            !vlegs.iter().any(|&(x, a, c)| (x - b).abs() < PITCH * 0.9 && a < l1 && c > l0)
        };
        let free_l = |hlegs: &[(f64, f64, f64)], l: f64, b0: f64, b1: f64| {
            !hlegs.iter().any(|&(y, a, c)| (y - l * m).abs() < PITCH * 0.9 && a < b1 && c > b0)
        };
        // An end under a title chip takes the free stretch of its side
        // beside the chip when there is one; else it crosses the title
        // side beside the chip and jogs to its port between the strip
        // and the node.
        let over = |b: f64, c: &R| b > c.0 - CL && b < c.1 + CL && c.3 > ys - 0.5 && c.2 < yt + 0.5;
        let mut head: Vec<(f64, f64)> = Vec::new();
        let mut tail: Vec<(f64, f64)> = Vec::new();
        let (mut x0, mut ys, mut xt, mut yt) = (x0, ys, xt, yt);
        if let Some(c) = chips.iter().find(|c| over(xt, c) && (yt - c.3) < (c.2 - ys)) {
            let free = c.1 + CL + 2.0;
            if tw.1 - 6.0 > free {
                xt = (free + tw.1 - 6.0) / 2.0;
            } else {
                let mut xe = free;
                let rj = (c.3 + yt) / 2.0;
                for _ in 0..20 {
                    if free_b(vlegs, xe, c.2 - PAD, rj) {
                        break;
                    }
                    xe += PITCH;
                }
                let mut rj2 = rj;
                for k in 0..4 {
                    rj2 = rj - 4.0 * k as f64;
                    if free_l(hlegs, rj2, xt.min(xe), xt.max(xe)) {
                        break;
                    }
                }
                tail = vec![(xt, rj2), (xt, yt)];
                hlegs.push((rj2 * m, xt.min(xe), xt.max(xe)));
                xt = xe;
                yt = rj2;
            }
        }
        if let Some(c) = chips.iter().find(|c| over(x0, c) && (c.2 - ys) < (yt - c.3)) {
            let free = c.1 + CL + 2.0;
            if sw.1 - 6.0 > free {
                x0 = (free + sw.1 - 6.0) / 2.0;
            } else {
                let mut xe = free;
                let rj = (ys + c.2) / 2.0;
                for _ in 0..20 {
                    if free_b(vlegs, xe, rj, c.3 + PAD) {
                        break;
                    }
                    xe += PITCH;
                }
                let mut rj2 = rj;
                for k in 0..4 {
                    rj2 = rj + 4.0 * k as f64;
                    if free_l(hlegs, rj2, x0.min(xe), x0.max(xe)) {
                        break;
                    }
                }
                head = vec![(x0, ys), (x0, rj2)];
                hlegs.push((rj2 * m, x0.min(xe), x0.max(xe)));
                x0 = xe;
                ys = rj2;
            }
        }
        let obs: Vec<R> = obs
            .iter()
            .copied()
            .chain(chips.iter().map(|c| (c.0 + 6.0, c.1 - 6.0, c.2 + 6.0, c.3 - 6.0)))
            .filter(|o| o.3 > ys + 0.5 && o.2 < yt - 0.5)
            .collect();
        // A free row in (lo, hi) for a level run over breadths b0..b1,
        // as near `prefer` as its gap and the rows taken allow, from
        // which `ok` holds.
        let free_row = |lo: f64, hi: f64, b0: f64, b1: f64, prefer: f64, hlegs: &[(f64, f64, f64)], ok: &dyn Fn(f64) -> bool| -> Option<f64> {
            let (b0, b1) = (b0.min(b1), b0.max(b1));
            let mut iv: Vec<(f64, f64)> = obs
                .iter()
                .filter(|o| o.0 - PAD < b1 && o.1 + PAD > b0)
                .map(|o| (o.2 - PAD, o.3 + PAD))
                .collect();
            iv.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut gaps: Vec<(f64, f64)> = Vec::new();
            let mut at = lo;
            for (a, b) in iv {
                if a > at {
                    gaps.push((at, a.min(hi)));
                }
                at = at.max(b);
                if at >= hi {
                    break;
                }
            }
            if at < hi {
                gaps.push((at, hi));
            }
            let mut best: Option<(f64, f64)> = None;
            for (g0, g1) in gaps {
                if g1 - g0 < 8.0 {
                    continue;
                }
                let base = prefer.clamp(g0 + 4.0, g1 - 4.0);
                if !ok(base) {
                    continue;
                }
                let mut r = None;
                for k in 0..16 {
                    for sgn in [-1.0, 1.0] {
                        let l = base + sgn * (k as f64) * PITCH;
                        if r.is_none() && l >= g0 + 4.0 && l <= g1 - 4.0 && free_l(hlegs, l, b0, b1) {
                            r = Some(l);
                        }
                    }
                }
                let r = r.unwrap_or(base);
                let d = (r - prefer).abs();
                if best.map_or(true, |b| d < b.1) {
                    best = Some((r, d));
                }
            }
            best.map(|b| b.0)
        };
        let ray_clear = |b: f64, l0: f64, l1: f64| {
            !obs.iter().any(|o| b > o.0 - PAD && b < o.1 + PAD && o.3 > l0 + 0.5 && o.2 < l1 - 0.5)
        };
        let mut pts = vec![(x0, ys)];
        let (mut x, mut l) = (x0, ys);
        let mut done = false;
        for _ in 0..12 {
            let hit = obs
                .iter()
                .filter(|o| x > o.0 - PAD && x < o.1 + PAD && o.3 > l + 0.5)
                .min_by(|a, b| a.2.total_cmp(&b.2))
                .copied();
            let limit = hit.map_or(yt, |o| (o.2 - PAD).min(yt));
            if (x - xt).abs() > 0.5 {
                let prefer = if t0 - 16.0 > l + 4.0 && t0 - 16.0 < limit { t0 - 16.0 } else { (l + limit) / 2.0 };
                if let Some(r) = free_row(l, limit, x, xt, prefer, hlegs, &|r| ray_clear(xt, r, yt)) {
                    pts.push((x, r));
                    pts.push((xt, r));
                    hlegs.push((r * m, x.min(xt), x.max(xt)));
                    done = true;
                    break;
                }
            } else if hit.is_none() {
                done = true;
                break;
            }
            let o = hit?;
            // The blocked breadths round x in o's band, merged.
            let (mut lo, mut hi) = (o.0 - PAD, o.1 + PAD);
            loop {
                let mut grew = false;
                for q in obs.iter().filter(|q| q.2 < o.3 + 2.0 * PAD && q.3 > o.2 - 2.0 * PAD) {
                    if q.0 - PAD < hi && q.1 + PAD > lo && (q.0 - PAD < lo || q.1 + PAD > hi) {
                        lo = lo.min(q.0 - PAD);
                        hi = hi.max(q.1 + PAD);
                        grew = true;
                    }
                }
                if !grew {
                    break;
                }
            }
            // The side that costs the least sideways travel to the target.
            let left = (x - lo) + (lo - xt).abs() <= (hi - x) + (hi - xt).abs();
            let mut nx = if left { lo } else { hi };
            for _ in 0..40 {
                if free_b(vlegs, nx, o.2 - PAD, o.3 + PAD) {
                    break;
                }
                nx += if left { -PITCH } else { PITCH };
            }
            let lower = if s1 > l && s1 < limit { s1 } else { l };
            let r = free_row(l, limit, x, nx, (lower + limit) / 2.0, hlegs, &|_| true)?;
            pts.push((x, r));
            pts.push((nx, r));
            hlegs.push((r * m, x.min(nx), x.max(nx)));
            x = nx;
            l = r;
        }
        if !done {
            return None;
        }
        pts.push((xt, yt));
        if !head.is_empty() {
            let mut h = head.clone();
            h.extend(pts.iter().copied());
            pts = h;
        }
        if !tail.is_empty() {
            pts.extend(tail.iter().copied());
        }
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01);
        for w in pts.windows(2) {
            if (w[0].0 - w[1].0).abs() < 0.5 {
                let (a, c) = tl(w[0].1, w[1].1);
                vlegs.push((w[0].0, a, c));
            }
        }
        Some(pts)
    };
