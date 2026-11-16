//! Tests of the block layout of subgraphs ([`crate::levels`]) on the
//! repository's fixtures and on generated diagrams: the rules of the
//! layering, one test per rule. Test-only.

use crate::layout::Layerings;
use crate::levels::{Tree, Unit};
use crate::model::{Document, Graph};
use std::collections::BTreeSet;

pub(crate) fn graph(src: &str) -> Graph {
    match crate::parser::parse_document(src).expect("parses") {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!("not a flowchart or state diagram"),
    }
}

/// A fixture by its path without the extension (under `tests/fixtures`,
/// or `examples/…`).
pub(crate) fn fixture(name: &str) -> Graph {
    let root = env!("CARGO_MANIFEST_DIR");
    let path = if name.starts_with("examples/") {
        format!("{root}/{name}.mmd")
    } else {
        format!("{root}/tests/fixtures/{name}.mmd")
    };
    graph(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}")))
}

/// Every flowchart and state fixture and example with a subgraph, by
/// name ([`fixture`]).
pub(crate) fn clustered_fixtures() -> Vec<(String, Graph)> {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut out = Vec::new();
    for (dir, prefix) in [
        ("examples", "examples/"),
        ("tests/fixtures/blocks", "blocks/"),
        ("tests/fixtures/fans", "fans/"),
        ("tests/fixtures/notes", "notes/"),
        ("tests/fixtures/outline", "outline/"),
        ("tests/fixtures/quality", "quality/"),
    ] {
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(format!("{root}/{dir}"))
            .unwrap()
            .map(|f| f.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        for f in files {
            let src = std::fs::read_to_string(&f).unwrap();
            if let Ok(Document::Flowchart(g) | Document::State(g)) =
                crate::parser::parse_document(&src)
            {
                if !g.subgraphs.is_empty() {
                    let stem = f.file_stem().unwrap().to_string_lossy();
                    out.push((format!("{prefix}{stem}"), g));
                }
            }
        }
    }
    out
}

/// The block layering of `g` (a diagram no subgraph of which sets its
/// own direction).
fn layers(g: &Graph) -> Layerings {
    crate::scene::layerings(g).expect("a diagram with subgraphs")
}

fn id(g: &Graph, name: &str) -> usize {
    g.node_index(name)
        .unwrap_or_else(|| panic!("no node {name}"))
}

/// The layer of node `name`.
fn row(g: &Graph, l: &Layerings, name: &str) -> usize {
    l.block[id(g, name)]
}

/// The first and last layer of the subgraph with id `sub`.
fn rows(g: &Graph, l: &Layerings, sub: &str) -> (usize, usize) {
    let s = g
        .subgraphs
        .iter()
        .position(|x| x.id == sub)
        .unwrap_or_else(|| panic!("no subgraph {sub}"));
    let nc = crate::scene::node_cluster_paths(g);
    let on: Vec<usize> = (0..g.nodes.len())
        .filter(|&v| nc[v].contains(&s))
        .map(|v| l.block[v])
        .collect();
    (
        on.iter().copied().min().expect("members"),
        on.iter().copied().max().expect("members"),
    )
}

/// The edges against the flow, as `from->to` by node id, sorted.
fn upstream(g: &Graph, l: &Layerings) -> Vec<String> {
    let pairs = crate::levels::Augmented::of(g).pairs;
    let stand_ins: Vec<(usize, usize)> = pairs.into_iter().flatten().collect();
    let mut out: Vec<String> = l
        .upstream
        .iter()
        .map(|&ei| {
            let (a, b) = match g.edges.get(ei) {
                Some(e) => (e.from, e.to),
                None => stand_ins[ei - g.edges.len()],
            };
            format!("{}->{}", g.nodes[a].id, g.nodes[b].id)
        })
        .collect();
    out.sort();
    out
}

/// The top level's bands: its units (a block by its subgraph id, a node
/// by its id) grouped by overlapping layers, in flow order.
fn bands(g: &Graph, l: &Layerings) -> Vec<BTreeSet<String>> {
    let tree = Tree::new(&l.paths, g.subgraphs.len());
    let mut spans: Vec<(usize, usize, String)> = tree.levels[0]
        .units
        .iter()
        .map(|&u| match u {
            Unit::Node(v) => (l.block[v], l.block[v], g.nodes[v].id.clone()),
            Unit::Block(s) => {
                let on: Vec<usize> = (0..g.nodes.len())
                    .filter(|&v| l.paths[v].contains(&s))
                    .map(|v| l.block[v])
                    .collect();
                (
                    on.iter().copied().min().unwrap_or(0),
                    on.iter().copied().max().unwrap_or(0),
                    g.subgraphs[s].id.clone(),
                )
            }
        })
        .collect();
    spans.sort();
    let mut out: Vec<BTreeSet<String>> = Vec::new();
    let mut end = 0usize;
    for (lo, hi, name) in spans {
        if out.is_empty() || lo > end {
            out.push(BTreeSet::new());
            end = hi;
        }
        end = end.max(hi);
        out.last_mut().expect("a band").insert(name);
    }
    out
}

/// The three properties of a block layering, at every level the lane
/// rule left alone (`l.paths` holds each node's chain of blocks as
/// layered): a block's members fill consecutive layers; two units a
/// link joins share no layer; sibling blocks never interleave.
fn assert_block_rows(name: &str, g: &Graph, l: &Layerings) {
    let ns = g.subgraphs.len();
    let tree = Tree::new(&l.paths, ns);
    let span = |u: Unit| -> (usize, usize) {
        match u {
            Unit::Node(v) => (l.block[v], l.block[v]),
            Unit::Block(s) => {
                let on: BTreeSet<usize> = (0..g.nodes.len())
                    .filter(|&v| l.paths[v].contains(&s))
                    .map(|v| l.block[v])
                    .collect();
                let (lo, hi) = (
                    *on.iter().next().expect("members"),
                    *on.iter().next_back().expect("members"),
                );
                assert_eq!(
                    on.len(),
                    hi - lo + 1,
                    "{name}: subgraph {} does not fill rows {lo}-{hi}",
                    g.subgraphs[s].id
                );
                (lo, hi)
            }
        }
    };
    for level in &tree.levels {
        let blocks: Vec<Unit> = level
            .units
            .iter()
            .copied()
            .filter(|u| matches!(u, Unit::Block(_)))
            .collect();
        for &a in &blocks {
            for &b in &blocks {
                let ((a0, a1), (b0, b1)) = (span(a), span(b));
                assert!(
                    !(a0 < b0 && b0 <= a1 && a1 < b1),
                    "{name}: blocks {a:?} rows {a0}-{a1} and {b:?} rows {b0}-{b1} interleave"
                );
            }
        }
    }
    // Links: the real edges, and the stand-ins of unflagged sub-edges.
    let aug = crate::levels::Augmented::of(g);
    let stand_ins = aug
        .pairs
        .iter()
        .zip(&aug.flagged)
        .filter(|(_, &flag)| !flag)
        .filter_map(|(p, _)| *p);
    for (a, b) in g.edges.iter().map(|e| (e.from, e.to)).chain(stand_ins) {
        if a == b {
            continue;
        }
        let (_, ua, ub) = tree.join(a, b);
        let ((a0, a1), (b0, b1)) = (span(ua), span(ub));
        assert!(
            a1 < b0 || b1 < a0,
            "{name}: {} -> {} joins rows {a0}-{a1} and {b0}-{b1}",
            g.nodes[a].id,
            g.nodes[b].id
        );
    }
}

/// A deterministic stream of numbers (the generator of the tests).
pub(crate) struct Lcg(pub(crate) u64);

impl Lcg {
    pub(crate) fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % below.max(1)
    }
}

/// A generated flowchart with blocks: 2 to 8 subgraphs nested up to
/// three deep, free nodes, edges between any two nodes (cycles,
/// exchanges there and back, hubs) and edges to and from whole
/// subgraphs, in direction `dir`.
pub(crate) fn generated(seed: u64, dir: &str) -> String {
    let mut r = Lcg(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xB10C);
    let blocks = 2 + r.next(7);
    let mut parent: Vec<Option<usize>> = Vec::new();
    let mut depth: Vec<usize> = Vec::new();
    for b in 0..blocks {
        let p = (b > 0 && r.next(3) == 0).then(|| r.next(b));
        let p = p.filter(|&p| depth[p] < 2);
        depth.push(p.map_or(0, |p| depth[p] + 1));
        parent.push(p);
    }
    let mut members: Vec<Vec<String>> = vec![Vec::new(); blocks];
    let mut nodes: Vec<String> = Vec::new();
    for (b, m) in members.iter_mut().enumerate() {
        for k in 0..1 + r.next(4) {
            let name = format!("b{b}n{k}");
            nodes.push(name.clone());
            m.push(name);
        }
    }
    let free = r.next(4);
    for k in 0..free {
        nodes.push(format!("f{k}"));
    }
    let mut out = format!("flowchart {dir}\n");
    for k in 0..free {
        out.push_str(&format!("    f{k}[free {k}]\n"));
    }
    fn emit(
        b: usize,
        parent: &[Option<usize>],
        members: &[Vec<String>],
        indent: usize,
        out: &mut String,
    ) {
        let pad = "    ".repeat(indent);
        out.push_str(&format!("{pad}subgraph B{b}[Block {b}]\n"));
        for m in &members[b] {
            out.push_str(&format!("{pad}    {m}\n"));
        }
        for c in (0..parent.len()).filter(|&c| parent[c] == Some(b)) {
            emit(c, parent, members, indent + 1, out);
        }
        out.push_str(&format!("{pad}end\n"));
    }
    for b in (0..blocks).filter(|&b| parent[b].is_none()) {
        emit(b, &parent, &members, 1, &mut out);
    }
    let edges = nodes.len() + r.next(nodes.len() + 1);
    for _ in 0..edges {
        let (a, b) = (r.next(nodes.len()), r.next(nodes.len()));
        if a == b {
            continue;
        }
        let arrow = ["-->", "-->", "-.->", "==>"][r.next(4)];
        let label = if r.next(3) == 0 { "|step|" } else { "" };
        out.push_str(&format!("    {} {arrow}{label} {}\n", nodes[a], nodes[b]));
        // Now and then the answer comes back.
        if r.next(6) == 0 {
            out.push_str(&format!("    {} --> {}\n", nodes[b], nodes[a]));
        }
    }
    for _ in 0..r.next(3) {
        let (v, b) = (r.next(nodes.len()), r.next(blocks));
        if r.next(2) == 0 {
            out.push_str(&format!("    {} --> B{b}\n", nodes[v]));
        } else {
            out.push_str(&format!("    B{b} --> {}\n", nodes[v]));
        }
    }
    if r.next(4) == 0 && blocks >= 2 {
        let (a, b) = (r.next(blocks), r.next(blocks));
        if a != b {
            out.push_str(&format!("    B{a} --> B{b}\n"));
        }
    }
    out
}

