<!-- Work package A6 (pie, journey, timeline). Merge these bullets into
     `## [Unreleased]` under the matching Keep-a-Changelog headings.
     Needs a **minor** bump: `PieChart`, `Journey`, `Timeline`,
     `JourneyTask`, `PieScene`, `TaskGlyph` and `PeriodGlyph` gain public
     fields and become `#[non_exhaustive]` (build the model structs with
     `Default` and set fields; the scene structs come from `scene()`). -->

### Added

- **`accTitle` / `accDescr` in pie, journey and timeline** now become the
  SVG's accessible name and `<desc>` (they were parsed and dropped, and the
  name was always "Pie chart" / "User journey" / "Timeline"). Without an
  `accTitle` the diagram's own `title` is the accessible name. New fields:
  `PieChart`, `Journey` and `Timeline` `::acc_title` / `::acc_descr`.
- **Pie labels down to 1%, as in mermaid** (slices under 4% used to get
  none). A label sits inside its wedge when its text box fits there, tested
  against both radial edges and the rim; a slice too thin for that gets its
  label just outside the rim on a short leader line, and neighbouring
  outside labels are pushed apart, away from the pie. The label geometry is
  in the scene for hosts: `PieScene::labels` (`pie::PieLabel`: text,
  anchor, inside/outside, leader). New: `pie::slice_color`,
  `pie::LABEL_R_MAX`, `pie::SEPARATOR_W`, `pie::LEADER_OUT`,
  `pie::OUTSIDE_ROW_H`.
- **Journey faces trace the satisfaction curve.** Each face (with its dots
  and label) sits `journey::SCORE_STEP` lower per point below 5, so the path
  linking the faces rises and falls with the score instead of being a flat
  line. Actor dots and faces carry native tooltips (`<title>Me</title>`,
  `<title>Make tea: 5</title>`). New: `TaskGlyph::{label, actor_ids,
  exact_score}`, `journey::{section_color, actor_color, SECTION_FILLS,
  SECTION_GAP, SCORE_STEP, LABEL_WRAP_W}`.
- **Journey scores like `3.5` or `-1` parse**, as mermaid's `Number()` reads
  them (they were a parse error). The score is clamped to 1..=5; `3.5` sits
  halfway between 3 and 4. New: `JourneyTask::exact_score` (`score` stays
  the rounded whole step).
- **Long journey task names and timeline periods/events wrap** (at 150px and
  160px) instead of widening their column without bound. New:
  `timeline::WRAP_W`, `PeriodGlyph::line_segments`.

### Changed

- **Pie:** radius 110 → 130, labels start at 0.72 r (was 0.62 r), so "5%"
  and "4%" in the showcase sit inside their wedges with room to spare
  instead of reading as "5%4%". Percentages round halves up like mermaid's
  `toFixed(0)` (1:7 prints 13% / 88%, not 12% / 88%). The legend is
  measured at the 13px it is drawn at (the right margin was 34px, now
  `PAD`) and the title as 16px bold; a chart narrower than its title is
  centred under it. Slices cycle sixteen colours (`style::accent_wide`), so
  a ninth slice no longer repeats the first; the first eight are unchanged.
  The white separators are drawn once per boundary, narrower next to a
  sliver, so a 0.35% slice shows as a slice rather than a seam.
- **Journey:** section bands are pale fills (each accent mixed 74% toward
  white) with the name in dark ink, and actors use their own colours, which
  skip green, amber and coral, so a band or an actor dot never reads as a
  score. Neighbouring bands keep an 8px gutter (their rounded corners used
  to notch each other). Columns widen to fit a section's bold name and a
  task's row of actor dots (12 actors overflowed). Long titles are measured
  as 17px bold, and the actor legend is truly centred (it sat ~9px left).
  Task dots and legend dots share one size (5px). A frown no longer rises
  into the eyes.
- **Timeline:** the dashed task line is drawn only in the gaps between
  events, so it never strikes through event text, and the tick is drawn
  over it (with a white ring, radius 4.5). Period labels are bold. The axis
  is darker and 2px (`timeline::AXIS_COLOR` is now `#8a90a8`, 3.2:1 on
  white; the task lines keep the pale colour as the new
  `timeline::TASK_LINE_COLOR`). Periods before the first `section` keep
  neutral ticks (`timeline::UNNAMED_COLOR`) and named sections take the
  palette from its start. A section name wider than its columns widens
  them; long titles are measured as 17px bold and the content is centred
  under them; the title keeps a `timeline::TITLE_GAP` above the first band.
