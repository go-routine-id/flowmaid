#### perm-b-subgraphs-reversed.elk.svg  (canvas 2422 x 2266, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 1022 | 206 | 1009 | 201 | 9% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 8 | 248 | 402 | 159 | 7% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 527 | 628 | 602 | 714 | 32% | 5 (5 incl. nested) | 3 / 1 / 1 |
| STG | 0 | - | 988 | 1625 | 472 | 633 | 28% | 4 (4 incl. nested) | 1 / 2 / 1 |
| DR | 0 | - | 1558 | 1625 | 604 | 285 | 13% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U1 , T1 , U2 | 132 |
| 1 | 206 | 407 | 201 | [GH] , [EXT] | 221 |
| 2 | 628 | 1342 | 714 | [PROD] | 283 |
| 3 | 1625 | 2258 | 633 | [STG] , [DR] | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U1 -> T1 = 43; band 0: T1 -> U2 = 40; band 1: [GH] -> [EXT] = 612; band 3: [STG] -> [DR] = 98.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 370 | 1 (176) |
| gap 1-2 | 15 | 4714 | 6 (437, 457, 477, 538, 558, 578) |
| band 2 inside [PROD] | 1 | 72 | 1 (748) |
| gap 2-3 | 12 | 5618 | 8 (1372, 1475, 1495, 1515, 1535, 1555, 1575, 1595) |
| band 3 inside [STG] | 2 | 59 | 2 (2021, 2033) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | gap 0-1 187 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | straight |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | gap 0-1 184 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 2 | gap 1-2 25; gap 1-2 725 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 3 | gap 1-2 68; gap 1-2 116; gap 2-3 935 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 3 | gap 1-2 75; gap 2-3 1197 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 2 | gap 1-2 512 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 3 | gap 1-2 142; gap 2-3 445 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 3 | gap 1-2 242; gap 2-3 507 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 3 | gap 1-2 56; gap 2-3 305 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 3 | gap 1-2 218; gap 2-3 317 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 3 | gap 1-2 373; gap 2-3 288 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 2 | gap 1-2 294; gap 1-2 562 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 3 -> 1 | gap 2-3 231; gap 1-2 64 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 2 -> 1 | band 2 inside [PROD] 72; gap 1-2 397 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 3 -> 1 | gap 2-3 135; gap 1-2 41 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 2 -> 1 | gap 1-2 525; gap 1-2 61 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 2 -> 1 | gap 1-2 217 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 2 -> 3 | gap 2-3 16; gap 2-3 125; band 3 inside [STG] 31 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 2 -> 3 | gap 2-3 146; gap 2-3 357; band 3 inside [STG] 28 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 2 -> 3 | gap 2-3 124; gap 2-3 489 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_PSDEV_0 over title of [DR]; L_R1_ARGO_0 over title of [PROD]; L_ARGO_TG_0 over title of [PROD]; L_LANE_OAI_0 over title of [PROD]; L_LANE_AZ_0 over title of [PROD]; L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 1 | 60 | 0.17 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 2 | 59 | 0.33 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 2 | 59 | 0.17 | - |
| L_R1_ARGO_0 | 128x21 | gap 1-2 | 1 | 144 | 0.12 | - |
| L_R2_STG_0 | 75x21 | band 2 beside blocks | 3 | 772 | 0.33 | - |
| L_R2_DR_0 | 141x21 | band 2 beside blocks | 1 | 671 | 0.26 | - |
| L_R2_PROD_0 | 160x21 | gap 1-2 | 1 | 126 | 0.17 | - |
| L_DNS_TR2_0 | 93x21 | band 2 beside blocks | 2 | 737 | 0.39 | - |
| L_DNS_PSPROD_0 | 104x21 | band 2 beside blocks | 0 | 839 | 0.38 | - |
| L_DNS_DRAPP_0 | 200x42 | band 2 beside blocks | 1 | 653 | 0.39 | - |
| L_DNS_PSDEV_0 | 123x21 | band 2 beside blocks | 0 | 816 | 0.44 | - |
| L_DNS_SITE_0 | 127x21 | band 2 beside blocks | 2 | 972 | 0.49 | - |
| L_DNS_TR1_0 | 101x21 | gap 1-2 | 0 | 413 | 0.32 | - |
| L_DRAPP_TG_0 | 90x21 | band 2 beside blocks | 2 | 659 | 0.59 | - |
| L_ARGO_TG_0 | 86x21 | gap 1-2 | 3 | 125 | 0.86 | - |
| L_PSPROD_TG_0 | 148x21 | band 2 beside blocks | 2 | 636 | 0.61 | - |
| L_LANE_OAI_0 | 46x21 | gap 1-2 | 3 | 174 | 0.85 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 2-3 | 3 | 221 | 0.21 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 2-3 | 3 | 647 | 0.39 | - |
| L_PG_DRAPP_0 | 200x63 | gap 2-3 | 3 | 227 | 0.22 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=508: 5 labels; y=984: 9 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 11; overlapping something: 0.

Totals: canvas area 5.49 Mpx2, edge length 29287 px, bends 157, edge crossings (drawn polylines) 19.