#[test]
fn a_subgraph_takes_consecutive_rows() {
    for (name, g) in clustered_fixtures() {
        if crate::compound::split(&g).is_some() {
            continue;
        }
        assert_block_rows(&name, &g, &layers(&g));
    }
    // `interleave`: the tall block takes rows 1-3, the source block the
    // two rows before the sink it feeds.
    let g = fixture("blocks/interleave");
    let l = layers(&g);
    assert_eq!(rows(&g, &l, "T"), (1, 3));
    assert_eq!(rows(&g, &l, "S"), (4, 5));
}

#[test]
fn linked_blocks_never_share_a_row() {
    for seed in 0..300u64 {
        let dir = ["TD", "LR", "BT", "RL"][(seed % 4) as usize];
        let src = generated(seed, dir);
        let g = graph(&src);
        if g.subgraphs.is_empty() {
            continue;
        }
        assert_block_rows(&format!("seed {seed}\n{src}"), &g, &layers(&g));
    }
}

#[test]
fn sibling_blocks_never_interleave() {
    // `l5loop`: with the source block chasing a target the pair rule
    // keeps pushing down, the rows would never settle; six rows.
    let g = fixture("blocks/l5loop");
    let l = layers(&g);
    assert_eq!(rows(&g, &l, "S"), (0, 1));
    assert_eq!(rows(&g, &l, "T"), (2, 5));
    assert_eq!(rows(&g, &l, "B"), (2, 4));
    assert_block_rows("l5loop", &g, &l);
}

#[test]
fn block_order_follows_declarations_until_it_costs_twice_the_minimum() {
    // The public fixture: the walk's order stands (four returns run up
    // to the shared services, one more than the least any order turns).
    let g = fixture("blocks/blocks-overview");
    let l = layers(&g);
    assert_eq!(
        upstream(&g, &l),
        ["ALERT->MAIL", "ALERT->PAGER", "CHARGE->MAIL", "SHIP->MAIL"]
    );
    assert!(rows(&g, &l, "EDGE").1 < rows(&g, &l, "WEB").0);
    // dense-td: a single walk turns 21 of its 47 edges; the net leaves 5.
    let g = fixture("quality/dense-td");
    assert!(upstream(&g, &layers(&g)).len() <= 5);
    // One link down: two or three links back up keep the declared
    // order, four flip it.
    for (name, front_first, turned) in [
        ("blocks/ratio2", true, 2),
        ("blocks/ratio3", true, 3),
        ("blocks/ratio4", false, 1),
    ] {
        let g = fixture(name);
        let l = layers(&g);
        assert_eq!(
            rows(&g, &l, "A").1 < rows(&g, &l, "B").0,
            front_first,
            "{name}"
        );
        assert_eq!(upstream(&g, &l).len(), turned, "{name}");
    }
    // Each pair by its own count: the unrelated pair beside it does not
    // flip a pair that keeps its order alone ...
    let g = fixture("blocks/ratio2x4");
    let l = layers(&g);
    assert!(rows(&g, &l, "B").1 < rows(&g, &l, "A").0);
    assert!(rows(&g, &l, "C").1 < rows(&g, &l, "D").0);
    // ... nor holds one that flips alone: four links back up beside a
    // there-and-back pair, which keeps its order.
    let g = fixture("blocks/ratio4mask");
    let l = layers(&g);
    assert!(rows(&g, &l, "B").1 < rows(&g, &l, "A").0);
    assert!(rows(&g, &l, "C").1 < rows(&g, &l, "D").0);
    assert_eq!(upstream(&g, &l), ["a1->b1", "d2->c2"]);
    // `cycle2`: the cycle breakers of the node walk are links of the
    // level, so they decide: the first block stays first.
    let g = fixture("blocks/cycle2");
    let l = layers(&g);
    assert!(rows(&g, &l, "A").1 < rows(&g, &l, "B").0);
}

#[test]
fn block_order_survives_reordered_statements() {
    let g = fixture("blocks/blocks-overview");
    let want = bands(&g, &layers(&g));
    assert!(want.len() >= 5, "{want:?}");
    for variant in [
        "edges-reversed",
        "edges-rotated",
        "actors-reversed",
        "subgraphs-reversed",
    ] {
        let g = fixture(&format!("blocks/blocks-overview-{variant}"));
        assert_eq!(bands(&g, &layers(&g)), want, "{variant}");
    }
}

#[test]
fn block_order_follows_first_mention() {
    // The two files differ in the order of two edge lines written
    // before the subgraph blocks, so in which node is mentioned first.
    // Each stacks its first-mentioned block first, with exactly one
    // edge against the flow and both blocks in their written inner
    // order: the node walk's mark on an inner link is dropped.
    let g = fixture("blocks/cycle2-tie");
    let l = layers(&g);
    assert!(rows(&g, &l, "B").1 < rows(&g, &l, "A").0);
    assert_eq!(upstream(&g, &l), ["a2->b1"]);
    assert!(row(&g, &l, "b1") < row(&g, &l, "b2"));
    assert!(row(&g, &l, "a1") < row(&g, &l, "a2"));
    let g = fixture("blocks/cycle2-tie-swapped");
    let l = layers(&g);
    assert!(rows(&g, &l, "A").1 < rows(&g, &l, "B").0);
    assert_eq!(upstream(&g, &l).len(), 1);
}

#[test]
fn a_ping_pong_level_stays_lanes() {
    // Two swimlanes handing a job back and forth: every stacking order
    // turns more than a third of the links and a path enters one lane
    // twice. The lanes stay side by side, every arrow forward.
    let g = fixture("blocks/swimlane");
    let l = layers(&g);
    assert_eq!(l.lanes, [None]);
    assert!(upstream(&g, &l).is_empty());
    assert_eq!(l.block, l.before);
    // With a closing edge: one back-edge, as before blocks.
    let g = fixture("blocks/swim-cycle");
    let l = layers(&g);
    assert_eq!(l.lanes, [None]);
    assert_eq!(upstream(&g, &l), ["b2->a1"]);
    // A cycle through a nested block of a dissolved lane is broken at
    // that block.
    let g = fixture("blocks/swimlane-nested");
    let l = layers(&g);
    assert_eq!(l.lanes, [None]);
    assert_eq!(upstream(&g, &l).len(), 1);
    assert_block_rows("swimlane-nested", &g, &l);
    // A there-and-back exchange is stacked, the first block first.
    for (name, turned) in [("blocks/reqresp", 1), ("blocks/reqresp22", 2)] {
        let g = fixture(name);
        let l = layers(&g);
        assert!(l.lanes.is_empty(), "{name}");
        assert!(rows(&g, &l, "A").1 < rows(&g, &l, "B").0, "{name}");
        assert_eq!(upstream(&g, &l).len(), turned, "{name}");
    }
    // The rule fires on these three fixtures and on no other.
    let mut fired: Vec<String> = Vec::new();
    for (name, g) in clustered_fixtures() {
        if crate::compound::split(&g).is_none() && !layers(&g).lanes.is_empty() {
            fired.push(name);
        }
    }
    fired.sort();
    println!("levels the lane rule keeps side by side: {fired:?}");
    assert_eq!(
        fired,
        [
            "blocks/swim-cycle",
            "blocks/swimlane",
            "blocks/swimlane-nested"
        ]
        .map(String::from)
        .into_iter()
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect::<Vec<String>>()
    );
}

#[test]
fn a_source_block_sits_beside_what_it_feeds() {
    // The build pipeline nothing links into but an actor starts beside
    // the shared services, not above everything.
    let g = fixture("blocks/blocks-overview");
    let l = layers(&g);
    assert_eq!(rows(&g, &l, "CI").0, rows(&g, &l, "EDGE").0);
    // A block with no link into it at all starts just above what it
    // feeds (`interleave`'s source block, two rows before the sink).
    let g = fixture("blocks/interleave");
    let l = layers(&g);
    assert_eq!(rows(&g, &l, "S").1 + 1, row(&g, &l, "Q"));
}

#[test]
fn a_note_card_keeps_its_row_in_its_block() {
    // A card is a member of its composite and takes a row there: the
    // layers of these two are the layers before blocks (a card after
    // the last row, a lead card before the first).
    for name in ["notes/near-drag", "blocks/note-edge-row"] {
        let g = fixture(name);
        let l = layers(&g);
        assert_eq!(l.block, l.before, "{name}");
    }
    // Every card keeps its distance, in layers, to the state it is tied
    // to. (Two fixtures hold composite self-transitions, which no longer
    // rank anything: their rows move.)
    for (name, g) in clustered_fixtures() {
        if !name.starts_with("notes/")
            || name == "notes/routed-around"
            || name == "notes/spread-face"
        {
            continue;
        }
        let l = layers(&g);
        let card = |v: usize| {
            let n = &g.nodes[v];
            crate::style::is_note_card(&n.id, n.shape, &n.style)
        };
        for e in g.edges.iter().filter(|e| card(e.from) || card(e.to)) {
            let gap = |layer: &[usize]| layer[e.to] as isize - layer[e.from] as isize;
            let (now, was) = (gap(&l.block), gap(&l.before));
            let ends = format!("{name}: {} -> {}", g.nodes[e.from].id, g.nodes[e.to].id);
            if name == "notes/fork-twins" {
                // Its nested composite is three rows as a block where it
                // was four rows with a choice between them, and the cards
                // ranked after its last state follow it up: nearer their
                // states, on the same side.
                assert!(
                    now.signum() == was.signum() && now.abs() <= was.abs(),
                    "{ends}"
                );
            } else {
                assert_eq!(now, was, "{ends}");
            }
        }
    }
}

#[test]
fn the_rows_equal_fixtures_are_listed() {
    // The fixtures in block mode whose block layers are the layers
    // before blocks: they are judged by "not drawn worse than before".
    let mut equal: Vec<String> = Vec::new();
    for (name, g) in clustered_fixtures() {
        if crate::compound::split(&g).is_some() || !crate::scene::block_mode(&g) {
            continue;
        }
        let l = layers(&g);
        if l.block == l.before {
            equal.push(name);
        }
    }
    println!("rows-equal fixtures: {equal:?}");
    for name in [
        "examples/advanced",
        "examples/subgraph",
        "quality/fanout-frames",
        "notes/near-drag",
        "notes/spread-bottom",
        "notes/straight-link",
        "notes/wide-frames",
        "blocks/p03-ext-dir",
        "blocks/p04-to-cluster",
        "blocks/p05-siblings",
        "blocks/p06-block-cycle",
        "blocks/p07-nested",
        "blocks/p08-chain3",
        "blocks/p10-mixed-rank",
        "blocks/p11-lr",
        "blocks/p12-state",
        "blocks/hub5",
        "blocks/cycle2",
        "blocks/state-note",
    ] {
        assert!(equal.iter().any(|n| n == name), "{name} is not rows-equal");
    }
}

