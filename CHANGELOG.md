# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.29.1] - 2026-08-31

### Fixed

- **Style values can no longer close the SVG attribute they are written into.**
  A `fill`, `stroke`, `color`, `dash`, or `label-fill` value carrying a quote could
  end its attribute and add markup of its own, so rendering an untrusted diagram
  could inject content into the SVG. Both quote forms mattered: `scene.rs` emits
  attributes with double quotes and `architecture.rs` with single ones.

  Reachable from a diagram's own text through flowchart `style` / `classDef`, the
  advance text DSL, the advance JSON node/edge `style` objects, and the advance
  diagram-level `style` block (`lane_fill`, `lane_stroke`, `text_color`,
  `edge_color`, `label_fill`). `gitGraph` and `architecture-beta` fill their node
  styles from the palette, so only an embedder driving the scene API can put text
  in those — they are covered for the same reason.

  Control characters are now dropped rather than escaped. XML 1.0 forbids them
  even as a numeric reference, so a single NUL in a colour or a label used to make
  the whole document unparseable.

  Colours are escaped at the render site rather than rejected at the parser.
  `Graph` and `NodeStyle` are public with public fields and the crate advertises
  the scene API for embedders, so a check in the parser would leave the
  programmatic path unprotected — and rejecting quotes would also turn away valid
  CSS such as `fill:url('#grad')` and break the `to_mermaid` round trip. The
  render site is the only boundary every colour actually crosses.

### Fixed

- **Sankey gradient ids are namespaced per diagram.** Ids were `fmsk0..N`, derived
  from the link index alone, so two sankey SVGs inlined on one page — which the docs
  site does — made every ribbon in the second resolve to the first's gradient, taking
  its stop colours *and* its `userSpaceOnUse` geometry. The id now carries a hash of
  the scene, which keeps output byte-identical across runs while making a collision
  between different diagrams effectively impossible.
- **Sankey config numbers must be finite and non-negative.** `f64::from_str` accepts
  `NaN` and `inf`, and they reached the SVG as `x="NaN"` or `viewBox="0 0 600 inf"`.
  `parse_sankey` already refused a non-finite *link* value; config numbers now get the
  same guard and fall back to their default.
- **An overflowing sum no longer produces infinite geometry.** Every link value can be
  finite while their sum reaches `+inf`, which became `height="inf"`. Sums are
  saturated, so the diagram draws — squashed — instead of emitting unparseable SVG.
- **Computed totals no longer leak float noise into labels.** A node total is a sum, so
  `0.1 + 0.2` rendered as `A (0.30000000000000004)`; it now reads `A (0.3)`.
- **`linkColor` and `nodeAlignment` ignore case**, like the YAML booleans beside them.
  `linkColor: Gradient` used to fall through to a CSS colour named "Gradient" and paint
  the ribbons black; `nodeAlignment: Left` silently laid out as `justify`. A value that
  is a real colour keeps its spelling, since CSS `var(--Name)` is case-sensitive.
- **`config.sankey` is read only as a direct child of `config`.** A `sankey` key nested
  under another namespace (`config.themeVariables.sankey`) silently reconfigured the
  diagram.
- The diagram-type count in `README.md` and `docs/index.html` was stale (eleven and ten
  against twelve shipped).

## [0.29.0] - 2026-08-31

### Added

- **`sankey-beta` diagram** — mermaid's CSV grammar (`source,target,value`, exactly
  three fields), double-quoted fields with `""` as one literal quote, `%%` comments,
  and `sankey` accepted as a spelling variant. Nodes are created on first mention and
  keyed by label, as in mermaid.
  - Layout follows d3-sankey: a node sits at its **longest** path from a source so
    every ribbon points rightwards, its thickness is the larger of inflow and outflow,
    and nodes are ordered within a column by neighbour barycentre to reduce crossings
    (ties keep declaration order, so the SVG stays a stable function of the input).
  - Ribbons are cubic-bezier bands; gradient ids come from the link index, never a
    counter, so output stays byte-identical across runs.
- **`config.sankey.*` read from YAML frontmatter** — `width`, `height`, `linkColor`
  (`gradient` default, plus `source` / `target` / any CSS colour), `nodeAlignment`,
  `showValues`, `prefix`, `suffix`, `nodeWidth`, `nodePadding`. Every default matches
  mermaid's `config.schema.yaml`. Unknown keys are skipped and an unparsable value
  keeps its default, so config for other diagram types never breaks a render.
  Frontmatter was previously accepted and thrown away.
- `examples/sankey.mmd`, a README section, and a docs-site row.
- All four `nodeAlignment` modes behave as d3-sankey defines them: `left` keeps a
  node's own depth, `justify` (default) flushes sinks right, `right` pushes each
  node as far right as its distance to a sink allows, and `center` pulls a
  source-less node up against its earliest target.

### Fixed

- **Colour values are escaped before they reach an SVG attribute.** An unescaped
  `linkColor` could close the `fill="..."` attribute and add markup of its own,
  so a diagram rendered from untrusted `.mmd` could inject content into the SVG.
- A column whose nodes all have value 0 no longer pins the scale for the whole
  diagram — it now constrains nothing, instead of capping every other column at
  1 px per unit.
- A cycle no longer inflates the column count. Depth is computed over a
  topological order with the loop-closing edges left out, which bounds it at
  `n - 1`; blind relaxation used to add a column per pass and shove every node
  against the right edge with ribbons pointing backwards.
- A column taller than the configured canvas grows the canvas instead of drawing
  nodes past the bottom edge.
- A node is now at least as tall as the ribbons it carries. Node heights and
  ribbon thicknesses were clamped to a minimum independently, so a node with many
  tiny flows had ribbons hanging outside the rectangle they left.
