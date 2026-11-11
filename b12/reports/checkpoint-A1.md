# Checkpoint A1: structure (B12.0-B12.6)

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`,
branch `ccr-ccc6d89a-5cxd7e`.

**Gate: NOT met. Stop rules fired** (4.1, row A1): crossings above the must on the analog (21 against 20), on all
five permutations, on the public fixture and its four permutations (9 against 7) and on dense-td (151 against 74.6).
Section 6 lists every must that failed, with its numbers. Per decision D6 nothing was re-baselined or loosened; the
package stops here and b12a2 should not start before the lead has read section 6.

What does hold: the layers and bands of B12.3 (analog: exactly the layers and the five upstream edges the spec lists,
under all five permutations; public fixture: the same band sets under its four permutations); chains through
flow-facing sides at the layout level (B12.4); the two constraints of the level order on every block fixture and on
the generated sweep; `scene_ms` within 4.6 on the five gated inputs on two runs; the harness total `x_drawn` static
over `tests/fixtures/quality` 390 against the starting release's 405; both declaration-order tests pass as written;
the default build is byte-identical and guard is green.

## 1. Commit, command lines, judge output

- Commits of the package (start `3abbfeb`): `c024ac0` B12.0, `bd7069a` B12.1, (B12.2: no commit, see checkpoint-S.md),
  `1fa410e` B12.3, `5f24084` B12.4, `15ecef7` B12.5, `48d9022` B12.6. Everything below ran on `48d9022`.
- Suite-on build: `RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=B/target-on CARGO_INCREMENTAL=0 cargo build
  --release --bin flowmaid --example quality` (and `cargo test --no-fail-fast` with `CARGO_PROFILE_DEV_DEBUG=0`).
  `B/target-on` is deleted after this report.
- Local acceptance, static: `cd B/local && ./run.sh fm B/target-on/release/flowmaid out/a1 b12-a1 && ./run.sh judge
  out/a1 --due A1` (92 inputs; reference tables and thresholds of B12.0). Output: `B/local/out/a1/judge-a1.txt`,
  metrics `out/a1/metrics.tsv`. Result: **155 checks, 19 must-fail, 2 targets missed**; 109 rules due at a later
  checkpoint were left out (`--due A1`).
- Harness: `B/target-on/release/examples/quality B/local/in > out/a1/quality-local.tsv`, the same on
  `R/tests/fixtures/quality` (`out/a1/quality-fixtures.tsv`), and with `--starts 1..7` on the 15 table inputs
  (`out/a1/starts-table.txt`).
- Repository fixtures: the rows of 4.3 (c) are in `tests/quality.rs` as `BLOCK_CEILINGS` with
  `block_crossing_ceiling` (suite-on: fails, see 3).

## 2. The tables of 4.3 (a)-(c): the rows due at A1 (static)

`xfr` = `xframe_runs`, `sibL` = `sib_side_linked`, `hard` = sum of the harness hard zeros.

| input | xfr (must 0) | sibL (must 0) | crossings | must | 0.38 / start / mermaid | fan_x (must 0) | hard | canvas breadth |
|---|---|---|---|---|---|---|---|---|
| analog | 0 | 0 | **21** | <= 20 | 24 / 23 / 16 | 0 | 0 | 2899 |
| v2 | 0 | 0 | 15 | <= 17 | 19 / 19 / 13 | **1** | 0 | 2815 |
| perm-a | 0 | 0 | **21** | <= 20 | 12 / 12 / 7 | 0 | 0 | 2899 |
| perm-b | 0 | 0 | **32** | <= 23 | 19 / 18 / 19 | **2** | 0 | 3127 (+7.9 %) |
| perm-c | 0 | 0 | **21** | <= 20 | 24 / 23 / 16 | 0 | 0 | 2899 |
| perm-d | 0 | 0 | **21** | <= 20 | 24 / 23 / 19 | 0 | 0 | 2899 |
| perm-e | 0 | 0 | **40** | <= 22 | 34 / 33 / 18 | **5** | 0 | **2599 (-10.3 %)** |
| public fixture | 0 | 0 | **9** | <= 7 | 14 / 14 / 5 | 0 | 0 | 1162 |
| its edges-reversed | 0 | 0 | **9** | <= 7 | 7 / 6 / 6 | 0 | 0 | 1162 |
| its edges-rotated | 0 | 0 | **9** | <= 7 | 23 / 23 / 5 | 0 | 0 | 1162 |
| its actors-reversed | 0 | 0 | **9** | <= 7 | 15 / 15 / 5 | 0 | 0 | 1162 |
| its subgraphs-reversed | 0 | 0 | **9** | <= 7 | 12 / 12 / 5 | 0 | 0 | 1233 (+6.1 %) |
| bench-75 | 0 | 0 | 223 | <= 325 | 302 / 294 / 228 | 14 | 0 | |
| dense-lr | 0 | 0 | 15 | <= 51.5 | 46 / 45 / 10 | 0 | 0 | |
| dense-td | **1** | 0 | **151** | <= 74.6 | 80 / 66 / 68 | 18 | 0 | |
| hub5 | 0 | 0 | 0 | | | 0 | 0 | |

Bands (top level, in flow order, the units of each band):

- analog and all five permutations: `actors / EXT GH / PROD / DR STG`: the same sets in the same flow order.
  The judge's `bands` string also orders the units of a band across the flow, and there three permutations differ
  from the analog (perm-b and perm-e: `GH EXT` and `STG DR`; perm-c: the actors `U2 T1 U1`): rule `same_as` on
  `bands` FAILS on those three. The order across the flow is the canonical (declaration) order where no crossing
  decides, and those files declare the nodes or the subgraphs in another order.
- public fixture and its four permutations: `OPS SHOP / CI EDGE / WEB / ORD / DATA / OBS` on all five (judge: pass).
- Layers of the analog as blocks (`quality --layers`): actors / R1 R2 GW DNS TG OAI AZ / ARGO TR1 VAULT / LANE / PG /
  SITE DRAPP PSDEV TR2 / PSPROD LANES4 / GITLAB, upstream DRAPP->TG, ARGO->TG, PSPROD->TG, LANE->OAI, LANE->AZ: as
  the spec lists them, on the analog and on every permutation.

Rows (d) for the analog family are due at A2 and R; measured now for information (static): `lateral_x` 3 on the
analog (0.38: 21), 3 on v2, 2-7 on the permutations; `up_lanes` 0 on the analog.

## 3. Suite-on failing tests against 4.5, and the default run

Default run (ordinary build) on `48d9022`: `tools/guard.sh` prints `GUARD: ALL GREEN`;
`everything_renders_as_before_with_blocks_off` passes (114 rows: SVG, route and route_partial hashes of the start
commit); `the_switch_changes_nothing_outside_block_mode` passes on the fixtures and on the generated sweep.

Suite-on `cargo test --no-fail-fast` (`out/a1/suite-on-tests.txt`): lib 1121 passed, **12 failed**; `tests/quality.rs`
7 passed, **8 failed**; cli, model_build, theme, doc tests pass.

| failing test | class |
|---|---|
| `casing::a_line_takes_one_role_across_a_cluster_of_crossings`, `casing::a_short_line_keeps_at_most_one_gap`, `scene::far_apart_back_edges_and_frame_legs_square_up_alike_in_route`, `comb::a_hubs_fan_is_a_comb_in_scene_and_route` | 4.5 intended (re-read: fixture-specific numbers on a re-laid dense fixture); not re-read yet, they wait for the final geometry |
| `block_fixtures_are_not_drawn_worse`, `quality_gate`, `defect_baseline_gate`, `edges_keep_off_frame_borders`, `channels_keep_edges_apart_and_off_chips`, `labels_cover_nothing_they_must_not`, `crossing_gaps_do_not_read_as_dashes`, `a_one_px_drag_leaves_unrelated_edges_put` | the gates of 4.5: static halves are A2's, routed halves and the drag jumps are R's (X, G, F, E's ends, R not built); the crossing ceiling of the public fixture (7) inside `block_fixtures_are_not_drawn_worse` is the A1 must that failed |
| `layout::a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` | must pass as written; narrowing on block layouts is B12.12 |
| `render::advanced_showcase_parses_and_renders`, `ports::a_nearly_straight_pair_is_drawn_straight` | must pass as written (the tests on `examples/advanced.mmd`); fail with the diagram in block mode, cause not analysed (ends and coordinates are not built) |
| `scene::a_composite_edge_to_its_own_state_drops_straight_from_the_border`, `scene::note_links_hold_their_ends_and_corners_in_scene_route_and_drag`, `scene::note_links_match_between_scene_and_route` | must pass (B12.14's checks: own-box edges, state diagrams); not built |
| `scene::a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`, `router::edges_into_a_wedged_frame_keep_out_of_the_gap` | must pass (own-frame tests, B12.8 / B12.13); `route()` is unchanged and meets block positions it was not written for |

The list is NOT a subset of "4.5's intended changes plus what A1's row names as not built": seven "must pass as
written" tests fail. Each is in territory A1's row names as not built (X, G, F, E's ends, R) or B12.14's, but none was
analysed to the cause. `edge_line_order_barely_moves_the_dense_layouts` and
`moving_a_node_declaration_barely_moves_dense_td` PASS as written, as do every test of `compound.rs`,
`every_fixture_has_a_ceiling`, `hops::a_pick_finds_the_painted_hops_on_every_fixture`, the `route_partial_*` tests.

## 4. Perf (4.6): two `perf.sh` runs each

Machine shared with a second package; no `rustc` running, load average 1.2-3.0 during the runs. `scene_ms` (static
row), run 1, run 2:

| file | start | switch on | default build | limit |
|---|---|---|---|---|
| analog | 5.79, 6.78 | 8.64, 7.89 | 5.78, 6.00 | 11.58 |
| public fixture | 6.15, 6.06 | 11.51, 8.80 | 6.62, 6.47 | 12.12 |
| bench-75 | 34.94, 35.46 | 38.63, 39.11 | 35.11, 57.73 | 61.94 |
| dense-td | 23.59, 22.64 | 47.94, 49.18 | 22.11, 37.30 | 50.68 |
| dense-lr | 18.67, 17.89 | 14.69, 15.69 | 16.94, 16.42 | 46.8 |
| advanced | 0.75, 0.75 | 1.31, 1.64 | 0.82, 1.09 | 1.42 |
| subgraph | 0.15, 0.14 | 0.17, 0.16 | 0.12, 0.12 | 0.38 |
| fanout-frames | 2.05, 1.82 | 2.83, 2.30 | 1.95, 1.79 | 4.38 |

A1's gate (`scene_ms` of the analog, the public fixture, bench-75, dense-td, dense-lr within 4.6): met on both runs,
dense-td with 1.5 ms to spare. `advanced` is on both sides of its limit with the switch on (1.31, 1.64 against 1.42);
it is judged at A2. The default build's second run was taken while the other package was building (bench-75 57.73,
dense-td 37.30); its first run equals the start.

`route_ms` / worst `drag_ms` (routed row; R's budgets, reported): the default build is the start within the noise of
the shared machine (analog route 2.03, 1.90; drag 6.93, 5.51 against a limit of 5.69: one run inside; public fixture
drag 5.38, 8.65 against 6.31: one run inside; dense-td drag 12.45, 14.18 against 17.80). With the switch on,
unchanged `route()` at block positions: analog route 3.30, 3.25 (limit 2.88), drag 6.33, 5.44; dense-td route 10.40,
15.65 (limit 9.20), drag 17.96, 22.60; bench-75 route 11.78, 12.30, drag 21.55, 21.22; dense-lr route 3.70, 5.36;
public fixture route 4.05, 2.77. These are what checkpoint S measured and what B12.13's corridors are for.
Full tables: `runs/b12a1-on-1`, `runs/b12a1-on-2`, `runs/b12a1-structure`, `runs/b12a1-structure-2`
(`out/a1/perf-table.txt`).

`route_partial`, worst frame, default build, `cargo test --release --lib -- --ignored --test-threads=1
fixture_drag_perf_guard` (three runs; the second ran under load and tripped the test's own 8 ms on dense-td):

| fixture | start | run 1 | run 2 | run 3 |
|---|---|---|---|---|
| dense-lr | 5.66, 5.87 | 5.18 | 5.69 | 5.36 |
| dense-td | 5.68, 5.55 | 6.00 | 8.47 | 5.96 |
| bench-75 | 7.96, 5.40 | 7.19 | (not reached) | 7.50 |
| public fixture | 4.04, 3.98 | 4.62 | | 3.98 |

dense-td reads 6.00 and 5.96 ms against the 6 ms line (start 5.68, 5.55). This is the test build: since B12.1 every
entry point records the block mode it used under `#[cfg(test)]` (the record `block_mode_agrees_between_scene_and_route`
reads), which costs one `Augmented::of` per drag frame there and nothing in an ordinary build. bench-75 was at 7.96
on the start commit already (reported in B12.0).

