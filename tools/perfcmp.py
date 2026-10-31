import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,[dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)]
hb,b=load(sys.argv[1]); hc,c=load(sys.argv[2])
key=lambda r:(r.get(hb[0]),r.get('mode'))
bm={key(r):r for r in b}
cols=[x for x in ['defects','route_ms','scene_ms','drag_ms','drag_mean','drag_over','crossings'] if x in hc]
print('file\tmode\t'+'\t'.join(f'{x}(base->now)' for x in cols))
for r in c:
    o=bm.get(key(r),{})
    print(f"{r.get(hc[0])}\t{r.get('mode')}\t"+'\t'.join(f"{o.get(x,'?')}->{r.get(x)}" for x in cols))
