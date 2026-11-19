# Checkpoint A1: structure (B12.0-B12.6), after fix round 5

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`, branch
`ccr-ccc6d89a-5cxd7e`, `pre` stays `3abbfeb`. This replaces the report of the "hold step 1" round (`34f322b`).
Everything below ran on `ec5dca4`. Work files: `B/work/r8/` (scripts `build.sh`, `h.sh`, `cmp.sh`, `on.sh`, `ton.sh`,
`pic.sh`; harness tables `h/`, pictures `pic/`, starts table `starts/`, test output `ton-full.txt`, guard
`guard5.txt`, perf `perf1.txt`, `perf2.txt`).

**Gate: met on every row the verifier named as blocking.** bench-75 static `fan_x` 4 (must <= 6, the start's);
blocks/nested-chain `lateral_x` 0 (must 0) with 0 crossings; `fan_x` 0 on the whole analog family and on all seven
permutations of the public fixture; every crossing must and `xframe_runs` 0 held; local judge **0 must-fail lines**.
Still open, with cause (section 6): dense-td's edge-line shuffles s2 and s6 (`fan_x` 10 and 3, start 1 and 1), the
x1-title-row stress input (X2.2 frame breadth, not built), note-link / own-box hard zeros (lead note 6).

| verifier finding (round 5) | state on `ec5dca4` |
|---|---|
| 1 bench-75 `fan_x` 8 (must <= 6) | **fixed**: 4 (`60fc56e`); x_drawn 210 -> 204, overlap pairs 1 -> 0 |
| 2 nested-chain `lateral_x` 2 (must 0) | **fixed**: 0, crossings 0 (`7d21d12`, X2.5 built) |
| 3 fan order beyond the fixtures | x4-fan-levels `fan_x` 2 -> **0**, crossings 2 -> 0 (start 0 / 0); dense-td shuffles s4 4 -> 0, s5 1 -> 0, s6 4 -> 3, but s2 1 -> **10** (6.2) |
| 4 x1-title-row, title-entry overrun | **open** (6.3): needs X2.2's frame breadth for a chip wider than its block |
| 5 `TitleSides` doc on the type alias | **fixed** (`7d21d12`) |
| 6 `quality::Report` not `#[non_exhaustive]` | **fixed** (`2e8b2e0`) |
| 7 pinned marquee test edited | **fixed**: the test is the start's again and passes in both runs (`ec5dca4`, the test's own path sampler) |

## 1. Commits, command lines, judge output

- `60fc56e` fix(blocks): a turned step keeps its run off the runs in of the steps it crosses (`ortho::part_runs`).
- `2e8b2e0` fix(quality): `quality::Report` is `#[non_exhaustive]`.
- `7d21d12` feat(blocks): X2.5 in the order (`layout::BlockOrder::clear_titles`), X2.2 room for runs past a chip
  (`layout::title_room`, capped by `TITLE_ROOM_MAX`), pinned title-crossing columns in `ortho::straighten` and
  `flow_path`, title-clear rule 1 by breadth alone for title-row members (`ports::title_row_side`), `TitleSides` doc.
- `ec5dca4` test(scene): the marquee test as written; `path_d_points` splits wide parameter steps of a cubic.
- None rewrites a commit from before the package; nothing pushed. Guard `GUARD: ALL GREEN` before each commit
  (`B/work/r8/guard3.txt`, `guard5.txt`); `everything_renders_as_before_with_blocks_off` passes (default run).
- Suite-on build: `B/work/r8/build.sh` (`RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=B/target-on`); harness
  `B/work/r8/h.sh <label>`; local acceptance `B/work/r8/on.sh <label>` (`run.sh fm`, `run.sh judge --due A1`).
  Result (`B/local/out/r8-p10/judge-a1.txt`, same code as `ec5dca4`): **155 checks on 92 diagrams, 0 must-fail
  lines, 7 targets missed** (crossings targets only).

## 2. Rows of 4.3 (a)-(c) due at A1 (static)

| input | xframe_runs | crossings / must (round 4 -> now) | fan_x | canvas breadth |
|---|---|---|---|---|
| analog | 0 | 17 -> **15** / 20 | 0 | 3130 |
| v2 | 0 | 14 / 17 | 0 | 3119 |
| perm-a, perm-c, perm-d | 0 | 17 -> 15 / 20 | 0 | 3130 |
| perm-b | 0 | 19 -> 18 / 23 | 0 | 3153 |
| perm-e | 0 | 17 / 22 | 0 | 3180 |
| public fixture (all permutations) | 0 | 6 / 7 (bt: 7 / 7) | 0 | 1142 |
| bench-75 | 0 | 210 -> 204 / 325 | **4** (start 6) | |
| dense-lr | 0 | 17 / 51.5 | 0 | |
| dense-td | 0 | 53 -> **48** / 74.6 | **0** (start 1) | |

