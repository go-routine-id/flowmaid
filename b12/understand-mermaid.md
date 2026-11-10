# How mermaid 12.2 places subgraphs — reference for B12 "subgraphs as blocks"

Label: u-mermaid. Date: 2026-11-10. Everything below was measured on the bundled build
`scratchpad/mm/node_modules/mermaid` (mermaid 12.2.0, elkjs 0.9.3, dagre-d3-es 7.0.14) with the sanitized analog
`b12/in/infra-overview.mmd`. No name or address from the private screenshots appears in this file.

All paths below are relative to `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/`
unless they start with `/`. Working directory of this study: `u-mermaid/`.

## 0. Findings in one page

1. **The user's mermaid screenshot is ELK, not dagre.** In mermaid 12.2.0 the root config default is `layout: "elk"`
   (in mermaid 11 it was `dagre`). Default config, `layout: elk`, front-matter `config: layout: elk` and `flowchart-elk`
   all give the same geometry. `layout: dagre` gives a drawing with the same defect the user saw in flowmaid 0.38
   (the "External services" frame is 1413 px tall, 89 % of the canvas height).
2. **In ELK every subgraph is one vertex of its parent's layering.** A block occupies exactly one layer (band) of the
   level it lives in; its members are layered inside it; the block's size is computed bottom-up. That alone produces
   "actors, then External services as one row, then the servers".
3. **Blocks that share a layer sit side by side**, 40 px apart, with corridors between them for long edges.
4. **An edge between blocks crosses frames only on the two flow-facing sides** (top/bottom in TD). Its sideways runs
   lie in lanes in the gap between two bands (20 px pitch) or, for a band it only passes, in a corridor beside the
   blocks of that band. It never crosses a block it does not belong to (0 cases in the 8 ELK renders measured; mermaid-dagre: 33 in 6).
5. **Block-level back edges are reversed by mermaid before ELK runs** (`subgraphFeedbackEdges.ts`). That is why the
   three "server -> Chat bot" edges do not drag "External services" below the servers: they run straight back up and
   enter the block from below.
6. **A node -> block edge ends on the block's frame**, at a port on the flow-facing side (alone on the side: at its
   middle).
7. **Labels are dummy nodes in a layer of their own**, on the line (inline), on the part of the edge that lies
   outside the blocks, at the middle layer of the edge's span.
8. `direction` inside a subgraph is honoured **only if no edge crosses the subgraph's border** (edges to the subgraph
   as a whole are fine). Otherwise it is silently ignored. Same condition as mermaid-dagre and as flowmaid's B9.
