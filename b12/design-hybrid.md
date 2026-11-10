# B12 "subgraphs as blocks" — design `d-hybrid`: the smallest change to today's pipeline

Label: `d-hybrid`. Date: 2026-11-10. Source read and referenced: `b12/tree/src` (dev = 0.38.0, commit 0366084);
every `file:line` below is into that tree. Everything measured here was measured on the sanitized analog
`b12/in/infra-overview.mmd` and on the repo's corpus. Nothing from `b12/private/` is quoted.

All paths are relative to `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/`
unless they start with `/`.

Scratch artefacts of this design (all under `d-hybrid/`):

| file | what |
|---|---|
| `d-hybrid/prototype.patch` | the whole prototype: a 581-line diff of `src/layout.rs` only (no other file touched) |
| `d-hybrid/tree/` | the tree copy with the patch applied (switches are env vars, see 3.1) |
| `d-hybrid/*.proto.png` / `.svg` | prototype renders: `infra-overview` (the analog), `infra-overview-v2`, `advanced`, `subgraph`, `fanout-frames`, `state`, `bench-75`, `dense-td`, `dense-lr`, `swimlane`, `nested-cases` |
| `d-hybrid/infra-overview.proto.routed.png` | `route()` fed the prototype's own centres (what a host shows after the first drag) |
| `d-hybrid/final/metrics.tsv` | block-metrics of the prototype on the 8 inputs (`metrics/run.sh fm …`) |
| `d-hybrid/q.off*.tsv`, `d-hybrid/q.on*.tsv` | flowmaid's own quality harness, static and routed, blocks off / on, three runs each |
| `d-hybrid/test-lib.txt`, `d-hybrid/test-quality.txt` | `cargo test --lib` and `--test quality` against the prototype |
| `d-hybrid/in/*.mmd`, `d-hybrid/notes/` | two probes (swimlane, nested cases) and the state fixtures before / after |

---

## 0. The design on one page

**Idea.** Keep the one global Sugiyama pass, the wall dummies, the dummy chains, the edge stages, `route()` and
`compound.rs` exactly as they are. Change **how a node gets its rank**: instead of one longest path over all
nodes, rank *nested* — every subgraph is one unit of the level it lives in, as deep as its own inside — so a
subgraph's members take consecutive ranks and two sibling subgraphs an edge joins never share a rank. Everything
downstream already does the right thing with such ranks: walls make the frame a rectangle, `apath` keeps a long
edge outside foreign walls, the frames are wrapped round the members. Five small companions make the result
usable: an edge turned round by the block order keeps its channel, a channel keeps one side of a block, a block is
packed across the flow, the gap between two stacked frames holds its tracks between the *frames*, and a diagram
whose nested ranks equal today's ranks is left byte-identical.

**What the prototype shows** (a real code hack, `src/layout.rs` only, static scene real, `route()` unchanged):

| analog | 0.38.0 | mermaid 12.2 | prototype |
|---|---|---|---|
| arrangement | 4 frames side by side, EXT 77 % of the height | actors / EXT + GH / PROD / DR + STG | actors / EXT + GH / PROD / STG + DR |
| canvas | 2787×1225 | 2199×2358 | 2451×1323 |
| frame compactness mean / max | 6.90 / 11.43 | 3.25 / 4.59 | 3.57 / 5.72 |
| largest frame share of the flow extent | 0.767 | 0.308 | 0.307 |
| side-by-side pairs joined by an edge | 5 | 0 | 0 |
| runs through a foreign frame | 3 (2012 px) | 0 | 0 |
| crossings | 24 | 16 | **44** |
| labels struck / crowded | 0 / 8 | 0 / 0 | **4 / 10** |
| edge length | 22350 | 30835 | 23690 |

The arrangement is mermaid's, on a canvas 37 % smaller than mermaid's. With the replication edge the way the
real diagram has it (`u-mermaid/in/infra-overview-v2.mmd`) the three servers stack one under another as in the
user's mermaid screenshot (`d-hybrid/infra-overview-v2.proto.png`).

**Where it falls short — said plainly.**

1. **Crossings go up, not down.** Analog 24 → 44 (mermaid 16), dense-td 80 → 201 (mermaid 68), bench-75
   302 → 396 (mermaid 228). The blocks are right; the lines between them are not yet tidy. The proposed
   acceptance rules (`metrics/judge`) report **15 must-fails for the prototype** (0.38: 7, mermaid: 0): the
   structural ones are gone, the edge and label ones are new.
2. **`route()` loses most of it and gets slower.** It has no dummy chains: at the prototype's own centres it
   draws 33 edge / foreign-frame pairs over the six flowchart inputs (static: 2; 0.38 routed: 45) and takes
   1.4× to 2.75× as long (analog 1.81 → 4.97 ms). A position-driven channel rule for `route()` is specified in
   2.3 but **not prototyped**; without it the drag budget of PLAN invariant 1 is broken on the analog.
3. **Two hard-zero gates fail in the prototype** (`shared_ports` 2 on dense-lr static, `edge_node_x` 1 on
   dense-td routed), bench-75's static defects go 4 → 22 (edges on top of each other), 11 of the 1047 library
   tests fail (4 with the ranking alone) and 7 of the 13 tests in `tests/quality.rs` (4.5). These are defects
   to fix, not gates to move.
4. It is **not a block layout**: blocks in one band share global rows, a block's inside is ordered and placed by
   the global pass (members still drift toward far neighbours before the packing step pulls them back), a block
   whose members have no edge among themselves is one long row, and frames sharing a band are aligned only as
   well as their first rows are (analog: DR's frame starts about 10 px above STG's).

My honest reading: this is the cheapest way to get the *arrangement* (about 350 lines in `layout.rs`), and a
first stage a fuller block design could keep, because the nested ranking and the block order rule are the
per-level layering such a design needs anyway. It does not by itself reach the user's bar ("tidak main-main dan premium"). The follow-up it needs is
listed in section 5; the two items that cannot be skipped are the channel rule in `route()` and a crossing pass
that knows about bands.

---

## 1. The algorithm

### 1.1 Inputs and terms

Inputs are exactly what `layout::rank` gets today (`layout.rs:2281`): the graph `ga` (`g` plus one stand-in edge
per sub-edge, `scene.rs:3455-3519`), the rank-only groups (`sub_edge_proxies` + `grid_pairs`, then narrowing's
stackings), `node_cluster` (each node's subgraph chain, outermost first, `scene.rs:4252`) and `g.lead_nodes`.

- **Level**: the root, or one non-empty subgraph. **Unit** of a level: a node whose innermost subgraph is that
  level, or a subgraph whose parent is that level. A subgraph with no node anywhere inside is no unit (it has no
  frame today either, `scene.rs:5346`).
- **Link**: a real edge, a stand-in, or one `from × to` pair of a rank-only group; self-loops excluded.
  A link `u → v` **belongs to** the level of the lowest common ancestor of `u` and `v`, where it joins the two
  units `cu`, `cv` that hold `u` and `v` (always two different units). A link weighs 1 when it is drawn (real
  edges and stand-ins, invisible links included: they are layout statements of the author) and 0 when it is a
  rank-only pair.
- **Declaration key** of a unit: the index in `Graph::nodes` of its first node (for a subgraph: the lowest node
  index anywhere inside it). Node indices follow first mention in the source, so this is "model order".
- **Rows** of a unit: 1 for a node; for a subgraph the number of ranks its own level needs (computed bottom-up).

### 1.2 Steps

**S0. Node-level cycle breakers stay.** `rank` first marks back-edges by its two DFS walks exactly as today
(`layout.rs:2314-2371`). Links marked there are back-edges for the rest of B12 too and take no part below. (So a
cycle among nodes is broken the way it is today, and the levels only have to deal with cycles between units.)

**S1. Level graphs.** Deepest levels first, the root last: collect the level's units (sorted by declaration key)
and its links.

**S2. Block order — which links run against the flow.** Per level whose links between units close a cycle:

1. Walk A: DFS over the units, roots = the units with no link in (in declaration order), then every unit in
   declaration order; a unit's links are followed in the declaration order of their targets, then link index.
   A link into a unit still on the stack is *turned*.
2. Walk B: the same DFS with roots in plain declaration order.
3. The walk that turns fewer drawn edges is taken, walk A on a tie.
4. Safety net: let `M` be the least number of drawn edges any order of this level's units turns (exact, by
   dynamic programming over subsets, for a level of at most 12 units; ties to the lexicographically smallest
   sequence of declaration keys). If the taken walk turns more than `2·M + 1`, the minimal order replaces it.
   A level of more than 12 units keeps its walk.