#[test]
fn layout_clustered_returns_blocks() {
    // The public layout function lays a graph with an edge across a
    // subgraph's border out as blocks once the block layout is on.
    let g = fixture("blocks/interleave");
    let nc = crate::scene::node_cluster_paths(&g);
    let sizes = crate::layout::node_sizes(&g);
    let lo = crate::levels::with_blocks(true, || crate::layout::layout_clustered(&g, &sizes, &nc));
    let l = |name: &str| lo.nodes[id(&g, name)].layer;
    assert!(l("t1") < l("t2") && l("t2") < l("t3"));
    // The source block lies wholly after the tall one (they would
    // interleave where they are first put).
    assert!(l("s1") > l("t3"));
    let off =
        crate::levels::with_blocks(false, || crate::layout::layout_clustered(&g, &sizes, &nc));
    assert_eq!(off.nodes[id(&g, "s1")].layer, 0);
}

// --- Chains both ways (the static scene, block layout switched on).

use crate::scene::{Scene, SceneEdge};

/// The static scene of `g` laid out as blocks.
fn block_scene(g: &Graph) -> Scene {
    crate::levels::with_blocks(true, || crate::scene::scene(g))
}

/// The points an edge is drawn through: its waypoints, or the ends of
/// its single curve.
fn path_of(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.is_empty() {
        vec![e.bezier[0], e.bezier[3]]
    } else {
        e.waypoints.clone()
    }
}

/// A node's box as (x0, y0, x1, y1).
fn node_box(sc: &Scene, id: &str) -> (f64, f64, f64, f64) {
    let n = sc
        .nodes
        .iter()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("no node {id}"));
    (
        n.x - n.w / 2.0,
        n.y - n.h / 2.0,
        n.x + n.w / 2.0,
        n.y + n.h / 2.0,
    )
}

/// A frame as (x0, y0, x1, y1).
fn frame_box(sc: &Scene, id: &str) -> (f64, f64, f64, f64) {
    let c = sc
        .clusters
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("no frame {id}"));
    (c.x, c.y, c.x + c.w, c.y + c.h)
}

fn edge<'a>(sc: &'a Scene, from: &str, to: &str) -> &'a SceneEdge {
    sc.edges
        .iter()
        .find(|e| e.from == from && e.to == to)
        .unwrap_or_else(|| panic!("no edge {from} -> {to}"))
}

/// Where `p` lies on the box `b`: 'u' on its upstream side, 'd' on its
/// downstream side (for flow direction `dir`), 'l' on a lateral side.
fn side_of(dir: &str, b: (f64, f64, f64, f64), p: (f64, f64)) -> char {
    let near = |a: f64, c: f64| (a - c).abs() < 1.5;
    let (top, bottom, left, right) = (
        near(p.1, b.1),
        near(p.1, b.3),
        near(p.0, b.0),
        near(p.0, b.2),
    );
    let (up, down) = match dir {
        "TD" => (top, bottom),
        "BT" => (bottom, top),
        "LR" => (left, right),
        _ => (right, left),
    };
    if up {
        'u'
    } else if down {
        'd'
    } else {
        'l'
    }
}

#[test]
fn a_long_edge_of_a_block_layout_runs_in_a_channel() {
    // No edge of a block layout is one curve across more than one layer:
    // forward or against the flow it has its channel. Only an upstream
    // edge between two plain nodes of one level keeps its lane, a note
    // card's link elbows, and an invisible link is not drawn.
    for (name, g) in clustered_fixtures() {
        if crate::compound::split(&g).is_some() || !crate::scene::block_mode(&g) {
            continue;
        }
        let aug = crate::levels::Augmented::of(&g);
        let rank_only = aug.rank_only();
        let (mut ga, stand_in) = aug.graph(&g);
        ga.block_links = Some(aug.block_links(&stand_in, ga.edges.len()));
        let sizes = crate::layout::node_sizes(&g);
        let (lo, _, paths) =
            crate::layout::layout_clustered_ranked(&ga, &sizes, &aug.node_cluster, &rank_only);
        let paths = paths.expect("laid out as blocks").0;
        let card = |v: usize| {
            let n = &g.nodes[v];
            crate::style::is_note_card(&n.id, n.shape, &n.style)
        };
        for (ei, e) in g.edges.iter().enumerate() {
            let (a, b) = (lo.nodes[e.from].layer, lo.nodes[e.to].layer);
            let lane = b < a && crate::layout::keeps_lane(Some(&paths), e.from, e.to);
            if matches!(e.kind, crate::model::EdgeKind::Invisible)
                || card(e.from)
                || card(e.to)
                || lane
            {
                continue;
            }
            assert_eq!(
                lo.edge_paths[ei].len(),
                a.abs_diff(b).saturating_sub(1),
                "{name}: {} -> {} across layers {a} to {b}",
                g.nodes[e.from].id,
                g.nodes[e.to].id
            );
        }
    }
}

#[test]
fn a_turned_edge_runs_up_its_channel_through_flow_sides() {
    // `b2 -> a2` of the two-block cycle runs against the flow across
    // two layers: out of its source's upstream side, into its target's
    // downstream side, in every direction.
    let root = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{root}/tests/fixtures/blocks/cycle2.mmd")).unwrap();
    for dir in ["TD", "BT", "LR", "RL"] {
        let g = graph(&src.replace("flowchart TD", &format!("flowchart {dir}")));
        let sc = block_scene(&g);
        let e = edge(&sc, "b2", "a2");
        let path = path_of(e);
        assert_eq!(
            side_of(dir, node_box(&sc, "b2"), path[0]),
            'u',
            "{dir}: {path:?}"
        );
        assert_eq!(
            side_of(dir, node_box(&sc, "a2"), path[path.len() - 1]),
            'd',
            "{dir}: {path:?}"
        );
    }
}

/// Whether the polyline `path` runs through the inside of box `b` (a
/// px in from its sides).
fn runs_through(path: &[(f64, f64)], b: (f64, f64, f64, f64)) -> bool {
    let b = (b.0 + 1.0, b.1 + 1.0, b.2 - 1.0, b.3 - 1.0);
    path.windows(2).any(|w| {
        // Sampled: the stretches are axis legs and short roundings.
        let steps = ((w[1].0 - w[0].0).abs().max((w[1].1 - w[0].1).abs()) / 2.0).ceil() as usize;
        (0..=steps.max(1)).any(|k| {
            let t = k as f64 / steps.max(1) as f64;
            let p = (
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            );
            p.0 > b.0 && p.0 < b.2 && p.1 > b.1 && p.1 < b.3
        })
    })
}

#[test]
fn a_turned_edge_of_a_whole_block_runs_up_its_channel_too() {
    // An edge from a whole block that the layering turned is its
    // stand-in's channel, cut back to the block's frame like a forward
    // one: out of the frame's upstream side, beside the block between,
    // into its target's downstream side. (Cut for a forward channel
    // only, it was drawn as one curve from frame to node, straight
    // through that block.)
    let src = "flowchart TD\ntop --> A\nsubgraph A[First]\na1 --> a2\nend\n\
               subgraph B[Second]\nb1 --> b2\nend\nA --> B\nB --> top\n";
    for dir in ["TD", "BT", "LR", "RL"] {
        let g = graph(&src.replace("flowchart TD", &format!("flowchart {dir}")));
        let sc = block_scene(&g);
        let path = path_of(edge(&sc, "B", "top"));
        assert_eq!(
            side_of(dir, frame_box(&sc, "B"), path[0]),
            'u',
            "{dir}: {path:?}"
        );
        assert_eq!(
            side_of(dir, node_box(&sc, "top"), path[path.len() - 1]),
            'd',
            "{dir}: {path:?}"
        );
        assert!(
            !runs_through(&path, frame_box(&sc, "A")),
            "{dir}: through the block between: {path:?}"
        );
    }
    // A state diagram: `Running --> Idle` past the composite `Review`,
    // which holds a note card on its last row.
    let src = "stateDiagram-v2\n[*] --> Idle\nIdle --> Review : submit\nstate Review {\n\
               [*] --> Queue\nQueue --> Check\nCheck --> Approve : ok\nCheck --> Reject : bad\n\
               Check --> Escalate\nnote right of Check : four eyes\n}\n\
               Review --> Running : approved\nstate Running {\n[*] --> Boot\nBoot --> Serve\n\
               Serve --> Drain : stop\nstate Serve {\n[*] --> Warm\nWarm --> Hot\n}\n}\n\
               Running --> Failed : crash\nFailed --> Review : retry\nRunning --> Idle : done\n\
               Reject --> Idle\nnote left of Failed : pager\nFailed --> [*]\n";
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(src, false))
        .expect("a state diagram");
    assert!(rep.block_mode);
    let sc = &rep.static_scene;
    let path = path_of(edge(sc, "Running", "Idle"));
    assert_eq!(side_of("TD", frame_box(sc, "Running"), path[0]), 'u');
    assert!(!runs_through(&path, frame_box(sc, "Review")), "{path:?}");
    assert_eq!(rep.statik.edge_frame_pairs, 0, "{:?}", rep.statik.details);
}

#[test]
fn the_steps_of_a_whole_blocks_turned_edges_stay_in_their_gap() {
    // blocks/turned-blocks: three stacked blocks, the last of which
    // sends two edges back up (to the first block and to the node above
    // it) while one of its members sends a third. An edge of a whole
    // block ends on a pin within its frame's side; its channel runs
    // beside the broader block between, so it steps across from the pin
    // in the gap between the two frames. That step takes a track of the
    // gap (`layout::place`, the tracks of a cut end): the frames stand
    // far enough apart for it, and the DRAWN edge stays out of the block
    // between. (Without the room the two frames stood a frame gap
    // apart, and both edges took their step a few px inside the border
    // of the block above, which holds neither end.)
    let root = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{root}/tests/fixtures/blocks/turned-blocks.mmd"))
        .expect("the fixture");
    for dir in ["TD", "BT", "LR", "RL"] {
        let g = graph(&src.replace("flowchart TD", &format!("flowchart {dir}")));
        let sc = block_scene(&g);
        let (work, ship) = (frame_box(&sc, "WORK"), frame_box(&sc, "SHIP"));
        for to in ["INTAKE", "start"] {
            let path = path_of(edge(&sc, "SHIP", to));
            assert_eq!(
                side_of(dir, ship, path[0]),
                'u',
                "{dir} SHIP -> {to}: {path:?}"
            );
            assert!(
                !runs_through(&path, work),
                "{dir}: SHIP -> {to} runs inside the block between: {path:?}"
            );
            assert!(
                !runs_through(&path[1..], ship),
                "{dir}: SHIP -> {to} runs back into its own block: {path:?}"
            );
        }
        // The gap between the two frames holds a step's track.
        let room = match dir {
            "TD" => ship.1 - work.3,
            "BT" => work.1 - ship.3,
            "LR" => ship.0 - work.2,
            _ => work.0 - ship.2,
        };
        assert!(
            room >= crate::ortho::ROW_CLEAR + crate::ortho::TRACK_PITCH,
            "{dir}: frames {room} px apart"
        );
    }
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
        .expect("a flowchart");
    assert!(rep.block_mode);
    assert_eq!(rep.statik.edge_frame_pairs, 0, "{:?}", rep.statik.details);
}

