#!/usr/bin/env python3
"""Count crossings between the relation paths of a flowmaid class-diagram SVG.

Edge paths = the run of <path fill="none"> elements right after the
background rect and before the first node rect.  Each path is flattened
(beziers sampled), and crossings between different edges are counted:
points where the two polylines meet transversally, clustered at 1.5 px.
Collinear overlaps (running along each other) are reported separately.
"""
import re
import sys
import math

NUM = r'-?\d+(?:\.\d+)?'


def edge_paths(svg):
    # elements in order
    els = re.findall(r'<(\w+)([^>]*)>', svg)
    out = []
    started = False
    for tag, attrs in els:
        if tag == 'rect' and ' x=' not in attrs and not started:
            started = True
            continue
        if not started:
            continue
        if tag == 'path' and 'fill="none"' in attrs:
            m = re.search(r' d="([^"]*)"', attrs)
            out.append(m.group(1))
            continue
        break
    return out


def flatten(d, n=16):
    toks = re.findall(r'[A-Za-z]|' + NUM, d)
    pts = []
    i = 0
    cur = None
    cmd = None
    cmds = set()
    while i < len(toks):
        t = toks[i]
        if t.isalpha():
            cmd = t
            cmds.add(t)
            i += 1
            if cmd in 'Zz':
                continue
        if cmd == 'M' or cmd == 'L':
            x, y = float(toks[i]), float(toks[i + 1])
            i += 2
            cur = (x, y)
            pts.append(cur)
        elif cmd == 'C':
            x1, y1, x2, y2, x, y = (float(v) for v in toks[i:i + 6])
            i += 6
            p0 = cur
            for k in range(1, n + 1):
                s = k / n
                a = (1 - s) ** 3
                b = 3 * (1 - s) ** 2 * s
                c = 3 * (1 - s) * s * s
                e = s ** 3
                pts.append((a * p0[0] + b * x1 + c * x2 + e * x,
                            a * p0[1] + b * y1 + c * y2 + e * y))
            cur = (x, y)
        elif cmd == 'Q':
            x1, y1, x, y = (float(v) for v in toks[i:i + 4])
            i += 4
            p0 = cur
            for k in range(1, n + 1):
                s = k / n
                a = (1 - s) ** 2
                b = 2 * (1 - s) * s
                c = s * s
                pts.append((a * p0[0] + b * x1 + c * x,
                            a * p0[1] + b * y1 + c * y))
            cur = (x, y)
        else:
            raise SystemExit('unhandled path command %r in %r' % (cmd, d[:80]))
    # drop zero-length steps, then merge exactly collinear same-way steps
    res = [pts[0]]
    for p in pts[1:]:
        if abs(p[0] - res[-1][0]) > 1e-9 or abs(p[1] - res[-1][1]) > 1e-9:
            res.append(p)
    out = [res[0]]
    for k in range(1, len(res)):
        if len(out) >= 2:
            a, b, c = out[-2], out[-1], res[k]
            cr = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
            dt = (b[0] - a[0]) * (c[0] - b[0]) + (b[1] - a[1]) * (c[1] - b[1])
            if abs(cr) < 1e-9 and dt > 0:
                out[-1] = c
                continue
        out.append(res[k])
    return out, cmds


def seg_x(p, q, r, s):
    """Intersection of segments pq and rs.  Returns ('x', point), ('o', len) or None."""
    d1 = (q[0] - p[0], q[1] - p[1])
    d2 = (s[0] - r[0], s[1] - r[1])
    den = d1[0] * d2[1] - d1[1] * d2[0]
    w = (r[0] - p[0], r[1] - p[1])
    l1 = math.hypot(*d1)
    l2 = math.hypot(*d2)
    if abs(den) < 1e-9 * l1 * l2 + 1e-12:
        # parallel: collinear overlap?
        dist = abs(w[0] * d1[1] - w[1] * d1[0]) / l1
        if dist > 0.05:
            return None
        ta = (w[0] * d1[0] + w[1] * d1[1]) / (l1 * l1)
        tb = ((s[0] - p[0]) * d1[0] + (s[1] - p[1]) * d1[1]) / (l1 * l1)
        lo, hi = max(0.0, min(ta, tb)), min(1.0, max(ta, tb))
        if hi - lo > 0:
            return ('o', (hi - lo) * l1)
        return None
    t = (w[0] * d2[1] - w[1] * d2[0]) / den
    u = (w[0] * d1[1] - w[1] * d1[0]) / den
    eps = 1e-9
    if -eps <= t <= 1 + eps and -eps <= u <= 1 + eps:
        return ('x', (p[0] + t * d1[0], p[1] + t * d1[1]))
    return None


def side(poly, pt, at):
    """Signed side of pt relative to the polyline direction near index at."""
    return None


def count(svg):
    ds = edge_paths(svg)
    polys = []
    cmds = set()
    for d in ds:
        p, c = flatten(d)
        polys.append(p)
        cmds |= c
    pairs = 0
    total = 0
    overlap = 0.0
    detail = []
    for a in range(len(polys)):
        A = polys[a]
        axs = [p[0] for p in A]
        ays = [p[1] for p in A]
        for b in range(a + 1, len(polys)):
            B = polys[b]
            bxs = [p[0] for p in B]
            bys = [p[1] for p in B]
            if max(axs) < min(bxs) or max(bxs) < min(axs) or max(ays) < min(bys) or max(bys) < min(ays):
                continue
            found = []
            ov = 0.0
            for i in range(len(A) - 1):
                for j in range(len(B) - 1):
                    r = seg_x(A[i], A[i + 1], B[j], B[j + 1])
                    if r is None:
                        continue
                    if r[0] == 'o':
                        ov += r[1]
                    else:
                        pt = r[1]
                        if not any(math.hypot(pt[0] - f[0], pt[1] - f[1]) < 1.5 for f in found):
                            found.append(pt)
            if found:
                pairs += 1
                total += len(found)
                detail.append((a, b, [(round(x, 1), round(y, 1)) for x, y in found]))
            if ov > 0.5:
                overlap += ov
    return len(polys), pairs, total, overlap, detail, cmds


if __name__ == '__main__':
    for f in sys.argv[1:]:
        svg = open(f).read()
        n, pairs, total, ov, detail, cmds = count(svg)
        print('%s\tedges=%d\tpairs=%d\tcrossings=%d\toverlap=%.1f\tcmds=%s' % (f, n, pairs, total, ov, ''.join(sorted(cmds))))
        for d in detail:
            print('   ', d)
