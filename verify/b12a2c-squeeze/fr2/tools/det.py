# det.py <svg>... : per labelled edge, distance (px) from chip centre to its own drawn path
import re, sys, math, html
def sample(d):
    toks = re.findall(r'[MLCAQZmlcaqz]|-?[\d.]+(?:e-?\d+)?', d)
    pts = []; i = 0; cur = None; cmd = None
    def num():
        nonlocal i
        v = float(toks[i]); i += 1; return v
    while i < len(toks):
        t = toks[i]
        if t.isalpha():
            cmd = t; i += 1
            if cmd in 'Zz': continue
        if cmd == 'M' or cmd == 'L':
            p = (num(), num()); pts.append(p); cur = p
        elif cmd == 'C':
            a = (num(), num()); b = (num(), num()); c = (num(), num())
            for k in range(1, 13):
                s = k / 12; u = 1 - s
                pts.append((u*u*u*cur[0]+3*u*u*s*a[0]+3*u*s*s*b[0]+s*s*s*c[0], u*u*u*cur[1]+3*u*u*s*a[1]+3*u*s*s*b[1]+s*s*s*c[1]))
            cur = c
        elif cmd == 'Q':
            a = (num(), num()); c = (num(), num())
            for k in range(1, 9):
                s = k / 8; u = 1 - s
                pts.append((u*u*cur[0]+2*u*s*a[0]+s*s*c[0], u*u*cur[1]+2*u*s*a[1]+s*s*c[1]))
            cur = c
        elif cmd == 'A':
            num(); num(); num(); num(); num(); c = (num(), num()); pts.append(c); cur = c
        else:
            i += 1
    return pts
def segd(p, a, b):
    dx, dy = b[0]-a[0], b[1]-a[1]; l2 = dx*dx+dy*dy
    t = 0 if l2 < 1e-12 else max(0, min(1, ((p[0]-a[0])*dx+(p[1]-a[1])*dy)/l2))
    return math.hypot(p[0]-a[0]-t*dx, p[1]-a[1]-t*dy)
for f in sys.argv[1:]:
    s = open(f).read()
    paths = re.findall(r'<path d="([^"]*)"[^>]*><title>([^<]*)</title>', s)
    lab = [(html.unescape(t), d) for d, t in paths if ': ' in html.unescape(t)]
    chips = re.findall(r'<rect x="([-\d.]+)" y="([-\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="4" fill="#ffffff" stroke="#d5d9ec"/>\s*<text[^>]*>([^<]*)', s)
    # multi-line chips: text may be in tspans; fall back to order only
    name = f.rsplit('/', 1)[-1]
    if len(chips) != len(lab):
        print(f"{name}\tMISMATCH chips {len(chips)} labelled paths {len(lab)}"); continue
    for (t, d), (x, y, w, h, txt) in zip(lab, chips):
        cx, cy = float(x)+float(w)/2, float(y)+float(h)/2
        pts = sample(d)
        dist = min(segd((cx, cy), pts[k], pts[k+1]) for k in range(len(pts)-1)) if len(pts) > 1 else float('nan')
        print(f"{name}\t{t}\t{dist:.1f}")