- `showValues` understands the YAML 1.1 booleans (`False`, `no`, `off`, `0`, ...);
  only the exact lowercase `false` used to switch it off.
- A quoted config value followed by a `#` comment no longer keeps its quotes, and
  a `#` inside the quotes stays part of the value.
- Characters between a closing `"` and the next comma in a sankey row are an error
  rather than silently discarded, so a typo cannot quietly re-target a flow.

## [0.28.1] - 2026-08-31

### Added

- **`timeline` diagram** — `title` / `section`, `period : event[: event...]` rows,
  `:`-continuation lines, `LR` / `TD` direction, `%%` and `#` comments, and `<br>`
  folding. `LR` draws sections side by side on one shared axis, `TD` stacks them.
- The rest of the Mermaid catalogue is now *recognised* — `requirementDiagram`,
  `quadrantChart`, `sankey` / `sankey-beta`, `xychart-beta`, `block-beta`,
  `packet-beta`, `radar-beta`, `treemap` / `treemap-beta`, `kanban`, `zenuml`, and
  the `C4*` family — so they fail with an explicit "not supported yet" instead of
  being parsed as a flowchart. Previously `quadrantChart`, `C4Context`, `kanban`
  and `radar` rendered a nonsense SVG and exited 0, while `sankey-beta` failed with
  the misleading `unknown edge operator near: '-beta'`.
- `architecture` is accepted as a spelling variant of `architecture-beta`. The
  dispatch already declared the alias, but the header was never recognised, so a
  bare `architecture` diagram fell through to the flowchart parser.
- Example fixture `examples/mindmap.mmd` — the last diagram type without one.

### Fixed

- The flowchart-only `parse()` no longer denies a diagram type the crate can
  actually parse. `mindmap`, `journey`, `gitGraph`, `architecture-beta`, and
  `timeline` now point at `parse_document()` like the other types, instead of
  answering with a self-contradictory `not supported yet (supported: ... mindmap ...)`.
- `flowmaid --help` advertised the supported headers without `timeline`.
- The crate description and `render_svg`'s doc comment likewise omitted `timeline`.

### Changed

- Recognised diagram headers live in one table (`DIAGRAM_HEADERS`) that drives
  header matching, the "supported: ..." list quoted by both parser entry points,
  and the CLI `--help`. Five hand-maintained lists had to agree before; three of
  them had already drifted.
- `parser::supported_types()` is public, so embedders and the CLI quote the list
  the dispatch actually implements.

## [0.28.0] - 2026-08-30

### Added

- **Advance text DSL quality upgrade** (`render_advance_text_svg`):
  - Brace-scoped **nested lanes** — `lane id "Title" { ... }` opens a child scope;
    nodes attach to the innermost open lane, a standalone `}` closes it.
  - `config` directive — `margin`, `lane_gap`, `node_gap_y`, `lane_pad_x/y`,
    `lane_title_h`, and `order declaration|topology`.
  - `<br/>` line breaks in node labels, edge labels (`-->|a<br/>b|`), and lane titles.
  - Mermaid-style styling — `classDef`, `class a,b name`, `id::class` shorthand,
    and `style` lines targeting nodes (`style a fill:#fee`) or edges
    (`style a-->b color:#188038,dash:4 2`).
  - **Ported edges** — `a:right --> b:top` pins the anchor sides with orthogonal
    routing; edges without sides keep the automatic routing byte-identical.
  - **Edge-label collision avoidance** — labels dodge other nodes and already-placed
    labels; untouched labels stay byte-identical.
- **Per-element styling in JSON** — nodes/edges accept a `style` object
  (`fill`/`stroke`/`stroke-width`/`color`; edges add `dash`/`label-fill`), and edges
  accept `from_side`/`to_side` (`left|right|top|bottom|auto`).
- **Per-color arrow markers** — one `<marker>` per resolved edge color (first-seen,
  deduplicated), so a styled edge's arrowhead matches its stroke.
- **Scene hit-testing** — `AdvanceHit` and `AdvanceScene::{hit_test, node_at,
  edge_at, lane_at, nearest_node}` for drag-and-drop picking (shape-precise for
  diamonds and circles).
- **CLI** — `flowmaid --advance-text <file.mmd>` renders the text DSL.
- **Docs & examples** — README section, docs site entry, fixture
  `examples/advance_swimlane.mmd`, and example `examples/advance_text.rs`.

### Changed

- Version bumped to 0.28.0 (minor — all additions are backward-compatible).

## [0.27.0] - 2026-08-30

### Added

- Nested horizontal lanes, corridor routing, and a text DSL for the advance /
  swimlane engine.

## [0.26.2] - 2026-08-30

### Added

- Advance swimlane engine enhancements.

## [0.26.1] - 2026-08-30

### Fixed

- Drag-positioned advance content no longer clips out of the canvas.

### Docs

- Advance swimlane module documented on the docs site.

## [0.26.0] - 2026-08-30

### Added

- Advance swimlane diagram module with JSON input (`render_advance_svg`,
  `layout_advance`, `render_advance_routed*`).

### Docs

- Diagram-type counts synced.

## [0.25.0] - 2026-08-30

### Added

- Opt-in responsive SVG viewport via `render_svg_advanced`.

### Fixed

- Mindmap sibling overlap via footprint-aware wedges.

## [0.24.1] - 2026-08-30

### Fixed

- Per-end arrowheads for architecture edges.

## [0.24.0] - 2026-08-30

### Added

- `architecture-beta` polish: icons, bidirectional arrows, group qualifiers, and
  centered layout.
