
## 3. Measurements

### 3.1 Method

- `u-mermaid/tools/mmr.js <outDir> <tag> '<json config>' <files.mmd>` renders with the bundled mermaid under a
  given config and prints the effective `layout`.
- `u-mermaid/tools/extract.js <out.json> <files.svg>` loads each SVG in Chromium at 1 px per user unit and reads:
  every cluster rectangle and title box, every node's shape box, every edge's route (`data-points`, mapped through
  the element's transform, so the nested groups of mermaid-dagre's recursive render are handled) plus a 6 px
  sampling of the drawn path, and every edge label box.
- `u-mermaid/tools/analyze.py <meas.json> <key> <x|y> [--full]` prints the tables below. Coordinates are px from
  the top-left corner of the SVG's viewBox. Definitions:
  - *band*: maximal set of top-level items (blocks and nodes outside any block) whose intervals along the flow
    overlap. With ELK a band is a layer. A band is *staggered* when its items are chained by partial overlaps and
    no flow coordinate is common to all of them (mermaid-dagre and flowmaid 0.38 on the analog).
  - *cross-flow run*: a segment of at least 8 px across the flow (horizontal in TD). It is located in a gap between
    two bands, inside a block, or in a band beside its blocks.
  - *through a foreign block*: at least two path samples more than 2 px inside a cluster that is not an ancestor of
    either end. *Over a title*: a sample inside a cluster's title box.
  - label columns: distance from the label centre to its own drawn line; path length from the label to the nearer
    end of the edge; what the label box overlaps (other labels, nodes, frames, titles, other lines).
- `u-mermaid/tools/summary.py` builds the cross-input tables. Raw numbers: `u-mermaid/meas.json`,
  `meas-probes.json`, `meas-perm.json`. Per-render tables with every inter-block edge: `u-mermaid/tables/*.md`,
  probes in `u-mermaid/tables/probes/`.

### 3.2 The analog, mermaid default (ELK)

Image: `u-mermaid/renders/infra-overview.elk.png`.

@@ANALOG@@

Reading: four bands. The 21 inter-block edges put 15 runs into the gap between EXT/GH and PROD (11 lanes) and 12
into the gap between PROD and DR/STG (8 lanes). The only runs inside a block are the last or first bends of edges
that end or start in PROD and STG (lanes at 12 px pitch under the title), which is what inflates PROD's top padding
to 162 px. Fourteen labels sit more than 300 px of path from the nearer end; the row at y = 1072 holds nine of
them, 750 to 990 px from either end.

The same analog with the replication edge as in the real diagram (`u-mermaid/in/infra-overview-v2.mmd`:
`DRAPP ==> PG` instead of `PG --> DRAPP`), image `u-mermaid/renders/infra-overview-v2.elk.png`:

@@ANALOGV2@@

One edge turned round moves STANDBY above PROD and STG below it: five bands, servers one under another.

### 3.3 Corpus inputs, mermaid default (ELK): the five asked for and dense-td

Sources: `tree/examples/advanced.mmd`, `tree/examples/subgraph.mmd`, `tree/tests/fixtures/quality/fanout-frames.mmd`
(LR), `tree/examples/state.mmd` (composite state with `direction LR`), `tree/tests/fixtures/quality/bench-75.mmd`
(LR, five frames, 75 edges), and as a sixth `tree/tests/fixtures/quality/dense-td.mmd` (ten frames nested three
deep). Images: `u-mermaid/renders/<name>.elk.png`; the per-edge tables are in `u-mermaid/tables/<name>.elk.md`.

@@CORPUS@@

Notes on these tables:
- advanced: mermaid makes `Dev` a member of CI because the node is mentioned inside the subgraph body; CI's
  `direction LR` and Workers' `direction LR` are both ignored (edges cross their borders).
- subgraph: `direction LR` of `platform` is ignored for the same reason; the result is one column.
- fanout-frames: all six blocks and `n21` share one band, 40 px apart; every edge ends on a frame at mid-height;
  three lanes (180, 200, 220) carry the fan without a crossing.
- state: the composite's `direction LR` is honoured (transitions attach to the composite as a whole).
- bench-75: five blocks in a row, one per band; 35 lanes in the gap between app and data (724 px wide).
- dense-td: three top-level blocks stacked; nested blocks are blocks inside their parent; 17 lanes in one gap.

### 3.4 All inputs side by side

@@SUM1@@

Canvas area in Mpx2 next to flowmaid 0.38.0 (`b12/fm038`). Node sizes differ (mermaid wraps node text at 120 px
and uses a 120 px minimum width), so this is a rough comparison:

@@SUM2@@

Frame padding of every block (ELK renders). Left, right and bottom are 24 everywhere except STG, whose frame is
widened to its 464 px title. Top is 48 for a one-line title and 90 for a three-line one; larger values are lanes
that ELK placed inside the frame above the first row:

@@SUM3@@
