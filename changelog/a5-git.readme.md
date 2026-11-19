<!-- Work package A5 (gitGraph): README.md edits for the integrator. -->

1. Feature list (README.md line ~28): replace the `gitGraph` bullet with:

- [x] `gitGraph`: LR/TB/BT orientations (mermaid's `gitGraph TB:` form too), commits (`id`, `tag`, `type: NORMAL|REVERSE|HIGHLIGHT`, `msg`), branches (with `order:` to place lanes), checkouts/switches, and merges with ids, tags and types. Each lane has its own colour. Forks and merges are drawn as rounded elbows, merge, REVERSE and HIGHLIGHT commits each get their own glyph, and tags are drawn as luggage labels. Ids are horizontal and measured, so they never collide or get struck by a line. *(v0.22.0)*
