# blockmetrics — the yardstick for B12 "subgraphs as blocks"

`blockmetrics` reads an SVG drawn by flowmaid or by mermaid and prints one TSV row per
diagram: how compact each subgraph frame is, whether sibling frames are stacked along the
flow or sit side by side, how much edge runs through frames it has no business in,
crossings, edge length, and where the edge labels sit. Both engines are measured by the
same code after their SVG has been reduced to the same model (frames, node boxes, edge
polylines, label boxes), so the two tables can be compared cell by cell.

Everything here is Node.js / python3 standard library only. Nothing in this directory is
part of the flowmaid crate.

## Files

| file | what |
|---|---|
| `blockmetrics` | the measuring tool (node). `--help`, `--columns`, `--selftest` |
| `structdump.rs` | Rust example: dumps a diagram's structure (+ flowmaid's scene) as JSON, public API only |
| `bin/structdump-038` | that example built against 0.38.0 (commit 0366084), ready to run |
| `struct/<name>.struct.json` | structure files for every input used here (the structure is a property of the input) |
| `run.sh` | one command: render the inputs with a flowmaid CLI (or mermaid) and measure them |
| `judge`, `thresholds.json` | check a table against the proposed B12 acceptance rules |
| `show` | print TSV tables aligned (`show -t` = one column per diagram) |
| `structinfo`, `inputs.tsv` | per input: edges across a subgraph border, open / closed subgraphs |
| `in/*.mmd` | the 8 inputs (the analog + every flowchart/state input with subgraphs in the three corpus dirs) |
| `fm038-svg/`, `mermaid-svg/` | their renders: flowmaid 0.38.0 CLI, mermaid 12.2.0 default config (`tools/mmrender.js`) |
| `baseline-038.tsv`, `mermaid.tsv` | **the baseline**: the two tables |
| `selftest/` | three hand-checked drawings in both SVG dialects, `expected.json` worked out by hand |
| `validation/` | outputs of the cross-checks described under "Validation" |
| `overlay/` | the analog with the findings drawn in (red = run through a foreign frame, magenta = crossing, blue = label to edge midpoint) |
| `extra/` | a wider sweep (49 flowchart/state inputs): `sweep-038.tsv`, `sweep-mermaid.tsv`, `in/` |

## Use

```sh
M=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/metrics

# judge a build: render the 8 inputs with its CLI, measure, compare with the rules
$M/run.sh fm /path/to/target/release/flowmaid /some/outdir b12
$M/judge /some/outdir/metrics.tsv            # exit 1 when a `must` fails; --all lists every rule
$M/show -t /some/outdir/metrics.tsv          # look at the numbers

# one file, with everything it found
$M/blockmetrics --struct $M/struct/infra-overview.struct.json --detail --overlay /some/outdir x.svg

# a new input: make its structure file once (any flowmaid version that parses it the same)
$M/bin/structdump-038 new.mmd > $M/struct/new.struct.json
# ... or against the tree under test (also refreshes the `scene` part for --check-scene):
cp $M/structdump.rs <tree>/examples/ && (cd <tree> && cargo build --release --example structdump)

# is the SVG parser still reading the painter right? compare with the scene of the SAME build
<target>/release/examples/structdump in.mmd > /tmp/in.struct.json
$M/blockmetrics --struct /tmp/in.struct.json --check-scene in.svg

$M/run.sh mermaid /some/outdir               # the mermaid side (NODE_PATH=$(npm root -g))
$M/blockmetrics --selftest                   # 534 checks against hand-worked numbers
```

`IN=<dir> run.sh ...` measures another directory of `.mmd` files (missing structure files
are made on the fly with `bin/structdump-038`), e.g. `IN=$M/extra/in`.

## What is read from the SVG, and what is not in it

**flowmaid** (`scene::to_svg`, flowcharts and state diagrams). No ids or classes, so
elements are told apart by where they sit and what they carry:

| element | how it is recognised |
|---|---|
| canvas | root `viewBox="0 0 W H"` |
| frame | top-level `<rect … rx="8" … stroke-width="1.4">` before the first edge; its title = the `<text font-weight="bold">` lines after the chip `<rect fill-opacity="0.9">` at frame + (6, 5) |
| edge | top-level `<path d=…>` with a `<title>` child (`from → to[: label]`); everything inside `<defs>` (markers, crossing masks) is skipped |
| node | each top-level `<g>` (optionally inside `<a>`); its box = the union of the shapes in it (rect, circle, polygon, path with arcs, line); order = `Graph::nodes` order |
| edge label | top-level `<rect … rx="4" stroke=…>` + the following `<text>`; the k-th chip belongs to the k-th labelled edge |

