
    /// A point `off` scene units to the left of edge `e`'s drawn curve,
    /// at its middle sample.
    fn beside_mid(e: &SceneEdge, off: f64) -> (f64, f64) {
        let pts = edge_polyline(e);
        let k = pts.len() / 2;
        let (a, b) = (pts[k - 1], pts[k]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let (nx, ny) = (-(b.1 - a.1) / len, (b.0 - a.0) / len);
        let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        (m.0 + nx * off, m.1 + ny * off)
    }

    #[test]
    fn edge_at_covers_the_painted_stroke_not_just_its_centre_line() {
        // A thick edge is 3.4 wide and a `linkStyle` one 10: a point on
        // the painted stroke but off its centre line picks the edge
        // even at zero tolerance, and a point just past the stroke does
        // not.
        let g = parse("flowchart LR\nA ==> B\nC --> D\nlinkStyle 1 stroke-width:10px").unwrap();
        let s = scene(&g);
        let thick = beside_mid(&s.edges[0], EDGE_WIDTH_THICK / 2.0 - 0.1);
        assert_eq!(s.edge_at(thick.0, thick.1, 0.0), Some(0));
        let past = beside_mid(&s.edges[0], EDGE_WIDTH_THICK / 2.0 + 0.1);
        assert_eq!(s.edge_at(past.0, past.1, 0.0), None);
        let wide = beside_mid(&s.edges[1], 4.9);
        assert_eq!(s.edge_at(wide.0, wide.1, 0.0), Some(1));
        assert_eq!(s.hit_test(wide.0, wide.1, 0.0), Some(Hit::Edge(1)));
    }

    #[test]
    fn a_styled_invisible_link_is_picked_because_it_is_painted() {
        // `~~~` with a `linkStyle` stroke width is drawn (edge_drawn), so
        // it must be pickable and marquee-selectable like any edge; the
        // unstyled `~~~` beside it stays unpickable.
        let g = parse("flowchart LR\nA ~~~ B\nC ~~~ D\nlinkStyle 0 stroke-width:2px").unwrap();
        let s = scene(&g);
        let svg = to_svg(&s);
        assert!(svg.contains(&format!("<path d=\"{}\"", edge_path_d(&s.edges[0], |p| p).split(' ').next().unwrap())));
        let p = beside_mid(&s.edges[0], 0.0);
        assert_eq!(s.edge_at(p.0, p.1, 0.0), Some(0));
        let q = beside_mid(&s.edges[1], 0.0);
        assert_eq!(s.edge_at(q.0, q.1, 4.0), None);
        let all = s.hits_in_rect(-1e6, -1e6, 1e6, 1e6);
        assert!(all.contains(&Hit::Edge(0)) && !all.contains(&Hit::Edge(1)));
    }

    /// Points along an edge path `d` as the edge painters emit it
    /// (`M`/`L`/`Q`/`C`, absolute), no further apart than `step` on
    /// straight runs and finely sampled on curves.
    fn path_d_points(d: &str, step: f64) -> Vec<(f64, f64)> {
        let toks: Vec<&str> = d.split(|c: char| c == ' ' || c == ',').filter(|t| !t.is_empty()).collect();
        let num = |i: usize| toks[i].parse::<f64>().unwrap();
        let (mut out, mut cur, mut i) = (Vec::new(), (0.0, 0.0), 0);
        while i < toks.len() {
            let pts: Vec<(f64, f64)> = match toks[i] {
                "M" | "L" => vec![(num(i + 1), num(i + 2))],
                "Q" => vec![(num(i + 1), num(i + 2)), (num(i + 3), num(i + 4))],
                "C" => vec![(num(i + 1), num(i + 2)), (num(i + 3), num(i + 4)), (num(i + 5), num(i + 6))],
                other => panic!("unexpected path command {other:?} in {d}"),
            };
            let cmd = toks[i];
            i += 1 + 2 * pts.len();
            let end = pts[pts.len() - 1];
            match cmd {
                "M" => out.push(end),
                "L" => {
                    let n = ((end.0 - cur.0).hypot(end.1 - cur.1) / step).ceil().max(1.0) as usize;
                    out.extend((1..=n).map(|k| {
                        let t = k as f64 / n as f64;
                        (cur.0 + (end.0 - cur.0) * t, cur.1 + (end.1 - cur.1) * t)
                    }));
                }
                _ => {
                    let b = if cmd == "Q" {
                        // Degree-elevate the quadratic to the same cubic.
                        let c = pts[0];
                        [cur, (cur.0 + 2.0 * (c.0 - cur.0) / 3.0, cur.1 + 2.0 * (c.1 - cur.1) / 3.0),
                         (end.0 + 2.0 * (c.0 - end.0) / 3.0, end.1 + 2.0 * (c.1 - end.1) / 3.0), end]
                    } else {
                        [cur, pts[0], pts[1], end]
                    };
                    out.extend((1..=64).map(|k| cubic_at(&b, k as f64 / 64.0)));
                }
            }
            cur = end;
        }
        out
    }

    #[test]
    fn edge_picking_matches_the_painted_strokes_on_every_flowchart_fixture() {
        // pick == render, end to end: for every flowchart / state fixture,
        // walk each painted edge along the path `d` that `to_svg` actually
        // writes (parsed back out of the SVG's own numbers, not through
        // `edge_polyline`), and at every point outside the nodes and the
        // label chips `edge_at` must answer that edge — or another edge
        // whose painted stroke covers the same point (a shared trunk, a
        // crossing). A middle routed waypoint or a bezier midpoint is NOT
        // such a point: the spline through the waypoints cuts corners.
        const TOL: f64 = 0.0;
        let root = env!("CARGO_MANIFEST_DIR");
        let mut files: Vec<std::path::PathBuf> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        let (mut graphs, mut probes, mut worst) = (0, 0, 0.0f64);
        for f in &files {
            let src = std::fs::read_to_string(f).unwrap();
            let g = match crate::parser::parse_document(&src) {
                Ok(crate::model::Document::Flowchart(g) | crate::model::Document::State(g)) => g,
                _ => continue,
            };
            graphs += 1;
            let s = scene(&g);
            let svg = to_svg(&s);
            // Undo to_svg's margin shift to get back to scene units.
            let mut bb = Bbox::new();
            grow_scene(&mut bb, &s.nodes, &s.edges, &s.clusters);
            let (minx, _, miny, _) = bb.finish();
            let (tx, ty) = (MARGIN - minx, MARGIN - miny);
            let curves: Vec<Option<Vec<(f64, f64)>>> = s
                .edges
                .iter()
                .map(|e| {
                    edge_drawn(e).then(|| {
                        let d = edge_path_d(e, |p| (p.0 + tx, p.1 + ty));
                        assert!(svg.contains(&format!("<path d=\"{d}\"")), "{f:?}: {d} not painted");
                        path_d_points(&d, 2.0).into_iter().map(|p| (p.0 - tx, p.1 - ty)).collect()
                    })
                })
                .collect();
            let covers = |j: usize, p: (f64, f64)| {
                let c = curves[j].as_ref().expect("picked edge is painted");
                let d = c.windows(2).map(|w| point_seg_dist(p.0, p.1, w[0], w[1])).fold(f64::INFINITY, f64::min);
                d - edge_stroke_width(&s.edges[j]) / 2.0
            };
            let in_chip = |p: (f64, f64)| {
                s.edges.iter().any(|e| {
                    e.label.as_ref().is_some_and(|(text, m, w)| {
                        let lines = 1 + text.matches("<br").count() + text.matches('\n').count();
                        (p.0 - m.0).abs() <= w / 2.0 + 2.0
                            && (p.1 - m.1).abs() <= LABEL_BOX_H / 2.0 + LINE_H * (lines - 1) as f64 + 2.0
                    })
                })
            };
            for (i, c) in curves.iter().enumerate() {
                let Some(c) = c else { continue };
                for &p in c {
                    if s.node_at(p.0, p.1).is_some() || in_chip(p) {
                        continue;
                    }
                    probes += 1;
                    let got = s.edge_at(p.0, p.1, TOL);
                    // How far off the painted stroke the pick lands (<= 0: on it).
                    let off = match got {
                        Some(j) if j == i => 0.0f64.min(covers(i, p)),
                        Some(j) => covers(j, p),
                        None => covers(i, p),
                    };
                    worst = worst.max(off);
                    assert!(
                        got.is_some() && off <= 1e-6,
                        "{f:?}: edge {i} ({} -> {}) at {p:?} picked {got:?}, {off:.3} off the stroke",
                        s.edges[i].from,
                        s.edges[i].to
                    );
                }
            }
        }
        assert!(graphs >= 10 && probes > 1000, "{graphs} graphs, {probes} probes");
        let _ = worst;
    }
