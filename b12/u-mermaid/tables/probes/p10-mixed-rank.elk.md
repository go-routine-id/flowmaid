#### p10-mixed-rank.elk.svg  (canvas 408 x 546, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| A | 0 | - | 8 | 188 | 200 | 255 | 47% | 3 (3 incl. nested) | 1 / 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | s | 40 |
| 1 | 93 | 138 | 45 | m | 50 |
| 2 | 188 | 443 | 255 | [A] , n | 50 |
| 3 | 493 | 538 | 45 | t | - |

Top-level blocks: 1; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 2: [A] -> n = 40.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 1.

Edges: 7 total, 2 with both ends in the same top-level block (intra) and 5 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 172 | 1 (73) |
| gap 2-3 | 2 | 172 | 1 (473) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_s_a1_0 | s (s) | a1 (A) | 0 -> 2 | gap 0-1 86 |
| L_s_m_0 | s (s) | m (m) | 0 -> 1 | gap 0-1 86 |
| L_m_n_0 | m (m) | n (n) | 1 -> 2 | straight |
| L_n_t_0 | n (n) | t (t) | 2 -> 3 | gap 2-3 86 |
| L_a3_t_0 | a3 (A) | t (t) | 2 -> 3 | gap 2-3 86 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_s_a1_0 over title of [A].

Totals: canvas area 0.22 Mpx2, edge length 950 px, bends 11, edge crossings (drawn polylines) 0.

