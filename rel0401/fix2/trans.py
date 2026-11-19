import sys, re, os, collections
A, Bd = sys.argv[1], sys.argv[2]
num = re.compile(r'-?\d+(?:\.\d+)?(?:e-?\d+)?')
def vb(s):
    m = re.search(r' viewBox="([^"]*)"', s); return [float(x) for x in m.group(1).split()]
def body(s):
    out = []
    for l in s.split('\n'):
        if l.startswith('<svg '): l = re.sub(r' viewBox="[^"]*"', '', l)
        if l.startswith('<rect ') and 'fill=' in l and len(out) < 3:
            l = re.sub(r'^<rect x="[^"]*" y="[^"]*" ', '<rect ', l)
        out.append(l)
    return '\n'.join(out)
res = collections.Counter(); bad = []
for fn in sorted(os.listdir(Bd)):
    kind = fn.rsplit('.', 2)[1]
    p = os.path.join(A, fn)
    if not os.path.exists(p): res['missing'] += 1; continue
    b = open(os.path.join(Bd, fn)).read(); h = open(p).read()
    static = kind in ('static', 'scene')
    if b == h: res[('same', 'static' if static else 'routed')] += 1; continue
    if static: res['STATIC CHANGED'] += 1; bad.append((fn, 'static')); continue
    hv = vb(h)
    if hv[0] == 0 and hv[1] == 0: res['VB00 CHANGED'] += 1; bad.append((fn, 'vb00')); continue
    if hv[0] > 0 or hv[1] > 0: res['VB POSITIVE'] += 1; bad.append((fn, 'pos', hv)); continue
    bv = vb(b)
    if abs(hv[2]-bv[2]) > 0.5 or abs(hv[3]-bv[3]) > 0.5: res['size_diff'] += 1; bad.append((fn, 'size', bv, hv)); continue
    bb, hb = body(b), body(h)
    if num.split(bb) != num.split(hb): res['skeleton_diff'] += 1; bad.append((fn, 'skeleton')); continue
    nb = [float(x) for x in num.findall(bb)]; nh = [float(x) for x in num.findall(hb)]
    odd = [(x, y) for x, y in zip(nb, nh) if min(abs(x-y), abs(x-y+hv[0]), abs(x-y+hv[1])) > 0.1001]
    if odd: res['not_translation'] += 1; bad.append((fn, 'odd', odd[:4], hv[:2]))
    else: res['translation'] += 1
for k, v in sorted(res.items(), key=str): print(v, k)
for x in bad[:30]: print(x)
