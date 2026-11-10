#### p07-nested.elk.svg  (canvas 330 x 694, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| O | 0 | - | 8 | 103 | 314 | 478 | 69% | 2 (4 incl. nested) | 1 / 1 / 1 / 1 |
| I | 1 | O | 98 | 371 | 200 | 186 | 27% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | x | 50 |
| 1 | 103 | 581 | 478 | [O] | 60 |
| 2 | 641 | 686 | 45 | y | - |

Top-level blocks: 1; bands: 3; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 2.

Edges: 6 total, 3 with both ends in the same top-level block (intra) and 3 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 1 inside [O] | 2 | 45 | 2 (226, 331) |
| gap 1-2 | 2 | 90 | 1 (621) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_x_i1_0 | x (x) | i1 (O) | 0 -> 1 | band 1 inside [O] 22 |
| L_i2_y_0 | i2 (O) | y (y) | 1 -> 2 | gap 1-2 45 |
| L_o1_y_0 | o1 (O) | y (y) | 1 -> 2 | band 1 inside [O] 23; gap 1-2 45 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 0.23 Mpx2, edge length 1246 px, bends 18, edge crossings (drawn polylines) 0.

