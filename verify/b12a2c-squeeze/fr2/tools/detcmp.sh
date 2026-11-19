#!/usr/bin/env bash
# detcmp.sh <predir> <headdir> : chips >3px off own path, newly or farther
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2c-squeeze/fr2
python3 $V/tools/det.py $1/*.svg > /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2c-squeeze/fr2/.dp.txt
python3 $V/tools/det.py $2/*.svg > /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2c-squeeze/fr2/.dh.txt
python3 - <<PY
from collections import defaultdict
def rd(p):
    d=defaultdict(list); mism=[]
    for l in open(p):
        f=l.rstrip('\n').split('\t')
        if len(f)==2: mism.append(f[0]); continue
        d[(f[0],f[1])].append(float(f[2]))
    return d,mism
a,ma=rd("$V/.dp.txt"); b,mb=rd("$V/.dh.txt")
print("mismatch pre",len(ma),"head",len(mb))
na=sum(1 for k in a for v in a[k] if v>3); nb=sum(1 for k in b for v in b[k] if v>3)
print("chips >3px: pre",na,"head",nb)
for k in sorted(set(a)|set(b)):
    va=sorted(a.get(k,[])); vb=sorted(b.get(k,[]))
    for i in range(max(len(va),len(vb))):
        x=va[i] if i<len(va) else None; y=vb[i] if i<len(vb) else None
        if y is not None and y>3 and (x is None or y>x+0.5):
            print("WORSE",k[0],k[1],x,"->",y)
        elif x is not None and x>3 and (y is None or y<x-0.5):
            print("better",k[0],k[1],x,"->",y)
PY
