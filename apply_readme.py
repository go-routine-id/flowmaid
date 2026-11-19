p='/home/user/flowmaid/README.md'
s=open(p).read()
def rep(old,new,count=1):
    global s
    assert s.count(old)==count, (s.count(old), old[:80])
    s=s.replace(old,new)

# --- feature list ---
rep("- [x] `erDiagram` — full crow's foot cardinalities, identifying/non-identifying lines, entity attribute tables\n",
    "- [x] `erDiagram` — full crow's foot cardinalities (symbols and mermaid's spelled-out forms), identifying/non-identifying lines, entity attribute tables with key and comment columns, entity aliases, `direction` (top-to-bottom by default, as in mermaid)\n")
rep("- [x] `sequenceDiagram` — participants/actors, 8 arrow types, notes, activations, autonumber, loop/opt/alt/par frames *(v0.10.0)*\n",
    "- [x] `sequenceDiagram` — participants and stick-figure actors, 8 arrow types, notes, activations, autonumber, loop/opt/alt/par frames, participants mirrored at the bottom *(v0.10.0)*\n")
rep("- [x] `journey` — title/section/task, score-colored smiley faces (1–5), section bands, actor legend + per-task dots *(v0.14.0)*\n",
    "- [x] `journey` — title/section/task, score-colored smiley faces (1–5) that rise and fall with the score (the satisfaction curve), fractional scores like `3.5`, pale section bands, actor legend + per-task dots with tooltips, long task names wrapped *(v0.14.0)*\n")
rep("- [x] `pie` — title, showData, percentage labels + legend *(v0.10.0)*\n",
    "- [x] `pie` — title, showData, percentage labels down to 1% (inside the wedge when they fit, else outside on a leader) + legend *(v0.10.0)*\n")
rep("- [x] `mindmap` — indentation-built tree, radial layout with colored branches, six node shapes (square/rounded/circle/hexagon/bang/cloud) *(v0.13.0)*\n",
    "- [x] `mindmap` — indentation-built tree, radial layout with children fanned round their parent and colored branches, seven node shapes (default/square/rounded/circle/hexagon/bang/cloud), `::icon(…)` glyphs, `:::class` (+ mindmap `classDef`) *(v0.13.0)*\n")
rep("- [x] `gitGraph` — LR/TB/BT orientations, commits, branches, checkouts/switches, merges with ids and tags *(v0.22.0)*\n",
    "- [x] `gitGraph` — LR/TB/BT orientations (mermaid's `gitGraph TB:` form too), commits (`id`, `tag`, `type: NORMAL|REVERSE|HIGHLIGHT`, `msg`), branches (with `order:` to place lanes), checkouts/switches, and merges with ids, tags and types. Each lane has its own colour. Forks and merges are drawn as rounded elbows, merge, REVERSE and HIGHLIGHT commits each get their own glyph, and tags are drawn as luggage labels. Ids are horizontal and measured, so they never collide or get struck by a line *(v0.22.0)*\n")
rep("- [x] `architecture-beta` — nested groups, services, icons, and port-based edges (`id:L -- R:other`, `-->`, `<-->`, `{group}` qualifiers) *(v0.24.0)*\n",
    "- [x] `architecture-beta` — nested groups, services, junctions, icons, and port-based edges (`id:L -- R:other`, `-->`, `<-->`, labelled `-[label]-`, mermaid's `{group}` modifier). Services are placed by their ports as in mermaid (`a:R -- L:b` puts `b` to the right), groups never overlap, and edges are routed orthogonally around every box and group title without ever being drawn onto an unrelated edge *(v0.24.0)*\n")
# timeline: append before version tag
rep("`<br>` and entity folding *(v0.28.1)*\n",
    "`<br>` and entity folding, bold periods, long periods/events wrapped *(v0.28.1)*\n")
# accessibility line (A5 draws no acc text for gitGraph; A6 + A8 do)
rep("(flowchart, state, class, sequence, ER; parsed and not yet drawn for pie, journey, timeline, gitGraph, architecture)",
    "(flowchart, state, class, sequence, ER, pie, journey, timeline, architecture; parsed and not yet drawn for gitGraph)")

