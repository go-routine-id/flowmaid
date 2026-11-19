import sys, os, re, collections
V, lstf, pre = sys.argv[1:4]
lst = [l.strip() for l in open(V + '/' + lstf) if l.strip()]
num = re.compile(r'-?\d+(?:\.\d+)?(?:e-?\d+)?')
def vb(s):
    m = re.search(r' viewBox="([^"]*)"', s); return [float(x) for x in m.group(1).split()]
res = collections.Counter(); samples = collections.defaultdict(list)
for fn in sorted(os.listdir(f'{V}/{pre}-head')):
    i, kind, _ = fn.split('.', 2)
    if kind in ('render', 'scene'): continue
    b = open(f'{V}/{pre}-base/{fn}').read(); h = open(f'{V}/{pre}-head/{fn}').read()
    if b == h: continue
    hv = vb(h)
    bl, hl = b.split('\n'), h.split('\n')
    if len(bl) != len(hl): res['linecount'] += 1; samples['linecount'].append((lst[int(i)], kind)); continue
    typ = open(lst[int(i)]).read().split()[0]
    kinds = set()
    for x, y in zip(bl[3:], hl[3:]):
        if num.split(x) != num.split(y): kinds.add('skeleton:' + x[:40]); continue
        for p, q in zip(num.findall(x), num.findall(y)):
            d = float(p) - float(q)
            if min(abs(d), abs(d + hv[0]), abs(d + hv[1])) > 0.1001:
                kinds.add('card' if 'font-size="11"' in y and y.startswith('<text') else 'other:' + y[:70])
                break
    if kinds:
        for k in kinds: res[(typ, kind, k if not k.startswith('other') else 'other')] += 1
        for k in kinds:
            if k != 'card': samples[k].append((lst[int(i)], kind))
for k, v in sorted(res.items(), key=str): print(v, k)
for k, v in samples.items(): print(k, v[:3])
