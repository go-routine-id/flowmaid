# Checkpoint A1: structure (B12.0-B12.6), after fix round 4

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`, branch
`ccr-ccc6d89a-5cxd7e`. This replaces the report of fix round 3. The lead's notes of SPEC section 0 apply ("After
fix round 1", "After verify#2"). Everything below ran on `f658a9b` (round 4: `33f0178`, `f658a9b` on top of
`649fee2`; `0ef304b` after it changes one doc comment only, guard green on it).

**Gate: NOT met.** Per decision D6 nothing was re-baselined or loosened, and the package stops here as the lead's
note "After verify#2", item 3, says: this was the fourth fix round. What round 4 did on the lead's item 2 (the
pulled-forward geometry for `xframe_runs` and `fan_x`), with numbers, is section 3; what is left open, each with its
cause per fixture, is section 6. In short:

- `xframe_runs`: **0 on every input** (bench-75 2 -> 0, notes/fork-twins 1 -> 0). The local judge: **0 must-fail
  lines** (round 3: 3).
- `fan_x`: state-note 1 -> 0 and the verifier's three-producer stress diagram 1 -> 0; bench-75 21 -> 18 (start 6).
  Still open: dense-td 18 (start 1), bench-75 18 (start 6), v2 1 (must 0), notes/fork-twins 2 (start 0).
- Static hard zeros, nested-chain's `lateral_x`, the sweep's `SWEEP_ADDED_MAX`: unchanged since round 3; their causes
  per fixture and per generated diagram are in section 6 (note links, own-box edges, a choice's over-full side,
  free pins on a lateral side, a title-clear entry).

| finding of verify round 3 | state after round 4 |
|---|---|
| #1 A1 gate | **partly**: `xframe_runs` met (both cases fixed, `33f0178`); `fan_x` 2 of 6 cases fixed (`f658a9b`), 4 open (6.2); hard zeros, `lateral_x` of nested-chain and the sweep open with causes (6.3-6.5). The lead's item 2 was attempted this round: section 3 |
| #2 numbers of the local family in `f1879b1`'s message (D5) | cannot be amended in a fix round; the replacement text for `f1879b1` and for `32a3ebf` (which has the same fault: "fan_x 1 -> 0 ... on six local acceptance inputs") is in `B/reports/reword-before-merge.md`, for the lead's tidy-up before the merge. Round 4's messages cite the public fixture, the dense and notes fixtures only |
| #3 `BLOCK_CEILINGS` row of blocks/turned-blocks | **fixed** (`33f0178`): (0,0,0,4,0) / (0,0,0,5,0), the start's own values (start binary: x_drawn 4 static, 5 routed) |
| #4, #5 lane corridor trade-offs (bulk 5a10594f, 4d55b170) | not changed: the corridor keeps a state of the outer composite out of the nested composite's frame (`node_frame`, a hard zero) at the cost of a crossing or two; undoing it brings the hard zero back. A lane that keeps out of what stands beside its block *and* off the fork bar's outputs needs the lane plan to see the corridor's other obstacles (B12.9 gap room); named for b12a2 |
| #6 blocks/lane-beside, `Gate`'s lane exits beside the entry | not changed: both lanes leave right of the entry, as the corridor puts them on the side away from `Fix`; the start and round 2 split them only because the right side was not yet bounded. The lane ends on a 28 px choice: the spill order of 2.8 (B12.11) |
| #7 three-producer stress diagram, crossing 0 -> 1 | **fixed** (`f658a9b`): crossings 1 -> 0, `fan_x` 1 -> 0 (pins in channel order) |
| #8 `strip_is_free` (pins on the growing side; segments between samples) | **fixed** (`33f0178`): segments tested against the added strips; a pin of the frame itself on a side that grows keeps the frame where it is. Unit test. No output change on the fixtures or the bulk |
| #9 `embrace_own_edges` only in the static scene | not changed: `route()` is R's (B12.10 / B12.13) |
| #10 generated g220, edge 23 (`dn3 -> bxn2`) diagonal waypoint step | not changed; flagged here for b12a2/b12a3 (PLAN invariant 2: a host's B-spline through the waypoints does not reproduce that step) |

## 1. Commits, command lines, judge output

- Round 4, on top of `649fee2`: `33f0178` (the whole-drawing strict try of the repair; `strip_is_free`; the
  turned-blocks ceiling), `f658a9b` (free pins in channel order).
- Suite-on build: `RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=B/target-on CARGO_INCREMENTAL=0 cargo build
  --release --bin flowmaid --example quality` (`B/work/r6/build.sh`); tests `B/work/r6/ton.sh --no-fail-fast`
  (`CARGO_TARGET_DIR=B/target-ont`).
- Local acceptance, static: `B/work/r6/on.sh r7-pins3` (`run.sh fm`, then `run.sh judge out/r7-pins3 --due A1`).
  Copies: `B/local/out/r7/{judge-a1.txt, metrics.tsv, local-harness.txt, suite-on-tests.txt, failed.txt,
  timing.txt}`. Result: **155 checks, 0 must-fail lines, 8 targets missed** (round 3: 3 must-fail lines).
- The judge's `bands` row of perm-c is accepted as in rounds 2 and 3 (same pairs, U2~U1).
- Bulk: the verifier's 2,169 inputs (1,077 in block mode) with the verifier's probe (`bulkm.rs`) built against
  this tree: `B/work/r6/bulk/now4.tsv`, compared with `B/work/r6/bulkcmp.py` against the verifier's round-3 table.

## 2. The rows of 4.3 (a)-(c) due at A1 (static)

| input | xframe_runs (0) | crossings / must | fan_x (0; harness) | canvas breadth | bands |
|---|---|---|---|---|---|
| analog | 0 | 15 / 20 | 0 | 3157 | U1 T1 U2 / EXT GH / PROD / DR STG |
| v2 | 0 | 15 / 17 | **1** | 3119 | U1 T1 U2 / GH EXT / DR / PROD / STG |
| perm-a, perm-c, perm-d | 0 | 15 / 20 | 0 | 3157 | as the analog (perm-c accepted) |
| perm-b | 0 | 22 / 23 | 0 | 3114 | as the analog |
| perm-e | 0 | 17 / 22 | 0 | 3233 | as the analog |
| public fixture, 4 permutations | 0 | 6 / 7 | 0 | 1142 on all five | OPS SHOP / CI EDGE / WEB / ORD / DATA / OBS |
| hub5 | 0 | 0 | 0 | | |
| bench-75 | **0** (was 2) | 240 / 325 | **18** (start 6; was 21) | | |
| dense-lr | 0 | 17 / 51.5 | 0 (start 2) | | |
| dense-td | 0 | 68 / 74.6 | **18** (start 1) | | |

Crossings, breadths and bands of the analog family and the public fixture are those of round 3 (round 4 changes
no layout). Harness total `x_drawn` static over `tests/fixtures/quality`: **325** (start 405; round 3: 333).

## 3. What the round built and tried (the lead's item 2)

Built (both behind the block layout; the default build is byte-identical, `everything_renders_as_before_with_blocks_off`):

1. **A whole-drawing strict try in the repair** (`router::repair`, `33f0178`). An edge the repair searches tries,
   after the priced search over the whole drawing and only when that search's best path enters a foreign frame and
   would replace the original (one through a node), once more clear of every node, band and foreign frame, over the
   whole drawing. This is the "corner leg" case of 2.8 met from the other end: bench-75's `gw -> logs` has both ends
   spilled round the corner of an over-full side (gw's right side: seven border ends; logs's left side: eight), the
   channel's last leg climbed into `logs` from below through `metrics`, the window round the ends held no clear way
   in, and the priced search ran the edge through `app` and `data`. Now it goes round them (picture
   `B/work/r6/pic/r4-bench-75.png`, start | round 3 | now). It also takes fork-twins' note link `__note_17 -> K0_0`
   out of `K0N` (x_drawn 13 -> 8). Bulk, static, block mode: `edge_frame_pairs` 10 -> 3, crossings 3169 -> 3136,
   no input worse in crossings or hard zeros.
   - Tried first and dropped: the same try for every edge the window found no clear way for. It also took the
     originals that only touch a title chip on long ways round: dense-td fan_x 18 -> 12, but on the bulk +12 to +16
     crossings on 14 inputs (`B/work/r6/bulk/now.tsv`), and nothing gained in `edge_frame_pairs` there.
2. **Free pins in channel order** (`ports::order_pins`, `f658a9b`; 2.8 "Free pins", the order part). The pins of
   one frame side that stand next to each other take their places in the order their channels come in; only the
   pins' own places are exchanged, nothing else on the side is passed; a run's new order is kept when the builders'
   lines of the run cross each other less; note links never move. state-note `fan_x` 1 -> 0, `x_drawn` 1 -> 0
   (`B/work/r6/pic/r4-state-note.png`), the verifier's three-producer diagram 1 -> 0 (`r4-y3.png`). Bulk: crossings
   -15 (fewer on 12, more on none), `fan_x` -13 (fewer on 10, more on none).

Tried and taken out (numbers from the suite-on build, static):

3. **Sibling crossings priced ahead of the other crossings** (a `Score` field counting crossings with accepted
   edges that share an end, between `near` and `cross`, in the repair): no change on any fixture (the repair keeps
   or replaces an edge on its hard and overlap fields first; the A* search never saw the field).
4. **The same in the lattice** (a crossing with a sibling's segment priced at 4x and at 2x `CROSS_COST`): 4x:
   bench-75 fan_x 18 -> 13 but overlap_pairs 6 -> 13, dense-td fan_x 12 -> 13 and **crossings 69 -> 78 (above its
   must, 74.6)**; 2x: bench-75 fan_x 22, dense-td fan_x 16, crossings 73. Not kept.
5. **A fan untangle pass** after the comb: each crossing pair of one fan has its two ports exchanged and both edges
   searched again over the whole drawing (both orders), kept when the fans' crossings fall and nothing else gets
   worse. dense-td fan_x 12 -> 11, bench-75 18 -> 17-18, v2 unchanged (its pair's best exchange crosses twice);
   static `scene_ms` dense-td 84 and bench-75 67 (limits 50.68 and 61.94). Not kept.

Not built, and why:

- **The track packing of a gap (2.6 G2).** dense-td's open fan crossings (6.2) come from the router's repairs of
  edges the ortho stage nested correctly (fan crossings after each stage on dense-td: ortho 7, repair 12, respace
  12, nudge 22, spread 22, comb 18; on bench-75: ortho 5, repair 31, comb 21 at round 3, 18 now). Packing a gap's tracks in
  the layout gives the fan and the turned edges that climb through it each a track, so the repair has nothing to
  search; it is the layout's G1/G2 room (B12.9), not a smallest form: every attempt above at keeping the repair
  from tangling a fan traded the fan's crossings for crossings elsewhere or for the time budget.
- **Title-clear entries (2.8 rules 1-3, X2.5).** v2's one fan crossing (`DNS -> SITE` against `DNS -> PSDEV`) and
  nested-chain's two lateral crossings both need room the layout reserves: a column beside the outermost member past
  the chip (X2.5) or jog room inside the frame (F2). With the coordinates of today the entry has no free column
  beside the chip (nested-chain: 14 px between `c` and the frame's border), and the router's way in round the chip
  is what is drawn now. That is B12.8 (X2.5) with B12.7 (F2), not a ports or router rule.

What `b12a2-geometry` therefore no longer builds: the order of free pins on a frame side (B12.11 "Free pins" builds
the even band, the corners' rule and pins in `route()`). The whole-drawing strict try is a router safety net, not a
spec item; B12.11's corner legs would make it fire less often.

## 4. Suite-on tests, default run, perf

Default run on `f658a9b`: `tools/guard.sh` prints `GUARD: ALL GREEN` (`B/work/r6/guard4.txt`);
`everything_renders_as_before_with_blocks_off` passes.

Suite-on `cargo test --no-fail-fast` on `f658a9b` (`B/local/out/r7/failed.txt`): lib 1140 passed, **10 failed**;
`tests/quality.rs` 9 passed, **6 failed**. The same list as rounds 2 and 3, none new; the round's three new tests pass:

| still failing | class | cause |
|---|---|---|
| `casing::a_line_takes_one_role_across_a_cluster_of_crossings`, `casing::a_short_line_keeps_at_most_one_gap`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route` | 4.5 intended: re-read | fixture-specific numbers on the re-laid dense fixtures |
| `comb::a_hubs_fan_is_a_comb_in_scene_and_route` | rule | dense-td's hub fan `n16`: 12 crossings inside it in the scene (section 6.2) |
| `ports::a_nearly_straight_pair_is_drawn_straight`, `render::advanced_showcase_parses_and_renders` | must pass | `examples/advanced.mmd`: no straight spine across a border without X1-X3 (B12.8, "not built yet: X" of A1's row) |
| `scene::a_composite_edge_to_its_own_state_drops_straight_from_the_border` | must pass | own-box edge (2.8, B12.14; the lead's note 6: due at C) |
| `scene::a_lane_stays_in_its_frames_in_route_and_on_a_drag_too` | must pass | `route()` at block positions (R1/F3, B12.13; due at R) |
| `scene::note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `scene::note_links_match_between_scene_and_route` | must pass | note links in block mode (B12.14 due at C; their routed half at R) |
| `quality.rs`: `block_fixtures_are_not_drawn_worse` | gate | static: `examples/advanced` overlap 1 (6.4), `notes/wide-frames` (6.3); routed lines (R). **`notes/fork-twins` static is gone from it** |
| `quality.rs`: `quality_gate`, `defect_baseline_gate`, `channels_keep_edges_apart_and_off_chips`, `crossing_gaps_do_not_read_as_dashes`, `a_one_px_drag_leaves_unrelated_edges_put` | gates | static halves due at A2, routed halves and drag jumps at R |

`scene_ms` with the switch on (`B/local/out/r7/timing.txt`, start | round 3 | now interleaved, 4 rounds, fastest
of 5 calls each; the box was bimodal again): analog min 9.2 (round 3 10.1; limit 11.58), public fixture 7.9
(round 3 8.7; limit 13.6), bench-75 43.7 (43.5; limit 61.94), dense-td 37.4 (38.3; limit 50.68; one slow run 54.8,
round 3 had two of 50.5 and 51.0), dense-lr 13.8 (13.8; limit 46.8). Inside the limits on the runs that agree; the
round adds no measurable time (the strict try runs only for an edge whose priced path crosses a foreign frame and
would be taken; the pin order is one pass over the frame sides).

Default build `tools/perf.sh` (`runs/b12a1-structure-r4`, `-r4b`; the default build draws what the start draws):
bench-75 worst drag 19.66 / 18.95 (limit 25.28), route_ms 11.10 / 9.24 (15.76); dense-td drag 15.84 / 12.23
(17.80), route_ms 8.43 / 9.35 (9.20); dense-lr drag 10.20 / 10.98 (17.76). dense-td's route_ms of 9.35 on the second
run is the box's: the start binary measured 9.82 and 6.54 in the same session, this build 8.81 and 6.63
(interleaved). `route_partial` is not touched by the round (static scene only).

## 5. Starts table, shuffles, generated sweep

Ordering and layering are untouched since round 2; the table crossings x starts of round 2
(`B/local/out/r5/starts-table.txt`) stands. dense-td under the six shuffles: not re-run (no layout change).

Generated sweep (200 diagrams in block mode): no panic, deterministic, block rows and one-side constraints hold;
the lane rule keeps a level side by side on 52 diagrams (list in the test's output). Hard zeros: 49 on 30 diagrams as blocks,
115 on 55 before blocks. By kind (`B/work/r6/sweep-details.txt`, from a scratch test over the same sweep):

| kind | diagrams | what draws it |
|---|---|---|
| `frame_side` | 21 | an edge to a whole block ends on a lateral side of its frame where the block and the edge's other end overlap along the flow (flow 2 BT: `B0 --> b4n0`, `B0` spans the layers of `B4`); 2.8 allows the lateral side there ("A lateral side is otherwise allowed only when the two units overlap along the flow"); the harness counts any end across the flow. 20 of them draw the same before blocks |
| shared port + arrow on start | 8 (state) | two note links of one state on one side (B12.14) |
| `edge_node_x` | 3 (flow 12, 107, 112) | each drew it before blocks too |
| `own_frame_x` | 1 (flow 122) | an edge from a subgraph to its own member (own-box, B12.14) |

Blocks add hard zeros on 9 (`SWEEP_ADDED_MAX` 9 stands): the 8 note-link state diagrams and `flow 23 RL` (a
`frame_side` pin, `B5 -> b7n0`, beside its block).

## 6. Every must that is open, with its cause

### 6.1 Runs through a foreign frame

None (all inputs 0; the new test `an_edge_the_repair_searches_goes_round_the_blocks_between` asserts
`edge_frame_pairs` 0 on every block-mode repository fixture).

### 6.2 Fan crossings (harness `fan_x`, static)

- **dense-td 18** (start 1): `n16 -> n37` against four siblings on `n16`'s bottom side, twice each (8), and against
  the turned edge `n36 -> n16` (3), `n16 -> n37` against `n20 -> n37` on `n37`'s top (3), `n16 -> n31` against
  `n31 -> n16` (counted on both fans, 2), `n31 -> n38` against `n25 -> n38`, `n28 -> n42` against `n25 -> n42`.
  Cause (measured stage by stage):
  the ortho stage nests the fan (7 fan crossings after it, 1 of them with the turned edge `n36 -> n16` that climbs
  through the fan); the repair searches `n16 -> n37` because it crosses that turned sibling twice and keeps a path
  80 px on top of another edge; the nudge then trades the run for crossings (`Score` ranks `overlap` before
  `cross`: 80 px / 6 -> 0 px / 16). The track packing of a gap (G2, B12.9) gives the fan and the climbing edge a
  track each; section 3 lists the four router-side attempts and what each cost.
- **bench-75 18** (start 6): `svc1`'s bottom fan to `queue`, `db`, `cache` (6), `svc5`'s (4), `logs` bottom and
  left (4), `svc1` right (2), `svc4` top (1), `metrics` top (1). Cause: the data block is one column of four
  members whose left sides are over-full (six to nine ends each), so ends spill to their top and bottom sides, and
  the ortho stage reaches a spilled end through the members stacked above or below it (alone-scores at the repair:
  `svc1 -> cache` through 3 nodes, `svc5 -> cache` 3, `svc6 -> queue` 3, `svc1 -> db` 2); the repair reroutes them
  and the fan order is lost. The spill order and corner legs of 2.8 (B12.11) with the gap room of G1/G2.
- **v2 1**: `DNS -> SITE` against `DNS -> PSDEV` on `DNS`'s bottom side. `SITE`'s whole top side lies under its
  frame's three-line title chip; the router enters the frame right of the chip and jogs under it to `SITE` at the
  depth of the chip's last line, from the port right of `DNS -> PSDEV`'s. Nested, `DNS -> SITE` would take the left
  port and the upper track and cross the border between the chip and `PSDEV`'s column. Title-clear entries with
  their jog room (2.8 rule 2, F2, B12.7/B12.11); the comb cannot reach a tooth with two steps.
- **notes/fork-twins 2** (start 0): `__note_12 -> K0NN_0` against `__note_13 -> K0NN_0` (two note links on one
  side: B12.14) and `K1_1 -> [*]` against `K1_1 -> K1_2` on `K1_1`'s right side beside the card `in270` (a side
  shared with a note card's link: the note-link builder's elbow is a pin, B12.14).

### 6.3 Static hard zeros, per fixture

| fixture | now | start | offending paths, what draws them |
|---|---|---|---|
| `notes/wide-frames` | 2 | 0 | `__note_12 -> K0N_1` and `K0N_1 -> __note_13` end on one point of `K0N_1`'s right side (`shared_ports`, `arrow_on_start`). In block mode both cards lie right of `K0N_1` (at the start one lay left); a note card's link end is a pin (`ports::NOTE_WEIGHT`), so two pins on one side are never spaced. B12.14 (note links in block mode) |
| `notes/fork-twins` | 1 | 3 | `__note_12`, `__note_13` share a port on `K0NN_0` (the same, B12.14) |
| `notes/nested-composite` | 2 | 4 | not a note link: the 28 px choice `K0_c` takes the entry `K0NN_1 -> K0_c` from above at its top-left facet, 2 px from where `K0_c -> K0NN_0` leaves (the side's three ends on a diamond side): the spill order of 2.8 on an over-full small side (B12.11) |
| `blocks/own-centre` | 1 | 1 | `S --> y2` through `y3`: own-box stub (2.8, B12.14) |
| `blocks/self-member` | 2 | 2 | `S --> b` through `c`, `c --> S` 146 px outside `S`: own-box edges (B12.14) |
| `blocks/sub-nested-self` | 2 | 3 | `OUT --> IN`, `IN --> OUT` through `o2`: own-box stubs between nested borders (B12.14) |
| `blocks/nested-chain` | 0 | 0 | (its `lateral_x` 2: section 6.5) |

### 6.4 `examples/advanced` static overlap (in `block_fixtures_are_not_drawn_worse`)

Unchanged since round 3: `Gate -> Cache`'s step lies 6 px beside `Gate -> Auth`'s for 70 px after the nudge; the
gap also carries `Cache -> Gate`'s lane: the steps of one fan in a gap do not keep `TRACK_PITCH` (G2, B12.9).

### 6.5 B12.4's check on nested-chain: 0 lateral crossings

`a -> c2` leaves `Inner source` and enters `Inner target` through their left walls (`lateral_x` 2). The chain's
column inside `Inner target` lies under that block's title chip (the dummy beside `c` on `c`'s layer); the router
takes the edge round the chip, out through the walls. X2.5 puts such an inner run in a column outside the chip's
interval (beside the outermost member past the chip's end), which the coordinates of today do not reserve: 14 px
between `c` and the frame's right border. B12.8 (X2.5).

### 6.6 Bulk corpus (the verifier's 2,169 inputs, 1,077 in block mode; `B/work/r6/bulk/now4.tsv`)

Static, start | round 3 | now: hard zeros 568 | 294 | **294**, `edge_frame_pairs` 535 | 10 | **3**, defects
1156 | 346 | **329**, crossings 4821 | 3169 | **3121**, `fan_x` 264 | 458 | **441**, overlaps 140 | 146 | **136**.
Against round 3: crossings fewer on 16 inputs and more on none; `fan_x` and overlaps more on one input each
(`74eeda72`: +2 / +1, with 12 crossings and 2 frame runs fewer). Routed rows unchanged (route() in block mode is R's).

## 7. What the open musts wait for

Every open must of A1's row is, per fixture, one of: the track packing of a gap and the gap room of G1/G2 (dense-td's
and bench-75's fans, advanced's overlap), the spill order and corner legs of 2.8 (bench-75's spilled fans,
nested-composite's choice), title-clear entries with their jog room and X2.5's column (v2's fan crossing,
nested-chain's lateral walls), or note links and own-box edges (fork-twins, wide-frames, the own-box fixtures, eight
generated state diagrams); and a harness count stricter than 2.8 on lateral pins of blocks that overlap their other
end along the flow (21 generated diagrams). Each is a `b12a2-geometry` (B12.7-B12.11) or `b12b-default` (B12.14)
item. Per the lead's note "After verify#2", item 3, the package stops here, and the question whether those rows are
judged at A2 goes to the user with these numbers.

Disk: the scratch target directories of this round (`B/target-ont`, `B/target-dbg`, `B/target-prev`,
`B/target-sweep`, `B/work/r6/t-probe`) and the scratch trees (`B/work/r6/{dbgtree, prevtree, sweeptree,
bulktree}`) are deleted after this report is written; `B/target-on` is kept for the verifier's first look.
