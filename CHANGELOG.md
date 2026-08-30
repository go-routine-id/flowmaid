# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
