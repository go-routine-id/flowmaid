<!-- Package C1-nonflow (parallel track): entries for CHANGELOG.md `## [Unreleased]`.
     The lead merges them at release; CHANGELOG.md itself is left untouched on this branch. -->

### Fixed

- **Journey and timeline text wraps balanced, and keeps its styling.**
  Long journey task names and timeline periods and events were cut
  greedily, so a last word could be left alone on its line ("… report
  for the | team"); they now wrap like node labels — as few lines as
  before, evened out, a word wider than the column kept whole. A
  timeline's `<b>` / `<i>` run that a break cuts used to lose its style
  on the later lines (each drawn line is read on its own); every line it
  reaches now opens it again. Journeys print tags verbatim (as mermaid
  does) and now measure them as the characters they are, so a name
  with tags keeps within its wrap width and its column. Text that fits
  is drawn as before.
- **A gitGraph merge into a farther lane is drawn as a merge.** The elbow
  between two lanes ran along whichever lane was farther from main, so
  `checkout feat` then `merge main` came out as a fork out of main's
  commit, in feat's colour, and a fork into a nearer lane (a branch
  given a lower `order:` than its parent) as a merge. Like mermaid, a
  fork now leaves its parent across the lanes and runs along the new
  branch, and a merge runs along the merged branch, in its colour, and
  climbs into the merge commit, whichever lane is farther out. (A commit
  sitting on the run's stretch still moves it to the other lane.)
- **`branch x order: 0` ties the way mermaid's does.** Mermaid reads a
  branch without `order:` as `order: 0` and keeps creation order among
  ties, so an explicit `order: 0` branch sits among the unordered ones
  in the order they were created; flowmaid put every `order: 0` branch
  before all unordered ones. Main still comes first, and `order: 1` and
  up still follow.
- **An ER label no longer sits on a neighbouring relationship's line.**
  Where two relationships run close (a pair between the same entities, a
  cycle), a label pill could cover the other line, which then read as
  passing under the wrong label ("employs" over "works in"). A pill that
  covers another line now moves along its own line, or beside it, to a
  spot clear of every other line and every crow's foot — only when that
  spot covers no more pills or tables than where it was (a pill over a
  pill hides more than one over a line), so in the tightest layouts it
  can still stay. A pill clear of the other lines stays where it was;
  the corpus, example and fixture diagrams are unchanged. (The crossing
  "X" a pair of relationships drew under `direction LR` was already gone
  with 0.34.0's pair-aware ports; a test now holds it for LR, RL, TB and
  BT.)
- **Pies with more than sixteen slices tell every slice apart.** The
  sixteen slice colours start over at the seventeenth slice, which then
  took the first one's colour — and on a full pie sits right beside it —
  with the same legend swatch. Each later turn of the colours is now
  hatched over its fill, wedge and swatch alike (stripes in the canvas
  colour at 45° for slices 17–32, then 135°, 0° and 90°), and a label
  inside a hatched wedge keeps a halo in its slice colour. Pies of up to
  sixteen slices draw exactly as before.
- **Journeys with more than ten actors tell every actor apart.** The ten
  actor colours start over at the eleventh actor, whose dots then
  matched the first one's in the legend and on every task. Each later
  turn of the colours now draws its dots as another shape — squares for
  actors 11–20, diamonds for 21–30, triangles for 31–40. Journeys of up
  to ten actors draw exactly as before.

### Added

- `pie::slice_hatch(i)` (with `pie::HATCH_W`, `HATCH_PITCH`,
  `HATCH_OPACITY`): the hatching a host draws over slice `i` past the
  sixteenth, so a painter of `PieScene` matches the SVG.
  `journey::actor_mark(i)` and `journey::ActorMark`
  (`Circle`/`Square`/`Diamond`/`Triangle`, `#[non_exhaustive]`): the
  shape of actor `i`'s dots, for painters of `JourneyScene`.

<!-- README follow-ups for the lead (README.md is outside this track):
  - "Feature" list, `journey` line: "... actor legend + per-task dots with
    tooltips (dots change shape past ten actors), long task names wrapped
    (balanced, no stranded last word)".
  - "Feature" list, `gitGraph` line: "branches (with `order:` to place
    lanes, ties in creation order as in mermaid)"; "Forks and merges are
    drawn as mermaid's rounded elbows (a merge runs along the merged
    branch)".
  - Pie section: "extended to sixteen colours before they repeat" ->
    "... sixteen colours, each later turn hatched so no two slices look
    alike".
  - Timeline section: "periods or events wider than 160px wrap" ->
    "... wrap, balanced, keeping their <b>/<i> styling across lines".
  - layout::wrap_text (+ lines_at and its test) has no caller since the
    journey/timeline change: delete it together with
    wrap::RETIRED_GREEDY_WRAP when layout.rs is merged.
-->
