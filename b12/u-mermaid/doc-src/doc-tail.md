
## 4. What mermaid does badly on these inputs — do not copy

Each item names the image that shows it. All crops are in `u-mermaid/evidence/`.

### 4.1 Lines through block titles

Titles are centred at the top of the frame and edges enter through the top, so they collide.
Counts (ELK): analog 7, analog v2 10, dense-td 8, subgraph 2, advanced 1.
- `e1-title-struck-STG.png`: five lines cross the three-line title of the DEV/STAGING block.
- `e5-PROD-header-zone.png`: two lines through PROD's title.
- `e6-direction-ignored-p03.png`: the single edge into the block runs through the middle of its title.
flowmaid's invariant "cluster titles never struck by edges" must hold in B12 (title slot beside the entry ports, or
the title moved off the port columns).

### 4.2 Labels far from both ends, in mid-air rows

ELK parks a centre label in the middle layer of the edge's span.
- `e2-label-row-far-from-ends.png`: seven of the nine labels of the row y = 1072; each sits on a bare vertical
  780 to 990 px of path from its source and from its target. The reader has to trace the line to know what
  "A 203.0.113.36" belongs to.
- Numbers: analog 14 of 20 labels more than 300 px of path from the nearer end (v2: 11 of 20, dense-td: 14 of 21).
  On the same count mermaid-dagre has 6 of 20.
This is the same complaint the user made about flowmaid ("labels far from both ends"), so copying ELK's label rule
would not fix it. What is good and worth keeping: labels never overlap anything (0 overlaps in every ELK render)
and always sit on their own line.

### 4.3 `direction` dropped without notice

- `e6-direction-ignored-p03.png` (`direction LR` declared, one edge in and one out, drawn top-down).
- `e3-advanced-CI-dangling-and-direction.png`: CI declares `direction LR`, drawn top-down.
flowmaid has to decide this explicitly (B9 already honours `direction` when no edge crosses the border). mermaid's
rule is "ignore", not a design to imitate blindly.

### 4.4 Block order depends on the order of the edge statements

The same graph with its statements in another order (rows 2 to 5), with one edge added (row 6) and with mermaid's
block-level reversal switched off (row 7). Inputs `u-mermaid/in/perm-*.mmd`, tables `u-mermaid/tables/perm-*.elk.md`,
image `e10-instability.png`:

| variant | bands, top to bottom (`[X]` = block) | canvas |
|---|---|---|
| original | U2 U1 T1 / [EXT] [GH] / [PROD] / [DR] [STG] | 2199 x 2358 |
| edge lines in reverse order | [GH] / [PROD] / U2 U1 T1 [DR] / [EXT] / [STG] | 2034 x 2837 |
| edge lines rotated by nine | U1 T1 U2 / [EXT] [GH] / [PROD] / [DR] [STG] | 2376 x 2206 |
| subgraphs declared in reverse order | U1 T1 U2 / [GH] [EXT] / [PROD] / [STG] [DR] | 2422 x 2266 |
| actors declared in reverse order | U1 U2 T1 / [EXT] [GH] / [PROD] / [DR] [STG] | 2205 x 2378 |
| one more edge inside PROD (`VAULT --> PG`) | U2 T1 U1 / [GH] [EXT] / [PROD] / [STG] [DR] | 2419 x 2254 |
| `elk.orientFeedbackEdges: false` | [GH] / [PROD] T1 U2 U1 / [EXT] / [STG] [DR] | 2607 x 2438 |

With the edge lines reversed the actors end up in the middle of the drawing and External services below PROD,
because the DFS that picks the back edges (R2) starts from another root. Adding one edge between two nodes of the
same block swaps the two blocks in both two-block bands and reorders the actors. flowmaid already has a
deterministic, declaration-robust feedback set (B8); B12 should apply that at block level and keep "source order
preserved" for ties, not port mermaid's DFS.

### 4.5 Wasted space and corridor bundles on dense inputs

- `e9-dense-td-area.png`: dense-td is 4131 x 4378 = 18.1 Mpx2 (flowmaid 0.38: 7.7, mermaid-dagre: 11.5). Large empty
  areas left of the middle block and right of the bottom block; gaps of 431 and 250 px in front of the second and
  third block; 17 lanes in one gap.
- `e8-bench75-bundles.png`, `e12-bench75-elk-vs-dagre.png`: bench-75 has 35 lanes in one 724 px gap, routes that go
  round the whole drawing above and below the blocks, and nine arrowheads stacked on one side of `db` a few px
  apart.
- `e11-gap-lanes.png`: the analog's gap between EXT/GH and PROD: 11 lanes over 301 px.
- `e5-PROD-header-zone.png`: PROD's first row starts 162 px below the frame top because the last bends of the
  entering edges are routed inside the frame. Same in bench-75 (`data`: 133) and dense-td (g8: 211).
Blocks buy tidiness with area. B12 needs a budget (for example: share lanes among edges of one fan, keep the first
row at title + padding, cap the gap) instead of one lane per edge.

### 4.6 Lines hugging a frame after mermaid trims it

