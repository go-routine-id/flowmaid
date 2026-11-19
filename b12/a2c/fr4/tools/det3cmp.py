#!/usr/bin/env python3
"""det3cmp.py <A> <B>: per set and mode, chips > 3 px off their own path in B that were not (or were nearer by
more than 0.5 px) in A. Exit 1 when any."""
import sys, os, glob
from collections import defaultdict
A, B = sys.argv[1], sys.argv[2]
def rd(p):
    d = defaultdict(list)
    if not os.path.exists(p): return None
    for l in open(p):
        f = l.rstrip('\n').split('\t')
        if len(f) != 3: d[(f[0], 'MISMATCH')].append(99.0); continue
        d[(f[0], f[1])].append(float(f[2]))
    return d
bad = 0
for p in sorted(glob.glob(os.path.join(B, 'det-*.txt'))):
    t = os.path.basename(p)[4:-4]
    b = rd(p); a = rd(os.path.join(A, os.path.basename(p)))
    if a is None: print('[%s] no baseline' % t); continue
    na = sum(1 for k in a for v in a[k] if v > 3); nb = sum(1 for k in b for v in b[k] if v > 3)
    worse = []; better = 0
    for k in sorted(set(a) | set(b)):
        va = sorted(a.get(k, [])); vb = sorted(b.get(k, []))
        for i in range(max(len(va), len(vb))):
            x = va[i] if i < len(va) else None; y = vb[i] if i < len(vb) else None
            if y is not None and y > 3 and (x is None or y > x + 0.5): worse.append('%s | %s %s -> %.1f' % (k[0], k[1], x, y))
            elif x is not None and x > 3 and (y is None or y < x - 0.5): better += 1
    bad += len(worse)
    print('[%s] chips > 3 px: %d -> %d; newly off or farther: %d; nearer: %d' % (t, na, nb, len(worse), better))
    for w in worse: print('    WORSE', w)
print('det: no worse' if bad == 0 else 'det: WORSE (%d)' % bad)
sys.exit(1 if bad else 0)
