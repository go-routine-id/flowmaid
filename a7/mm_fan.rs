/// A laid-out subtree as obstacles, relative to its root's centre: every
/// node box `(x0, y0, x1, y1)` (its own first) and every connector.
struct Shape {
    boxes: Vec<(f64, f64, f64, f64)>,
    segs: Vec<((f64, f64), (f64, f64))>,
    /// Bounding box of `boxes`, for a cheap first test.
    bbox: (f64, f64, f64, f64),
}

/// A laid-out subtree: its [`Shape`], and the centre of every strict
/// descendant (in preorder) relative to the subtree's root.
struct Entry {
    shape: Shape,
    pos: Vec<(f64, f64)>,
}

/// A child placed round its parent (at the origin): which subtree
/// layout ([`Entry`] index) and where its centre sits. Its spoke runs
/// from the origin to `o`.
#[derive(Clone, Copy)]
struct At {
    e: usize,
    o: (f64, f64),
}

type Rect = (f64, f64, f64, f64);
type Seg = ((f64, f64), (f64, f64));

/// Two boxes closer than `m` (on both axes)?
fn boxes_near(a: Rect, b: Rect, m: f64) -> bool {
    a.0 < b.2 + m && b.0 < a.2 + m && a.1 < b.3 + m && b.1 < a.3 + m
}

fn shift(b: Rect, o: (f64, f64)) -> Rect {
    (b.0 + o.0, b.1 + o.1, b.2 + o.0, b.3 + o.1)
}

fn shift_seg(s: Seg, o: (f64, f64)) -> Seg {
    ((s.0 .0 + o.0, s.0 .1 + o.1), (s.1 .0 + o.0, s.1 .1 + o.1))
}

/// Clear space a connector keeps from a box it does not belong to —
/// about half the thickest connector, so even a trunk never touches one.
const SEG_CLEAR: f64 = 5.0;

/// `s` passes through (or within [`SEG_CLEAR`] of) one of `sh`'s boxes
/// moved by `o`?
fn seg_hits_shape(s: Seg, sh: &Shape, o: (f64, f64)) -> bool {
    sh.boxes.iter().any(|&b| {
        let b = shift(b, o);
        segment_hits_box(
            s.0,
            s.1,
            (b.0 - SEG_CLEAR, b.1 - SEG_CLEAR, b.2 + SEG_CLEAR, b.3 + SEG_CLEAR),
        )
    })
}

/// Directions are whole multiples of this many degrees, so a subtree
/// laid out for one direction is memoised and reused.
const DEG_STEP: i32 = 3;

fn rad(deg: i32) -> f64 {
    f64::from(deg) * PI / 180.0
}

/// Length of the stand-in for a node's own incoming connector: a ray
/// straight back from it, long enough to reach past its parent, so a
/// node's descendants never cross where its connector comes from.
const INBOUND_RAY: f64 = 4000.0;

/// The parent-centred fan layout. [`Fan::layout`] lays out a node's
/// subtree with the node at the origin, for a given incoming direction,
/// memoised per (node, direction) — the parent sweeps candidate
/// directions for each child, and every candidate needs that child's
/// exact subtree, because axis-aligned boxes do not rotate rigidly.
struct Fan<'a> {
    d: &'a Mindmap,
    size: &'a [(f64, f64)],
    /// Preorder of the tree and each node's `[start, end)` span in it,
    /// so a subtree's members are one contiguous slice.
    pre: Vec<usize>,
    span: Vec<(usize, usize)>,
    memo: std::collections::HashMap<(usize, i32), Option<usize>>,
    entries: Vec<Entry>,
    /// A leaf's one-box entry (its layout does not depend on direction).
    leaf: Vec<Option<usize>>,
    /// Box-pair tests run so far, against [`FAN_WORK`].
    work: std::cell::Cell<usize>,
}

