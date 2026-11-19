#!/bin/bash
# sbcmp.sh: time scene() base vs now (2 interleaved rounds, min)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; cd $S/b10r2
F="$S/flowcorpus/advanced.mmd $S/flowcorpus/back-edge.mmd $S/flowcorpus/demo.mmd $S/flowcorpus/long-labels.mmd $S/flowcorpus/shapes.mmd $S/flowcorpus/stress.mmd $S/flowcorpus/subgraph.mmd $S/flowcorpus/wide-labels.mmd $S/flowcorpus/fanout.mmd $S/flowcorpus/lr.mmd $S/flowcorpus/fanout-frames.mmd"
for r in 1 2 3; do REPS=800 ./probe_base/target/release/sbench $F > sbx_base_$r.tsv; REPS=800 ./probe/target/release/sbench $F > sbx_now_$r.tsv; done
python3 - <<'PY'
import collections
d=collections.defaultdict(dict)
for p in ['base','now']:
    for r in [1,2,3]:
        for line in open(f'sbx_{p}_{r}.tsv'):
            n,t=line.split(); t=float(t); d[n][p]=min(d[n].get(p,1e9),t)
for n,v in d.items(): print(f"{n:14s} base {v['base']:.4f} now {v['now']:.4f}  {v['now']/v['base']:.2f}x")
PY
