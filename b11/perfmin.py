import sys,csv
# perfmin.py base.tsv run1.tsv run2.tsv ... : per-file min of timing columns over runs, flag budget
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
base=load(sys.argv[1]); runs=[load(p) for p in sys.argv[2:]]
bad=0
print("file           mode    route_ms(base->min)  scene_ms(base->min)  drag_ms(base->min)  verdict")
for k in runs[0]:
    b=base.get(k,{})
    out=[];v=[]
    for c in ['route_ms','scene_ms','drag_ms']:
        bv=float(b.get(c,0) or 0); mv=min(float(r[k][c]) for r in runs if k in r)
        out.append(f"{bv:.2f}->{mv:.2f}")
        if bv>0:
            lim = 2*bv if c=='scene_ms' else max(1.25*bv, bv+1.0)
            if mv>lim: v.append(f"{c} OVER {mv:.2f}>{lim:.2f}")
    if v: bad+=1
    print(f"{k[0][:14]:14} {k[1]:7} {out[0]:>18} {out[1]:>18} {out[2]:>18}  {' '.join(v) or 'ok'}")
print("OVER BUDGET rows:",bad)
