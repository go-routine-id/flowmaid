#### p11-lr.elk.svg  (canvas 1351 x 158, flow axis x)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| A | 0 | - | 210 | 8 | 412 | 142 | 30% | 2 (2 incl. nested) | 1 / 1 |
| B | 0 | - | 729 | 8 | 412 | 142 | 30% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 160 | 152 | u | 50 |
| 1 | 210 | 622 | 412 | [A] | 107 |
| 2 | 729 | 1141 | 412 | [B] | 50 |
| 3 | 1191 | 1343 | 152 | v | - |

Top-level blocks: 2; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via left: 3, leaves via right: 3.

Edges: 6 total, 2 with both ends in the same top-level block (intra) and 4 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 1-2 | 1 | 34 | 2 (652, 699) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_u_a1_0 | u (u) | a1 (A) | 0 -> 1 | straight |
| L_a2_b1_0 | a2 (A) | b1 (B) | 1 -> 2 | straight |
| L_a1_b2_0 | a1 (A) | b2 (B) | 1 -> 2 | gap 1-2 17; gap 1-2 17 |
| L_b2_v_0 | b2 (B) | v (v) | 2 -> 3 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_a2_b1_0 | 7x21 | gap 1-2 | 2 | 78 | 0.50 | - |
| L_a1_b2_0 | 7x21 | gap 1-2 | 3 | 263 | 0.44 | - |

Labels: 2; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.21 Mpx2, edge length 1023 px, bends 18, edge crossings (drawn polylines) 0.

