//! Compound layout (report B3, P0 #10): a subgraph that sets its own
//! `direction` is laid out on its own, in that direction, as one block,
//! and the block is placed in the drawing round it like a node.
//!
//! The rule is mermaid's, as its layout applies it to flowcharts and
//! state diagrams alike: a subgraph's `direction` holds only while none
//! of its nodes links to a node outside it. An edge to or from the
//! subgraph itself (`A --> sub`) is fine; an edge across its border to
//! or from a member (`A --> member`, `member --> sub` from inside) makes
//! it take the direction round it. flowmaid lays a subgraph out on its
//! own only when, besides, its direction differs from the one round it
//! and two of its members link to each other: members with no links
//! among them already lie in one row across the flow — the row a
//! direction across the flow asks for — as they always have.
//!
//! A [`Split`] cuts the graph into parts. Each outermost block is one
//! part: its members, the edges among them, the block's subgraph as the
//! part's one top-level frame with every subgraph nested in it, flowing
//! in the block's direction. The rest is the last part, with each block
//! a plain rect node — its proxy — the size of the block's frame; an
//! edge to the block's subgraph is an edge to the proxy there, so it
//! meets the frame like any node's border, labelled, ported and routed
//! like any edge. Each part goes through the whole pipeline on its own
//! (a block's nested blocks recursively), each block is moved into its
//! proxy's box, and the parts are joined into one scene that keeps the
//! [`Scene`] contract: nodes index-parallel with the graph's, its edges
//! first, then its sub-edges in order, the frames outermost-first.
//!
//! [`crate::scene::route`] and [`crate::scene::route_partial`] cut the
//! graph the same way: each block is routed in its own direction for the
//! host's centres and its frame wrapped round its members, then the rest
//! is routed with each proxy on its block's frame — so a drag inside a
//! block re-routes that block and, in the rest, the edges to its frame.

use crate::edgepass::{refit, Mode};
use crate::model::{
    Direction, Edge, End, Graph, Node, NodeStyle, PartNode, Shape, SubEdge, Subgraph,
};
use crate::scene::{Scene, SceneCluster, SceneEdge, SceneNode};

/// Where a part's node comes from.
#[derive(Clone, Copy, Debug)]
enum Slot {
    /// The graph's node.
    Node(usize),
    /// The proxy of block `b` (`Split::blocks[b]`).
    Proxy(usize),
}

/// Where a part's edge comes from.
#[derive(Clone, Copy, Debug)]
enum EdgeOf {
    /// The graph's edge.
    Edge(usize),
    /// The graph's sub-edge between a block's subgraph and a node or
    /// another block's subgraph: an edge to (from) the proxy here.
    Sub(usize),
}

/// One piece of a [`Split`], laid out (routed) on its own.
struct Part {
    g: Graph,
    /// Per node of `g`.
    nodes: Vec<Slot>,
    /// Per edge of `g`.
    edges: Vec<EdgeOf>,
    /// Per sub-edge of `g`: the graph's sub-edge.
    subs: Vec<usize>,
    /// Per subgraph of `g`: the graph's subgraph.
    subgraphs: Vec<usize>,
    /// Nesting depth in the graph of the part's top-level subgraphs (a
    /// block's own subgraph; 0 for the rest).
    lift: usize,
}

/// A graph cut into its outermost blocks and the rest (module docs).
pub(crate) struct Split {
    /// One part per outermost block, with the block's subgraph, in
    /// subgraph order.
    blocks: Vec<(usize, Part)>,
    /// Everything outside them, each block a proxy node.
    rest: Part,
}

/// Each subgraph's chain of subgraphs round it, outermost first and
/// itself last (bounded by the subgraph count, so a malformed parent
/// chain cannot loop), and each node's innermost subgraph.
fn nesting(g: &Graph) -> (Vec<Vec<usize>>, Vec<Option<usize>>) {
    let ns = g.subgraphs.len();
    let chains = (0..ns)
        .map(|s| {
            let mut c = Vec::new();
            let mut cur = Some(s);
            for _ in 0..ns {
                let Some(x) = cur.filter(|&x| x < ns) else {
                    break;
                };
                c.push(x);
                cur = g.subgraphs[x].parent;
            }
            c.reverse();
            c
        })
        .collect();
    let mut owner = vec![None; g.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &v in &s.nodes {
            if let Some(o) = owner.get_mut(v) {
                *o = Some(si);
            }
        }
    }
    (chains, owner)
}

/// The subgraphs an edge end lies inside, outermost first: every one
/// round a node; those strictly round a subgraph (an edge to the
/// subgraph itself does not cross into it).
fn inside<'a>(chains: &'a [Vec<usize>], owner: &[Option<usize>], end: End) -> &'a [usize] {
    match end {
        End::Node(v) => match owner.get(v).copied().flatten() {
            Some(s) => &chains[s],
            None => &[],
        },
        End::Sub(s) => chains
            .get(s)
            .map_or(&[], |c| &c[..c.len().saturating_sub(1)]),
    }
}

/// Per subgraph: whether it is laid out as a block of its own — it
/// sets a direction other than the one round it, two of its members
/// link to each other, and none links out of it (module docs).
pub(crate) fn blocks(g: &Graph) -> Vec<bool> {
    if !g.subgraphs.iter().any(|s| s.direction.is_some()) {
        return vec![false; g.subgraphs.len()];
    }
    let (chains, owner) = nesting(g);
    judge(g, &chains, &owner).0
}

/// [`blocks`], and the direction each subgraph's members flow in: a
/// block's own, any other subgraph's the one round it.
fn judge(g: &Graph, chains: &[Vec<usize>], owner: &[Option<usize>]) -> (Vec<bool>, Vec<Direction>) {
    let ns = g.subgraphs.len();
    // Per subgraph: an edge with both ends inside it; one with exactly
    // one end inside it (crossing its border).
    let mut internal = vec![false; ns];
    let mut external = vec![false; ns];
    let mut link = |a: End, b: End| {
        let (a, b) = (inside(chains, owner, a), inside(chains, owner, b));
        let k = a.iter().zip(b).take_while(|(x, y)| x == y).count();
        for &c in &a[..k] {
            internal[c] = true;
        }
        for &c in a[k..].iter().chain(&b[k..]) {
            external[c] = true;
        }
    };
    for e in &g.edges {
        if e.from != e.to {
            link(End::Node(e.from), End::Node(e.to));
        }
    }
    for e in &g.sub_edges {
        if e.from != e.to {
            link(e.from, e.to);
        }
    }
    // Outermost first: a subgraph's direction is judged against the one
    // its parent ends up with.
    let mut order: Vec<usize> = (0..ns).collect();
    order.sort_by_key(|&s| chains[s].len());
    let mut out = vec![false; ns];
    let mut flow = vec![g.direction; ns];
    for s in order {
        let round = match g.subgraphs[s].parent {
            Some(p) if p < ns && chains[p].len() < chains[s].len() => flow[p],
            _ => g.direction,
        };
        let own = g.subgraphs[s].direction.filter(|&d| d != round);
        out[s] = own.is_some() && internal[s] && !external[s];
        flow[s] = match own {
            Some(d) if out[s] => d,
            _ => round,
        };
    }
    (out, flow)
}

/// The direction each edge of a graph is laid out in: the graph's, or
/// inside a block the block's own ([`blocks`]) — what the quality
/// harness reads an edge's flow against.
pub(crate) struct Flows {
    chains: Vec<Vec<usize>>,
    owner: Vec<Option<usize>>,
    flow: Vec<Direction>,
    direction: Direction,
}

impl Flows {
    pub(crate) fn new(g: &Graph) -> Flows {
        let (chains, owner) = nesting(g);
        let flow = judge(g, &chains, &owner).1;
        Flows {
            chains,
            owner,
            flow,
            direction: g.direction,
        }
    }

