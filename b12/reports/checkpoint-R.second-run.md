# Checkpoint R: `route()`, `route_sized()`, `route_partial()` in block mode (B12.13), second run

Workspace only, never committed. `B` = the b12 workspace directory, `S` = the scratchpad, `R` = `/home/user/flowmaid`,
branch `ccr-ccc6d89a-5cxd7e`. Package `b12a3-route`, second implementation (2026-11-18, ~15:45-19:30 UTC), built on
the lead's rulings of ~15:30 UTC (SPEC section 0, "Lead rulings after `b12a3-route`'s first implementation"). The
first run's report is kept as `B/reports/checkpoint-R.first-run.md`.

**Result: checkpoint R is not met; this run stops under D6 on one conflict between the two rulings, with the work
committed as far as it is no worse.**

- **Ruling 1 is built** (`a4fb996`): `route()` of a block-mode part starts from the static layout's drawing of it
  (a crate-private, deterministic memo of the layout per graph, sizes, labels and start count; the "side table"
  of the ruling), so at the scene's own centres `route()` *is* `scene()`, and any drag is `route_partial()` from
  that drawing, in which an edge a drag redraws keeps the layout's columns and only its end legs follow the moved
  end, and a redrawn frame keeps the layout's room. With it on, every R row that ruling 1 was meant to pay is paid:
  `fr_max` 0 and `ends_apart` 0 on all 79 block-mode local inputs and all 265 block-mode direction copies,
  `dfr_own` <= 1.0 px and `dfr_far` 0, drag jumps within `DRAG_JUMPS_MAX`, routed hard zeros = the static ones,
  the local routed table 37 -> 19 must-fail lines, the routed copies' defects 161 -> 33, `x_drawn` 1995 -> 1283,
  `lateral_x` 418 -> 201, `fan_x` 181 -> 30, `edge_chip` 77 -> 7, `side_ends` 361 -> 216, and failing suite-on
  tests 15 -> 9.
