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
    println!(
        "file\tmode\t{}\tdefects\t{}",
        header.join("\t"),
        late.join("\t")
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
            println!(
                "{name}\t{mode}\t{}\t{}\t{}",
                vals.join("\t"),
                m.defects(),
                late.join("\t")
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
/// frames: 1069 / 2131.
const BORDER_LEN_MAX: (f64, f64) = (1_070.0, 2_140.0);

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
/// 0 / 4 on the generated dense fixtures.
const LBL_EDGE_MAX: (usize, usize) = (0, 4);

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
    /// Most aspect ratio of any fixture, static.
    const MAX_ASPECT: f64 = 5.0;
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
const CHANNEL_MAX: [(&str, f64, f64); 4] = [
    ("overlap_len", 2_920.0, 7_620.0),
    ("overlap_max", 400.0, 1_290.0),
    ("tight_len", 9_350.0, 21_900.0),
    ("hug_len", 3_110.0, 3_700.0),
];

/// Most edges (over all fixtures, per mode) that pass a title chip
/// closer than the chip spacing (`edge_chip`, G6). Static: none. Routed
/// (`route()` only prices a touch, as the wider rule costs a drag
/// reroutes that lie on other edges, G1): 2 in `bench-75`, legs into
/// the frame the chip names passing it within the spacing, never
/// through it.
const EDGE_CHIP_MAX: (usize, usize) = (0, 2);

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
