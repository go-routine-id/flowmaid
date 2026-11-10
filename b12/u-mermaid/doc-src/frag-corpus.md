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

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_Gate_Q_0 over title of [Workers].

Labels: 4; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 1.34 Mpx2, edge length 5108 px, bends 47, edge crossings (drawn polylines) 0.

#### subgraph.elk.svg  (canvas 466 x 749, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| platform | 0 | - | 8 | 194 | 450 | 442 | 59% | 2 (5 incl. nested) | 1 / 1 / 1 / 2 |
| events | 1 | platform | 32 | 390 | 402 | 222 | 30% | 3 (3 incl. nested) | 1 / 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | Client | 40 |
| 1 | 93 | 144 | 51 | GW | 50 |
| 2 | 194 | 636 | 442 | [platform] | 60 |
| 3 | 696 | 741 | 45 | Store | - |

Top-level blocks: 1; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 2.

Edges: 8 total, 4 with both ends in the same top-level block (intra) and 4 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 2-3 | 2 | 158 | 1 (676) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_GW_Auth_0 over title of [platform]; L_Users_Bus_0 over title of [events].

Totals: canvas area 0.35 Mpx2, edge length 843 px, bends 14, edge crossings (drawn polylines) 0.

#### fanout-frames.elk.svg  (canvas 648 x 2271, flow axis x)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| g1 | 0 | - | 240 | 8 | 400 | 547 | 62% | 5 (5 incl. nested) | 3 / 2 |
| g2 | 0 | - | 240 | 595 | 248 | 318 | 38% | 3 (3 incl. nested) | 3 |
| g3 | 0 | - | 240 | 953 | 376 | 400 | 58% | 6 (6 incl. nested) | 3 / 3 |
| g4 | 0 | - | 240 | 1393 | 200 | 270 | 31% | 2 (2 incl. nested) | 2 |
| g5 | 0 | - | 240 | 1703 | 224 | 138 | 35% | 1 (1 incl. nested) | 1 |
| g6 | 0 | - | 240 | 1881 | 200 | 297 | 31% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 160 | 152 | n0 | 80 |
| 1 | 240 | 640 | 400 | [g1] , [g2] , [g3] , [g4] , [g5] , [g6] , n21 | - |

Top-level blocks: 6; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 1: [g1] -> [g2] = 40; band 1: [g2] -> [g3] = 40; band 1: [g3] -> [g4] = 40; band 1: [g4] -> [g5] = 40; band 1: [g5] -> [g6] = 40; band 1: [g6] -> n21 = 40.

Sides of block frames crossed by inter-block edges: enters via left: 6.

Edges: 7 total, 0 with both ends in the same top-level block (intra) and 7 between different top-level items (inter); 6 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 6 | 3753 | 3 (180, 200, 220) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 1.47 Mpx2, edge length 4253 px, bends 12, edge crossings (drawn polylines) 0.

#### state.elk.svg  (canvas 649 x 886, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| Validating | 0 | - | 8 | 200 | 459 | 109 | 12% | 4 (4 incl. nested) | 4 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 22 | 14 | root_start | 40 |
| 1 | 62 | 99 | 37 | Idle | 101 |
| 2 | 200 | 309 | 109 | [Validating] | 40 |
| 3 | 349 | 386 | 37 | decide | 101 |
| 4 | 487 | 524 | 37 | forked , Rejected | 88 |
| 5 | 612 | 673 | 61 | Persist , Notify , Rejected----note | 64 |
| 6 | 737 | 747 | 10 | joined | 40 |
| 7 | 787 | 824 | 37 | Done | 40 |
| 8 | 864 | 878 | 14 | root_end | - |

Top-level blocks: 1; bands: 9; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 4: forked -> Rejected = 195; band 5: Persist -> Notify = 40; band 5: Notify -> Rejected----note = 100.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 1.

Edges: 16 total, 3 with both ends in the same top-level block (intra) and 13 between different top-level items (inter); 2 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 1-2 | 2 | 285 | 1 (119) |
| gap 3-4 | 1 | 266 | 2 (406, 467) |
| gap 4-5 | 2 | 137 | 1 (544) |
| gap 5-6 | 2 | 159 | 1 (717) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Labels: 5; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.57 Mpx2, edge length 2138 px, bends 22, edge crossings (drawn polylines) 0.

#### bench-75.elk.svg  (canvas 4327 x 1058, flow axis x)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| edge | 0 | - | 249 | 469 | 448 | 302 | 10% | 2 (2 incl. nested) | 1 / 1 |
| app | 0 | - | 1257 | 137 | 200 | 624 | 5% | 6 (6 incl. nested) | 6 |
| data | 0 | - | 2181 | 175 | 200 | 488 | 5% | 4 (4 incl. nested) | 4 |
| jobs | 0 | - | 2737 | 262 | 200 | 308 | 5% | 3 (3 incl. nested) | 3 |
| ops | 0 | - | 3361 | 396 | 696 | 281 | 16% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 179 | 171 | user | 70 |
| 1 | 249 | 697 | 448 | [edge] | 560 |
| 2 | 1257 | 1457 | 200 | [app] | 724 |
| 3 | 2181 | 2381 | 200 | [data] | 356 |
| 4 | 2737 | 2937 | 200 | [jobs] | 424 |
| 5 | 3361 | 4057 | 696 | [ops] | 110 |
| 6 | 4167 | 4319 | 152 | ext | - |

