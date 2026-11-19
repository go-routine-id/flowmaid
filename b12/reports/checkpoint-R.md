# Checkpoint R: `route()`, `route_sized()`, `route_partial()` in block mode (B12.13), fix round 1

Workspace only, never committed. `B` = the b12 workspace directory, `S` = the scratchpad, `W` = `B/r3c` (this
round's work directory), `T` = `B/tools-a2b`, `R` = `/home/user/flowmaid`, branch `ccr-ccc6d89a-5cxd7e`.
Package `b12a3-route`, fix round 1 (2026-11-19, ~03:00-05:00 UTC), built on the lead's decision of 2026-11-19
~02:45 UTC (SPEC section 0, "Lead decision after b12a3-route's second implementation"). The second run's report
is kept as `B/reports/checkpoint-R.second-run.md`, the first run's as `checkpoint-R.first-run.md`.

**Result.** With the lead's option A (route-from-layout on wherever the block switch is on), checkpoint R's own rows
are met in the suite-on run except the two the lead moved to step 4 (`route_partial` on dense-td; the full corridor
builder R2/R3/R4/R8 with its 4.4 tests) and the routed rows that now equal their static twins (the declared trade of
ruling 1, section 3). Every `route()` / drag budget of 4.6 holds on three interleaved runs (section 5). The drag
paths are no worse than the start (`157117e`) on the verifier's repros and drag sums (section 4).

Commit of this round (no amend, no rebase): **`d99c073`** `fix(blocks): route() of a block layout starts from the
layout; a drag follows a layout column only where that adds nothing (B12.13, fix round 1)`. No README, CHANGELOG
or version change.

## 1. What ran

- Start `157117e`; baselines `T/base-157117e` (the three checks), `S/runs/pre-b12a3-route` (default render).
- The three checks from the commit: `T/run-checks.sh T/cand-d99c073 --commit d99c073`, `T/compare.sh
  T/base-157117e T/cand-d99c073` (report `T/cand-d99c073.cmp`); identical verdict lines to the working-tree run
  `W/runs/fr1-c`.
- Drag probes (block mode, `route_partial` from the static scene, every node by (1,0), (60,25), (-150,80)):
  `W/probe-{start,head}` (`dragq`, `dump`, `worstdrag`, `coldwarm`, `onedrag`), start built from `git archive
  157117e` in `W/start`. Sums `W/dq15-*.txt` (the verifier's 15 inputs), `W/dq18-*.txt` (those + the public
  fixture BT/RL + nested-chain), `W/dqall-*.txt` (every `tests/fixtures/blocks` file); compare `W/dqcmp.py`.
- Perf: `B/r3b/perfrun.sh` (waits for no rustc and load < 1.5, records the box) over four binaries interleaved,
  three rounds: `S/runs/b12a3fr2-{on-head,def-head,on-base,def-base}-{1,2,3}`; tables `W/perftab-{head,base}.md`.
  `route_partial` worst frames: `W/rp-{head,start}` (the `fixture_drag_perf_guard` loop plus the analog), four
  rounds for head (block mode and default build) and two for the start, `W/rp-all.txt`.
- Default build: `S/tools/guard.sh` GUARD: ALL GREEN (`W/guard-2.txt`, the off-identity tests and rustdoc
  included); `S/tools/render.sh b12a3-route-fr1`: the 25 corpus SVGs byte-equal to `S/runs/pre-b12a3-route`.

## 2. What was built (each item of the verifier's list and the lead's decision)

1. **Option A, one switch** (finding 1, 8): `--cfg flowmaid_route_layout`, `levels::with_route_from_layout` and
   the public `quality::with_route_from_layout` are removed. `route_fitted` starts from the static layout's
   drawing whenever the part is drawn in block mode (`route_from_layout`). One internal switch again
   (`flowmaid_blocks`, `quality::with_blocks`), as spec 4.1 and section 5 ask.
2. **`scene()` hands its drawing to the memo** (finding 7, lead item 4): `scene_labelled` keeps a block layout's
   drawing in `LAID_SCENES` (`keep_laid_scene`, same key as `route()`'s: graph, fitted sizes, labels, start count;
   one entry per key, the oldest dropped past `LAID_SCENES_MAX` = 8). Test
   `scene_hands_a_block_layouts_drawing_to_route`.
3. **A follow that adds nothing** (finding 4, lead item 3): `refuse_costly_follows` weighs each followed path
   against the edge's base path in four counts (`drag_path_cost`: proper crossings, crossings within its fan,
   nodes it comes within `NODE_CLEARANCE` of, frames it enters that hold neither end). A followed path that adds
   to any count is drawn as the release before redrew it (the builders, then the stages, not held). Two
   variants were measured and dropped: judged against the builders' raw path (hub5's `bill` stayed followed: the
   raw builder curve crossed more than the follow, the staged re-plan crossed nothing), and refusing whenever the
   follow costs more than the raw builder path (1 px drags lost their follows). Run on corner lists with
   precomputed boxes: the first version cost 8.7 ms on dense-td's hub `n16`, the final one 0.4 ms.
