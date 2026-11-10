# B12 "subgraphs as blocks" — design d-recursive: recursive blocks (a generalised `compound.rs`)

Label: `d-recursive`. Date: 2026-11-10. Source read: `b12/tree/src` (dev = 0.38.0, commit 0366084); every `file:line`
points into that tree. Everything measured here was measured on the sanitized analog `b12/in/infra-overview.mmd` and
the repo's corpus; nothing from `b12/private/` is quoted.

Scratch artefacts, all under `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/d-recursive/`:

| file | what |
|---|---|
| `infra-overview.proto.png`, `advanced.proto.png`, `subgraph.proto.png`, `fanout-frames.proto.png`, `bench-75.proto.png`, `dense-td.proto.png` (+ `.svg`) | prototype renders |
| `advanced.fm038.png`, `subgraph.fm038.png`, `fanout-frames.fm038.png` | the same inputs by 0.38.0, for comparison |
| `dump/lg2-<block>.png`, `dump/lg2-root.png` | each level of the analog as its own pipeline run draws it (`PROD`, `DR`, `STG`, `EXT`; the root with blocks as plain boxes and the ends on their pins) |
| `dump/root-free-ends.png` | the root level before the ends are moved onto the pins: where the ports stage alone would put them |
| `metrics/metrics.tsv` | block-metrics rows of the prototype for the 8 inputs |
| `tree/src/blocks.rs`, `prototype.diff` | the prototype (1 223 lines new, 84 lines of hooks added in 7 other files) |
| `flowmaid-proto` | the prototype CLI (`flowmaid-proto in.mmd -o out.svg`; `FM_NOBLOCKS=1` = 0.38 behaviour) |

The build directory is deleted. Nothing was committed, and nothing under `b12/tree` or `/home/user/flowmaid` was touched.

---

## 0. The design on one page

1. **Every non-empty subgraph is a block.** A block is laid out on its own by the existing pipeline and is one unit
   of its parent's layout, the size of its frame. The diagram is a tree of *levels* (the root and every block); each
   edge belongs to exactly one level, the lowest one that contains both ends.
2. **An edge across a border is cut at the border into pieces, one per level it passes.** The cut point is a *pin*
   on the frame. Inside the block the piece runs between the member and the pin; in the parent it runs from pin to
   pin. The pieces are joined into one `SceneEdge`, so the scene contract does not change.
3. **Who decides what about a pin:**
   - the **side** (upstream or downstream side of the frame): decided first, top-down, from the orientation the
     parent level gives the edge after its cycles are broken;
   - the **place** along that side: decided by the block's own layout (a gate dummy on the border, lined up with the
     member's port, a corridor reserved through the rows between);
   - the **edge-to-slot assignment inside one member's bundle** of pins: decided by the parent's ordering, so the
     pieces outside fan out without crossing. Nothing has to be laid out twice.
4. **In its parent a block is a rigid group** of its pins between two walls, in one rank: the machinery today's
   cluster walls already use (`layout.rs:3255-3282`, `enforce_contiguity` `layout.rs:5513`). The parent's crossing
   minimisation and coordinate assignment therefore see where on the block each edge really leaves, which is the
   part a block-as-one-point layout gets wrong (measured in the prototype, section 3).
5. **Block-level cycles** are broken per level on the collapsed graph, by the breaker `rank()` already has. A
   reversed edge leaves its source block through the upstream side and enters its target block through the
   downstream side; it is laid out as a forward edge the other way round (a dummy chain), not as a lane round the
   drawing.
6. **`route()` and `route_partial()` use the same tree of levels, without any layout**: frames from the host's
   positions as today, sides and pins by geometry, one position-driven edge pass per level with child blocks as
   plain boxes in the way. That is what keeps edges out of foreign frames after a drag.
7. **The prototype is real code** (a hack in a tree copy, sections 3.1-3.4). It shows that the recursion, the gates
   and the stitching work with today's pipeline: the analog passes every block-shape rule of the acceptance list
   (compactness, no frame over 0.40 of the flow, no side-by-side linked pair, no run through a foreign frame). It
   also shows what is **not** solved by recursion alone: the prototype's edges between blocks are clearly worse
   than both 0.38 and mermaid (60 crossings against 24 and 16, 38 946 px of edge against 22 350 and 30 835). The
   cause is known and is point 4 above, which the prototype does not implement. This design is therefore proven for
   blocks and unproven for the quality of inter-block edges.

---

## 1. The algorithm

### 1.1 Inputs and terms

Inputs: `Graph` (nodes, edges, subgraphs with `parent`, sub-edges), node sizes and wrapped labels, exactly what
`scene_labelled` (`scene.rs:3050`) gets today. No new input.

- *Flow axis*: y for TD/BT, x for LR/RL. *Breadth*: the other axis. *Upstream / downstream side*: the two sides of a
  frame across the flow axis (top and bottom in TD).
- *Block*: a subgraph with at least one node at any depth. *Level*: the root or a block. *Units* of a level: its
  direct member nodes and its direct child blocks.
- *Chain* of an edge end: the blocks strictly round it, outermost first. For a node end: all blocks holding the
  node. For a subgraph end (`End::Sub(s)`): the blocks round `s`, without `s` (an edge to a subgraph ends on its
  frame, it does not go in; `compound::inside`, `compound.rs:119`).
