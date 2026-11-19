<!-- Work package A1 (classDiagram): README.md replacements for the integrator. -->

## Line 22 (feature list), replace the `classDiagram` bullet with:

- [x] `classDiagram` — three-compartment boxes, member visibility, all UML relations (inheritance/realization/composition/aggregation/association/dependency), cardinalities + labels, notes, `<<stereotypes>>`, generics, `direction`, hierarchy laid out parents-first with orthogonal routes *(v0.9.0)*

## "Class diagrams" section, replace the "Supported subset: …" paragraph (around line 170) with:

Supported subset:

- **Members.** Class blocks with `+ - # ~` member visibility. A member with `()` becomes a method, otherwise a field. Inline members work via `Name : +member`, and `class Duck, Fish` declares several classes on one line. Members read as mermaid shows them: a method's return type follows `" : "` (`+total() : Money`), `~T~` generics read `<T>`, and a `$` or `*` classifier draws the row underlined (static) or in italics (abstract).
- **Generics.** `class Box~T~` is one class `Box`, shown as `Box<T>`.
- **Stereotypes.** `<<interface>>` (or any `<<x>>`) inside a block, or on its own line as `<<interface>> Name`, draws as `«interface»` above the name.
- **Relations.** All UML relations are supported: inheritance `<|--`, realization `..|>`, composition `*--`, aggregation `o--`, association `-->`, dependency `..>`, and plain link `--`/`..`, in either direction. Each relation takes optional `"cardinality"` strings on each side (a colon or operator inside the quotes is protected) and a `: label`.
- **Layout.** As in mermaid, relations are laid out in source order, with the left operand first. In `Animal <|-- Dog`, the parent sits above its child (with `direction LR` it sits to the left, and `TB`, `BT` and `RL` work too). The end glyph (hollow triangle, filled or hollow diamond, or open arrow) sits at the decorated end. Realization and dependency use dashed lines.
- **Routing.** Edges are orthogonal with rounded corners, square to the box faces and clear of the corners. An orthogonal route never runs along another edge (where no lane is free, the edge keeps a curve), and the end glyphs on one face sit apart. A fan's jogs nest in order, and a fan wider than its parent's face sends its outermost legs to the side faces, so a class with up to about a dozen subclasses draws its fan without a crossing. Crowded fan-ins (six or more classes pointing at one) can still cross a few times, and much wider fans (twenty or more subclasses) many times, a leg with no free lane keeping a diagonal curve across the others. Cardinalities sit beside their end, clear of lines, glyphs and labels, in every direction. A class's self-relations alternate sides and nest, those with cardinalities outermost so each cardinality sits by its own end. They move to a clear face when a sibling sits close, and their labels sit beside the loops, not on them.
- **Notes.** `note "text"` and `note for Class "text"` draw as yellow cards, and an attached note sits just above its class, linked to it by a dotted line. `\n` or `<br/>` breaks a note line. A diagram may hold notes alone. As in mermaid, a note for a class that is never declared stays free.
- **Comments.** Trailing `%%` comments are stripped (mermaid prints them as part of the label).
- **Not yet supported:** `namespace` blocks, `classDef` / `:::` styling and `click` in class diagrams.

See `examples/class.mmd`.

## Line 364 (architecture notes), replace the `class` module paragraph with:

The `class` module follows the same pattern for `classDiagram`: `class::scene()`, `class::route()` and `class::to_svg()`. Each class becomes one node of a synthetic graph, sized from its member list, and each relationship becomes one edge, laid out in source order. Each note becomes a node after the classes, with a dotted connector edge after the relationships. The writer draws three-compartment boxes, with relationships redrawn as orthogonal routes. A GUI painter gets the same UML end glyphs and cardinality positions the SVG uses from `class::rel_head` and `class::card_spots`. `class::head` builds a glyph from any tip and direction.

## docs/docs/index.html (syntax table, `Class` row, around line 285), replace the description cell with:

      <td>Fields/methods with visibility, static/abstract members, generics,
      <code>&lt;&lt;interface&gt;&gt;</code> stereotypes, notes, <code>direction</code>,
      inheritance, composition/aggregation, dependencies, association labels &amp; multiplicities</td></tr>
