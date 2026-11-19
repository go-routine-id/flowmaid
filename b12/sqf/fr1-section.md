## b12a2b-squeeze (2026-11-18, on top of `8876984`; read this first)

**Fix round 1 (2026-11-18, ~06:15-08:40 UTC; read this first).** The verifier of the first run found that nothing
landed (#1), that R3 was built only in part (#2: top-level units only, no inner legs, slots only for legs that shift
into one gap from one side), that item 8(3) was not attempted although the head is over DEFECT_BASELINE's static
ceilings without any squeeze (#3), that the LR/RL squeeze was never measured (#4), that none of the lead note's
regressions landed (#5: n109-BT could be gated to block mode with a crate-private flag) and that the report and the
JSON disagreed on perf (#6). Two commits, both green with the guard and with decision 2's three checks:
**`d6853bc`** (8(3): a run into a glyph end lies on its axis, DEFECT_BASELINE static `idiom_spline` and `end_tilt_max`
back under their ceilings) and **`c9eb06b`** (lead note n109-BT: a block layout's under-line is cut once across a
cluster of crossings; `Scene::block_layout`). Work files: `B/b12/sqf/` (`B` = `scratchpad`): check runs `c-*`, the
dirs-only runs `f-<variant>/`, the experiment diffs named below, pictures in `v-head/`, `v-r3/`, `n109/`, `cas/`.

**Checkpoint A2 at the head (`c9eb06b`), corrected judge: still not met.** `--due A2` **86** must-fail lines of 444
(`7122a53` 89, package start 86), `--due A1` **0**, routed 37 (judged at R). By metric: `lateral_x` 15, `compact_max`
14, `sib_stagger` 12, `lbl_crowd` 12, `lbl_in_end` 7, `compact_mean` 7, `bends_max` 7, `lbl_struck` 5, `area` 2,
`straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `lbl_endpx_mean`, `detour_max` 1 each. By fixture: the analog 13
lines, v2 8, perm-a/c/d 7 each, perm-e 6, perm-b 6; the public fixture and its four permutations 5 each
(`sib_stagger` 1, `lateral_x` 2, `compact_max` 19.97 > 7.97, `compact_mean` 7.26 > 4.63, `lbl_crowd` 2), public BT
`lateral_x` 3, cycle2 `lateral_x` 2, dense-lr `lateral_x` 5 / `sib_stagger` 2 / `compact_max` 43.03 > 11.78,
dense-td `lateral_x` 6 > 1, bench-75 `compact_max` 71.64 > 52.03 (`tools-a2b/cand-c9eb06b/local/judge-A2.txt`).
Causes per row, still open: `sib_stagger` / `compact_*` / `area` wait for the squeeze (8(2), below); the label rows
for G3 (`b12a2b-base` item 5, never landed); `bends_max` for the gap-step merge (base item 6, never landed);
`lateral_x` for X3 / e4 / e5 (base items 4b and 7) and, on the dense fixtures, for the hub fans the squeeze and R3
have not cleared (below). **DEFECT_BASELINE static at the head: 5 fields over their ceilings (7 at the package
start)**: `lat_wiggles` 33 > 15, `lat_backtrack` 10599.64 > 1870, `band_pairs` 204 > 132, `band_len` 91602 > 74730,
`end_kink` 1 > 0 (gone: `end_tilt_max` 0.80 > 0.3, `idiom_spline` 1 > 0). The suite-on failing tests are the same 14
names as at `7122a53`; every A1-held test is green.

**Decision 2's checks.** Against `7122a53` (`tools-a2b/cand-c9eb06b.cmp`): cells no worse (89 -> 86: 3 gone, 7 better
inside failing; routed 37 -> 37), gate messages no worse (1224 tests, failing 14 -> 14), direction copies: the one
worse cell is `blocks.swimlane-nested-TD` static `min_edge_gap` 40 -> 25, the trade the lead accepted for base item 3
on 2026-11-18 ~00:20 UTC; nothing else moves against `7122a53` beyond base's static `lateral_x` 216 -> 201 and
`side_ends` 231 -> 216. Against the package start (`cand-c9eb06b/compare-vs-cand-8876984.txt`): `VERDICT: no worse`;
`defect_baseline_gate` loses `static end_tilt_max 0.80 > ceiling 0.3` and `static idiom_spline 1.00 > ceiling 0`;
dense-lr LR/RL copies `idiom_spline` 1 -> 0, `end_tilt_max` 0.80 -> 0. **Default build byte-identical**: identity
tests green in the guard, and `render.sh post-b12a2b-squeeze-fr1` gives 25 / 25 corpus SVGs byte-equal to
`pre-b12a2b-squeeze`. Guard `GUARD: ALL GREEN` before each commit (`sqf/guard-2.txt`, `sqf/guard-3.txt`).

**Perf (PLAN invariant 1; default build, the changes sit behind block-mode flags).** Four runs: `b12a2b-squeeze-fr1-a`
(after `d6853bc`'s change), `-b` (after `c9eb06b`'s), `-c` and `-c2` (head). Worst drags within budget in every run
(bench-75 18.73-20.22 against 25.28; advanced 1.67-2.36 against 2.61; lr 1.37-1.40 against 1.95), `route_ms` within
budget (bench-75 10.07-15.49 against 15.76: the 15.49 of run `-b` is the box's noise, 10.07-10.38 in the head's two
runs), `route_partial` well under 6 ms. Static `scene_ms`: each head run flags one sub-0.2 ms file (demo 0.17 against
2 x 0.07 in `-c`, back-edge 0.17 against 2 x 0.06 in `-c2`): the two runs disagree and the code those files run is
unchanged (the default build skips both changes), so no regression is called. No rustc of the other package ran during
any timing (`pgrep -c rustc` 0 before each). *(Correction of the first run's section below: it ran no perf because it
committed nothing; its JSON's `perf_ok` was not a measurement.)*

### Item 8(1): R3 corridors in `route()` (built in three forms, 21 variants measured; not committable)

Every variant below is measured on the 332 direction copies, routed (`f-<variant>/`, against `c-b`, the drawing of
`d6853bc`; static is unchanged in all of them). Diffs: `sqf/r3-v2-variants.diff` (builder-side, env knobs),
`sqf/r3-postports.diff` (the post-ports stage, `src/corridor.rs`). Table: totals (copies worse / better) and the
number of copies worse in any verdict column.

| variant | defects | x_drawn | lateral_x | fan_x | edge_chip | side_ends | gap down | copies worse |
|---|---|---|---|---|---|---|---|---|
| head | 169 | 1997 | 418 | 183 | 78 | 361 | - | - |
| builder, E1+E3 (first run's) | 149 (+1/-4) | 2076 (+20/-4) | 394 (+5/-14) | 299 (+24/-0) | 71 | 372 | 10 | 84 |
| builder, E1 only | 155 (+1/-4) | 2040 (+5/-1) | 414 | 203 (+4/-1) | 76 | 356 | 2 | 18 |
| builder, E1, columns only (no corners) | 164 | 2035 | 414 | 210 | 80 | 359 | 0 | 15 |
| builder, E1, monotone corridors only | 169 | 1997 | 418 | 183 | 78 | 361 | 0 | **0 (no-op)** |
| builder, E1, look-ahead gap choice | 148 (+1/-5) | 2087 | 408 | 234 | 74 | 356 | 1 | 14 |
| builder, E1, corridors only for ends with no other corridor | 166 (+0/-2) | 2006 (+2/-0) | 415 | 183 | 80 (+2) | 360 | 0 | 4 |
| post-ports, detours only | 141 (+1/-5) | 2008 (+3/-3) | 412 | 179 (+3/-2) | 75 | 358 | 1 | 15 |
| post-ports, detours + one-step corridors | 132 (+0/-6) | 2060 (+10/-3) | 417 | 226 | 68 | 349 | 9 | 43 |
| **post-ports, every E1 edge between two units, one track order per gap** | **127 (+0/-14)** | 1991 (+4/-18) | **390 (+0/-16)** | 186 (+2/-2) | 75 (+3/-5) | **337 (+1/-15)** | 17 | 27 |
| the same, tracks spread over the gap | 131 (+0/-14) | 2009 | 390 | 191 | 75 | 340 | 18 | 28 |
| the same, planned before and after the ports stage | 132 (+0/-13) | 2044 | 388 | 198 | 76 | 341 | 16 | 27 |

What the measurements show:
- **A corridor the builder hands over changes nothing where it runs one way across the flow** (the monotone rows: byte
  for byte the head's drawing on all 332 copies). The ports stage moves the ends; the path is then off the axes and
  `ortho` re-plans it from the ports, with the corridor's points only as columns. Every effect of the builder-side
  corridor (all of the first run's numbers) came from its detours round blocks, and the band-by-band gap choice (R5,
  nearest gap) makes long detours where flow-overlapping blocks merge into one band (dense-lr drawn BT: `n12 -> n46`
  left 2050 px round every frame, then right 3400 px). Look-ahead (nearest to the previous leg plus the far end) helps
  the small copies but not the dense ones.
- **R3 therefore has to be planned after the ports stage and delivered on the axes**, so `ortho` keeps it (it only
  rounds a path on the axes): built as `corridor::route_corridors` (post-ports, TD/BT, block layout's `route()`), with
  the unit rows' global gaps as the places every corridor turns, the slots of a band's gap dealt by the breadth each
  path takes next, and **one track order per gap over every corridor step in it, one-step channels included**
  (a step lies below another whose incoming column it spans and above one whose outgoing column it spans: the
  nesting of fan-outs and fan-ins). With every E1 edge between two units in it, this is the best R3 measured:
  defects 169 -> 127 (14 copies better, none worse), `lateral_x` 418 -> 390 (16 better, none worse), `side_ends`
  361 -> 337, `x_drawn` better on 18 copies. Public fixture BT, routed: `sessions` and both `write` edges straight
  into their targets' flow sides (picture `v-r3/bo-BT-p2all.png`, head | R3).
- **Why it is not committed:** 27 copies get worse. (a) `min_edge_gap` drops on the 16 public copies (BT 31.94 ->
  27.02: `cart --> queue`'s corridor column 27 px from `web --> metrics`; TD 13.5 -> 12.5: two steps a track apart);
  (b) the hub fans of bench-75 drawn TD / BT (`fan_x` 7 -> 17, 9 -> 27: `svc4`'s fan into the data zone, ports dealt
  for the old headings) and dense-td's `x_drawn` 107 -> 113 / 93 -> 109; (c) hub5 TD `edge_chip` 0 -> 1. The local
  routed table (62 cells) goes 37 -> 39 must-fail with 14 cells worse (label crowding and struck labels on the analog
  family's corridors, the analog's routed crossings 26 -> 28; `sqf/f-p3all-local`). Planning before the ports as well
  (so the ports stage deals the ends by the corridors' headings) does not fix the fans (bench-75 BT 9 -> 29).
- **What R3 still needs, measured to be the cause:** the ports of a side dealt for the corridors and the non-corridor
  edges together (the stage after the ports still sees the old order on a hub's side: bench-75's `svc4`), the label
  stage taught the corridor legs (R8: a label on the leg at the common-ancestor level; today's relabel crowds them),
  and a track pitch that keeps the head's gaps where the gap allows (the public copies' 27.02 / 12.5). Inner legs
  (inside the end blocks) did not come up as a cause on any worse copy of the post-ports form: every worse copy above
  is a top-level fan or gap. **E3** (upstream edges as corridors instead of lanes) costs the public copies their
  clean lanes on the head's layout (`v-r3/bo-TD.png`: `tracking mail` and `digest` round the mail relay, `fan_x` 0 ->
  2 on all 16 public TD/BT copies) and is left out of every post-ports variant.

### Item 8(2): the squeeze (TD/BT re-measured on the head; LR/RL measured in two forms; not committable)

- **TD/BT** (the study's prototype on the head, `sq/squeeze-proto-on-8876984.diff`, `f-sqp`): local static 86 -> 69
  with 35 cells worse (13 new: the public family's `bends_max` / `detour_max`, the analog's `lbl_off_max`, v2's
  `lateral_x` 2 and `lbl_own_gap_max`; 22 worse inside failing lines); copies static defects 41 -> 44, `x_drawn`
  1283 -> 1335, routed defects 169 -> 204, `fan_x` 183 -> 217, 25 copies lose `min_edge_gap`. Unchanged from the
  first run's measurement; the two `b12a2b-base` tests it breaks and the public routed hub fan are its blockers.
  G3's chip rooms and X2.5's column as constraints in `block_compact`: base items 5 and 6 never landed, so there is
  no room of either to hold.
- **LR/RL**, never measured before: the prototype's `block_compact` turned on for LR/RL (`FMX_SQLR`,
  `sqf/squeeze-lrrl-proto.diff`). As it is, frames collide (static copies defects 41 -> 139: `node_frame` +42,
  `frame_overlap` +40) because `block_compact` is handed no title-strip room, which in LR/RL lies across the flow.
  With the strip (`header`) handed in as Brandes-Köpf gets it: frames no longer collide (static defects 41 -> 51) and
  the compaction works: dense-lr LR/RL `compact_max` **43.03 -> 7.01** with static defects, `fan_x` and `x_drawn`
  unchanged (2, 0, 17); bench-75 LR/RL `compact_max` **71.64 -> 28.36**. But bench-75 LR static `fan_x` 4 -> 5
  (RL 4 -> 7), `min_edge_gap` 15.25 -> **13.50 < the 14.7 floor**, `x_drawn` 204 -> 226, defects 0 -> 7, routed
  `fan_x` 24 -> 38: the lead's rule of 2026-11-16 (bench-75 LR/RL `fan_x` <= 4, gap floor 14.7) forbids it. Local
  static with both directions squeezed: 86 -> 68, 37 cells worse (`f-sqlr2-local`; bench-75 and dense-lr `lateral_x`
  11 each). A per-direction commit (LR/RL for dense-lr only) would be a rule by fixture, not by geometry; not built.

### Item 8(3): DEFECT_BASELINE's static column (two fields fixed and committed; the rest measured)

Per fixture at the package start (suite-on, `tests/fixtures/quality`; the ceilings are the default build's totals,
which this table matches field by field: 15 / 1862.22 / 132 / 74724):

| fixture | lat_wiggles (default) | lat_backtrack (default) | band_pairs | band_len | end_kink | end_tilt_max | idiom_spline |
|---|---|---|---|---|---|---|---|
| bench-75 (LR) | 18 (11) | 2260.70 (1659.33) | 106 (61) | 29346 (26976) | 1 | 0.07 | 0 |
| dense-lr (LR) | 3 (1) | 1249.23 (20.75) | 33 (32) | 15284 (17044) | 0 | 0.80 -> 0 | 1 -> 0 |
| dense-td (TD) | 11 (2) | 7061.71 (154.15) | 55 (32) | 42878 (28464) | 0 | 0 | 0 |
| fanout-frames | 0 | 0 | 6 (3) | 3386 (1532) | 0 | 0 | 0 |
| wide-labels | 1 (1) | 28 (28) | 4 | 708 | 0 | 0 | 0 |

- **Fixed (`d6853bc`)**: `idiom_spline` and `end_tilt_max`, both dense-lr. `n22 -.-> n44`'s run kept the chain's last
  column 0.5 px off its port (`ortho::flow_path` passes a stop within `ALIGNED` without a jog) and `n26 --> n47`'s end
  went to the drum's shoulder 0.49 px across its run (`snap_ends`); both runs now follow the port in a block layout.
- **`end_kink` (bench-75 `svc1 --> logs`, open):** a router reroute's plain 12.5 px stub into `logs`' bottom (a flank
  in LR) leaves the corner's rounding under the head (52 deg). Asking the long stub for every glyph end in a block
  layout's static scene: `end_kink` 0, but bench-75 overlaps 0 -> 3, `fan_x` 4 -> 15, `min_edge_gap` 15.25 -> 12.66
  (below its 14.7 floor). For flank ends only (`sqf/endkink-flank-stub.diff`): overlaps 0 -> 2, `fan_x` 4 -> 15,
  `end_cross` 0 -> 5. Neither taken.
- **`lat_wiggles` / `lat_backtrack`:** each static wiggle classified (`sqf/wig/classify.py`, against the frames that
  hold neither end and lie between the ends along the flow): **14 of 33 are "blocked"**: those frames together cover
  every column between the two ends, so with no run through a foreign frame (`xframe_runs` 0, A1-held) the edge must
  go round and come back (bench-75 `svc2/svc3 -> logs`, `svc6 -> ext`, `gw -> logs`, `lb -> metrics`, `user -> ext`;
  dense-lr `n12 -> n46`, `n12 -> n44`; dense-td `n16 -> n41`, `n16 -> n43`, `n16 -> n44`, `n8 -> n42`, `n46 -> g9`'s
  frame end), and they carry 9 255 of the 10 600 px of `lat_backtrack`. The other 19 have a free column and are
  chain steps the layout makes (dense-td `n2 -> n7` 40 px round its own label's slot, `n16 -> n35` 44 px). With 14
  forced, the static ceiling of 15 leaves room for one more in the whole corpus: **4.5's rule ("`lat_wiggles` at or
  below the starting release; `band_*` and `lat_backtrack` may rise for a corridor with the edges named") cannot hold
  for `lat_wiggles` while the block layout keeps foreign frames out of the edges' way.** That is a question for the
  lead (a wiggle round a block counted with the corridor class, as `lat_backtrack` already is), not a threshold this
  package may move. `band_pairs` / `band_len` rise by the corridors and fans of the same edges.

### Item 8(4) and the lead note's regressions

- 8(4): no other A2 row moved (the causes per row are above; per fixture as in `b12a2b-base`'s fix round 4).
- **n109-BT, the detached crossing-gap fragment: fixed in `c9eb06b`.** The pin's two gaps 20 px apart round its corner
  are one cluster (`CROSS_CLUSTER`); in a block layout the stretch between is cut too (`casing::join_cluster_gaps`),
  gated by the new crate-private `Scene::block_layout` (set by `scene_labelled` and `route_built`), so the default
  build paints as before. 7 of 1224 drawings change (n109-BT, n081-TD, r049/r052/r087/r262-TD, bench-75 BT), each a
  short piece of an under-line left between two gaps (pictures `cas/r052-TD.crop.png`, `cas/quality.bench-75-BT.crop.png`,
  `n109/zoom2.png`); no metric reads the masks, so the checks do not move.
- **n070-TD / r170-TD (pin arrowhead on a member's entry), b9-TD / b11-TD (defects 0 -> 1 since `3e38be3`),
  nested-n-TD, twin-n-TD: unchanged.** At the head b9-TD: `client --> G`'s pin column 637 and `hub --> queue`'s
  displaced entry column 645 share the gap's step track (overlap 18 px, gap 3.4 px); the first run's two variants of
  "pins as taken columns" (all pins / pins of the entry's own source) and base fix round 4's trials stand as measured
  there (all pins: b9/b11 fixed, the random set's `x_drawn` 251 -> 270 on 15 files; own source: n070 unchanged, three
  twins `x_drawn` 0 -> 1). Not retried this round.

### What the lead may want to decide

1. `lat_wiggles` in DEFECT_BASELINE's static column (8(3) above): 14 of the head's 33 wiggles are forced by
   `xframe_runs` 0; whether a wiggle round a block counts with the corridor class of 4.5.
2. R3: the post-ports corridor stage is the form that improves the routed copies (defects -42, `lateral_x` -28,
   `side_ends` -24 over the 332 copies) but costs 27 copies (fans of bench-75 / dense-td, the public copies' gap, hub5)
   and two routed local lines; whether the next package builds it with the ports and labels it needs (above) or
   whether the routed rows it costs are judged at R as the spec's table says.
3. The LR/RL squeeze: dense-lr is clean under it, bench-75 is not (gap floor, fan); whether bench-75 LR/RL may trade.

---

### First run of this package (2026-11-18, ~04:57-06:06 UTC), kept as it was reported