Why this rule and not simply "the fewest turned edges": on the analog the minimum is 3 turned edges with PROD
*above* External services — the actors' three edges then run down past PROD and `DNS → ingress` runs back up.
The walk turns 5 and gives actors / EXT / servers, which is what the author wrote top-down and what mermaid
draws. On dense-td the walk enters the wrong block first (through the lone edge `n45 → g8`) and turns 19 of 33;
the minimum is 4 (`n45 n46 g1 g5 g8`, mermaid's order too) and the safety net takes it. So: the author's order
unless it costs more than twice the lines against the flow. Links turned by the block order, as the prototype
counts them (node-level cycle breakers of S0 not included): analog 5 (minimum 3), v2 5 (minimum 4), bench-75 3
(= minimum), dense-lr 3 (minimum 2), dense-td 2 (walk alone: 19), fork-twins 5 (minimum 2), advanced 0,
subgraph 0, nested-composite 0, routed-around 0, spread-face 0.

The rule does not read the order of the *edge* lines, only node declaration order, so shuffling edge lines
changes nothing when nodes are declared up front (as in the three quality fixtures) — see 4.4 for what was and
was not measured.

**S3. Lanes, not blocks (the rule for "better left as today").** If `M` is more than a third of the level's
drawn links between units, no stacking order is good: whatever goes on top, a third of the arrows point
backwards. The level's child subgraphs then stay *transparent*: their members become units of this level (their
inner ranks are dropped, their inner links become links of this level), which is exactly today's ranking for
that level; contiguity across the flow keeps their frames rectangles as today. Constant: `LANE_SHARE = 1/3`.
Example: `a1 → b1 → a2 → b2 → a3 → b3` over two subgraphs has `M` = 2 of 5 (40 %) and stays two lanes side by
side with every arrow forward (`d-hybrid/out/swimlane.off.png` is that drawing; `d-hybrid/swimlane.proto.png`
is what blocks would make of it: tidy, but two of five arrows point up). On the corpus the share is far below
the line everywhere (analog 3 / 21 = 14 %, dense-td 4 / 33 = 12 %, bench-75 at most 3 / 69), so **this rule is
not exercised by any corpus input and is not in the prototype**; it is a guard for process-style diagrams and
its constant needs a generated sweep before it is pinned.

**S4. Rows of the level: longest path with unit heights.** Constraints: for every link `cu → cv` not turned,
`start(cv) ≥ start(cu) + rows(cu)`; for a turned link the same with the ends swapped. Solve by longest path
(Kahn; the constraint graph is acyclic after S2). Then two adjustments:

- *No interleaving.* While two sibling **subgraphs** `A`, `B` of the level overlap in rows without one holding
  all of the other's (`start(A) < start(B) < end(A) < end(B)`), add `start(B) ≥ end(A)` and solve again. The
  added constraint always points from the earlier start to the later one, so it cannot close a cycle; at most
  one round per pair. (Free nodes may sit beside any rows of a block: that is what keeps a chain of plain nodes
  next to a tall block compact.)
- *A source block sits next to what it feeds.* A subgraph with no constraint into it starts at
  `min(start of its successors) − rows` when that is later than 0. Plain nodes are not moved (flat behaviour:
  sources on the first rank). On the analog this puts Code hosting beside External services, as mermaid has
  it; without it GH sits beside the actors and its four edges each pass one more band (area +18 %, edge length
  +18 % on the analog: variants `p2c` against `p2tc` in `d-hybrid/out/`).

`rows(level) = max(start + rows)`. A member with no link inside its block lands on the block's first row
(it is a source of its level) whatever links it has to the outside: this is the "a member with only outward
edges takes the block's first rank" rule, and it replaces `seat_box_sources` (`layout.rs:2497`) wherever B12 is
in force.

**S5. Absolute ranks.** Outermost first: `rank(node) = Σ start of its enclosing subgraphs + its own start`.
A rank-only hub gets `max(rank of its unturned sources) + 1`. Then step 4 of today's `rank` (lead note cards one
layer before their target, `layout.rs:2406-2432`) runs unchanged.

**S6. The trigger: is this diagram already block-shaped?** `rank` computes today's ranking as well (it has to
run its DFS anyway; `seat_box_sources` is the only extra work). If S5's ranks equal today's ranks and S2 turned
no link, **the whole pipeline continues exactly as 0.38 — none of S7-S10 runs — and the output is
byte-identical.** Measured with the prototype's switches (rank change only, no companion): `advanced`,
`subgraph`, `state`, `fanout-frames` and the state fixtures `near-drag`, `spread-bottom`, `straight-link`,
`wide-frames` are byte-identical to 0.38; `fork-twins`, `nested-composite`, `routed-around`, `spread-face`,
the analog, bench-75, dense-lr and dense-td change. (The prototype does not implement the trigger as a switch;
its companions run on every clustered input, which is why its `advanced` and `fanout-frames` differ, see 3.4.)

**S7. A turned link keeps its channel.** In `place`, an edge spanning more than one layer gets a dummy chain
unless it is a back-edge (`layout.rs:3108`). A link turned by S2 is a back-edge for every other purpose (it is
excluded from `NodeLinks`, staggering, hub needs) but **keeps its chain**, built from the lower layer to the
higher one and stored in from → to order as the code already does for a descending edge (`3071`,
`3129-3131`). The scene then draws it up its channel: out of the source's upstream side, beside the blocks of
the bands between, into the target's downstream side (`scene_from_layout` already handles `a_bottom = false`,
`scene.rs:3279-3285`). `back_lanes` skips an edge that has a chain and counts its skeleton among the forward
ones (`layout.rs:4448`, `4459`). A turned link between adjacent layers has no chain and is drawn
across the gap as an adjacent back-edge is today. Lanes round the outside (today's back-edge drawing) were
tried for turned links (`FMB12_LANES=1`): on the analog the five lanes run round whole frames and the picture
is worse (`d-hybrid/out/infra-overview.p2tcl.png`); numbers in 3.3.

**S8. A channel keeps its side of a block.** `arrange` (`layout.rs:5534`) orders the units of one layer by key:
a subgraph by one diagram-wide key (`cluster_keys`, `5487`), a free dummy by its own position in *that* layer.
So a channel passing a three-row block can sit left of it on one row and right of it on the next — the edge then
steps across the block in the gap between two of its rows (this is where 0.38's runs through foreign frames
come from, and v2 of the analog drew `R2 → STG` 1030 px through PROD that way). New: the dummies of one edge
that have the same cluster path on consecutive layers form a **run**, and a run has one key — the mean of its
dummies' keys — exactly as a subgraph's members share one. Deterministic, O(dummies) per `cluster_keys` call.
Measured: v2 runs through a foreign frame 1 → 0, dense-lr 2 → 1, crossings v2 37 → 34, dense-lr 55 → 41; but
analog 43 → 44 and bench-75 365 → 396, and the analog got 12 % wider. It removes the defect it is aimed at; it
is not a crossing optimiser.

**S9. A block is packed across the flow.** Brandes-Köpf aligns each member with its median neighbour wherever
that is, so a block's members spread toward the blocks they link to (PROD in the first prototype render was
920 px wide for 5 nodes, `d-hybrid/out/infra-overview.p1.png`). `compact_breadth` (`layout.rs:5201`) already
exists for narrowing: pack to the least breadth the order allows, then re-centre on neighbours within it, walls
moving as straight units. B12 runs it on the first placement too (`Opts::compact`, `1561-1564`). Analog:
compactness 6.01 / 12.60 → 3.57 / 5.72, width 3089 → 2522, edge length 28657 → 23486.

