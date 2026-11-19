## b12a2c-squeeze, fix round 4 (2026-11-19, on top of `1ae4222`; read this first)

`B` = `scratchpad/b12`; work files in `B/a2c/fr4/` (check runs `c-j1` = `f9bb447`, `c-x4f` = `b82d584`, `c-x4d` /
`c-x4e` = dropped forms of the X4 rule; chips off their lines over every set by `tools/det3.sh` / `det3cmp.py` in
`det-j1`, `det-x4f`; per-file verdict columns over every set by `tools/setcmp.py`; sequential timings over the
generated sets by `tools/tset.sh` / `tcmp.py` in `t-start2`, `t-x4f`; the verifier's big inputs timed in `tb/`;
pictures `pic/`).

**Two new commits, both green** (guard incl. rustdoc; the three checks against the package start `d99c073` and
against the commit before; chips over every set; the verifier's generated sets; default build byte-identical, the
25 corpus SVGs equal `runs/pre-b12a2c-squeeze`, 0 differ):

| commit | what | finding |
|---|---|---|
| `f9bb447` | `edgepass::stages` weighs the pin set by a local `Tally` of the lines the two drawings disagree on, not `quality::measure`; ports once, ortho twice, the later stages twice only where the ortho drawings differ; a pin's head beside an entry counts however near; chips judged one by one (`labels::chip_off_line`, own slack `CHIP_OFF_SLACK`) | #2, #3, #5 |
| `b82d584` | X4 levels down: where a later sibling is held, the blocks that start before it move downstream to it (no fan, no link leaving downstream, `LEVEL_DOWN_CLEAR` below, no line in a gap narrower than `LEVEL_DOWN_CORRIDOR` between them) | #1 (item 4) |

**`compare.sh` at `b82d584` against `d99c073`: VERDICT no worse** (cells, panics, dirs); against `1ae4222`: no
worse. Local static A2 **70 -> 65 must-fail lines** (the `sib_stagger` line of the analog and of perm-a, c, d, e;
perm-b keeps its, see #1), routed 18 (unchanged), gate messages 8 failing with the same names and no worse line.
Chips more than 3 px off their lines: no chip newly off or farther, in any set, against `1ae4222` or the start.
**Checkpoint A2 is still NOT met**: 65 lines, and DEFECT_BASELINE's static column as before (`lat_wiggles` 33 > 15,
`lat_backtrack` 10599.64 > 1870, `band_pairs` 204 > 132, `band_len` 91602 > 74730, `end_kink` 1 > 0).

### The verifier's findings, one by one

