## b12a2c-squeeze (2026-11-19, on top of `d99c073`; read this first)

**Two commits, both green** with the guard and with decision 2's three checks against the package start
(`tools-a2b/cand-d99c073`, suite-on): **`7208f50`** (a label's search on a block layout's line skips the spots inside
its edge's end blocks, G3) and **`22dac44`** (B12 X4: the top-level sibling frames of a TD / BT band start level, each
block moved whole; hop feet in a block layout's paint written on their line). **Checkpoint A2 is still not met**:
the local static A2 table is **86 -> 79 -> 68** must-fail lines of 444 (`--due A1` 0 must-fail; routed 62 cells 19 ->
18, worse 0, so checkpoint R's rows are held), gate messages 8 -> 8 failing with the same names, direction copies
(332 + 32, static and routed) no worse, default build byte-identical (25 corpus SVGs vs `pre-b12a2c-squeeze`: 0
differ). DEFECT_BASELINE's static column is still over on the same 5 fields as at the start (`lat_wiggles` 33 > 15,
`lat_backtrack` 10599.64 > 1870, `band_pairs` 204 > 132, `band_len` 91602 > 74730, `end_kink` 1 > 0): this package
moved none of them. Item 1 (the squeeze proper) hit the D6 stop on every form tried, items 2 and 3 were measured
and not committed; details and evidence below. Work files: `B/b12/a2c/` (`B` = `scratchpad`): check runs `c-*`
(`c-c1` = `7208f50`, `c-c2` = `22dac44`), the experiment diffs `all-experiments.diff`, `squeeze-variants.diff`,
`labels-endblocks.diff`, `scene-all.diff`, pictures `pic-p0/`, `pic-cs1/`, `l3/`, crowd details `crowd/`.

**For the lead (D5): `7208f50`'s message names the local family.** It says "the v2 input's lbl_endpx_mean 381 ->
400", "841 px from its nearer end instead of 465" and "the seven lbl_in_end lines of the local family". The code
comment that carried the 818 / 465 numbers was cleaned in `22dac44` (no amend, as the push loop requires); the
message needs your squash. `22dac44`'s message and every code comment of the package cite only the public fixture
and repo fixtures (checked with a grep of the diff for the family's names).

### Cells against the package start (`c-c2.cmp`)

- **gone 18**: `lbl_in_end` 1 -> 0 on the analog, v2 and perm-a..e (7, `7208f50`); `sib_stagger` 1 -> 0 on the
  analog, perm-a..e and the public fixture with its four reorderings (11, `22dac44`).
- **better inside failing 2**: analog `area` 4.51 M -> 4.41 M; perm-b `lbl_crowd` 8 -> 5.
- **worse inside failing 2, declared trades** (rows failing at the start, decision 5):
  - v2 `lbl_endpx_mean` 381 -> 401 (must <= 329; 400 after `7208f50`, +1 from `22dac44`'s levelling). Its one
    label that sat inside an end block (465 px from its nearer end) now sits at the only stretch of its line outside
    both end frames, 841 px from that end; there is no nearer clean spot outside. The cell it pays for is v2's
    `lbl_in_end` 1 -> 0.
  - perm-e `lbl_crowd` 5 -> 6 (must <= 0; `22dac44`). Levelling moves the block holding `pull image` 23.5 px
    upstream; the chip's nearest ink goes 13.4 -> 11.9 px, just under the 12 px crowd floor (`crowd/c1.detail` vs
    `crowd/c2.detail`). Against `7208f50` alone, `22dac44` also gives back the `lbl_crowd` -1 that `7208f50` had won on
    the analog and perm-a / c / d (3 -> 4, equal to the package start): the same chip, which `7208f50` had moved to
    12.x px, drops under the floor again once its block moves. Not fixed in this package: it needs the label stage to
    keep the crowd floor as a placement rule (no rule there reads it today), which is G3 chip-room work (item 1).
- informational: `target-worse` analog `lbl_off_mean` 0.144 -> 0.153 (target <= 0.2, met); drift `span_max` 0.300 ->
  0.306 on the analog / perm-a..e, `edge_len` +21 on the public copies, all inside passing rows.
- gates' stdout (informational, no verdict column, all from `7208f50`): DEFECT_BASELINE `chip_crowd` 17 -> 18,
  `chip_gap_min` 3.55 -> 3.52, `lbl_end_mean` 222.18 -> 226.99, `xhops` 200 -> 199 (static; routed alike);
  quality_gate dense-lr `chip_crowd` 5 -> 6, `lbl_end_mean` 371 -> 368, dense-td `lbl_end_mean` 739.6 -> 771.4.
  `22dac44` moves no gate cell.

### What landed

- **`7208f50` (item 4, G3).** `labels::place_long` / `Track::new`: in a block layout a long edge's label tries no
  spot whose chip meets one of its edge's end blocks (frames holding one end and not the other, `end_blocks`),
  unless no other spot is left; a label that sat in such a frame starts its search at the outside spot nearest where
  it sat (not at the line's middle). Test `labels::tests::a_labels_spots_keep_off_its_end_blocks`.
- **`22dac44` (item 4, X4 + paint).** `layout::level_sibling_frames`: on a layer that is the first row of two or more
  top-level blocks, each block whose frame's leading edge (first-row members less `FRAME_PAD` and, in TD, title strip
  and jog room) starts later than the earliest moves upstream whole (members at any depth and the dummies of the chains
  it holds). Needs no room; the frame keeps its size; frames stay functions of their members, so `route()` at the
  scene's centres draws the same frames. Forms tried and dropped (`c-x4*`): moving first-row members only stretched
  the frame (`compact_*` worse, 12 cells worse); levelling nested siblings too lengthened dense-td TD's parallel runs
  (`band_len` +242); LR / RL too took one more crossing on dense-lr and moved two lines of notes/spread-bottom 4 px
  nearer (direction copies worse). Levelling exposed a paint artifact on the public fixture's `REG --> ORD`: a level
  stroke at y .25 with its spline 1e-7 px off at a hop wrote the feet 0.1 px off the line before them ("519.2" vs
  "519.3"), and `bends_max` counted 5 turns instead of 4. `scene::hopped` now (block layouts only, `on_line` =
  `Scene::block_layout`) sets a foot within `FOOT_SNAP` = 0.05 px of the hop's line on it; picking draws the same
  strokes (`HopPick`). A snap without the 0.05 bound broke `marquee_selects_an_edge_once_its_painted_stroke_touches`
  (`c-snap`). Tests `blockcheck::sibling_frames_of_a_band_start_level_and_keep_their_size` (blocks-overview TD and BT)
  and `scene::tests::a_block_layouts_hop_feet_are_written_on_their_line`.

### Item 1, the squeeze redesigned: D6 stop on every form (not committed)

Built as the spec's item 3 asks (no X1; border-link weight; X2.2 keep rule; G3 chip rooms and the X2.5 column as
constraints in `block_compact`), TD / BT first. Every form broke the A1 / R floor or the direction copies, so LR / RL
was not started (its order comes after TD / BT, and bench-75 LR / RL `fan_x` <= 4 with the 14.7 gap floor was never
reached by the TD / BT forms either). Numbers are against the package start; switches `FMX_*` in
`all-experiments.diff`:

| form (`c-*`) | must-fail | worse cells | gates failing | dirs | note |
|---|---|---|---|---|---|
| `p0` prototype, all constraints | 86 -> 69 | 35 | 8 -> 13 (6 newly) | worse: defects 41 -> 44, `x_drawn` +12 copies, `min_edge_gap` down on 13 | newly failing: `a_frames_pin_keeps_off_the_columns_of_the_entries_beside_it`, `a_frames_pins_and_its_title_rows_entries_take_columns_apart`, `a_hub_fan_leaves_in_target_order`, `dragging_every_node_of_a_block_diagram_keeps_the_contract`, casing `..._keeps_no_fragment_between_two_gaps`, `note_links_match_between_scene_and_route` |
| `csg` / `cs1` gated by class + X4 | 86 -> 71 | 37 | 8 -> 17 (9 newly) | worse | |
| `cs1nx` same, no X4 | 86 -> 83 | 34 | - | worse | |
| `rlx` relaxed spines | 86 -> 83 | 29 | 4 newly (`c-rlxg`) | worse | |
| `r15` / `r15r` breadth ratio 1.5 | 86 -> 79 / 74 | 27 / 25 | - | worse (`r15r`: `fan_x` 30 -> 40, `x_drawn` +12 copies) | |
| `rk` root keep | 86 -> 91 | 35 | - | worse | |
| `rf` root runs aligned | 86 -> 84 | 29 | - | worse | |
| `noroot` | 86 -> 78 | 40 | - | worse | |
| `fan` fan-normalised weight | 86 -> 76 | 46 | - | worse | |
| `c3` class 3 only | 86 -> 76 | 42 | - | worse | |
| `c12` CSQ = 12 | 86 -> 89 | 8 | - | worse | |
| `nosq` | 86 -> 74 | 19 | - | worse (`x_drawn` +3, `fan_x` +1) | |

Causes, from the worse cells of `p0` (`bends_max` 11, `lbl_in_end` 6, `lateral_x` 6, `lbl_crowd` 5, `detour_max` 5) and
the pictures (`pic-p0/blocks-overview.three.png`, `pic-p0/infra-overview.three.png`, `pic-cs1/`): (a) compactness
is bought with straightness: a squeezed block moves its members off the columns their in- and out-edges run on, so
each such edge takes a jog (`bends_max`, `detour_max`), and on the hub fans the jogs cross (`lateral_x`, dense-lr /
bench-75 copies); (b) the G3 chip rooms never held as constraints: a chip room reserved between two blocks is
closed again by the squeeze of the block beside it (`lbl_in_end`, `lbl_crowd`); (c) squeezed title rows put pins and
title-row entries back into shared columns (the two `a_frames_pin...` tests) and change a hub fan's end order (MAIL's
fan with the digest far end, `a_hub_fan_leaves_in_target_order`); (d) the drag contract test sees a column jump on
the first drag. What would pay it: a squeeze that keeps each edge's column as a hard constraint (no jog) and only
closes gaps no edge crosses, with chip rooms reserved by the label stage first (G3) and read by `block_compact`.
That is a larger change than the package allowed after the D6 stop.

