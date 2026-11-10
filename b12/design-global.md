# B12 "subgraphs as blocks" — design d-global: one global layering with block constraints

Label: `d-global`. Source read: `b12/tree/src` (0.38.0, commit 0366084); every `file:line` below is into that tree.
Everything was measured on the sanitized analog `b12/in/infra-overview.mmd` and the corpus; nothing from `b12/private/` is quoted.
Scratch: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/d-global/` (called `D/` below).

## 0. The design in one page

Keep the single Sugiyama pass over all nodes (`scene_clustered` → `layout_clustered_ranked` → one `edgepass::finish`). Change three things in it:

1. **Layering is per level, bottom-up, with blocks as units.** A level is the root or one subgraph; its units are its direct member nodes and its direct child subgraphs. Each level's units are layered on their own; a block of `h` inner layers pushes its successors `h` layers down. A node's global layer is the sum of the starts of its chain of blocks plus its own start. Every subgraph therefore owns one contiguous layer interval, linked blocks never share a layer, and unlinked blocks may share an interval side by side.
2. **A long edge's chain bends where it crosses a block border**, in the gap before the block, instead of dragging the block's walls out to wherever its outer run lies.
3. **Breadth is assigned hierarchically**: every block is packed to its least breadth on its own (deepest first), relaxed towards its inner links, and then moves as one rigid unit of its parent level.

Everything else is today's machinery and already does the right thing once the layers are block-shaped: wall dummies and `apath` send a long edge's dummies outside foreign blocks (the corridors), `ortho`'s global row gaps coincide with the gaps between bands, frames come from member positions, `route()` and `route_partial()` keep their contracts.

What the prototype (a real code hack, section 3) shows on the analog: 4 bands as in mermaid (actors / External services + Code hosting / PROD / the other two servers), frame compactness 6.90 → 3.43 mean and 11.43 → 5.09 max (mermaid 3.25 / 4.59), largest frame 0.767 → 0.314 of the flow extent (mermaid 0.308), 0 runs through a foreign frame (0.38: 3), 0 linked side-by-side pairs (0.38: 5), canvas 2208 × 1281 = 2.83 Mpx² (0.38: 3.41, mermaid 5.18), edge length 20 356 px (0.38: 22 350, mermaid 30 835). With the replication edge turned as in the real diagram the three servers stack one under another. Block order is identical under all five declaration-order permutations that scramble mermaid.

What it does not yet deliver, frankly: crossings went **up** (24 → 40, mermaid 16), four edges enter a frame through a lateral side, 9 of 20 labels still have a foreign line within 12 px, and on bench-75 the compact blocks squeeze 69 inter-block edges into gaps the edge stages do not size (lines 0.5 px apart, drag jumps 16 → 46, `route()` 8 → 15 ms). Sections 1.6-1.8 specify the missing pieces (block-level crossing step, flow-facing ports, band-gap lanes, corridors in `route()`); they are not in the prototype.

## 1. The algorithm

### 1.1 Inputs and terms

Inputs, all available in `scene_clustered` (`scene.rs:3444`) today: the graph `ga` (real edges plus one stand-in edge per sub-edge, `scene.rs:3455-3519`), `node_cluster` (each node's subgraph chain, outermost first, `scene.rs:4252`), the rank-only groups (`sub_edge_proxies` `scene.rs:3994` + `grid_pairs` `scene.rs:4172`, later narrowing's stackings and wraps), node sizes, `g.lead_nodes`.

- **Level** `L`: the root, or a non-empty subgraph.
- **Unit** of `L`: a node whose innermost subgraph is `L`, or a non-empty subgraph whose parent is `L` (a *block unit*).
- **Key** of a unit (declaration order): a node's index in `Graph::nodes`; a block's = the least node index among its members at any depth. Keys are distinct.
- **Height** `h(u)`: 1 for a node; for a block the number of layers of its own level (computed first).
- **Band**: a maximal run of root layers covered by overlapping root units.

Which subgraphs are blocks: **every non-empty subgraph**, with or without an edge across its border. The one exception is unchanged from B9: a subgraph `compound::judge` (`compound.rs:144-189`) accepts (own `direction` differing from its surroundings, an inner edge, no border edge) is cut out by `compound::split` before any of this runs (`scene.rs:3058`) and is a plain proxy node of height 1 here.

### 1.2 Steps

**S1. Level links.** Every link `(a, b)` is assigned to the level of the lowest common ancestor of `a` and `b` and collapsed to the pair of units `(cu, cv)` of that level that contain `a` and `b` (`cu = a` when `a` is a direct member). Links with `cu == cv` belong to a deeper level and are dropped here. The links are: every `ga.edges` entry with `from != to` (real edges and stand-ins, invisible links included), and every `from × to` pair of every rank-only group. Parallel links between one pair of units are one link whose weight is their count.

**S2. Cycle breakers, per level.** Depth-first search over the level's units:
- roots: first the units with no incoming link, by key; then every other unit, by key;
- a unit's out-links are followed in order of the target's key;
- a link that reaches a unit still on the DFS stack is **upstream** (drawn against the flow); every other link is downstream.

For layering, an upstream link is used reversed. This is mermaid's rule (R2 of `understand-mermaid.md`) with one difference that matters: the orders are declaration order of the *units*, not the order of the edge statements, so shuffling edge lines cannot change the result.

**S3. Layers, per level, deepest level first.** Longest path over the level's DAG, Kahn's algorithm, queue seeded by key: `start(v) = max over links u→v of start(u) + h(u)`, sources at 0. Then, in reverse topological order: a source that is a **block**, and a lead note (`g.lead_nodes`), sinks to `min over its successors of start(v) − h(u)` when that is later (a source block sits in the band just above its nearest consumer; plain source nodes stay on the level's first layer, as flowmaid draws them today). The level's height is `max(start + h)`.

**S4. Global layers.** `layer(v) = start(v) + Σ start(block)` over the blocks of `v`'s chain. A real edge is a back-edge (`Ranking::back`) iff `layer(to) < layer(from)`.

Properties, by construction: a block's members fill exactly `[start, start + h)`; two units of one level joined by a link occupy disjoint intervals (so `sib_side_linked` = 0); no edge joins two nodes of one layer.

**S5. Ordering** (`place`, `layout.rs:3343-3397`, unchanged machinery): wall dummies on every layer of each block's interval, `apath` for chain dummies (target's path where the layer lies in the target's interval, else the source's, else the common prefix: `layout.rs:3302-3341`), weighted-median sweeps, `transpose`, `cluster_keys` + `enforce_contiguity`. New, section 1.6: a block-level transpose.

**S6. Coordinates.** Brandes-Köpf as today, except that the link between two dummies of one chain whose cluster paths differ is not offered for alignment (the chain bends at the border). Then the hierarchical packing:
- deepest block first; the block's items are its direct nodes, its inner chain dummies, its two wall chains and its child blocks (already packed, each one rigid unit with fixed inner offsets);
- least breadth: longest path over the left-of relation of every layer with `pair_sep` (`layout.rs:5379`) as distances, left wall at 0; the right wall's position is the block's breadth;
- relax: `COMPACT_SWEEPS` Gauss-Seidel sweeps placing each unit at the weighted median (weights as `compact_breadth`, `layout.rs:5201`: 1 / 2 / 8) of its adjacent-layer links **inside this level**, clamped between its layer neighbours;
- the root level does the same with top-level blocks, loose nodes and outer dummies as units and all remaining links as pulls.

**S7. Layer lines, frames, edge stages**: as today (`layer_lines` `layout.rs:4062-4175`, `cluster_raw_boxes` `scene.rs:5346`, `edgepass::finish`), with the additions of 1.7.

### 1.3 Tie-breaks (all deterministic)

| decision | rule |
|---|---|
| unit order | key = least member node index; never equal |
| DFS roots | units without in-link by key, then the rest by key |
| DFS out-links | by key of the target unit |
| longest path | Kahn, queue seeded by key; FIFO |
| sinking sources | reverse topological order, one pass |
| ordering | today's sweeps; equal medians keep their order (`reorder`, `layout.rs:4694`) |
| packing | fixed sweep count and directions; `weighted_median` (`layout.rs`, after `compact_breadth`) takes the lower point on a tie |
| containers | `Vec` and `BTreeMap` only where iteration reaches the output |

Considered and rejected for S2: a minimum weighted feedback set (Eades-Lin-Smyth with key ties). On the analog it reverses 3 edges instead of 5 by putting PROD above External services, which makes `DNS → ingress proxy`, the diagram's entry chain, run upwards. A walk from the sources keeps every chain that starts at a source pointing along the flow.

### 1.4 Edges between blocks: channels and lanes

- **Downstream, more than one layer apart**: a dummy chain, as today. It leaves the source's block through the downstream side (the chain's dummies take the source's path while inside its interval), passes every other band beside that band's blocks (common-prefix path, ordered outside foreign walls), and enters the target's block through its upstream side, descending inside it beside the members until the target's layer. Its label rides the chain (1.7).
- **Downstream, adjacent layers** (last layer of one band to the first of the next): no dummy; `ortho` draws the Z in the gap between the bands.
- **Upstream between two blocks or a block and a node: a back-edge lane**, planned by `back_lanes` (`layout.rs:4427`) exactly as a back-edge between two plain nodes is today, scoped to the level of the link. This is a decision taken on evidence, not the first plan: the prototype has both variants (`FM_B12=1` lanes, `FM_B12=2` an upstream dummy chain that enters the target block from its downstream side as ELK does). Lanes are no worse on any input and better on most (analog: crossings 40 vs 43, struck labels 0 vs 3, least lane gap 12.2 vs 7.0 px, label offset 0.136 vs 0.185; bench-75: crossings 335 vs 373, edge length 43 935 vs 49 576), and `route()` already draws exactly these edges as lanes by geometry (`route_backs`, `scene.rs:8097`), so `scene()` and `route()` agree on them without new code. The chain variant stays in the prototype for a later comparison once band gaps are laned (1.7).
- **Upstream between two plain nodes of one level**: lane, unchanged.

### 1.5 Cases

| case | rule |
|---|---|
| **nested subgraphs** | recursion of S1-S4; an edge from outside to a member two levels down is a link of the common-ancestor level to the outermost block; its chain adopts the target's path level by level (`fit`, `layout.rs:3306-3315`), so it crosses each frame on the upstream side |
| **node → whole subgraph** | the stand-in edge and the proxy group both collapse to `(unit of node, block)`; the block as a whole sits below the node; drawn as today: the stand-in's channel cut at the frame (`cut_to_frames`, `scene.rs:3772`), arrowhead on the upstream side, at mid-side when alone (`ports::assign`, `ports.rs:774`). The precondition "box wholly before or after the other end" (`scene.rs:3604-3607`) now always holds |
| **subgraph → subgraph** | same, both ends cut at their frames |
| **member → its own subgraph** | today's meaning kept: an edge to the block's entry members (`own_box_targets`); a link inside the block's level from the member to each entry member; if the member is itself the entry, no link; drawn whole (`scene.rs:3474-3485`) |
| **subgraph → its own member** | no link; drawn as the stub `own_box_stub` (`scene.rs:3832`) |
| **subgraph → itself** | no link; drawn as today (0.38 draws a zero-length path, a known defect outside this package) |
| **cycle between blocks** | S2: the link that closes the cycle on the walk from the sources is upstream and becomes a lane; the blocks keep disjoint intervals |
| **2-cycle between blocks** (`A → B`, `B → A`) | the link met first on the walk (from the lower key, or from the side a source reaches) is downstream, the other a lane |
| **hub linked to many blocks** (the analog's DNS node) | no special layering rule. Its block sits where S3 puts it; each out-edge to a later band is a chain, and the chains to blocks beyond the next band share the corridor beside the bands between. B11's comb bundling then applies to that fan unchanged (a fan's branches now leave through one side and run parallel, which is the shape the comb wants) |
| **empty subgraph** | no members at any depth: not a unit, no frame, its sub-edges skipped (today's contract, `scene.rs:130-134`) |
| **edge-less members** | start 0 of their block; `grid_pairs` still packs loose members into rows by rank-only pairs, which are inner links of that block |
| **subgraph with its own `direction`, no border edge** | B9, untouched: `compound::split` cuts it out, it is laid out in its own direction and stands as a proxy node; only the rest around it is block-layered (`examples/state.mmd` and `fanout-frames` come out byte-identical in the prototype) |
| **subgraph with `direction` and a border edge** | the direction is not honoured (mermaid's rule, README:284-287); an ordinary block |
| **LR / RL / BT** | layers run along the flow in every direction; the title strip is handled where it is today (TD: first layer `lextra`, BT: last layer, LR/RL: across the flow via `header_b` in `pair_sep`, `layout.rs:3226-3254`, `5402-5415`); the packing passes `header_b` through |
| **state diagrams** | a composite is a block; its `[*]` states and note cards are members (parser `5903-5912`, `5979-6003`); a transition to a composite is a sub-edge (row 2); a lead note sinks to the layer before its target inside its level (S3); `right of` cards and bar cards keep `bar_cards` ordering (`layout.rs:3398-3411`); fork/join bars are sized by `fit_bars` before layout (`scene.rs:3063`), unchanged |
| **invisible links** | a link like any other in S1-S3 (they rank today too), never a chain, never a lane (`layout.rs:3104-3112`); an upstream one is ignored |
| **very large drawings** | S1-S4 are linear in nodes + links × nesting depth and run inside `rank`, so every candidate of the search gets them; the packing is one pass per `place`. Past `ROUTE_MAX_EDGES` = 400 (`router.rs:263`) the ortho stage and the router are skipped as today: blocks and corridors still hold, because they come from the layout, not the router. Past `SEARCH_MAX_ELEMENTS` = 600 (`layout.rs:217`) there is no search as today; the block-level transpose (1.6) is skipped with it. `WALK_MAX_ELEMENTS` no longer applies on the clustered path (one walk per level) |

### 1.6 Ordering additions (specified, not prototyped)

- **Block transpose.** After `transpose` (`layout.rs:3373`), per level, try swapping two neighbouring sibling units as wholes (a block with its walls, inner order kept) on every layer they share; keep the swap when `count_crossings` (`layout.rs:4805`) drops. Two rounds, inside the search budget. Today's `transpose` only swaps same-path neighbours and never a wall (`layout.rs:4769-4772`), and `cluster_keys` (one mean position per subgraph, `layout.rs:5487`) is the only thing that orders siblings: on the analog it puts the block fed by three hub edges on the far side of its sibling, the main source of the extra crossings.
- **Entries clear of the title chip (TD/BT).** A chain dummy on a block's title-side layer whose chain crosses that border must lie beyond the title chip's breadth (`title_head`, `scene.rs:2885`) plus `CHIP_CLEAR`: a lower bound on its position in the packing; the block grows when title + entries do not fit. Without it the edge stages, which never strike a title (PLAN invariant 4), push such an edge round to a lateral side (seen in the prototype).

### 1.7 Gaps, labels, ports (specified, not prototyped)

- **Band-gap room.** `track_need` (`layout.rs:4002-4051`) counts the overlapping sideways steps of adjacent-layer links per gap. The bends S6 puts at block borders are such steps and must be counted there, one `TRACK_PITCH` each; the lane order in a gap follows the order of the chains' outer columns so parallel lanes do not cross.
- **Label slot.** Today the labelled dummy is the chain's middle one (`layout.rs:3132-3147`). New: the middle dummy of the part of the chain that lies in the common-ancestor level (outside both end blocks), so a label never lands inside a block or on its border; an edge between adjacent bands keeps today's labelled-gap room (`short_labels`, `layout.rs:4097-4145`).
- **Flow-facing ports.** An edge end on a member of a block the edge leaves or enters uses the member's side facing the border it crosses (downstream side when leaving downstream, upstream side when entering from upstream); a lateral side only for an end whose other end lies in the same band. In `ports::assign`.
- **Aligned leading borders.** Members on a block's first layer are aligned on the layer's leading line (top in TD) instead of its centre line when two sibling blocks open on that layer, so their frames start level (the analog's two bottom blocks start 17 px apart today because their first-row nodes differ in height).

### 1.8 What runs in `route()`

`route(g, centres)` gets positions from the host, so **none of S1-S6 runs there**. What keeps the look:

- frames are `cluster_raw_boxes` of the members (`scene.rs:5346`, `5389`), so a compact block stays compact, and follows its members on a drag (H3);
- upstream edges are decided by geometry and drawn as lanes (`route_backs`), the same rule the static scene now uses for them (1.4);
- a downstream edge between two cleanly stacked top-level boxes gets today's 4-point channel (`cross_cluster_route`, `scene.rs:4638-4655`, `8400`). Measured: at the prototype's positions unchanged `route()` leaves **0** edge/foreign-frame pairs on the analog and bench-75 (0.38: 3 and 0), 3 on dense-lr (13), 8 on dense-td (29);
- **new: band corridors.** For a downstream edge between different top-level units whose channel would cross a foreign top-level frame, build the path from the frames alone: out through the flow-facing side, a lane in the gap after the source's band, a corridor beside the crossed frames on the nearer free side (slot by edge index), a lane in the gap before the target's band, in through the flow-facing side. Bands and gaps are the merged flow intervals of the top-level frames and loose nodes, recomputed per call (the same merge `ortho_pass` does for rows, `ortho.rs:480-506`). Its interior points are handed to `ortho` as columns, which today exist only in static mode (`ortho.rs:586-603`). This removes the remaining pairs on the nested dense inputs.

## 2. Code plan

### 2.1 Changes by file

**`layout.rs`**

| where | change |
|---|---|
| `rank` `2281-2440` | with `node_cluster`: after the hub graph is built, call new `block_layers(g, nc, rank_only)` (S1-S4, about 150 lines; prototype: `D/prototype.patch`) instead of steps 1-4; set `back` from the layers. `seat_box_sources` (`2497-2773`) is no longer called on this path: a box fed from above lies below its feeder by construction. Lead-note step `2406-2432` becomes the sink rule. Without `node_cluster` the function is untouched |
| `narrow` `1656-1856` | stacking and wrap groups (`stack_candidates` `2157`, `wrap_candidates` `2042`) keep working as level links through `rank`; the "no path both ways" filter `2237-2242` becomes moot (linked blocks are already stacked); `stagger_proposals` (`2877`) may only move a node inside its block's interval |
| `place` chains `3066-3149` | unchanged; label dummy choice `3132-3147` moves after `apath` and picks the outside part (1.7) |
| `place` ordering `3343-3397` | + `block_transpose` after `transpose` (1.6) |
| `coordinates_bk` `4880` | chain links across a block border are dropped from `up` / `down` before `type1_conflicts` (prototype) |
| after `coordinates_bk` (the `opts.compact` call site in `place`) | new `block_compact` (S6, about 170 lines; shares unit / sweep code with `compact_breadth` `5201`), run when clustered. Rule for unchanged drawings: a block whose Brandes-Köpf breadth is within a few px of its packed breadth keeps its Brandes-Köpf coordinates, so a drawing that was already compact does not move |
| `layer_lines` `4062-4175`, `track_need` `4002-4051` | band-gap room and aligned leading borders (1.7) |
| `back_lanes` `4427` | unchanged (the prototype's extra filter only serves the chain variant) |
| public API (`layout_clustered`, `layout_sized`, `LayoutResult`, `Placed`, `28-50`, `1503-1520`) | signatures unchanged; `layout_clustered` gets the block layering too (documented) |

**`scene.rs`**: `scene_labelled` (`3050`), `scene_clustered` (`3444`), `sub_edge_proxies`, `grid_pairs`, `cluster_raw_boxes`, the frame growth `3535-3599`, `clusters_line_up` (`5277`): unchanged. `route_built` (`4435`): band corridors beside `cross_cluster_route` (1.8, about 120 lines).

**`ports.rs`** `assign` (`774`): flow-facing sides (1.7). **`ortho.rs`** `ortho_pass` (`477`): accept corridor columns in routed mode. **`edgepass.rs`**: unchanged; `ctx`'s geometric `back` (`221-230`) now agrees with the layout's for every inter-block edge. **`quality.rs`** / `tests/quality.rs`: new measures (frame slack, lateral border crossings, runs through foreign frames per block) and re-baselined ratchets (section 4). **README / CHANGELOG**.

**What `compound.rs` becomes: nothing new.** It stays the mechanism for direction blocks only, with its rule, split, join, `route` and `route_partial` as they are. This design does not extend the split to border edges; it makes the global pass produce blocks instead. That is the point of the angle: no edge is ever laid out in two halves, so H1 (one `SceneEdge` per edge, index-parallel), one port pass, one ortho pass, one 400-edge cap and one label pass hold without a stitching step. Each B9 part goes through `scene_labelled` and gets the block layering inside when it has nested subgraphs.

### 2.2 Consistency of the entry points

| entry | what happens |
|---|---|
| `scene()` / `scene_sized()` | S1-S7; one scene, one `finish` |
| `route()` / `route_sized()` | no layering; frames from members; lanes by geometry; channel or band corridor for downstream inter-block edges (1.8). Host positions are never moved |
| `route_partial()` | unchanged logic (`4870-5149`): `clusters_line_up` holds because frames are still a function of member positions; a frame with a moved member is redrawn as `route()` draws it; `PARTIAL_ROUTE_SHARE` fallback as today |
| `scene()` vs `route()` at the scene's positions | not equal today (frames up to 617 px apart on bench-75) and not made equal here; it narrows: frames differ only by the growth over inner lanes, which S6 keeps beside the members; upstream inter-block edges are lanes in both |
| hit-testing (`hit_test`, `cluster_at`, `hits_in_rect`, `scene.rs:257-337`) | read the `Scene` only; they need nested frames and disjoint siblings, which the packing guarantees through `pair_sep` + `cluster_gap` (prototype: `frame_overlap` 0, `node_frame` 0 on every input, both modes) |
| `fold::scene_compact` | refuses any graph with subgraphs (`fold.rs:246-248`): unchanged, and B12 never runs on a graph without them |
| `to_mermaid`, json export | graph / scene only: unchanged |
| `advance`, class, ER | no subgraphs (`class.rs:194`, `er.rs:155`): the new code is behind `node_cluster.is_some()` |

### 2.3 Cost

Measured with the prototype binary, `examples/quality.rs --drag`, same binary with the switch off as baseline, one shared 4-CPU box (two runs; noisy):

| file | `scene_ms` base → proto | `route_ms` | worst drag frame | limits (PLAN: 2× / 1.25× or +1 ms) |
|---|---|---|---|---|
| analog | 9.6 → 10.1-10.4 | 3.2 → 4.7 | 7.1 → 8.3-8.5 | – |
| bench-75 | 33.8 → 41-44 | 8.2 → **15.4-16.2** | 14.7 → 18.9-20.7 | 61.9 / 15.8 / 25.3 |
| dense-td | 25.6 → 23-25 | 6.5 → 7.7-7.9 | 14.1 → 14.5-17.2 | 50.7 / 9.2 / 17.8 |
| dense-lr | 22.7 → 25-33 | 4.9 → 6.9-11.8 | 8.5 → 12.5-14.4 | 46.8 / 8.0 / 17.8 |
| advanced | 0.90 → 1.1-1.5 | 0.57 → 0.8-0.9 | 1.8 → 2.1-2.8 | 1.42 / 1.72 / 2.61 |
| subgraph | 0.26 → 0.19 | 0.11 → 0.06 | 0.75 → 0.4 | 0.16 / 1.05 / 2.04 |

The layout itself costs about 1 ms on the analog and 7-10 ms on bench-75 (the prototype's packing rescans all layers per level; with per-level item lists it is one pass). **`route()` got slower with no change to `route()`**: compact blocks put more edges into narrower gaps and the router repairs more. bench-75 is at its `route_ms` limit. The band corridors (expected +0.1-0.3 ms) give the edge stages clean inputs and should bring that down; this has to be measured, not assumed.

## 3. Prototype

**What it is**: a real code hack in a copy of the tree, 514 diff lines in `layout.rs` only (`D/prototype.patch`, tree copy `D/tree`): `block_layers` hooked into `rank` (S1-S4), the alignment rule in `coordinates_bk` and `block_compact` (S6), plus the upstream-chain variant. Switch `FM_B12` = 0 (off: byte-identical to `fm038` on the analog, checked with `cmp`), 1 (upstream edges as lanes, the design), 2 (upstream chains). Routing, ports, labels, `route()` are untouched 0.38 code. Not in it: 1.6, 1.7, the band corridors of 1.8, the "keep Brandes-Köpf when already compact" rule. Output is deterministic (three inputs rendered twice, identical bytes).

**Pictures** (all under `D/`):
- `infra-overview.proto-lanes.png` — the analog, the design's variant; `infra-overview.038-proto-mermaid.png` — 0.38 | prototype | mermaid side by side;
- `infra-overview-v2.proto-lanes.png` — the analog with the replication edge as in the real diagram: the three servers stack, 1643 × 1459;
- `infra-overview.proto-channels.png`, `infra-overview.proto-channels.routed.png` — the upstream-chain variant, and unchanged `route()` at its positions;
- `advanced.proto-lanes.png`, `advanced.038-proto-mermaid.png`, `subgraph.proto-lanes.png`, `subgraph.proto-mermaid.png`, `fanout-frames.proto-lanes.png`, `state.proto-lanes.png`, `bench-75.proto-lanes.png`, `dense-td.proto-lanes.png`, `dense-lr.proto-lanes.png`.

**Block metrics, analog** (`metrics/run.sh fm` + `judge`; tables `D/proto-lanes/metrics.tsv`, `D/proto-channels/metrics.tsv`):

| metric | 0.38.0 | proto (lanes) | proto (chains) | mermaid |
|---|---|---|---|---|
| canvas | 2787 × 1225 | 2208 × 1281 | 2336 × 1281 | 2199 × 2358 |
| `area` | 3.41 M | 2.83 M | 2.99 M | 5.18 M |
| `compact_mean` / `compact_max` | 6.90 / 11.43 | 3.43 / 5.09 | 3.56 / 5.72 | 3.25 / 4.59 |
| `span_max` | 0.767 | 0.314 | 0.314 | 0.308 |
| `sib_side` / `sib_stacked` | 6 / 4 | 2 / 8 | 2 / 8 | 2 / 8 |
| `sib_stagger` / `sib_side_linked` | 3 / 5 | 1 / 0 | 1 / 0 | 0 / 0 |
| `xframe_runs` (px) | 3 (2012) | 0 | 0 | 0 |
| `crossings` | 24 | **40** | 43 | 16 |
| `edge_len` | 22 350 | 20 356 | 21 393 | 30 835 |
| `straight_max` | 1175 | 1093 | 1114 | 1236 |
| `lanes_max` / `lane_gap_min` | 6 / 9.3 | 3 / 12.2 | 6 / 7.0 | 6 / 12.0 |
| `lbl_off_mean` / `lbl_off_max` | 0.198 / 0.332 | 0.136 / 0.323 | 0.185 / 0.342 | 0.130 / 0.329 |
| `lbl_struck` / `lbl_crowd` / `lbl_gap_min` | 0 / 8 / 2.5 | 0 / **9** / 2.5 | 3 / 8 / 0.0 | 0 / 0 / 20.0 |
| `frame_overlap`, `node_hits`, `warn` | 0 | 0 | 0 | 0 |

`judge` on the lanes variant: 9 must-fails over the 8 inputs (0.38: 7, all on the analog). On the analog it clears `xframe_runs`, `sib_side_linked`, `compact_max`, `compact_mean`, `span_max`, area, edge length, `straight_max`, label offsets, lane gap; it fails `crossings` (40 > 24), `lbl_crowd` (9 > 4) and `sib_stagger` (1). Elsewhere: crossings on bench-75 (335 > 334), dense-td (126 > 90), dense-lr (66 > 52) and `lbl_struck` on dense-lr (7 > 5).

**Other inputs** (lanes variant):

| input | engine | W × H | compact mean / max | span_max | side / stacked / stagger / side_linked | xframe runs (px) | crossings | edge_len |
|---|---|---|---|---|---|---|---|---|
| advanced | 0.38 | 596 × 1236 | 5.75 / 8.28 | 0.377 | 0 / 1 / 0 / 0 | 0 | 0 | 2346 |
| advanced | proto | 593 × 1236 | 5.74 / 8.24 | 0.377 | 0 / 1 / 0 / 0 | 0 | 0 | 2532 |
| advanced | mermaid | 906 × 1484 | 5.33 / 8.24 | 0.428 | 0 / 1 / 0 / 0 | 0 | 0 | 5024 |
| subgraph | 0.38 = proto | 382 × 758 | 6.28 / 7.49 | 0.562 | – | 0 | 0 | 632 |
| fanout-frames, state | 0.38 = proto | byte-identical | | | | | | |
| bench-75 | 0.38 | 2222 × 1965 | 35.88 / 52.03 | 0.464 | 4 / 6 / 2 / 4 | 4 (2508) | 302 | 80 453 |
| bench-75 | proto | 1648 × 970 | 6.70 / 16.25 | 0.215 | 0 / 10 / 0 / 0 | 0 | 335 | 43 935 |
| bench-75 | mermaid | 4327 × 1058 | 5.33 / 9.88 | 0.161 | 0 / 10 / 0 / 0 | 0 | 228 | 106 034 |
| dense-td | 0.38 | 4249 × 1806 | 12.45 / 24.66 | 0.795 | 3 / 1 / 3 / 3 | 12 (14 713) | 80 | 63 286 |
| dense-td | proto | 4182 × 1711 | 6.07 / 8.11 | 0.296 | 1 / 3 / 0 / 0 | 3 (3349) | 126 | 73 450 |
| dense-td | mermaid | 4131 × 4378 | 8.77 / 15.12 | 0.308 | 0 / 4 / 0 / 0 | 0 | 68 | 142 183 |
| dense-lr | 0.38 | 2302 × 3468 | 8.76 / 11.78 | 0.603 | 7 / 4 / 4 / 5 | 8 (4081) | 46 | 38 537 |
| dense-lr | proto | 3548 × 2443 | 4.26 / 6.28 | 0.207 | 1 / 10 / 1 / 0 | 0 | 66 | 37 794 |
| dense-lr | mermaid | 4959 × 3858 | 3.74 / 6.92 | 0.195 | 1 / 10 / 0 / 0 | 0 | 10 | 78 383 |

flowmaid's own harness (`D/q.base.tsv` → `D/q.lanes.tsv`): edge/foreign-frame pairs static 46 → 4 and routed 45 → 11 over the eight inputs; the analog goes from 3 defects to 0 in both modes; drawn crossings 452 → 567 static.

**What each piece buys** (analog, chain variant): layering alone 3051 px wide, `compact_max` 11.55, `compact_mean` 5.75 (blocks stacked but as loose as before, because outer runs pull the walls); with the packing 2336 px, 5.72 / 3.56; packing off but alignment rule on: 3456 px, 8.12 / 5.26. The layering gives the bands, the packing gives the compact frames; neither is enough alone.

**Stability**: the five permutations of `u-mermaid/in/perm-*.mmd` (edges reversed, edges rotated, subgraphs reversed, actors reversed, one more inner edge) give the same layers per block and the same five upstream edges every time. mermaid moves the actors to mid-drawing and External services below PROD on the first of them.

**Frankly, what still looks worse than mermaid, and why**

1. **More crossings (40 against 16, and against 0.38's 24).** Three causes: siblings in a band are ordered by one mean position per block, with no crossing step that moves a block as a whole (three hub edges cross the whole gap under PROD to reach the block on the far side); the lanes in a band gap are not ordered against each other; and the edge stages redraw several chains (next point).
2. **Four edges enter a frame through its lateral side** (the hub's edges to the ingress proxies and to two more members). mermaid: none, 21 of 21 on flow-facing sides. The corridor arrives at the block's top-left, where the title chip is; flowmaid rightly refuses to strike the title and then goes round to the side.
3. **Gaps are too tight for what runs in them.** mermaid's two big gaps are about 300 px with one lane per edge at a 20 px pitch; the prototype's are 60-90 px. 9 of 20 labels have a foreign line within 12 px (mermaid 0), and two labels sit on a frame border.
4. **Upstream edges take long ways round.** Two of the three edges to the chat bot run in lanes beside the whole drawing; mermaid takes them straight up between the blocks. The chain variant does that too but scores worse today (1.4).
5. **Frame tops in a band are not level** (17 px between the two bottom blocks), and PROD keeps dead room beside its single-node rows (`compact_max` 5.09 against 4.59).
6. **The hub's six edges are six separate lines** down the corridor until B11's comb bundles them.
7. **bench-75 is compact but unreadable in the gaps**: lines 0.5 px apart, 335 crossings (mermaid 228), and in `route()` 33 defects against 1, drag jumps 46 against a ceiling of 16. The layout squeezes the blocks together and nothing sizes the gaps for 69 inter-block edges. mermaid spends 4327 px on the same drawing.

What is already better than mermaid: 0.55× its canvas area and 0.66× its edge length on the analog, labels nearer their ends (326 px against 558), no line through a title, stable block order.

## 4. Effect on other diagrams, pinned tests, risks

### 4.1 Diagrams without subgraphs

**Byte-identical**: the flat path (`scene_flat`, `layout_core(g, sizes, None, &[])`) reaches none of the new code; every addition is behind `node_cluster.is_some()` (layering) or `apath` / `border` being present (alignment, packing). Checked with the prototype binary against `fm038`, `cmp` on the SVG: architecture, back-edge, class, demo, er, fanout, fans, fans-lr, gitgraph, journey, long-labels, loops, lr, mindmap, pie, sankey, sequence, shapes, shapes-lr, shapes-td, stress, timeline, wide-labels: all identical. Class and ER build graphs without subgraphs and stay identical.

### 4.2 Diagrams with subgraphs but no border edge

- A direction block (B9): unchanged; `state.mmd` and `fanout-frames.mmd` are byte-identical in the prototype.
- A plain subgraph with no border edge, and nested ones whose layering does not change: the layers are the same, but the prototype's packing moves coordinates slightly (`subgraph.mmd`: 0.1 px; `advanced.mmd`: one bottom node shifts sideways and its labelled edge gets a longer step). The "keep Brandes-Köpf when already compact" rule of 2.1 is there to make such drawings byte-identical; it is not prototyped, so this is a goal, not a measured fact.

### 4.3 Pinned tests and gates (run against the prototype, lanes variant)

`cargo test --release --lib`: **9 of 1047 fail** (`D/test-lib.lanes.txt`):

| test | why |
|---|---|
| `layout::a_box_seats_its_loose_parts_on_its_top_layer`, `layout::a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` | state `seat_box_sources` / wrap behaviour; to be restated for the block rule |
| `render::advanced_showcase_parses_and_renders` | geometry of `advanced.mmd` (4.2) |
| `scene::a_composite_edge_to_its_own_state_drops_straight_from_the_border`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route`, `scene::note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `scene::a_static_canvas_is_its_drawing_with_even_margins` | moved positions; each needs a look: the first and third pin behaviours that must survive |
| `casing::a_short_line_keeps_at_most_one_gap`, `casing::a_line_takes_one_role_across_a_cluster_of_crossings` | crossing layout of a fixture changed |

The other subgraph tests pass unchanged, including `assert_clean_clusters`, the B9 tests, own-frame containment, `edge_picking_matches_the_painted_strokes_on_every_fixture`.

`tests/quality.rs`: **7 of 13 fail** (`D/test-quality.lanes.txt`): `quality_gate`, `defect_baseline_gate`, `channels_keep_edges_apart_and_off_chips`, `edges_keep_off_frame_borders`, `labels_cover_nothing_they_must_not`, `crossing_gaps_do_not_read_as_dashes`, `a_one_px_drag_leaves_unrelated_edges_put`. The ratchets (`CEILINGS`, `GAP_FLOORS`, `BORDER_LEN_MAX`, `DEFECT_BASELINE`, `CHANNEL_MAX`, `XGAP_MAX`) move because bench-75, dense-td and dense-lr are re-laid. Two are real regressions to fix, not to re-baseline: the least edge gap on bench-75 (static 14.7 → 11.9, routed 10.7 → 0.3) and the drag jumps (bench-75 16 → 46, dense-lr 0 → 9). The hard zeros `edge_node_x`, `shared_ports`, `arrow_on_start`, `wiggles`, `frame_side`, `node_overlap`, `node_frame`, `frame_overlap` and `own_frame_x` stay 0 on all eight inputs in both modes (`D/q.lanes.tsv`). Passing unchanged: `edge_line_order_barely_moves_the_dense_layouts`, `moving_a_node_declaration_barely_moves_dense_td`, `layout_shape_gate`, `every_shape_end_sits_on_its_outline`, `long_labels_wrap_and_dense_labels_do_not_tower`, `every_fixture_has_a_ceiling`.

README images and prose that change: `docs/img/compare/{advanced,subgraph}.png`, Subgraphs (`README.md:282-289`), How it works (`797-806`), the benchmark rows of the clustered inputs.

### 4.4 Risks

1. **Crossings.** The design makes blocks; it does not by itself reduce crossings, and the prototype raises them on every dense input. If the block transpose and laned gaps (1.6, 1.7) do not bring the analog to 24 or fewer, the package misses a must.
2. **`route()` and drag budgets.** bench-75's `route_ms` doubled to its limit and its drag jumps tripled with no change to `route()`. Tighter drawings cost the router more. The band corridors are the planned answer and are unmeasured.
3. **One layer grid for all blocks of a band.** Side-by-side blocks share layer lines, so a block with short rows next to one with tall rows inherits the taller pitch, and members cannot move to the side of their block that faces their outside links. A per-block layout (the other designers' angle) is free of both; this design accepts them to keep one pass, one finish and a `route()` that needs no stitching.
4. **Inner compactness rests on the packing**, a second coordinate pass after Brandes-Köpf. It replaces Brandes-Köpf's balance inside blocks by median sweeps; straight spines through a block border are no longer guaranteed (the alignment rule only frees chain links, but the packing starts from the least breadth, not from Brandes-Köpf's positions). The final version should start each level from the Brandes-Köpf positions clamped into the packed breadth.
5. **Block order follows declaration order on ties.** Stable under edge shuffles (tested), but a different first-mention order of nodes can give a different, equally valid stacking. Both existing order gates pass with the prototype (`edge_line_order_barely_moves_the_dense_layouts`, `moving_a_node_declaration_barely_moves_dense_td`); they gate width only, and only one moved declaration, so a wholesale reordering of node declarations is not covered.
6. **Source blocks sink, source nodes do not.** A loose source node stays on the first row of its level while a source block moves down to its consumer. Deliberate (it keeps 0.38's look for loose nodes), but it is two rules.
7. **`narrow()` and the search** were tuned on 0.38's layering. Stacking and wrapping go through the new layering as level links and the two layout tests that fail show their acceptance thresholds need a pass; `stagger_proposals` must be fenced to block intervals.
8. **State diagrams** were only spot-checked (state.mmd identical; the eight `notes/` fixtures render, and one note-link test fails). Note cards, composite `[*]` states and twin combs need their own review round.
9. **Upstream edges as lanes** is a choice made on today's numbers. With laned band gaps the chain variant may win; the switch should stay until that is measured.
10. Tall drawings: stacking trades breadth for depth (dense-lr 2302 × 3468 → 3548 × 2443 as an LR drawing; the analog's v2 is 1643 × 1459). `MAX_ASPECT` and the ink floor need re-checking over the fixtures.
