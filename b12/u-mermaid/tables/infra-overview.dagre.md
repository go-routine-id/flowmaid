#### infra-overview.dagre.svg  (canvas 3416 x 1587, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 1858 | 166 | 905 | 1413 | 89% | 5 (5 incl. nested) | 1 / 2 / 2 |
| PROD | 0 | - | 508 | 391 | 885 | 650 | 41% | 5 (5 incl. nested) | 1 / 1 / 3 |
| GH | 0 | - | 1414 | 391 | 424 | 137 | 9% | 2 (2 incl. nested) | 2 |
| STG | 0 | - | 8 | 599 | 480 | 776 | 49% | 4 (4 incl. nested) | 1 / 2 / 1 |
| DR | 0 | - | 2782 | 1154 | 626 | 221 | 14% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U2 , T1 , U1 | 92 |
| 1 | 166 | 1579 | 1413 | [STG] , [PROD] , [GH] , [EXT] , [DR] | - |

Top-level blocks: 5; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): band 1.

Cross-axis gaps between neighbours in the same band: band 0: U2 -> T1 = 263; band 0: T1 -> U1 = 145; band 1: [STG] -> [PROD] = 20; band 1: [PROD] -> [GH] = 20; band 1: [GH] -> [EXT] = 20; band 1: [EXT] -> [DR] = 20.

Sides of block frames crossed by inter-block edges: enters via left: 7, enters via right: 8, enters via top: 6, leaves via bottom: 6, leaves via left: 4, leaves via right: 8.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 1 inside [EXT] | 9 | 7673 | 7 (201, 203, 316, 317, 546, 1069, 1393) |
| band 1 inside [EXT]+[DR] | 3 | 1588 | 1 (1202) |
| band 1 inside [GH] | 1 | 188 | 1 (504) |
| band 1 inside [PROD] | 5 | 3619 | 3 (546, 777, 1000) |
| band 1 inside [PROD]+[EXT] | 2 | 1346 | 2 (879, 881) |
| band 1 inside [PROD]+[GH]+[EXT] | 1 | 636 | 1 (422) |
| band 1 inside [STG] | 2 | 683 | 1 (1069) |
| band 1 inside [STG]+[PROD]+[EXT] | 2 | 3492 | 2 (652, 883) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | band 1 inside [EXT] 353 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | straight |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | band 1 inside [EXT] 273 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 1 | band 1 inside [PROD] 655 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 1 | band 1 inside [STG]+[PROD]+[EXT] 1783 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 1 | band 1 inside [STG]+[PROD]+[EXT] 1709 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 1 | band 1 inside [EXT]+[DR] 529 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 1 | band 1 inside [EXT] 294; band 1 inside [EXT]+[DR] 549 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 1 | band 1 inside [EXT]+[DR] 510 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 1 | band 1 inside [EXT] 265; band 1 inside [PROD]+[GH]+[EXT] 636 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 1 -> 1 | band 1 inside [EXT] 679 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 1 -> 1 | band 1 inside [EXT] 1180 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 1 -> 1 | band 1 inside [EXT] 1683 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 1 -> 1 | band 1 inside [PROD]+[EXT] 594 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 1 -> 1 | band 1 inside [PROD]+[EXT] 752 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 1 -> 1 | band 1 inside [PROD] 613; band 1 inside [STG] 334 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 1 -> 1 | band 1 inside [PROD] 358; band 1 inside [STG] 349 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 1 -> 1 | band 1 inside [EXT] 1835 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 1 | band 1 inside [PROD] 1273 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 1 | band 1 inside [GH] 188; band 1 inside [EXT] 1112 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 1 | band 1 inside [PROD] 719 |

Edges drawn through a block they do not belong to (6 cases): L_DNS_TR2_0 through [PROD] for 881 px; L_DNS_PSPROD_0 through [PROD] for 888 px; L_DNS_TR1_0 through [GH] for 426 px; L_PG_DRAPP_0 through [EXT] for 899 px; L_R2_STG_0 through [PROD] for 879 px; L_R2_DR_0 through [EXT] for 904 px.

Edges drawn over a node that is not their end (10 cases): L_DNS_TR2_0 over LANE for 150 px; L_DNS_PSPROD_0 over ARGO for 150 px; L_DNS_PSPROD_0 over VAULT for 150 px; L_DNS_DRAPP_0 over SITE for 150 px; L_DNS_PSDEV_0 over DRAPP for 150 px; L_DNS_TR1_0 over R1 for 54 px; L_DNS_TR1_0 over R2 for 150 px; L_LANE_AZ_0 over OAI for 150 px; L_ARGO_GITLAB_0 over VAULT for 149 px; L_R2_DR_0 over R1 for 48 px.

Edges drawn over a block title: L_DNS_PSDEV_0 over title of [DR]; L_PG_DRAPP_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 1 | 47 | 0.11 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 1 | 47 | 0.38 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 1 | 47 | 0.14 | - |
| L_R1_ARGO_0 | 128x21 | band 1 inside [PROD] | 2 | 173 | 0.82 | - |
| L_DNS_TR2_0 | 93x21 | band 1 inside [EXT] | 1 | 288 | 0.13 | - |
| L_DNS_PSPROD_0 | 104x21 | band 1 inside [EXT] | 2 | 324 | 0.15 | - |
| L_DNS_DRAPP_0 | 200x42 | band 1 inside [EXT] | 3 | 461 | 0.33 | - |
| L_DNS_PSDEV_0 | 123x21 | band 1 inside [EXT] | 2 | 636 | 0.40 | - |
| L_DNS_SITE_0 | 127x21 | band 1 inside [EXT] | 1 | 388 | 0.30 | - |
| L_DNS_TR1_0 | 101x21 | band 1 inside [EXT] | 13 | 264 | 0.29 | - |
| L_DRAPP_TG_0 | 90x21 | band 1 inside [EXT] | 16 | 138 | 0.83 | - |
| L_ARGO_TG_0 | 86x21 | band 1 inside [EXT] | 1 | 204 | 0.87 | - |
| L_PSPROD_TG_0 | 148x21 | band 1 inside [STG] | 3 | 252 | 0.12 | - |
| L_LANE_OAI_0 | 46x21 | band 1 inside [PROD] | 18 | 24 | 0.04 | - |
| L_LANE_GITLAB_0 | 149x21 | band 1 inside [PROD] | 1 | 461 | 0.59 | - |
| L_ARGO_GITLAB_0 | 64x21 | band 1 inside [STG] | 25 | 66 | 0.91 | - |
| L_PG_DRAPP_0 | 200x63 | band 1 beside blocks | 37 | 48 | 0.97 | - |
| L_R2_STG_0 | 75x21 | band 1 beside blocks | 14 | 48 | 0.96 | - |
| L_R2_DR_0 | 141x21 | band 1 beside blocks | 28 | 323 | 0.83 | - |
| L_R2_PROD_0 | 160x21 | band 1 inside [PROD] | 172 | 0 | 1.00 | node LANE |

Label rows (>= 3 labels centred on the same flow coordinate): y=120: 3 labels; y=708: 4 labels.

Labels: 20; more than 12 px off their own line: 8; more than 300 px of path from the nearer end: 6; overlapping something: 1.

Totals: canvas area 5.42 Mpx2, edge length 25429 px, bends 119, edge crossings (drawn polylines) 23.

