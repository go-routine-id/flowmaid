#### fanout-frames.elk.svg  (canvas 648 x 2271, flow axis x)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| g1 | 0 | - | 240 | 8 | 400 | 547 | 62% | 5 (5 incl. nested) | 3 / 2 |
| g2 | 0 | - | 240 | 595 | 248 | 318 | 38% | 3 (3 incl. nested) | 3 |
| g3 | 0 | - | 240 | 953 | 376 | 400 | 58% | 6 (6 incl. nested) | 3 / 3 |
| g4 | 0 | - | 240 | 1393 | 200 | 270 | 31% | 2 (2 incl. nested) | 2 |
| g5 | 0 | - | 240 | 1703 | 224 | 138 | 35% | 1 (1 incl. nested) | 1 |
| g6 | 0 | - | 240 | 1881 | 200 | 297 | 31% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 160 | 152 | n0 | 80 |
| 1 | 240 | 640 | 400 | [g1] , [g2] , [g3] , [g4] , [g5] , [g6] , n21 | - |

Top-level blocks: 6; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 1: [g1] -> [g2] = 40; band 1: [g2] -> [g3] = 40; band 1: [g3] -> [g4] = 40; band 1: [g4] -> [g5] = 40; band 1: [g5] -> [g6] = 40; band 1: [g6] -> n21 = 40.

Sides of block frames crossed by inter-block edges: enters via left: 6.

Edges: 7 total, 0 with both ends in the same top-level block (intra) and 7 between different top-level items (inter); 6 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 6 | 3753 | 3 (180, 200, 220) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_n0_g1_0 | n0 (n0) | g1 (g1) | 0 -> 1 | gap 0-1 1222 |
| L_n0_g2_0 | n0 (n0) | g2 (g2) | 0 -> 1 | gap 0-1 757 |
| L_n0_g3_0 | n0 (n0) | g3 (g3) | 0 -> 1 | gap 0-1 367 |
| L_n0_g4_0 | n0 (n0) | g4 (g4) | 0 -> 1 | straight |
| L_n0_g5_0 | n0 (n0) | g5 (g5) | 0 -> 1 | gap 0-1 236 |
| L_n0_g6_0 | n0 (n0) | g6 (g6) | 0 -> 1 | gap 0-1 485 |
| L_n0_n21_0 | n0 (n0) | n21 (n21) | 0 -> 1 | gap 0-1 687 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 1.47 Mpx2, edge length 4253 px, bends 12, edge crossings (drawn polylines) 0.

