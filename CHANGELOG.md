# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
