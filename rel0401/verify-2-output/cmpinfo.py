import re, sys
from collections import Counter
hd, bd = sys.argv[1], sys.argv[2]
def load(d):
    files, rows = {}, {}
    for l in open(d + "/results.tsv"):
        f = l.rstrip("\n").split("\t")
        if f[1] == "file":
            files[f[0]] = f[4]; continue
        rows[(f[0], f[1])] = f
    return files, rows
fh, rh = load(hd); fb, rb = load(bd)
assert fh == fb
def kv(s):
    return dict(x.split("=", 1) for x in s.split(";") if "=" in x)
t = Counter(); bad = []
for k, h in rh.items():
    b = rb.get(k)
    if b is None:
        bad.append(("missing in base", fh[k[0]], k[1])); continue
    if h[4] == "" and b[4] == "":
        continue
    H, B = kv(h[4]), kv(b[4])
    for f in ("geom", "hits", "hitok", "pos"):
        if H.get(f) != B.get(f):
            t[(k[1].split(".")[0], f, "DIFF")] += 1
            bad.append((f, fh[k[0]], k[1], B.get(f), H.get(f)))
        else:
            t[(f, "same")] += 1
    if "pos" in H and H["pos"] != "1":
        bad.append(("pos!=1", fh[k[0]], k[1]))
    # width/height: head canvas = base canvas grown left/up by -origin
    if "o" in H:
        ox, oy = map(float, H["o"].split(","))
        if ox > 0 or oy > 0:
            bad.append(("origin>0", fh[k[0]], k[1], H["o"]))
        wb, hb, wh, hh = float(B["w"]), float(B["h"]), float(H["w"]), float(H["h"])
        t[("o!=0" if (ox, oy) != (0, 0) else "o==0", "rows")] += 1
        if (ox, oy) == (0, 0) and (abs(wb - wh) > 1e-9 or abs(hb - hh) > 1e-9):
            bad.append(("wh changed at o=0", fh[k[0]], k[1], B["w"], H["w"], B["h"], H["h"]))
for k in sorted(t): print(k, t[k])
print("bad:", len(bad))
for x in bad[:40]: print("  ", x)
