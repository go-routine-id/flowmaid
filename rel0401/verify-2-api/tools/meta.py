import os, re, sys, collections
H = sys.argv[1]
vbre = re.compile(r'viewBox="([^"]*)"')
c = collections.Counter(); bad = []
for f in sorted(os.listdir(H)):
    if not f.endswith('.meta'): continue
    base = f[:-5]; name, var = base.rsplit('.', 1)
    svg = open(os.path.join(H, base + '.svg')).read()
    vb = [float(x) for x in vbre.search(svg).group(1).split()]
    lines = open(os.path.join(H, f)).read().strip().split('\n')
    ox, oy, w, h = map(float, lines[0].split())
    kind = var.split('-')[0]
    isclass = 'routeall' in var or 'route0c' in var or 'class' in name or 'er' in name.lower()
    # positions kept
    for i, l in enumerate(lines[1:]):
        parts = l.split(' ', 6)
        x, y, nw, nh, gx, gy = map(float, parts[:6]); hit = parts[6]
        if gx == gx:  # not NaN
            if abs(x - gx) > 1e-9 or abs(y - gy) > 1e-9:
                bad.append(('moved', base, i, x, gx, y, gy)); break
        if not hit.startswith('Some(Node('):
            bad.append(('pick-miss', base, i, hit)); 
    c['files'] += 1
    # origin vs viewBox
    for ax, (o, v) in enumerate([(ox, vb[0]), (oy, vb[1])]):
        if v < 0:
            if kind in ('partial', 'partialback'):
                if v < o - 0.051: bad.append(('vb-before-origin', base, ax, o, v))
                c['partial-neg'] += 1
            elif abs(round(o, 1) - v) > 0.051:
                bad.append(('vb!=origin', base, ax, o, v))
            else: c['neg-match'] += 1
        else:
            if v > 0: bad.append(('positive-vb', base, ax, o, v))
            if o != 0 and kind not in ('partial', 'partialback'):
                # origin non-zero but viewBox 0: allowed only if sub-0.1 (flowchart) 
                bad.append(('origin-nonzero-vb0', base, ax, o, v))
print(dict(c)); k = collections.Counter(b[0] for b in bad); print(dict(k))
for b in bad[:30]: print(*b)