    /// The direction an edge between `a` and `b` flows in: that of the
    /// innermost subgraph holding both ends (an edge to a subgraph lies
    /// outside it, beside its frame).
    pub(crate) fn of(&self, a: End, b: End) -> Direction {
        let (a, b) = (
            inside(&self.chains, &self.owner, a),
            inside(&self.chains, &self.owner, b),
        );
        a.iter()
            .zip(b)
            .take_while(|(x, y)| x == y)
            .last()
            .map_or(self.direction, |(&c, _)| self.flow[c])
    }
}

/// A block's proxy id: its subgraph's id, primed until no node has it.
fn proxy_id(g: &Graph, si: usize) -> String {
    let mut id = g.subgraphs[si].id.clone();
    while g.node_index(&id).is_some() {
        id.push('\'');
    }
    id
}

/// Cut `g` into its outermost blocks and the rest, or `None` when no
/// subgraph is a block ([`blocks`]) — or the graph does not nest
/// cleanly enough to cut (an edge between a block and the rest, which
/// [`blocks`] rules out, found anyway).
pub(crate) fn split(g: &Graph) -> Option<Split> {
    let flags = blocks(g);
    if !flags.contains(&true) {
        return None;
    }
    let ns = g.subgraphs.len();
    let (chains, owner) = nesting(g);
    // The outermost blocks, in subgraph order, and the one round each
    // subgraph (itself included) and node.
    let tops: Vec<usize> = (0..ns)
        .filter(|&s| chains[s].iter().find(|&&c| flags[c]) == Some(&s))
        .collect();
    let sub_block: Vec<Option<usize>> = (0..ns)
        .map(|s| {
            let t = chains[s].iter().copied().find(|&c| flags[c])?;
            tops.iter().position(|&x| x == t)
        })
        .collect();
    let node_block: Vec<Option<usize>> =
        owner.iter().map(|o| o.and_then(|s| sub_block[s])).collect();
    // A sub-edge end's place: inside block `b`, on block `b`'s own frame,
    // or in the rest.
    #[derive(Clone, Copy, PartialEq)]
    enum Place {
        In(usize),
        Frame(usize),
        Rest,
    }
    let place = |end: End| match end {
        End::Node(v) => node_block
            .get(v)
            .copied()
            .flatten()
            .map_or(Place::Rest, Place::In),
        End::Sub(s) => match sub_block.get(s).copied().flatten() {
            Some(b) if tops[b] == s => Place::Frame(b),
            Some(b) => Place::In(b),
            None => Place::Rest,
        },
    };

    // The blocks.
    let mut blocks = Vec::with_capacity(tops.len());
    for (b, &top) in tops.iter().enumerate() {
        let members: Vec<usize> = (0..g.nodes.len())
            .filter(|&v| node_block[v] == Some(b))
            .collect();
        let slots: Vec<PartNode> = members.iter().map(|&v| PartNode::Of(v)).collect();
        let dir = g.subgraphs[top].direction.unwrap_or(g.direction);
        let mut pg = g.part(dir, &slots);
        let mut at = vec![None; g.nodes.len()];
        for (k, &v) in members.iter().enumerate() {
            at[v] = Some(k);
        }
        let subs_in: Vec<usize> = (0..ns).filter(|&s| sub_block[s] == Some(b)).collect();
        let mut sat = vec![None; ns];
        for (k, &s) in subs_in.iter().enumerate() {
            sat[s] = Some(k);
        }
        for &s in &subs_in {
            let sg = &g.subgraphs[s];
            pg.subgraphs.push(Subgraph {
                id: sg.id.clone(),
                title: sg.title.clone(),
                nodes: sg.nodes.iter().filter_map(|&v| at[v]).collect(),
                parent: if s == top {
                    None
                } else {
                    sg.parent.and_then(|p| sat[p])
                },
                direction: sg.direction,
            });
        }
        let mut edges = Vec::new();
        for (i, e) in g.edges.iter().enumerate() {
            match (at[e.from], at[e.to]) {
                (Some(a), Some(c)) => {
                    edges.push(EdgeOf::Edge(i));
                    pg.edges.push(Edge {
                        from: a,
                        to: c,
                        ..e.clone()
                    });
                }
                (None, None) => {}
                _ => return None,
            }
        }
        let mut subs = Vec::new();
        for (j, e) in g.sub_edges.iter().enumerate() {
            let (pa, pb) = (place(e.from), place(e.to));
            if pa != Place::In(b) && pb != Place::In(b) {
                continue;
            }
            if pa != pb {
                return None;
            }
            let end = |end: End| match end {
                End::Node(v) => at[v].map(End::Node),
                End::Sub(s) => sat[s].map(End::Sub),
            };
            subs.push(j);
            pg.sub_edges.push(SubEdge {
                from: end(e.from)?,
                to: end(e.to)?,
                ..e.clone()
            });
        }
        pg.label_end_room = room(g, &edges);
        blocks.push((
            top,
            Part {
                g: pg,
                nodes: members.into_iter().map(Slot::Node).collect(),
                edges,
                subs,
                subgraphs: subs_in,
                lift: depth(g, top),
            },
        ));
    }

    // The rest: every other node, each block's proxy where its first
    // member was.
    let mut slots: Vec<Slot> = Vec::new();
    let mut placed = vec![false; tops.len()];
    for v in 0..g.nodes.len() {
        match node_block[v] {
            Some(b) if !placed[b] => {
                placed[b] = true;
                slots.push(Slot::Proxy(b));
            }
            Some(_) => {}
            None => slots.push(Slot::Node(v)),
        }
    }
    let parts: Vec<PartNode> = slots
        .iter()
        .map(|s| match *s {
            Slot::Node(v) => PartNode::Of(v),
            Slot::Proxy(b) => PartNode::New(Node {
                id: proxy_id(g, tops[b]),
                label: String::new(),
                shape: Shape::Rect,
                style: NodeStyle::default(),
            }),
        })
        .collect();
    let mut pg = g.part(g.direction, &parts);
    let mut at = vec![None; g.nodes.len()];
    let mut proxy_at = vec![0; tops.len()];
    for (k, s) in slots.iter().enumerate() {
        match *s {
            Slot::Node(v) => at[v] = Some(k),
            Slot::Proxy(b) => proxy_at[b] = k,
        }
    }
    let subs_out: Vec<usize> = (0..ns).filter(|&s| sub_block[s].is_none()).collect();
    let mut sat = vec![None; ns];
    for (k, &s) in subs_out.iter().enumerate() {
        sat[s] = Some(k);
    }
    for &s in &subs_out {
        let sg = &g.subgraphs[s];
        let mut nodes: Vec<usize> = sg.nodes.iter().filter_map(|&v| at[v]).collect();
        // A block nested in this subgraph: its proxy is a member.
        nodes.extend(
            tops.iter()
                .enumerate()
                .filter(|&(_, &t)| g.subgraphs[t].parent == Some(s))
                .map(|(b, _)| proxy_at[b]),
        );
        pg.subgraphs.push(Subgraph {
            id: sg.id.clone(),
            title: sg.title.clone(),
            nodes,
            parent: sg.parent.and_then(|p| sat[p]),
            direction: sg.direction,
        });
    }
    let mut edges = Vec::new();
    for (i, e) in g.edges.iter().enumerate() {
        if let (Some(a), Some(c)) = (at[e.from], at[e.to]) {
            edges.push(EdgeOf::Edge(i));
            pg.edges.push(Edge {
                from: a,
                to: c,
                ..e.clone()
            });
        }
    }
    // A sub-edge in the rest: between nodes and blocks' frames an edge
    // between nodes and proxies, else a sub-edge with its block ends on
    // their proxies.
    let mut subs = Vec::new();
    for (j, e) in g.sub_edges.iter().enumerate() {
        let (pa, pb) = (place(e.from), place(e.to));
        if matches!(pa, Place::In(_)) || matches!(pb, Place::In(_)) {
            continue;
        }
        let node = |end: End, p: Place| match (end, p) {
            (_, Place::Frame(b)) => Some(proxy_at[b]),
            (End::Node(v), _) => at[v],
            (End::Sub(_), _) => None,
        };
        match (node(e.from, pa), node(e.to, pb)) {
            (Some(a), Some(c)) => {
                edges.push(EdgeOf::Sub(j));
                pg.edges.push(Edge {
                    from: a,
                    to: c,
                    label: e.label.clone(),
                    kind: e.kind,
                    ends: e.ends,
                    meta: e.meta.clone(),
                    end_room: (0.0, 0.0),
                });
            }
            (a, c) => {
                let end = |end: End, n: Option<usize>| match (end, n) {
                    (_, Some(k)) => Some(End::Node(k)),
                    (End::Sub(s), None) => sat[s].map(End::Sub),
                    (End::Node(_), None) => None,
                };
                subs.push(j);
                pg.sub_edges.push(SubEdge {
                    from: end(e.from, a)?,
                    to: end(e.to, c)?,
                    ..e.clone()
                });
            }
        }
    }
    pg.label_end_room = room(g, &edges);
    Some(Split {
        blocks,
        rest: Part {
            g: pg,
            nodes: slots,
            edges,
            subs,
            subgraphs: subs_out,
            lift: 0,
        },
    })
}

