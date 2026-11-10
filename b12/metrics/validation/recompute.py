#!/usr/bin/env python3
"""Independent recomputation of the block metrics from structdump's scene
(not from the SVG, not with blockmetrics' code), compared with a blockmetrics
table. Usage: recompute.py <struct-dir> <metrics.tsv>"""
import json, sys, os, csv, itertools

sdir, tsv = sys.argv[1], sys.argv[2]
rows = list(csv.reader(open(tsv), delimiter='\t'))
head = rows[0]
bad = 0
for r in rows[1:]:
    m = dict(zip(head, r))
    st = json.load(open(os.path.join(sdir, m['name'] + '.struct.json')))
    sc, SG = st['scene'], st['subgraphs']
    vertical = st['direction'] in ('TD', 'BT')
    box = {c['id']: c for c in sc['clusters']}
    def chain(k):
        out = []
        while k is not None:
            out.append(k); k = SG[k]['parent']
        return out
    # compactness and span
    comp, span = {}, {}
    for k, s in enumerate(SG):
        if s['id'] not in box: continue
        b = box[s['id']]
        na = sum(n['w'] * n['h'] for i, n in enumerate(sc['nodes']) if k in chain(st['nodes'][i]['sub']))
        if na > 0: comp[s['id']] = b['w'] * b['h'] / na
        span[s['id']] = (b['h'] / sc['height']) if vertical else (b['w'] / sc['width'])
    # sibling pairs
    def ends(e):
        return [chain(st['nodes'][x['n']]['sub']) if 'n' in x else chain(x['s']) for x in (e['from'], e['to'])]
    side = stacked = stagger = linked_side = 0
    for i, j in itertools.combinations(range(len(SG)), 2):
        if SG[i]['parent'] != SG[j]['parent'] or SG[i]['id'] not in box or SG[j]['id'] not in box: continue
        a, b = box[SG[i]['id']], box[SG[j]['id']]
        a0, a1 = (a['y'], a['y'] + a['h']) if vertical else (a['x'], a['x'] + a['w'])
        b0, b1 = (b['y'], b['y'] + b['h']) if vertical else (b['x'], b['x'] + b['w'])
        if min(a1, b1) - max(a0, b0) > 1.0:
            side += 1
            if not ((a0 <= b0 + 2 and a1 >= b1 - 2) or (b0 <= a0 + 2 and b1 >= a1 - 2)): stagger += 1
            if any((i in p and j in q) or (j in p and i in q) for p, q in map(ends, (e for e in st['edges'] if e['drawn']))): linked_side += 1
        else:
            stacked += 1
    want = {
        'compact_mean': sum(comp.values()) / len(comp) if comp else None,
        'compact_max': max(comp.values()) if comp else None,
        'span_max': max(span.values()) if span else None,
        'sib_side': side, 'sib_stacked': stacked, 'sib_stagger': stagger, 'sib_side_linked': linked_side,
    }
    line = []
    for k, v in want.items():
        got = m[k]
        if v is None: continue
        # compactness: 1 % - a cylinder is drawn up to 1 px shorter than its scene box (arc radii are
        # printed to 0.1 px), so the SVG's node area is a little under the scene's
        tol = max(0.011, 0.01 * v) if k.startswith('compact') else 0.0011 if k == 'span_max' else 0
        ok = abs(float(got) - v) <= tol
        bad += not ok
        line.append(f"{k} {got}{'' if ok else ' != ' + format(v, '.3f')}")
    print(f"{m['name']:15s} " + ('OK   ' if all('!=' not in x for x in line) else 'DIFF ') + '; '.join(line))
print('recompute:', 'all rows agree' if not bad else f'{bad} cell(s) differ')
sys.exit(1 if bad else 0)
