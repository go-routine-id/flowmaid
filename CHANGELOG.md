# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

Class and state diagrams were reworked to read at least as well as
mermaid's, keeping flowmaid's interactive scene API and portable SVG.
Minor, not patch: several class structs gain public fields (see
**Breaking** under Changed).

### Added

- **Class notes are drawn.** `note "text"` and `note for Class "text"` draw
  as yellow cards (`style::NOTE_STYLE`); before, they were silently dropped.
  An attached note is linked to its class by a dotted line. `\n` and
  `<br/>` break lines, as in mermaid's docs (mermaid 12 itself prints `\n`
  as text). Each card is a scene node after the classes, so hosts pick and
  drag it like a box (`ClassScene::notes[k]` describes
  `scene.nodes[boxes.len() + k]`). Each connector is a `DottedOpen` scene
  edge after the relationships (`NoteBox::edge`). `class::route` accepts
  class centres alone, which puts each note 40px before its class, or class
  + note centres, which keeps notes where you put them. New:
  `model::ClassNote` (built with `ClassNote::new(text, class)`, as the
  parser builds it), `ClassDiagram::notes`, `class::NoteBox`. A diagram of
  notes alone draws them (the CLI used to stop with "no classes defined"),
  and `note for` a class the diagram never declares leaves the note free
  instead of creating the class — both as in mermaid.
- **Class stereotypes.** `<<interface>>` (or any `<<x>>`) inside a class
  block, or on its own as `<<interface>> Name`, draws as `«interface»` above
  the class name. The standalone form used to be a parse error, and the
  block form was skipped. New: `Class::annotations`,
  `ClassBox::annotations`, `ClassBox::header_h()`, `class::ANNOTATION_H`.
- **`direction TB|BT|LR|RL` in class diagrams** is honoured (it used to be
  ignored, and the layout was always top-down). An unknown direction is
  still ignored, as in mermaid. New: `ClassDiagram::direction`.
- **Generic classes.** `class Box~T~` is one class `Box` shown as `Box<T>`,
  so a later `Box : +size() int` adds to it instead of making a second box.
  Relation operands may carry the generic too. Member generics read
  `List<int>` by mermaid's own pairing rule. New: `Class::generic`,
  `Class::display_name()`.
- **Static and abstract class members.** A `$` or `*` classifier (after a
  method's `)` or at the end of the member) underlines or italicises the
  row instead of printing the character. New: `ClassRow::is_static` and
  `ClassRow::is_abstract`.
- **Class host helpers that match the SVG:** `class::rel_head(cs, j)`
  returns relationship `j`'s end glyph as drawn, and `class::card_spots(cs)`
  returns where each cardinality sits. Also new: `class::compartment_h`,
  `class::COMP_PAD`, `ClassRel::new` and `ClassRel::source_order`.
- **State diagram notes are drawn.** `note right of X : text`,
  `note left of X : text` and the `note … end note` block form used to be
  parsed and thrown away, so the output was identical with or without them.
  Each note is now a yellow card (`style::NOTE_STYLE`) joined to its state by
  a dotted line, placed the way mermaid places it: `right of` one layer
  after the state along the flow, `left of` one layer before it. A note sits
  in its state's composite, and a note on a composite links to the
  composite's frame. Cards are ordinary nodes, so `scene`, `route`,
  `route_partial`, `hit_test` and dragging work on them. Edge tooltips call
  them "note". A card that lands wholly to one side of its state across the
  flow (in `LR`, a `note right of` card below the next state) is linked from
  the state's side facing it, so the dotted line stays out of the channel
  the state's own transitions and their labels use. When that elbow would
  cross another line from the state (a transition to a state beyond the
  card, or a second note's link), the card keeps the ordinary link instead.
  In a diagram where a transition ends on a composite's frame, the static
  scene and `route` can still choose differently for a card, so its link
  can change shape at the first drag.
- **`layout::CHOICE_SIZE`** (28px), the side of an unlabelled diamond,
  public like `BASE_H` / `MIN_W` so embedders can reproduce node sizes.

### Changed

- **Class hierarchies read top-down, like mermaid and UML.** Relations are
  laid out in source order (left operand first), so in `Animal <|-- Dog`
  the parent sits above the child. Before, parents, interfaces and wholes
  were drawn below their children. That inverted order also caused a false
  cycle, a giant back-edge arc (`charges` in the showcase) and a crossing,
  which are gone too. Scene edges still run `from` → `to`, with the glyph
  at the end, so hosts paint them as before. `ClassRel::head_left` records
  the source order.
- **Class edges are orthogonal with rounded corners, as in mermaid.** Each
  edge goes straight down when it can, takes a single jog otherwise, and
  runs a lane around the boxes for a cycle (or, after a drag, around boxes
  that end up in its way). Ends spread along the faces, away from the
  corners, and meet them square. An orthogonal route never lies along
  another edge's line: when no lane is free, the edge keeps its curve. Jogs
  that share a gap are stacked again in nesting order once all are drawn,
  and the rank gap grows to hold a wide fan's nested jogs, about 14px
  apart. A fan too wide for its parent's face sends its outermost legs to
  the side faces. So a class with up to about a dozen subclasses, or five
  labelled relations into one class, draws without a crossing, as in
  mermaid. Crowded fan-ins (six or more classes pointing at one) can still
  cross: 4 times for six plain ones, up to 12 times for ten labelled ones
  with cardinalities. Much wider fans (twenty or more subclasses) cross
  many times, about 18 for 45 subclasses, where a leg with no free lane
  keeps a diagonal curve across the others. An end whose spread place sits
  a few px beside another route's lane moves just clear of it instead of
  leaving its edge on a curve. A label stays clear of the other routes'
  corners, and a label a later jog runs through moves along its own line. A
  routed edge that would enter its end glyph on a stub shorter than the
  glyph is routed again. Routes come as router-style waypoints, so a host's
  spline draws the same path.
- **Class layout follows dagre where it reads better.** A class
  hierarchy's children stay in one row under their parent (leaf
  staggering, which interleaved the two rows' legs on the parent's face, is
  off for class diagrams). A source ranks just above its nearest target
  instead of on the first rank, so a `note for` sits just above its class
  instead of in the top row with a long connector, and a class pointing at
  one deep in the diagram sits next to it.
- **Class end glyphs and labels.** Glyphs follow the drawn line and are
  painted over the boxes, so a head is never hidden under its box. Ends
  sharing a face spread by their glyphs' widths and keep room for each
  cardinality on the side it takes, so triangles, diamonds and
  cardinalities sit apart instead of overlapping. A box whose face must
  hold many ends grows along it (wider in TB/BT, taller in LR/RL).
  Cardinalities sit beside their end, past the glyph, clear of lines,
  glyphs, boxes, labels and each other, in every direction and on a crowded
  face, nearer their own end than any other. A diagram with labels or
  cardinalities gets an 84px rank gap. In LR/RL, a gap with a label also
  grows to fit the glyph and cardinalities at each end.
- **Class self-relations.** A class with two or more self-relations draws
  them on alternate sides, and loops sharing a side nest, their end glyphs
  apart, the loops with cardinalities outermost so each cardinality sits by
  its own end. Before, the layout stacked them on one side, each starting
  where the last ended, so the loops wound round each other. Loops whose
  side would run into another box (a sibling close beside the class) or
  over another relation's end move to a clear face; past four loops on
  such a class, a loop can still pass under the sibling. A loop's label
  sits past its loop, the labels of one side in a row (a lone loop's label
  used to sit on the loop, hiding most of it on a small box), and the
  automatic layout's canvas grows to hold a loop moved to the left or top.
- **Class members read as mermaid shows them:** `+total() : Money`, with
  `" : "` before the return type (it used to be dropped).
- **Class boxes:** the name band is rounded only at the top, dividers use
  the class accent instead of a pale hairline, and the border is drawn on
  top of everything. Compartments are padded (`COMP_PAD` 4px), and empty
  compartments are taller (`EMPTY_H`, see **Breaking**), so they read as
  compartments.
- **Class `note` lines are checked as mermaid checks them.** `note` without
  text, and `note for` without a class name, are parse errors now. Before,
  flowmaid skipped every `note` line silently. Mermaid rejects both.
- **`<<choice>>` is a small 28×28 diamond**, mermaid's size, instead of an
  empty 80×64.6 one. The rule applies to any diamond with no text (a
  flowchart `A{ }` too): the larger minimum only exists to fit a label.
  Diamonds with labels are unchanged.
- **Fork/join bars span their branches, and the branches drop straight.** A
  bar was a fixed 60×8 rect, so its branches to states about 120px apart
  were S-curves, and a fork's single incoming edge landed 18px off centre,
  in line with one branch. A bar is now stretched to reach every state on
  its busier side. Each branch meets it directly above or below its state,
  and a bar's single incoming or outgoing edge meets it at the middle. In
  `LR`/`RL` the bar stands upright, as in mermaid. The static scene and
  `route` / `route_partial` fit bars the same way, so dragging never
  resizes a bar. Fitting is idempotent: `fold::scene_compact` and a host
  handing `route_sized` the sizes it drew get the same bars, also when a
  bar is a branch of another bar (`f --> j`, nested forks, which now drop
  straight too). Applies to flowchart `@{ shape: fork }` nodes too. A
  composite state on a bar counts as a branch (sized from its members), and
  a note beside a bar does not: the bar centres over its branches and the
  card sits beside them, joined to the bar's end.
- **A `<<choice>>`'s branch labels sit by their targets, in rows.** The
  small diamond's branches leave it a few px apart, so a label kept at the
  middle of its line sat on a sibling's line and read as the wrong
  transition. A choice's labels (any unlabelled diamond's, a flowchart
  `A{ }` too) now stack in rows across the far part of their lines, where
  the branches have spread apart: each on its own line, clear of the other
  lines, and at least 20px from a sibling beside it. The gap after a choice
  grows just enough for the rows. Two transitions to one state, whose lines
  run side by side, put their labels just beside their lines, one each
  side, also among more branches. In `TB`/`BT` such a state is spaced for a
  label either side of the pair, and labels too wide for any row spread
  their states apart so they sit side by side, as in mermaid. A choice
  dragged so close to its targets that the rows run out steps its labels
  beside their lines. Where even that finds no clean spot, a label can
  still touch a sibling's line: in `LR`/`RL`, a label longer than the
  stretch where the branches have spread apart (its states are not spread
  for it), or after a drag toward the targets; and with three transitions
  to one state, whose lines run too close for a label between them, the
  middle label lies over a neighbour's line. The static scene, `route` and
  `route_partial` agree. Flowcharts without such a diamond are unchanged.
- **Tooltips for choices and bars name the state.** A `<<choice>>` now has
  a `<title>` (its id, where it had none), and fork/join bars are titled by
  id (`forked`, `joined`) instead of "fork/join".
- **Breaking** (needs a **minor** bump). Nothing public is removed or
  renamed, and every function keeps its signature. What breaks, and how to
  migrate:
  - **Class structs gain fields and become `#[non_exhaustive]`:**
    `model::Class`, `model::ClassRel`, `class::ClassBox`, `class::ClassRow`
    and `class::ClassScene`. Outside the crate a struct literal and a
    pattern without `..` no longer compile. Get a `Class` from
    `ClassDiagram::ensure_class` and build a relation with `ClassRel::new`,
    then assign the fields. The `class` structs come from `class::scene()`
    / `class::route()`; reading and assigning their fields still works, and
    to change one, clone it and assign (struct update syntax is refused).
    `ClassDiagram` gains `direction` and `notes` too, but it already had a
    private field, so this changes nothing for it.
  - **Class scenes hold notes:** with notes, `ClassScene::scene` has more
    nodes than `boxes` and more edges than `rels`. Notes and their
    connectors come after the classes and relationships. A host that
    indexes `cs.boxes[i]` or `cs.rels[j]` for every scene node or edge must
    stop at `boxes.len()` / `rels.len()`, or zip the lists. A host that
    paints no notes can clear `ClassDiagram::notes` before `class::scene` /
    `class::route` and get the 0.33 shape.
  - **State graphs hold notes:** a parsed state diagram's `Graph` (and so
    its scene) now has a node per note (a `Shape::Rect` painted in
    `NOTE_STYLE`, id `__note_<k>` unless the source names it) and a
    `DottedOpen` edge joining it to its state. A host that treats every
    node as a state should skip those cards.
  - **Public constants with a new value or meaning:** `class::EMPTY_H` is
    12 (was 8). `class::HEADER_H` (28) is now the name band without
    stereotype lines; `ClassBox::header_h()` adds `ANNOTATION_H` per line.
    A member compartment is `class::compartment_h(rows)` tall, `rows ×
    ROW_H` plus `COMP_PAD` above and below.

### Fixed

- **Class names with hyphens or backticks parse, as in mermaid.** A
  hyphenated name used in a relation or a `note for` (`Animal-Name --> Dog`,
  `A-B --> C-D`, `A- --> B`) was a parse error, though `class Animal-Name`
  already declared it; `--` and `..` stay operators, scanned from the left,
  so `A--B` still links `A` to `B`. Backtick names (`` class `Foo Bar` ``,
  `` `Foo Bar` --> Baz ``, `` note for `Foo Bar` "x" ``) were parse errors
  or kept their backticks in the class name; the text between the backticks
  is now the class, and a colon, quote or operator inside it is part of
  the name. A marker letter glued to a name no longer hides the operator:
  `A--oC` links `A` to `oC` (it was a parse error) and `my_o--B` links
  `my_o` (it was the class `my_` aggregating `B`), as mermaid reads them.
- **Edge picking covers exactly the painted stroke.** `Scene::edge_at` (and
  so `hit_test`, also for ER and class scenes) measured to a fixed sampling
  of the drawn curve, a few points per spline span, which cuts inside a
  long gentle bend by up to 2px: a click on the painted line could miss
  it at a small tolerance, or pick a neighbour whose sampled chord, not
  whose stroke, passed there. It now measures to the curve itself, plus
  half the stroke width, so a point on a thick or `linkStyle`-widened edge
  picks it even at `tol = 0`, and where strokes overlap the topmost wins.
  A `~~~` link that a `linkStyle` makes visible is drawn, so it is now
  pickable and marquee-selectable too. Picking also got about 4x faster on
  large diagrams (no allocation per edge, far spans rejected by their
  control-point box). Rendering is unchanged.
