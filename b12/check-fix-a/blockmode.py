#!/usr/bin/env python3
"""The structural predicate of SPEC 2.2, computed from a structdump JSON.

block mode  <=>  the augmented graph (real edges + one stand-in per sub-edge,
as scene_clustered builds them in 0.38) has a border link: a real edge
(visible or invisible, no self-loop) or a stand-in whose end nodes have
different cluster paths.

B9 direction blocks are cut out first, as compound::split does; each part
is judged on its own. Prints one line per input.
"""
import json, math, sys, os


def paths(nodes_sub, subs):
    def chain(s):
        c = []
        cur = s
        for _ in range(len(subs) + 1):
            if cur is None:
                break
            c.append(cur)
            cur = subs[cur]["parent"]
        return list(reversed(c))
    return [chain(s) if s is not None else [] for s in nodes_sub], [chain(s) for s in range(len(subs))]


class G:
    def __init__(self, direction, nodes, subs, edges, sub_edges):
        self.direction = direction
        self.nodes = nodes          # list of dict {id, sub, note}
        self.subs = subs            # list of dict {id, parent, direction, nodes}
        self.edges = edges          # list of (from, to, kind, tag)
        self.sub_edges = sub_edges  # list of (('n',i)|('s',i), ('n',i)|('s',i), kind, tag)


def load(path):
    d = json.load(open(path))
    nodes = [{"id": n["id"], "sub": n["sub"], "note": n["id"].startswith("__note_")} for n in d["nodes"]]
    subs = [{"id": s["id"], "parent": s["parent"], "direction": s["direction"], "nodes": list(s["nodes"])} for s in d["subgraphs"]]
    edges, sub_edges = [], []
    for e in d["edges"]:
        f, t = e["from"], e["to"]
        if "n" in f and "n" in t:
            edges.append((f["n"], t["n"], e["kind"], "edge"))
        else:
            fe = ("n", f["n"]) if "n" in f else ("s", f["s"])
            te = ("n", t["n"]) if "n" in t else ("s", t["s"])
            sub_edges.append((fe, te, e["kind"], "sub"))
    return G(d["direction"], nodes, subs, edges, sub_edges)


def horizontal(d):
    return d in ("LR", "RL")


def b9_blocks(g):
    ns = len(g.subs)
    if not any(s["direction"] for s in g.subs):
        return [False] * ns
    npath, spath = paths([n["sub"] for n in g.nodes], g.subs)

    def inside(end):
        if end[0] == "n":
            return npath[end[1]]
        return spath[end[1]][:-1]
    internal = [False] * ns
    external = [False] * ns

    def link(a, b):
        a, b = inside(a), inside(b)
        k = 0
        while k < len(a) and k < len(b) and a[k] == b[k]:
            k += 1
        for c in a[:k]:
            internal[c] = True
        for c in a[k:] + b[k:]:
            external[c] = True
    for (f, t, _, _) in g.edges:
        if f != t:
            link(("n", f), ("n", t))
    for (f, t, _, _) in g.sub_edges:
        if f != t:
            link(f, t)
    order = sorted(range(ns), key=lambda s: len(spath[s]))
    out = [False] * ns
    flow = [g.direction] * ns
    for s in order:
        p = g.subs[s]["parent"]
        rnd = flow[p] if p is not None else g.direction
        own = g.subs[s]["direction"] if g.subs[s]["direction"] and g.subs[s]["direction"] != rnd else None
        out[s] = own is not None and internal[s] and not external[s]
        flow[s] = own if (own and out[s]) else rnd
    return out


