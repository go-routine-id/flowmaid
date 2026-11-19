//! Layout-quality regression gate over `tests/fixtures/quality/*.mmd`.
//!
//! Prints the metrics table (`cargo test --test quality -- --nocapture`)
//! and fails when a fixture gets WORSE than its recorded ceiling. The
//! ceilings started at the 0.31.1 baseline and are tightened as layout
//! work lands; never loosen one without saying why in the PR.

use flowmaid::quality::{measure_source, Metrics};

/// Upper bounds per fixture for (static, routed):
/// (edge_node_pairs, edge_frame_pairs, defects).
type Ceil = (usize, usize, usize);

const CEILINGS: &[(&str, Ceil, Ceil)] = &[
    // Tightened to the integrated ports + router + labels + layout
    // stages (0.31.1 in brackets: static, routed).
    ("back-edge", (0, 0, 0), (0, 0, 0)), // (0,0,1) (1,0,2)
    // bench-75 is new with the router; its first rows were (3,5,133)
    // (2,4,217).
    // Routed defects rose to 40 when a shared corner started counting
    // as an overlap (5 of them) and the router began keeping off frame
    // borders (border runs 7 -> 3), which costs it a few short overlaps.
    // Static 35 -> 15 with the channel nudge; routed 40 -> 36 on the
    // integrated round-2 pipeline; routed 36 -> 39 (one frame entry)
    // when a reroute lying on another edge stopped beating a curve that
    // only crosses a frame (G1 over G6).
    ("bench-75", (0, 2, 15), (0, 1, 39)),
    // The dense fixtures and fanout-frames come from
    // `examples/fixture_gen.rs` (seeded: their own nesting, fan-outs
    // and edges, nested frames crossed by long edges), measured on the
    // round-2 pipeline when they replaced the hand-made ones.
    // Routed edge_node 2 / 3 -> 0 once an edge the searches leave
    // through nodes gets the rescue detour (frames crossed, nodes not).
    ("dense-lr", (0, 11, 13), (0, 13, 20)),
    ("dense-td", (0, 28, 42), (0, 28, 47)),
    ("fanout-frames", (0, 0, 0), (0, 0, 0)),
    ("wide-labels", (0, 0, 0), (0, 0, 0)), // (0,0,6) (0,0,8)
];

/// Least `min_edge_gap` per fixture (static, routed): the closest two
/// unrelated, uncrossed edges come (px, G4). Floors at the integrated
/// values, so later work cannot squeeze edges closer; fixtures whose
/// pairs all sit at least the probe distance apart report 40.
const GAP_FLOORS: &[(&str, f64, f64)] = &[
    ("back-edge", 40.0, 40.0),
    // Static 3.4 before the channel nudge spread parallel legs apart.
    ("bench-75", 5.9, 2.1),
    // The generated dense fixtures, as first measured. dense-lr static
    // 9.6 -> 9.5 (report M7): its one label past the 200 px wrapping
    // width (`n32`, 204 px) now wraps to two lines, the drawing grows
    // 17 px taller, and a pair of parallel legs closes 9.63 -> 9.53 px
    // while the routed drawing crosses 4 fewer edges.
    //
    // B7 (report M3, M8, §2.4, §2.6, §2.8): every labelled gap keeps
    // its label clear of the head (72 px across a TD gap, the head and
    // a clearance either side along LR), a lone end meets its box side
    // in the middle, a comb spaces evenly, and a box's title strip no
    // longer leaves a strip of dead room under its first row (dense-td
    // 1 823 -> 1 663 px tall). Its static drawing's closest pair is
    // now two router legs 10.5 px apart in one shared channel under the
    // g5 / g6 frames, which closed by 160 px (12.16 -> 10.50); its
    // routed pairs opened 6.6 -> 12.0 and dense-lr's 9.5 / 9.7 ->
    // 11.7 / 12.1, ratcheted here.
    //
    // B7 review r1 (#3): the floor dense-td had lost is back at 12.1.
    // The router's dependency on the dead strip was channel room: two
    // unrelated edges the nudge leaves side by side under the spacing
    // now get one moved clear where that costs nothing else
    // (`router::spread_close`, static scene), and a gap a box opens in
    // keeps its border off the lateral runs crossing it (`layout`'s
    // `cross_need`): dense-td static 10.50 -> 13.04, dense-lr 11.74 ->
    // 14.50, ratcheted to 12.0.
    ("dense-lr", 12.0, 12.0),
    ("dense-td", 12.1, 12.0),
    ("fanout-frames", 40.0, 40.0),
    ("wide-labels", 40.0, 40.0),
];

fn check(name: &str, mode: &str, m: &Metrics, ceil: Ceil) -> Vec<String> {
    let mut bad = Vec::new();
    let mut gate = |what: &str, got: usize, max: usize| {
        if got > max {
            bad.push(format!("{name} {mode}: {what} = {got} > ceiling {max}"));
        }
    };
    gate("edge_node_pairs", m.edge_node_pairs, ceil.0);
    // No edge through an unrelated node, in either mode (G6, a round-1
    // win): what `route()` shows while dragging is gated too.
    gate("edge_node_x", m.edge_node_x, 0);
    gate("edge_frame_pairs", m.edge_frame_pairs, ceil.1);
    gate("defects", m.defects(), ceil.2);
    // Ports and free-edge sides (G7, G8, G9): every end has its own
    // port, curves never double back, frames are entered along the flow.
    gate("shared_ports", m.shared_ports, 0);
    gate("arrow_on_start", m.arrow_on_start, 0);
    gate("wiggles", m.wiggles, 0);
    gate("frame_side", m.frame_side, 0);
    // Invariants that already hold everywhere (G2).
    gate("node_overlap", m.node_overlap, 0);
    gate("node_frame", m.node_frame, 0);
    gate("frame_overlap", m.frame_overlap, 0);
    bad
}

