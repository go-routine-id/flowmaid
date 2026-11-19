#!/usr/bin/env bash
# qrun.sh <label> [ENV=val ...]: quality example of the critic stack over the 332 direction copies (a2i/dirs) -> q/<label>.tsv
C=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/critic
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2i
lab=$1; shift; mkdir -p $C/q
env "$@" $W/qcol.sh $C/target/release/examples/quality $W/dirs defects x_drawn lateral_x fan_x edge_chip side_ends min_edge_gap > $C/q/$lab.tsv
python3 - $W/qd-split.tsv $C/q/$lab.tsv <<'PY'
import sys
def load(p):
    d={}
    for l in open(p):
        f=l.rstrip('\n').split('\t')
        d[(f[0],f[1])]={k:float(v) for k,v in (x.split('=') for x in f[2:])}
    return d
a=load(sys.argv[1]); b=load(sys.argv[2])
cols=['defects','x_drawn','lateral_x','fan_x','edge_chip','side_ends']
for mode in ['static','routed']:
    tot={c:[0,0] for c in cols}; worse={c:[] for c in cols}; better={c:0 for c in cols}; gap=[]
    for k in a:
        if k[1]!=mode or k not in b: continue
        for c in cols:
            tot[c][0]+=a[k][c]; tot[c][1]+=b[k][c]
            if b[k][c]>a[k][c]: worse[c].append('%s %g->%g'%(k[0],a[k][c],b[k][c]))
            elif b[k][c]<a[k][c]: better[c]+=1
        if b[k]['min_edge_gap']<a[k]['min_edge_gap']-0.5: gap.append('%s %g->%g'%(k[0],a[k]['min_edge_gap'],b[k]['min_edge_gap']))
    print('[%s] '%mode+'  '.join('%s %g->%g (+%d/-%d)'%(c,tot[c][0],tot[c][1],len(worse[c]),better[c]) for c in cols))
    for c in ['defects','x_drawn','fan_x','lateral_x']:
        if worse[c]: print('   worse %s: %s'%(c,'; '.join(worse[c][:12])+(' ...' if len(worse[c])>12 else '')))
    if gap: print('   min_edge_gap down: '+'; '.join(gap[:10]))
PY