def split(g):
    """Parts as compound::split cuts them (blocks first, the rest last).
    Returns None when no subgraph is a B9 block."""
    flags = b9_blocks(g)
    if not any(flags):
        return None
    ns = len(g.subs)
    npath, spath = paths([n["sub"] for n in g.nodes], g.subs)
    tops = [s for s in range(ns) if next((c for c in spath[s] if flags[c]), None) == s]
    def sb(s):
        t = next((c for c in spath[s] if flags[c]), None)
        return tops.index(t) if t is not None else None
    sub_block = [sb(s) for s in range(ns)]
    node_block = [sub_block[n["sub"]] if n["sub"] is not None else None for n in g.nodes]

    def place(end):
        if end[0] == "n":
            b = node_block[end[1]]
            return ("rest",) if b is None else ("in", b)
        b = sub_block[end[1]]
        if b is None:
            return ("rest",)
        return ("frame", b) if tops[b] == end[1] else ("in", b)
    parts = []
    for b, top in enumerate(tops):
        members = [v for v in range(len(g.nodes)) if node_block[v] == b]
        at = {v: k for k, v in enumerate(members)}
        subs_in = [s for s in range(ns) if sub_block[s] == b]
        sat = {s: k for k, s in enumerate(subs_in)}
        pnodes = [dict(g.nodes[v]) for v in members]
        psubs = []
        for s in subs_in:
            sg = g.subs[s]
            psubs.append({"id": sg["id"], "parent": None if s == top else sat.get(sg["parent"]),
                          "direction": sg["direction"], "nodes": [at[v] for v in sg["nodes"] if v in at]})
        for k, v in enumerate(members):
            pnodes[k]["sub"] = sat[g.nodes[v]["sub"]]
        pedges = [(at[f], at[t], kind, tag) for (f, t, kind, tag) in g.edges if f in at and t in at]
        psub = []
        for (f, t, kind, tag) in g.sub_edges:
            pa, pb = place(f), place(t)
            if pa != ("in", b) and pb != ("in", b):
                continue
            def end(e):
                return ("n", at[e[1]]) if e[0] == "n" else ("s", sat[e[1]])
            psub.append((end(f), end(t), kind, tag))
        parts.append(("block " + g.subs[top]["id"], G(g.subs[top]["direction"] or g.direction, pnodes, psubs, pedges, psub)))
    # the rest
    slots = []
    placed = set()
    for v in range(len(g.nodes)):
        b = node_block[v]
        if b is None:
            slots.append(("node", v))
        elif b not in placed:
            placed.add(b)
            slots.append(("proxy", b))
    at = {}
    proxy_at = {}
    for k, (kind, x) in enumerate(slots):
        if kind == "node":
            at[x] = k
        else:
            proxy_at[x] = k
    subs_out = [s for s in range(ns) if sub_block[s] is None]
    sat = {s: k for k, s in enumerate(subs_out)}
    pnodes = []
    for kind, x in slots:
        if kind == "node":
            n = dict(g.nodes[x])
            n["sub"] = sat.get(n["sub"]) if n["sub"] is not None else None
            pnodes.append(n)
        else:
            p = g.subs[tops[x]]["parent"]
            pnodes.append({"id": g.subs[tops[x]]["id"] + "'", "sub": sat.get(p) if p is not None else None, "note": False})
    psubs = []
    for s in subs_out:
        sg = g.subs[s]
        nodes = [at[v] for v in sg["nodes"] if v in at]
        nodes += [proxy_at[b] for b, t in enumerate(tops) if g.subs[t]["parent"] == s]
        psubs.append({"id": sg["id"], "parent": sat.get(sg["parent"]) if sg["parent"] is not None else None,
                      "direction": sg["direction"], "nodes": nodes})
    pedges = [(at[f], at[t], kind, tag) for (f, t, kind, tag) in g.edges if f in at and t in at]
    psub = []
    for (f, t, kind, tag) in g.sub_edges:
        pa, pb = place(f), place(t)
        if pa[0] == "in" or pb[0] == "in":
            continue
        def node(e, p):
            if p[0] == "frame":
                return proxy_at[p[1]]
            if e[0] == "n":
                return at.get(e[1])
            return None
        a, c = node(f, pa), node(t, pb)
        if a is not None and c is not None:
            pedges.append((a, c, kind, "sub-as-edge"))
        else:
            def end(e, n):
                if n is not None:
                    return ("n", n)
                return ("s", sat[e[1]])
            psub.append((end(f, a), end(t, c), kind, tag))
    parts.append(("rest", G(g.direction, pnodes, psubs, pedges, psub)))
    return parts


GRID_MIN_MEMBERS = 3
GRID_MIN_EDGELESS = 4


def grid_pairs(g, npath):
    n = len(g.nodes)
    fixed = [False] * n
    sources = [[] for _ in range(n)]
    for (f, t, _, _) in g.edges:
        fixed[f] = True
        if f == t:
            fixed[t] = True
        else:
            sources[t].append(f)
    for (f, t, _, _) in g.sub_edges:
        for e in (f, t):
            if e[0] == "n":
                fixed[e[1]] = True
    out = []
    for si, s in enumerate(g.subs):
        if s["direction"] and horizontal(s["direction"]) != horizontal(g.direction):
            continue
        sets = {}
        for v in s["nodes"]:
            if fixed[v]:
                continue
            src = sources[v][0] if sources[v] else None
            one = all(u == src for u in sources[v])
            if one and (src is None or si not in npath[src]):
                sets.setdefault(src, []).append(v)
        for src in sorted(sets, key=lambda x: (x is not None, x if x is not None else -1)):
            free = sets[src]
            least = GRID_MIN_MEMBERS if src is not None else GRID_MIN_EDGELESS
            if len(free) < least:
                continue
            cols = math.ceil(math.sqrt(len(free)))
            for i in range(cols, len(free)):
                out.append((free[i - cols], free[i]))
    return out