# --- ER section ---
old_er=s[s.index("Supported subset: relationships with all crow's foot cardinalities"):]
old_er=old_er[:old_er.index("See `examples/er.mmd`.")+len("See `examples/er.mmd`.")]
rep(old_er, """Supported subset:
- **Cardinalities:** all crow's foot cardinalities, as symbols (`||` exactly one, `|o`/`o|` zero or one, `}o`/`o{` zero or many, `}|`/`|{` one or many; any symbol works on either side) or spelled out the way mermaid spells them (`only one`, `zero or one`, `one or more`, `zero or more`, `1`, `1+`, `0+`, `many(0)`, `many(1)`, …).
- **Lines:** identifying (`--` or `to`, solid) and non-identifying (`..`, `.-`, `-.` or `optionally to`, dashed).
- **Labels:** optional relationship labels. A label is kept clear of the crow's feet, and the layout leaves room for both glyphs and the label on a short line.
- **Entity blocks:** rows of the form `type name [PK|FK|UK] ["comment"]`. Several rows may share a line, and a whole block may fit on one (`A { int id PK "key" string name }`, `B {}`). Key lists may be written in any case (`pk, fk`). Types with parentheses (`varchar(255)`) and comments containing commas, parentheses or quotes are handled.
- **Comments:** comments are drawn in their own muted, italic column after the keys. One wider than 280px is cut with an ellipsis and keeps its full text as a tooltip.
- **Entities:** names may be quoted (`"Customer Account"`). An alias (`p[Person]`, `c["Customer Account"]`) replaces the name in the header. Mermaid accepts an alias only where an entity is declared; flowmaid also accepts one inside a relationship. Entities mentioned only in relationships render as title-only tables.
- **Layout:** `direction TB|BT|LR|RL` sets the flow; the default is top to bottom, as in mermaid. A relationship that closes a cycle is drawn as a plain line rather than a bow.

See `examples/er.mmd`.""")

# --- Sequence section ---
rep("`sequenceDiagram` input renders participant boxes across the top, dashed lifelines below, and one row per statement top-down:",
    "`sequenceDiagram` input renders participant boxes across the top (repeated below the lifelines, as mermaid's `mirrorActors` does), dashed lifelines between them, and one row per statement top-down:")
old_seq=s[s.index("Supported subset: `participant` / `actor` declarations"):]
old_seq=old_seq[:old_seq.index("See `examples/sequence.mmd`.")+len("See `examples/sequence.mmd`.")]
rep(old_seq, """Supported subset:

- **Participants.** `participant` / `actor` declarations with `as` aliases. Actors draw as UML stick figures with their name below. Implicit participants are created on first mention, in order of appearance.
- **Messages.** All eight arrows: `->>` solid with a filled head, `-->>` dashed with a filled head, `->` / `-->` plain lines without a head, `-x` / `--x` cross ends, and `-)` / `--)` async open arrows. Self-messages (`A->>A:`) draw as a loop beside the lifeline. `autonumber` adds bold `1.` prefixes.
- **Notes.** All four forms: `Note over A,B:`, `Note over A:`, `Note left of A:` and `Note right of A:`. The keywords are case-insensitive.
- **Activation bars.** Use `activate` / `deactivate` or the `+`/`-` arrow shorthand (`A->>+B:` activates B, `B-->>-A:` deactivates B). Both forms start and end the bar at the message they belong to, and arrows stop at the bar's edge. Unbalanced deactivation is a line-numbered error.
- **Frames.** `loop` / `opt` / `alt`+`else` / `par`+`and` draw as labelled boxes with dashed dividers. Each frame's conditions share one column next to its chip. By default, frames are aligned full-width bands that widen to contain their notes. Frontmatter `config: sequence: scopedFrames: true` (a flowmaid key, which mermaid ignores) sizes each frame to the lifelines it covers, as mermaid does.
- **Config.** Frontmatter `config: sequence: mirrorActors: false` drops the participant row at the bottom.
- **Comments.** Trailing `%%` comments are stripped. This deliberately differs from mermaid 12, which prints them as part of the message text.
- **Limitations.** Message and note text is single-line (`<br/>` collapses to a space). `box`, `rect`, `critical`, `break`, `create`/`destroy` and `autonumber` arguments produce an explicit "not supported yet" error.

See `examples/sequence.mmd`.""")

# --- Pie section ---
old_pie=s[s.index("Slices under 4% skip their percentage label"):]
old_pie=old_pie[:old_pie.index("but not yet written into the SVG.")+len("but not yet written into the SVG.")]
rep(old_pie, "Every slice of at least 1% gets a percentage label, as in mermaid: inside its wedge when the text fits there, otherwise just outside the rim on a short leader line (thinner slices keep their legend row). Percentages round halves up like mermaid's `toFixed(0)`. Zero-value slices are legend-only, a single 100% slice renders as a full circle, and an all-zero total draws an empty outline instead of NaN geometry. A duplicate label keeps one slice — the last value wins. Colors come from the same stable accent palette as ER/class diagrams, extended to sixteen colours before they repeat. `accTitle` / `accDescr` become the SVG's accessible name and `<desc>` (the value stops at `%%`, as in mermaid's langium grammar); without `accTitle`, the chart's title is the accessible name.")

# --- Timeline section ---
rep("separates events. Colors cycle through the stable accent palette per section.",
    "separates events. Colors cycle through the stable accent palette per named section; the unnamed leading group gets neutral ticks. Period labels are bold, the dashed task line is drawn only between events (never through their text), and periods or events wider than 160px wrap. `accTitle` / `accDescr` become the accessible name and `<desc>`.")

# --- Sankey section ---
rep("| `nodePadding` | `12` | Vertical gap between nodes in a column |\n",
    "| `nodePadding` | `12` | Vertical gap between nodes in a column |\n| `nodeColors` | `{}` | Node label → CSS colour, block or `{ … }` map; overrides the palette |\n| `labelStyle` | `legacy` | `outlined` draws labels over a white halo; labels left of the busiest column hang left |\n")
