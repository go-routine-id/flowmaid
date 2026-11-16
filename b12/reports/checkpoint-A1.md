# Checkpoint A1: structure (B12.0-B12.6), after fix round 1

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`, branch
`ccr-ccc6d89a-5cxd7e`. This replaces the first A1 report (2026-11-11); the resume note of SPEC section 0 applies.

**Gate: NOT met. Every crossing must is met now; four other musts are open** (section 6): a run through a foreign
frame on bench-75 (2), dense-td (2) and `notes/fork-twins` (1); `fan_x` on the local inputs (1 on six of seven) and
above the start on dense-td, bench-75 and fork-twins; the judge's `bands` string on one local permutation (a conflict
between two rules of the spec, section 6.3); static hard zeros not 0; eight suite-on tests that must pass as written.
Per decision D6 nothing was re-baselined or loosened.

What changed since the first report, in one table (drawn crossings, static, switch on):

| input | first A1 report | now | must | start / mermaid |
|---|---|---|---|---|
| analog | 21 | **16** | <= 20 | 23 / 16 |
| v2 | 15 | 15 | <= 17 | 19 / 13 |
| perm-a, perm-c, perm-d | 21 | **16** | <= 20 | 12, 23, 23 / 7, 16, 19 |
| perm-b | 32 | **22** | <= 23 | 18 / 19 |
| perm-e | 40 | **18** | <= 22 | 33 / 18 |
| public fixture and its four permutations | 9 | **6** | <= 7 | 14, 6, 23, 15, 12 / 5 |
| bench-75 | 223 | 247 | <= 325 | 294 / 228 |
| dense-lr | 15 | 16 | <= 51.5 | 45 / 10 |
| dense-td | 151 | **64** | <= 74.6 | 66 / 68 |

## 1. Commits, command lines, judge output

- Start `3abbfeb`. Package: `c024ac0` B12.0, `bd7069a` B12.1, `1fa410e` B12.3, `5f24084` B12.4, `15ecef7` B12.5,
  `48d9022` B12.6. Fix round: `fd8c7ba` (the paused round's work as it stood: O2's weighted adjacent exchange, source
  order for a member a channel holds out of place, `Ctx::lane`, the two strengthened tests, the record of block mode
  only while a test records), `978393f` (track room between frames, a moved end leaves the layout's columns, two
  `ortho` rules), `3ae7829` (the column rule limited to ends moved past `PORT_EVEN_MAX`; the two `ortho` rules taken out
  again, section 7), `8c6dc26` (the sweep asserts its hard zeros). Everything below ran on `3ae7829` / `8c6dc26` (the last
  one changes a test only).
- Suite-on build: `RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=B/target-on CARGO_INCREMENTAL=0 cargo build
  --release --bin flowmaid --example quality`; tests: the same with `cargo test --no-fail-fast` and
  `CARGO_PROFILE_DEV_DEBUG=0`. `B/target-on` and `B/target-dbg` are deleted after this report.
- Local acceptance, static: `cd B/local && ./run.sh fm B/target-on/release/flowmaid out/a1r2 b12-a1r2 && ./run.sh
  judge out/a1r2 --due A1`. Output `B/local/out/a1r2/judge-a1.txt`, metrics `out/a1r2/metrics.tsv`. Result: **155
  checks, 6 must-fail lines (4 distinct), 8 targets missed**; 109 rules due later left out.
- Harness rows (fixtures and local inputs, start binary and switch on): `B/work/r3/q/start.tsv`, `a1r2.tsv`
  (`B/work/r3/q.sh`, compared by `qcmp.py`). Starts table: `B/local/out/a1r2/starts-table.txt` (`B/work/r3/tab.sh`).
- Suite-on test log: `B/local/out/a1r2/suite-on-tests.txt`; guard log of the default build: `out/a1r2/guard.txt`.

Judge, must-fails (static, `--due A1`):

```
MUST FAIL perm-c  bands        U2 T1 U1 / EXT GH / PROD / DR STG   must == analog: U1 T1 U2 / EXT GH / PROD / DR STG
MUST FAIL bench-75   xframe_runs 2  must <= 0   [0.38 4, start 4, mermaid 0]      (two rules name it)
MUST FAIL dense-td   xframe_runs 2  must <= 0   [0.38 12, start 12, mermaid 0]    (two rules name it)
MUST FAIL fork-twins xframe_runs 1  must <= 0   [0.38 0, start 0, mermaid 0]
```

## 2. The rows of 4.3 (a)-(c) due at A1 (static)

| input | xframe_runs (0) | sib_side_linked (0) | crossings / must | fan_x (0) | canvas breadth | bands |
|---|---|---|---|---|---|---|
| analog | 0 | 0 | 16 / 20 | **1** | 2972 | U1 T1 U2 / EXT GH / PROD / DR STG |
| v2 | 0 | 0 | 15 / 17 | **1** | 3119 | U1 T1 U2 / GH EXT / DR / PROD / STG |
| perm-a | 0 | 0 | 16 / 20 | **1** | 2972 | as the analog |
| perm-b | 0 | 0 | 22 / 23 | 0 | 2969 (-0.1 %) | as the analog |
| perm-c | 0 | 0 | 16 / 20 | **1** | 2972 | **U2 T1 U1** / EXT GH / PROD / DR STG |
| perm-d | 0 | 0 | 16 / 20 | **1** | 2972 | as the analog |
| perm-e | 0 | 0 | 18 / 22 | **1** | 3048 (+2.6 %) | as the analog |
| public fixture, 4 permutations | 0 | 0 | 6 / 7 | 0 | 1142 on all five | OPS SHOP / CI EDGE / WEB / ORD / DATA / OBS on all five |
| hub5 | 0 | 0 | 0 | 0 | | |
| bench-75 | **2** | 0 | 247 / 325 | 21 (start 6) | | |
| dense-lr | 0 | 0 | 16 / 51.5 | 0 (start 2) | | |
| dense-td | **2** | 0 | 64 / 74.6 | 18 (start 1) | | |

The harness total `x_drawn` static over `tests/fixtures/quality`: **328** (start 405; first report 390).
Static rows (d) for information (due A2): `lateral_x` 3 on the analog and the public fixture, `up_lanes` 0 on all but
perm-b (1).

## 3. Suite-on failing tests against 4.5, and the default run

Default run on `8c6dc26`: `tools/guard.sh` prints `GUARD: ALL GREEN` (fmt, clippy, tests, doctests, MSRV 1.75, wasm,
rustdoc); `everything_renders_as_before_with_blocks_off` and `the_switch_changes_nothing_outside_block_mode` pass. (The
paused round's commit `fd8c7ba` did not pass clippy: two type aliases in `978393f` fix that.)

Suite-on `cargo test --no-fail-fast` on `3ae7829`: lib 1125 passed, **12 failed**; `tests/quality.rs` 8 passed, **7
failed**; cli, model_build, theme and doc tests pass.

| failing test | class | cause |
|---|---|---|
| `casing::a_line_takes_one_role_across_a_cluster_of_crossings`, `casing::a_short_line_keeps_at_most_one_gap`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route` | 4.5 intended: re-read | fixture-specific numbers on the re-laid dense fixtures; not re-read (they wait for the geometry) |
| `comb::a_hubs_fan_is_a_comb_in_scene_and_route` | 4.5 intended: re-read | **the rule itself does not hold yet**: 12 crossings inside dense-td's hub fan (`fan_x` 18, section 6.2) |
| `comb::combing_is_settled_after_one_pass` | must pass (B11) | **new in this round.** dense-td: a second comb run moves 2 edges. Passed before the track room between frames; cause not established (the deeper band gaps change what the comb's own test accepts; the test also builds its context with `edgepass::ctx`, which does not carry the block layout's lane classes that `scene()`'s own comb ran with) |
| `ports::a_nearly_straight_pair_is_drawn_straight`, `render::advanced_showcase_parses_and_renders` | must pass (`examples/advanced.mmd`) | the block layout puts the two ends of the pair each test names 210 px and 18 px apart across the flow: no straight spine across a border without the coordinate rules X1-X3 (B12.8). Not built |
| `router::edges_into_a_wedged_frame_keep_out_of_the_gap` | must pass | the test's two frames are stacked by the block layering (it reads "frames -1009 px apart": they overlap on the axis it measures a gap on), so the wedge it tests does not exist in block mode. The rule has nothing to measure; the test needs the lead's decision (re-read with a fixture L4 leaves side by side, or outside block mode) |
| `scene::a_composite_edge_to_its_own_state_drops_straight_from_the_border` | must pass | own-box edge (2.8, B12.14): the stub is drawn with a jog. Not built |
| `scene::a_lane_stays_in_its_frames_in_route_and_on_a_drag_too` | must pass | `route()` at block positions: its frame is not the scene's (`Outer` at x = -40). R1/F3, B12.13. Not built |
| `scene::note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `scene::note_links_match_between_scene_and_route` | must pass | note links of state diagrams in block mode, scene against route (B12.14, B12.13). Not built |
| `quality.rs`: `block_fixtures_are_not_drawn_worse` | gate | `examples/advanced` static defects 1 / overlap_pairs 1 against ceilings of 0; `notes/fork-twins` static edge_frame_pairs 1 against 0 (the public fixture's crossing ceiling of 7 now holds) |
| `quality.rs`: `quality_gate`, `defect_baseline_gate`, `edges_keep_off_frame_borders`, `channels_keep_edges_apart_and_off_chips`, `crossing_gaps_do_not_read_as_dashes`, `a_one_px_drag_leaves_unrelated_edges_put` | gates of 4.5 | static halves due at A2, routed halves and drag jumps at R (X, G, F, E's ends, R not built) |

Not a subset of "4.5's intended changes plus what A1 names as not built": `combing_is_settled_after_one_pass` and
`edges_into_a_wedged_frame_keep_out_of_the_gap` are in neither class. Passing as written now, failing in the first
report: `layout::a_hub_fed_row_of_shallow_frames_wraps_into_two_rows`, `labels_cover_nothing_they_must_not`. Passing
as written throughout: `edge_line_order_barely_moves_the_dense_layouts`,
`moving_a_node_declaration_barely_moves_dense_td`, every test of `compound.rs`, `every_fixture_has_a_ceiling`,
`hops::a_pick_finds_the_painted_hops_on_every_fixture`, the `route_partial_*` tests.

## 4. Perf (4.6)

No `rustc` running, load average 1.2-2.1. `scene_ms` (fastest of the example's runs), start binary, switch on twice:

| file | start | switch on, run 1 | run 2 | limit |
|---|---|---|---|---|
| analog | 5.80 | 9.66 | 9.72 | 11.58 |
| v2 | 12.16 (one run) | 8.86 | 8.44 | |
| public fixture | 6.09 | 7.62 | 7.59 | 12.12 |
| bench-75 | 33.44 | 39.85 | 40.78 | 61.94 |
| dense-td | 22.57 | 36.17 | 34.61 | 50.68 |
| dense-lr | 16.61 | 14.12 | 13.99 | 46.8 |
| advanced | 0.88 | 1.17 | 1.19 | 1.42 |
| subgraph | 0.14 | 0.14 | 0.14 | 0.38 |
| fanout-frames | 3.23 | 1.81 | 1.85 | 4.38 |

A1's gate on `scene_ms` (analog, public fixture, bench-75, dense-td, dense-lr): met on both runs; dense-td now has 14
ms to spare (first report: 1.5).

Default build, `tools/perf.sh` twice (`runs/b12a1-fix1`, `runs/b12a1-fix1-2`; against the PLAN baseline): dense-td
`route_ms` 6.35 / 5.86 (baseline 7.36), worst `drag_ms` 10.89 / 12.08 (14.24); bench-75 `route_ms` 8.83 (12.61), drag
16.89 (20.22); dense-lr route 5.26 (6.42), drag 8.61 (14.21); every other file inside its budget. The default build is
byte-identical to the start, so these are the start's timings.

`route_partial`, worst frame, default build, `cargo test --release --lib -- --ignored --test-threads=1
fixture_drag_perf_guard`: dense-lr 5.27 ms, dense-td **5.58** (start 5.68 / 5.55; first report 6.00 / 5.96: the
record of block mode is now taken only while a test records, so a drag frame pays nothing for it), bench-75 7.27
(start 7.96 / 5.40), public fixture 4.53 (start 4.04 / 3.98).

## 5. Crossings by number of starts, and the levels the lane rule fires on

Static `x_drawn` with the ordering forced to k starts (`quality --starts k`), then with the starts the budget grants
(`BLOCK_ORDER_STARTS` 9, `BLOCK_ORDER_WORK` 1400: 9 starts up to about 150 elements and links, the public fixture 5,
dense-td 3, bench-75 5):

```
input                    s1   s2   s3   s4   s5   s6   s7   s8   s9   granted  must
analog                   20   20   20   20   16   16   16   16   16   16       20
v2                       20   17   17   22   22   22   22   22   15   15       17
perm-a, perm-c, perm-d   20   20   20   20   16   16   16   16   16   16       20
perm-b                   27   27   27   26   26   22   22   22   22   22       23
perm-e                   32   25   25   25   25   25   18   18   18   18       22
public fixture (all 5)   6    6    6    6    6    6    6    6    6    6        7
dense-td                 98   37   64   64   64   64   64   64   64   64       74.6
dense-lr                 19   17   16   16   16   16   16   16   16   16       51.5
bench-75                 230  248  248  248  248  248  248  248  248  248      325
```

- The musts hold with the granted starts and inside the time limits. They are not monotone in the starts: v2 is 17
  with 2-3 starts, 22 with 4-8 and 15 with 9; dense-td is 37 with 2 starts and 64 with 3 and more. A start is chosen
  by its layered crossings and the drawing has others: the fewest layered crossings are not the fewest drawn. That is
  why the paused round raised `BLOCK_ORDER_STARTS` from 7 to 9 (v2 needs the ninth); it is a pin on one local input
  and should be read as fragile until the edge stages add nothing to the layered count.
- Where the difference between layered and drawn comes from now, stage by stage (static `x_drawn` after each edge
  stage; `B/work/r3/stages.sh` on a debug build of the round's first commit, the final counts as of its last): the
  public fixture layered 5, after `ortho` 5, after the router 6; the analog after `ortho` 16, after the router 19,
  final 16; dense-td layered 38, `ortho` 36, router 57, nudge 66, final 64; bench-75 layered 175, `ortho` 143, router
  254, final 248. The router's list of edges to repair on these
  inputs is almost only `chip: 1` (an entry or exit under a block's title chip: dense-td 9 edges, the public fixture
  4, the analog 5) and, on bench-75, node hits of ends spilled round a corner of an over-full side. Both are 2.8's
  (title-clear entries, the spill order: B12.11); neither is built.
- Levels the lane rule (L4) fires on: the root of `swimlane`, `swim-cycle` and `swimlane-nested`, no other fixture
  or local input; 52 of the 160 generated flowcharts of the sweep.

Generated sweep (B12.6, 240 graphs, 200 in block mode): no panic in `scene()` or `route()`, deterministic, block rows
and the two order constraints hold. Hard zeros in the static scene: 48 on 29 diagrams as blocks, 105 on 52 before
blocks; blocks add some on 9 diagrams (one flowchart, eight state diagrams with notes). Asserted since `8c6dc26` (at
most half, fewer diagrams, at most 9 added); the 0 of the spec is open.

## 6. Every must that is open, every stop rule

Stop rules of A1's row: **none of the crossing rules fires any more** (analog, v2, the permutations, the public
fixture, the dense fixtures, the harness total, the starts the budget grants). dense-td under the test's six
shuffles: `edge_line_order_barely_moves_the_dense_layouts` passes as written (width within 10 %); its crossings under
the six shuffles were NOT measured in this round. The gate is not met because of the musts below.

1. **Runs through a foreign frame** (`xframe_runs` 0): bench-75 2, dense-td 2, `notes/fork-twins` 1.
   - dense-td `n46 -> g9`: the step out of `n46` to its channel beside the first band is placed mid-gap between the
     two rows, which is inside that band's title strip (three frames nest there): 1 327 px under the band's title.
   - dense-td `n41 -> n42`: two members of one block on adjacent rows with a nested sibling block between them; the
     link runs through that block (217 px). The ordering holds "one side per (run, block)" for channels only; an
     adjacent-layer link between two nodes has no dummy. Not covered by 2.4 as written.
   - bench-75 `gw -> logs`: drawn by the router (a repair of a node hit) through two blocks.
   - fork-twins: a note card's link (B12.14).
   A rule for the first kind was built and taken out again (section 7).
2. **`fan_x`** (0 static on the analog family; not above the start elsewhere): 1 on the analog, v2 and four
   permutations (0 on perm-b, the public fixture, hub5); dense-td 18 (start 1), bench-75 21 (start 6), fork-twins 2
   (start 0). The fans cross where the router has taken an entry round a title chip or a spilled end round a
   corner; `comb::a_hubs_fan_is_a_comb_in_scene_and_route` fails for the same reason.
3. **`bands` same as the analog**: perm-c reads `U2 T1 U1` where the analog reads `U1 T1 U2`. The three actors have
   one link each, all to the same node: their order changes no crossing, so O4b puts them in declaration order, and
   perm-c declares them the other way round. 4.3 (b)'s `same_as` on the whole string and 2.4 O4b cannot both hold for
   this input. The band sets and their order along the flow are the analog's on all five permutations, the breadth
   within 2.6 %. Needs the lead's decision (compare the bands, not the order inside a band, for crossing-neutral
   units); nothing was changed.
4. **Static hard zeros 0**: not 0 on `blocks/own-centre` 1, `blocks/self-member` 1, `blocks/sub-nested-self` 2 (own-box
   edges, as at the start: B12.14), `notes/fork-twins` 1 (start 3), `notes/nested-composite` 2 (start 4),
   `notes/spread-face` 1 (start 3), **`notes/wide-frames` 2 (start 0)**; and on 29 of the 200 generated block
   diagrams. `blocks/nested-chain` is 0 again. Routed (R's): wide-frames 5 (start 0), dense-td 2 (start 0).
5. **Suite-on tests that must pass as written**: section 3 (eight, two of them outside both classes of 4.1).
6. Rows-equal inputs drawn worse than the start (4.3 e, due at C, visible now): `examples/advanced` static defects
   0 -> 1 (overlap_pairs 0 -> 1); `notes/wide-frames` defects 0 -> 2; `notes/fork-twins` x_drawn 9 -> 13,
   edge_frame_pairs 0 -> 1.

Verifier's findings not answered by code in this round:

- #9 (a frame-to-frame edge drawn as a slanted stroke; spines lost on three bulk inputs) and #10 (`route()` with the
  switch on draws outside its canvas on 10 bulk inputs and `blocks/reqresp22`): not touched. They are b12a2's ends and
  coordinates and b12a3's R7.
- #11: the consumers of `Ctx::back` ask for the lane class now (`Ctx::lane`, ports and router, from `fd8c7ba`);
  `a_turned_edge_one_layer_up_is_a_z_not_a_lane` still runs left to right only: top-down the router takes such an
  edge round the title chip until title-clear entries exist.
- #7: `a_member_a_channel_holds_out_of_place_goes_back_with_it` holds the verifier's `s5` input at 1, 3 and 9 starts.
  The state diagram `x3-state` still reads `Reject Approve`. By the dump of its orders the kept order has 1 layered
  crossing with the channel of `Review --> Running` on the block's far side; on the near side, where source order
  would put it, that channel crosses `Running --> Failed` as well (read off the dump, not measured), so the pair is
  not crossing-neutral and O4b does not apply.

## 7. Pulled forward, tried and taken out, not built

- **Pulled forward from B12.9 (2.6 G1), for the tracks only**: in a block layout the room a gap keeps for its steps
  is measured from frame to frame (`reach_of` on both sides). `b12a2-geometry` does not build this again; G1's two
  floors, the label rows and the crossed borders stay its own. Test
  `a_band_gap_holds_its_steps_between_the_frames`.
- **Title-clear entries (2.8) were not pulled forward.** The crossing musts are met without them. They are what the
  remaining difference between layered and drawn crossings waits for (section 5), and with them `fan_x`, the adjacent
  upstream Z top-down and most of the router's reroutes.
- **Tried and taken out (`978393f` in, `3ae7829` out)**: "a step that would lie inside a frame not holding both its
  ends moves out of it" (`ortho::plan_jog`) with "the ends of a side take their places as their turns nest"
  (`ortho::untangle`, judged by the second pass). Measured with both: dense-td `xframe_runs` 2 -> 1, bench-75 2 -> 1,
  `examples/advanced` back to 0 defects, the `ortho` stage leaves `quality/fanout-frames` without a crossing; but the
  step of the edge to the node past the frames, back in its channel, leaves the gap one track short at its pitch, the
  router reroutes one edge of the fan across another (0 -> 1 crossing), and
  `ports::a_fans_square_paths_nest_off_their_side` fails. It needs the track packing of a gap (G2) and the port order
  of 2.8 with it.
- Not built, as in the first report: B12.3's wrap of a long row (deferred to B12.12 by the lead); the routed half of
  `a_hub_fan_leaves_in_target_order` is asserted now (`fd8c7ba`), on the public fixture and hub5.
