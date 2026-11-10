# B12 "subgraphs as blocks" — specification

Date 2026-11-10. Written for an implementer who has seen none of the design work.

- `B` below is `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12`.
- Every `file:line` points into `B/tree/src` (dev = release 0.38.0, commit 0366084). **You will work on top of package
  B11 (crossing hops in the SVG painter, comb bundling for fans) and a class/state leftovers package, so find code by
  the function names given here; the line numbers only say where that function was in 0.38.0.**
- Read `scratchpad/PLAN.md` first. Its invariants are not negotiable.
- `B/private/` holds the user's two screenshots. Look at them, never copy them or any name or address in them anywhere
  else. Everything written down uses the sanitized analog `B/in/infra-overview.mmd` and its ids (U1, DNS, EXT, PROD
  ...).
- **The analog is derived from a private diagram and is not cleared for the public repository (decision D5).** The
  analog, `infra-overview-v2`, the five permutations and their direction copies are *local acceptance inputs*: they are
  judged from the workspace and never committed. The repository's fixture and README image use the synthetic diagram
  `B/revise/in/blocks-overview.mmd` (called *the public fixture* below), which has the same kinds of difficulty and a
  different structure.
- The user's answers to the open questions are in `B/USER-DECISIONS.md`; appendix B applies them.
- Background, in the order worth reading: `B/understand-flowmaid.md` (today's pipeline), `B/understand-mermaid.md` (what
  mermaid's ELK does, rules R1-R10), `B/metrics/README.md` (the yardstick), `B/design-global.md` (the design this spec
  builds), `B/design-hybrid.md` and `B/design-recursive.md` (source of the grafts).

## 0. Decisions in one page

1. **Design built: `d-global`**, the unanimous winner of the three judges: one Sugiyama pass over all nodes, one scene,
   one `edgepass::finish`, `compound.rs` (B9) untouched; blocks come from a block-aware layering, a block-aware ordering
   and a hierarchical breadth assignment. No edge is ever laid out in two halves, so the scene contract holds without a
   stitching step.
2. **Departures from `d-global`, each on evidence** (appendix A):
   - **Ordering is level-first with several starts, not "flat sweeps plus a block transpose".** The judges asked for the
     transpose to be shown to reach 24 crossings on the analog. It does not: swapping two sibling blocks, judged by the
     layered crossing count, changed nothing useful (40 -> 42 drawn), and forcing the swap gave 62-63. What works is
     ordering each level's units first (blocks and passing chains as units) and then the members inside, from several
     deterministic starts: **analog 20 drawn crossings (0.38: 24, d-global prototype: 40, mermaid: 16)**, but only from
     the fifth start on (one start: 37) and at 23 times the static time limit in the experiment's naive code.
   - **An upstream edge between blocks is a channel through the flow-facing sides, not a back-edge lane** (decision D3).
     With flat ordering lanes looked better (40 against 43), which is why `d-global` chose them. With level-first
     ordering chains win (20 against 34), because only then is every drawn edge in the model the ordering minimises: the
     layered count predicts the drawn count with chains (23 -> 20) and does not with lanes (11 -> 24 or 34). It is also
     mermaid's look (returns run straight up between the blocks).
   - **Layering is `d-hybrid`'s `block_rank`**: node-level cycle breakers kept, block order by two declaration-order
     walks with an exact-minimum safety net, no interleaving of siblings, the lane rule; plus one fix (back flags are
     recomputed from the final layers).
   - **Block mode is decided by structure, not by comparing ranks** (2.2, decision D7): it is on whenever a link joins a
     block to anything outside it. `d-hybrid`'s trigger ("a diagram whose block layers equal its 0.38 layers is drawn as
     before") left 21 of 36 framed inputs on the 0.38 path, among them plain block diagrams (a hub and five blocks, two
     blocks in a cycle) whose 0.38 drawing is less tidy than mermaid's.
