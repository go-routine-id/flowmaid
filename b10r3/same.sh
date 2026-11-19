#!/bin/bash
# same.sh: rebuild dump_now and compare static/route/route_partial SVG hashes with HEAD over the corpora
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
(cd $S/b10r3/dump_now && cargo build --release -q 2>&1 | grep -E '^error' -A5 | head)
FILES="$S/flowcorpus/*.mmd $S/b8r1/bulkst/*.mmd $S/b8r1/bulkfc0/*.mmd $S/b10r2/r0/*.mmd $S/verify/b10-orthogonal/r3/extra/*.mmd $S/cmp/corpus/*.mmd"
[ -n "$CLS" ] && FILES="$FILES $(ls $S/b8r1/bulkcl/*.mmd | head -400)"
if [ ! -f $S/b10r3/head.hashes ] || [ -n "$REHEAD" ]; then $S/b10r3/dump_head/target/release/dump $FILES > $S/b10r3/head.hashes; fi
$S/b10r3/dump_now/target/release/dump $FILES > $S/b10r3/now.hashes
python3 - <<'PY'
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r3'
a=[l.split() for l in open(S+'/head.hashes')]; b=[l.split() for l in open(S+'/now.hashes')]
d=[0,0,0]; ex=[[],[],[]]
for x,y in zip(a,b):
    for k in range(3):
        if x[k]!=y[k]:
            d[k]+=1; ex[k].append(x[3].split('/')[-1])
print(f'files {len(a)} diff static {d[0]} route {d[1]} partial {d[2]}')
for k in range(3):
    if ex[k]: print(' ', ['static','route','partial'][k], ex[k][:8])
PY
