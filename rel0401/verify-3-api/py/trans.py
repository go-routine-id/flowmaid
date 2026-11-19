# Is head SVG = base SVG translated by head's viewBox origin? usage: trans.py <diffs.tsv> <basedir> <headdir> [limit]
import re, sys, collections
NUM = re.compile(r'-?\d+(?:\.\d+)?(?:e-?\d+)?')
ATTR = re.compile(r'([\w:-]+)="([^"]*)"')
EL = re.compile(r'<[^>]*>|[^<]+')
XS = {'x', 'cx', 'x1', 'x2'}
YS = {'y', 'cy', 'y1', 'y2'}
TOL = 0.1 + 1e-6

def path_axes(d):
    # returns list of axis per number: 'x','y','-' ; or None if unsupported
    toks = re.findall(r'[A-Za-z]|-?\d+(?:\.\d+)?(?:e-?\d+)?', d)
    out = []; cmd = None; k = 0
    for t in toks:
        if t.isalpha():
            cmd = t; k = 0
            if cmd.islower() and cmd not in 'z':
                return None
            continue
        if cmd in 'MLCQST':
            out.append('x' if k % 2 == 0 else 'y')
        elif cmd == 'H':
            out.append('x')
        elif cmd == 'V':
            out.append('y')
        elif cmd == 'A':
            j = k % 7
            out.append('x' if j == 5 else 'y' if j == 6 else '-')
        else:
            return None
        k += 1
    return out

def cmp_nums(a, b, axes, vx, vy, where, errs):
    na = NUM.findall(a); nb = NUM.findall(b)
    if len(na) != len(nb) or len(axes) != len(na):
        errs.append(f'{where}: count {a[:80]} | {b[:80]}'); return
    for x, y, ax in zip(na, nb, axes):
        s = vx if ax == 'x' else vy if ax == 'y' else 0.0
        if abs(float(y) - (float(x) + s)) > TOL:
            errs.append(f'{where}: {x}+{s} != {y}'); return

def compare(bs, hs, vx, vy):
    errs = []
    be = EL.findall(bs); he = EL.findall(hs)
    if len(be) != len(he):
        return [f'element count {len(be)} vs {len(he)}']
    in_marker = False
    for i, (a, b) in enumerate(zip(be, he)):
        if a == b and not a.startswith('<'):
            continue
        if not a.startswith('<'):
            # text node: must match
            if a != b: errs.append(f'text {a[:60]!r} vs {b[:60]!r}')
            continue
        tag = re.match(r'</?([\w:-]+)', a)
        tag = tag.group(1) if tag else ''
        if a.startswith('<marker'): in_marker = True
        if a.startswith('</marker'): in_marker = False; continue
        if a.startswith('<svg'):
            continue
        sa = re.sub(r'\s+',' ',ATTR.sub('', a)); sb = re.sub(r'\s+',' ',ATTR.sub('', b))
        if sa != sb:
            errs.append(f'skeleton {a[:80]} vs {b[:80]}'); continue
        aa = ATTR.findall(a); ab = ATTR.findall(b)
        da = dict(aa); db = dict(ab)
        if [k for k, _ in aa] != [k for k, _ in ab]:
            # background rect gains x/y
            if tag == 'rect' and set(db) - set(da) == {'x', 'y'} and abs(float(db['x']) - vx) <= 0.05 and abs(float(db['y']) - vy) <= 0.05:
                db = {k: v for k, v in db.items() if k not in ('x', 'y')}
            else:
                errs.append(f'attrs {a[:100]} vs {b[:100]}'); continue
        for k, va in da.items():
            vb_ = db[k]
            if va == vb_: continue
            if in_marker:
                errs.append(f'marker {k} {va} vs {vb_}'); continue
            if k in XS: cmp_nums(va, vb_, ['x'], vx, vy, k, errs)
            elif k in YS: cmp_nums(va, vb_, ['y'], vx, vy, k, errs)
            elif k == 'points':
                n = len(NUM.findall(va)); cmp_nums(va, vb_, ['x' if j % 2 == 0 else 'y' for j in range(n)], vx, vy, k, errs)
            elif k == 'd':
                ax = path_axes(va)
                if ax is None: errs.append(f'd unsupported {va[:60]}')
                else: cmp_nums(va, vb_, ax, vx, vy, k, errs)
            elif k == 'transform':
                m = re.match(r'translate\(([^,\s]+)[,\s]+([^)]+)\)$', va)
                if m: cmp_nums(va, vb_, ['x', 'y'], vx, vy, k, errs)
                else: errs.append(f'transform {va} vs {vb_}')
            else:
                errs.append(f'{tag}.{k}: {va[:60]} vs {vb_[:60]}')
    return errs

if __name__ == '__main__':
    rows = [l.rstrip('\n').split('\t') for l in open(sys.argv[1]) if l.strip()]
    bd, hd = sys.argv[2], sys.argv[3]
    res = collections.Counter(); bad = []
    for r in rows:
        if r[0] != 'routed': continue
        name, var = r[1], r[2]
        vb = list(map(float, r[6].split()))
        bs = open(f'{bd}/{name}.{var}.svg').read(); hs = open(f'{hd}/{name}.{var}.svg').read()
        e = compare(bs, hs, vb[0], vb[1])
        res['translated' if not e else 'OTHER'] += 1
        if e: bad.append((name, var, e[:4], len(e)))
    print(res)
    for b in bad: print(b)
