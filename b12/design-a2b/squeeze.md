# Study "squeeze": B12.8 (X1-X4, BLOCK_SLACK) on 7122a53, test by test

Study folder: `B/design-a2b/squeeze/` (`B` = `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12`).
Worktree of 7122a53 used and removed; diffs, logs, judge outputs and pictures are kept there.

## 0. Answer in short

- The most complete squeeze diff (`B/work/a2g/b128-squeeze-on-9f51f77.diff`: X1, X2/X3 `block_compact`, X4
  `level_sibling_frames`, `BLOCK_SLACK` 4) applies to 7122a53 **without conflicts** (hunk offsets only).
  Re-applied as is: local judge `--due A2` **91 -> 70** (A1 0). The suite-on run adds **5** tests, not 4: the round-3
  four plus `a_displaced_end_never_steps_back_over_another_edges_crossing_of_its_title_side` (written in round 4,
  `fb56b2f`; lr BT x_drawn 2). `advanced_showcase_parses_and_renders` starts passing.
- **The comparison by failing-test names hid one regression**: inside the already-failing `quality_gate`, the
  squeeze adds the static row `bench-75 static: min_edge_gap = 12.50 < floor 14.7` (bench-75 is LR: X2 does not even
  run there; X1 does). That is exactly what lead note "after step 2's first stop" item 2 forbids.
- **X1 is the main culprit.** Measured per component (knobs in the worktree): X1 alone breaks bench-75's gap floor
  and fan (4 -> 6), the public fixture's routed fan, lr BT, and gives casing's 34 routed opposite patterns (all on
  bench-75); X1 together with X2 breaks dense-td's `n16` fan (both dense-td tests). X1 alone moves the local judge
  91 -> **99**. Dropping X1 never adds a must-fail line (squeeze without X1: **67**, with: 70).