/// A part's `label_end_room` (empty when the graph keeps none).
fn room(g: &Graph, edges: &[EdgeOf]) -> Vec<f64> {
    if g.label_end_room.is_empty() {
        return Vec::new();
    }
    edges
        .iter()
        .map(|of| match *of {
            EdgeOf::Edge(i) => g.label_end_room.get(i).copied().unwrap_or(0.0),
            EdgeOf::Sub(_) => 0.0,
        })
        .collect()
}

impl Split {
    /// Index of block `b`'s proxy among the rest's nodes.
    fn proxy(&self, b: usize) -> Option<usize> {
        self.rest
            .nodes
            .iter()
            .position(|s| matches!(*s, Slot::Proxy(x) if x == b))
    }

    /// The parts, blocks first, each with its scene.
    fn with<'a>(
        &'a self,
        blocks: &'a [Scene],
        rest: &'a Scene,
    ) -> impl Iterator<Item = (&'a Part, &'a Scene)> {
        self.blocks
            .iter()
            .map(|(_, p)| p)
            .zip(blocks)
            .chain(std::iter::once((&self.rest, rest)))
    }
}

/// A part's node inputs taken from the graph's (`of` per graph node),
/// each proxy's from `proxies` (its block's frame; a default before
/// there is one).
fn part_of<T: Clone>(part: &Part, of: &[T], proxies: &[T], none: T) -> Vec<T> {
    part.nodes
        .iter()
        .map(|s| match *s {
            Slot::Node(v) => of[v].clone(),
            Slot::Proxy(b) => proxies.get(b).cloned().unwrap_or_else(|| none.clone()),
        })
        .collect()
}

/// The top-level frame `id` of a block's scene, `(x, y, w, h)`.
fn frame(sc: &Scene, id: &str) -> Option<(f64, f64, f64, f64)> {
    sc.clusters
        .iter()
        .find(|c| c.depth == 0 && c.id == id)
        .map(|c| (c.x, c.y, c.w, c.h))
}

/// Nesting depth of subgraph `si` (0 = top level).
fn depth(g: &Graph, si: usize) -> usize {
    let mut d = 0;
    let mut cur = g.subgraphs.get(si).and_then(|s| s.parent);
    while let Some(p) = cur {
        if d > g.subgraphs.len() {
            break;
        }
        d += 1;
        cur = g.subgraphs.get(p).and_then(|s| s.parent);
    }
    d
}

/// The id an end of the graph's sub-edges is drawn with.
fn end_id(g: &Graph, end: End) -> String {
    match end {
        End::Node(v) => g.nodes[v].id.clone(),
        End::Sub(s) => g.subgraphs[s].id.clone(),
    }
}

/// Per sub-edge of `g`: the index of the scene edge drawing it, `None`
/// when the builder dropped it (an end on a subgraph without a frame) —
/// matched by ids in order, as `edgepass::ctx` matches them.
fn sub_slots(g: &Graph, sc: &Scene) -> Vec<Option<usize>> {
    let framed = |s: usize| {
        g.subgraphs
            .get(s)
            .is_some_and(|sg| sc.clusters.iter().any(|c| c.id == sg.id))
    };
    let id = |end: End| -> Option<&str> {
        match end {
            End::Node(v) => g.nodes.get(v).map(|n| n.id.as_str()),
            End::Sub(s) => framed(s).then(|| g.subgraphs[s].id.as_str()),
        }
    };
    let mut next = g.edges.len();
    g.sub_edges
        .iter()
        .map(|e| {
            let (a, b) = (id(e.from)?, id(e.to)?);
            let se = sc.edges.get(next)?;
            (se.from == a && se.to == b).then(|| {
                next += 1;
                next - 1
            })
        })
        .collect()
}

/// The parts' scenes joined into one for `g` (module docs): each node
/// and edge where the part holding it drew it, ids as `g` names them,
/// the frames outermost-first at their depths in `g`, the canvas the
/// largest part's (the caller refits it). `None` when a part's scene
/// does not line up with the part.
fn join(g: &Graph, split: &Split, blocks: &[Scene], rest: &Scene) -> Option<Scene> {
    let mut nodes: Vec<Option<SceneNode>> = vec![None; g.nodes.len()];
    let mut edges: Vec<Option<SceneEdge>> = vec![None; g.edges.len()];
    let mut subs: Vec<Option<SceneEdge>> = vec![None; g.sub_edges.len()];
    let mut clusters: Vec<(usize, SceneCluster)> = Vec::new();
    let (mut width, mut height) = (0.0f64, 0.0f64);
    let named = |j: usize, mut e: SceneEdge| {
        e.from = end_id(g, g.sub_edges[j].from);
        e.to = end_id(g, g.sub_edges[j].to);
        e
    };
    for (part, sc) in split.with(blocks, rest) {
        width = width.max(sc.width);
        height = height.max(sc.height);
        for (k, s) in part.nodes.iter().enumerate() {
            if let Slot::Node(v) = *s {
                nodes[v] = Some(sc.nodes.get(k)?.clone());
            }
        }
        for (k, of) in part.edges.iter().enumerate() {
            let e = sc.edges.get(k)?.clone();
            match *of {
                EdgeOf::Edge(i) => edges[i] = Some(e),
                EdgeOf::Sub(j) => subs[j] = Some(named(j, e)),
            }
        }
        for (k, slot) in sub_slots(&part.g, sc).into_iter().enumerate() {
            if let Some(i) = slot {
                let j = part.subs[k];
                subs[j] = Some(named(j, sc.edges[i].clone()));
            }
        }
        let mut used = vec![false; part.subgraphs.len()];
        for c in &sc.clusters {
            let k = (0..used.len()).find(|&k| !used[k] && part.g.subgraphs[k].id == c.id)?;
            used[k] = true;
            let si = part.subgraphs[k];
            clusters.push((
                si,
                SceneCluster {
                    depth: depth(g, si),
                    ..c.clone()
                },
            ));
        }
    }
    let nodes = nodes.into_iter().collect::<Option<Vec<_>>>()?;
    let mut edges = edges.into_iter().collect::<Option<Vec<_>>>()?;
    edges.extend(subs.into_iter().flatten());
    clusters.sort_by_key(|(si, c)| (c.depth, *si));
    Some(Scene {
        nodes,
        edges,
        clusters: clusters.into_iter().map(|(_, c)| c).collect(),
        width,
        height,
    })
}

