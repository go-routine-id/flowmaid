# Roadmap

Where flowmaid is going and how far it has come. Two things are tracked, in two places:

- **Syntax coverage**: which mermaid diagram types and statements are parsed and drawn. The board is [issue #10](https://github.com/go-routine-id/flowmaid/issues/10).
- **Drawing quality** and the work around it: this page.

The bar for drawing quality: the same source should look as good as it does in mermaid 12.2.0, or better and tidier. flowmaid keeps what mermaid does not offer: zero dependencies, an interactive scene API (drag, re-route, picking), and speed (most hand-written diagrams render in under a millisecond).

Last updated 2026-11-23 (latest release: 0.40.2).

## Done

| release | what it brought |
|---|---|
| 0.34.0 | Flowcharts: one arrowhead size, back-edges in orthogonal lanes, nested subgraphs and canvas fixes, room for labels and ports, edge ends on every shape's real outline, four of mermaid's themes. Class and state diagrams reworked. |
| 0.35.0 | Layering (the row along the flow each node is placed on): subgraphs fed by a hub, narrowing of broad flowcharts, drawings that stay stable when source lines move. |
| 0.36.0 | Layering follow-up. Journey and timeline text wrap, gitGraph merges, ER labels, pies past sixteen slices. |
| 0.37.0 | `direction` inside a subgraph or a state composite. |
| 0.38.0 | Flowchart and state edges run on the axes, with rounded corners. |
| 0.39.0 | Crossing lines hop over each other. A hub's fan is drawn as a comb. |
| 0.40.0 | Class diagrams: dotted names, a fan of six without crossings, self-relations kept off a neighbour. State diagrams: a drag inside a composite no longer resizes a fork or join bar. |
| 0.40.1 | Interactive API: `try_route`, a `route` that turns down positions that do not fit the diagram. A drag past the top-left margin no longer shifts the rest of the drawing. Class diagrams: a declared name with a stray dot is an error, as in mermaid. The first step of subgraphs as blocks (below) is in, switched off. |
| 0.40.2 | Interactive API: a drag in an ER, class or mindmap diagram redraws only what is attached to a moved box (`route_partial`), and mindmap, sequence, pie, journey, timeline and sankey scenes can be picked (`hit_test`). The first part of step 2 of subgraphs as blocks is in, switched off. |

[CHANGELOG.md](../CHANGELOG.md) has the details and the measurements for each release.

## In progress: subgraphs as blocks

A new layout for flowcharts and state diagrams in which a link joins a subgraph, or something inside it, to something outside it. Diagrams without subgraphs are not touched, and diagrams whose subgraphs are all isolated keep today's layout.

What it aims for:

- **One block per subgraph.** Every non-empty subgraph or state composite is one compact block: its members take consecutive rows along the flow, and the frame is as large as its content.
- **Linked blocks are stacked.** Two blocks joined by an edge are stacked along the flow, not set side by side. Blocks with no edge between them may sit side by side.
- **Edges run between blocks.** An edge between blocks runs in the gaps between them. It crosses a frame only through its upstream or downstream side (top or bottom in a top-down chart), and never runs through a block that holds neither of its ends.
- **Labels have room.** A label sits on the part of its edge outside the end blocks, with room reserved, clear of other lines.

It is built in four steps. Each is merged behind an internal switch that is off, so no drawing changes until the last step turns the new layout on.

| step | what | state |
|---|---|---|
| 1 | Structure: the tree of blocks, layering, ordering | merged behind the switch, which is off: no drawing changes |
| 2 | Geometry: frames, coordinates, room for gaps and labels | first part merged behind the switch, which is off: frames sized to their titles, room for labels in the gaps between blocks, edge ends kept clear of frame titles. No drawing changes. The second part, frames drawn tighter round their content and chains kept straight across levels, is not done: two rounds of work did not reach the bar, which is mermaid 12.2.0 on the same diagrams. It gets a design study first (see Next) |
| 3 | Routing under a drag: `route()`, `route_sized()` and `route_partial()` in block mode | done on the work branch, behind the switch, not yet merged: `route()` draws what the static layout draws. A drag redraws only what it moves, and an edge that would gain a crossing by following a moved box is redrawn instead. The drag time budgets hold. Corridors between the blocks for the edges a drag redraws, and faster drags on dense diagrams, move to step 4 |
| 4 | Corridors for redrawn edges, comparison sweep against mermaid and against the previous release, then on by default | not started; target: on by default in 0.42.0 |

**The dense test diagram.** It used to draw more than twice as many crossings in block mode as it has today. After step 1 it drew fewer: 49 crossings against 66 in today's layout (`tests/fixtures/quality/dense-td.mmd`, the static drawing), and after the first part of step 2, 42. Redrawn by `route()`, which a drag uses, it still has more, 107 against 76, until step 3. With step 3 on the work branch, `route()` draws the static drawing: 42 crossings both ways, against 66 static and 76 redrawn in today's layout.

## Next

Work on subgraphs as blocks is paused (2026-11-19). When it resumes, in this order:

1. **A design study for tighter blocks.** Several ways to draw the frames tighter round their content and keep chains straight are prototyped side by side and compared, picture by picture, with mermaid 12.2.0 on the same diagrams. One is chosen before it is built into the layout.
2. **Steps 2 (second part) and 3 into dev,** behind the switch, once the review passes.
3. **Step 4,** then the new layout on by default in a release.

**Target.** The block layout ships as an opt-in in **0.41.0**, with today's layout kept as the default, and becomes the default in **0.42.0**. The targets are releases, not dates: the design study decides how long the tighter blocks take.

## After that

Not scheduled. Listed so that they are not lost.

- **Class diagrams with many self-relations.** With three or more loops on one class, loop ends can hang past the class's corners and a label can land on a loop. The 0.40.0 changelog has the numbers.
- **Subgraphs laid out in their own `direction`.** Three small points are open: such a subgraph is placed after a sibling node that comes before its first member in the source, where mermaid puts the subgraph first; it can make a sibling subgraph in its layer wider; and a state composite that sets no `direction` keeps the direction round it, where mermaid lays it out top-down (see the 0.40.0 changelog). They are re-checked once the block layout is in, because it reworks that code.
- **Large graphs.** A dense graph of a thousand nodes takes seconds to draw (see [Large graphs](../README.md#large-graphs)). Making such graphs faster is on this list, with no date.
- **Interactive API**:
  - [#113](https://github.com/go-routine-id/flowmaid/issues/113) `route_partial` and the compact fold for hosts that measure text with their own font
- **Math in labels**: [#12](https://github.com/go-routine-id/flowmaid/issues/12).
- **Other open issues**:
  - [#57](https://github.com/go-routine-id/flowmaid/issues/57) id and class hooks on SVG nodes and edges, for interactivity and CSS theming
  - [#64](https://github.com/go-routine-id/flowmaid/issues/64) refactoring: split the long layout functions, name the magic numbers
  - [#75](https://github.com/go-routine-id/flowmaid/issues/75) layout follow-ups from 0.32: static render time, drag budget, crossings, frame and label clearance

## How a step gets in

Every step gets an independent review before it is merged. The review runs:

- the full test gate: formatting, lints, tests, doctests, the minimum supported Rust version, wasm and rustdoc;
- a byte comparison of the diagram types the step must not change;
- a measured comparison against the previous release and against mermaid 12.2.0: crossings, lines through boxes, labels under lines, and timings.

What a step costs is written in the changelog next to what it gains.
