import csv,sys
TIME={'route_ms','scene_ms','drag_ms','drag_mean','drag_over'}
def load(p):
    rows={}
    with open(p) as f:
        r=csv.reader(f,delimiter='\t'); h=next(r)
        for x in r:
            rows[(x[0],x[1])]=dict(zip(h,x))
    return h,rows
tot=0;nd=0
for s in sys.argv[3:]:
    h,a=load(f"{sys.argv[1]}-{s}.tsv"); _,b=load(f"{sys.argv[2]}-{s}.tsv")
    for k in sorted(set(a)|set(b)):
        tot+=1
        if k not in a or k not in b: print(s,k,'missing'); nd+=1; continue
        d=[(c,a[k][c],b[k][c]) for c in h if c not in TIME and a[k].get(c)!=b[k].get(c)]
        if d: nd+=1; print(s,k[0],k[1],' '.join(f"{c}={x}->{y}" for c,x,y in d))
print('rows',tot,'rows differing (timing columns ignored)',nd)
