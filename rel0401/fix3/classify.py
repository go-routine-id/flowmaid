import sys, os, re, collections
sys.argv_saved = list(sys.argv)
src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'cmp3.py')).read().split('\ndef main')[0]
exec(src)
B, H, L = sys.argv_saved[1], sys.argv_saved[2], sys.argv_saved[3]
files = open(L).read().split('\n')
def dtype(i):
    for l in open(files[i]):
        l = l.strip()
        if l and not l.startswith('%%') and not l.startswith('---'):
            return l.split()[0]
    return '?'
out = collections.Counter()
for fn in sorted(os.listdir(B)):
    kind = fn.split('.')[-2]
    if kind in ('render', 'scene'): continue
    b, h = open(os.path.join(B, fn)).read(), open(os.path.join(H, fn)).read()
    hv = vb(h)
    if b == h or (hv[0] == 0 and hv[1] == 0): continue
    t = translated(b, h, hv)
    if t == 'ok': continue
    i = int(fn.split('.')[0]); dt = dtype(i)
    if t.startswith('off'):
        tb, th = tokens(background_fix(b)), tokens(background_fix(h))
        els = set()
        for (sb, xb), (sh, xh), lb in zip(tb, th, background_fix(b).split('\n')):
            for (kb, a), (kh, c) in zip(xb, xh):
                d = {'x': hv[0], 'y': hv[1], 'n': 0}[kb]
                if abs(a + d - c) > 0.1001:
                    els.add(lb.split(' ')[0]); break
        out[(dt, 'off', ','.join(sorted(els)))] += 1
    else:
        out[(dt, t)] += 1
for k, v in sorted(out.items()): print(v, k)
