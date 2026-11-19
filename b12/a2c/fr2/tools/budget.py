#!/usr/bin/env python3
"""budget.py <run-label>... : PLAN invariant 1 per file against runs/baseline (drag_ms, route_ms <= max(1.25x, +1 ms);
static scene_ms <= 2x); also lists the package start's run v-b12a3-route-r1b for reference."""
import sys, csv
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/runs'
def load(l):
    R={}
    rows=list(csv.reader(open(f'{S}/{l}/perf.tsv'),delimiter='\t')); h=rows[0]
    for r in rows[1:]:
        if len(r)==len(h): R[(r[0],r[1])]=dict(zip(h,r))
    return R
base=load('baseline'); start=load('v-b12a3-route-r1b')
for lab in sys.argv[1:]:
    c=load(lab); bad=[]
    for (f,m),b in base.items():
        if f=='TOTAL' or (f,m) not in c: continue
        x=c[(f,m)]
        if m=='routed':
            for k in ('drag_ms','route_ms'):
                lim=max(1.25*float(b[k]), float(b[k])+1.0)
                if float(x[k])>lim: bad.append(f'{f} {k} {x[k]} > {lim:.2f} (base {b[k]}, start {start.get((f,m),{}).get(k)})')
        else:
            k='scene_ms'; lim=2*float(b[k])
            if float(x[k])>lim: bad.append(f'{f} {k} {x[k]} > {lim:.2f} (base {b[k]}, start {start.get((f,m),{}).get(k)})')
    print(lab, 'within budget' if not bad else 'OVER: '+'; '.join(bad))
