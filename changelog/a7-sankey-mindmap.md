<!-- Work package A7 (sankey-beta, mindmap). Merge these bullets into
     `## [Unreleased]` under the matching Keep-a-Changelog headings.
     Needs a **minor** bump: `SankeyConfig`, `SankeyNodeGlyph`,
     `SankeyScene`, `MindNode` and `mindmap::MindNodeBox` gain public
     fields and become `#[non_exhaustive]` (build `SankeyConfig` with
     `Default` and set fields; the others come from the parser /
     `scene()`), and `MindShape` gains a `Default` variant (now its
     `#[default]`) and becomes `#[non_exhaustive]`. `SankeyNodeGlyph`
     also gains `label_lines`. -->

### Added

- **Sankey `nodeColors` and `labelStyle`** (mermaid 12 config). A
  `config.sankey.nodeColors` map (block or `{ … }` flow form, quoted
  labels allowed) paints the named nodes in that colour verbatim (escaped),
  and the palette keeps the nodes around it clear of that colour — hex, CSS
  named colours (`steelblue`) and `rgb()` alike;
  `labelStyle: outlined` draws every label over a white halo and, as in
  mermaid, hangs labels left of the busiest column on their node's left.
  New: `SankeyConfig::{node_colors, label_style}`, `model::SankeyLabelStyle`,
  `SankeyNodeGlyph::{custom_color, label_text, label_lines, fill()}`,
  `SankeyScene::label_style`, `sankey::LABEL_H`.

- **Mindmap `::icon(…)` and `:::class` lines are kept** (they were
  dropped silently). An icon draws as a small built-in line glyph left of
  the label (book, file, code, cog, user, home, star, heart, check, bolt,
  cloud, database, globe, search, mail, lock — matched by name, so
  `fa fa-book`, `mdi mdi-cog` work; anything else gets a neutral sparkle),
  with the icon name as its tooltip; no icon font is needed. Classes go
  into the SVG as a `class` attribute, as in mermaid, so page CSS can style
  the node, and — a flowmaid extension — `classDef name fill:…,stroke:…,
  stroke-width:…,color:…` lines in the mindmap colour the nodes carrying
  that class (text still picks a readable ink when only `fill` is given,
  whether it is hex, a CSS colour name or `rgb()`).
  New: `MindNode::{icon, classes, style}`, `MindNodeBox::{font_size,
  font_weight, icon, classes, style, paint(), ink()}`,
  `mindmap::{icon_strokes, label_centre, underline}`.

### Fixed

- **The mindmap ring layout no longer overlaps boxes or runs connectors
  through them.** It put depth `k` on a ring `k · RING` (172px) out and
  spread each family over its parent's whole wedge, so big boxes on
  neighbouring rings overlapped and a connector to a child far round its
  ring cut back across others: on 60 random maps of 25–220 nodes it
  overlapped boxes on 54. Each ring now sits far enough out that its boxes
  clear the ring inside it, every box keeps to its own wedge, a family sits
  packed round its parent, and wherever a connector still bends back
  towards the root or passes through a sibling the gap out to that ring is
  widened until none does. Sparse, but clean on any tree; it is now only
  the fallback of the fan layout (see Changed).

### Changed

