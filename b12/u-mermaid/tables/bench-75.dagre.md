#### bench-75.dagre.svg  (canvas 3063 x 2016, flow axis x)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| edge | 0 | - | 229 | 503 | 404 | 1425 | 13% | 2 (2 incl. nested) | 1 / 1 |
| app | 0 | - | 683 | 504 | 1035 | 1266 | 34% | 6 (6 incl. nested) | 4 / 1 / 1 |
| data | 0 | - | 910 | 48 | 1514 | 436 | 49% | 4 (4 incl. nested) | 1 / 1 / 1 / 1 |
| jobs | 0 | - | 1970 | 809 | 202 | 346 | 7% | 3 (3 incl. nested) | 3 |
| ops | 0 | - | 2222 | 1040 | 631 | 789 | 21% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 179 | 171 | user | 50 |
| 1 | 229 | 633 | 404 | [edge] | 50 |
| 2 | 683 | 2853 | 2170 | [data] , [app] , [jobs] , [ops] | 50 |
| 3 | 2903 | 3055 | 152 | ext | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): band 2.

Cross-axis gaps between neighbours in the same band: band 2: [data] -> [app] = 20; band 2: [app] -> [jobs] = -961; band 2: [jobs] -> [ops] = -115.

Sides of block frames crossed by inter-block edges: enters via bottom: 25, enters via left: 25, enters via right: 2, enters via top: 10, leaves via bottom: 8, leaves via left: 2, leaves via right: 40, leaves via top: 17.

