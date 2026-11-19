import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
hb,b=load(sys.argv[1]); hc,c=load(sys.argv[2])
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic'}
for k in c:
    o=b.get(k)
    if not o: print(k,'missing in pre'); continue
    diffs=[]
    for col in hc:
        if col in skip or col in ('file','mode'): continue
        if o.get(col)!=c[k].get(col): diffs.append(f"{col}:{o.get(col)}->{c[k].get(col)}")
    if diffs: print(k[0],k[1],' '.join(diffs))