### Item 2, chains straight across levels (not committed)

`FMX_STR` straightened dummy chains across levels in a block layout (`c-str`): DEFECT_BASELINE static
`lat_wiggles` 33 -> 32, `lat_backtrack` 10599.64 -> 10574.14, `band_len` 91602 -> 90832 (still far over the
ceilings 15 / 1870 / 74730); local A2 86 -> 86, 3 cells worse; direction copies worse: bench-75 BT `x_drawn`
200 -> 204 and `fan_x` 6 -> 8, dense-td RL `x_drawn` 29 -> 30, bench-75 TD `side_ends` 15 -> 16. Without fan chains
(`c-strnf`) nothing moves at all; without labelled chains (`c-strnl`) the same bench-75 copies are worse. Cause: the
chains that wiggle are the ones a fan or a label holds off their straight column; straightening them moves the
crossing onto the fan. The remaining `lat_wiggles` sit on chains whose straight column is blocked by another block's
frame, which only the squeeze (item 1) moves.

### Item 3, b12a2b-base leftovers (not committed)

Pins counted in X2.2's breadth (`FMX_PINS`, `bin-pins`): on b9-TD and n070-TD the pin moves but its entry still
lands on the pin's column (`l3/b9-TD.svg` vs `l3/b9-TD.pins.svg`, `l3/n070-TD.svg` vs `l3/n070-TD.pins.svg`,
`l3/pins.png`); r170-TD, nested-n-TD and twin-n-TD unchanged (`l3/r170-TD.svg`, `l3/nested-n-TD.svg`). The earlier
trials (b12a2b-squeeze FR2) traded these overlaps for crossings. Cause: the entry's column comes from the port
stage after X2.2, so a breadth rule cannot keep the two apart; the fix is in the port dealing (ports for pins), not in
the breadth.

