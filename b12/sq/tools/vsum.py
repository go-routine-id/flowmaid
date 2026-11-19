import sys, csv
# vsum.py A_dir B_dir sets...: per set and mode, totals of the verdict columns A -> B and files worse/better
cols = ['defects', 'x_drawn', 'lateral_x', 'fan_x', 'edge_chip', 'side_ends', 'overlap_pairs']
def load(p):
    rows = list(csv.reader(open(p), delimiter='\t')); h = rows[0]; out = {}
    for r in rows[1:]:
        if len(r) < len(h) or r[0] in ('file', 'TOTAL'): continue
        out[(r[0], r[1])] = dict(zip(h, r))
    return out
A, B = sys.argv[1], sys.argv[2]
for s in sys.argv[3:]:
    a, b = load(f'{A}/{s}.tsv'), load(f'{B}/{s}.tsv')
    for mode in ('static', 'routed'):
        parts = []
        for c in cols:
            ta = sum(float(a[k][c]) for k in a if k[1] == mode)
            tb = sum(float(b[k][c]) for k in b if k[1] == mode and k in a)
            w = sum(1 for k in a if k[1] == mode and k in b and float(b[k][c]) > float(a[k][c]))
            bt = sum(1 for k in a if k[1] == mode and k in b and float(b[k][c]) < float(a[k][c]))
            parts.append(f'{c} {ta:g}->{tb:g} (+{w}/-{bt})')
        gw = sum(1 for k in a if k[1] == mode and k in b and float(b[k]['min_edge_gap']) < float(a[k]['min_edge_gap']) - 0.5)
        parts.append(f'gapdown {gw}')
        print(f'{s} {mode}: ' + '  '.join(parts))
