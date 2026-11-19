#!/usr/bin/env python3
"""evalcmp.py <A> <B> [mode] : per set totals of the verdict columns and per-file worse/better lists."""
import sys, csv, os
A, B = sys.argv[1], sys.argv[2]; mode = sys.argv[3] if len(sys.argv) > 3 else 'static'
cols = ['defects', 'x_drawn', 'lateral_x', 'fan_x', 'edge_chip', 'side_ends', 'overlap_pairs', 'tight_pairs']
def load(p):
    R = {}
    rows = list(csv.reader(open(p), delimiter='\t'))
    if not rows: return R
    h = rows[0]
    for r in rows[1:]:
        if len(r) == len(h) and r[1] == mode and r[0] != 'TOTAL': R[r[0]] = dict(zip(h, r))
    return R
for s in ['rnd', 'tw', 'nr', 'nt', 'sw72', 'i3']:
    a, b = load(os.path.join(A, s + '.tsv')), load(os.path.join(B, s + '.tsv'))
    line = []; worse = []; better = []
    for c in cols:
        ta = sum(float(v[c]) for v in a.values()); tb = sum(float(v[c]) for k, v in b.items() if k in a)
        up = [k for k in a if k in b and float(b[k][c]) > float(a[k][c])]
        dn = [k for k in a if k in b and float(b[k][c]) < float(a[k][c])]
        line.append(f'{c} {ta:g}->{tb:g} (+{len(up)}/-{len(dn)})')
        worse += [f'{k} {c} {a[k][c]}->{b[k][c]}' for k in up]
    gd = [k for k in a if k in b and float(b[k]['min_edge_gap']) < float(a[k]['min_edge_gap']) - 0.5]
    line.append(f'min_edge_gap down {len(gd)}')
    worse += [f'{k} min_edge_gap {a[k]["min_edge_gap"]}->{b[k]["min_edge_gap"]}' for k in gd]
    print(f'[{s}] {len(a)} files: ' + '  '.join(line))
    for w in worse[:40]: print('    WORSE', w)