**mermaid** (`flowchart-v2` and `stateDiagram-v2` output of 12.2): `g.cluster > rect`
(state: `g.statediagram-cluster rect.outer`; `note-cluster` is not a frame),
`g.node[transform]` (id `…-flowchart-<id>-<n>` / `…-state-<id>-<n>`),
`path[data-edge]` (id `L_<from>_<to>_<n>`; `edge-thickness-invisible` = a `~~~` link,
skipped), `g.edgeLabel > g.label[data-id] > foreignObject[width,height]`. Nested
`transform`s are accumulated, so the recursive `g.root` of the dagre renderer works too.

**Not in either SVG**: which subgraph a node belongs to, a frame's parent, and (flowmaid)
the node ids an edge joins. Two ways to get them:

* `--struct file.struct.json` (recommended, `membership = struct`): the structure file from
  `structdump`. flowmaid nodes are matched by order, mermaid nodes by id; a flowmaid frame
  is matched to the subgraph whose nodes all lie in it (smallest frame first, most nodes,
  title, depth); flowmaid edges by order (drawn edges of `Graph::edges`, then
  `Graph::sub_edges`), mermaid edges by the ids in `L_from_to_n` (state diagrams, whose
  edge ids are `edgeN`: by the drawn ends). Every mismatch is a warning (`warn` column).
* nothing (`membership = geom`): a node belongs to the smallest frame holding its centre, a
  frame's parent is the smallest frame containing it, an edge end is the node within 14 px
  of the path's end (else the frame whose border is within 6 px). Pass `--dir`; without it
  the direction is guessed and shown with a `?`.

On all 16 baseline SVGs the two modes give identical numbers (see Validation). The
structure file matters when a layout is *wrong*: only with it can `frame_foreign_nodes`
see a node sitting in a frame it is not a member of.

## Columns

Lengths are px in the SVG's user space. Flow axis = y for TD/BT, x for LR/RL.
An edge is its drawn path, flattened (curves sampled every px, crossing hops bridged —
see `hops`); a flowmaid path stops 5.5 px short under an arrowhead.

