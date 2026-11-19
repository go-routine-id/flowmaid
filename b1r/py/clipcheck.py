# Ink outside the viewBox: every edge path's coordinates (on-curve and control
# points, +- half stroke) and every marker's box, vs the root viewBox.
import re, sys, glob, math
def num(s): return float(s)
def parse_d(d):
    toks = re.findall(r'[MLCQZz]|-?\d+(?:\.\d+)?(?:e-?\d+)?', d)
    pts = []; segs = []; cmd = None; buf = []; cur = None; first = None
    for t in toks:
        if t in 'MLCQZz':
            cmd = t; buf = []; continue
        buf.append(float(t))
        need = {'M': 2, 'L': 2, 'C': 6, 'Q': 4}[cmd]
        if len(buf) == need:
            ps = [(buf[i], buf[i+1]) for i in range(0, need, 2)]
            if cmd == 'M': cur = ps[0]; first = cur; pts.append(cur)
            else:
                segs.append((cur, ps)); pts.extend(ps); cur = ps[-1]
            buf = []
    return first, segs, pts
def unit(a, b):
    dx, dy = b[0]-a[0], b[1]-a[1]; l = math.hypot(dx, dy)
    return (dx/l, dy/l) if l > 1e-9 else None
def end_dir(segs):
    for (frm, ps) in reversed(segs):
        end = ps[-1]
        for q in reversed([frm] + ps[:-1]):
            u = unit(q, end)
            if u: return end, u
    return None
def start_dir(first, segs):
    for (frm, ps) in segs:
        for q in ps:
            u = unit(first, q)
            if u: return first, u
    return None
def check(svg):
    s = open(svg).read()
    m = re.search(r'viewBox="([^"]*)"', s)
    if not m: return None
    vx, vy, vw, vh = map(float, m.group(1).split())
    markers = {}
    for mm in re.finditer(r'<marker id="([^"]+)" viewBox="([^"]+)" refX="([^"]+)" refY="([^"]+)"[^>]*markerWidth="([^"]+)"', s):
        vb = list(map(float, mm.group(2).split()))
        markers[mm.group(1)] = (vb, float(mm.group(3)), float(mm.group(4)), float(mm.group(5)))
    worst = []
    for pm in re.finditer(r'<path d="([^"]*)" fill="none"([^>]*)>', s):
        d, attrs = pm.group(1), pm.group(2)
        if 'A' in d or 'a' in d: continue
        sw = re.search(r'stroke-width="([^"]+)"', attrs)
        half = float(sw.group(1)) / 2 if sw else 1.0
        first, segs, pts = parse_d(d)
        out = -1e9
        for (x, y) in pts:
            out = max(out, vx - (x - half), vy - (y - half), (x + half) - (vx + vw), (y + half) - (vy + vh))
        for which in ('end', 'start'):
            mk = re.search(r'marker-%s="url\(#([^)]+)\)"' % which, attrs)
            if not mk or mk.group(1) not in markers: continue
            vb, rx, ry, mw = markers[mk.group(1)]
            sc = mw / vb[2]
            e = end_dir(segs) if which == 'end' else start_dir(first, segs)
            if not e: continue
            p, u = e; n = (-u[1], u[0])
            for (a, b) in [(vb[0], vb[1]), (vb[0]+vb[2], vb[1]), (vb[0], vb[1]+vb[3]), (vb[0]+vb[2], vb[1]+vb[3])]:
                q = (p[0] + u[0]*(a-rx)*sc + n[0]*(b-ry)*sc, p[1] + u[1]*(a-rx)*sc + n[1]*(b-ry)*sc)
                out = max(out, vx - q[0], vy - q[1], q[0] - (vx+vw), q[1] - (vy+vh))
        if out > float(__import__("os").environ.get("LIM", "0.05")):
            t = re.search(r'<title>([^<]*)', s[pm.end():pm.end()+300])
            worst.append((round(out, 1), t.group(1) if t else '?'))
    return sorted(worst, reverse=True)
tot = 0; bad = 0
for a in sys.argv[1:]:
    for f in sorted(glob.glob(a)):
        r = check(f)
        if r is None: continue
        tot += 1
        if r:
            bad += 1; print(f.split('/')[-1], len(r), r[:4])
print(f"files {tot} clipped {bad}")
