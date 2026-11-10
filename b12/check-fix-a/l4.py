#!/usr/bin/env python3
"""SPEC 2.3 L3/L4 by their letter: per level, the unit graph after L0's node-level
back-edges, M = least drawn links any unit order turns (exact per SCC, brute force),
and whether M > links/3 (the lane rule fires). Rank-only hubs are left out of L0's DFS
(stand-ins are in), B9 parts are judged one by one."""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(__file__))
import blockmode as bm

def l0_back(n, edges):
    def dfs(adj):
        state = [0]*n; back = set()
        for s in range(n):
            if state[s]: continue
            state[s] = 1; stack = [[s, 0]]
            while stack:
                u, ci = stack[-1]
                if ci < len(adj[u]):
                    stack[-1][1] += 1
                    v, ei = adj[u][ci]
                    if v == u: back.add(ei); continue
                    if state[v] == 0: state[v] = 1; stack.append([v, 0])
                    elif state[v] == 1: back.add(ei)
                else:
                    state[u] = 2; stack.pop()
        return back
    adj = [[] for _ in range(n)]
    for ei, (f, t) in enumerate(edges): adj[f].append((t, ei))
    b1 = dfs(adj)
    adj2 = [sorted(a) for a in adj]
    if adj2 != adj:
        b2 = dfs(adj2)
        if len(b2) < len(b1): b1 = b2
    return b1

def sccs(units, links):
    idx = {u: i for i, u in enumerate(units)}
    g = {u: set() for u in units}
    for (a, b) in links: g[a].add(b)
    # Tarjan
    index = {}; low = {}; onst = set(); st = []; out = []; c = [0]
    def strong(v):
        index[v] = low[v] = c[0]; c[0] += 1; st.append(v); onst.add(v)
        for w in g[v]:
            if w not in index: strong(w); low[v] = min(low[v], low[w])
            elif w in onst: low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = st.pop(); onst.discard(w); comp.append(w)
                if w == v: break
            out.append(comp)
    for u in units:
        if u not in index: strong(u)
    return out

def judge_part(name, g):
    if not g.subs: return []
    sis, npath = bm.stand_ins(g)
    n = len(g.nodes)
    edges = [(f, t, g.nodes[f]['note'] or g.nodes[t]['note']) for (f, t, k, tag) in g.edges if f != t]
    for (a, b, what, kind) in sis:
        if a is not None and not any(k in what for k in ('itself', 'own nested')):
            edges.append((a, b, 'note' in what))
    back = l0_back(n, [(f, t) for (f, t, _) in edges])
    res = []
    levels = {}
    for ei, (f, t, note) in enumerate(edges):
        if ei in back: f, t = t, f
        pa, pb = npath[f], npath[t]
        k = 0
        while k < len(pa) and k < len(pb) and pa[k] == pb[k]: k += 1
        level = tuple(pa[:k])
        cu = ('b', pa[k]) if k < len(pa) else ('n', f)
        cv = ('b', pb[k]) if k < len(pb) else ('n', t)
        if cu == cv: continue
        levels.setdefault(level, []).append((cu, cv, note))
    for level, links in levels.items():
        drawn = [(a, b) for (a, b, note) in links if not note]
        units = sorted(set(x for l in links for x in l[:2]))
        w = {}
        for (a, b) in drawn: w[(a, b)] = w.get((a, b), 0) + 1
        M = 0
        for comp in sccs(units, list(w)):
            if len(comp) < 2: continue
            if len(comp) > 9: M = None; break
            best = None
            for perm in itertools.permutations(comp):
                pos = {u: i for i, u in enumerate(perm)}
                turned = sum(c for (a, b), c in w.items() if a in pos and b in pos and pos[a] > pos[b])
                if best is None or turned < best: best = turned
            M += best
        nd = len(drawn)
        fires = M is not None and 3 * M > nd
        lv = '/'.join(g.subs[s]['id'] for s in level) or 'root'
        res.append((name, lv, nd, M, fires))
    return res

for p in sys.argv[1:]:
    g = bm.load(p)
    parts = bm.split(g) or [('whole', g)]
    nm = os.path.basename(p).replace('.struct.json', '')
    for pn, pg in parts:
        for (name, lv, nd, M, fires) in judge_part(pn, pg):
            if M: print(f"{nm}\t{pn}\tlevel {lv}\tdrawn links {nd}\tM {M}\t{'L4 FIRES' if fires else ''}")