#[test]
fn quality_gate() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    let header: Vec<&str> = Metrics::default().fields().iter().map(|f| f.0).collect();
    let late: Vec<&str> = Metrics::default()
        .late_fields()
        .iter()
        .map(|f| f.0)
        .collect();
    let tail: Vec<&str> = Metrics::default()
        .tail_fields()
        .iter()
        .map(|f| f.0)
        .collect();
    println!(
        "file\tmode\t{}\tdefects\t{}\t{}",
        header.join("\t"),
        late.join("\t"),
        tail.join("\t")
    );
    let mut bad = Vec::new();
    for &(name, cs, cr) in CEILINGS {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let rep = measure_source(&src, false).expect("flowchart fixture");
        assert!(rep.deterministic, "{name}: output not deterministic (G14)");
        let floors = GAP_FLOORS
            .iter()
            .find(|f| f.0 == name)
            .expect("every fixture has a gap floor");
        for (mode, m, ceil, floor) in [
            ("static", &rep.statik, cs, floors.1),
            ("routed", &rep.routed, cr, floors.2),
        ] {
            let vals: Vec<String> = m.fields().iter().map(|f| format!("{:.2}", f.1)).collect();
            let late: Vec<String> = m
                .late_fields()
                .iter()
                .map(|f| format!("{:.2}", f.1))
                .collect();
            let tail: Vec<String> = m
                .tail_fields()
                .iter()
                .map(|f| format!("{:.2}", f.1))
                .collect();
            println!(
                "{name}\t{mode}\t{}\t{}\t{}\t{}",
                vals.join("\t"),
                m.defects(),
                late.join("\t"),
                tail.join("\t")
            );
            bad.extend(check(name, mode, m, ceil));
            if m.min_edge_gap < floor {
                bad.push(format!(
                    "{name} {mode}: min_edge_gap = {:.2} < floor {floor}",
                    m.min_edge_gap
                ));
            }
        }
    }
    assert!(bad.is_empty(), "quality regressions:\n{}", bad.join("\n"));
}

/// Most px of edge riding a frame border (`border_len`, G4) over all
/// fixtures, per mode (static, routed). Before the router priced border
/// runs: 1826 static, 2387 routed (hand-made dense fixtures). Re-measured
/// on the generated dense fixtures, whose long edges cross nested
/// frames: 1069 / 2131. B1 review r3 (#3): 1082 static — two bench-75
/// ends just inside the `data` frame (svc3 -> db, user -> gw) that swing
/// in from the side now enter their heads along the line touching their
/// curve instead of an S hugging it, and that line runs 9 and 3 px
/// longer within reach of the frame's left border; no route changed.
/// B3 (report §2.2): 2140 -> 2070 routed; static 1082 -> 1020 — in
/// bench-75 the back-edges no longer bow through the zones but run in
/// lanes clear of every node (its static drawing crosses 15 fewer edges,
/// defects 13 -> 5), and `svc4 -> logs`, which the router used to redraw
/// round `alerts -> gw`'s bow and whose layout channel rides the app
/// frame's right border for 99 px, is searched on past a first
/// candidate that does not beat it (`router::repair`) and leaves the
/// border (`dense-td`'s `n21 -> n16`, a 63 px ride, is a reversed S now).
/// Routed, the fixtures measured 1975 before B3 and 2061 after: in
/// bench-75's `route()` the lanes now take the sides `scene()` takes,
/// and the forward edges the router redraws round them ride borders a
/// drag frame does not search off (`router::repair` searches only a
/// cramped node in `route()`, G13): `svc3 -> db` 140 px on the data
/// frame's left, `svc1 -> objects` 62 px on the app frame's right and
/// `queue -> job2` 57 px on the jobs frame's left, where `svc1 -> db`
/// (87 px) and `svc6 -> db` (85 px) rode before (bench-75 274 -> 360 px,
/// border_runs 4 -> 5). Ratcheted to just above that: 2070 -> 2065.
/// B4 (report Q7): 1020 -> 1022 static, as an intended drawing change:
/// an end on a cylinder's top or bottom now starts on the cap's arc
/// instead of the box side (`edgepass::snap_ends` since B6), so `dense-lr`'s hub
/// `n12`'s bottom-corner ends `n12 -> n36` / `n12 -> n38` start 6 px up
/// the bottom arc and their curves run 1-2 px longer beside g1's and
/// g2's right borders (1018.7 -> 1021.7); no route changed.
/// B7 (report M3, M8): 1022 -> 1475 static, 2065 -> 2105 routed, as
/// intended drawing changes cost it: dense-td lost the dead strip under
/// each box's first row (160 px over the drawing, as much as 99 px in
/// one gap), its labelled gaps grew by their head room, and its lone
/// ends moved to their sides' middles, so its long `n31 -> n38`
/// (316 + 192 -> 453 + 277 px under g5's and g6's bottoms) and
/// `n22 -> n40` (126 px down g6's left) ride borders the closer
/// channel leaves no lane beside; bench-75 routed 360 -> 402.
/// B7 review r1 (#2, #3): 1 475 -> 1 395 static, 2 105 -> 1 895 routed —
/// a gap a box opens in keeps the box's top off the middle of the runs
/// crossing it into the box (dense-td 1 098 -> 1 020 static, 1 180 ->
/// 971 routed: its `n45 -> g8` no longer rides g1's top) and the static
/// scene moves one of two edges side by side under the spacing clear.
const BORDER_LEN_MAX: (f64, f64) = (1_395.0, 1_895.0);

/// Edges keep off frame borders where their channel has room (G4).
#[test]
fn edges_keep_off_frame_borders() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    let (mut stat, mut routed) = (0.0, 0.0);
    for &(name, ..) in CEILINGS {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let rep = measure_source(&src, false).expect("flowchart fixture");
        stat += rep.statik.border_len;
        routed += rep.routed.border_len;
    }
    assert!(stat <= BORDER_LEN_MAX.0, "static border_len {stat:.0}");
    assert!(routed <= BORDER_LEN_MAX.1, "routed border_len {routed:.0}");
}

/// Every committed fixture is gated (a new one needs a ceiling row).
#[test]
fn every_fixture_has_a_ceiling() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "mmd") {
            let stem = p.file_stem().unwrap().to_string_lossy().to_string();
            assert!(
                CEILINGS.iter().any(|c| c.0 == stem),
                "{stem}.mmd has no ceiling in tests/quality.rs"
            );
        }
    }
}

/// Most labels (over all fixtures, per mode) another edge's stroke may
/// run through (136 static / 111 routed at 0.31.1, bench-75 aside;
/// 43 / 42 at round 1). The final label pass also moves a label off a
/// stroke, along its own line or just beside it, when a spot crossed
/// by fewer strokes exists; the rest sit in bundles with no such spot.
/// Integrated with the round-2 router (re-sorted ports, labelled edges
/// kept off spacing-only reroutes): 35 / 20 on the merged pipeline;
/// 0 / 4 on the generated dense fixtures. B7 (report M3): 0 -> 1
/// static — dense-lr's `n12 -> n33` label sits beside its line in the
/// cable of three curves that leaves hub `n12`'s over-full side (its
/// lone far ends now meet their targets in the middle), its box's edge
/// over the neighbour `n12 -> n36`; no spot along the cable is clear.
const LBL_EDGE_MAX: (usize, usize) = (1, 4);

