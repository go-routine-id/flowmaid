//! SCRATCH probes for the review of 75f04ae (plain_crossings). Run with
//! `--test-threads=1`: they toggle a process-wide mutant switch.
use crate::model::Document;
use crate::parser::parse_document;
use crate::scene::{edge_drawn, edge_stroke_width, Hit, Scene, MUTANT};
use std::sync::atomic::Ordering::Relaxed;

type Pt = (f64, f64);

fn set_mutant(m: u8) {
    MUTANT.store(m, Relaxed);
}

/// A plus: `l -- r` level and `t -- b` plumb across an empty middle cell.
const PLUS: &str = "architecture-beta
service x(server)[X]
service a1(server)[A1]
service a2(server)[A2]
service c1(server)[C1]
service c2(server)[C2]
service l(server)[L]
service r(server)[R]
service t(server)[T]
service b(server)[B]
x:L -- R:a1
a1:B -- T:a2
a2:B -- T:l
x:R -- L:c1
c1:B -- T:c2
c2:B -- T:r
l:B -- L:b
r:T -- R:t
l:R -- L:r
t:B -- T:b
";

/// The same with arrows on the two crossing edges.
const PLUS_ARROWS: &str = "architecture-beta
service x(server)[X]
service a1(server)[A1]
service a2(server)[A2]
service c1(server)[C1]
service c2(server)[C2]
service l(server)[L]
service r(server)[R]
service t(server)[T]
service b(server)[B]
x:L -- R:a1
a1:B -- T:a2
a2:B -- T:l
x:R -- L:c1
c1:B -- T:c2
c2:B -- T:r
l:B -- L:b
r:T -- R:t
l:R --> L:r
t:B --> T:b
";

fn arch_scene(src: &str) -> crate::architecture::ArchScene {
    match parse_document(src).unwrap() {
        Document::Architecture(a) => crate::architecture::scene(&a),
        other => panic!("{other:?}"),
    }
}

fn seg_dist(p: Pt, a: Pt, b: Pt) -> f64 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let l2 = vx * vx + vy * vy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - t * vx).hypot(p.1 - a.1 - t * vy)
}

fn poly_dist(p: Pt, w: &[Pt]) -> f64 {
    w.windows(2)
        .map(|q| seg_dist(p, q[0], q[1]))
        .fold(f64::INFINITY, f64::min)
}

/// A level and a plumb straight edge of `sc` that cross: (level, plumb, x, y).
fn straight_crossing(sc: &Scene) -> Option<(usize, usize, f64, f64)> {
    for (i, a) in sc.edges.iter().enumerate() {
        for (j, b) in sc.edges.iter().enumerate() {
            let (wa, wb) = (&a.waypoints, &b.waypoints);
            if wa.len() != 2 || wb.len() != 2 {
                continue;
            }
            let level = (wa[0].1 - wa[1].1).abs() < 1e-9;
            let plumb = (wb[0].0 - wb[1].0).abs() < 1e-9;
            if !level || !plumb {
                continue;
            }
            let (x, y) = (wb[0].0, wa[0].1);
            let inx = x > wa[0].0.min(wa[1].0) + 1.0 && x < wa[0].0.max(wa[1].0) - 1.0;
            let iny = y > wb[0].1.min(wb[1].1) + 1.0 && y < wb[0].1.max(wb[1].1) - 1.0;
            if inx && iny {
                return Some((i, j, x, y));
            }
        }
    }
    None
}

