
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
- **Two-way class relations and lollipop interfaces**, which were parse
  errors. A glyph on both ends of a relation draws one at each end
  (`Animal <|--|> Zebra`, `A *--* B`, `A o--o B`, `A <--> B`, `A <..> B`,
  `A <|..|> B`, mixed ends such as `A *--> B`); `bar ()-- foo` and
  `foo --() bar` draw the interface `bar` as its name alone, with a small
  circle where its line meets it. As in mermaid 12.2.0, each lollipop line
  makes its own interface (separate from a class of the same name), a
  lollipop facing another glyph (`A ()--* B`) joins two classes, and both
  kinds rank in source order, the left operand first. Any end glyph also
  goes on a dotted line now (`A *.. B`, `A ..o B` were parse errors).
  Cardinalities, labels, `route` and dragging work as on any relation. New:
  `ClassRel::from_kind` and `RelStyle::from_kind` (the `from` end's glyph),
  `RelKind::Lollipop`, `Class::lollipop` with `ClassDiagram::add_lollipop`,
  `ClassBox::lollipop`, `Head::circle` and `class::rel_tail(cs, j)` (the
  `from` glyph as drawn, like `rel_head`).
- **`layout::CHOICE_SIZE`** (28px), the side of an unlabelled diamond,
  public like `BASE_H` / `MIN_W` so embedders can reproduce node sizes.
- **Per-file benchmark.** `cargo run --release --example bench -- files
  [--runs N] <paths...>` times `render_svg` (parse + layout + SVG) on real
  `.mmd` files and prints TSV: name, diagram type, median and min ms of `N`
  runs (default 30, after 3 warm-ups) and SVG bytes, sorted by path.
- **The quality yardstick sees the defects it used to miss.** New
  `quality::Metrics::tail_fields`, printed by `examples/quality.rs` after
  the timing columns (every earlier column keeps its number and value):
  `end_tilt` / `end_tilt_max` (end glyphs whose drawn heading is more than
  10° off square to the side they land on, the radius on round nodes),
  `end_gaps` / `end_gap_max` (edge ends more than 1px off the outline
  their node is painted with: slanted, capped and rounded sides measured
  as drawn, not as their box), `lat_wiggles` / `lat_backtrack` (hooks,
  hairpins and dips across the flow, which `wiggles` only counts along
  it; a back-edge may overshoot once to get round), `own_frame_x` /
  `own_frame_len` (edges leaving the innermost frame holding both their
  ends), `band_pairs` / `band_len` (edges side by side 6–26px apart for
  more than 100px), `ink` (the painted share of the canvas), the idiom
  counts `idiom_ortho` / `idiom_scubic` / `idiom_spline` / `idiom_bow`
  (how each edge is drawn: orthogonal, one S-cubic, a waypoint spline, a
  bow) and `x_drawn` (every drawn crossing: `edge_edge_x` keeps its pair
  prune, which skips an exactly level or plumb edge — 8 crossings of
  `dense-td` static). `tests/quality.rs` gates them at today's fixture
  values (`DEFECT_BASELINE`, `INK_MEAN_MIN`), to be lowered as the
  defects are fixed. No rendering change.
- **`classDef default …` styles every flowchart node**, as in mermaid
  (report Q9). It used to be an ordinary class nobody carried, so the
  standard way to give a diagram a neutral look did nothing. It is now the
  bottom layer of every node's style: a node's own classes (`class`,
  `:::`) and `style` lines still win property by property, as mermaid
  compiles `default`, then the node's classes. A borderless shape (`text`,
  braces) takes only its text colour and keeps no box.
- **`style::cluster_style(depth)`, `style::CLUSTER_STYLES` and
  `style::CLUSTER_TITLE`**: the subgraph frame colours the SVG uses, for
  hosts that paint frames from the scene (`SceneCluster::depth`).
- **Colour themes** (report M10). flowmaid's semantic palette stays the
  default, byte for byte; mermaid's looks are options:
  `style::Theme::{Flowmaid, Mermaid, Neutral, Dark, Forest}` — mermaid's
  `default` (lavender `#ececff` nodes, `#9370db` borders, `#333333` lines),
  `neutral` (white nodes in one `#333333` ink), `dark` (light ink on a
  `#1e1f22` canvas) and `forest` (green nodes). Every diagram type takes
  them: flowchart and state nodes, frames, edges, heads, labels and notes;
  class boxes, ER tables and sequence participants in the theme's one node
  paint (as mermaid paints them) instead of an accent each; and the charts
  (pie, journey, timeline, git graph, sankey, mindmap, architecture) keep
  their categorical colours while their text, lines and canvas follow the
  theme. Every theme keeps text at 4.5:1 and lines and borders at 3:1 on
  what they are drawn over — the dark one included, where mermaid's own
  frame borders and edge-label text fall short; mermaid's `#aaaa33` frame
  and note borders are darkened, hue kept, to reach it. Frames still step
  a shade per nesting level. `style`, `classDef` and `linkStyle` colours
  win over any theme; a `style` fill without a `color` gets text that reads
  on it when a theme's ink would not (a pale highlight under the dark
  theme's light text). New: `SvgOptions::theme` (`Option<Theme>`),
  `style::Palette` (every colour a theme paints with, for hosts that paint
  a scene: `SvgOptions::palette()`), `Theme::from_name` / `name` / `ALL` /
  `palette`, `style::readable_on`. The advance writer takes an explicit
  theme too: lanes as its outermost frame, edges, text and chips in its
  colours, unless the diagram sets its own.
