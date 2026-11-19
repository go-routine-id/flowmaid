    /// `pts` sampled `steps` per command of the path (start, commands).
    fn dense_path(start: (f64, f64), segs: &[Seg], steps: usize) -> Vec<(f64, f64)> {
        let mut out = vec![start];
        let mut cur = start;
        for seg in segs {
            let b = seg.cubic(cur);
            out.extend((1..=steps).map(|k| cubic_at(&b, k as f64 / steps as f64)));
            cur = seg.end();
        }
        out
    }

    /// How far the squared end at `tip` (heading `d` into it) strays from
    /// the scene curve `plain`: the farthest point of `painted` from it,
    /// over the points outside the arrowhead (half stroke `hw`, `None`
    /// for a cross or a circle), as (beyond `near` of the tip, within).
    fn end_stray(
        painted: &[(f64, f64)],
        plain: &[(f64, f64)],
        tip: (f64, f64),
        d: (f64, f64),
        hw: Option<f64>,
        near: f64,
    ) -> (f64, f64) {
        let reach = 4.0 * near;
        let close = |q: &&(f64, f64)| (q.0 - tip.0).hypot(q.1 - tip.1) <= reach;
        let line: Vec<(f64, f64)> = plain.iter().filter(close).copied().collect();
        let mut out = (0.0f64, 0.0f64);
        for &q in painted.iter().filter(close) {
            let along = (tip.0 - q.0) * d.0 + (tip.1 - q.1) * d.1;
            let lat = ((q.0 - tip.0) * d.1 - (q.1 - tip.1) * d.0).abs();
            if hw.is_some_and(|hw| (0.0..=ARROW_MARKER_PX).contains(&along) && lat + hw <= along / 2.0)
            {
                continue;
            }
            let off = line
                .iter()
                .map(|p| (p.0 - q.0).hypot(p.1 - q.1))
                .fold(f64::INFINITY, f64::min);
            if (q.0 - tip.0).hypot(q.1 - tip.1) > near {
                out.0 = out.0.max(off);
            } else {
                out.1 = out.1.max(off);
            }
        }
        out
    }

    #[test]
    fn a_turning_curve_runs_straight_into_its_end_glyph_near_its_scene_curve() {
        // A fan: the outer branches are S-curves whose cubic still turns
        // under the head. The painter squares each end to the head: the
        // head faces the cubic's own end tangent (plumb onto the top side)
        // with its tip on the end, the stroke cut back to that tangent;
        // where bending into a straight run keeps within SQUARE_END_DEV of
        // the cubic, the stroke runs a whole head straight into the head.
        // Nowhere farther than two heads from the tip does the painted
        // curve stray more than that — the end point stays put, unlike a
        // stub that moves it and drags the whole tail along — and the
        // scene keeps its cubic (a host draws it).
        let g = parse("flowchart TD\nA --> B & C & D & E & F\n").unwrap();
        let sc = scene(&g);
        let mut full = 0;
        for e in &sc.edges {
            let (tip, heading) = edge_end_tangent(e, false);
            assert_eq!(tip, e.bezier[3]);
            assert!(heading.0.abs() < 1e-9 && heading.1 > 0.0, "{heading:?}");
            assert_eq!(e.bezier[2].0, e.bezier[3].0, "the scene's cubic");
            let (start, segs) = drawn_segs(e, |p| p);
            assert_eq!(segs.last().map(|s| s.end()), Some(tip));
            let (_, cut) = trim_segs(start, segs.clone(), 0.0, ARROW_TRIM);
            let end = cut.last().unwrap().end();
            assert!((end.0 - tip.0).abs() < 1e-9, "{end:?}");
            assert!((tip.1 - end.1 - ARROW_TRIM).abs() < 1e-6, "{end:?}");
            // How far the drawn curve runs plumb into the tip.
            let pts = dense_path(start, &segs, 64);
            let run = pts
                .iter()
                .rev()
                .take_while(|q| (q.0 - tip.0).abs() < 1e-6)
                .last()
                .map_or(0.0, |q| tip.1 - q.1);
            assert!(run >= ARROW_TRIM + SQUARE_END_ENTRY - 1e-6, "{run}");
            full += usize::from(run >= GLYPH_STUB - 1e-6);
            let plain = sample_cubic(&e.bezier, 400);
            let (far, near) = end_stray(
                &pts,
                &plain,
                tip,
                heading,
                Some(edge_stroke_width(e) / 2.0),
                SQUARE_END_ENTRY_REACH,
            );
            assert!(far <= SQUARE_END_DEV + 0.05, "{}->{}: {far}", e.from, e.to);
            assert!(near <= ARROW_MARKER_PX / 2.0, "{}->{}: {near}", e.from, e.to);
        }
        // The spine and its neighbours (gentle turns) get the whole head;
        // the outermost swing in too hard for it within a px.
        assert!((3..sc.edges.len()).contains(&full), "{full}");
    }

    #[test]
    fn a_head_faces_the_end_tangent_where_the_final_straight_is_shorter_than_the_cut() {
        // B1 review #1: a spline whose closing straight (a sixth of its
        // last waypoint leg, here 16 / 6 px) is shorter than the cut under
        // the head. Cutting the drawn curve by arc length used to end the
        // stroke on the bend before it, so `orient="auto"` tilted the head
        // ~15° off the side (subgraph.mmd routed, C1/C2 -> Store). The
        // squared end runs along the end tangent past the cut: the head is
        // square, its tip on the end.
        let wps = vec![(0.0, 0.0), (60.0, 0.0), (100.0, 40.0), (100.0, 56.0)];
        let e = routed_edge((0.0, 0.0), (100.0, 56.0), wps.clone());
        let (tip, head) = edge_end_tangent(&e, false);
        assert_eq!(tip, (100.0, 56.0));
        assert!(head.0.abs() < 1e-9 && (head.1 - 1.0).abs() < 1e-9, "{head:?}");
        let (start, segs) = drawn_segs(&e, |p| p);
        let (_, cut) = trim_segs(start, segs, 0.0, ARROW_TRIM);
        let end = cut.last().unwrap().end();
        assert!((end.0 - 100.0).abs() < 1e-9 && (56.0 - ARROW_TRIM - end.1).abs() < 1e-6);
        // The scene's waypoints are untouched: a host draws them.
        assert_eq!(e.waypoints, wps);
        // The plain cut, for the record: its stroke heads off square.
        let (s0, g0) = plain_segs(&e, |p| p);
        let (_, plain_cut) = trim_segs(s0, g0, 0.0, ARROW_TRIM);
        let Some(&Seg::C(_, c2, p)) = plain_cut.last() else {
            panic!("{plain_cut:?}")
        };
        let tilt = (p.0 - c2.0).atan2(p.1 - c2.1).to_degrees().abs();
        assert!(tilt > 10.0, "{tilt}");
    }

    #[test]
    fn squared_ends_face_the_curve_and_keep_near_it_on_every_fixture() {
        // Report §2.3 / M1, B1 review #1 and #2: on every flowchart and
        // state fixture, static and routed, each end glyph faces along its
        // scene curve's own end tangent and an arrowhead's stroke stops
        // exactly its cut back on that tangent (the tip on the end, never
        // tilted by a cut into a curve still turning); outside the head the
        // painted curve keeps within SQUARE_END_DEV of the scene curve the
        // routing, port and label stages spaced, beyond two heads of the
        // tip — and within half a head nearer, where a curve swinging in
        // from the side enters the head.
        let root = env!("CARGO_MANIFEST_DIR");
        let mut files: Vec<std::path::PathBuf> = ["tests/fixtures/quality", "examples"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{root}/{d}")).unwrap())
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1) < 1e-6;
        let (mut ends, mut squared) = (0, 0);
        for f in &files {
            use crate::model::Document;
            let src = std::fs::read_to_string(f).unwrap();
            let g = match crate::parser::parse_document(&src) {
                Ok(Document::Flowchart(g) | Document::State(g)) => g,
                _ => continue,
            };
            let st = scene(&g);
            let centres: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = route(&g, &centres);
            for sc in [&st, &rt] {
                for e in sc.edges.iter().filter(|e| edge_drawn(e) && e.from != e.to) {
                    let (s0, g0) = plain_segs(e, |p| p);
                    let (s1, g1) = drawn_segs(e, |p| p);
                    let trims = arrow_trims(e);
                    let (plain, painted) = (dense_path(s0, &g0, 48), dense_path(s1, &g1, 48));
                    for (at_start, m, trim) in [
                        (true, e.ends.start_marker(e.kind), trims.0),
                        (false, e.ends.end_marker(e.kind), trims.1),
                    ] {
                        let Some(m) = m else { continue };
                        ends += 1;
                        let (rs, rg) = reverse_segs(s0, &g0);
                        let (from, last) = if at_start {
                            let n = rg.len();
                            (if n > 1 { rg[n - 2].end() } else { rs }, rg[n - 1])
                        } else {
                            let n = g0.len();
                            (if n > 1 { g0[n - 2].end() } else { s0 }, g0[n - 1])
                        };
                        let Some(d) = end_heading(from, last) else { continue };
                        let (tip, head) = edge_end_tangent(e, at_start);
                        let what = format!("{f:?} {}->{} start={at_start}", e.from, e.to);
                        assert!(near(head, d), "{what}: {head:?} vs {d:?}");
                        if trim > 0.0 {
                            let (s, g) = drawn_segs(e, |p| p);
                            let (s, g) = trim_segs(s, g, trims.0, trims.1);
                            let end = if at_start { s } else { g.last().unwrap().end() };
                            let want = (tip.0 - d.0 * trim, tip.1 - d.1 * trim);
                            // A curve too short to lose the cut is stroked whole.
                            if !near(end, tip) {
                                assert!(near(end, want), "{what}: {end:?} vs {want:?}");
                            }
                        }
                        let hw = (m == Marker::Arrow).then(|| edge_stroke_width(e) / 2.0);
                        let (far, close) =
                            end_stray(&painted, &plain, tip, d, hw, SQUARE_END_ENTRY_REACH);
                        squared += usize::from(far + close > 0.0);
                        assert!(far <= SQUARE_END_DEV + 0.1, "{what}: {far}");
                        assert!(close <= ARROW_MARKER_PX / 2.0, "{what}: {close}");
                    }
                }
            }
        }
        assert!(ends > 500 && squared > 100, "{ends} ends, {squared} squared");
    }

    #[test]
    fn a_wide_stroke_stops_its_width_short_under_its_own_head() {
        // B1 review #4: a `linkStyle` stroke wider than the usual cut
        // showed its butt end's shoulders beside the head. Its stroke stops
        // its own width short (a head is as wide as it is long), its head
        // set that much further ahead so the tip stays on the border — the
        // same 11 px glyph, in a variant marker.
        let src = "flowchart LR\nA --> B\nC --> D\nlinkStyle 1 stroke-width:8px\n";
        let sc = scene(&parse(src).unwrap());
        assert_eq!(arrow_trims(&sc.edges[0]), (0.0, ARROW_TRIM));
        assert_eq!(arrow_trims(&sc.edges[1]), (0.0, 8.0));
        let svg = to_svg(&sc);
        let paths = edge_paths(&svg);
        assert!(paths[0].contains("marker-end=\"url(#arrow)\""), "{}", paths[0]);
        assert!(paths[1].contains("marker-end=\"url(#arrow-t8)\""), "{}", paths[1]);
        assert!(svg.contains(
            "<marker id=\"arrow-t8\" viewBox=\"0 0 10 10\" refX=\"2.727\" refY=\"5\" \
             markerUnits=\"userSpaceOnUse\" markerWidth=\"11\" markerHeight=\"11\""
        ));
        let mut bb = Bbox::new();
        grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (minx, _, _, _) = bb.finish();
        let d = &sc.nodes[3];
        let left = d.x - d.w / 2.0 + SVG_MARGIN - minx;
        let pts = painted_points(paths[1]);
        let end = pts[pts.len() - 1];
        assert!((left - end.0 - 8.0).abs() < 0.11, "{end:?} vs {left}");
        // Up to a whole head; a hairline keeps the usual cut.
        assert_eq!(arrow_trim(30.0), ARROW_MARKER_PX);
        assert_eq!(arrow_trim(1.0), ARROW_TRIM);
    }

