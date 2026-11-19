# Study "labels": G3 label slots and the CLEAR_GAP reservation (B12.9)

Read-only study for the follow-up package of `b12a2-geometry`. Base: work branch at `7122a53` (suite-on CLI identical to
`B/work/a2i/fm-on-split`, checked byte for byte). `B` = `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12`,
`L` = `B/design-a2b/labels`.

## 1. Answer first

1. **The crowded chips are almost never the chips G3's dummy rule already places.** Traced with a copy of the
   yardstick that names the side and the segment of the nearest foreign stroke (`L/tools/bm-near`): on the analog,
   every label sitting on its chain's sized dummy row is clean (nearest stroke 22-208 px). Every must-fail label is
   one of four kinds the layout gives no room today:
   - **gap labels**: an adjacent-band labelled edge (`U1/T1/U2 -> DNS`, `PG -> DRAPP`, v2's DNS fan into DR), a
     stand-in cut to a frame across one gap (`R2 -> [PROD]`), a chain whose run at the common level is empty
     (`LANE -> GITLAB`, v2 `ARGO -> GITLAB`). Their chip sits in a band gap among the gap's tracks and the verticals
     crossing it. The layout reserves nothing there (`short_labels` reserves depth for one label mid-gap; tracks and
     chips share it);
   - **chain labels moved off their reserved column by a later stage** (public fixture `webhooks`: the ortho stage
     straightened its column 18.6 px towards its port and onto `REG ==> [ORD]`'s line; the layout had kept the two
     64.6 px apart, exactly `pair_sep`);
   - **chips the router or the label stage moved** (dense-td `n16 -> n21`, `n16 -> n37`; v2 `ARGO -> GITLAB`);
   - **chips on a lateral entry** (public `CART -> QUEUE` into QUEUE's left side): a `lateral_x` case first.
2. **G3 as designed below fixes the first two kinds.** A prototype of its static part (TD only, behind the switch,
   `L/proto.diff`) measured with the suite-on CLI and a copy of the local judge:
   **A2 must-fail lines 91 -> 70-78** depending on two knobs, **label must-fail lines 27 -> 7-15**; the analog's
   `lbl_crowd` 4 -> 0, `lbl_struck` 1 -> 0, `lbl_in_end` 1 -> 0, `lbl_own_gap_max` 16.5 -> 0, `lbl_gap_min` 0 -> 12.5;
   the five permutations of the public fixture 2 -> 0-1 crowded; the crossing counts of the analog, v2 and the public
   fixture unchanged (15 / 14 / 6) and the local judge's `--due A1` still 0.
3. **The label rows fight each other and the area row.** Putting a gap label on the leg nearest an end (cheap: one
   label row of depth) breaks `lbl_off_max <= 0.35` on the analog (0.463: `PG -> DRAPP` 36 px down a 995 px edge);
   pushing labels towards their edge's middle costs gap depth, which lengthens every chain crossing that gap and
   breaks `lbl_endpx_mean <= 386` (461 with every label at off <= 0.33). Sliding chain labels towards their nearer
   end along their straight run buys it back: with both, the analog is at `lbl_off_max` 0.348 / `lbl_endpx_mean`
   396 or 0.360 / 383 depending on one constant, both a hair outside, and v2's `lbl_endpx_mean` goes 385 -> 335-341
   (must 329). The area grows +0.4 to +1.2 Mpx2 on the analog (must 4.27 M, 4.51 M already at `7122a53`), so G3
   must be planned and judged together with the squeeze (B12.8): the squeeze buys back breadth, G3 spends depth.
4. **Bulk**: on `b8r1/bulkfc` the prototype takes static `chip_crowd` from 657 (the verifier's number at
   `7122a53`) to 592, below step 2's start (610); `pin_corner` stays 45; `x_drawn` +45 over 1211 files. On
   `b8r1/bulkst` 778 -> 741 (start 770), `pin_corner` 238 -> 238 (section 7).
5. **Two A1-held tests break** with the prototype (`a_hubs_fan_keeps_its_order_through_the_bands`,
   `an_end_of_a_title_row_member_keeps_out_of_its_chips_breadth_below_the_jog_room`: four `fan_x` crossings into
   dense-td's `n38`, a labelled fan-in whose steps the prototype's per-jog exclusion splits). The cause is the
   prototype's shortcut, not the rule: the exclusion must act on a whole channel (section 4.4).

## 2. What exists today (code map, `7122a53`)

| piece | where | what it does | what G3 needs from it |
|---|---|---|---|
| chain label dummy | `layout::place`, after the cluster paths (`label_slot`, sizing `absize`/`alsize` of `chain[label_slot]`) | middle dummy of the run at the common level; else the middle of the whole chain | keep; add the slide (4.3) and the empty-run case (4.2) |
| breadth separation | `layout::pair_sep` | chip/2 + `EDGE_GAP`/2 + neighbour: a dummy neighbour's line ends up chip/2 + 26 px away | holds `CLEAR_GAP` already for dummy-row chips; nothing for gap labels |
| labelled gap depth | `place`: `short_labels`, `gap_need` (row packing), `hub_label_room`, `fan_label_room` | one label mid-gap, label rows only when labels overlap side by side; maxed with `track_need`, not added | a label row is a row of its own: added to the tracks |
| tracks | `ortho::plan_jog` / `place_tracks` | `above_label`: a labelled Z steps between its source and its label (label rides the run into the target); `label_half` keeps neighbours off a label riding a step | foreign tracks must keep `CLEAR_GAP` off a chip that is not theirs |
| column straightening | `ortho::ColumnRoom::slot` / `straighten` | other columns keep `COLUMN_CLEAR` (21) from a label box; a column carrying its own label is fenced only at the midpoint to its neighbours | a labelled column keeps chip/2 + `CLEAR_GAP` from the other columns on its label's row |
| end blocks | `scene::labels_off_end_blocks`, `label_off_frames` | moves a label inside an end block to the longest stretch outside them | becomes a fallback only (a gap label never lands there) |
| label stage | `labels::relabel` / `place_long` | moves struck or covering labels to clearer spots; a long edge keeps "the slot its dummy reserved" | must keep a band label's spot and know the slid slot |
| route() | `route_built` applies `labels_off_end_blocks` (`7892fb7`) | R8 is due at R | nothing new before R (4.7) |

## 3. Diagnosis per offender (A2 table, static, `7122a53`)

`L/tools/near.sh <svgdir> <name>` lists per label: chip, nearest foreign edge, which side of the chip (`above` /
`below` / `left` / `right` / `in`) and the segment (H/V).

| input | label | kind | nearest stroke at `7122a53` |
|---|---|---|---|
| analog | T1>DNS | gap label, labelled fan into DNS's top (3 adjacent-band edges) | U2's step **below** the chip, 3.8 px (a track) |
| analog | U1>DNS, U2>DNS | same fan | label stage put them 16.5 px beside their lines (`own_gap`): no clean spot on the line |
| analog | PG>DRAPP | gap label (adjacent band) riding its 787 px step under PROD | DNS>TR2's **vertical through** the chip (struck): passing chains at 991 / 1124 / 1249 / 1401 cross the step, no stretch of it is 385 px free |
| analog | LANE>GITLAB | chain with an empty run (PROD -> STG directly): `label_slot` falls back to the middle of the whole chain, inside STG | ARGO>GITLAB's step **below**, 6.6 px; `in_end STG` |
| analog | R2>[PROD] | stand-in cut at PROD's top, adjacent band | R2>[DR]'s step **above**, 10.2 px |
| v2 | T1>DNS | as the analog | U1's and U2's steps through it |
| v2 | DNS>DRAPP | gap label, straight drop DNS -> DRAPP | DRAPP>TG's **vertical** (leaving DRAPP's top 86 px away) through it |
| v2 | DNS>PSDEV, DNS>SITE | gap labels of DNS's fan into DR | each other's steps 5.6-10.3 px |
| v2 | DRAPP>TG, DRAPP>PG | gap labels (turned / adjacent) | DNS>PSPROD's / DNS>TR1's steps, 6.5-10.2 px |
| v2 | ARGO>GITLAB | chain with an empty run | in PROD (`in_end`) |
| perm-b, perm-e | DNS>SITE, DNS>TR1, ARGO>TG / LANE>OAI, PG>DRAPP | gap labels as above; ARGO>TG and LANE>OAI cross each other inside the gap | steps 3.0-10.3 px |
| public x5 | LB>QUEUE `webhooks` | chain on a sized dummy | REG==>[ORD]'s **vertical left**, 7.4 px: the ortho stage straightened `webhooks`'s column 628.7 -> 610.1 (layout kept 64.6 = `pair_sep`) |
| public x5 | CART>QUEUE `place order` | short edge entering QUEUE's **left** side | CART>CACHE's vertical 4.5 px left of the chip on the lateral run |

## 4. The design

### 4.1 Terms and data

- *Dummy-row label*: a chain label on a sized dummy of the run at the common-ancestor level (today's G3 rule).
- *Gap label*: every other label of a block layout's drawn edge whose ends are not in one block: an edge between
  adjacent layers, a stand-in cut to a frame (`Graph::frame_cut`) across one gap, and a chain with an empty run at
  the common level. Its **gap** is the window `(a, b)` of its layered path (upper unit, lower unit) where it crosses
  from the upper end's blocks into the lower end's (for a short edge: its only window).
- Per gap `li` (lower layer), built once in `place` after X with the loop `track_need` already runs:
  `cols[li] = [(edge, upper unit, lower unit, upper column, lower column)]` for every path window crossing the gap
  (pins of cut ends as `track_need` takes them: the other end's column held to the block's members; displaced
  title-side ends at their displaced column, from `title_entries`).
- Per labelled edge: `slot = Dummy(d) | Band { gap, side: Upper | Lower | Track | Middle, row, depth }`, its chip
  `(w, h)`; exported with the placement (today's `(BlockPaths, title_jog)` tuple gets a third member; the prototype
  uses a thread-local side channel).
- Constants: `CLEAR_GAP` (12) for every foreign stroke, `LABEL_GAP + 0.5` between two chips of one row, `ROW_CLEAR`
  between a band and the row or frame it hangs from, `G3_OFF_MAX` = 0.33 (a margin under the gate's 0.35).

### 4.2 Where each label goes (the slot)

**Dummy-row labels** keep `label_slot`, plus one rule (`lbl_endpx_mean`): after X, the label may *slide* along the
straight stretch of its run that holds the sized dummy (same column, same common path) to the dummy nearest the
edge's **nearer end** whose estimated off-middle fraction stays <= `G3_CHAIN_OFF` and whose layer has the chip's
room (`|bpos - bpos[d]| - half >= w/2 + CLEAR_GAP` for every other unit of that layer). The breadth reservation made
for the sized dummy holds along the whole stretch (same columns), so the slide costs no breadth. This is what "the
gap nearest the source when it has room" turns into once measured: for DNS's fan the run starts past the edges'
middle (the step to the column comes first), so the *source* side is the wrong side; the nearer end is the rule.

**Gap labels**, in this order, the first with room wins:

1. *Upper leg* (`Upper`): the chip rides the edge's leg out of the upper unit, in a band at the top of the gap. Room:
   the upper unit has no other line on its facing side in this gap (a fan's shared end never has room), and every
   other upper column of `cols[li]` and every frame wall crossing the gap lies >= `w/2 + CLEAR_GAP + 1` away.
2. *Lower leg* (`Lower`): the same at the bottom of the gap with lower columns (a pin is no port of the member it is
   ranked to: the lower unit of a cut stand-in is never "shared").
3. *Track* (`Track`): the chip rides the edge's own step, on a stretch `w + 2 CLEAR_GAP` long between its two
   columns that no passing column of the gap crosses; the edge's track is then the outermost of its channel on the
   side away from the steps crossing that stretch (they cross its legs, not its chip). Not prototyped.
4. *Middle row* (`Middle`): one label row inside the gap; every edge crossing the gap takes its step above or below
   the row so that its column *at the row* is outside every chip's reserved interval; an edge with both columns
   inside an interval makes the row infeasible. v2's `DNS -> DRAPP` needs it (DNS's other exits step above the row,
   `DRAPP -> TG` below). Not prototyped.
5. Else *today's spot* (mid-gap, `above_label`), reported per fixture as an open G3 case.

**Fans.** A fan's shared end never has room on its own side, so the labels of a fan go to their *free* ends: a
labelled fan *into* one side (U1, T1, U2 -> DNS) puts each label on its source's leg (`Upper`), a fan *out of* one
side (DNS -> SITE, PSDEV, DRAPP) on the targets' legs (`Lower`) where those are free; a target that is itself shared
(DRAPP's top carries three lines in v2) sends that label to `Track` / `Middle`. **Rows**: the band's chips are packed
by breadth (`LABEL_GAP + 0.5` apart, greedy by left edge); **the rows are ordered so that the label that needs the
most depth for `lbl_off` takes the deepest row** (the prototype packs by left edge only: U1 and U2 took row 0 at off
0.43 while T1, straight and 150 px long, took the lower row; one row down U1 and U2 sit at 0.354 with no extra
depth, a few px short of the gate, which the band depth of the next paragraph then closes).

**Depth for `lbl_off`.** A band's first row lies at `ROW_CLEAR` from the row or frame it hangs from, or deeper: at
the depth that puts each of its labels at off <= `G3_OFF_MAX` of its edge's estimated length (`|upper col - lower
col|` + the gap + the end offsets). The band's depth is the largest any of its labels asks. When that would grow the
gap by more than `G3_DEPTH_CAP` (proposed 40 px), the label goes to `Track` / `Middle` instead (a long Z's middle
is on its step). On the analog only `PG -> DRAPP` hits the cap (it asks +113 px).

### 4.3 Where it runs in the pipeline

1. `layout::place`, after X and before the layer lines: build `cols`, decide each gap label's side (the room tests
   use `bpos`), pack the rows, compute each gap's band depth; compute the chain slides. **Breadth**: a gap label
   whose own end is free but whose neighbours are closer than `w/2 + CLEAR_GAP` widens that end's spacing breadth
   (`sep[v] = max(sep[v], w + 2 CLEAR_GAP)`, the mechanism `fan_label_room` uses for B7) and X runs again; this is
   the only breadth G3 ever takes, and only for gap labels (dummy-row labels have `pair_sep`).
2. Layer lines (G1): in a block layout a gap's need becomes
   `ROW_CLEAR + top band + CLEAR_GAP + (tracks - 1) x TRACK_PITCH + CLEAR_GAP + bottom band + HEAD_LEG`, measured
   frame to frame like `track_need`'s block branch (a band row is a row of its own, *added* to the tracks; today's
   `gap_need` is maxed with them). The band's chip centres follow from `lcoord` and `reach_of`.
3. Scene: each label starts at its slot (band position on its leg's column, or the slid dummy).
4. `ortho::orthogonalize` (static and, at R, routed): the edge's own step keeps to the far side of its chip
   (`Upper`: below it; `Lower`: today's `above_label`); a **foreign channel** whose span meets a chip's breadth
   interval `+- CLEAR_GAP` is kept wholly on one side of the chip's flow interval `+- CLEAR_GAP` (the far side of a
   band; the side its wanted place lies on for a dummy-row chip). `straighten` keeps a labelled column's chip
   `+ CLEAR_GAP` off the other columns and runs on its label's row. The label is re-set onto its leg at its band
   depth (`relabel`).
5. `router::repair`, `nudge`, `spread_close`, `comb`: a band chip is a hard obstacle for foreign paths (dense-td's two
   struck chips in the prototype are router reroutes through band chips).
6. `labels::relabel` / `place_long`: a band label and a slid label are "the slot the layout reserved": kept unless
   covered. (The prototype misses this for chains: v2's `ARGO -> GITLAB` band label is moved back into PROD.)

### 4.4 How CLEAR_GAP keeps foreign runs out

- Along the flow: the band row is a row of its own in the gap's need (4.3.2) and the ortho exclusion keeps every
  foreign channel crossing the chip's breadth on the far side. **The exclusion must act per channel**, after the
  channel order is fixed: shrinking the room of single jogs (the prototype) can make `holds(order)` fail, revert the
  channel to its plain rooms and change its order; this is what put four `fan_x` crossings into dense-td's `n38`
  (`n31 -> n38` and `n25 -> n38` labelled, `n29 -> n38` not) and broke the two A1-held hub-fan tests.
- Across the flow: the room tests of 4.2 (columns crossing the band), `pair_sep` for dummy rows, the end widening
  for gap labels, and the `straighten` fence (prototype: `webhooks` 7.4 -> 10.5 px; with the slide, 0 crowded on the
  public fixture).
- Against later stages: chips as hard obstacles in the router and the comb (4.3.5).

### 4.5 When the gap has no room

Depth is owned by `place`'s layer lines (G1): a band always gets its rows; the cap of 4.2 only stops the *extra*
depth for `lbl_off`. Breadth is owned by X: only the end widening of 4.3.1. If no side has room (`Upper`, `Lower`,
`Track`, `Middle` all fail), the label keeps today's spot and the checkpoint report names it with its cause; nothing
is moved into an end block (`labels_off_end_blocks` stays as the last fallback and should then never fire).

### 4.6 The label budget (measured, analog)

| variant (knobs) | H | area | crowd / struck / in_end / own | off_mean / off_max | endpx | A2 must-fail (label) |
|---|---|---|---|---|---|---|
| `7122a53` | 1445 | 4.51 M | 4 / 1 / 1 / 16.5 | 0.144 / 0.326 | 383 | 91 (27) |
| p1 / p6: bands, no depth, no slide | 1572 | 4.90 M | 0 / 0 / 0 / 0 | 0.174 / **0.463** | 381 | 78 (15) |
| p3: depth for off 0.17, no slide | 1834 | 5.72 M | 0 / 0 / 0 / 0 | 0.145 / 0.348 | **461** | 75 (12) |
| p5-c0.34: depth 0.155, slide 0.34 | 1790 | 5.58 M | 0 / 0 / 0 / 0 | 0.189 / **0.360** | 383 | 70 (7) |
| p7: depth 0.17, slide 0.34 | 1834 | 5.72 M | 0 / 0 / 0 / 0 | 0.184 / 0.348 | **396** | 71 (8) |

Musts: off_mean <= 0.20, off_max <= 0.35, endpx <= 386, area <= 4.27 M. With exact lengths (the prototype estimates
an edge's length from layer indices and breadths; U1 and U2 land at 0.360 when they asked for 0.155) and the row
order of 4.2, the off_max / endpx pair is inside both musts by the numbers above; the area row is not, without the
squeeze. **The depth G3 needs is mostly `PG -> DRAPP` (+113 px) and labels that were already clean** (p3 grew gap 2,
where `DNS -> TR1`'s label sat clean at off 0.17, by 151 px): the cap of 4.2 and "a label already clean at its
default spot keeps it" (decided by the same room test at mid-gap) are the two levers.

### 4.7 route() and route_partial()

The spec puts G3 in `route()` at R (R8); the lead's note keeps it there. At A2 `route()` changes only through the
positions it is given (the quality harness routes at the scene's centres): the prototype leaves `route()`'s code
paths alone (the side channel is set only by `scene_clustered` and cleared after `finish_blocks`; the band logic in
`ortho` is gated on `Mode::Static`). For R the design moves the *decision* into `ortho` so that both entry points
share it: `route()` builds `cols` from the drawn ends (ports, pins, displaced columns), runs 4.2's room tests on the
actual gap (the free band between the rows of boxes `ortho` already reads), places the chip and excludes the
foreign channels exactly as the static scene does; at the scene's centres it then finds the gap the layout
reserved and reproduces the static spot (R8's "that leg runs through the room G3 reserved"). `route_partial()`: a
kept edge keeps its label and its tracks (they are `kept` obstacles to the exclusion); a redrawn edge's gap label is
placed in its gap with the kept chips as obstacles; no new state in the base scene.

### 4.8 Directions

The prototype is TD only. BT mirrors the flow axis (the layer lines are in flow coordinates already; the scene flips
`l`), LR / RL put the chip's *width* along the flow: a band row there is `w + 2 CLEAR_GAP` deep and the breadth test
uses `LABEL_BOX_H`. `lr_label_room` already sizes LR labelled gaps by width; the band need replaces it in block mode.

## 5. The prototype

`L/proto.diff` (588 lines on `7122a53`; `src/layout.rs`, `src/ortho.rs`, `src/scene.rs`; block mode and TD only;
debug prints behind `FMDBG_G3` / `FMDBG_LBL`; knobs `FM_G3_OFF_FRAC` (band depth for off, default 0.17),
`FM_G3_CHAIN_OFF` (slide bound, default 0.25), `FM_G3_OFF` (disable)). What it builds: 4.1's `cols`, the `Upper` /
`Lower` sides with the room tests, row packing by left edge, the band depth, the gap need, the chain slide, the
scene placement, the edge's own step on the far side of its chip, per-jog foreign exclusion, the `straighten` fence,
track exclusion around dummy-row chips. What it does not: `Track`, `Middle`, end widening in X, channel-level
exclusion, router / comb obstacles, the label stage's slot rule, deepest-row ordering, exact lengths, BT / LR / RL,
`route()`.

Measurements (suite-on CLI built from the worktree, local judge copied to `L/judge`, outputs `L/out/<variant>`):

- **A2 judge**: 91 -> 78 (p6), 75 (p3), 70 (p5-c0.34), 71 (p7); `--due A1` 0 in every variant. New must-fail lines
  against `7122a53`: p6 `infra-overview lbl_off_max` and `dense-td lbl_struck` (2, router reroutes through band
  chips); p5-c0.34 `infra-overview lbl_off_max` only; p7 `infra-overview lbl_endpx_mean` and `dense-td lbl_struck`.
- **Rows that moved besides labels** (p6): analog family `lateral_x` 1 -> 2 (`DNS -> TR1`: its `Lower` band sits at
  TR1's port column, which is under PROD's chip; the band must use the displaced column `title_entries` gives), the
  public fixture `lateral_x` 3 -> 4 (`REG ==> [ORD]`), `bends_max` 13 -> 12 on the analog family, crossings unchanged
  everywhere listed, area up 2-15 % on the small block inputs (one or two label rows each), `lbl_crowd` down on
  ratio2x4 / ratio3 / ratio4 / straight-link / wide-stroke / spread-face / dense-td (9 -> 4).
- **Suite-on tests** (`cargo test --release --lib --test quality`, p6 knobs, `L/ton-p6.txt`): 16 failing = the 14 of
  `ton-split.fail` + `a_hubs_fan_keeps_its_order_through_the_bands` and
  `an_end_of_a_title_row_member_keeps_out_of_its_chips_breadth_below_the_jog_room` (both: `fan_x n38 Top n29->n38 x
  n31->n38`, `x n25->n38`, four lines; cause in 4.4).
- **Bulk** (`b8r1/bulkfc`, 1211 files, harness static `chip_crowd`): 657 -> 592, `pin_corner` 45 -> 45,
  `x_drawn` +45; section 7.
- Pictures (`L/pic`): `fan-dns.pair.png` (the DNS fan, `7122a53` | p6: three chips on their own legs in two rows,
  the steps below them), `pg-drapp.pair.png` (the SSH chip on PG's leg under PROD, clean, but 36 px from PG),
  `r2-prod.pair.png` (R2's stand-in label in the lower band above PROD, its two siblings stepping above it),
  `analog-mm-small.png` (mermaid: the same fan, labels on the sources' legs, steps below).

## 6. Per offender: what the design does, what the prototype measured

| input | row(s) at `7122a53` | design | prototype |
|---|---|---|---|
| analog | crowd T1>DNS, own_gap U1/U2 16.5 | `Upper` for all three (fan into DNS: sources free), 2 rows, deepest row to U1/U2 | crowd 0, own_gap 0.05; off U1/U2 0.43 (p6) / 0.348 (p7) |
| analog | crowd+struck PG>DRAPP | `Upper` on PG's leg; depth for off capped -> `Track` needs X room (passing columns 215 px apart for a 385 px stretch) or the +113 px depth | clean at 30 px gap; off 0.463 (p6) / 0.342 (p7) |
| analog | crowd+in_end LANE>GITLAB | border gap PROD -> STG, `Upper` on its column | gap 49.7, in_end 0 |
| analog | crowd R2>[PROD] | `Lower` (R2's three exits share its side; the pin is free) | gap 57.0 |
| analog | gap_min 0, own_gap 16.5 | the above | 12.5 / 0 |
| analog | endpx 383 (pass) | slide + depth cap | 381 (p6) / 383-396 (p5, p7) |
| perm-a, perm-d | as the analog | as the analog | all label rows pass (p6) |
| perm-c | as the analog | as the analog | T1>DNS still struck: the label stage moved the chip 73 px off its line (its spot judged covered); to trace with 4.3.6 |
| perm-b | crowd 8, in_end 1 | as the analog + `Middle` for DNS>SITE, `Track` for DNS>TR1 | crowd 8 -> 2-4, in_end 0 |
| perm-e | crowd 5, in_end 1 | ARGO>TG / LANE>OAI cross inside the gap: `Middle` row with forced sides keeps the crossing off both chips | crowd 5 -> 3, in_end 0 |
| v2 | struck 3, crowd 6, in_end 1, endpx 385 | T1>DNS / DRAPP>PG `Upper` (done); DNS>DRAPP `Middle` (DNS's exits above, DRAPP>TG below); DNS>SITE, DRAPP>TG `Track`/`Middle`; ARGO>GITLAB needs the label stage's slot rule | struck 1, crowd 4-5, in_end 1, endpx 373 (p6) / 335-341 (slide) |
| public x5 | crowd 2 | `webhooks`: straighten fence; `place order`: on a lateral entry, fixed by the lateral fix (B12.8 X2.5/X3) or a band once the entry is flow-facing | crowd 1 (p6), 0 (with the slide: the chips moved with it) |
| dense-td (f) | struck 0 (must: not above the reference) | router obstacles (4.3.5), channel exclusion (4.4) | struck 2 (p6), 0 (p5-c0.34); crowd 9 -> 4 |

## 7. Bulk `chip_crowd`

The verifier saw bulk `chip_crowd` grow over step 2 (bulkfc static 610 -> 657). G3's mechanism is general (any gap
label, any labelled column), so the bulk number is the regression test of its breadth. Measured on `b8r1/bulkfc`
(1211 files) with the suite-on harness (`qcol.sh`; base = `B/work/a2i/quality-on-split`, which reproduces the
verifier's 657 / 45; prototype = the same binary type built from the worktree, p6 knobs; `L/bulkfc-*.tsv`):

| bulkfc TOTAL | `7122a53` | prototype p6 |
|---|---|---|
| static `chip_crowd` | 657 | **592** (below step 2's start, 610) |
| static `pin_corner` | 45 | 45 |
| static `x_drawn` | 86 300 | 86 345 (+45: 14 files worse, one by 22, 1 better) |
| routed `chip_crowd` / `x_drawn` | 690 / 98 425 | 687 / 98 441 (positions only) |

27 of the 1211 files change at all; `chip_crowd` falls in 24 and rises by 1 in 3. The `x_drawn` rise is the
per-jog exclusion of 4.4 reordering a channel (the same cause as dense-td's `n38`), to be re-measured with the
channel-level rule.

`b8r1/bulkst` (964 state files, same binaries, `L/bulkst-*.tsv`): static `chip_crowd` **778 -> 741** (the verifier's
770 -> 778 over step 2: back below the start), 45 files better, 13 worse; `pin_corner` 238 -> 238; static `x_drawn`
308 -> 307; routed `chip_crowd` 799 -> 800, routed `x_drawn` 459 -> 451.

## 8. Plan for the follow-up package (order)

1. `cols` and the gap-label sides `Upper` / `Lower` with their room tests (pins, displaced title-side columns), the
   row packing with deepest-row order, the band need in G1, export with the placement; test
   `a_gap_label_rides_its_free_end_in_a_band` (public fixture copy with a labelled fan into one side, TD and BT).
2. Ortho: own step on the far side; channel-level exclusion; `straighten` fence; tests
   `a_label_slot_is_clear_of_foreign_lines` (spec 4.4's name) on the public fixture and on `blocks/lr`, and the two
   hub-fan tests green.
3. Router / comb / nudge obstacles and the label stage's slot rule; `a_label_sits_on_its_own_line` (no chip moved
   off its line, all block fixtures, four directions).
4. Chain slide and band depth with exact lengths; check the analog family's off / endpx pair; together with the
   squeeze, the area rows.
5. `Track` and `Middle` (v2's DNS fan, perm-b, perm-e); then BT / LR / RL.
6. At R: the same decision in `ortho` for `route()` (4.7).
Verify each step with the copied local judge (label rows per input), `--due A1` 0, the suite-on list (no new names),
the bulk `chip_crowd`, and the default build byte-identical (all of it is behind block mode).

## 9. Risks

- **Hub fans and channel order.** Any exclusion that shrinks single jogs' rooms can reorder a channel (dense-td's
  `n38`, bulkfc +45 `x_drawn`). The channel-level rule is a must, and dense-td's two hub-fan tests are the check.
- **Area and `lbl_endpx_mean`.** Every band row and every px of depth for `lbl_off` lengthens the chains crossing
  that gap; the analog's area must already fails without G3 and needs the squeeze; the analog's endpx sits a few px
  either side of 386 depending on one constant. Judge G3 and B12.8 together, never one without the other.
- **Layout estimate vs drawn frames.** Band positions are computed from `reach_of`; frames grown by F2 / F3 / X2.2
  after the layout can reach into a band (v2 `ARGO -> GITLAB`, the analog family's `DNS -> TR1` under PROD's chip):
  the band must be measured from the drawn frame (or `ortho` must re-seat the chip inside the gap it reads off the
  scene).
- **Router and label stage.** Both move chips today; without the obstacle and slot rules (4.3.5, 4.3.6) the band
  is lost after it was made (dense-td struck 2, perm-c's T1 chip moved 73 px off its line).
- **Directions.** Nothing measured outside TD; LR / RL bands are a chip's width deep.
- **`lbl_off_max` on `PG -> DRAPP`.** Only depth (+113 px) or breadth for a `Track` placement (a 385 px stretch where
  passing columns stand 215 px apart) meets it; neither is free. If both are refused, this one row stays open and
  goes to the lead with its numbers (D6), never to a looser gate.

## 10. Evidence files

- `L/proto.diff`: the prototype (on `7122a53`); `L/proto-p1.diff`: its first stage (bands only).
- `L/out/<variant>/`: SVGs, `metrics.tsv`, `judge-A1.txt`, `judge-A2.txt` for `base`, `p1`, `p3`, `p6`,
  `p5-c0.34`, `p7-0.17-0.34` and the others named in 4.6; `L/judge/`: the copied judge, thresholds, references.
- `L/ton-p6.txt`, `L/ton-p6.fail`: suite-on tests with the prototype (p6 knobs).
- `L/bulkfc-{base,p6}.tsv`, `L/bulkst-{base,p6}.tsv`: bulk harness rows.
- `L/pic/`: `fan-dns.pair.png`, `pg-drapp.pair.png`, `r2-prod.pair.png`, `analog-mm-small.png`, `analog-now.png`,
  `v2-now.png`, `public-now-crop.png`, `p1/infra-overview.pair.png`.
- `L/tools/bm-near` (blockmetrics with the nearest stroke's side and segment), `L/tools/near.sh`,
  `L/tools/lblstat.sh`; `L/fm-base`, `L/fm-p1`, `L/fm-p3`, `L/fm-p5` (+ knob wrappers `L/fm-p6`, `L/fm-p5-c0.34`,
  `L/fm-p7-0.17-0.34`), `L/quality-p5`.
