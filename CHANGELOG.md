# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
