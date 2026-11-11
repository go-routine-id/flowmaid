# flowmaid

[![CI](https://github.com/go-routine-id/flowmaid/actions/workflows/ci.yml/badge.svg)](https://github.com/go-routine-id/flowmaid/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/flowmaid.svg)](https://crates.io/crates/flowmaid)
[![docs.rs](https://docs.rs/flowmaid/badge.svg)](https://docs.rs/flowmaid)
[![license](https://img.shields.io/crates/l/flowmaid.svg)](LICENSE)

A small Mermaid-like diagram engine written in pure std Rust with zero external dependencies. It takes Mermaid-syntax text and produces SVG — or live, draggable geometry for interactive apps. Twelve diagram types today: flowcharts, state, UML class, entity-relationship, sequence, pie, user-journey, git graphs, mindmap, timeline, sankey, and architecture-beta diagrams.

**Website:** https://go-routine-id.github.io/flowmaid/ · **Playground:** https://go-routine-id.github.io/flowmaid-web/ · **Desktop editor:** [flowmaid-desktop](https://github.com/go-routine-id/flowmaid-desktop) · **Terminal:** [flowcli](https://github.com/go-routine-id/flowcli) · Crate :  [Flowmaid Crate](https://crates.io/crates/flowmaid)

**Why flowmaid?**

- **Zero dependencies**, pure std Rust (MSRV 1.75), and `wasm32` out of the box. All twelve diagram types fit in a compact wasm bundle, where mermaid.js 12.2.0 is about 5.5 MB minified (1.6 MB gzipped).
- **Fast:** most hand-written diagrams render in under a millisecond, about **113× faster than mermaid** (see [Performance](#performance)).
- **One geometry source** for SVG export and interactive canvases: drag a node and only its edges re-route, pick what is under the pointer, save the graph back as mermaid text (see [Interactive API](#interactive-api)).
- **Deterministic, accessible SVG:** byte-identical output for the same input, with `role="img"` and an accessible name on every diagram, and per-item `<title>`s in flowchart, state, journey and sankey diagrams.
- **Clear errors:** parse errors carry line numbers, and the unsupported Mermaid headers flowmaid recognises fail with an explicit "not supported yet" message (see [Diagram types](#diagram-types)). UTF-8 BOMs are stripped and CRLF is fine.

## Installation

As a library:

```bash
cargo add flowmaid
```

Or in `Cargo.toml`:

```toml
[dependencies]
flowmaid = "0.39"
```

The crate is lib + bin; `cargo install flowmaid` installs the `flowmaid` command.

Then render a diagram from the command line or from code (more under [Usage](#usage)):

```bash
flowmaid diagram.mmd -o diagram.svg
```

```rust
let svg = flowmaid::render_svg("flowchart TD\nA[Start] --> B[Done]")?;
```

## Contents

- [Installation](#installation) · [flowmaid next to mermaid](#flowmaid-next-to-mermaid) · [Usage](#usage) · [Themes](#themes)
- [Diagram types](#diagram-types) · [Differences from mermaid](#differences-from-mermaid) · [Accessibility](#accessibility)
  - [Flowcharts](#flowcharts) · [State diagrams](#state-diagrams) · [Class diagrams](#class-diagrams) · [Entity-Relationship diagrams](#entity-relationship-diagrams) · [Sequence diagrams](#sequence-diagrams)
  - [Pie charts](#pie-charts) · [User journeys](#user-journeys) · [Git graphs](#git-graphs) · [Mindmaps](#mindmaps) · [Timeline diagrams](#timeline-diagrams) · [Sankey diagrams](#sankey-diagrams) · [Architecture diagrams](#architecture-diagrams)
  - [Advance swimlane diagrams](#advance-swimlane-diagrams-json-and-text-input) (engine-native, no mermaid header)
- [Interactive API](#interactive-api) · [How it works](#how-it-works) · [Performance](#performance)
- [Mermaid parity roadmap](#mermaid-parity-roadmap) · [Limitations](#limitations) · [License](#license)

## flowmaid next to mermaid

The same `.mmd` source rendered by mermaid 12.2.0 (left, in headless Chromium) and by flowmaid 0.39.0 (right, `flowmaid file.mmd`). flowmaid keeps its own semantic palette by default; mermaid's look is one `--theme default` away (see [Themes](#themes)).

**Flowchart with subgraphs, a decision and a back-edge**

![advanced flowchart: mermaid 12.2.0 vs flowmaid 0.39.0](docs/img/compare/advanced.png)

**Loop back to an earlier step**

![back-edge flowchart: mermaid vs flowmaid](docs/img/compare/back-edge.png)

**UML class diagram**

![class diagram: mermaid vs flowmaid](docs/img/compare/class.png)

**State diagram with a composite, a choice, fork/join and a note**

![state diagram: mermaid vs flowmaid](docs/img/compare/state.png)

<details>
<summary>More: basic flowchart, nested subgraphs, ER and sequence diagrams</summary>

![demo flowchart: mermaid vs flowmaid](docs/img/compare/demo.png)

![subgraph flowchart: mermaid vs flowmaid](docs/img/compare/subgraph.png)

![ER diagram: mermaid vs flowmaid](docs/img/compare/er.png)

![sequence diagram: mermaid vs flowmaid](docs/img/compare/sequence.png)

</details>

Rendering is about **113× faster than mermaid** (geometric mean over 25 diagrams) and the SVG is a fraction of the size — see [Performance](#performance).

## Usage

### Command line

```bash
flowmaid examples/demo.mmd -o demo.svg     # from a file
cat examples/lr.mmd | flowmaid > lr.svg    # or through a pipe
```

| option | effect |
|---|---|
| `-o <file>` | Write the SVG to a file. Without it the SVG goes to stdout. |
| `--compact <px>` | Fold a long linear chain to fit that many px along the flow axis, e.g. `--compact 600`. A diagram that is not one linear chain is drawn as usual. |
| `--transparent` | Drop the canvas background so the SVG sits on any page colour. Combines with `--compact`, `--advance` and `--advance-text`. |
| `--theme <name>` | `flowmaid` (the default), `default`, `neutral`, `dark` or `forest`. Beats a theme the source names. See [Themes](#themes). |
| `--advance <file.json>` | Render a [swimlane diagram](#advance-swimlane-diagrams-json-and-text-input) from JSON. |
| `--advance-text <file>` | Render a swimlane diagram from the text DSL. |

`flowmaid --help` lists every option. From a checkout, `cargo run -- examples/demo.mmd -o demo.svg` runs the CLI and `cargo test` runs the tests.

### Library

One call renders any supported diagram type, dispatched on the header:

```rust
let flow = flowmaid::render_svg("flowchart TD\nA[Start] --> B[Done]")?;
let uml  = flowmaid::render_svg("classDiagram\nAnimal <|-- Dog")?;
let fsm  = flowmaid::render_svg("stateDiagram-v2\n[*] --> Idle")?;
```

Options go through `SvgOptions` and `render_svg_advanced`; each diagram module also has a `*_with` writer. `SvgOptions` is `#[non_exhaustive]`: start from `Default` and set the fields you need, which keeps compiling as options are added.

```rust
let mut opts = flowmaid::SvgOptions::default();
opts.transparent = true; // no white canvas rect — same as CLI --transparent
opts.responsive = true;  // width="100%" instead of a fixed pixel size
let svg = flowmaid::render_svg_advanced("flowchart TD\nA --> B", &opts)?;
```

Transparent output keeps the theme's text and edge colours: on a dark page, pick the dark theme too.

For draggable, pickable geometry instead of an SVG string, see [Interactive API](#interactive-api).

### Themes

flowmaid's own palette is the default and does not change: nodes are coloured by shape, and class, ER and sequence items take an accent each. Mermaid's looks are options, set from code, the CLI or the diagram itself:

| theme | `Theme::` | look |
|---|---|---|
| `flowmaid` (default) | `Flowmaid` | semantic palette: stadium green, diamond amber, …; an accent per class, entity and participant |
| `default` | `Mermaid` | mermaid's default: lavender `#ececff` nodes with a `#9370db` border, `#333333` lines and text, pale yellow frames |
| `neutral` | `Neutral` | white nodes, one `#333333` ink for borders, lines and text, grey frames |
| `dark` | `Dark` | light text, lines and borders on a near-black `#1e1f22` canvas |
| `forest` | `Forest` | green `#cde498` nodes with a deep green border, black lines |

```rust
use flowmaid::style::Theme;
let mut opts = flowmaid::SvgOptions::default();
opts.theme = Some(Theme::Dark);
let svg = flowmaid::render_svg_advanced("flowchart TD\nA --> B", &opts)?;
```

- **From the diagram.** `render_svg` and `render_svg_advanced` honour a theme the source names, in an `init` directive or under the frontmatter's `config:`, the way mermaid reads it.
- **Theme variables.** Only `background`, `primaryColor` (node fill), `primaryBorderColor`, `primaryTextColor`, `lineColor` and `textColor` are applied. Other `themeVariables` are ignored.
- **Names.** `base` reads as `default`, `neo` as `neutral`, `neo-dark` as `dark`. An unknown name falls back to the `flowmaid` theme without an error; the CLI's `--theme` prints a warning.
- **Precedence.** An explicit `SvgOptions::theme` (or `--theme`) beats the source's theme name and variables, so `Some(Theme::Flowmaid)` renders a themed file in flowmaid's colours. If you parse first and render through a `render::*_with` writer, call `opts.with_source_theme(source)` to honour the source's theme as the CLI does.
- **Coverage.** Every theme applies to all twelve diagram types. The charts (pie, journey, timeline, git graph, sankey, mindmap, architecture) keep their categorical colours, while their text, lines and canvas follow the theme.
- **Contrast.** The mermaid themes keep text at 4.5:1 and lines and borders at 3:1 on what they are drawn over. Two exceptions: the faint hairlines round label chips and between ER rows, and the amber accent of git lanes and mindmap branches on a white canvas (2.6:1).
- **Your colours win.** `style`, `classDef` and `linkStyle` colours always win over the theme. Under the mermaid themes, a `style` or `classDef` fill without a `color` gets text that reads on it. The default `flowmaid` theme keeps its ink, so give a dark fill its own `color` there.
- **The `flowmaid` palette** lives in the `style` module. ER entities cycle through a stable accent palette, and text on accent fills is picked for WCAG AA contrast (`style::text_on`).
- **Hosts.** Hosts painting a `Scene` get the exact colours from `opts.palette()` (`style::Palette`). Marker ids carry a non-default edge colour, so diagrams of different themes can share a page.

## Diagram types

| diagram | header | syntax (mermaid's page) | examples | interactive (`route()`) |
|---|---|---|---|---|
| [Flowchart](#flowcharts) | `flowchart` / `graph` | [flowchart](https://mermaid.js.org/syntax/flowchart.html) | `demo.mmd`, `lr.mmd`, `advanced.mmd`, `subgraph.mmd`, `shapes.mmd`, `fanout.mmd`, `long-labels.mmd`, `stress.mmd` | yes |
| [State](#state-diagrams) | `stateDiagram-v2` / `stateDiagram` | [stateDiagram](https://mermaid.js.org/syntax/stateDiagram.html) | `state.mmd` | yes (flowchart API) |
| [Class](#class-diagrams) | `classDiagram` | [classDiagram](https://mermaid.js.org/syntax/classDiagram.html) | `class.mmd` | yes |
| [Entity-Relationship](#entity-relationship-diagrams) | `erDiagram` | [entityRelationshipDiagram](https://mermaid.js.org/syntax/entityRelationshipDiagram.html) | `er.mmd` | yes |
| [Sequence](#sequence-diagrams) | `sequenceDiagram` | [sequenceDiagram](https://mermaid.js.org/syntax/sequenceDiagram.html) | `sequence.mmd` | static |
| [Pie](#pie-charts) | `pie` | [pie](https://mermaid.js.org/syntax/pie.html) | `pie.mmd` | static |
| [User journey](#user-journeys) | `journey` | [userJourney](https://mermaid.js.org/syntax/userJourney.html) | `journey.mmd` | static |
| [Git graph](#git-graphs) | `gitGraph` | [gitgraph](https://mermaid.js.org/syntax/gitgraph.html) | `gitgraph.mmd` | static |
| [Mindmap](#mindmaps) | `mindmap` | [mindmap](https://mermaid.js.org/syntax/mindmap.html) | `mindmap.mmd` | yes |
| [Timeline](#timeline-diagrams) | `timeline` | [timeline](https://mermaid.js.org/syntax/timeline.html) | `timeline.mmd` | static |
| [Sankey](#sankey-diagrams) | `sankey-beta` / `sankey` | [sankey](https://mermaid.js.org/syntax/sankey.html) | `sankey.mmd` | static |
| [Architecture](#architecture-diagrams) | `architecture-beta` | [architecture](https://mermaid.js.org/syntax/architecture.html) | `architecture.mmd` | static |

Every example lives in `examples/`. The sections below do not repeat mermaid's syntax: follow the link for that. Each section names the part of that syntax flowmaid supports and what flowmaid adds. A **Not yet** line lists the gaps known so far; syntax a section does not name may not be supported.

Other Mermaid headers:

| header | result |
|---|---|
| `gantt`, `quadrantChart`, `requirementDiagram`, `xychart-beta`, `block-beta`, `packet`, `kanban`, `radar-beta`, `treemap`, `zenuml`, `C4Context` … `C4Deployment`, `swimlane-beta` | Detected: an explicit "not supported yet" error. |
| `info`, `ishikawa`, `eventmodeling` and the short forms `xychart`, `block` and `requirement` | Newer mermaid 12.2 headers, not recognised yet: read as flowchart text. |
| `venn-beta`, `treeView-beta`, `usecase-beta`, `wardley-beta`, `cynefin-beta`, `ishikawa-beta`, `agentflow-beta`, and `railroad-beta` with its `railroad-ebnf-beta`, `railroad-abnf-beta` and `railroad-peg-beta` forms | Newer `-beta` previews, not recognised yet: a generic "unknown edge operator" parse error. |

### Differences from mermaid

Where the same source gives a different result:

| what | mermaid | flowmaid |
|---|---|---|
| Default look | its `default` theme | the `flowmaid` semantic palette; mermaid's look is one `--theme default` away |
| Unknown theme name | falls back to its own default | falls back to the `flowmaid` theme, without an error |
| Unclosed `accDescr {` | swallows the rest of the file | a line-numbered error |
| `flowchart-elk` header | asks for the ELK layout engine | treated as plain `flowchart`: flowmaid has a single layout |
| Node label wrap width, flowcharts and state diagrams | 120 in mermaid 12.2 | 200, mermaid 11's default; `wrappingWidth` sets it |
| Flowchart edge with no `curve:` | drawn as `basis`, the default curve | runs on the axes with rounded corners, as mermaid 12.2's ELK renderer draws it |
| Subgraph with a `direction` whose members have no links among them | laid out in its own direction, where unlinked nodes share one rank: a column for `direction LR` | not laid out on its own: the members lie side by side across the flow round it |
| `linkStyle default` and an index style both set `stroke` | the markers take the default's colour | the markers follow the colour the line is drawn in |
| Circle marker of a coloured link | only its outline is coloured | filled as well as outlined in the edge colour |
| `click` url that is `javascript:`, `data:` or `vbscript:` | written as `about:blank` | not linked at all |
| An empty markdown label (``` "``" ```), or extra spaces in a `click` (`click A  href  "u"`) | rejected | accepted |
| State diagram `note "text" as N` | dropped | drawn as a free-standing card |
| State composite with no `direction` of its own | laid out top-down whichever way the diagram runs, unless one of its states links out of it or a note is attached to it or inside it | keeps the direction round it; write `direction TB` in the composite for mermaid's column |
| State composite with a `direction` and a note inside | the `direction` is dropped | the `direction` holds |
| Class name part that is just `o`, as in `o.A --> B` or `x.o--o.y` | read as an aggregation glyph, and the line fails | part of the name |
| Trailing `%%` comment in class and sequence diagrams | printed as part of the label or message text | stripped |
| ER alias, as in `p[Person]` | accepted only where an entity is declared | also accepted inside a relationship |
| Sequence frames | sized to the lifelines they cover | full-width bands by default; `scopedFrames: true` gives mermaid's sizing |
| Cyclic sankey input | rejected with d3-sankey's "circular link" error | drawn: the links that close a loop run back leftwards |

### Accessibility

- Every SVG carries `role="img"` and an accessible name (`<title>` and `aria-label`). Flowchart, state, journey and sankey diagrams also have a `<title>` per item.
- `accTitle: text` replaces the accessible name. A flowchart's default name is "Flowchart diagram".
- `accDescr: text`, or a multi-line `accDescr { … }` block, adds a `<desc>` that the root `aria-describedby` points at. Its id is a content hash, so inlined SVGs never collide.
- Both work in flowchart, state, class, sequence, ER, pie, journey, timeline and architecture diagrams. A gitGraph parses them but does not draw them yet.

## Flowcharts

Flowchart syntax follows mermaid **12.2.0**: see [mermaid's flowchart page](https://mermaid.js.org/syntax/flowchart.html). Where mermaid 12 changed a default, the difference is noted next to the feature. Supported:

- **Basics:** the `flowchart` and `graph` headers in all four directions, `%%` comments, and `;` between statements. `flowchart-elk` is accepted too, as plain `flowchart`.
- **Nodes:** the bracket shapes, and mermaid v11 metadata such as `A@{ shape: cyl, label: "DB" }`.
- **Labels:** quoted labels, entity codes, `<br/>` line breaks and markdown strings.
- **Edges:** arrow, open, dotted, thick and invisible `~~~` links; cross, circle and two-way ends; both label spellings; `&` fan-out, chains, cycles and self-loops; edge ids with `animate` and `curve`.
- **Subgraphs:** nesting, titles, a per-block `direction`, and edges to or from a subgraph itself.
- **Styling:** `style`, `classDef` (with `classDef default`), `class`, `:::` and `linkStyle`.
- **Other:** `click` links, and `accTitle` / `accDescr` (see [Accessibility](#accessibility)).
- **Not yet:** MathML for `$$…$$` math, and escaped quotes (`\"`). Some shapes are drawn as look-alikes and some metadata keys are not drawn (see [Node shapes](#node-shapes)). `click` callbacks are accepted and ignored.

### Node shapes

| drawn as | bracket shapes | `@{ shape: … }` names |
|---|---|---|
| the exact shape | rectangle, rounded, stadium, diamond, circle, double circle, cylinder, subroutine, hexagon, both parallelograms | `rect`, `rounded`, `stadium`, `fr-rect`, `cyl`, `circle`, `dbl-circ`, `diam`, `hex`, `lean-r`, `lean-l`, `sm-circ`, `fr-circ`, `fork`, `f-circ` and their aliases |
| the nearest look-alike | both trapezoids and the asymmetric shape (`A>text]`) | names flowmaid has no geometry for yet: `doc`, `trap-b`, `cloud`, `delay`, `tri`, … |
| no box | | `text`, the brace shapes, `bolt` |

- Every name in mermaid's shape registry is accepted. An unknown name, or one with uppercase or `_`, fails with mermaid's own "No such shape" message.
- As in mermaid 12, a metadata `label` is markdown, unless `labelType: string` or `labelType: text` keeps it plain.
- The metadata keys `icon`, `img`, `form`, `pos`, `w`, `h` and `constraint` are accepted but not drawn. An icon or image node keeps its id as its label.

### Labels, wrapping and markdown

- **Wrap width.** A node label that breaks none of its own lines wraps at 200px, in flowcharts and state diagrams alike (mermaid 12.2 wraps at 120). Set it with `flowchart.wrappingWidth`, in an `init` directive or the frontmatter's `config:`.
- **Where lines break.** At spaces and between CJK characters, evened out so no word is left alone on the last line. A word is never split: one wider than the width keeps its line. `<b>` / `<i>` markup wraps too.
- **Diamonds and circles** wrap at the width / 1.6, so a long question wraps into a diamond no flatter than 1.6:1.
- **`<br/>`.** A label with a `<br/>` keeps exactly the lines it was written with. An edge label stays on one line, each `<br/>` drawn as a space.
- **Markdown strings** take bold and italic. Other markdown (headings, lists, links, code, strikethrough) shows as written. Markdown node labels wrap the same way; edge labels and subgraph titles are styled but stay on one line. A raw backtick or `"` inside the string is a line-numbered error: write `#96;` / `#quot;`.
- **Entity codes.** A code that would make a control character (`#0;`, `#27;`) is not decoded and stays as written; a tab (`#9;`) and a line break (`#10;`) are the exceptions.
- **Math.** `$$…$$` in labels is tokenized: italic TeX passthrough with correct widths (MathML is on the [roadmap](#mermaid-parity-roadmap)).

### Edges

- **On the axes.** Flowchart and state edges run on the axes with rounded corners, as mermaid 12.2's ELK renderer draws them. A straight edge stays straight. An edge with an explicit `curve:` or `linkStyle … interpolate` keeps that curve, as do self-loops and a state note card's link.
- **Fans.** The edges leaving or entering one side of a box are drawn as a comb. Each keeps its own port and its own track, so they nest and do not cross each other under their box. A comb cannot undo every crossing: an edge led out one way and back under its fan, one that jogs sideways before its track, or two edges whose far ends lie the other way round can still cross.
- **Crossings.** Where a horizontal stretch of one edge crosses a vertical stretch of another, the horizontal one steps over on a small semicircle, always upwards, so both lines stay whole. Hops close together join into one bridge.
- **Gaps.** Where a hop has no room, or on an edge with its own `curve:`, one of the two strokes gets a small gap instead.
- **Limit.** A drawing of more than 400 edges is past the router's cap: its edges are drawn as curves and its crossings plain. The count is taken per laid-out part: each subgraph laid out on its own in its `direction` (see [Subgraphs](#subgraphs)), and the rest of the drawing round it.
- **Edge ids.** An id (`A e1@--> B`) is written to the SVG as `id="flowmaid-<hash>-e1" data-id="e1"`: mermaid's `<diagram-id>-<edge-id>`. The diagram id is a deterministic content hash, so several SVGs inlined on one page never share an edge id. The root `<svg>` carries it only when the diagram has edge ids or animation.
- **Animation.** `animate: true` (or `animation: fast` / `slow`) adds mermaid's `edge-animation-*` class plus its dash-animation CSS, scoped to that SVG so it never restyles the host page.
- **Curve names.** `curve:` takes all thirteen mermaid curve names. `natural`, `monotoneX`, `monotoneY`, `cardinal` and `catmullRom` are drawn as one Catmull-Rom spline. Naming `basis`, mermaid's default curve, draws the S-shaped basis curve, and an unknown name means `basis`.

### Subgraphs

- **Direction.** A block's `direction` follows mermaid's rule. The block is laid out on its own, in its direction, while no edge crosses its border to or from a member. An edge to or from the block itself (`X --> s`, `s --> Y`) is fine. A member's edge to its own block (`C --> s`), or an edge between an outside node and a block nested in it, makes the block take the direction round it, as a crossing edge does. One case differs from mermaid: a block whose members have no links among them (see [Differences from mermaid](#differences-from-mermaid)).
- **Member to its own box.** An edge from a member to its own enclosing box (`Dev --> CI` with `Dev` inside `CI`) is drawn to the box's entry member, the first member nothing inside the box leads to.
- **Frames.** Each nesting depth draws its frame a step deeper in a cool grey. Hosts that paint frames themselves use `style::cluster_style(depth)` and `style::CLUSTER_TITLE`.

### Styling

- **Nodes.** `style` and `classDef` apply `fill`, `stroke`, `stroke-width` (0 to 1000px) and `color` (label text).
- **Links.** `linkStyle` applies `stroke`, `stroke-width` (0 to 1000px), `stroke-dasharray`, `fill` and `color` (edge label text). An index list takes no spaces. The arrow, cross and circle markers take the stroke's colour, with two deliberate differences from mermaid (see [Differences from mermaid](#differences-from-mermaid)).
- **Unknown properties** are ignored in both.
- **Merging.** A later `linkStyle` for the same index replaces the earlier one. The default and an index style merge per property, the index winning.
- **Curves.** `linkStyle <n> interpolate <curve>` sets the curve, using the same names as `curve:`. An `e1@{ curve: … }` on the same edge wins.
- **Wide strokes.** Every end glyph is 11px, a thick `==>` edge included, up to a stroke of about 6px. A wider stroke gets a head grown to hold it.

### Links (`click`)

- **Supported:** a url with an optional tooltip and target, for one node or several.
- **Output.** The SVG wraps the node in `<a href=… xlink:href=…>`, with `rel="noopener noreferrer"` on `_blank`. A tooltip becomes the node's `<title>`.
- **Safety.** Urls are checked like mermaid's default `strict` security level (a port of `@braintree/sanitize-url`). A `javascript:`, `data:` or `vbscript:` url is not linked at all. Other urls stay exactly as written.
- **Callbacks** (`click A callback`, `click A call fn(args)`) are accepted for compatibility and ignored, since a static SVG runs no script. Their tooltip still applies.

### Layout

What the layout does that shows in a drawing:

- A gap between layers that carries an edge label grows to hold the label clear of the arrowhead.
- A drawing broader than 2.5 : 1 is narrowed by trying other layerings. A diagram of more than 600 nodes plus edges skips this search.
- The order the edge lines are written in matters much less on a graph of up to 400 nodes plus edges than on a larger one.

What every flowchart drawing guarantees:

- The canvas is the drawing itself plus a 28 px margin on each side, so self-loops and back-edges are never clipped.
- A back-edge runs in an orthogonal lane just past what lies between its ends, in `scene()` and `route()` alike.
- An edge between two members of a subgraph stays inside its frame.
- A self-loop keeps its room and sits on the same side in `scene()` and `route()` (a side a drag crowds sends it to the free one), and other edges are routed clear of it.
- Parallel edges (same node pair) separate automatically.

## State diagrams

`stateDiagram-v2` (and the v1 `stateDiagram` header) rides the flowchart pipeline: states are rounded nodes, transitions are labelled edges, and composite states are the same nested clusters subgraphs use. So dragging, custom positions, picking and SVG export all work unchanged.

- **Supported subset** of [mermaid's state diagram syntax](https://mermaid.js.org/syntax/stateDiagram.html): start and end states, labelled transitions, state declarations with titles and descriptions, nested composites with their own `direction`, transitions to or from a composite, `<<choice>>`, `<<fork>>` / `<<join>>`, and `note left of` / `note right of` notes, on one line or as a block.
- **Not yet:** concurrency regions (`--`) and entry/exit actions.
- **Direction.** A composite's `direction` follows the [subgraph rule](#subgraphs). A composite that sets none keeps the direction round it (see [Differences from mermaid](#differences-from-mermaid)).
- **Fork and join** bars are laid across the flow and stretched to span their branches.
- **Notes.** `note "text" as N` is drawn as a free-standing card. Note cards are ordinary nodes, so they can be picked and dragged like states.

## Class diagrams

- **Supported subset** of [mermaid's class diagram syntax](https://mermaid.js.org/syntax/classDiagram.html): class blocks and inline members, with visibility, return types, generics and the `$` / `*` classifiers; several classes on one line; hyphenated, dotted (`com.example.Order`) and backtick names; `<<stereotype>>` annotations. All UML relations, solid or dashed, in either direction or two-way, with cardinalities and labels; lollipop interfaces. `direction`, and notes with or without a class.
- **Not yet:** `namespace` blocks, `classDef` / `:::` styling and `click` in class diagrams.
- **Routing.** Edges are orthogonal with rounded corners, square to the box faces and clear of the corners. An orthogonal route never runs along another edge; where no lane is free, the edge keeps a curve.
- **Fans.** A class with up to about a dozen subclasses draws its fan without a crossing. Crowded fan-ins (six or more classes pointing at one) can still cross, up to about a dozen times with labels and cardinalities. Much wider fans (twenty or more subclasses) can cross many times.
- **Cardinalities** sit beside their end, clear of lines, glyphs and labels, in every direction.
- **Self-relations** of a class alternate sides and nest, and their labels sit beside the loops, not on them. Where a loop would run under another box, cross a relation or a neighbour's loop, or cover a label, the loops are shared among the class's faces the way that does so least; a loop under a box counts before anything else. A class with more than eight loops keeps to two faces.

## Entity-Relationship diagrams

- **Supported subset** of [mermaid's ER diagram syntax](https://mermaid.js.org/syntax/entityRelationshipDiagram.html): all crow's foot cardinalities and both line kinds, as symbols or in mermaid's words; optional relationship labels; entity blocks with types, `PK` / `FK` / `UK` keys and comments; quoted entity names; aliases, also inside a relationship; `direction`.
- **Labels** are kept clear of the crow's feet and of neighbouring lines.
- **Entity blocks.** Several rows may share a line, and a whole block may fit on one. Key lists may be written in any case (`pk, fk`).
- **Comments** are drawn in their own muted, italic column after the keys. One wider than 280px is cut with an ellipsis and keeps its full text as a tooltip.

## Sequence diagrams

- **Supported subset** of [mermaid's sequence diagram syntax](https://mermaid.js.org/syntax/sequenceDiagram.html): participants and actors with aliases, and implicit participants; the eight arrows `->>`, `-->>`, `->`, `-->`, `-x`, `--x`, `-)` and `--)`; self-messages; `autonumber`; notes over one or two participants and left or right of one; activation, by keyword or the `+` / `-` shorthand; the frames `loop`, `opt`, `alt` and `par`.
- **Not yet:** mermaid's two-way `<<->>` / `<<-->>` and its half-arrows are not parsed (a line-numbered "unknown message operator" error). `box`, `rect`, `critical`, `break`, `create` / `destroy` and `autonumber` arguments produce an explicit "not supported yet" error.
- **Text.** Message and note text is single-line (`<br/>` collapses to a space).
- **Activation.** Unbalanced deactivation is a line-numbered error.
- **Frames.** By default, frames are aligned full-width bands that widen to contain their notes. Frontmatter `config: sequence: scopedFrames: true` (a flowmaid key, which mermaid ignores) sizes each frame to the lifelines it covers, as mermaid does.
- **Config.** Frontmatter `config: sequence: mirrorActors: false` drops the participant row at the bottom.

## Pie charts

- **Supported subset** of [mermaid's pie chart syntax](https://mermaid.js.org/syntax/pie.html): every header form (with or without `showData` and a title), a standalone `title` line, and the label and value rows.
- **Edge cases.** A duplicate label keeps one slice; the last value wins. Zero-value slices are legend-only, a single 100% slice renders as a full circle, and an all-zero total draws an empty outline.
- **Percentages.** A percentage label sits inside its wedge when the text fits there, otherwise just outside the rim on a short leader line.
- **Colours** come from the same stable accent palette as ER and class diagrams, extended to sixteen colours. Past sixteen slices the colours repeat with a hatch, so no two slices look alike (`pie::slice_hatch`).
- **Config.** `%%{init}%%` config other than the theme is ignored.
- **Accessible name.** Without `accTitle`, the chart's title is the accessible name.

## User journeys

- **Supported subset** of [mermaid's user journey syntax](https://mermaid.js.org/syntax/userJourney.html): `title`, `section`, and `task: score: actor, actor` rows. Fractional scores like `3.5` work.
- **Drawing.** Tasks run left to right in source order as faces whose colour (red frown → green smile), mouth and height follow their 1–5 score. A thin path links them, so the faces trace the journey's satisfaction curve. Pale section bands span their tasks, and long task names wrap, balanced.
- **Actors.** Each actor gets a stable colour, chosen away from the score colours, shown in a legend and as per-task dots with tooltips. Past ten actors the colours repeat and the dots change shape (`journey::actor_mark`).

## Git graphs

- **Supported subset** of [mermaid's git graph syntax](https://mermaid.js.org/syntax/gitgraph.html): orientation `LR` (default), `TB` and `BT`, including mermaid's `gitGraph TB:` form; `commit` with `id`, `tag`, `type: NORMAL|REVERSE|HIGHLIGHT` and `msg`; `branch` with `order:`; `checkout` / `switch`; `merge` with ids, tags and types.
- **Lanes** take eight accent colours in order, so from the ninth lane on the colours repeat.
- **Commit ids** are horizontal and measured, so they never collide or get struck by a line.

## Mindmaps

- **Supported subset** of [mermaid's mindmap syntax](https://mermaid.js.org/syntax/mindmap.html): the indentation tree, and the default, square, rounded, circle, hexagon, bang and cloud shapes.
- **Layout.** The root sits at the centre. First-level branches run clockwise from 12 o'clock in source order, each with a stable accent that its whole subtree shares. Siblings fan out round their parent.
- **Guarantee.** Boxes never overlap, connectors never cross and none runs through a box.
- **Big maps.** A map too big for the layout search, or with more than 120 branches on the root, is laid out on rings round the root: much sparser, with the same guarantee. Maps of a few hundred nodes finish the search in well under a second.
- **Icons.** A `::icon(fa fa-book)` line under a node draws a small built-in glyph beside its label, for common names such as book, file, code, cog, user, home, star, check, bolt, cloud, database and globe. Anything else gets a neutral mark. No icon font is needed.
- **Classes.** A `:::urgent large` line adds CSS classes to the node's SVG. As a flowmaid extension, a `classDef urgent fill:#ffe066,stroke:#c92a2a,color:#222` line in the mindmap colours those nodes directly.

## Timeline diagrams

- **Supported subset** of [mermaid's timeline syntax](https://mermaid.js.org/syntax/timeline.html): `title`, `section`, `period : event : event` rows, a bare `: event` line that continues the previous period, `LR` (default) and `TD` direction, `%%` and `#` comments, and `<br>`.
- **Colons.** A `:` in event text is literal unless followed by whitespace, so `https://example.com` and `::` survive intact.
- **Sections.** Periods before the first `section` form a leading unnamed band, drawn without a header and with neutral ticks. Named sections cycle through the stable accent palette.
- **Wrapping.** Periods or events wider than 160px wrap, balanced, keeping their `<b>` / `<i>` styling across lines.

## Sankey diagrams

- **Supported subset** of [mermaid's sankey syntax](https://mermaid.js.org/syntax/sankey.html): the three-field CSV rows, with double-quoted fields and `""` for a quote; `%%` comments and blank lines. `sankey` is accepted as a spelling of `sankey-beta`.
- **Layout** follows d3-sankey, which is what mermaid wraps. A node's thickness is the larger of its inflow and outflow.
- **Cyclic input** is still drawn: the links that close a loop (self-loops included) run back leftwards.
- **Order.** Within a column, nodes are ordered to reduce ribbon crossings. Ties are broken by declaration order, which keeps the SVG a stable function of the input.
- **Colours.** Each node takes a colour from a sixteen-colour palette that differs from every node it shares a ribbon with and from its neighbours above and below.
- **Labels** are measured: one that would overprint another label or sit on another node slides along its node, moves to the other side or wraps. The canvas keeps its configured `width` when it can, and grows rather than letting labels overprint.
- **Config** is read from the frontmatter under `config.sankey`, with mermaid's defaults. The keys are `width`, `height`, `linkColor`, `nodeAlignment`, `showValues`, `prefix`, `suffix`, `nodeWidth`, `nodePadding`, `nodeColors` and `labelStyle`. An unknown key is skipped, and a value that does not parse keeps its default.

## Architecture diagrams

- **Supported subset** of [mermaid's architecture syntax](https://mermaid.js.org/syntax/architecture.html): nested `group`s, `service`s, `junction`s and icons; port-based edges (`id:L -- R:other`), `-->` and `<-->` arrows, labelled `-[label]-` edges, and mermaid's `{group}` modifier.
- **Layout.** Groups never overlap.
- **Routing.** Edges are routed orthogonally around every box and group title, without ever being drawn onto an unrelated edge. Edge label chips sit on their own line, clear of boxes, titles and other chips.

## Advance swimlane diagrams (JSON and text input)

The `advance` module is an engine-native swimlane renderer: vertical lane columns, top-down nodes and orthogonal edge routing, all sharing the flowchart shapes and edge kinds. This diagram kind has no mermaid header, so it is outside the parity roadmap, and the diagram counts elsewhere in this document refer to mermaid-syntax types only.

### Text DSL

The engine takes a concise text notation (`advance::render_advance_text_svg`, CLI: `--advance-text`), with nested lanes, per-node and per-edge styling, ported edges and `<br/>` line breaks:

```mmd
classDef primary fill:#e8f0fe,stroke:#4285f4,stroke-width:3
config margin 24

lane backend "Backend" {
  a[Auth]::primary
  b[API Gateway]
  a --> b
}
lane web "Web" {
  c{Check}
  # ported edge: enter b through its top
  c --> b:top
}
style a-->b color:#188038,dash:4 2
style c fill:#fff8e1
```

- **Nested lanes.** A `lane id "Title" { ... }` block opens a child scope; a standalone `}` closes it. Nodes attach to the innermost open lane. A lane may be written on one line too (`lane l "L" { a[A]; b[B] }`).
- **`config`.** `config margin 24`, `config lane_gap 36`, `config node_gap_y`, `config lane_pad_x/y`, `config lane_title_h`, `config order declaration|topology`. Unknown keys are ignored.
- **Classes.** `classDef <name> k:v,k:v` defines a reusable class; `class a,b <name>` (or the `a::name` shorthand) applies it.
- **`style`.** `style <id> k:v,k:v` styles a node by id, or an edge by literal `from-->to` (ports allowed: `style a:right-->b:top ...`). Styling directives are resolved after the whole file parses.
- **Keys.** Node keys: `fill`, `stroke`, `stroke-width`, `color` (label text). Edge keys: `color`, `stroke-width`, `dash` (space-separated values, e.g. `dash:4 2`), `label-fill`.
- **Ports.** An endpoint can pin its side: `a:right --> b:top` (`left`, `right`, `top` or `bottom`). Leave the side off for automatic routing; `auto` is a JSON `from_side` / `to_side` value only. A ported edge's first and last segments leave the sides perpendicularly.
- **`<br/>`** is a line break in node labels. In edge labels (`-->|a<br/>b|`) and lane titles it is kept as a `\n` in the parsed diagram and in `AdvanceScene`, so a GUI painter can break the line, but the SVG still draws those labels on one line.
- **Comments.** A line starting with `#` (whole lines only).

### Sub-elements and named anchors

A node can open a block holding *sub-elements* (compartments) and *anchors* (named points on a boundary). An edge end then references any of them by id:

```mmd
lane hw "Hardware" {
  cpu[CPU] {
    anchor out bottom 0.5
    core0[Core 0]
    core1[Core 1] { anchor irq right }
    layout column
  }
  mem[Memory] {
    bank0[Bank 0]
    bank1[Bank 1]
    layout row
  }
}
lane sw "Software" {
  bus[Bus]
}
cpu.core1@irq --> mem.bank0
cpu@out       --> bus:top
cpu.core0     --> mem.bank1
```

- **Declarations.** `anchor <id> <side> [offset 0..=1]` declares a point (offset `0.5` when omitted). `layout column` is the default; `row` lays sub-elements side by side. A block may also be written on one line (`core1[Core 1] { anchor irq right }`), and blocks nest.
- **References.** `.` descends into sub-elements, `@` names an anchor and `:` picks a side, composably, to any depth. Ids may therefore not contain `.` or `@`. The three edges above read: anchor on a sub-element → sub-element; named anchor on the node → a side; sub-element → sub-element with the sides chosen for you.
- **Sides.** Sub-elements are laid out without coordinates and the node grows to fit. A sub-element side can be used only if it reaches the node boundary. In a column that is `left` / `right` always and `bottom` for the last child; in a row, `bottom` always and `left` / `right` for the first / last. `top` never works, because the node's label band sits there. An interior side is a parse error.
- **Shapes.** Sub-elements need a `rect` or `rounded` node.
- **Edges inside one node.** An edge between two sub-elements of one node always travels outside the node.

See `examples/advance_swimlane.mmd`, `examples/advance_terminals.mmd` and `examples/advance_text.rs`.

### JSON input

The same engine takes plain JSON (no mermaid header):

```json
{
    "lanes": [{ "id": "dev", "title": "Development" },
              { "id": "qa",  "title": "QA" }],
    "nodes": [{ "id": "a", "label": "Design", "lane": "dev" },
              { "id": "b", "label": "Test", "lane": "qa", "shape": "diamond" }],
    "edges": [{ "from": "a", "to": "b", "label": "hand off" }]
}
```

- **Rendering.** `advance::render_advance_svg(&str)` goes straight from JSON to SVG (CLI: `--advance`). `advance::layout_advance` returns the positioned geometry (`AdvanceScene`) for GUI painters.
- **Dragging.** Drag-mode re-renders come from `advance::render_advance_routed` (node centre positions) and `advance::render_advance_routed_with_lanes` (plus caller-controlled lane widths, margin and inter-lane gap). Content dragged outside the canvas is translated back into view, and the canvas grows to hold any route that leaves the lanes.
- **Nodes** carry `style` (`fill`, `stroke`, `stroke-width`, and `color`, the label text colour), `anchors` (`{id, side, offset}`), nested `elements` (each with its own `anchors` / `elements` / `layout`) and `layout`.
- **Edges** carry `style` (`color`, the line colour, plus `stroke-width`, `dash` and `label-fill`) and `from_side` / `to_side` (`"left"`, `"right"`, `"top"`, `"bottom"` or `"auto"`). Their `from` / `to` accept the full reference grammar (`"cpu.core1@irq"`).
- **Scene.** The scene exposes each node's placed `elements` and resolved `anchors`, each edge's `from_point` / `to_point`, and `crossings`: how many pairs of edges cross in the drawing, which the router minimises and a host can display.
- **Picking.** `AdvanceScene` offers drag-and-drop picking: `hit_test`, `anchor_at`, `element_at`, `node_at`, `edge_at`, `lane_at` and `nearest_node`.

## Interactive API

Beyond static SVG, the engine exposes the geometry it draws, so GUI apps can paint it with any framework and let users drag it.

### Scene, drag and re-route

- **`scene::scene(&graph)`** returns a `Scene`: the position, size and shape of every node plus the path of every edge, in final coordinates, ready to draw with any painter. A static `Scene`'s `width` × `height` and node centres are exactly the SVG's canvas and coordinates. Nodes and edges keep a stable identity (`SceneNode.id`, `SceneEdge.from` / `to`).
- **`scene::route(&graph, &positions)`** re-routes every edge for custom node positions (a user's drag) *without* re-running layout, so nodes never jump back.
- **`scene::route_partial`** re-routes only the edges a move can affect, keeping the rest of a base drawing, so a drag frame costs what lies near the drag (see [Interactive editing](#interactive-editing)). It combs its small fans only; the larger ones are combed when the drag ends and `route()` runs.
- **`scene::box_edge_bezier`** gives flowmaid's curve for one edge between two freely placed boxes, for hosts that own their node model.
- **`scene::to_svg(&scene)`** exports any state, including after drags. `render()` is just a wrapper over the same pipeline, so the SVG and the interactive flow always agree.
- **Compact layout.** `fold::scene_compact` folds a long linear chain into balanced rows that fit a viewport (CLI `--compact <px>`).
- **Edge paths.** A flowchart or state edge drawn on the axes (see [Edges](#edges)) comes as `SceneEdge::waypoints`, which a painter's spline turns through as rounded corners. Read the drawn ends from `waypoints[0]` and `waypoints.last()`, and the arrowhead direction from the first and last legs. `SceneEdge::bezier` does not follow such a path: it is the edge only when `waypoints` is empty (a straight edge, a self-loop).
- **Curves.** A non-default mermaid `curve` is in `SceneEdge::meta.curve`. It is drawn between the bezier's two ends, or through the waypoints when the edge has them.
- **Edge ends.** An edge ends on its node's border, square to the side, so a host draws its own arrowhead there. Trimming the stroke under the arrowhead, and the hops and gaps at crossings, are done by flowmaid's SVG painter only: the waypoints hold the plain path.
- **Links.** Nodes with a mermaid `click` carry it on `SceneNode::link` (url, target, tooltip), so a host can open the link or show the tooltip itself.
- **Real fonts.** Embedders using a real font can inject its metrics. Size nodes with `layout::intrinsic_size_wrapped(node, g.wrapping_width(), measure)` and pass the same `measure` to `scene::scene_sized_with` / `scene::route_sized_with`. `Graph::set_wrapping_width` sets the wrap width (`None` = never wrap). `scene_sized` / `route_sized` break lines with the built-in estimate, as markdown labels always do.

See `examples/drag_sim.rs`.

### Picking

`Scene::{hit_test, node_at, edge_at, cluster_at, hits_in_rect, nearest_node}` are shape-precise and match what is painted, so pick → drag → route works without the host doing its own geometry. `edge_at` follows the painted stroke, a crossing hop's arc included. Picking costs microseconds.

### Saving back to mermaid

`emit::to_mermaid(&graph)` is the inverse of `parse`: a lossless editor round-trip via mermaid entity codes (`#quot;`, `#124;`), writing valid standard mermaid. It completes pick → drag → route → **persist**. Wrapping never changes what is saved: `to_mermaid` writes a label, markdown included, as it was written. A plain label also stays as written in the model, while the scene's node label carries the drawn lines. A markdown label's `Node::label` holds its wrapped lines, and `Graph::set_wrapping_width` wraps them again at the new width.

### Other diagram types

- **ER and class** diagrams have the same `scene` / `route` / `to_svg` triple (`er::`, `class::`). Each entity or class is one node, and each relationship one edge.
- **Glyph helpers.** `er::glyph` and `er::end_glyphs` return the crow's feet the SVG draws. `class::rel_head`, `class::rel_tail` and `class::card_spots` give the UML end glyphs and cardinality positions. `seq::head` and `seq::actor_figure` give sequence arrowheads and stick figures.
- **State** diagrams use the flowchart API directly: the parser maps them straight onto `Graph`.
- **Mindmaps** have `mindmap::scene` / `mindmap::route`.
- **Static diagrams** (sequence, pie, journey, timeline, sankey, git graph, architecture) have a `scene()` and a `to_svg()` but no `route()`.
- **Git graphs and architecture** diagrams also expose a generic `Scene`, so terminal and GUI hosts can draw (and pick on) them without knowing the source syntax.

### Apps built on it

Consumers built on this API (separate repos; the engine itself stays dependency-free):

- [flowmaid-desktop](https://github.com/go-routine-id/flowmaid-desktop): an eframe/egui editor with document tabs, a folder explorer, a drag & drop canvas with zoom & pan, and SVG export.
- [flowmaid-web](https://github.com/go-routine-id/flowmaid-web): the wasm playground, with the same drag model in the browser.
- [flowcli](https://github.com/go-routine-id/flowcli): a terminal renderer (ratatui) that draws the `Scene` as Unicode box- and line-drawing, with no SVG at all.

Other frameworks can show the SVG string (a Tauri or Dioxus webview, iced's svg widget, Slint and GTK4 natively) or draw the `Scene` directly with their own painter.

## How it works

Flowcharts (and state, class and ER diagrams, which map onto the same graph) go through one pipeline:

1. **Parse** (`parser.rs`): a hand-written parser with line-numbered error messages. It fills the shared model (`model.rs`: `Graph`, `Node`, `Edge`).
2. **Layout** (`layout.rs`): a Sugiyama pipeline following [dagre](https://github.com/dagrejs/dagre), the algorithm mermaid.js uses. All four directions are one transform at the end.
3. **Scene** (`scene.rs`): maps the layout to final coordinates and builds every edge. `scene()`, `route()` and `route_partial()` finish the edges in the same fixed stages: distinct, ordered ports on every node side, edges redrawn on the axes, repair of edges that pass through a node or a foreign frame or run on top of another edge, combed fans, and collision-aware label placement.
4. **SVG** (`scene::to_svg`): writes the scene. `render.rs` is just scene → SVG.

The other diagram types have their own modules (`seq`, `pie`, `journey`, `gitgraph`, `mindmap`, `timeline`, `sankey`, `architecture`, `advance`), each computing a scene and writing it. What the layout does and guarantees in a flowchart drawing is listed under [Layout](#layout).

## Performance

### Next to mermaid

25 sample diagrams — flowcharts from a handful of nodes up to 52 nodes or 75 edges, plus every other diagram type — rendered from the same `.mmd` source. flowmaid is about **113× faster** (geometric mean) and its SVG is 1.5–25× smaller.

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

**Method.** Each cell is the median of 30 timed renders after warm-ups (flowmaid 3, mermaid 2). flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build, `cargo run --release --example bench -- files <file.mmd>...`. mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string, including its in-browser text measurement) in headless Chromium 153 via Playwright. Speed-up = mermaid median ÷ flowmaid median. SVG KB = length of the returned SVG string. Measured on 0.39.0. Machine: 4 vCPUs, Intel Xeon @ 2.10 GHz, rustc 1.97; the box was shared with a background build, so expect ±20 % on both sides, and more on a single sub-millisecond row (er, gitgraph and journey took about half their 0.38.0 time in this run, with no change to their code).

### Interactive editing

Dragging a node does not re-run layout: `route_partial` re-routes only the edges the move can affect.

- **Drag frames** average about **3 ms a frame on the densest sample** (`bench-75`: 21 nodes, 75 edges; 6 ms at worst) and 2–2.4 ms on `dense-lr` and `dense-td`. The small corpus diagrams average 0.14–0.6 ms.
- **A full `route()`** of every edge averages about 8.5 ms on `bench-75` and 4–6.3 ms on the other dense samples (0.39.0). `cargo run --release --example quality -- <dir> --drag` times it.
- **Picking** (`Scene::hit_test`, `edge_at`, `hits_in_rect`) follows the painted stroke and costs microseconds.

**Method.** The drag frames are the quality harness's (`quality::drag_frames`: every node moved along the drawing's diagonal and nudged each way). They were measured on 0.38.0; the comb 0.39.0 adds makes the mean frame 1.4 % longer on `bench-75` and leaves the other two dense samples unchanged.

### Large graphs

flowmaid spends its time on drawing quality — crossing-aware routing, back-edge lanes, port spacing and label placement — and that work grows faster than linearly. The synthetic benchmark (`cargo run --release --example bench`: random edges between layers, about two edges per node) on the same box:

| nodes | edges | parse | layout | render* | SVG |
|------:|------:|------:|-------:|--------:|----:|
| 49 | 100 | 0.15 ms | 1.3 ms | 30 ms | 70 KB |
| 200 | 400 | 0.53 ms | 25 ms | 0.22 s | 589 KB |
| 1,000 | 2,010 | 2.8 ms | 0.92 s | 2.3 s | 3.2 MB |

\* render includes layout and routing.

Diagrams people write by hand — up to a few dozen nodes — render in milliseconds, fast enough to re-render on every keystroke. Dense graphs of hundreds of nodes take tenths of a second to seconds; making them faster is on the roadmap. Always measure with `--release` (debug builds are about 10× slower).

## Mermaid parity roadmap

The goal: **mermaid.js functionality, pure-Rust edition.** The progress board with all the details is [issue #10](https://github.com/go-routine-id/flowmaid/issues/10). Contributions are welcome, and every item has its own issue.

- **Diagram types not drawn yet** are tracked on the board. [Diagram types](#diagram-types) lists their headers and what each does today.
- **Open flowchart item:** `$$…$$` math in labels, KaTeX-style. Phase A (italic TeX passthrough, correct widths) was done in v0.20.0; next: MathML subset → native ([#12](https://github.com/go-routine-id/flowmaid/issues/12)).

## Limitations

- **Text width is estimated** (a character-class table), since there are no real font metrics by default: very long labels or CJK text can be off. Embedders with a real font can inject its metrics (see [Scene, drag and re-route](#scene-drag-and-re-route)).
- **Escaped quotes** (`\"`) inside labels aren't supported; write mermaid's `#quot;`.
- **Dense graphs** of hundreds of nodes take tenths of a second to seconds, and very dense drawings can still show crossings and edges running side by side.
- **More than 400 edges** in one laid-out part: edges are drawn as curves and crossings plain (see [Edges](#edges)).
- Known gaps in mermaid syntax are listed per diagram type above and on the [roadmap](#mermaid-parity-roadmap).

## License

GPL-3.0-or-later — free for everyone to use; distributed derivatives must remain open source under the same license. Full text in the `LICENSE` file.
