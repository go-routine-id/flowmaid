<!-- Work package A4 (sequenceDiagram). Merge these bullets into `## [Unreleased]`
     under the matching Keep-a-Changelog headings. -->

### Added

- **Sequence participants are repeated below the lifelines**, as in mermaid
  (`mirrorActors`, on by default). Each lifeline ends on its footer box. Turn
  it off with frontmatter `config: sequence: mirrorActors: false`. New:
  `seq::SeqScene::footer` (index-aligned with `boxes`) and
  `model::SequenceDiagram::mirror_actors`.
- **Sequence actors are UML stick figures** with their name below, at the top
  and at the bottom. Before, an actor was a pale outlined box shaped like a
  participant. GUI painters get the same geometry from
  `seq::actor_figure(&PartBox)`, which returns a `seq::ActorFigure`. An actor's
  `PartBox` is the slot for the figure and its name, `seq::ACTOR_H` (48px)
  tall. Participant boxes stay `BOX_H` and are top-aligned with it.
- **Scoped sequence frames, as an option.** With frontmatter
  `config: sequence: scopedFrames: true`, each `loop` / `opt` / `alt` / `par`
  frame spans only the lifelines and notes it covers, as in mermaid. This is a
  flowmaid key, and mermaid ignores it. Aligned full-width bands stay the
  default. New: `model::SequenceDiagram::scoped_frames`.

### Changed

- **Sequence messages meet activation bars at the bar's edge.** Heads used to
  run 4px into the bar, and lines started from its middle. With nested bars, a
  message on the left meets the outer bar and a message on the right meets the
  innermost one. A self-message loops out from the bar's right edge.
- **Sequence notes are a pale yellow card** (`style::NOTE_STYLE`), as in
  mermaid. They used to be the same amber as the third participant's header
  and activation bars, so a note looked like it belonged to that participant.
- **Frame conditions line up.** An `else` / `and` condition now sits in the
  same left-aligned column as the frame's first condition, next to the chip,
  with the same spacing. It used to be centred. Each condition label has a
  white card behind it, so lifelines no longer run through the text.
  Dividers are drawn as heavy as the frame border (1.2px). Activation bars are
  painted over the dividers and over the label cards and keyword chips, and the
  label text is painted over the bars, so an open bar stays one unbroken bar
  where it passes under a frame or `else` / `and` label.
- **Tighter column spacing next to a self-message.** A self-message's label
  used to keep half of the next participant's header box clear, even though
  that box is not on the message's row. It now only clears the next lifeline,
  allowing for an activation bar there, so that gap is no longer much wider
  than the others.
- **Breaking** (needs a minor bump, which is already required):
  `seq::SeqScene` gains a public field (`footer`) and becomes
  `#[non_exhaustive]`. Use `SeqScene::default()` and set its fields instead of
  a struct literal.

### Fixed

- **An explicit `activate` / `deactivate` bar starts and ends at its
  message**, as the `+` / `-` shorthand always did. It used to start and end
  14px late, one partial row after the message it belongs to.
- **Sequence frames contain their notes.** A `Note right of` the last
  participant, or `left of` the first, inside an `alt` / `opt` / `loop` hung
  half outside the frame, and the border cut through it. Frames (and nested
  frames, inset) now widen together to contain their notes, self-message labels
  and condition labels.
- **A headless dashed message reaches its lifeline.** The dash pattern now
  ends with ink at both ends (`stroke-dashoffset`). Before, a `-->` could stop
  in a 4px gap short of its target.