- Rerouted class relationships used to be drawn as a straight line through
  the boxes they had been routed around, with their inheritance triangles
  collapsed to nothing. The 'contains' label sat 18.5px off its line. The
  class writer now draws the route through the shared `scene::edge_path_d`.
- **Parallel edges no longer cross where their ends converge.** Two edges
  between one pair whose ports bunch up at one end (the lean pins both to a
  box's corner, or they leave a small diamond at almost one point) head the
  other way round seen from there, so spacing that end's ports in heading
  order swapped them: the lines crossed and the router then rerouted one
  round the far node's side (in `LR`, over its top). Port spacing now keeps
  parallel edges in the order they are drawn in; only the final reorder,
  which uncrosses legs, still places them by heading.
- **`note "text" as N` no longer swallows the rest of a state diagram.** It
  used to start a block note that ran to the end of the file, so every
  later line vanished. It is now a free-standing note card named `N`.
  mermaid drops it entirely. A block note with no `end note` shows the
  lines it swallowed, so the mistake can be seen.
- **A state note's link no longer jumps on the first drag.** In a diagram
  with composite states, the static layout judged whether the link could
  elbow round from the state's facing side against the courses of edges it
  never draws (the stand-ins that lay out transitions to a composite),
  while `route` and `route_partial` used the drawn transitions only, so the
  first drag moved the link 60–80px onto another course. Both now test the
  elbow against the same courses: the drawn transitions, plus each
  transition to or from a composite as far as it is drawn, up to the
  composite's frame (so a note inside the composite keeps its elbow where
  no line crosses it). Two notes beside one state no longer
  block each other's elbow either, so both links now leave the facing side.
- **A hand-built mindmap of any depth renders.** The fan layout recursed a
  few stack frames per level, so a chain several thousand levels deep (built
  with `Mindmap::add_node`) overflowed the stack. Maps deeper than 200 levels
  now take the ring layout.
- Doc comments: `gitgraph::FONT` now says that since 0.33 it no longer
  sizes tags, which are a size smaller. `pie::slice_color` matches
  `style::accent(i)` for slices 0–7 and again for 16–23, not only for the
  first eight as its doc (and the 0.33.0 notes) said.

## [0.33.0] - 2026-10-31

Advance mode's edge router was rewritten over three phases. Together they
make "no line through a box" a property of the search rather than a case
to detect, and take crossings on the measured scenarios from seven to
zero. ER, sequence, gitGraph, pie, journey, timeline, sankey, mindmap and
architecture diagrams were reworked to read at least as well as mermaid's,
keeping flowmaid's interactive scene API, portable SVG and compact size.
Flowchart output is byte-identical. Minor, not patch: `AdvanceScene` and
several diagram structs gain public fields (see **Breaking** under Changed).
Class and state diagram work lands in a later release.

### Added

- **`AdvanceScene::crossings`** — how many pairs of edges cross in the
  drawing, counted from the geometry actually emitted and carried in
  `scene_to_json` as `"crossings"`. Pairs, not points: two edges that
  cross twice count once.
- **Terminals addressed by id.** An edge end may name a node, a named
  anchor (`a@out`), a sub-element (`a.cpu`), or both (`a.cpu@pin1`), in
  the text DSL and in JSON. Sub-elements are first-class: they have ids,
  rects, anchors, and can be edge endpoints.
- **Contrast helpers in `style`** so hosts can match the SVG writers:
  `relative_luminance`, `contrast_ratio`, `text_on(fill)` (white where it
  reaches WCAG AA 4.5:1, else the new `DARK_INK`), `MIN_TEXT_CONTRAST`,
  `ACCENT_INKS` / `accent_ink(i)` (each accent darkened just enough to
  read as text on white), a sixteen-colour `accent_wide(i)` cycle
  (`ACCENTS` then the new `ACCENTS_DEEP`) for diagrams with more than
  eight items, and a `NOTE_STYLE` card colour outside the accent palette.
  `ACCENTS` and `accent()` are unchanged.
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
- **More of mermaid's gitGraph syntax parses:** the documented header forms
  `gitGraph TB:` / `gitGraph LR:` / `gitGraph BT:` and a bare `gitGraph:`
  (previously "unknown gitGraph orientation: 'TB:'"); `branch name order: n`;
  `merge name type: NORMAL|REVERSE|HIGHLIGHT`; quoted branch references
  (`branch "my topic"`, `checkout "my topic"`, `merge "my topic" id: "m"`);
  and commit messages (`commit msg: "…"` or `commit "…"`), read and not drawn,
  as in mermaid.
- **`gitgraph::REVERSE_MARK`**: the label a REVERSE commit's scene node
  carries, so a host that draws node labels shows the commit crossed out.
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
- **Sankey `nodeColors` and `labelStyle`** (mermaid 12 config). A
  `config.sankey.nodeColors` map (block or `{ … }` flow form, quoted
  labels allowed) paints the named nodes in that colour verbatim (escaped),
  and the palette keeps the nodes around it clear of that colour — hex, CSS
  named colours (`steelblue`) and `rgb()` alike;
  `labelStyle: outlined` draws every label over a white halo and, as in
  mermaid, hangs labels left of the busiest column on their node's left.
  New: `SankeyConfig::{node_colors, label_style}`, `model::SankeyLabelStyle`,
  `SankeyNodeGlyph::{custom_color, label_text, label_lines, fill()}`,
  `SankeyScene::label_style`, `sankey::LABEL_H`.
- **Mindmap `::icon(…)` and `:::class` lines are kept** (they were
  dropped silently). An icon draws as a small built-in line glyph left of
  the label (book, file, code, cog, user, home, star, heart, check, bolt,
  cloud, database, globe, search, mail, lock — matched by name, so
  `fa fa-book`, `mdi mdi-cog` work; anything else gets a neutral sparkle),
  with the icon name as its tooltip; no icon font is needed. Classes go
  into the SVG as a `class` attribute, as in mermaid, so page CSS can style
  the node, and — a flowmaid extension — `classDef name fill:…,stroke:…,
  stroke-width:…,color:…` lines in the mindmap colour the nodes carrying
  that class (text still picks a readable ink when only `fill` is given,
  whether it is hex, a CSS colour name or `rgb()`).
  New: `MindNode::{icon, classes, style}`, `MindNodeBox::{font_size,
  font_weight, icon, classes, style, paint(), ink()}`,
  `mindmap::{icon_strokes, label_centre, underline, ICON}`.