- *LCA level* of an edge: the innermost level whose block is in both chains (the root if none). At that level the
  edge joins two units. If an end's chain goes deeper than the LCA, the edge *descends* into a child block there.
- *Gate*: a zero-size dummy of a block's own layout on its frame border, one per descending edge end. *Pin*: the
  gate as its parent sees it (a fixed point on the child's border). *Free pin*: the end of an edge to the block
  itself; nothing inside the block fixes its place.
- *Bundle*: the gates of one member on one side of the block.

### 1.2 Steps

**S0. Tree** (pure function of the graph, O(nodes + edges × depth), shared by `scene`, `route`, `route_partial`).
Chains, each node's owner, each block's children, each edge's LCA level, its unit at every level on both chains.
Sub-edges are normalised first:
- member → its own block: the end on the block is replaced by the block's entry member (`own_box_targets`,
  `scene.rs:4075`), as today; it is then an ordinary edge of that block's level;
- block → its own member: a stub inside the block from the upstream border to the member, as `own_box_stub`
  (`scene.rs:3832`) draws today; it is a level edge whose source is a free pin on the inside of the border;
- block → itself: a self-loop of the block's unit in its parent level (today a zero-length path).

**S1. Orient, top-down** (topology only, no sizes). For each level L, outermost first, build the collapsed
multigraph H(L): one vertex per unit; one arc per edge whose LCA is L, between its two units; an invisible link is
an arc that constrains ranks and nothing else. For every edge that crosses L's own border (its side there is known
from L's parent), add an arc from a virtual source S to the inner unit (it enters through the upstream side) or from
the inner unit to a virtual sink T (it leaves through the downstream side). Break cycles with the breaker of
`rank()` (`layout.rs:2314-2371`), run on H(L). Result per edge: `back` at its LCA level.

The orientation fixes every side: a forward edge leaves through the downstream side of every block on its source
chain and enters through the upstream side of every block on its target chain; a `back` edge the other way round.
The S/T arcs make a block's inner ranking agree with the outside: a member fed from upstream ranks early, one that
feeds downstream ranks late.

**S2. Lay out each block, bottom-up** (post-order, iterative, no recursion). For block B build the *level graph*:
- its units; a child block is a *group* (S3);
- two gate rows: one before the first rank (upstream gates), one after the last (downstream gates). They are not
  ranks of their own with a rank gap: a gate sits on the frame border, i.e. `FRAME_PAD` (plus the title strip on
  the screen-top side) from the first or last row of units. Gates are edge dummies for spacing (`EDGE_GAP` 18,
  `layout.rs:84`), never nodes;
- one gate per descending edge end, with an arc gate → unit (upstream) or unit → gate (downstream). The arcs follow
  the flow whatever the edge's own direction, so the level has no back-edge because of them;
- the level's own edges (LCA = B), with the orientation of S1 forced (no second cycle breaking);
- the title as a reserved stretch of the gate row on the screen-top side (S5).