rep("with ties broken by declaration order, which keeps the SVG a stable function of the input.\n",
    "with ties broken by declaration order, which keeps the SVG a stable function of the input.\n\nWithin a column, nodes are ordered by barycentre sweeps in both directions, keeping the ordering with the fewest ribbon crossings. Each node takes a colour from a sixteen-colour palette that differs from every node it shares a ribbon with and from its neighbours above and below. Labels are measured: one that would overprint another label or sit on another node slides along its node or moves to the other side; a first- or last-column label with no room inside may hang off the canvas edge, the columns draw in to make room, and labels that still do not fit wrap onto more lines, so the canvas keeps its configured `width` when it can. When labels between columns still print on each other or on a node (long names at a narrow `width`), the columns spread apart instead — the canvas grows rather than letting labels overprint.\n")

# --- Mindmap section (new, before Sankey) ---
rep("## Sankey diagrams\n", """## Mindmaps

A `mindmap` builds its tree from indentation, as in mermaid. The root sits at the centre in a deep ink; first-level branches take a stable accent that their whole subtree shares, and run clockwise from 12 o'clock in source order. Every child hangs off its parent on a spoke and siblings fan out round their parent, so leaves cluster where they belong; the branches share the whole circle, each with a trunk only as long as it needs; boxes never overlap, connectors never cross and none runs through a box. A map too big for this search's work budget (which grows with the map: maps of a few hundred nodes finish in well under a second) or with more than 120 branches on the root is laid out on rings round the root instead — much sparser, with the same guarantee. Node shapes follow mermaid's wrappers: none (the default box with an underline), `[square]`, `(rounded)`, `((circle))`, `{{hexagon}}`, `))bang((` and `)cloud(`. A `::icon(fa fa-book)` line under a node draws a small built-in glyph beside its label (common names such as book, file, code, cog, user, home, star, check, bolt, cloud, database, globe; anything else gets a neutral mark — no icon font needed). A `:::urgent large` line adds CSS classes to the node's SVG; as a flowmaid extension, a `classDef urgent fill:#ffe066,stroke:#c92a2a,color:#222` line in the mindmap colours those nodes directly. `mindmap::scene` / `mindmap::route` give the geometry for interactive (drag) hosts. See `examples/mindmap.mmd`.

## Sankey diagrams
""")

# --- Library / architecture notes ---
rep("Each entity becomes one node of a synthetic left-to-right graph (sized from its attribute table via `scene::scene_sized`), each relationship one edge; only the writer differs — tables instead of shapes, crow's foot glyphs (exposed as plain geometry via `er::glyph`) instead of arrowheads.",
    "Each entity becomes one node of a synthetic graph (top-to-bottom unless the diagram sets `direction`; sized from its attribute table via `scene::scene_sized`), each relationship one edge; only the writer differs — tables instead of shapes, crow's foot glyphs (exposed as plain geometry via `er::glyph`) instead of arrowheads. `er::end_glyphs(edge, cards)` returns exactly the crow's feet the SVG draws for a relationship, laid along its drawn curve (rerouted relationships included). `ErTable::name_x` / `keys_end_x` / `comment_x` give the column positions of the type | name | keys | comment rows.")
old_mod=s[s.index("The `seq` module is different: sequence diagrams are not graphs"):]
old_mod=old_mod[:old_mod.index("\n")]
rep(old_mod, "The `seq` module is different: sequence diagrams are not graphs, so it skips the Sugiyama pipeline entirely. `seq::scene()` computes a linear layout — columns from declaration order (gaps widened until every message label and note fits), one row per statement top-down — and returns every box (header and footer), lifeline, message polyline, note, activation bar, and frame in final coordinates; `seq::to_svg()` serialises any scene. GUI painters get arrowheads from `seq::head` and actor stick figures from `seq::actor_figure`, as plain geometry.")
rep("`pie::to_svg()` serialises them (a ~100% slice becomes a real `<circle>`, since a single SVG arc cannot sweep 360°).",
    "`pie::to_svg()` serialises them (a ~100% slice becomes a real `<circle>`, since a single SVG arc cannot sweep 360°). Each `PieLabel` says whether it sits inside its wedge or outside on a leader (with the leader polyline), so a host can paint the same labels.")
open(p,'w').write(s)

# docs index: sequence row
p2='/home/user/flowmaid/docs/docs/index.html'
d=open(p2).read()
old="""      <td>Participants/actors, 8 arrow types, activations, notes,
      <code>autonumber</code>, <code>loop/opt/alt/par</code> frames</td></tr>"""
assert d.count(old)==1
d=d.replace(old,"""      <td>Participants/stick-figure actors (mirrored at the bottom), 8 arrow types,
      activations, notes, <code>autonumber</code>, <code>loop/opt/alt/par</code> frames</td></tr>""")
open(p2,'w').write(d)
print("ok")
