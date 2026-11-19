#!/usr/bin/env bash
# cmp.sh <label> [cols]: start (verifier pre) vs label, static rows that differ in the cols
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5
O=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r8/h
C=${2:-fan_x,x_drawn,edge_node_x,shared_ports,arrow_on_start,frame_side,node_frame,overlap_pairs,lateral_x}
for s in quality blocks notes fans xin shuf; do
python3 - $V/h/pre-$s.tsv $V/h/on-$s.tsv $O/$1-$s.tsv $C <<'PY'
import sys,csv
def load(p):
    rows={}
    try:
        with open(p) as f:
            r=csv.reader(f,delimiter='\t'); h=next(r)
            for x in r:
                if x[0]=='TOTAL' or x[1]!='static': continue
                rows[x[0]]=dict(zip(h,x))
    except Exception: pass
    return rows
a,b,c=load(sys.argv[1]),load(sys.argv[2]),load(sys.argv[3]); cols=sys.argv[4].split(',')
for k in sorted(c):
    pa,pb,pc=a.get(k,{}),b.get(k,{}),c[k]
    diff=[x for x in cols if pc.get(x)!=pb.get(x) or (x in('fan_x','x_drawn') and pa.get(x) not in (None,'?') and float(pc.get(x,0))>float(pa.get(x,0)))]
    if diff: print(f"{k:34s} "+'  '.join(f"{x}={pa.get(x,'?')}|{pb.get(x,'?')}->{pc.get(x,'?')}" for x in diff))
PY
done