blocks-overview-bt rose 6 -> 7 (its must is 7). The X2.2 room for runs is capped (`TITLE_ROOM_MAX` = 2 node gaps): an
uncapped first version widened perm-b to 3599 px (13 % over the analog's breadth, a must-fail of 4.3 (b)); capped,
breadths are within 10 %.

Repository fixtures (`tests/fixtures/{quality,blocks,notes,fans}`), harness static, start -> now: `x_drawn` 584 ->
332, `fan_x` 17 -> 5 (bench-75 4, notes/wide-frames 1 = start), overlap pairs 9 -> 5. Hard zeros: own-centre 1,
self-member 1, sub-nested-self 2, notes/wide-frames 2 (lead note 6, section 6.4); every other fixture 0.

## 3. What was built, and why (stage by stage)

1. **bench-75 (`60fc56e`).** Measured per stage (debug probe): ortho left 1 fan crossing, the repair added 21. The
   cause was one channel after the app zone: `svc3 -> metrics` turns up to a channel point level with svc5, between
   the ports of `svc5 -> queue` and `svc5 -> objects`; `place_tracks` put the turned step first (libavoid's key), so
   its run on lay 5 px beside both runs in for 110-150 px; the repair took them round svc5's corner, across its fan.
   `part_runs` (block layouts) moves a step past another when the earlier one's run on lies within a track's pitch
   of the later one's run in, if that lays fewer runs along each other and adds no more crossings than it removes.
   Overlaps after the ortho stage 8 -> 2; `fan_x` 8 -> 4.
2. **X2.5 (`7d21d12`).** After the first breadths, a channel point of a block's own level on its title row under
   the block's chip moves its channel past the chip (the column right of the nearest title-row unit reaching past the
   chip, else right of the row's last unit): first its run in the block, then the whole channel when both ends lie in
   the level holding the block; kept when layered crossings and the fans' crossings do not rise; breadths placed
   again. Moving the whole channel of a hub's fan from outside (dense-td `n16 -> n35`) reshuffled the comb and was the
   cause of a 10-crossing regression during the round: hence the "both ends in the level" rule, and the X2.5 moves
   are kept out of the warm-start seed of later placements.
3. **The column is kept.** The ortho stage moved the run back next to the chip (`straighten` towards its neighbours,
   then `flow_path`'s stair merge), where `b2 -> c`'s displaced column then lay right of it and crossed it twice.
   Columns whose run crosses a title side are pinned.
4. **X2.2 for runs** (`title_room`): a run on a title row short of the chip plus the block's displaced columns widens
   its element by the shortfall (capped).
5. **Title-clear rule 1 by breadth** (`ports::title_row_side`): a title-row member's side facing its frame's chip is
   shaded whatever the jog room's depth (2.7 F2). Without it dense-td's `n36` kept its end under `g8`'s chip and the
   router rerouted `n36 -> n16` across the hub's fan (10 crossings).

## 4. Suite-on tests, default run, perf

Suite-on `cargo test --no-fail-fast` (`B/work/r8/ton-full.txt`, before `ec5dca4`; the marquee test passes since):
lib **8 failed**, the same eight as round 5's list: `comb::a_hubs_fan_is_a_comb_in_scene_and_route` (route half, R),
`scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route` (R), `ports::a_nearly_straight_pair_is_drawn_straight`
and `render::advanced_showcase_parses_and_renders` (must pass; X3 straight spines across borders, B12.8, not built),
`scene::a_composite_edge_to_its_own_state_drops_straight_from_the_border` (C), `scene::a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`
(R), the two `note_links_*` tests (C / R). `tests/quality.rs`: 7 failed — the six of round 5 plus
`edges_keep_off_frame_borders` (**routed** `border_len` 2001 against 1895: `route()` at the re-laid block positions,
R / B12.13; its static half holds). `block_fixtures_are_not_drawn_worse` static rows: only notes/wide-frames (as
before); every other row it lists is routed.

Default build: byte-identical (off-identity test green). `tools/perf.sh` twice (`runs/b12a1-structure-r6`, `-r6b`):
bench-75 worst drag 17.56 / 19.01 (limit 25.28), route_ms 10.46 / 11.40 (15.76); dense-td drag 13.40 / 13.32 (17.80),
route 6.90 / 6.85 (9.20); dense-lr drag 10.42 / 11.10 (17.76), route 4.47 / 4.42 (8.03); every small file inside its
limit. A first run's bench-75 static scene_ms 59.0 was the box (interleaved start | now on the default build: 54.7
38.4 38.3 45.9 | 40.4 48.1 39.5 37.8). `route_partial` is not touched (all changes sit behind block mode).
Block-mode static scene_ms, fastest of three runs: analog 10.42 (limit 11.58), public fixture 10.09 (13.6), bench-75
24.10 (61.94), dense-td 30.10 (50.68), dense-lr 12.78 (46.8), advanced 1.04 (1.42), fanout-frames 3.14 (4.38). The
box was noisy (load 1.5-1.9); the analog's headroom is about 1 ms.

## 5. Starts table and shuffles

Harness crossings / fan_x by forced number of starts (`B/work/r8/starts/`, `--starts k`):

| starts | analog | v2 | perm-a | perm-b | perm-c | perm-d | perm-e | public | dense-td | dense-lr | bench-75 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 17/0 | 18/0 | 17/0 | 26/0 | 17/0 | 17/0 | 29/4 | 6/0 | 87/5 | 19/0 | 219/6 |
| 2 | 17/0 | 17/0 | 17/0 | 26/0 | 17/0 | 17/0 | 22/0 | 6/0 | 35/5 | 20/2 | 204/4 |
| 3 | 17/0 | 17/0 | 17/0 | 26/0 | 17/0 | 17/0 | 22/0 | 6/0 | 48/0 | 17/0 | 204/4 |
| 5 | 15/0 | 22/0 | 15/0 | 21/0 | 15/0 | 15/0 | 22/0 | 6/0 | 48/0 | 17/0 | 204/4 |
| 7 | 15/0 | 22/0 | 15/0 | 18/0 | 15/0 | 15/0 | 17/0 | 6/0 | 48/0 | 17/0 | 204/4 |
| 9 | 15/0 | 14/0 | 15/0 | 18/0 | 15/0 | 15/0 | 17/0 | 6/0 | 48/0 | 17/0 | 204/4 |

Still not monotone (v2 22 with 4-8 starts, 14 with 9; dense-td 35 with 2, 48 with 3+): lead note 7's question for A2.

dense-td under the test's six edge-line shuffles (`B/work/r8/h/fin-shuf.tsv`): crossings 41 58 41 41 48 44 (must
74.6, width unchanged), `fan_x` 0 10 0 0 0 3 (start 1 1 1 0 0 1; round 5: 1 1 1 4 1 4).

## 6. Open, with cause

### 6.1 Rows met
bench-75 `fan_x`, nested-chain `lateral_x`, analog family and public fixture `fan_x`, crossings, `xframe_runs`.

### 6.2 dense-td shuffles s2 (`fan_x` 10) and s6 (3)
s2: after the ortho stage `n36 -> n16`'s run out of `n36` (on the free stretch past `g8`'s chip, rule 1) lies 4 px
from `n11 -> n37`'s column above `g8` over 250 px; the repair, which in s2's edge order reroutes `n36 -> n16` rather
than `n11 -> n37` (the default order reroutes the latter, harmlessly), takes it across the hub `n16`'s fan. The
straightening of a channel column does not keep off other edges' port runs (`ColumnRoom` fences boxes, labels,
frames, columns and axis paths only). s6: three crossings at the top of `n16`'s bottom side (steps right under the
hub). Both are node-side end order / channel spacing (B12.11 "even band", the order of a node side's ends by their
channels), not built.