### Per fixture at the head (`c-c2/local/judge-A2.txt`, 68 lines)

| fixture | lines | open rows (value) | cause |
|---|---|---|---|
| analog | 11 | `lateral_x` 1, `compact_max` 9.71, `compact_mean` 5.4, `bends_max` 6, `detour_max` 1.72, `lbl_struck` 1, `lbl_crowd` 4, `lbl_gap_min` 0, `lbl_own_gap_max` 16.5, `straight_max` 1919, `area` 4.41 M | squeeze (compact / area / straight / detour), G3 chip rooms (labels), gap-step merge (bends), X3 (lateral) |
| v2 | 7 | `compact_max` 15.77, `compact_mean` 7.49, `bends_max` 6, `lbl_struck` 3, `lbl_crowd` 4, `lbl_endpx_mean` 401, `area` 5.09 M | as the analog; `lbl_endpx_mean` is the declared trade |
| perm-a / c / d | 5 each | `lateral_x` 1, `compact_max` 9.71, `lbl_struck` 1, `lbl_crowd` 4, `bends_max` 6 | as the analog |
| perm-b | 4 | `lateral_x` 2, `compact_max` 9.59, `lbl_crowd` 5, `bends_max` 6 | as the analog |
| perm-e | 4 | `lateral_x` 1, `compact_max` 10.12, `lbl_crowd` 6, `bends_max` 6 | as the analog; `lbl_crowd` +1 is the declared trade |
| public fixture and its 4 reorderings | 4 each | `lateral_x` 2, `compact_max` 19.97, `compact_mean` 7.26, `lbl_crowd` 2 | squeeze, G3, X3 |
| public BT | 1 | `lateral_x` 3 | X3 / e4 |
| cycle2 | 1 | `lateral_x` 2 | X3 |
| dense-lr | 3 | `lateral_x` 5, `sib_stagger` 2, `compact_max` 43.03 | e4 / e5; LR / RL X4 and squeeze (X4 in LR took a crossing, above) |
| dense-td | 1 | `lateral_x` 6 | hub fans (`n16`), R3 ports for corridors |
| bench-75 | 1 | `compact_max` 71.64 | LR / RL squeeze (bench-75 `fan_x` / gap floor) |

### Perf, guard, disk

- Guard green on both commits (fmt, clippy, tests, doctests, MSRV 1.75, wasm, rustdoc `-D warnings`).
- `perf.sh b12a2c-squeeze` (default build; rustc idle, load 0.34 / 0.82) twice against `v-b12a3-route-r1b` (the
  package start): defects 52 / 51 unchanged; static `scene_ms` total 37.78 -> 39.32 then 38.59, routed `route_ms`
  10.21 -> 10.32 then 10.25, `drag_ms` 15.18 -> 18.83 then 16.11. The first run's drag numbers were noise (the second
  is within the spread of the two start runs, `r1a` vs `r1b`: 37.78 / 39.18 static); the default build runs the same
  code apart from two empty per-edge vectors and one branch per hop.
- Disk: 8.1 GB free on `/` at the end; the experiment bins (`a2c/bin-*` other than `bin-c1`, `bin-c2`) and the scratch
  targets `a2c/target-on`, `a2c/target-probe` deleted after this report.

