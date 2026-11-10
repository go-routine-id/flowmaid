#### fanout-frames.dagre.svg  (canvas 1442 x 1520, flow axis x)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| g3 | 0 | - | 210 | 672 | 1224 | 183 | 85% | 6 (6 incl. nested) | 1 / 1 / 1 / 1 / 1 / 1 |
| g1 | 0 | - | 299 | 103 | 1046 | 286 | 73% | 5 (5 incl. nested) | 1 / 1 / 1 / 1 / 1 |
| g2 | 0 | - | 466 | 439 | 713 | 183 | 49% | 3 (3 incl. nested) | 1 / 1 / 1 |
| g6 | 0 | - | 509 | 1350 | 626 | 162 | 43% | 3 (3 incl. nested) | 1 / 1 / 1 |
| g4 | 0 | - | 610 | 905 | 424 | 204 | 29% | 2 (2 incl. nested) | 1 / 1 |
| g5 | 0 | - | 699 | 1159 | 246 | 141 | 17% | 1 (1 incl. nested) | 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 160 | 152 | n0 | 50 |
| 1 | 210 | 1434 | 1224 | n21 , [g1] , [g2] , [g3] , [g4] , [g5] , [g6] | - |

Top-level blocks: 6; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 1: n21 -> [g1] = 50; band 1: [g1] -> [g2] = 50; band 1: [g2] -> [g3] = 50; band 1: [g3] -> [g4] = 50; band 1: [g4] -> [g5] = 50; band 1: [g5] -> [g6] = 50.

Sides of block frames crossed by inter-block edges: enters via left: 6.

Edges: 7 total, 0 with both ends in the same top-level block (intra) and 7 between different top-level items (inter); 6 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 0 beside blocks | 4 | 2242 | 2 (138, 139) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_n0_n21_0 | n0 (n0) | n21 (n21) | 0 -> 1 | band 0 beside blocks 696 |
| L_n0_g1_0 | n0 (n0) | g1 (g1) | 0 -> 1 | band 0 beside blocks 483 |
| L_n0_g2_0 | n0 (n0) | g2 (g2) | 0 -> 1 | straight |
| L_n0_g3_0 | n0 (n0) | g3 (g3) | 0 -> 1 | straight |
| L_n0_g4_0 | n0 (n0) | g4 (g4) | 0 -> 1 | straight |
| L_n0_g5_0 | n0 (n0) | g5 (g5) | 0 -> 1 | band 0 beside blocks 432 |
| L_n0_g6_0 | n0 (n0) | g6 (g6) | 0 -> 1 | band 0 beside blocks 631 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 2.19 Mpx2, edge length 4455 px, bends 7, edge crossings (drawn polylines) 0.

