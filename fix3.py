import re,sys
def rep(p, old, new, ws=False):
    s=open(p).read()
    if ws:
        pat=r'\s+'.join(re.escape(w) for w in old.split()); m=list(re.finditer(pat,s))
        assert len(m)==1,(p,len(m),old[:60]); s=s[:m[0].start()]+new+s[m[0].end():]
    else:
        assert s.count(old)==1,(p,s.count(old),old[:60]); s=s.replace(old,new)
    open(p,'w').write(s)
R,C,SC=sys.argv[1:4]
PART_R=" The count is taken per laid-out part: each subgraph laid out on its own in its `direction` (see [Subgraphs](#subgraphs)), and the rest of the drawing round it, so a larger drawing whose parts each hold 400 or fewer edges is drawn on the axes."
rep(R, "which is drawn straight (within a px) or as a rounded Z there too.",
       "which is drawn straight (within a px) or as a rounded Z there too."+PART_R)
rep(R, "(a cubic from the path's start to its end with its first and last corners as controls, or, for an edge the router rerouted or its channel nudge redrew, the straight line between its ends)",
       "(a cubic from the path's start to its end whose controls only roughly follow it: where the axis stage put the first and last corners, or for a back-edge lane the ends of its run along the lane, and left there when the ports stage later moves an end; or the straight line between its ends for an edge the router rerouted, its channel nudge redrew or its spacing pass moved clear; rely on its ends only)")
rep(R, "in a drawing of up to 400 edges, every flowchart",
       "in a drawing (or a part laid out on its own in its `direction`) of up to 400 edges, every flowchart")
rep(C, """as do the edges of a drawing with more
  than 400 edges (past the router's cap), but for""",
"""as do the edges of a drawing with more
  than 400 edges (past the router's cap; counted per part, each subgraph
  laid out on its own in its `direction` and the rest round them), but for""", ws=True)
rep(C, """in a drawing of at most 400 edges (two at the
  plain gap,""", """in a drawing, or a part laid out on its own,
  of at most 400 edges (two at the plain gap,""", ws=True)
rep(C, """Past the router's
  400-edge cap, where the router and its nudge leave a drawing as built,""",
"""Past the router's
  400-edge cap (counted per part a `direction` lays out on its own),
  where the router and its nudge leave a drawing as built,""", ws=True)
rep(C, """`bezier` only a fallback
  (a cubic between its ends with its first and last corners as
  controls, or the straight line between them for an edge the router
  or its nudge redrew), so pick == render as before.""",
"""`bezier` only a fallback
  whose controls only roughly follow the path (a cubic between its ends,
  its controls where the axis stage put the first and last corners, or
  the straight line between the ends for an edge the router, its nudge or
  its spacing pass redrew; rely on its ends only), so pick == render as
  before.""", ws=True)
rep(SC, "any such edge in a drawing past the router's 400-edge cap).",
        "any such edge in a drawing past the router's 400-edge cap,\n    /// counted per part a `direction` lays out on its own).")
rep(SC, "explicit `curve`, a drawing past the router's 400-edge cap).",
        "explicit `curve`, a drawing past the router's 400-edge cap,\n    /// counted per part a `direction` lays out on its own).")