`evenGroupFrames` shrinks a frame after routing, so a bend that ELK placed inside the frame lands just outside it.
- `e16-lane-hugging-frame.png` (analog v2, wide): a lane 6 px below the bottom border of External services, with
  three hops on it.
- bench-75: lanes 6, 18 and 30 px beside the `edge` frame; three edges appear to cross a lateral side of `ops`.
Route against the final frame.

### 4.7 Edges to the own block, block self-loops, node self-loops

- `e3-advanced-CI-dangling-and-direction.png`: `Dev --> CI` with Dev inside CI is drawn as an arrow from the node
  down to the inside of its own frame's bottom border.
- `e7-block-self-loop-p14.png`: `A --> A` on a block leaves the bottom, runs round the right side and re-enters at
  the top, with the label hanging beside it.
- `e4-advanced-selfloop-title.png`: `W1 --> W1` is a stray arrowhead on the node's left side; the thick edge
  crosses the "Async Workers" title.

### 4.8 Reversed subgraph order, member capture

- Disconnected subgraphs are drawn in reverse declaration order (p09: A, B, C declared, C B A drawn).
- A node that is merely mentioned inside a subgraph body becomes its member (advanced: `Dev`).
Both are mermaid parser behaviour, not layout rules. flowmaid keeps its own rules (PLAN invariant 4, B2).

### 4.9 Things mermaid-ELK does well (the look the user wants)

| property | measured |
|---|---|
| every block is one compact rectangle in one band | no staggered band in any of the 8 ELK renders |
| no line through a block it does not belong to | 0 in 8 ELK renders (dagre: 33 in 6) |
| no line over a node that is not its end | 0 in 8 ELK renders (dagre: 50) |
| frames are crossed on flow-facing sides only | analog: top 16 + bottom 5 entries, bottom 13 + top 5 exits |
| labels on their line, none overlapping | 0 of 70 labels off line or overlapping |
| even frame padding | 24 px left, right, bottom on every block |
| even lanes | 20 px pitch |

## 5. What this means for B12 (checklist for the design)

Copy:
1. Recursive compound layout for **every** subgraph and composite state: block = one vertex in its parent's
   ranking, inner layout first, frame size from the inner layout (R1, R4). B9's `src/compound.rs` does this for the
   "own direction, no edge across the border" case; B12 removes both conditions.
2. Block-level feedback orientation before ranking, so that a few return edges do not reorder the blocks (R2).
3. Same-rank blocks side by side with a fixed gap and room for corridors (R3); alignment along the flow by
   in/out ratio or simply at the rank's start (R5).
4. Cross-block edges through flow-facing sides only, lanes in the gaps, corridors beside blocks for passing edges,
   never through a foreign block (R6).
5. Node-to-block edges end on the frame's flow-facing side (R7).
6. Labels on the outer part of the edge, never inside an end block (R8, first half).

Do not copy: 4.1 to 4.8. In short: titles stay clear, labels stay near an end or near their bundle, `direction`
is a decision not an accident, order is stable under re-ordering of statements, area is budgeted, frames are final
before routing, degenerate edges get a defined drawing.

Open points the measurements raise for the design:
- A block's inner layout uses the parent's direction in mermaid. flowmaid could keep a declared `direction` even
  with edges across the border (ports then sit on the sides that face the outer flow); mermaid gives no reference
  for that case.
- mermaid's node-level detail inside a band depends on ELK's hierarchical crossing minimisation (port order on the
  frame and node order inside are solved together). A simpler scheme (order blocks first, then order members with
  the frame ports fixed) should be compared on the analog: mermaid has 16 crossings there, 13 in v2.
- Diagrams without subgraphs are untouched by all of this in mermaid as well (a flat ELK graph).

## 6. Files

| path (under `b12/`) | content |
|---|---|
| `understand-mermaid.md` | this document |
| `u-mermaid/renders/` | SVG + PNG of the analog and the corpus under ELK, dagre and the config variants |
| `u-mermaid/in/` | analog variants: front-matter elk/dagre, `flowchart-elk`, v2 (replication edge turned round), five permutations |
| `u-mermaid/probes/`, `u-mermaid/probe-renders/` | 14 behaviour probes, each rendered with ELK and dagre |
| `u-mermaid/tables/`, `u-mermaid/tables/probes/` | generated measurement tables, one file per render |
| `u-mermaid/evidence/` | crops and sheets e1 to e16 cited in section 4 |
| `u-mermaid/sheets/` | contact sheets (ELK beside dagre) |
| `u-mermaid/tools/` | `mmr.js`, `extract.js`, `analyze.py`, `summary.py`, `elkprobe.js`, `crop.py`, `sheet.py` |
| `u-mermaid/meas*.json`, `u-mermaid/elkprobe.out.txt` | raw geometry and the direct-ELK probe output |
| `u-mermaid/fm038/` | flowmaid 0.38.0 SVGs of the same inputs (canvas sizes only) |

Nothing was built with cargo, nothing was written under `b12/tree` or `/home/user/flowmaid`, nothing was committed.
The only files added to `b12/private/` are five enlarged crops of the two private screenshots.