/// The router's own drawing has a crossing of two straight strokes; what
/// does a pick answer there with and without the fix?
#[test]
fn p1_real_architecture_crossing() {
    for src in [PLUS, PLUS_ARROWS] {
        set_mutant(0);
        let a = arch_scene(src);
        let sc = &a.scene;
        for (i, e) in sc.edges.iter().enumerate() {
            eprintln!("edge {i} {}->{} kind {:?} wp {} pts {:?}..{:?}", e.from, e.to, e.kind, e.waypoints.len(), e.waypoints.first(), e.waypoints.last());
        }
        let (lv, pl, cx, cy) = straight_crossing(sc).expect("a straight crossing in the drawing");
        eprintln!("crossing: level edge {lv}, plumb edge {pl} at ({cx}, {cy})");
        assert!(sc.plain_crossings);
        let k = crate::hops::HOP_R * std::f64::consts::FRAC_1_SQRT_2;
        // With the fix.
        let hops: usize = crate::hops::find(sc).iter().map(Vec::len).sum();
        let arc = sc.edge_at(cx + k, cy - k, 0.4);
        let at = sc.edge_at(cx, cy, 0.4);
        let marquee = sc.hits_in_rect(cx + 1.6, cy - 6.0, cx + 4.6, cy - 1.6);
        eprintln!("FIXED : hops {hops}, arc pick {arc:?}, crossing pick {at:?}, arc marquee {marquee:?}");
        assert_eq!(hops, 0);
        assert_eq!(arc, None);
        assert_eq!(at, Some(lv.max(pl)), "topmost stroke at the crossing");
        assert!(marquee.iter().all(|h| !matches!(h, Hit::Edge(_))));
        // The generic painter on this scene draws no arc either.
        let svg = crate::scene::to_svg(sc);
        let arcs_fixed = svg.matches(" A 5.0 5.0 ").count();
        // Without it (both reads ignore the flag).
        set_mutant(3);
        let hops_m: usize = crate::hops::find(sc).iter().map(Vec::len).sum();
        let arc_m = sc.edge_at(cx + k, cy - k, 0.4);
        let at_m = sc.edge_at(cx, cy, 0.4);
        let marquee_m = sc.hits_in_rect(cx + 1.6, cy - 6.0, cx + 4.6, cy - 1.6);
        let svg_m = crate::scene::to_svg(sc);
        let arcs_m = svg_m.matches(" A 5.0 5.0 ").count();
        eprintln!("MUTANT hops: {:?}", crate::hops::find(sc).iter().enumerate().filter(|(_, h)| !h.is_empty()).collect::<Vec<_>>());
        eprintln!("MUTANT: hops {hops_m}, arc pick {arc_m:?}, crossing pick {at_m:?}, arc marquee {marquee_m:?}");
        eprintln!("generic to_svg arcs: fixed {arcs_fixed}, mutant {arcs_m}; svg differ: {}", svg != svg_m);
        // Only the picker's quick switch ignored: still plain (legs empty).
        set_mutant(2);
        assert_eq!(sc.edge_at(cx + k, cy - k, 0.4), None, "legs_for_pick alone keeps the pick plain");
        // Only hops.rs ignored: the picker's switch alone keeps the pick plain.
        set_mutant(1);
        assert_eq!(sc.edge_at(cx + k, cy - k, 0.4), None, "HopPick::on alone keeps the pick plain");
        set_mutant(0);
        // The architecture painter's own SVG is the same either way.
        let own = crate::architecture::to_svg(&a);
        set_mutant(3);
        assert_eq!(own, crate::architecture::to_svg(&a));
        set_mutant(0);
        assert!(!own.contains(" A "));
    }
}

/// In a plain scene a pick is exactly the plain strokes: point by point
/// round the crossing, `edge_at` and `hits_in_rect` against the polyline
/// the architecture painter strokes.
#[test]
fn p2_pick_equals_plain_strokes_round_the_crossing() {
    set_mutant(0);
    let a = arch_scene(PLUS);
    let sc = &a.scene;
    let (_, _, cx, cy) = straight_crossing(sc).unwrap();
    let mut n = 0;
    let mut skipped = 0;
    let mut bad = Vec::new();
    let mut s = -15.0;
    while s <= 15.0 {
        let mut t = -15.0;
        while t <= 15.0 {
            let p = (cx + s, cy + t);
            let ds: Vec<f64> = sc.edges.iter().map(|e| poly_dist(p, &e.waypoints)).collect();
            let half: Vec<f64> = sc.edges.iter().map(|e| edge_stroke_width(e) / 2.0).collect();
            if ds.iter().zip(&half).any(|(d, h)| (d - h).abs() < 1e-6) {
                skipped += 1;
                t += 0.25;
                continue;
            }
            let want_all: Vec<usize> = (0..sc.edges.len())
                .filter(|&i| edge_drawn(&sc.edges[i]) && ds[i] <= half[i])
                .collect();
            let want_top = want_all.last().copied();
            let got = sc.edge_at(p.0, p.1, 0.0);
            let got_all: Vec<usize> = sc
                .hits_in_rect(p.0, p.1, p.0, p.1)
                .into_iter()
                .filter_map(|h| match h {
                    Hit::Edge(i) => Some(i),
                    _ => None,
                })
                .collect();
            if got != want_top || got_all != want_all {
                bad.push((s, t, got, want_top, got_all, want_all));
            }
            n += 1;
            t += 0.25;
        }
        s += 0.25;
    }
    eprintln!("checked {n} points, skipped {skipped} on a stroke's rim, {} disagree", bad.len());
    for b in bad.iter().take(10) {
        eprintln!("  {b:?}");
    }
    assert!(bad.is_empty());
    // And the same grid without the fix: how many points answer otherwise.
    set_mutant(3);
    let mut diff = 0;
    let mut s = -15.0;
    while s <= 15.0 {
        let mut t = -15.0;
        while t <= 15.0 {
            let p = (cx + s, cy + t);
            set_mutant(3);
            let m = sc.edge_at(p.0, p.1, 0.0);
            let mr = sc.hits_in_rect(p.0, p.1, p.0, p.1);
            set_mutant(0);
            let f = sc.edge_at(p.0, p.1, 0.0);
            let fr = sc.hits_in_rect(p.0, p.1, p.0, p.1);
            if m != f || mr != fr {
                diff += 1;
            }
            t += 0.25;
        }
        s += 0.25;
    }
    set_mutant(0);
    eprintln!("points whose pick the fix changes: {diff}");
    assert!(diff > 0, "the bug was real on this drawing");
}

