## b12a2c-squeeze, fix round 2 (2026-11-19, on top of `5e30a15`; read this first)

`B` = `scratchpad/b12`; work files of this round in `B/a2c/fr2/` (check runs `c-cr3` = `aa1c980`, `c-p3` = `0f2a838`,
`c-lr2` = `226b0f0`, `c-nest` = `9a0e27e`, `c-down` = `704b118`; the verifier's generated sets regenerated under
`sets/` and judged by `tools/evalsets.sh` / `tools/evalcmp.py`; chips off their line by `tools/detall.sh` /
`tools/detcmp.py`; pictures `pic/`). Checks are against the package start `d99c073` (`tools-a2b/cand-d99c073`),
suite-on, as decision 2 asks; each commit was also compared with the one before it.

**Five new commits, all green** (guard incl. rustdoc, the three checks, default build byte-identical on the 25
corpus SVGs vs `runs/pre-b12a2c-squeeze`):

| commit | what | verifier finding / item |
|---|---|---|
| `aa1c980` | a block layout's chip keeps `CLEAR_GAP` from other lines where its own line leaves it room (label stage) | #3 (public BT `deploy` / `write`), item 4 (`lbl_crowd`) |
| `0f2a838` | a frame's pins and its displaced title-row entries are dealt as one set of columns (`ortho::TitleSides`) | item 3 (pin-on-entry, b9 / b11, nested-n) |
| `226b0f0` | X4 in LR / RL (title-length growth read) | item 1/4: X4's LR / RL half |
| `9a0e27e` | X4 at any depth, outermost first | X4 completion: siblings at any depth |
| `704b118` | X4's downstream / last-row rule | X4 completion: last-row rule |

**`compare.sh` at `704b118` against `d99c073`: VERDICT no worse** (cells, panics, dirs). Local static A2 **86 -> 70**
must-fail lines of 444 (fr1 ended at 75; `--due A1` 0); routed 62 cells 19 -> 18, worse 0 (checkpoint R's rows
held); gate messages 8 -> 8 failing, same names, no worse line; direction copies (332 + 32) no worse in every
verdict column, static and routed. **Checkpoint A2 is still NOT met**: 70 must-fail lines remain, and
DEFECT_BASELINE's static column is over on the same five fields as at the start (`lat_wiggles` 33 > 15,
`lat_backtrack` 10599.64 > 1870, `band_pairs` 204 > 132, `band_len` 91602 > 74730, `end_kink` 1 > 0); their causes
are traced below.

### The verifier's findings, one by one

1. **A2 gate / DEFECT_BASELINE (blocking): not met.** 75 -> 70 this round (the five public `lbl_crowd` lines gone,
   `aa1c980`; better inside failing: the analog's and its permutations' `lbl_crowd` 4 -> 2 (perm-b 8 -> 5), v2
   `lbl_crowd` 4 -> 2 and `lbl_endpx_mean` 381 -> 361). The 70 per fixture with causes are in the table below; the
   DEFECT_BASELINE fields under item 2.
2. **Coverage (blocking): items 2 and 3 and X4's LR / RL half attempted this round.**
   - Item 1 (the squeeze): not retried. The lead's note of 08:30 lets fix rounds retry it "only in a form that is no
     worse on every cell and test; otherwise spend the rounds on the remaining must-fail lines"; fr1's measured
     forms (`fr1/squeeze-fr1.diff`: 27-35 worse static cells in every form) give no such form, so this round went
     to the lines and items below. Its causes stand as fr1 reported them (chips the label stage places in gaps the
     squeeze closes; members moved off their link columns).
   - Item 2 (chains straight; DEFECT_BASELINE static): traced, one experiment, nothing committed (below).
   - Item 3 (b12a2b-base leftovers): `0f2a838` fixes b9-TD, b11-TD and nested-n-TD; n070, twin-n, r170 traced
     (below).
   - X4: LR / RL (`226b0f0`), siblings at any depth (`9a0e27e`), the downstream / last-row rule (`704b118`). Each is
     a no-op on every repository drawing (their bands are held by `630096c`'s rules) and proven on new inputs by its
     test. dense-lr's `sib_stagger` 2 stays: its top-level pair is held by two of those rules (a link into a later
     row, one node feeding both first rows; traced in `level_sibling_frames`), its nested pair by the same rules
     after `9a0e27e`.
