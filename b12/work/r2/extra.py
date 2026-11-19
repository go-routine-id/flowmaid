#!/usr/bin/env python3
"""extra.py <name> <svg> <dbg>: drawn crossing pairs (blockmetrics) against layered inversions (FM_XXDBG dump)."""
import json, sys, subprocess, collections
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
name, svg, dbg = sys.argv[1:4]
import os, tempfile, shutil
td = tempfile.mkdtemp(); svg2 = os.path.join(td, name + '.svg'); shutil.copy(svg, svg2)
os.system(S+'/b12/metrics/blockmetrics --struct-dir '+S+'/b12/local/struct --json '+svg2+' > '+td+'/o.json 2>/dev/null')
j = json.load(open(td+'/o.json'))
j = j[0] if isinstance(j, list) else j
cr = j['detail']['crossings']; edges = j['detail']['edges']
if '-v' in sys.argv: print(cr[0], edges[0])
st = json.load(open(S+'/b12/local/struct/'+name+'.struct.json'))
# dump: edge of each link
d = json.loads(open(dbg).read().split("\n====\n")[0]); N = d["nodes"]
def owner(u, v):
    # the (from,to) of a link u->v in the augmented graph: a dummy's canon names its edge's ends
    for x in (u, v):
        if N[x]["run"] >= 0: return (N[x]["canon"][0], N[x]["canon"][2])
    if N[u]["border"] or N[v]["border"]: return None
    return (u, v)
inv = collections.Counter()
for l in range(d["nlayers"]-1):
    es = [(u, v) for u, n in enumerate(N) if n["l"] == l for v in n["succs"] if N[v]["l"] == l+1]
    for i in range(len(es)):
        for k in range(i+1, len(es)):
            a, b = es[i], es[k]
            if (N[a[0]]["pos"]-N[b[0]]["pos"])*(N[a[1]]["pos"]-N[b[1]]["pos"]) < 0:
                oa, ob = owner(*a), owner(*b)
                if oa and ob: inv[frozenset([frozenset(oa), frozenset(ob)])] += 1
print("layered inversions (pairs):", len(inv), "total", sum(inv.values()))
print("drawn crossings:", len(cr))
print("keys", list(cr[0].keys()) if cr else None)
ids = [n["id"] for n in st["nodes"]] if "nodes" in st else None
def nm(i): return ids[i] if (ids and isinstance(i, int)) else str(i)
def parse(s):
    a, b = s.split('>')
    return tuple(int(x[1:]) if x.startswith('#') else (ids.index(x) if x in ids else x) for x in (a, b))
drawn = collections.Counter()
for c in cr:
    drawn[frozenset([frozenset(parse(c['a'])), frozenset(parse(c['b']))])] += 1
both = sum(min(drawn[k], inv[k]) for k in drawn if k in inv)
extra_pairs = {k: drawn[k] - inv.get(k, 0) for k in drawn if drawn[k] > inv.get(k, 0)}
miss_pairs = {k: inv[k] - drawn.get(k, 0) for k in inv if inv[k] > drawn.get(k, 0)}
print("drawn & layered:", both, " extra drawn:", sum(extra_pairs.values()), " layered not drawn:", sum(miss_pairs.values()))
def fmt(k):
    es = [sorted(e, key=str) for e in k]
    return " x ".join(f"{nm(e[0])}~{nm(e[1])}" for e in es)
per = collections.Counter()
for k, c in extra_pairs.items():
    for e in k: per[tuple(sorted(e, key=str))] += c
print("extra crossings per edge:", ", ".join(f"{nm(a)}~{nm(b)}:{c}" for (a, b), c in per.most_common(16)))
share = sum(c for k, c in extra_pairs.items() if len(set().union(*k)) < 4)
print("extra between two edges of one node:", share)
if '-l' in sys.argv:
    for k, c in sorted(extra_pairs.items(), key=lambda kv: -kv[1]): print("  extra", c, fmt(k))
    for k, c in miss_pairs.items(): print("  undrawn", c, fmt(k))
