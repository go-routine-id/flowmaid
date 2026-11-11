| diagram | type | flowmaid ms | mermaid ms | speed-up × | flowmaid SVG KB | mermaid SVG KB |
|---|---|--:|--:|--:|--:|--:|
| advanced | flowchart | 2.03 | 152.9 | 75× | 11.5 | 57.7 |
| architecture | architecture | 0.14 | 46.3 | 324× | 4.0 | 18.5 |
| back-edge | flowchart | 0.43 | 57.1 | 132× | 3.7 | 78.5 |
| bench-75 | flowchart | 49.49 | 358.9 | 7× | 59.0 | 194.6 |
| class | class | 0.48 | 179.7 | 378× | 11.8 | 53.5 |
| demo | flowchart | 0.29 | 63.1 | 216× | 3.7 | 78.5 |
| dense-lr | flowchart | 22.80 | 469.4 | 21× | 45.4 | 489.1 |
| dense-td | flowchart | 29.08 | 527.4 | 18× | 48.6 | 487.0 |
| er | er | 0.27 | 120.3 | 452× | 14.8 | 127.4 |
| fanout-frames | flowchart | 2.61 | 139.8 | 54× | 11.8 | 298.2 |
| fanout | flowchart | 0.79 | 69.7 | 89× | 6.6 | 84.7 |
| gitgraph | gitgraph | 0.07 | 22.1 | 320× | 5.8 | 13.0 |
| journey | journey | 0.04 | 9.3 | 251× | 3.8 | 13.2 |
| long-labels | flowchart | 0.20 | 57.4 | 286× | 3.5 | 34.7 |
| lr | flowchart | 0.34 | 76.1 | 221× | 4.5 | 80.4 |
| mindmap | mindmap | 0.69 | 112.6 | 163× | 8.7 | 49.0 |
| pie | pie | 0.04 | 7.5 | 208× | 2.8 | 5.5 |
| sankey | sankey | 0.20 | 18.7 | 93× | 6.8 | 10.3 |
| sequence | sequence | 0.13 | 47.8 | 371× | 10.3 | 39.0 |
| shapes | flowchart | 0.27 | 64.5 | 236× | 5.0 | 81.9 |
| state | state | 1.24 | 159.5 | 129× | 8.5 | 70.4 |
| stress | flowchart | 0.44 | 54.7 | 124× | 4.0 | 37.7 |
| subgraph | flowchart | 0.40 | 63.1 | 159× | 4.6 | 38.5 |
| timeline | timeline | 0.04 | 11.2 | 273× | 3.7 | 15.9 |
| wide-labels | flowchart | 0.85 | 82.5 | 97× | 6.5 | 42.1 |

**Method.** Each cell is the median of 30 timed renders after warm-ups (flowmaid 3, mermaid 2) of the same
`.mmd` source. flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build
(`cargo run --release --example bench -- files`). mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string,
including its in-browser text measurement) in headless Chromium 153.0.8010.12 via Playwright. Speed-up = mermaid median
÷ flowmaid median; geometric mean over 25 diagrams: **134×**. SVG KB = length of the returned SVG string (all-ASCII here, so chars = bytes).
Machine: 4 CPUs, Intel(R) Xeon(R) Processor @ 2.10GHz; rustc 1.97.0 (2d8144b78 2026-07-07). One render at a time on both sides; timings on a shared box are noisy (±10–20 %).
