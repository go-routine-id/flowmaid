## b12a2b-squeeze (2026-11-18, on top of `8876984`; read this first)

**Result: no commit; checkpoint A2 not met; every item of decision 8 stops under D6 with the numbers below.** The
branch head stays `8876984` (`b12a2b-base`'s last commit). Nothing built in this package passed decision 2's three
checks against the package start (`b12/tools-a2b/cand-8876984`), and each failure is a passing row, an A1-held test, a
gate message or a direction copy getting worse, which decision 2 forbids (decision 5's intermediate worsening covers
only already-failing area / compact rows paid back by the package's last commit, and no later item of this package
landed to pay anything back). Work files: `B/b12/sq/` (`B` = `scratchpad`): the diffs named below, the check runs
`c-<label>/` with `c-<label>.cmp` (against `cand-8876984`), the dirs-only runs `f-<label>/`, the regression sweeps
`sw-<label>/`, pictures in `v-*/` and `reg/`.

**Checkpoint A2 at the head (`8876984`, unchanged by this package), corrected judge:** `--due A2` **86** must-fail
lines of 444 (`7122a53`: 89), `--due A1` **0**, routed 37 (judged at R). By metric: `lateral_x` 15, `compact_max` 14,
`sib_stagger` 12, `lbl_crowd` 12, `lbl_in_end` 7, `compact_mean` 7, `bends_max` 7, `lbl_struck` 5, `area` 2,
`straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `lbl_endpx_mean`, `detour_max` 1 each. Per fixture: the analog 13
lines, v2 8, the five permutations 32, the public fixture and its four permutations 25, public BT `lateral_x` 3, cycle2
`lateral_x` 2, dense-lr `lateral_x` 5 / `sib_stagger` 2 / `compact_max` 43.03, dense-td `lateral_x` 6, bench-75
`compact_max` 71.64 (full list: `tools-a2b/cand-8876984/local/judge-A2.txt`). **DEFECT_BASELINE's static column is
not within its ceilings at the head**: `lat_wiggles` 33 > 15, `lat_backtrack` 10599.64 > 1870, `band_pairs` 204 > 132,
`band_len` 91602 > 74730, `end_kink` 1 > 0, `end_tilt_max` 0.80 > 0.3, `idiom_spline` 1 > 0 (the gate is one of the 14
failing suite-on tests, as at `7122a53`). Causes per fixture: unchanged from fix round 4 of step 2 ("Causes per fixture
of every must-fail line", below) less what `b12a2b-base` fixed; the squeeze rows (`sib_stagger`, `compact_*`, `area`)
wait for item 8(2), the label rows for G3 (base item 5, skipped), `bends_max` for the gap-step merge (base item 6,
blocked on G3's label slot), `lateral_x` for X3 / e4 / e5 (base items 4b and 7, skipped with their numbers).

Default build: byte-identical (no code change; `render.sh pre-b12a2b-squeeze` taken at the start, nothing to diff).
Guard and perf: not run for a commit (none made). No rustc of the other package ran during the measurements below
(`pgrep -c rustc` 0 before each). Disk 6.0 GB free at the end; the scratch target `B/b12/sq/target-on` is deleted.

### Item 8(1): R3 corridors in `route()` (built, measured, not committable)

Built (`sq/r3-corridors-attempt.diff`, 475 lines, `scene.rs` only, block mode TD/BT only, behind the switch): for each
of `g.edges` whose ends lie in two different units of the top level, `scene::corridors` reads the units' current boxes;
when the two end units stand one after the other with room for a step out and in, the units between them along the flow
are grouped into bands; a path that steps once and misses every band keeps today's channel (`cross_cluster_route`),
otherwise the path takes one breadth per band (the one it comes with where that lies `LANE_PAD` clear in a free gap,
else the nearest free gap's near edge, the lower on a tie: R5), turns at the middle of the flow gaps (a step out of /
before the end unit where a gap is too short and the turn misses every unit), and the legs that shift into one gap from
one side take slots a `TRACK_PITCH` apart in the order of the breadth each path takes next (R3's slot rule). E3 edges
(upstream, between two units) take the same corridor mirrored instead of a lane. Label on the longest leg (R8). Bare
on-axes corners, so `ortho_pass` keeps and rounds them; `route_partial` builds moved edges the same way.

Direction copies, routed (static unchanged in every variant; against `cand-8876984`; env knobs in the experiment build):

| variant | defects | x_drawn | lateral_x | fan_x | edge_chip | side_ends | copies worse |
|---|---|---|---|---|---|---|---|
| head | 169 | 1997 | 418 | 183 | 78 | 361 | - |
| E1 + E3, one-step = today's channel | 149 | 2076 | 394 | 299 | 71 | 372 | 84 |
| E1 + E3, one-step drawn as an explicit Z | 138 | 2108 | 388 | 291 | 71 | 397 | 67 |
| E1 only (E3 stays a lane) | 155 | 2040 | 414 | 203 | 76 | 356 | 18 |
| E1 only, explicit Z | 131 | 2062 | 412 | 210 | 70 | 377 | 44 |
| E3 only | 166 | 2020 | 402 | 241 | 70 | 381 | 80 |
| E3 only, explicit Z | 164 | 2006 | 395 | 221 | 79 | 378 | 46 |

- Every variant takes defects down (up to 169 -> 131) and the lateral exits down, and every one takes the dense
  fixtures' fans up: the corridors of one fan converge on one target column and turn on one line in the last gap, and
  nothing in `route()` nests them (bench-75 BT: `ext`'s bottom gains 5 crossings among `dash`, `job1`, `job3`, `svc5`,
  `svc6 -> ext`; dense-td BT routed `fan_x` 15 -> 26..99). The best (E1 only) leaves 18 copies worse, all dense-td,
  dense-lr, bench-75 and hub5 TD (`edge_chip` 0 -> 1).
- **E1 corridors alone do not help the squeeze**: the crossing the squeeze adds on the public fixture's routed hub fan is
  between two lanes (E3: `ALERT -> MAIL` laid outside `ALERT -> PAGER` and coming back in to the mail relay's right side
  across the pager's column, `fan_x ALERT Top` at (769, 422)).
- E3 as a corridor fixes that crossing on the squeezed layout (`sq/v-x/blocks-overview.pair.png`: `digest` up the gaps
  into the mail relay from below, `fan_x` 0), but **on the head's layout** E3 as today's channel crosses it twice
  (routed `fan_x` 0 -> 2 on all 16 public-fixture TD/BT copies; the test `a_hub_fan_leaves_in_target_order` fails at the
  head): the one-step test reads ALERT's centre (19 px clear of Order processing's wall), the ports stage puts the end
  14 px left of it, and the router reroutes the leg round the far side. Drawn as an explicit Z the public copies keep
  `fan_x` 0 but take `side_ends` 3 -> 5 and `min_edge_gap` 31.94 -> 27.02 (16 copies).
- What a committable R3 needs (not built): the inner legs of R3 (the first leg planned against the port the ports stage
  will give, i.e. the whole side's spread, not the node centre), the slot order over every edge of a gap including the
  one-step channels, and the fan-in nesting at the target (the outer path turns nearest the target). That is B12.13's
  corridor builder, not a pull-forward of a few hundred lines; the lead's call.

### Item 8(2): the squeeze (measured on the head, not committable)

`sq/squeeze-proto-on-8876984.diff` = the study's `proto.diff` (no X1, cross-border weight 2, X2.2 keep rule, X4,
`BLOCK_SLACK` 4) on `8876984`, without its `ortho` hunk (superseded by base item 2). Run `c-sq0`:

- local static 86 -> **69** (30 lines gone: `sib_stagger` 12, `compact_mean` 6, `compact_max` 5, `lbl_struck` 5,
  `crossings` 5, `area` 2, `lateral_x` 1, `detour_max` 1), but **13 new** (public family `bends_max` 4 -> 6 and
  `detour_max` 1.22 -> 1.48, x5 each; analog `lbl_off_max` 0.357; v2 `lateral_x` 2, `lbl_own_gap_max` 9.5) and **22 worse
  inside failing** (analog `bends_max` 6 -> 8, v2 6 -> 9, perms 6 -> 8; `lbl_crowd` 4 -> 7 on the analog and three
  perms; `lbl_in_end` 1 -> 2 / 3; `lateral_x` 1 -> 2 on the analog and four perms); routed 37 -> 37 with 20 cells worse.
- gates 14 -> 16 failing: newly failing `a_frames_pin_keeps_off_the_columns_of_the_entries_beside_it` (twin blocks under
  `Order processing`: `q --> b1` through B's lateral wall, an overlap) and `a_frames_pins_and_its_title_rows_entries_take_columns_apart`
  (public fixture: OBS's pin and entry change sides), both `b12a2b-base` tests; `a_hub_fan_leaves_in_target_order`
  (routed, the lane crossing above); `advanced_showcase_parses_and_renders` newly passes. Worse messages:
  `defect_baseline` static `lat_wiggles` 33 -> 42, `lat_backtrack` 10599.64 -> 13680.11, `band_pairs` 204 -> 207,
  `band_len` 91602 -> 98160, routed `lat_wiggles` 23 -> 24, `lat_backtrack` 3856.27 -> 4753.93; `quality_gate` dense-td
  routed `edge_node_pairs` / `edge_node_x` 1 -> 2; drag: dense-lr 0 -> 1 jump (`n22`: `n12 -> n44` 21.5 px).
- copies: static defects 41 -> 44 (+3 / -2), `x_drawn` 1283 -> 1335 (+12 / -10), `lateral_x` 201 -> 183 (+7 / -22),
  `fan_x` 30 -> 25, `side_ends` 216 -> 194, `min_edge_gap` down on 13; routed defects 169 -> 204 (+10 / -4), `fan_x`
  183 -> 217 (+23 / -7), `lateral_x` 418 -> 500, `min_edge_gap` down on 25.
- With R3 as well (`c-sqr3`, squeeze + the first R3 variant): the public fixture's routed fan is clear, but the same test
  now fails on the **static** mail relay order (`ends at 671.37 and 690.59 lead to 646.55 and 502.53`: the squeezed
  columns lay `ALERT -> MAIL`'s end between the two returns from Order processing), with the two base tests; local
  static 86 -> 69 (35 worse), routed 37 -> 43, copies routed `x_drawn` +27 / -13, `lateral_x` 418 -> 453.

So beyond R3 the squeeze needs: the node-side end order of a fan-in on squeezed columns (static), the e2 dealing of
pins and title-row entries re-run on squeezed breadths (the two base tests), the chains kept straight across levels
(item 8(3): `lat_wiggles` / `lat_backtrack` / `band_len`), and G3's chip rooms and X2.5's column as constraints in
`block_compact` (G3 p5 and the X2.5 bound never landed: base items 5 and 6 were skipped, so there is nothing to hold yet).

**X4 alone** (`sq/x4-only.diff`, the prototype's `level_sibling_frames` without `block_compact`; `c-x4`): local static
86 -> **74** (the 12 `sib_stagger` lines gone, none new), 19 worse inside failing; gates 14 -> 19 (dense-td's
`n16 -> n35 x n16->n32` at (2110, 502) in both hub-fan tests and in the two comb idempotence tests;
`edges_keep_off_frame_borders` routed `border_len` 2310); copies: dense-td TD static `fan_x` 0 -> 1, routed dense-td /
dense-lr worse. **X4 on the top level only** (`sq/x4-knobs.diff`, `FMX_X4ROOT`; `c-x4root`): 86 -> **75** (11 gone,
none new), the 14 failing tests unchanged, but 19 static cells worse inside failing (analog family `compact_max` 9.71 ->
10.37, `compact_mean` 5.4 -> 5.53, `lbl_crowd` 4 -> 5, perm-b `lbl_in_end` 1 -> 3, v2 `lbl_endpx_mean` 381 -> 382, public
`compact_mean` 7.26 -> 7.29 x5), 8 routed cells worse, one gate message worse (`defect_baseline` static `band_pairs`
204 -> 205), 12 routed copies worse (public TD `side_ends` 3 -> 4 x8, dense-lr TD defects 3 -> 4, `x_drawn` 24 -> 27).
Cause of the compact rows: X4 moves only the first-row members of the block whose frame starts later, so that block's
frame grows along the flow by the shift. X4 needs the squeeze (or a frame-start rule that moves the whole block) to pay
for it; not committed.

### Item 8(3) and 8(4)

(3) not reached: it straightens the chains the squeeze makes step. (4): the rows open at the head are listed above;
none moved.

### The lead's non-blocking regressions (lead note at the start of this package)

- **Pin arrowhead on a member entry** (`fr5/cin2/n070-TD`, r170): `ortho::TitleSides` places a displaced title-side
  entry a track off the side's crossings but not off the frame's pins (they end on the border, so no crossing finds
  them). Tried (`sq/pin-apart-attempt.diff`): pins counted as taken columns. All pins: b9-TD / b11-TD defects 1 -> 0,
  `x_drawn` 4 -> 0, `min_edge_gap` 3.39 -> 19.88 / 18.94 (their `lateral_x` 1 -> 2); over the verifier's sets (static)
  rnd defects 28 -> 27 but `x_drawn` 251 -> 270 (15 files worse), nr defects 24 -> 21 (`x_drawn` 176 -> 182), nt 13 -> 12,
  sw72 2 -> 0, tw `x_drawn` 4 -> 7; the 364 copies unchanged (no repository fixture has the case). Only pins of the
  entry's own source: rnd unchanged, nt defects 13 -> 12, but tw t01 / t03 / t07 `x_drawn` 0 -> 1, and **n070 itself
  unchanged**: when `ortho` runs, `q --> b0` crosses B's title side at 534.9 and `q --> B`'s pin stands at 520.9, 14 px
  apart; a later stage lays them on one column. Not committed (crossings traded for overlaps); the remaining stage is
  the respacing / nudge after the repair, as for r170 in base fix round 4 #2.
- **b9-TD / b11-TD** (defects 0 -> 1 since `3e38be3`): fixed by the all-pins variant above, with its costs; not taken.
- **nested-n-TD, twin-n-TD**: unchanged, causes as in base fix round 4 #3 / #4.
- **n109-BT, detached gap fragment**: the casing cuts two gaps into one under-line about 40 px apart round its corner, at
  its two crossings with one over-edge that turns there too; `CROSS_CLUSTER` (25 px) does not join them and a chain of
  two costs nothing. A fix is a fragment rule in `casing::choose_under` (split the two crossings' roles); the casing is
  shared with the default build, so it changes default output (byte identity) and the dash gate; not built.

### What the lead may want to decide

1. Whether R3 (B12.13's corridor builder in full: inner legs, slot order over every edge of a gap, fan-in nesting) is
   built as its own package before the squeeze; the measurements say a partial R3 trades dense-diagram fans for defects.
2. Whether X4 may land with the compact rows it costs (11 `sib_stagger` lines for 19 worse compact / label cells and 12
   routed copies), paid by a later squeeze package (decision 5 as written does not cover it: the payer is not in this
   package).
3. Whether the pin rule may trade crossings for overlaps on the verifier's random inputs.

---

