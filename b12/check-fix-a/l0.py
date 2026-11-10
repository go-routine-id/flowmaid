#!/usr/bin/env python3
"""Which of rank's node-level cycle breakers (SPEC L0) fall on a link that lies on NO cycle
of its own level's unit graph: the node cycle leaves the block, so L3 breaks it again at an
ancestor level (a double break)."""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
import blockmode as bm, l4
for p in sys.argv[1:]:
    g = bm.load(p)
    parts = bm.split(g) or [('whole', g)]
    nm = os.path.basename(p).replace('.struct.json', '')
    for pn, pg in parts:
        if not pg.subs: continue
        sis, npath = bm.stand_ins(pg)
        edges = [(f, t) for (f, t, k, tag) in pg.edges if f != t]
        for (a, b, what, kind) in sis:
            if a is not None and not any(k in what for k in ('itself', 'own nested')): edges.append((a, b))
        back = l4.l0_back(len(pg.nodes), edges)
        levels = {}
        info = {}
        for ei, (f, t) in enumerate(edges):
            pa, pb = npath[f], npath[t]
            k = 0
            while k < len(pa) and k < len(pb) and pa[k] == pb[k]: k += 1
            level = tuple(pa[:k])
            cu = ('b', pa[k]) if k < len(pa) else ('n', f)
            cv = ('b', pb[k]) if k < len(pb) else ('n', t)
            if cu == cv: continue
            levels.setdefault(level, []).append((cu, cv))
            info[ei] = (level, cu, cv)
        for ei in sorted(back):
            if ei not in info: continue
            level, cu, cv = info[ei]
            links = levels[level]
            units = sorted(set(x for l in links for x in l))
            comp = next(c for c in l4.sccs(units, links) if cu in c)
            on_cycle = cv in comp
            f, t = edges[ei]
            lv = '/'.join(pg.subs[s]['id'] for s in level) or 'root'
            if not on_cycle:
                print(f"{nm}\t{pn}\tL0 marks {pg.nodes[f]['id']}->{pg.nodes[t]['id']} (level {lv}): no cycle of that level's unit graph runs through it")