## 5. The table crossings x starts, and the levels the lane rule fires on

Static `x_drawn` (harness) with the ordering forced to k starts (`quality --starts k`), then with the starts the
budget grants (`BLOCK_ORDER_WORK` 9000: 7 on every input here), and `scene_ms` with the granted starts:

```
input                                 s1  s2  s3  s4  s5  s6  s7  granted  scene_ms
analog                                26  21  21  21  21  21  21     21     8.57
v2                                    17  15  15  15  15  15  15     15     8.19
perm-a                                26  21  21  21  21  21  21     21     8.47
perm-b                                32  32  32  32  32  32  32     32     8.13
perm-c                                26  21  21  21  21  21  21     21     8.21
perm-d                                26  21  21  21  21  21  21     21     8.56
perm-e                                40  40  40  40  40  40  40     40    11.50
public fixture                         9   9   9   9   9   9   9      9     8.41
  edges-reversed / rotated             9   9   9   9   9   9   9      9
  actors-reversed / subgraphs-rev.     9   9   9   9   9   9   9      9
dense-td                             151 151 151 151 151 151 151    151    52.55
dense-lr                              16  16  16  16  16  16  15     15    15.96
bench-75                             265 265 297 297 224 224 224    224    39.69
```

More starts do not buy the musts: the analog is at 21 from the second start on, the public fixture at 9 from the
first. The start is chosen by layered crossings; what is drawn is more:

