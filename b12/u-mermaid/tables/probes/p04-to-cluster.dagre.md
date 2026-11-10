#### p04-to-cluster.dagre.svg  (canvas 672 x 528, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| S | 0 | - | 34 | 124 | 454 | 210 | 40% | 3 (3 incl. nested) | 1 / 1 / 1 |
| T | 0 | - | 210 | 405 | 454 | 115 | 22% | 2 (2 incl. nested) | 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | x | 71 |
| 1 | 124 | 334 | 210 | [S] | 71 |
| 2 | 405 | 520 | 115 | y , [T] | - |

Top-level blocks: 2; bands: 3; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 2: y -> [T] = 50.

Sides of block frames crossed by inter-block edges: enters via top: 2, leaves via bottom: 2.

Edges: 6 total, 3 with both ends in the same top-level block (intra) and 3 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_x_S_0 | x (x) | S (S) | 0 -> 1 | straight |
| L_S_y_0 | S (S) | y (y) | 1 -> 2 | straight |
| L_S_T_0 | S (S) | T (T) | 1 -> 2 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_x_S_0 | 48x21 | gap 0-1 | 2 | 34 | 0.50 | - |
| L_S_y_0 | 65x21 | gap 1-2 | 9 | 53 | 0.45 | - |
| L_S_T_0 | 85x21 | gap 1-2 | 6 | 28 | 0.67 | - |

Labels: 3; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.35 Mpx2, edge length 536 px, bends 6, edge crossings (drawn polylines) 0.

