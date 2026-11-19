import re, sys
from collections import Counter
hd = sys.argv[1]
files = {}; t = Counter(); ex = []
for l in open(hd + "/results.tsv"):
    f = l.rstrip("\n").split("\t")
    if f[1] == "file": files[f[0]] = f[4]; continue
    m = re.search(r"o=([-\d.]+),([-\d.]+)", f[4])
    if not m: continue
    ox, oy = float(m.group(1)), float(m.group(2))
    mode = f[1]
    if f[3] == "diff":
        svg = open(f"{hd}/{f[0]}.{mode}.svg").read()
        vb = [float(v) for v in re.search(r'<svg[^>]* viewBox="([^"]*)"', svg).group(1).split()]
        ok = abs(vb[0] - ox) <= 0.051 and abs(vb[1] - oy) <= 0.051
        t[(mode, "diff", "vb==origin" if ok else "vb!=origin")] += 1
        if not ok: ex.append((files[f[0]].split("/")[-1], mode, (ox, oy), vb[:2]))
    else:
        t[(mode, "same", "o!=0" if (ox, oy) != (0, 0) else "o=0")] += 1
        if (ox, oy) != (0, 0): ex.append((files[f[0]].split("/")[-1], mode, (ox, oy), "same svg"))
for k in sorted(t): print(k, t[k])
print(len(ex)); [print("  ", e) for e in ex[:30]]
