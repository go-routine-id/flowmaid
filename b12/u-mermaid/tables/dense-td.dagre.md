#### dense-td.dagre.svg  (canvas 6586 x 1749, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| g1 | 0 | - | 1148 | 8 | 3677 | 1205 | 69% | 7 (21 incl. nested) | 1 / 4 / 2 / 2 / 12 |
| g8 | 0 | - | 8 | 507 | 1120 | 1234 | 71% | 5 (9 incl. nested) | 2 / 3 / 2 / 1 / 1 |
| g5 | 0 | - | 4845 | 507 | 1733 | 935 | 53% | 3 (14 incl. nested) | 2 / 1 / 7 / 4 |
| g9 | 1 | g8 | 28 | 532 | 509 | 885 | 51% | 2 (4 incl. nested) | 1 / 2 / 1 |
| g6 | 1 | g5 | 5067 | 532 | 1289 | 885 | 51% | 7 (11 incl. nested) | 2 / 6 / 3 |
| g2 | 1 | g1 | 1253 | 740 | 1501 | 448 | 26% | 3 (8 incl. nested) | 1 / 7 |
| g4 | 1 | g1 | 2774 | 740 | 1997 | 448 | 26% | 6 (6 incl. nested) | 1 / 5 |
| g10 | 2 | g9 | 48 | 557 | 232 | 606 | 35% | 2 (2 incl. nested) | 1 / 1 |
| g7 | 2 | g6 | 5087 | 557 | 618 | 606 | 35% | 4 (4 incl. nested) | 1 / 3 |
| g3 | 2 | g2 | 1510 | 765 | 820 | 398 | 23% | 5 (5 incl. nested) | 1 / 4 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 1741 | 1733 | n45 , [g8] , n46 , [g1] , [g5] | - |

Top-level blocks: 3; bands: 1; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): band 0.

Cross-axis gaps between neighbours in the same band: band 0: n45 -> [g8] = -152; band 0: [g8] -> n46 = -918; band 0: n46 -> [g1] = 786; band 0: [g1] -> [g5] = 20.

Sides of block frames crossed by inter-block edges: enters via bottom: 1, enters via left: 11, enters via right: 19, enters via top: 2, leaves via bottom: 10, leaves via left: 11, leaves via right: 8.

