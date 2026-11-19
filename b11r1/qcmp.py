import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return h,[dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)]
ha,a=load(sys.argv[1]); hb,b=load(sys.argv[2])
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic','mode','ink'}
ka={(r[ha[0]],r['mode']):r for r in a}
cols=[c for c in hb[2:] if c not in skip and c in ha]
tot={}
worse={};better={}
for r in b:
    k=(r[hb[0]],r['mode']); o=ka.get(k)
    if not o: continue
    for c in cols:
        try: d=float(r[c])-float(o[c])
        except: continue
        if k[0]=='TOTAL':
            if abs(d)>1e-9: tot.setdefault(k[1],[]).append(f"{c}:{o[c]}->{r[c]}")
            continue
        if d>1e-9: worse.setdefault(c,[]).append((k[0][:12],k[1][0],o[c],r[c]))
        elif d<-1e-9: better.setdefault(c,[]).append((k[0][:12],k[1][0],o[c],r[c]))
for m in tot: print('TOTAL',m,' '.join(tot[m]))
show=set(sys.argv[3].split(',')) if len(sys.argv)>3 else None
for c in cols:
    if c in worse or c in better:
        print(f"  {c:18s} worse {len(worse.get(c,[])):4d}  better {len(better.get(c,[])):4d}", (worse.get(c,[])[:6] if (show is None or c in show) else ''))
