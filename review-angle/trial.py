import sys,re
src=open('wt/rel39/README.md',encoding='utf-8').read()
t=src
R=[]
def rep(tag,old,new):
    global t
    n=t.count(old)
    if n!=1:
        print('!! MISMATCH',tag,n); return
    before=len(t.encode()); t=t.replace(old,new); R.append((tag,before-len(t.encode())))

# F1 line 181
rep('F1-181',"Each section names what flowmaid supports, what it does not support yet, and what it adds.",
"Each section names the part of that syntax flowmaid supports and what flowmaid adds. A **Not yet** line lists the gaps known so far; syntax a section does not name may not be supported.")
# theme row
rep('theme-row',"| `--theme <name>` | Paint in one of mermaid's themes: `default`, `neutral`, `dark` or `forest`. `flowmaid`, the default, is the semantic palette. `--theme` beats a theme the source names, which applies otherwise. See [Themes](#themes). |",
"| `--theme <name>` | `flowmaid` (the default), `default`, `neutral`, `dark` or `forest`. Beats a theme the source names. See [Themes](#themes). |")
# themes
rep('themes-from',"- **From the diagram.** A diagram can name its theme the way mermaid reads it: `%%{init: {\"theme\": \"neutral\"}}%%` (or `%%{initialize: …}%%`, on one line or several), or `theme:` under a YAML frontmatter's `config:`. `render_svg` and `render_svg_advanced` honour it. The directive wins over the frontmatter, as in mermaid.",
"- **From the diagram.** `render_svg` and `render_svg_advanced` honour a theme the source names, in an `init` directive or under the frontmatter's `config:`, the way mermaid reads it.")
rep('themes-contrast',"Two exceptions: the faint hairlines round label chips and between ER rows (`#cccccc`, `#5a5d63` in the dark theme), and, on a white canvas, the amber chart accent (`#d99114`, 2.6:1) that git lanes and mindmap branches keep.",
"Two exceptions: the faint hairlines round label chips and between ER rows, and the amber accent of git lanes and mindmap branches on a white canvas (2.6:1).")
rep('themes-palette',"- **The `flowmaid` palette.** Shape determines colour (stadium green, diamond amber, circle violet, …; see the `style` module), and ER entities cycle through a stable accent palette. Text on accent fills is picked for WCAG AA contrast (`style::text_on`).",
"- **The `flowmaid` palette** lives in the `style` module. ER entities cycle through a stable accent palette, and text on accent fills is picked for WCAG AA contrast (`style::text_on`).")
# wrap width
rep('wrap',"wraps at 200px, in flowcharts and state diagrams alike. That is mermaid 11's `flowchart.wrappingWidth` default; mermaid 12.2's is 120. Set the width with `%%{init: {\"flowchart\": {\"wrappingWidth\": 120}}}%%` or `wrappingWidth: 120` under `config.flowchart` in the frontmatter.",
"wraps at 200px, in flowcharts and state diagrams alike (mermaid 12.2 wraps at 120). Set it with `flowchart.wrappingWidth`, in an `init` directive or the frontmatter's `config:`.")
# curve names
rep('curves',"`curve:` takes all thirteen mermaid curve names: `basis`, `linear`, `rounded`, `step`, `stepBefore`, `stepAfter`, `bumpX`, `bumpY`, `natural`, `monotoneX`, `monotoneY`, `cardinal` and `catmullRom`. The last five are drawn as one Catmull-Rom spline.",
"`curve:` takes all thirteen mermaid curve names. `natural`, `monotoneX`, `monotoneY`, `cardinal` and `catmullRom` are drawn as one Catmull-Rom spline.")
# subgraph direction
rep('subdir',"A block's `direction` follows mermaid's rule. It holds while no edge joins one of the block's nodes to a node outside it; edges to or from the block itself are fine. Otherwise the block takes the direction round it. One case differs:",
"A block's `direction` follows mermaid's rule: it holds only while no edge joins one of the block's nodes to a node outside it. One case differs:")
# linkStyle
rep('links',"- **Links.** `linkStyle <n>`, a comma list of indices (no spaces in the list) or `linkStyle default` applies `stroke` (the arrow, cross and circle markers take the same colour), `stroke-width` (0 to 1000px), `stroke-dasharray`, `fill` and `color` (edge label text).",
"- **Links.** `linkStyle` applies `stroke`, `stroke-width` (0 to 1000px), `stroke-dasharray`, `fill` and `color` (edge label text). An index list takes no spaces. The arrow, cross and circle markers take the stroke's colour, with two deliberate differences from mermaid (see [Differences from mermaid](#differences-from-mermaid)).")
rep('markers',"- **Markers.** Two deliberate differences in how markers are coloured are listed under [Differences from mermaid](#differences-from-mermaid).\n","")
# click
rep('click',"- **Supported:** a url, then an optional tooltip, then an optional `_self` / `_blank` / `_parent` / `_top` target, for one node or several.",
"- **Supported:** a url with an optional tooltip and target, for one node or several.")
# state
rep('state',"`[*]` start and end states, labelled transitions, bare-id declarations, `state \"Long title\" as id`, description lines `id : text`, nested composite blocks and transitions to or from a composite, a composite's `direction`, `<<choice>>`, `<<fork>>` / `<<join>>`, and notes (`note right of X`, `note left of X`, on one line or as an `end note` block).",
"start and end states, labelled transitions, state declarations with titles and descriptions, nested composites (with their own `direction`, and as the end of a transition), `<<choice>>`, `<<fork>>` / `<<join>>`, and `note left of` / `note right of` notes, on one line or as a block.")
# class
rep('class',"class blocks and inline `Name : +member` members with `+ - # ~` visibility; fields and methods with return types, `~T~` generics and the `$` / `*` classifiers; several classes on one line; hyphenated and backtick names; `<<stereotype>>` annotations; all UML relations, on solid or dashed lines, in either direction, with cardinalities and labels; two-way relations; lollipop interfaces; `direction`; notes (`note \"text\"`, `note for Class \"text\"`).",
"class blocks and inline members, with visibility, return types, generics and the `$` / `*` classifiers; several classes on one line; hyphenated and backtick names; `<<stereotype>>` annotations. All UML relations, solid or dashed, in either direction or two-way, with cardinalities and labels; lollipop interfaces. `direction`, and notes with or without a class.")
# ER
rep('er1',"all crow's foot cardinalities, as symbols or spelled out the way mermaid spells them; identifying and non-identifying lines, including `to` and `optionally to`; relationship labels; entity blocks with `type name [PK|FK|UK] [\"comment\"]` rows; quoted entity names; aliases, also inside a relationship; `direction TB|BT|LR|RL`.",
"all crow's foot cardinalities and both line kinds, as symbols or in mermaid's words; optional relationship labels; entity blocks with types, `PK` / `FK` / `UK` keys and comments; quoted entity names; aliases, also inside a relationship; `direction`.")
rep('er2',"- **Labels.** The relationship label is optional. A label is kept clear of the crow's feet and of neighbouring lines.",
"- **Labels** are kept clear of the crow's feet and of neighbouring lines.")
# sequence
rep('seq',"`participant` / `actor` with `as` aliases, and implicit participants; the eight arrows `->>`, `-->>`, `->`, `-->`, `-x`, `--x`, `-)` and `--)`; self-messages; `autonumber`; `Note over A,B`, `Note over A`, `Note left of A` and `Note right of A`; `activate` / `deactivate` and the `+` / `-` arrow shorthand; the frames `loop`, `opt`, `alt` + `else` and `par` + `and`.",
"participants and actors with aliases, and implicit participants; the eight arrows `->>`, `-->>`, `->`, `-->`, `-x`, `--x`, `-)` and `--)`; self-messages; `autonumber`; notes over one or two participants and left or right of one; activation, by keyword or the `+` / `-` shorthand; the frames `loop`, `opt`, `alt` and `par`.")
# pie
rep('pie',"all header forms (`pie`, `pie showData`, `pie title …`, `pie showData title …`), a standalone `title …` line, and `\"Quoted Label\" : value` rows.",
"every header form (with or without `showData` and a title), a standalone `title` line, and the label and value rows.")
# mindmap
rep('mm1',"- **Supported**, of [mermaid's mindmap syntax](https://mermaid.js.org/syntax/mindmap.html): the indentation tree and mermaid's shape wrappers, which are none (the default box with an underline), `[square]`, `(rounded)`, `((circle))`, `{{hexagon}}`, `))bang((` and `)cloud(`.",
"- **Supported subset** of [mermaid's mindmap syntax](https://mermaid.js.org/syntax/mindmap.html): the indentation tree, and the default, square, rounded, circle, hexagon, bang and cloud shapes.")
rep('mm2',"A map too big for the layout search's work budget, or with more than 120 branches on the root, is laid out on rings round the root instead: much sparser, with the same guarantee.",
"A map too big for the layout search, or with more than 120 branches on the root, is laid out on rings round the root: much sparser, with the same guarantee.")
# sankey
rep('sk1',"`source,target,value` rows of exactly three fields; double-quoted fields, where `\"\"` is one literal quote, so a label may contain commas; `%%` comments and blank lines. `sankey` is accepted as a spelling variant of `sankey-beta`.",
"the three-field CSV rows, with double-quoted fields and `\"\"` for a quote; `%%` comments and blank lines. `sankey` is accepted as a spelling of `sankey-beta`.")
rep('sk2',"- **Config** is read from mermaid's YAML frontmatter under `config.sankey`, and every default matches mermaid's `config.schema.yaml`. The keys are `width`, `height`, `linkColor` (`gradient`, `source`, `target` or any CSS colour), `nodeAlignment` (`justify`, `left`, `right`, `center`), `showValues`, `prefix`, `suffix`, `nodeWidth`, `nodePadding`, `nodeColors` (node label → CSS colour) and `labelStyle` (`legacy` or `outlined`). An unknown key is skipped and a value that does not parse keeps its default, so config for other diagram types in a shared file never breaks the render.",
"- **Config** is read from the frontmatter under `config.sankey`, with mermaid's defaults. The keys are `width`, `height`, `linkColor`, `nodeAlignment`, `showValues`, `prefix`, `suffix`, `nodeWidth`, `nodePadding`, `nodeColors` and `labelStyle`. An unknown key is skipped, and a value that does not parse keeps its default.")
# advance
rep('adv',"This diagram kind has no mermaid header, so it is outside the parity roadmap, and the diagram counts elsewhere in this document refer to mermaid-syntax types only.",
"It has no mermaid header, so it is outside the parity roadmap and is not counted among the twelve diagram types.")
# interactive
rep('paths',"- **Edge paths.** A flowchart or state edge drawn on the axes (see [Edges](#edges)) comes as `SceneEdge::waypoints`, which a painter's spline turns through as rounded corners. Its `SceneEdge::bezier` is only a fallback that does not follow the path: read `waypoints[0]` and `waypoints.last()` for the drawn ends, and the first and last legs for the end tangents and arrowhead direction. An edge with empty `waypoints` (a straight edge, a self-loop) is its single cubic `bezier`.",
"- **Edge paths.** A flowchart or state edge drawn on the axes (see [Edges](#edges)) comes as `SceneEdge::waypoints`, which a painter's spline turns through as rounded corners. Read the drawn ends from `waypoints[0]` and `waypoints.last()`, and the arrowhead direction from the first and last legs. `SceneEdge::bezier` does not follow such a path: it is the edge only when `waypoints` is empty (a straight edge, a self-loop).")
rep('ends',"- **Edge ends.** An edge ends on its node's border with its end tangent square to the side, so a host draws its own arrowhead there. Trimming the stroke under the arrowhead, and the hops and gaps at crossings, are done by flowmaid's SVG painter only. `SceneEdge::waypoints` hold the plain path, and a host that paints the scene itself draws its crossings its own way.",
"- **Edge ends.** An edge ends on its node's border, square to the side, so a host draws its own arrowhead there. Trimming the stroke under the arrowhead, and the hops and gaps at crossings, are done by flowmaid's SVG painter only: the waypoints hold the plain path.")
rep('fonts',"- **Real fonts.** Embedders using a real font can inject its metrics (`layout::intrinsic_size_with` / `intrinsic_size_wrapped`, `scene::scene_sized_with`, `TEXT_CALIBRATION`). Call `Graph::set_wrapping_width` (`None` = never wrap), size nodes with `layout::intrinsic_size_wrapped(node, g.wrapping_width(), measure)`, and pass the same `measure` to `scene::scene_sized_with` / `scene::route_sized_with`. The scene's label lines are then the ones the boxes were sized for. `scene_sized` / `route_sized` break lines with the built-in estimate, as markdown labels always do.",
"- **Real fonts.** Embedders using a real font can inject its metrics. Size nodes with `layout::intrinsic_size_wrapped(node, g.wrapping_width(), measure)` and pass the same `measure` to `scene::scene_sized_with` / `scene::route_sized_with`. `Graph::set_wrapping_width` sets the wrap width (`None` = never wrap). `scene_sized` / `route_sized` break lines with the built-in estimate, as markdown labels always do.")
old_other=t[t.index("- **ER and class** diagrams have the same"):t.index("### Apps built on it")]
new_other="""- **ER and class** diagrams have the same `scene` / `route` / `to_svg` triple (`er::`, `class::`). Each entity or class is one node, and each relationship one edge.
- **Glyph helpers.** `er::glyph` and `er::end_glyphs` return the crow's feet the SVG draws. `class::rel_head`, `class::rel_tail` and `class::card_spots` give the UML end glyphs and cardinality positions. `seq::head` and `seq::actor_figure` give sequence arrowheads and stick figures.
- **State** diagrams use the flowchart API directly: the parser maps them straight onto `Graph`.
- **Mindmaps** have `mindmap::scene` / `mindmap::route`.
- **Static diagrams** (sequence, pie, journey, timeline, sankey, git graph, architecture) have a `scene()` and a `to_svg()` but no `route()`.
- **Git graphs and architecture** diagrams also expose a generic `Scene`, so terminal and GUI hosts can draw (and pick on) them without knowing the source syntax.

"""
rep('other',old_other,new_other)
# how it works
rep('order',"- On a graph of up to 400 nodes plus edges, the drawing depends much less on the order the edge lines happen to be written in.",
"- The order the edge lines are written in matters much less on a graph of up to 400 nodes plus edges than on a larger one.")
# roadmap
rep('road1'," Contributions are welcome, and every item has its own issue. Flowchart syntax follows mermaid **12.2.0**."," Contributions are welcome, and every item has its own issue.")
rep('road2',"- **Diagram types not drawn yet**, tracked on the board: `swimlane-beta` · `gantt` · `quadrantChart` · `requirementDiagram` · `C4` · `zenuml` · `xychart` · `block` · `packet` · `kanban` · `radar` · `eventmodeling` · `treemap` · `venn` · `ishikawa` · `wardley` · `cynefin` · `treeview` · `usecase` · `railroad` · `agentflow`.",
"- **Diagram types not drawn yet** are tracked on the board. [Diagram types](#diagram-types) lists their headers and what each does today.")
rep('pick-dup'," `edge_at` follows the painted stroke, a crossing hop's arc included. Picking costs microseconds."," `edge_at` follows the painted stroke, a crossing hop's arc included.")

