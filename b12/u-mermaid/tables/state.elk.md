#### state.elk.svg  (canvas 649 x 886, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| Validating | 0 | - | 8 | 200 | 459 | 109 | 12% | 4 (4 incl. nested) | 4 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 22 | 14 | root_start | 40 |
| 1 | 62 | 99 | 37 | Idle | 101 |
| 2 | 200 | 309 | 109 | [Validating] | 40 |
| 3 | 349 | 386 | 37 | decide | 101 |
| 4 | 487 | 524 | 37 | forked , Rejected | 88 |
| 5 | 612 | 673 | 61 | Persist , Notify , Rejected----note | 64 |
| 6 | 737 | 747 | 10 | joined | 40 |
| 7 | 787 | 824 | 37 | Done | 40 |
| 8 | 864 | 878 | 14 | root_end | - |

Top-level blocks: 1; bands: 9; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 4: forked -> Rejected = 195; band 5: Persist -> Notify = 40; band 5: Notify -> Rejected----note = 100.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 1.

Edges: 16 total, 3 with both ends in the same top-level block (intra) and 13 between different top-level items (inter); 2 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 1-2 | 2 | 285 | 1 (119) |
| gap 3-4 | 1 | 266 | 2 (406, 467) |
| gap 4-5 | 2 | 137 | 1 (544) |
| gap 5-6 | 2 | 159 | 1 (717) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| edge0 | root_start (root_start) | Idle (Idle) | 0 -> 1 | straight |
| edge1 | Idle (Idle) | Validating (Validating) | 1 -> 2 | gap 1-2 122 |
| edge5 | Validating (Validating) | decide (decide) | 2 -> 3 | straight |
| edge6 | decide (decide) | Rejected (Rejected) | 3 -> 4 | gap 3-4 153; gap 3-4 113 |
| edge7 | decide (decide) | forked (forked) | 3 -> 4 | straight |
| edge8 | forked (forked) | Persist (Persist) | 4 -> 5 | gap 4-5 69 |
| edge9 | forked (forked) | Notify (Notify) | 4 -> 5 | gap 4-5 69 |
| edge10 | Persist (Persist) | joined (joined) | 5 -> 6 | gap 5-6 80 |
| edge11 | Notify (Notify) | joined (joined) | 5 -> 6 | gap 5-6 80 |
| edge12 | joined (joined) | Done (Done) | 6 -> 7 | straight |
| edge13 | Rejected (Rejected) | Idle (Idle) | 4 -> 1 | gap 1-2 162 |
| Rejected-Rejected----note-14 | Rejected (Rejected) | Rejected----note (Rejected----note) | 4 -> 5 | straight |
| edge15 | Done (Done) | root_end (root_end) | 7 -> 8 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| edge1 | 41x21 | gap 1-2 | 2 | 47 | 0.78 | - |
| edge3 | 15x21 | band 2 inside [Validating] | 3 | 29 | 0.50 | - |
| edge6 | 40x21 | gap 3-4 | 2 | 154 | 0.56 | - |
| edge7 | 29x21 | gap 3-4 | 2 | 47 | 0.53 | - |
| edge13 | 28x21 | band 2 beside blocks | 1 | 234 | 0.43 | - |

Labels: 5; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.57 Mpx2, edge length 2138 px, bends 22, edge crossings (drawn polylines) 0.

