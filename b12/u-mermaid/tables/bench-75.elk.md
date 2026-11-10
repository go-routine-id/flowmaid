#### bench-75.elk.svg  (canvas 4327 x 1058, flow axis x)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| edge | 0 | - | 249 | 469 | 448 | 302 | 10% | 2 (2 incl. nested) | 1 / 1 |
| app | 0 | - | 1257 | 137 | 200 | 624 | 5% | 6 (6 incl. nested) | 6 |
| data | 0 | - | 2181 | 175 | 200 | 488 | 5% | 4 (4 incl. nested) | 4 |
| jobs | 0 | - | 2737 | 262 | 200 | 308 | 5% | 3 (3 incl. nested) | 3 |
| ops | 0 | - | 3361 | 396 | 696 | 281 | 16% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 179 | 171 | user | 70 |
| 1 | 249 | 697 | 448 | [edge] | 560 |
| 2 | 1257 | 1457 | 200 | [app] | 724 |
| 3 | 2181 | 2381 | 200 | [data] | 356 |
| 4 | 2737 | 2937 | 200 | [jobs] | 424 |
| 5 | 3361 | 4057 | 696 | [ops] | 110 |
| 6 | 4167 | 4319 | 152 | ext | - |

Top-level blocks: 5; bands: 7; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via bottom: 2, enters via left: 52, enters via right: 8, leaves via bottom: 1, leaves via left: 8, leaves via right: 58.

