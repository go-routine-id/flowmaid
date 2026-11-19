<!-- Work package A2 (erDiagram). Merge these bullets into `## [Unreleased]`
     under the matching Keep-a-Changelog headings. -->

### Added

- **ER attribute comments are drawn**, in a fourth column (type | name |
  keys | comment, the comment muted and italic), instead of being silently
  dropped: the comparison fixture lost all 33. A comment wider than
  `er::COMMENT_MAX_W` (280px) is cut with an ellipsis and keeps its whole
  text as the row's tooltip (`<title>`). For GUI painters, `er::ErRow` gains
  `comment` / `comment_full`, and `er::ErTable` gains `title`, `name_col_w`,
  `keys_col_w` and `comment_col_w` plus `name_x()`, `keys_end_x(w)` and
  `comment_x()`, so they lay rows out exactly as the SVG does. A table
  without comments keeps its keys against the right border, as before.
- **ER `direction TB|BT|LR|RL`**, previously a parse error (`unknown
  relationship operator near: 'TB'`): new field `model::ErDiagram::direction`.
- **ER entity aliases.** `p[Person]` and `c["Customer Account"]` work on a
  declaration or a block opening, as in mermaid, and flowmaid also accepts
  them in a relationship. The header shows the alias while `SceneNode.id` /
  `ErTable.name` stay the entity name. New: `model::Entity::alias` and
  `Entity::title()`. As in mermaid, the first alias given sticks.
- **More of mermaid's ER syntax parses:** `"quoted entity names"`; a block
  on one line (`A { int id PK "key" string name }`, `B {}`); several
  attributes on one line; key lists in any case (`pk, fk`); the spelled-out
  cardinalities and line types (`only one to zero or more`, `one or more
  optionally to one`, `1+`, `0+`, `many(0)`, `many(1)`, `.-`, `-.`); and any
  cardinality symbol on either side of the line.
- **`er::end_glyphs(edge, cards)`** returns the crow's feet `to_svg` draws for
  a relationship, laid along its drawn curve (rerouted relationships
  included), so GUI painters match the SVG.
- **`model::Edge::end_room`**: how much line, at each end, glyphs drawn
  outside the flowchart writer take. The layout spaces two adjacent layers
  so both glyphs and the label fit between them. Non-breaking: `Edge` is
  `#[non_exhaustive]`. Flowcharts leave it at zero, and their output is
  byte-identical.

### Changed

- **ER diagrams flow top to bottom by default, as in mermaid.** The flow was
  hard-coded left to right, which stacked every unconnected table in one
  tall column: the comparison fixture went from a 602×950 portrait with an
  empty right two-thirds to a landscape layout arranged like mermaid's.
  `direction LR` restores the old flow.
- **Crow's feet are easier to read.** They follow the drawn curve, with each
  bar square to the stroke where it crosses it. Along the line they are
  longer (bars 6px apart, a 15px fork, a r=5 circle), but they stay 12px
  across, so the glyphs of two relationships on neighbouring ports never
  merge. They are now painted over the tables, so the border no longer
  covers prong tips that end on it.
- **A relationship that closes a cycle is drawn as a plain line.** It used to
  be laid out as a flowchart back-edge bow, which came in nearly along the
  border and crushed its crow's foot. Each scene edge still runs from its
  relationship's `from` entity to its `to` entity.
- **Entity headers:** the accent band is square where it meets the body (no
  more white notches at the join), and the table outline is drawn on top of
  it.
- **Breaking** (needs a minor bump, which is already required):
  `model::Entity`, `er::ErRow` and `er::ErTable` gain public fields and
  become `#[non_exhaustive]`. The parser and `er::scene` build them, so only
  struct literals and exhaustive patterns outside the crate are affected.

### Fixed

- **ER relationship labels no longer cover the crow's feet.** The label pill
  sat on the line's midpoint and was painted last. On the fixture it hid the
  zero circle, so `o{` read as `|{`; on a line widened only enough for its
  label, it hid both ends. A label now sits in the stretch the glyphs leave
  free, or beside the line when there is none, and the layout reserves that
  stretch.
- **Rerouted ER relationships follow their route.** A relationship rerouted
  around a table used to be drawn as its straight chord, through the boxes,
  with zero-size glyphs.