- **More of mermaid's architecture-beta syntax parses.** `junction id` /
  `junction id in group` (a bare point where edges meet, drawn as a dot);
  the `{group}` edge modifier (`a{group}:R --> L:b{group}`), which attaches
  that end to the service's group frame, as in mermaid; labelled edges
  (`a:R -[calls]-> L:b`, drawn with flowmaid's white label chip); optional
  `[title]`s (`service db(database)`), quoted titles (`["Edge, west"]`);
  iconify-style icon names (`mdi:database`); and `align row|column …` lines,
  accepted and not applied. All of these used to be parse errors.
- **`accTitle` / `accDescr` are drawn for architecture diagrams**: the title
  replaces the SVG's `<title>` / `aria-label`, the description becomes a
  `<desc>`. New fields `Architecture::{acc_title, acc_descr}`.
- New model fields: `ArchService::junction`, `ArchEdge::{from_group,
  to_group, title}`.
- `ArchScene::title_at`: where each group title is drawn (left end and centre
  line), index-parallel with `scene.clusters`, so a host paints titles where
  the routes avoid them.
- **Constructors for the model structs that became `#[non_exhaustive]`**, so
  downstream code can still build diagrams by hand (see **Breaking** under
  Changed):
  - `Mindmap::add_node(parent, text, shape)` fills in `depth`, `branch` and
    the parent's `children` the way the parser does, which now uses it too.
  - `JourneyTask::new(name, score, actors)` clamps and rounds the score
    exactly as the parser does (which now uses it too). A NaN counts as 3.
  - `ArchService::new(id, title)` and
    `ArchEdge::new(from, from_side, to, to_side)`.

  `tests/model_build.rs` builds each of these models from outside the crate
  and checks that it renders byte-identically to the source it mirrors.

### Changed

- **Every edge is routed on a channel grid** built from the layout, with
  A\* paying length, bends and conflicts, replacing eight hand-tuned
  routers. Edge paths change; node and lane geometry does not.
- **Crossings are negotiated away** rather than accepted: each pass lifts
  the edges involved in a crossing back out of the lattice and routes
  them against all the others while the price of a crossing escalates.
- The canvas now grows to contain routes that leave the lanes.
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
- **gitGraph looks like a git graph, not a draft.** Every line is drawn in
  the colour of the lane it runs along, 5px wide with round caps (it was a
  2px navy thread for every edge). Forks and merges are rounded elbows, as in
  mermaid: a fork drops straight from its parent and runs along the new
  branch, and a merge runs along its branch and climbs straight into the merge
  commit. The old curves hugged the source lane and arrived on a 38-72° slant.
  When a commit sits on the stretch the elbow would run along (main merged
  into a branch that has committed since), the elbow takes the other lane
  instead of running through that commit. Two legs that leave one commit on
  the same side run side by side instead of on top of each other, so a merge
  in and a fork out of the same commit stay distinct.
- **Commit glyphs:** a merge commit is a ring around a light core, REVERSE a
  filled disc crossed out with an X, HIGHLIGHT a square outlined in ink. As in
  mermaid, the commit type decides the glyph before merge-ness does. For
  generic `Scene` painters the glyph is the node shape: `DoubleCircle` for a
  merge, a square `Rect` for HIGHLIGHT, and a `Circle` labelled
  `REVERSE_MARK` for REVERSE.
- **Labels never sit on a line.** A commit id or tag that its own commit's leg
  would cross steps aside of it, and ids are drawn over a white halo. In LR,
  ids are measured: each column sits just far enough right that no id, tag or
  leg reaches into the one before it (`COMMIT_DX` is now the 50px minimum
  instead of a fixed 90px). Short ids pack tighter than before, and long ids
  no longer run together. In TB/BT, commit ids sit left of their commit and
  tags right of it (it used to be the other way round), and each lane sits far
  enough over to fit the ids beside it. The canvas is fitted around everything
  drawn (`PAD` 24px), so a long last id or a TB tag is never clipped.
- **Tags are luggage labels** (12px text in the lane's ink on a white pill
  with a hole). They used to be bold 13px text, styled exactly like the branch
  names. **Branch names sit on pills in the lane's colour**, with white or dark
  text, whichever reads on the pill. A dashed guide runs along every lane.
- **`GitBranch::order` is the lane rank** after `order:` options are applied
  (it was always the creation index). The renderer ranks branches by it, ties
  by index, so a hand-built graph may use any increasing numbers. Colours
  follow the lane.
- Scene edges carry their colour and width in `meta.style`, and an elbow
  carries its corner as both `bezier` control points plus a dense rounded
  `waypoints` polyline that a host's spline reproduces. Merge edges are now
  `EdgeKind::Open`: the scene said `Arrow` but the SVG never drew an
  arrowhead, and mermaid draws none. Commit dots are r=11 (was 12).
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
- **Mindmap layout: children hang off their parent** (report B6). Every
  depth used to sit on a fixed ring round the root (`RING` 172): a sparse
  909×773 canvas with 9% ink and leaf spokes from 55 to 223px. Each child
  now sits on a spoke from its parent, and siblings are swept round it
  clockwise in source order, each taking the first direction where its
  whole subtree stays clear (boxes 10px apart, no connector through a box
  or across another). Spokes start at `mindmap::TRUNK` (46px, border to
  border) from the root and `mindmap::SPOKE` (34px) below; a fan that does
  not fit lengthens its spokes only as far as it needs (searched back down
  after the first length that fits), then each child's own spoke is
  shortened as far as it stays clear, so a leaf branch sits close in beside
  a bushy one. The root hands its spare arc out evenly round the circle, so
  no sector is left empty: on 50 test maps the widest empty wedge round the
  root is under 65° (mostly under 15°), where a shared trunk left 100–157°.
  Other fans keep within 160° of the outward direction when they fit (up to
  280° when crowded) and aim their heaviest child straight on, so a chain
  runs straight; a root's only branch goes right. A crowded root (eight or
  more branches) also tries narrow first-level fans and keeps them when that
  shortens the longest connector. The showcase is 790×443 (node boxes
  cover 25% of it) with every spoke 34–54px; maps of 30–200 nodes need half
  the ring layout's canvas area or less. Boxes never overlap, connectors
  never cross and none runs through a box. The search has a work budget
  that grows with the map — random maps of 25–220 nodes and three to five
  levels lay out in 1–140 ms using at most 36% of theirs — so only a map
  that needs more (a full 511-node binary tree; a few thousand nodes
  against the cap of about a second's work) or that has more than 120
  branches on the root falls back to the ring layout, which is sparse but
  now just as clean (see Fixed). `route()` (drag) is unchanged.
- **Mindmap shapes.** `((circle))` is a true circle round its label (was a
  1.6:1 ellipse) and is drawn as `<circle>`. A node without a wrapper is
  now its own shape, `MindShape::Default` — mermaid's default node: rounded
  top corners, a flat bottom with a darker underline — instead of the same
  pill as `(rounded)`; `(rounded)` is a full pill and `[square]` has sharp
  corners. The cloud is made of round overlapping lobes (it was a jagged
  polygon that read as a second explosion); the bang's spikes are one depth
  all round a stadium that holds the label (they were 4× longer at the
  sides than on top). Both are sized round their label instead of by fixed
  multipliers. New: `mindmap::{BANG_SPIKE, CLOUD_LOBE, UNDERLINE_H,
  UNDERLINE_SHADE}`.
- **Mindmap hierarchy reads at a glance.** The root is filled with a deep
  indigo ink (`mindmap::ROOT_FILL` `#2f3768`, white bold 18px label)
  instead of a pale `#c9cfe0` that receded behind its own branches;
  first-level labels are 15px semibold, the rest 14px; connectors taper
  9 → 4.5 → 2.25 → 2px (were 4.5/3.5/2.5); horizontal padding 16px (was
  12). New: `mindmap::{ROOT_FONT, BRANCH_FONT}`.
- **Sankey colours never clash with a neighbour.** Nodes used to take
  `accent(i % 8)`, so Losses was painted like Electricity (a flat green
  ribbon between them) and Homes (indigo) sat on Industry (steel blue). Each
  node now gets a colour from the sixteen-colour `style::accent_wide` cycle
  that differs (CIE76 ΔE ≥ 35) from every node it shares a ribbon with and
  from the nodes directly above and below it, preferring one no other node
  uses; a diagram without clashes keeps the plain cycle.
- **Sankey labels are measured and kept off each other and the nodes.**
  At `width: 450`, "Electricity (124 TWh)" used to print over "Industry
  (48 TWh)", and a long sink name ran across the middle column. A label now
  slides along its own node's span (half-line steps), then tries the other
  side, until it clears every other label (with room to read two labels on
  one baseline apart) and every other node. Only when no clean spot is left
  inside may a first-column label hang off the left edge or a last-column
  label off the right. The canvas keeps its configured `width` whenever it
  can: the columns first draw in from the margins (to half their span, and
  only as long as no label gets crowded), then labels that still do not fit
  or still crowd wrap at spaces onto more lines (the value kept whole,
  `SankeyNodeGlyph::label_lines`, drawn as `<tspan>`s); only what is left
  over widens the canvas at that edge. Labels between columns cannot hang
  anywhere, so while one still prints on another label or a node (long
  names at a narrow `width`), the columns spread apart in steps of 1.2×,
  the long names still wrapped at the configured width's share, so the
  canvas grows only as far as that needs. A four-column, 17-node diagram
  at `width: 350` comes out 491px wide with every label clear; on 40
  random diagrams of three to five columns with long names at widths
  250–600, every label is clear. Stacked nodes too thin for a line of text
  get a wider gap. Labels are drawn at mermaid's 14px (was 13) and after
  all nodes.
- **Sankey crossings: a backward sweep.** The barycentre ordering only ever
  swept left to right, so the first column kept declaration order and the
  showcase kept one avoidable crossing (Coal→Electricity over Gas→Heating).
  Forward and backward sweeps now alternate and the ordering with the fewest
  crossings is kept — the first forward sweep's unless a later one strictly
  beats it, so the extra sweeps never add a crossing. The showcase now has
  none.
- **Sankey ribbons stack by position.** A ribbon that skips a column was
  stacked by comparing column ranks across different columns; ribbons now
  leave a node in the vertical order of their targets and enter it in the
  order of their sources.
- **architecture-beta lays services out by their ports, like mermaid.**
  `a:R -- L:b` puts `b` right of `a`, `a:B -- T:b` below it, a mixed pair
  such as `a:R -- T:b` diagonally; services feeding one port stack in a
  column in declaration order. Groups stay contiguous: no frame overlaps
  another or covers a service that is not its own. Unconnected groups and
  services are packed in rows of about √n instead of one long strip. On the
  comparison diagram Redis now sits right under Business Logic, Web and Mobile
  share a column, and the canvas is 882×488 (1.8:1) instead of a 1276×192
  strip (6.6:1). The old grid followed declaration order and ignored ports.
- **Edges leave and enter along their port normals and are routed around
  every service box, junction and group title**, orthogonally with rounded
  corners. Edges that share a port merge into one trunk in the gap between
  frames; other edges pay to cross each other, and an edge never turns on,
  runs along (or a hair beside) or passes through the stub of an unrelated
  edge, or runs through a junction's dot — each would draw a junction that is
  not there. Routes keep off frame outlines, and a few outer channels give
  edges going round the drawing room to stay apart. Edges are drawn after the
  boxes, so nothing can hide under a fill. Scene edges carry the routed path
  as densely sampled, corner-rounded `waypoints`, so a host splining through
  them (and `Scene::hit_test` / `edge_at`) follows the drawn line.
- **Service boxes:** labels are 14px (was 13px) and wrap inside the 150px box
  instead of overflowing it, over balanced lines; a single word too long to
  break widens its box. Icons are 24px (was 16px), and icon plus label are one
  block centred in the box (it sat 8px high, and the label 2px left of centre
  because of a trailing newline). Boxes take the accent colour of their
  top-level group instead of one colour per declaration index. Two services an
  edge joins sit 56px apart (was 28px for all siblings), so arrows keep a shaft.
- **Icons:** every glyph whose default presentation is text (☁ ✉ ⚙ 🗄 🖥 🛢)
  carries VARIATION SELECTOR-16, so all icons render in colour (some were drawn
  monochrome). `database` is an oil drum (was a file cabinet), `server` the
  rack-like cabinet (was a desktop monitor), `cpu` a gear (was a brain); new
  names: `desktop`, `firewall`, `container`/`docker`/`package`,
  `settings`/`config`/`gear`, `monitoring`/`metrics`, `log`/`logs`,
  `alert`/`bell`/`notification`, `api`/`plug`.
- flowmaid's own named qualifier `a{g}:R` now means the same as `{group}`
  (the edge attaches to group `g`'s frame) and still checks that `a` is in `g`.
- **Group titles move off an edge that rises through them.** When an edge
  enters (or leaves) the top port of a service right under its group's title,
  the title starts just right of the edge's line and the frame widens to hold
  it, so the edge runs straight instead of striking the title or wiggling
  round it. Titles are measured wide (bold text, colour emoji at 1.3em) and
  edges keep 8px from either end.
- Groups linked to several other groups keep their own grid: each group is laid
  out by its own edges from one anchor, and another group's services no longer
  push its cells around (they used to come out tangled in larger diagrams).
  Two boxes that face each other with a port each keep an edge's gap even when
  no edge joins them, so their stubs never meet and read as a link.
- Routing large architecture diagrams is much faster: the lattice is built per
  clearance instead of per point × clearance, and once a diagram's searches have
  done a lot of work, later ones weight their estimate. A 200-service random
  tangle takes ~1.4 s instead of ~20 s, a 120-service grid ~4 ms instead of
  ~60 ms; ordinary diagrams route exactly as before the budget.
- **Breaking** (needs a **minor** bump). Nothing public is removed or
  renamed, and every function keeps its signature. What breaks, and how to
  migrate:
  - **Model structs gain fields and become `#[non_exhaustive]`:**
    `model::Entity`, `PieChart`, `Journey`, `JourneyTask`, `Timeline`,
    `MindNode`, `Architecture`, `ArchService`, `ArchEdge` and
    `SankeyConfig`. Outside the crate a struct literal (with or without
    `..Default::default()`) and a pattern without `..` no longer compile.
    Start from `Default` and assign the fields for `PieChart`, `Journey`,
    `Timeline`, `Architecture` and `SankeyConfig`; get an `Entity` from
    `ErDiagram::ensure_entity`; use the new constructors for the rest:
    `Mindmap::add_node`, `JourneyTask::new`, `ArchService::new` and
    `ArchEdge::new` (see Added). `ErDiagram` (new field `direction`) and
    `SequenceDiagram` (new fields `mirror_actors`, `scoped_frames`) gain
    fields too, but they already had a private field, so this changes
    nothing for them. `Edge::end_room` is non-breaking because `Edge` was
    already `#[non_exhaustive]`.
  - **Scene structs gain fields and become `#[non_exhaustive]`:**
    `er::ErRow`, `er::ErTable`, `seq::SeqScene`, `pie::PieScene`,
    `journey::TaskGlyph`, `timeline::PeriodGlyph`, `sankey::SankeyNodeGlyph`,
    `sankey::SankeyScene`, `mindmap::MindNodeBox` and
    `architecture::ArchScene`. They come from each module's `scene()` (or
    `route()`). Reading and assigning their fields still works. To change
    one, clone it and assign: struct update syntax
    (`ArchScene { scene, ..old }`) is refused for these types.
    `SeqScene::default()` still exists.
  - **`MindShape`** gains a `Default` variant and becomes
    `#[non_exhaustive]`, so a `match` needs a `_` arm. `Default` is now
    `MindShape::default()`, and the parser gives it to a node without a
    wrapper (it used to give `Rounded`), so code that tested for
    `Rounded` to find such nodes should test for `Default`. `Rounded`
    (`id(text)`) still draws a pill.
  - **Defaults of hand-built models now match the parser.**
    `SequenceDiagram::default()` has `mirror_actors: true`, so a hand-built
    diagram gains the footer row of participant boxes; set it to `false`
    to drop that row. `ErDiagram::default()` now flows top to bottom
    (`direction` is `Direction::TD`) where ER used to be hard-coded left to
    right; set `direction = Direction::LR` for the old flow.
  - **Scene semantics:** in a gitGraph scene, merge edges are
    `EdgeKind::Open` (they were `Arrow`), and every edge carries its lane
    colour and width in `meta.style`. Commit nodes used to all be `Circle`.
    Now a merge is a `DoubleCircle`, a HIGHLIGHT commit a square `Rect`, and
    a REVERSE commit a `Circle` labelled `REVERSE_MARK`. The renderer now
    ranks lanes by `GitBranch::order` (see Changed). Pie slice `i` takes
    `pie::slice_color(i)`, which is `accent(i)` only for the first eight
    slices.
  - **Public constants with a new value or meaning.** They still compile,
    but code that lays things out from them gets new numbers. Each one whose
    meaning changed says what it was in 0.32 in its doc comment:
    - `architecture`: `CHILD_GAP_X` / `CHILD_GAP_Y` are 32 (were 28) and
      now only separate children that no edge joins. Two children an edge
      joins sit at least 56px apart. `GROUP_GAP` (50) now separates a group's frame
      from any sibling, nested groups included, where it used to apply
      only between top-level groups. `SERVICE_W` × `SERVICE_H`
      (150 × 70) is now the smallest box, not the fixed size. `FONT` is
      14 (was 13) and `ICON_FONT` is 24 (was 16).
    - `gitgraph`: `PAD` is 24 (was 30), the margin round the drawn
      content. `COMMIT_DX` is 50 (was 90) and is now the minimum column
      pitch: LR columns move right as their labels need. `BRANCH_DY` (70)
      is now the minimum lane pitch in TB/BT. `BRANCH_LABEL_W` (70) is now
      the minimum width of the LR name column, and TB/BT no longer
      reserve it. `R` is 11 (was 12).
    - `journey`: `DOT_R` is 5 (was 4) and now sizes the legend dots too.
      `DOTS_H` is 20 (was 16).
    - `mindmap`: `PAD_X` is 16 (was 12) and `PAD_Y` is 8 (was 7). `RING`
      (172) is now only the least ring spacing of the fallback layout.
      `FONT` (14) no longer applies to the root or the first level, which
      use `ROOT_FONT` and `BRANCH_FONT`.
    - `pie`: `RADIUS` is 130 (was 110). `LABEL_R` is 0.72 (was 0.62) and
      is now where a label starts: it may slide out to `LABEL_R_MAX`, or
      outside the rim. `MIN_LABEL_FRAC` is 0.01 (was 0.04).
    - `sankey`: `FONT` is 14 (was 13).
    - `timeline`: `AXIS_COLOR` is `#8a90a8` (was `#c2c8dc`) and now
      colours only the axis. The task lines keep the old colour as
      `TASK_LINE_COLOR`. `TICK_R` is 4.5 (was 3).

### Fixed

- A loop from a node back to itself between opposite sides drew a line
  straight across the node instead of a loop.
- An edge from a node to itself discarded any anchor or sub-element it
  named, because the loop was chosen before the terminals were resolved.
- Parallel edges were drawn on top of each other, and five or more
  parallel ported edges could be drawn through the node between them.
- A route could step out and come straight back along the same line.
- **Low-contrast white text on accent fills.** Class and ER headers,
  sequence participant boxes, journey and timeline section bands, pie
  percentages and mindmap branches wrote white on every accent, below
  WCAG AA on six of the eight (amber 2.6:1, green 3.2:1, rose 4.1:1).
  The text now takes white or a navy near-black by contrast with its
  fill, so every label reaches 4.5:1; indigo and violet keep white.
  Gitgraph lane names and tags, and sequence actor names, written in the
  raw accent on white, use the accent darkened as needed. Only text
  colours change: no geometry, and flowchart output is byte-identical.
- **ER relationship labels no longer cover the crow's feet.** The label pill
  sat on the line's midpoint and was painted last. On the fixture it hid the
  zero circle, so `o{` read as `|{`; on a line widened only enough for its
  label, it hid both ends. A label now sits in the stretch the glyphs leave
  free, or beside the line when there is none, and the layout reserves that
  stretch.
- **Rerouted ER relationships follow their route.** A relationship rerouted
  around a table used to be drawn as its straight chord, through the boxes,
  with zero-size glyphs.
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
- **A REVERSE commit no longer reads as a merge.** It was drawn as a white
  hollow ring, which is mermaid's merge marker (report P0 #15).
- **Merge curves no longer strike commit ids.** "merge-hotfix" was crossed out
  mid-word on the comparison fixture.
- **`branch dev order: 2`** created a branch literally named "dev order: 2".
- **`merge x type: HIGHLIGHT`** parsed, but the type was dropped.
- **TB/BT tags on the first lane** hung off the left edge of the canvas, and
  LR tags sat about 2px from its top.
- **A branch name longer than about 9 characters** ran into the first commit
  column. The name column now widens to fit the name.
- **The mindmap ring layout no longer overlaps boxes or runs connectors
  through them.** It put depth `k` on a ring `k · RING` (172px) out and
  spread each family over its parent's whole wedge, so big boxes on
  neighbouring rings overlapped and a connector to a child far round its
  ring cut back across others: on 60 random maps of 25–220 nodes it
  overlapped boxes on 54. Each ring now sits far enough out that its boxes
  clear the ring inside it, every box keeps to its own wedge, a family sits
  packed round its parent, and wherever a connector still bends back
  towards the root or passes through a sibling the gap out to that ring is
  widened until none does. Sparse, but clean on any tree; it is now only
  the fallback of the fan layout (see Changed).
- **architecture: an edge no longer passes behind or through a service box**
  (report P0 #12). On the comparison diagram `web -- api` ran behind Mobile
  App, so it read Web–Mobile–API, and `svc -- cache` ran through PostgreSQL,
  so Redis looked unconnected. Reordering the source drew a different false
  link.
- **architecture: services outside every group** were all drawn on top of
  each other at the canvas origin; they are laid out like any other service.
- architecture: a route over the top row (or left column) and its label chip
  could reach past the canvas edge and be cut off; the drawing now moves down
  or right to keep a margin round every route and chip.
- **architecture: edge label chips no longer cover each other, a service box,
  a junction or a group title.** Each chip is placed after every route is
  known. It slides along each leg of its edge's route. It prefers a stretch
  of line its edge does not share with another edge, and keeps clear of its
  own route's bends. When no stretch has room, it sits just beside its own
  line. Two labelled edges between the same two services (a request and its
  reply on one line) each get a chip's room in the layout, and their chips
  are placed together. Previously a chip went to the middle of the route's
  longest flat leg without checking anything else. On a labelled shop
  diagram, 'read replica' hid 'sql' on the trunk into Postgres and 'https'
  covered 'REST over https'. A chip wider than its leg hid the route's
  corners. Across 800 random diagrams there were 64 chip-on-chip overlaps,
  59 chips on a box and 160 chips on an unrelated edge. Now there are 0, 0
  and 13; the 13 are in dense tangles where no free spot touches the line.

## [0.32.0] - 2026-10-28

Flowchart layout and edge routing rework. Minor, not patch: almost every flowchart's SVG
output changes (ports, routes, label positions, crossing gaps, canvas size), and rendering
is slower than 0.31.1 in exchange for the quality gain.

- **Quality on a set of 13 real-world architecture flowcharts** (static | interactive
  `route()`): edges through unrelated nodes 16 | 77 -> 0 | 0; parallel overlap length
  49930 | 27954 px -> 626 | 376 px; shared ports 221 | 61 -> 0; worst aspect ratio 7.8 -> 2.8;
  layout defects counted by the new harness 425 | 373 -> 28 | 18.
- **Cost:** static `scene()+to_svg` is about 2-35x slower than 0.31.1 (largest measured:
  0.7 -> 24.6 ms native for a 41-edge diagram). The worst interactive `route()` drag frame
  is 9-10 ms native on dense diagrams; `route_partial()` stays under 6 ms. Details per
  stage are under Changed below.

### Added

- **Layout-quality metrics harness.** A hidden `flowmaid::quality` module measures a
  flowchart's static layout and its interactive `scene::route` path (fed the static centres,
  as the playground draws it): edges through unrelated nodes / frames, edge-edge crossings
  and parallel overlaps, cramped gaps, node / frame overlaps, label collisions, shared
  ports, flow-axis wiggles, wrong frame sides, largest-object centring, aspect ratio, and
  `route()` time. `examples/quality.rs` prints it as TSV for a directory of `.mmd` files
  (optionally writing both SVGs); `tests/quality.rs` gates synthetic fixtures under
  `tests/fixtures/quality/` against the 0.31.1 baseline. Not a stable API.

### Changed

- **Router round 2: channels with real separation (breaking output change).** One clearance
  block (`EDGE_SEP` 12, `NODE_SEP` 12, `FRAME_SEP` 14, `CHIP_SEP` 6 px) is shared by the
  edge stages and the quality yardstick, which re-exports it. The repair score now counts
  title chips an edge touches (as hard as a frame), and px instead of counts for runs on
  top of other edges, cramped runs within 12 px of an unrelated edge and runs along frame
  borders or node sides; a chip touch, a border ride or a cramped run over 24 px triggers a
  search for a clearer channel. Every candidate is scored against the accepted edges in
  both modes (`route()` no longer takes the first scene-clean one). The lattice keeps nodes
  12 px off and runs its frame tracks `FRAME_SEP / 2` (7.5 px) off each border, so a
  channel between two frames `FRAME_SEP` apart keeps one track down its middle; the score
  and the nudge hold the full `FRAME_SEP`. A blocked frame gets a banded ring (closed to the
  strict tries; in `route()` tagged with the frame, which is cheaper). The static scene adds
  offset tracks beside accepted legs that took a channel line; the channel detour uses the
  same separations and lanes two tracks deep, ranked against the accepted edges. A rerouted
  end on another side of its node takes the free slot nearest the side's middle instead of
  the shared midpoint. An original whose only fault is crossing a frame between its nodes
  keeps its free curve over a reroute that would ride a border 120 px longer or run cramped
  beside other edges (narrow-channel rule). In the static scene the rule also keeps an
  original whose only faults are frames or title chips over a reroute that threads a channel
  narrower than two frame separations (28 px, between two parallel frame borders or node
  sides) for more than 24 px while riding borders or running cramped longer than the
  original; such a candidate is passed over and the next try may find a clear channel. A
  labelled edge is no longer rerouted for spacing alone (a border ride under 120 px or a
  cramped run) in the static scene: on a reroute its label must be re-placed, and in the
  crowded spots the reroute leaves it, it landed on other edges' strokes and arrowheads.
- **Channel nudge over every orthogonal path, after the ports are respaced.** A new stage
  after `ports::respace` spreads the legs of orthogonal paths that share a channel, port legs
  included, 12 px apart (squeezed to 8, then 6.5 px only when the channel cannot hold them)
  inside the slab left by nodes, frame borders, chips, other legs and free curves, ordered
  so their turns do not cross. A channel too narrow for its legs at 8 px is split at a frame
  border beside it: the fewest legs on that side run past the border, a frame separation
  clear of it (into the frame when an end of theirs lies inside it, or out of the frame they
  run in), and the rest stay in the channel; `route()` then also tries the split with the
  border kept 7.5 px off (just outside a border ride). The last resort squeezes the group to
  6.5 px with nodes 8.5 px and frame borders 7.5 px off. No tier spills legs across or onto a
  frame border any more; a port leg slides its port along a straight side, in order
  and spaced from the side's other ends. The static scene also moves a leg alone in its
  channel off the borders and nodes beside it; `route()` spreads shared channels only. It
  replaces the router's old rerouted-legs-only nudge and border `unhug`. Stair steps under
  28 px are merged into one leg, and finished paths with inner corners are rounded with
  corners up to 36 px where the scene allows (one soft-orthogonal style). Every change of an
  edge is kept only when it scores no worse.
- **Ports follow the routed headings (end crossings), static scene.** After the nudge, a
  side two of whose neighbouring ends cross on their drawn legs is re-placed in the order the
  paths now head (`ports::reorder`), and the change is kept only when the moved edges cross
  fewer edges and hit no more nodes, frames, chips, labels or runs on top of or beside other
  edges. Fixture end crossings: 15 -> 10 static. `route()` does not re-sort (judging the
  sides cost a 75-edge drag frame about 1.2 ms); its routed end crossings stay at 21 (0.31.1
  base: 20).
- **Labels a redrawn path runs through are re-placed.** Besides the labels of edges the
  nudge or the port re-sort redraw, the labels those paths now cross are handed to the label
  stage, which keeps them off strokes and arrowheads where it can.
- **`route()` work accounting; drag frames are slower than the base.** The repair's scoring is charged by what it
  looks at (segment tests and distance samples) instead of per candidate point, finds the
  px an edge runs on top of or beside another as runs of samples (the distance to a segment
  is convex along a segment, so two bisections replace up to 512 samples) with one
  neighbour query per segment, scores an original against the accepted edges only when it is
  needed, and stops sampling a node once a path is through it. In `route()` the first pass
  only asks whether an edge runs on top of another, an edge that only enters a frame is left
  to the channel detour instead of the wide priced search, and an edge no remaining work can
  change is skipped. The nudge's spread has its own small budget in `route()`, and a path the
  nudge leaves alone keeps the router's 14 px corners there (only redrawn paths are judged for
  the wider rounding). The repair budget is the one under "Honest repair budgets" below: on
  the fixtures `route()` puts 3202 px of edges on top of each other (round 1: 3568) and
  9742 px cramped (12268). The nudge costs about 1 ms of a dense drag frame (scoring the
  spread legs, stair steps and roundings); see "Honest repair budgets" for drag times
  (G13 is not met by full `route()` frames on the densest diagrams).