- **Mindmap layout: children hang off their parent** (report B6). Every
  depth used to sit on a fixed ring round the root (`RING` 172): a sparse
  909×773 canvas with 9% ink and leaf spokes from 55 to 223px. Each child
  now sits on a spoke from its parent, and siblings are swept round it
  clockwise in source order, each taking the first direction where its
  whole subtree stays clear (boxes 10px apart, no connector through a box
  or across another). Spokes start at `mindmap::TRUNK` (46px, border to
  border) from the root and `mindmap::SPOKE` (34px) below; a fan that does
  not fit lengthens its spokes only as far as it needs (searched back down
  after the first length that fits), then each child's own spoke is
  shortened as far as it stays clear, so a leaf branch sits close in beside
  a bushy one. The root hands its spare arc out evenly round the circle, so
  no sector is left empty: on 50 test maps the widest empty wedge round the
  root is under 65° (mostly under 15°), where a shared trunk left 100–157°.
  Other fans keep within 160° of the outward direction when they fit (up to
  280° when crowded) and aim their heaviest child straight on, so a chain
  runs straight; a root's only branch goes right. A crowded root (eight or
  more branches) also tries narrow first-level fans and keeps them when that
  shortens the longest connector. The showcase is 790×443 (node boxes
  cover 25% of it) with every spoke 34–54px; maps of 30–200 nodes need half
  the ring layout's canvas area or less. Boxes never overlap, connectors
  never cross and none runs through a box. The search has a work budget
  that grows with the map — random maps of 25–220 nodes and three to five
  levels lay out in 1–140 ms using at most 36% of theirs — so only a map
  that needs more (a full 511-node binary tree; a few thousand nodes
  against the cap of about a second's work) or that has more than 120
  branches on the root falls back to the ring layout, which is sparse but
  now just as clean (see Fixed). `route()` (drag) is unchanged.
- **Mindmap shapes.** `((circle))` is a true circle round its label (was a
  1.6:1 ellipse) and is drawn as `<circle>`. A node without a wrapper is
  now its own shape, `MindShape::Default` — mermaid's default node: rounded
  top corners, a flat bottom with a darker underline — instead of the same
  pill as `(rounded)`; `(rounded)` is a full pill and `[square]` has sharp
  corners. The cloud is made of round overlapping lobes (it was a jagged
  polygon that read as a second explosion); the bang's spikes are one depth
  all round a stadium that holds the label (they were 4× longer at the
  sides than on top). Both are sized round their label instead of by fixed
  multipliers. New: `mindmap::{BANG_SPIKE, CLOUD_LOBE, UNDERLINE_H,
  UNDERLINE_SHADE}`.
- **Mindmap hierarchy reads at a glance.** The root is filled with a deep
  indigo ink (`mindmap::ROOT_FILL` `#2f3768`, white bold 18px label)
  instead of a pale `#c9cfe0` that receded behind its own branches;
  first-level labels are 15px semibold, the rest 14px; connectors taper
  9 → 4.5 → 2.25 → 2px (were 4.5/3.5/2.5); horizontal padding 16px (was
  12). New: `mindmap::{ROOT_FONT, BRANCH_FONT}`.

- **Sankey colours never clash with a neighbour.** Nodes used to take
  `accent(i % 8)`, so Losses was painted like Electricity (a flat green
  ribbon between them) and Homes (indigo) sat on Industry (steel blue). Each
  node now gets a colour from the sixteen-colour `style::accent_wide` cycle
  that differs (CIE76 ΔE ≥ 35) from every node it shares a ribbon with and
  from the nodes directly above and below it, preferring one no other node
  uses; a diagram without clashes keeps the plain cycle.
- **Sankey labels are measured and kept off each other and the nodes.**
  At `width: 450`, "Electricity (124 TWh)" used to print over "Industry
  (48 TWh)", and a long sink name ran across the middle column. A label now
  slides along its own node's span (half-line steps), then tries the other
  side, until it clears every other label (with room to read two labels on
  one baseline apart) and every other node. Only when no clean spot is left
  inside may a first-column label hang off the left edge or a last-column
  label off the right. The canvas keeps its configured `width` whenever it
  can: the columns first draw in from the margins (to half their span, and
  only as long as no label gets crowded), then labels that still do not fit
  or still crowd wrap at spaces onto more lines (the value kept whole,
  `SankeyNodeGlyph::label_lines`, drawn as `<tspan>`s); only what is left
  over widens the canvas at that edge. Labels between columns cannot hang
  anywhere, so while one still prints on another label or a node (long
  names at a narrow `width`), the columns spread apart in steps of 1.2×,
  the long names still wrapped at the configured width's share, so the
  canvas grows only as far as that needs. A four-column, 17-node diagram
  at `width: 350` comes out 491px wide with every label clear; on 40
  random diagrams of three to five columns with long names at widths
  250–600, every label is clear. Stacked nodes too thin for a line of text
  get a wider gap. Labels are drawn at mermaid's 14px (was 13) and after
  all nodes.
- **Sankey crossings: a backward sweep.** The barycentre ordering only ever
  swept left to right, so the first column kept declaration order and the
  showcase kept one avoidable crossing (Coal→Electricity over Gas→Heating).
  Forward and backward sweeps now alternate and the ordering with the fewest
  crossings is kept — the first forward sweep's unless a later one strictly
  beats it, so the extra sweeps never add a crossing. The showcase now has
  none.
- **Sankey ribbons stack by position.** A ribbon that skips a column was
  stacked by comparing column ranks across different columns; ribbons now
  leave a node in the vertical order of their targets and enter it in the
  order of their sources.
