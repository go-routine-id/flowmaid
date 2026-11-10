#### p12-state.elk.svg  (canvas 200 x 901, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| Work | 0 | - | 8 | 200 | 184 | 291 | 32% | 4 (4 incl. nested) | 1 / 1 / 1 / 1 |
| Review | 0 | - | 8 | 669 | 184 | 170 | 19% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 22 | 14 | root_start | 40 |
| 1 | 62 | 99 | 37 | Idle | 101 |
| 2 | 200 | 491 | 291 | [Work] | 101 |
| 3 | 592 | 629 | 37 | Done | 40 |
| 4 | 669 | 839 | 170 | [Review] | 40 |
| 5 | 879 | 893 | 14 | root_end | - |

Top-level blocks: 2; bands: 6; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 2, leaves via bottom: 2, leaves via top: 1.

Edges: 10 total, 4 with both ends in the same top-level block (intra) and 6 between different top-level items (inter); 5 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 1-2 | 2 | 16 | 1 (119) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| edge0 | root_start (root_start) | Idle (Idle) | 0 -> 1 | straight |
| edge1 | Idle (Idle) | Work (Work) | 1 -> 2 | gap 1-2 8 |
| edge5 | Work (Work) | Done (Done) | 2 -> 3 | straight |
| edge6 | Work (Work) | Idle (Idle) | 2 -> 1 | gap 1-2 8 |
| edge8 | Done (Done) | Review (Review) | 3 -> 4 | straight |
| edge9 | Review (Review) | root_end (root_end) | 4 -> 5 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| edge1 | 16x21 | gap 1-2 | 2 | 47 | 0.53 | - |
| edge3 | 26x21 | band 2 inside [Work] | 0 | 29 | 0.55 | - |
| edge5 | 15x21 | gap 2-3 | 3 | 48 | 0.50 | - |
| edge6 | 40x21 | gap 1-2 | 3 | 47 | 0.53 | - |

Labels: 4; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.18 Mpx2, edge length 519 px, bends 4, edge crossings (drawn polylines) 0.