### 6.3 x1-title-row, title-entry, title-hidden
The chip is wider than the whole title row: no column past the chip lies inside the frame, so rule 2 falls back to
"drawn as before" and the entries go round a lateral wall. Needs X2.2's frame breadth (frame >= chip + insets +
displaced columns, in the frame function and reserved by the layout: B12.7/B12.8). blocks/title-entry `lateral_x` 2,
title-hidden 1, unchanged; x1-title-row `lateral_x` 2, `side_ends` 1, `x_drawn` 1 (start 1).

### 6.4 Note links and own-box cases (lead note 6)
notes/wide-frames hard zeros 2 (two note-link pins on one point of `K0N_1`'s side), own-centre 1, self-member 1 (was
2), sub-nested-self 2: own-box stubs and note links (B12.14), as reported in round 5.

## 7. What b12a2-geometry no longer builds (added this round)

- X2.5 for runs under a title chip, in the order, and the column kept through the edge stages.
- X2.2's room for a run past a chip (capped; the frame breadth for a chip wider than its block is still to build).
- Title-clear rule 1 by breadth for title-row members.
- G2's channel order rule that keeps a turned step's run off the runs in of the steps after it.

Still to build: X2.2 frame breadth (6.3), rule 3 for runs partly under a chip, node-side end order and the even band
(6.2), G1's breadth room between stacked members, F2 in LR/RL, F3, and every `route()` / drag half (R).

## 8. Bulk corpus, disk

Bulk corpus (the verifier's 2,169 inputs) not re-run this round. Disk: 12 GB free; `B/target-on`, `B/target-ont` and
`B/target-dbg` (debug probe) kept for the verifier.
