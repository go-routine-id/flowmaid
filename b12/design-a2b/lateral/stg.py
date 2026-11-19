# stg.py: read probe stderr, print per edge only the stages where the path changed
import sys,re,collections
last={}; out=collections.OrderedDict()
for l in open(sys.argv[1]):
    m=re.match(r'^(\S+) \[([^\]]+)\] (.*)$',l.rstrip())
    if not m: 
        if l.startswith(('NODE','CLUSTER')): print(l.rstrip())
        continue
    e,st,rest=m.groups()
    if last.get(e)!=rest:
        out.setdefault(e,[]).append((st,rest)); last[e]=rest
for e,v in out.items():
    print('==',e)
    for st,r in v: print(f'  [{st}] {r}')
