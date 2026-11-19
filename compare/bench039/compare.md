| diagram | type | flowmaid ms | mermaid ms | speed-up × | flowmaid SVG KB | mermaid SVG KB |
|---|---|--:|--:|--:|--:|--:|
| advanced | flowchart | 1.38 | 101.5 | 74× | 11.5 | 58.0 |
| architecture | architecture | 0.09 | 37.9 | 426× | 4.0 | 18.5 |
| back-edge | flowchart | 0.28 | 48.5 | 175× | 3.7 | 78.5 |
| bench-75 | flowchart | 37.32 | 261.9 | 7× | 59.0 | 194.6 |
| class | class | 0.43 | 152.6 | 352× | 11.8 | 53.5 |
| demo | flowchart | 0.29 | 50.2 | 174× | 3.7 | 78.5 |
| dense-lr | flowchart | 20.22 | 324.0 | 16× | 45.4 | 489.0 |
| dense-td | flowchart | 27.32 | 351.1 | 13× | 48.6 | 487.0 |
| er | er | 0.23 | 96.9 | 425× | 14.8 | 127.3 |
| fanout-frames | flowchart | 2.38 | 111.6 | 47× | 11.8 | 298.3 |
| fanout | flowchart | 0.77 | 60.2 | 79× | 6.6 | 84.7 |
| gitgraph | gitgraph | 0.06 | 19.6 | 306× | 5.8 | 13.0 |
| journey | journey | 0.03 | 8.6 | 253× | 3.8 | 13.2 |
| long-labels | flowchart | 0.20 | 40.3 | 199× | 3.5 | 34.7 |
| lr | flowchart | 0.34 | 48.9 | 142× | 4.5 | 80.3 |
| mindmap | mindmap | 0.71 | 75.2 | 106× | 8.7 | 49.0 |
| pie | pie | 0.04 | 4.9 | 132× | 2.8 | 5.5 |
| sankey | sankey | 0.21 | 11.7 | 55× | 6.8 | 10.3 |
| sequence | sequence | 0.13 | 37.2 | 286× | 10.3 | 38.9 |
| shapes | flowchart | 0.26 | 55.0 | 216× | 5.0 | 81.9 |
| state | state | 0.82 | 111.9 | 136× | 8.5 | 70.6 |
| stress | flowchart | 0.40 | 43.6 | 109× | 4.0 | 37.7 |
| subgraph | flowchart | 0.41 | 56.2 | 138× | 4.6 | 38.2 |
| timeline | timeline | 0.04 | 7.9 | 193× | 3.7 | 15.9 |
| wide-labels | flowchart | 0.85 | 55.5 | 65× | 6.5 | 42.1 |

**Method.** Each cell is the median of 30 timed renders after warm-ups (flowmaid 3, mermaid 2) of the same
`.mmd` source. flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build
(`cargo run --release --example bench -- files`). mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string,
including its in-browser text measurement) in headless Chromium 153.0.8010.12 via Playwright. Speed-up = mermaid median
÷ flowmaid median; geometric mean over 25 diagrams: **113×**. SVG KB = length of the returned SVG string (all-ASCII here, so chars = bytes).
Machine: 4 CPUs, Intel(R) Xeon(R) Processor @ 2.10GHz; rustc 1.97.0 (2d8144b78 2026-07-07). One render at a time on both sides; timings on a shared box are noisy (±10–20 %).