- public fixture: layered 5, the layout's own skeleton count 5-6, drawn 9. dense-td: layered 98, skeleton 80,
  drawn after the builder 118, after the ports 103, after `ortho` 139, after the router 162, final 151.
- Where the difference comes from, as far as it was looked at: an edge into a block's first row crosses the title
  side where the title chip lies (TD), the router takes it round to the member's side, and the detour crosses its
  neighbours (seen on the public fixture, `nested-chain`, the two-block cycle; pictures `B/work/pic/`). Title-clear
  entries, free pins and gap room are b12a2's (B12.7-B12.11). On dense-td the middle band is one block as broad as
  the drawing, so every edge from the first band to the third now runs round it; 0.38 and the start ran those edges
  through it (12 runs through foreign frames, now 1).
- An earlier state of the ordering (barycentres summed in link order) drew the analog with 18 and the public fixture
  with 8; it was not independent of the order of the edge statements, which O4 requires, so it was not kept.

Levels the lane rule (L4) fires on: the root of `swimlane`, `swim-cycle` and `swimlane-nested`, and no other
fixture or local input (dense-td, bench-75, advanced checked by name: none). On the generated sweep (random edges)
it fires on 52 of 160 flowcharts. L3 as built departs from the spec's letter in one point (commit `1fa410e`): a plain
node nothing links into starts neither walk, the walk from the sources starts at what such nodes link to, heaviest
entry first. With the letter, the public fixture's actors-reversed permutation stacked its shared services below
the clusters they feed (3 turned links instead of 4) and failed "same bands".

