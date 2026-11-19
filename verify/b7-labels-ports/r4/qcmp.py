import csv,sys
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)},h
a,h=load(sys.argv[1]); b,_=load(sys.argv[2])
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic','nodes','edges','frames'}
for k in a:
    if k not in b: continue
    diffs=[]
    for c in h[2:]:
        if c in skip: continue
        x,y=a[k].get(c),b[k].get(c)
        if x!=y:
            try:
                if abs(float(x)-float(y))<1e-9: continue
            except: pass
            diffs.append(f"{c}:{x}->{y}")
    if diffs: print(k[0],k[1],' '.join(diffs))