/// Most hard label hits (node, label, chip or arrowhead) over all
/// fixtures, per mode (static, routed). None: a spot covering an
/// arrowhead now always loses to one merely crossing strokes, and a
/// label wider than its gap (`wide-labels` A->B in `route()`, round 1's
/// last hit) finds one in the fine sweep beside its line. 0.31.1: 13
/// static, 16 routed.
const LBL_HARD_MAX: (usize, usize) = (0, 0);

/// Edge labels never hide a node, another label, a frame title chip or
/// an arrowhead, nor crowd another label, in either mode (G11).
#[test]
fn labels_cover_nothing_they_must_not() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    let (mut stat, mut routed) = (0, 0);
    let (mut hard_s, mut hard_r) = (0, 0);
    for &(name, ..) in CEILINGS {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let rep = measure_source(&src, false).expect("flowchart fixture");
        let hard = |m: &Metrics| m.lbl_node + m.lbl_lbl + m.lbl_chip + m.lbl_arrow;
        hard_s += hard(&rep.statik);
        hard_r += hard(&rep.routed);
        stat += rep.statik.lbl_edge;
        routed += rep.routed.lbl_edge;
        // Other labels' boxes plus the crowd gap are a hard keep-out.
        assert_eq!(
            (rep.statik.lbl_crowd, rep.routed.lbl_crowd),
            (0, 0),
            "{name}: labels crowd each other"
        );
    }
    assert!(hard_s <= LBL_HARD_MAX.0, "static hard label hits {hard_s}");
    assert!(hard_r <= LBL_HARD_MAX.1, "routed hard label hits {hard_r}");
    assert!(
        stat <= LBL_EDGE_MAX.0,
        "static lbl_edge {stat} > {}",
        LBL_EDGE_MAX.0
    );
    assert!(
        routed <= LBL_EDGE_MAX.1,
        "routed lbl_edge {routed} > {}",
        LBL_EDGE_MAX.1
    );
}

/// Layout shape (G3, G12): the largest object of `wide-labels` sits near
/// the middle, and no fixture is drawn as a ribbon.
#[test]
fn layout_shape_gate() {
    /// Most `center_off` of the wide-labels fixture (0.31.1: 0.70).
    const WIDE_LABELS_CENTER_OFF: f64 = 0.35;
    /// Most aspect ratio of any fixture, static. 5.0 -> 5.5 with report
    /// Q2: the canvas no longer reaches a bow's control points, and
    /// wide-labels lost the blank 68 px band they added (413 -> 345 px
    /// tall, 4.47 -> 5.36, the drawing itself unchanged); its bows
    /// narrow with B3. 5.5 -> 12.5 with B3 (report §2.2, §2.13: this
    /// ceiling locked the defect in): wide-labels' back-edge no longer
    /// loops under the row by its label's width, so the LR drawing is as
    /// flat as its row of long labels (345 -> 160 px tall, aspect 11.97;
    /// mermaid draws it at 13.6). 12.5 -> 12.6 with B7 review r3 #4: its
    /// `consumed asynchronously by the worker pool` lies on its straight
    /// line instead of above it (the head box on a straight line no
    /// longer runs the line's whole length), the canvas 2.4 px shorter
    /// (160 -> 157.6 px, aspect 12.32 -> 12.51), the drawing unchanged.
    const MAX_ASPECT: f64 = 12.6;
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    for &(name, _, _) in CEILINGS {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let m = measure_source(&src, false)
            .expect("flowchart fixture")
            .statik;
        assert!(m.aspect <= MAX_ASPECT, "{name}: aspect {:.2}", m.aspect);
        if name == "wide-labels" {
            assert!(
                m.center_off <= WIDE_LABELS_CENTER_OFF,
                "{name}: center_off {:.2}",
                m.center_off
            );
        }
    }
}

/// Most edges (over all fixtures, per mode) whose crossing gaps chain
/// into a dashed look (`dash_edges`, G1: dashes mean `-.->`). Before the
/// under-edge was chosen per crossing (always the longer line): 31
/// static, 26 routed. All of them are in `bench-75`, whose dense bands
/// cross more often than every line can hold sparse gaps. Its static
/// count is 10, not the 8 a longer search reaches: the search runs on
/// every drag frame, and those 2 lines cost about 2 ms per frame there.
/// Routed 3 -> 4 on the round-2 router's geometry (bench-75 routed
/// crosses in different bands); static 10 -> 9 there. None since a gap
/// that would chain a third into a dash on its line is drawn as a
/// notch (the over-edge's halo alone, `xnotches`: 20 static, 15
/// routed on bench-75).
const DASH_EDGES_MAX: (usize, usize) = (0, 0);
/// Most full crossing gaps on one edge (`xgap_max`) per mode (static,
/// routed). Before: 31 static, 28 routed. Routed 11 -> 15 and static
/// 14 -> 13 on the round-2 router's geometry (bench-75); routed 11
/// once a reroute stopped lying on other edges.
const XGAP_MAX: (usize, usize) = (13, 11);

/// Crossing gaps are spread so that edges stay whole lines (G1).
#[test]
fn crossing_gaps_do_not_read_as_dashes() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    let (mut dash_s, mut dash_r) = (0, 0);
    let (mut gap_s, mut gap_r) = (0, 0);
    for &(name, ..) in CEILINGS {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let rep = measure_source(&src, false).expect("flowchart fixture");
        if name != "bench-75" {
            assert_eq!(rep.statik.dash_edges, 0, "{name} static");
            assert_eq!(rep.routed.dash_edges, 0, "{name} routed");
        }
        dash_s += rep.statik.dash_edges;
        dash_r += rep.routed.dash_edges;
        gap_s = gap_s.max(rep.statik.xgap_max);
        gap_r = gap_r.max(rep.routed.xgap_max);
    }
    assert!(dash_s <= DASH_EDGES_MAX.0, "static dash_edges {dash_s}");
    assert!(dash_r <= DASH_EDGES_MAX.1, "routed dash_edges {dash_r}");
    assert!(gap_s <= XGAP_MAX.0, "static xgap_max {gap_s}");
    assert!(gap_r <= XGAP_MAX.1, "routed xgap_max {gap_r}");
}

