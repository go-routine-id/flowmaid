import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):r for r in rows[1:] if len(r)>2}
h,a=load(sys.argv[1]); h2,b=load(sys.argv[2])
assert h==h2
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','static_ms'}
for k in sorted(a):
    if k not in b: continue
    diffs=[]
    for i,c in enumerate(h):
        if i<2 or c in skip or c.endswith('_ms'): continue
        try: x=float(a[k][i]); y=float(b[k][i])
        except: continue
        if abs(x-y)>1e-9: diffs.append(f"{c}:{a[k][i]}->{b[k][i]}")
    if diffs: print(k[0],k[1],' '.join(diffs))
