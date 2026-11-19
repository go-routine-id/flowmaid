#[test]
fn sibling_frames_of_a_band_start_level_left_to_right_too() {
    // B12 X4 in LR / RL: the frames of two top-level blocks that share
    // their first row start level along the flow, the later one moved
    // upstream as a whole. Drawn LR, a narrow first member put its frame
    // 120 px after its sibling's, whose first member is wide, while its
    // wide last member took it past the sibling's end: side by side and
    // neither's span holding the other's (`sib_stagger`).
    let src = "flowchart LR
  s[Start] --> a1
  t[Other start] --> b1
  subgraph A[Short block]
    a1[a] --> a2[a wide last member node]
  end
  subgraph B[Wide first row]
    b1[a very wide first member node] --> b2[b]
  end";
    for dir in ["LR", "RL"] {
        let g = graph(&in_direction(src, dir));
        let sc = block_scene(&g);
        let q = crate::quality::measure(&g, &sc);
        assert_eq!(q.sib_stagger, 0, "{dir}");
        let (a, b) = (frame_box(&sc, "A"), frame_box(&sc, "B"));
        let lead = |r: (f64, f64, f64, f64)| if dir == "LR" { r.0 } else { r.2 };
        assert!((lead(a) - lead(b)).abs() < 0.5, "{dir}: {a:?} {b:?}");
        // The moved block's frame keeps its size: its members and pad.
        let members = ["a1", "a2"].map(|id| node_box(&sc, id));
        let span = members
            .iter()
            .map(|m| m.2)
            .fold(f64::NEG_INFINITY, f64::max)
            - members.iter().map(|m| m.0).fold(f64::INFINITY, f64::min);
        assert!((a.2 - a.0) - span < 40.0, "{dir}: {a:?} round {span}");
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let routed = crate::levels::with_blocks(true, || crate::scene::route(&g, &at));
        for id in ["A", "B"] {
            let (p, r) = (frame_box(&sc, id), frame_box(&routed, id));
            assert!(
                (p.0 - r.0).abs() < 0.5 && (p.2 - r.2).abs() < 0.5,
                "{dir} {id}: {p:?} routed {r:?}"
            );
        }
    }
}

#[test]
fn nested_sibling_frames_start_level_too() {
    // B12 X4 at any depth: two blocks inside one parent that share their
    // first row start level, the later one moved upstream as a whole
    // inside its parent. A short first member put its frame 20 px after
    // its sibling's, whose first member is tall, while its tall last
    // member took it past the sibling's end (`sib_stagger`).
    let src = r#"flowchart TD
  s[Start] --> a1
  t[Other start] --> b1
  subgraph P[Parent block]
    subgraph A[Short first row]
      a1[a] --> a2["a tall<br/>last<br/>member<br/>node"]
    end
    subgraph B[Tall first row]
      b1["b tall<br/>first<br/>member<br/>node"] --> b2[b]
    end
  end"#;
    for dir in ["TD", "BT", "LR", "RL"] {
        let g = graph(&in_direction(src, dir));
        let sc = block_scene(&g);
        let q = crate::quality::measure(&g, &sc);
        assert_eq!(q.sib_stagger, 0, "{dir}");
        let (a, b) = (frame_box(&sc, "A"), frame_box(&sc, "B"));
        let lead = |r: (f64, f64, f64, f64)| match dir {
            "TD" => r.1,
            "BT" => r.3,
            "LR" => r.0,
            _ => r.2,
        };
        assert!((lead(a) - lead(b)).abs() < 0.5, "{dir}: {a:?} {b:?}");
        // Inside its parent still.
        let p = frame_box(&sc, "P");
        assert!(
            a.0 >= p.0 && a.1 >= p.1 && a.2 <= p.2 && a.3 <= p.3,
            "{dir}: {a:?} in {p:?}"
        );
    }
}

#[test]
fn sibling_frames_that_share_their_last_row_end_level() {
    // B12 X4 downstream: two sibling blocks that share their last row but
    // not their first end level, the one that ended earlier moved
    // downstream as a whole. A short last member ended its frame 25 px
    // before its sibling's, whose last member is tall, while its earlier
    // first row took it past the sibling's start (`sib_stagger`).
    let src = r#"flowchart TD
  s[Start] --> a1
  t[Other start] --> u[Step] --> b2
  subgraph A[First block]
    a1[a one] --> a2[a two] --> a3[a three]
  end
  subgraph B[Second block]
    b2[b two] --> b3["b tall<br/>last<br/>member"]
  end"#;
    for dir in ["TD", "BT", "LR", "RL"] {
        let g = graph(&in_direction(src, dir));
        let sc = block_scene(&g);
        let q = crate::quality::measure(&g, &sc);
        assert_eq!(q.sib_stagger, 0, "{dir}");
        let (a, b) = (frame_box(&sc, "A"), frame_box(&sc, "B"));
        let trail = |r: (f64, f64, f64, f64)| match dir {
            "TD" => r.3,
            "BT" => r.1,
            "LR" => r.2,
            _ => r.0,
        };
        assert!((trail(a) - trail(b)).abs() < 0.5, "{dir}: {a:?} {b:?}");
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let routed = crate::levels::with_blocks(true, || crate::scene::route(&g, &at));
        for id in ["A", "B"] {
            let (p, r) = (frame_box(&sc, id), frame_box(&routed, id));
            assert!(
                (p.0 - r.0).abs() < 0.5
                    && (p.1 - r.1).abs() < 0.5
                    && (p.2 - r.2).abs() < 0.5
                    && (p.3 - r.3).abs() < 0.5,
                "{dir} {id}: {p:?} routed {r:?}"
            );
        }
    }
}

