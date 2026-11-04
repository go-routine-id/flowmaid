| diagram | type | flowmaid ms | mermaid ms | speed-up × | flowmaid SVG KB | mermaid SVG KB |
|---|---|--:|--:|--:|--:|--:|
| advanced | flowchart | 1.64 | 122.4 | 75× | 10.9 | 58.0 |
| architecture | architecture | 0.12 | 65.2 | 567× | 4.0 | 18.4 |
| back-edge | flowchart | 0.53 | 62.5 | 118× | 3.5 | 78.5 |
| bench-75 | flowchart | 47.71 | 368.7 | 8× | 69.6 | 194.6 |
| class | class | 0.56 | 206.5 | 371× | 11.8 | 53.5 |
| demo | flowchart | 0.36 | 88.0 | 247× | 3.5 | 78.5 |
| dense-lr | flowchart | 26.71 | 569.3 | 21× | 51.0 | 489.0 |
| dense-td | flowchart | 47.01 | 588.5 | 13× | 53.9 | 487.0 |
| er | er | 0.47 | 155.1 | 332× | 14.8 | 127.3 |
| fanout-frames | flowchart | 4.45 | 187.4 | 42× | 12.5 | 298.1 |
| fanout | flowchart | 1.37 | 88.4 | 64× | 5.8 | 84.7 |
| gitgraph | gitgraph | 0.12 | 24.7 | 211× | 5.8 | 13.0 |
| journey | journey | 0.06 | 9.6 | 168× | 3.8 | 13.2 |
| long-labels | flowchart | 0.50 | 53.2 | 107× | 3.2 | 34.7 |
| lr | flowchart | 0.74 | 74.6 | 100× | 3.9 | 80.4 |
| mindmap | mindmap | 1.29 | 101.8 | 79× | 8.7 | 48.7 |
| pie | pie | 0.06 | 9.6 | 168× | 2.8 | 5.5 |
| sankey | sankey | 0.35 | 20.7 | 59× | 6.8 | 10.3 |
| sequence | sequence | 0.22 | 45.7 | 209× | 10.3 | 39.0 |
| shapes | flowchart | 0.58 | 81.3 | 141× | 5.0 | 81.6 |
| state | state | 1.51 | 188.8 | 125× | 8.3 | 70.4 |
| stress | flowchart | 0.49 | 58.9 | 119× | 3.7 | 37.7 |
| subgraph | flowchart | 0.47 | 68.8 | 146× | 4.5 | 38.5 |
| timeline | timeline | 0.04 | 8.9 | 212× | 3.7 | 15.9 |
| wide-labels | flowchart | 0.82 | 76.3 | 92× | 5.7 | 42.1 |

**Method.** Each cell is the median of 30 timed renders after warm-ups (flowmaid 3, mermaid 2) of the same
`.mmd` source. flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build
(`cargo run --release --example bench -- files`). mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string,
including its in-browser text measurement) in headless Chromium 153.0.8010.12 via Playwright. Speed-up = mermaid median
÷ flowmaid median; geometric mean over 25 diagrams: **105×**. SVG KB = length of the returned SVG string (all-ASCII here, so chars = bytes).
Machine: 4 CPUs, Intel(R) Xeon(R) Processor @ 2.10GHz; rustc 1.97.0 (2d8144b78 2026-07-07). One render at a time on both sides; timings on a shared box are noisy (±10–20 %).
