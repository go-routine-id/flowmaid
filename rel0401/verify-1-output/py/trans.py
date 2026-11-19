# For each (file, kind) whose SVG differs, check head == base translated by the head viewBox origin.
import sys, re, json, collections
V = sys.argv[1]
lst = [l.strip() for l in open(V + '/main.lst') if l.strip()]
num = re.compile(r'-?\d+(?:\.\d+)?(?:e-?\d+)?')
def vb(s):
    m = re.search(r' viewBox="([^"]*)"', s); return [float(x) for x in m.group(1).split()]
def body(s):
    lines = s.split('\n')
    out = []
    for l in lines:
        if l.startswith('<svg ') : l = re.sub(r' viewBox="[^"]*"', '', l)
        if l.startswith('<rect ') and 'fill=' in l and len(out) < 3:
            l = re.sub(r'^<rect x="[^"]*" y="[^"]*" ', '<rect ', l)
        out.append(l)
    return '\n'.join(out)
res = collections.Counter(); bad = []
import os
for fn in sorted(os.listdir(V + '/dump-head')):
    i, kind, _ = fn.split('.', 2)
    if kind in ('render', 'scene'): continue
    p = V + '/dump-base/' + fn
    if not os.path.exists(p): res['nobase'] += 1; continue
    b = open(p).read(); h = open(V + '/dump-head/' + fn).read()
    if b == h: res['same'] += 1; continue
    hv = vb(h); bv = vb(b)
    if bv[0] != 0 or bv[1] != 0: res['base_vb_nonzero'] += 1
    if abs(hv[2]-bv[2]) > 0.5 or abs(hv[3]-bv[3]) > 0.5: res['size_diff'] += 1; bad.append((lst[int(i)], kind, 'size', bv, hv)); continue
    bb, hb = body(b), body(h)
    sb, sh = num.split(bb), num.split(hb)
    if sb != sh: res['skeleton_diff'] += 1; bad.append((lst[int(i)], kind, 'skeleton')); continue
    nb = [float(x) for x in num.findall(bb)]; nh = [float(x) for x in num.findall(hb)]
    worst = 0; odd = []
    for x, y in zip(nb, nh):
        d = x - y
        e = min(abs(d), abs(d + hv[0]), abs(d + hv[1]))
        if e > 0.1001: odd.append((x, y))
        worst = max(worst, e)
    if odd: res['not_translation'] += 1; bad.append((lst[int(i)], kind, 'odd', odd[:6], hv[:2]))
    else: res['translation'] += 1
for k, v in sorted(res.items()): print(v, k)
for x in bad[:40]: print(x)
