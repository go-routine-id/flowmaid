#!/usr/bin/env python3
"""A1's proposed second condition for L4, checked independently: on the node graph with rank's
back-edges reversed (a DAG), does some path enter one child block of a level twice through drawn
links of that level?  Combined with 3*M > drawn links (M from the checker's l4 module)."""
import sys, os, io, contextlib
C='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/check-fix-a'
sys.path.insert(0, C)
argv = sys.argv[:]; sys.argv = sys.argv[:1]
import blockmode as bm
with contextlib.redirect_stdout(io.StringIO()):
    import l4
sys.argv = argv

def twice(n, edges, npath, back):
    # edges: (f,t) drawn (non-note) links incl. stand-ins; reversed where back
    es = [((t, f) if ei in back else (f, t)) for ei, (f, t) in enumerate(edges)]
    adj = [[] for _ in range(n)]; indeg = [0]*n
    for (f, t) in es: adj[f].append(t); indeg[t] += 1
    order = [v for v in range(n) if indeg[v] == 0]; i = 0
    while i < len(order):
        u = order[i]; i += 1
        for v in adj[u]:
            indeg[v] -= 1
            if indeg[v] == 0: order.append(v)
    assert len(order) == n, 'not a DAG'
    hits = []
    # (level, block) pairs
    pairs = set()
    info = []
    for (f, t) in es:
        pa, pb = npath[f], npath[t]; k = 0
        while k < len(pa) and k < len(pb) and pa[k] == pb[k]: k += 1
        lvl = tuple(pa[:k]); into = pb[k] if k < len(pb) else None
        cu = ('b', pa[k]) if k < len(pa) else ('n', f)
        cv = ('b', pb[k]) if k < len(pb) else ('n', t)
        info.append((lvl, into if cu != cv else None))
        if into is not None and cu != cv: pairs.add((lvl, into))
    for (lvl, X) in sorted(pairs):
        best = [0]*n
        for u in order:
            for ei, (f, t) in enumerate(es):
                if f != u: continue
                w = 1 if info[ei] == (lvl, X) else 0
                if best[u] + w > best[t]: best[t] = best[u] + w
        if max(best) >= 2: hits.append((lvl, X, max(best)))
    return hits

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
        back = l4.l0_back(len(pg.nodes), edges)
        ratio = {}
        with contextlib.redirect_stdout(io.StringIO()):
            res = l4.judge_part(pn, pg)
        for (name, lv, nd, M, fires) in res: ratio[lv] = (nd, M, fires)
        hits = twice(len(pg.nodes), edges, npath, back)
        lv_hits = {}
        for (lvl, X, c) in hits:
            lv = '/'.join(pg.subs[s]['id'] for s in lvl) or 'root'
            lv_hits.setdefault(lv, []).append((pg.subs[X]['id'], c))
        for lv, (nd, M, fires) in ratio.items():
            if fires:
                print(f"{nm}\t{pn}\tlevel {lv}\tM {M}/{nd}\tratio fires\tpath enters twice: {lv_hits.get(lv, 'no')}\t=> L4 {'FIRES' if lv in lv_hits else 'does not fire'}")