/// Channel routing ceilings over all fixtures (G1, G4, G6), static |
/// routed, from the round-2 router (round-1 values in brackets):
/// px of edges on top of each other (5064 | 3568), the longest such run
/// (684 | 378), px cramped beside another edge (11528 | 12268), px
/// riding a frame border or node side (4444 | 4970). Routed tight
/// 9 700 -> 9 800 when `route()` stopped re-sorting the ports after the
/// nudge (a drag frame's millisecond, G13): 9 638 -> 9 770. Routed
/// overlap 3 200 -> 3 210 with the round-2 labels (the router prices
/// paths round label boxes; `dense-lr` routed 300 -> 326 px). The
/// rest tightened to the integrated pipeline (1 536 | 3 202, 204 | 378,
/// 5 952 | 9 742, 1 146 | 2 674). Re-measured on the generated dense
/// fixtures (2 920 | 7 614, 396 | 1 286, 8 516 | 21 898, 3 048 | 4 439):
/// their long edges cross nested frames, and `dense-td` routed keeps
/// one 1 286 px overlap (an open problem, not a target). Static tight
/// 8 520 -> 9 350 and hug 3 050 -> 3 110 once the static scene keeps
/// the chip spacing off title chips (a reroute that used
/// to slip past a chip now takes a lane beside another leg); routed hug
/// 4 440 -> 3 700 with the rescue and detour lanes kept off borders.
/// B3 (back-edge lanes): static hug 3 110 -> 3 010 (bench-75's
/// `svc4 -> logs` off the app frame's border, see `BORDER_LEN_MAX`),
/// routed hug 3 667 -> 3 447, tight 9 224 -> 8 142 static and
/// 20 942 -> 19 120 routed, overlap 2 746 -> 2 390 and 7 594 -> 6 748.
/// B7 (report M3, M8, see `BORDER_LEN_MAX`): static overlap 2 920 ->
/// 3 200 and routed 7 620 -> 7 770 (dense-td 1 734 -> 2 290 static,
/// 4 318 -> 5 638 routed; dense-lr static 246 -> 490), the longest run
/// 400 -> 470 and 1 290 -> 1 400 (dense-td), static tight 9 350 ->
/// 12 860 (dense-td 3 888 -> 7 072: its channels a strip's depth
/// closer; dense-lr 960 -> 2 562: the fan out of hub `n12` whose far
/// ends now meet their targets in the middle runs as a cable); hug
/// ratcheted down to 2 820 | 3 430 (dense-td static 2 208 -> 2 008).
/// B7 review r1 (#2, #3, see `BORDER_LEN_MAX`): overlap 3 200 | 7 770 ->
/// 2 950 | 7 160 (dense-td 2 290 -> 2 008 static, 5 638 -> 5 024 routed),
/// tight 12 860 | 21 900 -> 12 630 | 20 700, routed hug 3 430 -> 3 290
/// (dense-td 1 811 -> 1 674); static hug 2 820 -> 2 830, as dense-td's
/// re-laid gaps cost it (2 008 -> 2 020: its `n25 -> n38` runs 12 px
/// longer beside node `n13`), still under the 2 969 before B7.
/// B8 (report M11, see `DEFECT_BASELINE`): routed hug 3 290 -> 3 350,
/// fanout-frames' one column of frames (0 -> 62: the hub's `route()`
/// edge to its fourth frame runs 62 px down the third's border); its
/// static hug falls 212 -> 0.
const CHANNEL_MAX: [(&str, f64, f64); 4] = [
    ("overlap_len", 2_950.0, 7_160.0),
    ("overlap_max", 470.0, 1_400.0),
    ("tight_len", 12_630.0, 20_700.0),
    ("hug_len", 2_830.0, 3_350.0),
];

/// Most edges (over all fixtures, per mode) that pass a title chip
/// closer than the chip spacing (`edge_chip`, G6). Static: none. Routed
/// (`route()` only prices a touch, as the wider rule costs a drag
/// reroutes that lie on other edges, G1): 2 in `bench-75`, legs into
/// the frame the chip names passing it within the spacing, never
/// through it. Routed 2 -> 3 with report Q2 / P1: a static scene is now
/// fitted to its drawing exactly (dense-td's had 315 px of unused room
/// on the left), so `route()` over its centres sees them translated and
/// float-exact router ties fall the other way (see `MARGIN`): one leg
/// into `dense-td`'s g9, running down its border, now passes its chip
/// within the spacing.
const EDGE_CHIP_MAX: (usize, usize) = (0, 3);

/// Edges keep their channel (G1/G4) and keep off frames' title chips
/// (G6), in either mode.
#[test]
fn channels_keep_edges_apart_and_off_chips() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    let mut tot = [[0.0f64; 2]; 4];
    let mut chips = [0usize; 2];
    for &(name, ..) in CEILINGS {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let rep = measure_source(&src, false).expect("flowchart fixture");
        for (k, m) in [&rep.statik, &rep.routed].into_iter().enumerate() {
            chips[k] += m.edge_chip;
            tot[0][k] += m.overlap_len;
            tot[1][k] = tot[1][k].max(m.overlap_max);
            tot[2][k] += m.tight_len;
            tot[3][k] += m.hug_len;
        }
    }
    assert!(chips[0] <= EDGE_CHIP_MAX.0, "static edge_chip {}", chips[0]);
    assert!(chips[1] <= EDGE_CHIP_MAX.1, "routed edge_chip {}", chips[1]);
    for (row, got) in CHANNEL_MAX.iter().zip(tot) {
        assert!(
            got[0] <= row.1,
            "static {} {:.0} > {}",
            row.0,
            got[0],
            row.1
        );
        assert!(
            got[1] <= row.2,
            "routed {} {:.0} > {}",
            row.0,
            got[1],
            row.2
        );
    }
}

