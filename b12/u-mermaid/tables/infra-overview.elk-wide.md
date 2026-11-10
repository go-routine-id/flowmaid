#### infra-overview.elk-wide.svg  (canvas 2624 x 1927, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 505 | 206 | 1139 | 159 | 8% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 2072 | 227 | 525 | 138 | 7% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 1255 | 666 | 898 | 537 | 28% | 5 (5 incl. nested) | 3 / 1 / 1 |
| DR | 0 | - | 22 | 1466 | 872 | 201 | 10% | 3 (3 incl. nested) | 3 |
| STG | 0 | - | 976 | 1466 | 656 | 453 | 24% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U2 , U1 , T1 | 132 |
| 1 | 206 | 365 | 159 | [EXT] , [GH] | 301 |
| 2 | 666 | 1203 | 537 | [PROD] | 263 |
| 3 | 1466 | 1919 | 453 | [DR] , [STG] | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U2 -> U1 = 40; band 0: U1 -> T1 = 41; band 1: [EXT] -> [GH] = 428; band 3: [DR] -> [STG] = 82.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 360 | 1 (176) |
| gap 1-2 | 15 | 5092 | 11 (395, 415, 435, 455, 475, 495, 515, 535, 596, 616, 636) |
| band 2 inside [PROD] | 5 | 616 | 4 (786, 798, 924, 936) |
| gap 2-3 | 12 | 6451 | 7 (1233, 1336, 1356, 1376, 1396, 1416, 1436) |
| band 3 inside [STG] | 2 | 72 | 1 (1778) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | straight |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | gap 0-1 181 |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | gap 0-1 179 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 2 | gap 1-2 108; band 2 inside [PROD] 67 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 3 | gap 1-2 69; gap 2-3 994 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 3 | gap 1-2 48; gap 2-3 1551 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 2 | gap 1-2 360 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 3 | gap 1-2 25; gap 2-3 482 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 3 | gap 1-2 122; gap 2-3 626 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 3 | gap 1-2 120; gap 2-3 182 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 3 | gap 1-2 492; gap 2-3 95 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 3 | gap 1-2 305; gap 2-3 159 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 2 | gap 1-2 172; gap 1-2 522 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 3 -> 1 | gap 2-3 182; gap 1-2 117 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 2 -> 1 | band 2 inside [PROD] 67; gap 1-2 573; gap 1-2 420 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 3 -> 1 | gap 2-3 314; gap 1-2 313 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 2 -> 1 | band 2 inside [PROD] 285; band 2 inside [PROD] 107; gap 1-2 56; gap 1-2 439 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 2 -> 1 | band 2 inside [PROD] 79; gap 1-2 167; gap 1-2 662 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 2 -> 3 | gap 2-3 59; gap 2-3 360; band 3 inside [STG] 44 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 2 -> 3 | band 2 inside [PROD] 12; gap 2-3 43; gap 2-3 452; band 3 inside [STG] 28 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 2 -> 3 | gap 2-3 190; gap 2-3 763 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_DNS_SITE_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 2 | 59 | 0.33 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 1 | 60 | 0.17 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 2 | 59 | 0.17 | - |
| L_R1_ARGO_0 | 128x21 | gap 1-2 | 3 | 222 | 0.34 | - |
| L_R2_STG_0 | 75x21 | band 2 beside blocks | 2 | 658 | 0.30 | - |
| L_R2_DR_0 | 141x21 | band 2 beside blocks | 2 | 635 | 0.23 | - |
| L_R2_PROD_0 | 160x21 | gap 1-2 | 3 | 228 | 0.33 | - |
| L_DNS_TR2_0 | 93x21 | band 2 beside blocks | 3 | 616 | 0.36 | - |
| L_DNS_PSPROD_0 | 104x21 | band 2 beside blocks | 3 | 713 | 0.35 | - |
| L_DNS_DRAPP_0 | 200x42 | band 2 beside blocks | 2 | 706 | 0.47 | - |
| L_DNS_PSDEV_0 | 123x21 | band 2 beside blocks | 1 | 708 | 0.60 | - |
| L_DNS_SITE_0 | 127x21 | band 2 beside blocks | 1 | 772 | 0.54 | - |
| L_DNS_TR1_0 | 101x21 | gap 1-2 | 2 | 389 | 0.33 | - |
| L_DRAPP_TG_0 | 90x21 | band 2 beside blocks | 1 | 702 | 0.53 | - |
| L_ARGO_TG_0 | 86x21 | gap 1-2 | 0 | 635 | 0.58 | - |
| L_PSPROD_TG_0 | 148x21 | band 2 beside blocks | 0 | 897 | 0.54 | - |
| L_LANE_OAI_0 | 46x21 | gap 1-2 | 0 | 654 | 0.56 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 2-3 | 3 | 180 | 0.15 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 2-3 | 2 | 437 | 0.31 | - |
| L_PG_DRAPP_0 | 200x63 | gap 2-3 | 3 | 293 | 0.22 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=564: 5 labels; y=936: 9 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 13; overlapping something: 0.

Totals: canvas area 5.06 Mpx2, edge length 30226 px, bends 155, edge crossings (drawn polylines) 16.

