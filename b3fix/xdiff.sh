#!/usr/bin/env bash
# xdiff.sh <dir> : per-edge crossing counts pre vs now (static and routed)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
D=$1
$S/b3fix/pre/target/release/examples/quality $D --details 2>&1 >/dev/null | grep XDBG > /tmp/xd_pre.txt
/home/user/flowmaid/target/release/examples/quality $D --details 2>&1 >/dev/null | grep XDBG > /tmp/xd_now.txt
python3 - <<'PY'
import collections
def load(f):
    c=collections.Counter()
    for l in open(f):
        p=l.rstrip('\n').split('\t')
        file,mode,rest=p[0],p[1],p[2]
        a,_,b,_,_=rest.split(' ',4)
        for e in (a,b):
            c[(file,mode,e)]+=1
    return c
a=load('/tmp/xd_pre.txt'); b=load('/tmp/xd_now.txt')
keys=sorted(set(a)|set(b))
for k in keys:
    if a[k]!=b[k]:
        print(k[0],k[1],k[2],a[k],'->',b[k])
PY
