# Checkpoint A2: geometry (B12.7-B12.12)

## b12a2b-base (2026-11-17, on top of `7122a53`; read this first)

**Fix round 3 (2026-11-18; read this first).** The verifier of `6e1bf72` found item 3's title strikes not fixed in
general (#1, BLOCKING: `router::nudge` undid the ports stage's cut), a new graze from `6e1bf72` (#2, BLOCKING), how
common both are (#3), the lead's open ruling on `blocks.swimlane-nested-TD` (#4) and two non-blocking points (#5,
#6). Commits: **`ffc3c5d`** (the fix, its tests and the verification below) and `5061607` (two code comments that
called the column of #2 a displaced entry; comments only, output byte-identical; head). Checks: `b12/tools-a2b/cand-ffc3c5d/` with `cand-ffc3c5d.cmp` (against
`base-7122a53`) and `cand-ffc3c5d/compare-vs-cand-6e1bf72.txt` (against the pre-fix head); work files, stage traces,
pictures and title-gap tables in `B/b12a2b/fr3/` (`B` = `scratchpad`).

**Result at the head (`5061607`, drawing as `ffc3c5d`) against `7122a53`:** local judge `--due A2` **89 -> 86** must-fail lines, `--due A1`
**0**, routed 37 -> 37 (cells identical to `6e1bf72`'s verdict: no cell worse, 3 gone, 7 better); suite-on failing
tests: the same **14** names, **no gate message new or worse** (1218 tests against 1208; one more than `6e1bf72`, the new router test, which passes);
direction copies: static `lateral_x` 216 -> 201, `side_ends` 231 -> 216, the other verdict columns unchanged, routed
and the 32 example copies unchanged; **the one copy marked worse is still `blocks.swimlane-nested-TD` static
`min_edge_gap` 40 -> 25 (item 3's X1, the lead's ruling, #4), so `compare.sh` still says `VERDICT: WORSE` on dirs and
decision 7's acceptance is formally unmet until the lead rules.** Against `6e1bf72`: `VERDICT: no worse` (no verdict
column moves; informational lines below). Default build byte-identical (off-identity test green; `render.sh
post-b12a2b-base-fr3`: 25 / 25 corpus SVGs byte-equal to `pre-b12a2b-base`).

### Fix round 3: the verifier's six points

1. **#1 (BLOCKING) and #2 (BLOCKING): fixed in `ffc3c5d`, by one chip measure through the static scene's stages.**
   Traced with stage dumps (`fr3/x/`, debug variants in `B/b12a2b/bin-fx`, `FMX` switches, since removed):
   - **#1 (`Shipping and returns desk`, the fan `a9-TD`)** is as the verifier found it: `place_ports` cuts `x`'s top at
     305.6 (ink 299.1 + `CHIP_CLEAR`), and the channel nudge's spread, whose fences grow the chip at the regular width
     by `CHIP_SEP + SEP_AIR`, slides `s1 --> x`'s port leg to 298.0.
   - **#2 (`Mammoth Wombat Pipeline Overview`, `a1-TD` / `w6`) has another cause than the one reported:** `ortho`'s
     title sides displace nothing on that input (no displaced crossing; `x`'s top reaches past the chip, so it is not
     "wholly under" it). The ports stage finds no room past the ink for the side's four ends (its port range ends at 367.0,
     the ink's clearance at 367.7) and, as at 7122a53, spreads them across the side under the title; `ortho` draws
     `s4 --> x` straight in through the chip; **the router's repair** reroutes it round the chip at the regular width
     (`chip_reach` = `CHIP_SEP` past 352.3), to the lattice column 361.4, 0.4 px from `w`; the nudge leaves it. At
     `ebe3d88` the pin `s5 --> G` stood at 358.9 and blocked that column, so the repair took 367.0; `6e1bf72`'s
     `free_pins` moved the pin past the ink and freed it. (So `TitleSides`' regular measure did not cause this
     graze; it is still changed below, for the one-measure rule.)

   What `ffc3c5d` changes (block layout, static scene only; `inked` = `Ctx::blocks`):
   - **router (nudge, close-pair spread):** `World::new(g, sc, inked)`; the world keeps `chips` at the regular width
     and adds `inks`, each title's ink past its chip. The spread fences every leg off it as off the chip
     (`single_stays`, `place_group`), except in a group made only of that frame's own pins: a pin ends on the
     border above the text (`World::ink_of`). (Fenced for a pin too, the public fixture drawn TD's `REG --> ORD`
     pin, 1 px past `Order processing`'s ink and a port's spacing from `LB --> QUEUE`'s crossing, was pushed past
     both and the entry took a new step: `min_edge_gap` 12.72 -> 11.1 on the 8 TD copies.) The scoring counts a
     graze of the ink as **`Score::ink`**, ranked right below a chip hit and outside `Score::hard`. (Counted as a
     chip hit, the graze made a path straight through the title score no worse than the grazing one: the nudge
     merged `s4 --> x`'s step away and drew it through `Overview` at 0.00 px.)
   - **router (repair): kept at the regular width, with numbers** (documented at `router::repair`). Inked
     (`fr3/q-Q2`), the
     repair reroutes for grazes: blocks/lr drawn BT `cart --> pager` goes round into pager's side across
     `ship --> pager` (`x_drawn` 1 -> 2, `side_ends` 1 -> 2), drawn TD `shopper --> cdn` comes in through its
     frame's lateral wall (`lateral_x` 0 -> 1), dense-td TD `side_ends` 1 -> 2. The nudge after the repair keeps its
     columns off the ink (on `a1` it moves `s4 --> x`'s column to 6.18 px).
   - **ortho:** the jog walls measure to the ink; `TitleSides`' first displaced column starts at
     `max(chip + CHIP_CLEAR + CORNER_R, ink + CHIP_CLEAR)`. **Which ends are displaced stays judged at the chip, with
     numbers:** judged at the ink (`fr3/q-PCWSNTUV2`), quality/dense-td drawn TD `defects` 0 -> 1, `x_drawn`
     42 -> 43 (in the first form, `fr3/q-T`, 42 -> 46 and `fan_x` 0 -> 4, as the wip of fix round 2 saw); starting
     the columns at the ink's clearance alone, without the max (`fr3/q-PCWSNU3`), gave blocks/nested-chain drawn BT a
     side end (`b2 --> c` into `c`'s side, `fr3/x/nc-U.png`).
   - **comb:** its chips reach the ink.
   - **ports, `title_side_columns`:** the frame's pins standing past the chip as drawn are dealt with the title-row
     entries even when they stand under the ink past it. Left out (as in `6e1bf72`), `a9`'s pin kept 297.5 while the
     entries were dealt past the ink, and the inked nudge pushed it in among them, 4.9 px from `s2 --> x`
     (`min_edge_gap` 14.33 -> 4.89, a harness defect; `fr3/qsw-*.tsv`). Dealt with them it comes in last, past the
     entries (`x_drawn` 2 -> 0, `side_ends` 1 -> 0, `min_edge_gap` 14.0). A trial that instead held such a pin out
     of the nudge's group kept it left of the fan (4 hops) and moved blocks/turned-blocks drawn RL (`min_edge_gap`
     10.26 -> 7.76): not taken.
2. **#3: covered.** Title gaps (Chromium `getBBox` of the bold title to the nearest edge centreline: the verifier's
   `tgall.js` on the sweep, `fr3/tg-sw-final.txt`; on the 42 drawings of the 364 copies, the 95 local inputs and the
   verifier's inputs that differ from `6e1bf72`, `fr3/tools/tgquick.js`, the same measure with the paths parsed from
   their `d` (hop arcs by their chord; it agrees with `tgall.js` to 0.01 px on the checked files),
   `fr3/tg-diff4q.txt`), 7122a53 / `6e1bf72` / `ffc3c5d`:

   | drawing | title (edge) | 7122a53 | 6e1bf72 | ffc3c5d |
   |---|---|---|---|---|
   | fan `a9` TD | Shipping and returns desk (`s1 --> x`) | 11.86 | 0.00 | **6.92** |
   | fan `a1` TD / `w6` | Mammoth Wombat Pipeline Overview (`s4 --> x`) | 5.95 | 0.36 | **6.18** |
   | fan `b6` BT | Data ingestion workers (`q3 --> m`) | 2.98 | 2.98 | **6.71** |
   | fan `b12` TD | Reporting (`s4 --> q1`) | 6.33 | 6.33 | **6.93** |
   | public fixture drawn BT + 7 copies | Edge and shared services (`CDN --> GATE`) | 1.45 | 1.45 | **6.54** |
   | blocks/lr drawn BT | Edge and shared services with a long title (`cart --> pager`) | 0.00 | 0.00 | **3.11** |
   | quality/dense-td drawn RL | consectetur nostrud lorem (`n16 --> n44`) | 0.00 | 0.00 | **7.17** |
   | quality/dense-td drawn BT | aliqua magna tempor (`n11 --> n37`) | 3.85 | 3.85 | **7.04** |
   | quality/dense-td drawn TD (and its local twin) | adipiscing eiusmod (`n25 --> n42`) | 4.17 | 4.17 | **6.97** |
   | verifier's `v1-pins-and-entries` TD / BT | Order processing and fulfilment pipeline | 3.75 / 5.70 | 3.75 / 5.70 | **6.81 / 6.80** |
   | the analog family (5 local inputs) | its long environment title | 6.12 | 6.11 | **9.81** |
   | fan `a6` BT | Data ingestion workers (`y --> z`) | 1.34 | 1.34 | 1.34 |
   | dense-td TD / BT (unchanged strikes) | lorem labore, consectetur nostrud lorem / veniam sed | 0.00 | 0.00 | 0.00 |

   **No title gap of the 42 changed drawings went down**; every changed one went up (the table; the rest within
   0.1 px). The dense-td strikes at 0.00 are pre-existing and untouched (fix round 2 #7).

   In the 72-input sweep every title drawn TD now clears its nearest line by 6.18 px or more (5.95 at 7122a53, 0.00
   at `6e1bf72`); drawn BT only `a6` stays under 5.5 px, unchanged since 7122a53: `y`'s exit up through the title
   side has no room past the ink on `y`'s narrow top, falls back under the ink, and `TitleSides` does not displace
   it (its side is not wholly under the chip; displacing it is the `T` measure above). Harness on the sweep
   (`fr3/qsw-{base,head,new}.tsv`, static): `a9` `min_edge_gap` 40 / 14.33 / 14.0, `x_drawn` 3 / 2 / 0; `a1`
   `min_edge_gap` 40 / 10 / 13.3; no verdict column worse on any of the 72. Every changed drawing of the 364 copies,
   the 95 local inputs and the verifier's inputs against `6e1bf72` is in `fr3/diff4/` (42 SVGs); pictures
   7122a53 | `6e1bf72` | `ffc3c5d`: `fr3/pic/sw--a9-TD.png`, `a9-zoom.png`, `sw--a1-TD.png`, `a1-zoom.png`,
   `sw--b6-BT.png`, `vin--v1-pins-and-entries.png`, `dirs--blocks.lr-BT.png`, `lrbt-zoom.png`.
   Informational against `6e1bf72` (no verdict column): blocks/lr drawn BT's `cart --> pager`, moved off the title
   (0.00 -> 3.11 px), now runs beside `ship --> pager` (`tight_pairs` 0 -> 1, `tight_len` 0 -> 182, `chip_crowd`
   0 -> 1, `chip_gap_min` 25.38 -> 4.48); the public fixture's BT copies `band_len` 488 -> 592 and `lat_backtrack`
   340.8 -> 345.9 (`CDN --> GATE` moved off the ink); `edge_len` +5 to +10 px on the public family.
   Tests: `blockcheck::a_title_rows_ports_clear_the_titles_ink` extended with the verifier's fan (a member whose top
   reaches just past the chip, four entries and a pin) under four titles, TD and BT (not `Data ingestion workers` BT,
   the `a6` case above), and the long-titled hub input now checks `ORD`'s title too (the comment that excused it
   is gone); it fails at `6e1bf72` on the hub (`hub -> q1` 3.4 px), the `Shipping and returns desk` fan (0.0 px)
   and the `Mammoth Wombat Pipeline Overview` fan (0.1 px) (run in a scratch worktree of `6e1bf72` with only the
   test applied). New: `router::tests::a_titles_ink_past_its_chip_scores_a_graze_below_a_chip_hit`.
3. **#4 (lead's ruling): unchanged.** `blocks.swimlane-nested-TD` static `min_edge_gap` 40 -> 25 is byte-equal to
   `6e1bf72`'s; `VERDICT: WORSE` on dirs until the lead rules.
4. **#5: fixed in `ffc3c5d`.** In a block layout, where the stretch past the ink and `CHIP_CLEAR` cannot hold a side's
   ends, and on an over-full title side, the cut now moves to `CHIP_SEP` past the ink before it falls back to
   `CHIP_SEP` past the chip alone. (On its own this changed no verdict column or cell: `fr3/q-P`.)
5. **#6: coverage restored on a fixture as it is.** A scan of all 364 copies with a print in the dealing
   (`bin-e2scan`, `bin-e2scan2`) finds one unmodified fixture copy where `title_side_columns` deals a pin with an
   entry: blocks/nested-cases drawn TD (`o1 --> I`'s pin, `hub --> i1`'s entry; the entry moves from 1 px inside the
   ink's clearance to it, a track past the pin; with the dealing switched off it stays at 204.9). The e2 test now
   also checks that copy (fails without the dealing); the retitled public fixture stays for the one-column-set case.
6. **The harness's `edge_chip` still measures the chip at the regular width** (quality.rs): unchanged (it feeds the
   judge), a point for the lead.
7. **Found while sweeping, not fixed here (pre-existing in this package, not this round's):** on the verifier's fan
   sweep run through the harness (`fr3/qsw-*.tsv`, static), the `b` shape (three entries and two pins into a title row
   of two members) drawn TD under `Shipping and returns desk` and `Edge and shared services` (`b9`,
   `b11`) has `defects` 0 -> 1, `x_drawn` 0 -> 4, `side_ends` 0 -> 1, `min_edge_gap` 14.6 -> 3.39 against 7122a53.
   Bisected over the stored check binaries: item 3 (`4d374f8`) takes `x_drawn` 0 -> 1 / 2 (`min_edge_gap` stays
   12-16), **item 7 e3 (`3e38be3`) takes it to 4 with the 3.39 px pair and the defect**; `ebe3d88`, `6e1bf72` and
   `ffc3c5d` draw it alike (picture 7122a53 | item 4a | e3 | `ffc3c5d`: `fr3/pic/b9-TD-4.png`: the pins of
   `client` and `batch` hook in beside each other and `caller`'s entry crosses them). `b7` TD gains one crossing at
   e3 too. None of the 364 copies or the 95 local inputs shows it (the three checks were no worse at e3). Open for
   the next round or the lead; the vs-7122a53 `min_edge_gap` drops of the `a` shape (40 -> 12-14 px) are item 3's
   X1 class, the same as `blocks.swimlane-nested-TD` (#4).

Guard `GUARD: ALL GREEN` on `ffc3c5d`'s tree (`fr3/guard-1.txt`) and on `5061607`. Perf (default build; the change is behind
`Ctx::blocks`): `perf.sh b12a2b-base-fr3` and `-r2`: worst drags advanced 2.43 / 2.12 (limit 2.61), lr 1.38 / 1.35
(1.95), stress 1.34 / 1.34 (2.15), back-edge 1.03 / 0.66 (2.00), bench-75 19.26 / 18.30 (25.28); route_ms within
budget (advanced 1.01 / 1.03 against 1.72, dense-td 6.78 / 6.88 against 9.20); static `scene_ms` within 2x but for
back-edge's first run (0.17 against 0.12; 0.10 on the second): noise on a sub-ms file, no regression claimed. Disk:
6.4-7.2 GB free throughout; deleted only scratch dirs made in this round (`fr3/target-head`, the scratch worktree
`fr3/wt-head`); the check runs remove their own target dirs.

**Fix round 2 (2026-11-17, night).** The verifier of the head before the container restart found a
new BLOCKING defect in item 3 (title grazes and strikes in block mode, #1), kept the lead's open question on
`blocks.swimlane-nested-TD` (#2) and five non-blocking points (#3-#7). Commits of the fix round: `0230c6d` (#3, #6: the
e3 unit test on synthetic steps, its last check made real, the `use` order), `58d64c2` (the restart's wip of #1,
unverified), `1d8599e` (the lead's merge of the pushed `ff1d41a`, no tree change), **`6e1bf72` (#1, completes the wip
with its tests and the verification below)**. Head: `6e1bf72`. Checks: `b12/tools-a2b/cand-6e1bf72/` with `cand-6e1bf72.cmp`
(`run-checks.sh --commit 6e1bf72`; the earlier tree build `cand-fr2c/` of the same sources gives the same CLI binary
and an identical report); title gaps, pictures and runs in `B/b12a2b/fr2c/`.

**Result at the head (`6e1bf72`) against `7122a53`:** local judge `--due A2` **89 -> 86** must-fail lines (444
checks), `--due A1` **0**, routed 37 -> 37 (62 checks); per-cell **no cell worse** (3 gone, 7 better inside failing
lines); suite-on failing tests: the same **14** names, **no gate message new or worse** (1217 tests against 1208:
9 new, all pass); direction copies: static `lateral_x` 216 -> 201, `side_ends` 231 -> 216, `defects`, `x_drawn`,
`fan_x`, `edge_chip` unchanged, routed and the 32 example copies unchanged; **one copy worse by the tool's rule, as
before: `blocks.swimlane-nested-TD` static `min_edge_gap` 40 -> 25 (item 3's X1; the lead's ruling is still open,
#2 below), so `compare.sh` says `VERDICT: WORSE` on dirs and the package's acceptance is not met until the lead
rules.** Default build byte-identical (off-identity test green in the guard; `render.sh post-b12a2b-base-fr2`: 25 /
25 corpus SVGs byte-equal to `pre-b12a2b-base`).

### Fix round 2: the verifier's seven points

1. **Item 3 drew lines onto bold titles (BLOCKING): fixed in `6e1bf72`.** Cause as the verifier found it: X1
   (`ports::unshaded`) and e2 (`ports::title_side_columns`, and `free_pins` with them) cleared the chip that
   `scene::title_head` measures, at the regular text width for a frame not grown to its title, while `to_svg` paints
   every title bold; the bold ink runs past that chip (`Order processing`: about 5 px), so the first column past the
   chip's clearance stood on the last letter. Fix: in a block layout these three rules measure the chip to the
   title's ink (`scene::title_head_inked`: the drawn chip or the drawn bold text, whichever ends farther; `ports::
   title_heads(sc, inked)`); where the stretch of a title-row side past the ink and `CHIP_CLEAR` cannot hold the
   side's ends (`ports::unshaded_fit` returns `None`), `place_ports` keeps 7122a53's cut (`CHIP_SEP` past the chip
   alone), as on an over-full title side. Measured by the verifier's method (Chromium `getBBox` of the painted title
   to every edge centreline, `fr2c/tools/titlegap-fast.js`, suite-on CLI; before = `cand-ebe3d88`'s binary, which
   draws as the pre-restart head):

   | drawing | title | 7122a53 | before | after |
   |---|---|---|---|---|
   | public fixture drawn TD and its 7 sibling TD copies (+ the 5 local permutations) | Order processing | 11.96 | 1.97 | **11.96** |
   | quality/dense-lr drawn BT | consectetur ipsum | 26.09 | 1.60 | **6.90** |
   | quality/dense-lr drawn TD | ullamco tempor magna | 32.90 | 1.71 | **6.71** |
   | verifier's `v1-pins-and-entries` (TD) | Observability stack with a long title | 7.50 | 0.00 | **7.50** |

   Over all 364 copies, the 95 local inputs and the verifier's 6 inputs, no title gap is under 4 px after the fix
   that was not so at 7122a53 (`fr2c/tg-pre-new-{dirs,loc,vin}.txt`). Those that are, all unchanged since 7122a53 and
   not this package's: the public fixture drawn BT `Edge and shared services` 1.45 (`CDN -> GATE`), blocks/lr BT 0.00
   (#7), dense-td TD / BT / RL 0.00 and 3.85, bench-75 BT `data zone` 0.00, `v1` `Order processing and fulfilment
   pipeline` 3.76 (`hub -> q1`, displaced past the chip by `ortho`'s title sides, which measure the regular chip; the
   wip's trial of measuring those to the ink fanned dense-td TD's displaced columns across each other, so it was
   left out). Pictures (7122a53 | before | after): `fr2c/pic/bo-td-zoom.png`, `dlr-td-zoom.png`, `dlr-bt-zoom.png`,
   `v1-zoom.png`.

   What the fix gives back of item 3 (all equal to or better than 7122a53, so no cell, gate message or copy is worse
   against the baseline; against the pre-restart head these are worse, named in the commit message):
   - **public fixture drawn TD** (8 copies, judge rows on the fixture and its 4 permutations): QUEUE's top past
     `Order processing`'s ink holds two columns, not three, so `CART --> QUEUE` comes into QUEUE's left side again,
     as at 7122a53; `QUEUE --> CHARGE` keeps item 3's way (no hop across `CART --> QUEUE`). Harness `lateral_x`
     1 -> 2 (3 at 7122a53), `side_ends` 3 -> 4 (5), `chip_crowd` 1 -> 2 (2), `chip_gap_min` 7.39 -> 4.51 (4.51),
     `band_len` 564 -> 462 (462: the verifier's #5 band is gone); judge `lateral_x` 1 -> 2 (3) and `lbl_crowd`
     1 -> 2 (2) on the fixture and its permutations. The drawing equals 7122a53's but for `QUEUE --> CHARGE` (and
     `OPS --> REPO` 1.3 px, `CHARGE --> MAIL` 0.5 px, their labels 0.5 px).
   - **blocks/lr drawn BT**: pager's top (x 276-344) lies under the long title, whose bold ink ends near x 335: past
     the ink and its clearance no stretch is left for its two entries, the side keeps 7122a53's cut, and `cart --> pager`'s 2.2 px S-jog is back (both PAGER edges byte-equal to 7122a53; `lateral_x`
     2 -> 3 = 7122a53; `fr2c/pic/lrbt-tri.png`). Decision 7 (3) said the S-jog goes with X1 + e2: it went only because
     the port stood under the title's ink, where the line strikes the title anyway (#7). Not undone a second way.
   - **blocks/nested drawn TD** (and its local twin): `i1 --> s1` moves 1.3 px (metrics's top is cut at
     `Monitoring`'s ink), and the label stage's scan, which has no G3 rule (item 5, skipped), now takes the line's
     middle inside Platform instead of the gap below it: harness `chip_crowd` 0 -> 1, `chip_gap_min` 29.38 -> 11.86,
     `lbl_end_mean` 181 -> 210; judge (unjudged cells) `lbl_in_end` 1 -> 2, `lbl_crowd` 0 -> 1, `lbl_gap_min`
     29.3 -> 11.7 (`fr2c/pic/nested-td-tri.png`). Traced (debug build): the changed edge's old label spot is off the
     moved line, `best_spot` scans from the preferred spot, and the middle, 11.45 px from `s2 --> IN`'s run, now
     counts as clean (at 1.3 px to the right it did not, and the scan went on to the gap). Informational; it belongs
     to G3 in the label stage (item 5).
   - Other block drawings: ports 0.1-8 px (nested BT, nested-cases TD, p14 TD, swimlane BT, swimlane-nested BT,
     dense-td BT / TD, fanout-frames TD, lr TD, the public fixture's BT family 1.6-2.5 px), dense-lr BT `n22 --> n44`
     on another track of its gap (`fr2c/pic/dlr-bt-n22.png`); p04-to-cluster TD and `v1` byte-equal to 7122a53.

   Tests: `blockcheck::a_title_rows_ports_clear_the_titles_ink` (from the wip: every line keeps `CHIP_SEP` off the
   title's ink box on the public fixture and its permutations TD, dense-lr TD / BT, and a long-titled hub input;
   fails before the fix: `CART -> QUEUE 1.5 px from ORD's title`), `scene::tests::an_inked_title_head_reaches_the_bold_
   text_of_a_frame_not_grown_to_it`, `ports::tests::a_cut_that_leaves_no_room_for_the_ends_is_told_apart`. Changed
   (in the wip, named in `6e1bf72`'s message): `blockcheck::a_frames_pins_and_its_title_rows_entries_take_columns_
   apart` titles the fixture's ORD frame `Orders`, so the stretch past the ink still holds the pin and both entries
   and the test keeps pinning e2's one column set. The harness's `edge_chip` (quality.rs) also measures the regular
   chip, which is why the three checks did not see the graze; it is not changed here (it would move the default
   build's quality numbers and the gates' inputs): a point for the lead.
2. **`blocks.swimlane-nested-TD` `min_edge_gap` 40 -> 25: unchanged, still the lead's ruling.** The fix of #1 does
   not touch it (the copy is byte-equal to the pre-restart head's).
   The question of fix round 1 stands: may a direction copy's `min_edge_gap` drop when its `lateral_x` / `side_ends`
   drop with it and the gap stays above every gap floor (25 >= 12.1)? The verifier read the head's drawing as the
   better one. Until the lead rules, `VERDICT: WORSE` on dirs and the acceptance of decision 7 is not met.
3. **e3's unit test held local acceptance numbers (D5): fixed in `0230c6d`.** Synthetic steps (rooms from 100, a
   jog-room step in 10 .. 28, a step beside the frame whose room reaches down to the jog room), the comment no longer
   narrates a drawing; the test still fails under the old rooms-only chaining and now also asserts the sets
   `split_by_room` makes. The vacuous `flat.len() == 6` is replaced: outside a block layout `split_by_room` returns
   one set and `place_tracks` places the six bit-equal to `place_channel` on the whole group, and the block layout's
   placement differs from it.
4. **scene() and route() disagree on ends these rules placed (static-only scope): R scope, unchanged.** As documented
   in `ebe3d88` (code docs of `ports::unshaded`, `ports::title_side_columns`, `router::World::hug_gap`,
   `ortho::split_by_room`) and in "Scope" below; the ink rule of #1 is keyed on `Ctx::blocks` too and runs in the
   static scene only. It must be closed in B12.13 (R), with the block paths `route()` does not carry yet.
5. **Informational, from item 3, re-measured at `6e1bf72`:** wide-stroke BT `chip_gap_min` 36 -> 8.98 and
   `chip_crowd` 0 -> 1 stay (the copy is byte-equal to the pre-restart head's: the dotted `done` step still lands
   9 px from `submit`'s chip; a G3 / label-stage matter, item 5); dense-lr TD `chip_crowd` 3 -> 4 stays (4 before and
   after the fix, `chip_gap_min` 10.25 both); the public family's `band_len` 462 -> 564 is **gone** (back to 462 with
   `CART --> QUEUE`'s old way).
6. **Test placement in the ortho tests module: fixed in `0230c6d`** (the `use` lines first).
7. **Pre-existing, for the record (not this package's, unchanged at `6e1bf72`):** blocks/lr drawn BT `cart -> PAGER`
   strikes `Edge and shared services with a long title` (0.00 px at 7122a53, before and after); the dense-td copies'
   titles struck at 0.00 px; the advanced example's `Users > Mon` picks and the diagonal waypoint segments in
   `route()`. The fixture's lr BT strike is a title-row member whose whole top lies under the title: no port cut can
   clear it; it needs X3's displacement or a grown frame (item 4b / squeeze).

Guard `GUARD: ALL GREEN` on the wip state and on `6e1bf72` (`fr2c/guard-wip.txt`, `fr2c/guard-final.txt`). Perf
(default build; the fix changes no default code path): `perf.sh b12a2b-base-fr2` and `-r2`, every drag and route
time within budget (worst drags advanced 2.02 / 2.07, limit 2.61; lr 1.41 / 1.38, 1.95; stress 1.35 / 1.34, 2.15;
long-labels 0.93 / 1.47, 2.31; bench-75 16.32 / 18.90, 25.28; route_ms dense-td 7.98 / 6.98, 9.20); static
`scene_ms` within 2x but for two sub-ms files whose runs disagree (demo 0.18 / 0.13 against 0.14, long-labels 0.09 /
0.13 against 0.10): noise, no regression claimed. Disk: 7.2 GB free at the start of the fix round, 7.3 GB at its end;
nothing deleted (the scratch target `B/b12a2b/target-on` is 37 MB).

Still open after fix round 2: the lead's ruling on #2; items 4b (X3 with its `route_title_jogs` half and the pins
counted in F2 there), 5 (G3 static), 6 (X2.5, the gap-step merge), 7 e4 / e5, each skipped below with its numbers;
the routed half of every rule (R, #4).

### Before fix round 2

Commits (branch `ccr-ccc6d89a-5cxd7e`, not pushed): `360c588` (item 1), `57d0736` (item 2), `4d374f8` (item 3),
`b53e3de` (item 4a, lateral X2), and from the fix round `3e38be3` (item 7, e3) and `ebe3d88` (comments: the static-only
scope). Items 4b (X3), 5 (G3), 6 (X2.5 and the gap-step merge) and the e4/e5 half of item 7 are **not committed**:
each skipped below with its numbers. Work files: `B/b12a2b/` (`B` = `scratchpad`; scripts
`tools/{on,quick,bulk,png,pair}.sh`, `tools/bulkcmp.py`; quick runs `q-<label>/`; pictures `pic/`; bulk TSVs
`bulk/`; the measured but uncommitted attempts `x3-attempt.diff`, `g3-proto-on-head.diff`, `x2-scene-pad.diff`,
`fr1/e4-attempt.diff`, `fr1/e5-probe.diff`). The three mandatory checks per committed item are
`b12/tools-a2b/cand-item{1,2,3,4a}/`, `cand-e3/` and `cand-ebe3d88/` (the head), each with its `.cmp`
(compare.sh against `base-7122a53`).

**Result at `ebe3d88` (the head before fix round 2) against `7122a53`:** local judge `--due A2` **89 -> 86** must-fail lines (444
checks), `--due A1` **0**, routed 37 -> 37 (62 checks); per-cell: **no cell worse** (3 gone, 12 better inside failing
lines); suite-on failing tests: the same **14** names, **no gate message new or worse** (6 new tests, all pass);
direction copies: static `lateral_x` 216 -> 192, `side_ends` 231 -> 208; `defects`, `x_drawn`, `fan_x`, `edge_chip`
unchanged; routed unchanged; **one copy worse by the tool's rule: `blocks.swimlane-nested-TD` static `min_edge_gap`
40 -> 25, made by item 3's X1 and kept to the end. Decision 5 does not cover it (the row passes); it needs the lead's
ruling on its class (fix round #1 below). Until then `compare.sh` says `VERDICT: WORSE` on dirs and the package's
acceptance is not met.** Default build byte-identical: off-identity test green in every guard; `render.sh
post-b12a2b-base` and, after the fix round, `render.sh post-b12a2b-base-fr1` are byte-identical to
`pre-b12a2b-base` on all 25 corpus SVGs. The head's 95 local SVGs, 95 routed twins and 364 copies are byte-identical
to `3e38be3`'s (`cand-ebe3d88` against `cand-e3`). No CHANGELOG / README / version change (block mode is behind the switch).

**Scope (corrected in the fix round):** every rule of this package (items 1, 3, 4a, 7) is keyed on `Ctx::blocks`,
which only the static scene sets (`edgepass::finish_blocks`). `route()`, `route_sized()` and `route_partial()` build
their context with `blocks: false` (`edgepass::ctx`), so none of these rules runs there: the routed table (37 -> 37)
and the routed copies do not move. The first version of this section said of item 4a that "`route()` takes the same
rule (the repair runs there with `cx.blocks`)": **that was false**. In block mode a drag can therefore move an end
these rules placed (blocks/hub5 TD: `scene()` ends `HUB -> b1` on b1's top at (311.07, 425), `route()` at the same
centres on its left side at (271.2, 444); at `7122a53` the two agreed). The routed half belongs to B12.13 (R, pulled
into `b12a2b-squeeze` only for R3's corridors, decision 4); `ebe3d88` says so in the docs of `ports::unshaded`,
`ports::title_side_columns`, `router::World::hug_gap` and `ortho::split_by_room`.

| item | commit | A2 | A1 | routed | cells | gate messages | copies (vs previous item) |
|---|---|---|---|---|---|---|---|
| 1 regress `split_by_room` by plain room, fixture `twoway-noted` | `360c588` | 89 -> 89 | 0 | 37 | none changed | none; +1 test | none changed (static, routed, 364) |
| 2 `TitleSides::of` one pass | `57d0736` | 89 -> 89 | 0 | 37 | none changed (95 SVGs byte-identical) | none; +1 test | none changed |
| 3 lateral X1 + e2 | `4d374f8` | 89 -> 88 | 0 | 37 | gone: hub5 `lateral_x` 3->0; better: public x5 `lateral_x` 3->1, `lbl_crowd` 2->1 | none; +2 tests | static `lateral_x` -24 (14 copies better), `side_ends` -23 (13); **swimlane-nested TD `min_edge_gap` 40->25 (lead's ruling needed, fix round #1)** |
| 4a lateral X2 (router form) | `b53e3de` | 88 -> 87 | 0 | 37 | gone: v2 `lateral_x` 1->0 | none; +1 test | none worse; info: fork-twins TD `lbl_end_mean` 183->192 |
| 4b lateral X3 | skipped | (86) | (0) | (37, 1 cell worse) | | | 16 worse (below) |
| 5 G3 (labels p5, no dummy-row exclusion) | skipped | (66) | (0) | (36, 13 cells worse) | 17 worse | | 23 worse (below) |
| 6 X2.5 breadth bound; displaced gap-step merge | skipped | | | | | | |
| 7 e3 (a jog-room step chains only the steps it stacks against) | `3e38be3` | 87 -> 86 | 0 | 37 | gone: infra-bt `lateral_x` 1->0; better: v2 `lbl_crowd` 6->4, `lbl_endpx_mean` 385->381 | none; +1 test | none changed (364 copies identical to item 4a's) |
| 7 e4 corner legs for ends leaving their block sideways (two variants) | skipped | (87) | (0) | (37) | NEW dense-lr `lbl_struck` 6 / 8 | not run | 8 / 11 worse (below) |
| 7 e5 LR strip band for inner chain dummies (layout probe) | skipped | (86) | (0) | (37, 1 cell worse) | 1 worse | not run | 57 worse (below) |
| fix round #3: docs of the static-only scope | `ebe3d88` | = | = | = | = (comments only) | = | = |
| fix round 2 #3/#6: e3 test synthetic, real last check | `0230c6d` | = | = | = | = (tests only) | = | = |
| fix round 2 #1: ports and pin columns clear the title's bold ink (completes wip `58d64c2`) | `6e1bf72` | 86 -> 86 | 0 | 37 | none worse vs 7122a53; vs `ebe3d88` 10 worse inside failing (public x5 `lateral_x` 1->2, `lbl_crowd` 1->2; = or better than 7122a53) | none; +3 tests | vs `ebe3d88`: `lateral_x` +9 (public TD x8 1->2, lr BT 2->3), `side_ends` +8; all = or better than 7122a53; title gaps fixed (above) |

### Fix round 1 (verifier of `b53e3de`), 2026-11-17 evening

The verifier's five points and what was done:

1. **`blocks.swimlane-nested-TD` `min_edge_gap` 40 -> 25 (BLOCKING): not changed in code; it needs the lead's ruling
   on its class.** What the fix round found:
   - **It is X1's, not e2's.** X1 alone (`B/b12a2b/q-x1`, measured before item 3 was committed) gives the same
     40 -> 25; Shop has no frame-level link, so e2 (`ports::title_side_columns`) has no pin to deal there. X1 is
     decision 7's item (3) and the cause of hub5's gone `lateral_x` line (3 -> 0) and the public family's QUEUE>CHARGE.
   - **25 px cannot be raised to 40 by spacing.** Probe (`B/b12a2b/fr1/swim.mmd`, debug build): Checkout's bottom
     at y 160, Shop's top at 222.5: a 62.5 px gap. `place order -> confirm order` now comes in through Shop's title
     side (track y 208, column 416.1), `confirm order -> pay invoice` leaves the same side (track 195.5, column
     430.1) and `pay invoice -> ship goods` runs at 183. That order is the only crossing-free one (the entry turns
     down left of the exit's column, the exit turns up right of the entry's). The pair the harness measures is
     `place order -> confirm order` against `pay invoice -> ship goods`, which share no end: two tracks, 25 px. Over
     40 px would need tracks 20 px apart in a gap whose rooms (BLOCK_BORDER_CLEAR off both walls) hold 33.5 px. The
     only drawing that keeps 40 is the old one, the entry through Shop's left wall (a `lateral_x` and a `side_ends`
     more on this copy).
   - **The class.** `min_edge_gap` counts only pairs that neither cross nor share an end, capped at 40. Any change
     that takes an edge out of a lateral detour into the channel its neighbours use, or removes a crossing between
     two edges that then run a port's spacing apart, lowers it: the verifier's `s1-titlerow-pins` TD (40 -> 14:
     `External caller`'s pin and `Hub -> gamma`'s entry a `PORT_SPACING` apart once they no longer cross; `x_drawn`
     2 -> 1) is the same class. Mermaid draws both inputs the way the head does: `place order -> confirm order`
     straight down into confirm order's top through Shop's top, and the two callers' pins side by side 12 px apart
     next to `Hub -> delta` (`B/b12a2b/fr1/pic/swimlane-nested-TD.mermaid-base-head.png`,
     `B/b12a2b/fr1/pic/s1-titlerow-pins-TD.mermaid-base-head.png`: mermaid | 7122a53 | head).
   - **Question for the lead:** may a direction copy's `min_edge_gap` drop when the copy's `lateral_x`, `side_ends`
     or `x_drawn` drops with it and the new gap stays above every gap floor (here 25 >= 12.1)? Undoing it means
     undoing X1 for title-row entries (hub5's A2 line, the public family's `lateral_x` 3 -> 1 partly, lr BT's S-jog).
     Nothing in this fix round reverts it; no later item pays it back.
2. **Item 7 attempted and measured.** e3 is committed (`3e38be3`); e4 and e5 were built, measured and not committed
   (below, "Item 7").
3. **Report inaccuracy and the static-only scope:** corrected above ("Scope") and in the code docs (`ebe3d88`).
4. **Non-blocking visual notes:** (a) wide-stroke BT `chip_gap_min` 36 -> 8.98 and `chip_crowd` 0 -> 1 come from X1
   alone (`q-x1`: the same 1 / 8.98): page sits in Front's title row, its top side is now cut at `CHIP_CLEAR`
   instead of `CHIP_SEP`, its entries move along it and the dotted `done` edge's step lands 9 px from the `submit`
   chip, whose own edge's step still hides behind it (as at 7122a53). Not changed: a label-stage matter (G3's slot
   rules, item 5, skipped). (b) `s4-nested-hub` TD, (c) lr BT / hub5 TD `bends_max` +1, (d) the public family's
   `band_len` 462 -> 564 / `band_pairs` 2 -> 3 on `CART -> QUEUE` along Order processing's top: informational,
   all from item 3 (X1 + e2), unchanged by the fix round; not traced further.
5. **Block-mode drag on dense-td** (`fixture_drag_perf_guard`, route_partial worst 8.53 ms at 7122a53, 8.54 ms at
   `b53e3de`, above PLAN's 6 ms): already so before the package, R scope (B12.13), not changed here (the package's
   rules do not run in `route_partial()`, see "Scope"). Recorded for `b12a2b-squeeze` / R.

### Item 7: e3 (`3e38be3`), e4 and e5 (skipped with numbers)

- **e3, committed.** Probe (debug build, `FMDBGJ`): on infra-bt (BT) the channel of the gap above PROD held 6 steps;
  `split_by_room` chained PG's jog-room step (room -695.9..-677.9, inside PROD under its title strip) with the gap's
  steps (rooms from -606.8) through `DRAPP -> TG`'s step, which runs beside PROD and whose room reaches -690.4.
  Ordered as one stack the jog-room step stood between `R2 -> DR` and the gap's later steps, and both `PG -> DRAPP`'s
  run on and `R2 -> DR` were clamped to -606.8, one track; the repair (class 0, overlap 950) took `PG -> DRAPP` out
  through PROD's left wall. The study's reading ("the displaced step is not in `place_tracks`' channel set") was not
  the mechanism: it was in the set, ordered against a step it never stacks with. Rule: two steps chain only where
  their spans meet within a track's pitch and their plain rooms within the least pitch (union of such pairs).
  A2 87 -> 86 (infra-bt `lateral_x` gone), v2 `lbl_crowd` 6 -> 4 and `lbl_endpx_mean` 385 -> 381 better, A1 0,
  routed unchanged, gates unchanged, all 364 copies identical to item 4a's (seven local acceptance drawings change,
  no repository fixture). Picture: `B/b12a2b/fr1/e3a-infra-bt-zoom.png` (top before, bottom after: the run now
  leaves the jog room past PROD's chip and takes its own track in the gap). Unit test
  `ortho::tests::a_jog_room_step_does_not_chain_a_gaps_steps_onto_one_track` (fails with the old chaining).
- **e4, skipped** (`B/b12a2b/fr1/e4-attempt.diff`, runs `q-e4a`, `q-e4b`, compared with `cand-e3`). dense-lr's 4
  `lateral_x` crossings of this class are `n12 -> n17` and `n15 -> n12` through g2's and g1's bottoms: n12's
  over-full right side spills them onto its bottom and the ortho stage draws plain Ls out through both blocks'
  bottoms. Built as the study proposes: in `breadth_stubs` / `breadth_ends` an end whose straight leg leaves a block
  holding it (and not the other end) through that block's lateral wall counts as blocked, so it takes the stub and
  corner leg back to the flow-facing side.
  - Variant a (the existing rule: then every end of that side takes a stub): dense-lr `lateral_x` 5 -> 1 (better
    inside failing), but **NEW must-fail dense-lr `lbl_struck` 6 (must <= 5)**, A2 86 -> 87, drift dense-lr
    `lbl_crowd` 5 -> 9; **8 copies worse**: bench-75 RL / TD `x_drawn` 190 -> 192 / 214 -> 217, LR / TD `lateral_x`
    2 -> 3 / 10 -> 11, LR `fan_x` 4 -> 6, LR / RL `side_ends` +1, LR `min_edge_gap` 15.25 -> 12.5 (bench-75's
    `gateway` spills too, and its stubs reorder the whole LR fan).
  - Variant b (only the end that leaves sideways takes a stub): dense-lr `lateral_x` 5 -> 1, **NEW dense-lr
    `lbl_struck` 8**, target-worse dense-lr `crossings` 17 -> 19; **11 copies worse** (dense-lr LR/RL `x_drawn`
    17 -> 19 and `fan_x` 0 -> 2, the bench-75 rows above). The stubs' runs along the flow land in the gap where n12's
    labelled fan has its steps, and the fan's labels are pushed onto the steps (`fr1/e4a-dense-lr-zoom.png`).
  Not committed: a passing row (`lbl_struck`) and passing copies get worse, which decision 2 forbids. What it needs:
  the spilled ends' corner legs placed with the fan's steps and labels (the channel and label slot of the gap), not
  after them.
- **e5, skipped** (`B/b12a2b/fr1/e5-probe.diff`, run `q-e5a`). dense-lr's fifth line, `n52 -> n6` through g1's top:
  its chain's dummy inside g1 lies at breadth 215.1, inside g1's LR title strip (chip 202.6-238.1); the ortho run
  along it would pass the chip, and the repair takes the edge above g1 and down through g1's top. Probe in the
  layout: in a block layout an inner chain dummy keeps the box's strip room from the box's left wall (`pair_sep`).
  **No line gained** (A2 86 -> 86, dense-lr `lateral_x` unchanged: the dummy's breadth is not set by that
  separation), bench-75 `compact_max` 71.64 -> 72.16 worse inside failing, routed bench-75 `xframe_runs` 1 -> 3
  worse, **57 copies worse** (23 `min_edge_gap` drops: the public family LR / RL 21.34 -> 14.64, bench-75 RL 12.5
  -> 1.73; routed bench-75 LR / RL `defects` 15 -> 24 / 8 -> 17). The strip band is drawn by the frame function
  from the members, not from the walls the layout separates, so the rule belongs where block mode places a chain
  inside a block (B12.8's coordinates, now `b12a2b-squeeze`), not in the wall separation. Not committed.

### Item 1: regress (`360c588`)

`regress/proto.diff` as the study gave it: `split_by_room` sorts and chains by `Jog::plain.0`. Fixture
`tests/fixtures/blocks/twoway-noted.mmd`, test `blockcheck::a_two_way_pair_between_blocks_keeps_its_steps_in_one_channel`
(fails at 7122a53). Identity row: the default build's blocks-off hashes `97f3b3acf179bb13 2ebf8a501a52edbd
9af2a3a1de5c6a97`; the static hash equals `B/start/flowmaid`'s SVG of the fixture (FNV-1a checked by hand), and
the off-identity test is green. `BLOCK_CEILINGS` row all 0 (default `quality` example: defects, edge_frame_pairs,
overlap_pairs, x_drawn, hard zeros 0, static and routed). Checks: no cell, no gate message, no copy changed.
Bulk (`B/b12a2b/bulk/`, suite-on `quality`, base = 7122a53's binary): `b8r1/bulkst` static `x_drawn` 308 -> 307,
`chip_crowd` 778 -> 777, `fan_x` 97 -> 96 (8ab502b3 1 -> 0, 9a94998d `ortho_edges` 7 -> 5, 3d210104
`chip_crowd` 1 -> 0; 3 files changed), routed unchanged; `bulkfc` and the 413 valid `b3r4/all` inputs unchanged.

### Item 2: `TitleSides::of` terminates (`57d0736`)

`ortho::column_clear_of`: one pass over the side's crossings sorted ascending (the column only moves forward). Unit
test `ortho::tests::a_displaced_column_is_found_in_one_pass_and_always_ends`: same column as the old search on 2000
deterministic draws, and an end at 1e16 where the old search loops. No output change (95 local SVGs byte-identical,
no copy, gate message or cell changed).

### Item 3: lateral X1 + e2 (`4d374f8`)

- **X1**: `ports::unshaded` cuts a block layout's **node** side at `ortho::CHIP_CLEAR` (layout `title_cap` and ortho's
  title sides already did). Frame sides keep `CHIP_SEP`: cut there too, crowded pins on K0's top in
  `notes/spread-bottom` BT moved 0.2-0.5 px and the router sent `K0 -> K2N` another way (copy `x_drawn` 1 -> 3,
  `fan_x` 0 -> 2), so that half was left out (measured, `q-x1` / `q-e2b`).
- **e2**: `ports::title_side_columns` (after `free_pins`, `order_pins`; TD/BT): a frame's free pins on its top and
  the entries from outside the frame into its title-row members partly under the chip are one set; where a pin and
  such an entry stand closer than `PORT_SPACING`, that run (taking in the next set end when it would run into it)
  is re-dealt from its start (not before the chip's clearance) in the order the channels come in, `PORT_SPACING`
  apart or `TRACK_PITCH` where that does not fit, an entry on its member's port range, up to a spacing short of the
  side's next foreign crossing. Where neither fits, `place_ports` keeps today's `CHIP_SEP` cut on that member's side
  (`title_side_over_full`): the public fixture's Observability title row (three scrape entries into METRICS plus
  OPS's dashboards pin past a 37 px stretch) keeps its lateral `[WEB] -> METRICS` entry, as at 7122a53.
- **"pins counted in F2" is only half built**: the ports stage counts the frame's pins against the member's stretch
  (the fallback above); the layout's and `route()`'s jog-room count (`title_entries`, `route_title_jogs`) do not.
  Counted there without X3's whole-member displacement, the OBS frame would only gain an unused jog room (METRICS is
  partly shaded, so ortho displaces none of its entries); it belongs with X3 (item 4b), which is skipped.
- **lr BT S-jog**: gone; `cart -> pager` comes down straight into pager (`pic/e2/lrbt.pair.png`), its label moved
  out of Storefront into the gap; harness rows of lr BT unchanged (x_drawn 1).
- Cells: A2 89 -> 88 (hub5 `lateral_x` 3 -> 0 gone); better inside failing: public fixture and its four
  permutations `lateral_x` 3 -> 1 (QUEUE>CHARGE by X1, CART>QUEUE by e2; [WEB]>METRICS stays) and `lbl_crowd` 2 -> 1;
  drift inside passing: the public family's `edge_len` 15483 -> 15621 (x5). Routed cells unchanged.
- Copies: static `lateral_x` 216 -> 192 (14 copies better: hub5 TD 3->0, public family TD 3->1 x8,
  swimlane-nested TD 7->6, ...), `side_ends` 231 -> 208; routed unchanged.
- **Declared trade**: `blocks.swimlane-nested-TD` `min_edge_gap` 40 -> 25. `place order -> confirm order` now comes
  in through Shop's title side beside `confirm order -> pay invoice` leaving the same side (25 px apart, the BT/LR/RL
  copies' own value) instead of through Shop's lateral wall (`lateral_x` 7 -> 6, `side_ends` 1 -> 0; picture
  `pic/x1/blocks.swimlane-nested-TD.pair.png`). 25 px is above every gap floor; no judged cell or gate moves.
  Decision 5 names trades only for failing rows: **the lead decides whether this one stands**.
- Bulk against item 1: `bulkst` unchanged but one idiom count; `bulkfc` static `lateral_x` 183 -> 172, `side_ends`
  355 -> 340, `x_drawn` 86300 -> 86297, `edge_chip` 47 -> 46, `chip_crowd` 657 -> 661 (four files +1); routed and
  `b3r4/all` unchanged. (A first version of e2 re-dealt every pin and member end of the side, inner edges included,
  and took `bulkst` `x_drawn` 307 -> 311, `fan_x` +2: `pic/bulk3/`; the committed rule only re-deals a colliding run
  of border entries and pins, and the commit was amended to it before any later item.)
- Tests: `a_title_rows_first_free_entry_clears_the_chip_ring` (hub5), `a_frames_pins_and_its_title_rows_entries_take_columns_apart`
  (public fixture and four permutations TD); both fail at 7122a53.

### Item 4a: lateral X2, the hug tie (`b53e3de`)

Probe (`FMDBGH`, scratch build): v2's `LANE>AZ` column stands 11.897 px from PROD's wall (F3 put the wall at
`LANE_PAD`; the simplified path's corner put the leg's middle 0.103 px inside), `hug 146 > HUG_TRADE`.
- The study's form (F3 pad `LANE_PAD + 0.5`, `x2-scene-pad.diff`, `q-x2`): v2 1 -> 0, but **13 already-failing
  `compact_max` cells worse by 0.01** (analog, perm-a..e, public x3, bench-75 71.64 -> 71.71, dense-lr 43.03 ->
  43.04) that nothing in this package pays back, and `quality.dense-td-RL` `x_drawn` 29 -> 30. Not taken.
- The router form in all four directions (`q-x2r`, `q-x2f`): no compact change, but the same dense-td RL crossing and
  a 290 px frame hug (harness `hug_len`) on that copy. Not taken.
- **Committed**: in a **TD/BT** block layout `router::side_run` counts a run beside a **frame's** border only within
  `CLEAR_GAP - HUG_AIR` (0.5); node sides and every other layout keep `CLEAR_GAP` (`World::hug_gap`). A2 88 -> 87
  (v2 `lateral_x` gone); no cell, gate message or copy worse; drift inside passing: v2 `bends` 72 -> 76 (must 114);
  informational: fork-twins TD `lbl_end_mean` 183.1 -> 192.4. Unit test `router::tests::a_leg_a_clearance_inside_a_frame_wall_rides_it_only_outside_a_block_layout`.
  ~~`route()` takes the same rule (the repair runs there with `cx.blocks`).~~ **Corrected in the fix round: false.**
  `cx.blocks` is set only in the static scene; `route()` and `route_partial()` keep `CLEAR_GAP` (see "Scope" above).

### Item 4b: lateral X3 (skipped, `x3-attempt.diff`, `q-x3`)

Built: `layout::title_cap` on the port span (side less `PORT_INSET`), all-or-nothing displacement
(`layout::title_displaced`, used by `title_entries` and `route_title_jogs`), `ports::unshaded` puts an over-full
title-row side's ends strictly under the chip, ortho's `TitleSides` displaces any title-row end under the chip.
Measured on top of item 4a: A2 87 -> **86** (cycle2 `lateral_x` 2 -> 0 gone; dense-td 6 -> 2, public-bt 3 -> 2
better), A1 0; but **routed cycle2 `lateral_x` 3 -> 4 (cell worse)** and **16 copies worse**: static
`sub-nested-self` TD/BT defects 2 -> 4 and `min_edge_gap` 7 -> 0.88 (the own-box stubs OUT<->IN on top of
`w2 -> store` once Workers takes jog room), `dense-td-TD` defects 0 -> 1 and `x_drawn` 42 -> 43, `bench-75-BT`
`lateral_x` 5 -> 8 and `side_ends` 8 -> 10, `bench-75-TD` `fan_x` 7 -> 9; routed `bench-75-BT` `x_drawn` 259 -> 262,
`cycle2-TD` `lateral_x` 3 -> 4, `edge_chip` +5 (b9-inside TD, cycle2 TD, sub-nested-self TD/BT, bench-75 BT). The lead's
condition for X3 ("only once its own block_fixtures rows and bench-75 TD/BT copy changes are gone") is not met; the
gate tests were not run on it. Not committed. What it needs: the own-box stubs' room in a jog-roomed frame (B12.14),
bench-75's BT displaced columns failing `TitleSides`' "no crossing between port and column" (ends left under the
chip get rerouted), and the routed displacement.

### Item 5: G3 static (skipped, `g3-proto-on-head.diff`, `q-g3p`)

The labels prototype applied cleanly to the head; measured with its p5 constants (`FM_G3_OFF_FRAC=0.155`,
`FM_G3_CHAIN_OFF=0.34`) and **without the dummy-row track exclusion** (the critic's `NOROWS`), TD only, against item
4a: A2 87 -> **66** (gone: `lbl_crowd` 9, `lbl_in_end` 6, `lbl_struck` 4, `lbl_gap_min`, `lbl_own_gap_max`,
`detour_max`, `crossings` 1 each), A1 0, routed 37 -> 36; but **17 static cells worse**: NEW analog `lbl_off_max`
0.36 (must 0.35); `lateral_x` 1 -> 2 on the analog, perm-a/c/d and the public family x5 (`DNS -> TR1`'s lower band at
TR1's port under PROD's chip; `REG ==> [ORD]`); `area` analog 4.51 -> 5.58 M, v2 5.09 -> 5.57 M; public family
`compact_mean` 7.26 -> 7.30; **13 routed cells worse** (`lbl_struck` 2 -> 4/5 on the analog family, `lbl_crowd`
6 -> 8 on perm-a/b, `crossings` 26 -> 28 on the analog and perm-a/c/d, 30 -> 32 on perm-e); **23 copies worse**
(public family TD static `lateral_x` 1 -> 2 and `side_ends` 3 -> 4 x8, `fork-twins-TD` `x_drawn` 8 -> 10, routed
`dense-lr-TD` `x_drawn` 24 -> 29, `lateral_x` 7 -> 10, `fan_x` 0 -> 2, `edge_chip` 3 -> 5, `spread-face-TD`
`side_ends` 0 -> 1). Besides fixing those, a committable G3 needs the prototype's thread-local side channels
(`G3_AT`, `G3_SLID`, `G3_CHAIN`) replaced by data carried with the placement, its env knobs made constants, BT,
and the slot rule in the router / nudge / comb / label stage (item 5's own text), none of which the prototype has.
Not committed; the A2 gain is real but the regressions above stand.

### Item 6: X2.5 breadth bound and the displaced gap-step merge (skipped)

- X2.5: the lead's text asks it measured with G3 present; the lateral study's X4 prototype alone trades the
  analog's `lbl_endpx_mean` 383 -> 389 (must 386: a NEW must-fail line) and broke two A1-held tests in its
  shift-after-`TITLE_FIT_PASSES` form. With G3 not landed, building it would add that must-fail line. Not built.
- Gap-step merge (6 turns -> 4): tried as "a chain's last run aims at its displaced entry's column, not its port"
  (`straighten` target and unpinned last column, `q-gm`): **no cell, copy or SVG changed**, because the edges behind
  the 7 `bends_max` lines (`DNS>PSDEV` on the analog family, `DNS>PSPROD` on v2) are labelled, and a labelled chain
  keeps the column its label's dummy reserved (`ortho_pass`: no `straighten`). Moving that column is G3's label
  slot. Reverted; nothing committed.

### Item 7 (first run): e3, e4/e5 not started

Superseded by the fix round's "Item 7" above (e3 committed as `3e38be3`, e4 and e5 built, measured and skipped).

### Suite-on, guard, perf, disk

- Suite-on failing tests at every commit: the 14 names of `base-7122a53/gates/failing.txt` (panics.py: no message
  new or worse; at the head two informational stdout cells: `defect_baseline_gate` static `lbl_end_mean` 222.135 ->
  222.181, `quality_gate` dense-td static `lbl_end_mean` 739.32 -> 739.59, both under their ceilings).
- Default guard `GUARD: ALL GREEN` before each commit (and before each amend).
- `perf.sh` (default build; this package changes no default code path): runs `b12a2b-base-1` (after item 3),
  `b12a2b-base` and `b12a2b-base-r2` (head). Every file within its budget in runs 1 and 3; run 2 had dense-td
  `route_ms` 10.06 (limit 9.20) and `scene_ms` 38.95 where runs 1 and 3 have 6.86 / 6.83 and 27.35 / 26.05: the runs
  disagree, no regression claimed. Worst drags: stress 1.43-2.10 (limit 2.15), lr 1.35-1.41 (1.95), advanced
  1.67-1.82 (2.61), bench-75 18.43-20.34 (25.28).
- Disk: 8.6 GB free at the start, 8.0 GB at the end; scratch target `B/b12a2b/target-on` deleted after this report.
- **Fix round:** guard `GUARD: ALL GREEN` before `3e38be3` and `ebe3d88` (`B/b12a2b/fr1/guard-{e3,docs}.txt`);
  suite-on: the same 14 failing names at `3e38be3` and at the head, no gate message new or worse (6 new tests pass).
  `perf.sh fr1-e3` (default build; the fix round changes no default code path: `split_by_room` returns at once
  outside a block layout): every file within budget; worst drags advanced 2.19 (limit 2.61), lr 1.41 (1.95),
  stress 1.40 (2.15), back-edge 1.03 (2.00), bench-75 19.51 (25.28); route_ms bench-75 10.25 (15.76), dense-td 6.79
  (9.20); scene_ms bench-75 40.56 (61.94), the sub-ms files within max(2x, +0.3 ms). Disk 7.9 GB free at the start
  of the fix round; the debug worktree `B/b12a2b/fr1/wt-dbg`, its target and `B/b12a2b/target-on` are deleted at
  its end.

---

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
  (2103.8, 928.6); the final chip lies inside STG, 6.6 px from ARGO>GITLAB. Most likely the label stage moved the
  struck chip along its path to the nearest free place (traced to the crossing at the slot, not stage by stage).
  Cause: the gap stretch is not kept free for the label (G3 `CLEAR_GAP`, not built).
- **Neighbouring lines of one fan (R2's three stand-ins; DNS's fan into DR; CART's two exits; LB / REG into ORD).**
  R2>[PROD]'s chip on its 192 px line lies 10.2 px from R2>[DR]'s line leaving the free pin 28 px beside it; the fan
  branches of DNS into DR (DNS>PSDEV / DNS>SITE, DRAPP>TG / DNS>PSPROD, DRAPP>PG / DNS>TR1) and the public fixture's
  CART>QUEUE / CART>CACHE and LB>QUEUE / REG>[ORD] are the same: a chip on a line whose neighbour runs at the fan's
  pitch (R2's pins 28 px apart, measured; the others read off the offenders' distances, 3.0-10.3 px). Cause: the
  gap's label room for a fan (G3) is not built; the even band of free pins (B12.11)
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
v2: LANE>AZ (through PROD's lateral wall); public
fixture and its permutations: QUEUE>CHARGE (into CHARGE's left side at its frame's wall), CART>QUEUE (QUEUE's left side),
[WEB]>METRICS (the stand-in runs down left of the blocks and enters METRICS's left side); public-bt: CART>QUEUE,
CART>CACHE, SHIP>DB; infra-bt: PG>DRAPP; hub5: HUB>b1, HUB>d1, HUB>e1; cycle2: b1>a1; dense-lr: n12>n17, n15>n12,
n52>n6 (5); dense-td: n8>n42, n4>n43, n25>n42 (6). Traced this round: the analog's DNS>PSPROD (the chain's dummies
stand left of STG down to PSPROD's row, x 961.6 against STG's wall at 993.2, so the end is on PSPROD's left side) and
the public fixture's three (QUEUE>CHARGE steps at y 1006 to x 734.5 and enters CHARGE's left side; CART>QUEUE enters
QUEUE's left side at y 898; [WEB]>METRICS runs down at x 88.8 left of the blocks and enters METRICS's left side). The
others are listed by edge from the yardstick's detail, not traced. Cause class: an end that 2.8 wants on its
flow-facing side is taken on a lateral side, because the chain's last dummies stand beside the block (X2.5 / X3, B12.8
under D6) or the title side has no free column past the chip (X2.2 for title-bound frames, D6 of round 2).

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

## Fix round 3 of 2026-11-17 (on top of `9f51f77`; read this first)

Commits: `507b0d6` (#2), `4812274` (#4), `7892fb7` (#5). Work files `B/work/a2h/` (scripts `bon.sh`, `ton.sh`,
`hcmp.sh`, `qcol.sh`, `svgdiff.sh`, `rpair.sh`, `tcmp.sh`, `on.sh`; suite-on outputs `ton-*.txt` / `.fail`;
pictures `pic/x22/`, `pic/even/`; experiment patches `bold-drawn-clearance-experiment.diff`,
`even-band-experiment.diff`, `bold-plus-untangle-experiment.diff`; perf logs `perf1..4.log`; local runs
`B/local/out/{h-x22,h-fr2,h-sq}`).

**Baseline correction.** The previous round's suite-on binaries `a2g/fm-on-9f51f77` / `quality-on-9f51f77` do NOT
draw what commit `9f51f77` draws: 167 of the 412 direction copies differ from a fresh suite-on build of `9f51f77`
(`a2h/fm-on-head`, `a2h/quality-on-head`). Every "before" in this section is the fresh build.

**Result: A2's gate is still not met (local judge `--due A2` 91 must-fail lines, `--due A1` 0). Items #2, #4 and #5
are fixed; #3 and #1 (the gate) stop under D6 again, this time after B12.11's pieces were built and measured (below).**
Nothing re-baselined or loosened; no README / CHANGELOG / version change; default build byte-identical in every
commit (off-identity test green in each guard).

| verifier item | result |
|---|---|
| 1 A2 gate (91 lines), B12.11 never attempted | **B12.11 attempted** (even band of free pins, node-side end order, bold chips for clearance, X2.2 column room): the column room is committed (`507b0d6`); the others do not hold A1's tests or make drawings worse, numbers below. The squeeze re-run on the new head: local A2 91 -> **72** but 4 A1-held tests fail (D6). Gate not met. |
| 2 t9 / t9b BT lateral_x 0 -> 4 (unreported) | **fixed** (`507b0d6`): 4 -> 0 (t9, t9b, every direction); t9c two-way TD/BT 4 -> 2 |
| 3 bold titles struck where `edge_chip` sees nothing | **stopped (D6)**: the rule that fixes all three inputs (chips bold for drawing and clearance in block mode) breaks 5 A1-held tests; numbers below |
| 4 frames along the flow differ on 20 of 258 copies; test skips BT | **fixed** (`4812274`): 0 of the copies differ (every block-mode fixture of `tests/fixtures/{blocks,quality,notes}` in four directions, new test); the skip is gone |
| 5 G3 routed: `browse` inside its end block in `route()` | **fixed** for `route()` / `route_sized()` (`7892fb7`); `route_partial()`'s redrawn labels are R8 (B12.13) |

### #2: t9 / t9b drawn BT (`507b0d6`)

Cause (traced stage by stage): `a`'s exit lies wholly under `Alpha one`'s chip (displaced, 2.8 rule 2), but the
frame (82 px, broader than its title) had no column past the chip inside it, so `ortho` left the link straight
through the chip and `router::repair` took it out through the right walls. Two parts were missing:
- X2.2's breadth for a frame its members make broader than its title: such a frame with displaced entries is now as
  broad as `title_column_width` (chip, `CHIP_CLEAR`, `CORNER_R`, a `TRACK_PITCH` per further entry, `LANE_PAD`),
  grown on the chip's far side, in the frame function (scene, route, drag) and reserved by the layout;
- G1/G2: the displaced end's step back across the gap was not counted (the member's column equals the far end's), so
  the two frames stood 13 px apart, less than two border clearances; `layout::title_entries` now names each
  displaced entry and its column, and `track_need` counts its step: 13 -> 37.5 px.
Measured (static, block mode, against the fresh `9f51f77` build): t9 BT/TD, t9b BT/TD `lateral_x` 0; t9c TD/BT 4 -> 2
(its turned twin still goes round: its step and its twin's share one gap and the router reroutes one); interleave TD
1 -> 0; `compact_max` up where a frame now holds its column (t9 8.50 -> 9.69, t9c 9.17 -> 11.49, interleave
4.43 -> 4.98). Routed: interleave `edge_chip` 0 -> 1 (R). 12 direction copies change. Picture `a2h/pic/x22/
t9-frames-8px-BT.pair.png`.
Not built: the column room for a frame F1 grew to its title. It adds 26.5 px to such a frame; the hub-fed row of
`layout::a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` (A1-held) then measures 1275.8 x 631 against
width < 2 x height (the row's frame `g0` is title-bound). Grown on either side or symmetrically the result is the
same 1275.8: the layout's wall reservation (`TITLE_FIT_PASSES`) pushes the members by 1.5 x the excess, and an exact
reservation would still give 1262.5 > 1262. So title-entry, title-hidden, nested-wide-title and x6 keep their
lateral entries (D6, as before).

### #4: one jog count in scene() and route() (`4812274`)

The layout read the chip at the frame the channels inside it widen and counted runs of the block under that chip;
`route()` has neither. Now both count F2 as 2.7 states it: chip from the F1 box (bold where F1 grew the frame to its
title), entries into title-row members only (a run needs no jog room, 2.5 X2.5; X2.5 still moves runs past the chip
as drawn). `frdiff2` over the 412 direction copies: 23 copies > 0.5 px before (bench-75 BT 75, dense-td TD 37.5, the
rest 12.5), **0** after. Static changes: 40 copies (frames lose jog room nothing used); dense-td `x_drawn` 49 -> 42,
`side_ends` 2 -> 1, `chip_crowd` 8 -> 9. Routed (R): dense-td `min_edge_gap` 11.97 -> 0.60 (twins `n16 -> n31` /
`n31 -> n16` overlap 14 px), defects 17 -> 18, in the `quality_gate` routed row that already failed (its message
lists the same five rows before and after).

### #5: G3 in route() (`7892fb7`)

`route_built` applies `labels_off_end_blocks` in block mode, as `scene_clustered` does. blocks/lr LR / RL routed:
`browse`, `cache miss`, `page` off their end blocks (the test checks `route()` now; it fails without the fix).

### #3 and B12.11: what was built and why it stops (D6)

Ink census (Chromium, the verifier's `inkstrike.js`) over the 404 rendered direction copies at `7892fb7`: strikes
(stroke on the bold ink, dist < 0) on bench-75 BT `data zone`, dense-td TD `lorem labore` and `consectetur nostrud
lorem`, dense-td BT `veniam sed`, dense-td RL `consectetur nostrud lorem`; the same five at the fresh `9f51f77`
build (not added by this round). The verifier's three inputs: v2-short-labels-lr-BT -1, v2-lr-labels-end-blocks-TD
-0.9, x3-nested-lr-multiline-TD -1, unchanged by this round's commits.

Experiments, each against this round's suite-on failing list (14 names; `ton-base.fail`):

| experiment | effect | suite-on tests added |
|---|---|---|
| every block-layout chip bold, layout count included | (previous round's) | 8: hub fans x2, both comb-settle, t9 test, frames-agree, router straight step, bold-title test |
| chips bold for drawing and clearance only, the F2 count unchanged (`bold-drawn-clearance-experiment.diff`) | the three inputs -1 / -0.9 / -1 -> **9.0 / 8.9 / 22.5** px; frames still agree | 6: `a_hubs_fan_keeps_its_order_through_the_bands`, `an_end_of_a_title_row_member_...` (dense-td `fan_x n16 Bottom n16->n35 x n16->n32`), `comb::a_block_layouts_comb_is_settled_after_one_pass`, `comb::combing_is_settled_after_one_pass`, `router::a_step_between_two_ports_straightens_onto_a_shared_line` (its BT C -> D steps off the bold chip), and `a_frame_holds_its_bold_title...` (restatable) |
| the same plus a node-side end order pass after the comb (`untangle_sides`: a side's ends in the order their paths first turn, kept only when crossings fall; `bold-plus-untangle-experiment.diff`) | dense-td `n16`'s bottom unchanged (the swap does not lower the crossings: `n16 -> n35`'s reroute wiggles back across), adds `n43` twin crossings | the same 6 |
| even band of free pins, 2.8 (`even-band-experiment.diff`): two or more pins of one free interval at a pitch in [PORT_SPACING, PORT_EVEN_MAX] | no test fails once a lone pin keeps its place (spec's "lone pin at the middle" breaks `edgepass::finish_is_idempotent_with_sub_edges`: `x --> S` becomes a Z); 33 copies change, harness columns (incl. `pin_corner`, `fan_uneven`) unchanged; state-note's `tick` / `done` pins bunch and their labels crowd (picture `pic/even/state-note-TD.pair.png`) | none; not committed (worse picture, no metric gained) |
| the B12.8 squeeze (X1, X2/X3, X4; TD/BT) re-applied on `7892fb7` | local A2 **91 -> 72** must-fail (compact_max / compact_mean / sib_stagger off the public family and the perms; bends_max, detour_max, lbl_struck, lbl_own_gap_max come on) | 4: `a_hub_fan_leaves_in_target_order` (public fixture routed `fan_x` 1), `a_hubs_fan_keeps_its_order...` and `an_end_of_a_title_row_member...` (dense-td `n16` twins and fan), `casing::a_line_takes_one_role...` (routed 34); `advanced_showcase_parses_and_renders` passes |

So the three stops reproduce after B12.11's pieces: the node-side order does not settle dense-td's `n16` fan (its
crossing comes from the router's reroute of `n16 -> n35`, not from the port order), and the public fixture's routed
fan is `route()`'s corridor ordering (R3, B12.13).

### Rows (static, suite-on CLI at `4812274`; `7892fb7` changes routed output only)

Local judge `--due A2`: **435 checks, 91 must-fail** (`B/local/out/h-fr2/judge-A2.txt`); `--due A1`: **0**. By metric:
`lateral_x` 18, `compact_max` 14, `sib_stagger` 12, `lbl_crowd` 12, `lbl_in_end` 7, `compact_mean` 7, `bends_max` 7,
`lbl_struck` 5, `bends` 2, `area` 2, `straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `lbl_endpx_mean`,
`detour_max` 1 each. Against `9f51f77`: analog `compact_max` 10.27 -> 9.71, `compact_mean` 5.51 -> 5.40; v2
15.77 / 7.49; perms `compact_max` -0.56 each; perm-b `lbl_crowd` 5 -> 8, `lbl_in_end` 3 -> 1; perm-e `lbl_in_end`
2 -> 1. B12.12: dense-lr area **10.87 Mpx2** (must <= 7.98), unchanged; "narrowing still accepts a candidate" not
measured (no switch disables narrowing). `BLOCK_SLACK` not pinned (the squeeze is not on the branch).

### Suite-on failing tests, default run

Suite-on at `7892fb7` (`ton-g3r.fail`): the same 14 names as at `9f51f77`. Default run: `GUARD: ALL GREEN` before
each of the three commits; off-identity test green.

### Perf (4.6), default build

Four `perf.sh` runs (`runs/b12a2-fr3-1..4`); the box was shared (another session's benchmark and builds, load
1.1-4.4). Run 3 (load 2.0 at the start): every file inside its budget (dense-td `route_ms` 6.81 / limit 9.20, drag
13.36 / 17.80; bench-75 10.17 / 15.76, drag 19.09 / 25.28; lr drag 1.40 / 1.95; advanced drag 2.44 / 2.61). Runs 1,
2, 4 (loaded) put dense-td `route_ms` at 9.72 / 10.41 / 9.92 and lr drag at 2.13 / 1.41 / 2.11. The default
build's code paths are unchanged (every new branch reads block mode), and an interleaved head-vs-now comparison of
the default `quality` example (`tcmp.sh`, three pairs at load 3.2-5.0) shows the same spread for both builds
(dense-td `route_ms` head 6.76 / 10.33 / 9.61, now 9.60 / 7.07 / 10.27; lr drag head 2.08 / 1.36 / 1.43, now
2.12 / 2.03 / 2.18): load, not a regression. Block-mode static `scene_ms` (suite-on, two runs at load 4.0): advanced
1.18 / 1.09, subgraph 0.21 / 0.35, fanout-frames 3.52 / 3.37, dense-td 30.9 / 48.2, dense-lr 12.8 / 13.5, bench-75
31.2 / 25.0, public fixture 13.6 / 10.0.

### Disk

6.7 GB free at the start, 4.5 GB at the end (another session's files); the probe target (`a2h/ptarget`, 61 MB) and a
scratch worktree of `9f51f77` with its target were deleted; `B/target-on` (56 MB) was deleted after this report,
its suite-on CLI and quality example of `7892fb7` kept as `B/work/a2h/fm-on-7892fb7` and `quality-on-7892fb7` (the
fresh `9f51f77` pair as `fm-on-head` / `quality-on-head`).

---


## Fix round 2 of 2026-11-17 (on top of `4a196db`; read this first)

Commits: `1c6cb2f` (#4 bold widths past ASCII), `c8249f4` (#1/#2: B12.10, one frame function in `scene()`,
`route()`, `route_partial()`; F3 on a drag; the layout's F2 count made spec-conformant), `9f51f77` (#1: G3, a label
keeps off its end blocks; lr `lbl_in_end` 4 -> 1). Work files `B/work/a2g/` (probe crate `probe/` with `frdiff`,
`frdiff2`, `pdiff`, `onsvg`, `meas`, `details`, `dump`; pre / head worktree probes; 4-direction copies of every
repository fixture `dirs/` (412 files); suite-on test outputs `ton-*.txt` / `.fail`; local runs
`B/local/out/{r1,g3}`; experiment diffs `x22-column-room-chipside.diff`, `b128-squeeze-on-9f51f77.diff`; suite-on
binaries `fm-on-9f51f77`, `quality-on-9f51f77`).

**Result: the A2 gate is still not met (local judge 91 must-fail lines, was 98; A1 judge 0). D6 stops below for the
squeeze (B12.8, every direction), the chip half (#3) and X2.2's column room (#6), each with its numbers.**
Nothing was re-baselined or loosened; no README / CHANGELOG / version change; default build byte-identical
(off-identity test green in every commit's guard).

| verifier item | result |
|---|---|
| 1 lead decision not carried out | F3 built first (`c8249f4`), then the squeeze retried per direction: **stopped (D6), every direction** (below). G3 for lr: **done**, `lbl_in_end` 4 -> 1 (`9f51f77`; the start had 2) |
| 2 scene / route / route_partial frames differ | **fixed** (`c8249f4`): numbers below |
| 3 x3 TD title strike (chip half) | **stopped (D6)**: three rules tried, each breaks A1-held tests (below) |
| 4 bold widths past ASCII | **fixed** (`1c6cb2f`): x7's two titles end 16.7 / 11.5 px inside their frames (were 40.5 / 3.2 px past) |
| 5 A2 gate 98 must-fail | **91** now (lines below); not met |
| 6 title-sized frames, lateral entries | **built and stopped (D6)**: fixes the inputs, breaks one A1-held narrowing test by 14 px (below) |

### #2 / B12.10: one frame function in three entry points (`c8249f4`)

- `route_built` takes the block mode (switch read once per entry call, `route_blocks` = the layout's predicate) and
  starts from `route_frame_base`: block mode = `frame_boxes` with the title fit (F1) and the jog room F2 counted from
  the boxes (`route_title_jogs`: R2's test on the F1 boxes for "above", title row by box overlap so a 1 px drag cannot
  change it); every builder routes against those boxes (R1 (i)-(ii)).
- The layout's F2 count (`title_counts`) now counts what F2 specifies: drawn edges ending on a title-row member.
  Stand-ins (cut back to their frame), rank-only proxy pairs and invisible links reserved jog room nobody used
  (dense-td 25 px less height; the public fixture's OBS 37.5 px), and `route()` could not count them.
- `keep_inner_room` (F3 on a drag): a frame `route_partial()` redraws round a moved member grows back over the
  waypoints of its kept edges inside its base box (never past base box U redrawn box).
- Measured, block mode (`frdiff2`, scene vs route at the scene's centres, along the flow, > 0.5 px): verifier's 294
  inputs 70 at the start (`9c142b3`), 33 with F1 alone, **21 now**; any frame > 0.5 px (`frdiff`, the verifier's
  measure): 164 at the start, 177 at `4a196db`, **134 now**; titles `route()` cuts that `scene()` draws whole: **0**.
  On the 412 direction copies of the repository fixtures: 23 left, all one of two causes said in `route_title_jogs`:
  the chip of a frame the layout's channels widened (F3, `scene()` only) stands further left (cycle2 BT, reqresp BT,
  bench-75, dense-lr TD, b9-inside2 BT, turned-blocks), or a run the layout's order moved off its member's column
  (blocks-overview and permutations BT: ORD; nested-chain TD/BT; p07-nested TD; dense-td TD g8, BT g2; wide-frames).
- 1 px drag of every node (`pdiff`, largest frame move): nested-wide-title TD 258.1 -> **1.0** (start 12.5),
  title-hidden TD 75.3 -> **0.5** (12.5), title-entry LR 60.9 -> **0.5** (1.0), title-entry TD 25 -> 0.5, x5 -> 1.0.
  Sum over the verifier's 294: start 15502, head 18141, **now 9560**; inputs above 1.5 px 166 -> 134. Above the
  start only x3-nested-lr-multiline BT 28.1 -> 80.2 and TD 25.0 -> 53.2, x4-channel-broadens 25 -> 27.5 (the
  redrawn edges' builder paths no longer take the channel the static frame grew over: R3's inner legs, B12.13).
- Tests: `frames_agree_between_scene_and_route_along_the_flow`, `a_one_px_drag_keeps_a_frame_grown_to_its_title`.
- Static changes from the F2 count (block mode): blocks-overview family TD/BT, dense-td, fanout-frames,
  nested-cases, note-edge-row, p04, p12-state, p13, spread-face, state-note, t9c two-way. Harness (quality corpus,
  static): every gate holds; dense-td `pin_corner` 0 -> 1 (`n46 -> g9`'s cut pin now `PORT_SPACING` from g9's corner:
  free pins, B12.11); dense-td routed defects 21 -> 17, `fr_max` 87.5 -> 49.9.

### #1 G3: labels off their end blocks (`9f51f77`)

blocks/lr LR `lbl_in_end` 4 -> **1** (the start had 2): `browse`, `miss`, `page` (cart -> PAGER) now in the gaps; left
is `db -> ship`'s `order`, whose gap between Storefront and Orders (32 px) is narrower than its chip (46.8), as at
the start. Local: `lbl_in_end` analog 3 -> 1, v2 3 -> 1, perms 3 -> 1 / 3 -> 2, public fixture family 1 -> 0. Cost:
`lbl_crowd` +1 on the analog family (3 -> 4, v2 6 -> 7), dense-td `chip_crowd` 5 -> 8, v2 `lbl_endpx_mean` 382 -> 392
(labels moved into gaps already holding lines: G3's `CLEAR_GAP` reservation is not built). Test
`a_label_keeps_off_the_blocks_that_hold_one_end_of_its_edge`.

### #1 / B12.8: the squeeze, retried with F3 in place, one direction at a time (D6 stop)

Patch `a2g/b128-squeeze-on-9f51f77.diff` (the earlier X1 + X2/X3 + X4 build, gated per direction for the
experiment). Suite-on failing tests added against this round's list, per squeezed direction:

| direction | tests added to the failing list |
|---|---|
| TD | `a_hub_fan_leaves_in_target_order` (public fixture routed `fan_x` 1), `a_hubs_fan_keeps_its_order_through_the_bands` and `an_end_of_a_title_row_member_...` (dense-td `n16`: 4 fan crossings), `casing::a_line_takes_one_role_...` (routed 34 > 3), `frames_agree_between_scene_and_route_along_the_flow` (ORD 12.5 px: a run moved under the chip) |
| BT | the TD list less `frames_agree`, plus `comb::a_block_layouts_comb_is_settled_after_one_pass`, `comb::combing_is_settled_after_one_pass` (dense-td 2), `edges_keep_off_frame_borders` |
| LR | the BT list plus `a_choices_spilled_and_cornered_ends_keep_off_each_other`, `a_generated_sweep_keeps_the_block_invariants`, `a_turned_step_keeps_off_the_runs_of_a_fan_it_crosses`, `scene::sibling_frames_never_intersect` (frames P and D) |
| RL | the BT list plus `a_generated_sweep_keeps_the_block_invariants`, `scene::sibling_frames_never_intersect` |

(X1's alignment rule and X4's first-row shifts act in every direction, which is why a BT-only squeeze still moves
the TD fixtures.) No direction holds the A1 tests: B12.8 is not on the branch; `BLOCK_SLACK` is not pinned.
`compact_max` / `compact_mean` / `sib_stagger` / `bends_max` stay on the must-fail list (section "Rows" below).

### #3 chip half (D6 stop)

x3's OUTER is not grown to its title (its members, the grown INNER frame, are 267.6 px broad, its title fit 258), so
it keeps the regular chip; its title's bold ink runs 17 px past that estimate and `in -> p1` runs through "here"
(Chromium, unchanged since `4a196db`). Three rules tried, suite-on, each against this round's list:
- every block-layout title bold: + 5 tests (dense-td hub fans, both comb-settle tests, `a_hubs_fan_is_a_comb...`);
- bold for frames F1 grew, also when channels broaden them: + `an_end_of_a_title_row_member_...` (dense-td `n42` Top
  `n28 -> n42` x `n25 -> n42`), + `edges_keep_off_frame_borders`;
- bold where the bold ink passes the estimated chip by more than `CHIP_PAD_W / 2 + CHIP_SEP` = 10 px (x3: 17.0;
  dense-td's titles 10.9-13.5): the same two; with the earlier `ports-widen.diff` the fan test passes but dense-td
  static crossings 49 -> 61 and routed `border_len` 1426 -> 2178 (gate 1895).
None committed. The strike on x3 TD stays (a regression against the package start, where the title was cut).

### #6 lateral entries into title-sized frames (D6 stop)

Built (`a2g/x22-column-room-chipside.diff`, and a symmetric variant): a frame F1 grows to its title also holds its
displaced entries' columns past the chip (`title_column_width` = chip + `CHIP_CLEAR` + `CORNER_R` + (k-1) x
`TRACK_PITCH` + `LANE_PAD`), reserved in the layout's F1 passes. Static `lateral_x`: x6 TD 4 -> 2, BT 4 -> 1;
nested-wide-title TD/BT 1 -> 0; title-entry TD 2 -> 0, BT 1 -> 0; title-hidden TD 1 -> 0. It breaks the A1-held
suite-on test `layout::a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` (1275.8 x 631 against width < 2 x height:
the column room of `eiusmod ut labore elit magna` adds 26.5 px to the hub's row; growing on the chip's side instead
gives the same width). Not committed.

### Rows of 4.3 at `9f51f77` (static, suite-on CLI; `B/local/out/g3`)

Local judge `--due A2`: **435 checks on 92 diagrams, 91 must-fail** (98 at `4a196db` and at the start), `--due A1`:
**0**. By metric: `lateral_x` 18, `compact_max` 14, `sib_stagger` 12, `lbl_crowd` 12, `lbl_in_end` 7, `compact_mean` 7,
`bends_max` 7, `lbl_struck` 5, `bends` 2, `area` 2, `straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `lbl_endpx_mean`,
`detour_max` 1 each. By input: public fixture family 25, analog 14, v2 10, perms 6-7, dense-lr 3, infra-bt, hub5,
dense-td, cycle2, blocks-overview-bt, bench-75 1. Changes against `4a196db`: public fixture `compact_max` 25.16 ->
19.97, `compact_mean` 8.00 -> 7.26 and `lbl_in_end` 1 -> 0 (all five copies); analog `compact_max` 10.37 -> 10.27,
`bends` 144 -> 142, `area` 4.60 -> 4.51 M, `straight_max` 1601 -> 1591, `lbl_in_end` 3 -> 1, `lbl_endpx_mean` off the
list, `lbl_crowd` 4 -> 4; v2 `bends` 153 -> 149, `lbl_in_end` 3 -> 1, `lbl_crowd` 6 -> 7, `lbl_endpx_mean` 386 -> 392;
perm-b `lbl_struck` off the list, `bends_max` 13 -> 12.

B12.12's rows: dense-lr static area 3232.9 x 3362.9 = **10.87 Mpx2 (must <= 7.98)**; the "narrowing still accepts a
candidate" row was not measured (no switch disables narrowing; not built); the two narrowing tests pass as written.

### Suite-on failing tests, default run

Suite-on at `9f51f77` (`ton-g3.fail`): A1's 15 names less `edges_keep_off_frame_borders` (passes since `c8249f4`).
Default run: `GUARD: ALL GREEN` before each of the three commits; off-identity test green.

### Perf (4.6), default build

Two `perf.sh` runs (`runs/b12a2-fr2-1`, `-2`). Run 1 was taken while another session's `vprobe` loaded the box (load
1.9-3.3): advanced `scene_ms` 1.56 and `drag_ms` 3.80 over their limits (1.42, 2.61). Run 2 at load 0.86: every file
inside its budget (advanced 0.81 / 0.62 / 1.55; bench-75 32.48 / 8.54 / 14.39; dense-td 22.12 / 5.91 / 9.95; dense-lr
17.08 / 3.75 / 9.03; lr drag 1.35). The default build's code paths are unchanged by this round (every new branch reads
block mode), so run 1's numbers are load. `fixture_drag_perf_guard`, `route_partial` worst frame, default build:
dense-lr 5.09, dense-td 5.87, bench-75 6.79 ms (the start: 5.40-7.96 on bench-75 over its two runs); suite-on: dense-lr
4.70, dense-td 7.49, bench-75 7.49, public fixture 3.67 ms (block mode's route_partial is R's).

### Disk

8.8 GB free at the round's start, 6.0 GB before cleanup; the suite-on target `B/target-on` and the round's probe
targets and worktrees (`a2g/ptarget*`, `a2g/pre-tree`, `a2g/head-tree`) are deleted after this report.

---

## Fix round of 2026-11-17 (on top of `acacab7`; read this first)

Commits: `64da873` (fix #1: bold title measure), `4a196db` (docs: #3, #4, #5 recorded in the code and history).
Work files `B/work/a2f/` (scripts `bon.sh`, `ton.sh`, `measure.js`, `titlefit-all.js`, `zoom.js`; suite-on outputs
`ton-fix1b.txt` / `.fail`; pictures `pic/fix1-*.png`; local run `B/local/out/f1`; experiment patches
`bold-everywhere-experiment*.diff`, `ports-widen.diff`, `f1-only.diff`; suite-on binaries `fm-on-64da873`,
`quality-on-64da873`).

| verifier item | result |
|---|---|
| 1 bold titles overflow grown frames (BLOCKING) | **fixed for every frame F1 grows to its title**; the chip half is **stopped (D6)** for frames broader than their titles, numbers below |
| 2 A2 gate: 98 must-fail lines | **unchanged: 98** (same lines, `B/local/out/f1/judge-A2.txt` vs `g1`); the D6 stop stands, the lead / user decides |
| 3 grown frames take labels in, unreported | **recorded** (`4a196db` message and the frame function's doc); not fixed: it is G3 (B12.9), under the stop |
| 4 wrong claim about t9b-frames-8px-lr | **corrected** here (section 2) and in `4a196db`'s message (acacab7 cannot be amended) |
| 5 title-sized frames push entries into lateral walls | **recorded** (`4a196db`); not fixed: it is X2.2's breadth (B12.8 / B12.11), under the stop; measured below |
| 6 history: unverified wip `395147c` | nothing to do on the branch (reverted by `b3b2f17`; the lead squashes at merge) |

### Item 1: what was built (`64da873`)

- `layout::bold_text_width`: a Helvetica-Bold advance table (AFM per 1000 em, x 14 px, rounded up to 0.1). Against
  Chromium (Liberation Sans Bold, metric-compatible) on the 118 subgraph titles of the repository fixtures and the
  verifier's stress inputs: never short by more than 1.12 px (glyph ink past the advance, e.g. `j`), long titles
  0.5-1.5 px over; the regular estimate was short by up to 24.3 px (`B/work/a2f/measured.tsv`, `cmpw.py`).
- F1 (`scene::title_fit_width`, used by the scene's frame function and the layout's reservation of it) measures bold.
- A frame F1 grows to its title and that nothing else makes broader is *title-bound* (`SceneCluster::bold_title`,
  crate-private): its chip is measured bold wherever a chip is read (`title_head`, `fit_cluster_title`, the
  painter's chip, the label stage's and the harness's chips), and so is the layout's `title_chip` (X2.5, jog count).
- Result, Chromium + the verifier's `titlefit.js`: no title ends within 2 px of or past its border on any of the 98
  block fixtures and stress inputs drawn in block mode (before: title-hidden 11.9, title-entry 7.7 / 13 (LR, RL), lr
  5.7, s1 9.5, s2 10.6 / 8.9, s4 14.3, s5 15 px past the border); every such title now ends 10.5-11.5 px inside
  (chip pad + title inset). On the nested fixture (new `blocks/nested-wide-title`, synthetic, the s4 case) the
  entry no longer runs through the title's last word (`pic/fix1-s4-nested-wide-title.png`, `pic/nwt.png`).
- Holds: default guard ALL GREEN; off-identity green (new fixture's row recorded from the default build); suite-on
  failing list = A1's 15 names with the same failure texts; the quality corpus's harness rows unchanged (dense-td,
  dense-lr, bench-75, fanout-frames identical); local A1 judge 0 must-fail; local A2 98 (same lines).
  Local inputs changed: the LR / RL copies of the analog (`infra-rl` `lbl_struck` 1 -> 0, `lbl_gap_min` 0.0 -> 3.7,
  `lbl_detach_max` 9.0 -> 3.3, i.e. back to the values before `bb48c30`; `infra-mini-lr` `lbl_in_end` 3 -> 4;
  frames 12-14 px longer), and the repository's title fixtures (frames 10-20 px broader; `compact_max` up by
  0.3-0.8 on lr, title-entry, title-hidden, p03).

### Item 1: what is stopped (D6), with numbers

Measuring **every** block-layout chip bold (the consistent rule; patch `bold-everywhere-experiment2.diff`) breaks
A1-held musts on dense-td, whose hub fans are the fragile ones of checkpoint A1 section 3:
- `a_hubs_fan_keeps_its_order_through_the_bands` / `an_end_of_a_title_row_member_...`: fan crossings `n16 Bottom
  n16->n35 x n16->n32` (the router's reroute of `n16 -> n35` is chaotic in the chip widths: it passes with the
  regular chips of all titles whose bold/regular difference is under 9.5 px, fails from 9 px, the 9.3 px of `g9`'s
  title) and `n42 Top n25->n42 x n28->n42` twice; `comb::a_block_layouts_comb_is_settled_after_one_pass` (dense-td
  `n16 -> n32`, `n16 -> n35`).
- Two workspace fixes cleared those: a comb pass after the label stage (block layouts), and, on a block layout's
  title-row side whose free stretch is short of its ends, taking only the shortfall of the chip's shade instead of
  the whole side (`ports-widen.diff`: `n42`'s top had 32 px free of the 42 its four ends need; it jumped to the
  whole side and the router took `n25 -> n42` along `g9`'s title strip across the fan).
- What stays: dense-td static `min_edge_gap` **11.12 < GAP_FLOORS 12.1** (`n16 -> n43` beside `n28 -> n42`, 59 px
  in a crowded channel, columns 9-11 px apart after the layout moved; `spread_close` does not clear it), so
  `a_free_pin_keeps_off_a_members_exit_past_the_chip` fails. Not committable.
- So frames broader than their titles keep the regular chip, as every frame in the default build has: a stroke kept
  `CHIP_SEP` off such a chip can still touch the bold ink when the title's bold width exceeds its estimate by more
  than 10 px (e.g. repository `lr` drawn BT: `cart -> PAGER` runs into the last letter of EDGE's title, before and
  after this round). The fix needs B12.11's title-clear rule 1 in full (the crossing ends on the free stretch) and
  the node-side end order, then the chips can all be bold.
- Also seen, not B12's: `blocks/p13-inner-dir-ext` (a B9 direction block, not a block layout) runs its title 114 px
  past its frame in the default build, the starting release and now alike.

### Items 3 and 5: numbers at `64da873` (static, suite-on; verifier's stress inputs and repository fixtures)

| input | lateral_x acacab7 -> now | lbl_in_end acacab7 -> now |
|---|---|---|
| blocks/lr (LR) | 0 -> 0 | 4 -> 4 (2 before `bb48c30`) |
| s1-wide-titles-td TD / BT | 4 -> 4 / 5 -> 5 | 0 |
| s2-lr-long-titles LR / RL / TD / BT | 1 -> 0 / 1 -> 0 / 5 -> 5 / 4 -> 4 | 0 |
| s3-nested-linked TD / BT / LR / RL | 4 / 5 / 2 / 2 (unchanged) | 1 / 1 / 0 / 0 |
| s4-nested-wide-title TD / BT / LR / RL | 1 / 1 / 0 / 0 (unchanged) | 0 |
| blocks/nested-wide-title (new) | 1 | 0 |
| title-entry / title-hidden | 2 / 1 (unchanged) | 0 |

Cause of item 5 (TD/BT): ortho displaces an entry under the chip to a column past it (`TitleSides`), but a column
that would leave the frame is not taken; a frame exactly as broad as its title has no such column, so the router
takes the entry in through a lateral wall. X2.2 reserves `TRACK_PITCH` per displaced entry past the chip and F3
grows the frame over the column. Not built: it widens dense-td's `g7`, `g9`, `g10` and the public fixture's `EDGE`,
`OBS` (the blocks with jog room), the same layout moves that broke dense-td's fans above. Cause of the LR rows
(s2): the title-length growth puts a gap's Z step inside the grown frame (G1's `reach_of` with F2's growth).

### Perf, disk, A2

- `tools/perf.sh b12a2-fix1` (default build; the round's code paths are block-mode only): every file inside its
  budget, totals static `scene_ms` 30.97 -> 40.86 (the per-file 2x rule holds: dense-lr 23.40 -> 16.33, dense-td
  25.34 -> 21.99), routed `route_ms` 12.61 -> 9.98, worst drag 20.22 -> 16.84, lr drag 0.95 -> 1.16 (inside base +
  1.0). One run: no regression is claimed (the default build's output is byte-identical, off-identity test).
- Disk: 8.8-9.0 GB free throughout; `B/target-on` (56 MB) deleted after this report; the suite-on CLI and quality
  example of `64da873` kept as `B/work/a2f/fm-on-64da873`, `quality-on-64da873`.
- The A2 gate is not met (98 must-fail lines, the same as at `acacab7`); the stop of D6 at B12.8 stands as
  section 1 below describes. Nothing was re-baselined or loosened.

## Run of 2026-11-16 (second resume)

Workspace only, never committed. `B` = the b12 workspace directory, `R` = `/home/user/flowmaid`, branch
`ccr-ccc6d89a-5cxd7e`, `pre` = `9c142b3`. Work files: `B/work/a2c/` (scripts `bon.sh`, `ton.sh`, `pic.sh`, `hc.py`;
harness tables `h-*.tsv`; suite-on test outputs `ton-*.txt` / `.fail`; pictures `pic/`), local runs under
`B/local/out/{c1,c3,g1}`.

**Result: the gate of A2 is not met. The stop rule fired at B12.8 (decision D6): the coordinates item could not be
built without failing tests that hold at A1, so it is not on the branch.** What is on the branch is green and holds
every must of A1. Nothing was re-baselined or loosened.

## 0. Commits of this run (on top of `9c142b3`)

| commit | what | state |
|---|---|---|
| `bb48c30` | B12.7, F1 + F2 (one frame function; a frame as broad / long as its title; jog room) | from the earlier run; verified here: guard green (`B/work/a2b/guard-b127.txt`), suite-on failures = A1's list (`B/work/a2c/ton-b127.fail`) |
| `395147c` | wip of B12.8 (X4 by moving whole blocks), committed unverified at the restart | verified here: **breaks five A1-held tests** (dense-td hub fan, comb settle, casing); the default guard fails on them |
| `b3b2f17` | revert of `395147c` | tree identical to `bb48c30` (diff 0 lines) |
| `acacab7` | B12.9 G1, the linked-frame floor (`LINKED_FRAME_GAP` = `FLOW_MIN_GAP` + 1) | guard **ALL GREEN**; suite-on failures = A1's list exactly; local A1 judge 0 must-fail |

Commands: default `tools/guard.sh R`; suite-on `RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=B/target-on cargo
test --release --no-fail-fast --lib --test quality` (`B/work/a2c/ton.sh`; release profile, the lib tests take 25 s);
local static acceptance `B/work/a2b/on.sh <label> A2` (= `B/local/run.sh fm` + `judge --due A2` and `--due A1`);
harness `quality <dir>` from the suite-on build.

## 1. B12.8 (coordinates): built, measured, not committable

Built in the workspace exactly as 2.5 specifies, starting from the earlier run's experiment
(`B/work/a2b/x2-squeeze-experiment.patch`, debug knobs removed): X1 (no Brandes-Köpf alignment between chain dummies
of different cluster paths), X2/X3 (`block_compact`: per level deepest first, keep Brandes-Köpf within `BLOCK_SLACK`
= 4 px, else squeeze to `W_min` seeded by Brandes-Köpf with spine merges and `COMPACT_SWEEPS` median sweeps; a child
block rigid at its parent's level), X4 rewritten to move **first-row members only** (the wip moved whole blocks),
deepest sibling groups first, LR/RL title-length growth included. One bug found and fixed on the way: a merged spine
could lay two members of one rigid unit over each other on a shared layer (generated sweep seed 16: blocks B0 and
B5 overlapping on layer 8, `chains_keep_one_side_of_a_block` failed); `pack_units` now refuses such a merge.
The clean TD/BT variant is kept as `B/work/a2c/b128-x1x2x4-td.patch` (applies on `acacab7`), with its two tests
`a_block_is_squeezed_to_its_content` and `a_spine_inside_a_block_is_straight` in `B/work/a2c/b128-x1x2x4-attempt.diff`.

What it achieves (static, suite-on; `c1` = all directions, `c3` = squeeze in TD/BT only; head `g1` for reference):

| input | compact_max g1 -> c3 (must) | compact_mean g1 -> c3 (must) | crossings g1 -> c3 (must) |
|---|---|---|---|
| analog | 10.37 -> 7.07 (<= 5.5) | 5.65 -> 4.18 (<= 3.9) | 15 -> 17 (<= 20) |
| v2 | 16.25 -> 6.12 (<= 5.5) | 7.63 -> 3.86 (<= 3.9) | 14 -> 16 (<= 17) |
| public fixture | 25.16 -> **6.70** (<= 7.97) | 8.00 -> 4.74 (<= 4.63) | 6 -> 5 (<= 7) |
| dense-td | 12.45 -> 12.77 (ref 12.45) | | 49 -> 44 (<= 74.6) |
| dense-lr | 43.03 -> 18.43 c3 / 7.01 c1 (ref 11.78) | | 17 -> 18 / 16 |
| bench-75 | 71.64 -> 68.33 c3 / 28.36 c1 (ref 52.03) | | 204 -> 208 / 243 |

`sib_stagger` 0 on every input of the table with X4; local A1 judge 0 must-fail for both `c1` and `c3`.

Why it stops (each a test or must that holds at A1 and fails with the item; measured, pictures in `B/work/a2c/pic/`):

1. **Squeeze in LR/RL (bench-75, `c1`).** A one-row block packed to `W_min` has no freedom left (feasible interval
   of every unit = one point), so every column its members shared with their fans' far ends is gone. bench-75 static:
   `fan_x` 4 -> **17** (A1 hold: <= 6), `min_edge_gap` 15.25 -> **12.50** (`GAP_FLOORS` 14.7, not loosenable),
   defects 0 -> 5, crossings 204 -> 243; tests `a_turned_step_keeps_off_the_runs_of_a_fan_it_crosses`,
   `quality_gate` (static rows). Skipping single blocks' squeezes gives 9-26 fan crossings, skipping the root's 26;
   pulling a unit without inner links by its outer links changes nothing (all one-row units are pinned). Picture
   `pic/b75.png`.
2. **Squeeze in TD/BT only (`c3`).** Every static A1 test passes, and the failing list adds two tests, both the routed
   half: `blockcheck::a_hub_fan_leaves_in_target_order` (public fixture **routed** `fan_x` 2: `METRICS Top
   ORD->METRICS x WEB->METRICS`, `ALERT Top ALERT->PAGER x ALERT->MAIL`; starting release 0, A1 0) and
   `casing::a_line_takes_one_role_across_a_cluster_of_crossings` (routed opposite patterns 34 against `OPPOSITE_MAX`
   3). Cause: `route()` at the squeezed centres orders a side's ends by the far end's breadth while the drawn path
   leaves the other way round a frame (`digest` goes round Order processing on the right, its port is left of
   `page`'s); `WEB -> METRICS` takes its horizontal just under its frame, across `ORD -> METRICS`'s exit. Both are the
   corridor slots and routed free pins of 2.9 R3 (B12.13). The first test runs in the default guard (it forces
   blocks on), so the guard fails: not committable. Picture `pic/bo-routed.png`.
3. **X4 alone, X1 alone, or X1 + X4 without the squeeze** each break dense-td's hub fan (static `fan_x` 1-2,
   `a_hubs_fan_keeps_its_order_through_the_bands`, `combing_is_settled_after_one_pass`,
   `a_block_layouts_comb_is_settled_after_one_pass`, `an_end_of_a_title_row_member_keeps_out_...`): the fan at `n16`
   is the fragile one of A1 sections 3 (shuffles s2 / s6), whose fix is the node-side end order of B12.11.
   With the TD squeeze on, dense-td passes all of them.

The spec's own edge-shape rows also move the wrong way with the squeeze (public fixture `bends_max` 8 -> 11 on
`OPS -> [OBS]`, `detour_max` 1.22 -> 1.48 on `SHOP -> CDN`; analog `lane_gap_min` 12.4 -> 4.2), so even with the
routed half fixed B12.8's check would still fail on its own rows.

What would unblock it, in the spec's order: B12.11's node-side end order and even band (dense-td `n16`, bench-75's
fans), then B12.13's corridors and routed free pins (the two routed fan crossings), then X2 in LR/RL.

## 2. Items B12.9-B12.12

- **B12.9**: built: the second floor of G1 (`acacab7`). t9-frames-8px and its two-way variant: frames 8 -> 13 px
  apart (TD, BT), **and t9b-frames-8px-lr as well: 8 -> 13 px, canvas 284 -> 289 px** (corrected in the fix round:
  the first text said "LR/RL were 13 already", which was wrong; the verifier's `on-pre` / `on-now` SVGs show it);
  test `linked_frames_stand_further_apart_than_route_reads_as_stacked`. No other local input changes. Not built: G1's `reach_of` with F2's growth beyond what B12.7
  already reserves, G3's label slot rule and `CLEAR_GAP` reservation (the label rows `lbl_crowd`, `lbl_in_end`,
  `lbl_own_gap_max`, `lbl_struck` below). `GAP_FLOORS` hold as written (bench-75 static 15.25).
- **B12.10** (F3 in three entry points), **B12.11** (even band of free pins, node-side end order, X2.2 frame breadth for
  displaced entries, the choice corner), **B12.12** (narrowing on block layouts): not built (stop rule). Their
  narrowing tests `a_hub_fed_row_of_shallow_frames_wraps_into_two_rows` and `wide_sibling_boxes_stack_instead_of_a_ribbon`
  pass in the suite-on run as written. `BLOCK_SLACK` is not pinned (it exists only in the workspace patch).
- flow 23 RL (lead note 2): unchanged, judge and sweep ratchet untouched; nothing built here clears it.

## 3. Rows of 4.3 at the head (`acacab7`, static, suite-on CLI; `B/local/out/g1/metrics.tsv`)

Local judge `--due A2`: **435 checks on 92 diagrams, 98 must-fail, 10 targets missed** (`g1/judge-A2.txt`);
`--due A1`: **0 must-fail** (`g1/judge-A1.txt`). By metric: `lateral_x` 18, `compact_max` 14, `sib_stagger` 12,
`lbl_in_end` 12, `lbl_crowd` 12, `compact_mean` 7, `bends_max` 7, `lbl_struck` 6, `lbl_endpx_mean` 2, `bends` 2,
`area` 2, `straight_max`, `lbl_own_gap_max`, `lbl_gap_min`, `detour_max` 1 each. By input: analog 15, v2 10, each
permutation 6-7, the public fixture and each permutation 6, dense-lr 3, infra-bt, hub5, dense-td, cycle2,
blocks-overview-bt, bench-75 1 each.

(a) analog / (b) v2 and permutations / (c) public fixture (must in brackets; NA where blockmetrics has no label):

| metric | analog (must) | v2 (must) | perm a b c d e | public (must) | public bt lr rl |
|---|---|---|---|---|---|
| xframe_runs | 0 (0) | 0 | 0 0 0 0 0 | 0 (0) | 0 0 0 |
| sib_side_linked | 0 (0) | 0 | 0 0 0 0 0 | 0 (0) | 0 0 0 |
| sib_stagger | **1** (0) | 0 | **1 1 1 1 1** | **1** (0) | 1 0 0 |
| lateral_x | **1** (0) | **1** | **1 2 1 1 1** | **3** (0) | 3 0 0 |
| compact_max | **10.37** (5.5) | **16.25** (5.5) | **10.37 10.25 10.37 10.37 10.76** | **25.16** (7.97) | 19.97 18.90 18.90 |
| compact_mean | **5.65** (3.9) | **7.63** (3.9) | | **8.00** (4.63) | |
| span_max | 0.312 (0.37) | 0.249 (0.28) | 0.31 | 0.197 (0.22) | 0.20 |
| crossings | 15 (20) | 14 (17) | 15 18 15 15 17 (20 23 20 20 22) | 6 (7) | 7 6 6 |
| bends / bends_max | **144** / **13** (120 / 8) | **153** / **13** (114 / 8) | bends_max **13** each | 86 / 8 (110 / 8) | |
| detour_max | **1.73** (1.63) | 1.47 (1.76) | | 1.22 (1.43) | |
| lbl_struck | **1** (0) | **3** (0) | **1 1 1 1** 0 | 0 (0) | |
| lbl_crowd / lbl_gap_min | **4** / **0.0** (0 / 12) | **6** (1) | **4** each | **2** (0) | |
| lbl_in_end / lbl_own_gap_max | **3** / **16.5** (0 / 1) | **3** / 0 | **3 2 3 3 2** | **1** / 0 (0 / 1) | |
| lbl_endpx_mean | **392** (386) | **386** (329) | | 237 (295) | |
| lane_gap_min | 12.4 (9.3) | 12.1 (8) | 12.4-12.5 | 12.4 (8) | |
| straight_max | **1601** (1240) | | | | |
| area | **4.60 M** (4.27 M) | **5.21 M** (3.21 M) | | 2.12 M (2.78 M) | |
| lbl_lbl, lbl_node, frame_overlap, frame_foreign_nodes, node_hits, warn | 0 | 0 | 0 | 0 | 0 |

(d), static, at the head: `lateral_x` hub5 3, cycle2 2, infra-bt 1, public-bt 3, dense-td 6 (must 1), dense-lr 5
(must 0), bench-75 2 (must 3); `fan_x` 0 on the analog family and the public fixture, bench-75 4 (start 6), dense-td 0;
`spine_off` 0 on the public fixture, hub5, spread-face, nested-composite, fork-twins (harness); `title_cut` 0 on
title-entry, title-hidden, lr in four directions (B12.7's test). (f) dense: crossings bench-75 204 (334), dense-lr 17
(51.5), dense-td 49 (74.6); `compact_max` bench-75 71.64 and dense-lr 43.03 above the reference (52.03, 11.78).

The edges and sides 4.3 (d) asks to be named were not re-derived this run (no B12.11 work); A1's lists stand
(`checkpoint-A1.md` sections 3-5).

## 4. Suite-on failing tests and the default run

Suite-on at `acacab7` (`B/work/a2c/ton-g1.fail`): lib 8 failed, `tests/quality.rs` 7 failed, the same names as
checkpoint A1's report (comb route half, `far_apart_back_edges_...`, `a_nearly_straight_pair_is_drawn_straight`,
`advanced_showcase_parses_and_renders`, own-state stub, lane in `route()`, two note-link tests; `quality_gate`
(routed rows), `block_fixtures_are_not_drawn_worse`, `defect_baseline_gate`, `channels_keep_edges_apart_and_off_chips`,
`crossing_gaps_do_not_read_as_dashes`, `edges_keep_off_frame_borders`, `a_one_px_drag_leaves_unrelated_edges_put`).
With the workspace B12.8 patch (`c3`, `ton-c3.fail`) `advanced_showcase_parses_and_renders` and
`edges_keep_off_frame_borders` pass, `a_nearly_straight_pair_is_drawn_straight` still fails, and the two routed tests
of section 1 item 2 are added; on the branch the list is A1's. Default run: `GUARD: ALL GREEN` at `acacab7`; off-identity test green.

## 5. Perf (4.6)

`tools/perf.sh b12a2-geometry` (default build, `B/../runs/b12a2-geometry/perf.tsv`): every file inside its budget
(e.g. bench-75 route_ms 12.61 -> 8.83, worst drag 20.22 -> 16.50; dense-td drag 14.24 -> 10.18; dense-lr 14.21 ->
8.78; lr drag 0.95 -> 1.30, inside base + 1.0). One run only: the default build's code paths are unchanged by this
run's commits (all new code reads block mode), so no regression is claimed or needed. `route_partial` not touched.

Block-mode static `scene_ms`, fastest of three (suite-on quality example, load 1.3-1.9): analog 8.97 (limit 11.58),
v2 8.34, public fixture 8.04 (13.6), bench-75 20.09 (61.94), dense-td 24.89 (50.68), dense-lr 10.63 (46.8), advanced
0.89 (1.42), subgraph 0.15 (0.38), fanout-frames 1.82 (4.38). The workspace B12.8 patch adds well under 1 ms on each.

## 6. Musts failed and stop rules fired

- **Stop rule D6 at B12.8**, with the numbers of section 1. A2's row is not met (section 3: 98 must-fail lines at
  the head; the B12.8 patch would take the public fixture's `compact_max` and the analog family's `sib_stagger` off
  the list but adds `bends_max`, `detour_max`, `lane_gap_min` and cannot be committed).
- Every must of A1 holds at the head: crossings (analog 15, v2 14, perms 15 18 15 15 17, public fixture 6, bt 7,
  dense-td 49, dense-lr 17, bench-75 204), `xframe_runs` 0, `fan_x` musts, nested-chain `lateral_x` 0 (test),
  dense-td `GAP_FLOORS` (12.50), the sweep's 8 added diagrams (test), the six shuffles (test).
- The starts table was not re-taken (no ordering change since A1); lead note 7's question stays open for the next
  run.

## 7. Disk

8.8 GB free at the start and the end. `B/target-on` (139 MB, the suite-on build of this run) is deleted after this
report as decision "Disk" asks; the two suite-on binaries of `acacab7` are kept as `B/work/a2c/fm-on-acacab7` and
`B/work/a2c/quality-on-acacab7`. `B/target-dbg` (earlier run) is left as found.
