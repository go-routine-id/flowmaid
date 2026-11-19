## b12a2b-base (2026-11-17, on top of `7122a53`; read this first)

Commits (branch `ccr-ccc6d89a-5cxd7e`, not pushed): `360c588` (item 1), `57d0736` (item 2), `4d374f8` (item 3),
`b53e3de` (item 4a, lateral X2). Items 4b (X3), 5 (G3), 6 (X2.5 and the gap-step merge) and 7 (e3, e4/e5) are
**not committed**: each skipped below with its numbers. Work files: `B/b12a2b/` (`B` = `scratchpad`; scripts
`tools/{on,quick,bulk,png,pair}.sh`, `tools/bulkcmp.py`; quick runs `q-<label>/`; pictures `pic/`; bulk TSVs
`bulk/`; the measured but uncommitted attempts `x3-attempt.diff`, `g3-proto-on-head.diff`, `x2-scene-pad.diff`).
The three mandatory checks per committed item are `b12/tools-a2b/cand-item{1,2,3,4a}/` with
`cand-item*.cmp` (compare.sh against `base-7122a53`).

**Result at the head (`b53e3de`) against `7122a53`:** local judge `--due A2` **89 -> 87** must-fail lines (444
checks), `--due A1` **0**, routed 37 -> 37 (62 checks); per-cell: **no cell worse** (2 gone, 10 better inside
failing lines); suite-on failing tests: the same **14** names, **no gate message new or worse** (5 new tests, all
pass); direction copies: static `lateral_x` 216 -> 192, `side_ends` 231 -> 208, `defects`, `x_drawn`, `fan_x`,
`edge_chip` unchanged; routed unchanged; **one copy worse by the tool's rule, declared as item 3's trade**:
`blocks.swimlane-nested-TD` `min_edge_gap` 40 -> 25 (see item 3). Default build byte-identical: off-identity test
green in every guard; `render.sh post-b12a2b-base` is byte-identical to `pre-b12a2b-base` on all 25 corpus SVGs.
No CHANGELOG / README / version change (block mode is behind the switch).

| item | commit | A2 | A1 | routed | cells | gate messages | copies (vs previous item) |
|---|---|---|---|---|---|---|---|
| 1 regress `split_by_room` by plain room, fixture `twoway-noted` | `360c588` | 89 -> 89 | 0 | 37 | none changed | none; +1 test | none changed (static, routed, 364) |
| 2 `TitleSides::of` one pass | `57d0736` | 89 -> 89 | 0 | 37 | none changed (95 SVGs byte-identical) | none; +1 test | none changed |
| 3 lateral X1 + e2 | `4d374f8` | 89 -> 88 | 0 | 37 | gone: hub5 `lateral_x` 3->0; better: public x5 `lateral_x` 3->1, `lbl_crowd` 2->1 | none; +2 tests | static `lateral_x` -24 (14 copies better), `side_ends` -23 (13); swimlane-nested TD `min_edge_gap` 40->25 (declared trade) |
| 4a lateral X2 (router form) | `b53e3de` | 88 -> 87 | 0 | 37 | gone: v2 `lateral_x` 1->0 | none; +1 test | none worse; info: fork-twins TD `lbl_end_mean` 183->192 |
| 4b lateral X3 | skipped | (86) | (0) | (37, 1 cell worse) | | | 16 worse (below) |
| 5 G3 (labels p5, no dummy-row exclusion) | skipped | (66) | (0) | (36, 13 cells worse) | 17 worse | | 23 worse (below) |
| 6 X2.5 breadth bound; displaced gap-step merge | skipped | | | | | | |
| 7 e3; e4/e5 | skipped (not started) | | | | | | |

### Item 1: regress (`360c588`)