- Two smaller causes sit in X2/X3 itself and are fixed statically in the prototype: X3's pull lets a cross-border
  chain segment weigh 8 (hub5's fan leaves out of target order), and X2 can pack a block under its title so that
  the X2.2 column room vanishes (lr BT).
- **Prototype** (`B/design-a2b/squeeze/proto.diff`): squeeze without X1 + cross-border chain links weigh as border
  links + X2.2 keep rule + a float fixed-point fix in `ortho::TitleSides::of` (a hang the squeeze exposed) + the two
  B12.8 tests. Result: **of the five tests, four pass**; the suite-on failing list is the 14 of 7122a53 minus
  `advanced_showcase` plus **one**: `a_hub_fan_leaves_in_target_order`, on the **public fixture routed** only
  (`fan_x` 1, `ALERT Top ALERT->PAGER x ALERT->MAIL`). Local judge **91 -> 72** (A1 0). No new `quality_gate` row.
  Default build byte-identical (identity tests green in a default-cfg build).
- **The remaining failure needs route()-side work that belongs to step 3 (B12.13, R3 corridors and their slot
  order).** The static scene is fan-free in every variant measured; `route()` at the same centres draws the
  digest edge round the far side of the pager edge whenever the observability block is not straight under the
  pager. It is not a frame problem (F3 is built and the frames agree). The smallest route()-side piece that fixes
  it is R3's per-band gap choice plus R3's slot order by far-end breadth for E1/E3 corridor edges: a real part of
  B12.13 (estimate below), not a tweak.
- Not solved and newly measured: the squeeze worsens the static `DEFECT_BASELINE` fields (lat_wiggles 33 -> 42,
  dense-td 11 -> 20: the whole `n16` fan now steps between band columns) and adds 12 B12.8 edge-shape/label must
  lines (public family `bends_max` / `detour_max`, analog `lbl_off_max`, v2 `lbl_own_gap_max`). LR/RL is still
  unsqueezed (bench-75 / dense-lr `compact_max`).

## 1. Setup and reproduction

- Worktree of 7122a53, `git apply B/work/a2g/b128-squeeze-on-9f51f77.diff`: clean (offsets 19-132 lines). The diff
  is the TD/BT variant of `B/work/a2c/b128-x1x2x4-td.patch` plus an `SQDIR` env gate; the two B12.8 tests of
  `B/work/a2c/b128-x1x2x4-attempt.diff` no longer apply (their neighbour test is gone) and were re-added by hand.
- Build: suite-on (`--cfg flowmaid_blocks`), release, `CARGO_BUILD_JOBS=2`, lib + `tests/quality.rs`
  (`squeeze/ton.sh`), CLI + `quality` example (`squeeze/build.sh`); local judge `squeeze/judge.sh` (runs
  `B/local/run.sh fm|judge` into `squeeze/local/<label>`; the live judge files are only read).
- Baseline check: with every knob off the CLI output is **byte-identical to `B/work/a2i/fm-on-split`** on all 95
  local inputs (judge 91 / 444 checks, same lines).
- For attribution the worktree got env knobs (`SQX1`, `SQX4`, `SQDIR`, later `SQW`, `SQK`; kept in
  `squeeze/p1-with-knobs.diff`, not in `proto.diff`).

### 1.1 Which component breaks what (static unless marked; 4 repository fixtures and lr drawn BT)

| X1 | X2/X3 (TD/BT) | X4 | bench-75 static x / fan / gap | public routed fan | dense-td static fan | hub5 order | lr BT x |
|---|---|---|---|---|---|---|---|
| - | - | - | 204 / 4 / 15.25 | 0 | 0 | ok | 1 |
| on | - | - | 208 / **6** / **12.50** | **1** | 0 | ok | **2** |
| - | - | on | 204 / 4 / 15.25 | 0 | **1** (`n16->n35 x n16->n32`) | ok | 1 |
| - | on | - | 204 / 4 / 15.25 | 0 | 0 | **fails** | **2** |
| on | on | - | 208 / 6 / 12.50 | **1** | **3** | ok | n/m |
| - | on | on | 204 / 4 / 15.25 | 0 | 0 | **fails** | 2 |
| on | on | on (= the diff) | 208 / 6 / 12.50 | **1** | **3** | ok | 2 |

(Measured with the knob build's `quality` example on `squeeze/in4` and `squeeze/inlr`; outputs `squeeze/sq0`,
`squeeze/base`; n/m = not measured. "hub5 order" is the test's end-order check on HUB's bottom side.) bench-75 is LR, so every bench-75 change comes from X1 (X4 changes nothing
there).

Local judge `--due A2` (444 checks): base 91; X1 alone **99**; full diff 70; diff without X1 **67**; prototype 72.

## 2. Each failing test: the exact cause

### 2.1 `blockcheck::a_hub_fan_leaves_in_target_order`

Two independent causes, one per fixture the test reads.

**(a) Public fixture, routed `fan_x` 1 (`ALERT Top ALERT->PAGER x ALERT->MAIL`).** Coordinate change: X1 stops
Brandes-Köpf aligning chain dummies of different cluster paths; in the squeezed layout the alert rules (`ALERT`, in
Observability) lose the column that the two upstream chains to the mail relay and the pager gave them: centre x
**775.7 (squeeze without X1) -> 726.6 (with X1)** (pager 757.2-872.8, mail relay 614.3-707.2; at 7122a53, unsqueezed,
the alert rules stand at 950.8 in an 815 px broad frame). Static is still fan-free: `ALERT->MAIL`'s channel at x 775.8 lies inside
`ALERT->PAGER`'s channel 809.8 / 854.2 (nested). `route()` at those centres:
- `ALERT->PAGER` (ends overlap in breadth) is drawn as a step from the port into the pager's column, x 780.4;
- `ALERT->MAIL` gets `cross_cluster_route`'s 4-point channel, whose straight legs run through Order processing /
  Payments; `router::repair` reroutes it **round the right**, column x 852.8, then left along y 296.5 into the mail
  relay;
- the ports / nudge / comb stages then order ALERT's top side by the paths' headings (pager 736.6, mail 756.2): the
  fan nests at the hub and **crosses at (780.4, 296.5)** where the mail edge comes back over the pager's column.
Without X1 the alert rules stay under the pager (775.7), `ALERT->PAGER` is one straight segment at x 801.3 and
`ALERT->MAIL` goes up at x 763.7 left of it: no crossing. With the prototype's weight rule (2.1 b) the Observability
block moves left (alert rules at 510.3) and the **same** route() mechanism crosses again at (769, 422)
(`ALERT->MAIL` column 812 right of `ALERT->PAGER`'s 769, entering the mail relay's right side).
Stage that makes it cross: `router::repair`'s lattice path for the rejected 4-point channel, which knows no corridor
and no slot order. The fix is route()-side: R3's corridor per band ("the wide-enough gap nearest the breadth of the
previous leg", R5) and R3's slot rule ("edges leaving at the same flow position are ordered by the breadth of the far
end"), which would put the mail edge in the gap between Order processing and the pager edge. Pictures:
`squeeze/pic/bo-routed.base-vs-sq0.png` (7122a53 vs full squeeze, routed), `squeeze/pic/bo-sq0.static-vs-routed.png`,
`squeeze/pic/bo-p1.static-vs-routed.png` (prototype).

**(b) hub5, static order (appears once X1 is off).** Message: `ends at 194.66 and 208.66 lead to 300.8 and 220.6`.
Coordinate change: X3 moves block Email right, `e1` (render) **228.8 -> 337.1**. Trace of the root-level sweep
(debug print in `block_compact`): Email's unit has three links, `e1` to the last dummy of `HUB -> e1` (weight 2,
target 71.7 relative), `e1` to `d2` (weight 1, 168.2) and the dummy of the back edge `e2 -.-> a1` inside Email to
its dummy outside (dummy-dummy, **weight 8**, 241.7 -> 258.8): the weighted median is the bounce's column. The
`HUB -> e1` chain is a separate rigid run left of Billing and Delivery (the order put it there), so the edge now runs
down the left and back right under Delivery into `e1`'s left side: its port (194.7) is left of `HUB -> d1`'s (208.7)
while its end is right of it. Fix that keeps the test's intent (static, X3): a chain segment that crosses a border is
no spine (the spec's own X1 reasoning), so it pulls a block like a border link: weight `COMPACT_MIXED_WEIGHT` (2),
not `COMPACT_DUMMY_WEIGHT` (8). Result `e1` end 196.7, `d1` end 197.5: in order. Picture
`squeeze/pic/hub5.nosqw-vs-p1.png`. Cost: the public family's `lbl_crowd` goes 0 -> 1 against the no-X1 variant (base
had 2), because Observability is pulled left to its scrape sources.