- **Port respacing keeps orthogonal paths orthogonal.** When a respaced end moves past the
  far corner of its path's next leg, the leg after it slides along instead of being dropped
  (which cut a diagonal across the drawing).
- **Fixture gates.** Static ceilings tightened (bench-75 defects 35 -> 15, dense-lr 15 ->
  12, dense-td 15 -> 14, border px 1700 -> 540, gap floors raised) and a new
  `channels_keep_edges_apart_and_off_chips` gate (no edge through a title chip; overlap,
  cramped and hug px in both modes, `route()` at or below round 1). The static label-stroke
  ceiling is 47 (was 43): three labels beside sides re-sorted after the nudge, and one beside
  a labelled curve no longer rerouted for spacing, now sit on a neighbour's stroke. The
  routed cramped-run ceiling is 9800 px (was 9700) since `route()` stopped re-sorting ports.

- **Edge ends get distinct ports; `route()` picks flow-side entries.** Every drawn flowchart
  edge end, plain arrows included (not only start / cross / circle glyphs), now gets its own
  port: ends sharing a node or frame side are ordered by where their edges head and spaced
  up to 14px apart with the least total move, a crowded short side spills its outermost
  forward ends round the corner, and an incoming arrowhead never lands on an outgoing
  edge's start (diamonds, circles and stadium caps included). Both `scene()` and `route()`
  run it. `route()` (and sub-edges in `scene()` that fall back to a free curve) now follow
  the flow direction: a box ahead is left by its flow-out side and entered by its flow-in
  side (in `LR` frames are entered from the left, never top / bottom), a box behind gets a
  back-edge bowing past both ends, side-by-side boxes join on their facing sides, and a
  member joined to its own frame runs to the frame side nearer along the flow. Free-curve
  tangents are capped at half the border gap, so curves never double back.
  `box_edge_bezier` keeps its dominant-axis side rule but uses the same capped tangent.
  Output coordinates of most flowchart edges change.

- **Flowchart `scene()` and `route()` now end in one shared edge-finishing pass.** Both
  builders hand their scene to the same internal pipeline (port assignment, obstacle repair,
  label re-placement, canvas refit) and `to_svg` gained a hook for crossing gaps. The
  pipeline itself adds no output change: what changes comes from the stages listed under
  Fixed. The pass only grows the canvas (never shrinks it), and never shifts a `route()`
  scene's host-given positions.
- **The obstacle repair of edges is skipped on very large or far-flung drawings.** Above 400
  edges, or when the drawing is wider or taller than 1,000,000 px (or its bounds are not
  finite, e.g. NaN centres passed to `route()`), the repair under Fixed is off and every edge
  is drawn as the builder placed it. The cut-off is abrupt: a 401-edge diagram gets no
  rerouting at all.
- **Flowchart edge labels avoid more obstacles, and forced crossings show a gap.** Labels in
  `scene()` and `route()` now also keep clear of subgraph title chips and arrowheads (not
  only nodes, frame borders and other labels). A label is only moved when it covers one of
  those (or its edge was re-drawn); it then slides along its own line, preferring the half
  away from a busy hub and spots no other edge's stroke crosses, and steps half off or just
  beside its line only when no spot on it is clear — never more than one text line from it
  and never nearer another edge than its own, so every label still reads as its own line's.
  Where two edges must cross, the edge drawn under (chosen per crossing, see "Crossing gaps
  no longer read as a dashed edge" below) gets a `<mask>` (`id="<dom-id>-x<i>"`, or a content hash
  without a `diagram_dom_id`) cutting a small gap around the other edge, so it reads as
  passing beneath. Label positions and the SVG of crossing diagrams change; diagrams with no
  crossing gain no markup.
- **Wide layers stagger leaves; largest object centred.** A flowchart laid out more than
  4:1 broader than deep (a ribbon) is now narrowed towards 2.5:1: the breadth is re-packed
  after Brandes-Köpf, sibling subgraphs that share layers may be stacked one above the
  other, sources move down next to their targets, and half the leaves of the broadest
  layer step one layer out (graphviz `unflatten -l`). Each step is kept only when it saves
  breadth without adding edge crossings or edges through nodes beyond a small slack. The
  largest top-level subgraph is moved to the sibling slot nearest the breadth centre when
  that adds no crossing. A top-level node counts as the largest only when its area is at
  least 1.3× that of every other top-level node and of every other node on its layer;
  it then walks towards the middle of its layer (only through swaps that add no crossing)
  and, unless one of its edges is already drawn straight, slides at least 12 px into the
  free room towards the centre. Equal-size nodes keep their declaration order. This search
  (narrowing, subgraph slots and the largest-node walk and slide) runs only for diagrams of
  at most 600 nodes plus edges; larger generated graphs get the single crossing-minimised
  placement with no centring, as before. A subgraph's members that are fed only by one node outside it (a feeder in a nested child
  subgraph counts as inside) now pack into a grid from 3 members (edge-less members still
  from 4). A frame widened to cover a channel lane now stops short of its sibling frames,
  including a sibling that was widened first. Node positions change for ribbon-shaped
  flowcharts, for those with one clearly largest top-level node that sat off-centre, and
  for those whose largest top-level subgraph sat off-centre.
