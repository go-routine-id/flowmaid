#!/usr/bin/env python3
"""Offline reference: how few layered crossings does the block model allow?
Reads the first record of an FM_XXDBG dump; state = per level one sequence of
its units (nodes, blocks, runs); search = sifting to a fixed point + restarts."""
import json, sys, random
from bisect import bisect_right

def load(path, rec=0):
    recs = open(path).read().split("\n====\n")
    return json.loads(recs[rec])

class Model:
    def __init__(self, d):
        self.nodes = d["nodes"]; self.nl = d["nlayers"]; self.final = d["final"]
        N = self.nodes
        self.edges = [[] for _ in range(self.nl)]  # per gap l: (u,v)
        for u, n in enumerate(N):
            for v in n["succs"]:
                if N[v]["l"] == n["l"] + 1:
                    self.edges[n["l"]].append((u, v))
        # levels: path tuple -> list of units
        self.levels = {}
        self.unit_of = {}   # (level path, v) -> unit key
        self.walls = {}     # (cluster, layer) -> (left, right)
        for v, n in enumerate(N):
            p = tuple(n["path"])
            if n["border"]:
                c = p[-1]
                w = self.walls.setdefault((c, n["l"]), [None, None])
                w[n["border"] - 1] = v
                continue
        self.members = {}  # level path -> set of units
        for v, n in enumerate(N):
            if n["border"]:
                continue
            p = tuple(n["path"])
            # own level unit
            u = ("r", n["run"]) if n["run"] >= 0 else ("n", v)
            self.members.setdefault(p, {})[u] = None
            for k in range(len(p)):
                self.members.setdefault(p[:k], {})[("b", p[k])] = None
            self.members.setdefault(p, {})
        # per layer, per level: which units are present, and nodes of runs
        self.on = {}  # (path, layer) -> dict unit -> node v (for n / r) or None for block
        for v, n in enumerate(N):
            if n["border"]:
                continue
            p = tuple(n["path"]); l = n["l"]
            u = ("r", n["run"]) if n["run"] >= 0 else ("n", v)
            self.on.setdefault((p, l), {})[u] = v
            for k in range(len(p)):
                self.on.setdefault((p[:k], l), {})[("b", p[k])] = None
        self.canon = {}
        for p, us in self.members.items():
            for u in us:
                if u[0] == "b":
                    self.canon[(p, u)] = (min(n["canon"][0] for n in N if not n["border"] and tuple(n["path"])[:len(p)+1] == p + (u[1],)), 0, 0, 0)
                elif u[0] == "n":
                    self.canon[(p, u)] = tuple(N[u[1]]["canon"])
                else:
                    self.canon[(p, u)] = min(tuple(n["canon"]) for n in N if n["run"] == u[1])
    def canonical(self):
        return {p: sorted(us, key=lambda u: self.canon[(p, u)]) for p, us in self.members.items()}
    def from_pos(self):
        # the order the Rust code ended with: unit key = mean pos of members
        st = {}
        N = self.nodes
        for p, us in self.members.items():
            acc = {u: [] for u in us}
            for v, n in enumerate(N):
                if n["border"]: continue
                q = tuple(n["path"])
                if q[:len(p)] != p: continue
                if len(q) == len(p):
                    u = ("r", n["run"]) if n["run"] >= 0 else ("n", v)
                else:
                    u = ("b", q[len(p)])
                acc[u].append(n["pos"] / max(1, self.layer_len[n["l"]]))
            st[p] = sorted(us, key=lambda u: (sum(acc[u]) / max(1, len(acc[u])), self.canon[(p, u)]))
        return st
    def expand(self, st, p, l, out):
        present = self.on.get((p, l), {})
        for u in st[p]:
            if u not in present: continue
            if u[0] == "b":
                c = u[1]
                w = self.walls.get((c, l))
                if w and w[0] is not None: out.append(w[0])
                self.expand(st, p + (c,), l, out)
                if w and w[1] is not None: out.append(w[1])
            else:
                out.append(present[u])
    def layers(self, st):
        return [self._layer(st, l) for l in range(self.nl)]
    def _layer(self, st, l):
        out = []; self.expand(st, (), l, out); return out
    def cost(self, st, gaps=None):
        lay = self.layers(st)
        pos = {}
        for lv in lay:
            for i, v in enumerate(lv): pos[v] = i
        tot = 0
        for l in range(self.nl - 1):
            es = sorted((pos[u], pos[v]) for u, v in self.edges[l])
            seq = [e[1] for e in es]
            tot += inv(seq)
        return tot

def inv(a):
    # count inversions (strict)
    s = []; c = 0
    for x in a:
        i = bisect_right(s, x)
        c += len(s) - i
        s.insert(i, x)
    return c

def sift(m, st, rng=None, verbose=False):
    best = m.cost(st)
    improved = True
    while improved:
        improved = False
        for p in sorted(st):
            seq = st[p]
            order = list(seq)
            for u in order:
                i0 = seq.index(u)
                seq.pop(i0)
                bi, bc = i0, best
                for i in range(len(seq) + 1):
                    if i == i0: continue
                    seq.insert(i, u)
                    c = m.cost(st)
                    seq.pop(i)
                    if c < bc:
                        bc, bi = c, i
                seq.insert(bi, u)
                if bc < best:
                    best = bc; improved = True
    return best

def adj(m, st):
    """adjacent exchange only, to a fixed point"""
    best = m.cost(st)
    improved = True
    while improved:
        improved = False
        for p in sorted(st):
            seq = st[p]
            for i in range(len(seq) - 1):
                seq[i], seq[i+1] = seq[i+1], seq[i]
                c = m.cost(st)
                if c < best:
                    best = c; improved = True
                else:
                    seq[i], seq[i+1] = seq[i+1], seq[i]
    return best

if __name__ == "__main__":
    path = sys.argv[1]; restarts = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    d = load(path)
    m = Model(d)
    m.layer_len = [0] * m.nl
    for n in m.nodes: m.layer_len[n["l"]] += 1
    st = m.canonical()
    c0 = m.cost(st)
    stf = m.from_pos()
    print("rust final", d["final"], "re-derived", m.cost(stf), "canonical", c0, "units", {len(p): 0 for p in st} and sum(len(v) for v in st.values()))
    a = {p: list(s) for p, s in st.items()}
    print("canonical + adjacent exchange:", adj(m, a))
    a = {p: list(s) for p, s in st.items()}
    cs = sift(m, a); print("canonical + sifting:", cs)
    b = {p: list(s) for p, s in stf.items()}
    print("rust final + sifting:", sift(m, b))
    rng = random.Random(1)
    bestall = cs
    for k in range(restarts):
        s2 = {p: rng.sample(s, len(s)) for p, s in st.items()}
        c = sift(m, s2)
        bestall = min(bestall, c)
        print(" restart", k, c, "best", bestall)
