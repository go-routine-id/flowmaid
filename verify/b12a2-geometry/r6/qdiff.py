import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:]}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic'}
for k in sorted(set(a)|set(b)):
    ra,rb=a.get(k,{}),b.get(k,{})
    ch=[]
    for c in h[2:]:
        if c in skip: continue
        va,vb=ra.get(c),rb.get(c)
        if va!=vb: ch.append(f"{c} {va}->{vb}")
    if ch: print(k[0],k[1],'; '.join(ch))