9. **Do not copy**: lines through block titles (7 in the analog), labels in mid-air rows far from both ends (14 of 20
   labels are more than 300 px of path from the nearer end), block order that depends on the order of the edge
   statements (same graph, edges listed in reverse: the actors land in the middle of the drawing), blocks swapping
   sides after one unrelated edge is added, area blow-up on dense inputs (dense-td: 18.1 Mpx2 against flowmaid 0.38's 7.7), lanes
   6 px from a frame after mermaid trims frames, dangling arrows for an edge from a member to its own block.
10. **The analog differs from the real diagram in one edge.** Both private screenshots draw the replication link
    thick and pointing from the standby stack to the database; the analog has `PG --> DRAPP`. With the analog as
    written mermaid puts STANDBY and DEV/STAGING side by side under PROD. With that one edge turned round
    (`u-mermaid/in/infra-overview-v2.mmd`) mermaid stacks STANDBY, PROD, DEV/STAGING one under another, as in the
    screenshot. Both are measured below. I did not edit `b12/in/`.

## 1. Which layout produced the screenshot

### 1.1 What the build does by default

| setting | effective layout | evidence |
|---|---|---|
| `mermaid.initialize({startOnLoad:false})` only | **elk** | `mermaid.mermaidAPI.getConfig().layout === "elk"`; schema default `"layout": "elk"` at `chunks/mermaid.core/chunk-DSNLSHJZ.mjs:4675`; `state` also carries `"layout": "elk"` (line 4250) |
| `{layout: "elk"}` | elk | cluster rectangles identical to the default render |
| front-matter `config: layout: elk` | elk | identical |
| `flowchart-elk TD` (default config) | elk | identical; detector `mermaid.core.mjs:421` sets `config.layout = "elk"` |
| `{layout: "dagre"}` or front-matter `config: layout: dagre` | dagre | different geometry, see below |
| `flowchart-elk TD` with `{layout: "dagre"}` | dagre | the keyword does not override an explicit site config |

ELK is bundled in this build (`chunks/mermaid.core/elk-QONTBCTQ.mjs`, loaded lazily by
`chunk-EJLWFMJV.mjs: registerDefaultLayoutLoaders`); no extra package is needed. The reference renders the project
already keeps in `scratchpad/cmp/out/*.mermaid.svg` are ELK renders as well: their cluster rectangles are identical
to my `{layout:"elk"}` renders of `advanced` and `subgraph`.

Cluster rectangles of the analog (x, y, w x h in px):

| render | DR | EXT | GH | PROD | STG | canvas |
|---|---|---|---|---|---|---|
| default / `layout: elk` / front-matter elk / `flowchart-elk` | 203,1717 604x285 | 434,206 1009x201 | 1736,248 402x159 | 1170,708 579x726 | 927,1717 472x633 | 2199 x 2358 |
| `layout: dagre` / front-matter dagre / `flowchart-elk` + dagre config | 2782,1154 626x221 | 1858,166 905x1413 | 1414,391 424x137 | 508,391 885x650 | 8,599 480x776 | 3416 x 1587 |

### 1.2 Comparison with the screenshot

| feature in the user's mermaid screenshot | ELK render of the analog | dagre render of the analog |
|---|---|---|
| edges are orthogonal polylines with small rounded corners | yes (`curve = "rounded"`, radius 5) | no: `basis` curves |
| small arcs where two lines cross | yes (`elk.lineHops: true`, radius 6) | no |
| each subgraph is one compact rectangle | yes | no: EXT spans 89 % of the height, DNS node at the top, Chat bot at the bottom |
| External services is one row of five nodes | yes (5 nodes in one inner layer) | no (3 inner ranks: 1 / 2 / 2) |
| Code hosting sits beside External services in the same band | yes | no |
| arrows enter and leave a block only through its top or bottom | yes (21 of 21 block-crossing edges) | no (left 7 / right 8 / top 6 entries) |
| edge labels lie on their line, several of them on one horizontal row | yes (rows of 3, 5, 9, 3 labels) | partly |
| three server blocks one under another | with the replication edge as in the real diagram (v2): yes | no |

Images: `u-mermaid/evidence/e13-analog-elk-vs-dagre.png` (both layouts of the analog),
`u-mermaid/evidence/e14-analog-v2-elk-wide.png` (v2, `flowchart.wrappingWidth: 400` to mimic the unwrapped node
labels of the playground; it reproduces the screenshot's arrangement block for block, mirrored left to right; the
order across the flow depends on declaration order, see 4.4).

Renders kept in `u-mermaid/renders/`: `infra-overview.default.*`, `.elk.*`, `.dagre.*`, `.elk-wide.*`,
`.elk-nofb.*` (ELK with `elk.orientFeedbackEdges: false`), `infra-overview-fm-elk.default.*`,
`infra-overview-fm-dagre.default.*`, `infra-overview-flowchart-elk.default.*`,
`infra-overview-flowchart-elk.dagre-cfg.*`, `infra-overview-v2.elk.*`, `infra-overview-v2.elk-wide.*`,
`perm-*.elk.*` (declaration-order permutations), and `<name>.elk.*` / `<name>.dagre.*` for advanced, subgraph,
fanout-frames, state, bench-75, dense-td. Fourteen behaviour probes: `u-mermaid/probes/*.mmd`, rendered to
`u-mermaid/probe-renders/*.{elk,dagre}.{svg,png}`.

## 2. How the layout handles subgraphs

### 2.1 What mermaid hands to ELK (`elk-QONTBCTQ.mjs`)

The adapter builds one nested ELK graph: every subgraph is an ELK node with `children`; all edges are stored at the
root, whatever their ends (`addEdgesToElkGraph`). Then:

| step | function | what it does |
|---|---|---|
| root options | `createRootElkGraph` | `elk.algorithm: elk.layered`, `elk.hierarchyHandling: INCLUDE_CHILDREN`, `elk.direction` from the diagram, `spacing.baseValue: 40`, layering `NETWORK_SIMPLEX`, cycle breaking `DEPTH_FIRST`, node placement `BRANDES_KOEPF` with `bk.fixedAlignment: BALANCED`, `considerModelOrder.strategy: NODES_AND_EDGES`, `unnecessaryBendpoints: true`, `mergeHierarchyEdges: true`, `mergeEdges: false`, `spacing.portsSurrounding: 12` on every side, edge routing left at ELK's default `ORTHOGONAL` |
| subgraph options | `buildSubgraphLayoutOptions` | `elk.padding: 24` on all four sides (ELK adds the title strip on top), `nodeLabels.placement: [H_CENTER V_TOP, INSIDE]`, `nodeSize.constraints: [MINIMUM_SIZE, NODE_LABELS]` with minimum width = title width + padding, `spacing.baseValue: 24`, `spacing.nodeNode: 50`, `elk.layered.spacing.edgeNodeBetweenLayers: 30`, `elk.spacing.edgeEdge: 20`, `nodePlacement.networkSimplex.nodeFlexibility: PORT_POSITION`, ports margin 12 |
| own direction | same function | only `if (node.dir)`: `elk.direction` = that direction and `elk.hierarchyHandling: SEPARATE_CHILDREN` |
| cross-border edges | `configureCrossHierarchyEdges` -> `setIncludeChildrenPolicy` | for every edge whose ends have different parents: both end nodes and all their ancestors up to the common ancestor get `elk.hierarchyHandling: INCLUDE_CHILDREN`. This overrides `SEPARATE_CHILDREN`, so a declared `direction` is lost |
| block-level back edges | `reverseSubgraphFeedbackEdges` (`subgraphFeedbackEdges.ts`), on unless `elk.orientFeedbackEdges: false` | see 2.3 rule R2 |
| edge labels | `addEdgesToElkGraph` | one ELK label per edge with `edgeLabels.inline: true`, `edgeLabels.placement: CENTER` |
| after layout | `evenGroupFrames` | every frame is **trimmed**: left, right and bottom are pulled in to (children and internal-edge points) + 24; the top stays. Bends that ELK had put inside the frame for edges crossing its border are not counted, so they end up outside the trimmed frame |
| after layout | `straightenEdgeTerminals` | removes a jog of at most 16 px near an end if that adds no crossing and covers no label |
| paint | `insertEdge` with `curve: "rounded"` | orthogonal polyline, corner radius 5; `applyElkLineJumps` draws arcs (radius 6) at crossings; `skipIntersect: true` (ELK's port points are used as they are) |

### 2.2 What ELK layered does with that graph

ELK's Java source is not readable here (elkjs ships it compiled), so each statement is backed by a probe: direct
elkjs runs in `u-mermaid/tools/elkprobe.js` (output `u-mermaid/elkprobe.out.txt`) or mermaid renders of
`u-mermaid/probes/`.

| statement | probe | observation |
|---|---|---|
| A block is one entry of one layer of its parent graph | elkprobe 3; p10 | p10: block A (255 px tall, three inner layers) shares a layer with the plain node `n`; both centres at y = 315.5 |
| Inside the block the members get their own layers, in the same direction | p01, p03, p04 | a->b->c stacked top-down inside the block in a TD chart |
| A child's `elk.direction` is ignored under `INCLUDE_CHILDREN`, honoured under `SEPARATE_CHILDREN` | elkprobe 2; p02, p03, p13 | same graph: 124x184 (stacked) against 364x64 (row) |
| A long edge passes a band beside its blocks, never through them | elkprobe 3; p08 | p08: the edge that skips block B runs at x = 28.7, 58.6 px left of B's frame (40 px spacing + half its label) |
| An edge from outside to a node two levels down crosses each frame on the flow-facing side | p07 | x -> i1 enters `outer` through its top at x = 241.6, runs down inside `outer` 12 px beside `o2`, enters `inner` through its top |
| Items of one layer are aligned along the flow by port ratio out / (in + out): only incoming = at the layer's start, only outgoing = at its end, both or none = centred | elkprobe 1; analog | elkprobe 1: next to a 200 px node the sink sits at the top, the source at the bottom, the through node in the middle. Analog: GH (only outgoing) is bottom-aligned with EXT (both bottoms at y = 407); DR and STG (only incoming after R2) are top-aligned (both tops at y = 1717); inside EXT the mail node (no edges) is centred, Chat bot / Blob / LLM hug the bottom |
| Without mermaid's pre-reversal a back edge between blocks keeps its sides and loops round the outside | elkprobe 4; `renders/p06-block-cycle.elk-nofb.png`, `renders/infra-overview.elk-nofb.png` | elkprobe 4: 6-point detour round both blocks. Analog without pre-reversal: External services drops below PROD and the actors (bands: GH / PROD + actors / EXT / STG + DR), canvas 6.36 Mpx2 instead of 5.18 |
| A labelled edge needs a layer for its label | p04, state | gap between two bands joined by one labelled straight edge = 40 + 21 + 40 = 101 px (p04, state), against 40-50 px without a label |
| Disconnected parts are packed side by side, subgraphs in reverse declaration order | p09 | A, B, C declared -> drawn C, B, A, then the lone node |
| Members without any edge: in a block that is laid out on its own (R9 case) ELK packs them as separate components into a grid; in a block with cross-border edges they join the row of the connected members | fanout-frames; p14; analog | fanout-frames g1: five edge-less nodes in 2 columns x 3 rows, g3: 2 x 3. p14: e2 (has an edge), e1, e3 in one row. Analog: the mail node sits in the row of External services |

### 2.3 Rules a re-implementation needs

Terms: *flow axis* = y for TD/TB, x for LR; *band* = one layer of the root graph; *level* = the graph of one block
(the root is a level too).

**R1. Hierarchy as nested graphs.** Each level's vertices are its direct member nodes and its direct sub-blocks. An
edge u -> v belongs to the level L of the lowest common ancestor of u and v. At L it joins cu and cv, the children of
L that contain u and v (or u, v themselves). If cu is a block, the edge continues inside cu from a point on cu's
frame (a *hierarchical port*) to u, level by level.

**R2. Block order along the flow = layering of the collapsed graph, after reversing block-level back edges.**
mermaid's rule (`findSubgraphFeedbackEdges`), per level: take the edges of that level collapsed to (cu, cv); run a
depth-first search whose roots are first the vertices without incoming edge, then all vertices, each in order of
first appearance in the edge list, following out-edges in edge-list order; an edge that reaches a vertex still on
the DFS stack is a back edge. Back edges with at least one end inside a block are reversed for layout and drawn
with the arrowhead at the original target. Back edges between two plain siblings are left to ELK's own
`DEPTH_FIRST` cycle breaker. Then `NETWORK_SIMPLEX` layering per level, every block counted as one vertex.
In the analog this reverses five edges (three servers -> Chat bot, product lane -> LLM API, product lane -> Blob
storage) and yields: actors / EXT + GH / PROD / DR + STG.
*Weak point, see 4.4:* the result depends on the order of the edge statements.

**R3. Two blocks sit side by side when the layering gives them the same layer**, that is when no chain of
(collapsed, oriented) edges orders them and network simplex has no reason to separate them: DR and STG (both fed by
EXT, GH and PROD, no edge between them), EXT and GH (GH only feeds the servers, so it is pulled down to the layer
just above its nearest target). One extra edge between two such blocks stacks them (v2: STANDBY -> PROD -> STG).
Order across the flow inside a layer: layer-sweep crossing minimisation over the whole hierarchy, started from and
tied by declaration order. Spacing 40 px between neighbours; wider where edge or label dummies sit between them
(analog: EXT -> GH 292 px, DR -> STG 121 px).

**R4. Block size is computed bottom-up.** Inner layout first, with the parent's direction (own `direction` only
under R9). Frame = inner bounding box + 24 px left, right and bottom, and title strip + 24 px on top (48 px for a
one-line title, 90 px for a three-line title of 63 px). Minimum width = title width + the cluster's `padding`
(8 here; STG: 464 px title, 472 px frame). Title centred at the top inside the frame.

**R5. Position along the flow inside a band**: by port ratio (table in 2.2). Sources hug the next band, sinks the
previous one.

**R6. Block-to-block edge routing** (TD wording; orthogonal, corner radius 5):
- it leaves the source node at the bottom, runs straight down through the bottom side of every frame around it;
- it turns in a lane in the gap below the source band or above the target band; one lane per edge;
- a band it merely passes is crossed in a corridor beside the blocks of that band, as a straight vertical;
- it enters the target block through the top side and runs straight down to the top of the target node;
- a reversed (back) edge does the same from target to source, so on the drawing it leaves the source node at the
  **top**, leaves the source block through its top, enters the target block through its **bottom** and ends at the
  bottom of the target node (p06: 8 points, both blocks stay in place; analog: "leaves via top: 5, enters via
  bottom: 5");
- lateral frame sides are never used by ELK. (After mermaid's frame trimming 3 of 70 edges in bench-75 look as if
  they entered through a lateral side: their last bend was inside ELK's frame and is outside the trimmed one.)

**R7. Node-to-block edge.** It ends on the frame, on the flow-facing side, arrowhead on the frame. Alone on that
side it sits at the middle (fanout-frames: all six hit their frame at exactly half its height; p04: x = 209 = centre
of S). With other crossings on the same side it is ordered among them by crossing minimisation (analog: the three
`R2 -> block` edges come from the right and end 28-56 px from the right corner of DR, PROD, STG). Ports keep 12 px
from a corner. An edge leaving a block as a whole starts on its frame the same way (p04: two edges leave S's bottom
at 1/3 and 2/3 of its width).

**R8. Labels.** One label = one dummy node on the edge, centred on the line (the line is interrupted by the chip),
always on the part of the edge that belongs to the common-ancestor level, so never inside either end block. For an
edge spanning several bands it sits in the middle band, beside that band's blocks (analog: 9 labels on the row
y = 1072 beside PROD). Labels of one layer share one row, at least 20 px apart. A labelled edge between adjacent
bands adds a label row to the gap.

**R9. `direction` in a subgraph.** Honoured exactly when no edge has one end inside the subgraph and the other
outside. Edges to or from the subgraph as a whole do not count. Otherwise the subgraph is laid out in its parent's
direction, without any warning. A subgraph without `direction` always uses the parent's direction in ELK.

**R10. Measured spacing (mermaid defaults, 14-16 px text).**

| quantity | value | where measured |
|---|---|---|
| neighbours across the flow, root level | 40 | p05 (40, 40), fanout-frames (6 x 40), actors of the analog (40, 43) |
| neighbours across the flow, inside a block | 50 | p04 (b -> c) |
| band gap, plain node to plain node, no lane | 40 | subgraph (Client -> GW), state |
| band gap, node to block or block to block, no label | 50-60 | p05, p10 (50), advanced (50, 50), subgraph (50, 60) |
| band gap with one labelled straight edge | 101 = 40 + 21 + 40 | p04, state |
| first lane after a plain node / after a block frame | 20 / 30 (20-40 seen) | p05; analog (EXT bottom 407, first lane 437); p04: 20; subgraph: 40 |
| lane pitch in a root gap / inside a block | 20 / 12 | analog gap 1-2: 437, 457 ... 577; inside PROD: 828, 840 |
| label row to the neighbouring lane | 20 | analog gap 1-2: lane 577, label top 597.5 |
| labels in one row | >= 20 apart | analog rows: gaps 20, 20, 20, 45, 49, 70 ... |
| layer gap inside a block, straight edge / with one lane | 24 / 60 (30 + 30) | p08 (b1 -> b2), p04 (a -> b, c) |
| frame padding left, right, bottom / top | 24 / title strip + 24 | all blocks of all inputs (table in 3.4) |
| port pitch on a node side | about (side - 24) / (n + 1) | DNS: six ports, 18.6 px apart on a 152 px side, first and last 29.5 px from the corners |
| corner radius / hop radius | 5 / 6 | `generateRoundedPath(.., 5)`, `JUMP_RADIUS = 6` |

A gap between two bands is therefore: 20 or 30 (clearance) + 20 x (lanes before the label row - 1) + label row with
20 on each side + 20 x (lanes after it - 1) + 20 or 30. The analog's gap between EXT/GH and PROD is 301 px: 8 lanes,
a label row, 3 lanes. Its gap between PROD and DR/STG is 283 px: 1 lane, a label row, 7 lanes.

### 2.4 mermaid-dagre (the old default; kept in 12.2 as `layout: dagre`)

Source: `chunks/mermaid.core/dagre-NK4GIZVX.mjs` (index.js), `chunk-ZCHMQSOR.mjs` (mermaid-graphlib.js),
`dagre-d3-es/src/dagre/{nesting-graph,add-border-segments,parent-dummy-chains}.js`, `order/add-subgraph-constraints.js`.

1. **Compound graph.** `prepareLayoutForDagre` builds one graphlib graph (`multigraph`, `compound`), `setParent` for
   members, `rankdir` = diagram direction, `nodesep` = `ranksep` = 50, margins 8. A self-loop becomes two 10x10
   dummy nodes and three edges.
2. **External connections.** `adjustClustersAndEdges`: a cluster has `externalConnections` when some edge has
   exactly one end among its descendants. An edge whose end is the cluster itself does not count.
3. **Edge to a cluster.** If the cluster has external connections, the edge is re-attached to a stand-in leaf inside
   it (`findNonClusterChild`, `getAnchorId`) and flagged `toCluster` / `fromCluster`; dagre lays it out as an edge to
   that leaf and the painter cuts the path where it meets the cluster rectangle (`cutPathAtIntersect` in
   `insertEdge`). If the cluster has no external connections the edge stays on the cluster, which becomes one node
   (next point), and is clipped at its rectangle.
4. **Recursive render of a cluster without external connections.** `extractor` removes the cluster and its
   descendants from the graph and replaces them by one node `clusterNode: true` that carries its own graph. That
   graph's `rankdir` is the declared `dir` or, when none is declared, **the opposite of the parent's** (`TB -> LR`,
   anything else `-> TB`); `nodesep` 50, `ranksep` = the parent's + 25. `measureDagreGraph` lays the inner
   graph out first, measures it and gives the node that size. Probe p01: a -> b -> c in an isolated subgraph of a TD
   chart is drawn left to right by dagre and top-down by ELK.
5. **A cluster with external connections is not a block.** It stays a dagre compound node: `nesting-graph.js` adds
   a top and a bottom border node per cluster and nesting edges, multiplies every `minlen` by 2 x (nesting height) + 1, and
   network simplex ranks **each member on its own**. The cluster covers the ranks from its topmost to its
   bottommost member; outside nodes share those ranks beside it. `add-border-segments.js` adds a left and a right
   border node per rank, the ordering keeps members contiguous in each rank (`add-subgraph-constraints.js`), and the
   rectangle is the box of the border nodes. This is the behaviour flowmaid 0.38 also has, and the reason for the
   tall "External services" frame.
6. **`direction` inside a subgraph**: used only in case 4. A cluster with an edge across its border ignores it.

Measured on the analog with dagre: EXT 905 x 1413 (89 % of the height), one band holding all five blocks with no
flow coordinate common to them, 6 edges through blocks they do not belong to, 10 edge-over-node cases, frame
crossings on all four sides, 8 of 20 labels more than 12 px off their line (`u-mermaid/tables/infra-overview.dagre.md`).

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

**infra-overview.elk.svg  (canvas 2199 x 2358, flow axis y)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 434 | 206 | 1009 | 201 | 9% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 1736 | 248 | 402 | 159 | 7% | 2 (2 incl. nested) | 2 |
| PROD | 0 | - | 1170 | 708 | 579 | 726 | 31% | 5 (5 incl. nested) | 3 / 1 / 1 |
| DR | 0 | - | 203 | 1717 | 604 | 285 | 12% | 3 (3 incl. nested) | 3 |
| STG | 0 | - | 927 | 1717 | 472 | 633 | 27% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U2 , U1 , T1 | 132 |
| 1 | 206 | 407 | 201 | [EXT] , [GH] | 301 |
| 2 | 708 | 1434 | 726 | [PROD] | 283 |
| 3 | 1717 | 2350 | 633 | [DR] , [STG] | - |

Top-level blocks: 5; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U2 -> U1 = 40; band 0: U1 -> T1 = 43; band 1: [EXT] -> [GH] = 292; band 3: [DR] -> [STG] = 121.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 2 | 370 | 1 (176) |
| gap 1-2 | 15 | 4682 | 11 (437, 457, 477, 497, 517, 537, 557, 577, 638, 658, 678) |
| band 2 inside [PROD] | 6 | 375 | 5 (828, 840, 1029, 1041, 1272) |
| gap 2-3 | 12 | 5519 | 8 (1464, 1567, 1587, 1607, 1627, 1647, 1667, 1687) |
| band 3 inside [STG] | 2 | 58 | 2 (2113, 2125) |

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_U1_DNS_0 | U1 (U1) | DNS (EXT) | 0 -> 1 | straight |
| L_T1_DNS_0 | T1 (T1) | DNS (EXT) | 0 -> 1 | gap 0-1 187 |
| L_U2_DNS_0 | U2 (U2) | DNS (EXT) | 0 -> 1 | gap 0-1 184 |
| L_R1_ARGO_0 | R1 (GH) | ARGO (PROD) | 1 -> 2 | gap 1-2 25; gap 1-2 101; band 2 inside [PROD] 39 |
| L_R2_STG_0 | R2 (GH) | STG (STG) | 1 -> 3 | gap 1-2 83; gap 2-3 795 |
| L_R2_DR_0 | R2 (GH) | DR (DR) | 1 -> 3 | gap 1-2 68; gap 2-3 1186 |
| L_R2_PROD_0 | R2 (GH) | PROD (PROD) | 1 -> 2 | gap 1-2 317 |
| L_DNS_TR2_0 | DNS (EXT) | TR2 (STG) | 1 -> 3 | gap 1-2 100; gap 2-3 410 |
| L_DNS_PSPROD_0 | DNS (EXT) | PSPROD (STG) | 1 -> 3 | gap 1-2 200; gap 2-3 472 |
| L_DNS_DRAPP_0 | DNS (EXT) | DRAPP (DR) | 1 -> 3 | gap 1-2 99; gap 2-3 248 |
| L_DNS_PSDEV_0 | DNS (EXT) | PSDEV (DR) | 1 -> 3 | gap 1-2 418; gap 2-3 233 |
| L_DNS_SITE_0 | DNS (EXT) | SITE (DR) | 1 -> 3 | gap 1-2 263; gap 2-3 262 |
| L_DNS_TR1_0 | DNS (EXT) | TR1 (PROD) | 1 -> 2 | gap 1-2 253; gap 1-2 436 |
| L_DRAPP_TG_0 | DRAPP (DR) | TG (EXT) | 3 -> 1 | gap 2-3 198; gap 1-2 201 |
| L_ARGO_TG_0 | ARGO (PROD) | TG (EXT) | 2 -> 1 | band 2 inside [PROD] 40; gap 1-2 476; gap 1-2 342 |
| L_PSPROD_TG_0 | PSPROD (STG) | TG (EXT) | 3 -> 1 | gap 2-3 224; gap 1-2 307 |
| L_LANE_OAI_0 | LANE (PROD) | OAI (EXT) | 2 -> 1 | band 2 inside [PROD] 144; band 2 inside [PROD] 67; gap 1-2 319 |
| L_LANE_AZ_0 | LANE (PROD) | AZ (EXT) | 2 -> 1 | band 2 inside [PROD] 51; gap 1-2 173; gap 1-2 499 |
| L_LANE_GITLAB_0 | LANE (PROD) | GITLAB (STG) | 2 -> 3 | band 2 inside [PROD] 22; gap 2-3 10; gap 2-3 271; band 3 inside [STG] 30 |
| L_ARGO_GITLAB_0 | ARGO (PROD) | GITLAB (STG) | 2 -> 3 | band 2 inside [PROD] 12; gap 2-3 60; gap 2-3 374; band 3 inside [STG] 27 |
| L_PG_DRAPP_0 | PG (PROD) | DRAPP (DR) | 2 -> 3 | gap 2-3 233; gap 2-3 544 |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_ARGO_TG_0 over title of [PROD]; L_DNS_SITE_0 over title of [DR].

Edge labels:

| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |
|---|---|---|---|---|---|---|
| L_U1_DNS_0 | 199x21 | gap 0-1 | 2 | 59 | 0.33 | - |
| L_T1_DNS_0 | 200x42 | gap 0-1 | 1 | 60 | 0.17 | - |
| L_U2_DNS_0 | 89x21 | gap 0-1 | 2 | 59 | 0.17 | - |
| L_R1_ARGO_0 | 128x21 | gap 1-2 | 0 | 245 | 0.39 | - |
| L_R2_STG_0 | 75x21 | band 2 beside blocks | 1 | 766 | 0.35 | - |
| L_R2_DR_0 | 141x21 | band 2 beside blocks | 1 | 749 | 0.29 | - |
| L_R2_PROD_0 | 160x21 | gap 1-2 | 2 | 227 | 0.36 | - |
| L_DNS_TR2_0 | 93x21 | band 2 beside blocks | 1 | 784 | 0.41 | - |
| L_DNS_PSPROD_0 | 104x21 | band 2 beside blocks | 1 | 882 | 0.40 | - |
| L_DNS_DRAPP_0 | 200x42 | band 2 beside blocks | 2 | 779 | 0.44 | - |
| L_DNS_PSDEV_0 | 123x21 | band 2 beside blocks | 2 | 958 | 0.53 | - |
| L_DNS_SITE_0 | 127x21 | band 2 beside blocks | 0 | 946 | 0.49 | - |
| L_DNS_TR1_0 | 101x21 | gap 1-2 | 0 | 472 | 0.39 | - |
| L_DRAPP_TG_0 | 90x21 | band 2 beside blocks | 1 | 880 | 0.51 | - |
| L_ARGO_TG_0 | 86x21 | gap 1-2 | 0 | 558 | 0.58 | - |
| L_PSPROD_TG_0 | 148x21 | band 2 beside blocks | 3 | 988 | 0.52 | - |
| L_LANE_OAI_0 | 46x21 | gap 1-2 | 1 | 533 | 0.56 | - |
| L_LANE_GITLAB_0 | 149x21 | gap 2-3 | 3 | 294 | 0.24 | - |
| L_ARGO_GITLAB_0 | 64x21 | gap 2-3 | 2 | 581 | 0.36 | - |
| L_PG_DRAPP_0 | 200x63 | gap 2-3 | 2 | 335 | 0.28 | - |

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=608: 5 labels; y=1072: 9 labels; y=1516: 3 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 14; overlapping something: 0.

Totals: canvas area 5.18 Mpx2, edge length 30945 px, bends 159, edge crossings (drawn polylines) 16.

Reading: four bands. The 21 inter-block edges put 15 runs into the gap between EXT/GH and PROD (11 lanes) and 12
into the gap between PROD and DR/STG (8 lanes). The only runs inside a block are the last or first bends of edges
that end or start in PROD and STG (lanes at 12 px pitch under the title), which is what inflates PROD's top padding
to 162 px. Fourteen labels sit more than 300 px of path from the nearer end; the row at y = 1072 holds nine of
them, 750 to 990 px from either end.

The same analog with the replication edge as in the real diagram (`u-mermaid/in/infra-overview-v2.mmd`:
`DRAPP ==> PG` instead of `PG --> DRAPP`), image `u-mermaid/renders/infra-overview-v2.elk.png`:

**infra-overview-v2.elk.svg  (canvas 1948 x 2755, flow axis y)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| EXT | 0 | - | 379 | 226 | 1009 | 201 | 7% | 5 (5 incl. nested) | 5 |
| GH | 0 | - | 1428 | 280 | 402 | 159 | 6% | 2 (2 incl. nested) | 2 |
| DR | 0 | - | 474 | 721 | 604 | 285 | 10% | 3 (3 incl. nested) | 3 |
| PROD | 0 | - | 943 | 1311 | 575 | 642 | 23% | 5 (5 incl. nested) | 3 / 1 / 1 |
| STG | 0 | - | 270 | 2114 | 472 | 633 | 23% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 74 | 66 | U1 , T1 , U2 | 152 |
| 1 | 226 | 439 | 213 | [EXT] , [GH] | 282 |
| 2 | 721 | 1006 | 285 | [DR] | 305 |
| 3 | 1311 | 1953 | 642 | [PROD] | 161 |
| 4 | 2114 | 2747 | 633 | [STG] | - |

Top-level blocks: 5; bands: 5; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 0: U1 -> T1 = 43; band 0: T1 -> U2 = 40; band 1: [EXT] -> [GH] = 40.

Sides of block frames crossed by inter-block edges: enters via bottom: 5, enters via top: 16, leaves via bottom: 13, leaves via top: 5.

Edges: 27 total, 6 with both ends in the same top-level block (intra) and 21 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 3 | 462 | 2 (176, 196) |
| band 1 beside blocks | 2 | 574 | 1 (433) |
| gap 1-2 | 15 | 4430 | 7 (445, 529, 549, 569, 589, 609, 691) |
| gap 2-3 | 7 | 1093 | 5 (1201, 1221, 1241, 1261, 1281) |
| gap 3-4 | 6 | 3626 | 4 (1983, 2044, 2064, 2084) |
| band 4 inside [STG] | 2 | 58 | 2 (2510, 2522) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_DNS_TR2_0 over title of [STG]; L_DNS_PSPROD_0 over title of [STG]; L_PSPROD_TG_0 over title of [STG]; L_LANE_GITLAB_0 over title of [STG]; L_ARGO_GITLAB_0 over title of [STG]; L_DNS_TR1_0 over title of [PROD]; L_LANE_OAI_0 over title of [PROD]; L_LANE_AZ_0 over title of [PROD]; L_DRAPP_PG_0 over title of [PROD]; L_DNS_SITE_0 over title of [DR].

Label rows (>= 3 labels centred on the same flow coordinate): y=136: 3 labels; y=648: 5 labels; y=896: 4 labels; y=1148: 5 labels.

Labels: 20; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 11; overlapping something: 0.

Totals: canvas area 5.37 Mpx2, edge length 29757 px, bends 163, edge crossings (drawn polylines) 13.

One edge turned round moves STANDBY above PROD and STG below it: five bands, servers one under another.

### 3.3 Corpus inputs, mermaid default (ELK): the five asked for and dense-td

Sources: `tree/examples/advanced.mmd`, `tree/examples/subgraph.mmd`, `tree/tests/fixtures/quality/fanout-frames.mmd`
(LR), `tree/examples/state.mmd` (composite state with `direction LR`), `tree/tests/fixtures/quality/bench-75.mmd`
(LR, five frames, 75 edges), and as a sixth `tree/tests/fixtures/quality/dense-td.mmd` (ten frames nested three
deep). Images: `u-mermaid/renders/<name>.elk.png`; the per-edge tables are in `u-mermaid/tables/<name>.elk.md`.

**advanced.elk.svg  (canvas 906 x 1484, flow axis y)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| CI | 0 | - | 194 | 8 | 449 | 452 | 30% | 4 (4 incl. nested) | 1 / 1 / 2 |
| Platform | 0 | - | 8 | 510 | 870 | 634 | 43% | 4 (7 incl. nested) | 1 / 3 / 2 / 1 |
| Workers | 1 | Platform | 32 | 691 | 414 | 262 | 18% | 3 (3 incl. nested) | 1 / 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 460 | 452 | [CI] | 50 |
| 1 | 510 | 1144 | 634 | [Platform] | 50 |
| 2 | 1194 | 1285 | 91 | Mon | 101 |
| 3 | 1386 | 1476 | 89 | Done , Rollback | - |

Top-level blocks: 2; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 3: Done -> Rollback = 274.

Sides of block frames crossed by inter-block edges: enters via top: 2, leaves via bottom: 3.

Edges: 20 total, 14 with both ends in the same top-level block (intra) and 6 between different top-level items (inter); 4 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 1 | 596 | 1 (490) |
| gap 1-2 | 2 | 30 | 1 (1174) |
| gap 2-3 | 2 | 542 | 2 (1305, 1366) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_Gate_Q_0 over title of [Workers].

Labels: 4; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 1.34 Mpx2, edge length 5108 px, bends 47, edge crossings (drawn polylines) 0.

**subgraph.elk.svg  (canvas 466 x 749, flow axis y)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| platform | 0 | - | 8 | 194 | 450 | 442 | 59% | 2 (5 incl. nested) | 1 / 1 / 1 / 2 |
| events | 1 | platform | 32 | 390 | 402 | 222 | 30% | 3 (3 incl. nested) | 1 / 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | Client | 40 |
| 1 | 93 | 144 | 51 | GW | 50 |
| 2 | 194 | 636 | 442 | [platform] | 60 |
| 3 | 696 | 741 | 45 | Store | - |

Top-level blocks: 1; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 2.

Edges: 8 total, 4 with both ends in the same top-level block (intra) and 4 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 2-3 | 2 | 158 | 1 (676) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_GW_Auth_0 over title of [platform]; L_Users_Bus_0 over title of [events].

Totals: canvas area 0.35 Mpx2, edge length 843 px, bends 14, edge crossings (drawn polylines) 0.

**fanout-frames.elk.svg  (canvas 648 x 2271, flow axis x)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| g1 | 0 | - | 240 | 8 | 400 | 547 | 62% | 5 (5 incl. nested) | 3 / 2 |
| g2 | 0 | - | 240 | 595 | 248 | 318 | 38% | 3 (3 incl. nested) | 3 |
| g3 | 0 | - | 240 | 953 | 376 | 400 | 58% | 6 (6 incl. nested) | 3 / 3 |
| g4 | 0 | - | 240 | 1393 | 200 | 270 | 31% | 2 (2 incl. nested) | 2 |
| g5 | 0 | - | 240 | 1703 | 224 | 138 | 35% | 1 (1 incl. nested) | 1 |
| g6 | 0 | - | 240 | 1881 | 200 | 297 | 31% | 3 (3 incl. nested) | 3 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 160 | 152 | n0 | 80 |
| 1 | 240 | 640 | 400 | [g1] , [g2] , [g3] , [g4] , [g5] , [g6] , n21 | - |

Top-level blocks: 6; bands: 2; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 1: [g1] -> [g2] = 40; band 1: [g2] -> [g3] = 40; band 1: [g3] -> [g4] = 40; band 1: [g4] -> [g5] = 40; band 1: [g5] -> [g6] = 40; band 1: [g6] -> n21 = 40.

Sides of block frames crossed by inter-block edges: enters via left: 6.

Edges: 7 total, 0 with both ends in the same top-level block (intra) and 7 between different top-level items (inter); 6 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 6 | 3753 | 3 (180, 200, 220) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 1.47 Mpx2, edge length 4253 px, bends 12, edge crossings (drawn polylines) 0.

**state.elk.svg  (canvas 649 x 886, flow axis y)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| Validating | 0 | - | 8 | 200 | 459 | 109 | 12% | 4 (4 incl. nested) | 4 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 22 | 14 | root_start | 40 |
| 1 | 62 | 99 | 37 | Idle | 101 |
| 2 | 200 | 309 | 109 | [Validating] | 40 |
| 3 | 349 | 386 | 37 | decide | 101 |
| 4 | 487 | 524 | 37 | forked , Rejected | 88 |
| 5 | 612 | 673 | 61 | Persist , Notify , Rejected----note | 64 |
| 6 | 737 | 747 | 10 | joined | 40 |
| 7 | 787 | 824 | 37 | Done | 40 |
| 8 | 864 | 878 | 14 | root_end | - |

Top-level blocks: 1; bands: 9; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Cross-axis gaps between neighbours in the same band: band 4: forked -> Rejected = 195; band 5: Persist -> Notify = 40; band 5: Notify -> Rejected----note = 100.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 1.

Edges: 16 total, 3 with both ends in the same top-level block (intra) and 13 between different top-level items (inter); 2 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 1-2 | 2 | 285 | 1 (119) |
| gap 3-4 | 1 | 266 | 2 (406, 467) |
| gap 4-5 | 2 | 137 | 1 (544) |
| gap 5-6 | 2 | 159 | 1 (717) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Labels: 5; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 0; overlapping something: 0.

Totals: canvas area 0.57 Mpx2, edge length 2138 px, bends 22, edge crossings (drawn polylines) 0.

**bench-75.elk.svg  (canvas 4327 x 1058, flow axis x)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| edge | 0 | - | 249 | 469 | 448 | 302 | 10% | 2 (2 incl. nested) | 1 / 1 |
| app | 0 | - | 1257 | 137 | 200 | 624 | 5% | 6 (6 incl. nested) | 6 |
| data | 0 | - | 2181 | 175 | 200 | 488 | 5% | 4 (4 incl. nested) | 4 |
| jobs | 0 | - | 2737 | 262 | 200 | 308 | 5% | 3 (3 incl. nested) | 3 |
| ops | 0 | - | 3361 | 396 | 696 | 281 | 16% | 4 (4 incl. nested) | 1 / 2 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 179 | 171 | user | 70 |
| 1 | 249 | 697 | 448 | [edge] | 560 |
| 2 | 1257 | 1457 | 200 | [app] | 724 |
| 3 | 2181 | 2381 | 200 | [data] | 356 |
| 4 | 2737 | 2937 | 200 | [jobs] | 424 |
| 5 | 3361 | 4057 | 696 | [ops] | 110 |
| 6 | 4167 | 4319 | 152 | ext | - |

Top-level blocks: 5; bands: 7; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via bottom: 2, enters via left: 52, enters via right: 8, leaves via bottom: 1, leaves via left: 8, leaves via right: 58.

Edges: 75 total, 5 with both ends in the same top-level block (intra) and 70 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 4 | 717 | 2 (199, 219) |
| band 1 inside [edge] | 7 | 298 | 4 (455, 467, 479, 491) |
| gap 1-2 | 15 | 5100 | 24 (703, 715, 727, 823, 843, 863, 883, 903, 923, 943, 963, 983, 1003, 1023 ...) |
| gap 2-3 | 35 | 7333 | 35 (1463, 1475, 1487, 1595, 1615, 1635, 1655, 1675, 1695, 1715, 1735, 1755, 1775, 1795 ...) |
| gap 3-4 | 9 | 1495 | 10 (2483, 2503, 2543, 2563, 2583, 2603, 2623, 2707, 2719, 2731) |
| gap 4-5 | 20 | 5258 | 15 (3027, 3047, 3067, 3087, 3107, 3127, 3147, 3167, 3187, 3207, 3227, 3247, 3331, 3343 ...) |
| band 5 inside [ops] | 9 | 735 | 6 (3567, 3579, 3591, 3603, 3627, 3851) |
| gap 5-6 | 7 | 2322 | 4 (4087, 4107, 4127, 4147) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: none.

Totals: canvas area 4.58 Mpx2, edge length 107188 px, bends 589, edge crossings (drawn polylines) 222.

**dense-td.elk.svg  (canvas 4131 x 4378, flow axis y)**

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| g1 | 0 | - | 8 | 166 | 2879 | 1349 | 31% | 7 (21 incl. nested) | 3 / 5 / 3 / 10 |
| g5 | 0 | - | 1302 | 1946 | 2236 | 1104 | 25% | 3 (14 incl. nested) | 1 / 1 / 8 / 4 |
| g8 | 0 | - | 1860 | 3405 | 1488 | 965 | 22% | 5 (9 incl. nested) | 3 / 1 / 2 / 1 / 1 / 1 |
| g4 | 1 | g1 | 32 | 224 | 1158 | 556 | 13% | 6 (6 incl. nested) | 3 / 3 |
| g2 | 1 | g1 | 560 | 1078 | 1310 | 403 | 9% | 3 (8 incl. nested) | 8 |
| g6 | 1 | g5 | 1945 | 2100 | 1569 | 926 | 21% | 7 (11 incl. nested) | 1 / 7 / 3 |
| g9 | 1 | g8 | 2691 | 3616 | 632 | 583 | 13% | 2 (4 incl. nested) | 2 / 1 / 1 |
| g3 | 2 | g2 | 786 | 1126 | 622 | 331 | 8% | 5 (5 incl. nested) | 2 / 3 |
| g7 | 2 | g6 | 2171 | 2263 | 620 | 469 | 11% | 4 (4 incl. nested) | 3 / 1 |
| g10 | 2 | g9 | 3037 | 3746 | 262 | 303 | 7% | 2 (2 incl. nested) | 1 / 1 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 116 | 108 | n46 | 50 |
| 1 | 166 | 1515 | 1349 | [g1] | 431 |
| 2 | 1946 | 3050 | 1104 | [g5] | 60 |
| 3 | 3110 | 3155 | 45 | n45 | 250 |
| 4 | 3405 | 4370 | 965 | [g8] | - |

Top-level blocks: 3; bands: 5; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via bottom: 4, enters via top: 29, leaves via bottom: 25, leaves via top: 4.

Edges: 47 total, 14 with both ends in the same top-level block (intra) and 33 between different top-level items (inter); 3 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|
| gap 0-1 | 1 | 3251 | 1 (136) |
| band 1 inside [g1] | 14 | 9945 | 15 (471, 495, 507, 519, 531, 543, 555, 567, 660, 820, 832, 844, 856, 868 ...) |
| gap 1-2 | 23 | 27826 | 17 (1555, 1575, 1595, 1615, 1635, 1655, 1675, 1695, 1715, 1735, 1755, 1816, 1836, 1856 ...) |
| band 2 inside [g5] | 8 | 5352 | 11 (2024, 2036, 2048, 2060, 2199, 2211, 2223, 2362, 2530, 2762, 2843) |
| gap 2-3 | 5 | 630 | 1 (3090) |
| gap 3-4 | 22 | 21551 | 11 (3175, 3195, 3215, 3235, 3255, 3275, 3295, 3315, 3335, 3355, 3375) |
| band 4 inside [g8] | 14 | 2238 | 13 (3504, 3516, 3528, 3540, 3552, 3564, 3576, 3694, 3706, 3845, 3950, 4079, 4229) |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_n31_n38_0 over title of [g8]; L_n25_n38_0 over title of [g8]; L_n16_n29_0 over title of [g6]; L_n16_n43_0 over title of [g10]; L_n16_n44_0 over title of [g10]; L_n43_n16_0 over title of [g10]; L_n4_n43_0 over title of [g10]; L_n33_n16_0 over title of [g7].

Label rows (>= 3 labels centred on the same flow coordinate): y=1784: 5 labels; y=2504: 8 labels; y=3132: 3 labels.

Labels: 21; more than 12 px off their own line: 0; more than 300 px of path from the nearer end: 14; overlapping something: 0.

Totals: canvas area 18.09 Mpx2, edge length 142591 px, bends 416, edge crossings (drawn polylines) 68.

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

Canvas area in Mpx2 next to flowmaid 0.38.0 (`b12/fm038`). Node sizes differ (mermaid wraps node text at 120 px
and uses a 120 px minimum width), so this is a rough comparison:

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

Frame padding of every block (ELK renders). Left, right and bottom are 24 everywhere except STG, whose frame is
widened to its 464 px title. Top is 48 for a one-line title and 90 for a three-line one; larger values are lanes
that ELK placed inside the frame above the first row:

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
