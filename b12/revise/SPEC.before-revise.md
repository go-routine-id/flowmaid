# B12 "subgraphs as blocks" — specification

Date 2026-11-10. Written for an implementer who has seen none of the design work.

- `B` below is `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12`.
- Every `file:line` points into `B/tree/src` (dev = release 0.38.0, commit 0366084). **You will work on top of package
  B11 (crossing hops in the SVG painter, comb bundling for fans) and a class/state leftovers package, so find code by the
  function names given here; the line numbers only say where that function was in 0.38.0.**
- Read `scratchpad/PLAN.md` first. Its invariants are not negotiable.
- `B/private/` holds the user's two screenshots. Look at them, never copy them or any name or address in them anywhere
  else. Everything written down uses the sanitized analog `B/in/infra-overview.mmd` and its ids (U1, DNS, EXT, PROD ...).
- Background, in the order worth reading: `B/understand-flowmaid.md` (today's pipeline), `B/understand-mermaid.md` (what
  mermaid's ELK does, rules R1-R10), `B/metrics/README.md` (the yardstick), `B/design-global.md` (the design this spec
  builds), `B/design-hybrid.md` and `B/design-recursive.md` (source of the grafts).

## 0. Decisions in one page

1. **Design built: `d-global`**, the unanimous winner of the three judges: one Sugiyama pass over all nodes, one scene,
   one `edgepass::finish`, `compound.rs` (B9) untouched; blocks come from a block-aware layering, a block-aware ordering
   and a hierarchical breadth assignment. No edge is ever laid out in two halves, so the scene contract holds without a
   stitching step.
2. **Departures from `d-global`, each on evidence** (the experiment is appendix A; I ran it for this spec):
   - **Ordering is level-first with several starts, not "flat sweeps plus a block transpose".** The judges asked for the
     transpose to be shown to reach 24 crossings on the analog. It does not: swapping two sibling blocks, judged by the
     layered crossing count, changed nothing useful (40 -> 42 drawn), and forcing the swap gave 62-63. What works is ordering
     each level's units first (blocks and passing chains as units) and then the members inside, from 7 deterministic
     starts: **analog 20 drawn crossings (0.38: 24, d-global prototype: 40, mermaid: 16)**, v2 20, the five permutations
     20-25, bench-75 295 (0.38: 302).
   - **An upstream edge between blocks is a dummy chain through the flow-facing sides, not a back-edge lane.** With flat
     ordering lanes looked better (40 against 43), which is why `d-global` chose them. With level-first ordering chains
     win (20 against 34), because only then is every drawn edge in the model the ordering minimises: the layered count
     predicts the drawn count with chains (23 -> 20) and does not with lanes (11 -> 24 or 34). It is also mermaid's look
     (returns run straight up between the blocks).
   - **Layering is `d-hybrid`'s `block_rank`**: node-level cycle breakers kept, block order by two declaration-order
     walks with an exact-minimum safety net, no interleaving of siblings, the lane rule; plus one fix (back flags are
     recomputed from the final layers).
   - **The trigger** (`d-hybrid` S6): a diagram whose block layers equal its 0.38 layers is drawn exactly as before, in
     `scene()`, `route()` and `route_partial()`.
3. **What is proven and what is not.** Proven by prototype or experiment: the bands, compact frames, no run through a
   foreign frame, the trigger's byte identity, the block-order safety net, crossings on the analog family. **Not built by
   anyone yet**: the coordinate rule of 2.5, gap and label room (2.6), flow-facing ends and title-clear entries (2.8),
   the corridors in `route()` (2.9), level-first ordering below the root level. Section 4 therefore has checkpoints with
   stop rules: if a checkpoint's gate cannot be met, stop and report; do not re-baseline a gate to pass.

## 1. Goal, the user's words, before and after, what must not change

### 1.1 Goal

User feedback on 0.38.0 (2026-11-10): the same private architecture diagram (a `flowchart TD`, 3 top-level actors, 5
subgraphs, 27 edges, most labelled, 18 across a subgraph border, 3 from a node to a whole subgraph) is "still far tidier"
in mermaid 12.2. Quality bar in the user's words: **"tidak main-main dan premium"**; rare cases matter.

User decisions: package B12 "subgraphs as blocks"; **the default for every flowchart and state diagram with subgraphs**
(not only `flowchart-elk`); diagrams without subgraphs must not change; it runs after B11 and the class/state leftovers.

What a block layout means here:

- every non-empty subgraph (and state composite) is **one compact block**: its members take consecutive rows along the
  flow, and the frame is as large as its content;
- two blocks an edge joins are **stacked along the flow**, never side by side; unlinked blocks may share a band;
- an edge between blocks **runs in the gaps between blocks**, crosses a frame only through a flow-facing side, and never
  runs through a block that holds neither end;
- labels sit on the part of their edge outside the end blocks, with room reserved.

### 1.2 Before and after on the analog

| picture | path |
|---|---|
| 0.38.0 (the complaint) | `B/infra-overview.fm038.png` |
| mermaid 12.2 default (ELK) | `B/metrics/mermaid-svg/infra-overview.mermaid.png` |
| `d-global` prototype (flat ordering, lanes) | `B/d-global/infra-overview.proto-lanes.png` |
| spec experiment (level-first ordering, chains, 7 starts) | `B/spec/evidence/infra-overview.lvl-chains-7starts.png` |
| the same, real diagram's structure (replication edge turned) | `B/spec/evidence/infra-overview-v2.lvl-chains-7starts.png` |
| unchanged `route()` at the experiment's centres | `B/spec/evidence/infra-overview.lvl-chains.routed.png` |

The analog differs from the real diagram in one edge (`PG --> DRAPP`; the real one runs from the standby stack to the
database). `B/u-mermaid/in/infra-overview-v2.mmd` has it turned; with it the three servers stack one under another as in
the user's mermaid screenshot. **Both files are acceptance inputs.**

Target picture: actors / External services + Code hosting / PROD / the two other servers (v2: STANDBY / PROD /
DEV-STAGING), DNS at the end of its block that faces its corridor, the hub's lines down one corridor beside PROD, returns
to the chat bot straight up between blocks.

Numbers (block-metrics, `B/metrics/`):

| metric | 0.38.0 | mermaid | `d-global` proto | spec experiment | B12 must |
|---|---|---|---|---|---|
| canvas | 2787x1225 | 2199x2358 | 2208x1281 | 2544x1281 | area <= mermaid's |
| frame compactness mean / max | 6.90 / 11.43 | 3.25 / 4.59 | 3.43 / 5.09 | 3.56 / 5.72 | <= 4.0 / <= 5.5 |
| largest frame share of the flow extent | 0.767 | 0.308 | 0.314 | 0.314 | <= 0.40 |
| side-by-side pairs joined by an edge | 5 | 0 | 0 | 0 | 0 |
| interleaved pairs (`sib_stagger`) | 3 | 0 | 1 | 1 | 0 |
| runs through a foreign frame | 3 (2012 px) | 0 | 0 | 0 | 0 |
| crossings | 24 | 16 | 40 | **20** | <= 24 |
| edge length | 22350 | 30835 | 20356 | 20488 | <= mermaid's |
| labels struck / crowded | 0 / 8 | 0 / 0 | 0 / 9 | 3 / 9 | 0 / <= 4 |

What the experiment's picture still gets wrong, and which step owns it: PROD is wider than its content (2.5); the two
bottom frames start 17 px apart (2.7 F2); labels sit on lines in the corridors, three are struck (2.6); `U1 -> DNS`,
`DNS -> PSDEV` and two more enter through a lateral side (2.8 E4, E5); the dotted `PSPROD -> TG` goes round the standby
block (2.4, ordering below the root; 2.8 E3).

### 1.3 What must not change

1. **Diagrams without subgraphs, and class / ER / sequence and every other diagram type: byte-identical SVG.** Every
   line B12 adds to shared code sits behind "clustered and in block mode". There is no SVG snapshot today, so B12.0 adds
   a hash test over every flat corpus input.
2. **A diagram with subgraphs whose block layers equal its 0.38 layers (2.2): byte-identical**, in `scene()`,
   `route()` and `route_partial()`. Today that is `examples/{advanced, subgraph, state}.mmd`,
   `tests/fixtures/quality/fanout-frames.mmd`, `tests/fixtures/notes/{near-drag, spread-bottom, straight-link,
   wide-frames}.mmd` (verified by three independent builds).
3. PLAN invariants 1-7: the interactive API and its speed (`scene`, `scene_sized`, `route`, `route_sized`,
   `route_partial`, hit-testing), the scene geometry contract (first `Graph::edges.len()` scene edges index-parallel,
   sub-edges after, frames outermost first, waypoint semantics), `fold::scene_compact`, `to_mermaid`, the `advance`
   module, determinism, zero dependencies, MSRV 1.75, wasm32, portable SVG.
4. B9: a subgraph with its own `direction`, an inner edge and no border edge is laid out by `compound.rs` exactly as
   today; one with a border edge takes the direction round it.
5. What flowmaid already does better than mermaid (PLAN invariant 4): titles never struck, white label chips, compact
   canvas, straight spines, source order as the tie-break, edges into a subgraph ending on the frame clear of the chip,
   stable output under re-ordering of edge statements.
6. Public API: `layout::{layout_clustered, layout_sized, LayoutResult, Placed}` keep their signatures. `layout_clustered`
   returns block layouts in block mode; say so in its doc comment and in the CHANGELOG.

## 2. The algorithm

