# flowmaid

[![CI](https://github.com/go-routine-id/flowmaid/actions/workflows/ci.yml/badge.svg)](https://github.com/go-routine-id/flowmaid/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/flowmaid.svg)](https://crates.io/crates/flowmaid)
[![docs.rs](https://docs.rs/flowmaid/badge.svg)](https://docs.rs/flowmaid)
[![license](https://img.shields.io/crates/l/flowmaid.svg)](LICENSE)

A small Mermaid-like diagram engine written in pure std Rust with zero external dependencies. Takes Mermaid-syntax text and produces SVG — or live, draggable geometry for interactive apps. Eight diagram types today: flowcharts, ER, UML class, sequence, pie, state, mindmap, and user-journey diagrams.

**Website:** https://go-routine-id.github.io/flowmaid/ · **Playground:** https://go-routine-id.github.io/flowmaid-web/ · **Desktop editor:** [flowmaid-desktop](https://github.com/go-routine-id/flowmaid-desktop)

## Mermaid parity roadmap

The goal: **mermaid.js functionality, pure-Rust edition.** Progress board with all the details in [issue #10](https://github.com/go-routine-id/flowmaid/issues/10) — contributions welcome, every item has its own issue.

**Diagram types**

- [x] `flowchart` / `graph` — TD/LR/RL/BT, 11 node shapes, 7 link types + labels, chains, fan-out, cycles, self-loops, parallel edges
- [x] `erDiagram` — full crow's foot cardinalities, identifying/non-identifying lines, entity attribute tables
- [x] `classDiagram` — three-compartment boxes, member visibility, all UML relations (inheritance/realization/composition/aggregation/association/dependency), cardinalities + labels *(v0.9.0)*
- [x] `sequenceDiagram` — participants/actors, 8 arrow types, notes, activations, autonumber, loop/opt/alt/par frames *(v0.10.0)*
- [x] `stateDiagram-v2` — `[*]` start/end, composites with nested `[*]`/`direction`, `<<choice>>`/`<<fork>>`/`<<join>>`, transition labels, descriptions *(v0.11.0)*
- [x] `journey` — title/section/task, score-colored smiley faces (1–5), section bands, actor legend + per-task dots *(v0.14.0)*
- [x] `pie` — title, showData, percentage labels + legend *(v0.10.0)*
- [x] `mindmap` — indentation-built tree, radial layout with colored branches, six node shapes (square/rounded/circle/hexagon/bang/cloud) *(v0.13.0)*
- [ ] The complete mermaid catalog, tracked on the board: `swimlanes` · `gantt` · `gitGraph` · `timeline` · `quadrantChart` · `requirementDiagram` · `C4` · `zenuml` · `sankey` · `xychart` · `block` · `packet` · `kanban` · `architecture` · `radar` · `eventmodeling` · `treemap` · `venn` · `ishikawa` · `wardley` · `cynefin` · `treeview`

**Flowchart features**

- [x] `subgraph` — nesting, titles, per-block `direction`, edges to/from a subgraph *(v0.5.0, v0.7.0)*
- [x] `<br/>` multi-line labels *(v0.7.0)*
- [x] `style` / `classDef` / `class` / `:::` custom colors *(v0.4.0)*
- [x] Semantic color theme (shape-based) + stable accent palette shared by ER / class / sequence / pie
- [x] Interactive scene API — drag nodes, edges re-route live (`scene`, `route`, `box_edge_bezier`)
- [x] Explicit "not supported yet" errors for every known mermaid header
- [ ] `$$…$$` math in labels, KaTeX-style — phased passthrough → MathML → native — [#12](https://github.com/go-routine-id/flowmaid/issues/12)
- [x] Fan-out `A --> B & C`, inline `-- text -->` labels, `-.-`/`===` open lines, `~~~` invisible links *(v0.6.0)*
- [x] More node shapes — cylinder `[( )]`, subroutine `[[ ]]`, hexagon `{{ }}`, parallelograms `[/ /]` `[\ \]`, double circle `((( )))` *(v0.8.0)*
- [ ] `click` interactions, frontmatter themes, `$$math$$` — see the board

**Why flowmaid?** Zero dependencies, `wasm32` out of the box (all eight diagram types fit in a compact wasm bundle — mermaid.js is ~2.5 MB), sub-millisecond renders, line-numbered parse errors, and one geometry source shared by SVG export and interactive canvases. Input is forgiving where it should be: UTF-8 BOMs are stripped, CRLF is fine, and every known-but-unsupported Mermaid header fails with an explicit message instead of a confusing parse error.

## Installation

```bash
cargo add flowmaid
```

Or in `Cargo.toml`:

```toml
[dependencies]
flowmaid = "0.11"
```

## Usage

```bash
cargo build --release

# from a file
./target/release/flowmaid examples/demo.mmd -o demo.svg

# or through a pipe
cat examples/lr.mmd | ./target/release/flowmaid > lr.svg

# during development
cargo run -- examples/demo.mmd -o demo.svg
cargo test
```

It also works as a library (the crate is lib + bin) — one call, any supported diagram type, dispatched on the header:

```rust
let flow = flowmaid::render_svg("flowchart TD\nA[Start] --> B[Done]")?;
let uml  = flowmaid::render_svg("classDiagram\nAnimal <|-- Dog")?;
let fsm  = flowmaid::render_svg("stateDiagram-v2\n[*] --> Idle")?;
```

## Supported syntax

The header sets the flow direction: `flowchart TD` (top-down, alias `TB`), `LR` (left-right), `RL`, or `BT`. The keyword `graph` is also accepted. Lines starting with `%%` are comments, and `;` separates multiple statements on one line.

Node shapes: `A[text]` rectangle, `A(text)` rounded, `A([text])` stadium, `A{text}` diamond, `A((text))` circle, `A(((text)))` double circle, `A[(text)]` cylinder (database), `A[[text]]` subroutine, `A{{text}}` hexagon, `A[/text/]` and `A[\text\]` parallelograms. Labels may be quoted to protect special characters (`A["odd [text]"]`) and use `<br/>` for line breaks.

Edges: `-->` arrow, `---` open line, `-.->` dotted, `-.-` dotted open, `==>` thick, `===` thick open, and `~~~` invisible links that shape the layout without being drawn. Labels come in both mermaid spellings — `-->|text|` and inline `-- text -->` (`-. text .-`, `== text ==>`). Fan-out lists work on either side: `A --> B & C`, `A & B --> C`. Chains like `A --> B & C --> D` are supported, as are cycles (`E --> B` looping back up) and self-loops (`A --> A`).

Subgraphs use mermaid's block syntax — `subgraph id [Title]` … `end` — with arbitrary nesting and an optional `direction LR` line per block (inherited from the parent otherwise). A node mentioned inside a block is claimed by it, mermaid-style, even if it first appeared elsewhere. Edges may freely cross cluster borders, and an edge may target a subgraph *itself* (`CF --> VPC`) — it attaches to the cluster box; forward references (edge before the block) work via a pre-scan. See `examples/subgraph.mmd`.

Custom colors use mermaid's styling syntax: `style A fill:#f9f,stroke:#333,stroke-width:4px,color:#fff`, reusable classes via `classDef hot fill:#ffe3e3,stroke:#e03131` + `class A,B hot` or the inline shorthand `A:::hot`. Supported properties: `fill`, `stroke`, `stroke-width`, `color` (label text); unknown properties are ignored. Unstyled nodes fall back to a semantic theme — shape determines color (stadium green, diamond amber, circle violet, ...; see the `style` module) — and ER entities cycle through a stable accent palette.

Complete examples live in `examples/` — `demo.mmd` and `lr.mmd` for the basics, `advanced.mmd` for everything at once (nested subgraphs, all shapes and link types, `<br/>`, custom colors), plus one showcase per diagram type (`er`, `class`, `sequence`, `pie`, `state`).

## Entity-Relationship diagrams

`erDiagram` input renders entities as attribute tables connected with crow's foot notation:

```
erDiagram
    users ||--o{ posts : "writes"
    users {
        uuid id PK "default gen_random_uuid()"
        varchar(255) email UK "not null"
    }
```

Supported subset: relationships with all crow's foot cardinalities (`||` exactly one, `|o`/`o|` zero or one, `}o`/`o{` zero or many, `}|`/`|{` one or many), identifying (`--`, solid) and non-identifying (`..`, dashed) lines, optional relationship labels, and entity blocks with `type name [PK|FK|UK] ["comment"]` rows. Types with parentheses (`varchar(255)`) and comments containing commas, parentheses, or single quotes are handled. Entities mentioned only in relationships render as title-only tables. Attribute comments are parsed into the model but not drawn. See `examples/er.mmd`.

## Class diagrams

`classDiagram` input renders UML classes as three-compartment boxes (name / fields / methods) connected with UML relationship glyphs:

```
classDiagram
    class Animal {
        +String name
        -int age
        +move() void
    }
    Animal <|-- Dog
    Animal "1" o-- "*" Toy : owns
    Dog "1" --> "*" Toy : plays with
    Cat ..> Toy : ignores
```

Supported subset: class blocks with `+ - # ~` member visibility (a member with `()` becomes a method, otherwise a field); inline members via `Name : +member`; multiple classes in one line (`class Duck, Fish`); and all UML relations — inheritance `<|--`, realization `..|>`, composition `*--`, aggregation `o--`, association `-->`, dependency `..>`, and plain link `--`/`..`, in either direction. Each relation takes optional `"cardinality"` strings on each side (a colon or operator inside the quotes is protected) and a `: label`; the diagram is normalised so the end glyph (hollow triangle, filled/hollow diamond, or open arrow) always sits at the target end. Dashed lines are used for realization and dependency. Trailing `%%` comments and `direction` / `note` lines are accepted and ignored. Not yet rendered: generics (`List~T~`), `<<stereotype>>` badges, and `namespace` blocks. See `examples/class.mmd`.

## Sequence diagrams

`sequenceDiagram` input renders participant boxes across the top, dashed lifelines below, and one row per statement top-down:

```
sequenceDiagram
    autonumber
    actor U as User
    participant API
    U->>+API: GET /profile
    API-->>-U: 200 OK
    Note over U,API: cached for 60s
    loop every minute
        API->>API: refresh token
    end
```

Supported subset: `participant` / `actor` declarations with `as` aliases (actors draw as outlined boxes; implicit participants are created on first mention, in order of appearance); all eight message arrows — `->>` solid + filled head, `-->>` dashed + filled head, `->` / `-->` plain lines without a head, `-x` / `--x` cross ends, `-)` / `--)` async open arrows — including self-messages (`A->>A:`, drawn as a loop beside the lifeline); `autonumber` (bold `1.` prefixes on messages); notes in all four forms (`Note over A,B:`, `Note over A:`, `Note left of A:`, `Note right of A:` — keywords case-insensitive); activation bars via `activate` / `deactivate` or the `+`/`-` arrow shorthand (`A->>+B:` activates B, `B-->>-A:` deactivates B; unbalanced deactivation is a line-numbered error); and `loop` / `opt` / `alt`+`else` / `par`+`and` frames drawn as labeled boxes with dashed dividers. Trailing `%%` comments are stripped. Limitations: message and note text is single-line (`<br/>` collapses to a space), frames span the full diagram width rather than only the involved participants, and `box`, `rect`, `critical`, `break`, `create`/`destroy`, and `autonumber` arguments produce an explicit "not supported yet" error. See `examples/sequence.mmd`.

## Pie charts

`pie` input renders proportional slices (clockwise from 12 o'clock, in source order) with percentage labels and a color legend:

```
pie showData
    title Key elements in Product X
    "Calcium" : 42.96
    "Potassium" : 50.05
    "Magnesium" : 10.01
    "Iron" : 6
```

Supported subset: all header forms (`pie`, `pie showData`, `pie title …`, `pie showData title …`), a standalone `title …` line, and `"Quoted Label" : value` data rows with non-negative numbers. `showData` appends the raw value to each legend entry (`Calcium [42.96]`). Slices under 4% skip their percentage label but keep their legend row, zero-value slices are legend-only, a single 100% slice renders as a full circle, and an all-zero total draws an empty outline instead of NaN geometry. A duplicate label keeps one slice — the last value wins. Colors come from the same stable accent palette as ER/class diagrams (wrapping after 8 slices). Not supported: config/theme directives (`%%{init: …}%%`), `accTitle`/`accDescr`. See `examples/pie.mmd`.

## State diagrams

`stateDiagram-v2` (and the v1 `stateDiagram` header) rides the flowchart pipeline: states are rounded nodes, transitions are labelled edges, and composite states are the same nested clusters subgraphs use — so dragging, custom positions, and SVG export all work unchanged:

```
stateDiagram-v2
    [*] --> Idle
    Idle : waiting for input
    Idle --> Validating : submit
    state Validating {
        direction LR
        [*] --> Syntax
        Syntax --> Semantics : ok
        Semantics --> [*]
    }
    state decide <<choice>>
    Validating --> decide
    decide --> Done : valid
    Done --> [*]
```

Supported subset: `[*]` start/end pseudostates (scoped — a composite gets its own), transitions `A --> B : label`, bare-id state declarations, `state "Long title" as id`, description lines `id : text` (the first replaces the id as the label, later ones stack), composite `state X { ... }` blocks with nesting, per-composite `direction`, transitions to/from a composite box itself (forward references included), and `<<choice>>` (diamond) / `<<fork>>` / `<<join>>` (bars). `note ...` lines and `note ... end note` blocks are accepted and skipped. Not yet: concurrency regions (`--`), rendered notes, entry/exit actions. See `examples/state.mmd`.

Other Mermaid diagram types (`gantt`, `timeline`, `gitGraph`, ...) are detected and produce an explicit "not supported yet" error instead of a confusing parse failure.

## Architecture

A three-stage pipeline, one module per stage:

1. `parser.rs` — hand-written character-cursor parser. Each line is parsed into a chain of nodes and edges, with line-numbered error messages.
2. `layout.rs` — a compact Sugiyama-style algorithm: (a) DFS marks *back-edges* so cycles can't break the layering, (b) *longest-path layering* assigns nodes to layers, (c) alternating *barycenter* sweeps reduce edge crossings, (d) coordinates come from per-layer packing followed by alignment towards the mean neighbour position without overlaps. Everything is computed in abstract coordinates (breadth × layer), so all four diagram directions are handled by a single transform at the end.
3. `render.rs` — maps abstract coordinates to final x,y according to the direction, then draws bezier curves with arrowheads (SVG markers), clips lines exactly at shape borders (rectangles, circles, and diamonds each have their own intersection formula), and places labels at curve midpoints.

`model.rs` holds the shared data structures (`Graph`, `Node`, `Edge`, shape and direction enums).

For interactive apps there is the `scene` module: `scene()` produces final ready-to-draw geometry (node positions, edge bezier curves), `route()` re-routes edges for custom node positions such as user drags, and `to_svg()` exports any arrangement. `render()` is now just a wrapper over the same pipeline. See `examples/drag_sim.rs` and the egui demo app.

The `er` module maps an `ErDiagram` onto the same machinery and mirrors the `scene` API: `er::scene()` for automatic layout, `er::route()` to follow dragged entity positions, `er::to_svg()` to export any arrangement. Each entity becomes one node of a synthetic left-to-right graph (sized from its attribute table via `scene::scene_sized`), each relationship one edge; only the writer differs — tables instead of shapes, crow's foot glyphs (exposed as plain geometry via `er::glyph`) instead of arrowheads.

The `class` module follows the same pattern for `classDiagram`: `class::scene()`, `class::route()`, `class::to_svg()`. Each class becomes one node of a synthetic top-down graph (sized from its member list), each relationship one edge; the writer draws three-compartment boxes and UML end glyphs (exposed as plain geometry via `class::head`) at the target end.

The `seq` module is different: sequence diagrams are not graphs, so it skips the Sugiyama pipeline entirely. `seq::scene()` computes a linear layout — columns from declaration order (gaps widened until every message label and note fits), one row per statement top-down — and returns every box, lifeline, message polyline, note, activation bar, and frame in final coordinates; `seq::to_svg()` serialises any scene, and `seq::head` exposes arrowheads as plain geometry for GUI painters.

The `pie` module is pure geometry — no graph at all. `pie::scene()` returns the circle, per-slice angles/fractions, percentage-label anchors, and legend rows; `pie::to_svg()` serialises them (a ~100% slice becomes a real `<circle>`, since a single SVG arc cannot sweep 360°).

State diagrams need no module of their own: the parser maps them straight onto `Graph` (states are rounded nodes, `[*]` pseudostates are dedicated shapes, composites are subgraph clusters), so they inherit the whole flowchart pipeline — including `route()` and dragging — for free.

## Performance

A built-in benchmark lives in `examples/bench.rs` (pure std, deterministic synthetic graphs) — run it with `cargo run --release --example bench`. Measurements on Linux x86_64, rustc 1.75, release build, best of 3 runs:

| nodes | edges  | parse   | layout  | render* | SVG      |
|------:|-------:|--------:|--------:|--------:|---------:|
| 49    | 100    | 0.04 ms | 0.03 ms | 0.29 ms | 23 KB    |
| 200   | 400    | 0.16 ms | 0.08 ms | 1.13 ms | 97 KB    |
| 1,000 | 2,010  | 0.84 ms | 0.50 ms | 6.16 ms | 505 KB   |
| 2,500 | 5,050  | 1.97 ms | 1.30 ms | 16.35 ms| 1,278 KB |
| 5,000 | 10,150 | 4.16 ms | 2.75 ms | 34.92 ms| 2,618 KB |

\* the render column includes running layout internally.

End-to-end through the CLI for the 5,000-node case — including reading 10,151 input lines and writing a 2.7 MB SVG — takes about 60 ms with ~9 MB peak RAM. The quadratic-trap case (2 layers × 2,500 nodes side by side) finishes in 21 ms, so practical scaling is linear. For realtime use this means: re-rendering from scratch on every keystroke is fine for reasonable diagrams (~0.3 ms), and the 60 fps budget (16 ms) is only reached around 2,500 nodes. The bottleneck is SVG string building, not the algorithms. Numbers depend on hardware — re-measure on your machine with the command above, and always use `--release` (debug builds are ~10× slower).

## Interactivity & desktop apps

Beyond static SVG, the engine exposes an interactive API for GUI apps through the `scene` module: `scene(&graph)` returns a `Scene` — the position, size, and shape of every node plus the bezier curve of every edge in final coordinates — ready to draw with any framework's painter. When a node is dragged, call `route(&graph, &positions)` to re-route edges for custom positions *without* re-running layout — so nodes never jump back. `to_svg(&scene)` exports any state, including after drags. Hit-testing is done by the app from the `Scene` geometry (per-node position + size + shape are all there). The same `scene`/`route`/`to_svg` triple exists for ER (`er::`) and class (`class::`) diagrams, and state diagrams ride the flowchart API directly; sequence diagrams and pie charts are static (their `scene()` has no `route()` — nothing sensible to drag).

Two complete consumers are built on this API (separate repos; the engine itself stays dependency-free): [flowmaid-desktop](https://github.com/go-routine-id/flowmaid-desktop), an eframe/egui editor with document tabs, a folder explorer, drag & drop canvas with zoom & pan, and SVG export; and [flowmaid-web](https://github.com/go-routine-id/flowmaid-web), the wasm playground with the same drag model in the browser. For other frameworks: Tauri/Dioxus can inject the SVG string into a webview; iced has an svg widget; Slint and GTK4 render SVG natively; or draw the `Scene` directly with each framework's painter like the desktop app does.

## Limitations & ideas

Already handled: the canvas is computed from the bounding box of all curve control points so self-loops and back-edges are never clipped; parallel edges (same node pair) separate automatically; long edges that align with a column of nodes bow sideways as a mitigation.

Still open: text width is estimated (~character-class table) since there are no real font metrics, so very long labels or CJK can be off; the long-edge mitigation is only a heuristic — the real solution is *virtual nodes* per crossed layer; edge labels can collide with nodes on dense diagrams; escaped quotes (`\"`) inside labels aren't supported; graphs with subgraphs route all edges with the position-aware free router (no layered back-edge arcs inside clusters). The full mermaid.js parity roadmap lives in [issue #10](https://github.com/go-routine-id/flowmaid/issues/10).

## License

GPL-3.0-or-later — free for everyone to use; distributed derivatives must remain open source under the same license. Full text in the `LICENSE` file.
