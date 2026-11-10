#### infra-overview-v2.elk-wide.svg  (canvas 2150 x 2324, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 306 | 226 | 1139 | 159 | 7% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 1485 | 259 | 525 | 138 | 6% | 2 (2 incl. nested) | 2 |
| DR | 0 | - | 459 | 699 | 872 | 201 | 9% | 3 (3 incl. nested) | 3 |
| PROD | 0 | - | 1006 | 1249 | 894 | 453 | 19% | 5 (5 incl. nested) | 3 / 1 / 1 |
| STG | 0 | - | 181 | 1863 | 656 | 453 | 19% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U1 , T1 , U2 | 152 |
| 1 | 226 | 397 | 171 | [EXT] , [GH] | 302 |
| 2 | 699 | 900 | 201 | [DR] | 349 |
| 3 | 1249 | 1702 | 453 | [PROD] | 161 |
| 4 | 1863 | 2316 | 453 | [STG] | - |

Top-level blocks: 5; bands: 5; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U1 -> T1 = 41; band 0: T1 -> U2 = 40; band 1: [EXT] -> [GH] = 40.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 3 | 452 | 2 (176, 196) |
| band 1 beside blocks | 2 | 629 | 1 (391) |
| gap 1-2 | 15 | 5459 | 8 (403, 487, 507, 527, 547, 567, 587, 669) |
| gap 2-3 | 7 | 756 | 3 (1179, 1199, 1219) |
| band 3 inside [PROD] | 2 | 37 | 1 (1435) |
| gap 3-4 | 6 | 4013 | 4 (1732, 1793, 1813, 1833) |
| band 4 inside [STG] | 2 | 72 | 1 (2175) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | gap 0-1 272 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | gap 0-1 90 |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | gap 0-1 90 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 3 | gap 1-2 101; gap 2-3 222 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 4 | gap 1-2 224; gap 1-2 205; gap 3-4 1315 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 2 | gap 1-2 248; gap 1-2 102 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 3 | gap 1-2 91; gap 2-3 90 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 4 | gap 1-2 572; gap 3-4 284 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 4 | gap 1-2 400; gap 3-4 356 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 2 | gap 1-2 226; gap 1-2 185 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 2 | gap 1-2 187; gap 1-2 118 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 2 | gap 1-2 64; gap 1-2 246 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 3 | gap 1-2 170; gap 1-2 563; gap 2-3 40 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 2 -> 1 | gap 1-2 22; gap 1-2 295 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 3 -> 1 | gap 2-3 190; gap 1-2 506; band 1 beside blocks 426 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 4 -> 1 | gap 3-4 269; gap 1-2 267; band 1 beside blocks 203 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 3 -> 1 | band 3 inside [PROD] 18; gap 2-3 37; gap 1-2 242 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 3 -> 1 | band 3 inside [PROD] 18; gap 2-3 14; gap 1-2 424 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 3 -> 4 | gap 3-4 23; gap 3-4 827; band 4 inside [STG] 44 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 3 -> 4 | gap 3-4 23; gap 3-4 916; band 4 inside [STG] 28 |
| L_DRAPP_PG_0 | DRAPP (DR) | PG (PROD) | 2 -> 3 | gap 2-3 163 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_DNS_TR1_0 over title of [PROD]; L_LANE_OAI_0 over title of [PROD]; L_LANE_AZ_0 over title of [PROD]; L_DRAPP_PG_0 over title of [PROD]; L_DNS_SITE_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 2 | 59 | 0.13 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 1 | 60 | 0.21 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 1 | 60 | 0.21 | - |
| L_R1_ARGO_0 | 128x21 | band 2 beside blocks | 3 | 592 | 0.46 | - |
| L_R2_STG_0 | 75x21 | gap 2-3 | 3 | 1175 | 0.37 | - |
| L_R2_DR_0 | 141x21 | gap 1-2 | 2 | 161 | 0.76 | - |
| L_R2_PROD_0 | 160x21 | band 2 beside blocks | 2 | 455 | 0.56 | - |
| L_DNS_TR2_0 | 93x21 | gap 2-3 | 3 | 1103 | 0.55 | - |
| L_DNS_PSPROD_0 | 104x21 | gap 2-3 | 0 | 1163 | 0.48 | - |
| L_DNS_DRAPP_0 | 200x42 | gap 1-2 | 2 | 335 | 0.59 | - |
| L_DNS_PSDEV_0 | 123x21 | gap 1-2 | 1 | 270 | 0.62 | - |
| L_DNS_SITE_0 | 127x21 | gap 1-2 | 3 | 323 | 0.45 | - |
| L_DNS_TR1_0 | 101x21 | band 2 beside blocks | 1 | 509 | 0.71 | - |
| L_DRAPP_TG_0 | 90x21 | gap 1-2 | 2 | 180 | 0.25 | - |
| L_ARGO_TG_0 | 86x21 | band 2 beside blocks | 1 | 653 | 0.31 | - |
| L_PSPROD_TG_0 | 148x21 | gap 2-3 | 3 | 1191 | 0.49 | - |
| L_LANE_OAI_0 | 46x21 | band 2 beside blocks | 3 | 634 | 0.46 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 3-4 | 1 | 228 | 0.15 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 3-4 | 2 | 377 | 0.22 | - |
| L_DRAPP_PG_0 | 200x63 | gap 2-3 | 1 | 251 | 0.28 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=628: 5 labels; y=872: 5 labels; y=1128: 5 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 12; overlapping something: 0.

Totals: canvas area 5.00 Mpx2, edge length 29416 px, bends 169, edge crossings (drawn polylines) 13.