Top-level blocks: 5; bands: 7; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via bottom: 2, enters via left: 52, enters via right: 8, leaves via bottom: 1, leaves via left: 8, leaves via right: 58.

Edges: 75 total, 5 with both ends in the same top-level block (intra) and 70 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 4 | 717 | 2 (199, 219) |
| band 1 inside [edge] | 7 | 298 | 4 (455, 467, 479, 491) |
| gap 1-2 | 15 | 5100 | 24 (703, 715, 727, 823, 843, 863, 883, 903, 923, 943, 963, 983, 1003, 1023 ...) |
| gap 2-3 | 35 | 7333 | 35 (1463, 1475, 1487, 1595, 1615, 1635, 1655, 1675, 1695, 1715, 1735, 1755, 1775, 1795 ...) |
| gap 3-4 | 9 | 1495 | 10 (2483, 2503, 2543, 2563, 2583, 2603, 2623, 2707, 2719, 2731) |
| gap 4-5 | 20 | 5258 | 15 (3027, 3047, 3067, 3087, 3107, 3127, 3147, 3167, 3187, 3207, 3227, 3247, 3331, 3343 ...) |
| band 5 inside [ops] | 9 | 735 | 6 (3567, 3579, 3591, 3603, 3627, 3851) |
| gap 5-6 | 7 | 2322 | 4 (4087, 4107, 4127, 4147) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 4.58 Mpx2, edge length 107188 px, bends 589, edge crossings (drawn polylines) 222.

#### dense-td.elk.svg  (canvas 4131 x 4378, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| g1 | 0 | - | 8 | 166 | 2879 | 1349 | 31% | 7 (21 incl. nested) | 3 / 5 / 3 / 10 |
| g5 | 0 | - | 1302 | 1946 | 2236 | 1104 | 25% | 3 (14 incl. nested) | 1 / 1 / 8 / 4 |
| g8 | 0 | - | 1860 | 3405 | 1488 | 965 | 22% | 5 (9 incl. nested) | 3 / 1 / 2 / 1 / 1 / 1 |
| g4 | 1 | g1 | 32 | 224 | 1158 | 556 | 13% | 6 (6 incl. nested) | 3 / 3 |
| g2 | 1 | g1 | 560 | 1078 | 1310 | 403 | 9% | 3 (8 incl. nested) | 8 |
| g6 | 1 | g5 | 1945 | 2100 | 1569 | 926 | 21% | 7 (11 incl. nested) | 1 / 7 / 3 |
| g9 | 1 | g8 | 2691 | 3616 | 632 | 583 | 13% | 2 (4 incl. nested) | 2 / 1 / 1 |
| g3 | 2 | g2 | 786 | 1126 | 622 | 331 | 8% | 5 (5 incl. nested) | 2 / 3 |
| g7 | 2 | g6 | 2171 | 2263 | 620 | 469 | 11% | 4 (4 incl. nested) | 3 / 1 |
| g10 | 2 | g9 | 3037 | 3746 | 262 | 303 | 7% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 116 | 108 | n46 | 50 |
| 1 | 166 | 1515 | 1349 | [g1] | 431 |
| 2 | 1946 | 3050 | 1104 | [g5] | 60 |
| 3 | 3110 | 3155 | 45 | n45 | 250 |
| 4 | 3405 | 4370 | 965 | [g8] | - |

Top-level blocks: 3; bands: 5; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via bottom: 4, enters via top: 29, leaves via bottom: 25, leaves via top: 4.

Edges: 47 total, 14 with both ends in the same top-level block (intra) and 33 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 1 | 3251 | 1 (136) |
| band 1 inside [g1] | 14 | 9945 | 15 (471, 495, 507, 519, 531, 543, 555, 567, 660, 820, 832, 844, 856, 868 ...) |
| gap 1-2 | 23 | 27826 | 17 (1555, 1575, 1595, 1615, 1635, 1655, 1675, 1695, 1715, 1735, 1755, 1816, 1836, 1856 ...) |
| band 2 inside [g5] | 8 | 5352 | 11 (2024, 2036, 2048, 2060, 2199, 2211, 2223, 2362, 2530, 2762, 2843) |
| gap 2-3 | 5 | 630 | 1 (3090) |
| gap 3-4 | 22 | 21551 | 11 (3175, 3195, 3215, 3235, 3255, 3275, 3295, 3315, 3335, 3355, 3375) |
| band 4 inside [g8] | 14 | 2238 | 13 (3504, 3516, 3528, 3540, 3552, 3564, 3576, 3694, 3706, 3845, 3950, 4079, 4229) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_n31_n38_0 over title of [g8]; L_n25_n38_0 over title of [g8]; L_n16_n29_0 over title of [g6]; L_n16_n43_0 over title of [g10]; L_n16_n44_0 over title of [g10]; L_n43_n16_0 over title of [g10]; L_n4_n43_0 over title of [g10]; L_n33_n16_0 over title of [g7].

Label rows (>= 3 labels centred on the same flow coordinate): y=1784: 5 labels; y=2504: 8 labels; y=3132: 3 labels.

Labels: 21; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 14; overlapping something: 0.

Totals: canvas area 18.09 Mpx2, edge length 142591 px, bends 416, edge crossings (drawn polylines) 68.
