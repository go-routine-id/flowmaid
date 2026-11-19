import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
h,a=load(sys.argv[1]); h2,b=load(sys.argv[2])
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','file','mode'}
for k in a:
    if k not in b: print('missing',k); continue
    diffs=[]
    for c in h:
        if c in skip or c not in b[k]: continue
        if a[k][c]!=b[k][c]: diffs.append(f"{c}:{a[k][c]}->{b[k][c]}")
    if diffs: print(k[0],k[1],' '.join(diffs))
print('cols only in new:', [c for c in h2 if c not in h])
