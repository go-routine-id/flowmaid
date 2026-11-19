
#[test]
fn a_frames_pin_keeps_off_the_columns_of_the_entries_beside_it() {
    // Two stacked blocks, the lower one's left member wholly under its
    // title and entered from the upper block, and a pin into the lower
    // frame from the left (the verifier's twin blocks, fix round 4 of
    // b12a2b-base). The repair took `a1 --> b1` round the lower title's
    // chip at the regular width, into its bold ink, between the chip and
    // the frame's pin `q --> B`, which the ports stage stood a chip's
    // clearance past the ink; the nudge's spread laid the pin right of
    // `a1 --> b1`'s column (its order key sums both ends' turns), across
    // that column, so its scoring took both moves back: `a1 --> b1` 0.04
    // px from `Shipping and returns desk two`'s last letter, and under
    // shorter titles a 58 px run beside the pin. The pin now comes in
    // left of the column it would cross: no line within `CHIP_SEP` of a
    // title's ink, no two lines on top of each other, no crossing.
    for title in [
        "Shipping and returns desk",
        "Order processing",
        "Identity provider",
        "Monitoring stack",
    ] {
        let twins = format!(
            "flowchart TD
    subgraph A[{title}]
        a1[a1 wide node]
        a2[a2]
    end
    subgraph B[{title} two]
        b1[b1]
        b2[b2 wide node]
    end
    a1 --> b1
    a2 --> b2
    a1 --> b2
    p[p] --> A
    p --> a2
    q[q] --> B
    q --> b1
"
        );
        let g = graph(&twins);
        let sc = block_scene(&g);
        titles_clear_their_ink(title, &sc, None);
        let q = crate::quality::measure(&g, &sc);
        assert_eq!(q.overlap_pairs, 0, "{title}: {:?}", q.details);
        assert_eq!(q.defects, 0, "{title}: {:?}", q.details);
        assert_eq!(q.x_drawn, 0, "{title}: {:?}", q.details);
    }
    // A pin into a frame beside an entry `ortho`'s title sides displaced
    // past the frame's title (a member wholly under it, entered from the
    // left): the spread laid the entry at the ink's clearance and the pin
    // past it, the scoring refused the entry's move (its turn under the
    // title row came within the chip's reach at its corner) and kept the
    // pin's, which took away a crossing: the pin's head came 5 px from
    // the entry's line (the verifier's random inputs r250 and r042). A
    // pin is not moved nearer another line than a stroke's spacing.
    let pinned = [
        "flowchart TD
    subgraph G[WWW Mammoth Wombat Overview]
        m0[ledger]
        m1[ledger]
        m2[db]
    end
    s0[src0] --> m0
    s1[src1] --> G
    s1 --> m2
    m2 --> z[end]
    G --> z2[after]
",
        "flowchart TD
    subgraph O[Shipping and returns desk]
        subgraph G[WWW Mammoth Wombat Overview]
            m0[db]
            m1[auth service]
            m2[ledger]
        end
        k[k]
    end
    m0 --> k
    m1 --> k
    s0[src0] -- \"lbl2\" --> m2
    s0 --> m2
    s1[src1] --> G
    s1 --> G
    s2[src2] --> m1
    m2 --> z[end]
",
    ];
    for (k, src) in pinned.iter().enumerate() {
        let g = graph(src);
        let sc = block_scene(&g);
        let q = crate::quality::measure(&g, &sc);
        assert_eq!(q.overlap_pairs, 0, "pinned {k}: {:?}", q.details);
        assert_eq!(q.defects, 0, "pinned {k}: {:?}", q.details);
    }
}
