#!/usr/bin/env python3
# Compare base vs head SVG dumps: identical / pure translation into viewBox / other.
import os, re, sys
from collections import Counter, defaultdict

B, H = sys.argv[1], sys.argv[2]
NUM = re.compile(r'(?<![A-Za-z0-9_.])-?\d+(?:\.\d+)?(?![A-Za-z0-9_])')
VB = re.compile(r' viewBox="([^"]*)"')


def vb(s):
    m = VB.search(s)
    return [float(x) for x in m.group(1).split()] if m else None


def body(s):
    lines = s.split('\n')
    out = []
    bg_done = False
    for l in lines:
        if l.startswith('<svg '):
            continue
        if not bg_done and l.startswith('<rect ') and 'width=' in l:
            l = re.sub(r'^<rect x="[^"]*" y="[^"]*" ', '<rect ', l)
            bg_done = True
        out.append(l)
    return '\n'.join(out)


def translated(bs, hs, vx, vy, tol=0.151):
    bb, hb = body(bs), body(hs)
    bsk, hsk = NUM.split(bb), NUM.split(hb)
    if bsk != hsk:
        return False, 'skeleton differs'
    bn, hn = NUM.findall(bb), NUM.findall(hb)
    bad = 0
    first = None
    for a, b in zip(bn, hn):
        d = float(b) - float(a)
        if abs(d) <= tol or abs(d - vx) <= tol or abs(d - vy) <= tol:
            continue
        bad += 1
        if first is None:
            first = (a, b)
    return bad == 0, f'{bad} numbers off, first {first}'


def meta(p):
    try:
        ls = open(p).read().split('\n')
    except FileNotFoundError:
        return None
    o = [float(x) for x in ls[0].split()]
    nodes = [l for l in ls[1:] if l]
    return o, nodes


files = sorted(f for f in os.listdir(H) if f.endswith('.svg'))
stats = defaultdict(Counter)
others = []
pos_only = []
origin_mismatch = []
node_moved = []
missing = 0
for f in files:
    pb, ph = os.path.join(B, f), os.path.join(H, f)
    if not os.path.exists(pb):
        missing += 1
        continue
    bs, hs = open(pb).read(), open(ph).read()
    variant = f.rsplit('.', 2)[1]
    kind = variant.split('-')[0]
    mh = meta(ph[:-4] + '.meta')
    mb = meta(pb[:-4] + '.meta')
    if mh and mb and mh[1] != mb[1]:
        node_moved.append(f)
    if bs == hs:
        stats[kind]['identical'] += 1
        v = vb(hs)
        if mh and v and (mh[0][0] != 0 or mh[0][1] != 0):
            stats[kind]['identical-but-origin-nonzero'] += 1
        continue
    vh, vbb = vb(hs), vb(bs)
    if not vh or not vbb:
        stats[kind]['other(no viewBox)'] += 1
        others.append((f, 'no viewBox'))
        continue
    vx, vy = vh[0], vh[1]
    if vbb[0] != 0 or vbb[1] != 0:
        stats[kind]['base-vb-nonzero'] += 1
    ok, why = translated(bs, hs, vx, vy)
    sign = 'neg' if (vx < 0 or vy < 0) else ('pos-only' if (vx > 0 or vy > 0) else 'zero')
    if ok:
        stats[kind][f'translated-{sign}'] += 1
        if sign == 'pos-only':
            pos_only.append((f, vx, vy))
    else:
        stats[kind][f'OTHER-{sign}'] += 1
        others.append((f, why))
    if mh:
        ox, oy = mh[0][0], mh[0][1]
        # A negative viewBox origin component must equal Scene.origin's.
        for v, o in ((vx, ox), (vy, oy)):
            if v < -0.05 and abs(v - o) > 0.051:
                origin_mismatch.append((f, (vx, vy), (ox, oy)))
                break
            if o < -0.05 and v > o + 0.051 and kind in ('route', 'partial') and 'class' not in f:
                pass

print('missing in base:', missing)
for k in sorted(stats):
    print(k, dict(stats[k]))
print('scene nodes differ base vs head:', len(node_moved), node_moved[:10])
print('negative viewBox origin != Scene.origin:', len(origin_mismatch))
for x in origin_mismatch[:10]:
    print('  ', x)
print('changed with positive-only viewBox origin:', len(pos_only))
for x in pos_only[:15]:
    print('  ', x)
print('OTHER (not a pure translation):', len(others))
for x in others[:40]:
    print('  ', x)
