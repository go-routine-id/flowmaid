#### p10-mixed-rank.dagre.svg  (canvas 425 x 491, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| A | 0 | - | 195 | 103 | 222 | 285 | 58% | 3 (3 incl. nested) | 1 / 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | s | 50 |
| 1 | 103 | 388 | 285 | m , n , [A] | 50 |
| 2 | 438 | 483 | 45 | t | - |

Top-level blocks: 1; bands: 3; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): band 1.

Cross-axis gaps between neighbours in the same band: band 1: m -> n = -152; band 1: n -> [A] = 35.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 1.

Edges: 7 total, 2 with both ends in the same top-level block (intra) and 5 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_s_a1_0 | s (s) | a1 (A) | 0 -> 1 | straight |
| L_s_m_0 | s (s) | m (m) | 0 -> 1 | straight |
| L_m_n_0 | m (m) | n (n) | 1 -> 1 | straight |
| L_n_t_0 | n (n) | t (t) | 1 -> 2 | straight |
| L_a3_t_0 | a3 (A) | t (t) | 1 -> 2 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_s_a1_0 over title of [A].

Totals: canvas area 0.21 Mpx2, edge length 649 px, bends 13, edge crossings (drawn polylines) 0.