/// [`crate::scene::scene_sized`] for a split graph: each block laid
/// out in its own direction, the rest with each proxy the size of its
/// block's frame, each block moved into its proxy's box. `sizes` are
/// sane already (fork / join bars are fitted per part, across the
/// part's own flow). `None` when the parts do not join (the caller lays
/// the graph out in one pass instead).
pub(crate) fn scene(
    g: &Graph,
    split: &Split,
    sizes: &[(f64, f64)],
    labels: &[String],
) -> Option<Scene> {
    let mut blocks = Vec::with_capacity(split.blocks.len());
    let mut frames = Vec::with_capacity(split.blocks.len());
    // The blocks a fork/join bar of the rest is fitted to: the routed
    // builders fit the bar to the same frames, kept from here (`LAID`).
    let fitted_to = bar_blocks(split);
    for ((si, part), &kept) in split.blocks.iter().zip(&fitted_to) {
        let ps = part_of(part, sizes, &[], (0.0, 0.0));
        let pl = part_of(part, labels, &[], String::new());
        let sc = crate::scene::scene_labelled(&part.g, &ps, &pl);
        let f = frame(&sc, &g.subgraphs[*si].id)?;
        if kept {
            keep_laid(&part.g, ps, pl, (f.2, f.3));
        }
        frames.push(f);
        blocks.push(sc);
    }
    let boxes: Vec<(f64, f64)> = frames.iter().map(|f| (f.2, f.3)).collect();
    let rest = crate::scene::scene_labelled(
        &split.rest.g,
        &part_of(&split.rest, sizes, &boxes, (0.0, 0.0)),
        &part_of(&split.rest, labels, &[], String::new()),
    );
    for (b, (sc, f)) in blocks.iter_mut().zip(&frames).enumerate() {
        let p = rest.nodes.get(split.proxy(b)?)?;
        crate::scene::shift_scene(sc, p.x - p.w / 2.0 - f.0, p.y - p.h / 2.0 - f.1);
    }
    let mut out = join(g, split, &blocks, &rest)?;
    refit(&mut out, Mode::Static, g.edge_paint_width.is_none());
    Some(out)
}

/// The routed counterpart of [`scene`] ([`crate::scene::route_sized`]
/// for a split graph): each block routed for the host's centres, its
/// frame wrapped round its members, then the rest with each proxy on its
/// block's frame. Positions stay the host's.
pub(crate) fn route(
    g: &Graph,
    split: &Split,
    centers: &[(f64, f64)],
    sizes: &[(f64, f64)],
    labels: &[String],
) -> Option<Scene> {
    let mut blocks = Vec::with_capacity(split.blocks.len());
    let mut at = Vec::with_capacity(split.blocks.len());
    let mut boxes = Vec::with_capacity(split.blocks.len());
    for (si, part) in &split.blocks {
        let sc = crate::scene::route_labelled(
            &part.g,
            &part_of(part, centers, &[], (0.0, 0.0)),
            &part_of(part, sizes, &[], (0.0, 0.0)),
            &part_of(part, labels, &[], String::new()),
        );
        let (x, y, w, h) = frame(&sc, &g.subgraphs[*si].id)?;
        at.push((x + w / 2.0, y + h / 2.0));
        boxes.push((w, h));
        blocks.push(sc);
    }
    let rest = crate::scene::route_fitted(
        &split.rest.g,
        &part_of(&split.rest, centers, &at, (0.0, 0.0)),
        &rest_sizes(g, split, sizes, labels, &boxes),
        &part_of(&split.rest, labels, &[], String::new()),
    );
    let mut out = join(g, split, &blocks, &rest)?;
    refit(&mut out, Mode::Routed, g.edge_paint_width.is_none());
    Some(out)
}

/// Per block: whether the length of a fork/join bar of the rest reads
/// its proxy's box ([`crate::scene::bar_branches`]) — the bar branches
/// to the block, or to a composite holding it.
fn bar_blocks(split: &Split) -> Vec<bool> {
    let branches = crate::scene::bar_branches(&split.rest.g);
    (0..split.blocks.len())
        .map(|b| split.proxy(b).is_some_and(|p| branches[p]))
        .collect()
}

/// The rest's node sizes for the routed builders: each proxy the box of
/// its block's frame now (`boxes`), each fork/join bar fitted
/// ([`crate::scene::fit_bars`]) as [`scene`] fits it. A bar that
/// branches to a block — or to a composite holding one — spans the
/// block's frame, and fitted to the frame as it stands it would stretch
/// whenever a drag inside the block pulls its states apart; but a drag
/// never resizes a bar. So every proxy whose box the fit reads
/// ([`bar_blocks`]) counts at its block's frame as [`scene`] draws it
/// ([`laid_box`]). The other blocks, and a graph without such a bar,
/// cost nothing more.
fn rest_sizes(
    g: &Graph,
    split: &Split,
    sizes: &[(f64, f64)],
    labels: &[String],
    boxes: &[(f64, f64)],
) -> Vec<(f64, f64)> {
    let rest = &split.rest;
    let now = part_of(rest, sizes, boxes, (0.0, 0.0));
    let is_bar = |k: usize| rest.g.nodes[k].shape == Shape::ForkBar;
    if !(0..rest.g.nodes.len()).any(is_bar) {
        return now;
    }
    let mut laid = now.clone();
    for (b, fitted_to) in bar_blocks(split).into_iter().enumerate() {
        let (si, part) = &split.blocks[b];
        let Some(p) = split.proxy(b).filter(|_| fitted_to) else {
            continue;
        };
        if let Some(f) = laid_box(g, *si, part, sizes, labels) {
            laid[p] = f;
        }
    }
    let fitted = crate::scene::fit_bars(&rest.g, &laid);
    (0..now.len())
        .map(|k| if is_bar(k) { fitted[k] } else { now[k] })
        .collect()
}

/// A block as the static layout was given it — its part's graph, node
/// sizes and labels — with the frame `(w, h)` that layout drew round it.
type Laid = (Graph, Vec<(f64, f64)>, Vec<String>, (f64, f64));

/// Most blocks [`LAID`] holds before it drops the oldest: more than one
/// diagram's bars branch to (a diagram with more lays the oldest out
/// again on each [`route`]), few enough to compare against on each.
const LAID_MAX: usize = 32;

