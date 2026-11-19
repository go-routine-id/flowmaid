<!-- Work package A1 (classDiagram). Merge these bullets into `## [Unreleased]`
     under the matching Keep-a-Changelog headings. -->

### Added

- **Class notes are drawn.** `note "text"` and `note for Class "text"` draw as
  yellow cards (`style::NOTE_STYLE`); before, they were silently dropped.
  An attached note is linked to its class by a dotted line. `\n` and `<br/>`
  break lines, as in mermaid's docs (mermaid 12 itself prints `\n` as text).
  Each card is a scene node after the classes, so hosts pick and drag it
  like a box (`ClassScene::notes[k]` describes `scene.nodes[boxes.len() + k]`).
  Each connector is a `DottedOpen` scene edge after the relationships
  (`NoteBox::edge`). `class::route` accepts class centres alone, which puts
  each note 40px before its class, or class + note centres, which keeps
  notes where you put them. New: `model::ClassNote`,
  `ClassDiagram::notes`, `class::NoteBox`. A diagram of notes alone
  draws them (the CLI used to stop with "no classes defined"), and
  `note for` a class the diagram never declares leaves the note free
  instead of creating the class — both as in mermaid.
- **Stereotypes.** `<<interface>>` (or any `<<x>>`) inside a class block, or
  on its own as `<<interface>> Name`, draws as `«interface»` above the class
  name. The standalone form used to be a parse error, and the block form was
  skipped. New: `Class::annotations`, `ClassBox::annotations`,
  `ClassBox::header_h()`, `class::ANNOTATION_H`.
- **`direction TB|BT|LR|RL` in class diagrams** is honoured (it used to be
  ignored, and the layout was always top-down). An unknown direction is
  still ignored, as in mermaid. New: `ClassDiagram::direction`.
- **Generic classes.** `class Box~T~` is one class `Box` shown as `Box<T>`,
  so a later `Box : +size() int` adds to it instead of making a second box.
  Relation operands may carry the generic too. Member generics read
  `List<int>` by mermaid's own pairing rule. New: `Class::generic`,
  `Class::display_name()`.
- **Static and abstract members.** A `$` or `*` classifier (after a method's
  `)` or at the end of the member) underlines or italicises the row instead
  of printing the character. New: `ClassRow::is_static` and
  `ClassRow::is_abstract`.
- **Host helpers that match the SVG:** `class::rel_head(cs, j)` returns
  relationship `j`'s end glyph as drawn, and `class::card_spots(cs)` returns
  where each cardinality sits. Also new: `class::compartment_h`,
  `class::COMP_PAD`, `ClassRel::new` and `ClassRel::source_order`.

### Changed

- **Class hierarchies read top-down, like mermaid and UML.** Relations are
  laid out in source order (left operand first), so in `Animal <|-- Dog`
  the parent sits above the child. Before, parents, interfaces and wholes
  were drawn below their children. That inverted order also caused a false
  cycle, a giant back-edge arc (`charges` in the showcase) and a crossing,
  which are gone too. Scene edges still run `from` → `to`, with the glyph at
  the end, so hosts paint them as before. `ClassRel::head_left` records the
  source order.
- **Class edges are orthogonal with rounded corners, as in mermaid.** Each
  edge goes straight down when it can, takes a single jog otherwise, and
  runs a lane around the boxes for a cycle (or, after a drag, around boxes
  that end up in its way). Ends spread along the faces, away from the
  corners, and meet them square. An orthogonal route never lies along
  another edge's line: when no lane is free, the edge keeps its curve.
  Jogs that share a gap are stacked again in nesting order once all are
  drawn, and the rank gap grows to hold a wide fan's nested jogs, about
  14px apart. A fan too wide for its parent's face sends its outermost
  legs to the side faces. So a class with up to about a dozen subclasses,
  or five labelled relations into one class, draws without a crossing, as
  in mermaid. Crowded fan-ins (six or more classes pointing at one) can
  still cross a few times, and much wider fans (twenty or more
  subclasses) many times: about 18 crossings for 45 subclasses, where a
  leg with no free lane keeps a diagonal curve across the others. An end whose spread place sits a few px beside another route's
  lane moves just clear of it instead of leaving its edge on a curve. A
  label stays clear of the other routes' corners, and a label a later jog
  runs through moves along its own line. A routed edge that would enter
  its end glyph on a stub shorter than the glyph is routed again. Routes
  come as router-style waypoints, so a host's spline draws the same path.