#[test]
fn a_frames_pins_and_its_displaced_entries_are_one_set_of_columns() {
    // B12 2.8: an entry under a block's title chip is displaced to a
    // column past the chip. A pin of an edge into the frame ends on the
    // same side, which no crossing of it finds: the column was dealt a
    // breadth's rounding off a pin and the two lines ran down together
    // (overlapping, the arrowheads one on the other), or, dealt past the
    // pins, it crossed every line into them. Pins and displaced columns
    // are one set, a track apart, in the order their lines come from; a
    // pin moves along the side where a column needs its place.
    let cases = [
        "flowchart TD
    s1[caller] --> q1
    s2[client] --> G
    s3[batch] --> G
    s4[hub] --> q2
    s4 --> q1
    subgraph G[Shipping and returns desk]
        q1[queue]
        q2[charge]
        q3[ship]
        q1 --> q3
        q2 --> q3
    end
    q3 --> m[metrics]",
        "flowchart TD
    a[a] --> x
    b[b] --> I
    c[c] --> O
    d[d] --> y
    subgraph O[Édition générale des données]
        subgraph I[Édition générale des données inner]
            x[x wide member]
            y[y]
        end
        k[k]
    end
    x --> k
    y --> k
    k --> z[z]",
    ];
    for src in cases {
        let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(src, false))
            .expect("a flowchart");
        assert!(rep.block_mode);
        let m = &rep.statik;
        assert_eq!(
            (m.overlap_pairs, m.x_drawn),
            (0, 0),
            "{src}\n{:?}",
            m.details
        );
        assert!(
            m.min_edge_gap >= crate::ortho::TRACK_PITCH - 0.5,
            "{src}\n{}",
            m.min_edge_gap
        );
    }
}

#[test]
fn a_block_layouts_chips_keep_clear_of_other_lines() {
    // B12 G3: a block layout's chip keeps `CLEAR_GAP` from every other
    // edge's line where its own line leaves it such a spot. The public
    // fixture drawn BT had `deploy` (image registry into the storefront)
    // 3 px off the thick `deploy` run under it once X4 levelled the build
    // pipeline, and the shipping worker's `write` chip, out of its end
    // block, pushed the ledger's `write` to 4 px from a `scrape` line.
    let gap = |r: (f64, f64, f64, f64), a: (f64, f64), b: (f64, f64)| {
        // Rect to segment, sampled finely enough for a 0.5 px answer.
        let n = ((b.0 - a.0).hypot(b.1 - a.1) / 0.5).ceil().max(1.0) as usize;
        (0..=n)
            .map(|k| {
                let t = k as f64 / n as f64;
                let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                let dx = (r.0 - p.0).max(p.0 - r.2).max(0.0);
                let dy = (r.1 - p.1).max(p.1 - r.3).max(0.0);
                dx.hypot(dy)
            })
            .fold(f64::INFINITY, f64::min)
    };
    for name in ["blocks-overview-bt", "blocks-overview"] {
        let g = fixture(&format!("blocks/{name}"));
        let sc = block_scene(&g);
        let lines: Vec<Vec<(f64, f64)>> =
            sc.edges.iter().map(crate::scene::edge_polyline).collect();
        for (i, e) in sc.edges.iter().enumerate() {
            let Some((text, at, w)) = &e.label else {
                continue;
            };
            let watched = matches!(
                (e.from.as_str(), e.to.as_str()),
                ("REG", "WEB") | ("SHIP", "DB") | ("LEDGER", "DB")
            );
            if !watched {
                continue;
            }
            let h = crate::layout::LABEL_BOX_H / 2.0;
            let r = (at.0 - w / 2.0, at.1 - h, at.0 + w / 2.0, at.1 + h);
            let near = lines
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .flat_map(|(_, l)| l.windows(2).map(|s| gap(r, s[0], s[1])))
                .fold(f64::INFINITY, f64::min);
            assert!(
                near >= crate::clearance::CLEAR_GAP - 0.5,
                "{name} {}->{} `{text}`: another line {near:.1} px from its chip",
                e.from,
                e.to
            );
        }
    }
}

