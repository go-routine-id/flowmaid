//! Levels: the structure the block layout of subgraphs reads a
//! diagram by.
//!
//! In a block layout every subgraph (and state composite) with a link
//! across its border is one compact block: its members take consecutive
//! rows along the flow, two blocks an edge joins are stacked along the
//! flow, and an edge between blocks runs in the gaps between them. The
//! layering, the ordering and the routing all need the same three
//! things from the graph, and take them from here so that
//! [`crate::scene::scene`], [`crate::scene::route`] and
//! [`crate::scene::route_partial`] cannot disagree:
//!
//! - the **augmented graph** ([`Augmented`]): one stand-in edge per edge
//!   that ends on a whole subgraph (`A --> box`), between a member pair
//!   that stands for it, and the rank-only groups that go with them;
//! - **block mode** ([`Augmented::block_mode`]): whether any link joins a
//!   subgraph, or something inside it, to something outside it. A pure
//!   predicate of the graph: no ranking, no sizes, no positions;
//! - the **level tree** ([`Tree`]): the root and every non-empty
//!   subgraph is a level; a level's units are its own nodes and its
//!   child blocks; every link belongs to the level of the lowest common
//!   ancestor of its two ends and joins two units there.
//!
//! The whole block layout sits behind one internal switch
//! ([`blocks_on`]) that is off in every ordinary build, so the default
//! output stays what it was while the layout is built.

use crate::layout::RankGroup;
use crate::model::{Edge, End, Graph};
use std::cell::Cell;

/// The switch's value in this build: on only when the crate is built
/// with `--cfg flowmaid_blocks` (the suite-on run of the block layout's
/// tests and measurements).
pub(crate) const BLOCKS_DEFAULT: bool = cfg!(flowmaid_blocks);

