#### p06-block-cycle.elk.svg  (canvas 260 x 581, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| A | 0 | - | 8 | 8 | 244 | 222 | 38% | 2 (2 incl. nested) | 1 / 1 |
| B | 0 | - | 8 | 351 | 244 | 222 | 38% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 230 | 222 | [A] | 121 |
| 1 | 351 | 573 | 222 | [B] | - |

Top-level blocks: 2; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via bottom: 1, enters via top: 1, leaves via bottom: 1, leaves via top: 1.

Edges: 4 total, 2 with both ends in the same top-level block (intra) and 2 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 0 inside [A] | 1 | 23 | 1 (131) |
| band 1 inside [B] | 1 | 23 | 1 (474) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_a2_b1_0 | a2 (A) | b1 (B) | 0 -> 1 | straight |
| L_b2_a1_0 | b2 (B) | a1 (A) | 1 -> 0 | band 1 inside [B] 23; band 0 inside [A] 23 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_a2_b1_0 | 47x21 | gap 0-1 | 2 | 83 | 0.44 | - |
| L_b2_a1_0 | 30x21 | gap 0-1 | 1 | 202 | 0.53 | - |

Labels: 2; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.15 Mpx2, edge length 770 px, bends 12, edge crossings (drawn polylines) 0.