impl Fan<'_> {
    fn new<'a>(d: &'a Mindmap, size: &'a [(f64, f64)]) -> Option<Fan<'a>> {
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
            pre,
            span,
            memo: std::collections::HashMap::new(),
            entries: Vec::new(),
            leaf: vec![None; n],
            work: std::cell::Cell::new(0),
        };
        for i in 0..n {
            if d.nodes[i].children.is_empty() {
                let b = fan.boxed(i, (0.0, 0.0));
                fan.entries.push(Entry {
                    shape: Shape {
                        boxes: vec![b],
                        segs: Vec::new(),
                        bbox: b,
                    },
                    pos: Vec::new(),
                });
                fan.leaf[i] = Some(fan.entries.len() - 1);
            }
        }
        Some(fan)
    }

    fn spent(&self) -> bool {
        self.work.get() > FAN_WORK
    }

    fn boxed(&self, m: usize, at: (f64, f64)) -> Rect {
        let (w, h) = self.size[m];
        (at.0 - w / 2.0, at.1 - h / 2.0, at.0 + w / 2.0, at.1 + h / 2.0)
    }

    /// The subtree of `c` laid out for incoming direction `deg`, as an
    /// index into `entries`; `None` when it cannot be laid out there.
    fn layout(&mut self, c: usize, deg: i32) -> Option<usize> {
        if let Some(e) = self.leaf[c] {
            return Some(e);
        }
        let deg = deg.rem_euclid(360);
        if let Some(&e) = self.memo.get(&(c, deg)) {
            return e;
        }
        let r = self.fan(c, deg).map(|pos| {
            let base = self.span[c].0 + 1;
            let mut boxes = vec![self.boxed(c, (0.0, 0.0))];
            let mut segs = Vec::with_capacity(pos.len());
            for (k, &p) in pos.iter().enumerate() {
                let m = self.pre[base + k];
                boxes.push(self.boxed(m, p));
                let parent = self.d.nodes[m].parent.unwrap_or(c);
                let from = if parent == c {
                    (0.0, 0.0)
                } else {
                    pos[self.span[parent].0 - base]
                };
                segs.push((from, p));
            }
            let bbox = boxes.iter().fold(
                (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
                |a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
            );
            self.entries.push(Entry {
                shape: Shape { boxes, segs, bbox },
                pos,
            });
            self.entries.len() - 1
        });
        self.memo.insert((c, deg), r);
        r
    }

    /// Child `c` of `i` placed for direction `deg`, spokes `gap` long.
    fn put(&mut self, i: usize, c: usize, deg: i32, gap: f64) -> Option<At> {
        let e = self.layout(c, deg)?;
        let t = rad(deg);
        let u = (t.cos(), t.sin());
        let r = spoke_len(self.size[i], self.size[c], u, gap);
        Some(At {
            e,
            o: (u.0 * r, u.1 * r),
        })
    }

    /// Do two children of the same parent keep clear of each other:
    /// boxes [`SEP`] apart, no connector (spokes included) through the
    /// other's boxes, no crossings?
    fn siblings_clear(&self, a: At, b: At) -> bool {
        let (sa, sb) = (&self.entries[a.e].shape, &self.entries[b.e].shape);
        let spoke_a: Seg = ((0.0, 0.0), a.o);
        let spoke_b: Seg = ((0.0, 0.0), b.o);
        if !boxes_near(shift(sa.bbox, a.o), shift(sb.bbox, b.o), SEP + SEG_CLEAR) {
            // A subtree's own connectors run between its boxes' centres,
            // so inside its hull; only the spokes reach outside it.
            self.work.set(self.work.get() + sa.boxes.len() + sb.boxes.len());
            return !seg_hits_shape(spoke_a, sb, b.o) && !seg_hits_shape(spoke_b, sa, a.o);
        }
        self.work.set(self.work.get() + sa.boxes.len() * sb.boxes.len() + 2 * (sa.segs.len() + 1) * (sb.segs.len() + 1));
        if sa.boxes.iter().any(|&p| {
            let p = shift(p, a.o);
            sb.boxes.iter().any(|&q| boxes_near(p, shift(q, b.o), SEP))
        }) {
            return false;
        }
        let segs_a = std::iter::once(spoke_a).chain(sa.segs.iter().map(|&s| shift_seg(s, a.o)));
        for s in segs_a {
            if seg_hits_shape(s, sb, b.o) {
                return false;
            }
            let crosses = std::iter::once(spoke_b)
                .chain(sb.segs.iter().map(|&q| shift_seg(q, b.o)))
                .any(|q| segments_cross(s.0, s.1, q.0, q.1));
            if crosses {
                return false;
            }
        }
        let mut segs_b = std::iter::once(spoke_b).chain(sb.segs.iter().map(|&s| shift_seg(s, b.o)));
        !segs_b.any(|s| seg_hits_shape(s, sa, a.o))
    }

    /// Does a placed child of `i` stay clear of `i` itself — its box
    /// (by [`SEP`]/2) and its incoming connector (`inbound`)?
    fn clear_of_parent(&self, i: usize, a: At, inbound: Option<Seg>) -> bool {
        let own = self.boxed(i, (0.0, 0.0));
        let sh = &self.entries[a.e].shape;
        self.work.set(self.work.get() + 2 * sh.boxes.len());
        // The child's own box keeps its distance through `spoke_len`,
        // and its spoke starts here; test everything below it.
        if sh.boxes[1..]
            .iter()
            .any(|&b| boxes_near(shift(b, a.o), own, SEP / 2.0))
        {
            return false;
        }
        let own_shape = Shape {
            boxes: vec![own],
            segs: Vec::new(),
            bbox: own,
        };
        if sh
            .segs
            .iter()
            .any(|&s| seg_hits_shape(shift_seg(s, a.o), &own_shape, (0.0, 0.0)))
        {
            return false;
        }
        inbound.map_or(true, |r| {
            !seg_hits_shape(r, sh, a.o)
                && !sh.segs.iter().any(|&s| {
                    let s = shift_seg(s, a.o);
                    segments_cross(r.0, r.1, s.0, s.1)
                })
        })
    }

    /// Sweep `i`'s children round it, clockwise in source order: child
    /// 0 from `start`, every later one from its predecessor's direction
    /// + `extra`, each stepping [`DEG_STEP`] at a time to the first
    /// direction where its whole subtree stays clear of `i` and of every
    /// child already placed. `None` once a child would pass `start +
    /// limit`. Every candidate is tested with the child's exact subtree
    /// for that direction, so the result is clean by construction.
    #[allow(clippy::too_many_arguments)]
    fn sweep(
        &mut self,
        i: usize,
        kids: &[usize],
        start: i32,
        extra: i32,
        limit: i32,
        gap: f64,
        inbound: Option<Seg>,
    ) -> Option<Vec<(i32, At)>> {
        let mut out: Vec<(i32, At)> = Vec::with_capacity(kids.len());
        for (j, &c) in kids.iter().enumerate() {
            let mut t = if j == 0 { start } else { out[j - 1].0 + extra };
            loop {
                if t - start > limit || self.spent() {
                    return None;
                }
                if let Some(a) = self.put(i, c, t, gap) {
                    // Newest siblings first: they are the likeliest clash.
                    let clear = self.clear_of_parent(i, a, inbound)
                        && (0..out.len())
                            .rev()
                            .all(|q| self.siblings_clear(a, out[q].1));
                    if clear {
                        out.push((t, a));
                        break;
                    }
                }
                t += DEG_STEP;
            }
        }
        Some(out)
    }

    /// Lay out `i`'s descendants round it (at the origin), its own
    /// connector arriving along `dir` degrees (screen coordinates:
    /// 0 = right, 90 = down). Returns every strict descendant's centre
    /// relative to `i`, in preorder; `None` when the fan cannot be made
    /// to fit or the work budget is spent.
    ///
    /// Each child sits on a spoke from `i` (see [`spoke_len`]). A tight
    /// sweep measures how much arc the children need; the real sweep
    /// then spreads them (up to [`FAN_PER_CHILD`] between neighbours)
    /// and centres the fan on `dir`, while the root shares its slack
    /// out round the full circle from 12 o'clock. A fan that will not
    /// fit first opens wider (to [`MAX_FAN_WIDE`]), then lengthens its
    /// spokes ×1.25.
    fn fan(&mut self, i: usize, dir: i32) -> Option<Vec<(f64, f64)>> {
        let kids = self.d.nodes[i].children.clone();
        let base = self.span[i].0 + 1;
        let mut pos = vec![(0.0, 0.0); self.span[i].1 - base];
        if kids.is_empty() {
            return Some(pos);
        }
        let root = self.d.nodes[i].parent.is_none();
        let k = kids.len() as i32;
        let deg = |r: f64| (r * 180.0 / PI).round() as i32;
        let snap = |d: i32| d.div_euclid(DEG_STEP) * DEG_STEP;
        let mut max_arc = if root { 360 } else { deg(MAX_FAN) };
        let mut gap = if root { TRUNK } else { SPOKE };
        let inbound = (!root).then(|| {
            let back = rad(dir) + PI;
            (
                (0.0, 0.0),
                (back.cos() * INBOUND_RAY, back.sin() * INBOUND_RAY),
            )
        });
        for _ in 0..FAN_GROWTHS {
            if self.spent() {
                return None;
            }
            let tight_limit = if root { 360 - DEG_STEP } else { max_arc };
            let swept = match self.sweep(i, &kids, snap(dir), 0, tight_limit, gap, inbound) {
                None => None,
                Some(tight) if root => {
                    // Room needed to close the circle: child 0 again,
                    // past the last child, must clear children 1.. .
                    let t0 = tight[0].0;
                    let mut t = tight[k as usize - 1].0.max(t0 + DEG_STEP);
                    let mut closed = None;
                    while t <= t0 + 360 && !self.spent() {
                        if let Some(a) = self.put(i, kids[0], t, gap) {
                            if (1..tight.len()).all(|q| self.siblings_clear(a, tight[q].1)) {
                                closed = Some(t - t0);
                                break;
                            }
                        }
                        t += DEG_STEP;
                    }
                    match closed {
                        Some(total) if total + DEG_STEP * k <= 360 => {
                            let extra = snap((360 - total) / k);
                            // Child 0's subtree starts just clockwise of
                            // 12 o'clock: offset by its own half-width.
                            let a0 = tight[0].1;
                            let lo = self.entries[a0.e]
                                .shape
                                .boxes
                                .iter()
                                .map(|&b| shift(b, a0.o))
                                .flat_map(|b| [(b.0, b.1), (b.2, b.1), (b.2, b.3), (b.0, b.3)])
                                .map(|(x, y)| wrap(y.atan2(x) - rad(t0)))
                                .fold(0.0f64, f64::min);
                            let start = snap(-90 - deg(lo) + extra / 2);
                            self.sweep(i, &kids, start, extra, 360 - DEG_STEP - extra / 2, gap, inbound)
                        }
                        _ => None,
                    }
                }
                Some(tight) => {
                    let span = tight[k as usize - 1].0 - tight[0].0;
                    let want = (deg(FAN_PER_CHILD) * (k - 1)).min(max_arc);
                    let extra = if k > 1 {
                        snap((want - span).max(0) / (k - 1))
                    } else {
                        0
                    };
                    let width = span + extra * (k - 1);
                    self.sweep(i, &kids, snap(dir - width / 2), extra, max_arc, gap, inbound)
                }
            };
            if let Some(placed) = swept {
                for (&c, &(_, a)) in kids.iter().zip(&placed) {
                    let at = self.span[c].0 - base;
                    pos[at] = a.o;
                    for (t, &q) in self.entries[a.e].pos.iter().enumerate() {
                        pos[at + 1 + t] = (q.0 + a.o.0, q.1 + a.o.1);
                    }
                }
                return Some(pos);
            }
            if !root && max_arc < deg(MAX_FAN_WIDE) {
                max_arc = (max_arc + deg(FAN_WIDEN)).min(deg(MAX_FAN_WIDE));
            } else {
                gap *= 1.25;
            }
        }
        None
    }
}

/// Centres (root at the origin) from the parent-centred fan layout, or
/// `None` when it could not produce a clean layout — see [`scene`].
fn fan_centres(d: &Mindmap, size: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let mut fan = Fan::new(d, size)?;
    let below = fan.fan(0, -90)?;
    let mut pos = vec![(0.0, 0.0); d.nodes.len()];
    for (k, &p) in below.iter().enumerate() {
        pos[fan.pre[1 + k]] = p;
    }
    clean(d, size, &pos).then_some(pos)
}

