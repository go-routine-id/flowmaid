#### perm-d-edges-rotated.elk.svg  (canvas 2376 x 2206, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 548 | 206 | 1009 | 201 | 9% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 1913 | 248 | 402 | 159 | 7% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 1017 | 608 | 602 | 714 | 32% | 5 (5 incl. nested) | 3 / 1 / 1 |
| DR | 0 | - | 324 | 1565 | 604 | 345 | 16% | 3 (3 incl. nested) | 3 |
| STG | 0 | - | 1123 | 1565 | 472 | 633 | 29% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U1 , T1 , U2 | 132 |
| 1 | 206 | 407 | 201 | [EXT] , [GH] | 201 |
| 2 | 608 | 1322 | 714 | [PROD] | 243 |
| 3 | 1565 | 2198 | 633 | [DR] , [STG] | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U1 -> T1 = 43; band 0: T1 -> U2 = 40; band 1: [EXT] -> [GH] = 356; band 3: [DR] -> [STG] = 195.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 370 | 1 (176) |
| gap 1-2 | 14 | 5424 | 6 (437, 457, 477, 538, 558, 578) |
| band 2 inside [PROD] | 1 | 74 | 1 (740) |
| gap 2-3 | 12 | 5170 | 6 (1352, 1455, 1475, 1495, 1515, 1535) |
| band 3 inside [DR] | 1 | 181 | 1 (1685) |
| band 3 inside [STG] | 3 | 158 | 3 (1751, 1961, 1973) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 3 | gap 1-2 365; gap 2-3 358; band 3 inside [DR] 181 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 3 | gap 1-2 532; gap 2-3 354 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 3 | gap 1-2 200; gap 2-3 187 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 2 | gap 1-2 147; gap 1-2 478 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 3 -> 1 | gap 2-3 105; gap 1-2 290 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 2 -> 1 | band 2 inside [PROD] 74; gap 1-2 169 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 3 -> 1 | band 3 inside [STG] 100; gap 2-3 175; gap 1-2 445 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 2 -> 1 | gap 1-2 404; gap 1-2 382 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 2 -> 1 | gap 1-2 490; gap 1-2 141 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 2 -> 3 | gap 2-3 8; gap 2-3 98; band 3 inside [STG] 30 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 2 -> 3 | gap 2-3 87; gap 2-3 69; band 3 inside [STG] 27 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 2 -> 3 | gap 2-3 203; gap 2-3 272 |
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | gap 0-1 187 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | straight |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | gap 0-1 184 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 2 | gap 1-2 25; gap 1-2 447 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 3 | gap 1-2 83; gap 2-3 761 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 3 | gap 1-2 68; gap 2-3 1242 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 2 | gap 1-2 664 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 3 | gap 2-3 595 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 3 | gap 1-2 94; gap 2-3 655 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_DNS_TR1_0 over title of [PROD]; L_ARGO_TG_0 over title of [PROD]; L_LANE_OAI_0 over title of [PROD]; L_LANE_AZ_0 over title of [PROD]; L_DNS_DRAPP_0 over title of [DR]; L_DNS_SITE_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_DNS_DRAPP_0 | 200x42 | band 2 beside blocks | 1 | 940 | 0.42 | - |
| L_DNS_PSDEV_0 | 123x21 | band 2 beside blocks | 1 | 1096 | 0.50 | - |
| L_DNS_SITE_0 | 127x21 | band 2 beside blocks | 2 | 774 | 0.45 | - |
| L_DNS_TR1_0 | 101x21 | gap 1-2 | 3 | 263 | 0.25 | - |
| L_DRAPP_TG_0 | 90x21 | band 2 beside blocks | 1 | 850 | 0.50 | - |
| L_ARGO_TG_0 | 86x21 | gap 1-2 | 3 | 286 | 0.53 | - |
| L_PSPROD_TG_0 | 148x21 | band 2 beside blocks | 1 | 1017 | 0.52 | - |
| L_LANE_OAI_0 | 46x21 | gap 1-2 | 0 | 498 | 0.63 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 2-3 | 1 | 221 | 0.22 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 2-3 | 0 | 586 | 0.46 | - |
| L_PG_DRAPP_0 | 200x63 | gap 2-3 | 1 | 305 | 0.35 | - |
| L_U1_DNS_0 | 199x21 | gap 0-1 | 1 | 60 | 0.17 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 2 | 59 | 0.33 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 2 | 59 | 0.17 | - |
| L_R1_ARGO_0 | 128x21 | gap 1-2 | 1 | 144 | 0.17 | - |
| L_R2_STG_0 | 75x21 | band 2 beside blocks | 1 | 659 | 0.33 | - |
| L_R2_DR_0 | 141x21 | band 2 beside blocks | 2 | 647 | 0.26 | - |
| L_R2_PROD_0 | 160x21 | gap 1-2 | 1 | 126 | 0.14 | - |
| L_DNS_TR2_0 | 93x21 | band 2 beside blocks | 1 | 587 | 0.31 | - |
| L_DNS_PSPROD_0 | 104x21 | band 2 beside blocks | 1 | 672 | 0.31 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=508: 5 labels; y=964: 9 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 12; overlapping something: 0.

Totals: canvas area 5.24 Mpx2, edge length 29336 px, bends 153, edge crossings (drawn polylines) 19.

