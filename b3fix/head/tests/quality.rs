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
    // only crosses a frame (G1 over G6). B3: static frame pairs 2 -> 3
    // (the jobs' back-edge lanes run past the `data` frame between their
    // ends, where their bows cut through and were rerouted; its static
    // defects 13 -> 11).
    ("bench-75", (0, 3, 15), (0, 1, 39)),
    // The dense fixtures and fanout-frames come from
    // `examples/fixture_gen.rs` (seeded: their own nesting, fan-outs
    // and edges, nested frames crossed by long edges), measured on the
    // round-2 pipeline when they replaced the hand-made ones.
    // Routed edge_node 2 / 3 -> 0 once an edge the searches leave
    // through nodes gets the rescue detour (frames crossed, nodes not).
    ("dense-lr", (0, 11, 13), (0, 13, 20)),
    // B3: static defects 42 -> 43 (an overlap of the hub's edges as the
    // router now draws them round n43's return edge; routed 44 -> 43).
    ("dense-td", (0, 28, 43), (0, 28, 47)),
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
    // The generated dense fixtures, as first measured.
    ("dense-lr", 9.6, 5.5),
    ("dense-td", 12.1, 3.2),
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
/// B3 (report §2.2): 1082 -> 1000 static, 2140 -> 2230 routed — the
/// back-edges of bench-75 run in orthogonal lanes past the frames between
/// their ends instead of bowing through them; in `route()`, whose router
/// has the smaller budget, two of the jobs' nested lanes into `db` ride
/// the data frame's border for part of their run.
const BORDER_LEN_MAX: (f64, f64) = (1_000.0, 2_230.0);

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
/// 0 / 4 on the generated dense fixtures. B3 (report §2.2): 0 -> 1
/// static — wide-labels' 2-cycle is drawn across its gap like mermaid's
/// (its return edge used to loop 230 px under the row), the two long
/// labels stacked in the gap; the return edge's 337 px label, on its own
/// line, still covers its twin's line near Client, the twins 14 px apart
/// on a 38 px side. Routed 4 -> 5: one more of dense-lr's bundled
/// labels sits on a stroke.
const LBL_EDGE_MAX: (usize, usize) = (1, 5);

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
    /// mermaid draws it at 13.6).
    const MAX_ASPECT: f64 = 12.5;
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
/// once a reroute stopped lying on other edges. Static 13 -> 15 with
/// B3: bench-75's `job1 -> svc1` back-edge runs in its lane round the
/// zones between, past the long edges fanning out of the hubs.
const XGAP_MAX: (usize, usize) = (15, 11);

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
/// B3 (back-edge lanes): hug 3 110 | 3 700 -> 3 130 | 4 080 — bench-75's
/// lanes ride beside node sides where its bows crossed them — while tight
/// drops 9 224 -> 7 836 static and 20 942 -> 19 704 routed, overlap
/// 2 746 -> 2 538 and 7 594 -> 7 156.
const CHANNEL_MAX: [(&str, f64, f64); 4] = [
    ("overlap_len", 2_920.0, 7_620.0),
    ("overlap_max", 400.0, 1_290.0),
    ("tight_len", 9_350.0, 21_900.0),
    ("hug_len", 3_130.0, 4_080.0),
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
/// within the spacing. Routed 3 -> 4 with B3: in bench-75 a forward
/// edge rerouted round the jobs' back-edge lanes passes a zone's chip
/// within the spacing.
const EDGE_CHIP_MAX: (usize, usize) = (0, 4);

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
/// does, so the bows are gone (idiom_bow 7 | 8 -> 0) and so are their
/// grazing ends (end_tilt 14 | 19 -> 9 | 12, end_tilt_max 86.3 -> 76.3
/// static, end_kink 60 | 77 -> 55 | 71) and dense-td's escape from its own
/// frame (own_frame 1 / 32 px -> 0, the frame grown over the lane);
/// lat_backtrack 1 902 -> 1 800, band_pairs 93 | 118 -> 90 | 109,
/// band_len 41 640 -> 36 900 static. Raised: a back-edge drawn across
/// its gap is a reversed S like its forward twin (idiom_scubic 60 | 83
/// -> 62 | 84; wide-labels, dense-td), lat_wiggles 16 -> 17 static
/// (wide-labels' `health probe` edge one bump sideways past the twins'
/// widened gap), idiom_spline 25 | 21 -> 26 | 22; bench-75's lanes run
/// round the zones between their ends where the bows cut through them,
/// and cross a few more of its 75 edges as drawn (x_drawn 521 | 517 ->
/// 525 | 522; the small diagrams lose theirs: fanout 2 -> 0, stress
/// 1 -> 0, wide-labels 1 -> 0), lat_backtrack 676 -> 696 and band_len
/// 46 246 -> 47 062 routed (the jobs' nested lanes into `db`).
const DEFECT_BASELINE: [(&str, f64, f64); 16] = [
    ("end_tilt", 9.0, 12.0),
    ("end_tilt_max", 76.3, 128.4),
    ("end_gaps", 11.0, 11.0),
    ("end_gap_max", 4.7, 4.7),
    ("lat_wiggles", 17.0, 16.0),
    ("lat_backtrack", 1_800.0, 696.0),
    ("own_frame_x", 0.0, 0.0),
    ("own_frame_len", 0.0, 0.0),
    ("band_pairs", 90.0, 109.0),
    ("band_len", 36_900.0, 47_062.0),
    ("idiom_scubic", 62.0, 84.0),
    ("idiom_spline", 26.0, 22.0),
    ("idiom_bow", 0.0, 0.0),
    ("x_drawn", 525.0, 522.0),
    ("end_kink", 55.0, 71.0),
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