fn class_scene(src: &str) -> (crate::model::ClassDiagram, crate::class::ClassScene) {
    match parse_document(src).unwrap() {
        Document::Class(d) => {
            let s = crate::class::scene(&d);
            (d, s)
        }
        other => panic!("{other:?}"),
    }
}

fn er_scene(src: &str) -> (crate::model::ErDiagram, crate::er::ErScene) {
    match parse_document(src).unwrap() {
        Document::Er(d) => {
            let s = crate::er::scene(&d);
            (d, s)
        }
        other => panic!("{other:?}"),
    }
}

/// Class and ER: every edge carries `paint_width`, so none hops, static
/// or routed, and the flag stays false.
#[test]
fn p3_class_and_er_take_no_hops() {
    set_mutant(0);
    let check = |name: &str, sc: &Scene| {
        assert!(!sc.plain_crossings, "{name}");
        assert!(sc.edges.iter().all(|e| e.paint_width.is_some()), "{name}");
        assert!(sc.edges.iter().all(|e| !crate::hops::hops_edge(e)), "{name}");
        assert!(crate::hops::find(sc).iter().all(Vec::is_empty), "{name}");
        // How many pairs of axis legs cross (crossings a flowchart might hop).
        let mut x = 0;
        for (i, a) in sc.edges.iter().enumerate() {
            for (j, b) in sc.edges.iter().enumerate() {
                if i == j {
                    continue;
                }
                for p in a.waypoints.windows(2) {
                    for q in b.waypoints.windows(2) {
                        let level = (p[0].1 - p[1].1).abs() < 1e-6 && (p[0].0 - p[1].0).abs() > 1.0;
                        let plumb = (q[0].0 - q[1].0).abs() < 1e-6 && (q[0].1 - q[1].1).abs() > 1.0;
                        if level
                            && plumb
                            && q[0].0 > p[0].0.min(p[1].0)
                            && q[0].0 < p[0].0.max(p[1].0)
                            && p[0].1 > q[0].1.min(q[1].1)
                            && p[0].1 < q[0].1.max(q[1].1)
                        {
                            x += 1;
                        }
                    }
                }
            }
        }
        eprintln!("{name}: {} edges, {x} level-over-plumb crossings, 0 hops", sc.edges.len());
    };
    let dense_class = "classDiagram\n A <|-- B\n A <|-- C\n A <|-- D\n B --> E\n C --> E\n D --> E\n B --> F\n C --> F\n D --> F\n E --> G\n F --> G\n A --> G\n B --> D\n A --> F\n C --> G\n";
    for (name, src) in [("class.mmd", include_str!("../examples/class.mmd")), ("dense class", dense_class)] {
        let (d, cs) = class_scene(src);
        check(name, &cs.scene);
        let centers: Vec<Pt> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut moved = centers.clone();
        for (k, c) in moved.iter_mut().enumerate() {
            c.0 += (k as f64 * 37.0) % 90.0;
            c.1 += (k as f64 * 53.0) % 70.0;
        }
        check(&format!("{name} routed"), &crate::class::route(&d, &moved).scene);
    }
    let dense_er = "erDiagram\n A ||--o{ B : x\n A ||--o{ C : x\n A ||--o{ D : x\n B ||--o{ E : x\n C ||--o{ E : x\n D ||--o{ E : x\n B ||--o{ F : x\n C ||--o{ F : x\n D ||--o{ F : x\n E ||--o{ G : x\n F ||--o{ G : x\n A ||--o{ G : x\n";
    for (name, src) in [("er.mmd", include_str!("../examples/er.mmd")), ("dense er", dense_er)] {
        let (d, es) = er_scene(src);
        check(name, &es.scene);
        let mut moved: Vec<Pt> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (k, c) in moved.iter_mut().enumerate() {
            c.0 += (k as f64 * 37.0) % 90.0;
            c.1 += (k as f64 * 53.0) % 70.0;
        }
        check(&format!("{name} routed"), &crate::er::route(&d, &moved).scene);
    }
}

