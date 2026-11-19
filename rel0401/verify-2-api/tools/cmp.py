import os, re, sys, collections
B, H = sys.argv[1], sys.argv[2]
num = re.compile(r'(?<![\w.#-])-?\d+(?:\.\d+)?(?![\w])')
vbre = re.compile(r'viewBox="([^"]*)"')
stat = collections.Counter(); bad = []; changed = collections.Counter(); notes = collections.Counter()
STATIC = {'static', 'scene', 'mscene'}
def vb(s):
    m = vbre.search(s); return [float(x) for x in m.group(1).split()] if m else None
for f in sorted(os.listdir(B)):
    if not f.endswith('.svg'): continue
    name, var, _ = f.rsplit('.', 2)
    kind = var.split('-')[0]
    b = open(os.path.join(B, f), encoding='utf8').read()
    hp = os.path.join(H, f)
    if not os.path.exists(hp): bad.append(('missing', f)); continue
    h = open(hp, encoding='utf8').read()
    if b == h: stat[('same', kind if kind in STATIC else 'routed')] += 1; continue
    if kind in STATIC:
        bad.append(('static-diff', f)); continue
    hv, bv = vb(h), vb(b)
    ox, oy = hv[0], hv[1]
    if ox == 0 and oy == 0:
        bad.append(('routed-diff-with-0-0-viewBox', f)); continue
    if ox > 0 or oy > 0:
        bad.append(('positive-origin', f, hv)); continue
    changed[kind] += 1
    # skeleton compare
    vbs = vbre.search(h).group(1).split()
    bgh = '<rect x="%s" y="%s" width=' % (vbs[0], vbs[1])
    nbg = h.count(bgh)
    if nbg: notes['bg-rect-at-origin'] += 1
    h2 = h.replace(bgh, '<rect width=', 1)
    hs = vbre.sub('VB', h2); bs = vbre.sub('VB', b)
    hn = num.findall(hs); bn = num.findall(bs)
    if num.sub('#', hs) != num.sub('#', bs) or len(hn) != len(bn):
        notes['skeleton-differs'] += 1
        bad.append(('skeleton', f)); continue
    odd = 0
    for x, y in zip(bn, hn):
        d = float(x) - float(y)
        if abs(d) < 0.111: continue
        if abs(d + ox) < 0.151 or abs(d + oy) < 0.151: continue
        odd += 1
    if odd:
        bad.append(('not-a-translation', f, odd, hv))
    else:
        stat['translated'] += 1
print('stats', dict(stat)); print('changed by kind', dict(changed))
kinds = collections.Counter(x[0] for x in bad); print('problems', dict(kinds))
for x in bad[:60]: print(*x)