/// The report-M0 yardsticks (`Metrics::tail_fields`, report §2.13: the
/// defects the older columns cannot see) over all fixtures, static |
/// routed — summed, or the largest value for the `_max` rows. Ratchets
/// at the values measured when the metrics landed, the defect baseline
/// of phase B: each later package LOWERS the rows its fix brings down,
/// and never raises one without saying why. End glyphs askew to their
/// side and ends off the painted outline (§2.3, §2.5), sideways hooks
/// and hairpins (§2.13), edges leaving the frame both their ends share
/// (§2.2, §2.8), long cables of edges 6–26 px apart; the idiom rows
/// count the edges not yet drawn orthogonally (§2.1: S-cubics, dummy
/// chain splines, back-edge bows), `x_drawn` the crossings a reader
/// sees, level and plumb edges included.
///
/// B1 (report Q1 + M1): end_tilt 24 -> 14 static — every head faces
/// along its curve's own end tangent, the stroke cut back to it, and a
/// long edge whose last channel leg slanted into its node ends in a
/// straight stub along the side's normal where that keeps the curve as
/// clear as it was; the routed 19 left are back-edge bows (B3) and ends
/// the ports stage turned round a corner. `end_kink` (new) lands at
/// 62 | 81: the painter bends a curve still turning at its end into a
/// straight run only within a px of the scene's curve, which the
/// routing, port and label stages spaced; an S-cubic whose handles are
/// a fraction of its sideways reach (a wide fan's outer branches)
/// turns far more over its last head than that allows, so its stroke
/// enters the head from the side (squaring those needs the scene's own
/// geometry). As painted, the squared ends step sideways by up to that
/// px: lat_backtrack 674 -> 676 routed (three turned ends in
/// bench-75), band_len 46 446 -> 46 610 routed. idiom_spline 19 -> 25
/// static: a long edge whose channel legs were level but whose last leg
/// slanted into its node counted as orthogonal though it never was; the
/// slant now sits before its stub, where the count sees it. B1 review r3
/// (#3): end_kink 62 | 81 -> 60 | 77, band_len 46 610 -> 46 604 and
/// x_drawn 521 -> 520 routed — a curve swinging in from the side enters
/// its head along the line touching it (no S where it leaves the curve,
/// no hump before the head), and one that slips in under the head first
/// keeps its own curve up to there. The kinks left are those S-cubics:
/// their stroke meets the head at the angle the scene's curve arrives
/// at, which only the edge's own shape can change.
///
/// B2 (report Q2 / P1): a static scene is fitted to its drawing exactly,
/// so `route()` over its centres sees translated inputs (dense-td moves
/// 315 px left, bench-75 4 px up) and float-exact router ties fall the
/// other way, the static drawings unchanged: routed band_pairs 117 ->
/// 118 (one more short cable in dense-td), band_len 46 604 -> 46 246 and
/// x_drawn 520 -> 517 (bench-75).
///
/// B3 (report Q4 + M4, §2.2): every back-edge runs in an orthogonal lane
/// beside what lies between its ends, or across the gap when nothing
/// does and its ends lie near enough across the flow, so the obstacle
/// bows are gone and so are their grazing ends (end_tilt 14 | 19 ->
/// 8 | 11, end_tilt_max 86.3 -> 76.8 static, end_kink 60 | 77 ->
/// 55 | 71) and the escapes from an edge's own frame (own_frame 1 / 32 px
/// -> 0); the lanes nest without crossing, are planned on the edges as
/// drawn and handed to the router first, so the drawings cross less
/// (x_drawn 521 | 517 -> 505 | 508; bench-75 364 | 377 -> 349 | 375) and
/// cable less (band_pairs 93 | 118 -> 91 | 101, band_len 41 640 -> 41 064
/// static: 40 580 until bench-75's `svc4 -> logs` left the app frame's
/// border, see `BORDER_LEN_MAX`, for a channel beside `metrics -> dash`
/// and `svc5 -> objects`, 484 px of cable for 261 px along the border
/// and a wiggle); lat_backtrack 1 902 -> 1 813 static, lat_wiggles
/// 19 -> 16 routed, idiom_spline 25 -> 23 static (`svc4 -> logs` is
/// orthogonal now). idiom_bow 7 | 8 -> 2 | 1: what is left are wide-labels'
/// labelled 2-cycle, whose two curves bow apart by a label's height so
/// each label rides its own line (it covered its twin). Raised, as
/// intended drawing changes: an adjacent-layer back-edge is drawn
/// across its gap as a reversed S (report §2.2 fix 4), so idiom_scubic
/// 60 -> 61 static (dense-td's `n21 -> n16` 2-cycle, bench-75's reversed
/// `svc4 -> cache` / `svc4 -> db` where a bow was) and idiom_spline
/// 21 -> 22 routed (bench-75); routed lat_backtrack 676 -> 696
/// (wide-labels' `health probe`, whose route() curve ran under the
/// bowed return edge's label, is rerouted below it and steps 14.5 px
/// up into its target's side; bench-75's 586 -> 592)
/// and band_len 46 246 -> 47 104 (dense-td's `n43 -> n16`, whose ends
/// lie 2 000 px apart across one gap, now runs in a lane beside its
/// twin instead of sweeping along the gap through 8 edges).
///
/// B4 (report Q7): an end on a cylinder's top or bottom is put onto the
/// cap's arc (then `edgepass::cap_ends`), so the ends floating above a
/// lid or below a drum are gone (dense-td's 5): end_gaps 11 | 11 -> 2 | 3
/// and end_gap_max 4.7 -> 4.5 (what is left is dense-lr's over-full hub,
/// whose corner ports sit on its flanks where they curve in).
///
/// B6 (report M6, §2.5, P0 #9): every end on a node is put on the
/// outline as painted, along its stroke's final run
/// (`edgepass::snap_ends`), a drum's shoulder end on the shoulder's
/// nearest point, and `end_gaps` counts self-loop ends too: end_gaps
/// 2 | 3 -> 0 | 0, end_gap_max 4.5 -> 0.05 (the flattening of the
/// outline polygon the metric measures against, `OUTLINE_FLATNESS`);
/// ceiling 0.1. No other row moved.
///
/// B7 (report M3, M8, §2.4, §2.6, §2.8): lone ends meet their sides in
/// the middle, combs space evenly, nearly straight pairs are drawn
/// straight, labelled gaps keep head room and title strips no dead room
/// under a box's first row. Lowered: end_tilt 8 -> 7 static, end_kink
/// 55 | 71 -> 54 | 69, idiom_spline 23 | 22 -> 22 | 21, idiom_scubic
/// routed 81 -> 80, x_drawn 505 | 508 -> 494 | 503, lat_backtrack routed
/// 696.1 -> 667. Raised, as the closer channels of the dense fixtures
/// cost them: band_pairs 91 | 101 -> 96 | 106 and band_len 41 064 |
/// 47 104 -> 46 184 | 52 184 (dense-td's fans run as cables a strip's
/// depth nearer each other, 19 804 -> 23 042 static, 22 150 -> 27 434
/// routed; dense-lr's `n12` fan 4 670 -> 6 570 static), lat_backtrack
/// static 1 813 -> 1 814.2 (dense-td 123.2 -> 124.5).
///
/// B7 review r1 (#2, #3): a gap a box opens in keeps the box's border off
/// the lateral runs crossing it, and the static scene moves one of two
/// edges side by side under the spacing clear. Lowered: band_pairs
/// 96 | 106 -> 95 | 104, band_len 46 184 | 52 184 -> 46 030 | 51 310,
/// end_kink static 54 -> 53, idiom_spline static 22 -> 21, x_drawn
/// routed 503 -> 500. Raised, as dense-td's re-laid gaps (82 px deeper
/// over the drawing) cost it, both still at or under the values before
/// B7: x_drawn static 494 -> 498 (dense-td 88 -> 92) and idiom_spline
/// routed 21 -> 22 (dense-td 0 -> 1, a `route()` channel spline the
/// router no longer redraws orthogonally). A comb of two or more
/// arrowheads keeps its heads a clear gap apart (`ports::HEAD_COMB_MIN`,
/// B7 review r1 #7): x_drawn routed 500 -> 502 (dense-td 73 -> 76, still
/// under the 503 before), band_pairs static 95 -> 94, band_len static
/// 46 030 -> 45 800.
///
/// B8 (report M11): a narrowing step may no longer grow the drawing's
/// area or its edges' length along the flow, so fanout-frames keeps its
/// column of frames in one column (as mermaid draws it) instead of
/// stacking its third frame under its first, beside the column. The
/// hub's edges into the column's far ends cross once more: x_drawn
/// 498 | 502 -> 499 | 504 (fanout-frames 2 | 1 -> 3 | 3), and the
/// column's longer bundled runs band_len 45 800 | 51 310 -> 45 930 |
/// 51 550 (fanout-frames 268 | 114 -> 396 | 352). Its static drawing
/// sheds a border hug (212 px), a tight run, a sideways wiggle and
/// three end kinks, its canvas 34 % narrower and 9 % smaller.
const DEFECT_BASELINE: [(&str, f64, f64); 16] = [
    ("end_tilt", 7.0, 11.0),
    ("end_tilt_max", 76.8, 128.4),
    ("end_gaps", 0.0, 0.0),
    ("end_gap_max", 0.1, 0.1),
    ("lat_wiggles", 16.0, 16.0),
    ("lat_backtrack", 1_814.2, 667.0),
    ("own_frame_x", 0.0, 0.0),
    ("own_frame_len", 0.0, 0.0),
    ("band_pairs", 94.0, 104.0),
    ("band_len", 45_930.0, 51_550.0),
    ("idiom_scubic", 61.0, 80.0),
    ("idiom_spline", 21.0, 22.0),
    ("idiom_bow", 2.0, 1.0),
    ("x_drawn", 499.0, 504.0),
    ("end_kink", 53.0, 69.0),
    ("ink_out", 0.0, 0.0),
];

