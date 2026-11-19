import re, sys, os
import xml.etree.ElementTree as ET

def tr(el):
    t = el.get('transform') or ''
    m = re.search(r'translate\(\s*([-\d.]+)[ ,]+([-\d.]+)\s*\)', t)
    if m: return float(m.group(1)), float(m.group(2))
    m = re.search(r'translate\(\s*([-\d.]+)\s*\)', t)
    if m: return float(m.group(1)), 0.0
    return 0.0, 0.0

def mermaid_pos(path):
    root = ET.parse(path).getroot()
    out = {}
    def walk(el, ox, oy):
        dx, dy = tr(el)
        ox += dx; oy += dy
        cls = el.get('class') or ''
        i = el.get('id') or ''
        if 'node' in cls.split() and i:
            m = re.search(r'state-(.+)-\d+$', i)
            name = m.group(1) if m else i
            out[name] = (round(ox,1), round(oy,1))
        for c in el: walk(c, ox, oy)
    walk(root, 0, 0)
    return out

def fm_pos(path):
    s = open(path).read()
    out = {}
    for m in re.finditer(r'<g><title>([^<]*)</title>\s*<rect x="([-\d.]+)" y="([-\d.]+)" width="([-\d.]+)" height="([-\d.]+)"', s):
        n, x, y, w, h = m.groups()
        out[n] = (round(float(x)+float(w)/2,1), round(float(y)+float(h)/2,1))
    return out

def shape(p, names=('P','Q','R')):
    pts = [p.get(n) for n in names]
    if any(q is None for q in pts): return 'MISSING %s' % [n for n in names if n not in p]
    v = [(pts[i+1][0]-pts[i][0], pts[i+1][1]-pts[i][1]) for i in range(2)]
    if all(abs(dx) > abs(dy) for dx,dy in v):
        strict = all(abs(dy) < 2 for dx,dy in v)
        d = 'L->R' if all(dx>0 for dx,dy in v) else ('R->L' if all(dx<0 for dx,dy in v) else 'mixed')
        return ('ROW ' if strict else 'row~ ') + d
    if all(abs(dy) > abs(dx) for dx,dy in v):
        strict = all(abs(dx) < 2 for dx,dy in v)
        d = 'T->B' if all(dy>0 for dx,dy in v) else ('B->T' if all(dy<0 for dx,dy in v) else 'mixed')
        return ('COLUMN ' if strict else 'column~ ') + d
    return 'OTHER'

W = sys.argv[1]
names = sorted(f[:-4] for f in os.listdir(os.path.join(W,'in')) if f.endswith('.mmd'))
if len(sys.argv) > 2: names = [n for n in names if any(n.startswith(a) for a in sys.argv[2:])]
for n in names:
    mp = mermaid_pos(os.path.join(W,'mm',n+'.mermaid.svg'))
    fp = fm_pos(os.path.join(W,'fm',n+'.svg'))
    sel = lambda p: ' '.join('%s=(%g,%g)'%(k,p[k][0],p[k][1]) for k in ('P','Q','R','X') if k in p)
    print('%-26s mermaid: %-13s fm040: %-13s' % (n, shape(mp), shape(fp)))
    print('    mm  %s' % sel(mp))
    print('    fm  %s' % sel(fp))
