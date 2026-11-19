| diagram | type | flowmaid ms | mermaid ms | speed-up × | flowmaid SVG KB | mermaid SVG KB |
|---|---|--:|--:|--:|--:|--:|
| advanced | flowchart | 1.69 | 119.4 | 71× | 11.5 | 58.0 |
| architecture | architecture | 0.11 | 46.7 | 428× | 4.0 | 18.5 |
| back-edge | flowchart | 0.36 | 55.8 | 155× | 3.7 | 78.5 |
| bench-75 | flowchart | 45.85 | 326.0 | 7× | 65.1 | 194.2 |
| class | class | 0.54 | 162.9 | 302× | 11.8 | 53.5 |
| demo | flowchart | 0.33 | 62.9 | 191× | 3.7 | 78.5 |
| dense-lr | flowchart | 22.48 | 377.5 | 17× | 51.3 | 489.1 |
| dense-td | flowchart | 31.96 | 396.1 | 12× | 54.3 | 487.0 |
| er | er | 0.52 | 113.6 | 220× | 14.8 | 127.4 |
| fanout-frames | flowchart | 2.69 | 138.8 | 52× | 11.8 | 298.2 |
| fanout | flowchart | 1.24 | 68.1 | 55× | 6.6 | 84.7 |
| gitgraph | gitgraph | 0.12 | 23.5 | 191× | 5.8 | 13.0 |
| journey | journey | 0.07 | 10.1 | 136× | 3.8 | 13.2 |
| long-labels | flowchart | 0.29 | 45.3 | 158× | 3.5 | 35.0 |
| lr | flowchart | 0.37 | 57.1 | 153× | 4.5 | 80.4 |
| mindmap | mindmap | 0.79 | 89.7 | 113× | 8.7 | 49.0 |
| pie | pie | 0.04 | 6.2 | 155× | 2.8 | 5.5 |
| sankey | sankey | 0.23 | 13.0 | 57× | 6.8 | 10.3 |
| sequence | sequence | 0.14 | 43.5 | 302× | 10.3 | 39.0 |
| shapes | flowchart | 0.28 | 67.3 | 238× | 5.0 | 81.9 |
| state | state | 0.96 | 130.5 | 136× | 8.5 | 70.6 |
| stress | flowchart | 0.47 | 48.4 | 104× | 4.0 | 37.7 |
| subgraph | flowchart | 0.43 | 73.8 | 173× | 4.6 | 38.5 |
| timeline | timeline | 0.05 | 9.4 | 204× | 3.7 | 15.9 |
| wide-labels | flowchart | 0.88 | 60.5 | 69× | 6.5 | 42.1 |

**Method.** Each cell is the median of 30 timed renders after warm-ups (flowmaid 3, mermaid 2) of the same
`.mmd` source. flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build
(`cargo run --release --example bench -- files`). mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string,
including its in-browser text measurement) in headless Chromium 153.0.8010.12 via Playwright. Speed-up = mermaid median
÷ flowmaid median; geometric mean over 25 diagrams: **106×**. SVG KB = length of the returned SVG string (all-ASCII here, so chars = bytes).
Machine: 4 CPUs, Intel(R) Xeon(R) Processor @ 2.10GHz; rustc 1.97.0 (2d8144b78 2026-07-07). One render at a time on both sides; timings on a shared box are noisy (±10–20 %).
