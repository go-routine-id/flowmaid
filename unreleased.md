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
  `model::ClassNote`, `ClassDiagram::notes`, `class::NoteBox`. A diagram of
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
    `NOTE_STYLE`, id `__note_<k>` unless the source names it) and a dotted
    edge joining it to its state. A host that treats every node as a state
    should skip those cards.
  - **Public constants with a new value or meaning:** `class::EMPTY_H` is
    12 (was 8). `class::HEADER_H` (28) is now the name band without
    stereotype lines; `ClassBox::header_h()` adds `ANNOTATION_H` per line.
    A member compartment is `class::compartment_h(rows)` tall, `rows ×
    ROW_H` plus `COMP_PAD` above and below.

### Fixed

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
