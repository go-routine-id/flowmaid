import sys, csv
# vcmp.py A.tsv B.tsv: per file and mode, the verdict columns that differ
cols = ['defects', 'x_drawn', 'lateral_x', 'fan_x', 'edge_chip', 'side_ends', 'min_edge_gap', 'overlap_pairs']
def load(p):
    rows = list(csv.reader(open(p), delimiter='\t'))
    h = rows[0]; out = {}
    for r in rows[1:]:
        if len(r) < len(h) or r[0] in ('file', 'TOTAL'): continue
        d = dict(zip(h, r)); out[(r[0], r[1])] = d
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
worse = better = 0
for k in sorted(set(a) | set(b)):
    if k not in a or k not in b: print('MISSING', k); continue
    diffs = []
    for c in cols:
        if c not in a[k]: continue
        x, y = float(a[k][c]), float(b[k][c])
        if abs(x - y) > 1e-6:
            bad = (y < x - 0.5) if c == 'min_edge_gap' else (y > x)
            good = (y > x + 0.5) if c == 'min_edge_gap' else (y < x)
            worse += bad; better += good
            diffs.append(f"{c} {a[k][c]}->{b[k][c]}{' WORSE' if bad else ''}")
    if diffs: print(k[0], k[1], '; '.join(diffs))
print(f'worse {worse} better {better}')
