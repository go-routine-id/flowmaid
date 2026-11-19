
    // ── Report Q12 / P0 #15 / P1: the gitGraph look ─────────────────

    /// The comparison corpus diagram (`gitgraph.mmd`).
    const CORPUS: &str = "gitGraph
    commit id: \"init\"
    branch feat
    checkout feat
    commit id: \"a\"
    commit id: \"b\"
    checkout main
    commit id: \"c\"
    merge feat id: \"merge-feat\" tag: \"v1.0\"
    branch hotfix
    checkout hotfix
    commit id: \"fix\" type: HIGHLIGHT
    checkout main
    merge hotfix id: \"merge-hotfix\"
    commit id: \"release\" type: REVERSE tag: \"v1.1\"";

    /// Long ids and a long branch name (report P1 / missed).
    const LONG: &str = "gitGraph
    commit id: \"initial-scaffold\"
    commit id: \"add-authentication\"
    branch feature/login-page
    checkout feature/login-page
    commit id: \"login-form\"
    commit id: \"validate-input\"
    checkout main
    merge feature/login-page id: \"merge-login\" tag: \"v0.1-alpha\"
    commit id: \"release-candidate-final\"";

    /// Main merged down into a branch that moved on since it forked.
    const MERGE_DOWN: &str = "gitGraph
    commit id: \"c1\"
    branch develop
    checkout develop
    commit id: \"d1\"
    checkout main
    commit id: \"c2\"
    checkout develop
    commit id: \"d2\"
    merge main id: \"sync\"
    commit id: \"d3\"
    checkout main
    merge develop id: \"rel\" tag: \"v1.0\"";

    /// Lanes reordered with `order:`, typed merges, two forks from one
    /// commit.
    const ORDERED: &str = "gitGraph
    commit id: \"init\" tag: \"v0\"
    branch dev order: 2
    branch feat order: 1
    checkout dev
    commit id: \"d1\"
    checkout feat
    commit id: \"f1\"
    checkout main
    commit id: \"m1\"
    merge dev id: \"md\" type: HIGHLIGHT
    merge feat id: \"mf\" type: REVERSE tag: \"v2\"";

    /// Four lanes with crossings and default (numeric) ids.
    const MANY: &str = "gitGraph
    commit
    branch develop
    commit
    branch feature
    commit
    checkout develop
    commit
    branch hotfix
    commit
    checkout feature
    commit
    checkout main
    merge hotfix
    checkout develop
    merge feature
    commit
    checkout main
    merge develop tag: \"v2.0\"
    commit";

    /// Every case in every orientation.
    fn all_cases() -> Vec<(String, GitScene)> {
        let mut out = Vec::new();
        for src in [CORPUS, LONG, MERGE_DOWN, ORDERED, MANY] {
            for header in ["gitGraph", "gitGraph TB:", "gitGraph BT:"] {
                let src = src.replacen("gitGraph", header, 1);
                let gs = scene(&git(&src));
                out.push((src, gs));
            }
        }
        out
    }

    /// The drawn path of an edge, sampled every couple of px: the two
    /// legs and the true arc of an elbow, or the straight line.
    fn drawn(e: &SceneEdge) -> Vec<Pt> {
        let [p0, c, _, p1] = e.bezier;
        let corner = c != p0 && c != p1;
        let pts = if corner {
            elbow_points(p0, c, p1)
        } else {
            vec![p0, p1]
        };
        let mut out = vec![pts[0]];
        for w in pts.windows(2) {
            let k = (dist(w[0], w[1]) / 2.0).ceil().max(1.0) as usize;
            for j in 1..=k {
                let t = j as f64 / k as f64;
                out.push((
                    w[0].0 + (w[1].0 - w[0].0) * t,
                    w[0].1 + (w[1].1 - w[0].1) * t,
                ));
            }
        }
        out
    }

    fn inside(r: Rect, q: Pt, pad: f64) -> bool {
        q.0 > r.0 - pad && q.0 < r.2 + pad && q.1 > r.1 - pad && q.1 < r.3 + pad
    }

    fn overlap(a: Rect, b: Rect) -> bool {
        a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
    }

    /// Every text box drawn: ids, tag pills, name pills.
    fn label_boxes(gs: &GitScene) -> Vec<(String, Rect)> {
        let ids = gs.commit_ids.iter().map(|l| (l.text.clone(), text_box(l)));
        let tags = gs.tags.iter().map(|l| (format!("tag {}", l.text), tag_pill(l)));
        let names = gs
            .branch_labels
            .iter()
            .map(|l| (format!("branch {}", l.text), name_pill(l)));
        ids.chain(tags).chain(names).collect()
    }

    /// Report label-strikethrough: no fork / merge / branch line runs
    /// through a commit id, a tag or a branch name — in any
    /// orientation, with long ids and reordered lanes too.
    #[test]
    fn no_line_strikes_a_label() {
        for (src, gs) in all_cases() {
            let boxes = label_boxes(&gs);
            for e in &gs.scene.edges {
                for q in drawn(e) {
                    for (text, r) in &boxes {
                        assert!(
                            !inside(*r, q, EDGE_W / 2.0),
                            "{}->{} strikes {text:?} at {q:?}\n{src}",
                            e.from,
                            e.to
                        );
                    }
                }
            }
        }
    }

    /// Report P1 (long ids collide and clip, TB tags off canvas) and
    /// tag-top-margin: no two labels overlap, and everything drawn sits
    /// inside the canvas with margin to spare.
    #[test]
    fn labels_never_overlap_and_stay_on_canvas() {
        for (src, gs) in all_cases() {
            let boxes = label_boxes(&gs);
            for (i, (ta, a)) in boxes.iter().enumerate() {
                for (tb, b) in &boxes[i + 1..] {
                    assert!(!overlap(*a, *b), "{ta:?} overlaps {tb:?}\n{src}");
                }
                let canvas = (PAD / 2.0, PAD / 2.0, gs.scene.width, gs.scene.height);
                let canvas = (canvas.0, canvas.1, canvas.2 - PAD / 2.0, canvas.3 - PAD / 2.0);
                assert!(
                    a.0 >= canvas.0 && a.1 >= canvas.1 && a.2 <= canvas.2 && a.3 <= canvas.3,
                    "{ta:?} {a:?} leaves the {:.0}x{:.0} canvas\n{src}",
                    gs.scene.width,
                    gs.scene.height
                );
            }
            // Ids and tags clear every commit glyph.
            for n in &gs.scene.nodes {
                let dot = (n.x - R, n.y - R, n.x + R, n.y + R);
                for (t, b) in &boxes {
                    assert!(!overlap(dot, *b), "{t:?} covers commit {}\n{src}", n.id);
                }
            }
        }
    }

    /// A line only ever touches the commits it joins: a merge into a
    /// branch that committed since takes the free lane instead of
    /// running through those commits (report: mermaid reroutes too).
    #[test]
    fn lines_pass_no_other_commit() {
        for (src, gs) in all_cases() {
            for e in &gs.scene.edges {
                for n in &gs.scene.nodes {
                    if n.id == e.from || n.id == e.to {
                        continue;
                    }
                    for q in drawn(e) {
                        assert!(
                            dist(q, (n.x, n.y)) > R + EDGE_W / 2.0,
                            "{}->{} runs through {}\n{src}",
                            e.from,
                            e.to,
                            n.id
                        );
                    }
                }
            }
        }
        let gs = scene(&git(MERGE_DOWN));
        let sync = gs.scene.edges.iter().find(|e| e.to == "sync" && e.from == "c2");
        let sync = sync.expect("the c2 -> sync merge");
        // It runs along main (main's colour) and drops into sync.
        assert_eq!(sync.meta.style.stroke.as_deref(), Some(accent(0)));
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(sync.bezier[1], (node("sync").x, node("c2").y));
    }

    /// Report hockey-stick-curves / fork-overlaps-trunk: a fork drops
    /// straight from its parent (no stretch of it rides on the trunk)
    /// and a merge climbs straight into the merge commit, each through
    /// one rounded corner.
    #[test]
    fn forks_and_merges_are_rounded_elbows() {
        let gs = scene(&git(CORPUS));
        let edge = |a: &str, b: &str| {
            gs.scene
                .edges
                .iter()
                .find(|e| e.from == a && e.to == b)
                .unwrap_or_else(|| panic!("no {a}->{b}"))
        };
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();

        let fork = edge("init", "a");
        let (p0, p1) = (fork.bezier[0], fork.bezier[3]);
        assert_eq!(fork.bezier[1], fork.bezier[2], "one corner");
        assert_eq!(fork.bezier[1], (p0.0, p1.1), "the corner is below the parent");
        for q in drawn(fork) {
            // Nothing of the fork lies on the trunk outside the dot.
            if (q.1 - p0.1).abs() < EDGE_W {
                assert!(dist(q, p0) < R, "fork rides the trunk at {q:?}");
            }
        }
        let merge = edge("b", "merge-feat");
        let m = node("merge-feat");
        let c = merge.bezier[1];
        assert_eq!(c.1, merge.bezier[0].1, "runs along feat first");
        assert!((c.0 - m.x).abs() < R, "then climbs at the merge commit");
        // The SVG writes the corner as a true arc.
        let svg = to_svg(&gs);
        assert!(svg.contains(&format!(" A {ELBOW_R:.1} {ELBOW_R:.1} 0 0 ")), "{svg}");
        // Hosts spline through the waypoints (and picking measures that
        // spline): it must stay on the drawn elbow.
        for e in &gs.scene.edges {
            let path = drawn(e);
            for q in crate::scene::edge_polyline(e) {
                let near = path.iter().map(|&p| dist(p, q)).fold(f64::INFINITY, f64::min);
                assert!(near < 1.5, "{}->{}: spline leaves the elbow by {near}", e.from, e.to);
            }
        }
    }

    /// Two legs leaving one commit on the same side run side by side
    /// (merge-feat takes feat's merge from the left and forks hotfix to
    /// the right), never on top of each other.
    #[test]
    fn legs_sharing_a_commit_run_side_by_side() {
        let gs = scene(&git(CORPUS));
        let m = gs.scene.nodes.iter().find(|n| n.id == "merge-feat").unwrap();
        let leg_x = |a: &str, b: &str| {
            let e = gs.scene.edges.iter().find(|e| e.from == a && e.to == b).unwrap();
            e.bezier[1].0
        };
        let (merge_in, fork_out) = (leg_x("b", "merge-feat"), leg_x("merge-feat", "fix"));
        assert!(merge_in < m.x && m.x < fork_out, "{merge_in} {} {fork_out}", m.x);
        assert!((fork_out - merge_in - LEG_PITCH).abs() < 1e-9);
        assert!(fork_out - m.x < R && m.x - merge_in < R, "both start under the dot");
        // Two forks from one commit: the longer leg runs outside, so
        // the shorter one's turn never crosses it.
        let gs = scene(&git(ORDERED));
        let leg = |b: &str| {
            let e = gs.scene.edges.iter().find(|e| e.from == "init" && e.to == b).unwrap();
            e.bezier[1].0
        };
        assert!(leg("d1") < leg("f1"), "dev (farther lane) outside");
    }

    /// Report monochrome-edges / thin-strokes: every line takes the
    /// colour of the lane it runs along (a fork its new branch's, a
    /// merge its branch's) and is drawn 5px wide with round caps.
    #[test]
    fn lines_take_their_lane_colour() {
        let gs = scene(&git(CORPUS));
        let colour = |a: &str, b: &str| {
            let e = gs.scene.edges.iter().find(|e| e.from == a && e.to == b).unwrap();
            assert_eq!(e.meta.style.stroke_width, Some(EDGE_W));
            e.meta.style.stroke.clone().unwrap()
        };
        assert_eq!(colour("init", "c"), accent(0));
        assert_eq!(colour("init", "a"), accent(1));
        assert_eq!(colour("a", "b"), accent(1));
        assert_eq!(colour("b", "merge-feat"), accent(1));
        assert_eq!(colour("merge-feat", "fix"), accent(2));
        assert_eq!(colour("fix", "merge-hotfix"), accent(2));
        let svg = to_svg(&gs);
        for k in 0..3 {
            assert!(
                svg.contains(&format!("stroke=\"{}\" stroke-width=\"5.0\" stroke-linecap=\"round\"", accent(k))),
                "lane {k}: {svg}"
            );
        }
        assert!(!svg.contains(&format!("stroke=\"{EDGE_COLOR}\"")), "{svg}");
    }

    /// Report merge-arrow-not-emitted: the scene and the SVG agree —
    /// no arrowheads, as in mermaid.
    #[test]
    fn merges_carry_no_arrowhead() {
        let gs = scene(&git(CORPUS));
        assert!(gs.scene.edges.iter().all(|e| e.kind == EdgeKind::Open));
        assert!(!to_svg(&gs).contains("marker"));
    }

    /// Report P0 #15, no-merge-glyph, reverse-as-hollow,
    /// highlight-too-subtle: each commit kind has a glyph of its own.
    #[test]
    fn commit_kinds_have_distinct_glyphs() {
        let gs = scene(&git(CORPUS));
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(node("init").shape, Shape::Circle);
        assert_eq!(node("init").label, "");
        // A merge is a ring around a light core.
        assert_eq!(node("merge-feat").shape, Shape::DoubleCircle);
        // REVERSE: a filled disc crossed out — no longer a hollow ring
        // (which read as a merge).
        let rev = node("release");
        assert_eq!(rev.shape, Shape::Circle);
        assert_eq!(rev.label, REVERSE_MARK);
        assert_eq!(rev.style.fill.as_deref(), Some(accent(0)));
        // HIGHLIGHT: a square.
        assert_eq!(node("fix").shape, Shape::Rect);
        let svg = to_svg(&gs);
        let m = node("merge-feat");
        assert!(svg.contains(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{HOLE_FILL}\"/>",
            m.x,
            m.y,
            R * 0.45
        )));
        let k = R * 0.45;
        assert!(svg.contains(&format!("<path d=\"M {:.1} {:.1} L", rev.x - k, rev.y - k)));
        let f = node("fix");
        assert!(svg.contains(&format!("<rect x=\"{:.1}\" y=\"{:.1}\"", f.x - f.w / 2.0, f.y - f.h / 2.0)));
        // The type decides a merge's glyph, as in mermaid.
        let gs = scene(&git(ORDERED));
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(node("md").shape, Shape::Rect);
        assert_eq!(node("mf").label, REVERSE_MARK);
        // Picking matches the glyph: the square's corner is the commit.
        let sq = node("md");
        let corner = (sq.x + sq.w / 2.0 - 1.0, sq.y - sq.h / 2.0 + 1.0);
        let i = gs.scene.nodes.iter().position(|n| n.id == "md").unwrap();
        assert_eq!(gs.scene.node_at(corner.0, corner.1), Some(i));
    }

    /// Report tags-look-like-branch-names / no-lane-guides: tags are
    /// luggage labels, branch names pills, and every lane has a dashed
    /// guide.
    #[test]
    fn tags_pills_and_lane_guides() {
        for src in [CORPUS, &CORPUS.replacen("gitGraph", "gitGraph TB:", 1)] {
            let gs = scene(&git(src));
            let svg = to_svg(&gs);
            assert_eq!(svg.matches("stroke-dasharray=\"3 4\"").count(), 3, "{svg}");
            assert_eq!(svg.matches(" rx=\"5\"").count(), 3, "name pills: {svg}");
            assert_eq!(
                svg.matches(&format!("rx=\"{:.1}\" fill=\"{TAG_FILL}\"", TAG_H / 2.0)).count(),
                2,
                "tag pills: {svg}"
            );
            let tag = gs.tags.iter().find(|t| t.text == "v1.0").unwrap();
            assert_eq!(tag.font_size, TAG_FONT);
            assert_eq!(tag.font_weight, None, "tags are not branch-name bold");
        }
    }

    /// Report P1 / missed (long ids): ids are measured — two long ids
    /// in a row keep `TEXT_GAP` apart, the last one is not cut off, and
    /// a long branch name's pill pushes the first commit right instead
    /// of running into it.
    #[test]
    fn long_ids_and_names_get_room() {
        let gs = scene(&git(LONG));
        let id = |t: &str| text_box(gs.commit_ids.iter().find(|l| l.text == t).unwrap());
        let (a, b) = (id("initial-scaffold"), id("add-authentication"));
        assert!(b.0 - a.2 >= TEXT_GAP - 1e-9, "{a:?} {b:?}");
        let last = id("release-candidate-final");
        assert!(last.2 <= gs.scene.width - PAD + 1e-9, "{last:?} {}", gs.scene.width);
        let pill = name_pill(&gs.branch_labels[1]);
        let first = &gs.scene.nodes[0];
        assert!(pill.2 + GAP <= first.x - R, "{pill:?} vs {}", first.x);
        // Short ids pack tighter than the old fixed 90px pitch.
        let gs = scene(&git("gitGraph\ncommit id: \"a\"\ncommit id: \"b\""));
        let (n0, n1) = (&gs.scene.nodes[0], &gs.scene.nodes[1]);
        assert_eq!(n1.x - n0.x, COMMIT_DX);
    }

    /// Report P1: in TB/BT a tag on the first lane used to hang off the
    /// left edge (x = 14, text-anchor end). Tags now sit right of their
    /// commit and the canvas fits around every label.
    #[test]
    fn tb_tags_stay_on_canvas() {
        let src = "gitGraph TB:\ncommit id: \"init\" tag: \"v0.1-alpha\"\ncommit id: \"two\"";
        for src in [src.to_string(), src.replacen("TB:", "BT:", 1)] {
            let gs = scene(&git(&src));
            let t = tag_pill(&gs.tags[0]);
            let n = &gs.scene.nodes[0];
            assert!(t.0 > n.x + R, "tag right of its commit");
            assert!(t.0 >= PAD - 1e-9 && t.2 <= gs.scene.width - PAD + 1e-9, "{t:?}");
        }
    }

    /// `branch x order: n` moves lanes (and colours follow the lane).
    #[test]
    fn branch_order_moves_lanes() {
        let gs = scene(&git(ORDERED));
        let y = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap().y;
        assert!(y("init") < y("f1") && y("f1") < y("d1"), "feat (order 1) above dev (order 2)");
        let fill = |id: &str| {
            let n = gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
            n.style.fill.clone().unwrap()
        };
        assert_eq!(fill("f1"), accent(1));
        assert_eq!(fill("d1"), accent(2));
    }

    /// The generic scene stays a working interactive scene: commits
    /// and elbows pick where they are drawn.
    #[test]
    fn scene_picks_commits_and_elbows() {
        let gs = scene(&git(CORPUS));
        let s = &gs.scene;
        for (i, n) in s.nodes.iter().enumerate() {
            assert_eq!(s.hit_test(n.x, n.y, 3.0), Some(crate::scene::Hit::Node(i)));
        }
        let k = s.edges.iter().position(|e| e.from == "init" && e.to == "a").unwrap();
        let e = &s.edges[k];
        // Halfway down the fork's leg.
        let q = (e.bezier[0].0, (e.bezier[0].1 + e.bezier[3].1) / 2.0);
        assert_eq!(s.hit_test(q.0, q.1, 3.0), Some(crate::scene::Hit::Edge(k)));
    }

    #[test]
    fn rendering_is_deterministic() {
        for (src, gs) in all_cases() {
            assert_eq!(to_svg(&gs), to_svg(&scene(&git(&src))));
        }
    }

    /// A hand-built graph whose `order`s skip and repeat still packs
    /// its lanes, and an empty one renders.
    #[test]
    fn hand_built_orders_pack() {
        let mut g = git("gitGraph\ncommit\nbranch a\ncommit\nbranch b\ncommit");
        g.branches[1].order = 40;
        g.branches[2].order = 7;
        let gs = scene(&g);
        let ys: Vec<f64> = gs.scene.nodes.iter().map(|n| n.y).collect();
        assert!(ys[0] < ys[2] && ys[2] < ys[1], "{ys:?}");
        assert!(ys[1] - ys[0] <= 2.0 * BRANCH_DY + 1e-9);
        let svg = to_svg(&scene(&GitGraph::default()));
        assert!(svg.contains(">main</text>"));
    }