### 2.2 `blockcheck::a_hubs_fan_keeps_its_order_through_the_bands` and `an_end_of_a_title_row_member_keeps_out_of_its_chips_breadth_below_the_jog_room`

Both fail on the same three dense-td fan crossings: `n16 Bottom n31->n16 x n16->n31 at (2174,727)`, `n31 Top` (the
same pair), `n16 Bottom n16->n35 x n16->n32 at (2295,1133)`. Only X1 + X2 together produce them (table 1.1).
- Coordinates: across the flow the nodes barely move (n43 2453.4 -> 2445.5, n44 -> 2459.3) but the **chain columns**
  do: with X1 the twins' chains no longer align across `g1`'s border, and the squeeze of `g1`'s level packs them in the
  other order. Layout chain points (stage `in`): `n31->n16` at (2142.8, 635.5) and (2283.1, 818.5), `n16->n31` at
  (2096.8, 635.5) and (2174.3, 818.5): the back twin runs **right** of the forward twin. G2 also counts five fewer
  tracks in a gap inside `g1` (`g1` bottom 922.5 -> 860.0, every lower frame 62.5 px up).
- Stage that crosses them (`dbg-stages` trace): `ports::assign` gives the twin pair its fixed pair order on `n16`'s
  bottom (`n31->n16` 2096.1 left, `n16->n31` 2105.7 right), i.e. against the chains; `ortho` steps the two across at
  y 727-739; the comb keeps it. `n16->n35` is rejected after `ortho` and `router::repair` reroutes it; `nudge` puts it
  on columns 2224.1 / 2294.9 across `n16->n32`'s run (the fragile fan of A1 sections 3, now with new columns).
- Fix that keeps the tests and the squeeze: drop X1 (X2 + X4 alone: dense-td static fan 0, x 45, both tests pass,
  also under the prototype). A narrower alternative, not built: make the twin pair's port order follow the order of
  their chains at the first layer (ports stage), which would cure the twin crossing but not the `n16->n35` reroute.

### 2.3 `casing::tests::a_line_takes_one_role_across_a_cluster_of_crossings` (routed 34 > 3)