**S10. Tracks between two stacked frames lie between the frames.** `track_need` (`layout.rs:4002-4051`) sizes a
gap from the row of boxes above to the slot below (`4166-4169`). Between two bands the usable room is smaller:
the upper frame's pad and the lower frame's pad and title strip are not track room. Where a frame closes on
layer `li − 1` **and** a frame opens on layer `li`: `lcoord[li] ≥ lcoord[li−1] + reach_of(li−1, 1) +
track_need[li] + reach_of(li, 0)` (`reach_of`, `3608`). (The prototype applies it wherever the gap has tracks,
also below a plain node; that is the 14 px by which its `fanout-frames` and `advanced` differ from 0.38 and is
not the rule proposed.)

**Frames, sub-edges, labels: unchanged.** Frames are `cluster_raw_boxes` of the members, widened over inner
channels as today (`scene.rs:3535-3599`); a sub-edge is its stand-in cut at the frame; a long edge's label is
its chain's middle dummy; a short labelled edge grows its gap.

### 1.3 Determinism and tie-breaks

Every choice is by integer comparison on declaration keys, link indices and counts: S2's walks (unit order, then
target order, then link index), "fewer turned" with walk A on a tie, the subset DP (integer costs; first
minimal `j` in key order when reconstructing), S4's pair search (first pair in key order). No hash-map iteration
reaches the output (units in sorted `Vec`s; S8 sums over a `Vec` indexed by run id). The prototype's SVGs are
byte-identical across runs and the harness's `deterministic` column is 1 on every input in both modes.

### 1.4 Rules per case

| case | rule | status |
|---|---|---|
| **Nested subgraphs** | S1-S5 are recursive: an inner subgraph is one unit of its parent's level and as many rows deep as its inside; a parent's rows are computed from its children's. An edge from outside to a node two levels down belongs to the level of the common ancestor; its dummies take the progressive cluster path they take today (`layout.rs:3292-3341`), so it enters each frame through the flow-facing side and descends inside. | prototype: `d-hybrid/nested-cases.proto.png`, dense-td / dense-lr (7 and 6 nested frames) |
| **Node → whole subgraph** (`R2 --> STG`) | the stand-in `R2 → first entry member` and the proxy group are links between R2's unit and STG at their common level: STG starts below the whole of R2's unit. Drawn as today: the stand-in's channel cut at the frame, the head on the flow-facing side (`scene.rs:3608-3743`). | prototype (three such edges in the analog) |
| **Subgraph → subgraph** | one link between the two units; if S2 turns it, it is drawn up its channel (S7) and still cut at both frames. | prototype (`I --> X` in nested-cases; state composites) |
| **Member → its own subgraph** (`o2 --> O`) | the stand-in is `member → entry member`, a link *inside* the block: it ranks like any inner edge and is drawn whole, as today (`own_box_targets`, H10). If the member is itself an entry member's descendant the link closes an inner cycle and `rank`'s node-level DFS (S0) breaks it as today. | prototype (nested-cases: drawn to `outer one`) |
| **Subgraph → its own member** | `sub_edge_proxies` gives nothing (`scene.rs:3994`); drawn as `own_box_stub`. Unchanged. | unchanged |
| **Cycle between blocks** | S2. The turned links run up their channels (S7); the blocks keep their bands. A two-block cycle `A ⇄ B` with `w(A→B) ≥ w(B→A)` in walk order puts A on top; with more than a third of the links turned whatever the order, S3 keeps the level as lanes. | prototype (analog: 5 turned; fork-twins: 5) |
| **A hub linked to many blocks** (the DNS node) | nothing special in ranking: the hub's block comes first by S2, each out-edge has its own channel, S8 keeps each channel on one side of the blocks it passes. Port order on the hub and the fan's crossings are left to the existing stages — and that is where the analog's extra crossings are (3.5). | prototype; **weak** |
| **Empty subgraph** | no node inside → no unit, no rows, no frame, sub-edges to it skipped: all as today (`scene.rs:130-134`). A subgraph holding only empty subgraphs is empty too. | unchanged (`empty_and_nested_empty_subgraphs_stay_finite` passes) |
| **Subgraph with one member and no links** | a unit of 1 row on its level's first row (or wherever its outside links put it). | prototype (`Empty-ish` in nested-cases) |
| **Own `direction`, no border edge** (B9's blocks) | `compound::split` runs first (`scene.rs:3058`) and takes the block out; the rest sees a proxy node, which is a plain unit. Each part goes through `scene_labelled` again, so a block's inside is ranked nested too. B9's rule, parts, proxies, `join`, `route`, `route_partial` are untouched. | `state.mmd` byte-identical; `compound.rs` tests pass |
| **Own `direction`, with a border edge** | as today and as mermaid: the direction is dropped (`compound.rs:182`) and the subgraph is a unit of the surrounding flow. | `subgraph.mmd` byte-identical under the trigger |
| **LR / RL / BT** | ranking is direction-free; everything direction-specific (`lextra`, `reach`, title strip across the breadth in LR/RL, BT's highest layer as screen top) is in `place` and unchanged. S10 uses `reach_of`, which already is per direction. | prototype: bench-75, dense-lr (LR), fork-twins (LR state). BT and RL **not run** |
| **State diagrams: composites** | a composite is a subgraph; transitions to a composite are sub-edges; its `[*]` nodes are members. Same rules. | prototype: 8 fixtures in `d-hybrid/notes/` |
| **State: notes** | a note card is a member of the composite it was written in; `note left of` cards are lead nodes and keep step 4 of `rank` (one layer before their target, floor at the hub that ranks their box). A note's link never gets a lane (`layout.rs:4439-4443`) and never a turned channel: a link with a card end is not turned by S2 — it is left out of the level graph and the card is ranked after its level by step 4 / beside its target. | prototype runs step 4 unchanged; "never turned" is **not in the prototype** |
| **State: fork / join bars** | `fit_bars` (`scene.rs:3063`) runs before the layout and is untouched; a bar is a node. Bars whose branches lie in one block are unaffected; a bar whose branches are spread over two blocks will now be stretched across blocks stacked along the flow — rare, and `fork_join_branches_drop_straight_and_drags_agree` must be re-read for it. | fork-twins rendered (bar inside one composite); cross-block bar **not tried** |
| **Invisible links** (`~~~`) | links of weight 1 for S2 (the author asked for that order); never a chain, as today (`layout.rs:3108`), also when turned. | advanced (`Users ~~~ Mon`) unchanged |
| **Very large drawings: the router's 400-edge cap** | unchanged: above `ROUTE_MAX_EDGES` the ortho stage and the router leave the drawing as built (`ortho.rs:180-185`); B12 changes ranks and channels only, which are built before that. `track_need` and S10 are already skipped above the cap (`layout.rs:4004`). | not run above the cap |
| **The layout search budget** | nested ranking is O(V + E) per level plus the subset DP (≤ 4096 × 144 steps for a 12-unit level, only on a cyclic level); it runs inside `rank`, i.e. once per candidate layering of `narrow` (stacking and wrap groups are rank-only links like any other and become constraints of their level). Above `SEARCH_MAX_ELEMENTS` (600) the search is off as today and S9 does not run (it is `place` work the budget gates); S7, S8, S10 are part of the one placement. Above `WALK_MAX_ELEMENTS` the subset DP is skipped too (walk only). | bench-75 (150 elements) |

---

## 2. Code plan

### 2.1 `src/layout.rs` (all of the static design)

| # | where | change | size |
|---|---|---|---|
| L1 | `struct Ranking`, `2267-2274` | new field `up: Vec<bool>` (per ranking link: turned by the block order) and `blocked: bool` (S6: nested ranks differ from the flat ones) | 6 |
| L2 | `rank`, `2281-2440`: after step 2 (`2373-2398`), replacing the unconditional step 3 (`2400-2404`) | keep step 3's result as `flat`; call new `block_rank(g, nc, &adj, n, &mut back, &mut up, &mut layer)`; set `blocked = layer != flat ‖ any up`; when not `blocked`, restore `flat` and clear `up` (0.38's ranking, bit for bit) | 15 |
| L3 | new `fn block_rank` next to `seat_box_sources` (`2497`) | S1-S5: level graphs, two walks, subset DP, lane rule, longest path with heights, band rule, source blocks, absolute ranks, hub layers. The prototype's version is 290 lines without S3 and without the note-card exclusion | ≈ 330 |
| L4 | `layout_core`, `1561-1564` | `compact: rk.blocked` for the first placement (S9); nothing else in the search changes | 2 |
| L5 | `place`, chain rule `3104-3112` | `back[ei] && !rk.up[ei]` gets no chain (S7) | 3 |
| L6 | `place`, after `apath` (`3302-3341`) | build `runs` (S8) when `rk.blocked` | 20 |
| L7 | `cluster_keys`, `5487-5506`, and its five callers (`3359`, `3383`, `3426`, `3434`, `4238`) | extra `runs: &[usize]` argument; a run's dummies take the mean key | 25 |
| L8 | `layer_lines`, `4166-4169` | S10, only when `rk.blocked` and a frame closes above and opens below | 8 |
| L9 | `back_lanes`, `4448` and `4459` | an edge with a chain is no lane and is a forward skeleton | 8 |
| L10 | `tighten_sources` (`2840`), `stagger_proposals` (`2877`), `stack_candidates` (`2157`) | read-only check: they move single nodes or add groups under `NodeLinks`; when `rk.blocked`, a move that takes a member out of its block's rows is refused (one `cluster_span` test per proposal) | 15 |

`seat_box_sources` stays (it is the flat ranking's step 3 and the reference for S6). No public item changes:
`layout_clustered`, `layout_sized`, `LayoutResult`, `Placed` keep their signatures (H15).

### 2.2 `src/scene.rs`, static path

Nothing has to change for the prototype's result: `static_lanes` (`3097`) and `scene_from_layout` (`3159`)
already draw a descending chain. Two small things for the real package:

- `scene_from_layout`, the label of a channel (`3292-3301`): a turned edge's label rides the chain's middle
  dummy like any other; fine as is.
- `scene_clustered`, frame widening (`3535-3599`): unchanged; `groups[ei]` already says which box each channel
  point lies in, for a descending chain too.

### 2.3 `route()`, `route_sized()`, `route_partial()` — what of the block logic runs there

`route()` gets the graph, centres and sizes (H2). Of the block logic:

| part | in `route()`? | why |
|---|---|---|
| nested ranking, block order (S1-S5) | **no** | positions are the host's; nothing is ranked |
| the trigger (S6) | no | not needed: `route()` behaves the same either way |
| compact frames | **yes, for free** | frames are `cluster_raw_boxes` of the members (`scene.rs:5346`, `route_clusters` `5389`) |
| which edges run against the flow | **yes, by geometry, as today** | `route_backs` (`scene.rs:8097`): target wholly upstream of the source. On the scene's own centres that is exactly the set S2 turned plus the node-level back-edges |
| channels round foreign blocks (the static dummy chains) | **new: R1 below** | today only `cross_cluster_route` (`8400`), a 4-point path that ignores what lies between |
| a channel keeping one side (S8), packing (S9), track room (S10) | no | layout concerns |

**R1 — block lanes in `route_built` (`scene.rs:4435`), not prototyped.** For every forward edge whose two ends
have different top-level boxes (`top_of`, the test already at `4638`) and for every back-edge `route_backs`
would draw directly: take the band between the two ends along the flow; collect the breadth spans of the
frames (and top-level nodes) that reach into the band and hold neither end; if the straight drop from the
source's port to the target's port meets none, keep today's path; else plan a lane with `lanes::plan`
(`lanes.rs:696`) — the `Want` it takes (`lanes.rs:659`: band, obstacle spans, `blocked`, clearance, the ends'
port spans) is precisely this situation, and it already nests lanes that share a band and judges crossings
against the other skeletons. The lane is drawn out of the source's flow side, across in the first free gap,
along the lane, across in the last free gap, into the target's flow side (a new `corridor_geometry` beside
`lane_geometry`; rounded as the router rounds, so `ortho` accepts it as "on the axes already",
`ortho.rs:716`). Obstacles are frames, so the cost is O(edges across blocks × top-level frames), no search.
`route_lane_clusters` (`5404`) is not involved: a block lane lies outside every frame it passes.

Expected effect, **an estimate, not a measurement**: the edges the router has to repair today because their Z
meets a frame get a clean path first, so `route_ms` should come back toward 0.38's (the excess measured below is
router work), and the routed edge / foreign-frame pairs toward the static ones.

