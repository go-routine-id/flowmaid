#### perm-a-edges-reversed.elk.svg  (canvas 2034 x 2837, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| GH | 0 | - | 1273 | 8 | 402 | 159 | 6% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 976 | 368 | 604 | 654 | 23% | 5 (5 incl. nested) | 3 / 1 / 1 |
| DR | 0 | - | 983 | 1245 | 604 | 285 | 10% | 3 (3 incl. nested) | 3 |
| EXT | 0 | - | 316 | 1834 | 1009 | 201 | 7% | 5 (5 incl. nested) | 5 |
| STG | 0 | - | 742 | 2196 | 472 | 633 | 22% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 167 | 159 | [GH] | 201 |
| 1 | 368 | 1022 | 654 | [PROD] | 223 |
| 2 | 1245 | 1592 | 347 | U2 , U1 , T1 , [DR] | 242 |
| 3 | 1834 | 2035 | 201 | [EXT] | 161 |
| 4 | 2196 | 2829 | 633 | [STG] | - |

Top-level blocks: 5; bands: 5; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 2: U2 -> U1 = 40; band 2: U1 -> T1 = 43; band 2: T1 -> [DR] = 40.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 4 | 1990 | 4 (173, 257, 277, 338) |
| band 1 inside [PROD] | 4 | 531 | 3 (617, 848, 860) |
| gap 1-2 | 8 | 4410 | 5 (1052, 1072, 1092, 1112, 1215) |
| gap 2-3 | 11 | 4440 | 7 (1622, 1704, 1724, 1744, 1764, 1784, 1804) |
| gap 3-4 | 6 | 2872 | 4 (2065, 2126, 2146, 2166) |
| band 4 inside [STG] | 2 | 58 | 2 (2592, 2604) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 1 -> 2 | gap 1-2 275; gap 1-2 78 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 1 -> 4 | gap 1-2 492; gap 3-4 827; band 4 inside [STG] 27 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 1 -> 4 | band 1 inside [PROD] 320; gap 1-2 389; gap 3-4 725; band 4 inside [STG] 30 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 1 -> 3 | band 1 inside [PROD] 92; gap 1-2 887; gap 2-3 313 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 1 -> 3 | band 1 inside [PROD] 97; gap 1-2 864; gap 2-3 471 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 4 -> 3 | gap 3-4 138 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 1 -> 3 | gap 1-2 313; gap 2-3 524 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 2 -> 3 | gap 2-3 198; gap 2-3 502 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 3 -> 1 | gap 2-3 531; gap 1-2 791; band 1 inside [PROD] 23 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 3 -> 2 | gap 2-3 430 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 3 -> 2 | gap 2-3 273; gap 2-3 27 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 3 -> 2 | gap 2-3 598 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 3 -> 4 | gap 3-4 38; gap 3-4 176 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 3 -> 4 | gap 3-4 38; gap 3-4 114 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 0 -> 1 | gap 0-1 83 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 0 -> 2 | gap 0-1 213; gap 1-2 267; gap 1-2 54 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 0 -> 4 | gap 0-1 181; gap 0-1 1315; gap 3-4 817 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 0 -> 1 | gap 0-1 45; gap 0-1 152 |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 2 -> 3 | gap 2-3 369 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 2 -> 3 | gap 2-3 34 |
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 2 -> 3 | gap 2-3 168 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_ARGO_GITLAB_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_DNS_TR2_0 over title of [STG]; L_DNS_TR1_0 over title of [EXT]; L_DNS_SITE_0 over title of [EXT]; L_DNS_PSDEV_0 over title of [EXT]; L_DNS_DRAPP_0 over title of [EXT]; L_U2_DNS_0 over title of [EXT]; L_T1_DNS_0 over title of [EXT]; L_U1_DNS_0 over title of [EXT].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_PG_DRAPP_0 | 200x63 | gap 1-2 | 1 | 239 | 0.65 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 2-3 | 0 | 1571 | 0.46 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 2-3 | 3 | 1559 | 0.48 | - |
| L_LANE_OAI_0 | 46x21 | band 2 beside blocks | 2 | 924 | 0.63 | - |
| L_PSPROD_TG_0 | 148x21 | gap 3-4 | 3 | 125 | 0.78 | - |
| L_ARGO_TG_0 | 86x21 | band 2 beside blocks | 2 | 1000 | 0.53 | - |
| L_DRAPP_TG_0 | 90x21 | gap 2-3 | 3 | 347 | 0.32 | - |
| L_DNS_TR1_0 | 101x21 | band 2 beside blocks | 0 | 989 | 0.38 | - |
| L_DNS_SITE_0 | 127x21 | gap 2-3 | 1 | 155 | 0.81 | - |
| L_DNS_PSDEV_0 | 123x21 | gap 2-3 | 2 | 173 | 0.74 | - |
| L_DNS_DRAPP_0 | 200x42 | gap 2-3 | 2 | 156 | 0.84 | - |
| L_DNS_PSPROD_0 | 104x21 | gap 3-4 | 3 | 114 | 0.19 | - |
| L_DNS_TR2_0 | 93x21 | gap 3-4 | 3 | 119 | 0.29 | - |
| L_R2_PROD_0 | 160x21 | gap 0-1 | 3 | 54 | 0.82 | - |
| L_R2_DR_0 | 141x21 | band 1 beside blocks | 1 | 760 | 0.47 | - |
| L_R2_STG_0 | 75x21 | band 2 beside blocks | 2 | 1583 | 0.64 | - |
| L_R1_ARGO_0 | 128x21 | gap 0-1 | 3 | 202 | 0.40 | - |
| L_U2_DNS_0 | 89x21 | gap 2-3 | 1 | 71 | 0.11 | - |
| L_T1_DNS_0 | 200x42 | gap 2-3 | 0 | 71 | 0.23 | - |
| L_U1_DNS_0 | 199x21 | gap 2-3 | 1 | 72 | 0.16 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=1420: 4 labels; y=1664: 9 labels; y=2096: 3 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 8; overlapping something: 0.

Totals: canvas area 5.77 Mpx2, edge length 30958 px, bends 159, edge crossings (drawn polylines) 7.