thread_local! {
    /// A scoped override of [`BLOCKS_DEFAULT`] ([`with_blocks`]). Per
    /// thread: the tests of one binary run side by side, and a
    /// process-wide value would leak block mode into tests that must
    /// stay off.
    static OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Whether the block layout is switched on for this call. Read once per
/// entry call ([`crate::scene::scene`], [`crate::scene::route`],
/// [`crate::scene::route_partial`], the public layout functions) and
/// passed down from there; nothing deeper reads it.
pub(crate) fn blocks_on() -> bool {
    OVERRIDE.with(|c| c.get()).unwrap_or(BLOCKS_DEFAULT)
}

/// Run `f` with the switch set to `on` on this thread, and put the
/// former value back afterwards, also when `f` unwinds.
pub(crate) fn with_blocks<T>(on: bool, f: impl FnOnce() -> T) -> T {
    struct Reset(Option<bool>);
    impl Drop for Reset {
        fn drop(&mut self) {
            OVERRIDE.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(OVERRIDE.with(|c| c.replace(Some(on))));
    f()
}

/// What a clustered diagram's edges to whole subgraphs stand for in the
/// layout: built once per entry call, by the one constructor
/// ([`Augmented::of`]).
pub(crate) struct Augmented {
    /// Each node's subgraph path, outermost first
    /// ([`crate::scene::node_cluster_paths`]).
    pub(crate) node_cluster: Vec<Vec<usize>>,
    /// The rank-only pairs packing loose members into a grid
    /// ([`crate::scene::grid_pairs`]).
    pub(crate) grid: Vec<RankGroup>,
    /// Per sub-edge, the rank-only group standing for it
    /// ([`crate::scene::sub_edge_proxies`]).
    pub(crate) proxies: Vec<RankGroup>,
    /// Per sub-edge, the member pair its stand-in edge joins: the first
    /// proxy source and the first proxy target that is another node.
    /// `None` without such a pair (an edge to an empty subgraph, a
    /// subgraph to its own member).
    pub(crate) pairs: Vec<Option<(usize, usize)>>,
    /// Per sub-edge: a degenerate own-box edge — its two ends are one
    /// subgraph (`S --> S`), or a subgraph and one nested in it
    /// (`OUT --> IN`, `IN --> OUT`). Such an edge is drawn on the frame
    /// and is no link: its stand-in and its group never decide block
    /// mode, and a block layout leaves them out of ranking and
    /// ordering (the rows of a diagram holding one are the rows of the
    /// same diagram without it).
    pub(crate) flagged: Vec<bool>,
}

impl Augmented {
    /// The stand-ins and rank-only groups of `g`.
    pub(crate) fn of(g: &Graph) -> Augmented {
        let node_cluster = crate::scene::node_cluster_paths(g);
        let grid = crate::scene::grid_pairs(g, &node_cluster);
        let proxies = crate::scene::sub_edge_proxies(g, &node_cluster, &grid);
        let pairs = proxies
            .iter()
            .map(|(from, to)| {
                let &a = from.first()?;
                let &b = to.iter().find(|&&b| b != a)?;
                Some((a, b))
            })
            .collect();
        // Whether subgraph `outer` is `inner` or holds it at some depth
        // (bounded by the subgraph count, so a malformed parent chain
        // cannot loop).
        let holds = |outer: usize, inner: usize| {
            let mut cur = Some(inner);
            for _ in 0..=g.subgraphs.len() {
                match cur {
                    Some(s) if s == outer => return true,
                    Some(s) => cur = g.subgraphs.get(s).and_then(|x| x.parent),
                    None => return false,
                }
            }
            false
        };
        let flagged = g
            .sub_edges
            .iter()
            .map(|e| match (e.from, e.to) {
                (End::Sub(s), End::Sub(t)) => holds(s, t) || holds(t, s),
                _ => false,
            })
            .collect();
        Augmented {
            node_cluster,
            grid,
            proxies,
            pairs,
            flagged,
        }
    }

    /// The rank-only groups of the layout: the sub-edges' proxies, then
    /// the grid pairs.
    pub(crate) fn rank_only(&self) -> Vec<RankGroup> {
        let mut out = self.proxies.clone();
        out.extend_from_slice(&self.grid);
        out
    }

    /// `g` with one stand-in edge appended per sub-edge that has a pair
    /// (the augmented graph), and per sub-edge the index of its stand-in
    /// there.
    pub(crate) fn graph(&self, g: &Graph) -> (Graph, Vec<Option<usize>>) {
        let mut ga = g.clone();
        let mut stand_in: Vec<Option<usize>> = vec![None; g.sub_edges.len()];
        for (i, (e, pair)) in g.sub_edges.iter().zip(&self.pairs).enumerate() {
            let Some((a, b)) = *pair else { continue };
            stand_in[i] = Some(ga.edges.len());
            ga.edges.push(Edge {
                from: a,
                to: b,
                label: e.label.clone(),
                kind: e.kind,
                ends: e.ends,
                meta: e.meta.clone(),
                end_room: (0.0, 0.0),
            });
        }
        (ga, stand_in)
    }

    /// What the layout needs to lay `g` out as blocks beside its
    /// augmented graph (whose edge count is `edges`) and its rank-only
    /// groups ([`Augmented::rank_only`]): which stand-ins (`stand_in`,
    /// from [`Augmented::graph`]) and which groups are flagged.
    pub(crate) fn block_links(&self, stand_in: &[Option<usize>], edges: usize) -> BlockLinks {
        let mut flagged_edges = vec![false; edges];
        for (si, &flag) in stand_in.iter().zip(&self.flagged) {
            if let (Some(ei), true) = (*si, flag) {
                flagged_edges[ei] = true;
            }
        }
        BlockLinks {
            flagged_edges,
            // The proxies come first in `rank_only`, one per sub-edge.
            flagged_groups: self.flagged.clone(),
        }
    }

    /// Whether `g` is laid out in block mode once the switch is on: the
    /// augmented graph has a **border link** — a real edge (visible or
    /// invisible) or an unflagged stand-in whose two end nodes have
    /// different subgraph paths, so that it crosses, or ends on, at
    /// least one frame. A self-loop is no link. A diagram whose
    /// subgraphs are all isolated has nothing for the block rules to do
    /// and is drawn as before.
    pub(crate) fn block_mode(&self, g: &Graph) -> bool {
        let border = |a: usize, b: usize| a != b && self.node_cluster[a] != self.node_cluster[b];
        g.edges.iter().any(|e| border(e.from, e.to))
            || self
                .pairs
                .iter()
                .zip(&self.flagged)
                .any(|(pair, &flag)| !flag && pair.is_some_and(|(a, b)| border(a, b)))
    }
}

/// Handed to the layout with a graph it is to lay out as blocks (block
/// mode is on: the switch, and the predicate of
/// [`Augmented::block_mode`]): the degenerate own-box pairs
/// ([`Augmented::flagged`]), which are no links there — left out of
/// ranking and ordering, no channel, no label slot.
#[derive(Clone, Debug, Default)]
pub(crate) struct BlockLinks {
    /// Per edge of the augmented graph (missing = no): a flagged
    /// stand-in.
    flagged_edges: Vec<bool>,
    /// Per rank-only group (missing = no): a flagged sub-edge's group.
    flagged_groups: Vec<bool>,
}

impl BlockLinks {
    /// Whether edge `ei` of the augmented graph is a flagged stand-in.
    pub(crate) fn flagged_edge(&self, ei: usize) -> bool {
        self.flagged_edges.get(ei).copied().unwrap_or(false)
    }

    /// Whether rank-only group `k` stands for a flagged sub-edge.
    pub(crate) fn flagged_group(&self, k: usize) -> bool {
        self.flagged_groups.get(k).copied().unwrap_or(false)
    }
}

/// Block mode for a graph that comes without stand-ins (the public
/// [`crate::layout::layout_clustered`]): a real edge whose two ends have
/// different subgraph paths in `node_cluster`.
pub(crate) fn border_edge(g: &Graph, node_cluster: &[Vec<usize>]) -> bool {
    g.edges
        .iter()
        .any(|e| e.from != e.to && node_cluster.get(e.from) != node_cluster.get(e.to))
}

/// One unit of a level: a node whose innermost subgraph is that level,
/// or a non-empty subgraph whose parent is (a *block*).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum Unit {
    Node(usize),
    /// By subgraph index.
    Block(usize),
}

/// The root, or a non-empty subgraph.
#[derive(Clone, Debug)]
pub(crate) struct Level {
    /// The subgraph this level is; `None` for the root.
    pub(crate) sub: Option<usize>,
    /// The level round it ([`Tree::levels`] index); `None` for the root.
    pub(crate) parent: Option<usize>,
    /// Nesting depth: 0 for the root.
    pub(crate) depth: usize,
    /// Its units in declaration order ([`Tree::key`]).
    pub(crate) units: Vec<Unit>,
}

/// The level tree of a clustered graph, from its nodes' subgraph paths
/// alone (so the public `layout_clustered`, which gets nothing else,
/// builds the same tree).
///
/// An empty subgraph (no node at any depth) is no level and no unit: it
/// is on no node's path.
#[derive(Clone, Debug)]
pub(crate) struct Tree {
    /// The root first, then the non-empty subgraphs by index.
    pub(crate) levels: Vec<Level>,
    /// Per subgraph, its level; `None` for an empty one.
    pub(crate) level_of_sub: Vec<Option<usize>>,
    /// Per subgraph, the least node index inside it at any depth
    /// (`usize::MAX` for an empty one): a block's declaration-order key.
    key_of_sub: Vec<usize>,
    /// Each node's subgraph path, outermost first.
    paths: Vec<Vec<usize>>,
}

impl Tree {
    /// The tree of the nodes whose subgraph paths (outermost first) are
    /// `node_cluster`, over `subgraphs` subgraphs.
    pub(crate) fn new(node_cluster: &[Vec<usize>], subgraphs: usize) -> Tree {
        let mut key_of_sub = vec![usize::MAX; subgraphs];
        // A subgraph's parent, read off the paths it is on.
        let mut parent_sub: Vec<Option<usize>> = vec![None; subgraphs];
        for (v, path) in node_cluster.iter().enumerate() {
            for (k, &s) in path.iter().enumerate() {
                key_of_sub[s] = key_of_sub[s].min(v);
                parent_sub[s] = k.checked_sub(1).map(|p| path[p]);
            }
        }
        let mut levels = vec![Level {
            sub: None,
            parent: None,
            depth: 0,
            units: Vec::new(),
        }];
        let mut level_of_sub = vec![None; subgraphs];
        for s in (0..subgraphs).filter(|&s| key_of_sub[s] != usize::MAX) {
            level_of_sub[s] = Some(levels.len());
            levels.push(Level {
                sub: Some(s),
                parent: None,
                depth: 0,
                units: Vec::new(),
            });
        }
        let level_of = |s: Option<usize>| s.map_or(0, |s| level_of_sub[s].unwrap_or(0));
        for s in 0..subgraphs {
            let Some(l) = level_of_sub[s] else { continue };
            let up = level_of(parent_sub[s]);
            levels[l].parent = Some(up);
            levels[up].units.push(Unit::Block(s));
        }
        for (v, path) in node_cluster.iter().enumerate() {
            levels[level_of(path.last().copied())]
                .units
                .push(Unit::Node(v));
            for (k, &s) in path.iter().enumerate() {
                if let Some(l) = level_of_sub[s] {
                    levels[l].depth = k + 1;
                }
            }
        }
        let mut tree = Tree {
            levels,
            level_of_sub,
            key_of_sub,
            paths: node_cluster.to_vec(),
        };
        let mut units: Vec<Vec<Unit>> = tree.levels.iter().map(|l| l.units.clone()).collect();
        for u in units.iter_mut() {
            u.sort_by_key(|&x| tree.key(x));
        }
        for (l, u) in tree.levels.iter_mut().zip(units) {
            l.units = u;
        }
        tree
    }

    /// A unit's place in declaration order: a node's index (the order of
    /// first mention), a block's least node index. Distinct within a
    /// level: a block's first node is no unit of the level itself.
    pub(crate) fn key(&self, u: Unit) -> usize {
        match u {
            Unit::Node(v) => v,
            Unit::Block(s) => self.key_of_sub[s],
        }
    }

    /// Node `v`'s chain of blocks, outermost first.
    pub(crate) fn chain(&self, v: usize) -> &[usize] {
        &self.paths[v]
    }

    /// The level the link `u -> v` belongs to — that of the lowest
    /// common ancestor of its ends — and the two units it joins there
    /// (equal for two nodes of one block's inside seen from above; a
    /// caller that wants links between different units checks).
    pub(crate) fn join(&self, u: usize, v: usize) -> (usize, Unit, Unit) {
        let (pu, pv) = (&self.paths[u], &self.paths[v]);
        let k = pu.iter().zip(pv).take_while(|(a, b)| a == b).count();
        let level = k
            .checked_sub(1)
            .and_then(|i| self.level_of_sub[pu[i]])
            .unwrap_or(0);
        let unit = |path: &[usize], node: usize| match path.get(k) {
            Some(&s) => Unit::Block(s),
            None => Unit::Node(node),
        };
        (level, unit(pu, u), unit(pv, v))
    }

    /// Whether the link `u -> v` is a border link: its end nodes have
    /// different subgraph paths, so one of its two units is a block.
    pub(crate) fn border(&self, u: usize, v: usize) -> bool {
        u != v && self.paths[u] != self.paths[v]
    }
}

/// Which entry point decided block mode, for the record a test reads.
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Entry {
    Scene,
    Route,
    Partial,
}

#[cfg(test)]
thread_local! {
    /// What each entry point decided, in call order: (entry, nodes and
    /// edges of the graph it was given, block mode).
    static USED: std::cell::RefCell<Vec<(Entry, usize, usize, bool)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Record the block mode an entry point used for `g` (tests only: the
/// three entry points must agree, and calling one predicate three times
/// could not show that they do).
#[cfg(test)]
pub(crate) fn note_used(entry: Entry, g: &Graph, on: bool) {
    USED.with(|u| {
        u.borrow_mut()
            .push((entry, g.nodes.len(), g.edges.len(), on))
    });
}

/// The record since the last call, emptied.
#[cfg(test)]
pub(crate) fn take_used() -> Vec<(Entry, usize, usize, bool)> {
    USED.with(|u| std::mem::take(&mut *u.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::scene::{route, route_partial, scene, to_svg};

    fn graph(src: &str) -> Graph {
        match crate::parser::parse_document(src).expect("parses") {
            Document::Flowchart(g) | Document::State(g) => g,
            _ => panic!("not a flowchart or state diagram"),
        }
    }

    fn fixture(name: &str) -> String {
        let root = env!("CARGO_MANIFEST_DIR");
        let path = if name.starts_with("examples/") {
            format!("{root}/{name}.mmd")
        } else {
            format!("{root}/tests/fixtures/{name}.mmd")
        };
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    /// Every flowchart and state fixture and example, by its path
    /// without the extension (under `tests/fixtures`, or `examples/…`).
    fn flow_fixtures() -> Vec<(String, Graph)> {
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
                    let stem = f.file_stem().unwrap().to_string_lossy();
                    out.push((format!("{prefix}{stem}"), g));
                }
            }
        }
        out
    }

    /// Whether `scene()` lays any part of `g` out in block mode with
    /// the switch on (a diagram with blocks laid out in their own
    /// direction is decided part by part).
    fn in_block_mode(g: &Graph) -> bool {
        with_blocks(true, || {
            take_used();
            scene(g);
            take_used().iter().any(|u| u.3)
        })
    }

    #[test]
    fn the_switch_is_off_unless_asked_for_and_scoped() {
        assert_eq!(blocks_on(), BLOCKS_DEFAULT);
        assert!(with_blocks(true, blocks_on));
        assert!(!with_blocks(false, blocks_on));
        assert!(with_blocks(true, || with_blocks(false, || !blocks_on()) && blocks_on()));
        assert_eq!(blocks_on(), BLOCKS_DEFAULT);
        // Put back when the closure unwinds, too.
        let r = std::panic::catch_unwind(|| with_blocks(!BLOCKS_DEFAULT, || panic!("unwind")));
        assert!(r.is_err());
        assert_eq!(blocks_on(), BLOCKS_DEFAULT);
    }

    #[test]
    fn the_tree_has_a_level_per_non_empty_subgraph() {
        // OUT holds o1, o2 and IN (i1, i2); SIDE holds s1; EMPTY and the
        // subgraph round it hold nothing; x and y are free.
        let g = graph(
            "flowchart TD\nx --> o1\nsubgraph OUT\no1 --> i1\nsubgraph IN\ni1 --> i2\nend\n\
             i2 --> o2\nend\nsubgraph SIDE\ns1\nend\nsubgraph HOLLOW\nsubgraph EMPTY\nend\nend\n\
             o2 --> y\ni2 --> s1\nx --> EMPTY\n",
        );
        let nc = crate::scene::node_cluster_paths(&g);
        let t = Tree::new(&nc, g.subgraphs.len());
        let sub = |id: &str| g.subgraphs.iter().position(|s| s.id == id).unwrap();
        let node = |id: &str| g.nodes.iter().position(|n| n.id == id).unwrap();
        let (out, inn, side) = (sub("OUT"), sub("IN"), sub("SIDE"));
        // Root, OUT, IN, SIDE: the two empty subgraphs are no levels.
        assert_eq!(t.levels.len(), 4);
        assert_eq!(t.level_of_sub[sub("EMPTY")], None);
        assert_eq!(t.level_of_sub[sub("HOLLOW")], None);
        let level = |s: usize| t.level_of_sub[s].unwrap();
        assert_eq!(t.levels[level(inn)].parent, Some(level(out)));
        assert_eq!(t.levels[level(out)].parent, Some(0));
        assert_eq!(
            (
                t.levels[0].depth,
                t.levels[level(out)].depth,
                t.levels[level(inn)].depth
            ),
            (0, 1, 2)
        );
        // Units in declaration order: x was mentioned first, then OUT's
        // o1; y and SIDE's s1 come later.
        assert_eq!(
            t.levels[0].units,
            vec![
                Unit::Node(node("x")),
                Unit::Block(out),
                Unit::Block(side),
                Unit::Node(node("y")),
            ]
        );
        assert_eq!(
            t.levels[level(out)].units,
            vec![
                Unit::Node(node("o1")),
                Unit::Block(inn),
                Unit::Node(node("o2"))
            ]
        );
        // A block's key is its first node.
        assert_eq!(t.key(Unit::Block(out)), node("o1"));
        assert_eq!(t.key(Unit::Block(inn)), node("i1"));
        // Links: x -> o1 belongs to the root and joins x with OUT; o1 ->
        // i1 belongs to OUT and joins o1 with IN; i1 -> i2 lies inside
        // IN; i2 -> s1 belongs to the root and joins OUT with SIDE.
        assert_eq!(
            t.join(node("x"), node("o1")),
            (0, Unit::Node(node("x")), Unit::Block(out))
        );
        assert_eq!(
            t.join(node("o1"), node("i1")),
            (level(out), Unit::Node(node("o1")), Unit::Block(inn))
        );
        assert_eq!(
            t.join(node("i1"), node("i2")),
            (level(inn), Unit::Node(node("i1")), Unit::Node(node("i2")))
        );
        assert_eq!(
            t.join(node("i2"), node("s1")),
            (0, Unit::Block(out), Unit::Block(side))
        );
        assert!(t.border(node("x"), node("o1")) && t.border(node("i2"), node("s1")));
        assert!(!t.border(node("i1"), node("i2")) && !t.border(node("x"), node("y")));
        assert_eq!(t.chain(node("i1")), &[out, inn]);
    }

    #[test]
    fn stand_ins_join_a_member_pair_and_own_box_edges_are_flagged() {
        // T --> S: a stand-in from T to S's entry member a. S --> b (its
        // own member): no stand-in. c --> S (own box): a stand-in to the
        // entry member. S --> S: flagged. T --> EMPTY: no stand-in.
        let g = graph(
            "flowchart TD\nsubgraph S[Service]\na --> b --> c\nend\nT[top] --> S\nS --> b\n\
             c --> S\nS --> S\nsubgraph EMPTY[Nothing here]\nend\nT --> EMPTY\n",
        );
        let aug = Augmented::of(&g);
        let node = |id: &str| g.nodes.iter().position(|n| n.id == id).unwrap();
        let of = |from: &str, to: &str| {
            let id = |e: End| match e {
                End::Node(v) => g.nodes[v].id.clone(),
                End::Sub(s) => g.subgraphs[s].id.clone(),
            };
            g.sub_edges
                .iter()
                .position(|e| id(e.from) == from && id(e.to) == to)
                .unwrap()
        };
        assert_eq!(aug.pairs[of("T", "S")], Some((node("T"), node("a"))));
        assert_eq!(aug.pairs[of("S", "b")], None);
        assert_eq!(aug.pairs[of("c", "S")], Some((node("c"), node("a"))));
        assert_eq!(aug.pairs[of("T", "EMPTY")], None);
        let flags: Vec<bool> = ["T>S", "S>b", "c>S", "S>S", "T>EMPTY"]
            .iter()
            .map(|p| {
                let (a, b) = p.split_once('>').unwrap();
                aug.flagged[of(a, b)]
            })
            .collect();
        assert_eq!(flags, [false, false, false, true, false]);
        // The augmented graph: the real edges, then one edge per pair.
        let (ga, stand_in) = aug.graph(&g);
        assert_eq!(ga.edges.len(), g.edges.len() + 3);
        let si = stand_in[of("T", "S")].unwrap();
        assert_eq!((ga.edges[si].from, ga.edges[si].to), (node("T"), node("a")));
        assert_eq!(stand_in[of("S", "b")], None);
        // T --> S is a border link.
        assert!(aug.block_mode(&g));
    }

    /// B12.1: block mode is on exactly when a link joins a subgraph, or
    /// something inside it, to something outside it.
    #[test]
    fn block_mode_is_decided_by_border_links() {
        let off = [
            "examples/state",
            "blocks/p01-iso",
            "blocks/p02-iso-dir",
            "blocks/p09-disconnected",
            "blocks/p13-inner-dir-ext",
            "blocks/iso-lane",
            "blocks/iso-lane-lr",
            "blocks/iso-state",
            "blocks/iso-nest",
            "blocks/empty-target",
            "blocks/t1-nest-own",
            "blocks/nested-self-only",
            "blocks/t2-self-nested",
            "blocks/t2b-self-nested-order",
            "blocks/selfsub-nested",
        ];
        let on = [
            "blocks/blocks-overview",
            "blocks/blocks-overview-lr",
            "quality/bench-75",
            "quality/dense-lr",
            "quality/dense-td",
            "examples/advanced",
            "examples/subgraph",
            "blocks/hub5",
            "blocks/cycle2",
            "blocks/nested",
            "blocks/swimlane",
            "blocks/state-note",
            "blocks/p12-state",
            "notes/near-drag",
            "notes/spread-bottom",
            "notes/straight-link",
            "notes/wide-frames",
        ];
        for name in off {
            assert!(!in_block_mode(&graph(&fixture(name))), "{name}");
        }
        for name in on {
            assert!(in_block_mode(&graph(&fixture(name))), "{name}");
        }
        // No flat diagram is in block mode, whatever its edges.
        for (name, g) in flow_fixtures() {
            if g.subgraphs.is_empty() {
                assert!(!in_block_mode(&g), "{name}");
            }
        }
        // With the switch off nothing is.
        with_blocks(false, || {
            take_used();
            scene(&graph(&fixture("blocks/hub5")));
            assert!(take_used().iter().all(|u| !u.3));
        });
    }

    /// B12.1: the drawings the identity table keeps for good
    /// (`tests/fixtures/identity.tsv`, every row not marked `blocks`)
    /// are exactly the fixtures no entry point lays out in block mode.
    #[test]
    fn the_permanent_identity_rows_are_the_fixtures_outside_block_mode() {
        let table = include_str!("../tests/fixtures/identity.tsv");
        let class_of = |name: &str| {
            let path = if name.starts_with("examples/") {
                format!("{name}.mmd")
            } else {
                format!("tests/fixtures/{name}.mmd")
            };
            table
                .lines()
                .find(|l| l.split('\t').next() == Some(path.as_str()))
                .and_then(|l| l.split('\t').nth(4))
                .unwrap_or_else(|| panic!("{name} has no row"))
                .to_string()
        };
        let mut blocks = 0;
        for (name, g) in flow_fixtures() {
            let on = in_block_mode(&g);
            assert_eq!(on, crate::scene::block_mode(&g), "{name}");
            assert_eq!(on, class_of(&name) == "blocks", "{name}");
            if !on {
                let want = if g.subgraphs.is_empty() {
                    "flat"
                } else {
                    "unlinked"
                };
                assert_eq!(class_of(&name), want, "{name}");
            }
            blocks += usize::from(on);
        }
        assert!(blocks > 50, "{blocks}");
    }

    /// B12.1: a subgraph's edge to itself, or between a subgraph and
    /// one nested in it, joins nothing to the outside, in whichever
    /// order the members were declared.
    #[test]
    fn an_own_box_edge_never_decides_block_mode() {
        for body in [
            "subgraph S\na --> b\nsubgraph N\nn1 --> n2\nend\nend\nS --> S",
            "subgraph S\na\nsubgraph N\nn1 --> n2\nend\na --> b\nend\nS --> S",
            "subgraph OUT\no1\nsubgraph IN\ni1 --> i2\nend\nend\nOUT --> IN",
            "subgraph OUT\nsubgraph IN\ni1 --> i2\nend\no1\nend\nIN --> OUT",
            "subgraph S\np\nsubgraph N\nq\nend\nend\nS --> S",
        ] {
            let g = graph(&format!("flowchart TD\nx --> y\n{body}\n"));
            let aug = Augmented::of(&g);
            assert!(aug.flagged.iter().all(|&f| f), "{body}");
            assert!(!aug.block_mode(&g), "{body}");
            assert!(!in_block_mode(&g), "{body}");
        }
        // The same subgraphs with one real link across a border are.
        let g = graph("flowchart TD\nx --> a\nsubgraph S\na --> b\nend\nS --> S\n");
        assert!(Augmented::of(&g).block_mode(&g));
        // A member's edge to its own box is a border link only when the
        // entry member it is drawn to lies in a nested block.
        let flat = graph("flowchart TD\nsubgraph S\na --> b\nb --> S\nend\n");
        assert!(!Augmented::of(&flat).block_mode(&flat));
        let nested = graph("flowchart TD\nsubgraph S\nsubgraph N\na\nend\nb\nb --> S\nend\n");
        assert!(Augmented::of(&nested).block_mode(&nested));
    }

    /// B12.1: `scene()`, `route()` and `route_partial()` decide block
    /// mode alike for every fixture, part by part.
    #[test]
    fn block_mode_agrees_between_scene_and_route() {
        with_blocks(true, || {
            for (name, g) in flow_fixtures() {
                take_used();
                let base = scene(&g);
                let by_scene = take_used();
                let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
                route(&g, &at);
                let by_route = take_used();
                let mut moved = at.clone();
                if let Some(p) = moved.first_mut() {
                    *p = (p.0 + 7.0, p.1 + 5.0);
                }
                route_partial(&g, &moved, &base, &at);
                let by_partial = take_used();
                let part = |u: &(Entry, usize, usize, bool)| (u.1, u.2, u.3);
                let scene_parts: Vec<_> = by_scene.iter().map(part).collect();
                assert!(by_scene.iter().all(|u| u.0 == Entry::Scene), "{name}");
                assert_eq!(
                    scene_parts,
                    by_route
                        .iter()
                        .filter(|u| u.0 == Entry::Route)
                        .map(part)
                        .collect::<Vec<_>>(),
                    "{name}: route() decides as scene() does"
                );
                // A drag frame re-routes the parts the drag reaches (and
                // may fall back to a whole route()): each decides as
                // scene() decided for that part.
                assert!(!by_partial.is_empty(), "{name}");
                for u in &by_partial {
                    assert!(scene_parts.contains(&part(u)), "{name}: {u:?}");
                }
            }
        });
    }

    /// B12.1 (until the block layout is the default): outside block
    /// mode the switch changes nothing — `scene()` and `route()` draw
    /// the same bytes with it on as with it off. The fixtures with an
    /// own-box edge (a subgraph's edge to itself, to its own member or
    /// to a nested subgraph) are left out: those edges are redrawn for
    /// every diagram with subgraphs.
    #[test]
    fn the_switch_changes_nothing_outside_block_mode() {
        let own_box = |g: &Graph| {
            let nc = crate::scene::node_cluster_paths(g);
            let aug = Augmented::of(g);
            aug.flagged.iter().any(|&f| f)
                || g.sub_edges.iter().any(
                    |e| matches!((e.from, e.to), (End::Sub(s), End::Node(v)) if nc[v].contains(&s)),
                )
        };
        let mut checked = 0;
        for (name, g) in flow_fixtures() {
            if in_block_mode(&g) || own_box(&g) {
                continue;
            }
            let draw = |on: bool| {
                with_blocks(on, || {
                    let s = scene(&g);
                    let at: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
                    (to_svg(&s), to_svg(&route(&g, &at)))
                })
            };
            assert_eq!(draw(true), draw(false), "{name}");
            checked += 1;
        }
        assert!(checked > 20, "{checked}");
    }
}