3. **The public fixture drawn BT, chip crowding (unreported regression): fixed, `aa1c980`.** `chip_crowd` 4 -> 1 and
   `chip_gap_min` 3.09 -> 10.0 on all eight BT copies (start: 3 / 9.72; the one left is the Data tier's `scrape`,
   10 px, as at the start). Causes, traced in the SVGs and by commit: (a) `deploy` (REG -> WEB): after `22dac44`
   levelled the build pipeline its line grew and its middle fell on the thick `deploy` run; the label stage took
   the clean spot nearest the middle, and "clean" meant no stroke within `STROKE_PAD` (3 px) of the chip, 1 px of
   air past a 3.5 px stroke; (b) the two `write` chips: since `7208f50` (not `5e30a15`: `bin-c1` already draws it)
   SHIP's `write` sits outside Order processing above the ledger's `write` run, and the ledger's chip was re-placed
   5.8 px nearer the `scrape` line (3.9 px). Fix: in a block layout a spot ranks, after its tier, by the foreign
   strokes within `CLEAR_GAP` of its chip, so a clean chip that close is searched on for a clean spot ON its line
   with that air; never beside the line (a beside spot ranks as before, so no chip leaves its line for air), not for
   a compact branch point's fan. Pictures `fr2/pic/public-bt-final.png` (mermaid | start | fr1 | now) and
   `public-bt-final-crop.png`: `deploy` sits clear above the thick run as at the start, both `write` chips have air.
   Over the 332 + 32 copies `chip_crowd` falls on 71 copies and rises on one, `cycle2` TD 0 -> 1: its `r3` left
   its end block in `7208f50` (not this round) for the gap between Front and Back, where no spot of its line keeps
   both the air and `LABEL_GAP` from the frame (`fr2/pic/crowd-cycle2-TD.png`; mermaid sets `r3` in that gap too).
   The smaller trades the verifier named (dense-td-BT `chip_crowd` 7 -> 10, ratio* +1) are gone or better with the
   rule: no copy but `cycle2` TD is above the start.
4. **D5 (commit messages of `630096c`, `5e30a15`; `7208f50` before them): needs the lead's squash**; cannot be
   fixed without a history rewrite. This round's five messages name no input of the local family and quote none of
   its numbers (repository fixtures, the public fixture and the verifier's generated sets only).
5. **Lead ruling 1 at its own threshold: measured.** Chips more than 3 px from their own drawn path, every chip of
   the 95 local inputs and the 332 + 32 block-mode copies, static and routed, `d99c073` against `704b118`
   (`fr2/det-start`, `fr2/det-down`): local 68 -> 61 static and 68 -> 61 routed, dirs 106 -> 98 and 108 -> 100,
   dirsx 4 -> 4 and 6 -> 6; **no chip newly off its line, none farther than at the start** (the per-label key is
   edge and text). A first form of `aa1c980` let a spot beside the line rank by its air too and carried five
   chips that already sat beside their lines farther out (9 -> 19.5 px and the like); it was changed before the
   commit.

### Item 2: chains straight across levels (DEFECT_BASELINE static) - traced, not committed

- The 33 static `lat_wiggles` are bench-75 18, dense-td 11, dense-lr 3, wide-labels 1 (the last one also in the
  default build). Every block-mode one is a hub fan's or a sink fan's line going round a block that lies between
  its two ends: bench-75's lines into `logs` / `metrics` / `ext` (the ops zone and the external API sit right of the
  app and data zones), dense-td's `n16` fan into the bottom block, which lies under the `adipiscing` block that
  holds neither end (`n16 -> n41 / n43 / n44`, 1156-1416 px of sideways travel each), dense-lr's `n12` fan
  (`fr2/pic/bench-75.png`, `dense-td.png`). With `xframe_runs` 0 (the A1 rule that no run crosses a block holding
  neither end) the only straight column is through that block, so these lines turn twice. Fixing them is the
  ordering's or the squeeze's work (put the sink block where the fan does not need to go round), not a chain
  straightening; fr1's `FMX_STR` (straighten the chains across levels) moved one wiggle and made the bench-75 BT copy
  worse.
