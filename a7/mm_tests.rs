
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn mind(src: &str) -> Mindmap {
        match parse_document(src).unwrap() {
            Document::Mindmap(m) => m,
            _ => panic!("expected a mindmap"),
        }
    }

    fn showcase() -> Mindmap {
        mind(include_str!("../examples/mindmap.mmd"))
    }

    fn by_text<'a>(ms: &'a MindScene, text: &str) -> &'a MindNodeBox {
        ms.nodes.iter().find(|n| n.text == text).expect(text)
    }

    #[test]
    fn builds_tree_from_indentation() {
        let m = mind("mindmap\n  root((Root))\n    A\n      A1\n    B\n");
        assert_eq!(m.nodes.len(), 4);
        assert_eq!(m.nodes[0].text, "Root");
        assert_eq!(m.nodes[0].shape, MindShape::Circle);
        assert_eq!(m.nodes[0].children.len(), 2);
        let a = m.nodes[0].children[0];
        assert_eq!(m.nodes[m.nodes[a].children[0]].text, "A1");
    }

    #[test]
    fn shapes_and_br_parse() {
        let m = mind("mindmap\nroot\n  [sq]\n  (round)\n  ((circ))\n  {{hex}}\n  a<br/>b\n");
        let shapes: Vec<MindShape> = m.nodes[1..].iter().map(|n| n.shape).collect();
        assert_eq!(
            shapes,
            [
                MindShape::Square,
                MindShape::Rounded,
                MindShape::Circle,
                MindShape::Hexagon,
                // No wrapper: mermaid's default node, not a pill.
                MindShape::Default,
            ]
        );
        assert_eq!(m.nodes.last().unwrap().text, "a\nb");
    }

    #[test]
    fn multiword_text_before_paren_stays_literal() {
        // An id can't contain a space, so "call foo(x)" is plain text,
        // not a rounded node labeled "x".
        let m = mind("mindmap\nRoot\n  call foo(x)\n  id(real)\n");
        assert_eq!(m.nodes[1].text, "call foo(x)");
        assert_eq!(m.nodes[1].shape, MindShape::Default);
        // A genuine adjacent id still strips.
        assert_eq!(m.nodes[2].text, "real");
        assert_eq!(m.nodes[2].shape, MindShape::Rounded);
    }

    #[test]
    fn radial_layout_stays_inside_canvas() {
        let ms = scene(&mind(
            "mindmap\n  root((Order))\n    Customer\n      Buat Order\n      Lihat Order\n    \
             Owner\n      Kelola\n      State Machine\n    Sistem\n",
        ));
        assert!(ms.width > 0.0 && ms.height > 0.0);
        for n in &ms.nodes {
            assert!(n.x >= -0.01 && n.y >= -0.01, "node top-left inside");
            assert!(n.x + n.w <= ms.width + 0.01, "node right inside canvas");
            assert!(n.y + n.h <= ms.height + 0.01, "node bottom inside canvas");
        }
        assert_eq!(ms.edges.len(), ms.nodes.len() - 1);
    }

    #[test]
    fn root_sits_near_the_centre() {
        let ms = scene(&mind("mindmap\nroot((R))\n  A\n  B\n  C\n  D\n"));
        let root = &ms.nodes[0];
        // Children radiate around the root, so it lands near the middle.
        assert!((root.cx() - ms.width / 2.0).abs() < ms.width * 0.28);
        assert!((root.cy() - ms.height / 2.0).abs() < ms.height * 0.28);
    }

    fn assert_no_overlap(ms: &MindScene) {
        let boxes: Vec<(f64, f64, f64, f64)> = ms
            .nodes
            .iter()
            .map(|n| (n.x, n.y, n.x + n.w, n.y + n.h))
            .collect();
        for i in 0..boxes.len() {
            for j in (i + 1)..boxes.len() {
                let (a, b) = (boxes[i], boxes[j]);
                let overlap = a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
                assert!(
                    !overlap,
                    "{:?} ({a:?}) overlaps {:?} ({b:?})",
                    ms.nodes[i].text, ms.nodes[j].text
                );
            }
        }
    }

    /// Connector segments (centre to centre) never cross each other.
    fn assert_no_crossing(ms: &MindScene) {
        let segs: Vec<((f64, f64), (f64, f64))> =
            ms.edges.iter().map(|e| (e.bezier[0], e.bezier[3])).collect();
        for (i, a) in segs.iter().enumerate() {
            for b in &segs[i + 1..] {
                assert!(
                    !segments_cross(a.0, a.1, b.0, b.1),
                    "connectors {a:?} and {b:?} cross"
                );
            }
        }
    }

    #[test]
    fn wide_siblings_do_not_overlap() {
        // Many long labels round the root: every box keeps its own cone.
        let mut src = String::from("mindmap\n  root(Center)\n");
        for i in 0..12 {
            src.push_str(&format!("    A very long branch label number {i:02}\n"));
        }
        let ms = scene(&mind(&src));
        assert_no_overlap(&ms);
        assert_no_crossing(&ms);
    }

    #[test]
    fn to_svg_draws_shapes_and_links() {
        let svg = to_svg(&scene(&mind(
            "mindmap\nroot((R))\n  A\n  ))boom((\n  )fog(\n  {{hx}}\n",
        )));
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("<circle"), "circle root");
        assert!(svg.contains("<polygon"), "bang/cloud/hexagon polygons");
        assert!(svg.contains("<path"), "connectors");
        assert!(svg.contains("fill=\"#ffffff\""), "root and indigo text is white");
    }

    #[test]
    fn route_follows_dragged_positions() {
        let m = mind("mindmap\nroot((R))\n  A\n  B\n");
        let auto = scene(&m);
        // Seed from auto, then "drag" node A far to the right.
        let mut pos: Vec<(f64, f64)> = auto.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        pos[1] = (pos[1].0 + 500.0, pos[1].1);
        let routed = route(&m, &pos);
        // Node A's box moved with it; the root→A connector's end follows.
        assert!((routed.nodes[1].cx() - pos[1].0).abs() < 0.01);
        let a_edge = &routed.edges[0];
        assert!((a_edge.bezier[3].0 - pos[1].0).abs() < routed.nodes[1].w);
        // Canvas grew to keep the dragged node inside.
        assert!(routed.width > auto.width);
        // Re-routing the auto layout's own centres reproduces it exactly.
        let same: Vec<(f64, f64)> = auto.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        assert_eq!(to_svg(&route(&m, &same)), to_svg(&auto));
    }

    #[test]
    fn bang_and_cloud_have_perimeters() {
        let ms = scene(&mind("mindmap\nR\n  ))b((\n  )c(\n"));
        let bang = ms
            .nodes
            .iter()
            .find(|n| n.shape == MindShape::Bang)
            .unwrap();
        let cloud = ms
            .nodes
            .iter()
            .find(|n| n.shape == MindShape::Cloud)
            .unwrap();
        assert!(perimeter(bang).unwrap().len() >= 12);
        assert!(perimeter(cloud).unwrap().len() >= 12);
        // A default box has no polygon perimeter.
        assert!(perimeter(&ms.nodes[0]).is_none());
    }

    /// Report Q5: branch nodes hard-coded white text on their accent
    /// (amber 2.6:1, green 3.2:1); the scene's per-node `text_color`
    /// (what host painters use) must agree with the SVG.
    #[test]
    fn branch_text_reads_on_every_accent() {
        let n = crate::style::ACCENTS.len();
        let mut src = String::from("mindmap\nroot((R))\n");
        for i in 0..n {
            src.push_str(&format!("  Br{i}\n    Leaf{i}\n"));
        }
        let ms = scene(&mind(&src));
        let svg = to_svg(&ms);
        for i in 0..n {
            let accent = crate::style::accent(i);
            for label in [format!("Br{i}"), format!("Leaf{i}")] {
                let ink = crate::style::assert_text_reads(&svg, &label, accent);
                assert_eq!(ink, crate::style::text_on(accent), "{label}");
                let node = ms.nodes.iter().find(|b| b.text == label).unwrap();
                assert_eq!((node.fill, node.text_color), (accent, ink));
            }
        }
    }

    /// Visible connector length: border to border along the line
    /// between the two centres.
    fn visible_spokes(ms: &MindScene) -> Vec<f64> {
        ms.edges
            .iter()
            .map(|e| {
                let (a, b) = (e.bezier[0], e.bezier[3]);
                let len = (b.0 - a.0).hypot(b.1 - a.1);
                let u = ((b.0 - a.0) / len, (b.1 - a.1) / len);
                let at = |p: (f64, f64)| {
                    ms.nodes
                        .iter()
                        .find(|n| (n.cx() - p.0).abs() < 1e-6 && (n.cy() - p.1).abs() < 1e-6)
                        .unwrap()
                };
                let (pa, pb) = (at(a), at(b));
                len - ext(pa.w, pa.h, u) - ext(pb.w, pb.h, u)
            })
            .collect()
    }

    /// Report B6 (sparse-oversized-canvas, uneven-long-leaf-spokes,
    /// leaf-fan-converges): fixed rings put every depth on one circle
    /// round the root — 9% ink on a 909×773 canvas, leaf spokes 55-223px.
    #[test]
    fn showcase_is_compact_with_even_spokes() {
        let ms = scene(&showcase());
        assert_no_overlap(&ms);
        assert_no_crossing(&ms);
        let spokes = visible_spokes(&ms);
        let (lo, hi) = spokes
            .iter()
            .fold((f64::MAX, 0.0f64), |(lo, hi), &s| (lo.min(s), hi.max(s)));
        assert!(lo >= SPOKE - 0.01, "a spoke shorter than SPOKE: {spokes:?}");
        assert!(hi / lo <= 2.0, "spokes {lo:.0}..{hi:.0}: {spokes:?}");
        // Ink: the node boxes alone cover a fifth of the canvas (the old
        // ring layout: under a tenth).
        let area: f64 = ms.nodes.iter().map(|n| n.w * n.h).sum();
        let fill = area / (ms.width * ms.height);
        assert!(fill >= 0.2, "boxes cover only {:.0}%", fill * 100.0);
        // Every leaf sits nearer its own parent than any other branch.
        let m = showcase();
        for (i, n) in m.nodes.iter().enumerate() {
            let Some(p) = n.parent.filter(|_| n.depth >= 2) else {
                continue;
            };
            let d = |j: usize| {
                (ms.nodes[i].cx() - ms.nodes[j].cx()).hypot(ms.nodes[i].cy() - ms.nodes[j].cy())
            };
            for &b in &m.nodes[0].children {
                if b != p {
                    assert!(d(p) < d(b), "{} drifts toward {}", n.text, m.nodes[b].text);
                }
            }
        }
    }

    #[test]
    fn branches_run_clockwise_from_twelve_in_source_order() {
        let m = showcase();
        let ms = scene(&m);
        let root = &ms.nodes[0];
        let angles: Vec<f64> = m.nodes[0]
            .children
            .iter()
            .map(|&c| {
                let a = (ms.nodes[c].cy() - root.cy()).atan2(ms.nodes[c].cx() - root.cx());
                // Measured clockwise from 12 o'clock, in [0, 2π).
                (a + FRAC_PI_2).rem_euclid(TAU)
            })
            .collect();
        assert!(angles.windows(2).all(|w| w[0] < w[1]), "{angles:?}");
        // And so do the leaves of each branch, round their parent.
        for &b in &m.nodes[0].children {
            let kids = &m.nodes[b].children;
            let base = (ms.nodes[b].cy() - root.cy()).atan2(ms.nodes[b].cx() - root.cx());
            let rel: Vec<f64> = kids
                .iter()
                .map(|&k| {
                    let a = (ms.nodes[k].cy() - ms.nodes[b].cy())
                        .atan2(ms.nodes[k].cx() - ms.nodes[b].cx());
                    wrap(a - base)
                })
                .collect();
            assert!(rel.windows(2).all(|w| w[0] < w[1]), "{rel:?}");
        }
    }

    /// Report: `root((flowmaid))` was a 1.6:1 ellipse.
    #[test]
    fn circle_is_a_true_circle_round_its_label() {
        let ms = scene(&mind("mindmap\nroot((flowmaid))\n  ((a much longer circle))\n"));
        for n in &ms.nodes {
            assert_eq!(n.shape, MindShape::Circle);
            assert!((n.w - n.h).abs() < 1e-9, "{} is {}x{}", n.text, n.w, n.h);
            // The label block's corners sit inside the circle.
            let (tw, th) = text_block(&n.text, n.depth);
            assert!(tw.hypot(th) <= n.w, "{}: label {tw}x{th} in d={}", n.text, n.w);
        }
        let svg = to_svg(&ms);
        assert!(svg.contains("<circle") && !svg.contains("<ellipse"), "{svg}");
    }

    /// Report: undelimited nodes drew as the same pill as `(rounded)`.
    #[test]
    fn default_rounded_and_square_draw_differently() {
        let ms = scene(&mind("mindmap\nRoot\n  plain\n  (pill)\n  [box]\n"));
        let svg = to_svg(&ms);
        let plain = by_text(&ms, "plain");
        assert_eq!(plain.shape, MindShape::Default);
        let (ux, uy, uw, uh) = underline(plain).unwrap();
        assert_eq!((ux, uw, uh), (plain.x, plain.w, UNDERLINE_H));
        assert!((uy + uh - (plain.y + plain.h)).abs() < 1e-9, "on the bottom edge");
        assert!(svg.contains("fill-opacity=\"0.24\""), "underline drawn: {svg}");
        let pill = by_text(&ms, "pill");
        assert!(underline(pill).is_none());
        assert!(
            svg.contains(&format!("rx=\"{:.1}\"", pill.h / 2.0)),
            "a pill's ends are fully round"
        );
        assert!(svg.contains("rx=\"0.0\""), "a square has sharp corners");
    }

    /// Fraction of a closed polygon's vertices where it turns the
    /// "wrong" way (concave notches).
    fn concave_share(pts: &[(f64, f64)]) -> f64 {
        let n = pts.len();
        let turn = |i: usize| {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0)
        };
        let total: f64 = (0..n).map(turn).sum();
        let concave = (0..n).filter(|&i| turn(i) * total < 0.0).count();
        concave as f64 / n as f64
    }

    /// Report: the cloud was a jagged |sin| polygon that read as a
    /// second explosion, and the bang's spikes were 4× longer at the
    /// sides than on top.
    #[test]
    fn cloud_is_puffy_and_bang_spikes_are_even() {
        let ms = scene(&mind("mindmap\nR\n  )Eleven diagram types(\n  ))Zero dependencies((\n"));
        let cloud = by_text(&ms, "Eleven diagram types");
        let bang = by_text(&ms, "Zero dependencies");
        let (cp, bp) = (perimeter(cloud).unwrap(), perimeter(bang).unwrap());
        // A cloud is round lobes: only the valleys between lobes turn
        // inward. A bang notches at every other vertex.
        assert!(concave_share(&cp) < 0.2, "cloud {:.2}", concave_share(&cp));
        assert!(concave_share(&bp) > 0.4, "bang {:.2}", concave_share(&bp));
        // Every spike tip stands BANG_SPIKE off the valley ellipse.
        let (rx, ry) = (bang.w / 2.0 - BANG_SPIKE, bang.h / 2.0 - BANG_SPIKE);
        for (j, &(x, y)) in bp.iter().enumerate() {
            let (dx, dy) = (x - bang.cx(), y - bang.cy());
            let on = (dx / rx).powi(2) + (dy / ry).powi(2);
            if j % 2 == 1 {
                assert!((on - 1.0).abs() < 1e-6, "valley {j} off the ellipse");
            } else {
                // Outside the valley ellipse, inside the box.
                assert!(on > 1.0, "tip {j} inside");
                assert!(dx.abs() <= bang.w / 2.0 + 1e-6 && dy.abs() <= bang.h / 2.0 + 1e-6);
            }
        }
        // Both keep the label inside the outline's inner ellipse.
        for n in [cloud, bang] {
            let (tw, th) = text_block(&n.text, n.depth);
            let (ex, ey) = label_ellipse(tw, th);
            let inner = if n.shape == MindShape::Cloud {
                CLOUD_LOBE
            } else {
                BANG_SPIKE
            };
            assert!((n.w / 2.0 - inner - ex).abs() < 1e-9 && (n.h / 2.0 - inner - ey).abs() < 1e-9);
            assert!((tw / 2.0 / ex).powi(2) + (th / 2.0 / ey).powi(2) < 1.0);
        }
    }

    /// Report: the root was a pale #c9cfe0 ellipse (1.56:1 on white),
    /// the faintest thing on the canvas; trunks barely tapered (4.5 vs
    /// 3.5px).
    #[test]
    fn root_is_the_strongest_node_and_trunks_taper() {
        let ms = scene(&showcase());
        let root = &ms.nodes[0];
        assert_eq!(root.fill, ROOT_FILL);
        let c = crate::style::contrast_ratio(ROOT_FILL, "#ffffff").unwrap();
        assert!(c >= 7.0, "root fill only {c:.1}:1 on the white canvas");
        assert_eq!(root.text_color, "#ffffff");
        assert_eq!((root.font_size, root.font_weight), (ROOT_FONT, 700));
        let svg = to_svg(&ms);
        assert!(svg.contains("font-size=\"18\" font-weight=\"bold\""), "{svg}");
        let trunk = ms.edges.iter().map(|e| e.width).fold(0.0, f64::max);
        let leaf = ms.edges.iter().map(|e| e.width).fold(f64::MAX, f64::min);
        assert!(trunk >= 2.0 * leaf, "trunk {trunk} vs leaf {leaf}");
        assert_eq!(link_width(1), 9.0);
    }

    /// Report: `::icon(…)` and `:::class` lines were dropped silently.
    #[test]
    fn icons_are_kept_and_drawn() {
        let m = mind("mindmap\nRoot\n  Docs\n  ::icon(fa fa-book)\n  Other\n    ::icon(mdi mdi-unheard-of)\n");
        assert_eq!(m.nodes[1].icon.as_deref(), Some("fa fa-book"));
        assert_eq!(m.nodes[2].icon.as_deref(), Some("mdi mdi-unheard-of"));
        assert_eq!(m.nodes.len(), 3, "decorations are not nodes");
        let ms = scene(&m);
        let plain = scene(&mind("mindmap\nRoot\n  Docs\n  Other\n"));
        let (with, without) = (by_text(&ms, "Docs"), by_text(&plain, "Docs"));
        assert!((with.w - without.w - (ICON + ICON_GAP)).abs() < 1e-9, "room for it");
        // The glyph sits left of the text, inside the box.
        let strokes = icon_strokes(with).unwrap();
        let (tw, _) = text_block("Docs", with.depth);
        let text_left = label_centre(with).0 - tw / 2.0;
        for &(x, y) in strokes.iter().flatten() {
            assert!(x >= with.x && x <= text_left, "{x} outside the icon slot");
            assert!(y >= with.y && y <= with.y + with.h);
        }
        let svg = to_svg(&ms);
        assert!(svg.contains("<title>fa fa-book</title>"), "{svg}");
        assert!(svg.contains("<title>mdi mdi-unheard-of</title>"), "unknown names still draw");
        assert!(icon_strokes(by_text(&ms, "Root")).is_none());
    }

    #[test]
    fn classes_reach_the_svg_and_class_defs_colour_nodes() {
        let m = mind(
            "mindmap\nRoot\n  Hot\n  :::urgent large\n  Calm\n  :::quiet\n  \
             classDef in a sentence\nclassDef urgent fill:#ffe066,stroke:#c92a2a,stroke-width:3px\n\
             classDef quiet fill:#495057,color:#ffd43b\n",
        );
        let hot = &m.nodes[1];
        assert_eq!(hot.classes, ["urgent", "large"]);
        assert_eq!(hot.style.fill.as_deref(), Some("#ffe066"));
        // A line that only starts with the word stays a node.
        assert!(m.nodes.iter().any(|n| n.text == "classDef in a sentence"));
        let ms = scene(&m);
        let hot = by_text(&ms, "Hot");
        assert_eq!(hot.paint(), "#ffe066");
        // Text follows the custom fill's contrast, not the branch's.
        assert_eq!(hot.ink(), crate::style::text_on("#ffe066"));
        let calm = by_text(&ms, "Calm");
        assert_eq!((calm.paint(), calm.ink()), ("#495057", "#ffd43b"));
        let svg = to_svg(&ms);
        assert!(svg.contains("<g class=\"urgent large\">"), "{svg}");
        assert!(svg.contains("fill=\"#ffe066\" stroke=\"#c92a2a\" stroke-width=\"3.0\""), "{svg}");
        assert!(svg.contains("fill=\"#ffd43b\""), "{svg}");
        // The branch colour is still what the connector uses.
        assert!(ms.edges.iter().all(|e| !e.color.starts_with("#ffe")));
    }

    #[test]
    fn class_names_and_colours_are_escaped() {
        let m = mind("mindmap\nRoot\n  A\n  :::x\"y\nclassDef x\"y fill:red\" onload=z\n");
        let svg = to_svg(&scene(&m));
        assert!(!svg.contains("\" onload=z"), "{svg}");
        assert!(!svg.contains("class=\"x\"y\""), "{svg}");
    }

    #[test]
    fn decorations_before_any_node_are_ignored() {
        let m = mind("mindmap\n::icon(fa fa-x)\n:::cls\nRoot\n");
        assert_eq!(m.nodes.len(), 1);
        assert!(m.nodes[0].icon.is_none() && m.nodes[0].classes.is_empty());
    }

    /// A deterministic spread of trees: whatever layout `scene` lands
    /// on, boxes never overlap and connectors never cross.
    #[test]
    fn random_trees_stay_clean() {
        let shapes = ["", "(", "[", "((", "{{", "))", ")"];
        let closes = ["", ")", "]", "))", "}}", "((", "("];
        for seed in 0..40u64 {
            let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            let mut next = |m: u64| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x % m
            };
            let mut src = String::from("mindmap\n  Root topic\n");
            let mut depth = 1;
            for i in 0..(5 + next(30)) {
                depth = (depth as i64 + next(3) as i64 - 1).clamp(1, 4) as usize;
                let s = next(shapes.len() as u64) as usize;
                let words = ["node", "a longer label", "x", "multi<br/>line"][next(4) as usize];
                src.push_str(&format!(
                    "{}{}{words} {i}{}\n",
                    "  ".repeat(depth + 1),
                    shapes[s],
                    closes[s]
                ));
            }
            let ms = scene(&mind(&src));
            assert_no_overlap(&ms);
            assert_no_crossing(&ms);
        }
    }

    #[test]
    fn ring_fallback_is_clean_too() {
        // The fallback must stay a valid layout on its own.
        let m = showcase();
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        let pos = ring_centres(&m, &sizes);
        assert!(clean(&m, &sizes, &pos));
    }

    #[test]
    fn a_deep_bushy_tree_still_lays_out() {
        // Three children per node, six levels: the fan budget may run
        // out, in which case the ring layout takes over — either way
        // the scene is complete and clean.
        let mut src = String::from("mindmap\nR\n");
        fn grow(src: &mut String, depth: usize, max: usize) {
            if depth > max {
                return;
            }
            for k in 0..3 {
                src.push_str(&format!("{}n{depth}{k}\n", " ".repeat(depth * 2)));
                grow(src, depth + 1, max);
            }
        }
        grow(&mut src, 1, 6);
        let ms = scene(&mind(&src));
        assert_eq!(ms.nodes.len(), 1 + 3 + 9 + 27 + 81 + 243 + 729);
        assert_no_overlap(&ms);
    }

    #[test]
    fn svg_is_deterministic() {
        let m = showcase();
        assert_eq!(to_svg(&scene(&m)), to_svg(&scene(&m)));
    }
}
