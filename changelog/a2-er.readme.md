<!-- Work package A2 (erDiagram): README.md edits for the integrator. -->

1. Feature list (README.md line ~21), replace the `erDiagram` bullet with:

- [x] `erDiagram`: full crow's foot cardinalities (symbols and mermaid's spelled-out forms), identifying/non-identifying lines, entity attribute tables with key and comment columns, entity aliases, `direction` (top-to-bottom by default, as in mermaid)

2. "Entity-Relationship diagrams" section, replace the "Supported subset: …" paragraph (line ~151) with:

Supported subset:
- **Cardinalities:** all crow's foot cardinalities, as symbols (`||` exactly one, `|o`/`o|` zero or one, `}o`/`o{` zero or many, `}|`/`|{` one or many; any symbol works on either side) or spelled out the way mermaid spells them (`only one`, `zero or one`, `one or more`, `zero or more`, `1`, `1+`, `0+`, `many(0)`, `many(1)`, …).
- **Lines:** identifying (`--` or `to`, solid) and non-identifying (`..`, `.-`, `-.` or `optionally to`, dashed).
- **Labels:** optional relationship labels. A label is kept clear of the crow's feet, and the layout leaves room for both glyphs and the label on a short line.
- **Entity blocks:** rows of the form `type name [PK|FK|UK] ["comment"]`. Several rows may share a line, and a whole block may fit on one (`A { int id PK "key" string name }`, `B {}`). Key lists may be written in any case (`pk, fk`). Types with parentheses (`varchar(255)`) and comments containing commas, parentheses or quotes are handled.
- **Comments:** comments are drawn in their own muted, italic column after the keys. One wider than 280px is cut with an ellipsis and keeps its full text as a tooltip.
- **Entities:** names may be quoted (`"Customer Account"`). An alias (`p[Person]`, `c["Customer Account"]`) replaces the name in the header. Mermaid accepts an alias only where an entity is declared; flowmaid also accepts one inside a relationship. Entities mentioned only in relationships render as title-only tables.
- **Layout:** `direction TB|BT|LR|RL` sets the flow; the default is top to bottom, as in mermaid. A relationship that closes a cycle is drawn as a plain line rather than a bow.

See `examples/er.mmd`.

3. Section on the `er` module (line ~362), replace "Each entity becomes one node of a synthetic left-to-right graph" with:

Each entity becomes one node of a synthetic graph (top-to-bottom unless the diagram sets `direction`)

and append to that paragraph:

`er::end_glyphs(edge, cards)` returns exactly the crow's feet the SVG draws for a relationship, laid along its drawn curve (rerouted relationships included). `ErTable::name_x` / `keys_end_x` / `comment_x` give the column positions of the type | name | keys | comment rows.
