# Study "regress": 7122a53's bulk regressions in block mode, and fb56b2f's two side effects

Read-only study for the focused follow-up package of `b12a2-geometry`. Base: `7122a53` (round 4 head). Worktree,
target dirs and binaries were removed after the measurements; diffs, notes, TSVs and pictures are kept in
`B/design-a2b/regress/` (`B` = `scratchpad/b12`). Nothing in `/home/user/flowmaid`, `B/metrics` or `B/local` was
changed.

## 1. Verdict

- **Cause confirmed, but not the verifier's exact mechanism.** `ortho::split_by_room` (7122a53) splits a channel by each
  step's *current* room. For a labelled step that is its room **above its label**, which is narrower than its plain room.
  Two things follow:
  1. **A set of one step never reaches `place_channel`.** A labelled step that ends up alone in its set keeps its
     above-label room. In the old single channel it would have dropped back to its plain room, because that channel
     also held an unlabelled step. The rule in `place_channel` says: "a step drawn above its label nests with the others
     only when every step of its channel does".
  2. **Sets are placed one after the other, ordered by their rooms.** The libavoid turn key, which orders a channel's
     steps so their legs do not cross, is no longer applied across sets. When the above-label rooms disagree with the
     turn key, the steps cross.

  The tracks are **not** within a pitch of each other: they stand 31 px apart (9a94998d) and 64 px apart (8ab502b3). A
  variant that merges the sets again when two tracks come within a pitch changes neither input (FM_RFIX=2 in
  `experiment-probe-variants.diff`). The verifier's mechanism (two independently placed sets that are never kept a pitch
  apart) is therefore not what happens.
- **Fix (prototyped):** split by the **plain** room (`Jog::plain.0`), the widest room `place_channel` can give a step.
  `jog()` builds the above-label room by clipping the plain room, so room ⊆ plain always holds. When two sets' plain
  rooms do not chain, the sets can neither share a track nor be reordered by the key: a frame border or title strip
  lies between them. Steps whose plain rooms meet stay one channel and get the old fallback and key order back. The
  change is 1 line of logic plus comments, a regression test and a new fixture: `B/design-a2b/regress/proto.diff`. It
  applies cleanly to 7122a53.
- **Result:**
  - Both regressions are gone. The two inputs, and 8 more of the 18 bulk inputs 7122a53 changed, render byte-identical
    to fb56b2f again.
  - 7122a53's gains are kept byte-identical: t9c TD/BT `lateral_x` 0, nested-cases LR.
  - fb56b2f's gains are kept: lr BT `x_drawn` 1, and 0 path reversals over 364 direction copies and 2588 bulk inputs.
  - No repository fixture copy, stress input or local-judge input changes, so the local judge stays at A2 91 / A1 0.
  - Suite-on tests: the same 14 failing names. The new test passes, and fails at 7122a53.
- **fb56b2f's TitleSides rule (x2-title-side):** the cause is confirmed (§5). The rule is correct by its own criterion.
  The loss comes from an in-between end that the layout reserved no jog room for. The obvious extension (displace
  that end too, variant T1) makes x2 TD and lr BT worse and is rejected with numbers. A real fix needs layout jog room,
  so it is not part of this patch.
- **2.2 px S-jog in lr BT:** the cause is traced (§6). It is a pre-existing half-pixel disagreement between the ports
  stage's chip limit and the router's chip clearance. fb56b2f only changed which reroute shows it. Not prototyped; a
  fix direction is given.

## 2. Cause, traced (probe build with env-gated prints; `experiment-probe-variants.diff`)

**9a94998d (LR, `K0 -> K1` and `K1 -> K0 : back`)**
- Gap 4 holds two steps:
  - edge 9 `K0->K1`: unlabelled, room = plain (541.2, 589.2), want 570.4.
  - edge 10 `back`: above-label room (596.2, 607.2), plain (551.7, 607.2).
- The above-label rooms do not chain, so 7122a53 makes two sets of one step each:
  - `K0->K1` goes to 570.45.
  - `back` keeps 601.72 in its above-label room.
  - `back`'s run at breadth 238 then crosses `K0->K1`'s step at 570.4 (which spans 217.8 to 269.7).
- `uncross` swaps the twins' ends. After `ortho` (stage trace):
  - `K0->K1` runs 238.0 → 293.2.
  - `K1->K0` runs 269.7 → 217.8 as a 14-point path that climbs to breadth 189.8 at flow 607.2, beside K1's wall.
  - `nudge` can no longer straighten either edge. These are the drawn "stepped Zs", `ortho_edges` 5 → 7.
