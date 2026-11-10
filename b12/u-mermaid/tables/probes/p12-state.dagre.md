#### p12-state.dagre.svg  (canvas 222 x 1057, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| Work | 0 | - | 8 | 180 | 206 | 423 | 40% | 4 (4 incl. nested) | 1 / 1 / 1 / 1 |
| Review | 0 | - | 8 | 761 | 206 | 224 | 21% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 22 | 14 | root_start | 50 |
| 1 | 72 | 109 | 37 | Idle | 71 |
| 2 | 180 | 603 | 423 | [Work] | 71 |
| 3 | 674 | 711 | 37 | Done | 50 |
| 4 | 761 | 985 | 224 | [Review] | 50 |
| 5 | 1035 | 1049 | 14 | root_end | - |

Top-level blocks: 2; bands: 6; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 2, leaves via bottom: 2, leaves via top: 1.

Edges: 10 total, 4 with both ends in the same top-level block (intra) and 6 between different top-level items (inter); 5 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| edge0 | root_start (root_start) | Idle (Idle) | 0 -> 1 | straight |
| edge1 | Idle (Idle) | Work (Work) | 1 -> 2 | straight |
| edge5 | Work (Work) | Done (Done) | 2 -> 3 | straight |
| edge6 | Work (Work) | Idle (Idle) | 2 -> 1 | straight |
| edge8 | Done (Done) | Review (Review) | 3 -> 4 | straight |
| edge9 | Review (Review) | root_end (root_end) | 4 -> 5 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| edge1 | 16x21 | gap 1-2 | 3 | 28 | 0.58 | - |
| edge5 | 15x21 | gap 2-3 | 0 | 30 | 0.55 | - |
| edge6 | 40x21 | gap 1-2 | 3 | 34 | 0.50 | - |
| edge3 | 26x21 | band 2 inside [Work] | 3 | 45 | 0.50 | - |

Labels: 4; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.23 Mpx2, edge length 634 px, bends 10, edge crossings (drawn polylines) 0.

