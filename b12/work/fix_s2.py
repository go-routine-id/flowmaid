import sys
p=sys.argv[1]; s=open(p).read()
a="""  the stage profile below). Its positions hold two staggered bands the spike's single greedy sweep does not clear in
  one pass."""
b="""  the stage profile below): the single greedy sweep leaves overlaps there that the router takes up again."""
assert a in s; s=s.replace(a,b)
a="""  spec expects: the experiment's positions give its seven bands gaps of 60 to 125 px between member boxes (one of
  287 px) for up to a dozen tracks each, so corridor rows collide and `repair` takes them all up again (7.5 ms of
  the 12)."""
b="""  spec expects: the experiment's positions give its seven bands gaps of 60 to 125 px between member boxes (one of
  288 px), less the frames' padding, for 75 edges; `repair` is 7.5 ms of the 12 there, so most corridors are
  searched again."""
assert a in s; s=s.replace(a,b)
open(p,'w').write(s)
