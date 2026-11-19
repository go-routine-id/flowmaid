import sys
root = sys.argv[1]
p = root + '/src/pie.rs'
s = open(p).read()

old_doc = '''    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s: the legend over the percentage labels over the
    /// wedges. A wedge is its angular sector inside the radius as drawn
    /// (a slice drawn as the whole disc covers the slices before it); a
    /// slice of 0 has none.'''
new_doc = '''    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s: the legend over the percentage labels over the
    /// wedges. A wedge is the region its path fills, at the 0.1 px
    /// [`to_svg`] writes it to: its angular sector inside the radius,
    /// but with its arc on the circle a renderer puts through the
    /// written ends (ends that lie a few px apart can put that a px or
    /// more off the pie's centre). A slice drawn as the whole disc
    /// covers the slices before it; a slice of 0 has none.'''
assert s.count(old_doc) == 1
s = s.replace(old_doc, new_doc)

old_fn = s[s.index('    /// Whether `p` lies in slice `i` as [`to_svg`] fills it'):s.index('#[cfg(test)]\nmod tests {')]
new_fn = '''    /// Whether `p` lies in slice `i` as [`to_svg`] fills it, at the
    /// 0.1 px it writes every number to ([`written`]): nothing for a
    /// slice of 0, the disc for one drawn as a circle (~100%, or a
    /// dominant slice whose arc ends meet), else the wedge its path
    /// closes ([`wedge_contains`]).
    fn slice_contains(&self, i: usize, p: (f64, f64)) -> bool {
        let sl = &self.slices[i];
        if sl.frac <= 0.0 {
            return false;
        }
        let c = (written(self.cx), written(self.cy));
        let r = written(self.r);
        let a = polar(self.cx, self.cy, self.r, sl.start_angle);
        let b = polar(self.cx, self.cy, self.r, sl.end_angle);
        let coincident = (a.0 - b.0).hypot(a.1 - b.1) < 0.2;
        if sl.frac >= 1.0 - 1e-9 || (sl.frac > 0.5 && coincident) {
            return (p.0 - c.0).hypot(p.1 - c.1) <= r;
        }
        let a = (written(a.0), written(a.1));
        let b = (written(b.0), written(b.1));
        wedge_contains(c, a, b, r, sl.frac > 0.5, p)
    }
}

/// `v` as [`to_svg`] writes it (`{:.1}`) and a renderer reads it back:
/// the nearest tenth. A value at a tie, which `{:.1}` rounds to even, is
/// formatted to be sure.
fn written(v: f64) -> f64 {
    let t = v * 10.0;
    if (t - t.floor() - 0.5).abs() < 1e-6 {
        return format!("{v:.1}").parse().unwrap_or(v);
    }
    t.round() / 10.0
}

/// Whether `p` lies in the region the wedge path
/// `M c L a A r r 0 large 1 b Z` fills (SVG's nonzero rule): the
/// triangle `c a b`, counted with its winding, plus the segment the
/// chord `a b` cuts off the arc's circle on the arc's side. The arc's
/// circle is SVG's (implementation notes, "conversion from endpoint to
/// center parameterization"): radius `r` through both ends, centred on
/// the side `large` and the clockwise sweep put it, the radius grown to
/// half the chord where the chord is longer than the diameter. An arc
/// whose ends meet is not drawn, and one of radius 0 is a straight line.
fn wedge_contains(
    c: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
    r: f64,
    large: bool,
    p: (f64, f64),
) -> bool {
    // (u - o) x (v - o): positive when v lies clockwise of u round o on
    // screen (the y-axis points down).
    let cross = |o: (f64, f64), u: (f64, f64), v: (f64, f64)| {
        (u.0 - o.0) * (v.1 - o.1) - (u.1 - o.1) * (v.0 - o.0)
    };
    let mut winding = 0;
    let turn = cross(c, a, b);
    if turn != 0.0 {
        let s = turn.signum();
        if cross(c, a, p) * s >= 0.0 && cross(a, b, p) * s >= 0.0 && cross(b, c, p) * s >= 0.0 {
            winding += if s > 0.0 { 1 } else { -1 };
        }
    }
    let (hx, hy) = ((a.0 - b.0) / 2.0, (a.1 - b.1) / 2.0);
    let d2 = hx * hx + hy * hy;
    if d2 > 0.0 && r > 0.0 {
        let rr = r.max(d2.sqrt());
        let k = ((rr * rr - d2) / d2).max(0.0).sqrt() * if large { -1.0 } else { 1.0 };
        let m = ((a.0 + b.0) / 2.0 + k * hy, (a.1 + b.1) / 2.0 - k * hx);
        // The arc runs clockwise from `a` to `b`, the chord back: the
        // segment lies to the right of `b -> a` on screen.
        if (p.0 - m.0).hypot(p.1 - m.1) <= rr && cross(b, a, p) >= 0.0 {
            winding += 1;
        }
    }
    winding != 0
}

'''
s = s.replace(old_fn, new_fn)
open(p, 'w').write(s)
