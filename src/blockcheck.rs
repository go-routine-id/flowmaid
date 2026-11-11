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
