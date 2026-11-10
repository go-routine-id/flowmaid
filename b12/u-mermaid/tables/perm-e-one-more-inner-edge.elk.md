#### perm-e-one-more-inner-edge.elk.svg  (canvas 2419 x 2254, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 777 | 206 | 1009 | 201 | 9% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 61 | 248 | 402 | 159 | 7% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 773 | 648 | 518 | 702 | 31% | 5 (5 incl. nested) | 2 / 2 / 1 |
| STG | 0 | - | 936 | 1613 | 472 | 633 | 28% | 4 (4 incl. nested) | 1 / 2 / 1 |
| DR | 0 | - | 1595 | 1613 | 604 | 285 | 13% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U2 , T1 , U1 | 132 |
| 1 | 206 | 407 | 201 | [GH] , [EXT] | 241 |
| 2 | 648 | 1350 | 702 | [PROD] | 263 |
| 3 | 1613 | 2246 | 633 | [STG] , [DR] | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U2 -> T1 = 40; band 0: T1 -> U1 = 44; band 1: [GH] -> [EXT] = 314; band 3: [STG] -> [DR] = 187.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 28 total, 7 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 371 | 1 (176) |
| gap 1-2 | 14 | 5570 | 8 (437, 457, 477, 497, 558, 578, 598, 618) |
| band 2 inside [PROD] | 1 | 78 | 1 (768) |
| gap 2-3 | 12 | 5613 | 7 (1380, 1483, 1503, 1523, 1543, 1563, 1583) |
| band 3 inside [STG] | 3 | 160 | 3 (1799, 2009, 2021) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | gap 0-1 187 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | straight |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | gap 0-1 184 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 2 | gap 1-2 46; gap 1-2 443 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 3 | gap 1-2 83; gap 2-3 915 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 3 | gap 1-2 68; gap 2-3 1389 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 2 | gap 1-2 678 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 3 | gap 1-2 93; gap 2-3 483 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 3 | gap 2-3 544 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 3 | gap 1-2 298; gap 2-3 265 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 3 | gap 1-2 617; gap 2-3 251 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 3 | gap 1-2 462; gap 2-3 279 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 2 | gap 1-2 60; gap 1-2 502 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 3 -> 1 | gap 2-3 318; gap 1-2 62 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 2 -> 1 | band 2 inside [PROD] 78; gap 1-2 85; gap 1-2 224 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 3 -> 1 | band 3 inside [STG] 102; gap 2-3 313; gap 1-2 85; gap 1-2 505 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 2 -> 1 | gap 1-2 425; gap 1-2 340 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 2 -> 1 | gap 1-2 408; gap 1-2 86 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 2 -> 3 | gap 2-3 50; band 3 inside [STG] 31 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 2 -> 3 | gap 2-3 96; gap 2-3 217; band 3 inside [STG] 28 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 2 -> 3 | gap 2-3 32; gap 2-3 460 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_DNS_TR1_0 over title of [PROD]; L_ARGO_TG_0 over title of [PROD]; L_LANE_OAI_0 over title of [PROD]; L_LANE_AZ_0 over title of [PROD]; L_DNS_SITE_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 1 | 60 | 0.17 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 2 | 59 | 0.33 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 2 | 59 | 0.17 | - |
| L_R1_ARGO_0 | 128x21 | gap 1-2 | 0 | 185 | 0.21 | - |
| L_R2_STG_0 | 75x21 | band 2 beside blocks | 2 | 696 | 0.31 | - |
| L_R2_DR_0 | 141x21 | band 2 beside blocks | 2 | 677 | 0.25 | - |
| L_R2_PROD_0 | 160x21 | gap 1-2 | 1 | 144 | 0.15 | - |
| L_DNS_TR2_0 | 93x21 | band 2 beside blocks | 2 | 701 | 0.37 | - |
| L_DNS_PSPROD_0 | 104x21 | band 2 beside blocks | 2 | 622 | 0.31 | - |
| L_DNS_DRAPP_0 | 200x42 | band 2 beside blocks | 2 | 910 | 0.49 | - |
| L_DNS_PSDEV_0 | 123x21 | band 2 beside blocks | 1 | 946 | 0.56 | - |
| L_DNS_SITE_0 | 127x21 | band 2 beside blocks | 2 | 976 | 0.52 | - |
| L_DNS_TR1_0 | 101x21 | gap 1-2 | 1 | 198 | 0.20 | - |
| L_DRAPP_TG_0 | 90x21 | band 2 beside blocks | 1 | 670 | 0.60 | - |
| L_ARGO_TG_0 | 86x21 | gap 1-2 | 1 | 358 | 0.54 | - |
| L_PSPROD_TG_0 | 148x21 | band 2 beside blocks | 1 | 1193 | 0.51 | - |
| L_LANE_OAI_0 | 46x21 | gap 1-2 | 3 | 473 | 0.65 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 2-3 | 2 | 276 | 0.29 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 2-3 | 2 | 594 | 0.41 | - |
| L_PG_DRAPP_0 | 200x63 | gap 2-3 | 1 | 132 | 0.15 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=528: 5 labels; y=1000: 9 labels; y=1432: 3 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 12; overlapping something: 0.

Totals: canvas area 5.45 Mpx2, edge length 30274 px, bends 151, edge crossings (drawn polylines) 18.