/// gitGraph: does any real drawing give a hop without the fix?
#[test]
fn p4_gitgraph_real_drawings() {
    let srcs: Vec<(String, String)> = {
        let base = include_str!("../examples/gitgraph.mmd").to_string();
        let busy = "gitGraph\n commit\n branch a\n commit\n branch b\n commit\n checkout main\n commit\n merge a\n checkout b\n commit\n checkout main\n merge b\n branch c\n commit\n checkout a\n commit\n merge c\n checkout main\n merge a\n cherry-pick id: \"x\"\n".replace(" cherry-pick id: \"x\"\n", "");
        let mut v = vec![("example".to_string(), base.clone()), ("busy".to_string(), busy.to_string())];
        for o in ["TB", "BT"] {
            v.push((format!("example {o}"), base.replacen("gitGraph", &format!("gitGraph {o}:"), 1)));
            v.push((format!("busy {o}"), busy.replacen("gitGraph", &format!("gitGraph {o}:"), 1)));
        }
        v
    };
    for (name, src) in &srcs {
        let d = match parse_document(src) {
            Ok(Document::GitGraph(g)) => g,
            other => {
                eprintln!("{name}: not parsed as gitGraph ({:?})", other.map(|_| ()));
                continue;
            }
        };
        set_mutant(0);
        let gs = crate::gitgraph::scene(&d);
        let sc = &gs.scene;
        assert!(sc.plain_crossings);
        assert!(crate::hops::find(sc).iter().all(Vec::is_empty));
        let hopping = sc.edges.iter().filter(|e| crate::hops::hops_edge(e)).count();
        let straight = sc.edges.iter().filter(|e| e.waypoints.is_empty()).count();
        set_mutant(3);
        let hops: usize = crate::hops::find(sc).iter().map(Vec::len).sum();
        set_mutant(0);
        eprintln!(
            "{name}: {} edges ({straight} straight, {hopping} hops_edge), hops without the fix: {hops}",
            sc.edges.len()
        );
    }
}

/// Architecture: how many real drawings of the unit corpus hop without the fix.
#[test]
fn p5_architecture_other_drawings() {
    let srcs = [
        include_str!("../examples/architecture.mmd"),
        PLUS,
        PLUS_ARROWS,
    ];
    for (k, src) in srcs.iter().enumerate() {
        set_mutant(0);
        let a = arch_scene(src);
        assert!(a.scene.plain_crossings);
        assert!(crate::hops::find(&a.scene).iter().all(Vec::is_empty));
        set_mutant(3);
        let hops: usize = crate::hops::find(&a.scene).iter().map(Vec::len).sum();
        set_mutant(0);
        eprintln!("arch #{k}: {} edges, hops without the fix: {hops}", a.scene.edges.len());
    }
}

