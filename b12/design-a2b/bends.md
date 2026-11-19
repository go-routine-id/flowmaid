# Study "bends": the yardstick's `bends` / `bends_max` / straight runs

Date 2026-11-17. Read-only study for the follow-up package to step 2 (`b12a2-geometry`, stopped at A2 with 91 must-fail
lines at `7122a53`). `B` = `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12`,
`D` = `B/design-a2b/bends`. Nothing in `/home/user/flowmaid`, `B/metrics` or `B/local` was changed (checked with `cmp`
against the copies in `D/orig`). No worktree or cargo build was needed: the candidate SVGs `B/local/out/fr4-split` are
byte-equal to fresh renders of `B/work/a2i/fm-on-split` (`7122a53`, block mode; 5 inputs checked), and the three
reference tables are reproduced cell for cell by the live `blockmetrics` from the stored SVGs (`B/local/out/start`,
`fm038`, `mermaid` and the `t9bt-*` dirs; the routed twins from `start/q`, `fm038/q`).

## 1. Verdict

**The claim of fix round 4 is right: it is a measuring bug.** `blockmetrics::straightRuns` (live file line ~510) grows
each straight run on the line of the run's **first** segment. flowmaid paints every rounded corner as two cubics whose
outer control points lie on the straight stretches, so the curve meets a stretch asymptotically; the first segment of
the run after a corner is a curve sample up to 1.5 deg off the axis, its line drifts 0.75 px off the stretch after
about 29 px, and the stretch is cut into several runs, each counted by `bends`. The same function feeds
`straight_max`, `lanes_max` and `lane_gap_min`, which share the bug.

- Analog, 7122a53, `DNS>PSDEV`: live 13 bends; by hand from the path 6 (down, left, down, right, down, left, down);
  corrected counter 6. The 399.8 horizontal is cut at 729.5 and 676.8 exactly as the report says (picture 1).
- v2, 7122a53, `DNS>PSPROD`: live 13; **by hand 6, not the 8 the report states**; corrected 6 (picture 2).
- 0.38, analog, `ARGO>TG` (the source of the `bends_max` bound "0.38's own worst edge"): live 8; by hand 4;
  corrected 4 (picture 3). 0.38's worst edge on the analog has 4 counted turns, not 8.
- Public fixture, 7122a53, `OPS>[OBS]`: live 8, by hand 4, corrected 4 (picture 4).
- mermaid 12.2 paints `L` + `Q` corners of radius 7; its straight parts are exact `L` segments and the live counter
  already reads them right: analog `LANE>OAI` 7 = 7 (picture 7). mermaid's rows change in 15 cells only (sub-pixel jogs,
  section 3).

