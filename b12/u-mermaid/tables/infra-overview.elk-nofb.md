#### infra-overview.elk-nofb.svg  (canvas 2607 x 2438, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| GH | 0 | - | 721 | 8 | 402 | 159 | 7% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 721 | 348 | 604 | 654 | 27% | 5 (5 incl. nested) | 3 / 1 / 1 |
| EXT | 0 | - | 957 | 1304 | 1009 | 201 | 8% | 5 (5 incl. nested) | 5 |
| STG | 0 | - | 83 | 1747 | 472 | 633 | 26% | 4 (4 incl. nested) | 1 / 2 / 1 |
| DR | 0 | - | 1325 | 1747 | 604 | 285 | 12% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 167 | 159 | [GH] | 181 |
| 1 | 348 | 1002 | 654 | [PROD] , T1 , U2 , U1 | 302 |
| 2 | 1304 | 1505 | 201 | [EXT] | 242 |
| 3 | 1747 | 2380 | 633 | [STG] , [DR] | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 1: [PROD] -> T1 = 219; band 1: T1 -> U2 = 40; band 1: U2 -> U1 = 40; band 3: [STG] -> [DR] = 769.

Sides of block frames crossed by inter-block edges: enters via top: 21, leaves via bottom: 17, leaves via right: 1.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| after last band | 2 | 3014 | 2 (2410, 2430) |
| gap 0-1 | 3 | 2820 | 3 (173, 257, 318) |
| band 1 inside [PROD] | 4 | 638 | 3 (597, 828, 840) |
| gap 1-2 | 12 | 6041 | 10 (1032, 1052, 1072, 1092, 1112, 1194, 1214, 1234, 1254, 1274) |
| gap 2-3 | 11 | 6838 | 7 (1535, 1555, 1575, 1657, 1677, 1697, 1717) |
| band 3 inside [STG] | 2 | 59 | 2 (2143, 2155) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 1 -> 2 | gap 1-2 133; gap 1-2 235 |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 1 -> 2 | gap 1-2 133; gap 1-2 133 |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 1 -> 2 | gap 1-2 133; gap 1-2 51 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 0 -> 1 | straight |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 0 -> 3 | gap 0-1 181; gap 0-1 763; gap 2-3 78 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 0 -> 3 | gap 0-1 1251; gap 2-3 466 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 0 -> 1 | straight |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 2 -> 3 | gap 2-3 1339; gap 2-3 27 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 2 -> 3 | gap 2-3 1198; gap 2-3 15 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 2 -> 3 | gap 2-3 131; gap 2-3 358 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 2 -> 3 | gap 2-3 422; gap 2-3 304 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 2 -> 3 | gap 2-3 296; gap 2-3 362 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 2 -> 1 | gap 2-3 917; gap 1-2 100; gap 0-1 624 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 3 -> 2 | after last band 962; gap 1-2 487 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 1 -> 2 | band 1 inside [PROD] 485; gap 1-2 767; gap 1-2 261 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 3 -> 2 | after last band 2052; gap 1-2 658 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 1 -> 2 | band 1 inside [PROD] 31; gap 1-2 182; gap 1-2 174 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 1 -> 2 | band 1 inside [PROD] 28; gap 1-2 133; gap 1-2 303 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 1 -> 3 | band 1 inside [PROD] 94; gap 1-2 862; gap 2-3 70; band 3 inside [STG] 31 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 1 -> 3 | gap 1-2 696; gap 2-3 33; band 3 inside [STG] 28 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 1 -> 3 | gap 1-2 600; gap 2-3 822 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_DNS_SITE_0 over title of [DR]; L_LANE_AZ_0 over title of [EXT].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 1-2 | 2 | 280 | 0.40 | - |
| L_T1_DNS_0 | 200x42 | gap 1-2 | 1 | 280 | 0.47 | - |
| L_U2_DNS_0 | 89x21 | gap 1-2 | 2 | 239 | 0.54 | - |
| L_R1_ARGO_0 | 128x21 | gap 0-1 | 2 | 143 | 0.49 | - |
| L_R2_STG_0 | 75x21 | gap 1-2 | 2 | 665 | 0.74 | - |
| L_R2_DR_0 | 141x21 | gap 1-2 | 3 | 1048 | 0.68 | - |
| L_R2_PROD_0 | 160x21 | gap 0-1 | 3 | 59 | 0.71 | - |
| L_DNS_TR2_0 | 93x21 | gap 2-3 | 1 | 240 | 0.86 | - |
| L_DNS_PSPROD_0 | 104x21 | gap 2-3 | 0 | 353 | 0.79 | - |
| L_DNS_DRAPP_0 | 200x42 | gap 2-3 | 2 | 263 | 0.32 | - |
| L_DNS_PSDEV_0 | 123x21 | gap 2-3 | 0 | 516 | 0.52 | - |
| L_DNS_SITE_0 | 127x21 | gap 2-3 | 1 | 425 | 0.43 | - |
| L_DNS_TR1_0 | 101x21 | gap 1-2 | 1 | 1348 | 0.44 | - |
| L_DRAPP_TG_0 | 90x21 | gap 2-3 | 1 | 905 | 0.70 | - |
| L_ARGO_TG_0 | 86x21 | gap 1-2 | 1 | 449 | 0.81 | - |
| L_PSPROD_TG_0 | 148x21 | gap 2-3 | 1 | 1116 | 0.74 | - |
| L_LANE_OAI_0 | 46x21 | gap 1-2 | 1 | 365 | 0.60 | - |
| L_LANE_GITLAB_0 | 149x21 | band 2 beside blocks | 2 | 868 | 0.64 | - |
| L_ARGO_GITLAB_0 | 64x21 | band 2 beside blocks | 1 | 827 | 0.65 | - |
| L_PG_DRAPP_0 | 200x63 | band 2 beside blocks | 2 | 1026 | 0.45 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=1152: 8 labels; y=1404: 3 labels; y=1616: 7 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 13; overlapping something: 0.

Totals: canvas area 6.36 Mpx2, edge length 37027 px, bends 157, edge crossings (drawn polylines) 14.

