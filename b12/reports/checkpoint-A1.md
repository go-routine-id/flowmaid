# Checkpoint A1: structure (B12.0-B12.6), after fix round 2

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`, branch
`ccr-ccc6d89a-5cxd7e`. This replaces the report of fix round 1 (2026-11-16 ~04:00); the lead's notes of SPEC
section 0 ("After fix round 1") apply. Everything below ran on `08c6408` (library code as of `fbd943d`).

**Gate: NOT met.** Per decision D6 nothing was re-baselined or loosened; the round stopped where the open musts need
the title-clear entries with their jog room (2.7 F2, 2.8), which could not be built in a form that leaves the
crossing musts standing (section 7). Open at A1: `xframe_runs` on bench-75 (2), dense-td (1) and `notes/fork-twins`
(1); `fan_x` on one local input (1) and above the start on dense-td, bench-75 and fork-twins; static hard zeros on
`blocks/nested-chain` and four notes fixtures; ten suite-on tests (none new, two fewer).

What this round changed (static, switch on):

| finding of verify round 1 | state |
|---|---|
| #3 `comb::combing_is_settled_after_one_pass` (this package's regression) | **fixed**, cause found (section 3) |
| #4 a transition through a foreign composite (`s4-state`) | **fixed**: `xframe_runs` 1 -> 0, `edge_frame_pairs` 1 -> 0 |
| #1 `xframe_runs`: dense-td `n41 -> n42` (a direct link is a run) | **fixed**: dense-td 2 -> 1 |
| #1 / #6 `fan_x` on the local family | **0 on six of seven** (was 1 on six), v2 still 1 |
| #2 `blocks/nested-chain` | **open**: `own_frame_x` 1; the test's comment now says what it asserts (section 6.4) |
| the judge's `bands` check (lead's decision 1) | **corrected** in the workspace; perm-c accepted, pairs printed |
| `router::edges_into_a_wedged_frame_keep_out_of_the_gap` (lead's decision 4) | **re-read**, with a block-mode twin |
| #1 the rest (`xframe_runs` bench-75, dense-td `n46 -> g9`, fork-twins; `fan_x` dense; hard zeros) | open, causes in section 6 |
| #5, #6 over the bulk corpus | re-measured on the state diagrams only (section 6.6) |
| #7-#11 | not blocking; numbers in sections 4 and 5 |

## 1. Commits, command lines, judge output

- Start `3abbfeb`. Package and round 1: see the previous report. Round 2, all on top of `8c6dc26`:
  `8634785` comb (a block layout's side is dealt again after its ports were evened), `4f638ea` a turned edge of a
  whole block runs up its channel, `32a3ebf` free pins stand clear of what else crosses their frame side, `fbd943d`
  one side per (link, block), `08c6408` tests (wedged frame, nested-chain's comment).
- Suite-on build: `RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=B/target-on CARGO_INCREMENTAL=0 cargo build
  --release --bin flowmaid --example quality` (`B/work/r3/on.sh`); tests: `B/work/r4/ton.sh --no-fail-fast` (the same
  flags, `CARGO_TARGET_DIR=B/target-ont`, `CARGO_PROFILE_DEV_DEBUG=0`).
- Disk: the round's scratch target directories (`B/target-on`, `B/target-ont`, `B/target-dbg`, `B/target-dbgt`,
  `B/target-startt`, about 730 MB) were deleted after this report was written; the scratch trees `B/work/r4/dbgtree`
  (debug prints, never in the repository) and `B/work/r4/starttree` (`git archive 3abbfeb`) are sources only.
- Local acceptance, static: `cd B/local && ./run.sh fm B/target-on/release/flowmaid out/r4h b12-r4h && ./run.sh judge
  out/r4h --due A1`. Copies for this report: `B/local/out/r5/{judge-a1.txt, metrics.tsv, harness.tsv,
  starts-table.txt, suite-on-tests.txt, failed.txt}`. Result: **155 checks, 5 must-fail lines (3 distinct), 8 targets
  missed**; 109 rules due later left out.
- The judge (`B/metrics/judge`, the version before this round kept as `judge.before-r2`): `same_as` on the `bands`
  column now also accepts two rows whose bands hold the same units in the same order along the flow and differ, inside
  a band, only by units with identical links in their structure files (`--struct-dir`, passed by `run.sh judge`); the
  row is printed as `accepted` with both strings and the pairs. Selftest: 32 checks pass (5 new).

Judge, must-fails and the accepted row (static, `--due A1`):

```
accepted   perm-c  bands  U2 T1 U1 / EXT GH / PROD / DR STG  == analog: U1 T1 U2 / EXT GH / PROD / DR STG
           up to crossing-neutral units: U2~U1 (both: out DNS); U1~U2 (both: out DNS)