- **Forced crossings read as an under-pass at 1:1.** The gap cut into the under-edge is
  now 1.25 times its own stroke width plus a 1.5px halo on each side of the over-edge's
  stroke (it was a flat 3px; about 7.7px across on a default edge), so mask circles in the
  SVG are larger. The obstacle pass also prices
  a crossing three times higher (five corners' worth instead of under two), so a rerouted
  edge takes a free channel a few bends longer before it crosses another edge: fewer
  crossings, and rerouted paths change.
- **Crossing gaps no longer read as a dashed edge.** Which edge goes under is now chosen per
  crossing so that no edge collects a run of closely spaced gaps (three gaps each less than
  60px apart read as `-.->`). A greedy pass gives each gap to the edge where it chains least
  with the gaps already there (a back-edge under a forward one, then the edge with fewer
  gaps, then the shorter edge, then the later one only break ties); a bounded local search
  then moves gaps out of chains that still read as dashed, and finally evens out how many
  gaps each edge holds. The search is deterministic (it counts moves, not time) and capped
  at 3 moves per crossing (at least 1000 per drawing), so the search's work grows with the
  number of crossings and a drag frame grows by under 1 ms on the densest fixture. On the quality
  fixtures, edges that read as dashed fall from 31 to 10 (static) and 26 to 3 (`route()`),
  and the most gaps on one edge from 31 to 14 and 28 to 11. Long edges now often pass over
  where they used to go under, so the `mask` attributes and mask ids (`<dom-id>-x<i>`, `i`
  the under-edge) of crossing diagrams change, and the gap circles are smaller.
- **Edge labels never sit on a node while an arrowhead or stroke would do.** When no spot on
  its line is clear, a label now takes the spot that hides the least text: covering a node,
  a title chip or another label always ranks worse than covering any number of arrowheads
  or frame border lines. Some fallback label positions change.
- **Edge labels keep off arrowheads, other labels and strokes; label placement is cheaper.**
  What a label spot covers is now ranked, not weighed: hiding text (a node, a title chip,
  another label) loses to covering an arrowhead, which loses to a frame border line or a
  label nearer than 4px, which loses to merely crossing strokes, so a label never settles
  on an arrowhead while a spot that only crosses strokes exists. A label slides along the
  middle 70% of its drawn line (the far half of a hub edge first), then steps just beside
  it (wide labels also beside steep lines, up to twice the old reach; a label still hitting
  something sweeps across the whole line in finer steps), then tries the line's ends. Other
  labels' boxes plus 4px are a hard keep-out. The most constrained labels (fewest open spots,
  then hub edges, ties by edge index) are placed first, and up to three rounds re-try the
  labels a moved label might free. The edge pipeline's final label stage also moves a label
  that another edge's stroke runs through when a spot crossed by fewer strokes exists, even
  if its own edge did not change; the scene builders' pass before the edge stages only
  frees labels from what they cover. On the quality fixtures labels on arrowheads drop
  0 | 1 -> 0 | 0 (static | routed), crowded label pairs 1 | 0 -> 0 | 0, labels on other edges
  43 | 42 -> 34 | 27. The label stage's grids are packed, walked without allocating and
  share one pass over the fixed obstacles, distances are squared, and searches stop as soon
  as nothing can win: on the dense fixtures the stage costs about 1 ms per `scene()` /
  `route()`, about half of before, and `wide-labels` `scene()` drops from 1.3 to 0.2 ms.
  Many label positions change.
- **The quality yardstick counts crossings up to a shared end.** `quality::Metrics`
  `edge_edge_x`, the crossing gaps and the router's crossing count now include crossings
  within 16px of a node the two edges share (two ends on one side crossing on their port
  legs); `edge_edge_x` values are not comparable with earlier runs. `--details` also names
  parallel overlaps, unrelated edge pairs closer than 8px, and labels on nodes or
  arrowheads.
- **Large static diagrams render fast again.** The static obstacle repair's work budget is
  a quarter of what it was (the user diagrams it was tuned on spend under half of the new
  one), and above 400 edges no crossing gets a gap mask: finding the crossings of thousands
  of edges dominated the render and the masks doubled the SVG. Native release, `scene()` +
  `to_svg` on `examples/bench`'s diagrams: 2010 edges from about 1.1 s to 0.56 s (0.6 s
  before the quality work), 5050 edges from about 130 s to 4.3 s (5.0 s before), 400 edges
  from about 118 ms to 66 ms; a dense 100-edge one stays near 19 ms. Diagrams over 400
  edges lose their crossing gaps.
- **Rerouted paths keep off frame borders and parallel edges fan out.** The obstacle
  lattice now prices every step running along a frame border (within 10px, inside or out)
  at a quarter of a run beside another edge, so a path takes a free channel before it rides
  a border. Ends of parallel edges between one pair of ends are spread wider on a run
  flatter than the side's normal (up to 8x the 14px port spacing, as room allows), so two
  shallow curves no longer run 2px apart; in `route()`, parallel edges between one pair get
  their own offset in the cross-subgraph channel and between a node and a subgraph, as
  edges between nodes already did. Rerouted paths and some port positions change.
- **The quality yardstick counts shared corners and border runs.** `quality::Metrics`
  counts two edges touching (closer than 3px, nearly parallel, away from shared ends; or
  crossing twice within 16px) as an overlap (`overlap_pairs`, also counted in the new
  `corner_touch`), and measures edges riding a frame border within 7px for more than 40px
  (`border_runs`, `border_len`). The new columns come from `Metrics::late_fields` and are
  printed after the existing ones, so earlier column numbers stay; `tests/quality.rs`
  gates the fixtures' total `border_len`, and its `defects` ceilings for `bench-75` and
  `dense-td` routed are raised by the newly counted touches.
- **The quality yardstick measures cramping, dashed-looking crossings and drag cost.**
  New `Metrics::late_fields`, printed after the existing columns: `overlap_max` (longest
  overlapping run of one pair), `tight_pairs` / `tight_len` (unrelated edges parallel within
  12px for more than 24px), `hug_runs` / `hug_len` / `hug_max` (an edge along a frame border
  or an unrelated node side within 12px, away from its own ends), `edge_chip` (an edge
  through a frame title chip), `end_cross` (a crossing within 16px of an end both edges
  share, which reads as a junction), `xgap_max` / `dash_edges` (crossing gaps `to_svg` cuts
  into one edge, and edges whose gaps chain three or more within 60px of each other, which
  reads as a dashed edge), `lbl_crowd` (labels closer than 4px) and `ortho_edges` (edges drawn
  as orthogonal router paths). `quality::drag_frames` lists a drag benchmark's frames (each
  node along the drawing's diagonal and nudged 60px each way); `examples/quality.rs` adds
  `scene_ms` and, with `--drag`, the worst / mean `route()` frame and the frames over 8ms.
  `Report` gains `scene_ms`. No gate changes.
- **`route_partial` is incremental.** Only the edges a drag can affect go through the edge
  stages again. Drawn afresh: edges on a moved node or on a frame holding one, and edges
  whose base label covers or touches a node or an arrowhead (with the edge owning that
  head), since such a label stays there, or lands on the head once a port is respaced,
  while the lines are kept. Kept unless they now hit something: edges whose base path
  comes within 12px of a moved node's old or new box, of the strip a frame grew by or of
  its moved title chip. Every other edge keeps its base geometry and is a fixed obstacle
  the others are routed round; only a port it shares with a redrawn edge is spaced again,
  and its label moves only if it now covers something. A frame holding a moved node is
  drawn round its members as `route()` draws it (before this round it only grew, which
  left edges entering it on the wrong side once several members had moved: fanout-frames
  frame_side 180 vs `route()`'s 43 over the drag sweep). The repair budget follows the
  share of edges not kept, from 40 000 up to 60 000 units (detours scale the same way from
  20 000; the channel nudge moves only the redrawn edges' legs), and **when more than half the edges would be
  drawn afresh the frame is a plain `route()`**, identical to it and at its cost. A
  redrawn edge's label starts at its spot on the new line and moves only if it covers
  something; `route()` slides every label before routing, a second label pass that costs
  3-4ms of a dense frame and is left out here. Breaking: an unmoved edge whose base path
  the dragged node now covers is rerouted instead of kept verbatim, and an untouched
  edge's port may slide a few px when a redrawn edge joins its node side. On the dragged
  node's edges the output equals `route()`'s when nothing else changed (tested). `base`
  must be a full drawing at `base_centers` (the `scene()` output and its centres, or a
  `route()` of them), not a previous `route_partial` output. A base that no longer
  matches the graph now also falls back to `route()` when only a subgraph changed (renamed,
  or a node moved into or out of one), an edge's label text, style, end glyphs or
  metadata changed, or a node's text or shape changed (before, the stale edge or label
  was kept).

  Measured over `quality::drag_frames` (sums over all frames, `route_partial` vs a full
  `route()` of the same frame; "moved" = that share of the nodes already moved 15/10px
  from the auto layout before the drag, as an editor that compares every drag with its
  auto layout sees them):

  | fixture, nodes moved | edge-node pairs | overlap px | lbl_node | lbl_arrow | frame_side |
  |---|---|---|---|---|---|
  | dense-lr, 0 | 1237 vs 2630 | 464k vs 390k | 71 vs 61 | 110 vs 130 | 54 vs 57 |
  | dense-lr, 25% | 3330 vs 2375 | 611k vs 447k | 67 vs 63 | 109 vs 132 | 54 vs 57 |
  | dense-td, 0 | 733 vs 1020 | 906k vs 532k | 51 vs 56 | 58 vs 65 | 57 vs 60 |
  | dense-td, 25% | 1865 vs 1102 | 1125k vs 504k | 53 vs 56 | 55 vs 64 | 57 vs 60 |
  | bench-75, 0 | 630 vs 796 | 982k vs 1004k | 0 vs 0 | 0 vs 0 | 0 vs 0 |
  | bench-75, 25% | 1185 vs 823 | 1171k vs 1085k | 0 vs 0 | 0 vs 0 | 0 vs 0 |
  | wide-labels, 0 | 1 vs 1 | 876 vs 602 | 33 vs 26 | 24 vs 67 | 0 vs 0 |
  | fanout-frames, 0 | 181 vs 242 | 9k vs 26k | 0 vs 0 | 0 vs 0 | 48 vs 48 |
  | any fixture, 50% or 100% | identical (the frames are `route()`) | | | | |

  So a single drag from the auto layout is about as clean as `route()` (labels on
  arrowheads and nodes now match it: dense-lr was 231 / 158 before this change), but kept
  edges are not re-spread round the redrawn ones, so parallel overlap stays longer (dense-td
  1.7x), and with a quarter of the nodes already moved edge-node pairs and overlap are
  clearly worse than `route()` (up to 1.7x and 2.2x). On hand-written diagrams whose
  static drawing already has a label on an arrowhead, `route_partial` redraws those edges
  every frame: fewer labels on heads than before this change, but more overlap than a kept
  line, and still several times `route()`'s count. Time (native release, fastest of 3 per
  frame, machine load 7-20): worst single-drag frame 6.1-6.3ms dense-lr, 6.3-6.9ms
  dense-td, 2.7ms bench-75 (before this change 5.9-6.1 / 5.9-6.3 / 1.5); with a quarter of the nodes moved 6.9-13ms (frames past the half-redrawn
  line are `route()` frames); with half or all moved it is `route()`: 13-14ms worst on
  dense-lr / dense-td, i.e. **G13 is not met there**.
- **Honest repair budgets (G13).** The router's work units now follow time: one unit is one
  lattice vertex built (~17ns native); an A\* pop costs 6, the dead-end flood a third per
  vertex, and scoring is charged by what it looks at (four segment tests or distance samples
  per unit; the first scoring pass of every edge is free). The channel detours get a budget
  of their own, and the channel nudge keeps its own spread and scoring budgets. Budgets per
  `route()` repair: 300 000 (search and scoring), 120 000 (detours); per `route_partial`
  repair 40 000-60 000 / 20 000; the static scene keeps 1 000 000 / 120 000, so static output
  is unchanged. 300 000 is about the least that keeps the routed quality gates green (at
  280 000 bench-75 keeps overlaps and defects they reject), and a detour budget under
  100 000 leaves edges through nodes on a large hand-written diagram. Measured over
  `quality::drag_frames` (native release, fastest of 3 per frame, two runs): worst `route()`
  frame 10.6-10.7ms dense-lr, 12.6-13.1ms dense-td, 8.4-8.5ms bench-75 (round 1: 14.6 /
  14.6 / 7.4), 7.3ms the synthetic 75-edge diagram; most of the excess in the worst frames
  is the label stage. **G13 (worst drag frame <= 8ms native) is met by `route_partial` for a
  single drag from the auto layout (worst 6.1 / 7.5 / 3.8ms on the three dense fixtures),
  not by `route()`, and not by `route_partial` once more than half the edges are redrawn
  (it is then a `route()` frame, see above).**