# --- structural edits ---
# Flowcharts: Not yet line
rep('fc-notyet',"- **Other:** `click` links, and `accTitle` / `accDescr` (see [Accessibility](#accessibility)).\n",
"- **Other:** `click` links, and `accTitle` / `accDescr` (see [Accessibility](#accessibility)).\n- **Not yet:** markdown other than bold and italic, `click` callbacks (accepted and ignored), escaped quotes (`\\\"`), and MathML for `$$…$$` math. Some shapes are drawn as look-alikes and some metadata keys are not drawn (see [Node shapes](#node-shapes)).\n")
# Git graphs: Not yet second
ny="- **Not yet:** `accTitle` / `accDescr` are parsed but not drawn.\n"
lanes="- **Lanes** take eight accent colours in order, so from the ninth lane on the colours repeat.\n"
assert t.count(ny)==1 and t.count(lanes)==1
t=t.replace(ny,"").replace(lanes,ny+lanes)
# State: paragraph -> bullet
para="`stateDiagram-v2` (and the v1 `stateDiagram` header) rides the flowchart pipeline: states are rounded nodes, transitions are labelled edges, and composite states are the same nested clusters subgraphs use. So dragging, custom positions, picking and SVG export all work unchanged.\n\n"
assert t.count(para)==1
t=t.replace(para,"")
rep('state-bullet',"- **Direction.** A composite's `direction` follows the [subgraph rule](#subgraphs).\n",
"- **Same pipeline as flowcharts.** States are rounded nodes, transitions are labelled edges and composites are the nested clusters subgraphs use. So dragging, custom positions, picking and SVG export all work unchanged.\n- **Direction.** A composite's `direction` follows the [subgraph rule](#subgraphs).\n")
# How it works: move the two lists to Flowcharts > Layout
i=t.index("What the layout does that shows in a drawing:"); j=t.index("## Performance")
block=t[i:j]; t=t[:i]+t[j:]
k=t.index("## State diagrams")
t=t[:k]+"### Layout\n\n"+block+t[k:]
# Interactive: host-painter bullets after the entry points
def take(start):
    global t
    i=t.index(start); j=t.index("\n",i)+1; b=t[i:j]; t=t[:i]+t[j:]; return b
b1=take("- **Edge paths.**"); b2=take("- **Curves.** A non-default"); b3=take("- **Edge ends.**")
k=t.index("- **Links.** Nodes with a mermaid `click`")
t=t[:k]+b1+b2+b3+t[k:]
k=t.index("- **Compact layout.**"); j=t.index("\n",k)+1; cl=t[k:j]; t=t[:k]+t[j:]
k=t.index("- **Edge paths.**"); t=t[:k]+cl+t[k:]

open('review-angle/README.trial.md','w',encoding='utf-8').write(t)
tot=0
for tag,d in R: print(f"{d:5d}  {tag}"); tot+=d
print('total saved',tot,'bytes ->',len(t.encode()),'bytes,',t.count('\n'),'lines')