Edges: 75 total, 5 with both ends in the same top-level block (intra) and 70 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| band 0 beside blocks | 1 | 729 | 1 (151) |
| band 1 inside [edge] | 7 | 4210 | 3 (382, 383, 479) |
| gap 1-2 | 2 | 202 | 1 (671) |
| band 2 beside blocks | 9 | 1441 | 1 (2210) |
| band 2 inside [app] | 6 | 3336 | 5 (730, 731, 838, 1161, 1565) |
| band 2 inside [data] | 12 | 10691 | 3 (1731, 2185, 2210) |
| band 2 inside [data]+[app] | 12 | 9863 | 8 (959, 960, 1160, 1362, 1363, 1364, 1563, 1564) |
| band 2 inside [data]+[jobs] | 6 | 4310 | 2 (2019, 2020) |
| band 2 inside [data]+[ops] | 2 | 1837 | 2 (2271, 2272) |
| band 2 inside [jobs] | 4 | 2383 | 1 (2021) |
| band 2 inside [ops] | 5 | 2180 | 5 (2270, 2437, 2496, 2497, 2698) |
| band 3 beside blocks | 6 | 2887 | 2 (2926, 2927) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_user_gw_0 | user (user) | gw (edge) | 0 -> 1 | band 0 beside blocks 729 |
| L_user_lb_0 | user (user) | lb (edge) | 0 -> 1 | band 1 inside [edge] 444 |
| L_gw_svc1_0 | gw (edge) | svc1 (app) | 1 -> 2 | band 1 inside [edge] 618 |
| L_lb_svc1_0 | lb (edge) | svc1 (app) | 1 -> 2 | band 2 inside [app] 485 |
| L_gw_svc2_0 | gw (edge) | svc2 (app) | 1 -> 2 | band 1 inside [edge] 598 |
| L_lb_svc2_0 | lb (edge) | svc2 (app) | 1 -> 2 | band 2 inside [app] 823 |
| L_gw_svc3_0 | gw (edge) | svc3 (app) | 1 -> 2 | band 2 inside [app] 343 |
| L_lb_svc3_0 | lb (edge) | svc3 (app) | 1 -> 2 | straight |
| L_gw_svc4_0 | gw (edge) | svc4 (app) | 1 -> 2 | band 2 inside [app] 531 |
| L_lb_svc4_0 | lb (edge) | svc4 (app) | 1 -> 2 | band 2 inside [app] 796 |
| L_gw_svc5_0 | gw (edge) | svc5 (app) | 1 -> 2 | band 1 inside [edge] 598; gap 1-2 120 |
| L_lb_svc5_0 | lb (edge) | svc5 (app) | 1 -> 2 | straight |
| L_gw_svc6_0 | gw (edge) | svc6 (app) | 1 -> 2 | band 1 inside [edge] 618; gap 1-2 81 |
| L_lb_svc6_0 | lb (edge) | svc6 (app) | 1 -> 2 | straight |
| L_svc1_db_0 | svc1 (app) | db (data) | 2 -> 2 | band 2 inside [data]+[app] 512 |
| L_svc1_cache_0 | svc1 (app) | cache (data) | 2 -> 2 | band 2 inside [data]+[app] 372 |
| L_svc1_queue_0 | svc1 (app) | queue (data) | 2 -> 2 | band 2 inside [data] 482 |
| L_svc2_db_0 | svc2 (app) | db (data) | 2 -> 2 | band 2 inside [data]+[app] 444 |
| L_svc2_cache_0 | svc2 (app) | cache (data) | 2 -> 2 | straight |
| L_svc2_queue_0 | svc2 (app) | queue (data) | 2 -> 2 | band 2 inside [data] 347 |
| L_svc3_db_0 | svc3 (app) | db (data) | 2 -> 2 | band 2 inside [data]+[app] 1062 |
| L_svc3_cache_0 | svc3 (app) | cache (data) | 2 -> 2 | band 2 inside [data]+[app] 873 |
| L_svc3_queue_0 | svc3 (app) | queue (data) | 2 -> 2 | band 2 inside [data] 910 |
| L_svc4_db_0 | svc4 (app) | db (data) | 2 -> 2 | band 2 inside [data]+[app] 491 |
| L_svc4_cache_0 | svc4 (app) | cache (data) | 2 -> 2 | straight |
| L_svc4_queue_0 | svc4 (app) | queue (data) | 2 -> 2 | band 2 inside [data] 376 |
| L_svc5_db_0 | svc5 (app) | db (data) | 2 -> 2 | band 2 inside [data]+[app] 1329 |
| L_svc5_cache_0 | svc5 (app) | cache (data) | 2 -> 2 | band 2 inside [data]+[app] 1139 |
| L_svc5_queue_0 | svc5 (app) | queue (data) | 2 -> 2 | band 2 inside [data] 1099 |
| L_svc6_db_0 | svc6 (app) | db (data) | 2 -> 2 | band 2 inside [data]+[app] 1516 |
| L_svc6_cache_0 | svc6 (app) | cache (data) | 2 -> 2 | band 2 inside [data]+[app] 1326 |
| L_svc6_queue_0 | svc6 (app) | queue (data) | 2 -> 2 | band 2 inside [data] 1246 |
| L_svc1_objects_0 | svc1 (app) | objects (data) | 2 -> 2 | band 2 inside [data] 482 |
| L_svc3_objects_0 | svc3 (app) | objects (data) | 2 -> 2 | band 2 inside [data] 872 |
| L_svc5_objects_0 | svc5 (app) | objects (data) | 2 -> 2 | band 2 inside [data] 1099 |
| L_svc1_logs_0 | svc1 (app) | logs (ops) | 2 -> 2 | band 2 inside [jobs] 564 |
| L_svc2_logs_0 | svc2 (app) | logs (ops) | 2 -> 2 | band 2 inside [data] 567; band 2 inside [data] 1073 |
| L_svc3_logs_0 | svc3 (app) | logs (ops) | 2 -> 2 | straight |
| L_svc4_logs_0 | svc4 (app) | logs (ops) | 2 -> 2 | band 2 inside [data] 604; band 2 inside [data] 1073 |
| L_svc5_logs_0 | svc5 (app) | logs (ops) | 2 -> 2 | straight |
| L_svc6_logs_0 | svc6 (app) | logs (ops) | 2 -> 2 | band 2 beside blocks 110; band 2 inside [ops] 304 |
| L_svc1_metrics_0 | svc1 (app) | metrics (ops) | 2 -> 2 | band 2 inside [jobs] 564 |
| L_svc2_metrics_0 | svc2 (app) | metrics (ops) | 2 -> 2 | band 2 inside [jobs] 623 |
| L_svc3_metrics_0 | svc3 (app) | metrics (ops) | 2 -> 2 | straight |
| L_svc4_metrics_0 | svc4 (app) | metrics (ops) | 2 -> 2 | band 2 inside [jobs] 632 |
| L_queue_job1_0 | queue (data) | job1 (jobs) | 2 -> 2 | band 2 inside [data]+[jobs] 785 |
| L_job1_db_0 | job1 (jobs) | db (data) | 2 -> 2 | band 2 inside [data]+[jobs] 933 |
| L_job1_ext_0 | job1 (jobs) | ext (ext) | 2 -> 3 | band 2 beside blocks 81; band 3 beside blocks 412 |
| L_queue_job2_0 | queue (data) | job2 (jobs) | 2 -> 2 | band 2 inside [data]+[jobs] 538 |
| L_job2_db_0 | job2 (jobs) | db (data) | 2 -> 2 | band 2 inside [data]+[jobs] 685 |
| L_job2_ext_0 | job2 (jobs) | ext (ext) | 2 -> 3 | band 3 beside blocks 572 |
| L_queue_job3_0 | queue (data) | job3 (jobs) | 2 -> 2 | band 2 inside [data]+[jobs] 612 |
| L_job3_db_0 | job3 (jobs) | db (data) | 2 -> 2 | band 2 inside [data]+[jobs] 759 |
| L_job3_ext_0 | job3 (jobs) | ext (ext) | 2 -> 3 | band 3 beside blocks 506 |
| L_alerts_gw_0 | alerts (ops) | gw (edge) | 2 -> 1 | band 2 beside blocks 150; band 1 inside [edge] 677 |
| L_dash_user_0 | dash (ops) | user (user) | 2 -> 0 | band 2 inside [ops] 337; band 2 beside blocks 179 |
| L_job1_svc1_0 | job1 (jobs) | svc1 (app) | 2 -> 2 | band 2 inside [app] 358 |
| L_db_svc2_0 | db (data) | svc2 (app) | 2 -> 2 | band 2 inside [data]+[app] 484 |
| L_svc6_ext_0 | svc6 (app) | ext (ext) | 2 -> 3 | band 2 beside blocks 179; band 3 beside blocks 453 |
| L_svc5_ext_0 | svc5 (app) | ext (ext) | 2 -> 3 | band 2 beside blocks 219; band 3 beside blocks 394 |
| L_cache_svc4_0 | cache (data) | svc4 (app) | 2 -> 2 | band 2 inside [data]+[app] 316 |
| L_objects_logs_0 | objects (data) | logs (ops) | 2 -> 2 | band 2 inside [ops] 704 |
| L_db_metrics_0 | db (data) | metrics (ops) | 2 -> 2 | band 2 inside [data]+[ops] 1052 |
| L_queue_metrics_0 | queue (data) | metrics (ops) | 2 -> 2 | band 2 inside [data]+[ops] 785 |
| L_gw_logs_0 | gw (edge) | logs (ops) | 1 -> 2 | band 1 inside [edge] 657; band 2 beside blocks 187; band 2 inside [ops] 361 |
| L_lb_metrics_0 | lb (edge) | metrics (ops) | 1 -> 2 | band 2 beside blocks 148; band 2 inside [ops] 474 |
| L_user_ext_0 | user (user) | ext (ext) | 0 -> 3 | band 3 beside blocks 550 |
| L_job2_objects_0 | job2 (jobs) | objects (data) | 2 -> 2 | band 2 inside [data] 461 |
| L_job3_logs_0 | job3 (jobs) | logs (ops) | 2 -> 2 | band 2 beside blocks 187 |
| L_dash_ext_0 | dash (ops) | ext (ext) | 2 -> 3 | straight |

