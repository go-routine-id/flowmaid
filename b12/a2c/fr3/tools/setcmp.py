#!/usr/bin/env python3
"""setcmp.py <A> <B> [sets...]: per set (q-<set>.tsv), static and routed, per-file verdict columns B vs A."""
import sys, os, glob
A, B = sys.argv[1], sys.argv[2]
sets = sys.argv[3:] or [os.path.basename(p)[2:-4] for p in sorted(glob.glob(os.path.join(B, 'q-*.tsv')))]
cols = ['defects','x_drawn','lateral_x','fan_x','edge_chip','side_ends']
def load(p):
    L = [l.rstrip('\n').split('\t') for l in open(p)]
    h = L[0]; d = {}
    for r in L[1:]:
        if len(r) < len(h) or r[0] == 'TOTAL': continue
        d[(r[0], r[1])] = {c: float(r[h.index(c)]) for c in cols + ['min_edge_gap']}
    return d
tot_bad = 0
for s in sets:
    pa, pb = os.path.join(A, 'q-%s.tsv' % s), os.path.join(B, 'q-%s.tsv' % s)
    if not (os.path.exists(pa) and os.path.exists(pb)): continue
    a, b = load(pa), load(pb)
    for mode in ['static', 'routed']:
        keys = [k for k in b if k[1] == mode]
        worse = []; better = 0
        tots = {c: (sum(a[k][c] for k in keys if k in a), sum(b[k][c] for k in keys)) for c in cols}
        for k in keys:
            if k not in a: worse.append('%s MISSING' % k[0]); continue
            w = [c for c in cols if b[k][c] > a[k][c]]
            if b[k]['min_edge_gap'] < a[k]['min_edge_gap'] - 0.5: w.append('gap')
            bt = [c for c in cols if b[k][c] < a[k][c]]
            if w: worse.append('%s %s' % (k[0], ' '.join('%s %g->%g' % (c, a[k][c], b[k][c]) if c != 'gap' else 'gap %.2f->%.2f' % (a[k]['min_edge_gap'], b[k]['min_edge_gap']) for c in w)))
            elif bt: better += 1
        tot_bad += len(worse)
        print('[%s %s] %d files; worse %d, better %d;' % (s, mode, len(keys), len(worse), better), ' '.join('%s %g->%g' % (c, *tots[c]) for c in cols))
        for w in worse: print('    WORSE', w)
print('sets: no worse' if tot_bad == 0 else 'sets: WORSE (%d)' % tot_bad)
