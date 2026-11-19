# Checkpoint A2: geometry (B12.7-B12.12)

## Fix round 2 of 2026-11-17 (on top of `4a196db`; read this first)

Commits: `1c6cb2f` (#4 bold widths past ASCII), `c8249f4` (#1/#2: B12.10, one frame function in `scene()`,
`route()`, `route_partial()`; F3 on a drag; the layout's F2 count made spec-conformant), `9f51f77` (#1: G3, a label
keeps off its end blocks; lr `lbl_in_end` 4 -> 1). Work files `B/work/a2g/` (probe crate `probe/` with `frdiff`,
`frdiff2`, `pdiff`, `onsvg`, `meas`, `details`, `dump`; pre / head worktree probes; 4-direction copies of every
repository fixture `dirs/` (412 files); suite-on test outputs `ton-*.txt` / `.fail`; local runs
`B/local/out/{r1,g3}`; experiment diffs `x22-column-room-chipside.diff`, `b128-squeeze-on-9f51f77.diff`; suite-on
binaries `fm-on-9f51f77`, `quality-on-9f51f77`).

**Result: the A2 gate is still not met (local judge 91 must-fail lines, was 98; A1 judge 0). D6 stops below for the
squeeze (B12.8, every direction), the chip half (#3) and X2.2's column room (#6), each with its numbers.**
Nothing was re-baselined or loosened; no README / CHANGELOG / version change; default build byte-identical
(off-identity test green in every commit's guard).

| verifier item | result |
|---|---|
| 1 lead decision not carried out | F3 built first (`c8249f4`), then the squeeze retried per direction: **stopped (D6), every direction** (below). G3 for lr: **done**, `lbl_in_end` 4 -> 1 (`9f51f77`; the start had 2) |
| 2 scene / route / route_partial frames differ | **fixed** (`c8249f4`): numbers below |
| 3 x3 TD title strike (chip half) | **stopped (D6)**: three rules tried, each breaks A1-held tests (below) |
| 4 bold widths past ASCII | **fixed** (`1c6cb2f`): x7's two titles end 16.7 / 11.5 px inside their frames (were 40.5 / 3.2 px past) |
| 5 A2 gate 98 must-fail | **91** now (lines below); not met |
| 6 title-sized frames, lateral entries | **built and stopped (D6)**: fixes the inputs, breaks one A1-held narrowing test by 14 px (below) |

### #2 / B12.10: one frame function in three entry points (`c8249f4`)

- `route_built` takes the block mode (switch read once per entry call, `route_blocks` = the layout's predicate) and
  starts from `route_frame_base`: block mode = `frame_boxes` with the title fit (F1) and the jog room F2 counted from
  the boxes (`route_title_jogs`: R2's test on the F1 boxes for "above", title row by box overlap so a 1 px drag cannot
  change it); every builder routes against those boxes (R1 (i)-(ii)).
- The layout's F2 count (`title_counts`) now counts what F2 specifies: drawn edges ending on a title-row member.
  Stand-ins (cut back to their frame), rank-only proxy pairs and invisible links reserved jog room nobody used
  (dense-td 25 px less height; the public fixture's OBS 37.5 px), and `route()` could not count them.
- `keep_inner_room` (F3 on a drag): a frame `route_partial()` redraws round a moved member grows back over the
  waypoints of its kept edges inside its base box (never past base box U redrawn box).
- Measured, block mode (`frdiff2`, scene vs route at the scene's centres, along the flow, > 0.5 px): verifier's 294
  inputs 70 at the start (`9c142b3`), 33 with F1 alone, **21 now**; any frame > 0.5 px (`frdiff`, the verifier's
  measure): 164 at the start, 177 at `4a196db`, **134 now**; titles `route()` cuts that `scene()` draws whole: **0**.
  On the 412 direction copies of the repository fixtures: 23 left, all one of two causes said in `route_title_jogs`:
  the chip of a frame the layout's channels widened (F3, `scene()` only) stands further left (cycle2 BT, reqresp BT,
  bench-75, dense-lr TD, b9-inside2 BT, turned-blocks), or a run the layout's order moved off its member's column
  (blocks-overview and permutations BT: ORD; nested-chain TD/BT; p07-nested TD; dense-td TD g8, BT g2; wide-frames).
- 1 px drag of every node (`pdiff`, largest frame move): nested-wide-title TD 258.1 -> **1.0** (start 12.5),
  title-hidden TD 75.3 -> **0.5** (12.5), title-entry LR 60.9 -> **0.5** (1.0), title-entry TD 25 -> 0.5, x5 -> 1.0.
  Sum over the verifier's 294: start 15502, head 18141, **now 9560**; inputs above 1.5 px 166 -> 134. Above the
  start only x3-nested-lr-multiline BT 28.1 -> 80.2 and TD 25.0 -> 53.2, x4-channel-broadens 25 -> 27.5 (the
  redrawn edges' builder paths no longer take the channel the static frame grew over: R3's inner legs, B12.13).
- Tests: `frames_agree_between_scene_and_route_along_the_flow`, `a_one_px_drag_keeps_a_frame_grown_to_its_title`.
- Static changes from the F2 count (block mode): blocks-overview family TD/BT, dense-td, fanout-frames,
  nested-cases, note-edge-row, p04, p12-state, p13, spread-face, state-note, t9c two-way. Harness (quality corpus,
  static): every gate holds; dense-td `pin_corner` 0 -> 1 (`n46 -> g9`'s cut pin now `PORT_SPACING` from g9's corner:
  free pins, B12.11); dense-td routed defects 21 -> 17, `fr_max` 87.5 -> 49.9.

### #1 G3: labels off their end blocks (`9f51f77`)

blocks/lr LR `lbl_in_end` 4 -> **1** (the start had 2): `browse`, `miss`, `page` (cart -> PAGER) now in the gaps; left
is `db -> ship`'s `order`, whose gap between Storefront and Orders (32 px) is narrower than its chip (46.8), as at
the start. Local: `lbl_in_end` analog 3 -> 1, v2 3 -> 1, perms 3 -> 1 / 3 -> 2, public fixture family 1 -> 0. Cost:
`lbl_crowd` +1 on the analog family (3 -> 4, v2 6 -> 7), dense-td `chip_crowd` 5 -> 8, v2 `lbl_endpx_mean` 382 -> 392
(labels moved into gaps already holding lines: G3's `CLEAR_GAP` reservation is not built). Test
`a_label_keeps_off_the_blocks_that_hold_one_end_of_its_edge`.

### #1 / B12.8: the squeeze, retried with F3 in place, one direction at a time (D6 stop)

Patch `a2g/b128-squeeze-on-9f51f77.diff` (the earlier X1 + X2/X3 + X4 build, gated per direction for the
experiment). Suite-on failing tests added against this round's list, per squeezed direction:

| direction | tests added to the failing list |
|---|---|
| TD | `a_hub_fan_leaves_in_target_order` (public fixture routed `fan_x` 1), `a_hubs_fan_keeps_its_order_through_the_bands` and `an_end_of_a_title_row_member_...` (dense-td `n16`: 4 fan crossings), `casing::a_line_takes_one_role_...` (routed 34 > 3), `frames_agree_between_scene_and_route_along_the_flow` (ORD 12.5 px: a run moved under the chip) |
| BT | the TD list less `frames_agree`, plus `comb::a_block_layouts_comb_is_settled_after_one_pass`, `comb::combing_is_settled_after_one_pass` (dense-td 2), `edges_keep_off_frame_borders` |
| LR | the BT list plus `a_choices_spilled_and_cornered_ends_keep_off_each_other`, `a_generated_sweep_keeps_the_block_invariants`, `a_turned_step_keeps_off_the_runs_of_a_fan_it_crosses`, `scene::sibling_frames_never_intersect` (frames P and D) |
| RL | the BT list plus `a_generated_sweep_keeps_the_block_invariants`, `scene::sibling_frames_never_intersect` |

(X1's alignment rule and X4's first-row shifts act in every direction, which is why a BT-only squeeze still moves
the TD fixtures.) No direction holds the A1 tests: B12.8 is not on the branch; `BLOCK_SLACK` is not pinned.
`compact_max` / `compact_mean` / `sib_stagger` / `bends_max` stay on the must-fail list (section "Rows" below).

### #3 chip half (D6 stop)

x3's OUTER is not grown to its title (its members, the grown INNER frame, are 267.6 px broad, its title fit 258), so
it keeps the regular chip; its title's bold ink runs 17 px past that estimate and `in -> p1` runs through "here"
(Chromium, unchanged since `4a196db`). Three rules tried, suite-on, each against this round's list:
- every block-layout title bold: + 5 tests (dense-td hub fans, both comb-settle tests, `a_hubs_fan_is_a_comb...`);
- bold for frames F1 grew, also when channels broaden them: + `an_end_of_a_title_row_member_...` (dense-td `n42` Top
  `n28 -> n42` x `n25 -> n42`), + `edges_keep_off_frame_borders`;
- bold where the bold ink passes the estimated chip by more than `CHIP_PAD_W / 2 + CHIP_SEP` = 10 px (x3: 17.0;
  dense-td's titles 10.9-13.5): the same two; with the earlier `ports-widen.diff` the fan test passes but dense-td
  static crossings 49 -> 61 and routed `border_len` 1426 -> 2178 (gate 1895).
None committed. The strike on x3 TD stays (a regression against the package start, where the title was cut).

### #6 lateral entries into title-sized frames (D6 stop)

Built (`a2g/x22-column-room-chipside.diff`, and a symmetric variant): a frame F1 grows to its title also holds its
displaced entries' columns past the chip (`title_column_width` = chip + `CHIP_CLEAR` + `CORNER_R` + (k-1) x
`TRACK_PITCH` + `LANE_PAD`), reserved in the layout's F1 passes. Static `lateral_x`: x6 TD 4 -> 2, BT 4 -> 1;
nested-wide-title TD/BT 1 -> 0; title-entry TD 2 -> 0, BT 1 -> 0; title-hidden TD 1 -> 0. It breaks the A1-held
suite-on test `layout::a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` (1275.8 x 631 against width < 2 x height:
the column room of `eiusmod ut labore elit magna` adds 26.5 px to the hub's row; growing on the chip's side instead
gives the same width). Not committed.

### Rows of 4.3 at `9f51f77` (static, suite-on CLI; `B/local/out/g3`)

Local judge `--due A2`: **435 checks on 92 diagrams, 91 must-fail** (98 at `4a196db` and at the start), `--due A1`:
**0**. By metric: `lateral_x` 18, `compact_max` 14, `sib_stagger` 12, `lbl_crowd` 12, `lbl_in_end` 7, `compact_mean` 7,
`bends_max` 7, `lbl_struck` 5, `bends` 2, `area` 2, `straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `lbl_endpx_mean`,
`detour_max` 1 each. By input: public fixture family 25, analog 14, v2 10, perms 6-7, dense-lr 3, infra-bt, hub5,
dense-td, cycle2, blocks-overview-bt, bench-75 1. Changes against `4a196db`: public fixture `compact_max` 25.16 ->
19.97, `compact_mean` 8.00 -> 7.26 and `lbl_in_end` 1 -> 0 (all five copies); analog `compact_max` 10.37 -> 10.27,
`bends` 144 -> 142, `area` 4.60 -> 4.51 M, `straight_max` 1601 -> 1591, `lbl_in_end` 3 -> 1, `lbl_endpx_mean` off the
list, `lbl_crowd` 4 -> 4; v2 `bends` 153 -> 149, `lbl_in_end` 3 -> 1, `lbl_crowd` 6 -> 7, `lbl_endpx_mean` 386 -> 392;
perm-b `lbl_struck` off the list, `bends_max` 13 -> 12.

B12.12's rows: dense-lr static area 3232.9 x 3362.9 = **10.87 Mpx2 (must <= 7.98)**; the "narrowing still accepts a
candidate" row was not measured (no switch disables narrowing; not built); the two narrowing tests pass as written.

### Suite-on failing tests, default run

Suite-on at `9f51f77` (`ton-g3.fail`): A1's 15 names less `edges_keep_off_frame_borders` (passes since `c8249f4`).
Default run: `GUARD: ALL GREEN` before each of the three commits; off-identity test green.

### Perf (4.6), default build

Two `perf.sh` runs (`runs/b12a2-fr2-1`, `-2`). Run 1 was taken while another session's `vprobe` loaded the box (load
1.9-3.3): advanced `scene_ms` 1.56 and `drag_ms` 3.80 over their limits (1.42, 2.61). Run 2 at load 0.86: every file
inside its budget (advanced 0.81 / 0.62 / 1.55; bench-75 32.48 / 8.54 / 14.39; dense-td 22.12 / 5.91 / 9.95; dense-lr
17.08 / 3.75 / 9.03; lr drag 1.35). The default build's code paths are unchanged by this round (every new branch reads
block mode), so run 1's numbers are load. `fixture_drag_perf_guard`, `route_partial` worst frame, default build:
dense-lr 5.09, dense-td 5.87, bench-75 6.79 ms (the start: 5.40-7.96 on bench-75 over its two runs); suite-on: dense-lr
4.70, dense-td 7.49, bench-75 7.49, public fixture 3.67 ms (block mode's route_partial is R's).

### Disk

8.8 GB free at the round's start, 6.0 GB before cleanup; the suite-on target `B/target-on` and the round's probe
targets and worktrees (`a2g/ptarget*`, `a2g/pre-tree`, `a2g/head-tree`) are deleted after this report.

---

## Fix round of 2026-11-17 (on top of `acacab7`; read this first)

Commits: `64da873` (fix #1: bold title measure), `4a196db` (docs: #3, #4, #5 recorded in the code and history).
Work files `B/work/a2f/` (scripts `bon.sh`, `ton.sh`, `measure.js`, `titlefit-all.js`, `zoom.js`; suite-on outputs
`ton-fix1b.txt` / `.fail`; pictures `pic/fix1-*.png`; local run `B/local/out/f1`; experiment patches
`bold-everywhere-experiment*.diff`, `ports-widen.diff`, `f1-only.diff`; suite-on binaries `fm-on-64da873`,
`quality-on-64da873`).

| verifier item | result |
|---|---|
| 1 bold titles overflow grown frames (BLOCKING) | **fixed for every frame F1 grows to its title**; the chip half is **stopped (D6)** for frames broader than their titles, numbers below |
| 2 A2 gate: 98 must-fail lines | **unchanged: 98** (same lines, `B/local/out/f1/judge-A2.txt` vs `g1`); the D6 stop stands, the lead / user decides |
| 3 grown frames take labels in, unreported | **recorded** (`4a196db` message and the frame function's doc); not fixed: it is G3 (B12.9), under the stop |
| 4 wrong claim about t9b-frames-8px-lr | **corrected** here (section 2) and in `4a196db`'s message (acacab7 cannot be amended) |
| 5 title-sized frames push entries into lateral walls | **recorded** (`4a196db`); not fixed: it is X2.2's breadth (B12.8 / B12.11), under the stop; measured below |
| 6 history: unverified wip `395147c` | nothing to do on the branch (reverted by `b3b2f17`; the lead squashes at merge) |

### Item 1: what was built (`64da873`)

- `layout::bold_text_width`: a Helvetica-Bold advance table (AFM per 1000 em, x 14 px, rounded up to 0.1). Against
  Chromium (Liberation Sans Bold, metric-compatible) on the 118 subgraph titles of the repository fixtures and the
  verifier's stress inputs: never short by more than 1.12 px (glyph ink past the advance, e.g. `j`), long titles
  0.5-1.5 px over; the regular estimate was short by up to 24.3 px (`B/work/a2f/measured.tsv`, `cmpw.py`).
- F1 (`scene::title_fit_width`, used by the scene's frame function and the layout's reservation of it) measures bold.
- A frame F1 grows to its title and that nothing else makes broader is *title-bound* (`SceneCluster::bold_title`,
  crate-private): its chip is measured bold wherever a chip is read (`title_head`, `fit_cluster_title`, the
  painter's chip, the label stage's and the harness's chips), and so is the layout's `title_chip` (X2.5, jog count).
- Result, Chromium + the verifier's `titlefit.js`: no title ends within 2 px of or past its border on any of the 98
  block fixtures and stress inputs drawn in block mode (before: title-hidden 11.9, title-entry 7.7 / 13 (LR, RL), lr
  5.7, s1 9.5, s2 10.6 / 8.9, s4 14.3, s5 15 px past the border); every such title now ends 10.5-11.5 px inside
  (chip pad + title inset). On the nested fixture (new `blocks/nested-wide-title`, synthetic, the s4 case) the
  entry no longer runs through the title's last word (`pic/fix1-s4-nested-wide-title.png`, `pic/nwt.png`).
- Holds: default guard ALL GREEN; off-identity green (new fixture's row recorded from the default build); suite-on
  failing list = A1's 15 names with the same failure texts; the quality corpus's harness rows unchanged (dense-td,
  dense-lr, bench-75, fanout-frames identical); local A1 judge 0 must-fail; local A2 98 (same lines).
  Local inputs changed: the LR / RL copies of the analog (`infra-rl` `lbl_struck` 1 -> 0, `lbl_gap_min` 0.0 -> 3.7,
  `lbl_detach_max` 9.0 -> 3.3, i.e. back to the values before `bb48c30`; `infra-mini-lr` `lbl_in_end` 3 -> 4;
  frames 12-14 px longer), and the repository's title fixtures (frames 10-20 px broader; `compact_max` up by
  0.3-0.8 on lr, title-entry, title-hidden, p03).

### Item 1: what is stopped (D6), with numbers

Measuring **every** block-layout chip bold (the consistent rule; patch `bold-everywhere-experiment2.diff`) breaks
A1-held musts on dense-td, whose hub fans are the fragile ones of checkpoint A1 section 3:
- `a_hubs_fan_keeps_its_order_through_the_bands` / `an_end_of_a_title_row_member_...`: fan crossings `n16 Bottom
  n16->n35 x n16->n32` (the router's reroute of `n16 -> n35` is chaotic in the chip widths: it passes with the
  regular chips of all titles whose bold/regular difference is under 9.5 px, fails from 9 px, the 9.3 px of `g9`'s
  title) and `n42 Top n25->n42 x n28->n42` twice; `comb::a_block_layouts_comb_is_settled_after_one_pass` (dense-td
  `n16 -> n32`, `n16 -> n35`).
- Two workspace fixes cleared those: a comb pass after the label stage (block layouts), and, on a block layout's
  title-row side whose free stretch is short of its ends, taking only the shortfall of the chip's shade instead of
  the whole side (`ports-widen.diff`: `n42`'s top had 32 px free of the 42 its four ends need; it jumped to the
  whole side and the router took `n25 -> n42` along `g9`'s title strip across the fan).
- What stays: dense-td static `min_edge_gap` **11.12 < GAP_FLOORS 12.1** (`n16 -> n43` beside `n28 -> n42`, 59 px
  in a crowded channel, columns 9-11 px apart after the layout moved; `spread_close` does not clear it), so
  `a_free_pin_keeps_off_a_members_exit_past_the_chip` fails. Not committable.
- So frames broader than their titles keep the regular chip, as every frame in the default build has: a stroke kept
  `CHIP_SEP` off such a chip can still touch the bold ink when the title's bold width exceeds its estimate by more
  than 10 px (e.g. repository `lr` drawn BT: `cart -> PAGER` runs into the last letter of EDGE's title, before and
  after this round). The fix needs B12.11's title-clear rule 1 in full (the crossing ends on the free stretch) and
  the node-side end order, then the chips can all be bold.
- Also seen, not B12's: `blocks/p13-inner-dir-ext` (a B9 direction block, not a block layout) runs its title 114 px
  past its frame in the default build, the starting release and now alike.

### Items 3 and 5: numbers at `64da873` (static, suite-on; verifier's stress inputs and repository fixtures)

| input | lateral_x acacab7 -> now | lbl_in_end acacab7 -> now |
|---|---|---|
| blocks/lr (LR) | 0 -> 0 | 4 -> 4 (2 before `bb48c30`) |
| s1-wide-titles-td TD / BT | 4 -> 4 / 5 -> 5 | 0 |
| s2-lr-long-titles LR / RL / TD / BT | 1 -> 0 / 1 -> 0 / 5 -> 5 / 4 -> 4 | 0 |
| s3-nested-linked TD / BT / LR / RL | 4 / 5 / 2 / 2 (unchanged) | 1 / 1 / 0 / 0 |
| s4-nested-wide-title TD / BT / LR / RL | 1 / 1 / 0 / 0 (unchanged) | 0 |
| blocks/nested-wide-title (new) | 1 | 0 |
| title-entry / title-hidden | 2 / 1 (unchanged) | 0 |

Cause of item 5 (TD/BT): ortho displaces an entry under the chip to a column past it (`TitleSides`), but a column
that would leave the frame is not taken; a frame exactly as broad as its title has no such column, so the router
takes the entry in through a lateral wall. X2.2 reserves `TRACK_PITCH` per displaced entry past the chip and F3
grows the frame over the column. Not built: it widens dense-td's `g7`, `g9`, `g10` and the public fixture's `EDGE`,
`OBS` (the blocks with jog room), the same layout moves that broke dense-td's fans above. Cause of the LR rows
(s2): the title-length growth puts a gap's Z step inside the grown frame (G1's `reach_of` with F2's growth).

### Perf, disk, A2

- `tools/perf.sh b12a2-fix1` (default build; the round's code paths are block-mode only): every file inside its
  budget, totals static `scene_ms` 30.97 -> 40.86 (the per-file 2x rule holds: dense-lr 23.40 -> 16.33, dense-td
  25.34 -> 21.99), routed `route_ms` 12.61 -> 9.98, worst drag 20.22 -> 16.84, lr drag 0.95 -> 1.16 (inside base +
  1.0). One run: no regression is claimed (the default build's output is byte-identical, off-identity test).
- Disk: 8.8-9.0 GB free throughout; `B/target-on` (56 MB) deleted after this report; the suite-on CLI and quality
  example of `64da873` kept as `B/work/a2f/fm-on-64da873`, `quality-on-64da873`.
- The A2 gate is not met (98 must-fail lines, the same as at `acacab7`); the stop of D6 at B12.8 stands as
  section 1 below describes. Nothing was re-baselined or loosened.

## Run of 2026-11-16 (second resume)

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`, branch
`ccr-ccc6d89a-5cxd7e`, `pre` = `9c142b3`. Work files: `B/work/a2c/` (scripts `bon.sh`, `ton.sh`, `pic.sh`, `hc.py`;
harness tables `h-*.tsv`; suite-on test outputs `ton-*.txt` / `.fail`; pictures `pic/`), local runs under
`B/local/out/{c1,c3,g1}`.

**Result: the gate of A2 is not met. The stop rule fired at B12.8 (decision D6): the coordinates item could not be
built without failing tests that hold at A1, so it is not on the branch.** What is on the branch is green and holds
every must of A1. Nothing was re-baselined or loosened.

## 0. Commits of this run (on top of `9c142b3`)

| commit | what | state |
|---|---|---|
| `bb48c30` | B12.7, F1 + F2 (one frame function; a frame as broad / long as its title; jog room) | from the earlier run; verified here: guard green (`B/work/a2b/guard-b127.txt`), suite-on failures = A1's list (`B/work/a2c/ton-b127.fail`) |
| `395147c` | wip of B12.8 (X4 by moving whole blocks), committed unverified at the restart | verified here: **breaks five A1-held tests** (dense-td hub fan, comb settle, casing); the default guard fails on them |
| `b3b2f17` | revert of `395147c` | tree identical to `bb48c30` (diff 0 lines) |
| `acacab7` | B12.9 G1, the linked-frame floor (`LINKED_FRAME_GAP` = `FLOW_MIN_GAP` + 1) | guard **ALL GREEN**; suite-on failures = A1's list exactly; local A1 judge 0 must-fail |

Commands: default `tools/guard.sh R`; suite-on `RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=B/target-on cargo
test --release --no-fail-fast --lib --test quality` (`B/work/a2c/ton.sh`; release profile, the lib tests take 25 s);
local static acceptance `B/work/a2b/on.sh <label> A2` (= `B/local/run.sh fm` + `judge --due A2` and `--due A1`);
harness `quality <dir>` from the suite-on build.

## 1. B12.8 (coordinates): built, measured, not committable

Built in the workspace exactly as 2.5 specifies, starting from the earlier run's experiment
(`B/work/a2b/x2-squeeze-experiment.patch`, debug knobs removed): X1 (no Brandes-Köpf alignment between chain dummies
of different cluster paths), X2/X3 (`block_compact`: per level deepest first, keep Brandes-Köpf within `BLOCK_SLACK`
= 4 px, else squeeze to `W_min` seeded by Brandes-Köpf with spine merges and `COMPACT_SWEEPS` median sweeps; a child
block rigid at its parent's level), X4 rewritten to move **first-row members only** (the wip moved whole blocks),
deepest sibling groups first, LR/RL title-length growth included. One bug found and fixed on the way: a merged spine
could lay two members of one rigid unit over each other on a shared layer (generated sweep seed 16: blocks B0 and
B5 overlapping on layer 8, `chains_keep_one_side_of_a_block` failed); `pack_units` now refuses such a merge.
The clean TD/BT variant is kept as `B/work/a2c/b128-x1x2x4-td.patch` (applies on `acacab7`), with its two tests
`a_block_is_squeezed_to_its_content` and `a_spine_inside_a_block_is_straight` in `B/work/a2c/b128-x1x2x4-attempt.diff`.

What it achieves (static, suite-on; `c1` = all directions, `c3` = squeeze in TD/BT only; head `g1` for reference):

| input | compact_max g1 -> c3 (must) | compact_mean g1 -> c3 (must) | crossings g1 -> c3 (must) |
|---|---|---|---|
| analog | 10.37 -> 7.07 (<= 5.5) | 5.65 -> 4.18 (<= 3.9) | 15 -> 17 (<= 20) |
| v2 | 16.25 -> 6.12 (<= 5.5) | 7.63 -> 3.86 (<= 3.9) | 14 -> 16 (<= 17) |
| public fixture | 25.16 -> **6.70** (<= 7.97) | 8.00 -> 4.74 (<= 4.63) | 6 -> 5 (<= 7) |
| dense-td | 12.45 -> 12.77 (ref 12.45) | | 49 -> 44 (<= 74.6) |
| dense-lr | 43.03 -> 18.43 c3 / 7.01 c1 (ref 11.78) | | 17 -> 18 / 16 |
| bench-75 | 71.64 -> 68.33 c3 / 28.36 c1 (ref 52.03) | | 204 -> 208 / 243 |

`sib_stagger` 0 on every input of the table with X4; local A1 judge 0 must-fail for both `c1` and `c3`.

Why it stops (each a test or must that holds at A1 and fails with the item; measured, pictures in `B/work/a2c/pic/`):

1. **Squeeze in LR/RL (bench-75, `c1`).** A one-row block packed to `W_min` has no freedom left (feasible interval
   of every unit = one point), so every column its members shared with their fans' far ends is gone. bench-75 static:
   `fan_x` 4 -> **17** (A1 hold: <= 6), `min_edge_gap` 15.25 -> **12.50** (`GAP_FLOORS` 14.7, not loosenable),
   defects 0 -> 5, crossings 204 -> 243; tests `a_turned_step_keeps_off_the_runs_of_a_fan_it_crosses`,
   `quality_gate` (static rows). Skipping single blocks' squeezes gives 9-26 fan crossings, skipping the root's 26;
   pulling a unit without inner links by its outer links changes nothing (all one-row units are pinned). Picture
   `pic/b75.png`.
2. **Squeeze in TD/BT only (`c3`).** Every static A1 test passes, and the failing list adds two tests, both the routed
   half: `blockcheck::a_hub_fan_leaves_in_target_order` (public fixture **routed** `fan_x` 2: `METRICS Top
   ORD->METRICS x WEB->METRICS`, `ALERT Top ALERT->PAGER x ALERT->MAIL`; starting release 0, A1 0) and
   `casing::a_line_takes_one_role_across_a_cluster_of_crossings` (routed opposite patterns 34 against `OPPOSITE_MAX`
   3). Cause: `route()` at the squeezed centres orders a side's ends by the far end's breadth while the drawn path
   leaves the other way round a frame (`digest` goes round Order processing on the right, its port is left of
   `page`'s); `WEB -> METRICS` takes its horizontal just under its frame, across `ORD -> METRICS`'s exit. Both are the
   corridor slots and routed free pins of 2.9 R3 (B12.13). The first test runs in the default guard (it forces
   blocks on), so the guard fails: not committable. Picture `pic/bo-routed.png`.
3. **X4 alone, X1 alone, or X1 + X4 without the squeeze** each break dense-td's hub fan (static `fan_x` 1-2,
   `a_hubs_fan_keeps_its_order_through_the_bands`, `combing_is_settled_after_one_pass`,
   `a_block_layouts_comb_is_settled_after_one_pass`, `an_end_of_a_title_row_member_keeps_out_...`): the fan at `n16`
   is the fragile one of A1 sections 3 (shuffles s2 / s6), whose fix is the node-side end order of B12.11.
   With the TD squeeze on, dense-td passes all of them.

The spec's own edge-shape rows also move the wrong way with the squeeze (public fixture `bends_max` 8 -> 11 on
`OPS -> [OBS]`, `detour_max` 1.22 -> 1.48 on `SHOP -> CDN`; analog `lane_gap_min` 12.4 -> 4.2), so even with the
routed half fixed B12.8's check would still fail on its own rows.

What would unblock it, in the spec's order: B12.11's node-side end order and even band (dense-td `n16`, bench-75's
fans), then B12.13's corridors and routed free pins (the two routed fan crossings), then X2 in LR/RL.

## 2. Items B12.9-B12.12

- **B12.9**: built: the second floor of G1 (`acacab7`). t9-frames-8px and its two-way variant: frames 8 -> 13 px
  apart (TD, BT), **and t9b-frames-8px-lr as well: 8 -> 13 px, canvas 284 -> 289 px** (corrected in the fix round:
  the first text said "LR/RL were 13 already", which was wrong; the verifier's `on-pre` / `on-now` SVGs show it);
  test `linked_frames_stand_further_apart_than_route_reads_as_stacked`. No other local input changes. Not built: G1's `reach_of` with F2's growth beyond what B12.7
  already reserves, G3's label slot rule and `CLEAR_GAP` reservation (the label rows `lbl_crowd`, `lbl_in_end`,
  `lbl_own_gap_max`, `lbl_struck` below). `GAP_FLOORS` hold as written (bench-75 static 15.25).
- **B12.10** (F3 in three entry points), **B12.11** (even band of free pins, node-side end order, X2.2 frame breadth for
  displaced entries, the choice corner), **B12.12** (narrowing on block layouts): not built (stop rule). Their
  narrowing tests `a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` and `wide_sibling_boxes_stack_instead_of_a_ribbon`
  pass in the suite-on run as written. `BLOCK_SLACK` is not pinned (it exists only in the workspace patch).
- flow 23 RL (lead note 2): unchanged, judge and sweep ratchet untouched; nothing built here clears it.

## 3. Rows of 4.3 at the head (`acacab7`, static, suite-on CLI; `B/local/out/g1/metrics.tsv`)

Local judge `--due A2`: **435 checks on 92 diagrams, 98 must-fail, 10 targets missed** (`g1/judge-A2.txt`);
`--due A1`: **0 must-fail** (`g1/judge-A1.txt`). By metric: `lateral_x` 18, `compact_max` 14, `sib_stagger` 12,
`lbl_in_end` 12, `lbl_crowd` 12, `compact_mean` 7, `bends_max` 7, `lbl_struck` 6, `lbl_endpx_mean` 2, `bends` 2,
`area` 2, `straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `detour_max` 1 each. By input: analog 15, v2 10, each
permutation 6-7, the public fixture and each permutation 6, dense-lr 3, infra-bt, hub5, dense-td, cycle2,
blocks-overview-bt, bench-75 1 each.

(a) analog / (b) v2 and permutations / (c) public fixture (must in brackets; NA where blockmetrics has no label):

| metric | analog (must) | v2 (must) | perm a b c d e | public (must) | public bt lr rl |
|---|---|---|---|---|---|
| xframe_runs | 0 (0) | 0 | 0 0 0 0 0 | 0 (0) | 0 0 0 |
| sib_side_linked | 0 (0) | 0 | 0 0 0 0 0 | 0 (0) | 0 0 0 |
| sib_stagger | **1** (0) | 0 | **1 1 1 1 1** | **1** (0) | 1 0 0 |
| lateral_x | **1** (0) | **1** | **1 2 1 1 1** | **3** (0) | 3 0 0 |
| compact_max | **10.37** (5.5) | **16.25** (5.5) | **10.37 10.25 10.37 10.37 10.76** | **25.16** (7.97) | 19.97 18.90 18.90 |
| compact_mean | **5.65** (3.9) | **7.63** (3.9) | | **8.00** (4.63) | |
| span_max | 0.312 (0.37) | 0.249 (0.28) | 0.31 | 0.197 (0.22) | 0.20 |
| crossings | 15 (20) | 14 (17) | 15 18 15 15 17 (20 23 20 20 22) | 6 (7) | 7 6 6 |
| bends / bends_max | **144** / **13** (120 / 8) | **153** / **13** (114 / 8) | bends_max **13** each | 86 / 8 (110 / 8) | |
| detour_max | **1.73** (1.63) | 1.47 (1.76) | | 1.22 (1.43) | |
| lbl_struck | **1** (0) | **3** (0) | **1 1 1 1** 0 | 0 (0) | |
| lbl_crowd / lbl_gap_min | **4** / **0.0** (0 / 12) | **6** (1) | **4** each | **2** (0) | |
| lbl_in_end / lbl_own_gap_max | **3** / **16.5** (0 / 1) | **3** / 0 | **3 2 3 3 2** | **1** / 0 (0 / 1) | |
| lbl_endpx_mean | **392** (386) | **386** (329) | | 237 (295) | |
| lane_gap_min | 12.4 (9.3) | 12.1 (8) | 12.4-12.5 | 12.4 (8) | |
| straight_max | **1601** (1240) | | | | |
| area | **4.60 M** (4.27 M) | **5.21 M** (3.21 M) | | 2.12 M (2.78 M) | |
| lbl_lbl, lbl_node, frame_overlap, frame_foreign_nodes, node_hits, warn | 0 | 0 | 0 | 0 | 0 |

(d), static, at the head: `lateral_x` hub5 3, cycle2 2, infra-bt 1, public-bt 3, dense-td 6 (must 1), dense-lr 5
(must 0), bench-75 2 (must 3); `fan_x` 0 on the analog family and the public fixture, bench-75 4 (start 6), dense-td 0;
`spine_off` 0 on the public fixture, hub5, spread-face, nested-composite, fork-twins (harness); `title_cut` 0 on
title-entry, title-hidden, lr in four directions (B12.7's test). (f) dense: crossings bench-75 204 (334), dense-lr 17
(51.5), dense-td 49 (74.6); `compact_max` bench-75 71.64 and dense-lr 43.03 above the reference (52.03, 11.78).

The edges and sides 4.3 (d) asks to be named were not re-derived this run (no B12.11 work); A1's lists stand
(`checkpoint-A1.md` sections 3-5).

## 4. Suite-on failing tests and the default run

Suite-on at `acacab7` (`B/work/a2c/ton-g1.fail`): lib 8 failed, `tests/quality.rs` 7 failed, the same names as
checkpoint A1's report (comb route half, `far_apart_back_edges_...`, `a_nearly_straight_pair_is_drawn_straight`,
`advanced_showcase_parses_and_renders`, own-state stub, lane in `route()`, two note-link tests; `quality_gate`
(routed rows), `block_fixtures_are_not_drawn_worse`, `defect_baseline_gate`, `channels_keep_edges_apart_and_off_chips`,
`crossing_gaps_do_not_read_as_dashes`, `edges_keep_off_frame_borders`, `a_one_px_drag_leaves_unrelated_edges_put`).
With the workspace B12.8 patch (`c3`, `ton-c3.fail`) `advanced_showcase_parses_and_renders` and
`edges_keep_off_frame_borders` pass, `a_nearly_straight_pair_is_drawn_straight` still fails, and the two routed tests
of section 1 item 2 are added; on the branch the list is A1's. Default run: `GUARD: ALL GREEN` at `acacab7`; off-identity test green.

## 5. Perf (4.6)

`tools/perf.sh b12a2-geometry` (default build, `B/../runs/b12a2-geometry/perf.tsv`): every file inside its budget
(e.g. bench-75 route_ms 12.61 -> 8.83, worst drag 20.22 -> 16.50; dense-td drag 14.24 -> 10.18; dense-lr 14.21 ->
8.78; lr drag 0.95 -> 1.30, inside base + 1.0). One run only: the default build's code paths are unchanged by this
run's commits (all new code reads block mode), so no regression is claimed or needed. `route_partial` not touched.

Block-mode static `scene_ms`, fastest of three (suite-on quality example, load 1.3-1.9): analog 8.97 (limit 11.58),
v2 8.34, public fixture 8.04 (13.6), bench-75 20.09 (61.94), dense-td 24.89 (50.68), dense-lr 10.63 (46.8), advanced
0.89 (1.42), subgraph 0.15 (0.38), fanout-frames 1.82 (4.38). The workspace B12.8 patch adds well under 1 ms on each.

## 6. Musts failed and stop rules fired

- **Stop rule D6 at B12.8**, with the numbers of section 1. A2's row is not met (section 3: 98 must-fail lines at
  the head; the B12.8 patch would take the public fixture's `compact_max` and the analog family's `sib_stagger` off
  the list but adds `bends_max`, `detour_max`, `lane_gap_min` and cannot be committed).
- Every must of A1 holds at the head: crossings (analog 15, v2 14, perms 15 18 15 15 17, public fixture 6, bt 7,
  dense-td 49, dense-lr 17, bench-75 204), `xframe_runs` 0, `fan_x` musts, nested-chain `lateral_x` 0 (test),
  dense-td `GAP_FLOORS` (12.50), the sweep's 8 added diagrams (test), the six shuffles (test).
- The starts table was not re-taken (no ordering change since A1); lead note 7's question stays open for the next
  run.

## 7. Disk

8.8 GB free at the start and the end. `B/target-on` (139 MB, the suite-on build of this run) is deleted after this
report as decision "Disk" asks; the two suite-on binaries of `acacab7` are kept as `B/work/a2c/fm-on-acacab7` and
`B/work/a2c/quality-on-acacab7`. `B/target-dbg` (earlier run) is left as found.