MUST FAIL  bench-75    xframe_runs 2  must <= 0   [0.38 4, start 4, mermaid 0]      (two rules name it)
MUST FAIL  dense-td    xframe_runs 1  must <= 0   [0.38 12, start 12, mermaid 0]    (two rules name it)
MUST FAIL  fork-twins  xframe_runs 1  must <= 0   [0.38 0, start 0, mermaid 0]
```

Why the pair is crossing-neutral: the two actors have one link each, to the same node, so exchanging them maps the
drawing onto itself; `T1`, which also has other links, stands in the same place in both rows.

## 2. The rows of 4.3 (a)-(c) due at A1 (static)

| input | xframe_runs (0) | sib_side_linked (0) | crossings / must | fan_x (0) | canvas breadth | bands |
|---|---|---|---|---|---|---|
| analog | 0 | 0 | **15** / 20 | **0** | 3157 | U1 T1 U2 / EXT GH / PROD / DR STG |
| v2 | 0 | 0 | 15 / 17 | **1** | 3119 | U1 T1 U2 / GH EXT / DR / PROD / STG |
| perm-a, perm-c, perm-d | 0 | 0 | 15 / 20 | 0 | 3157 | as the analog (perm-c: accepted, section 1) |
| perm-b | 0 | 0 | 22 / 23 | 0 | 3114 (-1.4 %) | as the analog |
| perm-e | 0 | 0 | 17 / 22 | 0 | 3233 (+2.4 %) | as the analog |
| public fixture, 4 permutations | 0 | 0 | 6 / 7 | 0 | 1142 on all five | OPS SHOP / CI EDGE / WEB / ORD / DATA / OBS on all five |
| hub5 | 0 | 0 | 0 | 0 | | |
| bench-75 | **2** | 0 | 247 / 325 | 21 (start 6) | | |
| dense-lr | 0 | 0 | 17 / 51.5 | 0 (start 2) | | |
| dense-td | **1** | 0 | 68 / 74.6 | 18 (start 1) | | |

- The analog draws 15 crossings now, one fewer than mermaid's 16.
- **The analog's canvas is 185 px broader than at the last report** (2972 -> 3157; area 3.83 -> 4.07 M against the A2
  must of 4.27 M). Cause: the edge to the whole bottom block is no longer rerouted (its pin stands clear now), so it
  keeps the outermost column the layout gave its channel. It is a coordinate matter (X, b12a2) and is inside the
  breadth rule across permutations; said here because it eats area headroom.
- The harness total `x_drawn` static over `tests/fixtures/quality`: **333** (start 405; last report 328: dense-td
  63 -> 68 with the link rule).
- Static rows (d) for information (due A2): `lateral_x` 3 on the analog and the public fixture, `up_lanes` 0 on all but
  perm-b (1).

## 3. Suite-on failing tests against 4.5, and the default run

Default run on `08c6408`: `tools/guard.sh` prints `GUARD: ALL GREEN` (fmt, clippy, tests, doctests, MSRV 1.75, wasm,
rustdoc; logs `B/work/r4/guard{1,3,4,5}.txt`, one per commit group); `everything_renders_as_before_with_blocks_off`
and `the_switch_changes_nothing_outside_block_mode` pass. Every library change of the round sits behind the block
layout (`cx.blocks`, the `blocks` argument of `cut_to_frames`, `BlockOrder`).

Suite-on `cargo test --no-fail-fast` on `08c6408` minus its last test-only commit (`B/local/out/r5/suite-on-tests.txt`;
the two router tests of `08c6408` were then run alone, both pass in both runs): lib 1131 passed, **11 failed** (10 with
`08c6408`); `tests/quality.rs` 9 passed, **6 failed**; cli, model_build, theme and doc tests pass. Against the
verifier's list of round 1 (12 + 7): **no new failure**; gone are `comb::combing_is_settled_after_one_pass`,
`edges_keep_off_frame_borders` and, with `08c6408`, `router::edges_into_a_wedged_frame_keep_out_of_the_gap`.

**`comb::combing_is_settled_after_one_pass`, cause found** (`8634785`). Not the context the test builds: the comb
reads neither the lane classes nor the block flag (`comb.rs` reads `drawn`, `fixed`, `held`, `shaped` and the ends).
Traced with a second comb run after every stage (`B/work/r4`, debug tree): the second run already changes two edges
right after the first. On dense-td's `n42` the two arrivals on the top side cross; every deal of the first run is
refused at the ports the nudge left; then `plan_even` evens the side's ports; at the even ports the first deal is
kept, which only a second run tried. Fix: in a block layout a side whose ports were evened is dealt once more. Test
`comb::a_block_layouts_comb_is_settled_after_one_pass` (five fixtures as blocks).

**`router::edges_into_a_wedged_frame_keep_out_of_the_gap`** (lead's decision 4): the body runs with the block layout
switched off (the path it was written for, in both runs); the rule is tested in block mode by
`edges_between_two_lanes_keep_off_their_gap_as_blocks` on `blocks/swimlane` and `blocks/swim-cycle`, the two fixtures
the lane rule leaves side by side with a wedge (frames 20 px apart, under `2 * FRAME_SEP` = 28). Tried and without a
wedge: `blocks/swimlane-nested` (28 px apart).

| still failing | class | cause |
|---|---|---|
| `casing::a_line_takes_one_role_across_a_cluster_of_crossings`, `casing::a_short_line_keeps_at_most_one_gap`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route` | 4.5 intended: re-read | fixture-specific numbers on the re-laid dense fixtures; wait for the geometry |
| `comb::a_hubs_fan_is_a_comb_in_scene_and_route` | 4.5 intended: re-read | the rule does not hold: crossings inside dense-td's hub fan (`fan_x` 18, section 6.2) |
| `ports::a_nearly_straight_pair_is_drawn_straight`, `render::advanced_showcase_parses_and_renders` | must pass (`examples/advanced.mmd`) | no straight spine across a border without X1-X3 (B12.8) |
| `scene::a_composite_edge_to_its_own_state_drops_straight_from_the_border` | must pass | own-box edge (2.8, B12.14) |
| `scene::a_lane_stays_in_its_frames_in_route_and_on_a_drag_too` | must pass | `route()` at block positions (R1/F3, B12.13) |
| `scene::note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `scene::note_links_match_between_scene_and_route` | must pass | note links in block mode, scene against route (B12.14, B12.13) |
| `quality.rs`: `block_fixtures_are_not_drawn_worse` | gate | the same 20 lines as before the round (`examples/advanced` static overlap 1; `notes/fork-twins` static; `notes/wide-frames`; the routed rows of the public fixture, nested-cases; `blocks/nested-chain` static and routed hard zeros 1) |
| `quality.rs`: `quality_gate`, `defect_baseline_gate`, `channels_keep_edges_apart_and_off_chips`, `crossing_gaps_do_not_read_as_dashes`, `a_one_px_drag_leaves_unrelated_edges_put` | gates of 4.5 | static halves due at A2, routed halves and drag jumps at R |

## 4. Perf (4.6)

The machine was noisy today (no `rustc` of the other package running; load 1-2.5; single runs swing by half). Every
number below is from three runs; a timing is called only where runs agree.

`scene_ms` with the switch on (`B/work/r4/timing`, start binary and suite-on binary interleaved, three rounds):

| file | start | switch on | limit |
|---|---|---|---|
| analog | 10.2 / 10.2 / 6.9 | 10.24 / 10.29 / 11.95 | 11.58 |
| v2 | 12.5 / 10.7 / 10.0 | 9.93 / 9.47 / 10.49 | |
| public fixture | 6.95 / 6.87 / 6.94 | 9.42 / 14.50 / 8.92 | 12.12 |
| bench-75 | 37.8 / 37.0 / 36.5 | 45.94 / 47.92 / 45.39 | 61.94 |
| dense-td | 25.0 / 24.2 / 26.3 | 40.92 / 64.92 / 40.52 | 50.68 |
| dense-lr | 19.4 / 19.6 / 18.7 | 16.33 / 22.95 / 13.93 | 46.8 |
| advanced | 0.85 / 0.88 / 1.11 | 1.36 / 1.30 / 1.86 | 1.42 |
| subgraph | 0.13 / 0.15 / 0.30 | 0.18 / 0.17 / 0.26 | 0.38 |
| fanout-frames | 2.05 / 3.18 / 2.60 | 2.19 / 2.11 / 2.24 | 4.38 |

The second round of the three was a noisy one (every file slower). On the two that agree A1's gate (analog, public
fixture, bench-75, dense-td, dense-lr) is inside the limits, **with less room than at the last report**: dense-td
about 40.5 ms (was 35), the analog about 10.3 (was 9.7). The link rule runs once per level order and the pins once
per scene; neither was optimised. advanced is not reliably inside its limit, as the verifier found (A2's).

Default build, `tools/perf.sh` three times (`runs/b12a1-fix2`, `-2`, `-3`; the build is byte-identical to the start,
so these are the start's code paths): on the quiet run dense-td `route_ms` 6.69 (limit 9.20), worst `drag_ms` 17.04
(17.80); bench-75 9.38 (15.76) and 17.10 (25.27); dense-lr 4.18 and 14.52 (17.76); fanout-frames drag 2.52 (3.44; 3.58
and 2.99 on the two noisy runs); `lr` drag 1.40 (1.95; 2.02 on one noisy run). No file is over its limit on two
runs.

`route_partial`, worst frame (`fixture_drag_perf_guard`, release, one thread). The figure cannot carry "< 6 ms" on
this machine either way, as the verifier wrote; the start commit, built from `git archive 3abbfeb` and run between the
runs of HEAD, swings the same way:

| | dense-lr | dense-td | bench-75 |
|---|---|---|---|
| HEAD, six runs | 5.63 / 6.37 / 7.11 / 7.19 / 6.04 / 6.48 | 7.86 / 6.89 / 9.58 / 7.19 / 6.61 / 8.50 | 8.12 / 8.50 / 8.90 / 8.38 |
| start, three runs in the same session | 5.70 / 6.71 / 5.96 | 6.79 / 6.44 / 8.99 | 9.03 / 8.09 |

## 5. Crossings by number of starts, the six shuffles, the lane rule

Static `x_drawn` with the ordering forced to k starts, then with the starts the budget grants
(`B/local/out/r5/starts-table.txt`):

```
input                    s1   s2   s3   s4   s5   s6   s7   s8   s9   granted  must
analog, perm-a/c/d       19   19   19   19   15   15   15   15   15   15       20
v2                       20   17   17   22   22   22   22   22   15   15       17
perm-b                   27   27   27   26   26   22   22   22   22   22       23
perm-e                   31   25   25   25   25   25   17   17   17   17       22
public fixture (all 5)   6    6    6    6    6    6    6    6    6    6        7
dense-td                 98   37   68   68   68   68   68   68   68   68       74.6
dense-lr                 19   19   17   17   17   17   17   17   17   17       51.5
bench-75                 230  248  248  248  248  248  248  248  248  248      325
```

- Still not monotone (v2: 22 with 4 to 8 starts, 15 with 9; dense-td: 37 with 2, 68 with 3 and more). The pin
  `BLOCK_ORDER_STARTS = 9` / `BLOCK_ORDER_WORK = 1400` is unchanged, as the lead decided.
- **dense-td under the test's six edge-line shuffles** (lead's note 7; `B/work/r4/shuf`, the generator of
  `edge_line_order_barely_moves_the_dense_layouts`): crossings 68 in source order, then 59, 59, 59, 69, 59, 63; width
  3774.55 on all seven. Inside the must of 74.6 and the 10 % of width.
- Levels the lane rule (L4) fires on: the root of `swimlane`, `swim-cycle` and `swimlane-nested`, no other fixture or
  local input; 52 of the 160 generated flowcharts of the sweep (unchanged).

Generated sweep (B12.6, 240 graphs, 200 in block mode): no panic, deterministic, block rows and the order constraints
hold, now including "one side per (link, block)" on 120 generated diagrams. Hard zeros in the static scene: 48 on 29
diagrams as blocks, 105 on 52 before blocks; blocks add some on 9 diagrams. What draws those nine
(`B/work/r4/sweepin`): on the eight state diagrams it is one pair each, the links of two notes on one state (`note
right of` and `note left of` the same state) sharing a port on it (`port K0_0 __note_5 __note_6`; note links,
B12.14); on `flow 23 RL` an edge from a whole block ends on a lateral side of its frame (`frame_side`; a pin, B12.11).
The ratchet of `8c6dc26` stands; the 0 of the spec is open.

## 6. Every must that is open, every stop rule

Stop rules of A1's row: none of the crossing rules fires (table above; the six shuffles measured). The gate is open on
the musts below.

1. **Runs through a foreign frame** (`xframe_runs` 0): bench-75 2, dense-td 1, `notes/fork-twins` 1.
   - dense-td `n46 -> g9` (an edge to a whole block): its step out of `n46` lies in the gap under the first band,
     inside that band's title strip, where three frames nest. A pin and track matter (2.8, 2.6 G2).
   - bench-75 `gw -> logs`: drawn by the router repairing a node hit of an end spilled round a corner of an over-full
     side; through `app` and `data`. The spill order of 2.8.
   - fork-twins `__note_17 -> K0_0`: a note card's link (B12.14).
   - Fixed this round: dense-td `n41 -> n42` (`fbd943d`: the two ends of a link between two rows stand on one side of
     every block that spans both rows; `assert_one_side` checks it on every block fixture) and the verifier's
     `s4-state` (`4f638ea`: the stand-in channel of a turned edge of a whole block was not cut to its frames, because
     `cut_to_frames` took forward channels only, and the edge fell back to one curve from frame to node. The note card
     was not the cause; it only changed which rows the composite between took).
2. **`fan_x`**: 0 on the analog, perm-a to perm-e, the public fixture with its permutations, hub5; **v2 1**; dense-td
   18 (start 1), bench-75 21 (start 6), fork-twins 2 (start 0).
   - The local family's crossing was a free pin on the entry column of another edge (2 px beside it): the two ran on
     top of each other, the router took one away, across its own fan. Fixed by `32a3ebf`.
   - v2's is `DNS -> SITE`, a direct entry into a first-row member that lies wholly under its block's title chip; the
     router takes it round and it crosses `DNS -> PSDEV`. It needs a displaced entry (2.8 rule 2).
   - dense-td and bench-75: after the `ortho` stage they stand at 7 and 5; the router's repairs bring them to 12 and
     31 (dense-td: 9 edges with a chip hit; bench-75: 6 chip hits and node hits of spilled ends), the nudge and comb to
     18 and 21. Title-clear entries and the spill order again.
3. **Static hard zeros 0**:
   - `blocks/nested-chain` 1 (`own_frame_x`; section 6.4).
   - `notes/wide-frames` 2 (start 0), shown per the lead's note 6: both are one pair of note links on `K0N_1`,
     `__note_12 -> K0N_1` and `K0N_1 -> __note_13`, 2.6 px apart on one side (`shared_ports` 1, `arrow_on_start` 1).
     Nothing else on the fixture is a hard zero. Note links, B12.14.
   - `notes/fork-twins` 2 (start 3): `__note_16 -> K0N_0` leaves its composite for 689 px (`own_frame_x`) and a shared
     port; `notes/nested-composite` 2 (start 4): a shared port and an arrow on a start. Note links.
   - `blocks/own-centre` 1, `blocks/self-member` 2, `blocks/sub-nested-self` 3: own-box edges, as at the start
     (B12.14).
   - Fixed this round: `notes/spread-face` 1 -> 0 (a pin shared a port).
4. **`blocks/nested-chain`** (verifier's finding 2). The claim of the last report ("0 again") was wrong: the row had
   been read without `own_frame_x`. The drawing is as the verifier describes. Cause: the channel of `a -> c2` enters
   `Inner target` on that block's first row, left of its only first-row member, i.e. under the title chip; the `ortho`
   stage draws the whole edge as one straight drop there, the router finds a chip hit and takes it round through the
   blocks' side walls, outside `Outer`. The layout's half of B12.4's check (the subgraph path of every channel point)
   holds and is what the test asserts; its comment says so now. The drawn half needs the channel to stand clear of
   the chip (X2.5) **and** the direct entry `b2 -> c` to stay clear of it too: a rule for the channel alone was
   built and taken out (section 7), because it only moved the defect to `b2 -> c`.
5. **Suite-on tests that must pass as written**: section 3 (seven left of the eight; none new).
6. **Blocks against the start elsewhere** (verifier's findings 5, 6):
   - `examples/advanced` static: one overlap, `Gate -> Auth` on `Gate -> Cache` for 70 px (unchanged; not analysed
     further this round).
   - State bulk corpus (`b8r1/bulkst`, 964 inputs, 833 in block mode; `B/work/r4/bulk`): static `fan_x` 144 -> 135
     (worse on 47, better on 57), crossings 656 -> 330, hard zeros 534 -> 299 in all, **added on 64 inputs**; routed
     `fan_x` 160 -> 174, hard zeros added on 62.
   - Flowchart bulk corpus (`b8r1/bulkfc`): **not re-measured in full**. The harness example did not get past a slow
     input within 25 minutes with either binary; on the 151 inputs before it (34 in block mode) static `fan_x` is
     14 -> 109 (worse on 8, better on 2) and no input gains a hard zero. The verifier's finding 6 (fan crossings
     double) stands.

Verifier's findings of round 0 not touched: #9 (slanted strokes, lost spines) and #10 (`route()` outside its canvas):
b12a2's and b12a3's.

## 7. Pulled forward, tried and taken out, not built

- **Pulled forward from B12.11 (2.8 free pins), in its smallest form** (`32a3ebf`, `ports::free_pins`, after the
  member ports are placed, block layouts only): a pin within `PORT_SPACING` of another crossing of its frame side, or
  under the frame's title chip, moves to the nearest clear place inside the gap between the crossings on either side
  of it; it never changes places with one. `b12a2-geometry` still builds: pins as even bands in the order of the
  entries, `pin_corner` (the inset used here is exactly `PORT_SPACING`, which the measure counts), pins in `route()`.
  **An over-full side is left as it was**: on the public fixture the pin of `OPS -> OBS` stands on the first entry
  into `metrics store`, whose three ports fill the stretch the title chip leaves; the router takes that entry away as
  before.
- **Built with the lead's decision 2** (`fbd943d`): one side per (link, block), in the ordering.
- **Tried and taken out: a half pixel of air in the chip's shadow** (`ports::unshaded` clearing by
  `ortho::CHIP_CLEAR` instead of `CHIP_SEP` in block layouts). `unshaded` leaves the first free port exactly on the
  router's limit beside a chip, and the router counts a touch; with the air hub5's three edges that today go round to
  a lateral side enter their block's first member through its top, beside the title. But on two permutations of the
  public fixture the entry it frees is the one the over-full pin above stands on: 18 px of two edges on top of each
  other, `overlap_pairs` 0 -> 1 against a ceiling of 0. Left for B12.11 with its pin bands; `unshaded` has the
  parameter and its doc comment says why it is not used.
- **Tried and taken out: title-clear channels as a rule of the ordering** (X2.5's place for an inner run; the patch
  is `B/work/r4/title-run-rule.patch`): a channel point on a block's title row stands past the members the chip
  reaches over, moving past plain nodes only. On nested-chain `a -> c2` then drops straight beside both blocks'
  members, as mermaid draws it, but the member `c` slides under the chip and `b2 -> c` is taken round instead
  (`own_frame_x` stays 1). On the acceptance inputs it cost crossings: perm-b 22 -> 28 (must 23), dense-td 63 -> 74
  (must 74.6). An order constraint without the breadth X2.2 reserves and without displaced entries moves the defect
  and perturbs the multi-start result. Not landed.
- **What the open musts wait for, in one place**: the displaced entry with its jog room (2.7 F2, 2.8 rule 2: v2's fan
  crossing, nested-chain, most of the dense fixtures' router repairs), the spill order (bench-75), the track packing
  of a gap for pins (dense-td `n46 -> g9`), note links (B12.14). None could be met "in the smallest form" this round
  without breaking a crossing must or a ceiling; the measurements are above.
- A question for the lead, not acted on: the spec displaces an entry under a chip and gives it a jog (rule 2). The
  alternative seen while trying the above is to keep a first-row member with such entries far enough along its row
  that its title-facing side has a free stretch for them (no jog, the block broader by up to the stretch). It is a
  different design from 2.8 and was not built.