Edges: 47 total, 14 with both ends in the same top-level block (intra) and 33 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 0 inside [g1] | 19 | 22365 | 12 (272, 443, 444, 450, 841, 892, 894, 895, 896, 897, 899, 1201) |
| band 0 inside [g1]+[g5] | 7 | 9797 | 7 (589, 590, 802, 988, 989, 990, 991) |
| band 0 inside [g5] | 3 | 3789 | 2 (678, 1231) |
| band 0 inside [g8] | 9 | 27804 | 2 (1231, 1460) |
| band 0 inside [g8]+[g1] | 8 | 19636 | 5 (589, 804, 989, 991, 992) |
| band 0 inside [g8]+[g1]+[g5] | 1 | 4042 | 1 (992) |
| band 0 inside [g8]+[g5] | 2 | 10932 | 1 (1293) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_n16_n32_0 | n16 (g1) | n32 (g5) | 0 -> 0 | band 0 inside [g1] 418; band 0 inside [g1]+[g5] 678 |
| L_n16_n31_0 | n16 (g1) | n31 (g5) | 0 -> 0 | band 0 inside [g1] 276; band 0 inside [g5] 1805 |
| L_n16_n40_0 | n16 (g1) | n40 (g8) | 0 -> 0 | band 0 inside [g1] 847; band 0 inside [g8]+[g1] 2134 |
| L_n16_n41_0 | n16 (g1) | n41 (g8) | 0 -> 0 | band 0 inside [g1] 929; band 0 inside [g8]+[g1] 2465 |
| L_n16_n35_0 | n16 (g1) | n35 (g5) | 0 -> 0 | band 0 inside [g1] 500; band 0 inside [g1]+[g5] 989 |
| L_n16_n43_0 | n16 (g1) | n43 (g8) | 0 -> 0 | band 0 inside [g8]+[g1] 3690 |
| L_n16_n37_0 | n16 (g1) | n37 (g8) | 0 -> 0 | band 0 inside [g1] 1058; band 0 inside [g8] 2092 |
| L_n16_n23_0 | n16 (g1) | n23 (g5) | 0 -> 0 | band 0 inside [g1] 643; band 0 inside [g1]+[g5] 1715 |
| L_n16_n44_0 | n16 (g1) | n44 (g8) | 0 -> 0 | band 0 inside [g1] 1050; band 0 inside [g8]+[g1] 2611 |
| L_n16_n29_0 | n16 (g1) | n29 (g5) | 0 -> 0 | band 0 inside [g1] 555; band 0 inside [g1]+[g5] 1558 |
| L_n31_n16_0 | n31 (g5) | n16 (g1) | 0 -> 0 | band 0 inside [g5] 1810; band 0 inside [g1] 357 |
| L_n36_n16_0 | n36 (g8) | n16 (g1) | 0 -> 0 | band 0 inside [g8]+[g1] 3051 |
| L_n33_n16_0 | n33 (g5) | n16 (g1) | 0 -> 0 | band 0 inside [g5] 173; band 0 inside [g1]+[g5] 1299 |
| L_n43_n16_0 | n43 (g8) | n16 (g1) | 0 -> 0 | band 0 inside [g8]+[g1] 3757 |
| L_n4_n30_0 | n4 (g1) | n30 (g5) | 0 -> 0 | band 0 inside [g1] 328; band 0 inside [g1]+[g5] 2000 |
| L_n8_n42_0 | n8 (g1) | n42 (g8) | 0 -> 0 | band 0 inside [g8] 1159 |
| L_n28_n42_0 | n28 (g5) | n42 (g8) | 0 -> 0 | band 0 inside [g8]+[g5] 5365 |
| L_n11_n37_0 | n11 (g1) | n37 (g8) | 0 -> 0 | band 0 inside [g1] 1129; band 0 inside [g8] 2045 |
| L_n11_n39_0 | n11 (g1) | n39 (g8) | 0 -> 0 | band 0 inside [g1] 1279; band 0 inside [g8] 2298 |
| L_n31_n38_0 | n31 (g5) | n38 (g8) | 0 -> 0 | band 0 inside [g8] 5199 |
| L_n29_n38_0 | n29 (g5) | n38 (g8) | 0 -> 0 | band 0 inside [g8] 5254 |
| L_n15_n38_0 | n15 (g1) | n38 (g8) | 0 -> 0 | band 0 inside [g1] 988; band 0 inside [g8] 2158 |
| L_n4_n43_0 | n4 (g1) | n43 (g8) | 0 -> 0 | band 0 inside [g1] 2324; band 0 inside [g8]+[g1] 925 |
| L_n5_n44_0 | n5 (g1) | n44 (g8) | 0 -> 0 | band 0 inside [g1] 1160; band 0 inside [g8]+[g1] 1004 |
| L_n22_n40_0 | n22 (g5) | n40 (g8) | 0 -> 0 | band 0 inside [g8]+[g1]+[g5] 4042 |
| L_n25_n42_0 | n25 (g5) | n42 (g8) | 0 -> 0 | band 0 inside [g8]+[g5] 5567 |
| L_n25_n38_0 | n25 (g5) | n38 (g8) | 0 -> 0 | band 0 inside [g8] 5222 |
| L_n20_n37_0 | n20 (g1) | n37 (g8) | 0 -> 0 | band 0 inside [g1] 953; band 0 inside [g8] 2378 |
| L_n7_n33_0 | n7 (g1) | n33 (g5) | 0 -> 0 | band 0 inside [g1]+[g5] 1558 |
| L_n46_n17_0 | n46 (n46) | n17 (g1) | 0 -> 0 | band 0 inside [g1] 3094 |
| L_n46_g1_0 | n46 (n46) | g1 (g1) | 0 -> 0 | band 0 inside [g1] 3555; band 0 inside [g1] 922 |
| L_n46_g9_0 | n46 (n46) | g9 (g8) | 0 -> 0 | straight |
| L_n45_g8_0 | n45 (n45) | g8 (g8) | 0 -> 0 | straight |

Edges drawn through a block they do not belong to (20 cases): L_n16_n40_0 through [g2] for 1498 px; L_n16_n40_0 through [g3] for 815 px; L_n16_n41_0 through [g2] for 1500 px; L_n16_n41_0 through [g3] for 822 px; L_n16_n43_0 through [g2] for 1499 px; L_n16_n43_0 through [g3] for 816 px; L_n16_n23_0 through [g6] for 1288 px; L_n16_n23_0 through [g7] for 617 px; L_n16_n44_0 through [g2] for 1498 px; L_n16_n44_0 through [g3] for 815 px; L_n16_n29_0 through [g7] for 617 px; L_n36_n16_0 through [g2] for 1494 px; L_n36_n16_0 through [g3] for 816 px; L_n43_n16_0 through [g2] for 1498 px; L_n43_n16_0 through [g3] for 815 px; L_n4_n30_0 through [g7] for 611 px; L_n22_n40_0 through [g1] for 3674 px; L_n22_n40_0 through [g4] for 1996 px; L_n22_n40_0 through [g2] for 1493 px; L_n22_n40_0 through [g3] for 821 px.