Generated sweep (B12.6, 240 graphs, 200 in block mode): no panic in `scene()` or `route()`, deterministic, block rows
and the two order constraints hold on all. Hard zeros in the static scene: 29 diagrams (49 in all) as blocks, 52
diagrams (105) before blocks; not zero. On 9 diagrams (one flowchart, eight state diagrams) blocks add hard zeros
where there were none.

## 6. Every must that failed, every stop rule that fired

Stop rules of A1 that fired:

1. Crossings above the must: analog 21 > 20; perm-a 21 > 20, perm-b 32 > 23, perm-c 21 > 20, perm-d 21 > 20, perm-e
   40 > 22; public fixture and its four permutations 9 > 7; dense-td 151 > 74.6 (the smaller of 1.1 x 0.38's + 2 and
   1.1 x the start's + 2).
2. (Not fired: the harness total `x_drawn` static over `tests/fixtures/quality` is 390 against 405; the musts do not
   need more starts than the budget grants, they are not reached with any number; dense-td under the test's six
   shuffles stays within 10 % in width, the test passes as written. dense-td's crossing must fails under every
   shuffle, as in rule 1.)

Other musts due at A1 that failed:

3. `xframe_runs` 0: dense-td 1, `notes/fork-twins` 1.
4. `bands` `same_as` the analog: perm-b, perm-c, perm-e (order across the flow inside a band; the band sets and their
   flow order are the analog's).
5. Canvas breadth within 10 % of the analog's: perm-e 2599 against 2609-3189 (-10.3 %).
6. `fan_x` 0 on the analog family: v2 1, perm-b 2, perm-e 5 (analog, perm-a, perm-c, perm-d, the public fixture
   and hub5: 0).
7. Hard zeros 0 in the static scene: not 0 on `notes/fork-twins` 2, `blocks/nested-chain` 1, `blocks/own-centre` 1,
   `blocks/self-member` 2, `notes/spread-face` 1, `blocks/sub-nested-self` 3, `notes/wide-frames` 5 (own-box edges
   and note links: B12.14; `nested-chain`: an entry under a title chip); and on 29 of the 200 generated block
   diagrams.
8. Suite-on tests that must pass as written and do not: section 3.

Not built in this package, and said so in the commits:

- B12.3's wrap of a long row of members without inner links. As worded it wraps every one-row block of three or more
  members with depth to spare beside it: on the analog DR becomes a column of three and on the public fixture the
  shared services become two rows, against the layers B12.3's own check lists; restricted to members with no link at
  all it fires on no input. It needs the lead's decision on its trigger; its acceptance (dense-lr's area, "no block
  row longer than its band needs") is B12.12's.
- The weighted adjacent exchange after each half of a level's sweeps (O2): the level order is barycentre sweeps
  only. The `#[cfg(test)]` start-index offset of B12.5's check is replaced by forcing the number of starts
  (`with_order_starts`).
- The consumers of `edgepass::Ctx::back` do not ask "E4?" yet: the edge stages still treat an upstream chain as a
  back-edge (it is not reshaped by the ports stage). The builder and `ortho` draw it through flow-facing sides; the
  router is what takes it round a title chip.