Then the unchanged pipeline stages run on the level graph: ranks (longest path with the forced orientation),
ordering, Brandes-Köpf, layer lines, `narrow`. A gate whose member lies on the border row is given the breadth of
the member's port, so its inner piece is one straight leg; any other gate gets a dummy chain through the rows
between (today's long-edge chain, `layout.rs:3066-3149`), which is the corridor the piece runs in.

Output of S2 for B: the frame (bounding box of the units plus `SUB_PAD` and the title strip, exactly
`cluster_raw_boxes`, `scene.rs:5346`, grown over B's own pieces where they lie outside it), the placed units, the
bundles (per member and side: the breadths of its gate slots, in order), and the inner pieces (member ↔ slot).

**S3. A block in its parent's layer** is a contiguous group in one rank:
`wall, slots and free pins in the block's own left-to-right order, wall`.
- The order of bundles is frozen (it is the block's own order). Inside one bundle the slots are interchangeable:
  the parent's ordering decides which edge takes which slot. A free pin may sit anywhere between the walls except
  under the title.
- `pair_sep` (`layout.rs:5379`) between two neighbours of the group is the block's own distance between them, so
  the group is as wide as the frame; after Brandes-Köpf the group is made rigid again (every slot back on
  `block origin + its offset`; the origin is the median of what the alignment asked for).
- Upstream and downstream slots share the one row; in the sweep between ranks r−1 and r only the upstream slots
  have arcs, between r and r+1 only the downstream ones.
- The block's extent along the flow is its rank's extent. Blocks sharing a rank are aligned along the flow by their
  pins: entry pins only → flush with the upstream edge of the band, exit pins only → flush with the downstream
  edge, both or none → centred (shorter legs; mermaid's rule R5).

**S4. Draw the level's edges** with the static builders as today (`scene_from_layout`, `scene.rs:3159`; lanes for
back-edges between two plain nodes, `static_lanes`, `scene.rs:3097`), then the edge stages
(`edgepass::finish_held`, `edgepass.rs:361`) with one addition: an end on a slot or free pin is *pinned*. The ports
stage leaves it where it is, ortho starts from it, the router, the respacing and the nudge never move it. A reversed
block-level edge (S1) is drawn as its forward twin and its points are reversed afterwards.

**S5. The title and the ports** (TD and BT only; in LR/RL the strip lies on a breadth side). The title chip is a
dummy of the gate row on the screen-top side, as wide as the chip (`title_head`, `scene.rs:2885`), ordered directly
after the left wall like a wall (`arrange`, `layout.rs:5534`). No gate or free pin can be placed in its stretch, so
no edge has to cross the chip. A member under the title that has a gate on that side gets its gate beside the chip
and a jog under it; `track_need` (`layout.rs:4002-4051`) grows the gap between the strip and the first row for that
jog only where one exists, and Brandes-Köpf's alignment pulls such a member out from under the title when the row
has room. Titles stay top-left, are never struck, and the scene contract needs no new field.

**S6. Stitch and finish.** Each graph edge = inner pieces on its source chain (innermost first), its LCA piece,
inner pieces on its target chain; consecutive pieces meet at a pin with the same tangent (both run along the flow
there), so their rounded waypoint lists are concatenated as they are. `kind`, `ends`, `meta`, ids come from the
graph edge. The label is the LCA piece's (never inside an end block). Frames get `depth` from the tree and are
sorted outermost first; nodes go back to graph order; edges first, sub-edges after, in order. One whole-scene label
pass (`labels::relabel`, `labels.rs:1693`, static budget) clears label-against-foreign-stroke conflicts that no
single level can see; `refit` (`edgepass.rs:1127`) normalises the canvas once.

### 1.3 Deterministic tie-breaks

| decision | rule |
|---|---|
| unit order of a level before ordering starts | declaration order: a node by its index, a block by the lowest node index inside it (as `compound::split` places a proxy, `compound.rs:364-377`) |
| cycle breaking in H(L) | `rank()`'s: depth-first in unit order and in target-declaration order, whichever reverses fewer arcs (each real edge counts once); on a tie the first. Never derived from the order of edge statements (H16) |
| bundles on a side | the block's own order of their members; never changed by the parent |
| slots inside a bundle | assigned by the parent's ordering; ties by edge index |
| free pin alone on its side | middle of the side's stretch that is free of the title; with other pins: ordered among them by the parent's ordering, `PORT_SPACING` from each neighbour |
| blocks sharing a rank | ordering of the level; ties by declaration order; `centre_largest` (`layout.rs:1936`) as today |
| maps | `Vec` indexed by node / subgraph / edge only; no hash iteration reaches the output |

### 1.4 Cases

| case | rule |
|---|---|
| **Nested subgraphs** | The tree. An edge from outside to a node two blocks down has a gate on each frame and three pieces. Each level is laid out once, innermost first. |
| **Node → whole subgraph** | Ends on a free pin of the block, on the side S1 gives (upstream for a forward edge), clear of the title; the block's inside is not involved. The whole block ranks after the source because the block is one unit (today's rule, test `scene.rs:11600`). |
| **Subgraph → subgraph** | Free pins at both ends. |
| **Member → its own subgraph / subgraph → its own member / subgraph → itself** | S0: entry member as today; inside stub as today; a loop on the block in its parent (new, replaces the zero-length path). |
| **A cycle between blocks** | S1 per level. The edges against the chosen order are `back`: out through the upstream side, in through the downstream side, drawn as forward chains reversed. In the analog this gives actors + code hosting / external services / PROD / the two other servers, with five edges running back up into external services (the same five mermaid reverses). |
| **A cycle between a block and plain nodes of the same level** | The same breaker; an edge with at least one pinned end is drawn as a reversed chain, an edge between two plain nodes keeps today's lane. |
| **A hub linked to many blocks** (the DNS node) | One bundle per side of the hub's block: six slots under the hub at `EDGE_GAP` pitch, lined up with its ports. The parent assigns the six edges to the slots in the order of their targets, so they leave as a fan without crossing each other. B11's comb for fans treats a bundle as one node side. |
| **Empty subgraph** | No frame, no block, as today (`cluster_raw_boxes`); a sub-edge to it is skipped, as the contract says (`scene.rs:130-134`). A subgraph whose only content is empty subgraphs is empty too. |
| **A subgraph with its own `direction`** | The B9 rule is unchanged (H9): the direction holds only while no node-to-node edge crosses the border, the direction differs from the one round it, and two members link to each other (`compound::judge`, `compound.rs:144-189`). Such a block is the same kind of level; its level graph simply flows in its own direction, and it can only have free pins. With a border-crossing edge the direction is ignored, as in mermaid; unlinked members of a `direction` block stay one row across the flow, as today. |
| **LR / RL / BT** | All of S1-S4 work in flow coordinates (`ortho::to_flow`, `ortho.rs:89`). BT: the title strip is on the downstream side, so S5 reserves the exit gate row. LR / RL: the strip is on a breadth side; S5 does nothing. |
| **State diagrams** | A composite is a block. Its own `[*]` nodes and note cards are members already (`parser.rs:5903-5912`, `5979-6003`), so their edges and note links stay inside one level; a note link keeps its builder's elbow (`ortho.rs:684-694`). A lead node (`note left of`) is ranked beside its target inside the target's level. Fork / join bars are fitted per level (`fit_bars`, `scene.rs:7456`), as `compound.rs` fits them per part today; a branch that descends into a composite counts with its pin. A transition to a composite ends on a free pin. |
| **Invisible links** | An arc of H(L) at their LCA level (they rank the two units), no gate, no piece; the scene keeps their slot. |
| **Very large drawings** | The router's cap (`ROUTE_MAX_EDGES` 400, `router.rs:263`) and the layout search budget (`SEARCH_MAX_ELEMENTS` 600, `layout.rs:217`) count per level, as they count per part for B9 today (README:269). A 500-edge drawing with blocks is therefore routed where 0.38 gives up, at a cost bounded by the largest level. Two guards: more than `BLOCK_LEVELS_MAX` (64) levels with edges, or nesting deeper than `BLOCK_DEPTH_MAX` (8): the smallest / deepest blocks are laid out with their parent by today's clustered path, which stays in the code for that. |

---

## 2. Code plan

### 2.1 What changes, what is new, what `compound.rs` becomes

`compound.rs` becomes the module of this design (kept under its name, or renamed `blocks.rs` with `compound` items
re-exported; about 1 500-2 000 new lines plus tests).

| piece | where today | change |
|---|---|---|
| `blocks()`, `judge()`, `Flows` | `compound.rs:134-227` | Keep their meaning ("laid out in its own direction"): the quality harness reads them (`quality.rs:1020`). New `Tree::new(g)` = S0 (chains, owners, children, per-edge LCA and units). |
| `Split`, `Part`, `Slot`, `EdgeOf` | `compound.rs:41-83` | `Split` → `Tree` of `Level`s. `Slot` gains `Gate(edge, end)`; `EdgeOf` gains `Piece { edge, role: Whole / Open(gate) / Aux }`. The three `return None` on a border edge (`compound.rs:326-327`, `336-338`) go away. |
| `split()` | `compound.rs:242-487` | Builds one level graph per level instead of "blocks and the rest"; a child block is a group, not a plain rect node. |
| `scene()` | `compound.rs:668-698` | Post-order over the levels (S2-S4), then S6. |
| `join()` | `compound.rs:603-660` | → `stitch()`: pieces concatenated per graph edge (S6). |
| `route()`, `route_partial()`, `part_base()` | `compound.rs:704-877` | Per level (section 2.2). `part_base` cuts a base edge into its pieces at the frame borders (`cut_to_frames`, `scene.rs:3772`), so no piece has to be stored. |
| `scene_labelled` | `scene.rs:3050-3070` | The `compound::split` hook (`3058-3062`) becomes "any non-empty subgraph → blocks", before `scene_clustered`. `scene_flat` (`3083`) is untouched. |
| `route_labelled`, `route_partial` | `scene.rs:4402-4428`, `4870`, `4909-4912` | Same hooks. |
| `rank()` | `layout.rs:2281` | Optional forced orientation (`back` per edge) and two pinned rows (gate rows); nothing changes when both are absent. |
| `place()`: walls, `apath`, ordering | `layout.rs:3255-3397`, `5487-5593` | A group = walls + slots with frozen bundle order (new unit kind next to "cluster with walls"); slots of one bundle free. The title dummy sorts as a wall. |
| `pair_sep`, Brandes-Köpf | `layout.rs:5379`, `4880` | Fixed separations inside a group; the rigid pass after alignment; flow alignment of blocks in a band. |
| `layer_lines` | `layout.rs:4062-4175` | A gate row has no gap of its own: `FRAME_PAD` plus `track_need` for jogs under the border. |
| `scene_from_layout` | `scene.rs:3159` | A chain that starts or ends on a slot starts or ends at the pin. |
| `Ctx` | `edgepass.rs:74-119` | New `pinned: Vec<[bool; 2]>` per edge end (not `held`, which is per edge and only from `route_partial`). |
| `ports::assign`, `respace` | `ports.rs:774`, `783` | Skip pinned ends; space the others round them. |
| `ortho`, `router::repair`, `nudge`, `spread_close`, `snap_ends` | `ortho.rs:477`, `router.rs:5120`, `4109`, `4360`, `edgepass.rs:671` | Never move a pinned end; a child block is a node, i.e. a hard obstacle, already. |
| `sub_edge_proxies`, `seat_box_sources`, stand-ins, `frame_cut`, `grid_pairs` | `scene.rs:3994`, `layout.rs:2497`, `scene.rs:3455-3519`, `4172` | Unused on the block path (a block is a unit; an edge to it ranks it whole), except `grid_pairs`, which keeps packing a level's loose members. They stay for `layout::layout_clustered` (public, H15) and for the fallback of section 1.4. |
| Public API | | Nothing added, removed or re-defined. Gates and pins are never scene nodes. |

### 2.2 `scene()`, `route()`, `route_sized()`, `route_partial()`, hit-testing, fold

**What runs where.**

| | `scene()` | `route()` / `route_sized()` | `route_partial()` |
|---|---|---|---|
| S0 tree | yes | yes (same function of the graph) | yes |
| S1 orientation | yes | no: by geometry (below) | no |
| S2-S3 layout, gates, corridors, title reservation | yes | no: positions are the host's | no |
| frames | wrapped round the placed members | `cluster_raw_boxes` of the host's positions, exactly as today (`route_clusters`, `scene.rs:5389`) | as today: base frame unless a member moved (`scene.rs:4980-5017`) |
| pins | the block's gate slots | projection (below) | kept for clean levels, projection for dirty ones |
| level edges | static builders + `finish(Static)` with pinned ends | `route_built` + `finish(Routed)` per level, pins as fixed points, child blocks as plain boxes | `route_partial_with` per dirty level |
| S6 stitch | yes | yes | yes |

**Pins in `route()`** (the host may put blocks anywhere, so the rule is geometric and total):
- side of block B for an edge to a unit outside: the other unit's box wholly downstream of B's frame → downstream
  side; wholly upstream → upstream side; overlapping along the flow (the host put them side by side) → the breadth
  side facing it. Breadth-side pins exist only here;
- place: the member's port projected onto that side when the straight leg to the border is clear of B's other
  units, else the nearest free stretch of the side; ends of one bundle in the order of their far ends; a free pin
  as in 1.3. The title chip stays the obstacle it is.

So in `route()` a block whose crossing ends all have a straight leg costs nothing; a level runs the edge stages
only when it has level edges. Because a foreign block is a plain box at its parent's level, the router cannot run
an inter-block edge through it (today a foreign frame is only priced, `router.rs:1-69`, which is why the experiment
of `understand-flowmaid.md` 2.5 failed).

**`route_partial()`**: a level is dirty when one of its direct units moved, a child block's frame moved or
resized, or one of its pins moved. A drag of one member dirties its level and every ancestor (the frame follows
it), nothing else. Clean levels keep their base pieces (cut out of the base at the frame borders). Each dirty level
is one `route_partial_with` (`scene.rs:4889`) with its child frames passed as `sized` proxies, as
`compound::route_partial` does today (`compound.rs:858-873`). The fallbacks stay: base not lining up → `route()`
(`same_graph`, `clusters_line_up`, `scene.rs:5277`); more than half redrawn → `route()` (`PARTIAL_ROUTE_SHARE`).

**Is `scene()` equal to `route()` at its own positions?** Frames: equal wherever no inner piece lies outside the
raw box; where one does, both grow the frame over the level's own pieces with one shared function (today only the
static scene widens, `scene.rs:3535-3599`, hence the 71-617 px differences). Edges: equal for every end with a
straight leg; an end through a corridor may take another stretch of the side in `route()`. That is less different
than today (19 of 27 edges differ on the analog), and `route_partial` keeps the static pieces of every level the
drag does not touch.

**Hit-testing** reads the `Scene` only (`scene.rs:257`, `314`, `337`). Frames nest by construction and sibling
frames cannot overlap (they are separate units of one layout). An edge is one stitched `SceneEdge`, so `edge_at`
follows the painted stroke; `edge_picking_matches_the_painted_strokes_on_every_fixture` (`scene.rs:10822`) covers
whatever is drawn.

**Fold / compact, `to_mermaid`, `advance`, class, ER.** `fold::scene_compact` refuses any graph with subgraphs
(`fold.rs:247`) and keeps returning `scene()`. `to_mermaid` works on the graph. `advance`, class and ER build
graphs without subgraphs and never reach the block path.

### 2.3 Expected cost

Measured: whole CLI runs (parse, scene, SVG, process start; mean of 10) of 0.38.0 and of the prototype, which lays
every level out **twice** (its first pass only finds the orientation that S1 gets from the topology).

| input | levels | 0.38 CLI | prototype CLI (2 passes) | 0.38 `scene_ms` (perf.tsv) | B12 `scene_ms`, estimate | static limit (PLAN) |
|---|---:|---:|---:|---:|---:|---:|
| infra-overview | 6 | 11.2 ms | 21.9 ms | 6.6 | 9-11 | (new input) |
| bench-75 | 6 | 48.4 ms | 74.0 ms | 39.4 | 35-45 | 61.9 |
| dense-td | 11 | 37.8 ms | 42.0 ms | 25.4 | 18-25 | 50.7 |
| advanced | 4 | 3.4 ms | 4.6 ms | 1.28 | 1.5-1.9 | 1.42 |
| subgraph | 3 | 2.6 ms | 2.9 ms | 0.16 | 0.25-0.35 | 0.16 |

The estimates are the prototype's times less the fixed CLI cost and less its first pass; the group layout of S3 is
not in the prototype and will add ordering work at levels with many pins (bench-75's root: 138). They are
estimates, not measurements.

- Large inputs get cheaper or stay level: six small problems instead of one with every wall dummy on every layer.
- **Small inputs pay a fixed cost per level and will exceed the 2× static budget**: `subgraph.mmd` is at its limit
  today and has three levels; `advanced.mmd` is within 10 % of its limit. In absolute terms this is 0.1-0.5 ms. It
  needs a decision (section 5).
- `route()`: levels with edges only. bench-75: the root (69 pieces, 138 fixed points) plus 5 leaf blocks with 6
  edges between them: expected 9-12 ms against 9.1 today (limit 15.8), worst drag frame 18-22 ms against 17.0
  (limit 25.3). dense-td: 11 levels, expected 6-8 ms against 6.6 (limit 9.2). The analog: 6 levels, expected
  2-3 ms against 1.9.
- `route_partial()`: one leaf level plus its ancestors; on bench-75 one leaf and the root, expected 3-4 ms (limit 6).

---

## 3. The prototype

### 3.1 What it is

A hack in a copy of the tree: `d-recursive/tree/src/blocks.rs` plus hooks in `scene.rs`, `edgepass.rs`,
`ortho.rs`, `layout.rs`, `labels.rs`, `model.rs`, `lib.rs` (`d-recursive/prototype.diff`). It runs the real pipeline: every
level is one `scene_labelled` call, every picture is `to_svg` of a real `Scene`. It covers `scene()` only.

What it implements of the design: S0; S2 with gates and corridors; the bottom-up composition; pinned ends in the
edge stages; S6 without the final label pass; nested blocks; edges to whole blocks; member → own block; invisible
links.

Shortcuts, each of which costs quality in the pictures:

| shortcut | instead of | effect |
|---|---|---|
| two passes; the first, without gates, tells which block-level edges run against the flow | S1 | twice the time |
| a gate is a 2 px node one full rank away from the frame | a dummy on the border | 50-70 px of extra gap on every side with ports; two extra bends per piece whenever gate and port do not line up |
| in its parent a block is one plain box; the ports stage places the ends, then they are moved onto the pins and held | the group of S3 | the parent's ordering and coordinates do not know where on the block an edge leaves: long sideways runs, crossings, lanes that are no longer spaced |
| gates of a bundle ordered by where the far end was in the first pass | slot assignment by the parent | fine on the analog |
| the title slides right to a free stretch of the top side (a thread-local, painter and `title_head` patched) | S5 | works for PROD and STG; DR has no free stretch as wide as its title, so three edges still go round the chip and enter nodes from the side |
| `route()` / `route_partial()` untouched | 2.2 | after the first drag the picture is 0.38's routing on the new positions |
| graphs with a B9 block go to `compound.rs` as before | one level kind | `state.mmd` is byte-identical to 0.38 |

One earlier variant drew the level pieces with the position-driven pipeline (`route_labelled`) between pin nodes
(env `FMB_ROUTEPASS=1`); it was worse (more detours), which is why the static pipeline with pinned ends is the
default. `FMB_SEED=1` seeds each block's member order with where its outside neighbours were; on the analog it
changed 60 crossings into 64 and produced two diagonal fallbacks, so it is off.

### 3.2 The pictures

- `d-recursive/infra-overview.proto.png` (the analog), next to `b12/infra-overview.fm038.png` and
  `metrics/mermaid-svg/infra-overview.mermaid.png`.
- `d-recursive/advanced.proto.png`, `subgraph.proto.png`, `fanout-frames.proto.png` (each with its `.fm038.png`).
- `d-recursive/bench-75.proto.png`, `dense-td.proto.png`: the dense inputs, where the prototype's edges fail.
- `d-recursive/dump/lg2-PROD.png`, `lg2-DR.png`: one block's own layout with its gates.
- `d-recursive/dump/root-free-ends.png`: the root level with blocks as plain boxes and the ends where the ports
  stage wanted them. It is what the root looks like when the level is free to place the ends; it is far tidier
  than the final analog picture, and the difference between the two is the cost of the third shortcut.

### 3.3 Numbers (block-metrics, `d-recursive/metrics/metrics.tsv`)

The analog:

| metric | 0.38.0 | prototype | mermaid 12.2 | acceptance (must) |
|---|---:|---:|---:|---|
| canvas | 2787 × 1225 | 2786 × 1755 | 2199 × 2358 | |
| area (Mpx²) | 3.41 | 4.89 | 5.18 | ≤ mermaid's: pass (target ≤ 4.27: miss) |
| compactness mean / max | 6.90 / 11.43 | 3.52 / 5.52 | 3.25 / 4.59 | ≤ 4.0: pass / ≤ 5.5: fail by 0.02 |
| largest frame share of the flow extent | 0.767 | 0.219 | 0.308 | ≤ 0.40: pass |
| sibling pairs side by side / stacked | 6 / 4 | 1 / 9 | 2 / 8 | |
| side-by-side pairs joined by an edge | 5 | 0 | 0 | = 0: pass |
| interleaved pairs | 3 | 0 | 0 | = 0: pass |
| runs through a foreign frame | 3 (2012 px) | 0 | 0 | = 0: pass |
| node hits, frame overlaps, label / label, label / node | 0 | 0 | 0 | = 0: pass |
| crossings | 24 | 60 | 16 | ≤ 24: **fail** |
| total edge length (px) | 22 350 | 38 946 | 30 835 | ≤ mermaid's: **fail** |
| longest straight run (px) | 1175 | 1660 | 1236 | ≤ 1240: **fail** |
| bends | 109 | 230 | 100 | |
| smallest gap between parallel lanes (px) | 9.3 | 7.0 | 12.0 | ≥ 8: **fail** |
| label offset from the midpoint, mean / max | 0.198 / 0.332 | 0.204 / 0.467 | 0.130 / 0.329 | ≤ 0.20 / ≤ 0.35: **fail** / **fail** |
| labels with a foreign line within 12 px | 8 | 11 | 0 | ≤ 4: **fail** |
| labels struck by a foreign line | 0 | 1 | 0 | = 0: **fail** |

The other inputs (prototype / 0.38 / mermaid):

| input | compactness max | largest span | runs through a foreign frame | node hits | crossings | edge length | area (Mpx²) |
|---|---|---|---|---|---|---|---|
| advanced | 10.74 / 8.28 / 8.24 | 0.398 / 0.377 / 0.428 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 3102 / 2346 / 5024 | 0.90 / 0.74 / 1.34 |
| subgraph | 9.42 / 7.49 / 5.82 | 0.546 / 0.562 / 0.590 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1081 / 632 / 843 | 0.38 / 0.29 / 0.35 |
| fanout-frames | 3.37 / 3.37 / 2.66 | 0.637 / 0.462 / 0.618 | 0 / 0 / 0 | 0 / 0 / 0 | 1 / 0 / 0 | 4425 / 4564 / 4253 | 1.67 / 1.31 / 1.47 |
| state | identical to 0.38 (B9 path) | | | | | | |
| bench-75 | 19.22 / 52.03 / 9.88 | 0.137 / 0.464 / 0.161 | 0 / 4 / 0 | **4** / 0 / 0 | **668** / 302 / 228 | 85 155 / 80 453 / 106 034 | 3.38 / 4.37 / 4.58 |
| dense-lr | 8.55 / 11.78 / 6.92 | 0.204 / 0.603 / 0.195 | 1 (21 px) / 8 / 0 | **2** / 0 / 0 | **80** / 46 / 10 | 83 169 / 38 537 / 78 383 | 15.2 / 7.98 / 19.1 |
| dense-td | 14.97 / 24.66 / 15.12 | 0.268 / 0.795 / 0.308 | 2 (2244 px) / 12 / 0 | **7** / 0 / 0 | **521** / 80 / 68 | 174 350 / 63 286 / 142 183 | 14.3 / 7.67 / 18.1 |

`judge` on the eight inputs: 138 checks, 30 musts failed, 14 targets missed. On the analog 13 must-lines fail: the
edge and label rules of the table above (some are listed under two rule sets) and compactness max by 0.02; every
other block rule passes.

Byte identity: the prototype's SVG equals 0.38.0's byte for byte on `demo`, `fanout`, `long-labels`, `lr`,
`shapes`, `stress`, `back-edge`, `wide-labels` (no subgraphs), `class`, `er`, `sequence`, and `state` (B9).

### 3.4 Frankly: what still looks worse than mermaid, and why

Looking at `infra-overview.proto.png` beside the mermaid render:

1. **The space between the blocks is messier than mermaid's.** mermaid has evenly pitched lanes and 16 crossings;
   the prototype has 60 crossings, lanes 7 px apart in places, and runs up to 1 660 px long. Three causes, in order
   of weight: the root level placed PROD at the far left and the code-hosting block at the top right without
   knowing that every edge of external services leaves from one node at its left end and arrives at one node at
   its right end (S3 is missing); every piece has two bends more than it needs because the gates stand a rank away
   from the frame; ends moved after the ports stage are held, so the nudge no longer spaces their lanes.
2. **Labels**: 11 of 20 chips have a foreign line within 12 px (mermaid 0, 0.38 8), one is struck, one sits 0.47 of
   its edge's length from the middle. They are placed per level on lanes nothing spaces.
3. **Three edges still enter nodes from the side in the standby block** (title chip in the way, no free stretch):
   the case S5 is designed for and the prototype's slide cannot solve.
4. **The canvas has a hole** right of PROD and is 2 786 px wide for a drawing whose widest band needs about 1 900.
5. **One edge is still a free S-curve** (`DNS → TR2`; in `advanced`, `W2 → Users`): a later stage redrew a held
   edge as a cubic.
6. **Dense inputs are bad**: bench-75 has 668 crossings and 4 node hits, dense-td 521 and 7. Their blocks are
   compact and stacked like mermaid's, but with 138 pinned ends on one level the third shortcut breaks down
   completely. No conclusion about dense inputs can be drawn from the prototype except that S3 is not optional.
7. Small inputs got taller (`subgraph` 758 → 982 px, `advanced` 1236 → 1427 px): the gate rank on each side.

What is already as good as or better than mermaid on the analog: each subgraph is one compact block (largest
frame 22 % of the height against 31 %); bands in mermaid's order with the three actors and code hosting on top; no
edge through a foreign frame; no title struck (mermaid: 7); the inside of PROD and of the staging block reads
cleanly (`dump/lg2-PROD.png`); a smaller canvas than mermaid's.

Against the user's complaint: the tall "External services" frame, the side-by-side servers and the runs across
foreign frames are gone. The corridor of parallel lines with far-away labels is **not** fixed by the prototype; it
moved into the gaps between the bands.

---

## 4. Effects, pinned tests, risks

### 4.1 Diagrams without subgraphs

Byte-identical, by construction and by test:
- the block path is entered only when some subgraph has a node; a flat graph goes to `scene_flat` as today;
- every change in shared code (`rank`, `place`, `pair_sep`, ports, ortho, router) is behind "this level graph has
  gates, groups or pinned ends", which a flat graph never has. (The prototype's one unguarded line, `pair_sep`
  treating a node of at most 2.5 px as a dummy, does not exist in the design: gates are dummies.)
- class and ER build graphs without subgraphs (`class.rs:194`, `er.rs:155`).

There are no SVG snapshots today, so this needs a new test: the SVG of every flat corpus input (examples,
`tests/fixtures/quality/{back-edge, wide-labels}`, `tests/fixtures/outline`, class, ER) compared with hashes
recorded from the release before B12.

### 4.2 Diagrams with subgraphs but no edge across a border

They become blocks too (the user's decision: the default for every diagram with subgraphs). Their blocks have free
pins only.
- `state.mmd`: its composite is a B9 block today; the rest round it is the root level with one proxy, as now.
  Expected unchanged; it must be asserted.
- `fanout-frames`: changes. In the prototype: 842 × 1984 against 566 × 2311, one crossing against none, area
  +28 %. Each frame's inside is the same grid; what moved is the root level (six proxies and two plain nodes
  instead of six clusters). The ratchets of this fixture (all-zero ceilings, `GAP_FLOORS` 40, 0 crossings) are the
  canary: B12 has to meet them unchanged.

### 4.3 Pinned tests and gates that will move

Will change value (to be re-baselined with the reason "blocks", never loosened to hide a regression):
- `tests/quality.rs`: `CEILINGS` (bench-75, dense-lr, dense-td; `:14-58`), `GAP_FLOORS` (`:64-109`),
  `BORDER_LEN_MAX` (`:245`), `LBL_EDGE_MAX` (`:296`), `XGAP_MAX` (`:415`), `CHANNEL_MAX` / `EDGE_CHIP_MAX`
  (`:481`, `:499`), `DEFECT_BASELINE` (`x_drawn`, `band_len`; `:730`), `INK_MEAN_MIN` (`:754`; stacking adds gaps),
  `DRAG_JUMPS_MAX` (`:967`), and the declaration-order checks (`:1038`, `:1098-1142`: now the *length along the
  flow* is what could move, and the unit-order rule of 1.3 must keep both within 10 %).
- New gates from block-metrics (S8 of the analysis): compactness, largest frame share, side-by-side linked pairs,
  runs through a foreign frame.

Must keep passing as written:
- `scene.rs:11186` (two linked subgraphs stack), `11566` / `11587` (`assert_clean_clusters`), `11600`, `11629`,
  `12225`, `12249`, `12284`, `12327`, `12357`, `14708`, `14726`, `14814`;
- `render.rs:295-357` (advanced: CI above Platform; `Reg --> Platform` onto the top side; `Rollback --> Platform`
  up into the bottom side; `Dev --> CI` drawn to `Build`): all four hold in the prototype picture;
- `compound.rs:986-1191` (B9);
- `layout.rs:5605-5833` and every other test that calls `layout::layout_clustered` directly: that function and
  `seat_box_sources` stay.

At risk, to be re-examined one by one:
- `scene.rs:12048` (hub-fed leaves pack into a grid): `grid_pairs` must run per level;
- `scene.rs:12102` (two wide unconnected boxes stack), `layout.rs:5833` (a row of shallow frames wraps): `narrow`
  now sees proxies, not clusters;
- `scene.rs:12129` (the largest box centred among its siblings): `centre_largest` on proxies;
- the scene-against-route agreements (`scene.rs:11010`, `14999`, `15151`, `16564`, `16712`, `11733`;
  `compound.rs:1112`).

Docs: README Subgraphs (`:282-289`), How it works (`:797-806`), guarantees (`:811-817`), the benchmark rows and the
drag numbers (`:825-857`), the three comparison images with subgraphs, `docs/ex-subgraph.svg`, CHANGELOG.

### 4.4 Risks

1. **Inter-block edge quality is unproven.** The prototype is worse than 0.38 on crossings and edge length on
   every input with border edges. The design's answer (S3: the block as a group of its pins in the parent's
   ordering and coordinates, gates on the border) is not in the prototype. It reuses the wall machinery, but
   frozen-order groups with fixed inner distances inside Brandes-Köpf are new, and it is the largest piece of work
   in the package. If it underdelivers, the package does not meet the acceptance list.