Probe test over `tests/fixtures/quality` (ignored test in the worktree, per fixture): 7122a53 `bench-75 routed 3`; full
squeeze **`bench-75 routed 34`**, nothing else (clusters at (700,272), (714,290), (1286,571), (1803,607-630): the
service fans into `db`, `cache`, `metrics`, `logs`). bench-75 is LR, so this is **X1 alone**: it moves bench-75's
chain columns (static fan 4 -> 6, gap 15.25 -> 12.50, x 204 -> 208) and `route()` at those centres turns them into
crossing clusters (routed x 243 -> 274, defects 15 -> 27, min gap 12.50 -> 0.02). Without X1 bench-75 is untouched
(3) but dense-td adds one (`n16->n41` routed at (2200,1233)) = 4 > 3; with the cross-border weight rule dense-td's is
gone again and the test passes (3).

### 2.4 `blockcheck::a_displaced_end_never_steps_back_...` (lr BT x_drawn 2; not in round 3's list, written in round 4)

Two independent causes (table 1.1): X1 alone and X2 alone each give 2. The X2 one, traced: the squeeze packs the
`EDGE` block ("Edge and shared services with a long title") to its members' least breadth; the frame becomes
**title-bound** (F1 grows it to its title), and `scene::frame_boxes` adds X2.2's column room only to frames that are
not title-bound (the D6 of rounds 2-3: "a frame F1 grew to its title is not grown for them"). `pager` then lies
wholly under the chip; `ship -> pager` crosses the title side outside the frame and enters through its right wall,
`cart -> pager` goes round into `pager`'s right side; they cross twice. Picture `squeeze/pic/lrbt.base-vs-x2.png`.
Fix that keeps the test and the squeeze (static, X2.2 "nothing foreign ever lies under a title that is wider than the
content"): count each block's displaced entries at Brandes-Köpf's breadths (`title_counts`, which is the spec's
`k0`: "BK; k0 per block; X2 with k0") and, for a block with entries whose members Brandes-Köpf spread at least as
broad as its title, keep Brandes-Köpf's breadths when the squeeze would pack the members under the title. lr BT
x_drawn **0** (7122a53: 1). Picture `squeeze/pic/lrbt.nok-vs-p1.png`. It changes no local judge line.

### 2.5 Also found

- **A hang** (not a test failure in the shipped list, but `routing_stays_finite_for_any_node_sizes_and_coincident_centres`
  ran > 15 min once X1 was off): `scene_sized` of `blocks-overview-subgraphs-reversed` with non-finite sizes loops in
  `ortho::TitleSides::of` (gdb: `ortho.rs:1452`). `while let Some(&o) = taken.iter().find(|&&o| (o - col).abs() <
  TRACK_PITCH) { col = o + TRACK_PITCH; }` is a float fixed point: `(o + TRACK_PITCH) - o` can round below
  `TRACK_PITCH`, so `find` returns the same `o` for ever. Latent in 7122a53 (written in `fb56b2f`/`507b0d6`), exposed
  by the squeeze's coordinates. Fix: compare against `TRACK_PITCH - 1e-6` (`squeeze/ortho-pitch-eps.diff`); the hang
  is gone and no output changes (prototype and knob runs byte-identical on the 95 local inputs).
- `quality_gate`'s hidden row: `bench-75 static: min_edge_gap = 12.50 < floor 14.7` with the full diff (X1); gone
  without X1.

## 3. The prototype (`squeeze/proto.diff`, 934 lines: `layout.rs` +790, `ortho.rs` +10, `blockcheck.rs` +57)

Contents, all block mode only:
1. The squeeze diff **without X1** (`coordinates_bk` and `vertical_alignment` as at 7122a53); X2/X3 in TD/BT, X4 in
   every direction, `BLOCK_SLACK` = 4, `RUN_ROOM` = `CLEAR_GAP`, as in the a2g diff.
2. In `block_compact`'s weights: two dummies of different cluster paths weigh `COMPACT_MIXED_WEIGHT` (2.1 b).
3. X2.2 keep rule: `title_counts` at Brandes-Köpf's breadths before the squeeze, `title_least[c]` =
   `title_fit_width` for blocks with entries; a level whose packed member span would fall under it while
   Brandes-Köpf's did not keeps its breadths (2.4).
4. `ortho::TitleSides::of`: `PITCH_EPS` (2.5).
5. Tests `a_block_is_squeezed_to_its_content` (public family + BT: `compact_max <= 7.97`, no frame overlap / node in
   frame) and `a_spine_inside_a_block_is_straight` (REPO / BUILD / REG one column; `spine_off` 0 on the public
   fixture, hub5, spread-face, nested-composite, fork-twins). Both pass.

