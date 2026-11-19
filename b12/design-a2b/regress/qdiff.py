# qdiff.py A.tsv B.tsv [mode]: per-file changed columns between two quality TSVs (TOTAL rows skipped)
import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if r and r[0]!='TOTAL'}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
mode=sys.argv[3] if len(sys.argv)>3 else None
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic'}
for k in sorted(set(a)|set(b)):
    if mode and k[1]!=mode: continue
    ra,rb=a.get(k,{}),b.get(k,{})
    ch=[f"{c} {ra.get(c)}->{rb.get(c)}" for c in h[2:] if c not in skip and ra.get(c)!=rb.get(c)]
    if ch: print(k[0],k[1],'; '.join(ch))