All rules are stated in flow coordinates: *flow axis* = y for TD/BT, x for LR/RL; *breadth* = the other axis;
*upstream / downstream side* = the two sides of a box across the flow axis. A rule that differs per direction says so.

### 2.1 Terms

- **Level**: the root, or a non-empty subgraph (a subgraph with at least one node at any depth). An empty subgraph is
  no level, no unit, has no frame, and its sub-edges are skipped, as today.
- **Unit** of a level: a node whose innermost subgraph is that level, or a non-empty subgraph whose parent is that level
  (a *block unit*). A B9 direction block has been cut out by `compound::split` before any of this runs and appears as a
  plain proxy node.
- **Key** of a unit (declaration order): a node's index in `Graph::nodes`; a block's = the least node index inside it.
  Keys are distinct.
- **Link**: a real edge, a stand-in edge for a sub-edge (`scene_clustered` builds one per sub-edge), an invisible link,
  or one `from x to` pair of a rank-only group (`sub_edge_proxies`, `grid_pairs`, narrowing's stackings and wraps).
  Self-loops are no links. A link is *drawn* when it is a real edge or a stand-in (weight 1); a rank-only pair weighs 0.
- A link `u -> v` **belongs to** the level of the lowest common ancestor of `u` and `v` and joins there the two units
  `cu`, `cv` that hold `u` and `v`. An edge is an **inter-unit edge with a block end** when `cu` or `cv` is a block,
  i.e. when it crosses at least one frame.
- **Rows** of a unit: 1 for a node, for a block the number of layers of its own level.
- **Band**: a maximal run of a level's layers covered by overlapping units.
- **Run**: the dummies of one chain that have the same cluster path on consecutive layers (the chain passing one level
  beside that level's blocks, or descending inside one block).

### 2.2 Block mode and the trigger

`block_mode` is decided **once per `layout_core` call**, from the first ranking, before any narrowing:

1. `flat` = today's `rank()` (hub graph, node-level cycle breakers, longest path, `seat_box_sources`, lead nodes).
2. `blk` = the layering of 2.3.
3. Block mode is **on** iff some node's layer differs between `flat` and `blk`, or 2.3 L3 turned a link.

Off: the whole call continues on the pre-B12 path (every candidate of `narrow`, ordering, coordinates, layer lines), and
`scene_clustered`, the edge stages, `route()` and `route_partial()` run their pre-B12 code. On: every later step of this
section applies, to every candidate.

The decision is a pure function of the graph (ranks do not depend on sizes), exposed as `layout::block_mode(g,
node_cluster, rank_only)` so `route_built` and `route_partial_with` reach the same answer without a layout.
A B9 part (a block's inside, or the rest with its proxies) decides for itself when it goes through `scene_labelled`.

Why: it protects every diagram that is already block-shaped (the eight of 1.3 item 2, the README images, the two files
with no static timing headroom) and it is the precise form of "leave it alone".

### 2.3 Step L: layering (replaces steps 3-4 of `rank` in block mode)

**L0. Node-level back-edges stay.** `rank` marks back-edges with its two DFS walks exactly as today. A link marked there
is used reversed in everything below.

**L1. Level graphs**, deepest level first, the root last: the level's units sorted by key, and its links collapsed to
unit pairs (`cu != cv`; parallel links between one pair are one link with their summed weight).

**L2. Note cards.** A link with a note-card end (`style::is_note_card`) takes no part in L3-L5. After L6 a card takes
`layer(target) + (flat.layer(card) - flat.layer(target))`, clamped into its own block's interval: it keeps the position
relative to its target that 0.38 gives it (`note left of` one layer before, the lead-node step of `rank` unchanged).

**L3. Block order: which links run against the flow.** Only for a level whose unit graph has a cycle.

1. Walk A: DFS over the units; roots = units without an incoming link in key order, then every unit in key order; a
   unit's links are followed in key order of their targets, then link index. A link into a unit still on the stack is
   *turned*.
2. Walk B: the same DFS with the roots in plain key order.
3. The walk that turns fewer drawn links is taken; walk A on a tie.
4. Safety net: for a level of at most `BLOCK_ORDER_EXACT_MAX` = 12 units let `M` be the least number of drawn links any
   order of the units turns (exact, dynamic programme over subsets; ties to the lexicographically smallest key
   sequence). If the taken walk turns more than `2 * M + 1`, the minimal order replaces it. Larger levels, and drawings
   past `WALK_MAX_ELEMENTS`, keep the walk.

Reason for a threshold instead of "always the minimum": on the analog the minimum (3) puts PROD above External services
and sends `DNS -> ingress` upwards; the walk turns 5 and gives the order the author wrote and mermaid draws. On dense-td
the single walk of `d-global` turns 21 of 47 edges; with the net 5. The rule reads node declaration order only, never
the order of edge statements. Every level therefore has at most `2 * M + 1` turned drawn links.

**L4. Lane rule.** If `M` (known when L3.4 ran) is more than `LANE_SHARE` = 1/3 of the level's drawn links, no stacking
order is good. The level's child subgraphs then stay *transparent* for layering: their members are ranked as units of
this level, which is 0.38's ranking for that level (two swimlanes with `a1 -> b1 -> a2 -> b2 -> a3 -> b3` stay side by
side with every arrow forward; `d-global`'s prototype stacks them and points 2 of 5 arrows up). No corpus input
triggers it. The constant is pinned by the sweep of B12.13.

**L5. Rows of a level**: longest path (Kahn, queue seeded by key) with `start(cv) >= start(cu) + rows(cu)` for every
link not turned, and with the ends swapped for a turned one. Then:

- *No interleaving*: while two sibling **blocks** `A`, `B` overlap in rows without one containing the other
  (`start(A) < start(B) < end(A) < end(B)`), add `start(B) >= end(A)` and solve again (first such pair in key order; the
  constraint points from the earlier start to the later, so it closes no cycle).
- *A source block sits next to what it feeds*: a block with no constraint into it starts at
  `min(start of its successors) - rows` when that is later. Plain source nodes stay on the first row, as today.

`rows(level) = max(start + rows)`.

**L6. Absolute layers.** `layer(node) = sum of the starts of its enclosing blocks + its own start`. A rank-only hub gets
`max(layer of its unturned sources) + 1`. Cards as L2. Then, for every drawn edge, **`back[e] = layer(to) < layer(from)`**
(this replaces the flags of L0; `d-hybrid` kept the old flags, so an edge marked back that ended up pointing downstream
got no chain and was drawn as one curve across several layers).

Properties, by construction: a block's members fill exactly its interval; two units of one level joined by a link have
disjoint intervals; no edge joins two nodes of one layer; sibling blocks never interleave.

**L7. Later re-rankings are fenced in block mode.** `tighten_sources` and `stagger_proposals` may not move a node out of
its block's interval nor change a block's rows (one interval test per proposal). `stack_candidates` and
`wrap_candidates` keep working: their groups are rank-only links, i.e. constraints of their level.

### 2.4 Step O: ordering (in `place`, block mode)

**O0. Chains.** Every drawn edge spanning more than one layer gets a dummy chain, **downstream and upstream**, except an
invisible link and an upstream edge whose two units are both plain nodes of one level (that one keeps today's back-edge
lane, package B3). An upstream chain is built from the lower layer to the higher and stored in from -> to order, as
`place` already does for a descending edge. Chain dummies take cluster paths by today's `apath` rule (the end's path
where the layer lies in that end's interval, else the common prefix), so a chain passes a foreign band at the level of
the common ancestor and enters each frame on its chain through a flow-facing side.

**O1. Units of a level, per layer**: its direct nodes, its child blocks (on every layer of their interval), and its
runs.

**O2. Level order, top-down, the root first.** For level `L` build the collapsed layered graph: one vertex per (unit,
layer); every link of the augmented graph between two layers of `L`, collapsed to units, weighted by its multiplicity;
links inside one child block dropped; a child block tied to itself across consecutive layers with weight `TIE` (1000: a
run can never change sides of a block between two of its rows, which is `d-hybrid`'s "run keys" as a property). For a
level other than the root the layers just outside it are fixed neighbours with the positions the parent gave them.
Order it by weighted-barycentre sweeps (down, then up) with a weighted adjacent exchange after each half; **judge after
each half**, keep the least weighted crossings, earlier on a tie; at most 12 rounds, from two starts (current order,
each layer reversed). Result per layer: an ordinal for every child block and a *slot* (the number of blocks before it)
for every direct node and run.

**O3. Inside the slots.** Today's sweeps (`reorder`, `transpose`, then regrouping) run over the whole augmented graph,
with `arrange` sorting the units of each level by (slot or ordinal from O2, own position): blocks stay where O2 put
them, nodes and runs are free inside their slot, members are free inside their block. Regroup after every layer's
reorder, judge after the downward half and after the upward half, up to 16 rounds, stop at 0. (Today's loop judges only
after the upward half; on the analog that parks every chain where its target is and leaves 39 crossings in one gap.)

**O4. Starts.** O2 + O3 run from `BLOCK_ORDER_STARTS` starts: start 0 is the written order; start `k > 0` shuffles every
layer with a fixed linear congruential generator seeded with `k` (integer arithmetic only: deterministic on every
platform). The start with the fewest layered crossings (`count_crossings`) wins, the lowest index on a tie.
`BLOCK_ORDER_STARTS` = 7 (ELK's default thoroughness), reduced by a work budget
(`starts = clamp(BLOCK_ORDER_WORK / (augmented nodes + links), 1, 7)`), 1 past `SEARCH_MAX_ELEMENTS`.

**O5.** Bar cards are placed as today. `centre_largest` does not reorder the blocks of a level O2 ordered.

**O6. Fans.** The chains leaving one node side are in the order of their first dummies, so a hub's fan (DNS: six edges)
does not cross itself; `ports::assign` orders the ends by heading as today. Acceptance counts crossings between two
edges of one node side: 0 on the analog.

Evidence (drawn crossings, block-metrics; full table in appendix A):

| ordering | analog | v2 | perms a-e | bench-75 | dense-td | dense-lr |
|---|---|---|---|---|---|---|
| 0.38.0 | 24 | 19 | 12 19 24 24 34 | 302 | 80 | 46 |
| flat sweeps, lanes (`d-global`) | 40 | 23 | 36 47 40 48 43 | 335 | 126 | 66 |
| flat sweeps, chains | 43 | 29 | 46 43 43 43 35 | 373 | 122 | 55 |
| level-first root + 7 starts, lanes | 34 | 28 | 34 34 34 34 37 | 314 | 101 | 36 |
| **level-first root + 7 starts, chains** | **20** | **20** | **22 25 20 22 22** | **295** | 112 | 41 |
| mermaid | 16 | 13 | | 228 | 68 | 10 |

The experiment ordered only the root level and used `d-global`'s single walk for the block order. dense-td (7 nested
frames, 21 turned edges under that walk) is the input that needs O2 on every level and L3's safety net; its must is 90.

### 2.5 Step X: coordinates across the flow (block mode)

**X1. Brandes-Köpf aligns inside a block only.** In `coordinates_bk` an alignment link is offered only when both ends
have the same cluster path (walls as today). A member lines up with members of its block, a chain is straight inside
each block and beside each band and bends where it crosses a border, i.e. in a gap.

**X2. Each block is fixed on its own, deepest first.** Items: its direct nodes, inner runs, two walls, child blocks
(already fixed, rigid).

1. `W_min`: longest path over the left-of relation of every layer with `pair_sep` as distances (the block's least
   breadth for this order).
2. Wall-to-wall breadth is at least the title chip's breadth plus `CHIP_CLEAR` (TD/BT), so nothing foreign ever lies
   under a title that is wider than the content. The frame itself stays the box of the members (2.7).
3. If the breadth Brandes-Köpf gave the block is within `BLOCK_SLACK` = 4 px of that: keep its coordinates.
4. Else squeeze, seeded by Brandes-Köpf: merge every *spine* (two items of the block on adjacent layers that are each
   other's only inner link in that direction) into one rigid column; clamp each unit's Brandes-Köpf offset into its
   feasible interval for breadth `W_min` (left-packed to right-packed position), left to right so the order holds; then
   `COMPACT_SWEEPS` sweeps moving each unit to the weighted median of its links **inside the block** (weights as
   `compact_breadth`: 1 / 2 / 8), within its neighbours.

**X3. A fixed block is one rigid unit of its parent level**, which is treated the same way (keep Brandes-Köpf when
compact, else squeeze), up to the root. A block as a whole is pulled toward the median of its border-crossing links.

**X4.** Along the flow, blocks of one band start on the band's first layer line; a source block ends on its last (L5).

`d-global`'s prototype starts X2 from the left-packed positions and so discards Brandes-Köpf: chains inside a composite
stagger, its alignment rule changes no byte. X2.3 and X2.4 are the fix, **specified, not prototyped**; B12.5 gates them
with a new measure: an intra-block edge on adjacent layers whose ends are each other's only inner link is straight
(`spine_off` = 0).

### 2.6 Step G: layer lines, gap room, label slots (block mode)

**G1. Room between frames.** Where a frame closes on layer `li - 1` or opens on layer `li`:
`lcoord[li] >= lcoord[li-1] + reach_of(li-1, 1) + need(li) + reach_of(li, 0)`, with `need` the largest of `track_need`,
the label rows of `gap_need`, `cross_need` and the labelled-gap room of `short_labels`. Today the room is measured from
the row above to the slot below and ignores the frame pad and the title strip (the layer-line section of `place`), which
is why the prototypes crush the band gaps (bench-75: lines 0.5 px apart).

**G2. Tracks.** `track_need` already counts every sideways step in a gap, chain bends included; the bends X1 puts at
block borders are such steps. One `TRACK_PITCH` per overlapping step. Steps on one side going the same way nest (the
rule `ortho` has since B10).

**G3. Label slot.** A chain's labelled dummy is the middle dummy of the chain's run **at the edge's common-ancestor
level** (outside both end blocks), chosen after `apath` is known. When that run is empty the label takes the border gap
by the `short_labels` rule, measured from the frames. An adjacent-band labelled edge keeps `short_labels`. A label never
sits inside an end block or on a frame border.

**G4.** None of this runs outside block mode. Wider gaps are paid for by the canvas: the area caps of 4.3 apply.

### 2.7 Step F: frames

**F1.** A frame is `cluster_raw_boxes` of the member positions (pad, title strip on the screen top), in every entry
point.

**F2. Aligned starts** (block mode, inside the shared frame function): sibling frames whose flow projections overlap and
whose leading edges differ by at most `ALIGN_SNAP` (= `SUB_HEADER`) share the earlier one. It is a rule on positions, so
`scene()`, `route()` and `route_partial()` draw the same frame. Fixes `sib_stagger` (17 px on the analog) without moving
a node off its layer line.

**F3. Growth is one function.** Today only the static scene widens a frame over the channels laid inside it, so frames
snap on the first pixel of a drag (71 px on the analog, 617 on bench-75). In block mode the static widening in
`scene_clustered` and `route_lane_clusters` call one function of (frames, drawn paths of the edges with both ends in
that frame, lanes of its own back-edges, displaced title-side entries of 2.8): grow across the flow to cover them plus
`LANE_PAD`, `stop_at_siblings`, `renest_boxes`. X2 keeps inner runs between the walls, so the growth is normally nil. An edge between two members stays
inside their frame in `scene()`, `route()` and on a drag (existing tests, unchanged).

### 2.8 Step E: how each kind of edge is drawn

| kind | static scene | `route()` |
|---|---|---|
| E1 downstream, ends in different units, more than one layer apart | dummy chain (O0): out through the downstream side of every frame round the source, beside the blocks of the bands between, in through the upstream side of every frame round the target, down inside the target block beside its members | corridor (2.9 R3) |
| E2 downstream, adjacent layers | no dummy; `ortho` draws the Z in the band gap, sized by G1 | today's channel when nothing lies between, else corridor |
| E3 **upstream, at least one end unit a block** | dummy chain the other way: out of the source's **upstream** side, in through the target's **downstream** side | corridor, mirrored |
| E4 upstream, both units plain nodes of one level | today's lane (`back_lanes`, `static_lanes`) | today's `route_backs` |
| node -> whole subgraph, subgraph -> subgraph | the stand-in's chain cut at the frame(s) (`cut_to_frames`); the end is a *free pin* on the flow-facing side | the same against the current frames |
| member -> its own subgraph | an edge to the block's entry member (`own_box_targets`), a link inside the block, drawn whole | unchanged |
| subgraph -> its own member | the stub `own_box_stub` | unchanged |
| subgraph -> itself | **new**: a loop on the block's frame, drawn like a node's self-loop with the frame as the box (0.38 draws a zero-length path) | the same |
| invisible link | ranks (L), orders, never a chain, never drawn | unchanged |
| note link | never turned (L2), never a lane or chain, as today | unchanged |

**Flow-facing ends (E1-E3).** The end on a member of a block the edge leaves or enters is on the member's side that
faces the border it crosses. The side is set where the builder sets it (`scene_from_layout`'s anchors for chains; the
corridor builder in `route_built`), and `ortho` and the router keep it; `ports::assign` only spaces the ends of a side.
A lateral side is allowed only when the two units overlap along the flow (a host can drag them side by side).

**Free pins.** An edge to a whole block ends at the middle of the stretch of the flow-facing side that is free of the
title chip when it is alone on that side; among other entries it is ordered with them by O3 and keeps `PORT_SPACING`.

**Title-clear entries (TD: the entry side; BT: the exit side; LR/RL: void, the strip is on a breadth side and pins stay
out of its breadth interval).** An edge that would cross the strip side within the chip's breadth plus `CHIP_CLEAR`
crosses the border at the nearest free column beyond the chip instead, in the order of the entries, and jogs under the
strip to its port. The jog needs a track between the strip and the first row: the header slot of that block grows by
`ROW_CLEAR + (tracks - 1) * TRACK_PITCH`, only when the block has such an entry. X2.2 reserves the room (breadth >=
chip + `CHIP_CLEAR`, plus `EDGE_GAP` per displaced entry), and when the member box is narrower than that the frame
grows across the flow to the last displaced entry's column plus `LANE_PAD`, by the shared growth function of F3 (a
function of the frame, the title and the entering edges, so `route()` grows it the same way). This replaces `d-global` 1.6, which bounded a
chain dummy that does not exist when the target is on the block's first row. Today (and in every prototype) such an
edge goes round to the member's side: `B/spec/evidence/title-entry.fm038.png`.

**Labels**: G3.

### 2.9 `route()`, `route_sized()`, `route_partial()`

The host gives positions, so no layering, ordering or coordinates run. What runs, in block mode only (2.2):

- **R1. Frames**: F1-F3, the same functions as the static scene.
- **R2. Classes by geometry and the level tree.** For an edge whose units `cu != cv` at its common-ancestor level are
  not both plain nodes: let `S`, `T` be the boxes of `cu`, `cv` (a block's frame or the node). `T` wholly downstream of
  `S` by `FLOW_MIN_GAP` -> E1/E2; wholly upstream -> E3; overlapping along the flow -> today's builders with lateral
  sides. Both plain nodes -> today's rules (`route_backs` for E4). At the scene's own centres this reproduces the static
  classes, because L6 defines `back` by the layers.
- **R3. Corridors, per level, not only at the root.** Obstacles of an edge = the other units of its common-ancestor
  level whose flow interval meets the open interval between `S` and `T`, and, inside each end block, that block's other
  units. If today's 4-point channel (`cross_cluster_route`) meets none, keep it. Else plan with `lanes::plan`: its
  `Want` (band, obstacle spans, clearance, the ends' port spans) is this situation, and it nests lanes that share a band
  and judges crossings against the other skeletons. Path: out through the flow-facing side at the port's column (title
  rule of 2.8), across in the first free gap, along the corridor, across in the last free gap, in through the
  flow-facing side. Emit bare on-axes corners so `ortho_pass` keeps the path and only rounds it; no change to `ortho`'s
  static-only columns.
- **R4. Hard obstacles.** For these edges a block that is on neither end's chain is a hard obstacle in
  `router::repair` (today a foreign frame is only priced).
- **R5. Stability under a 1 px move.** Every coordinate of a corridor is a continuous function of the frames; slots in
  a corridor go by edge index; the side is the free side nearer the source port's column, the lower breadth on a tie.
- **R6. `route_partial`** keeps its logic: `clusters_line_up` holds (frames are functions of member positions); a frame
  holding a moved node is redrawn as `route()` draws it; an edge on a moved node or a regrown frame is redrawn by the
  same builders; a kept edge keeps its base path, also a corridor that passes the moved block unless the block now
  touches it; `PARTIAL_ROUTE_SHARE` unchanged.
- **R7. The canvas** of a routed block drawing ends `MARGIN` past every drawn edge
  (`a_static_canvas_is_its_drawing_with_even_margins` fails on `d-global`'s prototype for dense-td; it must pass).

`scene()` and `route()` at the scene's centres are not equal today and are not required to be equal. They must be
closer than in 0.38: frames equal (F), edge classes equal (R2), and the gates `fr_max` and `ends_apart` of 4.3.

### 2.10 Cases

| case | rule |
|---|---|
| nested subgraphs | L1-L6 recurse; O2 runs per level; X2 fixes deepest first; an edge to a member two levels down belongs to the common ancestor's level, its chain adopts the target's path level by level and crosses each frame on a flow-facing side; R3 per level |
| node -> whole subgraph | the stand-in and the proxy group collapse to a link (node's unit, block): the whole block lies after the node; free pin (2.8) |
| subgraph -> subgraph | one link between the two units; both ends free pins; when turned it is E3 |
| member -> own subgraph / subgraph -> own member / subgraph -> itself | 2.8 table |
| cycle between blocks | L3; turned links are E3 chains; every level has at most `2M + 1` |
| two-block cycle `A <-> B` | the direction with more drawn links runs downstream; on a tie the block with the lower key is first |
| more than a third of a level's links against any order | L4: that level stays as 0.38 draws it |
| hub linked to many blocks | no layering rule; its block sits where L5 puts it; its chains share the corridor O2 chose and leave in fan order (O6); B11's comb applies to the fan (4.2 B12.10) |
| empty subgraph; subgraph of empty subgraphs | no unit, no frame, sub-edges skipped, as today |
| subgraph with one member | a block of one row |
| members without inner links | first row of their block; `grid_pairs` still packs loose members into rows by rank-only pairs, which are inner links |
| unlinked blocks | all start on the level's first row, side by side in key order; `narrow` may stack or wrap them as today |
| own `direction`, inner edge, no border edge (B9) | unchanged: `compound::split` cuts it out; the proxy is a plain unit; each part decides block mode for itself, so a B9 block inside a bordered block sits in that block's rows as one unit |
| own `direction` with a border edge | the direction is not honoured (mermaid's rule, README), an ordinary block |
| LR / RL / BT | layers run along the flow in every direction; title strip handled where it is today (`lextra`, `reach`, `header_b` in `pair_sep`); title-clear entries per 2.8; tests run all four directions |
| state composites | a composite is a block; its `[*]` states and note cards are members; a transition to a composite is a sub-edge |
| note cards, lead notes | L2; a card beside a fork/join bar keeps `bar_cards` ordering |
| fork / join bars | `fit_bars` runs before the layout, unchanged; a bar whose branches lie in different blocks is a plain unit of their common level and its branches are chains |
| invisible links | links of weight 1 for L3 (the author asked for that order), never chains |
| node self-loops | unchanged (`loop_reach` room in the separations) |
| large drawings | L is linear per level plus the subset programme (<= 4096 x 144 steps, cyclic levels of <= 12 units); O4 is budgeted; past `SEARCH_MAX_ELEMENTS` one start and no narrowing as today; past `ROUTE_MAX_EDGES` (400 scene edges) `ortho` and the router are skipped as today and blocks, chains and corridors still hold, because they come from the layout and the builders |
| class, ER, `advance`, flat flowcharts | never clustered: untouched |

### 2.11 Tie-breaks and determinism

| decision | rule |
|---|---|
| unit order | key; never equal |
| DFS roots, DFS links | L3 |
| fewer turned | walk A on a tie; subset programme: lexicographically smallest key sequence |
| longest path | Kahn, queue seeded by key, FIFO |
| no-interleave pair | first pair in key order |
| level order (O2) | fewer weighted crossings; earlier half-round; start "current order" before "reversed" |
| starts (O4) | fewer layered crossings; lower start index |
| squeeze (X2) | fixed sweep count and directions; `weighted_median` takes the lower point on a tie |
| corridor side (R5) | nearer the source port; lower breadth on a tie |
| containers | `Vec` and `BTreeMap` wherever iteration order can reach the output; no float is used as a map key |

Byte-identical SVG across runs is asserted for every block-mode fixture (the existing determinism checks cover them once
they are fixtures).

### 2.12 New constants (name them, explain them in a comment, as the code base does)

| constant | value | pinned by |
|---|---|---|
| `BLOCK_ORDER_EXACT_MAX` | 12 units | cost bound |
| turned-link threshold | `2 * M + 1` | analog (5 against 3 keeps the walk), dense-td (21 against 4 switches); sweep B12.13 |
| `LANE_SHARE` | 1/3 | sweep B12.13 |
| `BLOCK_ORDER_STARTS`, `BLOCK_ORDER_WORK` | 7, measured | experiment (7 = 16 on all 14 inputs); `perf.sh` |
| `TIE` | 1000 | larger than any link multiplicity |
| `BLOCK_SLACK` | 4 px | B12.5 |
| `ALIGN_SNAP` | `SUB_HEADER` | B12.7 |
| `CHIP_CLEAR` | existing constant | |

## 3. Code plan

Prototype code to start from (hacks with env switches; none is mergeable as is):

| piece | where | state |
|---|---|---|
| L0-L3, L5, source blocks, hub layers | `B/d-hybrid/prototype.patch`, `fn block_rank` (tree copy `B/d-hybrid/tree`) | works; lacks L2, L4, the L6 back-flag fix and the trigger as a switch |
| a shorter L1/L5/L6 | `B/d-global/prototype.patch`, `fn block_layers` | works; single walk, no safety net |
| X1 (weaker form), X2 without seeding | `B/d-global/prototype.patch`, the `coordinates_bk` hunk and `fn block_compact` | works; to be changed per X2.3, X2.4 |
| upstream chains (O0), `back_lanes` filter | `B/d-global/prototype.patch` (mode 2), `d-hybrid` S7 | works |
| O2 at the root, O3, O4, the trigger on `d-global`'s layering | `B/spec/experiment.patch` (diff against `B/d-global/tree`; tree copy `B/spec/tree`; switches `FM_B12=2 FM_B12_LVL=1 FM_B12_STARTS=7 FM_B12_TRIG=1`) | works; **far too slow** (3.5), root level only |
| per-input structure dump, block-metrics, `judge` | `B/metrics/` | ready |
| invariants probe (frames and edge ends, scene against route; drag jumps with sub-edges) | `B/j-invariants/b12inv.rs` | ready; turn into tests |

### 3.1 By file

**`src/levels.rs` (new, small, pure).** The level tree from the cluster paths (`node_cluster`, as `layout` gets them, so
the public `layout_clustered` works too): units and keys per level, each link's common-ancestor level and unit pair,
each node's chain of blocks. One function of the graph, used by `layout::block_rank`, by the ordering, by
`scene::route_built` (R2, R3) and by the new quality measures.

**`src/layout.rs`**

| function (0.38 line) | change |
|---|---|
| `struct Ranking` (2267), `rank` (2281) | keep steps 1-5 as `flat`; with clusters call new `block_rank` (L1-L6); `Ranking::blocked` per 2.2; when not blocked return `flat` untouched. Without `node_cluster` not one line changes |
| new `pub(crate) fn block_mode` | 2.2, for `route_built` |
| `layout_core` (1545), `struct Opts` (1607) | decide block mode from the first `rank`; carry it in `Opts` to every `place` and to `narrow` |
| `narrow` (1656), `tighten_sources` (2840), `stagger_proposals` (2877) | L7 fences. `stack_candidates` (2157) / `wrap_candidates` (2042) unchanged; re-read `RIBBON_ENTRY` and the acceptance slacks of `narrow` on block layouts (B12.12) |
| `place` (3026), chain loop (the "invisible links and back-edges get no channel" branch) | O0: in block mode only E4 goes without a chain |
| `place`, label dummy sizing (3132-3147) | moves after `apath` is built (3292-3341); G3 |
| `place`, ordering (3343-3397), `cluster_keys` (5487), `arrange` (5534), `transpose` (4750), `count_crossings` (4805) | block mode: new `level_order` (O2) and the multi-start loop (O3, O4); `arrange` takes slot / ordinal keys per layer. Flat and non-block clustered diagrams keep today's loop byte for byte (do not "improve" the half-round judging there) |
| `place`, `centre_largest` hint (3431-3439; `centre_largest` 1936) | O5 |
| `coordinates_bk` (4880) | X1, behind block mode |
| after `coordinates_bk` (where `opts.compact` calls `compact_breadth`, 5201) | block mode: new `block_compact` (X2, X3). `compact_breadth` stays for narrowing outside block mode |
| `place`, layer-line section (4062-4175), `reach_of` (3608), `track_need` (4002), `gap_need` (3878), `short_labels` (3630), `lextra` (3192) | G1-G3; the jog track of 2.8 in the header slot |
| `back_lanes` (4427) | block mode: an edge with a chain is no lane and counts among the forward skeletons. Guard it with block mode (`d-global`'s filter is unguarded) |
| `drawn_crossings` (4629), `drawn_breadth` | E3 edges are drawn like chains: no bow. PLAN invariant 6: the back-edge bow rule lives in `scene.rs` and here; change both or neither |
| `layout_clustered` (1514, public) | doc comment: returns a block layout when 2.2 says so |

`seat_box_sources` (2497) stays: it is step 4 of the flat ranking and the reference of the trigger.

**`src/scene.rs`**

| function (0.38 line) | change |
|---|---|
| `scene_labelled` (3050), `scene_flat` (3083), `sub_edge_proxies` (3994), `grid_pairs` (4172), `clusters_line_up` (5277), `compound` hooks | none |
| `scene_clustered` (3444) | frames (step g, 3535-3599): F2, F3 through the shared function in block mode; today's widening otherwise |
| `scene_from_layout` (3159) | E1-E3 anchors on flow-facing sides (it already draws a descending chain: the `a_bottom` branch at 3279-3285); title-clear entry columns; free pins; label on the G3 dummy |
| `static_lanes` (3097) | E4 only in block mode |
| sub-edge drawing (3608-3743): `cut_to_frames` (3772), `own_box_stub` (3832), `sub_edge_ported` (3889), `free_edge` (8541) | unchanged; new small builder for `subgraph -> itself` |
| `cluster_raw_boxes` (5346), `route_clusters` (5389), `route_lane_clusters` (5404), `stop_at_siblings` (5481), `renest_boxes` (5524) | one frame function for the three entry points: F1 + F2 + F3 in block mode |
| `route_built` (4435) | `block_mode` once per call; R2 classes; corridor builder beside `cross_cluster_route` (8400) on `lanes::plan`; `route_backs` (8097) for E4 only in block mode; the subgraph self-loop |
| `route_partial_with` (4889) | R6; no new state in the base scene |
| `title_head` (2885), `fit_cluster_title` (2902) | read by the title rule; unchanged |

**Other files**

- `src/lanes.rs`: `plan` (696) and `Want` (659) reused for corridors; a `corridor_geometry` beside the lane geometry.
- `src/ortho.rs`: no rule change intended. Verify, with tests, that `ortho_pass` (477) keeps an on-axes builder path and
  only rounds it (the rule at 716-727), keeps the end sides the builder set, and that `plan_jog` (1562) puts a
  band-gap step between the two frames (G1 gives it the room).
- `src/router.rs`: `repair` (5120): R4.
- `src/ports.rs`: no rule change. `d-hybrid`'s prototype had `shared_ports` 2 on dense-lr once turned edges left a hub
  by its upstream side; the hard zero must hold, fix the spacing if it does not.
- `src/edgepass.rs`: `ctx` (126) keeps the geometric `back`; every consumer that treats a back-edge as a lane (find them
  by `cx.back`) must ask "E4?" in block mode. `finish_held` (361) unchanged.
- `src/compound.rs`: untouched; its tests are the proof.
- `src/quality.rs`: new measures appended to `Metrics::tail_fields` (so every earlier column keeps its number):
  `compact_max`, `span_max`, `sib_side_linked`, `sib_stagger` (as block-metrics defines them), `lateral_x` (frame
  crossings through a lateral side by an edge with exactly one end in the frame), `spine_off` (2.5), `fan_x` (O6); and in
  `measure_source` (486), which has both drawings, `fr_max` (largest frame difference between `scene()` and
  `route(scene centres)`) and `ends_apart` (edges with an end more than 20 px apart between the two).
  `frame_side` (1649-1666) and `wiggles` (1626-1647) keep their definitions.

### 3.2 How the entry points stay consistent

| entry | block mode | what it shares with the others |
|---|---|---|
| `scene()`, `scene_sized()` | L, O, X, G, F, E; one scene, one `finish` | `block_mode`, level tree, frame function |
| `route()`, `route_sized()` | R1-R5; no layout; host positions never moved | `block_mode` (same pure function), level tree, frame function, edge classes (R2 = L6 at the scene's centres) |
| `route_partial()` | R6 | same builders as `route()` for redrawn edges; base frames for untouched blocks |
| hit-testing (`hit_test`, `node_at`, `edge_at`, `cluster_at`, `hits_in_rect`, `nearest_node`) | reads the `Scene` only | frames nest and siblings are disjoint (`pair_sep` + `cluster_gap`; `assert_clean_clusters`); one `SceneEdge` per edge, so `edge_at` follows the painted stroke (`edge_picking_matches_the_painted_strokes_on_every_fixture` runs on the new fixtures) |
| `fold::scene_compact` | refuses graphs with subgraphs (`FoldSkip::HasSubgraphs`), returns `scene()` | unchanged; assert it for a block-mode fixture |
| `to_mermaid`, json export | graph / scene only; no new field | round trip renders byte-identical |
| `advance`, class, ER | never clustered | flat hash test |
| `layout::layout_clustered` (public) | block layout | documented |

### 3.3 B11

B11's hops are drawn by the painter from finished scene edges and need nothing. Its comb bundles a fan's branches; a
hub's chains now leave one side in fan order and run parallel, which is the shape the comb wants, but **nobody has run
the two together**. B12.10 does, on the analog and bench-75. If the comb worsens a must (crossings, `lbl_*`,
`lane_gap_min`), restrict it to fans whose branches stay in one level and say so in the commit. block-metrics counts a
shared trunk once per edge and bridges hops before measuring (`B/metrics/README.md`).

### 3.4 Guards in shared code

Every new branch reads `blocks` (which implies clustered). No change to a line a flat graph executes: not the sweep
loop's judging, not `coordinates_bk`'s candidates, not `back_lanes`. B12.0's hash test is the proof.

### 3.5 Cost

- L: linear per level; one extra `rank`-sized pass for the trigger. `subgraph.mmd` has a 0.16 ms static limit and is at
  it; measure the trigger there first (decision D1, appendix B).
- O: the experiment's implementation is naive (maps rebuilt per call, quadratic crossing counts, every layer regrouped
  after every reorder) and costs 50 ms on the analog and 1.5 s on bench-75. Required: collapsed graphs in index
  arrays; bilayer crossings by `inversions`; exchange by local deltas; regroup only the layer just reordered; starts
  budgeted by `BLOCK_ORDER_WORK`. Target: ordering under 15 % of `scene_ms` on bench-75.
- `route()`: unchanged `route()` at block positions is slower because the router repairs more (same binary, switch off
  -> on: analog 1.9 -> 4.0 ms, dense-td 6.2 -> 8.7, dense-lr 5.8 -> 7.9, bench-75 11.3 -> 9.2). Corridors give the edge
  stages clean paths; whether that brings the numbers inside the budgets is unmeasured, which is why B12.8 comes before
  any polish.

## 4. Work items, acceptance, tests, gates, budgets, docs

### 4.1 Checkpoints

Three checkpoints. Until C passes, block mode sits behind one internal switch that is off by default, so `dev` stays
releasable: with the switch off every output is byte-identical to the release before B12.

| checkpoint | items | gate | stop rule |
|---|---|---|---|
| A structure | B12.0-B12.7 | blocks, order, compact frames, gap room: the structural musts and the crossing musts of 4.3; hard zeros 0 in the static scene; flat hash test green | crossings: analog or v2 above 24, a permutation above 26, dense-td above 90, bench-75 above 334 or dense-lr above 52 after B12.4 -> stop and report with the table |
| R route | B12.8 | every budget of 4.6; `DRAG_JUMPS_MAX` unchanged; routed hard zeros 0; `fr_max`, `ends_apart` | a budget broken on two `perf.sh` runs in a row -> stop and report |
| C acceptance | B12.9-B12.14 | `judge` reports 0 must-fails; every rule of 4.3; guard green | never flip the default with a must failing |

### 4.2 Items

Each item ends with guard green (`tools/guard.sh`) and its own regression tests. Commit per item.

**B12.0 Baselines and yardstick.** Record SVG hashes of every flat corpus input from the release B12 starts from
(`examples/*.mmd` without subgraphs, `tests/fixtures/quality/{back-edge, wide-labels}`, `tests/fixtures/outline/*`,
class, ER, sequence and the other diagram examples) and of the eight trigger-unchanged inputs of 1.3; add the test that
compares them. Add fixtures `tests/fixtures/blocks/`: the analog (if decision D5 allows; else an equivalent synthetic
diagram with the same structure), v2, the five permutations of `B/u-mermaid/in/perm-*.mmd`, `B/d-hybrid/in/{swimlane,
nested-cases}.mmd`, the probes `B/u-mermaid/probes/p*.mmd` that have subgraphs, and BT / LR / RL copies of the analog.
Add the measures of 3.1 to `quality.rs`. `perf.sh` baseline with the analog in its corpus; before renders.
*Check*: the hash test passes on the untouched tree; the new measures agree with block-metrics on the 8 inputs
(`compact_max`, `span_max`, `sib_*` within rounding).

**B12.1 Level tree and trigger.** `levels.rs`; `block_mode`; the internal switch (a `pub(crate)` constant with a
`#[cfg(test)]` override; `tests/quality.rs` and the examples reach it through one `#[doc(hidden)]` function that
B12.14 removes; no environment variable in the library).
*Check*: unit tests of the tree on nested, sub-edge, own-member and empty-subgraph graphs; `block_mode` false on the
eight inputs of 1.3 and true on the analog, v2, bench-75, dense-lr, dense-td and
`notes/{fork-twins, nested-composite, routed-around, spread-face}`.

**B12.2 Layering.** `block_rank` (L0-L7).
*Check*: on the analog the layers are actors / R1 R2 GW DNS TG OAI AZ / ARGO TR1 VAULT / LANE / PG / SITE DRAPP PSDEV TR2
/ PSPROD LANES4 / GITLAB with five upstream edges (`DRAPP->TG`, `ARGO->TG`, `PSPROD->TG`, `LANE->OAI`, `LANE->AZ`); the
same bands under all five permutations; dense-td at most 5 edges against the flow (single walk: 21); swimlane: 0 turned
(L4); no interleaved siblings on any fixture; a card keeps its layer offset to its target on every notes fixture.

**B12.3 Chains both ways and label slots (static).** O0, E1-E4, G3.
*Check*: no edge drawn as one curve across more than one layer in block mode; `wiggles` 0; a label never inside an end
block (test on the analog: every chip's centre outside both end frames).

**B12.4 Ordering.** O1-O6 on every level, budgeted starts.
*Check*: the crossing rows of 4.3; `fan_x` 0 on the analog; runs through a foreign frame 0 static on the analog, v2,
bench-75, dense-lr, dense-td (`d-global`: 3 on dense-td); ordering cost per 3.5. **Stop rule of checkpoint A.**

**B12.5 Coordinates.** X1-X4.
*Check*: analog `compact_max` <= 5.5 and `compact_mean` <= 4.0; `spine_off` 0 on the analog, v2, spread-face,
nested-composite, fork-twins; `frame_overlap`, `node_frame` 0; own-frame containment tests unchanged
(`a_lane_between_two_members_stays_in_their_box`, `a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`,
`a_rerouted_edge_between_members_stays_in_their_frame_in_route`, `back_edge_inside_a_box_bows_beside_it...`) and the
same assertion on `nested-cases` and spread-face.

**B12.6 Gap room.** G1, G2.
*Check*: `GAP_FLOORS` hold as written (bench-75 static 14.7; prototypes: 8.6-11.9); bench-75 static `overlap_pairs` 0;
analog `lane_gap_min` >= 8, `lbl_struck` 0, `lbl_crowd` <= 4, `lbl_gap_min` >= 2.5; the same label rules on v2 and the
five permutations (they had 1-6 struck chips in every prototype); area caps of 4.3.

**B12.7 Frames.** F1-F3.
*Check*: analog `sib_stagger` 0; `fr_max` <= 0.5 px on the analog, v2 and their BT/LR/RL copies, and on no input above
0.38's value by the same probe (`B/j-invariants/b12inv.rs`: nested-composite 41, wide-frames 54, spread-bottom 7.5,
routed-around 24, bench-75 458, dense-lr 303, dense-td 89; `d-global`'s prototype: 80, 84, 40, 33.5, 74, 18, 0).

**B12.8 `route()`, `route_sized()`, `route_partial()`.** R1-R7. Build before any polish; measure with `perf.sh`.
*Check*: budgets of 4.6 on two runs; `DRAG_JUMPS_MAX` as written (bench-75 16, dense-td 5, dense-lr 0, notes 0); routed
hard zeros 0; routed edge / foreign-frame pairs: analog, v2, bench-75 0, dense-lr and dense-td not above their static
values + 2 (experiment without corridors: 9 and 2; 0.38: 13 and 29); routed `min_edge_gap` at or above `GAP_FLOORS`
(experiment: 0.05 on bench-75); `ends_apart` not above 0.38's count on any input (analog 14, dense-td 12);
`a_static_canvas_is_its_drawing_with_even_margins`, `route_partial_keeps_unmoved_edges_and_reroutes_moved`,
`route_partial_matches_route_on_the_dragged_node`, `dragging_a_member_pulls_its_cluster_along` pass unchanged; a new
drag test on the analog (every node dragged across the drawing: no panic, frames follow, `clusters_line_up` holds).
**Stop rule of checkpoint R.**

**B12.9 Flow-facing ends and title-clear entries**, static and routed (2.8).
*Check*: `lateral_x` 0 static on the analog, v2, permutations and their BT/LR/RL copies; routed `lateral_x` 0 at the
scene's centres; no title struck (existing chip gates); `frame_side` 0; the probe `B/spec/evidence/title-entry.mmd` in
block mode enters beside the chip (it is trigger-unchanged, so test the rule on a variant with a border-crossing chain).

**B12.10 Hub fans with B11's comb** (3.3).
*Check*: analog and bench-75 musts hold with the comb on; a test pins the fan of DNS (six ends on one side, in target
order, no crossing among them).

**B12.11 State diagrams, degenerate edges.** Review every `tests/fixtures/notes` drawing before and after, static and
routed, and add fixtures for a fork/join bar whose branches lie in two composites and for a twin comb across a
composite border. The `subgraph -> itself` loop.
*Check*: `a_composite_edge_to_its_own_state_drops_straight_from_the_border`,
`note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `note_links_match_between_scene_and_route`,
`fork_join_branches_drop_straight_and_drags_agree`, `state_composite_keeps_flow_order` pass unchanged; block-metrics
`warn` 0 on all 49 inputs of `B/metrics/extra/in` (0.38 warns on three, all composite self-transitions).

**B12.12 Search and narrowing in block mode.** `narrow`'s entry and acceptance on block layouts;
`a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` and `wide_sibling_boxes_stack_instead_of_a_ribbon` pass unchanged
(both are trigger-unchanged). Wrap a long row of unlinked members inside a block (`grid_pairs` extended to members
without inner links when the row is more than `RIBBON_ENTRY` times as broad as the block is deep).
*Check*: dense-lr area <= 1.25 x 0.38's (7.98 Mpx2; experiment 8.93) and no block row longer than its band needs;
`MAX_ASPECT` and `INK_MEAN_MIN` hold as written.

**B12.13 Generated sweep.** The generated corpus the project used for B8 (about 1800 files) plus generated block
diagrams (2-8 blocks, 0-3 nesting levels, cycles, hubs, all four directions, state composites). Pin `2M + 1`,
`LANE_SHARE`, `BLOCK_ORDER_STARTS`, `BLOCK_SLACK`, `ALIGN_SNAP`.
*Check*: no panic, deterministic, hard zeros 0 in both modes on every file; no file with more crossings than both 0.38
and 1.1 x 0.38 + 2 without a named reason; trigger-unchanged files byte-identical.

**B12.14 Default on, ratchets, docs.** Remove the switch. Re-baseline per 4.5. README, CHANGELOG, images, benchmark
table (4.7). Final `perf.sh`, `render.sh`, `judge`.
*Check*: everything in 4.3-4.6.

### 4.3 Acceptance

**(a) The analog, block-metrics** (`B/metrics/run.sh fm <cli> <out>`, then `B/metrics/judge <out>/metrics.tsv`;
rules in `B/metrics/thresholds.json`): 0 must-fails.

| metric | 0.38 | mermaid | must | target |
|---|---|---|---|---|
| `xframe_runs`, `sib_side_linked`, `sib_stagger` | 3, 5, 3 | 0, 0, 0 | 0 | |
| `compact_max` / `compact_mean` | 11.43 / 6.90 | 4.59 / 3.25 | <= 5.5 / <= 4.0 | <= 4.6 / <= 3.3 |
| `span_max` | 0.767 | 0.308 | <= 0.40 | <= 0.31 |
| `crossings` | 24 | 16 | <= 24 | <= 16 |
| `lbl_off_mean` / `lbl_off_max` | 0.198 / 0.332 | 0.130 / 0.329 | <= 0.20 / <= 0.35 | <= 0.13 |
| `lbl_crowd`, `lbl_gap_min` | 8, 2.5 | 0, 20 | <= 4, >= 2.5 | 0, >= 12 |
| `lane_gap_min` | 9.3 | 12.0 | >= 8 | >= 12 |
| `straight_max` | 1175 | 1236 | <= 1240 | |
| `area` | 3.41 M | 5.18 M | <= mermaid's | <= 1.25 x 0.38's |
| `edge_len` | 22350 | 30835 | <= mermaid's | |
| `lbl_lbl`, `lbl_node`, `lbl_struck`, `frame_overlap`, `frame_foreign_nodes`, `node_hits`, `warn` | 0 | 0 | 0 | |

**(b) v2 and the five permutations** (rules added to `thresholds.json` in B12.0): the structural zeros;
`compact_max` <= 5.5; `span_max` <= 0.40; `lbl_struck` 0; `lbl_crowd` <= 4; crossings <= 24 on v2 (0.38: 19, mermaid:
13) and <= 26 on each permutation; **the same band order on the analog and all five permutations** (mermaid scrambles
it on the first); canvas width within 10 % across the permutations.

**(c) New measures**, static and routed at the scene's centres, on every block-mode fixture: `lateral_x` 0 (analog
family) and not above 0.38's elsewhere; `spine_off` 0; `fan_x` 0; `fr_max` and `ends_apart` per B12.7 and B12.8.

**(d) "No worse than 0.38.0" on the corpus.**

- `judge` scope `*` (advanced, subgraph, state, fanout-frames, bench-75, dense-lr, dense-td): the zero rules;
  `xframe_runs`, `sib_side_linked`, `sib_stagger`, `lbl_struck`, `lbl_crowd`, `compact_max` not above 0.38's; crossings
  <= 1.1 x 0.38's + 2 (bench-75 334, dense-td 90, dense-lr 52); area <= mermaid's; edge length <= the larger of
  mermaid's and 1.1 x 0.38's.
- flowmaid's harness (`examples/quality.rs`, `tests/quality.rs`): every hard zero 0 in both modes (`edge_node_x`,
  `shared_ports`, `arrow_on_start`, `wiggles`, `frame_side`, `node_overlap`, `node_frame`, `frame_overlap`) and
  `own_frame_x` 0; per fixture and mode `defects`, `edge_frame_pairs`, `overlap_pairs` not above 0.38's;
  `min_edge_gap` not below `GAP_FLOORS`.
- The 49 inputs of `B/metrics/extra/in`: runs through a foreign frame 0 on every block-mode input (mermaid: 0 on all 36
  inputs with frames; 0.38: 45 runs on 10), `warn` 0.
- Trigger-unchanged inputs and all flat inputs: byte-identical (hash test).

**(e) Invariants**: guard green; determinism; the scene contract test
(`scene_carries_stable_identity_and_stays_index_parallel`) on the block fixtures; hit-testing, fold, `to_mermaid` round
trip on the block fixtures in all four directions.

### 4.4 Tests to add (names are suggestions; one per rule)

| test | asserts |
|---|---|
| `flat_and_unchanged_inputs_render_as_before` | B12.0 hashes |
| `block_mode_is_decided_by_the_ranks` | B12.1 list, both ways |
| `a_subgraph_takes_consecutive_rows`, `linked_blocks_never_share_a_row`, `sibling_blocks_never_interleave` | L5, L6 properties on generated graphs |
| `block_order_follows_declarations_until_it_costs_twice_the_minimum` | L3 on the analog (walk kept) and dense-td (net) |
| `block_order_survives_reordered_statements` | same bands on the five permutations |
| `a_ping_pong_level_stays_lanes` | L4, swimlane |
| `a_source_block_sits_beside_what_it_feeds` | GH beside EXT |
| `a_turned_edge_runs_up_its_channel_through_flow_sides` | E3: out of the top, into the bottom (TD), all four directions |
| `chains_keep_one_side_of_a_block` | no run through a foreign frame, static, on every block fixture |
| `level_order_is_deterministic_and_bounded` | O4: same SVG twice; start count under the budget |
| `a_hub_fan_leaves_in_target_order` | O6 |
| `a_spine_inside_a_block_is_straight`, `a_compact_block_keeps_its_brandes_koepf_columns` | X2 |
| `a_band_gap_holds_its_tracks_and_labels_between_the_frames` | G1 |
| `a_chain_label_sits_outside_both_end_blocks` | G3 |
| `sibling_frames_of_a_band_start_level_in_scene_and_route`, `frames_agree_between_scene_and_route_in_block_mode` | F2, F3 |
| `an_entry_under_a_title_jogs_in_beside_the_chip` | 2.8, TD and BT |
| `an_edge_to_a_block_ends_mid_side_clear_of_the_chip` | free pin |
| `a_subgraph_self_edge_is_a_loop_in_scene_and_route` | 2.8 |
| `route_takes_a_corridor_round_a_foreign_block`, `corridors_nest_and_do_not_jump_on_a_one_px_drag`, `a_nested_edge_crosses_each_frame_on_a_flow_side_in_route` | R3, R5 |
| `route_and_scene_agree_on_which_edges_run_upstream` | R2 |
| `dragging_every_node_of_a_block_diagram_keeps_the_contract` | B12.8 |
| `a_direction_block_inside_a_bordered_block_keeps_its_direction` | B9 inside block mode |
| `layout_clustered_returns_blocks` | public API doc example |

### 4.5 Pinned tests, gates and images: what may change and why

**Must pass as written** (a failure is a regression, not a re-baseline): every test of `compound.rs`;
`assert_clean_clusters` users; the own-frame tests; `render.rs` advanced tests (295-357; trigger-unchanged);
`a_composite_edge_to_its_own_state_drops_straight_from_the_border`;
`note_links_hold_their_ends_and_corners_in_scene_route_and_drag`; `a_static_canvas_is_its_drawing_with_even_margins`;
`a_hub_fed_row_of_shallow_frames_wraps_into_two_rows`; `hub_fed_leaves_in_a_box_pack_into_a_grid`;
`largest_box_is_centred_among_its_siblings`; `route_partial_*`; `edge_picking_matches_the_painted_strokes_on_every_fixture`;
`marquee_selects_an_edge_once_its_painted_stroke_touches`; `moving_a_node_declaration_barely_moves_dense_td`;
`edge_line_order_barely_moves_the_dense_layouts`; `every_fixture_has_a_ceiling` (the new fixtures need rows).

**Intended changes** (say each in its commit message):

| test or gate | change | why |
|---|---|---|
| `layout::a_box_seats_its_loose_parts_on_its_top_layer` | restated | it pins a `seat_box_sources` case in which a loose member sits level with the hub that feeds its box through another member; a block lies wholly below its feeder |
| `tests/quality.rs` `CEILINGS` rows of bench-75, dense-lr, dense-td | ratchet **down** | edge / foreign-frame pairs fall (static 46 -> at most 4 over the fixtures); `defects` may not rise |
| `DEFECT_BASELINE` (`x_drawn` 428 / 494, `band_len`, sideways-run fields) | re-measured, each value at or below the old one | the three dense fixtures are re-laid. `x_drawn` may not rise: that is the crossing must |
| `BORDER_LEN_MAX`, `CHANNEL_MAX`, `EDGE_CHIP_MAX`, `XGAP_MAX`, `LBL_EDGE_MAX` | re-measured; a higher value needs a named cause and a picture | positions change; the prototypes' routed values (border length 4530, chips 8) were `route()` without corridors, i.e. regressions |
| `GAP_FLOORS`, `DRAG_JUMPS_MAX`, `INK_MEAN_MIN`, `MAX_ASPECT`, hard zeros | **not loosened** | they are the invariants |
| `casing::a_short_line_keeps_at_most_one_gap`, `casing::a_line_takes_one_role_across_a_cluster_of_crossings`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route` | re-read; update the fixture-specific numbers if the rule each states still holds | they count crossings or measure a leg on a dense fixture that is re-laid |
| new gates | block shape per fixture (`compact_max`, `span_max`, `sib_side_linked`, `sib_stagger`), `lateral_x`, `spine_off`, `fr_max`, `ends_apart`, the analog's row | no gate sees these today |

**README images.** `docs/img/compare/{advanced, state, subgraph}.png` and `docs/ex-subgraph.svg` do not change (trigger).
Add one comparison image for a block diagram (the analog if D5 allows). Re-render the benchmark renders of bench-75,
dense-lr, dense-td.

### 4.6 Perf budgets (PLAN invariant 1; `tools/perf.sh`, two runs before calling a regression)

| file | `scene_ms` limit (2 x baseline) | `route_ms` limit | worst `drag_ms` limit |
|---|---|---|---|
| advanced | 1.42 | 1.72 | 2.61 |
| subgraph | 0.16 (D1) | 1.05 | 2.04 |
| fanout-frames | 4.38 | 1.34 | 3.44 |
| dense-lr | 46.8 | 8.03 | 17.76 |
| dense-td | 50.68 | 9.20 | 17.80 |
| bench-75 | 61.94 | 15.76 | 25.28 |
| analog (new; baseline = 0.38.0: 6.6 / 1.9 / 5.8) | 13.2 | 2.9 | 7.25 |

`route_partial` under 6 ms a frame on every file (README publishes "about 3 ms, 6 ms at worst" for bench-75). The flat
corpus files keep their numbers. Limits are max(1.25 x baseline, baseline + 1.0 ms) for `route_ms` and `drag_ms`.

### 4.7 Docs

- **README**: Subgraphs section (0.38: lines 282-289): the block rule in three sentences (consecutive rows; linked
  blocks stack along the flow; edges between blocks run in the gaps and cross frames on flow-facing sides), the `direction`
  rule next to it (unchanged), when a drawing is left as it was (the trigger, the lane rule), `flowchart-elk` (accepted,
  same as the default). How it works (797-806): Layers, Ordering, Coordinates, Narrowing. Guarantees (811-817): add "no
  edge through a block that holds neither end (static)". Benchmark rows of the clustered inputs (825-851) and the drag
  numbers (857), re-measured.
- **CHANGELOG** under `## [Unreleased]`: Changed (blocks are the default for flowcharts and state diagrams with
  subgraphs; upstream edges between blocks run in channels; `layout_clustered` returns block layouts), Fixed (subgraph
  self-edge drawn as a loop; frames no longer snap on the first drag in block mode), with the analog's numbers.
- Doc comments on every new `pub(crate)` item that states a rule, in the code base's style (why, constants by name).

## 5. Out of scope, and the split

Out of scope for B12:

- Generalising `compound.rs` to bordered subgraphs (`d-recursive`): per-block pipelines, pins, stitching.
- Honouring `direction` in a subgraph that has a border edge (mermaid ignores it too; B9's rule stays).
- Title-clear entries and flow-facing ends on the trigger-unchanged path (those drawings stay exactly as 0.38 draws
  them, lateral entries under a title included).
- Aligning a short block inside a tall band by its pins (mermaid's R5 beyond "sources sit last").
- Reaching mermaid's 16 crossings on the analog (a target, not a must), and one lane per edge at 20 px pitch (mermaid
  pays 5.18 Mpx2 for it).
- Styling subgraphs (`style`, `class` on a subgraph are still dropped), class and ER diagrams, the `advance` module.

**Split: yes, in two packages, cut after B12.8.**

- **B12a "blocks: structure and routing"** = B12.0-B12.8 (checkpoints A and R), merged with the internal switch off:
  no output changes, no release note, every existing test passes as written, and the block path is reviewed on what it
  can already be judged on (structure, crossings, invariants, budgets).
- **B12b "blocks: edges, labels, default on"** = B12.9-B12.14 (checkpoint C): acceptance, ratchets, docs, the release.

Reason: 9 of 1047 library tests and 7 of 13 quality tests fail on the best prototype and nearly every ratchet moves; one
review cannot tell an intended re-baseline from a regression unless there is a point where nothing visible has changed
yet. B12a alone must not ship as the default. If the workflow keeps one package, the three checkpoints are its review
rounds and the switch is removed in B12.14 all the same.

## 6. The judges' blocking flaws and where each is resolved

| # | flaw (judges) | resolution |
|---|---|---|
| 1 | crossings rise; the block transpose is unprototyped (all three) | transpose tested and rejected; 2.4 level-first ordering with starts and chains, evidence in appendix A (analog 20); B12.4 with a stop rule |
| 2 | edges enter frames through lateral walls and run along title strips; rule 1.6 cannot work for a first-row target; "ports" was the wrong stage (visual, engineering) | 2.8: sides set by the builders, title-clear entries with a jog track, X2.2 breadth; gate `lateral_x`; B12.9 |
| 3 | labels and gap room; struck labels on v2 and the permutations; room measured row to slot (all three) | 2.6 G1-G3; label rules gated on v2 and the permutations too; B12.6 |
| 4 | own-frame containment breaks inside blocks (visual) | wide-frames is trigger-unchanged; X2 keeps inner runs between the walls; F3; the four tests unchanged plus spread-face and nested-cases; B12.5 |
| 5 | straight spines lost; packing discards Brandes-Köpf; alignment rule dead (visual, engineering) | 2.5 X1, X2.3, X2.4 (keep when compact, seeded squeeze, rigid spines); gate `spine_off`; B12.5 |
| 6 | block order without a safety net, 21 of 47 edges upstream on dense-td; no bound (all three) | L3: two walks, exact minimum beyond `2M + 1`; B12.2 check |
| 7 | no rule for ping-pong levels (visual, invariants) | L4; fixture; constant from the sweep |
| 8 | partially overlapping unlinked siblings (engineering) | L5 no interleaving |
| 9 | `route()` and drag: budgets, drag jumps, routed gaps, corridors unbuilt and root-only, scene and route disagree on more edges (all three) | 2.9 R1-R7, per level, on `lanes::plan`; B12.8 before polish, with a stop rule; `ends_apart` |
| 10 | upstream lanes run round whole bands (engineering, visual) | E3: chains through flow-facing sides in scene and route |
| 11 | state composites snap more on the first drag; frames differ between scene and route (invariants) | F1-F3: one frame function; gate `fr_max` |
| 12 | diagrams that need no change move; state fixtures; static time on tiny files (engineering, invariants) | 2.2 trigger, also in `route()`; D1 |
| 13 | the layout search goes idle or changes regime; `layout_clustered` changes silently (engineering) | L7, O5, B12.12; doc comment and CHANGELOG |
| 14 | state diagrams under-specified; the sink rule for lead notes untested (engineering, invariants) | L2 (cards keep 0.38's offset; no sink rule); B12.11 review round with named tests and new fixtures |
| 15 | which pinned tests are re-baselined and which are regressions (all three) | 4.5 |
| 16 | block shape leftovers: frame tops 17 px apart, PROD too wide, dense-lr L-shaped (visual) | F2; X2; B12.12 |
| 17 | cannot be verified as one step (engineering) | 4.1 checkpoints; section 5 split |
| 18 | routed canvas margin test fails unexplained; RL not run; B11's comb untested with blocks; prototype numbers of two modes mixed (engineering, invariants) | R7 and B12.8; all four directions in the fixtures; B12.10; every number in this spec names its mode |
| 19 | shared-code guards; flat byte identity unpinned (invariants) | 3.4; B12.0 hash test |
| 20 | `d-hybrid`'s stale back flags (engineering) | L6 |

## Appendix A. The experiment behind 2.4 (run for this spec)

Tree `B/spec/tree` = `B/d-global/tree` + `B/spec/experiment.patch` (layout.rs only). Build: `cd B/spec/tree &&
CARGO_TARGET_DIR=B/target-x cargo build --release --bin flowmaid --example quality` (the build dir used here is
deleted). Inputs `B/spec/in` (the 8 metric inputs, v2, five permutations), structure files `B/spec/struct`, runner
`B/spec/measure.sh <label> ENV=...`. Tables: `B/spec/evidence/metrics.*.tsv` (block-metrics), `quality.*.tsv`
(flowmaid's harness, static and routed).

Switches: `FM_B12` 0 off / 1 lanes / 2 chains (as `d-global`); `FM_B12_BT=1` sibling-block swap judged by the layered
count; `FM_B12_FORCE="a,b"` force a swap with pinned keys; `FM_B12_LVL=1` level-first order of the root (O2) with slot
keys (O3), both half-rounds judged; `FM_B12_STARTS=k` (O4); `FM_B12_SEED=n`; `FM_B12_TRIG=1` the trigger;
`FM_B12_DUMP=1` layers, per-gap crossings, orders.

Findings:

1. **Block transpose does nothing.** Lanes: the search swaps Code hosting and External services (layered 21 -> 19) and
   the drawn count goes 40 -> 42; swapping the two bottom blocks raises the layered count (24) and is refused. Forced
   with pinned keys: 63 (lanes), 62 (chains). The chains beside PROD do not change sides with their block.
2. **Why the flat sweeps fail**: the loop judges only after the upward half, so every chain ends where its target is.
   With chains the analog ends at 42 layered crossings from each of 60 shuffled starts; in the written order 39 of them
   lie in the one gap under External services.
3. **Level-first order of the root** (collapsed cost 6-8), from the written order alone: analog 40 -> 29-36 (lanes),
   43 -> 36-37 (chains), depending on how the inner rounds are judged. Not enough without starts.
4. **With starts** (7 give the same result as 16 on all 14 inputs): the table in 2.4. The winning analog order is
   External services [DNS GW TG OAI AZ] left of Code hosting, all of DNS's chains left of PROD, STANDBY left of
   DEV/STAGING: mermaid's arrangement. Same band order under all five permutations.
5. **The layered count predicts the drawn count only with chains** (23 -> 20 and 25; with lanes 11 -> 24 and 34).
6. `judge` on that build: 9 must-fails over 14 inputs: dense-td crossings (112 > 90) and, on the analog,
   `sib_stagger`, `compact_max` 5.72, `lbl_off_max` 0.364, `lbl_crowd` 9, `lbl_gap_min` 0, `lbl_struck` 3: the pieces
   2.5-2.7 own. Hard zeros 0 in both modes on the 8 harness inputs; edge / foreign-frame pairs static 47 -> 1, routed
   47 -> 11 (unchanged `route()`).
7. The trigger on `d-global`'s layering leaves the same eight inputs byte-identical as `d-hybrid`'s.
8. An entry under a title chip goes round to the member's side in 0.38 already (`B/spec/evidence/title-entry.fm038.png`).

## Appendix B. Decisions only the user can take

- **D1 Static time on tiny clustered files.** `subgraph.mmd` is at its limit (0.16 ms); the trigger adds one linear pass.
  Proposed: for files under 1 ms the static limit is max(2 x baseline, baseline + 0.3 ms). Without it B12 must first
  make the 0.38 path faster.
- **D2 Split** into B12a (no visible change) and B12b (default on, release), or one package with three review rounds.
- **D3 Upstream edges between blocks as channels.** They leave the top of a node and enter the bottom of a block
  (mermaid's look) instead of flowmaid's side lane. Back-edges between two plain nodes keep the lane.
- **D4 Lane rule.** A level where more than a third of the links must run against the flow stays as 0.38 draws it.
- **D5 The analog as a public fixture and README image.** It is sanitized, but derived from a private diagram.
- **D6 If a stop rule fires** (a crossing or perf gate cannot be met), the choice between a looser gate and more work is
  the user's.