- **Class layout follows dagre where it reads better.** A class
  hierarchy's children stay in one row under their parent (leaf
  staggering, which interleaved the two rows' legs on the parent's face,
  is off for class diagrams). A source ranks just above its nearest
  target instead of on the first rank, so a `note for` sits just above
  its class instead of in the top row with a long connector, and a class
  pointing at one deep in the diagram sits next to it.
- **Class end glyphs and labels.** Glyphs follow the drawn line and are
  painted over the boxes, so a head is never hidden under its box. Ends
  sharing a face spread by their glyphs' widths and keep room for each
  cardinality on the side it takes, so triangles, diamonds and
  cardinalities sit apart instead of overlapping. A box whose face must
  hold many ends grows along it (wider in TB/BT, taller in LR/RL).
  Cardinalities sit beside their end, past the glyph, clear of lines,
  glyphs, boxes, labels and each other, in every direction and on a
  crowded face, nearer their own end than any other. A diagram with
  labels or cardinalities gets an 84px rank gap. In LR/RL, a gap with a
  label also grows to fit the glyph and cardinalities at each end.
- **Class self-relations.** A class with two or more self-relations draws
  them on alternate sides, and loops sharing a side nest, their end
  glyphs apart, the loops with cardinalities outermost so each
  cardinality sits by its own end. Before, the layout stacked them on one side, each
  starting where the last ended, so the loops wound round each other.
  Loops whose side would run into another box (a sibling close beside
  the class) or over another relation's end move to a clear face. A
  loop's label sits past its loop, the labels of one side in a row (a
  lone loop's label used to sit on the loop, hiding most of it on a
  small box), and the automatic layout's canvas grows to hold a loop
  moved to the left or top.
- **Class members read as mermaid shows them:** `+total() : Money`, with
  `" : "` before the return type (it used to be dropped).
- **Class boxes:** the name band is rounded only at the top, dividers use
  the class accent instead of a pale hairline, and the border is drawn on
  top of everything. Compartments are padded (`COMP_PAD` 4px), and
  `class::EMPTY_H` grows from 8 to 12px, so empty compartments read as
  compartments.
- **Class `note` lines are checked as mermaid checks them.** `note` without
  text, and `note for` without a class name, are parse errors now. Before,
  flowmaid skipped every `note` line silently. Mermaid rejects both.
- **Breaking** (0.33.0 is out, so the release carrying this needs a
  minor bump): `model::Class`,
  `model::ClassRel`, `class::ClassBox`, `class::ClassRow` and
  `class::ClassScene` gain public fields and become `#[non_exhaustive]`. Use
  `ClassDiagram::ensure_class` and `ClassRel::new` instead of struct
  literals.
- **Breaking for class-scene hosts:** with notes, `ClassScene::scene` has
  more nodes than `boxes` and more edges than `rels`. Notes and their
  connectors come after the classes and relationships. A host that indexes
  `cs.boxes[i]` or `cs.rels[j]` for every scene node or edge must stop at
  `boxes.len()` / `rels.len()`, or zip the lists. A host that paints no
  notes can clear `ClassDiagram::notes` before `class::scene` /
  `class::route` and get the 0.32 shape.

### Fixed

- Rerouted class relationships used to be drawn as a straight line through
  the boxes they had been routed around, with their inheritance triangles
  collapsed to nothing. The 'contains' label sat 18.5px off its line. The
  class writer now draws the route through the shared `scene::edge_path_d`.