`regress/proto.diff` as the study gave it: `split_by_room` sorts and chains by `Jog::plain.0`. Fixture
`tests/fixtures/blocks/twoway-noted.mmd`, test `blockcheck::a_two_way_pair_between_blocks_keeps_its_steps_in_one_channel`
(fails at 7122a53). Identity row: the default build's blocks-off hashes `97f3b3acf179bb13 2ebf8a501a52edbd
9af2a3a1de5c6a97`; the static hash equals `B/start/flowmaid`'s SVG of the fixture (FNV-1a checked by hand), and
the off-identity test is green. `BLOCK_CEILINGS` row all 0 (default `quality` example: defects, edge_frame_pairs,
overlap_pairs, x_drawn, hard zeros 0, static and routed). Checks: no cell, no gate message, no copy changed.
Bulk (`B/b12a2b/bulk/`, suite-on `quality`, base = 7122a53's binary): `b8r1/bulkst` static `x_drawn` 308 -> 307,
`chip_crowd` 778 -> 777, `fan_x` 97 -> 96 (8ab502b3 1 -> 0, 9a94998d `ortho_edges` 7 -> 5, 3d210104
`chip_crowd` 1 -> 0; 3 files changed), routed unchanged; `bulkfc` and the 413 valid `b3r4/all` inputs unchanged.

### Item 2: `TitleSides::of` terminates (`57d0736`)

`ortho::column_clear_of`: one pass over the side's crossings sorted ascending (the column only moves forward). Unit
test `ortho::tests::a_displaced_column_is_found_in_one_pass_and_always_ends`: same column as the old search on 2000
deterministic draws, and an end at 1e16 where the old search loops. No output change (95 local SVGs byte-identical,
no copy, gate message or cell changed).

### Item 3: lateral X1 + e2 (`4d374f8`)

- **X1**: `ports::unshaded` cuts a block layout's **node** side at `ortho::CHIP_CLEAR` (layout `title_cap` and ortho's
  title sides already did). Frame sides keep `CHIP_SEP`: cut there too, crowded pins on K0's top in
  `notes/spread-bottom` BT moved 0.2-0.5 px and the router sent `K0 -> K2N` another way (copy `x_drawn` 1 -> 3,
  `fan_x` 0 -> 2), so that half was left out (measured, `q-x1` / `q-e2b`).
- **e2**: `ports::title_side_columns` (after `free_pins`, `order_pins`; TD/BT): a frame's free pins on its top and
  the entries from outside the frame into its title-row members partly under the chip are one set; where a pin and
  such an entry stand closer than `PORT_SPACING`, that run (taking in the next set end when it would run into it)
  is re-dealt from its start (not before the chip's clearance) in the order the channels come in, `PORT_SPACING`
  apart or `TRACK_PITCH` where that does not fit, an entry on its member's port range, up to a spacing short of the
  side's next foreign crossing. Where neither fits, `place_ports` keeps today's `CHIP_SEP` cut on that member's side
  (`title_side_over_full`): the public fixture's Observability title row (three scrape entries into METRICS plus
  OPS's dashboards pin past a 37 px stretch) keeps its lateral `[WEB] -> METRICS` entry, as at 7122a53.
- **"pins counted in F2" is only half built**: the ports stage counts the frame's pins against the member's stretch
  (the fallback above); the layout's and `route()`'s jog-room count (`title_entries`, `route_title_jogs`) do not.
  Counted there without X3's whole-member displacement, the OBS frame would only gain an unused jog room (METRICS is
  partly shaded, so ortho displaces none of its entries); it belongs with X3 (item 4b), which is skipped.
- **lr BT S-jog**: gone; `cart -> pager` comes down straight into pager (`pic/e2/lrbt.pair.png`), its label moved
  out of Storefront into the gap; harness rows of lr BT unchanged (x_drawn 1).
- Cells: A2 89 -> 88 (hub5 `lateral_x` 3 -> 0 gone); better inside failing: public fixture and its four
  permutations `lateral_x` 3 -> 1 (QUEUE>CHARGE by X1, CART>QUEUE by e2; [WEB]>METRICS stays) and `lbl_crowd` 2 -> 1;
  drift inside passing: public `edge_len` 15483 -> 15621, two `lbl_endpx_mean` 231 -> 236. Routed cells unchanged.
- Copies: static `lateral_x` 216 -> 192 (14 copies better: hub5 TD 3->0, public family TD 3->1 x8,
  swimlane-nested TD 7->6, ...), `side_ends` 231 -> 208; routed unchanged.
- **Declared trade**: `blocks.swimlane-nested-TD` `min_edge_gap` 40 -> 25. `place order -> confirm order` now comes
  in through Shop's title side beside `confirm order -> pay invoice` leaving the same side (25 px apart, the BT/LR/RL
  copies' own value) instead of through Shop's lateral wall (`lateral_x` 7 -> 6, `side_ends` 1 -> 0; picture
  `pic/x1/blocks.swimlane-nested-TD.pair.png`). 25 px is above every gap floor; no judged cell or gate moves.
  Decision 5 names trades only for failing rows: **the lead decides whether this one stands**.
- Bulk against item 1: `bulkst` unchanged but one idiom count; `bulkfc` static `lateral_x` 183 -> 172, `side_ends`
  355 -> 340, `x_drawn` 86300 -> 86297, `edge_chip` 47 -> 46, `chip_crowd` 657 -> 661 (four files +1); routed and
  `b3r4/all` unchanged. (A first version of e2 re-dealt every pin and member end of the side, inner edges included,
  and took `bulkst` `x_drawn` 307 -> 311, `fan_x` +2: `pic/bulk3/`; the committed rule only re-deals a colliding run
  of border entries and pins, and the commit was amended to it before any later item.)
- Tests: `a_title_rows_first_free_entry_clears_the_chip_ring` (hub5), `a_frames_pins_and_its_title_rows_entries_take_columns_apart`
  (public fixture and four permutations TD); both fail at 7122a53.

### Item 4a: lateral X2, the hug tie (`b53e3de`)

Probe (`FMDBGH`, scratch build): v2's `LANE>AZ` column stands 11.897 px from PROD's wall (F3 put the wall at
`LANE_PAD`; the simplified path's corner put the leg's middle 0.103 px inside), `hug 146 > HUG_TRADE`.
- The study's form (F3 pad `LANE_PAD + 0.5`, `x2-scene-pad.diff`, `q-x2`): v2 1 -> 0, but **13 already-failing
  `compact_max` cells worse by 0.01** (analog, perm-a..e, public x3, bench-75 71.64 -> 71.71, dense-lr 43.03 ->
  43.04) that nothing in this package pays back, and `quality.dense-td-RL` `x_drawn` 29 -> 30. Not taken.
- The router form in all four directions (`q-x2r`, `q-x2f`): no compact change, but the same dense-td RL crossing and
  a 290 px frame hug (harness `hug_len`) on that copy. Not taken.
- **Committed**: in a **TD/BT** block layout `router::side_run` counts a run beside a **frame's** border only within
  `CLEAR_GAP - HUG_AIR` (0.5); node sides and every other layout keep `CLEAR_GAP` (`World::hug_gap`). A2 88 -> 87
  (v2 `lateral_x` gone); no cell, gate message or copy worse; informational: fork-twins TD `lbl_end_mean` 183.1 ->
  192.4. Unit test `router::tests::a_leg_a_clearance_inside_a_frame_wall_rides_it_only_outside_a_block_layout`.
  `route()` takes the same rule (the repair runs there with `cx.blocks`).

### Item 4b: lateral X3 (skipped, `x3-attempt.diff`, `q-x3`)

Built: `layout::title_cap` on the port span (side less `PORT_INSET`), all-or-nothing displacement
(`layout::title_displaced`, used by `title_entries` and `route_title_jogs`), `ports::unshaded` puts an over-full
title-row side's ends strictly under the chip, ortho's `TitleSides` displaces any title-row end under the chip.
Measured on top of item 4a: A2 87 -> **86** (cycle2 `lateral_x` 2 -> 0 gone; dense-td 6 -> 2, public-bt 3 -> 2
better), A1 0; but **routed cycle2 `lateral_x` 3 -> 4 (cell worse)** and **16 copies worse**: static
`sub-nested-self` TD/BT defects 2 -> 4 and `min_edge_gap` 7 -> 0.88 (the own-box stubs OUT<->IN on top of
`w2 -> store` once Workers takes jog room), `dense-td-TD` defects 0 -> 1 and `x_drawn` 42 -> 43, `bench-75-BT`
`lateral_x` 5 -> 8 and `side_ends` 8 -> 10, `bench-75-TD` `fan_x` 7 -> 9; routed `bench-75-BT` `x_drawn` 259 -> 262,
`cycle2-TD` `lateral_x` 3 -> 4, `edge_chip` +5 (b9-inside TD, cycle2 TD, sub-nested-self TD/BT, bench-75 BT). The lead's
condition for X3 ("only once its own block_fixtures rows and bench-75 TD/BT copy changes are gone") is not met; the
gate tests were not run on it. Not committed. What it needs: the own-box stubs' room in a jog-roomed frame (B12.14),
bench-75's BT displaced columns failing `TitleSides`' "no crossing between port and column" (ends left under the
chip get rerouted), and the routed displacement.

### Item 5: G3 static (skipped, `g3-proto-on-head.diff`, `q-g3p`)

The labels prototype applied cleanly to the head; measured with its p5 constants (`FM_G3_OFF_FRAC=0.155`,
`FM_G3_CHAIN_OFF=0.34`) and **without the dummy-row track exclusion** (the critic's `NOROWS`), TD only, against item
4a: A2 87 -> **66** (gone: `lbl_crowd` 9, `lbl_in_end` 6, `lbl_struck` 4, `lbl_gap_min`, `lbl_own_gap_max`,
`detour_max`, `crossings` 1 each), A1 0, routed 37 -> 36; but **17 static cells worse**: NEW analog `lbl_off_max`
0.36 (must 0.35); `lateral_x` 1 -> 2 on the analog, perm-a/c/d and the public family x5 (`DNS -> TR1`'s lower band at
TR1's port under PROD's chip; `REG ==> [ORD]`); `area` analog 4.51 -> 5.58 M, v2 5.09 -> 5.57 M; public family
`compact_mean` 7.26 -> 7.30; **13 routed cells worse** (`lbl_struck` 2 -> 4/5 on the analog family, `lbl_crowd`
6 -> 8 on perm-a/b, `crossings` 26 -> 28 on the analog and perm-a/c/d, 30 -> 32 on perm-e); **23 copies worse**
(public family TD static `lateral_x` 1 -> 2 and `side_ends` 3 -> 4 x8, `fork-twins-TD` `x_drawn` 8 -> 10, routed
`dense-lr-TD` `x_drawn` 24 -> 29, `lateral_x` 7 -> 10, `fan_x` 0 -> 2, `edge_chip` 3 -> 5, `spread-face-TD`
`side_ends` 0 -> 1). Besides fixing those, a committable G3 needs the prototype's thread-local side channels
(`G3_AT`, `G3_SLID`, `G3_CHAIN`) replaced by data carried with the placement, its env knobs made constants, BT,
and the slot rule in the router / nudge / comb / label stage (item 5's own text), none of which the prototype has.
Not committed; the A2 gain is real but the regressions above stand.

### Item 6: X2.5 breadth bound and the displaced gap-step merge (skipped)

- X2.5: the lead's text asks it measured with G3 present; the lateral study's X4 prototype alone trades the
  analog's `lbl_endpx_mean` 383 -> 389 (must 386: a NEW must-fail line) and broke two A1-held tests in its
  shift-after-`TITLE_FIT_PASSES` form. With G3 not landed, building it would add that must-fail line. Not built.
- Gap-step merge (6 turns -> 4): tried as "a chain's last run aims at its displaced entry's column, not its port"
  (`straighten` target and unpinned last column, `q-gm`): **no cell, copy or SVG changed**, because the edges behind
  the 7 `bends_max` lines (`DNS>PSDEV` on the analog family, `DNS>PSPROD` on v2) are labelled, and a labelled chain
  keeps the column its label's dummy reserved (`ortho_pass`: no `straighten`). Moving that column is G3's label
  slot. Reverted; nothing committed.

### Item 7: e3, e4/e5 (skipped, not started)

Not attempted in this package: the budget went into items 3-6 and their measurements. Neither was prototyped by the
studies (lateral 3.7, 3.8 give the mechanism and a prediction only: infra-bt 1 -> 0; dense-lr 5 -> 0-1).

### Suite-on, guard, perf, disk

- Suite-on failing tests at every commit: the 14 names of `base-7122a53/gates/failing.txt` (panics.py: no message
  new or worse; at the head two informational stdout cells: `defect_baseline_gate` static `lbl_end_mean` 222.135 ->
  222.181, `quality_gate` dense-td static `lbl_end_mean` 739.32 -> 739.59, both under their ceilings).
- Default guard `GUARD: ALL GREEN` before each commit (and before each amend).
- `perf.sh` (default build; this package changes no default code path): runs `b12a2b-base-1` (after item 3),
  `b12a2b-base` and `b12a2b-base-r2` (head). Every file within its budget in runs 1 and 3; run 2 had dense-td
  `route_ms` 10.06 (limit 9.20) and `scene_ms` 38.95 where runs 1 and 3 have 6.86 / 6.83 and 27.35 / 26.05: the runs
  disagree, no regression claimed. Worst drags: stress 1.43-2.10 (limit 2.15), lr 1.35-1.41 (1.95), advanced
  1.67-1.82 (2.61), bench-75 18.43-20.34 (25.28).
- Disk: 8.6 GB free at the start, 8.0 GB at the end; scratch target `B/b12a2b/target-on` deleted after this report.
