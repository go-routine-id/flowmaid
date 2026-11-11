# B12 "subgraphs as blocks" — specification

Date 2026-11-10. Written for an implementer who has seen none of the design work.

- `B` below is `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12`.
- Every `file:line` points into `B/tree/src` (dev = release 0.38.0, commit 0366084). **You will work on top of package
  B11 (crossing hops in the SVG painter, comb bundling for fans) and a class/state leftovers package, so find code by
  the function names given here; the line numbers only say where that function was in 0.38.0.**
- **Two releases are named.** *0.38* (or 0.38.0) is release 0.38.0 itself: the release the complaint is about, the
  block-metrics baseline `B/metrics/baseline-038.tsv`, the pictures, and every number this spec quotes from
  `tests/quality.rs` (B11 has already moved several of those; a quoted number only shows the size). *The starting
  release* is the release B12 branches from (B11 and the leftovers on top of 0.38). The harness ratchets, B12.0's
  recordings, the off-identity test and every "not above the reference" in 4.3 use the starting release; 4.3 says
  exactly which value "the reference" is.
- Read `scratchpad/PLAN.md` first. Its invariants are not negotiable.
- `B/private/` holds the user's two screenshots. Look at them, never copy them or any name or address in them anywhere
  else. Everything written down uses the sanitized analog `B/in/infra-overview.mmd` and its ids (U1, DNS, EXT, PROD
  ...).
- **The analog is derived from a private diagram and is not cleared for the public repository (decision D5).** The
  analog, `infra-overview-v2`, the five permutations and their direction copies are *local acceptance inputs*: they are
  judged from the workspace and never committed. No file name, node id, node label or number of theirs goes into the
  repository: not into fixtures, tests, test names, code comments, commit messages, the CHANGELOG or the README.
  Comments and commit messages cite the public fixture and the ratio / hub / cycle fixtures. The repository's fixture
  and README image use the synthetic diagram `B/revise/in/blocks-overview.mmd` (called *the public fixture* below),
  which has the same kinds of difficulty and a different structure.
- The user's answers to the open questions are in `B/USER-DECISIONS.md`; appendix B applies them.
- Background, in the order worth reading: `B/understand-flowmaid.md` (today's pipeline), `B/understand-mermaid.md` (what
  mermaid's ELK does, rules R1-R10), `B/metrics/README.md` (the yardstick), `B/design-global.md` (the design this spec
  builds), `B/design-hybrid.md` and `B/design-recursive.md` (source of the grafts).

## 0. Decisions in one page

**Lead's decisions after the second review pass (2026-11-10; they override any sentence below that says otherwise):**

- **D7 is confirmed: option (b), block mode for every diagram with a border link.** The user chose "Default untuk
  semua" (every diagram with subgraphs takes the block layout; diagrams without subgraphs do not change) when the
  package was agreed, and later gave the bar "pokoknya mirip atau lebih bagus dan rapi dari mermaid js 12". Nothing
  waits on a further confirmation.
- **`lateral_x` on bench-75, dense-lr and dense-td: the must is "not above mermaid's count" (3 / 0 / 1), the target
  is 0.** The user's bar is mermaid; a stricter must would only add a stop rule. The README sentence of 4.7 may claim
  "none" only for the inputs where the measured count is 0.
- **The workflow runs B12 as four packages**, cut at the checkpoints as section 5 allows, each one
  implement-verify-fix cycle: `b12a1-structure` = B12.0-B12.6 (checkpoints S, A1), `b12a2-geometry` = B12.7-B12.12
  (A2), `b12a3-route` = B12.13 (R), `b12b-default` = B12.14-B12.16 (C). The first three end with the switch off and
  byte-identical default output, are merged to `dev`, and are not released; the release (0.40.0) follows the fourth.
  The lead does the release itself (version, release branch, pull requests); B12.16 prepares the flip, the ratchets,
  the README text and images and the CHANGELOG entry under `[Unreleased]`.
- **B12.0's workspace tooling** (block-metrics columns, judge rules, reference tables) is reviewed by the verifier of
  `b12a1-structure`, who re-derives at least three rows by hand.
