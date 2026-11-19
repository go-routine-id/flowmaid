#!/bin/bash
# qcmp.sh <changed list>: quality diff (base vs now) for the source files of changed svgs
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b7r2
files=$(sed -E 's/\.routed\.svg$|\.svg$//' $1 | sort -u | while read n; do d=${n%%__*}; b=${n#*__}; grep -E "/$d/$b\.mmd$" $S/list.txt | head -1; done)
cd $S/qd && cargo build --release -q 2>&1 | grep -E "^error" -A5 | head
$S/qdb/target/release/qd $files > $S/q_base.txt 2>/dev/null
$S/qd/target/release/qd $files > $S/q_now.txt 2>/dev/null
python3 - $S/q_base.txt $S/q_now.txt <<'PY'
import sys
def load(p):
    d={}
    for l in open(p):
        parts=l.rstrip('\n').split('\t')
        if len(parts)<3: continue
        kv=dict(x.split('=',1) for x in parts[2].split())
        d[(parts[0],parts[1])]=kv
    return d
a,b=load(sys.argv[1]),load(sys.argv[2])
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','width','height','aspect','area_per_node','ink'}
for k in sorted(a):
    if k not in b: continue
    diffs=[]
    for m in a[k]:
        if m in skip: continue
        if a[k][m]!=b[k].get(m): diffs.append(f"{m}:{a[k][m]}->{b[k].get(m)}")
    if diffs: print(k[0],k[1],' '.join(diffs))
PY
