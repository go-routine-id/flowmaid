#!/usr/bin/env bash
# qcmp.sh <label> : run quality on flowcorpus + qin for head worktree (cached) and working tree; diff totals + per-file defects
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1; O=$S/b7r3/q/$L; mkdir -p $O
H=$S/b7r3/head/target/release/examples/quality
(cd /home/user/flowmaid && cargo build --release -q --example quality 2>&1 | grep -E "^error" )
N=/home/user/flowmaid/target/release/examples/quality
for d in flowcorpus qin; do
  D=$S/$d; [ $d = qin ] && D=$S/b7r3/qin
  [ -f $S/b7r3/q/head-$d.tsv ] || $H $D > $S/b7r3/q/head-$d.tsv 2>/dev/null
  $N $D > $O/$d.tsv 2>/dev/null
done
python3 - "$S/b7r3/q" "$O" <<'PY'
import sys,csv
q,o=sys.argv[1],sys.argv[2]
for d in ['flowcorpus','qin']:
    def load(p):
        rows=[l.rstrip('\n').split('\t') for l in open(p)]
        hdr=rows[0]; return hdr,{(r[0],r[1]):r for r in rows[1:] if len(r)>2}
    h,a=load(f'{q}/head-{d}.tsv'); _,b=load(f'{o}/{d}.tsv')
    skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic'}
    print('==',d)
    for k in sorted(a):
        if k not in b: continue
        diffs=[]
        for i,c in enumerate(h):
            if i<2 or c in skip: continue
            if a[k][i]!=b[k][i]: diffs.append(f'{c} {a[k][i]}->{b[k][i]}')
        if diffs and (k[0].startswith('TOTAL') or 'defects' in ' '.join(diffs)):
            print(k[0],k[1],'; '.join(diffs)[:600])
PY
