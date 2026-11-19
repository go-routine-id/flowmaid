import sys
p=sys.argv[1]; s=open(p).read()
def rep(old,new):
    global s
    assert s.count(old)==1,(s.count(old),old[:70]); s=s.replace(old,new)
rep("its label on that step, or, where the gap has room for the step between the source and the label (as in a decision fan whose gap grew for its tracks), the step passes above the label, as mermaid draws a labelled branch,",
    "its label on that step, or, where the gap has room for it (as in a decision fan whose gap grew for its tracks), the step runs between the source and the label, as mermaid draws a labelled branch,")
rep("A drawing of more than 400 edges, past the router's cap, keeps the earlier curves on every edge (S-shaped cubics, bowed twins and splines through a long edge's dummies).",
    "A drawing of more than 400 edges, past the router's cap, keeps the earlier curves (S-shaped cubics, bowed twins and splines through a long edge's dummies) and the plain gaps between its ranks, but for an S whose ends lie under 28 px apart across, which is drawn straight (within a px) or as a rounded Z there too.")
rep("or, for an edge the router rerouted, the straight line between its ends)",
    "or, for an edge the router rerouted or its channel nudge redrew, the straight line between its ends)")
rep("as on a labelled edge's short first step)",
    "as on a labelled edge's short first leg, from its source to its step)")
open(p,'w').write(s)
p=sys.argv[2]; s=open(p).read()
rep("""    /// Bezier points: start, control1, control2, end. This is the
    /// single-curve form used when `waypoints` is empty (adjacent
    /// layers, free-routed edges) and as the fallback everywhere. The
    /// ends lie on the node borders;""",
"""    /// Bezier points: start, control1, control2, end. With `waypoints`
    /// empty this single cubic is the edge: a straight edge, a self-loop,
    /// or a builder's curve between adjacent ranks that no later stage
    /// redraws (one with its own `curve`, an ER edge the router leaves as
    /// built, any such edge in a drawing past the router's 400-edge cap).
    /// With `waypoints` set it is only a fallback that does not follow
    /// the path. The ends lie on the node borders;""")
rep("""    /// Routed polyline (final coords) a long edge threads through — its
    /// node-boundary start, the per-layer channel points, and its end.
    /// Empty = draw `bezier`. A painter splines a smooth curve through
    /// these so the edge stays in its channel instead of cutting across.""",
"""    /// Routed polyline (final coords) from the edge's node-boundary start
    /// to its end: a flowchart or state edge drawn on the axes (each
    /// corner flanked by the points a spline rounds it through), a
    /// back-edge's lane, a route the router or a class or ER stage drew,
    /// or a long edge's channel points where its curve is kept (an
    /// explicit `curve`, a drawing past the router's 400-edge cap).
    /// Empty = draw `bezier`. A painter splines a smooth curve through
    /// these so the edge stays on its path instead of cutting across.""")
open(p,'w').write(s)
