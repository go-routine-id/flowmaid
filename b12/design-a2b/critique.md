# Critique of the five design-a2b studies (skeptic)

`B` = `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12`,
`K` = `B/design-a2b/critic`. Base: `7122a53` (local judge `--due A2` 91 must-fail, `--due A1` 0, suite-on 14 failing
tests). Nothing in `/home/user/flowmaid`, `B/metrics` or `B/local` was changed (live judge files `cmp`-equal to the
copies in `B/design-a2b/bends/orig`; labels' judge copy `cmp`-equal to the live one). My worktree and target dir are
removed; HEAD is still `7122a53`, `git status` is clean.

## 0. Bottom line

1. **Every study's headline judge number reproduces** (recomputed from its own SVGs with the live yardstick):
   squeeze 70 / 67 / 99 / 72, labels 78 / 75 / 70 / 71, lateral 90 / 88 / 84, bends 89, regress 91. A1 stays 0
   everywhere.
2. **Three of the five studies hid regressions** the same way the squeeze study says round 3 did: they compared
   must-fail line counts and test names, not cell values, gate messages or the direction copies.
   - squeeze: 23 cells inside already-failing lines get worse. Among them the analog family's `lbl_crowd` goes 4 -> 8,
     `lbl_in_end` 1 -> 2/3 and `lateral_x` 1 -> 2/3. Under the corrected bends counter the worst edge goes 6 -> 8/9.
     The routed `defect_baseline` fields worsen too. On the 332 direction copies routed `fan_x` goes 183 -> 217, with
     every public-fixture copy, TD **and BT**, going 0 -> 1. Static defects go 41 -> 44.
   - lateral: "X1 / X2: no test changes" is false at the message level. Inside the already-failing
     `block_fixtures_are_not_drawn_worse`, X1 adds static `defects 1 / overlap_pairs 1` on two public permutations,
     X2 adds the same on `nested-cases`, and X3 takes `sub-nested-self` defects 2 -> 4.
   - labels: the variant it recommends (p5-c0.34) **breaks the A1-held `a_free_pin_keeps_off_a_members_exit_past_the_chip`**:
     dense-td's static `min_edge_gap` is 10.27, below its 12.1 gap floor. The study ran the suite only with p6.
3. **One diagnosis is wrong as stated (labels).** The two dense-td hub-fan failures do not come from keeping tracks
   off band chips. They come from the prototype's track exclusion around **chain labels on their dummy rows**.
   Switching only that part off makes both hub-fan tests and the free-pin test pass, and it changes **no** judge line
   (p5: 70 -> 70; p6: 78 -> 78, dense-td `lateral_x` 8 -> 6).
4. **The real conflict is squeeze x G3, then squeeze x X2.5.**
   - Squeeze plus G3 together give 76, worse than either alone (squeeze 72, G3 70). G3 loses its label gains on the
     squeezed breadths: analog crowd 0 -> 7, struck 1, `lbl_gap_min` 0. Its band depth pushes the analog and v2
     `area` back over their musts.
   - Together they also break 4 more A1-held tests: both dense-td hub fans, `casing` (routed 4) and
     `edges_keep_off_frame_borders` (routed `border_len` 2130).
   - On squeezed breadths, lateral's X2.5 breadth bound prints "no room" for the analog runs and changes nothing: the
     squeeze removed the column X2.5 needs.
   - Regress, lateral X1-X3 and G3 (without the dummy-row exclusion) compose additively: 91 -> **67** must-fail lines,
     with one extra test, lateral's own `frames_agree`.
5. **Two studies' items are one item.** Lateral X1 (the ports stage cuts at `CHIP_CLEAR`) is exactly regress §6's "make
   the two limits one". On lr BT it removes the 2.2 px S-jog: `cart -> pager` runs straight down at 321.1 (measured).
   X1 also causes lateral's e2 collision as a **static overlap** (`[WEB]>METRICS` on `OPS>[OBS]`'s pin column 328.6;
   `min_edge_gap` 12.72 -> 0 on two public TD copies). So X1 must land together with e2, not alone.
