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

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_n16_n32_0 | n16 (g1) | n32 (g5) | 1 -> 2 | band 1 inside [g1] 230; gap 1-2 176; gap 1-2 1341; band 2 inside [g5] 354; band 2 inside [g5] 156 |
| L_n16_n31_0 | n16 (g1) | n31 (g5) | 1 -> 2 | band 1 inside [g1] 19; gap 1-2 351; gap 1-2 905; band 2 inside [g5] 440; band 2 inside [g5] 266; band 2 inside [g5] 1022 |
| L_n16_n40_0 | n16 (g1) | n40 (g8) | 1 -> 4 | band 1 inside [g1] 244; gap 1-2 134; gap 3-4 1703; band 4 inside [g8] 108 |
| L_n16_n41_0 | n16 (g1) | n41 (g8) | 1 -> 4 | band 1 inside [g1] 60; band 1 inside [g1] 1314; gap 1-2 1537; gap 1-2 312; gap 3-4 860 |
| L_n16_n35_0 | n16 (g1) | n35 (g5) | 1 -> 2 | band 1 inside [g1] 74; band 1 inside [g1] 1314; gap 1-2 1558; gap 1-2 593; band 2 inside [g5] 111; band 2 inside [g5] 131 |
| L_n16_n43_0 | n16 (g1) | n43 (g8) | 1 -> 4 | band 1 inside [g1] 216; gap 1-2 216; gap 3-4 1668; band 4 inside [g8] 584; band 4 inside [g8] 374 |
| L_n16_n37_0 | n16 (g1) | n37 (g8) | 1 -> 4 | band 1 inside [g1] 8; gap 1-2 288; gap 3-4 1496; band 4 inside [g8] 120 |
| L_n16_n23_0 | n16 (g1) | n23 (g5) | 1 -> 2 | band 1 inside [g1] 46; band 1 inside [g1] 1314; gap 1-2 556; gap 1-2 633 |
| L_n16_n44_0 | n16 (g1) | n44 (g8) | 1 -> 4 | band 1 inside [g1] 88; band 1 inside [g1] 1462; gap 1-2 1856; gap 3-4 725; band 4 inside [g8] 26 |
| L_n16_n29_0 | n16 (g1) | n29 (g5) | 1 -> 2 | band 1 inside [g1] 33; gap 1-2 428; gap 1-2 818; band 2 inside [g5] 939; band 2 inside [g5] 233 |
| L_n31_n16_0 | n31 (g5) | n16 (g1) | 2 -> 1 | band 2 inside [g5] 225; gap 1-2 772; gap 1-2 1748; band 1 inside [g1] 1439; band 1 inside [g1] 288 |
| L_n36_n16_0 | n36 (g8) | n16 (g1) | 4 -> 1 | band 4 inside [g8] 186; gap 3-4 1631; gap 1-2 244; band 1 inside [g1] 203 |
| L_n33_n16_0 | n33 (g5) | n16 (g1) | 2 -> 1 | band 2 inside [g5] 242; band 2 inside [g5] 440; gap 1-2 977; gap 1-2 288 |
| L_n43_n16_0 | n43 (g8) | n16 (g1) | 4 -> 1 | band 4 inside [g8] 67; gap 3-4 782; gap 1-2 1904; band 1 inside [g1] 1462; band 1 inside [g1] 102 |
| L_n4_n30_0 | n4 (g1) | n30 (g5) | 1 -> 2 | gap 1-2 1534; gap 1-2 344 |
| L_n8_n42_0 | n8 (g1) | n42 (g8) | 1 -> 4 | gap 1-2 663; gap 1-2 1409; gap 3-4 837; band 4 inside [g8] 18 |
| L_n28_n42_0 | n28 (g5) | n42 (g8) | 2 -> 4 | gap 2-3 66; gap 3-4 570 |
| L_n11_n37_0 | n11 (g1) | n37 (g8) | 1 -> 4 | gap 1-2 49; gap 3-4 1087; band 4 inside [g8] 97 |
| L_n11_n39_0 | n11 (g1) | n39 (g8) | 1 -> 4 | gap 1-2 49; gap 3-4 1055; band 4 inside [g8] 140; band 4 inside [g8] 23 |
| L_n31_n38_0 | n31 (g5) | n38 (g8) | 2 -> 4 | gap 2-3 58; gap 3-4 490 |
| L_n29_n38_0 | n29 (g5) | n38 (g8) | 2 -> 4 | band 2 inside [g5] 19; gap 2-3 400; gap 3-4 32 |
| L_n15_n38_0 | n15 (g1) | n38 (g8) | 1 -> 4 | gap 1-2 118; gap 3-4 1286; band 4 inside [g8] 214 |
| L_n4_n43_0 | n4 (g1) | n43 (g8) | 1 -> 4 | gap 1-2 1849; gap 3-4 742 |
| L_n5_n44_0 | n5 (g1) | n44 (g8) | 1 -> 4 | gap 1-2 1475; gap 3-4 829; band 4 inside [g8] 21 |
| L_n22_n40_0 | n22 (g5) | n40 (g8) | 2 -> 4 | gap 2-3 48; gap 3-4 899; band 4 inside [g8] 140 |
| L_n25_n42_0 | n25 (g5) | n42 (g8) | 2 -> 4 | gap 3-4 488; band 4 inside [g8] 10 |
| L_n25_n38_0 | n25 (g5) | n38 (g8) | 2 -> 4 | gap 2-3 58; gap 3-4 528 |
| L_n20_n37_0 | n20 (g1) | n37 (g8) | 1 -> 4 | gap 1-2 244; gap 3-4 1618; band 4 inside [g8] 110 |
| L_n7_n33_0 | n7 (g1) | n33 (g5) | 1 -> 2 | gap 1-2 1678; gap 1-2 781; band 2 inside [g5] 488; band 2 inside [g5] 288 |
| L_n46_n17_0 | n46 (n46) | n17 (g1) | 0 -> 1 | band 1 inside [g1] 25 |
| L_n46_g1_0 | n46 (n46) | g1 (g1) | 0 -> 1 | straight |
| L_n46_g9_0 | n46 (n46) | g9 (g8) | 0 -> 4 | gap 0-1 3251; gap 3-4 857 |
| L_n45_g8_0 | n45 (n45) | g8 (g8) | 3 -> 4 | gap 3-4 1370 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_n31_n38_0 over title of [g8]; L_n25_n38_0 over title of [g8]; L_n16_n29_0 over title of [g6]; L_n16_n43_0 over title of [g10]; L_n16_n44_0 over title of [g10]; L_n43_n16_0 over title of [g10]; L_n4_n43_0 over title of [g10]; L_n33_n16_0 over title of [g7].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_n16_n32_0 | 83x21 | gap 1-2 | 2 | 1738 | 0.41 | - |
| L_n16_n31_0 | 125x21 | gap 1-2 | 2 | 1702 | 0.31 | - |
| L_n16_n40_0 | 23x21 | band 2 beside blocks | 0 | 2429 | 0.46 | - |
| L_n16_n17_0 | 129x21 | band 1 inside [g1] | 1 | 66 | 0.84 | - |
| L_n16_n41_0 | 102x21 | band 2 beside blocks | 1 | 2196 | 0.71 | - |
| L_n16_n35_0 | 41x21 | gap 1-2 | 0 | 1589 | 0.73 | - |
| L_n16_n43_0 | 58x21 | band 2 beside blocks | 2 | 2482 | 0.38 | - |
| L_n16_n37_0 | 90x21 | band 2 beside blocks | 0 | 2350 | 0.46 | - |
| L_n16_n21_0 | 18x21 | band 1 inside [g1] | 1 | 66 | 0.89 | - |
| L_n16_n23_0 | 105x21 | gap 1-2 | 2 | 947 | 0.77 | - |
| L_n16_n44_0 | 99x21 | band 2 beside blocks | 1 | 2213 | 0.71 | - |
| L_n16_n29_0 | 31x21 | gap 1-2 | 3 | 1798 | 0.40 | - |
| L_n8_n42_0 | 90x21 | band 2 beside blocks | 0 | 2446 | 0.56 | - |
| L_n28_n42_0 | 66x21 | gap 3-4 | 3 | 861 | 0.50 | - |
| L_n11_n37_0 | 47x21 | band 2 beside blocks | 1 | 1114 | 0.33 | - |
| L_n31_n38_0 | 72x21 | band 3 beside blocks | 2 | 233 | 0.18 | - |
| L_n22_n40_0 | 138x21 | band 3 beside blocks | 1 | 150 | 0.09 | - |
| L_n25_n38_0 | 16x21 | band 3 beside blocks | 2 | 186 | 0.14 | - |
| L_n20_n37_0 | 160x21 | band 2 beside blocks | 0 | 1986 | 0.41 | - |
| L_n2_n7_0 | 154x21 | band 1 inside [g1] | 1 | 183 | 0.48 | - |
| L_n29_n31_0 | 44x21 | band 2 inside [g5] | 1 | 83 | 0.73 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=1784: 5 labels; y=2504: 8 labels; y=3132: 3 labels.

Labels: 21; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 14; overlapping something: 0.

Totals: canvas area 18.09 Mpx2, edge length 142591 px, bends 416, edge crossings (drawn polylines) 68.

