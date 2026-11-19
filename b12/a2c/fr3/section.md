## b12a2c-squeeze, fix round 3 (2026-11-19, on top of `704b118`; read this first)

`B` = `scratchpad/b12`; work files in `B/a2c/fr3/` (check runs `c-A` = `967c5ea`, `c-B` = `9a79456`, `c-C` = `1ae4222`;
`c-D` = a dropped nudge experiment; chips off their lines over every set by `tools/det3.sh` / `tools/det3cmp.py`
(the verifier's `det.py`, 3 px) in `det-start` (`d99c073`), `det-704` (`704b118`), `det-C` (head); per-file
verdict columns over every set by `tools/setcmp.py`; pictures `pic/`). The sets: the 95 local inputs, the 332 + 32
direction copies, the verifier's generated set `gen` (250) and its `in` / `in2` / `it3` inputs, and fr2's generated
sets (rnd 300, nr 250, nt 150, sw72 72, tw 33, i3 7).

**Three new commits, all green** (guard incl. rustdoc; the three checks against the package start `d99c073` and
against the commit before; default build byte-identical: the 25 corpus SVGs equal `runs/pre-b12a2c-squeeze`, 0
differ):

| commit | what | finding |
|---|---|---|
| `967c5ea` | the label stage's final moves keep `aa1c980`'s air only for a chip already on its line (`labels::Field::keeps_air`, `ON_LINE_TOL` 3 px) | #2 |
| `9a79456` | a title side deals its frame pins and displaced entries as one set only where a pin meets a column, another pin or a crossing; the edge stages keep the set only where `quality::measure` finds the finished drawing no worse (`edgepass::stages`) | #4, item 3 |
| `1ae4222` | X4 holds a sibling block whose last row feeds one node outside together with another sibling's (`layout::level_sibling_frames`, `fan_from_lasts`) | #3, #5 |

**`compare.sh` at `1ae4222` against `d99c073`: VERDICT no worse** (cells, panics, dirs; also against `704b118`: every
cell, gate message and copy unchanged). Local static A2 **70 must-fail lines** (unchanged), routed 18, gate messages
8 failing with the same names. **Checkpoint A2 is still NOT met**: the 70 lines and DEFECT_BASELINE's static column
(`lat_wiggles` 33 > 15, `lat_backtrack` 10599.64 > 1870, `band_pairs` 204 > 132, `band_len` 91602 > 74730,
`end_kink` 1 > 0) are as at `704b118`; items 1 and 2 were not built this round (below).

### The verifier's findings, one by one

1. **A2 gate / DEFECT_BASELINE / items 1-2 (blocking): not met, not retried.** This round went to the regressions the
   verifier found (#2-#5), which the lead's rules put first (no chip newly off its line, no pin rule that trades
   crossings for overlaps); the squeeze's D6 stop stands as fr1 measured it (27-35 worse cells in every form,
   `fr1/squeeze-fr1.diff`), and no form that is no worse on every cell and test was found. Item 2's causes stand as
   fr2 traced them (the 33 wiggles are fan lines going round a block that holds neither end; `end_kink` is
   `router::repair`'s stub on bench-75 `svc1 -> logs`, whose fix costs bench-75 defects 0 -> 3 and `fan_x` 4 -> 15).
   The 70 lines per fixture and their causes are as in fr2's table (below, unchanged).
2. **Lead ruling 1 outside the gated sets (`aa1c980`'s air in the final return): fixed, `967c5ea`.** Cause as the
   verifier found: the return to a line's middle (and the head-clear move) asked for no stroke within `CLEAR_GAP`
   of the new spot, so a chip the builders' pass had set beside its line stayed there. Now the air rule holds only
   where the chip already sits on its line; a chip off its line returns as before `aa1c980`. Back on their lines:
   `chips-crowd` TD `reg -> web deploy` 21.2 -> 0, `g7019` 7.7 -> 0, `g7066` 11.9 -> 0, `g7115` 10.6 -> 0, and one the
   verifier did not list, nr / rnd `r174-TD` `lbl2` 17.2 -> 0 (also `aa1c980`). The public fixture's `deploy` /
   `write` chips keep their air (`a_block_layouts_chips_keep_clear_of_other_lines` passes). Picture
   `fr3/pic/chips-crowd.png` (start | `704b118` | head). Test `a_chip_returns_to_its_line_however_near_another_line_passes`.
3. **More chips newly off their lines.** Bisected over the package's commits (`det3.sh` on every commit's binaries):
   - `nest-sib-lbl` TD `b2 -> z done` 21.6 px (`9a0e27e`): **fixed**, `1ae4222` (finding 5's rule; the chip is on its
     line in all four directions, `fr3/pic/nsl.png`).
   - `r022-TD` 9.4 -> 17.5, `r136-TD` 9.0 -> 9.7, `r240-TD` 2.0 -> 9.0, `r280-TD` 0 -> 9.0 (all `0f2a838`, not in the
     verifier's list): **fixed**, `9a79456` (the pin judge counts chips off their lines, `labels::chips_off_lines`).
   - `g7247` `audit` 0 -> 9.0 and `g7248` `pack` 0 -> 21.1 (`5e30a15`): **not fixed; both are swaps.** In `g7248` the
     `queue` chip, 25.5 px beside its line inside its end block at the start, now sits on its line outside the block
     (`5e30a15`'s rule), on the top run that `pack`'s line shares 14 px away, and `pack` has no clean spot left on its
     own line (`fr3/pic/g7248.png`); in `g7247` the two chips of a fan into one node trade places (start: `split` 9.0
     beside its line, `audit` on it; now the reverse, 9.0). Per input the count of chips off their lines is the same
     and the farthest is no farther (`g7248` 25.5 -> 21.1). Making both fit needs the label stage to place the two
     labels together (a global choice), not done.
   Over every set at the head against the start: chips more than 3 px off their lines local 72 -> 65, dirs 124 -> 115,
   gen 119 -> 105, rnd 40 -> 34, nr 33 -> 29, others equal; newly off or farther: only the two swaps above.
4. **The pin rule's trades (r183-TD and four more): fixed, `9a79456`.** `0f2a838` dealt a title side's pins with its
   displaced entries as one set on every side with both. Now (a) a side is dealt so only where a pin would come within
   a track of a column, another pin or another line's crossing (elsewhere it is dealt exactly as before `0f2a838`),
   and (b) where it is dealt so, the edge stages run a second time with the pins left alone and keep the set only
   where `quality::measure` finds no more defects, crossings, fan crossings, chips on lines or chips off their lines,
   and no more wall entries, flank ends or smaller gap unless the set takes a defect away. Static, start | `704b118` |
   head:

   | input | defects | x_drawn | lateral_x | fan_x | side_ends | min gap |
   |---|---|---|---|---|---|---|
   | r183-TD | 2 / 1 / 2 | 7 / 13 / 7 | 1 / 3 / 1 | 0 / 2 / 0 | 3 / 4 / 3 | 0.25 / 16.91 / 0.25 |
   | v4-pin-and-labels-TD | 0 / 0 / 0 | 3 / 0 / 3 | 0 / 1 / 0 | 0 | 0 / 1 / 0 | 23.46 / 12.5 / 23.46 |
   | r042-TD | 0 | 2 / 3 / 2 | 0 | 0 | 0 | 10.5 / 12.5 / 10.5 |
   | v6-state-twins | 0 | 1 / 2 / 1 | 0 | 0 | 0 | 12.51 / 12.5 / 12.51 |
   | n078-TD | 0 / 1 / 0 | 1 / 0 / 1 | 0 | 0 | 1 / 0 / 1 | 6.89 |
   | b9-TD, b11-TD | 1 / 0 / 0 | 4 / 0 / 0 | 1 / 2 / 2 | 0 | 1 | 3.39 / 18.9 / 18.9 |
   | nested-n-TD | 1 / 0 / 0 | 0 | 2 | 0 | 0 | 5.91 / 19.41 / 19.41 |
   | n121-TD | 2 / 1 / 1 | 18 | 4 / 3 / 3 | 3 / 0 / 0 | 5 / 3 / 3 | 13.65 / 1.65 / 1.65 |

   r183 keeps its start drawing (its two overlaps stay; the set form's cost was six crossings). The trades the judge
   accepts, all for a defect taken away and never for a crossing: b9 / b11 one wall entry (x_drawn 4 -> 0), n121 a
   1.65 px gap (an overlap and three fan crossings gone), rnd / nr `r116-TD` gap 12.88 -> 12.31 (an overlap gone). Per
   file over all sets against the start no crossing, fan crossing, chip on a line or chip off its line is added
   anywhere; totals rnd defects 28 -> 22, x_drawn 221 -> 215; nt 13 -> 11, 166 -> 159; sw72 2 -> 0, 69 -> 61; tw equal.
   Cost: in a block layout's static scene the stages keep a clone of the scene (all inputs), and run twice plus two
   `measure`s only where a side is dealt as a set (no repository fixture has the case). Test
   `a_frames_pins_move_past_its_entries_only_where_that_draws_no_worse` (r183 and v4; fails at `704b118`).
5. **X4 at any depth on nest-sib-lbl BT (and TD's chip, #3): fixed, `1ae4222`.** The three nested blocks feed one end
   below their parent; levelling the first moved its leg of that fan. `level_sibling_frames` now holds a block whose
   last row feeds one node outside with another sibling's (the mirror of `630096c`'s fan-into-first-rows rule). BT:
   lateral_x 2 -> 0, side_ends 1 -> 0 (as at the start; `sib_stagger` and `edge_chip` go back to the start's too);
   TD: `done` on its line. `nested_sibling_frames_start_level_too` and the public fixture's `sib_stagger` 0 hold
   (no gated cell changed). Test `sibling_frames_whose_last_rows_fan_into_one_node_stay_on_their_layer_lines` (four
   directions; fails at `704b118`). Pictures `fr3/pic/nsl.png`, `nsl-bt.png`.
6. **Item 3 remainder.**
   - **n070-TD overlap: traced, not committed.** Stage dump (`FMDBG_EDGE`, not committed): after `ortho` `q -> b0`
     enters `b0` under the title chip; `router::repair` reroutes it past the chip on column 511.2; the channel nudge
     spreads that leg and `q -> B`'s pin leg (520.9) as one group (519.9 / 532.4), then judges each edge on its own:
     the pin's move is refused and the entry's kept, 1 px beside the pin. Experiment `fr3/nudge-revert-experiment.diff`
     (block layouts TD / BT: a kept move that lands within `EDGE_SEP_MIN` of a neighbour whose move was refused goes
     back): n070 overlap 1 -> 0, n109-BT x_drawn 2 -> 0, nothing else in any set; but n109-BT is the input of
     `casing::a_block_layouts_under_line_keeps_no_fragment_between_two_gaps`, whose two gaps the experiment removes
     with the crossings (the test fails: "the pin is cut"). Changing that test's input is a test change I did not make
     on my own; dropped (`c-D`: panics WORSE by that one test). A second form (`nudge-stays.diff`: the nudge's
     "scene alone" tiers keep off legs that do not move) did not reach n070 and cost three gaps; dropped.
   - **twin-n-TD `lateral_x` 3, r170-TD tight pair: not attempted this round** (unchanged since the start).
7. **D5:** this round's three messages name no input of the local family and quote none of its numbers (the
   verifier's generated inputs and the public fixture only). `7208f50`, `630096c`, `5e30a15` still need the squash.

### The 70 lines at `1ae4222` (`fr3/c-C/local/judge-A2.txt`)

Identical to `704b118`'s (fr2's table below): analog 12, v2 8, perm-a / c / d 6 each, perm-b / e 5 each, the public
fixture and its four reorderings 3 each, public BT 1, cycle2 1, dense-lr 3, dense-td 1, bench-75 1; causes as there
(squeeze: compact / area / straight / detour; labels on the local family; lateral X3 / e4; X4 held; LR / RL squeeze).

### Perf, guard, disk

- Guard green on all three commits (fmt, clippy, tests, doctests, MSRV 1.75, wasm, rustdoc `-D warnings`).
- `perf.sh` (default build; rustc idle, load 1.0-1.9) two runs `b12a2c-fr3-a` / `-b`: within every PLAN budget
  against `runs/baseline` (bench-75 drag 22.9 / 19.7 ms against 20.2, route 14.2 / 14.0 against 12.6, static scene
  53.2 / 51.0 against 31.0 (2x = 61.9); dense-td drag 17.3 / 17.0 against 14.2 (1.25x = 17.8)). Against fr2's run b
  bench-75 static scene 40.2 -> 51.0 and route 10.6 -> 14.0 on the default build, which runs none of the changed code
  (title sides, the second stage run and X4 are block-layout only; the label stage's air test returns at once outside
  block layouts) and is byte-identical: the box's noise, as in fr1 / fr2.
- Disk: 7.1 GB free on `/`; `fr3/` 335 MB (experiment binaries and outputs removed; one target dir kept for the
  verifier, safe to delete).

