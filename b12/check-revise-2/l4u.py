#!/usr/bin/env python3
"""Combined lane rule: 3*U > drawn links (U on the unit graph in written directions, exact per SCC)
AND some path of the node DAG (rank's raw marks reversed) enters one child block of the level twice."""
import sys, os, io, contextlib, itertools
C='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/check-fix-a'
sys.path.insert(0, C)
argv = sys.argv[:]; sys.argv = sys.argv[:1]
import blockmode as bm
with contextlib.redirect_stdout(io.StringIO()):
    import l4
sys.argv = argv
def lvl_of(npath, f, t):
    pa, pb = npath[f], npath[t]; k = 0
    while k < len(pa) and k < len(pb) and pa[k] == pb[k]: k += 1
    cu = ('b', pa[k]) if k < len(pa) else ('n', f)
    cv = ('b', pb[k]) if k < len(pb) else ('n', t)
    return tuple(pa[:k]), cu, cv
def mfas(units, w):
    tot = 0
    for comp in l4.sccs(units, list(w)):
        if len(comp) < 2: continue
        if len(comp) > 9: return None
        best = None
        for perm in itertools.permutations(comp):
            pos = {u: i for i, u in enumerate(perm)}
            turned = sum(c for (a, b), c in w.items() if a in pos and b in pos and pos[a] > pos[b])
            if best is None or turned < best: best = turned
        tot += best
    return tot
def twice(n, es, npath, level):
    adj = [[] for _ in range(n)]; indeg = [0]*n
    for (f, t) in es: adj[f].append(t); indeg[t] += 1
    order = [v for v in range(n) if indeg[v] == 0]; i = 0
    while i < len(order):
        u = order[i]; i += 1
        for v in adj[u]:
            indeg[v] -= 1
            if indeg[v] == 0: order.append(v)
    assert len(order) == n
    blocks = set(); info = []
    for (f, t) in es:
        lv, cu, cv = lvl_of(npath, f, t)
        hit = (lv == level and cu != cv and cv[0] == 'b')
        info.append(cv if hit else None)
        if hit: blocks.add(cv)
    res = []
    for X in sorted(blocks):
        best = [0]*n
        for u in order:
            for ei, (f, t) in enumerate(es):
                if f != u: continue
                w = 1 if info[ei] == X else 0
                if best[u] + w > best[t]: best[t] = best[u] + w
        if max(best) >= 2: res.append((X[1], max(best)))
    return res
for p in sys.argv[1:]:
    g = bm.load(p)
    parts = bm.split(g) or [('whole', g)]
    nm = os.path.basename(p).replace('.struct.json', '')
    for pn, pg in parts:
        if not pg.subs: continue
        sis, npath = bm.stand_ins(pg)
        edges = [(f, t) for (f, t, k, tag) in pg.edges if f != t and not (pg.nodes[f]['note'] or pg.nodes[t]['note'])]
        for (a, b, what, kind) in sis:
            if a is not None and 'note' not in what and not any(k in what for k in ('itself', 'own nested')):
                edges.append((a, b))
        n = len(pg.nodes)
        back = l4.l0_back(n, edges)
        dag = [((t, f) if ei in back else (f, t)) for ei, (f, t) in enumerate(edges)]
        levels = {}
        for (f, t) in edges:
            lv, cu, cv = lvl_of(npath, f, t)
            if cu != cv: levels.setdefault(lv, []).append((cu, cv))
        for lv, links in levels.items():
            w = {}
            for l in links: w[l] = w.get(l, 0) + 1
            units = sorted(set(x for l in links for x in l))
            U = mfas(units, w); nd = len(links)
            if not U: continue
            ratio = 3*U > nd
            tw = twice(n, dag, npath, lv) if ratio else []
            name = '/'.join(pg.subs[s]['id'] for s in lv) or 'root'
            print(f"{nm}\t{pn}\tlevel {name}\tU {U}/{nd}\tratio {'yes' if ratio else 'no'}\ttwice {[(pg.subs[x]['id'], c) for x, c in tw] if ratio else '-'}\t=> {'L4 FIRES' if ratio and tw else 'stacked'}")
