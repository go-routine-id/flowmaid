import sys
p=sys.argv[1]; s=open(p).read()
def rep(a,b):
    global s
    if a not in s:
        print("MISSING:", a[:60]); return
    s=s.replace(a,b)
rep("| 3.55 \\| 4.76 | 7.11 |","| 3.55 \\| 4.76 | 7.64 |")
rep("""2.82 | 2.83 ms. The worst frames are drags that push a node into another block's band, where the corridor classes
  fall back to today's builders; R6 (the kept edges of a drag) is not in the spike. B12.13 has to bring the worst
  frame under 5.69.""","""2.82 | 2.83 ms. Which frames are the worst was not looked into; the spike has no R6 (the kept edges of a drag) and
  falls back to today's builders whenever two ends overlap along the flow. B12.13 has to bring the worst frame under
  5.69.""")
rep("""  stage profile below). Its positions hold two staggered bands the spike's single greedy sweep does not clear in
  one pass.""","""  stage profile below): the single greedy sweep leaves overlaps there that the router takes up again.""")
rep("""`edge_node_x` is 0 everywhere, before and after. dense-lr keeps one edge through foreign frames: a chain whose
corridor column is taken on both sides of a block the experiment placed without a gap; B12.13's target there is
"not above the static value + 2".""","""`edge_node_x` is 0 everywhere, before and after. dense-lr keeps one edge through two foreign frames (cause not
looked into); B12.13's target there is "not above the static value + 2".""")
rep("""- The registry is linear in the edges of a gap; on bench-75 it is 0.3 ms of the call. It needs no index at these
  sizes.
""","""- The registry of taken rows and columns is a linear scan; its cost was not profiled apart from the rest of the
  builder (the whole call, builder included, is the `route_ms` above).
""")
open(p,'w').write(s)