#[test]
fn a_turned_edge_one_layer_up_is_a_z_not_a_lane() {
    // A three-block cycle whose closing edge runs one layer up: it
    // crosses the gap between its two rows, no lane round the blocks.
    // (Left to right: across the flow a title chip lies over the entry
    // of a block's first row, and until entries are kept clear of it the
    // router takes such an edge round to the member's side.)
    let src = "flowchart LR\nsubgraph A[First]\na1 --> a2\nend\nsubgraph B[Second]\nb1\nend\n\
               subgraph C[Third]\nc1\nend\na2 --> b1\nb1 --> c1\nc1 --> b1\nc1 --> a1\n";
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(src, false))
        .expect("a flowchart");
    assert!(rep.block_mode);
    assert_eq!(rep.statik.up_lanes, 0);
    assert_eq!(rep.statik.wiggles, 0);
    let sc = &rep.static_scene;
    // `c1 -> b1` runs one layer up: it stays between its two rows.
    let (b, c) = (node_box(sc, "b1"), node_box(sc, "c1"));
    for p in path_of(edge(sc, "c1", "b1")) {
        assert!(p.0 > b.0 - 1.0 && p.0 < c.2 + 1.0, "{p:?} leaves the gap");
    }
    // The fixture cycle: no upstream edge of it leaves by a side lane.
    let src = std::fs::read_to_string(format!(
        "{}/tests/fixtures/blocks/p06-block-cycle.mmd",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
        .expect("a flowchart");
    assert_eq!(rep.statik.up_lanes, 0);
    assert_eq!(rep.statik.wiggles, 0);
}

/// The block layout of `g` as the layout returns it: positions, each
/// channel point's subgraph path, and each node's chain of blocks.
fn block_layout(
    g: &Graph,
) -> (
    crate::layout::LayoutResult,
    crate::layout::EdgeGroups,
    crate::layout::BlockPaths,
) {
    let aug = crate::levels::Augmented::of(g);
    let rank_only = aug.rank_only();
    let (mut ga, stand_in) = aug.graph(g);
    ga.block_links = Some(aug.block_links(&stand_in, ga.edges.len()));
    let sizes = crate::layout::node_sizes(g);
    let (lo, groups, paths) =
        crate::layout::layout_clustered_ranked(&ga, &sizes, &aug.node_cluster, &rank_only);
    (lo, groups, paths.expect("laid out as blocks").0)
}

#[test]
fn a_chain_out_of_a_nested_block_leaves_through_its_downstream_side() {
    // `a -> c2` starts in one nested block and ends in its sibling: its
    // channel stays inside the source block down to that block's last
    // row and inside the target block from its first row on, so it
    // crosses both frames on their flow-facing sides. (With "the
    // target's path wins" the channel took the outer block's path on the
    // source block's remaining rows and lay beside it.)
    //
    // This is the layout's half: the subgraph path of each channel
    // point. The drawn edge: see
    // `an_edge_between_members_of_a_block_is_drawn_inside_it`.
    let g = fixture("blocks/nested-chain");
    let (lo, groups, _) = block_layout(&g);
    let ei = g
        .edges
        .iter()
        .position(|e| g.nodes[e.from].id == "a" && g.nodes[e.to].id == "c2")
        .expect("the edge");
    let sub = |id: &str| {
        g.subgraphs
            .iter()
            .position(|s| s.id == id)
            .expect("subgraph")
    };
    let (x, y, z) = (sub("X"), sub("Y"), sub("Z"));
    let last_of_y = lo.nodes[id(&g, "b2")].layer;
    let from = lo.nodes[id(&g, "a")].layer;
    assert!(!groups[ei].is_empty());
    for (k, path) in groups[ei].iter().enumerate() {
        let layer = from + 1 + k;
        let want = if layer <= last_of_y {
            vec![x, y]
        } else {
            vec![x, z]
        };
        assert_eq!(path, &want, "channel point on layer {layer}");
    }
}

/// Whether every point of `pts` lies in the box `b` (to a px).
fn all_inside(pts: &[(f64, f64)], b: (f64, f64, f64, f64)) -> bool {
    pts.iter()
        .all(|p| p.0 >= b.0 - 1.0 && p.0 <= b.2 + 1.0 && p.1 >= b.1 - 1.0 && p.1 <= b.3 + 1.0)
}

#[test]
fn an_edge_between_members_of_a_block_is_drawn_inside_it() {
    // blocks/nested-chain: `a -> c2` joins two members of `Outer` across
    // its two nested blocks. Its channel enters `Inner target` under that
    // block's title chip, and the router takes the edge round the chip,
    // out past the side walls of the two nested blocks: the block both
    // ends belong to grows over that way, so the DRAWN edge stays inside
    // `Outer` (it ran 177 px outside it), in every direction, and the
    // grown frame holds nothing it did not hold before.
    let root = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{root}/tests/fixtures/blocks/nested-chain.mmd"))
        .expect("the fixture");
    for dir in ["TD", "BT", "LR", "RL"] {
        let g = graph(&src.replace("flowchart TD", &format!("flowchart {dir}")));
        let sc = block_scene(&g);
        let outer = frame_box(&sc, "X");
        for e in sc.edges.iter().take(g.edges.len()) {
            let pts = crate::scene::edge_polyline(e);
            assert!(
                all_inside(&pts, outer),
                "{dir}: {} -> {} leaves Outer {outer:?}: {:?}",
                e.from,
                e.to,
                path_of(e)
            );
        }
        for n in &sc.nodes {
            let b = node_box(&sc, &n.id);
            assert!(
                all_inside(&[(b.0, b.1), (b.2, b.3)], outer),
                "{dir}: {}",
                n.id
            );
        }
    }
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
        .expect("a flowchart");
    assert!(rep.block_mode);
    assert_eq!(rep.statik.own_frame_x, 0, "{:?}", rep.statik.details);
    assert_eq!(rep.statik.node_frame, 0, "{:?}", rep.statik.details);
    assert_eq!(rep.statik.edge_frame_pairs, 0, "{:?}", rep.statik.details);
}

#[test]
fn a_blocks_lanes_keep_short_of_what_stands_beside_it() {
    // blocks/lane-beside: the composite `Retry` sends two transitions back
    // up inside itself (lanes beside its states), and `Fix`, a state of
    // the composite round it, stands beside `Retry`'s first row. The
    // frame of `Retry` grows over its lanes on every one of its rows, so
    // a lane on `Fix`'s side drew `Retry`'s frame round `Fix` (and the
    // transitions into and out of `Fix` through it): the lanes take the
    // side where nothing else stands. No node lies in a frame that does
    // not hold it, and every lane stays in `Retry`.
    let g = fixture("blocks/lane-beside");
    let sc = block_scene(&g);
    let nc = crate::scene::node_cluster_paths(&g);
    for (si, sub) in g.subgraphs.iter().enumerate() {
        let f = frame_box(&sc, &sub.id);
        for (v, n) in sc.nodes.iter().enumerate() {
            if nc[v].contains(&si) {
                continue;
            }
            let b = node_box(&sc, &n.id);
            assert!(
                b.2 <= f.0 || b.0 >= f.2 || b.3 <= f.1 || b.1 >= f.3,
                "{} lies in the frame of {} {f:?}",
                n.id,
                sub.id
            );
        }
    }
    let retry = frame_box(&sc, "Retry");
    for (from, to) in [("Gate", "Run"), ("Gate", "Plan")] {
        let pts = crate::scene::edge_polyline(edge(&sc, from, to));
        assert!(all_inside(&pts, retry), "{from} -> {to}: {pts:?}");
    }
    let src = std::fs::read_to_string(format!(
        "{}/tests/fixtures/blocks/lane-beside.mmd",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the fixture");
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
        .expect("a state diagram");
    assert!(rep.block_mode);
    assert_eq!(rep.statik.node_frame, 0, "{:?}", rep.statik.details);
    assert_eq!(rep.statik.edge_frame_pairs, 0, "{:?}", rep.statik.details);
    assert_eq!(rep.statik.own_frame_x, 0, "{:?}", rep.statik.details);
}

#[test]
fn a_step_past_a_block_keeps_out_of_its_title_strip() {
    // quality/dense-td: `n46` above the first band sends a dotted edge to
    // the whole block `g9` far below, and its channel runs beside the
    // band. The step from `n46` across to that channel lies in the gap
    // above the band: placed mid-gap it ran 1 300 px along inside the
    // title strip of `g1`, the band's outer block, which holds neither
    // end. It keeps out of every frame that holds neither end, and no
    // edge of the drawing runs through such a frame.
    let g = fixture("quality/dense-td");
    let sc = block_scene(&g);
    let g1 = frame_box(&sc, "g1");
    let pts = crate::scene::edge_polyline(edge(&sc, "n46", "g9"));
    let inside = |p: &(f64, f64)| p.0 > g1.0 && p.0 < g1.2 && p.1 > g1.1 && p.1 < g1.3;
    assert!(
        !pts.iter().any(inside),
        "n46 -> g9 runs in g1 {g1:?}: {pts:?}"
    );
    let src = std::fs::read_to_string(format!(
        "{}/tests/fixtures/quality/dense-td.mmd",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the fixture");
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
        .expect("a flowchart");
    assert!(rep.block_mode);
    assert_eq!(rep.statik.edge_frame_pairs, 0, "{:?}", rep.statik.details);
}

#[test]
fn an_edge_the_repair_searches_goes_round_the_blocks_between() {
    // quality/bench-75 (LR): `gw -> logs` joins the edge block to the ops
    // block past the app and data blocks. Both its ends spill round the
    // corner of an over-full side (`gw`'s right side and `logs`'s left
    // side hold more border ends than fit), so its channel below `app`
    // and `data` left `gw` from the bottom and climbed into `logs` from
    // below, through `metrics`. The repair's window round the two ends
    // held no way in that missed `metrics`, and its priced search over
    // the whole drawing then ran the edge straight through `app` and
    // `data`. Searched clear of every frame over the whole drawing first,
    // it goes round them. The same holds for every drawn edge of the
    // repository's block fixtures: none runs through a frame that holds
    // neither of its ends (spec 4.3 f, `xframe_runs`).
    let g = fixture("quality/bench-75");
    let sc = block_scene(&g);
    let pts = crate::scene::edge_polyline(edge(&sc, "gw", "logs"));
    for f in ["app", "data", "jobs"] {
        assert!(
            !runs_through(&pts, frame_box(&sc, f)),
            "gw -> logs runs through {f}: {:?}",
            path_of(edge(&sc, "gw", "logs"))
        );
    }
    for (name, g) in clustered_fixtures() {
        if crate::compound::split(&g).is_some() || !crate::scene::block_mode(&g) {
            continue;
        }
        let sc = block_scene(&g);
        let m = crate::quality::measure(&g, &sc);
        assert_eq!(m.edge_frame_pairs, 0, "{name}: {:?}", m.details);
    }
}

#[test]
fn a_chain_label_sits_outside_both_end_blocks() {
    // The label of an edge between two blocks sits on the chain's run
    // between them: its centre lies in no frame that holds exactly one
    // of the edge's ends.
    let g = fixture("blocks/blocks-overview");
    let sc = block_scene(&g);
    let nc = crate::scene::node_cluster_paths(&g);
    let (_, groups, _) = block_layout(&g);
    let mut checked = 0;
    for (ei, (e, se)) in g.edges.iter().zip(&sc.edges).enumerate() {
        let Some((text, at, _)) = &se.label else {
            continue;
        };
        // A chain with a run between its end blocks. (Without one —
        // two bands with no layer between them — the label keeps to
        // the gap between the frames, which the gap room has to hold.)
        let common: Vec<usize> = nc[e.from]
            .iter()
            .zip(&nc[e.to])
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| *a)
            .collect();
        if !groups[ei].contains(&common) {
            continue;
        }
        checked += 1;
        for (s, sub) in g.subgraphs.iter().enumerate() {
            if nc[e.from].contains(&s) == nc[e.to].contains(&s) {
                continue;
            }
            let f = frame_box(&sc, &sub.id);
            assert!(
                !(at.0 > f.0 && at.0 < f.2 && at.1 > f.1 && at.1 < f.3),
                "label {text:?} of {} -> {} inside frame {}",
                se.from,
                se.to,
                sub.id
            );
        }
    }
    assert!(checked >= 5, "{checked} labelled chains checked");
}

// --- The level-first ordering.

/// Per layer, the node ids in the order the block layout draws them.
fn layer_orders(g: &Graph) -> Vec<Vec<String>> {
    let (lo, _, _) = block_layout(g);
    let layers = lo.nodes.iter().map(|p| p.layer).max().map_or(0, |m| m + 1);
    (0..layers)
        .map(|l| {
            let mut on: Vec<usize> = (0..g.nodes.len())
                .filter(|&v| lo.nodes[v].layer == l)
                .collect();
            on.sort_by(|&a, &b| lo.nodes[a].b.total_cmp(&lo.nodes[b].b));
            on.into_iter().map(|v| g.nodes[v].id.clone()).collect()
        })
        .collect()
}

/// The two constraints of a level's order, read off a block layout:
/// two sibling blocks that share layers stand in one order on all of
/// them, and a channel passing a block keeps to one side of it.
fn assert_one_side(name: &str, g: &Graph) {
    let (lo, groups, _) = block_layout(g);
    let nc = crate::scene::node_cluster_paths(g);
    let ns = g.subgraphs.len();
    // Per (block, layer): the breadth span of its members.
    let mut span: std::collections::BTreeMap<(usize, usize), (f64, f64)> = Default::default();
    for (v, path) in nc.iter().enumerate() {
        let p = &lo.nodes[v];
        for &c in path {
            let e = span
                .entry((c, p.layer))
                .or_insert((f64::INFINITY, f64::NEG_INFINITY));
            *e = (e.0.min(p.b - p.bsize / 2.0), e.1.max(p.b + p.bsize / 2.0));
        }
    }
    let parent = |c: usize| g.subgraphs[c].parent;
    for a in 0..ns {
        for b in a + 1..ns {
            if parent(a) != parent(b) {
                continue;
            }
            let mut order: Option<bool> = None;
            for (&(c, l), &(a0, a1)) in span.iter().filter(|(k, _)| k.0 == a) {
                debug_assert_eq!(c, a);
                let Some(&(b0, b1)) = span.get(&(b, l)) else {
                    continue;
                };
                assert!(
                    a1 <= b0 + 0.5 || b1 <= a0 + 0.5,
                    "{name}: blocks {} and {} overlap on layer {l}",
                    g.subgraphs[a].id,
                    g.subgraphs[b].id
                );
                let first = a1 <= b0 + 0.5;
                assert!(
                    order.map_or(true, |o| o == first),
                    "{name}: blocks {} and {} change sides on layer {l}",
                    g.subgraphs[a].id,
                    g.subgraphs[b].id
                );
                order = Some(first);
            }
        }
    }
    // A channel's points at one subgraph path on consecutive layers are
    // a run: it stays on one side of every block of that level.
    for (ei, e) in g.edges.iter().enumerate() {
        let (la, lb) = (lo.nodes[e.from].layer, lo.nodes[e.to].layer);
        let layer_of = |k: usize| if la < lb { la + 1 + k } else { la - 1 - k };
        for c in 0..ns {
            let mut side: Option<(bool, &Vec<usize>)> = None;
            for (k, (point, path)) in lo.edge_paths[ei].iter().zip(&groups[ei]).enumerate() {
                let level: Vec<usize> = nc
                    .iter()
                    .find(|p| p.contains(&c))
                    .map(|p| p[..p.iter().position(|&x| x == c).unwrap_or(0)].to_vec())
                    .unwrap_or_default();
                if *path != level {
                    side = None;
                    continue;
                }
                let Some(&(c0, c1)) = span.get(&(c, layer_of(k))) else {
                    side = None;
                    continue;
                };
                let left = point.0 < (c0 + c1) / 2.0;
                if let Some((was, at)) = side {
                    assert!(
                        was == left || at != path,
                        "{name}: {} -> {} changes sides of block {}",
                        g.nodes[e.from].id,
                        g.nodes[e.to].id,
                        g.subgraphs[c].id
                    );
                }
                side = Some((left, path));
            }
        }
    }
    // A link between two rows — a whole edge, or a channel's step
    // between two of its points or to its end — whose two ends are units
    // of one level stands on one side of every block of that level that
    // spans both rows.
    for (ei, e) in g.edges.iter().enumerate() {
        if matches!(e.kind, crate::model::EdgeKind::Invisible) || e.from == e.to {
            continue;
        }
        let (la, lb) = (lo.nodes[e.from].layer, lo.nodes[e.to].layer);
        if la == lb || lo.edge_paths[ei].len() + 1 != la.abs_diff(lb) {
            continue;
        }
        let layer_of = |k: usize| if la < lb { la + 1 + k } else { la - 1 - k };
        // (breadth, layer, subgraph path) of each point of the edge.
        let mut points: Vec<(f64, usize, &[usize])> = vec![(lo.nodes[e.from].b, la, &nc[e.from])];
        for (k, (point, path)) in lo.edge_paths[ei].iter().zip(&groups[ei]).enumerate() {
            points.push((point.0, layer_of(k), path));
        }
        points.push((lo.nodes[e.to].b, lb, &nc[e.to]));
        for w in points.windows(2) {
            let ((xa, la, pa), (xb, lb, pb)) = (w[0], w[1]);
            if pa != pb {
                continue;
            }
            for c in 0..ns {
                let level: Option<Vec<usize>> = nc
                    .iter()
                    .find(|p| p.contains(&c))
                    .map(|p| p[..p.iter().position(|&x| x == c).unwrap_or(0)].to_vec());
                if level.as_deref() != Some(pa) {
                    continue;
                }
                let (Some(&(a0, a1)), Some(&(b0, b1))) = (span.get(&(c, la)), span.get(&(c, lb)))
                else {
                    continue;
                };
                assert_eq!(
                    xa < (a0 + a1) / 2.0,
                    xb < (b0 + b1) / 2.0,
                    "{name}: {} -> {} passes block {} between layers {la} and {lb}",
                    g.nodes[e.from].id,
                    g.nodes[e.to].id,
                    g.subgraphs[c].id
                );
            }
        }
    }
}

#[test]
fn a_link_between_two_rows_keeps_one_side_of_a_block() {
    // dense-td: `n41 -> n42` joins two members of one block on
    // neighbouring rows, and a nested block spans both rows. Each end
    // placed by its own links, `n41` stood after that block and `n42`
    // before it, and the link ran through it; the two now stand on one
    // side of it, and the drawn edge keeps out of its frame.
    let g = fixture("quality/dense-td");
    assert_one_side("quality/dense-td", &g);
    let sc = block_scene(&g);
    let path = path_of(edge(&sc, "n41", "n42"));
    assert!(!runs_through(&path, frame_box(&sc, "g10")), "{path:?}");
}

#[test]
fn chains_keep_one_side_of_a_block() {
    for (name, g) in clustered_fixtures() {
        if crate::compound::split(&g).is_some() || !crate::scene::block_mode(&g) {
            continue;
        }
        assert_one_side(&name, &g);
    }
    for seed in 0..120u64 {
        let dir = ["TD", "LR", "BT", "RL"][(seed % 4) as usize];
        let src = generated(seed, dir);
        let g = graph(&src);
        if !g.subgraphs.is_empty() && crate::scene::block_mode(&g) {
            assert_one_side(&format!("seed {seed}\n{src}"), &g);
        }
    }
}

#[test]
fn level_order_is_deterministic_and_bounded() {
    // The same drawing twice, on a thread of its own too (nothing of the
    // order depends on what ran before).
    for name in [
        "blocks/blocks-overview",
        "quality/dense-td",
        "blocks/nested",
    ] {
        let g = fixture(name);
        let svg = |g: &Graph| crate::scene::to_svg(&block_scene(g));
        let first = svg(&g);
        assert_eq!(first, svg(&g), "{name}");
        let other = std::thread::scope(|s| s.spawn(|| svg(&g)).join().expect("renders"));
        assert_eq!(first, other, "{name}");
    }
    // More starts than the budget grants the public fixture change
    // nothing more: the most the ordering ever takes find no better.
    let g = fixture("blocks/blocks-overview");
    let at = |starts: usize| {
        crate::levels::with_order_starts(starts, || crate::scene::to_svg(&block_scene(&g)))
    };
    let most = crate::layout::BLOCK_ORDER_PINS.2;
    assert_eq!(at(most), crate::scene::to_svg(&block_scene(&g)));
}

#[test]
fn the_level_order_keeps_the_public_fixture_within_its_crossings() {
    // The public fixture and its four permutations (edge lines reversed
    // and rotated, subgraphs and actors declared the other way round):
    // at most 7 crossings as drawn with the starts the budget grants
    // (mermaid: 5; before blocks: 14, 6, 23, 15 and 12). The half
    // sweeps alone leave 9 on every one of them: a long unit's one key
    // is the mean of what its layers ask for, and the adjacent
    // exchange after each half is what tries the neighbour that mean
    // passed over.
    for variant in [
        "",
        "-edges-reversed",
        "-edges-rotated",
        "-actors-reversed",
        "-subgraphs-reversed",
    ] {
        let name = format!("blocks/blocks-overview{variant}");
        let root = env!("CARGO_MANIFEST_DIR");
        let src = std::fs::read_to_string(format!("{root}/tests/fixtures/{name}.mmd")).unwrap();
        let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
            .expect("a flowchart");
        assert!(
            rep.statik.x_drawn <= 7,
            "{name}: {} crossings",
            rep.statik.x_drawn
        );
    }
}

/// How often the drawn paths of two edges cross (their corner or
/// waypoint polylines, segment by segment).
fn crossings_between(a: &SceneEdge, b: &SceneEdge) -> usize {
    let cross = |p: (f64, f64), q: (f64, f64), r: (f64, f64), s: (f64, f64)| {
        let side = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
        };
        side(p, q, r) * side(p, q, s) < 0.0 && side(r, s, p) * side(r, s, q) < 0.0
    };
    let (pa, pb) = (path_of(a), path_of(b));
    pa.windows(2)
        .map(|u| {
            pb.windows(2)
                .filter(|v| cross(u[0], u[1], v[0], v[1]))
                .count()
        })
        .sum()
}

#[test]
fn a_band_gap_holds_its_steps_between_the_frames() {
    // The room for the steps across a gap is measured from frame to
    // frame where frames close and open on the gap: the steps between
    // two bands of nested frames keep a track each. Measured from row
    // to row, the pads and title strips of dense-td's second and third
    // band took the room, fifteen steps shared 30 px and crossed twice
    // a pair: 151 crossings as drawn where the release before blocks
    // has 66 (the must of the dense fixture: 1.1 x that + 2).
    let root = env!("CARGO_MANIFEST_DIR");
    let src =
        std::fs::read_to_string(format!("{root}/tests/fixtures/quality/dense-td.mmd")).unwrap();
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
        .expect("a flowchart");
    assert!(
        rep.statik.x_drawn <= 74,
        "dense-td: {} crossings",
        rep.statik.x_drawn
    );
    // A small one: a fan and an edge across it out of one block into
    // the next, both with a frame nested on the facing rows. The gap
    // between the two outer frames has the room of their steps (the
    // frames' own floor is 8 px).
    let g = graph(
        "flowchart TD\n subgraph A[Above]\n  subgraph AI[Inner above]\n   a1\n   a2\n   a3\n  end\n end\n subgraph B[Below]\n  subgraph BI[Inner below]\n   b1\n   b2\n   b3\n  end\n end\n a1 --> b1\n a1 --> b2\n a1 --> b3\n a3 --> b1\n",
    );
    let sc = block_scene(&g);
    let (above, below) = (frame_box(&sc, "A"), frame_box(&sc, "B"));
    let room = below.1 - above.3;
    assert!(
        room >= crate::ortho::ROW_CLEAR + crate::ortho::TRACK_PITCH,
        "frames {room} px apart"
    );
}

#[test]
fn a_pin_moved_off_a_title_leaves_its_channel_in_place() {
    // `REG ==> ORD` ends on the frame of the order block, its channel
    // left of the channel of `LB --> QUEUE`, which enters the block's
    // first member beside it. The ports stage moves the pin out from
    // under the block's title; the channel keeps the column the
    // ordering gave it (slid along with the pin it crossed the channel
    // beside it, and the two crossed twice).
    let g = fixture("blocks/blocks-overview");
    let sc = block_scene(&g);
    let (deploy, hooks) = (edge(&sc, "REG", "ORD"), edge(&sc, "LB", "QUEUE"));
    assert_eq!(crossings_between(deploy, hooks), 0);
}

#[test]
fn the_start_order_ignores_edge_statement_order() {
    // The public fixture with its edge lines reversed, and rotated, is
    // the same graph: the same order on every layer.
    let want = layer_orders(&fixture("blocks/blocks-overview"));
    for variant in ["edges-reversed", "edges-rotated"] {
        let g = fixture(&format!("blocks/blocks-overview-{variant}"));
        assert_eq!(layer_orders(&g), want, "{variant}");
    }
}

/// The ends of `hub`'s edges in `sc`, as (place across the flow of the
/// end on the hub, place across the flow of the edge's other end, the
/// end's height on the hub): a top-down drawing's fan.
fn fan_of(sc: &Scene, hub: &str) -> Vec<(f64, f64, f64)> {
    sc.edges
        .iter()
        .filter(|e| e.from != e.to && (e.from == hub || e.to == hub))
        .map(|e| {
            let pts = path_of(e);
            let (near, far) = if e.from == hub {
                (pts[0], pts[pts.len() - 1])
            } else {
                (pts[pts.len() - 1], pts[0])
            };
            (near.0, far.0, near.1)
        })
        .collect()
}

#[test]
fn a_hub_fan_leaves_in_target_order() {
    // No two edges of one node's or frame's fan cross each other, in the
    // static scene and routed at its centres (B11's comb has run in
    // both).
    for name in ["blocks/blocks-overview", "blocks/hub5"] {
        let root = env!("CARGO_MANIFEST_DIR");
        let src = std::fs::read_to_string(format!("{root}/tests/fixtures/{name}.mmd")).unwrap();
        let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(&src, false))
            .expect("a flowchart");
        assert_eq!(rep.statik.fan_x, 0, "{name} static");
        assert_eq!(rep.routed.fan_x, 0, "{name} routed");
        // The hubs: every end on the hub's one flow-facing side (its
        // bottom border: one height), and along it in the order of the
        // nodes the edges lead to. `HUB` feeds five blocks below it;
        // the mail relay of the public fixture takes three returns
        // from the blocks below it, up their channels.
        let hub = if name.ends_with("hub5") {
            "HUB"
        } else {
            "MAIL"
        };
        for (mode, sc) in [("static", &rep.static_scene), ("routed", &rep.routed_scene)] {
            let node = sc.nodes.iter().find(|n| n.id == hub).expect("the hub");
            let mut fan = fan_of(sc, hub);
            assert!(fan.len() >= 3, "{name} {mode}: {fan:?}");
            let bottom = node.y + node.h / 2.0;
            for &(_, _, y) in &fan {
                assert!(
                    (y - bottom).abs() < 1.0,
                    "{name} {mode}: an end at {y}, bottom {bottom}"
                );
            }
            fan.sort_by(|a, b| a.0.total_cmp(&b.0));
            // (Two that end in one column — blocks stacked under the
            // hub — have no order across the flow: they nest.)
            let column = 20.0;
            for w in fan.windows(2) {
                assert!(
                    w[0].1 <= w[1].1 + column,
                    "{name} {mode}: ends at {} and {} lead to {} and {}",
                    w[0].0,
                    w[1].0,
                    w[0].1,
                    w[1].1
                );
            }
        }
    }
}

#[test]
fn crossing_neutral_members_keep_declaration_order() {
    // The shared services of the public fixture: none of its members'
    // places changes a crossing against another's, so they stand as
    // declared, whatever shuffle the winning start began with.
    let g = fixture("blocks/blocks-overview");
    let declared = ["CDN", "LB", "MAIL", "PAGER"];
    for starts in [1, 3, 7] {
        let orders = crate::levels::with_order_starts(starts, || layer_orders(&g));
        let row: Vec<&str> = orders
            .iter()
            .flatten()
            .map(String::as_str)
            .filter(|id| declared.contains(id))
            .collect();
        assert_eq!(row, declared, "{starts} starts");
    }
}

/// The ids of `g`'s nodes on the row of `id` in its block layout, by
/// place across the flow.
fn row_of(g: &Graph, id: &str) -> Vec<String> {
    layer_orders(g)
        .into_iter()
        .find(|row| row.iter().any(|x| x == id))
        .expect("a row")
}

#[test]
fn a_member_a_channel_holds_out_of_place_goes_back_with_it() {
    // `R --> I` runs up from R's last row (from `A`, the member it is
    // drawn from) past `Q` and `C`. The sweeps leave the channel right
    // of those two and `A` under it, last in its row: no crossing, and
    // `B E A` where the source says `A B E`. Neither moves back alone
    // without crossing `C`'s links; the two together do, the channel to
    // the left end of the block on every row it passes.
    let src = "flowchart TD\n    I[idle] --> Q\n    subgraph R[Review]\n        Q[queue] --> C[check]\n        C --> A[approve]\n        C --> B[reject]\n        C --> E[escalate]\n    end\n    R --> D[done]\n    R --> I\n";
    let g = graph(src);
    let most = crate::layout::BLOCK_ORDER_PINS.2;
    for starts in [1, 3, most] {
        let row = crate::levels::with_order_starts(starts, || row_of(&g, "A"));
        assert_eq!(row, ["A", "B", "E"], "{starts} starts");
    }
    let rep = crate::levels::with_blocks(true, || crate::quality::measure_source(src, false))
        .expect("a flowchart");
    assert_eq!(rep.statik.x_drawn, 0);
    // Without the edge up, and with a member's own edge out instead,
    // the row is the source's as it was.
    for (drop, add) in [
        ("    R --> I\n", ""),
        ("    R --> D[done]\n    R --> I\n", "    A --> D[done]\n"),
    ] {
        let g = graph(&format!("{}{add}", src.replace(drop, "")));
        assert_eq!(row_of(&g, "A"), ["A", "B", "E"]);
    }
}

/// The fan crossings of a drawn scene (`quality`'s `fan_x` details),
/// as `(owner side, edge, edge)`.
fn fan_crossings(g: &Graph, sc: &Scene) -> Vec<String> {
    crate::quality::measure(g, sc)
        .details
        .into_iter()
        .filter(|d| d.starts_with("fan_x"))
        .collect()
}

#[test]
fn a_hubs_fan_keeps_its_order_through_the_bands() {
    // dense-td's hub `n16` sends twelve edges out of its block's bottom
    // side through two bands and takes three turned ones back. Drawn as
    // blocks its fan crossed itself 18 times: the nine steps under its
    // block shared room for seven, the outer two 4 px apart (the gap's
    // tracks: G2), the repair took the outermost off along a frame
    // border and round the crowd, the respacing re-sorted the side by
    // chords, and the ordering left two channels of one fan crossed
    // where they run through three levels into one node (O6). The start
    // drew it with one fan crossing, a turned edge's into the hub.
    let g = fixture("quality/dense-td");
    let sc = block_scene(&g);
    let fans = fan_crossings(&g, &sc);
    assert!(fans.len() <= 1, "{fans:#?}");
    assert!(
        fans.iter()
            .all(|d| !d.contains(" n16 ") && !d.contains(" n37 ")),
        "{fans:#?}"
    );
    // Every step under the hub's block keeps a track of its own: no
    // edge of the hub runs on top of another, nor any other pair (the
    // hub's turned `n36 -> n16` leaves its block past `g8`'s title chip
    // a port's spacing from that frame's free pin: see
    // `a_free_pin_keeps_off_a_members_exit_past_the_chip`).
    let m = crate::quality::measure(&g, &sc);
    let on_top: Vec<&String> = m
        .details
        .iter()
        .filter(|d| {
            d.split_once(" overlap ")
                .is_some_and(|(a, b)| a.contains("n16") && b.contains("n16"))
        })
        .collect();
    assert!(on_top.is_empty(), "{on_top:?}");
    assert_eq!(m.overlap_pairs, 0, "{:?}", m.details);
}

#[test]
fn a_free_pin_keeps_off_a_members_exit_past_the_chip() {
    // dense-td as blocks: `n45 -> g8` ends on `g8`'s title side at a
    // free pin, its channel coming down left of the frame's two-line
    // chip; `n36`, in the frame's title row, sends the turned `n36 ->
    // n16` up out of its top on the free stretch past the chip. Both
    // took the first place past the chip, 5 px apart: the pin's
    // arrowhead ran beside the exit's stroke for 24 px (the drawing's
    // closest pair 4.99 px, its one overlap pair). The pin's gap closed
    // by the chip and the exit, it takes the next gap, past the exit;
    // its channel crosses the exit's leg once.
    let g = fixture("quality/dense-td");
    let sc = block_scene(&g);
    let pin = path_of(edge(&sc, "n45", "g8"))
        .last()
        .copied()
        .expect("a path");
    let exit = path_of(edge(&sc, "n36", "n16"))[0];
    let g8 = &sc.clusters[sc.clusters.iter().position(|c| c.id == "g8").expect("g8")];
    assert!((pin.1 - g8.y).abs() < 0.5, "{pin:?} on g8's top");
    assert!(
        (pin.0 - exit.0).abs() >= crate::ports::PORT_SPACING - 0.5,
        "pin {pin:?}, exit {exit:?}"
    );
    let m = crate::quality::measure(&g, &sc);
    assert!(
        m.min_edge_gap >= 12.1,
        "{}: {:?}",
        m.min_edge_gap,
        m.details
    );
}

#[test]
fn a_run_under_a_title_chip_moves_past_it() {
    // blocks/nested-chain: `a -> c2` ran down beside `c`, under `Inner
    // target`'s title chip, and the router took it round the chip, out
    // of and into the two inner blocks through their left walls
    // (lateral crossings 2). X2.5 moves its channel past the chip in the
    // order, down the right side of both inner blocks, and keeps its
    // column clear of the chip and of the column `b2 -> c` takes past
    // it as a displaced entry, so the entry jogs in left of the run.
    let g = fixture("blocks/nested-chain");
    let sc = block_scene(&g);
    let m = crate::quality::measure(&g, &sc);
    assert_eq!(m.lateral_x, 0, "{:?}", m.details);
    assert_eq!(m.x_drawn, 0, "{:?}", m.details);
    // A hub's fan into four blocks: the channel to `q3` ran under
    // `Second`'s chip, stepped across inside the frame and crossed the
    // hub's step to `q1` above it. Past the chip its run lies right of
    // `q1` and `q2`, and the fan is crossing-free.
    let src = "flowchart TD\n    h[hub] --> p1\n    h --> q1\n    h --> q3\n    h --> r2\n    h --> s1\n    subgraph P[First]\n        p1 --> p2\n    end\n    subgraph Q[Second]\n        q1 --> q2 --> q3\n    end\n    subgraph R[Third]\n        r1 --> r2\n    end\n    subgraph S[Fourth]\n        s1\n    end\n    p2 --> r1\n    q3 --> s1\n    r2 --> h\n";
    let g = graph(src);
    let sc = block_scene(&g);
    assert!(
        fan_crossings(&g, &sc).is_empty(),
        "{:#?}",
        fan_crossings(&g, &sc)
    );
    let m = crate::quality::measure(&g, &sc);
    assert!(m.x_drawn <= 1, "{:?}", m.details);
}

#[test]
fn an_end_of_a_title_row_member_keeps_out_of_its_chips_breadth_below_the_jog_room() {
    // dense-td as blocks: `n36` stands in `g8`'s title row, its top side
    // partly under the frame's two-line chip, and the turned `n36 ->
    // n16` leaves that side. The jog room put the chip further from the
    // side than the ports stage's shade reaches, so the end stayed under
    // the chip; the router took the edge off it and round across the
    // hub `n16`'s fan (ten fan crossings). In a block layout a title-row
    // member's side is shaded by breadth alone (2.7 F2).
    let g = fixture("quality/dense-td");
    let sc = block_scene(&g);
    let g8 = sc.clusters.iter().position(|c| c.id == "g8").expect("g8");
    let chip = crate::scene::title_head(&sc.clusters, g8, sc.width);
    let e = edge(&sc, "n36", "n16");
    let start = path_of(e)[0];
    let sep = crate::clearance::CHIP_SEP;
    assert!(
        start.0 <= chip.0 - sep || start.0 >= chip.2 + sep,
        "{start:?} under {chip:?}"
    );
    assert!(
        fan_crossings(&g, &sc).is_empty(),
        "{:#?}",
        fan_crossings(&g, &sc)
    );
}

#[test]
fn a_turned_step_keeps_off_the_runs_of_a_fan_it_crosses() {
    // bench-75 as blocks: `svc3 -> metrics` turns up across the gap
    // after the app zone to a channel point level with svc5, between the
    // ports of `svc5 -> queue` and `svc5 -> objects`. Stepped first, its
    // run on lay 5 px beside both their runs in for 110-150 px; the
    // repair took those two off round svc5's corner and across the rest
    // of its fan (fan crossings 8, the start's 6). Stepped after them it
    // crosses as many legs and lies along none.
    let g = fixture("quality/bench-75");
    let sc = block_scene(&g);
    let m = crate::quality::measure(&g, &sc);
    assert!(m.fan_x <= 6, "{:#?}", fan_crossings(&g, &sc));
    assert!(
        fan_crossings(&g, &sc).iter().all(|d| !d.contains(" svc5 ")),
        "{:#?}",
        fan_crossings(&g, &sc)
    );
    assert_eq!(m.overlap_pairs, 0, "{:?}", m.details);
}

#[test]
fn an_entry_under_a_title_chip_crosses_past_it_and_jogs_inside_the_frame() {
    // `m1`'s whole top side lies under its block's title chip. Drawn
    // into it straight, the entry ran through the chip, and the router
    // took it round, in through the block's lateral wall. As a displaced
    // entry (2.8 rule 2) it crosses the title side past the chip's end
    // and steps over to its port on a track between the title strip and
    // the title row, inside the frame, in the jog room the layout
    // reserves for it (F2).
    let src = "flowchart TD\n    A[source] --> M1\n    A --> M2\n    subgraph S[A block title over one member]\n        M1[m1]\n        M2[the second member]\n    end\n";
    let g = graph(src);
    let sc = block_scene(&g);
    let s = sc.clusters.iter().position(|c| c.id == "S").expect("frame");
    let f = &sc.clusters[s];
    let m1 = sc.nodes.iter().find(|n| n.id == "M1").expect("m1");
    let chip = crate::scene::title_head(&sc.clusters, s, sc.width);
    assert!(
        chip.0 <= m1.x - m1.w / 2.0 && chip.2 >= m1.x + m1.w / 2.0,
        "{chip:?}"
    );
    // The jog room: the frame reaches a track further above its row.
    let room = (m1.y - m1.h / 2.0) - f.y;
    assert!(
        room >= crate::layout::CLUSTER_HEADER + crate::layout::FRAME_PAD + crate::ortho::ROW_CLEAR
            - 0.5,
        "{room}"
    );
    let e = sc
        .edges
        .iter()
        .find(|e| e.from == "A" && e.to == "M1")
        .expect("edge");
    let c = crate::router::ortho_corners(e).expect("on the axes");
    // Where it crosses the frame's top: past the chip.
    let cross = c
        .windows(2)
        .find(|w| (w[0].1 - f.y) * (w[1].1 - f.y) < 0.0)
        .expect("crosses the title side");
    assert!(cross[0].0 > chip.2, "{c:?} {chip:?}");
    // Every corner below the frame's top lies inside the frame.
    for p in c.iter().filter(|p| p.1 > f.y) {
        assert!(p.0 > f.x && p.0 < f.x + f.w, "{c:?}");
    }
    let m = crate::quality::measure(&g, &sc);
    assert_eq!(
        (m.lateral_x, m.edge_chip, m.x_drawn),
        (0, 0, 0),
        "{:?}",
        m.details
    );
}

#[test]
fn a_choices_spilled_and_cornered_ends_keep_off_each_other() {
    // notes/nested-composite: the 28 px choice `K0_c` takes an entry from
    // above and sends two returns out to the left. The entry stood where
    // the builder leaned it, on the facet next to the left vertex, 1 px
    // from a return's end there (a shared port, an arrow on a start).
    // As a lone end of its side it meets that side in the middle.
    let g = fixture("notes/nested-composite");
    let m = crate::quality::measure(&g, &block_scene(&g));
    assert_eq!(
        (m.shared_ports, m.arrow_on_start),
        (0, 0),
        "{:?}",
        m.details
    );
    // notes/fork-twins: two note links of one state no longer share
    // their port either (spilled onto a side of their own, they stood at
    // its corner).
    let g = fixture("notes/fork-twins");
    let m = crate::quality::measure(&g, &block_scene(&g));
    assert_eq!(m.shared_ports, 0, "{:?}", m.details);
}

#[test]
fn a_narrowing_candidate_starts_from_the_winning_order() {
    // dense-td's first placement as blocks is a ribbon over 7 000 px
    // broad. Narrowing still gives breadth back: its candidates start
    // from the first placement's final order, not from a start of
    // their own (which would lose to the multi-start best on crossings
    // and leave the ribbon as it is).
    let g = fixture("quality/dense-td");
    let (lo, _, _) = block_layout(&g);
    assert!(lo.total_b < 5000.0, "{} px broad", lo.total_b);
}

// --- The generated sweep.

/// A generated state diagram: composites (one nested), transitions
/// across their borders and to whole composites, notes.
pub(crate) fn generated_state(seed: u64) -> String {
    let mut r = Lcg(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x57A7E);
    let composites = 2 + r.next(3);
    let mut out = String::from("stateDiagram-v2\n[*] --> A\n");
    let mut states: Vec<String> = vec!["A".into()];
    for c in 0..composites {
        out.push_str(&format!("state K{c} {{\n  [*] --> K{c}_0\n"));
        let inner = 1 + r.next(3);
        for k in 0..inner {
            states.push(format!("K{c}_{k}"));
            if k > 0 {
                out.push_str(&format!("  K{c}_{} --> K{c}_{k}\n", k - 1));
            }
        }
        if r.next(2) == 0 {
            out.push_str(&format!("  note right of K{c}_0 : n{c}\n"));
        }
        if r.next(3) == 0 {
            out.push_str(&format!("  note left of K{c}_0 : lead{c}\n"));
        }
        out.push_str("}\n");
    }
    out.push_str("A --> K0\n");
    for c in 1..composites {
        match r.next(3) {
            0 => out.push_str(&format!("K{} --> K{c}\n", c - 1)),
            1 => out.push_str(&format!("K{}_0 --> K{c}_0 : go\n", c - 1)),
            _ => out.push_str(&format!("K{}_0 --> K{c}\n", c - 1)),
        }
    }
    for _ in 0..r.next(4) {
        let (a, b) = (r.next(states.len()), r.next(states.len()));
        if a != b {
            out.push_str(&format!("{} --> {} : t\n", states[a], states[b]));
        }
    }
    out.push_str(&format!("K{} --> [*]\n", composites - 1));
    out
}

/// A generated flowchart no edge of which crosses a subgraph's border:
/// isolated subgraphs with edges inside them, and free nodes linked
/// among themselves. Never in block mode.
pub(crate) fn generated_isolated(seed: u64, dir: &str) -> String {
    let mut r = Lcg(seed.wrapping_mul(0xA24B_AED4_963E_E407) ^ 0x150);
    let mut out = format!("flowchart {dir}\n");
    for b in 0..1 + r.next(4) {
        let n = 1 + r.next(5);
        out.push_str(&format!("    subgraph S{b}[Island {b}]\n"));
        for k in 0..n {
            out.push_str(&format!("        s{b}n{k}\n"));
        }
        for _ in 0..r.next(n + 2) {
            let (a, c) = (r.next(n), r.next(n));
            if a != c {
                let label = if r.next(3) == 0 { "|in|" } else { "" };
                out.push_str(&format!("        s{b}n{a} -->{label} s{b}n{c}\n"));
            }
        }
        out.push_str("    end\n");
    }
    let free = 2 + r.next(4);
    for _ in 0..free + r.next(free) {
        let (a, c) = (r.next(free), r.next(free));
        if a != c {
            out.push_str(&format!("    f{a} --> f{c}\n"));
        }
    }
    out
}

/// The graphs of the sweep: generated flowcharts in the four
/// directions, generated state diagrams, and flowcharts whose
/// subgraphs are all isolated.
pub(crate) fn sweep() -> Vec<(String, String)> {
    let dir = |seed: u64| ["TD", "LR", "BT", "RL"][(seed % 4) as usize];
    let mut out: Vec<(String, String)> = (0..160u64)
        .map(|seed| {
            (
                format!("flow {seed} {}", dir(seed)),
                generated(seed, dir(seed)),
            )
        })
        .collect();
    out.extend((0..40u64).map(|seed| (format!("state {seed}"), generated_state(seed))));
    out.extend((0..40u64).map(|seed| {
        (
            format!("isolated {seed} {}", dir(seed)),
            generated_isolated(seed, dir(seed)),
        )
    }));
    out
}

/// The most generated diagrams of the sweep on which a block layout
/// draws hard zeros the layout before blocks did not (a ratchet: down
/// only, to 0 with the note links of B12.14). Each one's cause is
/// pinned by the sweep: a note card's link sharing its state's port
/// with another card's (seven state diagrams), or a whole block's end
/// on a lateral side of its frame on a level the lane rule dissolved,
/// where the other end lies within the block's layers (`flow 23 RL`).
const SWEEP_ADDED_MAX: usize = 8;

/// The hard zeros a block layout adds to a generated diagram (`was`
/// before blocks, `is` as blocks) have one of two causes, both judged
/// later by the spec's table: a note card's link sharing its state's
/// port with another card's link (the note links are B12.14's, lead
/// note 6: judged at checkpoint C), and a whole block's end on a
/// lateral side of its frame where the lane rule dissolved the block's
/// level and the other end lies within the block's layers (spec 2.8
/// allows a lateral side there: no side of the frame faces the other
/// end along the flow; the layout before blocks happened to put that
/// end past the block). No other kind is added.
fn assert_added_zeros(
    name: &str,
    g: &Graph,
    l: &Layerings,
    was: &crate::quality::Metrics,
    is: &crate::quality::Metrics,
) {
    for (kind, a, b) in [
        ("edge_node_x", was.edge_node_x, is.edge_node_x),
        ("wiggles", was.wiggles, is.wiggles),
        ("node_overlap", was.node_overlap, is.node_overlap),
        ("node_frame", was.node_frame, is.node_frame),
        ("frame_overlap", was.frame_overlap, is.frame_overlap),
        ("own_frame_x", was.own_frame_x, is.own_frame_x),
    ] {
        assert!(b <= a, "{name}: {kind} {a} -> {b}\n{:#?}", is.details);
    }
    let card = |id: &str| {
        g.node_index(id).is_some_and(|v| {
            let n = &g.nodes[v];
            crate::style::is_note_card(&n.id, n.shape, &n.style)
        })
    };
    let new = |d: &&String| !was.details.contains(d);
    for d in is.details.iter().filter(new) {
        let w: Vec<&str> = d.split(' ').collect();
        if w[0] == "port" {
            assert!(
                w.len() == 4 && card(w[2]) && card(w[3]),
                "{name}: a shared port that is no pair of note links: {d}"
            );
        }
        if w.len() == 2 && w[1] == "frame_side" {
            let (a, b) = w[0].split_once("->").expect("an edge");
            let sub = |id: &str| g.subgraphs.iter().position(|s| s.id == id);
            let (s, other) = match (sub(a), sub(b)) {
                (Some(s), None) => (s, b),
                (None, Some(s)) => (s, a),
                _ => panic!("{name}: a frame end between two blocks: {d}"),
            };
            assert!(
                l.lanes.contains(&g.subgraphs[s].parent),
                "{name}: a frame end on a lateral side, its level not dissolved: {d}"
            );
            let (r0, r1) = rows(g, l, &g.subgraphs[s].id);
            assert!(
                (r0..=r1).contains(&row(g, l, other)),
                "{name}: a frame end on a lateral side, the other end past the block: {d}"
            );
        }
    }
}

#[test]
fn a_generated_sweep_keeps_the_block_invariants() {
    // The constants the sweep pins.
    assert_eq!(crate::layout::BLOCK_ORDER_PINS, (12, (1, 3), 9));
    let (mut blocks, mut lanes, mut hard) = (0, Vec::new(), Vec::new());
    for (name, src) in sweep() {
        let g = graph(&src);
        if g.subgraphs.is_empty() || crate::compound::split(&g).is_some() {
            continue;
        }
        // No panic in scene() or route(), and the same drawing twice.
        let sc = block_scene(&g);
        let svg = crate::scene::to_svg(&sc);
        assert_eq!(svg, crate::scene::to_svg(&block_scene(&g)), "{name}\n{src}");
        let centres: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let routed = crate::levels::with_blocks(true, || crate::scene::route(&g, &centres));
        assert_eq!(routed.nodes.len(), sc.nodes.len(), "{name}");
        // Rows: consecutive, linked units apart, no interleaving; every
        // level's constraints acyclic and its rows settled within their
        // bound (debug assertions of the layering).
        let l = layers(&g);
        assert_block_rows(&format!("{name}\n{src}"), &g, &l);
        if !l.lanes.is_empty() {
            lanes.push(name.clone());
        }
        if !crate::scene::block_mode(&g) {
            continue;
        }
        blocks += 1;
        assert_one_side(&format!("{name}\n{src}"), &g);
        let zeros = |m: &crate::quality::Metrics| {
            m.edge_node_x
                + m.shared_ports
                + m.arrow_on_start
                + m.wiggles
                + m.frame_side
                + m.node_overlap
                + m.node_frame
                + m.frame_overlap
                + m.own_frame_x
        };
        let was = crate::quality::measure(
            &g,
            &crate::levels::with_blocks(false, || crate::scene::scene(&g)),
        );
        let is = crate::quality::measure(&g, &sc);
        let (before, now) = (zeros(&was), zeros(&is));
        if now > before {
            assert_added_zeros(&format!("{name}\n{src}"), &g, &l, &was, &is);
        }
        if before + now > 0 {
            hard.push((name, before, now));
        }
    }
    println!("sweep: {blocks} diagrams in block mode");
    println!(
        "levels the lane rule keeps side by side: {} diagrams: {lanes:?}",
        lanes.len()
    );
    // (name, before blocks, as blocks)
    println!(
        "hard zeros in the static scene, before blocks | as blocks: {} | {} diagrams, {} | {} in all: {hard:?}",
        hard.iter().filter(|h| h.1 > 0).count(),
        hard.iter().filter(|h| h.2 > 0).count(),
        hard.iter().map(|h| h.1).sum::<usize>(),
        hard.iter().map(|h| h.2).sum::<usize>(),
    );
    // Hard zeros are not 0 on every generated diagram yet (own-box
    // edges, note links and entries under a title chip are drawn by
    // the rules from before blocks until their items are built). What
    // holds, and is held here: over the sweep a block layout draws
    // fewer of them than the layout before blocks, on fewer diagrams,
    // and it adds some to few diagrams (lowered only).
    // What they add is named (`assert_added_zeros`).
    let added: Vec<&(String, usize, usize)> = hard.iter().filter(|h| h.2 > h.1).collect();
    println!(
        "blocks add hard zeros on {} diagrams: {added:?}",
        added.len()
    );
    let (was, now): (usize, usize) = hard.iter().fold((0, 0), |a, h| (a.0 + h.1, a.1 + h.2));
    assert!(now * 2 <= was, "hard zeros as blocks {now}, before {was}");
    assert!(
        hard.iter().filter(|h| h.2 > 0).count() < hard.iter().filter(|h| h.1 > 0).count(),
        "{hard:?}"
    );
    assert!(added.len() <= SWEEP_ADDED_MAX, "{added:?}");
    assert!(blocks >= 120, "{blocks}");
    // The isolated ones are never laid out as blocks.
    for seed in 0..40u64 {
        let g = graph(&generated_isolated(seed, "TD"));
        assert!(!crate::scene::block_mode(&g), "isolated {seed}");
    }
}