### 3.1 Numbers (suite-on, prototype vs 7122a53)

| what | 7122a53 | full diff | prototype |
|---|---|---|---|
| suite-on failing tests | 14 | 14 - 1 + 5 | **14 - 1 + 1** (`a_hub_fan_leaves_in_target_order`, public routed) |
| the four round-3 tests + lr BT test | pass | 5 fail | 4 pass, public routed fan fails |
| local judge A2 / A1 (444 checks) | 91 / 0 | 70 / 0 | **72** / 0 (67 without rule 2) |
| new `quality_gate` rows | - | bench-75 static gap 12.50 | none |
| dense-td static x / fan / gap | 42 / 0 / 12.50 | 39 / 3 / 12.50 | **45 / 0 / 12.50** (must 74.6) |
| bench-75 static x / fan / gap | 204 / 4 / 15.25 | 208 / 6 / 12.50 | **204 / 4 / 15.25** (unchanged) |
| dense-td routed x / fan | 107 / 34 | 94 / 24 | 103 / 24 |
| bench-75 routed x / fan / defects | 243 / 24 / 15 | 274 / 27 / 27 | 243 / 24 / 15 (unchanged) |
| public fixture compact_max / mean, sib_stagger | 19.97 / 7.25, 1 | 6.47 / 4.54, 0 | 6.47 / 4.61, 0 |
| hub5 compact_max | 9.81 | 5.23 | 6.47 |

Judge lines, prototype against 7122a53: **31 fixed** (sib_stagger 12, compact_mean 6, compact_max 5, lbl_struck 4,
area 2, detour_max 1, lateral_x 1); **12 new**: public family `bends_max` 8 -> 12 on `OPS>[OBS]` and `detour_max`
1.22 -> 1.48 on `SHOP>CDN` (5 copies each; round 1 saw the same two edges), analog `lbl_off_max` 0.357 (must 0.35),
v2 `lbl_own_gap_max` 9.5 (must 1). The same 12 come with the full diff and without X1, so they are X2/X3/X4's, not
the prototype's rules. Remaining squeeze-related must lines: bench-75 and dense-lr `compact_max` (LR is not squeezed).

Already-failing tests, changes inside them (must be reported, none is a new test name):
- `defect_baseline_gate` static: lat_wiggles **33 -> 42** (dense-td 11 -> 20: `n16->n32, n31, n40, n41, n35, n43,
  n37, n23, n44, n29` now step between band columns), lat_backtrack 10600 -> 13683, band_len 91602 -> 98168,
  band_pairs 204 -> 207, x_drawn 263 -> 267; sib_stagger 2 -> 0. (Full diff: lat_wiggles 36.)
- `a_one_px_drag_leaves_unrelated_edges_put`: dense-lr 0 -> 1 jump (`drag n22: n12 -> n44 21.5`), dense-td 3 -> 0.
- `block_fixtures_are_not_drawn_worse`: fewer violations (`notes/wide-frames` static and routed, `nested-cases`
  routed gone). `channels_keep_edges_apart_and_off_chips`: edge_chip 3 -> 2.
- `quality_gate`: the same five rows as 7122a53; inside them dense-td routed edge_node 1 -> 2, min gap 0.60 -> 0.94.

Default build: `identity::everything_renders_as_before_with_blocks_off` and the two other identity tests pass in a
default-cfg release build of the prototype. The full guard was not run (prototype, not a commit).
Timing (indicative only; load 2.2 from a parallel test run, two runs each): static `scene_ms` blocks-overview
10-15 -> 16-17 ms, hub5 3.0 -> 5.3-5.5 ms, dense-td 47 -> 37 ms. `tools/perf.sh` must be run before any commit.

## 4. Route()-side work: is it step 3, and how much

