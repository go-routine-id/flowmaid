| diagram | type | flowmaid ms | mermaid ms | speed-up × | flowmaid SVG KB | mermaid SVG KB |
|---|---|--:|--:|--:|--:|--:|
| advanced | flowchart | 1.57 | 151.4 | 96× | 11.5 | 58.0 |
| architecture | architecture | 0.08 | 61.5 | 741× | 4.0 | 18.5 |
| back-edge | flowchart | 0.36 | 62.8 | 175× | 3.7 | 78.5 |
| bench-75 | flowchart | 41.41 | 410.4 | 10× | 59.0 | 194.7 |
| class | class | 0.51 | 247.5 | 482× | 11.8 | 53.5 |
| demo | flowchart | 0.34 | 73.8 | 220× | 3.7 | 78.2 |
| dense-lr | flowchart | 26.57 | 459.7 | 17× | 45.4 | 489.0 |
| dense-td | flowchart | 41.44 | 418.6 | 10× | 48.6 | 487.1 |
| er | er | 0.40 | 146.6 | 367× | 14.8 | 127.3 |
| fanout-frames | flowchart | 3.69 | 163.4 | 44× | 11.8 | 298.3 |
| fanout | flowchart | 1.12 | 72.2 | 64× | 6.6 | 84.4 |
| gitgraph | gitgraph | 0.07 | 30.9 | 412× | 5.8 | 12.9 |
| journey | journey | 0.04 | 14.9 | 392× | 3.8 | 13.2 |
| long-labels | flowchart | 0.47 | 59.4 | 126× | 3.5 | 35.0 |
| lr | flowchart | 0.43 | 55.4 | 129× | 4.5 | 80.4 |
| mindmap | mindmap | 0.85 | 127.5 | 150× | 8.7 | 49.0 |
| pie | pie | 0.06 | 7.8 | 137× | 2.8 | 5.4 |
| sankey | sankey | 0.35 | 14.4 | 41× | 6.8 | 10.3 |
| sequence | sequence | 0.21 | 54.4 | 264× | 10.3 | 38.9 |
| shapes | flowchart | 0.28 | 67.4 | 243× | 5.0 | 81.6 |
| state | state | 0.92 | 173.8 | 189× | 8.5 | 70.6 |
| stress | flowchart | 0.44 | 63.2 | 142× | 4.0 | 37.7 |
| subgraph | flowchart | 0.44 | 82.1 | 187× | 4.6 | 38.5 |
| timeline | timeline | 0.04 | 11.3 | 263× | 3.7 | 15.9 |
| wide-labels | flowchart | 0.85 | 69.0 | 81× | 6.5 | 42.1 |

**Method.** Each cell is the median of 30 timed renders after warm-ups (flowmaid 3, mermaid 2) of the same
`.mmd` source. flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build
(`cargo run --release --example bench -- files`). mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string,
including its in-browser text measurement) in headless Chromium 153.0.8010.12 via Playwright. Speed-up = mermaid median
÷ flowmaid median; geometric mean over 25 diagrams: **127×**. SVG KB = length of the returned SVG string (all-ASCII here, so chars = bytes).
Machine: 4 CPUs, Intel(R) Xeon(R) Processor @ 2.10GHz; rustc 1.97.0 (2d8144b78 2026-07-07). One render at a time on both sides; timings on a shared box are noisy (±10–20 %).
