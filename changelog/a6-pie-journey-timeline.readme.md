<!-- Work package A6 (pie, journey, timeline): README edits. -->

1. Feature checklist, `journey` line — replace with:

- [x] `journey` — title/section/task, score-colored smiley faces (1–5) that rise and fall with the score (the satisfaction curve), fractional scores like `3.5`, pale section bands, actor legend + per-task dots with tooltips, long task names wrapped *(v0.14.0)*

2. Feature checklist, `pie` line — replace with:

- [x] `pie` — title, showData, percentage labels down to 1% (inside the wedge when they fit, else outside on a leader) + legend *(v0.10.0)*

3. Feature checklist, `timeline` line — append before the version tag: `, bold periods, long periods/events wrapped`.

4. Feature checklist, accessibility line — replace the parenthesis with:
   `(flowchart, state, class, sequence, ER, pie, journey, timeline; parsed and not yet drawn for gitGraph, architecture)`
   (if packages A5/A8 land, drop those two as well).

5. "Pie charts" section, the "Supported subset" paragraph — replace the sentences from "Slices under 4% skip…" to "…but not yet written into the SVG." with:

Every slice of at least 1% gets a percentage label, as in mermaid: inside its wedge when the text fits there, otherwise just outside the rim on a short leader line (thinner slices keep their legend row). Percentages round halves up like mermaid's `toFixed(0)`. Zero-value slices are legend-only, a single 100% slice renders as a full circle, and an all-zero total draws an empty outline instead of NaN geometry. A duplicate label keeps one slice — the last value wins. Colors come from the same stable accent palette as ER/class diagrams, extended to sixteen colours before they repeat. `accTitle` / `accDescr` become the SVG's accessible name and `<desc>` (the value stops at `%%`, as in mermaid's langium grammar); without `accTitle`, the chart's title is the accessible name.

6. "Timeline diagrams" section, end of the "Supported subset" paragraph — replace "Colors cycle through the stable accent palette per section." with:

Colors cycle through the stable accent palette per named section; the unnamed leading group gets neutral ticks. Period labels are bold, the dashed task line is drawn only between events (never through their text), and periods or events wider than 160px wrap. `accTitle` / `accDescr` become the accessible name and `<desc>`.

7. Library section, the `pie` module paragraph — "percentage-label anchors" is now literally true (`PieScene::labels`); optionally add: "Each `PieLabel` says whether it sits inside its wedge or outside on a leader (with the leader polyline), so a host can paint the same labels."
