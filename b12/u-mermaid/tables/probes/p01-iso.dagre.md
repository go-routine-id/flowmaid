#### p01-iso.dagre.svg  (canvas 899 x 226, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| S | 0 | - | 8 | 8 | 681 | 115 | 51% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 123 | 115 | [S] , x | 50 |
| 1 | 173 | 218 | 45 | y | - |

Top-level blocks: 1; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: [S] -> x = 50.

Edges: 3 total, 2 with both ends in the same top-level block (intra) and 1 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_x_y_0 | x (x) | y (y) | 0 -> 1 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 0.20 Mpx2, edge length 223 px, bends 3, edge crossings (drawn polylines) 0.

