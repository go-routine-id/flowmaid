#### p07-nested.dagre.svg  (canvas 530 x 541, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| O | 0 | - | 195 | 8 | 328 | 430 | 79% | 2 (4 incl. nested) | 1 / 1 / 1 / 1 |
| I | 1 | O | 215 | 223 | 222 | 190 | 35% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 438 | 430 | x , [O] | 50 |
| 1 | 488 | 533 | 45 | y | - |

Top-level blocks: 1; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: x -> [O] = 35.

Sides of block frames crossed by inter-block edges: enters via left: 1, leaves via bottom: 2.

Edges: 6 total, 3 with both ends in the same top-level block (intra) and 3 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 0 inside [O] | 1 | 134 | 1 (183) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_x_i1_0 | x (x) | i1 (O) | 0 -> 0 | band 0 inside [O] 134 |
| L_i2_y_0 | i2 (O) | y (y) | 0 -> 1 | straight |
| L_o1_y_0 | o1 (O) | y (y) | 0 -> 1 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_o2_i1_0 over title of [I].

Totals: canvas area 0.29 Mpx2, edge length 895 px, bends 19, edge crossings (drawn polylines) 0.

