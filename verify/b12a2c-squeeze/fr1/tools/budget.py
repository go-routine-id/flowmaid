import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
base=load(sys.argv[1]); runs=[load(p) for p in sys.argv[2:]]
f=lambda r,k: float(r.get(k,0) or 0)
for key in sorted(runs[0]):
    if key not in base: continue
    b=base[key]
    for col,rule in (('drag_ms','d'),('route_ms','d'),('scene_ms','s')):
        bv=f(b,col)
        if bv==0: continue
        lim = max(1.25*bv, bv+1.0) if rule=='d' else 2*bv
        vals=[f(r[key],col) for r in runs if key in r]
        if all(v>lim for v in vals):
            print(f"OVER(all runs) {key[0]:18} {key[1]:6} {col:9} base {bv:.2f} lim {lim:.2f} runs {vals}")
        elif any(v>lim for v in vals):
            print(f"over(some)     {key[0]:18} {key[1]:6} {col:9} base {bv:.2f} lim {lim:.2f} runs {vals}")
