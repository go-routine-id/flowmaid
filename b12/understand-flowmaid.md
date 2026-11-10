# B12 "subgraphs as blocks" — how flowmaid 0.38.0 lays out and draws a clustered flowchart today

Label: `u-flowmaid`. Source read: `b12/tree/src` (dev = 0.38.0, commit 0366084). All `file:line` references are into that tree.
Everything measured here was measured on the sanitized analog `b12/in/infra-overview.mmd` and on the repo's own corpus; nothing from `b12/private/` is quoted.

Scratch artefacts (all under `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/u-flowmaid/`):

- `tree/` — a copy of the tree with env-guarded debug prints (`FMDUMP`, `FMBLOCKS`, `FMSTATICROUTE`) and `examples/b12probe.rs` (summary / detail / pos / routeat). With no env var set its CLI output is byte-identical to `b12/fm038` on the analog (checked with `cmp`).
- `out/infra.dump.txt` — ranks, orders, proxies and dummy counts of the analog; `out/infra.quality.tsv` — the quality harness row for it.
- `blocks/stack.png`, `blocks/stack-staticfinish.png` — the experiment of section 2.5 (hand-placed compact blocks drawn by today's edge stages).

---

## 0. Summary for a designer (read this first)

1. **A node's rank is decided globally and per node** (`layout::rank`, longest path over the whole graph, `layout.rs:2281`). Subgraphs take part only through `seat_box_sources` (`layout.rs:2497`) and rank-only groups. Nothing makes a subgraph's members take consecutive ranks, so a frame is as tall as the rank spread of its members.
2. **Contiguity exists only across the flow, layer by layer** (`enforce_contiguity` / `arrange`, `layout.rs:5513` / `5534`, plus wall dummies, `layout.rs:3255-3282`). A frame is therefore always one clean rectangle with no foreign node inside, but not a compact one.
3. **Siblings are placed relative to each other by one ordering key per subgraph** (`cluster_keys`, `layout.rs:5487`), i.e. side by side across the flow wherever their rank spans overlap. They end up stacked along the flow only when ranks separate them, or when `narrow()` adds a stacking group, which it does only for a drawing broader than 2.5 : 1 and never for two subgraphs that reach each other both ways (`layout.rs:2237-2238`).
4. On the analog: longest path runs actors → DNS → ingress → lane → database → standby → chat bot, entering "External services" at rank 1 and again at rank 6, so that frame spans ranks 1..6. All four big frames overlap in rank, so they sit side by side, and every inter-frame edge becomes a long step across the flow in a global rank gap (16 997 px of sideways run in total).
5. `compound.rs` (B9) already is a block layout: split into parts, lay each out with the whole pipeline, place the block as a proxy node, join. It refuses any subgraph with an edge across its border in three places (`judge` `compound.rs:182`, `split` `compound.rs:326-327` and `336-338`) and its `join` needs every graph edge to be drawn whole by exactly one part (`compound.rs:649-650`).
6. `route()` gets only the graph and node centres. Frames in `route()` are a pure function of member positions (`cluster_raw_boxes`, `scene.rs:5346`). Back-edges are decided geometrically (`route_backs`, `scene.rs:8097`). Compact frames therefore survive a drag for free; channels between blocks do not exist in `route()` at all.
7. Experiment (section 2.5): with the analog's nodes hand-placed as compact stacked blocks and today's edge stages run unchanged, the frames are compact and the sideways run halves (8 952 px), but long forward edges run straight through the frames between their ends (5 edge/foreign-frame pairs against 3) and crossings rise from 24 to 39-43. **Placing blocks is not enough: inter-block edges need channels round the blocks, in `scene()` and in `route()`.**
8. Block-level cycles are the norm, not an edge case: in 11 of the 16 clustered inputs checked (the analog included) some top-level subgraph reaches itself through other top-level units.
9. `scene()` is **not** equal to `route()` at the scene's own positions today, neither frames (up to 617 px apart on bench-75, 71 px on the analog) nor edge ends (19 of 27 edges differ by more than 1 px on the analog). Only specific agreements are pinned by tests (section 4.3).
10. There are no byte-exact SVG snapshots. What pins layouts is roughly 115 geometry tests that involve subgraphs (by a text scan, section 5.2) and the ratchets in `tests/quality.rs`; four of its six fixtures have subgraphs and three of those have 28-69 edges across borders, so B12 moves nearly every number in that file.

---

## 1. The pipeline for a clustered flowchart, stage by stage

Entry: `scene::scene` (`scene.rs:3014`) → `layout::node_texts` (wrapped labels and sizes, `layout.rs:1425`) → `scene_labelled` (`scene.rs:3050`).

`scene_labelled` does, in order:

1. `sane_sizes` (`scene.rs:3056`).
2. `compound::split(g)`; if it returns a split, `compound::scene` builds the whole scene and returns (`scene.rs:3058-3062`). Section 3.
3. `fit_bars` (fork/join bars sized across the flow, `scene.rs:3063`, def `7456`).
4. `scene_flat` when `g.subgraphs.is_empty()`, else `scene_clustered` (`scene.rs:3064-3068`).
5. `edgepass::finish(g, &mut sc, Mode::Static)` (`scene.rs:3069`).

State diagrams ride the same path: `lib.rs:109-110`, `Document::State(g)` is a `Graph`.

### 1.1 Parser

Flowchart (`parser::parse`, `parser.rs:1642`):

- Subgraph ids are pre-scanned (`sub_ids`, `parser.rs:1722`) so forward references work.
- `subgraph id [Title]` pushes a `Subgraph { id, title, nodes: [], parent: stack top, direction: None }` (`parser.rs:1798-1827`); `end` pops (`1854`); `direction XX` inside a block sets `Subgraph::direction` (`1858-1871`), at top level it is an error.
- Membership: mentioning a node inside a block claims it for that block; the first block to close keeps it; an open enclosing block gives it up to the inner one (`parser.rs:2737-2758`). `Subgraph::nodes` holds direct members only; nesting is by `parent` (`model.rs:336-351`).
- An id that names a subgraph resolves to `End::Sub(si)` (`parser.rs:2713-2726`). Every edge pair with a subgraph end goes to `Graph::sub_edges` as a `SubEdge { from: End, to: End, … }`, everything else to `Graph::edges` (`parser.rs:2423-2442`; types `model.rs:356-376`).
- The `flowchart-elk` header is accepted and means nothing today (`parser.rs:1307`, `1758`); `to_mermaid` re-emits it as `flowchart` (`emit.rs:1739-1740`).
- Styling a subgraph (`style`, `class`) and `id@{…}` on a subgraph are accepted and dropped (`parser.rs:1921-1929`, `1968-1975`, `1846-1851`).

State diagram (`parse_state`, `parser.rs:5639`): a composite `state X {` is a `Subgraph` (`5737-5747`); a transition with a composite end is a `SubEdge` (`5761-5775`); a composite's own `[*]` nodes and its note cards are pushed into its `nodes` (`5903-5912`, `5979-6003`); `direction` inside a composite sets its `direction` (`5692-5696`).

### 1.2 `scene_clustered` (`scene.rs:3444-3763`)

| step | what | where |
|---|---|---|
| a | `node_cluster_paths(g)`: per node, its subgraph chain outermost first (empty = top level) | `scene.rs:4252-4271`, called `3445` |
| b | `grid_pairs`: rank-only pairs that pack a subgraph's loose members (edge-less, or fed only by one outside node) into a near-square grid; skipped for a subgraph whose `direction` lies across the diagram's | `scene.rs:4172-4231`, consts `3432-3437` |
| c | `sub_edge_proxies`: per sub-edge, the members a box stands for when ranking. A box as target = its entry members (no in-edge from inside the box); as source = its exit members; a cycle inside falls back to `cycle_ends`. Member → own box gives `(v, entry members)`; box → own member gives nothing | `scene.rs:3994-4061`, `4112-4158` |
| d | Stand-in edges: one real `Edge` per sub-edge, appended after the real ones in a clone `ga`, from the first `from` proxy to the first other `to` proxy, carrying the sub-edge's label. `ga.frame_cut[ei]` says which frame(s) it is cut back to; `ga.drawn_edges` is what the scene will draw | `scene.rs:3455-3519` |
| e | `layout_clustered_ranked(&ga, sizes, &node_cluster, &rank_only)` with `rank_only = proxies + grid` | `scene.rs:3520`, `layout.rs:1536` |
| f | `static_lanes` (back-edge lanes on the layers) and `scene_from_layout` (node positions, one cubic per short edge, dummy-chain waypoints per long edge, lanes, labels, direction transform) | `scene.rs:3521-3522`, defs `3097`, `3159` |
| g | Frames: `cluster_raw_boxes`, widened over the channel lanes laid inside each box and over "home" back-edge lanes, `stop_at_siblings`, `renest_boxes`, `clusters_of` | `scene.rs:3535-3599` |
| h | Sub-edges drawn: member → own box as its stand-in whole; box ↔ own member as `own_box_stub`; else the stand-in's waypoints cut at the frame(s) (`cut_to_frames`), else `sub_edge_ported`, else `free_edge` box to box | `scene.rs:3608-3743`, helpers `3772`, `3832`, `3889` |
| i | `avoid_label_overlaps`, then the canvas re-normalised to `MARGIN` | `scene.rs:3746-3761` |

### 1.3 Layout (`layout.rs`)

`layout_core` (`layout.rs:1545-1598`): `rank` → first `place` → (if within `SEARCH_MAX_ELEMENTS` = 600 nodes + edges) `narrow` when `total_b > RIBBON_ENTRY (2.5) × depth` → `centre_largest`.

**Ranking — `rank` (`layout.rs:2281-2440`).**

1. Ranking graph = real nodes plus one hub per rank-only group (`from → hub` one layer, `hub → to` none) (`2289-2312`).
2. Cycle breakers by iterative DFS, in source order and (≤ 400 elements) in target-declaration order, whichever reverses fewer real edges (`2314-2371`).
3. Longest-path layering, Kahn style (`2373-2398`).
4. With clusters: `seat_box_sources` (`2400-2404`, def `2497-2773`). A box fed from a node outside every box starts on the row below its topmost such feeder, and its source members (edge-less, or only linking onward) come down to that first row. A box fed only from other boxes stays put (`2476-2486`).
5. Lead nodes (state `note left of`) one layer before their target (`2406-2432`).

Later re-rankings: `tighten_sources` (class diagrams, and step 2 of `narrow`), `stack_candidates` / `wrap_candidates` (rank-only groups added by `narrow`, then `rank` run again), `stagger_proposals` (`layout.rs:2840`, `2157`, `2042`, `2877`).

**Placement — `place` (`layout.rs:3026-4271`).**

- Dummy chains: an edge spanning more than one layer gets one dummy per crossed layer; a labelled one sizes its middle dummy to its label (`3066-3149`). Back-edges and invisible links get no chain.
- Rank-only groups add ordering links only, no channel (`3150-3173`).
- Cluster handling in the ordering:
  - `cluster_span`: per subgraph the min..max layer of its members (`3188-3211`).
  - Wall dummies: for every subgraph, a left and a right zero-size dummy on every layer of its span, chained along the flow so the walls come out straight (`3255-3282`).
  - `apath`: the cluster path of every augmented node. A long edge's dummy takes, at each layer, the target's path if that layer lies in the target's cluster span, else the source's, else the common prefix (`3292-3341`). This is what makes a long edge run outside every foreign box and enter its target's box from the first layer of that box's span.
  - Ordering: 8 rounds of weighted-median sweeps + `transpose`, each followed by `cluster_keys` + `enforce_contiguity` on every layer; the fewest-crossing round is kept (`3343-3397`). `transpose` swaps only same-path neighbours and never a wall (`4769-4772`).
  - `cluster_keys` (`5487-5506`): one key per subgraph for the whole diagram = the mean normalised in-layer position of its non-wall members over all layers. `arrange` (`5534-5593`) sorts the units of a layer (a subgraph's group, or a free node) by key, recursing per nesting level; a wall sorts to its group's edge.
  - `centre_largest` may override the key of the largest top-level subgraph (`Opts::hint`, `3431-3439`, search `1936-1998`).
- Coordinates across the flow: Brandes-Köpf (`coordinates_bk`, `4880`), separations from `pair_sep` (`5379-5417`): `GAP_B` 50 between real nodes, `EDGE_GAP` 18 for dummies, `BORDER_GAP` 4 at a wall, plus `cluster_gap` = `CLUSTER_PAD` 18 per subgraph boundary crossed (`5471-5475`), plus the title strip beside a left wall in LR/RL (`5402-5415`).
- Header slots: in TD/BT a member on its box's screen-top layer grows that layer's slot by `CLUSTER_HEADER` 26 + `title_extra` per box opening there, on the strip's side only (`lextra`, `3212-3238`). `reach` = how far the frames a node opens/closes reach past it (`FRAME_PAD` 14 per level + header) (`3239-3254`).
- Layer lines and gap needs: `layer_lines` (`4062-4175`).
  - frames closing on one layer and opening on the next clear each other by `FRAME_GAP` 8 (`4083-4088`);
  - a labelled adjacent-layer edge grows its gap for the label and head room (`short_labels`, `4097-4145`), measured from the frame border when the end's frame closes/opens there, and for a stand-in from the frames it is cut to (`3729-3772`);
  - `hub_need` (`4149-4156`), `gap_need` for stacked label rows (`3878-3904`, `4157-4162`);
  - `cross_need`: a gap in which an edge crosses a box border sideways keeps the border `FRAME_SEP` off the run's middle (`3930-3988`, `4163-4165`);
  - `track_need`: `ROW_CLEAR + HEAD_LEG + (tracks − 1) × TRACK_PITCH` for the most overlapping steps in the gap (`4002-4051`, `4166-4169`).
- The search judges a candidate on `drawn_crossings`, `node_hits`, `drawn_breadth` and the "judged" layer lines (`4216-4236`).

**Narrowing — `narrow` (`layout.rs:1656-1856`).** Runs only for a ribbon. Step 1 tries sibling stackings and one wrap per round (`1710-1784`); a pair is a candidate only if the two share a layer and **no path runs both ways between them** (`2237-2242`); accepted only within area +2 %, edge length +10 %, flow length +15 %, crossing and node-hit budgets (`1693-1708`).

### 1.4 Frames

`cluster_raw_boxes` (`scene.rs:5346-5386`): deepest first, bbox of the direct members and child frames, plus `SUB_PAD` 14 on every side and `SUB_HEADER` 26 + `title_extra` on the screen top (in every direction). Empty subgraph = no frame. The title's width plays no part.

Growth in the static scene only (`scene.rs:3535-3599`): each frame is widened across the flow to cover channel waypoints whose dummy was laid out in that box (by `EdgeGroups`, the `apath` of each waypoint) plus `LANE_PAD` 12, and "home" back-edge lanes with their labels; `stop_at_siblings` (`5481`) stops `FRAME_GAP` short of a sibling; `renest_boxes` (`5524`) re-encloses children.

Titles: `fit_cluster_title` (`scene.rs:2902-2948`) keeps a title whole unless it runs into a non-nested frame or past the canvas, then cuts each over-long line with an ellipsis. `title_head` (`2885`) is the chip rectangle the edge stages keep clear of.

### 1.5 Edge stages — `edgepass::finish_held` (`edgepass.rs:361-409`)

Shared by `scene()`, `route()` and `route_partial()`:

1. `ctx` (`edgepass.rs:126-258`): ends per scene edge (`EndRef::Node` / `EndRef::Frame`), matched by ids in order; a member → own-box sub-edge ends on the entry member (`own_box_targets`). **`back` is geometric**: target centre behind source centre along `g.direction` (`221-230`).
2. `ports::assign` (`370`): ends grouped per side of a node or frame, ordered by heading, spaced.
3. `ortho::orthogonalize` (`373`): every flowchart/state edge redrawn on the axes. Skipped above `ROUTE_MAX_EDGES` = 400 scene edges (`ortho.rs:180-185`, `router.rs:263`). **Its gaps are global**: `ortho_pass` merges the flow extents of all nodes of the scene into rows and takes the free bands between them (`ortho.rs:480-506`); frames only contribute walls a step keeps off (`523-536`). A step with no gap between its ends goes mid-way between them in a private gap (`ortho.rs:1600-1601`); with several, `plan_jog` picks by boxes hit, foreign frames crossed, lines crossed, track load (`1603-1680`). Long edges follow their dummy columns only in the static scene (`586-603`).
4. `router::repair` (`374`): A* reroute of edges through a node, a foreign frame or a title chip, or on top of another edge; nodes block, foreign frames are only priced (`router.rs:1-69`); a much smaller budget in `route()`.
5. `ports::respace`, `router::nudge`, `router::spread_close`, `glyph_stubs`, `straight_cubics`, `square_s_cubics`, `snap_ends`, `ports::reround` (`379-400`).
6. `labels::relabel` (`405`), `refit` (`408`, def `1127`).

### 1.6 SVG — `to_svg_a11y_with` (`scene.rs:6407`)

Canvas from `grow_canvas`; crossing gaps from `casing::masks` (`6461`); paint order: frames outermost first (`6477-6492`), edges (`6525`), nodes (`6589`), frame titles last on chips so no edge strikes them.

### 1.7 The three questions

| question | deciding stage | notes |
|---|---|---|
| (a) a node's rank | `layout::rank` (`layout.rs:2281`): global DFS cycle breakers + longest path, then `seat_box_sources`, lead nodes; re-run by `narrow` with stacking/wrap groups; `tighten_sources`, `stagger_proposals` move single nodes | inputs: `g.edges` incl. stand-ins, rank-only groups from `sub_edge_proxies` and `grid_pairs` |
| (b) whether a subgraph's members stay contiguous | across the flow, per layer: `enforce_contiguity` / `arrange` + wall dummies + same-path-only `transpose` (`layout.rs:5513`, `5534`, `3255-3282`, `4769`). Along the flow: **no stage** — the span is just min..max of the members' ranks (`3188-3211`) | result: a rectangle with no foreign node in it, as tall as the rank spread |
| (c) where siblings sit relative to each other | across the flow: `cluster_keys` (`layout.rs:5487`), optionally `centre_largest`'s hint, then BK + `pair_sep`. Along the flow: only ranks — real edges, sub-edge proxies, or a stacking/wrap group from `narrow` (ribbons only, never for mutually reachable siblings) | `seat_box_sources` moves a box down below a top-level feeder |

---

## 2. Why the analog looks as it does

Dump: `u-flowmaid/out/infra.dump.txt` (`FMDUMP=1`). Node ids are the analog's.

### 2.1 Ranks

No cycle breaker. Longest path gives:

| layer | nodes (subgraph) |
|---|---|
| 0 | U1, T1, U2 (top level); R1, R2 (GH) |
| 1 | GW, DNS (EXT); VAULT, ARGO (PROD) |
| 2 | PSDEV, SITE (DR); TR1 (PROD); TR2 (STG) |
| 3 | LANE (PROD); PSPROD, LANES4 (STG) |
| 4 | OAI, AZ (EXT); PG (PROD); GITLAB (STG) |
| 5 | DRAPP (DR) |
| 6 | TG (EXT) |

The longest chain is `U* → DNS → TR1 → LANE → PG → DRAPP → TG`: it passes EXT, PROD, PROD, PROD, DR, EXT.

- GW is edge-less; it sits on layer 1 because EXT has top-level feeders (the actors), so `seat_box_sources` seats it on EXT's first row.
- VAULT is edge-less; it sits on layer 1 because the sub-edge `R2 --> PROD` ranks PROD's entry members `[ARGO, TR1, VAULT]` below R2 (`sub_edge_proxies`). PROD has no top-level feeder, so it is not seated.
- Stand-ins: `R2 → TR2` (for `R2 --> STG`), `R2 → SITE` (for `R2 --> DR`), `R2 → ARGO` (for `R2 --> PROD`).

### 2.2 Why "External services" spans the full height

Its members are ranked one by one: DNS and GW on layer 1 (fed by the actors), OAI and AZ on layer 4 (fed by LANE on layer 3), TG on layer 6 (fed by DRAPP on layer 5, and by ARGO and PSPROD). `cluster_span[EXT] = (1, 6)`; walls are created on all six layers; `cluster_raw_boxes` wraps the member bbox. Frame: 631 × 939 px on a 1 225 px tall canvas.

DR is the same in small: SITE and PSDEV on layer 2, DRAPP on layer 5 (because `PG --> DRAPP`), frame 706 × 589.

### 2.3 Why the three servers (and EXT) sit side by side

Layer spans: EXT 1..6, PROD 1..4, DR 2..5, STG 2..4. They all overlap, and members of one subgraph must be contiguous on every shared layer with one order for all layers, so the four frames are four bands in `cluster_keys` order: DR 0.148, GH 0.286, EXT 0.423, PROD 0.606, STG 0.842 — exactly the left-to-right order in `infra-overview.fm038.png` (GH is on layer 0 only, above).

Nothing can stack them:

- the ranks interleave because the edges between them run both ways (EXT → PROD by `DNS --> TR1`, PROD → EXT by `LANE --> OAI`; EXT ⇄ DR; EXT ⇄ STG; PROD → DR; PROD → STG);
- `narrow()` never runs: the first placement is 2 687.5 broad × 1 245.6 deep = 2.16 : 1, under `RIBBON_ENTRY` 2.5 (dump line "ribbon=false");
- if it ran, `stack_candidates` would offer no pair among EXT/PROD/DR/STG that reach each other both ways (`layout.rs:2237-2238`).

### 2.4 Where the long horizontal runs come from

Every inter-frame edge joins two nodes on adjacent or near layers whose frames are whole bands apart across the flow. `ortho` draws it as a Z whose step runs in the global gap between those two layers, and that gap runs across every frame whose span covers both layers.

Per-edge numbers from `b12probe detail` (px; "lat" = total sideways run; label = distance from the label to the nearer end):

| edge | lat | label | remark |
|---|---:|---:|---|
| PG → DRAPP | 1 441 | 699 | step in the L4/L5 gap, straight across the whole EXT frame |
| DNS → TR2 | 1 336 | 210 | step in the L1/L2 gap, across PROD |
| PSPROD → TG | 1 271 | 377 | 2 dummies, enters EXT at its span |
| R2 → STG | 1 064 | 503 | stand-in `R2 → TR2` |
| DNS → PSPROD | 1 032 | 251 | across PROD |
| ARGO → TG | 998 | 283 | 4 dummies, runs down inside EXT |
| ARGO → GITLAB | 996 | 474 | |
| DNS → DRAPP | 984 | 295 | 3 dummies, descends inside DR |
| R1 → ARGO | 862 | 191 | |
| U2 → DNS | 790 | 129 | |

Total sideways run: 16 997 px over 27 edges. The intra-frame edges (TR1 → LANE, LANE → PG, TR2 → LANES4) have 0.

The corridor of parallel lines is the single gap between layers 1 and 2: ten edges cross it (six out of the hub DNS to three different frames, the chains of ARGO → TG and ARGO → GITLAB, two stand-ins from R2). `track_need` (`layout.rs:4002-4051`) grows that gap by one `TRACK_PITCH` per overlapping step, so the lines are stacked a pitch apart for over a thousand px.

Labels far from both ends: a labelled Z's label rides the middle of its step, or the run into the target when the step fits between the source and the label (`ortho::relabel`, `ortho.rs:2220`). On a step 1 000+ px long the middle is 300-700 px from either end.

What the quality harness says about this drawing: 3 defects in both modes (3 edge/foreign-frame pairs), `frame_overlap` 0, `node_frame` 0, 24 drawn crossings, 16 parallel "band" pairs over 6 796 px, aspect 2.28, `scene_ms` 6.6, `route_ms` 1.9, worst drag frame 5.8 ms. **None of today's gated metrics sees a frame that is far larger than its content, nor the length of sideways runs, nor label-to-end distance.** B12 needs its own measures for those.

### 2.5 Experiment: the same edge stages on hand-placed blocks

Each subgraph of the analog was laid out on its own by 0.38.0 (its members and internal edges only), the blocks were stacked along the flow (actors; GH and EXT side by side; DR; PROD; STG, 150 px apart) and the whole graph was drawn with `route(g, centres)`, i.e. the unchanged position-driven pipeline.

| | 0.38.0 `scene()` | blocks + `route()` | blocks + static-mode finish |
|---|---:|---:|---:|
| sum of the five frame areas (px²) | 1 805 170 | 889 419 | 889 419 |
| EXT frame | 631 × 939 | 1 058 × 126 | same |
| canvas | 2 787 × 1 225 | 2 132 × 1 804 (`route()` does not normalise; the drawing itself is as in the next column) | 1 822 × 1 792 |
| total sideways run (px) | 16 997 | 8 952 | 9 204 |
| edge / foreign-frame pairs | 3 | 5 | 5 |
| drawn crossings | 24 | 43 | 39 |
| labels with another stroke through them | 0 | 5 | 5 |
| `route()` time (ms) | 1.87 | 4.77 | (7.99) |

Reading (`blocks/stack.png`):

- The frames are compact because `route()` derives them from member positions. Nothing else is needed for that.
- The five edges against the flow (into EXT from below, and `PG → DRAPP`) are taken by `route_backs` as back-edges and drawn as lanes beside everything. That part works.
- The long forward edges (`DNS → TR2`, `DNS → PSPROD`, `ARGO → GITLAB`, `LANE → GITLAB`) run straight down through the DR and PROD frames. `route()` has no dummy columns; `cross_cluster_route` (`scene.rs:8400`) only makes a 4-point path between the two end boxes; the router prices foreign frames instead of blocking them, and a larger static budget does not clear them either (third column).
- `route()` got 2.5× slower because more edges need the router.

So today's mechanism for "edges run in the gaps between frames" is the dummy chain ordered outside foreign walls in the one global Sugiyama pass (`apath`, section 1.3). A block layout must replace it with something that also exists in `route()`.

---

## 3. `compound.rs` (B9)

Module doc: `compound.rs:1-33`.

### 3.1 Which subgraphs are blocks — `blocks` / `judge` (`compound.rs:134-189`)

Per subgraph, outermost first:

- `internal[s]`: some edge or sub-edge has both ends inside `s`;
- `external[s]`: some edge or sub-edge has exactly one end inside `s`. An end `End::Sub(s)` lies inside the ancestors of `s` only (`inside`, `119-129`), so `X --> s` is not external to `s`; `member --> s` from inside is.
- `out[s] = direction set and different from the direction round it && internal[s] && !external[s]` (`181-182`).

`Flows` (`194-227`) gives the quality harness each edge's flow direction.

In the corpus only `examples/state.mmd`'s `Validating` is a block today (`FMBLOCKS=1` over every clustered input).

### 3.2 `split` (`compound.rs:242-487`)

- Outermost blocks only (`tops`, `251-253`); a block nested in a block is found again when the block's part goes through `scene_labelled`.
- Block part (`283-362`): `g.part(dir, members)` (`model.rs:819`), the block's subgraph and everything nested as the part's subgraphs with the block as its one top-level frame, edges with both ends inside. **An edge with one end inside → `return None`** (`326-327`). A sub-edge with one end in the block and the other elsewhere → `return None` (`336-338`).
- Rest part (`364-486`): every other node, plus one **proxy** per block: a plain `Shape::Rect` node with an empty label and the block's subgraph id (`proxy_id`, `230-236`), placed where the block's first member was in node order. A sub-edge between a block's frame and a node or another block's frame becomes an ordinary edge to the proxy (`447-459`). A block nested in a non-block subgraph makes its proxy a member of that subgraph (`407-413`).

### 3.3 `scene` (`compound.rs:668-698`)

1. Each block: `scene_labelled(part.g, …)` — the full pipeline including its own `finish` — then read its top-level frame (`frame`, `540`).
2. Rest: `scene_labelled` with each proxy sized to its block's frame.
3. Each block scene is shifted into its proxy's box (`691-694`).
4. `join` (`603-660`): nodes back to graph order; each graph edge from the part that drew it; edges-to-proxies renamed to the sub-edge's ids; sub-edges in graph order (`sub_slots`, `572-596`, by ids in order); frames with `depth` recomputed in `g` and sorted outermost first. **Returns `None` if any graph node or edge was drawn by no part** (`649-650`).
5. `refit(&mut out, Mode::Static, …)` (`696`). There is no joint edge pass over the joined scene.

### 3.4 `route` and `route_partial`

- `route` (`704-735`): each block `route_labelled` for the host's centres; its frame as wrapped round its members becomes the proxy's centre and size; then the rest is routed; `join`; `refit(Routed)`.
- `route_partial` (`816-877`): `part_base` (`741-808`) cuts the base scene into per-part bases (needs every part edge present in the base); each block is `route_partial_with`; then the rest with `sized = (proxy, frame size, resized?)` so a proxy whose frame moved or resized counts as moved (`858-873`); `join`; `refit`. Falls back to `route()` when anything does not line up (`scene.rs:4909-4912`).
- Call sites: `scene.rs:3058`, `4419`, `4909`.

### 3.5 Preconditions and consequences

- Each part is finished alone: ports, ortho gaps, router, labels and the 400-edge cap are per part (README:269, `layout.rs:3997-4001`).
- A block is one node on one layer of the rest. The rest's long edges get dummy columns beside it and the rest's router treats it as a node, i.e. a hard obstacle. This is exactly the "edges in the gaps between blocks" behaviour, but only for edges that end on the block's frame.
- Fork/join bars are fitted per part across the part's own flow (`scene.rs:3057`, test `compound.rs:1085`).
- Tests: `compound.rs:986-1191`. `contract` (`928-982`) is a compact statement of the scene contract.

### 3.6 Precisely what stops it handling an edge across a border

1. The rule itself: `!external[s]` (`182`), which is mermaid's rule for when a `direction` holds.
2. `split` aborts on such an edge (`326-327`, `336-338`).
3. The data model of a split has no place for it: a part edge is `EdgeOf::Edge(i)` or `EdgeOf::Sub(j)`, drawn whole by one part (`52-58`); `join` needs each graph edge from exactly one part (`625`, `649-650`); `part_base` needs the same of the base (`778-790`).
4. The proxy carries no information about the inside: no port position for "the member this edge is really going to", so the rest cannot aim an edge at the right spot of the frame, and the block cannot continue it to the member.
5. The block is laid out before, and without, the rest: its ranking and ordering cannot put the members that link outwards on the side they link to.
6. No stage sees both halves: port spacing on the frame, the ortho gaps, crossings, label placement and the 400-edge cap are all per part.
7. `route()` and `route_partial()` cut the graph the same way, so they would have to stitch the same halves on every drag frame, inside the drag budget.

---

## 4. The interactive contract for clustered diagrams

### 4.1 What a `Scene` guarantees

- `Scene::nodes[i]` describes `Graph::nodes[i]`, with ids (`scene.rs:95-99`).
- The first `g.edges.len()` scene edges are index-parallel with `g.edges`; sub-edges follow in order, a sub-edge with an end on an empty (frameless) subgraph is skipped (`scene.rs:130-134`; matching by ids in order in `edgepass::ctx`, `edgepass.rs:121-125`, `205-219`; `compound::sub_slots`).
- `SceneEdge::from` / `to` are node ids, or the subgraph id for a box end (`scene.rs:138-142`).
- `Scene::clusters`: one per non-empty subgraph, top-left `(x, y)`, `depth` = nesting, sorted outermost first (`scene.rs:190-205`, `clusters_of` `5542-5558`).
- Waypoints are corner-rounded, densely sampled paths a host splines through (`scene.rs:161-170`, PLAN invariant 2).
- Tests: `scene_carries_stable_identity_and_stays_index_parallel` (`scene.rs:11272`), `subgraph_scene_wraps_members_and_nests` (`12570`), `empty_and_nested_empty_subgraphs_stay_finite` (`12607`), `assert_clean_clusters` (`11489`, used by a dozen tests): sibling frames never intersect, every node lies inside exactly the frames of its chain and overlaps no other, labels cover no node, every edge ends on its target's border.

### 4.2 `route`, `route_sized`, `route_partial`

- `route(g, centres)` (`scene.rs:4363`) keeps positions exactly as given, never normalises. `route_sized` accepts invalid sizes (`4368-4384`).
- `route_built` (`4435-4819`) knows only the graph, the centres and the sizes:
  - back-edges by geometry: target wholly upstream of the source by ≥ `FLOW_MIN_GAP` 12 px → a lane or a direct return (`route_backs`, `8097-8277`); obstacles scoped to the innermost box holding both ends;
  - a forward edge whose ends lie in different top-level boxes (a top-level node counts as its own box) that are cleanly stacked along the flow, at least 32 px apart, gets a 4-point channel (`cross_cluster_route`, `4638-4655`, `8400-8422`); every other forward edge is a free curve for `ortho` and the router to redraw;
  - **frames are derived from the member positions**: `route_clusters` = `cluster_raw_boxes` (`5389`), or `route_lane_clusters` which also grows a frame over the back-edge lanes kept inside it (`4679-4687`, `5404`);
  - sub-edges against the current frames (`4693-4795`).
- `route_partial(g, centres, base, base_centres)` (`4870`, `4889-5149`):
  - falls back to `route()` if the base does not line up: node/edge identity, kinds, labels (`same_graph`, `4929-4942`) or **`clusters_line_up`** (`4958`, def `5277-5305`): one frame per non-empty subgraph with the same id, title and depth, holding every member's box (0.5 px slack) and **no other node's centre**;
  - a frame holding a moved node is redrawn as `route()` draws it; every other frame keeps its base box (`4980-5017`);
  - edges on a moved node or on a regrown frame are redrawn; edges near the change are kept but may be rerouted; the rest are fixed obstacles (`5051-5136`);
  - if more than half of the edges would be redrawn, the frame is a plain `route()` (`PARTIAL_ROUTE_SHARE`, `5139`).
- Tests: `route_partial_keeps_unmoved_edges_and_reroutes_moved` (`11303`, two subgraphs with cross edges), `route_partial_falls_back_when_only_subgraphs_changed` (`11342`), `route_partial_matches_route_on_the_dragged_node` (`11367`), `route_partial_falls_back_after_a_label_or_style_edit` (`11425`), `route_partial_hands_a_mostly_redrawn_frame_to_route` (`11445`), `dragging_a_member_pulls_its_cluster_along` (`12634`), `a_lane_stays_in_its_frames_in_route_and_on_a_drag_too` (`14726`), `compound::route_and_drags_lay_a_block_out_as_the_scene_does` (`compound.rs:1112`), and `tests/quality.rs` `a_one_px_drag_leaves_unrelated_edges_put` (`983`, `DRAG_JUMPS_MAX` `967`).

### 4.3 Is `scene()` equal to `route()` at the scene's own positions?

No, not in general.

- Frames: static frames are widened over inner channel lanes, `route()` frames are not. Largest frame difference between `scene()` and `route(scene centres)`: analog 71 px (STG 634 vs 563 wide), dense-td 129, dense-lr 326, bench-75 617, the notes fixtures 7.5-142; 0 on advanced, subgraph, state, fanout-frames.
- Edges on the analog: 19 of 27 have an end more than 1 px apart (worst 164 px), because the static scene follows dummy chains and `route()` does not.

What is pinned as equal or alike:

- counts and identity (`render.rs:286-293`, `scene.rs:11206-11210`, `11286-11290`);
- a member's edge to its own box, in both (`render.rs:329-357`);
- end glyphs of sub-edges (`subgraph_edges_keep_their_glyphs_in_scene_and_route`, `scene.rs:11010`);
- back-edge lane sides (`nested_lanes_take_the_same_sides_in_scene_and_route` `scene.rs:14999`, `route_lanes_take_the_static_side_and_no_leg_slides_into_a_node` `15151`), self-loop side;
- note links (`note_links_match_between_scene_and_route` `16712`, `note_links_hold_their_ends_and_corners_in_scene_route_and_drag` `16564`), fork/join branches (`fork_join_branches_drop_straight_and_drags_agree` `11733`), state twin combs;
- a B9 block's frame within 0.5 px (`compound.rs:1118-1122`).

The routed drawing is gated on its own: `quality::measure_source` measures `scene()` and `route(scene centres)` (`quality.rs:486-513`) and every gate in `tests/quality.rs` has a static and a routed value. The playground shows the routed drawing after the first drag.

### 4.4 Hit-testing and fold

- `hit_test` (`scene.rs:257`): node, then edge, then cluster. `cluster_at` (`314-329`): deepest frame containing the point, the smaller one at equal depth. `hits_in_rect` (`337`) lists frames whose box overlaps. All read `Scene` only; they need frames that nest properly and siblings that do not overlap.
- `edge_at` follows the painted stroke; `edge_picking_matches_the_painted_strokes_on_every_fixture` (`scene.rs:10822`) walks every `tests/fixtures/quality` and `examples` file, so it runs on whatever B12 draws.
- `fold::scene_compact` refuses any graph with subgraphs or sub-edges (`FoldSkip::HasSubgraphs`, `fold.rs:246-248`) and returns `scene()` unchanged (asserted for a block at `compound.rs:1106-1108`).
- `to_mermaid` works on `Graph` only (`emit.rs:125-128`); layout does not touch it.
- The `advance` module, class and ER scenes do not use subgraphs: class/ER build synthetic graphs with none and call `scene_sized` / `route_sized` (`class.rs:194`, `220`; `er.rs:155`, `162`).

### 4.5 Perf budgets (PLAN.md invariant 1)

Per file: worst `drag_ms` and `route_ms` ≤ max(1.25 × baseline, baseline + 1.0 ms); `route_partial` < 6 ms; static `scene_ms` ≤ 2 × baseline. Tool: `$S/tools/perf.sh` (`examples/quality.rs --drag` over the 14-file flow corpus; `drag_ms` is the worst `route()` frame of `quality::drag_frames`).

Files with subgraphs, baseline → 0.38.0 (`runs/baseline/perf.tsv`, `runs/rel038/perf.tsv`), and the limit:

| file | scene_ms | limit | route_ms | limit | drag_ms | limit |
|---|---|---:|---|---:|---|---:|
| advanced | 0.71 → 1.28 | 1.42 | 0.72 → 0.91 | 1.72 | 1.61 → 2.00 | 2.61 |
| subgraph | 0.08 → 0.16 | 0.16 | 0.05 → 0.07 | 1.05 | 1.04 → 0.60 | 2.04 |
| fanout-frames | 2.19 → 2.19 | 4.38 | 0.34 → 0.30 | 1.34 | 2.44 → 2.43 | 3.44 |
| dense-lr | 23.40 → 19.06 | 46.8 | 6.42 → 4.28 | 8.03 | 14.21 → 9.70 | 17.76 |
| dense-td | 25.34 → 25.43 | 50.68 | 7.36 → 6.56 | 9.20 | 14.24 → 12.55 | 17.80 |
| bench-75 | 30.97 → 39.44 | 61.94 | 12.61 → 9.07 | 15.76 | 20.22 → 17.03 | 25.28 |

`subgraph.mmd` is already at its static limit and `advanced.mmd` within 10 %. The timings are noisy on this box. README:857 also publishes "about 3 ms a frame, 6 ms at worst" for `route_partial` on bench-75.

---

## 5. What pins today's layouts

### 5.1 No stored snapshots

There is no SVG golden file. "Byte-identical" is asserted only as determinism (`scene.rs:11251-11268`, `quality.rs:502-503`, `tests/quality.rs:161`) and for the default theme (`tests/theme.rs`).

### 5.2 Geometry tests

Tests whose body builds a subgraph or a composite, or reads a clustered fixture (a text scan, so approximate), by file: `scene.rs` 65, `layout.rs` 11, `router.rs` 8, `compound.rs` 4, `ports.rs` 3, `edgepass.rs` 3, `casing.rs` 3, `labels.rs` 2, `ortho.rs` 2, `lanes.rs` 2, `render.rs` 2, `tests/quality.rs` 11 (parser and emit tests, 39 more, do not depend on layout).

The ones that state a layout rule for subgraphs:

| test | rule |
|---|---|
| `scene.rs:11186` `cross_cluster_layout_separates_boxes_and_stays_contained` | two subgraphs joined by an edge stack along the flow |
| `scene.rs:11566` `sibling_frames_never_intersect`, `11587` `walls_of_a_box_are_straight…` | `assert_clean_clusters` on five shapes incl. LR, RL, nested |
| `scene.rs:11600` `edges_into_a_subgraph_rank_its_members_below_the_source` | `root --> box` puts the whole box below root; box as source above its target |
| `scene.rs:11629` `state_composite_keeps_flow_order` | Idle above the composite, Done below |
| `scene.rs:12048` `hub_fed_leaves_in_a_box_pack_into_a_grid` | 4 hub-fed leaves in a box fill 2 × 2; 3 edge-less members stay one row |
| `scene.rs:12102` `wide_sibling_boxes_stack_instead_of_a_ribbon` | two wide unconnected boxes end up one above the other |
| `scene.rs:12129` `largest_box_is_centred_among_its_siblings` | the big box sits between its two small siblings |
| `scene.rs:12225` `stacked_nested_frames_clear_each_other_in_every_direction` | BT and deep nests |
| `scene.rs:12249` `back_edge_inside_a_box_bows_beside_it…` | a lane inside P never crosses sibling W |
| `scene.rs:12284` `edge_to_a_far_box_routes_round_nodes_and_stops_at_its_frame` | a sub-edge ends on the frame's top side, crosses no node |
| `scene.rs:12327`, `12357` | box-to-box ranking; a box that is one cycle still has an entry and an exit |
| `render.rs:295-357` | advanced.mmd: CI above Platform; `Reg --> Platform` onto its top side; `Rollback --> Platform` straight up into its bottom side; `Dev --> CI` drawn to `Build` |
| `layout.rs:5605` `a_hub_fed_column_of_frames_stays_one_column` | fanout-frames' shape |
| `layout.rs:5648`, `5694`, `5731`, `5764`, `5796` | `seat_box_sources` behaviour |
| `layout.rs:5833` `a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` | the wrap of `narrow` |
| `layout.rs:6112` `a_labelled_gap_between_composites_holds_its_labels` | label room between two stacked composites |
| `compound.rs:986-1191` | B9: direction rule, nested directions, bars, route/drag agreement, harness |
| `scene.rs:14708`, `14726`, `14814` (`a_lane_between_two_members_stays_in_their_box`, `a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`, `a_rerouted_edge_between_members_stays_in_their_frame_in_route`) | own-frame containment |

### 5.3 Quality gates and ratchets (`tests/quality.rs`)

Fixtures: `tests/fixtures/quality/{back-edge, bench-75, dense-lr, dense-td, fanout-frames, wide-labels}.mmd`; every fixture needs a ceiling row (`263`).

| gate | where | value (static, routed) |
|---|---|---|
| `CEILINGS` (edge_node_pairs, edge_frame_pairs, defects) | `14-58` | bench-75 (0,4,4) (0,0,2); dense-lr (0,13,15) both; dense-td (0,26,29) (0,29,30); fanout-frames all 0 |
| hard zeros in `check` | `111-135` | `edge_node_x`, `shared_ports`, `arrow_on_start`, `wiggles`, `frame_side`, `node_overlap`, `node_frame`, `frame_overlap` |
| `GAP_FLOORS` (min_edge_gap) | `64-109` | bench-75 14.7 / 10.6; dense-lr 12 / 12; dense-td 12.1 / 12; fanout-frames 40 / 40 |
| `BORDER_LEN_MAX` | `245` | 1 395, 1 895 |
| `LBL_EDGE_MAX`, `LBL_HARD_MAX`, `lbl_crowd` | `296`, `304`, `322` | (6, 4), (0, 0), 0 |
| `MAX_ASPECT`, `WIDE_LABELS_CENTER_OFF` | `353`, `372` | 12.7, 0.5 |
| `DASH_EDGES_MAX`, `XGAP_MAX` | `402`, `415` | (0, 0), (9, 12) |
| `CHANNEL_MAX`, `EDGE_CHIP_MAX` | `481`, `499` | overlap/tight/hug lengths; (0, 3) |
| `DEFECT_BASELINE`, `INK_MEAN_MIN` | `730`, `754` | incl. `own_frame_x` 0, `x_drawn` 428 / 494, `band_len` 71 360 / 69 110; ink ≥ 0.095 / 0.108 |
| `DRAG_JUMPS_MAX` | `967` | dense-lr 0, dense-td 5, bench-75 16, notes/{spread-face, fork-twins, near-drag} 0 |
| declaration-order robustness | `1038`, `1098-1142` | dense-td width within 10 % when the `n45` line moves; dense-td, dense-lr, bench-75 width within 10 % under six edge-line shuffles |

`frame_side` (`quality.rs:1649-1666`) counts an edge end on a frame whose nearest frame side lies across the flow (left/right in TD). `wiggles` (`1626-1647`) counts an edge that goes both forwards and backwards along the flow by more than the tolerance.

Other fixture sets: `tests/fixtures/notes/*.mmd` (state diagrams with composites, used by the note-link and drag-jump tests), `tests/fixtures/outline/*.mmd` (no subgraphs).

### 5.4 README and docs

- `README.md:37`, `49`, `56`: `docs/img/compare/{advanced, state, subgraph}.png` show mermaid 12.2.0 next to flowmaid; all three inputs have subgraphs. `docs/ex-subgraph.svg` is used by `docs/index.html:269`.
- Prose that states today's behaviour and would need a rewrite: Subgraphs (`README.md:282-289`), How it works: Layers, Ordering, Narrowing, compound (`797-806`), the guarantees list (`811-817`), the benchmark table rows for the six clustered flowcharts and state (`825-851`), the drag numbers (`857`).

### 5.5 Which inputs have subgraphs, and which have edges across a border

From `b12probe summary` (cross = node-to-node edges whose ends have different subgraph chains; sub = edges to a whole subgraph; block cycle = some top-level subgraph reaches itself through other top-level units; B9 = laid out by `compound.rs` today).

| input | kind | subgraphs (nested) | cross | sub | block cycle | B9 block | changes with B12 (inferred) |
|---|---|---|---:|---:|---|---|---|
| `b12/in/infra-overview.mmd` | flow | 5 (0) | 18 | 3 | yes | – | yes |
| `examples/advanced.mmd` | flow | 3 (1) | 4 | 4 | yes | – | yes |
| `examples/subgraph.mmd` | flow | 2 (1) | 4 | 0 | no | – | yes |
| `examples/state.mmd` | state | 1 (0) | 0 | 2 | yes | Validating | only the rest round the block, if block-level ranking changes |
| `quality/bench-75.mmd` | flow | 5 (0) | 69 of 75 | 0 | yes | – | yes, heavily |
| `quality/dense-lr.mmd` | flow | 11 (6) | 28 | 2 | yes | – | yes |
| `quality/dense-td.mmd` | flow | 10 (7) | 30 | 3 | yes | – | yes |
| `quality/fanout-frames.mmd` | flow | 6 (0) | 0 | 6 | no | – | only if blocks without border edges change too |
| `notes/fork-twins.mmd` | state | 4 (2) | 3 | 7 | no | – | yes |
| `notes/near-drag.mmd` | state | 1 (0) | 1 | 2 | yes | – | yes |
| `notes/nested-composite.mmd` | state | 3 (2) | 7 | 6 | no | – | yes |
| `notes/routed-around.mmd` | state | 3 (1) | 1 | 9 | yes | – | yes |
| `notes/spread-bottom.mmd` | state | 4 (1) | 2 | 11 | yes | – | yes |
| `notes/spread-face.mmd` | state | 5 (2) | 6 | 13 | no | – | yes |
| `notes/straight-link.mmd` | state | 1 (0) | 0 | 4 | yes | – | sub-edges only |
| `notes/wide-frames.mmd` | state | 3 (2) | 1 | 10 | yes | – | yes |

`cmp/corpus/*.mmd` repeats advanced, bench-75, dense-lr, dense-td, fanout-frames, state and subgraph; its other files have no subgraphs. Without subgraphs (must not change): demo, fanout, long-labels, lr, shapes, stress, back-edge, wide-labels, the outline fixtures, and every class / ER input.

In the state fixtures many "cross" edges are note links and transitions between a composite's states and outer states; state diagrams also carry many sub-edges (transitions to a composite itself).

---

## 6. Constraints and preferences for a block layout

### 6.1 Hard constraints

| # | constraint | tied to |
|---|---|---|
| H1 | One `SceneEdge` per graph edge at its index, sub-edges after in order, nodes index-parallel, frames outermost first with `depth` = nesting in `g`. An edge laid out in pieces by several parts must still come out as one edge. | `scene.rs:95-99`, `130-134`, `190-205`; `edgepass.rs:121-125`; `compound.rs:603-660`, `928-982`; test `scene.rs:11272` |
| H2 | `route(g, centres)` and `route_sized` see only the graph, centres and sizes. Anything the block layout decides that is not recoverable from those (which inter-block edges run against the flow, channel positions) does not exist during a drag. Block membership and "is a block" can be recomputed from `g` (as `compound::blocks` does). | `scene.rs:4363-4428`, `4435`; `route_backs` `8097`; `edgepass.rs:221-230` |
| H3 | Frames in `route()` are `cluster_raw_boxes` of the members (+ home lanes). If `scene()` draws a frame differently (padding for a title, for ports, for inner channels), the frame snaps on the first px of a drag, and `route_partial` must still pass `clusters_line_up`: every member's box inside, no other node's centre inside. | `scene.rs:5346`, `4679-4687`, `5277-5305`, `4980-5017` |
| H4 | Drag and static budgets of section 4.5. `subgraph.mmd` (0.16 ms) and `advanced.mmd` (1.28 ms) have almost no static headroom; `route_partial` < 6 ms on bench-75. A per-part pipeline pays the fixed cost of `finish` once per part. | PLAN invariant 1; `runs/baseline/perf.tsv` |
| H5 | Diagrams without subgraphs, and class / ER, byte-identical. The flat path is `scene_flat` (`scene.rs:3083`) and `layout_core(…, None, &[])`; shared code in `layout.rs`, `ortho.rs`, `router.rs`, `ports.rs`, `labels.rs` must stay as it is for them. | `scene.rs:3064-3068`; `class.rs:194`; `er.rs:155`; user decision |
| H6 | Determinism: byte-identical SVG across runs; ordered maps only where iteration order reaches the output. | `tests/quality.rs:161`; `layout.rs:3185-3187`, `3255-3257` |
| H7 | Gated at zero on every quality fixture, both modes: no edge through an unrelated node, no shared ports, no wiggle along the flow, no frame end on a side across the flow, no node/frame overlap, no sibling frame overlap. Edges entering a block's frame from the side, or an inter-block edge that first runs with the flow and then against it, would trip `frame_side` / `wiggles` as defined today. | `tests/quality.rs:111-135`; `quality.rs:1626-1666`; `assert_clean_clusters` `scene.rs:11489` |
| H8 | An edge between two members of a subgraph stays inside its frame, in `scene()`, `route()` and on a drag. | README:815; `DEFECT_BASELINE` `own_frame_x`; tests `scene.rs:14708`, `14726`, `14814` |
| H9 | The B9 rule keeps holding: a subgraph with its own `direction` and no border edge is laid out in that direction; one with a border edge takes the direction round it (mermaid's rule); unlinked members of a `direction` block stay one row across the flow. | `compound.rs:134-189`, tests `1019-1057`; README:284-287 |
| H10 | Sub-edge semantics: an edge to a box ends on the frame side facing along the flow, clear of the title chip; member → own box is drawn to the box's entry member in `scene()` and `route()`; box → own member is a stub; an edge into a box ranks the whole box after its source. | `scene.rs:3608-3743`, `4075`; `render.rs:319-357`; tests `scene.rs:11600`, `12284`, `12327`, `12357` |
| H11 | Each scene handed to `edgepass::finish` is treated as one flow direction (`g.direction`) with global row gaps and one 400-edge cap. Side-by-side blocks whose inner rows do not line up leave `ortho` no shared gaps; a block flowing another way needs its own finish. | `edgepass.rs:227`; `ortho.rs:479-506`, `180-185` |
| H12 | The static scene's edge builders read a global layer index (`Placed::layer`) and per-edge dummy paths: `static_lanes` / `back_lanes` decide back-edges by layer, `scene_from_layout` picks curve vs chain by layer. A composed layout must supply consistent layers or build per part. | `scene.rs:3097-3144`, `3186-3197`; `layout.rs:4427-4450` |
| H13 | Title chips are never struck: `title_head` is an obstacle for ports, ortho, router and labels and titles are painted last. A title is cut with an ellipsis only when it would run into a sibling frame or off the canvas; compact blocks make that collision likelier unless a block's size accounts for its title. | `scene.rs:2885-2948`, `6477-6492`; test `scene.rs:12539`; PLAN invariant 4 |
| H14 | Title strip on the screen top in all four directions; nested pads and strips must clear each other (`FRAME_GAP`), BT included. | `scene.rs:5378-5383`; `layout.rs:3212-3254`, `4083-4088`; test `scene.rs:12225` |
| H15 | Public API stays: `layout::layout_clustered` / `layout_sized` / `LayoutResult` / `Placed` are `pub`; scene structs are `#[non_exhaustive]` (adding fields is fine). Zero deps, MSRV 1.75, wasm32, clippy `-D warnings`. | `layout.rs:28-50`, `1503-1520`; PLAN invariants 2, 3 |
| H16 | Block-level cycles must be handled deterministically and stably under declaration order: 11 of 16 clustered inputs have one, and the width of dense-td / dense-lr / bench-75 may move at most 10 % when edge lines are shuffled or a declaration moves. | section 5.5; `tests/quality.rs:1038-1142` |
| H17 | Gates may be re-baselined only where the change is intended and said so; never loosened to hide a regression, no test deleted or ignored. Nearly every ratchet of section 5.3 will move, since bench-75, dense-td and dense-lr are 80-90 % border edges. | PLAN invariant 5 |

### 6.2 Soft preferences

| # | preference | tied to |
|---|---|---|
| S1 | The routed drawing should keep what the static one gains. Compact frames do (H3); channels between blocks do not exist in `route()` today (section 2.5), so a block-aware rule there is what keeps the look after the first drag. | `quality.rs:486-513`; section 2.5 |
| S2 | Reuse `compound.rs`: `Split` / `Part` / proxies / `join` / `part_base` and its `route` and `route_partial` already solve "block as a node" for all three entry points, with blocks as hard obstacles and dummy columns round them. The missing piece is an edge that ends inside a block (section 3.6). | `compound.rs` |
| S3 | Keep `grid_pairs` packing and the wrap/stack ideas where they still apply inside or between blocks, or replace them with something that passes the same tests. | `scene.rs:4172`; tests `scene.rs:12048`, `12102`; `layout.rs:5605`, `5833` |
| S4 | Declaration order as the tie-break; equal boxes keep their order; the largest unit near the middle. | `layout.rs:4707`; tests `scene.rs:12129`, `12149` |
| S5 | Labels keep a slot the layout reserved (a sized dummy or a grown gap) rather than being found a place afterwards, and sit near their edge's ends or middle, not mid-way along a long sideways run. | `layout.rs:3132-3147`, `4097-4145`; `ortho.rs:2220` |
| S6 | Compact canvas and ink: mean ink floor 0.095 / 0.108 over the fixtures, aspect ≤ 12.7; stacking blocks trades breadth for depth and adds inter-block gaps. | `tests/quality.rs:372`, `754` |
| S7 | Drag stability: a 1 px drag must not throw unrelated ends more than 20 px. | `tests/quality.rs:967-1036` |
| S8 | New measures for what the user complained about, since no current gate sees it: frame area against content, sideways run length, label-to-end distance, edges through foreign frames. | section 2.4 |
| S9 | README images and prose (section 5.4), CHANGELOG entry, and a statement of the new rule next to the B9 rule. | PLAN invariant 7 |

### 6.3 Open points a design has to answer (not decided here)

- Which inter-block edges run against the flow when the block graph is cyclic, and how `route()` reproduces that choice from positions alone (today: it does, by geometry, section 2.5).
- Whether a nested subgraph with edges to its parent's members (`subgraph.mmd`: `Users --> Bus`; `advanced.mmd`: `Gate ==> Q`, `W1 & W2 --> Users`) is a block inside a block, and how an edge crosses two borders.
- Whether a top-level free node (the analog's three actors) is a unit of the block-level layout like a block.
- Where the port of a border-crossing edge sits on the frame, and whether the frame end counts as a "frame side" for `frame_side`.
- Whether `fanout-frames` and `state.mmd` (no border edges) change at all.
