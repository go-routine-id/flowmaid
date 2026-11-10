#### p09-disconnected.dagre.svg  (canvas 1680 x 131, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| C | 0 | - | 8 | 8 | 454 | 115 | 88% | 2 (2 incl. nested) | 2 |
| B | 0 | - | 512 | 8 | 454 | 115 | 88% | 2 (2 incl. nested) | 2 |
| A | 0 | - | 1016 | 8 | 454 | 115 | 88% | 2 (2 incl. nested) | 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 123 | 115 | [C] , [B] , [A] , lone | - |

Top-level blocks: 3; bands: 1; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: [C] -> [B] = 50; band 0: [B] -> [A] = 50; band 0: [A] -> lone = 50.

Edges: 3 total, 3 with both ends in the same top-level block (intra) and 0 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 0.22 Mpx2, edge length 213 px, bends 3, edge crossings (drawn polylines) 0.

