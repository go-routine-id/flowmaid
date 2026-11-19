# Checkpoint R: `route()`, `route_sized()`, `route_partial()` in block mode (B12.13)

Workspace only, never committed. `B` = the b12 workspace directory, `S` = the scratchpad, `R` = `/home/user/flowmaid`,
branch `ccr-ccc6d89a-5cxd7e`. Package `b12a3-route`, run of 2026-11-18 (~13:30-15:30 UTC), built on the unsqueezed
layout as the lead's order of 2026-11-18 asks.

**Result: checkpoint R is not met, and this package stops under D6 without a commit.** The branch head stays
`157117e` (the package's start). Two things stop it, each measured twice:

1. **The gate fails at the start already, on rows no corridor builder reaches** (section 2): the 4.6 budgets in block
   mode (dense-td `route_ms` 11.14 | 14.00 against 9.20 and worst `drag_ms` 19.48 | 19.52 against 17.80, where the
   default build on the same box gives 7.35 | 10.08 and 14.13 | 16.46; `route_partial` worst frame dense-td 9.35 | 9.12
   ms and dense-lr 7.94 | 7.13 ms against 6 ms, though the default build is over 6 ms on this box today too), and
   `fr_max` above 0.5 px on 150 of the 265 block-mode direction
   copies (public fixture 42.32, the analog family 341-1307, bench-75 459-538, dense-lr up to 1040), which is the static
   scene's F3 widening over the layout's chains, a growth `route()` has no layout to reproduce (section 4).
2. **The full corridor builder was built and measured (section 3) and is not committable**: against `base-157117e` it
   takes the local routed table from 37 to 29 must-fail lines (8 gone, 13 better, 2 worse) and the routed copies'
   defects 169 -> 122 and `lateral_x` 418 -> 325, but 85 direction copies get worse in a verdict column, 5 suite-on
   tests newly fail, among them the A1-held `blockcheck::a_hub_fan_leaves_in_target_order`, and 23 gate-message lines
   get worse. Decision 2 forbids such a commit; decision 5 does not cover it (the worse rows pass today).

No README, CHANGELOG, version or repository file changed. The default build is untouched (no commit), so B12a's
"default run: nothing changed" holds trivially on `157117e`.

## 1. What ran, and the command lines

- Start commit `157117e` (merge of dev #137 back into the work branch). Fresh baseline of the three checks:
  `B/tools-a2b/run-checks.sh B/tools-a2b/base-157117e --commit 157117e` (297 s): local A2 86 must-fail, A1 0, routed
  37; suite-on failing tests 15 (the 14 of `15b5383` plus dev's `scene::tests::drag_past_top_left_keeps_every_position`,
  failing in block mode only on a 0.05000000000000071 float against its 0.05 tolerance, as the merge said).
- Before render (default build): `S/tools/render.sh pre-b12a3-route` (25 corpus SVGs in `S/runs/pre-b12a3-route`).
- Iteration loop (`B/r3/iter.sh <label> [ENV=...]`): incremental suite-on build of the CLI and the quality example in
  `B/r3/target-on`, then check 1 (`fmrun.sh`, 95 local inputs) and check 3 (`qrun.sh`, 332 + 32 copies) against
  `base-157117e`. Variants v1-v9 in `B/r3/runs/`; the experiment's knobs were environment variables during the
  experiment only (`FMX_NOCXB`, `FMX_E3COR`, ...), never committed.
- Full three checks of the best candidate (v8): `FMX_NOCXB=1 run-checks.sh B/r3/cand-v8 --tree R`, compared with
  `compare.sh base-157117e B/r3/cand-v8` -> `B/r3/cand-v8.cmp`.
- Perf: the start's suite-on `quality` binary (`base-157117e/bin/quality`) over `S/flowcorpus --drag`, twice
  (`S/runs/b12a3-route-on-start-{1,2}`), and the default build's, twice, as the box's control
  (`S/runs/b12a3-route-off-start-{1,2}`); `route_partial` from `base-157117e/bin/lib-tests --ignored
  fixture_drag_perf_guard` with its source worktree re-created, twice (`B/r3/rp-start-{1,2}.txt`).
- No `rustc` of the other package ran during any timing (`pgrep -c rustc` 0 before each); load average 1.0-1.5 (its
  benchmark processes).

## 2. Checkpoint R's gate at the start (`157117e`, suite-on)

| row of 4.1 R | value at the start | must | state |
|---|---|---|---|
| budgets of 4.6, two runs | dense-td `route_ms` 11.14 \| 14.00 (limit 9.20), worst `drag_ms` 19.48 \| 19.52 (17.80); advanced drag 2.70 \| 2.62 (2.61); fanout-frames drag 4.01 \| 3.86 (3.44) | within | **fails** on dense-td (control run of the default build 7.35 \| 10.08 and 14.13 \| 16.46, so block mode itself costs it); advanced and fanout-frames are inside the box's noise (the default build gives 2.04 \| 2.24 and 3.69 \| 3.95 there) |
| `route_partial` worst frame | dense-lr 7.94 \| 7.13 ms, dense-td 9.35 \| 9.12 ms (the guard test stops at dense-td: it asserts 8 ms); `route()` worst frame in the same loop dense-td 18.35 \| 18.52 ms, 539 frames over 8 ms | < 6 ms | **fails** (both runs), but not attributable to block mode on these runs: the default build on the box today gives dense-td 9.42 \| 6.98, dense-lr 6.22 \| 6.00, bench-75 7.95 (`B/r3/rp-off-{1,2}.txt`). Block mode's own cost shows in `route()` per frame: 18.35 \| 18.52 ms against the default build's 14.02 \| 12.64, 539 frames over 8 ms against 96 \| 81 |
| `DRAG_JUMPS_MAX` | `a_one_px_drag_leaves_unrelated_edges_put`: bench-75 24 > 13 drag jumps | as the starting release | **fails** |
| routed hard zeros 0 | `quality_gate`: dense-td routed `edge_node_x` 1; `block_fixtures_are_not_drawn_worse`: nested-cases and nested-chain routed hard zeros 1, notes/wide-frames routed 5 | 0 | **fails** |
| `fr_max` | public fixture and its permutations 42.32, BT 42.32, LR/RL 44.75; analog 341.41, v2 1306.57, perms 341-534; 150 of 265 block-mode copies above 0.5 px (`B/r3/frmax-start.txt`) | <= 0.5 px on the analog family and the public fixture | **fails** (section 4) |
| `ends_apart` | public fixture 15, BT 17, analog 17, v2 17, dense-td 28, bench-75 19 | not above the starting release's | not judged (the 0.38-path reference of this column is not in the tables of `B/metrics`; reported) |
| `dfr_own` / `dfr_far` (1 px drags) | public fixture 41.32 / 0, BT 41.32 / 0, LR 44.75 / 0, hub5 121.45 / 0, cycle2 45.25 / 0, nested-cases 113.75 / 0, title-entry 0.50 / 0 (`B/r3/dr-base.tsv`) | `dfr_own` <= 1.5 px on the analog family and public fixture; `dfr_far` 0 | `dfr_far` holds; `dfr_own` **fails** (same cause as `fr_max`) |
| 4.3 (h), local routed table | 37 must-fail lines of 62: `lateral_x` 21, `lbl_struck` 7, crossings 7, `lbl_crowd` 2, bench-75 `xframe_runs` 1 | 0 | **fails** |
| DEFECT_BASELINE routed | `lat_wiggles` 23 > 5, `lat_backtrack` 3856.27 > 425, `fan_x` 64 > 13, `end_tilt_max` 0.61 > 0.6, `band_*` within | at or below the ceilings | **fails** |

## 3. The corridor builder (built, measured, not committed)

**What was built** (`B/r3/corridor-builder-experiment.diff` with `B/r3/corridor.rs.v9`, about 430 lines; the tree
was restored to `157117e` afterwards):

- `src/corridor.rs`: R2 classes from the F1+F2 boxes of the two units each edge joins at its ends' common level
  (`levels::Tree::join`), `FLOW_MIN_GAP` apart along the flow; per E1 / E3 edge, the rows of boxes between its two
  flow-facing sides, per row the breadths a column may take (clear of every box and of every frame holding neither end
  by the ortho stage's `COLUMN_CLEAR`, inside every frame holding an end), one column per row, the nearest to the
  column before (R5, lower breadth on a tie), no corridor where a straight run down the source's or the target's
  column passes every row (one step, left to the ortho stage) or where no row lies between (E2); then R3's slots: the
  runs of different corridors closer than a `TRACK_PITCH` over a shared stretch are dealt slots a pitch apart inside the
  interval they share, ordered by the side they leave to and how early they leave (the one leaving first lies nearest
  its side), then by the breadth they go to, ties by edge index (`ports::place` keeps the order and the least move);
  R8's label spot on the column between the two end units.
- `route_built`: the corridors handed over as dummy-like waypoints (a port on each flow-facing side, one point per
  row), so the ports stage deals the ends by the first and last column (the first legs planned for the real port
  spread); E3 edges between blocks no longer enter the lane planning (`route_backs` takes a `no_lane` mask) and get a
  facing-side builder curve (out of the source's upstream side, into the target's downstream side).
- `ortho`: a corridor's columns are run as a static chain's are (`Ctx::corridor`), in `route()` and on every drag
  frame; a new `edgepass::finish_routed_blocks` carries the lanes and corridors to the stages.

**Measured variants** (all against `base-157117e`; "copies worse" counts copies worse in any verdict column, routed):

| variant | local routed must-fail | routed copies: defects, x_drawn, lateral_x, fan_x | copies worse |
|---|---|---|---|
| v1: static block stage rules in `route()` only (`Ctx::blocks`) | 37 -> 41 | 141, 2050, 415, 228 | 76 |
| v2: + corridors, no slots | 37 -> 39 | 123, 2064, 403, 248 | 75 |
| v4: + slots (R3) | 37 -> 39 | **84**, **1976**, 408, 225 | 73 |
| v5: corridors + slots, no block stage rules | 37 -> 34 | 128, 2036, 419, 220 | **39** |
| v6: v4 + E3 as channels | 37 -> 37 | 89, 1941, **316**, 212 | 97 |
| **v8: v5 + E3 as channels** | **37 -> 29** (8 gone, 13 better, 2 worse) | 122, 2041, 325, 221 | 85 |
| v9: v8, E3 a channel only where it has a corridor | 37 -> 35 | 119, 2110, 371, 238 | 64 |

v8 on the local routed table: crossings gone on the analog and perm-c/d/e (26 -> 23, 30 -> 21), `lateral_x` gone on
infra-lr/rl and the public LR/RL copies and down on every analog-family input (9 -> 4 or 5), v2 `lbl_struck` 5 -> 2;
worse: perm-b `lbl_crowd` 6 -> 9, v2 crossings 22 -> 24. Full checks of v8 (`B/r3/cand-v8.cmp`): **cells WORSE (2),
panics WORSE (23), dirs WORSE (136 lines on 85 copies)**. Newly failing suite-on tests:
`blockcheck::a_hub_fan_leaves_in_target_order` (A1-held), `blockcheck::a_label_keeps_off_the_blocks_that_hold_one_end_of_its_edge`,
`casing::tests::a_line_takes_one_role_across_a_cluster_of_crossings`, `edges_keep_off_frame_borders`,
`labels_cover_nothing_they_must_not`. Worse gate messages, among them: DEFECT_BASELINE routed `lat_backtrack` 3856.27
-> 9140.75, `fan_x` 64 -> 72, new `band_pairs` 174 > 141 and `band_len` 83118 > 71020; bench-75 routed defects 15 ->
17 and `min_edge_gap` 1.18 < floor 10.6; bench-75 drag jumps 24 -> 25 (`svc2`: `svc4 -> logs` 50.5).

**The worse copies of v8, by cause** (`B/r3/worse.py v8-e3-nocxb`):

- *E3 as a channel* (min_edge_gap falls where an upstream edge between two blocks now runs a track beside a downstream
  one: reqresp/reqresp22 LR/RL, hub5, note-edge-row, spread-bottom, turned-blocks, self-member; 30 copies drop 0.8 to
  27 px, none under its floor), and the public TD/BT copies' `side_ends` 3 -> 5 / 6 -> 7: `SHIP -> MAIL` (tracking mail)
  has no corridor (the target's column is clear), its one-step channel meets something the router reroutes, and the
  reroute leaves shipping worker's right side and enters the mail relay's left side.
- *Corridors whose hub ends are dealt for the old headings*: ratio4 / ratio2x4 / ratio4mask TD/BT (`lateral_x` and
  `side_ends` 0 -> 1: b2 -> a5 leaves b2's flank), dense-td TD/BT/RL (`x_drawn` 107 -> 125, 93 -> 126, 73 -> 86;
  `fan_x` 34 -> 42, 15 -> 43), dense-lr TD (`x_drawn` 24 -> 45, `fan_x` 0 -> 17), bench-75 LR/RL (defects 15 -> 17,
  8 -> 11): the fans that converge on one target cross where the slots of one row and the tracks of the next gap
  disagree; nested-cases, spread-face (`x_drawn` 2-4 -> 7), wide-stroke.
- What the measurements say the next attempt needs, beyond this builder: the ports stage dealing a hub's side by the
  corridors' slot order (it still re-sorts by heading), the slots of one gap planned together with the tracks of the
  next (the ortho stage places tracks after the slots are fixed), and the static block stage rules (`Ctx::blocks`)
  taught `route()`'s geometry (on their own they make 76 copies worse, v1). The previous package's finding stands:
  a partial R3 trades dense fans for defects; the full one built here trades them less (v4: defects 169 -> 84, x_drawn
  1997 -> 1976) but still makes 73 copies worse.

## 4. `fr_max` above 0.5 px, with causes (R's reporting row)

At the start 150 of the 265 block-mode direction copies (`B/r3/frmax-start.txt`) and every analog-family input. The
cause, checked frame by frame on the public fixture and on v2: along the flow and in F1's breadth the frames of `scene()`
and `route()` agree (B12.10's work); across the flow the static scene's F3 widens a frame over the dummy chains the
**layout** placed inside it, beside its members, and `route()` has no layout to place them:

- public fixture: only Observability differs, 42.3 px narrower in `route()` (the chains of `scrape` and `dashboards`
  the layout laid inside it);
- v2: DEV/STAGING is 2298 px wide in `scene()` and 908 px in `route()` (a chain laid along the frame's far side);
- hub5: Email 265 px against 142.6 (the upstream `bounce` edge's chain inside it, the label beside it);
- bench-75, dense-lr, dense-td: 459-1076 px, the same widening on their largest frames.

A corridor builder can make `route()`'s F3 grow over its own inner legs, but it puts them where its rule puts them (the
nearest free column beside the members), not where the layout's ordering put the chain (hub5: a column at about 329
against the layout's 431). So `fr_max` <= 0.5 px at the scene's centres, and `dfr_own` <= 1.5 px on a 1 px drag of a
member (a redrawn frame loses the static widening its kept edges do not cover), need either the layout's chain columns
carried in the scene for `route()` to reuse (new state the scene does not hold today; R6 says "no new state in the base
scene") or a static F3 that does not widen frames for chains (a layout change). Both are the lead's and the user's to
decide; neither is in this package.

## 5. Suite-on failing tests and the default run

- Suite-on at `157117e` (`base-157117e/gates/failing.txt`): 15 failing, the 14 of `15b5383` plus dev's
  `drag_past_top_left_keeps_every_position` (block mode only, float tolerance). R-due among them:
  `a_one_px_drag_leaves_unrelated_edges_put`, `quality_gate` (routed rows), `defect_baseline_gate` (routed column),
  `block_fixtures_are_not_drawn_worse` (routed rows), `edges_keep_off_frame_borders`, `crossing_gaps_do_not_read_as_dashes`,
  `scene::tests::a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`,
  `scene::tests::far_apart_back_edges_and_frame_legs_square_up_alike_in_route`, `comb::tests::a_hubs_fan_is_a_comb_in_scene_and_route`.
- Default run: no commit; `S/tools/guard.sh` on the unchanged head `157117e` at the end of this run: `GUARD: ALL
  GREEN` (`B/r3/guard-head.txt`; the off-identity tests are part of it). The working tree is clean.

## 6. Musts failed and stop rules fired

- Stop rule of R ("a budget broken on two `perf.sh` runs in a row"): **fired at the start** on dense-td (`route_ms`,
  `drag_ms`, both runs, block mode against a default-build control inside or near its limits). `route_partial` is over
  6 ms on both block-mode runs (dense-td 9.35 | 9.12, dense-lr 7.94 | 7.13) and on the default build's runs too
  (dense-td 9.42 | 6.98, dense-lr 6.22 | 6.00, bench-75 7.95), so that row is not judged from these runs.
- Musts of R failing at the start and not met by the builder: `fr_max`, `dfr_own`, routed hard zeros, `DRAG_JUMPS_MAX`,
  4.3 (h) (37 lines; 29 with the builder), DEFECT_BASELINE routed.
- Decision 2's checks against the start: the builder (v8) is WORSE in all three; nothing committed.

## 7. What the lead may want to decide

1. Whether `route()` may read the static layout's chain columns (a crate-private field of the scene, or a side table
   the host hands back) so that at the scene's centres its corridors and F3 are the layout's: the only way measured
   here to `fr_max` <= 0.5 and `dfr_own` <= 1.5, and the most direct way to `route()` = `scene()` at the first paint;
   it is new state in the base scene, which R6 rules out as written.
2. Whether v8's trade (local routed 37 -> 29 must-fail lines, routed copies defects -47 and `lateral_x` -93, against 85
   copies worse, 30 of them `min_edge_gap` drops that stay above every floor) may land as a declared trade, or the
   builder waits for the ports stage dealt by the corridors' slots and the tracks planned with the slots.
3. The block-mode drag budget: dense-td's `route()` is over its 4.6 limits at the start (11.14 | 14.00 ms, worst drag
   frame 19.48 | 19.52), before any B12.13 work, and the corridor builder does not pay it (v8: 10.77 ms on the TD copy,
   11.20 on BT); `route_partial` needs a quieter box (or a longer measurement) to be judged at all.

## 8. Disk

14 GB free at the end. `B/r3/target-on` (the suite-on scratch target) and `B/r3/cand-v8/bin` are deleted; the run
directories under `B/r3/runs/` keep their binaries (36 MB each) for the verifier.