6. **Bends: the fix measures truthfully and only tightens.** I re-ran it: selftest 813, 91 -> 89, the threshold diff
   holds only tightenings, and the corrected counter is near scene truth (start: 1855 counted against 1987 true
   turns). Under it, the squeeze's bends regressions are real turns, not counter noise: public worst edge 4 -> 6,
   analog family 6 -> 8/9.

## 1. What I re-ran (all in `K`)

- `rejudge.sh`: recompute `metrics.tsv` from a folder of SVGs with the live and with the corrected (`bends/fixed`)
  blockmetrics, then judge each against its own tables (live: `B/metrics`; corrected: `bends/data/fix-*` +
  `bends/thresholds-corrected`). Outputs are in `K/j/<label>/{live,fix}-{A1,A2}.txt`.
- `cells.py base variant`: per (input, metric) must line, it prints new, gone, **worse inside failing** and better
  inside failing.
- `panics.py`: diffs the panic messages of the gate tests (`quality_gate`, `defect_baseline_gate`,
  `block_fixtures_are_not_drawn_worse`, `a_one_px_drag…`, `channels_keep…`) against `B/work/a2i/ton-split.txt`.
- **One combined build** (`K/stack-with-knobs.diff`, against 7122a53): regress `proto.diff`, lateral
  `lateral-experiments.diff`, labels `proto.diff` and squeeze `proto.diff` (3-way applied, no conflicts), plus env
  knobs.
  - The knobs: `NORFIX` (old `split_by_room`), `SQOFF` (no `block_compact`, no X4), `FM_G3_OFF` (fully off; the
    labels study's own switch leaves 13 of 95 drawings changed), `FM_G3_NOROWS` / `FM_G3_NOCHIPS` / `FM_G3_NOEXCL`
    (the prototype's foreign-track exclusions), and lateral's `FMX1..4`.
  - With every knob off it is byte-identical to `fm-on-split` on all 95 local inputs and to `qd-split.tsv` on the 332
    direction copies.
  - Suite-on lib and quality tests: `K/t/<combo>.{txt,fail}`. Direction copies: `K/q/<combo>.tsv`, summary in
    `K/q/summary.txt`.
- Pictures:
  - `K/pic/labels-vs-squeeze-labels/infra-overview.pair.png`: G3 p5 alone | G3 p5 with the squeeze.
  - `K/pic/lrbt-x1/lr-BT.pair.png` (lr BT, 7122a53 | lateral X1).
- Spot check of regress on bulkst 9a94998d / 8ab502b3: `ortho_edges` 7 -> 5 and `x_drawn` / `fan_x` 1 -> 0, as
  claimed.

### 1.1 Local judge per combination (`--due A2`, live / corrected-bends; A1 = 0 in every row)

R = regress, X = lateral X1+X2+X3, S = squeeze prototype, L5 / L6 = labels p5-c0.34 / p6, nr = without the dummy-row
exclusion, X4 = lateral X2.5 prototype.

| combo | A2 | combo | A2 |
|---|---|---|---|
| 7122a53 | 91 / 89 | RXS | 69 / 67 |
| R | 91 / 89 | L5 = L5nr | 70 / 68 |
| X | 88 / 86 | RXL5 = **RXL5nr** | **67 / 65** |
| S | 72 / 70 | RSL5 = RSL5nr | **76 / 74** |
| L6 = L6nr | 78 / 76 | RXSL5 = RXSL5nr = RXSL5X4 | 73 / 71 |

### 1.2 Suite-on failing tests beyond 7122a53's 14

This table leaves out the tests that only fail because a piece is switched off: `a_block_is_squeezed…` with SQOFF and
the regress test with NORFIX.

| combo | added A1-held failures |
|---|---|
| R | none |
| X (RX) | `frames_agree…` (lateral's route_title_jogs half) |
| L6 | both dense-td hub-fan tests |
| L6nr | none |
| L5 | `a_free_pin_keeps_off_a_members_exit_past_the_chip` (dense-td static `min_edge_gap` 10.27 < 12.1) |
| L5nr | **none** |
| S | `a_hub_fan_leaves_in_target_order` (`advanced_showcase` starts passing) |
| RXS | `a_hub_fan…`, `frames_agree…` |
| RXL5nr | `frames_agree…` only |
| RSL5 / RSL5nr | `a_hub_fan…`, `a_hubs_fan_keeps_its_order…`, `an_end_of_a_title_row_member…`, `casing::…one_role…` (routed 4), `edges_keep_off_frame_borders` (routed border_len 2130) |
| RXSL5(nr) | the RSL5 list + `frames_agree…` |

### 1.3 The 332 direction copies (none of squeeze / labels / lateral measured them)

Static and routed totals, 7122a53 -> variant (`K/q/summary.txt`):

| combo | static | routed |
|---|---|---|
| R | unchanged | unchanged |
| X1 alone | defects 41 -> 43 (public perms TD: overlap, `min_edge_gap` 12.72 -> 0); notes/spread-bottom BT `fan_x` 0 -> 2 | unchanged |
| RX | defects 41 -> 48 (+ nested-cases TD, sub-nested-self TD/BT 2 -> 4); bench-75 TD `fan_x` 7 -> 9; bench-75 BT `lateral_x` 5 -> 8 | `edge_chip` 78 -> 85 |
| L5nr | public family TD `lateral_x` 3 -> 4 (8 copies) | dense-lr TD `fan_x` 0 -> 2 |
| S | defects 41 -> 44 (bench-75 TD 1 -> 5, nested-composite TD/BT 0 -> 1); `x_drawn` 1283 -> 1335; 12 copies lose `min_edge_gap` (ratio2x4 TD 40 -> 12.7, turned-blocks TD 40 -> 12.5) | defects 169 -> 204; `fan_x` 183 -> 217 (every blocks-overview copy, TD and BT, 0 -> 1); `lateral_x` 418 -> 500 |
| RXL5nr | defects 41 -> 48 (lateral's) | ~flat |

## 2. Verdict per study

### 2.1 squeeze: diagnosis proven, proposal not sound as written

Proven:
- The component table: X1 alone gives 99, its parts break bench-75's gap and fan, lr BT and casing 34.
- The traces for hub5 (weight 8) and lr BT (title-bound frame).
- The route() mechanism for the public fixture's routed fan.

All of it reproduces in my build: S = 72, `a_hub_fan_leaves_in_target_order` fails, `advanced_showcase` passes.

Problems:
- **Hidden worse cells.** "31 fixed, 12 added" omits **23 cells that get worse inside already-failing lines**:
  - analog / perm-a / c / d `lbl_crowd` 4 -> 8;
  - `lbl_in_end` 1 -> 2 (analog, perms) and 1 -> 3 (v2, perm-e);
  - `lateral_x` 1 -> 2 on the analog and perm-a / c / d / e, 1 -> 3 on v2. These are new lateral edges: `PSPROD>TG` on
    the analog; `R1>ARGO`, `LANE>GITLAB` and `ARGO>GITLAB` on v2;
  - `bends_max` 13 -> 14 / 16. With the corrected counter this is 6 -> 8 / 9, real turns on `LANE>GITLAB` and
    `PSPROD>TG`.

  Also unreported:
  - `defect_baseline` routed `lat_backtrack` 3856 -> 4754 and `lat_wiggles` 23 -> 24;
  - crossings: v2 14 -> 16 (must 17), wide-frames 2 -> 4, dense-lr 17 -> 18.
- **No direction copies were measured** (table 1.3). The routed fan on the public fixture also appears in the **BT**
  copies, so R3 is needed for BT as well. Bench-75's TD copy goes from 1 to 5 static defects.
- **It removes the room lateral's X2.5 needs.** With the squeeze on, X4 prints `no room (next 18 at -1600.3)` for the
  analog's two runs, and RXSL5X4 is byte-identical to RXSL5 except for 2 inputs. X2.5 (and G3's room) must be
  constraints **inside** `block_compact`, not moves after it.
- **The hang fix is not robust.** The `TitleSides::of` fix compares against `TRACK_PITCH - 1e-6`. That stops the
  observed loop, but once coordinates are large enough that one ulp exceeds 1e-6 (around 1e10), `o + TRACK_PITCH`
  can again land within the pitch of `o`, so the loop is still non-terminating in principle. A loop with guaranteed
  progress (one pass over the sorted `taken`, or a bounded step count) is the correct fix.
- **"R3 first" is a scope decision, not a measured necessity.** Pulling R3 forward is the only option the lead's rule
  leaves, because per-direction squeezing does not help: the hub-fan fixtures are TD and the BT copies fan too. But
  R3 is unbuilt and its 250-400 line size is an estimate. It needs the lead's call.

### 2.2 labels: offender diagnosis proven, hub-fan diagnosis wrong, proposal unsound until corrected

Proven: the four offender kinds, traced with `bm-near`, and the label-budget table (it reproduces: 78 / 75 / 70 / 71).

Problems:
- **The hub-fan cause is misattributed.** The cause is the dummy-row ("rows") exclusion. `FM_G3_NOROWS` fixes both
  hub-fan tests; `FM_G3_NOCHIPS` does not. The rows exclusion buys no judge line. Proposal step 2's "channel-level
  exclusion" therefore targets the wrong mechanism. Drop the dummy-row exclusion: `pair_sep` already holds
  `CLEAR_GAP` there, as the study's own code map says.
- **The recommended variant breaks an A1-held test.** p5-c0.34 breaks `a_free_pin_keeps_off…`, which holds dense-td's
  12.1 gap floor. Unreported, because the suite was run only with p6.
- **Unreported worse cells and gate rows:**
  - analog `area` 4.51 -> 5.58 M and v2 5.09 -> 5.57 M (both already failing);
  - dense-td `lateral_x` 6 -> 8 and crossings 42 -> 44 (p6);
  - wide-frames crossings 2 -> 3;
  - `defect_baseline` static `lat_wiggles` 33 -> 34, `band_pairs` +1;
  - direction copies: public TD `lateral_x` 3 -> 4 and dense-lr TD routed `fan_x` 0 -> 2.
- **It does not survive the squeeze** (§0 point 4; picture `K/pic/labels-vs-squeeze-labels`). "Judge G3 and B12.8
  together" understates this: G3's room tests and band depth have to be designed on squeezed breadths, or the squeeze
  has to keep G3's rooms.
- `FM_G3_OFF` is not a full off switch (13 / 95 drawings differ). This is experiment hygiene only.

### 2.3 bends: sound

The measuring bug is proven three ways: scene polylines, sampling invariance and the selftests. The threshold
re-derivation reproduces the live files byte for byte and **only tightens** (diff checked: 5 rules `bends_max` 8 -> 4,
public `bends` 110 -> 108; routed file equal). Fixing the counter without the new bounds would be the loosening; the
study says so and prevents it.

Caveats:
1. The corrected count is ~7 % **below** scene truth at the start (1855 / 1987; 150 edges under-counted). The cause
   is the unchanged 8 px `BEND_RUN_MIN` together with flowmaid's larger corner radius (a Z middle under ~32 px
   counts once). That favours flowmaid slightly against mermaid's exact counts. It is small (analog 53 against 55
   true) but should be written next to the rule.
2. Stage 3 is new logic and needs its own code review.
3. The rule question is sharper than the study states. The public family's bound 4 is below **0.38's, the start's
   and mermaid's own** public worst edge (6 / 6 / 6), not only below mermaid's analog 7.
4. The 7 remaining `bends_max` lines (6 > 4) can probably be fixed without any rule change (my inference, not
   measured). `DNS>PSDEV`'s 6 turns are 2 for the gap step to PSDEV's column plus 4 for the displacement. If the
   gap step went straight to the displaced column past DR's chip (G2: the displaced crossing column is the edge's
   column in the gap), the edge has 4 turns.