- **The theme a diagram names is honoured** (report P1: "theme from
  init/frontmatter ignored"). `%%{init: {"theme": "dark"}}%%` (also
  `%%{initialize: …}%%`, single quotes or bare words, over one line or
  several) and `theme:` under a YAML frontmatter's `config:` pick the
  theme, a directive winning over the frontmatter as in mermaid. Their
  `themeVariables` `background`, `primaryColor`, `primaryBorderColor`,
  `primaryTextColor`, `lineColor` and `textColor` recolour the matching
  roles (a `primaryColor` without a text colour gets text that reads on
  it). An unknown theme name is the default theme, without error, as in
  mermaid; `base` reads as `default`, `neo` as `neutral` and `neo-dark` as
  `dark`. An explicit `SvgOptions::theme` beats the source — its theme and
  its variables alike; `Some(Theme::Flowmaid)` renders a themed source in
  flowmaid's own colours. `render_svg` / `render_svg_advanced` read the
  source themselves; `SvgOptions::with_source_theme(source)` does it for
  callers that parse first and render through `render::*_with`.
- **`flowmaid --theme <name>`** (`flowmaid`, `default`, `neutral`, `dark`,
  `forest`) picks a theme for every input mode, beating one the source
  names; an unknown name warns and renders the default theme. Without the
  flag the CLI now honours the source's theme.

### Changed