/// Flowchart, state, compact and routed scenes keep `plain_crossings == false`
/// and keep their hops (clone / route / route_partial / fold / compound).
#[test]
fn p6_flowchart_paths_stay_hopping() {
    set_mutant(0);
    let graph = |src: &str| match parse_document(src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        other => panic!("{other:?}"),
    };
    let mut total = 0;
    for (name, src) in [
        ("advanced", include_str!("../examples/advanced.mmd")),
        ("subgraph", include_str!("../examples/subgraph.mmd")),
        ("state", include_str!("../examples/state.mmd")),
        ("stress", include_str!("../examples/stress.mmd")),
        ("bench-75", include_str!("../tests/fixtures/quality/bench-75.mmd")),
        ("compound", "flowchart TD\n subgraph S\n direction LR\n a --> b --> c\n end\n x --> a\n c --> y\n x --> y\n"),
    ] {
        let g = graph(src);
        let sc = crate::scene::scene(&g);
        assert!(!sc.plain_crossings, "{name} scene");
        let centers: Vec<Pt> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = crate::scene::route(&g, &centers);
        assert!(!r.plain_crossings, "{name} route");
        let mut moved = centers.clone();
        if let Some(c) = moved.first_mut() {
            c.0 += 40.0;
            c.1 += 25.0;
        }
        let rp = crate::scene::route_partial(&g, &moved, &sc, &centers);
        assert!(!rp.plain_crossings, "{name} route_partial");
        let same = crate::scene::route_partial(&g, &centers, &sc, &centers);
        assert!(!same.plain_crossings, "{name} route_partial (no move)");
        let sizes: Vec<Pt> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
        let rs = crate::scene::route_sized(&g, &centers, &sizes);
        assert!(!rs.plain_crossings, "{name} route_sized");
        let c = crate::fold::scene_compact(&g, &crate::fold::CompactOptions::for_extent(600.0));
        assert!(!c.scene.plain_crossings, "{name} compact");
        let h: usize = crate::hops::find(&sc).iter().map(Vec::len).sum();
        let hr: usize = crate::hops::find(&r).iter().map(Vec::len).sum();
        eprintln!("{name}: hops static {h}, routed {hr}");
        total += h + hr;
    }
    assert!(total > 0, "flowcharts still hop");
    assert!(!Scene::empty(10.0, 10.0).plain_crossings);
}


/// The gitGraph test's geometry: which radius would its hop have, and does
/// the test's pick probe (at HOP_R) tell a hop from none?
#[test]
fn p7_gitgraph_test_probe_radius() {
    let src = "gitGraph\n commit\n branch a\n commit\n checkout main\n commit\n merge a\n";
    let d = match parse_document(src).unwrap() {
        Document::GitGraph(g) => g,
        other => panic!("{other:?}"),
    };
    set_mutant(0);
    let mut sc = crate::gitgraph::scene(&d).scene;
    let (cx, cy) = (sc.width + 200.0, sc.height + 200.0);
    sc.edges[0].waypoints = vec![(cx - 100.0, cy), (cx + 100.0, cy)];
    sc.edges[1].waypoints = vec![(cx, cy - 100.0), (cx, cy + 100.0)];
    eprintln!("stroke widths: {} {}", edge_stroke_width(&sc.edges[0]), edge_stroke_width(&sc.edges[1]));
    let k5 = 5.0 * std::f64::consts::FRAC_1_SQRT_2;
    let k8 = 8.0 * std::f64::consts::FRAC_1_SQRT_2;
    let mut seen = Vec::new();
    for m in [0u8, 3] {
        set_mutant(m);
        let hops: Vec<_> = crate::hops::find(&sc).into_iter().flatten().collect();
        let at5 = sc.edge_at(cx + k5, cy - k5, 0.4);
        let at8 = sc.edge_at(cx + k8, cy - k8, 0.4);
        let r8 = sc.hits_in_rect(cx + k8 - 0.2, cy - k8 - 0.2, cx + k8 + 0.2, cy - k8 + 0.2);
        eprintln!("mutant {m}: hops {hops:?}; pick at radius 5: {at5:?}; at radius 8: {at8:?}; marquee at radius 8: {r8:?}");
        seen.push((hops.len(), at5, at8));
    }
    set_mutant(0);
    eprintln!("test's own pick probe (radius 5) differs with/without the fix: {}", seen[0].1 != seen[1].1);
    eprintln!("a probe at the real radius (8) differs: {}", seen[0].2 != seen[1].2);
}

/// The architecture test's geometry, pick assertions alone (hops::find aside).
#[test]
fn p8_architecture_test_pick_probe() {
    let a = arch_scene(PLUS);
    set_mutant(0);
    let mut sc = a.scene.clone();
    let (cx, cy) = (sc.width + 200.0, sc.height + 200.0);
    sc.edges[0].waypoints = vec![(cx - 100.0, cy), (cx + 100.0, cy)];
    sc.edges[1].waypoints = vec![(cx, cy - 100.0), (cx, cy + 100.0)];
    let k = crate::hops::HOP_R * std::f64::consts::FRAC_1_SQRT_2;
    for m in [0u8, 3] {
        set_mutant(m);
        eprintln!(
            "mutant {m}: arc pick {:?}, marquee {:?}, at crossing {:?}",
            sc.edge_at(cx + k, cy - k, 0.4),
            sc.hits_in_rect(cx + k - 0.2, cy - k - 0.2, cx + k + 0.2, cy - k + 0.2),
            sc.edge_at(cx, cy, 0.4)
        );
    }
    set_mutant(0);
}
