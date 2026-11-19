## Fix round 4 of 2026-11-17 (on top of `7892fb7`; read this first)

Commits: `fb56b2f` (#2, #3), `7122a53` (#7). Work files `B/work/a2i/` (scripts `bon.sh`, `ton.sh`, `on.sh`, `qcol.sh`,
`tools/mkdirs.py` (332 direction copies of `tests/fixtures/{blocks,quality,notes}` with unique names, `dirs/`; the 32
of `examples/` in `dirsx/`), `tools/offenders.py` + `tools/bm-gapedge` (blockmetrics with the nearest foreign edge and
the striking edges named per label), `tools/runs.js` (the yardstick's straight runs of one path); suite-on binaries
`fm-on-head` / `quality-on-head` (7892fb7), `fm-on-split` / `quality-on-split` (7122a53), `fm-on-507b`,
`quality-on-507b0d6`, `quality-on-4812274`; suite-on test lists `ton-head.fail`, `ton-fix2.fail`, `ton-split.fail`;
guard logs `guard-fix2.txt`, `guard-split.txt`; perf `perf1..3.log`; local runs `B/local/out/{fr4-head,fr4-split}`).

**Result: A2's gate is still not met: local judge `--due A2` 91 must-fail lines on 444 checks (the t9 copies of #4
added 2 checks; at the round's start the same judge gave 93), `--due A1` 0. This is the D6 stop that lead note 5 of
"after verify#2" foresees; every failing row has its cause per fixture below.** Items #2, #3, #4 and #7 (t9c) are done;
#1, #5, #6 are reported. Nothing re-baselined or loosened (the judge gained one rule, item #4); no README / CHANGELOG
/ version change; default build byte-identical (off-identity test green in both guards).

| verifier item | result |
|---|---|
| 1 A2 gate (91) | **not met (D6)**: 91 must-fail lines, each cause per fixture in the tables below; dense-lr area 10.87 Mpx2 (must 7.98) with its cause; "narrowing still accepts a candidate" now measured; `BLOCK_SLACK` not pinned (the squeeze is not on the branch); `defect_baseline_gate` static fields per fixture with their edges |
| 2 lr BT 1 -> 3 crossings, doubling-back end at pager | **fixed** (`fb56b2f`): x_drawn 3 -> 1, no path reversal on any block-mode copy |
| 3 507b0d6 changed 14 copies, not 12 | **corrected** in `fb56b2f`'s message and here (list below) |
| 4 t9 BT copies not in the local judge | **done** (workspace): three BT copies in `B/local/in`, their start / 0.38 / mermaid rows, and a `lateral_x` rule on the t9 family; the judge now flags the 9f51f77 regression (4 lines) |
| 5 causes missing for label / shape rows | **given** per fixture and per label / edge (section "Causes") |
| 6 routed regressions not reported | **reported** with the commit that made each (section "Routed") |
| 7 the t9 test accepted lateral_x <= 2; s3-nested-two-way TD 4 | t9c **fixed** (`7122a53`): 2 -> 0 TD and BT, the test asks 0 everywhere; s3 TD **not fixed**, cause below |

### #2: blocks/lr drawn BT (`fb56b2f`)

Traced stage by stage (`edgepass::stages` with a scratch probe): after `ortho`, `cart -> pager` entered pager's top
at 320.5 just past EDGE's chip; `router::repair` rejected it and its wider lattice found a path into pager's **right**
side, coming down at x 362.75 (a lattice line 18.75 px off the side, inside the 20.75 px stub) and reaching the stub
(364.75) by running on to 368.3 and back: `search` only forbids reaching a goal from inside its node, so the
overshoot cost 7 px and no bend. Two changes, block layout's static scene only:
- `router::despike` drops such spikes from a found path (the leg then runs straight into the port);
- `ortho::TitleSides::of`: a displaced end takes a column past the chip only when no other edge's crossing of the
  same title side lies between its port and the column. EDGE (grown by 507b0d6 for the columns) has `ship -> pager`'s
  crossing on the free stretch past the chip, and `cdn -> gateway` (an exit wholly under the chip at the frame's other
  end) was displaced past it and stepped back over it and `cart -> pager` (two of the three crossings).
lr BT now: x_drawn 1, lateral_x 3 (as 9f51f77: `cdn`'s exits leave by EDGE's left wall), side_ends 1, no title struck
(the frame keeps its X2.2 breadth, so `cart -> pager` passes past the title's end; at 9f51f77 it ran into its last
letter). Of the 332 + 32 direction copies only lr BT changes; the verifier's `reversals.py` finds 0 reversals (was 1);
routed unchanged everywhere. A blanket "no U-turn" rule in the search was tried first and dropped: it moved bench-75 LR's
searches (static `fan_x` 4 -> 15, defects 0 -> 2). Test
`blockcheck::a_displaced_end_never_steps_back_over_another_edges_crossing_of_its_title_side` (lr BT x_drawn <= 1, and
no drawn path of any block fixture in any direction turns straight back), `router::tests::a_found_path_loses_the_spike_past_its_stub`.
Picture `B/work/a2i/pic/lrbt.pair.png` (9f51f77 | now).

### #3: what 507b0d6 changed (record)

Suite-on CLI at 507b0d6 against the fresh 9f51f77 build over the 364 copies: **14** changed, `blocks/interleave` TD,
`l5loop` TD, **`lr` TD and BT**, `nested-chain` TD/BT, `t9-frames-8px` TD/BT, `t9b-frames-8px-lr` TD/BT,
`t9c-frames-8px-two-way` TD/BT, `title-entry` TD, `quality/bench-75` TD (`B/work/a2i/diff-507b.txt`). lr TD: EDGE
50 px broader on its right (column room), `cdn -> queue` straight without its kink, harness numbers unchanged
(`pic/d/blocks.lr-TD.pair.png`); lr BT: the regression of #2. Recorded in `fb56b2f`'s message (507b0d6 is not amended).

### #4: the t9 copies in the local judge (workspace)

`B/local/in/t9-frames-8px-bt.mmd`, `t9b-frames-8px-lr-bt.mmd`, `t9c-frames-8px-two-way-bt.mmd` (header changed to BT);
their structure files; rows appended to `B/metrics/ref-start.tsv` (`B/start/flowmaid`), `baseline-038.tsv`
(`B/fm038`) and `ref-mermaid.tsv` (mermaid 12.2), from `B/local/out/t9bt-{start,038,mm}`; `mkthresholds.py` gained
one A2 rule, `lateral_x <= 0` on the six t9 inputs (mermaid's 0; 0.38 and the start draw 3 / 3 / 8); `thresholds.json`
regenerated (133 -> 134 rules, the only difference). Backups `B/work/a2i/metrics-backup/`. With the fresh 9f51f77
CLI the judge now reports the regression the verifier found: t9 BT 4, t9b BT 4, t9c TD 4, t9c BT 4 (4 must-fail
lines); at 7892fb7 t9c TD / BT 2 each; now none.

### #7: two-way pairs (`7122a53`)

t9c drawn TD: the four steps of the pair share one layer gap (between the stacked frames, and `Beta one`'s jog room
inside the lower frame) and their spans overlap, so `ortho::place_tracks` ordered them as one stack of tracks; the
order interleaved the two rooms and the jog room's two steps landed at 291.5 and 296.5 (5 px apart); the router
rerouted `b -> a` round the right of both blocks. In a block layout each set of steps whose rooms chain is now a
channel of its own: the jog-room steps stand 12.5 px apart and the ortho path is kept. t9c TD / BT lateral_x 2 -> 0;
`nested-cases` LR changes geometry only; every other copy and every routed output unchanged. The t9 test now asks
lateral_x 0 for all three fixtures in every direction. Picture `pic/e/blocks.t9c-frames-8px-two-way-TD.pair.png`.

**s3-nested-two-way drawn TD stays at lateral_x 4** (verifier's stress input). Cause, traced: `a -> b` (label `go`)
has its label slot left of block B (the layer gap's label column at x 125, B spans 183-293), so the ortho path runs
down beside B and turns into `b` through B's and `B one`'s left walls at y 313.5 (2); `b -> a` (label `back`) was
drawn by `ortho` out of `b`'s top under `B one`'s chip and up at x 204.9, and `router::repair` rerouted it out of `b`'s
left side and round (2). The label column belongs to the layout's label slot (G3's slot rule, not built); not fixed.

### Causes per fixture of every must-fail line (static, suite-on CLI at `7122a53`, `B/local/out/fr4-split`)

By metric: `lateral_x` 18, `compact_max` 14, `sib_stagger` 12, `lbl_crowd` 12, `lbl_in_end` 7, `compact_mean` 7,
`bends_max` 7, `lbl_struck` 5, `bends` 2, `area` 2, `straight_max`, `lbl_own_gap_max`, `lbl_gap_min`,
`lbl_endpx_mean`, `detour_max` 1 each.

**Label rows.** `lbl_crowd` counts labels within 12 px of a foreign edge, `lbl_gap_min` is the least such distance,
`lbl_struck` counts foreign edges running through a chip, `lbl_in_end` chips in an end block, `lbl_own_gap_max` the
largest distance from a chip to its own edge. Offenders (label edge: distance to the named edge):

| input | lbl_crowd offenders | struck | in_end | own_gap |
|---|---|---|---|---|
| analog | T1>DNS 3.8 to U2>DNS; LANE>GITLAB 6.6 to ARGO>GITLAB; PG>DRAPP 0.0 to DNS>TR2; R2>[PROD] 10.2 to R2>[DR] | PG>DRAPP by DNS>TR2 | LANE>GITLAB in STG | U1>DNS, U2>DNS 16.5 |
| v2 | T1>DNS 0.0 to U1>DNS; DNS>DRAPP 0.0 to DRAPP>TG; DNS>PSDEV 10.3 to DNS>SITE; DNS>SITE 6.5 to DNS>PSDEV; DRAPP>TG 10.2 to DNS>PSPROD; DRAPP>PG 6.5 to DNS>TR1 | T1>DNS by U1>DNS and U2>DNS; DNS>DRAPP by DRAPP>TG | ARGO>GITLAB in PROD | - |
| perm-a, perm-c, perm-d | as the analog (same six offenders, same numbers) | PG>DRAPP by DNS>TR2 | LANE>GITLAB in STG | U1, U2 16.5 |
| perm-b | T1>DNS 3.8; DNS>PSPROD 4.8 to PSPROD>TG; DNS>SITE 3.0 to DNS>PSPROD; DNS>TR1 10.3 to DNS>TR2; LANE>GITLAB 3.8 to ARGO>GITLAB; ARGO>GITLAB 6.0 to R2>[DR]; PG>DRAPP 10.2 to R2>[DR]; R2>[PROD] 4.6 to R2>[DR] | - | LANE>GITLAB in STG | U1, U2 16.5 |
| perm-e | T1>DNS 3.8; ARGO>TG 10.3 to LANE>OAI; LANE>OAI 9.6 to ARGO>TG; PG>DRAPP 10.2 to R2>[DR]; R2>[PROD] 10.2 to R2>[DR] | - | LANE>GITLAB in STG | U1, U2 16.5 |
| public fixture and its 4 permutations | LB>QUEUE `webhooks` 7.4 to REG>[ORD]; CART>QUEUE `place order` 4.5 to CART>CACHE | - | - | - |

Causes, one per offender kind (every offender above falls under exactly one):
- **A labelled fan into one side (T1, U1, U2 -> DNS; analog family).** The three top-row sources feed DNS's top in
  one gap. T1's chip (310 px) sits on its 125 px straight line between U1's and U2's lines; the label stage, keeping
  chips off each other, moved U1's and U2's chips 16.5 px off their own lines (`own_gap`), and T1's stays 3.8 px from
  U2's line (v2: struck by both). Cause: G3's label slots for a fan (one slot per labelled edge in the gap, with
  `CLEAR_GAP` from the other lines) are not built; the gap holds the three chips side by side only if it reserves them.
- **A long chip on a stretch crossed by other runs (PG>DRAPP analog / a / c / d; perm-b, perm-e at 10.2).** Its 361 px
  chip rides its horizontal run in the gap above DR / STG, which DNS>TR2, DNS>PSPROD, PSPROD>TG and R2>[DR] cross at
  x 961.6, 1138.6, 1423.6 (crossing list of the analog): no stretch of the path between crossings is as long as the
  chip, so the label stage leaves it struck by DNS>TR2 (`lbl_gap_min` 0, `lbl_struck` 1). Same kind: v2's DNS>DRAPP
  struck by DRAPP>TG. Cause: G3's `CLEAR_GAP` reservation (a labelled stretch kept free of foreign runs), not built.
- **A label moved into its end block by the label stage (LANE>GITLAB analog family; ARGO>GITLAB v2).**
  `labels_off_end_blocks` puts LANE>GITLAB's chip on the stretch between PROD and STG, which R2>[DR]'s run crosses at
  (2103.8, 928.6); the label stage then moves the struck chip along its path to the nearest free place, inside STG
  (its distance to ARGO>GITLAB is the 6.6 px `lbl_crowd`). Cause: as above, the gap stretch is not kept free for the
  label (G3 `CLEAR_GAP`), and the label stage's re-placement only avoids end blocks for a label it moves itself on a
  short edge (`labels::place_long`).
- **Neighbouring lines of one fan (R2's three stand-ins; DNS's fan into DR; CART's two exits; LB / REG into ORD).**
  R2>[PROD]'s chip on its 192 px line lies 10.2 px from R2>[DR]'s line leaving the free pin 28 px beside it; the fan
  branches of DNS into DR (DNS>PSDEV / DNS>SITE, DRAPP>TG / DNS>PSPROD, DRAPP>PG / DNS>TR1) and the public fixture's
  CART>QUEUE / CART>CACHE and LB>QUEUE / REG>[ORD] are the same: a chip on a line whose neighbour runs at the fan's
  pitch (28 px or less). Cause: the gap's label room for a fan (G3) is not built; the even band of free pins (B12.11)
  would set R2's pin pitch, and was tried (fix round 3) without any metric gained.
- **Two labelled lines crossing (perm-e: ARGO>TG and LANE>OAI).** They cross at (1670.0, 382.8) in the gap below EXT
  and each chip sits ~10 px from the other's line: the crossing lies within half a chip of both labels' slots (G3).

`lbl_endpx_mean` (v2 385 against 329): the mean distance of a chip from its nearer end; the long chains' labels sit
in the band gap their chain's slot is in (`layout::label_slot`), 465-573 px from the ends on ARGO>GITLAB and
DRAPP>TG and 242-377 px on DNS's fan into DR. G3's slot is "the gap nearest the source when it has room", not built.

**Edge-shape rows.**
- `bends_max` (analog and perms a, c, d, e: DNS>PSDEV 13; perm-b 12; v2 DNS>PSPROD 13). DNS>PSDEV's scene path has
  **6 axis turns**: a step from DNS's column to PSDEV's in the gap below EXT (2), the displaced crossing of DR's title
  side at the column past its chip (x 361.2, 2.8 rule 2: PSDEV sits wholly under DR's chip) and the jog back to
  PSDEV's port inside DR (2), as the spec draws a displaced entry. The yardstick counts 13 because every corner's
  painted cubic tail is a straight run of its own under its 1.5 deg / 0.75 px tolerance: `tools/runs.js` on the
  painted path lists 14 runs >= 8 px (the 399.8 horizontal alone splits into 39.8 + 52.8 + 499.2 at the corner's
  tail `C 780.5 399.8, 775.9 399.8, 674.0 399.8`). v2's DNS>PSPROD: 8 turns (a step below EXT, a step at y 669 where
  the chain's dummy column changes between bands, X1 / X3 not on the branch, and the displaced jog into PSPROD).
  So the excess is the displaced entry's step (2.8, by design: mermaid strikes the title instead), the chain columns
  of B12.8 (D6), and the yardstick's corner tails (not changed: it is the gate).
- `bends` (analog 141 / 120, v2 147 / 114): the same three causes summed over all edges.
- `detour_max` (analog LANE>AZ 1.72 / 1.63): LANE (in PROD) -> AZ (in EXT above), upstream (E3): its exit from LANE's
  top lies under PROD's chip and is displaced to the column past it (x 2052.5, 2.8 rule 2) and steps back to AZ's
  column (1896.4) in the gap below EXT: the displaced exit's 2 x 156 px is the detour.
- `straight_max` (analog R2>[DR] 1590 / 1240): the stand-in from R2 (top right) to the whole of DR (bottom left)
  runs round PROD on the right and then along the whole breadth of the gap above DR and STG (y 928.6); its straight
  run is that gap's breadth, set by the block order (DR leftmost, R2 rightmost) and the unsqueezed frames (B12.8).
- `area` (analog 4.51 / 4.27 M, v2 5.09 / 3.21 M), `compact_max`, `compact_mean`, `sib_stagger`: the frames are not
  squeezed and first rows not levelled (B12.8 X2-X4 not on the branch, D6 of earlier rounds with its numbers; the
  squeeze re-run of round 3 took local A2 from 91 to 72 and broke 4 A1-held tests).

**`lateral_x` (18 lines), per edge (side entered / left, from the blockmetrics detail):** analog and perms a, c, d, e:
DNS>PSPROD (STG's left wall at y 1219.8: the chain's dummies stand left of STG down to PSPROD's row and the end is
taken on PSPROD's left side, not through STG's title side); perm-b: DNS>PSPROD and PSPROD>TG (the same, both ways);
v2: LANE>AZ (out of PROD's lateral wall: its exit under PROD's chip has no column past it that stays inside); public
fixture and its permutations: QUEUE>CHARGE (into CHARGE's left side at its frame's wall), CART>QUEUE (QUEUE's left side),
[WEB]>METRICS (the stand-in runs down left of the blocks and enters METRICS's left side); public-bt: CART>QUEUE,
CART>CACHE, SHIP>DB; infra-bt: PG>DRAPP; hub5: HUB>b1, HUB>d1, HUB>e1; cycle2: b1>a1; dense-lr: n12>n17, n15>n12,
n52>n6 (5); dense-td: n8>n42, n4>n43, n25>n42 (6). Cause class for all: an end that 2.8 wants on the flow-facing side
is taken on a lateral side because the title side has no free column past the chip (X2.2 for title-bound frames, D6
of round 2 with its numbers) or because the chain's last dummies stand beside the block (X2.5 / X3 of B12.8, D6); the
title-row cases are the ones fix round 3 measured.

**B12.12.** dense-lr: 3233 x 3363 = **10.87 Mpx2** (must 7.98). Measured this round with a scratch switch that skips
`narrow`: dense-td's width is 3770 with narrowing and 7272 without (narrowing accepts a candidate: holds); dense-lr is
3233 x 3363 with and without: `narrow` is not entered (breadth / depth 3298 / 2985 = 1.10, below `RIBBON_ENTRY` 2.5),
so the row "narrowing still accepts a candidate on dense-lr" fails, and the area excess is not a ribbon: it is the
unsqueezed frames (dense-lr `compact_max` 43.0; the round-1 squeeze took it to 7.0 in all directions and broke
bench-75's fan musts, D6). `BLOCK_SLACK` is not pinned (it exists only in the squeeze patch).

**`defect_baseline_gate`, static column** (fails in the suite-on run; per fixture, start -> now, from `quality --details`):

| field | ceiling -> now | bench-75 | dense-td | dense-lr | fanout-frames |
|---|---|---|---|---|---|
| end_tilt_max | 0.3 -> 0.80 | 0 -> 0.07 | 0 | 0.27 -> **0.80** | 0 |
| lat_wiggles | 15 -> 33 | 11 -> 18 | 2 -> 11 | 1 -> 3 | 0 |
| lat_backtrack | 1870 -> 10600 | 1659 -> 2261 | 154 -> 7062 | 21 -> 1249 | 0 |
| band_pairs | 132 -> 204 | 61 -> 106 | 32 -> 55 | 32 -> 33 | 3 -> 6 |
| band_len | 74730 -> 91602 | 26976 -> 29346 | 28464 -> 42878 | 17044 -> 15284 | 1532 -> 3386 |
| idiom_spline | 0 -> 1 | 0 | 0 | 0 -> 1 (`n22 -> n44`) | 0 |
| end_kink | 0 -> 1 | 0 -> 1 (`svc1 -> logs`, 51.6 deg on logs) | 0 | 0 | 0 |

The wiggles by edge (static): bench-75 `gw->svc1 svc1->logs svc2->logs svc3->logs svc1->metrics svc2->metrics
svc3->metrics job1->ext job2->ext job3->ext svc6->ext svc5->ext objects->logs db->metrics queue->metrics gw->logs
lb->metrics user->ext` (start: 11 others); dense-td `n16->n41 n16->n35 n16->n43 n16->n44 n8->n42 n11->n39 n15->n38
n4->n43 n5->n44 n2->n7 n46->g9` (start: `n16->n37 n5->n44`); dense-lr `n12->n46 n12->n44 n18->n45` (start
`n12->n44`). Cause: these are the edges block mode turns into channels through flow-facing sides (E1 / E3, decision
D3): an upstream edge between blocks runs against the flow to its target's downstream side (lat_backtrack, the class
4.5 lets rise with named edges), and the fans of dense-td's `n16` and bench-75's services into `logs` / `metrics` /
`ext` step between band columns (lat_wiggles, which 4.5 does not let rise: the cause is X1 / X3, chains straight
across bands, B12.8 under D6). dense-lr's end tilt 0.80 is most likely `n22 -> n44`, the one edge left as a spline
(the harness does not itemize the tilt). Pictures of the corridors were not made this round.

### Routed (R scope, B12.13; item #6)

Suite-on `quality` routed rows, fresh 9f51f77 -> 507b0d6 -> 4812274 -> 7892fb7 (= now; this round's commits change no
routed output):

| row | 9f51f77 | 507b0d6 | 4812274 | now | made by |
|---|---|---|---|---|---|
| bench-75 TD x_drawn / defects | 257 / 7 | 259 / 9 | 259 / 9 | 259 / 9 | 507b0d6 (X2.2 room changes bench-75 TD's frames) |
| bench-75 BT fan_x / defects / x_drawn | 8 / 12 / 267 | 8 / 12 / 267 | 9 / 13 / 259 | 9 / 13 / 259 | 4812274 (one jog count) |
| dense-td BT fan_x / defects / x_drawn | 8 / 22 / 112 | 8 / 22 / 112 | 15 / 16 / 102 | 15 / 17 / 93 | 4812274; 7892fb7 (labels off end blocks) |
| dense-td TD min_edge_gap / defects | 11.97 / 17 | 11.97 / 17 | 0.60 / 18 | 0.60 / 18 | 4812274 (reported in round 3) |
| v2-short-labels-lr-BT lateral_x | 6 | 6 | 7 | 7 | 4812274 |

Cause, shared: 4812274 counts a frame's jog room from its F1 box and title-row entries only, so `route()`'s frames
lost rows of jog room nothing used; `route()`'s own corridors and free pins (R3) are not built, so its fans re-order
on any frame change. These rows are due at R; none is pinned by a test outside the 14 already failing.

### Suite-on, guard, perf, disk

- Suite-on failing tests (`ton-split.fail`): the same 14 names as at 7892fb7.
- Default guard: `GUARD: ALL GREEN` before both commits (off-identity test green).
- `perf.sh` three runs (`runs/b12a2-fr4-1..3`, load 1.0-1.8, no other build): run 2 every file inside its budget
  (dense-td route 6.81 / limit 9.20, drag 12.57; bench-75 11.26 / 19.60; lr drag 1.43 / 1.95; advanced drag 1.80 /
  2.61; stress drag 1.39 / 2.15). Run 1 had lr drag 1.98 and dense-td route 9.82, run 3 stress drag 2.17 (limit 2.15):
  the runs disagree, so no regression is claimed; the default build's code paths of this round run only in block
  mode (the one shared change, `split_by_room`, returns the channel as it is outside a block layout). Static
  `scene_ms` of the sub-0.4 ms files varies between runs as in round 3 (demo 0.15-0.19, subgraph 0.17-0.24, 0.07-0.21
  in round 3's runs); dense-td 27.3 / 50.7, bench-75 42.7 / 61.9, dense-lr 23.7 / 46.8.
- Disk: 9.3 GB free at the start, 8.8 GB at the end. Scratch targets made this round (`B/work/a2i/wt-target`,
  `ptarget`, the worktree `wt9f`) and `B/target-on` are deleted after this report; the suite-on binaries are kept as
  named above.

### Still open (A2), in the order that would unblock it

G3's label slots and `CLEAR_GAP` reservation (every label row above); B12.8's squeeze and straight chains (compact /
stagger / area / straight_max / part of bends and lateral_x; D6 since round 1 with A1-held tests breaking); X2.2 for
title-bound frames (lateral_x on title-sized frames; D6 round 2); route()'s corridors (R). The lead takes the stop to
the user (lead note 5 after verify#2).

---

