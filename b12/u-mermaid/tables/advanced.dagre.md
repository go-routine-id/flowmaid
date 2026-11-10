#### advanced.dagre.svg  (canvas 1467 x 1321, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| CI | 0 | - | 1117 | 8 | 342 | 554 | 42% | 4 (4 incl. nested) | 1 / 1 / 1 / 1 |
| Platform | 0 | - | 282 | 406 | 815 | 746 | 57% | 4 (7 incl. nested) | 1 / 2 / 3 / 1 |
| Workers | 1 | Platform | 613 | 612 | 464 | 515 | 39% | 3 (3 incl. nested) | 1 / 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 1152 | 1144 | Mon , [Platform] , [CI] | 71 |
| 1 | 1223 | 1313 | 89 | Done , Rollback | - |

Top-level blocks: 2; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): band 0.

Cross-axis gaps between neighbours in the same band: band 0: Mon -> [Platform] = 70; band 0: [Platform] -> [CI] = 20; band 1: Done -> Rollback = 388.

Sides of block frames crossed by inter-block edges: enters via bottom: 1, enters via right: 1, leaves via bottom: 1, leaves via left: 2.

Edges: 20 total, 14 with both ends in the same top-level block (intra) and 6 between different top-level items (inter); 4 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 0 inside Mon+[Platform] | 1 | 632 | 1 (1017) |
| band 0 inside [Platform] | 4 | 812 | 4 (575, 719, 746, 1011) |
| gap 0-1 | 2 | 696 | 2 (1219, 1221) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_Mon_Done_0 | Mon (Mon) | Done (Done) | 0 -> 1 | straight |
| L_Mon_Rollback_0 | Mon (Mon) | Rollback (Rollback) | 0 -> 1 | gap 0-1 326 |
| L_Users_Mon_0 | Users (Platform) | Mon (Mon) | 0 -> 0 | band 0 inside [Platform] 206 |
| L_Reg_Platform_0 | Reg (CI) | Platform (Platform) | 0 -> 0 | band 0 inside [Platform] 298 |
| L_Platform_Mon_0 | Platform (Platform) | Mon (Mon) | 0 -> 0 | band 0 inside [Platform] 268; band 0 inside Mon+[Platform] 632 |
| L_Rollback_Platform_0 | Rollback (Rollback) | Platform (Platform) | 1 -> 0 | gap 0-1 370; band 0 inside [Platform] 40 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_Scan_Reg_0 | 30x21 | band 0 inside [CI] | 1 | 34 | 0.38 | - |
| L_Scan_Dev_0 | 18x21 | band 0 inside [CI] | 2 | 69 | 0.48 | - |
| L_Mon_Done_0 | 22x21 | gap 0-1 | 2 | 29 | 0.75 | - |
| L_Mon_Rollback_0 | 16x21 | gap 0-1 | 17 | 89 | 0.22 | - |

Labels: 4; more than 12 px off their own line: 1; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 1.94 Mpx2, edge length 3535 px, bends 52, edge crossings (drawn polylines) 2.

