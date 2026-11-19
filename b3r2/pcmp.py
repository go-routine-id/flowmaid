import sys,csv
def load(p):
    r=list(csv.reader(open(p),delimiter='\t'))
    h=r[0]; return h,{(x[0],x[1]):dict(zip(h,x)) for x in r[1:]}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic','nodes','edges','frames'}
higher_better={'min_node_gap','min_edge_gap','ink'}
for k in a:
    if k not in b: continue
    out=[]
    for c in h[2:]:
        if c in skip: continue
        try: x=float(a[k][c]); y=float(b[k][c])
        except: continue
        if abs(x-y)<0.05: continue
        worse = (y<x) if c in higher_better else (y>x)
        out.append(('!' if worse else '')+f"{c} {x:g}->{y:g}")
    if out: print(k[0],k[1],' '.join(out))
