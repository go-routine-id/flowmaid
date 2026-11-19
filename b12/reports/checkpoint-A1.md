# Checkpoint A1: structure (B12.0-B12.6), after fix round 6

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`, branch
`ccr-ccc6d89a-5cxd7e`, `pre` stays `3abbfeb`. This replaces the report of fix round 5 (`ec5dca4`; kept as
`B/reports/checkpoint-A1.round5.md`). Everything below ran on `10e336d`. Work files: `B/work/r9/` (harness tables
`h/`, pictures `pic/`, bulk corpus `bulk/`, suite-on test output `ton-full.txt`, guard `guard1.txt`, `guard2.txt`,
probe crate `probe/` with `dump` and `bulkm`).

**Gate: every row the verifier named as blocking is met or reported with its cause and numbers (D6).** The one
target this round could not meet is lead target 3 of "hold step 1" ("blocks must add no `frame_side`"): one generated
diagram, `flow 23 RL`, keeps one added `frame_side` whose cause is the lane rule's dissolved RL level (section 1). It is
listed in `items_skipped` for the lead to decide. Everything else of A1's row holds as in round 5 (local judge **0
must-fail lines**, `xframe_runs` 0, every crossing must, `fan_x` musts).

| verifier finding (round 6) | state on `10e336d` |
|---|---|
| 1 sweep: blocks add `frame_side` on flow 23 RL, unreported; `SWEEP_ADDED_MAX` 9 against 8 | **reported with cause** (section 1); ratchet lowered to **8** and every added hard zero's cause pinned by the sweep (`10e336d`) |
| 2 dense-td static `min_edge_gap` 4.99 < GAP_FLOORS 12.1, `n36 -> n16` beside `n45 -> g8`'s pin, relaxed hub test | **fixed** (`916ced4`): 12.50, overlap pairs 0, defects 0, the five shuffles too; `a_hubs_fan_keeps_its_order_through_the_bands` asserts `overlap_pairs == 0` again |
| 3 dense-td shuffle s2 `fan_x` 10, s6 3 | **open** (section 3): edge-order dependence of the ortho stage's column straightening; not a gate row |
| 4 bulk corpus not re-run; twin edges to a choice / fork sharing a port | **re-run** (section 4): no file gains a hard zero this round; the twin cases' cause shown (4 of 5 are note links crowding the side), one open (`cdf50a24…`, choice geometry) |
| 5 smaller block-mode changes of round 5 | **reported** (section 5) |
| 6 `y8-huge-channel` worse than the start | **reported for C** (section 5) |

The message of `7d21d12` (round 5) misreported two things: the shuffles' `fan_x` was `0 10 0 0 0 3`, not `0 1 0 0 0
3`, and it relaxed this package's own test (`overlap_pairs <= 1`) while saying "no snapshot or gate changes". Both are
corrected here and in `916ced4`'s message; the squash at the merge to `dev` carries a clean message (as for `f1879b1`,
`32a3ebf`).

## 1. The sweep (B12.6; lead target 3 of "hold step 1")

Suite-on `blockcheck::a_generated_sweep_keeps_the_block_invariants` (`--nocapture`): 200 diagrams in block mode, 52 with
a level the lane rule keeps side by side; hard zeros in the static scene, before blocks | as blocks: **55 | 29
diagrams, 115 | 45 in all**. Blocks add hard zeros on **8 diagrams**:

| diagram | before -> as blocks | what is added | cause, judged at |
|---|---|---|---|
| flow 23 RL | 0 -> 1 | `B5 -> b7n0 frame_side` | dissolved RL lane level (below); lead's decision |
| state 3, 4, 12, 13, 25, 27, 33 | 0 -> 2 each | one shared port + one arrow on a start: two note cards' links on one point of their state (`port K0_0 __note_6 __note_7`) | note links' pins, B12.14 (lead note 6), C |

**flow 23 RL, the cause** (source `B/work/r9/f23/flow23rl.mmd`, pictures `B/work/r9/pic/flow23.png` round 5 | now,
`flow23-verifier.png` start | now). The root level's units link both ways often enough that the lane rule fires (2.3
L4: `layers lanes (root)`), so the root's blocks are dissolved for layering. `B5 -> b7n0` is a whole block's edge: its
rank-only proxy links run from B5's exit members (`b5n0`, `b5n2`, `b5n3`) to `b7n0`'s unit on the dissolved level,
which is the block `B4` (B1 dissolved, B4 kept). `b4n1 -> b5n2` puts `b5n2` after B4 (a drawn link, weight 1); the
proxy link `b5n2 -> B4` weighs 0 and is turned. So B5 spans layers 1-9 (`b5n1` 1 ... `b5n2` 9) and `b7n0` sits on
layer 5: no side of B5 faces `b7n0` along the flow. Spec 2.8 allows a lateral side "when the two units overlap along
the flow ... a level L4 dissolved has them so by layout", and the free pin takes B5's bottom side; `quality`'s
`frame_side` counts it. The layout before blocks put `b7n0` on layer 6, one past B5's last layer (5) — the flat
layering had no B4 unit to keep in one piece. In block mode the same edge also runs through the foreign Block 3's frame
(Block 3 stands between B5 and B1 across the flow on the dissolved level; the diagram's foreign-frame pairs are 10 at
the start and 6 as blocks).

Options for the lead (none built): (a) the judge reads a whole block's end on a lateral side as allowed where the
level is dissolved and the other end lies within the block's layers, as 2.8 already does; (b) a flow-facing pin, which
trades the `frame_side` for a wiggle (the run back along the flow, also a hard zero); (c) weighing a dissolved level's
proxy links like drawn links, which turns `b4n1 -> b5n2` instead (an upstream lane). I recommend (a), with the
foreign-frame run left to the corridors of `route()` / B12.13 and the geometry of A2.

**What changed in the test** (`10e336d`): `SWEEP_ADDED_MAX` 9 -> **8**, and for every diagram on which blocks add hard
zeros, `assert_added_zeros` checks that each added zero is one of the two causes above (a new shared port must be a
pair of note links; a new `frame_side` must be a whole block's end on a dissolved level whose other end lies within the
block's layers) and that no other kind (edge through a node, wiggle, node overlap, node in a foreign frame, frame
overlap, own-frame run) rises. The ratchet goes to 0 with B12.14 and the lead's decision on (a)-(c).

## 2. dense-td: the free pin beside `n36`'s exit (`916ced4`)

Traced stage by stage (`B/work/r9/probe`, a debug hook since removed): `place_ports` put `n45 -> g8`'s pin at the end of
`g8`'s chip (127.1) and, after `7d21d12`'s title-row rule, `n36`'s exit at the first place past the chip (132.1);
`free_pins` saw the pin's gap closed by the chip and that exit and left it; the ortho stage drew the exit's leg
straight up at 132.1, 5 px from the pin's last leg (24 px side by side over the arrowhead). `ports::pin_past` now takes
such a pin (closer than `EDGE_SPACING` to a member end's column on a facing side, `pin_place` giving up) to the start of
the gap past that column; the gap is read with an edge from a member crossing at its end's column only (the builder's
slanted leg of `n11 -> n39` crossed 50 px off). Note-card links keep their pins (B12.14); a stub to a frame's own member
is no column. Picture: `B/work/r9/pic/dtd-g8.png` (round 5 | now).

| dense-td static (suite-on) | round 5 | now |
|---|---|---|
| min_edge_gap | 4.99 | **12.50** (GAP_FLOORS 12.1) |
| overlap pairs / defects | 1 / 1 | **0 / 0** |
| drawn crossings | 48 | 49 (the pin's channel over the exit's leg; must 74.6) |
| `fan_x` | 0 | 0 |
| six shuffles: crossings | 41 58 41 41 48 44 | 42 58 42 42 49 45 |
| six shuffles: overlap pairs | 1 0 1 1 1 1 | **0 0 0 0 0 0** |
| six shuffles: `fan_x` | 0 10 0 0 0 3 | 0 10 0 0 0 3 |

Every other repository fixture is unchanged (harness `B/work/r9/h/head-*` against `p4-*`); on the local family only v2
changed (`lateral_x` 2 -> 1). Two narrower first versions were measured and dropped: moving every pin closer than a
port's spacing cost `blocks/nested-cases` a crossing (13 px gap), and reading the whole side the new way moved the
analog family's pins (`fan_x` 0 -> 1). The suite-on `quality_gate` no longer has a static dense-td row.

## 3. dense-td shuffles s2 (`fan_x` 10) and s6 (3): open, not a gate row

s2, traced: after the ortho stage `n36 -> n16`'s exit leg runs up at 132.1 from `g8`'s title side to its step at 1499.6,
and `n11 -> n37`'s straightened channel column comes down at 136.4 from 1349 to 1738: 4 px apart over 240 px. In s2's
edge order `n11 -> n37` (edge 8) is straightened before `n36 -> n16` (edge 40) is drawn, so `ColumnRoom`'s `runs` fence
does not see the exit leg (in the default order it does); the router then reroutes `n36 -> n16` across the hub `n16`'s
fan. The fix is a fence for the port runs every edge will take, known before the columns are straightened (node-side
end order and the even band, B12.11, with G1's breadth room): not built. s6: three crossings at the top of `n16`'s
bottom side (the same family). Both stay inside the crossing must (58 and 45 against 74.6) and the width rule.

## 4. Bulk corpus (the verifier's 2,169 inputs; `B/work/r9/bulk/`, start = the verifier's blocks-off run)

1,077 inputs in block mode. Static, block-mode inputs only:

| | start | round 5 (verifier) | now |
|---|---|---|---|
| hard zeros | 568 | 247 | **247** |
| shared ports / arrow on start | 229 / 79 | 139 / 79 | 139 / 79 |
| `frame_side` | 196 | 5 | 5 |
| node in a foreign frame | 7 | 0 | 0 |
| own-frame runs | 21 | 5 | 5 |
| edge / foreign-frame pairs | 535 | 0 | 0 |
| drawn crossings | 4,821 | 2,676 | 2,680 |
| fan crossings | 264 | 214 | 214 |
| overlap pairs | 140 | 104 | **99** |

This round changes 6 rows: five dense-td-like files lose their overlap pair (one crossing more each), one loses a
crossing; no file gains a hard zero. **48 files** draw more hard zeros than the start (the same 48 as round 5),
classified from the added detail lines (`B/work/r9/worse-on.txt` against `worse-off.txt`):

- 40: pairs of note-card links on one point of their state (lead note 6, C);
- 1 (`affa3036…`): a note card's link to a composite on a lateral side (`__note_17->K1 frame_side`; note links, C);
- 3: own-box edges through a member or a note card (`K0->K0_1 edge_node __note_17`, `C0->C0i0 edge_node __note_4`,
  `C0->C0i2 edge_node C0i0`; B12.14, C);
- 1: a member's edge to its own composite run along its frame (`C0i2->C0 own_frame C0`; B12.14, C);
- 5 with a shared port of two non-note edges: `0b968b60…` (`port C0i0 C0ch C0ch`), `1ab4c56b…` (`port K2_0 K2_f
  K2_f`), `eeab8cb8…`, `fac0ed90…`, `cdf50a24…`. With the note lines removed from the source, the first four draw no
  shared port at all (`B/work/r9/nonote3/`): their notes are members of the row below and their links take the same
  side of the state, which is then over-full and packs the twins together (picture `B/work/r9/pic/tw1b.png`); their
  cause is the note links' pins (B12.14, C). **`cdf50a24…` stays without notes** (`port C0ch C0i0 C0i0`): a choice
  placed beside its state in a two-cycle takes the entry and the `y` exit on its upper-left facet 2 px apart (picture
  `B/work/r9/pic/cdf.png`). That is the choice-corner geometry of 2.8's spill order (B12.11), the family of
  `notes/nested-composite`'s choice target, present in round 5 too; not built for this case. Reported, not fixed.

## 5. Smaller block-mode changes, stress inputs (static, start -> now)

- `blocks-overview-bt` drawn crossings 7 (must 7; round 4: 6) — unchanged this round.
- `x4-fan-levels` `lateral_x` 3 (the start's own 3), `fan_x` 0, crossings 0: `hub -> d1` and `hub -> c2` go round the
  left and in through `Delta`'s and `Gamma`'s lateral walls, `c2 -> hub` leaves `Gamma`'s right wall (picture
  `B/work/r9/pic/x4.png`). The tops of `d1` and `c2` hold their blocks' own entries; side ends of a fan into stacked
  blocks are B12.11's even band and corner legs. Not built.
- `y4-hub-blocks` crossings 32 -> 13 (round 4: 11), overlap 1 -> 0, `fan_x` 5 -> 0.
- `y8-huge-channel` (an L4 lane level, 60 edges between two blocks): crossings 62 -> 69, overlap pairs 2 -> 4 against the
  start, the same as round 5. B12.15's "no level L4 dissolves drawn worse than the starting release" is due at C;
  noted for C.
- x1-title-row, blocks/title-entry, title-hidden: as round 5 (6.3 of the round-5 report: X2.2 frame breadth, A2).

## 6. Suite-on tests, default run, perf

Suite-on `cargo test --no-fail-fast` (`B/work/r9/ton-full.txt`): lib **8 failed**, the same eight as round 5 (comb
route half R; `far_apart_back_edges_and_frame_legs_square_up_alike_in_route` R; `a_nearly_straight_pair_is_drawn_straight`
and `advanced_showcase_parses_and_renders` X3, A2; own-state stub C; lane in `route()` R; two note-link tests C/R).
`tests/quality.rs` **7 failed**, the same names as round 5: `quality_gate` (now **routed rows only**: bench-75 and
dense-td routed, R — the static dense-td GAP_FLOORS row of round 5 is gone), `block_fixtures_are_not_drawn_worse`
(static: notes/wide-frames only, C; the rest routed), `defect_baseline_gate` (static DEFECT_BASELINE, A2),
`channels_keep_edges_apart_and_off_chips` (static `edge_chip` 3 on dense-td, A2 labels), `crossing_gaps_do_not_read_as_dashes`
(routed), `edges_keep_off_frame_borders` (routed `border_len`), `a_one_px_drag_leaves_unrelated_edges_put` (drag, R).

Default build: guard `GUARD: ALL GREEN` before each commit (`guard1.txt`, `guard2.txt`), off-identity test green (the
changes sit in `free_pins`, which runs in a block layout only, and in tests). `tools/perf.sh` twice
(`runs/b12a1-structure-r7`, `-r7b`): bench-75 worst drag 19.31 / 17.56 (limit 25.28), route_ms 14.66 / 13.17 (15.76);
dense-td drag 15.25 / 14.31 (17.80), route 6.85 / 6.84 (9.20); dense-lr drag 10.50 / 13.51 (17.76), route 6.56 / 5.22
(8.03); fanout-frames drag 3.72 in the first run (limit 3.44) and 2.45 in the second: the box (load 1.5-2.2; the default
build's code is unchanged); every other file inside its limit in both. `route_partial` not touched. Block-mode static
scene_ms, fastest of three: analog 10.70 (limit 11.58), bench-75 23.98 (61.94), dense-td 29.45 (50.68), dense-lr 12.59
(46.8), fanout-frames 2.10 (4.38).

Local acceptance (`B/local/out/r9-p4/judge-a1.txt`): **155 checks on 92 diagrams, 0 must-fail lines, 7 targets missed**
(crossings targets only); crossings analog 15, v2 14, perms 15 18 15 15 17, public fixture 6 (bt 7), bench-75 204,
dense-lr 17, dense-td 49; `xframe_runs` 0 everywhere; breadths as round 5.

## 7. Rows of 4.3 (a)-(c) due at A1 (static), unchanged from round 5 but dense-td

| input | xframe_runs | crossings / must | fan_x | canvas breadth |
|---|---|---|---|---|
| analog | 0 | 15 / 20 | 0 | 3130 |
| v2 | 0 | 14 / 17 | 0 | 3119 |
| perm-a, perm-c, perm-d | 0 | 15 / 20 | 0 | 3130 |
| perm-b | 0 | 18 / 23 | 0 | 3153 |
| perm-e | 0 | 17 / 22 | 0 | 3180 |
| public fixture (all permutations) | 0 | 6 / 7 (bt: 7 / 7) | 0 | 1142 |
| bench-75 | 0 | 204 / 325 | 4 (start 6) | |
| dense-lr | 0 | 17 / 51.5 | 0 | |
| dense-td | 0 | 48 -> **49** / 74.6 | 0 (start 1) | |

Starts table and the X2.5 / X2.2 work of round 5: as in `checkpoint-A1.round5.md` sections 3 and 5 (no ordering or
layering change this round). What `b12a2-geometry` no longer builds: as round 5's section 7, plus the free-pin rule of
section 2 here (a pin on top of a member's exit takes the gap past it). Still to build there: the even band of free
pins and node-side end order (sections 3 and 5), X2.2's frame breadth, F2 in LR/RL, F3, the choice corner of section 4.

## 8. Disk

12 GB free. Kept for the verifier: `B/target-on`, `B/target-ont`, `B/target-dbg`, `B/work/r9/probe/target` (34 MB,
probe `dump` and `bulkm`), `B/work/r9/headtarget` (36 MB, round-5 suite-on build for head | now pictures).
