
/// A slice as `to_svg` fills it, read off the SVG text: a disc
/// (`<circle>`, filled) or a wedge path
/// `M cx cy L ax ay A r r 0 large 1 bx by Z`.
enum DrawnSlice {
    Disc((f64, f64), f64),
    Wedge((f64, f64), (f64, f64), f64, bool, (f64, f64)),
}

/// The slices of a pie's SVG in paint order (a hatching painted over
/// its slice's colour repeats the shape: read once).
fn drawn_slices(svg: &str) -> Vec<DrawnSlice> {
    let num = |s: &str| -> Vec<f64> {
        s.split(|c: char| c == ' ' || c == '"')
            .filter_map(|t| t.parse().ok())
            .collect()
    };
    let attr = |tag: &str, name: &str| -> f64 {
        let at = tag.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        tag[at..].split('"').next().unwrap().parse().unwrap()
    };
    let mut out = Vec::new();
    let mut last = String::new();
    for line in svg.lines() {
        let line = line.trim();
        let shape = if let Some(rest) = line.strip_prefix("<path d=\"M ") {
            let d = rest.split('"').next().unwrap().to_string();
            if d == last {
                continue;
            }
            last = d.clone();
            let v = num(&d.replace(['L', 'A', 'Z'], " "));
            DrawnSlice::Wedge((v[0], v[1]), (v[2], v[3]), v[4], v[7] == 1.0, (v[9], v[10]))
        } else if line.starts_with("<circle ") && !line.contains("fill=\"none\"") {
            let d = line.to_string();
            if d.replace("url(#", "") != d && last == "circle" {
                continue;
            }
            last = "circle".into();
            DrawnSlice::Disc((attr(line, "cx"), attr(line, "cy")), attr(line, "r"))
        } else {
            continue;
        };
        out.push(shape);
    }
    out
}

/// Whether `p` lies in `s` as an SVG renderer fills it: the wedge's arc
/// centred as SVG's endpoint-to-centre conversion puts it, the wedge
/// then flattened to a polygon (4096 points along the arc) and tested
/// by its winding number.
fn drawn_contains(s: &DrawnSlice, p: (f64, f64)) -> bool {
    match *s {
        DrawnSlice::Disc(c, r) => (p.0 - c.0).hypot(p.1 - c.1) <= r,
        DrawnSlice::Wedge(c, a, r, large, b) => {
            let (hx, hy) = ((a.0 - b.0) / 2.0, (a.1 - b.1) / 2.0);
            let d2 = hx * hx + hy * hy;
            let rr = r.max(d2.sqrt());
            let sign = if large { -1.0 } else { 1.0 };
            let k = sign * ((rr * rr - d2) / d2).max(0.0).sqrt();
            let m = ((a.0 + b.0) / 2.0 + k * hy, (a.1 + b.1) / 2.0 - k * hx);
            let ta = (a.1 - m.1).atan2(a.0 - m.0);
            let tb = (b.1 - m.1).atan2(b.0 - m.0);
            let sweep = (tb - ta).rem_euclid(std::f64::consts::TAU);
            let mut poly = vec![c, a];
            for i in 1..4096 {
                let t = ta + sweep * i as f64 / 4096.0;
                poly.push((m.0 + rr * t.cos(), m.1 + rr * t.sin()));
            }
            poly.push(b);
            let mut w = 0;
            for i in 0..poly.len() {
                let (u, v) = (poly[i], poly[(i + 1) % poly.len()]);
                let side = (v.0 - u.0) * (p.1 - u.1) - (p.0 - u.0) * (v.1 - u.1);
                if u.1 <= p.1 && v.1 > p.1 && side > 0.0 {
                    w += 1;
                } else if u.1 > p.1 && v.1 <= p.1 && side < 0.0 {
                    w -= 1;
                }
            }
            w != 0
        }
    }
}

/// A slice is picked as `to_svg` fills it, at the 0.1 px its numbers
/// are written to. Where a slice's arc runs between ends a few px apart
/// (a sliver beside a dominant slice), a renderer centres the arc's
/// circle a px or more off the pie's centre: the dominant slice's rim
/// then lies past the pie's radius on one side and short of it on the
/// other, and the picks follow it. Over rings round the rim and through
/// the wedges (points within 0.05 px of a drawn edge left out), every
/// slice pick agrees with the SVG's own shapes.
#[test]
fn pie_slices_are_picked_as_their_paths_are_drawn() {
    use pie::PieHit;
    let mut docs = docs_of!(Pie);
    for src in [
        "pie\n\"a\" : 100\n\"b\" : 0.5\n",
        "pie\n\"a\" : 1000\n\"b\" : 10\n\"c\" : 5\n",
        "pie\n\"a\" : 0.004\n\"b\" : 0.006\n\"c\" : 0.99\n",
        "pie title Pets\n\"Dogs\" : 386\n\"Cats\" : 85\n\"Rats\" : 15\n\"Bats\" : 1\n",
        "pie\n\"a\" : 300\n\"b\" : 1\n",
    ] {
        if let Document::Pie(d) = parse(src) {
            docs.push((src.into(), d));
        }
    }
    for (name, d) in docs {
        let ps = pie::scene(&d);
        let drawn = drawn_slices(&pie::to_svg(&ps));
        let shown: Vec<usize> = (0..ps.slices.len())
            .filter(|&i| ps.slices[i].frac > 0.0)
            .collect();
        assert_eq!(drawn.len(), shown.len(), "{name}");
        let top = |p: (f64, f64)| {
            (0..drawn.len())
                .rev()
                .find(|&k| drawn_contains(&drawn[k], p))
                .map(|k| shown[k])
        };
        let (mut checked, mut off_exact) = (0, 0);
        for ring in (0..=24).map(|k| ps.r - 3.0 + 0.25 * k as f64).chain([0.3 * ps.r, 0.6 * ps.r]) {
            for step in 0..1440 {
                let t = step as f64 / 1440.0 * std::f64::consts::TAU;
                let p = (ps.cx + ring * t.sin(), ps.cy - ring * t.cos());
                let want = top(p);
                let steady = [(0.05, 0.0), (-0.05, 0.0), (0.0, 0.05), (0.0, -0.05)]
                    .iter()
                    .all(|d| top((p.0 + d.0, p.1 + d.1)) == want);
                if !steady {
                    continue;
                }
                let got = match ps.hit_test(p.0, p.1, 0.0) {
                    Some(PieHit::Slice(i)) => Some(i),
                    None => None,
                    Some(_) => continue,
                };
                assert_eq!(got, want, "{name}: at {p:?} (ring {ring})");
                checked += 1;
                if (ring > ps.r) == want.is_some() {
                    off_exact += 1;
                }
            }
        }
        assert!(checked > 10_000, "{name}: {checked}");
        // Not vacuous: the near-full slices are drawn off the exact disc.
        if name.contains("\"b\" : 0.5") {
            assert!(off_exact > 100, "{name}: {off_exact}");
        }
    }
}
