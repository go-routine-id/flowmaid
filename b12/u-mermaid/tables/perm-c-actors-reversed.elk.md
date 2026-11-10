#### perm-c-actors-reversed.elk.svg  (canvas 2205 x 2378, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 440 | 206 | 1009 | 201 | 8% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 1742 | 248 | 402 | 159 | 7% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 1176 | 708 | 579 | 726 | 31% | 5 (5 incl. nested) | 3 / 1 / 1 |
| DR | 0 | - | 209 | 1737 | 604 | 285 | 12% | 3 (3 incl. nested) | 3 |
| STG | 0 | - | 933 | 1737 | 472 | 633 | 27% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U1 , U2 , T1 | 132 |
| 1 | 206 | 407 | 201 | [EXT] , [GH] | 301 |
| 2 | 708 | 1434 | 726 | [PROD] | 303 |
| 3 | 1737 | 2370 | 633 | [DR] , [STG] | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U1 -> U2 = 40; band 0: U2 -> T1 = 40; band 1: [EXT] -> [GH] = 292; band 3: [DR] -> [STG] = 121.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 368 | 1 (176) |
| gap 1-2 | 15 | 4700 | 11 (437, 457, 477, 497, 517, 537, 557, 577, 638, 658, 678) |
| band 2 inside [PROD] | 6 | 375 | 5 (828, 840, 1029, 1041, 1272) |
| gap 2-3 | 12 | 5525 | 9 (1464, 1567, 1587, 1607, 1627, 1647, 1667, 1687, 1707) |
| band 3 inside [STG] | 2 | 58 | 2 (2133, 2145) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | gap 0-1 184 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | gap 0-1 184 |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | straight |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 2 | gap 1-2 25; gap 1-2 101; band 2 inside [PROD] 39 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 3 | gap 1-2 83; gap 2-3 795 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 3 | gap 1-2 68; gap 2-3 1186 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 2 | gap 1-2 317 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 3 | gap 1-2 100; gap 2-3 410 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 3 | gap 1-2 200; gap 2-3 472 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 3 | gap 1-2 104; gap 2-3 254 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 3 | gap 1-2 424; gap 2-3 239 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 3 | gap 1-2 269; gap 2-3 268 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 2 | gap 1-2 253; gap 1-2 436 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 3 -> 1 | gap 2-3 198; gap 1-2 201 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 2 -> 1 | band 2 inside [PROD] 40; gap 1-2 476; gap 1-2 342 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 3 -> 1 | gap 2-3 224; gap 1-2 307 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 2 -> 1 | band 2 inside [PROD] 144; band 2 inside [PROD] 67; gap 1-2 319 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 2 -> 1 | band 2 inside [PROD] 51; gap 1-2 173; gap 1-2 500 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 2 -> 3 | band 2 inside [PROD] 22; gap 2-3 16; gap 2-3 266; band 3 inside [STG] 30 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 2 -> 3 | band 2 inside [PROD] 12; gap 2-3 54; gap 2-3 368; band 3 inside [STG] 27 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 2 -> 3 | gap 2-3 238; gap 2-3 538 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_ARGO_TG_0 over title of [PROD]; L_DNS_SITE_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 2 | 59 | 0.17 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 2 | 59 | 0.17 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 2 | 59 | 0.33 | - |
| L_R1_ARGO_0 | 128x21 | gap 1-2 | 0 | 245 | 0.39 | - |
| L_R2_STG_0 | 75x21 | band 2 beside blocks | 1 | 767 | 0.35 | - |
| L_R2_DR_0 | 141x21 | band 2 beside blocks | 1 | 750 | 0.29 | - |
| L_R2_PROD_0 | 160x21 | gap 1-2 | 2 | 227 | 0.36 | - |
| L_DNS_TR2_0 | 93x21 | band 2 beside blocks | 2 | 785 | 0.40 | - |
| L_DNS_PSPROD_0 | 104x21 | band 2 beside blocks | 3 | 880 | 0.40 | - |
| L_DNS_DRAPP_0 | 200x42 | band 2 beside blocks | 1 | 786 | 0.44 | - |
| L_DNS_PSDEV_0 | 123x21 | band 2 beside blocks | 3 | 983 | 0.53 | - |
| L_DNS_SITE_0 | 127x21 | band 2 beside blocks | 1 | 953 | 0.48 | - |
| L_DNS_TR1_0 | 101x21 | gap 1-2 | 0 | 472 | 0.39 | - |
| L_DRAPP_TG_0 | 90x21 | band 2 beside blocks | 2 | 881 | 0.52 | - |
| L_ARGO_TG_0 | 86x21 | gap 1-2 | 0 | 558 | 0.58 | - |
| L_PSPROD_TG_0 | 148x21 | band 2 beside blocks | 2 | 983 | 0.53 | - |
| L_LANE_OAI_0 | 46x21 | gap 1-2 | 1 | 533 | 0.56 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 2-3 | 1 | 300 | 0.24 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 2-3 | 3 | 575 | 0.36 | - |
| L_PG_DRAPP_0 | 200x63 | gap 2-3 | 3 | 341 | 0.29 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=608: 5 labels; y=1072: 9 labels; y=1516: 3 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 14; overlapping something: 0.

Totals: canvas area 5.24 Mpx2, edge length 31204 px, bends 159, edge crossings (drawn polylines) 16.

