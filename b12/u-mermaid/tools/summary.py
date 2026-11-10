#!/usr/bin/env python3
"""Cross-input summary from the per-render tables (tables/*.md) and block padding from meas.json."""
import json, re, sys
sys.path.insert(0, 'tools')
from analyze import Model
inputs = [('infra-overview', 'y'), ('infra-overview-v2', 'y'), ('advanced', 'y'), ('subgraph', 'y'), ('fanout-frames', 'x'), ('state', 'y'), ('bench-75', 'x'), ('dense-td', 'y')]
d = json.load(open('meas.json'))
fm = {}
for n, _ in inputs:
    try:
        s = open('fm038/%s.svg' % n).read(); vb = [float(x) for x in re.search(r'viewBox="([\d.\- ]+)"', s).group(1).split()]
        fm[n] = vb[2] * vb[3] / 1e6
    except Exception:
        fm[n] = None


def stats(n, lay):
    try:
        t = open('tables/%s.%s.md' % (n, lay)).read()
    except Exception:
        return None
    g = lambda pat, default='0': (re.search(pat, t).group(1) if re.search(pat, t) else default)
    bands = int(g(r'bands: (\d+);'))
    stag = g(r'not one layer\): ([^.\n]+)\.', 'none')
    gaps = [int(x) for x in re.findall(r'^\| \d+ \| \d+ \| \d+ \| \d+ \| .* \| (\d+) \|$', t, re.M)]
    lanes = [int(m.group(2)) for m in re.finditer(r'^\| (gap [^|]+) \| \d+ \| \d+ \| (\d+)', t, re.M)]
    thr = int(g(r'do not belong to \((\d+) cases\)'))
    nod = int(g(r'not their end \((\d+) cases\)'))
    tit = t.count('over title of')
    lab = re.search(r'Labels: (\d+); more than 12 px off their own line: (\d+); more than 300 px of path from the nearer end: (\d+); overlapping something: (\d+)', t)
    tot = re.search(r'canvas area ([\d.]+) Mpx2, edge length (\d+) px, bends (\d+), edge crossings \(drawn polylines\) (\d+)', t)
    return dict(bands=bands, stag=stag, gapmax=max(gaps) if gaps else 0, lanemax=max(lanes) if lanes else 0, thr=thr, nod=nod, tit=tit,
                lab=lab.groups() if lab else None, area=float(tot.group(1)), elen=int(tot.group(2)), bends=int(tot.group(3)), cross=int(tot.group(4)))


print('| input | layout | canvas Mpx2 | bands | staggered bands | largest band gap | most lanes in one gap | edges through foreign blocks | edges over foreign nodes | lines over block titles | labels: total / off line / far from both ends / overlapping | edge length px | crossings |')
print('|---|---|---|---|---|---|---|---|---|---|---|---|---|')
for n, a in inputs:
    for lay in ('elk', 'dagre'):
        s = stats(n, lay)
        if not s:
            continue
        lab = ' / '.join(s['lab']) if s['lab'] else '-'
        print('| %s | %s | %.2f | %d | %s | %s | %s | %d | %d | %d | %s | %d | %d |' % (n, 'ELK (default)' if lay == 'elk' else 'dagre', s['area'], s['bands'], s['stag'], s['gapmax'] if lay == 'elk' else 'n/a', s['lanemax'] if lay == 'elk' else 'n/a (curves)', s['thr'], s['nod'], s['tit'], lab, s['elen'], s['cross']))
print('@@SPLIT@@')
print('| input | mermaid ELK | mermaid dagre | flowmaid 0.38.0 |')
print('|---|---|---|---|')
for n, a in inputs:
    e = stats(n, 'elk'); g = stats(n, 'dagre')
    print('| %s | %.2f | %s | %s |' % (n, e['area'], ('%.2f' % g['area']) if g else '-', ('%.2f' % fm[n]) if fm.get(n) else '-'))
print('@@SPLIT@@')
print('| input | block | frame w x h | padding left / top / right / bottom | title w x h |')
print('|---|---|---|---|---|')
for n, a in inputs:
    v = d.get(n + '.elk.svg')
    if not v:
        continue
    M = Model(v, a)
    for c in sorted(M.clusters, key=lambda c: (c['depth'], c['y'], c['x'])):
        kids = [k for k in M.clusters if k['parent'] == c['id']]
        rs = [x['r'] for x in M.nodes if x['cluster'] == c['id']] + [k['r'] for k in kids]
        if not rs:
            continue
        l = min(r[0] for r in rs) - c['r'][0]; t = min(r[1] for r in rs) - c['r'][1]; r_ = c['r'][2] - max(r[2] for r in rs); b = c['r'][3] - max(r[3] for r in rs)
        tb = c.get('titleBox') or {'w': 0, 'h': 0}
        print('| %s | %s | %.0f x %.0f | %.0f / %.0f / %.0f / %.0f | %.0f x %.0f |' % (n, c['id'], c['w'], c['h'], l, t, r_, b, tb['w'], tb['h']))