- **But with ruling 1 the routed rows at the scene's centres are the static rows, by construction**, and on 30
  of the 265 block-mode copies (and in DEFECT_BASELINE's routed column, and in one local routed cell) the static
  drawing is worse than today's routed drawing in a column ruling 2 forbids to get worse (section 3). None of
  these is a property of `route()`: each is a static defect that the A2 / C packages own, now also shown by the
  routed twin. Ruling 2 ("crossings, defects, lateral_x, fan_x, side_ends and edge_chip may not get worse on any
  copy") therefore cannot hold together with ruling 1 until those static rows are fixed.
- So the route-from-layout path is committed **behind a second internal setting** that is off in the suite-on run
  too (`--cfg flowmaid_route_layout`, `quality::with_route_from_layout`), and everything else of this run (the
  drag paths of R6, which the three checks measure only through the gate tests) is on. In that state decision 2's
  three checks are **no worse** against `157117e` (cells and dirs unchanged, gate messages: failing 15 -> 14,
  `a_one_px_drag_leaves_unrelated_edges_put` passes again). The lead decides whether the setting is turned on
  (and the static rows of section 3 are then judged as declared trades, or fixed first by the squeeze package).

Commits (no amend, no rebase): `a4fb996` (the drag paths, route-from-layout behind its setting, five tests),
`67b99fc` (two tests), `0a3fd4d` (a fallback shortcut, section 5). No README, CHANGELOG or version change. Default
build byte-identical: guard green before each commit (the off-identity tests are part of it), and the 25 corpus
SVGs of the default CLI at the head are byte-equal to `S/runs/pre-b12a3-route` (`B/r3b/post`). Decision 2's three
checks at the head (`B/tools-a2b/run-checks.sh cand-0a3fd4d --commit 0a3fd4d`, `compare.sh base-157117e`):
**VERDICT: no worse** (cells 86 -> 86 / routed 37 -> 37, gate messages failing 15 -> 14, dirs unchanged).

## 1. What ran, and the command lines

- Start `157117e`; baselines `B/tools-a2b/base-157117e` (three checks, made by the first run) and
  `S/runs/pre-b12a3-route` (default render), `S/runs/b12a3-route-{on,off}-start-{1,2}` (perf at the start).
- Iteration: `B/r3b/iter.sh <label>` (suite-on CLI + quality example in `B/r3b/target-on`, check 1 and check 3
  against `base-157117e`), `B/tools-a2b/build.sh` + `trun.sh` + `panics.py` for check 2. Runs in `B/r3b/runs/`:
  `v1`, `v2` (route-from-layout on, before it got its setting), `v3` (setting off: the committed suite-on state),
  `rl1` (commit `a4fb996` built with `--cfg flowmaid_blocks --cfg flowmaid_route_layout`, the three checks by
  `compare.sh`, report `B/r3b/runs/rl1/compare.txt`, `dirs-full.txt`).
- `rl` builds: `B/r3b/build2.sh` (= `tools-a2b/build.sh` taking `RUSTFLAGS_ON`), target `B/r3b/target-rl`.
- Per-edge causes of the worse copies: `quality --details` over the copies in `B/r3b/wc/in` for the base and `rl1`
  binaries (`B/r3b/wc/{base,rl1}.details`).
- Perf: `B/r3b/perfrun.sh <bindir> <label>` (waits for a quiet box up to 10 min, records `rustc` count and load at
  start / middle / end in `box.txt`, then `quality S/flowcorpus --drag` and the `route_partial` lines of
  `fixture_drag_perf_guard`), into `S/runs/b12a3-route-v3-{def,on,rl}-{1,2}` (section 4).

## 2. What was built

### 2.1 `route()` from the layout (ruling 1; `a4fb996`, behind `flowmaid_route_layout`)

- `route_fitted` (the one entry of a part that is not split further): in block mode with the setting on,
  `route_from_layout` takes the static drawing of the part from `laid_scene` (thread-local memo `LAID_SCENES`, at
  most 8 drawings, key: graph, fitted sizes, labels, `order_starts`; forgotten when the block switch or the start
  count changes, `levels::forget_layouts`; a pure function's memo, so a `route()` with or without an earlier
  `scene()` on the thread draws the same: test `route_at_the_scenes_centres_draws_the_scene` clears the memo and
  compares the SVG).
- At the layout's centres it returns that drawing (marked routed); a host that moved the whole drawing by one
  offset (`common_offset`: the offset shared by most nodes among the first three probes) gets it moved, its canvas
  refitted as `route_built` starts one; any other set of moved nodes is `partial_core` from it, i.e. exactly what
  `route_partial()` draws from the scene (test `route_follows_a_drag_as_route_partial_does_from_the_scene`).
  Compound parts are routed part by part as before, each part from its own laid drawing (the translation covers the
  block placed in its proxy).

### 2.2 The drag paths (R6; `a4fb996`, on in every suite-on run)

- **Follow** (`follow_drag`): an edge redrawn only because an end moved keeps its base corners (the layout's
  columns, corridors and inner legs); the moved end is carried by its shift, the corner next to it moves on the end
  leg's axis, the path is delivered corner-rounded and densely sampled (`router::smooth`), its ends are held; a
  straight path whose ends moved apart across it, or one the shift would bend back (a leg turning, an end leg below
  `FOLLOW_END_MIN` = `CORNER_R / 2` where the base's was longer), is drawn afresh by the builders as before. Ends on
  a redrawn frame follow the frame (`frame_follow`).
- **Frames**: a redrawn frame keeps the room its kept and followed edges take inside it (`keep_inner_room` with the
  followed paths), and in block mode never draws narrower than the layout drew it (the static frame was grown over
  chain columns the edge stages may have moved off; blocks/cycle2's Front 45 px).
- **Held spilled sides** (`ports::place_ports`, block layouts' scenes only): a side all of whose ends the drag holds
  is left as drawn also when the pass spilled one onto it (bench-75 `svc5 -> logs` 24.2 px on every drag).
- **Early fallback**: more than `PARTIAL_ROUTE_SHARE` of the edges on the moved nodes alone -> the fallback before
  the builders run (same frame, cheaper; no note card where a link could be kept through the fallback).

### 2.3 What R3's corridor builder now has to do, and what was not built

With ruling 1 the corridor builder of 2.9 R3 is needed only where `route()` does not start from the layout: an edge
a drag redraws whose follow is refused (a large drag that turns a leg), and a host's own positions (more than
`PARTIAL_ROUTE_SHARE` of the edges moved). There the builders of the start still draw (no corridors; the v8/v9
experiment of the first run was not ported: every gate this run measures is met or failed independently of it, and
the first run measured it to make 85 copies worse when it draws every routed edge). Not built: R3 for those cases,
R4 (foreign frames as hard obstacles in the repair), R8 for followed edges beyond keeping the label on the path,
and the own-box rules of 2.8 (B12.14, named "not built yet" in R's row).

## 3. The conflict: the routed rows carry the static drawing's numbers

With route-from-layout on (`rl1`), against `base-157117e`:

**Check 3 (dirs), routed:** totals much better, 30 copies worse in a verdict column, 63 `min_edge_gap` drops. Each
worse copy's routed row equals its own static row, which is unchanged against the start (the static column of check
3 is identical); the cause is the static drawing, read per edge with `quality --details` (`B/r3b/wc`):

| column | copies (routed start -> now = static) | cause in the static drawing (owner) |
|---|---|---|
| side_ends | the 8 public TD copies 3 -> 4 | `CART -> QUEUE` enters the order queue by a lateral side, besides the three both drawings share (`SEARCH -> INDEX`, `REG ==> ORD`, `WEB -> METRICS`; static A2 row `side_ends`) |
| lateral_x | interleave-BT, l5loop-TD, nested-BT, nested-wide-title-BT, title-hidden-TD 0 -> 1 | one chain each across a lateral wall (`s2 -> Q`, `s2 -> t1`, `i2 -> o2`, `a1 -> dst`, `A -> Q`; static A2 `lateral_x`) |
| lateral_x | notes/spread-bottom x4 0 -> 1 | `K0 -> K2N` across K2's wall (notes, C) |
| defects | dense-lr LR/RL 1 -> 2 | the hub `n12`'s fan: two overlapping pairs (`n12 -> n26` / `n27` 84 px, `n12 -> n38` / `n36` 24 px; static, A2) |
| defects, fan_x, side_ends | notes/wide-frames LR/RL | state note links on `K0N_1` (C) |
| x_drawn, side_ends | notes/fork-twins BT/RL/TD | note links and a spilled end (C) |
| fan_x | notes/spread-face LR/RL 0 -> 1 | `K0 -> B0` against a note link (C) |
| x_drawn | self-member RL/TD, sub-nested-self LR/RL 0 -> 1 | own-box edges (`S -> b`, `OUT -> IN`: 2.8, B12.14, "not built yet") |

The 63 `min_edge_gap` drops (the kind ruling 2 accepts as a declared trade when above every floor) are listed in
`B/r3b/runs/rl1/dirs-full.txt`; five of them are not above the floors and so could not be declared:
turned-blocks-LR 39.89 -> 1.59, sub-nested-self LR/RL 40 -> 6, BT/TD 40 -> 7 (all static values; `lane_gap_min` /
`EDGE_SEP_MIN` 8).

**Check 2 (gate messages)**: failing 15 -> 9 (newly passing: `quality_gate`, `a_one_px_drag_leaves_unrelated_edges_put`,
`comb::a_hubs_fan_is_a_comb_in_scene_and_route`, `crossing_gaps_do_not_read_as_dashes`,
`a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`, `drag_past_top_left_keeps_every_position`,
`note_links_match_between_scene_and_route`), but:
- `defect_baseline_gate`, routed column = the static column: `lat_wiggles` 23 -> 33, `lat_backtrack` 3856 -> 10600,
  new `band_pairs` 203 > 141, `band_len` 91456 > 71020, `end_kink` 1 > 0 (gone: `end_tilt_max`, `fan_x` 64 -> 5).
  These are the static fields over their ceilings that the squeeze package owns (8(3)); 4.5 lets `band_*` and
  `lat_backtrack` rise for corridors, not `lat_wiggles` or `end_kink`.
- `block_fixtures_are_not_drawn_worse`: new `blocks/self-member routed: x_drawn = 1 > 0` and `hard zeros = 2 > 1`
  (the static values; own-box edges, B12.14) — and 12 lines better or gone.
- `squared_ends_face_the_curve_and_keep_near_it_on_every_fixture` fell under its non-vacuity floor (9 squared ends
  of 544, floor 10): fixed in `67b99fc` by reading the block fixtures too (1740 ends; 72 / 52 / 16 squared in the
  default build / as blocks / with route-from-layout; the floor unchanged).

**Check 1 (cells)**: routed must-fail 37 -> 19 (18 gone, 13 better), static unchanged; one cell worse:
`perm-b-subgraphs-reversed lbl_crowd` 6 -> 8 (must <= 5), the static value 8 (A2 label rows).

So the question for the lead (section 7): with ruling 1, a routed row is the static row; is "routed = static, and
worse only where its static twin is" accepted as the declared trade of ruling 1, or does the setting stay off until
the squeeze package has fixed those static rows?

## 4. Checkpoint R's gate

Columns: **off** = the committed suite-on state (route-from-layout off, the drag paths on); **on** = with
`flowmaid_route_layout`.

| row of 4.1 R | off | on | must |
|---|---|---|---|
| `fr_max` | as at the start: 48 of 79 block-mode local inputs above 0.5 px (public 42.3, analog 341.4, v2 1306.6) | **0.00 on all 79 inputs and all 265 copies** | <= 0.5 on the analog family and public fixture |
| `ends_apart` | as at the start (public 15, analog 17, dense-td 28) | **0 everywhere** | not above the start's |
| `dfr_own` (1 px drags, `quality::frame_drags`) | public TD/BT/LR/RL 1.00 (start 41.3 / 41.3 / 44.8 / 44.8), analog 1.00 (341.4), v2 1.00 (1307.6), perms 1.00 (341-533), infra-lr/rl/bt 1.00, hub5 1.00 (121.5), cycle2 0.00 (45.3), nested-cases 1.00 (113.8), title-entry 0.50 | the same (a drag is `route_partial` either way) | <= 1.5 on the analog family and public fixture |
| `dfr_far` | 0 (float noise below 1e-9 where a hub's drag falls back to a whole route) | 0 | 0 |
| `DRAG_JUMPS_MAX` | bench-75 24 -> **7** (<= 13), dense-td, dense-lr, spread-face, fork-twins, near-drag 0 | the same | as the starting release |
| routed hard zeros | as at the start (dense-td `edge_node_x` 1, nested-cases / nested-chain 1, wide-frames 5) | = static: wide-frames 2 (static 2), self-member 2 (static; own-box, B12.14), every other block fixture 0 | 0 |
| 4.3 (h), local routed table | 37 must-fail (as at the start) | **19**: `lateral_x` 1-3 on the analog family / public family / cycle2 (static A2 `lateral_x`), `lbl_struck` 1-3 on analog / v2 / perm-a/c/d (static A2), perm-b `lbl_crowd` 8 (static) | 0 |
| DEFECT_BASELINE routed | as at the start (`lat_wiggles` 23, `lat_backtrack` 3856, `fan_x` 64, `end_tilt_max` 0.61 over) | = static (section 3) | 4.5 |
| budgets of 4.6 | section 4.1 | section 4.1 | 4.6 |
| `route_partial` worst frame | section 4.1 | section 4.1 | < 6 ms, or vs the default build by PLAN's rule |

The (h) rows with the setting on (value at the start / now, must; bold = must-fail):

| input | xframe_runs | lateral_x | lbl_struck | lbl_crowd | crossings |
|---|---|---|---|---|---|
| analog | 0 / 0 | **9** / **1** | **2** / **1** | 6 / 4 (<= 7) | **26** / 15 (<= 24) |
| v2 | 0 / 0 | **5** / 0 | **5** / **3** | 7 / 4 (<= 8) | **22** / 14 (<= 20) |
| perm-a | 0 / 0 | **9** / **1** | **2** / **1** | **6** / 4 (<= 5) | **26** / 15 (<= 18) |
| perm-b | 0 / 0 | **9** / **2** | **3** / 0 | **6** / **8** (<= 5) | **30** / 18 (<= 18) |
| perm-c | 0 / 0 | **9** / **1** | **2** / **1** | 6 / 4 (<= 8) | **26** / 15 (<= 24) |
| perm-d | 0 / 0 | **9** / **1** | **2** / **1** | 6 / 4 (<= 7) | **26** / 15 (<= 24) |
| perm-e | 0 / 0 | **7** / **1** | **2** / 0 | 6 / 5 (<= 7) | **30** / 17 (<= 26) |
| public fixture | 0 / 0 | **2** / **2** | 0 / 0 | 2 / 2 (<= 5) | 10 / 6 (<= 11) |

(d) routed `lateral_x` with the setting on: infra-bt / infra-lr / infra-rl / hub5 / public LR / public RL 0 (start
6 / 1 / 2 / 2 / 2 / 2); public BT 3 (4), cycle2 2 (3), the other public permutations 2 (2). bench-75 routed
`xframe_runs` 1 -> 0. Routed crossings of the public fixture equal across its permutations (6 on all, start 10):
the routed half of `corridor_slots_do_not_follow_statement_order` holds because the static one does.

### 4.1 Perf (4.6) and `route_partial`

Three builds of the package's head, each run twice on the same box, interleaved (default build, suite-on with the
setting off = the committed suite-on state, suite-on with `flowmaid_route_layout`): `S/runs/b12a3-route-v3-{def,on,rl}-{1,2}`,
`quality S/flowcorpus --drag`; `box.txt` beside each: no `rustc` running, load 1.3-2.4 at start (the other package's
renders and verifies ran before, between and during some runs; every run waited up to 10 minutes for load < 1.5).
Limits as in 4.6 (the public fixture and v2 by the rule, from the start's two default runs); bold = over.
Columns: `scene_ms` / `route_ms` / worst `drag_ms` (a full `route()` per drag frame).

| file | limit scene / route / drag | b12a3-route-v3-def-1 | b12a3-route-v3-def-2 | b12a3-route-v3-on-1 | b12a3-route-v3-on-2 | b12a3-route-v3-rl-1 | b12a3-route-v3-rl-2 |
|---|---|---|---|---|---|---|---|
| advanced | 1.42 / 1.72 / 2.61 | 1.32 / 0.96 / 2.10 | 1.22 / 0.88 / 2.30 | 1.15 / 0.57 / 2.36 | 1.22 / 0.58 / **3.09** | 1.14 / 0.01 / 1.67 | 1.15 / 0.01 / 1.84 |
| subgraph | 0.38 / 1.05 / 2.04 | 0.17 / 0.07 / 0.46 | 0.26 / 0.12 / 0.76 | 0.19 / 0.07 / 0.54 | 0.19 / 0.07 / 0.57 | 0.33 / 0.01 / 0.83 | 0.18 / 0.00 / 0.45 |
| fanout-frames | 4.38 / 1.34 / 3.44 | 2.20 / 0.31 / 2.44 | 2.14 / 0.30 / 2.45 | 2.43 / 0.12 / 3.13 | 2.28 / 0.12 / 3.12 | 2.28 / 0.01 / 2.70 | 2.29 / 0.01 / 2.84 |
| dense-lr | 46.80 / 8.03 / 17.76 | 19.12 / 7.08 / 10.30 | 19.20 / 4.51 / 13.34 | 12.97 / 4.25 / 10.95 | 12.88 / 4.12 / 10.07 | 12.97 / 0.05 / 6.57 | 12.91 / 0.05 / 7.20 |
| dense-td | 50.68 / 9.20 / 17.80 | 27.60 / 6.85 / 11.60 | 26.40 / **9.28** / 13.07 | 49.19 / **16.43** / **20.40** | 31.56 / **11.00** / **20.01** | 32.04 / 0.04 / 8.94 | 32.83 / 0.06 / 10.25 |
| bench-75 | 61.94 / 15.76 / 25.28 | 39.19 / 10.52 / 20.88 | 38.65 / 14.05 / 18.96 | 24.33 / 13.87 / 21.39 | 24.26 / 13.60 / 24.45 | 24.67 / 0.02 / 6.01 | 24.73 / 0.02 / 8.55 |
| infra-overview | 13.20 / 2.90 / 7.25 | 6.99 / 2.25 / 5.93 | 6.94 / 2.23 / 5.20 | **14.92** / **5.52** / **7.34** | 11.13 / **3.87** / 6.63 | 11.25 / 0.02 / 5.13 | 11.35 / 0.02 / 4.95 |
| blocks-overview | 21.12 / 5.02 / 10.70 | 7.24 / 2.92 / 6.42 | 7.24 / 2.90 / 5.96 | 9.69 / 2.39 / 4.48 | 9.96 / 2.43 / 6.64 | 15.57 / 0.02 / 3.41 | 9.88 / 0.02 / 4.44 |
| infra-overview-v2 | 20.38 / 4.54 / 11.04 | 10.37 / 3.56 / 10.51 | 10.21 / 3.54 / 7.19 | 14.32 / **6.48** / 6.83 | 10.94 / 4.25 / 6.81 | 10.10 / 0.02 / 4.01 | 10.26 / 0.03 / 4.91 |

- **With the setting on, every budget holds on both runs**: `route()` at the scene's centres costs 0.01-0.06 ms (a
  memo hit), a drag frame is the drag of `route_partial` (dense-td worst 8.94 | 10.25 against 17.80, bench-75 6.01 |
  8.55 against 25.28, the analog 5.13 | 4.95 against 7.25); the static `scene_ms` is the block layout's as before.
- **With it off** (committed suite-on state) dense-td's `route_ms` 16.43 | 11.00 and worst `drag_ms` 20.40 | 20.01 and
  the analog's `route_ms` 5.52 | 3.87 break their limits on both runs, as at the start (dense-td 11.14 | 14.00 and
  19.48 | 19.52, the analog 3.87 | 5.39; first run's report): block mode's fresh `route()`, untouched by this package.
  advanced's 3.09 is one run (2.36 the other).
- **`route_partial` worst frame** (`B/r3b/rpprobe`: `fixture_drag_perf_guard`'s loop and timing, fastest of
  `DRAG_RUNS`, re-timed with 10 runs over 8 ms, without stopping at the first miss; `B/r3b/rpprobe/rp-all.txt`, a quiet
  box: no rustc, load 1.06-1.21), ms, two runs:

| fixture | default build | suite-on, setting off | setting on | default build's limit by PLAN's rule (lower run) |
|---|---|---|---|---|
| dense-lr | 7.06 \| 6.18 (5 \| 1 frames over 6) | 7.99 \| 7.51 (5 \| 5) | 5.81 \| 6.37 (0 \| 4) | 7.73 |
| dense-td | 9.56 \| 6.67 (5 \| 6) | **9.95 \| 10.30** (48 \| 46) | **9.76 \| 8.54** (41 \| 46) | 8.34 |
| bench-75 | 8.56 \| 7.63 (12 \| 13) | 7.64 \| 7.97 (4 \| 12) | 8.27 \| **9.65** (9 \| 17) | 9.54 |
| public fixture | 4.54 \| 4.45 | 4.14 \| 4.35 | 4.05 \| 3.47 | (< 6 ms) |

  The 6 ms limit is missed by the default build itself on dense-lr, dense-td and bench-75 on both runs, so block
  mode is judged against it (ruling 3): dense-td misses in both block configurations on both runs (41-48 frames over
  6 ms against the default's 5-6; the start's suite-on runs: 9.35 | 9.12), i.e. block mode's dense-td drag, not this
  package's paths; bench-75 with the setting on misses on one run of two. **The route_partial budget is not met on
  dense-td in block mode** (stop rule of R: a budget broken on two runs).
- Default build against the start's default runs (`perf.sh b12a3-route-v2-c1` and the two `def` runs): no file slower
  beyond the spread (the default build runs the same code but the early fallback, which draws the same frame).

## 5. The fallback shortcut (`0a3fd4d`)

`0a3fd4d`: with the setting on, a `route_partial()` frame that falls back to `route()` drew the drag a second time from
the same drawing before falling back again; when the base draws as the drawing `route()` starts from (`laid_is`,
memo peek only), the fallback draws afresh at once. Same frame (test: a hub drag in
`route_follows_a_drag_as_route_partial_does_from_the_scene`); no change with the setting off.

## 5b. Pictures (`B/r3b/pic/`)

- `blocks-overview-static-start-rl.png`, `infra-overview-static-start-rl.png`, `infra-overview-v2-static-start-rl.png`:
  static | routed at the start | routed with the setting on (4.3 h's routed picture beside the static one; the third
  panel is the first, byte for byte; the start's routed drawing runs through frames and narrows Observability).
- `gap-*.png`: the routed drawing at the start | with the setting on, for the largest `min_edge_gap` drops that stay
  above the floors (turned-blocks-RL 40 -> 10.26, nested-chain-TD 40 -> 12.5, dense-lr-LR 40 -> 12.51,
  nested-chain-BT 39.5 -> 12.5, nested-cases-LR 38.9 -> 14) and the public BT copy (31.94 -> 12.5, 8 copies): in
  each the narrower gap is a corridor of the static drawing beside a block, where the start's routed edge ran through
  or over the block.
- `cycle2-hub5.png`: the static frames whose room the layout's chain columns took (Front's left, Email's right), the
  cause of `dfr_own` 45 / 121 px before the frame rule of 2.2.

## 6. Suite-on failing tests and the default run

- Suite-on (setting off, committed state; `B/r3b/runs/v3/gates/failing.txt`): 14 failing, the start's 15 less
  `a_one_px_drag_leaves_unrelated_edges_put`. The R-due ones still failing: `quality_gate` (routed rows),
  `defect_baseline_gate` (routed column), `block_fixtures_are_not_drawn_worse` (routed rows), `channels_keep_edges_apart_and_off_chips`,
  `crossing_gaps_do_not_read_as_dashes`, `a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`,
  `far_apart_back_edges_and_frame_legs_square_up_alike_in_route`, `comb::a_hubs_fan_is_a_comb_in_scene_and_route`,
  `drag_past_top_left_keeps_every_position`, `note_links_*` (each as at the start).
- With the setting on (`rl1`): 9 failing (section 3).
- New tests, green in the default run, suite-on, and with the setting on:
  `a_one_px_drag_moves_a_block_layouts_frames_by_a_px_at_most`, `a_drag_keeps_the_layouts_columns_of_the_edges_it_redraws`,
  `a_drag_leaves_a_hubs_spilled_ends_where_the_scene_put_them`, `route_at_the_scenes_centres_draws_the_scene`,
  `route_follows_a_drag_as_route_partial_does_from_the_scene`, `dragging_every_node_of_a_block_diagram_keeps_the_contract`.
- Default run: `S/tools/guard.sh` GUARD: ALL GREEN before each commit (`B/r3b/guard-*.txt`), the off-identity tests
  included; `perf.sh b12a3-route-v2-c1` (default build) against the start's default runs: no file slower beyond the
  box's spread (bench-75 drag 20.63 -> 19.80, dense-td 14.13 -> 13.65, dense-lr 13.92 -> 15.34 on a loaded box,
  advanced 2.04 -> 1.83).

## 7. Musts failed, stop rules, and what the lead may want to decide

- **Stop (D6)**: rulings 1 and 2 conflict on the routed rows (section 3); the route-from-layout path is committed
  behind its setting, off.
- With the setting on, R's own rows are met except: (h) 19 lines and DEFECT_BASELINE's routed column, all equal to
  their static rows (A2's open rows and C's own-box / note rows), and `route_partial` on dense-td (9.76 | 8.54 ms
  against the default build's 9.56 | 6.67, limit 8.34 by PLAN's rule; block mode's dense-td drag, 41-46 frames over
  6 ms where the default build has 5-6) — R's stop rule (a budget broken on two runs) fires there. Every `route()`
  / drag budget of 4.6 holds on both runs.
- With it off (committed suite-on state), R is not met: `fr_max`, `ends_apart`, (h) 37, routed hard zeros, and
  dense-td's `route_ms` / `drag_ms`, the analog's `route_ms` and dense-td's `route_partial` over their limits on
  both runs, as at the start.

Decisions asked:
1. Turn `flowmaid_route_layout` on (the routed rows become the static rows; the 30 copies, the one cell and the
   DEFECT_BASELINE routed fields of section 3 recorded as the declared trade of ruling 1, each a static row already
   owned by A2 or C), or keep it off until the squeeze package has fixed those static rows.
2. If on: the five `min_edge_gap` copies below the floors (turned-blocks-LR, sub-nested-self x4) are static values
   too; whether they wait for B12.14 (own-box edges) and the squeeze.
3. R3's corridor builder is now needed only for large drags and host positions (2.3); whether it stays in R or moves
   to C.
4. dense-td's `route_partial` in block mode (both settings, as at the start): the drag of a dense-td node redraws
   many more edges in its block layout than in its flat one; whether that budget is judged after the squeeze (which
   changes dense-td's layout) or needs its own item.
5. Follow-up noted, not built: `scene()` could hand its drawing to the memo (with the setting on), so that the first
   `route()` after `scene()` on a thread does not lay the graph out once more (dense-td ~30 ms once).

D5 note: `a4fb996`'s message says "Local acceptance inputs: dfr_own <= 1.0 px on every one measured"; the number
is the measured bound, not a name, but the allowed sentence is "local acceptance inputs: all musts met". It cannot
be amended (branch pushed); the squash at the merge can drop it.

## 8. Disk

9.3 GB free at the end. Deleted: `B/r3b/target-probe`, the three `B/r3b/target-rp-*`, the check worktree and target
of `run-checks.sh` (removed by the script). Kept for the verifier: `B/r3b/target-on` (195 MB), `B/r3b/target-rl`
(272 MB), `B/r3b/runs` (296 MB, binaries included); all three may be deleted after the verification.