| column | meaning |
|---|---|
| `name`, `engine`, `dir` | input name; `fm` / `mermaid` (or `--label`); flow direction (`TD?` = guessed) |
| `W`, `H`, `area`, `aspect` | canvas (viewBox) width, height, W·H, W/H |
| `fill` | sum of node box areas / canvas area — scale-free "how full is the canvas" |
| `nodes`, `edges`, `frames`, `top_frames` | drawn nodes, drawn edges, subgraph frames, frames without a parent |
| `compact_mean`, `compact_max`, `compact_max_id` | per frame: **frame area / sum of the box areas of all nodes inside it** (members of nested frames included); mean and max over frames, and which frame is the max. 1.0 would be a frame with no air at all. A one-row block of flowmaid's text-sized nodes is about 2–3.5 |
| `span_max`, `span_max_id` | largest share of the canvas a single frame takes along the flow axis (frame extent / canvas extent) |
| `sib_pairs` | pairs of frames with the same parent (top level counts) |
| `sib_side` / `sib_stacked` | of those: pairs whose projections on the flow axis overlap by more than 1 px (**side by side**) / do not (**stacked**) |
| `sib_stagger` | side-by-side pairs where neither projection contains the other (2 px slack): interleaved, not one aligned band |
| `sib_side_linked` | side-by-side pairs that an edge joins (one end in or on each frame). In a block layout such a pair is stacked |
| `frame_overlap` | sibling frames whose rectangles overlap (> 0.5 px on both axes) |
| `frame_foreign_nodes` | nodes whose centre lies in a frame they are not a member of (needs `--struct`) |
| `xframe_runs`, `xframe_len`, `xframe_edges` | **runs through a frame that holds neither end**: maximal stretches of an edge at least 0.5 px inside such a frame and at least 2 px long; their number, total length (overlapping frames counted once) and the number of edges with at least one. A frame *holds* an end when the end node is inside it at any depth, or the edge ends on that frame itself; of a foreign nest only the outermost frame counts |
| `node_hits` | (edge, node) pairs where the edge runs ≥ 2 px inside the box (1 px in) of a node that is not one of its ends |
| `crossings` | points where one edge passes from one side of another to the other. Two strokes within 0.5 px are "in contact"; a contact is a crossing when the edge is on different sides 2 px before and after it. A shared trunk, a T-touch and a common end are contacts without a side change; a contact reaching either edge's own end is a touch |
| `hops` | crossing hops drawn in the paths (a detour ≤ 26 px long, ≤ 9 px off its line, that returns to the line, which then carries straight on; back-to-back hops are chained). Hops are replaced by their chord before anything is measured, so they neither add length nor break a straight run. mermaid 12.2 draws them; flowmaid will from B11 |
| `edge_len` | total length of all edges |
| `edge_len_n` | `edge_len` / side of the average node box (√(node area / nodes)) — comparable across engines, whose boxes differ in size |
| `straight_max`, `straight_max_edge` | the longest straight run (a run continues while a segment turns < 1.5° and stays within 0.75 px of the run's line) and its edge |
| `bends` | per edge, (straight runs ≥ 8 px) − 1, summed. Auxiliary: meaningless for splines |
| `lanes_max` | the most parallel lines one perpendicular cut crosses with no gap wider than 26 px between neighbours (axis-aligned straight runs ≥ 40 px only) — the widest bundle |
| `lane_gap_min` | smallest distance between two parallel straight runs of different edges that overlap for ≥ 40 px |
| `lbl_n` | labelled edges |
| `lbl_off_mean`, `lbl_off_max` | **distance from the label to the edge's midpoint as a fraction of the edge's length**: \|s − L/2\| / L, s = position along the path of the point nearest the label's centre. 0 = at the middle, 0.5 = at an end |
| `lbl_endpx_mean`, `lbl_endpx_max` | px along the edge from the label to its nearer end, min(s, L − s) |
| `lbl_detach_max` | largest distance from a label's centre to its own edge |
| `lbl_lbl` | pairs of label boxes that overlap (> 0.5 px on both axes) |
| `lbl_node` | (label, node) overlaps with a node that is not an end of the label's edge |
| `lbl_struck` | (label, other edge) pairs where the other edge runs ≥ 2 px through the label box (1 px in) |
| `lbl_gap_min` | smallest distance from any label box to a foreign edge (0 = touched or struck) |
| `lbl_crowd` | labels with a foreign edge nearer than 12 px to their box (flowmaid's `CLEAR_GAP`): labels in a corridor that cannot be attributed at a glance |
| `membership` | `struct`, `geom`, or `struct+Ngeom` (N nodes the structure file does not know, e.g. mermaid's start/end pseudo-states) |
| `edge_ends` | how edge ends were resolved: `struct`, `id`, `geom`, `id+geom` |
| `warn` | warnings (stderr; all of them with `--detail`). Anything but 0 means: look before trusting the row |

`--detail` prints the per-frame, per-sibling-pair, per-edge, per-run, per-crossing and
per-label tables behind every column; `--overlay DIR` draws them into a copy of the SVG.

The tolerances are constants at the top of `blockmetrics`.

## Inputs

The three corpus directories hold the same files under several names (identical md5);
each appears once here.

| name | paths | what `structinfo` says (`inputs.tsv`) |
|---|---|---|
| infra-overview | `b12/in/` (the sanitized analog) | TD, 5 subgraphs, 27 edges: 18 across a border, 3 end on a subgraph; all 5 subgraphs open; 8 of 10 sibling pairs joined by an edge |
| advanced | `tree/examples`, `cmp/corpus` | TD, 3 subgraphs (1 nested): 3 edges across a border (all into/out of `Workers`), 4 end on a subgraph |
| subgraph | `tree/examples`, `cmp/corpus` | TD, 2 subgraphs (nested): 4 of 8 edges across a border |
| state | `tree/examples`, `cmp/corpus` | TD, 1 composite with `direction LR`: **no** edge across its border, 2 end on it |
| bench-75 | `tree/tests/fixtures/quality`, `cmp/corpus` | LR, 5 subgraphs: 69 of 75 edges across a border |
| dense-lr | `tree/tests/fixtures/quality`, `cmp/corpus` | LR, 11 subgraphs (6 nested): 28 of 42 across a border, 2 end on a subgraph |
| dense-td | `tree/tests/fixtures/quality`, `cmp/corpus` | TD, 10 subgraphs (7 nested): 30 of 47 across a border, 3 end on a subgraph |
| fanout-frames | `tree/tests/fixtures/quality`, `cmp/corpus` | LR, 6 subgraphs with `direction`: **no** edge across a border, 6 end on a subgraph |

"Open" = at least one node-to-node edge crosses the subgraph's border. 0.38 lays a
subgraph out on its own only when it is closed *and* has a `direction` (state, fanout-frames,
`CI` in advanced); every other subgraph above is what B12 is about.

## Baseline (0.38.0 vs mermaid 12.2.0)

The analog, `infra-overview` (all columns in the two `.tsv` files):

| metric | flowmaid 0.38.0 | mermaid 12.2.0 |
|---|---|---|
| canvas `W` × `H` (`area`) | 2787 × 1225 (3.41 Mpx²) | 2199 × 2358 (5.18 Mpx²) |
| `aspect`, `fill` | 2.28, 0.081 | 0.93, 0.071 |
| `compact_mean` / `compact_max` | 6.90 / 11.43 (EXT) | 3.25 / 4.59 (PROD) |
| `span_max` | 0.767 (EXT) | 0.308 (PROD) |
| `sib_side` / `sib_stacked` (of 10 pairs) | 6 / 4 | 2 / 8 |
| `sib_stagger`, `sib_side_linked` | 3, 5 | 0, 0 |
| `xframe_runs` / `xframe_len` / `xframe_edges` | 3 / 2012 px / 3 | 0 / 0 / 0 |
| `crossings` (`hops`) | 24 (0) | 16 (16) |
| `edge_len` (`edge_len_n`) | 22350 px (199.4) | 30835 px (237.7) |
| `straight_max` | 1175 (PG→DRAPP) | 1236 (R2→STG) |
| `lanes_max`, `lane_gap_min` | 6, 9.3 px | 6, 12.0 px |
| `lbl_off_mean` / `lbl_off_max` (20 labels) | 0.198 / 0.332 | 0.130 / 0.329 |
| `lbl_endpx_mean` / `lbl_endpx_max` | 309 / 753 px | 558 / 986 px |
| `lbl_lbl`, `lbl_node`, `lbl_struck` | 0, 0, 0 | 0, 0, 0 |
| `lbl_gap_min`, `lbl_crowd` | 2.5 px, 8 | 20.0 px, 0 |
| `frame_overlap`, `node_hits`, `warn` | 0, 0, 0 | 0, 0, 0 |

What the numbers say about the user's complaint:

* *Blocks.* 0.38 frames are 2–2.5× looser than mermaid's and one of them (EXT) spans 77 %
  of the drawing's height; the four frames below "Code hosting" all overlap along the flow
  (6 side-by-side pairs, 5 of them joined by an edge, 3 interleaved). mermaid has two
  side-by-side pairs, exactly the two pairs no edge joins (EXT | GH and DR | STG), and no
  interleaving. In the user's own mermaid render only the first of these two pairs is side
  by side; the three servers are stacked.
* *Edges in the gaps.* 0.38 runs three edges 630–690 px through a frame that holds neither
  end; mermaid none — on the analog and on every one of the 36 inputs with frames in
  `extra/` (0.38: 45 such runs on 10 of them).
* *Labels.* Distance to the ends does **not** separate the engines (mermaid's labels are
  farther from their ends than 0.38's), and neither does the width of the corridor
  (`lanes_max` 6 and 6). What does: 8 of 0.38's 20 chips have a foreign line within
  12 px (nearest 2.5 px — a 20 px chip centred on a lane with neighbours at flowmaid's
  12 px pitch), none of mermaid's (nearest 20 px). `lbl_off_mean` is a third lower in
  mermaid.
* *Where 0.38 is ahead and must stay ahead:* canvas area (0.66× mermaid's) and edge
  length; on dense-lr / dense-td mermaid's canvas is 2.4× larger.

The other inputs (subset of columns; `NA` = no labels):

| input | engine | W×H | compact mean / max | span_max | side / stacked / stagger / side_linked | xframe runs (px) | crossings | edge_len | lbl_struck / lbl_crowd |
|---|---|---|---|---|---|---|---|---|---|
| advanced | fm 0.38 | 596×1236 | 5.75 / 8.28 | 0.377 | 0 / 1 / 0 / 0 | 0 | 0 | 2346 | 0 / 0 |
| advanced | mermaid | 906×1484 | 5.33 / 8.24 | 0.428 | 0 / 1 / 0 / 0 | 0 | 0 | 5024 | 0 / 0 |
| bench-75 | fm 0.38 | 2222×1965 | 35.88 / 52.03 | 0.464 | 4 / 6 / 2 / 4 | 4 (2508) | 302 | 80453 | 0 / 0 |
| bench-75 | mermaid | 4327×1058 | 5.33 / 9.88 | 0.161 | 0 / 10 / 0 / 0 | 0 | 228 | 106034 | 0 / 0 |
| dense-lr | fm 0.38 | 2302×3468 | 8.76 / 11.78 | 0.603 | 7 / 4 / 4 / 5 | 8 (4081) | 46 | 38537 | 5 / 10 |
| dense-lr | mermaid | 4959×3858 | 3.74 / 6.92 | 0.195 | 1 / 10 / 0 / 0 | 0 | 10 | 78383 | 0 / 0 |
| dense-td | fm 0.38 | 4249×1806 | 12.45 / 24.66 | 0.795 | 3 / 1 / 3 / 3 | 12 (14713) | 80 | 63286 | 1 / 12 |
| dense-td | mermaid | 4131×4378 | 8.77 / 15.12 | 0.308 | 0 / 4 / 0 / 0 | 0 | 68 | 142183 | 0 / 2 |
| fanout-frames | fm 0.38 | 566×2311 | 2.88 / 3.37 | 0.462 | 15 / 0 / 0 / 0 | 0 | 0 | 4564 | 0 / 0 |
| fanout-frames | mermaid | 648×2271 | 2.24 / 2.66 | 0.618 | 15 / 0 / 0 / 0 | 0 | 0 | 4253 | 0 / 0 |
| state | fm 0.38 | 535×873 | 5.35 / 5.35 | 0.105 | – | 0 | 0 | 1674 | 0 / 0 |
| state | mermaid | 649×886 | 4.78 / 4.78 | 0.123 | – | 0 | 0 | 2139 | 0 / 0 |
| subgraph | fm 0.38 | 382×758 | 6.28 / 7.49 | 0.562 | – | 0 | 0 | 632 | 0 / 0 |
| subgraph | mermaid | 466×749 | 5.08 / 5.82 | 0.590 | – | 0 | 0 | 843 | 0 / 0 |

mermaid has `xframe_runs = 0`, `sib_side_linked = 0` and `sib_stagger = 0` on all eight.
Both engines draw the same number of nodes, edges and frames on every input (also on the
49 of `extra/`).

## Proposed acceptance rules (`thresholds.json`, checked by `judge`)

`must` = acceptance, `target` = "as tidy as mermaid". On the analog:

| metric | 0.38 | mermaid | must | target | why |
|---|---|---|---|---|---|
| `xframe_runs` | 3 | 0 | = 0 | | edges between blocks run in the gaps |
| `sib_side_linked` | 5 | 0 | = 0 | | blocks an edge joins are stacked along the flow (implies ≥ 8 of 10 pairs stacked) |
| `sib_stagger` | 3 | 0 | = 0 | | blocks sharing a band are aligned |
| `compact_max` | 11.43 | 4.59 | ≤ 5.5 | ≤ 4.6 | every subgraph one compact block |
| `compact_mean` | 6.90 | 3.25 | ≤ 4.0 | ≤ 3.3 | |
| `span_max` | 0.767 | 0.308 | ≤ 0.40 | ≤ 0.31 | no frame spans the drawing |
| `crossings` | 24 | 16 | ≤ 24 | ≤ 16 | |
| `lbl_off_mean` | 0.198 | 0.130 | ≤ 0.20 | ≤ 0.13 | |
| `lbl_off_max` | 0.332 | 0.329 | ≤ 0.35 | | |
| `lbl_crowd` | 8 | 0 | ≤ 4 | = 0 | labels attributable at a glance |
| `lbl_gap_min` | 2.5 | 20.0 | ≥ 2.5 | ≥ 12 | |
| `lane_gap_min` | 9.3 | 12.0 | ≥ 8 | ≥ 12 | flowmaid's `EDGE_SEP_MIN` / `EDGE_SEP` |
| `straight_max` | 1175 | 1236 | ≤ 1240 | | |
| `area` | 3.41 M | 5.18 M | ≤ mermaid's | ≤ 1.25 × 0.38 | the compact canvas stays a flowmaid strength |
| `edge_len` | 22350 | 30835 | ≤ mermaid's | | |
| `edge_len_n` | 199.4 | 237.7 | | ≤ mermaid's | |
| `aspect` | 2.28 | 0.93 | | 0.5 … 2.0 | |
| `lbl_lbl`, `lbl_node`, `lbl_struck`, `frame_overlap`, `frame_foreign_nodes`, `node_hits`, `warn` | 0 | 0 | = 0 | | |

The compactness bounds leave mermaid's values ~20 % of slack on purpose: flowmaid's nodes
are text-sized, so the same padding and gaps weigh more against a smaller node area.

On every other input (scope `*`): the zero rules above; `xframe_runs`, `sib_side_linked`,
`sib_stagger`, `lbl_struck`, `lbl_crowd`, `compact_max` not above 0.38's (target: 0, 0, 0,
0, –, ≤ 1.2 × mermaid's); `crossings` ≤ 1.1 × 0.38's + 2 (target ≤ mermaid's); `area`
≤ mermaid's (target ≤ 1.25 × 0.38's); `edge_len` ≤ the larger of mermaid's and 1.1 × 0.38's.

Sanity of the rules themselves: `judge baseline-038.tsv` reports 7 must-fails, all on the
analog (`xframe_runs`, `sib_side_linked`, `sib_stagger`, `compact_max`, `compact_mean`,
`span_max`, `lbl_crowd`); `judge mermaid.tsv` reports none (it misses only the
0.38-relative `area` / `edge_len` targets).

Not measured here and still part of B12's bar: diagrams *without* subgraphs must not
change (compare the SVG bytes), and the PLAN invariants (drag speed, `route` /
`route_partial`, determinism, …).

## Validation

Every number was checked in two ways: on drawings simple enough to measure by hand, and
against an independent measurement of real output.

### 1. Hand-checked drawings (`selftest/`, `blockmetrics --selftest`: 534 checks, all pass)

`selftest/gen.py` writes three drawings, each in the flowmaid dialect and in the mermaid
dialect (the latter with nested `transform`s and, in `hand`, a hop arc), plus a
hand-written structure file. `expected.json` holds numbers worked out by hand from the
coordinates in `gen.py`; each drawing is run with and without the structure file
(12 runs). `hand-fm.png` etc. show them.

**`hand`** (TD, 600 × 400). Frames F1 (20,20 200×100), F2 (300,20 200×100), F3 (20,220
480×120). Nodes A, B (80×40) in F1, C (100×40) in F2, D, E (80×40) in F3, T (40×40) free.
Edges: e1 A→D straight down x=80 (90→270); e2 C→D (400,90)→(400,165)→(70,165)→(70,270);
e3 B→E (170,50)→(170,35)→(520,35)→(520,290)→(460,290); e4 D→E (140,290)→(380,290);
e5 A→B (120,70)→(130,70).

| number | by hand |
|---|---|
| `compact_*` | F1 20000/(2·3200) = 3.125, F2 20000/4000 = 5, F3 57600/6400 = 9 → mean 5.7083, max 9 (F3) |
| `span_max` | F3: 120/400 = 0.30 |
| sibling pairs | F1–F2 share y 20..120 → side; F1–F3, F2–F3 disjoint → stacked: 1 / 2, stagger 0; no edge joins F1 and F2 → `sib_side_linked` 0 |
| `xframe_*` | e3 runs along y=35 through F2 (x 300..500) which holds neither B nor E: 200 − 2·0.5 inset = 199 px, 1 run, 1 edge |
| `crossings` | e2's leg y=165 crosses e1 (x=80) at (80,165): 1. Nothing else meets |
| `hops` | mermaid dialect: e2 is drawn `…L86,165 A6,6 0 0 0 74,165 L70,165…`: 1 hop, same length and crossing count |
| `edge_len` | 180 + (75+330+105) + (15+350+255+60) + 240 + 10 = 1620 |
| `straight_max`, `bends` | e3's 350; bends 0+2+3+0+0 = 5 |
| `lanes_max`, `lane_gap_min` | e2's leg x=70 (165..270) beside e1 x=80: 2 lanes, 10 px |
| `lbl_off_*` | e1 chip at s=90 of 180 → 0; e2 chip (105,165): s = 75+295 = 370 of 510 → 0.2255; e3 chip (520,170): s = 15+350+135 = 500 of 680 → 0.2353; e5 → 0. mean 0.1152, max 0.2353 |
| `lbl_endpx_*` | 90, 140, 180, 5 → mean 103.75, max 180 |
| `lbl_lbl` | e1's chip (60..100 × 170..190) and e2's (80..130 × 155..175) overlap 20 × 5: 1 |
| `lbl_node` | e3's chip (490..550 × 160..180) overlaps T (540..580 × 150..190): 1. e5's chip overlaps A and B, its own ends: not counted |
| `lbl_struck` | e2's leg x=70 runs through e1's chip: 1 |
| `lbl_gap_min`, `lbl_crowd` | e1's chip struck (0), e2's touched by e1 at x=80 (0), e3's 90, e5's √(30²+10²) = 31.6 → min 0, crowd 2 |
| `fill`, `edge_len_n` | 18400/240000 = 0.0767; 1620/√(18400/6) = 29.25 |

**`hand2`** (LR, 500 × 300): a nest (P holds K), an edge that ends on a frame, an edge
through the foreign nest, a staggered pair, a node hit. P (20,20 200×200) ⊃ K (40,60
100×80); Q (260,100 100×150); R (300,10 150×70). Nodes K1 in K, P1 in P, Q1, R1, free S, U, V.
Edges U→R1 (130,250)→(130,45)→(350,45); K1→P1 (90,120)→(90,180)→(150,180);
Q1→S (340,160)→(460,160)→(460,200); S→P (440,220)→(400,220)→(400,190)→(220,190).

| number | by hand |
|---|---|
| `frames`, `top_frames` | 4, 3 |
| `compact_*` | P 40000/4400 = 9.0909 (K1 counts for P too), K 8000/2400 = 3.3333, Q 6.25, R 4.375 → mean 5.7623, max 9.0909 (P) |
| `span_max` | P: 200/500 = 0.40 (x is the flow axis) |
| sibling pairs | P–Q, P–R disjoint in x → stacked 2; Q (260..360) and R (300..450) overlap, neither contains the other → side 1, stagger 1 |
| `xframe_*` | S→P at y=190 through Q (x 260..360): 99 px. U→R1 inside P from (130,219.5) to (130,45) to (219.5,45): 174.5 + 89.5 = 264 px, one run; K lies in foreign P and is not counted again. 2 runs, 363 px, 2 edges |
| edge that ends on a frame | S→P stops on P's border (220,190): P holds that end, so nothing of it is foreign to P |
| `node_hits` | U→R1 at y=45 runs through V (240..280 × 30..60): 1 |
| `crossings` | K1→P1's leg y=180 crosses U→R1 (x=130): 1 |
| `edge_len`, `straight_max`, `bends` | 425 + 120 + 160 + 250 = 955; 220; 1+1+1+2 = 5 |
| `lanes_max`, `lane_gap_min` | no two lanes within 26 px → 1; y=160 (340..460) and y=190 (220..400) overlap 60 px, 30 px apart → 30 |
| label | "to P" at (350,190): s = 40+30+50 = 120 of 250 → off 0.02, endpx 120; nearest foreign line y=160 → gap 20; crowd 0 |

**`hand3`** (TD, 320 × 260): A1 | B1 side by side with an edge a→b between them, C1
under both: `sib_side` 1, `sib_stacked` 2, **`sib_side_linked` 1**; `compact` 8000/1800 =
4.444 (A1, B1) and 22400/1800 = 12.444 (C1); `edge_len` 120 + 200 = 320.

Kernel unit checks in the same selftest: length of a quarter circle arc (r = 100) = 50π;
a cubic and a quadratic of known length; relative path commands; a plain X and a `+`
(1 crossing each); a T-touch (0); a trunk two edges share and leave to opposite sides (0);
a trunk joined from the left and left to the left (0) / to the right (1); an S through a
line (2); a hop arc over a line (1); two parallel lines 1 px apart (0); a rounded corner
crossing (1); hop bridging: one arc, two arcs back to back, a cubic bump (bridged), a Z
jog and a bump that then turns away (kept); a run clipped to a frame (30 − 1 = 29); a run
exactly on a frame's border (outside).

### 2. Against independent measurements of real output

| what | checked against | result |
|---|---|---|
| node boxes, frames, label chips, canvas read from the flowmaid SVG | the `scene` in the structure file (`--check-scene`, `validation/check-scene-038.txt`) | all 8 inputs: ≤ 0.1 px (the SVG prints one decimal); cylinders ≤ 1 px — their arc radii are printed to 0.1 px, so the drawn drum really is that much shorter |
| drawn path length | length of the scene's waypoint polyline | straight edges: exactly 5.5 px shorter (the arrowhead trim: 77.0 → 71.5, 80.5 → 75.0, 85.5 → 80.0); routed edges 13–15 px shorter on average (trim + ~3.9 px per rounded corner; U1→DNS, 2 corners: 588.3 → 575.1) |
| `crossings`, `xframe_edges`, `lbl_struck`, canvas | flowmaid's own harness, `examples/quality.rs` in static mode: `x_drawn`, `edge_frame_x`, `lbl_edge`, `width` × `height` — another implementation, in Rust, on the Scene (`validation/quality-crosscheck-038.txt`) | equal on all 7 flowchart inputs: crossings 0 / 302 / 46 / 80 / 0 / 24 / 0, edges through a foreign frame 0 / 3 / 8 / 11 / 0 / 3 / 0, struck chips 0 / 0 / 5 / 1 / 0 / 0 / 0 |
| `compact_*`, `span_max`, `sib_*` | recomputed in python from the scene dump, sharing no code with blockmetrics (`validation/recompute.py`, `recompute-038.txt`) | all 8 inputs agree (compactness within the cylinder rounding above) |
| structure-file mode vs geometry-only mode | each other | every numeric cell identical on all 16 baseline SVGs |
| mermaid frames | the frame table another agent extracted with its own script (`b12/u-mermaid/frag-analog.md`: EXT 1009×201, GH 402×159, PROD 579×726, DR 604×285, STG 472×633, canvas 2199×2358, bands [EXT GH] / [PROD] / [DR STG]) | same boxes (its coordinates have the viewBox origin 4,4 subtracted), same bands: 2 side-by-side pairs, 8 stacked |
| mermaid `crossings` | the hop arcs mermaid itself draws | analog: 16 crossings, 16 hops. dense-lr: 10 crossings, 9 hops; the tenth, looked at in the overlay, is a real crossing right before a corner of the crossing edge, where mermaid draws no hop. bench-75 228 / 168, dense-td 68 / 60 |
| analog, by eye | `overlay/infra-overview.fm038.overlay.png`, `…mermaid.overlay.png` | the 3 red runs are DNS→TR2 and DNS→PSPROD through PROD and PG→DRAPP through EXT; every magenta ring sits on a visible crossing; the dashed frame and node boxes sit on the drawn ones |
| a label, by hand | U1→DNS in 0.38: legs 28.5 + 413.6 + 140.6 minus two rounded corners = 575.1; the chip at x=1338.5 on the y=151.5 leg is 28.5 + 302.1 − 3.8 = 326.8 along; \|326.8 − 287.5\| / 575.1 = 0.068 | the table says 0.068 |
| reproducibility | a second mermaid render, a second 0.38 render | identical rows / identical SVG bytes; `run.sh` reproduces both `.tsv` files byte for byte |
| robustness | 49 flowchart/state inputs (`extra/in`: examples, test fixtures incl. nested composites and notes, the analog's permutations, probes), both engines | no failure, no warning on mermaid; on 0.38 three inputs warn, all for one real defect (below) |

### Known limits

* A `-->` from a subgraph to itself (`A --> A`) and a composite's self-transition are drawn
  by 0.38 as a zero-length path (`M 69 230 C 69 230, …`). blockmetrics warns and leaves the
  edge out of the label metrics. Three `extra/` inputs do this.
* Frames are recognised by `stroke-width="1.4"`; if the painter changes that, the row gets
  warnings (`frame … matches no subgraph`), not silent nonsense.
* `lanes_max` / `lane_gap_min` look at axis-aligned straight runs only.
* `edge_len` counts a trunk shared by k bundled edges k times (each edge has its own path).
* Membership from geometry cannot see a node placed inside a foreign frame — use `--struct`
  when judging a new layout.
* mermaid's numbers are for its default configuration as `tools/mmrender.js` runs it. Its
  boxes are larger than flowmaid's (average node side 1.16× on the analog); use `fill`,
  `edge_len_n` and the ratios for cross-engine comparison, px for flowmaid-vs-flowmaid.