Edges: 75 total, 5 with both ends in the same top-level block (intra) and 70 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 4 | 717 | 2 (199, 219) |
| band 1 inside [edge] | 7 | 298 | 4 (455, 467, 479, 491) |
| gap 1-2 | 15 | 5100 | 24 (703, 715, 727, 823, 843, 863, 883, 903, 923, 943, 963, 983, 1003, 1023 ...) |
| gap 2-3 | 35 | 7333 | 35 (1463, 1475, 1487, 1595, 1615, 1635, 1655, 1675, 1695, 1715, 1735, 1755, 1775, 1795 ...) |
| gap 3-4 | 9 | 1495 | 10 (2483, 2503, 2543, 2563, 2583, 2603, 2623, 2707, 2719, 2731) |
| gap 4-5 | 20 | 5258 | 15 (3027, 3047, 3067, 3087, 3107, 3127, 3147, 3167, 3187, 3207, 3227, 3247, 3331, 3343 ...) |
| band 5 inside [ops] | 9 | 735 | 6 (3567, 3579, 3591, 3603, 3627, 3851) |
| gap 5-6 | 7 | 2322 | 4 (4087, 4107, 4127, 4147) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_user_gw_0 | user (user) | gw (edge) | 0 -> 1 | gap 0-1 283 |
| L_user_lb_0 | user (user) | lb (edge) | 0 -> 1 | gap 0-1 173 |
| L_gw_svc1_0 | gw (edge) | svc1 (app) | 1 -> 2 | band 1 inside [edge] 56; gap 1-2 354; gap 1-2 38 |
| L_lb_svc1_0 | lb (edge) | svc1 (app) | 1 -> 2 | gap 1-2 28; gap 1-2 212; gap 1-2 234 |
| L_gw_svc2_0 | gw (edge) | svc2 (app) | 1 -> 2 | band 1 inside [edge] 37; gap 1-2 321; gap 1-2 80 |
| L_lb_svc2_0 | lb (edge) | svc2 (app) | 1 -> 2 | gap 1-2 18; gap 1-2 171; gap 1-2 192 |
| L_gw_svc3_0 | gw (edge) | svc3 (app) | 1 -> 2 | band 1 inside [edge] 19; gap 1-2 289; gap 1-2 121 |
| L_lb_svc3_0 | lb (edge) | svc3 (app) | 1 -> 2 | gap 1-2 9; gap 1-2 129; gap 1-2 151 |
| L_gw_svc4_0 | gw (edge) | svc4 (app) | 1 -> 2 | gap 1-2 256; gap 1-2 163 |
| L_lb_svc4_0 | lb (edge) | svc4 (app) | 1 -> 2 | gap 1-2 88; gap 1-2 109 |
| L_gw_svc5_0 | gw (edge) | svc5 (app) | 1 -> 2 | band 1 inside [edge] 19; gap 1-2 224; gap 1-2 204 |
| L_lb_svc5_0 | lb (edge) | svc5 (app) | 1 -> 2 | gap 1-2 9; gap 1-2 46; gap 1-2 68 |
| L_gw_svc6_0 | gw (edge) | svc6 (app) | 1 -> 2 | band 1 inside [edge] 37; gap 1-2 191; gap 1-2 246 |
| L_lb_svc6_0 | lb (edge) | svc6 (app) | 1 -> 2 | gap 1-2 18; gap 1-2 26 |
| L_svc1_db_0 | svc1 (app) | db (data) | 2 -> 3 | gap 2-3 15; gap 2-3 15; gap 2-3 87 |
| L_svc1_cache_0 | svc1 (app) | cache (data) | 2 -> 3 | gap 2-3 15; gap 2-3 348 |
| L_svc1_queue_0 | svc1 (app) | queue (data) | 2 -> 3 | gap 2-3 15; gap 2-3 266 |
| L_svc2_db_0 | svc2 (app) | db (data) | 2 -> 3 | gap 2-3 28; gap 2-3 14; gap 2-3 42 |
| L_svc2_cache_0 | svc2 (app) | cache (data) | 2 -> 3 | gap 2-3 14; gap 2-3 291 |
| L_svc2_queue_0 | svc2 (app) | queue (data) | 2 -> 3 | gap 2-3 10; gap 2-3 14; gap 2-3 209 |
| L_svc3_db_0 | svc3 (app) | db (data) | 2 -> 3 | gap 2-3 28; gap 2-3 61 |
| L_svc3_cache_0 | svc3 (app) | cache (data) | 2 -> 3 | gap 2-3 10; gap 2-3 199 |
| L_svc3_queue_0 | svc3 (app) | queue (data) | 2 -> 3 | gap 2-3 19; gap 2-3 117 |
| L_svc4_db_0 | svc4 (app) | db (data) | 2 -> 3 | gap 2-3 28; gap 2-3 49; gap 2-3 104 |
| L_svc4_cache_0 | svc4 (app) | cache (data) | 2 -> 3 | gap 2-3 10; gap 2-3 105 |
| L_svc4_queue_0 | svc4 (app) | queue (data) | 2 -> 3 | gap 2-3 19; gap 2-3 23 |
| L_svc5_db_0 | svc5 (app) | db (data) | 2 -> 3 | gap 2-3 28; gap 2-3 58; gap 2-3 303 |
| L_svc5_cache_0 | svc5 (app) | cache (data) | 2 -> 3 | gap 2-3 10; gap 2-3 58; gap 2-3 42 |
| L_svc5_queue_0 | svc5 (app) | queue (data) | 2 -> 3 | gap 2-3 19; gap 2-3 58; gap 2-3 126 |
| L_svc6_db_0 | svc6 (app) | db (data) | 2 -> 3 | gap 2-3 22; gap 2-3 348 |
| L_svc6_cache_0 | svc6 (app) | cache (data) | 2 -> 3 | gap 2-3 87 |
| L_svc6_queue_0 | svc6 (app) | queue (data) | 2 -> 3 | gap 2-3 14; gap 2-3 171 |
| L_svc1_objects_0 | svc1 (app) | objects (data) | 2 -> 3 | gap 2-3 12; gap 2-3 15; gap 2-3 149 |
| L_svc3_objects_0 | svc3 (app) | objects (data) | 2 -> 3 | straight |
| L_svc5_objects_0 | svc5 (app) | objects (data) | 2 -> 3 | gap 2-3 58; gap 2-3 244 |
| L_svc1_logs_0 | svc1 (app) | logs (ops) | 2 -> 5 | gap 2-3 21; gap 2-3 156; gap 4-5 374; band 5 inside [ops] 134 |
| L_svc2_logs_0 | svc2 (app) | logs (ops) | 2 -> 5 | gap 2-3 16; gap 2-3 203; gap 4-5 356; band 5 inside [ops] 113 |
| L_svc3_logs_0 | svc3 (app) | logs (ops) | 2 -> 5 | gap 2-3 16; gap 2-3 337; gap 4-5 159 |
| L_svc4_logs_0 | svc4 (app) | logs (ops) | 2 -> 5 | gap 2-3 16; gap 2-3 284; gap 4-5 177; band 5 inside [ops] 22 |
| L_svc5_logs_0 | svc5 (app) | logs (ops) | 2 -> 5 | gap 2-3 222; gap 4-5 177; band 5 inside [ops] 40 |
| L_svc6_logs_0 | svc6 (app) | logs (ops) | 2 -> 5 | gap 2-3 225; gap 4-5 248; band 5 inside [ops] 59 |
| L_svc1_metrics_0 | svc1 (app) | metrics (ops) | 2 -> 5 | gap 2-3 33; gap 2-3 138; gap 4-5 416; gap 4-5 108 |
| L_svc2_metrics_0 | svc2 (app) | metrics (ops) | 2 -> 5 | gap 2-3 212; gap 4-5 366; gap 4-5 87 |
| L_svc3_metrics_0 | svc3 (app) | metrics (ops) | 2 -> 5 | gap 2-3 328; gap 4-5 165; gap 4-5 9 |
| L_svc4_metrics_0 | svc4 (app) | metrics (ops) | 2 -> 5 | gap 2-3 275; gap 4-5 168; gap 4-5 46 |
| L_queue_job1_0 | queue (data) | job1 (jobs) | 3 -> 4 | gap 3-4 74; gap 3-4 101 |
| L_job1_db_0 | job1 (jobs) | db (data) | 4 -> 3 | gap 3-4 9 |
| L_job1_ext_0 | job1 (jobs) | ext (ext) | 4 -> 6 | gap 4-5 136; gap 5-6 482 |
| L_queue_job2_0 | queue (data) | job2 (jobs) | 3 -> 4 | gap 3-4 63; gap 3-4 18 |
| L_job2_db_0 | job2 (jobs) | db (data) | 4 -> 3 | gap 3-4 18; gap 3-4 79 |
| L_job2_ext_0 | job2 (jobs) | ext (ext) | 4 -> 6 | gap 4-5 210; gap 5-6 464 |
| L_queue_job3_0 | queue (data) | job3 (jobs) | 3 -> 4 | straight |
| L_job3_db_0 | job3 (jobs) | db (data) | 4 -> 3 | gap 3-4 103; gap 3-4 79 |
| L_job3_ext_0 | job3 (jobs) | ext (ext) | 4 -> 6 | gap 4-5 280; gap 5-6 446 |
| L_alerts_gw_0 | alerts (ops) | gw (edge) | 5 -> 1 | gap 4-5 455; gap 1-2 521; band 1 inside [edge] 75 |
| L_dash_user_0 | dash (ops) | user (user) | 5 -> 0 | band 5 inside [ops] 122; gap 4-5 287; gap 0-1 122 |
| L_job1_svc1_0 | job1 (jobs) | svc1 (app) | 4 -> 2 | gap 3-4 242; gap 2-3 129; gap 2-3 24 |
| L_db_svc2_0 | db (data) | svc2 (app) | 3 -> 2 | gap 2-3 33; gap 2-3 14; gap 2-3 19 |
| L_svc6_ext_0 | svc6 (app) | ext (ext) | 2 -> 6 | gap 2-3 244; gap 5-6 271 |
| L_svc5_ext_0 | svc5 (app) | ext (ext) | 2 -> 6 | gap 2-3 16; gap 2-3 281; gap 5-6 232 |
| L_cache_svc4_0 | cache (data) | svc4 (app) | 3 -> 2 | gap 2-3 9; gap 2-3 105 |
| L_objects_logs_0 | objects (data) | logs (ops) | 3 -> 5 | gap 3-4 238; gap 4-5 113; band 5 inside [ops] 72 |
| L_db_metrics_0 | db (data) | metrics (ops) | 3 -> 5 | gap 3-4 309; gap 4-5 104; gap 4-5 44 |
| L_queue_metrics_0 | queue (data) | metrics (ops) | 3 -> 5 | gap 3-4 161; gap 4-5 98 |
| L_gw_logs_0 | gw (edge) | logs (ops) | 1 -> 5 | band 1 inside [edge] 56; gap 1-2 310; gap 4-5 269; band 5 inside [ops] 78 |
| L_lb_metrics_0 | lb (edge) | metrics (ops) | 1 -> 5 | gap 1-2 27; gap 1-2 248; gap 4-5 278; gap 4-5 130 |
| L_user_ext_0 | user (user) | ext (ext) | 0 -> 6 | gap 0-1 138; gap 5-6 352 |
| L_job2_objects_0 | job2 (jobs) | objects (data) | 4 -> 3 | straight |
| L_job3_logs_0 | job3 (jobs) | logs (ops) | 4 -> 5 | band 5 inside [ops] 94 |
| L_dash_ext_0 | dash (ops) | ext (ext) | 5 -> 6 | gap 5-6 76 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 4.58 Mpx2, edge length 107188 px, bends 589, edge crossings (drawn polylines) 222.

