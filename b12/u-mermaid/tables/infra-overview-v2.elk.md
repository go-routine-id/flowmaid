#### infra-overview-v2.elk.svg  (canvas 1948 x 2755, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 379 | 226 | 1009 | 201 | 7% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 1428 | 280 | 402 | 159 | 6% | 2 (2 incl. nested) | 2 |
| DR | 0 | - | 474 | 721 | 604 | 285 | 10% | 3 (3 incl. nested) | 3 |
| PROD | 0 | - | 943 | 1311 | 575 | 642 | 23% | 5 (5 incl. nested) | 3 / 1 / 1 |
| STG | 0 | - | 270 | 2114 | 472 | 633 | 23% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U1 , T1 , U2 | 152 |
| 1 | 226 | 439 | 213 | [EXT] , [GH] | 282 |
| 2 | 721 | 1006 | 285 | [DR] | 305 |
| 3 | 1311 | 1953 | 642 | [PROD] | 161 |
| 4 | 2114 | 2747 | 633 | [STG] | - |

Top-level blocks: 5; bands: 5; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U1 -> T1 = 43; band 0: T1 -> U2 = 40; band 1: [EXT] -> [GH] = 40.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 3 | 462 | 2 (176, 196) |
| band 1 beside blocks | 2 | 574 | 1 (433) |
| gap 1-2 | 15 | 4430 | 7 (445, 529, 549, 569, 589, 609, 691) |
| gap 2-3 | 7 | 1093 | 5 (1201, 1221, 1241, 1261, 1281) |
| gap 3-4 | 6 | 3626 | 4 (1983, 2044, 2064, 2084) |
| band 4 inside [STG] | 2 | 58 | 2 (2510, 2522) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | gap 0-1 279 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | gap 0-1 92 |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | gap 0-1 92 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 3 | gap 1-2 81; gap 2-3 372 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 4 | gap 1-2 181; gap 1-2 160; gap 3-4 1201 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 2 | gap 1-2 359; gap 1-2 124 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 3 | gap 1-2 12; gap 2-3 173 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 4 | gap 1-2 581; gap 3-4 342 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 4 | gap 1-2 444; gap 3-4 373 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 2 | gap 1-2 98; gap 1-2 148 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 2 | gap 1-2 221; gap 1-2 123 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 2 | gap 1-2 70; gap 1-2 154 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 3 | gap 1-2 167; gap 1-2 363; gap 2-3 40 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 2 -> 1 | gap 1-2 117 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 3 -> 1 | gap 2-3 288; gap 1-2 384; band 1 beside blocks 383 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 4 -> 1 | gap 3-4 260; gap 1-2 311; band 1 beside blocks 191 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 3 -> 1 | gap 2-3 40; gap 1-2 86 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 3 -> 1 | gap 2-3 28; gap 1-2 244 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 3 -> 4 | gap 3-4 658; band 4 inside [STG] 30 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 3 -> 4 | gap 3-4 32; gap 3-4 760; band 4 inside [STG] 27 |
| L_DRAPP_PG_0 | DRAPP (DR) | PG (PROD) | 2 -> 3 | gap 2-3 152 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_DNS_TR1_0 over title of [PROD]; L_LANE_OAI_0 over title of [PROD]; L_LANE_AZ_0 over title of [PROD]; L_DRAPP_PG_0 over title of [PROD]; L_DNS_SITE_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 2 | 59 | 0.13 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 2 | 59 | 0.21 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 2 | 59 | 0.21 | - |
| L_R1_ARGO_0 | 128x21 | band 2 beside blocks | 1 | 557 | 0.39 | - |
| L_R2_STG_0 | 75x21 | gap 2-3 | 2 | 1068 | 0.33 | - |
| L_R2_DR_0 | 141x21 | gap 1-2 | 1 | 185 | 0.77 | - |
| L_R2_PROD_0 | 160x21 | band 2 beside blocks | 2 | 486 | 0.46 | - |
| L_DNS_TR2_0 | 93x21 | gap 2-3 | 3 | 1325 | 0.49 | - |
| L_DNS_PSPROD_0 | 104x21 | gap 2-3 | 3 | 1187 | 0.43 | - |
| L_DNS_DRAPP_0 | 200x42 | gap 1-2 | 1 | 299 | 0.53 | - |
| L_DNS_PSDEV_0 | 123x21 | gap 1-2 | 1 | 276 | 0.63 | - |
| L_DNS_SITE_0 | 127x21 | gap 1-2 | 0 | 306 | 0.50 | - |
| L_DNS_TR1_0 | 101x21 | band 2 beside blocks | 0 | 569 | 0.64 | - |
| L_DRAPP_TG_0 | 90x21 | gap 1-2 | 1 | 162 | 0.31 | - |
| L_ARGO_TG_0 | 86x21 | band 2 beside blocks | 1 | 790 | 0.38 | - |
| L_PSPROD_TG_0 | 148x21 | gap 2-3 | 3 | 1252 | 0.53 | - |
| L_LANE_OAI_0 | 46x21 | gap 2-3 | 4 | 365 | 0.28 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 3-4 | 1 | 251 | 0.17 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 3-4 | 1 | 509 | 0.28 | - |
| L_DRAPP_PG_0 | 200x63 | gap 2-3 | 0 | 168 | 0.17 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=648: 5 labels; y=896: 4 labels; y=1148: 5 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 11; overlapping something: 0.

Totals: canvas area 5.37 Mpx2, edge length 29757 px, bends 163, edge crossings (drawn polylines) 13.

