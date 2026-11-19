#!/usr/bin/env python3
"""detcmp.py <startdir> <canddir> : per table, chips more than 3 px off their line in the candidate that were
not (or were nearer) at the start. Exit 1 when any."""
import sys, os, re
a, b = sys.argv[1], sys.argv[2]
def load(p):
    d = {}; cur = None; tot = {}
    for l in open(p):
        if not l.startswith('    '):
            f = l.rstrip('\n').split('\t'); cur = f[0]; tot[cur] = (int(f[1]), float(f[3])); d.setdefault(cur, {})
        else:
            parts = [x.strip() for x in l.strip().split('|')]
            toks = parts[1].split()
            while toks and re.match(r'^-?[0-9.]+$', toks[-1]):
                toks.pop()
            key = parts[0] + ' | ' + ' '.join(toks)
            d[cur][key] = float(parts[2])
    return d, tot
bad = 0
for t in ['local-st', 'local-rt', 'dirs-st', 'dirs-rt', 'dirsx-st', 'dirsx-rt']:
    A, ta = load(os.path.join(a, 'det-%s.txt' % t)); B, tb = load(os.path.join(b, 'det-%s.txt' % t))
    n3a = sum(v[0] for v in ta.values()); n3b = sum(v[0] for v in tb.values())
    worse = []
    for name, labs in B.items():
        for k, v in labs.items():
            old = A.get(name, {}).get(k)
            if old is None or v > old + 0.5:
                worse.append('%s: %s %s -> %.1f' % (name, k, '-' if old is None else '%.1f' % old, v))
    print('[%s] chips > 3 px: %d -> %d; newly off or farther: %d' % (t, n3a, n3b, len(worse)))
    for w in worse: print('   ', w)
    bad += len(worse)
sys.exit(1 if bad else 0)