- `end_kink` 1 is bench-75's `svc1 -> logs`: traced stage by stage (`fr2/dbg-dump.diff`, an `FMDBG_EDGE` dump not
  committed), `router::repair` reroutes it into logs' bottom from a track 12.5 px below the side (its stub is
  `clear + STUB_EXTRA`), and the arrowhead sits on the corner (51.6 deg). Experiment `fr2/exp-stub.diff` (a glyph
  end's stub at least head + `CORNER_R` in a block layout): `end_kink` 1 -> 0 but bench-75 defects 0 -> 3 and
  `fan_x` 4 -> 15; not committed.
- `lat_backtrack`, `band_pairs`, `band_len` are the class 4.5 lets rise with a named cause; their sources are the
  same fans (parallel runs round the blocks).

### Item 3: b12a2b-base leftovers

Measured on the lead note's inputs and on the verifier's generated sets of b12a2b-base (rnd 300, nr 250, nt 150,
sw72 72, tw 33; regenerated from the verifier's `gen2b.py` / `gentw.py` with their seeds), static:

| input / set | `7122a53` | start `d99c073` | `0f2a838` |
|---|---|---|---|
| b9-TD, b11-TD | defects 0, crossings 0, gap 14.6 | defects 1 (overlap), crossings 4, gap 3.39 | defects 0, crossings 0, gap 18.9 / 19.9, `lateral_x` 1 -> 2 |
| nested-n-TD | 0, gap 13.79 | 1 (pins 5.91 px apart) | 0, gap 19.41 |
| n070-TD | 0 | 1 (overlap) | 1 (unchanged) |
| twin-n-TD `lateral_x` | 2 | 3 | 3 |
| r170-TD | tight 1 | tight 1 | tight 1 |
| rnd defects / crossings / `lateral_x` | | 28 / 221 / 118 | 21 / 220 / 112 |
| nt | | 13 / 166 / 69 | 11 / 158 / 63 |
| sw72 | | 2 / 69 / 58 | 0 / 62 / 57 |
| tw | | 3 / 31 / 14 | 2 / 33 / 11 |

- **Cause (proven by a stage dump on b9-TD):** after `ortho`, the pin of `client -> G` stands at 313.3 and the
  displaced `hub -> queue` crosses the title side at 314.7; `TitleSides::of` dealt the displaced columns off the
  side's crossings, and a pin ends ON the side, so no crossing finds it. `router::repair` then rerouted `client -> G`
  onto the hub's track (y 114), and the nudge moved the hub 1.9 px: one overlap, two arrowheads together. fr1's
  "likely, not proven" is now proven and fixed: pins and displaced columns are one set, in the order their lines
  come from; a pin moves along the side where a column needs its place (`0f2a838`). Counted as taken columns alone
  (the earlier trial) the pins pushed every column past all pins and across the lines into them (r240: crossings
  0 -> 5); the order rule removes that.
- **Worse per file on the generated sets** (none in the three checks: no repository fixture has the case):
  nt n078 defects 0 -> 1 (`p -> B`'s pin, which the ports stage set 4 px off `p -> b1`'s entry and does not move,
  used to be rerouted by `repair` as a side effect; now it is not), nt n121 `min_edge_gap` 13.65 -> 1.65 (a dense
  side of five pins and entries: `fr2/pic/pins-n121-TD.png`), a few `x_drawn` +1 / +2 (rnd 7 files up, 10 down; nt 4
  up, 12 down). Reported for the lead; the totals improve in every set but tw's crossings (31 -> 33).
- **n070-TD**: its overlap is `q -> b0`'s displaced column and `q -> B`'s pin, 14 px apart after `ortho` (fr1's
  trace); a later stage lays them together. Not fixed. **twin-n-TD** `lateral_x` 3 and **r170-TD**'s tight pair:
  unchanged, not traced further this round. **n109-BT**: fixed in `c9eb06b` (b12a2b-squeeze), unchanged since.

### The 70 lines at `704b118` (`fr2/c-down/local/judge-A2.txt`)

| fixture | lines | open rows | cause |
|---|---|---|---|
| analog | 12 | `area`, `bends_max` 6, `compact_max` 9.71, `compact_mean` 5.4, `detour_max` 1.72, `lateral_x` 1, `lbl_crowd` 2, `lbl_gap_min` 0, `lbl_own_gap_max` 16.5, `lbl_struck` 1, `sib_stagger` 1, `straight_max` | squeeze (compact, area, straight, detour; D6); labels: a 361 px chip on a run crossed by many verticals is struck (no clean spot on its line), two chips beside their lines (`lbl_own_gap_max`) and one crowded at 3.8 px; X4 held (its staging block is entered past its first row); lateral (X3 / e4) |
| v2 | 8 | `area`, `bends_max` 6, `compact_*`, `lbl_crowd` 2, `lbl_endpx_mean` 361, `lbl_in_end` 1 (ruling trade), `lbl_struck` 3 | as the analog |
| perm-a / c / d | 6 each | `bends_max` 6, `compact_max`, `lateral_x` 1, `lbl_crowd` 2, `lbl_struck` 1, `sib_stagger` 1 | as the analog |
| perm-b / e | 5 each | `bends_max` 6, `compact_max`, `lateral_x` 2 / 1, `lbl_crowd` 5 / 2, `sib_stagger` 1 | as the analog |
| public fixture and its 4 reorderings | 3 each | `compact_max` 19.97, `compact_mean` 7.26, `lateral_x` 2 | squeeze (Observability 815 px wide, D6); lateral |
| public BT | 1 | `lateral_x` 3 | lateral (X3 / e4) |
| cycle2 | 1 | `lateral_x` 2 | lateral (X3) |
| dense-lr | 3 | `compact_max` 43.03, `lateral_x` 5, `sib_stagger` 2 | squeeze; X4 held (above); lateral |
| dense-td | 1 | `lateral_x` 6 | hub fan `n16` |
| bench-75 | 1 | `compact_max` 71.64 | LR / RL squeeze (gap floor, fan) |

### Perf, guard, disk

- Guard green on every commit (fmt, clippy, tests, doctests, MSRV 1.75, wasm, rustdoc `-D warnings`).
- `perf.sh` (default build; rustc idle, load 1.5-2.4 from the other package's runs) three runs `b12a2c-fr2-a/b/c`:
  run b within every PLAN budget against `runs/baseline`; runs a and c show single sub-ms `scene_ms` spikes on small
  files (back-edge 0.22, stress 0.34, demo 0.17 ms) that the other runs do not. The default build runs none of the
  changed code but a few empty lookups (the label stage's `near` returns 0 without block layout; `TitleSides` is
  empty) and is byte-identical. Block-mode drags (`quality --drag`, bench-75, public, dense-lr, dense-td), two runs
  each, start against now: worst drag 8.1-9.4 vs 8.4-9.3 ms (bench-75), 8.9-10.4 vs 10.2-12.7 ms (dense-td; the
  start's own two runs differ by 1.4 ms), within the box's noise.
- Disk: 8.0 GB free on `/`; `fr2/` holds 470 MB (one target dir and the check runs), kept for the verifier.
