#### state.dagre.svg  (canvas 804 x 873, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| Validating | 0 | - | 181 | 180 | 615 | 107 | 12% | 4 (4 incl. nested) | 4 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 22 | 14 | root_start | 50 |
| 1 | 72 | 109 | 37 | Idle | 71 |
| 2 | 180 | 287 | 107 | [Validating] | 71 |
| 3 | 358 | 395 | 37 | decide | 71 |
| 4 | 466 | 503 | 37 | Rejected , forked | 75 |
| 5 | 578 | 629 | 51 | Rejected----note , Notify , Persist | 75 |
| 6 | 704 | 714 | 10 | joined | 50 |
| 7 | 764 | 801 | 37 | Done | 50 |
| 8 | 851 | 865 | 14 | root_end | - |

Top-level blocks: 1; bands: 9; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 4: Rejected -> forked = 262; band 5: Rejected----note -> Notify = 162; band 5: Notify -> Persist = 50.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 1.

Edges: 16 total, 3 with both ends in the same top-level block (intra) and 13 between different top-level items (inter); 2 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 1-2 | 2 | 274 | 2 (126, 127) |
| gap 3-4 | 1 | 254 | 2 (413, 448) |
| gap 5-6 | 2 | 163 | 1 (692) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| edge0 | root_start (root_start) | Idle (Idle) | 0 -> 1 | straight |
| edge6 | decide (decide) | Rejected (Rejected) | 3 -> 4 | gap 3-4 136; gap 3-4 118 |
| edge7 | decide (decide) | forked (forked) | 3 -> 4 | straight |
| edge8 | forked (forked) | Persist (Persist) | 4 -> 5 | straight |
| edge9 | forked (forked) | Notify (Notify) | 4 -> 5 | straight |
| edge10 | Persist (Persist) | joined (joined) | 5 -> 6 | gap 5-6 81 |
| edge11 | Notify (Notify) | joined (joined) | 5 -> 6 | gap 5-6 81 |
| edge12 | joined (joined) | Done (Done) | 6 -> 7 | straight |
| edge13 | Rejected (Rejected) | Idle (Idle) | 4 -> 1 | gap 1-2 156 |
| Rejected-Rejected----note-14 | Rejected (Rejected) | Rejected----note (Rejected----note) | 4 -> 5 | straight |
| edge15 | Done (Done) | root_end (root_end) | 7 -> 8 | straight |
| edge1 | Idle (Idle) | Validating (Validating) | 1 -> 2 | gap 1-2 118 |
| edge5 | Validating (Validating) | decide (decide) | 2 -> 3 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| edge6 | 40x21 | gap 3-4 | 1 | 113 | 0.55 | - |
| edge7 | 29x21 | gap 3-4 | 3 | 29 | 0.58 | - |
| edge13 | 28x21 | gap 2-3 | 2 | 143 | 0.32 | - |
| edge1 | 41x21 | gap 1-2 | 11 | 23 | 0.83 | - |
| edge3 | 15x21 | band 2 inside [Validating] | 0 | 39 | 0.53 | - |

Labels: 5; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.70 Mpx2, edge length 1868 px, bends 25, edge crossings (drawn polylines) 0.

