| diagram | type | flowmaid ms | mermaid ms | speed-up × | flowmaid SVG KB | mermaid SVG KB |
|---|---|--:|--:|--:|--:|--:|
| advanced | flowchart | 0.99 | 117.2 | 119× | 9.6 | 58.0 |
| architecture | architecture | 0.10 | 47.1 | 457× | 4.0 | 18.4 |
| back-edge | flowchart | 0.20 | 58.5 | 298× | 3.1 | 78.5 |
| bench-75 | flowchart | 39.26 | 354.7 | 9× | 68.5 | 194.6 |
| class | class | 0.91 | 180.8 | 200× | 11.8 | 53.5 |
| demo | flowchart | 0.32 | 53.5 | 168× | 3.1 | 78.5 |
| dense-lr | flowchart | 25.54 | 408.8 | 16× | 51.9 | 489.0 |
| dense-td | flowchart | 30.13 | 429.0 | 14× | 54.3 | 487.2 |
| er | er | 0.44 | 128.5 | 293× | 14.8 | 127.3 |
| fanout-frames | flowchart | 2.88 | 136.0 | 47× | 12.3 | 298.3 |
| fanout | flowchart | 0.81 | 71.7 | 89× | 6.1 | 84.7 |
| gitgraph | gitgraph | 0.08 | 36.5 | 468× | 5.8 | 12.9 |
| journey | journey | 0.04 | 9.4 | 229× | 3.8 | 13.2 |
| long-labels | flowchart | 0.13 | 45.1 | 342× | 2.8 | 35.0 |
| lr | flowchart | 0.38 | 56.9 | 149× | 3.6 | 80.4 |
| mindmap | mindmap | 0.80 | 93.5 | 117× | 8.7 | 49.0 |
| pie | pie | 0.04 | 5.7 | 154× | 2.8 | 5.5 |
| sankey | sankey | 0.24 | 12.8 | 54× | 6.8 | 10.3 |
| sequence | sequence | 0.16 | 44.3 | 279× | 10.3 | 39.0 |
| shapes | flowchart | 0.21 | 70.5 | 333× | 5.0 | 81.9 |
| state | state | 0.73 | 134.3 | 184× | 7.9 | 70.6 |
| stress | flowchart | 0.26 | 62.8 | 238× | 3.4 | 37.7 |
| subgraph | flowchart | 0.26 | 67.4 | 263× | 4.2 | 38.2 |
| timeline | timeline | 0.05 | 8.8 | 187× | 3.7 | 15.9 |
| wide-labels | flowchart | 1.01 | 64.5 | 64× | 5.8 | 42.1 |

**Method.** Each cell is the median of 30 timed renders after warm-ups (flowmaid 3, mermaid 2) of the same
`.mmd` source. flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build
(`cargo run --release --example bench -- files`). mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string,
including its in-browser text measurement) in headless Chromium 153.0.8010.12 via Playwright. Speed-up = mermaid median
÷ flowmaid median; geometric mean over 25 diagrams: **131×**. SVG KB = length of the returned SVG string.
Machine: 4 CPUs, Intel(R) Xeon(R) Processor @ 2.10GHz; rustc 1.97.0 (2d8144b78 2026-07-07). Single-threaded on both sides; timings on a shared box are noisy (±10–20 %).