4. **Frame room** (finding 5, lead item 3): a redrawn frame keeps the layout's box side by side only where the
   added strip takes in no node it does not hold, no frame that neither holds it nor lies in it, and no path this
   call draws afresh; elsewhere the side the members give it (F1, F2). Test
   `a_redrawn_frame_keeps_its_room_off_what_it_does_not_hold`.
5. **Labels and corners** (finding 6): a followed label stays on its leg at the same share, its chip a rounding
   clear of a turning corner or no nearer than the base had it (`followed_edge`; test
   `a_followed_label_stays_on_its_leg_clear_of_the_turn`); every path a block-mode drag draws gets a rounding
   point back where the ports stage left one farther than the router's rounding (`round_corners_evenly`, which
   inserts points only, so glyph stubs stay; test `a_followed_path_is_rounded_as_the_router_rounds_one`).
   The verifier's `v2-lr-styles` S3 1 px drag: corners now 14 / 14 px (was 16.8 / 14). The public fixture's
   `place order` (drag CART by (120, 40)): the static scene itself puts that chip 6.6 px from the corner (base
   label (282.6, 897.3), corner (276, 898)); the follow keeps it as near and no nearer
   (`W/pics/ov-cart.final.png`). Moving it further would make a 1 px drag throw it 50 px; it belongs to the
   static label rows (A2).
6. **Resized nodes** (finding 9): a node the host moves and resizes in one call carries its followed ends onto its
   new box on the same side, as far from its centre as before where the side reaches (`node_end_follow`). Test
   `a_resized_node_carries_its_followed_ends_onto_its_new_box` (fails without the change: `a2 -> b2` ended at the
   old border).