- With the plain-room split, the two steps form one channel again:
  - `place_channel` falls back to plain rooms: 586.5 and 554.4, nested by the key.
  - `nudge` then straightens both edges (217.0 and 244.0, `n=2`), byte-identical to 9c142b3, 7892fb7 and fb56b2f.

**8ab502b3 (TD, `A->K1 : g2`, `A->B0 : g4`, `K1->A : back`)**
- All three steps are labelled and above their labels.
- The above-label rooms split them into {g2 (169.5, 201.2), g4 (169.5, 236.8)} and {back (259.1, 275.0)}.
- `back` is placed at 267.06, under g4's step at 203.1, so its run crosses g4's leg into B0: `x_drawn`, `edge_edge_x`
  and `fan_x` each +1.
- As one channel the order does not hold (back's room lies above g4's), so all three fall back to plain rooms. All
  three plain rooms chain: (169.5, 257.0), (169.5, 269.5) and (173.5, 275.0). The key then puts `back` between g2 and
  g4, as 7892fb7 drew it.

**Repository fixture that shows it.**
- None of the 364 direction copies changes under the fix, so a new fixture was needed:
  `tests/fixtures/blocks/twoway-noted.mmd` (also saved as `regress/twoway-noted.mmd`). It is a state pair of composites
  `K0`/`K1` with `K1 --> K0 : back` and two notes left of K1. All names are neutral and none comes from the local
  family.
- Drawn LR and RL, 7122a53 shows the same mechanism: `back` runs up K1's wall, static `border_runs` 1, `border_len`
  61.9, `ortho_edges` 3.
- With the fix: `border_runs` 0, `ortho_edges` 1, two straight runs. The output equals 9c142b3's.
- TD/BT are unaffected in every build.

## 3. The prototype (`regress/proto.diff`, 5 files, +81 / -11)

- `src/ortho.rs`, `split_by_room`:
  - It uses `let room = |k| jogs[k].plain.0;` for the sort and the chaining, instead of `jogs[k].room`.
  - Its doc comment says why: room ⊆ plain, the fallback, and turn order versus room order.
  - The comment in `place_tracks` now says "plain rooms".
  - Nothing else changes. `place_channel` and the singleton handling stay as they are. Outside a block layout the
    function still returns the channel unchanged, so the default build stays byte-identical.
- New test: `blockcheck::a_two_way_pair_between_blocks_keeps_its_steps_in_one_channel`.
  - Fixture `blocks/twoway-noted` drawn LR and RL.
  - Asserts `(border_runs, x_drawn) == (0, 0)` and that both pair edges are each one straight run.
  - It fails at 7122a53 (checked: panics at the first assert) and passes with the fix.
