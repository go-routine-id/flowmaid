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

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_DNS_TR1_0 over title of [PROD]; L_LANE_OAI_0 over title of [PROD]; L_LANE_AZ_0 over title of [PROD]; L_DRAPP_PG_0 over title of [PROD]; L_DNS_SITE_0 over title of [DR].

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=648: 5 labels; y=896: 4 labels; y=1148: 5 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 11; overlapping something: 0.

Totals: canvas area 5.37 Mpx2, edge length 29757 px, bends 163, edge crossings (drawn polylines) 13.
