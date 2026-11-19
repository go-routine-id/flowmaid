# Checkpoint A2: geometry (B12.7-B12.12)

## Fix round 3 of 2026-11-17 (on top of `9f51f77`; read this first)

Commits: `507b0d6` (#2), `4812274` (#4), `7892fb7` (#5). Work files `B/work/a2h/` (scripts `bon.sh`, `ton.sh`,
`hcmp.sh`, `qcol.sh`, `svgdiff.sh`, `rpair.sh`, `tcmp.sh`, `on.sh`; suite-on outputs `ton-*.txt` / `.fail`;
pictures `pic/x22/`, `pic/even/`; experiment patches `bold-drawn-clearance-experiment.diff`,
`even-band-experiment.diff`, `bold-plus-untangle-experiment.diff`; perf logs `perf1..4.log`; local runs
`B/local/out/{h-x22,h-fr2,h-sq}`).

**Baseline correction.** The previous round's suite-on binaries `a2g/fm-on-9f51f77` / `quality-on-9f51f77` do NOT
draw what commit `9f51f77` draws: 167 of the 412 direction copies differ from a fresh suite-on build of `9f51f77`
(`a2h/fm-on-head`, `a2h/quality-on-head`). Every "before" in this section is the fresh build.

**Result: A2's gate is still not met (local judge `--due A2` 91 must-fail lines, `--due A1` 0). Items #2, #4 and #5
are fixed; #3 and #1 (the gate) stop under D6 again, this time after B12.11's pieces were built and measured (below).**
Nothing re-baselined or loosened; no README / CHANGELOG / version change; default build byte-identical in every
commit (off-identity test green in each guard).

| verifier item | result |
|---|---|
| 1 A2 gate (91 lines), B12.11 never attempted | **B12.11 attempted** (even band of free pins, node-side end order, bold chips for clearance, X2.2 column room): the column room is committed (`507b0d6`); the others do not hold A1's tests or make drawings worse, numbers below. The squeeze re-run on the new head: local A2 91 -> **72** but 4 A1-held tests fail (D6). Gate not met. |
| 2 t9 / t9b BT lateral_x 0 -> 4 (unreported) | **fixed** (`507b0d6`): 4 -> 0 (t9, t9b, every direction); t9c two-way TD/BT 4 -> 2 |
| 3 bold titles struck where `edge_chip` sees nothing | **stopped (D6)**: the rule that fixes all three inputs (chips bold for drawing and clearance in block mode) breaks 5 A1-held tests; numbers below |
| 4 frames along the flow differ on 20 of 258 copies; test skips BT | **fixed** (`4812274`): 0 of the copies differ (every block-mode fixture of `tests/fixtures/{blocks,quality,notes}` in four directions, new test); the skip is gone |
| 5 G3 routed: `browse` inside its end block in `route()` | **fixed** for `route()` / `route_sized()` (`7892fb7`); `route_partial()`'s redrawn labels are R8 (B12.13) |

### #2: t9 / t9b drawn BT (`507b0d6`)

Cause (traced stage by stage): `a`'s exit lies wholly under `Alpha one`'s chip (displaced, 2.8 rule 2), but the
frame (82 px, broader than its title) had no column past the chip inside it, so `ortho` left the link straight
through the chip and `router::repair` took it out through the right walls. Two parts were missing:
- X2.2's breadth for a frame its members make broader than its title: such a frame with displaced entries is now as
  broad as `title_column_width` (chip, `CHIP_CLEAR`, `CORNER_R`, a `TRACK_PITCH` per further entry, `LANE_PAD`),
  grown on the chip's far side, in the frame function (scene, route, drag) and reserved by the layout;
- G1/G2: the displaced end's step back across the gap was not counted (the member's column equals the far end's), so
  the two frames stood 13 px apart, less than two border clearances; `layout::title_entries` now names each
  displaced entry and its column, and `track_need` counts its step: 13 -> 37.5 px.
