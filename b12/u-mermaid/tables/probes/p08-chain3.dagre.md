#### p08-chain3.dagre.svg  (canvas 343 x 707, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| A | 0 | - | 8 | 8 | 327 | 190 | 27% | 2 (2 incl. nested) | 1 / 1 |
| B | 0 | - | 8 | 248 | 222 | 211 | 30% | 2 (2 incl. nested) | 1 / 1 |
| C | 0 | - | 8 | 509 | 327 | 190 | 27% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 198 | 190 | [A] | 50 |
| 1 | 248 | 459 | 211 | [B] | 50 |
| 2 | 509 | 699 | 190 | [C] | - |

Top-level blocks: 3; bands: 3; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 3, leaves via bottom: 3.

Edges: 6 total, 3 with both ends in the same top-level block (intra) and 3 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_a2_b1_0 | a2 (A) | b1 (B) | 0 -> 1 | straight |
| L_b2_c1_0 | b2 (B) | c1 (C) | 1 -> 2 | straight |
| L_a1_c2_0 | a1 (A) | c2 (C) | 0 -> 2 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_a2_b1_0 over title of [B].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_a1_c2_0 | 38x21 | band 1 beside blocks | 1 | 287 | 0.51 | - |

Labels: 1; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.24 Mpx2, edge length 968 px, bends 22, edge crossings (drawn polylines) 0.