def cycle_ends(g, npath, grid, s, entry):
    inside = lambda v: s in npath[v]
    n = len(g.nodes)
    adj = [[] for _ in range(n)]
    for (a, b) in [(f, t) for (f, t, _, _) in g.edges] + grid:
        if a != b and inside(a) and inside(b):
            adj[a].append(b)
    state = [0] * n
    keep_in = [False] * n
    keep_out = [False] * n
    for root in range(n):
        if not inside(root) or state[root] != 0:
            continue
        state[root] = 1
        stack = [[root, 0]]
        while stack:
            u, nx = stack[-1]
            if nx >= len(adj[u]):
                state[u] = 2
                stack.pop()
                continue
            stack[-1][1] += 1
            w = adj[u][nx]
            if state[w] == 1:
                continue
            keep_out[u] = True
            keep_in[w] = True
            if state[w] == 0:
                state[w] = 1
                stack.append([w, 0])
    flags = keep_in if entry else keep_out
    return [v for v in range(n) if inside(v) and not flags[v]]


def stand_ins(g):
    """(from node, to node, kind of sub-edge) per sub-edge, None when
    scene_clustered builds no stand-in."""
    npath, spath = paths([n["sub"] for n in g.nodes], g.subs)
    grid = grid_pairs(g, npath)
    has_in, has_out = set(), set()
    for (f, t) in [(f, t) for (f, t, _, _) in g.edges] + grid:
        if f == t:
            continue
        for ca, cb in zip(npath[f], npath[t]):
            if ca != cb:
                break
            has_in.add((ca, t))
            has_out.add((ca, f))

    def members(s, entry):
        flags = has_in if entry else has_out
        m = [v for v in range(len(g.nodes)) if s in npath[v] and (s, v) not in flags]
        return m if m else cycle_ends(g, npath, grid, s, entry)

    def ends(e, entry):
        return [e[1]] if e[0] == "n" else members(e[1], entry)
    out = []
    for (f, t, kind, tag) in g.sub_edges:
        what = None
        if f[0] == "n" and t[0] == "s" and t[1] in npath[f[1]]:
            v, s = f[1], t[1]
            linked = lambda m: any(a == v and b == m for (a, b, _, _) in g.edges)
            fr, to = [v], [m for m in members(s, True) if m != v and not linked(m)]
            what = "member->own sub"
        elif f[0] == "s" and t[0] == "n" and f[1] in npath[t[1]]:
            fr, to = [], []
            what = "sub->own member"
        else:
            fr, to = ends(f, False), ends(t, True)
            if f[0] == "s" and t[0] == "s":
                if f[1] == t[1]:
                    what = "sub->itself"
                elif f[1] in spath[t[1]] or t[1] in spath[f[1]]:
                    what = "sub<->own nested sub"
                else:
                    what = "sub->sub"
            else:
                what = "node<->sub"
        note = any(e[0] == "n" and g.nodes[e[1]]["note"] for e in (f, t))
        if note:
            what = "note on composite (" + what + ")"
        a = fr[0] if fr else None
        b = next((x for x in to if x != a), None) if a is not None else None
        out.append((a, b, what, kind) if a is not None and b is not None else (None, None, what, kind))
    return out, npath


def judge_part(g):
    """Border links of one part: list of (kind tag, from id, to id)."""
    if not g.subs:
        return None  # flat: never clustered
    sis, npath = stand_ins(g)
    border = []
    for (f, t, kind, tag) in g.edges:
        if f == t:
            continue
        if npath[f] != npath[t]:
            what = "edge"
            if kind in ("Invisible",):
                what = "invisible edge"
            if g.nodes[f]["note"] or g.nodes[t]["note"]:
                what = "note link (edge)"
            if tag == "sub-as-edge":
                what = "edge to a B9 proxy"
            border.append((what, g.nodes[f]["id"], g.nodes[t]["id"]))
    for (a, b, what, kind) in sis:
        if a is None:
            continue
        if npath[a] != npath[b]:
            border.append(("stand-in: " + what, g.nodes[a]["id"], g.nodes[b]["id"]))
    return border


def judge(path):
    g = load(path)
    parts = split(g)
    res = []
    if parts is None:
        parts = [("whole", g)]
    for name, pg in parts:
        b = judge_part(pg)
        res.append((name, b))
    return g, res


if __name__ == "__main__":
    verbose = "-v" in sys.argv
    for p in [a for a in sys.argv[1:] if a != "-v"]:
        g, res = judge(p)
        name = os.path.basename(p).replace(".struct.json", "")
        if not g.subs:
            print(f"{name}\tFLAT")
            continue
        on = any(b for _, b in res if b)
        kinds = {}
        for pn, b in res:
            for (what, f, t) in (b or []):
                kinds[what] = kinds.get(what, 0) + 1
        parts = ",".join(f"{pn}:{'flat' if b is None else ('ON' if b else 'off')}" for pn, b in res)
        degenerate = [w for (_, _, w, _) in stand_ins(g)[0] if any(k in w for k in ("itself", "own"))]
        only_degenerate = on and all(any(k in w for k in ("itself", "own")) for w in kinds)
        print(f"{name}\t{'BLOCK' if on else 'off'}\t{parts}\t{kinds}\tdegenerate={sorted(set(degenerate))}{' ONLY-DEGENERATE-BORDER' if only_degenerate else ''}")
        if verbose:
            for pn, b in res:
                for x in (b or []):
                    print("    ", pn, x)