Edges drawn through a block they do not belong to (7 cases): L_svc1_logs_0 through [jobs] for 342 px; L_svc2_logs_0 through [data] for 874 px; L_svc4_logs_0 through [data] for 869 px; L_svc1_metrics_0 through [jobs] for 324 px; L_svc2_metrics_0 through [jobs] for 353 px; L_svc4_metrics_0 through [jobs] for 347 px; L_svc5_ext_0 through [ops] for 42 px.

Edges drawn over a node that is not their end (17 cases): L_svc1_logs_0 over job1 for 42 px; L_svc1_logs_0 over job3 for 42 px; L_svc1_metrics_0 over job1 for 42 px; L_svc1_metrics_0 over job3 for 42 px; L_svc2_metrics_0 over job1 for 42 px; L_svc2_metrics_0 over job2 for 24 px; L_svc2_metrics_0 over job3 for 48 px; L_svc4_metrics_0 over job1 for 42 px; L_svc4_metrics_0 over job2 for 42 px; L_svc4_metrics_0 over job3 for 42 px; L_metrics_alerts_0 over logs for 12 px; L_queue_job1_0 over job2 for 42 px; L_queue_job1_0 over job3 for 42 px; L_job1_db_0 over job2 for 42 px; L_job1_db_0 over job3 for 42 px; L_queue_job3_0 over job2 for 42 px; L_job3_db_0 over job2 for 42 px.

Edges drawn over a block title: L_job1_db_0 over title of [jobs]; L_queue_job2_0 over title of [jobs]; L_job2_db_0 over title of [jobs]; L_queue_job3_0 over title of [jobs]; L_job3_db_0 over title of [jobs]; L_svc2_db_0 over title of [app]; L_db_svc2_0 over title of [app].

Totals: canvas area 6.18 Mpx2, edge length 98890 px, bends 724, edge crossings (drawn polylines) 303.

