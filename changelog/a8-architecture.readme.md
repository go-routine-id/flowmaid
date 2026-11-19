<!-- Work package A8 (architecture-beta): README.md edits for the integrator. -->

1. Feature list (README.md line ~29): replace the `architecture-beta` bullet with:

- [x] `architecture-beta` — nested groups, services, junctions, icons, and port-based edges (`id:L -- R:other`, `-->`, `<-->`, labelled `-[label]-`, mermaid's `{group}` modifier). Services are placed by their ports as in mermaid (`a:R -- L:b` puts `b` to the right), groups never overlap, and edges are routed orthogonally around every box and group title without ever being drawn onto an unrelated edge *(v0.24.0)*

2. Accessibility bullet (README.md line ~47): architecture now draws `accTitle` / `accDescr`.
   Remove `architecture` from the "parsed and not yet drawn for …" list (if work package A6
   also edits this line, keep both removals), e.g.:

- [x] `accTitle:` / `accDescr:` / multi-line `accDescr { … }` — the title replaces the SVG's accessible name, the description becomes a `<desc>` with `aria-describedby` (flowchart, state, class, sequence, ER, architecture; parsed and not yet drawn for pie, journey, timeline, gitGraph)
