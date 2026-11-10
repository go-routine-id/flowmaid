#### p05-siblings.elk.svg  (canvas 696 x 461, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| B | 0 | - | 248 | 103 | 200 | 255 | 55% | 3 (3 incl. nested) | 1 / 1 / 1 |
| A | 0 | - | 8 | 138 | 200 | 186 | 40% | 2 (2 incl. nested) | 1 / 1 |
| C | 0 | - | 488 | 172 | 200 | 117 | 25% | 1 (1 incl. nested) | 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | r | 50 |
| 1 | 103 | 358 | 255 | [A] , [B] , [C] | 50 |
| 2 | 408 | 453 | 45 | z | - |

Top-level blocks: 3; bands: 3; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 1: [A] -> [B] = 40; band 1: [B] -> [C] = 40.

Sides of block frames crossed by inter-block edges: enters via top: 3, leaves via bottom: 3.

Edges: 9 total, 3 with both ends in the same top-level block (intra) and 6 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 414 | 1 (73) |
| gap 1-2 | 2 | 414 | 1 (388) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_r_a1_0 | r (r) | a1 (A) | 0 -> 1 | gap 0-1 207 |
| L_r_b1_0 | r (r) | b1 (B) | 0 -> 1 | straight |
| L_r_c1_0 | r (r) | c1 (C) | 0 -> 1 | gap 0-1 207 |
| L_a2_z_0 | a2 (A) | z (z) | 1 -> 2 | gap 1-2 207 |
| L_b3_z_0 | b3 (B) | z (z) | 1 -> 2 | straight |
| L_c1_z_0 | c1 (C) | z (z) | 1 -> 2 | gap 1-2 207 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_r_c1_0 over title of [C]; L_r_b1_0 over title of [B]; L_r_a1_0 over title of [A].

Totals: canvas area 0.32 Mpx2, edge length 1566 px, bends 14, edge crossings (drawn polylines) 0.