2. **Member order inside a block is decided without the outside.** A block whose members have no edges among them
   (external services, the standby block) keeps declaration order, so a hub may sit at the far end from its
   targets. One bounded re-seed round per such block is the obvious remedy; the prototype's attempt at it did not
   help (64 crossings against 60), so its value is open.
3. **Static time on small inputs** exceeds the 2× budget (2.3).
4. **`route()` parity and drag feel.** Pins come from two rules (layout in `scene()`, projection in `route()`).
   Ends with a straight leg agree; ends through a corridor may jump on the first frame of a drag in their level.
   `DRAG_JUMPS_MAX` is at 0 for three fixtures.
5. **Area.** Stacking blocks along the flow is taller. The analog: 3.41 → 4.89 Mpx² in the prototype (mermaid
   5.18; target 4.27). Gates on the border give back roughly 100 px per band; whether that reaches the target is
   not measured.
6. **Title against ports** (S5) costs height in exactly the blocks whose first-row member under the title has
   upstream links, and is not prototyped.
7. **A second code path stays** (today's clustered layout: public `layout_clustered`, the fallback of 1.4), with
   its tests.
8. **B11 and the class/state leftovers land first** in the same files (`ortho.rs`, `router.rs`, `ports.rs`,
   `casing.rs`): hops at crossings work on finished scene edges and are unaffected; the comb for fans must learn
   that a bundle is one node side.

---

## 5. Decisions this design needs from the user

1. **The static budget for small clustered inputs** (PLAN invariant 1: `scene_ms` at most 2× baseline per file).
   `subgraph.mmd` (0.16 ms, at its limit) and `advanced.mmd` (1.28 ms, limit 1.42) will go over it by a few tenths
   of a millisecond, because every level pays the pipeline's fixed cost. Either the limit gets an absolute floor
   for B12 (as the drag budgets have: "or baseline + 1.0 ms"), or B12 must first cut that fixed cost.
2. **Order of the bands when the blocks form a cycle.** The design keeps `rank()`'s breaker, which on the analog
   gives mermaid's order (external services above the servers, five edges running back up). The order with the
   fewest reversed edges would put PROD above external services (one edge back instead of five). I recommend the
   first: it is what a reader of the source expects and what the user's reference shows.
3. **Phasing.** The static scene (S0-S6) and the routed one (2.2) are separable. Shipping the static part alone
   would mean the playground shows 0.38's routing on the new positions after the first drag, which the experiment
   in `understand-flowmaid.md` 2.5 showed to be worse than either. I recommend one release for both.
