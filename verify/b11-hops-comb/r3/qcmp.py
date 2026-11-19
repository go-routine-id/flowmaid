import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:]}
hb,b=load(sys.argv[1]); hc,c=load(sys.argv[2])
timing={'route_ms','scene_ms','drag_ms','drag_mean','drag_over'}
skip={'nodes','edges','frames'}
for k in c:
    if k not in b: print('NEW',k); continue
    diffs=[]
    for col in hc[2:]:
        if col in timing or col in skip: continue
        if col not in b[k]:
            if float(c[k][col] or 0)!=0: diffs.append(f"{col}=+{c[k][col]}")
            continue
        x,y=b[k][col],c[k][col]
        if x!=y:
            try:
                fx,fy=float(x),float(y)
                if abs(fx-fy)<1e-9: continue
            except: pass
            diffs.append(f"{col}:{x}->{y}")
    if diffs: print(k[0],k[1],'  '.join(diffs))
print('--- timing (pre -> now)')
for k in c:
    if k in b:
        print(k[0],k[1],' '.join(f"{t}:{b[k].get(t)}->{c[k].get(t)}" for t in ['scene_ms','route_ms','drag_ms','drag_mean','drag_over']))