7. **A pan through `route_partial`**: its fallback goes through `route()` (which moves the layout's drawing)
   instead of drawing afresh; keeps `route_partial_hands_a_mostly_redrawn_frame_to_route` green now that route()
   starts from the layout.
8. **Cost of `levels::augmented` + `block_mode` per drag frame** (B12.13's check, finding 8): measured with a
   temporary test (not committed): dense-td 0.008 ms, dense-lr 0.008, bench-75 0.002, public fixture 0.004 ms per
   call, against a 1 px `route_partial` of 1.65 / 1.09 / 1.03 / 0.62 ms. Each drag frame calls it once (twice on a
   fallback).

Not built here (the lead's decision, item 5): the full corridor builder (2.9 R2 classes by geometry, R3 corridors
per band with one slot order per gap, fan nesting and first legs for the real port spread, R4 foreign frames as
hard obstacles in `router::repair`, R8 corridor labels) and the 4.4 corridor tests. It moves to step 4
(`b12b-default`, checkpoint C) and must exist before the default is flipped. Also the own-box rules of 2.8
(B12.14, C).

## 3. The declared trade of ruling 1 (routed = static at the scene's centres)

At the scene's own centres `route()` now draws the static drawing: the routed SVG equals the static SVG on every
block-mode local input (78 of 95; the other 17 are not drawn in block mode and route afresh as before). Every routed
row worse than the start's routed drawing equals its static twin (lead item 2); the static column is unchanged
against the start on all 444 cells, 332 + 32 copies and the 95 static SVGs (byte-equal).

| check | worse than `157117e` (routed) | its static twin |
|---|---|---|
| cells | `perm-b-subgraphs-reversed lbl_crowd` 6 -> 8 (must <= 5) | static 8 |
| dirs, defects | notes.wide-frames-RL 1 -> 3; quality.dense-lr-LR / -RL 1 -> 2 | static 3 / 2 / 2 |
| dirs, x_drawn | blocks.self-member-RL / -TD 0 -> 1; blocks.sub-nested-self-LR / -RL 0 -> 1; notes.fork-twins-BT 9 -> 10, -RL 10 -> 13, -TD 5 -> 8 | equal (every listed copy) |
| dirs, lateral_x | blocks.interleave-BT, l5loop-TD, nested-BT, nested-wide-title-BT, title-hidden-TD; notes.spread-bottom-BT/LR/RL/TD: 0 -> 1 | equal |
| dirs, fan_x | notes.spread-face-LR / -RL 0 -> 1; notes.wide-frames-RL 1 -> 2 | equal |
| dirs, side_ends | the 8 public-fixture TD copies 3 -> 4; notes.fork-twins-BT 0 -> 1, -TD 0 -> 2; notes.wide-frames-LR / -RL 0 -> 1 | equal |
| dirs, min_edge_gap drop | 63 copies (list in `T/cand-d99c073.cmp`), among them the five below every floor: turned-blocks-LR 39.89 -> 1.59, sub-nested-self LR/RL 40 -> 6, BT/TD 40 -> 7 | equal |
| gates | `defect_baseline_gate` routed lat_wiggles 23 -> 33, lat_backtrack 3856 -> 10600, new band_pairs 203 > 141, band_len 91456 > 71020, end_kink 1 > 0 | static 33 / 10600 / 204 / 91602 / 1 (routed <= static) |
| gates | `block_fixtures_are_not_drawn_worse`: new `blocks/self-member routed: x_drawn = 1`, `hard zeros = 2` | static values (own-box edges, B12.14) |

Checked per copy (`W/rsdiff.py` over `dirs.tsv`): the routed verdict columns differ from the static ones only on
blocks.iso-lane / iso-lane-lr LR/RL (`min_edge_gap` 31.5 static, 40 routed) and quality.wide-labels-LR/RL
(x_drawn, fan_x 0 static, 1 routed); none of them is in block mode and none is worse than the start. The 19 routed
must-fail cells each equal their static cell (`W/runs/fr1-c/local/metrics{,-routed}.tsv`). These static rows stay
owned by `b12a2c-squeeze` (A2) and step 4 (C).

## 4. Checkpoint R's gate

| row of 4.1 R | value at `d99c073` (suite-on) | must | verdict |
|---|---|---|---|
| `fr_max` | 0.00 on every block-mode local input (analog 341.4 -> 0, v2 1306.6 -> 0, public 42.3 -> 0, perms 341-534 -> 0) and every block-mode copy; above 0.5 only on iso-lane-lr / iso-lane LR/RL, 2.0, not block mode, as at the start | <= 0.5 on the analog family and the public fixture | met |
| `ends_apart` | 0 on every block-mode input; local total 774 -> 1, copies 1772 -> 12 (iso-lane, wide-labels, as at the start) | not above the start's | met |
| `dfr_own` / `dfr_far` (1 px drags) | public fixture, analog, v2, bench-75, dense-td, dense-lr, fanout-frames, subgraph 1.00 / 0 (start: public 41.3, analog 341.4, v2 1307.6, bench-75 78.8, dense-td 50.9, dense-lr 98.0); test `a_one_px_drag_moves_a_block_layouts_frames_by_a_px_at_most` (public x4, hub5, cycle2, nested-cases) green. advanced 53.5 at start and now (not the analog family; a graph with `direction` blocks, split by the compound layout) | <= 1.5 analog family + public; dfr_far 0 | met |
| `DRAG_JUMPS_MAX` | `a_one_px_drag_leaves_unrelated_edges_put` passes suite-on (failed at the start) | as the starting release | met |
| routed hard zeros | = static: self-member 2 (own-box, B12.14), notes/wide-frames 2 (C); every other block fixture 0 | 0 | trade of ruling 1 (section 3) |
| 4.3 (h), local routed | 37 -> 19 must-fail, each equal to its static cell (section 3) | 0 | trade of ruling 1 |
| (d)/(f) routed | public LR/RL, infra-lr/rl/bt, hub5 lateral_x 0; public BT 3, cycle2 2, public permutations 2 (= static) | | trade of ruling 1 |
| DEFECT_BASELINE routed | = static column (section 3) | 4.5 | trade of ruling 1 |
| budgets of 4.6 | section 5 | 4.6 | met on three runs |
| `route_partial` | dense-td judged at C (lead item 6); others section 5 | < 6 ms, or vs the default build | section 5 |

The (h) rows (routed value at the start / now, must):

| input | xframe_runs | lateral_x | lbl_struck | lbl_crowd | crossings |
|---|---|---|---|---|---|
| analog | 0 / 0 | 9 / **1** | 2 / **1** | 6 / 4 (<= 7) | 26 / 15 (<= 24) |
| v2 | 0 / 0 | 5 / 0 | 5 / **3** | 7 / 4 (<= 8) | 22 / 14 (<= 20) |
| perm-a | 0 / 0 | 9 / **1** | 2 / **1** | 6 / 4 (<= 5) | 26 / 15 (<= 18) |
| perm-b | 0 / 0 | 9 / **2** | 3 / 0 | 6 / **8** (<= 5) | 30 / 18 (<= 18) |
| perm-c | 0 / 0 | 9 / **1** | 2 / **1** | 6 / 4 (<= 8) | 26 / 15 (<= 24) |
| perm-d | 0 / 0 | 9 / **1** | 2 / **1** | 6 / 4 (<= 7) | 26 / 15 (<= 24) |
| perm-e | 0 / 0 | 7 / **1** | 2 / 0 | 6 / 5 (<= 7) | 30 / 17 (<= 26) |
| public fixture | 0 / 0 | 2 / **2** | 0 / 0 | 2 / 2 (<= 5) | 10 / 6 (<= 11) |

Bold: still a must-fail, each equal to its static value (static A2 rows `lateral_x`, `lbl_struck`, `lbl_crowd`).

### 4.1 Drags against the start (lead item 3)

The verifier's repros, `route_partial` from the static scene (`W/pics/*.final.png`: base | 157117e | head):

| repro | 157117e | previous head (`0a3fd4d`) | `d99c073` |
|---|---|---|---|
| blocks/hub5, b1 by (60, 25) | x_drawn 0, fan_x 0, lateral_x 0 | x_drawn 1 (`bill` across `token -> invoice`) | x_drawn 0, fan_x 0, lateral_x 0 |
| examples/advanced, Q by (-150, 80) | x_drawn 0, edge_node_x 1, defects 3 | x_drawn 4, fan_x 2, Async Workers over Redis | x_drawn 0, edge_node_x 1, defects 3, Async Workers as at the start |
| examples/advanced, every node by (-150, 80) (sums) | defects 17, x_drawn 3, edge_node_x 2, fan_x 0 | x_drawn 11, fan_x 4 | defects 17, x_drawn 3, edge_node_x 2, fan_x 0 (= start); by (60, 25) = start; 1 px x_drawn 2 -> 0, fan_x 2 -> 0 |

Drag sums (block mode, every node by each offset), start -> `d99c073`:

| set | drag | defects | x_drawn | edge_node_x | fan_x |
|---|---|---|---|---|---|
| the verifier's 15 inputs | 1 px | 7 -> 0 | 392 -> 301 | 0 -> 0 | 33 -> 0 |
| | (60, 25) | 59 -> 50 | 408 -> 347 | 0 -> 0 | 42 -> 26 |
| | (-150, 80) | 267 -> 257 | 483 -> 454 | 17 -> 16 | 49 -> 49 |
| 18 inputs (+ public BT, RL, nested-chain) | 1 px | 9 -> 0 | 714 -> 574 | 0 -> 0 | 41 -> 0 |
| | (60, 25) | 73 -> 64 | 724 -> 630 | 0 -> 0 | 43 -> 26 |
| | (-150, 80) | 338 -> 325 | 824 -> 764 | 20 -> 19 | **60 -> 61** |
| every block fixture (73 files) | 1 px | 91 -> 62 | 1254 -> 1045 | 25 -> 25 | 43 -> 3 |
| | (60, 25) | 346 -> 325 | 1285 -> 1128 | 19 -> 19 | 38 -> 31 |
| | (-150, 80) | 875 -> 840 | 1454 -> 1346 | 84 -> 84 | 65 -> 60 |

(The verifier said "18 block inputs"; its sums 267 -> 271 and 49 -> 57 are exactly the 15 files of `verify/
b12a3-route/files.txt` plus blocks-overview-lr, nested and lr, which `W/dq-start.txt` reproduces byte for byte.)

Per-file lines still worse than the start (all others equal or better), with causes:
- v1-hubblocks 1 px x_drawn 7 -> 10, (60, 25) 9 -> 11: a 1 px drag now keeps the layout's drawing, whose crossings
  are the static ones (the start redrew the dragged node's edges); the trade of section 3 on drag frames.
- v3-state (60, 25) x_drawn 13 -> 14; blocks-overview BT `CACHE` and LR `MAIL` by (-150, 80) fan_x 0 -> 1 (the
  18-input fan line): in both an edge `follow_drag` itself refuses (end leg too short) is drawn by the builders and
  the stages and crosses a sibling kept on the layout's column; x_drawn on those frames 6 -> 7 and 8 -> 7.
- cycle2 (-150, 80) x_drawn 3 -> 4; nested-cases 21 -> 22 (far).
- b9-inside (60, 25) defects 2 -> 3, x_drawn 0 -> 3; b9-inside2 x_drawn 2 -> 3, fan_x 2 -> 3; lane-beside far
  defects 25 -> 26, edge_node_x 1 -> 2: compound graphs (a `direction` block) whose rest part is drawn in block
  mode. In b9-inside (s3 by (60, 25)) Pipeline keeps the layout's left room (static x 28; members' frame 57.5), the
  kept `ship -> request` is rerouted because Pipeline grew right over its column, and the router, which only
  prices a foreign frame (R4 not built), takes it through that room. R4 (moved to step 4) is what removes this.

## 5. Perf (4.6) and `route_partial`

Three interleaved rounds per binary: suite-on head (`on-head`, block mode, route-from-layout on), default head
(`def-head`), suite-on start (`on-base`), default start (`def-base`). Box: no rustc at any start, load 1.0-1.4
(`S/runs/b12a3fr2-*/box.txt`). `scene_ms / route_ms / worst drag_ms` (a full `route()` per drag frame); bold =
over the limit.

| file | limit | on-head-1 | on-head-2 | on-head-3 | on-base-1 | on-base-2 | on-base-3 |
|---|---|---|---|---|---|---|---|
| advanced | 1.42 / 1.72 / 2.61 | 1.08 / 0.01 / 1.86 | 1.13 / 0.01 / 1.79 | 1.11 / 0.01 / 1.70 | **1.51** / 0.94 / 2.36 | **1.64** / 0.96 / 2.55 | 1.14 / 0.59 / 2.20 |
| subgraph | 0.38 / 1.05 / 2.04 | 0.18 / 0.00 / 0.51 | 0.19 / 0.01 / 0.47 | 0.19 / 0.00 / 0.46 | 0.18 / 0.07 / 0.47 | 0.20 / 0.08 / 0.49 | 0.18 / 0.07 / 0.60 |
| fanout-frames | 4.38 / 1.34 / 3.44 | 2.17 / 0.01 / 2.70 | 2.21 / 0.01 / 2.57 | 2.22 / 0.01 / 2.62 | 2.20 / 0.13 / 2.65 | 2.22 / 0.13 / 2.71 | 2.26 / 0.14 / 2.69 |
| dense-lr | 46.80 / 8.03 / 17.76 | 12.54 / 0.05 / 5.45 | 12.71 / 0.05 / 5.47 | 12.72 / 0.05 / 6.14 | 12.61 / 4.09 / 11.79 | 12.75 / 4.03 / 10.44 | 12.40 / 4.19 / 8.71 |
| dense-td | 50.68 / 9.20 / 17.80 | 31.03 / 0.04 / 9.00 | 31.77 / 0.04 / 10.10 | 30.04 / 0.04 / 8.72 | 31.08 / **10.84** / **17.96** | 31.74 / **11.02** / **18.58** | 31.73 / **11.06** / 16.80 |
| bench-75 | 61.94 / 15.76 / 25.28 | 24.14 / 0.02 / 6.52 | 25.30 / 0.02 / 8.34 | 24.64 / 0.02 / 8.63 | 23.68 / 13.26 / 18.55 | 24.26 / 13.68 / 21.77 | 23.42 / 13.45 / 20.18 |
| analog | 13.20 / 2.90 / 7.25 | 11.00 / 0.02 / 3.88 | 11.10 / 0.02 / 3.98 | 10.70 / 0.02 / 3.83 | **13.21** / **4.50** / 6.14 | 10.96 / **3.79** / **8.10** | 10.58 / **3.66** / **8.59** |
| public fixture | 21.12 / 5.02 / 10.70 | 9.78 / 0.02 / 3.28 | 9.53 / 0.02 / 3.70 | 9.70 / 0.02 / 3.72 | 9.63 / 2.34 / 4.29 | 9.48 / 2.35 / 5.09 | 9.51 / 2.34 / 4.46 |
| v2 | 20.38 / 4.54 / 11.04 | 10.93 / 0.02 / 6.38 | 10.07 / 0.04 / 4.36 | 10.06 / 0.03 / 4.23 | 10.17 / 4.08 / 8.71 | **16.34** / 4.15 / 8.24 | 9.98 / 4.02 / 6.25 |

- **Every 4.6 budget holds in block mode on all three runs.** The start broke dense-td's `route_ms` / `drag_ms` and
  the analog's `route_ms` / `drag_ms` on two runs or more. The default head is within the PLAN rule of the
  default start on every file (`W/perftab-*.md`; the default code path is unchanged; its one analog drag 7.99 on
  run 2 is the default build, not block mode, 4.98 / 5.06 on the other runs, 5.07-7.23 at the start).
- An earlier measurement of this round (`S/runs/b12a3fr-on-head-{1..4}`, before the refusal ran on corner lists)
  had dense-td's worst drag 13.1-13.3 ms on three runs and 19.48 on one, and one analog `scene_ms` 15.78: the
  refusal cost 8.7 ms on dense-td's hub `n16` (`W/probe-head/src/bin/onedrag.rs` with a stage timer). Fixed in
  the commit (0.4 ms); worst dense-td drag frame 13.45 -> 8.54 ms (`worstdrag`).
- **`route()` cold and warm** (lead item 4; `coldwarm`, fastest of 5-10, two runs agree within 5 %): warm (after
  `scene()` on the thread) dense-td 0.04-0.06 ms, bench-75 0.03, dense-lr 0.05, public fixture 0.02; cold (a
  fresh thread, no memo: the first `route()` of a graph, a host that changes a size or a label, or a graph past
  the 8 the memo keeps) = one static layout: dense-td 33.4-33.8 ms, bench-75 25.7-26.0, dense-lr 13.3-13.4, public
  fixture 10.3-10.5 (the start's fresh block-mode `route()`: 11.0, 13.5, 4.1, 2.3). A host that calls
  `route_sized` with newly measured sizes on every edit pays the cold cost each time; a drag (`route_partial`
  or `route()` with nodes moved) does not.
- **`route_partial` worst frame** (ms; frames over 6 ms), block mode / default build of the same head, four rounds;
  the start (suite-on) two rounds:

| fixture | block mode (head) | default (head) | start, block mode | verdict |
|---|---|---|---|---|
| dense-lr | 5.73, 6.98, 5.52, 5.49 | 5.88, 6.58, 6.71, 7.98 | 7.15, 7.45 | default over 6 on three runs; block mode within PLAN's rule of the default on every run (6.98 vs 8.23) |
| dense-td | 8.33, 10.65, 8.56, 8.91 (28-64 frames) | 6.49, 8.18, 6.57, 6.66 | 9.46, 8.48 (42-61) | judged at C (lead item 6); as at the start |
| bench-75 | 7.10, 7.93, 8.75, 6.32 | 7.45, 8.00, 6.37, 6.39 | 7.95, 7.46 | default over 6 on every run; within the rule on runs 1, 2, 4, over on run 3 (8.75 vs 7.96): not two in a row |
| public fixture | 4.01, 3.39, 3.24, 3.21 | 4.50, 5.01, 4.60, 4.58 | 4.72, 4.31 | < 6 met |
| analog | 5.19, 6.88, 5.25, 5.00 | 6.83, 4.84, 6.82, 6.03 | 4.86, 3.80 | one frame over 6 on one run of four; the default build over 6 on three runs |

## 6. Suite-on tests and the default run

- Suite-on (`T/cand-d99c073/gates/failing.txt`): 8 failing, a subset of the start's 15, none newly failing:
  `block_fixtures_are_not_drawn_worse`, `channels_keep_edges_apart_and_off_chips`, `defect_baseline_gate` (the
  routed lines of section 3), `ports::tests::a_nearly_straight_pair_is_drawn_straight`,
  `render::tests::advanced_showcase_parses_and_renders` (A2's X3), `a_composite_edge_to_its_own_state_drops_
  straight_from_the_border`, `far_apart_back_edges_and_frame_legs_square_up_alike_in_route`,
  `note_links_hold_their_ends_and_corners_in_scene_route_and_drag` (each failing at the start). Newly passing:
  `quality_gate`, `a_one_px_drag_leaves_unrelated_edges_put`, `comb::a_hubs_fan_is_a_comb_in_scene_and_route`,
  `crossing_gaps_do_not_read_as_dashes`, `a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`,
  `drag_past_top_left_keeps_every_position`, `note_links_match_between_scene_and_route`.
- New tests, green in the default run and suite-on: `a_drag_keeps_a_layout_column_only_where_it_adds_no_crossing`,
  `a_redrawn_frame_keeps_its_room_off_what_it_does_not_hold`, `a_followed_path_is_rounded_as_the_router_rounds_one`,
  `a_followed_label_stays_on_its_leg_clear_of_the_turn`, `a_resized_node_carries_its_followed_ends_onto_its_new_box`,
  `scene_hands_a_block_layouts_drawing_to_route`; the previous round's six (two of them now without the removed
  setting).
- B12a's acceptance from the commit (section 5 of the spec): default run unchanged (guard green, off-identity
  tests, 25 corpus SVGs byte-equal, switch off); suite-on A1 judge 0 must-fail (as at the start), A2 static 86
  (as at the start; A2 is not met and not this package's), R as in section 4.

## 7. Open, for the lead

1. **Not built, moved to step 4 by the lead:** the full corridor builder (R2, R3, R4, R8) and its 4.4 tests
   (`corridors_nest_and_do_not_jump_on_a_one_px_drag`, `a_corridor_passes_two_staggered_bands`,
   `a_corridor_reaches_a_second_row_target`, `route_takes_a_corridor_round_a_foreign_block`,
   `corridor_slots_do_not_follow_statement_order`, `a_far_end_drag_never_changes_a_frame_it_does_not_enter`,
   `a_redrawn_frame_covers_the_edges_that_did_not_move`). R4 would also remove the compound-graph drag lines of
   4.1 (b9-inside: a kept edge rerouted through a foreign frame's room).
2. `route_partial` on dense-td in block mode (8.3-10.7 ms, as at the start): judged at C (lead item 6).
3. The routed rows equal to their static twins (section 3) wait for `b12a2c-squeeze` and step 4.
4. Per-file drag lines of 4.1 that are still worse than the start (v1-hubblocks's 1 px trade, two far-drag fan
   crossings on public-fixture copies, three compound-graph files); every drag sum is no worse.
5. D5: no name or number of the local family in `d99c073`'s message. `a4fb996`'s sentence stays for the squash.

## 8. Disk

9.7 GB free at the end. Kept for the verifier: `W/target-on` (suite-on), `W/runs/fr1-*`, the probe targets
`W/target-p{start,head}`, `W/target-rp{def,on,base}`, `W/start`; all may be deleted after the verification
(`W/target-*` about 300 MB, `W/runs` 111 MB). `T/cand-d99c073` keeps its binaries (36 MB); its worktree and target were removed by
`run-checks.sh`.
