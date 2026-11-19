#!/usr/bin/env python3
"""detach_cmp.py pre.json head.json [thr]: chips newly detached (> thr in head, <= thr in pre) and farther (> thr and > pre + 0.5)."""
import sys, json
a = json.load(open(sys.argv[1])); b = json.load(open(sys.argv[2])); thr = float(sys.argv[3]) if len(sys.argv) > 3 else 3.0
new = far = better = 0; n3a = n3b = 0
lines = []
for name in sorted(set(a) | set(b)):
    la = a.get(name, []); lb = b.get(name, [])
    # match by (edge, text) occurrence order
    from collections import defaultdict
    da = defaultdict(list); db = defaultdict(list)
    for e, t, d in la: da[(e, t)].append(d)
    for e, t, d in lb: db[(e, t)].append(d)
    n3a += sum(1 for _, _, d in la if d > thr); n3b += sum(1 for _, _, d in lb if d > thr)
    for k in sorted(set(da) | set(db)):
        xa = da.get(k, []); xb = db.get(k, [])
        for j in range(max(len(xa), len(xb))):
            pa = xa[j] if j < len(xa) else None; pb = xb[j] if j < len(xb) else None
            if pb is None or pa is None:
                if (pb or 0) > thr: lines.append(f"UNMATCHED {name} {k} pre {pa} head {pb}"); new += 1
                continue
            if pb > thr and pa <= thr: new += 1; lines.append(f"NEW   {name:55} {k[0]:28} {k[1]:22} {pa:6.1f} -> {pb:6.1f}")
            elif pb > thr and pb > pa + 0.5: far += 1; lines.append(f"FAR   {name:55} {k[0]:28} {k[1]:22} {pa:6.1f} -> {pb:6.1f}")
            elif pa > thr and pb < pa - 0.5: better += 1; lines.append(f"better {name:54} {k[0]:28} {k[1]:22} {pa:6.1f} -> {pb:6.1f}")
print(f"detached > {thr}px: pre {n3a} head {n3b}; newly {new}, farther {far}, better {better}")
for l in lines: print(l)
