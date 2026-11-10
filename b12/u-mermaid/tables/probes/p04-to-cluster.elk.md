#### p04-to-cluster.elk.svg  (canvas 425 x 671, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| S | 0 | - | 8 | 154 | 402 | 222 | 33% | 3 (3 incl. nested) | 1 / 2 |
| T | 0 | - | 217 | 477 | 200 | 186 | 28% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | x | 101 |
| 1 | 154 | 376 | 222 | [S] | 101 |
| 2 | 477 | 663 | 186 | y , [T] | - |

Top-level blocks: 2; bands: 3; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 2: y -> [T] = 40.

Sides of block frames crossed by inter-block edges: enters via top: 2, leaves via bottom: 2.

Edges: 6 total, 3 with both ends in the same top-level block (intra) and 3 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 1-2 | 2 | 89 | 1 (396) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_x_S_0 | x (x) | S (S) | 0 -> 1 | straight |
| L_S_y_0 | S (S) | y (y) | 1 -> 2 | gap 1-2 44 |
| L_S_T_0 | S (S) | T (T) | 1 -> 2 | gap 1-2 44 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_x_S_0 | 48x21 | gap 0-1 | 1 | 46 | 0.53 | - |
| L_S_y_0 | 65x21 | gap 1-2 | 1 | 47 | 0.65 | - |
| L_S_T_0 | 85x21 | gap 1-2 | 1 | 47 | 0.65 | - |

Labels: 3; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.29 Mpx2, edge length 649 px, bends 8, edge crossings (drawn polylines) 0.