### 2.4 lateral: tracing credible, proposal unsound as ordered

The stage traces and probes are kept, and all its judge numbers reproduce (X1 / X2 / X3 90, X123 88, X1234 84).

Problems:
- **"No test changes" is false at the message level.** X1, X2 and X3 each add static violation rows inside the
  already-failing `block_fixtures_are_not_drawn_worse` (§0). X1 alone makes the e2 collision a static
  `overlap_pairs` defect. X3 also takes `defect_baseline` static `band_len` 91602 -> 98610.
- **Direction copies** (unmeasured by the study): RX adds bench-75 TD `fan_x` 7 -> 9, bench-75 BT `lateral_x`
  5 -> 8, spread-bottom BT `fan_x` 0 -> 2, and routed `edge_chip` +7.
- **The order is wrong.** "X1 alone first" would commit a static overlap. X1 must ship with e2 (pins and member entries
  as one column set), X2 must ship with its `nested-cases` overlap fixed, and X3 with `sub-nested-self` fixed and
  route_title_jogs updated.
- X4 cannot work after the squeeze (§2.1).
- The (c) proposal touches two tests. It is correctly left to the lead; the study argues from spec 2.8 rule 2 that the
  bold-title assertion pins a missing spec rule.

### 2.5 regress: sound

- The fix (`split_by_room` keyed on `plain.0`) is correct by construction: `jog()` builds `room` by clipping `plain`.
- My build shows zero change on the 95 local inputs, on the 332 direction copies (static and routed) and in every gate
  message.
