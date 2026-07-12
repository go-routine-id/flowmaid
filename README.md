# flowmaid

[![CI](https://github.com/go-routine-id/flowmaid/actions/workflows/ci.yml/badge.svg)](https://github.com/go-routine-id/flowmaid/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/flowmaid.svg)](https://crates.io/crates/flowmaid)
[![docs.rs](https://docs.rs/flowmaid/badge.svg)](https://docs.rs/flowmaid)
[![license](https://img.shields.io/crates/l/flowmaid.svg)](LICENSE)

A small Mermaid-like diagram engine written in pure std Rust with zero external dependencies. Takes Mermaid-syntax text and produces SVG — or live, draggable geometry for interactive apps.

**Website:** https://go-routine-id.github.io/flowmaid/ · **Playground:** https://go-routine-id.github.io/flowmaid-web/ · **Desktop editor:** [flowmaid-desktop](https://github.com/go-routine-id/flowmaid-desktop)

## Mermaid parity roadmap

The goal: **mermaid.js functionality, pure-Rust edition.** Progress board with all the details in [issue #10](https://github.com/go-routine-id/flowmaid/issues/10) — contributions welcome, every item has its own issue.

**Diagram types**

- [x] `flowchart` / `graph` — TD/LR/RL/BT, 5 shapes, 4 link types + labels, chains, cycles, self-loops, parallel edges
- [x] `erDiagram` — full crow's foot cardinalities, identifying/non-identifying lines, entity attribute tables
- [ ] `classDiagram` — [#5](https://github.com/go-routine-id/flowmaid/issues/5)
- [ ] `sequenceDiagram` — [#6](https://github.com/go-routine-id/flowmaid/issues/6)
- [ ] `stateDiagram-v2` — [#7](https://github.com/go-routine-id/flowmaid/issues/7)
- [ ] `journey` — [#8](https://github.com/go-routine-id/flowmaid/issues/8)
- [ ] `pie` — [#9](https://github.com/go-routine-id/flowmaid/issues/9)
- [ ] `mindmap` — [#11](https://github.com/go-routine-id/flowmaid/issues/11)
- [ ] The complete mermaid catalog, tracked on the board: `swimlanes` · `gantt` · `gitGraph` · `timeline` · `quadrantChart` · `requirementDiagram` · `C4` · `zenuml` · `sankey` · `xychart` · `block` · `packet` · `kanban` · `architecture` · `radar` · `eventmodeling` · `treemap` · `venn` · `ishikawa` · `wardley` · `cynefin` · `treeview`

**Flowchart features**

- [x] `subgraph` — nesting, titles, per-block `direction`, edges to/from a subgraph *(v0.5.0, v0.7.0)*
- [x] `<br/>` multi-line labels *(v0.7.0)*
- [x] `style` / `classDef` / `class` / `:::` custom colors *(v0.4.0)*
- [x] Semantic color theme (shape-based) + stable ER accent palette
- [x] Interactive scene API — drag nodes, edges re-route live (`scene`, `route`, `box_edge_bezier`)
- [x] Explicit "not supported yet" errors for every known mermaid header
- [ ] `$$…$$` math in labels, KaTeX-style — phased passthrough → MathML → native — [#12](https://github.com/go-routine-id/flowmaid/issues/12)
- [x] Fan-out `A --> B & C`, inline `-- text -->` labels, `-.-`/`===` open lines, `~~~` invisible links *(v0.6.0)*
- [ ] More node shapes (cylinder, subroutine, hexagon, …) — [#4](https://github.com/go-routine-id/flowmaid/issues/4)
- [ ] `click` interactions, frontmatter themes, `$$math$$` — see the board

**Why flowmaid?** Zero dependencies, `wasm32` out of the box (the whole engine is a ~166 KB wasm bundle — mermaid.js is ~2.5 MB), sub-millisecond renders, line-numbered parse errors, and one geometry source shared by SVG export and interactive canvases.

## Installation

```bash
cargo add flowmaid
```

Or in `Cargo.toml`:

```toml
[dependencies]
flowmaid = "0.1"
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

It also works as a library (the crate is lib + bin):

```rust
let svg = flowmaid::render_svg("flowchart TD\nA[Start] --> B[Done]")?;
```

## Supported syntax

The header sets the flow direction: `flowchart TD` (top-down, alias `TB`), `LR` (left-right), `RL`, or `BT`. The keyword `graph` is also accepted. Lines starting with `%%` are comments, and `;` separates multiple statements on one line.

Node shapes: `A[text]` rectangle, `A(text)` rounded, `A([text])` stadium, `A{text}` diamond, `A((text))` circle. Labels may be quoted to protect special characters: `A["odd [text]"]`.

Edges: `-->` arrow, `---` open line, `-.->` dotted, `-.-` dotted open, `==>` thick, `===` thick open, and `~~~` invisible links that shape the layout without being drawn. Labels come in both mermaid spellings — `-->|text|` and inline `-- text -->` (`-. text .-`, `== text ==>`). Fan-out lists work on either side: `A --> B & C`, `A & B --> C`. Chains like `A --> B & C --> D` are supported, as are cycles (`E --> B` looping back up) and self-loops (`A --> A`).

Subgraphs use mermaid's block syntax — `subgraph id [Title]` … `end` — with arbitrary nesting and an optional `direction LR` line per block (inherited from the parent otherwise). A node mentioned inside a block is claimed by it, mermaid-style, even if it first appeared elsewhere. Edges may freely cross cluster borders; edges to/from a subgraph *itself* aren't supported yet and fail with a clear error. See `examples/subgraph.mmd`.

Custom colors use mermaid's styling syntax: `style A fill:#f9f,stroke:#333,stroke-width:4px,color:#fff`, reusable classes via `classDef hot fill:#ffe3e3,stroke:#e03131` + `class A,B hot` or the inline shorthand `A:::hot`. Supported properties: `fill`, `stroke`, `stroke-width`, `color` (label text); unknown properties are ignored. Unstyled nodes fall back to a semantic theme — shape determines color (stadium green, diamond amber, circle violet, ...; see the `style` module) — and ER entities cycle through a stable accent palette.

Complete examples live in `examples/demo.mmd` and `examples/lr.mmd`.

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

Other Mermaid diagram types (`sequenceDiagram`, `classDiagram`, `gantt`, ...) are detected and produce an explicit "not supported yet" error instead of a confusing parse failure.

## Architecture

A three-stage pipeline, one module per stage:

1. `parser.rs` — hand-written character-cursor parser. Each line is parsed into a chain of nodes and edges, with line-numbered error messages.
2. `layout.rs` — a compact Sugiyama-style algorithm: (a) DFS marks *back-edges* so cycles can't break the layering, (b) *longest-path layering* assigns nodes to layers, (c) alternating *barycenter* sweeps reduce edge crossings, (d) coordinates come from per-layer packing followed by alignment towards the mean neighbour position without overlaps. Everything is computed in abstract coordinates (breadth × layer), so all four diagram directions are handled by a single transform at the end.
3. `render.rs` — maps abstract coordinates to final x,y according to the direction, then draws bezier curves with arrowheads (SVG markers), clips lines exactly at shape borders (rectangles, circles, and diamonds each have their own intersection formula), and places labels at curve midpoints.

`model.rs` holds the shared data structures (`Graph`, `Node`, `Edge`, shape and direction enums).

For interactive apps there is the `scene` module: `scene()` produces final ready-to-draw geometry (node positions, edge bezier curves), `route()` re-routes edges for custom node positions such as user drags, and `to_svg()` exports any arrangement. `render()` is now just a wrapper over the same pipeline. See `examples/drag_sim.rs` and the egui demo app.

The `er` module maps an `ErDiagram` onto the same machinery and mirrors the `scene` API: `er::scene()` for automatic layout, `er::route()` to follow dragged entity positions, `er::to_svg()` to export any arrangement. Each entity becomes one node of a synthetic left-to-right graph (sized from its attribute table via `scene::scene_sized`), each relationship one edge; only the writer differs — tables instead of shapes, crow's foot glyphs (exposed as plain geometry via `er::glyph`) instead of arrowheads.

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

Beyond static SVG, the engine exposes an interactive API for GUI apps through the `scene` module: `scene(&graph)` returns a `Scene` — the position, size, and shape of every node plus the bezier curve of every edge in final coordinates — ready to draw with any framework's painter. When a node is dragged, call `route(&graph, &positions)` to re-route edges for custom positions *without* re-running layout — so nodes never jump back. `to_svg(&scene)` exports any state, including after drags. Hit-testing is done by the app from the `Scene` geometry (per-node position + size + shape are all there).

A complete demo (separate crate; the engine itself stays dependency-free) shows a live text editor with a *last good render* pattern on the left, a drag & drop canvas with zoom & pan on the right, `.mmd` file drop, and SVG export — built on eframe/egui. For other frameworks: Tauri/Dioxus can inject the SVG string into a webview; iced has an svg widget; Slint and GTK4 render SVG natively; or draw the `Scene` directly with each framework's painter like the egui demo does.

## Limitations & ideas

Already handled: the canvas is computed from the bounding box of all curve control points so self-loops and back-edges are never clipped; parallel edges (same node pair) separate automatically; long edges that align with a column of nodes bow sideways as a mitigation.

Still open: text width is estimated (~character-class table) since there are no real font metrics, so very long labels or CJK can be off; the long-edge mitigation is only a heuristic — the real solution is *virtual nodes* per crossed layer; edge labels can collide with nodes on dense diagrams; escaped quotes (`\"`) inside labels aren't supported; graphs with subgraphs route all edges with the position-aware free router (no layered back-edge arcs inside clusters). The full mermaid.js parity roadmap lives in [issue #10](https://github.com/go-routine-id/flowmaid/issues/10).

## License

GPL-3.0-or-later — free for everyone to use; distributed derivatives must remain open source under the same license. Full text in the `LICENSE` file.
