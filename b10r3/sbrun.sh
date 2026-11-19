#!/bin/bash
# interleaved scene() min: base vs now, 3 rounds; prints ratio of min-of-mins
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
F=$(ls $S/flowcorpus/*.mmd | grep -v -e bench-75 -e dense-)
B=${B:-400}
for r in 1 2 3; do
  $S/b10r3/sb_base/target/release/sb $B $F > $S/b10r3/sbb_$r.tsv
  $S/b10r3/sb_now/target/release/sb $B $F > $S/b10r3/sbn_$r.tsv
done
python3 - <<'PY'
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r3'
import collections
def load(p):
    d={}
    for l in open(p):
        n,mn,md,c=l.split('\t'); d[n]=(float(mn),float(md))
    return d
b=collections.defaultdict(list); n=collections.defaultdict(list)
for r in (1,2,3):
    for k,v in load(f'{S}/sbb_{r}.tsv').items(): b[k].append(v)
    for k,v in load(f'{S}/sbn_{r}.tsv').items(): n[k].append(v)
for k in b:
    bm=min(x[0] for x in b[k]); nm=min(x[0] for x in n[k]); bmed=min(x[1] for x in b[k]); nmed=min(x[1] for x in n[k])
    print(f'{k:14s} min {bm:.3f}->{nm:.3f} {nm/bm:.2f}x   med {bmed:.3f}->{nmed:.3f} {nmed/bmed:.2f}x')
PY
