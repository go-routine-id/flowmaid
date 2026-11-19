<!-- Work package A4 (sequenceDiagram): README.md replacements for the integrator. -->

## Line 23 (feature list), replace the `sequenceDiagram` bullet with:

- [x] `sequenceDiagram`: participants and stick-figure actors, 8 arrow types, notes, activations, autonumber, loop/opt/alt/par frames, participants mirrored at the bottom *(v0.10.0)*

## "Sequence diagrams" section, first sentence (around line 174), replace with:

`sequenceDiagram` input renders participant boxes across the top (repeated below the lifelines, as mermaid's `mirrorActors` does), dashed lifelines between them, and one row per statement top-down:

## "Sequence diagrams" section, replace the "Supported subset: …" paragraph (around line 189) with:

Supported subset:

- **Participants.** `participant` / `actor` declarations with `as` aliases. Actors draw as UML stick figures with their name below. Implicit participants are created on first mention, in order of appearance.
- **Messages.** All eight arrows: `->>` solid with a filled head, `-->>` dashed with a filled head, `->` / `-->` plain lines without a head, `-x` / `--x` cross ends, and `-)` / `--)` async open arrows. Self-messages (`A->>A:`) draw as a loop beside the lifeline. `autonumber` adds bold `1.` prefixes.
- **Notes.** All four forms: `Note over A,B:`, `Note over A:`, `Note left of A:` and `Note right of A:`. The keywords are case-insensitive.
- **Activation bars.** Use `activate` / `deactivate` or the `+`/`-` arrow shorthand (`A->>+B:` activates B, `B-->>-A:` deactivates B). Both forms start and end the bar at the message they belong to, and arrows stop at the bar's edge. Unbalanced deactivation is a line-numbered error.
- **Frames.** `loop` / `opt` / `alt`+`else` / `par`+`and` draw as labelled boxes with dashed dividers. Each frame's conditions share one column next to its chip. By default, frames are aligned full-width bands that widen to contain their notes. Frontmatter `config: sequence: scopedFrames: true` (a flowmaid key, which mermaid ignores) sizes each frame to the lifelines it covers, as mermaid does.
- **Config.** Frontmatter `config: sequence: mirrorActors: false` drops the participant row at the bottom.
- **Comments.** Trailing `%%` comments are stripped. This deliberately differs from mermaid 12, which prints them as part of the message text.
- **Limitations.** Message and note text is single-line (`<br/>` collapses to a space). `box`, `rect`, `critical`, `break`, `create`/`destroy` and `autonumber` arguments produce an explicit "not supported yet" error.

See `examples/sequence.mmd`.

## Line 366 (architecture notes), replace the `seq` module paragraph with:

The `seq` module is different. Sequence diagrams are not graphs, so it skips the Sugiyama pipeline entirely. `seq::scene()` computes a linear layout: columns follow declaration order (gaps are widened until every message label and note fits), with one row per statement, top-down. It returns every box (header and footer), lifeline, message polyline, note, activation bar and frame in final coordinates. `seq::to_svg()` serialises any scene. GUI painters get arrowheads from `seq::head` and actor stick figures from `seq::actor_figure`, as plain geometry.

## docs/docs/index.html (syntax table, `Sequence` row, around line 289), replace the description cell with:

      <td>Participants/stick-figure actors (mirrored at the bottom), 8 arrow types,
      activations, notes, <code>autonumber</code>, <code>loop/opt/alt/par</code> frames</td></tr>