/// Least mean `ink` over the fixtures (static, routed): the painted
/// share of the canvas (report §6 keeps the compact, text-sized
/// drawing; B6 judges sparse canvases by it). A floor at the landing
/// values (0.0889 | 0.1048), raised as canvases tighten: 0.0960 |
/// 0.1091 once the canvas stopped reaching control points (report Q2).
const INK_MEAN_MIN: (f64, f64) = (0.095, 0.108);

/// The defect baseline of report M0 holds in either mode: no package
/// adds askew ends, gaps at outlines, sideways wiggles, own-frame
/// escapes, cables, non-orthogonal edges or crossings.
#[test]
fn defect_baseline_gate() {
    use flowmaid::quality::Agg;
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    // Per mode: every tail field of every fixture.
    type Rows = Vec<Vec<(&'static str, f64, Agg)>>;
    let mut cols: [Rows; 2] = [Vec::new(), Vec::new()];
    for &(name, ..) in CEILINGS {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let rep = measure_source(&src, false).expect("flowchart fixture");
        cols[0].push(rep.statik.tail_fields());
        cols[1].push(rep.routed.tail_fields());
    }
    let fold = |rows: &Rows, c: usize| {
        let vals = rows.iter().map(|r| r[c].1);
        match rows[0][c].2 {
            Agg::Sum => vals.sum::<f64>(),
            Agg::Max => vals.fold(f64::NEG_INFINITY, f64::max),
            Agg::Min => vals.fold(f64::INFINITY, f64::min),
            Agg::Mean => vals.sum::<f64>() / rows.len() as f64,
        }
    };
    let mut bad = Vec::new();
    for (k, mode) in ["static", "routed"].into_iter().enumerate() {
        let rows = &cols[k];
        for c in 0..rows[0].len() {
            let (col, got) = (rows[0][c].0, fold(rows, c));
            println!("{mode}\t{col}\t{got:.4}");
            // The idiom counts partition the edges: the ceilings on the
            // other three are this one's floor.
            if col == "idiom_ortho" {
                continue;
            }
            if col == "ink" {
                let min = [INK_MEAN_MIN.0, INK_MEAN_MIN.1][k];
                if got < min {
                    bad.push(format!("{mode} ink {got:.4} < floor {min}"));
                }
                continue;
            }
            let row = DEFECT_BASELINE
                .iter()
                .find(|r| r.0 == col)
                .unwrap_or_else(|| panic!("{col} has no row in DEFECT_BASELINE"));
            let max = [row.1, row.2][k];
            if got > max {
                bad.push(format!("{mode} {col} {got:.2} > ceiling {max}"));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "defect baseline regressions:\n{}",
        bad.join("\n")
    );
}

/// Report M6 / P0 #9 (§2.5): every edge end — self-loops' included — sits
/// on the outline its node is painted with, whatever the shape
/// (hexagon, parallelogram, trapezoid, cylinder, circle, diamond,
/// stadium), in `scene()`, in `route()` over its centres and in every
/// drag frame `route_partial` draws: ends used to float 7-10 px beside a
/// parallelogram's or a hexagon's flank (41 ends over the repro corpus,
/// up to 9.3 px off).
#[test]
fn every_shape_end_sits_on_its_outline() {
    use flowmaid::model::Document;
    use flowmaid::quality::{drag_frames, measure, END_GAP_TOL};
    use flowmaid::scene::{route, route_partial, scene};
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/outline");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("outline corpus")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
        .collect();
    files.sort();
    assert!(files.len() >= 5, "the repro corpus is there");
    for path in files {
        let src = std::fs::read_to_string(&path).expect("fixture");
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let g = match flowmaid::parser::parse_document(&src).expect("parses") {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => panic!("{name}: not a flowchart"),
        };
        let st = scene(&g);
        let at: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route(&g, &at);
        let check = |what: &str, sc: &flowmaid::scene::Scene| {
            let m = measure(&g, sc);
            assert!(
                m.end_gaps == 0 && m.end_gap_max <= 0.1,
                "{name} {what}: {} ends off the outline, worst {:.2} px (tolerance {END_GAP_TOL}): {:?}",
                m.end_gaps,
                m.end_gap_max,
                m.details.iter().filter(|d| d.contains("end_gap")).collect::<Vec<_>>()
            );
        };
        check("static", &st);
        check("routed", &rt);
        // Each frame from the routed drawing, and a drag carried on
        // from the frame before (fixed edges kept from it). A frame that
        // drops the node onto another (boxes overlapping, centres even
        // coincident) has no outline between them to end on: skipped.
        let (mut prev, mut base) = (at.clone(), rt.clone());
        let size: Vec<(f64, f64)> = rt.nodes.iter().map(|n| (n.w, n.h)).collect();
        for (v, centres) in drag_frames(&rt) {
            let overlaps = (0..centres.len()).any(|u| {
                u != v
                    && (centres[u].0 - centres[v].0).abs() < (size[u].0 + size[v].0) / 2.0
                    && (centres[u].1 - centres[v].1).abs() < (size[u].1 + size[v].1) / 2.0
            });
            if overlaps {
                continue;
            }
            check(
                &format!("drag of node {v}"),
                &route_partial(&g, &centres, &rt, &at),
            );
            let sc = route_partial(&g, &centres, &base, &prev);
            check(&format!("carried drag of node {v}"), &sc);
            (prev, base) = (centres, sc);
        }
    }
}

/// Report M7 / §2.14 #2: the default wrapping width turns long-labels
/// from a 672 px strip (a 616 px sliver of a diamond) into a 406 px
/// drawing, while the dense fixtures — whose labels break their own
/// lines, mermaid stacking each of those into 4-8-line towers — stay as
/// written: a node only ever gains lines when it has no break of its
/// own, and then at most one.
#[test]
fn long_labels_wrap_and_dense_labels_do_not_tower() {
    use flowmaid::model::Document;
    use flowmaid::scene::scene;
    let graph = |path: &str| {
        let src = std::fs::read_to_string(path).expect("source");
        match flowmaid::parser::parse_document(&src).expect("parses") {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => panic!("{path}: not a flowchart"),
        }
    };
    let root = env!("CARGO_MANIFEST_DIR");
    let sc = scene(&graph(&format!("{root}/examples/long-labels.mmd")));
    assert!(sc.width <= 420.0, "long-labels {} px wide", sc.width);
    for name in ["dense-td", "dense-lr", "fanout-frames", "bench-75"] {
        let g = graph(&format!("{root}/tests/fixtures/quality/{name}.mmd"));
        let sc = scene(&g);
        let mut wrapped = 0;
        for (n, s) in g.nodes.iter().zip(&sc.nodes) {
            let (was, now) = (n.label.lines().count(), s.label.lines().count());
            if n.label.contains('\n') {
                assert_eq!(now, was, "{name} {}: a broken label is left alone", n.id);
            } else {
                assert!(now <= 2, "{name} {}: {:?}", n.id, s.label);
                wrapped += usize::from(now > was);
            }
        }
        assert!(wrapped <= 1, "{name}: {wrapped} labels wrapped");
    }
}

/// Most edges per fixture (over a 1 px drag of each of its nodes in
/// turn, `route_partial` from the static scene) that move an end more
/// than [`DRAG_JUMP`] px though neither end is the dragged node. What
/// is left is edges the drag redraws because they pass near it (a
/// redrawn edge takes `route()`'s course, which may differ from
/// `scene()`'s) and labels on a node or head, redrawn on every frame.
/// B7 review r1 (#1): the M3 comb re-spaced hub sides whose ends were
/// all kept edges, and dense-lr's hub `n12` threw its fan 23-49 px on a
/// drag of `n50`: 150 / 225 / 78 jumps here, against 2 / 94 / 26 before
/// B7. A comb now takes its centre from the ends a drag keeps and is
/// not used when it would move one, and a side holding kept and redrawn
/// ends places the redrawn ones round the kept ones.
///
/// B7 review r2 (#1, #2): a side whose every end the drag keeps was
/// still re-placed when the plain rule found the base's ends unsettled
/// (a parallel pair's slant asks for more than the port spacing): a
/// state choice's labelled twins on such a side swapped across their
/// sibling on every drag of any node (`notes/spread-face` 0 -> 68,
/// `notes/fork-twins` (= verify g92) 0 -> 75 over B7). Such a side is
/// now left as drawn, which also leaves dense-lr / dense-td / bench-75
/// at 0 / 17 / 18 (from 1 / 93 / 28). Paths are under `tests/fixtures`.
///
/// B7 review r3 (#7): an edge kept from the base because it merely
/// passes near the drag had its ends re-placed by `route()`'s rules (and
/// was searched again for spacing alone): a 1 px drag of a note card
/// beside it threw it 22 px (`notes/near-drag` = verify g44, 1 -> 0).
/// Such an edge now holds its ends (`Ctx::held`) and is rerouted only
/// for what it hits: dense-td 17 -> 3, bench-75 18 -> 15.
const DRAG_JUMPS_MAX: &[(&str, usize)] = &[
    ("quality/dense-lr", 0),
    ("quality/dense-td", 3),
    ("quality/bench-75", 15),
    ("notes/spread-face", 0),
    ("notes/fork-twins", 0),
    ("notes/near-drag", 0),
];

/// How far (px) an unrelated edge's end may move under a 1 px drag
/// before [`DRAG_JUMPS_MAX`] counts it.
const DRAG_JUMP: f64 = 20.0;

/// A 1 px drag never throws the fans of nodes it does not touch
/// (report invariant: the interactive drag stays stable, B7 review r1).
#[test]
fn a_one_px_drag_leaves_unrelated_edges_put() {
    use flowmaid::model::Document;
    use flowmaid::scene::{route_partial, scene, SceneEdge};
    type P = (f64, f64);
    let ends = |e: &SceneEdge| -> (P, P) {
        match (e.waypoints.first(), e.waypoints.last()) {
            (Some(&a), Some(&b)) if e.waypoints.len() >= 2 => (a, b),
            _ => (e.bezier[0], e.bezier[3]),
        }
    };
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut bad = Vec::new();
    for &(name, max) in DRAG_JUMPS_MAX {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let g = match flowmaid::parser::parse_document(&src).expect("parses") {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => panic!("{name}: not a flowchart"),
        };
        let base = scene(&g);
        let at: Vec<P> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut jumps = Vec::new();
        for (v, node) in g.nodes.iter().enumerate() {
            let mut c = at.clone();
            c[v].0 += 1.0;
            let p = route_partial(&g, &c, &base, &at);
            for (a, b) in base.edges.iter().zip(&p.edges).take(g.edges.len()) {
                if a.from == node.id || a.to == node.id {
                    continue;
                }
                let ((a0, a1), (b0, b1)) = (ends(a), ends(b));
                let d = |x: P, y: P| (x.0 - y.0).hypot(x.1 - y.1);
                let m = d(a0, b0).max(d(a1, b1));
                if m > DRAG_JUMP {
                    jumps.push(format!("drag {}: {} -> {} {m:.1}", node.id, a.from, a.to));
                }
            }
        }
        println!("{name}: {} drag jumps", jumps.len());
        if jumps.len() > max {
            bad.push(format!(
                "{name}: {} > {max} drag jumps\n  {}",
                jumps.len(),
                jumps.join("\n  ")
            ));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Moving one top-level node declaration of dense-td (report §2.9's
/// repro: the `n45` line) to other top-level lines keeps its width
/// within [`SHUFFLE_WIDTH_MAX`] of the source order's. Over all 56 such
/// positions it lays out 4 249 – 4 492 px (+5.7 %); it was 4 155 –
/// 5 740 px, a third either way of the 4 : 1 entry to narrowing and of
/// a stacking refused for its node hits (B8 review r1 #3).
#[test]
fn moving_a_node_declaration_barely_moves_dense_td() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/quality/dense-td.mmd"
    ))
    .expect("fixture");
    let lines: Vec<&str> = src.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.trim_start().starts_with("n45["))
        .expect("n45 is declared");
    let mut rest = lines.clone();
    let decl = rest.remove(at);
    // Top-level lines (outside every subgraph block), past the header.
    let mut depth = 0usize;
    let mut tops: Vec<usize> = Vec::new();
    for (i, l) in rest.iter().enumerate() {
        let t = l.trim();
        if i >= 2 && depth == 0 {
            tops.push(i);
        }
        if t.starts_with("subgraph") {
            depth += 1;
        } else if t == "end" {
            depth -= 1;
        }
    }
    tops.push(rest.len());
    let width = |s: &str| {
        let g = match flowmaid::parser::parse_document(s).expect("parses") {
            flowmaid::Document::Flowchart(g) => g,
            _ => unreachable!("a flowchart fixture"),
        };
        flowmaid::scene::scene(&g).width
    };
    let w0 = width(&src);
    // Six positions spread over the file: after `n46`, between the
    // subgraph blocks, among the edges, at the end.
    for k in 0..6 {
        let i = tops[k * (tops.len() - 1) / 5];
        let mut moved = rest.clone();
        moved.insert(i, decl);
        let w = width(&moved.join("\n"));
        assert!(
            (w - w0).abs() <= SHUFFLE_WIDTH_MAX * w0,
            "n45 before line {i}: width {w:.0} against {w0:.0}"
        );
    }
}

/// Most a shuffle of a fixture's edge lines may change its canvas
/// width (share of the source order's): the layout's output depends on
/// declaration order only through its documented tie-breaks (report
/// §2.9, B8: "permuting the edge lines of dense-td must not change width
/// by more than ~10 %"). Measured over twelve shuffles: dense-td 4 249
/// px, its shuffles 4 249 or 4 392 (+3.4 %); dense-lr always 2 299;
/// bench-75 1 772, its shuffles 1 772 – 1 906 (+7.6 %; it was 1 772 –
/// 2 129, +20 %, while its cycle breakers followed the edge lines'
/// order alone, B8 review r1 #3).
const SHUFFLE_WIDTH_MAX: f64 = 0.10;

/// Shuffling the edge lines of the generated dense fixtures and of
/// bench-75 (nodes all declared up front, in their subgraphs) keeps
/// their width within [`SHUFFLE_WIDTH_MAX`]; all three are drawn with
/// the flow along their width or their width as the breadth the
/// layout narrows (dense-td).
#[test]
fn edge_line_order_barely_moves_the_dense_layouts() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality");
    for name in ["dense-td", "dense-lr", "bench-75"] {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mmd")).expect("fixture");
        let lines: Vec<&str> = src.lines().collect();
        let is_edge = |l: &str| ["-->", "-.->", "==>"].iter().any(|a| l.contains(a));
        let at: Vec<usize> = (0..lines.len()).filter(|&i| is_edge(lines[i])).collect();
        let width = |s: &str| {
            let g = match flowmaid::parser::parse_document(s).expect("parses") {
                flowmaid::Document::Flowchart(g) => g,
                _ => unreachable!("a flowchart fixture"),
            };
            flowmaid::scene::scene(&g).width
        };
        let w0 = width(&src);
        // Deterministic shuffles (xorshift), a few seeds.
        for seed in 1..=6u64 {
            let mut x = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            let mut order: Vec<&str> = at.iter().map(|&i| lines[i]).collect();
            for i in (1..order.len()).rev() {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                order.swap(i, (x % (i as u64 + 1)) as usize);
            }
            let mut out: Vec<&str> = lines.clone();
            for (&i, l) in at.iter().zip(order) {
                out[i] = l;
            }
            let w = width(&out.join("\n"));
            assert!(
                (w - w0).abs() <= SHUFFLE_WIDTH_MAX * w0,
                "{name} shuffle {seed}: width {w:.0} against {w0:.0}"
            );
        }
    }
}