- **Perf guards cover the fixtures.** The ignored `fixture_drag_perf_guard` drags every node
  of `dense-lr`, `dense-td` and `bench-75` through `quality::drag_frames`: `route_partial`
  must stay under 8ms a frame for a single drag from the auto layout (frames with most
  edges redrawn are `route()` frames and are not covered). Full `route()` frames are timed against the same 8ms and
  reported (worst frame and frames over), not asserted, since G13 is not met for `route()`
  yet; no looser ceiling is written down for it. `scene_perf_guard` bounds each fixture's
  static `scene()+to_svg` as a ratio to 0.31.1 (about twice the measured 2-26x). Run them
  one at a time: `cargo test --release --lib -- --ignored --test-threads=1 perf_guard`.
  Static `scene()+to_svg` against 0.31.1 is 2-40x on the fixtures and real diagrams
  (re-measured with the generated fixtures, native release, fastest of 7, machine load
  about 8: 1.9x `back-edge`, 3.3x `wide-labels`, 9.4x `fanout-frames`, 15.6-16.2x the
  dense fixtures, 24x `bench-75`), above the 5x aimed for: the channel router's repair (about 55% of the
  slowest static render, its budget binding) and the narrowing search that fixes a
  ribbon's aspect (about 20%) are most of it. A smaller static repair budget buys little
  time for much quality (at 60% of it `bench-75` static overlap 926 -> 2036 px for 14%
  less time), so it is kept. With the label stage budgeted, the label-heavy worst cases
  are router-bound: a hub with 40 labelled out-edges 19 ms (63x 0.31.1's 0.30 ms), 40
  unlabelled 23 ms (106x), 25 parallel labelled edges 7.3 ms (36x), 50 of them 30 ms (57x);
  the static repair grows about quadratically with edges sharing one channel. **The 5x
  aim is missed and needs an explicit decision.**
- **Layout search budget.** The ribbon narrowing and largest-subgraph centring may lay out at
  most 12 000 element placements per diagram (nodes plus edges per candidate), about twenty
  candidates of the largest searched graph; hand-written diagrams stay far below it. Centring
  is skipped when the largest subgraph already sits within the 5% it would have to gain, so
  a centred diagram no longer lays out every sibling slot (static `scene()` about 20% faster
  on such diagrams, same output).
- **Generated quality fixtures.** `tests/fixtures/quality/dense-lr.mmd`, `dense-td.mmd` and
  `fanout-frames.mmd` are replaced by the output of a new seeded generator,
  `examples/fixture_gen.rs` (`dense 1 TB`, `dense 17 LR`, `fanout 7 LR`): each has its own
  random nesting tree (up to three levels), node and edge counts, a labelled hub fan-out,
  long edges between top-level frames, back-edges into the hub and lorem labels; nothing
  is kept from the fixtures they replace. The two dense ones are harder than before (their
  long edges cross nested frames: 11 and 28 frame entries static), so every gate in
  `tests/quality.rs` measured on them is re-measured, and `tests/quality.rs` gates each
  fixture's `min_edge_gap` against a floor.
- **Label stage work budget (G13).** The edge-label stage now charges every item its
  queries look at (obstacles, stroke segments, other labels) against a budget per pass:
  100 000 units in `route()` / `route_partial` (about 1 ms native on crowded drawings),
  400 000 in the static scene. Past it, the labels not yet searched keep their own line's
  spot (a label left off its line goes back onto it at the spot nearest the preferred
  one). The real and fixture diagrams spend at most a third of it, so their output is
  unchanged; labelled edges sharing one channel no longer grow a drag frame without bound:
  worst `route()` drag frame 25 parallel labelled edges 17.8 -> 5.6 ms, 50 edges 61.8 ->
  15.9 ms, 100 edges 311 -> 23 ms, a hub with 40 labelled out-edges 15.6 -> 11.8 ms (now
  router-bound). There the labels the budget did not reach stay where the builder put
  them, so a few sit on other strokes or arrowheads (G11 traded for G13 on such drawings
  only). `fixture_drag_perf_guard` and `scene_perf_guard` gain both label-heavy cases.
- **Detours keep off the accepted edges; a stacked reroute no longer beats a frame crossing
  (G1).** The channel detour of `route()` and the static scene gets tracks an edge
  separation either side of every accepted straight leg over 96 px near the edge, and
  ranks its candidates by how many legs run within that separation of an accepted one
  before their length. A reroute (searched or detoured) that lies on another edge for over
  12 px no longer replaces an original whose only fault is crossing a frame: that edge
  keeps its free curve (an original through a node or a title chip still takes the reroute).
  On diagrams whose detours used to share lanes, `route()` keeps a channel between them
  (several hundred px less on top of other edges, fewer crossings) for an occasional
  frame entry. On `bench-75` `route()`: one frame entry and three defects more, 14 fewer
  crossings.
- **Crossing notches instead of dashes.** After the under-edge choice, a crossing whose gap
  would make a third gap within 60 px on its under-edge is drawn as a notch: the cut is only
  the over-edge's stroke plus the halo (about 4.5 px, against 7.7 px for a full gap), so
  the crossing still shows which line passes beneath without the line reading as `-.->`.
  `casing::crossings` reports the kind; `quality::Metrics` counts notches in a new
  `xnotches` column (after `ortho_edges`, so the timing columns of `examples/quality`
  move one to the right) and leaves them out of `xgap_max` / `dash_edges`. Dashed-looking
  edges on the fixtures: 9 -> 0 static, 4 -> 0 routed (bench-75 has 20 / 15 notches).
- **Dense crossings are drawn plain instead of cut (G1).** A line takes at most two cuts
  (gaps or notches) within any 100 px; a further crossing in that span gets no cut at all,
  as four to six cuts on one long line read as `-.->` even with notches. `dash_edges` now
  also counts an edge with three cuts, notches included, within `quality::DASH_WINDOW`
  (100 px): on the round-2 output it flagged the long `bench-75` lines the eye reads as
  dashed and the old 60 px chain of full gaps missed. Fixtures: 0 dashed both modes.
- **Title chips, header corner and arrowheads count in the repair.** `edge_chip` (and the
  router's chip score) now covers the chip grown up to the frame's top border and counts
  an edge that passes within `CHIP_SEP` (6 px), not only one that touches it; an edge
  ending on the frame itself still counts only on a touch. `route()` scores only a touch
  (the wider rule cost `bench-75` drags about 800 px of edges on top of each other). Ports
  on a straight side right under a title chip keep out of its shadow. The score gains a
  `head` term: crossings within an arrowhead plus a port zone (27 px) of either edge's end.
  A static reroute that rides along a node it cramps (or crosses another edge at its
  arrowhead) while riding sides 120 px more than the curve no longer replaces a curve that
  hides no node or chip. Gates: static tight 8 520 -> 9 350 px and hug 3 050 -> 3 110 px
  (legs that slipped past chips now take a lane beside another leg); routed `edge_chip` 2
  (`bench-75` legs into the frame the chip names, within 6 px of it, not through it).
- **No edge left through a node by a spent budget (G6).** After the repair, every edge still
  through an unrelated node gets a rescue detour that may cross frames but keeps every leg
  clear of nodes and chips, the edge through the most nodes first (its own 80 000 units).
  Routed `edge_node_x` on the fixtures 2 -> 0, now gated at 0 in both modes. In `route()`
  a reroute that leaves foreign frames may also run on top of another edge for up to 48 px
  per frame it leaves (the nudge spreads such a stretch); before, any such run kept the
  curve across the frames and along their borders.
- **Detour lanes keep off frame borders and crowded channels.** The channel detour adds
  tracks `FRAME_SEP` off the borders of the frames it may cross or run inside, ranks leg
  sets with a leg within a separation of an accepted leg or a border after the clear ones
  (before the rest of the score), and counts only clear leg sets towards its tries (at most
  24 lines looked at). A stair step under 20 px right after a searched path's port stub is
  folded into the port leg when the port can slide there on its side.
- **Static cost (not changed by this round, re-measured).** `scene()+to_svg` against 0.31.1,
  native release, fastest of 7, load about 8: `back-edge` 2.0x, `wide-labels` 3.4x,
  `fanout-frames` 9.8x, dense fixtures 16x, `bench-75` 23x. A static repair budget of 400 000 (from 1 000 000)
  saves 20-30% time but loses the round-1 wins (`bench-75` overlap 778 -> 3 672 px), and
  at 200 000 edges cross nodes again; the layout search and the nudge set a floor of about
  10x without any repair. `scene_perf_guard` now bounds each fixture at about 1.5x its
  measured ratio. **The 5x aim stays missed and needs the owner's sign-off.**

### Fixed

- **Edges route around nodes and frames, also in `route()`.** A new obstacle-aware pass
  reroutes only the edges that cross an unrelated node, enter a frame holding neither end,
  or run on top of another edge, on a sparse lattice with A\* search. It keeps clear of
  nodes and title chips, never steps back along the flow (no S-wiggles), spreads rerouted
  edges that share a channel, and rounds the corners. Back-edges in the interactive
  `route()` now bow around the nodes between their ends instead of cutting through them.
  Rerouted edges are drawn through `waypoints`, so their SVG paths change. Each edge is
  searched on a lattice built only round its own ends. All of it is bounded by a
  deterministic work budget, never by time: lattice vertices built, pruned and priced, A\*
  steps, and candidates scored. `route()` gets a small budget so a drag frame stays cheap:
  on a dense diagram the edges left over when it runs out keep their builder geometry. The
  static `scene()` gets a much larger one and is correspondingly slower to lay out (native
  release, measured: a 75-edge diagram from about 2 ms to about 19 ms, a 400-edge one from
  about 8 ms to 60-70 ms), so a host that re-renders `scene()` on every keystroke should
  debounce it. Spreading a shared channel is kept for an edge only when it adds no
  crossing, overlap or hit against the scene and the other edges.
- **A rerouted edge no longer lands on another edge's port.** When the obstacle pass moves an
  end to another side of its node, the ends on that side are spaced again (without spilling
  round corners), and the rerouted leg follows its port instead of doubling back.
- **`route()` no longer falls back to a free curve across the drawing.** When the obstacle
  pass's lattice is over its cap or its budget is spent, an edge still crossing a node or
  frame now gets an orthogonal channel detour (out along its source stub, across on a free
  channel line between the boxes or round the drawing's outside, in along its target stub),
  with its own small work budget, when that hits fewer nodes and frames. Before, it kept
  its single bezier, which on a dragged diagram could cut through a dozen boxes.
- **Rerouted edges keep off frame borders, labels and each other.** A rerouted leg running
  closer than 10px along a frame's border (inside or out) is moved out to 10px where no
  node is in the way; the lattice prices paths under another edge's label; and in `scene()`
  a reroute that only clears the nodes and frames but runs on top of another edge no longer
  stops the search: a later try may find one beside no edge. Among equal candidates the
  one hugging fewer frame borders and running under fewer labels wins.
- **`route()` no longer panics on a NaN or infinite centre.** A free curve's side point
  clamped its port with `f64::clamp`, which panics when a bound is NaN, so one bad drag
  value trapped the wasm module. The new clamps use `max` / `min`, and the route returns a
  scene as before the quality work.
- **`route()` draws a member joined to its own subgraph like `scene()`.** An edge between a
  node and a subgraph holding it now gets the static layout's straight stub to the nearer
  flow-side of the frame, with its label just outside the frame; before, the label sat on
  the short inner run and covered the node (and the arrowhead).
- **Ends on one side no longer cross at the node.** A side whose ends were already spaced
  was kept as drawn even when two neighbours sat in the wrong order and crossed on their
  legs; before routing such a side is now re-placed in heading order.

## [0.31.1] - 2026-10-27

### Fixed

- **Class and ER diagrams no longer clip nodes dragged past the canvas origin.**
  `class::route` / `er::route` keep caller positions verbatim, but their `to_svg` drew the
  canvas as `0 0 width height`, so a class or table dragged left of or above the origin fell
  outside the viewBox (and out of exported SVGs). The scene is now shifted into view and the
  canvas grown just enough; laid-out and in-bounds scenes render byte-identically. As with
  the flowchart `to_svg`, a shifted drawing's SVG coordinates are offset from the scene's,
  so hosts map pointer events through the rendered SVG.

## [0.31.0] - 2026-10-27

### Added

- **Markdown strings in flowchart labels.** ``"`**bold** _italic_`"`` in node, edge (pipe and
  inline) and subgraph labels: CommonMark-style `**` / `__` / `*` / `_` emphasis with
  flanking and rule-of-three handling, backslash escapes, multi-line bodies (indentation
  dropped, blank lines collapsed), and greedy word wrap of node labels at mermaid 11's
  200px markdown width. Headings, lists, links and other block or inline markdown stay
  literal, as in mermaid's default HTML labels; entity codes never act as markdown. The
  markdown is lowered to the existing `<b>` / `<i>` / newline label markup at parse time,
  so there is no public API change. Edge labels and subgraph titles stay single-line.
  Subgraph titles now render `<b>` / `<i>` markup (plain titles are unchanged). Diagrams
  without markdown strings render byte-identically.
- **Flowchart `click` statements.** Every mermaid form parses: `click ID[,ID…] [href] "url"
  ["tooltip"] [_self|_blank|_parent|_top]`, `click ID callback ["tooltip"]` and `click ID
  call fn(args) ["tooltip"]`, with line-numbered errors for a bad target, a tooltip after
  the target, or a missing url. A linked node is wrapped in `<a href xlink:href [target]>`
  (`rel="noopener noreferrer"` on `_blank`; `xmlns:xlink` declared on the `<a>`, so the
  root header is unchanged) and a tooltip replaces the node's `<title>`. Urls go through
  `model::sanitize_href`, a port of the `@braintree/sanitize-url` checks mermaid's
  default `strict` level uses: `javascript:` / `data:` / `vbscript:` urls, also hidden by
  percent or entity encoding or control characters, are not linked. Callbacks are kept by
  name and never run. Links land only on nodes that already exist (no phantom nodes), and
  tooltips reach later ones, as in mermaid. `to_mermaid` round-trips links, tooltips,
  targets and callback names. New: `model::NodeLink`, `model::LinkTarget`,
  `model::sanitize_href`, a `Graph::links` field (non-breaking: `Graph` has a private
  field) and a `SceneNode::link` field (non-breaking: `#[non_exhaustive]`). Diagrams
  without `click` render byte-identically.
- **Flowchart `linkStyle`.** `linkStyle <n>[,<n>…]` and `linkStyle default`, each with an
  optional `interpolate <curve>`, style edges by mermaid's link index (source order, fan-out
  pairs, `~~~` and subgraph links included; out-of-range and forward indices fail with
  mermaid's message). `stroke`, `stroke-width`, `stroke-dasharray`, `fill` and `color`
  (label text) are drawn; arrow, cross and circle markers get a variant in the edge colour,
  and a styled width uses a fixed-size arrowhead. A later `linkStyle` for an index
  replaces the earlier one; the default merges under it. `interpolate` sets the edge curve.
  `to_mermaid` writes one resolved `linkStyle` line per styled link, numbered in emitted
  order. `stroke-width` takes 0 to 1000px (negative, non-finite or larger widths are
  errors). New: `model::LinkStyle` and an `EdgeMeta::style` field (non-breaking:
  `EdgeMeta` is `#[non_exhaustive]`). Diagrams without `linkStyle` render byte-identically.
- **`accTitle` / `accDescr` accessibility statements.** `accTitle: …`, `accDescr: …` and
  the multi-line `accDescr { … }` block parse with mermaid's rules (value to end of line,
  `;` and `%%` included; case-sensitive keywords; last one wins; block indentation and
  blank lines folded). The title replaces the SVG's `<title>` / `aria-label`; the
  description becomes a `<desc>` right after it, referenced by `aria-describedby` with a
  content-hashed id. Flowchart, state, class, sequence and ER diagrams store and render
  them, and `to_mermaid` round-trips them; pie, journey, timeline, gitGraph and
  architecture accept the lines without drawing them yet (those models have all-public
  fields, so storing them would break struct literals). Diagrams without these statements
  render byte-identically. New: `acc_title` / `acc_descr` fields on `Graph`,
  `ClassDiagram`, `SequenceDiagram` and `ErDiagram` (non-breaking: each has a private
  field), `scene::A11y` and `scene::to_svg_a11y_with`.
- **Flowchart edge ids and edge metadata.** `A e1@--> B` names an edge on every operator
  form (inline and `|pipe|` labels, markers, dotted, thick, long, `~~~`); like mermaid's
  `${diagramId}-${id}`, the path gets `id="flowmaid-<hash>-e1" data-id="e1"`, where
  `flowmaid-<hash>` is a content-hashed (deterministic) id written on the root `<svg>`
  only when the diagram has edge ids or animation. `e1@{ … }` (one line or several, a
  YAML subset read without a new dependency) sets `animate` / `animation` (mermaid's
  `edge-animation-fast` / `-slow` class plus its dash-animation CSS, written once in
  `<defs>` and scoped to the SVG: `#flowmaid-<hash> .edge-animation-fast`, keyframes
  `flowmaid-<hash>-dash`, so an inlined SVG never restyles its host page or another
  diagram) and `curve` (all
  thirteen mermaid names; unknown = `basis`). Fan-out, duplicate-id and
  metadata-before-edge semantics follow mermaid; `class` / `style` on an edge id no longer
  mint a stray node. `to_mermaid` round-trips ids and metadata. New: `model::EdgeMeta`,
  `model::EdgeAnimation`, `model::EdgeCurve` (`from_mermaid`, `mermaid_name`), and a `meta`
  field on `Edge`, `SubEdge` and `SceneEdge`; `EdgeMeta` is `#[non_exhaustive]` like
  `Edge` / `SubEdge` (set it on an edge from `Graph::add_edge`: `g.edges[i].meta.id =
  Some(…)`). Diagrams without ids render byte-identically.
- **Mermaid v11 node metadata (`A@{ shape: cyl, label: "DB" }`).** Accepted after a node id
  (and after its bracket shape or `:::class`) in every statement position, on one line or
  several. All mermaid registry shape names parse: the 15 families with a flowmaid
  equivalent draw it exactly, the rest fall back to the nearest existing shape (`text`,
  braces and `bolt` draw borderless), and invalid names fail with mermaid's own `No such
  shape: X.` / `… Shape names should be lowercase.` messages. `label` replaces the label
  and, as in mermaid 12, is markdown (`**bold**`, `_italic_`, wrapping, like a
  ``"`…`"`` label) unless `labelType: string` / `text` keeps it plain; `icon`, `img`,
  `form`, `pos`, `w`, `h`, `constraint` are accepted and not drawn. A nested `{…}` value
  is reported as such (the block ends at its first `}`, as in mermaid), not as an
  "unclosed collection". `id@{ … }` on a subgraph id is validated and ignored. No public API change.
- **`flowchart-elk` header.** Accepted as an alias of `flowchart` with the same direction
  rules; the ELK layout hint is dropped (`to_mermaid` writes `flowchart`).

- **Flowchart edge markers (#59).** Cross ends (`--x`, `-.-x`, `==x`), circle ends
  (`--o`, `-.-o`, `==o`) and two-way links (`<-->`, `<-.->`, `<==>`, `x--x`, `o--o`) parse
  on every line style and length, with `|label|` and inline (`-- text --x`,
  `<-- text -->`) labels, render with the matching glyph at each end, and round-trip
  through `to_mermaid`. `--x` / `--o` no longer fail with "inline edge label … is never
  closed". New `model::Marker` / `model::EdgeEnds`, `Graph::add_edge_with_ends`, and an
  `ends` field on `Edge`, `SubEdge` and `SceneEdge`; `EdgeKind` is unchanged (a marker
  edge is the matching open kind, so consumers that only know `EdgeKind` draw a plain
  line).

- **Trapezoid and asymmetric bracket shapes.** `A[/text\]` (trapezoid), `A[\text/]`
  (inverted trapezoid) and `id>text]` (mermaid's asymmetric `odd`) parse, drawn as the
  same nearest look-alikes as their `@{ shape: … }` names (the `[/ /]` and `[\ \]`
  parallelograms and a rectangle). A slanted label now ends at whichever of `/]` and
  `\]` comes first, as in mermaid's lexer. No public API change.
- **Opt-in transparent background** ([#62](https://github.com/go-routine-id/flowmaid/issues/62)).
  `SvgOptions::transparent` omits the full-canvas white `<rect>` for every diagram type
  (`render_svg_advanced`, each `*_with` writer, plus new `render_advance_svg_with`,
  `render_advance_text_svg_with`, `render_advance_routed_with`,
  `render_advance_routed_with_lanes_with`, `fold::render_compact_with` and
  `fold::render_compact_titled_with`); CLI `--transparent`. Default output is unchanged
  byte-for-byte. Text and edge colours are unchanged (still dark, tuned for a light
  background), so transparent output can be hard to read on a dark page; theming is
  tracked in [#57](https://github.com/go-routine-id/flowmaid/issues/57).

### Changed

- **Breaking (minor bump required):** `SvgOptions` gained the public field `transparent`
  and is now `#[non_exhaustive]`. Outside the crate, struct literals no longer compile
  (not even with `..Default::default()`); build it with `SvgOptions::default()` and assign
  the fields you need. This makes future option fields non-breaking. Release this as
  0.31.0, not 0.30.x.
