import sys
p = sys.argv[1]
s = open(p, encoding='utf-8').read()
lines = s.split('\n')

def find(prefix, start=0):
    hits = [i for i, l in enumerate(lines) if l.startswith(prefix) and i >= start]
    assert len(hits) == 1, (prefix, hits)
    return hits[0]

# ---- F18: Scene, drag and re-route ----
h = find('### Scene, drag and re-route')
end = find('See `examples/drag_sim.rs`.')
block = [l for l in lines[h+1:end] if l.strip()]
keys = ['- **`scene::scene(&graph)`**', '- **Edge paths.**', '- **Curves.**', '- **`scene::route(&graph, &positions)`**',
        '- **`scene::route_partial`**', '- **`scene::box_edge_bezier`**', '- **`scene::to_svg(&scene)`**',
        '- **Edge ends.**', '- **Links.**', '- **Compact layout.**', '- **Real fonts.**']
assert len(block) == len(keys), len(block)
d = {}
for k in keys:
    m = [b for b in block if b.startswith(k)]
    assert len(m) == 1, k
    d[k] = m[0]
d['- **Edge paths.**'] = ("- **Edge paths.** A flowchart or state edge drawn on the axes (see [Edges](#edges)) comes as `SceneEdge::waypoints`, "
    "which a painter's spline turns through as rounded corners. Read the drawn ends from `waypoints[0]` and `waypoints.last()`, "
    "and the arrowhead direction from the first and last legs. `SceneEdge::bezier` does not follow such a path: it is the edge only "
    "when `waypoints` is empty (a straight edge, a self-loop).")
d['- **Edge ends.**'] = ("- **Edge ends.** An edge ends on its node's border, square to the side, so a host draws its own arrowhead there. "
    "Trimming the stroke under the arrowhead, and the hops and gaps at crossings, are done by flowmaid's SVG painter only: "
    "the waypoints hold the plain path.")
d['- **Real fonts.**'] = ("- **Real fonts.** Embedders using a real font can inject its metrics. Size nodes with "
    "`layout::intrinsic_size_wrapped(node, g.wrapping_width(), measure)` and pass the same `measure` to "
    "`scene::scene_sized_with` / `scene::route_sized_with`. `Graph::set_wrapping_width` sets the wrap width (`None` = never wrap). "
    "`scene_sized` / `route_sized` break lines with the built-in estimate, as markdown labels always do.")
order = ['- **`scene::scene(&graph)`**', '- **`scene::route(&graph, &positions)`**', '- **`scene::route_partial`**',
         '- **`scene::box_edge_bezier`**', '- **`scene::to_svg(&scene)`**', '- **Compact layout.**',
         '- **Edge paths.**', '- **Curves.**', '- **Edge ends.**', '- **Links.**', '- **Real fonts.**']
lines[h+1:end] = [''] + [d[k] for k in order] + ['']

# ---- F19: Other diagram types ----
h = find('### Other diagram types')
end = find('### Apps built on it')
new = [
 "- **ER and class** diagrams have the same `scene` / `route` / `to_svg` triple (`er::`, `class::`). Each entity or class is one node, and each relationship one edge.",
 "- **Glyph helpers.** `er::glyph` and `er::end_glyphs` return the crow's feet the SVG draws. `class::rel_head`, `class::rel_tail` and `class::card_spots` give the UML end glyphs and cardinality positions. `seq::head` and `seq::actor_figure` give sequence arrowheads and stick figures.",
 "- **State** diagrams use the flowchart API directly: the parser maps them straight onto `Graph`.",
 "- **Mindmaps** have `mindmap::scene` / `mindmap::route`.",
 "- **Static diagrams** (sequence, pie, journey, timeline, sankey, git graph, architecture) have a `scene()` and a `to_svg()` but no `route()`.",
 "- **Git graphs and architecture** diagrams also expose a generic `Scene`, so terminal and GUI hosts can draw (and pick on) them without knowing the source syntax.",
]
old_block = [l for l in lines[h+1:end] if l.strip()]
assert len(old_block) == 9, len(old_block)
assert old_block[4] == new[3] and old_block[5] == new[4] and old_block[8] == new[5]
lines[h+1:end] = [''] + new + ['']

# ---- F20: move the two lists to "### Layout" at the end of Flowcharts ----
a = find('What the layout does that shows in a drawing:')
b = find('## Performance')
moved = lines[a:b]
while moved and not moved[-1].strip():
    moved.pop()
assert moved[-1].startswith('- Parallel edges'), moved[-1]
# cut, leaving exactly one blank line before "## Performance"
lines[a:b] = []
assert lines[a-1] == '' and lines[a].startswith('## Performance')
st = find('## State diagrams')
assert lines[st-1] == ''
lines[st:st] = ['### Layout', ''] + moved + ['']

open(p, 'w', encoding='utf-8').write('\n'.join(lines))
