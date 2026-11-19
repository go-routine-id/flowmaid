## [0.38.0] - 2026-11-10

### Highlights

Flowchart parity phase B: B10, edges on the axes. No public API is removed
or changed.

- **Flowchart and state edges are drawn on the axes, with rounded corners,**
  as mermaid 12.2's ELK renderer draws them, where they were S-shaped
  curves, bowed twins and splines. An edge between adjacent ranks is a Z
  (an L into a side) stepping across the gap, its label on the step, or
  on the straight run into the target where the gap has room for the step
  between the source and the label; a long edge runs down its layout
  column; the steps in one gap sit on their own tracks, ordered so their
  turns cross as little as possible. A straight edge stays straight; an
  edge with an explicit `curve:` / `linkStyle … interpolate`, note links
  and self-loops keep their curves, as do the edges of a drawing with more
  than 400 edges (past the router's cap), but for an S whose ends lie
  under 28 px apart across, which is squared there too; back-edge lanes,
  on the axes already, are not redrawn, though a lane can now take the
  other side (bench-75's static `dash -> user` runs over the top through
  `edge` and `app`, as its `route()` lane already did; see Known costs
  under Changed); class and ER output is byte-identical.
- **Dragging keeps the look.** `route()` and `route_partial()` draw the
  same idiom; hosts get the path as `SceneEdge::waypoints` with rounded
  corners, and picking follows it.
- **Cleaner drawings.** Over the 14 corpus drawings (static | routed):
  S-shaped curves 92 | 112 -> 0 | 0, defects 57 | 90 -> 47 | 46, drawn
  crossings 499 | 504 -> 428 | 494; over 303 generated flowcharts defects
  404 | 331 -> 87 | 91. Known costs are listed under Changed (parallel
  cables on the corpus 97 | 107 -> 128 | 143 pairs, some LR fan labels
  over a neighbouring step, dense-td's `route()` crossings 73 -> 83); the
  small corpus drawings render in about their 0.37.0 time (0.5-1.2x,
  benchmark medians).
- **Benchmark re-measured on 0.38.0:** about 106x faster than mermaid
  12.2.0 (geometric mean over 25 diagrams), 19 of the 25 under a
  millisecond, SVG 1.5-25x smaller. The 0.37.0 run said 126x; the drop is
  the shared box, not this release: flowmaid's medians match that run's
  (1.01x overall, 0.90x over the 15 flowchart and state diagrams) while
  mermaid's ran 15% faster.

### Changed

- **Flowchart and state edges are drawn on the axes, with rounded corners**
  (report §2.1, B1, M2, M5, P0 #14), as mermaid 12.2's ELK renderer draws
  them, where they were S-shaped cubics, bowed twins and splines through a
  long edge's dummies. A new stage (`ortho`) runs after the ports are
  placed and before the router: an edge between adjacent ranks is a Z (an
  L into a side port) whose step runs across the gap, between its source
  and its label where the room above the label holds every step of that
  gap (the label then rides the straight run into the target, as mermaid
  draws a labelled branch), else with its label on the step; a long edge runs down its layout column, straightened within the
  room its dummies leave (21 px off a box, frame border, label or self-loop
  beside it, half way to the next column), stepping across a gap only
  where that room moves; a labelled twin bulges clear of its twin's line
  where the two would run together — unless the bulge would sweep across
  another end on its side or leave a composite holding both ends, when it
  is drawn plain and its label goes onto its own line, off its twin's; a
  straight edge stays a straight cubic. Each gap's steps sit on tracks a
  pitch apart (12.5 px, 6.5 px in a gap too shallow for that), ordered as
  libavoid orders a channel so their turns do not cross (a labelled fan's
  steps go under their labels when the room above them cannot hold that
  order at the least pitch, rather than two sharing a track); the steps of a
  fan going either way from a node fork apart on their own tracks, a step
  that carries its label keeps the label's half and its gap off the
  tracks beside it, and a step whose legs would cross a curve the stage
  leaves (a note card's link) or another step moves to the nearest place
  in its room that crosses fewest. In the static scene, two ends of one
  side whose paths still cross each other twice swap places (an L round
  the outside of a Z off the same side), and an end lying between a note
  card's link and its card goes to the link's other side, each kept only
  when the edges it changes cross fewer lines. A breadth side whose starts
  would run along their own row through a box (a `<<choice>>` fan spilled
  round the diamond's corners, past the note cards beside it) leaves on
  stubs planned nested by their order along the flow and steps in the gap
  instead (the channel nudge may still order two such steps by where their
  far ends turn when one of them is rerouted: bench-75's `svc1 --> queue`
  and `svc1 --> db` cross once either way, and the nudge puts that
  crossing under `svc1`). A long run in `route()` steps in the gap whose legs cross the
  fewest lines. The layout grows a gap by the
  tracks its steps need, in a drawing of at most 400 edges (two at the
  plain gap, one more per further step one way).
  A state composite's edge to one of its own states leaves the
  composite's border from inside, straight into the state. `route()` and
  the drag path (`route_partial`) draw the same idiom; an end a drag holds
  on a hub starts where the base had it even across sides, so the redrawn
  step matches the kept ones, and a single cubic that a later spacing
  slid a few px off its line (a choice's two ends spread, a drag frame
  respacing a kept end) is squared again — straight under a px, else a Z.
  In `scene()` and `route()`, a corner whose rounding point a moved end
  slid away from or dropped (the spline swept such a corner over up to
  1 600 px: bench-75's `svc6 --> ext`) gets it back, 14 px from the
  corner; `route_partial` does the same for the edges it redraws, not for
  an edge it keeps whose end the ports stage respaces (see Known costs). Past the router's
  400-edge cap, where the router and its nudge leave a drawing as built,
  the builders' curves stay too and no gap grows for tracks (a 531-edge
  chart's `route()` 25.7 -> 2.6 ms), but for an S under 28 px across,
  which the last stage squares whatever the size.
  The path comes as `SceneEdge::waypoints` (each corner flanked by points
  14 px either side, or half the shorter adjoining leg where a leg is
  under 28 px, which the painter's spline turns through as a curve of
  about 8 px radius, tighter at such a corner), `bezier` only a fallback
  (a cubic between its ends with its first and last corners as
  controls, or the straight line between them for an edge the router
  or its nudge redrew), so pick == render as before. Explicit
  `curve:` / `linkStyle … interpolate`, state note card links,
  self-loops, invisible `~~~` links and class / ER scenes are unchanged
  (class and ER output is byte-identical). Back-edge lanes are not
  redrawn, but `route()` now counts each border of a frame holding
  neither end that a lane's legs would cross when it picks the lane's
  side, and a lane can land on the other side from 0.37.0 (bench-75's
  static `dash -> user` now runs over the top, as its `route()` lane
  did). The router weighs a lateral run as `route()` does in
  the static scene too (`H_WEIGHT` 1.0), prices a forward edge's run
  along a node's flank, takes a second pass over pairs still drawn on top
  of each other, searches a way round for a static unlabelled edge
  crossing four or more others — and, in both modes, for an edge crossing
  a note card's link out of a node it shares (in the static scene also one
  crossing an edge out of a node it shares twice) — and the static reorder (G9) compares two
  squared paths off one side over their whole corners, so a fan's Ls
  turning the same way nest. The ports stage steers a Z or an L by the
  chord through its corners and slides an end's leg along with it (an end
  moved round a diamond's corner no longer kinks back to its old line),
  settles a side whose ends all hold finished square paths (a second pass
  leaves a 24-end hub as drawn), and puts an end heading into its own
  frame at the end of the side it lies at; the label stage keeps a long
  edge's label in its dummy's slot when the edge runs straight, restacks a
  compact branch point's fan whose labels crowd each other, and a
  nearly-on label lands on its new line. Two-point straight paths are
  drawn as straight cubics; an end sunk into a drum's lid along an axis is
  brought back to its outline. A path's rounded corners are sampled with
  a few chords (its legs as lines) for hit-testing and the label stage,
  which keeps the static scene near its 0.37.0 time on the corpus.
  Measured against 0.37.0 (c72e56b, the code B10 was built on), over the
  14 corpus drawings (static | routed):
  defects 57 | 90 -> 47 | 46, S-cubics 92 | 112 -> 0 | 0, splines 23 | 24
  -> 0 | 0, bows 2 | 2 -> 0 | 0, end kinks 74 | 94 -> 1 | 1, edges on a
  frame border 11 | 20 -> 0 | 7, drawn crossings 499 | 504 -> 428 | 494
  (bench-75 349 | 375 -> 302 | 361, fanout-frames 3 | 3 -> 0 | 1,
  dense-td 92 | 73 -> 80 | 83); bench-75 defects 5 | 27 -> 4 | 1. Over 964
  generated state diagrams: drawn crossings 762 | 1 021 -> 712 | 826,
  defects 633 | 558 -> 616 | 537, pairs of edges out of one node crossing
  each other twice 20 | 17 -> 14 | 16, edges through a node 45 | 55 -> 32
  | 39; over 303 generated flowcharts: crossings 2 902 | 2 988 -> 2 874 |
  2 775, defects 404 | 331 -> 87 | 91. Known costs: parallel cables (a
  channel's tracks, a fan's bundled stubs) 97 | 107 -> 128 | 143 pairs
  on the corpus; bench-75's static crossings still above mermaid's 222
  (its layout's node order bounds them); two more edge-frame pairs there
  (2 -> 4: `dash -> user`'s lane now runs over the top, through `edge`
  and `app`); dense-lr's LR fan
  labels over a neighbouring step (static label-over-line 1 -> 5);
  dense-td's `route()` crossings 73 -> 83; plain twins' labels over a line
  in narrow composites; and on bench-75 `route()` takes `job1 -> svc1`'s
  lane round the other side from the static layout (its planner reads gap
  depths in px); a 1 px drag throws one more unrelated end on dense-td
  and on bench-75 (the re-rounded corners change where the ports stage
  reads a kept path heading); in a `route_partial` frame an edge the drag
  keeps whose end the ports stage respaces keeps its old corner points,
  so its painted curve can sweep its corner by up to about 14 px. Static
  `scene()` takes about its 0.37.0 time on the small corpus drawings
  (renders 0.5-1.2x, benchmark medians); against the phase-B baseline
  (5f318a0, before 0.33.0) it is up to 1.9x (fastest of interleaved
  runs: 1.3-1.9x on eight of the eleven, advanced the slowest, its
  labels, ports and nudge over 18 squared paths; lr and fanout take
  less), mostly from the layout and edge work released in 0.34.0-0.37.0.
  Drags take less than before on the large
  ones: a full `route()` drag frame on bench-75 averages 8.7 ms (13.1 ms
  at the phase-B baseline), and `route_partial` 3.2 ms.

