<!-- Work package A8 (architecture-beta). Merge these bullets into `## [Unreleased]`
     under the matching Keep-a-Changelog headings. Needs a **minor** bump:
     `model::Architecture`, `ArchService` and `ArchEdge` gain public fields and
     become `#[non_exhaustive]`; `architecture::ArchScene` gains `title_at` and
     becomes `#[non_exhaustive]`. -->

### Added

- **More of mermaid's architecture-beta syntax parses.** `junction id` /
  `junction id in group` (a bare point where edges meet, drawn as a dot);
  the `{group}` edge modifier (`a{group}:R --> L:b{group}`), which attaches
  that end to the service's group frame, as in mermaid; labelled edges
  (`a:R -[calls]-> L:b`, drawn with flowmaid's white label chip); optional
  `[title]`s (`service db(database)`), quoted titles (`["Edge, west"]`);
  iconify-style icon names (`mdi:database`); and `align row|column …` lines,
  accepted and not applied. All of these used to be parse errors.
- **`accTitle` / `accDescr` are drawn for architecture diagrams**: the title
  replaces the SVG's `<title>` / `aria-label`, the description becomes a
  `<desc>`. New fields `Architecture::{acc_title, acc_descr}`.
- New model fields: `ArchService::junction`, `ArchEdge::{from_group,
  to_group, title}`. **Breaking (minor bump):** `Architecture`, `ArchService`
  and `ArchEdge` are now `#[non_exhaustive]`; build an `Architecture` with
  `Architecture::default()` and set its fields, services and edges come from
  the parser.
- `ArchScene::title_at`: where each group title is drawn (left end and centre
  line), index-parallel with `scene.clusters`, so a host paints titles where
  the routes avoid them. **Breaking (same minor bump):** `ArchScene` is now
  `#[non_exhaustive]` (it is only built by `architecture::scene`).

### Changed

- **architecture-beta lays services out by their ports, like mermaid.**
  `a:R -- L:b` puts `b` right of `a`, `a:B -- T:b` below it, a mixed pair
  such as `a:R -- T:b` diagonally; services feeding one port stack in a
  column in declaration order. Groups stay contiguous: no frame overlaps
  another or covers a service that is not its own. Unconnected groups and
  services are packed in rows of about √n instead of one long strip. On the
  comparison diagram Redis now sits right under Business Logic, Web and Mobile
  share a column, and the canvas is 882×488 (1.8:1) instead of a 1276×192
  strip (6.6:1). The old grid followed declaration order and ignored ports.
- **Edges leave and enter along their port normals and are routed around
  every service box, junction and group title**, orthogonally with rounded
  corners. Edges that share a port merge into one trunk in the gap between
  frames; other edges pay to cross each other, and an edge never turns on,
  runs along (or a hair beside) or passes through the stub of an unrelated
  edge, or runs through a junction's dot — each would draw a junction that is
  not there. Routes keep off frame outlines, and a few outer channels give
  edges going round the drawing room to stay apart. Edges are drawn after the
  boxes, so nothing can hide under a fill. Scene edges carry the routed path
  as densely sampled, corner-rounded `waypoints`, so a host splining through
  them (and `Scene::hit_test` / `edge_at`) follows the drawn line.
- **Service boxes:** labels are 14px (was 13px) and wrap inside the 150px box
  instead of overflowing it, over balanced lines; a single word too long to
  break widens its box. Icons are 24px (was 16px), and icon plus label are one
  block centred in the box (it sat 8px high, and the label 2px left of centre
  because of a trailing newline). Boxes take the accent colour of their
  top-level group instead of one colour per declaration index. Two services an
  edge joins sit 56px apart (was 28px for all siblings), so arrows keep a shaft.
- **Icons:** every glyph whose default presentation is text (☁ ✉ ⚙ 🗄 🖥 🛢)
  carries VARIATION SELECTOR-16, so all icons render in colour (some were drawn
  monochrome). `database` is an oil drum (was a file cabinet), `server` the
  rack-like cabinet (was a desktop monitor), `cpu` a gear (was a brain); new
  names: `desktop`, `firewall`, `container`/`docker`/`package`,
  `settings`/`config`/`gear`, `monitoring`/`metrics`, `log`/`logs`,
  `alert`/`bell`/`notification`, `api`/`plug`.
- flowmaid's own named qualifier `a{g}:R` now means the same as `{group}`
  (the edge attaches to group `g`'s frame) and still checks that `a` is in `g`.
- **Group titles move off an edge that rises through them.** When an edge
  enters (or leaves) the top port of a service right under its group's title,
  the title starts just right of the edge's line and the frame widens to hold
  it, so the edge runs straight instead of striking the title or wiggling
  round it. Titles are measured wide (bold text, colour emoji at 1.3em) and
  edges keep 8px from either end.
- Groups linked to several other groups keep their own grid: each group is laid
  out by its own edges from one anchor, and another group's services no longer
  push its cells around (they used to come out tangled in larger diagrams).
  Two boxes that face each other with a port each keep an edge's gap even when
  no edge joins them, so their stubs never meet and read as a link.
- Routing large architecture diagrams is much faster: the lattice is built per
  clearance instead of per point × clearance, and once a diagram's searches have
  done a lot of work, later ones weight their estimate. A 200-service random
  tangle takes ~1.4 s instead of ~20 s, a 120-service grid ~4 ms instead of
  ~60 ms; ordinary diagrams route exactly as before the budget.

### Fixed

- **architecture: an edge no longer passes behind or through a service box**
  (report P0 #12). On the comparison diagram `web -- api` ran behind Mobile
  App, so it read Web–Mobile–API, and `svc -- cache` ran through PostgreSQL,
  so Redis looked unconnected. Reordering the source drew a different false
  link.
- **architecture: services outside every group** were all drawn on top of
  each other at the canvas origin; they are laid out like any other service.
- architecture: a route over the top row (or left column) and its label chip
  could reach past the canvas edge and be cut off; the drawing now moves down
  or right to keep a margin round every route and chip.
- **architecture: edge label chips no longer cover each other, a service box,
  a junction or a group title.** Each chip is placed after every route is
  known. It slides along each leg of its edge's route. It prefers a stretch
  of line its edge does not share with another edge, and keeps clear of its
  own route's bends. When no stretch has room, it sits just beside its own
  line. Two labelled edges between the same two services (a request and its
  reply on one line) each get a chip's room in the layout, and their chips
  are placed together. Previously a chip went to the middle of the route's
  longest flat leg without checking anything else. On a labelled shop
  diagram, 'read replica' hid 'sql' on the trunk into Postgres and 'https'
  covered 'REST over https'. A chip wider than its leg hid the route's
  corners. Across 800 random diagrams there were 64 chip-on-chip overlaps,
  59 chips on a box and 160 chips on an unrelated edge. Now there are 0, 0
  and 13; the 13 are in dense tangles where no free spot touches the line.
