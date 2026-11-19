<!-- Work package A7: README changes for the integrator to apply. -->

1. Roadmap line for mindmap (currently "- [x] `mindmap` — indentation-built
   tree, radial layout with colored branches, six node shapes
   (square/rounded/circle/hexagon/bang/cloud) *(v0.13.0)*"), replace with:

- [x] `mindmap` — indentation-built tree, radial layout with children fanned round their parent and colored branches, seven node shapes (default/square/rounded/circle/hexagon/bang/cloud), `::icon(…)` glyphs, `:::class` (+ mindmap `classDef`) *(v0.13.0)*

2. Sankey config table (section "Sankey diagrams"): add two rows after
   `nodePadding`:

| `nodeColors` | `{}` | Node label → CSS colour, block or `{ … }` map; overrides the palette |
| `labelStyle` | `legacy` | `outlined` draws labels over a white halo; labels left of the busiest column hang left |

   and after the "Layout follows d3-sankey…" paragraph add:

Within a column, nodes are ordered by barycentre sweeps in both directions, keeping the ordering with the fewest ribbon crossings. Each node takes a colour from a sixteen-colour palette that differs from every node it shares a ribbon with and from its neighbours above and below. Labels are measured: one that would overprint another label or sit on another node slides along its node or moves to the other side; a first- or last-column label with no room inside may hang off the canvas edge, the columns draw in to make room, and labels that still do not fit wrap onto more lines, so the canvas keeps its configured `width` when it can. When labels between columns still print on each other or on a node (long names at a narrow `width`), the columns spread apart instead — the canvas grows rather than letting labels overprint.

3. New short section before "## Sankey diagrams" (there is no mindmap
   section yet):

## Mindmaps

A `mindmap` builds its tree from indentation, as in mermaid. The root sits at the centre in a deep ink; first-level branches take a stable accent that their whole subtree shares, and run clockwise from 12 o'clock in source order. Every child hangs off its parent on a spoke and siblings fan out round their parent, so leaves cluster where they belong; the branches share the whole circle, each with a trunk only as long as it needs; boxes never overlap, connectors never cross and none runs through a box. A map too big for this search's work budget (which grows with the map: maps of a few hundred nodes finish in well under a second) or with more than 120 branches on the root is laid out on rings round the root instead — much sparser, with the same guarantee. Node shapes follow mermaid's wrappers: none (the default box with an underline), `[square]`, `(rounded)`, `((circle))`, `{{hexagon}}`, `))bang((` and `)cloud(`. A `::icon(fa fa-book)` line under a node draws a small built-in glyph beside its label (common names such as book, file, code, cog, user, home, star, check, bolt, cloud, database, globe; anything else gets a neutral mark — no icon font needed). A `:::urgent large` line adds CSS classes to the node's SVG; as a flowmaid extension, a `classDef urgent fill:#ffe066,stroke:#c92a2a,color:#222` line in the mindmap colours those nodes directly. `mindmap::scene` / `mindmap::route` give the geometry for interactive (drag) hosts. See `examples/mindmap.mmd`.