1. **A2 gate / DEFECT_BASELINE / items 1-2 (blocking): 70 -> 65, not met.**
   - Worked this round, worst-first among the lines the pictures show: `sib_stagger` (six lines, one cause). The
     staging block is held by X4's first rule (a long link enters it on its second row), and nothing moved the
     standby block instead. `b82d584` levels the earlier block down to the held one (picture
     `pic/analog-band.png`, and the synthetic `pic/ld3-level-down.png`). A first form without the corridor rule took
     all six lines (`c-x4d`, 70 -> 64) but on perm-b moved two lines in the 41 px left between the two frames to the
     moved frame's side (`DNS -> PSPROD`'s leg 1005.1 -> 991.3: `router::repair`'s lattice column next to the moved
     corner, then the nudge), and the `example.org proxied` chip lost every clean spot on its line: 0 -> 69.8 px
     beside it (`pic/permb-site.png`) and perm-b `lbl_in_end` NEW. Under ruling 1 that is a regression, so perm-b
     keeps its line (`LEVEL_DOWN_CORRIDOR`: no line between the two in a gap under eight clear gaps).
   - Traced, not built: `bends_max` 6 (analog family, seven lines) is `DNS -> PSDEV` / `DNS -> PSPROD`: the chain
     runs down its target's column, jogs past the title chip into the displaced column (X2.5) and back under the
     chip (`pic/analog-psdev.png`: 6 turns). The labelled chain keeps the column its label dummy reserved
     (b12a2b-base item 6's finding), so the fix is the gap-step merge with the label slot moved to the displaced
     column, in the order/B-K placement (the chain's upper dummies aligned to the title-row dummy, not the target).
     Not reached this round.
   - Unchanged since fr2's table: the squeeze lines (compact / area / straight / detour; the D6 stop stands: no
     form no worse on every cell and test), the public family `lateral_x` 2 (`CART -> QUEUE` into ORD's left wall,
     `[WEB] -> METRICS` into OBS's: QUEUE's top lies under ORD's title chip and its one free column holds the
     webhooks entry; METRICS's top holds three scrape pins; lateral class e2 / title-bound room, decision 6 keeps
     lateral (c) out of these packages), labels on the analog family, dense-lr / dense-td / bench-75 (LR squeeze,
     hub fan). Item 2 (chains straight, DEFECT_BASELINE static): causes as fr2 traced them (fan lines round a block
     holding neither end; `end_kink` is `router::repair`'s stub on bench-75), not built.
2. **Block-mode static perf cliff (blocking): fixed, `f9bb447`.** `quality::measure` is gone from the hot path.
   `stages` runs the ports stage once and the ortho stage twice (with and without the set), and the later stages on
   both drawings only where the two ortho drawings differ; the two finished drawings are weighed by `Tally`, which
   samples only the lines near the edges the drawings disagree on (bounding boxes from control points; straight
   legs collapsed; squared distances, box-pruned segment pairs). Suite-on `quality`, fastest of 5, two runs
   (start `d99c073` | `1ae4222` | now): bench-75 + n121 41 / 32 | 462 / 424 | 50 / 50 ms; dense-td + n121 85 / 92 |
   519 / 519 | 85 / 86 ms; dense-lr + r183 73 / 82 | 153 / 163 | 78 / 79 ms; advanced + n121 9 / 6 | 18 / 17 |
   10 / 10 ms (`tb/`). The 1062 generated block-mode inputs, sequential: total 2252 -> 2212 ms against the start;
   14 are more than 2x slower and +0.3 ms (at most 3.3x, 0.21 -> 0.70 ms; the largest +3.1 ms, r183-TD 2.80 ->
   5.88), all inputs where the set moves a pin and the later stages run twice (`t-x4f`; the verifier counted 91).
   A cold `route()` in block mode is one static layout and pays the same.
3. **n121-TD's pin on the entry: the set is no longer kept.** Stage dump: the set deals `q -> B`'s pin at 309.8
   and `a2 -> b1`'s displaced column at 318.4; `ports::respace` then respaces b1's top and drags the column to
   311.4, 1.6 px from the pin. The tally counts a pin's head within `EDGE_SPACING` of another line, and two lines
   side by side over half an arrowhead, as one line on another however near (`CLOSE_RUN`), so the set no longer
   reads as taking an overlap away; n121-TD draws as at the package start (its 0.4 px pair, defects 2, fan_x 3),
   so its claim in fr3 is withdrawn. The same rule refuses no other set the harness counted as better: against
   `1ae4222` over every generated set only n121 changes (back to the start) and sw72 `b3-TD` moves 1.3 px nearer at a
   39 px gap (the tally measures against plumb lines, which the harness's legacy prune skips: set 18.3 px against
   12.3 px, harness 37.7 against 39.0; pictures equal to the eye, `pic/b3-TD.png`). `r223-TD` keeps the set: its
   apart drawing has three such pairs (one a 0.0 px corner touch), the set two, one of them a 65 px run 1.6 px
   apart that the harness does not see (`pic/r223-TD.png`). Test
   `a_pins_head_beside_an_entry_counts_as_one_line_on_another_however_near`.
4. **g7247 `audit` 0 -> 9.0 px and g7248 `pack` 0 -> 21.1 px: not fixed (non-blocking).** Unchanged since fr3's
   trace: two chips of one fan swap which sits on its line (g7247) and `queue`'s chip, back on its line outside its
   end block since `5e30a15`, takes `pack`'s only clean spot (g7248). Fitting both needs the label stage to place the
   two labels together; not built.
5. **`chips_off_lines` summed, `GAP_SLACK` reused: fixed, `f9bb447`.** The set is kept only where no chip of a
   changed edge lies farther off its own line than in the other drawing (`chips_no_farther`, chip by chip, a chip
   within `ON_LINE_TOL` counting as on it), with its own slack `CHIP_OFF_SLACK` (rounding). Test
   `a_chip_moving_off_its_line_is_not_paid_for_by_another_coming_back`.
6. **v4-pin-and-labels-TD "back to its start drawing": corrected.** Its crossings, wall entries and side ends are as
   at the start, but the `audit` chip sits about 7 px lower and `chip_gap_min` is 5.68 against 10.25 at the start
   (the verifier's `pic/v4.png`); r183-TD differs from the start by a 0.1 px rounding only. Unchanged this round.
7. **cycle2-TD `chip_crowd` 0 -> 1 (informational):** as fr2 reported (the `r3` chip in the gap between the frames,
   8.9 px from the `request` run, as mermaid sets it). Unchanged.
8. **Item 3 leftovers** (n070-TD overlap, r170-TD tight pair, twin-n-TD `lateral_x` 3, r183-TD's two overlaps):
   unchanged; causes as fr3 traced (n070: the nudge, its fix breaks a casing test whose input only the lead can
   change).

### The 65 lines at `b82d584` (`c-x4f/local/judge-A2.txt`)

analog 11, v2 8, perm-a / b / c / d 5 each, perm-e 4, the public fixture and its four reorderings 3 each, public BT
1, cycle2 1, dense-lr 3, dense-td 1, bench-75 1. Causes as in fr2's table below, less the `sib_stagger` lines of the
analog and perm-a / c / d / e (gone); perm-b's `sib_stagger` is held by the corridor rule (above).

### Perf, guard, disk

- Guard green on both commits (fmt, clippy, tests, doctests, MSRV 1.75, wasm, rustdoc `-D warnings`).
- `perf.sh` (default build, rustc idle, load 1.0-2.1), four runs `b12a2c-fr4-a` .. `-d`. Run c is within every PLAN
  budget against `runs/baseline` (bench-75 drag 20.39 against 20.22, route 11.27 against 12.61, static scene 41.5
  against 31.0 (2x = 61.9); dense-td route 6.90 against 7.36, drag 12.60 against 14.24; small files within 2x).
  Runs a, b, d show the box's single spikes seen in every round: sub-ms static scenes over 2x (stress 0.28-0.33
  against 0.11, subgraph 0.23-0.25 against 0.08, wide-labels 0.75-0.82 against 0.39), dense-td route 9.31 once and
  drag 23.6 once. The default build runs none of the changed code (`stages` takes its old path without a block
  layout; X4 runs only in block layouts) and is byte-identical; an A/B of the start, `1ae4222` and the head on the
  same corpus in alternation (`ab/`) shows the same spread on all three (wide-labels 0.56-0.78 at the start).
- Disk: 6.9 GB free on `/`; `fr4/` 480 MB (check runs and one target dir kept for the verifier, safe to delete).