- A pipe edge label ``|"`x`"|`` is now a markdown string (label `x`), as in mermaid; it
  used to keep the backticks.
- **Mermaid's marker-letter rule applies.** A letter glued to the end of an operator is
  a marker, as in mermaid: `A---oB` is now a circle edge to `B` (was an open line to
  `oB`), and `A --xray` a cross edge to `ray`. Separate the id with a space to keep it.
  Code that builds `model::Edge` / `model::SubEdge` with struct literals must add
  `ends: Default::default()` and `meta: Default::default()`.
- **`model::Edge` and `model::SubEdge` are `#[non_exhaustive]`.** Fields stay public to
  read, but downstream code can no longer build them with struct literals (this release
  already breaks those by adding `ends` and `meta`), so future fields won't break it again. Build
  edges with `Graph::add_edge` / `Graph::add_edge_with_ends`; `SubEdge`s come from the
  parser.
- **Unquoted inline labels end at the first closer, as in mermaid.** Mermaid's lexer
  tries its closing rule (`[xo<]?--+[-xo>]`, `[xo<]?==+[=xo>]`, `[xo<]?-?.+-[xo>]?`)
  before every edge-text character, so edge text ends at the first match, even
  mid-word, and an `x` / `o` / `<` right before the closer belongs to it; flowmaid now
  does the same. `A -- go--on --> B` becomes a two-way circle edge `A o--o n` labelled
  `g` plus `n --> B` (was one edge labelled `go--on`); `A -- hello--x B` is a cross edge
  labelled `hell`; `A --xfer--> B` becomes `A --x fer --> B`; `A -. t -.-x B` and
  `A -. t ..-x B` are labelled `t` (were `t -` / `t .`); `A -- to--x --> B` and
  `A -- use --xml flag --> B` are now errors. Quote the label to keep it whole:
  `A -- "go--on" --> B`. Quoted text is never scanned for a closer, so
  `A -- "a --> b" --> B` and `A -. "a .-x" .-> B` now parse too.
- **Node `style` / `classDef` `stroke-width` must be 0 to 1000px.** Negative widths,
  `NaN` / `inf` and larger values were accepted and wrote invalid or 300-digit SVG
  numbers; they are now "invalid stroke-width" errors, the same check `linkStyle` uses.
  `to_mermaid` drops a programmatic width outside that range, as it already dropped
  non-finite ones.
- **Glyph ends sharing a node side are spread apart.** When an edge end on a box's side
  carries a start, cross or circle glyph, the ends on that side are spaced up to 14px
  apart so the glyphs stay readable. Sides without such glyphs render exactly as before.

### Fixed

- **Markdown emphasis is linear.** Matching `*` / `_` runs used to rescan every earlier
  run per closer, so a 128 KB label of `_*` took seconds; it now uses CommonMark's
  "openers bottom" bound and a linked delimiter list (256 KB: milliseconds).
- **`SceneNode::link` re-sanitises urls.** A `Graph` filled in through the API (not the
  parser) with a `javascript:` / `data:` url no longer hands it to host apps through the
  scene; the unsafe `href` is dropped and the rest of the link kept.
- **`classDef` / `style` accept a bare item after a property.** Mermaid splits style
  values on every comma, so `stroke-dasharray: 9,5` leaves a stray `5` that it keeps as a
  no-op; flowmaid now ignores it too, so the `classDef animate …` example from mermaid's
  flowchart docs parses. A statement that starts with a bare item is still an error.
- **`@{` in label text no longer opens a metadata block.** Inside a bracket shape label,
  a `|pipe|` label or an inline `-- text -->` label, `@{` is plain text, as in mermaid's
  lexer. It used to join the following lines into one statement and fail with "'@{'
  metadata block is never closed" (or swallow the next statement). Regression from the
  multi-line `@{ … }` support in this release cycle.
- **Accessibility statements inside multi-line labels and `@{ … }` blocks.** A line inside
  a multi-line markdown string or `@{ … }` block is label / YAML text, so `accTitle:` /
  `accDescr` there no longer sets the diagram's title or description (or eats the label
  line), and `…`"] --> B; accTitle: x` on the line that closes a markdown string works.
- **YAML block scalars in `@{ … }` follow js-yaml.** A `>` folded scalar keeps a
  more-indented line on its own line with its extra indent instead of folding it, and a
  `|` / `>` line indented less than the first content line is "bad indentation", as in
  mermaid, instead of being accepted.