- Registering the new fixture (needed by three directory-iterating tests; this is not a loosened gate):
  - `tests/fixtures/identity.tsv`: a row
    `tests/fixtures/blocks/twoway-noted.mmd 97f3b3acf179bb13 2ebf8a501a52edbd 9af2a3a1de5c6a97 blocks`. These are the
    blocks-off hashes the identity test prints. The default-cfg build (blocks off, byte-identical to dev by the
    package's own gate) passes `identity::everything_renders_as_before_with_blocks_off` and
    `levels::tests::the_permanent_identity_rows_are_the_fixtures_outside_block_mode` with it.
  - `tests/quality.rs`, `BLOCK_CEILINGS`: `("blocks/twoway-noted", (0,0,0,0,0), (0,0,0,0,0))`. These are the drawing's
    own numbers before the block layout, measured with the default build's quality example: defects,
    edge_frame_pairs, overlap_pairs, x_drawn and hard zeros are all 0, static and routed.

## 4. Measurements

**Configs:**

| config | commit | binary |
|---|---|---|
| pre | 9c142b3, block mode | verifier `r6/pre-on-target` |
| t78 | 7892fb7 | verifier `r6/t78-target` |
| on | 7122a53 | my probe build, byte-identical output to the verifier's `on-target` |
| proto | 7122a53 + fix | — |

- The clean `proto` build and the env variant `FM_RFIX=1` produce identical SVGs on all 2975 inputs measured
  (`m/proto-vs-r1.txt`), so the `*-r1.tsv` files are proto's numbers.
- Quality example, static unless marked. TSVs are in `regress/m/`.

**Families:**

| family | inputs |
|---|---|
| b8r1/bulkst | 964 |
| b8r1/bulkfc | 1210 (one file does not parse in any build) |
| b3r4/all | **413 valid**: 583 of its 996 entries are dangling symlinks, so the verifier's 996 include 583 inputs that fail identically in every build |
| a2i `dirs` + `dirsx` | 332 + 32 direction copies |
| verifier r6 stress | 23 |

**Totals (static)**

| family | config | x_drawn | chip_crowd | fan_x | lateral_x | side_ends | ortho_edges | bends_max | defects |
|---|---|---|---|---|---|---|---|---|---|
| bulkst | pre | 314 | 770 | 99 | 19 | 66 | 6790 | 5747 | 275 |
| bulkst | t78 | 307 | 777 | 96 | 18 | 64 | 6788 | 5679 | 269 |
| bulkst | on | **308** | **778** | **97** | 18 | 64 | **6789** | 5678 | 269 |
| bulkst | proto | **307** | **777** | **96** | 18 | 64 | **6788** | 5678 | 269 |
| bulkfc | pre / t78 / on / proto | 86335 / 86300 / 86300 / 86300 | 610 / 657 / 657 / 657 | 1248 / 1249 / 1249 / 1249 | 184 / 183 / 183 / 183 | 357 / 355 / 355 / 355 | 9246 (all four) | 4377 / 4356 / 4356 / 4356 | 37637 / 37638 / 37638 / 37638 |
| all | pre / t78 = on = proto | 303 / 294 | 76 / 81 | 5 / 5 | 13 / 13 | 48 / 46 | 1686 / 1686 | 1623 / 1590 | 41 / 40 |
| dirs | pre / t78 / on / proto | 1290 / 1285 / 1283 / 1283 | 262 / 273 / 272 / 272 | 30 (all four) | 223 / 219 / 216 / 216 | 226 / 231 / 231 / 231 | 2603 (all four) | 1379 / 1381 / 1380 / 1380 | 41 (all four) |
| stress | pre / t78 / on / proto | 1 / 2 / 1 / 1 | 3 / 5 / 5 / 5 | 0 | 13 / 13 / 16 / 16 | 7 / 4 / 7 / 7 | 92 | 70 / 73 / 72 / 72 | 0 |

Routed totals are equal between on and proto everywhere. The only routed change on → proto is 9a94998d `ends_apart`
4 → 2.

**The 19 inputs round 4 changed** (`m/r4chg-class.txt`, classified by SVG md5 against the t78, tfb (fb56b2f), on and
proto CLIs):
- **fb56b2f** changed 1 (bulkfc/2ff70159). **7122a53** changed 18.
- **proto** renders 10 of those 18 byte-identical to fb56b2f again: 9a94998d, 8ab502b3, 2fc99881, 3d210104, b0ebca7e,
  cd11c667, d077d02b, d4555aee, r6_rand_r210, r6_rand_r246.
- proto keeps 7122a53's output on the other 8: 1452324c, 165c1a64, 33802599, 542c7f20, 7133c5d0, a57fba35, 030a05e7,
  9d2b0471.

Their t78 → proto changes are small and net neutral:
- chip_crowd 165c 1→0 and 7133c 0→1;
- bends_max a57f 2→1;
- edge lengths a little shorter;
- 030a05e7's edge_len 976 → 963.

Sums over the 19 (static):

| config | x_drawn | chip_crowd | fan_x | ortho_edges | bends_max | band_pairs |
|---|---|---|---|---|---|---|
| pre (9c142b3) | 12 | 19 | 3 | 161 | 116 | 8 |
| t78 (7892fb7) | 12 | 20 | 3 | 161 | 114 | 8 |
| on (7122a53) | **13** | **21** | **4** | **162** | 113 | **11** |
| proto | **12** | **20** | **3** | **161** | 113 | **8** |

- This reproduces the verifier's "x_drawn 12→13, chip_crowd 19→21, fan_x 3→4" (pre → on).
- The remaining chip_crowd +1 against pre is bulkst/1452324c (1 → 2). It was already 2 at 7892fb7, so round 4 did not
  make it; it is flagged here for whoever owns the earlier commits.

**Per input, on → proto** (`m/diff-on-r1.txt`):
- 8ab502b3: x_drawn, edge_edge_x and fan_x 1→0; band_pairs 3→0.
- 9a94998d: ortho_edges 7→5, fan_uneven 2→1, edge_len 2197→2053.
- 3d210104: chip_crowd 1→0.
- 2fc99881: tight_pairs 2→1.
- d077d02b / r6_rand_r210: min_node_gap 14.5→25.9.
- No input gets worse on a counted row. `chip_gap_min` falls back to its 7892fb7 values where 7122a53 had raised it.

**Gains kept** (`m/diff-t78-r1.txt`; pictures `pic/kept/`, 7892fb7 | proto):

| what | result |
|---|---|
| t9c TD / BT | `lateral_x` 2→0, SVG byte-identical to 7122a53 |
| nested-cases LR | byte-identical to 7122a53 |
| lr BT (fb56b2f) | x_drawn 3→1, `xhops` 1→0, detour_max 2.80→1.18 |
| reversals (`verify/.../r5/reversals.py`) | **0** over the 364 proto-rendered direction copies and 0 over 2588 proto-rendered bulk inputs (7122a53: also 0) |

**Not changed by the fix:**
- 0 of 364 direction copies, 0 of 23 stress inputs, 0 of `tests/fixtures/*/*.mmd`.
- 0 of the 95 files in `B/local/in`, so the local judge result is that of `fr4-split`: `--due A2` 91 must-fail,
  `--due A1` 0. I did not run the judge, to keep `B/local/out` untouched.

**Tests:**
- Suite-on (`--cfg flowmaid_blocks`, `cargo test --release --lib --test quality`, `regress/ton-proto.txt` and `.fail`):
  1181 passed, 8 failed (lib) plus 9 passed, 6 failed (quality). The failing list is the same 14 names as
  `a2i/ton-split.fail`. Without the identity and ceiling rows, the three fixture-registration tests also fail; with
  them, they pass.
- Default cfg: the identity, levels-row and new tests pass.
- Not run: the full guard (clippy, msrv, wasm, doc) and perf. The change is O(n log n) in the same sort as before, so no
  cost is expected.

## 5. fb56b2f's TitleSides rule on x2-title-side

**Effects:**

| copy | lateral_x | side_ends | x_drawn |
|---|---|---|---|
| x2 TD (and its undirected copy) | 1→2 | 0→1 | 0→0 (lat_backtrack 512→237, edge_len 3097→2581) |
| x2 BT | 0→1 | 0→1 | 1→0 |

**Cause (probe output):**
- Frame GW's chip ends at c1 = 398.5. Displaced, wholly shaded: `u1->g1` at 121.5 and `u3->g3` at 244.0.
- `u2->g2` enters `b` at 400.6, 2.1 px past c1. `b` is only partly under the chip, so the entry is not displaced; it is
  a "taken" crossing.
- `u1->g1`'s first free column is 413.1, and 400.6 lies between its port and that column. The rule stops there ("and
  every entry after it"), so neither shaded entry is displaced.
- `ortho` draws both through the chip. `router::repair` then:
  - takes `u1->g1` in through GW's left wall (+1 lateral);
  - takes `u3->g3` round past the chip;
  - moves `u2->g2`'s exit to user two's right side (+1 side_ends).
- By its own criterion the rule is right: the step back from 413 to 121.5 would cross `u2->g2`'s run inside the frame.
  At 7892fb7 the router happened to move that entry out past the columns.

**Variant T1 tried (FM_TFIX=1):**
- What it does: give the side's member ends lying between a displaced port and its column a column too, in breadth
  order, all or none.
- On x2 TD it displaces all three, as intended (cols 412.5 / 425 / 437.5).
- Result: x2 TD x_drawn 0→2, x2 BT x_drawn 0→1 (lateral −1, side_ends −1). lr BT: x_drawn 1→0, but bends_max 3→6,
  chip_crowd 0→2, lateral_x 3→4, and the dotted `page` takes a long detour. Pictures: `pic/t1/`.
- Why it fails: `layout::title_entries` and `track_need` reserve jog room only for the shaded entries. A third step in
  the jog room does not fit, and the router reroutes.
- Rejected. A real fix needs the layout to count such in-between member ends in G1/G2 (jog room plus their columns past
  the chip) before ortho may displace them. That is a layout and ortho change of its own, to be measured against lr BT
  and dense-td's `n43` fan (the reason the shaded-only rule exists). It is not part of this patch.

## 6. The 2.2 px S-jog in blocks/lr BT

`cart -> pager` (label `page`) at fb56b2f and 7122a53 comes down at x 322.8 and steps to 320.6 at y 661.5, which is
EDGE's title side, then runs into pager's top.

**Stage trace:**
- The ortho path is clean: down at 320.5 into pager.
- `router::repair` replaces it with a path that comes down the lattice line x 323.5 and steps 3.0 px to the port
  (320.5). `nudge` trims that to 2.3 px. `despike` plays no part, because the step is perpendicular.

**Why repair replaced the path:**
- `ports::unshaded` puts the first free port of a chip-shaded side exactly at chip + `CHIP_SEP`. Its own comment says
  "exactly on the router's limit … counts as running along the frame's name and is rerouted".
- `ortho`'s `CHIP_CLEAR` is `CHIP_SEP + SEP_AIR`; TitleSides puts c1 at 321.0, so 320.5 counts as under the chip.
- The router's chip obstacles are grown by a scaled `CHIP_CLEAR` (`router::solids`), so its lattice line stands a few
  px further out.

**Whose change:**
- This half-pixel disagreement predates round 4.
- At 7892fb7 the same edge was rerouted into pager's right side with a 4 px spike.
- fb56b2f's TitleSides change (`cdn -> gateway` no longer displaced) let the reroute come into the top instead, with the
  jog.

**Fix direction (not prototyped):** make the two limits one. Either the ports stage leaves the first free port where a
stroke may run (chip + the router's clearance + `SEP_AIR`), or the router accepts a run at the ports stage's limit.
Either changes every chip-shaded side in block layouts, so it needs its own measurement over the 364 copies and the
bulk sets.

## 7. Proposed plan for the follow-up package

1. Apply `regress/proto.diff` on the package head:
   - `ortho.rs` `split_by_room` uses `plain.0` rooms, with its comments;
   - new fixture `blocks/twoway-noted.mmd` and test `a_two_way_pair_between_blocks_keeps_its_steps_in_one_channel`;
   - its identity row and ceiling row. Re-take the identity hashes from dev's own build per the standing rule
     (expected `97f3b3acf179bb13 2ebf8a501a52edbd 9af2a3a1de5c6a97`).

   One commit. Its message names only the public and repository fixtures it changes: none of the existing copies; the
   new fixture LR/RL.
2. Verify:
   - suite-on list = the 14 names;
   - default guard green, with the off-identity test;
   - SVG md5 over the 364 copies unchanged against 7122a53 (expected 0 changed);
   - bulk quality over bulkst, bulkfc and the 413 valid b3r4/all inputs: the 10 inputs listed in §4 return to fb56b2f's
     output, nothing else changes;
   - reversals 0;
   - perf.sh two runs.
3. Keep the x2-title-side TitleSides item and the lr BT S-jog as separate, reported items, with causes as in §5 and §6.
   Do not fold them into this commit.

## 8. Files

- `regress/proto.diff`: the prototype (applies cleanly to 7122a53).
- `regress/experiment-probe-variants.diff`: the probe build:
  - env-gated prints;
  - `FM_RFIX` 1/2/3 = plain split / re-merge on clash / no split;
  - `FM_TFIX=1` (T1);
  - the a2i stage-debug patch (`FMDBGE`).
- `regress/m/`: quality TSVs per family and config (`*-pre`, `*-t78`, `*-on`, `*-r1` = proto, `*-r1t1`). Also
  `diff-on-r1.txt`, `diff-t78-r1.txt`, `r4chg-class.txt`, `proto-vs-r1.txt`.
- `regress/ton-proto.txt` and `regress/ton-proto.fail`: the suite-on test run with the prototype.
- `regress/qrun.sh`, `qdiff.py`, `summ.py`: measurement scripts. qrun.sh's `all` entry points at the raw b3r4/all; the
  final `all` TSVs were taken over `regress/in/all`, the 413 valid links.
- `regress/min/`: `reduce.py`, the delta-reduced inputs and the crafted cases (`c/c3.mmd` = the fixture).
- `regress/pic/`:
  - `fix/` (7122a53 | proto): 9a94998d, 8ab502b3, twoway-noted;
  - `kept/` (7892fb7 | proto): t9c TD/BT, nested-cases LR, lr BT;
  - `t1/`: the rejected T1 variant;
  - `min/c3.pair.png`.
