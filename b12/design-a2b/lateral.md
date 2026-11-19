# Study "lateral": every `lateral_x` must-fail line of A2, traced to its stage

Base: `7122a53` in block mode (my build of the worktree reproduces `B/work/a2i/fm-on-split` byte for byte on all 92
local inputs, and its suite-on test run fails exactly the 14 names of `ton-split.fail`). Local judge `--due A2`:
91 must-fail lines, 18 of them `lateral_x`. Study folder: `B/design-a2b/lateral/` (B = `.../scratchpad/b12`).

## 1. Result in short

- **The round-4 reading was mostly wrong about the cause.** 16 of the 18 lines are not "chain dummies beside the
  block" or "unsqueezed coordinates": the ortho stage draws the edge through the flow-facing side, and
  `router::repair` throws that path away and finds one through a lateral wall. What it throws away is, in all but
  two cases, a **chip hit** (`Score.chip`). Six different mechanisms put the ortho path on or under a title chip,
  two of them pure boundary ties of 0.0 px. No line traces to the label slot (a) or to X2.2 for title-bound
  frames (c).
- Four of the fixes were prototyped behind run-time switches in one build (`FMX1`..`FMX4`, `FMX6`, `FMX7`), judged
  with the live local judge (copied runner calls, outputs in my folder) and run against the whole suite-on test
  suite (lib + `tests/quality.rs`):

| switches | A2 must-fail | `lateral_x` lines | A1 must-fail | suite-on failing tests (base 14) |
|---|---|---|---|---|
| none (= 7122a53) | 91 | 18 | 0 | 14 |
| X1 ring tie | 90 | 17 | 0 | 14 |
| X2 hug tie | 90 | 17 | 0 | 14 |
| X3 one title stretch | 90 | 17 | 0 | 16 (+2) |
| X1+X2+X3 | **88** | **15** | 0 | 15 (+1, the expected route() jog count) |
| X4 run past chip (prototype) | 88 | 14 | 0 | 16 (+2) |
| X1+X2+X3+X4 | **84** | **10** | 0 | 17 (+3) |

  Per input under X1+X2+X3+X4 (lateral_x, base -> now): hub5 3->0, cycle2 2->0, analog 1->0, perm-a/c/d 1->0,
  v2 1->0, dense-td 6->1 (its must is <= 1: passes), public-bt 3->1, public fixture and its four permutations
  3->2/2/2/1/1, infra-bt 1->1, perm-b 2->2, perm-e 1->1, dense-lr 5->5; outside the must lines also b9-inside,
  l5loop, sub-nested-self, swim-nested, title-hidden 1->0, swimlane-nested 7->6. One must line comes on with X4:
  the analog's `lbl_endpx_mean` 383 -> 389 (must <= 386), see 3.4.
- What is left after the four (10 lines) has three more causes, traced but not prototyped: a free pin standing on
  the column of a member's first free port (public family, 6 lines), a displaced exit sharing a gap track with a
  stand-in (infra-bt), an over-full LR side spilled round its corner plus a chain inside an LR title strip
  (dense-lr), and two X2.5 runs with no free column before the next title-row unit (perm-b, perm-e).

## 2. Method

- Stage probe: `B/work/a2i/dbg-stages.diff` extended (`FMDBGE` takes a list or `*`; `FMDBGN=1` dumps node boxes,
  frame boxes and drawn chips; a probe after `refit`). Added `FMDBGR=1` in `router::repair`: every edge it marks bad,
  its class (0 hard, 1 soft), its scene score and **which chip** the path touches (same test as `World::score`),
  plus a hug breakdown per side; `FMDBGP=1` in `ports::free_pins`; `FMDBGX=1` for the X4 prototype.