Yes. The one failure left is `route()` drawing a hub's two upstream corridor edges in the wrong nesting: R3 of 2.9
(corridors per band, "inner legs first", slots ordered by the far end's breadth) and R5 (gap nearest the previous
leg). Today `route_built` builds such an edge with `cross_cluster_route`'s 4-point channel and leaves every
collision to `router::repair`'s lattice, which has no notion of slot order. R1/R2 (frames, classes) are built
(B12.10), F3 is built, frames agree: feeding route() the squeezed frames is already done and is not enough.
Smallest useful piece: for E1/E3 edges whose 4-point channel meets a foreign unit, (i) group the obstacles of the
common-ancestor level into bands along the flow, (ii) per band pick the gap nearest the previous leg's breadth
(R5), (iii) give the edges sharing a gap slots ordered by the breadth of their far end, (iv) emit on-axes corners
for `ortho_pass`. Estimate: 250-400 lines in `scene.rs` (`route_built`) plus tests; it is the core of B12.13's R3,
so pulling it forward is pulling most of R3's corridor builder forward, not a patch. The two fixtures the test reads
are enough to verify it (`blocks-overview` routed, hub5 routed), plus the five public permutations and dense-td /
bench-75 routed rows (`quality_gate`) because those reroute on any frame change.

A static-only way to keep this row was looked for and not found: the row is clean only when the Observability block
happens to stand under the pager (X1 off and weight 8), which is the configuration that breaks hub5. No coordinate
rule found keeps both.

## 5. Proposal for the follow-up package

1. Commit the `ortho::TitleSides::of` pitch epsilon on its own, with a unit test (a `taken` set where `o +
   TRACK_PITCH` rounds under the pitch; the case is in `blocks-overview-subgraphs-reversed` with non-finite sizes
   once the squeeze is on). No output changes.
2. Take the decision on X1 to the lead (spec 2.5 X1 is a design rule): evidence is table 1.1, X1 alone 91 -> 99
   local must lines, bench-75 gap floor and fan, dense-td's `n16` fan, casing 34, lr BT. The squeeze keeps X1's own
   intent through `block_compact`'s spine classes (dummies of different cluster paths are never merged into a
   spine).
3. Build R3's corridor slot order in `route()` (section 4) **before or together with** the squeeze, since lead
   decision "after step 2's first stop" item 1 forbids a commit with `a_hub_fan_leaves_in_target_order` red.
   Verify: that test, `quality_gate` routed rows (no row name added; dense-td / bench-75 routed not worse), the
   frames-agree tests.
4. Then the squeeze as in `proto.diff` (X2/X3 TD/BT, X4, cross-border weight, X2.2 keep rule, `BLOCK_SLACK` 4 pinned)
   with its two tests. Expected: suite-on list = 7122a53's 14 minus `advanced_showcase`; judge about 72 (31 lines
   fixed, 12 added); no new `quality_gate` row.
5. Still open after that, each needing its own work: the 12 added B12.8 rows (`OPS>[OBS]` bends, `SHOP>CDN` detour,
   two label rows) and `DEFECT_BASELINE` lat_wiggles / lat_backtrack / band_len (dense-td's `n16` fan steps at
   band borders: X3's border spines / a run's columns kept straight across levels when the feasible intervals
   allow, not built), and the LR/RL squeeze (bench-75 / dense-lr `compact_max`; round 1's D6, bench-75 fan 4 -> 17).

## 6. Files

- `squeeze/proto.diff` (the prototype against 7122a53), `squeeze/p1-with-knobs.diff` (same plus the env knobs and
  debug probes used for attribution), `squeeze/ortho-pitch-eps.diff`.
- Suite-on logs: `squeeze/ton-sq0.*` (full diff), `ton-nox1b.*` (no X1), `ton-p1nosqw.*` (no X1, keep rule, weight
  8), `ton-p1.*` and `ton-proto.*` (prototype; identical lists). `ton-nox1.*` is the run the hang interrupted.
- Local judge: `squeeze/local/{base,sqfull,sqnox1,knobsoff,x1only,p1,p1-nosqw,proto}/judge-A2.txt` (+ `metrics.tsv`).
- Quality tables: `squeeze/base/qq.tsv`, `full-qq.tsv`, `p1-qq.tsv`, `proto-qq.tsv` (tests/fixtures/quality),
  `squeeze/base/q4.tsv`, `sq0/q4.tsv` (+ `.details`).
- Pictures: `squeeze/pic/bo-routed.base-vs-sq0.png`, `bo-sq0.static-vs-routed.png`, `bo-p1.static-vs-routed.png`,
  `hub5.nosqw-vs-p1.png`, `lrbt.base-vs-x2.png`, `lrbt.nok-vs-p1.png`.
- Prototype suite-on CLI: `squeeze/fm-proto`. Probe crate source: `squeeze/probe/` (dump, finite).
