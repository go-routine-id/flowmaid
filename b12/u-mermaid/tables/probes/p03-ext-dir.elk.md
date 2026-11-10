#### p03-ext-dir.elk.svg  (canvas 216 x 461, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| S | 0 | - | 8 | 103 | 200 | 255 | 55% | 3 (3 incl. nested) | 1 / 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | x | 50 |
| 1 | 103 | 358 | 255 | [S] | 50 |
| 2 | 408 | 453 | 45 | y | - |

Top-level blocks: 1; bands: 3; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 1.

Edges: 4 total, 2 with both ends in the same top-level block (intra) and 2 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_x_a_0 | x (x) | a (S) | 0 -> 1 | straight |
| L_c_y_0 | c (S) | y (y) | 1 -> 2 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_x_a_0 over title of [S].

Totals: canvas area 0.10 Mpx2, edge length 204 px, bends 2, edge crossings (drawn polylines) 0.