- Scripts in the study folder: `stg.py` (one line per stage where a path changed), `chipdist.py` (least distance
  from a stage's path to every title head), `overlap.py` (runs on top of each other at a stage), `col.py`
  (metric per input across variants), `jrun.sh` (render + judge A2/A1), `tsuite.sh` (suite-on tests with switches).
- Experiment diff (all switches and probes, against 7122a53): `B/design-a2b/lateral/lateral-experiments.diff`.
  Runs: `runs/<variant>/` (SVGs, `metrics.tsv`, `judge-A2.txt`, `judge-A1.txt`); tests: `tests-<variant>.{txt,fail}`;
  probe outputs: `probe/`.

## 3. Trace per line and the classes

### 3.1 Table of the 18 lines

Stage notation: path as `ortho` drew it -> what `router::repair` scored -> where the reroute went.

| input (lines) | edge(s) | what the ortho stage drew | repair score | class |
|---|---|---|---|---|
| hub5 (3) | HUB>b1, HUB>d1, HUB>e1 | entry into the member's top at its first free port, **exactly 6.00 px** from the chip (b1: port 310.6, chip end 304.6) | chip 1 (ring touch) | d1 ring tie |
| cycle2 (2) | b1>a1 (A and B left walls) | exit straight up out of b1's top **under** B's chip (x 69 = chip end) | chip 1 | e1 stretch mismatch |
| public fixture x5 (3 each) | QUEUE>CHARGE, CART>QUEUE, [WEB]>METRICS | entries at the first free port, 6.00 px from the chip of PAY / ORD / OBS | chip 1 (ring touch) | d1; once d1 is fixed, CART>QUEUE and [WEB]>METRICS fall to e2 (pin on the same column) |
| public-bt (3) | CART>QUEUE, CART>CACHE | both exits of CART under WEB's chip (CART's free stretch is 5.4 px raw, 0 after the port inset) | chip 1 | e1 |
| public-bt | SHIP>DB | the chain's run on ORD's title row stands under ORD's chip (x 560.9, chip 522.5-622) | chip 1 | b (X2.5 run) |
| analog, perm-a/c/d (1) | DNS>PSPROD | the chain's run on STG's title row (TR2's layer) at x 1005.2, under STG's 415 px chip (999.2-1414.6); the "961.6 left of STG" of round 4 is the **repaired** path | chip 1 | b (X2.5 run) |
| perm-b (2), perm-e (1) | DNS>PSPROD (+ PSPROD>TG) | the same two runs; PSPROD>TG's column 1415.3 lies 0.7 px past the drawn chip | chip 1 | b (no free column, see 3.4) |
| v2 (1) | LANE>AZ | chain column inside PROD exactly 12.000 px from PROD's wall (F3 pad) | class 1 soft, hug 146 | d2 hug tie |
| infra-bt (1) | PG>DRAPP | correctly displaced (column past PROD's chip at 1813.7), but its gap run at y 606.8 lies **on** the stand-in R2>[DR]'s run (882-1813) | overlap | e3 shared track |
| dense-lr (5) | n12>n17 (g1, g2 bottoms), n15>n12 (same) | n12's right side holds 11 border exits; `ports::assign` spills these two onto n12's bottom; ortho draws an L whose corner lies 30 px below g1's bottom wall; no repair | (not rerouted) | e4 spill |
| dense-lr | n52>n6 (g1 top) | chain's first dummy inside g1 at breadth y 215.1, inside g1's LR title strip; the step runs through g1's chip | chip 1 | e5 LR strip |
| dense-td (6) | n4>n43 (g1, g8, g9, g10 right walls) | entry at n43's top x 1625.6, inside g10's chip ring (n43's raw free stretch 4.2 px, after inset none) | chip 1 | e1 |
| dense-td | n8>n42 (g9 L), n25>n42 (g6 L) | both runs on g9's title row under g9's chip (x 909.0 and 1012.2; chip 903-1014.2) | chip 1 | b (X2.5 run) |
| s3-nested-two-way TD (stress, not judged) | b>a (B1, B L) / a>b (B, B1 L) | b>a: exit under B1's chip (e1); a>b: the label column left of B, step on B1's jog track | chip 1 / not rerouted | e1 / a |

### 3.2 d1, the ring tie (hub5 3, public family first round)

`ports::unshaded` cuts a title-row side at the chip **plus `CHIP_SEP` (6)**; its own comment says what happens:
"That leaves the first free port exactly on the router's limit: in a block layout a stroke there that touches the
chip's ring counts as running along the frame's name and is rerouted round to a lateral side of its node." The
router grows the chip by `chip_reach = CHIP_SEP` and counts a touch. The layout's F2 (`title_cap`) and `ortho`'s
title sides already use `CHIP_CLEAR = CHIP_SEP + SEP_AIR` (6.5).
**Fix:** in block mode `unshaded` cuts at `CHIP_CLEAR` (one clearance for layout, ports and ortho).
**Measured (X1):** hub5 3 -> 0, QUEUE>CHARGE fixed on all five public copies, swim-nested 1 -> 0, swimlane-nested
7 -> 6; A2 91 -> 90; no test changes. Default output untouched (gated on `cx.blocks`).

### 3.3 d2, the hug tie (v2 1)

F3 grows a frame over an inner leg to the leg's column + `LANE_PAD` (12). The router's hug test is
`|c - line| < CLEAR_GAP` (12); on the simplified path the leg's midpoint lands a hair inside, so a 146 px chain
column beside PROD's wall scores `hug 146 > HUG_TRADE 120`, the labelled edge is searched for spacing, and the
search leaves PROD through its right wall.
**Fix:** F3's pad for inner legs in block mode `LANE_PAD + SEP_AIR` (as `TRACK_PITCH = EDGE_SEP + SEP_AIR` does),
in the one frame function, so `route()`/`route_partial()` take the same 0.5 px (B12.10).
**Measured (X2, scene only):** v2 1 -> 0; `compact_max` +0.01 on a few inputs; no test changes.

### 3.4 b, X2.5 runs left under the chip (analog family 4+2+1, public-bt 1, dense-td 2)

X2.5 is built as an **order** move only (`BlockOrder::clear_titles`): it offers columns right of title-row units
whose right edge is past the chip, and reverts the move when layered crossings rise. The analog's STG has one
title-row member (TR2, 1755-1927) and free room between the chip's end (1421) and TR2, but no order move can
express "stay left of TR2, right of the chip"; the move past TR2 adds crossings and is reverted. The breadth half
(`title_room`) skips a run whose left neighbour is the block's wall and caps the move at `TITLE_ROOM_MAX` = 100 px.
Same on public-bt (ORD: run at 560.9, free room 628-767 before PAY) and dense-td (g9: runs at 909 and 1012, free
room up to g10 at 1470). Spec X2.5 asks exactly for "a free column between title-row members where one exists".
**Fix:** a breadth bound, not an order move: an inner run on a title row whose order puts it before the next
title-row unit takes `bpos >= chip_hi(after the move) + j * TRACK_PITCH` when that room exists before the next unit;
the chip must be recomputed with the run moved (a run that sets the frame's wall drags the frame and its chip:
first prototype moved STG's run 430 px and the chip followed it 72 px). It must be built as a constraint of the
breadth placement **before** `TITLE_FIT_PASSES`; my prototype shifts after them (`FMX4`, layout.rs) and therefore
breaks two A1-held tests (`a_frame_is_as_broad_as_its_title`, `a_one_px_drag_keeps_a_frame_grown_to_its_title`:
title-hidden's frame was held by the run's wall).
**Measured (X4 prototype):** analog, perm-a, perm-c, perm-d 1 -> 0; public-bt SHIP>DB fixed; dense-td 2 -> 1
(n25>n42 fixed; n8>n42 has 21 px too little room); title-hidden 1 -> 0. perm-b and perm-e keep theirs: a title-row
member stands before the chip's end (`no room (next unit at 1333 < 1412)`), so only "past the outermost member"
remains, which `clear_titles` reverts for crossings.
**Cost:** the analog's runs now go round STG's 415 px, three-line chip; their labels ride the middle of the longer
run: `lbl_endpx_mean` 383 -> 389 (must <= 386). G3's "slot nearest the source" (labels study) would take that back;
built without G3 it trades one must line for another on the analog.

### 3.5 e1, two title stretches that disagree (cycle2 2, public-bt 2, dense-td 4, s3 TD b>a)

F2's `layout::title_cap` measures a member's free stretch on its **raw box** with `CHIP_CLEAR`; the ports stage
(`unshaded`) measures it on the **port span** (box less `PORT_INSET` = 8 on each side) with `CHIP_SEP`, and when the
free part cannot hold all of the side's ends at `PORT_SPACING` it **gives up the shading and spreads them over the
whole side, under the chip**. `ortho`'s title sides displace only ends of a member wholly under the chip. A partly
shaded member whose ends do not fit is handled by nobody: cycle2's b1 (layout cap 2, ports 13 px < 14 for two),
CART in BT (layout cap 1 in 5.4 px, ports none), n43 (layout cap 1 in 4.2 px; its end at 1625.6 also blocks n16's
twins' displaced columns, `fb56b2f`'s "no crossing between port and column" rule). The spec (F2) asks for one
`pub(crate)` function for that stretch.
**Fix:** one function on the port span with `CHIP_CLEAR`, used by `title_cap`, `unshaded`, `route_title_jogs` and
ortho's test; a member whose title-side border ends exceed its cap has **all** of them displaced (not "the first
cap stay, the rest jog": mixed, a displaced jog always crosses the free entries' legs on a narrow member); the
ports stage then keeps them all under the chip, strictly inside ortho's interval, and ortho displaces per end.
**Measured (X3):** cycle2 2 -> 0, dense-td 6 -> 2 (n4>n43's four walls; also takes n16's twins off g10's title
text, picture), public-bt 3 -> 2, b9-inside, l5loop, sub-nested-self 1 -> 0. Tests: `frames_agree_between_scene_and_route_along_the_flow_on_every_fixture`
fails (b9-inside TD: scene jog room 12.5 px more than route's) because I did not put the rule into
`route_title_jogs`; that is the fix's own half, not a conflict. `a_hubs_fan_keeps_its_order_through_the_bands` fails
with X3 alone (`n16->n43 overlap n43->n16 252px`) and passes with X1+X2+X3: fragile, to be watched.
Interaction found and fixed in the prototype: with X1 the fallback interval ended exactly on ortho's chip interval
(75.5), one end stayed "free" and blocked the other's column.

### 3.6 e2, a free pin on a member's first free column (public family, 6 lines after X1)

ORD's frame top and QUEUE's top lie under the same chip; `place_ports` puts the stand-in REG>[ORD]'s pin on the
frame side's first free port (chip end + clearance) and CART>QUEUE on QUEUE's first free port: the same column
(628.0 / 628.5). `free_pins` sees the collision but has no room (chip on one side, five crossings within 46 px on
the other) and leaves it; the two runs lie on top of each other, the repair takes CART>QUEUE round ORD's left wall.
Same for OPS>[OBS] and [WEB]>METRICS on OBS (328.6). With `CHIP_SEP` the two were 0.5 px apart and the member's
entry touched the chip ring first (d1), which is why round 4 saw a chip and not this.
**Fix (not prototyped):** one set of columns per frame title side: pins and member entries are spaced together
(2.8 free pins: "centred in the free interval between the member-entry columns, the corners and the title chip, at
least PORT_SPACING from each" presupposes the member entries leave such an interval); F2 counts a frame's pins on
its title side among the side's entries, so an over-full title side displaces (or, for a pin, moves the member
entries along) instead of stacking. **Predicted:** public fixture and permutations 2/2/2/1/1 -> 0 (6 lines).

### 3.7 e3, a displaced exit sharing a gap track (infra-bt 1)

PG>DRAPP is displaced correctly (column past PROD's chip), but its long run in the gap above PROD takes y 606.8,
the track of the stand-in R2>[DR]; `repair` (class 0, overlap) escapes left through PROD's wall.
**Fix (not prototyped):** the displaced crossing's gap step joins `place_tracks`' channel set with the gap's other
steps (G2); a test on infra-bt-like public fixture. **Predicted:** infra-bt 1 -> 0.

### 3.8 e4/e5, dense-lr (5)

- e4: n12 (in g2 in g1) sends 11 border edges out of its right (flow-facing) side; the spill puts n12>n17 and
  n15>n12 on its bottom (2.8 allows it: "only when the border ends alone exceed the side's capacity", counted in
  lateral_x). Their ortho path is a plain L (down 1650 px inside g2, corner below g1). **Fix:** a spilled border end
  takes the corner leg the spec calls for, back to the frame's flow-facing side (leave the lateral side by a stub,
  turn, cross g2/g1's right walls, then the step to the target), as step 1 did for TD's corner legs. **Predicted:**
  4 lines' worth of crossings, dense-lr 5 -> 1.
- e5: in LR the title strip is on a lateral side; the chain n52>n6 enters g1 at breadth 215.1, inside the strip
  (chip 207.6-238.1), the step runs through the chip, the repair crosses g1's top. Spec 2.8 only says pins keep
  out of the strip's band. **Fix:** in LR/RL a chain's first dummy inside a block keeps out of the strip's breadth
  band (the title-clear rule turned a quarter). **Predicted:** dense-lr 1 -> 0.

### 3.9 (a) label slot, and (c) X2.2 for title-bound frames

- (a): only s3-nested-two-way TD's a>b (stress input, not judged): the gap's label column is left of B, and ortho
  runs the step on B1's jog track inside B. Its twin b>a is e1, not G3.
- (c): **no must line traces to it.** It concerns title-entry (lateral 2), title-hidden, nested-wide-title, x6,
  which are not A2 must rows. The conflict, measured with the test binary (`FMDBGW` prints the test's canvas):

| variant | hub-fed row canvas | g0 (title-bound) | test `w < 2h` | title-entry lateral |
|---|---|---|---|---|
| 7122a53 | 1236.1 x 631 | w 186.7 | pass | 2 |
| E6: column room for title-bound frames as built for others (`title_column_width`, round 3's attempt) | 1275.8 x 631 | w 213.2 (+26.5) | **fail** (< 1262) | 0 |
| E6 with an exact reservation (computed) | 1262.6 x 631 | +26.5 | fail by 0.6 px | |
| E7: first displaced column at the chip's clearance (no `CORNER_R`), jog room `+CORNER_R` for title-bound frames | 1254.8 x 645 | w 199.2 (+12.5), h +14 | **pass** | 1 |

  Why it fails: `title_column_width` = chip + `CHIP_CLEAR` + `CORNER_R` + `LANE_PAD` = the F1 frame + 26.5 px, twice
  the spec's X2.2 amount (F1 breadth + `TRACK_PITCH` per displaced entry), because ortho puts the first displaced
  column a corner radius past the chip's clearance, so the turn onto the jog track stays out of the chip's ring;
  and the layout's wall reservation (`TITLE_FIT_PASSES`) pushes the members by 1.5 x the excess (26.5 -> 39.75 px
  of canvas). Even exact, 26.5 px leaves the hub-fed row 0.6 px too wide.
  What keeps both: take the corner's room **along the flow** for a title-bound frame (column at chip end +
  `CHIP_CLEAR`, i.e. the spec's X2.2 breadth, and the jog track `CORNER_R` deeper), which costs width 12.5 px and
  depth 14 px; on an aspect bound that is a 2:1 trade. E7 does this and passes the test, but (1) it changes F2's
  jog formula, pinned by `jog_room_counts_only_what_crosses_the_title_side` (title-entry 39 != 25), and (2) any
  column room for a title-bound frame contradicts `a_frame_holds_its_bold_title_as_a_browser_draws_it`'s line
  `!c.bold_title || c.w <= least + 0.5`, written when such frames got no column (round 2's D6). Spec 2.8 rule 2
  says the frame grows to the last displaced column + `LANE_PAD`, so that assertion pins the absence of a spec rule;
  changing either test is a spec decision for the lead/user, not something a fix round may do. E7 also leaves one
  of title-entry's two lateral entries (not traced in this study).

## 4. Proposal

Order (each its own commit with numbers; all in block mode only; default byte-identical):
1. **X1 ring tie** (`ports::unshaded` cuts at `CHIP_CLEAR` in block mode). Test: a title-row member's first free
   entry clears the chip ring (hub5 HUB>b1 path unchanged by the repair). Expected A2 -1 (hub5), A1-held tests
   unchanged (measured).
2. **X2 hug tie** (F3 inner-leg pad `LANE_PAD + SEP_AIR` in the one frame function, scene and route). Expected A2
   -1 (v2), no test changes (measured, scene only; check route()'s frames agree across the flow).
3. **X3 one title stretch** (one function on the port span with `CHIP_CLEAR`; all-or-nothing per member; ports keep
   them under the chip; ortho displaces per end; `route_title_jogs` takes the same count). Expected A2 -1 (cycle2)
   and dense-td 6 -> 2, public-bt 3 -> 2; must keep `a_hubs_fan_keeps_its_order_through_the_bands` green (it is with
   1+2, not alone: land after 1 and 2). Measured X1+X2+X3: A2 91 -> 88, A1 0, suite 14 -> 15 failing with the one
   expected route() count.
4. **X2.5 as a breadth bound** (into existing room, chip recomputed after the move, before `TITLE_FIT_PASSES`).
   Expected A2 -4 lateral lines (analog, perm-a/c/d), dense-td 2 -> 1 (passes its must), public-bt SHIP>DB; trades
   the analog's `lbl_endpx_mean` (+1 line) unless G3's source-side slot lands first (labels study). Must not break
   `a_frame_is_as_broad_as_its_title` / `a_one_px_drag_keeps_a_frame_grown_to_its_title` (the prototype did,
   because it shifted after the title fit).
5. **e2 pins and member entries spaced together on a title side** (B12.11's even band, now with a reason: it is
   what the public family's 6 remaining lines need). Expected -6 lines.
6. **e3** displaced gap steps in `place_tracks` (infra-bt -1), **e4/e5** dense-lr's corner legs for spilled border
   ends and the LR strip band (dense-lr 5 -> 0).
7. perm-b / perm-e: X2.5 with no free column needs "past the outermost member" accepted at a crossing cost or the
   frame grown; take it to the lead with numbers after 4.
8. (c) is off the A2 path; if wanted, E7's trade is the shape that keeps the hub-fed row test, and it needs the
   lead's decision on F2's jog formula and on the bold-title assertion.

Predicted end state on `lateral_x`: 18 lines -> 15 (1-3, measured) -> 10 (4, measured with the prototype) -> 4
(5) -> 1-2 (6) -> perm-b/perm-e only. Verify each step with: the local judge (`--due A2`, `--due A1`), the suite-on
run against `ton-split.fail`, the default guard and its off-identity test, the 364 direction copies (only the
92 local inputs were judged here), routed rows (not measured here) and perf.

## 5. Pictures (offending edges stroked red; left 7122a53, right X1+X2+X3+X4)

- `B/design-a2b/lateral/pic/hub5.pair.png`: three hub entries through Billing/Delivery/Email's left walls -> through
  the tops beside the chips (d1).
- `B/design-a2b/lateral/pic/dense-td-g10.pair.png`: n4>n43 going round the whole drawing and in through four right
  walls -> displaced column past g10's chip (e1); n16's twins also leave g10's title text.
- `B/design-a2b/lateral/pic/blocks-overview.pair.png`: QUEUE>CHARGE fixed; CART>QUEUE and [WEB]>METRICS still
  lateral (e2: their entries share a column with REG>[ORD]'s / OPS>[OBS]'s pins).
- also `dense-lr-n12.pair.png` (the spilled pair under n12, e4, unchanged), `dense-lr-n52.pair.png` (e5),
  `dense-td-g9.pair.png` (n8>n42, n25>n42 under g9's chip).

## 6. Risks

- X1, X2 move block-mode geometry by 0.5 px everywhere (ports, frames): byte changes in every block drawing; the
  judge's other rows moved by at most 0.01 (`compact_max`); crossings under X1+X2+X3: dense-td 42 -> 43, two public
  permutations 6 -> 5, the rest unchanged.
- X3 adds jog room where members are now wholly displaced (frames taller, more bends on narrow members) and the hub
  fan test is fragile under it (fails alone).
- X4 as prototyped breaks two A1-held tests and the analog's `lbl_endpx_mean`; its proper form is a constraint in
  the breadth placement and depends on G3.
- e2..e5 are predictions from traces, not measured.
- Routed rows, the direction copies and perf were not measured; the switches are read from the environment at run
  time (experiment only).