3. **The bar is mermaid 12.2 itself** (the user, 2026-11-10: "pokoknya mirip atau lebih bagus dan rapi dari mermaid js
   12"): on the analog family the acceptance numbers of 4.3 are stated against mermaid's, each with its tolerance, not
   against 0.38's.
4. **What is proven and what is not.**
   - Proven by prototype or experiment: the bands, compact frames, no run through a foreign frame, the block-order
     safety net.
   - Shown to exist, not shown within budget: the crossing numbers of the analog family (20-25 from seven shuffled
     starts at about 300 ms a scene; the limit is 13.2 ms).
   - **Not built by anyone yet**: the canonical start order and the key-order tie-break (2.4), the coordinate rule of
     2.5, gap and label room (2.6), the frame growth of 2.7, flow-facing ends, free pins and title-clear entries (2.8),
     the corridors in `route()` (2.9), level-first ordering below the root level, block mode on the inputs whose rows
     0.38 already stacks (4.3 e). No build has met these musts of 4.3: `lbl_crowd` 0 (best: 8), crossings <= 17 on v2
     (best: 20), <= 24 on every permutation (perm-b: 25).
   Section 4 therefore has checkpoints with stop rules: if a checkpoint's gate cannot be met, stop and report; a gate
   is never re-baselined to pass (decision D6).

## 1. Goal, the user's words, before and after, what must not change

### 1.1 Goal

User feedback on 0.38.0 (2026-11-10): the same private architecture diagram (a `flowchart TD`, 3 top-level actors, 5
subgraphs, 27 edges, most labelled, 18 across a subgraph border, 3 from a node to a whole subgraph) is "still far
tidier" in mermaid 12.2. Quality bar in the user's words: **"tidak main-main dan premium"**; rare cases matter. Asked
what the acceptance bar is, the user answered: it must look like mermaid 12 or better and tidier.

User decisions: package B12 "subgraphs as blocks"; **the default for every flowchart and state diagram with subgraphs**
(not only `flowchart-elk`); diagrams without subgraphs must not change; it runs after B11 and the class/state leftovers.

What a block layout means here. It holds for every diagram **drawn in block mode**: every flowchart or state diagram in
which a link joins a subgraph, or something inside it, to something outside it (2.2). A diagram whose subgraphs are all
isolated has nothing for these rules to do and is drawn as before.

- every non-empty subgraph (and state composite) is **one compact block**: its members take consecutive rows along the
  flow, and the frame is as large as its content;
- two blocks an edge joins are **stacked along the flow**, never side by side; unlinked blocks may share a band;
- an edge between blocks **runs in the gaps between blocks**, crosses a frame only through a flow-facing side, and never
  runs through a block that holds neither end;
- labels sit on the part of their edge outside the end blocks, with room reserved, **clear of foreign lines**.

One level is exempt from the first two rules: a level the lane rule (2.3 L4) leaves side by side, because every
stacking order would turn more than a third of its links against the flow.

On labels: the complaint names "labels far from both ends" in a corridor of eight parallel lines. Distance to the ends
is not what separates the two engines (mermaid's labels are farther from their ends: 558 px against 309 on the analog);
**crowding** is (0.38: 8 of 20 chips have a foreign line within 12 px; mermaid: none). So the label goal is: no chip
struck, no chip crowded, and the distance to the ends not above 0.38's by more than a tenth (4.3).

### 1.2 Before and after

| picture | path |
|---|---|
| 0.38.0 (the complaint) | `B/infra-overview.fm038.png` |
| mermaid 12.2 default (ELK) | `B/metrics/mermaid-svg/infra-overview.mermaid.png` |
| `d-global` prototype (flat ordering, lanes) | `B/d-global/infra-overview.proto-lanes.png` |
| spec experiment (level-first ordering, chains, 7 starts) | `B/spec/evidence/infra-overview.lvl-chains-7starts.png` |
| the same, real diagram's structure (replication edge turned) | `B/spec/evidence/infra-overview-v2.lvl-chains-7starts.png` |
| unchanged `route()` at the experiment's centres | `B/spec/evidence/infra-overview.lvl-chains.routed.png` |
| public fixture: 0.38.0, mermaid, spec experiment | `B/revise/fixture/blocks-overview.{fm038,mermaid,exp}.png` |
| a hub and five blocks: 0.38.0, mermaid | `B/critic/out/hub5.038.png`, `B/critic/mm/hub5.mermaid.png` |
| two blocks in a cycle: 0.38.0, mermaid | `B/critic/out/cycle2.038.png`, `B/critic/mm/cycle2.mermaid.png` |

The analog differs from the real diagram in one edge (`PG --> DRAPP`; the real one runs from the standby stack to the
database). `B/u-mermaid/in/infra-overview-v2.mmd` has it turned; with it the three servers stack one under another as in
the user's mermaid screenshot. **Both files are acceptance inputs.**

Target picture: actors / External services + Code hosting / PROD / the two other servers (v2: STANDBY / PROD /
DEV-STAGING), DNS at the end of its block that faces its corridor, the hub's lines down one corridor beside PROD,
returns to the chat bot straight up between blocks.

Numbers on the analog (block-metrics, `B/metrics/`; the full acceptance table is 4.3):

| metric | 0.38.0 | mermaid | `d-global` proto | spec experiment | B12 must |
|---|---|---|---|---|---|
| canvas | 2787x1225 | 2199x2358 | 2208x1281 | 2544x1281 | area <= 1.25 x 0.38's |
| frame compactness mean / max | 6.90 / 11.43 | 3.25 / 4.59 | 3.43 / 5.09 | 3.56 / 5.72 | <= 3.9 / <= 5.5 |
| largest frame share of the flow extent | 0.767 | 0.308 | 0.314 | 0.314 | <= 0.37 |
| side-by-side pairs joined by an edge | 5 | 0 | 0 | 0 | 0 |
| interleaved pairs (`sib_stagger`) | 3 | 0 | 1 | 1 | 0 |
| runs through a foreign frame | 3 (2012 px) | 0 | 0 | 0 | 0 |
| crossings | 24 | 16 | 40 | **20** | <= 20 |
| edge length | 22350 | 30835 | 20356 | 20488 | <= mermaid's |
| labels struck / crowded | 0 / 8 | 0 / 0 | 0 / 9 | 3 / 9 | 0 / 0 |

What the experiment's picture still gets wrong, and which step owns it: PROD is wider than its content (2.5 X2); the two
bottom frames start 17 px apart (2.5 X4); labels sit on lines in the corridors, three are struck (2.6); six edges,
`U1 -> DNS` and `DNS -> PSDEV` among them, cross a frame through a lateral side, 7 crossings in all (2.8; 0.38: 21 on
14 edges); the three node -> block edges end 14, 14 and 32 px from a frame corner (2.8 free pins); the dotted
`PSPROD -> TG` goes round the standby block (2.4, ordering below the root; 2.8 E3).

The public fixture (`B/revise/in/blocks-overview.mmd`: two actors, seven subgraphs of which one is nested, 21 nodes, 30
edges, a shared-services block that feeds the clusters below it and takes their returns, six edges to or from a whole
subgraph) shows the same defect in 0.38 and the same remedy:

| metric | 0.38.0 | mermaid | spec experiment | B12 must |
|---|---|---|---|---|
| canvas | 1542x1864 | 1206x2309 | 944x1753 | area <= mermaid's |
| frame compactness mean / max | 9.02 / 26.52 | 3.86 / 6.64 | 4.37 / 6.20 | <= 4.63 / <= 7.97 |
| largest frame share of the flow extent | 0.902 | 0.183 | 0.198 | <= 0.22 |
| side-by-side pairs joined by an edge | 4 | 0 | 0 | 0 |
| `sib_stagger` | 0 | 0 | 1 | 0 |
| runs through a foreign frame | 2 (672 px) | 0 | 0 | 0 |
| frame crossings through a lateral side (`B/critic/lateral.py`) | 14 on 11 edges | 0 (ELK's rule) | 2 | 0 |
| crossings | 14 | 5 | 7 | <= 9 |
| edge length | 17204 | 20125 | 15038 | <= mermaid's |
| labels struck / crowded | 0 / 5 | 0 / 0 | 0 / 4 | 0 / 0 |

### 1.3 What must not change

1. **Diagrams without subgraphs, and class / ER / sequence and every other diagram type: byte-identical SVG.** Every
   line B12 adds to shared code sits behind "clustered". There is no SVG snapshot today, so B12.0 adds a hash test over
   every flat corpus input.
2. **A diagram with subgraphs that is not in block mode (2.2: no link joins a block to anything outside it):
   byte-identical static SVG**, unless it holds one of the degenerate own-box edges of 2.8, which are fixed for every
   clustered diagram. Today that is `examples/state.mmd` (its composite is a B9 direction block) and the probes
   `B/u-mermaid/probes/{p01-iso, p02-iso-dir, p09-disconnected, p13-inner-dir-ext}.mmd`. Their `route()` changes in one
   respect only: the frames are grown by the function the static scene uses (2.7 F3), so they no longer snap on the
   first drag.
3. PLAN invariants 1-7: the interactive API and its speed (`scene`, `scene_sized`, `route`, `route_sized`,
   `route_partial`, hit-testing), the scene geometry contract (first `Graph::edges.len()` scene edges index-parallel,
   sub-edges after, frames outermost first, waypoint semantics), `fold::scene_compact`, `to_mermaid`, the `advance`
   module, determinism, zero dependencies, MSRV 1.75, wasm32, portable SVG.
4. B9: a subgraph with its own `direction`, an inner edge and no border edge is laid out by `compound.rs` exactly as
   today; one with a border edge takes the direction round it.
5. What flowmaid already does better than mermaid (PLAN invariant 4): titles never struck, white label chips, compact
   canvas, straight spines (inside a block and across a border), source order as the tie-break (2.4 O4b), edges into a
   subgraph ending on the frame clear of the chip, stable output under re-ordering of edge statements (2.4, 2.11).
6. Public API: `layout::{layout_clustered, layout_sized, LayoutResult, Placed}` keep their signatures.
   `layout_clustered` returns block layouts in block mode; say so in its doc comment and in the CHANGELOG.
7. **Diagrams 0.38 already draws as stacked blocks are not drawn worse.** Block mode re-lays them (2.2); 4.3 (e) gates
   each of them against its 0.38 drawing, picture by picture.

## 2. The algorithm

All rules are stated in flow coordinates: *flow axis* = y for TD/BT, x for LR/RL; *breadth* = the other axis;
*upstream / downstream side* = the two sides of a box across the flow axis; *lateral sides* = the other two. A rule
that differs per direction says so.

### 2.1 Terms

- **Level**: the root, or a non-empty subgraph (a subgraph with at least one node at any depth). An empty subgraph is
  no level, no unit, has no frame, and its sub-edges are skipped, as today.
- **Unit** of a level: a node whose innermost subgraph is that level, or a non-empty subgraph whose parent is that level
  (a *block unit*). A B9 direction block has been cut out by `compound::split` before any of this runs and appears as a
  plain proxy node.
- **Key** of a unit (declaration order): a node's index in `Graph::nodes`, i.e. the order of first mention; a block's =
  the least node index inside it. Keys are distinct.
- **Link**: a real edge, a stand-in edge for a sub-edge (`scene_clustered` builds one per sub-edge), an invisible link,
  or one `from x to` pair of a rank-only group (`sub_edge_proxies`, `grid_pairs`, narrowing's stackings and wraps).
  Self-loops are no links. A link is *drawn* when it is a real edge or a stand-in (weight 1); a rank-only pair weighs 0.
- **Augmented graph**: the graph with one stand-in edge appended per sub-edge, as `scene_clustered` builds it today
  (its `ga`). It is built by one constructor that the three entry points share (2.2).
- A link `u -> v` **belongs to** the level of the lowest common ancestor of `u` and `v` and joins there the two units
  `cu`, `cv` that hold `u` and `v`. A **border link** is a real edge (visible or invisible) or a stand-in whose end
  nodes have different cluster paths; then `cu != cv` and at least one of them is a block: the link crosses, or ends
  on, at least one frame. A drawn border link is an **inter-unit edge with a block end**.
- **Rows** of a unit: 1 for a node, for a block the number of layers of its own level.
- **Band**: a maximal run of a level's layers covered by overlapping units.
- **Run**: the dummies of one chain that have the same cluster path on consecutive layers (the chain passing one level
  beside that level's blocks, or descending inside one block).
- **Title side** of a block: the side its title strip lies on (screen top: the upstream side in TD, the downstream side
  in BT, a lateral side in LR/RL). **Title row**: the members of the block, at any depth, from which the title side is
  reached along the flow without passing another member.

### 2.2 Block mode

**Block mode is on iff the augmented graph has a border link.** It is a pure predicate of the graph: no ranking, no
sizes, no positions. `levels::block_mode` computes it from the output of `levels::augmented(g)`, the one constructor of
stand-ins and rank-only groups, and `scene_clustered`, `route_built` and `route_partial_with` all take both from there,
so the three entry points cannot disagree. A B9 part (a block's inside, or the rest with its proxies) decides for
itself when it goes through `scene_labelled`.

- **On**: every step of this section applies, to every candidate of `narrow`, and `route()` / `route_partial()` run
  2.9.
- **Off**: the whole call runs the pre-B12 path (every candidate of `narrow`, ordering, coordinates, layer lines,
  `scene_clustered`, the edge stages, `route()`, `route_partial()`), with two exceptions that hold for every clustered
  diagram: the frame growth of 2.7 F3 and the degenerate own-box edges of 2.8.

Why structure and not a comparison of ranks (decision D7). `d-hybrid`'s trigger kept the 0.38 path whenever the block
layers equalled the 0.38 layers. Measured on the experiment binary it leaves 21 of the 36 framed inputs of
`B/metrics/extra/in` byte-identical (`B/critic/sweep/trigger.txt`), and with them 0.38's edges: `hub5` keeps 5 crossings
of a lateral frame wall on 4 edges (mermaid: 0), `cycle2` keeps 6 on all 4 inter-block edges with the returns as side
lanes (mermaid: channels, 0), `p06-block-cycle` 2, `p03`, `p07` and `notes/spread-bottom` 1 each; and one added edge
flips a drawing between two edge styles. That is not "the default for every diagram with subgraphs", and it is below
the user's bar.

What it costs: the inputs whose rows 0.38 already stacks (*rows-equal inputs*: `examples/{advanced, subgraph}.mmd`,
`quality/fanout-frames.mmd`, `notes/{near-drag, spread-bottom, straight-link, wide-frames}.mmd`, the probes p03-p08,
p10-p12, `hub5`, `cycle2`) are re-laid by O, X, G and E. They are not automatically as good as before: in the
experiment's block mode `advanced.mmd` gains a lateral exit and a longer lane (`B/revise/out/advanced.{038,blk}.png`),
`subgraph.mmd` is the same picture (`B/revise/out/subgraph.{038,blk}.png`). 4.3 (e) gates each of them against 0.38,
and the two with no static timing headroom (`subgraph.mmd`, `advanced.mmd`) against 4.6.

### 2.3 Step L: layering (replaces steps 3-5 of `rank` in block mode)

`rank` today: (1) the hub graph of the rank-only groups, (2) node-level cycle breakers, (3) longest path, (4)
`seat_box_sources`, (5) lead cards one layer before their target. Block mode keeps 1-2 (L0) and replaces 3-5.

**L0. Node-level back-edges stay.** `rank` marks back-edges with its two DFS walks exactly as today. A link marked there
is used reversed in everything below.

**L1. Level graphs**, deepest level first, the root last: the level's units sorted by key, and its links collapsed to
unit pairs (`cu != cv`; parallel links between one pair are one link with their summed weight).

**L2. Note links.** A card is a member of the box the parser puts it in and takes a row there like any member (0.38:
`notes/near-drag`'s card `in301` is the last row of composite `K0`). So a link with a note-card end
(`style::is_note_card`) constrains rows in L5 like any link. It is never turned (L3 skips it; a card has one link, so
it lies on no cycle) and is not counted among the drawn links of L4. After L5, a lead card (`Graph::lead_nodes`, a
`note left of`) takes `start(target) - 1` inside its level when that is later: step 5 of `rank`, applied per level.
There is no clamp.

**L3. Block order: which links run against the flow.** Only for a level whose unit graph has a cycle.

1. Walk A: DFS over the units; roots = units without an incoming link in key order, then every unit in key order; a
   unit's links are followed in key order of their targets, then link index. A link into a unit still on the stack is
   *turned*.
2. Walk B: the same DFS with the roots in plain key order.
3. The walk that turns fewer drawn links is taken; walk A on a tie.
4. Safety net. Let `M` be the least number of drawn links any order of the units turns. It is the sum over the strongly
   connected components of the level's unit graph (a link between two components is on no cycle). A component of at
   most `BLOCK_ORDER_EXACT_MAX` = 12 units is solved exactly (dynamic programme over subsets; ties to the
   lexicographically smallest key sequence); a larger one, and every component of a drawing past `WALK_MAX_ELEMENTS`,
   contributes the taken walk's own count, an upper bound. If the taken walk turns more than `2 * M + 1`, every
   component that was solved exactly takes its minimal order; the others keep the walk.

Reason for a threshold instead of "always the minimum": on the analog the minimum (3) puts PROD above External services
and sends `DNS -> ingress` upwards; the walk turns 5 and gives the order the author wrote and mermaid draws. On dense-td
the single walk of `d-global` turns 21 of 47 edges; with the net 5. Counting per component keeps the net on a level
with many units: the root of `examples/state.mmd` has exactly 12 units, and one more state would have dropped a
per-level cap.

The rule reads the first-mention order of the nodes (the key). It is independent of the order of edge statements when
the nodes are declared before the edges, as in every block fixture. When nodes are first mentioned in edge lines the
key follows those lines: `B/critic/in/cycle2-tie.mmd` and `cycle2-tie-swapped.mmd` differ only in the order of two edge
lines written before the subgraph blocks and stack Front and Back in opposite orders, as 0.38 does. A fixture documents
it.

**L4. Lane rule.** If `M` is more than `LANE_SHARE` = 1/3 of the level's drawn links (note links not counted), no
stacking order is good. Then nothing is turned at this level, and its direct child blocks are *transparent* for
layering: their members, and the child blocks of those blocks as units, are ranked as units of this level. Two
swimlanes with `a1 -> b1 -> a2 -> b2 -> a3 -> b3` keep every arrow forward and their frames side by side, five rows
tall, as 0.38 draws them (`B/critic/out/swimlane.038.png`); mermaid and `d-global`'s prototype stack the two blocks and
point 2 of 5 arrows up (`B/critic/mm/swimlane.mermaid.png`, `B/critic/out/swimlane.exp.png`). Only the layering of
such a level is 0.38's. The rest of block mode still runs on it: its blocks share layers, so O2 keeps them apart across
the flow, the edges between them take lateral sides (2.8: the two units overlap along the flow), X and G apply. The
level does not meet the block-shape rules, by design; its gate is "not worse than 0.38" (4.3 e). Where a component was
too large to solve, `M` holds its walk's count, so the rule errs towards 0.38's layering. No corpus input triggers it.
The constant is pinned by the sweep of B12.13.

**L5. Rows of a level**: longest path (Kahn, queue seeded by key) with `start(cv) >= start(cu) + rows(cu)` for every
link not turned (note links included), and with the ends swapped for a turned one. Then two refinements, **repeated in
this order until neither changes anything**:

1. *A source block sits next to what it feeds*: a block with no constraint into it starts at
   `min(start of its successors) - rows` when that is later, and keeps that start as a lower bound from then on. Plain
   source nodes stay on the first row, as today.
2. *No interleaving*: while two sibling **blocks** `A`, `B` overlap in rows without one containing the other
   (`start(A) < start(B) < end(A) < end(B)`), add `start(B) >= end(A)` and solve again (first such pair in key order;
   the constraint points from the earlier start to the later, so it closes no cycle).

Each pass only adds a constraint or raises a lower bound, so starts never decrease, they are bounded by the level's
total rows, and the loop ends. (Run once and in the other order the two rules leave the pattern they forbid:
`B/critic/in/interleave.mmd`, where pushing T below a source block S and then sinking S gives T rows 2-4 and S rows
4-5.) Then the lead cards of L2.

`rows(level) = max(start + rows)`.

**L6. Absolute layers.** `layer(node) = sum of the starts of its enclosing blocks + its own start`. A rank-only hub gets
`max(layer of its unturned sources) + 1`. Then, for every drawn edge, **`back[e] = layer(to) < layer(from)`** (this
replaces the flags of L0; `d-hybrid` kept the old flags, so an edge marked back that ended up pointing downstream got no
chain and was drawn as one curve across several layers).

Properties, by construction, at every level L4 did not fire on: a block's members fill exactly its interval; two units
joined by a link have disjoint intervals; no edge joins two nodes of one layer; sibling blocks never interleave.

**L7. Later re-rankings are fenced in block mode.** `tighten_sources` and `stagger_proposals` may not move a node out of
its block's interval nor change a block's rows (one interval test per proposal). `stack_candidates` and
`wrap_candidates` keep working: their groups are rank-only links, i.e. constraints of their level.

### 2.4 Step O: ordering (in `place`, block mode)

**O0. Chains.** Every drawn edge spanning more than one layer gets a dummy chain, **downstream and upstream**, except an
invisible link and an upstream edge whose two units are both plain nodes of one level (class E4 of 2.8: that one keeps
today's back-edge lane, package B3). An upstream chain is built from the lower layer to the higher and stored in
from -> to order, as `place` already does for a descending edge. An upstream edge with a block end that spans a single
layer has no dummy and is no lane either (2.8 E3). Chain dummies take cluster paths by today's `apath` rule (the end's
path where the layer lies in that end's interval, else the common prefix), so a chain passes a foreign band at the
level of the common ancestor and enters each frame on its chain through a flow-facing side.

**O1. Units of a level, per layer**: its direct nodes, its child blocks (on every layer of their interval), and its
runs. Blocks and runs are *long units*: they exist on several layers.

**The canonical order.** On every layer: the units of the root by key, a block's members contiguous and ordered the
same way inside it, a chain dummy keyed by (key of its edge's source node, key of its target node, edge index).
Adjacency lists are sorted by the neighbour's canonical position before the first sweep, and every tie in O2-O4b falls
back to the canonical position. Nothing in this step reads the order of the edge statements (two parallel edges of
one node pair excepted). This is what `edge_line_order_barely_moves_the_dense_layouts` needs: the experiment started
from the written order and dense-td came out 3829 px wide in source order and 2986-2989 on four of the test's six
shuffles (-22 %, limit 10 %), with identical layers.

**O2. Level order, top-down, the root first.** For level `L` build the collapsed layered graph: one vertex per (unit,
layer); every link of the augmented graph between two layers of `L`, collapsed to units, weighted by its multiplicity;
links inside one child block dropped. For a level other than the root the layers just outside it are fixed neighbours
with the positions the parent gave them. Order it by weighted-barycentre sweeps (down, then up) with a weighted adjacent
exchange after each half; **judge after each half**, keep the least weighted crossings, earlier on a tie; at most 12
rounds, from two starts (the start order, each layer reversed).

Two properties hold as **constraints**, not as costs (a weight can be outbid: one run changing sides of a block cost
1000 in the experiment, less than five crossings of two 15-edge bundles, and dense-lr kept one run through a foreign
frame):

- *one ordinal per child block*: two child blocks that share a layer are in the same order on every layer they share;
- *one side per (run, block)*: a run is on the same side of a block on every layer they share.

The simplest way to hold both is to keep one sequence of the level's long units (a long unit's sweep key is the
weighted mean of its barycentres over its layers), take each layer's order as that sequence restricted to the layer,
and merge the direct nodes in by their own barycentres. Both properties are asserted after O2 (a debug assertion, and
the gate `xframe_runs`).

Result per layer: an ordinal for every child block and a *slot* (the number of blocks before it) for every direct node
and run.

**O3. Inside the slots.** Today's sweeps (`reorder`, `transpose`, then regrouping) run over the whole augmented graph,
with `arrange` sorting the units of each level by (slot or ordinal from O2, own position): blocks stay where O2 put
them, nodes and runs are free inside their slot, members are free inside their block. Regroup after every layer's
reorder, judge after the downward half and after the upward half, up to 16 rounds, stop at 0. (Today's loop judges only
after the upward half; on the analog that parks every chain where its target is and leaves 39 crossings in one gap.)

**O4. Starts.** O2 + O3 run from `BLOCK_ORDER_STARTS` starts: start 0 is the canonical order; start `k > 0` shuffles
the canonical order of every layer with a fixed linear congruential generator seeded with `k` (integer arithmetic only:
deterministic on every platform). The start with the fewest layered crossings (`count_crossings`) wins, the lowest
index on a tie. `BLOCK_ORDER_STARTS` = 7 (ELK's default thoroughness), reduced by a work budget:
`starts = clamp(BLOCK_ORDER_WORK / (augmented nodes + links), 1, 7)` for the first placement of a `layout_core` call;
every later candidate of `narrow` runs one start, the winner's (a candidate moves a few layers, not the problem); one
start past `SEARCH_MAX_ELEMENTS`. **The crossing musts of 4.3 must hold with the starts this budget grants, inside the
time limits of 4.6** (B12.4).

**O4b. Source order among equals.** A shuffled start leaves the members that change no crossing wherever the shuffle
put them (analog, seven starts: External services reads `DNS GW TG OAI AZ` or `GW DNS TG OAI AZ` depending on the base
seed, with 23 layered crossings either way; GW has no edge and is declared first). After the winning start, two
adjacent units of one level in one slot (two members of a block, two nodes or runs of a slot) swap when the left one is
later in the canonical order and the weighted layered crossing count does not rise; repeat to a fixed point. Each swap
removes one inversion against the canonical order, so it ends.

**O5.** Bar cards are placed as today. `centre_largest` does not reorder the blocks of a level O2 ordered.

**O6. Fans.** The chains leaving one node side are in the order of their first dummies, so a hub's fan (DNS: six edges)
does not cross itself; `ports::assign` orders the ends by heading as today. Acceptance counts crossings between two
edges of one node side: 0 on the analog and on the public fixture.

Evidence (drawn crossings, block-metrics; full tables in appendix A):

| ordering | analog | v2 | perms a-e | bench-75 | dense-td | dense-lr |
|---|---|---|---|---|---|---|
| 0.38.0 | 24 | 19 | 12 19 24 24 34 | 302 | 80 | 46 |
| flat sweeps, lanes (`d-global`) | 40 | 23 | 36 47 40 48 43 | 335 | 126 | 66 |
| flat sweeps, chains | 43 | 29 | 46 43 43 43 35 | 373 | 122 | 55 |
| level-first root + 7 starts, lanes | 34 | 28 | 34 34 34 34 37 | 314 | 101 | 36 |
| **level-first root + 7 starts, chains** | **20** | **20** | **22 25 20 22 22** | **295** | 112 | 41 |
| the same with 1 start | 37 | 20 | 37 32 37 42 39 | | | |
| mermaid | 16 | 13 | 7 19 16 19 18 | 228 | 68 | 10 |

The experiment ordered only the root level, started from the written order, and used `d-global`'s single walk for the
block order. The analog reaches 20 only from the fifth start (1-2 starts: 37; 3-4: 36; 5 and 7: 20). dense-td (7 nested
frames, 21 turned edges under that walk) is the input that needs O2 on every level and L3's safety net; its must is 90.
mermaid's numbers on the permutations come with a scrambled band order (perm-a puts the actors mid-drawing).

### 2.5 Step X: coordinates across the flow (block mode)

**X1. Brandes-Köpf: no alignment between two chain dummies of different cluster paths.** In `coordinates_bk` such a
pair is not offered as an alignment link (the prototyped rule: `d-global/prototype.patch`, the `coordinates_bk` hunk).
A chain is straight inside each block and beside each band and bends where it crosses a border, i.e. in a gap. A real
node may align with its neighbour across a border, real or dummy: in the experiment `R1 -> ARGO` is straight under this
rule, and PLAN invariant 4 asks for it.

**X2. Each block is fixed on its own, deepest first.** Items: its direct nodes, inner runs, two walls, child blocks
(already fixed, rigid).

1. `W_min`: longest path over the left-of relation of every layer with `pair_sep` as distances (the block's least
   breadth for this order).
2. Wall-to-wall breadth is at least the title chip's breadth plus `CHIP_CLEAR` (TD/BT), plus `EDGE_GAP` per displaced
   entry (2.7 F2), so nothing foreign ever lies under a title that is wider than the content. The frame itself stays
   the box of the members (2.7).
3. If the breadth Brandes-Köpf gave the block is within `BLOCK_SLACK` = 4 px of that: keep its coordinates.
4. Else squeeze, seeded by Brandes-Köpf: merge every *spine* (two items of the block on adjacent layers that are each
   other's only inner link in that direction) into one rigid column; clamp each unit's Brandes-Köpf offset into its
   feasible interval for breadth `W_min` (left-packed to right-packed position), left to right so the order holds; then
   `COMPACT_SWEEPS` sweeps moving each unit to the weighted median of its links **inside the block** (weights as
   `compact_breadth`: 1 / 2 / 8), within its neighbours.
5. An inner run takes a free column between two members when the order gives it one. A run that ends up beside the
   outermost member lies outside the member box; the frame then grows over it (2.7 F3), as 0.38's frames do.

**X3. A fixed block is one rigid unit of its parent level**, which is treated the same way (keep Brandes-Köpf when
compact, else squeeze), up to the root. A block as a whole is pulled toward the median of its border links. A *border
spine* (an adjacent-layer border link whose two ends are each other's only link on that side) stays straight: X1 lets
Brandes-Köpf align it, and in the squeeze it is merged into one rigid column at the parent's level, the block moving as
a whole, when the block's feasible interval allows.

**X4. Along the flow** a node is centred on its layer line, with one exception that levels sibling frames: on a layer
that is the first row of two or more sibling blocks of one band, the first-row members of each block (at any depth)
move upstream, as one group per block, until the block's frame leading edge (as 2.7 F1-F2 compute it: member edge, pad,
own title strip, jog room) coincides with the earliest of them. The same holds downstream on a layer that is the last
row of two or more sibling blocks which do not share their first row (a source block ends on its band's last layer by
L5). The shift is a layout coordinate, not a frame rule: frames stay functions of their own members in all three entry
points. It needs no extra room, because `reach_of` already reserves the largest reach on that layer (2.6 G1). It fixes
`sib_stagger` (the analog's two bottom frames start 17 px apart: their first-row nodes differ in height).

`d-global`'s prototype starts X2 from the left-packed positions and so discards Brandes-Köpf: chains inside a composite
stagger, its alignment rule changes no byte. X2.3, X2.4, X3's border spines and X4 are the fix, **specified, not
prototyped**; B12.5 gates them with a new measure: an adjacent-layer edge whose two ends are each other's only link on
that side is straight, inside a block and across a border (`spine_off` = 0).

### 2.6 Step G: layer lines, gap room, label slots (block mode)

**G1. Room between frames.** Where a frame closes on layer `li - 1` or opens on layer `li`:
`lcoord[li] >= lcoord[li-1] + reach_of(li-1, 1) + need(li) + reach_of(li, 0)`, with `need` the largest of `track_need`,
the label rows of `gap_need`, `cross_need` and the labelled-gap room of `short_labels`. `reach_of` is measured to the
frame as 2.7 draws it: pad, title strip, and the growth of F2 (jog room; in LR/RL the title length). Today the room is
measured from the row above to the slot below and ignores the frame pad and the title strip (the layer-line section of
`place`), which is why the prototypes crush the band gaps (bench-75: lines 0.5 px apart). F2's growth depends on the
members' breadth positions, so `lextra` and `reach_of` take it after step X and before the layer lines.

**G2. Tracks.** `track_need` already counts every sideways step in a gap, chain bends included; the bends X1 puts at
block borders are such steps. One `TRACK_PITCH` per overlapping step. Steps on one side going the same way nest (the
rule `ortho` has since B10).

**G3. Label slot.** A chain's labelled dummy is the middle dummy of the chain's run **at the edge's common-ancestor
level** (outside both end blocks), chosen after `apath` is known. When that run is empty the label takes the border gap
by the `short_labels` rule, measured from the frames. An adjacent-band labelled edge keeps `short_labels`. A label never
sits inside an end block or on a frame border. A slot is clear of foreign lines: the labelled dummy reserves its chip
plus `CLEAR_GAP` on each breadth side (in `pair_sep`), and a label row in a gap is a row of its own in `gap_need`, with
no foreign track within `CLEAR_GAP` of the chip along the flow. What the reservation has to achieve is the gate
`lbl_crowd` of 4.3 (no prototype has it: 8-9 crowded chips on the analog).

**G4.** None of this runs outside block mode. Wider gaps are paid for by the canvas: the area caps of 4.3 apply.

### 2.7 Step F: frames

A frame is a function of **its own members' positions, its title, and the graph**, in every entry point. No frame rule
reads another block's members, and only F3's inner legs read where an edge comes from (2.9 R6 covers that case).

**F1. Box.** `cluster_raw_boxes` of the member positions (pad, title strip on the screen top).

**F2. Growth along the flow (block mode).**

- *Jog room.* For a member `m` of the title row let `border(m)` be the number of border links with exactly one end in
  the block, that end being `m`, and `cap(m)` the number of ports that fit on `m`'s title-facing side outside the
  chip's breadth widened by `CHIP_CLEAR`, at `PORT_SPACING`. A block has `k = sum over the title row of
  max(0, border(m) - cap(m))` *displaced entries*. With `k > 0` its frame grows on the title side by
  `ROW_CLEAR + (k - 1) * TRACK_PITCH`: the room between the title strip and the title row in which 2.8's entries jog.
  `k` is counted from the block's own members and the graph, not from where the far ends lie, so dragging a far end
  never changes a frame; it may reserve a track for a link that leaves through the other side.
- *Title length (LR/RL).* The title runs along the flow there. A frame is at least as long along the flow as its title
  chip plus `2 * TITLE_INSET`; a shorter one grows equally on both flow sides. (In the experiment a one-column block
  titled "External services with a long title" is cut to "External s…": `B/critic/out/lr.exp.png`; 0.38 shows it
  whole.)

Both are part of the shared frame function, and the static layout reserves the same amounts (G1). In `route()` a
growth stops `FRAME_GAP` short of whatever lies beyond that side; when the jog room cannot grow, the displaced entries
take the lateral side and count in the routed `lateral_x`; when the title length cannot, the title is cut as today.

**F3. Growth across the flow is one function, for every clustered diagram.** Today only the static scene widens a
frame over the channels laid inside it, so frames snap on the first pixel of a drag (71 px on the analog, 617 on
bench-75). The static widening in `scene_clustered` and `route_lane_clusters` become one function of the frames and the
drawn paths. A frame grows across the flow to cover, plus `LANE_PAD`:

- the drawn paths of the edges with both ends in it, the lanes of its own back-edges and their labels (as today);
- the inner legs of every edge with exactly one end in it: in `scene()` the chain waypoints the layout placed in that
  box (what 0.38's widening covers: "a channel lane can sit just outside the outermost member node"), in `route()` the
  inner legs of 2.9 R3;
- in block mode, the border-crossing columns of its displaced entries.

Then `stop_at_siblings`, `renest_boxes`. Outside block mode the function returns exactly today's static frames in the
static scene (the hash test of B12.0 proves it). An edge between two members stays inside their frame in `scene()`,
`route()` and on a drag (existing tests, unchanged).

### 2.8 Step E: how each kind of edge is drawn

| kind | static scene | `route()` |
|---|---|---|
| E1 downstream, ends in different units, more than one layer apart | dummy chain (O0): out through the downstream side of every frame round the source, beside the blocks of the bands between, in through the upstream side of every frame round the target, down inside the target block beside its members | corridor (2.9 R3) |
| E2 downstream, adjacent layers | no dummy; `ortho` draws the Z in the band gap, sized by G1 | today's channel when nothing lies between, else corridor |
| E3 **upstream, at least one end unit a block**, more than one layer apart | dummy chain the other way: out of the source's **upstream** side, in through the target's **downstream** side | corridor, mirrored |
| E3, adjacent layers (the analog's `ARGO -> TG`) | no dummy; drawn as E2 mirrored: out of the upstream side, the Z in the band gap, into the downstream side; **never a lane** | the same |
| E4 upstream, both units plain nodes of one level | today's lane (`back_lanes`, `static_lanes`) | today's `route_backs` |
| node -> whole subgraph, subgraph -> subgraph | the stand-in's chain; the end on a frame is a *free pin* on its flow-facing side | the same against the current frames |
| member -> its own subgraph | an edge to the block's entry member (`own_box_targets`), drawn whole; a border link when the entry member lies in a nested block | unchanged |
| invisible link | ranks (L), orders, never a chain, never drawn | unchanged |
| note link | never turned (L2), never a lane, as today | unchanged |

Every "is this a lane?" test in the code asks "is it E4?", not "does it have a chain?".

**Degenerate own-box edges, in every clustered diagram** (block mode or not; 0.38 draws all three through members or
as nothing):

| kind | rule, in `scene()` and `route()` |
|---|---|
| subgraph -> itself | a loop on the block's frame, drawn like a node's self-loop with the frame as the box (0.38: a zero-length path, `M 84.0 400.0 C 84.0 400.0, ...` in `B/critic/out/self-member.038.svg`) |
| subgraph -> its own member | the stub `own_box_stub` keeps today's side when its way to the member passes no other member (`a_composite_edge_to_its_own_state_drops_straight_from_the_border` is that case); else it takes the first frame side, in the order upstream, the lateral side nearer the member, downstream, from which the member is reached without passing one (0.38 runs `S --> b` from the bottom border through member `c`) |
| subgraph <-> its own nested subgraph | no link for L; a stub between the two borders on a side where no member lies between them (0.38: two straight strokes through member `store`, `B/critic/out/sub-nested-self.038.png`) |

**Flow-facing ends (E1-E3).** The end on a member of a block the edge leaves or enters is on the member's side that
faces the border it crosses. The side is set where the builder sets it (`scene_from_layout`'s anchors for chains; the
corridor builder in `route_built`), and `ortho` and the router keep it; `ports::assign` only spaces the ends of a side.
A lateral side is allowed only when the two units overlap along the flow (a host can drag them side by side; an L4
level has them so by layout).

**Free pins.** The chain of an edge to a whole block aims at a proxy member and would be cut wherever it happens to
cross the border (experiment: 14, 14 and 32 px from a frame corner). Instead the chain is cut at its last dummy outside
the block and the builder sets the end at the pin: alone on its side, the middle of the stretch of the flow-facing side
that is free of the title chip; among other entries of that side, the middle of the free interval between its
neighbours in O3's order, at least `PORT_SPACING` from them and from a corner. `ortho` draws the last gap step.
`route()` places the pin by the same rule against the current frame.

**Title-clear entries.** The title side of a block is crossed in TD by the entries of downstream edges and the exits
of upstream ones, in BT the other way round; in LR/RL the strip is on a lateral side, the rule is void, and pins stay
out of its breadth interval. An edge that would cross the title side within the chip's breadth plus `CHIP_CLEAR`:

1. keeps clear by its port when it can: the member's ports on that side are placed outside the chip's interval while
   the side has room (`cap(m)` of F2);
2. else is *displaced*: it crosses the border at the nearest free column beyond the chip, in the order of the entries,
   and jogs to its port on a track in the jog room of F2. The jog lies **inside the frame**, between the title strip
   and the title row. X2.2 reserves the breadth for the crossing columns, and when the member box is narrower the frame
   grows across the flow to the last displaced entry's column plus `LANE_PAD` (F3).

This replaces `d-global` 1.6, which bounded a chain dummy that does not exist when the target is on the block's first
row. Today, and in every prototype, such an edge goes round to the member's side
(`B/spec/evidence/title-entry.fm038.png`); on the analog TR1, TR2 and PSDEV sit under their chips.

**Labels**: G3.

### 2.9 `route()`, `route_sized()`, `route_partial()`

The host gives positions, so no layering, ordering or coordinates run. What runs, in block mode (2.2):

- **R1. Frames**: F1-F3, the same functions as the static scene.
- **R2. Classes by geometry and the level tree.** For an edge whose units `cu != cv` at its common-ancestor level are
  not both plain nodes: let `S`, `T` be the boxes of `cu`, `cv` (a block's frame or the node). `T` wholly downstream of
  `S` by `FLOW_MIN_GAP` -> E1/E2; wholly upstream -> E3; overlapping along the flow -> today's builders with lateral
  sides. Both plain nodes -> today's rules (`route_backs` for E4). At the scene's own centres this reproduces the static
  classes, because L6 defines `back` by the layers.
- **R3. Corridors, per level, not only at the root.** Obstacles of an edge = the other units of its common-ancestor
  level whose flow interval meets the open interval between `S` and `T`, and, inside each end block, that block's other
  units. If today's 4-point channel (`cross_cluster_route`) meets none, keep it. Else the path is a polyline with **one
  breadth per band it passes and one per end block it descends in**, bending in the gaps like the static chain:
  - *bands*: the obstacles grouped into maximal runs that overlap along the flow, from the current boxes;
  - *per band*: the corridor is one of the band's gaps across the flow (between two neighbouring obstacles, or outside
    the outermost) wide enough for its slot; which one is R5's rule; the *slot* is the edge's rank by edge index among
    the edges passing that band in that gap; breadth = the gap's nearer edge + `LANE_PAD` + slot x `TRACK_PITCH`;
  - *inner legs*: inside an end block the path is planned at that block's level, from the border crossing to the member
    (or to the next frame inwards), obstacles = that block's other units, by the same per-band rule; a free column
    inside the block is preferred to one beside its outermost member (F3 then has nothing to grow);
  - *ends*: out through the flow-facing side at the port's column (title rule of 2.8), across in the first free gap,
    along each corridor, across in the last free gap, in through the flow-facing side.

  `lanes::plan` is **not** used for placement: it orders by band length and a geometric "far" measure and re-deals
  breadths among the lanes of a side, and a `Lane` has one breadth (lanes.rs:616-642, 689-721, 793-850). Its stub and
  nesting geometry may be reused. Emit bare on-axes corners so `ortho_pass` keeps the path and only rounds it; no
  change to `ortho`'s static-only columns.
- **R4. Hard obstacles.** For these edges a block that is on neither end's chain is a hard obstacle in
  `router::repair` (today a foreign frame is only priced), unless its frame overlaps either end's unit box: a host may
  put a node on a foreign frame, and that frame is then priced as today.
- **R5. Stability under a 1 px move.** The corridor of a band is the wide-enough gap nearest the breadth of the
  previous leg (the source port's column for the first band), the lower breadth on a tie; slots go by edge index.
  Within one choice of gaps every coordinate of a corridor is a continuous function of the frames. The choice itself
  changes only where two gaps are equally near, and on a drag only for the edges of the moved node: a kept edge keeps
  its path (R6).
- **R6. `route_partial`** keeps its logic: `clusters_line_up` holds (frames are functions of their own members, F1-F2);
  a frame is redrawn as `route()` draws it when it holds a moved node **or is an end block of a border link on a moved
  node** (its inner leg, and with it F3's growth, may have moved; an unchanged box costs nothing, as today); an edge on
  a moved node or a regrown frame is redrawn by the same builders; a kept edge keeps its base path, also a corridor
  that passes the moved block unless the block now touches it; `PARTIAL_ROUTE_SHARE` unchanged; no new state in the
  base scene.
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
| member -> own subgraph / subgraph -> own member / subgraph -> itself / subgraph <-> own nested subgraph | 2.8 |
| cycle between blocks | L3; turned links are E3; every level has at most `2M + 1` |
| two-block cycle `A <-> B` | when it is a node cycle, L0's node-level breakers decide and L3 never runs (`cycle2`: Front stays first, as in mermaid); otherwise the block with the lower key is first unless that turns more than `2M + 1` drawn links: with one link down, 2 or 3 links up keep declaration order, 4 flip it (`B/critic/in/ratio{2,3,4}.mmd`) |
| more than a third of a level's links against any order | L4: that level's child blocks are ranked as 0.38 ranks them and stay side by side; lateral sides between them; gate "not worse than 0.38" |
| hub linked to many blocks | no layering rule; its block sits where L5 puts it; its chains share the corridor O2 chose and leave in fan order (O6); B11's comb applies to the fan (4.2 B12.10) |
| empty subgraph; subgraph of empty subgraphs | no unit, no frame, sub-edges skipped, as today |
| subgraph with one member | a block of one row |
| members without inner links | first row of their block; `grid_pairs` still packs loose members into rows by rank-only pairs, which are inner links |
| unlinked blocks | all start on the level's first row, side by side in key order; `narrow` may stack or wrap them as today |
| no border link at all | not block mode (2.2): the 0.38 path |
| own `direction`, inner edge, no border edge (B9) | unchanged: `compound::split` cuts it out; the proxy is a plain unit; each part decides block mode for itself, so a B9 block inside a bordered block sits in that block's rows as one unit |
| own `direction` with a border edge | the direction is not honoured (mermaid's rule, README), an ordinary block |
| LR / RL / BT | layers run along the flow in every direction; title strip handled where it is today (`lextra`, `reach`, `header_b` in `pair_sep`); title-clear entries and the title length per 2.7 F2 and 2.8; tests run all four directions |
| state composites | a composite is a block; its `[*]` states and note cards are members; a transition to a composite is a sub-edge |
| note cards, lead notes | L2: a card takes a row of its block; a card beside a fork/join bar keeps `bar_cards` ordering |
| fork / join bars | `fit_bars` runs before the layout, unchanged; a bar whose branches lie in different blocks is a plain unit of their common level and its branches are chains |
| invisible links | links of weight 1 for L3 (the author asked for that order), border links for 2.2, never chains |
| node self-loops | unchanged (`loop_reach` room in the separations) |
| nodes first mentioned in edge lines | the key follows those lines (L3); documented by the `cycle2-tie` pair |
| large drawings | L is linear per level plus the subset programme (<= 4096 x 144 steps per component of <= 12 units); O4 is budgeted; past `SEARCH_MAX_ELEMENTS` one start and no narrowing as today; past `ROUTE_MAX_EDGES` (400 scene edges) `ortho` and the router are skipped as today and blocks, chains and corridors still hold, because they come from the layout and the builders |
| class, ER, `advance`, flat flowcharts | never clustered: untouched |

### 2.11 Tie-breaks and determinism

| decision | rule |
|---|---|
| unit order | key; never equal |
| DFS roots, DFS links | L3 |
| fewer turned | walk A on a tie; subset programme: lexicographically smallest key sequence |
| longest path | Kahn, queue seeded by key, FIFO |
| source blocks, no-interleave pair | L5's order; first pair in key order |
| start order of a layer | canonical (2.4): units by key, chain dummies by (key of the source node, key of the target node, edge index) |
| level order (O2) | fewer weighted crossings; earlier half-round; start order before "reversed"; ties inside a sweep by canonical position |
| starts (O4) | fewer layered crossings; lower start index |
| crossing-neutral neighbours (O4b) | canonical order |
| squeeze (X2) | fixed sweep count and directions; `weighted_median` takes the lower point on a tie |
| free pin | middle of the free interval; O3's order among entries |
| corridor of a band (R5) | nearest wide-enough gap to the previous leg; lower breadth on a tie; slots by edge index |
| containers | `Vec` and `BTreeMap` wherever iteration order can reach the output; no float is used as a map key |

Byte-identical SVG across runs is asserted for every block-mode fixture (the existing determinism checks cover them once
they are fixtures).

### 2.12 New constants (name them, explain them in a comment, as the code base does)

| constant | value | pinned by |
|---|---|---|
| `BLOCK_ORDER_EXACT_MAX` | 12 units per strongly connected component | cost bound |
| turned-link threshold | `2 * M + 1` | analog (5 against 3 keeps the walk), dense-td (21 against 4 switches), `ratio{2,3,4}`; sweep B12.13 |
| `LANE_SHARE` | 1/3 | sweep B12.13 |
| `BLOCK_ORDER_STARTS`, `BLOCK_ORDER_WORK` | 7, measured | experiment (7 = 16 on all 14 inputs; fewer than 5 misses the analog); `perf.sh`; the table of B12.4 |
| `BLOCK_SLACK` | 4 px | B12.5 |
| `CHIP_CLEAR`, `CLEAR_GAP`, `PORT_SPACING`, `ROW_CLEAR`, `TRACK_PITCH`, `LANE_PAD`, `FRAME_GAP`, `TITLE_INSET` | existing constants | |

## 3. Code plan

Prototype code to start from (hacks with env switches; none is mergeable as is):

| piece | where | state |
|---|---|---|
| L0-L3, L5, source blocks, hub layers | `B/d-hybrid/prototype.patch`, `fn block_rank` (tree copy `B/d-hybrid/tree`) | works; treats note links as ordinary links (keep that: L2); lacks the per-component net, L4, L5's loop, the L6 back-flag fix |
| a shorter L1/L5/L6 | `B/d-global/prototype.patch`, `fn block_layers` | works; single walk, no safety net |
| X1, X2 without seeding | `B/d-global/prototype.patch`, the `coordinates_bk` hunk and `fn block_compact` | works; X2 to be changed per X2.3, X2.4 |
| upstream chains (O0), `back_lanes` filter | `B/d-global/prototype.patch` (mode 2), `d-hybrid` S7 | works |
| O2 at the root, O3, O4 on `d-global`'s layering | `B/spec/experiment.patch` (diff against `B/d-global/tree`; tree copy `B/spec/tree`; switches `FM_B12=2 FM_B12_LVL=1 FM_B12_STARTS=7`; prebuilt CLI `B/critic/fm-spec`) | works; **far too slow** (3.5), root level only, written start order, `TIE` weight instead of constraints |
| per-input structure dump, block-metrics, `judge` | `B/metrics/` | ready |
| invariants probe (frames and edge ends, scene against route; drag jumps with sub-edges) | `B/j-invariants/b12inv.rs` | ready; turn into tests |
| lateral frame crossings from an SVG | `B/critic/lateral.py` | ready; the reference for `lateral_x` |

### 3.1 By file

**`src/levels.rs` (new, small, pure).** `augmented(g)`: the stand-in edges and rank-only groups, moved out of
`scene_clustered` so that `route_built` and `route_partial_with` build the same ones. `block_mode` (2.2). The level
tree from the cluster paths (`node_cluster`, as `layout` gets them, so the public `layout_clustered` works too): units
and keys per level, each link's common-ancestor level and unit pair, each node's chain of blocks, the title row of a
block. Used by `layout::block_rank`, by the ordering, by `scene::route_built` (R2, R3), by the frame function and by
the new quality measures.

**`src/layout.rs`**

| function (0.38 line) | change |
|---|---|
| `struct Ranking` (2267), `rank` (2281) | its five steps stay as they are for the non-block path. In block mode `rank` runs steps 1-2 (L0), then new `block_rank` (L1-L6), and returns its layers and back flags. Without `node_cluster` not one line changes |
| `layout_core` (1545), `struct Opts` (1607) | take block mode from `levels::block_mode`; carry it in `Opts` to every `place` and to `narrow` |
| `narrow` (1656), `tighten_sources` (2840), `stagger_proposals` (2877) | L7 fences; candidates run one start (O4). `stack_candidates` (2157) / `wrap_candidates` (2042) unchanged; re-read `RIBBON_ENTRY` and the acceptance slacks of `narrow` on block layouts (B12.12) |
| `place` (3026), chain loop (the "invisible links and back-edges get no channel" branch) | O0: in block mode only E4 goes without a chain or a Z |
| `place`, label dummy sizing (3132-3147) | moves after `apath` is built (3292-3341); G3, with the `CLEAR_GAP` reservation |
| `place`, ordering (3343-3397), `cluster_keys` (5487), `arrange` (5534), `transpose` (4750), `count_crossings` (4805) | block mode: the canonical order, new `level_order` (O2, long units), the multi-start loop (O3, O4), O4b; `arrange` takes slot / ordinal keys per layer. Flat and non-block clustered diagrams keep today's loop byte for byte (do not "improve" the half-round judging there) |
| `place`, `centre_largest` hint (3431-3439; `centre_largest` 1936) | O5 |
| `coordinates_bk` (4880) | X1, behind block mode |
| after `coordinates_bk` (where `opts.compact` calls `compact_breadth`, 5201) | block mode: new `block_compact` (X2, X3). `compact_breadth` stays for narrowing outside block mode |
| `place`, layer-line section (4062-4175), `reach_of` (3608), `track_need` (4002), `gap_need` (3878), `short_labels` (3630), `lextra` (3192) | G1-G3; F2's growth in `lextra` / `reach_of`, taken after step X; X4's shifts along the flow |
| `back_lanes` (4427) | block mode: only an E4 edge is a lane; every E3 edge, with or without a chain, counts among the forward skeletons. Guard it with block mode (`d-global`'s filter is unguarded) |
| `drawn_crossings` (4629), `drawn_breadth` | E3 edges are drawn like chains: no bow. PLAN invariant 6: the back-edge bow rule lives in `scene.rs` and here; change both or neither |
| `layout_clustered` (1514, public) | doc comment: returns a block layout when 2.2 says so |

`seat_box_sources` (2497) stays: it is step 4 of the non-block ranking.

**`src/scene.rs`**

| function (0.38 line) | change |
|---|---|
| `scene_labelled` (3050), `scene_flat` (3083), `sub_edge_proxies` (3994), `grid_pairs` (4172), `clusters_line_up` (5277), `compound` hooks | none |
| `scene_clustered` (3444) | stand-ins from `levels::augmented`; frames (step g, 3535-3599) through the shared frame function |
| `scene_from_layout` (3159) | E1-E3 anchors on flow-facing sides (it already draws a descending chain: the `a_bottom` branch at 3279-3285); the adjacent-layer E3 as a mirrored Z; title-clear ports and displaced entries; free pins; label on the G3 dummy |
| `static_lanes` (3097) | E4 only in block mode |
| sub-edge drawing (3608-3743): `cut_to_frames` (3772), `own_box_stub` (3832), `sub_edge_ported` (3889), `free_edge` (8541) | free pins (the chain is cut at its last outside dummy); the three degenerate own-box rules of 2.8, for every clustered diagram |
| `cluster_raw_boxes` (5346), `route_clusters` (5389), `route_lane_clusters` (5404), `stop_at_siblings` (5481), `renest_boxes` (5524) | one frame function for the three entry points: F1, F2 (block mode), F3 (every clustered diagram) |
| `route_built` (4435) | `levels::augmented` and `block_mode` once per call; R2 classes; a corridor builder beside `cross_cluster_route` (8400) per R3; `route_backs` (8097) for E4 only in block mode; free pins; the degenerate own-box rules |
| `route_partial_with` (4889) | R6: the frame loop at 4980-4995 also redraws the end blocks of a moved node's border links; no new state in the base scene |
| `title_head` (2885), `fit_cluster_title` (2902) | read by the title rules; unchanged |

**Other files**

- `src/lanes.rs`: `plan` (696) and `Want` (659) stay as they are and keep serving E4. A `corridor_geometry` for R3
  beside the lane geometry may reuse the stub and nesting helpers.
- `src/ortho.rs`: no rule change intended. Verify, with tests, that `ortho_pass` (477) keeps an on-axes builder path and
  only rounds it (the rule at 716-727), keeps the end sides the builder set, and that `plan_jog` (1562) puts a
  band-gap step between the two frames (G1 gives it the room).
- `src/router.rs`: `repair` (5120): R4.
- `src/ports.rs`: ports of a title-row member's title side keep out of the chip's interval (2.8). `d-hybrid`'s
  prototype had `shared_ports` 2 on dense-lr once turned edges left a hub by its upstream side; the hard zero must hold,
  fix the spacing if it does not.
- `src/edgepass.rs`: `ctx` (126) keeps the geometric `back`; every consumer that treats a back-edge as a lane (find them
  by `cx.back`) must ask "E4?" in block mode. `finish_held` (361) unchanged.
- `src/compound.rs`: untouched; its tests are the proof.
- `src/quality.rs`: new measures appended to `Metrics::tail_fields` (so every earlier column keeps its number):
  `compact_max`, `span_max`, `sib_side_linked`, `sib_stagger` (as block-metrics defines them), `lateral_x` (frame
  crossings through a lateral side by an edge with exactly one end in the frame; reference `B/critic/lateral.py`),
  `up_lanes` (upstream edges with a block end that do not leave their source through its upstream side), `spine_off`
  (2.5), `fan_x` (O6), `pin_corner` (edge ends on a frame at or within `PORT_SPACING` of one of its corners; the
  experiment has 2 on the analog), `title_cut` (frame titles drawn with an ellipsis); and in `measure_source` (486),
  which has both drawings, `fr_max` (largest frame difference between `scene()` and `route(scene centres)`) and
  `ends_apart` (edges with an end more than 20 px apart between the two). `frame_side` (1649-1666) and `wiggles`
  (1626-1647) keep their definitions.

### 3.2 How the entry points stay consistent

| entry | block mode | what it shares with the others |
|---|---|---|
| `scene()`, `scene_sized()` | L, O, X, G, F, E; one scene, one `finish` | `levels::augmented`, `block_mode`, level tree, frame function |
| `route()`, `route_sized()` | R1-R5; no layout; host positions never moved | the same constructor and predicate, level tree, frame function, edge classes (R2 = L6 at the scene's centres) |
| `route_partial()` | R6 | same builders as `route()` for redrawn edges; base frames for blocks that hold no moved node and no end of a moved node's border link |
| hit-testing (`hit_test`, `node_at`, `edge_at`, `cluster_at`, `hits_in_rect`, `nearest_node`) | reads the `Scene` only | frames nest and siblings are disjoint (`pair_sep` + `cluster_gap`; `assert_clean_clusters`); one `SceneEdge` per edge, so `edge_at` follows the painted stroke (`edge_picking_matches_the_painted_strokes_on_every_fixture` runs on the new fixtures) |
| `fold::scene_compact` | refuses graphs with subgraphs (`FoldSkip::HasSubgraphs`), returns `scene()` | unchanged; assert it for a block-mode fixture |
| `to_mermaid`, json export | graph / scene only; no new field | round trip renders byte-identical |
| `advance`, class, ER | never clustered | flat hash test |
| `layout::layout_clustered` (public) | block layout | documented |

### 3.3 B11

B11's hops are drawn by the painter from finished scene edges and need nothing. Its comb bundles a fan's branches; a
hub's chains now leave one side in fan order and run parallel, which is the shape the comb wants, but **nobody has run
the two together**. B12.10 does, on the analog, the public fixture and bench-75. If the comb worsens a must (crossings,
`lbl_*`, `lane_gap_min`), restrict it to fans whose branches stay in one level and say so in the commit. block-metrics
counts a shared trunk once per edge and bridges hops before measuring (`B/metrics/README.md`).

### 3.4 Guards in shared code

Every new branch reads `blocks` (which implies clustered), except the two rules that apply to every clustered diagram
(F3, the degenerate own-box edges), which read `clustered`. No change to a line a flat graph executes: not the sweep
loop's judging, not `coordinates_bk`'s candidates, not `back_lanes`. B12.0's hash test is the proof.

### 3.5 Cost

- L: linear per level; block mode itself is one pass over the links. `subgraph.mmd` (static baseline 0.08 ms) and
  `advanced.mmd` are now drawn in block mode: measure them first (limits in 4.6; decision D1 gives the sub-millisecond
  files `baseline + 0.3 ms`).
- O: the experiment's implementation is naive (maps rebuilt per call, quadratic crossing counts, every layer regrouped
  after every reorder). Measured `scene_ms` with seven starts: **analog 310 (one start: 50-60, and then 36-37
  crossings), bench-75 1510, dense-td 648, dense-lr 763, fanout-frames 32**; 0.38: 6.5, 48.6, 24.9, 20.3, 2.9. The
  limits of 4.6 are 13.2, 61.9, 50.7, 46.8, 4.4. Required: collapsed graphs in index arrays; bilayer crossings by
  `inversions`; exchange by local deltas; regroup only the layer just reordered; starts budgeted by
  `BLOCK_ORDER_WORK`; one start per narrowing candidate. Target: ordering under 15 % of `scene_ms` on bench-75. The
  budget may not buy speed with crossings: B12.4 delivers the table crossings x starts (1..7) and shows the musts at
  the granted number.
- `route()`: unchanged `route()` at block positions is slower because the router repairs more (same binary, switch off
  -> on: analog 1.9 -> 4.0 ms, dense-td 6.2 -> 8.7, dense-lr 5.8 -> 7.9, bench-75 11.3 -> 9.2; `d-global` 4.7 and
  `d-hybrid` 5.0 on the analog; hand-placed blocks 4.8). **On the analog every one of these breaks the limit of 2.9 ms,
  and none has corridors.** Corridors give the edge stages clean paths; whether that brings the numbers inside the
  budgets is unmeasured, which is why a spike measures it right after B12.1 (4.1, B12.1s) and B12.8 comes before any
  polish.
- `levels::augmented` + `block_mode` run once per `route()` and per `route_partial()` frame: measure them in B12.8.

## 4. Work items, acceptance, tests, gates, budgets, docs

### 4.1 Checkpoints

Until C passes, everything B12 changes sits behind one internal switch that is off by default, so `dev` stays
releasable: with the switch off every output is byte-identical to the release before B12.

| checkpoint | items | gate | stop rule |
|---|---|---|---|
| S spike | B12.1s | `route_ms` of R2 + R3 at the experiment's block positions within 4.6 on the analog, bench-75 and dense-td, on two `perf.sh` runs | over a limit on both runs -> stop and report before B12.2 starts: the corridor design decides whether B12.2-B12.7 are worth building |
| A structure | B12.0-B12.7 | blocks, order, compact frames, gap room: the structural musts and the crossing musts of 4.3, **with the number of starts `BLOCK_ORDER_WORK` grants and `scene_ms` of the analog, the public fixture, bench-75, dense-td and dense-lr within 4.6**; both declaration-order tests pass as written; hard zeros 0 in the static scene; flat hash test green | after B12.4: crossings above the must on the analog (20), v2 (17), a permutation (24), the public fixture (9), dense-td (90), bench-75 (334) or dense-lr (52); or the musts need more starts than the budget grants; or dense-td under the test's six shuffles is outside 10 % in width or above its crossing must -> stop and report with the table crossings x starts |
| R route | B12.8 | every budget of 4.6; `DRAG_JUMPS_MAX` unchanged; routed hard zeros 0; `fr_max`, `ends_apart` | a budget broken on two `perf.sh` runs in a row -> stop and report |
| C acceptance | B12.9-B12.14 | `judge` reports 0 must-fails; every rule of 4.3; guard green | never flip the default with a must failing; a rows-equal input worse than 0.38 on a rule of 4.3 (e) that cannot be fixed -> stop and report (the user's fallback is option (a) of D7) |

### 4.2 Items

Each item ends with guard green (`tools/guard.sh`) and its own regression tests. Commit per item.

**B12.0 Baselines, fixtures, yardstick.**

- Hash test: SVG hashes, from the release B12 starts from, of every flat corpus input (`examples/*.mmd` without
  subgraphs, `tests/fixtures/quality/{back-edge, wide-labels}`, `tests/fixtures/outline/*`, class, ER, sequence and the
  other diagram examples) and of the clustered inputs that are not in block mode (`examples/state.mmd`, the probes
  p01-iso, p02-iso-dir, p09-disconnected, p13-inner-dir-ext); a file with a degenerate own-box edge is left out.
- Repository fixtures `tests/fixtures/blocks/`: the public fixture `B/revise/in/blocks-overview.mmd`; four
  permutations of it written here (edge statements reversed, subgraph blocks reversed, actors reversed, edge statements
  rotated) and its BT / LR / RL copies; from `B/critic/in`: `hub5`, `cycle2`, `ratio2`, `ratio3`, `ratio4`,
  `cycle2-tie`, `cycle2-tie-swapped`, `interleave`, `lr`, `nested`, `self-member`, `sub-nested-self`, `b9-inside`,
  `b9-inside2`, and the state diagrams `state-note`, `note-edge-row`; `B/d-hybrid/in/{swimlane, nested-cases}.mmd`;
  the probes `B/u-mermaid/probes/p*.mmd`; `B/spec/evidence/title-entry.mmd`.
- **Local acceptance inputs, never committed**: the analog, v2, the five permutations `B/u-mermaid/in/perm-*.mmd`,
  `B/critic/in/infra-{bt,lr}.mmd` and an RL copy. Keep them in a directory outside the repository and point
  `perf.sh` and `B/metrics/run.sh` at it. No file name, node label or number of theirs goes into the repository, the
  CHANGELOG or the README.
- Add the measures of 3.1 to `quality.rs`. Write the rows of 4.3 into `B/metrics/thresholds.json` (today's file holds
  earlier, looser bounds) and add `lateral_x` there from `B/critic/lateral.py`.
- Record from the starting release, for every block-mode fixture and local input, static and routed: the harness row,
  the block-metrics row, `lateral_x`, `up_lanes`, `fr_max`, `ends_apart`; and mermaid's block-metrics row where a
  render exists (`B/metrics/mermaid.tsv`, `B/metrics/extra/sweep-mermaid.tsv`, `B/critic/mm`, `B/revise/mmm`). These
  are the references of 4.3 (d) and (e). `perf.sh` baseline with the public fixture and the analog in its corpus;
  before renders.

*Check*: the hash test passes on the untouched tree; the new measures agree with block-metrics on the 8 metric inputs
(`compact_max`, `span_max`, `sib_*` within rounding) and with `lateral.py` on hub5 (5), cycle2 (6), the public fixture
(14) and spread-bottom (1: an LR diagram; `B/revise/lat2.py file.svg:h` runs the script with the flow axis horizontal).

**B12.1 Level tree and block mode.** `levels.rs` (`augmented`, `block_mode`, the tree); the internal switch (a
`pub(crate)` constant with a `#[cfg(test)]` override; `tests/quality.rs` and the examples reach it through one
`#[doc(hidden)]` function that B12.14 removes; no environment variable in the library).
*Check*: unit tests of the tree on nested, sub-edge, own-member and empty-subgraph graphs; `block_mode` false on every
flat input, on `examples/state.mmd` and on the probes p01, p02, p09, p13; true on the analog family, the public
fixture, bench-75, dense-lr, dense-td, fanout-frames, `examples/{advanced, subgraph}.mmd`, hub5, cycle2 and every
`notes` fixture with a transition or a note across a composite border; `scene_clustered`, `route_built` and
`route_partial_with` reach the same answer on every fixture (`block_mode_agrees_between_scene_and_route`).

**B12.1s Corridor spike (throwaway, not merged).** R2 + R3 + R5 in `route_built`, enough to route the three inputs, on
the experiment's block positions: build `B/spec/tree` with the switches of appendix A, dump each node's centre and size
from its static scene (an example of ten lines), and call `route_sized` with them on the working branch. Measure
`route_ms` and the worst `drag_ms` the way `perf.sh` does, on the analog, bench-75 and dense-td.
*Check*: the table against 4.6; routed runs through a foreign frame on the three inputs. **Stop rule of checkpoint S.**

**B12.2 Layering.** `block_rank` (L0-L7).
*Check*: on the analog the layers are actors / R1 R2 GW DNS TG OAI AZ / ARGO TR1 VAULT / LANE / PG / SITE DRAPP PSDEV
TR2 / PSPROD LANES4 / GITLAB with five upstream edges (`DRAPP->TG`, `ARGO->TG`, `PSPROD->TG`, `LANE->OAI`, `LANE->AZ`);
the same bands under all five permutations; the public fixture's bands under its four permutations; dense-td at most 5
edges against the flow (single walk: 21); swimlane: 0 turned (L4); `ratio2` and `ratio3` keep Front first, `ratio4` puts
Back first, `cycle2` keeps Front first; no interleaved siblings on any fixture, `interleave` included (T rows 1-3, S
rows 4-5); every card keeps the layer 0.38 gives it relative to its target on every notes fixture, and `notes/near-drag`
and `note-edge-row` keep their 0.38 layers (a card is its composite's last row there).

**B12.3 Chains both ways and label slots (static).** O0, E1-E4, G3's choice of dummy.
*Check*: no edge drawn as one curve across more than one layer in block mode; an adjacent-layer upstream edge with a
block end is a Z through flow-facing sides, not a lane (test on a three-block cycle: `up_lanes` 0); `wiggles` 0; a
label never inside an end block (public fixture and analog: every chip's centre outside both end frames).

**B12.4 Ordering.** The canonical order, O1-O6 on every level, O4b, budgeted starts.
*Check*: the crossing rows of 4.3 **with the granted number of starts**, and the table crossings x starts (1..7) for
the analog, v2, the five permutations, the public fixture and its permutations, dense-td, dense-lr, bench-75;
`scene_ms` of the analog, the public fixture, bench-75, dense-td, dense-lr within 4.6;
`edge_line_order_barely_moves_the_dense_layouts` and `moving_a_node_declaration_barely_moves_dense_td` pass as
written, and dense-td keeps its crossing must under the six shuffles; `fan_x` 0 on the analog and the public fixture;
runs through a foreign frame 0 static on the analog, v2, the public fixture, bench-75, dense-lr, dense-td (`d-global`:
3 on dense-td; experiment: 1 on dense-lr); the order inside External services does not depend on the generator's base
seed, and its crossing-neutral members are in declaration order (O4b). **Stop rule of checkpoint A.**

**B12.5 Coordinates.** X1-X4.
*Check*: analog `compact_max` <= 5.5 and `compact_mean` <= 3.9, the public fixture <= 7.97 / 4.63; `spine_off` 0 on the
analog, v2, the public fixture, hub5, spread-face, nested-composite, fork-twins; `sib_stagger` 0 on the analog and the
public fixture; `frame_overlap`, `node_frame` 0; own-frame containment tests unchanged
(`a_lane_between_two_members_stays_in_their_box`, `a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`,
`a_rerouted_edge_between_members_stays_in_their_frame_in_route`, `back_edge_inside_a_box_bows_beside_it...`) and the
same assertion on `nested-cases` and spread-face.

**B12.6 Gap room and labels.** G1-G3.
*Check*: `GAP_FLOORS` hold as written (bench-75 static 14.7; prototypes: 8.6-11.9); bench-75 static `overlap_pairs` 0;
`lane_gap_min` >= 8 and the label rows of 4.3 (a)-(c) on the analog, v2, the five permutations and the public fixture
(every prototype had 4-12 crowded chips on each of them, and 1-6 struck ones on v2 and the permutations); area caps of
4.3. The label rows are judged again at C,
after B12.9 has moved the entries; if `lbl_crowd` is above its must here, say so in checkpoint A's report.

**B12.7 Frames.** F1-F3 as one function.
*Check*: `fr_max` <= 0.5 px on the analog, v2, the public fixture and their BT/LR/RL copies, and on no input above
0.38's value by the same probe (`B/j-invariants/b12inv.rs`: nested-composite 41, wide-frames 54, spread-bottom 7.5,
routed-around 24, bench-75 458, dense-lr 303, dense-td 89; `d-global`'s prototype: 80, 84, 40, 33.5, 74, 18, 0);
`title_cut` 0 on `lr` and on every block fixture where 0.38 has 0, in all four directions; the hash test still green
(F3 outside block mode returns today's static frames).

**B12.8 `route()`, `route_sized()`, `route_partial()`.** R1-R7. Build before any polish; measure with `perf.sh`.
*Check*: budgets of 4.6 on two runs, the cost of `levels::augmented` + `block_mode` per drag frame stated;
`DRAG_JUMPS_MAX` as written (bench-75 16, dense-td 5, dense-lr 0, notes 0); routed hard zeros 0; routed edge /
foreign-frame pairs: analog, v2, public fixture, bench-75 0, dense-lr and dense-td not above their static values + 2
(experiment without corridors: 9 and 2; 0.38: 13 and 29); routed `min_edge_gap` at or above `GAP_FLOORS` (experiment:
0.05 on bench-75); `ends_apart` not above 0.38's count on any input (analog 14, dense-td 12);
`a_static_canvas_is_its_drawing_with_even_margins`, `route_partial_keeps_unmoved_edges_and_reroutes_moved`,
`route_partial_matches_route_on_the_dragged_node`, `dragging_a_member_pulls_its_cluster_along` pass unchanged; new
tests: a corridor that passes two bands of staggered blocks, a target on the second row of its block, a far end
dragged from above a block to beside it (the frame is the one `route()` draws, no edge jumps more than `DRAG_JUMP`
besides the dragged node's), every node of the public fixture dragged across the drawing (no panic, frames follow,
`clusters_line_up` holds). **Stop rule of checkpoint R.**

**B12.9 Flow-facing ends, free pins, title-clear entries**, static and routed (2.7 F2, 2.8).
*Check*: `lateral_x` 0 static on the analog, v2, the permutations, the public fixture, hub5, cycle2 and their BT/LR/RL
copies; routed `lateral_x` 0 at the scene's centres; `pin_corner` 0; no title struck (existing chip gates);
`frame_side` 0; on a variant of `B/spec/evidence/title-entry.mmd` with a border-crossing chain the entry jogs in beside
the chip **and the jog lies inside the frame**, in `scene()` and in `route()`.

**B12.10 Hub fans with B11's comb** (3.3).
*Check*: analog, public fixture and bench-75 musts hold with the comb on; a test pins a hub's fan on the public fixture
(its ends on one side, in target order, no crossing among them).

**B12.11 State diagrams, degenerate edges.** Review every `tests/fixtures/notes` drawing before and after, static and
routed, and add fixtures for a fork/join bar whose branches lie in two composites and for a twin comb across a
composite border. The three degenerate own-box rules of 2.8, for every clustered diagram.
*Check*: `a_composite_edge_to_its_own_state_drops_straight_from_the_border`,
`note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `note_links_match_between_scene_and_route`,
`fork_join_branches_drop_straight_and_drags_agree`, `state_composite_keeps_flow_order` pass unchanged; `self-member`
and `sub-nested-self` have `edge_node_x` 0, static and routed; block-metrics `warn` 0 on all 49 inputs of
`B/metrics/extra/in` (0.38 warns on three, all composite self-transitions).

**B12.12 Search and narrowing in block mode.** `narrow`'s entry and acceptance on block layouts;
`a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` and `wide_sibling_boxes_stack_instead_of_a_ribbon` pass as
written (their diagrams are now drawn in block mode; the rules they state still hold). Wrap a long row of unlinked
members inside a block (`grid_pairs` extended to members without inner links when the row is more than `RIBBON_ENTRY`
times as broad as the block is deep).
*Check*: dense-lr area <= 1.25 x 0.38's (7.98 Mpx2; experiment 8.93) and no block row longer than its band needs;
`MAX_ASPECT` and `INK_MEAN_MIN` hold as written.

**B12.13 Generated sweep.** The generated corpus the project used for B8 (about 1800 files) plus generated block
diagrams (2-8 blocks, 0-3 nesting levels, cycles, hubs, all four directions, state composites with notes). Pin
`2M + 1`, `LANE_SHARE`, `BLOCK_ORDER_STARTS`, `BLOCK_SLACK`.
*Check*: no panic, deterministic, hard zeros 0 in both modes on every file; L5's loop ends and leaves no interleaved
pair on any file; O2's two constraints hold on every file; no file with more crossings than both 0.38 and
1.1 x 0.38 + 2 without a named reason; files not in block mode byte-identical; no L4 level drawn worse than 0.38.

**B12.14 Default on, ratchets, docs.** Remove the switch. Re-baseline per 4.5. README, CHANGELOG, images, benchmark
table (4.7). Final `perf.sh`, `render.sh`, `judge`.
*Check*: everything in 4.3-4.6.

### 4.3 Acceptance

block-metrics: `B/metrics/run.sh fm <cli> <out>`, then `B/metrics/judge <out>/metrics.tsv`, rules in
`B/metrics/thresholds.json` (rewritten in B12.0 to the tables below): 0 must-fails. Where a row is stated against
mermaid, the tolerance and its reason are in the last column.

**(a) The analog (local input), against mermaid 12.2.**

| metric | 0.38 | mermaid | experiment | must | tolerance, reason |
|---|---|---|---|---|---|
| `xframe_runs`, `sib_side_linked`, `sib_stagger`, `lateral_x` | 3, 5, 3, 21 | 0, 0, 0, 0 | 0, 0, 1, 7 | 0 | none |
| `compact_max` / `compact_mean` | 11.43 / 6.90 | 4.59 / 3.25 | 5.72 / 3.56 | <= 5.5 / <= 3.9 | mermaid's + 20 %: flowmaid's nodes are text-sized, so the same pad weighs more against a smaller node area |
| `span_max` | 0.767 | 0.308 | 0.314 | <= 0.37 | mermaid's + 20 %: flowmaid's canvas is shorter along the flow, so the same block takes a larger share |
| `crossings` | 24 | 16 | 20 | <= 20 | mermaid's + 4; target <= 16 |
| `lbl_struck` | 0 | 0 | 3 | 0 | none |
| `lbl_crowd`, `lbl_gap_min` | 8, 2.5 | 0, 20 | 9, 0 | 0, >= 12 | mermaid's count of crowded chips |
| `lbl_off_mean` / `lbl_off_max` | 0.198 / 0.332 | 0.130 / 0.329 | 0.140 / 0.364 | <= 0.20 / <= 0.35 | target <= 0.13 |
| `lbl_endpx_mean` | 309 | 558 | 332 | <= 340 | 1.1 x 0.38's: flowmaid keeps its labels nearer their ends than mermaid |
| `lane_gap_min` | 9.3 | 12.0 | 12.4 | >= 8 | `EDGE_SEP_MIN`; target >= 12 |
| `straight_max` | 1175 | 1236 | 1112 | <= 1240 | mermaid's |
| `area` | 3.41 M | 5.18 M | 3.26 M | <= 4.27 M | the smaller of mermaid's and 1.25 x 0.38's: the compact canvas stays |
| `edge_len` | 22350 | 30835 | 20488 | <= mermaid's | |
| `lbl_lbl`, `lbl_node`, `frame_overlap`, `frame_foreign_nodes`, `node_hits`, `warn` | 0 | 0 | 0 | 0 | |

**(b) v2 and the five permutations (local inputs).** mermaid on v2: crossings 13, `compact` 4.03 / 3.14, `span_max`
0.233, `lbl_crowd` 1, `sib_stagger` 1, area 5.37 M.

- v2: the zeros of (a) (`sib_stagger` 0, better than mermaid's 1; `lateral_x`: 0.38 24, experiment 11); `compact_max` <=
  5.5 and `compact_mean` <= 3.9 (the family's bound: mermaid's PROD is 3.97-4.69 across the family); `span_max` <= 0.28;
  crossings <= 17 (mermaid's + 4; 0.38: 19; experiment: 20); `lbl_struck` 0; `lbl_crowd` <= 1; `lbl_endpx_mean` <= 290
  (1.1 x 0.38's 263); area <= 3.21 M (1.25 x 0.38's); edge length <= mermaid's.
- each permutation: the zeros; `compact_max` <= 5.5; `span_max` <= 0.37; `lbl_struck` 0; `lbl_crowd` 0; crossings <= 24
  (experiment: 22 25 20 22 22; 0.38: 12 19 24 24 34); **the same band order as the analog** (mermaid scrambles it on
  the first); canvas width within 10 % across the permutations.

**(c) The public fixture (in the repository) and its permutations**, against mermaid: the zeros of (a);
`compact_max` <= 7.97 and `compact_mean` <= 4.63 (mermaid's 6.64 / 3.86 + 20 %); `span_max` <= 0.22 (0.183 + 20 %);
crossings <= 9 (mermaid's 5 + 4; 0.38: 14; experiment: 7); `lbl_struck` 0; `lbl_crowd` 0; `lbl_endpx_mean` <= 295
(1.1 x 0.38's 268); area <= mermaid's 2.78 M (experiment: 1.65 M); edge length <= mermaid's 20125; the same band order
and a canvas width within 10 % across its four permutations. The same rows hold in `tests/quality.rs` through the new
`quality.rs` measures, so the gate lives in the repository.

**(d) New measures**, static and routed at the scene's centres, on every block-mode fixture: `lateral_x` 0 on the
analog family, the public fixture, hub5 and cycle2 and not above 0.38's elsewhere; `up_lanes` 0 everywhere; `spine_off`
0; `fan_x` 0; `pin_corner` 0; `title_cut` not above 0.38's, in all four directions; `fr_max` and `ends_apart` per
B12.7 and B12.8.

**(e) Rows-equal inputs: not worse than 0.38, and as tidy as mermaid where 0.38 was not.** For `examples/{advanced,
subgraph}.mmd`, `quality/fanout-frames.mmd`, `notes/{near-drag, spread-bottom, straight-link, wide-frames}.mmd`, the
probes p03-p08 and p10-p12, hub5, cycle2, state-note, swimlane and every other fixture whose block layers equal its
0.38 layers (B12.2 prints the list): per input and mode the harness `defects`, `x_drawn`, `edge_frame_pairs`,
`overlap_pairs` not above 0.38's, hard zeros 0; block-metrics crossings, `xframe_runs`, `sib_side_linked`,
`sib_stagger`, `lbl_struck`, `lbl_crowd` not above 0.38's; `compact_max` <= the larger of 0.38's + 0.05 and
1.2 x mermaid's (a frame may grow by its jog room: hub5 0.38 3.73, mermaid 3.97); area <= the smaller of mermaid's and
1.25 x 0.38's; edge length <= 1.1 x 0.38's; `lateral_x` not above 0.38's and 0 where mermaid has 0 (hub5: 5 -> 0;
cycle2: 6 -> 0); and a picture triple (0.38, B12, mermaid) of each reviewed and attached to checkpoint C's report.
**Stop rule of checkpoint C.**

**(f) "No worse than 0.38.0" on the dense corpus.**

- `judge` on bench-75, dense-lr, dense-td: the zero rules; `xframe_runs`,
  `sib_side_linked`, `sib_stagger`, `lbl_struck`, `lbl_crowd`, `compact_max` not above 0.38's; crossings
  <= 1.1 x 0.38's + 2 (bench-75 334, dense-td 90, dense-lr 52); area <= mermaid's; edge length <= the larger of
  mermaid's and 1.1 x 0.38's.
- flowmaid's harness (`examples/quality.rs`, `tests/quality.rs`): every hard zero 0 in both modes (`edge_node_x`,
  `shared_ports`, `arrow_on_start`, `wiggles`, `frame_side`, `node_overlap`, `node_frame`, `frame_overlap`) and
  `own_frame_x` 0; per fixture and mode `defects`, `edge_frame_pairs`, `overlap_pairs` not above 0.38's;
  `min_edge_gap` not below `GAP_FLOORS`.
- The 49 inputs of `B/metrics/extra/in`: runs through a foreign frame 0 on every block-mode input (mermaid: 0 on all 36
  inputs with frames; 0.38: 45 runs on 10), `warn` 0.
- All flat inputs and the clustered inputs not in block mode: byte-identical (hash test).

**(g) Invariants**: guard green; determinism; the scene contract test
(`scene_carries_stable_identity_and_stays_index_parallel`) on the block fixtures; hit-testing, fold, `to_mermaid` round
trip on the block fixtures in all four directions.

### 4.4 Tests to add (names are suggestions; one per rule; every one runs on repository fixtures only)

| test | asserts |
|---|---|
| `flat_and_unlinked_inputs_render_as_before` | B12.0 hashes |
| `block_mode_is_decided_by_border_links`, `block_mode_agrees_between_scene_and_route` | B12.1 lists; the three entry points on every fixture |
| `a_subgraph_takes_consecutive_rows`, `linked_blocks_never_share_a_row`, `sibling_blocks_never_interleave` | L5, L6 properties on generated graphs and on `interleave` |
| `block_order_follows_declarations_until_it_costs_twice_the_minimum` | L3 on the public fixture (walk kept), dense-td (net), `ratio2`, `ratio3` (declaration order), `ratio4` (flipped), `cycle2` (node cycle: L0 decides) |
| `block_order_survives_reordered_statements`, `block_order_follows_first_mention` | same bands on the public fixture's permutations; the `cycle2-tie` pair documents the first-mention rule |
| `a_ping_pong_level_stays_lanes` | L4, swimlane: 0 turned, frames side by side, not worse than 0.38 |
| `a_source_block_sits_beside_what_it_feeds` | the build pipeline beside the shared services |
| `a_note_card_keeps_its_row_in_its_block` | L2: `near-drag`, `note-edge-row` (a card after the last row, a lead card before the first) |
| `a_turned_edge_runs_up_its_channel_through_flow_sides`, `a_turned_edge_one_layer_up_is_a_z_not_a_lane` | E3: out of the top, into the bottom (TD), all four directions; the adjacent-layer case |
| `chains_keep_one_side_of_a_block` | O2's constraints; no run through a foreign frame, static, on every block fixture |
| `level_order_is_deterministic_and_bounded`, `the_start_order_ignores_edge_statement_order` | O4: same SVG twice, start count under the budget; shuffled edge lines give the same layer orders |
| `crossing_neutral_members_keep_declaration_order` | O4b |
| `a_hub_fan_leaves_in_target_order` | O6 |
| `a_spine_inside_a_block_is_straight`, `a_spine_across_a_border_is_straight`, `a_compact_block_keeps_its_brandes_koepf_columns` | X1-X3 |
| `sibling_frames_of_a_band_start_level_in_scene_and_route` | X4: `sib_stagger` 0; the same frames from `route()` at the scene's centres |
| `a_band_gap_holds_its_tracks_and_labels_between_the_frames` | G1 |
| `a_chain_label_sits_outside_both_end_blocks`, `a_label_slot_is_clear_of_foreign_lines` | G3 |
| `frames_agree_between_scene_and_route`, `a_far_end_drag_never_changes_a_frame_it_does_not_enter` | F1-F3, R6 |
| `an_lr_frame_is_as_long_as_its_title` | F2, `lr`, LR and RL |
| `an_entry_under_a_title_jogs_in_beside_the_chip_inside_the_frame` | 2.8, TD and BT, scene and route |
| `an_edge_to_a_block_ends_mid_side_clear_of_the_chip_and_the_corners` | free pin, alone and among other entries |
| `a_subgraph_self_edge_is_a_loop_in_scene_and_route`, `a_stub_to_an_own_member_passes_no_member`, `a_stub_to_an_own_nested_subgraph_passes_no_member` | 2.8, with and without block mode |
| `route_takes_a_corridor_round_a_foreign_block`, `a_corridor_passes_two_staggered_bands`, `a_corridor_reaches_a_second_row_target`, `corridors_nest_and_do_not_jump_on_a_one_px_drag`, `a_nested_edge_crosses_each_frame_on_a_flow_side_in_route` | R3, R5 |
| `a_node_dragged_onto_a_foreign_frame_is_still_routed` | R4 |
| `route_and_scene_agree_on_which_edges_run_upstream` | R2 |
| `dragging_every_node_of_a_block_diagram_keeps_the_contract` | B12.8 |
| `a_direction_block_inside_a_bordered_block_keeps_its_direction` | B9 inside block mode (`b9-inside`, `b9-inside2`) |
| `layout_clustered_returns_blocks` | public API doc example |

### 4.5 Pinned tests, gates and images: what may change and why

**Must pass as written** (a failure is a regression, not a re-baseline): every test of `compound.rs`;
`assert_clean_clusters` users; the own-frame tests; the tests on `examples/advanced.mmd`
(`render::advanced_showcase_parses_and_renders`, `scene::a_two_cycle_runs_side_by_side_across_its_gap`,
`scene::a_two_cycle_into_a_diamond_keeps_its_twin_ends_apart`,
`scene::a_run_into_a_box_crosses_its_border_clear_of_the_run`, `ports::a_nearly_straight_pair_is_drawn_straight`: the
diagram is now drawn in block mode and the rules they state still hold);
`a_composite_edge_to_its_own_state_drops_straight_from_the_border`;
`note_links_hold_their_ends_and_corners_in_scene_route_and_drag`; `a_static_canvas_is_its_drawing_with_even_margins`;
`a_hub_fed_row_of_shallow_frames_wraps_into_two_rows`; `hub_fed_leaves_in_a_box_pack_into_a_grid`;
`largest_box_is_centred_among_its_siblings`; `route_partial_*`;
`edge_picking_matches_the_painted_strokes_on_every_fixture`; `marquee_selects_an_edge_once_its_painted_stroke_touches`;
`moving_a_node_declaration_barely_moves_dense_td`; `edge_line_order_barely_moves_the_dense_layouts` (the canonical order
of 2.4 is what keeps it); `every_fixture_has_a_ceiling` (the new fixtures need rows).

**Intended changes** (say each in its commit message):

| test or gate | change | why |
|---|---|---|
| `layout::a_box_seats_its_loose_parts_on_its_top_layer` | restated | it pins a `seat_box_sources` case in which a loose member sits level with the hub that feeds its box through another member; a block lies wholly below its feeder |
| `tests/quality.rs` `CEILINGS` rows of bench-75, dense-lr, dense-td | ratchet **down** | edge / foreign-frame pairs fall (static 46 -> at most 4 over the fixtures); `defects` may not rise |
| `CEILINGS` rows of advanced, subgraph, fanout-frames and the notes fixtures | re-measured, no value above the old one | they are drawn in block mode; 4.3 (e) |
| `DEFECT_BASELINE` (`x_drawn` 428 / 494, `band_len`, sideways-run fields) | re-measured, each value at or below the old one | the dense fixtures are re-laid. `x_drawn` may not rise: that is the crossing must |
| `BORDER_LEN_MAX`, `CHANNEL_MAX`, `EDGE_CHIP_MAX`, `XGAP_MAX`, `LBL_EDGE_MAX` | re-measured; a higher value needs a named cause and a picture | positions change; the prototypes' routed values (border length 4530, chips 8) were `route()` without corridors, i.e. regressions |
| `GAP_FLOORS`, `DRAG_JUMPS_MAX`, `INK_MEAN_MIN`, `MAX_ASPECT`, `SHUFFLE_WIDTH_MAX`, hard zeros | **not loosened** | they are the invariants |
| `casing::a_short_line_keeps_at_most_one_gap`, `casing::a_line_takes_one_role_across_a_cluster_of_crossings`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route` | re-read; update the fixture-specific numbers if the rule each states still holds | they count crossings or measure a leg on a dense fixture that is re-laid |
| new gates | block shape per fixture (`compact_max`, `span_max`, `sib_side_linked`, `sib_stagger`), `lateral_x`, `up_lanes`, `spine_off`, `pin_corner`, `title_cut`, `fr_max`, `ends_apart`, the public fixture's rows of 4.3 (c) | no gate sees these today |

**README images.** `docs/img/compare/state.png` does not change. `docs/img/compare/{advanced, subgraph}.png` and
`docs/ex-subgraph.svg` are re-rendered if their bytes change, each with its before and after in the commit (4.3 e).
Add one comparison image for a block diagram: the public fixture, never the analog. Re-render the benchmark renders of
bench-75, dense-lr, dense-td.

### 4.6 Perf budgets (PLAN invariant 1; `tools/perf.sh`, two runs before calling a regression)

| file | `scene_ms` limit | `route_ms` limit | worst `drag_ms` limit |
|---|---|---|---|
| advanced | 1.42 | 1.72 | 2.61 |
| subgraph | 0.38 (D1) | 1.05 | 2.04 |
| fanout-frames | 4.38 | 1.34 | 3.44 |
| dense-lr | 46.8 | 8.03 | 17.76 |
| dense-td | 50.68 | 9.20 | 17.80 |
| bench-75 | 61.94 | 15.76 | 25.28 |
| analog (local; baseline = 0.38.0: 6.6 / 1.9 / 5.8) | 13.2 | 2.9 | 7.25 |
| public fixture (baseline measured in B12.0) | 2 x baseline | by the rule below | by the rule below |

`scene_ms`: 2 x baseline; for a file whose baseline is under 1 ms, max(2 x baseline, baseline + 0.3 ms) (decision D1).
`route_ms` and `drag_ms`: max(1.25 x baseline, baseline + 1.0 ms). `route_partial` under 6 ms a frame on every file
(README publishes "about 3 ms, 6 ms at worst" for bench-75). The flat corpus files keep their numbers. Every file of
the table is drawn in block mode.

### 4.7 Docs

- **README**: Subgraphs section (0.38: lines 282-289): the block rule in three sentences (consecutive rows; linked
  blocks stack along the flow; edges between blocks run in the gaps and cross frames on flow-facing sides), each said
  of "a diagram in which an edge joins a subgraph to something outside it"; the `direction` rule next to it
  (unchanged); the lane rule (when a level stays side by side); `flowchart-elk` (accepted, same as the default). How it
  works (797-806): Layers, Ordering, Coordinates, Narrowing. Guarantees (811-817): add "in a diagram laid out as
  blocks, no edge runs through a block that holds neither end (static)", worded for diagrams drawn in block mode and
  not for a level the lane rule leaves side by side. Benchmark rows of the clustered inputs (825-851) and the drag
  numbers (857), re-measured.
- **CHANGELOG** under `## [Unreleased]`: Changed (subgraphs are laid out as blocks in every flowchart and state
  diagram with an edge across a subgraph border; upstream edges between blocks run in channels; `layout_clustered`
  returns block layouts), Fixed (a subgraph self-edge is drawn as a loop, an edge from a subgraph to its own member or
  nested subgraph no longer runs through a member, frames no longer snap on the first drag: all three in every diagram
  with subgraphs), with the public fixture's numbers.
- Doc comments on every new `pub(crate)` item that states a rule, in the code base's style (why, constants by name).

## 5. Out of scope, and the split

Out of scope for B12:

- Generalising `compound.rs` to bordered subgraphs (`d-recursive`): per-block pipelines, pins, stitching.
- Honouring `direction` in a subgraph that has a border edge (mermaid ignores it too; B9's rule stays).
- Stacking the blocks of a level the lane rule leaves side by side (mermaid stacks them and turns two arrows of five).
- Aligning a short block inside a tall band by its pins (mermaid's R5 beyond "sources sit last").
- One lane per edge at 20 px pitch (mermaid pays 5.18 Mpx2 for it), and mermaid's 16 crossings on the analog as a
  must (it is the target; the must is 20).
- Styling subgraphs (`style`, `class` on a subgraph are still dropped), class and ER diagrams, the `advance` module.

**Split: two packages, cut after B12.8 (decision D2).**

- **B12a "blocks: structure and routing"** = B12.0-B12.8 (checkpoints S, A and R), merged to `dev` with the internal
  switch off: no output changes, no release, no release note, every existing test passes as written, and the block
  path is reviewed on what it can already be judged on (structure, crossings, invariants, budgets).
- **B12b "blocks: edges, labels, default on"** = B12.9-B12.14 (checkpoint C): acceptance, ratchets, docs, release
  0.41.0.

Reason: 9 of 1047 library tests and 7 of 13 quality tests fail on the best prototype and nearly every ratchet moves; one
review cannot tell an intended re-baseline from a regression unless there is a point where nothing visible has changed
yet. B12a alone must not ship as the default.

## 6. The judges' blocking flaws and where each is resolved

| # | flaw (judges) | resolution |
|---|---|---|
| 1 | crossings rise; the block transpose is unprototyped (all three) | transpose tested and rejected; 2.4 level-first ordering with starts and chains, evidence in appendix A (analog 20 from the fifth start); B12.4 with the starts table, a time gate and a stop rule |
| 2 | edges enter frames through lateral walls and run along title strips; rule 1.6 cannot work for a first-row target; "ports" was the wrong stage (visual, engineering) | 2.8: sides set by the builders, title-clear ports, displaced entries with jog room inside the frame (2.7 F2), X2.2 breadth; gate `lateral_x`; B12.9 |
| 3 | labels and gap room; struck labels on v2 and the permutations; room measured row to slot (all three) | 2.6 G1-G3; label rules gated on v2, the permutations and the public fixture; B12.6 |
| 4 | own-frame containment breaks inside blocks (visual) | X2 keeps inner runs between the members where it can; F3 grows the frame over the rest, as 0.38 does; the four tests unchanged plus spread-face and nested-cases; B12.5 |
| 5 | straight spines lost; packing discards Brandes-Köpf; alignment rule dead (visual, engineering) | 2.5 X1, X2.3, X2.4, X3 (keep when compact, seeded squeeze, rigid spines, border spines); gate `spine_off`; B12.5 |
| 6 | block order without a safety net, 21 of 47 edges upstream on dense-td; no bound (all three) | L3: two walks, exact minimum per component beyond `2M + 1`; B12.2 check |
| 7 | no rule for ping-pong levels (visual, invariants) | L4; fixture; constant from the sweep; decision D4 |
| 8 | partially overlapping unlinked siblings (engineering) | L5 no interleaving, in a loop with the source rule |
| 9 | `route()` and drag: budgets, drag jumps, routed gaps, corridors unbuilt and root-only, scene and route disagree on more edges (all three) | 2.9 R1-R7, per level, one breadth per band; a spike before the structure work (B12.1s) and B12.8 before polish, each with a stop rule; `ends_apart` |
| 10 | upstream lanes run round whole bands (engineering, visual) | E3: channels through flow-facing sides in scene and route, the adjacent-layer case included |
| 11 | state composites snap more on the first drag; frames differ between scene and route (invariants) | F1-F3: one frame function, a frame a function of its own members; gate `fr_max` |
| 12 | diagrams that need no change move; state fixtures; static time on tiny files (engineering, invariants) | block mode by structure (2.2): a diagram without a border link is byte-identical; the rows-equal inputs are gated one by one against 0.38 (4.3 e); D1 |
| 13 | the layout search goes idle or changes regime; `layout_clustered` changes silently (engineering) | L7, O5, B12.12; doc comment and CHANGELOG |
| 14 | state diagrams under-specified; the sink rule for lead notes untested (engineering, invariants) | L2 (a card is a member and takes a row; lead cards per level); B12.11 review round with named tests and new fixtures |
| 15 | which pinned tests are re-baselined and which are regressions (all three) | 4.5 |
| 16 | block shape leftovers: frame tops 17 px apart, PROD too wide, dense-lr L-shaped (visual) | X4; X2; B12.12 |
| 17 | cannot be verified as one step (engineering) | 4.1 checkpoints; section 5 split |
| 18 | routed canvas margin test fails unexplained; RL not run; B11's comb untested with blocks; prototype numbers of two modes mixed (engineering, invariants) | R7 and B12.8; all four directions in the fixtures; B12.10; every number in this spec names its mode |
| 19 | shared-code guards; flat byte identity unpinned (invariants) | 3.4; B12.0 hash test |
| 20 | `d-hybrid`'s stale back flags (engineering) | L6 |

## Appendix A. Experiments and measurements

### A.1 The experiment behind 2.4

Tree `B/spec/tree` = `B/d-global/tree` + `B/spec/experiment.patch` (layout.rs only). Build: `cd B/spec/tree &&
CARGO_TARGET_DIR=B/target-x cargo build --release --bin flowmaid --example quality` (the build dir used for the spec is
deleted; a prebuilt CLI is `B/critic/fm-spec`). Inputs `B/spec/in` (the 8 metric inputs, v2, five permutations),
structure files `B/spec/struct`, runner `B/spec/measure.sh <label> ENV=...`. Tables: `B/spec/evidence/metrics.*.tsv`
(block-metrics), `quality.*.tsv` (flowmaid's harness, static and routed).

Switches (each is "set or unset": `FM_B12_TRIG=0` still turns the trigger on): `FM_B12` 0 off / 1 lanes / 2 chains (as
`d-global`); `FM_B12_BT=1` sibling-block swap judged by the layered count; `FM_B12_FORCE="a,b"` force a swap with
pinned keys; `FM_B12_LVL=1` level-first order of the root (O2) with slot keys (O3), both half-rounds judged;
`FM_B12_STARTS=k` (O4); `FM_B12_SEED=n`; `FM_B12_TRIG=1` `d-hybrid`'s rank trigger; `FM_B12_DUMP=1` layers, per-gap
crossings, orders.

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
6. `judge` (the earlier, looser rules) on that build: 9 must-fails over 14 inputs: dense-td crossings (112 > 90) and,
   on the analog, `sib_stagger`, `compact_max` 5.72, `lbl_off_max` 0.364, `lbl_crowd` 9, `lbl_gap_min` 0, `lbl_struck`
   3: the pieces 2.5-2.7 own. Hard zeros 0 in both modes on the 8 harness inputs; edge / foreign-frame pairs static
   47 -> 1, routed 47 -> 11 (unchanged `route()`).
7. An entry under a title chip goes round to the member's side in 0.38 already
   (`B/spec/evidence/title-entry.fm038.png`).

### A.2 Measurements made while the spec was reviewed (same binary; scripts and outputs in `B/critic`, `B/revise`)

1. **Starts** (`B/critic/starts/s{1,2,3,4,5,7}/metrics.tsv`), drawn crossings for 1, 2, 3, 4, 5, 7 starts: analog 37,
   37, 36, 36, 20, 20; v2 20 throughout; perm-a 37, 37, 37, 37, 22, 22; perm-b 32, 25, 25, 25, 25, 25; perm-c 37, 37,
   36, 36, 20, 20; perm-d 42, 42, 36, 36, 22, 22; perm-e 39, 39, 39, 39, 22, 22. CLI time on the analog: 0.38 13 ms;
   experiment 60 ms (1 start), 255 ms (5), 302 ms (7).
2. **Edge-statement shuffles** (`B/critic/shuf.py`, the six xorshift shuffles of
   `edge_line_order_barely_moves_the_dense_layouts`): dense-td 3829 px in source order, 3776, 2989, 3769, 2986, 2986,
   2986 shuffled (0.38: 4249 and 4407); the 26 layers are identical, the layered crossing count is 84 against 70-71.
   bench-75 and dense-lr stay within 4.5 %.
3. **The rank trigger** (`B/critic/sweep/trigger.txt`): 21 of the 36 framed inputs byte-identical to 0.38. Crossings
   of a lateral frame wall in those 0.38 drawings (`B/critic/lateral.py`, with the flow axis of each input): hub5 5,
   cycle2 6, p06-block-cycle 2, p03 1, p07 1, spread-bottom 1; advanced, subgraph, wide-frames, straight-link,
   near-drag, p04, p05, p08, p10, p11-lr, p12 0. For comparison the analog has 21 on 14 edges and v2 24.
4. **Block mode on every framed input** (the experiment with the trigger unset; it has no flow-facing ends yet): 7 of
   36 byte-identical to 0.38 (state, fanout-frames, p01, p02, p03, p05, p13); lateral wall crossings hub5 5 -> 3,
   cycle2 6 -> 1 (returns straight up, `B/revise/out/cycle2.blk.png`), advanced 0 -> 1, wide-frames 0 -> 1,
   spread-bottom 1 -> 0, analog 21 -> 7, v2 24 -> 11; pictures `B/revise/out/*.{038,blk}.png`.
5. **The public fixture** (`B/revise/mfm`, `mmm`, `mexp`): the table in 1.2.
6. **`route_ms` on the analog** against the limit 2.9: 0.38 1.9; experiment 4.0; `d-global` 4.7; `d-hybrid` 5.0;
   hand-placed blocks 4.8. None has corridors.
7. **Labels**: `lbl_endpx_mean` on the analog 309 (0.38), 332 (experiment), 558 (mermaid).

## Appendix B. Decisions

Taken by the user on 2026-11-10 (`B/USER-DECISIONS.md`):

- **The bar.** "pokoknya mirip atau lebih bagus dan rapi dari mermaid js 12": acceptance on the analog and on v2 is
  stated against mermaid 12.2 (4.3 a, b). The tolerances there are this spec's reading of "a small stated margin":
  block shape + 20 % with the reason given, crossings + 4, no struck chip, no more crowded chips than mermaid, edges
  through flow-facing sides only; area, edge length and label distance stay better than mermaid's.
- **D1 Static time on tiny clustered files.** For files under 1 ms the static limit is max(2 x baseline, baseline +
  0.3 ms).
- **D2 Split**: yes. B12a (nothing visible, merged to dev, no release), then B12b and release 0.41.0.
- **D3 Upstream edges between blocks**: channels. They leave the top of a node and enter the bottom of a block
  (mermaid's look). Back-edges between two plain nodes keep the lane.
- **D4 Lane rule**: the principle is accepted; the constant comes from the sweep; such diagrams must not get worse.
  What the rule means in pictures: under L4 a level keeps its linked frames side by side, as 0.38 draws them
  (`B/critic/out/swimlane.038.png`), where mermaid stacks two compact blocks with 2 of 5 arrows upward
  (`B/critic/mm/swimlane.mermaid.png`, also `d-global`'s result, `B/critic/out/swimlane.exp.png`). Such a level fails
  the block-shape rules by design.
- **D5 The analog in the public repository**: not granted. The analog family stays a local acceptance input; the
  repository's fixture and README image are the synthetic `blocks-overview`.
- **D6 If a stop rule fires**: the implementer stops and reports; no gate is re-baselined to pass.

Taken by this spec on the user's standing answer, to be confirmed:

- **D7 Diagrams whose rows 0.38 already stacks.** Two options. (a) Keep 0.38's drawing for them (`d-hybrid`'s rank
  trigger): 21 of 36 framed inputs stay byte-identical, the README images of `advanced` and `subgraph` do not move,
  and simple block diagrams keep 0.38's edges: `hub5` 5 crossings of a lateral frame wall where mermaid has none
  (`B/critic/out/hub5.038.png` against `B/critic/mm/hub5.mermaid.png`), `cycle2` its returns as side lanes where
  mermaid draws channels (`B/critic/out/cycle2.038.png` against `B/critic/mm/cycle2.mermaid.png`). (b) Block mode for
  every diagram with a border link (2.2): those drawings get the block edge rules; `advanced`, `subgraph`,
  `fanout-frames` and four notes fixtures are re-laid and gated one by one against 0.38 (4.3 e), two README images may
  be re-rendered, and the sub-millisecond files run the block pipeline inside D1's budget. **This spec builds (b)**: it
  is what "the default for every flowchart and state diagram with subgraphs" says, and (a) is below the bar on hub5
  and cycle2. Under (a) the scope sentences of 1.1 and 4.7 would read "diagrams whose rows change", 2.2 would compare
  the two rankings, and 4.3 (e) would fall away.

## Appendix C. Reviewed and kept

The review of this spec raised 22 findings; all 22 hold and are resolved in the body. Where the body departs from the
change a finding proposed, or keeps something the review questioned, the reason is here.

- **Block mode (2.2).** The review proposed to leave D7 open and, for option (b), to turn block mode on "when a drawn
  edge with a block end runs upstream or crosses a frame". The decision is taken as (b) on the user's standing answer
  and flagged for confirmation. The predicate is "a border link exists" because whether a 0.38 edge crosses a lateral
  wall cannot be known before drawing, and a hub with blocks and only downstream edges has such crossings too. So
  `subgraph.mmd` and `fanout-frames` are in block mode as well, beyond the review's list.
- **`block_mode`'s inputs.** The shared constructor is kept as proposed. The predicate no longer reads L3's count of
  turned links, so the weight of a stand-in against a rank-only pair cannot make the entry points disagree.
- **L4 and "the trigger stays off".** With block mode structural there is no off state for such a level; 2.3 L4 says
  exactly which steps run and 4.3 (e) gates the result against 0.38.
- **LR titles.** The review proposed "the block's extent along the flow plus the gap after it is at least the title's
  length". The frame itself grows instead (2.7 F2): a title hanging over into a band gap would be crossed by the
  tracks of that gap, and titles are never struck.
- **Jog room.** Kept as proposed, with one addition: the number of displaced entries is counted from the block's own
  members and the graph, never from where far ends lie, so the frame rule cannot reopen the hole the frame alignment
  had.
- **F3's inner legs.** Kept as proposed. They are the one frame input that depends on a far end, so R6 redraws the end
  blocks of a moved node's border links.
- **Label distance.** Of the two proposed ways the gate is taken, at 1.1 x 0.38's and not at 0.38's own value: stacked
  blocks lengthen the edges between them (the experiment has 332 against 309 with a shorter total edge length), and
  the user's bar asks to stay better than mermaid (558), which the gate does by a wide margin.
- **Starts.** Beyond the proposed gates, the budget is spent on the first placement only and narrowing candidates run
  one start; otherwise the time gate could not be met on inputs that go through `narrow`.
- **One number of the review is corrected.** `lateral.py` assumes a top-down flow. `notes/spread-bottom` is an LR
  diagram with 1 lateral crossing, not 4, and `p11-lr` has none; hub5 (5), cycle2 (6) and p06 (2) stand, and so does
  the finding.
- **Kept against the review's doubts**: the `2M + 1` threshold (the ratio fixtures show what it does: 1:2 and 1:3 keep
  the author's order, 1:4 flips), chains for upstream edges, level-first ordering with seven starts as the default
  thoroughness, the lane rule at one third.
