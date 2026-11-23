import sys, itertools
sys.path.insert(0, '.')
import sim
from collections import deque

def ev(cols, rows, v):
    a = sim.area_for(cols, rows, True)
    b, t = sim.paint(v, a)
    d, w = sim.defects(b, t, a)
    full = frozenset(x['name'] for x in b if x['rect'] == x['full'])
    cut = frozenset(x['name'] for x in b if x['rect'] != x['full'])
    hcut = frozenset(x['name'] for x in b if x['rect'][0] != x['full'][0] or x['rect'][2] != x['full'][2])
    toks = tuple(sorted(x['text'] for x in t if x['drawn'] == x['text']))
    clipped = [x['text'] for x in t if x['drawn'] != x['text']]
    return dict(clean=not d and not clipped, full=full, cut=cut, hcut=hcut, toks=toks, d=d)

def mk(cols, rows, keys):
    v = sim.View(); v.page_rows = rows - 8 - 1
    for k in keys: v.key(k)
    return v

def copyv(v):
    n = sim.View(); n.ox, n.oy, n.zoom, n.page_rows = v.ox, v.oy, v.zoom, v.page_rows; return n

def component(cols, rows, v0, maxn=400):
    """clean states reachable from v0 by hjkl (v0 must be clean)."""
    key = lambda v: (round(v.ox, 6), round(v.oy, 6))
    seen = {key(v0): (v0, '')}
    q = deque([v0])
    while q and len(seen) < maxn:
        v = q.popleft()
        for k in 'hjkl':
            n = copyv(v).key(k)
            kk = key(n)
            if kk in seen: continue
            e = ev(cols, rows, n)
            if not e['clean']: continue
            seen[kk] = (n, seen[key(v)][1] + k)
            q.append(n)
    return seen

if __name__ == '__main__':
    res = []
    pres = [''] + list('jklhDU') + [a + b for a in 'jklhDU' for b in 'jklhDU']
    for cols in range(96, 112, 2):
        for rows in range(26, 34):
            for pre in pres:
                for zk in ['-', '--']:
                    ok = True
                    v = mk(cols, rows, pre)
                    for z in zk:
                        v.key(z)
                        if not ev(cols, rows, v)['clean']:
                            ok = False; break
                    if not ok: continue
                    comp = component(cols, rows, v)
                    tabs = set(); toks = set(); fulls = set()
                    for kk, (s, path) in comp.items():
                        e = ev(cols, rows, s)
                        fulls |= e['full']; toks |= set(e['toks'])
                    res.append((len(fulls), len(toks), len(comp), cols, rows, pre, zk, sorted(fulls), sorted(toks)))
    res.sort(reverse=True)
    for r in res[:60]:
        print(r)