- **Labelled gaps keep their labels off the arrowhead; ends meet their
  sides in the middle** (report M3, §2.4, §2.6). In a flowchart or state
  diagram a gap between adjacent layers that carries an edge label grows
  to hold the label, an arrowhead and a stub either side of it: 72 px
  down the page for a one-line label (the 60 px gap used to stay, a
  20 px chip swallowing a third of it and the edge reading as two hooks),
  and along LR / RL the label's width, the head (both heads of a `<-->`
  edge, whose label otherwise stepped above its line) and a clearance
  either side (the head used to be left out); there a label on its line sits
  centred on the run clear of its arrowhead, so the clearance falls at
  both ends of its chip (at the curve's middle the chip stopped 3-4 px
  short of the head, which seemed to grow out of it). So does the gap
  between two composite states (or subgraphs) stacked one after the
  other when a labelled transition between them is drawn from border to
  border: such a gap stayed about 18 px whenever the transition's
  stand-in ran on to members deeper inside a box, and its chip covered
  the whole edge and its arrowhead. A styled edge's
  LR / RL gap (dotted, thick, animated, or `linkStyle`-painted with a
  colour, a width other than the plain 2 px, or a dash; `linkStyle
  default stroke-width:2px` draws plain and leaves a plain gap) also
  keeps a run of its stroke in sight either
  side of the chip: 15 px, or two periods of a longer dash (the chip
  used to leave about 6 px, one dash of a dotted edge, and a thick or
  coloured edge read as plain); where a drag brings the ends nearer than
  that, the label steps beside its line instead of hiding the stroke. A
  plain edge's gap is unchanged. On a straight line across the page a
  head keeps a label only its own length (and 3 px) away: a `curve:
  linear` or `step` edge's label no longer steps beside its line (the
  head's room took the whole line), and `route()` keeps such a label on
  its line where `scene()` does (its straight curve's sparse samples
  near the ends made the head 21 px long, so a 1 px drag moved the
  label 20 px); class and ER relations, whose painters draw their own
  end glyphs, keep their labels' distance from those glyphs as before
  (their output is unchanged). A decision's branch label sits in
  the middle of its curve, where the S runs straight, instead of in the
  half that turns into its target. So does a hub's (a node with more
  ends) where the drawing runs down the page: the gap grows (by up to
  48 px) until the label, centred on its curve's middle, clears the
  fan's other curves (a hub's `express` sat on the bend 6 px off its
  head, its middle crossed by a sibling's curve); a fan too crowded for
  that, or with parallel twins, keeps its gap and its labels keep to the
  half away from the hub. A label the first placement pass moved aside
  (it runs before a fan's ends are spread) returns to its curve's middle
  once that spot is clear: a decision fan's `thick` stayed 3 px off its
  head after its sibling had moved 30 px away. An edge end alone on its box side
  meets that side in the middle (long-labels' arrivals landed up to
  44 px off their targets' centres, and the flow through a node jogged);
  three or more ends on one box side are spaced evenly, centred where
  their edges head (fanout's Aggregator took 14 / 29 / 29 / 14 px), and
  a comb of arrowheads keeps a clear gap between its heads; when
  ends must make room, the ends of straight edges keep their place and a
  back-edge's give way; and a 1:1 pair on facing sides a few px apart
  along them is drawn straight, also after the router re-spaced a side
  it changed (advanced's `Gate --> Auth` was left 2.9 px off straight).
  Parallel edges keep their own spacing.
  In a drag (`route_partial`) the ends of the edges the drag keeps hold
  their places: a side whose every end the drag keeps is left exactly as
  drawn, a side is combed only round its kept ends, and a redrawn end
  that would throw a kept one more than half a port spacing takes the
  nearest free spot beside the kept fan instead, so a 1 px drag never
  re-spaces the fan of a node it does not touch (a state choice's
  labelled twins, whose slant asks for more than the port spacing, used
  to swap across their sibling on every drag of any node). An edge the
  drag keeps because it merely passes near the dragged node holds its
  ends too, and is rerouted only for a node, frame or title it now hits
  or a run on top of another edge, never for spacing alone (its ends
  used to be re-spaced by `route()`'s
  rules: a 1 px drag of a note card beside one threw it 22 px; over
  dense-td's and bench-75's drags 17 and 18 such jumps, now 3 and 15).
  A drag that
  falls back to a full `route()` (a composite state drawn wider than its
  members sends every drag there) draws the fans as `route()` does. A
  state note card's link holds its end whatever else shares its node's
  side (an even spread never pushes it aside), and keeps the router's
  corners, so `scene()` and `route()` draw an elbow or straight link
  alike (its elbow ends used to sit 7-18 px apart in the two, a straight
  one 4 px); a link the router redrew round something may still take
  another course in a full `route()`. A drag (`route_partial`) keeps a
  note link exactly as drawn while neither its card nor its state moves
  (unless the dragged node now lies across it), and carries it along
  when one of them moves up to 8 px, also when the drag is redrawn by a
  full `route()`; a 1 px drag used to throw it 20-70 px in about one
  composite-state diagram in ten. Class and ER diagrams,
  whose painters set their own ends, are unchanged. The layout search
  judges a candidate on the old depths, so the head room never changes
  a layering or ordering choice.
- **No dead room under a subgraph's first row** (report M8, §2.8). A
  box's title strip now takes its room only above the box's first row
  (below it in BT, the screen top), so the next row inside follows at the
  usual layer gap; the strip used to be reserved on both sides of the
  row, leaving 26 px of empty band under every box's first row. Titles
  are still never struck. Where an edge runs sideways across a gap into
  (or out of) a box, the gap keeps the box's border clear of the run's
  middle, so the curve crosses the border instead of riding it (the
  strip used to give that room by chance: advanced's `Gate ==> Job
  Queue` would otherwise run along Async Workers' top); a straight drop
  into a box keeps the plain gap (subgraph 786 -> 758 px tall, advanced
  1 267 -> 1 236, dense-td 1 823 -> 1 746). In the static scene, two
  unrelated edges left side by side closer than the edge spacing get one
  moved clear (a free curve slid along the flow, a router leg across its
  channel) where that costs nothing else, and an end's straight run into
  its arrowhead is left out where it would swing the curve into a box it
  passes beside (it is squared at paint time instead). A path whose one
  step lies between its two port legs is drawn straight where both ports
  can slide onto a line their sides share (BT, a port slid off a frame's
  title chip left a 29 px step right under the head, on the border of the
  frame round its node). `route()` no longer threads a channel between
  two boxes closer than its two 16 px steps: the step out of one passed
  the step into the other and the path doubled back on itself; such an
  edge starts as the static scene's direct curve.
- **Long node labels wrap** (report M7, §2.12). A flowchart or state
  label that breaks none of its own lines now wraps at 200 px (mermaid's
  `flowchart.wrappingWidth` default) instead of stretching its node into
  a banner: long-labels was 672 px wide with a 616 × 65 sliver of a
  diamond, and is 406 px now (mermaid: 358). It breaks at its spaces,
  and between CJK characters, which need none (never before closing
  punctuation or a small kana); a word is never split — one wider than
  the width keeps its line, the other lines may fill up to it, and a
  lone word beside it joins it instead of being stranded. The lines are
  evened out, so no word is left alone on the last ("Return documents /
  to the applicant"), and `<b>` / `<i>` markup wraps too, its tags
  re-opened on each line. A label with a `<br/>` keeps exactly its own
  lines, so the dense diagrams mermaid turns into 4–8-line towers stay as
  written (one node of the dense fixtures wraps, into two lines). The
  width is set with `%%{init: {"flowchart": {"wrappingWidth": N}}}%%` or
  `wrappingWidth: N` under `config.flowchart` in the frontmatter (mermaid
  12.2 draws its node labels at 120; at 120 or 150 a dense drawing's
  three- and four-word labels wrap too and reshuffle its layering), and
  survives `to_mermaid`. Circles grow both ways with their text, so they
  wrap at the width / 1.6, as diamonds do (below): "A circle with a long
  label inside" is a 126 px circle of two lines, not a 238 px one. The
  wrap is layout only: the model's `Node::label` and `to_mermaid` keep a
  plain label as written, the scene's node label (`SceneNode::label`,
  what hosts paint) carries the drawn lines, and the node is sized for
  them. An embedder measuring with a real font sizes nodes with
  `layout::intrinsic_size_wrapped(node, g.wrapping_width(), measure)`
  and passes the same measure to the new `scene::scene_sized_with` /
  `scene::route_sized_with`, whose label lines are the ones its boxes
  were sized for (`scene_sized` / `route_sized` wrap with the built-in
  estimate). Markdown node labels wrap the same way at the same width
  (a fixed 175 px greedy before), each of their lines on its own; their
  lines are broken as they are lowered, so `Node::label` holds them, but
  `to_mermaid` writes the markdown back as written and
  `Graph::set_wrapping_width` lowers it again at the new width. Each
  label is wrapped once per `scene()` / `route()` / `route_partial()`
  call, and a drag's frames reuse the wrapped labels, so dragging long
  labels costs no more than before. New: `layout::WRAP_WIDTH`,
  `layout::wrap_label`, `layout::intrinsic_size_wrapped`,
  `scene::scene_sized_with`, `scene::route_sized_with`,
  `Graph::wrapping_width`, `Graph::set_wrapping_width` (`None` keeps
  every label on its written lines, as class and ER boxes do). Edge
  labels and subgraph titles stay on one line.
- **Long decision labels wrap into diamonds no flatter than 1.6 : 1**
  (report §2.12, `layout::DIAMOND_ASPECT_MAX`). A rhombus is 1.6 times
  as wide as its text, so a diamond's label wraps at the width / 1.6
  (125 px by default: a decision is then no wider than a box wrapping at
  200), and takes a line more while that keeps the rhombus within 1.6 : 1
  at its text's own height (long-labels' question: four even lines in a
  220 × 151 rhombus; "Payment approved?" two lines in 145 × 94). Short
  labels keep their compact one-line diamonds, as before (`A{ok?}` 80 ×
  65, fanout's "All succeeded?" 192 × 65), and a label-less branch point
  its small square; only a single word too long to wrap grows the
  rhombus taller to the cap instead of into a sliver.
- **Back-edges run in orthogonal lanes, as in mermaid** (report §2.2,
  Q4 + M4). A flowchart or state edge running against the flow used to be
  one wide cubic "bow" round every node on the layers it spans, its own
  ends included, entering its target at 13–17° and often under the box.
  It now leaves its source square out of the side facing upstream, runs
  in a lane just past what lies *between* its ends (the layers strictly
  between them, and anything on an end's layer that reaches into the
  band), and enters its target square, with rounded corners delivered as
  densely sampled `SceneEdge::waypoints` (router style, so a host's spline
  through them draws the same path; `bezier` is a cubic through the lane's
  corners). The lane keeps 24 px plus half its label from what it passes,
  takes the spot that crosses fewest edges (then the shorter way round),
  never threads a gap between nodes, and nests with other lanes without
  crossing their legs: lanes from ends stacked on one layer into one node
  run side by side straight out of their ends and turn in one after the
  other, and lanes nested round each other are dealt out so the end
  nearest their side takes the outermost lane and the shortest stub;
  lanes of edges between the same two ends leave and enter in that order
  too, the outermost nearest the lane (x6: two `Gamma -> Alpha` lanes
  crossed twice in `route()`). A lane counts the lanes planned before it
  among the edges it may cross, and of two sides that cost the same takes
  the one the lanes sharing an end with it took, so `scene()` and
  `route()` pick the same sides (x2-nested-backs: `fail` and `rollback`
  swapped sides on the first drag, and the static `rollback` crossed
  `retry`'s lane). `route()` judges a forward edge that runs through a box
  as drawn round it — as the layout's channel and the router draw it — and
  a straight one lying along the lane as on it, not crossed (a lane jumped
  sides on the first drag where a forward edge's chord ran through a box
  between its ends, or ran on top of such a chord).
  Where it meets an end another edge also uses, the lane is judged by
  where that edge heads, so it never turns across an edge arriving beside
  it: the state `retry` loop runs straight up out of `Rejected` round the
  outside, crossing nothing (it ran between the Validating frame and
  `Rejected`, across `invalid`). A lane running straight into its
  target's busy side counts the edges leaving beside it, and a leg out of
  a member of a frame the lane leaves runs a stub past that frame's
  border instead of along it. When another edge fans out of (or into) an
  end toward the lane — a hub — the lane meets that end on its side
  facing the lane, the lanes meeting one side one after the other, when
  the lane passes outside the end itself by 24 px (a lane within the
  end's breadth ran on along it through the box and turned to its flank
  from inside, the arrowhead hidden under the node and pointing out:
  `rollback` into `Check queue` in an RL chart, `Check -> c` in an LR
  state choice). A lane meets a hexagon or parallelogram on the flat run
  of its side, off the slant (advanced's `fail` lane put its head half
  over Developer's corner), and its end stays on its side when the side
  is over-full: the ports stage moves other ends round the corner, never
  a lane's, whose last leg it would swing in at a slant. The
  router takes the lanes first, so the forward edges route round them,
  and in `scene()` an edge it searches for riding a frame border or
  cramped searches on past a first candidate that does not beat it
  (bench-75's `svc4 -> logs` rode the app frame's border for 99 px).
  demo is 321 px wide (was 383), back-edge 290 (351), fanout's `no` loop
  goes round the outside, and the flow corpus crosses fewer edges (524 ->
  505 static, 519 -> 508 in `route()`). The same rule runs in `scene()`,
  in the layout's own estimate of the drawing it centres (so it reserves
  exactly the lane's room) and in `route()` / `route_partial()`
  (O(nodes + edges) per back-edge, inside the drag budget).
- **A back-edge with nothing between its ends crosses the gap** like a
  forward edge, a reversed S out of its source's flow-in side — a 2-cycle
  (`A --> B`, `B --> A`) draws its two edges side by side, uncrossed, in
  every direction, both labels in the gap (the layout fits the gap to
  the return edge's label too). lr is 226 px tall (was 299, the return
  edge a V-shaped loop below the row); the twins' ports keep their drawn
  order at both ends, and the pair counts as one link when the layout
  aligns the two nodes. In LR / RL a labelled 2-cycle's two curves bow
  apart by a label's height, so each label rides its own line (wide-labels'
  return label covered its twin near Client) — in a short gap less, so
  each end still leaves and enters within 9.5° of square, and the ports
  stage moves a control that runs straight out of an end along with it
  (state `lr_right`'s return head entered `Draft` at 30°). Where the
  twins' ends lie far apart across a short gap, their middles ran nearly
  across the flow only a port spacing times the slope apart — a single
  thick band (state r215: 60 px within 6 px), or the router took the
  return edge off its twin and ran it beside a note link — so the return
  curve now moves its middle away along the flow, by lengthening the
  tangent at the end it moves from: its twin and the forward edge's
  course stay as they are, both ends still leave and enter square, and a
  labelled return edge moves far enough for its label to keep off the
  twin. Twins already bent hard into an end, and a return edge among
  more parallel edges between the same two nodes, keep their course, and
  the router reroutes them as before. A back-edge whose ends lie
  more than 450 px apart across the flow takes a lane instead: a reversed S
  would sweep along the gap through every edge there and meet its end at a
  slant (bench-75's `svc2 -> db` entered `db` at 71° in `route()`). On a
  diamond or circle such an end stays on the part of the border facing
  along the flow, where the ports stage spaces it from its twin's (a
  2-cycle into a diamond put its head on its twin's start, on the
  diamond's flank).
- **An LR / RL back-edge's label takes its height across the flow**, not
  its width: wide-labels is 160 px tall (was 413, the loop under the row
  sized by a 337 px label).
- **An edge between two members of a subgraph stays inside it**: the
  frame grows over a back-edge's lane and its label (advanced's `fail`
  edge bulged out of CI/CD) — in `route()` and `route_partial()` too, so
  a labelled loop inside nested boxes no longer jumps out of both frames
  on the first drag — and the router prices any path that leaves the
  innermost frame holding both its ends right after a foreign frame or
  title chip it enters, ahead of a backtrack or a crossing (never a
  reason to reroute on its own, but of the paths a reroute weighs the one
  that stays home wins: a member's edge to its own box's entry member no
  longer leaves the box through its bottom and comes back in, 798 px out,
  in `route()`).
- **Self-loops keep their room** (report P0 #13): the layout keeps a
  looped node's loop (and 12 px) clear of its high-side neighbour —
  widening the gap only by what the loop reaches past it (stress is 269
  px wide, 221 before, 293 while the whole reach was added) — so a
  long edge's channel beside it no longer runs through the loop's apex
  (stress: `A ==> D`), and a back-edge's lane clears the loop too: it
  never meets an end on the flank that end loops on, but crosses its flow
  side there instead (a lane into a looped hub turned in through the loop,
  its head on the loop's own). The
  router treats every self-loop as an obstacle, so `route()` keeps other
  edges off it as well (stress's `A ==> D` ran 5 px from `B`'s loop on a
  drag). `route()` draws a loop on the side `scene()` does — in LR / RL
  below its node, not on the screen's right, where its node's edges
  leave (x6: a loop hidden under three parallel edges) — and a drag that
  crowds that side moves it to the free one. A node's loops stack 16 px
  apart along its side, labelled or not (their labels sit past their
  apexes), each starting and ending on the border: a stack longer than
  the side (three loops on a 38 px box overhung it by 5 px) is squeezed
  evenly onto it, in order, short of its corners.
- **A labelled parallel edge keeps its label off its twin** (report
  P0 #13): in TD / BT, parallel edges next to a labelled one leave
  half the label's width plus 15 px between their lines (self-loops
  excepted: they stack along their node's side; a 16 px gap put
  stress's `parallel` label on the unlabelled edge), and a label both
  lines touch counts as the one whose line runs nearer its centre — an
  unlabelled line only: a labelled one carries its own label, so a
  `<<choice>>`'s middle twin label keeps its spot beside its line
  instead of moving onto it between both neighbours (state `twins3`).
  In LR / RL a gap holding a label on an edge of a node a lane leaves
  or enters by a leg across that gap widens to keep the legs clear
  (x6-rl-loops: `forward` and `parallel` lay over two lanes' legs).
- **Class and ER diagrams follow the same back-edge, 2-cycle, parallel
  label and loop rules**, as they ride the flowchart layout and scene:
  a relation against the flow of a cyclic diagram comes as a lane the
  class layout keeps orthogonal, a 2-cycle's two lines and labels stand
  apart (ER `works in` / `employs` no longer read as one band), a TB / BT
  class's labelled parallel relations leave room for their labels
  between their lines, a class's self-relations keep their room beside
  it (nine self-relations on one class ran into both its neighbours), and fewer
  relations are left on their curves (399 -> 346 over the class test
  inputs, 456 -> 385 in `route()`). A dense cyclic diagram can grow:
  seven classes each related both ways to every other are 1473 px wide
  (was 1027). There, a routed relation whose end moved used to slide a
  leg along the square exit of another class's last end on that face,
  which then had no way out and kept its curve — a sweep across the
  drawing's orthogonal lines; such a slide is now refused and the relation
  stays as routed.

- **One arrowhead size for every flowchart edge, sharper and square onto
  the node** (report §2.3, Q1 + M1). Every end glyph — the arrowhead at
  either end, the cross and the circle — is now 11 px in user space
  (`markerUnits="userSpaceOnUse"`), so a thick `==>` edge no longer draws
  a head twice the size (about 22 px) and a `linkStyle stroke-width` edge
  up to about 6 px shares the head too. The head is the wider
  `M 0 0 L 10 5 L 0 10 z` triangle. The SVG painter stops the stroke half
  a head short under an arrowhead — a `linkStyle` stroke wider than that
  its own width short — with `refX` putting the tip back exactly on the
  node border, so the butt end of the stroke no longer pokes out beside
  the tip and the tip no longer overshoots into the node. Over a stroke so
  wide the head would no longer hold its cut, the corner onto the straight
  run past it and half the stroke there, the head grows to just that
  (`width + run + width / 2` px, the run past the cut a quarter of the
  width: 14 px for 8 px, 24.5 px for 14 px, 42 px for 24 px; a 14 px
  stroke used to end in a butt wider than its 11 px head, notched where it
  met a 1.5 px run), its stroke runs straight through it, bending into the
  run within half its width of the curve. Where a curve turns too hard
  over its last stretch for such a run along its end tangent (a back-edge
  bow turning into its node a few px out, a routed lane turning up just
  short of the end, a tight S, a self-loop under a very wide stroke), the
  head turns with it: the stroke runs straight into the end along the
  chord from where the curve first lies a run's length away, so the tip
  stays on the border — a 24 px stroke's head used to face along the
  curve's tangent at its cut, its tip up to 19 px off the node (37 px
  mid-drag), beside it instead of on it, with ink picking did not reach.
  A curve too short or too curled even for that (nodes all but touching
  mid-drag) is cut no deeper than the straight run it ends in, so its tip
  still lands on the end. The same holds for mermaid's `step`,
  `stepBefore` and `stepAfter` curves (`e1@{ curve: step }`, `linkStyle …
  interpolate step`), whose last leg ends at a corner: a head faces along
  that leg, or turns with the step where the leg is shorter than the head
  needs, with its tip on the border in `scene()`, `route()` and every drag
  frame, and picking reaches its ink. On a curve too short
  to lose its cut (nodes a few px apart mid-drag, stroked whole) a head
  is cut nowhere, its tip on the curve's end, and shrinks onto its share
  of the curve (left out under 1 px), instead of reaching its usual cut
  past the curve's end into the node, where picking did not see it. Every glyph faces along
  its curve's own end tangent (square to the side wherever the curve
  arrives square): the painter ends a curve that still turns there in a
  straight run along that tangent, bending into it within a pixel of the
  curve, so the stroke reaches the head's base already heading its way;
  a curve that swings in from the side too hard for that (a wide fan's
  outer branches, an S-curve whose short end handle only turns square in
  its last pixels) leaves its curve where the line to the head touches it
  and runs on straight into the head — no S where it leaves the curve, no
  hump before the head — its head still square with the tip on the
  border, and one that slips in under the head first keeps its own curve
  up to there. A long edge's spline whose
  last channel leg slanted into its node gets a straight stub along the
  side's normal in `SceneEdge::waypoints` (so hosts draw it too), unless
  the stub would bring it nearer a node or along another edge than the
  routing left it, and a route the router draws into a head grown past
  11 px takes a stub long enough to hold it where that costs about the
  same way round (a 16 px `linkStyle` edge turned into its node 14 px
  out under a 28 px head, which hooked past the side into it, in
  `route()`). The squared ends and the cut are the SVG painter's:
  `SceneEdge::bezier` is unchanged and still ends on the border (hosts draw
  their own heads along its end tangent), the painted curve stays within a
  pixel of it outside the head beyond two head lengths of the tip, and
  `Scene::edge_at` / `hits_in_rect` follow the painted stroke — at their
  old speed: `to_svg` squares each end once (the crossing casing reuses
  the strokes it paints), and picking squares an end only where that can
  change the answer (near its tip, close to the curve), so a marquee or a
  pointer away from the arrowheads costs what it did. Marker ids:
  a `linkStyle` width no longer needs a size variant, so the `-u` ids
  (`arrow-u`, `arrow-u__<colour>`) are gone — a coloured end is
  `<base id>_<colour>` (`arrow__ff3`), a stroke wider than 5.5 px gets
  `<base id>-t<width>` (the head set further ahead), a head not 11 px long
  adds `-s<length>` (`arrow-t8-s13_5`; a shrunk one `arrow-t0-s4_1`), and
  an unstyled one keeps its base id. Arrowheads standing side by side
  closer than a head and 1.5 px apart (an over-full side's ports, about
  10 px apart) are drawn narrower, `<base id>-w<width>` (`arrow-w8_5`,
  never under 6 px nor too narrow to cover the stroke's end), so they
  keep a gap instead of running together into one saw-tooth band. The
  advance painter draws the same head and cut, its last stretch along
  the route's last leg however short that leg. A crossing gap is no longer cut within a head and a gap's
  radius of an edge end, where it could bite the arrowhead.
- **Stroke tokens** (report Q10): edges are 2 px (thick 3.5 px, from
  1.7 / 3.4) and node outlines 1.8 px (from 1.6, the cylinder's lid, the
  subroutine's bars and a final state's ring included). The canvas keeps
  its 28 px padding — the scene's own gutter — rather than Q10's 16 px:
  `to_svg` pads exactly as `Scene` coordinates, `width` and `height` do,
  so a host drawing the scene sees the SVG's canvas, and moving every
  laid-out coordinate by 12 px would flip the router's float-exact
  tie-breaks (a sixth of the test graphs reroute).
- **Quality yardstick** (`quality::Metrics`): `end_tilt` judges each end
  glyph's heading as painted (along the stroke cut short under it) and
  reads frame ends either way across the border; new `end_kink` (appended
  after `x_drawn`) counts glyph ends whose stroke meets the head's base
  more than 10° off square. The yardstick measures the painted strokes
  (squared glyph ends included), as the crossing casing cuts its gaps in
  them; `quality::ARROW_LEN` follows the head size (11 px).
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
- **Dotted edges read as dotted** (report Q9). `-.->` and `-.-` were drawn
  with `5 4` dashes, which read as dashed; they are now dots one stroke
  long a stroke and a half apart — `stroke-dasharray="2 3"` at the 2 px
  edge (mermaid draws `2 2`). The pattern scales with the stroke, so a
  `linkStyle … stroke-width:4px` dotted edge keeps the look (`4 6`) instead
  of turning into stripes; a `linkStyle` / `classDef` `stroke-dasharray`
  still wins. The advance writer draws the same dots.
- **Sibling edge labels keep 20 px apart** (report Q14). The labels of
  edges out of one node (or into one) whose other ends sit side by side —
  a decision's `valid` / `invalid` — could sit 4 px apart and read as one
  phrase ("valid invalid"). The label stage now keeps such siblings
  20 px apart, side by side or stacked, as a `<<choice>>`'s fan already
  kept them, in
  `scene()`, `route()` and `route_partial()` alike, sliding a label along
  its line or stepping it just beside it as for any other obstacle; the
  layout is unchanged. A `<<choice>>`'s fan keeps its own rows, and in a
  busy hub this ranks with the other soft obstacles (a label still never
  covers a node, a head or another label to get it), so a dozen labels
  out of one node can still sit closer.
- **Cylinders read as drums** (report Q7). The lid is filled like the
  body (the drum used to run under the lid's front arc, so the canvas
  showed through the lid), its depth follows the width as in mermaid
  (`rx / (2.5 + w / 50)`: 7.5 px on the narrowest drum, up to 10 px from
  about 80 px wide; it was a fixed 8 px) and the label sits in the body
  under the lid (0.75 of the lid's depth below the box centre) instead
  of up against its rim. Node sizes are unchanged. A `style` /
  `classDef` `stroke-width` now reaches the lid's front arc and a
  subroutine's bars (and a final state's ring), which kept the default
  width. An edge end on a cylinder's top or bottom now meets the drum's
  arc instead of the box side, in `scene()`, `route()` and every drag
  frame (it floated up to 4 px above the lid's back arc, or below the
  bottom arc, off the middle). The advance writer draws the same drum.
- **Nested subgraph frames read apart, and their borders read at all**
  (report Q6). Every frame used to be the same `#f7f8fd` with a `#c9cfe8`
  border, 1.55:1 on the white canvas, so nesting was hard to see and edges
  ended on barely visible lines. Each nesting depth now takes a cool grey a
  step deeper (four steps, then the deepest), with a border at least 3:1
  on the canvas, on the frame round it and on its own fill; a title's chip
  takes its frame's fill and the title ink is `#4a5070` (5.7:1 or better
  on every depth; `#6a7086` fell under 4.5:1 on a nested frame). State
  diagram composites draw the same way. Layout and titles' placement are
  unchanged; titles still paint after the edges, never struck.
- **Crossing gaps no longer make a line look broken** (report Q15,
  §2.10). Which line of a crossing goes under was chosen crossing by
  crossing to keep gaps sparse, so a line crossing two others a few px
  apart could go under the first and over the second — a gap, a stroke
  and the other line's gap in a row, which reads as a break (fanout's
  loop-back did, 14 px apart). Crossings on one line closer than 25 px
  are now one cluster, and the line takes one role across it — the role
  most of them already gave it (a tie: under for a back-edge, else over,
  so the line stays whole) — wherever that chains no gaps into a dashed
  look and piles no line with more gaps than the busiest already had
  (quality fixtures: 109 | 138 such flips static | routed -> 76 | 99,
  dense-lr 6 -> 1, dense-td 13 -> 2; the rest sit on bench-75's busiest
  bands). A line shorter than three dash periods (180 px) now also hands
  its extra gaps to the longer lines crossing it where that dashes
  nothing, so a short edge under two long sweeps is cut once, not twice
  (a back-edge keeps its gaps, going under by preference).
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
  - **Two-way relations and lollipops:** `model::RelKind` gains the
    variant `Lollipop`, so an exhaustive `match` on it needs an arm (or
    `_`). `class::RelStyle` gains `from_kind` and becomes
    `#[non_exhaustive]` (it comes from `class::scene()` / `class::route()`;
    read its fields). `class::Head` gains `circle`: a struct literal must
    name it or end in `..Default::default()`, and a painter should draw it
    (a hollow circle) or lollipops show no glyph. A painter that draws only
    `rel_head` misses the second glyph of a two-way relation: draw
    `class::rel_tail` too. A lollipop interface is a class
    (`Class::lollipop`, `ClassBox::lollipop`) whose box is not drawn, only
    its name; a host that paints every `ClassBox` as a box should skip it.
  - **`quality::Metrics` gains columns and becomes `#[non_exhaustive]`:**
    the phase-B yardsticks (`end_tilt` … `x_drawn`), `end_kink` and `ink_out` are new
    fields, so a struct literal of it no longer compiles outside the crate
    (it already broke with the first new field). It is what
    `quality::measure` returns: read its fields, or start from
    `Metrics::default()` and assign them. New columns no longer break it.

### Fixed

- **Edge ends touch every shape's outline** (report M6, P0 #9). The edge
  stages met a parallelogram, a trapezoid (drawn as one), a hexagon and a
  cylinder as their bounding box, so ends floated 7–10 px beside a
  slanted flank or a hexagon's point and up to 4–9 px beside a drum's
  shoulder; a self-loop's ends sat on the box side of any shape, a
  diamond's included. A final stage now puts every end on the outline as
  painted, along its stroke's final run (a run leaving at more than 60°
  to its side goes straight in from it instead; an end on a drum's
  shoulder goes to the shoulder's nearest point), in `scene()`, `route()`
  and every `route_partial()` drag frame, so hosts painting the scene's
  geometry get it too. Ends spread over a hexagon's or a parallelogram's
  top or bottom keep to its straight run, off the slants, and a hexagon's
  point spreads its ends like a diamond's corner. `Scene::node_at` /
  `hit_test` now pick hexagons, parallelograms, cylinders, stadiums and
  rounded boxes by the outline those ends sit on (their empty box
  corners are no longer the node); plain boxes still pick by their box.
  Architecture service cards, picked as rounded boxes, are painted with
  the same 9 px corner (it was 8 px).
  `quality::Metrics::end_gaps` counts self-loop ends too. The advance
  writer is unchanged.
- **A `%%{ … }%%` directive written over several lines parses.** Its inner
  lines were read as diagram statements — `%%{\n  init: {\n    "theme":
  "dark"\n  }\n}%%` failed every diagram type with "expected a node id".
  The lines are now blanked before parsing (line numbers stay true), as the
  frontmatter block is.
- **Spacing an edge's end never slides its path into a node.** When the
  ports stage moved the end of an orthogonal path along its side, the
  straight leg out of that end slid with it — a long leg the router ran a
  dozen px past a box landed in it (a forward edge in `route()` drew up
  through the box beside two lanes' legs). A side whose re-spacing would do
  that keeps its ends where the router put them.
- **A nested subgraph keeps its nodes when the outer block mentions them
  again.** In `subgraph Platform … subgraph Workers … end … W1 --> Users
  … end`, the mention of `W1` after `Workers` closed moved it back into
  `Platform`; `Workers` was left empty and was not drawn at all (the
  advanced showcase drew 2 frames for its 3 subgraphs). As in mermaid, the
  first block to close keeps a node: a later mention, in the enclosing
  block or in a sibling opened afterwards, leaves it where it is. A block
  that is still open (an enclosing one) still gives a node up to the
  nested block mentioning it, and a top-level node still joins the first
  block that mentions it.
- **An edge into (or out of) a subgraph ranks it even when a cycle runs
  through the box.** `A --> box` lays the box out after `A` by standing
  for the box's entry members — those without an in-edge from inside it.
  A cycle inside the box (`Cache --> Gate` in the advanced showcase's
  Platform) left every member an internal in-edge, so the edge shaped no
  rank at all: the CI box sat beside Platform instead of above it, and
  `Reg --> Platform` ran sideways into Platform's flank. The edges that
  close such a cycle are now set aside (a depth-first walk over the box's
  own edges, in declaration order), so `Gate` enters Platform, CI stacks
  above it and `Reg --> Platform` drops straight onto its top. An edge
  meeting a box against the flow (`Rollback --> Platform` from below) now
  runs straight across the gap into the facing side instead of bending
  towards the box's entry member at its far side.
- **An edge from a member to its own box leads to the box's entry
  member.** `Dev --> CI` with `Dev` inside `CI` was ranked as an edge to
  `Build` (CI's entry member) but drawn as a short arrow up to CI's top
  border, beside the title and against the flow, leaving an empty band
  where the ranked edge ran. It is now drawn — and routed, labelled and
  picked — as that edge, from `Dev` onto `Build`, in `scene`, `route` and
  drag frames (`route_partial`, which redraws it when the entry member
  moves) alike; the scene edge keeps its ids (`Dev` → `CI`) and its index.
  The entry member is the box's first member nothing inside the box leads
  to, other than the node and anything it already links to; without one
  (or for a box drawn to its own member) the stub to the frame stays. A
  box drawn to its own member no longer ranks the member below the box's
  exit members through a stand-in nobody saw.
- **The canvas fits the drawing, with even margins.** The canvas was
  bounded by each edge's bezier control points, which reach far past the
  curve of a back-edge's bow: the demo had 65 px of empty canvas right of
  its loop against 27 px on the left (now 28 px on both sides, 383 ->
  346 px wide), back-edge's canvas was 351 px wide (now 315), stress's
  251 (now 221), and state, lr, wide-labels, bench-75 and advanced were
  padded the same way. The canvas (`to_svg`'s
  view box, a scene's `width` / `height`, the quality yardstick's) is now
  the drawing itself — each curve to its exact extremes, every stroke's
  width and end glyph, every label — and the same 28 px margin on every
  side: a static scene is shifted and sized to fit it exactly (room the
  layout left before the drawing is dropped too: dense-td's scene had
  315 px of it on the left), and `route` / `route_sized` / drag frames
  size their canvas the same way (their node positions stay the host's).
- **`scene()` of an LR / RL flowchart with wide edge labels has the
  SVG's geometry.** The layout laid an edge label's width across the flow
  in every direction, so in LR it inflated the scene's height: wide-labels
  reported a 624 px tall scene for a 413 px SVG (now 345 px, both), every
  node 119 px lower than drawn, so a host painting the `Scene` saw a
  different picture than the SVG. A label's width now runs along the flow
  in LR / RL, and a static scene's node centres are exactly where the SVG
  draws them.
- **Nothing drawn is cut off at the canvas edge.** The canvas (`to_svg`'s
  view box, a laid-out scene's `width` / `height`, the quality yardstick's
  `width` / `height`) was the bounds of the nodes, frames, labels and each
  edge's bezier points, so a route the router took round the outermost
  node or frame could run off it (a detour leg past the last column,
  clipped by up to 34 px), and so could a stroke's width or an end glyph
  at the edge of the drawing. It now covers every drawn edge — the routed
  spline to its exact extremes, half the stroke width out, and each end
  glyph's box — in the static scene, `route` / `route_sized` and drag
  frames (`route_partial`) alike, the padding round it as before. A
  static scene whose drawing reached into its 28 px gutter is shifted
  right/down so it starts 28 px in, exactly where the SVG puts it (only a
  translation after every stage: no route moves). New
  `quality::Metrics::ink_out` counts edges with ink off the canvas, gated
  at 0.
- **A twin's label in a `<<choice>>` fan finds a spot beside its line**
  when its row is crossed by its twin's steeper line: it may now step
  beside its line anywhere along the line's far half, so it is stacked
  `FAN_SIBLING_GAP` clear of its siblings instead of being left to the
  general placer.
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
- **Routing never returns NaN for odd node sizes.** `scene::route_sized`
  (and `scene_sized`, `box_edge_bezier`, and so `er::route` / `class::route`)
  wrote NaN bezier points and label spots when a host passed a 0 × 0 size:
  a zero-size diamond divided 0 by 0 on the way to its border. Host sizes
  now have documented semantics: a negative or non-finite (NaN, infinite)
  width or height counts as 0, and a 0 × 0 box is a point node whose edges
  meet at its centre. For finite centres — coincident ones included — every
  bezier point, waypoint and label spot is finite and the SVG writes no
  `NaN`/`inf`. Valid sizes are used exactly as before; rendering is
  unchanged.
- **Marquee selection measures the painted stroke.** `Scene::hits_in_rect`
  tested the rectangle against a sampling of each edge's centre line, so a
  marquee covering only the stroke beside the line missed it, and on a long
  bend the samples cut up to ~2px inside the drawn curve. An edge is now
  selected once its painted stroke (the curve `to_svg` draws, widened by
  half its stroke width) touches the rectangle, as `edge_at` measures it;
  a full-canvas marquee is also about 4x faster on large diagrams.
- **ER and class edges are picked at the width they are painted.** Picking
  assumed the flowchart's 1.7px stroke for every scene, while the class
  painter strokes relationships 1.6px and note links 1.2px wide. The width
  each ER/class edge is painted with now travels on the scene edge and is
  read by the painter and by `edge_at` / `hits_in_rect` alike.
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
- **A 2-cycle no longer pulls its node off the middle of its fan** (report
  Q8). With the return edge written first (`CACHE --> SVC` before
  `SVC --> CACHE`), the pair was linked twice in Brandes-Köpf's neighbour
  lists, so the median landed on it and `lr`'s Service Layer sat level with
  Redis Cache instead of centred on Database / Redis. A 2-cycle now counts
  once whichever edge comes first, so either order draws the same
  symmetric fans. Parallel forward edges still count once each: a twin
  pair runs straight down its own column, as its curves are drawn for.

