# Roadmap

Where flowmaid is going and how far it has come. Two things are tracked, in two places:

- **Syntax coverage**: which mermaid diagram types and statements are parsed and drawn. The board is [issue #10](https://github.com/go-routine-id/flowmaid/issues/10).
- **Drawing quality** and the work around it: this page.

The bar for drawing quality: the same source should look as good as it does in mermaid 12.2.0, or better and tidier. flowmaid keeps what mermaid does not offer: zero dependencies, an interactive scene API (drag, re-route, picking), and speed (most hand-written diagrams render in under a millisecond).

Last updated for 0.40.0 (2026-11-11).

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
| 1 | Structure: the tree of blocks, layering, ordering | a first version is on a work branch; its review did not pass (see the open question below), so it is not merged or released |
| 2 | Geometry: frames, coordinates, room for gaps and labels | not started |
| 3 | Routing under a drag: `route()`, `route_sized()` and `route_partial()` in block mode | not started |
| 4 | Comparison sweep against mermaid and against the previous release, then on by default | not started |

**Open question.** One dense test diagram draws with more than twice as many crossings in block mode as it has today. Either the design is extended to cover it, or a diagram that the block layout would draw worse keeps the current layout.

## After that

Not scheduled. Listed so that they are not lost.

- **Class diagrams with many self-relations.** With three or more loops on one class, loop ends can hang past the class's corners and a label can land on a loop. The 0.40.0 changelog has the numbers.
- **Subgraphs laid out in their own `direction`.** Three small points are open: such a subgraph is placed after a sibling node that comes before its first member in the source, where mermaid puts the subgraph first; it can make a sibling subgraph in its layer wider; and a state composite that sets no `direction` keeps the direction round it, where mermaid lays it out top-down (see the 0.40.0 changelog). They are re-checked once the block layout is in, because it reworks that code.
- **Large graphs.** A dense graph of a thousand nodes takes seconds to draw (see [Large graphs](../README.md#large-graphs)). Making such graphs faster is on this list, with no date.
- **Interactive API**:
  - [#110](https://github.com/go-routine-id/flowmaid/issues/110) a `route` for class and ER diagrams that does not panic when the number of positions does not match the diagram
  - [#111](https://github.com/go-routine-id/flowmaid/issues/111) `route_partial` for ER, class and mindmap
  - [#112](https://github.com/go-routine-id/flowmaid/issues/112) hit-testing for mindmap, sequence, pie, journey, timeline and sankey scenes
  - [#113](https://github.com/go-routine-id/flowmaid/issues/113) `route_partial` and the compact fold for hosts that measure text with their own font
- **Math in labels**: [#12](https://github.com/go-routine-id/flowmaid/issues/12).
- **Other open issues**:
  - [#57](https://github.com/go-routine-id/flowmaid/issues/57) id and class hooks on SVG nodes and edges, for interactivity and CSS theming
  - [#64](https://github.com/go-routine-id/flowmaid/issues/64) refactoring: split the long layout functions, name the magic numbers
  - [#75](https://github.com/go-routine-id/flowmaid/issues/75) layout follow-ups from 0.32: static render time, drag budget, crossings, frame and label clearance
  - [#76](https://github.com/go-routine-id/flowmaid/issues/76) `route()`: a top-left overflow shifts the whole scene

## How a step gets in

Every step gets an independent review before it is merged. The review runs:

- the full test gate: formatting, lints, tests, doctests, the minimum supported Rust version, wasm and rustdoc;
- a byte comparison of the diagram types the step must not change;
- a measured comparison against the previous release and against mermaid 12.2.0: crossings, lines through boxes, labels under lines, and timings.

What a step costs is written in the changelog next to what it gains.