- **Disk:** the suite-on run's second cargo target directory is deleted after each checkpoint report is written.
- **Order and release number (user decisions, 2026-11-11):** B12 starts right after B11 (released as 0.39.0) and does
  not wait for the class/state leftovers package. That package now runs AT THE SAME TIME in another worktree (branch
  `ccr-ccc6d89a-5cxd7e-par`); it skips the two block-placement items and keeps its state/compound edits small. The
  "starting release" of every baseline in this spec is the commit B12.0 starts from (B11's head).
  **Release number:** B12 takes the next free number when it is released (0.40.0 if it passes before the leftovers
  package, otherwise 0.41.0). Wherever this spec says "0.40.0", read "B12's release". Never write a version number
  into code, tests or fixtures; the CHANGELOG entry goes under `[Unreleased]`; the lead bumps the version.
  **Merges between B12 packages:** the lead merges `dev` (the 0.39.0 release commits, later the leftovers package)
  into this branch only between two B12 packages, never while one runs, and re-takes B12.0's baselines in that step
  when a default drawing changed. A B12 implementer who finds, at the start of its package, a baseline that differs
  for a class or state input because of such a merge re-takes that baseline with B12.0's scripts and says so in its
  report; this is not a loosened gate. A difference on a flowchart input is never explained this way: report it.

- **After checkpoint A1's first report (lead, 2026-11-11 ~06:30 UTC; read `reports/checkpoint-A1.md` first).** A1's
  stop rule fired on drawn crossings: the analog 21 against 20, the public fixture 9 against 7, perm-b 32 against 23,
  perm-e 40 against 22, dense-td 151 against 74.6. **No must is changed.** What follows is the order of the work, which
  is the lead's to set:
  1. **O2's weighted adjacent exchange belongs to `b12a1-structure` and was not built.** Build it, then take the table
     crossings x starts again.
  2. **Title-clear entries (2.8, listed under B12.11) may be pulled forward into `b12a1-structure`** if A1's
     drawn-crossings musts cannot be met without them. The report shows the public fixture at 5 layered crossings and 9
     drawn, the difference being entries the router takes round a title chip. Build them as 2.8 specifies, with their
     tests, in a commit of their own, and say so in the checkpoint report so that `b12a2-geometry` does not build them
     again. Pulling an item forward is not a loosened gate. Judging A1's drawn crossings by the layered count would be
     one, and is not allowed.
  3. **dense-td (151 drawn against the start's 66; 98 layered) is a structure problem and belongs to this package.**
     Find where the block layering or ordering loses to the flat layout (the report: one block as wide as the drawing
     lies between two bands, and every edge past it runs round it) and fix the rule: block mode must not draw a dense
     diagram worse than the release before it. If that cannot be done inside the design, stop and report the
     measurements with pictures; the lead takes it to the user. (The fallback then on the table: a drawing keeps the
     old path when its block layout has more crossings. It costs a second layout and changes decision D7, so it is
     the user's to decide, not the implementer's and not the lead's.)
  4. **B12.3's wrap of a long row of members without inner links is deferred to B12.12**, where its acceptance lives
     (the report found its trigger, as worded, wraps rows the layers of B12.3's own check keep whole).
     `b12a1-structure` does not build it and is not failed for it.

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
   - **Layering is `d-hybrid`'s `block_rank`**: node-level cycle breakers kept where they break a cycle of their own
     level (2.3 L0), block order by two declaration-order walks with an exact-minimum safety net per component, no
     interleaving of siblings, the lane rule; plus one fix (back flags are recomputed from the final layers).
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
   - **Not built by anyone yet**: the canonical start order, the start scheme and the key-order tie-break (2.4), the
     coordinate rule of 2.5, gap and label room (2.6), the frame growth of 2.7, flow-facing ends, free pins and
     title-clear entries (2.8), the corridors in `route()` (2.9), level-first ordering below the root level, L0's test
     of a mark, the lane rule's second condition and L5's pair rule (2.3; each checked on paper and by script only),
     block mode on the inputs whose rows 0.38 already stacks (4.3 e).
   - **No build has met these musts of 4.3**: `lbl_crowd` 0 (best: 8); crossings <= 17 on v2 (best: 20), <= 20 on
     perm-a and perm-d (best: 22 and 22), <= 23 on perm-b (25); the edge-shape rows (`bends` and `bends_max` on the analog, v2
     and the public fixture: best 141 / 18, 127 / 15 and 128 / 14 against 120 / 8, 114 / 8 and 110 / 8; `detour_max` on
     the analog: 3.61 against 1.63); `lbl_endpx_mean` with gap room, on
     the analog and on v2 (measured only with crushed gaps); the harness totals over the dense fixtures (experiment:
     `x_drawn` 450 static against 0.38's 428); every routed row of 4.3 (h) (experiment without corridors: 28 crossings,
     3 struck chips, 7 crowded on the analog).
   Section 4 therefore has checkpoints with stop rules: if a checkpoint's gate cannot be met, stop and report; a gate
   is never re-baselined to pass (decision D6).
5. **Two packages (decision D2), one internal switch.** B12a builds the whole block path, static and routed, behind a
   switch that is off in every ordinary build: nothing visible, no release, byte-identical default output. B12b adds
   the last edge kinds, runs the acceptance, flips the default and releases 0.40.0. Items, checkpoints and each
   package's own acceptance are in 4.1, 4.2 and section 5.

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

One level is exempt from the first two rules: a level the lane rule (2.3 L4) leaves side by side. The rule fires only
when every stacking order would turn more than a third of the level's links against the flow **and** some path enters
one of its blocks twice (a ping-pong between two swimlanes). A there-and-back exchange between two blocks (a request
down, a response up) is stacked.

On labels: the complaint names "labels far from both ends" in a corridor of eight parallel lines. Distance to the ends
is not what separates the two engines (mermaid's labels are farther from their ends: 558 px against 309 on the analog);
**crowding** is (0.38: 8 of 20 chips have a foreign line within 12 px; mermaid: none). So the label goal is: no chip
struck, no chip crowded, and the distance to the ends below mermaid's and not above 0.38's by more than a quarter
(4.3).

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
| a request and its response between two blocks: 0.38.0, mermaid | `B/check-fix-a/out/reqresp.038.png`, `B/check-fix-a/mm/reqresp.mermaid.png` |

The analog differs from the real diagram in one edge (`PG --> DRAPP`; the real one runs from the standby stack to the
database). `B/u-mermaid/in/infra-overview-v2.mmd` has it turned; with it the three servers stack one under another as in
the user's mermaid screenshot. **Both files are acceptance inputs.**

Target picture: actors / External services + Code hosting / PROD / the two other servers (v2: STANDBY / PROD /
DEV-STAGING). External services reads GW DNS TG OAI AZ: O4b puts the edge-less GW first, as declared, and DNS next to
the end of its block that faces its corridor. The hub's lines run down one corridor beside PROD; the returns to the
chat bot run straight up between blocks.

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
| most bends on one edge / longest detour | 8 / 1.78 | 7 / 1.36 | | 18 / 3.61 | <= 8 / <= 1.63 |
| edge length | 22350 | 30835 | 20356 | 20488 | <= mermaid's |
| labels struck / crowded | 0 / 8 | 0 / 0 | 0 / 9 | 3 / 9 | 0 / 0 |

What the experiment's picture still gets wrong, and which step owns it: PROD is wider than its content (2.5 X2); the two
bottom frames start 17 px apart (2.5 X4); edges stair-step, up to 18 bends on one edge (2.5 X1-X3); labels sit on lines
in the corridors, three are struck, three lie inside an end block and one sits 16.5 px off its own line (2.6); six
edges, `U1 -> DNS` and `DNS -> PSDEV` among them, cross a frame through a lateral side, 7 crossings in all (2.8; 0.38:
21 on 14 edges), and six ends meet their node on a lateral side (2.8); the three node -> block edges end 14, 14 and 32
px from a frame corner (2.8 free pins); the dotted `PSPROD -> TG` goes round the standby block, 2771 px for a detour of
3.61 (2.4, ordering below the root; 2.8 E3).

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
| frame crossings through a lateral side (`lateral_x`) | 14 on 11 edges | 0 (measured, 4.3) | 2 | 0 |
| crossings | 14 | 5 | 7 | <= 7 |
| most bends on one edge / longest detour | 12 / 1.45 | 6 / 1.19 | 14 / 1.27 | <= 8 / <= 1.43 |
| edge length | 17204 | 20125 | 15038 | <= mermaid's |
| labels struck / crowded | 0 / 5 | 0 / 0 | 0 / 4 | 0 / 0 |

### 1.3 What must not change

1. **With the internal switch off, nothing changes at all**: every output of `scene()`, `route()` and
   `route_partial()` and every renderer's SVG is byte-identical to the starting release, on every input, in every
   commit of B12a and of B12b before its last item (4.1; the off-identity test of B12.0).
2. **Diagrams without subgraphs, and class / ER / sequence and every other diagram type: byte-identical SVG for
   good.** Every line B12 adds to shared code sits behind "clustered". There is no SVG snapshot today; B12.0 adds the
   hash test that stays.
3. **A diagram with subgraphs that is not in block mode (2.2: no link joins a block to anything outside it):
   byte-identical static SVG**, unless it holds one of the degenerate own-box edges of 2.8, which are fixed for every
   clustered diagram. Today that is `examples/state.mmd` (its composite is a B9 direction block), the probes
   `B/u-mermaid/probes/{p01-iso, p02-iso-dir, p09-disconnected, p13-inner-dir-ext}.mmd` and the fixtures B12.0 adds
   for this rule (`iso-lane` and others). Their `route()` and `route_partial()` change in one respect only: the frames
   are grown by the function the static scene uses (2.7 F3), so a dragged block's frame keeps the room of the edges
   that did not move.
4. PLAN invariants 1-7: the interactive API and its speed (`scene`, `scene_sized`, `route`, `route_sized`,
   `route_partial`, hit-testing), the scene geometry contract (first `Graph::edges.len()` scene edges index-parallel,
   sub-edges after, frames outermost first, waypoint semantics), `fold::scene_compact`, `to_mermaid`, the `advance`
   module, determinism, zero dependencies, MSRV 1.75, wasm32, portable SVG.
5. B9: a subgraph with its own `direction`, an inner edge and no border edge is laid out by `compound.rs` exactly as
   today; one with a border edge takes the direction round it.
6. What flowmaid already does better than mermaid (PLAN invariant 4): titles never struck, white label chips, compact
   canvas, straight spines (inside a block and across a border), source order as the tie-break (2.4 O4b), edges into a
   subgraph ending on the frame clear of the chip, stable output under re-ordering of edge statements (2.4, 2.11).
7. Public API: `layout::{layout_clustered, layout_sized, LayoutResult, Placed}` keep their signatures.
   `layout_clustered` returns block layouts in block mode; say so in its doc comment and in the CHANGELOG.
8. **Diagrams 0.38 already draws as stacked blocks are not drawn worse.** Block mode re-lays them (2.2); 4.3 (e) gates
   each of them against the reference, picture by picture, and the rows of `BLOCK_CEILINGS` in `tests/quality.rs`
   (4.5) keep the harness half of that gate in the repository.

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
  Self-loops are no links, and neither are the degenerate own-box edges below. A link is *drawn* when it is a real edge
  or a stand-in (weight 1); a rank-only pair weighs 0.
- **Augmented graph**: the graph with one stand-in edge appended per sub-edge, as `scene_clustered` builds it today
  (its `ga`). It is built by one constructor, `levels::augmented`, that the three entry points share (2.2).
- **Degenerate own-box edge**: a sub-edge whose two ends are one subgraph (`S --> S`), or a subgraph and its own nested
  subgraph (`OUT --> IN`, `IN --> OUT`). Today's constructor gives each a stand-in between an exit member and an entry
  member and a rank-only group, although the edge is drawn on the frame (2.8). `augmented` still builds the pair, for
  the code that draws the edge, and **flags** it. A flagged stand-in or group is no link, with the switch on, on every
  path: it is never a border link and never decides block mode, it is left out of ranking and ordering, and it gets no
  chain and no label dummy. So the rows of a diagram that holds one are the rows of the same diagram without that edge,
  in block mode and outside it (0.38: `S --> S` stacks the members of an otherwise loose block, and block mode would
  flip with the declaration order of S's members). `subgraph -> its own member` has no stand-in today and needs no
  flag; `member -> its own subgraph` is not degenerate (2.8).
- A link `u -> v` **belongs to** the level of the lowest common ancestor of `u` and `v` and joins there the two units
  `cu`, `cv` that hold `u` and `v`. A **border link** is a real edge (visible or invisible) or an unflagged stand-in
  whose end nodes have different cluster paths; then `cu != cv` and at least one of them is a block: the link crosses,
  or ends on, at least one frame. A drawn border link is an **inter-unit edge with a block end**.
- **Rows** of a unit: 1 for a node, for a block the number of layers of its own level.
- **Band**: a maximal run of a level's layers covered by overlapping units.
- **Run**: the dummies of one chain that have the same cluster path on consecutive layers (the chain passing one level
  beside that level's blocks, or descending inside one block).
- **Title side** of a block: the side its title strip lies on (screen top: the upstream side in TD, the downstream side
  in BT, a lateral side in LR/RL). **Title row** (TD/BT): the members of the block, at any depth, from whose centre
  column the title side is reached along the flow without passing another member's box. It is read from the member
  boxes, so `scene()` and `route()` apply the same test.

### 2.2 Block mode

**Block mode is on iff the augmented graph has a border link.** It is a pure predicate of the graph: no ranking, no
sizes, no positions. `levels::block_mode` computes it from the output of `levels::augmented(g)`, the one constructor of
stand-ins and rank-only groups, and `scene_clustered`, `route_built` and `route_partial_with` all take both from there,
so the three entry points cannot disagree. `layout_core` receives the augmented graph and cannot build it again, so
block mode arrives with it (a field beside `frame_cut`); for the public `layout_clustered`, which has no stand-ins, the
predicate is "a real edge of `g` whose ends differ in `node_cluster`". A B9 part (a block's inside, or the rest with
its proxies) decides for itself when it goes through `scene_labelled`.

- **On**: every step of this section applies, to every candidate of `narrow`, and `route()` / `route_partial()` run
  2.9.
- **Off**: the whole call runs the pre-B12 path (every candidate of `narrow`, ordering, coordinates, layer lines,
  `scene_clustered`, the edge stages, `route()`, `route_partial()`), with two exceptions that hold for every clustered
  diagram: the frame function of 2.7 F3 in all three entry points (in `route_partial()` by the rule of 2.9 R6), and the
  degenerate own-box edges of 2.1 and 2.8.

During the work both sit behind the internal switch of 4.1: "on" above means "the switch is on and the predicate
holds", and the two every-clustered rules read "the switch is on and the diagram is clustered".

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
`subgraph.mmd` is the same picture (`B/revise/out/subgraph.{038,blk}.png`). 4.3 (e) gates each of them against the
reference, and the two with no static timing headroom (`subgraph.mmd`, `advanced.mmd`) against 4.6.

### 2.3 Step L: layering (in block mode `rank` keeps its hub graph and its back-edge marks; `block_rank` does the rest)

`rank` today, by the names of its stages (the code's comments number the last four 1-4): it builds the hub graph of the
rank-only groups, marks back-edges, layers by longest path, seats box sources (`seat_box_sources`), and places lead
cards one layer before their target. Block mode keeps the hub graph and the marks (L0) and replaces the other three
stages.

**L0. Node-level back-edges stay where they break a cycle of their own level.** `rank` marks back-edges with its two
DFS walks exactly as today (the *raw marks*). A raw mark is **kept** only when the link lies on a cycle of its own
level's unit graph: its two units are in one strongly connected component of that level's graph of L1, taken with
every link in its written direction, before any reversal. A kept mark is used reversed in everything below. Every
other raw mark is **dropped**: the node cycle it broke leaves its block, and that cycle is broken once, by L3, at the
level of the frames it crosses. Kept, such a mark would cost its cycle two edges against the flow, the inner one and
the one L3 turns: in `B/critic/in/cycle2-tie.mmd` the walk starts at `b2` and marks the inner edge `b1 -> b2` of Back
(`d-hybrid`'s prototype then draws `b1` and `b2` in one row and turns `a2 -> b1` as well). A kept mark reversed stays
inside its component; a dropped mark joins two components and lies on no cycle of its level.

**L1. Level graphs**, deepest level first, the root last: the level's units sorted by key, and its links (flagged
pairs of 2.1 left out) collapsed to unit pairs (`cu != cv`; parallel links between one pair are one link with their
summed weight), the kept marks reversed, the dropped ones in their written direction.

**L2. Note links.** A card is a member of the box the parser puts it in and takes a row there like any member (0.38:
`notes/near-drag`'s card `in301` is the last row of composite `K0`). So a link with a note-card end
(`style::is_note_card`) constrains rows in L5 like any link. It is never turned (L3 skips it; a card has one link, so
it lies on no cycle) and is not counted among the drawn links of L3 and L4. After L5, a lead card
(`Graph::lead_nodes`, a `note left of`) takes `start(target) - 1` inside its level when that is later: the lead-card
stage of `rank`, applied per level. There is no clamp.

**L3. Block order: which links run against the flow.** Only for a level whose unit graph (L1) has a cycle.

1. Walk A: DFS over the units; roots = units without an incoming link in key order, then every unit in key order; a
   unit's links are followed in key order of their targets, then link index. A link into a unit still on the stack is
   *turned*.
2. Walk B: the same DFS with the roots in plain key order.
3. The walk that turns fewer drawn links is taken; walk A on a tie.
4. Safety net, **per strongly connected component** of the level's unit graph (a link between two components is on no
   cycle). For a component `c` let `M_c` be the least number of its drawn links any order of its units turns. A
   component of at most `BLOCK_ORDER_EXACT_MAX` = 12 units is solved exactly (dynamic programme over subsets; ties to
   the lexicographically smallest key sequence); a larger one, and every component of a drawing past
   `WALK_MAX_ELEMENTS`, is not solved. A solved component takes its minimal order when the taken walk turns more than
   `2 * M_c + 1` of that component's drawn links; every other component keeps the walk. `M` of the level is the sum of
   the `M_c`, with the walk's own count for a component that was not solved.

Reason for a threshold instead of "always the minimum": on the analog the minimum (3) puts PROD above External services
and sends `DNS -> ingress` upwards; the walk turns 5 and gives the order the author wrote and mermaid draws. On dense-td
the single walk of `d-global` turns 21 of 47 edges; with the net 5. Reason for testing per component: the order of one
pair of blocks must not depend on an unrelated pair of the same level. With one test per level
(`d-hybrid`'s prototype) `B/check-fix-b/in/ratio2x4.mmd` (a pair with one link down and two up beside a pair with one
down and four up) flips both pairs, although the first alone keeps its declaration order, and
`B/check-fix-b/in/ratio4mask.mmd` keeps a pair with four links upstream that flips when it stands alone. Counting per
component also keeps the net on a level with many units: the root of `examples/state.mmd` has exactly 12 units, and
one more state would have dropped a per-level cap. On dense-td the walk's turned links all lie in one root component
(19 against a minimum of 2 in `d-hybrid`'s dump), so the per-component test switches there as the level-wide one did.

The rule reads the first-mention order of the nodes (the key). It is independent of the order of edge statements when
the nodes are declared before the edges, as in every block fixture. When nodes are first mentioned in edge lines the
key follows those lines: `B/critic/in/cycle2-tie.mmd` and `cycle2-tie-swapped.mmd` differ only in the order of two edge
lines written before the subgraph blocks and stack Front and Back in opposite orders (Back first in the first file,
Front first in the second), each with exactly one upstream edge and both blocks in their written inner order. A
fixture documents it.

**L4. Lane rule.** It is evaluated on every level whose unit graph, with every link in its written direction, has a
cycle (also when L0's kept marks left the graph of L1 acyclic and L3 did not run). Let `U` be the least number of the
level's drawn links (note links not counted), each taken in its written direction, that any order of its units turns.
`U` is computed like `M`, per component, on the unit graph before L0's reversal; a component too large to solve
contributes the number of its drawn links that run against the order L0 and L3 leave. The rule **fires** when both
hold:

1. `3 * U >` the level's drawn links (integers; `LANE_SHARE` = 1/3): every stacking order turns more than a third of
   them;
2. some path enters one child block of the level twice through drawn links of this level. The path is taken on the
   node graph of the augmented graph's drawn links (real edges and unflagged stand-ins, no rank-only pair) with every
   raw mark of L0 reversed, so that the graph is acyclic and every path is simple; a link *enters* a block when it is a
   drawn link of this level whose target lies in the block and whose source does not. It is one longest-path pass per
   child block, run only when the first test has passed.

Two swimlanes with `a1 -> b1 -> a2 -> b2 -> a3 -> b3` meet both (`U` = 2 of 5; the path enters Shop three times).
A there-and-back exchange meets the first test and not the second, and is stacked by L0 and L3 with its responses as
channels, as mermaid and both prototypes draw it: `B/check-fix-a/in/reqresp.mmd` (one request down, one response up:
`U` = 1 of 2), `reqresp22` (2 of 4), the `cycle2-tie` pair, `p06-block-cycle`. The first test alone cannot tell the
two apart (swimlane 2 of 5, reqresp 1 of 2), and without the second, 0.38's drawing of `reqresp` would stay: Front
464 px tall beside Back (`B/check-fix-a/out/reqresp.038.png`), the very defect B12 removes.

When the rule fires, the level's direct child blocks are **dissolved for layering**: L1 is rebuilt with their members
and their child blocks as units of this level, and L0's test, L3 and then L4 run again on the rebuilt graph. With only
plain nodes as new units the rebuilt graph is acyclic after L0 and nothing is turned: the swimlanes keep every arrow
forward and their frames side by side, five rows tall, as 0.38 draws them (`B/critic/out/swimlane.038.png`); mermaid
and `d-global`'s prototype stack the two blocks and point 2 of 5 arrows up (`B/critic/mm/swimlane.mermaid.png`,
`B/critic/out/swimlane.exp.png`). A cycle through a grandchild block is broken by L3 like any other, and the links it
turns are E3 (`B/check-revise-2/in/swimlane-nested.mmd`). If L4 fires again, the next block level dissolves; every
pass removes one block level, so it ends.

The blocks of a dissolved level share layers, as in 0.38. The rest of block mode still runs on it: O2 keeps the blocks
apart across the flow; between two of them that share a layer, forward edges keep the members' flow sides and cross
the frames' lateral walls in the layer gap, and an upstream edge between two of their members keeps today's lane (the
members are plain units of the dissolved level, so it is E4), as 0.38 draws both (2.8); X and G apply. The level
does not meet the block-shape rules, by design; its gate is "not worse than the reference" (4.3 e). Where a
component was too large to solve, `U` is an upper bound, so the rule errs towards 0.38's layering. Of the fixtures only `swimlane`, `swim-cycle` (`a1 -> b1 -> a2 -> b2 -> a1`: 2 of 4 in
written directions, where L0's reversal would hide the ping-pong) and `swimlane-nested` fire it; `reqresp`,
`reqresp22`, the `cycle2-tie` pair, `swim-nested`, `p06-block-cycle`, `cycle2` (1 of 4) and `ratio2`-`ratio4` must
not (script and table: `B/check-revise-2/l4u.py`, `out/l4u.txt`). B12.3 prints the levels that fire it. The constant is pinned by the
sweep of B12.6.

**L5. Rows of a level**: longest path (Kahn, queue seeded by key) with `start(cv) >= start(cu) + rows(cu)` for every
link not turned (note links included), and with the ends swapped for a turned one. Then two refinements, **repeated in
this order until neither changes anything**:

1. *A source block sits next to what it feeds*: a block with no constraint into it starts at
   `min(start of every unit it has a constraint to) - rows` when that is later, and keeps that start as a lower bound
   from then on. "A constraint to" means the units it links to **and the blocks rule 2 put after it**. Plain source
   nodes stay on the first row, as today.
2. *No interleaving*: while two sibling **blocks** `A`, `B` overlap in rows without one containing the other
   (`start(A) < start(B) < end(A) < end(B)`), add `start(B) >= end(A)` and solve again. Among such pairs take the one
   with the earliest `start(A)`, then the earliest `start(B)`, then key order. The constraint points from the earlier
   start to the later, so it closes no cycle.

Rule 1 then moves a block only into its own slack and moves no other unit; rule 2 adds at most one constraint per pair
of blocks. So at most (pairs of blocks + 1) passes change something and the loop runs at most (pairs of blocks + 2)
passes; that bound is a debug assertion. (If rule 1 looked only at the units a block links to, it would chase a
successor that rule 2 keeps pushing down: `B/check-fix-b/in/l5loop.mmd` never ends, nor do 1.6 % of random levels;
with the wording above 0 of 40,000 fail. Run once and in the other order the two rules leave the pattern they forbid:
`B/critic/in/interleave.mmd`. The pair rule gives `l5loop` 6 rows, S 0-1, T 2-5, B 2-4, where key order gives 9.) Then
the lead cards of L2.

`rows(level) = max(start + rows)`.

**L6. Absolute layers.** `layer(node) = sum of the starts of its enclosing blocks + its own start`. A rank-only hub gets
`max(layer of its unturned sources) + 1`. Then, for every drawn edge, **`back[e] = layer(to) < layer(from)`** (this
replaces the flags of L0; `d-hybrid` kept the old flags, so an edge marked back that ended up pointing downstream got no
chain and was drawn as one curve across several layers).

Properties, by construction, at every level L4 did not dissolve: a block's members fill exactly its interval; two units
joined by a link have disjoint intervals; no edge joins two nodes of one layer; sibling blocks never interleave.

**L7. Later re-rankings are fenced in block mode.** `tighten_sources` and `stagger_proposals` may not move a node out of
its block's interval nor change a block's rows (one interval test per proposal). `stack_candidates` and
`wrap_candidates` keep working: their groups are rank-only links, i.e. constraints of their level.

### 2.4 Step O: ordering (in `place`, block mode)

**O0. Chains.** Every drawn edge spanning more than one layer gets a dummy chain, **downstream and upstream**, except an
invisible link and an upstream edge whose two units are both plain nodes of one level (class E4 of 2.8: that one keeps
today's back-edge lane, package B3). A flagged pair (2.1) is no drawn link and gets neither chain nor label dummy. An
upstream chain is built from the lower layer to the higher and stored in from -> to order, as `place` already does for
a descending edge. An upstream edge with a block end that spans a single layer has no dummy and is no lane either (2.8
E3).

**Cluster paths of chain dummies: the deeper fit wins.** In block mode a chain dummy on layer `l` takes the longer of
`nc[from][..fit(from, l)]` and `nc[to][..fit(to, l)]`: the source's path while the layer lies in the source block's
interval, the target's inside the target block, the common prefix between (the two units of a link have disjoint
intervals, L6). Today's rule (`place`, the `apath` block) lets the target's prefix win whenever it is not empty. That
is right only at the root: below it the shared ancestor makes the target's prefix non-empty on every layer, so a chain
out of a nested block gets the ancestor's path on the block's remaining layers, lies beside the block and crosses its
lateral wall (`B/check-cold-read/apath-nested.mmd`: 1 lateral crossing in the experiment; the same graph without the
outer subgraph: 0). On a level L4 dissolved both fits can exceed the common prefix; there the target's wins, as today.
So a chain passes a foreign band at the level of the common ancestor, and enters and leaves each frame on its chain
through a flow-facing side.

**O1. Units of a level, per layer**: its direct nodes, its child blocks (on every layer of their interval), and its
runs. Blocks and runs are *long units*: they exist on several layers.

**The canonical order.** On every layer, the units and runs of each level form **one sequence by key**: a unit by its
key, a run by the key of its edge's source node; two runs of one source go by the key of the target node, then by
edge index. (A run and a unit of one level and layer cannot share a key, because a run whose source is a block's
first-declared member lies inside that block there; "a run after a unit of equal key" only makes the order total.) A
block's members and inner runs are contiguous and ordered the same way inside it. Adjacency lists are sorted by the
neighbour's canonical position before the first sweep, and every tie in O2-O4b falls back to the canonical position.
Nothing in this step reads the order of the edge statements (two parallel edges of one node pair excepted). This is
what `edge_line_order_barely_moves_the_dense_layouts` needs: the experiment started from the written order and dense-td
came out 3829 px wide in source order and 2986-2989 on four of the test's six shuffles (-22 %, limit 10 %), with
identical layers.

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

**O4. Starts.** O2 + O3 run from `BLOCK_ORDER_STARTS` starts. Start 0 is the canonical order. Start `k > 0` shuffles,
per level, the sequence of that level's long units (one permutation) and, per layer, that level's direct nodes; never
a layer as a whole, so O2's two constraints hold before the first sweep. One generator stream per start:
`x0 = k * 6364136223846793005 + 1442695040888963407` (mod 2^64), advanced once per draw by the same multiplier and
increment, Fisher-Yates with `j = (x >> 33) mod (i + 1)`; the stream is consumed in a fixed order: levels by key, in
each level first its sequence of long units, then its layers in ascending order. Integer arithmetic only:
deterministic on every platform. The start with the fewest layered crossings (`count_crossings`) wins, the lowest
index on a tie. `BLOCK_ORDER_STARTS` = 7 (ELK's default thoroughness), reduced by a work budget:
`starts = clamp(BLOCK_ORDER_WORK / (augmented nodes + links), 1, 7)` for the first placement of a `layout_core` call;
one start past `SEARCH_MAX_ELEMENTS`.

**Every later placement of the same `layout_core` call** (each candidate of `narrow`, and `centre_largest`'s)
**starts from the order the winning start ended with**, with no shuffle: on each layer the elements the placement
keeps there stay in the winner's final order; a node or dummy new to a layer is inserted before the first kept element
that follows it in the canonical order, at the end if there is none, and inside its block when it is a member; then
one run of O2 + O3, and O4b as after any start. The step-0 packed placement of `narrow` has the winner's layers and
keeps its order as it is. (A candidate that started from a seed would lose by construction: its layers differ from the
winner's, so the seed gives an unrelated order; one unrelated start is the 37-crossing regime of appendix A, and
`narrow` accepts a candidate only within `NARROW_CROSS_SLACK` of the multi-start best. Narrowing would go idle on the
inputs that need it.)

**The crossing musts of 4.3 must hold with the starts this budget grants, inside the time limits of 4.6** (B12.5).
The table crossings x starts of B12.5 is measured with this scheme. Appendix A's table used a flat per-layer shuffle
from the written order: it is evidence that starts matter, not a reference.

**O4b. Source order among equals.** A shuffled start leaves the members that change no crossing wherever the shuffle
put them (analog, seven starts: External services reads `DNS GW TG OAI AZ` or `GW DNS TG OAI AZ` depending on the base
seed, with 23 layered crossings either way; GW has no edge and is declared first). After the winning start, two
adjacent units of one level in one slot (two members of a block, two nodes or runs of a slot) swap when the left one is
later in the canonical order and the weighted layered crossing count does not rise; repeat to a fixed point. Each swap
removes one inversion against the canonical order, so it ends.

**O5.** Bar cards are placed as today. `centre_largest` does not reorder the blocks of a level O2 ordered.

**O6. Fans.** The chains leaving one node side are in the order of their first dummies, so a hub's fan (DNS: six edges)
does not cross itself. From B11 on, the final order of the ends of a side is the comb's (its turn order, kept only by
its own `keeps()` test, 3.3); `ports::assign` spaces them. The measure is the starting release's `Metrics::fan_x` (a
node or frame side with at least `FAN_MIN` ends; crossings between two of its edges, counted wherever they cross): 0
on the analog family and on the public fixture; `fan_uneven` is not above the starting release's (4.3 d).

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
frames, 21 turned edges under that walk) is the input that needs O2 on every level and L3's safety net; its must is at
most 90 (4.3 f). mermaid's numbers on the permutations come with a scrambled band order (perm-a puts the actors
mid-drawing).

### 2.5 Step X: coordinates across the flow (block mode)

**Order of the step in `place`.** Brandes-Köpf; `k0` per block (the displaced entries of 2.7 F2, counted from those
positions); X2 with `k0`; the F1 boxes; `k` from the final positions; where `k > k0`, X2 once more for that block with
`k`; X3; X4; then G1. The `k` that F2, G1 and the ports stage use is the one counted after the last X2 pass. X2.2's
reservation is a lower bound; F3 covers a column it did not reserve.

**X1. Brandes-Köpf: no alignment between two chain dummies of different cluster paths.** In `coordinates_bk` such a
pair is not offered as an alignment link (the prototyped rule: `d-global/prototype.patch`, the `coordinates_bk` hunk).
A chain is straight inside each block and beside each band and bends where it crosses a border, i.e. in a gap. A real
node may align with its neighbour across a border, real or dummy: in the experiment `R1 -> ARGO` is straight under this
rule, and PLAN invariant 4 asks for it.

**X2. Each block is fixed on its own, deepest first.** Items: its direct nodes, inner runs, two walls, child blocks
(already fixed, rigid).

1. `W_min`: longest path over the left-of relation of every layer with `pair_sep` as distances (the block's least
   breadth for this order).
2. Wall-to-wall breadth is at least the breadth 2.7 F1 gives the frame (in TD/BT the title chip plus
   `2 * TITLE_INSET`), plus `TRACK_PITCH` per displaced entry (2.7 F2) and `EDGE_GAP` per inner run that crosses the
   title side (item 5), so nothing foreign ever lies under a title that is wider than the content.
3. If the breadth Brandes-Köpf gave the block is within `BLOCK_SLACK` = 4 px of that: keep its coordinates.
4. Else squeeze, seeded by Brandes-Köpf: merge every *spine* (two items of the block on adjacent layers that are each
   other's only inner link in that direction) into one rigid column; clamp each unit's Brandes-Köpf offset into its
   feasible interval for breadth `W_min` (left-packed to right-packed position), left to right so the order holds; then
   `COMPACT_SWEEPS` sweeps moving each unit to the weighted median of its links **inside the block** (weights as
   `compact_breadth`: 1 / 2 / 8), within its neighbours.
5. An inner run takes a free column between two members when the order gives it one. A run that ends up beside the
   outermost member lies outside the member box; the frame then grows over it (2.7 F3), as 0.38's frames do. In TD/BT
   an inner run that crosses the title side (its member lies behind the title row) takes a column outside the chip's
   interval widened by `CHIP_CLEAR`: a free column between title-row members where one exists, else the column beside
   the outermost member past the chip's end. It reaches its member through the block's own layer gap and needs no jog
   room. (`B/check-fix-b/in/title-hidden.mmd`: the chip spans the block and the target is on the second row; 0.38 and
   the experiment both go round through the lateral wall, which 2.8 forbids.)

**X3. A fixed block is one rigid unit of its parent level**, which is treated the same way (keep Brandes-Köpf when
compact, else squeeze), up to the root. A block as a whole is pulled toward the median of its border links. A *border
spine* (an adjacent-layer border link whose two ends are each other's only link on that side) stays straight unless
2.8 displaces its entry or moves its port off the chip's interval: X1 lets Brandes-Köpf align it, and in the squeeze it
is merged into one rigid column at the parent's level, the block moving as a whole, when the block's feasible interval
allows.

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
prototyped**; B12.8 gates them with the edge-shape rows of 4.3 (`bends_max`, `detour_max`) and with the measure
`spine_off`. It counts an edge `u -> v` for which `v` is `u`'s only link on `u`'s downstream side and `u` is `v`'s only
link on `v`'s upstream side (links of the augmented graph) and no node centre lies strictly between the two centres
along the flow (adjacent layers, read off the drawing; the routed row uses the scene's centres), when the drawn edge is
not one straight segment or its ends differ in breadth by more than `STRAIGHT_TOL`. It does not count an edge whose
entry 2.8 displaces or whose port 2.8 moved off the chip's interval, nor a border spine that X3's feasible interval
forbids; those are listed in the report.

### 2.6 Step G: layer lines, gap room, label slots (block mode)

**G1. Room between frames.** Where a frame closes on layer `li - 1` or opens on layer `li`:
`lcoord[li] >= lcoord[li-1] + reach_of(li-1, 1) + need(li) + reach_of(li, 0)`, with `need` the largest of `track_need`,
the label rows of `gap_need`, `cross_need` and the labelled-gap room of `short_labels`. `reach_of` is measured to the
frame as 2.7 draws it: pad, title strip, and the growth of F2 (jog room; in LR/RL the title length). Today the room is
measured from the row above to the slot below and ignores the frame pad and the title strip (the layer-line section of
`place`), which is why the prototypes crush the band gaps (bench-75: lines 0.5 px apart). F2's growth depends on the
members' breadth positions, so `lextra` and `reach_of` take it after step X and before the layer lines.

`need(li)` is never below `FRAME_GAP`, and never below `FLOW_MIN_GAP + 1` px where a link (turned or not) joins a
frame that closes on `li - 1` to one that opens on `li`. `route()` reads two units as stacked only when their boxes
are `FLOW_MIN_GAP` = 12 px apart (2.9 R2), while the layout's own floor between two frames is `FRAME_GAP` = 8 px and
`track_need` is 0 for a straight link: two linked blocks with nested frames on their facing rows are 8 px apart in
0.38 and in the experiment (`B/check-fix-a/in/t9-frames-8px.mmd` and its LR copy). The extra pixel keeps R2's
comparison off an equality.

**G2. Tracks.** `track_need` already counts every sideways step in a gap, chain bends included; the bends X1 puts at
block borders are such steps. One `TRACK_PITCH` per overlapping step. Steps on one side going the same way nest (the
rule `ortho` has since B10). Where `track_need` is not computed today (a graph with `edge_paint_width`, or more drawn
edges than `ROUTE_MAX_EDGES`) the room for these bends is absent, as today (2.10 "large drawings").

**G3. Label slot.** A chain's labelled dummy is the middle dummy of the chain's run **at the edge's common-ancestor
level** (outside both end blocks), chosen after the cluster paths of O0 are known. When that run is empty the label
takes the border gap by the `short_labels` rule, measured from the frames. An adjacent-band labelled edge keeps
`short_labels`. A label never sits inside an end block or on a frame border (gate `lbl_in_end`), and its chip sits on
its own line (gate `lbl_own_gap_max`: the experiment has a chip 16.5 px beside its line, next to two parallel lines).
A slot is clear of foreign lines: the labelled dummy reserves its chip plus `CLEAR_GAP` on each breadth side (in
`pair_sep`), and a label row in a gap is a row of its own in `gap_need`, with no foreign track within `CLEAR_GAP` of
the chip along the flow. What the reservation has to achieve is the gate `lbl_crowd` of 4.3 (no prototype has it: 8-9
crowded chips on the analog).

**G4.** None of this runs outside block mode. Wider gaps are paid for by the canvas and by the labelled edges' length:
the area caps and the `lbl_endpx_mean` rows of 4.3 apply.

### 2.7 Step F: frames

A frame is a function of **its own members' boxes, its title, and the graph**, in every entry point. Besides these it
reads two things and nothing else: on which side of the block the far unit of a border link lies (F2's count and F3's
inner legs read it; 2.9 R6 covers the drag), and, in F3's last step, the neighbouring frames (`stop_at_siblings` and
`renest_boxes`, as today, in every entry point).

**F1. Box.** `cluster_raw_boxes` of the member positions (pad, title strip on the screen top). *Title breadth (block
mode, TD/BT)*: the box is at least as broad as its title chip plus `2 * TITLE_INSET`; a narrower one grows equally on
both lateral sides. The static layout reserves that breadth (X2.2). 0.38 and the experiment cut a title that fits no
frame where the canvas ends ("A long block title that covers…" in `B/check-fix-b/out/title-hidden.{038,exp}.png`), so
room reserved beside a block would be wasted for the outermost block; mermaid never cuts a title. The F1 box is the box
R2 classifies with, and the box from which the title row, the chip's breadth interval and `cap(m)` are read: never the
grown frame, so no count depends on a growth it decides.

**F2. Growth along the flow (block mode).**

- *Jog room (TD/BT only; in LR/RL the strip lies on a lateral side and `k` = 0).* For a member `m` of the title row let
  `border(m)` be the number of drawn real edges with one end at `m` and the other outside the block that cross the
  block's title side: in TD the downstream edges (E1, E2) into `m` and the upstream edges (E3) out of `m`, in BT the
  other way round (2.8 "Title-clear entries"). An invisible link is not counted, nor is a stand-in, whose end on the
  block is a free pin. The class is the entry point's own: L6's layers of the two units in `scene()`, R2 in `route()`.
  `cap(m)` is the number of ports on the free stretch of `m`'s title-facing side: the side's range less the chip's
  breadth interval widened by `ortho::CHIP_CLEAR`, the longer part; `floor(stretch / PORT_SPACING) + 1`, or 0 when
  nothing is free. One `pub(crate)` function gives that stretch. `ports::unshaded` calls it for such a side, and in
  block mode it applies by breadth alone, without `unshaded`'s depth test: the edge crosses the title side whatever
  the jog room's depth, and with the depth test a jog room of two tracks would move the chip out of `CHIP_SHADE`, the
  whole side would count as free, `k` would fall to 0 and the room would go. A block has
  `k = sum over the title row of max(0, border(m) - cap(m))` *displaced entries*. With `k > 0` its frame grows on the
  title side by `ROW_CLEAR + (k - 1) * TRACK_PITCH`: the room between the title strip and the title row in which 2.8's
  entries jog.
  Why only the links that cross the title side are counted: counted over every border link, the analog's External
  services reserves 50 px for the six links its hub sends out downstream (border 9, cap 5), although its three
  incoming links fit beside the chip and nothing jogs; the public fixture's edge block reserves 25 px where 12.5 are
  needed (`B/check-fix-a/jog.py`).
- *Title length (LR/RL).* The title runs along the flow there. A frame is at least as long along the flow as its title
  chip plus `2 * TITLE_INSET`; a shorter one grows equally on both flow sides. (In the experiment a one-column block
  with a long title is cut to an ellipsis: `B/critic/out/lr.exp.png`; 0.38 shows it whole.)

Both are part of the shared frame function, and the static layout reserves the same amounts (G1). **The growth is
unconditional in every entry point**: a function of the block's own members, its title, the graph and the classes of
its border links. At the scene's centres the layout has reserved it. A foreign box that a host drags into the jog room
overlaps the frame like any node dragged onto a foreign frame (R4 prices it), and the displaced entries keep their
jog. In LR/RL the frame is as long as its title in every entry point; `fit_cluster_title` cuts a title only where it
would run into another frame, as today.

**F3. Growth across the flow is one function, for every clustered diagram.** Today only the static scene widens a
frame over the channels laid inside it, so frames snap on the first pixel of a drag (71 px on the analog, 617 on
bench-75). The static widening in `scene_clustered` and `route_lane_clusters` become one function of the frames and the
drawn paths. A frame grows across the flow to cover, plus `LANE_PAD`:

- the drawn paths of the edges with both ends in it, the lanes of its own back-edges and their labels (as today);
- the inner legs of every edge with exactly one end in it: in `scene()` the chain waypoints the layout placed in that
  box (what 0.38's widening covers: "a channel lane can sit just outside the outermost member node"), in `route()` the
  inner legs of 2.9 R3, in `route_partial()` the paths that call draws (2.9 R6);
- in block mode, the border-crossing columns of its displaced entries and of the inner runs that cross its title side
  (X2.5).

Then `stop_at_siblings`, `renest_boxes`. Outside block mode the function returns exactly today's static frames in the
static scene. The off-identity test of B12.0 proves it, on inputs that reach the rewritten code: 0.38 widens the frame
of `B/check-fix-a/in/iso-lane.mmd` 29.6 px past its members, while every other not-block-mode input of the corpus is
widened by 0. An edge between two members stays inside their frame in `scene()`, `route()` and on a drag (existing
tests, unchanged).

### 2.8 Step E: how each kind of edge is drawn

| kind | static scene | `route()` |
|---|---|---|
| E1 downstream, ends in different units, more than one layer apart | dummy chain (O0): out through the downstream side of every frame round the source, beside the blocks of the bands between, in through the upstream side of every frame round the target, down inside the target block beside its members | corridor (2.9 R3) |
| E2 downstream, adjacent layers | no dummy; `ortho` draws the Z in the band gap, sized by G1 | today's channel when nothing lies between, else corridor |
| E3 **upstream, at least one end unit a block**, more than one layer apart | dummy chain the other way: out of the source's **upstream** side, in through the target's **downstream** side | corridor, mirrored |
| E3, adjacent layers (the analog's `ARGO -> TG`) | no dummy; drawn as E2 mirrored: out of the upstream side, the Z in the band gap, into the downstream side; **never a lane** | the same |
| E4 upstream, both units plain nodes of one level (on a level L4 dissolved, the members of its dissolved blocks are such units) | today's lane (`back_lanes`, `static_lanes`) | today's `route_backs` |
| node -> whole subgraph, subgraph -> subgraph | the stand-in's chain; the end on a frame is a *free pin* on its flow-facing side | the same against the current frames |
| member -> its own subgraph | an edge to the block's entry member (`own_box_targets`), drawn whole; a border link when the entry member lies in a nested block | unchanged |
| invisible link | ranks (L), orders, never a chain, never drawn | unchanged |
| note link | never turned (L2), never a lane, as today | unchanged |

Every "is this a lane?" test in the code asks "is it E4?", not "does it have a chain?".

Where `ortho::orthogonalize` does not run (more than `ROUTE_MAX_EDGES` scene edges, or a graph with
`edge_paint_width`, i.e. a `linkStyle default` stroke width), E2, the adjacent-layer E3 and a free pin's last step
keep the builder's curve (today's S), still through flow-facing sides.

**Own-box edges, in every clustered diagram** (block mode or not; 0.38 draws all three through members or as
nothing). The first and the third are the degenerate edges of 2.1: no link, no chain, no say in block mode.

| kind | rule, in `scene()` and `route()` |
|---|---|
| subgraph -> itself | no link (2.1); a loop on the block's frame, drawn like a node's self-loop with the frame as the box (0.38: a zero-length path, `M 84.0 400.0 C 84.0 400.0, ...` in `B/critic/out/self-member.038.svg`) |
| subgraph -> its own member | the stub `own_box_stub` keeps today's side when its way to the member passes no other member (`a_composite_edge_to_its_own_state_drops_straight_from_the_border` is that case); else it takes the first frame side from which the member is reached without passing one, tried in the order upstream, the lateral side nearer the member, downstream, the farther lateral side (0.38 runs `S --> b` from the bottom border through member `c`); when no side qualifies the stub keeps today's side (`B/check-fix-b/in/own-centre.mmd`: the member has a member on all four sides) |
| subgraph <-> its own nested subgraph | no link (2.1); a stub between the two borders on a side where no member lies between them (0.38: two straight strokes through member `store`, `B/critic/out/sub-nested-self.038.png`); two anti-parallel stubs between one pair of borders are spaced by `sub_parallel_offsets` |

**Flow-facing ends (E1-E3).** The end on a member of a block the edge leaves or enters is on the member's side that
faces the border it crosses; the end on a top-level node of such an edge is on its flow-facing side too. The side is
set where the builder sets it (`scene_from_layout`'s anchors for chains; the corridor builder in `route_built`), and
`ortho` and the router keep it. `ports::assign` spaces the ends of a side and, on a side over-full at
`PORT_MIN_SPACING`, spills the outermost round the corner (`ports::spill`). In block mode the spill order is: forward
ends of edges inside one block first; an E4 lane keeps today's rule (never round the corner; a back end stays while
the side holds `PORT_HARD_SPACING`); the end of an E1-E3 edge on a block member last, and only when the border ends
alone exceed the side's capacity. Such surplus ends count in `lateral_x`, `side_ends` and `up_lanes` and are named in
the report (4.3 d). A lateral side is otherwise allowed only when the two units overlap along the flow (a host can drag
them side by side; a level L4 dissolved has them so by layout).

**Free pins.** The chain of an edge to a whole block aims at a proxy member and would be cut wherever it happens to
cross the border (experiment: 14, 14 and 32 px from a frame corner). Instead the chain is cut at its last dummy outside
the block and the builder sets the end at the pin. The entries of a frame side are ordered by O3 in `scene()` and by
the breadth of each entry's previous leg in `route()` (ties: edge index). Free pins that are consecutive in that order
form one even band: equal pitch between `PORT_SPACING` and `PORT_EVEN_MAX`, centred in the free interval between the
neighbouring member-entry columns, the corners and the title chip, at least `PORT_SPACING` from each. A side whose
pins are split by member entries has several bands. A lone pin is the band of one: the middle of its free interval.
This is the shape B11's `comb::plan_even` leaves alone when the side is one band; a split side is named in the report.
`ortho` draws the last gap step. `route()` places the pins by the same rule against the current frame.

**Title-clear entries (TD/BT).** The title side of a block is crossed in TD by the entries of downstream edges and the
exits of upstream ones, in BT the other way round; in LR/RL the strip is on a lateral side, the rule is void, and pins
stay out of its breadth interval. An edge that would cross the title side within the chip's breadth plus `CHIP_CLEAR`:

1. keeps clear by its port when it can: the first `cap(m)` border entries of the member, in port order, take the free
   stretch of that side (F2);
2. else is *displaced*: it crosses the border at the nearest free column beyond the chip, in the order of the entries,
   the columns `TRACK_PITCH` apart, and jogs to its port on a track in the jog room of F2. The jog lies **inside the
   frame**, between the title strip and the title row. X2.2 reserves the breadth for the crossing columns, and when the
   member box is narrower the frame grows across the flow to the last displaced entry's column plus `LANE_PAD` (F3);
3. an inner run to a member behind the title row crosses the title side at the column X2.5 gives it, outside the
   chip's interval, and needs no jog.

A displaced entry is a comb tooth to B11 (stub, track, turn). The comb's own `keeps()` test rejects a plan that adds a
hit on a title chip, so it should leave the jog where it is; B12.11's test asserts that the jog lies inside the frame
after the comb has run, and only if it fails is the tooth's far limit clamped to the frame's inner title-side edge in
`comb.rs`.

This replaces `d-global` 1.6, which bounded a chain dummy that does not exist when the target is on the block's first
row. Today, and in every prototype, such an edge goes round to the member's side
(`B/spec/evidence/title-entry.fm038.png`); on the analog TR1, TR2 and PSDEV sit under their chips.

**Labels**: G3; in `route()`, R8.

### 2.9 `route()`, `route_sized()`, `route_partial()`

The host gives positions, so no layering, ordering or coordinates run. What runs, in block mode (2.2):

- **R1. Frames, in three steps.** (i) Before any edge: the F1 boxes; R2 classifies with them. (ii) F2's growth, which
  needs the classes. The F1 + F2 boxes are the obstacle boxes of R3 and R4. (iii) After the builders: F3's growth over
  the inner legs, then `stop_at_siblings` and `renest_boxes`. (Today's `route_built` already builds every edge against
  the member boxes and only then grows the frames.)
- **R2. Classes by geometry and the level tree.** For an edge whose units `cu != cv` at its common-ancestor level are
  not both plain nodes: let `S`, `T` be the F1 boxes of `cu`, `cv` (members, pad, title strip, before the growths of
  F2 and F3; a plain node's own box), so that a class never depends on a growth it decides. `T` wholly downstream of
  `S` by `FLOW_MIN_GAP` -> E1/E2; wholly upstream -> E3; overlapping along the flow -> today's builders with lateral
  sides. Both plain nodes -> today's rules (`route_backs` for E4). At the scene's own centres this reproduces the
  static classes: L6 defines `back` by the layers, and G1 keeps linked frames more than `FLOW_MIN_GAP` apart.
- **R3. Corridors, per level, not only at the root.** Obstacles of an edge = the other units of its common-ancestor
  level whose flow interval meets the open interval between `S` and `T`, and, inside each end block, that block's other
  units. If today's 4-point channel (`cross_cluster_route`) meets none, keep it. Else the path is a polyline with **one
  breadth per band it passes and one per end block it descends in**, bending in the gaps like the static chain:
  - *bands*: the obstacles grouped into maximal runs that overlap along the flow, from the current boxes;
  - *inner legs first*: inside an end block the path is planned at that block's level, from the border crossing to the
    member (or to the next frame inwards), obstacles = that block's other units, by the same per-band rule; a free
    column inside the block is preferred to one beside its outermost member (F3 then has nothing to grow); a leg that
    crosses the title side takes a column outside the chip's interval (2.8). The columns the inner legs take outside
    the member box are reserved in the gap beside that block before any band corridor is dealt: slot 0 of a passing
    corridor starts beyond them, so the strip F3 adds to the block afterwards never meets a corridor;
  - *per band*: the corridor is one of the band's gaps across the flow (between two neighbouring obstacles, or outside
    the outermost) wide enough for its slot; which one is R5's rule; breadth = the gap's nearer edge + `LANE_PAD` +
    slot x `TRACK_PITCH`;
  - *slots*: among the edges that share a gap and leave it to the same side, the one that leaves first along the flow
    lies nearest that side; edges leaving at the same flow position are ordered by the breadth of the far end of their
    next leg (for the last band: the target's column); edges leaving to opposite sides lie on their own side of the
    corridor; edge index only breaks ties. (Slots by edge index alone would follow the order of the edge statements,
    which 2.4 removes from `scene()`: a hub's five edges down one corridor would cross each other in one of two
    permutations of the same graph, and nothing later re-nests them.)
  - *ends*: out through the flow-facing side at the port's column (title rule of 2.8), across in the first free gap,
    along each corridor, across in the last free gap, in through the flow-facing side.

  `lanes::plan` is **not** used for placement: it orders by band length and a geometric "far" measure and re-deals
  breadths among the lanes of a side, and a `Lane` has one breadth (lanes.rs:616-642, 689-721, 793-850). Its stub and
  nesting geometry may be reused. Emit bare on-axes corners so `ortho_pass` keeps the path and only rounds it; no
  change to `ortho`'s static-only columns. Where `orthogonalize` does not run (2.8), the corridor builder passes its
  corners through `router::smooth` itself: an orthogonal path is always delivered corner-rounded and densely sampled
  (PLAN invariant 2).
- **R4. Hard obstacles.** For these edges a block that is on neither end's chain is a hard obstacle in
  `router::repair` (today a foreign frame is only priced), unless its frame overlaps either end's unit box: a host may
  put a node on a foreign frame, and that frame is then priced as today.
- **R5. Stability under a 1 px move.** The corridor of a band is the wide-enough gap nearest the breadth of the
  previous leg (the source port's column for the first band), the lower breadth on a tie; slots by R3's rule.
  Within one choice of gaps every coordinate of a corridor is a continuous function of the frames. The choice itself
  changes only where two gaps are equally near, and on a drag only for the edges of the moved node: a kept edge keeps
  its path (R6).
- **R6. `route_partial`** keeps its logic; `clusters_line_up` holds (F1 and F2 are functions of a block's own members
  and of the classes of its border links).
  - *Frames.* A frame is redrawn when it holds a moved node, or when it lies on an end's chain of blocks (below the
    link's common-ancestor level) of a border link whose other end is a moved node or lies in a unit that holds one:
    that unit's box moved, so the link's class, F2's count and the inner leg may have. A redrawn frame takes F1 and F2
    from its members and F3 from the paths this call draws: the base paths of its kept edges (their part inside the
    frame's base box) and the builder paths of its redrawn edges; never `route()`'s fresh paths of kept edges, which
    are not drawn. Every other frame keeps its base box (an unchanged box costs nothing, as today), also where a moved
    node or a regrown frame has left a side that `stop_at_siblings` had cut short: `route()` at the same centres lets
    that side grow, `route_partial()` does not, as today.
  - *Edges.* An edge on a moved node is redrawn by the same builders as in `route()`; so is a sub-edge that ends on a
    regrown frame (today's `moved_end`; not every edge with an end inside it), and an edge that shares a corridor gap
    whose slot order changed. A kept edge keeps its base path, also a corridor that passes the moved block unless the
    block now touches it. `PARTIAL_ROUTE_SHARE` bounds the redraw as today; no new state in the base scene.
  - A far end dragged across a block changes that block's title edge by one track when the link's class flips:
    `TRACK_PITCH`, less than `DRAG_JUMP`.

  The frame rule holds for every clustered diagram (2.2 Off), block mode or not.
- **R7. The canvas** of a routed block drawing ends `MARGIN` past every drawn edge
  (`a_static_canvas_is_its_drawing_with_even_margins` fails on `d-global`'s prototype for dense-td; it must pass).
- **R8. Labels.** A corridor edge's label sits on its leg at the common-ancestor level, outside both end blocks, at the
  place today's label placement in `route()` picks on that leg. At the scene's centres that leg runs through the room
  G3 reserved, which is what the routed label rows of 4.3 (h) rest on.

`scene()` and `route()` at the scene's centres are not equal today and are not required to be equal. They must be
closer than in 0.38: frames equal (F), edge classes equal (R2), the gates `fr_max` and `ends_apart` of 4.3 (d), and
the routed rows of 4.3 (h), because the routed drawing at the scene's centres is what the playground paints before the
first drag.

### 2.10 Cases

| case | rule |
|---|---|
| nested subgraphs | L1-L6 recurse; O2 runs per level; X2 fixes deepest first; an edge to a member two levels down belongs to the common ancestor's level, its chain takes the deeper fit level by level (O0) and crosses each frame on a flow-facing side; R3 per level |
| node -> whole subgraph | the stand-in and the proxy group collapse to a link (node's unit, block): the whole block lies after the node; free pin (2.8) |
| subgraph -> subgraph | one link between the two units; both ends free pins; when turned it is E3 |
| member -> own subgraph / subgraph -> own member / subgraph -> itself / subgraph <-> own nested subgraph | 2.8; the last two are no links (2.1) |
| cycle between blocks | L3; turned links are E3; every solved component has at most `2 * M_c + 1` |
| two-block cycle `A <-> B` | when rank's breakers are links of that level (`cycle2`: Front stays first, as in mermaid) they decide and L3 never runs; when rank marked an inner link of one block (the `cycle2-tie` pair) the mark is dropped and L3 decides by key; a there-and-back exchange, 1:1 or 2:2 (`reqresp`, `reqresp22`): stacked, lower key first; in general the block with the lower key is first unless that turns more than `2 * M_c + 1` drawn links: with one link down, 2 or 3 links up keep declaration order, 4 flip it (`B/critic/in/ratio{2,3,4}.mmd`) |
| more than a third of a level's links against any order **and** a path that enters one block twice | L4: that level's child blocks are dissolved for layering and stay side by side, forward edges across the lateral walls, upstream edges as lanes; gate "not worse than the reference" (4.3 e). More than a third without such a path (a there-and-back exchange): stacked |
| hub linked to many blocks | no layering rule; its block sits where L5 puts it; its chains share the corridor O2 chose and leave in fan order (O6); B11's comb runs on the fan like on any other (3.3) |
| empty subgraph; subgraph of empty subgraphs | no unit, no frame, sub-edges skipped, as today |
| subgraph with one member | a block of one row |
| members without inner links | first row of their block; `grid_pairs` still packs loose members into rows by rank-only pairs, which are inner links |
| unlinked blocks | all start on the level's first row, side by side in key order; `narrow` may stack or wrap them as today |
| no border link at all | not block mode (2.2): the 0.38 path |
| own `direction`, inner edge, no border edge (B9) | unchanged: `compound::split` cuts it out; the proxy is a plain unit; each part decides block mode for itself, so a B9 block inside a bordered block sits in that block's rows as one unit |
| own `direction` with a border edge | the direction is not honoured (mermaid's rule, README), an ordinary block |
| LR / RL / BT | layers run along the flow in every direction; title strip handled where it is today (`lextra`, `reach`, `header_b` in `pair_sep`); title-clear entries and jog room in TD/BT only, the title length in LR/RL (2.7, 2.8); tests run all four directions |
| state composites | a composite is a block; its `[*]` states and note cards are members; a transition to a composite is a sub-edge; a composite's self-transition is a loop on its frame (2.8) |
| note cards, lead notes | L2: a card takes a row of its block; a card beside a fork/join bar keeps `bar_cards` ordering |
| fork / join bars | `fit_bars` runs before the layout, unchanged; a bar whose branches lie in different blocks is a plain unit of their common level and its branches are chains |
| invisible links | links of weight 1 for L3 (the author asked for that order), border links for 2.2, never chains, never counted in F2 |
| node self-loops | unchanged (`loop_reach` room in the separations) |
| nodes first mentioned in edge lines | the key follows those lines (L3); documented by the `cycle2-tie` pair |
| over-full node side | 2.8: border ends spill last; the surplus is named in the report |
| large drawings; `linkStyle default` stroke width | L is linear per level plus the subset programme (<= 4096 x 144 steps per component of <= 12 units); O4 is budgeted; past `SEARCH_MAX_ELEMENTS` one start and no narrowing as today. Past `ROUTE_MAX_EDGES` (400 scene edges), and in any graph with `edge_paint_width`, `ortho` and the router's track room are skipped as today: blocks, chains and corridors still hold, because they come from the layout and the builders; the builders round their own corners (R3), E2 and E3 keep the builder's curve (2.8), and G2's room for the bends at block borders is absent |
| class, ER, `advance`, flat flowcharts | never clustered: untouched |

### 2.11 Tie-breaks and determinism

| decision | rule |
|---|---|
| unit order | key; never equal |
| DFS roots, DFS links | L3 |
| fewer turned | walk A on a tie; subset programme: lexicographically smallest key sequence |
| longest path | Kahn, queue seeded by key, FIFO |
| source blocks, no-interleave pair | L5's order; the pair with the earliest `start(A)`, then the earliest `start(B)`, then key order |
| start order of a layer | canonical (2.4): one sequence of units and runs by key, a run keyed by its source node, then target node, then edge index |
| level order (O2) | fewer weighted crossings; earlier half-round; start order before "reversed"; ties inside a sweep by canonical position |
| starts (O4) | fewer layered crossings; lower start index; one generator stream per start, consumed in a fixed order |
| later placements of a call (O4) | the winner's final order; a new element before the first kept one that follows it canonically |
| crossing-neutral neighbours (O4b) | canonical order |
| squeeze (X2) | fixed sweep count and directions; `weighted_median` takes the lower point on a tie |
| free pins | even bands in the order of the entries (O3 in `scene()`, previous leg's breadth then edge index in `route()`) |
| corridor of a band (R5) | nearest wide-enough gap to the previous leg; lower breadth on a tie |
| corridor slots (R3) | leaving first lies nearest its side; then the far end's breadth; then edge index |
| containers | `Vec` and `BTreeMap` wherever iteration order can reach the output; no float is used as a map key |

Byte-identical SVG across runs is asserted for every block-mode fixture (the existing determinism checks get
`tests/fixtures/blocks` added to their directory lists).

### 2.12 New constants (name them, explain them in a comment, as the code base does)

| constant | value | pinned by |
|---|---|---|
| `BLOCK_ORDER_EXACT_MAX` | 12 units per strongly connected component | cost bound |
| turned-link threshold | `2 * M_c + 1`, per component | the public fixture (walk kept), dense-td (the net switches), `ratio{2,3,4}`, `ratio2x4`; sweep B12.6; the analog (5 against 3 keeps the walk) is a local cross-check |
| `LANE_SHARE` | 1/3, with the path condition of L4 | `swimlane`, `swim-cycle`, `reqresp`; sweep B12.6 |
| `BLOCK_ORDER_STARTS`, `BLOCK_ORDER_WORK` | 7, measured | experiment (7 = 16 on all 14 inputs; fewer than 5 misses the local inputs); `perf.sh`; the table of B12.5; sweep B12.6 |
| `BLOCK_SLACK` | 4 px | B12.8, pinned at checkpoint A2 |
| `ortho::CHIP_CLEAR` | existing (6.5 px), made `pub(crate)` | `router.rs` has a private constant of the same name and another value (6.0): it keeps its value and is renamed, so the router's chip clearance does not move in any diagram |
| `CLEAR_GAP`, `PORT_SPACING`, `PORT_EVEN_MAX`, `PORT_MIN_SPACING`, `ROW_CLEAR`, `TRACK_PITCH`, `LANE_PAD`, `FRAME_GAP`, `FLOW_MIN_GAP`, `TITLE_INSET`, `EDGE_GAP` | existing constants | |

## 3. Code plan

Prototype code to start from (hacks with env switches; none is mergeable as is):

| piece | where | state |
|---|---|---|
| L0-L3, L5, source blocks, hub layers | `B/d-hybrid/prototype.patch`, `fn block_rank` (tree copy `B/d-hybrid/tree`) | works; treats note links as ordinary links (keep that: L2); keeps every raw mark (L0 drops some), one net per level (L3: per component), one pass of L5 with key-order pairs; lacks L4 and the L6 back-flag fix |
| a shorter L1/L5/L6 | `B/d-global/prototype.patch`, `fn block_layers` | works; single walk, no safety net |
| X1, X2 without seeding | `B/d-global/prototype.patch`, the `coordinates_bk` hunk and `fn block_compact` | works; X2 to be changed per X2.3, X2.4 |
| upstream chains (O0), `back_lanes` filter | `B/d-global/prototype.patch` (mode 2), `d-hybrid` S7 | works; cluster paths by today's rule (O0 changes it) |
| O2 at the root, O3, O4 on `d-global`'s layering | `B/spec/experiment.patch` (diff against `B/d-global/tree`; tree copy `B/spec/tree`; switches `FM_B12=2 FM_B12_LVL=1 FM_B12_STARTS=7`; prebuilt CLI `B/critic/fm-spec`) | works; **far too slow** (3.5), root level only, written start order, flat per-layer shuffle, `TIE` weight instead of constraints |
| per-input structure dump, block-metrics, `judge` | `B/metrics/` | ready for 0.38's SVGs; B12.0 extends them (B11's hop bridges, new columns, reference tables, `judge`'s rule kinds) |
| invariants probe (frames and edge ends, scene against route; drag jumps with sub-edges; `dfr_own`, `dfr_far`) | `B/j-invariants/b12inv.rs` | ready; turn into tests |
| lateral frame crossings from a flowmaid SVG | `B/critic/lateral.py` (TD), `B/revise/lat2.py file.svg:h` (LR) | ready; the cross-check for the block-metrics column `lateral_x` (it reads flowmaid's dialect only and prints 0 for any mermaid SVG) |
| the rules of L3-L5 by their letter, over every structure file | `B/check-revise-2/l4u.py` (L4), `B/check-fix-a/l4.py`, `B/check-verify-fix-b/py/l5v.py`, `l5w.py` (L5, with a fuzz) | ready; reference for the unit tests |
| F2's count on a drawing | `B/check-fix-a/jog.py` | ready |
| the switch (cfg, thread-local, scoped setter) | `B/check-verify-cold-read/cfgprobe` | builds clean under clippy, MSRV 1.75 and wasm32 |

### 3.1 By file

**`src/levels.rs` (new, small, pure).** `BLOCKS_DEFAULT` and the override of the switch (4.1). `augmented(g)`: the
stand-in edges and rank-only groups, moved out of `scene_clustered` so that `route_built` and `route_partial_with`
build the same ones, with the flag of 2.1 on the degenerate pairs. `block_mode` (2.2). The level tree from the cluster
paths (`node_cluster`, as `layout` gets them, so the public `layout_clustered` works too): units and keys per level,
each link's common-ancestor level and unit pair, each node's chain of blocks, and per member its border edges (F2
counts those of the title-crossing class; the title row itself is geometric and lives with the frame function). Used by
`layout::block_rank`, by the ordering, by `scene::route_built` (R2, R3), by the frame function and by the tree-based
quality measures.

**`src/layout.rs`**

| function (0.38 line) | change |
|---|---|
| `struct Ranking` (2267), `rank` (2281) | its stages stay as they are for the non-block path. In block mode `rank` builds the hub graph and marks back-edges (L0), then new `block_rank` (L1-L6) returns the layers and back flags. Without `node_cluster` not one line changes |
| `layout_core` (1545), `struct Opts` (1607) | block mode arrives with the augmented graph (a field beside `frame_cut`; for the public `layout_clustered`: a real edge whose ends differ in `node_cluster`); carried in `Opts` to every `place` and to `narrow`. The switch is read here only when the call comes through the public layout functions (4.1) |
| `narrow` (1656), `tighten_sources` (2840), `stagger_proposals` (2877) | L7 fences; every candidate starts from the winner's order (O4). `stack_candidates` (2157) / `wrap_candidates` (2042) unchanged; re-read `RIBBON_ENTRY` and the acceptance slacks of `narrow` on block layouts (B12.12) |
| `place` (3026), chain loop (the "invisible links and back-edges get no channel" branch) | O0: in block mode only E4 goes without a chain or a Z; a flagged pair gets nothing |
| `place`, the `apath` block (3302-3341) | block mode: the deeper fit wins (O0) |
| `place`, label dummy sizing (3132-3147) | moves after the cluster paths are built; G3, with the `CLEAR_GAP` reservation |
| `place`, ordering (3343-3397), `cluster_keys` (5487), `arrange` (5534), `transpose` (4750), `count_crossings` (4805) | block mode: the canonical order, new `level_order` (O2, long units), the multi-start loop and the warm start of later placements (O3, O4), O4b; `arrange` takes slot / ordinal keys per layer. Flat and non-block clustered diagrams keep today's loop byte for byte (do not "improve" the half-round judging there) |
| `place`, `centre_largest` hint (3431-3439; `centre_largest` 1936) | O5; its placement starts from the winner's order (O4) |
| `coordinates_bk` (4880) | X1, behind block mode |
| after `coordinates_bk` (where `opts.compact` calls `compact_breadth`, 5201) | block mode: new `block_compact` (X2, X3) in the order 2.5 gives. `compact_breadth` stays for narrowing outside block mode |
| `place`, layer-line section (4062-4175), `reach_of` (3608), `track_need` (4002), `gap_need` (3878), `short_labels` (3630), `lextra` (3192) | G1-G3 with the two floors of G1; F2's growth in `lextra` / `reach_of`, taken after step X; X4's shifts along the flow |
| `grid_pairs` callers | B12.3: a long row of unlinked members inside a block is wrapped |
| `back_lanes` (4427) | block mode: only an E4 edge is a lane; every E3 edge, with or without a chain, counts among the forward skeletons. Guard it with block mode (`d-global`'s filter is unguarded) |
| `drawn_crossings` (4629), `drawn_breadth` | E3 edges are drawn like chains: no bow. PLAN invariant 6: the back-edge bow rule lives in `scene.rs` and here; change both or neither |
| `layout_clustered` (1514, public) | doc comment: returns a block layout when 2.2 says so |

`seat_box_sources` (2497) stays: it is the box-source stage of the non-block ranking.

**`src/scene.rs`**

| function (0.38 line) | change |
|---|---|
| `scene_labelled` (3050), `route_labelled` (4402) | read the switch once per call and pass it down (4.1) |
| `scene_flat` (3083), `sub_edge_proxies` (3994), `grid_pairs` (4172), `clusters_line_up` (5277), `compound` hooks | none |
| `scene_clustered` (3444) | stand-ins from `levels::augmented`; frames (step g, 3535-3599) through the shared frame function |
| `scene_from_layout` (3159) | E1-E3 anchors on flow-facing sides (it already draws a descending chain: the `a_bottom` branch at 3279-3285); the adjacent-layer E3 as a mirrored Z; title-clear ports and displaced entries; free pins; label on the G3 dummy |
| `static_lanes` (3097) | E4 only in block mode |
| sub-edge drawing (3608-3743): `cut_to_frames` (3772), `own_box_stub` (3832), `sub_edge_ported` (3889), `free_edge` (8541), `sub_parallel_offsets` (7403) | free pins (the chain is cut at its last outside dummy); the three own-box rules of 2.8, for every clustered diagram (B12.14) |
| `cluster_raw_boxes` (5346), `route_clusters` (5389), `route_lane_clusters` (5404), `stop_at_siblings` (5481), `renest_boxes` (5524) | one frame function for the three entry points: F1 with the title breadth and F2 (block mode; a pure function of member boxes, title, graph and classes, B12.7), F3 (every clustered diagram, B12.10) |
| `route_built` (4435) | `levels::augmented` and `block_mode` once per call; R1's three steps; R2 classes; a corridor builder beside `cross_cluster_route` (8400) per R3; `route_backs` (8097) for E4 only in block mode; free pins; R8; the own-box rules |
| `route_partial_with` (4889) | R6: the frame loop at 4980-4995 also redraws the end blocks of a border link whose other end's unit holds a moved node, and a redrawn frame takes F3 from the paths this call draws; no new state in the base scene |
| `title_head` (2885), `fit_cluster_title` (2902) | read by the title rules; unchanged |

**Other files**

- `src/lanes.rs`: `plan` (696) and `Want` (659) stay as they are and keep serving E4. A `corridor_geometry` for R3
  beside the lane geometry may reuse the stub and nesting helpers.
- `src/ortho.rs`: no rule change intended; `CHIP_CLEAR` becomes `pub(crate)`. Verify, with tests, that `ortho_pass`
  (477) keeps an on-axes builder path and only rounds it (the rule at 716-727), keeps the end sides the builder set,
  and that `plan_jog` (1562) puts a band-gap step between the two frames (G1 gives it the room).
- `src/router.rs`: `repair` (5120): R4. `smooth` (2742) is called by the corridor builder where `orthogonalize` does
  not run. Its private `CHIP_CLEAR` is renamed and keeps its value.
- `src/ports.rs`: two functions change. `unshaded` (1563) takes the free stretch of a title-row member's title side
  from the function F2 uses, in block mode by breadth alone; the first `cap(m)` border entries keep the free part and
  the rest are displaced; outside block mode it is unchanged. `spill` (1743) follows the spill order of 2.8.
  `d-hybrid`'s prototype had `shared_ports` 2 on dense-lr once turned edges left a hub by its upstream side; the hard
  zero must hold.
- `src/edgepass.rs`: `ctx` (126) keeps the geometric `back`; every consumer that treats a back-edge as a lane (find them
  by `cx.back`) must ask "E4?" in block mode. `finish_held` (361) is unchanged; from B11 on it runs the comb as its
  last path stage (3.3). If the comb has to be restricted to fans whose branches stay in one level, `edgepass::Ctx`
  needs the level tree.
- `src/comb.rs` (B11): no change intended; the conditional clamp of 2.8 only if B12.11's test asks for it.
- `src/compound.rs`: untouched; its tests are the proof.
- `src/quality.rs`: new measures appended to `Metrics::tail_fields` (so every earlier column keeps its number).
  - Geometry only (B12.0): `compact_max`, `compact_mean`, `span_max`, `sib_stagger` (as block-metrics defines them),
    `title_cut` (frame titles drawn with an ellipsis), `edge_len`, `bends_max`, `detour_max` (block-metrics' columns
    of the same names), `lbl_end_mean` (block-metrics' `lbl_endpx_mean`), `chip_crowd` and `chip_gap_min`
    (block-metrics' `lbl_crowd` and `lbl_gap_min`: labels with a foreign stroke nearer than `CLEAR_GAP` to their chip,
    and the smallest such distance). **The harness's existing `lbl_crowd` keeps its name and its meaning**: it counts
    pairs of label boxes closer than `LABEL_CROWD_GAP` = 4 px, reads 0 on the complaint drawing, and is not the gate
    for crowded chips.
  - With the level tree (B12.1): `sib_side_linked`; `lateral_x` (frame crossings through a lateral side by an edge
    with exactly one end in the frame; note links counted); `up_lanes` (upstream edges with a block end that do not
    leave their source through its upstream side); `side_ends` (ends of E1-E3 edges, both ends, also where the node
    is top-level, that lie on a node side across the flow; E4 lanes, self-loops and note links are not counted);
    `spine_off` (2.5); `pin_corner` (edge ends on a frame at or within `PORT_SPACING` of one of its corners; the
    experiment has 2 on the analog).
  - Existing since B11, reused: `fan_x` and `fan_uneven` (O6). Do not add a second `fan_x`.
  - In `measure_source` (486), which has both drawings, as fields of `quality::Report` (the module is `doc(hidden)`,
    not a stable API), printed as extra columns of the routed row: `fr_max` (largest frame difference between
    `scene()` and `route(scene centres)`), `ends_apart` (edges with an end more than 20 px apart between the two),
    and from the drag frames `dfr_own` / `dfr_far` (largest move, on a 1 px drag from `scene()`, of a frame that
    2.9 R6 redraws / of any other frame).
  - `frame_side` (1649-1666) and `wiggles` (1626-1647) keep their definitions.
  - Names in `tests/quality.rs` for the block-metrics rows of 4.3: crossings -> `x_drawn`; `xframe_runs` 0 ->
    `edge_frame_x` 0 (the validated pair is `xframe_edges` = `edge_frame_x`; for a zero gate the two are equivalent);
    `lbl_struck` -> `lbl_edge`; `lbl_crowd` -> `chip_crowd`; `lbl_endpx_mean` -> `lbl_end_mean`; area -> width x
    height; the others by their own names.

### 3.2 How the entry points stay consistent

| entry | block mode | what it shares with the others |
|---|---|---|
| `scene()`, `scene_sized()` | L, O, X, G, F, E; one scene, one `finish` | the switch value, `levels::augmented`, `block_mode`, level tree, frame function |
| `route()`, `route_sized()` | R1-R5, R7, R8; no layout; host positions never moved | the same constructor and predicate, level tree, frame function, edge classes (R2 = L6 at the scene's centres, by G1's floor) |
| `route_partial()` | R6 | same builders as `route()` for redrawn edges; F1 and F2 from the members and F3 from the paths it draws for a redrawn frame; base frames for every other block |
| hit-testing (`hit_test`, `node_at`, `edge_at`, `cluster_at`, `hits_in_rect`, `nearest_node`) | reads the `Scene` only | frames nest and siblings are disjoint (`pair_sep` + `cluster_gap`; `assert_clean_clusters`); one `SceneEdge` per edge, so `edge_at` follows the painted stroke (`edge_picking_matches_the_painted_strokes_on_every_fixture` gets `tests/fixtures/blocks` added to its directory list) |
| `fold::scene_compact` | refuses graphs with subgraphs (`FoldSkip::HasSubgraphs`), returns `scene()` | unchanged; assert it for a block-mode fixture |
| `to_mermaid`, json export | graph / scene only; no new field | round trip renders byte-identical |
| `advance`, class, ER | never clustered | the permanent hash test |
| `layout::layout_clustered` (public) | block layout | documented |

### 3.3 B11

From B11 on two things run on every drawing, so **every block-mode number from B12.4 on includes them**; they are not
something a later item switches on.

- *Hops* are drawn by the painter from finished scene edges and need nothing from B12. Neighbouring hops join into one
  bridge (a quarter circle up, level over the bundle, a quarter circle down). block-metrics bridges single hops today
  and gives up on a detour longer than 26 px, so a bridge over two or more tracks would read as two extra bends, a
  broken straight run and a level stretch 5 px off its line, which can sit 7 to 7.5 px from the next line and fail
  `lane_gap_min` as an artefact. B12.0 teaches the tool B11's bridges before any B12 number is taken.
- *The comb* is the last path stage of `edgepass::finish_held` in `scene()`, `route()` and `route_partial()`
  (`comb::comb` after `spread_close`), and it combs frame sides (`EndRef::Frame`) as well as node sides: `plan_even`
  re-spaces a side of `FAN_MIN` or more movable ends evenly between its outermost two, and `fan_uneven` counts a node
  or frame side whose pitches differ by more than `FAN_PITCH_TOL`. Three B12 rules meet it, with this precedence: the
  final order of a side's ends is the comb's (O6); free pins are placed as even bands, the shape `plan_even` leaves
  alone (2.8); a displaced entry's jog must still lie inside the frame after the comb (2.8, B12.11). `fan_x` and
  `fan_uneven` of the starting release are the references.

A hub's chains now leave one side in fan order and run parallel, which is the shape the comb wants; nobody has run the
two together on block layouts. If the comb worsens a must (crossings, `lbl_*`, `lane_gap_min`), restrict it to fans
whose branches stay in one level (that needs the level tree in `edgepass::Ctx`) and say so in the commit.
block-metrics counts a shared trunk once per edge.

### 3.4 Guards in shared code

Every new branch reads `blocks` (the switch is on and the predicate of 2.2 holds, which implies clustered), except the
two rules that apply to every clustered diagram (F3, the own-box edges), which read "the switch is on and the diagram
is clustered". The switch value is read once per entry call and passed down; no code deeper in reads the thread-local.
No change to a line a flat graph executes: not the sweep loop's judging, not `coordinates_bk`'s candidates, not
`back_lanes`. The off-identity test of B12.0 is the proof while the switch exists, the permanent hash test after it.

### 3.5 Cost

- L: linear per level, plus per cyclic level the subset programme for `M_c` and for `U` and, where L4's first test
  passes, one longest-path pass per child block; block mode itself is one pass over the links. `subgraph.mmd` (static
  baseline 0.08 ms) and `advanced.mmd` are now drawn in block mode: measure them first (limits in 4.6; decision D1
  gives the sub-millisecond files `baseline + 0.3 ms`).
- O: the experiment's implementation is naive (maps rebuilt per call, quadratic crossing counts, every layer regrouped
  after every reorder). Measured `scene_ms` with seven starts: **analog 310 (one start: 50-60, and then 36-37
  crossings), bench-75 1510, dense-td 648, dense-lr 763, fanout-frames 32**; 0.38: 6.6, 48.6, 24.9, 20.3, 2.9. The
  limits of 4.6 are 13.2, 61.9, 50.7, 46.8, 4.4. Required: collapsed graphs in index arrays; bilayer crossings by
  `inversions`; exchange by local deltas; regroup only the layer just reordered; starts budgeted by
  `BLOCK_ORDER_WORK`; later placements warm-started, one run each. Target: ordering under 15 % of `scene_ms` on
  bench-75. The budget may not buy speed with crossings: B12.5 delivers the table crossings x starts (1..7) and shows
  the musts at the granted number.
- `route()`: unchanged `route()` at block positions is slower because the router repairs more (same binary, switch off
  -> on: analog 1.9 -> 4.0 ms, dense-td 6.2 -> 8.7, dense-lr 5.8 -> 7.9, bench-75 11.3 -> 9.2; `d-global` 4.7 and
  `d-hybrid` 5.0 on the analog; hand-placed blocks 4.8). **On the analog every one of these breaks the limit of 2.9 ms,
  and none has corridors.** Corridors give the edge stages clean paths; whether that brings the numbers inside the
  budgets is unmeasured, which is why a spike measures it right after B12.1 (B12.2, checkpoint S) and the route item
  is built before the package's acceptance, with its own stop rule.
- `levels::augmented` + `block_mode` run once per `route()` and per `route_partial()` frame: measure them in B12.13.

## 4. Work items, acceptance, tests, gates, budgets, docs

### 4.1 The switch and the checkpoints

**The switch.** Until checkpoint C passes, everything B12 changes sits behind one internal switch that is off in every
ordinary build, so `dev` stays releasable.

- `levels::BLOCKS_DEFAULT: bool = cfg!(flowmaid_blocks)`: false in every ordinary build (declare the cfg under
  `[lints.rust] unexpected_cfgs` with its `check-cfg` in `Cargo.toml`), plus a thread-local `Cell<Option<bool>>`
  override. The value is read once per entry call (`scene_labelled`, `route_labelled`, `route_partial_with`, and
  `layout_core` when it is reached through the public layout functions) and passed down (3.4). There is no
  process-global (the tests of one binary run on parallel threads and one would leak block mode into tests that must
  stay off) and no environment variable in the library.
- `#[doc(hidden)] pub fn flowmaid::quality::with_blocks<T>(on: bool, f: impl FnOnce() -> T) -> T` is the only setter:
  scoped, and it restores the old value also on unwind (`quality` is already a `doc(hidden)` module). Every new test of
  4.4 wraps its body in it.
- **The default run** is the ordinary build. In it the off-identity test of B12.0 is green in every commit of B12a and
  of B12b before B12.16: every output is byte-identical to the starting release.
- **The suite-on run** is the whole build with `RUSTFLAGS='--cfg flowmaid_blocks'` and its own `CARGO_TARGET_DIR`
  (delete it after the checkpoint; disk is limited): `cargo test`, the `quality` example behind `perf.sh`, and the CLI
  handed to `B/metrics/run.sh` and `render.sh`. This is how the CLI, `perf.sh` and the renders enter block mode: the
  CLI has no option for it. **Every "passes as written", "unchanged" and "holds as written" in 4.2 and 4.5 means "in
  the suite-on run".** Each checkpoint's report attaches the suite-on list of failing tests; it must be a subset of
  4.5's intended changes plus the rules the checkpoint's row names as not built yet.
- B12.16: the constant becomes `true`; the cfg, the thread-local and `with_blocks` are deleted.

**Checkpoints.** Each ends with a report (4.8). A gate that cannot be met is a stop: stop and report, never re-baseline
(decision D6).

| checkpoint | after | gate | stop rule, besides "any part of the gate cannot be met" |
|---|---|---|---|
| S spike | B12.2 | `route_ms` of R2 + R3 at the experiment's block positions on the analog within 4.6, on two runs of the spike's own harness (fastest of `ROUTE_RUNS`, as `quality::measure_source` times `route`). `drag_ms` is reported, not gated. bench-75 and dense-td are reported with their band gaps noted: the experiment's positions crush those gaps (2.6 G1), so a number there has a second cause | over the limit on the analog on both runs -> stop and report before B12.3 starts: the corridor design decides whether the rest is worth building |
| A1 structure | B12.6 | layers and bands (B12.3); chains through flow-facing sides (B12.4); the crossing musts of 4.3 **with the number of starts `BLOCK_ORDER_WORK` grants**, the table crossings x starts, and `scene_ms` of the analog, the public fixture, bench-75, dense-td and dense-lr within 4.6 (B12.5); the harness total `x_drawn` (static) over `tests/fixtures/quality` not above the starting release's; both declaration-order tests pass as written; the sweep's invariants (B12.6); hard zeros 0 in the static scene; not built yet: X, G, F, E's ends, R | crossings above the must on the analog (20), v2 (17), a permutation (20 / 23 / 20 / 20 / 22), the public fixture (7), or a dense fixture (4.3 f), or the harness total above the starting release's; or the musts need more starts than the budget grants; or dense-td under the test's six shuffles is outside 10 % in width or above its crossing must -> stop and report with the table crossings x starts |
| A2 geometry | B12.12 | every static row of 4.3 (a)-(d), (f) and (g), on the local inputs and on the repository fixtures; the static column of `DEFECT_BASELINE` by the rule of 4.5; `scene_ms` within 4.6 on two runs; `BLOCK_SLACK` pinned; not built yet: R, the own-box rules of 2.8 | a must that held at A1 and fails at A2 -> stop and report |
| R route | B12.13 | every budget of 4.6, the `route_partial` column included; `DRAG_JUMPS_MAX` as the starting release has it; routed hard zeros 0; `fr_max`, `ends_apart`, `dfr_own`, `dfr_far` (4.3 d); the routed rows of 4.3 (d), (f), (h); the routed column of `DEFECT_BASELINE` by the rule of 4.5; not built yet: the own-box rules of 2.8 | a budget broken on two `perf.sh` runs in a row -> stop and report |
| C acceptance | B12.15, before the flip of B12.16 | `judge` reports 0 must-fails, static and routed, against complete reference tables; every rule of 4.3; guard green; nothing "not built yet" | never flip the default with a must failing; a rows-equal input worse than the reference on a rule of 4.3 (e) that cannot be fixed -> stop and report (the user's fallback is option (a) of D7) |

Which rows of 4.3 are due where:

| rows | first due | judged again |
|---|---|---|
| `xframe_runs`, `sib_side_linked`; crossings of (a)-(c) and (f); band order and canvas breadth across permutations; `fan_x` | A1 | A2, C |
| `compact_*`, `span_max`, `sib_stagger`, area, `edge_len`, `straight_max`, `bends*`, `detour_max`, `spine_off`; every label row; `lane_gap_min`; `lateral_x`, `side_ends`, `up_lanes`, `pin_corner`, `title_cut`, `fan_uneven`; the static halves of (d), (f), (g) | A2 | C |
| (h); the routed halves of (d), (f), (g); `fr_max`, `ends_apart`, `dfr_*`; the budgets of 4.6 | R | C |
| (e) with its picture triples; `warn` 0 on `B/metrics/extra/in`; the own-box rules; everything again in the default build after the flip | C | |

### 4.2 Items

Each item ends with guard green (`tools/guard.sh`, default run) and its own regression tests. Commit per item. Items
B12.0-B12.13 are package B12a, B12.14-B12.16 package B12b (section 5).

**B12.0 Baselines, fixtures, yardstick.** Nothing in this item changes the library's output.

- *Starting-release artefacts.* Record the start commit. Keep its CLI binary and its `quality` example binary in
  `B/start/` (outside the repository): later items compare against them.
- *Off-identity test* `everything_renders_as_before_with_blocks_off`, a unit test inside the crate (`scene::DefsKey`
  is `pub(crate)`). For every `.mmd` under `examples/` and `tests/fixtures/` (the new `blocks/` included): the FNV-1a
  64 hash (`scene::DefsKey`, never `DefaultHasher`, which is not stable across toolchains) of the SVG; for flowchart
  and state inputs also of `to_svg(route(g, scene centres))` and of `to_svg(route_partial(..))` with the first node
  moved by (7, 5). The hashes are recorded on the start commit before any other change and asserted with the switch
  off in every commit of B12a and B12b (from B12.1 on the test wraps its body in `with_blocks(false, ..)`, so it holds
  in the suite-on run too). B12.16 shrinks the test to the flat and not-block-mode rows
  (`flat_and_unlinked_inputs_render_as_before`), which stay for good; a file with an own-box edge of 2.8 leaves that
  permanent list.
- *Repository fixtures* `tests/fixtures/blocks/`. Block fixtures never go into `tests/fixtures/quality`: dozens of
  new files there would move every summed gate.
  - the public fixture `B/revise/in/blocks-overview.mmd`; four permutations of it written here (edge statements
    reversed, subgraph blocks reversed, actors reversed, edge statements rotated) and its BT / LR / RL copies;
  - from `B/critic/in`: `hub5`, `cycle2`, `ratio2`, `ratio3`, `ratio4`, `cycle2-tie`, `cycle2-tie-swapped`,
    `interleave`, `nested`, `self-member`, `sub-nested-self`, `b9-inside`, `b9-inside2`, the state diagrams
    `state-note`, `note-edge-row`; and `lr`, **rewritten with neutral ids and labels before it is committed** (the
    public fixture's vocabulary: a shared-services block with a long title, a CDN, a pager, storefront and orders
    blocks): `B/critic/in/lr.mmd` itself is a miniature of the analog and stays a local input;
  - `B/d-hybrid/in/{swimlane, nested-cases}.mmd`; the probes `B/u-mermaid/probes/p*.mmd`;
    `B/spec/evidence/title-entry.mmd` with its title reworded (it carries a phrase of an analog title);
  - from `B/check-fix-a/in`: `reqresp`, `reqresp22`, `t9-frames-8px`, `t9b-frames-8px-lr`, `t1-nest-own`,
    `t2-self-nested`, `t2b-self-nested-order`, and the two-way variant `B/check-verify-fix-a/in/s3.mmd`;
  - from `B/check-fix-b/in`: `l5loop`, `swim-cycle`, `swim-nested`, `ratio2x4`, `ratio4mask`, `selfsub-nested`,
    `nested-self-only`, `own-centre`, `title-hidden`;
  - `B/check-cold-read/apath-nested.mmd` as `nested-chain`; `B/check-revise-2/in/swimlane-nested.mmd`;
  - not in block mode (for 1.3 item 3): `B/check-fix-a/in/iso-lane.mmd` (an isolated subgraph with a labelled inner
    back-edge and an inner edge over three rows), its LR copy, `iso-state` (an isolated composite with an inner cycle
    and a note), and, written here, an isolated two-level nest and an edge to an empty subgraph;
  - written here: a block fixture with `linkStyle default stroke-width:3px` (2.8, R3).
- **Local acceptance inputs, never committed**: the analog, v2, the five permutations `B/u-mermaid/in/perm-*.mmd`,
  `B/critic/in/infra-{bt,lr}.mmd`, an RL copy, and `B/critic/in/lr.mmd`. Keep them in one directory outside the
  repository, with copies of the repository's block fixtures and the inputs of 4.3 (e) and (f) beside them, and point
  `B/metrics/run.sh` (`IN=<dir>`) at it. The rule of the header applies: nothing of theirs in the repository.
- *Harness measures.* Add the geometry-only measures of 3.1 to `quality.rs` (the tree-based ones are B12.1's).
- *block-metrics on the starting release.* (1) Extend `dehop` to B11's bridges: a detour that leaves its line on an arc
  of radius <= `HOP_R_MAX` (9), runs level at that offset for any length and returns to the same line; selftest
  drawings with a single hop, a bridge over 3 tracks and one over 6. (2) New columns, each with hand-checked selftest
  cases: `lateral_x` and `lateral_edges` (per crossing of a frame border by an edge with exactly one end held by the
  frame: lateral or flow-facing; computed on the common model for both SVG dialects and all four directions; note
  links are counted, they are what mermaid's 1 on `notes/spread-bottom` and `nested-composite` is); `bends_max` (most
  bends on one edge) and `detour_max` (path length over the Manhattan distance of its two ends, edges of 200 px and
  more); `lbl_in_end` (chips whose centre lies inside, or whose box overlaps the border of, a frame that holds exactly
  one end of their edge) and `lbl_own_gap_max` (largest distance from a chip's box to its own edge; 0 = the line
  touches the chip); `bands` (top-level frames and free nodes grouped into bands by overlapping flow projections,
  bands in flow order, members by position across the flow, as one string). (3) `judge`: a must whose bound resolves
  to NA is a MUST FAIL (today it is skipped); a rule scoped to a name that is absent from the candidate table is a
  MUST FAIL; a third reference table `start` beside `baseline` and `mermaid` (`--start`); bound kinds `{min: [..]}`
  (NA when any member is NA), `{same_as: <name>}` for a string column and `{within: 0.10, of: <name>}`; a switch
  `--only <names>` for the interim checks that run a subset, which prints the
  names it skipped.
- *Reference tables*, one row per acceptance input under the name the candidate run uses (the sweep tables' long names
  such as `tree_tests_fixtures_notes_near-drag` would not match): `B/metrics/baseline-038.tsv` gets the rows of v2,
  the permutations, the public fixture with its permutations and direction copies, and every input of 4.3 (e) and
  (f); `B/metrics/ref-start.tsv` the same rows from the starting release; `B/metrics/ref-mermaid.tsv` mermaid's rows
  wherever a render exists (`B/metrics/mermaid.tsv`, `B/metrics/extra/sweep-mermaid.tsv`, `B/critic/mm`,
  `B/revise/mmm`, `B/check-fix-a/mm`); and the routed twins `ref-038-routed.tsv` and `ref-start-routed.tsv` from
  `examples/quality --svg`'s `<name>.routed.svg`. Rewrite `B/metrics/thresholds.json` to the tables of 4.3 (today's
  file holds earlier, looser bounds) and write `thresholds-routed.json` for 4.3 (h).
- *Harness recordings from the starting release*, per fixture and local input, static and routed: the harness row with
  the new measures, `fan_x`, `fan_uneven`, and the totals `DEFECT_BASELINE`, `XGAP_MAX`, `DRAG_JUMPS_MAX` and the
  casing limits as the starting release has them. Write the new table `BLOCK_CEILINGS` (4.5) and the per-file numbers
  of 4.3 (f).
- *Tests B11 and the leftovers added.* List, by grep for the dense fixture names, every test they added that names a
  clustered fixture, and classify each under 4.5.
- *Perf baselines.* Copy the public fixture and the local inputs into the corpus directory `perf.sh` reads (outside the
  repository) and record `perf.sh b12-start` twice on the starting release; the limits are read off its table by hand
  (`perfcmp.py` prints, it does not judge). Add the public fixture to the list of `fixture_drag_perf_guard`, run the
  same loop for the analog from a local example, and record the worst `route_partial` frame per file on two runs
  (4.6). Before renders.

*Check*: the off-identity test passes on the untouched tree. On the starting release's SVGs of the 8 metric inputs:
`hops` > 0 and `warn` 0 on bench-75, dense-td and the analog, `crossings` equals the harness's `x_drawn`, and
`--check-scene` passes. The harness measures agree with block-metrics on the 8 metric inputs (`compact_*`, `span_max`,
`sib_stagger` within rounding; `chip_crowd` equals `lbl_crowd`, analog: 8, or each differing chip is named: one reads
the scene, the other the trimmed SVG). `lateral_x` agrees with `lateral.py` on the starting release's SVGs of hub5 (5),
cycle2 (6), the public fixture (14) and, with `lat2.py file.svg:h`, spread-bottom (1), and reads 0 on mermaid's analog
with 36 flow-side crossings counted. `judge` on the 0.38 rows of v2, the permutations and the public fixture reports
must-fails on each (today it reports none: every bound is NA).

**B12.1 Level tree, block mode, the switch.** `levels.rs` (`augmented` with the flag of 2.1, `block_mode`, the tree);
the switch of 4.1; the tree-based measures of 3.1, recorded at once with the switch off (the drawing is still the
starting release's): `lateral_x`, `up_lanes`, `side_ends`, `spine_off`, `pin_corner`, `sib_side_linked`, `fr_max`,
`ends_apart`, `dfr_own`, `dfr_far` per fixture and mode.
*Check*: unit tests of the tree on nested, sub-edge, own-member and empty-subgraph graphs. `block_mode` false on every
flat input, on `examples/state.mmd`, on the probes p01, p02, p09, p13, on the not-block-mode fixtures of B12.0, on an
isolated nest joined only by `OUT --> IN` (`t1-nest-own`, `nested-self-only`) and on an isolated subgraph with a nested
block and `S --> S` in both member orders (`t2-self-nested`, `t2b-self-nested-order`, `selfsub-nested`); true on the
public fixture, bench-75, dense-lr, dense-td, fanout-frames, `examples/{advanced, subgraph}.mmd`, hub5, cycle2 and
every `notes` fixture with a transition or a note across a composite border (and, locally, on the analog family).
`block_mode_agrees_between_scene_and_route`: each entry point records the value it used in a `#[cfg(test)]`
thread-local and the test compares the three records per fixture (calling one predicate three times could not fail).
A test asserts that the set of repository fixtures with `block_mode` false equals the set the permanent hash test
lists. `the_switch_changes_nothing_outside_block_mode` (it runs until B12.16): for every fixture with `block_mode`
false and no own-box edge of 2.8, the `scene()` SVG with the switch on equals the SVG with it off, and `route()`
differs in frames only. The measures agree with `lateral.py` as in B12.0's check.

**B12.2 Corridor spike (throwaway, not merged).** R2 + R3 + R5 in `route_built`, enough to route the three inputs, on
the experiment's block positions: build `B/spec/tree` with the switches of appendix A, dump each node's centre and size
from its static scene (an example of ten lines), and call `route_sized` with them on the working branch. Measure
`route_ms` (fastest of `ROUTE_RUNS`) and the worst `drag_ms` on the analog, bench-75 and dense-td, twice.
*Check*: the table against 4.6; routed runs through a foreign frame on the three inputs. **Checkpoint S.**

**Decision D7 is confirmed** (section 0, appendix B): B12.3 builds option (b). With the off-identity test over all
fixtures, nothing recorded in B12.0 depends on it.

**B12.3 Layering.** `block_rank` (L0-L7). `grid_pairs` is extended to members without inner links: a row of them inside
a block is wrapped when it is more than `RIBBON_ENTRY` times as broad as the block is deep. Deliverable for 4.3 (e):
the `quality` example prints, with `--layers`, each node's layer in block mode and in the starting release's ranking,
and a test prints the fixtures whose two layerings are equal (the rows-equal list).
*Check*: the public fixture has the same bands under its four permutations (locally: on the analog the layers are
actors / R1 R2 GW DNS TG OAI AZ / ARGO TR1 VAULT / LANE / PG / SITE DRAPP PSDEV TR2 / PSPROD LANES4 / GITLAB with
five upstream edges, `DRAPP->TG`, `ARGO->TG`, `PSPROD->TG`, `LANE->OAI`, `LANE->AZ`, and the same bands under all five
permutations). dense-td at most 5 edges against the flow (single walk: 21).
`ratio2` and `ratio3` keep Front first, `ratio4` puts Back first, `cycle2` keeps Front first; `ratio2x4`: Left/Right
keep declaration order, Front/Back flip. `reqresp` and `reqresp22` are stacked with Front first and 1 and 2 upstream
edges. `cycle2-tie`: Back first, `b1` above `b2`, `a1` above `a2`, exactly one upstream edge (`a2 -> b1`);
`cycle2-tie-swapped`: Front first, one upstream edge. L4 fires on `swimlane` (0 turned), `swim-cycle`
(one back-edge, a lane, as in 0.38) and `swimlane-nested` (one link turned at the dissolved level, an E3 into the
nested block) and on no other fixture; the list of levels that
fire it is printed, with dense-td, bench-75 and advanced checked by name. No interleaved siblings on any fixture:
`interleave` has T rows 1-3 and S rows 4-5, `l5loop` S 0-1, T 2-5, B 2-4, and L5's pass bound holds. Every card keeps
the layer 0.38 gives it relative to its target on every notes fixture, and `notes/near-drag` and `note-edge-row` keep
their 0.38 layers (a card is its composite's last row there); `notes/routed-around` and `notes/spread-face` are not
compared: they hold composite self-transitions, which no longer rank anything (2.1).

**B12.4 Chains both ways and label slots (static).** O0 with the deeper-fit rule, E1-E4, G3's choice of dummy.
*Check*: no edge drawn as one curve across more than one layer in block mode; an adjacent-layer upstream edge with a
block end is a Z through flow-facing sides, not a lane (test on a three-block cycle: `up_lanes` 0); `wiggles` 0;
`nested-chain`: the chain out of the nested block leaves through that block's downstream side, 0 lateral crossings;
`lbl_in_end` 0 on the public fixture (and on the analog family): every chip's centre outside both end frames.

**B12.5 Ordering.** The canonical order, O1-O6 on every level, O4b, budgeted starts, the warm start of later
placements.
*Check*: the crossing rows of 4.3 **with the granted number of starts**, and the table crossings x starts (1..7) for
the analog, v2, the five permutations, the public fixture and its permutations, dense-td, dense-lr, bench-75;
`scene_ms` of the analog, the public fixture, bench-75, dense-td, dense-lr within 4.6;
`edge_line_order_barely_moves_the_dense_layouts` and `moving_a_node_declaration_barely_moves_dense_td` pass as
written, and dense-td keeps its crossing must under the six shuffles; `fan_x` 0 on the analog family and the public
fixture, with a test that pins a hub's fan on the public fixture (its ends on one side, in target order, no crossing
among them, B11's comb included); runs through a foreign frame 0 static on the analog, v2, the public fixture,
bench-75, dense-lr, dense-td (`d-global`: 3 on dense-td; experiment: 1 on dense-lr); a `#[cfg(test)]` offset added to
the start index `k` does not change the order inside the public fixture's shared-services block (locally: inside
External services), and its crossing-neutral members are in declaration order (O4b); the shuffled starts hold O2's
two constraints before the first sweep.

**B12.6 Generated sweep: invariants and constants.** The generator: the corpus the project used for B8 (about 1800
files) plus generated block diagrams (2-8 blocks, 0-3 nesting levels, cycles, there-and-back exchanges, ping-pong
lanes, hubs, all four directions, state composites with notes); it is seeded. Pin `2 * M_c + 1`, `LANE_SHARE`,
`BLOCK_ORDER_STARTS` and `BLOCK_ORDER_WORK`.
*Check*: no panic in `scene()` and `route()`, deterministic, hard zeros 0 in the static scene on every file; L5's loop
ends within its bound and leaves no interleaved pair; O2's two constraints hold; L0's kept marks and L3 leave every
level's graph acyclic; the levels that fire L4 are listed; every generated file that is not in block mode is
byte-identical to the SVG of the starting release's binary (`B/start/`), and
`the_switch_changes_nothing_outside_block_mode` takes the generator's graphs. **Checkpoint A1.**

**B12.7 Frame function: F1 and F2.** One pure function (member boxes, title, graph, classes -> box), the title row,
the shared free-stretch function with `ports::unshaded`, used by the static scene here and by `route()` from B12.10
on.
*Check*: outside block mode it returns today's static frames (off-identity). Unit tests of the function: `lr` (the
frame is as long as its title chip plus `2 * TITLE_INSET`), `title-hidden` (as broad), `title-entry` (two displaced
entries: `ROW_CLEAR + TRACK_PITCH` of jog room); a title-row hub whose exits leave downstream reserves no jog room,
and a block whose three entries fit beside the chip reserves none. `title_cut` 0 on `title-entry`, `title-hidden` and
`lr` in all four directions.

**B12.8 Coordinates.** X1-X4, in the order 2.5 gives.
*Check*: public fixture `compact_max` <= 7.97 and `compact_mean` <= 4.63 (analog: <= 5.5 / <= 3.9); the edge-shape
rows of 4.3 (`bends`, `bends_max`, `detour_max`) on the analog, v2 and the public fixture; `spine_off` 0 on the
analog, v2, the public fixture, hub5, spread-face, nested-composite, fork-twins; `sib_stagger` 0 on the analog and the
public fixture; `frame_overlap`, `node_frame` 0; own-frame containment tests unchanged
(`a_lane_between_two_members_stays_in_their_box`, `a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`,
`a_rerouted_edge_between_members_stays_in_their_frame_in_route`, `back_edge_inside_a_box_bows_beside_it...`) and the
same assertion on `nested-cases` and spread-face.

**B12.9 Gap room and labels.** G1-G3.
*Check*: `GAP_FLOORS` hold as written (bench-75 static 14.7; prototypes: 8.6-11.9); bench-75 static `overlap_pairs` 0;
the frames of `t9-frames-8px`, its LR copy and the two-way variant are more than `FLOW_MIN_GAP` apart. On the analog,
v2, the five permutations and the public fixture (every prototype had 4-12 crowded chips on each of them, and 1-6
struck ones on v2 and the permutations): hard here `lbl_struck` 0, `lbl_lbl` 0, `lbl_node` 0, `lbl_in_end` 0,
`lbl_own_gap_max`, `lane_gap_min` and the area caps of 4.3; reported here and hard at checkpoint A2, after B12.11 has
moved the entries: `lbl_crowd`, `lbl_gap_min`, `lbl_endpx_mean`. Report `lbl_endpx_mean` and the mean length of a
labelled edge for the analog and v2.

**B12.10 Frames across the flow; one frame function in three entry points.** F3 for every clustered diagram, and R1's
three steps with R2's class test in `route_built`, so that `scene()`, `route()` and `route_partial()` call the same
function. Until B12.13 supplies R3's inner legs, F3 in `route()` grows over the paths today's builders draw.
*Check*: outside block mode the static frames are today's (off-identity, `iso-lane` included); along the flow, and
in F1's breadth, the frames of `scene()` and of `route()` at the scene's centres are equal to 0.5 px on every block
fixture in all four directions (`lr`, `title-hidden` and `title-entry` by name: title length, title breadth and jog
room). Across the flow F3 still differs until B12.13 supplies the inner legs: the `fr_max` gate and
`frames_agree_between_scene_and_route` are B12.13's.

**B12.11 Flow-facing ends, free pins, title-clear entries (static)** (2.7 F2, 2.8).
*Check*: `lateral_x` 0 and `side_ends` 0 static on the analog, v2, the permutations, the public fixture, hub5, cycle2
and their BT/LR/RL copies; `pin_corner` 0; no title struck (existing chip gates); `frame_side` 0; `up_lanes` 0; on
`title-entry` and on a variant of it with a border-crossing chain the entry jogs in beside the chip **and the jog
lies inside the frame after the comb has run**; on `title-hidden` the chain to the second-row member crosses the
title side beside the chip, not the lateral wall; the frame covers the last displaced entry's column plus `LANE_PAD`;
`fan_uneven` not above the starting release's per fixture, a side whose pins are split by member entries named; dense-td
`n16` and dense-lr `n12` by name: which of their ends spill, and that no border end spills while an inner end could.

**B12.12 Narrowing on block layouts.** `narrow`'s entry (`RIBBON_ENTRY`) and acceptance slacks on block layouts;
`a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` and `wide_sibling_boxes_stack_instead_of_a_ribbon` pass as
written (their diagrams are now drawn in block mode; the rules they state still hold).
*Check*: dense-lr area <= 1.25 x 0.38's (7.98 Mpx2; experiment 8.93) and no block row longer than its band needs;
narrowing still accepts a candidate on dense-td and dense-lr (their block-mode width is not above the width with
narrowing disabled); `MAX_ASPECT` and `INK_MEAN_MIN` hold as written; `BLOCK_SLACK` pinned on the sweep of B12.6.
**Checkpoint A2.**

**B12.13 `route()`, `route_sized()`, `route_partial()`.** R1-R8, with the routed half of 2.8 (flow-facing ends, free
pins and title-clear entries against the current frames). Measure with `perf.sh`.
*Check*: budgets of 4.6 on two runs, the `route_partial` column included, the cost of `levels::augmented` +
`block_mode` per drag frame stated. `DRAG_JUMPS_MAX` as the starting release has it. Routed hard zeros 0, also on the
generated corpus of B12.6. Routed edge / foreign-frame pairs: analog, v2, public fixture, bench-75 0, dense-lr and
dense-td not above their static values + 2 (experiment without corridors: 9 and 2; 0.38: 13 and 29). Routed
`min_edge_gap` at or above `GAP_FLOORS` (experiment: 0.05 on bench-75). `ends_apart` not above the starting release's
count on any input (0.38: analog 14, dense-td 12). `fr_max` <= 0.5 px on the analog, v2, the public fixture and their
BT/LR/RL copies, and on no input above the starting release's value by the same probe (0.38: nested-composite 41,
wide-frames 54, spread-bottom 7.5, routed-around 24, bench-75 458, dense-lr 303, dense-td 89; block layouts with
0.38's `route()`: 80, 84, 40, 33.5, 74, 18, 0); every fixture above 0.5 px is listed with its value and cause. On 1
px drags from `scene()`: `dfr_far` 0 (no frame moves that R6 does not redraw); `dfr_own` <=
1.5 px on the analog family and the public fixture and not above the starting release's elsewhere; no kept edge has a
leg outside its end block's frame after any drag of `dragging_every_node_of_a_block_diagram_keeps_the_contract`. The
rows of 4.3 (h): routed `lateral_x` 0, `pin_corner` 0, crossings, struck and crowded chips; routed `x_drawn` and
`fan_x` of the public fixture are the same for its four permutations. `a_static_canvas_is_its_drawing_with_even_margins`,
`route_partial_keeps_unmoved_edges_and_reroutes_moved`, `route_partial_matches_route_on_the_dragged_node`,
`dragging_a_member_pulls_its_cluster_along` pass unchanged. On the `linkStyle default stroke-width` fixture: no run
through a foreign frame, corridor waypoints sampled as `router::smooth` samples them, static and routed. New tests: a
corridor that passes two bands of staggered blocks; a target on the second row of its block; a far end dragged from
above a block to beside it (no edge jumps more than `DRAG_JUMP` besides the dragged node's); a node dragged away from
above a block with displaced entries (the block's frame and its jogs are the same in `route_partial()` and in
`route()`); every node of the public fixture dragged across the drawing (no panic, frames follow, `clusters_line_up`
holds). **Checkpoint R.**

**B12.14 State diagrams and own-box edges.** Review every `tests/fixtures/notes` drawing before and after, static and
routed, and add fixtures for a fork/join bar whose branches lie in two composites and for a twin comb across a
composite border. The three own-box rules of 2.8, for every clustered diagram: the drawing, and the layout dropping
the flagged pairs outside block mode too (2.1); like everything else they sit behind the switch.
*Check*: `a_composite_edge_to_its_own_state_drops_straight_from_the_border`,
`note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `note_links_match_between_scene_and_route`,
`fork_join_branches_drop_straight_and_drags_agree`, `state_composite_keeps_flow_order` pass unchanged; `self-member`,
`sub-nested-self`, `selfsub-nested`, `nested-self-only` and `own-centre` have `edge_node_x` 0, static and routed,
where a side qualifies; block-metrics `warn` 0 on all 49 inputs of `B/metrics/extra/in` (0.38 warns on three, all
composite self-transitions); `notes/routed-around` and `notes/spread-face`, whose rows move with their
self-transitions, are reviewed picture by picture.

**B12.15 Generated sweep: comparison.** The corpus of B12.6 against the starting release and, where renders exist,
mermaid.
*Check*: no file with more crossings than both the starting release's count and 1.1 x that count + 2 without a named
reason; no level L4 dissolved drawn worse than the starting release; hard zeros 0 in both modes on every file; files
not in block mode and without an own-box edge byte-identical to the starting release's binary. Then the gate of
**checkpoint C**: everything in 4.3-4.6 in the suite-on run.

**B12.16 Default on, ratchets, docs, release.** Only after checkpoint C. Flip and remove the switch (4.1). Re-baseline
per 4.5 (numbers already accepted at A2, R and C are written down; nothing is first discovered here). README,
CHANGELOG, images, benchmark table (4.7). Final `perf.sh`, `render.sh`, `judge` in the default build. Release 0.40.0.
*Check*: everything in 4.3-4.6 in the default build; the permanent hash test green.

### 4.3 Acceptance

**How it is run.** Static rows: `IN=<the local directory of B12.0> B/metrics/run.sh fm <cli> <out>`, then
`B/metrics/judge <out>/metrics.tsv --baseline B/metrics/baseline-038.tsv --start B/metrics/ref-start.tsv --mermaid
B/metrics/ref-mermaid.tsv` with the rules of `B/metrics/thresholds.json`. Routed rows: `examples/quality --svg` writes
`<name>.routed.svg`; block-metrics on those, then `judge` with `thresholds-routed.json` and the routed reference
tables. `<cli>` and the example come from the suite-on run until B12.16 and from the default build after it. The
result must be 0 must-fails; a must without a reference value and a rule whose input is missing from the run both
count as must-fails (B12.0), so the sentence cannot pass by omission. Before C, `judge` runs with `--only` on the
inputs whose rows are due at that checkpoint (4.1) and the report prints the names it skipped. The repository
fixtures' rows are also asserted in `tests/quality.rs`; the local inputs' rows are judged from the workspace only
(4.8).

**The reference.** Rows (a)-(c) carry their numbers here, stated against mermaid 12.2 and 0.38.0. In (d)-(h), "not
above the reference" means: for a block-metrics column, not above 0.38.0's value and not above the starting release's
(`{min: [baseline, start]}`); for a harness column, not above the starting release's value recorded in B12.0 and
B12.1. The user will hold the releases between 0.38 and B12 by then, so a drawing worse than the release before it
does not pass. The 0.38 numbers printed in (d)-(f) show the size; B12.0 writes the starting release's beside them.

Rows (a)-(c) are measured on the static SVG; (h) judges the routed drawing. Where a row is stated against mermaid,
the tolerance and its reason are in the last column.

**(a) The analog (local input), against mermaid 12.2.**

| metric | 0.38 | mermaid | experiment | must | tolerance, reason |
|---|---|---|---|---|---|
| `xframe_runs`, `sib_side_linked`, `sib_stagger`, `lateral_x` | 3, 5, 3, 21 | 0, 0, 0, 0 (measured) | 0, 0, 1, 7 | 0 | none |
| `compact_max` / `compact_mean` | 11.43 / 6.90 | 4.59 / 3.25 | 5.72 / 3.56 | <= 5.5 / <= 3.9 | mermaid's + 20 %: flowmaid's nodes are text-sized, so the same pad weighs more against a smaller node area |
| `span_max` | 0.767 | 0.308 | 0.314 | <= 0.37 | mermaid's + 20 %: flowmaid's canvas is shorter along the flow, so the same block takes a larger share |
| `crossings` | 24 | 16 | 20 | <= 20 | mermaid's + 4; target <= 16 |
| `bends` / `bends_max` | 109 / 8 | 100 / 7 | 141 / 18 | <= 120 / <= 8 | mermaid's total + 20 %; 0.38's own worst edge |
| `detour_max` | 1.78 | 1.36 | 3.61 | <= 1.63 | mermaid's + 20 % |
| `lbl_struck` | 0 | 0 | 3 | 0 | none |
| `lbl_crowd`, `lbl_gap_min` | 8, 2.5 | 0, 20 | 9, 0 | 0, >= 12 | mermaid's count of crowded chips |
| `lbl_in_end`, `lbl_own_gap_max` | 7, 0 | 0, 0 | 3, 16.5 | 0, <= 1 px | none |
| `lbl_off_mean` / `lbl_off_max` | 0.198 / 0.332 | 0.130 / 0.329 | 0.140 / 0.364 | <= 0.20 / <= 0.35 | target <= 0.13 |
| `lbl_endpx_mean` | 309 | 558 | 332 | <= 386 | the smaller of mermaid's and 1.25 x 0.38's, the rule of the area row; target <= 340 |
| `lane_gap_min` | 9.3 | 12.0 | 12.4 | >= 9.3 | 0.38's: the corridor the complaint names may not get tighter; target >= 12 (mermaid's) |
| `straight_max` | 1175 | 1236 | 1112 | <= 1240 | mermaid's |
| `area` | 3.41 M | 5.18 M | 3.26 M | <= 4.27 M | the smaller of mermaid's and 1.25 x 0.38's: the compact canvas stays |
| `edge_len` | 22350 | 30835 | 20488 | <= mermaid's | |
| `lbl_lbl`, `lbl_node`, `frame_overlap`, `frame_foreign_nodes`, `node_hits`, `warn` | 0 | 0 | 0 | 0 | |

On `lbl_endpx_mean`: the value is the distance to the nearer end, about (0.5 - `lbl_off_mean`) x the mean length of a
labelled edge. G3 puts labels at the middle and G1, G3 and F2 lengthen exactly the labelled edges between blocks; the
experiment's 332 px was measured with the band gaps crushed (frame gaps of 75 and 83 px for about ten tracks and their
labels). A must of 340 would leave 8 px for all of that on a drawing that still beats mermaid by a third.

Harness `lat_backtrack` is reported per input, static and routed, and **not gated on the block inputs**: a corridor
beside a block is sideways travel by design. By the same measure mermaid's drawings have about 1160 px on the analog
and 2030 px on v2; 0.38 has 364 and 257, with three runs through foreign frames. The edge-shape gate is `bends_max`
and `detour_max`.

**(b) v2 and the five permutations (local inputs).** mermaid on v2: crossings 13, `compact` 4.03 / 3.14, `span_max`
0.233, `lbl_crowd` 1, `sib_stagger` 1, area 5.37 M, `bends` 95, `bends_max` 6, `detour_max` 1.47.

- v2: the zeros of (a) (`sib_stagger` 0, better than mermaid's 1; `lateral_x`: 0.38 24, experiment 11);
  `compact_max` <= 5.5 and `compact_mean` <= 3.9: the analog's bound, because the frame that sets it (PROD) is the
  same block with the same members in both inputs (experiment 5.72 and 5.74), which against mermaid's v2 values is +36
  % and +24 %; `span_max` <= 0.28; crossings <= 17 (mermaid's + 4; 0.38: 19; experiment: 20); `bends` <= 114,
  `bends_max` <= 8, `detour_max` <= 1.76 (`bends` and `detour_max`: mermaid's + 20 %; `bends_max`: the analog's
  bound; experiment 127 / 15 / 1.55); `lbl_struck` 0; `lbl_crowd`
  <= 1; `lbl_in_end` 0; `lbl_own_gap_max` <= 1 px; `lbl_endpx_mean` <= 329 (the smaller of mermaid's 496 and 1.25 x
  0.38's 263; target <= 290; the experiment's 223 was measured with its labels off the middle); `lane_gap_min` >= 8;
  area <= 3.21 M (1.25 x 0.38's); edge length <= mermaid's.
- each permutation: the zeros; `compact_max` <= 5.5; `span_max` <= 0.37; `lbl_struck` 0; `lbl_crowd` 0; `lbl_in_end`
  0; `bends_max` <= 8; `lane_gap_min` >= 8 (`EDGE_SEP_MIN`; experiment 1.6 on perm-a, 2.9 on perm-d); crossings <= 20
  on perm-a, perm-c and perm-d, <= 23 on perm-b, <= 22 on perm-e. Perm-a and perm-d are the analog's graph with the
  same node declarations and only the edge statements moved, so by 2.4 they get the analog's layer orders and its
  must; perm-c, perm-b and perm-e take mermaid's count + 4 (mermaid: 7 19 16 19 18; experiment: 22 25 20 22 22; 0.38:
  12 19 24 24 34). A gate at 24, 0.38's count on the analog, is the gate the user's decision rejects. **The same band
  order as the analog** (`bands`, rule `same_as`; mermaid scrambles it on the first) and a canvas breadth within 10 %
  of the analog's (rule `within`).

**(c) The public fixture (in the repository) and its permutations**, against mermaid: the zeros of (a);
`compact_max` <= 7.97 and `compact_mean` <= 4.63 (mermaid's 6.64 / 3.86 + 20 %); `span_max` <= 0.22 (0.183 + 20 %);
crossings <= 7 (mermaid's 5 + 2, the experiment's count; target <= 5; 0.38: 14; the analog's margin of 4 would be +80
% here, on the diagram that becomes the README's comparison picture); `bends` <= 110, `bends_max` <= 8, `detour_max`
<= 1.43 (mermaid 92 / 6 / 1.19; experiment 128 / 14 / 1.27); `lbl_struck` 0; `lbl_crowd` 0; `lbl_in_end` 0;
`lbl_own_gap_max` <= 1 px; `lbl_endpx_mean` <= 295 (1.1 x 0.38's 268; experiment 249); `lane_gap_min` >= 8; area <=
mermaid's 2.78 M (experiment: 1.65 M); edge length <= mermaid's 20125; the same band order and a canvas breadth within
10 % across its four permutations. In `tests/quality.rs` these rows are asserted through `compact_max`,
`compact_mean`, `span_max`, `sib_side_linked`, `sib_stagger`, `lateral_x`, `edge_frame_x` 0, `x_drawn`, `bends_max`,
`detour_max`, `lbl_edge` 0, `chip_crowd` 0, `lbl_end_mean`, width x height and `edge_len` (the names of 3.1), so the
gate lives in the repository.

**(d) New measures**, static and routed at the scene's centres.

- `lateral_x`: 0, static and routed, on the analog family, the public fixture, hub5 and cycle2; on bench-75, dense-lr and
  dense-td static not above mermaid's count (3 / 0 / 1; 0.38: 43 / 51 / 93), with 0 as the target (2.8 aims at it;
  the README sentence of 4.7 says "none" only where 0 is measured), routed there not above the reference.
  On the rows-equal inputs the rule of (e). On a level L4 dissolved, and for the surplus ends of an over-full side
  (2.8), not above the reference, each such edge named in the report.
- `side_ends`: 0 static on the analog family, the public fixture, hub5 and cycle2 (mermaid: 0 of 51 / 51 / 54 ends on
  the analog / v2 / public fixture; 0.38: 3 / 4 / 3; experiment: 6 / 11 / 2); elsewhere, and routed, not above the
  reference.
- `up_lanes`: 0 on every input where no flow-facing side is over-full at `PORT_MIN_SPACING`; elsewhere not above the
  number of surplus ends.
- `fan_x`: 0 static on the analog family, the public fixture with its permutations, and hub5; on every other
  block-mode fixture, and routed, not above the starting release's per-fixture value (B11 leaves a remainder on the
  dense fixtures' over-full sides and says why). `fan_uneven`: not above the starting release's per fixture.
- `spine_off`: 0, static and routed, on the inputs of B12.8 (analog, v2, public fixture, hub5, spread-face,
  nested-composite, fork-twins); elsewhere not above the reference. Displaced entries are not counted (`title-entry`
  has 2), nor is a border spine X3's feasible interval forbids; both are listed in the report.
- `pin_corner`: 0, static and routed, on every block-mode fixture; the one exception is a frame side whose free
  stretch cannot hold its pins at `PORT_SPACING`, each such side listed in the report.
- `title_cut`: 0 on `title-entry`, `title-hidden` and `lr` in all four directions; not above the reference on the
  other fixtures.
- `fr_max`, `ends_apart`, `dfr_own`, `dfr_far`: per B12.13.

**(e) Rows-equal inputs: not worse than the reference, and as tidy as mermaid where 0.38 was not.** For
`examples/{advanced, subgraph}.mmd`, `quality/fanout-frames.mmd`, `notes/{near-drag, spread-bottom, straight-link,
wide-frames}.mmd`, the probes p03-p08 and p10-p12, hub5, cycle2, state-note, every other fixture whose block layers
equal its starting-release layers (B12.3 prints the list), and the fixtures with a level L4 dissolves (`swimlane`,
`swim-cycle`, `swimlane-nested`): per input and mode the harness `defects`,
`x_drawn`, `edge_frame_pairs`, `overlap_pairs` not above the reference, hard zeros 0 (these rows are asserted in
`tests/quality.rs` by `BLOCK_CEILINGS`, 4.5); block-metrics crossings, `xframe_runs`, `sib_side_linked`,
`sib_stagger`, `lbl_struck`, `lbl_crowd` not above the reference; `compact_max` <= the larger of the reference + 0.05
and 1.2 x mermaid's (a frame may grow by its jog room: hub5 0.38 3.73, mermaid 3.97); area <= the smaller of
mermaid's and 1.25 x the reference; edge length <= 1.1 x the reference; `lateral_x` not above the reference and not
above mermaid's measured count from `ref-mermaid.tsv` (0 on hub5, cycle2, p03, p06, p07, where 0.38 has 5, 6, 1, 2,
1; 1 on `notes/spread-bottom`, a note link); and a picture triple (0.38, B12, mermaid) of each reviewed and attached
to checkpoint C's report. **Stop rule of checkpoint C.**

**(f) "No worse" on the dense corpus.**

- `judge` on bench-75, dense-lr, dense-td: the zero rules; `xframe_runs` 0, `sib_side_linked` 0, `sib_stagger` 0 (L5,
  L6 and X4 give them on every level L4 does not dissolve; mermaid: 0 on all three; 0.38: 4 / 5 / 3 and 2 / 4 / 3);
  `lbl_struck`, `lbl_crowd`, `compact_max` not above the reference; crossings per file <= the smaller of 1.1 x
  0.38's + 2 (bench-75 334, dense-td 90, dense-lr 52) and 1.1 x the starting release's + 2 (B12.0 writes the
  numbers), **and** their sum over `tests/fixtures/quality` (harness `x_drawn`, static | routed) not above
  `DEFECT_BASELINE` as the starting release has it. The two rules count the same thing (block-metrics crossings equal
  the harness's `x_drawn` on these inputs) and both are judged at A1, so the stricter one is never first met at the
  last item. Area <= mermaid's; edge length <= the larger of mermaid's and 1.1 x the reference.
- flowmaid's harness (`examples/quality.rs`, `tests/quality.rs`): every hard zero 0 in both modes (`edge_node_x`,
  `shared_ports`, `arrow_on_start`, `wiggles`, `frame_side`, `node_overlap`, `node_frame`, `frame_overlap`) and
  `own_frame_x` 0; per fixture and mode `defects`, `edge_frame_pairs`, `overlap_pairs` not above the reference;
  `min_edge_gap` not below `GAP_FLOORS`; the fields of `DEFECT_BASELINE` by the rule of 4.5.
- The 49 inputs of `B/metrics/extra/in`: runs through a foreign frame 0 on every block-mode input (mermaid: 0 on all 36
  inputs with frames; 0.38: 45 runs on 10), `warn` 0.
- All flat inputs and the clustered inputs not in block mode: byte-identical (the permanent hash test).

**(g) Invariants**: guard green; determinism; the scene contract test
(`scene_carries_stable_identity_and_stays_index_parallel`) on the block fixtures; hit-testing, fold, `to_mermaid` round
trip on the block fixtures in all four directions.

**(h) Routed at the scene's centres** (what the playground paints before the first drag), block-metrics on
`<name>.routed.svg` for the analog, v2, each permutation and the public fixture: `xframe_runs` 0; `lateral_x` 0;
`lbl_struck` 0; `lbl_crowd` not above the reference's routed value; crossings not above the reference's routed count
and not above the static must + 4 (analog <= 24, v2 <= 21, public fixture <= 11; 0.38 routed: 27 and 21 on the analog and v2;
experiment without corridors: 28 and 24, with 3 struck and 7 crowded chips on the analog). The routed picture of each
goes into the reports of checkpoints R and C beside the static one.

### 4.4 Tests to add (names are suggestions; one per rule; every one runs on repository fixtures only)

| test | asserts |
|---|---|
| `everything_renders_as_before_with_blocks_off`, later `flat_and_unlinked_inputs_render_as_before` | B12.0 hashes: every fixture with the switch off; after B12.16 the flat and not-block-mode rows |
| `the_switch_changes_nothing_outside_block_mode` (until B12.16) | B12.1, B12.6: fixtures and generated graphs outside block mode are the same with the switch on |
| `block_mode_is_decided_by_border_links`, `block_mode_agrees_between_scene_and_route`, `an_own_box_edge_never_decides_block_mode` | B12.1 lists; the three entry points' recorded values; `S --> S` and `OUT --> IN` in both member orders |
| `a_subgraph_takes_consecutive_rows`, `linked_blocks_never_share_a_row`, `sibling_blocks_never_interleave` | L5, L6 properties on generated graphs, on `interleave` (T 1-3, S 4-5) and on `l5loop` (S 0-1, T 2-5, B 2-4; the pass bound) |
| `block_order_follows_declarations_until_it_costs_twice_the_minimum` | L3 on the public fixture (walk kept), dense-td (net), `ratio2`, `ratio3` (declaration order), `ratio4` (flipped), `ratio2x4` (each pair by its own count), `cycle2` (rank's marks are links of the level: L0 decides) |
| `block_order_survives_reordered_statements`, `block_order_follows_first_mention` | same bands on the public fixture's permutations, and their canvas breadth within 10 % of the fixture's; the `cycle2-tie` pair documents the first-mention rule, with exactly one upstream edge in each file (L0 drops the inner mark) |
| `a_ping_pong_level_stays_lanes` | L4: `swimlane` (0 turned, frames side by side, not worse than the starting release), `swim-cycle`, `swimlane-nested` (the grandchild's cycle broken by L3); and stacked, not lanes: `reqresp`, `reqresp22` (Front first, 1 and 2 upstream edges), the `cycle2-tie` pair, `swim-nested`, `p06-block-cycle` |
| `a_source_block_sits_beside_what_it_feeds` | the build pipeline beside the shared services |
| `a_note_card_keeps_its_row_in_its_block` | L2: `near-drag`, `note-edge-row` (a card after the last row, a lead card before the first) |
| `a_turned_edge_runs_up_its_channel_through_flow_sides`, `a_turned_edge_one_layer_up_is_a_z_not_a_lane` | E3: out of the top, into the bottom (TD), all four directions; the adjacent-layer case |
| `a_chain_out_of_a_nested_block_leaves_through_its_downstream_side` | O0's deeper-fit rule on `nested-chain`: 0 lateral crossings |
| `chains_keep_one_side_of_a_block` | O2's constraints; no run through a foreign frame, static, on every block fixture |
| `level_order_is_deterministic_and_bounded`, `the_start_order_ignores_edge_statement_order`, `a_narrowing_candidate_starts_from_the_winning_order` | O4: same SVG twice, start count under the budget; shuffled edge lines give the same layer orders; narrowing still accepts a candidate on dense-td and dense-lr |
| `crossing_neutral_members_keep_declaration_order` | O4b |
| `a_hub_fan_leaves_in_target_order` | O6, with B11's comb, scene and route |
| `a_spine_inside_a_block_is_straight`, `a_spine_across_a_border_is_straight`, `a_compact_block_keeps_its_brandes_koepf_columns` | X1-X3 |
| `sibling_frames_of_a_band_start_level_in_scene_and_route` | X4: `sib_stagger` 0; the same frames from `route()` at the scene's centres |
| `a_band_gap_holds_its_tracks_and_labels_between_the_frames`, `linked_frames_are_never_closer_than_route_calls_stacked` | G1 and its floors (the `t9` fixtures) |
| `a_chain_label_sits_outside_both_end_blocks`, `a_label_sits_on_its_own_line`, `a_label_slot_is_clear_of_foreign_lines` | G3 |
| `a_frame_is_as_broad_as_its_title`, `an_lr_frame_is_as_long_as_its_title` | F1 on `title-hidden`, TD and BT; F2 on `lr`, LR and RL |
| `jog_room_counts_only_what_crosses_the_title_side` | F2: a title-row hub whose exits leave downstream reserves none |
| `frames_agree_between_scene_and_route`, `a_far_end_drag_never_changes_a_frame_it_does_not_enter`, `a_redrawn_frame_covers_the_edges_that_did_not_move` | F1-F3, R6 (B12.13): on a drag an end block's frame changes only by the redrawn edge's own inner leg and by one jog track when the link's class flips; no other frame moves; a kept edge stays inside its frame; a node dragged away from above a block with displaced entries leaves that frame equal in `route_partial()` and `route()` |
| `an_entry_under_a_title_jogs_in_beside_the_chip_inside_the_frame` | 2.8, TD and BT, scene and route, after the comb: `title-entry`, its chain variant, `title-hidden` |
| `an_edge_to_a_block_ends_mid_side_clear_of_the_chip_and_the_corners` | free pins: alone, an even band, a side split by member entries |
| `border_ends_spill_last` | 2.8 on an over-full side |
| `a_subgraph_self_edge_is_a_loop_in_scene_and_route`, `a_stub_to_an_own_member_passes_no_member`, `a_stub_to_an_own_nested_subgraph_passes_no_member` | 2.8, with and without block mode (B12.14) |
| `route_takes_a_corridor_round_a_foreign_block`, `a_corridor_passes_two_staggered_bands`, `a_corridor_reaches_a_second_row_target`, `corridors_nest_and_do_not_jump_on_a_one_px_drag`, `a_nested_edge_crosses_each_frame_on_a_flow_side_in_route` | R3, R5 |
| `corridor_slots_do_not_follow_statement_order` | R3: routed `x_drawn` and `fan_x` equal across the public fixture's permutations |
| `a_corridor_is_rounded_where_ortho_does_not_run` | R3 on the `linkStyle default stroke-width` fixture, static and routed |
| `a_node_dragged_onto_a_foreign_frame_is_still_routed` | R4 |
| `route_and_scene_agree_on_which_edges_run_upstream` | R2, the `t9` pair and its two-way variant included |
| `dragging_every_node_of_a_block_diagram_keeps_the_contract` | B12.13 |
| `a_direction_block_inside_a_bordered_block_keeps_its_direction` | B9 inside block mode (`b9-inside`, `b9-inside2`) |
| `every_block_fixture_has_a_ceiling` | every file of `tests/fixtures/blocks` and `tests/fixtures/notes` and the two examples have a row in `BLOCK_CEILINGS` |
| `layout_clustered_returns_blocks` | public API doc example |

### 4.5 Pinned tests, gates and images: what may change and why

Every number of `tests/quality.rs` quoted in this spec is 0.38.0's and only shows the size. The values that hold are
the starting release's, recorded in B12.0.

**Must pass as written**, in the suite-on run (a failure is a regression, not a re-baseline): every test of
`compound.rs`; `assert_clean_clusters` users; the own-frame tests; the tests on `examples/advanced.mmd`
(`render::advanced_showcase_parses_and_renders`, `scene::a_two_cycle_runs_side_by_side_across_its_gap`,
`scene::a_two_cycle_into_a_diamond_keeps_its_twin_ends_apart`,
`scene::a_run_into_a_box_crosses_its_border_clear_of_the_run`, `ports::a_nearly_straight_pair_is_drawn_straight`: the
diagram is now drawn in block mode and the rules they state still hold);
`a_composite_edge_to_its_own_state_drops_straight_from_the_border`;
`note_links_hold_their_ends_and_corners_in_scene_route_and_drag`; `a_static_canvas_is_its_drawing_with_even_margins`;
`long_titles_are_cut_only_where_they_would_run_into_a_sibling`;
`a_hub_fed_row_of_shallow_frames_wraps_into_two_rows`; `hub_fed_leaves_in_a_box_pack_into_a_grid`;
`largest_box_is_centred_among_its_siblings`; `route_partial_*`;
`marquee_selects_an_edge_once_its_painted_stroke_touches`; `moving_a_node_declaration_barely_moves_dense_td`;
`edge_line_order_barely_moves_the_dense_layouts` (the canonical order of 2.4 is what keeps it);
`every_fixture_has_a_ceiling`; `hops::a_pick_finds_the_painted_hops_on_every_fixture`; and every test B11 and the
leftovers added that B12.0 does not list under the intended changes.

**Intended changes** (say each in its commit message):

| test or gate | change | why |
|---|---|---|
| `layout::a_box_seats_its_loose_parts_on_its_top_layer` | restated | it pins a `seat_box_sources` case in which a loose member sits level with the hub that feeds its box through another member; a block lies wholly below its feeder |
| `comb::a_hubs_fan_is_a_comb_in_scene_and_route` (B11) | re-read | the rule (a hub's fan is a comb with no crossing inside it, in scene and route) must hold on the re-laid dense-td and fanout-frames; the side and the number of ends it names are fixture-specific (turned edges may now leave the hub by its upstream side) |
| `edge_picking_matches_the_painted_strokes_on_every_fixture`, the determinism checks | `tests/fixtures/blocks` added to their directory lists | they read `tests/fixtures/quality` and `examples` only; the rule is unchanged |
| `CEILINGS` rows of bench-75, dense-lr, dense-td, fanout-frames | re-measured, ratchet **down** | edge / foreign-frame pairs fall (static 43 over the three dense fixtures in 0.38 -> at most 4); `defects` may not rise |
| new table `BLOCK_CEILINGS`, next to `CEILINGS` | rows for `examples/advanced.mmd`, `examples/subgraph.mmd`, the eight notes fixtures and every fixture of `tests/fixtures/blocks`, per mode | none of them has a per-fixture row today (`CEILINGS` holds the six files of `tests/fixtures/quality`), so "not drawn worse" would live in the workspace only. `defects`, `edge_frame_pairs`, `overlap_pairs`: the starting release's values, set in B12.0, lowered only. `x_drawn`: the starting release's value for the rows-equal list of B12.3 (4.3 e); for the other block fixtures the larger of that value and 1.1 x it + 2, or the fixture's own row of 4.3 (c) |
| `DEFECT_BASELINE` | re-measured against the starting release's values | the dense fixtures are re-laid. `x_drawn`, `lat_wiggles`, `end_tilt*`, `own_frame_*`, `fan_x`, `fan_uneven` and every other field not named next: at or below the starting release; `x_drawn` is the crossing must of 4.3 (f). `band_pairs`, `band_len` and `lat_backtrack` may rise where the cause is a corridor (parallel chains a `TRACK_PITCH` apart; an edge that goes round a block the starting release ran it through), with the edges named and a picture. The static column is judged at checkpoint A2 and the routed column at R, each with the checkpoint's stop rule; B12.16 only writes down numbers already accepted |
| `BORDER_LEN_MAX`, `CHANNEL_MAX`, `EDGE_CHIP_MAX`, `XGAP_MAX`, `LBL_EDGE_MAX` | re-measured; a higher value needs a named cause and a picture | positions change; the prototypes' routed values (border length 4530, chips 8) were `route()` without corridors, i.e. regressions |
| `GAP_FLOORS`, `DRAG_JUMPS_MAX`, `INK_MEAN_MIN`, `MAX_ASPECT`, `SHUFFLE_WIDTH_MAX`, hard zeros | **not loosened** | they are the invariants |
| `casing::a_short_line_keeps_at_most_one_gap`, `casing::a_line_takes_one_role_across_a_cluster_of_crossings`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route` | re-read; update the fixture-specific numbers if the rule each states still holds | they count crossings or measure a leg on a dense fixture that is re-laid |
| new gates | block shape per fixture (`compact_max`, `span_max`, `sib_side_linked`, `sib_stagger`), `lateral_x`, `side_ends`, `up_lanes`, `spine_off`, `pin_corner`, `title_cut`, `fr_max`, `ends_apart`, `dfr_own`, `dfr_far`, the public fixture's rows of 4.3 (c) by the names of 3.1 | no gate sees these today |

**README images.** `docs/img/compare/state.png` does not change. `docs/img/compare/{advanced, subgraph}.png` and
`docs/ex-subgraph.svg` are re-rendered if their bytes change, each with its before and after in the commit (4.3 e).
Add one comparison image for a block diagram: the public fixture, never the analog. Re-render the benchmark renders of
bench-75, dense-lr, dense-td.

### 4.6 Perf budgets (PLAN invariant 1; `tools/perf.sh`, two runs before calling a regression)

| file | `scene_ms` limit | `route_ms` limit | worst `drag_ms` limit | worst `route_partial` frame |
|---|---|---|---|---|
| advanced | 1.42 | 1.72 | 2.61 | |
| subgraph | 0.38 (D1) | 1.05 | 2.04 | |
| fanout-frames | 4.38 | 1.34 | 3.44 | |
| dense-lr | 46.8 | 8.03 | 17.76 | < 6 ms |
| dense-td | 50.68 | 9.20 | 17.80 | < 6 ms |
| bench-75 | 61.94 | 15.76 | 25.28 | < 6 ms |
| analog (local) | 13.2 | 2.9 | 7.25 | < 6 ms |
| public fixture | 2 x baseline | by the rule below | by the rule below | < 6 ms |

`scene_ms`: 2 x baseline; for a file whose baseline is under 1 ms, max(2 x baseline, baseline + 0.3 ms) (decision D1).
`route_ms` and `drag_ms`: max(1.25 x baseline, baseline + 1.0 ms). The flat corpus files keep their numbers. Every
file of the table is drawn in block mode.

Baselines. The six corpus rows: `runs/baseline/perf.tsv` (PLAN). The analog's row is computed from 0.38.0's 6.6 / 1.9
/ 5.8 ms (`B/u-flowmaid/out/infra.quality.tsv`); other 0.38-path runs in the workspace put its `route_ms` between 1.9
and 3.2 ms, on both sides of the limit, so one run cannot carry a stop rule: B12.0 measures the analog and the public
fixture twice with `perf.sh` on the starting release, the baseline is the lower of the two runs, and the analog's
limits above are replaced by the limits computed from it when those are lower.

`route_partial`: under 6 ms a frame on every file (PLAN invariant 1; the limit is not raised by this spec). `perf.sh`
does not measure it: its `drag_ms`, `drag_mean` and `drag_over` time a full `route()` per drag frame. The number is
the worst frame over `quality::drag_frames`, read from the "route_partial worst" line that `cargo test --release
--lib -- --ignored --test-threads=1 fixture_drag_perf_guard` prints per fixture (the test itself asserts 8 ms, so the
6 ms is read off the line). B12.0 adds the public fixture to that test's list, runs the same loop for the analog from
a local example, and records the worst frame per file on two runs of the starting release. The README publishes "about
3 ms, 6 ms at worst" for bench-75 on 0.38.0: if the starting release is itself at or above 6 ms on a file on both
runs, B12.0 reports that before any B12 work.

### 4.7 Docs

- **README**: Subgraphs section (0.38: lines 282-289): the block rule in three sentences (consecutive rows; linked
  blocks stack along the flow; edges between blocks run in the gaps and cross frames on flow-facing sides), each said
  of "a diagram in which an edge joins a subgraph to something outside it"; the `direction` rule next to it
  (unchanged); the lane rule (a level stays side by side when every stacking order would turn more than a third of
  its links and a path enters one of its blocks twice); `flowchart-elk` (accepted, same as the default). How it works
  (797-806): Layers, Ordering, Coordinates, Narrowing. Guarantees (811-817): add "in a diagram laid out as blocks, no
  edge runs through a block that holds neither end (static)", worded for diagrams drawn in block mode and not for a
  level the lane rule leaves side by side. Benchmark rows of the clustered inputs (825-851) and the drag numbers (857),
  re-measured.
- **CHANGELOG** under `## [Unreleased]`: Changed (subgraphs are laid out as blocks in every flowchart and state
  diagram with an edge across a subgraph border; upstream edges between blocks run in channels; a subgraph's title is
  never cut while its frame can grow; `layout_clustered` returns block layouts), Fixed (a subgraph self-edge is drawn
  as a loop and no longer ranks its members; an edge from a subgraph to its own member or nested subgraph no longer
  runs through a member; a dragged block's frame keeps the room of the edges that did not move: all three in every
  diagram with subgraphs), with the public fixture's numbers. The frame sentence claims what B12.13's gates prove and
  no more: if a fixture stays above 0.5 px of `fr_max`, the sentence names the exception.
- Doc comments on every new `pub(crate)` item that states a rule, in the code base's style (why, constants by name).
- B12a adds no README text, no image and no release note: nothing it builds is visible.

### 4.8 Reports and the local acceptance

Each checkpoint ends with `B/reports/checkpoint-<S|A1|A2|R|C>.md`, written in the workspace and never committed:

1. the commit it ran on, the command lines, and the `judge` output for the local inputs and for the repository
   fixtures (static; from R on also routed);
2. the tables of 4.3 (a), (b), (c) and, from R on, (h), with the measured value beside each must, and the rows of (d)
   for the analog family;
3. the suite-on list of failing tests against 4.5, and the result of the default run (guard, off-identity test);
4. the two `perf.sh` tables and the `route_partial` lines against 4.6;
5. per checkpoint: A1 the table crossings x starts and the levels that fire L4; A2 the edges and sides the rules of
   4.3 (d) ask to be named; R the fixtures above 0.5 px of `fr_max` with value and cause; C the picture triples of
   4.3 (e) and, for the analog, v2 and the public fixture, 0.38 | B12 static | B12 routed | mermaid;
6. every must that failed and every stop rule that fired, with the numbers.

The local acceptance (analog family) is run from the workspace by the implementer and re-run by the verifier from the
same commit with the commands of 4.3. Its numbers and pictures live in these reports and nowhere else; the repository
gets only the sentence "local acceptance inputs: all musts met" in the item's commit message.

## 5. Out of scope, and the two packages

Out of scope for B12:

- Generalising `compound.rs` to bordered subgraphs (`d-recursive`): per-block pipelines, pins, stitching.
- Honouring `direction` in a subgraph that has a border edge (mermaid ignores it too; B9's rule stays).
- Stacking the blocks of a level the lane rule leaves side by side (mermaid stacks them and turns two arrows of five).
- Aligning a short block inside a tall band by its pins (mermaid's R5 beyond "sources sit last").
- One lane per edge at 20 px pitch (mermaid pays 5.18 Mpx2 for it), and mermaid's 16 crossings on the analog as a
  must (it is the target; the must is 20).
- Styling subgraphs (`style`, `class` on a subgraph are still dropped), class and ER diagrams, the `advance` module.

**Two packages (decision D2).** The user-visible contract of D2 is unchanged: nothing visible until one release,
0.40.0. What moved against the first cut is where the edge work sits: flow-facing ends, free pins, title-clear entries
and label room are built in B12a, because B12a's frames (F2), coordinates (X2.2) and corridors (R3) are built on them
and `route()` would otherwise be reopened after its checkpoint. B12b keeps the last edge kinds, the acceptance and the
flip.

**B12a "blocks: structure, geometry and routing, behind the switch"** = B12.0-B12.13, in three parts, each ending at
a checkpoint and each merged to `dev` with the switch off:

| part | items | checkpoints |
|---|---|---|
| 1 structure | B12.0 baselines, B12.1 level tree and switch, B12.2 corridor spike, B12.3 layering, B12.4 chains, B12.5 ordering, B12.6 sweep invariants | S (after B12.2), A1 |
| 2 geometry | B12.7 frame function F1 + F2, B12.8 coordinates, B12.9 gap room and labels, B12.10 one frame function, B12.11 flow-facing ends, pins and title entries (static), B12.12 narrowing | A2 |
| 3 route | B12.13 `route()`, `route_sized()`, `route_partial()` with the routed ends | R |

*B12a's acceptance, which a verifier runs on the package's last commit:*

1. **Default run: nothing changed.** `tools/guard.sh` green; `everything_renders_as_before_with_blocks_off` green
   (every fixture, `scene()`, `route()`, `route_partial()`); two `perf.sh` runs show no file slower than the starting
   release beyond the run-to-run spread; no README text, image, CHANGELOG release note or version changed; the switch
   is off by default and no public item sets it except the `doc(hidden)` `quality::with_blocks`.
2. **Suite-on run: the block path is right as far as it is built.** The gates of checkpoints A1, A2 and R (4.1)
   re-run from the commit: `cargo test` with the failing tests a subset of 4.5's intended changes; `judge` with 0
   must-fails on the static rows of 4.3 (a)-(d), (f), (g) and on the routed rows of (d), (f), (h), for the repository
   fixtures and for the local inputs; the budgets of 4.6 on two runs.
3. The reports of S, A1, A2 and R exist with the content of 4.8, and no stop rule is open.

B12a ends with **no release**. It must not ship as the default: the own-box edges, the state-diagram review, the
rows-equal gate (4.3 e) and the comparison sweep are not done.

**B12b "blocks: remaining edges, acceptance, default on"** = B12.14 state diagrams and own-box edges, B12.15
comparison sweep, B12.16 default on, ratchets, docs, release 0.40.0; checkpoint C between B12.15 and B12.16.

*B12b's acceptance:* before the flip, checkpoint C in the suite-on run (`judge` 0 must-fails static and routed
against complete reference tables, every rule of 4.3 including (e) with its picture triples, the suite-on failing
tests exactly 4.5's intended changes). After the flip, in the default build: guard green with the ratchets of 4.5,
everything in 4.3-4.6 again, the permanent hash test, the docs of 4.7, and no trace of the switch. The stop rules of
checkpoint C apply; the default is never flipped with a must failing.

Reason for the split: 9 of 1047 library tests and 7 of 13 quality tests fail on the best prototype and nearly every
ratchet moves; one review cannot tell an intended re-baseline from a regression unless there is a point where nothing
visible has changed yet. Reason for the parts: almost all of B12a is, in the words of section 0, not built by anyone
yet, so each checkpoint is one implement-verify-fix cycle with its own stop rule, and no item sits after a gate it
would invalidate (narrowing and the constants are settled before A2; the entries that F2, X2.2 and R3 reserve room
for exist before R).

## 6. The judges' blocking flaws and where each is resolved

| # | flaw (judges) | resolution |
|---|---|---|
| 1 | crossings rise; the block transpose is unprototyped (all three) | transpose tested and rejected; 2.4 level-first ordering with starts and chains, evidence in appendix A (analog 20 from the fifth start); B12.5 with the starts table, a time gate and a stop rule |
| 2 | edges enter frames through lateral walls and run along title strips; rule 1.6 cannot work for a first-row target; "ports" was the wrong stage (visual, engineering) | 2.8: sides set by the builders, title-clear ports, displaced entries with jog room inside the frame (2.7 F2), X2.2 breadth; gates `lateral_x`, `side_ends`; B12.11, B12.13 |
| 3 | labels and gap room; struck labels on v2 and the permutations; room measured row to slot (all three) | 2.6 G1-G3; label rules gated on v2, the permutations and the public fixture, static and routed; B12.9 |
| 4 | own-frame containment breaks inside blocks (visual) | X2 keeps inner runs between the members where it can; F3 grows the frame over the rest, as 0.38 does; the four tests unchanged plus spread-face and nested-cases; B12.8 |
| 5 | straight spines lost; packing discards Brandes-Köpf; alignment rule dead (visual, engineering) | 2.5 X1, X2.3, X2.4, X3 (keep when compact, seeded squeeze, rigid spines, border spines); gates `spine_off`, `bends_max`, `detour_max`; B12.8 |
| 6 | block order without a safety net, 21 of 47 edges upstream on dense-td; no bound (all three) | L3: two walks, exact minimum per component beyond `2 * M_c + 1`; B12.3 check |
| 7 | no rule for ping-pong levels (visual, invariants) | L4 with its two tests; fixtures on both sides of it; constant from the sweep; decision D4 |
| 8 | partially overlapping unlinked siblings (engineering) | L5 no interleaving, in a loop with the source rule, with a pass bound |
| 9 | `route()` and drag: budgets, drag jumps, routed gaps, corridors unbuilt and root-only, scene and route disagree on more edges (all three) | 2.9 R1-R8, per level, one breadth per band; a spike before the structure work (B12.2) and the route item with its own checkpoint and stop rule; `ends_apart`; 4.3 (h) |
| 10 | upstream lanes run round whole bands (engineering, visual) | E3: channels through flow-facing sides in scene and route, the adjacent-layer case included |
| 11 | state composites snap more on the first drag; frames differ between scene and route (invariants) | F1-F3: one frame function, a frame a function of its own members and its border links' classes; R6's frame rule; gates `fr_max`, `dfr_own`, `dfr_far` |
| 12 | diagrams that need no change move; state fixtures; static time on tiny files (engineering, invariants) | block mode by structure (2.2): a diagram without a border link is byte-identical; the rows-equal inputs are gated one by one (4.3 e, `BLOCK_CEILINGS`); D1 |
| 13 | the layout search goes idle or changes regime; `layout_clustered` changes silently (engineering) | L7, O4's warm start of later placements, O5, B12.12; doc comment and CHANGELOG |
| 14 | state diagrams under-specified; the sink rule for lead notes untested (engineering, invariants) | L2 (a card is a member and takes a row; lead cards per level); B12.14 review round with named tests and new fixtures |
| 15 | which pinned tests are re-baselined and which are regressions (all three) | 4.5; the suite-on run of 4.1 |
| 16 | block shape leftovers: frame tops 17 px apart, PROD too wide, dense-lr L-shaped (visual) | X4; X2; rows wrapped inside a block (B12.3) and narrowing on block layouts (B12.12) |
| 17 | cannot be verified as one step (engineering) | 4.1 checkpoints; section 5: two packages, B12a in three parts |
| 18 | routed canvas margin test fails unexplained; RL not run; B11's comb untested with blocks; prototype numbers of two modes mixed (engineering, invariants) | R7 and B12.13; all four directions in the fixtures; 3.3 (the comb runs in every number from B12.4 on); every number in this spec names its mode |
| 19 | shared-code guards; flat byte identity unpinned (invariants) | 3.4; the off-identity test and the permanent hash test of B12.0 |
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
   External services [DNS GW TG OAI AZ] (before O4b, which puts the edge-less GW first) left of Code hosting, all of
   DNS's chains left of PROD, STANDBY left of DEV/STAGING: mermaid's arrangement. Same band order under all five
   permutations.
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

### A.3 Measurements of the second review (scripts and outputs in `B/check-*`)

1. **There-and-back against ping-pong** (`B/check-fix-a`, `B/check-revise-2/l4u.py`): `reqresp` in 0.38 has Front 464
   px tall beside Back (`span_max` 0.892, `sib_side_linked` 1); mermaid and `d-hybrid`'s prototype stack the two
   (`span_max` 0.382 and 0.401). Among the fixtures and probes the third-of-the-links test, in written directions, passes on the
   `cycle2-tie` pair, `reqresp`, `reqresp22`, `p06-block-cycle`, `p14-empty-and-self` (inside one block),
   `swim-nested`, `swimlane`, `swim-cycle` and `swimlane-nested`; a path enters a block twice only in the last three.
2. **A raw mark inside a block** (`cycle2-tie`, `d-hybrid`'s dump): layers `b2 b1 / a1 / a2`, turned `a2 -> b1`: two
   edges of one cycle against the flow where mermaid and the experiment draw one.
3. **L5** (`B/check-verify-fix-b/py/l5v.py`, `l5w.py`): with rule 1 read on links only, `l5loop` never ends and 665 of
   40,000 random levels diverge; with rule 2's constraints included 0 of 40,000 fail and 7 passes is the most. The
   earliest-start pair rule gives fewer rows in 5075 of 60,000 random levels and more in 192.
4. **Per-component net** (`d-hybrid`'s prototype, level-wide net): `ratio2x4` flips both pairs, `ratio4mask` keeps a
   pair with four links upstream.
5. **Jog room** (`B/check-fix-a/jog.py` on the experiment's drawings): counted over every border link, External
   services grows 50 px (compactness 2.57 -> 3.59, the analog's `compact_mean` 3.56 -> 3.87 against the must of 3.9);
   the three links that cross its title side fit beside the chip.
6. **Cluster paths below the root** (`B/check-cold-read`): `apath-nested` has 1 lateral wall crossing in the
   experiment, `apath-root` 0.
7. **Frames 8 px apart** (`t9-frames-8px`, its LR copy, `B/check-verify-fix-a/in/s3.mmd`): two linked blocks with
   nested frames on their facing rows, in 0.38 and in the experiment; plain frames are 32 px apart.
8. **Own-box edges** (`B/check-fix-b`): `selfsub` 82x190 with the self-edge, 186x92 without; block mode by the letter
   of the first draft is off for `t2-self-nested` and on for `t2b-self-nested-order`, the same graph with one member
   declared earlier.
9. **Hop bridges** (`B/check-acceptance/tool/hoptest.js`): a single hop of radius 5 is bridged by block-metrics; B11's
   bridges over 2, 3 and 6 tracks are not (three straight runs, +5.7 px, two extra bends each). Crossings are counted
   correctly either way.
10. **Edge shape** (`bends.js`, `detour.js`), 0.38 / mermaid / experiment: `bends` 109 / 100 / 141 (analog), 124 / 95
    / 127 (v2), 89 / 92 / 128 (public fixture); most bends on one edge 8 / 7 / 18, 12 / 6 / 15, 12 / 6 / 14; longest
    detour 1.78 / 1.36 / 3.61, 1.28 / 1.47 / 1.55, 1.45 / 1.19 / 1.27. Sideways backtracking by the harness's measure
    on mermaid's own drawings: about 1160 px (analog) and 2030 px (v2); 0.38: 364 and 257.
11. **Lateral crossings in mermaid** (the measure on the common model): 0 on the analog family, the public fixture,
    hub5 and cycle2 (with 36 / 36 / 35 / 13 / 8 flow-side crossings, so not vacuous); bench-75 3 (real), dense-td 1 (a
    touch on a frame edge), `notes/nested-composite` and `notes/spread-bottom` 1 each (note links).
12. **Ends on a lateral node side** (`sideends.js`): mermaid 0 of 51 / 51 / 54; 0.38 3 / 4 / 3; experiment 6 / 11 / 2.
13. **Labels** (`extra.js`): chips inside an end block 7 / 0 / 3 on the analog (0.38 / mermaid / experiment), 7 / 0 /
    5 on v2, 7 / 0 / 1 on the public fixture; a chip 16.5 px off its own line in both experiment pictures.
14. **Routed at the scene's centres** (`B/spec/evidence/infra-overview.lvl-chains.routed.svg`): crossings 28 (static
    20), 3 struck and 7 crowded chips, 8 lateral crossings on 7 edges; harness routed `x_drawn` in 0.38: analog 27, v2
    21.
15. **Harness totals over the four dense fixtures**, 0.38 -> experiment: static `x_drawn` 428 -> 450, `band_pairs`
    120 -> 164, `lat_wiggles` 14 -> 20, `lat_backtrack` 1834 -> 2252, `band_len` 70646 -> 62860; routed `x_drawn` 493
    -> 494, `lat_backtrack` 405 -> 1001.
16. **The switch** (`B/check-verify-cold-read/cfgprobe`): `cfg!(flowmaid_blocks)`, a thread-local override and a
    scoped setter build clean under clippy with `-D warnings`, on 1.75 and on wasm32, and pass with and without the
    cfg.

## Appendix B. Decisions

Taken by the user on 2026-11-10 (`B/USER-DECISIONS.md`):

- **The bar.** "pokoknya mirip atau lebih bagus dan rapi dari mermaid js 12": acceptance on the analog and on v2 is
  stated against mermaid 12.2 (4.3 a, b). The tolerances there are this spec's reading of "a small stated margin":
  block shape + 20 % on the analog with the reason given (v2 takes the analog's bound for the block the two inputs
  share, which is + 36 % on `compact_max` against mermaid's v2 drawing; holding v2 to + 20 % would mean lowering the
  analog's row to 4.84 as well); crossings + 4 (+ 2 on the public fixture); bends and detour + 20 %; no struck chip,
  no more crowded chips than mermaid, edges through flow-facing sides only; area, edge length and label distance stay
  better than mermaid's (label distance: on the analog and v2 the smaller of mermaid's and 1.25 x 0.38's, on the
  public fixture, which has the room, 1.1 x 0.38's).
- **D1 Static time on tiny clustered files.** For files under 1 ms the static limit is max(2 x baseline, baseline +
  0.3 ms).
- **D2 Split**: yes. B12a (nothing visible, merged to dev, no release), then B12b and release 0.40.0. The contents of
  the two packages are in section 5; against the first cut, the edge ends and the label room are built in B12a,
  behind the switch.
- **D3 Upstream edges between blocks**: channels. They leave the top of a node and enter the bottom of a block
  (mermaid's look). Back-edges between two plain nodes keep the lane.
- **D4 Lane rule**: the principle is accepted; the constant comes from the sweep; such diagrams must not get worse.
  What the rule means in pictures: under L4 a level keeps its linked frames side by side, as 0.38 draws them
  (`B/critic/out/swimlane.038.png`), where mermaid stacks two compact blocks with 2 of 5 arrows upward
  (`B/critic/mm/swimlane.mermaid.png`, also `d-global`'s result, `B/critic/out/swimlane.exp.png`). Such a level fails
  the block-shape rules by design. It applies to a ping-pong only; a request and its response are stacked
  (`B/check-fix-a/mm/reqresp.mermaid.png`).
- **D5 The analog in the public repository**: not granted. The analog family stays a local acceptance input; the
  repository's fixture and README image are the synthetic `blocks-overview`.
- **D6 If a stop rule fires**: the implementer stops and reports; no gate is re-baselined to pass.

Taken by this spec on the user's standing answer, **to be confirmed before B12.3 starts**:

- **D7 Diagrams whose rows 0.38 already stacks.** Two options. (a) Keep 0.38's drawing for them (`d-hybrid`'s rank
  trigger): 21 of 36 framed inputs stay byte-identical, the README images of `advanced` and `subgraph` do not move,
  and simple block diagrams keep 0.38's edges: `hub5` 5 crossings of a lateral frame wall where mermaid has none
  (`B/critic/out/hub5.038.png` against `B/critic/mm/hub5.mermaid.png`), `cycle2` its returns as side lanes where
  mermaid draws channels (`B/critic/out/cycle2.038.png` against `B/critic/mm/cycle2.mermaid.png`). (b) Block mode for
  every diagram with a border link (2.2): those drawings get the block edge rules; `advanced`, `subgraph`,
  `fanout-frames` and four notes fixtures are re-laid and gated one by one (4.3 e), two README images may be
  re-rendered, and the sub-millisecond files run the block pipeline inside D1's budget. **This spec builds (b), and
  B12a starts on it**: it is what "the default for every flowchart and state diagram with subgraphs" says, and (a) is
  below the bar on hub5 and cycle2. B12.0, B12.1 and the spike do not depend on the answer (the off-identity test
  covers every fixture). If the user chooses (a): `levels::block_mode` additionally compares the two rankings, on
  every `route()` and `route_partial()` frame, so it is no longer a pure predicate, the budgets of 4.6 are
  re-measured and `block_mode_agrees_between_scene_and_route` carries the load; the permanent hash list grows by the
  rows-equal inputs; the scope sentences of 1.1 and 4.7 read "diagrams whose rows change"; 4.3 (e) falls away.

## Appendix C. Reviewed and kept

The spec was reviewed twice. The first review raised 22 findings; the second re-checked the revision and raised 60
(11 on the algorithm's first half, 11 on its second half, 18 from a cold read, 20 on the acceptance). All hold and all
are resolved in the body. Where the body departs from the change a finding proposed, or keeps something a review
questioned, the reason is here.

**From the first review**

- **Block mode (2.2).** The review proposed to leave D7 open and, for option (b), to turn block mode on "when a drawn
  edge with a block end runs upstream or crosses a frame". The decision is taken as (b) on the user's standing answer
  and flagged for confirmation. The predicate is "a border link exists" because whether a 0.38 edge crosses a lateral
  wall cannot be known before drawing, and a hub with blocks and only downstream edges has such crossings too. So
  `subgraph.mmd` and `fanout-frames` are in block mode as well, beyond the review's list.
- **`block_mode`'s inputs.** The shared constructor is kept as proposed. The predicate no longer reads L3's count of
  turned links, so the weight of a stand-in against a rank-only pair cannot make the entry points disagree.
- **L4 and "the trigger stays off".** With block mode structural there is no off state for such a level; 2.3 L4 says
  exactly which steps run and 4.3 (e) gates the result.
- **LR titles.** The review proposed "the block's extent along the flow plus the gap after it is at least the title's
  length". The frame itself grows instead (2.7 F2): a title hanging over into a band gap would be crossed by the
  tracks of that gap, and titles are never struck.
- **One number of the review is corrected.** `lateral.py` assumes a top-down flow. `notes/spread-bottom` is an LR
  diagram with 1 lateral crossing, not 4, and `p11-lr` has none; hub5 (5), cycle2 (6) and p06 (2) stand, and so does
  the finding.
- **Kept against the review's doubts**: the turned-link threshold (the ratio fixtures show what it does: 1:2 and 1:3
  keep the author's order, 1:4 flips), chains for upstream edges, level-first ordering with seven starts as the
  default thoroughness, the lane rule at one third (now with its second test).

**From the second review: proposed changes that were not applied as proposed**

- **Lane rule (three findings on L4).** One proposed a second condition (a path that enters a block twice), one
  proposed to count the minimum in written directions and to run L3 and L4 again after dissolving, one proposed the
  re-run alone. The body has all three as one rule. Not applied: the sentence "no link between two former child
  blocks is turned" (it contradicts breaking a grandchild's cycle, whose links run between exactly those blocks), and
  the worry that counting in written directions makes `p06-block-cycle` a lane level (the path condition keeps it
  stacked; checked by script). `swim-nested`, proposed as a lane fixture, is a there-and-back exchange under the
  merged rule and is asserted as stacked; `swimlane-nested` is the fixture for the re-run.
- **Jog-room count.** Not the proposed "far end lies on the title side by position" (the count would depend on a
  growth it decides, and `route_partial` would not see a far block move) nor the fallback "incoming links only" (it
  reserves nothing for a turned exit under a chip). Counted by the edge's class instead (2.7 F2), with R6 redrawing
  the frames whose links can change class.
- **F2 stops in `route()`.** The first draft stopped a growth `FRAME_GAP` short of whatever lay beyond; one finding
  proposed to redraw, in `route_partial`, every frame whose box differs from its base by a strip that meets a drag
  zone, another to keep base boxes and document the difference. The growth is unconditional instead (2.7), which
  removes the stop for F2; for F3's `stop_at_siblings`, which 0.38 has too, base boxes are kept as today. The
  proposed redraw clause would move frames that hold no end of the dragged node and put `DRAG_JUMPS_MAX` at risk.
- **Own-box edges.** Of the two proposals ("flag the pair, lay out as today outside block mode" and "build no pair
  at all") the body takes the flag as the mechanism and the second one's result: a flagged pair ranks nothing on any
  path, so a block's rows do not depend on whether some other border link exists. The pair is still built because
  the drawing code uses it until B12.14.
- **Inner members in the jog count.** Not applied: a per-member capacity double-counts columns that several runs
  share, `route()` cannot evaluate "columns X2.5 can give", and an inner run needs no jog room. The run takes a
  column beside the chip instead (X2.5).
- **Title cut at the canvas edge.** The proposal extended the canvas past reserved chips. The frame grows to its
  title instead (2.7 F1): the canvas is defined from nodes, frames and edges
  (`a_static_canvas_is_its_drawing_with_even_margins`), cannot know block mode, and "a chip X2.2 reserved room for"
  has no meaning in `route()`.
- **`cap(m)` through `ports::unshaded` unchanged; deleting the router's `CHIP_CLEAR`.** Not applied: `unshaded`'s
  depth test would switch the jog room off once it is two tracks deep, and deleting the router's constant moves the
  router's chip clearance from 6.0 to 6.5 px in every clustered diagram, against 1.3 item 3.
- **Free pins as one even band per side.** Applied per run of consecutive pins instead: one band for the whole side
  would drop the order among the member entries that cross the same side. The clamp in `comb.rs` is conditional on
  B12.11's test, because the comb's own chip test should already keep the jog.
- **Numerals B11 moved.** Several findings proposed to write B11's numbers for `DEFECT_BASELINE`, `XGAP_MAX`, the
  casing limits and `DRAG_JUMPS_MAX` into the spec. B11 has already moved them again and is not finished; the spec
  says "as the starting release has it" and B12.0 records them.
- **`route_partial` limit.** The proposed max(6 ms, starting release + 0.5 ms) is not applied: PLAN invariant 1 says
  under 6 ms. The spec says how the number is measured and what to do if the starting release is already at it.
- **`lat_backtrack`.** One finding proposed it as a must not above 0.38's on the block inputs, another to hold every
  `DEFECT_BASELINE` field at or below the starting release. Mermaid's own drawings have three to eight times 0.38's
  value by this measure: going round a block is sideways travel. So it is reported, not gated, on the block inputs,
  and in `DEFECT_BASELINE` it is in the class that may rise with a named cause and a picture, with `band_pairs` and
  `band_len`. `lat_wiggles` and the other fields stay at or below.
- **v2's block shape.** The proposed 4.84 / 3.77 (mermaid's v2 values + 20 %) is not applied: the frame that sets the
  number is the same block in the analog and in v2. The bound stays 5.5 / 3.9 and appendix B says what that is
  against mermaid on v2.
- **v2's label distance.** "v2 has room" did not hold (its experiment value was measured with labels off the middle);
  v2 gets the analog's rule, 329.
- **`lateral_x` on the dense fixtures.** Not above mermaid's count, target 0 (the lead's decision, section 0), and not "0 on every other block-mode fixture": the notes
  fixtures keep the rule of 4.3 (e), where mermaid itself has a note link through a lateral wall.
- **Routed `spine_off` and `pin_corner`.** The proposal loosened them to "not above the starting release's". They
  stay 0 on the named inputs: at the scene's centres a spine's two nodes are aligned and `route()` places pins by the
  static rule.
- **Spill order.** Not "inner edges and E4 lanes spill first": a lane never goes round the corner in today's `spill`.
- **`spine_off` without layers.** Not "whose two boxes face each other", which can be read as overlapping in breadth
  and would not count a spine that is far off; defined by the absence of a node centre between the two along the
  flow.
- **The cut.** One finding proposed four packages. The body keeps the two packages of D2 and gives B12a three parts
  with their own checkpoints; narrowing is split so that its row-changing half goes with the layering and its
  acceptance half comes after the gap room exists. If each part should be its own implement-verify-fix cycle, the
  parts are cut at A1 and A2 without any change to the items.
- **D7 under option (a).** "Nothing else in B12a is redone" was too strong; appendix B says what changes.
- **Names.** Of the two names proposed for the harness twin of block-metrics' `lbl_crowd`, `chip_crowd` is used.

**Item numbers.** The items were renumbered into the order they are built. First draft -> this spec: B12.0 -> B12.0;
B12.1 -> B12.1; B12.1s -> B12.2; B12.2 -> B12.3 (with the row-wrapping half of the old B12.12); B12.3 -> B12.4;
B12.4 -> B12.5 (with the fan test of the old B12.10); B12.5 -> B12.8; B12.6 -> B12.9; B12.7 -> B12.7 (F1, F2) and
B12.10 (F3, the three entry points); B12.8 -> B12.13 (with the routed half of the old B12.9); B12.9 -> B12.11
(static) and B12.13 (routed); B12.10 -> dissolved (3.3); B12.11 -> B12.14; B12.12 -> B12.3 and B12.12; B12.13 ->
B12.6 (invariants, constants) and B12.15 (comparison); B12.14 -> B12.16. Checkpoint A became A1 and A2.
