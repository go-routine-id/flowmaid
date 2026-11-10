#### advanced.elk.svg  (canvas 906 x 1484, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| CI | 0 | - | 194 | 8 | 449 | 452 | 30% | 4 (4 incl. nested) | 1 / 1 / 2 |
| Platform | 0 | - | 8 | 510 | 870 | 634 | 43% | 4 (7 incl. nested) | 1 / 3 / 2 / 1 |
| Workers | 1 | Platform | 32 | 691 | 414 | 262 | 18% | 3 (3 incl. nested) | 1 / 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 460 | 452 | [CI] | 50 |
| 1 | 510 | 1144 | 634 | [Platform] | 50 |
| 2 | 1194 | 1285 | 91 | Mon | 101 |
| 3 | 1386 | 1476 | 89 | Done , Rollback | - |

Top-level blocks: 2; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 3: Done -> Rollback = 274.

Sides of block frames crossed by inter-block edges: enters via top: 2, leaves via bottom: 3.

Edges: 20 total, 14 with both ends in the same top-level block (intra) and 6 between different top-level items (inter); 4 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 1 | 596 | 1 (490) |
| gap 1-2 | 2 | 30 | 1 (1174) |
| gap 2-3 | 2 | 542 | 2 (1305, 1366) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_Reg_Platform_0 | Reg (CI) | Platform (Platform) | 0 -> 1 | straight |
| L_Platform_Mon_0 | Platform (Platform) | Mon (Mon) | 1 -> 2 | gap 1-2 15 |
| L_Mon_Done_0 | Mon (Mon) | Done (Done) | 2 -> 3 | straight |
| L_Mon_Rollback_0 | Mon (Mon) | Rollback (Rollback) | 2 -> 3 | gap 2-3 44; gap 2-3 279 |
| L_Rollback_Platform_0 | Rollback (Rollback) | Platform (Platform) | 3 -> 1 | gap 2-3 219; gap 0-1 596 |
| L_Users_Mon_0 | Users (Platform) | Mon (Mon) | 1 -> 2 | gap 1-2 15 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_Gate_Q_0 over title of [Workers].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_Scan_Reg_0 | 30x21 | band 0 inside [CI] | 1 | 29 | 0.83 | - |
| L_Scan_Dev_0 | 18x21 | band 0 inside [CI] | 2 | 53 | 0.74 | - |
| L_Mon_Done_0 | 22x21 | gap 2-3 | 1 | 46 | 0.53 | - |
| L_Mon_Rollback_0 | 16x21 | gap 2-3 | 0 | 89 | 0.22 | - |

Labels: 4; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 1.34 Mpx2, edge length 5108 px, bends 47, edge crossings (drawn polylines) 0.

