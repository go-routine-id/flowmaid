"""Drawn node centres vs scene centres in head SVGs (flowchart/state)."""
import re, sys, html
from collections import Counter, defaultdict
d = sys.argv[1]
files, rows = {}, {}
for l in open(d + "/results.tsv"):
    f = l.rstrip("\n").split("\t")
    if f[1] == "file": files[f[0]] = f[4]; continue
    rows[(f[0], f[1])] = f
def centres(info):
    m = re.search(r"c=([^;]*)", info)
    if not m or not m.group(1): return []
    return [tuple(map(float, x.split(":")[:2])) for x in m.group(1).strip("|").split("|")]
def texts(svg, labels):
    """label -> (x, y) of the first <text> after its <g><title>."""
    out = {}
    for m in re.finditer(r"<g><title>([^<]*)</title>\n(.*?)</g>", svg, re.S):
        t = html.unescape(m.group(1))
        tm = re.search(r'<text x="([-\d.]+)" y="([-\d.]+)"', m.group(2))
        if tm:
            out.setdefault(t, []).append((float(tm.group(1)), float(tm.group(2))))
    return out
t = Counter(); ex = defaultdict(list)
for (fid, mode), f in rows.items():
    if mode in ("render", "scene") or "c=" not in f[4]: continue
    try:
        labels = open(f"{d}/{fid}.labels.txt").read().split("\n")
    except FileNotFoundError:
        continue
    cs = centres(rows[(fid, "scene")][4]); cd = centres(f[4])
    sv_s = open(f"{d}/{fid}.scene.svg").read(); sv_d = open(f"{d}/{fid}.{mode}.svg").read()
    ts, td = texts(sv_s, labels), texts(sv_d, labels)
    vb = [float(v) for v in re.search(r'<svg[^>]* viewBox="([^"]*)"', sv_d).group(1).split()]
    shifts = []
    moved = [i for i in range(len(cs)) if cs[i] != cd[i]]
    per = {}
    for i, lab in enumerate(labels):
        if "\\n" in lab or "<" in lab or labels.count(lab) != 1 or not lab.strip(): continue
        if lab not in ts or lab not in td or len(ts[lab]) != 1 or len(td[lab]) != 1: continue
        (sx, sy), (dx, dy) = ts[lab][0], td[lab][0]
        s = ((dx - cd[i][0]) - (sx - cs[i][0]), (dy - cd[i][1]) - (sy - cs[i][1]))
        shifts.append(s); per[i] = s
    if len(shifts) < 1:
        t[(mode, "no labels")] += 1; continue
    xs = sorted(s[0] for s in shifts); ys = sorted(s[1] for s in shifts)
    spread = max(xs[-1] - xs[0], ys[-1] - ys[0])
    shx, shy = xs[len(xs)//2], ys[len(ys)//2]
    over_x, over_y = vb[0] < 0, vb[1] < 0
    key = []
    if spread > 0.21: key.append("NONCONST")
    if over_x and abs(shx) > 0.11: key.append("x-overflow-but-shifted")
    if over_y and abs(shy) > 0.11: key.append("y-overflow-but-shifted")
    if abs(shx) > 0.11 or abs(shy) > 0.11:
        key.append("svg!=scene" + (" (overflow other axis)" if (over_x or over_y) else " (no overflow)"))
    dragged_ok = all(abs(per[i][0]) <= 0.11 and abs(per[i][1]) <= 0.11 for i in moved if i in per)
    if moved and not dragged_ok: key.append("dragged-not-at-caller")
    k = " ".join(key) or "drawn==scene"
    t[(mode, k)] += 1
    ex[k].append((files[fid].split("/")[-1], mode, round(shx, 2), round(shy, 2), vb[:2], moved[:3]))
for k in sorted(t): print(k, t[k])
for k, v in ex.items():
    if k == "drawn==scene": continue
    print("==", k, len(v))
    for e in v[:12]: print("   ", e)

# Pick at the dragged node's drawn centre, mapped through the viewBox
# (SVG user space taken as scene space): inside its own box?
print("\n== pick at drawn centre of the dragged node (outside own box => wrong pick)")
pk = Counter(); pex = []
def cwh(info):
    m = re.search(r"c=([^;]*)", info)
    return [tuple(map(float, x.split(":"))) for x in m.group(1).strip("|").split("|")]
for (fid, mode), f in rows.items():
    if not re.match(r"d(1|2|3|4|5|9|10|12)\.", mode) or "c=" not in f[4]: continue
    try: labels = open(f"{d}/{fid}.labels.txt").read().split("\n")
    except FileNotFoundError: continue
    cs = centres(rows[(fid, "scene")][4]); nd = cwh(f[4])
    sv_s = open(f"{d}/{fid}.scene.svg").read(); sv_d = open(f"{d}/{fid}.{mode}.svg").read()
    ts, td = texts(sv_s, labels), texts(sv_d, labels)
    moved = [i for i in range(len(cs)) if cs[i] != nd[i][:2]]
    for i in moved:
        lab = labels[i]
        if "\\n" in lab or "<" in lab or labels.count(lab) != 1 or not lab.strip(): continue
        if lab not in ts or lab not in td or len(ts[lab]) != 1 or len(td[lab]) != 1: continue
        (sx, sy), (dx, dy) = ts[lab][0], td[lab][0]
        s = ((dx - nd[i][0]) - (sx - cs[i][0]), (dy - nd[i][1]) - (sy - cs[i][1]))
        inside = abs(s[0]) <= nd[i][2] / 2 and abs(s[1]) <= nd[i][3] / 2
        pk[(mode.split(".")[0], "ok" if inside and max(map(abs, s)) <= 0.11 else "inside-but-off" if inside else "MISS")] += 1
        if not inside: pex.append((files[fid].split("/")[-1], mode, lab, [round(v, 1) for v in s], nd[i][2:]))
for k in sorted(pk): print(k, pk[k])
for e in pex[:25]: print("   ", e)
