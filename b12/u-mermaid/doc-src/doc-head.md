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
