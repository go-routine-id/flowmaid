"""cmp3.py BASE_DIR HEAD_DIR [--prev PREV_DIR]: compare dumped SVGs.

Static kinds (render, scene) must be byte-identical. A routed SVG whose head
viewBox starts at 0 0 must be byte-identical. Any other routed SVG must be
the base SVG translated by the head viewBox origin (vx, vy) on both axes:
x-like attributes and path/points x coordinates move by vx, y-like by vy,
everything else is unchanged (0.1 px print tolerance per number).
With --prev, also classify how head differs from prev.
"""
import os, re, sys, collections

num_re = re.compile(r'-?\d+(?:\.\d+)?(?:e-?\d+)?')
attr_re = re.compile(r' ([a-zA-Z0-9_:-]+)="([^"]*)"')
NS = {'width', 'height'}
XS = {'x', 'cx', 'x1', 'x2'}
YS = {'y', 'cy', 'y1', 'y2'}


def vb(s):
    m = re.search(r' viewBox="([^"]*)"', s)
    return [float(x) for x in m.group(1).split()]


def path_pairs(d):
    """[(kind, value)] where kind is 'x', 'y' or 'n' (not a coordinate)."""
    out = []
    runs = []
    for tok in re.findall(r'[A-Za-z]|-?\d+(?:\.\d+)?(?:e-?\d+)?', d):
        if tok.isalpha():
            runs.append((tok.upper(), []))
        else:
            if not runs:
                runs.append(('P', []))
            runs[-1][1].append(float(tok))
    for cmd, ns in runs:
        if cmd == 'A':
            for i in range(0, len(ns), 7):
                g = ns[i:i + 7]
                out += [('n', v) for v in g[:5]] + [('x', g[5]), ('y', g[6])]
        elif cmd == 'H':
            out += [('x', v) for v in ns]
        elif cmd == 'V':
            out += [('y', v) for v in ns]
        elif cmd == 'Z':
            pass
        else:
            for i, v in enumerate(ns):
                out.append(('x' if i % 2 == 0 else 'y', v))
    return out


def tokens(svg):
    """Per line: (skeleton, [(kind, value)])."""
    res = []
    for l in svg.split('\n'):
        # marker internals are in their own coordinates: compared verbatim;
        # the rest of <defs> (hop masks) is in user space
        markers = re.findall(r'<marker .*?</marker>', l)
        l = re.sub(r'<marker .*?</marker>', '<MARKER/>', l)
        if l.startswith('<svg '):
            l = re.sub(r' viewBox="[^"]*"', '', l)
        vals = []
        for m in attr_re.finditer(l):
            k, v = m.group(1), m.group(2)
            if k in XS or k in YS or (k in NS and not l.startswith('<svg ')):
                try:
                    vals.append(('x' if k in XS else 'y' if k in YS else 'n', float(v)))
                except ValueError:
                    pass
            elif k in ('d', 'points'):
                vals += path_pairs(v)
        skel = attr_re.sub(lambda m: ' %s=#' % m.group(1) if m.group(1) in XS | YS | {'d', 'points'} | (set() if l.startswith('<svg ') else NS) else m.group(0), l)
        res.append((skel + '|'.join(markers), vals))
    return res


def background_fix(svg):
    # the background rect writes x/y only at a non-zero corner
    return re.sub(r'^<rect x="[^"]*" y="[^"]*" (width="[^"]*" height="[^"]*" fill=)', r'<rect \1', svg, count=1, flags=re.M)


def translated(b, h, v):
    tb, th = tokens(background_fix(b)), tokens(background_fix(h))
    if len(tb) != len(th):
        return 'lines'
    worst = 0.0
    for (sb, vb_), (sh, vh) in zip(tb, th):
        if sb != sh or len(vb_) != len(vh):
            return 'skeleton'
        for (kb, xb), (kh, xh) in zip(vb_, vh):
            d = {'x': v[0], 'y': v[1], 'n': 0.0}[kb]
            worst = max(worst, abs(xb + d - xh))
    return 'ok' if worst <= 0.1001 else 'off %.2f' % worst


def main():
    B, H = sys.argv[1], sys.argv[2]
    P = sys.argv[sys.argv.index('--prev') + 1] if '--prev' in sys.argv else None
    res = collections.Counter()
    bad = []
    pos = collections.Counter()
    vsp = collections.Counter()
    for fn in sorted(os.listdir(B)):
        kind = fn.split('.')[-2]
        hp = os.path.join(H, fn)
        if not os.path.exists(hp):
            res['missing in head'] += 1
            continue
        b, h = open(os.path.join(B, fn)).read(), open(hp).read()
        static = kind in ('render', 'scene')
        hv = vb(h)
        if static:
            res[('static', 'same' if b == h else 'CHANGED')] += 1
            if b != h:
                bad.append((fn, 'static changed'))
            continue
        if hv[0] == 0 and hv[1] == 0:
            res[('routed vb 0 0', 'same' if b == h else 'CHANGED')] += 1
            if b != h:
                bad.append((fn, 'vb00 changed'))
            continue
        if b == h:
            res[('routed vb !=0', 'same?!')] += 1
            bad.append((fn, 'vb nonzero but same'))
            continue
        bv = vb(b)
        if bv[0] != 0 or bv[1] != 0:
            res['base vb not 0 0'] += 1
        if abs(hv[2] - bv[2]) > 0.5 or abs(hv[3] - bv[3]) > 0.5:
            res[('routed vb !=0', 'size differs')] += 1
            bad.append((fn, 'size', bv, hv))
            continue
        t = translated(b, h, hv)
        res[('routed vb !=0', 'translation' if t == 'ok' else 'NOT translation')] += 1
        pos[('x>0' if hv[0] > 0 else 'x<0' if hv[0] < 0 else 'x=0') + ' ' + ('y>0' if hv[1] > 0 else 'y<0' if hv[1] < 0 else 'y=0')] += 1
        if t != 'ok':
            bad.append((fn, t, hv[:2]))
        if P:
            p = open(os.path.join(P, fn)).read()
            pv = vb(p)
            if p == h:
                vsp['prev==head'] += 1
            else:
                # prev shifted the axis that head now keeps: prev vb 0 there
                why = []
                for i, a in enumerate('xy'):
                    if pv[i] == 0 and hv[i] != 0:
                        why.append(a)
                    elif pv[i] != hv[i]:
                        why.append(a + '!')
                vsp['prev!=head: newly kept ' + ','.join(why)] += 1
    for k, v in sorted(res.items(), key=str):
        print(v, k)
    print('head viewBox origin signs among changed:', dict(pos))
    if P:
        print('vs prev:', dict(vsp))
    print('bad', len(bad))
    for x in bad[:40]:
        print('  ', x)


main()