thread_local! {
    /// The frame [`scene`] draws round each block a fork/join bar is
    /// fitted to, by everything that layout reads: the bar keeps that
    /// length in every [`route`] of the graph ([`rest_sizes`]), and
    /// laying the block out again for each frame of a drag would cost
    /// more than routing it (a 60-state block: 4 ms on top of 3). A
    /// memo of a pure function — the answer is the same either way, so
    /// output stays deterministic — kept by [`scene`] itself, so only a
    /// `route` of a graph (or sizes) no `scene` call on this thread has
    /// laid out since pays for the layout, once.
    static LAID: std::cell::RefCell<Vec<Laid>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Keep the frame `(w, h)` the static layout drew round a block's part
/// `pg` for these node sizes and labels ([`LAID`]).
fn keep_laid(pg: &Graph, sizes: Vec<(f64, f64)>, labels: Vec<String>, frame: (f64, f64)) {
    LAID.with(|m| {
        let mut m = m.borrow_mut();
        if m.iter()
            .any(|l| l.1 == sizes && l.2 == labels && l.0 == *pg)
        {
            return;
        }
        if m.len() >= LAID_MAX {
            m.remove(0);
        }
        m.push((pg.clone(), sizes, labels, frame));
    });
}

/// The box `(w, h)` of the frame [`scene`] draws round block `part`
/// (subgraph `si` of `g`) for these node `sizes` and `labels`: kept
/// from the last time it was laid out ([`LAID`]), else laid out now.
/// `None` when the layout draws no such frame.
fn laid_box(
    g: &Graph,
    si: usize,
    part: &Part,
    sizes: &[(f64, f64)],
    labels: &[String],
) -> Option<(f64, f64)> {
    let ps = part_of(part, sizes, &[], (0.0, 0.0));
    let pl = part_of(part, labels, &[], String::new());
    let kept = LAID.with(|m| {
        m.borrow()
            .iter()
            .find(|l| l.1 == ps && l.2 == pl && l.0 == part.g)
            .map(|l| l.3)
    });
    if kept.is_some() {
        return kept;
    }
    // Not under the borrow: a block's nested blocks come through here.
    let sc = crate::scene::scene_labelled(&part.g, &ps, &pl);
    let f = frame(&sc, &g.subgraphs[si].id)?;
    keep_laid(&part.g, ps, pl, (f.2, f.3));
    Some((f.2, f.3))
}

/// `base` cut to `part`: its nodes (each proxy on `frames[b]`, its
/// block's frame in `base`), edges and frames, ids as the part names
/// them. `subs` holds [`sub_slots`] of the graph in `base`. `None` when
/// `base` misses one of the part's edges.
fn part_base(
    part: &Part,
    base: &Scene,
    subs: &[Option<usize>],
    frames: &[(f64, f64, f64, f64)],
) -> Option<Scene> {
    let pg = &part.g;
    let mut nodes = Vec::with_capacity(part.nodes.len());
    for (k, s) in part.nodes.iter().enumerate() {
        nodes.push(match *s {
            Slot::Node(v) => base.nodes.get(v)?.clone(),
            Slot::Proxy(b) => {
                let (x, y, w, h) = *frames.get(b)?;
                SceneNode {
                    id: pg.nodes[k].id.clone(),
                    x: x + w / 2.0,
                    y: y + h / 2.0,
                    w,
                    h,
                    shape: Shape::Rect,
                    label: String::new(),
                    style: NodeStyle::default(),
                    link: None,
                }
            }
        });
    }
    let named = |mut e: SceneEdge, from: End, to: End| {
        let id = |end: End| match end {
            End::Node(v) => pg.nodes[v].id.clone(),
            End::Sub(s) => pg.subgraphs[s].id.clone(),
        };
        e.from = id(from);
        e.to = id(to);
        e
    };
    let mut edges = Vec::with_capacity(part.edges.len() + part.subs.len());
    for (of, pe) in part.edges.iter().zip(&pg.edges) {
        let i = match *of {
            EdgeOf::Edge(i) => i,
            EdgeOf::Sub(j) => (*subs.get(j)?)?,
        };
        let e = base.edges.get(i)?.clone();
        edges.push(named(e, End::Node(pe.from), End::Node(pe.to)));
    }
    for (&j, pe) in part.subs.iter().zip(&pg.sub_edges) {
        if let Some(i) = subs.get(j).copied().flatten() {
            edges.push(named(base.edges.get(i)?.clone(), pe.from, pe.to));
        }
    }
    // The frames of the part's subgraphs, at their depths in the part.
    let clusters = base
        .clusters
        .iter()
        .filter(|c| pg.subgraphs.iter().any(|s| s.id == c.id))
        .map(|c| SceneCluster {
            depth: c.depth.saturating_sub(part.lift),
            ..c.clone()
        })
        .collect();
    Some(Scene {
        nodes,
        edges,
        clusters,
        width: base.width,
        height: base.height,
    })
}

/// [`crate::scene::route_partial`] for a split graph: `base` is cut
/// into the parts, each block re-routed locally for its centres, then
/// the rest with each proxy on its block's frame now — a proxy whose
/// frame moved or changed size counts as moved, so the rest redraws the
/// edges to it. A fork/join bar of the rest that branches to a block
/// is fitted as [`route`] fits it ([`rest_sizes`]), not to the frame as
/// the drag leaves it, whatever `base` holds. (A part none of whose
/// nodes moved is `base`'s, as a graph none of whose nodes moved is:
/// with it a block keeps the bars `base` drew inside it.) `None` when
/// `base` does not line up with the split (the caller falls back to
/// [`crate::scene::route`]).
pub(crate) fn route_partial(
    g: &Graph,
    split: &Split,
    centers: &[(f64, f64)],
    base: &Scene,
    base_centers: &[(f64, f64)],
) -> Option<Scene> {
    if base.edges.len() < g.edges.len()
        || base.nodes.iter().zip(&g.nodes).any(|(a, b)| a.id != b.id)
    {
        return None;
    }
    let subs = sub_slots(g, base);
    let base_frames: Vec<(f64, f64, f64, f64)> = split
        .blocks
        .iter()
        .map(|(si, _)| {
            let id = &g.subgraphs[*si].id;
            base.clusters
                .iter()
                .find(|c| &c.id == id)
                .map(|c| (c.x, c.y, c.w, c.h))
        })
        .collect::<Option<_>>()?;
    let centre = |f: &(f64, f64, f64, f64)| (f.0 + f.2 / 2.0, f.1 + f.3 / 2.0);
    let mut blocks = Vec::with_capacity(split.blocks.len());
    let mut frames = Vec::with_capacity(split.blocks.len());
    for (si, part) in &split.blocks {
        let pb = part_base(part, base, &subs, &base_frames)?;
        let sc = crate::scene::route_partial_with(
            &part.g,
            &part_of(part, centers, &[], (0.0, 0.0)),
            &pb,
            &part_of(part, base_centers, &[], (0.0, 0.0)),
            &[],
        );
        frames.push(frame(&sc, &g.subgraphs[*si].id)?);
        blocks.push(sc);
    }
    let rb = part_base(&split.rest, base, &subs, &base_frames)?;
    let now: Vec<(f64, f64)> = frames.iter().map(centre).collect();
    let was: Vec<(f64, f64)> = base_frames.iter().map(centre).collect();
    let mut sized: Vec<crate::scene::GivenBox> = frames
        .iter()
        .zip(&base_frames)
        .enumerate()
        .filter_map(|(b, (f, o))| {
            let resized = (f.2 - o.2).abs() > 0.01 || (f.3 - o.3).abs() > 0.01;
            Some((split.proxy(b)?, (f.2, f.3), resized))
        })
        .collect();
    // Where a bar of the rest branches to a block, every bar as `route`
    // fits it. (Elsewhere the rest fits its own, reading no proxy.)
    if bar_blocks(split).contains(&true) {
        let (labels, sizes) = crate::layout::node_texts(g);
        let boxes: Vec<(f64, f64)> = frames.iter().map(|f| (f.2, f.3)).collect();
        let fitted = rest_sizes(g, split, &sizes, &labels, &boxes);
        for (k, (n, drawn)) in split.rest.g.nodes.iter().zip(&rb.nodes).enumerate() {
            if n.shape == Shape::ForkBar {
                let (w, h) = fitted[k];
                let resized = (w - drawn.w).abs() > 0.01 || (h - drawn.h).abs() > 0.01;
                sized.push((k, (w, h), resized));
            }
        }
    }
    let rest = crate::scene::route_partial_with(
        &split.rest.g,
        &part_of(&split.rest, centers, &now, (0.0, 0.0)),
        &rb,
        &part_of(&split.rest, base_centers, &was, (0.0, 0.0)),
        &sized,
    );
    let mut out = join(g, split, &blocks, &rest)?;
    refit(&mut out, Mode::Routed, g.edge_paint_width.is_none());
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::scene::{edge_polyline, route, route_partial, scene, to_svg};

    fn state(src: &str) -> Graph {
        match crate::parser::parse_document(src).unwrap() {
            Document::State(g) => g,
            other => panic!("not a state diagram: {other:?}"),
        }
    }

    fn flow(src: &str) -> Graph {
        crate::parser::parse(src).unwrap()
    }

    fn node<'a>(g: &Graph, sc: &'a Scene, id: &str) -> &'a SceneNode {
        &sc.nodes[g.node_index(id).unwrap()]
    }

    fn frame_of<'a>(sc: &'a Scene, id: &str) -> &'a SceneCluster {
        sc.clusters.iter().find(|c| c.id == id).unwrap()
    }

    /// The members of subgraph `id` in a row (one centre line, along x in
    /// source order) or a column (along y).
    fn in_line(g: &Graph, sc: &Scene, ids: &[&str], row: bool) {
        let at: Vec<&SceneNode> = ids.iter().map(|id| node(g, sc, id)).collect();
        for w in at.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (across, along) = if row {
                (b.y - a.y, b.x - a.x)
            } else {
                (b.x - a.x, b.y - a.y)
            };
            assert!(
                across.abs() < 1.0,
                "{} / {} not in line: {across}",
                a.id,
                b.id
            );
            assert!(along > 0.0, "{} / {} out of order: {along}", a.id, b.id);
        }
    }

    /// Every member lies inside its frame (and every nested frame
    /// inside its parent's), the frames come outermost-first at their
    /// depths, the scene's nodes and edges line up with the graph.
    fn contract(g: &Graph, sc: &Scene) {
        assert_eq!(sc.nodes.len(), g.nodes.len());
        for (n, m) in sc.nodes.iter().zip(&g.nodes) {
            assert_eq!(n.id, m.id);
        }
        assert!(sc.edges.len() >= g.edges.len());
        for (e, m) in sc.edges.iter().zip(&g.edges) {
            assert_eq!(
                (e.from.as_str(), e.to.as_str()),
                (g.nodes[m.from].id.as_str(), g.nodes[m.to].id.as_str())
            );
        }
        let ids: Vec<(String, String)> = g
            .sub_edges
            .iter()
            .map(|e| (end_id(g, e.from), end_id(g, e.to)))
            .collect();
        let drawn: Vec<(String, String)> = sc.edges[g.edges.len()..]
            .iter()
            .map(|e| (e.from.clone(), e.to.clone()))
            .collect();
        assert_eq!(ids, drawn, "sub-edges in order");
        assert!(sc.clusters.windows(2).all(|w| w[0].depth <= w[1].depth));
        for (si, s) in g.subgraphs.iter().enumerate() {
            let c = frame_of(sc, &s.id);
            assert_eq!(c.depth, depth(g, si), "{}", s.id);
            for &v in &s.nodes {
                let n = &sc.nodes[v];
                assert!(
                    n.x - n.w / 2.0 >= c.x
                        && n.x + n.w / 2.0 <= c.x + c.w
                        && n.y - n.h / 2.0 >= c.y
                        && n.y + n.h / 2.0 <= c.y + c.h,
                    "{} outside {}",
                    n.id,
                    s.id
                );
            }
            if let Some(p) = s.parent {
                let o = frame_of(sc, &g.subgraphs[p].id);
                assert!(
                    c.x >= o.x && c.y >= o.y && c.x + c.w <= o.x + o.w && c.y + c.h <= o.y + o.h
                );
            }
        }
        let mut bb = crate::scene::Bbox::new();
        crate::scene::grow_canvas(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
        let (x0, x1, y0, y1) = bb.finish();
        assert!(x0 >= -1e-6 && y0 >= -1e-6 && x1 <= sc.width + 1e-6 && y1 <= sc.height + 1e-6);
        assert!(sc.edges.iter().all(|e| e
            .bezier
            .iter()
            .chain(&e.waypoints)
            .all(|p| p.0.is_finite() && p.1.is_finite())));
    }

    const STATE: &str = include_str!("../examples/state.mmd");

    #[test]
    fn a_state_composite_flows_in_its_own_direction() {
        // Report B3 / P0 #10: `direction LR` inside `Validating` was
        // ignored, the composite drawn as a 126 × 368 column on a
        // 403 × 1133 canvas. Mermaid draws the composite as one row
        // (about 459 × 109) and so does flowmaid now.
        let g = state(STATE);
        assert_eq!(blocks(&g), vec![true]);
        let sc = scene(&g);
        contract(&g, &sc);
        let start = &g.nodes[g.subgraphs[0].nodes[0]].id;
        in_line(&g, &sc, &[start, "Syntax", "Semantics"], true);
        let v = frame_of(&sc, "Validating");
        assert!(v.w > 3.0 * v.h, "{} x {}", v.w, v.h);
        assert!(v.h < 120.0 && v.w < 470.0, "{} x {}", v.w, v.h);
        // Its row sits between the state that enters it and the choice
        // it leaves for, down the diagram's own flow.
        let (idle, decide) = (node(&g, &sc, "Idle"), node(&g, &sc, "decide"));
        assert!(idle.y + idle.h / 2.0 < v.y && v.y + v.h < decide.y - decide.h / 2.0);
        assert!(sc.height < 900.0, "{}", sc.height);
        // The transitions to and from the composite meet its frame.
        let into = sc
            .edges
            .iter()
            .find(|e| e.from == "Idle" && e.to == "Validating")
            .unwrap();
        let end = *edge_polyline(into).last().unwrap();
        assert!((end.1 - v.y).abs() < 1.5, "{end:?} vs frame top {}", v.y);
        // Deterministic.
        assert_eq!(to_svg(&sc), to_svg(&scene(&g)));
    }

    #[test]
    fn a_subgraph_direction_holds_while_nothing_links_out_of_it() {
        // mermaid: a subgraph keeps its own direction unless one of its
        // nodes links to a node outside it; edges to the subgraph itself
        // are fine.
        let g = flow(
            "flowchart TB\nX --> s\nsubgraph s\n  direction LR\n  A --> B --> C\nend\ns --> Y",
        );
        assert_eq!(blocks(&g), vec![true]);
        let sc = scene(&g);
        contract(&g, &sc);
        in_line(&g, &sc, &["A", "B", "C"], true);
        let s = frame_of(&sc, "s");
        assert!(node(&g, &sc, "X").y < s.y && node(&g, &sc, "Y").y > s.y + s.h);
        // Linked out of it, member to node: the parent's direction.
        for src in [
            "flowchart TB\nX --> A\nsubgraph s\n  direction LR\n  A --> B --> C\nend\nC --> Y",
            "flowchart TB\nsubgraph s\n  direction LR\n  A --> B --> C\nend\nC --> Y",
            // A member to its own subgraph crosses its border too.
            "flowchart TB\nsubgraph s\n  direction LR\n  A --> B --> C\nend\nC --> s",
        ] {
            let g = flow(src);
            assert_eq!(blocks(&g), vec![false], "{src}");
            let sc = scene(&g);
            in_line(&g, &sc, &["A", "B", "C"], false);
        }
        // An edge to a subgraph nested in it reaches inside it.
        let g = flow(
            "flowchart TB\nX --> t\nsubgraph s\n  direction LR\n  A --> B\n  subgraph t\n    C\n  end\nend",
        );
        assert_eq!(blocks(&g), vec![false, false]);
        // No links among the members: they already lie in one row
        // across the flow, as they always have.
        let g = flow("flowchart TB\nX --> s\nsubgraph s\n  direction LR\n  A\n  B\n  C\nend");
        assert_eq!(blocks(&g), vec![false]);
        in_line(&g, &scene(&g), &["A", "B", "C"], true);
        // The diagram's own direction asks for no block.
        let g = flow("flowchart LR\nsubgraph s\n  direction LR\n  A --> B\nend\nX --> s");
        assert_eq!(blocks(&g), vec![false]);
    }

    #[test]
    fn nested_subgraphs_flow_in_their_own_directions() {
        // mermaid's documentation example: TOP top-down in an LR chart,
        // B1 right-to-left and B2 bottom-to-top inside it.
        let g = flow(
            "flowchart LR\nsubgraph TOP\n  direction TB\n  subgraph B1\n    direction RL\n    \
             i1 -->f1\n  end\n  subgraph B2\n    direction BT\n    i2 -->f2\n  end\nend\n\
             A --> TOP --> B\nB1 --> B2",
        );
        assert_eq!(blocks(&g), vec![true, true, true]);
        let sc = scene(&g);
        contract(&g, &sc);
        in_line(&g, &sc, &["f1", "i1"], true);
        in_line(&g, &sc, &["f2", "i2"], false);
        let (top, b1, b2) = (
            frame_of(&sc, "TOP"),
            frame_of(&sc, "B1"),
            frame_of(&sc, "B2"),
        );
        assert!(b1.y + b1.h < b2.y, "B1 above B2");
        assert!(node(&g, &sc, "A").x < top.x && node(&g, &sc, "B").x > top.x + top.w);
        // The frames' palette depth is the nesting in the graph.
        assert_eq!((top.depth, b1.depth, b2.depth), (0, 1, 1));
    }

    #[test]
    fn a_bar_lies_across_its_blocks_own_flow() {
        // A fork / join bar is laid across the flow it stands in: upright
        // in an LR composite of a top-down diagram, its branches one
        // column — fitted alike by the whole graph and by the block.
        let g = state(
            "stateDiagram-v2\n[*] --> P\nstate P {\n  direction LR\n  state f <<fork>>\n  \
             [*] --> f\n  f --> A\n  f --> B\n  state j <<join>>\n  A --> j\n  B --> j\n  \
             j --> [*]\n}\nP --> Q",
        );
        assert_eq!(blocks(&g), vec![true]);
        let sc = scene(&g);
        contract(&g, &sc);
        for bar in ["f", "j"] {
            let n = node(&g, &sc, bar);
            assert!(n.h > 2.0 * n.w, "{bar}: {} x {}", n.w, n.h);
            let v = g.node_index(bar).unwrap();
            let (_, sizes) = crate::layout::node_texts(&g);
            assert_eq!(crate::scene::fit_bars(&g, &sizes)[v], (n.w, n.h), "{bar}");
        }
        in_line(&g, &sc, &["A", "B"], false);
        // The serpentine fold leaves a diagram with frames as `scene` draws it.
        let c = crate::fold::scene_compact(&g, &crate::fold::CompactOptions::for_extent(200.0));
        assert_eq!(to_svg(&c.scene), to_svg(&sc));
    }

    /// A fork into a row block and a plain state, joined again.
    const BAR_ROW: &str = "stateDiagram-v2\n[*] --> f\nstate f <<fork>>\nf --> P\nf --> Q\n\
        state P {\n  direction LR\n  a --> b\n  b --> c\n  c --> d\n}\n\
        state j <<join>>\nP --> j\nQ --> j\nj --> [*]";

    /// The sizes `sc` draws the fork/join bars of `g` at, in node order.
    fn bar_sizes(g: &Graph, sc: &Scene) -> Vec<(f64, f64)> {
        g.nodes
            .iter()
            .zip(&sc.nodes)
            .filter(|(n, _)| n.shape == Shape::ForkBar)
            .map(|(_, d)| (d.w, d.h))
            .collect()
    }

    /// B9 verification leftover: a fork or join that branches to a block
    /// spans the block's frame, and the interactive builders fitted it
    /// to the frame as it stood — so dragging a state inside the block,
    /// which stretches the frame, stretched the bar too (`f` 321 -> 446
    /// px with `d` pulled 250 px along its row), against the rule that
    /// a drag never resizes a bar. Every builder now draws the bar the
    /// static scene draws, wherever the block's states are dragged —
    /// also a bar that branches to a plain composite holding the block
    /// (review: `route()` still fitted that one to the frame as it
    /// stood, 335 -> 460 px, where `route_partial()` kept it).
    #[test]
    fn a_drag_inside_a_block_resizes_no_bar() {
        // A sideways diagram whose column block holds a fork of its own.
        let column = "stateDiagram-v2\ndirection LR\n[*] --> f\nstate f <<fork>>\n\
                      f --> P\nf --> Q\nstate P {\n  direction TB\n  a --> b\n  b --> c\n  \
                      state g <<fork>>\n  c --> g\n  g --> d\n  g --> e\n}\n\
                      P --> Done\nQ --> Done";
        // The bars branch to `Outer`, which sets no direction and holds
        // the block `P`; and to a block inside a block.
        let held = "stateDiagram-v2\n[*] --> f\nstate f <<fork>>\nf --> Outer\nf --> Q\n\
                    state Outer {\n  x --> P\n  state P {\n    direction LR\n    \
                    a --> b\n    b --> c\n    c --> d\n  }\n}\n\
                    state j <<join>>\nOuter --> j\nQ --> j\nj --> [*]";
        let nested = "stateDiagram-v2\ndirection LR\n[*] --> Outer\nstate Outer {\n  \
                      direction TB\n  [*] --> f\n  state f <<fork>>\n  f --> P\n  f --> x\n  \
                      state P {\n    direction LR\n    a --> b\n    b --> c\n    c --> d\n  }\n  \
                      state j <<join>>\n  P --> j\n  x --> j\n}\nOuter --> Done";
        for (src, dragged, by) in [
            (BAR_ROW, "d", (250.0, 0.0)),
            (BAR_ROW, "a", (-120.0, 40.0)),
            (column, "e", (0.0, 200.0)),
            (column, "a", (90.0, -60.0)),
            (held, "d", (250.0, 0.0)),
            (held, "c", (30.0, 140.0)),
            (nested, "d", (250.0, 0.0)),
        ] {
            let g = state(src);
            assert!(blocks(&g).contains(&true), "{src}");
            let sc = scene(&g);
            let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
            let v = g.node_index(dragged).unwrap();
            let mut to = at.clone();
            to[v] = (to[v].0 + by.0, to[v].1 + by.1);
            let routed = route(&g, &to);
            let drawings = [
                ("route", route(&g, &at)),
                ("route, dragged", routed.clone()),
                (
                    "route_sized, dragged",
                    crate::scene::route_sized(&g, &to, &sizes),
                ),
                ("route_partial", route_partial(&g, &to, &sc, &at)),
                (
                    "route_partial, dragged back",
                    route_partial(&g, &at, &routed, &to),
                ),
            ];
            // The drag did stretch the block's frame.
            let (was, now) = (frame_of(&sc, "P"), frame_of(&routed, "P"));
            assert!(
                (now.w - was.w).abs() > 20.0 || (now.h - was.h).abs() > 20.0,
                "{dragged}: the frame kept {} x {}",
                now.w,
                now.h
            );
            let bars = g
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.shape == Shape::ForkBar);
            for (k, bar) in bars {
                let drawn = (sc.nodes[k].w, sc.nodes[k].h);
                assert!(
                    drawn.0.max(drawn.1) > 100.0,
                    "{} is fitted: {drawn:?}",
                    bar.id
                );
                for (what, d) in &drawings {
                    contract(&g, d);
                    assert_eq!(
                        (d.nodes[k].w, d.nodes[k].h),
                        drawn,
                        "{what}, {dragged} dragged: bar {}",
                        bar.id
                    );
                }
            }
        }
    }

    /// Review: `route_partial()` with a `base` that is not this graph's
    /// (an edge label edited since) drew the rest afresh with each bar
    /// fitted to its block's frame as the drag left it (446 px where
    /// `route()` drew 321). The rest's bars are fitted as `route()` fits
    /// them whatever `base` holds, so the two agree — on the bar the
    /// static layout of the edited graph draws.
    #[test]
    fn a_stale_base_fits_a_bar_as_route_does() {
        let g = state(BAR_ROW);
        let base = scene(&g);
        let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
        for (edit, to) in [
            // A transition of the rest, and one inside the block (which
            // widens the block's own layout, and so the bar).
            ("f --> Q", "f --> Q : edited since"),
            ("a --> b", "a --> b : edited since"),
        ] {
            let edited = state(&BAR_ROW.replace(edit, to));
            let fresh = bar_sizes(&edited, &scene(&edited));
            for dragged in ["d", "Q"] {
                let v = edited.node_index(dragged).unwrap();
                let mut c = at.clone();
                c[v].0 += 250.0;
                let p = route_partial(&edited, &c, &base, &at);
                contract(&edited, &p);
                let what = format!("`{to}`, {dragged} dragged");
                assert_eq!(bar_sizes(&edited, &route(&edited, &c)), fresh, "{what}");
                assert_eq!(bar_sizes(&edited, &p), fresh, "{what}: route_partial");
            }
        }
        // The edit inside the block did change the bar.
        let edited = state(&BAR_ROW.replace("a --> b", "a --> b : edited since"));
        assert_ne!(
            bar_sizes(&edited, &scene(&edited)),
            bar_sizes(&g, &base),
            "a longer row, a longer bar"
        );
    }

    /// Review: to fit such a bar, `route()` laid the block out as the
    /// static layout does on every call (a 60-state block: 7 ms a frame
    /// for 3). The frame is now kept from the last layout of the same
    /// block ([`LAID`]): the answer is the same with it or without —
    /// before any `scene()` of the graph, after one, and after more
    /// graphs than the memo holds.
    #[test]
    fn the_kept_block_frames_change_no_bar() {
        // The same diagram with rows of different lengths.
        let row = |k: usize| {
            let chain: String = (0..k + 2)
                .map(|i| format!("  s{i} --> s{}\n", i + 1))
                .collect();
            format!(
                "stateDiagram-v2\n[*] --> f\nstate f <<fork>>\nf --> P\nf --> Q\n\
                 state P {{\n  direction LR\n{chain}}}\n\
                 state j <<join>>\nP --> j\nQ --> j\nj --> [*]"
            )
        };
        let graphs: Vec<Graph> = (0..LAID_MAX + 3).map(|k| state(&row(k))).collect();
        let far = |g: &Graph| -> Vec<(f64, f64)> {
            (0..g.nodes.len())
                .map(|i| (140.0 * i as f64, 90.0 * (i % 3) as f64))
                .collect()
        };
        // route() first: nothing kept yet on this thread.
        let routed: Vec<_> = graphs
            .iter()
            .map(|g| bar_sizes(g, &route(g, &far(g))))
            .collect();
        LAID.with(|m| assert_eq!(m.borrow().len(), LAID_MAX, "the memo is bounded"));
        for (k, g) in graphs.iter().enumerate() {
            let fresh = bar_sizes(g, &scene(g));
            assert!(fresh[0].0 > 100.0, "{k}: fitted, {fresh:?}");
            assert_eq!(routed[k], fresh, "{k}: route() before any scene()");
            assert_eq!(bar_sizes(g, &route(g, &far(g))), fresh, "{k}: and after");
            if k > 0 {
                assert_ne!(routed[k], routed[k - 1], "{k}: a longer row, a longer bar");
            }
        }
        // The first graphs fell out of the memo since: still the same.
        assert_eq!(
            bar_sizes(&graphs[0], &route(&graphs[0], &far(&graphs[0]))),
            routed[0]
        );
        // Sizes are part of what a frame is kept by: a host's wider
        // states give a longer bar, and its own again after.
        let g = &graphs[1];
        let own = crate::layout::node_sizes(g);
        let wide: Vec<(f64, f64)> = own.iter().map(|&(w, h)| (w + 40.0, h)).collect();
        let at = far(g);
        let widened = bar_sizes(g, &crate::scene::route_sized(g, &at, &wide));
        assert!(
            widened[0].0 > routed[1][0].0 + 40.0,
            "{widened:?} vs {:?}",
            routed[1]
        );
        assert_eq!(bar_sizes(g, &crate::scene::scene_sized(g, &wide)), widened);
        assert_eq!(
            bar_sizes(g, &crate::scene::route_sized(g, &at, &own)),
            routed[1]
        );
    }

    #[test]
    fn route_and_drags_lay_a_block_out_as_the_scene_does() {
        let g = state(STATE);
        let sc = scene(&g);
        let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        // route() of the scene's own centres: the same frame, the row's
        // transitions running along it, side to side.
        let r = route(&g, &at);
        contract(&g, &r);
        let (a, b) = (frame_of(&sc, "Validating"), frame_of(&r, "Validating"));
        assert!((a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() < 0.5);
        assert!((a.w - b.w).abs() < 0.5 && (a.h - b.h).abs() < 0.5);
        let syntax = node(&g, &r, "Syntax").clone();
        let e = r
            .edges
            .iter()
            .position(|e| e.from == "Syntax" && e.to == "Semantics")
            .unwrap();
        let pts = edge_polyline(&r.edges[e]);
        assert!(
            (pts[0].0 - (syntax.x + syntax.w / 2.0)).abs() < 1.5,
            "{:?}",
            pts[0]
        );
        // Picked where it is drawn.
        let mid = pts[pts.len() / 2];
        assert_eq!(r.edge_at(mid.0, mid.1, 1.0), Some(e));

        // A drag inside the block: the frame follows its member, the
        // edges into the dragged state end on its new box, the rest's
        // edge to the frame meets the frame as it now stands — as
        // route() draws the same centres.
        let v = g.node_index("Syntax").unwrap();
        let mut moved = at.clone();
        moved[v].0 += 30.0;
        moved[v].1 += 60.0;
        let p = route_partial(&g, &moved, &sc, &at);
        contract(&g, &p);
        let full = route(&g, &moved);
        let (fp, ff) = (frame_of(&p, "Validating"), frame_of(&full, "Validating"));
        assert!((fp.h - ff.h).abs() < 0.5 && (fp.y - ff.y).abs() < 0.5);
        assert!(fp.h > a.h + 50.0, "{} vs {}", fp.h, a.h);
        let n = &p.nodes[v];
        for e in p.edges.iter().filter(|e| e.to == "Syntax") {
            let end = *edge_polyline(e).last().unwrap();
            assert!(
                (end.0 - n.x).abs() <= n.w / 2.0 + 1.5 && (end.1 - n.y).abs() <= n.h / 2.0 + 1.5,
                "{end:?} off Syntax"
            );
        }
        let out = p.edges.iter().find(|e| e.from == "Validating").unwrap();
        let start = edge_polyline(out)[0];
        assert!(
            (start.1 - (fp.y + fp.h)).abs() < 1.5,
            "{start:?} vs frame bottom"
        );
        // A drag outside the block leaves the block's drawing alone.
        let w = g.node_index("Done").unwrap();
        let mut away = at.clone();
        away[w].0 += 40.0;
        let q = route_partial(&g, &away, &sc, &at);
        contract(&g, &q);
        for (x, y) in q.edges.iter().zip(&sc.edges) {
            if x.from == "Syntax" || x.to == "Syntax" {
                assert_eq!(format!("{:?}", x.bezier), format!("{:?}", y.bezier));
                assert_eq!(x.waypoints.len(), y.waypoints.len());
            }
        }
    }

    #[test]
    fn the_quality_harness_reads_a_block_in_its_own_direction() {
        // A row inside a top-down diagram is no sideways wiggle and no
        // edge entering a frame from the side.
        let g = flow(
            "flowchart TB\nX --> s\nsubgraph s\n  direction LR\n  A --> B --> C\n  \
             subgraph t\n    direction TB\n    D --> E\n  end\n  C --> t\nend\ns --> Y",
        );
        let m = crate::quality::measure(&g, &scene(&g));
        assert_eq!((m.wiggles, m.frame_side, m.lat_wiggles), (0, 0, 0), "{m:?}");
    }
}