- **Subgraph frames no longer overlap (#60).** Sibling subgraph boxes never intersect
  and nodes are drawn only inside their own frames: a box widens only for edge lanes laid
  out inside it, sibling clusters keep one left-to-right order on every rank with straight
  sides, and LR/RL layouts reserve room for each box's title strip.
- **Edges to a subgraph id are ranked (#60).** `A --> sub` (and state composites) now
  places the subgraph's contents below `A` instead of on `A`'s rank, so a TB chart of
  subgraphs no longer collapses into one row. Four or more edge-less members of a subgraph
  are packed into a grid, unless the subgraph sets a `direction` across the chart's flow.
- **Edges reach nodes at the top of a subgraph.** The title-strip room no longer inflates
  the node itself, so arrows end on the visible border instead of 26px short.

- **Edge labels no longer hide nodes or frame borders (#61).** A label too wide for the
  gap between its two ends grows that gap until it clears them (and, when an end sits in
  a subgraph, that frame's border); a label still covering a node, a frame border line or
  an earlier label slides along its own edge to the nearest clear spot, and a self-loop's
  label steps out past its loop. In TD/BT a fan of labels sharing one gap gets a row each;
  in LR/RL the gap only grows to its widest label, since the labels separate across the
  flow. Labels that cover nothing keep their exact spot.
- **Arrows, em dashes and emoji are measured at about 1em (#61).** Text containing `→`,
  `—` or symbols from the arrows / misc-symbols / dingbats blocks was measured as narrow
  Latin letters, so node boxes clipped it. The measure is shared by every diagram type,
  so sequence diagrams whose messages contain these glyphs come out a few pixels wider
  too.
- **Subgraph titles are not clipped by the canvas edge.** A title running past the
  canvas is cut with `…` to fit its frame, like a title running into a sibling frame.
- **`<br/>` in a subgraph title draws a new line, as in mermaid.** Titles used to be
  joined into one line and then cut with `…`, losing most of it. Each line now gets its
  own row (the frame's header grows to fit, in every direction); only a line still too
  wide for its frame is cut, and a cut drops a dangling separator (` ·`, `-`, `,`, `:`)
  before the `…`. `model::Subgraph::title` now carries `\n` for each `<br/>`, and
  `to_mermaid` writes it back as `<br/>`.
- **An edge between a member and its own subgraph stays off the members.** `C --> G`
  (or `G --> C`) where `C` sits inside `G` is drawn as a short stub from `C` to the
  nearer frame side along the flow, its label just outside the frame. In LR/RL the curve
  used to cross `C` with its label on top of it; in TB/BT it doubled back over the node.
- **Colliding edge labels are placed in near-linear time.** Obstacles are bucketed on a
  grid and spots along an edge are sorted lazily, so a 2000-edge labelled fan renders in
  about 0.2s instead of 7s. Placement is unchanged.

Layout changes are confined to diagrams with subgraphs, edge labels that collided with a
node, label or frame border, and text containing the re-measured glyphs; every other
diagram renders byte-identically.

||||||| parent of 6481359 (fix(advance): make the crossing guarantee survive the diagram it is in)
## [0.30.0] - 2026-09-07

### Fixed

- **Dragged node positions no longer swap nodes between lanes.** `render_advance_routed`
  and `render_advance_routed_with_lanes` consumed `positions` in declaration order while
  documenting — and while `layout_advance` emits — lane order. A host that read the scene,
  moved one node and re-rendered teleported the others. Positions are now mapped through
  the layout's own order, so they cannot drift from the array the caller was handed.
- **Lane boxes are sized by their own nodes when lanes nest.** `build_lanes_with_widths`
  matched a flat depth-first lane index against an enumeration of top-level lanes, so with
  `[A[A1, A2], B]` the nodes in `A1` counted toward `B` and `B` was drawn as if empty.
- **The visible node's anchor is reachable again.** `hit_test` took the globally nearest
  anchor and discarded it when it belonged to another node, leaving the top node's own
  anchor unpickable whenever an occluded one sat slightly nearer; the search is now scoped
  to the node under the point.
- **An edge separator inside a label is text.** `a[A] { p[x---y] }` failed with
  `edge references unknown node 'a[A] { p[x'`; any label containing `---`, `-->`, `==>` or
  `-.->` was unusable. The edge parser and every guard now share one bracket- and
  quote-aware scanner, and separator precedence is unchanged.
- **A one-line lane block accepts bare ids.** `lane l "L" { a }` was swallowed whole —
  title `L" { a }`, no nodes, no error — because the node-shape disambiguation guard ran
  before the lane branch.
- An anchor id may not contain `:`; `parse_end` refuses it in an `@`-reference, so such an
  anchor could be declared but never used.
- A `style from-->to` target matching no edge is an error for plain node pairs too, not
  only for terminal references.

### Fixed

- **Advance arrow-marker ids no longer collide between diagrams, and no longer change
  between runs.** The id came from a process-global counter, so two advance SVGs
  rendered separately and inlined on one page both emitted `advance-arrow-1` and every
  arrow in the second took the first one's colour — the counter gives uniqueness only
  *within* one process. It also meant the same diagram rendered differently each time,
  against the crate's byte-identical output guarantee. Ids now carry a hash of the
  scene, which gives both: stable per diagram, distinct between diagrams.
- The hashing helper is shared with sankey's gradient ids (`scene::DefsKey`) instead of
  being written twice.

### Added

- **Advance terminals** — an edge end can now reference a *sub-element* or a *named
  anchor*, not only a node side: `node.element@anchor`, `node@anchor`, `node.element`,
  composably to any depth (`.` descends, `@` names an anchor, `:` picks a side).
  Text DSL: a node opens a `{ ... }` block holding `anchor <id> <side> [offset]`,
  `layout column|row`, and sub-element declarations (nestable, and writable on one
  line: `core1[Core 1] { anchor irq right }`). JSON: nodes take `anchors`, `elements`
  and `layout`; `from`/`to` accept the reference grammar.
  - Sub-elements are laid out without coordinates — stacked compartments or a row —
    and the node grows to fit (an explicit `w`/`h` is clamped to that minimum). A
    sub-element side may be used only if it reaches the node boundary — never `top`,
    where the label band sits; an interior side is a parse error, not a lead that
    pierces a sibling or the label.
  - The scene carries each node's placed `elements` (flat, `parent`-linked) and
    resolved `anchors`, and each edge's `from_point` / `to_point` and `from_end` /
    `to_end`; `scene_to_json` emits them. `AdvanceScene` gains `anchor_at` and
    `element_at`, and `AdvanceHit` gains `Element` and `Anchor` variants.
  - `style a.x@p-->b.y` styles exactly that terminal edge (and errors when it matches
    none); a plain `style a-->b` still styles every edge between the two nodes.
  - Sub-elements need a `rect` or `rounded` node; any other shape is refused at parse
    time — inside a diamond, six of eight compartment corners fell outside the outline
    and could not be picked.
  - A lane block may be written on one line, like a node block. `lane l "L" { a[A] }`
    used to swallow the title and the node without a word.
  - An id ending in a side keyword (`a:right`) is refused: every edge would read it as
    node `a` on its right side, so it could never be referenced. `a:b` is still fine.
  - Text and JSON accept the same sub-element nesting depth (16); the multi-line block
    form was one short.
- `examples/advance_terminals.mmd` and a README section; the approved design is in
  `docs/design/advance-terminals-and-router.md`.

### Fixed

- A ported edge no longer runs back through its own source or target, nor through a
  third node. With `d:right --> b:top` and `b` to the left, the channel used to sit at
  centre height and cut across both nodes; it now moves to the nearest clear gap —
  above, below, or between the two nodes — and every node counts as an obstacle.
  Parallel ported edges keep their fan. Ported edges that were already clear are
  byte-identical.
- **Edges attach to the node they name.** `route_edges` indexed scene nodes in
  declaration order while layout emits them in lane order, so with lanes `A, B` and
  nodes `n1@A, n2@B, n3@A` the edge `n1 --> n2` was drawn to **n3**. Scene nodes are
  now looked up by id.
- Edge labels move with their routes when the canvas is shifted to fit — `label_pos`
  was the one coordinate `fit_canvas` left behind, so a label could sit 100 px off its
  edge for explicit-coordinate and dragged diagrams.

### Changed

- **Ids may no longer contain `.` or `@`** — they are reserved by the reference
  grammar. A diagram that used them (`a.b[X]`) parsed before and now gets a
  line-numbered error naming the offender.
- `AdvanceHit` has two new variants (`Element`, `Anchor`); an exhaustive `match` on it
  needs two more arms. `AdvanceNode`, `AdvanceEdge`, `AdvanceSceneNode` and
  `AdvanceSceneEdge` gain fields; struct-literal construction outside the crate needs
  them. Every existing text or JSON input still parses identically.

## [0.29.2] - 2026-09-01

### Fixed

- `swimlane-beta` is recognised and refused explicitly. Mermaid added the type in
  v11.16, but flowmaid's catalogue recorded the token as `swimlanes`, so the real
  header was never registered and fell through to the flowchart parser — which
  reported `unknown edge operator near: '-beta'`. The README's standing claim of an
  explicit error for every known mermaid header holds again.

## [0.29.1] - 2026-08-31

### Fixed

- **Style values can no longer close the SVG attribute they are written into.**
  A `fill`, `stroke`, `color`, `dash`, or `label-fill` value carrying a quote could
  end its attribute and add markup of its own, so rendering an untrusted diagram
  could inject content into the SVG. Both quote forms mattered: `scene.rs` emits
  attributes with double quotes and `architecture.rs` with single ones.

  Reachable from a diagram's own text through flowchart `style` / `classDef`, the
  advance text DSL, the advance JSON node/edge `style` objects, and the advance
  diagram-level `style` block (`lane_fill`, `lane_stroke`, `text_color`,
  `edge_color`, `label_fill`). `gitGraph` and `architecture-beta` fill their node
  styles from the palette, so only an embedder driving the scene API can put text
  in those — they are covered for the same reason.

  Control characters are now dropped rather than escaped. XML 1.0 forbids them
  even as a numeric reference, so a single NUL in a colour or a label used to make
  the whole document unparseable.

  Colours are escaped at the render site rather than rejected at the parser.
  `Graph` and `NodeStyle` are public with public fields and the crate advertises
  the scene API for embedders, so a check in the parser would leave the
  programmatic path unprotected — and rejecting quotes would also turn away valid
  CSS such as `fill:url('#grad')` and break the `to_mermaid` round trip. The
  render site is the only boundary every colour actually crosses.

### Fixed

- **Sankey gradient ids are namespaced per diagram.** Ids were `fmsk0..N`, derived
  from the link index alone, so two sankey SVGs inlined on one page — which the docs
  site does — made every ribbon in the second resolve to the first's gradient, taking
  its stop colours *and* its `userSpaceOnUse` geometry. The id now carries a hash of
  the scene, which keeps output byte-identical across runs while making a collision
  between different diagrams effectively impossible.
- **Sankey config numbers must be finite and non-negative.** `f64::from_str` accepts
  `NaN` and `inf`, and they reached the SVG as `x="NaN"` or `viewBox="0 0 600 inf"`.
  `parse_sankey` already refused a non-finite *link* value; config numbers now get the
  same guard and fall back to their default.
- **An overflowing sum no longer produces infinite geometry.** Every link value can be
  finite while their sum reaches `+inf`, which became `height="inf"`. Sums are
  saturated, so the diagram draws — squashed — instead of emitting unparseable SVG.
- **Computed totals no longer leak float noise into labels.** A node total is a sum, so
  `0.1 + 0.2` rendered as `A (0.30000000000000004)`; it now reads `A (0.3)`.
- **`linkColor` and `nodeAlignment` ignore case**, like the YAML booleans beside them.
  `linkColor: Gradient` used to fall through to a CSS colour named "Gradient" and paint
  the ribbons black; `nodeAlignment: Left` silently laid out as `justify`. A value that
  is a real colour keeps its spelling, since CSS `var(--Name)` is case-sensitive.
- **`config.sankey` is read only as a direct child of `config`.** A `sankey` key nested
  under another namespace (`config.themeVariables.sankey`) silently reconfigured the
  diagram.
- The diagram-type count in `README.md` and `docs/index.html` was stale (eleven and ten
  against twelve shipped).

## [0.29.0] - 2026-08-31

### Added

- **`sankey-beta` diagram** — mermaid's CSV grammar (`source,target,value`, exactly
  three fields), double-quoted fields with `""` as one literal quote, `%%` comments,
  and `sankey` accepted as a spelling variant. Nodes are created on first mention and
  keyed by label, as in mermaid.
  - Layout follows d3-sankey: a node sits at its **longest** path from a source so
    every ribbon points rightwards, its thickness is the larger of inflow and outflow,
    and nodes are ordered within a column by neighbour barycentre to reduce crossings
    (ties keep declaration order, so the SVG stays a stable function of the input).
  - Ribbons are cubic-bezier bands; gradient ids come from the link index, never a
    counter, so output stays byte-identical across runs.
- **`config.sankey.*` read from YAML frontmatter** — `width`, `height`, `linkColor`
  (`gradient` default, plus `source` / `target` / any CSS colour), `nodeAlignment`,
  `showValues`, `prefix`, `suffix`, `nodeWidth`, `nodePadding`. Every default matches
  mermaid's `config.schema.yaml`. Unknown keys are skipped and an unparsable value
  keeps its default, so config for other diagram types never breaks a render.
  Frontmatter was previously accepted and thrown away.
- `examples/sankey.mmd`, a README section, and a docs-site row.
- All four `nodeAlignment` modes behave as d3-sankey defines them: `left` keeps a
  node's own depth, `justify` (default) flushes sinks right, `right` pushes each
  node as far right as its distance to a sink allows, and `center` pulls a
  source-less node up against its earliest target.

### Fixed

- **Colour values are escaped before they reach an SVG attribute.** An unescaped
  `linkColor` could close the `fill="..."` attribute and add markup of its own,
  so a diagram rendered from untrusted `.mmd` could inject content into the SVG.
- A column whose nodes all have value 0 no longer pins the scale for the whole
  diagram — it now constrains nothing, instead of capping every other column at
  1 px per unit.
- A cycle no longer inflates the column count. Depth is computed over a
  topological order with the loop-closing edges left out, which bounds it at
  `n - 1`; blind relaxation used to add a column per pass and shove every node
  against the right edge with ribbons pointing backwards.
- A column taller than the configured canvas grows the canvas instead of drawing
  nodes past the bottom edge.
- A node is now at least as tall as the ribbons it carries. Node heights and
  ribbon thicknesses were clamped to a minimum independently, so a node with many
  tiny flows had ribbons hanging outside the rectangle they left.
- `showValues` understands the YAML 1.1 booleans (`False`, `no`, `off`, `0`, ...);
  only the exact lowercase `false` used to switch it off.
- A quoted config value followed by a `#` comment no longer keeps its quotes, and
  a `#` inside the quotes stays part of the value.
- Characters between a closing `"` and the next comma in a sankey row are an error
  rather than silently discarded, so a typo cannot quietly re-target a flow.

## [0.28.1] - 2026-08-31

### Added

- **`timeline` diagram** — `title` / `section`, `period : event[: event...]` rows,
  `:`-continuation lines, `LR` / `TD` direction, `%%` and `#` comments, and `<br>`
  folding. `LR` draws sections side by side on one shared axis, `TD` stacks them.
- The rest of the Mermaid catalogue is now *recognised* — `requirementDiagram`,
  `quadrantChart`, `sankey` / `sankey-beta`, `xychart-beta`, `block-beta`,
  `packet-beta`, `radar-beta`, `treemap` / `treemap-beta`, `kanban`, `zenuml`, and
  the `C4*` family — so they fail with an explicit "not supported yet" instead of
  being parsed as a flowchart. Previously `quadrantChart`, `C4Context`, `kanban`
  and `radar` rendered a nonsense SVG and exited 0, while `sankey-beta` failed with
  the misleading `unknown edge operator near: '-beta'`.
- `architecture` is accepted as a spelling variant of `architecture-beta`. The
  dispatch already declared the alias, but the header was never recognised, so a
  bare `architecture` diagram fell through to the flowchart parser.
- Example fixture `examples/mindmap.mmd` — the last diagram type without one.

### Fixed

- The flowchart-only `parse()` no longer denies a diagram type the crate can
  actually parse. `mindmap`, `journey`, `gitGraph`, `architecture-beta`, and
  `timeline` now point at `parse_document()` like the other types, instead of
  answering with a self-contradictory `not supported yet (supported: ... mindmap ...)`.
- `flowmaid --help` advertised the supported headers without `timeline`.
- The crate description and `render_svg`'s doc comment likewise omitted `timeline`.

### Changed

- Recognised diagram headers live in one table (`DIAGRAM_HEADERS`) that drives
  header matching, the "supported: ..." list quoted by both parser entry points,
  and the CLI `--help`. Five hand-maintained lists had to agree before; three of
  them had already drifted.
- `parser::supported_types()` is public, so embedders and the CLI quote the list
  the dispatch actually implements.

## [0.28.0] - 2026-08-30

### Added

- **Advance text DSL quality upgrade** (`render_advance_text_svg`):
  - Brace-scoped **nested lanes** — `lane id "Title" { ... }` opens a child scope;
    nodes attach to the innermost open lane, a standalone `}` closes it.
  - `config` directive — `margin`, `lane_gap`, `node_gap_y`, `lane_pad_x/y`,
    `lane_title_h`, and `order declaration|topology`.
  - `<br/>` line breaks in node labels, edge labels (`-->|a<br/>b|`), and lane titles.
  - Mermaid-style styling — `classDef`, `class a,b name`, `id::class` shorthand,
    and `style` lines targeting nodes (`style a fill:#fee`) or edges
    (`style a-->b color:#188038,dash:4 2`).
  - **Ported edges** — `a:right --> b:top` pins the anchor sides with orthogonal
    routing; edges without sides keep the automatic routing byte-identical.
  - **Edge-label collision avoidance** — labels dodge other nodes and already-placed
    labels; untouched labels stay byte-identical.
- **Per-element styling in JSON** — nodes/edges accept a `style` object
  (`fill`/`stroke`/`stroke-width`/`color`; edges add `dash`/`label-fill`), and edges
  accept `from_side`/`to_side` (`left|right|top|bottom|auto`).
- **Per-color arrow markers** — one `<marker>` per resolved edge color (first-seen,
  deduplicated), so a styled edge's arrowhead matches its stroke.
- **Scene hit-testing** — `AdvanceHit` and `AdvanceScene::{hit_test, node_at,
  edge_at, lane_at, nearest_node}` for drag-and-drop picking (shape-precise for
  diamonds and circles).
- **CLI** — `flowmaid --advance-text <file.mmd>` renders the text DSL.
- **Docs & examples** — README section, docs site entry, fixture
  `examples/advance_swimlane.mmd`, and example `examples/advance_text.rs`.

### Changed

- Version bumped to 0.28.0 (minor — all additions are backward-compatible).

## [0.27.0] - 2026-08-30

### Added

- Nested horizontal lanes, corridor routing, and a text DSL for the advance /
  swimlane engine.

## [0.26.2] - 2026-08-30

### Added

- Advance swimlane engine enhancements.

## [0.26.1] - 2026-08-30

### Fixed

- Drag-positioned advance content no longer clips out of the canvas.

### Docs

- Advance swimlane module documented on the docs site.

## [0.26.0] - 2026-08-30

### Added

- Advance swimlane diagram module with JSON input (`render_advance_svg`,
  `layout_advance`, `render_advance_routed*`).

### Docs

- Diagram-type counts synced.

## [0.25.0] - 2026-08-30

### Added

- Opt-in responsive SVG viewport via `render_svg_advanced`.

### Fixed

- Mindmap sibling overlap via footprint-aware wedges.

## [0.24.1] - 2026-08-30

### Fixed

- Per-end arrowheads for architecture edges.

## [0.24.0] - 2026-08-30

### Added

- `architecture-beta` polish: icons, bidirectional arrows, group qualifiers, and
  centered layout.