Measured (static, block mode, against the fresh `9f51f77` build): t9 BT/TD, t9b BT/TD `lateral_x` 0; t9c TD/BT 4 -> 2
(its turned twin still goes round: its step and its twin's share one gap and the router reroutes one); interleave TD
1 -> 0; `compact_max` up where a frame now holds its column (t9 8.50 -> 9.69, t9c 9.17 -> 11.49, interleave
4.43 -> 4.98). Routed: interleave `edge_chip` 0 -> 1 (R). 12 direction copies change. Picture `a2h/pic/x22/
t9-frames-8px-BT.pair.png`.
Not built: the column room for a frame F1 grew to its title. It adds 26.5 px to such a frame; the hub-fed row of
`layout::a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` (A1-held) then measures 1275.8 x 631 against
width < 2 x height (the row's frame `g0` is title-bound). Grown on either side or symmetrically the result is the
same 1275.8: the layout's wall reservation (`TITLE_FIT_PASSES`) pushes the members by 1.5 x the excess, and an exact
reservation would still give 1262.5 > 1262. So title-entry, title-hidden, nested-wide-title and x6 keep their
lateral entries (D6, as before).

### #4: one jog count in scene() and route() (`4812274`)

The layout read the chip at the frame the channels inside it widen and counted runs of the block under that chip;
`route()` has neither. Now both count F2 as 2.7 states it: chip from the F1 box (bold where F1 grew the frame to its
title), entries into title-row members only (a run needs no jog room, 2.5 X2.5; X2.5 still moves runs past the chip
as drawn). `frdiff2` over the 412 direction copies: 23 copies > 0.5 px before (bench-75 BT 75, dense-td TD 37.5, the
rest 12.5), **0** after. Static changes: 40 copies (frames lose jog room nothing used); dense-td `x_drawn` 49 -> 42,
`side_ends` 2 -> 1, `chip_crowd` 8 -> 9. Routed (R): dense-td `min_edge_gap` 11.97 -> 0.60 (twins `n16 -> n31` /
`n31 -> n16` overlap 14 px), defects 17 -> 18, in the `quality_gate` routed row that already failed (its message
lists the same five rows before and after).

### #5: G3 in route() (`7892fb7`)

`route_built` applies `labels_off_end_blocks` in block mode, as `scene_clustered` does. blocks/lr LR / RL routed:
`browse`, `cache miss`, `page` off their end blocks (the test checks `route()` now; it fails without the fix).

### #3 and B12.11: what was built and why it stops (D6)

Ink census (Chromium, the verifier's `inkstrike.js`) over the 404 rendered direction copies at `7892fb7`: strikes
(stroke on the bold ink, dist < 0) on bench-75 BT `data zone`, dense-td TD `lorem labore` and `consectetur nostrud
lorem`, dense-td BT `veniam sed`, dense-td RL `consectetur nostrud lorem`; the same five at the fresh `9f51f77`
build (not added by this round). The verifier's three inputs: v2-short-labels-lr-BT -1, v2-lr-labels-end-blocks-TD
-0.9, x3-nested-lr-multiline-TD -1, unchanged by this round's commits.

Experiments, each against this round's suite-on failing list (14 names; `ton-base.fail`):

| experiment | effect | suite-on tests added |
|---|---|---|
| every block-layout chip bold, layout count included | (previous round's) | 8: hub fans x2, both comb-settle, t9 test, frames-agree, router straight step, bold-title test |
| chips bold for drawing and clearance only, the F2 count unchanged (`bold-drawn-clearance-experiment.diff`) | the three inputs -1 / -0.9 / -1 -> **9.0 / 8.9 / 22.5** px; frames still agree | 6: `a_hubs_fan_keeps_its_order_through_the_bands`, `an_end_of_a_title_row_member_...` (dense-td `fan_x n16 Bottom n16->n35 x n16->n32`), `comb::a_block_layouts_comb_is_settled_after_one_pass`, `comb::combing_is_settled_after_one_pass`, `router::a_step_between_two_ports_straightens_onto_a_shared_line` (its BT C -> D steps off the bold chip), and `a_frame_holds_its_bold_title...` (restatable) |
| the same plus a node-side end order pass after the comb (`untangle_sides`: a side's ends in the order their paths first turn, kept only when crossings fall; `bold-plus-untangle-experiment.diff`) | dense-td `n16`'s bottom unchanged (the swap does not lower the crossings: `n16 -> n35`'s reroute wiggles back across), adds `n43` twin crossings | the same 6 |
| even band of free pins, 2.8 (`even-band-experiment.diff`): two or more pins of one free interval at a pitch in [PORT_SPACING, PORT_EVEN_MAX] | no test fails once a lone pin keeps its place (spec's "lone pin at the middle" breaks `edgepass::finish_is_idempotent_with_sub_edges`: `x --> S` becomes a Z); 33 copies change, harness columns (incl. `pin_corner`, `fan_uneven`) unchanged; state-note's `tick` / `done` pins bunch and their labels crowd (picture `pic/even/state-note-TD.pair.png`) | none; not committed (worse picture, no metric gained) |
| the B12.8 squeeze (X1, X2/X3, X4; TD/BT) re-applied on `7892fb7` | local A2 **91 -> 72** must-fail (compact_max / compact_mean / sib_stagger off the public family and the perms; bends_max, detour_max, lbl_struck, lbl_own_gap_max come on) | 4: `a_hub_fan_leaves_in_target_order` (public fixture routed `fan_x` 1), `a_hubs_fan_keeps_its_order...` and `an_end_of_a_title_row_member...` (dense-td `n16` twins and fan), `casing::a_line_takes_one_role...` (routed 34); `advanced_showcase_parses_and_renders` passes |

So the three stops reproduce after B12.11's pieces: the node-side order does not settle dense-td's `n16` fan (its
crossing comes from the router's reroute of `n16 -> n35`, not from the port order), and the public fixture's routed
fan is `route()`'s corridor ordering (R3, B12.13).

### Rows (static, suite-on CLI at `4812274`; `7892fb7` changes routed output only)

Local judge `--due A2`: **435 checks, 91 must-fail** (`B/local/out/h-fr2/judge-A2.txt`); `--due A1`: **0**. By metric:
`lateral_x` 18, `compact_max` 14, `sib_stagger` 12, `lbl_crowd` 12, `lbl_in_end` 7, `compact_mean` 7, `bends_max` 7,
`lbl_struck` 5, `bends` 2, `area` 2, `straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `lbl_endpx_mean`,
`detour_max` 1 each. Against `9f51f77`: analog `compact_max` 10.27 -> 9.71, `compact_mean` 5.51 -> 5.40; v2
15.77 / 7.49; perms `compact_max` -0.56 each; perm-b `lbl_crowd` 5 -> 8, `lbl_in_end` 3 -> 1; perm-e `lbl_in_end`
2 -> 1. B12.12: dense-lr area **10.87 Mpx2** (must <= 7.98), unchanged; "narrowing still accepts a candidate" not
measured (no switch disables narrowing). `BLOCK_SLACK` not pinned (the squeeze is not on the branch).

### Suite-on failing tests, default run

Suite-on at `7892fb7` (`ton-g3r.fail`): the same 14 names as at `9f51f77`. Default run: `GUARD: ALL GREEN` before
each of the three commits; off-identity test green.

### Perf (4.6), default build

Four `perf.sh` runs (`runs/b12a2-fr3-1..4`); the box was shared (another session's benchmark and builds, load
1.1-4.4). Run 3 (load 2.0 at the start): every file inside its budget (dense-td `route_ms` 6.81 / limit 9.20, drag
13.36 / 17.80; bench-75 10.17 / 15.76, drag 19.09 / 25.28; lr drag 1.40 / 1.95; advanced drag 2.44 / 2.61). Runs 1,
2, 4 (loaded) put dense-td `route_ms` at 9.72 / 10.41 / 9.92 and lr drag at 2.13 / 1.41 / 2.11. The default
build's code paths are unchanged (every new branch reads block mode), and an interleaved head-vs-now comparison of
the default `quality` example (`tcmp.sh`, three pairs at load 3.2-5.0) shows the same spread for both builds
(dense-td `route_ms` head 6.76 / 10.33 / 9.61, now 9.60 / 7.07 / 10.27; lr drag head 2.08 / 1.36 / 1.43, now
2.12 / 2.03 / 2.18): load, not a regression. Block-mode static `scene_ms` (suite-on, two runs at load 4.0): advanced
1.18 / 1.09, subgraph 0.21 / 0.35, fanout-frames 3.52 / 3.37, dense-td 30.9 / 48.2, dense-lr 12.8 / 13.5, bench-75
31.2 / 25.0, public fixture 13.6 / 10.0.

### Disk

6.7 GB free at the start, 4.5 GB at the end (another session's files); the probe target (`a2h/ptarget`, 61 MB) and a
scratch worktree of `9f51f77` with its target were deleted; `B/target-on` (56 MB) is deleted after this report.

---

