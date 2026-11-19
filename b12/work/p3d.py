p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''/// A strongly connected component of a level's units is ordered exactly''','''/// A clustered layout graph's layering as blocks and as it was before
/// blocks, side by side ([`layerings`]).
pub(crate) struct Layerings {
    /// Each node's layer in the block layering.
    pub(crate) block: Vec<usize>,
    /// Each node's layer in the layering before blocks.
    pub(crate) before: Vec<usize>,
    /// The edges that run against the flow in the block layering.
    pub(crate) upstream: Vec<usize>,
    /// The levels the lane rule dissolved (`None`: the top level).
    pub(crate) lanes: Vec<Option<usize>>,
    /// Each node's chain of blocks in the block layering, outermost
    /// first (its subgraph path without the dissolved subgraphs).
    pub(crate) paths: Vec<Vec<usize>>,
}

/// Both layerings of the clustered layout graph `g` ([`rank`] with and
/// without `links`), for the reports and tests that compare them: the
/// inputs whose rows the block layout leaves as they were are judged by
/// "not worse than before".
pub(crate) fn layerings(
    g: &Graph,
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
    links: &BlockLinks,
) -> Layerings {
    let walk = if g.nodes.len() + g.edges.len() <= WALK_MAX_ELEMENTS {
        Walk::Fewest
    } else {
        Walk::Source
    };
    let drawn = rank_only.len();
    let before = rank(g, rank_only, drawn, Some(node_cluster), walk, None);
    let block = rank(g, rank_only, drawn, Some(node_cluster), walk, Some(links));
    let upstream = (0..g.edges.len())
        .filter(|&ei| g.edges[ei].from != g.edges[ei].to && block.back[ei])
        .collect();
    let blocks = block.blocks.unwrap_or_default();
    Layerings {
        block: block.layer,
        before: before.layer,
        upstream,
        lanes: blocks.fired,
        paths: blocks.paths,
    }
}

/// A strongly connected component of a level's units is ordered exactly''')
open(p,'w').write(s)

p='src/scene.rs'; s=open(p).read()
rep('''/// A flat diagram's scene as its builder draws it, before the edge''','''/// How a clustered graph `g` (one part of a diagram: no subgraph of it
/// sets its own direction) is layered as blocks and as before blocks;
/// `None` for a flat graph. For reports and tests.
pub(crate) fn layerings(g: &Graph) -> Option<crate::layout::Layerings> {
    if g.subgraphs.is_empty() {
        return None;
    }
    let aug = crate::levels::Augmented::of(g);
    let rank_only = aug.rank_only();
    let (ga, stand_in) = aug.graph(g);
    let links = aug.block_links(&stand_in, ga.edges.len());
    Some(crate::layout::layerings(
        &ga,
        &aug.node_cluster,
        &rank_only,
        &links,
    ))
}

/// A flat diagram's scene as its builder draws it, before the edge''')
open(p,'w').write(s)

p='src/quality.rs'; s=open(p).read()
rep('''/// Fastest of [`ROUTE_RUNS`] calls of `f` (ms).
///
/// Uses `std::time::Instant`''','''/// How `g` is layered as blocks and how it was layered before blocks,
/// as text. Per part of the diagram that has subgraphs (the whole
/// diagram unless a subgraph sets its own direction): a header line
/// `part <k> block_mode <0|1> rows_equal <0|1>`, the block layers as one
/// line (`layers a b / c / ...`), one line `upstream a -> b` per edge
/// that runs against the flow, one line `lanes <subgraph>` per level
/// the lane rule keeps side by side, and one line per node
/// `node <id> <block layer> <layer before>`.
pub fn layers_dump(g: &Graph) -> Vec<String> {
    let out = std::cell::RefCell::new(Vec::new());
    let part = std::cell::Cell::new(0usize);
    crate::compound::any_part(g, &|p| {
        let k = part.replace(part.get() + 1);
        let Some(l) = scene::layerings(p) else {
            return false;
        };
        let mut out = out.borrow_mut();
        let mode = crate::levels::Augmented::of(p).block_mode(p);
        out.push(format!(
            "part {k} block_mode {} rows_equal {}",
            u8::from(mode),
            u8::from(l.block == l.before)
        ));
        let rows = l.block.iter().copied().max().map_or(0, |m| m + 1);
        let by_row: Vec<String> = (0..rows)
            .map(|r| {
                let ids: Vec<&str> = (0..p.nodes.len())
                    .filter(|&v| l.block[v] == r)
                    .map(|v| p.nodes[v].id.as_str())
                    .collect();
                ids.join(" ")
            })
            .collect();
        out.push(format!("layers {}", by_row.join(" / ")));
        // Stand-ins come after the part's own edges: name those by
        // their end nodes too.
        for &ei in &l.upstream {
            let ends = match p.edges.get(ei) {
                Some(e) => Some((e.from, e.to)),
                None => stand_in_ends(p, ei - p.edges.len()),
            };
            if let Some((a, b)) = ends {
                out.push(format!("upstream {} -> {}", p.nodes[a].id, p.nodes[b].id));
            }
        }
        for lane in &l.lanes {
            out.push(format!(
                "lanes {}",
                lane.map_or("(root)", |s| p.subgraphs[s].id.as_str())
            ));
        }
        for (v, node) in p.nodes.iter().enumerate() {
            out.push(format!("node {} {} {}", node.id, l.block[v], l.before[v]));
        }
        false
    });
    out.into_inner()
}

/// The member pair the `k`-th stand-in of `g`'s augmented graph joins.
fn stand_in_ends(g: &Graph, k: usize) -> Option<(usize, usize)> {
    crate::levels::Augmented::of(g)
        .pairs
        .iter()
        .flatten()
        .nth(k)
        .copied()
}

/// Fastest of [`ROUTE_RUNS`] calls of `f` (ms).
///
/// Uses `std::time::Instant`''')
open(p,'w').write(s)

p='examples/quality.rs'; s=open(p).read()
rep('''//! `--levels` lists each diagram's level tree on stderr.''','''//! `--levels` lists each diagram's level tree on stderr, `--layers` its
//! layers as blocks beside its layers before blocks
//! (`quality::layers_dump`).''')
rep('''[--details] [--drag] [--levels]  (or''','''[--details] [--drag] [--levels] [--layers]  (or''')
rep('''    let mut levels = false;
    while''','''    let mut levels = false;
    let mut layers = false;
    while''')
rep('''        } else if a == "--levels" {
            levels = true;''','''        } else if a == "--levels" {
            levels = true;
        } else if a == "--layers" {
            layers = true;''')
rep('''        if levels {
            for l in quality::levels_dump(&g) {
                eprintln!("{name}\\tlevel\\t{l}");
            }
        }''','''        if levels {
            for l in quality::levels_dump(&g) {
                eprintln!("{name}\\tlevel\\t{l}");
            }
        }
        if layers {
            for l in quality::layers_dump(&g) {
                eprintln!("{name}\\tlayers\\t{l}");
            }
        }''')
open(p,'w').write(s)
