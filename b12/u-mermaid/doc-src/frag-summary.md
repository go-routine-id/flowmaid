| input | layout | canvas Mpx2 | bands | staggered bands | largest band gap | most lanes in one gap | edges through foreign blocks | edges over foreign nodes | lines over block titles | labels: total / off line / far from both ends / overlapping | edge length px | crossings |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infra-overview | ELK (default) | 5.18 | 4 | none | 301 | 11 | 0 | 0 | 7 | 20 / 0 / 14 / 0 | 30945 | 16 |
| infra-overview | dagre | 5.42 | 2 | band 1 | n/a | n/a (curves) | 6 | 10 | 2 | 20 / 8 / 6 / 1 | 25429 | 23 |
| infra-overview-v2 | ELK (default) | 5.37 | 5 | none | 305 | 7 | 0 | 0 | 10 | 20 / 0 / 11 / 0 | 29757 | 13 |
| advanced | ELK (default) | 1.34 | 4 | none | 101 | 2 | 0 | 0 | 1 | 4 / 0 / 0 / 0 | 5108 | 0 |
| advanced | dagre | 1.94 | 2 | band 0 | n/a | n/a (curves) | 0 | 0 | 0 | 4 / 1 / 0 / 0 | 3535 | 2 |
| subgraph | ELK (default) | 0.35 | 4 | none | 60 | 1 | 0 | 0 | 2 | - | 843 | 0 |
| subgraph | dagre | 0.35 | 4 | none | n/a | n/a (curves) | 0 | 0 | 2 | - | 634 | 0 |
| fanout-frames | ELK (default) | 1.47 | 2 | none | 80 | 3 | 0 | 0 | 0 | - | 4253 | 0 |
| fanout-frames | dagre | 2.19 | 2 | none | n/a | n/a (curves) | 0 | 0 | 0 | - | 4455 | 0 |
| state | ELK (default) | 0.57 | 9 | none | 101 | 2 | 0 | 0 | 0 | 5 / 0 / 0 / 0 | 2138 | 0 |
| state | dagre | 0.70 | 9 | none | n/a | n/a (curves) | 0 | 0 | 0 | 5 / 0 / 0 / 0 | 1868 | 0 |
| bench-75 | ELK (default) | 4.58 | 7 | none | 724 | 35 | 0 | 0 | 0 | - | 107188 | 222 |
| bench-75 | dagre | 6.18 | 4 | band 2 | n/a | n/a (curves) | 7 | 17 | 7 | - | 98890 | 303 |
| dense-td | ELK (default) | 18.09 | 5 | none | 431 | 17 | 0 | 0 | 8 | 21 / 0 / 14 / 0 | 142591 | 68 |
| dense-td | dagre | 11.52 | 1 | band 0 | n/a | n/a (curves) | 20 | 23 | 0 | 21 / 8 / 10 / 9 | 105077 | 52 |
@@SPLIT@@
| input | mermaid ELK | mermaid dagre | flowmaid 0.38.0 |
|---|---|---|---|
| infra-overview | 5.18 | 5.42 | 3.41 |
| infra-overview-v2 | 5.37 | - | - |
| advanced | 1.34 | 1.94 | 0.74 |
| subgraph | 0.35 | 0.35 | 0.29 |
| fanout-frames | 1.47 | 2.19 | 1.31 |
| state | 0.57 | 0.70 | 0.47 |
| bench-75 | 4.58 | 6.18 | 4.37 |
| dense-td | 18.09 | 11.52 | 7.67 |
@@SPLIT@@
| input | block | frame w x h | padding left / top / right / bottom | title w x h |
|---|---|---|---|---|
| infra-overview | EXT | 1009 x 201 | 24 / 48 / 24 / 24 | 107 x 21 |
| infra-overview | GH | 402 x 159 | 24 / 48 / 24 / 24 | 82 x 21 |
| infra-overview | PROD | 579 x 726 | 24 / 162 / 24 / 24 | 284 x 63 |
| infra-overview | DR | 604 x 285 | 24 / 90 / 24 / 24 | 337 x 63 |
| infra-overview | STG | 472 x 633 | 29 / 90 / 42 / 24 | 464 x 63 |
| infra-overview-v2 | EXT | 1009 x 201 | 24 / 48 / 24 / 24 | 107 x 21 |
| infra-overview-v2 | GH | 402 x 159 | 24 / 48 / 24 / 24 | 82 x 21 |
| infra-overview-v2 | DR | 604 x 285 | 24 / 90 / 24 / 24 | 337 x 63 |
| infra-overview-v2 | PROD | 575 x 642 | 24 / 90 / 24 / 24 | 284 x 63 |
| infra-overview-v2 | STG | 472 x 633 | 29 / 90 / 42 / 24 | 464 x 63 |
| advanced | CI | 449 x 452 | 24 / 48 / 24 / 24 | 92 x 21 |
| advanced | Platform | 870 x 634 | 24 / 48 / 24 / 24 | 123 x 21 |
| advanced | Workers | 414 x 262 | 24 / 48 / 24 / 24 | 94 x 21 |
| subgraph | platform | 450 x 442 | 24 / 48 / 24 / 24 | 52 x 21 |
| subgraph | events | 402 x 222 | 24 / 48 / 24 / 24 | 62 x 21 |
| fanout-frames | g1 | 400 x 547 | 24 / 48 / 24 / 24 | 82 x 21 |
| fanout-frames | g2 | 248 x 318 | 24 / 48 / 24 / 24 | 76 x 21 |
| fanout-frames | g3 | 376 x 400 | 24 / 48 / 24 / 24 | 214 x 21 |
| fanout-frames | g4 | 200 x 270 | 24 / 48 / 24 / 24 | 146 x 21 |
| fanout-frames | g5 | 224 x 138 | 24 / 48 / 24 / 24 | 121 x 21 |
| fanout-frames | g6 | 200 x 297 | 24 / 48 / 24 / 24 | 18 x 21 |
| state | Validating | 459 x 109 | 24 / 48 / 24 / 24 | 60 x 21 |
| bench-75 | app | 200 x 624 | 24 / 80 / 24 / 24 | 58 x 21 |
| bench-75 | data | 200 x 488 | 24 / 133 / 24 / 24 | 62 x 21 |
| bench-75 | jobs | 200 x 308 | 24 / 50 / 24 / 24 | 60 x 21 |
| bench-75 | ops | 696 x 281 | 24 / 48 / 24 / 24 | 57 x 21 |
| bench-75 | edge | 448 x 302 | 24 / 122 / 24 / 24 | 65 x 21 |
| dense-td | g1 | 2879 x 1349 | 24 / 58 / 24 / 24 | 132 x 21 |
| dense-td | g5 | 2236 x 1104 | 24 / 154 / 24 / 24 | 62 x 21 |
| dense-td | g8 | 1488 x 965 | 24 / 211 / 24 / 24 | 92 x 42 |
| dense-td | g4 | 1158 x 556 | 24 / 48 / 24 / 24 | 72 x 21 |
| dense-td | g2 | 1310 x 403 | 24 / 48 / 24 / 24 | 135 x 21 |
| dense-td | g6 | 1569 x 926 | 24 / 163 / 24 / 24 | 39 x 42 |
| dense-td | g9 | 632 x 583 | 24 / 130 / 24 / 24 | 119 x 21 |
| dense-td | g3 | 622 x 331 | 24 / 48 / 24 / 24 | 63 x 21 |
| dense-td | g7 | 620 x 469 | 24 / 129 / 24 / 24 | 108 x 42 |
| dense-td | g10 | 262 x 303 | 24 / 129 / 24 / 24 | 162 x 42 |