Edges drawn over a node that is not their end (23 cases): L_n16_n40_0 over n8 for 186 px; L_n16_n40_0 over n11 for 66 px; L_n16_n41_0 over n8 for 12 px; L_n16_n41_0 over n40 for 90 px; L_n16_n35_0 over n32 for 36 px; L_n16_n35_0 over n34 for 150 px; L_n16_n23_0 over n25 for 150 px; L_n16_n23_0 over n29 for 174 px; L_n16_n23_0 over n35 for 144 px; L_n16_n44_0 over n41 for 186 px; L_n16_n29_0 over n25 for 144 px; L_n16_n29_0 over n35 for 144 px; L_n4_n30_0 over n33 for 174 px; L_n28_n42_0 over n37 for 150 px; L_n5_n44_0 over n41 for 180 px; L_n22_n40_0 over n8 for 186 px; L_n22_n40_0 over n9 for 150 px; L_n22_n40_0 over n11 for 150 px; L_n22_n40_0 over n13 for 150 px; L_n22_n40_0 over n15 for 144 px; L_n22_n40_0 over n19 for 24 px; L_n25_n42_0 over n37 for 150 px; L_n2_n7_0 over n4 for 144 px.

Edges drawn over a block title: none.

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_n16_n32_0 | 83x21 | band 0 inside [g1] | 2 | 418 | 0.38 | line L_n22_n40_0 |
| L_n16_n31_0 | 125x21 | band 0 inside [g1] | 3 | 366 | 0.16 | - |
| L_n16_n40_0 | 23x21 | band 0 inside [g1] | 10 | 845 | 0.28 | - |
| L_n16_n17_0 | 129x21 | band 0 inside [g1] | 56 | 60 | 0.92 | - |
| L_n16_n41_0 | 102x21 | band 0 inside [g1] | 10 | 930 | 0.27 | line L_n16_n40_0 |
| L_n16_n35_0 | 41x21 | band 0 inside [g1] | 6 | 497 | 0.33 | line L_n16_n32_0, line L_n22_n40_0 |
| L_n16_n43_0 | 58x21 | band 0 inside [g8] | 4 | 54 | 0.99 | - |
| L_n16_n37_0 | 90x21 | band 0 inside [g1] | 3 | 246 | 0.07 | - |
| L_n16_n21_0 | 18x21 | band 0 inside [g1] | 18 | 53 | 0.35 | - |
| L_n16_n23_0 | 105x21 | band 0 inside [g1] | 10 | 641 | 0.27 | line L_n16_n35_0, line L_n16_n29_0 |
| L_n16_n44_0 | 99x21 | band 0 inside [g1] | 9 | 1049 | 0.29 | line L_n16_n40_0, line L_n16_n41_0 |
| L_n16_n29_0 | 31x21 | band 0 inside [g1] | 10 | 557 | 0.26 | line L_n16_n35_0, line L_n22_n40_0 |
| L_n8_n42_0 | 90x21 | band 0 inside [g8] | 73 | 36 | 0.97 | - |
| L_n28_n42_0 | 66x21 | band 0 inside [g5] | 28 | 150 | 0.03 | line L_n16_n31_0, line L_n31_n16_0 |
| L_n11_n37_0 | 47x21 | band 0 inside [g8] | 27 | 24 | 0.99 | - |
| L_n31_n38_0 | 72x21 | band 0 inside [g8] | 34 | 12 | 1.00 | - |
| L_n22_n40_0 | 138x21 | band 0 inside [g5] | 46 | 30 | 0.01 | line L_n16_n23_0 |
| L_n25_n38_0 | 16x21 | band 0 inside [g5] | 3 | 384 | 0.07 | - |
| L_n20_n37_0 | 160x21 | band 0 inside [g8] | 23 | 78 | 0.98 | line L_n16_n37_0, line L_n11_n37_0 |
| L_n2_n7_0 | 154x21 | band 0 inside [g1] | 1 | 1239 | 0.53 | - |
| L_n29_n31_0 | 44x21 | band 0 inside [g5] | 0 | 53 | 0.77 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=932: 10 labels; y=1248: 5 labels.

Labels: 21; more than 12 px off their own line: 8; more than 300 px of path from the nearer end: 10; overlapping something: 9.

Totals: canvas area 11.52 Mpx2, edge length 105077 px, bends 173, edge crossings (drawn polylines) 52.

