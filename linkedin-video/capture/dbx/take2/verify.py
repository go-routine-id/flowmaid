#!/usr/bin/env python3
"""Per-frame verification of clips/dbx2: map each fNNNN.png (hard link) back
to its capture by inode, run check.py on that capture's plain text."""
import glob, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check
W2 = os.path.dirname(os.path.abspath(__file__))
OUT = "/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/clips/dbx2"
ino = {os.stat(p).st_ino: os.path.basename(p)[:-4] for p in glob.glob(W2 + "/png/*.png")}
res = {}
dirty, clean, nonerd = [], [], []
for f in sorted(glob.glob(OUT + "/f*.png")):
    n = int(os.path.basename(f)[1:5])
    name = ino[os.stat(f).st_ino]
    if name not in res:
        try:
            d, w, info = check.check(open("%s/caps/%s.txt" % (W2, name), encoding="utf-8").read())
            res[name] = ("dirty" if d else "clean", d)
        except StopIteration:
            res[name] = ("no-erd", [])
    k = res[name][0]
    (dirty if k == "dirty" else clean if k == "clean" else nonerd).append((n, name))
def ranges(lst):
    out, prev = [], None
    for n, name in lst:
        if out and n == out[-1][1] + 1 and name == out[-1][2]:
            out[-1][1] = n
        else:
            out.append([n, n, name])
    return ["f%04d-f%04d %s" % tuple(r) for r in out]
print("frames: %d  clean ERD: %d  dirty ERD: %d  no ERD on screen: %d" % (len(dirty) + len(clean) + len(nonerd), len(clean), len(dirty), len(nonerd)))
print("dirty:", ranges(dirty))
for name, (k, d) in sorted(res.items()):
    if k == "dirty":
        print(" ", name, d)
print("no-erd:", ranges(nonerd))