- The bulk spot check matches.
- Its test fails at 7122a53 and passes with the fix.
- Risks are minor: the new fixture enters directory-walking tests, and its identity row must be re-taken from dev's
  build.
- It must land before G3's ortho work and lateral's e3, which change the same channel split.

## 3. Conflicts between the proposals (measured unless marked)

1. **Squeeze x G3.** 72 and 70 alone give 76 together. G3's crowd gains vanish, `area` fails again, and four more
   A1-held tests break (§0).
2. **Squeeze x lateral X2.5.** The squeeze deletes the free column past the chip; X4 then finds no room. Both rules
   (and the squeeze's own X2.2 keep rule) belong in one function, `block_compact`.
3. **Squeeze x labels on the analog family.** The squeeze alone doubles the analog's `lbl_crowd` (4 -> 8) and adds
   `lbl_off_max` 0.357 and v2 `lbl_own_gap_max` 9.5. G3 then has to fix more than it did on 7122a53.
4. **Squeeze x bends.** The public `bends_max` 4 -> 6 and the analog family 6 -> 8/9 are real turns under the
   corrected counter. Squeezed breadths force extra steps.
5. **Lateral X1 x regress §6.** These are the same change (measured on lr BT). X1 x lateral e2: X1 turns the pin
   collision into a static overlap.
6. **Lateral X4 x labels.** X4 costs the analog's `lbl_endpx_mean` (389 > 386). G3 p5's slide pays it back only if
   both land, measured together.
7. **Labels x regress.** Both change the channel split. With either key (room or plain) the hub-fan outcome is the
   same, because the cause is the dummy-row exclusion, so there is no functional conflict. There is a textual one,
   in the same function.
8. **bends bound 4 x spec 2.8 rule 2** (inferred). A displaced entry drawn as "step, displace, jog back" is 6 turns.
   Either the gap step merges into the displaced column, or the bound stays unmet by design.

## 4. Recommended build order

0. **Workspace first: the yardstick and the comparison discipline** (lead).
   - Install the corrected bends counter and the re-derived thresholds (tightening only).
   - Every later step is judged by:
     - per-cell no-worse (`cells.py`), not by line counts;
     - gate-message diffs (`panics.py`);
     - the 332 + 32 direction copies (`qrun.sh`) as well as the 95 local inputs.

   Reason: three studies missed regressions that only these show. The squeeze's bends regressions are visible only
   with the corrected counter. The bends rule question goes to the user but does not block, because the tighter bound
   applies meanwhile.
1. **regress `split_by_room`** (own commit). It is zero-risk on every measured set, fixes the bulk regressions, and
   G3 and e3 build on the same split.
2. **`TitleSides::of` termination fix**, with a progress-guaranteed loop and a unit test. No output change.
3. **Lateral X1 + e2 in one commit.** Measure lr BT's S-jog as part of it (closes regress §6). Reason: X1 alone adds
   a static overlap.
4. **Lateral X2, then X3 with its route_title_jogs half.** Each only once its `block_fixtures` rows (nested-cases,
   sub-nested-self) and the bench-75 TD/BT copy changes are gone. Measured gain: 3 lines.
5. **G3 static (labels p5 rules) without the dummy-row exclusion.** Measured with 1, 3-4 under it: RXL5nr = 67 / 65,
   no A1-held test added. Open items: the analog / v2 `area` increase (lead decision on whether a failing A2 row may
   worsen in an intermediate commit; otherwise G3 waits for step 8), public TD `lateral_x` 3 -> 4, dense-lr TD routed
   `fan_x` 0 -> 2.
6. **X2.5 as a breadth bound, before `TITLE_FIT_PASSES`.** Measure it with G3 present (endpx). Then the displaced-step
   merge for `bends_max` (inferred to bring 6 -> 4).
7. **R3 corridors in route()** (lead scope decision: B12.13 pulled forward). Both the public fixture and its BT copies
   need it before any squeeze.
8. **Squeeze, redesigned.**
   - Keep from the prototype: no X1, cross-border weight 2, the X2.2 keep rule.
   - Hold G3's chip rooms and X2.5's column past the chip as **constraints inside `block_compact`**.
   - Judge it against all 23 worse cells, the direction copies (routed `fan_x`, bench-75 TD defects, `min_edge_gap`
     drops) and `defect_baseline` static and routed.
9. Then e3 (into `place_tracks`), e4/e5 (dense-lr), perm-b / perm-e under D6, and the LR/RL squeeze.

## 5. Loosening risks to watch

- Fixing the bends counter without the re-derived bounds drops 7 `bends_max` lines (82 lines): a loosening. They must
  ship together. Reverting to bound 8 because mermaid fails 4 is a rule change only the user can make.
- The analog / v2 `bends` lines disappear because the counts halve. This is legitimate only because mermaid's
  reference is unchanged and exact; keep the scene-truth evidence with the commit.
- Lateral (c) would rewrite `jog_room_counts_only_what_crosses_the_title_side` and the bold-title assertion. That is
  allowed only as an exact restated bound with the spec evidence and a lead/user decision, never by widening the
  tolerance.
- Dropping squeeze X1 departs from spec 2.5. It needs the lead's sign-off. It is not a gate change and must not be
  presented as one.
- Net must-fail counts and failing-test names are not acceptable evidence. They hid 23 squeeze cells, 8 lateral gate
  rows, the labels' gap-floor break and G3's `area` rise.
- No re-baselining of `BLOCK_CEILINGS` or `identity.tsv`: the new regress fixture's rows must be measured blocks-off,
  from dev's build.
- `area` and compact musts already fail on the analog. The temptation is to accept G3's +1 M because "the squeeze will
  buy it back". That is an intermediate worsening and needs the lead's explicit decision.

## 6. Files

- `K/rejudge.sh`, `K/cells.py`, `K/panics.py`, `K/fmrun.sh`, `K/trun.sh`, `K/qrun.sh`, `K/build.sh`
- `K/stack-with-knobs.diff`: the four prototypes with knobs, against 7122a53. Experiment only.
- `K/j/*`: judge outputs per combo (live and corrected); `K/t/*`: suite-on test logs; `K/q/*`: direction copies plus
  `summary.txt`.
- `K/svg/<combo>/`: SVGs of the 95 local inputs per combo; `K/tmp/lrbt-*.svg`, `K/tmp/ber-*.svg`.
- `K/pic/labels-vs-squeeze-labels/infra-overview.pair.png`, `K/pic/lrbt-x1/lr-BT.pair.png`
