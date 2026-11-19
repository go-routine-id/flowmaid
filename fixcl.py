import re,sys
p=sys.argv[1]; s=open(p).read()
def rep(old,new):
    global s
    pat=r'\s+'.join(re.escape(w) for w in old.split())
    m=list(re.finditer(pat,s))
    assert len(m)==1,(len(m),old[:60])
    s=s[:m[0].start()]+new+s[m[0].end():]
# Highlights (#5 + #8)
rep("""as does every edge of a drawing with
  more than 400 edges (past the router's cap); back-edge lanes (on the
  axes already) are unchanged, and class and ER output is byte-identical.""",
"""as do the edges of a drawing with more
  than 400 edges (past the router's cap), but for an S whose ends lie
  under 28 px apart across, which is squared there too; back-edge lanes,
  on the axes already, are not redrawn, though a lane can now take the
  other side (bench-75's static `dash -> user` runs over the top through
  `edge` and `app`, as its `route()` lane already did; see Known costs
  under Changed); class and ER output is byte-identical.""")
# #6 track growth only under the cap
rep("""The layout grows a gap by the
  tracks its steps need (two at the plain gap, one more per further step
  one way).""",
"""The layout grows a gap by the
  tracks its steps need, in a drawing of at most 400 edges (two at the
  plain gap, one more per further step one way).""")
# #5 B10 entry
rep("""Past the router's 400-edge
  cap, where the router and its nudge leave a drawing as built, the
  builders' curves stay too (a 531-edge chart's `route()` 25.7 -> 2.6 ms).""",
"""Past the router's
  400-edge cap, where the router and its nudge leave a drawing as built,
  the builders' curves stay too and no gap grows for tracks (a 531-edge
  chart's `route()` 25.7 -> 2.6 ms), but for an S under 28 px across,
  which the last stage squares whatever the size.""")
# #2 bezier fallback
rep("""`bezier` only a fallback
  (a cubic between its ends with its first and last corners as
  controls), so pick == render as before.""",
"""`bezier` only a fallback
  (a cubic between its ends with its first and last corners as
  controls, or the straight line between them for an edge the router
  or its nudge redrew), so pick == render as before.""")
# #8 B10 entry lanes
rep("""Explicit `curve:` / `linkStyle … interpolate`,
  state note card links, self-loops, back-edge lanes, invisible `~~~`
  links and class / ER scenes are unchanged (class and ER output is
  byte-identical).""",
"""Explicit
  `curve:` / `linkStyle … interpolate`, state note card links,
  self-loops, invisible `~~~` links and class / ER scenes are unchanged
  (class and ER output is byte-identical). Back-edge lanes are not
  redrawn, but `route()` now counts each border of a frame holding
  neither end that a lane's legs would cross when it picks the lane's
  side, and a lane can land on the other side from 0.37.0 (bench-75's
  static `dash -> user` now runs over the top, as its `route()` lane
  did).""")
# #7
rep("""two more edge-frame pairs there
  (2 -> 4, lanes taken over the top through `app`);""",
"""two more edge-frame pairs there
  (2 -> 4: `dash -> user`'s lane now runs over the top, through `edge`
  and `app`);""")
# #1
rep("""against the phase-B baseline
  (5f318a0, before 0.33.0) it is 1.4-2.0x, mostly from the layout and edge
  work released in 0.34.0-0.37.0 (advanced 2.0x, its labels, ports and
  nudge over 18 squared paths).""",
"""against the phase-B baseline
  (5f318a0, before 0.33.0) it is up to 1.9x (fastest of interleaved
  runs: 1.3-1.9x on eight of the eleven, advanced the slowest, its
  labels, ports and nudge over 18 squared paths; lr and fanout take
  less), mostly from the layout and edge work released in 0.34.0-0.37.0.""")
open(p,'w').write(s)