Independent check against the scene of the same build (the start commit's `struct.scene.edges[].points`, the router's
polyline before painting, compared edge by edge with the start SVGs, `D/scenetruth.js`, `D/data/scenetruth-start.tsv`):
of 841 routed edges the live counter equals the scene's number of axis turns on **154 (18 %)** and is above it on 620;
the corrected counter equals it on **666 (79 %)**, is above it on 25 (every one an unbridged hop, section 5) and below
it on 150 (every one has a scene piece under 32 px, i.e. a straight part under `BEND_RUN_MIN` = 8 px once the two
corner roundings take their share: the yardstick's own rule, unchanged).

Two more checks (`D/data/turnhist.txt`, `D/data/sampling.txt`):
- What each counted bend turns by: on the start's 95 drawings the live counter counts 3427 bends, **1656 of them turn
  less than 5 deg** (no axis turn at all); the corrected counter counts 1921, 118 under 5 deg (real jogs between
  parallel runs whose middle is under 8 px, kinks of 1.5-5 deg on note-link splines, unbridged hops). 0.38: 3411 -> 1888,
  7122a53: 3909 -> 2265, mermaid 3497 -> 3470.
- Sampling: the same painted paths flattened at 6, 12, 24 points per cubic and at the yardstick's ~1 px: the live count
  changes with the sampling on 631 of 1289 edges (start), the corrected count on 38 (self-loops, single-cubic note links,
  and middles of almost exactly 8 px).

## 2. The corrected counter (a copy; the live judge is untouched)

`D/fixed/blockmetrics` (diff `D/blockmetrics-corrected.diff`). The tolerances keep their meaning (a run continues while a
segment turns < 1.5 deg from the run's line and stays within 0.75 px of it; runs of 8 px or more count); what changes
is the line they are measured from, and that a rounded corner is one turn:

1. **`seedRuns`: the run's line is the line of its longest segment**, not its first: segments seed runs longest first
   (on a painted path the `L` segment, or the straight middle of a cubic), each seed takes its unclaimed connected
   neighbours on both sides under the two tolerances. (The repository's own harness, `src/quality.rs::straight_runs`,
   already does exactly this, and its doc comment describes this very bug.)
2. **`straightRuns`: collinear consecutive runs merge.** Two consecutive runs of 8 px or more are one when the shorter,
   and whatever lies between them, never leaves the 0.75 px band of the longer one's line (a corner tail that has not
   left the line yet; mermaid's sub-pixel jogs). A jog wider than the band, a hop or a kink that leaves the band stays.
3. **`turnRuns` (used for `bends` / `bends_max` only): a rounded corner is one turn.** A long-tailed corner (flowmaid
   paints some with a control arm of 360 px, bench-75 `user>ext`, dense-td `n31>n38`) holds an 8 px piece straight
   within 1.5 deg that has already left the band. A run B between P and N is such a piece when it turns the same way
   from P and into N, a quarter turn or less in all (90 + 2 x 1.5 deg), the path turns straight on from it to the
   neighbour it is within 45 deg of (no connector that steps aside, so a jog is kept), and it is shorter than that
   neighbour; the shortest such piece goes first. A Z, a U and a straight-drawn 45 deg chamfer keep every run.

Selftest of the copy: the 790 hand-worked checks of the live yardstick all pass unchanged, plus 23 new unit checks:
an L with one painted rounded corner = 1 (live 2); a Z with two painted corners = 2; a U = 2; a straight line drawn as
collinear pieces with 0.1 px print rounding and a straight cubic = 0, one run of its whole length; mermaid's rounded
`L`+`Q` path (the analog's `DNS>PSDEV` as mermaid draws it) = 4, the live counter agrees; a lone `L`+`Q` corner = 1; the
7122a53 `DNS>PSDEV` path = 6 (live 13); bench-75 `user>ext` (long-tailed corner) = 3; a 28 px Z step with 8.7 px of
straight middle = 2 (live 1); a Z step with no 8 px straight middle = 1 (rule unchanged); a 0.5 px jog drawn with two
tiny quadratics = 0, a 1.5 px one = 1; a sharp 2 deg kink between 100 px lines = 1; a sharp 45 deg chamfer = 2; a
quarter arc of radius 100 = 0; a flowmaid hop on a straight leg = 0. Run: `D/fixed/blockmetrics --selftest` (813 pass).

The corrected count is not one-sidedly lower: on 23 edges of 7122a53 (13 of the start) it is higher, every one a Z step
whose middle has 8.0-8.7 px of straight part that the live counter cut into pieces under 8 px (picture 5, public
fixture `SHOP>CDN` 3 -> 4; by hand 4).

## 3. Re-measured references and regenerated thresholds

All rows the thresholds come from were re-measured with the corrected copy: start (95), 0.38 (95), mermaid (95), the
routed start and 0.38 twins (92 each), and 7122a53 (`D/data/fix-*.tsv`; live re-measurements `D/data/orig-*.tsv`).
Changed columns: flowmaid rows `bends` (71 rows), `bends_max` (67-71), `straight_max` (55-62), `lane_gap_min` (28-41),
`lanes_max` (1-2); mermaid rows `bends` 15, `bends_max` 2, `straight_max` 2, `lane_gap_min` 2 (all sub-0.75 px jogs;
none on the analog or v2; the public fixture family 92 -> 90). No other column changes (`detour_max`, crossings,
labels, frames, `lateral_x`: 0 cells).

`mkthresholds.py` types its constants in; `D/mkthresholds-derived.py` (diff `D/mkthresholds-derived.diff`) is the same
rules with every constant that was read off a reference value of a changed column computed by the formula its `why`
states. `--check` proves the formulas: on the live tables they give 120 / 8 / 1240 / 9.3 / 114 / 110 and reproduce the
live `thresholds.json` and `thresholds-routed.json` byte for byte. On the corrected tables
(`D/thresholds-corrected/`, diff `D/thresholds-corrected.diff`):

| rule (scope) | formula (`why`) | reference live -> corrected | must live -> corrected |
|---|---|---|---|
| `bends_max` analog | 0.38's own worst edge | 0.38 analog 8 -> 4 | **8 -> 4** |
| `bends_max` v2 | the analog's bound | | **8 -> 4** |
| `bends_max` perm-a..e | 4.3 (b), the analog's bound | | **8 -> 4** |
| `bends_max` public fixture + 4 perms | 4.3 (c), the analog's bound | | **8 -> 4** |
| `bends` public fixture + 4 perms | mermaid's + 20 % | mermaid 92 -> 90 | **110 -> 108** |
| `bends` analog | mermaid's total + 20 % | mermaid 100 -> 100 | 120 (same) |
| `bends` v2 | mermaid's + 20 % | mermaid 95 -> 95 | 114 (same) |
| `straight_max` analog | mermaid's | mermaid 1236 -> 1236 | 1240 (same) |
| `lane_gap_min` analog | 0.38's | 0.38 9.3 -> 9.3 | 9.3 (same) |
| every rule read at judge time (`ref`/`min`/`max`) | | reads only unchanged columns | unchanged |
| routed rules (`thresholds-routed.json`) | | no changed column | unchanged (byte-equal) |

Nothing gets looser: 6 rule rows get tighter, the rest are equal.

## 4. Must-fail lines at 7122a53 (local judge, `--due A2`)

Live judge + live tables: 444 checks, **91** must-fail (reproduced exactly). Corrected counter + corrected references +
regenerated thresholds: 444 checks, **89** must-fail, 10 targets missed (same 10). `--due A1`: 0 -> 0.
(`D/data/judge-live-A2.txt`, `D/data/judge-corrected-A2.txt`, diff `D/data/judge-diff-A2.txt`.)

| line | live | corrected |
|---|---|---|
| analog `bends` | 141 > 120, must-fail | 70 <= 120, **disappears** |
| v2 `bends` | 147 > 114, must-fail | 72 <= 114, **disappears** |
| analog `bends_max` | 13 > 8 | 6 > 4, stays |
| v2 `bends_max` | 13 > 8 | 6 > 4, stays |
| perm-a, b, c, d, e `bends_max` | 13 / 12 / 13 / 13 / 13 > 8 | 6 > 4 each, stay |
| analog `straight_max` | 1590 > 1240 | 1919 > 1240, stays (the truthful run is longer) |
| public family `bends_max`, `bends` | 8 <= 8, 83 <= 110 (pass) | 4 <= 4, 53 <= 108 (pass) |

No line appears. The 7 `bends_max` lines stay at 6 against 4: `DNS>PSDEV`'s six real turns (the step below EXT, the
displaced crossing of DR's title side and the jog back, fix round 4's reading of 2.8 rule 2) are geometry, not the
measure. **Fixing the counter without re-deriving the bound would drop those 7 lines too (82, `D/data/judge-corrected-
counter-old-thresholds-A2.txt`); that would be a loosening, and the derived bound 4 prevents it.**

**A rule question for the lead (not decided here):** measured truthfully, "0.38's own worst edge" (4) is now below
mermaid's own worst edge (7 on the analog, 6-8 on v2, the perms and the public family). Judged with the corrected
judge, mermaid's own rows fail 12 `bends_max` lines they passed before (`D/data/sanity-corrected-mermaid.txt`; 16 -> 28
must-fail). With the old measure 0.38's 8 sat above mermaid's 7, so rule and the user's bar ("like mermaid or tidier")
agreed; now they do not. The unchanged rule gives 4; any other reference would be a rule change and is the user's call.

## 5. Other metrics: which share the bug

- `straight_max`, `lanes_max`, `lane_gap_min`, `bends`: **yes**, same function; corrected with it (section 3).
- `detour_max`, `crossings`, `xframe_*`, label columns, `lateral_x`, frames: **no** (they do not use straight runs; no
  cell changed).
- **Hops on cubic tails (`dehop`), a sibling bug, not fixed in the main copy.** `dehop` bridges a hop only after a
  segment of 2 px or more; a hop painted on a corner cubic's straight tail follows a ~1 px curve sample, so 39 of the
  start's 587 drawn hops and 43 of 7122a53's 351 are not bridged: each leaves a spurious bend, cuts a straight run and
  adds ~5.7 px of length. It accounts for all 25 over-counts against the scene (picture 8: analog `R2>[DR]`, 5 counted,
  4 by hand). Prototype `D/fixed2/blockmetrics` (diff `D/blockmetrics-hopfix-on-top.diff`: borrow the direction from the
  last 2 px when they lie within 0.3 px of one chord, judge the carrying-on line at its start): bridges 15 (start) /
  24 (7122a53) more, scene over-counts 25 -> 19, selftest passes; 24 / 19 hops remain (ends not level on a curving tail, e.g. bench-75
  `svc3>db`), so it is unfinished. Effect on A2: no line changes (89), analog `straight_max` 1919 -> 2003; it changes no
  threshold source (0.38 draws no hops, mermaid unchanged); on the start rows it also moves `edge_len`, `lbl_off_*` and
  one `lbl_crowd` cell, so it needs its own review before it goes live.
- **The repository harness** (`src/quality.rs`, not touched): `straight_runs` is already stage 1, but `quality-on-split`
  reads `bends_max` 10 / 12 / 10 / 10 / 16 on the analog / v2 / perm-b / bench-75 / dense-td where the corrected
  yardstick reads 6 / 6 / 6 / 6 / 12. Emulating its sampling (12 points per curve, no hops) on the painted paths gives
  10 / 11 / 10 / 10 / 16: coarse samples of a long corner tail are chords of 8 px or more turning > 1.5 deg, counted as
  legs. Stages 2-3 on the same coarse samples give 6 / 6 / 6 / 6 / 12. `tests/quality.rs` asserts no `bends_max` yet,
  so no test moves; whoever wires 4.3 (c)'s `bends_max` into it should port stages 2-3.
- Harness `idiom_spline` (classifies the scene's waypoints / bezier, not samples) and `lat_wiggles` / `lat_backtrack`
  (reversals of the lateral coordinate beyond a tolerance; a corner's monotone tail reverses nothing): **not affected**.

Limits of the corrected counter: on smooth large-radius curves (splines, note links; radius above ~150 px) `bends`
stays "auxiliary" as the README says: a quarter arc of radius 300-400 reads 2 (live 53), 200 and 800 read 6 and 9.
No gated input draws such a curve.

## 6. Pictures (`D/pic/`, each: live | corrected, runs coloured, bends numbered, a corner zoom, the drawing with the edge in red)

1 analog 7122a53 `DNS>PSDEV` (13 -> 6); 2 v2 7122a53 `DNS>PSPROD` (13 -> 6); 3 analog 0.38 `ARGO>TG`, the bound's
source (8 -> 4); 4 public 7122a53 `OPS>[OBS]` (8 -> 4); 5 public 7122a53 `SHOP>CDN`, a rise (3 -> 4); 6 bench-75
7122a53 `user>ext`, long-tailed corner (10 -> 3); 7 analog mermaid `LANE>OAI` (7 = 7); 8 analog 7122a53 `R2>[DR]`, the
unbridged hop (10 -> 5, by hand 4). Script `D/pic.js`.

## 7. Proposal

1. With these pictures, replace the live `straightRuns` (and `bends` with `turnRuns`) by the corrected copy's
   (`D/blockmetrics-corrected.diff`), keep its selftest; update the README's `straight_max` / `bends` rows.
2. Re-derive the constants as `D/mkthresholds-derived.py` does (or type the derived ones in: `bends_max` 4 on
   analog / v2 / perms / public, public `bends` 108) and re-write the reference tables with the corrected copy; update
   spec 4.3's quoted numbers (0.38 analog `bends` / `bends_max` 109 / 8 -> 53 / 4, `straight_max` 1175 -> 1405).
3. Take section 4's rule question (bound 4 against mermaid's 7) to the user.
4. Separately: finish the `dehop` fix (section 5) with its own review; port stages 2-3 into `src/quality.rs` when a
   test asserts `bends_max`.

Expected effect on A2 at 7122a53: 91 -> 89 must-fail lines (analog and v2 `bends` disappear); the 7 `bends_max` lines
stay (6 against 4) and point at `DNS>PSDEV`'s displaced title-side crossing, the step below EXT and the jog back.