**`route_sized`**: same function (`4368-4384`); R1 reads sizes for the port spans only.

**`route_partial`** (`4889-5149`): no new state. `clusters_line_up` (`5277`) holds for a block layout as for any
other (frames hold their members and no foreign centre; measured `node_frame` 0, `frame_overlap` 0 on every
input in both modes). A frame not holding a moved node keeps its base box; edges on a moved node or a regrown
frame are redrawn by the same builders as `route()` (so R1 applies to them), the rest keep their static
channels. `PARTIAL_ROUTE_SHARE` (`4833`) is unchanged. What to watch: dragging one member of a block regrows the
block's frame, and every channel that passes that block is "near the change"; on the analog a member of PROD
touches up to 8 passing channels. `DRAG_JUMPS_MAX` (`tests/quality.rs:967`) is where this shows; not measured
here.

**Scene ≠ route at the scene's own centres** stays true, as it is today (understand-flowmaid 4.3). B12 makes the
difference more visible because the static drawing has more long channels. R1 is what narrows it.

### 2.4 Hit-testing, fold, emit, compound

- `hit_test` / `node_at` / `edge_at` / `cluster_at` / `hits_in_rect` read the `Scene` only. They need frames
  that nest and siblings that do not overlap; both hold (above). `edge_picking_matches_the_painted_strokes_on_every_fixture`
  (`scene.rs:10822`) passes on the prototype.
- `fold::scene_compact` refuses graphs with subgraphs (`fold.rs:246-248`): unchanged. `to_mermaid` reads the
  `Graph` only: unchanged. `json` export: no new field (`Ranking` is private).
- **`compound.rs` becomes nothing new.** It stays B9: a subgraph with its own direction and no border edge.
  This is the point where the hybrid differs most from a block layout: it does *not* generalise `Split` / `Part`
  / proxies to subgraphs with border edges. What B9 gains: each part and the rest are ranked nested, so a B9
  block inside a bordered subgraph sits in that subgraph's rows as one unit.

### 2.5 Cost (ms; prototype, release build, best of three, this 4-CPU box under load)

| input | `scene()` 0.38 → prototype | `route()` at the scene's centres, 0.38 → prototype (no R1) |
|---|---|---|
| analog (22 nodes, 27 edges) | 6.30 → 8.40 (×1.33) | 1.81 → **4.97 (×2.75)** |
| bench-75 | 33.84 → 39.03 (×1.15) | 7.43 → 10.30 (×1.39) |
| dense-td | 22.60 → 32.71 (×1.45) | 6.25 → 8.75 (×1.40) |
| dense-lr | 17.18 → 15.59 (×0.91) | 3.70 → 6.63 (×1.79) |
| advanced | 0.75 → 1.06 (×1.41) | 0.49 → 0.69 (×1.41) |
| subgraph | 0.13 → 0.19 (×1.46) | 0.06 → 0.06 |
| fanout-frames | 1.97 → 2.13 | 0.27 → 0.20 |

Reading:

- Static is within 2× of 0.38's own numbers everywhere. Against the PLAN baselines `subgraph.mmd` (limit
  0.16 ms) is over in the prototype at 0.19. The prototype reads environment variables in inner loops and runs
  S8-S10 on every clustered input; with the trigger (S6) `advanced`, `subgraph`, `fanout-frames` and `state`
  take 0.38's path plus one `block_rank` (linear, a few µs at their size). `subgraph.mmd` has no headroom, so
  that addition must be measured with `perf.sh` before anything else.
- The static growth on the analog and dense-td is mostly more dummies: every edge between two bands that are
  not neighbours has a chain now, and turned edges have one too.
- **`route()` is the problem.** Nothing in `route()` changed; it is slower because block positions give it more
  edges whose direct path meets a frame, and those go to the router. Against the budget
  max(1.25 × baseline, baseline + 1.0 ms): the analog (limit 2.81) fails; dense-lr and dense-td pass against
  the PLAN baselines (8.03, 9.20) and fail against 0.38's own numbers ×1.25; bench-75 passes. `drag_ms` and
  `route_partial` were not measured. R1 is therefore part of the package, not an option.

Expected for the real package: `scene()` analog ≈ 8 ms, dense-td ≈ 30 ms, bench-75 ≈ 38 ms (as measured, minus
the env lookups); `route()` unknown until R1 exists — target: within 1.25× of 0.38.

---

## 3. Prototype

### 3.1 What it is

A full code hack: `d-hybrid/prototype.patch` applied to a copy of the tree, built in release, driven through the
normal CLI. It implements S0-S2, S4, S5, S7, S8, S9, S10 (in the broad form noted there) in `src/layout.rs`;
`scene.rs`, `route()`, the edge stages and `compound.rs` are untouched. Not implemented: S3 (lane rule), S6 as
a switch (shown by running with the companions off), R1 (`route()` channels), the note-card exclusion.

Switches (env): `FMB12=0` → 0.38's ranking (verified byte-identical to `fm038` on the analog and to the
baseline table on all 8 inputs); `FMB12_COMPACT=1` (S9), `FMB12_TIGHT=1` (source blocks, S4),
`FMB12_ORDER=half` (S2's safety net; `fas` = always the minimum, `model` = walk A only), `FMB12_LANES=1`
(turned links as lanes instead of S7), `FMB12_NORUN=1`, `FMB12_NOGAP=1`, `FMB12_NOBAND=1`, `FMB12_DUMP=1`.
**Every number called "prototype" below is `FMB12_COMPACT=1 FMB12_TIGHT=1 FMB12_ORDER=half`.**

### 3.2 Pictures

| picture | what to look at |
|---|---|
| `d-hybrid/infra-overview.proto.png` | **the analog.** Compare with `metrics/mermaid-svg/infra-overview.mermaid.png` and `infra-overview.fm038.png` |
| `d-hybrid/infra-overview-v2.proto.png` | the analog with the replication edge as in the real diagram: EXT + GH, then the three servers one under another |
| `d-hybrid/infra-overview.proto.routed.png` | `route()` at the prototype's centres: frames stay compact, channels are gone |
| `d-hybrid/advanced.proto.png`, `subgraph.proto.png`, `fanout-frames.proto.png`, `state.proto.png` | the small examples (prototype without the trigger; with it all four are byte-identical to 0.38) |
| `d-hybrid/bench-75.proto.png`, `dense-td.proto.png`, `dense-lr.proto.png` | the dense fixtures: five zones as five columns; dense-td's three bands |
| `d-hybrid/nested-cases.proto.png` | nested blocks, node → block, block → block, member → own block, a hub into three blocks, a one-member block |
| `d-hybrid/swimlane.proto.png` against `d-hybrid/out/swimlane.off.png` | what S3 is about |
| `d-hybrid/notes/fork-twins.png` against `fork-twins.038.png` | an LR state diagram with nested composites, notes and a fork bar |
| `d-hybrid/out/infra-overview.p1.png` → `p1tc` → `p2tc` → `d-hybrid/infra-overview.proto.png` | the steps: ranking only; + packing and source blocks; + track room; + channel sides |

### 3.3 Numbers on the analog (block-metrics, `metrics/blockmetrics`)

| metric | 0.38.0 | mermaid 12.2 | prototype | must | verdict |
|---|---|---|---|---|---|
| canvas | 2787×1225 | 2199×2358 | 2451×1323 | | |
| `area` | 3 414 075 | 5 184 964 | 3 242 673 | ≤ mermaid's | pass (also below 0.38) |
| `compact_mean` / `compact_max` | 6.90 / 11.43 | 3.25 / 4.59 | 3.57 / 5.72 | ≤ 4.0 / ≤ 5.5 | pass / **fail by 0.22** (PROD) |
| `span_max` | 0.767 | 0.308 | 0.307 | ≤ 0.40 | pass (target 0.31 met) |
| `sib_side` / `sib_stacked` | 6 / 4 | 2 / 8 | 2 / 8 | | as mermaid |
| `sib_side_linked` | 5 | 0 | 0 | = 0 | pass |
| `sib_stagger` | 3 | 0 | 1 | = 0 | **fail** (DR and STG: frame tops 10 px apart) |
| `xframe_runs` / `xframe_len` | 3 / 2012 | 0 / 0 | 0 / 0 | = 0 | pass |
| `crossings` | 24 | 16 | 44 | ≤ 24 | **fail** |
| `edge_len` | 22350 | 30835 | 23690 | ≤ mermaid's | pass |
| `straight_max` | 1175 | 1236 | 1766 | ≤ 1240 | **fail** |
| `lanes_max` / `lane_gap_min` | 6 / 9.3 | 6 / 12.0 | 6 / 7.8 | ≥ 8 | **fail by 0.2** |
| `lbl_off_mean` / `lbl_off_max` | 0.198 / 0.332 | 0.130 / 0.329 | 0.152 / 0.344 | ≤ 0.20 / ≤ 0.35 | pass |
| `lbl_endpx_mean` | 309 | 558 | 389 | | |
| `lbl_struck` | 0 | 0 | 4 | = 0 | **fail** |
| `lbl_crowd` / `lbl_gap_min` | 8 / 2.5 | 0 / 20.0 | 10 / 0.0 | ≤ 4 / ≥ 2.5 | **fail** |
| `node_hits`, `frame_overlap`, `frame_foreign_nodes`, `warn` | 0 | 0 | 0 | = 0 | pass |

Variants on the analog (same tool), to show what each choice costs:

| variant | canvas | compact mean / max | crossings | edge_len | struck / crowd | note |
|---|---|---|---|---|---|---|
| ranking only (S1-S5, S7) | 3089×1331 | 5.98 / 12.47 | 40 | 27236 | 2 / 6 | blocks stacked, frames loose across the flow (`out/infra-overview.p1.png`) |
| + packing, + track room (S9, S10) | 2522×1338 | 3.57 / 5.72 | 35 | 23486 | 5 / 9 | `out/infra-overview.p2c.png` |
| + source blocks (S4) | 2188×1310 | 3.57 / 5.72 | 43 | 19869 | 4 / 8 | the tidiest analog of the series (`out/infra-overview.p2tc.png`) |
| + channel sides (S8) = **prototype** | 2451×1323 | 3.57 / 5.72 | 44 | 23690 | 4 / 10 | wider on the analog; fixes v2 and dense-lr |
| as the third row, turned links as lanes instead of channels | 2238×1306 | 3.43 / 5.09 | 36 | 20054 | 1 / 9 | fewer crossings, but five lanes round whole frames (`out/infra-overview.p2tcl.png`) |
| as the third row, block order = always the minimum (PROD above EXT) | 1942×1347 | 3.62 / 5.33 | 30 | 18959 | 1 / 8 | better numbers, **not mermaid's arrangement**; 1 run through a frame (`out/infra-overview.o2.png`) |

The last row is the honest price of "mermaid's arrangement on the analog": 14 crossings and 20 % of the edge
length. It is S2's reason for a threshold instead of a blanket rule.

### 3.4 Numbers on the other inputs

| input | engine | W×H | compact mean / max | span_max | side / stagger / side_linked | xframe runs | crossings | edge_len | struck / crowd |
|---|---|---|---|---|---|---|---|---|---|
| advanced | 0.38 | 596×1236 | 5.75 / 8.28 | 0.377 | 0 / 0 / 0 | 0 | 0 | 2346 | 0 / 0 |
| advanced | mermaid | 906×1484 | 5.33 / 8.24 | 0.428 | 0 / 0 / 0 | 0 | 0 | 5024 | 0 / 0 |
| advanced | prototype | 624×1258 | 5.74 / 8.24 | 0.371 | 0 / 0 / 0 | 0 | 0 | 2828 | 0 / 0 |
| bench-75 | 0.38 | 2222×1965 | 35.88 / 52.03 | 0.464 | 4 / 2 / 4 | 4 | 302 | 80453 | 0 / 0 |
| bench-75 | mermaid | 4327×1058 | 5.33 / 9.88 | 0.161 | 0 / 0 / 0 | 0 | 228 | 106034 | 0 / 0 |
| bench-75 | prototype | 1849×971 | 6.62 / 16.25 | 0.192 | 0 / 0 / 0 | 0 | **396** | 51606 | 0 / 0 |
| dense-lr | 0.38 | 2302×3468 | 8.76 / 11.78 | 0.603 | 7 / 4 / 5 | 8 | 46 | 38537 | 5 / 10 |
| dense-lr | mermaid | 4959×3858 | 3.74 / 6.92 | 0.195 | 1 / 0 / 0 | 0 | 10 | 78383 | 0 / 0 |
| dense-lr | prototype | 3677×2557 | 4.40 / 6.70 | 0.199 | 1 / 1 / 0 | 1 | 41 | 45343 | **6 / 11** |
| dense-td | 0.38 | 4249×1806 | 12.45 / 24.66 | 0.795 | 3 / 3 / 3 | 12 | 80 | 63286 | 1 / 12 |
| dense-td | mermaid | 4131×4378 | 8.77 / 15.12 | 0.308 | 0 / 0 / 0 | 0 | 68 | 142183 | 0 / 2 |
| dense-td | prototype | 3855×1929 | 7.81 / 12.00 | 0.375 | 1 / 0 / 0 | 1 | **201** | 92105 | **2** / 10 |
| fanout-frames | 0.38 | 566×2311 | 2.88 / 3.37 | 0.462 | 15 / 0 / 0 | 0 | 0 | 4564 | 0 / 0 |
| fanout-frames | prototype | 580×2311 | 2.88 / 3.37 | 0.451 | 15 / 0 / 0 | 0 | 0 | 4568 | 0 / 0 |
| state | 0.38 = prototype | 535×873 | 5.35 / 5.35 | 0.105 | – | 0 | 0 | 1674 | 0 / 0 |
| subgraph | 0.38 | 382×758 | 6.28 / 7.49 | 0.562 | – | 0 | 0 | 632 | 0 / 0 |
| subgraph | prototype | 382×758 | 6.28 / 7.49 | 0.562 | – | 0 | 0 | 664 | 0 / 0 |

`metrics/judge d-hybrid/final/metrics.tsv`: 138 checks, **15 must-fails**, 12 targets missed. The must-fails:
analog `crossings` (twice: the absolute and the relative rule), `lbl_crowd` (twice), `lbl_struck`,
`lbl_gap_min`, `lane_gap_min`, `straight_max`, `sib_stagger`, `compact_max`; bench-75 `crossings`; dense-td
`crossings`, `lbl_struck`; dense-lr `lbl_struck`, `lbl_crowd`. For comparison `judge` gives 0.38 seven
must-fails (all structural, all on the analog) and mermaid none.

Structure improves on every input that changes: bench-75's frames go from 36× to 6.6× their content and from
"4 side by side" to five stacked columns on a canvas 59 % smaller; dense-td's largest frame from 80 % to 38 %
of the flow extent; runs through foreign frames over all eight inputs 27 → 2. What gets worse on every dense
input is the edge work: crossings, and lines lying on or close to each other.

flowmaid's own harness (`examples/quality.rs`, six flowchart inputs, static / routed; `d-hybrid/q.*.tsv`):

| metric (total) | static 0.38 → prototype | routed 0.38 → prototype |
|---|---|---|
| edge / foreign-frame pairs | 46 → **2** | 45 → 33 |
| drawn crossings (`x_drawn`) | 452 → 683 | 520 → 780 |
| `defects` | 50 → 31 | 49 → **82** |
| edges overlapping (`overlap_pairs`, px) | 4 (174) → 27 (3918) | 4 (318) → 48 (5240) |
| `shared_ports` (hard zero) | 0 → **2** (dense-lr) | 0 → 0 |
| `edge_node_x` (hard zero) | 0 → 0 | 0 → **1** (dense-td) |
| `lbl_edge` | 6 → 12 | 5 → 11 |
| `frame_overlap`, `node_frame`, `node_overlap`, `wiggles`, `frame_side`, `own_frame_x` | 0 → 0 | 0 → 0 |
| `lat_wiggles` / `lat_backtrack` px | 16 / 2198 → 44 / 12968 | 4 / 405 → 23 / 2882 |
| `deterministic` | 1 | 1 |

### 3.5 Looked at next to mermaid: what is still worse, and why

Looking at `d-hybrid/infra-overview.proto.png` beside `metrics/mermaid-svg/infra-overview.mermaid.png` (and the
user's mermaid screenshot, whose arrangement `infra-overview-v2.proto.png` matches):

1. **The lines between bands are a tangle where mermaid's are a comb.** In the gap between External services
   and PROD the prototype stacks about nine long horizontal runs with three label rows; three lines to and
   from the standby server run the whole width of the gap to pass PROD on its far side. mermaid's edges leave the hub straight down and
   turn once. Cause: the order of blocks inside a band and of channels beside them is still found by the global
   median sweeps with one mean key per subgraph (`cluster_keys`); nothing weighs "DR's three edges come from
   the left end of EXT, so DR belongs on the left". mermaid has DR left of STG, the prototype STG left of DR.
   A band-level ordering step is the missing piece (section 5, F2).
2. **Edges enter boxes from the side.** `a.example.org` meets the DNS node on its left, `A 203.0.113.177`
   the ingress proxy on its left, `b.dev …` the DEV node on its left. That is the ports stage reacting to a
   first channel point far to one side; 0.38 does the same in places (its `example.org proxied`). In a block
   layout every inter-block edge should use the flow-facing sides, as mermaid's 21 of 21 do.
3. **Labels in the corridor.** 4 chips have a foreign line through them and 10 a line within 12 px. The label
   of a long edge is its chain's middle dummy, which in a block layout is beside a block, in the corridor where
   the other channels run; the ortho stage then moves lines and the chips no longer have their slot. mermaid
   gives labels a row of their own between bands (and pays 300 px gaps for it).
4. **Three lines run 1700 px** (`straight_max` 1766 against 1236): the detours round PROD's far side in 1.
5. **PROD's frame is wider than its content** (5.72 against the 5.5 bound): its first row holds three wide
   boxes plus the two channels that climb out of the block; flowmaid's boxes are one line of text where
   mermaid wraps to 200 px, so a row of three is 900 px. Not a defect of the method, but it makes the corridors
   long.
6. **Frames of one band do not line up exactly**: DR's frame starts 10 px above STG's, because their first
   rows are of different heights and a frame is wrapped round its members. mermaid aligns by layer.
7. **After a drag** (`infra-overview.proto.routed.png`): the frames stay compact, but `playbooks/standby.yml`
   runs down through PROD between two of its boxes, the dotted failover edge takes a detour through the middle
   of the drawing, and labels sit on other lines (5). This is finding 2 of the summary, in a picture.

What is as good as or better than mermaid: the canvas (3.24 against 5.18 Mpx²), edge length (23690 against
30835), no line through a block title (mermaid: 7), labels nearer their ends (389 px mean against 558), the
node → block edges end on the frame clear of the title chip.

On the dense fixtures the same reading holds with more weight: `dense-td.proto.png` has the three bands of
mermaid's drawing on less than half its canvas, with three times mermaid's crossings and one bundle of a dozen
lines climbing from a hub with overlaps of up to 1982 px (`overlap_max`).

---

## 4. Effect on everything else

### 4.1 Diagrams without subgraphs — byte-identical

Every change is behind `node_cluster.is_some()` (L2, L4-L8) or behind `rk.up`, which is all-false without
clusters (L5, L9). `scene_flat` calls `layout_core(…, None, &[])` (`scene.rs:3083`), class and ER build graphs
without subgraphs (`class.rs:194`, `er.rs:155`). Checked with the prototype **switched on**: 24 of 24 inputs
without subgraphs are byte-identical to `fm038` — `examples/{architecture, class, demo, er, fanout, gitgraph,
journey, long-labels, lr, mindmap, pie, sankey, sequence, shapes, state, stress, timeline}`,
`tests/fixtures/outline/*` (5), `quality/{back-edge, wide-labels}`. (`state.mmd` is in the list because its
only composite is a B9 block and its rest graph has no subgraph left.)

### 4.2 Diagrams with subgraphs but nothing to compact — byte-identical under the trigger (S6)

A subgraph with no border edge and no sub-edge is ranked by its inner edges alone, nested or not. More
generally, when the nested ranks equal the flat ones nothing else of B12 runs. With the prototype's companions
switched off (`FMB12_NOGAP=1 FMB12_NORUN=1`, no `COMPACT`), i.e. the nested ranking alone:

| byte-identical to 0.38 | changed |
|---|---|
| `examples/advanced`, `examples/subgraph`, `examples/state`, `quality/fanout-frames`, `notes/near-drag`, `notes/spread-bottom`, `notes/straight-link`, `notes/wide-frames` | the analog, `quality/bench-75`, `quality/dense-lr`, `quality/dense-td`, `notes/fork-twins`, `notes/nested-composite`, `notes/routed-around`, `notes/spread-face` |

So the three README comparison images (`advanced`, `state`, `subgraph`) and `docs/ex-subgraph.svg` do not
change, and neither do the two files with no static timing headroom.

### 4.3 Pinned tests and gates

Both test targets were run against the prototype; the results are in 4.5. The two tables here say what each
gate and each rule-stating test means for the package.

Gates in `tests/quality.rs` that the prototype's numbers break (static, routed):

| gate | today | prototype | what it means for B12 |
|---|---|---|---|
| hard zero `shared_ports` (`111-135`) | 0 | dense-lr static 2 | **defect**: two ends on one port, on a hub whose side is over-full once turned edges leave by the upstream side. Fix in the ports stage, never loosen |
| hard zero `edge_node_x` | 0 | dense-td routed 1 | **defect** in `route()` on block positions; R1 |
| `CEILINGS` (`14-58`) | bench-75 (0,4,4) (0,0,2); dense-lr (0,13,15); dense-td (0,26,29) (0,29,30) | bench-75 (0,0,**22**) (0,0,**41**); dense-lr (0,1,4) (0,5,7); dense-td (0,1,4) (1,27,32) | frame pairs fall and can be ratcheted down; bench-75's defects are overlapping edges and must be fixed before its row is touched |
| `GAP_FLOORS` (`64-109`) | bench-75 14.7 / 10.6 | 12.06 / **2.0** | routed: lines 2 px apart; R1 and the nudge |
| `DEFECT_BASELINE` `x_drawn` (`730`) | 428 / 494 | 639 / 738 (bench-75 + dense-lr + dense-td) | the crossing regression of 3.4; an intended re-baseline only once F2 has brought it down, with the new value justified against mermaid's 306 |
| `DEFECT_BASELINE` `band_len`, `LBL_EDGE_MAX` (6, 4), `lbl_crowd` | | static `lbl_edge` 8 on the fixtures (12 with the analog) | label work, F3 |
| `BORDER_LEN_MAX` (1395, 1895) | | routed **4530** on the fixtures | routed runs along frame borders; R1 |
| `DRAG_JUMPS_MAX` (`967`) | dense-lr 0, dense-td 5, bench-75 16 | dense-lr **2**, dense-td **17**, bench-75 **55**; the notes fixtures 0 | a 1 px drag re-decides more edges on block positions (every channel beside the dragged block); R1 has to be stable under small moves |
| declaration-order robustness (`1038`, `1098-1142`) | width within 10 % | **both tests pass** on the prototype | S2 reads node declaration order only |
| `CHANNEL_MAX` / `EDGE_CHIP_MAX` (`481`, `499`) | (0, 3) | routed `edge_chip` 8 | R1 |
| `XGAP_MAX` (`415`) | (9, 12) | static 10 | one more than the gate; crossing work, F2 |
| `MAX_ASPECT`, `INK_MEAN_MIN` | | all aspects fall; ink rises (smaller canvases) | hold |

Geometry tests in the library that state a rule B12 changes or must be re-read against (from
understand-flowmaid 5.2; "holds" = true by construction of S4):

| test | expectation under B12 |
|---|---|
| `scene.rs:11186` `cross_cluster_layout_separates_boxes_and_stays_contained`, `11600` `edges_into_a_subgraph_rank_its_members_below_the_source`, `11629` `state_composite_keeps_flow_order`, `12327`, `12357` | hold: they are the block rule |
| `scene.rs:11566`, `11587`, `12225` (clean frames, straight walls, nested frames clear) | hold (walls and `FRAME_GAP` unchanged) |
| `layout.rs:5648`, `5694`, `5731`, `5764`, `5796` (`seat_box_sources` behaviour) | each pins a case of "a box fed from outside starts below its feeder, its sources on its first row". Four of the five pass on the prototype; `5648` `a_box_seats_its_loose_parts_on_its_top_layer` fails (it wants a loose member level with the hub that feeds its box only through another member; S4 puts the whole box below the hub). That one has to be rewritten to the block rule, with the reason in the commit |
| `layout.rs:5605` `a_hub_fed_column_of_frames_stays_one_column`, `5833` `…wraps_into_two_rows`, `scene.rs:12102` `wide_sibling_boxes_stack_instead_of_a_ribbon` | narrowing's stackings and wraps become constraints of the level; all three pass on the prototype |
| `scene.rs:12048` `hub_fed_leaves_in_a_box_pack_into_a_grid` | holds: `grid_pairs` are links inside the block and make its rows |
| `scene.rs:12129` `largest_box_is_centred_among_its_siblings` | `centre_largest` still runs; passes on the prototype |
| `scene.rs:12249`, `14708`, `14726`, `14814` (a lane stays in its frame) | node-level back-edges only; unchanged |
| `render.rs:295-357` (advanced.mmd) | unchanged under the trigger (fails in the prototype, which packs `advanced` although its ranks did not change: `Rollback --> Platform` no longer straight) |
| `compound.rs:986-1191` | unchanged |
| `layout.rs:6112` `a_labelled_gap_between_composites_holds_its_labels` | S10 only adds room; holds |

### 4.4 Stability under reordering (what was measured)

The analog's five permutations (`u-mermaid/in/perm-*.mmd`), walk A only (before the safety net existed; the net
does not fire on any of them, 5 against 3):

| permutation | block arrangement | canvas | crossings |
|---|---|---|---|
| as written | actors / EXT + GH / PROD / STG + DR | 2188×1310 | 43 |
| edges reversed | same | 2476×1323 | 43 |
| subgraphs reversed | same | 2188×1310 | 43 |
| actors reversed | same | 2188×1310 | 43 |
| edges rotated | same | 2545×1327 | 42 |

The arrangement is the same in all five (mermaid's is not: its `perm-a` puts the actors mid-drawing). The
width moves by up to 16 %, from the stages after ranking (node-level cycle breakers and the median sweeps read
edge order). The fixtures' own tests (`edge_line_order_barely_moves_the_dense_layouts`,
`moving_a_node_declaration_barely_moves_dense_td`: width within 10 %) pass on the prototype (4.5).

### 4.5 What the test runs said

`cargo test --release --lib` in the prototype tree (1052 tests, 5 ignored):

| configuration | passed | failed |
|---|---|---|
| `FMB12=0` (the patch switched off) | 1047 | 0 |
| nested ranking + turned channels only (S1-S5, S7; `FMB12_NOGAP=1 FMB12_NORUN=1`, no packing) | 1043 | 4 |
| the prototype (S8-S10 on every clustered input, no trigger) | 1036 | 11 |

The four that fail with the ranking alone — these are the tests B12 really touches:

| test | what it pins | reading |
|---|---|---|
| `layout.rs:5648` `a_box_seats_its_loose_parts_on_its_top_layer` | a `seat_box_sources` case | intended change (table above) |
| `scene.rs:17241` `a_composite_edge_to_its_own_state_drops_straight_from_the_border` | composite → own state drawn straight | the state moved a row; the stub jogs by 4 px. Defect to fix (`own_box_stub` aims at the member's new spot) |
| `scene.rs:14640` `far_apart_back_edges_and_frame_legs_square_up_alike_in_route` | a leg 7 px off a frame border in `route()` | positions changed under a routed test; re-read, likely a fixture update |
| `casing.rs:1383` `a_line_takes_one_role_across_a_cluster_of_crossings` | a crossing-cluster count on a dense fixture ("static 148") | follows the crossing numbers; F2 |

The seven more that fail in the prototype come from S9 / S10 running on diagrams whose ranks did not change
(`render.rs:196` advanced, `ports.rs:2455`, `router.rs:5926`, `router.rs:6655`, `scene.rs:17272`,
`scene.rs:11303` `route_partial_keeps_unmoved_edges_and_reroutes_moved` — a base scene 1.6 px off) and from a
note link whose static and routed corners now differ (`scene.rs:16564`, `nested-composite`). The first six are
what the trigger (S6) is for; the note link is risk K7.

`cargo test --release --test quality` on the prototype: 6 passed, 7 failed.

| test | result |
|---|---|
| `edge_line_order_barely_moves_the_dense_layouts`, `moving_a_node_declaration_barely_moves_dense_td` | **pass** |
| `layout_shape_gate`, `every_shape_end_sits_on_its_outline`, `long_labels_wrap_and_dense_labels_do_not_tower`, `every_fixture_has_a_ceiling` | pass |
| `quality_gate` | fail: ceilings and hard zeros of 4.3 |
| `defect_baseline_gate` | fail: `x_drawn`, `lat_wiggles`, `band` |
| `a_one_px_drag_leaves_unrelated_edges_put` | fail: 2 / 17 / 55 jumps on dense-lr / dense-td / bench-75 |
| `edges_keep_off_frame_borders` | fail: routed `border_len` 4530 |
| `channels_keep_edges_apart_and_off_chips` | fail: routed `edge_chip` 8 |
| `crossing_gaps_do_not_read_as_dashes` | fail: static `xgap_max` 10 |
| `labels_cover_nothing_they_must_not` | fail: static `lbl_edge` 8 > 6 |

Five of the seven failures are in the routed or drag columns: they are `route()` on block positions without R1.

### 4.6 Risks

| # | risk | how likely / what to do |
|---|---|---|
| K1 | **Drag budget.** `route()` is 1.4-2.75× slower on block positions without R1, and R1 is designed, not built. If R1 does not bring it back, B12 cannot ship as the default | high impact; build R1 first and measure with `perf.sh` before the layout work is polished |
| K2 | **Crossings.** The global median sweep does not find good band orders; crossings rise 30-150 % on the inputs that change. A premium result needs a band-aware crossing step (F2); its size is unknown | certain; this is the main reason this design alone does not meet the bar |
| K3 | **Hard-zero gates** (`shared_ports`, `edge_node_x`) fail in the prototype | must be fixed in ports / `route()`, both look local |
| K4 | **Block order threshold.** `2·M + 1` is tuned on two inputs (analog 5 : 3 keeps, dense-td 19 : 4 switches). One added edge can flip a level's order (as in mermaid, but bounded) | medium; needs the 1800-file generated sweep the project used for B8 before the constant is pinned |
| K5 | **Lane rule** (S3) is unexercised by the corpus and unprototyped | low impact (it only keeps today's drawing), but the constant is a guess |
| K6 | **Long rows.** A block whose members have no edge among themselves is one row (dense-lr: a column of 14 boxes, 1600 px). mermaid does the same; `grid_pairs` packs only edge-less or hub-fed members | medium; F4 |
| K7 | **State diagrams.** Four of eight state fixtures change; fork bars across blocks and note cards next to turned edges are reasoned about, not tested. The note-link tests (`scene.rs:16564`, `16712`) pin scene = route for links | medium |
| K8 | **Stability under reordering**: the fixtures' two robustness tests pass on the prototype, but the analog's width moved 16 % over its permutations (4.4) | low to medium; the later stages read edge order, as they do today |
| K9 | **Static budget on tiny inputs**: `subgraph.mmd` is at its limit; S6 adds one linear pass | low; measure |
| K10 | **`narrow()` interplay**: stackings and wraps are level constraints now; a candidate that used to stack two boxes may be a no-op (already stacked) and narrowing loses a move; `stagger_proposals` could push a member out of its block's rows without L10 | medium; L10 |
| K11 | The prototype's result depends on several interacting heuristics; three of its variants swap places on different inputs (table in 3.3). The package needs the generated corpus, not eight files | certain |

---

## 5. Where this stops short of a block layout, and what a follow-up needs

What "true block layout" has that this does not:

| | this design | a block layout |
|---|---|---|
| a block's inside | ordered and placed by the global pass, then packed | laid out on its own; its size is known before the parent is laid out |
| blocks in one band | share global rows; a 1-row block beside a 3-row block is aligned to its first row | each is one vertex of the band, aligned as a whole (ELK: by port ratio) |
| order of blocks in a band | by-product of the node-level sweeps | a decision at the level, on the level's edges |
| hierarchical ports | none: an edge crosses a frame wherever its channel happens to be | a port on the frame, ordered with the others, the inner and outer parts joined there |
| `route()` | geometric lanes (R1) | the same block structure, recomputed from positions |
| labels of inter-block edges | the chain's middle dummy, anywhere | on the part of the edge between the bands, with room reserved |

Follow-ups, in the order I would do them:

- **F1 (part of B12, not optional): R1**, block lanes in `route()` — 2.3.
- **F2: band-aware crossing reduction.** After the sweeps, per level and band: order the blocks of the band
  (and the runs passing it) by the barycentre of their inter-block links, try the permutations when there are
  at most 4 (inside `SEARCH_WORK`, judged by `drawn_crossings` as `centre_largest` judges its hint,
  `layout.rs:1936-1998`), feed the winner through `Opts::hint` generalised to several keys. This is the lever
  for the analog's 44 crossings (DR and STG are in the wrong order) and probably for dense-td.
- **F3: labels of inter-block edges between the bands**: size the dummy of the first layer *outside* both end
  blocks instead of the chain's middle one, and count label rows in S10's room. Aimed at `lbl_struck` 4 and
  `lbl_crowd` 10.
- **F4: wrap a long row inside a block** (`grid_pairs` extended to members without inner edges, when the row is
  more than `RIBBON_ENTRY` times as broad as deep); edges to the second row pass the first inside the frame,
  which `apath` already does.
- **F5: band alignment of frames**: sibling frames opening on one layer share their start (and those closing
  on one layer their end). It has to be a rule on positions (in `cluster_raw_boxes`, e.g. side-by-side sibling
  frames whose starts differ by less than `SUB_HEADER` share the smaller one) so that `route()` and
  `route_partial` draw the same frame; done only in the static scene it would make frames snap on the first
  drag (H3). Fixes `sib_stagger`.
- **F6: flow-facing ports for inter-block edges** (the side entries of 3.5 item 2) in `ports::assign`.
- **F7 (the real thing): blocks as parts.** Generalise `compound.rs` to subgraphs with border edges — hierarchical
  ports on the proxy, the inner and outer halves of an edge joined at the frame, one edge pass over the joined
  scene. Everything in section 1 except S7-S9 carries over (the nested ranking *is* the per-level layering of
  such a design; S2 and S3 are its block order). That is a different package size; this design is its first
  third.

---

## Appendix A. How to reproduce

```sh
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
cd $B/d-hybrid/tree            # = $B/tree with src/layout.rs patched ($B/d-hybrid/prototype.patch is the diff)
CARGO_TARGET_DIR=$B/target-x cargo build --release --bin flowmaid --example quality
export FMB12_COMPACT=1 FMB12_TIGHT=1 FMB12_ORDER=half            # "the prototype"
$B/metrics/run.sh fm $B/target-x/release/flowmaid /some/out proto && $B/metrics/judge /some/out/metrics.tsv
$B/target-x/release/examples/quality <dir of .mmd>               # static and routed rows, scene_ms, route_ms
FMB12=0 …                                                         # 0.38's ranking, byte-identical output
```

The build directory used here (`target-d-hybrid`) was deleted after the runs; `d-hybrid/tree` holds the patched
sources.
